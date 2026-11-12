//! Stoneshard saves (`data.sav` inside `characters_v1/character_N/<save folder>/`): a zlib
//! stream of one JSON text, then the 32 hex digits of an MD5 over the JSON plus a salt made from
//! the folder names, then a zero byte. Layout and salt as published by stoneshard-save-editor
//! (denbkh, Apache-2.0).

use md5::{Digest, Md5};
use std::io::{Read, Write};

const OLD_SALT: &str = "stOne!characters_v1!shArd";

fn md5_hex(json: &[u8], salt: &str) -> String {
    let mut h = Md5::new();
    h.update(json);
    h.update(salt.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The salts to try: from the two folder names above the file, then the one the first game
/// versions used.
pub fn salts(path: &str) -> Vec<String> {
    let parts: Vec<&str> = path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    let mut v = vec![];
    if parts.len() >= 3 {
        v.push(format!("stOne!characters_v1!{}!{}!shArd", parts[parts.len() - 3], parts[parts.len() - 2]));
    }
    v.push(OLD_SALT.to_string());
    v
}

/// `(salt that matches, whether it was verified, the JSON)`.
pub fn open(d: &[u8], path: &str) -> Option<(String, bool, Vec<u8>)> {
    if d.len() < 8 || d[0] != 0x78 {
        return None;
    }
    let mut all = Vec::new();
    flate2::read::ZlibDecoder::new(d).read_to_end(&mut all).ok()?;
    if all.last() == Some(&0) {
        all.pop();
    }
    if all.len() < 40 {
        return None;
    }
    let (json, tail) = all.split_at(all.len() - 32);
    if !tail.iter().all(u8::is_ascii_hexdigit) || json.first() != Some(&b'{') || !json.windows(17).any(|w| w == b"\"characterDataMap") {
        return None;
    }
    serde_json::from_slice::<serde_json::Value>(json).ok()?;
    let tail = std::str::from_utf8(tail).ok()?.to_ascii_lowercase();
    let cands = salts(path);
    for s in &cands {
        if md5_hex(json, s) == tail {
            return Some((s.clone(), true, json.to_vec()));
        }
    }
    Some((cands[0].clone(), false, json.to_vec()))
}

pub fn pack(salt: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let v: serde_json::Value = serde_json::from_slice(data).map_err(|e| format!("The save is no longer valid JSON: {e}"))?;
    let json = serde_json::to_vec(&v).map_err(|e| e.to_string())?;
    let mut text = json.clone();
    text.extend_from_slice(md5_hex(&json, salt).as_bytes());
    text.push(0);
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&text).map_err(|e| e.to_string())?;
    z.finish().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_with_checksum() {
        let json = br#"{"characterDataMap":{"nameKey":"Ada","LVL":12.0,"XP":4300.0}}"#;
        let path = r"C:\x\characters_v1\character_2\exitsave_1\data.sav";
        let salt = salts(path)[0].clone();
        let file = pack(&salt, json).unwrap();
        let (s, ok, back) = open(&file, path).unwrap();
        assert!(ok);
        assert_eq!(s, salt);
        assert_eq!(back, json);
        // Another folder: opens, but can't be verified.
        let (_, ok, _) = open(&file, r"C:\y\characters_v1\character_3\exitsave_1\data.sav").unwrap();
        assert!(!ok);
    }
}
