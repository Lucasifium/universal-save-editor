//! Minecraft Dungeons character saves (`.dat`): the bytes `D001` and four zero bytes, then
//! AES-256 in ECB mode (one fixed key, zero padding) of a JSON text.
//!
//! The key and layout are as published by MCDSaveEdit and DungeonTools (CutFlame); only
//! those facts are used.

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};

const MAGIC: &[u8; 8] = b"D001\0\0\0\0";
const KEY: [u8; 32] = [
    0x5C, 0xEB, 0x9D, 0x0A, 0xEB, 0xB9, 0x5A, 0xC0, 0x27, 0x0B, 0x0A, 0xF6, 0x75, 0x3D, 0xFC, 0x0E, 0xE3, 0xE6, 0x8B, 0xB6, 0x94, 0x79, 0x02, 0x0F, 0x24, 0x30, 0xE2, 0xEA, 0x00, 0x2B, 0xD4, 0xC9,
];

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 24 && d.starts_with(MAGIC) && (d.len() - 8) % 16 == 0
}

/// The JSON text inside a save.
pub fn open(d: &[u8]) -> Option<Vec<u8>> {
    if !looks(d) {
        return None;
    }
    let c = aes::Aes256::new(GenericArray::from_slice(&KEY));
    let mut out = d[8..].to_vec();
    for blk in out.chunks_mut(16) {
        c.decrypt_block(GenericArray::from_mut_slice(blk));
    }
    // The game's text ends at the first zero byte.
    if let Some(z) = out.iter().position(|b| *b == 0) {
        out.truncate(z);
    }
    let t = std::str::from_utf8(&out).ok()?.trim_start();
    (t.starts_with('{') || t.starts_with('[')).then_some(out)
}

pub fn pack(json: &[u8]) -> Vec<u8> {
    let c = aes::Aes256::new(GenericArray::from_slice(&KEY));
    let mut data = json.to_vec();
    while data.len() % 16 != 0 {
        data.push(0);
    }
    for blk in data.chunks_mut(16) {
        c.encrypt_block(GenericArray::from_mut_slice(blk));
    }
    [&MAGIC[..], &data].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let json = br#"{"currency":[{"type":"Emerald","count":12}],"xp":3}"#;
        let enc = pack(json);
        assert!(looks(&enc));
        assert_eq!(open(&enc).unwrap(), json);
    }

    #[test]
    fn real_save_opens() {
        let Ok(p) = std::env::var("MCD_SAMPLE") else { return };
        let d = std::fs::read(p).unwrap();
        let j = open(&d).expect("opens");
        println!("{}", String::from_utf8_lossy(&j));
        assert!(serde_json::from_slice::<serde_json::Value>(&j).is_ok());
    }
}
