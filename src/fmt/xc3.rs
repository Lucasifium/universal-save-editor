//! Xenoblade Chronicles 3 (Nintendo Switch) save files (`bf3game*.sav`, about 1.8 MB): a plain,
//! unencrypted binary with the magic `6A FA 68 B3` and save version 10, no checksum.
//!
//! Offsets after roccodev/recordkeeper (library LGPL-3.0, see THIRD_PARTY_NOTICES.md): play time
//! at 0x10, gold at 0x20, 64 character records of 4444 bytes from 0xe3a0 (level, experience,
//! bonus experience) and the item slots (16 bytes each) from 0x53ca0: cylinders, gems,
//! collectibles. Item names live in the game's data files, so items are shown by id.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Xenoblade Chronicles 3 save";
const MAGIC: [u8; 4] = [0x6a, 0xfa, 0x68, 0xb3];
const VERSION: u8 = 10;
const CHARS: usize = 0xe3a0;
const CHAR_SIZE: usize = 4444;
const CHAR_MAX: usize = 64;
const INV_SLOTS: usize = 0x53ca0;
const CATEGORIES: [(&str, usize); 3] = [("Ether cylinders", 16), ("Gem crystals", 300), ("Collectibles", 1500)];

pub fn looks(d: &[u8]) -> bool {
    d.len() > 0x54000 && d[..4] == MAGIC && d[4] == VERSION
}

fn le32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Xenoblade Chronicles 3 save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Party", "Play time (seconds)"], 0x10, Kind::U32, 0, 359_999_999);
    b.ranged(&["Party", "Gold"], 0x20, Kind::U32, 0, 99_999_999);
    for c in 0..CHAR_MAX {
        let at = CHARS + c * CHAR_SIZE;
        let level = le32(d, at);
        if level == 0 || level > 99 {
            continue;
        }
        let name = format!("Character {}", c + 1);
        b.ranged(&["Characters", &name, "Level"], at, Kind::U32, 1, 99);
        b.ranged(&["Characters", &name, "Experience"], at + 4, Kind::U32, 0, 99_999_999);
        b.ranged(&["Characters", &name, "Bonus experience"], at + 8, Kind::U32, 0, 99_999_999);
    }
    let mut base = INV_SLOTS;
    for (title, n) in CATEGORIES {
        for i in 0..n {
            let at = base + i * 16;
            if d.get(at + 0xe).is_none_or(|f| f & 1 == 0) {
                continue;
            }
            let id = u16::from_le_bytes([d[at], d[at + 1]]);
            b.ranged(&[title, &format!("Item {id} (slot {})", i + 1), "Amount"], at + 0xc, Kind::U16, 0, 99);
        }
        base += n * 16;
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the game first. Item names are not stored in the save, so items show by id; amounts above 99 are not allowed because the game caps stacks there.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 0x60000];
        d[..4].copy_from_slice(&MAGIC);
        d[4] = VERSION;
        d[0x20..0x24].copy_from_slice(&1000u32.to_le_bytes());
        d[CHARS..CHARS + 4].copy_from_slice(&20u32.to_le_bytes());
        let col = INV_SLOTS + (16 + 300) * 16;
        d[col..col + 2].copy_from_slice(&2041u16.to_le_bytes());
        d[col + 0xc..col + 0xe].copy_from_slice(&5u16.to_le_bytes());
        d[col + 0xe] = 1;
        d
    }

    #[test]
    fn edits_in_place() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Party/Gold", 5000.into()).unwrap();
        doc.set("/Characters/Character 1/Level", 30.into()).unwrap();
        doc.set("/Collectibles/Item 2041 (slot 1)/Amount", 99.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(le32(&out, 0x20), 5000);
        assert_eq!(le32(&out, CHARS), 30);
        assert_eq!(out[INV_SLOTS + 316 * 16 + 0xc], 99);
        assert!(doc.set("/Collectibles/Item 2041 (slot 1)/Amount", 100.into()).is_err());
    }
}
