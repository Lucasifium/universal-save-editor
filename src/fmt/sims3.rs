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

/// Skills the editor offers: the skill's id (its `mSkillGuid`), the name the game shows and
/// its highest level. Ids are matched to names through the skill classes found in saves and
/// the public skill tuning lists; skills whose id couldn't be named are left out.
pub const SKILLS: &[(u64, &str, i64)] = &[
    (2, tx("Athletic"), 10),
    (3, tx("Charisma"), 10),
    (5, tx("Cooking"), 10),
    (6, tx("Dancing"), 3),
    (7, tx("Diving"), 5),
    (8, tx("Fishing"), 10),
    (9, tx("Gardening"), 10),
    (10, tx("Guitar"), 10),
    (11, tx("Handiness"), 10),
    (12, tx("Logic"), 10),
    (13, tx("Painting"), 10),
    (15, tx("Potty Training"), 1),
    (16, tx("Talking"), 1),
    (17, tx("Walking"), 1),
    (18, tx("Chess"), 10),
    (19, tx("Hacking"), 10),
    (20, tx("Video Gaming"), 10),
    (21, tx("Writing"), 10),
    (22, tx("Mooch"), 10),
    (23, tx("Xylophone"), 3),
    (24, tx("Homework"), 10),
    (25, tx("Peg Box"), 3),
    (26, tx("Air Guitar"), 10),
    (27, tx("Blocks"), 3),
    (28, tx("Child Cooking"), 3),
    (29, tx("Collecting"), 10),
    (30, tx("Photography"), 10),
    (31, tx("Foosball"), 10),
    (32, tx("Martial Arts"), 10),
    (33, tx("Snake Charming"), 10),
    (34, tx("Nectar Making"), 10),
    (35, tx("Tattooing"), 10),
    (36, tx("Trampoline"), 10),
    (37, tx("Sculpting"), 10),
    (38, tx("Gnubb"), 10),
    (39, tx("Inventing"), 10),
    (40, tx("Consignment"), 10),
    (41, tx("Styling"), 10),
    (48, tx("Piano"), 10),
    (49, tx("Club Dancing"), 3),
    (50, tx("Shuffleboard"), 10),
    (51, tx("Mixology"), 10),
    (52, tx("Rock Band"), 1),
    (53, tx("Bass"), 10),
    (54, tx("Drums"), 10),
    (55, tx("Darts"), 10),
    (56, tx("Bubble Blowing"), 10),
    (57, tx("Arcade Machines"), 10),
    (64, tx("Child Gardening"), 3),
    (65, tx("Child Athletic"), 3),
    (66, tx("Child Guitar"), 3),
    (67, tx("Child Piano"), 3),
    (68, tx("Child Bass"), 3),
    (69, tx("Child Drums"), 3),
    (70, tx("Sandbox"), 4),
    (71, tx("Learning to Drive"), 3),
    (72, tx("Ballet"), 3),
    (73, tx("Scouting"), 3),
    (80, tx("Hopscotch"), 10),
    (81, tx("Riding"), 10),
    (82, tx("Jumping"), 10),
    (83, tx("Racing"), 10),
    (84, tx("Wildlife Friend"), 6),
    (85, tx("Tricks"), 5),
    (86, tx("Cat Hunting"), 10),
    (87, tx("Dog Hunting"), 10),
    (88, tx("Golf"), 10),
    (89, tx("Gambling"), 10),
    (96, tx("Acrobatics"), 10),
    (97, tx("Magic"), 10),
    (98, tx("Singing"), 10),
    (99, tx("Pool"), 10),
    (100, tx("Karaoke"), 3),
    (101, tx("DJ Turntable"), 5),
    (102, tx("Dominoes"), 10),
    (103, tx("Alchemy"), 10),
    (104, tx("Lycanthropy"), 10),
    (105, tx("Broom Riding"), 10),
    (112, tx("Spellcasting"), 10),
    (113, tx("Fairy Magic"), 10),
    (120, tx("Street Art"), 10),
    (121, tx("Science"), 10),
    (128, tx("Nerd Influence"), 10),
    (129, tx("Rebel Influence"), 10),
    (130, tx("Socialite Influence"), 10),
    (131, tx("Social Networking"), 10),
    (132, tx("Bot Building"), 10),
    (133, tx("Advanced Technology"), 10),
    (134, tx("Laser Rhythm-a-con"), 10),
    (144, tx("Scuba Diving"), 10),
];

/// Needs (motives) shown: the commodity id and its name. Values go from -100 to 100.
pub const NEEDS: &[(u32, &str)] = &[
    (0x010F_38D7, tx("Hunger")),
    (0x010F_38D8, tx("Energy")),
    (0x010F_38D6, tx("Bladder")),
    (0x010F_38D9, tx("Hygiene")),
    (0x010F_38E0, tx("Social")),
    (0x010F_38DA, tx("Fun")),
    (0x0126_B27F, tx("Thirst")),
    (0x0159_68DD, tx("Battery")),
    (0x0159_68E4, tx("Maintenance")),
    (0x01A9_8DA1, tx("Scratch")),
    (0x01A9_8DA0, tx("Destruction")),
    (0x01A9_8DA2, tx("Exercise")),
    (0x01A9_8DA3, tx("Horse thirst")),
];

/// Highest level of a skill shown under `name`.
pub fn skill_max(name: &str) -> i64 {
    SKILLS.iter().find(|(_, n, _)| *n == name).map_or(10, |s| s.2)
}

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

/// The value of an enum field at `p` (class number, type byte, value).
fn enum_at(d: &[u8], p: usize) -> Option<u64> {
    let mut q = p;
    vi_be(d, &mut q)?;
    let t = *d.get(q)?;
    q += 1;
    Some(match t {
        3 => u64::from(*d.get(q)?),
        9 | 14 => u64::from(u32_at(d, q)?),
        10 | 15 => u64::from_le_bytes(d.get(q..q + 8)?.try_into().ok()?),
        _ => return None,
    })
}

#[derive(Default)]
struct Sim {
    name: String,
    house: usize,
    actor: usize,
    happiness: usize,
    points: usize,
    total: f32,
    career: usize,
    celebrity: usize,
    skills: Vec<(usize, usize)>,
    needs: Vec<(usize, usize, usize)>,
    job: Option<(usize, i64)>,
    school: Option<(usize, i64)>,
    fame: Option<(usize, usize)>,
    occult_ref: usize,
    occult: Option<u64>,
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
    let has = |c: &Class, f: &str| c.fields.iter().any(|(x, _)| x == f);
    let household = find("Sims3.Gameplay.CAS.Household");
    let simdesc = find("Sims3.Gameplay.CAS.SimDescription");
    let motives = find("Sims3.Gameplay.Autonomy.Motives");
    let text = find("System.String");
    let skill_class: Vec<bool> = cls.iter().map(|c| has(c, "SkillLevel") && has(c, "mSkillOwner") && has(c, "mSkillGuid")).collect();
    // The class and fields of object `r - 1` (a reference).
    let object = |r: usize| -> Option<(usize, Vec<(String, usize)>)> {
        let i = r.checked_sub(1)?;
        if i >= n {
            return None;
        }
        let (a, b) = (offs[i], end(i));
        if a + 5 > b || b > d.len() {
            return None;
        }
        Some((u32_at(d, a + 1)? as usize, fields_of(d, a, b, &cls)?))
    };
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
    let refv = |all: &[(String, usize)], name: &str| at(all, name).and_then(|p| u32_at(d, p)).unwrap_or(0) as usize;
    let mut houses: Vec<(usize, String, usize)> = vec![];
    let mut sims: HashMap<usize, Sim> = HashMap::new();
    let mut order: Vec<usize> = vec![];
    let mut skills: Vec<(usize, u64, usize)> = vec![];
    let mut needs_of: Vec<(usize, Vec<(usize, usize, usize)>)> = vec![];
    for i in 0..n {
        let (a, b) = (offs[i], end(i));
        if a + 5 > b || b > d.len() || d[a] != 0x10 {
            continue;
        }
        let c = u32_at(d, a + 1).unwrap_or(u32::MAX) as usize;
        let is_skill = skill_class.get(c).copied().unwrap_or(false);
        if !(Some(c) == household || Some(c) == simdesc || Some(c) == motives || is_skill) {
            continue;
        }
        let Some(all) = fields_of(d, a, b, &cls) else { continue };
        let r = |name: &str| refv(&all, name);
        if Some(c) == household {
            if let Some(m) = at(&all, "mFamilyFunds") {
                houses.push((i, str_at(r("mName") as u32), m));
            }
        } else if Some(c) == simdesc {
            let (Some(h), Some(p)) = (at(&all, "mLifetimeHappiness"), at(&all, "mSpendableHappiness")) else { continue };
            let name = format!("{} {}", shown_name(&str_at(r("mFirstName") as u32)), shown_name(&str_at(r("mLastName") as u32))).trim().to_string();
            let total = f32::from_bits(u32_at(d, h).unwrap_or(0));
            sims.insert(
                i,
                Sim { name, house: r("mHousehold"), actor: r("mSim"), happiness: h, points: p, total, career: r("CareerManager"), celebrity: r("CelebrityManager"), occult_ref: r("OccultManager"), ..Default::default() },
            );
            order.push(i);
        } else if Some(c) == motives {
            // Motives -> Dictionary<int, Motive>: count (7 bits per byte, high first), then key and reference pairs.
            let sim = r("mSim");
            let di = r("mMotives");
            let mut list = vec![];
            if let Some(j) = di.checked_sub(1).filter(|j| *j < n) {
                let (da, db) = (offs[j], end(j));
                let mut p = da + 5;
                if da + 5 <= db && db <= d.len() && d[da] == 0x10 {
                    if let Some(k) = vi_be(d, &mut p) {
                        if p + 8 * k == db {
                            for e in 0..k {
                                let key = u32_at(d, p + 8 * e).unwrap_or(0);
                                let mr = u32_at(d, p + 8 * e + 4).unwrap_or(0) as usize;
                                if let Some(x) = NEEDS.iter().position(|(id, _)| *id == key) {
                                    if let Some((_, mf)) = object(mr) {
                                        if let (Some(v), Some(u)) = (at(&mf, "mValue"), at(&mf, "mUIValue")) {
                                            list.push((x, v, u));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            needs_of.push((sim, list));
        } else if let (Some(l), Some(g)) = (at(&all, "SkillLevel"), at(&all, "mSkillGuid")) {
            if let Some(id) = enum_at(d, g) {
                if SKILLS.iter().any(|s| s.0 == id) {
                    skills.push((r("mSkillOwner"), id, l));
                }
            }
        }
    }
    let by_actor: HashMap<usize, usize> = sims.iter().filter(|(_, s)| s.actor != 0).map(|(k, s)| (s.actor, *k)).collect();
    for (actor, list) in needs_of {
        if let Some(s) = by_actor.get(&actor).and_then(|k| sims.get_mut(k)) {
            s.needs = list;
        }
    }
    for (owner, id, l) in skills {
        if let Some(s) = owner.checked_sub(1).and_then(|o| sims.get_mut(&o)) {
            let x = SKILLS.iter().position(|s| s.0 == id).unwrap_or(0);
            if !s.skills.iter().any(|(k, _)| *k == x) {
                s.skills.push((x, l));
            }
        }
    }
    // Job, school and fame of each Sim.
    let perf = |r: usize| -> Option<(usize, i64)> {
        let (_, f) = object(r)?;
        let p = at(&f, "mPerformance")?;
        let lvl = at(&f, "mCurLevelVal").and_then(|q| u32_at(d, q)).map_or(0, |v| v as i32 as i64);
        Some((p, lvl))
    };
    for s in sims.values_mut() {
        if let Some((_, cm)) = object(s.career) {
            s.job = perf(refv(&cm, "mJob"));
            s.school = perf(refv(&cm, "mSchool"));
        }
        if let Some((_, om)) = object(s.occult_ref) {
            s.occult = at(&om, "mCurrentOccultTypes").and_then(|p| enum_at(d, p));
        }
        if let Some((_, ce)) = object(s.celebrity) {
            if let (Some(l), Some(p)) = (at(&ce, "mLevel"), at(&ce, "mPoints")) {
                s.fame = Some((l, p));
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
            let path = |rest: &[&str]| -> Vec<String> { base.iter().chain(rest.iter()).map(|x| x.to_string()).collect() };
            let go = |b: &mut Builder, rest: &[&str], at: usize, kind: Kind, lo: i64, hi: i64| {
                let p = path(rest);
                let p: Vec<&str> = p.iter().map(String::as_str).collect();
                b.ranged(&p, at + 4, kind, lo, hi);
            };
            go(&mut b, &["Lifetime happiness"], s.happiness, Kind::F32Int, 0, 99_999_999);
            go(&mut b, &["Lifetime reward points"], s.points, Kind::F32Int, 0, 99_999_999);
            if let Some((l, p)) = s.fame {
                go(&mut b, &["Celebrity level"], l, Kind::U32, 0, 5);
                go(&mut b, &["Celebrity points"], p, Kind::U32, 0, 99_999_999);
            }
            if let Some(o) = s.occult {
                b.info(&[&base[..], &["Supernatural"]].concat(), o);
            }
            if let Some((p, lvl)) = s.job {
                go(&mut b, &["Job performance"], p, Kind::F32Int, -100, 100);
                b.info(&[&base[..], &["Job level"]].concat(), lvl);
            }
            if let Some((p, _)) = s.school {
                go(&mut b, &["School performance"], p, Kind::F32Int, -100, 100);
            }
            let mut nd = s.needs.clone();
            nd.sort();
            for (x, v, u) in nd {
                let name = NEEDS[x].1;
                go(&mut b, &["Needs", name], v, Kind::F32Int, -100, 100);
                let p = path(&["Needs", name]);
                let p: Vec<&str> = p.iter().map(String::as_str).collect();
                b.mirror(&p, u + 4);
            }
            let mut sk = s.skills.clone();
            sk.sort();
            for (x, l) in sk {
                let (_, name, max) = SKILLS[x];
                go(&mut b, &["Skills", name], l, Kind::I32, 0, max);
            }
        }
    }
    if b.is_empty() {
        return Err("no households found".into());
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("The game data is saved back uncompressed, so the file grows. Close the game first.")))
}

#[cfg(test)]
pub(crate) mod tests {
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
            &[("mLifetimeHappiness", 12), ("mSpendableHappiness", 12), ("mFlags", 23), ("mHousehold", 1), ("mFirstName", 1), ("mLastName", 1), ("When", 16), ("mSim", 1)],
        );
        class(&mut ct, "Sims3.Gameplay.Skills.Cooking", &[("SkillLevel", 9), ("SkillPoints", 12), ("mSkillOwner", 1), ("mSkillGuid", 23)]);
        class(&mut ct, "Sims3.Gameplay.Autonomy.Motives", &[("mMotives", 1), ("mSim", 1)]);
        class(&mut ct, "System.Collections.Generic.Dictionary`2<System.Int32,Sims3.Gameplay.Autonomy.Motive>", &[]);
        class(&mut ct, "Sims3.Gameplay.Autonomy.Motive", &[("mValue", 12), ("mUIValue", 12), ("Commodity", 23)]);
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
        // 1: household, 2: its name, 3: Sim, 4: first, 5: last, 6: skill, 7-9: needs
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
        sd.extend(42u32.to_le_bytes());
        objs.push(obj(3, &sd));
        objs.push(s("Gameplay/Excel/PV/Sims:Bella"));
        objs.push(s("Gameplay/Excel/PV/Sims:Goth"));
        let mut sk = 3i32.to_le_bytes().to_vec();
        sk.extend(4000f32.to_le_bytes());
        sk.extend(3u32.to_le_bytes());
        sk.extend([0x81, 0x03, 0x0F]);
        sk.extend(5u64.to_le_bytes());
        objs.push(obj(4, &sk));
        // 7: motives of actor 42, 8: their dictionary, 9: hunger
        let mut mo = 8u32.to_le_bytes().to_vec();
        mo.extend(42u32.to_le_bytes());
        objs.push(obj(5, &mo));
        let mut di = vec![1u8];
        di.extend(0x010F_38D7u32.to_le_bytes());
        di.extend(9u32.to_le_bytes());
        objs.push(obj(6, &di));
        let mut hu = (-20f32).to_le_bytes().to_vec();
        hu.extend((-20f32).to_le_bytes());
        hu.extend([0x81, 0x04, 0x09]);
        hu.extend(0x010F_38D7u32.to_le_bytes());
        objs.push(obj(7, &hu));
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

    pub(crate) fn package() -> Vec<u8> {
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
        assert_eq!(v(&doc, "/Households/Goth/Sims/Bella Goth/Needs/Hunger"), Some(Value::from(-20)));
        assert_eq!(rebuild(&d, &doc.to_bytes().unwrap()).unwrap(), d);
        doc.set("/Households/Goth/Funds", Value::from(1_000_000)).unwrap();
        doc.set("/Households/Goth/Sims/Bella Goth/Lifetime reward points", Value::from(50_000)).unwrap();
        doc.set("/Households/Goth/Sims/Bella Goth/Skills/Cooking", Value::from(10)).unwrap();
        assert!(doc.set("/Households/Goth/Sims/Bella Goth/Skills/Cooking", Value::from(11)).is_err());
        doc.set("/Households/Goth/Sims/Bella Goth/Needs/Hunger", Value::from(100)).unwrap();
        let out = rebuild(&d, &doc.to_bytes().unwrap()).unwrap();
        // The other resource is kept byte for byte.
        let (_, es, _, _) = index(&out).unwrap();
        let o = es.iter().find(|e| e.ty == 0x1234).unwrap();
        assert_eq!(&out[o.offset..o.offset + o.size], b"other resource");
        let again = parse(&flatten(&out).unwrap()).unwrap();
        assert_eq!(v(&again, "/Households/Goth/Funds"), Some(Value::from(1_000_000)));
        assert_eq!(v(&again, "/Households/Goth/Sims/Bella Goth/Lifetime reward points"), Some(Value::from(50_000)));
        assert_eq!(v(&again, "/Households/Goth/Sims/Bella Goth/Skills/Cooking"), Some(Value::from(10)));
        assert_eq!(v(&again, "/Households/Goth/Sims/Bella Goth/Needs/Hunger"), Some(Value::from(100)));
        // The shown copy of the need (mUIValue) is written too.
        let fl = flatten(&out).unwrap();
        let hundred = 100f32.to_le_bytes();
        assert!(fl.windows(8).any(|w| w[..4] == hundred && w[4..] == hundred));
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
            if std::env::var("SAVE_EDITOR_SIMS3_SHOW").is_ok() {
                println!("{}", serde_json::to_string_pretty(&hs.values().next().unwrap()["Sims"]).unwrap());
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
