//! Diablo II (classic 1.10+) and Diablo II: Resurrected characters (`.d2s`).
//!
//! A fixed header (magic AA55AA55, version, file size, checksum), the character block,
//! quests, waypoints and NPC flags, then the attributes ("gf"): a bit stream of
//! (9-bit id, value) pairs ending at id 0x1FF, where stats that are 0 are left out.
//! Then skills ("if" + 30 bytes), then items.
//!
//! Because a stat at 0 isn't written, setting one means writing the stats again (the
//! file grows a little); this backend rebuilds that section from the edited values,
//! then fixes the size and the checksum (rotate left 1, add each byte, with the
//! checksum field zeroed). Layout facts from halbu (feor, MIT) and d2s by dschu012.

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

const MAGIC: u32 = 0xAA55_AA55;
pub const CLASSES: [&str; 8] = ["Amazon", "Sorceress", "Necromancer", "Paladin", "Barbarian", "Druid", "Assassin", "Warlock"];

/// The stats a character file holds: id, label, bit width, 13.8 fixed point, highest value.
const STATS: [(u16, &str, u32, bool, i64); 16] = [
    (0, "Strength", 10, false, 1023),
    (1, "Energy", 10, false, 1023),
    (2, "Dexterity", 10, false, 1023),
    (3, "Vitality", 10, false, 1023),
    (4, "Stat points", 10, false, 1023),
    (5, "Skill points", 8, false, 255),
    (6, "Life", 21, true, 8191),
    (7, "Max life", 21, true, 8191),
    (8, "Mana", 21, true, 8191),
    (9, "Max mana", 21, true, 8191),
    (10, "Stamina", 21, true, 8191),
    (11, "Max stamina", 21, true, 8191),
    (12, "Level", 7, false, 99),
    (13, "Experience", 32, false, u32::MAX as i64),
    (14, "Gold", 25, false, 33_554_431),
    (15, "Stash gold", 25, false, 33_554_431),
];

fn stat(id: u16) -> Option<&'static (u16, &'static str, u32, bool, i64)> {
    STATS.iter().find(|s| s.0 == id)
}

fn skill_names() -> &'static Vec<Vec<String>> {
    static T: OnceLock<Vec<Vec<String>>> = OnceLock::new();
    T.get_or_init(|| {
        include_str!("data/d2_skills.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()).map(|l| l.split('\t').map(str::to_string).collect()).collect()
    })
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Offsets that moved in "Reign of the Warlock" (version 105).
struct Layout {
    status: usize,
    class: usize,
    level: usize,
    stats: usize,
    name: Option<(usize, usize)>,
}

fn layout(d: &[u8]) -> Option<Layout> {
    let v = u32_at(d, 4)?;
    Some(match v {
        96 | 97 => Layout { status: 0x24, class: 0x28, level: 0x2B, stats: 765, name: Some((0x14, 16)) },
        98 | 99 => Layout { status: 0x24, class: 0x28, level: 0x2B, stats: 765, name: Some((0x10B, 48)) },
        105 => Layout { status: 0x14, class: 0x18, level: 0x1B, stats: 833, name: Some((299, 48)) },
        _ => return None,
    })
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 790 && u32_at(d, 0) == Some(MAGIC) && u32_at(d, 8) == Some(d.len() as u32) && layout(d).is_some()
}

pub fn checksum(d: &[u8]) -> u32 {
    let mut c: u32 = 0;
    for (i, &b) in d.iter().enumerate() {
        let b = if (12..16).contains(&i) { 0 } else { b };
        c = c.rotate_left(1).wrapping_add(u32::from(b));
    }
    c
}

/// Reads the stat stream at `at` (just after "gf"): the stats and where the section ends.
fn read_stats(d: &[u8], at: usize) -> Option<(BTreeMap<u16, u32>, usize)> {
    let mut bit = 0usize;
    let mut take = |n: u32| -> Option<u32> {
        let mut v = 0u32;
        for k in 0..n as usize {
            let byte = *d.get(at + (bit + k) / 8)?;
            v |= u32::from((byte >> ((bit + k) % 8)) & 1) << k;
        }
        bit += n as usize;
        Some(v)
    };
    let mut out = BTreeMap::new();
    loop {
        let id = take(9)? as u16;
        if id == 0x1FF {
            break;
        }
        let (_, _, w, ..) = stat(id)?;
        out.insert(id, take(*w)?);
        if out.len() > 64 {
            return None;
        }
    }
    Some((out, at + bit.div_ceil(8)))
}

fn write_stats(stats: &BTreeMap<u16, u32>) -> Vec<u8> {
    let mut bits: Vec<bool> = vec![];
    let mut put = |v: u32, n: u32| {
        for k in 0..n {
            bits.push((v >> k) & 1 == 1);
        }
    };
    for (&id, &v) in stats {
        if v == 0 {
            continue;
        }
        put(u32::from(id), 9);
        put(v, stat(id).map_or(32, |s| s.2));
    }
    put(0x1FF, 9);
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, b) in bits.iter().enumerate() {
        if *b {
            out[i / 8] |= 1 << (i % 8);
        }
    }
    out
}

#[derive(Clone, Debug)]
enum Target {
    Stat(u16),
    Skill(usize),
}

#[derive(Clone, Debug)]
struct D2Backend {
    data: Arc<Vec<u8>>,
    targets: Vec<Target>,
}

fn as_int(v: &Value) -> Result<i64, String> {
    v.as_i64().or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64)).ok_or_else(|| "expected a whole number".to_string())
}

impl D2Backend {
    fn range(&self, t: &Target) -> (i64, i64) {
        match t {
            Target::Stat(12) => (1, 99),
            Target::Stat(id) => (0, stat(*id).map_or(0, |s| s.4)),
            Target::Skill(_) => (0, 99),
        }
    }
}

impl Backend for D2Backend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let d = self.data.as_ref();
        if changes.is_empty() {
            return Ok(d.clone());
        }
        let lay = layout(d).ok_or("unknown file version")?;
        let start = lay.stats + 2;
        let (mut stats, end) = read_stats(d, start).ok_or("the attributes couldn't be read")?;
        let mut skills: Vec<(usize, u8)> = vec![];
        for (leaf, v) in changes {
            let t = &self.targets[*leaf];
            let n = as_int(v)?;
            let (lo, hi) = self.range(t);
            if n < lo || n > hi {
                return Err(format!("must be between {lo} and {hi}"));
            }
            match t {
                Target::Stat(id) => {
                    let fixed = stat(*id).is_some_and(|s| s.3);
                    let old = stats.get(id).copied().unwrap_or(0);
                    // Life and mana keep their fraction (the low 8 bits).
                    let raw = if fixed { ((n as u32) << 8) | (old & 0xFF) } else { n as u32 };
                    stats.insert(*id, if n == 0 { 0 } else { raw });
                }
                Target::Skill(i) => skills.push((*i, n as u8)),
            }
        }
        let mut out = d[..start].to_vec();
        out.extend(write_stats(&stats));
        let shift = out.len() as isize - end as isize;
        out.extend_from_slice(&d[end..]);
        // Skills: "if" right after the attributes, then one byte per skill.
        let skills_at = (end as isize + shift) as usize;
        if out.get(skills_at..skills_at + 2) != Some(&b"if"[..]) {
            return Err("the skills section wasn't where it should be".into());
        }
        for (i, n) in skills {
            out[skills_at + 2 + i] = n;
        }
        if let Some(&lv) = stats.get(&12) {
            out[lay.level] = lv as u8;
        }
        let len = out.len() as u32;
        out[8..12].copy_from_slice(&len.to_le_bytes());
        let c = checksum(&out);
        out[12..16].copy_from_slice(&c.to_le_bytes());
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let n = as_int(v)?;
        let (lo, hi) = self.range(&self.targets[leaf]);
        if n < lo || n > hi {
            return Err(format!("must be between {lo} and {hi}"));
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Diablo II character".into());
    }
    if u32_at(d, 12) != Some(checksum(d)) {
        return Err("the character's checksum doesn't match".into());
    }
    let lay = layout(d).unwrap();
    if d.get(lay.stats..lay.stats + 2) != Some(&b"gf"[..]) {
        return Err("no attributes section".into());
    }
    let (stats, end) = read_stats(d, lay.stats + 2).ok_or("the attributes couldn't be read")?;
    if d.get(end..end + 2) != Some(&b"if"[..]) {
        return Err("no skills section".into());
    }
    let mut view = Map::new();
    let mut editable = HashMap::new();
    let mut targets = vec![];
    let mut add = |view: &mut Map<String, Value>, group: &str, key: &str, v: Value, t: Target| {
        editable.insert(format!("/{}/{}", crate::json_view::pointer_escape(group), crate::json_view::pointer_escape(key)), targets.len());
        targets.push(t);
        view.entry(group.to_string()).or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap().insert(key.to_string(), v);
    };
    let mut info = Map::new();
    let class = d[lay.class] as usize;
    if let Some((at, n)) = lay.name {
        let raw = &d[at..at + n];
        let name = String::from_utf8_lossy(&raw[..raw.iter().position(|&c| c == 0).unwrap_or(n)]).into_owned();
        info.insert("Name".into(), name.into());
    }
    info.insert("Class".into(), CLASSES.get(class).copied().unwrap_or("?").into());
    let status = d[lay.status];
    info.insert("Hardcore".into(), (status & 0x04 != 0).into());
    info.insert("Expansion".into(), (status & 0x20 != 0).into());
    info.insert("Version".into(), u32_at(d, 4).unwrap().into());
    view.insert("Character".into(), Value::Object(info));
    for &(id, label, _, fixed, _) in &STATS {
        let raw = stats.get(&id).copied().unwrap_or(0);
        let shown = if fixed { raw >> 8 } else { raw };
        let group = match id {
            0..=5 => "Attributes",
            6..=11 => "Life and mana",
            _ => "Progress",
        };
        add(&mut view, group, label, shown.into(), Target::Stat(id));
    }
    let names = skill_names();
    for i in 0..30 {
        let label = names.get(class).and_then(|c| c.get(i)).cloned().unwrap_or_else(|| format!("Skill {}", i + 1));
        add(&mut view, "Skills", &label, d[end + 2 + i].into(), Target::Skill(i));
    }
    let backend = D2Backend { data: Arc::new(d.to_vec()), targets };
    Ok(TreeDoc::new(Value::Object(view), editable, "Diablo II character", Box::new(backend))
        .with_note("Level, experience, gold, attributes, unspent points and skill points; the file's size and checksum are fixed when you save. Gold can be at most 10,000 × level on the character."))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic v99 character: header, empty blocks, stats, skills, an empty item list.
    pub fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 765];
        d[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        d[4..8].copy_from_slice(&99u32.to_le_bytes());
        d[0x28] = 1; // Sorceress
        d[0x2B] = 10;
        d[0x10B..0x10B + 4].copy_from_slice(b"Test");
        d.extend(b"gf");
        let mut s = BTreeMap::new();
        s.insert(0, 25);
        s.insert(1, 35);
        s.insert(6, 80 << 8);
        s.insert(12, 10);
        s.insert(13, 5000);
        d.extend(write_stats(&s));
        d.extend(b"if");
        let mut sk = [0u8; 30];
        sk[0] = 3;
        d.extend(sk);
        d.extend(b"JM\0\0");
        let n = d.len() as u32;
        d[8..12].copy_from_slice(&n.to_le_bytes());
        let c = checksum(&d);
        d[12..16].copy_from_slice(&c.to_le_bytes());
        d
    }

    #[test]
    fn reads_and_adds_gold() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/Class"), Some(&"Sorceress".into()));
        assert_eq!(doc.view.pointer("/Attributes/Energy"), Some(&35.into()));
        assert_eq!(doc.view.pointer("/Life and mana/Life"), Some(&80.into()));
        assert_eq!(doc.view.pointer("/Progress/Gold"), Some(&0.into()));
        assert_eq!(doc.view.pointer("/Skills/Fire Bolt"), Some(&3.into()));
        assert_eq!(doc.to_bytes().unwrap(), d);
        // Gold wasn't written (it was 0): adding it makes the file longer.
        doc.set("/Progress/Gold", 4321.into()).unwrap();
        doc.set("/Progress/Level", 12.into()).unwrap();
        doc.set("/Skills/Warmth", 5.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(out.len() > d.len());
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Progress/Gold"), Some(&4321.into()));
        assert_eq!(again.view.pointer("/Progress/Level"), Some(&12.into()));
        assert_eq!(again.view.pointer("/Skills/Warmth"), Some(&5.into()));
        assert_eq!(out[0x2B], 12);
    }

    /// Research aid: SAVE_EDITOR_D2 = a folder of .d2s files.
    #[test]
    #[ignore]
    fn real_saves() {
        fn walk(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(p).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "d2s") {
                    out.push(p);
                }
            }
        }
        let mut files = vec![];
        walk(std::path::Path::new(&std::env::var("SAVE_EDITOR_D2").unwrap()), &mut files);
        for f in files {
            let d = std::fs::read(&f).unwrap();
            match parse(&d) {
                Ok(mut doc) => {
                    let v = &doc.view;
                    println!("{}: {} {} lvl {} gold {} xp {}", f.display(), v["Character"]["Name"], v["Character"]["Class"], v["Progress"]["Level"], v["Progress"]["Gold"], v["Progress"]["Experience"]);
                    assert_eq!(doc.to_bytes().unwrap(), d);
                    // An edit (gold + 1, which is new on fresh characters) reads back.
                    let g = doc.view.pointer("/Progress/Gold").and_then(Value::as_i64).unwrap();
                    doc.set("/Progress/Gold", (g + 1).into()).unwrap();
                    let out = doc.to_bytes().unwrap();
                    let again = parse(&out).unwrap();
                    assert_eq!(again.view.pointer("/Progress/Gold").and_then(Value::as_i64), Some(g + 1), "{}", f.display());
                    assert_eq!(again.view.pointer("/Progress/Experience"), doc.view.pointer("/Progress/Experience"));
                    let after = |x: &[u8]| {
                        let l = layout(x).unwrap();
                        let (_, e) = read_stats(x, l.stats + 2).unwrap();
                        x[e..].to_vec()
                    };
                    assert_eq!(after(&out), after(&d), "skills and items unchanged");
                }
                Err(e) => println!("{}: ERR {e}", f.display()),
            }
        }
    }
}
