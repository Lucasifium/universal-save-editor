//! The Witcher 2 saves (`*.sav`): a "DZIP" container (version 2) holding one file. The header is
//! the magic, the version, the file count, a constant 1, the offset of the index (u64) and a
//! 64-bit value kept as it is. The file's data starts at 0x20 with a table of u32 offsets (from
//! 0x20) to its 64 KB pieces, each packed with LZF; the index at the end gives the file's name,
//! time, unpacked size, offset and packed size.
//!
//! The file is a "SAVY" save: blocks (`BLCK` name size) and values (`AVAL` name type size data),
//! names written as a length byte (0x80 | length) and the text. Geralt's entity has properties
//! (`property` blocks of name, type and value) such as `experience` and `talents` (unspent
//! talent points); his inventory follows with items, one of them `Orens` with a `quantity`.
//!
//! Values are edited in place; the pieces are packed again and the container written anew.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "The Witcher 2 save";

const PIECE: usize = 0x1_0000;
/// Unpacked saves are a few MB; anything far larger is refused.
const MAX_SIZE: usize = 256 << 20;

fn u32le(d: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn u64le(d: &[u8], at: usize) -> Option<usize> {
    usize::try_from(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?)).ok()
}

/// The one file's index entry.
struct Entry {
    /// Where the index starts.
    index: usize,
    /// The name with its length field, as stored.
    name: Vec<u8>,
    time: [u8; 8],
    size: usize,
}

fn entry(d: &[u8]) -> Option<Entry> {
    if !d.starts_with(b"DZIP") || u32le(d, 4)? != 2 || u32le(d, 8)? != 1 {
        return None;
    }
    let index = u64le(d, 0x10)?;
    let n = u16::from_le_bytes(d.get(index..)?.get(..2)?.try_into().ok()?) as usize;
    let p = index + 2 + n;
    let time: [u8; 8] = d.get(p..p + 8)?.try_into().ok()?;
    let size = u64le(d, p + 8)?;
    let (off, packed) = (u64le(d, p + 16)?, u64le(d, p + 24)?);
    (off == 0x20 && off.checked_add(packed)? == index && p + 32 == d.len() && size <= MAX_SIZE && size > 0)
        .then(|| Entry { index, name: d[index..index + 2 + n].to_vec(), time, size })
}

pub fn looks(d: &[u8]) -> bool {
    entry(d).is_some()
}

/// LZF (liblzf's format): a control byte below 32 starts that many plus one literal bytes;
/// otherwise its top three bits are the length minus two (7: one more length byte follows) and
/// the low five bits with the next byte the distance back minus one.
fn unlzf(src: &[u8], limit: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(limit);
    let mut i = 0;
    while i < src.len() {
        let c = src[i] as usize;
        i += 1;
        if c < 32 {
            out.extend_from_slice(src.get(i..i + c + 1)?);
            i += c + 1;
        } else {
            let mut len = c >> 5;
            if len == 7 {
                len += *src.get(i)? as usize;
                i += 1;
            }
            let back = ((c & 31) << 8) + *src.get(i)? as usize + 1;
            i += 1;
            let from = out.len().checked_sub(back)?;
            for k in 0..len + 2 {
                let b = out[from + k];
                out.push(b);
            }
        }
        if out.len() > limit {
            return None;
        }
    }
    Some(out)
}

fn lzf(src: &[u8]) -> Vec<u8> {
    const HBITS: u32 = 14;
    let hash = |p: usize| -> usize {
        let v = (u32::from(src[p]) << 16) | (u32::from(src[p + 1]) << 8) | u32::from(src[p + 2]);
        (v.wrapping_mul(2_654_435_761) >> (32 - HBITS)) as usize
    };
    let mut table = vec![usize::MAX; 1 << HBITS];
    let mut out = Vec::with_capacity(src.len() / 2 + 16);
    let mut lit: Vec<u8> = Vec::new();
    let flush = |out: &mut Vec<u8>, lit: &mut Vec<u8>| {
        for c in lit.chunks(32) {
            out.push((c.len() - 1) as u8);
            out.extend_from_slice(c);
        }
        lit.clear();
    };
    let n = src.len();
    let mut i = 0;
    while i + 2 < n {
        let h = hash(i);
        let r = table[h];
        table[h] = i;
        if r != usize::MAX && i - r <= 8192 && src[r..r + 3] == src[i..i + 3] {
            let most = (n - i).min(264);
            let mut len = 3;
            while len < most && src[r + len] == src[i + len] {
                len += 1;
            }
            flush(&mut out, &mut lit);
            let (back, l) = (i - r - 1, len - 2);
            if l < 7 {
                out.push(((l << 5) | (back >> 8)) as u8);
            } else {
                out.push(((7 << 5) | (back >> 8)) as u8);
                out.push((l - 7) as u8);
            }
            out.push(back as u8);
            for k in i + 1..i + len {
                if k + 2 < n {
                    table[hash(k)] = k;
                }
            }
            i += len;
        } else {
            lit.push(src[i]);
            i += 1;
        }
    }
    lit.extend_from_slice(&src[i..]);
    flush(&mut out, &mut lit);
    out
}

fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    let e = entry(d)?;
    let pieces = e.size.div_ceil(PIECE);
    let mut out = Vec::with_capacity(e.size.min(d.len().saturating_mul(16)));
    for k in 0..pieces {
        let a = 0x20 + u32le(d, 0x20 + k * 4)?;
        let b = if k + 1 < pieces { 0x20 + u32le(d, 0x20 + (k + 1) * 4)? } else { e.index };
        let want = PIECE.min(e.size - out.len());
        let p = unlzf(d.get(a..b)?, want)?;
        if p.len() != want {
            return None;
        }
        out.extend(p);
    }
    Some(out)
}

/// The container for edited data, made like the original (same name, time and header value).
fn pack(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let Some(e) = entry(outer) else { return outer.to_vec() };
    let pieces: Vec<Vec<u8>> = inner.chunks(PIECE).map(lzf).collect();
    let mut body = Vec::new();
    let mut at = pieces.len() * 4;
    for p in &pieces {
        body.extend((at as u32).to_le_bytes());
        at += p.len();
    }
    for p in &pieces {
        body.extend(p);
    }
    let index = 0x20 + body.len();
    let mut out = outer[..0x10].to_vec();
    out.extend((index as u64).to_le_bytes());
    out.extend(&outer[0x18..0x20]);
    out.extend(body);
    out.extend(&e.name);
    out.extend(e.time);
    out.extend((inner.len() as u64).to_le_bytes());
    out.extend(0x20u64.to_le_bytes());
    out.extend(((index - 0x20) as u64).to_le_bytes());
    out
}

fn name(s: &str) -> Vec<u8> {
    let mut v = vec![0x80 | s.len() as u8];
    v.extend(s.as_bytes());
    v
}

/// The bytes of a `property` block up to its value's data, for a property `n` of type `t`.
fn property(n: &str, t: &str) -> Vec<u8> {
    let mut v = b"AVAL".to_vec();
    v.extend(name("name"));
    v.extend(name("String"));
    v.extend((n.len() as u32 + 1).to_le_bytes());
    v.extend(name(n));
    v.extend(b"AVAL");
    v.extend(name("type"));
    v.extend(name("String"));
    v.extend((t.len() as u32 + 1).to_le_bytes());
    v.extend(name(t));
    v.extend(b"AVAL");
    v.extend(name("value"));
    v.extend(name(t));
    v.extend(4u32.to_le_bytes());
    v
}

fn find_all(d: &[u8], pat: &[u8]) -> Vec<usize> {
    d.windows(pat.len()).enumerate().filter(|(_, w)| *w == pat).map(|(i, _)| i + pat.len()).collect()
}

/// Where the value of Geralt's property `n` (type `t`) is, when the save has exactly one.
fn player_value(d: &[u8], n: &str, t: &str) -> Option<usize> {
    let v = find_all(d, &property(n, t));
    (v.len() == 1).then(|| v[0])
}

/// Offsets of the quantity of every Orens item stack.
fn orens(d: &[u8]) -> Vec<usize> {
    let mut head = b"AVAL".to_vec();
    head.extend(name("name"));
    head.extend(name("String"));
    head.extend(6u32.to_le_bytes());
    head.extend(name("Orens"));
    let mut out = Vec::new();
    for p in find_all(d, &head) {
        // staticRandomSeed and flags (two bytes each), then the quantity
        let mut q = p;
        let mut ok = true;
        for (field, ty, len) in [("staticRandomSeed", "Uint16", 2u32), ("flags", "Uint16", 2), ("quantity", "Uint", 4)] {
            let mut want = b"AVAL".to_vec();
            want.extend(name(field));
            want.extend(name(ty));
            want.extend(len.to_le_bytes());
            if d.get(q..q + want.len()) != Some(&want[..]) {
                ok = false;
                break;
            }
            q += want.len() + if field == "quantity" { 0 } else { len as usize };
        }
        if ok && q + 4 <= d.len() {
            out.push(q);
        }
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Witcher 2 save".into());
    }
    let inner = unpack(d).ok_or("the save could not be unpacked")?;
    if !inner.starts_with(b"SAVY") {
        return Err("the unpacked save is not a SAVY file".into());
    }
    let xp = player_value(&inner, "experience", "Int").ok_or("Geralt's experience was not found in this save")?;
    let mut b = Builder::new(&inner);
    b.inside(d, pack);
    b.ranged(&["Geralt", "Experience"], xp, Kind::I32, 0, 99_999_999);
    if let Some(t) = player_value(&inner, "talents", "Int") {
        b.ranged(&["Geralt", "Talent points"], t, Kind::I32, 0, 999);
    }
    // Geralt's own inventory comes right after his properties; a copy of it kept elsewhere in the
    // save (with nearly the same amount) is changed together with it.
    let all = orens(&inner);
    if let Some(i) = all.iter().position(|&o| o > xp && o - xp < 0x10_0000) {
        let read = |o: usize| u32le(&inner, o).unwrap_or(0) as u64;
        let own = read(all[i]);
        if b.ranged(&["Geralt", "Orens"], all[i], Kind::U32, 0, 99_999_999).is_some() {
            if let Some(&copy) = all.get(i + 1) {
                let other = read(copy);
                if own.abs_diff(other) * 20 <= own.max(1) {
                    b.mirror(&["Geralt", "Orens"], copy);
                }
            }
        }
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn item(n: &str, qty: u32) -> Vec<u8> {
        let mut v = b"AVAL".to_vec();
        v.extend(name("name"));
        v.extend(name("String"));
        v.extend((n.len() as u32 + 1).to_le_bytes());
        v.extend(name(n));
        for (field, ty, data) in [("staticRandomSeed", "Uint16", vec![1u8, 2]), ("flags", "Uint16", vec![7, 0]), ("quantity", "Uint", qty.to_le_bytes().to_vec())] {
            v.extend(b"AVAL");
            v.extend(name(field));
            v.extend(name(ty));
            v.extend((data.len() as u32).to_le_bytes());
            v.extend(data);
        }
        v
    }

    /// A made-up SAVY file: filler, Geralt's properties, his inventory and a copy of it.
    fn savy() -> Vec<u8> {
        let mut v = b"SAVY\x02\0\0\0\x01\0\0\0".to_vec();
        v.extend((0..150_000u32).map(|i| (i % 7) as u8));
        v.extend(property("experience", "Int"));
        v.extend(31_250i32.to_le_bytes());
        v.extend(property("talents", "Int"));
        v.extend(2i32.to_le_bytes());
        v.extend(item("Orens", 12_345));
        v.extend((0..90_000u32).map(|i| (i * 31 % 251) as u8));
        v.extend(item("Orens", 12_340));
        v
    }

    fn dzip(inner: &[u8]) -> Vec<u8> {
        let mut head = b"DZIP\x02\0\0\0\x01\0\0\0\x01\0\0\0".to_vec();
        head.extend(0u64.to_le_bytes());
        head.extend(0x1234_5678_9abc_def0u64.to_le_bytes());
        let name = b"QuickSave.sav.tmp\0";
        // an empty container with the index right after the header, then packed through pack()
        let mut d = head;
        d.extend((name.len() as u16).to_le_bytes());
        d.extend(name);
        d.extend([9u8; 8]);
        d.extend(1u64.to_le_bytes());
        d.extend(0x20u64.to_le_bytes());
        d.extend(0u64.to_le_bytes());
        d[0x10..0x18].copy_from_slice(&0x20u64.to_le_bytes());
        pack(&d, inner)
    }

    #[test]
    fn lzf_round_trips() {
        for data in [vec![], vec![5u8], b"abcabcabcabcabcabc".to_vec(), (0..70_000u32).map(|i| (i * i % 253) as u8).collect(), vec![0u8; 70_000]] {
            let z = lzf(&data);
            assert_eq!(unlzf(&z, data.len()).unwrap(), data);
        }
    }

    #[test]
    fn reads_edits_and_repacks() {
        let inner = savy();
        let d = dzip(&inner);
        assert!(looks(&d));
        assert_eq!(unpack(&d).unwrap(), inner);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Geralt/Experience"), Some(&Value::from(31_250)));
        assert_eq!(doc.view.pointer("/Geralt/Talent points"), Some(&Value::from(2)));
        assert_eq!(doc.view.pointer("/Geralt/Orens"), Some(&Value::from(12_345)));
        assert_eq!(unpack(&doc.to_bytes().unwrap()).unwrap(), inner, "an untouched save unpacks the same");
        doc.set("/Geralt/Orens", Value::from(5_000)).unwrap();
        doc.set("/Geralt/Talent points", Value::from(7)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[0x18..0x20], &d[0x18..0x20], "the header value is kept");
        let again = unpack(&out).unwrap();
        let q = orens(&again);
        assert_eq!(q.len(), 2);
        assert_eq!(u32le(&again, q[0]), Some(5_000));
        assert_eq!(u32le(&again, q[1]), Some(5_000), "the copy changes together");
        assert_eq!(parse(&out).unwrap().view.pointer("/Geralt/Talent points"), Some(&Value::from(7)));
    }

    #[test]
    fn other_files_are_refused() {
        assert!(!looks(b"DZIP"));
        let mut bad = b"DZIP\x02\0\0\0\x01\0\0\0\x01\0\0\0".to_vec();
        bad.extend([0xFF; 16]);
        assert!(!looks(&bad), "an index offset past the end is refused");
        assert!(parse(&[0u8; 64]).is_err());
    }

    /// Run with `W2_SAVES=<folder of .sav>`: every save must read and repack to the same data.
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("W2_SAVES") else { return };
        let mut seen = 0;
        for f in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(f.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let inner = unpack(&d).unwrap();
            let mut doc = parse(&d).unwrap();
            eprintln!("{:?}: {}", f.file_name(), doc.view);
            doc.set("/Geralt/Orens", Value::from(1234)).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = unpack(&out).unwrap();
            assert_eq!(again.len(), inner.len());
            assert_eq!(parse(&out).unwrap().view.pointer("/Geralt/Orens"), Some(&Value::from(1234)));
            seen += 1;
        }
        assert!(seen > 0);
    }
}
