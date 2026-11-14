//! Potion Craft `.pcsave`: several text lines, each the base64 of a JSON document XORed with a
//! 16-byte repeating key. Shown as one JSON array with a part per line (the first part is the
//! save's name and day, the second the game state).
//!
//! The key and layout are as published by PotionCraft_save_decoder (Varisox137, MIT).

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

const KEY: &[u8; 16] = b"a^19uh%47x71e%sd";

fn xor(d: &[u8]) -> Vec<u8> {
    d.iter().enumerate().map(|(i, b)| b ^ KEY[i % 16]).collect()
}

pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(data).ok()?;
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if lines.len() < 2 || !lines.iter().all(|l| l.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))) {
        return None;
    }
    let mut out = b"[".to_vec();
    for (i, l) in lines.iter().enumerate() {
        let plain = xor(&STANDARD.decode(l).ok()?);
        if plain.first() != Some(&b'{') {
            return None;
        }
        serde_json::from_slice::<Value>(&plain).ok()?;
        if i > 0 {
            out.push(b',');
        }
        out.extend(plain);
    }
    out.push(b']');
    Some(out)
}

pub fn pack(data: &[u8]) -> Result<Vec<u8>, String> {
    let parts: Vec<Value> = serde_json::from_slice(data).map_err(|e| format!("Potion Craft save: {e}"))?;
    let lines: Vec<String> = parts
        .iter()
        .map(|p| serde_json::to_string(p).map(|s| STANDARD.encode(xor(s.as_bytes()))).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    Ok(lines.join("\n").into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let a = STANDARD.encode(xor(br#"{"day":3,"customName":"x"}"#));
        let b = STANDARD.encode(xor(br#"{"gold":10,"playerInventory":[{"name":"Leaf","count":2}]}"#));
        let file = format!("{a}\n{b}");
        let json = open(file.as_bytes()).unwrap();
        assert_eq!(pack(&json).unwrap(), file.into_bytes());
    }
}
