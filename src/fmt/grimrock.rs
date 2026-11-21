//! Legend of Grimrock (2012) saves: `savegameNN.sav`, the bytes `GRIM`, a version, the unpacked
//! size and one zlib stream. The unpacked data is a list of chunks (a four-letter tag, a 32-bit
//! length, the data). Each `CHAR` chunk is one party member: a champion record whose named
//! blocks (`STAT` current stats, `BSTA` base stats, `COND` conditions, `SKIL` skill levels)
//! hold the values as 64-bit floats. Values are changed in place and no length changes, so
//! the chunk list stays valid. The layout was worked out from public saves (no editor or
//! documentation of it is known).

use crate::tree::{Backend, TreeDoc};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;

pub const KIND: &str = "Legend of Grimrock save";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    version: u32,
    original: Arc<Vec<u8>>,
    file: Arc<Vec<u8>>,
}

pub fn maybe(d: &[u8]) -> bool {
    d.len() > 64 && d.starts_with(b"GRIM")
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn f64_at(d: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !maybe(d) {
        return None;
    }
    let version = u32_at(d, 4)?;
    let size = u32_at(d, 8)? as usize;
    if size > 400 << 20 {
        return None;
    }
    let mut inner = Vec::with_capacity(size);
    ZlibDecoder::new(&d[12..]).take(size as u64 + 1).read_to_end(&mut inner).ok()?;
    if inner.len() != size || !inner.starts_with(b"DESC") {
        return None;
    }
    Some((Packed { version, original: Arc::new(inner.clone()), file: Arc::new(d.to_vec()) }, inner))
}

pub fn close(p: &Packed, data: &[u8]) -> Result<Vec<u8>, String> {
    if data == p.original.as_slice() {
        return Ok((*p.file).clone());
    }
    let mut out = Vec::with_capacity(data.len() / 3);
    out.extend_from_slice(b"GRIM");
    out.extend_from_slice(&p.version.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    let mut z = ZlibEncoder::new(out, Compression::default());
    z.write_all(data).map_err(|e| e.to_string())?;
    z.finish().map_err(|e| e.to_string())
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"DESC") && chunks(d).is_some_and(|c| c.iter().any(|c| &c.tag == b"CHAR"))
}

struct Chunk {
    tag: [u8; 4],
    /// Offset of the chunk's data and its length.
    at: usize,
    len: usize,
}

/// Every chunk, or None when the list does not add up exactly.
fn chunks(d: &[u8]) -> Option<Vec<Chunk>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < d.len() {
        let tag: [u8; 4] = d.get(i..i + 4)?.try_into().ok()?;
        if !tag.iter().all(u8::is_ascii_uppercase) {
            return None;
        }
        let len = u32_at(d, i + 4)? as usize;
        let end = (i + 8).checked_add(len)?;
        if end > d.len() {
            return None;
        }
        out.push(Chunk { tag, at: i + 8, len });
        i = end;
    }
    Some(out)
}

fn string_at(d: &[u8], at: usize) -> Option<(String, usize)> {
    if u32_at(d, at)? != 0 {
        return None;
    }
    let n = u32_at(d, at + 4)? as usize;
    if n > 200 {
        return None;
    }
    let s = String::from_utf8_lossy(d.get(at + 8..at + 8 + n)?).into_owned();
    Some((s, at + 8 + n))
}

/// A plain length-prefixed string (no type tag).
fn plain_at(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = u32_at(d, at)? as usize;
    if n > 200 {
        return None;
    }
    let s = String::from_utf8_lossy(d.get(at + 4..at + 4 + n)?).into_owned();
    Some((s, at + 4 + n))
}

fn number_at(d: &[u8], at: usize) -> Option<(f64, usize)> {
    if u32_at(d, at)? != 1 {
        return None;
    }
    let v = f64_at(d, at + 4)?;
    v.is_finite().then_some((v, at + 12))
}

/// A named block's values: `[version 0][name][(1, f64)…]`, returning the name and the offset of each number.
fn block(d: &[u8], at: usize, len: usize) -> Option<(String, Vec<usize>)> {
    if u32_at(d, at)? != 0 {
        return None;
    }
    let (name, mut i) = plain_at(d, at + 4)?;
    let mut nums = Vec::new();
    while i + 12 <= at + len {
        let (_, next) = number_at(d, i)?;
        nums.push(i + 4);
        i = next;
    }
    (i == at + len).then_some((name, nums))
}

struct Champion {
    name: String,
    race: String,
    class: String,
    level: f64,
    experience: f64,
    /// Named blocks by tag and name: where their numbers are.
    blocks: HashMap<(String, String), Vec<usize>>,
    /// The order skills were stored in.
    skills: Vec<String>,
}

fn champion(d: &[u8], c: &Chunk) -> Option<Champion> {
    // A number (the party slot), then the CHAM record.
    let (_, i) = number_at(d, c.at)?;
    if d.get(i..i + 4)? != b"CHAM" {
        return None;
    }
    let len = u32_at(d, i + 4)? as usize;
    let (start, end) = (i + 8, i + 8 + len);
    if end > c.at + c.len {
        return None;
    }
    if u32_at(d, start)? != 0 {
        return None;
    }
    let (name, mut p) = plain_at(d, start + 4)?;
    // Three numbers behind a 3, then a flag and the race, class, level, experience, the
    // experience of the next level and the sex.
    if u32_at(d, p)? != 3 {
        return None;
    }
    p += 4;
    for _ in 0..3 {
        p = number_at(d, p)?.1;
    }
    if u32_at(d, p)? != 2 {
        return None;
    }
    p += 5;
    let (race, p2) = string_at(d, p)?;
    let (class, p3) = string_at(d, p2)?;
    let (level, p4) = number_at(d, p3)?;
    let (experience, p5) = number_at(d, p4)?;
    let (_, p6) = number_at(d, p5)?;
    let (_, mut p) = string_at(d, p6)?;
    let mut blocks = HashMap::new();
    let mut skills = Vec::new();
    while p < end {
        let tag: [u8; 4] = d.get(p..p + 4)?.try_into().ok()?;
        let blen = u32_at(d, p + 4)? as usize;
        let bend = p + 8 + blen;
        if bend > end {
            return None;
        }
        if matches!(&tag, b"STAT" | b"BSTA" | b"COND" | b"SKIL") {
            let (bname, nums) = block(d, p + 8, blen)?;
            if &tag == b"SKIL" {
                skills.push(bname.clone());
            }
            blocks.insert((String::from_utf8_lossy(&tag).into_owned(), bname), nums);
        }
        p = bend;
    }
    (p == end).then_some(Champion { name, race, class, level, experience, blocks, skills })
}

#[derive(Clone, Debug)]
struct Leaf {
    /// Offsets of the numbers that follow this value.
    targets: Vec<usize>,
    shown: f64,
}

#[derive(Clone, Debug)]
struct GrimBackend {
    data: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
}

impl Backend for GrimBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = (*self.data).clone();
        for (leaf, v) in changes {
            let l = &self.leaves[*leaf];
            let new = v.as_f64().filter(|f| f.is_finite()).ok_or("expected a number")?;
            let delta = new - l.shown;
            for &t in &l.targets {
                // What this number carries on top of the base (an item's bonus) stays.
                let cur = f64_at(&out, t).ok_or("the save is cut off")?;
                out[t..t + 8].copy_from_slice(&(cur + delta).to_le_bytes());
            }
        }
        Ok(out)
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        match v.as_f64() {
            Some(f) if f.is_finite() && (0.0..=100_000.0).contains(&f) => Ok(()),
            _ => Err("must be between 0 and 100000".into()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn shown(f: f64) -> Value {
    if f.fract().abs() < 1e-9 && f.abs() < 1e12 {
        Value::from(f.round() as i64)
    } else {
        crate::tree::num((f * 100.0).round() / 100.0)
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let cs = chunks(d).ok_or("not a Legend of Grimrock save")?;
    let mut party = Vec::new();
    let mut leaves: Vec<Leaf> = Vec::new();
    let mut editable = HashMap::new();
    for c in cs.iter().filter(|c| &c.tag == b"CHAR") {
        let Some(ch) = champion(d, c) else { continue };
        let idx = party.len();
        let mut m = Map::new();
        m.insert("Name".into(), Value::String(ch.name.clone()));
        m.insert("Race".into(), Value::String(ch.race.clone()));
        m.insert("Class".into(), Value::String(ch.class.clone()));
        m.insert("Level".into(), shown(ch.level));
        m.insert("Experience".into(), shown(ch.experience));
        let mut add = |m: &mut Map<String, Value>, group: Option<&str>, label: &str, targets: Vec<usize>| {
            let Some(&first) = targets.first() else { return };
            let Some(v) = f64_at(d, first) else { return };
            let ptr = match group {
                Some(g) => format!("/Party/{idx}/{g}/{label}"),
                None => format!("/Party/{idx}/{label}"),
            };
            let leaf = leaves.len();
            leaves.push(Leaf { targets, shown: v });
            editable.insert(ptr, leaf);
            let slot = match group {
                Some(g) => m.entry(g.to_string()).or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap(),
                None => m,
            };
            slot.insert(label.to_string(), shown(v));
        };
        // Current and highest value of the pools; the base stats and what they add up to.
        for (stat, label, max_label) in [("health", "Health", "Maximum health"), ("energy", "Energy", "Maximum energy")] {
            let mut cur = Vec::new();
            let mut max = Vec::new();
            for tag in ["STAT", "BSTA"] {
                if let Some(n) = ch.blocks.get(&(tag.to_string(), stat.to_string())) {
                    if n.len() == 2 {
                        cur.push(n[0]);
                        max.push(n[1]);
                    }
                }
            }
            add(&mut m, None, label, cur);
            add(&mut m, None, max_label, max);
        }
        for (stat, label) in [("strength", "Strength"), ("dexterity", "Dexterity"), ("vitality", "Vitality"), ("willpower", "Willpower")] {
            let mut t = Vec::new();
            for tag in ["STAT", "BSTA"] {
                if let Some(n) = ch.blocks.get(&(tag.to_string(), stat.to_string())) {
                    if n.len() == 2 {
                        // Value and highest value both follow the stat.
                        t.push(n[0]);
                        t.push(n[1]);
                    }
                }
            }
            add(&mut m, None, label, t);
        }
        if let Some(n) = ch.blocks.get(&("COND".to_string(), "unused_skill_points".to_string())) {
            if let Some(&first) = n.first() {
                add(&mut m, None, "Unused skill points", vec![first]);
            }
        }
        for s in &ch.skills {
            if let Some(n) = ch.blocks.get(&("SKIL".to_string(), s.clone())) {
                if n.len() == 1 {
                    add(&mut m, Some("Skills"), s, n.clone());
                }
            }
        }
        party.push(Value::Object(m));
    }
    if party.is_empty() {
        return Err("no party members".into());
    }
    let mut root = Map::new();
    root.insert("Party".into(), Value::Array(party));
    let backend = GrimBackend { data: Arc::new(d.to_vec()), leaves };
    let note = crate::i18n::tx("Close the game first. A stat is changed in the base value and in the current value together, so what items add stays. Level and experience are shown for reference; the game raises a level when the experience is earned.");
    Ok(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(backend)).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(v: f64) -> Vec<u8> {
        let mut o = 1u32.to_le_bytes().to_vec();
        o.extend_from_slice(&v.to_le_bytes());
        o
    }

    fn text(s: &str) -> Vec<u8> {
        let mut o = 0u32.to_le_bytes().to_vec();
        o.extend_from_slice(&(s.len() as u32).to_le_bytes());
        o.extend_from_slice(s.as_bytes());
        o
    }

    fn named(tag: &[u8; 4], name: &str, vals: &[f64]) -> Vec<u8> {
        let mut body = 0u32.to_le_bytes().to_vec();
        body.extend_from_slice(&text(name)[4..]);
        for v in vals {
            body.extend_from_slice(&num(*v));
        }
        let mut o = tag.to_vec();
        o.extend_from_slice(&(body.len() as u32).to_le_bytes());
        o.extend_from_slice(&body);
        o
    }

    pub fn save() -> Vec<u8> {
        let mut cham = 0u32.to_le_bytes().to_vec();
        cham.extend_from_slice(&text("Aregor")[4..]);
        cham.extend_from_slice(&3u32.to_le_bytes());
        for v in [0.0, 4.0, 750.0] {
            cham.extend_from_slice(&num(v));
        }
        cham.extend_from_slice(&2u32.to_le_bytes());
        cham.push(1);
        cham.extend_from_slice(&text("Human"));
        cham.extend_from_slice(&text("Fighter"));
        for v in [11.0, 60_880.0, 71_000.0] {
            cham.extend_from_slice(&num(v));
        }
        cham.extend_from_slice(&text("male"));
        cham.extend_from_slice(&named(b"STAT", "health", &[150.0, 153.0]));
        cham.extend_from_slice(&named(b"STAT", "strength", &[18.0, 18.0]));
        cham.extend_from_slice(&named(b"COND", "unused_skill_points", &[3.0, 0.0]));
        cham.extend_from_slice(&named(b"SKIL", "swords", &[23.0]));
        cham.extend_from_slice(&named(b"BSTA", "health", &[150.0, 153.0]));
        cham.extend_from_slice(&named(b"BSTA", "strength", &[16.0, 16.0]));
        let mut ch = num(1.0);
        ch.extend_from_slice(b"CHAM");
        ch.extend_from_slice(&(cham.len() as u32).to_le_bytes());
        ch.extend_from_slice(&cham);
        let mut out = b"DESC".to_vec();
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(b"CHAR");
        out.extend_from_slice(&(ch.len() as u32).to_le_bytes());
        out.extend_from_slice(&ch);
        out.extend_from_slice(b"ENTY");
        out.extend_from_slice(&3u32.to_le_bytes());
        out.extend_from_slice(&[9, 9, 9]);
        out
    }

    fn file(inner: &[u8]) -> Vec<u8> {
        let mut o = b"GRIM".to_vec();
        o.extend_from_slice(&6u32.to_le_bytes());
        o.extend_from_slice(&(inner.len() as u32).to_le_bytes());
        let mut z = ZlibEncoder::new(o, Compression::fast());
        z.write_all(inner).unwrap();
        z.finish().unwrap()
    }

    #[test]
    fn reads_edits_and_packs_again() {
        let inner = save();
        let f = file(&inner);
        let (p, data) = open(&f).unwrap();
        assert_eq!(data, inner);
        assert!(looks(&data));
        let mut doc = parse(&data).unwrap();
        assert_eq!(doc.view.pointer("/Party/0/Class").unwrap(), "Fighter");
        assert_eq!(doc.view.pointer("/Party/0/Level").unwrap(), 11);
        assert_eq!(doc.view.pointer("/Party/0/Strength").unwrap(), 18);
        assert_eq!(doc.view.pointer("/Party/0/Skills/swords").unwrap(), 23);
        assert_eq!(close(&p, &doc.to_bytes().unwrap()).unwrap(), f);
        doc.set("/Party/0/Strength", serde_json::json!(30)).unwrap();
        doc.set("/Party/0/Skills/swords", serde_json::json!(50)).unwrap();
        doc.set("/Party/0/Unused skill points", serde_json::json!(40)).unwrap();
        let out = close(&p, &doc.to_bytes().unwrap()).unwrap();
        let (_, again) = open(&out).unwrap();
        let d2 = parse(&again).unwrap();
        assert_eq!(d2.view.pointer("/Party/0/Strength").unwrap(), 30);
        assert_eq!(d2.view.pointer("/Party/0/Skills/swords").unwrap(), 50);
        assert_eq!(d2.view.pointer("/Party/0/Unused skill points").unwrap(), 40);
        // The base stat followed by the same amount: 16 + 12.
        let c = champion(&again, &chunks(&again).unwrap()[1]).unwrap();
        let base = c.blocks[&("BSTA".to_string(), "strength".to_string())][0];
        assert_eq!(f64_at(&again, base), Some(28.0));
        assert_eq!(again.len(), inner.len());
        assert!(doc.set("/Party/0/Strength", serde_json::json!(-5)).is_err());
    }

    #[test]
    fn other_files_are_not_taken() {
        assert!(open(&[0u8; 100]).is_none());
        assert!(!looks(b"DESC\x00"));
    }
}
