//! Dragon Ball Xenoverse (the first game, `dbxv.sav`): a `#SAV` file of 210,816 bytes. The
//! eight custom-character slots are 880 bytes each from offset 199,404.
//!
//! The layout follows the Xenoverse 2 tools' reader of this file (LazyBone152, MIT).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Dragon Ball Xenoverse save";
const SIZE: usize = 210_816;
const CAC: usize = 199_404;
const CAC_SIZE: usize = 880;

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && d.starts_with(b"#SAV")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Dragon Ball Xenoverse save".into());
    }
    let mut b = Builder::new(d);
    for i in 0..8 {
        let at = CAC + i * CAC_SIZE;
        let raw = &d[at + 36..at + 100];
        let end = raw.iter().position(|c| *c == 0).unwrap_or(raw.len());
        let name = String::from_utf8_lossy(&raw[..end]).into_owned();
        if name.is_empty() {
            continue;
        }
        let slot = format!("Slot {} - {}", i + 1, name);
        b.ranged(&["Characters", &slot, "Level"], at + 140, Kind::I32, 1, 99);
        b.ranged(&["Characters", &slot, "Experience"], at + 144, Kind::I32, 0, 99_999_999);
        b.ranged(&["Characters", &slot, "Attribute points"], at + 148, Kind::I32, 0, 9_999);
        for (k, s) in ["Health", "Ki", "Basic attack", "Strike supers", "Ki blast supers", "Stamina"].iter().enumerate() {
            b.ranged(&["Characters", &slot, s], at + 152 + 4 * k, Kind::I32, 0, 100);
        }
    }
    if b.is_empty() {
        return Err("no custom character".into());
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("Each custom character's level, experience, attribute points and the six attributes (up to 100). The save has no checksum we know of.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_a_character() {
        let mut d = vec![0u8; SIZE];
        d[..4].copy_from_slice(b"#SAV");
        d[CAC + 36..CAC + 40].copy_from_slice(b"Goku");
        d[CAC + 140..CAC + 144].copy_from_slice(&50i32.to_le_bytes());
        let mut t = parse(&d).unwrap();
        t.set("/Characters/Slot 1 - Goku/Level", serde_json::json!(99)).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(i32::from_le_bytes(out[CAC + 140..CAC + 144].try_into().unwrap()), 99);
    }
}
