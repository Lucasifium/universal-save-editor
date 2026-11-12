//! The Forest saves (`__RESUME__`): Base64 text holding a UnitySerializer stream whose data
//! string is `NOCOMPRESSION` plus a second Base64 layer. Inside, the player's inventory is a run
//! of records `01 id, 02 amount, 03 max amount, 04 bonus` (each a tag, `ff ff` and a 32-bit
//! number). Amounts are edited in place, so nothing changes size and the stream stays valid.
//! Item ids and names come from the community wiki's console-command table.

use super::forest_items::NAMES;
use crate::tree::{child, Backend, TreeDoc};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "The Forest save";
const NOCOMP: &[u8] = b"NOCOMPRESSION";
const HEAD: &[u8] = &[0xFF, 0xFF, 0x4F, 0x04, 0x01, 0x00, 0xFF, 0xFF];

#[derive(Clone, Debug)]
struct Layout {
    /// Outer stream (decoded) before the inner Base64 string, and after it.
    pre: Vec<u8>,
    post: Vec<u8>,
    inner: Vec<u8>,
    /// Whitespace after the outer Base64 text.
    tail: Vec<u8>,
}

fn varint_len(n: usize) -> Vec<u8> {
    let mut v = vec![];
    let mut n = n;
    loop {
        let b = (n & 0x7F) as u8;
        n >>= 7;
        if n == 0 {
            v.push(b);
            return v;
        }
        v.push(b | 0x80);
    }
}

/// Splits the outer stream: the data string starts with `NOCOMPRESSION` and its length is a
/// 7-bit varint just before it.
fn split(file: &[u8]) -> Option<Layout> {
    let text_end = file.iter().rposition(|b| !b.is_ascii_whitespace())? + 1;
    let outer = STANDARD.decode(&file[..text_end]).ok()?;
    if !outer.starts_with(b"\x06SerV10") {
        return None;
    }
    let at = outer.windows(NOCOMP.len()).position(|w| w == NOCOMP)?;
    // The varint (1–5 bytes) ends just before `at`; the right one gives a Base64 string that
    // decodes to another SerV10 stream.
    for n in 1..=5usize {
        if at < n {
            break;
        }
        let bytes = &outer[at - n..at];
        if !(bytes.iter().take(n - 1).all(|b| b & 0x80 != 0) && bytes[n - 1] & 0x80 == 0) {
            continue;
        }
        let v = bytes.iter().enumerate().fold(0usize, |a, (k, b)| a | (((b & 0x7F) as usize) << (7 * k)));
        if v < NOCOMP.len() || at + v > outer.len() {
            continue;
        }
        let Ok(inner) = STANDARD.decode(&outer[at + NOCOMP.len()..at + v]) else { continue };
        if inner.starts_with(b"\x06SerV10") {
            return Some(Layout { pre: outer[..at + NOCOMP.len()].to_vec(), post: outer[at + v..].to_vec(), inner, tail: file[text_end..].to_vec() });
        }
    }
    None
}

fn join(l: &Layout) -> Vec<u8> {
    let b64 = STANDARD.encode(&l.inner);
    // The varint length before NOCOMPRESSION is unchanged (the Base64 is the same length).
    let mut outer = l.pre.clone();
    outer.extend_from_slice(b64.as_bytes());
    outer.extend_from_slice(&l.post);
    let mut out = STANDARD.encode(&outer).into_bytes();
    out.extend_from_slice(&l.tail);
    out
}

fn i32at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// (id, amount offset, amount, max) of each inventory record.
fn records(inner: &[u8]) -> Vec<(i32, usize, i32, i32)> {
    let key = b"_possessedItems";
    let Some(k) = inner.windows(key.len()).position(|w| w == key) else { return vec![] };
    let Some(first) = inner[k..].windows(HEAD.len()).position(|w| w == HEAD).map(|p| p + k) else { return vec![] };
    let mut out = vec![];
    let mut p = first;
    // Each record: HEAD id | 02 00 ff ff amount | 03 00 ff ff max | 04 00 ff ff bonus | one byte
    while inner.get(p..p + HEAD.len()) == Some(HEAD) {
        let id = i32at(inner, p + 8);
        let tag2 = inner.get(p + 12..p + 16);
        let tag3 = inner.get(p + 20..p + 24);
        let tag4 = inner.get(p + 28..p + 32);
        let (Some(id), Some(t2), Some(t3), Some(t4)) = (id, tag2, tag3, tag4) else { break };
        if t2 != [2, 0, 0xFF, 0xFF] || t3 != [3, 0, 0xFF, 0xFF] || t4 != [4, 0, 0xFF, 0xFF] {
            break;
        }
        let (Some(amount), Some(max)) = (i32at(inner, p + 16), i32at(inner, p + 24)) else { break };
        out.push((id, p + 16, amount, max));
        p += 8 + 4 + 4 + 4 + 4 + 4 + 4 + 4 + 1;
    }
    out
}

#[derive(Clone, Debug)]
struct ForestBackend {
    file: Vec<u8>,
    at: Vec<usize>,
}

impl Backend for ForestBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut l = split(&self.file).ok_or("not a The Forest save")?;
        for (id, v) in changes {
            let n = v.as_i64().ok_or("Amounts are whole numbers")?;
            let n = i32::try_from(n).map_err(|_| "Too large".to_string())?;
            l.inner[self.at[*id]..self.at[*id] + 4].copy_from_slice(&n.to_le_bytes());
        }
        Ok(join(&l))
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        match v.as_i64() {
            Some(n) if n >= 0 => Ok(()),
            _ => Err("An amount is a whole number from 0".into()),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"BlNlclYx") && split(d).is_some_and(|l| !records(&l.inner).is_empty())
}

pub fn name(id: i32) -> String {
    NAMES.iter().find(|(i, _)| *i == id).map_or_else(|| format!("Item {id}"), |(_, n)| n.to_string())
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let l = split(d).ok_or("not a The Forest save")?;
    let recs = records(&l.inner);
    if recs.is_empty() {
        return Err("no inventory found".into());
    }
    let mut inv = Map::new();
    let mut editable: HashMap<String, usize> = HashMap::new();
    let mut at = vec![];
    for (id, off, amount, max) in recs {
        let mut key = name(id);
        let mut k = 2;
        while inv.contains_key(&key) {
            key = format!("{} ({k})", name(id));
            k += 1;
        }
        let mut o = Map::new();
        o.insert("Amount".into(), Value::from(amount));
        o.insert("Max amount".into(), Value::from(if max == i32::MAX { -1 } else { max as i64 }));
        o.insert("Item id".into(), Value::from(id));
        editable.insert(child(&child("/Inventory", &key), "Amount"), at.len());
        at.push(off);
        inv.insert(key, Value::Object(o));
    }
    let mut root = Map::new();
    root.insert("Inventory".into(), Value::Object(inv));
    let mut doc = TreeDoc::new(Value::Object(root), editable, KIND, Box::new(ForestBackend { file: d.to_vec(), at }));
    doc.note = Some("Only the inventory amounts are shown; the rest of the save is kept exactly as it was. Max amount -1 means the game sets no limit.".into());
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut inner = b"\x06SerV10 stuff _possessedItems ".to_vec();
        for (id, amt, max) in [(53i32, 5i32, 5i32), (57, 4, 10), (33, 20, i32::MAX)] {
            inner.extend_from_slice(HEAD);
            inner.extend_from_slice(&id.to_le_bytes());
            inner.extend_from_slice(&[2, 0, 0xFF, 0xFF]);
            inner.extend_from_slice(&amt.to_le_bytes());
            inner.extend_from_slice(&[3, 0, 0xFF, 0xFF]);
            inner.extend_from_slice(&max.to_le_bytes());
            inner.extend_from_slice(&[4, 0, 0xFF, 0xFF, 0, 0, 0, 0, 0]);
        }
        inner.extend_from_slice(b"end");
        let b64 = STANDARD.encode(&inner);
        let mut outer = b"\x06SerV10\x00\x01".to_vec();
        outer.extend(varint_len(NOCOMP.len() + b64.len()));
        outer.extend_from_slice(NOCOMP);
        outer.extend_from_slice(b64.as_bytes());
        outer.extend_from_slice(b"\xFF\xFFtail");
        STANDARD.encode(&outer).into_bytes()
    }

    #[test]
    fn amounts_edit_in_place() {
        let f = sample();
        assert!(!looks(&f) || f.starts_with(b"BlNlclYx"));
        let mut doc = parse(&f).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), f);
        assert_eq!(doc.view.pointer("/Inventory/Rock/Amount"), Some(&Value::from(5)));
        assert_eq!(doc.view.pointer("/Inventory/Cloth/Max amount"), Some(&Value::from(-1)));
        doc.set("/Inventory/Stick/Amount", Value::from(99)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), f.len());
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Inventory/Stick/Amount"), Some(&Value::from(99)));
        assert_eq!(again.view.pointer("/Inventory/Rock/Amount"), Some(&Value::from(5)));
    }
}
