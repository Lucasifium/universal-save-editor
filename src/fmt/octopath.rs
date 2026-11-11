//! Octopath Traveler and Octopath Traveler II: plain Unreal `GVAS` saves of the class
//! `KSSaveGameBP_C`, read by `gvas.rs` (no encryption, compression or checksum; numbers are
//! changed in place). This module only names them and finds the values the game screen
//! shows. Octopath Traveler (2018, UE 4.18) gives every property a Blueprint suffix
//! (`Money_2_2E94…`), so names are matched by their start; Octopath Traveler II uses bare
//! names. The party members are an array of structs (`PlayerMember/0`, `/1`, …): the eight
//! travellers by character id, then guests and empty slots.
//!
//! Character and job ids: Octopath Traveler II from RyudoSynbios' game-tools-collection
//! template (MIT), Octopath Traveler from AppleBoiy's editor (GPL, facts only); both checked
//! against the first job each character has in sample saves.

use crate::tree::TreeDoc;
use serde_json::{Map, Value};

pub const OT1: &str = "Octopath Traveler save";
pub const OT2: &str = "Octopath Traveler II save";

/// The eight travellers by character id 1–8.
pub const OT1_CHARACTERS: [&str; 8] = [
    "Olberic", "Tressa", "Cyrus", "Primrose", "H'aanit", "Therion", "Ophilia", "Alfyn",
];
pub const OT2_CHARACTERS: [&str; 8] = [
    "Hikari",
    "Ochette",
    "Castti",
    "Partitio",
    "Temenos",
    "Osvald",
    "Thron\u{00E9}",
    "Agnea",
];
/// The base jobs 0–7 (the same in both games).
pub const JOBS: [&str; 8] = [
    "Merchant",
    "Thief",
    "Warrior",
    "Hunter",
    "Cleric",
    "Dancer",
    "Scholar",
    "Apothecary",
];

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"GVAS") && d.windows(12).take(4096).any(|w| w == b"KSSaveGameBP")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut doc = super::gvas::parse(d)?;
    let root = doc.view.as_object().ok_or("not an Octopath save")?;
    if key(root, "PlayerMember").is_none() || key(root, "PlayerBackpack").is_none() {
        return Err("not an Octopath save".into());
    }
    // Only the first game's names (inside the top-level structs) carry a suffix.
    let first = ptr(&doc.view, "", "PlayerBackpack")
        .and_then(|p| {
            doc.view
                .pointer(&p)?
                .as_object()
                .map(|m| m.keys().any(|k| bare(k) != k))
        })
        .unwrap_or(false);
    doc.kind_name = (if first { OT1 } else { OT2 }).into();
    Ok(doc)
}

/// The property's bare name: `Level_7_2387E075…` → `Level`, `PlayerMember (3)` → `PlayerMember`.
pub fn bare(k: &str) -> &str {
    let k = match k.rsplit_once(" (") {
        Some((a, b))
            if b.ends_with(')') && b[..b.len() - 1].bytes().all(|c| c.is_ascii_digit()) =>
        {
            a
        }
        _ => k,
    };
    // Blueprint suffix: _<number>_<32 hex digits>.
    if let Some((head, guid)) = k.rsplit_once('_') {
        if guid.len() == 32 && guid.bytes().all(|c| c.is_ascii_hexdigit()) {
            if let Some((name, n)) = head.rsplit_once('_') {
                if !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) {
                    return name;
                }
            }
        }
    }
    k
}

/// The key of the first property named `name` (either game's spelling).
pub fn key<'a>(m: &'a Map<String, Value>, name: &str) -> Option<&'a String> {
    m.keys().find(|k| bare(k) == name)
}

/// Pointer of `name` inside the object at `base` ("" for the top).
pub fn ptr(root: &Value, base: &str, name: &str) -> Option<String> {
    let m = if base.is_empty() {
        root.as_object()?
    } else {
        root.pointer(base)?.as_object()?
    };
    key(m, name).map(|k| format!("{base}/{}", crate::tree::esc(k)))
}

/// Every party member slot, in order: (pointer, character id).
pub fn members(root: &Value) -> Vec<(String, i64)> {
    let Some(m) = root.as_object() else {
        return vec![];
    };
    let mut out = vec![];
    for (k, v) in m.iter().filter(|(k, _)| bare(k) == "PlayerMember") {
        let p = format!("/{}", crate::tree::esc(k));
        match v {
            Value::Array(a) => out.extend((0..a.len()).map(|i| format!("{p}/{i}"))),
            Value::Object(_) => out.push(p),
            _ => {}
        }
    }
    out.into_iter()
        .map(|p| {
            let id = ptr(root, &p, "CharacterID")
                .and_then(|q| root.pointer(&q)?.as_i64())
                .unwrap_or(-1);
            (p, id)
        })
        .collect()
}

/// The values of a list of ints (an array, or one property repeated per index).
pub fn ints(root: &Value, base: &str, name: &str) -> Vec<i64> {
    let Some(Value::Object(m)) = root.pointer(base) else {
        return vec![];
    };
    m.iter()
        .filter(|(k, _)| bare(k) == name)
        .flat_map(|(_, v)| match v {
            Value::Array(a) => a.iter().filter_map(Value::as_i64).collect::<Vec<_>>(),
            v => v.as_i64().into_iter().collect(),
        })
        .collect()
}

pub fn character(kind: &str, id: i64) -> Option<&'static str> {
    let names = if kind == OT1 {
        &OT1_CHARACTERS
    } else {
        &OT2_CHARACTERS
    };
    usize::try_from(id - 1)
        .ok()
        .and_then(|i| names.get(i))
        .copied()
}

pub fn job(id: i64) -> String {
    match usize::try_from(id).ok().and_then(|i| JOBS.get(i)) {
        Some(j) => j.to_string(),
        None if id < 0 => "None".into(),
        None => format!("Job {id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs(s: &str) -> Vec<u8> {
        let mut v = (s.len() as i32 + 1).to_le_bytes().to_vec();
        v.extend(s.as_bytes());
        v.push(0);
        v
    }
    fn int(name: &str, v: i32) -> Vec<u8> {
        let mut o = fs(name);
        o.extend(fs("IntProperty"));
        o.extend(4i32.to_le_bytes());
        o.extend(0i32.to_le_bytes());
        o.push(0);
        o.extend(v.to_le_bytes());
        o
    }
    fn strukt(name: &str, ty: &str, body: Vec<u8>) -> Vec<u8> {
        let mut body = body;
        body.extend(fs("None"));
        let mut o = fs(name);
        o.extend(fs("StructProperty"));
        o.extend((body.len() as i32).to_le_bytes());
        o.extend(0i32.to_le_bytes());
        o.extend(fs(ty));
        o.extend([0u8; 16]);
        o.push(0);
        o.extend(body);
        o
    }

    /// A synthetic save shaped like either game's: money, the party members (an array of
    /// structs) and the front row. The first game suffixes the names inside the structs.
    pub fn sample(first_game: bool) -> Vec<u8> {
        let n = |s: &str, i: u32| {
            if first_game {
                format!("{s}_{i}_{:032X}", u128::from(i) * 0x1234_5678_9ABC)
            } else {
                s.to_string()
            }
        };
        let mut v = b"GVAS".to_vec();
        v.extend(2i32.to_le_bytes());
        v.extend(522i32.to_le_bytes());
        v.extend([4, 0, 27, 0, 2, 0, 1, 2, 3, 4]);
        v.extend(fs("++UE4+Release-4.27"));
        v.extend(3i32.to_le_bytes());
        v.extend(0i32.to_le_bytes());
        v.extend(fs("/Game/System/BP/SaveLoad/KSSaveGameBP.KSSaveGameBP_C"));
        v.extend(strukt(
            "PlayerBackpack",
            "KSPlayerBackpack",
            int(&n("Money", 2), 1500),
        ));
        // PlayerMember: an array of three structs.
        let mut elems = vec![];
        for i in 0..3 {
            let mut m = int(&n("CharacterID", 6), i + 1);
            m.extend(int(&n("Level", 7), 10 + i));
            m.extend(int(&n("Exp", 63), 500 + i));
            m.extend(int(&n("FirstJobID", 90), [2, 3, 7][i as usize]));
            m.extend(int(&n("JobPoint", 119), 40));
            m.extend(fs("None"));
            elems.extend(m);
        }
        let mut arr = 3i32.to_le_bytes().to_vec();
        arr.extend(fs("PlayerMember"));
        arr.extend(fs("StructProperty"));
        arr.extend((elems.len() as i32).to_le_bytes());
        arr.extend(0i32.to_le_bytes());
        arr.extend(fs("KSPlayerMember"));
        arr.extend([0u8; 16]);
        arr.push(0);
        arr.extend(elems);
        v.extend(fs("PlayerMember"));
        v.extend(fs("ArrayProperty"));
        v.extend((arr.len() as i32).to_le_bytes());
        v.extend(0i32.to_le_bytes());
        v.extend(fs("StructProperty"));
        v.push(0);
        v.extend(arr);
        // The front row: a list of member indexes.
        let mut row = 2i32.to_le_bytes().to_vec();
        row.extend(2i32.to_le_bytes());
        row.extend(0i32.to_le_bytes());
        let mut party = fs(&n("MainMemberID", 26));
        party.extend(fs("ArrayProperty"));
        party.extend((row.len() as i32).to_le_bytes());
        party.extend(0i32.to_le_bytes());
        party.extend(fs("IntProperty"));
        party.push(0);
        party.extend(row);
        v.extend(strukt("PlayerParty", "KSPlayerParty", party));
        v.extend(int("GamePlaySecond", 3600));
        v.extend(fs("None"));
        v.extend(0i32.to_le_bytes());
        v
    }

    #[test]
    fn finds_party_in_both_games() {
        for first in [false, true] {
            let d = sample(first);
            assert!(looks(&d));
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.kind_name, if first { OT1 } else { OT2 });
            assert_eq!(doc.to_bytes().unwrap(), d);
            let ms = members(&doc.view);
            assert_eq!(ms.len(), 3);
            assert_eq!(ms[2].1, 3);
            assert_eq!(ints(&doc.view, "/PlayerParty", "MainMemberID"), vec![2, 0]);
            assert_eq!(
                character(&doc.kind_name, ms[0].1),
                Some(if first { "Olberic" } else { "Hikari" })
            );
            let money = ptr(&doc.view, "/PlayerBackpack", "Money").unwrap();
            let level = ptr(&doc.view, &ms[1].0, "Level").unwrap();
            assert_eq!(doc.view.pointer(&level), Some(&Value::from(11)));
            doc.set(&money, Value::from(99_999)).unwrap();
            doc.set(&level, Value::from(50)).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            let again = parse(&out).unwrap();
            assert_eq!(again.view.pointer(&money), Some(&Value::from(99_999)));
            assert_eq!(again.view.pointer(&level), Some(&Value::from(50)));
            let other = ptr(&again.view, &ms[2].0, "Level").unwrap();
            assert_eq!(again.view.pointer(&other), Some(&Value::from(12)));
        }
        assert_eq!(bare("Level_7_2387E0754078BE22EED8E689263C5693"), "Level");
        assert_eq!(bare("PlayerMember (12)"), "PlayerMember");
        assert_eq!(bare("Main_Story"), "Main_Story");
        assert_eq!(job(7), "Apothecary");
    }

    /// Research aid: SAVE_EDITOR_OCTOPATH = saves separated by `|`; each parses, writes back
    /// unchanged, and takes a money and a level edit with nothing else changing.
    #[test]
    #[ignore]
    fn real_saves() {
        for path in std::env::var("SAVE_EDITOR_OCTOPATH").unwrap().split('|') {
            let d = std::fs::read(path).unwrap();
            let mut doc = parse(&d).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(doc.to_bytes().unwrap(), d);
            let before = doc.view.clone();
            let money = ptr(&before, "/PlayerBackpack", "Money").unwrap();
            let ms = members(&before);
            let level = ptr(&before, &ms[0].0, "Level").unwrap();
            let (m, l) = (
                before.pointer(&money).unwrap().as_i64().unwrap(),
                before.pointer(&level).unwrap().as_i64().unwrap(),
            );
            println!("{path}: {} money {m}", doc.kind_name);
            for (p, id) in ms.iter().filter(|m| (1..=8).contains(&m.1)) {
                let g = |n: &str| {
                    ptr(&before, p, n)
                        .and_then(|q| before.pointer(&q).cloned())
                        .unwrap_or(Value::Null)
                };
                println!(
                    "   {:<10} lv {} exp {} jp {} job {}",
                    character(&doc.kind_name, *id).unwrap(),
                    g("Level"),
                    g("Exp"),
                    g("JobPoint"),
                    job(g("FirstJobID").as_i64().unwrap_or(-1))
                );
            }
            doc.set(&money, Value::from(m + 1)).unwrap();
            doc.set(&level, Value::from(l + 1)).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            let mut again = parse(&out).unwrap().view;
            assert_eq!(again.pointer(&money), Some(&Value::from(m + 1)));
            assert_eq!(again.pointer(&level), Some(&Value::from(l + 1)));
            *again.pointer_mut(&money).unwrap() = Value::from(m);
            *again.pointer_mut(&level).unwrap() = Value::from(l);
            assert_eq!(again, before);
        }
    }
}
