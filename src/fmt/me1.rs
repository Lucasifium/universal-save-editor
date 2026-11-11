//! Mass Effect (2008) `player.sav`, the main entry of a `.MassEffectSave` (an image header
//! and a zip; see `formats.rs`). It's an Unreal Engine 3 package: a name table, imports
//! (classes) and exports (saved objects), each export's data being property tags whose
//! names are indexes into the name table (name index, number, type index, number, size,
//! array index, value; structs add their struct name, bools are 4 bytes with size 0).
//!
//! The package layout follows Trilogy Save Editor (Karlitos, CeCILL 2.1; facts only).
//! Every value is changed in place.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

struct Pkg<'a> {
    d: &'a [u8],
    names: Vec<String>,
    /// (class name, data offset, data size) per export.
    exports: Vec<(String, usize, usize)>,
}

#[derive(Debug, Clone)]
struct Prop {
    name: String,
    ty: String,
    at: usize,
    size: usize,
    /// StructProperty: its tags; ArrayProperty of structs: one list per element.
    sub: Vec<Vec<Prop>>,
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn fstring(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = u32_at(d, at)? as i32;
    if n >= 0 {
        let n = n as usize;
        let s = d.get(at + 4..at + 4 + n)?;
        Some((s.iter().take_while(|&&c| c != 0).map(|&c| super::bethesda_cp1252(c)).collect(), at + 4 + n))
    } else {
        let n = 2 * n.unsigned_abs() as usize;
        let s = d.get(at + 4..at + 4 + n)?;
        let u: Vec<u16> = s.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&x| x != 0).collect();
        Some((String::from_utf16_lossy(&u), at + 4 + n))
    }
}

impl<'a> Pkg<'a> {
    fn open(d: &'a [u8]) -> Option<Self> {
        let h = u32_at(d, 8)? as usize;
        if u32_at(d, h)? != 0x9E2A_83C1 {
            return None;
        }
        let (_, p) = fstring(d, h + 12)?; // package name
        let f = |i: usize| u32_at(d, p + 4 + 4 * i).map(|v| v as usize);
        let (name_len, name_off, obj_len, obj_off, cls_len, cls_off) = (f(0)?, f(1)?, f(2)?, f(3)?, f(4)?, f(5)?);
        if name_len > 100_000 || obj_len > 100_000 || cls_len > 100_000 {
            return None;
        }
        let mut names = Vec::with_capacity(name_len);
        let mut q = name_off;
        for _ in 0..name_len {
            let (s, e) = fstring(d, q)?;
            names.push(s);
            q = e + 8;
        }
        let mut classes = Vec::with_capacity(cls_len);
        for i in 0..cls_len {
            classes.push(names.get(u32_at(d, cls_off + 28 * i + 20)? as usize)?.clone());
        }
        let mut exports = Vec::with_capacity(obj_len);
        for i in 0..obj_len {
            let e = obj_off + 72 * i;
            let cid = u32_at(d, e)? as i32;
            let class = if cid < 0 { classes.get((-cid - 1) as usize).cloned().unwrap_or_default() } else { String::new() };
            let (size, off) = (u32_at(d, e + 32)? as usize, u32_at(d, e + 36)? as usize);
            if off + size > d.len() {
                return None;
            }
            exports.push((class, off, size));
        }
        Some(Pkg { d, names, exports })
    }

    fn name(&self, at: usize) -> Option<&str> {
        self.names.get(u32_at(self.d, at)? as usize).map(String::as_str)
    }

    /// Property tags from `at` to "None" (within `limit`).
    fn tags(&self, at: usize, limit: usize, depth: usize) -> Option<(Vec<Prop>, usize)> {
        let mut out = vec![];
        let mut p = at;
        while p < limit {
            let name = self.name(p)?.to_string();
            if name == "None" {
                return Some((out, p + 8));
            }
            let ty = self.name(p + 8)?.to_string();
            if !ty.ends_with("Property") {
                return None;
            }
            let size = u32_at(self.d, p + 16)? as usize;
            let mut v = p + 24;
            if ty == "StructProperty" {
                v += 8;
            }
            let (size, end) = if ty == "BoolProperty" { (4, v + 4) } else { (size, v + size) };
            if end > limit {
                return None;
            }
            let mut sub = vec![];
            if depth < 6 {
                if ty == "StructProperty" {
                    if let Some((s, e)) = self.tags(v, end, depth + 1) {
                        if e == end {
                            sub.push(s);
                        }
                    }
                } else if ty == "ArrayProperty" && size > 4 {
                    let n = u32_at(self.d, v)? as usize;
                    let mut e = v + 4;
                    for _ in 0..n.min(10_000) {
                        match self.tags(e, end, depth + 1) {
                            Some((s, x)) => {
                                sub.push(s);
                                e = x;
                            }
                            None => {
                                sub.clear();
                                break;
                            }
                        }
                    }
                    if e != end {
                        sub.clear();
                    }
                }
            }
            out.push(Prop { name, ty, at: v, size, sub });
            p = end;
        }
        None
    }

    /// The tags of export `i` (0-based).
    fn object(&self, i: usize) -> Option<Vec<Prop>> {
        let (_, off, size) = self.exports.get(i)?;
        self.tags(off + 4, off + size, 0).map(|x| x.0)
    }
}

fn get<'p>(p: &'p [Prop], name: &str) -> Option<&'p Prop> {
    p.iter().find(|x| x.name == name)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 4096
        && u32_at(d, 8).is_some_and(|h| u32_at(d, h as usize) == Some(0x9E2A_83C1))
        && d.windows(18).any(|w| w == b"BioSquadSaveObject")
}

fn squad_name(tag: &str) -> String {
    match tag.to_lowercase().trim_start_matches("hench_") {
        "humanfemale" => "Ashley".into(),
        "humanmale" => "Kaidan".into(),
        "asari" => "Liara".into(),
        "turian" => "Garrus".into(),
        "krogan" => "Wrex".into(),
        "quarian" => "Tali".into(),
        "jenkins" => "Jenkins".into(),
        other => other.to_string(),
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let pkg = Pkg::open(d).ok_or("not a Mass Effect player package")?;
    let mut b = Builder::new(d);
    b.info(&["Save", "Game"], "Mass Effect (2008)");
    let int = |b: &mut Builder, path: &[&str], p: Option<&Prop>, hi: i64| {
        if let Some(p) = p.filter(|p| p.ty == "IntProperty" && p.size == 4) {
            b.ranged(path, p.at, Kind::I32, 0, hi);
        }
    };
    let float = |b: &mut Builder, path: &[&str], p: Option<&Prop>| {
        if let Some(p) = p.filter(|p| p.ty == "FloatProperty" && p.size == 4) {
            b.field(path, p.at, Kind::F32);
        }
    };
    let text = |p: Option<&Prop>| p.and_then(|p| if p.ty == "StrProperty" { fstring(d, p.at).map(|x| x.0) } else { pkg.name(p.at).map(str::to_string) });
    let objs: Vec<Option<Vec<Prop>>> = (0..pkg.exports.len()).map(|i| pkg.object(i)).collect();
    let of_class = |c: &'static str| pkg.exports.iter().enumerate().filter(move |(_, e)| e.0 == c).map(|(i, _)| i);

    let mut henchmen: Vec<(String, usize)> = vec![];
    if let Some(game) = of_class("BioSaveGame").next().and_then(|i| objs[i].as_ref()) {
        if let Some(m) = text(get(game, "m_sMapName")) {
            b.info(&["Save", "Location"], m);
        }
        if let Some(t) = get(game, "m_fPlaythroughTime").filter(|p| p.ty == "FloatProperty") {
            b.info(&["Save", "Seconds played"], f32::from_le_bytes(d[t.at..t.at + 4].try_into().unwrap()) as f64);
        }
        int(&mut b, &["Save", "Kills"], get(game, "m_nPlayerKills"), i32::MAX as i64);
        for el in get(game, "m_Henchmen").map(|p| p.sub.clone()).unwrap_or_default() {
            let tag = get(&el, "Tag").and_then(|p| pkg.name(p.at)).map(str::to_string);
            let save = get(&el, "Save").and_then(|p| u32_at(d, p.at)).map(|v| v as i32);
            if let (Some(t), Some(s)) = (tag, save) {
                if s > 0 && !t.eq_ignore_ascii_case("player") {
                    henchmen.push((squad_name(&t), s as usize - 1));
                }
            }
        }
    }
    if let Some(inv) = of_class("BioInventorySaveObject").next().and_then(|i| objs[i].as_ref()) {
        int(&mut b, &["Resources", "Credits"], get(inv, "m_nResourceCredits"), 9_999_999);
        int(&mut b, &["Resources", "Grenades"], get(inv, "m_nResourceGrenades"), 99);
        float(&mut b, &["Resources", "Medi-gel"], get(inv, "m_fResourceMedigel"));
        float(&mut b, &["Resources", "Omni-gel"], get(inv, "m_fResourceSalvage"));
    }
    if let Some(sq) = of_class("BioSquadSaveObject").next().and_then(|i| objs[i].as_ref()) {
        int(&mut b, &["Shepard", "Experience"], get(sq, "m_nSquadExperience"), i32::MAX as i64);
    }

    // A character: level, talent points and talent ranks. Shepard is saved twice (the
    // pawn and its copy for the squad screen): edits go to both.
    let character = |b: &mut Builder, base: &[&str], p: &[Prop], mirror: Option<&[Prop]>| {
        let mut one = |path: &[&str], name: &str, hi: i64| {
            if let Some(x) = get(p, name).filter(|x| x.ty == "IntProperty" && x.size == 4) {
                b.ranged(path, x.at, Kind::I32, 0, hi);
                if let Some(m) = mirror.and_then(|m| get(m, name)).filter(|m| m.ty == "IntProperty") {
                    b.mirror(path, m.at);
                }
            }
        };
        let mut lv: Vec<&str> = base.to_vec();
        lv.push("Level");
        one(&lv, "m_XPLevel", 60);
        let mut tp: Vec<&str> = base.to_vec();
        tp.push("Talent points");
        one(&tp, "m_TalentPoints", 999);
        let talents = |q: &[Prop]| get(q, "m_aComplexTalents").map(|t| t.sub.clone()).unwrap_or_default();
        let mine = talents(p);
        let theirs = mirror.map(talents).unwrap_or_default();
        for (i, t) in mine.iter().enumerate() {
            let Some(id) = get(t, "m_TalentID").and_then(|x| u32_at(d, x.at)) else { continue };
            let Some(rank) = get(t, "m_Ranks").filter(|x| x.ty == "IntProperty") else { continue };
            let max = get(t, "m_MaxRank").and_then(|x| u32_at(d, x.at)).unwrap_or(12).clamp(1, 20);
            let label = super::me1le::talent_name(id as i32);
            let mut path: Vec<&str> = if base.first() == Some(&"Shepard") { vec!["Powers"] } else { [base, &["Talents"]].concat() };
            path.push(&label);
            b.ranged(&path, rank.at, Kind::I32, 0, max as i64);
            if let Some(m) = theirs.get(i).and_then(|x| get(x, "m_Ranks")) {
                b.mirror(&path, m.at);
            }
        }
    };

    let behaviours: Vec<usize> = of_class("BioPawnBehaviorSaveObject").collect();
    let shepards: Vec<usize> = behaviours.iter().copied().filter(|&i| objs[i].as_ref().is_some_and(|p| get(p, "m_FirstName").is_some())).collect();
    if let Some(&s) = shepards.first() {
        let p = objs[s].as_ref().unwrap();
        if let Some(n) = text(get(p, "m_FirstName")) {
            b.info(&["Shepard", "First name"], n);
        }
        let mirror = shepards.get(1).and_then(|&m| objs[m].as_deref());
        character(&mut b, &["Shepard"], p, mirror);
    }
    for (name, i) in &henchmen {
        if let Some(p) = objs.get(*i).and_then(|o| o.as_ref()) {
            if pkg.exports[*i].0 == "BioPawnBehaviorSaveObject" {
                character(&mut b, &["Squad", name], p, None);
            }
        }
    }
    if shepards.is_empty() {
        return Err("no player data found".into());
    }
    Ok(b.finish("Mass Effect (2008) save", Checksum::None).with_note(
        "Shepard, resources and the squad from player.sav. Paragon and Renegade live in state.sav \
         (pick it from the archive's entries).",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs(o: &mut Vec<u8>, s: &str) {
        o.extend(((s.len() + 1) as i32).to_le_bytes());
        o.extend(s.as_bytes());
        o.push(0);
    }

    /// A tiny synthetic package: an inventory with credits and Shepard's pawn behaviour.
    fn sample() -> Vec<u8> {
        let names = ["None", "IntProperty", "StrProperty", "m_nResourceCredits", "BioInventorySaveObject", "BioPawnBehaviorSaveObject", "m_FirstName", "m_XPLevel", "BioSquadSaveObject", "m_nSquadExperience"];
        let n = |s: &str| names.iter().position(|x| *x == s).unwrap() as u32;
        let tag = |o: &mut Vec<u8>, name: &str, ty: &str, size: u32| {
            for v in [n(name), 0, n(ty), 0, size, 0] {
                o.extend(v.to_le_bytes());
            }
        };
        let mut objs: Vec<(u32, Vec<u8>)> = vec![];
        let mut inv = vec![0u8; 4];
        tag(&mut inv, "m_nResourceCredits", "IntProperty", 4);
        inv.extend(5000i32.to_le_bytes());
        inv.extend(n("None").to_le_bytes());
        inv.extend(0u32.to_le_bytes());
        objs.push((1, inv));
        let mut pawn = vec![0u8; 4];
        tag(&mut pawn, "m_XPLevel", "IntProperty", 4);
        pawn.extend(12i32.to_le_bytes());
        let mut name = vec![];
        fs(&mut name, "Jane");
        tag(&mut pawn, "m_FirstName", "StrProperty", name.len() as u32);
        pawn.extend(name);
        pawn.extend(n("None").to_le_bytes());
        pawn.extend(0u32.to_le_bytes());
        objs.push((2, pawn));
        let mut sq = vec![0u8; 4];
        tag(&mut sq, "m_nSquadExperience", "IntProperty", 4);
        sq.extend(777i32.to_le_bytes());
        sq.extend(n("None").to_le_bytes());
        sq.extend(0u32.to_le_bytes());
        objs.push((3, sq));

        let mut d = vec![0u8; 8];
        d.extend(12u32.to_le_bytes());
        let hdr = d.len();
        d.extend(0x9E2A_83C1u32.to_le_bytes());
        d.extend([0u8; 8]); // version, data offset
        fs(&mut d, "None");
        d.extend(0u32.to_le_bytes()); // flags
        let counts_at = d.len();
        d.extend([0u8; 24]);
        let name_off = d.len();
        for s in names {
            fs(&mut d, s);
            d.extend([0u8; 8]);
        }
        let cls_off = d.len();
        for c in ["BioInventorySaveObject", "BioPawnBehaviorSaveObject", "BioSquadSaveObject"] {
            let mut e = [0u8; 28];
            e[20..24].copy_from_slice(&n(c).to_le_bytes());
            d.extend(e);
        }
        let obj_off = d.len();
        let mut data_at = obj_off + 72 * objs.len();
        for (cls, data) in &objs {
            let mut e = [0u8; 72];
            e[0..4].copy_from_slice(&(-(*cls as i32)).to_le_bytes());
            e[32..36].copy_from_slice(&(data.len() as u32).to_le_bytes());
            e[36..40].copy_from_slice(&(data_at as u32).to_le_bytes());
            data_at += data.len();
            d.extend(e);
        }
        for (_, data) in &objs {
            d.extend(data);
        }
        for (i, v) in [names.len(), name_off, objs.len(), obj_off, 3, cls_off].iter().enumerate() {
            d[counts_at + 4 * i..counts_at + 4 * i + 4].copy_from_slice(&(*v as u32).to_le_bytes());
        }
        let _ = hdr;
        d.extend([0u8; 4096]);
        d
    }

    #[test]
    fn reads_and_edits() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Resources/Credits"), Some(&5000.into()));
        assert_eq!(doc.view.pointer("/Shepard/First name"), Some(&"Jane".into()));
        assert_eq!(doc.view.pointer("/Shepard/Level"), Some(&12.into()));
        assert_eq!(doc.view.pointer("/Shepard/Experience"), Some(&777.into()));
        doc.set("/Resources/Credits", 999_999.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(parse(&out).unwrap().view.pointer("/Resources/Credits"), Some(&999_999.into()));
    }

    /// Research aid: SAVE_EDITOR_ME1 = a player.sav taken out of a .MassEffectSave.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_ME1").unwrap()).unwrap();
        let doc = super::parse(&d).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view).unwrap());
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}

#[cfg(test)]
mod roundtrip {
    /// Research aid: SAVE_EDITOR_ME1_FULL = a .MassEffectSave; writes an edited copy to ME1_OUT.
    #[test]
    #[ignore]
    fn whole_file() {
        use crate::formats::{decode, encode, Content};
        let raw = std::fs::read(std::env::var("SAVE_EDITOR_ME1_FULL").unwrap()).unwrap();
        let mut d = decode(&raw);
        println!("{}", crate::formats::describe(&d.layers, &d.content));
        let Content::Tree(t) = &mut d.content else { panic!("not a tree") };
        t.set("/Resources/Credits", 1234567.into()).unwrap();
        let out = encode(&d.layers, &d.content).unwrap();
        std::fs::write(std::env::var("ME1_OUT").unwrap(), &out).unwrap();
        let again = decode(&out);
        let Content::Tree(t3) = &again.content else { panic!("re-read failed") };
        assert_eq!(t3.view.pointer("/Resources/Credits"), Some(&1234567.into()));
    }
}
