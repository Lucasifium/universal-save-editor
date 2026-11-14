//! Mario & Luigi: Dream Team (Nintendo 3DS) decrypted save (`ML4_001.sav`, 96,688 bytes): plain
//! values at fixed offsets, no checksum is known to be kept over them.
//!
//! Offsets after Cefo14/MLDT_SE (MIT, see THIRD_PARTY_NOTICES.md): coins 0x91c, each brother's
//! level at +0x2b / experience at +0x28 (three bytes) / HP, BP, Power, Defense, Speed and Stache
//! from the brother's block (Mario 0x74e, Luigi 0x836), and one byte of count per item or bean.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Mario & Luigi: Dream Team save";
const SIZE: usize = 96_688;
const MARIO: usize = 0x74e;
const LUIGI: usize = 0x836;

const ITEMS: [(&str, usize); 19] = [
    ("Mushroom", 0x920), ("Super Mushroom", 0x921), ("Ultra Mushroom", 0x922), ("Max Mushroom", 0x923),
    ("Nut", 0x924), ("Super Nut", 0x925), ("Ultra Nut", 0x926), ("Max Nut", 0x927),
    ("Syrup Jar", 0x928), ("Super Syrup Jar", 0x929), ("Ultra Syrup Jar", 0x92a), ("Max Syrup Jar", 0x92b),
    ("Candy", 0x92c), ("Super Candy", 0x92d), ("Ultra Candy", 0x92e), ("Max Candy", 0x92f),
    ("1-Up Mushroom", 0x930), ("1-Up Deluxe", 0x931), ("Refreshing Herb", 0x932),
];
const BATTLE: [(&str, usize); 4] = [("Taunt Ball", 0x939), ("Shock Bomb", 0x93a), ("Boo Biscuit", 0x93b), ("Secret Box", 0x93c)];
const BEANS: [(&str, usize); 12] = [
    ("HP Bean", 0x933), ("Bros. Bean", 0x934), ("Power Bean", 0x935), ("Defense Bean", 0x936), ("Speed Bean", 0x937), ("Stache Bean", 0x938),
    ("Super HP Bean", 0x93d), ("Super Bros. Bean", 0x93e), ("Super Power Bean", 0x93f), ("Super Defense Bean", 0x940),
    ("Super Speed Bean", 0x941), ("Super Stache Bean", 0x942),
];

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && (1..=99).contains(&d[MARIO + 0x2d]) && (1..=99).contains(&d[LUIGI + 0x2d])
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Mario & Luigi: Dream Team save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Coins"], 0x91c, Kind::U32, 0, 999_999);
    for (name, at) in [("Mario", MARIO), ("Luigi", LUIGI)] {
        b.ranged(&[name, "Level"], at + 0x2d, Kind::U8, 1, 99);
        b.bits(&[name, "Experience"], at + 0x2a, Kind::U32, 0, 24, 0, 0, 16_777_215);
        b.ranged(&[name, "HP"], at, Kind::U16, 0, 999);
        b.ranged(&[name, "Max HP"], at + 4, Kind::U16, 1, 999);
        b.ranged(&[name, "BP"], at + 2, Kind::U16, 0, 999);
        b.ranged(&[name, "Max BP"], at + 6, Kind::U16, 1, 999);
        b.ranged(&[name, "Power"], at + 8, Kind::U16, 0, 999);
        b.ranged(&[name, "Defense"], at + 10, Kind::U16, 0, 999);
        b.ranged(&[name, "Speed"], at + 12, Kind::U16, 0, 999);
        b.ranged(&[name, "Stache"], at + 14, Kind::U16, 0, 999);
    }
    for (title, list) in [("Recovery items", &ITEMS[..]), ("Battle items", &BATTLE[..]), ("Beans", &BEANS[..])] {
        for (name, at) in list {
            b.ranged(&[title, name], *at, Kind::U8, 0, 99);
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. A 3DS save must be the decrypted one (from Checkpoint, JKSM or GodMode9); counts are limited to 99.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        for at in [MARIO, LUIGI] {
            d[at + 0x2d] = 20;
        }
        d[0x91c..0x920].copy_from_slice(&6830u32.to_le_bytes());
        d[0x920] = 3;
        d
    }

    #[test]
    fn edits_in_place() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Coins", 99999.into()).unwrap();
        doc.set("/Mario/Level", 30.into()).unwrap();
        doc.set("/Mario/Experience", 231_160.into()).unwrap();
        doc.set("/Recovery items/Mushroom", 99.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes([out[0x91c], out[0x91d], out[0x91e], out[0x91f]]), 99999);
        assert_eq!(out[MARIO + 0x2d], 30);
        assert_eq!(out[0x920], 99);
        assert_eq!(out.len(), SIZE);
        assert_eq!(out[LUIGI + 0x2d], 20);
        assert!(doc.set("/Recovery items/Mushroom", 100.into()).is_err());
    }
}
