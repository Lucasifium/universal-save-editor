//! Darkest Dungeon (2016) profile files: `persist.*.json` in a `profile_<n>` folder. Despite
//! the name they are binary "DSON" files (format documented by robojumper's
//! DarkestDungeonSaveEditor, MIT; the type rules below follow its `DsonTypes`/`DsonField`).
//!
//! A 64-byte header, a table of objects (Meta1), a table of fields (Meta2: name hash, offset,
//! info bits) and the data block, where each field is its name (NUL-terminated) and then its
//! value. Values carry no type: one byte is a bool (or a character), four aligned bytes an
//! int (or a float, for a few known names), an int length and that many bytes a string, and a
//! string starting with the DSON magic is a whole embedded file (each hero in the roster).
//!
//! Every value is shown; numbers and bools are changed in place (nothing moves, nothing to
//! re-count, no checksum).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use serde_json::{Map, Value};
use std::collections::HashSet;

pub const MAGIC: [u8; 4] = [0x01, 0xB1, 0x00, 0x00];

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 0x40 && d[..4] == MAGIC && u32_at(d, 8) == Some(0x40)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}

/// The game's string hash (field names, and many ids stored as ints).
pub fn name_hash(s: &str) -> u32 {
    s.bytes().fold(0u32, |h, b| h.wrapping_mul(53).wrapping_add(u32::from(b)))
}

// Field names whose type can't be told from the size (from robojumper's DsonTypes). A
// pattern is matched against the field's name and its parents' names, last part first.
const FLOATS: &[&[&str]] = &[&["current_hp"], &["m_Stress"], &["actor", "buff_group", "*", "amount"], &["chapters", "*", "*", "percent"], &["non_rolled_additional_chances", "*", "chance"]];
const INT_VECTORS: &[&[&str]] = &[
    &["read_page_indexes"],
    &["raid_read_page_indexes"],
    &["raid_unread_page_indexes"],
    &["dungeons_unlocked"],
    &["played_video_list"],
    &["trinket_retention_ids"],
    &["last_party_guids"],
    &["dungeon_history"],
    &["buff_group_guids"],
    &["result_event_history"],
    &["dead_hero_entries"],
    &["additional_mash_disabled_infestation_monster_class_ids"],
    &["mash", "valid_additional_mash_entry_indexes"],
    &["party", "heroes"],
    &["skill_cooldown_keys"],
    &["skill_cooldown_values"],
    &["bufferedSpawningSlotsAvailable"],
    &["curioGroups", "*", "curios"],
    &["curioGroups", "*", "curio_table_entries"],
    &["raid_finish_quirk_monster_class_ids"],
    &["narration_audio_event_queue_tags"],
    &["dispatched_events"],
    &["backer_heroes", "*", "combat_skills"],
    &["backer_heroes", "*", "camping_skills"],
    &["backer_heroes", "*", "quirks"],
];
const STRING_VECTORS: &[&[&str]] = &[&["goal_ids"], &["roaming_dungeon_2_ids", "*", "s"], &["quirk_group"], &["backgroundNames"], &["backgroundGroups", "*", "backgrounds"], &["backgroundGroups", "*", "background_table_entries"]];
const FLOAT_ARRAYS: &[&[&str]] = &[&["map", "bounds"], &["areas", "*", "bounds"], &["areas", "*", "tiles", "*", "mappos"], &["areas", "*", "tiles", "*", "sidepos"]];
const TWO_INTS: &[&[&str]] = &[&["killRange"]];

/// `chain` is the field's name, then its parent's, and so on.
fn is_a(table: &[&[&str]], chain: &[&str]) -> bool {
    table.iter().any(|pat| pat.len() <= chain.len() && pat.iter().rev().zip(chain).all(|(p, c)| *p == "*" || p == c))
}

/// Limits for values the game keeps in a known range.
fn range_for(chain: &[&str]) -> Option<(i64, i64)> {
    Some(match chain {
        ["m_Stress", ..] => (0, 200),
        ["current_hp", ..] => (0, 999),
        // Equipment ranks count from 0 (a fresh recruit) to 4 (the fifth level).
        ["weapon_rank" | "armour_rank", ..] => (0, 4),
        ["resolveXp", ..] => (0, i32::MAX as i64),
        ["amount", _, "wallet", ..] => (0, i32::MAX as i64),
        // Trinkets and estate items: how many of each the estate holds.
        ["amount", _, "items", ..] => (1, 999),
        _ => return None,
    })
}

struct Level {
    key: String,
    name: String,
    remaining: usize,
    used: HashSet<String>,
}

fn unique(used: &mut HashSet<String>, name: &str) -> String {
    let mut k = name.to_string();
    let mut n = 2;
    while used.contains(&k) {
        k = format!("{name} ({n})");
        n += 1;
    }
    used.insert(k.clone());
    k
}

/// One DSON file at `base..base + len` of the builder's data; its values go under `prefix`.
fn file(b: &mut Builder, base: usize, len: usize, prefix: &[String], depth: usize) -> Result<(), String> {
    let all: &[u8] = b.d;
    let d = all.get(base..base + len).ok_or("the file ends early")?;
    if !looks(d) {
        return Err("not a Darkest Dungeon file".into());
    }
    let at = |o: usize| u32_at(d, o).unwrap_or(0) as usize;
    let (n1, m1off, n2, m2off, dlen, doff) = (at(0x14), at(0x18), at(0x2C), at(0x30), at(0x38), at(0x3C));
    let fits = |off: usize, n: usize, size: usize| n < 10_000_000 && off.checked_add(n * size).is_some_and(|e| e <= len);
    if !fits(m1off, n1, 16) || !fits(m2off, n2, 12) || !fits(doff, dlen, 1) {
        return Err("the file's tables don't fit in it".into());
    }
    let m1: Vec<usize> = (0..n1).map(|i| at(m1off + 16 * i + 8)).collect(); // direct children
    let m2: Vec<(u32, usize, u32)> = (0..n2).map(|i| (at(m2off + 12 * i) as u32, at(m2off + 12 * i + 4), at(m2off + 12 * i + 8) as u32)).collect();
    let data = &d[doff..doff + dlen];
    let mut offsets: Vec<usize> = m2.iter().map(|e| e.1).collect();
    offsets.sort_unstable();
    let next = |off: usize| offsets.get(offsets.partition_point(|&o| o <= off)).copied().unwrap_or(dlen);
    let mut stack: Vec<Level> = vec![];
    let mut root_used = HashSet::new();
    for &(hash, off, info) in &m2 {
        let nl = ((info >> 2) & 0x1FF) as usize;
        if nl == 0 || off + nl > dlen || data[off + nl - 1] != 0 {
            return Err(format!("bad field name at {off:#x}"));
        }
        let name = String::from_utf8_lossy(&data[off..off + nl - 1]).into_owned();
        if name_hash(&name) != hash {
            return Err(format!("field name {name:?} doesn't match its hash"));
        }
        let key = unique(stack.last_mut().map_or(&mut root_used, |l| &mut l.used), &name);
        let mut path: Vec<String> = prefix.to_vec();
        path.extend(stack.iter().map(|l| l.key.clone()));
        path.push(key.clone());
        let p: Vec<&str> = path.iter().map(String::as_str).collect();
        if info & 1 == 1 {
            let idx = ((info >> 11) & 0xF_FFFF) as usize;
            let children = *m1.get(idx).ok_or("bad object index")?;
            if children > 0 {
                stack.push(Level { key, name, remaining: children, used: HashSet::new() });
                continue;
            }
            b.info(&p, Value::Object(Map::new()));
        } else {
            let start = off + nl;
            let end = next(off).max(start);
            let raw = data.get(start..end).ok_or("bad field size")?;
            let skip = (4 - start % 4) % 4;
            let mut chain: Vec<&str> = vec![name.as_str()];
            chain.extend(stack.iter().rev().map(|l| l.name.as_str()));
            value(b, &p, &chain, raw, base + doff + start, skip, depth);
        }
        // This field is done; so is every object whose last child it was.
        while let Some(top) = stack.last_mut() {
            top.remaining -= 1;
            if top.remaining > 0 {
                break;
            }
            stack.pop();
        }
    }
    if !stack.is_empty() {
        return Err("the file ends inside an object".into());
    }
    Ok(())
}

fn i32s(a: &[u8]) -> Vec<i32> {
    a.chunks_exact(4).map(|c| i32::from_le_bytes(c.try_into().unwrap())).collect()
}

/// One value: `raw` is everything after the name (at `raw_at` in the data); aligned values
/// start `skip` bytes in.
fn value(b: &mut Builder, p: &[&str], chain: &[&str], raw: &[u8], raw_at: usize, skip: usize, depth: usize) {
    let a = raw.get(skip..).unwrap_or(&[]);
    let abs = raw_at + skip;
    if is_a(FLOAT_ARRAYS, chain) {
        let v: Vec<Value> = a.chunks_exact(4).map(|c| crate::tree::f32_value(f32::from_le_bytes(c.try_into().unwrap()))).collect();
        b.info(p, Value::Array(v));
        return;
    }
    if is_a(INT_VECTORS, chain) && a.len() >= 4 {
        let n = i32s(&a[..4])[0];
        if n >= 0 && a.len() == (n as usize + 1) * 4 {
            b.info(p, Value::Array(i32s(&a[4..]).into_iter().map(Value::from).collect()));
            return;
        }
    }
    if is_a(STRING_VECTORS, chain) {
        if let Some(v) = string_vector(a) {
            b.info(p, Value::Array(v.into_iter().map(Value::from).collect()));
            return;
        }
    }
    if is_a(FLOATS, chain) && a.len() == 4 {
        match range_for(chain) {
            Some((lo, hi)) => b.ranged(p, abs, Kind::F32, lo, hi),
            None => b.field(p, abs, Kind::F32),
        };
        return;
    }
    if is_a(TWO_INTS, chain) && a.len() == 8 {
        b.info(p, Value::Array(i32s(a).into_iter().map(Value::from).collect()));
        return;
    }
    if raw.len() == 1 {
        let c = raw[0];
        if (0x20..=0x7E).contains(&c) {
            b.info(p, (c as char).to_string());
        } else {
            b.field(p, raw_at, Kind::B8);
        }
        return;
    }
    if a.len() == 8 && a[0] <= 1 && a[4] <= 1 {
        b.info(p, Value::Array(vec![Value::Bool(a[0] != 0), Value::Bool(a[4] != 0)]));
        return;
    }
    if a.len() == 4 {
        match range_for(chain) {
            Some((lo, hi)) => b.ranged(p, abs, Kind::I32, lo, hi),
            None => b.field(p, abs, Kind::I32),
        };
        return;
    }
    if a.len() >= 5 && i32s(&a[..4])[0] as i64 == a.len() as i64 - 4 {
        let payload = &a[4..];
        if payload.starts_with(&MAGIC) {
            let prefix: Vec<String> = p.iter().map(|s| s.to_string()).collect();
            if depth < 4 && file(b, abs + 4, payload.len(), &prefix, depth + 1).is_ok() {
                return;
            }
            b.info(p, format!("<embedded file, {} bytes>", payload.len()));
            return;
        }
        let text = payload.strip_suffix(&[0]).unwrap_or(payload);
        b.info(p, String::from_utf8_lossy(text).into_owned());
        return;
    }
    let hex: String = a.iter().take(32).map(|x| format!("{x:02x}")).collect();
    b.info(p, format!("<{} bytes: {hex}>", a.len()));
}

/// A count, then strings (each its length and text), aligned to 4 between them.
fn string_vector(a: &[u8]) -> Option<Vec<String>> {
    let n = i32::from_le_bytes(a.get(..4)?.try_into().ok()?);
    let mut pos = 4;
    let mut out = vec![];
    for i in 0..n.max(0) {
        let len = i32::from_le_bytes(a.get(pos..pos + 4)?.try_into().ok()?);
        let len = usize::try_from(len).ok()?;
        let s = a.get(pos + 4..pos + 4 + len)?;
        out.push(String::from_utf8_lossy(s.strip_suffix(&[0]).unwrap_or(s)).into_owned());
        pos += 4 + len;
        if i < n - 1 {
            pos += (4 - (pos - 4) % 4) % 4;
        }
    }
    Some(out)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut b = Builder::new(d);
    file(&mut b, 0, d.len(), &[], 0)?;
    let doc = b.finish("Darkest Dungeon save file", Checksum::None);
    let kind = if doc.view.pointer("/base_root/wallet").is_some() {
        "Darkest Dungeon estate"
    } else if doc.view.pointer("/base_root/heroes").is_some() {
        "Darkest Dungeon roster"
    } else {
        "Darkest Dungeon save file"
    };
    let mut doc = doc;
    doc.kind_name = kind.into();
    Ok(doc)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A field for `dson`: name, then a value of this kind.
    pub enum F {
        Obj(&'static str, Vec<F>),
        Int(&'static str, i32),
        Float(&'static str, f32),
        Bool(&'static str, bool),
        Str(&'static str, String),
        File(&'static str, Vec<u8>),
    }

    /// Builds a DSON file from a field tree (the way the game lays it out).
    pub fn dson(root: Vec<F>) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];
        let mut m1: Vec<[i32; 4]> = vec![];
        let mut m2: Vec<[u32; 3]> = vec![];
        fn put(f: &F, parent: i32, data: &mut Vec<u8>, m1: &mut Vec<[i32; 4]>, m2: &mut Vec<[u32; 3]>) -> usize {
            let name = match f {
                F::Obj(n, _) | F::Int(n, _) | F::Float(n, _) | F::Bool(n, _) | F::Str(n, _) | F::File(n, _) => *n,
            };
            let off = data.len() as u32;
            data.extend(name.as_bytes());
            data.push(0);
            let nl = (name.len() as u32 + 1) << 2;
            let align = |data: &mut Vec<u8>| {
                while data.len() % 4 != 0 {
                    data.push(0);
                }
            };
            let my = m2.len();
            m2.push([name_hash(name), off, nl]);
            match f {
                F::Obj(_, kids) => {
                    let idx = m1.len();
                    m1.push([parent, my as i32, kids.len() as i32, 0]);
                    m2[my][2] |= 1 | ((idx as u32) << 11);
                    let mut all = 0;
                    for k in kids {
                        all += 1 + put(k, idx as i32, data, m1, m2);
                    }
                    m1[idx][3] = all as i32;
                    return all;
                }
                F::Int(_, v) => {
                    align(data);
                    data.extend(v.to_le_bytes());
                }
                F::Float(_, v) => {
                    align(data);
                    data.extend(v.to_le_bytes());
                }
                F::Bool(_, v) => data.push(u8::from(*v)),
                F::Str(_, s) => {
                    align(data);
                    data.extend((s.len() as i32 + 1).to_le_bytes());
                    data.extend(s.as_bytes());
                    data.push(0);
                }
                F::File(_, f) => {
                    align(data);
                    data.extend((f.len() as i32).to_le_bytes());
                    data.extend(f);
                }
            }
            0
        }
        for f in &root {
            put(f, -1, &mut data, &mut m1, &mut m2);
        }
        let m1off = 0x40;
        let m2off = m1off + 16 * m1.len();
        let doff = m2off + 12 * m2.len();
        let mut out = MAGIC.to_vec();
        for v in [0x5E55u32, 0x40, 0, (16 * m1.len()) as u32, m1.len() as u32, m1off as u32, 0, 0, 0, 0, m2.len() as u32, m2off as u32, 0, data.len() as u32, doff as u32] {
            out.extend(v.to_le_bytes());
        }
        for e in &m1 {
            e.iter().for_each(|v| out.extend(v.to_le_bytes()));
        }
        for e in &m2 {
            e.iter().for_each(|v| out.extend(v.to_le_bytes()));
        }
        out.extend(data);
        out
    }

    pub fn estate() -> Vec<u8> {
        let wallet = |i: &'static str, n: i32, t: &str| F::Obj(i, vec![F::Int("amount", n), F::Str("type", t.into())]);
        dson(vec![F::Obj(
            "base_root",
            vec![
                F::Int("version", 34),
                F::Obj("wallet", vec![wallet("0", 1234, "gold"), wallet("1", 5, "bust"), wallet("2", 7, "crest")]),
                F::Obj("trinkets", vec![F::Obj("items", vec![F::Obj("0", vec![F::Str("id", "lucky_dice".into()), F::Str("type", "trinket".into()), F::Int("amount", 1)])])]),
                F::Obj("empty", vec![]),
            ],
        )])
    }

    pub fn roster() -> Vec<u8> {
        let hero = dson(vec![F::Obj(
            "base_root",
            vec![
                F::Obj("actor", vec![F::Str("name", "Testa".into()), F::Float("current_hp", 25.5)]),
                F::Str("heroClass", "vestal".into()),
                F::Int("resolveXp", 3),
                F::Float("m_Stress", 40.0),
                F::Bool("visited_deaths_door", false),
                F::Int("weapon_rank", 1),
                F::Int("armour_rank", 0),
            ],
        )]);
        dson(vec![F::Obj("base_root", vec![F::Obj("heroes", vec![F::Obj("7", vec![F::Obj("hero_file_data", vec![F::File("raw_data", hero)])])]), F::Int("nextGuid", 8)])])
    }

    #[test]
    fn reads_estate_and_edits_gold() {
        let d = estate();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Darkest Dungeon estate");
        assert_eq!(doc.view.pointer("/base_root/wallet/0/amount"), Some(&Value::from(1234)));
        assert_eq!(doc.view.pointer("/base_root/wallet/2/type"), Some(&Value::from("crest")));
        assert_eq!(doc.view.pointer("/base_root/trinkets/items/0/id"), Some(&Value::from("lucky_dice")));
        assert_eq!(doc.view.pointer("/base_root/empty"), Some(&Value::Object(Map::new())));
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert!(doc.set("/base_root/wallet/0/amount", Value::from(-1)).is_err());
        doc.set("/base_root/wallet/0/amount", Value::from(99_999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/base_root/wallet/0/amount"), Some(&Value::from(99_999)));
        assert_eq!(again.view.pointer("/base_root/wallet/1/amount"), Some(&Value::from(5)));
    }

    #[test]
    fn reads_heroes_inside_the_roster() {
        let d = roster();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Darkest Dungeon roster");
        let h = "/base_root/heroes/7/hero_file_data/raw_data/base_root";
        assert_eq!(doc.view.pointer(&format!("{h}/actor/name")), Some(&Value::from("Testa")));
        assert_eq!(doc.view.pointer(&format!("{h}/m_Stress")).and_then(Value::as_f64), Some(40.0));
        assert_eq!(doc.view.pointer(&format!("{h}/visited_deaths_door")), Some(&Value::Bool(false)));
        assert_eq!(doc.view.pointer("/base_root/nextGuid"), Some(&Value::from(8)));
        assert!(doc.set(&format!("{h}/m_Stress"), Value::from(250.0)).is_err());
        assert!(doc.set(&format!("{h}/weapon_rank"), Value::from(5)).is_err());
        doc.set(&format!("{h}/m_Stress"), Value::from(0.0)).unwrap();
        doc.set(&format!("{h}/resolveXp"), Value::from(30)).unwrap();
        doc.set(&format!("{h}/weapon_rank"), Value::from(4)).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer(&format!("{h}/m_Stress")).and_then(Value::as_f64), Some(0.0));
        assert_eq!(again.view.pointer(&format!("{h}/resolveXp")), Some(&Value::from(30)));
        assert_eq!(again.view.pointer(&format!("{h}/weapon_rank")), Some(&Value::from(4)));
        assert_eq!(again.view.pointer(&format!("{h}/actor/current_hp")).and_then(Value::as_f64), Some(25.5));
    }

    #[test]
    fn rejects_a_bad_hash() {
        let mut d = estate();
        let m2off = u32_at(&d, 0x30).unwrap() as usize;
        d[m2off] ^= 1;
        assert!(parse(&d).is_err());
        assert_eq!(name_hash("base_root"), "base_root".bytes().fold(0u32, |h, b| h.wrapping_mul(53).wrapping_add(b as u32)));
    }

    /// Research aid: every file in SAVE_EDITOR_DD1 (a profile folder) parses, writes back
    /// unchanged, and takes an edit with nothing else changing.
    #[test]
    #[ignore]
    fn real_profile() {
        let dir = std::env::var("SAVE_EDITOR_DD1").unwrap();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let mut doc = parse(&d).unwrap_or_else(|err| panic!("{}: {err}", e.path().display()));
            println!("{}: {} ({} editable)", e.file_name().to_string_lossy(), doc.kind_name, doc.editable.len());
            assert_eq!(doc.to_bytes().unwrap(), d);
            let before = doc.view.clone();
            let edit = match doc.kind_name.as_str() {
                "Darkest Dungeon estate" => Some("/base_root/wallet/0/amount".to_string()),
                "Darkest Dungeon roster" => {
                    let heroes = doc.view.pointer("/base_root/heroes").and_then(Value::as_object).unwrap();
                    let first = heroes.keys().next().unwrap();
                    Some(format!("/base_root/heroes/{first}/hero_file_data/raw_data/base_root/resolveXp"))
                }
                _ => None,
            };
            if let Some(p) = edit {
                let n = doc.view.pointer(&p).and_then(Value::as_i64).unwrap();
                doc.set(&p, Value::from(n + 1)).unwrap();
                let out = doc.to_bytes().unwrap();
                assert_eq!(out.len(), d.len());
                let mut again = parse(&out).unwrap().view;
                assert_eq!(again.pointer(&p), Some(&Value::from(n + 1)));
                *again.pointer_mut(&p).unwrap() = Value::from(n);
                assert_eq!(again, before);
                println!("   {p}: {n} -> {}", n + 1);
            }
        }
    }
}
