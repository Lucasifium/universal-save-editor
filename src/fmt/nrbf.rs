//! .NET BinaryFormatter ([MS-NRBF]) — common in older Unity and XNA games.
//!
//! Records are parsed into an object table; the view follows references from the
//! root object (List<T> and Dictionary<K,V> are shown as plain lists / maps).
//! Editable primitives and strings keep their byte ranges and are patched in place —
//! the format has no outer length fields, so this is always safe.

use crate::tree::{child, int_in, num, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Prim {
    Bool,
    U8,
    I8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Str,
}

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    prim: Prim,
}

#[derive(Clone, Debug)]
enum Val {
    /// A primitive: its display value and leaf (None = read-only).
    Prim(Value, Option<usize>),
    Ref(i32),
    Null,
}

#[derive(Clone, Debug)]
enum Obj {
    Class {
        name: String,
        members: Vec<(String, Val)>,
    },
    Str(Value, usize),
    Array(Vec<Val>),
}

#[derive(Clone, Debug)]
struct ClassMeta {
    name: String,
    names: Vec<String>,
    /// (binary type, primitive type for Primitive/PrimitiveArray) per member; None = untyped.
    types: Option<Vec<(u8, u8)>>,
}

struct P<'a> {
    d: &'a [u8],
    pos: usize,
    leaves: Vec<Leaf>,
    objects: HashMap<i32, Obj>,
    metas: HashMap<i32, ClassMeta>,
    root: i32,
}

enum Rec {
    Val(Val),
    Nulls(usize),
    End,
    Skip,
}

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
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn count(&mut self) -> Result<usize, String> {
        let n = self.i32()?;
        if n < 0 || n as usize > self.d.len() * 8 + 64 {
            return Err("bad count".into());
        }
        Ok(n as usize)
    }
    fn lps_len(&mut self) -> Result<usize, String> {
        let mut n: usize = 0;
        for i in 0..5 {
            let b = self.u8()?;
            n |= ((b & 0x7f) as usize) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(n);
            }
        }
        Err("bad string length".into())
    }
    fn lps(&mut self) -> Result<String, String> {
        let n = self.lps_len()?;
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
    fn leaf(&mut self, start: usize, prim: Prim) -> usize {
        self.leaves.push(Leaf {
            start,
            end: self.pos,
            prim,
        });
        self.leaves.len() - 1
    }
    /// Read one primitive of type `pt`.
    fn prim(&mut self, pt: u8) -> Result<Val, String> {
        let start = self.pos;
        macro_rules! n {
            ($t:ty, $p:expr) => {{
                let v =
                    <$t>::from_le_bytes(self.take(std::mem::size_of::<$t>())?.try_into().unwrap());
                let id = self.leaf(start, $p);
                Val::Prim(Value::from(v), Some(id))
            }};
        }
        Ok(match pt {
            1 => {
                let b = self.u8()? != 0;
                let id = self.leaf(start, Prim::Bool);
                Val::Prim(Value::Bool(b), Some(id))
            }
            2 => n!(u8, Prim::U8),
            10 => n!(i8, Prim::I8),
            7 => n!(i16, Prim::I16),
            14 => n!(u16, Prim::U16),
            8 => n!(i32, Prim::I32),
            15 => n!(u32, Prim::U32),
            9 => n!(i64, Prim::I64),
            16 => n!(u64, Prim::U64),
            11 => {
                let v = f32::from_le_bytes(self.take(4)?.try_into().unwrap());
                let id = self.leaf(start, Prim::F32);
                Val::Prim(crate::tree::f32_value(v), Some(id))
            }
            6 => {
                let v = f64::from_le_bytes(self.take(8)?.try_into().unwrap());
                let id = self.leaf(start, Prim::F64);
                Val::Prim(num(v), Some(id))
            }
            3 => {
                // Char: one UTF-8 character.
                let first = self.u8()?;
                let extra = match first {
                    0..=0x7f => 0,
                    0xc0..=0xdf => 1,
                    0xe0..=0xef => 2,
                    _ => 3,
                };
                let rest = self.take(extra)?.to_vec();
                let s = String::from_utf8_lossy(&[&[first], &rest[..]].concat()).into_owned();
                Val::Prim(Value::String(s), None)
            }
            5 => Val::Prim(Value::String(self.lps()?), None), // Decimal (as text)
            12 | 13 => {
                let v = i64::from_le_bytes(self.take(8)?.try_into().unwrap());
                Val::Prim(
                    Value::String(format!("{} ticks", v & 0x3FFF_FFFF_FFFF_FFFF)),
                    None,
                )
            }
            17 => Val::Null,
            18 => {
                let s = self.lps()?;
                let id = self.leaf(start, Prim::Str);
                Val::Prim(Value::String(s), Some(id))
            }
            _ => return Err(format!("unknown primitive type {pt}")),
        })
    }
    fn class_info(&mut self) -> Result<(i32, String, Vec<String>), String> {
        let id = self.i32()?;
        let name = self.lps()?;
        let n = self.count()?;
        let names = (0..n).map(|_| self.lps()).collect::<Result<_, _>>()?;
        Ok((id, name, names))
    }
    fn member_types(&mut self, n: usize) -> Result<Vec<(u8, u8)>, String> {
        let bts: Vec<u8> = self.take(n)?.to_vec();
        let mut out = Vec::with_capacity(n);
        for bt in bts {
            let extra = match bt {
                0 | 7 => self.u8()?,
                3 => {
                    self.lps()?;
                    0
                }
                4 => {
                    self.lps()?;
                    self.i32()?;
                    0
                }
                1 | 2 | 5 | 6 => 0,
                _ => return Err(format!("unknown member type {bt}")),
            };
            out.push((bt, extra));
        }
        Ok(out)
    }
    fn members(&mut self, meta: &ClassMeta) -> Result<Vec<(String, Val)>, String> {
        let mut out = Vec::with_capacity(meta.names.len());
        for (i, name) in meta.names.iter().enumerate() {
            let v = match &meta.types {
                Some(t) if t[i].0 == 0 => self.prim(t[i].1)?,
                _ => match self.record()? {
                    Rec::Val(v) => v,
                    _ => return Err("unexpected record in member".into()),
                },
            };
            out.push((name.clone(), v));
        }
        Ok(out)
    }
    fn class_record(&mut self, id: i32, meta: ClassMeta) -> Result<Rec, String> {
        let members = self.members(&meta)?;
        self.objects.insert(
            id,
            Obj::Class {
                name: meta.name.clone(),
                members,
            },
        );
        self.metas.insert(id, meta);
        Ok(Rec::Val(Val::Ref(id)))
    }
    fn elements(&mut self, n: usize) -> Result<Vec<Val>, String> {
        let mut out = Vec::with_capacity(n.min(4096));
        while out.len() < n {
            match self.record()? {
                Rec::Val(v) => out.push(v),
                Rec::Nulls(k) => out.extend(std::iter::repeat_n(Val::Null, k)),
                _ => return Err("unexpected record in array".into()),
            }
        }
        if out.len() != n {
            return Err("array length mismatch".into());
        }
        Ok(out)
    }
    fn record(&mut self) -> Result<Rec, String> {
        let t = self.u8()?;
        Ok(match t {
            0 => {
                self.root = self.i32()?;
                self.take(12)?;
                Rec::Skip
            }
            12 => {
                self.i32()?;
                self.lps()?;
                Rec::Skip
            }
            1 => {
                let id = self.i32()?;
                let meta_id = self.i32()?;
                let meta = self
                    .metas
                    .get(&meta_id)
                    .cloned()
                    .ok_or("unknown class reference")?;
                self.class_record(id, meta)?
            }
            2 | 3 => {
                let (id, name, names) = self.class_info()?;
                if t == 3 {
                    self.i32()?;
                }
                self.class_record(
                    id,
                    ClassMeta {
                        name,
                        names,
                        types: None,
                    },
                )?
            }
            4 | 5 => {
                let (id, name, names) = self.class_info()?;
                let types = self.member_types(names.len())?;
                if t == 5 {
                    self.i32()?;
                }
                self.class_record(
                    id,
                    ClassMeta {
                        name,
                        names,
                        types: Some(types),
                    },
                )?
            }
            6 => {
                let id = self.i32()?;
                let start = self.pos;
                let s = self.lps()?;
                let leaf = self.leaf(start, Prim::Str);
                self.objects.insert(id, Obj::Str(Value::String(s), leaf));
                Rec::Val(Val::Ref(id))
            }
            7 => {
                let id = self.i32()?;
                let kind = self.u8()?;
                let rank = self.count()?;
                let lens: Vec<usize> = (0..rank).map(|_| self.count()).collect::<Result<_, _>>()?;
                if matches!(kind, 3..=5) {
                    self.take(4 * rank)?;
                }
                let bt = self.u8()?;
                let extra = self.member_types_one(bt)?;
                let n: usize = lens.iter().product();
                let items = if bt == 0 {
                    (0..n).map(|_| self.prim(extra)).collect::<Result<_, _>>()?
                } else {
                    self.elements(n)?
                };
                self.objects.insert(id, Obj::Array(items));
                Rec::Val(Val::Ref(id))
            }
            8 => {
                let pt = self.u8()?;
                Rec::Val(self.prim(pt)?)
            }
            9 => Rec::Val(Val::Ref(self.i32()?)),
            10 => Rec::Val(Val::Null),
            11 => Rec::End,
            13 => Rec::Nulls(self.u8()? as usize),
            14 => Rec::Nulls(self.count()?),
            15 => {
                let id = self.i32()?;
                let n = self.count()?;
                let pt = self.u8()?;
                let items = (0..n).map(|_| self.prim(pt)).collect::<Result<_, _>>()?;
                self.objects.insert(id, Obj::Array(items));
                Rec::Val(Val::Ref(id))
            }
            16 | 17 => {
                let id = self.i32()?;
                let n = self.count()?;
                let items = self.elements(n)?;
                self.objects.insert(id, Obj::Array(items));
                Rec::Val(Val::Ref(id))
            }
            _ => return Err(format!("unsupported record type {t}")),
        })
    }
    fn member_types_one(&mut self, bt: u8) -> Result<u8, String> {
        Ok(match bt {
            0 | 7 => self.u8()?,
            3 => {
                self.lps()?;
                0
            }
            4 => {
                self.lps()?;
                self.i32()?;
                0
            }
            _ => 0,
        })
    }
}

/// "<Gold>k__BackingField" → "Gold".
fn member_name(n: &str) -> String {
    if let Some(rest) = n.strip_prefix('<') {
        if let Some(i) = rest.find(">k__BackingField") {
            return rest[..i].to_string();
        }
    }
    n.to_string()
}

struct V<'a> {
    objects: &'a HashMap<i32, Obj>,
    editable: HashMap<String, usize>,
    visiting: HashSet<i32>,
}

impl V<'_> {
    fn val(&mut self, v: &Val, ptr: &str, depth: usize) -> Value {
        match v {
            Val::Null => Value::Null,
            Val::Prim(x, leaf) => {
                if let Some(l) = leaf {
                    self.editable.insert(ptr.to_string(), *l);
                }
                x.clone()
            }
            Val::Ref(id) => self.obj(*id, ptr, depth),
        }
    }

    fn key_text(&self, v: &Val) -> String {
        match v {
            Val::Prim(x, _) => x
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| x.to_string()),
            Val::Ref(id) => match self.objects.get(id) {
                Some(Obj::Str(s, _)) => s.as_str().unwrap_or("").to_string(),
                _ => format!("#{id}"),
            },
            Val::Null => "null".into(),
        }
    }

    fn obj(&mut self, id: i32, ptr: &str, depth: usize) -> Value {
        if depth > 200 || !self.visiting.insert(id) {
            return Value::String("<reference>".into());
        }
        let out = match self.objects.get(&id).cloned() {
            None => Value::String("<missing object>".into()),
            Some(Obj::Str(s, leaf)) => {
                self.editable.insert(ptr.to_string(), leaf);
                s
            }
            Some(Obj::Array(items)) => Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| self.val(v, &format!("{ptr}/{i}"), depth + 1))
                    .collect(),
            ),
            Some(Obj::Class { name, members }) => self.class(&name, &members, ptr, depth),
        };
        self.visiting.remove(&id);
        out
    }

    fn member<'m>(members: &'m [(String, Val)], name: &str) -> Option<&'m Val> {
        members.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    fn class(&mut self, name: &str, members: &[(String, Val)], ptr: &str, depth: usize) -> Value {
        // List<T>: show the first _size items of _items.
        if name.starts_with("System.Collections.Generic.List`1") {
            if let (Some(Val::Ref(items)), Some(Val::Prim(size, _))) = (
                Self::member(members, "_items"),
                Self::member(members, "_size"),
            ) {
                let size = size.as_u64().unwrap_or(0) as usize;
                if let Some(Obj::Array(elems)) = self.objects.get(items).cloned() {
                    return Value::Array(
                        elems
                            .iter()
                            .take(size)
                            .enumerate()
                            .map(|(i, v)| self.val(v, &format!("{ptr}/{i}"), depth + 1))
                            .collect(),
                    );
                }
            }
        }
        // Dictionary<K,V>: KeyValuePairs → map.
        if name.starts_with("System.Collections.Generic.Dictionary`2") {
            if let Some(Val::Ref(pairs)) = Self::member(members, "KeyValuePairs") {
                if let Some(Obj::Array(elems)) = self.objects.get(pairs).cloned() {
                    let mut m = Map::new();
                    for e in &elems {
                        let Val::Ref(pid) = e else { continue };
                        let Some(Obj::Class { members: kv, .. }) = self.objects.get(pid).cloned()
                        else {
                            continue;
                        };
                        let (Some(k), Some(v)) =
                            (Self::member(&kv, "key"), Self::member(&kv, "value"))
                        else {
                            continue;
                        };
                        let key = unique_key(&m, &self.key_text(k));
                        let val = self.val(v, &child(ptr, &key), depth + 1);
                        m.insert(key, val);
                    }
                    return Value::Object(m);
                }
            }
        }
        let mut m = Map::new();
        for (n, v) in members {
            let key = unique_key(&m, &member_name(n));
            let val = self.val(v, &child(ptr, &key), depth + 1);
            m.insert(key, val);
        }
        Value::Object(m)
    }
}

#[derive(Clone, Debug)]
struct NrbfBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

fn lps_bytes(s: &str) -> Vec<u8> {
    let mut n = s.len();
    let mut out = Vec::new();
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            break;
        }
        out.push(b | 0x80);
    }
    out.extend_from_slice(s.as_bytes());
    out
}

fn encode(p: Prim, v: &Value) -> Result<Vec<u8>, String> {
    macro_rules! int {
        ($t:ty) => {
            (int_in(v, <$t>::MIN as i128, <$t>::MAX as i128, "This number")? as $t)
                .to_le_bytes()
                .to_vec()
        };
    }
    Ok(match p {
        Prim::Bool => vec![v.as_bool().ok_or("must be on/off")? as u8],
        Prim::U8 => int!(u8),
        Prim::I8 => int!(i8),
        Prim::I16 => int!(i16),
        Prim::U16 => int!(u16),
        Prim::I32 => int!(i32),
        Prim::U32 => int!(u32),
        Prim::I64 => int!(i64),
        Prim::U64 => int!(u64),
        Prim::F32 => (v.as_f64().ok_or("must be a number")? as f32)
            .to_le_bytes()
            .to_vec(),
        Prim::F64 => v.as_f64().ok_or("must be a number")?.to_le_bytes().to_vec(),
        Prim::Str => lps_bytes(v.as_str().ok_or("must be text")?),
    })
}

impl Backend for NrbfBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            p.add(l.start, l.end, encode(l.prim, v)?);
        }
        p.apply(&self.raw)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if self.leaves[leaf].prim == Prim::F32 {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode(self.leaves[leaf].prim, v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(d: &[u8]) -> bool {
    // SerializationHeader record: type 0, rootId, headerId, MajorVersion 1, MinorVersion 0.
    d.len() > 17 && d[0] == 0 && d[9..13] == [1, 0, 0, 0] && d[13..17] == [0, 0, 0, 0]
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut p = P {
        d,
        pos: 0,
        leaves: vec![],
        objects: HashMap::new(),
        metas: HashMap::new(),
        root: 0,
    };
    // One message, or (Green Hell) several written one after another, then zero padding.
    let mut messages: Vec<(String, Value)> = vec![];
    let mut editable = HashMap::new();
    loop {
        p.objects.clear();
        p.metas.clear();
        loop {
            if let Rec::End = p.record()? {
                break;
            }
        }
        let class = match p.objects.get(&p.root) {
            Some(Obj::Class { name, .. }) => name.rsplit('.').next().unwrap_or(name).to_string(),
            _ => String::new(),
        };
        let n = messages.len();
        let mut v = V {
            objects: &p.objects,
            editable: HashMap::new(),
            visiting: HashSet::new(),
        };
        let view = v.obj(p.root, &format!("/{n}"), 0);
        editable.extend(v.editable);
        messages.push((class, view));
        let rest = &d[p.pos..];
        if rest.iter().all(|&b| b == 0) {
            break;
        }
        if !looks(rest) || messages.len() > 100_000 {
            return Err(format!("extra data after the message at {}", p.pos));
        }
    }
    let backend = NrbfBackend {
        raw: d.to_vec(),
        leaves: p.leaves,
    };
    if messages.len() == 1 {
        let (class, view) = messages.pop().unwrap();
        let editable = editable
            .into_iter()
            .map(|(k, l)| (k["/0".len()..].to_string(), l))
            .collect();
        let name = if class.is_empty() {
            ".NET BinaryFormatter".to_string()
        } else {
            format!(".NET BinaryFormatter ({class})")
        };
        return Ok(TreeDoc::new(view, editable, name, Box::new(backend)));
    }
    let view = Value::Array(messages.into_iter().map(|(_, v)| v).collect());
    Ok(TreeDoc::new(
        view,
        editable,
        ".NET BinaryFormatter stream",
        Box::new(backend),
    ))
}
