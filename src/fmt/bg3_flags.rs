//! Baldur's Gate 3 story flags.
//!
//! The game keeps every global flag it has touched in `Globals.lsf`, under
//! `OsirisVariableHelper/VariableManager/FlagMap/FlagContainerMap`: one `Object` per flag with
//! the flag's GUID (`MapKey`) and a `FlagContainerMap` child whose `Flags` byte is 1 (set) or 0
//! (cleared). The story scripts keep their own list of the set flags, the Osiris database
//! `DB_GlobalFlag` in `StorySave.bin`, one fact per flag as "Name_GUID". That list is also
//! where the flags get their names, so a flag named only there drops out of the list once it
//! is cleared and the save is opened again.
//!
//! An edit flips the byte in `Globals.lsf`; when the package is written, `DB_GlobalFlag` is
//! brought in step: a cleared flag's fact is taken out and a set flag gets a fact (copied from
//! an existing one, with the new name). Checked on public sample saves: every flag in
//! `DB_GlobalFlag` is set in the flag map, and no cleared flag is in it.

use super::fields::{Builder, Kind};
use super::lsf::Lsf;
use super::osiris::{self, Val};
use std::collections::{HashMap, HashSet};

pub const FLAG_MAP: [&str; 5] = ["OsirisVariableHelper", "VariableManager", "FlagMap", "FlagContainerMap", "Object"];

/// Each flag in the flag map: GUID text and where its on/off byte is.
pub fn flag_map(l: &Lsf, plain: &[u8]) -> Vec<(String, usize)> {
    let mut out = vec![];
    for o in l.at_path(&FLAG_MAP) {
        let Some(k) = l.attr(o, "MapKey").filter(|a| a.len == 16 && a.at + 16 <= plain.len()) else { continue };
        let Some(c) = l.child(o, "FlagContainerMap") else { continue };
        let Some(f) = l.attr(c, "Flags").filter(|a| a.ty == 19 && a.len == 1) else { continue };
        out.push((super::larian::guid_text(&plain[k.at..k.at + 16]), f.at));
    }
    out
}

/// "Name_GUID" by GUID, from every string in the story's databases.
pub fn names(story: &osiris::Story) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for d in &story.databases {
        for f in &d.facts {
            for v in &f.values {
                if let Val::Str(s) = v {
                    if let Some(g) = osiris::guid_of(s) {
                        out.entry(g.to_ascii_lowercase()).or_insert_with(|| s.clone());
                    }
                }
            }
        }
    }
    out
}

/// Where a flag is listed: its area or companion, from the name's prefix.
pub fn section(name: &str) -> String {
    let mut parts = name.split('_');
    let first = parts.next().unwrap_or_default();
    let area = match first {
        "ORI" => return format!("Companion: {}", parts.next().unwrap_or("other")),
        "GLO" => "General",
        "CAMP" => "Camp",
        "TUT" => "Nautiloid",
        "DEN" => "Emerald Grove",
        "GOB" => "Goblin camp",
        "UND" => "Underdark",
        "SCL" => "Shadow-Cursed Lands",
        "MOO" => "Moonrise Towers",
        "LOW" => "Lower City",
        "CTY" => "Baldur's Gate",
        "WYR" => "Wyrm's Crossing",
        _ => return format!("Other ({first})"),
    };
    area.to_string()
}

/// Adds the named flags to the view as on/off values under "Story flags/<section>/<name>".
pub fn add(b: &mut Builder, l: &Lsf, plain: &[u8], base: usize, story: &osiris::Story) -> usize {
    let names = names(story);
    let mut n = 0;
    let mut seen = HashSet::new();
    for (g, at) in flag_map(l, plain) {
        let Some(full) = names.get(&g) else { continue };
        let name = osiris::strip_guid(full).to_string();
        let key = if seen.insert(name.clone()) { name.clone() } else { format!("{name} ({})", &g[..8]) };
        if b.bits(&["Story flags", &section(&name), &key], base + at, Kind::U8, 0, 1, 0, 0, 1).is_some() {
            n += 1;
        }
    }
    n
}

/// The story with `DB_GlobalFlag` brought in step with the flag map edits between `old` and
/// `new` (the unpacked `Globals.lsf` before and after). `None` when nothing needs changing.
pub fn sync_story(old: &[u8], new: &[u8], story_bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let (Some(lo), Some(ln)) = (Lsf::read(old), Lsf::read(new)) else { return Ok(None) };
    let before: HashMap<String, usize> = flag_map(&lo, old).into_iter().collect();
    let mut set = vec![];
    let mut cleared = HashSet::new();
    for (g, at) in flag_map(&ln, new) {
        let Some(&was_at) = before.get(&g) else { continue };
        let (was, now) = (old.get(was_at).copied().unwrap_or(0) != 0, new.get(at).copied().unwrap_or(0) != 0);
        if was != now {
            if now {
                set.push(g);
            } else {
                cleared.insert(g);
            }
        }
    }
    if set.is_empty() && cleared.is_empty() {
        return Ok(None);
    }
    let story = osiris::read(story_bytes).ok_or("the story file doesn't read")?;
    let db = story.db("DB_GlobalFlag").ok_or("the story has no global flag list")?;
    let names = names(&story);
    let have: HashSet<String> = db.facts.iter().filter_map(|f| match f.values.first() {
        Some(Val::Str(s)) => osiris::guid_of(s).map(str::to_ascii_lowercase),
        _ => None,
    }).collect();
    // A fact to copy: column count 1, the value's header ending in has-value 1, then the
    // scrambled name ending in 0xAD. Only the name is replaced.
    let template = db.facts.iter().find_map(|f| {
        let Some(Val::Str(s)) = f.values.first() else { return None };
        let b = story_bytes.get(f.start..f.end)?;
        let head = b.len().checked_sub(s.len() + 1)?;
        let ok = f.values.len() == 1 && s.is_ascii() && b[0] == 1 && head >= 2 && b[head - 1] == 1 && b[..head].contains(&b'0') && b[b.len() - 1] == 0xAD;
        ok.then(|| b[..head].to_vec())
    }).ok_or("no flag in the story's list to copy the layout from")?;
    let mut drop: Vec<(usize, usize)> = db.facts.iter().filter(|f| match f.values.first() {
        Some(Val::Str(s)) => osiris::guid_of(s).is_some_and(|g| cleared.contains(&g.to_ascii_lowercase())),
        _ => false,
    }).map(|f| (f.start, f.end)).collect();
    let mut add = vec![];
    for g in &set {
        if have.contains(g) {
            continue;
        }
        let full = names.get(g).ok_or_else(|| format!("flag {g} has no name in the story"))?;
        add.extend(&template);
        add.extend(full.bytes().map(|c| c ^ 0xAD));
        add.push(0xAD);
    }
    let added = set.iter().filter(|g| !have.contains(*g)).count();
    let count = db.facts.len() - drop.len() + added;
    let insert_at = db.facts.last().map_or(db.count_at + 4, |f| f.end);
    drop.sort();
    let mut out = Vec::with_capacity(story_bytes.len() + add.len());
    let mut p = 0;
    for (s, e) in &drop {
        out.extend(&story_bytes[p..*s]);
        p = *e;
        if p > insert_at {
            return Err("unexpected story layout".into());
        }
    }
    out.extend(&story_bytes[p..insert_at]);
    out.extend(&add);
    out.extend(&story_bytes[insert_at..]);
    // The count sits before every fact, so nothing before it moved.
    out[db.count_at..db.count_at + 4].copy_from_slice(&(count as u32).to_le_bytes());
    // Read back: the list must hold exactly the expected flags.
    let check = osiris::read(&out).ok_or("the edited story doesn't read back")?;
    let got = check.db("DB_GlobalFlag").map_or(0, |d| d.facts.len());
    if got != count || check.databases.len() != story.databases.len() {
        return Err("the edited story doesn't read back as expected".into());
    }
    Ok(Some(out))
}
