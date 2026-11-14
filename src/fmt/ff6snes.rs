//! Final Fantasy VI (SNES, "Final Fantasy III" in the USA) battery saves (`.srm` / `.sav`,
//! 8 KiB): three slots of 0xa00 bytes. A slot starts with 16 character records of 0x25 bytes
//! and ends in a little-endian 16-bit sum of the slot's other bytes. Money, steps and play time
//! sit at 0x260-0x268, the item ids at 0x269 and their quantities at 0x369.
//!
//! Layout and name tables follow the MIT-licensed save editor in
//! RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md); the sums are checked on a
//! public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::OnceLock;

pub const KIND: &str = "Final Fantasy VI (SNES) save";
const SLOT: usize = 0xa00;

struct Names {
    items: Vec<(u64, String)>,
    chars: Vec<(u8, String)>,
    espers: Vec<(u64, String)>,
}

fn names() -> &'static Names {
    static N: OnceLock<Names> = OnceLock::new();
    N.get_or_init(|| {
        let mut n = Names { items: vec![], chars: vec![], espers: vec![] };
        for l in include_str!("data/ff6_snes.txt").lines() {
            let mut p = l.splitn(3, ':');
            let (Some(t), Some(i), Some(name)) = (p.next(), p.next(), p.next()) else { continue };
            let Ok(i) = i.parse::<u64>() else { continue };
            match t {
                "item" => n.items.push((i, name.to_string())),
                "char" => n.chars.push((i as u8, name.to_string())),
                "esper" => n.espers.push((i, name.to_string())),
                _ => {}
            }
        }
        n
    })
}

fn sum(d: &[u8], s: usize) -> u16 {
    d[s..s + 0x9fe].iter().fold(0u16, |a, &b| a.wrapping_add(b as u16))
}

fn valid(d: &[u8], n: usize) -> bool {
    let s = n * SLOT;
    let have = u16::from_le_bytes([d[s + 0x9fe], d[s + 0x9ff]]);
    have != 0 && sum(d, s) == have
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == 0x2000 && (0..3).any(|n| valid(d, n))
}

/// The game's text: A-Z 0x80.., a-z 0x9a.., 0-9 0xb4.., 0xff is blank.
fn text(b: &[u8]) -> String {
    b.iter()
        .filter_map(|&c| match c {
            0x80..=0x99 => Some((b'A' + (c - 0x80)) as char),
            0x9a..=0xb3 => Some((b'a' + (c - 0x9a)) as char),
            0xb4..=0xbd => Some((b'0' + (c - 0xb4)) as char),
            _ => None,
        })
        .collect()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Final Fantasy VI (SNES) save".into());
    }
    let nm = names();
    let items: Vec<(String, u64)> = nm.items.iter().map(|(i, n)| (n.clone(), *i)).chain([("(empty)".to_string(), 0xff)]).collect();
    let espers: Vec<(String, u64)> = nm.espers.iter().map(|(i, n)| (n.clone(), *i)).chain([("(none)".to_string(), 0xff)]).collect();
    let mut b = Builder::new(d);
    let mut found = 0;
    for n in 0..3 {
        if !valid(d, n) {
            continue;
        }
        found += 1;
        let s = n * SLOT;
        let slot = ["Slot 1", "Slot 2", "Slot 3"][n];
        b.bits(&[slot, "Game", "Money"], s + 0x260, Kind::U32, 0, 24, 0, 0, 9_999_999);
        b.bits(&[slot, "Game", "Steps"], s + 0x266, Kind::U32, 0, 24, 0, 0, 9_999_999);
        b.ranged(&[slot, "Game", "Play time hours"], s + 0x263, Kind::U8, 0, 99);
        b.ranged(&[slot, "Game", "Play time minutes"], s + 0x264, Kind::U8, 0, 59);
        b.ranged(&[slot, "Game", "Play time seconds"], s + 0x265, Kind::U8, 0, 59);
        let mut seen: Vec<String> = vec![];
        for c in 0..16 {
            let r = s + c * 0x25;
            let id = d[r];
            if id == 0xff || d[r + 8] == 0 {
                continue;
            }
            let base = nm.chars.iter().find(|(i, _)| *i == id).map_or("Character".to_string(), |(_, n)| n.clone());
            let given = text(&d[r + 2..r + 8]);
            let mut key = if given.is_empty() || given == base { base } else { format!("{base} ({given})") };
            if seen.contains(&key) {
                key = format!("{key} #{}", c + 1);
            }
            seen.push(key.clone());
            let k = key.as_str();
            let p = |col: &'static str| [slot, "Party", k, col];
            b.ranged(&p("Level"), r + 8, Kind::U8, 1, 99);
            b.ranged(&p("HP"), r + 9, Kind::U16, 0, 9999);
            b.bits(&p("Max HP"), r + 0xb, Kind::U16, 0, 14, 0, 1, 9999);
            b.ranged(&p("MP"), r + 0xd, Kind::U16, 0, 999);
            b.bits(&p("Max MP"), r + 0xf, Kind::U16, 0, 14, 0, 0, 999);
            b.bits(&p("Experience"), r + 0x11, Kind::U32, 0, 24, 0, 0, 15_000_000);
            b.ranged(&p("Vigor"), r + 0x1a, Kind::U8, 0, 128);
            b.ranged(&p("Speed"), r + 0x1b, Kind::U8, 0, 128);
            b.ranged(&p("Stamina"), r + 0x1c, Kind::U8, 0, 128);
            b.ranged(&p("Magic power"), r + 0x1d, Kind::U8, 0, 128);
            b.pick(&p("Esper"), r + 0x1e, Kind::U8, &espers);
            for (i, col) in ["Right hand", "Left hand", "Head", "Body", "Relic 1", "Relic 2"].into_iter().enumerate() {
                b.pick(&[slot, "Equipment", k, col], r + 0x1f + i, Kind::U8, &items);
            }
        }
        for i in 0..256 {
            if d[s + 0x269 + i] == 0xff {
                continue;
            }
            let key = format!("{:03}", i + 1);
            b.pick(&[slot, "Inventory", &key, "Item"], s + 0x269 + i, Kind::U8, &items);
            b.ranged(&[slot, "Inventory", &key, "Quantity"], s + 0x369 + i, Kind::U8, 1, 99);
        }
    }
    b.info(&["Info", "Slots in use"], found);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Each slot's sum is rewritten on save."))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..3 {
        let s = n * SLOT;
        if s + SLOT <= d.len() && (d[s + 0x9fe] != 0 || d[s + 0x9ff] != 0) {
            let v = sum(d, s);
            d[s + 0x9fe..s + 0xa00].copy_from_slice(&v.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_character_and_money() {
        let mut d = vec![0u8; 0x2000];
        for c in 0..16 {
            d[c * 0x25] = 0xff;
        }
        d[0] = 0;
        d[2..8].copy_from_slice(&[0x95, 0x84, 0x91, 0x91, 0x80, 0xff]);
        d[8] = 4;
        d[0xb..0xd].copy_from_slice(&77u16.to_le_bytes());
        d[0x269] = 0xff;
        d[0x270] = 1;
        d[0x369 + 7] = 5;
        d[0x9fe] = 1;
        fix(&mut d);
        assert!(valid(&d, 0));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Game/Money", 999_999.into()).unwrap();
        doc.set("/Slot 1/Party/Terra (VERRA)/Level", 50.into()).unwrap();
        doc.set("/Slot 1/Party/Terra (VERRA)/Experience", 123_456.into()).unwrap();
        doc.set("/Slot 1/Inventory/008/Quantity", 99.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[0x260..0x263], &[0x3f, 0x42, 0x0f]);
        assert_eq!(out[8], 50);
        assert_eq!(&out[0x11..0x14], &[0x40, 0xe2, 0x01]);
        assert_eq!(out[0x369 + 7], 99);
        assert!(valid(&out, 0));
    }
}
