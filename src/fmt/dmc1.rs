//! Devil May Cry HD Collection, Devil May Cry 1 (`dmc1.sav`, 24,420 bytes): ten slots of 2,416
//! bytes. A used slot starts with the bytes 01 00 00. Nothing is checksummed.
//!
//! Where the play time, red orbs, blue orbs and yellow orbs are kept is as published by
//! dmcsaveeditor (joaovitorbf, MIT).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Devil May Cry 1 save";
const SLOT: usize = 2416;
const SIZE: usize = 10 * SLOT + 260;

fn used(d: &[u8], i: usize) -> bool {
    d[i * SLOT..i * SLOT + 3] == [1, 0, 0]
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && (0..10).any(|i| used(d, i)) && (0..10).all(|i| !used(d, i) || (d[i * SLOT + 1568..i * SLOT + 1570] != [0xFF, 0xFF]))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Devil May Cry 1 save".into());
    }
    let mut b = Builder::new(d);
    for i in 0..10 {
        if !used(d, i) {
            continue;
        }
        let name = format!("Slot {}", i + 1);
        let o = i * SLOT;
        b.ranged(&[&name, "Red orbs"], o + 1588, Kind::U32, 0, 999_999_999);
        b.ranged(&[&name, "Yellow orbs"], o + 1568, Kind::U16, 0, 999);
        b.ranged(&[&name, "Blue orbs"], o + 1592, Kind::U8, 0, 6);
        b.ranged(&[&name, "Play time (frames, 60 per second)"], o + 44, Kind::U32, 0, 4_000_000_000);
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Save slots that are empty are left out."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_edits_a_slot() {
        let mut d = vec![0u8; SIZE];
        d[0] = 1;
        d[1568..1570].copy_from_slice(&999u16.to_le_bytes());
        d[1588..1592].copy_from_slice(&1234u32.to_le_bytes());
        assert!(looks(&d));
        let t = parse(&d).unwrap();
        assert_eq!(t.view["Slot 1"]["Red orbs"], 1234);
        assert_eq!(t.view["Slot 1"]["Yellow orbs"], 999);
    }
}
