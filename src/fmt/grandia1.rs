//! Grandia (PlayStation, Western and Japanese releases): one 8 KiB block per save. The block
//! header is one icon frame longer in the Western saves' counterpart: saves whose icon byte is 0x13
//! keep their data 0x180 bytes later than those with 0x11. The save has no checksum. Offsets
//! and names follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, title};
use super::retro::flag;
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "Grandia (PlayStation) save";

const CHARACTERS: [&str; 8] = ["Justin", "Feena", "Sue", "Gadwin", "Rapp", "Milda", "Guido", "Liete"];
/// "グランディア" in Shift-JIS.
const JAPANESE: [u8; 12] = [0x83, 0x4f, 0x83, 0x89, 0x83, 0x93, 0x83, 0x66, 0x83, 0x42, 0x83, 0x41];

fn shift(d: &[u8]) -> Option<usize> {
    match d.get(2)? {
        0x11 => Some(0),
        0x13 => Some(0x180),
        _ => None,
    }
}

pub fn looks(d: &[u8]) -> bool {
    if !is_save(d, 1) || !(title(d).starts_with("GRANDIA") || d[4..16] == JAPANESE) {
        return false;
    }
    let Some(s) = shift(d) else { return false };
    matches!(d[0x1a9 + s], 1 | 2 | 0x82) && d[0x313 + s] <= 99
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Grandia save".into());
    }
    let s = shift(d).unwrap_or(0);
    let mut names: HashMap<&str, Vec<(String, u64)>> = HashMap::new();
    let mut everything: Vec<(String, u64)> = vec![("-".to_string(), 0)];
    let mut skills: Vec<(String, String, usize)> = vec![];
    for l in include_str!("data/grandia1.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let f: Vec<&str> = l.split('\t').collect();
        if f[0] == "skill" {
            skills.push((f[2].to_string(), f[3].to_string(), f[1].parse().unwrap_or(0)));
            continue;
        }
        let id: u64 = f[1].parse().unwrap_or(0);
        names.entry(f[0]).or_insert_with(|| vec![("-".to_string(), 0)]).push((f[2].to_string(), id));
        everything.push((f[2].to_string(), id));
    }
    let empty = vec![("-".to_string(), 0u64)];
    let list = |k: &str| names.get(k).unwrap_or(&empty).clone();
    let mut b = Builder::new(d);
    b.ranged(&["General", "Gold pieces"], 0x1b0 + s, Kind::U32, 0, 9_999_999);
    for (c, n) in CHARACTERS.iter().enumerate() {
        let o = s + c * 0x80;
        b.ranged(&["Party", n, "Level"], 0x313 + o, Kind::U8, 1, 99);
        b.ranged(&["Party", n, "Experience"], 0x314 + o, Kind::U32, 0, 65535);
        b.ranged(&["Party", n, "Total experience"], 0x344 + o, Kind::U32, 0, 9_999_999);
        b.ranged(&["Party", n, "HP"], 0x31c + o, Kind::U16, 0, 999);
        b.ranged(&["Party", n, "Max HP"], 0x31a + o, Kind::U16, 1, 999);
        b.ranged(&["Party", n, "SP"], 0x328 + o, Kind::U16, 0, 999);
        b.ranged(&["Party", n, "Max SP"], 0x326 + o, Kind::U16, 0, 999);
        for (k, st) in ["Strength", "Vitality", "Wit", "Agility"].iter().enumerate() {
            b.ranged(&["Party", n, st], 0x31e + o + k * 2, Kind::U16, 0, 999);
        }
        for (k, el) in ["Fire", "Water", "Wind", "Earth"].iter().enumerate() {
            b.ranged(&["Magic", n, &format!("{el} level")], 0x33c + o + [0, 1, 2, 3][k], Kind::U8, 0, 99);
        }
        for (k, el) in ["Fire", "Water", "Wind", "Earth"].iter().enumerate() {
            b.ranged(&["Magic", n, &format!("{el} experience")], 0x32c + o + k * 2, Kind::U16, 0, 99);
        }
        for (k, (t, ty)) in ["Weapon", "Shield", "Armor", "Helmet", "Shoes", "Jewelry"].iter().zip(["weapons", "shields", "armors", "helmets", "shoes", "jewelries"]).enumerate() {
            b.pick(&["Equipment", n, t], 0x35c + o + k * 2, Kind::U16, &list(ty));
        }
        for k in 0..12 {
            b.pick(&["Items", n, &format!("Item {}", k + 1)], 0x368 + o + k * 2, Kind::U16, &everything);
        }
        for k in 0..4 {
            b.pick(&["Weapon skills", n, &format!("Weapon type {}", k + 1)], 0x384 + o + k, Kind::U8, &super::ps1::opts(&[("-", 0), ("Dagger", 1), ("Sword", 2), ("Mace", 3), ("Ax", 4), ("Whip", 5), ("Bow", 6)]));
            b.ranged(&["Weapon skills", n, &format!("Weapon level {}", k + 1)], 0x340 + o + k, Kind::U8, 0, 99);
            b.ranged(&["Weapon skills", n, &format!("Weapon experience {}", k + 1)], 0x334 + o + k * 2, Kind::U16, 0, 99);
        }
    }
    for (group, name, at) in &skills {
        let group = if group == "Skill" { "Techniques" } else { group };
        for (c, n) in CHARACTERS.iter().enumerate() {
            flag(&mut b, &[&format!("Skills: {group}"), name, n], at + s, c as u32);
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. The game keeps no checksum. A skill is learned by each character whose box says Yes; items in a character's pack can be anything the game knows (equipment, key items and consumables share one list).")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_gold_party_and_skills() {
        let mut d = vec![0u8; 0x2000];
        d[..2].copy_from_slice(b"SC");
        d[2] = 0x11;
        d[3] = 1;
        // "GRANDIA-1" in full-width Shift-JIS letters
        let t: Vec<u8> = "ＧＲＡＮＤＩＡ－１".chars().flat_map(|c| {
            let w = match c {
                'Ａ'..='Ｚ' => 0x8260 + (c as u32 - 'Ａ' as u32),
                '－' => 0x817c,
                '１' => 0x8250,
                _ => 0x8140,
            };
            [(w >> 8) as u8, w as u8]
        }).collect();
        d[4..4 + t.len()].copy_from_slice(&t);
        d[0x1a9] = 1;
        d[0x313] = 5;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/General/Gold pieces", serde_json::json!(777)).unwrap();
        doc.set("/Party/Feena/Level", serde_json::json!(12)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes(out[0x1b0..0x1b4].try_into().unwrap()), 777);
        assert_eq!(out[0x313 + 0x80], 12);
    }
}
