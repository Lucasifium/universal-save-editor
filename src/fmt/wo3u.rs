//! Warriors Orochi 3 Ultimate Definitive Edition (`SAVEDATA.BIN`, exactly 0x2119CA bytes): a
//! plain little-endian save with no cipher and no checksum. The first four bytes are the
//! revision (`F1 18 03 14`); the officer and weapon records each start with the same vtable
//! fragment (its low byte differs between game builds), which is how the layout is recognised. Field offsets follow the Universal Koei
//! Tecmo Save Editor by Sanrenan (MIT), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Warriors Orochi 3 Ultimate save";
const SIZE: usize = 0x2119CA;
const REV: [u8; 4] = [0xF1, 0x18, 0x03, 0x14];
const OFFICERS: usize = 0xECF2;
const STRIDE: usize = 0x2B0;
const WEAPONS: usize = 0xC8010;
const WSTRIDE: usize = 0x1C;
/// (start, count) of the crafting-material byte spans.
pub const MATERIALS: [(usize, usize); 7] = [(0xE9A8, 16), (0xE9C8, 16), (0xE9E8, 16), (0xEA08, 34), (0xEA3A, 34), (0xEA6C, 34), (0xEAA8, 145)];
pub const ORBS: usize = 0xE944;
pub const ORB_COUNT: usize = 58;

pub fn looks(d: &[u8]) -> bool {
    if d.len() != SIZE || d[..4] != REV {
        return false;
    }
    let om = &d[OFFICERS - 10..OFFICERS - 6];
    let wm = &d[WEAPONS - 4..WEAPONS];
    om[1] == 0x6D && wm[1] == 0x62 && om[2..] == wm[2..] && (0..150).all(|i| d[OFFICERS - 10 + i * STRIDE..OFFICERS - 6 + i * STRIDE] == *om) && (0..145 * 16).step_by(97).all(|i| d[WEAPONS - 4 + i * WSTRIDE..WEAPONS + i * WSTRIDE] == *wm)
}

fn w16(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Warriors Orochi 3 Ultimate save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Resources", "Growth Points"], 0x1378, Kind::U32, 0, 9_999_999);
    b.ranged(&["Resources", "Gems"], 0x137C, Kind::U32, 0, 9_999_999);
    for i in 0..145usize {
        let at = OFFICERS + i * STRIDE;
        if w16(d, at) == 0 {
            continue;
        }
        let n = format!("Officer {:03}", i + 1);
        for (k, (stat, hi)) in [("Health", 999), ("Musou", 999), ("Attack", 999), ("Defense", 999), ("Speed", 200)].into_iter().enumerate() {
            b.ranged(&["Officers", &n, stat], at + 2 * k, Kind::U16, 0, hi);
        }
    }
    for i in 0..ORB_COUNT {
        if (1..=99).contains(&d[ORBS + i]) {
            b.ranged(&["Weapon attribute orbs", &format!("Orb {:02}", i + 1)], ORBS + i, Kind::U8, 0, 99);
        }
    }
    let mut n = 0;
    for (start, count) in MATERIALS {
        for i in 0..count {
            n += 1;
            if (1..=99).contains(&d[start + i]) {
                b.ranged(&["Crafting materials", &format!("Material {n:03}")], start + i, Kind::U8, 0, 99);
            }
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("The save is plain data with no checksum. Close the game first. Officers, orbs and materials are shown by their number in the save, not by name; only orbs and materials you own are listed.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        d[..4].copy_from_slice(&REV);
        for i in 0..150 {
            d[OFFICERS - 10 + i * STRIDE..OFFICERS - 6 + i * STRIDE].copy_from_slice(&[0xA8, 0x6D, 0x11, 0x22]);
        }
        for i in 0..145 * 16 {
            d[WEAPONS - 4 + i * WSTRIDE..WEAPONS + i * WSTRIDE].copy_from_slice(&[0x28, 0x62, 0x11, 0x22]);
        }
        d
    }

    #[test]
    fn edits_in_place() {
        let mut d = blank();
        d[0x1378..0x137C].copy_from_slice(&5u32.to_le_bytes());
        d[OFFICERS..OFFICERS + 2].copy_from_slice(&300u16.to_le_bytes());
        d[ORBS + 3] = 7;
        d[MATERIALS[3].0 + 2] = 9;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Resources/Growth Points", 9_999_999.into()).unwrap();
        doc.set("/Officers/Officer 001/Attack", 999.into()).unwrap();
        doc.set("/Weapon attribute orbs/Orb 04", 99.into()).unwrap();
        doc.set("/Crafting materials/Material 051", 99.into()).unwrap();
        assert!(doc.set("/Weapon attribute orbs/Orb 04", 100.into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[ORBS + 3], 99);
        assert_eq!(out[MATERIALS[3].0 + 2], 99);
        assert!(looks(&out));
    }
}
