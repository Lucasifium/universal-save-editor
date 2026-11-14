//! RollerCoaster Tycoon 2 saved games (`.SV6`). The file is a row of chunks, each a one-byte
//! encoding (0 none, 1 run-length, 2 run-length plus repeats, 3 rotate), a 32-bit length and the
//! data, followed by a 32-bit checksum that is the plain sum of every byte before it. The last
//! chunk holds the park (entities, finances, settings); that chunk is what the editor opens, and
//! on save it is written back run-length packed with the file sum made right again. The earlier
//! chunks (header, objects, map) are copied as they were.
//!
//! The park's cash is kept scrambled (`rol32(x ^ 0xF4EC9621, 13)`); it is unscrambled here and
//! scrambled again on save. Chunk layout, the scrambling and the field positions follow
//! OpenRCT2 (GPL-3.0); see THIRD_PARTY_NOTICES.md.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::Arc;

pub const KIND: &str = "RollerCoaster Tycoon 2 save";

/// What follows the entity list in the park chunk.
const TAIL: usize = 488_812;
const ENTITY: usize = 256;
const MAGIC: u32 = 0x0003_1144;

const CASH: usize = 5180;
const INITIAL_CASH: usize = 32;
const LOAN: usize = 36;
const FEE: usize = 44;
const MAX_LOAN: usize = 3380;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    /// Every chunk before the park chunk, as in the file.
    prefix: Arc<Vec<u8>>,
    /// The park chunk as unpacked (before the cash was unscrambled), to hand back an unchanged
    /// file untouched.
    original: Arc<Vec<u8>>,
    file: Arc<Vec<u8>>,
}

fn ror8(b: u8, n: u32) -> u8 {
    b.rotate_right(n)
}

fn rle_decode(s: &[u8]) -> Option<Vec<u8>> {
    let mut o = Vec::with_capacity(s.len() * 2);
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c & 0x80 != 0 {
            let n = 257 - c as usize;
            o.resize(o.len() + n, *s.get(i + 1)?);
            i += 2;
        } else {
            let n = c as usize + 1;
            o.extend_from_slice(s.get(i + 1..i + 1 + n)?);
            i += 1 + n;
        }
        if o.len() > 0x0100_0000 {
            return None;
        }
    }
    Some(o)
}

fn repeat_decode(s: &[u8]) -> Option<Vec<u8>> {
    let mut o: Vec<u8> = Vec::with_capacity(s.len() * 2);
    let mut i = 0;
    while i < s.len() {
        if s[i] == 0xFF {
            o.push(*s.get(i + 1)?);
            i += 2;
        } else {
            let n = (s[i] & 7) as usize + 1;
            let back = 32 - (s[i] >> 3) as usize;
            let start = o.len().checked_sub(back)?;
            for k in 0..n {
                let b = *o.get(start + k)?;
                o.push(b);
            }
            i += 1;
        }
    }
    Some(o)
}

fn decode(enc: u8, s: &[u8]) -> Option<Vec<u8>> {
    match enc {
        0 => Some(s.to_vec()),
        1 => rle_decode(s),
        2 => repeat_decode(&rle_decode(s)?),
        3 => {
            let mut code = 1u32;
            Some(
                s.iter()
                    .map(|&b| {
                        let v = ror8(b, code);
                        code = (code + 2) % 8;
                        v
                    })
                    .collect(),
            )
        }
        _ => None,
    }
}

/// Run-length packing as the game reads it: a byte under 128 copies that many plus one bytes, a
/// byte from 129 repeats the next byte `257 - n` times.
fn rle_encode(s: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(s.len() / 2);
    let mut lit_start = 0;
    let mut i = 0;
    let flush = |o: &mut Vec<u8>, from: usize, to: usize| {
        let mut p = from;
        while p < to {
            let n = (to - p).min(128);
            o.push((n - 1) as u8);
            o.extend_from_slice(&s[p..p + n]);
            p += n;
        }
    };
    while i < s.len() {
        let mut run = 1;
        while i + run < s.len() && s[i + run] == s[i] && run < 128 {
            run += 1;
        }
        if run >= 3 {
            flush(&mut o, lit_start, i);
            o.push((257 - run) as u8);
            o.push(s[i]);
            i += run;
            lit_start = i;
        } else {
            i += run;
        }
    }
    flush(&mut o, lit_start, s.len());
    o
}

fn sum(d: &[u8]) -> u32 {
    d.iter().fold(0u32, |a, &b| a.wrapping_add(b as u32))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn unscramble(x: u32) -> u32 {
    (x ^ 0xF4EC_9621).rotate_left(13)
}

fn scramble(x: u32) -> u32 {
    x.rotate_right(13) ^ 0xF4EC_9621
}

/// Cheap test before any unpacking: the first chunk is the rotated 32-byte header.
pub fn maybe(d: &[u8]) -> bool {
    d.len() > 100_000 && d[0] == 3 && u32_at(d, 1) == Some(0x20)
}

fn tail_at(inner: &[u8]) -> Option<usize> {
    let n = inner.len().checked_sub(4 + TAIL)?;
    (n % ENTITY == 0 && matches!(n / ENTITY, 10_000 | 15_000)).then_some(inner.len() - TAIL)
}

/// The park chunk of a saved game (cash unscrambled), when the file's sum is right.
pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !maybe(d) {
        return None;
    }
    let body = d.len() - 4;
    if sum(&d[..body]) != u32_at(d, body)? {
        return None;
    }
    let mut p = 0;
    let mut chunks = Vec::new();
    while p < body {
        let enc = d[p];
        let len = u32_at(d, p + 1)? as usize;
        let end = p + 5 + len;
        if end > body {
            return None;
        }
        chunks.push((p, enc, end));
        p = end;
    }
    let header = decode(chunks.first()?.1, &d[5..chunks[0].2])?;
    // A saved game: type 0, the game's own magic number, no packed objects.
    if header.len() != 0x20 || header[0] != 0 || header[2] != 0 || header[3] != 0 || u32_at(&header, 8)? != MAGIC || chunks.len() != 5 {
        return None;
    }
    let (start, enc, end) = *chunks.last()?;
    let mut inner = decode(enc, &d[start + 5..end])?;
    let tail = tail_at(&inner)?;
    let original = Arc::new(inner.clone());
    let cash = u32_at(&inner, tail + CASH)?;
    inner[tail + CASH..tail + CASH + 4].copy_from_slice(&unscramble(cash).to_le_bytes());
    Some((Packed { prefix: Arc::new(d[..start].to_vec()), original, file: Arc::new(d.to_vec()) }, inner))
}

pub fn close(p: &Packed, data: &[u8]) -> Result<Vec<u8>, String> {
    let tail = tail_at(data).ok_or("the park data has the wrong size")?;
    let mut inner = data.to_vec();
    let cash = u32_at(&inner, tail + CASH).ok_or("short park data")?;
    inner[tail + CASH..tail + CASH + 4].copy_from_slice(&scramble(cash).to_le_bytes());
    if inner == **p.original {
        return Ok((*p.file).clone());
    }
    let packed = rle_encode(&inner);
    let mut out = (*p.prefix).clone();
    out.push(1);
    out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
    out.extend_from_slice(&packed);
    let s = sum(&out);
    out.extend_from_slice(&s.to_le_bytes());
    Ok(out)
}

pub fn looks(d: &[u8]) -> bool {
    tail_at(d).is_some_and(|t| u32_at(d, t + 5120) == Some(MAGIC))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let t = tail_at(d).ok_or("not a RollerCoaster Tycoon 2 save")?;
    let mut b = Builder::new(d);
    let money = |b: &mut Builder, name: &str, at: usize, hi: i64| {
        b.ranged(&["Park", name], t + at, Kind::TenthsI32, -100_000, hi);
    };
    money(&mut b, "Cash", CASH, 200_000_000);
    // The game also keeps the starting cash beside it; saves from a park that has not been
    // played keep both the same, so they are moved together.
    let cash = u32_at(d, t + CASH).unwrap_or(0);
    if u32_at(d, t + INITIAL_CASH) == Some(cash) {
        b.mirror(&["Park", "Cash"], t + INITIAL_CASH);
    }
    money(&mut b, "Bank loan", LOAN, 200_000_000);
    money(&mut b, "Maximum loan", MAX_LOAN, 200_000_000);
    b.ranged(&["Park", "Entrance fee (tenths of a unit)"], t + FEE, Kind::I16, 0, 3_000);
    b.info(&["Info", "Entities in the park"], (d.len() - TAIL - 4) / ENTITY);
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Close the game first. Cash, loan and fees are in whole currency units with one decimal (the game keeps tenths). \
         The park's chunk is unpacked and packed again on save, with the file's sum written right.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_round_trip() {
        let mut v = vec![0u8; 1000];
        v.extend((0..=255u8).cycle().take(700));
        v.extend(std::iter::repeat(7).take(300));
        v.push(1);
        v.push(2);
        assert_eq!(rle_decode(&rle_encode(&v)).unwrap(), v);
    }

    #[test]
    fn cash_scrambling_round_trip() {
        for x in [0u32, 1, 97_269_310, 0xFFFF_FFFF] {
            assert_eq!(unscramble(scramble(x)), x);
        }
    }

    /// Run with `SV6_SAMPLE=<path to an .SV6>` to check a real park (not part of the normal test run).
    #[test]
    #[ignore]
    fn real_park_round_trip() {
        use crate::formats::{decode, encode, Content};
        let Ok(path) = std::env::var("SV6_SAMPLE") else { return };
        let raw = std::fs::read(path).unwrap();
        let mut d = decode(&raw);
        assert!(matches!(d.content, Content::Tree(_)), "park not recognised");
        assert_eq!(encode(&d.layers, &d.content).unwrap(), raw, "unchanged park must be written byte for byte");
        let Content::Tree(t) = &mut d.content else { unreachable!() };
        t.set("/Park/Cash", serde_json::json!(123456.7)).unwrap();
        let bytes = encode(&d.layers, &d.content).unwrap();
        let again = decode(&bytes);
        let Content::Tree(t2) = &again.content else { panic!("edited park not recognised") };
        assert_eq!(t2.view.pointer("/Park/Cash"), Some(&serde_json::json!(123456.7)));
        let Content::Tree(t1) = &d.content else { unreachable!() };
        assert_eq!(t1.view, t2.view);
    }
}
