//! Kingdom Come: Deliverance saves (`*.whs`): the save data cut into pieces of at most 32 KB,
//! each stored as `[u32 packed size][u32 unpacked size][zlib stream]`, then a 64-byte trailer
//! (`0XBP`, 16 bytes of unknown meaning, zeros). The trailer is kept as it is.
//!
//! The unpacked data opens for the Hex view and Value search; no values are named yet.

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use std::sync::Arc;

const PIECE: usize = 0x8000;
const TRAILER: usize = 64;
/// Unpacked saves are tens of MB; anything far larger is refused.
const MAX_SIZE: usize = 512 << 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    trailer: Arc<Vec<u8>>,
    original: Arc<Vec<u8>>,
    file: Arc<Vec<u8>>,
}

fn u32_at(d: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?) as usize)
}

pub fn maybe(d: &[u8]) -> bool {
    d.len() > TRAILER + 10 && d[d.len() - TRAILER..].starts_with(b"0XBP") && u32_at(d, 4).is_some_and(|u| (1..=PIECE).contains(&u)) && d.get(8) == Some(&0x78)
}

pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !maybe(d) {
        return None;
    }
    let end = d.len() - TRAILER;
    let mut out = Vec::new();
    let mut p = 0;
    while p < end {
        let (c, u) = (u32_at(d, p)?, u32_at(d, p + 4)?);
        if u == 0 || u > PIECE {
            return None;
        }
        let z = d.get(p + 8..(p + 8).checked_add(c)?)?;
        let before = out.len();
        ZlibDecoder::new(z).take(u as u64 + 1).read_to_end(&mut out).ok()?;
        if out.len() - before != u || out.len() > MAX_SIZE {
            return None;
        }
        p += 8 + c;
    }
    (p == end).then(|| (Packed { trailer: Arc::new(d[end..].to_vec()), original: Arc::new(out.clone()), file: Arc::new(d.to_vec()) }, out))
}

pub fn close(p: &Packed, data: &[u8]) -> Result<Vec<u8>, String> {
    if data == p.original.as_slice() {
        return Ok((*p.file).clone());
    }
    let mut out = Vec::with_capacity(data.len() / 4);
    for c in data.chunks(PIECE) {
        let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
        z.write_all(c).map_err(|e| e.to_string())?;
        let z = z.finish().map_err(|e| e.to_string())?;
        out.extend((z.len() as u32).to_le_bytes());
        out.extend((c.len() as u32).to_le_bytes());
        out.extend(z);
    }
    out.extend(p.trailer.iter());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save(data: &[u8]) -> Vec<u8> {
        let mut trailer = b"0XBP".to_vec();
        trailer.extend([0xAB; 16]);
        trailer.resize(TRAILER, 0);
        let p = Packed { trailer: Arc::new(trailer), original: Arc::new(vec![]), file: Arc::new(vec![]) };
        close(&p, data).unwrap()
    }

    #[test]
    fn opens_and_packs_again() {
        let data: Vec<u8> = (0..100_000u32).map(|i| (i % 97) as u8).collect();
        let d = save(&data);
        let (p, inner) = open(&d).unwrap();
        assert_eq!(inner, data);
        assert_eq!(close(&p, &inner).unwrap(), d, "unchanged data gives back the file");
        let mut edited = inner.clone();
        edited[40_000] = 0xFF;
        let out = close(&p, &edited).unwrap();
        assert_eq!(open(&out).unwrap().1, edited);
        assert_eq!(&out[out.len() - TRAILER..], &d[d.len() - TRAILER..], "the trailer is kept");
    }

    #[test]
    fn other_files_are_refused() {
        assert!(open(&[0u8; 200]).is_none());
        let mut d = save(b"hello");
        d[0] ^= 1;
        assert!(open(&d).is_none());
    }
}
