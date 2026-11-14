//! Phoenix Wright: Ace Attorney Trilogy (PC and Switch) `systemdata`: a fixed 1,496,880-byte file.
//! The episodes unlocked in each game are the high four bits of the bytes at 0x12C4 … 0x12C6
//! (episode n unlocked = n << 4); the options follow. The positions follow emiyl/PWAATeditor
//! (GPL-3.0; see THIRD_PARTY_NOTICES.md). The file keeps no checksum.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Phoenix Wright: Ace Attorney Trilogy save";
const SIZE: usize = 1_496_880;
const GAMES: [(&str, i64); 3] = [("Phoenix Wright: Ace Attorney", 5), ("Justice for All", 4), ("Trials and Tribulations", 5)];

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && (0x12C4..0x12C7).all(|i| (1..=5).contains(&(d[i] >> 4))) && (1..=10).contains(&d[0x12C8]) && (1..=10).contains(&d[0x12CA])
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not an Ace Attorney Trilogy save".into());
    }
    let mut b = Builder::new(d);
    for (i, (game, n)) in GAMES.iter().enumerate() {
        b.bits(&["Episodes unlocked", game], 0x12C4 + i, Kind::U8, 4, 4, 0, 1, *n);
    }
    // Volumes are kept as 1 … 10.
    b.ranged(&["Options", "Music volume"], 0x12C8, Kind::U8, 1, 10);
    b.ranged(&["Options", "Sound effects volume"], 0x12CA, Kind::U8, 1, 10);
    if u16::from_le_bytes([d[0x12D8], d[0x12D9]]) != 0 {
        b.ranged(&["Options", "Resolution width"], 0x12D8, Kind::U16, 640, 7680);
        b.ranged(&["Options", "Resolution height"], 0x12DC, Kind::U16, 360, 4320);
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlocks_episodes() {
        let mut d = vec![0u8; SIZE];
        d[0x12C4..0x12C7].copy_from_slice(&[0x5f, 0x10, 0x10]);
        d[0x12C8] = 3;
        d[0x12CA] = 3;
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Episodes unlocked/Phoenix Wright: Ace Attorney"), Some(&serde_json::json!(5)));
        doc.set("/Episodes unlocked/Justice for All", serde_json::json!(4)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0x12C5], 0x40);
        assert_eq!(out[0x12C4], 0x5f);
        assert!(!looks(&vec![0u8; SIZE]));
    }

    #[test]
    fn public_sample() {
        let Ok(p) = std::env::var("PWAAT_SAMPLE") else { return };
        let d = std::fs::read(p).unwrap();
        let doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}
