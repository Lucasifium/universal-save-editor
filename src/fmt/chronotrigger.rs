//! Chrono Trigger (SNES) battery saves (`.srm` / `.sav`, 8 KiB): three slots of 0xa00 bytes.
//! Gold, play time and the formation are at 0x5e0-0x5e7, seven 0x50-byte character records start
//! at 0x200. Each slot's 16-bit sum (words added from the end, folded at 0xffff) is kept at
//! 0x1ff0 + 2 * slot; 1b e4 marks the file's tail at 0x1ff8.
//!
//! Layout and item names follow the MIT-licensed save editor in
//! RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md); the sums are checked on a
//! public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::OnceLock;

pub const KIND: &str = "Chrono Trigger (SNES) save";
const SLOT: usize = 0xa00;
const CHARS: [&str; 7] = ["Crono", "Marle", "Lucca", "Robo", "Frog", "Ayla", "Magus"];

fn items() -> &'static Vec<(u64, u64, String)> {
    static N: OnceLock<Vec<(u64, u64, String)>> = OnceLock::new();
    N.get_or_init(|| {
        include_str!("data/ct_snes.txt")
            .lines()
            .filter_map(|l| {
                let mut p = l.splitn(3, ':');
                Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.to_string()))
            })
            .collect()
    })
}

fn sum(d: &[u8], s: usize) -> u16 {
    let mut c: u32 = 0;
    for i in (s..s + SLOT).step_by(2).rev() {
        c %= 0xffff;
        c += u16::from_le_bytes([d[i], d[i + 1]]) as u32;
    }
    c as u16
}

fn stored(d: &[u8], n: usize) -> u16 {
    u16::from_le_bytes([d[0x1ff0 + 2 * n], d[0x1ff1 + 2 * n]])
}

fn valid(d: &[u8], n: usize) -> bool {
    stored(d, n) != 0 && sum(d, n * SLOT) == stored(d, n)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == 0x2000 && [0x1ff8, 0x1ffa, 0x1ffc].iter().any(|&o| d[o] == 0x1b && d[o + 1] == 0xe4) && (0..3).any(|n| valid(d, n))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Chrono Trigger (SNES) save".into());
    }
    let it = items();
    let pick_of = |kinds: &[u64]| -> Vec<(String, u64)> {
        std::iter::once(("(none)".to_string(), 0u64)).chain(it.iter().filter(|(_, k, _)| kinds.contains(k)).map(|(i, _, n)| (n.clone(), *i))).collect()
    };
    let weapons = pick_of(&[0, 1, 2, 3, 4, 5, 6]);
    let armors = pick_of(&[7]);
    let helmets = pick_of(&[8]);
    let accessories = pick_of(&[9]);
    let mut b = Builder::new(d);
    let mut found = 0;
    for n in 0..3 {
        if !valid(d, n) {
            continue;
        }
        found += 1;
        let s = n * SLOT;
        let slot = ["Slot 1", "Slot 2", "Slot 3"][n];
        b.bits(&[slot, "Game", "Gold"], s + 0x5e0, Kind::U32, 0, 24, 0, 0, 9_999_999);
        b.ranged(&[slot, "Game", "Silver points"], s + 0x655, Kind::U8, 0, 200);
        for c in 0..7 {
            let r = s + 0x200 + c * 0x50;
            let who = d[r] as usize;
            let mut key = CHARS.get(who).map_or(format!("Character {}", c + 1), |s| s.to_string());
            if (0..c).any(|e| d[s + 0x200 + e * 0x50] as usize == who) {
                key = format!("{key} #{}", c + 1);
            }
            let k = key.as_str();
            b.bits(&[slot, "Party", k, "Experience"], r + 0x13, Kind::U32, 0, 24, 0, 0, 9_999_999);
            b.bits(&[slot, "Party", k, "Tech points"], r + 0x16, Kind::U32, 0, 24, 0, 0, 9_999_999);
            b.ranged(&[slot, "Party", k, "HP"], r + 0x3, Kind::U16, 0, 999);
            b.ranged(&[slot, "Party", k, "Max HP"], r + 0x3f, Kind::U16, 1, 999);
            b.ranged(&[slot, "Party", k, "MP"], r + 0x7, Kind::U16, 0, 99);
            b.ranged(&[slot, "Party", k, "Max MP"], r + 0x9, Kind::U16, 0, 99);
            for (col, off) in [("Power", 0xb), ("Stamina", 0xc), ("Speed", 0xd), ("Magic", 0xe), ("Hit", 0xf), ("Evade", 0x10), ("Magic defense", 0x11)] {
                b.ranged(&[slot, "Party", k, col], r + off, Kind::U8, 0, 99);
            }
            b.pick(&[slot, "Equipment", k, "Weapon"], r + 0x29, Kind::U8, &weapons);
            b.pick(&[slot, "Equipment", k, "Helmet"], r + 0x27, Kind::U8, &helmets);
            b.pick(&[slot, "Equipment", k, "Armor"], r + 0x28, Kind::U8, &armors);
            b.pick(&[slot, "Equipment", k, "Accessory"], r + 0x2a, Kind::U8, &accessories);
        }
    }
    b.info(&["Info", "Slots in use"], found);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Level follows experience when the game next levels the character; each slot's sum is rewritten on save."))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..3 {
        if stored(d, n) != 0 {
            let v = sum(d, n * SLOT);
            d[0x1ff0 + 2 * n..0x1ff2 + 2 * n].copy_from_slice(&v.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_gold_and_character() {
        let mut d = vec![0u8; 0x2000];
        d[0x1ffa] = 0x1b;
        d[0x1ffb] = 0xe4;
        d[0x5e0] = 100;
        for c in 0..7 {
            d[0x200 + c * 0x50] = c as u8;
        }
        d[0x1ff0] = 1;
        fix(&mut d);
        assert!(valid(&d, 0));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Game/Gold", 99_999.into()).unwrap();
        doc.set("/Slot 1/Party/Crono/Tech points", 1234.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[0x5e0..0x5e3], &[0x9f, 0x86, 0x01]);
        assert_eq!(&out[0x216..0x219], &[0xd2, 0x04, 0x00]);
        assert!(valid(&out, 0));
    }
}
