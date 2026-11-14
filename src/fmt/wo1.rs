//! Warriors Orochi (the original 2008 PC release, `save.dat`, exactly 0x24180 bytes): a plain
//! little-endian save with revision 2 at offset 4 and an additive checksum, the sum of every
//! byte before it, in the dword at 0x24160. Offsets follow the Universal Koei Tecmo Save Editor
//! by Sanrenan (MIT), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Warriors Orochi save";
const SIZE: usize = 0x24180;
const CK: usize = 0x24160;

fn sum(d: &[u8]) -> u32 {
    d[..CK].iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b)))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() == SIZE {
        let s = sum(d);
        d[CK..CK + 4].copy_from_slice(&s.to_le_bytes());
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && u16::from_le_bytes([d[4], d[5]]) == 2 && u32::from_le_bytes([d[CK], d[CK + 1], d[CK + 2], d[CK + 3]]) == sum(d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Warriors Orochi save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Army", "Growth Points"], 0x40C8, Kind::U32, 0, 99_999);
    for i in 0..79usize {
        let at = 0xC + i * 0xC8;
        let n = format!("Officer {:02}", i + 1);
        b.info(&["Officers", &n, "Level"], i64::from(d[at]) + 1);
        for (k, (stat, hi)) in [("Life", 480), ("Musou", 240), ("Attack", 480), ("Defense", 480), ("Speed", 240)].into_iter().enumerate() {
            b.ranged(&["Officers", &n, stat], at + 4 + 2 * k, Kind::U16, 0, hi);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the game first. The additive checksum is written again. Officers are shown by their number in the save; a level-up can reset their stats.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_fixes_sum() {
        let mut d = vec![0u8; SIZE];
        d[4] = 2;
        d[0xC + 4..0xC + 6].copy_from_slice(&300u16.to_le_bytes());
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Army/Growth Points", 99_999.into()).unwrap();
        doc.set("/Officers/Officer 01/Life", 480.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(u32::from_le_bytes([out[0x40C8], out[0x40C9], out[0x40CA], out[0x40CB]]), 99_999);
    }
}
