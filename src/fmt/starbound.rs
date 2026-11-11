//! Starbound "versioned JSON" files (`SBVJ01`): characters (`<uuid>.player`), their
//! metadata and other single-object stores.
//!
//! The file is `SBVJ01`, a name (`PlayerEntity`, `PlayerMetadata`…: a varint length then
//! UTF-8), a flag byte and, when it's set, a big-endian i32 version, then one value in the
//! game's binary JSON ("SBON"): a type byte, 1 null, 2 big-endian f64, 3 bool byte,
//! 4 zigzag varint, 5 string, 6 list (varint count, values), 7 map (varint count, key
//! string and value each). Varints are big-endian groups of 7 bits, high bit = more.
//!
//! Lists and maps are prefixed by their item count, never by a byte size, so a value can
//! be rewritten on its own even when its new encoding is longer or shorter: each edited
//! scalar is re-encoded in place and every other byte is kept.
//!
//! Format facts from blixt/py-starbound (MIT), checked against real characters.

use crate::tree::{child, int_in, num, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const MAGIC: &[u8] = b"SBVJ01";
pub const KIND_PLAYER: &str = "Starbound character";

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(MAGIC)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ty {
    Float,
    Bool,
    Int,
    Text,
}

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    ty: Ty,
}

#[derive(Clone, Debug)]
struct SbBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    let mut tmp = vec![(v & 0x7f) as u8];
    v >>= 7;
    while v != 0 {
        tmp.push(0x80 | (v & 0x7f) as u8);
        v >>= 7;
    }
    tmp.reverse();
    out.extend_from_slice(&tmp);
}

fn zigzag(n: i64) -> u64 {
    if n >= 0 {
        (n as u64) << 1
    } else {
        (((-(n + 1)) as u64) << 1) | 1
    }
}

/// The bytes of one scalar (type byte included).
fn encode(ty: Ty, v: &Value) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    match ty {
        Ty::Float => {
            let f = v.as_f64().ok_or("must be a number")?;
            if !f.is_finite() {
                return Err("must be a number".into());
            }
            out.push(2);
            out.extend_from_slice(&f.to_be_bytes());
        }
        Ty::Bool => {
            out.push(3);
            out.push(u8::from(v.as_bool().ok_or("must be on/off")?));
        }
        Ty::Int => {
            let n = int_in(v, i64::MIN as i128, i64::MAX as i128, "this number")? as i64;
            out.push(4);
            put_varint(&mut out, zigzag(n));
        }
        Ty::Text => {
            let s = v.as_str().ok_or("must be text")?;
            out.push(5);
            put_varint(&mut out, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
    }
    Ok(out)
}

impl Backend for SbBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            p.add(l.start, l.end, encode(l.ty, v)?);
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode(self.leaves[leaf].ty, v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

struct Rd<'a> {
    d: &'a [u8],
    p: usize,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
}

impl Rd<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.d.get(self.p).ok_or("file ends early")?;
        self.p += 1;
        Ok(b)
    }

    fn varint(&mut self) -> Result<u64, String> {
        let mut v: u64 = 0;
        for _ in 0..10 {
            let b = self.byte()?;
            if v >> 57 != 0 {
                return Err("number too large".into());
            }
            v = (v << 7) | u64::from(b & 0x7f);
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("number too long".into())
    }

    fn text(&mut self) -> Result<String, String> {
        let n = self.varint()? as usize;
        let end = self
            .p
            .checked_add(n)
            .filter(|&e| e <= self.d.len())
            .ok_or("text runs past the end")?;
        let s = std::str::from_utf8(&self.d[self.p..end])
            .map_err(|_| "text isn't UTF-8")?
            .to_string();
        self.p = end;
        Ok(s)
    }

    fn leaf(&mut self, ptr: &str, start: usize, ty: Ty, v: Value) -> Value {
        self.editable.insert(ptr.to_string(), self.leaves.len());
        self.leaves.push(Leaf {
            start,
            end: self.p,
            ty,
        });
        v
    }

    fn value(&mut self, ptr: &str, depth: usize) -> Result<Value, String> {
        if depth > 256 {
            return Err("nested too deeply".into());
        }
        let start = self.p;
        Ok(match self.byte()? {
            1 => Value::Null,
            2 => {
                let b: [u8; 8] = self
                    .d
                    .get(self.p..self.p + 8)
                    .ok_or("file ends early")?
                    .try_into()
                    .unwrap();
                self.p += 8;
                let f = f64::from_be_bytes(b);
                if f.is_finite() {
                    self.leaf(ptr, start, Ty::Float, num(f))
                } else {
                    Value::String(f.to_string())
                }
            }
            3 => {
                let b = self.byte()? != 0;
                self.leaf(ptr, start, Ty::Bool, Value::Bool(b))
            }
            4 => {
                let z = self.varint()?;
                let n = if z & 1 == 1 {
                    -((z >> 1) as i64) - 1
                } else {
                    (z >> 1) as i64
                };
                self.leaf(ptr, start, Ty::Int, Value::from(n))
            }
            5 => {
                let s = self.text()?;
                self.leaf(ptr, start, Ty::Text, Value::String(s))
            }
            6 => {
                let n = self.varint()?;
                if n as usize > self.d.len() - self.p {
                    return Err("list longer than the file".into());
                }
                let mut out = Vec::with_capacity(n as usize);
                for i in 0..n {
                    out.push(self.value(&format!("{ptr}/{i}"), depth + 1)?);
                }
                Value::Array(out)
            }
            7 => {
                let n = self.varint()?;
                if n as usize > self.d.len() - self.p {
                    return Err("map longer than the file".into());
                }
                let mut out = Map::new();
                for _ in 0..n {
                    let k = self.text()?;
                    let key = unique_key(&out, &k);
                    let v = self.value(&child(ptr, &key), depth + 1)?;
                    out.insert(key, v);
                }
                Value::Object(out)
            }
            t => return Err(format!("unknown value type {t} at byte {start}")),
        })
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Starbound file".into());
    }
    let mut r = Rd {
        d,
        p: MAGIC.len(),
        leaves: vec![],
        editable: HashMap::new(),
    };
    let name = r.text()?;
    let version = if r.byte()? != 0 {
        let b: [u8; 4] = d
            .get(r.p..r.p + 4)
            .ok_or("file ends early")?
            .try_into()
            .unwrap();
        r.p += 4;
        Some(i32::from_be_bytes(b))
    } else {
        None
    };
    let view = r.value("", 0)?;
    if r.p != d.len() {
        return Err("data after the end of the value".into());
    }
    let kind = match name.as_str() {
        "PlayerEntity" => KIND_PLAYER.to_string(),
        n => format!("Starbound {n}"),
    };
    let mut doc = TreeDoc::new(
        view,
        r.editable,
        kind,
        Box::new(SbBackend {
            raw: d.to_vec(),
            leaves: r.leaves,
        }),
    );
    if name == "PlayerEntity" && version.is_some_and(|v| v < 20) {
        doc = doc.with_note("This character comes from an early-access version of Starbound (before 1.0); the game converts it when it loads, and the screen shows only what that older layout has.");
    }
    Ok(doc)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small synthetic file: SBVJ01, a name, a version and an SBON map.
    pub fn sample() -> Vec<u8> {
        fn s(out: &mut Vec<u8>, t: &str) {
            put_varint(out, t.len() as u64);
            out.extend_from_slice(t.as_bytes());
        }
        fn int(out: &mut Vec<u8>, n: i64) {
            out.push(4);
            put_varint(out, zigzag(n));
        }
        fn item(out: &mut Vec<u8>, name: &str, count: i64) {
            out.push(7);
            put_varint(out, 3);
            s(out, "id");
            out.push(5);
            s(out, "Item");
            s(out, "version");
            int(out, 8);
            s(out, "content");
            out.push(7);
            put_varint(out, 3);
            s(out, "parameters");
            out.extend_from_slice(&[7, 0]);
            s(out, "name");
            out.push(5);
            s(out, name);
            s(out, "count");
            int(out, count);
        }
        let mut d = MAGIC.to_vec();
        s(&mut d, "PlayerEntity");
        d.push(1);
        d.extend_from_slice(&31i32.to_be_bytes());
        d.push(7);
        put_varint(&mut d, 5);
        s(&mut d, "uuid");
        d.push(5);
        s(&mut d, "0123456789abcdef0123456789abcdef");
        s(&mut d, "identity");
        d.push(7);
        put_varint(&mut d, 2);
        s(&mut d, "name");
        d.push(5);
        s(&mut d, "Nova");
        s(&mut d, "species");
        d.push(5);
        s(&mut d, "human");
        s(&mut d, "inventory");
        d.push(7);
        put_varint(&mut d, 2);
        s(&mut d, "currencies");
        d.push(7);
        put_varint(&mut d, 2);
        s(&mut d, "money");
        int(&mut d, 60);
        s(&mut d, "essence");
        int(&mut d, -3);
        s(&mut d, "itemBags");
        d.push(7);
        put_varint(&mut d, 1);
        s(&mut d, "mainBag");
        d.push(6);
        put_varint(&mut d, 3);
        item(&mut d, "coalore", 12);
        d.push(1);
        item(&mut d, "torch", 40);
        s(&mut d, "statusController");
        d.push(7);
        put_varint(&mut d, 1);
        s(&mut d, "resourceValues");
        d.push(7);
        put_varint(&mut d, 2);
        s(&mut d, "health");
        d.push(2);
        d.extend_from_slice(&100.0f64.to_be_bytes());
        s(&mut d, "food");
        d.push(2);
        d.extend_from_slice(&52.5f64.to_be_bytes());
        s(&mut d, "log");
        d.push(7);
        put_varint(&mut d, 1);
        s(&mut d, "introComplete");
        d.extend_from_slice(&[3, 1]);
        d
    }

    #[test]
    fn synthetic_character() {
        let raw = sample();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.kind_name, KIND_PLAYER);
        assert_eq!(doc.to_bytes().unwrap(), raw);
        assert_eq!(
            doc.view.pointer("/inventory/currencies/essence").unwrap(),
            -3
        );
        assert_eq!(
            doc.view
                .pointer("/inventory/itemBags/mainBag/2/content/name")
                .unwrap(),
            "torch"
        );
        assert_eq!(
            doc.view
                .pointer("/statusController/resourceValues/food")
                .unwrap()
                .as_f64(),
            Some(52.5)
        );
        // 60 is one varint byte; 1,000,000 takes three: the file grows and the rest stays put.
        doc.set("/inventory/currencies/money", Value::from(1_000_000))
            .unwrap();
        doc.set(
            "/inventory/itemBags/mainBag/0/content/count",
            Value::from(999),
        )
        .unwrap();
        doc.set(
            "/statusController/resourceValues/health",
            Value::from(77.25),
        )
        .unwrap();
        doc.set("/identity/name", Value::from("Nova Prime"))
            .unwrap();
        doc.set("/log/introComplete", Value::Bool(false)).unwrap();
        assert!(doc
            .set("/inventory/currencies/money", Value::from("lots"))
            .is_err());
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(
            back.view.pointer("/inventory/currencies/money").unwrap(),
            1_000_000
        );
        assert_eq!(
            back.view
                .pointer("/inventory/itemBags/mainBag/0/content/count")
                .unwrap(),
            999
        );
        assert_eq!(
            back.view
                .pointer("/statusController/resourceValues/health")
                .unwrap()
                .as_f64(),
            Some(77.25)
        );
        assert_eq!(back.view.pointer("/identity/name").unwrap(), "Nova Prime");
        assert_eq!(back.view.pointer("/log/introComplete").unwrap(), false);
        assert_eq!(
            back.view.pointer("/inventory/currencies/essence").unwrap(),
            -3
        );
        assert_eq!(
            back.view
                .pointer("/inventory/itemBags/mainBag/2/content/count")
                .unwrap(),
            40
        );
        assert!(parse(&raw[..raw.len() - 1]).is_err());
        assert!(parse(b"SBVJ01\x01x\x00\x09").is_err());
    }

    #[test]
    #[ignore]
    fn real_starbound_files() {
        let list = std::env::var("STARBOUND_SAVES").expect("set STARBOUND_SAVES");
        for path in list.split('|').filter(|s| !s.is_empty()) {
            let raw = std::fs::read(path).unwrap();
            let dec = crate::formats::decode(&raw);
            let crate::formats::Content::Tree(t) = &dec.content else {
                panic!("{path}: not a tree")
            };
            assert!(
                t.kind_name.starts_with("Starbound"),
                "{path}: {}",
                t.kind_name
            );
            assert_eq!(
                crate::formats::encode(&dec.layers, &dec.content).unwrap(),
                raw,
                "{path}: unchanged differs"
            );
            if t.kind_name != KIND_PLAYER {
                continue;
            }
            let mut doc = (**t).clone();
            let money = ["/inventory/currencies/money", "/inventory/money"]
                .into_iter()
                .find(|p| doc.view.pointer(p).is_some())
                .unwrap();
            let n = doc.view.pointer(money).and_then(Value::as_i64).unwrap();
            // Past 2^20 so the varint gets longer.
            doc.set(money, Value::from(n + 5_000_000)).unwrap();
            let health = "/statusController/resourceValues/health";
            let had_health = doc.view.pointer(health).is_some();
            if had_health {
                doc.set(health, Value::from(1234.5)).unwrap();
            }
            let out = doc.to_bytes().unwrap();
            let back = parse(&out).unwrap();
            assert_eq!(
                back.view.pointer(money).and_then(Value::as_i64),
                Some(n + 5_000_000),
                "{path}"
            );
            if had_health {
                assert_eq!(
                    back.view.pointer(health).and_then(Value::as_f64),
                    Some(1234.5),
                    "{path}"
                );
            }
            let mut a = t.view.clone();
            let mut b = back.view.clone();
            *a.pointer_mut(money).unwrap() = Value::Null;
            *b.pointer_mut(money).unwrap() = Value::Null;
            if had_health {
                *a.pointer_mut(health).unwrap() = Value::Null;
                *b.pointer_mut(health).unwrap() = Value::Null;
            }
            assert!(a == b, "{path}: other values changed");
            println!(
                "{path}: ok ({} values, {} bytes -> {})",
                t.editable.len(),
                raw.len(),
                out.len()
            );
        }
    }
}
