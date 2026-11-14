//! The Legend of Zelda: A Link to the Past (SNES) battery saves (`.srm` / `.sav`, 8 KiB):
//! three slots of 0x500 bytes at 0, 0x500 and 0xa00 (the game keeps a spare copy of each at
//! +0xf00 and only falls back to it when the first fails its check). Each slot ends in a
//! 16-bit word that makes the little-endian word sum of the slot 0x5a5a.
//!
//! Offsets follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md) and are checked on a public save's sum.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "A Link to the Past save";
const SLOT: usize = 0x500;

fn word_sum(d: &[u8], s: usize) -> u16 {
    d[s..s + 0x4fe].chunks(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])))
}

fn valid(d: &[u8], n: usize) -> bool {
    let s = n * SLOT;
    word_sum(d, s).wrapping_add(u16::from_le_bytes([d[s + 0x4fe], d[s + 0x4ff]])) == 0x5a5a
}

pub fn looks(d: &[u8]) -> bool {
    // Nothing in the file names the game: three slots whose sums fit, and at least one is in use.
    d.len() == 0x2000
        && (0..3).any(|n| u16::from_le_bytes([d[n * SLOT + 0x36c], d[n * SLOT + 0x36d]]) != 0)
        && (0..3).filter(|&n| valid(d, n)).count() >= 1
        && (0..3).all(|n| d[n * SLOT + 0x36c] <= 160)
}

const YN: &[&str] = &["No", "Yes"];
const BOTTLES: &[&str] = &["Empty slot", "Mushroom (unused)", "Bottle", "Life medicine (red)", "Magic medicine (green)", "Cure-all (blue)", "Faerie", "Bee", "Golden bee"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not an A Link to the Past save".into());
    }
    let mut b = Builder::new(d);
    let mut found = 0;
    for n in 0..3 {
        let s = n * SLOT;
        if !valid(d, n) || d[s + 0x36c] == 0 {
            continue;
        }
        found += 1;
        let slot = ["Slot 1", "Slot 2", "Slot 3"][n];
        let num = |b: &mut Builder, name: &str, off: usize, k: Kind, hi: i64| {
            b.ranged(&[slot, name], s + off, k, 0, hi);
        };
        num(&mut b, "Max health (8 per heart)", 0x36c, Kind::U8, 160);
        num(&mut b, "Health (8 per heart)", 0x36d, Kind::U8, 160);
        num(&mut b, "Heart pieces", 0x36b, Kind::U8, 3);
        num(&mut b, "Magic", 0x36e, Kind::U8, 128);
        b.ranged(&[slot, "Rupees"], s + 0x360, Kind::U16, 0, 999);
        b.mirror(&[slot, "Rupees"], s + 0x362);
        num(&mut b, "Bombs", 0x343, Kind::U8, 50);
        num(&mut b, "Bomb capacity upgrades (0-7)", 0x370, Kind::U8, 7);
        num(&mut b, "Arrows", 0x377, Kind::U8, 70);
        num(&mut b, "Arrow capacity upgrades (0-7)", 0x371, Kind::U8, 7);
        num(&mut b, "Rupees in the fairy pond", 0x36a, Kind::U8, 99);
        let list = |b: &mut Builder, name: &str, off: usize, names: &[&str]| {
            b.named(&[slot, "Items", name], s + off, Kind::U8, names);
        };
        list(&mut b, "Sword", 0x359, &["None", "Fighter's sword", "Master sword", "Tempered sword", "Golden sword"]);
        list(&mut b, "Shield", 0x35a, &["None", "Fighter's shield", "Red shield", "Mirror shield"]);
        list(&mut b, "Tunic", 0x35b, &["Green", "Blue", "Red"]);
        list(&mut b, "Bow", 0x340, &["None", "Bow", "Bow and arrows", "?", "Bow and silver arrows"]);
        list(&mut b, "Boomerang", 0x341, &["None", "Boomerang", "Magical boomerang"]);
        list(&mut b, "Mushroom / magic powder", 0x344, &["None", "Mushroom", "Magic powder"]);
        list(&mut b, "Shovel / flute", 0x34c, &["None", "Shovel", "Flute", "Flute with bird"]);
        list(&mut b, "Gloves", 0x354, &["None", "Power glove", "Titan's mitt"]);
        list(&mut b, "Magic mirror", 0x353, &["None", "Magic scroll", "Magic mirror"]);
        for (name, off) in [
            ("Hookshot", 0x342), ("Fire rod", 0x345), ("Ice rod", 0x346), ("Bombos medallion", 0x347), ("Ether medallion", 0x348), ("Quake medallion", 0x349),
            ("Lamp", 0x34a), ("Magic hammer", 0x34b), ("Bug-catching net", 0x34d), ("Book of Mudora", 0x34e), ("Cane of Somaria", 0x350),
            ("Cane of Byrna", 0x351), ("Magic cape", 0x352), ("Pegasus boots", 0x355), ("Zora's flippers", 0x356), ("Moon pearl", 0x357),
        ] {
            list(&mut b, name, off, YN);
        }
        for (i, off) in [0x35c, 0x35d, 0x35e, 0x35f].into_iter().enumerate() {
            list(&mut b, &format!("Bottle {}", i + 1), off, BOTTLES);
        }
        list(&mut b, "Half magic use", 0x37b, YN);
    }
    b.info(&["Info", "Slots in use"], found);
    if found == 0 {
        return Err("no slot in this save has been started".into());
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Health counts eighths of a heart (8 = one heart); rupees follow the on-screen counter. The slot's sum is kept right; the game's spare copy is left alone and is only used when the first fails its check."))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..3 {
        let s = n * SLOT;
        if s + SLOT <= d.len() && valid_before(d, n) {
            let sum = 0x5a5au16.wrapping_sub(word_sum(d, s));
            d[s + 0x4fe..s + 0x500].copy_from_slice(&sum.to_le_bytes());
        }
    }
}

/// Only slots in use have a sum to keep (an empty slot's words are all zero).
fn valid_before(d: &[u8], n: usize) -> bool {
    d[n * SLOT + 0x36c] != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_fixes_sum() {
        let mut d = vec![0u8; 0x2000];
        d[0x36c] = 24;
        d[0x36d] = 24;
        fix(&mut d);
        assert!(valid(&d, 0));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Rupees", 500.into()).unwrap();
        doc.set("/Slot 1/Items/Sword", "Golden sword".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u16::from_le_bytes([out[0x360], out[0x361]]), 500);
        assert_eq!(u16::from_le_bytes([out[0x362], out[0x363]]), 500);
        assert_eq!(out[0x359], 4);
        assert!(valid(&out, 0));
    }
}
