//! World War Z `user_progression.dat` / `user_profile.dat`: a zlib stream whose first four bytes
//! are XORed with a key worked out from the fourth character of the file name; the text inside
//! is JSON with a trailing zero byte.
//!
//! The key formula is as published by World-War-Z-Save-Editor (Chase-Simmons, MIT).

use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

fn key(seed: u8) -> [u8; 4] {
    let k = 0x358637BDu32.wrapping_mul(0x49F42400).wrapping_add(0x40490FDB).wrapping_mul(seed as u32);
    k.to_le_bytes()
}

fn xor4(d: &mut [u8], seed: u8) {
    let k = key(seed);
    for (b, k) in d.iter_mut().zip(k) {
        *b ^= k;
    }
}

/// The seed (the file name's fourth character) and the JSON text.
pub fn open(data: &[u8], file_name: &str) -> Option<(u8, Vec<u8>)> {
    let seed = *file_name.as_bytes().get(3)?;
    if data.len() < 8 {
        return None;
    }
    let mut d = data.to_vec();
    xor4(&mut d, seed);
    if d[0] != 0x78 {
        return None;
    }
    let mut out = Vec::new();
    ZlibDecoder::new(&d[..]).read_to_end(&mut out).ok()?;
    (out.first() == Some(&b'{')).then_some((seed, out))
}

pub fn pack(seed: u8, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::new(1));
    e.write_all(plain).map_err(|e| e.to_string())?;
    let mut d = e.finish().map_err(|e| e.to_string())?;
    xor4(&mut d, seed);
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let plain = b"{\"UserProgression\":{\"Shared\":{}}}\0".to_vec();
        let packed = pack(b'r', &plain).unwrap();
        assert_eq!(open(&packed, "user_progression.dat").unwrap().1, plain);
        assert!(open(&packed, "abcXe").is_none());
    }
}
