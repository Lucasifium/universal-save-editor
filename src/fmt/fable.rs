//! Fable: The Lost Chapters (PC) saves: the `FableSave!` container.
//!
//! ```text
//! "FableSave!" 00 00, u32 data end, u32 chunk 0 size, u32 chunk 1 size, u32 chunk 0 packed size,
//! chunk 0 (zlib), u32 chunk 1 packed size, chunk 1 (zlib), u32 checksum, zero padding to 300 KB
//! ```
//!
//! The checksum is a CRC-32 table run from 0 with no final inversion over everything before it.
//! Chunk 1 holds `SAVED_ENTITIES`, a run of zlib-packed cells framed as
//! `u64 0, u64 29 + packed size, u64 cell id, u64 packed size, u32 size, data`, with zero bytes
//! between records.
//! The hero's `CTCHeroStats` component lives in one cell, as fields of `u32 CRC-32 of the name`
//! followed by the value. Layout as documented by FableDecomp (MIT).

use crate::tree::{f32_value, round_f32, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};

pub const KIND: &str = "Fable: The Lost Chapters save";
const MAGIC: &[u8] = b"FableSave!";
const SLAB: usize = 0x4B000;

const FIELDS: [(&str, &str, Ty); 4] = [
    ("Gold", "Money", Ty::I32),
    ("Morality", "Morality", Ty::I32),
    ("Renown level", "RenownLevel", Ty::I32),
    ("Age", "Age", Ty::F32),
];

#[derive(Clone, Copy, Debug)]
enum Ty {
    I32,
    F32,
}

fn crc(d: &[u8]) -> u32 {
    let mut c = 0u32;
    for &b in d {
        let mut x = (c ^ b as u32) & 0xFF;
        for _ in 0..8 {
            x = if x & 1 != 0 {
                (x >> 1) ^ 0xEDB8_8320
            } else {
                x >> 1
            };
        }
        c = (c >> 8) ^ x;
    }
    c
}

fn u32at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

fn inflate(z: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![];
    flate2::read::ZlibDecoder::new(z)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

fn deflate(u: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::best());
    z.write_all(u).expect("writing to memory");
    z.finish().expect("writing to memory")
}

#[derive(Clone, Debug)]
struct Save {
    chunk0_raw: Vec<u8>,
    chunk0_len: u32,
    chunk1: Vec<u8>,
    /// Where SAVED_ENTITIES' length is, and the hero cell's record and data.
    se_len_at: usize,
    cell_at: usize,
    cell: Vec<u8>,
    /// Where each field's value is in the inflated hero cell.
    fields: Vec<Option<usize>>,
}

fn open(d: &[u8]) -> Option<Save> {
    if !d.starts_with(MAGIC) {
        return None;
    }
    let end = u32at(d, 12)? as usize;
    let (u0, u1, c0) = (
        u32at(d, 16)?,
        u32at(d, 20)? as usize,
        u32at(d, 24)? as usize,
    );
    let c1 = u32at(d, 28 + c0)? as usize;
    if 32 + c0 + c1 != end || u32at(d, end)? != crc(&d[..end]) {
        return None;
    }
    let chunk1 = inflate(d.get(32 + c0..end)?)?;
    if chunk1.len() != u1 {
        return None;
    }
    let se = chunk1.windows(15).position(|w| w == b"SAVED_ENTITIES\0")? + 15;
    let se_end = se + 4 + u32at(&chunk1, se)? as usize;
    let mut q = se + 16;
    while q + 36 < se_end.min(chunk1.len()) {
        if chunk1[q..q + 8].iter().all(|b| *b == 0) {
            let (rec, cl) = (u64at(&chunk1, q + 8)?, u64at(&chunk1, q + 24)? as usize);
            let cell = (rec == 29 + cl as u64 && chunk1[q + 36] == 0x78)
                .then(|| chunk1.get(q + 36..q + 36 + cl))
                .flatten()
                .and_then(inflate);
            if let Some(cell) = cell {
                if let Some(k) = cell.windows(13).position(|w| w == b"CTCHeroStats\0") {
                    let window = k..(k + 0x400).min(cell.len());
                    let fields = FIELDS
                        .iter()
                        .map(|(_, name, _)| {
                            let tag = crc(name.as_bytes()).to_le_bytes();
                            cell[window.clone()]
                                .windows(4)
                                .position(|w| w == tag)
                                .map(|i| window.start + i + 4)
                        })
                        .collect::<Vec<_>>();
                    if fields[0].is_some() {
                        return Some(Save {
                            chunk0_raw: d[28..28 + c0].to_vec(),
                            chunk0_len: u0,
                            chunk1,
                            se_len_at: se,
                            cell_at: q,
                            cell,
                            fields,
                        });
                    }
                }
                q += 36 + cl;
                continue;
            }
        }
        q += 1;
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(MAGIC)
}

impl Save {
    fn value(&self, i: usize) -> Option<Value> {
        let at = self.fields[i]?;
        let b: [u8; 4] = self.cell.get(at..at + 4)?.try_into().ok()?;
        Some(match FIELDS[i].2 {
            Ty::I32 => Value::from(i32::from_le_bytes(b)),
            Ty::F32 => f32_value(f32::from_le_bytes(b)),
        })
    }

    fn build(&self, cell: &[u8]) -> Vec<u8> {
        let q = self.cell_at;
        let old_cl = u64at(&self.chunk1, q + 24).unwrap_or(0) as usize;
        let z = deflate(cell);
        let mut c1 = self.chunk1[..q + 8].to_vec();
        c1.extend((29 + z.len() as u64).to_le_bytes());
        c1.extend_from_slice(&self.chunk1[q + 16..q + 24]);
        c1.extend((z.len() as u64).to_le_bytes());
        c1.extend_from_slice(&self.chunk1[q + 32..q + 36]);
        c1.extend(&z);
        c1.extend_from_slice(&self.chunk1[q + 36 + old_cl..]);
        let se = u32at(&c1, self.se_len_at).unwrap_or(0) as i64 + z.len() as i64 - old_cl as i64;
        c1[self.se_len_at..self.se_len_at + 4].copy_from_slice(&(se as u32).to_le_bytes());
        let packed1 = deflate(&c1);
        let end = 32 + self.chunk0_raw.len() + packed1.len();
        let mut out = MAGIC.to_vec();
        out.extend([0, 0]);
        for n in [
            end,
            self.chunk0_len as usize,
            c1.len(),
            self.chunk0_raw.len(),
        ] {
            out.extend((n as u32).to_le_bytes());
        }
        out.extend(&self.chunk0_raw);
        out.extend((packed1.len() as u32).to_le_bytes());
        out.extend(packed1);
        let sig = crc(&out);
        out.extend(sig.to_le_bytes());
        if out.len() < SLAB {
            out.resize(SLAB, 0);
        }
        out
    }
}

#[derive(Clone, Debug)]
struct B(Save);

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut cell = self.0.cell.clone();
        for (id, v) in changes {
            let at = self
                .0
                .fields
                .get(*id)
                .copied()
                .flatten()
                .ok_or("This value isn't in the save")?;
            let bytes = match FIELDS[*id].2 {
                Ty::I32 => (v.as_i64().ok_or("Must be a whole number")? as i32).to_le_bytes(),
                Ty::F32 => (v.as_f64().ok_or("Must be a number")? as f32).to_le_bytes(),
            };
            cell[at..at + 4].copy_from_slice(&bytes);
        }
        Ok(self.0.build(&cell))
    }
    fn normalize(&self, leaf: usize, v: Value) -> Value {
        match FIELDS.get(leaf).map(|f| f.2) {
            Some(Ty::F32) => round_f32(v),
            _ => v,
        }
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match FIELDS.get(leaf).map(|f| f.2) {
            Some(Ty::F32) if v.as_f64().is_some_and(|n| (0.0..=1000.0).contains(&n)) => Ok(()),
            Some(Ty::F32) => Err("Must be a number from 0 to 1,000".into()),
            _ if v.as_i64().is_some_and(|n| i32::try_from(n).is_ok()) => Ok(()),
            _ => Err("Must be a whole number from -2,147,483,648 to 2,147,483,647".into()),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let s = open(d)?;
    let mut hero = Map::new();
    let mut edit = HashMap::new();
    for (i, (label, _, _)) in FIELDS.iter().enumerate() {
        if let Some(v) = s.value(i) {
            edit.insert(format!("/Hero/{label}"), i);
            hero.insert(label.to_string(), v);
        }
    }
    let mut root = Map::new();
    root.insert("Hero".into(), Value::Object(hero));
    let mut doc = TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B(s)));
    doc.note =
        Some("Fable save: unpacked for you, with its checksum made again when you save.".into());
    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut cell = b"PlayerCreature\0junkCTCHeroStats\0\0\0\0\0\x40\0\0\0".to_vec();
        for (name, v) in [
            ("Morality", 300i32.to_le_bytes()),
            ("Age", 18.5f32.to_le_bytes()),
            ("Money", 1625i32.to_le_bytes()),
            ("RenownLevel", 2i32.to_le_bytes()),
        ] {
            cell.extend(crc(name.as_bytes()).to_le_bytes());
            cell.extend(v);
        }
        let z = deflate(&cell);
        let mut c1 = b"ENTITIES\0".to_vec();
        c1.extend(b"SAVED_ENTITIES\0");
        let se_len = 12 + 36 + z.len() + 9 + 20;
        c1.extend((se_len as u32).to_le_bytes());
        c1.extend([0u8; 12]);
        c1.extend([0u8; 20]);
        c1.extend([0u8; 8]);
        c1.extend((29 + z.len() as u64).to_le_bytes());
        c1.extend(0x3020dfu64.to_le_bytes());
        c1.extend((z.len() as u64).to_le_bytes());
        c1.extend((cell.len() as u32).to_le_bytes());
        c1.extend(&z);
        c1.extend([0u8; 9]);
        c1.extend(b"PLAYER\0rest");
        let c0 = b"HEADER\0\x04\0\0\0abcd".to_vec();
        let s = Save {
            chunk0_raw: deflate(&c0),
            chunk0_len: c0.len() as u32,
            chunk1: c1,
            se_len_at: 0,
            cell_at: 0,
            cell: vec![],
            fields: vec![],
        };
        let mut out = MAGIC.to_vec();
        out.extend([0, 0]);
        let p1 = deflate(&s.chunk1);
        for n in [
            32 + s.chunk0_raw.len() + p1.len(),
            c0.len(),
            s.chunk1.len(),
            s.chunk0_raw.len(),
        ] {
            out.extend((n as u32).to_le_bytes());
        }
        out.extend(&s.chunk0_raw);
        out.extend((p1.len() as u32).to_le_bytes());
        out.extend(p1);
        let sig = crc(&out);
        out.extend(sig.to_le_bytes());
        out.resize(SLAB, 0);
        out
    }

    #[test]
    fn reads_and_edits_hero() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Hero/Gold").unwrap(), 1625);
        assert_eq!(doc.view.pointer("/Hero/Morality").unwrap(), 300);
        doc.set("/Hero/Gold", Value::from(999_999)).unwrap();
        doc.set("/Hero/Morality", Value::from(-1000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), SLAB);
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Hero/Gold").unwrap(), 999_999);
        assert_eq!(back.view.pointer("/Hero/Morality").unwrap(), -1000);
        assert_eq!(back.view.pointer("/Hero/Renown level").unwrap(), 2);
        assert!(!looks(b"FableSav"));
    }
}
