//! BioShock Infinite `.bkm` saves, worked out from sample saves: an MD5 of the rest of the
//! file, then data with Unreal Engine 3 compressed chunks inside it, each after its total
//! length (u32): magic 9E2A83C1, block size, packed size, unpacked size, (packed, unpacked)
//! per block, then the zlib blocks. The editor shows the file with every chunk unpacked in
//! place, and packs them again when saving.

use md5::Digest;
use std::io::{Read, Write};

const MAGIC: [u8; 4] = [0xC1, 0x83, 0x2A, 0x9E];

fn u32_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

/// Where each chunk sits in the unpacked view: (start, unpacked size, block size).
pub type Chunks = Vec<(usize, usize, usize)>;

/// One chunk at `i` (its magic), with the length field before it: (unpacked, block size, end).
fn chunk(d: &[u8], i: usize) -> Option<(Vec<u8>, usize, usize)> {
    let len = u32_at(d, i.checked_sub(4)?)?;
    let (bs, packed, raw) = (u32_at(d, i + 4)?, u32_at(d, i + 8)?, u32_at(d, i + 12)?);
    if bs == 0 || raw > 64 << 20 {
        return None;
    }
    let n = raw.div_ceil(bs).max(1);
    if len != 16 + 8 * n + packed || i + len > d.len() {
        return None;
    }
    let mut q = i + 16 + 8 * n;
    let mut out = Vec::with_capacity(raw);
    for k in 0..n {
        let (c, r) = (u32_at(d, i + 16 + 8 * k)?, u32_at(d, i + 20 + 8 * k)?);
        let before = out.len();
        flate2::read::ZlibDecoder::new(d.get(q..q + c)?)
            .read_to_end(&mut out)
            .ok()?;
        if out.len() - before != r {
            return None;
        }
        q += c;
    }
    (out.len() == raw).then_some((out, bs, i + len))
}

pub fn unpack(d: &[u8]) -> Option<(Chunks, Vec<u8>)> {
    if d.len() < 64 || md5::Md5::digest(&d[16..])[..] != d[..16] {
        return None;
    }
    let mut view = Vec::with_capacity(d.len() * 4);
    let mut chunks = Vec::new();
    let mut p = 16;
    let mut q = 16;
    while let Some(i) = d[q..].windows(4).position(|w| w == MAGIC).map(|i| i + q) {
        q = i + 4;
        let Some((raw, bs, end)) = chunk(d, i) else {
            continue;
        };
        view.extend(&d[p..i - 4]);
        chunks.push((view.len(), raw.len(), bs));
        view.extend(raw);
        p = end;
        q = end;
    }
    view.extend(&d[p..]);
    Some((chunks, view))
}

pub fn pack(chunks: &Chunks, view: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; 16];
    let mut p = 0;
    for &(at, raw, bs) in chunks {
        if at + raw > view.len() || at < p {
            return Err("this save's data can't change size".into());
        }
        out.extend(&view[p..at]);
        let data = &view[at..at + raw];
        let blocks: Vec<Vec<u8>> = data
            .chunks(bs)
            .map(|b| {
                let mut z =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                z.write_all(b).unwrap();
                z.finish().unwrap()
            })
            .collect();
        let blocks = if blocks.is_empty() {
            vec![
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default())
                    .finish()
                    .unwrap(),
            ]
        } else {
            blocks
        };
        let packed: usize = blocks.iter().map(Vec::len).sum();
        out.extend(((16 + 8 * blocks.len() + packed) as u32).to_le_bytes());
        out.extend(MAGIC);
        for v in [bs, packed, raw] {
            out.extend((v as u32).to_le_bytes());
        }
        for (k, b) in blocks.iter().enumerate() {
            out.extend((b.len() as u32).to_le_bytes());
            out.extend((data.len().saturating_sub(k * bs).min(bs) as u32).to_le_bytes());
        }
        blocks.iter().for_each(|b| out.extend(b));
        p = at + raw;
    }
    out.extend(&view[p..]);
    let sum = md5::Md5::digest(&out[16..]);
    out[..16].copy_from_slice(&sum);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let inner: Vec<u8> = (0..300_000u32).map(|i| (i % 97) as u8).collect();
        let chunks = vec![(10, 300_000, 0x20000)];
        let mut view = b"0123456789".to_vec();
        view.extend(&inner);
        view.extend(b"tail");
        let file = pack(&chunks, &view).unwrap();
        let (c2, back) = unpack(&file).unwrap();
        assert_eq!(back, view);
        assert_eq!(c2, chunks);
    }

    /// Research aid: SAVE_EDITOR_BIO = a .bkm save.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_BIO").unwrap()).unwrap();
        let (c, v) = unpack(&d).expect("unpack");
        println!("{} chunks, {} bytes unpacked", c.len(), v.len());
        let again = pack(&c, &v).unwrap();
        assert_eq!(unpack(&again).unwrap().1, v);
    }
}
