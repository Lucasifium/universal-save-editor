//! Dragon's Dogma: Dark Arisen (PC) save: a 32-byte header and one zlib stream holding the save
//! as XML, in a file that is always 524,288 bytes long (the rest is zero).
//!
//! Header (little-endian u32s): version (21), unpacked size, packed size, three fixed marks, the
//! CRC-32 of the packed bytes (inverted) and a last fixed mark. The layout is public knowledge
//! (FluffyQuack's DDsavetool, ported to Python by RamKromberg, who says it may be used freely);
//! the code here is new.

use std::io::{Read, Write};

pub const FILE_SIZE: usize = 524_288;
const HEAD: usize = 32;
const MARKS: [(usize, u32); 4] = [(12, 860_693_325), (16, 0), (20, 860_700_740), (28, 1_079_398_965)];

fn word(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

/// `crc32(packed) ^ 0xFFFFFFFF`, which is how the game stores it.
fn sum(packed: &[u8]) -> u32 {
    let mut h = flate2::Crc::new();
    h.update(packed);
    h.sum() ^ 0xFFFF_FFFF
}

/// The XML text of a save, only when the header, both sizes and the checksum all agree.
pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() != FILE_SIZE || word(data, 0) != 21 || MARKS.iter().any(|&(at, v)| word(data, at) != v) {
        return None;
    }
    let (raw, packed) = (word(data, 4) as usize, word(data, 8) as usize);
    if packed == 0 || packed > FILE_SIZE - HEAD || raw == 0 || raw > (1 << 28) {
        return None;
    }
    let stream = &data[HEAD..HEAD + packed];
    if sum(stream) != word(data, 24) {
        return None;
    }
    let mut out = Vec::with_capacity(raw);
    flate2::read::ZlibDecoder::new(stream).read_to_end(&mut out).ok()?;
    (out.len() == raw && out.starts_with(b"<?xml")).then_some(out)
}

/// The file for an XML text: packed, with the header written again and the rest zero.
pub fn pack(xml: &[u8]) -> Result<Vec<u8>, String> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
    z.write_all(xml).map_err(|e| e.to_string())?;
    let packed = z.finish().map_err(|e| e.to_string())?;
    if packed.len() > FILE_SIZE - HEAD {
        return Err("The edited save no longer fits the game's fixed 512 KB save file".into());
    }
    let mut out = vec![0u8; FILE_SIZE];
    out[0..4].copy_from_slice(&21u32.to_le_bytes());
    out[4..8].copy_from_slice(&(xml.len() as u32).to_le_bytes());
    out[8..12].copy_from_slice(&(packed.len() as u32).to_le_bytes());
    for (at, v) in MARKS {
        out[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    out[24..28].copy_from_slice(&sum(&packed).to_le_bytes());
    out[HEAD..HEAD + packed.len()].copy_from_slice(&packed);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let xml = b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<class name=\"x\" type=\"sSave::saveDataAllDA\">\n<u32 name=\"mGold\" value=\"5\"/>\n</class>\n";
        let file = pack(xml).unwrap();
        assert_eq!(file.len(), FILE_SIZE);
        assert_eq!(open(&file).unwrap(), xml);
        // A wrong checksum, size or mark is turned down.
        let mut bad = file.clone();
        bad[24] ^= 1;
        assert!(open(&bad).is_none());
        let mut bad = file.clone();
        bad[16] = 1;
        assert!(open(&bad).is_none());
        assert!(open(&file[..FILE_SIZE - 1]).is_none());
    }

    #[test]
    fn too_big_is_refused() {
        // Incompressible data cannot fit.
        let mut x: u64 = 1;
        let noise: Vec<u8> = (0..FILE_SIZE).map(|_| { x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (x >> 33) as u8 }).collect();
        assert!(pack(&noise).is_err());
    }

    /// A real save: `DDDA_SAVE=path cargo test --release --lib ddda_real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn ddda_real() {
        use crate::formats::{decode, encode, Content};
        let raw = std::fs::read(std::env::var("DDDA_SAVE").unwrap()).unwrap();
        let t = std::time::Instant::now();
        let mut d = decode(&raw);
        println!("decoded in {:?}, layers {:?}", t.elapsed(), d.layers);
        let Content::Tree(tree) = &mut d.content else { panic!("not a tree") };
        let g = "/dd_savedata1018/mPlayerDataManual/mPlCmcEditAndParam/mPl/mParam/mGold";
        println!("kind {} gold {:?}", tree.kind_name, tree.view.pointer(g));
        tree.set(g, serde_json::json!(9_999_999)).unwrap();
        let out = encode(&d.layers, &d.content).unwrap();
        println!("repacked {} bytes", out.len());
        let back = decode(&out);
        let Content::Tree(t2) = &back.content else { panic!() };
        println!("gold after {:?} base {:?}", t2.view.pointer(g), t2.view.pointer(&g.replace("Manual", "Base")));
    }
}
