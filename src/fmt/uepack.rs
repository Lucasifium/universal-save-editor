//! Unreal-style compressed save containers, unpacked so the data can be searched and edited in
//! place, then packed again on save.
//!
//! - Astroneer (`*.savegame`): the bytes `BE 40 37 4A EE 0B 74 A3`, a version (1), the unpacked
//!   size, then zlib streams that unpack to a `GVAS` file. The game's decompressor only keeps a
//!   4 KiB window, so the zlib header says so (`48 89`) and the data is written as stored
//!   blocks, which any window size can read. Layout as in astroneer-save (NotFabii, MIT).
//! - Satisfactory and Remnant 2 (`*.sav`): a header, then chunks of up to 128 KiB, each with its own header
//!   (`C1 83 2A 9E`, a version, the chunk size, the zlib code and the sizes). The header is kept
//!   as it is (Remnant 2's CRC-32 is made again). Chunk layout as in satisfactory-file-parser
//!   (etothepii4, MIT) and lib.remnant2.saves (AndrewSav, MIT).

use std::io::{Read, Write};

const ASTRO_MAGIC: [u8; 8] = [0xBE, 0x40, 0x37, 0x4A, 0xEE, 0x0B, 0x74, 0xA3];
const SAT_TAG: [u8; 4] = [0xC1, 0x83, 0x2A, 0x9E];

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn adler32(d: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in d.chunks(5552) {
        for x in chunk {
            a += *x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    b << 16 | a
}

// ------------------------------------------------------------------ Astroneer

pub fn astro_open(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 20 || d[..8] != ASTRO_MAGIC || u32le(d, 8)? != 1 {
        return None;
    }
    let want = u32le(d, 12)? as usize;
    let mut out = Vec::with_capacity(want);
    let mut pos = 16;
    while pos < d.len() && out.len() < want {
        let mut z = flate2::bufread::ZlibDecoder::new(&d[pos..]);
        let before = out.len();
        z.read_to_end(&mut out).ok()?;
        let used = d.len() - pos - z.into_inner().len();
        if used == 0 || out.len() == before {
            return None;
        }
        pos += used;
    }
    (out.len() == want && out.starts_with(b"GVAS")).then_some(out)
}

pub fn astro_pack(payload: &[u8]) -> Vec<u8> {
    let mut out = ASTRO_MAGIC.to_vec();
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0x48, 0x89]);
    let mut blocks = payload.chunks(65535).peekable();
    if blocks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(b) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        out.extend_from_slice(&(b.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
        out.extend_from_slice(b);
    }
    out.extend_from_slice(&adler32(payload).to_be_bytes());
    out
}

// ------------------------------------------------------------------ Satisfactory

/// What a Satisfactory save keeps besides its data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SatInfo {
    /// Everything before the first chunk.
    pub header: Vec<u8>,
    /// Remnant 2: the 12-byte header starts with a CRC-32 of the rest of the unpacked file.
    pub r2: bool,
    /// 0 (48-byte chunk headers) or 0x22222222 (49 bytes, with the compression code).
    pub version: u32,
    pub max: u32,
}

pub fn sat_open(d: &[u8]) -> Option<(SatInfo, Vec<u8>)> {
    let first = d.windows(4).position(|w| w == SAT_TAG)?;
    if first < 12 || first > 4096 {
        return None;
    }
    let version = u32le(d, first + 4)?;
    if version != 0 && version != 0x2222_2222 {
        return None;
    }
    let max = u32le(d, first + 8)?;
    if max == 0 || max > 1 << 24 {
        return None;
    }
    let hs = if version == 0 { 48 } else { 49 };
    let (cl_at, ul_at) = if version == 0 { (32, 24) } else { (33, 25) };
    let mut pos = first;
    let mut body = Vec::new();
    while pos < d.len() {
        if d.get(pos..pos + 4)? != SAT_TAG {
            return None;
        }
        let clen = u32le(d, pos + cl_at)? as usize;
        let ulen = u32le(d, pos + ul_at)? as usize;
        let z = d.get(pos + hs..pos + hs + clen)?;
        let before = body.len();
        flate2::read::ZlibDecoder::new(z).read_to_end(&mut body).ok()?;
        if body.len() - before != ulen {
            return None;
        }
        pos += hs + clen;
    }
    // Remnant 2: CRC-32, unpacked size and save version in front of the chunks.
    let r2 = first == 12 && version == 0x2222_2222 && u32le(d, 4).is_some_and(|n| n as usize == body.len() + 8);
    Some((SatInfo { header: d[..first].to_vec(), r2, version, max }, body))
}

/// CRC-32 (IEEE) of Remnant 2's unpacked file after its first four bytes.
fn r2_crc(header: &[u8], body: &[u8]) -> u32 {
    let mut c = flate2::Crc::new();
    c.update(&header[4..12]);
    c.update(body.get(4..).unwrap_or(&[]));
    c.sum()
}

pub fn sat_pack(info: &SatInfo, body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = info.header.clone();
    if info.r2 {
        let crc = r2_crc(&out, body);
        out[..4].copy_from_slice(&crc.to_le_bytes());
    }
    let hs = if info.version == 0 { 48 } else { 49 };
    for part in body.chunks(info.max as usize) {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(part).map_err(|e| e.to_string())?;
        let comp = z.finish().map_err(|e| e.to_string())?;
        let mut h = vec![0u8; hs];
        h[0..4].copy_from_slice(&SAT_TAG);
        h[4..8].copy_from_slice(&info.version.to_le_bytes());
        h[8..12].copy_from_slice(&info.max.to_le_bytes());
        let (c1, u1, c2, u2) = if info.version == 0 { (16, 24, 32, 40) } else { (17, 25, 33, 41) };
        if info.version != 0 {
            h[16] = 3;
        }
        for (at, v) in [(c1, comp.len()), (u1, part.len()), (c2, comp.len()), (u2, part.len())] {
            h[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
        }
        out.extend_from_slice(&h);
        out.extend_from_slice(&comp);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn astroneer_round_trip() {
        let mut payload = b"GVAS".to_vec();
        payload.extend((0..200_000u32).map(|i| (i % 251) as u8));
        let f = astro_pack(&payload);
        assert_eq!(astro_open(&f).unwrap(), payload);
        assert_eq!(&f[16..18], &[0x48, 0x89]);
    }

    #[test]
    fn satisfactory_round_trip() {
        let body: Vec<u8> = (0..300_000u32).map(|i| (i % 253) as u8).collect();
        let info = SatInfo { header: vec![7; 40], r2: false, version: 0x2222_2222, max: 131072 };
        let f = sat_pack(&info, &body).unwrap();
        let (i2, b2) = sat_open(&f).unwrap();
        assert_eq!(i2, info);
        assert_eq!(b2, body);
        let info0 = SatInfo { version: 0, ..info };
        let (_, b0) = sat_open(&sat_pack(&info0, &body).unwrap()).unwrap();
        assert_eq!(b0, body);
    }

    #[test]
    fn remnant2_header_and_crc() {
        let mut body: Vec<u8> = (0..70_000u32).map(|i| (i % 249) as u8).collect();
        let n = body.len() as u32 + 4;
        body[..4].copy_from_slice(&n.to_le_bytes());
        let mut header = vec![0u8; 12];
        header[4..8].copy_from_slice(&(body.len() as u32 + 8).to_le_bytes());
        header[8..12].copy_from_slice(&9u32.to_le_bytes());
        let info = SatInfo { header, r2: true, version: 0x2222_2222, max: 0x20000 };
        let f = sat_pack(&info, &body).unwrap();
        let (i2, b2) = sat_open(&f).unwrap();
        assert!(i2.r2);
        assert_eq!(b2, body);
        let mut c = flate2::Crc::new();
        c.update(&f[4..12]);
        c.update(&body[4..]);
        assert_eq!(&f[..4], &c.sum().to_le_bytes());
    }

    #[test]
    fn real_saves() {
        let Ok(dir) = std::env::var("SAT_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let d = std::fs::read(&p).unwrap();
            let (info, body) = sat_open(&d).unwrap_or_else(|| panic!("{p:?}"));
            let again = sat_pack(&info, &body).unwrap();
            let (_, b2) = sat_open(&again).unwrap();
            assert_eq!(b2, body, "{p:?}");
            println!("{p:?}: header {} bytes, body {} bytes", info.header.len(), body.len());
        }
    }
}
