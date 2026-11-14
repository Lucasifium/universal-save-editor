//! Devil May Cry (PlayStation 2) saves: each "SaveData-NN" file of the save folder is one slot of
//! 2,416 bytes, starting with the bytes 01 00 00 and guarded by a byte sum at offset 8. Offsets
//! follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::ps1::opts;
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Devil May Cry (PlayStation 2) save";

fn sum(d: &[u8]) -> u32 {
    d[0x10..0x970].iter().fold(0u32, |s, b| s.wrapping_add(*b as u32))
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == 0x970 && d[..3] == [1, 0, 0] && u32::from_le_bytes([d[8], d[9], d[10], d[11]]) == sum(d) && d[0x24] <= 23
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Devil May Cry slot".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["General", "Clear count"], 0x22, Kind::U16, 0, 99);
    b.ranged(&["General", "Save count"], 0x20, Kind::U16, 0, 65535);
    b.pick(&["General", "Character"], 0x27, Kind::U8, &opts(&[("Dante", 0), ("Legendary Dark Knight", 1), ("Super Dante", 2)]));
    b.pick(&["General", "Difficulty"], 0x26, Kind::U8, &opts(&[("Easy Automatic", 2), ("Normal", 3), ("Hard", 5), ("Dante Must Die!", 6)]));
    let missions: Vec<(String, u64)> = (0..24).map(|i| (if i == 0 { "Prologue".to_string() } else { format!("Mission {i}") }, i as u64)).collect();
    b.pick(&["General", "Mission"], 0x24, Kind::U8, &missions);
    b.ranged(&["General", "Maximum health (units of 100)"], 0x624, Kind::U8, 10, 30);
    b.ranged(&["Orbs", "Red orbs"], 0x634, Kind::U32, 0, 999_999_999);
    b.ranged(&["Orbs", "Yellow orbs"], 0x620, Kind::U16, 0, 999);
    b.ranged(&["Orbs", "Blue orb fragments"], 0x638, Kind::U32, 0, 3);
    b.ranged(&["Orbs", "Purchased blue orbs"], 0x626, Kind::U8, 0, 7);
    b.ranged(&["Orbs", "Purchased purple orbs"], 0x627, Kind::U8, 0, 7);
    for (n, at, bit) in ALASTOR {
        flag(&mut b, &["Alastor skills", n], *at, *bit);
    }
    for (n, at, bit) in IFRIT {
        flag(&mut b, &["Ifrit skills", n], *at, *bit);
    }
    let ranks = opts(&[("S", 0), ("A", 1), ("B", 2), ("C", 3), ("D", 4), ("-", 0xff)]);
    for (lvl, at) in [("Normal", 0x7bc), ("Hard", 0x7d3), ("Dante Must Die!", 0x7ea)] {
        for i in 0..23 {
            b.pick(&["Ranks", lvl, &format!("Mission {}", i + 1)], at + i, Kind::U8, &ranks);
        }
    }
    for (i, (at, bit)) in SECRET.iter().enumerate() {
        flag(&mut b, &["Secret missions", &format!("Secret mission {}", i + 1)], *at, *bit);
    }
    b.ranged(&["Statistics", "Total retries"], 0x622, Kind::U16, 0, 65535);
    b.ranged(&["Statistics", "Damage received"], 0x808, Kind::U32, 0, 999_999_999);
    b.ranged(&["Statistics", "Red orbs obtained"], 0x83c, Kind::U32, 0, 999_999_999);
    for (n, at) in [("Dull", 0x4d8), ("Cool", 0x4dc), ("Bravo", 0x4e0), ("Absolute", 0x4e4), ("Stylish", 0x4e8)] {
        b.ranged(&["Combo rankings", n], at, Kind::U32, 0, 99_999);
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. Each file of the save folder is one slot (Tools › Open another file in this archive lists them). The slot's checksum is written again on save.")))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() == 0x970 {
        let s = sum(d);
        d[8..12].copy_from_slice(&s.to_le_bytes());
    }
}

const ALASTOR: &[(&str, usize, u32)] = &[("Stringer level 1", 0x62b, 4), ("Stringer level 2", 0x62b, 3), ("Round Trip", 0x62b, 5), ("Air Hike", 0x62b, 6), ("Air Raid", 0x62b, 0), ("Vortex level 1", 0x62b, 2), ("Vortex level 2", 0x62b, 1)];
const IFRIT: &[(&str, usize, u32)] = &[("Magma Drive", 0x62a, 6), ("Kick 13 level 1", 0x62a, 5), ("Kick 13 level 2", 0x62a, 4), ("Rolling Blaze", 0x62a, 7), ("Meteor level 1", 0x62a, 3), ("Meteor level 2", 0x62a, 2), ("Inferno", 0x62a, 1)];
const SECRET: &[(usize, u32)] = &[(0x33, 6), (0x33, 5), (0x33, 4), (0x33, 3), (0x33, 2), (0x32, 4), (0x32, 3), (0x32, 5), (0x32, 6), (0x33, 1), (0x33, 0), (0x32, 7)];

#[cfg(test)]
mod tests {
    use super::*;

    fn slot() -> Vec<u8> {
        let mut d = vec![0u8; 0x970];
        d[0] = 1;
        d[0x27] = 0;
        d[0x26] = 3;
        d[0x620] = 3;
        fix(&mut d);
        d
    }

    #[test]
    fn edits_orbs_and_fixes_the_sum() {
        let d = slot();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Orbs/Red orbs", 123456.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes(out[0x634..0x638].try_into().unwrap()), 123456);
        assert!(looks(&out));
    }
}
