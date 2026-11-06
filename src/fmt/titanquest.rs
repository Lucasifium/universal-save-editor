//! Titan Quest (Anniversary Edition / Immortal Throne) characters (`Player.chr`).
//!
//! The file is a flat stream of key/value records with no checksum: a key is a u32 length
//! and its ASCII name, and the value's type follows from the key (u32 or f32, a text with
//! a u32 length, UTF-16 text with a u32 count, or a 16-byte id). `begin_block` and
//! `end_block` carry markers. Keys repeat between blocks (`temp`), so values are found by
//! key and order: the five attributes are the `temp` values right after `skillPoints`
//! (strength, dexterity, intelligence, health, energy). Layout facts from TQVaultAE
//! (MIT) and tqrespec (facts only). Every number is changed in place.

use super::fields::{Builder, Checksum, Kind};
use super::grimdawn::record_name;
use crate::tree::TreeDoc;
use std::collections::HashMap;

/// A value's position, length and how it was read.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ty {
    Word,
    Text,
    Wide,
    Blob,
}

struct Tok<'a> {
    key: &'a str,
    at: usize,
    len: usize,
    ty: Ty,
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Is there a key at `p` (or the end of the file)?
fn is_key(d: &[u8], p: usize) -> bool {
    if p == d.len() {
        return true;
    }
    let Some(n) = u32_at(d, p).map(|n| n as usize) else { return false };
    if !(1..=64).contains(&n) {
        return false;
    }
    let Some(k) = d.get(p + 4..p + 4 + n) else { return false };
    (k[0].is_ascii_alphabetic() || k[0] == b'_') && k.iter().all(|&c| c.is_ascii_alphanumeric() || b"_.[]".contains(&c))
}

/// Splits the file into key/value records; a value's type is told by where the next key starts.
fn tokens(d: &[u8]) -> Result<Vec<Tok<'_>>, String> {
    let mut out = vec![];
    let mut p = 0;
    while p < d.len() {
        if !is_key(d, p) {
            return Err(format!("expected a key at {p}"));
        }
        let n = u32_at(d, p).unwrap() as usize;
        let key = std::str::from_utf8(&d[p + 4..p + 4 + n]).map_err(|e| e.to_string())?;
        p += 4 + n;
        let v = u32_at(d, p).map(|v| v as usize);
        let (len, ty) = if key == "begin_block" || key == "end_block" || (v.is_some() && is_key(d, p + 4)) {
            (4, Ty::Word)
        } else if v.is_some_and(|v| v > 0 && v < 4096 && is_key(d, p + 4 + v)) {
            (4 + v.unwrap(), Ty::Text)
        } else if v.is_some_and(|v| v > 0 && v < 2048 && is_key(d, p + 4 + 2 * v)) {
            (4 + 2 * v.unwrap(), Ty::Wide)
        } else {
            // Something else (an id, a trailing description): up to the next key.
            let mut q = p + 1;
            while q < d.len() && !is_key(d, q) {
                q += 1;
            }
            (q - p, Ty::Blob)
        };
        if p + len > d.len() {
            return Err("the save ends early".into());
        }
        out.push(Tok { key, at: p, len, ty });
        p += len;
    }
    Ok(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && d.starts_with(b"\x0d\0\0\0headerVersion")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Titan Quest character".into());
    }
    let toks = tokens(d)?;
    let mut b = Builder::new(d);
    let text = |t: &Tok| -> String {
        let s = &d[t.at + 4..t.at + t.len];
        match t.ty {
            Ty::Text => s.iter().map(|&c| c as char).collect(),
            Ty::Wide => String::from_utf16_lossy(&s.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>()),
            // The mobile version writes the name as UTF-32.
            Ty::Blob if t.len >= 4 && u32_at(d, t.at).is_some_and(|n| 4 + 4 * n as usize == t.len) => {
                s.chunks(4).filter_map(|c| char::from_u32(u32::from_le_bytes([c[0], c[1], c[2], c[3]]))).collect()
            }
            _ => String::new(),
        }
    };
    let first = |key: &str| toks.iter().position(|t| t.key == key);
    let word = |key: &str| first(key).filter(|&i| toks[i].ty == Ty::Word).map(|i| &toks[i]);
    if let Some(i) = first("myPlayerName") {
        b.info(&["Character", "Name"], text(&toks[i]));
    }
    for (key, label) in [("playerCharacterClass", "Class"), ("playerClassTag", "Class tag")] {
        if let Some(i) = first(key).filter(|&i| toks[i].ty == Ty::Text) {
            b.info(&["Character", label], text(&toks[i]));
        }
    }
    let max = i32::MAX as i64;
    if let Some(t) = word("currentStats.charLevel") {
        b.ranged(&["Character", "Level"], t.at, Kind::U32, 1, 85);
        if let Some(h) = word("playerLevel") {
            b.mirror(&["Character", "Level"], h.at);
        }
    }
    if let Some(t) = word("currentStats.experiencePoints") {
        b.ranged(&["Character", "Experience"], t.at, Kind::U32, 0, max);
    }
    if let Some(t) = word("money") {
        b.ranged(&["Character", "Money"], t.at, Kind::U32, 0, max);
    }
    for (key, label) in [("modifierPoints", "Attribute points"), ("skillPoints", "Skill points")] {
        if let Some(t) = word(key) {
            b.ranged(&["Points", label], t.at, Kind::U32, 0, 10_000);
        }
    }
    // The five attributes: the next `temp` values after skillPoints (past block markers).
    if let Some(i) = first("skillPoints") {
        let temps: Vec<&Tok> = toks[i + 1..].iter().filter(|t| t.key != "begin_block" && t.key != "end_block").take(5).collect();
        if temps.len() == 5 && temps.iter().all(|t| t.key == "temp" && t.ty == Ty::Word) {
            for (t, label) in temps.iter().zip(["Strength", "Dexterity", "Intelligence", "Health", "Energy"]) {
                b.ranged(&["Attributes", label], t.at, Kind::F32, 0, 100_000);
            }
        }
    }
    // Skills: a skillName followed straight away by its skillLevel.
    let mut seen: HashMap<String, usize> = HashMap::new();
    for w in toks.windows(2) {
        if w[0].key == "skillName" && w[0].ty == Ty::Text && w[1].key == "skillLevel" && w[1].ty == Ty::Word {
            let name = record_name(&text(&w[0]));
            let n = seen.entry(name.clone()).or_default();
            *n += 1;
            let label = if *n > 1 { format!("{name} ({n})") } else { name };
            b.ranged(&["Skills", &label], w[1].at, Kind::U32, 0, 255);
        }
    }
    for (key, label) in [("playTimeInSeconds", "Play time (seconds)"), ("numberOfKills", "Kills"), ("numberOfDeaths", "Deaths")] {
        if let Some(v) = word(key).and_then(|t| u32_at(d, t.at)) {
            b.info(&["Statistics", label], v);
        }
    }
    Ok(b.finish("Titan Quest character", Checksum::None).with_note(
        "Level, experience, money, unspent points, the five attributes and skill levels (skills show the game's \
         internal record names). This file has no checksum; values are changed in place.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(d: &mut Vec<u8>, k: &str) {
        d.extend((k.len() as u32).to_le_bytes());
        d.extend(k.as_bytes());
    }
    fn word(d: &mut Vec<u8>, k: &str, v: u32) {
        key(d, k);
        d.extend(v.to_le_bytes());
    }
    fn text(d: &mut Vec<u8>, k: &str, v: &str) {
        key(d, k);
        d.extend((v.len() as u32).to_le_bytes());
        d.extend(v.as_bytes());
    }

    /// A synthetic character with the records the reader looks for.
    fn sample() -> Vec<u8> {
        let mut d = vec![];
        word(&mut d, "headerVersion", 3);
        text(&mut d, "playerCharacterClass", "Warrior");
        word(&mut d, "playerLevel", 10);
        key(&mut d, "myPlayerName");
        d.extend(4u32.to_le_bytes());
        d.extend("Hero".encode_utf16().flat_map(|c| c.to_le_bytes()));
        word(&mut d, "begin_block", 0xB01D_FACE);
        word(&mut d, "money", 1234);
        text(&mut d, "skillName", r"Records\Skills\Storm\StormMastery.dbr");
        word(&mut d, "skillLevel", 8);
        text(&mut d, "skillName", r"Records\Skills\Storm\IceShard.dbr");
        word(&mut d, "skillLevel", 3);
        word(&mut d, "end_block", 0xDEAD_C0DE);
        word(&mut d, "currentStats.charLevel", 10);
        word(&mut d, "currentStats.experiencePoints", 5000);
        word(&mut d, "modifierPoints", 2);
        word(&mut d, "skillPoints", 6);
        word(&mut d, "end_block", 0xDEAD_C0DE);
        word(&mut d, "begin_block", 0xB01D_FACE);
        for f in [50.0f32, 60.0, 70.0, 300.0, 300.0] {
            word(&mut d, "temp", f.to_bits());
        }
        word(&mut d, "end_block", 0xDEAD_C0DE);
        key(&mut d, "uniqueId");
        d.extend([0x99; 16]);
        word(&mut d, "temp", 1);
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Character/Name"), Some(&"Hero".into()));
        assert_eq!(v.pointer("/Character/Class"), Some(&"Warrior".into()));
        assert_eq!(v.pointer("/Character/Money"), Some(&1234.into()));
        assert_eq!(v.pointer("/Attributes/Intelligence"), Some(&70.0.into()));
        assert_eq!(v.pointer("/Skills/Storm: Storm mastery"), Some(&8.into()));
        assert_eq!(v.pointer("/Skills/Storm: Ice shard"), Some(&3.into()));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Character/Level", 11.into()).unwrap();
        doc.set("/Attributes/Strength", 55.0.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Character/Level"), Some(&11.into()));
        assert_eq!(again.view.pointer("/Attributes/Strength"), Some(&55.0.into()));
        // The header's copy of the level follows.
        let at = d.windows(11).position(|w| w == b"playerLevel").unwrap() + 11;
        assert_eq!(&out[at..at + 4], &11u32.to_le_bytes());
    }

    /// SAVE_EDITOR_TQ = a folder with Player.chr files.
    #[test]
    #[ignore]
    fn real_saves() {
        crate::fmt::grimdawn::tests::research("SAVE_EDITOR_TQ", |n| n.eq_ignore_ascii_case("player.chr"), "Titan Quest character", &["/Character/Money", "/Attributes/Strength", "/Character/Level", "/Points/Skill points"]);
    }
}
