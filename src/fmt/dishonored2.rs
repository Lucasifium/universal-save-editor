//! Dishonored 2 checkpoint saves (`savegame\SLOT<n>\checkpoint.dat`).
//!
//! ```text
//! 0x00  u32 check: the four little-endian words of the MD5 of the rest of the file, xored
//! 0x04  arithmetic-coded stream (id Tech's adaptive order-0 coder: 8-bit symbols, 16-bit
//!       low/high/code, bits read from the low bit of each byte up). The model starts again
//!       every 16 KiB of output, right at the next unread bit.
//! ```
//!
//! The decoded stream is a byte-aligned LZ: a length byte, 0 for one literal byte that
//! follows, otherwise a u16 offset back into the output and `length + 3` bytes copied from
//! there (never overlapping its own output, and never reaching back past the start of the
//! current 128 KiB of output, where the game's dictionary starts again). What comes
//! out is the game state: a u32 length of what follows, then "ARKS6" and the rest.
//!
//! The bonecharm crafting state is one record: its learned traits as (u32 id, i32 level)
//! pairs behind a u16 count, a second such list, then the Raw Whalebone count as a 32-bit
//! float. Worked out from real saves; an edited save loads in the game.

use crate::tree::{Backend, TreeDoc};
use md5::Digest;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Dishonored 2 save";
const BLOCK: usize = 16 * 1024;
/// The LZ dictionary starts again every 128 KiB of the state.
const LZ_BLOCK: usize = 128 * 1024;
/// The record in front of the bonecharm crafting state.
const CRAFT: &[u8] = b"SCHK\x15\x00\x00\x2b\x00";
pub const WHALEBONE_MAX: i64 = 999;

fn check(body: &[u8]) -> u32 {
    let h = md5::Md5::digest(body);
    (0..4).fold(0, |x, i| x ^ u32::from_le_bytes(h[4 * i..4 * i + 4].try_into().unwrap()))
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 16 && u32::from_le_bytes(d[..4].try_into().unwrap()) == check(&d[4..])
}

// ----------------------------------------------------------------- arithmetic coder

struct Model {
    lo: [u32; 256],
    hi: [u32; 256],
    scale: u32,
}

impl Model {
    fn new() -> Self {
        let mut m = Model { lo: [0; 256], hi: [0; 256], scale: 256 };
        for i in 0..256 {
            m.lo[i] = i as u32;
            m.hi[i] = i as u32 + 1;
        }
        m
    }
    fn bump(&mut self, s: usize) {
        self.hi[s] += 1;
        for i in s + 1..256 {
            self.lo[i] += 1;
            self.hi[i] += 1;
        }
        self.scale += 1;
    }
}

fn ac_decode(d: &[u8]) -> Vec<u8> {
    let bits = d.len() * 8;
    let mut pos = 0usize;
    let bit = |pos: &mut usize| -> u32 {
        let p = *pos;
        *pos += 1;
        if p < bits {
            (d[p >> 3] >> (p & 7)) as u32 & 1
        } else {
            0
        }
    };
    let mut out = Vec::with_capacity(d.len() * 3);
    let (mut low, mut high, mut code) = (0u32, 0xffffu32, 0u32);
    let mut m = Model::new();
    // The game reads past the end as zeros; a few bytes of slack decode the last symbols.
    while pos < bits + 32 {
        if out.len() % BLOCK == 0 {
            low = 0;
            high = 0xffff;
            code = 0;
            m = Model::new();
            for _ in 0..16 {
                code = ((code << 1) | bit(&mut pos)) & 0xffff;
            }
        }
        let range = high - low + 1;
        let count = ((code.wrapping_sub(low) + 1) * m.scale - 1) / range;
        let s = match m.hi.partition_point(|&h| h <= count) {
            s if s < 256 && m.lo[s] <= count => s,
            _ => break,
        };
        high = (low + range * m.hi[s] / m.scale - 1) & 0xffff;
        low = (low + range * m.lo[s] / m.scale) & 0xffff;
        loop {
            if (high & 0x8000) == (low & 0x8000) {
            } else if (low & 0x4000) != 0 && (high & 0x4000) == 0 {
                code ^= 0x4000;
                low &= 0x3fff;
                high |= 0x4000;
            } else {
                m.bump(s);
                break;
            }
            low = (low << 1) & 0xffff;
            high = ((high << 1) | 1) & 0xffff;
            code = ((code << 1) | bit(&mut pos)) & 0xffff;
        }
        out.push(s as u8);
    }
    out
}

struct Bits {
    buf: Vec<u8>,
    n: usize,
}

impl Bits {
    fn put(&mut self, b: u32) {
        if self.n.is_multiple_of(8) {
            self.buf.push(0);
        }
        if b != 0 {
            *self.buf.last_mut().unwrap() |= 1 << (self.n % 8);
        }
        self.n += 1;
    }
}

fn ac_encode(data: &[u8]) -> Vec<u8> {
    let mut w = Bits { buf: Vec::with_capacity(data.len() / 2), n: 0 };
    for blk in data.chunks(BLOCK) {
        let (mut low, mut high, mut under) = (0u32, 0xffffu32, 0u32);
        let mut m = Model::new();
        for &s in blk {
            let s = s as usize;
            let range = high - low + 1;
            high = (low + range * m.hi[s] / m.scale - 1) & 0xffff;
            low = (low + range * m.lo[s] / m.scale) & 0xffff;
            loop {
                if (high & 0x8000) == (low & 0x8000) {
                    let b = high >> 15;
                    w.put(b);
                    while under > 0 {
                        w.put(b ^ 1);
                        under -= 1;
                    }
                } else if (low & 0x4000) != 0 && (high & 0x4000) == 0 {
                    under += 1;
                    low &= 0x3fff;
                    high |= 0x4000;
                } else {
                    m.bump(s);
                    break;
                }
                low = (low << 1) & 0xffff;
                high = ((high << 1) | 1) & 0xffff;
            }
        }
        // Two bits settle the last symbol; the decoder then reads 14 more before the next block.
        let b = (low >> 14) & 1;
        w.put(b);
        under += 1;
        while under > 0 {
            w.put(b ^ 1);
            under -= 1;
        }
        for _ in 0..14 {
            w.put(0);
        }
    }
    w.buf
}

// ----------------------------------------------------------------- LZ

fn lz_decode(d: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(d.len() * 3);
    let mut i = 0;
    while i < d.len() {
        let len = d[i] as usize;
        i += 1;
        if len == 0 {
            let Some(&b) = d.get(i) else { break };
            out.push(b);
            i += 1;
        } else {
            let Some(off) = d.get(i..i + 2).map(|o| u16::from_le_bytes([o[0], o[1]]) as usize) else { break };
            i += 2;
            if off == 0 || off > out.len() {
                break;
            }
            let st = out.len() - off;
            for k in 0..len + 3 {
                out.push(out[st + k]);
            }
        }
    }
    out
}

/// Packs the way the game does, so an unchanged state packs to the very same bytes: greedy,
/// the longest match (4 to 258 bytes, never overlapping its own output) at the nearest offset,
/// and a dictionary that starts again every 128 KiB, which no match reaches across.
fn lz_encode(d: &[u8]) -> Vec<u8> {
    const HASH: usize = 1 << 16;
    let mut head = vec![usize::MAX; HASH];
    let mut prev = vec![usize::MAX; d.len()];
    let hash = |i: usize| -> usize {
        let v = u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
        (v.wrapping_mul(2_654_435_761) >> 16) as usize
    };
    let mut out = Vec::with_capacity(d.len());
    let mut i = 0;
    while i < d.len() {
        if i % LZ_BLOCK == 0 {
            head.fill(usize::MAX);
        }
        let start = i - i % LZ_BLOCK;
        let end = (start + LZ_BLOCK).min(d.len());
        let (mut best, mut boff) = (0usize, 0usize);
        if i + 4 <= end {
            let mut p = head[hash(i)];
            while p != usize::MAX && p >= start && i - p <= 0xffff {
                let lim = 258.min(i - p).min(end - i);
                if lim > best && lim >= 4 {
                    let mut l = 0;
                    while l < lim && d[p + l] == d[i + l] {
                        l += 1;
                    }
                    if l >= 4 && l > best {
                        best = l;
                        boff = i - p;
                        if l == 258 {
                            break;
                        }
                    }
                }
                p = prev[p];
            }
        }
        let step = if best >= 4 {
            out.push((best - 3) as u8);
            out.extend((boff as u16).to_le_bytes());
            best
        } else {
            out.push(0);
            out.push(d[i]);
            1
        };
        for k in i..i + step {
            if k + 4 <= d.len() {
                let h = hash(k);
                prev[k] = head[h];
                head[h] = k;
            }
        }
        i += step;
    }
    out
}

/// The game state inside a checkpoint file.
pub fn decode(d: &[u8]) -> Option<Vec<u8>> {
    if !looks(d) {
        return None;
    }
    let mut plain = lz_decode(&ac_decode(&d[4..]));
    // The state starts with its own length; what decodes after it is only the coder's padding.
    let len = u32::from_le_bytes(plain.get(..4)?.try_into().ok()?) as usize + 4;
    if plain.get(4..9) != Some(b"ARKS6") || plain.len() < len {
        return None;
    }
    plain.truncate(len);
    Some(plain)
}

pub fn encode(plain: &[u8]) -> Vec<u8> {
    let body = ac_encode(&lz_encode(plain));
    let mut out = check(&body).to_le_bytes().to_vec();
    out.extend(body);
    out
}

// ----------------------------------------------------------------- values

/// Where the Raw Whalebone float is and how many bonecharm traits are learned.
fn crafting(p: &[u8]) -> Option<(usize, usize)> {
    let mut found = None;
    let mut from = 0;
    while let Some(k) = p[from..].windows(CRAFT.len()).position(|w| w == CRAFT) {
        let at = from + k;
        from = at + 1;
        let mut q = at + CRAFT.len();
        let u16_at = |q: usize| p.get(q..q + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
        let Some(n1) = u16_at(q) else { break };
        q += 2 + 8 * n1;
        let Some(n2) = u16_at(q) else { continue };
        q += 2 + 8 * n2;
        if n1 > 256 || n2 > 256 || p.get(q + 8..q + 12) != Some(b"SCHK") {
            continue;
        }
        let v = f32::from_le_bytes(p[q..q + 4].try_into().ok()?);
        if !(0.0..=1.0e6).contains(&v) || v.fract() != 0.0 {
            continue;
        }
        if found.is_some() {
            return None; // two candidates: don't guess
        }
        found = Some((q, n1));
    }
    found
}

/// The map name stored near the top of the state.
fn map_name(p: &[u8]) -> Option<String> {
    let at = p.windows(9).position(|w| w == b"campaign/")?;
    let len = u32::from_le_bytes(p.get(at - 4..at)?.try_into().ok()?) as usize;
    let s = p.get(at..at + len)?;
    Some(String::from_utf8_lossy(s).into_owned())
}

/// The player's inventory: after its record header, a u32 count, then one 27-byte entry each
/// (u32 item id, u32 count, u32 most the player can carry, two more u32s, two flag bytes, and a
/// 5-byte link to the next entry).
const INV: &[u8] = b"SCHK\x1d\x01\x00\x0b\x00";
const ENTRY: usize = 27;

/// Item ids matched to the in-game inventory of a known save.
const ITEMS: [(u32, &str); 9] = [
    (0xb2e8_21bb, "Runes"),
    (0x2702_011a, "Crossbow bolts"),
    (0x56f6_376a, "Pistol bullets"),
    (0x1f03_fc80, "Incendiary bolts"),
    (0xdc2d_4e4a, "Grenades"),
    (0x5f49_6480, "Spring razors"),
    (0x4871_3317, "Stun mines"),
    (0xf7cc_212f, "Sleep darts or special bolts (A)"),
    (0xbf8c_9a34, "Sleep darts or special bolts (B)"),
];

/// Where each known item's count is, with the most the game lets the player carry.
fn inventory(p: &[u8]) -> Vec<(&'static str, usize, u32)> {
    let u32_at = |i: usize| p.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let mut from = 0;
    while let Some(k) = p[from..].windows(INV.len()).position(|w| w == INV) {
        let at = from + k;
        from = at + 1;
        let (Some(n), st) = (u32_at(at + 17), at + 21) else { break };
        let n = n as usize;
        if n == 0 || n > 10_000 || st + ENTRY * n > p.len() + 5 || (0..n - 1).any(|i| p[st + ENTRY * i + 26] != 1) {
            continue;
        }
        let mut out: Vec<(&str, usize, u32)> = Vec::new();
        for i in 0..n {
            let e = st + ENTRY * i;
            let (Some(id), Some(count), Some(max)) = (u32_at(e), u32_at(e + 4), u32_at(e + 8)) else { continue };
            if let Some(&(_, name)) = ITEMS.iter().find(|(x, _)| *x == id) {
                if count <= max && max <= 100_000 && !out.iter().any(|(n, ..)| *n == name) {
                    out.push((name, e + 4, max));
                }
            }
        }
        return out;
    }
    Vec::new()
}

#[derive(Clone, Debug)]
struct Save {
    orig: Vec<u8>,
    plain: Vec<u8>,
    whalebone: usize,
    /// (offset of the u32 count, most allowed) per leaf after the whalebone.
    items: Vec<(usize, u32)>,
}

#[derive(Clone, Debug)]
struct B(Save);

const WHALEBONE: usize = 0;

impl B {
    fn max(&self, leaf: usize) -> i64 {
        match leaf {
            WHALEBONE => WHALEBONE_MAX,
            l => self.0.items.get(l - 1).map_or(0, |i| i.1 as i64),
        }
    }
}

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.0.orig.clone());
        }
        let mut p = self.0.plain.clone();
        for (leaf, v) in changes {
            let n = v.as_i64().filter(|n| (0..=self.max(*leaf)).contains(n)).ok_or("Value out of range")?;
            match *leaf {
                WHALEBONE => {
                    let at = self.0.whalebone;
                    p[at..at + 4].copy_from_slice(&(n as f32).to_le_bytes());
                }
                l => {
                    let at = self.0.items.get(l - 1).ok_or("Not editable")?.0;
                    p[at..at + 4].copy_from_slice(&(n as u32).to_le_bytes());
                }
            }
        }
        let out = encode(&p);
        // Never hand back a file that doesn't read back to exactly what was meant.
        match decode(&out) {
            Some(back) if back == p => Ok(out),
            _ => Err("The save could not be packed again safely".into()),
        }
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let max = self.max(leaf);
        match v.as_i64() {
            Some(n) if (0..=max).contains(&n) => Ok(()),
            _ => Err(format!("Must be a whole number from 0 to {max}")),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let plain = decode(d)?;
    let (whalebone, traits) = crafting(&plain)?;
    let bone = f32::from_le_bytes(plain[whalebone..whalebone + 4].try_into().ok()?) as i64;
    let mut edit = HashMap::new();
    let mut crafting_m = Map::new();
    crafting_m.insert("Raw Whalebone".into(), Value::from(bone));
    edit.insert("/Crafting/Raw Whalebone".to_string(), WHALEBONE);
    let mut inv = Map::new();
    let mut items = Vec::new();
    for (name, at, max) in inventory(&plain) {
        let n = u32::from_le_bytes(plain[at..at + 4].try_into().ok()?);
        inv.insert(name.into(), Value::from(n));
        items.push((at, max));
        edit.insert(format!("/Inventory/{name}"), items.len());
    }
    let mut info = Map::new();
    if let Some(m) = map_name(&plain) {
        info.insert("Map".into(), Value::from(m));
    }
    info.insert("Bonecharm traits learned".into(), Value::from(traits as i64));
    let mut root = Map::new();
    root.insert("Crafting".into(), Value::Object(crafting_m));
    if !inv.is_empty() {
        root.insert("Inventory".into(), Value::Object(inv));
    }
    root.insert("Save".into(), Value::Object(info));
    Some(TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B(Save { orig: d.to_vec(), plain, whalebone, items }))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small game state: header, a map name, filler that compresses and some that doesn't,
    /// and the crafting record with 59 whalebone.
    fn plain() -> Vec<u8> {
        let mut p = vec![0u8; 4];
        p.extend(b"ARKS6\0\0\0\x03\0MD");
        let name = b"campaign/boat/curator/boat_curator_p";
        p.extend((name.len() as u32).to_le_bytes());
        p.extend(name);
        let mut x = 0x1234_5678u32;
        for i in 0..40_000u32 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            p.push(if i % 3 == 0 { (x >> 24) as u8 } else { b"SCHKstoryflags"[(i % 14) as usize] });
        }
        p.extend(CRAFT);
        p.extend(2u16.to_le_bytes());
        for (h, v) in [(0xcab1_a1deu32, 4i32), (0xf3d8_eb90, 4)] {
            p.extend(h.to_le_bytes());
            p.extend(v.to_le_bytes());
        }
        p.extend(1u16.to_le_bytes());
        p.extend(0x00d3_7522u32.to_le_bytes());
        p.extend(0i32.to_le_bytes());
        p.extend(59.0f32.to_le_bytes());
        p.extend(7u32.to_le_bytes());
        p.extend(b"SCHK\x1d\x01\x00\x1c\x00");
        p.extend(INV);
        p.extend([0u8; 8]);
        p.extend(3u32.to_le_bytes());
        for (i, (id, count, max)) in [(0xb2e8_21bbu32, 3u32, 10_000u32), (0x1234_5678, 1, 1), (0x2702_011a, 20, 20)].iter().enumerate() {
            p.extend(id.to_le_bytes());
            for v in [*count, *max, 0, 0] {
                p.extend(v.to_le_bytes());
            }
            p.extend([1, 1]);
            if i < 2 {
                p.extend(b"\x10\x00\xff\x3f\x01");
            }
        }
        p.extend(vec![0u8; 20_000]);
        let len = (p.len() - 4) as u32;
        p[..4].copy_from_slice(&len.to_le_bytes());
        p
    }

    #[test]
    fn matches_never_overlap_their_output() {
        let mut d = b"abababababababababababab".repeat(50);
        d.extend(vec![7u8; 5000]);
        // Past the 128 KiB line the same text must be written out again, not referenced.
        d.resize(LZ_BLOCK - 600, 1);
        d.extend(b"abababababababababababab".repeat(50));
        let z = lz_encode(&d);
        let (mut i, mut out) = (0, 0usize);
        while i < z.len() {
            if z[i] == 0 {
                i += 2;
                out += 1;
            } else {
                let off = u16::from_le_bytes([z[i + 1], z[i + 2]]) as usize;
                assert!(off >= z[i] as usize + 3, "match at {out} overlaps");
                assert!(out - off >= out - out % LZ_BLOCK, "match at {out} reaches past the 128 KiB line");
                out += z[i] as usize + 3;
                i += 3;
            }
        }
        assert_eq!(lz_decode(&z), d);
    }

    #[test]
    fn codec_round_trips_across_blocks() {
        let p = plain();
        let d = encode(&p);
        assert!(looks(&d));
        assert!(d.len() < p.len());
        assert_eq!(decode(&d).unwrap(), p);
    }

    #[test]
    fn unchanged_save_is_written_back_byte_for_byte() {
        let d = encode(&plain());
        let doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
    }

    #[test]
    fn reads_and_edits_whalebone() {
        let d = encode(&plain());
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Crafting/Raw Whalebone").unwrap(), 59);
        assert_eq!(doc.view.pointer("/Save/Bonecharm traits learned").unwrap(), 2);
        assert_eq!(doc.view.pointer("/Save/Map").unwrap(), "campaign/boat/curator/boat_curator_p");
        assert_eq!(doc.view.pointer("/Inventory/Runes").unwrap(), 3);
        assert_eq!(doc.view.pointer("/Inventory/Crossbow bolts").unwrap(), 20);
        assert!(doc.view.pointer("/Inventory/Grenades").is_none());
        assert!(doc.set("/Inventory/Crossbow bolts", Value::from(21)).is_err());
        assert!(doc.set("/Crafting/Raw Whalebone", Value::from(1000)).is_err());
        doc.set("/Crafting/Raw Whalebone", Value::from(999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Crafting/Raw Whalebone").unwrap(), 999);
        // Nothing else in the state moved.
        let (a, b) = (decode(&d).unwrap(), decode(&out).unwrap());
        let diff: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
        assert!(!diff.is_empty() && diff.len() <= 4 && diff[diff.len() - 1] - diff[0] < 4, "only the float changes");
        let mut doc = parse(&out).unwrap();
        doc.set("/Inventory/Runes", Value::from(50)).unwrap();
        let back = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(back.view.pointer("/Inventory/Runes").unwrap(), 50);
        assert_eq!(back.view.pointer("/Crafting/Raw Whalebone").unwrap(), 999);
    }

    #[test]
    fn corrupt_file_is_not_taken() {
        let mut d = encode(&plain());
        let n = d.len();
        d[n / 2] ^= 0x10;
        assert!(parse(&d).is_none());
    }
}
