//! The Legend of Zelda: The Wind Waker (GameCube) `.gci` saves: three slots of 0x770 bytes from
//! 0x2040, each ending in a 64-bit sum (the sum of the bytes and the sum of their complements).
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, ww};
use crate::tree::TreeDoc;

pub const KIND: &str = "The Wind Waker save";
const SLOT: usize = 0x770;
const SIZE: usize = 98_368;

fn sum(d: &[u8], k: usize) -> [u8; 8] {
    let seg = &d[0x2048 + k * SLOT..0x27b0 + k * SLOT];
    let high = seg.iter().fold(0u32, |a, &b| a.wrapping_add(u32::from(b)));
    let low = seg.iter().fold(0u32, |a, &b| a.wrapping_add(!u32::from(b)));
    let mut o = [0u8; 8];
    o[..4].copy_from_slice(&high.to_be_bytes());
    o[4..].copy_from_slice(&low.to_be_bytes());
    o
}

fn good(d: &[u8], k: usize) -> bool {
    d[0x27b0 + k * SLOT..0x27b8 + k * SLOT] == sum(d, k)
}

fn used(d: &[u8], k: usize) -> bool {
    d[0x219f + k * SLOT] != 0
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && matches!(&d[..4], b"GZLE" | b"GZLP" | b"GZLJ") && (0..3).any(|k| used(d, k) && good(d, k))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Wind Waker save".into());
    }
    let mut b = Builder::new(d);
    for k in (0..3).filter(|&k| used(d, k) && good(d, k)) {
        add(&mut b, &[&format!("Slot {}", k + 1)], k * SLOT, ww::SLOT);
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note("Close the emulator first. Health is in quarter hearts, so 12 is three hearts. Changing the sword, shield or bracelets also sets the matching equipped flag. Each edited slot's sum is made again."))
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    for k in 0..3 {
        let (lo, hi) = (0x2048 + k * SLOT, 0x27b0 + k * SLOT);
        if !touched.iter().any(|&t| t >= lo && t < hi) {
            continue;
        }
        let at = |o: usize| o + k * SLOT;
        if touched.iter().any(|&t| t == at(0x2056)) {
            d[at(0x20fa)] = match d[at(0x2056)] {
                0x38 => 1,
                0x39 => 3,
                0x3a => 7,
                0x3e => 0xf,
                _ => 0,
            };
        }
        if touched.iter().any(|&t| t == at(0x2057)) {
            d[at(0x20fb)] = match d[at(0x2057)] {
                0x3b => 1,
                0x3c => 3,
                _ => 0,
            };
        }
        if touched.iter().any(|&t| t == at(0x2058)) {
            d[at(0x20fc)] = u8::from(d[at(0x2058)] == 0x28);
        }
        let v = sum(d, k);
        d[hi..hi + 8].copy_from_slice(&v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_a_slot_and_its_sum() {
        let mut d = vec![0u8; SIZE];
        d[..6].copy_from_slice(b"GZLE01");
        d[0x219f + SLOT] = 1;
        d[0x2056 + SLOT] = 0xff;
        let v = sum(&d, 1);
        d[0x27b0 + SLOT..0x27b8 + SLOT].copy_from_slice(&v);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 2/Inventory/Equipment/Sword", "Master Sword".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0x2056 + SLOT], 0x39);
        assert_eq!(out[0x20fa + SLOT], 3);
        assert!(good(&out, 1));
    }
}
