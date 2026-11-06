//! Larian's LSPK package (`.lsv` saves of Divinity: Original Sin 2 and Baldur's Gate 3).
//!
//! Version 13 (DOS2 Definitive Edition) keeps its header at the end of the file
//! (`… header, u32 header size, "LSPK"`); versions 15, 16 and 18 (BG3) start with it.
//! The file table is an LZ4 block of fixed-size entries; each file is stored on its own,
//! compressed with zlib, LZ4 or zstd (low nibble of its flags), padded with 0xAD to 64
//! bytes. The header's MD5 is taken over every file's *unpacked* bytes in
//! case-insensitive name order, then each digest byte is raised by one.
//!
//! Layout ported from LSLib (Norbyte, MIT): `PackageFormat.cs`, `PackageReader.cs`,
//! `PackageWriter.cs`; checked against sample saves of both games.

use md5::{Digest, Md5};
use std::collections::HashMap;
use std::io::{Read, Write};

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub offset: u64,
    pub size_on_disk: u64,
    pub size: u64,
    pub flags: u8,
    /// The entry's raw bytes in the table (rewritten in place on save).
    raw: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Package {
    pub version: u32,
    pub entries: Vec<Entry>,
    /// Where file data starts (after a leading header).
    data_start: usize,
    /// The header's checksum (checked by tests; always rewritten on save).
    #[cfg_attr(not(test), allow(dead_code))]
    pub md5: [u8; 16],
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

fn entry_size(version: u32) -> Option<usize> {
    match version {
        10 | 13 => Some(280),
        15 | 16 => Some(296),
        18 => Some(272),
        _ => None,
    }
}

/// Is this an LSPK package (either header placement)?
pub fn looks(d: &[u8]) -> bool {
    d.len() > 48 && (d.starts_with(b"LSPK") || d.ends_with(b"LSPK"))
}

pub fn read(d: &[u8]) -> Option<Package> {
    if !looks(d) {
        return None;
    }
    let (version, list_at, md5, data_start, table) = if d.starts_with(b"LSPK") {
        let version = u32_at(d, 4)?;
        if !matches!(version, 15 | 16 | 18) {
            return None;
        }
        let list_at = usize::try_from(u64_at(d, 8)?).ok()?;
        let md5: [u8; 16] = d.get(22..38)?.try_into().ok()?;
        let parts = if version >= 16 { u16_at(d, 38)? } else { 1 };
        if parts != 1 {
            return None;
        }
        let csize = u32_at(d, list_at + 4)? as usize;
        (version, list_at, md5, if version >= 16 { 40 } else { 38 }, d.get(list_at + 8..list_at + 8 + csize)?)
    } else {
        let hsize = u32_at(d, d.len() - 8)? as usize;
        let h = d.len().checked_sub(hsize)?;
        let version = u32_at(d, h)?;
        if version != 13 {
            return None;
        }
        let list_at = u32_at(d, h + 4)? as usize;
        let list_size = u32_at(d, h + 8)? as usize;
        if u16_at(d, h + 12)? != 1 {
            return None;
        }
        let md5: [u8; 16] = d.get(h + 16..h + 32)?.try_into().ok()?;
        (version, list_at, md5, 0, d.get(list_at + 4..list_at + list_size)?)
    };
    let esz = entry_size(version)?;
    let n = u32_at(d, list_at)? as usize;
    if n == 0 || n > 100_000 {
        return None;
    }
    let table = lz4_flex::block::decompress(table, n * esz).ok()?;
    if table.len() != n * esz {
        return None;
    }
    let mut entries = Vec::with_capacity(n);
    for e in table.chunks(esz) {
        let name_len = e[..256].iter().position(|&c| c == 0).unwrap_or(256);
        let name = String::from_utf8_lossy(&e[..name_len]).into_owned();
        let (offset, size_on_disk, size, part, flags) = match version {
            18 => (u64::from(u32_at(e, 256)?) | u64::from(u16_at(e, 260)?) << 32, u64::from(u32_at(e, 264)?), u64::from(u32_at(e, 268)?), u32::from(e[262]), e[263]),
            15 | 16 => (u64_at(e, 256)?, u64_at(e, 264)?, u64_at(e, 272)?, u32_at(e, 280)?, u32_at(e, 284)? as u8),
            _ => (u64::from(u32_at(e, 256)?), u64::from(u32_at(e, 260)?), u64::from(u32_at(e, 264)?), u32_at(e, 268)?, u32_at(e, 272)? as u8),
        };
        if part != 0 || offset.checked_add(size_on_disk)? > list_at as u64 {
            return None;
        }
        entries.push(Entry { name, offset, size_on_disk, size, flags, raw: e.to_vec() });
    }
    Some(Package { version, entries, data_start, md5 })
}

/// Unpacks data compressed with a Larian compression method (low nibble of `flags`).
/// `frame`: LZ4 data is a frame (chunked) rather than a single block.
pub fn decompress(data: &[u8], flags: u8, size: usize, frame: bool) -> Option<Vec<u8>> {
    let out = match flags & 0x0F {
        0 => data.to_vec(),
        1 => {
            let mut out = Vec::with_capacity(size);
            flate2::read::ZlibDecoder::new(data).read_to_end(&mut out).ok()?;
            out
        }
        2 if frame => {
            let mut out = Vec::with_capacity(size);
            lz4_flex::frame::FrameDecoder::new(data).read_to_end(&mut out).ok()?;
            out
        }
        2 => lz4_flex::block::decompress(data, size).ok()?,
        3 => zstd::decode_all(data).ok()?,
        _ => return None,
    };
    (out.len() == size).then_some(out)
}

/// Packs data the way `decompress` reads it (level from the high nibble of `flags`).
pub fn compress(data: &[u8], flags: u8, frame: bool) -> Result<Vec<u8>, String> {
    let io = |e: std::io::Error| e.to_string();
    let level = flags & 0xF0;
    Ok(match flags & 0x0F {
        0 => data.to_vec(),
        1 => {
            let l = match level {
                0x10 => flate2::Compression::fast(),
                0x40 => flate2::Compression::best(),
                _ => flate2::Compression::default(),
            };
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), l);
            e.write_all(data).map_err(io)?;
            e.finish().map_err(io)?
        }
        2 if frame => {
            let mut e = lz4_flex::frame::FrameEncoder::new(Vec::new());
            e.write_all(data).map_err(io)?;
            e.finish().map_err(|e| e.to_string())?
        }
        2 => lz4_flex::block::compress(data),
        3 => zstd::encode_all(data, if level == 0x40 { 9 } else { 3 }).map_err(io)?,
        m => return Err(format!("unknown compression method {m}")),
    })
}

impl Package {
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))
    }

    /// A file's unpacked contents.
    pub fn file(&self, d: &[u8], e: &Entry) -> Option<Vec<u8>> {
        let raw = d.get(usize::try_from(e.offset).ok()?..usize::try_from(e.offset + e.size_on_disk).ok()?)?;
        if e.size == 0 {
            return Some(vec![]);
        }
        decompress(raw, e.flags, e.size as usize, false)
    }

    /// The header checksum for these unpacked files (name → contents).
    fn checksum(&self, files: &HashMap<String, Vec<u8>>) -> [u8; 16] {
        let mut names: Vec<&String> = files.keys().collect();
        names.sort_by_key(|n| n.to_ascii_lowercase());
        let mut h = Md5::new();
        for n in names {
            h.update(&files[n]);
        }
        let mut out: [u8; 16] = h.finalize().into();
        for b in &mut out {
            *b = b.wrapping_add(1);
        }
        out
    }

    /// Does the header's checksum match the files?
    #[cfg(test)]
    pub fn checksum_ok(&self, d: &[u8]) -> bool {
        let mut files = HashMap::new();
        for e in &self.entries {
            let Some(b) = self.file(d, e) else { return false };
            files.insert(e.name.clone(), b);
        }
        self.checksum(&files) == self.md5
    }

    /// Writes the package again with some files replaced (name → new unpacked contents),
    /// each packed with its own method; the other files are copied as they are. The table,
    /// offsets, CRCs and the header's checksum are updated.
    pub fn rebuild(&self, d: &[u8], replace: &HashMap<String, Vec<u8>>) -> Result<Vec<u8>, String> {
        let mut unpacked: HashMap<String, Vec<u8>> = HashMap::new();
        let mut stored: Vec<Vec<u8>> = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            let new = replace.iter().find(|(k, _)| k.eq_ignore_ascii_case(&e.name)).map(|(_, v)| v);
            let plain = match new {
                Some(v) => v.clone(),
                None => self.file(d, e).ok_or(format!("{} doesn't unpack", e.name))?,
            };
            let disk = match new {
                Some(v) if v.is_empty() => vec![],
                Some(v) => compress(v, e.flags, false)?,
                None => d[e.offset as usize..(e.offset + e.size_on_disk) as usize].to_vec(),
            };
            unpacked.insert(e.name.clone(), plain);
            stored.push(disk);
        }
        // Files keep their order on disk (case-insensitive name order in the game's own saves).
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        order.sort_by_key(|&i| self.entries[i].offset);
        let mut out = if self.data_start > 0 { d[..self.data_start].to_vec() } else { vec![] };
        let mut table = Vec::with_capacity(self.entries.len() * entry_size(self.version).unwrap_or(0));
        let mut new_at = vec![0u64; self.entries.len()];
        for &i in &order {
            new_at[i] = out.len() as u64;
            out.extend_from_slice(&stored[i]);
            while (out.len() - self.data_start) % 64 != 0 {
                out.push(0xAD);
            }
        }
        for (i, e) in self.entries.iter().enumerate() {
            let mut raw = e.raw.clone();
            let (at, n) = (new_at[i], stored[i].len() as u64);
            let size = unpacked[&e.name].len() as u64;
            let crc = || {
                let mut c = flate2::Crc::new();
                c.update(&stored[i]);
                c.sum()
            };
            match self.version {
                18 => {
                    raw[256..260].copy_from_slice(&(at as u32).to_le_bytes());
                    raw[260..262].copy_from_slice(&((at >> 32) as u16).to_le_bytes());
                    raw[264..268].copy_from_slice(&(n as u32).to_le_bytes());
                    raw[268..272].copy_from_slice(&(size as u32).to_le_bytes());
                }
                15 | 16 => {
                    raw[256..264].copy_from_slice(&at.to_le_bytes());
                    raw[264..272].copy_from_slice(&n.to_le_bytes());
                    raw[272..280].copy_from_slice(&size.to_le_bytes());
                    raw[288..292].copy_from_slice(&crc().to_le_bytes());
                }
                _ => {
                    let at = u32::try_from(at).map_err(|_| "the package is too big for its format")?;
                    raw[256..260].copy_from_slice(&at.to_le_bytes());
                    raw[260..264].copy_from_slice(&(n as u32).to_le_bytes());
                    raw[264..268].copy_from_slice(&(size as u32).to_le_bytes());
                    raw[276..280].copy_from_slice(&crc().to_le_bytes());
                }
            }
            table.extend(raw);
        }
        let md5 = self.checksum(&unpacked);
        let packed_table = lz4_flex::block::compress(&table);
        let list_at = out.len();
        out.extend((self.entries.len() as u32).to_le_bytes());
        if self.version == 13 {
            out.extend(&packed_table);
            let h = d.len() - u32_at(d, d.len() - 8).unwrap_or(40) as usize;
            let mut head = d[h..].to_vec();
            head[4..8].copy_from_slice(&(list_at as u32).to_le_bytes());
            head[8..12].copy_from_slice(&(4 + packed_table.len() as u32).to_le_bytes());
            head[16..32].copy_from_slice(&md5);
            out.extend(head);
        } else {
            out.extend((packed_table.len() as u32).to_le_bytes());
            out.extend(&packed_table);
            out[8..16].copy_from_slice(&(list_at as u64).to_le_bytes());
            out[16..20].copy_from_slice(&(8 + packed_table.len() as u32).to_le_bytes());
            out[22..38].copy_from_slice(&md5);
        }
        Ok(out)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A package of `files` (name, contents) in the given version, each file packed with
    /// `flags`, laid out and checksummed the way the games write them.
    pub fn build(version: u32, files: &[(&str, &[u8])], flags: u8) -> Vec<u8> {
        let esz = entry_size(version).unwrap();
        let start = if version == 13 { 0 } else { 40 };
        let mut out = vec![0u8; start];
        let mut sorted: Vec<&(&str, &[u8])> = files.iter().collect();
        sorted.sort_by_key(|f| f.0.to_ascii_lowercase());
        let mut table = vec![];
        let mut h = Md5::new();
        for (name, data) in sorted {
            h.update(data);
            let packed = compress(data, flags, false).unwrap();
            let at = out.len() as u64;
            out.extend(&packed);
            while (out.len() - start) % 64 != 0 {
                out.push(0xAD);
            }
            let mut e = vec![0u8; esz];
            e[..name.len()].copy_from_slice(name.as_bytes());
            if version == 18 {
                e[256..260].copy_from_slice(&(at as u32).to_le_bytes());
                e[263] = flags;
                e[264..268].copy_from_slice(&(packed.len() as u32).to_le_bytes());
                e[268..272].copy_from_slice(&(data.len() as u32).to_le_bytes());
            } else {
                e[256..260].copy_from_slice(&(at as u32).to_le_bytes());
                e[260..264].copy_from_slice(&(packed.len() as u32).to_le_bytes());
                e[264..268].copy_from_slice(&(data.len() as u32).to_le_bytes());
                e[272] = flags;
                let mut c = flate2::Crc::new();
                c.update(&packed);
                e[276..280].copy_from_slice(&c.sum().to_le_bytes());
            }
            table.extend(e);
        }
        let md5: Vec<u8> = h.finalize().iter().map(|b| b.wrapping_add(1)).collect();
        let packed = lz4_flex::block::compress(&table);
        let list_at = out.len();
        out.extend((files.len() as u32).to_le_bytes());
        if version == 13 {
            out.extend(&packed);
            out.extend(13u32.to_le_bytes());
            out.extend((list_at as u32).to_le_bytes());
            out.extend((4 + packed.len() as u32).to_le_bytes());
            out.extend(1u16.to_le_bytes());
            out.extend([1u8, 0]);
            out.extend(&md5);
            out.extend(40u32.to_le_bytes());
            out.extend(b"LSPK");
        } else {
            out.extend((packed.len() as u32).to_le_bytes());
            out.extend(&packed);
            out[..4].copy_from_slice(b"LSPK");
            out[4..8].copy_from_slice(&version.to_le_bytes());
            out[8..16].copy_from_slice(&(list_at as u64).to_le_bytes());
            out[16..20].copy_from_slice(&(8 + packed.len() as u32).to_le_bytes());
            out[22..38].copy_from_slice(&md5);
            out[38..40].copy_from_slice(&1u16.to_le_bytes());
        }
        out
    }

    #[test]
    fn reads_and_rebuilds_both_layouts() {
        for (version, flags) in [(13, 0x21), (18, 0x23), (18, 0x22)] {
            let a: Vec<u8> = (0..5000u32).map(|i| (i % 7) as u8).collect();
            let pkg_bytes = build(version, &[("meta.lsf", b"meta data"), ("Globals.lsf", &a), ("b.png", b"")], flags);
            let p = read(&pkg_bytes).unwrap();
            assert_eq!(p.version, version);
            assert!(p.checksum_ok(&pkg_bytes));
            assert_eq!(p.file(&pkg_bytes, p.find("globals.lsf").unwrap()).unwrap(), a);
            // Rebuilding with nothing changed gives the same layout.
            let same = p.rebuild(&pkg_bytes, &HashMap::new()).unwrap();
            let q = read(&same).unwrap();
            assert!(q.checksum_ok(&same));
            assert_eq!(q.entries.iter().map(|e| (e.name.clone(), e.offset)).collect::<Vec<_>>(), p.entries.iter().map(|e| (e.name.clone(), e.offset)).collect::<Vec<_>>());
            // A changed file moves the ones after it and the checksum follows.
            let mut r = HashMap::new();
            r.insert("Globals.lsf".to_string(), b"short now".to_vec());
            let out = p.rebuild(&pkg_bytes, &r).unwrap();
            let q = read(&out).unwrap();
            assert!(q.checksum_ok(&out));
            assert_ne!(q.md5, p.md5);
            assert_eq!(q.file(&out, q.find("Globals.lsf").unwrap()).unwrap(), b"short now");
            assert_eq!(q.file(&out, q.find("meta.lsf").unwrap()).unwrap(), b"meta data");
            assert_eq!(q.find("b.png").unwrap().size, 0);
        }
    }
}
