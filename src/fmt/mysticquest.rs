//! Final Fantasy Mystic Quest (SNES, "Mystic Quest Legend") battery saves (`.srm` / `.sav`,
//! 8 KiB): three slots of 0x38c bytes. A used slot has a non-zero first byte and a 16-bit
//! little-endian sum (mod 0xffff) of the words from +6 to the end, stored at +4. Money is at
//! 0xa6 (24 bits); the hero's and the partner's records are 0x50 apart (level +0x16, experience
//! +0x17, HP +0x1a, max HP +0x1c, spell counts +0x1e..0x20 and their maxima +0x21..0x23).
//!
//! Layout follows the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md); the sums are checked on public saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Mystic Quest (SNES) save";
const SLOT: usize = 0x38c;

fn sum(d: &[u8], s: usize) -> u16 {
    let mut c: u32 = 0;
    let mut i = s + 6;
    while i + 1 < s + SLOT {
        c = (c + u16::from_le_bytes([d[i], d[i + 1]]) as u32) % 0xffff;
        i += 2;
    }
    c as u16
}

fn valid(d: &[u8], n: usize) -> bool {
    let s = n * SLOT;
    d[s] != 0 && sum(d, s) == u16::from_le_bytes([d[s + 4], d[s + 5]])
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == 0x2000 && (0..3).any(|n| valid(d, n)) && (0..3).all(|n| d[n * SLOT] == 0 || valid(d, n))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Mystic Quest (SNES) save".into());
    }
    let mut b = Builder::new(d);
    let mut found = 0;
    for n in 0..3 {
        if !valid(d, n) {
            continue;
        }
        found += 1;
        let s = n * SLOT;
        let slot = ["Slot 1", "Slot 2", "Slot 3"][n];
        b.bits(&[slot, "Game", "Money"], s + 0xa6, Kind::U32, 0, 24, 0, 0, 9_999_999);
        for (c, who) in ["Hero", "Partner"].iter().enumerate() {
            let r = s + c * 0x50;
            if d[r + 0x16] == 0xff {
                continue;
            }
            b.ranged(&[slot, "Party", who, "Level"], r + 0x16, Kind::U8, 1, 41);
            b.bits(&[slot, "Party", who, "Experience"], r + 0x17, Kind::U32, 0, 24, 0, 0, 9_999_999);
            b.ranged(&[slot, "Party", who, "HP"], r + 0x1a, Kind::U16, 0, 999);
            b.ranged(&[slot, "Party", who, "Max HP"], r + 0x1c, Kind::U16, 1, 999);
            for (i, m) in ["White magic", "Black magic", "Sorcery"].iter().enumerate() {
                b.ranged(&[slot, "Party", who, &format!("{m} charges")], r + 0x1e + i, Kind::U8, 0, 99);
                b.ranged(&[slot, "Party", who, &format!("{m} max charges")], r + 0x21 + i, Kind::U8, 0, 99);
            }
        }
    }
    b.info(&["Info", "Slots in use"], found);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. Attack, defense and the other derived stats are tied to equipment in this game, so they're not shown. Each slot's sum is rewritten on save.")))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..3 {
        let s = n * SLOT;
        if d[s] != 0 {
            let v = sum(d, s);
            d[s + 4..s + 6].copy_from_slice(&v.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_level_and_money() {
        let mut d = vec![0u8; 0x2000];
        d[SLOT] = 1;
        d[SLOT + 0x16] = 1;
        d[SLOT + 0x1a] = 16;
        d[SLOT + 0x1c] = 40;
        d[SLOT + 0x66] = 0xff;
        fix(&mut d);
        assert!(valid(&d, 1));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 2/Game/Money", 5000.into()).unwrap();
        doc.set("/Slot 2/Party/Hero/Level", 30.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[SLOT + 0xa6..SLOT + 0xa9], &[0x88, 0x13, 0]);
        assert_eq!(out[SLOT + 0x16], 30);
        assert!(valid(&out, 1));
    }
}
