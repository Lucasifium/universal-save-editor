//! The Legend of Zelda: Breath of the Wild `game_data.sav` (Wii U, big-endian, or Switch,
//! little-endian; about 0.9 to 1 MB): a 12-byte header, then pairs of a 32-bit value hash and
//! its 32-bit value. Values are found by hash; the inventory is 420 names of 16 four-letter
//! pieces (one per pair), with the counts in a second run of pairs.
//!
//! Hashes, item ids, English names and limits after marcrobledo/savegame-editors (MIT; names
//! datamined by MrCheeze, see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Breath of the Wild save";
const HEADERS: [u32; 16] = [
    0x24e2, 0x24ee, 0x2588, 0x29c0, 0x2a46, 0x2f8e, 0x3ef8, 0x3ef9, 0x471a, 0x471b, 0x471b, 0x471e, 0x0f423d, 0x0f423e, 0x0f423f, 0x4730,
];
const ITEMS_DATA: &str = include_str!("data/botw_items.txt");
const MAX_ITEMS: usize = 420;

fn u32_at(d: &[u8], at: usize, be: bool) -> Option<u32> {
    let b: [u8; 4] = d.get(at..at + 4)?.try_into().ok()?;
    Some(if be { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
}

/// Which byte order the file is in, if it is a Breath of the Wild save.
fn order(d: &[u8]) -> Option<bool> {
    if !(896_976..=1_500_000).contains(&d.len()) {
        return None;
    }
    [true, false].into_iter().find(|&be| u32_at(d, 4, be) == Some(0xffff_ffff) && u32_at(d, 0, be).is_some_and(|h| HEADERS.contains(&h)))
}

pub fn looks(d: &[u8]) -> bool {
    order(d).is_some()
}

/// Position of the value that follows `hash`, searching on from `from`.
fn find(d: &[u8], be: bool, hash: u32, from: usize) -> Option<usize> {
    let mut i = from;
    while i + 8 <= d.len() {
        if u32_at(d, i, be) == Some(hash) {
            return Some(i + 4);
        }
        i += 8;
    }
    None
}

fn name_at(d: &[u8], be: bool, at: usize) -> String {
    let _ = be;
    let mut s = String::new();
    for k in 0..16 {
        let Some(p) = d.get(at + k * 8..at + k * 8 + 4) else { break };
        for &c in p {
            if c != 0 {
                s.push(c as char);
            }
        }
    }
    s
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let be = order(d).ok_or("not a Breath of the Wild save")?;
    let mut b = Builder::new(d);
    b.big_endian = be;
    let mut from = 0x0c;
    let mut at = |hash: u32, from: &mut usize| -> Option<usize> {
        let p = find(d, be, hash, *from)?;
        *from = p + 4;
        Some(p)
    };
    // The hashes are in ascending order, so each search carries on from the last.
    let mut ent: std::collections::HashMap<u32, usize> = Default::default();
    for h in [
        0x0bee9e46u32, 0x0cbf052a, 0x1e3fd294, 0x23149bf8, 0x2906f327, 0x333aa6e5, 0x3adff047, 0x441b7231, 0x54679940, 0x57ee221d, 0x5f283289,
        0x6150c6be, 0x698266be, 0x69f17e8a, 0x6a09fc59, 0x73c29681, 0x7b74e117, 0x8a94e07a, 0x9383490e, 0x97f925c3, 0x982ba201, 0x9c6cfd3f,
        0xa40ba103, 0xa6d926bc, 0xc247b696, 0xc5238d2b, 0xc9328299, 0xce7afed3, 0xd913b769, 0xe1a0ca54, 0xea9def3f, 0xf1cf4807, 0xfda0cde4,
    ] {
        if let Some(p) = at(h, &mut from) {
            ent.insert(h, p);
        }
    }
    if let Some(&p) = ent.get(&0x23149bf8) {
        b.ranged(&["Player", "Rupees"], p, Kind::U32, 0, 999_999);
    }
    if let Some(&p) = ent.get(&0xce7afed3) {
        b.ranged(&["Player", "Monster parts (Mons)"], p, Kind::U32, 0, 999_999);
    }
    if let Some(&p) = ent.get(&0x2906f327) {
        b.ranged(&["Player", "Max hearts (4 per heart)"], p, Kind::U32, 4, 120);
    }
    if let Some(&p) = ent.get(&0x3adff047) {
        b.ranged(&["Player", "Max stamina (1000 per wheel)"], p, Kind::F32Int, 1000, 3000);
    }
    if let Some(&p) = ent.get(&0x8a94e07a) {
        b.ranged(&["Player", "Korok seeds found"], p, Kind::U32, 0, 900);
    }
    if let Some(&p) = ent.get(&0x54679940) {
        b.ranged(&["Player", "Hinox defeated"], p, Kind::U32, 0, 99_999);
    }
    if let Some(&p) = ent.get(&0x698266be) {
        b.ranged(&["Player", "Talus defeated"], p, Kind::U32, 0, 99_999);
    }
    if let Some(&p) = ent.get(&0x441b7231) {
        b.ranged(&["Player", "Molduga defeated"], p, Kind::U32, 0, 99_999);
    }
    if let Some(&p) = ent.get(&0x73c29681) {
        b.ranged(&["Player", "Play time (seconds)"], p, Kind::U32, 0, 4_000_000_000);
    }
    if let (Some(&names), Some(&qty)) = (ent.get(&0x5f283289), ent.get(&0x6a09fc59)) {
        let cat = |id: &str| -> Option<(&'static str, i64)> {
            let line = ITEMS_DATA.lines().find(|l| l.split('\t').nth(1) == Some(id))?;
            let c = line.split('\t').next()?;
            match c {
                "materials" => Some(("Materials", 999)),
                "food" => Some(("Food", 999)),
                "bows" if id.ends_with("Arrow") || id.ends_with("Arrow_A") => Some(("Arrows", 999)),
                _ if id == "Obj_KorokNuts" => Some(("Other", 900)),
                _ if id == "Obj_DungeonClearSeal" => Some(("Other", 120)),
                _ => None,
            }
        };
        for i in 0..MAX_ITEMS {
            let id = name_at(d, be, names + i * 0x80);
            if id.is_empty() {
                continue;
            }
            if let Some((group, max)) = cat(&id) {
                let q = qty + i * 8;
                if q + 4 <= d.len() {
                    b.ranged(&[group, &id, "Quantity"], q, Kind::U32, 0, max);
                }
            }
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the game or emulator first. Weapons, armour and key items are not edited. Item names show in English.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_by_hash() {
        let be = true;
        let mut d = vec![0u8; 900_000];
        d[..4].copy_from_slice(&0x24eeu32.to_be_bytes());
        d[4..8].copy_from_slice(&0xffff_ffffu32.to_be_bytes());
        d[0x0c..0x10].copy_from_slice(&0x23149bf8u32.to_be_bytes());
        d[0x10..0x14].copy_from_slice(&500u32.to_be_bytes());
        assert_eq!(order(&d), Some(be));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Player/Rupees", 999.into()).unwrap();
        assert_eq!(u32::from_be_bytes([doc.to_bytes().unwrap()[0x10], doc.to_bytes().unwrap()[0x11], doc.to_bytes().unwrap()[0x12], doc.to_bytes().unwrap()[0x13]]), 999);
    }
}
