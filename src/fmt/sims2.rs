//! The Sims 2 neighbourhood packages (`N001_Neighborhood.package`, `.package` files in
//! `Neighborhoods\N001\`): families with their funds and, for each Sim, personality, skills and
//! the rest of the character data.
//!
//! The package is a DBPF 1.x file (see `dbpf`); the editor unpacks the `FAMI` (family), `SDSC`
//! (Sim description) and the `STR#` that names each family into a flat form, and the layer in
//! `formats` writes the package again. `FAMI` (version 0x55) has the lot at body offset 12, the
//! funds at 28 and the member count at 40. `SDSC` has u16 values: personality at 0x10 (Nice),
//! 0x12 (Active), 0x16 (Playful), 0x18 (Outgoing), 0x1A (Neat); skills at 0x1E (Cleaning), 0x20
//! (Cooking), 0x22 (Charisma), 0x24 (Mechanical), 0x2A (Creativity), 0x2E (Body), 0x30 (Logic);
//! age at 0x80, family number at 0x86 (the `FAMI` instance), job performance at 0x8A, gender at
//! 0x8E, aspiration score at 0x14C. Offsets from the ModTheSims wiki and SimPE (GPL-2.0-or-later),
//! checked on a public neighbourhood. Sim names are in other files, so Sims are listed by id.

use super::dbpf::{self, flat_entries};
use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "The Sims 2 neighbourhood";
pub const FAMI: u32 = 0x4641_4D49;
pub const SDSC: u32 = 0xAACE_2EFB;
pub const STRS: u32 = 0x5354_5223;
/// How much of a `STR#` the flat form keeps (the file name, then the family's name).
const STR_CAP: usize = 160;

/// What the flat form of a package holds: the resource type decides how much is kept.
pub fn want(k: &[u32; 4]) -> Option<usize> {
    match k[0] {
        FAMI | SDSC => Some(usize::MAX),
        STRS => Some(STR_CAP),
        _ => None,
    }
}

pub fn editable(k: &[u32; 4]) -> bool {
    matches!(k[0], FAMI | SDSC)
}

pub fn looks_package(d: &[u8]) -> bool {
    dbpf::index(d).is_some_and(|i| i.iter().any(|e| e.key[0] == FAMI) && i.iter().any(|e| e.key[0] == SDSC))
}

pub fn flatten(d: &[u8]) -> Option<Vec<u8>> {
    looks_package(d).then(|| dbpf::flatten(d, want)).flatten()
}

pub fn looks(d: &[u8]) -> bool {
    flat_entries(d).is_some_and(|e| e.iter().any(|f| f.key[0] == FAMI))
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

fn i32_at(d: &[u8], o: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// The family's name: after the 64-byte file name, `FD FF`, a count, then per string a language
/// byte and the title.
fn family_name(b: &[u8]) -> Option<String> {
    if b.get(64..66)? != [0xFD, 0xFF] {
        return None;
    }
    let t = b.get(69..)?;
    let end = t.iter().position(|&x| x == 0).unwrap_or(t.len());
    let s: String = t[..end].iter().map(|&x| x as char).collect();
    (!s.trim().is_empty() && s.chars().all(|c| !c.is_control())).then(|| s.trim().to_string())
}

const PERSONALITY: [(&str, usize); 5] = [("Neat", 0x1A), ("Outgoing", 0x18), ("Active", 0x12), ("Playful", 0x16), ("Nice", 0x10)];
const SKILLS: [(&str, usize); 7] =
    [("Cooking", 0x20), ("Mechanical", 0x24), ("Charisma", 0x22), ("Body", 0x2E), ("Logic", 0x30), ("Creativity", 0x2A), ("Cleaning", 0x1E)];

fn age(v: u16) -> String {
    match v {
        1 => "Baby".into(),
        2 => "Toddler".into(),
        3 => "Child".into(),
        0x10 => "Teen".into(),
        0x13 => "Adult".into(),
        0x33 => "Elder".into(),
        0x40 | 0x41 => "Young adult".into(),
        n => format!("Age {n:#x}"),
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let es = flat_entries(d).ok_or("not a Sims 2 neighbourhood")?;
    let names: HashMap<u32, String> = es.iter().filter(|f| f.key[0] == STRS).filter_map(|f| Some((f.key[2], family_name(f.body)?))).collect();
    let mut b = Builder::new(d);
    let mut sims: HashMap<u32, Vec<&dbpf::Flat>> = HashMap::new();
    for f in es.iter().filter(|f| f.key[0] == SDSC && f.body.len() >= 0x150) {
        sims.entry(u16_at(f.body, 0x86).unwrap_or(0) as u32).or_default().push(f);
    }
    let mut used = std::collections::HashSet::new();
    let mut fams: Vec<&dbpf::Flat> = es.iter().filter(|f| f.key[0] == FAMI && f.body.len() >= 44).collect();
    fams.sort_by_key(|f| f.key[2]);
    for f in fams {
        let mut name = names.get(&f.key[2]).cloned().unwrap_or_else(|| format!("Family {}", f.key[2]));
        if !used.insert(name.clone()) {
            name = format!("{name} ({})", f.key[2]);
            used.insert(name.clone());
        }
        let lot = i32_at(f.body, 12).unwrap_or(0);
        b.ranged(&["Families", &name, "Funds"], f.at + 28, Kind::I32, 0, 99_999_999);
        b.info(&["Families", &name, "Lot"], if lot > 0 { format!("{lot}") } else { "None (not moved in)".to_string() });
        for s in sims.remove(&f.key[2]).unwrap_or_default() {
            person(&mut b, &["Families", &name, "Sims", &format!("Sim {}", s.key[2])], s);
        }
    }
    let mut rest: Vec<_> = sims.into_values().flatten().collect();
    rest.sort_by_key(|s| s.key[2]);
    for s in rest {
        person(&mut b, &["Other Sims", &format!("Sim {}", s.key[2])], s);
    }
    Ok(b
        .finish(KIND, Checksum::None)
        .with_advanced(vec!["Other Sims".into()])
        .with_note("Sims are listed by their id; their names are in the character files. Edited parts are stored uncompressed when you save. Change the file while the game is closed."))
}

fn person(b: &mut Builder, path: &[&str], f: &dbpf::Flat) {
    let d = f.body;
    let at = |o: usize| f.at + o;
    let mut p: Vec<&str> = path.to_vec();
    let gender = u16_at(d, 0x8E).unwrap_or(0);
    p.push("About");
    b.info(&p, format!("{}, {}", age(u16_at(d, 0x80).unwrap_or(0)), if gender == 1 { "male" } else { "female" }));
    p.pop();
    for (group, list) in [("Personality", &PERSONALITY[..]), ("Skills", &SKILLS[..])] {
        for (name, o) in list {
            p.push(group);
            p.push(name);
            b.ranged(&p, at(*o), Kind::U16, 0, 1000);
            p.pop();
            p.pop();
        }
    }
    p.push("Job performance");
    b.field(&p, at(0x8A), Kind::U16);
    p.pop();
    p.push("Aspiration score");
    b.field(&p, at(0x14C), Kind::U16);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn flat(parts: &[([u32; 4], Vec<u8>)]) -> Vec<u8> {
        let mut o = b"S2FL".to_vec();
        o.extend((parts.len() as u32).to_le_bytes());
        for (k, b) in parts {
            for v in k {
                o.extend(v.to_le_bytes());
            }
            o.extend((b.len() as u32).to_le_bytes());
            o.extend(b);
        }
        o
    }

    pub fn sample() -> Vec<u8> {
        let mut fami = vec![0u8; 52];
        fami[12..16].copy_from_slice(&7i32.to_le_bytes());
        fami[28..32].copy_from_slice(&20_000i32.to_le_bytes());
        let mut sdsc = vec![0u8; 0x160];
        sdsc[0x20..0x22].copy_from_slice(&450u16.to_le_bytes());
        sdsc[0x1A..0x1C].copy_from_slice(&300u16.to_le_bytes());
        sdsc[0x80..0x82].copy_from_slice(&0x13u16.to_le_bytes());
        sdsc[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        sdsc[0x8E..0x90].copy_from_slice(&1u16.to_le_bytes());
        let mut st = vec![0u8; 64];
        st.extend([0xFD, 0xFF, 1, 0, 1]);
        st.extend(b"Goth\0");
        flat(&[([FAMI, 0, 1, 0], fami), ([SDSC, 0, 5, 0], sdsc), ([STRS, 0, 1, 0], st)])
    }

    #[test]
    fn reads_and_edits_flat() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Families/Goth/Funds"), Some(Value::from(20_000)));
        assert_eq!(v(&doc, "/Families/Goth/Sims/Sim 5/Skills/Cooking"), Some(Value::from(450)));
        assert_eq!(v(&doc, "/Families/Goth/Sims/Sim 5/About"), Some(Value::from("Adult, male")));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Families/Goth/Funds", Value::from(99_999_999)).unwrap();
        doc.set("/Families/Goth/Sims/Sim 5/Skills/Logic", Value::from(1000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(v(&parse(&out).unwrap(), "/Families/Goth/Funds"), Some(Value::from(99_999_999)));
    }

    /// Research aid: SAVE_EDITOR_SIMS2 = packages separated by `|`.
    #[test]
    #[ignore]
    fn real_files() {
        for path in std::env::var("SAVE_EDITOR_SIMS2").unwrap().split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            let f = flatten(&d).unwrap();
            let doc = parse(&f).unwrap();
            for (k, v) in doc.view["Families"].as_object().unwrap() { println!("{k}: funds {} sims {}", v["Funds"], v["Sims"].as_object().map(|m| m.len()).unwrap_or(0)); }
            assert_eq!(doc.to_bytes().unwrap(), f);
            assert_eq!(dbpf::rebuild(&d, &f, editable).unwrap(), d, "unchanged data keeps the file");
            let mut doc = parse(&f).unwrap();
            let fam = doc.view["Families"].as_object().unwrap().keys().next().unwrap().clone();
            doc.set(&format!("/Families/{fam}/Funds"), Value::from(1_234_567)).unwrap();
            let out = dbpf::rebuild(&d, &doc.to_bytes().unwrap(), editable).unwrap();
            let idx = dbpf::index(&out).unwrap();
            assert_eq!(idx.len(), dbpf::index(&d).unwrap().len());
            let again = parse(&flatten(&out).unwrap()).unwrap();
            assert_eq!(again.view["Families"][&fam]["Funds"], Value::from(1_234_567));
            println!("rewritten: {} -> {} bytes", d.len(), out.len());
        }
    }
}
