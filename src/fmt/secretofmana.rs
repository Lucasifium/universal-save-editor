//! Secret of Mana (SNES, "Seiken Densetsu 2") battery saves (`.srm` / `.sav`, 8 KiB): four
//! slots of 0x400 bytes. A used slot starts with 0xc5, then a little-endian 16-bit sum of the
//! bytes 3..0x2bb. Money is at 0x6d (24 bits); the three characters' records are 0x80 apart
//! (level - 1 at +0x104, HP +0x105, MP +0x109, experience +0x110), their weapon and mana
//! spirit levels are nibbles at +0x143 and the matching experience bytes follow at +0x153.
//!
//! Layout follows the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md); the sums are checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Secret of Mana (SNES) save";
const SLOT: usize = 0x400;
const WHO: [&str; 3] = ["Boy", "Girl", "Sprite"];
const SKILLS: [&str; 16] = [
    "Gloves", "Sword", "Axe", "Spear", "Whip", "Bow", "Boomerang", "Javelin", "Gnome", "Undine", "Salamando", "Sylphid", "Luna", "Dryad", "Shade", "Lumina",
];

fn sum(d: &[u8], s: usize) -> u16 {
    d[s + 3..s + 0x2bb].iter().fold(0u16, |a, &b| a.wrapping_add(b as u16))
}

fn valid(d: &[u8], n: usize) -> bool {
    let s = n * SLOT;
    d[s] == 0xc5 && sum(d, s) == u16::from_le_bytes([d[s + 1], d[s + 2]])
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == 0x2000 && (0..4).any(|n| valid(d, n)) && (0..4).all(|n| d[n * SLOT] == 0xc5 || d[n * SLOT] == 0xff || d[n * SLOT] == 0)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Secret of Mana (SNES) save".into());
    }
    let mut b = Builder::new(d);
    let mut found = 0;
    for n in 0..4 {
        if !valid(d, n) {
            continue;
        }
        found += 1;
        let s = n * SLOT;
        let slot = ["Slot 1", "Slot 2", "Slot 3", "Slot 4"][n];
        b.bits(&[slot, "Game", "Money"], s + 0x6d, Kind::U32, 0, 24, 0, 0, 9_999_999);
        for (c, who) in WHO.iter().enumerate() {
            let r = s + c * 0x80;
            b.bits(&[slot, "Party", who, "Level"], r + 0x104, Kind::U8, 0, 8, 1, 1, 99);
            b.bits(&[slot, "Party", who, "Experience"], r + 0x110, Kind::U32, 0, 24, 0, 0, 9_999_999);
            b.ranged(&[slot, "Party", who, "HP"], r + 0x105, Kind::U16, 0, 999);
            b.ranged(&[slot, "Party", who, "MP"], r + 0x109, Kind::U8, 0, 99);
            for (i, skill) in SKILLS.iter().enumerate() {
                let key = format!("{who}: {skill}");
                let shift = if i % 2 == 0 { 4 } else { 0 };
                b.bits(&[slot, "Skills", &key, "Level"], r + 0x143 + i / 2, Kind::U8, shift, 4, 0, 0, 8);
                b.ranged(&[slot, "Skills", &key, "Experience"], r + 0x153 + i, Kind::U8, 0, 99);
            }
        }
    }
    b.info(&["Info", "Slots in use"], found);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Strength, defense and the other derived stats are worked out by the game from level and equipment, so they're not shown. Each slot's sum is rewritten on save."))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..4 {
        let s = n * SLOT;
        if d[s] == 0xc5 {
            let v = sum(d, s);
            d[s + 1..s + 3].copy_from_slice(&v.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_level_and_skill() {
        let mut d = vec![0xffu8; 0x2000];
        d[..0x400].fill(0);
        d[0] = 0xc5;
        d[0x6d] = 6;
        d[0x143] = 0x31;
        fix(&mut d);
        assert!(valid(&d, 0));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Game/Money", 9_999_999.into()).unwrap();
        doc.set("/Slot 1/Party/Boy/Level", 50.into()).unwrap();
        doc.set("/Slot 1/Skills/Boy: Sword/Level", 8.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[0x6d..0x70], &[0x7f, 0x96, 0x98]);
        assert_eq!(out[0x104], 49);
        assert_eq!(out[0x143], 0x38);
        assert!(valid(&out, 0));
    }
}
