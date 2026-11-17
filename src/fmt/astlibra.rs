//! ASTLIBRA Revision `SAVE_<n>.DAT`: a 0x74xx-byte file that starts with "ALBR_SAV" and keeps
//! its values at fixed places. The last eight bytes are two CRC-32s: one of everything before
//! the final 0x400 bytes of data (the part the load screen shows), one of those 0x400 bytes.
//!
//! Offsets as published by qazmlpok's astlibra-save-editor (MIT): level 0x7370, experience
//! towards the next level 0x7390, gold 0x7394, the six force pools from 0x73B4 (fire, earth,
//! water, darkness, wind, holy). See THIRD_PARTY_NOTICES.md.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "ASTLIBRA Revision save";
const MAGIC: &[u8] = b"ALBR_SAV";
const MIN: usize = 0x73D0;
const TAIL: usize = 400;

const FORCE: [(&str, usize); 6] = [("Fire", 0), ("Earth", 1), ("Water", 2), ("Darkness", 3), ("Wind", 4), ("Holy", 5)];

fn crc32(d: &[u8]) -> u32 {
    !super::crc32_raw(!0, d)
}

fn sums(d: &[u8]) -> Option<(u32, u32)> {
    let body = d.len().checked_sub(8)?;
    let left = body.checked_sub(TAIL)?;
    Some((crc32(&d[..left]), crc32(&d[left..body])))
}

/// The magic, a plausible size and both checksums matching: a loose "ALBR_SAV" prefix alone
/// is not enough.
pub fn looks(d: &[u8]) -> bool {
    if d.len() < MIN || !d.starts_with(MAGIC) {
        return false;
    }
    let n = d.len();
    sums(d).is_some_and(|(l, r)| d[n - 8..n - 4] == l.to_le_bytes() && d[n - 4..] == r.to_le_bytes())
}

fn fix(d: &mut Vec<u8>) {
    if let Some((l, r)) = sums(d) {
        let n = d.len();
        d[n - 8..n - 4].copy_from_slice(&l.to_le_bytes());
        d[n - 4..].copy_from_slice(&r.to_le_bytes());
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not an ASTLIBRA Revision save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Hero", "Level"], 0x7370, Kind::I32, 1, 999);
    b.ranged(&["Hero", "Experience towards next level"], 0x7390, Kind::I32, 0, 99_999_999);
    b.ranged(&["Hero", "Gold"], 0x7394, Kind::I32, 0, 99_999_999);
    for (name, i) in FORCE {
        b.ranged(&["Force", name], 0x73B4 + 4 * i, Kind::I32, 0, 99_999);
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx(
        "Close the game first. The save's two checksums are written again when you save. Level only changes the number the game shows; the stat points a level gives are not added.",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn save() -> Vec<u8> {
        let mut d = vec![0u8; 0x7500];
        d[..8].copy_from_slice(MAGIC);
        d[0x7370..0x7374].copy_from_slice(&41i32.to_le_bytes());
        d[0x7394..0x7398].copy_from_slice(&20325i32.to_le_bytes());
        d[0x73B8..0x73BC].copy_from_slice(&3175i32.to_le_bytes());
        let n = d.len();
        let (l, r) = sums(&d).unwrap();
        d[n - 8..n - 4].copy_from_slice(&l.to_le_bytes());
        d[n - 4..].copy_from_slice(&r.to_le_bytes());
        d
    }

    #[test]
    fn reads_edits_and_fixes_both_checksums() {
        let d = save();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Hero/Gold"), Some(&Value::from(20325)));
        assert_eq!(doc.view.pointer("/Force/Earth"), Some(&Value::from(3175)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Hero/Gold", Value::from(99999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[0x7394..0x7398], &99999i32.to_le_bytes());
        assert!(looks(&out), "checksums rewritten");
        assert_ne!(out, d);
    }

    #[test]
    fn a_wrong_checksum_is_not_this_game() {
        let mut d = save();
        d[0x100] ^= 1;
        assert!(!looks(&d));
    }
}
