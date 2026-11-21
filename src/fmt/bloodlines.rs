//! Vampire: The Masquerade - Bloodlines saves (`Vampire-NNN.sav`, a few MB): a "JSAV" container
//! (a 20-byte header, a token table, then one entry per map file). Every entry is a 260-byte
//! name, the unpacked size, the packed size and a zlib stream; files over 512 KB continue as
//! `[packed size][zlib stream]` pieces of 512 KB each. The map the game was saved in is named
//! (`maps/<name>.bsp`) in the header.
//!
//! Each `NAME.HL1` file is a Source engine state file ("VALV"): a token table (every field name,
//! one slot per token, empty slots stored as one zero byte) followed by the entities as records
//! of `[u16 size][u16 token][data]`. The player's record run holds the character sheet as four-byte
//! values (attributes, abilities, disciplines, each as a base and a current copy) and the money.
//! Only the map the game was saved in is read when the game loads, so only its player copy is edited.
//!
//! Values are edited in place; the map file is then packed again and written back into the container.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

pub const KIND: &str = "Vampire: Bloodlines save";

/// Where the first entry starts in every save.
const FIRST: usize = 16677;
const CHUNK: usize = 0x80000;
const VALV: u32 = 0x564c_4156;

struct Ent {
    at: usize,
    end: usize,
    name: String,
    size: usize,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u16le(d: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > FIRST + 268 && d.starts_with(b"JSAV") && u32le(d, 4) == Some(117)
}

/// Every map file in the container, when the entries add up to exactly the file.
fn entries(d: &[u8]) -> Option<Vec<Ent>> {
    let mut v = Vec::new();
    let mut p = FIRST;
    while p < d.len() {
        let raw = d.get(p..p + 260)?;
        let n = raw.iter().position(|&b| b == 0)?;
        if n == 0 {
            return None;
        }
        let name = String::from_utf8_lossy(&raw[..n]).into_owned();
        let size = u32le(d, p + 260)? as usize;
        let first = u32le(d, p + 264)? as usize;
        let mut q = p.checked_add(268)?.checked_add(first)?;
        d.get(p + 268..q)?;
        let mut left = size.saturating_sub(CHUNK);
        while left > 0 {
            let n = u32le(d, q)? as usize;
            q = q.checked_add(4)?.checked_add(n)?;
            d.get(..q)?;
            left = left.saturating_sub(CHUNK);
        }
        v.push(Ent { at: p, end: q, name, size });
        p = q;
    }
    (p == d.len() && !v.is_empty()).then_some(v)
}

fn unzlib(b: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    ZlibDecoder::new(b).read_to_end(&mut out).ok()?;
    Some(out)
}

fn inflate(d: &[u8], e: &Ent) -> Option<Vec<u8>> {
    let first = u32le(d, e.at + 264)? as usize;
    let mut q = e.at + 268;
    let mut out = unzlib(d.get(q..q + first)?)?;
    q += first;
    while out.len() < e.size {
        let n = u32le(d, q)? as usize;
        out.extend(unzlib(d.get(q + 4..q + 4 + n)?)?);
        q += 4 + n;
    }
    (out.len() == e.size && q == e.end).then_some(out)
}

fn zlib(b: &[u8]) -> Vec<u8> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::best());
    let _ = z.write_all(b);
    z.finish().unwrap_or_default()
}

/// The entry's header and packed body for `data`.
fn pack(name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut chunks = data.chunks(CHUNK);
    let first = zlib(chunks.next().unwrap_or(&[]));
    let mut out = name.to_vec();
    out.extend((data.len() as u32).to_le_bytes());
    out.extend((first.len() as u32).to_le_bytes());
    out.extend(first);
    for c in chunks {
        let z = zlib(c);
        out.extend((z.len() as u32).to_le_bytes());
        out.extend(z);
    }
    out
}

/// The map the game was saved in, named in the header.
fn current_map(d: &[u8]) -> Option<String> {
    let head = d.get(..FIRST)?;
    let a = head.windows(5).position(|w| w == b"maps/")? + 5;
    let n = head[a..].windows(4).position(|w| w == b".bsp")?;
    Some(String::from_utf8_lossy(&head[a..a + n]).into_owned())
}

fn map_entry(d: &[u8]) -> Option<Ent> {
    let want = format!("{}.hl1", current_map(d)?.to_lowercase());
    entries(d)?.into_iter().find(|e| e.name.to_lowercase() == want)
}

/// Puts the edited map file back into the container.
fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let Some(e) = map_entry(outer) else { return outer.to_vec() };
    let mut out = outer[..e.at].to_vec();
    out.extend(pack(&outer[e.at..e.at + 260], inner));
    out.extend(&outer[e.end..]);
    out
}

/// One map file: its token names and where the records start.
struct Valv<'a> {
    d: &'a [u8],
    toks: Vec<&'a [u8]>,
    base: usize,
}

impl<'a> Valv<'a> {
    fn new(d: &'a [u8]) -> Option<Self> {
        if u32le(d, 0)? != VALV || u32le(d, 4)? != 117 {
            return None;
        }
        let tsz = u32le(d, 8)? as usize;
        let count = u32le(d, 12)? as usize;
        let table = d.get(24..24usize.checked_add(tsz)?)?;
        let toks: Vec<&[u8]> = table.split(|&b| b == 0).take(count).collect();
        (toks.len() == count && count > 0).then_some(Self { d, toks, base: 24 + tsz })
    }

    fn token(&self, name: &str) -> Option<usize> {
        self.toks.iter().position(|t| *t == name.as_bytes())
    }

    /// The record at `p` (offset in the records): its token and size.
    fn rec(&self, p: usize) -> Option<(usize, usize)> {
        let size = u16le(self.d, self.base + p)?;
        let tok = u16le(self.d, self.base + p + 2)?;
        (size <= 0x2000 && self.toks.get(tok).is_some_and(|t| !t.is_empty()) && self.base + p + 4 + size <= self.d.len()).then_some((tok, size))
    }

    /// Offsets (in the whole file) of every four-byte value in the player's run of records.
    fn player(&self) -> Option<Vec<(usize, usize)>> {
        let xp = self.token("m_iVAttributesCurrent[ v_attribute_experience ]")?;
        let tail = &self.d[self.base..];
        let mut found = None;
        let pat = [4u8, 0, (xp & 0xff) as u8, (xp >> 8) as u8];
        for (at, w) in tail.windows(4).enumerate() {
            if w != pat {
                continue;
            }
            // Walk the records from just before it: the run must lead exactly to this record.
            let start = (at.saturating_sub(1500)..at).find(|&s| {
                let mut p = s;
                let mut n = 0;
                while p < at {
                    match self.rec(p) {
                        Some((_, sz)) => p += 4 + sz,
                        None => return false,
                    }
                    n += 1;
                }
                p == at && n >= 40
            });
            if let Some(s) = start {
                if found.is_some() {
                    return None;
                }
                found = Some((s, at));
            }
        }
        let (s, at) = found?;
        let mut out = Vec::new();
        let mut p = s;
        let mut n = 0;
        while let Some((tok, sz)) = self.rec(p) {
            if sz == 4 {
                out.push((tok, self.base + p + 4));
            }
            p += 4 + sz;
            n += 1;
            if n > 1200 && p > at {
                break;
            }
        }
        Some(out)
    }
}

const ATTRIBUTES: [(&str, &str); 9] = [
    ("Strength", "strength"),
    ("Dexterity", "dexterity"),
    ("Stamina", "stamina"),
    ("Charisma", "charisma"),
    ("Manipulation", "manipulation"),
    ("Appearance", "appearance"),
    ("Perception", "perception"),
    ("Intelligence", "intelligence"),
    ("Wits", "wits"),
];

const ABILITIES: [(&str, &str); 8] = [
    ("Academics", "academics"),
    ("Brawl", "brawl"),
    ("Computers", "computers"),
    ("Dodge", "dodge"),
    ("Finance", "finance"),
    ("Firearms", "firearms"),
    ("Melee", "melee"),
    ("Security", "security"),
];

const DISCIPLINES: [(&str, &str); 13] = [
    ("Animalism", "animalism"),
    ("Auspex", "auspex"),
    ("Blood healing", "blood_healing"),
    ("Celerity", "celerity"),
    ("Corpus vampirus", "corpus_vampirus"),
    ("Dementation", "dementation"),
    ("Dominate", "dominate"),
    ("Fortitude", "fortitude"),
    ("Obfuscate", "obfuscate"),
    ("Potence", "potence"),
    ("Presence", "presence"),
    ("Protean", "protean"),
    ("Thaumaturgy", "thaumaturgy"),
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Vampire: Bloodlines save".into());
    }
    let e = map_entry(d).ok_or("the save's map file was not found")?;
    let inner = inflate(d, &e).ok_or("the save's map file could not be unpacked")?;
    let v = Valv::new(&inner).ok_or("the map file is not a Source state file")?;
    let run = v.player().ok_or("the player's character sheet was not found in this save")?;
    let at = |name: &str| -> Option<usize> {
        let t = v.token(name)?;
        run.iter().find(|(tok, _)| *tok == t).map(|(_, off)| *off)
    };
    let mut b = Builder::new(&inner);
    b.inside(d, put);
    b.info(&["About", "Map"], current_map(d).unwrap_or_default());
    if let Some(o) = at("m_iMoney") {
        b.ranged(&["Character", "Money"], o, Kind::I32, 0, 9_999_999);
    }
    let both = |b: &mut Builder, path: [&str; 2], base: &str, cur: &str, lo: i64, hi: i64| {
        let (Some(x), y) = (at(base), at(cur)) else { return };
        if b.ranged(&path, x, Kind::I32, lo, hi).is_some() {
            if let Some(y) = y {
                b.mirror(&path, y);
            }
        }
    };
    both(&mut b, ["Character", "Experience to spend"], "m_iVAttributesBase[ v_attribute_experience ]", "m_iVAttributesCurrent[ v_attribute_experience ]", 0, 99_999);
    both(&mut b, ["Character", "Humanity"], "m_iVAttributesBase[ v_attribute_humanity ]", "m_iVAttributesCurrent[ v_attribute_humanity ]", 0, 10);
    for (label, key) in ATTRIBUTES {
        both(&mut b, ["Attributes", label], &format!("m_iVAttributesBase[ v_attribute_{key} ]"), &format!("m_iVAttributesCurrent[ v_attribute_{key} ]"), 1, 5);
    }
    for (label, key) in ABILITIES {
        both(&mut b, ["Abilities", label], &format!("m_iVAbilitiesBase[ v_ability_{key} ]"), &format!("m_iVAbilitiesCurrent[ v_ability_{key} ]"), 0, 5);
    }
    for (label, key) in DISCIPLINES {
        both(&mut b, ["Disciplines", label], &format!("m_iVDisciplinesBase[ v_discipline_{key} ]"), &format!("m_iVDisciplinesCurrent[ v_discipline_{key} ]"), -1, 5);
    }
    if b.is_empty() {
        return Err("no Bloodlines values found".into());
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn rec(tok: u16, data: &[u8]) -> Vec<u8> {
        let mut v = (data.len() as u16).to_le_bytes().to_vec();
        v.extend(tok.to_le_bytes());
        v.extend(data);
        v
    }

    /// A made-up map file: a token table and a player run of records.
    fn valv(values: &[(&str, i32)]) -> Vec<u8> {
        let names: Vec<&str> = std::iter::once("filler").chain(values.iter().map(|(n, _)| *n)).collect();
        let mut table = Vec::new();
        for n in &names {
            table.extend(n.as_bytes());
            table.push(0);
        }
        let mut recs = Vec::new();
        for i in 0..45 {
            recs.extend(rec(0, &[i as u8, 0, 0, 0]));
        }
        for (i, (_, v)) in values.iter().enumerate() {
            recs.extend(rec(i as u16 + 1, &v.to_le_bytes()));
        }
        recs.extend(rec(0, &[1, 2, 3, 4]));
        let mut b = Vec::new();
        for w in [VALV, 117, table.len() as u32, names.len() as u32, 0, recs.len() as u32] {
            b.extend(w.to_le_bytes());
        }
        b.extend(table);
        b.extend(recs);
        b
    }

    fn save(inner_a: &[u8], inner_b: &[u8]) -> Vec<u8> {
        let mut d = vec![0u8; FIRST];
        d[..4].copy_from_slice(b"JSAV");
        d[4..8].copy_from_slice(&117u32.to_le_bytes());
        let m = b"maps/sm_hub_1.bsp";
        d[100..100 + m.len()].copy_from_slice(m);
        for (name, data) in [("sm_apartment_1.HL1", inner_b), ("SM_HUB_1.HL1", inner_a)] {
            let mut n = [0u8; 260];
            n[..name.len()].copy_from_slice(name.as_bytes());
            d.extend(pack(&n, data));
        }
        d
    }

    const NAMES: [&str; 6] = [
        "m_iVAttributesBase[ v_attribute_strength ]",
        "m_iVAttributesCurrent[ v_attribute_strength ]",
        "m_iVAttributesCurrent[ v_attribute_experience ]",
        "m_iVAttributesBase[ v_attribute_experience ]",
        "m_iMoney",
        "m_iVDisciplinesBase[ v_discipline_celerity ]",
    ];

    fn sample() -> (Vec<u8>, Vec<u8>) {
        let values = [(NAMES[0], 3), (NAMES[1], 3), (NAMES[3], 8), (NAMES[2], 8), (NAMES[4], 1500), (NAMES[5], -1)];
        let inner = valv(&values);
        // a second copy (an older map) with other values must stay untouched
        let other = valv(&[(NAMES[3], 1), (NAMES[2], 1), (NAMES[4], 5)]);
        (save(&inner, &other), inner)
    }

    #[test]
    fn reads_edits_and_repacks() {
        let (d, _) = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/Money"), Some(&Value::from(1500)));
        assert_eq!(doc.view.pointer("/Attributes/Strength"), Some(&Value::from(3)));
        assert_eq!(doc.view.pointer("/Disciplines/Celerity"), Some(&Value::from(-1)));
        assert_eq!(doc.to_bytes().unwrap(), d, "an untouched save is written back unchanged");
        doc.set("/Character/Money", Value::from(777_777)).unwrap();
        doc.set("/Attributes/Strength", Value::from(5)).unwrap();
        doc.set("/Disciplines/Celerity", Value::from(4)).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Character/Money"), Some(&Value::from(777_777)));
        assert_eq!(again.view.pointer("/Attributes/Strength"), Some(&Value::from(5)));
        assert_eq!(again.view.pointer("/Disciplines/Celerity"), Some(&Value::from(4)));
        // both the base and the current copy changed, and the other map file is as it was
        let e = map_entry(&out).unwrap();
        let inner = inflate(&out, &e).unwrap();
        let v = Valv::new(&inner).unwrap();
        let run = v.player().unwrap();
        let s = |n: &str| run.iter().find(|(t, _)| Some(*t) == v.token(n)).map(|(_, o)| i32::from_le_bytes(inner[*o..*o + 4].try_into().unwrap()));
        assert_eq!(s(NAMES[0]), Some(5));
        assert_eq!(s(NAMES[1]), Some(5));
        let old = entries(&out).unwrap().into_iter().find(|e| e.name == "sm_apartment_1.HL1").unwrap();
        let (o_in, d_in) = (inflate(&out, &old).unwrap(), inflate(&d, &entries(&d).unwrap().into_iter().find(|e| e.name == "sm_apartment_1.HL1").unwrap()).unwrap());
        assert_eq!(o_in, d_in);
    }

    #[test]
    fn large_files_use_pieces() {
        let data: Vec<u8> = (0..CHUNK * 2 + 1000).map(|i| (i % 251) as u8).collect();
        let mut n = [0u8; 260];
        n[..6].copy_from_slice(b"x.HL1\0");
        let mut d = vec![0u8; FIRST];
        d.extend(pack(&n, &data));
        let e = entries(&d).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(inflate(&d, &e[0]).unwrap(), data);
    }

    #[test]
    fn other_files_are_refused() {
        assert!(!looks(b"JSAV"));
        assert!(parse(&vec![0u8; 20000]).is_err());
    }

    /// Run with `VTMB_SAVES=<folder of Vampire-NNN.sav>`: every save must read, and writing it back unchanged must repack to the same map file.
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("VTMB_SAVES") else { return };
        let mut seen = 0;
        for f in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(f.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let Ok(mut doc) = parse(&d) else {
                eprintln!("{:?}: no character sheet", f.file_name());
                continue;
            };
            let m = doc.view.pointer("/Character/Money").cloned();
            if m.is_none() {
                eprintln!("{:?}: no money record yet", f.file_name());
                continue;
            }
            doc.set("/Character/Money", Value::from(4242)).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out).unwrap();
            assert_eq!(again.view.pointer("/Character/Money"), Some(&Value::from(4242)), "{:?}", f.file_name());
            let (e0, e1) = (map_entry(&d).unwrap(), map_entry(&out).unwrap());
            let (a, b) = (inflate(&d, &e0).unwrap(), inflate(&out, &e1).unwrap());
            assert_eq!(a.len(), b.len());
            assert_eq!(entries(&d).unwrap().len(), entries(&out).unwrap().len());
            eprintln!("{:?}: money {:?} ok", f.file_name(), m);
            seen += 1;
        }
        assert!(seen > 0);
    }
}
