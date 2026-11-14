//! Aliens: Fireteam Elite saves: a JSON text with every byte one less than the real character.

pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    // `{` is 0x7B, so the file starts with 0x7A.
    if data.first() != Some(&0x7A) || data.len() < 16 {
        return None;
    }
    let plain: Vec<u8> = data.iter().map(|b| b.wrapping_add(1)).collect();
    let head = std::str::from_utf8(&plain[..16]).ok()?;
    let rest = head.trim_start_matches('{').trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    serde_json::from_slice::<serde_json::Value>(&plain).ok()?;
    Some(plain)
}

pub fn pack(plain: &[u8]) -> Vec<u8> {
    plain.iter().map(|b| b.wrapping_sub(1)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let file = pack(b"{\r\n\t\"Name\": \"character\"\r\n}");
        assert_eq!(open(&file).unwrap(), b"{\r\n\t\"Name\": \"character\"\r\n}");
    }
}
