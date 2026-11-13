//! Dishonored (2012) `.sav` files, worked out from sample saves.
//!
//! The file is a short plain header followed by sections. Each section is a u32 with the
//! total length of the chunks after it, then those chunks back to back. A chunk is the Unreal
//! Engine 3 compressed package format: magic 9E2A83C1, block size, packed size, unpacked
//! size, (packed, unpacked) per block, then the zlib blocks. The editor shows every chunk
//! unpacked one after another and packs them again (with the section lengths) on save.

use std::io::{Read, Write};

const MAGIC: [u8; 4] = [0xC1, 0x83, 0x2A, 0x9E];

/// What is needed to build the file again: the header, the (unpacked size, block size) of every
/// chunk grouped by section, and whatever follows the last section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    head: Vec<u8>,
    sections: Vec<Vec<(usize, usize)>>,
    tail: Vec<u8>,
}

fn u32_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

/// The chunk at `i` (its magic): its unpacked bytes, block size and where it ends.
fn chunk(d: &[u8], i: usize) -> Option<(Vec<u8>, usize, usize)> {
    if d.get(i..i + 4)? != MAGIC {
        return None;
    }
    let (bs, packed, raw) = (u32_at(d, i + 4)?, u32_at(d, i + 8)?, u32_at(d, i + 12)?);
    if bs == 0 || raw > 64 << 20 {
        return None;
    }
    let n = raw.div_ceil(bs).max(1);
    let mut q = i + 16 + 8 * n;
    let end = q.checked_add(packed).filter(|&e| e <= d.len())?;
    let mut out = Vec::with_capacity(raw);
    for k in 0..n {
        let (c, r) = (u32_at(d, i + 16 + 8 * k)?, u32_at(d, i + 20 + 8 * k)?);
        let before = out.len();
        flate2::read::ZlibDecoder::new(d.get(q..q + c)?).read_to_end(&mut out).ok()?;
        if out.len() - before != r {
            return None;
        }
        q += c;
    }
    (q == end && out.len() == raw).then_some((out, bs, end))
}

pub fn unpack(d: &[u8]) -> Option<(Layout, Vec<u8>)> {
    let first = d.get(..256.min(d.len()))?.windows(4).position(|w| w == MAGIC)?;
    let mut p = first.checked_sub(4)?;
    let head = d[..p].to_vec();
    let mut view = Vec::with_capacity(d.len() * 4);
    let mut sections = Vec::new();
    while d.get(p + 4..p + 8) == Some(&MAGIC[..]) {
        let len = u32_at(d, p)?;
        let stop = (p + 4).checked_add(len)?;
        let mut at = p + 4;
        let mut list = Vec::new();
        while at < stop {
            let (raw, bs, end) = chunk(d, at)?;
            list.push((raw.len(), bs));
            view.extend(raw);
            at = end;
        }
        if at != stop {
            return None;
        }
        sections.push(list);
        p = stop;
    }
    if sections.is_empty() {
        return None;
    }
    Some((Layout { head, sections, tail: d[p..].to_vec() }, view))
}

fn pack_chunk(data: &[u8], bs: usize, out: &mut Vec<u8>) {
    let mut blocks: Vec<Vec<u8>> = data
        .chunks(bs)
        .map(|b| {
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            z.write_all(b).unwrap();
            z.finish().unwrap()
        })
        .collect();
    if blocks.is_empty() {
        blocks.push(flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default()).finish().unwrap());
    }
    let packed: usize = blocks.iter().map(Vec::len).sum();
    out.extend(MAGIC);
    for v in [bs, packed, data.len()] {
        out.extend((v as u32).to_le_bytes());
    }
    for (k, b) in blocks.iter().enumerate() {
        out.extend((b.len() as u32).to_le_bytes());
        out.extend((data.len().saturating_sub(k * bs).min(bs) as u32).to_le_bytes());
    }
    blocks.iter().for_each(|b| out.extend(b));
}

pub fn pack(l: &Layout, view: &[u8]) -> Result<Vec<u8>, String> {
    let total: usize = l.sections.iter().flatten().map(|c| c.0).sum();
    if total != view.len() {
        return Err("this save's data can't change size".into());
    }
    let mut out = l.head.clone();
    let mut p = 0;
    for list in &l.sections {
        let at = out.len();
        out.extend([0u8; 4]);
        for &(raw, bs) in list {
            pack_chunk(&view[p..p + raw], bs, &mut out);
            p += raw;
        }
        let len = (out.len() - at - 4) as u32;
        out[at..at + 4].copy_from_slice(&len.to_le_bytes());
    }
    out.extend(&l.tail);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Layout, Vec<u8>) {
        let l = Layout { head: vec![9; 37], sections: vec![vec![(300, 128)], vec![(70_000, 65_536), (10, 65_536)]], tail: vec![1, 2, 3] };
        let view: Vec<u8> = (0..70_310u32).map(|i| (i % 91) as u8).collect();
        (l, view)
    }

    #[test]
    fn round_trip() {
        let (l, view) = sample();
        let file = pack(&l, &view).unwrap();
        let (l2, back) = unpack(&file).unwrap();
        assert_eq!(back, view);
        assert_eq!(l2, l);
        let mut edited = view.clone();
        edited[1234] ^= 0xFF;
        let (_, again) = unpack(&pack(&l, &edited).unwrap()).unwrap();
        assert_eq!(again, edited);
    }

    #[test]
    fn size_must_stay() {
        let (l, view) = sample();
        assert!(pack(&l, &view[1..]).is_err());
    }

    /// Research aid: SAVE_EDITOR_DIS1 = a Dishonored `.sav`.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_DIS1").unwrap()).unwrap();
        let (l, v) = unpack(&d).expect("unpack");
        println!("{} sections, {} bytes unpacked", l.sections.len(), v.len());
        assert_eq!(unpack(&pack(&l, &v).unwrap()).unwrap().1, v);
    }
}
