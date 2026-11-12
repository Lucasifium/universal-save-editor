//! Easy Save 2 files (Unity games such as My Summer Car: `defaultES2File.txt`, `items.txt`).
//!
//! ```text
//! entry = '~' u8 tag length, tag, u32 body length, body
//! body  = u8 0xFF, u32 type hash, value, '{'      (a single value)
//!       | anything else                           (lists and dictionaries: kept as they are)
//! ```
//!
//! Single floats, whole numbers, true/false and texts are shown by tag and can be changed;
//! vectors, colours, transforms and collections are left exactly as they were. Worked out from
//! public My Summer Car saves.

use crate::tree::{f32_value, round_f32, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Easy Save 2 file";

const FLOAT: u32 = 0x6E3E_D76B;
const INT: u32 = 0xE2A8_0856;
const BOOL: u32 = 0xAD4D_7C9C;
const STRING: u32 = 0xFDE9_F1EE;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ty {
    F32,
    I32,
    Bool,
    Str,
}

#[derive(Clone, Debug)]
struct Entry {
    tag: String,
    /// Where the entry starts and its body starts, and the end of the entry.
    start: usize,
    body: usize,
    end: usize,
    ty: Option<Ty>,
}

/// A 7-bit encoded length (as .NET's BinaryWriter writes it).
fn read_len(d: &[u8], at: &mut usize) -> Option<usize> {
    let mut n = 0usize;
    for shift in (0..35).step_by(7) {
        let b = *d.get(*at)?;
        *at += 1;
        n |= ((b & 0x7F) as usize) << shift;
        if b & 0x80 == 0 {
            return Some(n);
        }
    }
    None
}

fn write_len(mut n: usize, out: &mut Vec<u8>) {
    loop {
        let b = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn entries(d: &[u8]) -> Option<Vec<Entry>> {
    let mut out = vec![];
    let mut p = 0;
    while p < d.len() {
        if d[p] != b'~' {
            return None;
        }
        let start = p;
        p += 1;
        let n = read_len(d, &mut p)?;
        let tag = std::str::from_utf8(d.get(p..p + n)?).ok()?.to_string();
        p += n;
        let len = u32::from_le_bytes(d.get(p..p + 4)?.try_into().ok()?) as usize;
        p += 4;
        let body = p;
        let end = body.checked_add(len)?;
        if end > d.len() || len == 0 {
            return None;
        }
        let b = &d[body..end];
        let ty = if b.len() >= 6 && b[0] == 0xFF && b[b.len() - 1] == b'{' {
            match (u32::from_le_bytes(b[1..5].try_into().unwrap()), b.len()) {
                (FLOAT, 10) => Some(Ty::F32),
                (INT, 10) => Some(Ty::I32),
                (BOOL, 7) => Some(Ty::Bool),
                (STRING, _) => {
                    let mut q = 5;
                    let n = read_len(b, &mut q)?;
                    (q + n + 1 == b.len() && std::str::from_utf8(&b[q..q + n]).is_ok()).then_some(Ty::Str)
                }
                _ => None,
            }
        } else {
            None
        };
        out.push(Entry { tag, start, body, end, ty });
        p = end;
    }
    (!out.is_empty()).then_some(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.first() == Some(&b'~') && entries(d).is_some_and(|e| e.iter().any(|e| e.ty.is_some()))
}

fn value(d: &[u8], e: &Entry) -> Option<Value> {
    let v = &d[e.body + 5..e.end - 1];
    Some(match e.ty? {
        Ty::F32 => f32_value(f32::from_le_bytes(v.try_into().ok()?)),
        Ty::I32 => Value::from(i32::from_le_bytes(v.try_into().ok()?)),
        Ty::Bool => Value::Bool(v[0] != 0),
        Ty::Str => {
            let mut q = 0;
            let n = read_len(v, &mut q)?;
            Value::String(String::from_utf8_lossy(&v[q..q + n]).into_owned())
        }
    })
}

#[derive(Clone, Debug)]
struct B {
    raw: Vec<u8>,
    entries: Vec<Entry>,
}

impl B {
    fn encode(&self, e: &Entry, v: &Value) -> Result<Vec<u8>, String> {
        let mut body = self.raw[e.body..e.body + 5].to_vec();
        match e.ty.ok_or("This value can't be changed")? {
            Ty::F32 => body.extend((v.as_f64().ok_or("Must be a number")? as f32).to_le_bytes()),
            Ty::I32 => body.extend((v.as_i64().ok_or("Must be a whole number")? as i32).to_le_bytes()),
            Ty::Bool => body.push(v.as_bool().ok_or("Must be true or false")? as u8),
            Ty::Str => {
                let s = v.as_str().ok_or("Must be text")?;
                write_len(s.len(), &mut body);
                body.extend(s.as_bytes());
            }
        }
        body.push(b'{');
        let mut out = self.raw[e.start..e.body - 4].to_vec();
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        Ok(out)
    }
}

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut by: HashMap<usize, &Value> = HashMap::new();
        for (id, v) in changes {
            by.insert(*id, v);
        }
        let mut out = Vec::with_capacity(self.raw.len());
        for (i, e) in self.entries.iter().enumerate() {
            match by.get(&i) {
                Some(v) => out.extend(self.encode(e, v)?),
                None => out.extend_from_slice(&self.raw[e.start..e.end]),
            }
        }
        Ok(out)
    }
    fn normalize(&self, leaf: usize, v: Value) -> Value {
        match self.entries.get(leaf).and_then(|e| e.ty) {
            Some(Ty::F32) => round_f32(v),
            _ => v,
        }
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match self.entries.get(leaf).and_then(|e| e.ty) {
            Some(Ty::F32) if v.as_f64().is_some_and(f64::is_finite) => Ok(()),
            Some(Ty::I32) if v.as_i64().is_some_and(|n| i32::try_from(n).is_ok()) => Ok(()),
            Some(Ty::Bool) if v.is_boolean() => Ok(()),
            Some(Ty::Str) if v.is_string() => Ok(()),
            Some(Ty::F32) => Err("Must be a number".into()),
            Some(Ty::I32) => Err("Must be a whole number from -2,147,483,648 to 2,147,483,647".into()),
            Some(Ty::Bool) => Err("Must be true or false".into()),
            _ => Err("Must be text".into()),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let entries = entries(d)?;
    entries.iter().any(|e| e.ty.is_some()).then_some(())?;
    let mut root = Map::new();
    let mut edit = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        if root.contains_key(&e.tag) {
            continue;
        }
        match value(d, e) {
            Some(v) => {
                edit.insert(format!("/{}", crate::json_view::pointer_escape(&e.tag)), i);
                root.insert(e.tag.clone(), v);
            }
            None => {
                root.insert(e.tag.clone(), Value::String(format!("({} bytes, kept as it is)", e.end - e.body)));
            }
        }
    }
    Some(TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B { raw: d.to_vec(), entries })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(tag: &str, body: &[u8]) -> Vec<u8> {
        let mut v = vec![b'~', tag.len() as u8];
        v.extend(tag.as_bytes());
        v.extend((body.len() as u32).to_le_bytes());
        v.extend(body);
        v
    }

    fn single(hash: u32, val: &[u8]) -> Vec<u8> {
        let mut b = vec![0xFF];
        b.extend(hash.to_le_bytes());
        b.extend(val);
        b.push(b'{');
        b
    }

    #[test]
    fn reads_and_writes_values() {
        let mut d = entry("PlayerMoney", &single(FLOAT, &3000f32.to_le_bytes()));
        d.extend(entry("Slot1Cash", &single(INT, &5310i32.to_le_bytes())));
        d.extend(entry("floppy1in", &single(BOOL, &[0])));
        d.extend(entry("ShitCrimeTime", &single(STRING, b"\x0512:00")));
        d.extend(entry("Wheel", &[0xFF, 1, 2, 3, 4, 9, 9, 9, b'{']));
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view["PlayerMoney"].as_f64(), Some(3000.0));
        assert_eq!(doc.view["ShitCrimeTime"], "12:00");
        doc.set("/PlayerMoney", Value::from(99999)).unwrap();
        doc.set("/ShitCrimeTime", Value::from("08:30:00")).unwrap();
        doc.set("/floppy1in", Value::Bool(true)).unwrap();
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view["PlayerMoney"].as_f64(), Some(99999.0));
        assert_eq!(back.view["ShitCrimeTime"], "08:30:00");
        assert_eq!(back.view["floppy1in"], true);
        assert_eq!(back.view["Slot1Cash"], 5310);
        assert!(out.ends_with(&entry("Wheel", &[0xFF, 1, 2, 3, 4, 9, 9, 9, b'{'])));
    }
}
