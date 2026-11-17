//! Attack on Titan (`atwin0000.dat`, 769,568 bytes) and Attack on Titan 2: Final Battle
//! (`Attack on Titan2_AOT2_PK_WIN0000.dat`, 3,687,964 bytes) on Windows. Both start with a
//! little-endian 16-bit word sum and 16-bit seed, then a DWORD XOR stream (state = state *
//! 0x5B1A7851 + 0xCE4E, advanced three times per word in the first game and once in the second)
//! over the rest of the file. The sum is the 16-bit word sum of the decrypted body. The decrypted
//! body opens with the completion figures as text, and the same figures follow as numbers at body
//! offset 0x604, which is how the layouts are recognised. The first game's body is the PS Vita
//! layout behind a 0x61C-byte header (money and item quantity rows as published in the public
//! Vita patch lists, checked on a public save); the second game's gift table and its offset
//! match the public PS4 patch list shifted by 0x114740, checked on a public save. The second
//! game's regiment funds (0x114D74, with the total acquired beside it) and Wings of Freedom
//! (0x247A4C) were matched to values read in the game from a save of its own, and so was the
//! crafting material table (0x146E98, 999 each).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND1: &str = "Attack on Titan save";
pub const KIND2: &str = "Attack on Titan 2 save";
const SIZE1: usize = 0x000B_BE20;
const SIZE2: usize = 0x0038_461C;
const P: usize = 4;

const MONEY1: usize = P + 0x650;
const ITEMS1: usize = P + 0x362BC;
const ITEM_ROWS1: usize = 81;
const ITEM_STRIDE1: usize = 0x30;
const FUNDS2: usize = P + 0x114D74;
const WINGS2: usize = P + 0x247A4C;
const EXP2: usize = P + 0x22C6B8;
/// Experience at level 120 (identical in two level-120 saves).
const EXP2_CAP: u32 = 605_770;
const MATERIALS2: usize = P + 0x146E98;
const MATERIAL_ROWS2: usize = 53;
const GIFTS2: usize = P + 0x14B698;
const GIFT_ROWS2: usize = 50;
const GIFT_STRIDE2: usize = 0x30;

fn w16(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

fn w32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

fn stream(body: &mut [u8], seed: u32, advances: usize) {
    let mut st = seed;
    for c in body.chunks_exact_mut(4) {
        for _ in 0..advances {
            st = st.wrapping_mul(0x5B1A_7851).wrapping_add(0xCE4E);
        }
        let w = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) ^ st;
        c.copy_from_slice(&w.to_le_bytes());
    }
}

fn word_sum(body: &[u8]) -> u16 {
    body.chunks_exact(2).fold(0u16, |a, c| a.wrapping_add(u16::from_le_bytes([c[0], c[1]])))
}

fn advances(len: usize) -> Option<usize> {
    match len {
        SIZE1 => Some(3),
        SIZE2 => Some(1),
        _ => None,
    }
}

/// The completion figures at the top of the body: text (UTF-16 in the first game, ASCII in the
/// second) that must equal the numbers stored from body offset 0x604.
fn figures_agree(p: &[u8], first: bool) -> bool {
    let text: String = if first {
        p[P + 0x200..P + 0x240].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).filter_map(|u| char::from_u32(u32::from(u))).collect()
    } else {
        p[P..P + 0x40].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect()
    };
    if !text.chars().all(|c| c.is_ascii_digit() || c == ' ') {
        return false;
    }
    let mut nums: Vec<u32> = text.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    if first && nums.len() == 7 {
        nums.remove(0); // the leading 0000000 block
    }
    nums.len() >= 5 && nums.iter().take(5).enumerate().all(|(i, &n)| w32(p, P + 0x604 + 4 * i) == n)
}

fn unpack(d: &[u8]) -> Option<(Vec<u8>, bool)> {
    let adv = advances(d.len())?;
    let mut p = d.to_vec();
    stream(&mut p[P..], u32::from(w16(d, 2)), adv);
    let first = d.len() == SIZE1;
    (word_sum(&p[P..]) == w16(d, 0) && figures_agree(&p, first)).then_some((p, first))
}

fn pack(p: &mut Vec<u8>) {
    let Some(adv) = advances(p.len()) else { return };
    let ck = word_sum(&p[P..]);
    p[..2].copy_from_slice(&ck.to_le_bytes());
    let seed = u32::from(w16(p, 2));
    stream(&mut p[P..], seed, adv);
}

pub fn looks(d: &[u8]) -> bool {
    unpack(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (p, first) = unpack(d).ok_or("not an Attack on Titan save")?;
    let mut b = Builder::new(&p);
    if first {
        b.ranged(&["Resources", "Funds"], MONEY1, Kind::U32, 0, 99_999_999);
        for i in 0..ITEM_ROWS1 {
            let at = ITEMS1 + i * ITEM_STRIDE1;
            if w32(&p, at) != 0 && w32(&p, at) <= 999 {
                b.ranged(&["Items", &format!("Item {:02}", i + 1)], at, Kind::U16, 0, 999);
            }
        }
        Ok(b.finish(KIND1, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. The stream cipher and the word checksum are written again. Items are shown by their number in the save, not by name; only items you own are listed.")))
    } else {
        // Current funds sit beside the total acquired (equal in a new game); wings never exceed 999.
        if w32(&p, FUNDS2) <= w32(&p, FUNDS2 + 4) {
            b.ranged(&["Resources", "Regiment funds"], FUNDS2, Kind::U32, 0, 99_999_999);
        }
        if w32(&p, WINGS2) <= 999 {
            b.ranged(&["Resources", "Wings of Freedom"], WINGS2, Kind::U32, 0, 999);
        }
        if w32(&p, EXP2) <= EXP2_CAP {
            b.ranged(&["Player", "Experience"], EXP2, Kind::U32, 0, EXP2_CAP as i64);
        }
        for i in 0..MATERIAL_ROWS2 {
            let at = MATERIALS2 + i * GIFT_STRIDE2;
            if w16(&p, at) != 0 && w16(&p, at) <= 999 && matches!(w16(&p, at + 2), 0x21 | 0x29) {
                b.ranged(&["Crafting materials", &format!("Material {:02}", i + 1)], at, Kind::U16, 0, 999);
            }
        }
        for i in 0..GIFT_ROWS2 {
            let at = GIFTS2 + i * GIFT_STRIDE2;
            if w16(&p, at) != 0 && w16(&p, at) <= 99 {
                b.ranged(&["Gifts", &format!("Gift {:02}", i + 1)], at, Kind::U16, 0, 99);
            }
        }
        Ok(b.finish(KIND2, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. The stream cipher and the word checksum are written again. Gifts and materials are shown by their number in the save, not by name; only ones you own are listed.")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(first: bool) -> Vec<u8> {
        let size = if first { SIZE1 } else { SIZE2 };
        let mut p = vec![0u8; size];
        let nums = [4u32, 6, 29, 40, 3, 51];
        if first {
            let text = "0000000 004 06 29 040 003 051";
            for (i, u) in text.encode_utf16().enumerate() {
                p[P + 0x200 + 2 * i..P + 0x202 + 2 * i].copy_from_slice(&u.to_le_bytes());
            }
        } else {
            p[P..P + 17].copy_from_slice(b"004 06 29 040 003");
        }
        for (i, n) in nums.iter().take(if first { 6 } else { 5 }).enumerate() {
            p[P + 0x604 + 4 * i..P + 0x608 + 4 * i].copy_from_slice(&n.to_le_bytes());
        }
        p[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        p
    }

    #[test]
    fn first_game_round_trip() {
        let mut p = blank(true);
        p[MONEY1..MONEY1 + 4].copy_from_slice(&1234u32.to_le_bytes());
        p[ITEMS1 + 7 * ITEM_STRIDE1..ITEMS1 + 7 * ITEM_STRIDE1 + 4].copy_from_slice(&392u32.to_le_bytes());
        pack(&mut p);
        let d = p;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Resources/Funds", 99_999_999.into()).unwrap();
        doc.set("/Items/Item 08", 999.into()).unwrap();
        assert!(doc.set("/Items/Item 08", 1000.into()).is_err());
        assert!(doc.view.pointer("/Items/Item 01").is_none());
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(parse(&out).unwrap().view.pointer("/Resources/Funds").unwrap(), 99_999_999);
    }

    #[test]
    fn second_game_round_trip() {
        let mut p = blank(false);
        p[GIFTS2 + 3 * GIFT_STRIDE2..GIFTS2 + 3 * GIFT_STRIDE2 + 2].copy_from_slice(&7u16.to_le_bytes());
        p[MATERIALS2 + 3 * GIFT_STRIDE2..MATERIALS2 + 3 * GIFT_STRIDE2 + 4].copy_from_slice(&[0x61, 0x03, 0x29, 0x00]);
        pack(&mut p);
        let d = p;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Gifts/Gift 04", 99.into()).unwrap();
        assert!(doc.view.pointer("/Crafting materials/Material 01").is_none());
        doc.set("/Crafting materials/Material 04", 999.into()).unwrap();
        assert!(doc.set("/Crafting materials/Material 04", 1000.into()).is_err());
        doc.set("/Resources/Regiment funds", 5000.into()).unwrap();
        doc.set("/Resources/Wings of Freedom", 999.into()).unwrap();
        assert!(doc.set("/Resources/Wings of Freedom", 1000.into()).is_err());
        doc.set("/Player/Experience", 605_770.into()).unwrap();
        assert!(doc.set("/Player/Experience", 605_771.into()).is_err());
        assert!(doc.set("/Gifts/Gift 04", 100.into()).is_err());
        assert!(looks(&doc.to_bytes().unwrap()));
    }

    #[test]
    fn rejects_other_data() {
        assert!(!looks(&vec![0u8; SIZE1]));
        let mut d = blank(true);
        pack(&mut d);
        d[100] ^= 1;
        assert!(!looks(&d));
    }

    /// Research: real saves from SAVE_EDITOR_AOT (files separated by `|`) go through the whole
    /// decode chain, open on their own screen and survive an edit.
    #[test]
    #[ignore]
    fn real_saves_decode_and_edit() {
        use crate::formats::{decode_with, Content, Options};
        let list = std::env::var("SAVE_EDITOR_AOT").expect("SAVE_EDITOR_AOT");
        for f in list.split('|').filter(|s| !s.is_empty()) {
            let path = std::path::Path::new(f);
            let raw = std::fs::read(path).unwrap();
            let d = decode_with(&raw, &Options::for_path(path));
            let Content::Tree(mut t) = d.content else { panic!("{f}: not a tree") };
            println!("{f}: {} {}", t.kind_name, t.view);
            assert_eq!(t.to_bytes().unwrap(), raw, "{f}: unchanged round trip");
            let first = t.kind_name == KIND1;
            let (path_, hi) = if first { ("/Resources/Funds", 99_999_999) } else { ("/Gifts/Gift 01", 99) };
            t.set(path_, (hi - 1).into()).unwrap();
            let out = t.to_bytes().unwrap();
            assert_eq!(out.len(), raw.len());
            assert!(looks(&out));
        }
    }
}
