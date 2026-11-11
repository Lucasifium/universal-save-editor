//! Dead Cells saves (`user_0.dat` and friends): a 59-byte header with a SHA-1 of the whole
//! file (taken with the hash field zeroed), then one zlib stream holding the save's chunks.
//!
//! Layout from the Dead Cells Community Modding Docs ("Save Files"). The chunks inside are
//! hxbit binary data, so they open in the hex view and the value search; saving packs them
//! again and writes a fresh SHA-1, so the game accepts the edited file.

use sha1::{Digest, Sha1};
use std::io::{Read, Write};

pub const HEADER: usize = 59;
const SUM: std::ops::Range<usize> = 5..25;

fn sum(file: &[u8]) -> [u8; 20] {
    let mut h = Sha1::new();
    h.update(&file[..SUM.start]);
    h.update([0u8; 20]);
    h.update(&file[SUM.end..]);
    h.finalize().into()
}

pub fn looks(d: &[u8]) -> bool {
    const MAGICS: [[u8; 4]; 4] = [
        [0xDE, 0xAD, 0xCE, 0x11],
        [0x11, 0xCE, 0xAD, 0xDE],
        [0x20, 0xDE, 0xAD, 0x19],
        [0x19, 0xAD, 0xDE, 0x20],
    ];
    d.len() > HEADER + 2 && MAGICS.iter().any(|m| d.starts_with(m))
}

/// The header (with its checksum checked) and the unpacked chunks.
pub fn open(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if !looks(d) || d[SUM] != sum(d) {
        return None;
    }
    let mut out = vec![];
    flate2::read::ZlibDecoder::new(&d[HEADER..])
        .read_to_end(&mut out)
        .ok()?;
    Some((d[..HEADER].to_vec(), out))
}

/// The save file for `header` and the (edited) chunks: packed at the game's level 9, with a
/// new checksum.
pub fn close(header: &[u8], chunks: &[u8]) -> Result<Vec<u8>, String> {
    let mut e = flate2::write::ZlibEncoder::new(header.to_vec(), flate2::Compression::best());
    e.write_all(chunks).map_err(|e| e.to_string())?;
    let mut file = e.finish().map_err(|e| e.to_string())?;
    let s = sum(&file);
    file[SUM].copy_from_slice(&s);
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_checksum() {
        let mut header = vec![0xDE, 0xAD, 0xCE, 0x11, 1];
        header.extend([0u8; 20]);
        header.extend([0xAB; 20]);
        header.extend(b"2025-01-01");
        header.extend(0x1C7u32.to_le_bytes());
        assert_eq!(header.len(), HEADER);
        let chunks = b"\x05\0\0\0hello\x08\0\0\0\0\0\0\0\0\0\xf0\x3f".to_vec();
        let file = close(&header, &chunks).unwrap();
        let (h, c) = open(&file).expect("opens");
        assert_eq!(c, chunks);
        assert_eq!(close(&h, &c).unwrap(), file);
        let mut bad = file.clone();
        bad[HEADER + 3] ^= 1;
        assert!(open(&bad).is_none());
    }
}
