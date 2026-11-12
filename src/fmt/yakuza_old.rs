//! Yakuza 3, 4 and 5 Remastered (PC) `savegameN.sav` / `SAVEGAMEN.sav`: a fixed-size memory
//! image of the game's save data, the same layout as the PlayStation 3 originals. No magic
//! bytes and no checksum (the third word of the header is the play time, and the `.clr`
//! "clear data" copies differ from the `.sav` only in values, never in a hash).
//!
//! The values were placed with the game's own memory map: the in-memory addresses of the
//! player's money, experience and the rest keep the same distance from each other in the
//! save, so one known value pins every other. Checked on a public save of each game whose
//! values were all set to their maximum, which also confirmed the other offsets.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND_Y3: &str = "Yakuza 3 Remastered save";
pub const KIND_Y4: &str = "Yakuza 4 Remastered save";
pub const KIND_Y5: &str = "Yakuza 5 Remastered save";

const Y3_LEN: usize = 103_104;
const Y4_LEN: usize = 121_456;
const Y5_LEN: usize = 179_392;

fn u32at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

#[derive(PartialEq)]
enum Which {
    Y3,
    Y4,
    Y5,
}

fn which(d: &[u8]) -> Option<Which> {
    if d.len() < 64 || u32at(d, 4) != 1 || u32at(d, 12) != 0 {
        return None;
    }
    match (d.len(), u32at(d, 0)) {
        (Y3_LEN, 1) => Some(Which::Y3),
        (Y4_LEN, 3) => Some(Which::Y4),
        (Y5_LEN, 1) => Some(Which::Y5),
        _ => None,
    }
}

pub fn looks(d: &[u8]) -> bool {
    which(d).is_some()
}

// Yakuza 3: Kiryu alone. Experience: the u32 at EXP is the total, the one after it the
// experience in hand.
const Y3_MONEY: usize = 0x7ca8;
const Y3_CLUB: usize = 0x7cbc;
const Y3_EXP: usize = 0x58c8;

// Yakuza 4: (name, money, experience). To the experience's right: experience to the next
// level, and the skill orbs (a byte) 0x10 after it.
const Y4_CHARS: [(&str, usize, usize); 4] = [
    ("Akiyama", 0x82d4, 0x588c),
    ("Saejima", 0x82d8, 0x593c),
    ("Tanimura", 0x82dc, 0x59ec),
    ("Kiryu", 0x82d0, 0x57dc),
];
const Y4_ECO: usize = 0x8338;

// Yakuza 5: (name, money, experience); the experience's next-level word follows it, and
// the orbs (a byte) sit 8 after it.
const Y5_CHARS: [(&str, usize, usize); 5] = [
    ("Kiryu", 0xc638, 0x6a94),
    ("Saejima", 0xc640, 0x6d94),
    ("Akiyama", 0xc63c, 0x6c14),
    ("Shinada", 0xc644, 0x6f14),
    ("Haruka", 0xc648, 0xc4e4),
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let Some(w) = which(d) else { return Err("not a Yakuza 3, 4 or 5 Remastered save".into()) };
    let mut b = Builder::new(d);
    match w {
        Which::Y3 => {
            b.ranged(&["Kiryu", "Money"], Y3_MONEY, Kind::U32, 0, 999_999_999);
            b.ranged(&["Kiryu", "Club money"], Y3_CLUB, Kind::U32, 0, 999_999_999);
            b.ranged(&["Kiryu", "Total experience"], Y3_EXP, Kind::U32, 0, 999_999);
            b.ranged(&["Kiryu", "Experience in hand"], Y3_EXP + 4, Kind::U32, 0, 999_999);
            Ok(b.finish(KIND_Y3, Checksum::None).with_note(
                "Kiryu's money, the hostess club's money and his experience. The save has no checksum; close the game first.",
            ))
        }
        Which::Y4 => {
            for (name, money, exp) in Y4_CHARS {
                b.ranged(&[name, "Money"], money, Kind::U32, 0, 999_999_999);
                b.ranged(&[name, "Experience"], exp, Kind::U32, 0, 999_999);
                b.ranged(&[name, "Experience to next level"], exp + 4, Kind::U32, 0, 999_999);
                b.ranged(&[name, "Skill orbs"], exp + 0x10, Kind::U8, 0, 99);
            }
            b.ranged(&["Shared", "Eco points"], Y4_ECO, Kind::U32, 0, 999_999);
            Ok(b.finish(KIND_Y4, Checksum::None).with_note(
                "Each of the four playable characters' money (it is kept apart), experience, skill orbs, and the eco points. The save has no checksum; close the game first.",
            ))
        }
        Which::Y5 => {
            for (name, money, exp) in Y5_CHARS {
                b.ranged(&[name, "Money"], money, Kind::U32, 0, 999_999_999);
                b.ranged(&[name, "Experience"], exp, Kind::U32, 0, 99_999);
                b.ranged(&[name, "Experience to next level"], exp + 4, Kind::U32, 0, 99_999);
                b.ranged(&[name, "Skill orbs"], exp + 8, Kind::U8, 0, 99);
            }
            Ok(b.finish(KIND_Y5, Checksum::None).with_note(
                "Money, experience and skill orbs for each of the five playable characters. The save has no checksum; close the game first.",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(len: usize, first: u32) -> Vec<u8> {
        let mut d = vec![0u8; len];
        d[0..4].copy_from_slice(&first.to_le_bytes());
        d[4..8].copy_from_slice(&1u32.to_le_bytes());
        d
    }

    fn put(d: &mut [u8], at: usize, v: u32) {
        d[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn yakuza3() {
        let mut d = blank(Y3_LEN, 1);
        put(&mut d, Y3_MONEY, 1234);
        put(&mut d, Y3_EXP, 800);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Kiryu/Money").unwrap(), 1234);
        doc.set("/Kiryu/Money", serde_json::Value::from(999_999_999u64)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32at(&out, Y3_MONEY), 999_999_999);
        assert_eq!(u32at(&out, Y3_EXP), 800);
        assert_eq!(out.len(), d.len());
    }

    #[test]
    fn yakuza4_has_four_wallets() {
        let mut d = blank(Y4_LEN, 3);
        for (i, (_, money, exp)) in Y4_CHARS.iter().enumerate() {
            put(&mut d, *money, 100 + i as u32);
            put(&mut d, *exp, 5000);
            d[exp + 0x10] = 7;
        }
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Kiryu/Money").unwrap(), 103);
        assert_eq!(doc.view.pointer("/Akiyama/Skill orbs").unwrap(), 7);
        doc.set("/Saejima/Skill orbs", serde_json::Value::from(99)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[Y4_CHARS[1].2 + 0x10], 99);
    }

    #[test]
    fn yakuza5_five_characters() {
        let mut d = blank(Y5_LEN, 1);
        for (_, money, _) in Y5_CHARS {
            put(&mut d, money, 77);
        }
        let doc = parse(&d).unwrap();
        for (name, _, _) in Y5_CHARS {
            assert_eq!(doc.view.pointer(&format!("/{name}/Money")).unwrap(), 77);
        }
    }

    #[test]
    fn rejects_other_files() {
        assert!(!looks(&blank(Y3_LEN, 3)));
        assert!(!looks(&blank(1000, 1)));
    }
}
