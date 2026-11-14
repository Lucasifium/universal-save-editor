//! Don't Starve save: `KLEI     1D`, then base64 of a 16-byte header (version, header size,
//! text size, packed size) and a zlib stream of Lua text. Layout facts only; the code is new.

use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::{Read, Write};

const MAGIC: &[u8] = b"KLEI     1D";

pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    let body = data.strip_prefix(MAGIC)?;
    let raw = STANDARD.decode(body.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect::<Vec<u8>>()).ok()?;
    let packed = raw.get(16..)?;
    let mut text = Vec::new();
    flate2::read::ZlibDecoder::new(packed).read_to_end(&mut text).ok()?;
    Some(text)
}

pub fn pack(text: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = z.write_all(text);
    let packed = z.finish().unwrap_or_default();
    let mut raw = Vec::with_capacity(16 + packed.len());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.extend_from_slice(&16u32.to_le_bytes());
    raw.extend_from_slice(&(text.len() as u32).to_le_bytes());
    raw.extend_from_slice(&(packed.len() as u32).to_le_bytes());
    raw.extend_from_slice(&packed);
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(STANDARD.encode(&raw).as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let file = pack(b"return {a=1}");
        assert_eq!(open(&file).unwrap(), b"return {a=1}");
    }
}
