//! Persona 3 Portable (PC) `P3PSAVE*.BIN`. Field positions, item and Persona names from the
//! P3P Save Editor by Nightingale04 (GPL-3.0): the protagonist's level at 0x3E, the three
//! social stats from 0x74, experience at 0x7C, the item counts (one byte per item id) from
//! 0x4F0, twelve Persona slots of 52 bytes from 0x10CC and yen at 0x4A14. The game keeps an
//! MD5 of each save in the matching `.BINslot` (bytes 0x18–0x27), updated when the save is
//! written (see `formats::companion_files`).

use super::fields::{Builder, Checksum, Kind};
use super::p3p_tables::{ITEMS, PERSONAS};
use crate::tree::TreeDoc;
use std::collections::HashSet;

const LEVEL: usize = 0x3E;
const STATS: usize = 0x74;
const XP: usize = 0x7C;
const INVENTORY: usize = 0x4F0;
const SLOTS: usize = 0x10CC;
const YEN: usize = 0x4A14;
const PERSONA_SIZE: usize = 52;

pub fn looks(d: &[u8]) -> bool {
    // Both names start with the game's own text marker (0x80), and the Persona 4 Golden
    // header ("SG" at 8) is not there.
    d.len() >= YEN + 4
        && d.len() < 200_000
        && d.get(8..10) != Some(b"SG")
        && d[0x0C] == 0x80
        && d[0x1E] == 0x80
        && (1..=99).contains(&d[LEVEL])
        && u32::from_le_bytes([d[YEN], d[YEN + 1], d[YEN + 2], d[YEN + 3]]) <= 9_999_999
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Persona 3 Portable save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Protagonist", "Yen"], YEN, Kind::U32, 0, 9_999_999);
    b.ranged(&["Protagonist", "Level"], LEVEL, Kind::U8, 1, 99);
    b.ranged(&["Protagonist", "Total experience"], XP, Kind::U32, 0, 99_999_999);
    for (i, s) in ["Academics", "Charm", "Courage"].iter().enumerate() {
        b.ranged(&["Social stats", s], STATS + 2 * i, Kind::U16, 0, 999);
    }
    for i in 0..12 {
        let at = SLOTS + i * PERSONA_SIZE;
        let id = match d.get(at + 2..at + 4) {
            Some(v) => u16::from_le_bytes([v[0], v[1]]) as usize,
            None => break,
        };
        if d.get(at..at + 2) == Some(&[0, 0]) || id == 0 {
            break;
        }
        let name = PERSONAS.get(id).copied().unwrap_or("Unknown Persona");
        let slot = format!("Slot {} {}", i + 1, name);
        b.ranged(&["Personas", &slot, "Level"], at + 4, Kind::U8, 1, 99);
        b.ranged(&["Personas", &slot, "Total experience"], at + 8, Kind::U32, 0, 99_999_999);
        for (k, s) in ["Strength", "Magic", "Endurance", "Agility", "Luck"].iter().enumerate() {
            b.ranged(&["Personas", &slot, s], at + 28 + k, Kind::U8, 1, 99);
        }
    }
    let mut seen = HashSet::new();
    for (id, cat, name) in ITEMS {
        let at = INVENTORY + *id as usize;
        if at >= d.len() {
            continue;
        }
        let label = if seen.insert((*cat, *name)) { name.to_string() } else { format!("{name} (#{id})") };
        b.ranged(&["Items", cat, &label], at, Kind::U8, 0, 99);
    }
    Ok(b.finish("Persona 3 Portable save", Checksum::None).with_note(
        "Yen, level, experience, social stats, the Persona slots in use and every item count. The matching .BINslot file's checksum is updated when you save. Key items are left out (the game needs a flag as well).",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 0x4B00];
        d[0x0C] = 0x80;
        d[0x1E] = 0x80;
        d[LEVEL] = 20;
        d[YEN..YEN + 4].copy_from_slice(&1234u32.to_le_bytes());
        d[SLOTS..SLOTS + 2].copy_from_slice(&1u16.to_le_bytes());
        d[SLOTS + 2..SLOTS + 4].copy_from_slice(&1u16.to_le_bytes());
        d[SLOTS + 4] = 9;
        d[INVENTORY + 100] = 3;
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = sample();
        let mut t = parse(&d).unwrap();
        let s = serde_json::to_string(t.view_and_editable().0).unwrap();
        assert!(s.contains("1234") && s.contains("Orpheus"));
        t.set("/Protagonist/Yen", serde_json::json!(5000)).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes(out[YEN..YEN + 4].try_into().unwrap()), 5000);
        assert!(!looks(&d[..100]));
    }
}
