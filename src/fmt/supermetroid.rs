//! Super Metroid (SNES) battery saves (`.srm` / `.sav`, 8 KiB): three Samus slots of 0x65c
//! bytes starting at 0x10. Each slot has a 16-bit sum of the words of its data at `slot * 2`
//! and the same sum xor 0xffff at `8 + slot * 2`; the file's last bytes (0x1ff0 and 0x1ff8)
//! repeat both tables.
//!
//! Offsets follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md); the equipment bits are the game's own.

use super::fields::{Builder, Checksum, Kind};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Super Metroid save";
const SLOT: usize = 0x65c;
const COPY: usize = 0x1ff0;

pub fn looks(d: &[u8]) -> bool {
    d.len() == 0x2000 && (&d[0x1fe0..0x1fec] == b"madadameyohn" || &d[0x1fe0..0x1fec] == b"supermetroid")
}

fn used(d: &[u8], n: usize) -> bool {
    // A slot that has been started holds its maximum energy.
    u16::from_le_bytes([d[n * SLOT + 0x32], d[n * SLOT + 0x33]]) != 0
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Super Metroid save".into());
    }
    let mut b = Builder::new(d);
    let mut found = 0;
    for n in 0..3 {
        if !used(d, n) {
            continue;
        }
        found += 1;
        let s = n * SLOT;
        let slot = ["Samus A", "Samus B", "Samus C"][n];
        let num = |b: &mut Builder, path: &[&str], off: usize, hi: i64| {
            b.ranged(path, s + off, Kind::U16, 0, hi);
        };
        let bit = |b: &mut Builder, path: &[&str], off: usize, bit: u32| {
            flag(b, path, s + off, bit);
        };
        num(&mut b, &[slot, "Energy"], 0x30, 1499);
        num(&mut b, &[slot, "Energy tanks (max energy)"], 0x32, 1499);
        num(&mut b, &[slot, "Reserve energy"], 0x44, 400);
        num(&mut b, &[slot, "Reserve tanks (max reserve)"], 0x42, 400);
        num(&mut b, &[slot, "Missiles"], 0x34, 230);
        num(&mut b, &[slot, "Missiles (max)"], 0x36, 230);
        num(&mut b, &[slot, "Super missiles"], 0x38, 50);
        num(&mut b, &[slot, "Super missiles (max)"], 0x3a, 50);
        num(&mut b, &[slot, "Power bombs"], 0x3c, 50);
        num(&mut b, &[slot, "Power bombs (max)"], 0x3e, 50);
        num(&mut b, &[slot, "Play time hours"], 0x4e, 99);
        num(&mut b, &[slot, "Play time minutes"], 0x4c, 59);
        num(&mut b, &[slot, "Play time seconds"], 0x4a, 59);
        // (name, offset of the obtained byte, bit); the equipped copy is 2 bytes lower.
        for (name, off, bit_no) in [
            ("Charge beam", 0x17, 4), ("Ice beam", 0x16, 1), ("Wave beam", 0x16, 0), ("Spazer", 0x16, 2), ("Plasma beam", 0x16, 3),
            ("Varia suit", 0x12, 0), ("Gravity suit", 0x12, 5), ("Morphing ball", 0x12, 2), ("Bombs", 0x13, 4), ("Spring ball", 0x12, 1),
            ("Screw attack", 0x12, 3), ("Grappling beam", 0x13, 6), ("X-ray scope", 0x13, 7), ("Hi-jump boots", 0x13, 0),
            ("Space jump", 0x13, 1), ("Speed booster", 0x13, 5),
        ] {
            bit(&mut b, &[slot, "Obtained", name], off, bit_no);
            bit(&mut b, &[slot, "Equipped", name], off - 2, bit_no);
        }
    }
    b.info(&["Info", "Slots in use"], found);
    if found == 0 {
        return Err("no Super Metroid slot has been started".into());
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Energy and ammunition are limited to what the game can show; each slot's sums (and their copies at the end of the file) are kept right."))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..3 {
        for copy in [0, COPY] {
            let s = n * SLOT;
            let sum = d[s + 0x10..s + 0x66c].chunks(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])));
            d[copy + n * 2..copy + n * 2 + 2].copy_from_slice(&sum.to_le_bytes());
            d[copy + 8 + n * 2..copy + 10 + n * 2].copy_from_slice(&(sum ^ 0xffff).to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_fixes_sums() {
        let mut d = vec![0u8; 0x2000];
        d[0x1fe0..0x1fec].copy_from_slice(b"supermetroid");
        d[0x32..0x34].copy_from_slice(&99u16.to_le_bytes());
        d[0x30..0x32].copy_from_slice(&99u16.to_le_bytes());
        fix(&mut d);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Samus A/Missiles", 100.into()).unwrap();
        doc.set("/Samus A/Obtained/Varia suit", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u16::from_le_bytes([out[0x34], out[0x35]]), 100);
        assert_eq!(out[0x12] & 1, 1);
        let sum = out[0x10..0x66c].chunks(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])));
        assert_eq!(u16::from_le_bytes([out[0], out[1]]), sum);
        assert_eq!(u16::from_le_bytes([out[8], out[9]]), sum ^ 0xffff);
        assert_eq!(u16::from_le_bytes([out[COPY], out[COPY + 1]]), sum);
        assert_eq!(u16::from_le_bytes([out[COPY + 8], out[COPY + 9]]), sum ^ 0xffff);
    }
}
