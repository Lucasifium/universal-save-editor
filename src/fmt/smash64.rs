//! Super Smash Bros. (Nintendo 64) SRAM saves (`.sra`, 32 KiB, or an emulator's combined
//! `.srm`; the words may be swapped as Project64 writes them): one block of 0x5ec bytes with
//! the unlocks, the 1P and 2P records and the options, ending in a 32-bit weighted byte sum.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, smash64, sram, sram_put};
use crate::tree::TreeDoc;

pub const KIND: &str = "Super Smash Bros. save";

fn sum_of(s: &[u8]) -> u32 {
    s[..0x5e8].iter().enumerate().fold(0u32, |a, (i, &b)| a.wrapping_add(u32::from(b).wrapping_mul(i as u32 + 1)))
}

fn valid(s: &[u8]) -> bool {
    let sum = sum_of(s);
    sum != 0 && s[0x5e8..0x5ec] == sum.to_be_bytes()
}

pub fn looks(d: &[u8]) -> bool {
    sram(d, valid).is_some()
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    sram_put(outer, inner, valid)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (_, inner) = sram(d, valid).ok_or("not a Super Smash Bros. save")?;
    let mut b = Builder::new(&inner);
    b.inside(d, put);
    add(&mut b, &[], 0, smash64::SLOT);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Times are in 1/60 s (3600 is one minute)."))
}

fn fix(d: &mut Vec<u8>) {
    let sum = sum_of(d).to_be_bytes();
    d[0x5e8..0x5ec].copy_from_slice(&sum);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::n64data::swap32;

    #[test]
    fn edits_a_swapped_save_and_its_sum() {
        let mut s = vec![0u8; 0x8000];
        s[0x10] = 1;
        fix(&mut s);
        assert!(valid(&s));
        let file = swap32(&s);
        assert!(looks(&file));
        let mut doc = parse(&file).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), file);
        doc.set("/2P Records/Mario/TKOs", 42.into()).unwrap();
        let out = swap32(&doc.to_bytes().unwrap());
        assert_eq!(u16::from_be_bytes([out[0x26], out[0x27]]), 42);
        assert!(valid(&out));
    }
}
