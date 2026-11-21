//! Just Cause 2 (PC) saves (`justcause2saveNN.sav`, about 890 KB). The first words are 7, 0, then
//! the chaos points and the money (both also kept in the table below), and a timestamp at 0x20.
//! Further in, near 0x39a00, the game keeps its counters as records of a 16-bit key, a 32-bit size
//! (4) and the value; keys 0 to 4 follow each other: weapon parts, armor parts, vehicle parts,
//! money and chaos points.
//!
//! The record keys and the header copies follow the public-domain Horizon save editor
//! (unknownv2/save-editor-tools, Unlicense); checked on public PC saves where the table matches the
//! header. Values are written in place, both copies together.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Just Cause 2 save";
const TABLE: usize = 0x39a00;
const REGION: usize = 0x2000;

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Where the value of each of the five records sits, when keys 0 to 4 follow each other.
fn table(d: &[u8]) -> Option<[usize; 5]> {
    (TABLE..(TABLE + REGION).min(d.len().saturating_sub(50))).find_map(|p| {
        let rec = |n: usize| {
            let at = p + n * 10;
            (d[at] as usize == n && d[at + 1] == 0 && u32le(d, at + 2) == Some(4)).then_some(at + 6)
        };
        Some([rec(0)?, rec(1)?, rec(2)?, rec(3)?, rec(4)?])
    })
}

pub fn looks(d: &[u8]) -> bool {
    if d.len() < TABLE + REGION || u32le(d, 0) != Some(7) || u32le(d, 4) != Some(0) {
        return false;
    }
    table(d).is_some_and(|t| u32le(d, t[4]) == u32le(d, 8) && u32le(d, t[3]) == u32le(d, 12))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let t = match table(d).filter(|_| looks(d)) {
        Some(t) => t,
        None => return Err("not a Just Cause 2 save".into()),
    };
    let mut b = Builder::new(d);
    b.field(&["Money"], t[3], Kind::U32);
    b.mirror(&["Money"], 12);
    b.field(&["Chaos points"], t[4], Kind::U32);
    b.mirror(&["Chaos points"], 8);
    b.field(&["Parts", "Weapon parts"], t[0], Kind::U32);
    b.field(&["Parts", "Armor parts"], t[1], Kind::U32);
    b.field(&["Parts", "Vehicle parts"], t[2], Kind::U32);
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; TABLE + REGION];
        d[0] = 7;
        d[8..12].copy_from_slice(&500u32.to_le_bytes());
        d[12..16].copy_from_slice(&900u32.to_le_bytes());
        let vals = [11u32, 22, 33, 900, 500];
        for (n, v) in vals.iter().enumerate() {
            let at = TABLE + 0x30 + n * 10;
            d[at] = n as u8;
            d[at + 2] = 4;
            d[at + 6..at + 10].copy_from_slice(&v.to_le_bytes());
        }
        d
    }

    #[test]
    fn edits_both_copies_in_place() {
        let d = synthetic();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Parts/Vehicle parts"), Some(&serde_json::Value::from(33)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Money", serde_json::Value::from(777)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32le(&out, 12), Some(777));
        assert_eq!(u32le(&out, TABLE + 0x30 + 36), Some(777));
        assert_eq!(out.len(), d.len());
    }

    #[test]
    fn other_files_are_refused() {
        assert!(!looks(&vec![0u8; TABLE + REGION]));
        let mut d = synthetic();
        d[12] ^= 1; // header and table disagree
        assert!(!looks(&d));
    }
}
