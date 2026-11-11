//! Multi-entry game containers opened one entry at a time (like a zip):
//!
//! * FromSoftware `.sl2` / `.sl3` saves (Elden Ring, Dark Souls II/III, Sekiro):
//!   a BND4 archive of `USER_DATA###` entries, each `MD5(rest) || rest`. In Dark Souls
//!   II/III and Dark Souls Remastered `rest` is `IV || AES-128-CBC(data)` with a fixed
//!   per-game key.
//! * Minecraft region files (`.mca` / `.mcr`): 1024 chunk slots, each a compressed NBT.

use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use md5::{Digest, Md5};
use std::io::{Read, Write};

// ------------------------------------------------------------------ BND4

const DS3_KEY: [u8; 16] = [
    0xFD, 0x46, 0x4D, 0x69, 0x5E, 0x69, 0xA3, 0x9A, 0x10, 0xE3, 0x19, 0xA7, 0xAC, 0xE8, 0xB7, 0xFA,
];
const DS2_KEY: [u8; 16] = [
    0x59, 0x9F, 0x9B, 0x69, 0x96, 0x40, 0xA5, 0x52, 0x36, 0xEE, 0x2D, 0x70, 0x83, 0x5E, 0xC7, 0x44,
];
const DSR_KEY: [u8; 16] = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10,
];

#[derive(Clone, Debug)]
pub struct Bnd4Entry {
    pub name: String,
    pub offset: usize,
    pub size: usize,
}

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

pub fn bnd4_entries(d: &[u8]) -> Option<Vec<Bnd4Entry>> {
    if !d.starts_with(b"BND4") || d.len() < 0x40 {
        return None;
    }
    let count = le32(d, 0x0C)? as usize;
    let header_size = le32(d, 0x20)? as usize;
    // Saves use 0x20-byte entry headers with sizes, offsets and UTF-16 names.
    if header_size != 0x20 || d[0x30] != 1 || count == 0 || count > 1000 {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let h = 0x40 + i * header_size;
        let size = u64::from_le_bytes(d.get(h + 8..h + 16)?.try_into().ok()?) as usize;
        let offset = le32(d, h + 0x10)? as usize;
        let name_off = le32(d, h + 0x14)? as usize;
        if offset.checked_add(size)? > d.len() || size < 16 {
            return None;
        }
        let units: Vec<u16> = d
            .get(name_off..)?
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .take(64)
            .collect();
        out.push(Bnd4Entry {
            name: String::from_utf16(&units).ok()?,
            offset,
            size,
        });
    }
    Some(out)
}

fn zero_share(b: &[u8]) -> f64 {
    b.iter().filter(|&&x| x == 0).count() as f64 / b.len().max(1) as f64
}

/// Open an entry: verify its MD5, then decrypt it if this game encrypts saves.
/// Returns (data, key used, IV).
pub fn bnd4_open(d: &[u8], e: &Bnd4Entry) -> Option<(Vec<u8>, Option<[u8; 16]>, Vec<u8>)> {
    let raw = &d[e.offset..e.offset + e.size];
    let (sum, rest) = raw.split_at(16);
    if Md5::digest(rest).as_slice() != sum {
        return None;
    }
    // Unencrypted saves (Elden Ring, Sekiro) are full of zeros; ciphertext isn't.
    if zero_share(rest) > 0.05 || rest.len() < 32 || rest.len() % 16 != 0 {
        return Some((rest.to_vec(), None, vec![]));
    }
    let (iv, body) = rest.split_at(16);
    let mut best: Option<(f64, Vec<u8>, [u8; 16], Vec<u8>)> = None;
    for key in [DS3_KEY, DS2_KEY, DSR_KEY] {
        let mut buf = body.to_vec();
        cbc::Decryptor::<aes::Aes128>::new(&key.into(), iv.into())
            .decrypt_padded_mut::<NoPadding>(&mut buf)
            .ok()?;
        let z = zero_share(&buf);
        if z > 0.05 && best.as_ref().is_none_or(|b| z > b.0) {
            best = Some((z, buf, key, iv.to_vec()));
        }
    }
    match best {
        Some((_, plain, key, iv)) => Some((plain, Some(key), iv)),
        None => Some((rest.to_vec(), None, vec![])),
    }
}

/// Put edited entry data back: re-encrypt, new MD5, same place and size.
pub fn bnd4_replace(
    d: &[u8],
    name: &str,
    data: &[u8],
    key: Option<[u8; 16]>,
    iv: &[u8],
) -> Result<Vec<u8>, String> {
    let entries = bnd4_entries(d).ok_or("the save archive changed")?;
    let e = entries
        .iter()
        .find(|e| e.name == name)
        .ok_or("entry not found")?;
    let rest = match key {
        None => data.to_vec(),
        Some(key) => {
            let iv16: [u8; 16] = iv.try_into().map_err(|_| "bad IV")?;
            let mut buf = data.to_vec();
            let n = buf.len();
            cbc::Encryptor::<aes::Aes128>::new(&key.into(), &iv16.into())
                .encrypt_padded_mut::<NoPadding>(&mut buf, n)
                .map_err(|_| "data must stay a multiple of 16 bytes")?;
            [iv, &buf].concat()
        }
    };
    if rest.len() + 16 != e.size {
        return Err("the save slot can't change size".into());
    }
    let mut out = d.to_vec();
    out[e.offset..e.offset + 16].copy_from_slice(&Md5::digest(&rest));
    out[e.offset + 16..e.offset + e.size].copy_from_slice(&rest);
    Ok(out)
}

/// The entry to open by default: the first character slot that isn't empty.
pub fn bnd4_default(d: &[u8], entries: &[Bnd4Entry]) -> Option<usize> {
    // The first slot that really holds a character.
    if let Some(i) = entries.iter().position(|e| {
        bnd4_open(d, e).is_some_and(|(p, _, _)| super::fromsoft::detect(&p).is_some())
    }) {
        return Some(i);
    }
    entries
        .iter()
        .position(|e| {
            bnd4_open(d, e).is_some_and(|(p, _, _)| p.iter().any(|&b| b != 0)) && e.size > 0x10000
        })
        .or_else(|| entries.iter().position(|e| bnd4_open(d, e).is_some()))
}

// ------------------------------------------------------------------ Minecraft regions

pub fn region_chunks(d: &[u8]) -> Option<Vec<(usize, String)>> {
    if d.len() < 8192 || d.len() % 4096 != 0 {
        return None;
    }
    let mut out = Vec::new();
    for i in 0..1024 {
        let loc = u32::from_be_bytes(d[i * 4..i * 4 + 4].try_into().unwrap());
        let (off, count) = ((loc >> 8) as usize, (loc & 0xFF) as usize);
        if loc == 0 {
            continue;
        }
        if off < 2 || count == 0 || (off + count) * 4096 > d.len() {
            return None;
        }
        out.push((i, format!("chunk {},{}", i % 32, i / 32)));
    }
    (!out.is_empty()).then_some(out)
}

/// Decompressed NBT of one chunk, with its compression type.
pub fn region_open(d: &[u8], index: usize) -> Option<(Vec<u8>, u8)> {
    let loc = u32::from_be_bytes(d[index * 4..index * 4 + 4].try_into().unwrap());
    let off = (loc >> 8) as usize * 4096;
    let len = u32::from_be_bytes(d.get(off..off + 4)?.try_into().ok()?) as usize;
    let kind = *d.get(off + 4)?;
    let body = d.get(off + 5..off + 4 + len)?;
    let mut out = Vec::new();
    match kind {
        1 => flate2::read::GzDecoder::new(body)
            .read_to_end(&mut out)
            .ok()?,
        2 => flate2::read::ZlibDecoder::new(body)
            .read_to_end(&mut out)
            .ok()?,
        3 => {
            out = body.to_vec();
            0
        }
        _ => return None,
    };
    Some((out, kind))
}

pub fn region_replace(d: &[u8], index: usize, data: &[u8], kind: u8) -> Result<Vec<u8>, String> {
    let io = |e: std::io::Error| e.to_string();
    let body = match kind {
        1 => {
            let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(data).map_err(io)?;
            e.finish().map_err(io)?
        }
        2 => {
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(data).map_err(io)?;
            e.finish().map_err(io)?
        }
        _ => data.to_vec(),
    };
    let mut chunk = ((body.len() + 1) as u32).to_be_bytes().to_vec();
    chunk.push(kind);
    chunk.extend_from_slice(&body);
    let sectors = chunk.len().div_ceil(4096);
    if sectors > 255 {
        return Err("this chunk is too big for a region file".into());
    }
    let mut out = d.to_vec();
    let loc = u32::from_be_bytes(d[index * 4..index * 4 + 4].try_into().unwrap());
    let (mut off, count) = ((loc >> 8) as usize, (loc & 0xFF) as usize);
    if sectors > count {
        // Doesn't fit where it was: move it to the end of the file.
        off = out.len() / 4096;
        out.resize((off + sectors) * 4096, 0);
    }
    let start = off * 4096;
    let end = start + count.max(sectors) * 4096;
    out[start..end].fill(0);
    out[start..start + chunk.len()].copy_from_slice(&chunk);
    let new_loc = ((off as u32) << 8) | sectors as u32;
    out[index * 4..index * 4 + 4].copy_from_slice(&new_loc.to_be_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_round_trip() {
        let mut d = vec![0u8; 8192];
        let nbt = b"\x0a\x00\x00\x01\x00\x01a\x05\x00".to_vec();
        let r = region_replace(&d, 5, &nbt, 2).unwrap();
        d = r;
        assert_eq!(
            region_chunks(&d).unwrap(),
            vec![(5, "chunk 5,0".to_string())]
        );
        assert_eq!(region_open(&d, 5).unwrap(), (nbt.clone(), 2));
        // A much bigger chunk moves to the end of the file.
        let big: Vec<u8> = (0..20000u32).map(|i| (i * 7919 % 251) as u8).collect();
        let r = region_replace(&d, 5, &big, 2).unwrap();
        assert_eq!(region_open(&r, 5).unwrap().0, big);
    }

    #[test]
    fn bnd4_round_trip() {
        // Build a tiny BND4 with one plain and one encrypted entry.
        let plain = vec![0u8; 64];
        let mut enc_plain = vec![0u8; 64];
        enc_plain[5] = 9;
        let iv = [7u8; 16];
        let mut ct = enc_plain.clone();
        cbc::Encryptor::<aes::Aes128>::new(&DS3_KEY.into(), &iv.into())
            .encrypt_padded_mut::<NoPadding>(&mut ct, 64)
            .unwrap();
        let e0 = [Md5::digest(&plain).to_vec(), plain.clone()].concat();
        let body1 = [iv.to_vec(), ct].concat();
        let e1 = [Md5::digest(&body1).to_vec(), body1].concat();
        let mut d = vec![0u8; 0x100];
        d[..4].copy_from_slice(b"BND4");
        d[0x0C] = 2;
        d[0x20] = 0x20;
        d[0x30] = 1;
        let names = 0x80;
        let n0: Vec<u8> = "USER_DATA000\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let n1: Vec<u8> = "USER_DATA001\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        d[names..names + n0.len()].copy_from_slice(&n0);
        d[names + 0x20..names + 0x20 + n1.len()].copy_from_slice(&n1);
        for (i, (e, name_at)) in [(&e0, names), (&e1, names + 0x20)].into_iter().enumerate() {
            let h = 0x40 + i * 0x20;
            let off = d.len();
            d[h + 8..h + 16].copy_from_slice(&(e.len() as u64).to_le_bytes());
            d[h + 0x10..h + 0x14].copy_from_slice(&(off as u32).to_le_bytes());
            d[h + 0x14..h + 0x18].copy_from_slice(&(name_at as u32).to_le_bytes());
            d.extend_from_slice(e);
        }
        let entries = bnd4_entries(&d).unwrap();
        let (p0, k0, _) = bnd4_open(&d, &entries[0]).unwrap();
        assert_eq!((p0, k0), (plain, None));
        let (p1, k1, iv1) = bnd4_open(&d, &entries[1]).unwrap();
        assert_eq!((p1.clone(), k1), (enc_plain, Some(DS3_KEY)));
        let mut edited = p1;
        edited[0] = 42;
        let out = bnd4_replace(&d, "USER_DATA001", &edited, k1, &iv1).unwrap();
        let again = bnd4_open(&out, &bnd4_entries(&out).unwrap()[1]).unwrap();
        assert_eq!(again.0, edited);
    }

    #[test]
    #[ignore]
    fn real_souls_saves() {
        let home = std::env::var("APPDATA").unwrap();
        for dir in ["EldenRing", "DarkSoulsIII", "DarkSoulsII", "Sekiro"] {
            let Ok(walk) = std::fs::read_dir(format!("{home}\\{dir}")) else {
                continue;
            };
            for sub in walk.flatten().filter(|e| e.path().is_dir()) {
                for f in std::fs::read_dir(sub.path()).unwrap().flatten() {
                    let p = f.path();
                    if !p.extension().is_some_and(|x| x == "sl2") {
                        continue;
                    }
                    let d = std::fs::read(&p).unwrap();
                    let entries = bnd4_entries(&d).unwrap();
                    let first = bnd4_default(&d, &entries).unwrap();
                    for e in &entries {
                        let (plain, key, iv) = bnd4_open(&d, e).expect("checksum ok");
                        let back = bnd4_replace(&d, &e.name, &plain, key, &iv).unwrap();
                        assert_eq!(
                            back,
                            d,
                            "{} {} must re-encode identically",
                            p.display(),
                            e.name
                        );
                    }
                    let (p0, key, _) = bnd4_open(&d, &entries[first]).unwrap();
                    println!(
                        "{} {} entries, default {} encrypted {} zeros {:.0}%",
                        p.display(),
                        entries.len(),
                        entries[first].name,
                        key.is_some(),
                        zero_share(&p0) * 100.0
                    );
                }
            }
        }
    }
}
