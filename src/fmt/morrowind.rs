//! The Elder Scrolls III: Morrowind saves (`.ess`).
//!
//! A save is a list of records, each `tag(4) size(4) unused(4) flags(4) body`, whose bodies are
//! lists of sub-records `tag(4) size(4) data`. Nothing in the file is checksummed or compressed,
//! so every value is patched in place. The player is two records:
//!
//! * the `NPC_` record named `player`: level (`NPDT`, u16), the inventory (`NPCO`: a 4-byte count
//!   and a 32-byte item id; the gold is the item `Gold_001`) and the spell list (`NPCS`);
//! * the `REFR` record named `PlayerSaveGame`: `ACDT` holds the live health, fatigue, magicka
//!   and the eight attributes, each as a current and a base float; `CHRD` holds the 27 skills as
//!   current and base integers.
//!
//! The `GAME` record's `GMDT` repeats the health for the load screen, so it is kept equal.
//! Layouts worked out on public saves (a level-53 Breton and a cheated Dark Elf), with the
//! record framing as described by the long-standing community documentation of the format.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Morrowind save";

const ATTRIBUTES: [&str; 8] = ["Strength", "Intelligence", "Willpower", "Agility", "Speed", "Endurance", "Personality", "Luck"];
const SKILLS: [&str; 27] = [
    "Block", "Armorer", "Medium armor", "Heavy armor", "Blunt weapon", "Long blade", "Axe", "Spear", "Athletics", "Enchant", "Destruction",
    "Alteration", "Illusion", "Conjuration", "Mysticism", "Restoration", "Alchemy", "Unarmored", "Security", "Sneak", "Acrobatics", "Light armor",
    "Short blade", "Marksman", "Mercantile", "Speechcraft", "Hand-to-hand",
];

/// Offsets inside `ACDT` (4-byte slots): health, fatigue and magicka as (current, base), then
/// the attributes as (current, base) pairs.
const ACDT_HEALTH: usize = 40;
const ACDT_FATIGUE: usize = 48;
const ACDT_MAGICKA: usize = 56;
const ACDT_ATTRS: usize = 80;
const ACDT_LEN: usize = 264;

pub fn looks(d: &[u8]) -> bool {
    d.len() > 400 && d.starts_with(b"TES3") && find_game(d)
}

fn find_game(d: &[u8]) -> bool {
    records(d).any(|(t, _, _)| &t == b"GAME")
}

fn u32at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

/// `(tag, body start, body length)` of every top-level record.
fn records(d: &[u8]) -> impl Iterator<Item = ([u8; 4], usize, usize)> + '_ {
    let mut at = 0usize;
    std::iter::from_fn(move || {
        let tag: [u8; 4] = d.get(at..at + 4)?.try_into().ok()?;
        let size = u32at(d, at + 4)?;
        let start = at + 16;
        if start.checked_add(size)? > d.len() {
            return None;
        }
        at = start + size;
        Some((tag, start, size))
    })
}

/// `(tag, data start, data length)` of every sub-record in a record body.
fn subs(d: &[u8], start: usize, len: usize) -> Vec<([u8; 4], usize, usize)> {
    let mut out = vec![];
    let end = start + len;
    let mut at = start;
    while at + 8 <= end {
        let Ok(tag) = <[u8; 4]>::try_from(&d[at..at + 4]) else { break };
        let Some(size) = u32at(d, at + 4) else { break };
        if at + 8 + size > end {
            break;
        }
        out.push((tag, at + 8, size));
        at += 8 + size;
    }
    out
}

fn text(d: &[u8], at: usize, len: usize) -> String {
    let b = &d[at..at + len];
    let b = &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())];
    b.iter().map(|&c| super::bethesda_cp1252(c)).collect()
}

fn sub_text(d: &[u8], s: &[([u8; 4], usize, usize)], tag: &[u8; 4]) -> Option<String> {
    s.iter().find(|x| &x.0 == tag).map(|&(_, at, len)| text(d, at, len))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !d.starts_with(b"TES3") {
        return Err("not a Morrowind save".into());
    }
    let mut npc = None;
    let mut refr = None;
    let mut gmdt = None;
    for (tag, start, len) in records(d) {
        match &tag {
            b"NPC_" if npc.is_none() => {
                let s = subs(d, start, len);
                if sub_text(d, &s, b"NAME").is_some_and(|n| n.eq_ignore_ascii_case("player")) {
                    npc = Some(s);
                }
            }
            b"REFR" if refr.is_none() => {
                let s = subs(d, start, len);
                if sub_text(d, &s, b"NAME").is_some_and(|n| n == "PlayerSaveGame") {
                    refr = Some(s);
                }
            }
            b"GAME" if gmdt.is_none() => {
                gmdt = subs(d, start, len).into_iter().find(|x| &x.0 == b"GMDT");
            }
            _ => {}
        }
    }
    let npc = npc.ok_or("not a Morrowind save (no player record)")?;
    let refr = refr.ok_or("not a Morrowind save (no player data)")?;
    let acdt = refr.iter().find(|x| &x.0 == b"ACDT" && x.2 >= ACDT_LEN).map(|x| x.1).ok_or("no actor data")?;
    let chrd = refr.iter().find(|x| &x.0 == b"CHRD" && x.2 >= 27 * 8).map(|x| x.1).ok_or("no skills")?;

    let mut b = Builder::new(d);
    if let Some(n) = sub_text(d, &npc, b"FNAM") {
        b.info(&["Character", "Name"], n);
    }
    if let Some(n) = sub_text(d, &npc, b"RNAM") {
        b.info(&["Character", "Race"], n);
    }
    if let Some(n) = sub_text(d, &npc, b"CNAM") {
        b.info(&["Character", "Class"], n);
    }
    if let Some(&(_, at, len)) = npc.iter().find(|x| &x.0 == b"NPDT") {
        if len >= 52 {
            b.ranged(&["Character", "Level"], at, Kind::U16, 1, 9999);
        }
    }
    // The inventory: 36 bytes each, a signed count then a 32-byte id.
    let mut seen: std::collections::HashMap<String, usize> = Default::default();
    let mut gold = false;
    for &(_, at, len) in npc.iter().filter(|x| &x.0 == b"NPCO") {
        if len != 36 {
            continue;
        }
        let id = text(d, at + 4, 32);
        if id.is_empty() {
            continue;
        }
        if id == "Gold_001" && !gold {
            gold = true;
            b.ranged(&["Character", "Gold"], at, Kind::I32, 0, i32::MAX as i64);
            continue;
        }
        let n = seen.entry(id.clone()).or_insert(0);
        *n += 1;
        let key = if *n == 1 { id } else { format!("{id} ({n})") };
        b.field(&["Inventory", &key], at, Kind::I32);
    }
    let spells: Vec<String> = npc.iter().filter(|x| &x.0 == b"NPCS" && x.2 == 32).map(|&(_, at, len)| text(d, at, len)).collect();
    if !spells.is_empty() {
        b.info(&["Spells"], serde_json::Value::from(spells));
    }

    for (i, name) in ATTRIBUTES.iter().enumerate() {
        let at = acdt + ACDT_ATTRS + i * 8;
        b.ranged(&["Attributes", name], at, Kind::F32Int, 0, 9999);
        b.mirror(&["Attributes", name], at + 4);
    }
    for (i, name) in SKILLS.iter().enumerate() {
        let at = chrd + i * 8;
        b.ranged(&["Skills", name], at, Kind::I32, 0, 9999);
        b.mirror(&["Skills", name], at + 4);
    }
    let vital = |b: &mut Builder, name: &str, base: &str, at: usize, copy: Option<usize>| {
        b.ranged(&["Vitals", name], acdt + at, Kind::F32, 0, 99_999);
        b.ranged(&["Vitals", base], acdt + at + 4, Kind::F32, 0, 99_999);
        if let Some(g) = copy {
            b.mirror(&["Vitals", name], g);
            b.mirror(&["Vitals", base], g + 4);
        }
    };
    let g = gmdt.filter(|x| x.2 >= 8).map(|x| x.1);
    vital(&mut b, "Health", "Full health", ACDT_HEALTH, g);
    vital(&mut b, "Fatigue", "Full fatigue", ACDT_FATIGUE, None);
    vital(&mut b, "Magicka", "Full magicka", ACDT_MAGICKA, None);
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Level, gold, attributes, skills, health, fatigue, magicka and the inventory counts of a Morrowind .ess save. Checked on public saves, not yet tried in the game. Back up the save first.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = tag.to_vec();
        v.extend((data.len() as u32).to_le_bytes());
        v.extend(data);
        v
    }

    fn rec(tag: &[u8; 4], subs: Vec<Vec<u8>>) -> Vec<u8> {
        let body: Vec<u8> = subs.concat();
        let mut v = tag.to_vec();
        v.extend((body.len() as u32).to_le_bytes());
        v.extend([0u8; 8]);
        v.extend(body);
        v
    }

    fn item(id: &str, n: i32) -> Vec<u8> {
        let mut v = n.to_le_bytes().to_vec();
        let mut name = id.as_bytes().to_vec();
        name.resize(32, 0);
        v.extend(name);
        v
    }

    pub fn synthetic() -> Vec<u8> {
        let mut npdt = vec![0u8; 52];
        npdt[0] = 12;
        let mut acdt = vec![0u8; ACDT_LEN];
        for (o, v) in [(ACDT_HEALTH, 90.0f32), (ACDT_HEALTH + 4, 100.0), (ACDT_FATIGUE, 200.0), (ACDT_FATIGUE + 4, 200.0)] {
            acdt[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        for i in 0..8 {
            for k in 0..2 {
                let o = ACDT_ATTRS + i * 8 + k * 4;
                acdt[o..o + 4].copy_from_slice(&(40.0f32 + i as f32).to_le_bytes());
            }
        }
        let mut chrd = vec![];
        for i in 0..27i32 {
            chrd.extend((10 + i).to_le_bytes());
            chrd.extend((10 + i).to_le_bytes());
        }
        let mut gmdt = vec![0u8; 124];
        gmdt[0..4].copy_from_slice(&90.0f32.to_le_bytes());
        gmdt[4..8].copy_from_slice(&100.0f32.to_le_bytes());
        let mut v = rec(b"TES3", vec![sub(b"HEDR", &[0u8; 300])]);
        v.extend(rec(b"GAME", vec![sub(b"GMDT", &gmdt)]));
        v.extend(rec(
            b"NPC_",
            vec![
                sub(b"NAME", b"player\0"),
                sub(b"FNAM", b"Tester\0"),
                sub(b"RNAM", b"Nord\0"),
                sub(b"CNAM", b"Knight\0"),
                sub(b"NPDT", &npdt),
                sub(b"NPCO", &item("Gold_001", 500)),
                sub(b"NPCO", &item("misc_soulgem_petty", 3)),
                sub(b"NPCS", &{
                    let mut s = b"flare".to_vec();
                    s.resize(32, 0);
                    s
                }),
            ],
        ));
        v.extend(rec(b"REFR", vec![sub(b"NAME", b"PlayerSaveGame\0"), sub(b"ACDT", &acdt), sub(b"CHRD", &chrd)]));
        v
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = synthetic();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/Level"), Some(&serde_json::json!(12)));
        assert_eq!(doc.view.pointer("/Character/Gold"), Some(&serde_json::json!(500)));
        assert_eq!(doc.view.pointer("/Character/Name"), Some(&serde_json::json!("Tester")));
        assert_eq!(doc.view.pointer("/Attributes/Speed"), Some(&serde_json::json!(44)));
        assert_eq!(doc.view.pointer("/Skills/Block"), Some(&serde_json::json!(10)));
        assert_eq!(doc.view.pointer("/Skills/Hand-to-hand"), Some(&serde_json::json!(36)));
        assert_eq!(doc.view.pointer("/Inventory/misc_soulgem_petty"), Some(&serde_json::json!(3)));
        doc.set("/Character/Gold", serde_json::json!(999_999)).unwrap();
        doc.set("/Skills/Sneak", serde_json::json!(100)).unwrap();
        doc.set("/Attributes/Luck", serde_json::json!(100)).unwrap();
        doc.set("/Vitals/Health", serde_json::json!(300)).unwrap();
        assert!(doc.set("/Skills/Sneak", serde_json::json!(-5)).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Character/Gold"), Some(&serde_json::json!(999_999)));
        assert_eq!(back.view.pointer("/Skills/Sneak"), Some(&serde_json::json!(100)));
        assert_eq!(back.view.pointer("/Attributes/Luck"), Some(&serde_json::json!(100)));
        assert_eq!(back.view.pointer("/Vitals/Health"), Some(&serde_json::json!(300.0)));
        // The load screen's copy of the health follows.
        let g = d.windows(4).position(|w| w == b"GMDT").unwrap() + 8;
        assert_eq!(&out[g..g + 4], &300.0f32.to_le_bytes());
        // Only the edited bytes differ.
        assert!(out.iter().zip(&d).filter(|(a, b)| a != b).count() < 24);
    }

    #[test]
    fn refuses_plugins_and_junk() {
        let esp = rec(b"TES3", vec![sub(b"HEDR", &[0u8; 300])]);
        assert!(parse(&esp).is_err());
        assert!(parse(b"nothing here").is_err());
    }

    /// Research aid: SAVE_EDITOR_MW = a Morrowind `.ess`; edits are written to SAVE_EDITOR_MW_OUT.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_MW").unwrap()).unwrap();
        let mut doc = parse(&d).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view["Character"]).unwrap());
        let unchanged = doc.to_bytes().unwrap();
        assert_eq!(unchanged, d, "an unedited save is written back byte for byte");
        doc.set("/Character/Gold", serde_json::json!(777_777)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(parse(&out).unwrap().view.pointer("/Character/Gold"), Some(&serde_json::json!(777_777)));
    }
}
