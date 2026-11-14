//! Assassin's Creed saves from Brotherhood to Odyssey (the "Anvil" engine family), as the PC
//! versions write them.
//!
//! File layers, outermost first (all worked out from public sample saves):
//!
//! * A Ubisoft Connect wrapper in front of the body: `u32 length of the rest of the header`,
//!   `u32 0`, then the header (0x224, 0x24 or 0x204 bytes); cracked copies use `CDX` plus a
//!   0x110 byte header, or 0x208 zero bytes. Older games write the body bare.
//! * The body is XOR-ed with a 16-byte key, `plain[i] = file[i] ^ key[(len - i) % 16]` (len is
//!   the body's length). The key is the MD5 of the account id, or a fixed key; it is
//!   recovered here from the known plain bytes of the block header, so no id is needed and
//!   the same key is used when the file is written again. Brotherhood is not encrypted.
//! * The body starts `u32 version, AC DB FE 00` and holds containers: `33 AA FB 57 99 FA 04 10`,
//!   `u16 version`, `u8 compression` (0 LZO1X, 2 LZO2A), `u32 block size`, then blocks
//!   `01 | u32 packed | u32 unpacked | u32 checksum | packed bytes` (checksum: Adler-32 with
//!   both sums starting at 0, over the packed bytes) or `00 | u32 size | u32 0 | raw bytes`.
//!   Each container's packed size is also stored in the 16 bytes before it.
//!
//! The decompressed containers are joined behind an `ANVIL` marker and handed to the reader
//! ([`parse`]). On saving only the blocks whose bytes changed are packed again; every other
//! block is copied back as it was, and the checksums and size fields are written again.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::Arc;

pub const KIND: &str = "Assassin's Creed save";
const SIG: [u8; 8] = [0x33, 0xAA, 0xFB, 0x57, 0x99, 0xFA, 0x04, 0x10];
const MARK: &[u8; 8] = b"ANVIL\0\0\0";

// ---------------------------------------------------------------- LZO1X

fn ext_len(src: &[u8], ip: &mut usize, base: usize) -> Option<usize> {
    let mut l = 0usize;
    while *src.get(*ip)? == 0 {
        l += 255;
        *ip += 1;
        if l > 1 << 24 {
            return None;
        }
    }
    l += base + *src.get(*ip)? as usize;
    *ip += 1;
    Some(l)
}

fn copy_match(out: &mut Vec<u8>, d: usize, len: usize) -> Option<()> {
    if d == 0 || d > out.len() {
        return None;
    }
    let start = out.len() - d;
    for k in 0..len {
        let b = out[start + k];
        out.push(b);
    }
    Some(())
}

/// LZO1X decompression, written from the stream format.
pub fn lzo1x_decompress(src: &[u8], cap: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(cap);
    let mut ip = 0usize;
    let mut state = 0u8;
    if *src.first()? > 17 {
        let t = (src[0] - 17) as usize;
        ip = 1;
        out.extend_from_slice(src.get(ip..ip + t)?);
        ip += t;
        state = if t < 4 { 1 } else { 4 };
    }
    loop {
        let t = *src.get(ip)? as usize;
        ip += 1;
        let (len, d, s);
        if t >= 64 {
            d = 1 + ((t >> 2) & 7) + ((*src.get(ip)? as usize) << 3);
            ip += 1;
            len = (t >> 5) + 1;
            s = t & 3;
        } else if t >= 32 {
            let mut l = t & 31;
            l = if l == 0 { ext_len(src, &mut ip, 31)? } else { l };
            len = l + 2;
            let v = u16::from_le_bytes(src.get(ip..ip + 2)?.try_into().ok()?) as usize;
            ip += 2;
            d = 1 + (v >> 2);
            s = v & 3;
        } else if t >= 16 {
            let mut l = t & 7;
            l = if l == 0 { ext_len(src, &mut ip, 7)? } else { l };
            len = l + 2;
            let v = u16::from_le_bytes(src.get(ip..ip + 2)?.try_into().ok()?) as usize;
            ip += 2;
            d = 16384 + ((t & 8) << 11) + (v >> 2);
            s = v & 3;
            if d == 16384 {
                break;
            }
        } else if state == 0 {
            let l = if t == 0 { ext_len(src, &mut ip, 15)? } else { t };
            out.extend_from_slice(src.get(ip..ip + l + 3)?);
            ip += l + 3;
            state = 4;
            continue;
        } else {
            let b = *src.get(ip)? as usize;
            ip += 1;
            if state < 4 {
                d = 1 + (t >> 2) + (b << 2);
                len = 2;
            } else {
                d = 1 + 0x800 + (t >> 2) + (b << 2);
                len = 3;
            }
            s = t & 3;
        }
        copy_match(&mut out, d, len)?;
        out.extend_from_slice(src.get(ip..ip + s)?);
        ip += s;
        state = s as u8;
        if out.len() > cap + 4096 {
            return None;
        }
    }
    Some(out)
}

fn hash4(d: &[u8]) -> usize {
    let v = u32::from_le_bytes([d[0], d[1], d[2], d[3]]);
    (v.wrapping_mul(0x9E37_79B1) >> 18) as usize
}

fn match_run(src: &[u8], a: usize, b: usize) -> usize {
    let mut l = 0;
    while b + l < src.len() && src[a + l] == src[b + l] {
        l += 1;
    }
    l
}

/// A simple greedy LZO1X compressor (matches up to 16 KiB back); every stream it makes is read
/// by [`lzo1x_decompress`] and by any standard LZO1X decoder.
pub fn lzo1x_compress(src: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(src.len() / 2 + 64);
    let mut table = vec![usize::MAX; 1 << 14];
    let (mut i, mut lit) = (0usize, 0usize);
    let mut last_match_at: Option<usize> = None; // index in `out` of the last match instruction's trailing-literal byte
    let mut first = true;
    let flush_lits = |out: &mut Vec<u8>, src: &[u8], from: usize, to: usize, last: &mut Option<usize>, first: &mut bool| {
        let n = to - from;
        if n == 0 {
            return;
        }
        if n <= 3 && !*first && last.is_some() {
            // Trailing literals ride in the previous match's low two bits.
            let at = last.unwrap();
            out[at] |= n as u8;
            out.extend_from_slice(&src[from..to]);
            return;
        }
        if *first && n <= 238 {
            out.push((n + 17) as u8);
        } else if n - 3 <= 15 && n >= 4 {
            out.push((n - 3) as u8);
        } else if n >= 4 {
            out.push(0);
            let mut r = n - 3 - 15;
            while r > 255 {
                out.push(0);
                r -= 255;
            }
            out.push(r as u8);
        } else {
            // 1..3 literals with no previous match to carry them: use the long form.
            out.push(0);
            out.push(0);
            out.extend_from_slice(&src[from..to]);
            // (unreachable for streams made here, kept valid anyway)
            return;
        }
        out.extend_from_slice(&src[from..to]);
    };
    while i + 4 <= src.len() {
        let h = hash4(&src[i..]);
        let cand = table[h];
        table[h] = i;
        if cand != usize::MAX && i - cand <= 16384 && src[cand..cand + 4] == src[i..i + 4] {
            let len = match_run(src, cand, i);
            let d = i - cand;
            // The 1..3 literals before this match must be carried by the previous match.
            let lits = i - lit;
            if lits > 0 && lits <= 3 && (first || last_match_at.is_none()) {
                i += 1;
                continue;
            }
            flush_lits(&mut out, src, lit, i, &mut last_match_at, &mut first);
            first = false;
            if len <= 8 && d <= 2048 {
                out.push((((len - 1) << 5) | (((d - 1) & 7) << 2)) as u8);
                out.push(((d - 1) >> 3) as u8);
                last_match_at = Some(out.len() - 2);
            } else {
                if len - 2 <= 31 {
                    out.push(32 | (len - 2) as u8);
                } else {
                    out.push(32);
                    let mut r = len - 2 - 31;
                    while r > 255 {
                        out.push(0);
                        r -= 255;
                    }
                    out.push(r as u8);
                }
                let v = ((d - 1) << 2) as u16;
                out.extend_from_slice(&v.to_le_bytes());
                last_match_at = Some(out.len() - 2);
            }
            i += len;
            lit = i;
        } else {
            i += 1;
        }
    }
    flush_lits(&mut out, src, lit, src.len(), &mut last_match_at, &mut first);
    out.extend_from_slice(&[17, 0, 0]);
    out
}

// ---------------------------------------------------------------- LZO2A

struct Bits<'a> {
    s: &'a [u8],
    ip: usize,
    b: u8,
    n: u8,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<usize> {
        if self.n == 0 {
            self.b = *self.s.get(self.ip)?;
            self.ip += 1;
            self.n = 8;
        }
        let v = self.b & 1;
        self.b >>= 1;
        self.n -= 1;
        Some(v as usize)
    }
    fn byte(&mut self) -> Option<usize> {
        let v = *self.s.get(self.ip)? as usize;
        self.ip += 1;
        Some(v)
    }
}

/// LZO2A decompression (Brotherhood): bits are taken from the low end of each byte; the first
/// bit of a pair is the low bit of the length.
pub fn lzo2a_decompress(src: &[u8], cap: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(cap);
    let mut r = Bits { s: src, ip: 0, b: 0, n: 0 };
    loop {
        if r.bit()? == 0 {
            let b = r.byte()? as u8;
            out.push(b);
            continue;
        }
        let (len, d);
        if r.bit()? == 0 {
            let a = r.bit()?;
            let b = r.bit()?;
            len = 2 + ((b << 1) | a);
            d = r.byte()? + 1;
        } else {
            let t = r.byte()?;
            let u = r.byte()?;
            d = (t & 31) | (u << 5);
            let l = t >> 5;
            if l == 0 {
                let mut l = 9usize;
                while *src.get(r.ip)? == 0 {
                    l += 255;
                    r.ip += 1;
                }
                l += r.byte()?;
                len = l;
            } else {
                if d == 0 {
                    break;
                }
                len = l + 2;
            }
        }
        copy_match(&mut out, d, len)?;
        if out.len() > cap + 4096 {
            return None;
        }
    }
    Some(out)
}

struct BitW {
    out: Vec<u8>,
    at: usize,
    n: u8,
}

impl BitW {
    fn bit(&mut self, v: usize) {
        if self.n == 0 {
            self.at = self.out.len();
            self.out.push(0);
            self.n = 8;
        }
        let used = 8 - self.n;
        self.out[self.at] |= ((v & 1) as u8) << used;
        self.n -= 1;
    }
}

/// A greedy LZO2A compressor (matches up to 8 KiB back).
pub fn lzo2a_compress(src: &[u8]) -> Vec<u8> {
    let mut w = BitW { out: Vec::with_capacity(src.len() / 2 + 64), at: 0, n: 0 };
    let mut table = vec![usize::MAX; 1 << 14];
    let mut i = 0usize;
    while i < src.len() {
        let mut best = (0usize, 0usize);
        if i + 4 <= src.len() {
            let h = hash4(&src[i..]);
            let cand = table[h];
            table[h] = i;
            if cand != usize::MAX && i - cand <= 8191 && src[cand..cand + 4] == src[i..i + 4] {
                best = (match_run(src, cand, i), i - cand);
            }
        }
        let (len, d) = best;
        if len >= 4 {
            if len <= 5 && d <= 256 {
                w.bit(1);
                w.bit(0);
                let l = len - 2;
                w.bit(l & 1);
                w.bit(l >> 1);
                w.out.push((d - 1) as u8);
            } else {
                w.bit(1);
                w.bit(1);
                if len <= 9 {
                    w.out.push((((len - 2) << 5) | (d & 31)) as u8);
                    w.out.push((d >> 5) as u8);
                } else {
                    w.out.push((d & 31) as u8);
                    w.out.push((d >> 5) as u8);
                    let mut r = len - 9;
                    while r > 255 {
                        w.out.push(0);
                        r -= 255;
                    }
                    w.out.push(r as u8);
                }
            }
            for k in 1..len {
                if i + k + 4 <= src.len() {
                    let h = hash4(&src[i + k..]);
                    table[h] = i + k;
                }
            }
            i += len;
        } else {
            w.bit(0);
            w.out.push(src[i]);
            i += 1;
        }
    }
    w.bit(1);
    w.bit(1);
    w.out.extend_from_slice(&[0x20, 0]);
    w.out
}

// ---------------------------------------------------------------- container

pub fn adler0(d: &[u8]) -> u32 {
    let (mut a, mut b) = (0u32, 0u32);
    for &x in d {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn u32_at(d: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn put32(d: &mut [u8], at: usize, v: usize) {
    d[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Blk {
    /// Start and end of the block in the body.
    at: usize,
    end: usize,
    /// Where its bytes sit in the joined inner data (after the marker).
    inner_at: usize,
    len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Part {
    start: usize,
    end: usize,
    comp: u8,
    ver: u16,
    bs: u32,
    /// Where the container's packed size is stored (body offset) and what is added to it.
    size_at: Option<(usize, usize)>,
    blocks: Vec<Blk>,
}

/// Where the parts sit in the original file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    original: Arc<Vec<u8>>,
    inner: Arc<Vec<u8>>,
    body_at: usize,
    key: Option<Vec<u8>>,
    parts: Vec<Part>,
}

fn body_start(d: &[u8]) -> usize {
    if d.starts_with(b"CDX\0") {
        return 0x110;
    }
    if let (Some(a), Some(b)) = (u32_at(d, 0), u32_at(d, 4)) {
        if b == 0 && matches!(a, 0x224 | 0x24 | 0x204) {
            return a + 4;
        }
    }
    if d.len() > 0x208 && d[..0x208].iter().all(|&b| b == 0) {
        return 0x208;
    }
    0
}

fn xor(body: &[u8], key: &[u8]) -> Vec<u8> {
    let n = body.len();
    body.iter().enumerate().map(|(i, &b)| b ^ key[(n - i) % key.len()]).collect()
}

/// Works out the 16-byte key from the known bytes of the first container's header.
fn solve_key(body: &[u8]) -> Option<Vec<u8>> {
    let n = body.len();
    for off in [16usize, 20, 24, 28, 32, 36] {
        if off + 28 > n {
            continue;
        }
        for ver in 1u16..=3 {
            for typ in [0u8, 2] {
                for bs in [0x8000u32, 0x40000] {
                    let mut pt = SIG.to_vec();
                    pt.extend_from_slice(&ver.to_le_bytes());
                    pt.push(typ);
                    pt.extend_from_slice(&bs.to_le_bytes());
                    pt.push(1);
                    let mut key = [0u8; 16];
                    for (j, p) in pt.iter().enumerate() {
                        let i = off + j;
                        key[(n - i) % 16] = body[i] ^ p;
                    }
                    let k = key.to_vec();
                    let head: Vec<u8> = (0..8).map(|i| body[i] ^ k[(n - i) % 16]).collect();
                    if head[4..8] != [0xAC, 0xDB, 0xFE, 0x00] {
                        continue;
                    }
                    let cs = u32_at(&xor_range(body, &k, off + 16, off + 28), 0)?;
                    let blk = xor_range(body, &k, off + 28, (off + 28 + cs).min(n));
                    if blk.len() == cs && adler0(&blk) == u32_at(&xor_range(body, &k, off + 24, off + 28), 0)? as u32 {
                        return Some(k);
                    }
                }
            }
        }
    }
    None
}

fn xor_range(body: &[u8], key: &[u8], a: usize, b: usize) -> Vec<u8> {
    let n = body.len();
    (a..b.min(n)).map(|i| body[i] ^ key[(n - i) % key.len()]).collect()
}

fn unpack(comp: u8, packed: &[u8], size: usize) -> Option<Vec<u8>> {
    let o = match comp {
        0 => lzo1x_decompress(packed, size)?,
        2 => lzo2a_decompress(packed, size)?,
        _ => return None,
    };
    (o.len() == size).then_some(o)
}

fn pack(comp: u8, raw: &[u8]) -> Vec<u8> {
    if comp == 2 {
        lzo2a_compress(raw)
    } else {
        lzo1x_compress(raw)
    }
}

/// A cheap check before the key search: a Ubisoft wrapper, or a bare body.
pub fn maybe(d: &[u8]) -> bool {
    d.len() >= 64 && (body_start(d) > 0 || d[4..8] == [0xAC, 0xDB, 0xFE, 0x00])
}

/// The decompressed containers behind an `ANVIL` marker, when this is a supported save.
pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if d.len() < 64 {
        return None;
    }
    let body_at = body_start(d);
    let raw = &d[body_at..];
    let (body, key) = if raw.get(4..8)? == [0xAC, 0xDB, 0xFE, 0x00] {
        (raw.to_vec(), None)
    } else {
        let k = solve_key(raw)?;
        (xor(raw, &k), Some(k))
    };
    if body.get(4..8)? != [0xAC, 0xDB, 0xFE, 0x00] {
        return None;
    }
    let s0 = body.get(..64.min(body.len()))?.windows(8).position(|w| w == SIG)?;
    let mut inner = MARK.to_vec();
    let mut parts: Vec<Part> = vec![];
    let mut s = s0;
    loop {
        let ver = u16::from_le_bytes(body.get(s + 8..s + 10)?.try_into().ok()?);
        let comp = *body.get(s + 10)?;
        let bs = u32_at(&body, s + 11)? as u32;
        let usz = u32_at(&body, s - 4)?;
        let mut p = s + 15;
        let mut got = 0usize;
        let mut blocks = vec![];
        if usz > 1 << 28 {
            return None;
        }
        while got < usz {
            let at = p;
            let flag = *body.get(p)?;
            let (raw_bytes, end) = if flag == 1 {
                let cs = u32_at(&body, p + 1)?;
                let us = u32_at(&body, p + 5)?;
                let sum = u32_at(&body, p + 9)? as u32;
                let packed = body.get(p + 13..p + 13 + cs)?;
                if adler0(packed) != sum {
                    return None;
                }
                (unpack(comp, packed, us)?, p + 13 + cs)
            } else if flag == 0 {
                let sz = u32_at(&body, p + 1)?;
                (body.get(p + 9..p + 9 + sz)?.to_vec(), p + 9 + sz)
            } else {
                return None;
            };
            blocks.push(Blk { at, end, inner_at: inner.len(), len: raw_bytes.len() });
            got += raw_bytes.len();
            inner.extend_from_slice(&raw_bytes);
            p = end;
        }
        if got != usz {
            return None;
        }
        let used = p - s;
        let size_at = [(s.wrapping_sub(16), 12usize), (s.wrapping_sub(8), 4), (s.wrapping_sub(8), 12)]
            .into_iter()
            .find(|&(at, add)| at < s && u32_at(&body, at) == Some(used + add));
        parts.push(Part { start: s, end: p, comp, ver, bs, size_at, blocks });
        // The next container follows a 8, 12 or 16 byte record (…, packed size, unpacked size).
        match [8usize, 12, 16].into_iter().find(|&g| body.get(p + g..p + g + 8) == Some(&SIG[..])) {
            Some(g) => s = p + g,
            None => break,
        }
    }
    if parts.is_empty() {
        return None;
    }
    let packed = Packed { original: Arc::new(d.to_vec()), inner: Arc::new(inner.clone()), body_at, key, parts };
    Some((packed, inner))
}

/// Packs the (edited) inner data back into a save.
pub fn close(p: &Packed, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() != p.inner.len() || !data.starts_with(MARK) {
        return Err("the save's size changed; only in-place edits can be written back".into());
    }
    let raw = &p.original[p.body_at..];
    let plain = match &p.key {
        Some(k) => xor(raw, k),
        None => raw.to_vec(),
    };
    let mut out: Vec<u8> = Vec::with_capacity(plain.len() + 1024);
    let mut cursor = 0usize;
    for part in &p.parts {
        out.extend_from_slice(&plain[cursor..part.start]);
        let base = out.len();
        out.extend_from_slice(&plain[part.start..part.start + 15]);
        for b in &part.blocks {
            let (old, new) = (&p.inner[b.inner_at..b.inner_at + b.len], &data[b.inner_at..b.inner_at + b.len]);
            if old == new {
                out.extend_from_slice(&plain[b.at..b.end]);
                continue;
            }
            let packed = pack(part.comp, new);
            if plain[b.at] == 1 && packed.len() < new.len() {
                out.push(1);
                out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
                out.extend_from_slice(&(new.len() as u32).to_le_bytes());
                out.extend_from_slice(&adler0(&packed).to_le_bytes());
                out.extend_from_slice(&packed);
            } else {
                out.push(0);
                out.extend_from_slice(&(new.len() as u32).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(new);
            }
        }
        if let Some((at, add)) = part.size_at {
            let pos = out.len() - (out.len() - base) - (part.start - at);
            let used = out.len() - base;
            put32(&mut out, pos, used + add);
        }
        cursor = part.end;
    }
    out.extend_from_slice(&plain[cursor..]);
    let body = match &p.key {
        Some(k) => xor(&out, k),
        None => out,
    };
    let mut file = p.original[..p.body_at].to_vec();
    file.extend_from_slice(&body);
    Ok(file)
}

// ---------------------------------------------------------------- reader

/// Property leaves in the decompressed stream are `u32 L | key | ext | 00 00 | u16 type | 0B | value`
/// with `L` = 13 + the value's size; the keys are CRC-32 of the property names.
const K_CATEGORY: u32 = 0xFF3A7B97;
const K_COUNT: u32 = 0xF77B9912;
const K_ITEM_ID: u32 = 0xBF298A20;
const K_QUANTITY: u32 = 0x66857460;
const K_XP: u32 = 0x58D0_9457;
const K_POINTS: u32 = 0x6E5A_0AC8;
/// Item id of drachmae in Origins and Odyssey.
const DRACHMAE: u64 = 0x0000_001B_FDB9_F655;

fn crc32(b: &[u8]) -> u32 {
    let mut c = !0u32;
    for &x in b {
        c ^= x as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg());
        }
    }
    !c
}

/// `InvItem_*` category names that were matched by their CRC-32 against public saves.
const CATEGORY_NAMES: &[(&str, &str)] = &[
    ("Money", "Money"),
    ("SmokeBomb", "Smoke bombs"),
    ("Knives", "Throwing knives"),
    ("Poison", "Poison darts"),
    ("RopeDarts", "Rope darts"),
    ("Arrows", "Arrows"),
    ("Lockpicks", "Lockpicks"),
    ("Wood", "Wood"),
    ("Metal", "Metal"),
    ("Cloth", "Cloth"),
    ("Rum", "Rum"),
    ("Sugar", "Sugar"),
    ("Tobacco", "Tobacco"),
    ("HeavyShot", "Heavy shot"),
    ("Crew", "Crew"),
    ("Harpoon", "Harpoons"),
];

fn named(hash: u32) -> Option<&'static str> {
    CATEGORY_NAMES.iter().find(|(k, _)| crc32(format!("InvItem_{k}").as_bytes()) == hash).map(|(_, n)| *n)
}

/// Positions of every leaf with this key and length (the position is the key's).
fn leaves(d: &[u8], key: u32, len: u32) -> Vec<usize> {
    let kb = key.to_le_bytes();
    let mut out = vec![];
    let mut i = 4usize;
    while i + 4 <= d.len() {
        if d[i..i + 4] == kb && d[i - 4..i] == len.to_le_bytes() {
            out.push(i);
            i += 4;
        } else {
            i += 1;
        }
    }
    out
}

fn value_at(p: usize) -> usize {
    p + 13
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(MARK)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not an Assassin's Creed save".into());
    }
    let mut b = Builder::new(d);
    let cats = leaves(d, K_CATEGORY, 0x15);
    // Category -> (numeric id, [(count position, value)]) in file order.
    let mut by_cat: Vec<(u32, u32, Vec<(usize, u32)>)> = vec![];
    for (n, &p) in cats.iter().enumerate() {
        let hash = u32::from_le_bytes(d.get(value_at(p) + 4..value_at(p) + 8).ok_or("cut off")?.try_into().unwrap());
        let id = u32::from_le_bytes(d[value_at(p)..value_at(p) + 4].try_into().unwrap());
        let end = cats.get(n + 1).copied().unwrap_or(d.len()).min(p + 200);
        let Some(q) = leaves(&d[p..end.max(p)], K_COUNT, 0x11).first().map(|&x| x + p) else { continue };
        let v = u32::from_le_bytes(d.get(value_at(q)..value_at(q) + 4).ok_or("cut off")?.try_into().unwrap());
        match by_cat.iter_mut().find(|(h, _, _)| *h == hash) {
            Some((_, _, v_list)) => v_list.push((value_at(q), v)),
            None => by_cat.push((hash, id, vec![(value_at(q), v)])),
        }
    }
    let mut found = 0;
    for (hash, id, copies) in &by_cat {
        let name = named(*hash);
        let label = match name {
            Some(n) => n.to_string(),
            None => format!("Counter {id}"),
        };
        let group = match name {
            Some("Money") => "Inventory",
            Some("Wood" | "Metal" | "Cloth" | "Rum" | "Sugar" | "Tobacco") => "Cargo",
            Some(_) => "Inventory",
            None => "Other counters",
        };
        // The player's own count is the biggest one; other characters' and older copies come after.
        let first = *copies.iter().max_by_key(|c| (c.1, std::cmp::Reverse(c.0))).unwrap();
        if name.is_none() && first.1 == 0 {
            continue;
        }
        if b.ranged(&[group, &label], first.0, Kind::U32, 0, 1_000_000_000).is_some() {
            found += 1;
            let mut distinct: Vec<u32> = vec![];
            for &(at, v) in copies.iter().filter(|c| c.0 != first.0) {
                if v == first.1 {
                    b.mirror(&[group, &label], at);
                } else if name.is_some() && v != 0 {
                    match distinct.iter().position(|&x| x == v) {
                        Some(k) => b.mirror(&["Other copies", &format!("{label} (copy {})", k + 2)], at),
                        None => {
                            distinct.push(v);
                            b.ranged(&["Other copies", &format!("{label} (copy {})", distinct.len() + 1)], at, Kind::U32, 0, 1_000_000_000);
                        }
                    }
                }
            }
        }
    }
    // Origins and Odyssey keep items in a list: drachmae by item id, experience and ability points by key.
    let ids = leaves(d, K_ITEM_ID, 0x15);
    let mut drachmae: Vec<(usize, u32)> = vec![];
    for (n, &p) in ids.iter().enumerate() {
        if u64::from_le_bytes(d[value_at(p)..value_at(p) + 8].try_into().unwrap()) != DRACHMAE {
            continue;
        }
        let end = ids.get(n + 1).copied().unwrap_or(d.len()).min(p + 2000);
        if let Some(&q) = leaves(&d[p..end], K_QUANTITY, 0x11).first() {
            let at = value_at(q + p);
            drachmae.push((at, u32::from_le_bytes(d[at..at + 4].try_into().unwrap())));
        }
    }
    if let Some(&(at, v)) = drachmae.iter().max_by_key(|x| x.1) {
        b.ranged(&["Character", "Drachmae"], at, Kind::U32, 0, 999_999_999);
        for &(a2, v2) in &drachmae {
            if a2 != at && v2 == v {
                b.mirror(&["Character", "Drachmae"], a2);
            }
        }
        found += 1;
    }
    if !drachmae.is_empty() {
        // The live counters are the first non-zero ones of the first two copies.
        for (key, name, hi) in [(K_XP, "Experience", 999_999_999i64), (K_POINTS, "Ability points", 9_999)] {
            let ls = leaves(d, key, 0x11);
            if let Some(&p) = ls.iter().take(2).find(|&&p| u32::from_le_bytes(d[value_at(p)..value_at(p) + 4].try_into().unwrap()) != 0) {
                b.ranged(&["Character", name], value_at(p), Kind::U32, 0, hi);
            }
        }
    }
    if found == 0 {
        return Err("no money or items found in this save".into());
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Not every save slot is the one the game loads: edit the newest one. Another copy of a value (an older checkpoint) is listed under Other copies when it differs."))
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize) -> Vec<u8> {
        let mut v = vec![];
        let mut x = 12345u32;
        while v.len() < n {
            x = x.wrapping_mul(1103515245).wrapping_add(12345);
            let run = (x >> 16) as usize % 40;
            let b = (x >> 8) as u8 % 7;
            v.extend(std::iter::repeat(b).take(run));
            v.extend_from_slice(b"InventoryContainer");
            v.push((x >> 3) as u8);
        }
        v.truncate(n);
        v
    }

    #[test]
    fn lzo1x_round_trip() {
        for n in [0usize, 1, 2, 3, 4, 5, 17, 200, 4000, 70000] {
            let d = sample(n);
            let c = lzo1x_compress(&d);
            assert_eq!(lzo1x_decompress(&c, d.len()).unwrap(), d, "size {n}");
        }
    }

    #[test]
    fn lzo2a_round_trip() {
        for n in [0usize, 1, 2, 3, 4, 5, 17, 200, 4000, 70000] {
            let d = sample(n);
            let c = lzo2a_compress(&d);
            assert_eq!(lzo2a_decompress(&c, d.len()).unwrap(), d, "size {n}");
        }
    }

    /// `ANVIL_SAMPLES` = files separated by `|`; each must open, write back byte-identical, and
    /// survive a one-byte edit (the changed block is packed again).
    #[test]
    #[ignore]
    fn real_samples() {
        let Ok(list) = std::env::var("ANVIL_SAMPLES") else { return };
        for f in list.split('|') {
            let d = std::fs::read(f).unwrap();
            let Some((p, inner)) = open(&d) else {
                println!("{f}: not opened");
                continue;
            };
            let same = close(&p, &inner).unwrap();
            assert_eq!(same, d, "{f}: unchanged write differs");
            let mut edited = inner.clone();
            let at = edited.len() / 2;
            edited[at] ^= 0x55;
            let out = close(&p, &edited).unwrap();
            let (_, back) = open(&out).unwrap_or_else(|| panic!("{f}: edited file does not open"));
            assert_eq!(back, edited, "{f}: edit lost");
            println!("{f}: ok, parts {}, inner {} bytes, edited file {} (was {})", p.parts.len(), inner.len(), out.len(), d.len());
            if let Ok(dir) = std::env::var("ANVIL_DUMP") {
                let name = f.replace(['/', ' ', '\''], "_");
                std::fs::write(format!("{dir}/{name}.bin"), &inner).unwrap();
            }
        }
    }

    fn leaf(key: u32, ty: u16, value: &[u8]) -> Vec<u8> {
        let mut o = ((13 + value.len()) as u32).to_le_bytes().to_vec();
        o.extend(key.to_le_bytes());
        o.extend([0, 0, 0, 0, 0, 0]);
        o.extend(ty.to_le_bytes());
        o.push(0x0B);
        o.extend_from_slice(value);
        o
    }

    fn category(id: u32, name: &str, count: u32) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend(crc32(format!("InvItem_{name}").as_bytes()).to_le_bytes());
        let mut o = leaf(K_CATEGORY, 0x19, &v);
        o.extend(leaf(0x6AF7_D44F, 7, &0u32.to_le_bytes()));
        o.extend(leaf(K_COUNT, 7, &count.to_le_bytes()));
        o
    }

    #[test]
    fn reads_inventory_counts_and_copies() {
        assert_eq!(crc32(b"InvItem_Money"), 0x12F6_B6F1);
        let mut d = MARK.to_vec();
        d.extend([0u8; 14]);
        d.extend(category(1, "Money", 500));
        d.extend(category(43, "Cloth", 12));
        d.extend(category(1, "Money", 500)); // snapshot with the same value
        d.extend(category(1, "Money", 480)); // older checkpoint
        let mut t = parse(&d).unwrap();
        assert_eq!(t.view.pointer("/Inventory/Money").unwrap(), 500);
        assert_eq!(t.view.pointer("/Cargo/Cloth").unwrap(), 12);
        assert_eq!(t.view.pointer("/Other copies/Money (copy 2)").unwrap(), 480);
        t.set("/Inventory/Money", 9_999_999.into()).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let t2 = parse(&out).unwrap();
        assert_eq!(t2.view.pointer("/Inventory/Money").unwrap(), 9_999_999);
        // Both equal copies were written.
        assert_eq!(leaves(&out, K_COUNT, 0x11).iter().filter(|&&p| out[value_at(p)..value_at(p) + 4] == 9_999_999u32.to_le_bytes()).count(), 2);
    }

    /// `ANVIL_SAMPLES` files: open through the whole pipeline, change the first value, save, reopen.
    #[test]
    #[ignore]
    fn real_edit_through_layers() {
        let Ok(list) = std::env::var("ANVIL_SAMPLES") else { return };
        for f in list.split('|') {
            let raw = std::fs::read(f).unwrap();
            let mut d = crate::formats::decode(&raw);
            let crate::formats::Content::Tree(t) = &mut d.content else {
                println!("{f}: not a tree");
                continue;
            };
            let path = ["/Inventory/Money", "/Character/Drachmae"].into_iter().find(|p| t.view.pointer(p).is_some());
            let Some(path) = path else {
                println!("{f}: no money ({:?})", t.kind_name);
                continue;
            };
            let before = t.view.pointer(path).unwrap().clone();
            t.set(path, 99_999_999.into()).unwrap();
            let out = crate::formats::encode(&d.layers, &d.content).unwrap();
            let d2 = crate::formats::decode(&out);
            let crate::formats::Content::Tree(t2) = &d2.content else { panic!("{f}: reopened as hex") };
            assert_eq!(t2.view.pointer(path).unwrap(), 99_999_999);
            println!("{f}: {path} {before} -> 99999999 ok (file {} -> {})", raw.len(), out.len());
        }
    }
}
