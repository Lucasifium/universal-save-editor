//! Falcom saves that are plain fixed-layout files: The Legend of Heroes: Trails in the Sky the
//! 3rd (`SVDATnnn.SAV`, 184,176 bytes) and Trails of Cold Steel (`saveNNN.dat`, 460,992 bytes).
//! Offsets and item tables follow the GameSaveEditor by Andrew Cornforth and contributors
//! (GPL-3.0); the character fields were checked against public saves.

use super::falcom_tables::{CS1_ITEMS, SKY3_ITEMS};
use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashSet;

pub const KIND_SKY3: &str = "Trails in the Sky the 3rd save";
pub const KIND_CS1: &str = "Trails of Cold Steel save";

const SKY3_SIZE: usize = 184_176;
const CS1_SIZE: usize = 460_992;

fn u16at(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

fn u32at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

pub fn looks_sky3(d: &[u8]) -> bool {
    d.len() == SKY3_SIZE
        && u32at(d, 0x26BCC) <= 99_999_999
        && (0..4).all(|i| matches!(u32at(d, 0x1F42C + 4 * i), 0..=18 | 255))
        && u16at(d, 0x1F48C) <= 200
}

pub fn looks_cs1(d: &[u8]) -> bool {
    d.len() == CS1_SIZE && u32at(d, 0) as usize == CS1_SIZE && u32at(d, 4) == 8 && u32at(d, 0x65814) <= 99_999_999
}

/// A name that is not used yet in this group.
fn unique(seen: &mut HashSet<String>, name: String, id: u16) -> String {
    if seen.insert(name.clone()) {
        name
    } else {
        let n = format!("{name} ({id})");
        seen.insert(n.clone());
        n
    }
}

pub fn parse_sky3(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_sky3(d) {
        return Err("not a Trails in the Sky the 3rd save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Party", "Mira"], 0x26BCC, Kind::U32, 0, 99_999_999);
    for (i, name) in ["Earth", "Water", "Fire", "Wind", "Time", "Space", "Mirage"].iter().enumerate() {
        b.ranged(&["Sepith", name], 0x26BD4 + 4 * i, Kind::U32, 0, 9999);
    }
    const NAMES: [&str; 19] = [
        "Estelle", "Joshua", "Scherazard", "Olivier", "Kloe", "Agate", "Tita", "Zin", "Kevin", "Anelace", "Josette", "Richard", "Mueller", "Julia", "Ries", "Renne",
        "Deen", "Rais", "Rocco",
    ];
    for (i, name) in NAMES.iter().enumerate() {
        let at = 0x1F48C + 60 * i;
        if u16at(d, at) == 0 {
            continue;
        }
        b.ranged(&["Characters", name, "Level"], at, Kind::U16, 1, 250);
    }
    let mut seen = HashSet::new();
    for i in 0..366 {
        let at = 0x22F0C + 4 * i;
        let id = u16at(d, at);
        if id == 0 {
            break;
        }
        let Some(&(_, name, max)) = SKY3_ITEMS.iter().find(|r| r.0 == id) else {
            let n = unique(&mut seen, format!("Item {id}"), id);
            b.ranged(&["Items", &n], at + 2, Kind::U16, 0, 99);
            continue;
        };
        let n = unique(&mut seen, name.to_string(), id);
        b.ranged(&["Items", &n], at + 2, Kind::U16, 0, i64::from(max));
    }
    Ok(b.finish(KIND_SKY3, Checksum::None).with_note(crate::i18n::tx("Close the game first. Counts of items already in the bag are edited; setting a count to 0 leaves the slot in the bag. The save has no checksum.")))
}

pub fn parse_cs1(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_cs1(d) {
        return Err("not a Trails of Cold Steel save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Party", "Mira"], 0x65814, Kind::U32, 0, 99_999_999);
    for (i, name) in ["Earth", "Water", "Fire", "Wind", "Time", "Space", "Mirage", "Mass"].iter().enumerate() {
        b.ranged(&["Sepith", name], 0x657D4 + 4 * i, Kind::U32, 0, 99_999);
    }
    const NAMES: [&str; 12] = ["Rean", "Alisa", "Elliot", "Laura", "Machias", "Emma", "Jusis", "Fie", "Gaius", "Millium", "Crow", "Sara"];
    for (i, name) in NAMES.iter().enumerate() {
        let at = 0x480 + 80 * i;
        if u16at(d, at + 36) == 0 {
            continue;
        }
        b.ranged(&["Characters", name, "Level"], at + 36, Kind::U16, 1, 99);
        b.ranged(&["Characters", name, "Experience"], at + 40, Kind::U32, 0, 99_999_999);
        b.ranged(&["Characters", name, "HP"], at, Kind::U32, 0, 99_999);
        b.ranged(&["Characters", name, "Max HP"], at + 4, Kind::U32, 1, 99_999);
        b.ranged(&["Characters", name, "EP"], at + 8, Kind::U16, 0, 9999);
        b.ranged(&["Characters", name, "Max EP"], at + 10, Kind::U16, 0, 9999);
        b.ranged(&["Characters", name, "CP"], at + 12, Kind::U16, 0, 200);
        b.ranged(&["Characters", name, "Max CP"], at + 14, Kind::U16, 0, 200);
        for (k, stat) in ["STR", "DEF", "ATS", "ADF", "SPD", "DEX", "AGL", "MOV", "RNG"].iter().enumerate() {
            b.ranged(&["Characters", name, stat], at + 16 + 2 * k, Kind::U16, 0, 9999);
        }
    }
    let mut seen = HashSet::new();
    let mut at = 0x40C24;
    for _ in 0..4000 {
        if at + 4 > d.len() {
            break;
        }
        let id = u16at(d, at);
        if id == 9999 {
            break;
        }
        let name = CS1_ITEMS.iter().find(|r| r.0 == id).map_or_else(|| format!("Item {id}"), |r| r.1.to_string());
        let n = unique(&mut seen, name, id);
        b.ranged(&["Items", &n], at + 2, Kind::U16, 0, 99);
        at += 0x24;
    }
    Ok(b.finish(KIND_CS1, Checksum::None).with_note(crate::i18n::tx("Close the game first. Counts of items already in the bag are edited; setting a count to 0 leaves the slot in the bag. The save has no checksum.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky3_edits_mira_and_items() {
        let mut d = vec![0u8; SKY3_SIZE];
        d[0x1F42C..0x1F42C + 4].copy_from_slice(&0u32.to_le_bytes());
        for i in 1..4 {
            d[0x1F42C + 4 * i..0x1F42C + 4 * i + 4].copy_from_slice(&255u32.to_le_bytes());
        }
        d[0x1F48C..0x1F48E].copy_from_slice(&90u16.to_le_bytes());
        d[0x26BCC..0x26BD0].copy_from_slice(&1234u32.to_le_bytes());
        d[0x22F0C..0x22F0E].copy_from_slice(&501u16.to_le_bytes());
        d[0x22F0E..0x22F10].copy_from_slice(&4u16.to_le_bytes());
        let mut doc = parse_sky3(&d).unwrap();
        assert_eq!(doc.view.pointer("/Party/Mira").unwrap(), 1234);
        assert_eq!(doc.view.pointer("/Characters/Estelle/Level").unwrap(), 90);
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Party/Mira", 5_000_000.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32at(&out, 0x26BCC), 5_000_000);
        assert!(doc.set("/Characters/Estelle/Level", 999.into()).is_err());
    }

    #[test]
    fn cs1_walks_the_bag() {
        let mut d = vec![0u8; CS1_SIZE];
        d[..4].copy_from_slice(&(CS1_SIZE as u32).to_le_bytes());
        d[4] = 8;
        d[0x65814..0x65818].copy_from_slice(&29685u32.to_le_bytes());
        d[0x480 + 36] = 80;
        d[0x40C24..0x40C28].copy_from_slice(&[0, 0, 30, 0]);
        d[0x40C24 + 0x24..0x40C24 + 0x24 + 4].copy_from_slice(&[0x0F, 0x27, 0, 0]);
        let doc = parse_cs1(&d).unwrap();
        assert_eq!(doc.view.pointer("/Party/Mira").unwrap(), 29685);
        assert_eq!(doc.view.pointer("/Items/Tear Balm").unwrap(), 30);
    }
}
