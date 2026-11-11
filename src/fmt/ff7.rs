//! Final Fantasy VII (1998 PC / 2013 Steam re-release) `save00.ff7` … `save09.ff7`: a
//! 9-byte header and 15 save slots of 0x10F4 bytes, each starting with a CRC-16 of the
//! rest. Layout as documented on the Qhimm wiki ("FF7/Savemap").

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const HEADER: usize = 9;
const SLOT: usize = 0x10F4;
const SLOTS: usize = 15;
const RECORD: usize = 0x84;
const ROSTER: [&str; 11] = ["Cloud", "Barret", "Tifa", "Aerith", "Red XIII", "Yuffie", "Cait Sith", "Vincent", "Cid", "Young Cloud", "Sephiroth"];

fn crc(b: &[u8]) -> u16 {
    let mut r: u32 = 0xFFFF;
    for &t in b {
        r ^= u32::from(t) << 8;
        for _ in 0..8 {
            r = if r & 0x8000 != 0 { (r << 1) ^ 0x1021 } else { r << 1 };
            r &= 0xFFFF;
        }
    }
    (r ^ 0xFFFF) as u16
}

fn fix_checksums(d: &mut Vec<u8>) {
    for i in 0..SLOTS {
        let s = HEADER + i * SLOT;
        if d.len() < s + SLOT {
            break;
        }
        let c = crc(&d[s + 4..s + 4 + 0x10F0]);
        d[s..s + 2].copy_from_slice(&c.to_le_bytes());
    }
}

/// FF7's text: byte + 0x20 for the common range, 0xFF ends it.
fn text(b: &[u8]) -> String {
    b.iter().take_while(|&&c| c != 0xFF).filter(|&&c| c < 0x60).map(|&c| (c + 0x20) as char).collect()
}

pub fn looks(d: &[u8]) -> bool {
    if d.len() != HEADER + SLOTS * SLOT {
        return false;
    }
    // At least one slot in use, and every used slot's checksum right.
    let mut used = 0;
    for i in 0..SLOTS {
        let s = HEADER + i * SLOT;
        let sl = &d[s..s + SLOT];
        if sl[4..].iter().all(|&b| b == 0) {
            continue;
        }
        if u16::from_le_bytes([sl[0], sl[1]]) != crc(&sl[4..4 + 0x10F0]) {
            return false;
        }
        used += 1;
    }
    used > 0
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Final Fantasy VII save".into());
    }
    let mut b = Builder::new(d);
    for i in 0..SLOTS {
        let s = HEADER + i * SLOT;
        if d[s + 4..s + SLOT].iter().all(|&x| x == 0) {
            continue;
        }
        let slot = format!("Slot {}", i + 1);
        let sl = slot.as_str();
        b.info(&[sl, "Location"], text(&d[s + 0x28..s + 0x48]));
        b.ranged(&[sl, "Gil"], s + 0xB7C, Kind::U32, 0, 99_999_999);
        b.mirror(&[sl, "Gil"], s + 0x20);
        b.ranged(&[sl, "Play time (seconds)"], s + 0xB80, Kind::U32, 0, 359_999);
        b.mirror(&[sl, "Play time (seconds)"], s + 0x24);
        for c in 0..9 {
            let r = s + 0x54 + c * RECORD;
            let id = d[r] as usize;
            let mut name = text(&d[r + 0x10..r + 0x1C]);
            if name.is_empty() {
                name = ROSTER.get(id).copied().unwrap_or("Character").to_string();
            }
            let role = ROSTER.get(id).copied().unwrap_or("?");
            // Two records are shared: young Cloud and Sephiroth (the Kalm flashback) hold the
            // places Cait Sith and Vincent take when they join.
            let role = match id {
                9 => "Young Cloud \u{2014} Cait Sith's place until he joins",
                10 => "Sephiroth \u{2014} Vincent's place until he joins",
                _ => role,
            };
            let who = if name == role || (id < 9 && name == ROSTER[id]) { name.clone() } else { format!("{name} ({role})") };
            let p = |k: &str| [sl, "Party", who.as_str(), k].map(|x| x.to_string());
            macro_rules! f {
                ($k:expr, $off:expr, $kind:expr, $lo:expr, $hi:expr) => {{
                    let path = p($k);
                    let refs: Vec<&str> = path.iter().map(String::as_str).collect();
                    b.ranged(&refs, r + $off, $kind, $lo, $hi);
                }};
            }
            f!("Level", 0x01, Kind::U8, 1, 99);
            f!("Strength", 0x02, Kind::U8, 0, 255);
            f!("Vitality", 0x03, Kind::U8, 0, 255);
            f!("Magic", 0x04, Kind::U8, 0, 255);
            f!("Spirit", 0x05, Kind::U8, 0, 255);
            f!("Dexterity", 0x06, Kind::U8, 0, 255);
            f!("Luck", 0x07, Kind::U8, 0, 255);
            f!("Limit level", 0x0E, Kind::U8, 1, 4);
            f!("Limit gauge", 0x0F, Kind::U8, 0, 255);
            f!("Kills", 0x24, Kind::U16, 0, 65535);
            f!("HP", 0x2C, Kind::U16, 0, 9999);
            f!("Base max HP", 0x2E, Kind::U16, 1, 9999);
            f!("MP", 0x30, Kind::U16, 0, 999);
            f!("Base max MP", 0x32, Kind::U16, 0, 999);
            f!("Max HP", 0x38, Kind::U16, 1, 9999);
            f!("Max MP", 0x3A, Kind::U16, 0, 999);
            f!("Experience", 0x3C, Kind::U32, 0, 99_999_999);
        }
    }
    Ok(b.finish("Final Fantasy VII save", Checksum::Custom(fix_checksums))
        .with_note("Each slot's checksum is updated when you save. Max HP/MP are recalculated by the game from level and equipment when needed."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_reference() {
        // CRC-16 (0x1021, init and final xor 0xFFFF) of "123456789".
        assert_eq!(crc(b"123456789"), 0xD64E);
    }

    /// Research aid: SAVE_EDITOR_FF7 = a save00.ff7 to read.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_FF7").unwrap()).unwrap();
        let mut doc = parse(&d).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view).unwrap().chars().take(2500).collect::<String>());
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Gil", serde_json::Value::from(5000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(u32::from_le_bytes(out[HEADER + 0x20..HEADER + 0x24].try_into().unwrap()), 5000);
    }
}
