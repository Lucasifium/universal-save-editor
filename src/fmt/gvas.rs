//! Unreal Engine 4/5 SaveGame files (`GVAS`).
//!
//! Every property tag carries the byte size of its value, so a property we don't
//! understand is simply skipped (shown read-only) instead of failing the whole file.
//! Numbers and booleans are patched in place; text edits also update the size field
//! of every enclosing property so the file stays consistent.
//!
//! Unreal Engine 5.4 and later write each property's complete type (`MapProperty` of
//! `NameProperty` to `IntProperty`, …) in the tag, so arrays, sets and maps are read
//! element by element there; older saves still show maps and sets as skipped bytes.

use crate::tree::{child, int_in, num, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
enum Kind {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    /// A UE 5.4 boolean kept in its tag's flags byte (bit 0x10); holds the other bits.
    TagBool(u8),
    Str,
}

/// A property's complete type as UE 5.4+ writes it: a name and its parameters.
#[derive(Clone, Debug)]
struct TypeName {
    name: String,
    inner: Vec<TypeName>,
}

/// UE 5.4 property tag flags.
const TAG_ARRAY_INDEX: u8 = 0x01;
const TAG_GUID: u8 = 0x02;
const TAG_EXTENSIONS: u8 = 0x04;
const TAG_NATIVE: u8 = 0x08;
const TAG_BOOL_TRUE: u8 = 0x10;

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    kind: Kind,
    /// Offsets of the i32 size fields of every enclosing property (for text edits).
    sizes: Vec<usize>,
}

struct P<'a> {
    d: &'a [u8],
    pos: usize,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
    /// UE 5.4+ tags with complete type names.
    full_types: bool,
    /// UE5 large world coordinates (vectors of doubles).
    lwc: bool,
}

/// Native structs serialized as raw values: (name, component kind, count).
const NATIVE: &[(&str, &str, usize)] = &[
    ("Vector", "f", 3),
    ("Rotator", "f", 3),
    ("Vector2D", "f", 2),
    ("Vector4", "f", 4),
    ("Quat", "f", 4),
    ("LinearColor", "f32", 4),
    ("Color", "u8", 4),
    ("IntPoint", "i32", 2),
    ("IntVector", "i32", 3),
];

impl P<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.d.len())
            .ok_or("unexpected end of data")?;
        let s = &self.d[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn fstring(&mut self) -> Result<String, String> {
        let n = self.i32()?;
        if n == 0 {
            return Ok(String::new());
        }
        if n > 0 {
            let b = self.take(n as usize)?;
            Ok(String::from_utf8_lossy(&b[..b.len() - 1]).into_owned())
        } else {
            let units = n.unsigned_abs() as usize;
            let b = self.take(units * 2)?;
            let u: Vec<u16> = b
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&u[..u.len() - 1]))
        }
    }
    fn guid(&mut self) -> Result<(), String> {
        if self.u8()? != 0 {
            self.take(16)?;
        }
        Ok(())
    }
    fn leaf(&mut self, ptr: &str, start: usize, kind: Kind, sizes: &[usize], v: Value) -> Value {
        self.editable.insert(ptr.to_string(), self.leaves.len());
        self.leaves.push(Leaf {
            start,
            end: self.pos,
            kind,
            sizes: sizes.to_vec(),
        });
        v
    }
    fn scalar(&mut self, kind: Kind, ptr: &str, sizes: &[usize]) -> Result<Value, String> {
        let start = self.pos;
        macro_rules! rd {
            ($t:ty) => {
                <$t>::from_le_bytes(self.take(std::mem::size_of::<$t>())?.try_into().unwrap())
            };
        }
        let v = match kind {
            Kind::I8 => Value::from(rd!(i8)),
            Kind::I16 => Value::from(rd!(i16)),
            Kind::I32 => Value::from(rd!(i32)),
            Kind::I64 => Value::from(rd!(i64)),
            Kind::U8 => Value::from(rd!(u8)),
            Kind::U16 => Value::from(rd!(u16)),
            Kind::U32 => Value::from(rd!(u32)),
            Kind::U64 => Value::from(rd!(u64)),
            Kind::F32 => crate::tree::f32_value(rd!(f32)),
            Kind::F64 => num(rd!(f64)),
            Kind::Bool => Value::Bool(rd!(u8) != 0),
            Kind::TagBool(_) => Value::Bool(rd!(u8) & TAG_BOOL_TRUE != 0),
            Kind::Str => Value::String(self.fstring()?),
        };
        Ok(self.leaf(ptr, start, kind, sizes, v))
    }
    fn kind_of(ty: &str) -> Option<Kind> {
        Some(match ty {
            "Int8Property" => Kind::I8,
            "Int16Property" => Kind::I16,
            "IntProperty" => Kind::I32,
            "Int64Property" => Kind::I64,
            "UInt16Property" => Kind::U16,
            "UInt32Property" => Kind::U32,
            "UInt64Property" => Kind::U64,
            "FloatProperty" => Kind::F32,
            "DoubleProperty" => Kind::F64,
            "StrProperty" => Kind::Str,
            _ => return None,
        })
    }
    /// One key of a UE 4 map or element of a UE 4 set, as text (never editable). Struct keys
    /// are taken to be GUIDs, which is what saves use.
    fn legacy_key(&mut self, ty: &str) -> Result<String, String> {
        match ty {
            "StructProperty" => Ok(self.take(16)?.iter().map(|b| format!("{b:02x}")).collect()),
            "NameProperty" | "StrProperty" | "ObjectProperty" | "EnumProperty"
            | "SoftObjectProperty" => self.fstring(),
            other => {
                let k = Self::kind_of(other).ok_or("unsupported map key")?;
                let leaves = self.leaves.len();
                let v = self.scalar(k, "\u{0}key", &[])?;
                self.leaves.truncate(leaves);
                self.editable.retain(|_, id| *id < leaves);
                Ok(key_text(&v))
            }
        }
    }
    /// A UE 4 set or map (key type `kt`, value type `vt`, empty for a set): its scalar values
    /// are editable in place; anything it can't read is left to the caller to skip.
    fn legacy_container(
        &mut self,
        kt: &str,
        vt: &str,
        ptr: &str,
        chain: &[usize],
    ) -> Result<Value, String> {
        if self.i32()? != 0 {
            return Err("container with removed entries".into());
        }
        let n = self.count()?;
        if vt.is_empty() {
            return (0..n)
                .map(|_| self.legacy_key(kt).map(Value::String))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array);
        }
        let mut m = Map::new();
        for _ in 0..n {
            let key = self.legacy_key(kt)?;
            let k = unique_key(&m, &key);
            let at = child(ptr, &k);
            let v = match Self::kind_of(vt) {
                Some(kind) => self.scalar(kind, &at, chain)?,
                None if vt == "StructProperty" => {
                    self.props(&at, chain, None).map(Value::Object)?
                }
                None if vt == "BoolProperty" => self.scalar(Kind::Bool, &at, chain)?,
                None => return Err("unsupported map value".into()),
            };
            m.insert(k, v);
        }
        Ok(Value::Object(m))
    }
    /// A property list terminated by "None".
    fn props(
        &mut self,
        ptr: &str,
        sizes: &[usize],
        end: Option<usize>,
    ) -> Result<Map<String, Value>, String> {
        let mut m = Map::new();
        loop {
            if end.is_some_and(|e| self.pos >= e) {
                return Err("property list overran its struct".into());
            }
            let name = self.fstring()?;
            if name == "None" {
                return Ok(m);
            }
            let key = unique_key(&m, &name);
            let v = if self.full_types {
                self.prop5(&child(ptr, &key), sizes)?
            } else {
                self.prop(&child(ptr, &key), sizes)?
            };
            m.insert(key, v);
        }
    }
    /// One property after its name: type, size, tag data, value.
    fn prop(&mut self, ptr: &str, sizes: &[usize]) -> Result<Value, String> {
        let ty = self.fstring()?;
        let size_off = self.pos;
        let size = self.i32()?;
        let _index = self.i32()?;
        if size < 0 || size as usize > self.d.len() {
            return Err("bad property size".into());
        }
        let size = size as usize;
        let mut chain = sizes.to_vec();
        chain.push(size_off);
        match ty.as_str() {
            "BoolProperty" => {
                let v = self.scalar(Kind::Bool, ptr, &chain)?;
                self.guid()?;
                Ok(v)
            }
            "StructProperty" => {
                let st = self.fstring()?;
                self.take(16)?;
                self.guid()?;
                let start = self.pos;
                Ok(self.value_or_skip(start, size, |p| p.struct_value(&st, size, ptr, &chain)))
            }
            "ByteProperty" | "EnumProperty" => {
                let en = self.fstring()?;
                self.guid()?;
                let start = self.pos;
                Ok(self.value_or_skip(start, size, |p| {
                    if ty == "ByteProperty" && en == "None" && size == 1 {
                        p.scalar(Kind::U8, ptr, &chain)
                    } else {
                        p.fstring().map(Value::String)
                    }
                }))
            }
            "ArrayProperty" => {
                let inner = self.fstring()?;
                self.guid()?;
                let start = self.pos;
                Ok(self.value_or_skip(start, size, |p| p.array(&inner, start + size, ptr, &chain)))
            }
            "NameProperty" | "ObjectProperty" | "SoftObjectProperty" | "InterfaceProperty"
            | "ClassProperty" => {
                self.guid()?;
                let start = self.pos;
                Ok(self.value_or_skip(start, size, |p| p.fstring().map(Value::String)))
            }
            "SetProperty" | "MapProperty" => {
                let kt = self.fstring()?;
                let vt = if ty == "MapProperty" {
                    self.fstring()?
                } else {
                    String::new()
                };
                self.guid()?;
                let start = self.pos;
                if self.pos + size > self.d.len() {
                    return Err("property runs past the file".into());
                }
                let value =
                    self.value_or_skip(start, size, |p| p.legacy_container(&kt, &vt, ptr, &chain));
                Ok(match value {
                    Value::String(_) => Value::String(format!("<{ty}: {size} bytes>")),
                    v => v,
                })
            }
            other => {
                self.guid()?;
                let start = self.pos;
                match Self::kind_of(other) {
                    Some(k) => Ok(self.value_or_skip(start, size, |p| p.scalar(k, ptr, &chain))),
                    None => {
                        self.take(size)?;
                        Ok(Value::String(format!("<{other}: {size} bytes>")))
                    }
                }
            }
        }
    }
    fn type_name(&mut self, depth: usize) -> Result<TypeName, String> {
        let name = self.fstring()?;
        let n = self.i32()?;
        if !(0..=8).contains(&n) || depth > 16 {
            return Err("bad property type".into());
        }
        let inner = (0..n)
            .map(|_| self.type_name(depth + 1))
            .collect::<Result<_, _>>()?;
        Ok(TypeName { name, inner })
    }
    /// One UE 5.4+ property after its name: complete type, size, flags, value.
    fn prop5(&mut self, ptr: &str, sizes: &[usize]) -> Result<Value, String> {
        let ty = self.type_name(0)?;
        let size_off = self.pos;
        let size = self.i32()?;
        if size < 0 || size as usize > self.d.len() {
            return Err("bad property size".into());
        }
        let size = size as usize;
        let flags_at = self.pos;
        let flags = self.u8()?;
        if flags & !0x1F != 0 || flags & TAG_EXTENSIONS != 0 {
            return Err("unsupported property tag flags".into());
        }
        if flags & TAG_ARRAY_INDEX != 0 {
            self.i32()?;
        }
        if flags & TAG_GUID != 0 {
            self.take(16)?;
        }
        if ty.name == "BoolProperty" {
            // The value is the flag; nothing follows.
            let end = self.pos;
            self.pos = flags_at;
            let v = self.scalar(Kind::TagBool(flags & !TAG_BOOL_TRUE), ptr, sizes)?;
            self.leaves.last_mut().unwrap().end = flags_at + 1;
            self.pos = end;
            return Ok(v);
        }
        let mut chain = sizes.to_vec();
        chain.push(size_off);
        let start = self.pos;
        Ok(self.value_or_skip(start, size, |p| {
            // A byte with an enum type is the enumerator's name, unless it's one raw byte.
            if ty.name == "ByteProperty" && size == 1 {
                return p.scalar(Kind::U8, ptr, &chain);
            }
            p.typed(&ty, ptr, &chain, flags & TAG_NATIVE != 0)
        }))
    }
    /// A value of a known UE 5.4+ type (also the elements of arrays, sets and maps).
    fn typed(
        &mut self,
        ty: &TypeName,
        ptr: &str,
        sizes: &[usize],
        native: bool,
    ) -> Result<Value, String> {
        let param = |i: usize| {
            ty.inner
                .get(i)
                .ok_or_else(|| format!("{} without its type", ty.name))
        };
        match ty.name.as_str() {
            "StructProperty" => {
                let st = param(0)?.name.as_str();
                if let Some(r) = self.native5(st, ptr, sizes) {
                    return r;
                }
                if native {
                    return Err("struct in its own binary form".into());
                }
                self.props(ptr, sizes, None).map(Value::Object)
            }
            "ArrayProperty" | "SetProperty" => {
                let inner = param(0)?;
                if ty.name == "SetProperty" && self.i32()? != 0 {
                    return Err("set with removed entries".into());
                }
                let n = self.count()?;
                if inner.name == "ByteProperty" && inner.inner.is_empty() && n > 4096 {
                    self.take(n)?;
                    return Ok(Value::String(format!("<{n} bytes>")));
                }
                (0..n)
                    .map(|i| self.typed(inner, &format!("{ptr}/{i}"), sizes, false))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array)
            }
            "MapProperty" => {
                let (kt, vt) = (param(0)?, param(1)?);
                if self.i32()? != 0 {
                    return Err("map with removed entries".into());
                }
                let n = self.count()?;
                let mut m = Map::new();
                for _ in 0..n {
                    // Keys are shown as the entry's name, not edited.
                    let leaves = self.leaves.len();
                    let key = key_text(&self.typed(kt, "\u{0}key", sizes, false)?);
                    self.leaves.truncate(leaves);
                    self.editable.retain(|_, id| *id < leaves);
                    let k = unique_key(&m, &key);
                    let v = self.typed(vt, &child(ptr, &k), sizes, false)?;
                    m.insert(k, v);
                }
                Ok(Value::Object(m))
            }
            "BoolProperty" => self.scalar(Kind::Bool, ptr, sizes),
            "ByteProperty" if ty.inner.is_empty() => self.scalar(Kind::U8, ptr, sizes),
            "ByteProperty" | "EnumProperty" | "NameProperty" | "ObjectProperty"
            | "ClassProperty" | "InterfaceProperty" => self.fstring().map(Value::String),
            "SoftObjectProperty" | "SoftClassProperty" => {
                let parts = [self.fstring()?, self.fstring()?, self.fstring()?];
                Ok(Value::String(
                    parts
                        .iter()
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(":"),
                ))
            }
            other => match Self::kind_of(other) {
                Some(k) => self.scalar(k, ptr, sizes),
                None => Err(format!("unsupported type {other}")),
            },
        }
    }
    /// An element count that can't be more than the bytes left.
    fn count(&mut self) -> Result<usize, String> {
        let n = self.i32()?;
        if n < 0 || n as usize > self.d.len() - self.pos {
            return Err("bad element count".into());
        }
        Ok(n as usize)
    }
    /// Structs UE 5 writes as raw values.
    fn native5(&mut self, st: &str, ptr: &str, sizes: &[usize]) -> Option<Result<Value, String>> {
        let (kind, n) = match st {
            "Vector" | "Rotator" => (if self.lwc { Kind::F64 } else { Kind::F32 }, 3),
            "Vector2D" => (if self.lwc { Kind::F64 } else { Kind::F32 }, 2),
            "Vector4" | "Quat" => (if self.lwc { Kind::F64 } else { Kind::F32 }, 4),
            "LinearColor" => (Kind::F32, 4),
            "Color" => (Kind::U8, 4),
            "IntPoint" => (Kind::I32, 2),
            "IntVector" => (Kind::I32, 3),
            "DateTime" | "Timespan" => return Some(self.scalar(Kind::I64, ptr, sizes)),
            "Guid" => {
                return Some(
                    self.take(16)
                        .map(|b| Value::String(b.iter().map(|x| format!("{x:02x}")).collect())),
                )
            }
            _ => return None,
        };
        Some(
            (0..n)
                .map(|i| self.scalar(kind, &format!("{ptr}/{i}"), sizes))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
        )
    }
    /// Parse a value that must occupy exactly `size` bytes; on any mismatch, skip it
    /// (read-only) and drop leaves registered while trying.
    fn value_or_skip(
        &mut self,
        start: usize,
        size: usize,
        f: impl FnOnce(&mut Self) -> Result<Value, String>,
    ) -> Value {
        // Leaves are numbered in order, so the ones added by a failed try are those past `leaves`
        // (no copy of the whole map per value: big saves have tens of thousands of them).
        let leaves = self.leaves.len();
        match f(self) {
            Ok(v) if self.pos == start + size => v,
            r => {
                if std::env::var_os("GVAS_DEBUG").is_some() {
                    eprintln!("SKIP {size} bytes: {:?}", r.as_ref().err());
                }
                self.leaves.truncate(leaves);
                self.editable.retain(|_, id| *id < leaves);
                self.pos = start + size;
                Value::String(format!("<{size} bytes>"))
            }
        }
    }
    fn native(
        &mut self,
        st: &str,
        size: usize,
        ptr: &str,
        sizes: &[usize],
    ) -> Option<Result<Value, String>> {
        let &(_, comp, n) = NATIVE.iter().find(|(name, _, _)| *name == st)?;
        let kind = match comp {
            "f" if size == n * 8 => Kind::F64, // UE5 large world coordinates
            "f" | "f32" => Kind::F32,
            "u8" => Kind::U8,
            _ => Kind::I32,
        };
        Some(
            (0..n)
                .map(|i| self.scalar(kind, &format!("{ptr}/{i}"), sizes))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
        )
    }
    fn struct_value(
        &mut self,
        st: &str,
        size: usize,
        ptr: &str,
        sizes: &[usize],
    ) -> Result<Value, String> {
        if st == "Guid" && size == 16 {
            return Ok(Value::String(
                self.take(16)?.iter().map(|b| format!("{b:02x}")).collect(),
            ));
        }
        if let Some(r) = self.native(st, size, ptr, sizes) {
            return r;
        }
        let end = self.pos + size;
        self.props(ptr, sizes, Some(end)).map(Value::Object)
    }
    fn array(
        &mut self,
        inner: &str,
        end: usize,
        ptr: &str,
        sizes: &[usize],
    ) -> Result<Value, String> {
        let n = self.i32()?;
        if n < 0 {
            return Err("bad array length".into());
        }
        let n = n as usize;
        let item = |i: usize| format!("{ptr}/{i}");
        let mut out = Vec::new();
        match inner {
            "StructProperty" => {
                self.fstring()?;
                self.fstring()?;
                let size_off = self.pos;
                let total = self.i32()? as usize;
                self.i32()?;
                let st = self.fstring()?;
                self.take(16)?;
                self.guid()?;
                let mut chain = sizes.to_vec();
                chain.push(size_off);
                let each = if n > 0 { total / n } else { 0 };
                for i in 0..n {
                    // Elements of a property-list struct differ in size; each ends at its own "None".
                    let raw = st == "Guid" || NATIVE.iter().any(|(name, _, _)| *name == st);
                    out.push(if raw {
                        self.struct_value(&st, each, &item(i), &chain)?
                    } else {
                        Value::Object(self.props(&item(i), &chain, None)?)
                    });
                }
            }
            "BoolProperty" => (0..n).try_for_each(|i| {
                self.scalar(Kind::Bool, &item(i), sizes)
                    .map(|v| out.push(v))
            })?,
            // An embedded file compressed by Unreal (Hogwarts Legacy's database uses Oodle,
            // which has no open decoder): far too many bytes to list, and nothing to edit.
            "ByteProperty"
                if end - self.pos == n
                    && n > 4096
                    && self.d[self.pos..].starts_with(&[0xC1, 0x83, 0x2A, 0x9E]) =>
            {
                self.pos = end;
                return Ok(Value::String(format!(
                    "<{n} bytes of compressed Unreal data>"
                )));
            }
            "ByteProperty" if end - self.pos == n => (0..n)
                .try_for_each(|i| self.scalar(Kind::U8, &item(i), sizes).map(|v| out.push(v)))?,
            "NameProperty" | "EnumProperty" | "ObjectProperty" | "SoftObjectProperty" => {
                for _ in 0..n {
                    out.push(Value::String(self.fstring()?));
                }
            }
            other => {
                let k = Self::kind_of(other).ok_or("unsupported array type")?;
                (0..n).try_for_each(|i| self.scalar(k, &item(i), sizes).map(|v| out.push(v)))?;
            }
        }
        Ok(Value::Array(out))
    }
}

#[derive(Clone, Debug)]
struct GvasBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

/// A map key as the entry's name: text as is, numbers written out, struct fields joined.
fn key_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Object(m) => m.values().map(key_text).collect::<Vec<_>>().join("|"),
        Value::Array(a) => a.iter().map(key_text).collect::<Vec<_>>().join("|"),
        other => other.to_string(),
    }
}

fn fstring_bytes(s: &str) -> Vec<u8> {
    if s.is_empty() {
        return 0i32.to_le_bytes().to_vec();
    }
    if s.is_ascii() {
        let mut out = ((s.len() + 1) as i32).to_le_bytes().to_vec();
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        out
    } else {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut out = (-((units.len() + 1) as i32)).to_le_bytes().to_vec();
        for u in units {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out.extend_from_slice(&[0, 0]);
        out
    }
}

fn encode(kind: Kind, v: &Value) -> Result<Vec<u8>, String> {
    macro_rules! int {
        ($t:ty) => {
            (int_in(v, <$t>::MIN as i128, <$t>::MAX as i128, "This number")? as $t)
                .to_le_bytes()
                .to_vec()
        };
    }
    Ok(match kind {
        Kind::I8 => int!(i8),
        Kind::I16 => int!(i16),
        Kind::I32 => int!(i32),
        Kind::I64 => int!(i64),
        Kind::U8 => int!(u8),
        Kind::U16 => int!(u16),
        Kind::U32 => int!(u32),
        Kind::U64 => int!(u64),
        Kind::F32 => (v.as_f64().ok_or("must be a number")? as f32)
            .to_le_bytes()
            .to_vec(),
        Kind::F64 => v.as_f64().ok_or("must be a number")?.to_le_bytes().to_vec(),
        Kind::Bool => vec![v.as_bool().ok_or("must be on/off")? as u8],
        Kind::TagBool(rest) => vec![
            rest | if v.as_bool().ok_or("must be on/off")? {
                TAG_BOOL_TRUE
            } else {
                0
            },
        ],
        Kind::Str => fstring_bytes(v.as_str().ok_or("must be text")?),
    })
}

impl Backend for GvasBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        let mut size_delta: HashMap<usize, i64> = HashMap::new();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            let bytes = encode(l.kind, v)?;
            let delta = bytes.len() as i64 - (l.end - l.start) as i64;
            if delta != 0 {
                for &s in &l.sizes {
                    *size_delta.entry(s).or_default() += delta;
                }
            }
            p.add(l.start, l.end, bytes);
        }
        for (off, delta) in size_delta {
            let old = i32::from_le_bytes(self.raw[off..off + 4].try_into().unwrap()) as i64;
            let new = i32::try_from(old + delta).map_err(|_| "property too large")?;
            p.add(off, off + 4, new.to_le_bytes().to_vec());
        }
        p.apply(&self.raw)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if matches!(self.leaves[leaf].kind, Kind::F32) {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode(self.leaves[leaf].kind, v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut p = P {
        d,
        pos: 0,
        leaves: vec![],
        editable: HashMap::new(),
        full_types: false,
        lwc: false,
    };
    if p.take(4)? != b"GVAS" {
        return Err("not GVAS".into());
    }
    let save_version = p.i32()?;
    p.i32()?; // UE4 package version
    if save_version >= 3 {
        p.i32()?; // UE5 package version
    }
    let engine = p.take(10)?; // engine major/minor/patch + changelist
    let (major, minor) = (
        u16::from_le_bytes([engine[0], engine[1]]),
        u16::from_le_bytes([engine[2], engine[3]]),
    );
    p.full_types = (major, minor) >= (5, 4);
    p.lwc = major >= 5;
    p.fstring()?; // branch
    p.i32()?; // custom version format
    let n = p.i32()?;
    if !(0..10_000).contains(&n) {
        return Err("bad custom version count".into());
    }
    p.take(n as usize * 20)?;
    let class = p.fstring()?;
    if p.full_types {
        p.u8()?; // flags byte after the class name (0)
    }
    let props = p.props("", &[], None)?;
    let trailing = d.len() - p.pos;
    if trailing > 16 && d[p.pos..].iter().any(|&b| b != 0) {
        return Err(format!(
            "unexpected data after properties ({trailing} bytes)"
        ));
    }
    let short = class.rsplit('.').next().unwrap_or(&class).to_string();
    let backend = GvasBackend {
        raw: d.to_vec(),
        leaves: p.leaves,
    };
    let name = if short.is_empty() {
        "Unreal Engine save".to_string()
    } else {
        format!("Unreal Engine save ({short})")
    };
    Ok(TreeDoc::new(
        Value::Object(props),
        p.editable,
        name,
        Box::new(backend),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs(s: &str) -> Vec<u8> {
        fstring_bytes(s)
    }

    /// Property header + tag data + value. `tag` goes between size/index and the guid byte.
    fn prop(name: &str, ty: &str, tag: &[u8], value: &[u8], value_in_tag: bool) -> Vec<u8> {
        let mut v = fs(name);
        v.extend(fs(ty));
        v.extend((if value_in_tag { 0 } else { value.len() as i32 }).to_le_bytes());
        v.extend(0i32.to_le_bytes());
        v.extend(tag);
        if value_in_tag {
            v.extend(value);
        }
        v.push(0); // no property guid
        if !value_in_tag {
            v.extend(value);
        }
        v
    }

    fn sample() -> Vec<u8> {
        let mut v = b"GVAS".to_vec();
        v.extend(2i32.to_le_bytes());
        v.extend(522i32.to_le_bytes());
        v.extend([4, 0, 27, 0, 2, 0, 1, 2, 3, 4]);
        v.extend(fs("++UE4+Release-4.27"));
        v.extend(3i32.to_le_bytes());
        v.extend(0i32.to_le_bytes());
        v.extend(fs("/Script/MyGame.MySaveGame"));
        v.extend(prop(
            "Gold",
            "IntProperty",
            &[],
            &1500i32.to_le_bytes(),
            false,
        ));
        v.extend(prop("PlayerName", "StrProperty", &[], &fs("Ash"), false));
        v.extend(prop("Hardcore", "BoolProperty", &[], &[1], true));
        v.extend(prop(
            "Health",
            "FloatProperty",
            &[],
            &97.5f32.to_le_bytes(),
            false,
        ));
        // Struct with nested properties.
        let mut stats = prop("Level", "IntProperty", &[], &7i32.to_le_bytes(), false);
        stats.extend(prop("Title", "StrProperty", &[], &fs("Novice"), false));
        stats.extend(fs("None"));
        let mut tag = fs("PlayerStats");
        tag.extend([0u8; 16]);
        v.extend(prop("Stats", "StructProperty", &tag, &stats, false));
        // Array of ints.
        let mut arr = 3i32.to_le_bytes().to_vec();
        for x in [10, 20, 30] {
            arr.extend((x as i32).to_le_bytes());
        }
        v.extend(prop(
            "Scores",
            "ArrayProperty",
            &fs("IntProperty"),
            &arr,
            false,
        ));
        v.extend(fs("None"));
        v.extend(0i32.to_le_bytes());
        v
    }

    #[test]
    fn edit_numbers_and_strings_with_size_fixups() {
        let raw = sample();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        assert_eq!(doc.view.pointer("/Gold").unwrap(), 1500);
        assert_eq!(doc.view.pointer("/Stats/Title").unwrap(), "Novice");
        doc.set("/Gold", Value::from(999999)).unwrap();
        doc.set("/Hardcore", Value::Bool(false)).unwrap();
        doc.set("/Scores/1", Value::from(-5)).unwrap();
        doc.set("/PlayerName", Value::from("Misty Waterflower"))
            .unwrap();
        doc.set("/Stats/Title", Value::from("Grand Champion Ünïcode"))
            .unwrap();
        doc.set("/Stats/Level", Value::from(99)).unwrap();
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        for (p, v) in [
            ("/Gold", Value::from(999999)),
            ("/Hardcore", Value::Bool(false)),
            ("/Scores/1", Value::from(-5)),
            ("/PlayerName", Value::from("Misty Waterflower")),
            ("/Stats/Title", Value::from("Grand Champion Ünïcode")),
            ("/Stats/Level", Value::from(99)),
        ] {
            assert_eq!(again.view.pointer(p).unwrap(), &v, "{p}");
        }
        assert!(doc.set("/Gold", Value::from(1_i64 << 40)).is_err());
    }

    /// UE 4 maps with GUID keys, sets, and arrays of structs whose elements differ in size
    /// (Deep Rock Galactic's layout), with zero padding after the properties.
    #[test]
    fn legacy_maps_and_struct_arrays() {
        let mut v = b"GVAS".to_vec();
        v.extend(2i32.to_le_bytes());
        v.extend(522i32.to_le_bytes());
        v.extend([4, 0, 27, 0, 2, 0, 1, 2, 3, 4]);
        v.extend(fs("++UE4+Release-4.27"));
        v.extend(3i32.to_le_bytes());
        v.extend(0i32.to_le_bytes());
        v.extend(fs("/Script/FSD.FSDSaveGame"));
        // Map<Guid, float>
        let mut map = 0i32.to_le_bytes().to_vec();
        map.extend(2i32.to_le_bytes());
        for (g, f) in [(1u8, 37.0f32), (2, 10.5)] {
            map.extend([g; 16]);
            map.extend(f.to_le_bytes());
        }
        let mut tag = fs("StructProperty");
        tag.extend(fs("FloatProperty"));
        v.extend(prop("Owned", "MapProperty", &tag, &map, false));
        // Set<Name>
        let mut set = 0i32.to_le_bytes().to_vec();
        set.extend(1i32.to_le_bytes());
        set.extend(fs("Seen"));
        v.extend(prop(
            "Events",
            "SetProperty",
            &fs("NameProperty"),
            &set,
            false,
        ));
        // Array<Struct{XP: int, Tag: str}> with elements of different sizes.
        let mut elems = vec![];
        for (xp, t) in [(5i32, "a"), (6, "a longer tag")] {
            elems.extend(prop("XP", "IntProperty", &[], &xp.to_le_bytes(), false));
            elems.extend(prop("Tag", "StrProperty", &[], &fs(t), false));
            elems.extend(fs("None"));
        }
        let mut arr = 2i32.to_le_bytes().to_vec();
        arr.extend(fs("Chars"));
        arr.extend(fs("StructProperty"));
        arr.extend((elems.len() as i32).to_le_bytes());
        arr.extend(0i32.to_le_bytes());
        arr.extend(fs("CharSave"));
        arr.extend([0u8; 16]);
        arr.push(0);
        arr.extend(&elems);
        v.extend(prop(
            "Chars",
            "ArrayProperty",
            &fs("StructProperty"),
            &arr,
            false,
        ));
        v.extend(fs("None"));
        v.extend(0i32.to_le_bytes());
        v.extend([0u8; 40]);
        let raw = v;
        let mut doc = parse(&raw).unwrap();
        assert_eq!(
            doc.view
                .pointer("/Owned/01010101010101010101010101010101")
                .unwrap(),
            37.0
        );
        assert_eq!(doc.view.pointer("/Events/0").unwrap(), "Seen");
        assert_eq!(doc.view.pointer("/Chars/1/Tag").unwrap(), "a longer tag");
        assert_eq!(doc.to_bytes().unwrap(), raw);
        doc.set("/Owned/02020202020202020202020202020202", Value::from(9999))
            .unwrap();
        doc.set("/Chars/0/XP", Value::from(318093)).unwrap();
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(
            again
                .view
                .pointer("/Owned/02020202020202020202020202020202")
                .unwrap(),
            9999.0
        );
        assert_eq!(again.view.pointer("/Chars/0/XP").unwrap(), 318093);
    }

    /// A UE 5.4 type name: name, parameter count, parameters.
    fn tn(name: &str, inner: &[Vec<u8>]) -> Vec<u8> {
        let mut v = fs(name);
        v.extend((inner.len() as i32).to_le_bytes());
        for i in inner {
            v.extend(i);
        }
        v
    }

    fn prop5(name: &str, ty: Vec<u8>, flags: u8, value: &[u8]) -> Vec<u8> {
        let mut v = fs(name);
        v.extend(ty);
        v.extend((value.len() as i32).to_le_bytes());
        v.push(flags);
        v.extend(value);
        v
    }

    use serde_json::json;

    fn sample5() -> Vec<u8> {
        let int = || tn("IntProperty", &[]);
        let name = || tn("NameProperty", &[]);
        let mut v = b"GVAS".to_vec();
        v.extend(3i32.to_le_bytes());
        v.extend(522i32.to_le_bytes());
        v.extend(1012i32.to_le_bytes());
        v.extend([5, 0, 4, 0, 4, 0, 1, 2, 3, 4]);
        v.extend(fs("++UE5+Release-5.4"));
        v.extend(3i32.to_le_bytes());
        v.extend(0i32.to_le_bytes());
        v.extend(fs("/Game/Save/BP_Save.BP_Save_C"));
        v.push(0);
        v.extend(prop5("Gold", int(), 0, &1500i32.to_le_bytes()));
        v.extend(prop5(
            "Hardcore",
            tn("BoolProperty", &[]),
            TAG_BOOL_TRUE,
            &[],
        ));
        // Map<Name, Int>
        let mut inv = 0i32.to_le_bytes().to_vec();
        inv.extend(2i32.to_le_bytes());
        for (k, n) in [("Potion", 3i32), ("Elixir", 1)] {
            inv.extend(fs(k));
            inv.extend(n.to_le_bytes());
        }
        v.extend(prop5(
            "Inventory",
            tn("MapProperty", &[name(), int()]),
            0,
            &inv,
        ));
        // Map<Name, Struct{Level: Int, Title: Str, Spot: Vector}>
        let mut hero = prop5("Level", int(), 0, &7i32.to_le_bytes());
        hero.extend(prop5("Title", tn("StrProperty", &[]), 0, &fs("Novice")));
        let st = |s: &str, pkg: &str| tn("StructProperty", &[tn(s, &[tn(pkg, &[])])]);
        let spot: Vec<u8> = [1.0f64, 2.0, 3.0]
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        hero.extend(prop5(
            "Spot",
            st("Vector", "/Script/CoreUObject"),
            TAG_NATIVE,
            &spot,
        ));
        hero.extend(fs("None"));
        let mut chars = 0i32.to_le_bytes().to_vec();
        chars.extend(1i32.to_le_bytes());
        chars.extend(fs("Hero"));
        chars.extend(hero);
        v.extend(prop5(
            "Chars",
            tn("MapProperty", &[name(), st("S_Char", "/Game/S_Char")]),
            0,
            &chars,
        ));
        let mut arr = 3i32.to_le_bytes().to_vec();
        for x in [10i32, 20, 30] {
            arr.extend(x.to_le_bytes());
        }
        v.extend(prop5("Scores", tn("ArrayProperty", &[int()]), 0, &arr));
        v.extend(fs("None"));
        v.extend(0i32.to_le_bytes());
        v
    }

    #[test]
    fn ue54_full_type_tags_with_maps() {
        let raw = sample5();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        for (p, v) in [
            ("/Gold", json!(1500)),
            ("/Hardcore", json!(true)),
            ("/Inventory/Potion", json!(3)),
            ("/Inventory/Elixir", json!(1)),
            ("/Chars/Hero/Level", json!(7)),
            ("/Chars/Hero/Title", json!("Novice")),
            ("/Chars/Hero/Spot/2", json!(3.0)),
            ("/Scores/2", json!(30)),
        ] {
            assert_eq!(doc.view.pointer(p).unwrap(), &v, "{p}");
        }
        doc.set("/Inventory/Potion", json!(99)).unwrap();
        doc.set("/Hardcore", json!(false)).unwrap();
        doc.set("/Chars/Hero/Title", json!("Grand Champion"))
            .unwrap();
        doc.set("/Chars/Hero/Level", json!(80)).unwrap();
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        for (p, v) in [
            ("/Gold", json!(1500)),
            ("/Hardcore", json!(false)),
            ("/Inventory/Potion", json!(99)),
            ("/Inventory/Elixir", json!(1)),
            ("/Chars/Hero/Level", json!(80)),
            ("/Chars/Hero/Title", json!("Grand Champion")),
            ("/Scores/1", json!(20)),
        ] {
            assert_eq!(again.view.pointer(p).unwrap(), &v, "{p}");
        }
    }
}

#[cfg(test)]
mod research {
    /// Research: why a GVAS file doesn't parse (SAVE_EDITOR_GVAS = file).
    #[test]
    #[ignore]
    fn gvas_error() {
        let Ok(f) = std::env::var("SAVE_EDITOR_GVAS") else {
            return;
        };
        match super::parse(&std::fs::read(f).unwrap()) {
            Ok(t) => println!("OK {}", t.kind_name),
            Err(e) => println!("ERR {e}"),
        }
    }
}
