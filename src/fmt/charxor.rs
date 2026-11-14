//! JSON whose every character is XORed with 0x81 (The Messenger's `SaveGame.txt`): the file is
//! UTF-8 text, and each character's code point is the JSON character's code point XOR 0x81.

const KEY: u32 = 0x81;

fn xor(text: &str) -> Option<String> {
    text.chars().map(|c| char::from_u32(c as u32 ^ KEY)).collect()
}

/// The decoded JSON text and whether the file ends in a plain newline (which isn't XORed), when
/// the file is such a save.
pub fn open(data: &[u8]) -> Option<(Vec<u8>, bool)> {
    // `{` XOR 0x81 is U+00FA, UTF-8 C3 BA.
    if !data.starts_with(&[0xC3, 0xBA]) {
        return None;
    }
    let (body, newline) = match data.strip_suffix(b"\n") {
        Some(b) => (b, true),
        None => (data, false),
    };
    let plain = xor(std::str::from_utf8(body).ok()?)?;
    serde_json::from_str::<serde_json::Value>(&plain).ok()?;
    Some((plain.into_bytes(), newline))
}

pub fn pack(json: &[u8], newline: bool) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(json).map_err(|e| e.to_string())?;
    let mut out = xor(text).map(String::into_bytes).ok_or_else(|| "a character can't be written back".to_string())?;
    if newline {
        out.push(b'\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let json = "{\"a\": [1, 2], \"b\": \"é\"}".as_bytes();
        let packed = pack(json, true).unwrap();
        assert_eq!(open(&packed).unwrap(), (json.to_vec(), true));
        assert!(open(json).is_none());
    }
}
