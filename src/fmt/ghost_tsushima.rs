//! Ghost of Tsushima Director's Cut (PC) saves: `manual_NNNN.sav`, `auto.sav`, `backup_NN.sav`.
//!
//! ```text
//! 0x00  u32 334
//! 0x04  u32 content size (the file size on PC)
//! 0x08  u32 checksum: sum of the bytes 0x10..size + size + 334 + version
//! 0x0C  u32 save format version (49 on PC)
//! ```
//!
//! The header and checksum follow the MIT-licensed notes of KOVRlN's
//! ghostoftsushima-save-converter. The rest was worked out from public saves.
//!
//! The game keeps its items as a list behind a u32 count: each entry is a name (u32 length and
//! ASCII text) and five u32 values. For resources the first value is 1 when the player has
//! some and the second is the amount. The currencies (`Xp`, `Talent`, …) are a second list:
//! a name and three u32 values (available, spent, total).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Ghost of Tsushima save";
const MAGIC: u32 = 334;
/// Highest amount a field accepts (saves with more than 10,000 supplies and 999,999 Ghost Flowers are known).
pub const SUPPLIES_LIMIT: i64 = 999_999;
pub const MATERIAL_LIMIT: i64 = 9_999;
/// What the Max crafting materials button raises materials to (fan wikis give 500 as the carry
/// limit and many full saves hold exactly 500; one known save holds 819 Yew Wood).
pub const MATERIAL_MAX: i64 = 500;
pub const SUPPLIES_MAX: i64 = 9_999;

/// `(save name, shown name, field limit)`.
pub const RESOURCES: &[(&str, &str, i64)] = &[
    ("Supplies", "Supplies", SUPPLIES_LIMIT),
    ("Flowers", "Flowers", SUPPLIES_LIMIT),
    ("NGPFlowers", "Ghost Flowers", SUPPLIES_LIMIT),
    ("IngredCoreBamboo", "Bamboo", MATERIAL_LIMIT),
    ("IngredUpgradeYew", "Yew Wood", MATERIAL_LIMIT),
    ("IngredUpgradeWaxTree", "Wax Wood", MATERIAL_LIMIT),
    ("IngredCoreOre", "Iron", MATERIAL_LIMIT),
    ("IngredUpgradeSteel", "Steel", MATERIAL_LIMIT),
    ("IngredUpgradeGold", "Gold", MATERIAL_LIMIT),
    ("IngredCoreLinen", "Linen", MATERIAL_LIMIT),
    ("IngredUpgradeLeather", "Leather", MATERIAL_LIMIT),
    ("IngredUpgradeSilk", "Silk", MATERIAL_LIMIT),
    ("Hide", "Predator Hides", MATERIAL_LIMIT),
];

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn checksum(d: &[u8], size: usize) -> u32 {
    let version = u32_at(d, 12).unwrap_or(0);
    d[16..size].iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b))).wrapping_add(size as u32).wrapping_add(MAGIC).wrapping_add(version)
}

fn size(d: &[u8]) -> Option<usize> {
    let size = u32_at(d, 4)? as usize;
    let version = u32_at(d, 12)?;
    (u32_at(d, 0)? == MAGIC && (48..=64).contains(&version) && size >= 0x200 && size <= d.len()).then_some(size)
}

pub fn looks(d: &[u8]) -> bool {
    size(d).is_some_and(|n| u32_at(d, 8) == Some(checksum(d, n)))
}

fn fix(d: &mut Vec<u8>) {
    // A resource raised from nothing needs its "has some" mark, or the game ignores the amount.
    if let Some(items) = items(d) {
        for (_, at) in items {
            if u32_at(d, at + 4).is_some_and(|n| n > 0) && u32_at(d, at) == Some(0) {
                d[at..at + 4].copy_from_slice(&1u32.to_le_bytes());
            }
        }
    }
    if let Some(n) = size(d) {
        let c = checksum(d, n);
        d[8..12].copy_from_slice(&c.to_le_bytes());
    }
}

/// A name (u32 length, then ASCII letters and digits) at `o`: the name and where it ends.
fn name(d: &[u8], o: usize) -> Option<(&str, usize)> {
    let n = u32_at(d, o)? as usize;
    let s = d.get(o + 4..o + 4 + n)?;
    (1..=48).contains(&n).then_some(())?;
    s.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_').then_some(())?;
    Some((std::str::from_utf8(s).ok()?, o + 4 + n))
}

/// The item list: `(name, offset of its five values)`.
fn items(d: &[u8]) -> Option<Vec<(String, usize)>> {
    const FIRST: &[u8] = b"\x0b\x00\x00\x00ArrowNormal";
    let mut from = 0;
    while let Some(p) = d[from..].windows(FIRST.len()).position(|w| w == FIRST) {
        let start = from + p;
        from = start + 1;
        let Some(count) = start.checked_sub(4).and_then(|c| u32_at(d, c)) else { continue };
        if !(20..=400).contains(&count) {
            continue;
        }
        let mut out = Vec::new();
        let mut o = start;
        for _ in 0..count {
            let Some((n, end)) = name(d, o) else { break };
            if end + 20 > d.len() {
                break;
            }
            out.push((n.to_string(), end));
            o = end + 20;
        }
        if out.len() == count as usize && out.iter().any(|(n, _)| n == "Supplies") {
            return Some(out);
        }
    }
    None
}

/// The currency list (`Xp`, `Talent`, …): `(name, offset of its three values)`.
fn currencies(d: &[u8]) -> Option<Vec<(String, usize)>> {
    const FIRST: &[u8] = b"\x02\x00\x00\x00Xp";
    let start = d.windows(FIRST.len()).position(|w| w == FIRST)?;
    let count = u32_at(d, start.checked_sub(4)?)?;
    if !(2..=64).contains(&count) {
        return None;
    }
    let mut out = Vec::new();
    let mut o = start;
    for _ in 0..count {
        let (n, end) = name(d, o)?;
        u32_at(d, end + 8)?;
        out.push((n.to_string(), end));
        o = end + 12;
    }
    Some(out)
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let n = size(d)?;
    let items = items(&d[..n])?;
    let mut b = Builder::new(d);
    for (key, shown, limit) in RESOURCES {
        if let Some((_, at)) = items.iter().find(|(k, _)| k == key) {
            b.ranged(&["Resources", shown], at + 4, Kind::U32, 0, *limit);
        }
    }
    if let Some(cur) = currencies(&d[..n]) {
        let get = |k: &str| cur.iter().find(|(n, _)| n == k).map(|(_, at)| *at);
        if let Some(at) = get("Talent") {
            b.ranged(&["Progress", "Technique points to spend"], at, Kind::U32, 0, 999);
        }
        if let Some(at) = get("Xp") {
            if let Some(v) = u32_at(d, at + 8) {
                b.info(&["Progress", "Legend earned (Tsushima)"], v);
            }
        }
        if let Some(at) = get("XpIki") {
            if let Some(v) = u32_at(d, at + 8) {
                b.info(&["Progress", "Legend earned (Iki Island)"], v);
            }
        }
    }
    b.info(&["Save", "Format version"], u32_at(d, 12)?);
    let note = crate::i18n::tr("Supplies, flowers, crafting materials and technique points. The checksum is made again on save. Checked on public saves, not yet tried in the game.");
    Some(b.finish(KIND, Checksum::Custom(fix)).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, vals: [u32; 5]) -> Vec<u8> {
        let mut v = (name.len() as u32).to_le_bytes().to_vec();
        v.extend(name.as_bytes());
        for x in vals {
            v.extend(x.to_le_bytes());
        }
        v
    }

    /// A made-up save with the same layout as a real one.
    fn sample() -> Vec<u8> {
        let mut body = vec![0u8; 0x198];
        body.extend(2u32.to_le_bytes());
        for (n, vals) in [("Xp", [3160u32, 0, 64825]), ("Talent", [4, 72, 1])] {
            body.extend((n.len() as u32).to_le_bytes());
            body.extend(n.as_bytes());
            for x in vals {
                body.extend(x.to_le_bytes());
            }
        }
        let mut list: Vec<(&str, [u32; 5])> = vec![("ArrowNormal", [3, 20, 20, 20, 20])];
        for _ in 0..19 {
            list.push(("ArrowFire", [3, 10, 10, 3, 3]));
        }
        list.push(("IngredCoreBamboo", [1, 366, 0, 0, 30]));
        list.push(("IngredUpgradeGold", [0, 0, 0, 0, u32::MAX]));
        list.push(("Supplies", [1, 10170, 0, 0, u32::MAX]));
        body.extend((list.len() as u32).to_le_bytes());
        for (n, v) in list {
            body.extend(entry(n, v));
        }
        body.extend([0u8; 64]);
        let n = body.len() as u32;
        body[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        body[4..8].copy_from_slice(&n.to_le_bytes());
        body[12..16].copy_from_slice(&49u32.to_le_bytes());
        let c = checksum(&body, body.len());
        body[8..12].copy_from_slice(&c.to_le_bytes());
        body
    }

    #[test]
    fn reads_resources_and_points() {
        let d = sample();
        assert!(looks(&d));
        let t = parse(&d).unwrap();
        assert_eq!(t.view.pointer("/Resources/Supplies"), Some(&serde_json::json!(10170)));
        assert_eq!(t.view.pointer("/Resources/Bamboo"), Some(&serde_json::json!(366)));
        assert_eq!(t.view.pointer("/Resources/Gold"), Some(&serde_json::json!(0)));
        assert_eq!(t.view.pointer("/Progress/Technique points to spend"), Some(&serde_json::json!(4)));
    }

    #[test]
    fn writes_amount_mark_and_checksum() {
        let d = sample();
        let mut t = parse(&d).unwrap();
        t.set("/Resources/Gold", serde_json::json!(500)).unwrap();
        t.set("/Resources/Supplies", serde_json::json!(20000)).unwrap();
        let out = t.to_bytes().unwrap();
        assert!(looks(&out));
        let items = items(&out).unwrap();
        let gold = items.iter().find(|(n, _)| n == "IngredUpgradeGold").unwrap().1;
        assert_eq!(u32_at(&out, gold), Some(1));
        assert_eq!(u32_at(&out, gold + 4), Some(500));
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Resources/Supplies"), Some(&serde_json::json!(20000)));
        assert_eq!(out.len(), d.len());
    }

    #[test]
    fn refuses_a_bad_checksum() {
        let mut d = sample();
        d[0x300] ^= 1;
        assert!(!looks(&d));
    }

    /// Research aid: SAVE_EDITOR_GOT = a folder of Ghost of Tsushima `.sav` files.
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("SAVE_EDITOR_GOT").unwrap();
        let mut seen = 0;
        for e in walk(std::path::Path::new(&dir)) {
            let d = std::fs::read(&e).unwrap();
            assert!(looks(&d), "{}", e.display());
            let t = parse(&d).unwrap_or_else(|| panic!("{}", e.display()));
            let mut t2 = parse(&d).unwrap();
            // Writing without changes gives the same bytes.
            assert_eq!(t2.to_bytes().unwrap(), d);
            let _ = t2.set("/Resources/Silk", serde_json::json!(321));
            let out = t2.to_bytes().unwrap();
            assert!(looks(&out));
            println!("{}: {:?}", e.display(), t.view.get("Resources"));
            let any = crate::fmt::parse_binary(&d).unwrap();
            assert_eq!(any.kind_name, KIND);
            let game = crate::game_ui::detect(KIND, &any.view).unwrap();
            assert_eq!(game.name(), Some("Ghost of Tsushima"));
            seen += 1;
        }
        assert!(seen > 0);
    }

    fn walk(p: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = vec![];
        for e in std::fs::read_dir(p).unwrap().flatten() {
            let q = e.path();
            if q.is_dir() {
                out.extend(walk(&q));
            } else if q.extension().is_some_and(|x| x == "sav") {
                out.push(q);
            }
        }
        out
    }
}
