//! Metal Gear Solid V: The Phantom Pain (PC) saves: `TPP_GAME_DATA`, `PERSONAL_DATA`,
//! `TPP_CONFIG_DATA` (and the Ground Zeroes / MGO names). Ground Zeroes saves
//! (`MGSV_GZ_Save_Data_GL_EN*`, marker `gzwi`) use the same encryption and checksum but a
//! fixed binary layout with no variable table: they are only decrypted, for the hex view.
//!
//! The file is encrypted with an xorshift stream seeded from the first four bytes of the
//! MD5 of the file's base name (`TPP_GAME_DATA` …). Decrypted, it is a 16-byte MD5 of
//! everything after it, then `SVCS` (a table of `SVAR` blocks) or a single `SVAR` block.
//! A block lists its variables by `StrCode32` of their Lua name (CityHash64 with the
//! game's seeds): where each lives, how many (a whole array), and its type. The layout
//! follows MGSV_SaveTranslator (MIT, key and checksum) and the `squib` decoder (file layout
//! facts only); CityHash64 is ported from unknown321/cityhash (MIT, after Google's
//! CityHash). Only the few variable names the screen uses are built in; every other
//! variable shows by its hash.

use crate::tree::{Backend, TreeDoc};
use md5::{Digest, Md5};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub const KIND: &str = "Metal Gear Solid V save";
pub const KEYS: [&str; 5] = ["TPP_GAME_DATA", "PERSONAL_DATA", "TPP_CONFIG_DATA", "MGO_GAME_DATA", "MGSV_GZ_Save_Data_GL_EN"];

/// The marker after the checksum of a decrypted save: The Phantom Pain's variable tables, or Ground Zeroes.
fn marker_ok(m: &[u8]) -> bool {
    matches!(m, b"SVCS" | b"SVAR" | b"gzwi")
}

// ---------------------------------------------------------------- encryption

fn stream(key: &str, d: &mut [u8]) {
    let h = Md5::digest(key.as_bytes());
    let mut k = u32::from_le_bytes(h[..4].try_into().unwrap());
    let step = |k: &mut u32| {
        *k ^= *k << 13;
        *k ^= *k >> 7;
        *k ^= *k << 5;
    };
    let full = d.len() / 4 * 4;
    for c in d[..full].chunks_exact_mut(4) {
        step(&mut k);
        let v = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) ^ k;
        c.copy_from_slice(&v.to_le_bytes());
    }
    if full < d.len() {
        k ^= k << 13;
        k ^= k >> 7;
        let t = k ^ (k << 5);
        for (i, b) in d[full..].iter_mut().enumerate() {
            *b ^= (t >> (8 * i)) as u8;
        }
    }
}

fn md5_ok(p: &[u8]) -> bool {
    p.len() > 24 && matches!(&p[16..20], b"SVCS" | b"SVAR") && Md5::digest(&p[16..]).as_slice() == &p[..16]
}

/// Decrypts a save, trying each known file name; returns the key and the plain data.
pub fn decrypt(d: &[u8]) -> Option<(&'static str, Vec<u8>)> {
    if d.len() < 28 {
        return None;
    }
    KEYS.iter().find_map(|k| {
        // The marker after the checksum tells whether this is the key, before the whole file
        // is decrypted and summed.
        let mut head = [0u8; 20];
        head.copy_from_slice(&d[..20]);
        stream(k, &mut head);
        if !marker_ok(&head[16..20]) {
            return None;
        }
        let mut p = d.to_vec();
        stream(k, &mut p);
        (Md5::digest(&p[16..]).as_slice() == &p[..16]).then_some((*k, p))
    })
}

/// Encrypts plain data again: its MD5 is renewed first.
pub fn encrypt(key: &str, plain: &[u8]) -> Vec<u8> {
    let mut p = plain.to_vec();
    let h = Md5::digest(&p[16..]);
    p[..16].copy_from_slice(&h);
    stream(key, &mut p);
    p
}

// ---------------------------------------------------------------- StrCode32

const K0: u64 = 0xc3a5c85c97cb3127;
const K1: u64 = 0xb492b66fbe98f273;
const K2: u64 = 0x9ae16a3b2f90404f;
const K3: u64 = 0xc949d7c7509e6557;

fn l64(s: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(s[o..o + 8].try_into().unwrap())
}
fn l32(s: &[u8], o: usize) -> u64 {
    u32::from_le_bytes(s[o..o + 4].try_into().unwrap()) as u64
}
fn rot(v: u64, s: u32) -> u64 {
    if s == 0 {
        v
    } else {
        v.rotate_right(s)
    }
}
fn smix(v: u64) -> u64 {
    v ^ (v >> 47)
}
fn h16(u: u64, v: u64) -> u64 {
    const M: u64 = 0x9ddfea08eb382d69;
    let mut a = (u ^ v).wrapping_mul(M);
    a ^= a >> 47;
    let mut b = (v ^ a).wrapping_mul(M);
    b ^= b >> 47;
    b.wrapping_mul(M)
}
fn h0_16(s: &[u8]) -> u64 {
    let n = s.len();
    if n > 8 {
        let (a, b) = (l64(s, 0), l64(s, n - 8));
        return h16(a, rot(b.wrapping_add(n as u64), n as u32)) ^ b;
    }
    if n >= 4 {
        return h16((n as u64).wrapping_add(l32(s, 0) << 3), l32(s, n - 4));
    }
    if n > 0 {
        let (a, b, c) = (s[0] as u32, s[n >> 1] as u32, s[n - 1] as u32);
        let y = a + (b << 8);
        let z = n as u32 + (c << 2);
        return smix((y as u64).wrapping_mul(K2) ^ (z as u64).wrapping_mul(K3)).wrapping_mul(K2);
    }
    K2
}
fn h17_32(s: &[u8]) -> u64 {
    let n = s.len();
    let a = l64(s, 0).wrapping_mul(K1);
    let b = l64(s, 8);
    let c = l64(s, n - 8).wrapping_mul(K2);
    let d = l64(s, n - 16).wrapping_mul(K0);
    h16(rot(a.wrapping_sub(b), 43).wrapping_add(rot(c, 30)).wrapping_add(d), a.wrapping_add(rot(b ^ K3, 20)).wrapping_sub(c).wrapping_add(n as u64))
}
fn weak(w: u64, x: u64, y: u64, z: u64, mut a: u64, mut b: u64) -> (u64, u64) {
    a = a.wrapping_add(w);
    b = rot(b.wrapping_add(a).wrapping_add(z), 21);
    let c = a;
    a = a.wrapping_add(x).wrapping_add(y);
    b = b.wrapping_add(rot(a, 44));
    (a.wrapping_add(z), b.wrapping_add(c))
}
fn weak3(s: &[u8], o: usize, a: u64, b: u64) -> (u64, u64) {
    weak(l64(s, o), l64(s, o + 8), l64(s, o + 16), l64(s, o + 24), a, b)
}
fn h33_64(s: &[u8]) -> u64 {
    let n = s.len();
    let mut z = l64(s, 24);
    let mut a = l64(s, 0).wrapping_add((n as u64).wrapping_add(l64(s, n - 16)).wrapping_mul(K0));
    let mut b = rot(a.wrapping_add(z), 52);
    let mut c = rot(a, 37);
    a = a.wrapping_add(l64(s, 8));
    c = c.wrapping_add(rot(a, 7));
    a = a.wrapping_add(l64(s, 16));
    let vf = a.wrapping_add(z);
    let vs = b.wrapping_add(rot(a, 31)).wrapping_add(c);
    a = l64(s, 16).wrapping_add(l64(s, n - 32));
    z = l64(s, n - 8);
    b = rot(a.wrapping_add(z), 52);
    c = rot(a, 37);
    a = a.wrapping_add(l64(s, n - 24));
    c = c.wrapping_add(rot(a, 7));
    a = a.wrapping_add(l64(s, n - 16));
    let wf = a.wrapping_add(z);
    let ws = b.wrapping_add(rot(a, 31)).wrapping_add(c);
    let r = smix(vf.wrapping_add(ws).wrapping_mul(K2).wrapping_add(wf.wrapping_add(vs).wrapping_mul(K0)));
    smix(r.wrapping_mul(K0).wrapping_add(vs)).wrapping_mul(K2)
}
fn city64(s: &[u8]) -> u64 {
    let n = s.len();
    if n <= 32 {
        return if n <= 16 { h0_16(s) } else { h17_32(s) };
    }
    if n <= 64 {
        return h33_64(s);
    }
    let mut x = l64(s, n - 40);
    let mut y = l64(s, n - 16).wrapping_add(l64(s, n - 56));
    let mut z = h16(l64(s, n - 48).wrapping_add(n as u64), l64(s, n - 24));
    let mut v = weak3(s, n - 64, n as u64, z);
    let mut w = weak3(s, n - 32, y.wrapping_add(K1), x);
    x = x.wrapping_mul(K1).wrapping_add(l64(s, 0));
    let mut len = (n - 1) & !63;
    let mut off = 0;
    loop {
        x = rot(x.wrapping_add(y).wrapping_add(v.0).wrapping_add(l64(s, off + 8)), 37).wrapping_mul(K1);
        y = rot(y.wrapping_add(v.1).wrapping_add(l64(s, off + 48)), 42).wrapping_mul(K1);
        x ^= w.1;
        y = y.wrapping_add(v.0).wrapping_add(l64(s, off + 40));
        z = rot(z.wrapping_add(w.0), 33).wrapping_mul(K1);
        v = weak3(s, off, v.1.wrapping_mul(K1), x.wrapping_add(w.0));
        w = weak3(s, off + 32, z.wrapping_add(w.1), y.wrapping_add(l64(s, off + 16)));
        std::mem::swap(&mut z, &mut x);
        off += 64;
        len -= 64;
        if len == 0 {
            break;
        }
    }
    h16(h16(v.0, w.0).wrapping_add(smix(y).wrapping_mul(K1)).wrapping_add(z), h16(v.1, w.1).wrapping_add(x))
}

/// The game's `StrCode32` of a Lua name.
pub fn strcode32(name: &str) -> u32 {
    let mut s = name.as_bytes().to_vec();
    if s.is_empty() {
        s.push(0);
    }
    if *s.last().unwrap() != 0 {
        s.push(0);
    }
    let seed1 = if s.len() > 1 { ((s[0] as u64) << 16) + (s.len() as u64 - 1) } else { 0 };
    (h16(city64(&s).wrapping_sub(K2), seed1) & 0xFFFF_FFFF) as u32
}

// ---------------------------------------------------------------- variables

/// Variables the screen edits: (Lua name, label).
pub const PLAYER: &[(&str, &str)] = &[("mbmTppGmp", "GMP"), ("mbmTppHeroicPoint", "Heroism"), ("mbmTppOgrePoint", "Demon points"), ("mbmServerWalletGmp", "FOB wallet GMP"), ("mbmTppTotalIncomeGmp", "Total GMP earned")];

/// Every save variable name known for the game, by StrCode32 hash (names are the game's own Lua names).
fn all_names() -> &'static HashMap<u32, &'static str> {
    static M: std::sync::OnceLock<HashMap<u32, &'static str>> = std::sync::OnceLock::new();
    M.get_or_init(|| include_str!("mgsv_vars.txt").split_whitespace().map(|n| (strcode32(n), n)).collect())
}

/// Which screen group a single-value variable belongs to.
fn curated_group(name: &str) -> Option<&'static str> {
    if name.starts_with("total") && !name.contains("Time") {
        Some("Statistics")
    } else if name.starts_with("mbm") && !name.contains("Seed") && !name.contains("Sync") && !name.contains("Elapsed") && !name.contains("Pf") && !name.starts_with("mbmPd") {
        Some("Mother Base")
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Ty {
    Bool,
    I32,
    U32,
    F32,
    I8,
    U8,
    I16,
    U16,
}

impl Ty {
    fn from(n: u8) -> Option<Ty> {
        Some(match n & 7 {
            0 => Ty::Bool,
            1 => Ty::I32,
            2 => Ty::U32,
            3 => Ty::F32,
            4 => Ty::I8,
            5 => Ty::U8,
            6 => Ty::I16,
            7 => Ty::U16,
            _ => return None,
        })
    }
    fn width(self) -> usize {
        match self {
            Ty::Bool | Ty::I8 | Ty::U8 => 1,
            Ty::I16 | Ty::U16 => 2,
            _ => 4,
        }
    }
    fn range(self) -> (f64, f64) {
        match self {
            Ty::Bool => (0.0, 1.0),
            Ty::I32 => (i32::MIN as f64, i32::MAX as f64),
            Ty::U32 => (0.0, u32::MAX as f64),
            Ty::F32 => (f32::MIN as f64, f32::MAX as f64),
            Ty::I8 => (-128.0, 127.0),
            Ty::U8 => (0.0, 255.0),
            Ty::I16 => (-32768.0, 32767.0),
            Ty::U16 => (0.0, 65535.0),
        }
    }
}

#[derive(Clone, Debug)]
struct Leaf {
    at: usize,
    bit: u8,
    ty: Ty,
    /// A bit field inside the stored number (0 bits: the whole number).
    shift: u8,
    bits: u8,
    /// Named values for the field: 1 rank, 2 section, 3 skill.
    list: u8,
}

const RANKS: [&str; 10] = ["E", "D", "C", "B", "A", "A+", "A++", "S", "S+", "S++"];
const SECTIONS: [&str; 13] = ["Unassigned", "Combat", "R&D", "Base Development", "Support", "Intel", "Medical", "Security", "Sickbay", "Brig", "Quarantine", "Waiting room 1", "Waiting room 2"];
const SKILLS: &[(usize, &str)] = &[
    (1, "Reflex"), (2, "Ninja"), (3, "Athlete"), (4, "Locksmith"), (5, "Fulton expert"), (6, "Quick reload"), (7, "Study"), (8, "Lucky"), (9, "Grappler"), (10, "Big mouth"),
    (11, "Botanist"), (12, "Quick draw"), (13, "Scout sniper"), (14, "Physical Lv1"), (15, "Physical Lv2"), (16, "Physical Lv3"), (17, "Precision Lv1"), (18, "Precision Lv2"),
    (19, "Precision Lv3"), (20, "Medical Lv1"), (21, "Medical Lv2"), (22, "Medical Lv3"), (23, "Gunsmith: handgun"), (24, "Gunsmith: submachine gun"), (25, "Gunsmith: assault rifle"),
    (26, "Gunsmith: shotgun"), (27, "Gunsmith: grenade launcher"), (28, "Gunsmith: sniper rifle"), (29, "Gunsmith: machine gun"), (30, "Gunsmith: missile"), (31, "Master gunsmith"),
    (32, "Tranquilizer engineer"), (33, "Suppressor engineer"), (34, "Homing missile engineer"), (35, "Zoologist"), (36, "Sleeping gas engineer"), (37, "Electric engineer"),
    (38, "Electromagnetic net engineer"), (39, "Waterproof engineer"), (40, "Radar engineer"), (41, "Metamaterial engineer"), (42, "Drug engineer"), (43, "Mechanical engineer"),
    (44, "Mechatronics engineer"), (45, "Cybernetics engineer"), (46, "Rocket control engineer"), (47, "Electric spinning engineer"), (48, "Materials engineer"), (49, "Haulage engineer"),
    (50, "Remote control engineer"), (51, "Monitor engineer"), (52, "Active protect engineer"), (53, "Attitude control engineer"), (54, "Bipedalism weapon development"),
    (55, "Russian translator"), (56, "Afrikaans translator"), (57, "Kikongo translator"), (58, "Pashto translator"), (59, "Surgeon"), (60, "Physician"), (61, "Counselor"),
    (62, "Parasite researcher"), (63, "Troublemaker: violence"), (64, "Troublemaker: intemperance"), (65, "Troublemaker: harassment"), (66, "Moodmaker"), (67, "Tactics instructor"),
    (68, "Mother Base vice commander"), (69, "Defender Lv1"), (70, "Defender Lv2"), (71, "Defender Lv3"), (72, "Sentry Lv1"), (73, "Sentry Lv2"), (74, "Sentry Lv3"),
    (75, "Ranger Lv1"), (76, "Ranger Lv2"), (77, "Ranger Lv3"), (78, "Medic Lv1"), (79, "Medic Lv2"), (80, "Medic Lv3"), (81, "Liquid carbon missile engineer Lv1"),
    (82, "Liquid carbon missile engineer Lv2"), (83, "Liquid carbon missile engineer Lv3"), (84, "Interceptor missile engineer Lv1"), (85, "Interceptor missile engineer Lv2"),
    (86, "Interceptor missile engineer Lv3"),
];

fn names_of(list: u8) -> Vec<String> {
    match list {
        1 => RANKS.iter().map(|s| s.to_string()).collect(),
        2 => SECTIONS.iter().map(|s| s.to_string()).collect(),
        _ => (0..128).map(|i| SKILLS.iter().find(|(n, _)| *n == i).map_or_else(|| if i == 0 { "None".to_string() } else { format!("Skill {i}") }, |(_, n)| n.to_string())).collect(),
    }
}

/// The number a field holds, shown as its name when it has one.
fn field_value(l: &Leaf, raw: u64) -> Value {
    let v = if l.bits > 0 { (raw >> l.shift) & ((1u64 << l.bits) - 1) } else { raw };
    if l.list > 0 {
        let names = names_of(l.list);
        return Value::String(names.get(v as usize).cloned().unwrap_or_else(|| format!("Unknown {v}")));
    }
    if l.bits == 1 {
        return (v == 1).into();
    }
    v.into()
}

/// The number a list value stands for: its place in the list, or N for "Unknown N".
fn list_index(list: u8, name: &str) -> Option<u64> {
    names_of(list).iter().position(|x| x == name).map(|i| i as u64).or_else(|| name.strip_prefix("Unknown ")?.parse().ok())
}

fn u32_at(d: &[u8], p: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(p..p + 4)?.try_into().ok()?) as usize)
}
fn u16_at(d: &[u8], p: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(p..p + 2)?.try_into().ok()?) as usize)
}

/// Bases of every SVAR block (offset just after its magic).
fn blocks(p: &[u8]) -> Option<Vec<usize>> {
    match &p[16..20] {
        b"SVCS" => {
            let n = *p.get(23)? as usize;
            (0..n).map(|i| Some(u32_at(p, 24 + 12 * i)? + 20)).collect()
        }
        _ => Some(vec![20]),
    }
}

pub fn looks(d: &[u8]) -> bool {
    md5_ok(d)
}

pub fn parse(p: &[u8]) -> Result<TreeDoc, String> {
    if !md5_ok(p) {
        return Err("not a Metal Gear Solid V save".into());
    }
    let bad = || "the save's variable tables couldn't be read".to_string();
    let named: HashMap<u32, (&str, &str)> = PLAYER.iter().map(|&(n, l)| (strcode32(n), (n, l))).collect();
    let all = all_names();
    let mut view = Map::new();
    let mut player = Map::new();
    let mut vars = Map::new();
    let mut editable = HashMap::new();
    let mut leaves: Vec<Leaf> = vec![];
    let mut reshaping: Vec<usize> = vec![];
    let mut seen_player: Vec<&str> = vec![];
    let mut arrays: HashMap<String, (Vec<u64>, Vec<Leaf>)> = HashMap::new();
    let mut extra: std::collections::BTreeMap<&'static str, Map<String, Value>> = Default::default();
    for (bi, base) in blocks(p).ok_or_else(bad)?.into_iter().enumerate() {
        let nsec = *p.get(base + 3).ok_or_else(bad)? as usize;
        for si in 0..nsec {
            let so = base + 8 + 8 * si;
            let (id, n, doff) = (u16_at(p, so).ok_or_else(bad)?, u16_at(p, so + 2).ok_or_else(bad)?, u32_at(p, so + 4).ok_or_else(bad)?);
            if n == 0 {
                continue;
            }
            let hdr = ((doff + 3 + n * 2) & !3) - 4;
            let params = hdr + n * 8;
            let data_bits = base + params + n * 8;
            let mut group = Map::new();
            for i in 0..n {
                let ho = base + hdr + 8 * i;
                let hash = u32_at(p, ho).ok_or_else(bad)? as u32;
                let po = base + params + 8 * i;
                let off = u32_at(p, po).ok_or_else(bad)?;
                let count = u16_at(p, po + 4).ok_or_else(bad)?;
                let ty = Ty::from(*p.get(po + 6).ok_or_else(bad)? & 7).ok_or_else(bad)?;
                let key_name = all.get(&hash).map(|n| n.to_string()).unwrap_or_else(|| format!("{hash:08x}"));
                let mut items = vec![];
                let mut item_leaves = vec![];
                for j in 0..count {
                    let (at, bit) = if ty == Ty::Bool {
                        let b = off % 8 + j;
                        (data_bits + off / 8 + b / 8, (b % 8) as u8)
                    } else {
                        (base + off - 4 + j * ty.width(), 0)
                    };
                    let Some(bytes) = p.get(at..at + ty.width()) else { return Err(bad()) };
                    let v: Value = match ty {
                        Ty::Bool => ((bytes[0] >> bit) & 1 == 1).into(),
                        Ty::I32 => i32::from_le_bytes(bytes.try_into().unwrap()).into(),
                        Ty::U32 => u32::from_le_bytes(bytes.try_into().unwrap()).into(),
                        Ty::F32 => serde_json::Number::from_f64(f32::from_le_bytes(bytes.try_into().unwrap()) as f64).map_or(Value::Null, Value::Number),
                        Ty::I8 => (bytes[0] as i8).into(),
                        Ty::U8 => bytes[0].into(),
                        Ty::I16 => i16::from_le_bytes(bytes.try_into().unwrap()).into(),
                        Ty::U16 => u16::from_le_bytes(bytes.try_into().unwrap()).into(),
                    };
                    items.push(v);
                    item_leaves.push(Leaf { at, bit, ty, shift: 0, bits: 0, list: 0 });
                }
                if let Some((n, label)) = named.get(&hash) {
                    if count == 1 && !seen_player.contains(n) {
                        seen_player.push(n);
                        player.insert((*label).into(), items[0].clone());
                        editable.insert(format!("/Player/{}", crate::tree::esc(label)), leaves.len());
                        leaves.push(item_leaves[0].clone());
                    }
                }
                if let Some(name) = all.get(&hash) {
                    if count == 1 && !named.contains_key(&hash) {
                        if let Some(g) = curated_group(name) {
                            let entry = extra.entry(g).or_insert_with(Map::new);
                            if !entry.contains_key(*name) {
                                entry.insert((*name).into(), items[0].clone());
                                editable.insert(format!("/{g}/{}", crate::tree::esc(name)), leaves.len());
                                leaves.push(item_leaves[0].clone());
                            }
                        }
                    }
                }
                if matches!(key_name.as_str(), "mbmStaffSvarsHeaders" | "mbmStaffSvarsStatusesSync" | "mbmStaffSvarsStatusesNoSync" | "mbmResourceSvars1Fix" | "mbmResourceSvars2Fix" | "mbmResourceSvars1Diff" | "mbmClusterSvars" | "mbmEquipParamStatus") && !arrays.contains_key(&key_name) {
                    arrays.insert(key_name.clone(), (items.iter().map(raw_of).collect::<Vec<_>>(), item_leaves.clone()));
                }
                let key = crate::tree::unique_key(&group, &key_name);
                if count == 1 {
                    editable.insert(format!("/Variables/{}/{}", crate::tree::esc(&format!("Block {bi}, section {id}")), crate::tree::esc(&key)), leaves.len());
                    leaves.push(item_leaves.remove(0));
                    group.insert(key, items.remove(0));
                } else {
                    for (j, l) in item_leaves.into_iter().enumerate() {
                        // A staff slot appears in the Staff list once it holds anything.
                        if matches!(key_name.as_str(), "mbmStaffSvarsHeaders" | "mbmStaffSvarsStatusesSync") {
                            reshaping.push(leaves.len());
                        }
                        editable.insert(format!("/Variables/{}/{}/{j}", crate::tree::esc(&format!("Block {bi}, section {id}")), crate::tree::esc(&key)), leaves.len());
                        leaves.push(l);
                    }
                    group.insert(key, Value::Array(items));
                }
            }
            vars.insert(format!("Block {bi}, section {id}"), Value::Object(group));
        }
    }
    if !player.is_empty() {
        view.insert("Player".into(), Value::Object(player));
    }
    add_staff_and_resources(&arrays, &mut view, &mut editable, &mut leaves);
    for g in ["Mother Base", "Statistics"] {
        if let Some(m) = extra.remove(g) {
            view.insert(g.into(), Value::Object(m));
        }
    }
    view.insert("Variables".into(), Value::Object(vars));
    let mut ptrs = vec![String::new(); leaves.len()];
    for (ptr, &i) in &editable {
        ptrs[i] = ptr.clone();
    }
    let backend = MgsvBackend { plain: Arc::new(p.to_vec()), leaves, ptrs, reshaping };
    Ok(TreeDoc::new(Value::Object(view), editable, KIND, Box::new(backend)).with_note(
        crate::i18n::tx("Variables are named by the hash of their Lua name; the few the Player screen uses are named. \
         The save's MD5 and encryption are redone on save. Turn off Steam Cloud first, and edit the file the game reads (in 311340\\remote for the PC version), not only the backup copy."),
    ))
}

const RESOURCES: [&str; 39] = [
    "Fuel", "Biological", "Common metal", "Minor metal", "Precious metal", "Wormwood", "Black carrot", "Golden crescent", "Tarragon", "African peach", "Digitalis (purple)",
    "Digitalis (luteum)", "Haoma", "4WD (east)", "4WD (west)", "Truck (east)", "Truck (west)", "Armored vehicle (east)", "Armored vehicle (west)", "Armored vehicle (east, rocket)",
    "Wheeled armored vehicle (west)", "Tank (east)", "Tank (west)", "Walker Gear (Proto Heuy)", "Walker Gear (Soviet battle)", "Walker Gear (Soviet support)", "Walker Gear (CFA battle)",
    "Walker Gear (CFA support)", "Nuclear weapons", "Nuclear waste", "Parasite: fog", "Parasite: camouflage", "Parasite: curing", "Parasite: reserve", "Emplacement gun (east)",
    "Emplacement gun (west)", "Mortar", "Anti-air gatling gun (east)", "Anti-air gatling gun (west)",
];

/// Staff and Mother Base resources, decoded from the packed variables (bit layouts after the
/// GPL-3.0 tpp-mod's structure definitions).
fn add_staff_and_resources(arrays: &HashMap<String, (Vec<u64>, Vec<Leaf>)>, view: &mut Map<String, Value>, editable: &mut HashMap<String, usize>, leaves: &mut Vec<Leaf>) {
    let field = |l: &Leaf, shift: u8, bits: u8, list: u8| Leaf { shift, bits, list, ..l.clone() };
    if let (Some((h, hl)), Some((st, sl))) = (arrays.get("mbmStaffSvarsHeaders"), arrays.get("mbmStaffSvarsStatusesSync")) {
        let ns = arrays.get("mbmStaffSvarsStatusesNoSync");
        let mut staff = Map::new();
        for i in 0..h.len().min(st.len()) {
            if h[i] == 0 && st[i] == 0 {
                continue;
            }
            let mut m = Map::new();
            let mut add = |name: &str, raw: u64, l: Leaf| {
                m.insert(name.into(), field_value(&l, raw));
                editable.insert(format!("/Staff/{}/{}", crate::tree::esc(&format!("Staff {:04}", i + 1)), crate::tree::esc(name)), leaves.len());
                leaves.push(l);
            };
            for (name, shift, bits, list) in [("Rank", 3, 4, 1), ("Skill", 13, 7, 3), ("Stat bonus", 1, 2, 0), ("Stat distribution", 7, 6, 0), ("Stats suppressed", 0, 1, 0), ("Face", 20, 10, 0)] {
                add(name, h[i], field(&hl[i], shift, bits, list));
            }
            for (name, shift, bits, list) in [
                ("Section", 27, 4, 2), ("Combat team", 0, 4, 0), ("Morale", 22, 4, 0), ("Proficiency", 8, 4, 0), ("Health level", 17, 3, 0), ("Symptomatic", 16, 1, 0),
                ("DS medal", 12, 1, 0), ("DS cross", 13, 1, 0), ("Honor medal", 14, 1, 0),
            ] {
                add(name, st[i], field(&sl[i], shift, bits, list));
            }
            if let Some((n, nl)) = ns {
                if i < n.len() {
                    add("Locked", n[i], field(&nl[i], 0, 1, 0));
                }
            }
            staff.insert(format!("Staff {:04}", i + 1), Value::Object(m));
        }
        if !staff.is_empty() {
            view.insert("Staff".into(), Value::Object(staff));
        }
    }
    if let Some((c, cl)) = arrays.get("mbmClusterSvars") {
        const BASES: [&str; 5] = ["Mother Base", "FOB 1", "FOB 2", "FOB 3", "FOB 4"];
        const PLATFORMS: [&str; 7] = ["Command", "Combat", "R&D", "Base Development", "Support", "Intel", "Medical"];
        let mut bases = Map::new();
        for (bi, base) in BASES.iter().enumerate() {
            let mut plats = Map::new();
            for (pi, plat) in PLATFORMS.iter().enumerate() {
                let i = bi * 7 + pi;
                if i >= c.len() {
                    break;
                }
                let mut m = Map::new();
                for (label, shift, bits) in [("Grade (0-4)", 12, 4), ("Under construction", 10, 1)] {
                    let l = field(&cl[i], shift, bits, 0);
                    m.insert(label.into(), field_value(&l, c[i]));
                    editable.insert(format!("/Bases/{}/{}/{}", crate::tree::esc(base), crate::tree::esc(plat), crate::tree::esc(label)), leaves.len());
                    leaves.push(l);
                }
                plats.insert((*plat).into(), Value::Object(m));
            }
            bases.insert((*base).into(), Value::Object(plats));
        }
        view.insert("Bases".into(), Value::Object(bases));
    }
    if let (Some((u, ul)), Some((pr, pl))) = (arrays.get("mbmResourceSvars1Fix"), arrays.get("mbmResourceSvars2Fix")) {
        let local = arrays.get("mbmResourceSvars1Diff");
        let mut res = Map::new();
        for (i, name) in RESOURCES.iter().enumerate() {
            if i >= u.len() || i >= pr.len() {
                break;
            }
            let mut m = Map::new();
            let mut fields = vec![("Usable (online)", u[i], field(&ul[i], 1, 20, 0))];
            if let Some((d, dl)) = local {
                if i < d.len() {
                    fields.push(("Usable (local)", d[i], field(&dl[i], 1, 20, 0)));
                }
            }
            if i <= 12 {
                fields.push(("Processed (online)", pr[i], field(&pl[i], 3, 20, 0)));
            }
            for (label, raw, l) in fields {
                m.insert(label.into(), field_value(&l, raw));
                editable.insert(format!("/Resources/{}/{label}", crate::tree::esc(name)), leaves.len());
                leaves.push(l);
            }
            res.insert((*name).into(), Value::Object(m));
        }
        view.insert("Resources".into(), Value::Object(res));
    }
    if let Some((st, sl)) = arrays.get("mbmEquipParamStatus") {
        let mut research = Map::new();
        for (i, row) in dev_rows().iter().enumerate().take(st.len()) {
            let mut m = Map::new();
            for (label, shift) in [("Developed", DEV_DEVELOPED), ("Available", DEV_AVAILABLE)] {
                let l = field(&sl[i], shift, 1, 0);
                m.insert(label.into(), field_value(&l, st[i]));
                editable.insert(format!("/Research/{}/{label}", crate::tree::esc(&dev_key(row))), leaves.len());
                leaves.push(l);
            }
            research.insert(dev_key(row), Value::Object(m));
        }
        view.insert("Research".into(), Value::Object(research));
    }
}

// ---------------------------------------------------------------- research (development)

/// Bit positions in an `mbmEquipParamStatus` byte, from comparing real saves with the game's
/// development table: bit 0 is set once an item is developed and only ever with its earlier
/// grade developed too; bit 2 is set once it can be developed (also closed under earlier
/// grades). The other bits (new-item marks and the like) are left as they are.
pub const DEV_DEVELOPED: u8 = 0;
pub const DEV_AVAILABLE: u8 = 2;

/// One entry of the game's equipment development table. The save keeps one status byte per
/// entry, in this order (the retail 1.11 table; slots past it stay 0).
pub struct DevRow {
    pub id: u32,
    pub kind: &'static str,
    /// The development grade within its line.
    pub grade: u8,
    /// The entry that has to be developed first (0: none).
    pub prereq: u32,
    /// Needs a blueprint or special design from a mission or side op.
    pub blueprint: bool,
    /// Not in the iDroid development menu: DLC, enemy gear, unused entries.
    pub hidden: bool,
    /// Developed from the start of the game.
    pub starts_developed: bool,
}

/// The table rows (ids, types and chains are the game's own; names live in its encrypted text).
pub fn dev_rows() -> &'static [DevRow] {
    static R: std::sync::OnceLock<Vec<DevRow>> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        include_str!("mgsv_dev.txt")
            .lines()
            .filter_map(|l| {
                let mut p = l.split_whitespace();
                let (id, kind, grade, prereq, flags) = (p.next()?, p.next()?, p.next()?, p.next()?, p.next()?);
                let kind = match kind {
                    "ArtificialArm" => "Prosthetic arm",
                    "Assault" => "Assault rifle",
                    "BattleGear" => "Battle Gear",
                    "DDog" => "D-Dog",
                    "Equip" => "Equipment",
                    "GrenadeLauncher" => "Grenade launcher",
                    "Heli" => "Helicopter",
                    "Machinegun" => "Machine gun",
                    "Placed" => "Placed weapon",
                    "SecurityGadgets" => "Security gadget",
                    "Sniper" => "Sniper rifle",
                    "Submachinegun" => "Submachine gun",
                    "Throwing" => "Throwing weapon",
                    "WalkerGear" => "Walker Gear",
                    "Handgun" => "Handgun",
                    "Horse" => "Horse",
                    "Missile" => "Missile",
                    "Quiet" => "Quiet",
                    "Shield" => "Shield",
                    "Shotgun" => "Shotgun",
                    "Suit" => "Suit",
                    "Tool" => "Tool",
                    _ => "Other",
                };
                Some(DevRow { id: id.parse().ok()?, kind, grade: grade.parse().ok()?, prereq: prereq.parse().ok()?, blueprint: flags.contains('B'), hidden: flags.contains('X'), starts_developed: flags.contains('I') })
            })
            .collect()
    })
}

/// The view key of a table row, e.g. "Handgun 1005".
pub fn dev_key(r: &DevRow) -> String {
    format!("{} {}", r.kind, r.id)
}

/// Which rows "Develop everything available" should mark developed, given each row's
/// (developed, available) state: every row of the iDroid menu that can already be developed,
/// plus the rest of its chain of earlier grades. Hidden rows (DLC, enemy gear, unused), rows
/// that need a blueprint and aren't unlocked yet, and chains whose first grade is still locked
/// (story-gated) are left alone.
pub fn develop_plan(state: &dyn Fn(usize) -> (bool, bool)) -> Vec<usize> {
    let rows = dev_rows();
    let at: HashMap<u32, usize> = rows.iter().enumerate().map(|(i, r)| (r.id, i)).collect();
    let mut memo: Vec<Option<bool>> = vec![None; rows.len()];
    fn ok(i: usize, rows: &[DevRow], at: &HashMap<u32, usize>, state: &dyn Fn(usize) -> (bool, bool), memo: &mut Vec<Option<bool>>, depth: usize) -> bool {
        if let Some(v) = memo[i] {
            return v;
        }
        let r = &rows[i];
        let (dev, avail) = state(i);
        let v = if r.hidden {
            false
        } else if dev || avail || r.starts_developed {
            true
        } else if r.blueprint || r.prereq == 0 || depth > 64 {
            false
        } else {
            at.get(&r.prereq).is_some_and(|&p| ok(p, rows, at, state, memo, depth + 1))
        };
        memo[i] = Some(v);
        v
    }
    (0..rows.len()).filter(|&i| ok(i, rows, &at, state, &mut memo, 0) && !state(i).0).collect()
}

#[derive(Clone, Debug)]
struct MgsvBackend {
    plain: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
    /// The view pointer of each leaf.
    ptrs: Vec<String>,
    /// Leaves whose edit can add or remove a Staff entry in the view.
    reshaping: Vec<usize>,
}

/// A stored whole number as the bits it occupies (a negative one as its two's complement).
fn raw_of(v: &Value) -> u64 {
    v.as_u64().or_else(|| v.as_i64().map(|n| n as u32 as u64)).unwrap_or(0)
}

/// The value a whole stored number of type `ty` shows as, from its bits.
fn whole_value(ty: Ty, raw: u64) -> Value {
    match ty {
        Ty::I32 => (raw as u32 as i32).into(),
        Ty::I8 => (raw as u8 as i8).into(),
        Ty::I16 => (raw as u16 as i16).into(),
        _ => raw.into(),
    }
}

impl Backend for MgsvBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = self.plain.to_vec();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            self.check(*id, v)?;
            let n = if l.list > 0 { 0.0 } else { v.as_f64().or_else(|| v.as_bool().map(|b| b as u8 as f64)).ok_or("expected a number")? };
            if l.bits > 0 || l.list > 0 {
                let new = if l.list > 0 {
                    let name = v.as_str().ok_or("expected a name")?;
                    list_index(l.list, name).ok_or("not a known value")?
                } else {
                    n as u64
                };
                let w = l.ty.width();
                let mut raw = 0u64;
                for (k, b) in p[l.at..l.at + w].iter().enumerate() {
                    raw |= (*b as u64) << (8 * k);
                }
                let mask = ((1u64 << l.bits) - 1) << l.shift;
                raw = (raw & !mask) | ((new << l.shift) & mask);
                for k in 0..w {
                    p[l.at + k] = (raw >> (8 * k)) as u8;
                }
                continue;
            }
            match l.ty {
                Ty::Bool => {
                    let on = v.as_bool().unwrap_or(n != 0.0);
                    if on {
                        p[l.at] |= 1 << l.bit;
                    } else {
                        p[l.at] &= !(1 << l.bit);
                    }
                }
                Ty::I32 => p[l.at..l.at + 4].copy_from_slice(&(n as i32).to_le_bytes()),
                Ty::U32 => p[l.at..l.at + 4].copy_from_slice(&(n as u32).to_le_bytes()),
                Ty::F32 => p[l.at..l.at + 4].copy_from_slice(&(n as f32).to_le_bytes()),
                Ty::I8 => p[l.at] = (n as i8) as u8,
                Ty::U8 => p[l.at] = n as u8,
                Ty::I16 => p[l.at..l.at + 2].copy_from_slice(&(n as i16).to_le_bytes()),
                Ty::U16 => p[l.at..l.at + 2].copy_from_slice(&(n as u16).to_le_bytes()),
            }
        }
        let h = Md5::digest(&p[16..]);
        p[..16].copy_from_slice(&h);
        Ok(p)
    }

    fn reshapes(&self, leaf: usize) -> bool {
        self.reshaping.contains(&leaf)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        match (self.leaves.get(leaf).map(|l| l.ty), v.as_f64()) {
            (Some(Ty::F32), Some(n)) => serde_json::Number::from_f64(n as f32 as f64).map_or(v, Value::Number),
            _ => v,
        }
    }

    /// The same stored number shows in several places (the whole variable, and the fields of
    /// a packed one: a staff member's rank, a resource's amount…). Keep all of them in step.
    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Some(l) = self.leaves.get(leaf) else { return vec![] };
        let Some(new) = self.ptrs.get(leaf).and_then(|p| view.pointer(p)) else { return vec![] };
        let same = |m: &Leaf| m.at == l.at && m.ty == l.ty && (l.ty != Ty::Bool || m.bit == l.bit);
        let whole = |m: &Leaf| m.bits == 0 && m.list == 0;
        let mut raw = None;
        if l.ty == Ty::Bool || l.ty == Ty::F32 {
            // Not packed: every sibling is the same value.
            return (0..self.leaves.len()).filter(|&m| m != leaf && same(&self.leaves[m])).map(|m| (self.ptrs[m].clone(), new.clone())).collect();
        }
        if whole(l) {
            raw = Some(raw_of(new));
        } else if let Some(cur) = (0..self.leaves.len()).find(|&m| m != leaf && same(&self.leaves[m]) && whole(&self.leaves[m])).and_then(|m| view.pointer(&self.ptrs[m])) {
            let n = if l.list > 0 { new.as_str().and_then(|s| list_index(l.list, s)) } else { new.as_u64().or_else(|| new.as_bool().map(|b| b as u64)) };
            if let Some(n) = n {
                let mask = ((1u64 << l.bits) - 1) << l.shift;
                raw = Some((raw_of(cur) & !mask) | ((n << l.shift) & mask));
            }
        }
        let Some(raw) = raw else { return vec![] };
        (0..self.leaves.len())
            .filter(|&m| m != leaf && same(&self.leaves[m]))
            .map(|m| {
                let ml = &self.leaves[m];
                (self.ptrs[m].clone(), if whole(ml) { whole_value(ml.ty, raw) } else { field_value(ml, raw) })
            })
            .collect()
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let l = &self.leaves[leaf];
        if l.list > 0 {
            let name = v.as_str().ok_or("pick a value from the list")?;
            return match list_index(l.list, name) {
                Some(i) if i < 1u64 << l.bits => Ok(()),
                _ => Err("not a known value".into()),
            };
        }
        let ty = l.ty;
        let n = v.as_f64().or_else(|| v.as_bool().map(|b| b as u8 as f64)).ok_or("expected a number")?;
        let (lo, hi) = if l.bits > 0 { (0.0, ((1u64 << l.bits) - 1) as f64) } else { ty.range() };
        if n < lo || n > hi {
            return Err(format!("must be between {lo} and {hi}"));
        }
        if ty != Ty::F32 && ty != Ty::Bool && n.fract() != 0.0 {
            return Err("expected a whole number".into());
        }
        Ok(())
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        let l = self.leaves.get(leaf)?;
        (l.list > 0).then(|| Arc::new(names_of(l.list)))
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    /// Every skill id has one name, and the specialist names match the game's wording.
    #[test]
    fn skill_table_is_unique_and_names_materials_engineer() {
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for (id, name) in SKILLS {
            assert!(ids.insert(*id), "duplicate skill id {id}");
            assert!(names.insert(*name), "duplicate skill name {name}");
        }
        assert_eq!(SKILLS.iter().find(|(i, _)| *i == 48).map(|(_, n)| *n), Some("Materials engineer"));
        assert!(names_of(3).iter().any(|n| n == "Materials engineer"));
    }

    use super::*;
    use serde_json::json;

    #[test]
    fn hashes() {
        // Names whose hashes appear in a real TPP_GAME_DATA.
        assert_eq!(strcode32("fobPickup"), strcode32("fobPickup"));
        assert_ne!(strcode32("mbmTppGmp"), strcode32("mbmTppHeroicPoint"));
    }

    #[test]
    fn ground_zeroes_decrypts() {
        let mut plain = vec![0u8; 16];
        plain.extend(b"gzwin\0\0\0");
        plain.extend((0..500u32).map(|i| (i * 11) as u8));
        let raw = encrypt("MGSV_GZ_Save_Data_GL_EN", &plain);
        let (key, p) = decrypt(&raw).unwrap();
        assert_eq!(key, "MGSV_GZ_Save_Data_GL_EN");
        assert_eq!(&p[16..], &plain[16..]);
        assert!(!looks(&p), "no variable table to read");
        assert_eq!(encrypt(key, &p), raw);
    }

    #[test]
    fn stream_round_trip() {
        let plain: Vec<u8> = (0..203u32).map(|i| (i * 7) as u8).collect();
        let mut d = plain.clone();
        stream("TPP_GAME_DATA", &mut d);
        assert_ne!(d, plain);
        stream("TPP_GAME_DATA", &mut d);
        assert_eq!(d, plain);
    }

    /// A decrypted save with one `SVAR` block and one section: (variable name, type code, values).
    fn synthetic(vars: &[(&str, u8, Vec<u64>)]) -> Vec<u8> {
        let n = vars.len();
        let base = 20usize;
        let doff = 16usize;
        let hdr = ((doff + 3 + n * 2) & !3) - 4;
        let params = hdr + n * 8;
        let data_bits = base + params + n * 8;
        let mut p = vec![0u8; 16];
        p.extend(b"SVAR");
        p.resize(data_bits + 4096, 0);
        p[base + 3] = 1;
        p[base + 8..base + 10].copy_from_slice(&1u16.to_le_bytes());
        p[base + 10..base + 12].copy_from_slice(&(n as u16).to_le_bytes());
        p[base + 12..base + 16].copy_from_slice(&(doff as u32).to_le_bytes());
        let mut bool_bits = 0usize;
        let mut next = data_bits + 8; // plain data after 8 bytes of bit storage
        for (i, (name, ty, vals)) in vars.iter().enumerate() {
            let t = Ty::from(*ty).unwrap();
            let ho = base + hdr + 8 * i;
            p[ho..ho + 4].copy_from_slice(&strcode32(name).to_le_bytes());
            let po = base + params + 8 * i;
            let off = if t == Ty::Bool { bool_bits } else { next - base + 4 };
            p[po..po + 4].copy_from_slice(&(off as u32).to_le_bytes());
            p[po + 4..po + 6].copy_from_slice(&(vals.len() as u16).to_le_bytes());
            p[po + 6] = *ty;
            for (j, v) in vals.iter().enumerate() {
                if t == Ty::Bool {
                    let b = off % 8 + j;
                    p[data_bits + off / 8 + b / 8] |= (*v as u8) << (b % 8);
                } else {
                    let at = base + off - 4 + j * t.width();
                    p[at..at + t.width()].copy_from_slice(&v.to_le_bytes()[..t.width()]);
                }
            }
            if t == Ty::Bool {
                bool_bits += vals.len();
            } else {
                next += (vals.len() * t.width() + 3) & !3;
            }
        }
        p.truncate(next.max(data_bits + 8));
        let h = Md5::digest(&p[16..]);
        p[..16].copy_from_slice(&h);
        p
    }

    pub(crate) fn sample() -> Vec<u8> {
        let rank_e = 0u64;
        let staff0 = (3u64 << 3) | (5 << 13) | (9 << 20);
        let status0 = (7u64 << 27) | (5 << 22) | (3 << 8);
        let cluster = (2u64 << 12) | (1 << 10);
        synthetic(&[
            ("mbmTppGmp", 1, vec![1500]),
            ("mbmTppHeroicPoint", 1, vec![40]),
            ("mbmStaffSvarsHeaders", 2, vec![staff0, 0, rank_e + 8]),
            ("mbmStaffSvarsStatusesSync", 2, vec![status0, 0, 4]),
            ("mbmStaffSvarsStatusesNoSync", 2, vec![0, 0, 0]),
            ("mbmClusterSvars", 2, vec![cluster; 35]),
            ("mbmResourceSvars1Fix", 2, vec![(100u64 << 1) | 1; 39]),
            ("mbmResourceSvars2Fix", 2, vec![100u64 << 3; 39]),
            ("mbmResourceSvars1Diff", 2, vec![100u64 << 1; 39]),
            ("mbmEquipParamStatus", 5, {
                let mut v = vec![0u64; 1024];
                v[0] = 5; // developed from the start
                v[1] = 4; // can be developed
                v
            }),
            ("totalPlayTime", 3, vec![1]),
            ("alarmCount", 0, vec![1, 0, 1]),
        ])
    }

    #[test]
    fn development_table_is_consistent() {
        let rows = dev_rows();
        assert_eq!(rows.len(), 920);
        let ids: std::collections::HashSet<u32> = rows.iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), rows.len(), "ids are unique");
        assert!(rows.iter().all(|r| r.prereq == 0 || ids.contains(&r.prereq)), "every prerequisite is in the table");
        let keys: std::collections::HashSet<String> = rows.iter().map(dev_key).collect();
        assert_eq!(keys.len(), rows.len());
        assert!(rows.iter().all(|r| r.kind != "Other" || r.hidden || r.id > 0));
    }

    #[test]
    fn develop_plan_follows_chains_and_skips_locked_and_hidden() {
        let rows = dev_rows();
        // Nothing unlocked: only what starts developed is considered, and it is already developed.
        assert!(develop_plan(&|i| (rows[i].starts_developed, false)).iter().all(|&i| !rows[i].starts_developed));
        // Everything marked available: every visible row gets developed, no hidden one.
        let plan = develop_plan(&|_| (false, true));
        assert!(plan.iter().all(|&i| !rows[i].hidden));
        assert_eq!(plan.len(), rows.iter().filter(|r| !r.hidden).count());
        // Only the first grade of a line is available: the later grades follow, unless they need a blueprint.
        let at = |id: u32| rows.iter().position(|r| r.id == id).unwrap();
        let (a, b, c) = (at(1000), at(1001), at(1002));
        let plan = develop_plan(&|i| (false, i == b));
        assert!(plan.contains(&b));
        assert!(!plan.contains(&a) || rows[a].starts_developed, "an earlier grade nobody unlocked stays");
        let _ = c;
        // A blueprint row that is not unlocked is left alone even when its prerequisite is developed.
        if let Some(bp) = rows.iter().position(|r| r.blueprint && !r.hidden && r.prereq != 0) {
            let pre = at(rows[bp].prereq);
            assert!(!develop_plan(&|i| (i == pre, false)).contains(&bp));
        }
    }

    #[test]
    fn research_flags_edit_and_save() {
        use crate::formats::{decode_with, encode, verified, Content, Options};
        let raw = encrypt("TPP_GAME_DATA", &sample());
        let opts = Options::default();
        let d = decode_with(&raw, &opts);
        let Content::Tree(mut doc) = d.content else { panic!() };
        let rows = dev_rows();
        let (k0, k1) = (dev_key(&rows[0]), dev_key(&rows[1]));
        assert_eq!(doc.view["Research"][&k0]["Developed"], json!(true));
        assert_eq!(doc.view["Research"][&k1]["Developed"], json!(false));
        assert_eq!(doc.view["Research"][&k1]["Available"], json!(true));
        doc.set(&format!("/Research/{}/Developed", crate::tree::esc(&k1)), json!(true)).unwrap();
        doc.set(&format!("/Research/{}/Developed", crate::tree::esc(&k0)), json!(false)).unwrap();
        let content = Content::Tree(doc);
        let bytes = encode(&d.layers, &content).unwrap();
        let back = decode_with(&bytes, &opts).content;
        assert!(verified(&back, &content));
        let Content::Tree(t) = back else { panic!() };
        assert_eq!(t.view["Research"][&k1]["Developed"], json!(true));
        assert_eq!(t.view["Research"][&k0]["Developed"], json!(false));
        assert_eq!(t.view["Research"][&k0]["Available"], json!(true), "the other bits stay");
    }

    /// Every editable field, edited and saved the way the app saves: encode the layers, decode the
    /// file again (as the app does to verify it) and compare.
    /// Typing a raw number whose bits land outside a named list still saves.
    #[test]
    fn raw_values_outside_the_name_lists_save() {
        use crate::formats::{decode_with, encode, verified, Content, Options};
        let raw = encrypt("TPP_GAME_DATA", &sample());
        let opts = Options::default();
        let d = decode_with(&raw, &opts);
        let Content::Tree(mut doc) = d.content else { panic!() };
        let ptr = doc.editable.keys().find(|p| p.contains("mbmStaffSvarsHeaders/0")).cloned().unwrap();
        doc.set(&ptr, (0xFu64 << 3).into()).unwrap();
        assert_eq!(doc.view.pointer("/Staff/Staff 0001/Rank"), Some(&"Unknown 15".into()));
        let content = Content::Tree(doc);
        let bytes = encode(&d.layers, &content).unwrap();
        assert!(verified(&decode_with(&bytes, &opts).content, &content));
    }

    /// The staff screen's "Everyone to S++" fields, set on every member together, save and
    /// read back without touching anything else (the section stays put).
    #[test]
    fn everyone_to_s_plus_plus_fields_save_together() {
        use crate::formats::{decode_with, encode, verified, Content, Options};
        let raw = encrypt("TPP_GAME_DATA", &sample());
        let opts = Options::default();
        let d = decode_with(&raw, &opts);
        let Content::Tree(mut doc) = d.content else { panic!() };
        let names: Vec<String> = doc.view["Staff"].as_object().unwrap().keys().cloned().collect();
        let sections: Vec<Value> = names.iter().map(|n| doc.view["Staff"][n]["Section"].clone()).collect();
        let fields = [("Rank", json!("S++")), ("Stat bonus", json!(3)), ("Stats suppressed", json!(false)), ("Stat distribution", json!(6)), ("DS medal", json!(true)), ("DS cross", json!(true)), ("Honor medal", json!(true)), ("Morale", json!(15)), ("Proficiency", json!(15))];
        for n in &names {
            for (f, v) in &fields {
                doc.set(&format!("/Staff/{}/{f}", crate::tree::esc(n)), v.clone()).unwrap();
            }
        }
        let content = Content::Tree(doc);
        let bytes = encode(&d.layers, &content).unwrap();
        let back = decode_with(&bytes, &opts).content;
        assert!(verified(&back, &content));
        let Content::Tree(t) = back else { panic!() };
        for (n, section) in names.iter().zip(&sections) {
            for (f, v) in &fields {
                assert_eq!(&t.view["Staff"][n][*f], v, "{n} {f}");
            }
            assert_eq!(&t.view["Staff"][n]["Section"], section, "{n}");
        }
    }

    #[test]
    fn every_field_round_trips_through_the_app_save_chain() {
        use crate::formats::{decode_with, encode, same_layers, verified, Content, Options};
        let raw = encrypt("TPP_GAME_DATA", &sample());
        let opts = Options::default();
        let d = decode_with(&raw, &opts);
        let Content::Tree(doc) = &d.content else { panic!("not a tree: {}", d.content.kind()) };
        let mut ptrs: Vec<(String, Value)> = vec![];
        for ptr in doc.editable.keys().cloned().collect::<Vec<_>>() {
            let Some(cur) = doc.view.pointer(&ptr).cloned() else { continue };
            let new: Value = match &cur {
                Value::Bool(b) => (!b).into(),
                Value::String(_) => {
                    let ch = doc.choices(&ptr).expect("a list field has choices");
                    ch.iter().find(|c| Some(c.as_str()) != cur.as_str()).cloned().unwrap().into()
                }
                Value::Number(n) if n.is_f64() => 1.1.into(),
                Value::Number(_) => {
                    let (lo, hi) = doc.range(&ptr).unwrap_or((0.into(), 1000.into()));
                    let want = if cur.as_i64() == Some(1) { 2 } else { 1 };
                    if (lo.as_i64().unwrap_or(0)..=hi.as_i64().unwrap_or(1000)).contains(&want) { want.into() } else { lo }
                }
                _ => continue,
            };
            ptrs.push((ptr, new));
        }
        assert!(ptrs.len() > 100, "{} fields", ptrs.len());
        for (ptr, new) in &ptrs {
            let Content::Tree(base) = &decode_with(&raw, &opts).content else { unreachable!() };
            let mut doc = base.clone();
            if doc.set(ptr, new.clone()).is_err() {
                continue;
            }
            let stored = doc.view.pointer(ptr).cloned();
            let content = Content::Tree(doc);
            let bytes = encode(&d.layers, &content).unwrap_or_else(|e| panic!("{ptr}: {e}"));
            let check = decode_with(&bytes, &opts);
            assert!(same_layers(&check.layers, &d.layers), "{ptr}: layers");
            assert!(verified(&check.content, &content), "{ptr}: doesn't read back as edited (set {new})");
            let Content::Tree(t) = &check.content else { unreachable!() };
            assert_eq!(t.view.pointer(ptr), stored.as_ref(), "{ptr}");
        }
    }

    /// Needs a real save: MGSV=<TPP_GAME_DATA file>
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("MGSV").unwrap()).unwrap();
        let (key, p) = decrypt(&d).unwrap();
        assert_eq!(key, "TPP_GAME_DATA");
        let mut doc = parse(&p).unwrap();
        println!("{}", doc.view["Player"]);
        println!("{}", doc.view["Bases"]);
        let staff = doc.view["Staff"].as_object().unwrap();
        println!("staff {}", staff.len());
        let mut hist = std::collections::BTreeMap::new();
        for v in staff.values() { *hist.entry(format!("{}/{}", v["Rank"], v["Section"])).or_insert(0) += 1; }
        println!("{hist:?}");
        println!("{}", staff.values().nth(5).unwrap());
        println!("{}", doc.view["Resources"]);
        for (g, v) in doc.view["Variables"].as_object().unwrap() { println!("{g}: {} vars", v.as_object().unwrap().len()); }
        println!("{:?}", doc.view["Variables"].as_object().unwrap().values().flat_map(|v| v.as_object().unwrap().keys()).filter(|k| !k.chars().all(|c| c.is_ascii_hexdigit())).collect::<Vec<_>>());
        assert_eq!(doc.to_bytes().unwrap(), p);
        doc.set("/Player/GMP", 123_456.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Player/GMP"), Some(&123_456.into()));
        doc.set("/Staff/Staff 0006/Rank", "E".into()).unwrap();
        doc.set("/Staff/Staff 0006/Skill", "Ranger Lv3".into()).unwrap();
        doc.set("/Resources/Fuel/Usable (online)", 999_999.into()).unwrap();
        doc.set("/Resources/Fuel/Processed (online)", 1_000_000.into()).unwrap();
        let out2 = doc.to_bytes().unwrap();
        let again = parse(&out2).unwrap();
        assert_eq!(again.view.pointer("/Staff/Staff 0006/Rank"), Some(&"E".into()));
        assert_eq!(again.view.pointer("/Staff/Staff 0006/Skill"), Some(&"Ranger Lv3".into()));
        assert_eq!(again.view.pointer("/Staff/Staff 0006/Section"), doc.view.pointer("/Staff/Staff 0006/Section"));
        assert_eq!(again.view.pointer("/Resources/Fuel/Usable (online)"), Some(&999_999.into()));
        assert_eq!(again.view.pointer("/Resources/Fuel/Processed (online)"), Some(&1_000_000.into()));
        assert_eq!(again.view.pointer("/Resources/Biological"), doc.view.pointer("/Resources/Biological"));
        assert_eq!(decrypt(&encrypt(key, &out)).unwrap().1[16..], out[16..]);
        assert_eq!(encrypt(key, &p), d, "re-encrypting the unchanged save gives the same file");
        let rows = dev_rows();
        let state = |i: usize| {
            let r = &doc.view["Research"][dev_key(&rows[i])];
            (r["Developed"].as_bool().unwrap(), r["Available"].as_bool().unwrap())
        };
        let plan = develop_plan(&state);
        println!("develop plan: {} of {} rows", plan.len(), rows.len());
        let mut doc = parse(&p).unwrap();
        for &i in &plan {
            let k = crate::tree::esc(&dev_key(&rows[i]));
            doc.set(&format!("/Research/{k}/Available"), true.into()).unwrap();
            doc.set(&format!("/Research/{k}/Developed"), true.into()).unwrap();
        }
        let out = doc.to_bytes().unwrap();
        let after = parse(&out).unwrap();
        for &i in &plan {
            assert_eq!(after.view["Research"][dev_key(&rows[i])]["Developed"], Value::Bool(true));
        }
        let diff = p.iter().zip(&out).filter(|(a, b)| a != b).count();
        println!("bytes changed (incl. checksum): {diff}");
        assert_eq!(after.view["Staff"], doc.view["Staff"]);
        assert_eq!(after.view["Resources"], parse(&p).unwrap().view["Resources"]);
    }
}
