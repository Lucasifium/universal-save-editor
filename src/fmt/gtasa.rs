//! Grand Theft Auto: San Andreas (PC, original release): `GTASAsf1.b` … `GTASAsf8.b`, a fixed 202,752-byte file
//! of `BLOCK` sections (34 markers; the player info, statistics and one more block are stored twice) followed by
//! a u32 sum of every other byte.
//!
//! Where the values sit follows the public documentation of the layout (facts); the offsets were checked against a
//! public save. Offsets below are counted from the end of a section's `BLOCK` marker.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "GTA San Andreas save";
const SIZE: usize = 202_752;
const MARK: &[u8] = b"BLOCK";

const STORY: [&str; 5] = ["Los Santos", "San Fierro", "Las Venturas", "Back to Los Santos", "Completed"];
const DAYS: [(&str, u64); 7] = [("Sunday", 1), ("Monday", 2), ("Tuesday", 3), ("Wednesday", 4), ("Thursday", 5), ("Friday", 6), ("Saturday", 7)];
const WEAPONS: [(&str, u64); 36] = [
    ("None", 0), ("Brass knuckles", 1), ("Golf club", 2), ("Nightstick", 3), ("Knife", 4), ("Baseball bat", 5), ("Shovel", 6), ("Pool cue", 7), ("Katana", 8),
    ("Chainsaw", 9), ("Purple dildo", 10), ("Dildo", 11), ("Vibrator", 12), ("Silver vibrator", 13), ("Flowers", 14), ("Cane", 15), ("Grenade", 16),
    ("Tear gas", 17), ("Molotov cocktail", 18), ("Pistol", 22), ("Silenced pistol", 23), ("Desert Eagle", 24), ("Shotgun", 25), ("Sawn-off shotgun", 26),
    ("Combat shotgun", 27), ("Micro SMG", 28), ("SMG", 29), ("AK-47", 30), ("M4", 31), ("Tec-9", 32), ("Country rifle", 33), ("Sniper rifle", 34),
    ("RPG", 35), ("Flamethrower", 37), ("Minigun", 38), ("Spray can", 41),
];
const MORE_WEAPONS: [(&str, u64); 10] = [
    ("Heat-seeking RPG", 36), ("Satchel charges", 39), ("Detonator", 40), ("Fire extinguisher", 42), ("Camera", 43), ("Night-vision goggles", 44),
    ("Thermal goggles", 45), ("Parachute", 46), ("Jetpack", 47), ("Unknown", 48),
];
const SKILLS: [(&str, usize); 11] = [
    ("Pistol", 0x114), ("Silenced pistol", 0x118), ("Desert Eagle", 0x11c), ("Shotgun", 0x120), ("Sawn-off shotgun", 0x124), ("Combat shotgun", 0x128),
    ("Machine pistol", 0x12c), ("SMG", 0x130), ("AK-47", 0x134), ("M4", 0x138), ("Gambling", 0x144),
];

fn blocks(d: &[u8]) -> Option<Vec<usize>> {
    if d.len() != SIZE || !d.starts_with(MARK) {
        return None;
    }
    let body = &d[..SIZE - 4];
    let mut v = Vec::with_capacity(34);
    let mut at = 0;
    while v.len() < 34 {
        let i = body[at..].windows(MARK.len()).position(|w| w == MARK)? + at;
        v.push(i + MARK.len());
        at = i + MARK.len();
    }
    // Section 10 (the script block) is always 18,892 bytes long.
    (v[11] - v[10] - MARK.len() == 18_892 && v[0] == MARK.len()).then_some(v)
}

pub fn looks(d: &[u8]) -> bool {
    blocks(d).is_some()
}

fn fix_sum(d: &mut Vec<u8>) {
    let n = d.len() - 4;
    let sum = d[..n].iter().fold(0u32, |a, &b| a.wrapping_add(u32::from(b)));
    d[n..].copy_from_slice(&sum.to_le_bytes());
}

fn options(list: &[(&str, u64)]) -> Vec<(String, u64)> {
    list.iter().map(|&(n, v)| (n.to_string(), v)).collect()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let bl = blocks(d).ok_or("not a GTA San Andreas save")?;
    let mut b = Builder::new(d);
    let (b0, b2, b15, b16) = (bl[0], bl[2], bl[15], bl[16]);
    let (c15, c16) = (bl[30], bl[31]);
    let g = "Player";
    let text: Vec<u8> = d[b0 + 4..b0 + 104].iter().copied().take_while(|&c| c != 0).collect();
    b.info(&[g, "Save title"], String::from_utf8_lossy(&text).into_owned());
    b.ranged(&[g, "Money"], b15 + 4, Kind::I32, -999_999_999, 999_999_999);
    b.mirror(&[g, "Money"], c15 + 4);
    b.ranged(&[g, "Money shown on screen"], b15 + 0x10, Kind::I32, -999_999_999, 999_999_999);
    b.mirror(&[g, "Money shown on screen"], c15 + 0x10);
    b.ranged(&[g, "Health"], b2 + 0x20, Kind::F32Int, 0, 1000);
    b.ranged(&[g, "Armor"], b2 + 0x24, Kind::F32Int, 0, 150);
    b.ranged(&[g, "Maximum health"], b16 + 0x60, Kind::F32Int, 569, 1000);
    b.mirror(&[g, "Maximum health"], c16 + 0x60);
    b.ranged(&[g, "Respect"], b16 + 0x100, Kind::F32Int, 0, 1000);
    b.mirror(&[g, "Respect"], c16 + 0x100);
    b.ranged(&[g, "Fat"], b16 + 0x54, Kind::F32Int, 0, 1000);
    b.mirror(&[g, "Fat"], c16 + 0x54);
    b.ranged(&[g, "Stamina"], b16 + 0x58, Kind::F32Int, 0, 1000);
    b.mirror(&[g, "Stamina"], c16 + 0x58);
    b.ranged(&[g, "Muscle"], b16 + 0x5c, Kind::F32Int, 0, 1000);
    b.mirror(&[g, "Muscle"], c16 + 0x5c);
    b.ranged(&[g, "Sex appeal"], b16 + 0x140, Kind::F32Int, 0, 1000);
    b.mirror(&[g, "Sex appeal"], c16 + 0x140);
    b.named(&[g, "Story line"], b16 + 0x23c, Kind::I32, &STORY);
    b.mirror(&[g, "Story line"], c16 + 0x23c);
    for (label, at, mx) in [("Driving skill", 0x1e8, 1000), ("Flying skill", 0x2e4, 1000), ("Lung capacity", 0x2ec, 1000), ("Bike skill", 0x2fc, 1000), ("Cycling skill", 0x300, 1000)] {
        b.ranged(&["Skills", label], b16 + at, Kind::I32, 0, mx);
        b.mirror(&["Skills", label], c16 + at);
    }
    for (label, at) in SKILLS {
        let path = ["Skills", label];
        b.ranged(&path, b16 + at, Kind::F32Int, 0, 1000);
        b.mirror(&path, c16 + at);
    }
    for (label, at, mx) in [("Times busted", 0x17c, 999_999), ("Times wasted", 0x184, 999_999), ("Times cheated", 0x18c, 999_999), ("Days passed", 0x180, 99_999)] {
        b.ranged(&["Statistics", label], b16 + at, Kind::I32, 0, mx);
        b.mirror(&["Statistics", label], c16 + at);
    }
    b.ranged(&["Statistics", "Minute length (milliseconds)"], b0 + 0x7c, Kind::I32, 100, 60_000);
    b.ranged(&["Statistics", "Hour"], b0 + 0x86, Kind::U8, 0, 23);
    b.ranged(&["Statistics", "Minute"], b0 + 0x87, Kind::U8, 0, 59);
    b.pick(&["Statistics", "Day of the week"], b0 + 0x88, Kind::U8, &options(&DAYS));
    for (label, at, kind) in [("Riots in Los Santos", b0 + 0xe0, Kind::B8), ("All taxis have nitro", b0 + 0x135, Kind::B8), ("Prostitutes pay you", b0 + 0x136, Kind::B8)] {
        b.field(&["Options", label], at, kind);
    }
    for (label, at) in [("Infinite sprint", 0x20), ("Fast reload", 0x21), ("Fireproof", 0x22), ("Free release once busted", 0x25), ("Free release once wasted", 0x26), ("Drive-by shooting", 0x27)] {
        b.field(&["Options", label], b15 + at, Kind::B8);
        b.mirror(&["Options", label], c15 + at);
    }
    let all: Vec<(String, u64)> = options(&WEAPONS).into_iter().chain(options(&MORE_WEAPONS)).collect();
    for slot in 0..13 {
        let name = format!("Slot {}", slot + 1);
        b.pick(&["Weapons", &name, "Weapon"], b2 + 0x28 + 28 * slot, Kind::I32, &all);
        b.ranged(&["Weapons", &name, "Ammo"], b2 + 0x34 + 28 * slot, Kind::I32, 0, 99_999);
    }
    Ok(b.finish(KIND, Checksum::Custom(fix_sum)).with_note(crate::i18n::tr("Close the game first. The save's checksum is rewritten, and the values the save keeps twice are written to both places. Health 569 is full health at the standard maximum.")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        let mut at = 0;
        for i in 0..34 {
            d[at..at + 5].copy_from_slice(MARK);
            at += 5;
            at += match i {
                10 => 18_892,
                16 | 31 => 1940,
                2 => 0x300,
                _ => 60,
            };
        }
        d[0x20..0x24].copy_from_slice(&1.0f32.to_le_bytes());
        fix_sum(&mut d);
        d
    }

    #[test]
    fn money_goes_to_both_copies_and_the_sum_is_fixed() {
        let d = sample();
        let bl = blocks(&d).unwrap();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Player/Money", Value::from(777_777_777)).unwrap();
        doc.set("/Weapons/Slot 3/Ammo", Value::from(500)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(i32::from_le_bytes(out[bl[15] + 4..bl[15] + 8].try_into().unwrap()), 777_777_777);
        assert_eq!(i32::from_le_bytes(out[bl[30] + 4..bl[30] + 8].try_into().unwrap()), 777_777_777);
        assert_eq!(i32::from_le_bytes(out[bl[2] + 0x34 + 56..bl[2] + 0x38 + 56].try_into().unwrap()), 500);
        let n = out.len() - 4;
        assert_eq!(out[..n].iter().fold(0u32, |a, &b| a.wrapping_add(u32::from(b))), u32::from_le_bytes(out[n..].try_into().unwrap()));
    }

    #[test]
    fn public_sample_if_given() {
        let Ok(path) = std::env::var("GTASA_SAMPLE") else { return };
        let d = std::fs::read(path).unwrap();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        println!("{}", serde_json::to_string_pretty(doc.view_and_editable().0).unwrap());
        doc.set("/Player/Money", serde_json::Value::from(999_999_999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
    }
}
