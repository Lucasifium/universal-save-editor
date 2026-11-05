//! RE Engine field-based saves (what's inside a decrypted Onimusha: Way of the Sword,
//! Dragon's Dogma 2 or Monster Hunter Wilds save, among others).
//!
//! The file is a run of named objects: a name hash, then a class (field count, class name
//! hash, fields). A field is its name hash, a type (via.reflection.TypeKind, or -1 for an
//! array) and a value: classes nest; a string is a u32 length and UTF-16 text; anything else
//! is a u32 size, then the value aligned to its size. Arrays give the member type and size,
//! a count and whether the members are values or classes (classes may be preceded by
//! 0xFFEEFFEE and one class hash per member). Everything is 4-byte aligned between fields.
//! Names are MurmurHash3 (x86, 32-bit, seed 0xFFFFFFFF) of the UTF-8 name; the ones known
//! are in `ree_names`. Layout as documented by kvasszn's ree-save-editor.
//!
//! Numbers, flags and enums are edited where they are, so the file never changes size.

use super::ree_names::NAMES;
use crate::tree::{esc, int_in, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

/// MurmurHash3_x86_32.
pub fn murmur3(data: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xCC9E_2D51;
    const C2: u32 = 0x1B87_3593;
    let mut h = seed;
    let mut chunks = data.chunks_exact(4);
    for c in &mut chunks {
        let k = u32::from_le_bytes(c.try_into().unwrap()).wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
        h = (h ^ k).rotate_left(13).wrapping_mul(5).wrapping_add(0xE654_6B64);
    }
    let tail = chunks.remainder();
    if !tail.is_empty() {
        let mut k = 0u32;
        for (i, &b) in tail.iter().enumerate() {
            k |= (b as u32) << (8 * i);
        }
        h ^= k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
    }
    h ^= data.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^ (h >> 16)
}

pub fn name_hash(name: &str) -> u32 {
    murmur3(name.as_bytes(), 0xFFFF_FFFF)
}

fn name(hash: u32) -> String {
    static N: OnceLock<HashMap<u32, &'static str>> = OnceLock::new();
    let names = N.get_or_init(|| NAMES.iter().map(|n| (name_hash(n), *n)).collect());
    names.get(&hash).map_or_else(|| format!("#{hash:08x}"), |n| n.to_string())
}

/// The encrypted file's header, which value alignment counts from.
const FILE_HEADER: usize = 16;
const ARRAY: i32 = -1;
const ENUM: i32 = 1;
const BOOL: i32 = 2;
const F32: i32 = 11;
const F64: i32 = 12;
const STRING: i32 = 0xF;
const STRUCT: i32 = 0x10;
const CLASS: i32 = 0x11;

/// An editable number in the file: where, its type and its size in bytes.
#[derive(Clone, Copy, Debug)]
struct Leaf {
    at: usize,
    ty: i32,
    size: usize,
}

impl Leaf {
    fn signed(&self) -> bool {
        matches!(self.ty, ENUM | 3 | 5 | 7 | 9)
    }
}

struct Reader<'a> {
    d: &'a [u8],
    pos: usize,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
    /// Name hashes seen and how many times (for the "unknown names" research aid).
    seen: HashMap<u32, usize>,
}

impl Reader<'_> {
    fn align(&mut self, n: usize) {
        if n > 1 {
            self.pos = (self.pos + n - 1) & !(n - 1);
        }
    }
    /// Where a value of `size` bytes starts: the game rounds the offset in the whole file
    /// (counting the 16-byte DSSS header in front of this data) with
    /// `(offset + size - 1) & !(size - 1)` — for sizes like 24 that isn't a plain alignment.
    fn align_value(&mut self, size: usize) {
        if size > 1 {
            let at = self.pos + FILE_HEADER;
            self.pos = ((at + size - 1) & !(size - 1)) - FILE_HEADER;
        }
    }
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let b = self.d.get(self.pos..self.pos + n).ok_or("unexpected end of save")?;
        self.pos += n;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u32()? as i32)
    }

    /// A number of `size` bytes at the cursor, as a view value, made editable at `ptr`.
    fn number(&mut self, ty: i32, size: usize, ptr: &str) -> Result<Value, String> {
        if !matches!(size, 1 | 2 | 4 | 8) {
            return Err(format!("unexpected value size {size}"));
        }
        self.align_value(size);
        let at = self.pos;
        let b = self.take(size)?;
        let mut raw = [0u8; 8];
        raw[..size].copy_from_slice(b);
        let u = u64::from_le_bytes(raw);
        let leaf = Leaf { at, ty, size };
        let v = match ty {
            BOOL => Value::Bool(u != 0),
            F32 if size == 4 => crate::tree::f32_value(f32::from_bits(u as u32)),
            F64 if size == 8 => serde_json::Number::from_f64(f64::from_bits(u)).map_or(Value::Null, Value::Number),
            F32 | F64 => return Err("odd float size".into()),
            STRUCT => return Err("struct as a number".into()),
            _ if leaf.signed() => {
                let shift = 64 - 8 * size as u32;
                Value::from(((u << shift) as i64) >> shift)
            }
            _ => Value::from(u),
        };
        if !(ty == BOOL && u > 1) {
            self.editable.insert(ptr.to_string(), self.leaves.len());
            self.leaves.push(leaf);
        }
        Ok(v)
    }

    fn string(&mut self) -> Result<Value, String> {
        self.align(4);
        let n = self.u32()? as usize;
        let units: Vec<u16> = self.take(n * 2)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Ok(Value::from(String::from_utf16_lossy(&units).trim_end_matches('\0').to_string()))
    }

    /// One value of a value array, or of a field after its size.
    fn sized(&mut self, ty: i32, size: usize, ptr: &str) -> Result<Value, String> {
        match ty {
            STRUCT => {
                self.align_value(size.max(1));
                let b = self.take(size)?;
                Ok(Value::from(b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")))
            }
            ARRAY => self.array(ptr),
            _ => self.number(ty, size, ptr),
        }
    }

    fn value(&mut self, ty: i32, ptr: &str) -> Result<Value, String> {
        match ty {
            ARRAY => self.array(ptr),
            CLASS => self.class(ptr),
            STRING => self.string(),
            0 => Err(format!("unknown field type at {:#x}", self.pos)),
            1..=0x10 => {
                self.align(4);
                let size = self.u32()? as usize;
                self.sized(ty, size, ptr)
            }
            _ => Err(format!("field type {ty} at {:#x}", self.pos)),
        }
    }

    fn array(&mut self, ptr: &str) -> Result<Value, String> {
        self.align(4);
        let member = self.i32()?;
        let size = self.u32()? as usize;
        let len = self.u32()? as usize;
        let classes = self.i32()? == 1;
        if len > self.d.len() {
            return Err("array too long".into());
        }
        if classes && self.d.get(self.pos..self.pos + 4) == Some(&[0xEE, 0xFF, 0xEE, 0xFF][..]) {
            self.pos += 4 + 4 * len;
        }
        let mut out = Vec::with_capacity(len.min(4096));
        for i in 0..len {
            let p = format!("{ptr}/{i}");
            out.push(if classes {
                self.class(&p)?
            } else if member == STRING {
                self.string()?
            } else {
                self.sized(member, size, &p)?
            });
        }
        self.align(4);
        Ok(Value::Array(out))
    }

    fn class(&mut self, ptr: &str) -> Result<Value, String> {
        let n = self.u32()? as usize;
        let _type_hash = self.u32()?;
        if n > 100_000 {
            return Err("too many fields".into());
        }
        let mut m = Map::new();
        for _ in 0..n {
            let h = self.u32()?;
            *self.seen.entry(h).or_default() += 1;
            let ty = self.i32()?;
            let key = unique_key(&m, &name(h));
            let v = self.value(ty, &format!("{ptr}/{}", esc(&key)))?;
            self.align(4);
            m.insert(key, v);
        }
        Ok(Value::Object(m))
    }
}

/// Quick check before parsing: a name, a sensible field count, and a known field type.
pub fn looks(d: &[u8]) -> bool {
    let u = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    d.len() >= 24 && matches!(u(4), Some(1..=4096)) && matches!(u(16).map(|t| t as i32), Some(-1..=0x11))
}

fn read(d: &[u8]) -> Result<(Value, Vec<Leaf>, HashMap<String, usize>, HashMap<u32, usize>), String> {
    let mut r = Reader { d, pos: 0, leaves: vec![], editable: HashMap::new(), seen: HashMap::new() };
    let mut root = Map::new();
    while r.pos + 8 <= d.len() {
        let h = r.u32()?;
        let key = unique_key(&root, &name(h));
        let v = r.class(&format!("/{}", esc(&key)))?;
        root.insert(key, v);
    }
    if r.pos != d.len() || root.is_empty() {
        return Err("not an RE Engine save".into());
    }
    Ok((Value::Object(root), r.leaves, r.editable, r.seen))
}

/// Name hashes in `d` that the name table doesn't know, most used first.
#[cfg(test)]
pub fn unknown_names(d: &[u8]) -> Vec<(u32, usize)> {
    let Ok((_, _, _, seen)) = read(d) else { return vec![] };
    let mut v: Vec<(u32, usize)> = seen.into_iter().filter(|(h, _)| name(*h).starts_with('#')).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}

#[derive(Clone, Debug)]
struct ReeBackend {
    data: std::sync::Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
}

impl ReeBackend {
    fn encode(leaf: &Leaf, v: &Value) -> Result<Vec<u8>, String> {
        let bytes: Vec<u8> = match leaf.ty {
            BOOL => vec![u8::from(v.as_bool().ok_or("tick or untick it")?)],
            F32 => (v.as_f64().ok_or("expected a number")? as f32).to_le_bytes().to_vec(),
            F64 => v.as_f64().ok_or("expected a number")?.to_le_bytes().to_vec(),
            _ => {
                let bits = 8 * leaf.size as u32;
                let (min, max) = if leaf.signed() {
                    (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
                } else {
                    (0, (1i128 << bits) - 1)
                };
                let n = int_in(v, min, max, "This value")?;
                (n as u64).to_le_bytes()[..leaf.size].to_vec()
            }
        };
        if bytes.len() != leaf.size {
            return Err("this value can't change size".into());
        }
        Ok(bytes)
    }
}

impl Backend for ReeBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = self.data.as_ref().clone();
        for (leaf, v) in changes {
            let l = &self.leaves[*leaf];
            out[l.at..l.at + l.size].copy_from_slice(&Self::encode(l, v)?);
        }
        Ok(out)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if self.leaves[leaf].ty == F32 {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let l = &self.leaves[leaf];
        if matches!(l.ty, F32 | F64) && !v.as_f64().is_some_and(f64::is_finite) {
            return Err("expected a number".into());
        }
        Self::encode(l, v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (view, leaves, editable, _) = read(d)?;
    let backend = ReeBackend { data: std::sync::Arc::new(d.to_vec()), leaves };
    Ok(TreeDoc::new(view, editable, "RE Engine save", Box::new(backend)).with_note(
        "Fields whose names aren't known yet show as #hash. Numbers, ticks and choices can be edited; \
         use Tools \u{203A} Compare with another save to find a value you know.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_reference() {
        // MurmurHash3_x86_32 reference values.
        assert_eq!(murmur3(b"", 0), 0);
        assert_eq!(murmur3(b"hello", 0), 0x248B_FA47);
        assert_eq!(murmur3(b"The quick brown fox jumps over the lazy dog", 0x9747_B28C), 0x2FA8_26CD);
    }

    /// Builds a tiny save: one object with a u32, a bool, a string and an array of i32.
    fn sample() -> Vec<u8> {
        let mut d = vec![];
        let p = |d: &mut Vec<u8>, v: u32| d.extend(v.to_le_bytes());
        p(&mut d, name_hash("_System"));
        p(&mut d, 4);
        p(&mut d, 0x1234);
        p(&mut d, name_hash("Money"));
        p(&mut d, 8);
        p(&mut d, 4);
        p(&mut d, 1500);
        p(&mut d, name_hash("IsActive"));
        p(&mut d, BOOL as u32);
        p(&mut d, 1);
        d.extend([1, 0, 0, 0]);
        p(&mut d, name_hash("Title"));
        p(&mut d, STRING as u32);
        p(&mut d, 3);
        d.extend([b'O', 0, b'n', 0, b'i', 0, 0, 0]);
        p(&mut d, name_hash("Values"));
        p(&mut d, ARRAY as u32);
        p(&mut d, 7);
        p(&mut d, 4);
        p(&mut d, 2);
        p(&mut d, 0);
        p(&mut d, (-5i32) as u32);
        p(&mut d, 9);
        d
    }

    #[test]
    fn reads_and_edits() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view["_System"]["Money"], 1500);
        assert_eq!(doc.view["_System"]["IsActive"], true);
        assert_eq!(doc.view["_System"]["Title"], "Oni");
        assert_eq!(doc.view["_System"]["Values"][0], -5);
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/_System/Money", Value::from(99999)).unwrap();
        doc.set("/_System/Values/0", Value::from(-7)).unwrap();
        assert!(doc.set("/_System/Values/1", Value::from(1i64 << 40)).is_err());
        let back = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(back.view, doc.view);
    }

    /// Research aid: parses RE_SAVE (a decrypted save) and lists the unknown names.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("RE_SAVE").expect("RE_SAVE")).unwrap();
        let doc = parse(&d).unwrap();
        println!("{} editable values", doc.editable.len());
        let unknown = unknown_names(&d);
        println!("{} unknown names, top: {:x?}", unknown.len(), &unknown[..unknown.len().min(20)]);
        if let Ok(out) = std::env::var("RE_UNKNOWN_OUT") {
            std::fs::write(out, unknown.iter().map(|(h, _)| format!("{h:08x}
")).collect::<String>()).unwrap();
        }
        let mut keys = vec![];
        fn walk(v: &Value, p: &str, keys: &mut Vec<String>, depth: usize) {
            if let Value::Object(m) = v {
                for (k, x) in m {
                    if depth < 4 {
                        keys.push(format!("{p}/{k}{}", if x.is_object() { "" } else { &"" }));
                        walk(x, &format!("{p}/{k}"), keys, depth + 1);
                    }
                }
            }
        }
        walk(&doc.view, "", &mut keys, 0);
        for k in keys.iter().filter(|k| !k.contains('#')).take(120) {
            println!("{k}");
        }
    }
}
