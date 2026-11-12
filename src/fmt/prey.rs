//! Prey (2017) `save.CSF` and `*.level` files: "CRY3SDK", then RC4 (a key of 256 zero bytes,
//! the first 1024 keystream bytes dropped) over zlib chunks ([u32 packed][u32 unpacked][zlib])
//! and a 64-byte XMLCPB header ("0XBP") whose bytes 4..20 are an MD5 of the chunks and the
//! header with those bytes zeroed. The editor shows the unpacked CryEngine binary XML and
//! packs it again (32 KiB chunks) with a new MD5. Layout from Shere-Khan510's
//! prey-save-editor (MIT).

use md5::{Digest, Md5};
use std::io::{Read, Write};

const MAGIC: &[u8] = b"CRY3SDK";
const XMLCPB: &[u8] = b"0XBP";
const CHUNK: usize = 0x8000;

fn rc4(data: &mut [u8]) {
    let mut s: [u8; 256] = std::array::from_fn(|i| i as u8);
    let mut j = 0u8;
    for i in 0..256 {
        j = j.wrapping_add(s[i]);
        s.swap(i, j as usize);
    }
    let (mut i, mut j) = (0u8, 0u8);
    let mut next = || {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        s[(s[i as usize].wrapping_add(s[j as usize])) as usize]
    };
    for _ in 0..1024 {
        next();
    }
    for b in data {
        *b ^= next();
    }
}

fn digest(body: &[u8], header: &[u8]) -> [u8; 16] {
    let mut h = Md5::new();
    h.update(body);
    h.update(&header[..4]);
    h.update([0u8; 16]);
    h.update(&header[20..]);
    h.finalize().into()
}

/// (unpacked data, the 64-byte header), when the signature, MD5 and every chunk check out.
pub fn open(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if !d.starts_with(MAGIC) || d.len() < MAGIC.len() + 64 {
        return None;
    }
    let mut p = d[MAGIC.len()..].to_vec();
    rc4(&mut p);
    let (body, header) = p.split_at(p.len() - 64);
    if &header[..4] != XMLCPB || digest(body, header)[..] != header[4..20] {
        return None;
    }
    let mut out = Vec::new();
    let mut at = 0;
    while at < body.len() {
        let c = u32::from_le_bytes(body.get(at..at + 4)?.try_into().ok()?) as usize;
        let u = u32::from_le_bytes(body.get(at + 4..at + 8)?.try_into().ok()?) as usize;
        let before = out.len();
        flate2::read::ZlibDecoder::new(body.get(at + 8..at + 8 + c)?).read_to_end(&mut out).ok()?;
        if out.len() - before != u {
            return None;
        }
        at += 8 + c;
    }
    Some((out, header.to_vec()))
}

pub fn close(header: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let mut body = Vec::with_capacity(data.len() / 2);
    for blk in data.chunks(CHUNK) {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(5));
        z.write_all(blk).map_err(|e| e.to_string())?;
        let c = z.finish().map_err(|e| e.to_string())?;
        body.extend_from_slice(&(c.len() as u32).to_le_bytes());
        body.extend_from_slice(&(blk.len() as u32).to_le_bytes());
        body.extend_from_slice(&c);
    }
    let mut h = header.to_vec();
    let sum = digest(&body, &h);
    h[4..20].copy_from_slice(&sum);
    body.extend_from_slice(&h);
    rc4(&mut body);
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_drops_the_first_1024_bytes() {
        // RC4 with a 256-byte zero key; keystream bytes 1024.. (checked against a plain loop).
        let mut s: Vec<u8> = (0..=255).collect();
        let mut j = 0usize;
        for i in 0..256 {
            j = (j + s[i] as usize) & 255;
            s.swap(i, j);
        }
        let (mut i, mut j) = (0usize, 0usize);
        let mut ks = Vec::new();
        for _ in 0..1030 {
            i = (i + 1) & 255;
            j = (j + s[i] as usize) & 255;
            s.swap(i, j);
            ks.push(s[(s[i] as usize + s[j] as usize) & 255]);
        }
        let mut z = [0u8; 6];
        rc4(&mut z);
        assert_eq!(&z[..], &ks[1024..]);
    }

    #[test]
    fn container_round_trip() {
        let data: Vec<u8> = (0..100_000u32).map(|i| (i % 97) as u8).collect();
        let mut header = vec![0u8; 64];
        header[..4].copy_from_slice(XMLCPB);
        header[20..24].copy_from_slice(&7u32.to_le_bytes());
        let raw = close(&header, &data).unwrap();
        let (back, h) = open(&raw).unwrap();
        assert_eq!(back, data);
        assert_eq!(&h[20..24], &7u32.to_le_bytes());
        let mut bad = raw.clone();
        bad[40] ^= 1;
        assert!(open(&bad).is_none());
    }
}
