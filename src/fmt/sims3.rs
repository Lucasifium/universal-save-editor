//! The Sims 3 saves (`<Town>.sims3\<World>_0x….nhd`): a DBPF 2.0 package whose resource of
//! type `0x06B981ED` holds the game's script objects, packed with RefPack.
//!
//! That resource (unpacked) starts with `00 05 00 00`, `OBJS`, then: an unknown word, the
//! object count, the offset of the class table, the offset of a table with one u32 offset
//! per object, and the offset of a resource-key table. An object is `0x10`, its class number
//! (u32) and its fields in the order the class table lists them; a lone value starts with its
//! own type byte instead.
//!
//! Class table: per class a type name, a field count (7 bits per byte, high byte first, `7F`
//! for none) and per field a name and a type byte. A type name is a kind byte then: kind 0 or 4
//! a plain name (length, text); kind `0x2n` a generic type with n+1 arguments, each a type
//! name; kinds 1–3 an array of the type name that follows. Field types: 1 a reference to an
//! object (its number + 1, 0 for none), 2 bool, 3 byte, 5 date-time (8 bytes), 9 int32,
//! 10 int64, 12 float, 14 uint32, 15 uint64, 25 resource key (index, 4 bytes), 16 a struct
//! (u32 class number, then its fields), 23 an enum (class number in 7-bit groups, type byte of
//! its value, the value). Strings are objects of `System.String`: length (7 bits per byte, low
//! first) and UTF-8 text.
//!
//! The editor reads households (`Sims3.Gameplay.CAS.Household`: funds), Sims
//! (`Sims3.Gameplay.CAS.SimDescription`: names, lifetime happiness, lifetime reward points)
//! and skill objects (any class with `SkillLevel` and `mSkillOwner`, which points at the Sim).
//! The flat form is `S3FL` followed by the unpacked resource; every value is changed in place,
//! so its size never changes. On save the resource is stored uncompressed in the package.
//! Layout worked out from public saves.

use super::dbpf::qfs;
use super::fields::{Builder, Checksum, Kind};
use crate::i18n::tx;
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "The Sims 3 save";
const HEAD: usize = 96;
const OBJS: u32 = 0x06B9_81ED;
const FLAT: &[u8; 4] = b"S3FL";

/// Skills the editor offers (class name, as the game screen shows it, highest level). Only
/// skills seen at level 10 in real saves are listed; toddler, child and lesson skills and the
/// ones kept in the plain `Skill` class are left alone.
pub const SKILLS: &[(&str, &str)] = &[
    ("Athletic", tx("Athletic")),
    ("Charisma", tx("Charisma")),
    ("Cooking", tx("Cooking")),
    ("Fishing", tx("Fishing")),
    ("Gardening", tx("Gardening")),
    ("Guitar", tx("Guitar")),
    ("Handiness", tx("Handiness")),
    ("LogicSkill", tx("Logic")),
    ("PaintingSkill", tx("Painting")),
    ("Writing", tx("Writing")),
    ("Chess", tx("Chess")),
    ("Hacking", tx("Hacking")),
    ("Mooch", tx("Mooch")),
    ("Collecting", tx("Collecting")),
    ("Photography", tx("Photography")),
    ("MartialArts", tx("Martial Arts")),
    ("SnakeCharming", tx("Snake Charming")),
    ("NectarSkill", tx("Nectar Making")),
    ("ScubaDivingSkill", tx("Scuba Diving")),
    ("SculptingSkill", tx("Sculpting")),
    ("InventingSkill", tx("Inventing")),
    ("Consignment", tx("Consignment")),
    ("Piano", tx("Piano")),
    ("Bartending", tx("Mixology")),
    ("DrumSkill", tx("Drums")),
    ("BassGuitarSkill", tx("Bass")),
    ("DartSkill", tx("Darts")),
    ("BubblesSkill", tx("Bubble Blowing")),
    ("ArcadeMachineSkill", tx("Arcade Machines")),
    ("RidingSkill", tx("Riding")),
    ("GolfingSkill", tx("Golf")),
    ("GamblingSkill", tx("Gambling")),
    ("PoolSkill", tx("Pool")),
    ("Dominoes", tx("Dominoes")),
    ("MagicSkill", tx("Magic")),
    ("SpellcraftSkill", tx("Spellcraft")),
    ("SpellcastingSkill", tx("Spellcasting")),
    ("BroomRidingSkill", tx("Broom Riding")),
    ("StreetArtSkill", tx("Street Art")),
    ("ScienceSkill", tx("Science")),
    ("InfluenceSkillNerd", tx("Nerd Influence")),
    ("InfluenceSkillRebel", tx("Rebel Influence")),
    ("InfluenceSkillSocialite", tx("Socialite Influence")),
    ("SocialNetworkingSkill", tx("Social Networking")),
    ("BotBuildingSkill", tx("Bot Building")),
    ("FutureSkill", tx("Advanced Technology")),
    ("LaserHarpSkill", tx("Laser Rhythm-a-con")),
];
pub const SKILL_MAX: i64 = 10;

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o.checked_add(4)?)?.try_into().ok()?))
}

// ---- the package

struct Entry {
    /// Where the entry's own words start in the index (after the shared ones).
    at: usize,
    ty: u32,
    offset: usize,
    size: usize,
    mem: usize,
    comp: u16,
}

/// The index: its flags word, the words every entry shares, and the entries (each entry's
/// type comes from the shared words when bit 0 of the flags is set).
fn index(d: &[u8]) -> Option<(u32, Vec<Entry>, usize, usize)> {
    if d.len() < HEAD || &d[..4] != b"DBPF" || u32_at(d, 4)? != 2 || u32_at(d, 8)? != 0 {
        return None;
    }
    let (n, size, off) = (u32_at(d, 36)? as usize, u32_at(d, 44)? as usize, u32_at(d, 64)? as usize);
    if off.checked_add(size)? > d.len() {
        return None;
    }
    let flags = u32_at(d, off)?;
    if flags > 0xF {
        return None;
    }
    let shared = flags.count_ones() as usize;
    let own = 4 - shared;
    let per = own * 4 + 16;
    if size != 4 + shared * 4 + per * n {
        return None;
    }
    let shared_ty = if flags & 1 != 0 { u32_at(d, off + 4) } else { None };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let b = off + 4 + shared * 4 + per * i;
        let ty = match shared_ty {
            Some(t) => t,
            None => u32_at(d, b)?,
        };
        let w = b + own * 4;
        let e = Entry {
            at: b,
            ty,
            offset: u32_at(d, w)? as usize,
            size: (u32_at(d, w + 4)? & 0x7FFF_FFFF) as usize,
            mem: u32_at(d, w + 8)? as usize,
            comp: u16::from_le_bytes(d.get(w + 12..w + 14)?.try_into().ok()?),
        };
        if e.offset.checked_add(e.size)? > d.len() {
            return None;
        }
        out.push(e);
    }
    Some((flags, out, off, per))
}

fn body(d: &[u8], e: &Entry) -> Option<Vec<u8>> {
    let raw = d.get(e.offset..e.offset + e.size)?;
    match e.comp {
        0 => Some(raw.to_vec()),
        0xFFFF => qfs(raw),
        _ => None,
    }
}

pub fn looks_package(d: &[u8]) -> bool {
    index(d).is_some_and(|(_, es, _, _)| es.iter().any(|e| e.ty == OBJS && e.mem > 64))
}

pub fn flatten(d: &[u8]) -> Option<Vec<u8>> {
    let (_, es, _, _) = index(d)?;
    let b = body(d, es.iter().find(|e| e.ty == OBJS)?)?;
    if b.len() < 28 || &b[4..8] != b"OBJS" {
        return None;
    }
    let mut out = FLAT.to_vec();
    out.extend(b);
    Some(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 32 && d.starts_with(FLAT) && &d[8..12] == b"OBJS"
}

/// The package again with `flat`'s resource stored uncompressed (the original when unchanged).
pub fn rebuild(original: &[u8], flat: &[u8]) -> Result<Vec<u8>, String> {
    let (_, es, ioff, per) = index(original).ok_or("the package changed")?;
    let old = flatten(original).ok_or("the save data can't be read")?;
    if old == flat {
        return Ok(original.to_vec());
    }
    if old.len() != flat.len() || !looks(flat) {
        return Err("the save data is damaged".into());
    }
    let new = &flat[4..];
    let first = es.first().map_or(ioff, |e| e.at);
    let mut out = original[..HEAD].to_vec();
    let mut idx = original[ioff..first].to_vec();
    for x in &es {
        let (slice, mem, comp): (&[u8], usize, u16) =
            if x.ty == OBJS { (new, new.len(), 0) } else { (&original[x.offset..x.offset + x.size], x.mem, x.comp) };
        let off = out.len();
        out.extend_from_slice(slice);
        let mut ent = original[x.at..x.at + per].to_vec();
        let w = per - 16;
        let flag = u32_at(&ent, w + 4).unwrap_or(0) & 0x8000_0000;
        ent[w..w + 4].copy_from_slice(&(off as u32).to_le_bytes());
        ent[w + 4..w + 8].copy_from_slice(&(slice.len() as u32 | flag).to_le_bytes());
        ent[w + 8..w + 12].copy_from_slice(&(mem as u32).to_le_bytes());
        ent[w + 12..w + 14].copy_from_slice(&comp.to_le_bytes());
        idx.extend(ent);
    }
    let at = out.len();
    out.extend(idx);
    if out.len() > u32::MAX as usize {
        return Err("the save is too big".into());
    }
    out[64..68].copy_from_slice(&(at as u32).to_le_bytes());
    Ok(out)
}

// ---- the script objects

struct Class {
    name: String,
    fields: Vec<(String, u8)>,
}

/// 7 bits per byte, high byte first (class table).
fn vi_be(d: &[u8], p: &mut usize) -> Option<usize> {
    let mut v = 0usize;
    for _ in 0..5 {
        let b = *d.get(*p)?;
        *p += 1;
        v = (v << 7) | (b & 0x7F) as usize;
        if b < 0x80 {
            return Some(v);
        }
    }
    None
}

/// 7 bits per byte, low byte first (string lengths).
fn vi_le(d: &[u8], p: &mut usize) -> Option<usize> {
    let mut v = 0usize;
    for s in 0..5 {
        let b = *d.get(*p)?;
        *p += 1;
        v |= ((b & 0x7F) as usize) << (7 * s);
        if b < 0x80 {
            return Some(v);
        }
    }
    None
}

fn type_name(d: &[u8], p: &mut usize, depth: u32) -> Option<String> {
    if depth > 16 {
        return None;
    }
    let k = *d.get(*p)?;
    *p += 1;
    match k {
        0 | 4 => {
            let l = vi_be(d, p)?;
            let s = String::from_utf8_lossy(d.get(*p..p.checked_add(l)?)?).into_owned();
            *p += l;
            Some(s)
        }
        k if k & 0xE0 == 0x20 => {
            let base = type_name(d, p, depth + 1)?;
            let mut args = vec![];
            for _ in 0..=(k & 0x1F) {
                args.push(type_name(d, p, depth + 1)?);
            }
            Some(format!("{base}<{}>", args.join(",")))
        }
        1..=3 => Some(format!("{}[]", type_name(d, p, depth + 1)?)),
        _ => None,
    }
}

fn classes(d: &[u8], from: usize, to: usize) -> Option<Vec<Class>> {
    let mut p = from;
    let mut out = vec![];
    while p < to {
        let name = type_name(d, &mut p, 0)?;
        let mut n = vi_be(d, &mut p)?;
        if n == 0x7F {
            n = 0;
        }
        let mut fields = Vec::with_capacity(n);
        for _ in 0..n {
            let l = vi_be(d, &mut p)?;
            let f = String::from_utf8_lossy(d.get(p..p.checked_add(l)?)?).into_owned();
            p += l;
            fields.push((f, *d.get(p)?));
            p += 1;
        }
        out.push(Class { name, fields });
    }
    (p == to).then_some(out)
}

fn plain_size(t: u8) -> Option<usize> {
    Some(match t {
        2 | 3 => 1,
        1 | 9 | 12 | 14 | 25 => 4,
        5 | 10 | 15 => 8,
        _ => return None,
    })
}

/// Skips one value of type `t` at `p`.
fn skip(d: &[u8], p: &mut usize, t: u8, cls: &[Class], depth: u32) -> Option<()> {
    if depth > 16 {
        return None;
    }
    if let Some(n) = plain_size(t) {
        *p += n;
        return (*p <= d.len()).then_some(());
    }
    match t {
        16 => {
            let c = cls.get(u32_at(d, *p)? as usize)?;
            *p += 4;
            for (_, ft) in &c.fields {
                skip(d, p, *ft, cls, depth + 1)?;
            }
            Some(())
        }
        23 => {
            vi_be(d, p)?;
            let u = *d.get(*p)?;
            *p += 1;
            let n = match u {
                3 => 1,
                9 | 14 => 4,
                10 | 15 => 8,
                _ => return None,
            };
            *p += n;
            (*p <= d.len()).then_some(())
        }
        _ => None,
    }
}

/// An object's fields as (name, offset), when the whole object reads back to its exact end.
fn fields_of(d: &[u8], a: usize, b: usize, cls: &[Class]) -> Option<Vec<(String, usize)>> {
    if *d.get(a)? != 0x10 {
        return None;
    }
    let c = u32_at(d, a + 1)? as usize;
    let class = cls.get(c)?;
    let mut p = a + 5;
    let mut all = vec![];
    for (f, t) in &class.fields {
        all.push((f.clone(), p));
        skip(d, &mut p, *t, cls, 0)?;
    }
    (p == b).then_some(all)
}

fn string(d: &[u8], a: usize, b: usize) -> Option<String> {
    let mut p = a + 5;
    let l = vi_le(d, &mut p)?;
    (p + l == b).then(|| String::from_utf8_lossy(&d[p..b]).into_owned())
}

/// A name key the game looks up ("Gameplay/Excel/PV/Sims:Bella") as its last part.
fn shown_name(s: &str) -> String {
    match s.rsplit_once(':') {
        Some((k, v)) if k.contains('/') => v.to_string(),
        _ => s.to_string(),
    }
}

struct Sim {
    name: String,
    house: usize,
    happiness: usize,
    points: usize,
    total: f32,
    skills: Vec<(&'static str, usize)>,
}

pub fn parse(f: &[u8]) -> Result<TreeDoc, String> {
    let bad = || "damaged Sims 3 data".to_string();
    let d = f.get(4..).ok_or_else(bad)?;
    let w = |o: usize| u32_at(d, o).map(|v| v as usize).ok_or_else(bad);
    let (n, ctab, otab, keys) = (w(12)?, w(16)?, w(20)?, w(24)?);
    if ctab > keys || otab > ctab || keys > d.len() || otab.checked_add(n.checked_mul(4).ok_or_else(bad)?).ok_or_else(bad)? > ctab {
        return Err(bad());
    }
    let cls = classes(d, ctab, keys).ok_or_else(bad)?;
    let offs: Vec<usize> = (0..n).map(|i| u32_at(d, otab + 4 * i).unwrap_or(0) as usize).collect();
    let end = |i: usize| if i + 1 < n { offs[i + 1] } else { otab };
    let find = |name: &str| cls.iter().position(|c| c.name == name);
    let household = find("Sims3.Gameplay.CAS.Household");
    let simdesc = find("Sims3.Gameplay.CAS.SimDescription");
    let text = find("System.String");
    let skill_class: HashMap<usize, &'static str> = cls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.fields.iter().any(|(f, _)| f == "SkillLevel") && c.fields.iter().any(|(f, _)| f == "mSkillOwner"))
        .filter_map(|(i, c)| {
            let short = c.name.rsplit('.').next().unwrap_or("");
            SKILLS.iter().find(|(k, _)| *k == short).map(|(_, v)| (i, *v))
        })
        .collect();
    let str_at = |r: u32| -> String {
        let i = r as usize;
        if i == 0 || i > n {
            return String::new();
        }
        let (a, b) = (offs[i - 1], end(i - 1));
        if a + 5 > b || b > d.len() || text.is_none_or(|t| u32_at(d, a + 1) != Some(t as u32)) {
            return String::new();
        }
        string(d, a, b).unwrap_or_default()
    };
    let at = |all: &[(String, usize)], name: &str| all.iter().find(|(f, _)| f == name).map(|(_, p)| *p);
    // Households (object number -> name, funds offset), Sims, skills by owner.
    let mut houses: Vec<(usize, String, usize)> = vec![];
    let mut sims: HashMap<usize, Sim> = HashMap::new();
    let mut order: Vec<usize> = vec![];
    let mut skills: Vec<(usize, &'static str, usize)> = vec![];
    for i in 0..n {
        let (a, b) = (offs[i], end(i));
        if a + 5 > b || b > d.len() || d[a] != 0x10 {
            continue;
        }
        let c = u32_at(d, a + 1).unwrap_or(u32::MAX) as usize;
        let wanted = Some(c) == household || Some(c) == simdesc || skill_class.contains_key(&c);
        if !wanted {
            continue;
        }
        let Some(all) = fields_of(d, a, b, &cls) else { continue };
        let r = |name: &str| at(&all, name).and_then(|p| u32_at(d, p)).unwrap_or(0);
        if Some(c) == household {
            if let Some(m) = at(&all, "mFamilyFunds") {
                houses.push((i, str_at(r("mName")), m));
            }
        } else if Some(c) == simdesc {
            let (Some(h), Some(p)) = (at(&all, "mLifetimeHappiness"), at(&all, "mSpendableHappiness")) else { continue };
            let name = format!("{} {}", shown_name(&str_at(r("mFirstName"))), shown_name(&str_at(r("mLastName")))).trim().to_string();
            let total = f32::from_bits(u32_at(d, h).unwrap_or(0));
            sims.insert(i, Sim { name, house: r("mHousehold") as usize, happiness: h, points: p, total, skills: vec![] });
            order.push(i);
        } else if let Some(l) = at(&all, "SkillLevel") {
            skills.push((r("mSkillOwner") as usize, skill_class[&c], l));
        }
    }
    for (owner, name, l) in skills {
        if let Some(s) = owner.checked_sub(1).and_then(|o| sims.get_mut(&o)) {
            if !s.skills.iter().any(|(k, _)| *k == name) {
                s.skills.push((name, l));
            }
        }
    }
    // Households with Sims in them, the most-played first (by their Sims' lifetime happiness).
    let mut shown: Vec<(f64, String, usize, Vec<usize>)> = vec![];
    for (i, name, money) in &houses {
        if name.contains("ServiceNpc") {
            continue;
        }
        let members: Vec<usize> = order.iter().copied().filter(|s| sims[s].house == i + 1).collect();
        if members.is_empty() {
            continue;
        }
        let score: f64 = members.iter().map(|s| f64::from(sims[s].total)).sum();
        shown.push((score, shown_name(name), *money, members));
    }
    shown.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut b = Builder::new(f);
    let mut used = std::collections::HashSet::new();
    for (k, (_, name, money, members)) in shown.iter().enumerate() {
        let mut label = if name.is_empty() { format!("Household {}", k + 1) } else { name.clone() };
        if !used.insert(label.clone()) {
            label = format!("{label} ({})", k + 1);
            used.insert(label.clone());
        }
        b.ranged(&["Households", &label, "Funds"], money + 4, Kind::I32, 0, 99_999_999);
        let mut names = std::collections::HashSet::new();
        for (j, s) in members.iter().enumerate() {
            let s = &sims[s];
            let mut who = if s.name.is_empty() { format!("Sim {}", j + 1) } else { s.name.clone() };
            if !names.insert(who.clone()) {
                who = format!("{who} ({})", j + 1);
                names.insert(who.clone());
            }
            let base = ["Households", &label, "Sims", &who];
            b.ranged(&[&base[..], &["Lifetime happiness"]].concat(), s.happiness + 4, Kind::F32Int, 0, 99_999_999);
            b.ranged(&[&base[..], &["Lifetime reward points"]].concat(), s.points + 4, Kind::F32Int, 0, 99_999_999);
            let mut sk = s.skills.clone();
            sk.sort_by_key(|(k, _)| SKILLS.iter().position(|(_, v)| v == k));
            for (k, l) in sk {
                b.ranged(&[&base[..], &["Skills", k]].concat(), l + 4, Kind::I32, 0, SKILL_MAX);
            }
        }
    }
    if b.is_empty() {
        return Err("no households found".into());
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("The game data is saved back uncompressed, so the file grows. Close the game first.")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn name(out: &mut Vec<u8>, s: &str) {
        out.push(0);
        out.push(s.len() as u8);
        out.extend(s.as_bytes());
    }

    fn class(out: &mut Vec<u8>, n: &str, fields: &[(&str, u8)]) {
        name(out, n);
        out.push(if fields.is_empty() { 0x7F } else { fields.len() as u8 });
        for (f, t) in fields {
            out.push(f.len() as u8);
            out.extend(f.as_bytes());
            out.push(*t);
        }
    }

    /// A tiny OBJS resource: one household, one Sim with a skill, their names.
    fn objs() -> Vec<u8> {
        let mut ct = vec![];
        class(&mut ct, "System.String", &[]);
        class(&mut ct, "Sims3.Gameplay.CAS.Household", &[("mHouseholdId", 15), ("mFamilyFunds", 9), ("mName", 1), ("mExportTime", 5)]);
        class(&mut ct, "Sims3.Gameplay.Utilities.DateAndTime", &[("Ticks", 10)]);
        class(
            &mut ct,
            "Sims3.Gameplay.CAS.SimDescription",
            &[("mLifetimeHappiness", 12), ("mSpendableHappiness", 12), ("mFlags", 23), ("mHousehold", 1), ("mFirstName", 1), ("mLastName", 1), ("When", 16)],
        );
        class(&mut ct, "Sims3.Gameplay.Skills.Cooking", &[("SkillLevel", 9), ("SkillPoints", 12), ("mSkillOwner", 1)]);
        let mut objs: Vec<Vec<u8>> = vec![];
        let obj = |c: u32, body: &[u8]| {
            let mut o = vec![0x10];
            o.extend(c.to_le_bytes());
            o.extend(body);
            o
        };
        let s = |t: &str| {
            let mut v = vec![t.len() as u8];
            v.extend(t.as_bytes());
            obj(0, &v)
        };
        // 1: household, 2: its name, 3: Sim, 4: first, 5: last, 6: skill
        let mut h = 7u64.to_le_bytes().to_vec();
        h.extend(20_000i32.to_le_bytes());
        h.extend(2u32.to_le_bytes());
        h.extend([0; 8]);
        objs.push(obj(1, &h));
        objs.push(s("Gameplay/Excel/PV/Households:Goth"));
        let mut sd = 5000f32.to_le_bytes().to_vec();
        sd.extend(1200f32.to_le_bytes());
        sd.extend([0x81, 0x02, 0x0F]);
        sd.extend(0u64.to_le_bytes());
        sd.extend(1u32.to_le_bytes());
        sd.extend(4u32.to_le_bytes());
        sd.extend(5u32.to_le_bytes());
        sd.extend(2u32.to_le_bytes());
        sd.extend(9u64.to_le_bytes());
        objs.push(obj(3, &sd));
        objs.push(s("Gameplay/Excel/PV/Sims:Bella"));
        objs.push(s("Gameplay/Excel/PV/Sims:Goth"));
        let mut sk = 3i32.to_le_bytes().to_vec();
        sk.extend(4000f32.to_le_bytes());
        sk.extend(3u32.to_le_bytes());
        objs.push(obj(4, &sk));
        let mut d = vec![0, 5, 0, 0];
        d.extend(b"OBJS");
        d.extend(0u32.to_le_bytes());
        d.extend((objs.len() as u32).to_le_bytes());
        d.extend([0; 12]);
        let mut offs = vec![];
        for o in &objs {
            offs.push(d.len() as u32);
            d.extend(o);
        }
        let otab = d.len();
        for o in offs {
            d.extend(o.to_le_bytes());
        }
        let ctab = d.len();
        d.extend(&ct);
        let keys = d.len();
        d.extend(0u32.to_le_bytes());
        d[16..20].copy_from_slice(&(ctab as u32).to_le_bytes());
        d[20..24].copy_from_slice(&(otab as u32).to_le_bytes());
        d[24..28].copy_from_slice(&(keys as u32).to_le_bytes());
        d
    }

    fn package() -> Vec<u8> {
        let body = objs();
        let other = b"other resource".to_vec();
        let mut d = b"DBPF".to_vec();
        d.extend([2, 0, 0, 0, 0, 0, 0, 0]);
        d.resize(HEAD, 0);
        let mut idx = 0u32.to_le_bytes().to_vec();
        for (ty, data) in [(0x1234u32, &other), (OBJS, &body)] {
            let off = d.len();
            d.extend(data);
            let mut e = vec![0u8; 32];
            e[0..4].copy_from_slice(&ty.to_le_bytes());
            e[16..20].copy_from_slice(&(off as u32).to_le_bytes());
            e[20..24].copy_from_slice(&(data.len() as u32 | 0x8000_0000).to_le_bytes());
            e[24..28].copy_from_slice(&(data.len() as u32).to_le_bytes());
            idx.extend(e);
        }
        let ioff = d.len();
        d[36..40].copy_from_slice(&2u32.to_le_bytes());
        d[44..48].copy_from_slice(&(idx.len() as u32).to_le_bytes());
        d[64..68].copy_from_slice(&(ioff as u32).to_le_bytes());
        d.extend(idx);
        d
    }

    #[test]
    fn reads_edits_and_rebuilds() {
        let d = package();
        assert!(looks_package(&d));
        let f = flatten(&d).unwrap();
        assert!(looks(&f));
        let mut doc = parse(&f).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Households/Goth/Funds"), Some(Value::from(20_000)));
        assert_eq!(v(&doc, "/Households/Goth/Sims/Bella Goth/Lifetime reward points"), Some(Value::from(1200)));
        assert_eq!(v(&doc, "/Households/Goth/Sims/Bella Goth/Lifetime happiness"), Some(Value::from(5000)));
        assert_eq!(v(&doc, "/Households/Goth/Sims/Bella Goth/Skills/Cooking"), Some(Value::from(3)));
        assert_eq!(rebuild(&d, &doc.to_bytes().unwrap()).unwrap(), d);
        doc.set("/Households/Goth/Funds", Value::from(1_000_000)).unwrap();
        doc.set("/Households/Goth/Sims/Bella Goth/Lifetime reward points", Value::from(50_000)).unwrap();
        doc.set("/Households/Goth/Sims/Bella Goth/Skills/Cooking", Value::from(10)).unwrap();
        assert!(doc.set("/Households/Goth/Sims/Bella Goth/Skills/Cooking", Value::from(11)).is_err());
        let out = rebuild(&d, &doc.to_bytes().unwrap()).unwrap();
        // The other resource is kept byte for byte.
        let (_, es, _, _) = index(&out).unwrap();
        let o = es.iter().find(|e| e.ty == 0x1234).unwrap();
        assert_eq!(&out[o.offset..o.offset + o.size], b"other resource");
        let again = parse(&flatten(&out).unwrap()).unwrap();
        assert_eq!(v(&again, "/Households/Goth/Funds"), Some(Value::from(1_000_000)));
        assert_eq!(v(&again, "/Households/Goth/Sims/Bella Goth/Lifetime reward points"), Some(Value::from(50_000)));
        assert_eq!(v(&again, "/Households/Goth/Sims/Bella Goth/Skills/Cooking"), Some(Value::from(10)));
    }

    /// Research aid: SAVE_EDITOR_SIMS3 = `.nhd` files separated by `|`.
    #[test]
    #[ignore]
    fn real_files() {
        for path in std::env::var("SAVE_EDITOR_SIMS3").unwrap().split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            assert!(looks_package(&d));
            let f = flatten(&d).unwrap();
            let mut doc = parse(&f).unwrap();
            let hs = doc.view["Households"].as_object().unwrap().clone();
            println!("{path}: {} households", hs.len());
            for (k, v) in hs.iter().take(4) {
                println!("  {k}: {}  sims {}", v["Funds"], v["Sims"].as_object().map_or(0, |m| m.len()));
            }
            assert_eq!(rebuild(&d, &doc.to_bytes().unwrap()).unwrap(), d);
            let first = hs.keys().next().unwrap().clone();
            doc.set(&format!("/Households/{first}/Funds"), Value::from(1_234_567)).unwrap();
            let out = rebuild(&d, &doc.to_bytes().unwrap()).unwrap();
            let again = parse(&flatten(&out).unwrap()).unwrap();
            assert_eq!(again.view["Households"][&first]["Funds"], Value::from(1_234_567));
            let (_, a, _, _) = index(&d).unwrap();
            let (_, b, _, _) = index(&out).unwrap();
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(&b) {
                assert_eq!(x.ty, y.ty);
                if x.ty != OBJS {
                    assert_eq!(&d[x.offset..x.offset + x.size], &out[y.offset..y.offset + y.size]);
                }
            }
        }
    }
}
