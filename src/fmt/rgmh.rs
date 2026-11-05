//! BioShock Remastered / BioShock 2 Remastered `.bsb` saves, worked out from sample saves:
//! "RGMH", version, header size, …, the size of a PNG thumbnail at 0x14, the thumbnail at
//! the header's end, the save's title and the names of the level files inside, then each
//! level file as an Unreal package whose data is a run of (u32 packed size, zlib → 32 KB)
//! blocks.
//!
//! The package tables that point at those blocks aren't rewritten: the editor unpacks every
//! block in place and, when saving, packs each one again into exactly the space it had
//! (zlib stops at its own end, so the rest of the space is padding).

use std::io::{Read, Write};

/// For each block: where it sits in the file, its space, where its data sits in the view, and its size.
pub type Blocks = Vec<(usize, usize, usize, usize)>;

pub fn looks(d: &[u8]) -> bool {
    d.len() > 0x2100 && d.starts_with(b"RGMH")
}

/// Inflates `z`, which must be one zlib stream (and zero padding).
fn inflate_exact(z: &[u8]) -> Option<Vec<u8>> {
    let mut dec = flate2::read::ZlibDecoder::new(z);
    let mut out = Vec::new();
    dec.read_to_end(&mut out).ok()?;
    // The stream may end before its space does (padding, as the editor writes it).
    let used = dec.total_in() as usize;
    (used >= z.len().min(8) && z[used..].iter().all(|&b| b == 0)).then_some(out)
}

pub fn unpack(d: &[u8]) -> Option<(Blocks, Vec<u8>)> {
    if !looks(d) {
        return None;
    }
    let mut view = Vec::with_capacity(d.len() * 3);
    let mut blocks = Vec::new();
    let mut p = 0; // copied up to here
    let mut q = 0x2028; // searching from here
    while q + 6 < d.len() {
        if d[q] == 0x78 && matches!(d[q + 1], 0x01 | 0x5e | 0x9c | 0xda) && q >= 4 {
            let len = u32::from_le_bytes(d[q - 4..q].try_into().unwrap()) as usize;
            if len > 2 && q + len <= d.len() {
                if let Some(raw) = inflate_exact(&d[q..q + len]) {
                    view.extend(&d[p..q]);
                    blocks.push((q, len, view.len(), raw.len()));
                    view.extend(raw);
                    q += len;
                    p = q;
                    continue;
                }
            }
        }
        q += 1;
    }
    view.extend(&d[p..]);
    (!blocks.is_empty()).then_some((blocks, view))
}

/// Packs the view again: blocks whose data didn't change keep the original bytes; a changed
/// block is packed into exactly the space it had.
pub fn pack_from(original: &[u8], blocks: &Blocks, view: &[u8]) -> Result<Vec<u8>, String> {
    let (_, old) = unpack(original).ok_or("the original save can't be read")?;
    let mut out = Vec::with_capacity(original.len());
    let mut v = 0;
    for &(at, space, from, raw) in blocks {
        if from < v || from + raw > view.len() {
            return Err("this save's data can't change size".into());
        }
        out.extend(&view[v..from]);
        if out.len() != at {
            return Err("this save's data can't change size".into());
        }
        let data = &view[from..from + raw];
        if old.get(from..from + raw) == Some(data) {
            out.extend(&original[at..at + space]);
        } else {
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            z.write_all(data).map_err(|e| e.to_string())?;
            let mut z = z.finish().map_err(|e| e.to_string())?;
            if z.len() > space {
                return Err("this change doesn't fit in the save's space for it; try a smaller change".into());
            }
            z.resize(space, 0);
            out.extend(z);
        }
        v = from + raw;
    }
    out.extend(&view[v..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(data: &[u8]) -> Vec<u8> {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        z.write_all(data).unwrap();
        let z = z.finish().unwrap();
        let mut out = (z.len() as u32).to_le_bytes().to_vec();
        out.extend(z);
        out
    }

    #[test]
    fn blocks_keep_their_space() {
        let mut file = b"RGMH".to_vec();
        file.resize(0x2028, 0);
        file.extend(b"title");
        let a: Vec<u8> = (0..32768u32).map(|i| (i % 7) as u8).collect();
        file.extend(block(&a));
        file.extend(b"between");
        file.extend(block(&a));
        file.extend(b"end");
        let (blocks, mut view) = unpack(&file).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(pack_from(&file, &blocks, &view).unwrap(), file);
        view[blocks[1].2 + 5] = 99;
        let out = pack_from(&file, &blocks, &view).unwrap();
        assert_eq!(out.len(), file.len());
        let (_, back) = unpack(&out).unwrap();
        assert_eq!(back, view);
        assert_eq!(out[..blocks[0].0 + blocks[0].1], file[..blocks[0].0 + blocks[0].1], "the unchanged block is untouched");
    }

    /// Research aid: SAVE_EDITOR_RGMH = a .bsb save.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_RGMH").unwrap()).unwrap();
        let t = std::time::Instant::now();
        let (b, v) = unpack(&d).expect("unpack");
        println!("{} blocks, {} bytes unpacked in {:?}", b.len(), v.len(), t.elapsed());
        assert_eq!(pack_from(&d, &b, &v).unwrap(), d);
    }
}

#[cfg(test)]
mod peek {
    #[test]
    #[ignore]
    fn dump_view() {
        let p = std::env::var("SAVE_EDITOR_RGMH").unwrap();
        let d = std::fs::read(&p).unwrap();
        let (_, v) = super::unpack(&d).unwrap();
        std::fs::write(format!("{p}.view"), v).unwrap();
    }
}
