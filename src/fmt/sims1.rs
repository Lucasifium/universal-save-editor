//! The Sims (2000) and The Sims Legacy Collection: `Neighborhood.iff`.
//!
//! A Maxis IFF 2.5 file: a 64-byte header (a 60-byte text line, then the big-endian place of
//! the resource map), then chunks. Each chunk has a 76-byte header (four-letter type,
//! big-endian size including the header, big-endian id, flags, 64-byte label) and a
//! little-endian body. Nothing is compressed and there is no checksum, so every value is
//! changed in place.
//!
//! `FAMI` is one family: zero, version, `IMAF`, house number, family number, the budget at
//! body offset 20, the value of the house, the family's friends, flags, the member count and
//! each member's GUID. `FAMs` with the same id holds the family's name. `NBRS` lists every
//! Sim: a marker (1), a record version (4 or 0xA, which adds a number), the Sim's file name,
//! zero, a mode, and when the mode is above zero a block of 0xA0 (version 4) or 0x200 bytes
//! whose first 16-bit values are the person data (personality at 2 to 7, skills at 9 to 18,
//! job at 56, age at 58, job performance at 63, gender at 65); then the neighbour id, the
//! GUID, -1 and the relationship list.
//!
//! Layout and the person-data index names from SimObliterator Suite (Dnf_Jeff, MIT), itself
//! a port of FreeSO's NBRS reader; checked on a public neighbourhood.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "The Sims neighbourhood";
const MAGIC: &[u8] = b"IFF FILE 2.5:TYPE FOLLOWED BY SIZE";
const HEAD: usize = 76;

pub const PERSONALITY: [(&str, usize); 6] = [("Neat", 7), ("Outgoing", 6), ("Active", 3), ("Playful", 5), ("Nice", 2), ("Generous", 4)];
/// The seven skills the game shows.
pub const SKILLS: [(&str, usize); 7] =
    [("Cooking", 10), ("Mechanical", 12), ("Charisma", 11), ("Body", 17), ("Logic", 18), ("Creativity", 15), ("Cleaning", 9)];
const JOB: usize = 56;
const AGE: usize = 58;
const JOB_PERFORMANCE: usize = 63;
const GENDER: usize = 65;
struct Chunk {
    kind: [u8; 4],
    id: u16,
    body: usize,
    end: usize,
}

fn chunks(d: &[u8]) -> Vec<Chunk> {
    let mut out = vec![];
    let mut at = 64;
    while at + HEAD <= d.len() {
        let size = u32::from_be_bytes(d[at + 4..at + 8].try_into().unwrap()) as usize;
        if size < HEAD || at + size > d.len() {
            break;
        }
        out.push(Chunk { kind: d[at..at + 4].try_into().unwrap(), id: u16::from_be_bytes([d[at + 8], d[at + 9]]), body: at + HEAD, end: at + size });
        at += size;
    }
    out
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 + HEAD && d.starts_with(MAGIC) && chunks(d).iter().any(|c| &c.kind == b"FAMI" || &c.kind == b"NBRS")
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The first string of a `FAMs` chunk (the family's name).
fn family_name(d: &[u8], c: &Chunk) -> Option<String> {
    let b = &d[c.body..c.end];
    // Format -3: a count, then per string a language byte, the text and a comment.
    if b.len() < 6 || b[0] != 0xFD || b[1] != 0xFF {
        return None;
    }
    let text = &b[5..];
    let end = text.iter().position(|&x| x == 0)?;
    let s: String = text[..end].iter().map(|&x| x as char).collect();
    (!s.trim().is_empty()).then(|| s.trim().to_string())
}

/// One Sim of the `NBRS` list: its file name, GUID and where its person data starts.
struct Sim {
    name: String,
    guid: u32,
    data: usize,
}

fn sims(d: &[u8], c: &Chunk) -> Option<Vec<Sim>> {
    let end = c.end;
    if d.get(c.body + 8..c.body + 12)? != b"SRBN" {
        return None;
    }
    let count = i32_at(d, c.body + 12)?.max(0) as usize;
    let mut p = c.body + 16;
    let mut out = vec![];
    for _ in 0..count {
        if p + 4 > end {
            break;
        }
        if i32_at(d, p)? != 1 {
            p += 4;
            continue;
        }
        let version = i32_at(d, p + 4)?;
        p += if version == 0xA { 12 } else { 8 };
        let len = d.get(p..end)?.iter().position(|&x| x == 0)?;
        let name: String = d[p..p + len].iter().map(|&x| x as char).collect();
        p += len + 1 + usize::from(len % 2 == 0);
        let mode = i32_at(d, p + 4)?;
        p += 8;
        let mut data = 0;
        if mode > 0 {
            data = p;
            p += if version == 4 { 0xA0 } else { 0x200 };
        }
        let guid = i32_at(d, p + 2)? as u32;
        let relations = i32_at(d, p + 10)?.max(0) as usize;
        p += 14;
        for _ in 0..relations {
            let values = i32_at(d, p + 8)?.max(0) as usize;
            p += 12 + 4 * values;
        }
        if p > end {
            return None;
        }
        if data != 0 {
            out.push(Sim { name, guid, data });
        }
    }
    Some(out)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Sims neighbourhood file".into());
    }
    let cs = chunks(d);
    let names: HashMap<u16, String> = cs.iter().filter(|c| &c.kind == b"FAMs").filter_map(|c| Some((c.id, family_name(d, c)?))).collect();
    let all: Vec<Sim> = cs.iter().filter(|c| &c.kind == b"NBRS").filter_map(|c| sims(d, c)).flatten().collect();
    let by_guid: HashMap<u32, &Sim> = all.iter().map(|s| (s.guid, s)).collect();
    let mut b = Builder::new(d);
    let mut used = std::collections::HashSet::new();
    let mut taken = std::collections::HashSet::new();
    for c in cs.iter().filter(|c| &c.kind == b"FAMI") {
        if d.get(c.body + 8..c.body + 12) != Some(b"IMAF".as_slice()) || c.end < c.body + 40 {
            continue;
        }
        let house = i32_at(d, c.body + 12).unwrap_or(0);
        let members = i32_at(d, c.body + 36).unwrap_or(0).clamp(0, 64) as usize;
        let mut name = names.get(&c.id).cloned().unwrap_or_else(|| format!("Family {}", c.id));
        if !used.insert(name.clone()) {
            name = format!("{name} ({})", c.id);
            used.insert(name.clone());
        }
        let fam = name.as_str();
        b.ranged(&["Families", fam, "Funds"], c.body + 20, Kind::I32, 0, 9_999_999);
        b.info(&["Families", fam, "House"], if house > 0 { format!("{house}") } else { "None (not moved in)".to_string() });
        if let Some(v) = i32_at(d, c.body + 24) {
            b.info(&["Families", fam, "House value"], v as i64);
        }
        b.ranged(&["Families", fam, "Family friends"], c.body + 28, Kind::I32, 0, 999);
        for m in 0..members {
            let Some(guid) = i32_at(d, c.body + 40 + 4 * m).map(|g| g as u32) else { break };
            let Some(sim) = by_guid.get(&guid) else { continue };
            if taken.insert(sim.data) {
                person(&mut b, &["Families", fam, "Sims", &sim.name], d, sim.data);
            }
        }
    }
    for sim in all.iter().filter(|s| !taken.contains(&s.data)) {
        let key = format!("{} ({:08X})", sim.name, sim.guid);
        person(&mut b, &["Other Sims", &key], d, sim.data);
    }
    Ok(b
        .finish(KIND, Checksum::None)
        .with_advanced(vec!["Other Sims".into()])
        .with_note("Sims are listed by their file name (user00012 is Characters\\User00012.iff, which holds the Sim's own name). Change the file while the game is closed."))
}

fn person(b: &mut Builder, path: &[&str], d: &[u8], data: usize) {
    let at = |i: usize| data + 2 * i;
    let mut p: Vec<&str> = path.to_vec();
    let age = i16::from_le_bytes([d[at(AGE)], d[at(AGE) + 1]]);
    let gender = i16::from_le_bytes([d[at(GENDER)], d[at(GENDER) + 1]]);
    p.push("About");
    b.info(&p, format!("{} {}", if age == 0 { "Child," } else { "Adult," }, if gender == 0 { "male" } else { "female" }));
    p.pop();
    for (group, list) in [("Personality", &PERSONALITY[..]), ("Skills", &SKILLS[..])] {
        for (name, i) in list {
            p.push(group);
            p.push(name);
            b.ranged(&p, at(*i), Kind::I16, 0, 1000);
            p.pop();
            p.pop();
        }
    }
    p.push("Career");
    let job = i16::from_le_bytes([d[at(JOB)], d[at(JOB) + 1]]);
    b.info(&p, if job > 0 { format!("Career track {job}") } else { "Unemployed".to_string() });
    p.pop();
    if job > 0 {
        p.push("Job performance");
        b.field(&p, at(JOB_PERFORMANCE), Kind::I16);
        p.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn chunk(kind: &[u8; 4], id: u16, body: &[u8]) -> Vec<u8> {
        let mut c = kind.to_vec();
        c.extend(((HEAD + body.len()) as u32).to_be_bytes());
        c.extend(id.to_be_bytes());
        c.extend([0, 0x10]);
        c.extend([0u8; 64]);
        c.extend(body);
        c
    }

    fn sim(name: &str, version: i32, guid: u32, set: &[(usize, i16)]) -> Vec<u8> {
        let mut r = vec![];
        r.extend(1i32.to_le_bytes());
        r.extend(version.to_le_bytes());
        if version == 0xA {
            r.extend(9i32.to_le_bytes());
        }
        r.extend(name.bytes());
        r.push(0);
        if name.len() % 2 == 0 {
            r.push(0);
        }
        r.extend(0i32.to_le_bytes());
        r.extend(9i32.to_le_bytes());
        let mut data = vec![0u8; if version == 4 { 0xA0 } else { 0x200 }];
        for (i, v) in set {
            data[2 * i..2 * i + 2].copy_from_slice(&v.to_le_bytes());
        }
        r.extend(data);
        r.extend(7i16.to_le_bytes());
        r.extend(guid.to_le_bytes());
        r.extend((-1i32).to_le_bytes());
        // One relationship with two values.
        r.extend(1i32.to_le_bytes());
        r.extend([1i32, 3, 2, 50, 60].iter().flat_map(|v| v.to_le_bytes()));
        r
    }

    fn synthetic() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.resize(64, 0);
        let mut fami = vec![];
        for v in [0i32, 9, i32::from_le_bytes(*b"IMAF"), 5, 1, 20_000, 31_000, 2, 0, 2, 0x1111, 0x2222] {
            fami.extend(v.to_le_bytes());
        }
        d.extend(chunk(b"FAMI", 3, &fami));
        d.extend(chunk(b"FAMs", 3, b"\xFD\xFF\x01\x00\x01Goth\0\0\0"));
        let mut nbrs = vec![];
        nbrs.extend(0i32.to_le_bytes());
        nbrs.extend(0x49i32.to_le_bytes());
        nbrs.extend(b"SRBN");
        nbrs.extend(4i32.to_le_bytes());
        nbrs.extend(sim("user00001", 0xA, 0x1111, &[(7, 300), (10, 450), (56, 4), (58, 1), (63, 20), (65, 1)]));
        // A removed entry is a lone zero.
        nbrs.extend(0i32.to_le_bytes());
        nbrs.extend(sim("user02", 4, 0x2222, &[(18, 1000), (58, 0)]));
        nbrs.extend(sim("npc_maid", 0xA, 0x9999, &[(2, 500)]));
        d.extend(chunk(b"NBRS", 1, &nbrs));
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = synthetic();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Families/Goth/Funds"), Some(Value::from(20_000)));
        assert_eq!(v(&doc, "/Families/Goth/House"), Some(Value::from("5")));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user00001/Personality/Neat"), Some(Value::from(300)));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user00001/Skills/Cooking"), Some(Value::from(450)));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user00001/Career"), Some(Value::from("Career track 4")));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user00001/About"), Some(Value::from("Adult, female")));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user00001/Job performance"), Some(Value::from(20)));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user02/Skills/Logic"), Some(Value::from(1000)));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user02/About"), Some(Value::from("Child, male")));
        assert_eq!(v(&doc, "/Families/Goth/Sims/user02/Job performance"), None);
        assert_eq!(v(&doc, "/Other Sims/npc_maid (00009999)/Personality/Nice"), Some(Value::from(500)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Families/Goth/Funds", Value::from(9_999_999)).unwrap();
        doc.set("/Families/Goth/Sims/user02/Skills/Cooking", Value::from(1000)).unwrap();
        assert!(doc.set("/Families/Goth/Sims/user02/Skills/Cooking", Value::from(1001)).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).unwrap();
        assert_eq!(v(&again, "/Families/Goth/Funds"), Some(Value::from(9_999_999)));
        assert_eq!(v(&again, "/Families/Goth/Sims/user02/Skills/Cooking"), Some(Value::from(1000)));
        assert_eq!(v(&again, "/Families/Goth/Sims/user00001/Skills/Cooking"), Some(Value::from(450)));
        assert!(!looks(&d[..60]));
    }

    /// Research aid: SAVE_EDITOR_SIMS1 = `Neighborhood.iff` files separated by `|`.
    #[test]
    #[ignore]
    fn real_files() {
        for path in std::env::var("SAVE_EDITOR_SIMS1").unwrap().split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            let doc = parse(&d).unwrap();
            let fams = doc.view["Families"].as_object().map(|m| m.len()).unwrap_or(0);
            let others = doc.view["Other Sims"].as_object().map(|m| m.len()).unwrap_or(0);
            println!("{fams} families, {others} other Sims");
            println!("{}", serde_json::to_string(&doc.view["Families"]).unwrap().chars().take(3000).collect::<String>());
            assert_eq!(doc.to_bytes().unwrap(), d, "round trip");
        }
    }
}
