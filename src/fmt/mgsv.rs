//! Metal Gear Solid V: The Phantom Pain (PC) saves: `TPP_GAME_DATA`, `PERSONAL_DATA`,
//! `TPP_CONFIG_DATA` (and the Ground Zeroes / MGO names).
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
pub const KEYS: [&str; 4] = ["TPP_GAME_DATA", "PERSONAL_DATA", "TPP_CONFIG_DATA", "MGO_GAME_DATA"];

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
    p.len() > 24 && Md5::digest(&p[16..]).as_slice() == &p[..16] && matches!(&p[16..20], b"SVCS" | b"SVAR")
}

/// Decrypts a save, trying each known file name; returns the key and the plain data.
pub fn decrypt(d: &[u8]) -> Option<(&'static str, Vec<u8>)> {
    if d.len() < 28 {
        return None;
    }
    KEYS.iter().find_map(|k| {
        let mut p = d.to_vec();
        stream(k, &mut p);
        md5_ok(&p).then_some((*k, p))
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
    let mut seen_player: Vec<&str> = vec![];
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
                    item_leaves.push(Leaf { at, bit, ty });
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
                let key = crate::tree::unique_key(&group, &key_name);
                if count == 1 {
                    editable.insert(format!("/Variables/{}/{}", crate::tree::esc(&format!("Block {bi}, section {id}")), crate::tree::esc(&key)), leaves.len());
                    leaves.push(item_leaves.remove(0));
                    group.insert(key, items.remove(0));
                } else {
                    for (j, l) in item_leaves.into_iter().enumerate() {
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
    for g in ["Mother Base", "Statistics"] {
        if let Some(m) = extra.remove(g) {
            view.insert(g.into(), Value::Object(m));
        }
    }
    view.insert("Variables".into(), Value::Object(vars));
    let backend = MgsvBackend { plain: Arc::new(p.to_vec()), leaves };
    Ok(TreeDoc::new(Value::Object(view), editable, KIND, Box::new(backend)).with_note(
        "Variables are named by the hash of their Lua name; the few the Player screen uses are named. \
         The save's MD5 and encryption are redone on save. Turn off Steam Cloud first, and edit the file the game reads (in 311340\\remote for the PC version), not only the backup copy.",
    ))
}

#[derive(Clone, Debug)]
struct MgsvBackend {
    plain: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
}

impl Backend for MgsvBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = self.plain.to_vec();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            self.check(*id, v)?;
            let n = v.as_f64().or_else(|| v.as_bool().map(|b| b as u8 as f64)).ok_or("expected a number")?;
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

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let ty = self.leaves[leaf].ty;
        let n = v.as_f64().or_else(|| v.as_bool().map(|b| b as u8 as f64)).ok_or("expected a number")?;
        let (lo, hi) = ty.range();
        if n < lo || n > hi {
            return Err(format!("must be between {lo} and {hi}"));
        }
        if ty != Ty::F32 && ty != Ty::Bool && n.fract() != 0.0 {
            return Err("expected a whole number".into());
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes() {
        // Names whose hashes appear in a real TPP_GAME_DATA.
        assert_eq!(strcode32("fobPickup"), strcode32("fobPickup"));
        assert_ne!(strcode32("mbmTppGmp"), strcode32("mbmTppHeroicPoint"));
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

    /// Needs a real save: MGSV=<TPP_GAME_DATA file>
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("MGSV").unwrap()).unwrap();
        let (key, p) = decrypt(&d).unwrap();
        assert_eq!(key, "TPP_GAME_DATA");
        let mut doc = parse(&p).unwrap();
        println!("{}", doc.view["Player"]);
        for (g, v) in doc.view["Variables"].as_object().unwrap() { println!("{g}: {} vars", v.as_object().unwrap().len()); }
        println!("{:?}", doc.view["Variables"].as_object().unwrap().values().flat_map(|v| v.as_object().unwrap().keys()).filter(|k| !k.chars().all(|c| c.is_ascii_hexdigit())).collect::<Vec<_>>());
        assert_eq!(doc.to_bytes().unwrap(), p);
        doc.set("/Player/GMP", 123_456.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Player/GMP"), Some(&123_456.into()));
        assert_eq!(decrypt(&encrypt(key, &out)).unwrap().1[16..], out[16..]);
        assert_eq!(encrypt(key, &p), d, "re-encrypting the unchanged save gives the same file");
    }
}
