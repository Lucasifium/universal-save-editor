//! Resident Evil 5 (Steam) `savedata.bin`: XORed in 8-byte blocks, each block with the encrypted
//! block before it (the first with a fixed key), and a 32-bit sum of the data at 0x08.
//!
//! The key, the sum's two ranges and where the money and exchange points are kept are as
//! published by RE5-Save-Editor (shinneider, MIT). The old Games for Windows Live save is a
//! different, signed file and isn't read here.

use crate::tree::{Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Resident Evil 5 save";
const KEY: [u8; 8] = [0x00, 0x21, 0x11, 0x08, 0xC0, 0x4B, 0x00, 0x00];
const SUM_AT: usize = 0x08;
const MONEY: usize = 0x194;
const POINTS: usize = 0x198;
const MIN_LEN: usize = 0x3BB0 + 0x5C0 * 4;

fn decode(d: &[u8]) -> Vec<u8> {
    let mut key = KEY;
    let mut out = Vec::with_capacity(d.len());
    for blk in d.chunks(8) {
        out.extend(blk.iter().zip(key).map(|(a, b)| a ^ b));
        if blk.len() == 8 {
            key.copy_from_slice(blk);
        }
    }
    out
}

fn encode(p: &[u8]) -> Vec<u8> {
    let mut key = KEY;
    let mut out = Vec::with_capacity(p.len());
    for blk in p.chunks(8) {
        let enc: Vec<u8> = blk.iter().zip(key).map(|(a, b)| a ^ b).collect();
        if enc.len() == 8 {
            key.copy_from_slice(&enc);
        }
        out.extend(enc);
    }
    out
}

fn u32at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

fn sum(p: &[u8]) -> u32 {
    let add = |r: u32, from: usize, n: usize| (0..n).fold(r, |r, i| r.wrapping_add(u32at(p, from + i * 4)));
    add(add(0, 0x10, 0xEE7), 0x3BB0, 0x5C0)
}

/// The decrypted save, if this is one.
fn open(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < MIN_LEN || d.len() % 8 != 0 {
        return None;
    }
    let p = decode(d);
    (u32at(&p, SUM_AT) == sum(&p) && (2009..2100).contains(&u32at(&p, 0x10))).then_some(p)
}

pub fn looks(d: &[u8]) -> bool {
    open(d).is_some()
}

#[derive(Clone, Debug)]
struct B(Vec<u8>);

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let n = v.as_u64().filter(|n| *n <= u32::MAX as u64).ok_or("Must be a whole number")?;
            let at = if *id == 0 { MONEY } else { POINTS };
            p.add(at, at + 4, (n as u32).to_le_bytes().to_vec());
        }
        let mut plain = p.apply(&self.0)?;
        let s = sum(&plain);
        plain[SUM_AT..SUM_AT + 4].copy_from_slice(&s.to_le_bytes());
        Ok(encode(&plain))
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        match v.as_u64() {
            Some(n) if n <= 99_999_999 => Ok(()),
            _ => Err("Must be a whole number from 0 to 99,999,999".into()),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let p = open(d)?;
    let t: Vec<u32> = (0..6).map(|i| u32at(&p, 0x10 + i * 4)).collect();
    let mut m = Map::new();
    m.insert("Money".into(), Value::from(u32at(&p, MONEY)));
    m.insert("Exchange points".into(), Value::from(u32at(&p, POINTS)));
    m.insert("Saved".into(), Value::String(format!("{}-{:02}-{:02} {:02}:{:02}:{:02}", t[0], t[1], t[2], t[3], t[4], t[5])));
    let mut root = Map::new();
    root.insert("Save".into(), Value::Object(m));
    let edit = HashMap::from([("/Save/Money".to_string(), 0usize), ("/Save/Exchange points".to_string(), 1)]);
    let mut doc = TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B(p)));
    doc.note = Some("Resident Evil 5 save: decrypted for you, with its checksum made again when you save.".into());
    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut p = vec![0u8; MIN_LEN + 8];
        p[0x10..0x14].copy_from_slice(&2026u32.to_le_bytes());
        p[MONEY..MONEY + 4].copy_from_slice(&5000u32.to_le_bytes());
        p[0x3BB0 + 40] = 9;
        let s = sum(&p);
        p[SUM_AT..SUM_AT + 4].copy_from_slice(&s.to_le_bytes());
        encode(&p)
    }

    #[test]
    fn round_trip_and_edit() {
        let d = sample();
        assert_eq!(decode(&encode(&decode(&d))), decode(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Save/Money").unwrap(), 5000);
        doc.set("/Save/Money", Value::from(999_999)).unwrap();
        doc.set("/Save/Exchange points", Value::from(123_456)).unwrap();
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Save/Money").unwrap(), 999_999);
        assert_eq!(back.view.pointer("/Save/Exchange points").unwrap(), 123_456);
        assert!(!looks(&[0u8; 64]));
    }
}
