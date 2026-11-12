//! Rune Factory 4 Special (PC and extracted Switch) slot saves, `rf4_sNN.sav`.
//!
//! A CRC-32 at 0 covers bytes 4 up to the last 224 (a trailer kept as it is). Field positions
//! from rf4s_pc_saveedit (rikimtasu), GPL-3.0: money, lumber, material stone, fertilizer,
//! health, rune points, the three base attributes (low 24 bits of their words), order points
//! and tourists. Ranges are the game's own clamps where that editor found them.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Rune Factory 4 Special save";
const TRAILER: usize = 224;
const PC_LEN: usize = 140_424;
const SWITCH_LEN: usize = PC_LEN - 8;
pub const MATERIAL_CAP: i64 = 50_000;

fn crc32(d: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in d {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

pub fn looks(d: &[u8]) -> bool {
    (d.len() == PC_LEN || d.len() == SWITCH_LEN) && crc32(&d[4..d.len() - TRAILER]).to_le_bytes() == d[..4]
}

fn fix(d: &mut Vec<u8>) {
    let n = d.len() - TRAILER;
    let c = crc32(&d[4..n]);
    d[..4].copy_from_slice(&c.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Rune Factory 4 Special save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Player", "Money"], 0x4C, Kind::I32, 0, 2_000_000_000);
    let max_hp = i32::from_le_bytes(d[0x54..0x58].try_into().unwrap()).max(1) as i64;
    b.ranged(&["Player", "Max HP"], 0x54, Kind::I32, 1, i32::MAX as i64);
    b.ranged(&["Player", "HP"], 0x58, Kind::I32, 0, max_hp);
    let max_rp = u16::from_le_bytes([d[0x5C], d[0x5D]]).max(1) as i64;
    b.ranged(&["Player", "Max RP"], 0x5C, Kind::U16, 1, 65_535);
    b.ranged(&["Player", "RP"], 0x5E, Kind::U16, 0, max_rp);
    for (k, at) in [("Strength", 0x60), ("Intelligence", 0x64), ("Vitality", 0x68)] {
        b.bits(&["Player", k], at, Kind::U32, 0, 24, 0, 1, 16_777_215);
    }
    b.ranged(&["Player", "Order points"], 0x20710, Kind::I32, 0, 99_999);
    b.ranged(&["Player", "Tourists"], 0x20714, Kind::I32, 0, 99_999_999);
    b.ranged(&["Materials", "Lumber"], 0x3C, Kind::I32, 0, MATERIAL_CAP);
    b.ranged(&["Materials", "Material stone"], 0x40, Kind::I32, 0, MATERIAL_CAP);
    b.ranged(&["Materials", "Fertilizer"], 0x44, Kind::I32, 0, MATERIAL_CAP);
    b.info(&["Save", "Platform"], if d.len() == PC_LEN { "PC" } else { "Switch" });
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(
        "Money, health, rune points, the base attributes, order points, tourists, lumber, material stone and fertilizer. The save's CRC is written again on save.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_the_crc() {
        let mut d = vec![0u8; PC_LEN];
        d[0x4C..0x50].copy_from_slice(&200i32.to_le_bytes());
        d[0x54..0x58].copy_from_slice(&25i32.to_le_bytes());
        d[0x58..0x5C].copy_from_slice(&25i32.to_le_bytes());
        d[0x60..0x64].copy_from_slice(&0xAB00_0005u32.to_le_bytes());
        let n = PC_LEN - TRAILER;
        d[n] = 0x77;
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Player/Strength"), Some(&serde_json::Value::from(5)));
        doc.set("/Player/Money", serde_json::Value::from(1_000_000)).unwrap();
        doc.set("/Player/Strength", serde_json::Value::from(999)).unwrap();
        doc.set("/Materials/Lumber", serde_json::Value::from(50_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(&out[0x60..0x64], &(0xAB00_0000u32 | 999).to_le_bytes());
        assert_eq!(out[n], 0x77);
        assert!(doc.set("/Player/Money", serde_json::Value::from(-1)).is_err());
    }
}
