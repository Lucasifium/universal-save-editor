//! Sleeping Dogs: Definitive Edition saves (`HK GameSlot n`, `HK Autosave Slot`).
//!
//! ```text
//! 0x00  "TC01", u32 version 0x60001, u64 FILETIME, 32-byte title, u32 file size
//! 0x34  scrambled body
//! end-4 u32 CRC of the plain body (MSB-first CRC-32, polynomial 0x04C11DB7, start 0xFFFFFFFF,
//!       no final xor), stored as it is
//! ```
//!
//! The body is scrambled byte by byte: with `n` the number of set bits in the FILETIME and
//! `s` the running byte sum of the plain bytes so far, byte `i` is xored with
//! `K[(n + i) & 31] + K[s & 31] + i`. The plain body starts with 16 (count, end offset) pairs,
//! then one 0x28-byte record per value (its name's CRC at +0x18, and at +0x20 a 64-bit offset
//! to the value, relative to that field), grouped by type; group 3 holds the 32-bit integers.
//! Worked out from the game's own executable.

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Sleeping Dogs save";
const HEAD: usize = 0x34;
const KEY: [u8; 32] = [
    0x37, 0x0c, 0x43, 0xb9, 0x91, 0xc9, 0x8c, 0x6c, 0xe5, 0x6f, 0x9d, 0x69, 0xb4, 0xc7, 0x20, 0xd0,
    0x81, 0xae, 0x0d, 0x93, 0xe9, 0xb5, 0x65, 0x6e, 0xb2, 0xf2, 0x11, 0x7c, 0x74, 0xe8, 0x36, 0xd8,
];
const INTS: usize = 3;

/// (label, value name, largest value)
const FIELDS: [(&str, &str, i64); 4] = [
    ("Money", "Money", 99_999_999),
    ("Face XP", "Standing", 50_000),
    ("Cop XP", "CopXP", 50_000),
    ("Triad XP", "TriadXP", 50_000),
];

fn crc(d: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in d {
        c ^= (b as u32) << 24;
        for _ in 0..8 {
            c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04c1_1db7 } else { c << 1 };
        }
    }
    c
}

fn bits(d: &[u8]) -> usize {
    u64::from_le_bytes(d[8..16].try_into().unwrap()).count_ones() as usize
}

fn key(n: usize, i: usize, s: u8) -> u8 {
    KEY[(n + i) & 31].wrapping_add(KEY[s as usize & 31]).wrapping_add(i as u8)
}

fn unscramble(d: &[u8]) -> Vec<u8> {
    let n = bits(d);
    let mut out = d.to_vec();
    let mut s = 0u8;
    for i in 0..d.len() - HEAD - 4 {
        let p = d[HEAD + i] ^ key(n, i, s);
        out[HEAD + i] = p;
        s = s.wrapping_add(p);
    }
    out
}

fn scramble(plain: &[u8]) -> Vec<u8> {
    let n = bits(plain);
    let mut out = plain.to_vec();
    let mut s = 0u8;
    for i in 0..plain.len() - HEAD - 4 {
        let p = plain[HEAD + i];
        out[HEAD + i] = p ^ key(n, i, s);
        s = s.wrapping_add(p);
    }
    out
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > HEAD + 0x84
        && d.starts_with(b"TC01\x01\x00\x06\x00")
        && u32::from_le_bytes(d[0x30..0x34].try_into().unwrap()) as usize == d.len()
}

/// Where each 32-bit integer value is in the whole (plain) file, by name CRC.
fn ints(p: &[u8]) -> Option<HashMap<u32, usize>> {
    let body = &p[HEAD..p.len() - 4];
    let mut rec = 0x80usize;
    let mut out = HashMap::new();
    for g in 0..16 {
        let count = u32::from_le_bytes(body.get(8 * g..8 * g + 4)?.try_into().ok()?) as usize;
        if count > body.len() / 0x28 {
            return None;
        }
        for j in 0..count {
            let r = rec + j * 0x28;
            let id = u32::from_le_bytes(body.get(r + 0x18..r + 0x1c)?.try_into().ok()?);
            let rel = i64::from_le_bytes(body.get(r + 0x20..r + 0x28)?.try_into().ok()?);
            let at = usize::try_from((r + 0x20) as i64 + rel).ok()?;
            if g == INTS && rel != 0 && at + 4 <= body.len() {
                out.insert(id, HEAD + at);
            }
        }
        rec += count * 0x28;
    }
    Some(out)
}

#[derive(Clone, Debug)]
struct Save {
    plain: Vec<u8>,
    at: Vec<usize>,
}

fn open(d: &[u8]) -> Option<Save> {
    if !looks(d) {
        return None;
    }
    let plain = unscramble(d);
    let n = plain.len();
    if crc(&plain[HEAD..n - 4]) != u32::from_le_bytes(plain[n - 4..].try_into().ok()?) {
        return None;
    }
    let map = ints(&plain)?;
    let at = FIELDS.iter().map(|f| map.get(&crc(f.1.as_bytes())).copied()).collect::<Option<Vec<_>>>()?;
    Some(Save { plain, at })
}

#[derive(Clone, Debug)]
struct B(Save);

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = self.0.plain.clone();
        for (id, v) in changes {
            let n = v.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or("Must be a whole number")?;
            let at = *self.0.at.get(*id).ok_or("Not editable")?;
            p[at..at + 4].copy_from_slice(&n.to_le_bytes());
        }
        let n = p.len();
        let c = crc(&p[HEAD..n - 4]);
        p[n - 4..].copy_from_slice(&c.to_le_bytes());
        Ok(scramble(&p))
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let max = FIELDS.get(leaf).map_or(0, |f| f.2);
        match v.as_i64() {
            Some(n) if (0..=max).contains(&n) => Ok(()),
            _ => Err(format!("Must be a whole number from 0 to {max}")),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let s = open(d)?;
    let mut wei = Map::new();
    let mut edit = HashMap::new();
    for (i, (label, ..)) in FIELDS.iter().enumerate() {
        let at = s.at[i];
        wei.insert(label.to_string(), Value::from(i32::from_le_bytes(s.plain[at..at + 4].try_into().ok()?)));
        edit.insert(format!("/Wei/{label}"), i);
    }
    let mut root = Map::new();
    root.insert("Wei".into(), Value::Object(wei));
    Some(TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B(s))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain save with one group-3 record per field, scrambled.
    fn sample() -> Vec<u8> {
        let mut p = b"TC01\x01\x00\x06\x00".to_vec();
        p.extend(0x01dd_04e7_7c0e_e3d0u64.to_le_bytes());
        p.extend([0u8; 32]);
        p.extend([0u8; 4]);
        let n = FIELDS.len();
        let mut body = vec![0u8; 0x80];
        body[24..28].copy_from_slice(&(n as u32).to_le_bytes());
        let data = 0x80 + n * 0x28;
        for (i, f) in FIELDS.iter().enumerate() {
            let r = 0x80 + i * 0x28;
            let mut rec = vec![0u8; 0x28];
            rec[0x18..0x1c].copy_from_slice(&crc(f.1.as_bytes()).to_le_bytes());
            rec[0x20..0x28].copy_from_slice(&((data + 4 * i) as i64 - (r + 0x20) as i64).to_le_bytes());
            body.extend(rec);
        }
        for v in [2_379_842i32, 50_000, 12_000, 300] {
            body.extend(v.to_le_bytes());
        }
        p.extend(&body);
        p.extend(crc(&body).to_le_bytes());
        let len = p.len() as u32;
        p[0x30..0x34].copy_from_slice(&len.to_le_bytes());
        scramble(&p)
    }

    #[test]
    fn scramble_round_trips() {
        let d = sample();
        assert_eq!(scramble(&unscramble(&d)), d);
    }

    #[test]
    fn reads_and_edits_money_and_xp() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Wei/Money").unwrap(), 2_379_842);
        assert_eq!(doc.view.pointer("/Wei/Cop XP").unwrap(), 12_000);
        doc.set("/Wei/Money", Value::from(50_000_000)).unwrap();
        doc.set("/Wei/Triad XP", Value::from(50_000)).unwrap();
        assert!(doc.set("/Wei/Cop XP", Value::from(60_000)).is_err());
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Wei/Money").unwrap(), 50_000_000);
        assert_eq!(back.view.pointer("/Wei/Triad XP").unwrap(), 50_000);
        assert_eq!(back.view.pointer("/Wei/Face XP").unwrap(), 50_000);
    }
}

