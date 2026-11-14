//! Generation Zero `savegame`: an Avalanche ADF file (version 4) whose type definitions travel
//! with the data. One instance (`SaveGameRoot`) points to the characters, the profile and the
//! world. The type table is walked here to find where each struct's members live, so a game
//! update that moves members keeps working. Offsets of arrays and pointers count from the start
//! of the instance data; an array is (offset, 4 more bytes, count) in 16 bytes.
//!
//! Layout and item hashes follow GrimChan/GenZ-Save-File-Editor (MIT). Nothing is checksummed.

use super::fields::{Builder, Checksum, Kind};
use super::genzero_names::ITEMS;
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "Generation Zero save";

fn u32at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}
fn u64at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

struct Struct {
    size: usize,
    members: Vec<(String, usize)>,
}

struct Adf {
    /// Start of the instance data (pointers and arrays count from here).
    base: usize,
    root_type: u32,
    types: HashMap<String, Struct>,
    root_hash: HashMap<u32, String>,
}

fn read_adf(d: &[u8]) -> Option<Adf> {
    if d.get(0..4)? != b" FDA" || u32at(d, 4)? != 4 {
        return None;
    }
    let (ic, io, tc, to, nc, no) = (u32at(d, 8)?, u32at(d, 12)?, u32at(d, 16)?, u32at(d, 20)?, u32at(d, 32)?, u32at(d, 36)?);
    if ic != 1 || tc == 0 || tc > 4096 || nc > 8192 {
        return None;
    }
    let (n0, nc) = (no as usize, nc as usize);
    let lens = d.get(n0..n0 + nc)?;
    let mut p = n0 + nc;
    let mut names = vec![];
    for &l in lens {
        names.push(String::from_utf8_lossy(d.get(p..p + l as usize)?).to_string());
        p += l as usize + 1;
    }
    let inst = io as usize;
    let root_type = u32at(d, inst + 4)?;
    let base = u32at(d, inst + 8)? as usize;
    let mut types = HashMap::new();
    let mut root_hash = HashMap::new();
    let mut p = to as usize;
    for _ in 0..tc {
        let (ty, size, hash, name_ix) = (u32at(d, p)?, u32at(d, p + 4)? as usize, u32at(d, p + 12)?, u64at(d, p + 16)? as usize);
        p += 36;
        match ty {
            1 => {
                let mc = u32at(d, p)? as usize;
                p += 4;
                let mut members = vec![];
                for _ in 0..mc {
                    let (ni, off) = (u64at(d, p)? as usize, u32at(d, p + 16)? as usize & 0xFF_FFFF);
                    members.push((names.get(ni)?.clone(), off));
                    p += 32;
                }
                let name = names.get(name_ix)?.clone();
                root_hash.insert(hash, name.clone());
                types.insert(name, Struct { size, members });
            }
            // An enumeration lists (name, value) pairs of 12 bytes.
            8 => p += 4 + u32at(d, p)? as usize * 12,
            // Arrays, bit fields and the other kinds carry one more 4-byte field.
            _ => p += 4,
        }
    }
    Some(Adf { base, root_type, types, root_hash })
}

impl Adf {
    /// The struct named `name`, or the same name followed by a version number.
    fn find(&self, name: &str) -> Option<&Struct> {
        self.types.get(name).or_else(|| {
            let mut best: Option<(&String, &Struct)> = None;
            for (n, s) in &self.types {
                if n.strip_prefix(name).is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())) && best.is_none_or(|(b, _)| n < b) {
                    best = Some((n, s));
                }
            }
            best.map(|(_, s)| s)
        })
    }
    fn member(&self, st: &str, member: &str) -> Option<usize> {
        self.find(st)?.members.iter().find(|(n, _)| n == member).map(|(_, o)| *o)
    }
    fn size(&self, st: &str) -> Option<usize> {
        self.find(st).map(|s| s.size)
    }
}

pub fn looks(d: &[u8]) -> bool {
    read_adf(d).is_some_and(|a| a.root_hash.get(&a.root_type).is_some_and(|n| n == "SaveGameRoot") && a.find("CharacterProfile").is_some() && a.find("OwnedEquipment").is_some())
}

/// Where an array's elements start, and how many there are.
fn array(d: &[u8], base: usize, at: usize) -> Option<(usize, usize)> {
    let off = u32at(d, at)? as usize;
    let n = u64at(d, at + 8)? as usize;
    (n <= 1_000_000).then_some((base + off, n))
}

fn item_name(hash: u32) -> Option<&'static str> {
    ITEMS.binary_search_by_key(&hash, |(h, _)| *h).ok().map(|i| ITEMS[i].1)
}

/// The name an item is listed under, made unique inside one list.
fn unique(seen: &mut HashMap<String, u32>, hash: u32) -> String {
    let base = item_name(hash).map_or_else(|| format!("Item {hash}"), str::to_string);
    let n = seen.entry(base.clone()).or_insert(0);
    *n += 1;
    if *n == 1 {
        base
    } else {
        format!("{base} ({n})")
    }
}

/// Items of an `OwnedEquipment` array as records with their amount.
fn items(b: &mut Builder, a: &Adf, d: &[u8], path: &[&str], at: usize) -> Option<()> {
    let (start, n) = array(d, a.base, at)?;
    let size = a.size("OwnedEquipment")?;
    let (hash_at, amount_at) = (a.member("OwnedEquipment", "EquipmentHash")?, a.member("OwnedEquipment", "Amount")?);
    let mut seen = HashMap::new();
    for i in 0..n {
        let o = start + i * size;
        let key = unique(&mut seen, u32at(d, o + hash_at)?);
        let mut p = path.to_vec();
        p.push(&key);
        p.push("Amount");
        b.ranged(&p, o + amount_at, Kind::U32, 0, 999_999);
    }
    Some(())
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let a = read_adf(d).ok_or("not a Generation Zero save")?;
    if !looks(d) {
        return Err("not a Generation Zero save".into());
    }
    let bad = || "unexpected Generation Zero save layout".to_string();
    let follow = |at: usize| -> Option<usize> { Some(a.base + u32at(d, a.base + at)? as usize) };
    let root_at = a.member("SaveGameRoot", "SaveData").ok_or_else(bad)?;
    let data = follow(root_at).ok_or_else(bad)?;
    let rel = |st: &str, m: &str| a.member(st, m).map(|o| data + o).ok_or_else(bad);
    let chars_at = follow(rel("SaveGameData1", "Characters")? - a.base).ok_or_else(bad)?;
    let profile_at = follow(rel("SaveGameData1", "Profile")? - a.base).ok_or_else(bad)?;

    let mut b = Builder::new(d);
    // Characters: four inline profiles.
    let first = chars_at + a.member("CharactersSaveData", "Characters").ok_or_else(bad)?;
    let psize = a.size("CharacterProfile").ok_or_else(bad)?;
    let m = |name: &str| a.member("CharacterProfile", name).ok_or_else(bad);
    let (sp, xp, lv, cr, inv) = (m("SkillPoints")?, m("Xp")?, m("Level")?, m("Created")?, m("Inventory")?);
    for i in 0..4 {
        let o = first + i * psize;
        if o + psize > d.len() {
            break;
        }
        let used = d[o + cr] != 0 || u32at(d, o + xp).unwrap_or(0) > 0;
        if !used {
            continue;
        }
        let name = format!("Character {}", i + 1);
        b.ranged(&[&name, "Skill points"], o + sp, Kind::U32, 0, 9_999);
        b.ranged(&[&name, "Experience"], o + xp, Kind::U32, 0, 99_999_999);
        b.ranged(&[&name, "Level"], o + lv, Kind::U16, 1, 99);
        items(&mut b, &a, d, &[&name, "Inventory"], o + inv).ok_or_else(bad)?;
    }
    // Profile: the crafting resource store and the safehouse storage.
    let pm = |name: &str| a.member("ProfileSaveData", name).map(|o| profile_at + o);
    if let Some(at) = pm("ResourceStorage") {
        let (start, n) = array(d, a.base, at).ok_or_else(bad)?;
        let size = a.size("ResourceStorageItem").unwrap_or(8);
        let (ni, ai) = (a.member("ResourceStorageItem", "Name").unwrap_or(0), a.member("ResourceStorageItem", "Amount").unwrap_or(4));
        for i in 0..n {
            let o = start + i * size;
            let name = item_name(u32at(d, o + ni).ok_or_else(bad)?).map_or_else(|| format!("Resource {}", u32at(d, o + ni).unwrap_or(0)), str::to_string);
            b.ranged(&["Resource storage", &name], o + ai, Kind::U32, 0, 999_999);
        }
    }
    if let Some(at) = pm("SafehouseStorage") {
        items(&mut b, &a, d, &["Safehouse storage"], at).ok_or_else(bad)?;
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Items are listed by their game id; quality is shown as Q1 to Q6. Only the amounts are edited."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_sorted_for_lookup() {
        assert!(ITEMS.windows(2).all(|w| w[0].0 <= w[1].0));
        assert_eq!(item_name(1065120515), Some("Thread"));
    }

    #[test]
    fn duplicate_names_get_a_number() {
        let mut seen = HashMap::new();
        assert_eq!(unique(&mut seen, 1065120515), "Thread");
        assert_eq!(unique(&mut seen, 1065120515), "Thread (2)");
    }
}
