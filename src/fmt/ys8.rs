//! Ys VIII: Lacrimosa of DANA (PC `savedata.dat`, 819,200 bytes, the same layout as the Switch
//! version). The last eight bytes hold two checksums over every other 32-bit word: their sum
//! and the sum of their complements. Layout facts follow turtle-insect/YS8 (GPL-3.0).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Ys VIII save";
const SIZE: usize = 819_200;
const ITEMS: usize = 0xB4EC;
const ITEM_SLOTS: usize = 1024;
const EQUIP: usize = 0x7D0;
const EQUIP_STRIDE: usize = 412;

fn sums(d: &[u8]) -> (u32, u32) {
    let (mut a, mut b) = (0u32, 0u32);
    for c in d[..d.len() - 8].chunks_exact(4) {
        let w = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        a = a.wrapping_add(w);
        b = b.wrapping_add(!w);
    }
    (a, b)
}

pub fn looks(d: &[u8]) -> bool {
    if d.len() != SIZE {
        return false;
    }
    let (a, b) = sums(d);
    let n = d.len();
    d[n - 8..n - 4] == a.to_le_bytes() && d[n - 4..] == b.to_le_bytes()
}

fn fix(d: &mut Vec<u8>) {
    let (a, b) = sums(d);
    let n = d.len();
    d[n - 8..n - 4].copy_from_slice(&a.to_le_bytes());
    d[n - 4..].copy_from_slice(&b.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Ys VIII save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Play time (frames, 30 per second)"], 0x2844, Kind::U32, 0, u32::MAX as i64);
    for (i, who) in ["Adol", "Laxia"].iter().enumerate() {
        let at = EQUIP + i * EQUIP_STRIDE;
        for (k, slot) in ["Weapon", "Body", "Arm", "Accessory 1", "Accessory 2"].iter().enumerate() {
            b.ranged(&["Equipment (item id)", who, slot], at + 2 * k, Kind::U16, 0, 65_535);
        }
    }
    for id in 0..ITEM_SLOTS {
        let at = ITEMS + 2 * id;
        if u16::from_le_bytes([d[at], d[at + 1]]) > 0 {
            b.ranged(&["Items", &id.to_string()], at, Kind::U16, 0, 999);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(
        "Play time, Adol's and Laxia's equipment and every item you hold, by item id (up to 999 each). Both checksums are updated when you save.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_keeps_checksums() {
        let mut d = vec![0u8; SIZE];
        d[ITEMS + 2 * 316..ITEMS + 2 * 316 + 2].copy_from_slice(&20u16.to_le_bytes());
        fix(&mut d);
        assert!(looks(&d));
        let mut t = parse(&d).unwrap();
        t.set("/Items/316", serde_json::json!(999)).unwrap();
        let out = t.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(u16::from_le_bytes([out[ITEMS + 632], out[ITEMS + 633]]), 999);
    }
}
