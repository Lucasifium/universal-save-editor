//! A one-line peek at a save for the "Find my saves" list (character, level, place), read
//! from a header the format keeps up front, so listing many saves stays quick. Only formats
//! whose header can be read without decoding the save are covered.

use std::io::Read;
use std::path::Path;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// A string stored as a 16-bit length and that many bytes; returns it and the next offset.
fn wstring(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize;
    let bytes = d.get(at + 2..at + 2 + n)?;
    Some((bytes.iter().map(|&b| b as char).collect(), at + 2 + n))
}

/// Skyrim (LE and Special Edition) and Fallout 4 saves: name, level, location.
fn bethesda(head: &[u8]) -> Option<String> {
    let magic_len = if head.starts_with(b"TESV_SAVEGAME") {
        13
    } else if head.starts_with(b"FO4_SAVEGAME") {
        12
    } else {
        return None;
    };
    // magic, header size, then: version, save number, name, level, location.
    let at = magic_len + 4 + 4 + 4;
    let (name, at) = wstring(head, at)?;
    let level = u32_at(head, at)?;
    let (place, _) = wstring(head, at + 4)?;
    if name.trim().is_empty() || level == 0 || level > 10_000 {
        return None;
    }
    let place = place.trim();
    Some(if place.is_empty() { format!("{name}, level {level}") } else { format!("{name}, level {level}, {place}") })
}

/// "Name, level 12, Place" for a save file, when its header allows it.
pub fn peek(path: &Path) -> Option<String> {
    let mut head = [0u8; 512];
    let n = std::fs::File::open(path).ok()?.read(&mut head).ok()?;
    bethesda(&head[..n])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skyrim(name: &str, level: u32, place: &str) -> Vec<u8> {
        let mut d = b"TESV_SAVEGAME".to_vec();
        d.extend(100u32.to_le_bytes()); // header size
        d.extend(12u32.to_le_bytes()); // version
        d.extend(7u32.to_le_bytes()); // save number
        d.extend((name.len() as u16).to_le_bytes());
        d.extend(name.bytes());
        d.extend(level.to_le_bytes());
        d.extend((place.len() as u16).to_le_bytes());
        d.extend(place.bytes());
        d
    }

    #[test]
    fn reads_a_skyrim_header() {
        assert_eq!(bethesda(&skyrim("Lydia", 24, "Whiterun")).as_deref(), Some("Lydia, level 24, Whiterun"));
        assert_eq!(bethesda(&skyrim("Lydia", 24, "")).as_deref(), Some("Lydia, level 24"));
    }

    #[test]
    fn other_files_have_no_preview() {
        assert_eq!(bethesda(b"not a save at all"), None);
        assert_eq!(bethesda(&skyrim("", 5, "x")), None);
        assert_eq!(bethesda(&skyrim("A", 0, "x")), None);
        assert_eq!(bethesda(b"TESV_SAVEGAME\x00\x00"), None, "a cut-off header is not read past its end");
    }
}
