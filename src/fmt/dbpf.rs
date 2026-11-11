//! Maxis DBPF 1.x packages with a 7.2 index (The Sims 2 neighbourhoods and lots).
//!
//! A 96-byte header, the resources, then the index: 24-byte entries (type, group, instance,
//! second instance, offset, size). The `DIR` resource (type E86B1EEF) lists, as 20-byte
//! entries, which resources are compressed; those are a 4-byte packed size followed by
//! QFS/RefPack (`10 FB`, a 3-byte unpacked size, then commands).
//!
//! Only the resources a game screen wants are unpacked into a flat form (`S2FL`, a count, then
//! type, group, instance, second instance, length, body for each). On save the changed ones
//! are stored uncompressed and dropped from `DIR` (the way SimPE writes them), and the index
//! is written again after the data.

pub const DIR_TYPE: u32 = 0xE86B_1EEF;
const HEAD: usize = 96;
const FLAT: &[u8; 4] = b"S2FL";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
    pub key: [u32; 4],
    pub offset: u32,
    pub size: u32,
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

pub fn index(d: &[u8]) -> Option<Vec<Entry>> {
    if d.len() < HEAD || &d[..4] != b"DBPF" || u32_at(d, 4)? != 1 || u32_at(d, 32)? != 7 || u32_at(d, 60)? != 2 {
        return None;
    }
    let (n, off, size) = (u32_at(d, 36)? as usize, u32_at(d, 40)? as usize, u32_at(d, 44)? as usize);
    if size != n.checked_mul(24)? || off.checked_add(size)? > d.len() {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let b = off + 24 * i;
        let e = Entry { key: [u32_at(d, b)?, u32_at(d, b + 4)?, u32_at(d, b + 8)?, u32_at(d, b + 12)?], offset: u32_at(d, b + 16)?, size: u32_at(d, b + 20)? };
        if (e.offset as usize).checked_add(e.size as usize)? > off.max(d.len()) {
            return None;
        }
        out.push(e);
    }
    Some(out)
}

pub fn qfs(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 5 || data[1] != 0xFB {
        return None;
    }
    let big = data[0] & 0x80 != 0;
    let mut p = if big { 6 } else { 5 };
    let size = if big { u32::from_be_bytes(data.get(2..6)?.try_into().ok()?) as usize } else { (data[2] as usize) << 16 | (data[3] as usize) << 8 | data[4] as usize };
    if size > 1 << 28 {
        return None;
    }
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let lit = |out: &mut Vec<u8>, p: &mut usize, n: usize| -> Option<()> {
        out.extend_from_slice(data.get(*p..*p + n)?);
        *p += n;
        Some(())
    };
    while out.len() < size {
        let b0 = *data.get(p)? as usize;
        p += 1;
        let (nlit, off, n);
        if b0 < 0x80 {
            let b1 = *data.get(p)? as usize;
            p += 1;
            nlit = b0 & 3;
            off = ((b0 & 0x60) << 3) + b1 + 1;
            n = ((b0 & 0x1C) >> 2) + 3;
        } else if b0 < 0xC0 {
            let (b1, b2) = (*data.get(p)? as usize, *data.get(p + 1)? as usize);
            p += 2;
            nlit = b1 >> 6;
            off = ((b1 & 0x3F) << 8) + b2 + 1;
            n = (b0 & 0x3F) + 4;
        } else if b0 < 0xE0 {
            let (b1, b2, b3) = (*data.get(p)? as usize, *data.get(p + 1)? as usize, *data.get(p + 2)? as usize);
            p += 3;
            nlit = b0 & 3;
            off = ((b0 & 0x10) << 12) + (b1 << 8) + b2 + 1;
            n = ((b0 & 0x0C) << 6) + b3 + 5;
        } else if b0 < 0xFC {
            lit(&mut out, &mut p, ((b0 & 0x1F) << 2) + 4)?;
            continue;
        } else {
            lit(&mut out, &mut p, b0 & 3)?;
            break;
        }
        lit(&mut out, &mut p, nlit)?;
        if off > out.len() {
            return None;
        }
        for _ in 0..n {
            out.push(out[out.len() - off]);
        }
    }
    (out.len() >= size).then(|| {
        out.truncate(size);
        out
    })
}

/// The 20-byte `DIR` entries: the keys of the compressed resources.
fn dir_keys(d: &[u8], idx: &[Entry]) -> Vec<[u32; 4]> {
    let Some(e) = idx.iter().find(|e| e.key[0] == DIR_TYPE) else { return vec![] };
    let b = &d[e.offset as usize..(e.offset + e.size) as usize];
    b.chunks_exact(20).map(|c| [0, 4, 8, 12].map(|o| u32::from_le_bytes(c[o..o + 4].try_into().unwrap()))).collect()
}

pub fn resource(d: &[u8], e: &Entry, compressed: bool) -> Option<Vec<u8>> {
    let raw = d.get(e.offset as usize..(e.offset + e.size) as usize)?;
    if compressed {
        qfs(raw.get(4..)?)
    } else {
        Some(raw.to_vec())
    }
}

/// Unpacks the resources `want` accepts into the flat form.
pub fn flatten(d: &[u8], want: impl Fn(&[u32; 4]) -> Option<usize>) -> Option<Vec<u8>> {
    let idx = index(d)?;
    let comp: std::collections::HashSet<[u32; 4]> = dir_keys(d, &idx).into_iter().collect();
    let mut out = FLAT.to_vec();
    let mut n = 0u32;
    out.extend(0u32.to_le_bytes());
    for e in &idx {
        let Some(cap) = want(&e.key) else { continue };
        let Some(mut body) = resource(d, e, comp.contains(&e.key)) else { return None };
        body.truncate(cap);
        for v in e.key {
            out.extend(v.to_le_bytes());
        }
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        n += 1;
    }
    out[4..8].copy_from_slice(&n.to_le_bytes());
    Some(out)
}

pub struct Flat<'a> {
    pub key: [u32; 4],
    pub body: &'a [u8],
    pub at: usize,
}

pub fn flat_entries(f: &[u8]) -> Option<Vec<Flat<'_>>> {
    if f.get(..4)? != FLAT {
        return None;
    }
    let n = u32_at(f, 4)? as usize;
    let mut at = 8;
    let mut out = vec![];
    for _ in 0..n {
        let key = [u32_at(f, at)?, u32_at(f, at + 4)?, u32_at(f, at + 8)?, u32_at(f, at + 12)?];
        let len = u32_at(f, at + 16)? as usize;
        let body = f.get(at + 20..at + 20 + len)?;
        out.push(Flat { key, body, at: at + 20 });
        at += 20 + len;
    }
    (at == f.len()).then_some(out)
}

/// Writes the package again: `flat` (same resources as `flatten` gave, same lengths) replaces
/// the originals that differ.
pub fn rebuild(original: &[u8], flat: &[u8], editable: impl Fn(&[u32; 4]) -> bool) -> Result<Vec<u8>, String> {
    let idx = index(original).ok_or("the package changed")?;
    let old = flat_entries(flat).ok_or("the package data is damaged")?;
    let comp: std::collections::HashSet<[u32; 4]> = dir_keys(original, &idx).into_iter().collect();
    let mut changed: std::collections::HashMap<[u32; 4], &[u8]> = Default::default();
    for f in &old {
        if !editable(&f.key) {
            continue;
        }
        let e = idx.iter().find(|e| e.key == f.key).ok_or("a resource is missing")?;
        let cur = resource(original, e, comp.contains(&f.key)).ok_or("a resource can't be read")?;
        if cur.len() != f.body.len() {
            return Err("this save's resources can't change size".into());
        }
        if cur != f.body {
            changed.insert(f.key, f.body);
        }
    }
    if changed.is_empty() {
        return Ok(original.to_vec());
    }
    let mut out = original[..HEAD].to_vec();
    let mut entries: Vec<([u32; 4], u32, u32)> = vec![];
    let dir_new: Vec<u8> = {
        let Some(e) = idx.iter().find(|e| e.key[0] == DIR_TYPE) else { return Err("this package has no directory resource".into()) };
        original[e.offset as usize..(e.offset + e.size) as usize]
            .chunks_exact(20)
            .filter(|c| !changed.contains_key(&[0, 4, 8, 12].map(|o| u32::from_le_bytes(c[o..o + 4].try_into().unwrap()))))
            .flatten()
            .copied()
            .collect()
    };
    for e in &idx {
        let bytes: &[u8] = if e.key[0] == DIR_TYPE {
            &dir_new
        } else if let Some(b) = changed.get(&e.key) {
            b
        } else {
            &original[e.offset as usize..(e.offset + e.size) as usize]
        };
        entries.push((e.key, out.len() as u32, bytes.len() as u32));
        out.extend_from_slice(bytes);
    }
    let at = out.len() as u32;
    for (k, off, size) in &entries {
        for v in k {
            out.extend(v.to_le_bytes());
        }
        out.extend(off.to_le_bytes());
        out.extend(size.to_le_bytes());
    }
    out[40..44].copy_from_slice(&at.to_le_bytes());
    out[44..48].copy_from_slice(&((entries.len() * 24) as u32).to_le_bytes());
    out[48..60].fill(0);
    Ok(out)
}
