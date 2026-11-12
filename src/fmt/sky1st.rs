//! Trails in the Sky 1st Chapter (2025; Steam saves are zstd-packed, which is its own layer).
//!
//! A CRC-32 (the IEEE table, started at 0x1F3FFC, the length it covers, and not inverted)
//! of 0x1F3FFC bytes from 0x0C is stored at 0x08. Field positions from turtle-insect's
//! TrailsintheSky1stChapter editor (GPL-3.0): mira, the eight sepith, the eight characters'
//! experience, HP and EP, and item counts.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Trails in the Sky 1st Chapter save";
const SPAN: usize = 0x1F3FFC;
const SEPITH: usize = 0x1F38D8;
const MIRA: usize = 0x1F38F8;
const CHARS: usize = 0x102588;
const CHAR_SIZE: usize = 664;
pub const SEPITH_NAMES: [&str; 8] = ["Earth", "Water", "Fire", "Wind", "Time", "Space", "Mirage", "Mass"];
pub const SEPITH_CAP: i64 = 9999;

fn crc(d: &[u8]) -> u32 {
    let mut c = SPAN as u32;
    for &b in &d[0x0C..0x0C + SPAN] {
        c ^= b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg());
        }
    }
    c
}

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 0x0C + SPAN && crc(d).to_le_bytes() == d[8..12]
}

fn fix(d: &mut Vec<u8>) {
    let c = crc(d);
    d[8..12].copy_from_slice(&c.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Trails in the Sky 1st Chapter save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Party", "Mira"], MIRA, Kind::U32, 0, 9_999_999);
    for (i, s) in SEPITH_NAMES.iter().enumerate() {
        b.ranged(&["Sepith", s], SEPITH + i * 4, Kind::U32, 0, SEPITH_CAP);
    }
    for i in 0..8 {
        let at = CHARS + i * CHAR_SIZE;
        let slot = format!("Character {}", i + 1);
        b.info(&["Characters", &slot, "Id"], d[at]);
        b.info(&["Characters", &slot, "Level"], d[at + 4]);
        b.field(&["Characters", &slot, "Experience"], at + 8, Kind::U32);
        b.field(&["Characters", &slot, "HP"], at + 12, Kind::U32);
        b.field(&["Characters", &slot, "Max HP"], at + 16, Kind::U32);
        b.field(&["Characters", &slot, "EP"], at + 20, Kind::U32);
        b.field(&["Characters", &slot, "Max EP"], at + 24, Kind::U32);
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(
        "Mira, the eight sepith and each character's experience, HP and EP. The save's checksum is written again on save.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_the_checksum() {
        let mut d = vec![0u8; 0x0C + SPAN + 16];
        d[MIRA..MIRA + 4].copy_from_slice(&1234u32.to_le_bytes());
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Party/Mira"), Some(&serde_json::Value::from(1234)));
        doc.set("/Sepith/Earth", serde_json::Value::from(9999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert!(doc.set("/Sepith/Fire", serde_json::Value::from(10_000)).is_err());
    }

    /// The editor's own loop (each byte XORed into the low byte, then shifted through the
    /// table) is the plain reflected CRC-32 update; check one against the other.
    #[test]
    fn checksum_matches_the_table_loop() {
        let mut d = vec![0u8; 0x0C + SPAN];
        for (i, b) in d.iter_mut().enumerate().skip(0x0C).step_by(997) {
            *b = (i % 251) as u8;
        }
        let mut table = [0u32; 256];
        for (i, t) in table.iter_mut().enumerate() {
            let mut x = i as u32;
            for _ in 0..8 {
                x = if x & 1 == 0 { x >> 1 } else { 0xEDB8_8320 ^ (x >> 1) };
            }
            *t = x;
        }
        let mut c = SPAN as u32;
        for &b in &d[0x0C..0x0C + SPAN] {
            c = (c >> 8) ^ table[((c ^ b as u32) & 0xFF) as usize];
        }
        assert_eq!(c, crc(&d));
    }
}
