//! Donkey Kong 64 (Nintendo 64) EEPROM saves (`.eep`, 2048 bytes, or an emulator's combined
//! `.srm`): four 0x1ac-byte records (the slot number at 0x1a0, in use when bit 3 of 0x19e is
//! set), each closed by a CRC-32 of the record, then the options and Mystery menu at 0x6b0 with a
//! CRC-32 of their own at 0x6ec. The game packs its numbers across byte borders with every
//! 32-bit word stored backwards, so the fields are read from a copy with each word turned round.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, dk, swap32};
use crate::tree::TreeDoc;

pub const KIND: &str = "Donkey Kong 64 save";
const EEP: usize = 0x800;
const REC: usize = 0x1ac;
const SUM: usize = 0x1a8;
const GLOBAL: usize = 0x6b0;
const GLOBAL_SUM: usize = 0x6ec;

fn crc32(d: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in d {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

fn be(d: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

pub fn looks(d: &[u8]) -> bool {
    matches!(d.len(), 2048 | 296_960) && be(d, GLOBAL_SUM) == crc32(&d[GLOBAL..GLOBAL_SUM])
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let mut o = outer.to_vec();
    o[..EEP].copy_from_slice(&swap32(inner));
    o
}

/// The record of slot `slot` (0 to 2) in the console's byte order, if it is in use.
fn record(p: &[u8], slot: u8) -> Option<usize> {
    (0..4)
        .map(|i| i * REC)
        .filter(|&r| p[r + 0x19e] & 8 != 0 && (p[r + 0x1a0] >> 2) & 3 == slot)
        .max_by_key(|&r| (be(p, r + 0x1a0) >> 2) & 0xff_ffff)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Donkey Kong 64 save".into());
    }
    let s = swap32(&d[..EEP]);
    let mut b = Builder::new(&s);
    b.inside(d, put);
    let mut used = 0;
    for slot in 0..3u8 {
        let Some(r) = record(d, slot) else { continue };
        used += 1;
        add(&mut b, &[&format!("Slot {}", slot + 1)], r, dk::SLOT);
    }
    add(&mut b, &[], 0, dk::GLOBAL);
    b.info(&["Info", "Slots in use"], used);
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note(crate::i18n::tr("Close the emulator first. Times are in seconds. Each Kong's Golden Banana count follows the Golden Bananas ticked. The high scores and sound options are left out: the Japanese release keeps them one bit further on. Every edited slot's CRC and the Mystery menu's CRC are made again.")))
}

fn fix(s: &mut Vec<u8>, touched: &[usize]) {
    let mut p = swap32(s);
    for slot in 0..3u8 {
        let Some(r) = record(&p, slot) else { continue };
        if !touched.iter().any(|&t| (r..r + SUM).contains(&t)) {
            continue;
        }
        for &(at, shift, flags) in dk::GOLDEN {
            if !flags.iter().any(|&(f, _)| touched.contains(&(r + f))) {
                continue;
            }
            let n = flags.iter().filter(|&&(f, bit)| s[r + f] >> bit & 1 == 1).count() as u16;
            let v = u16::from_le_bytes([s[r + at], s[r + at + 1]]);
            let v = (v & !(7 << shift)) | (n << shift);
            s[r + at..r + at + 2].copy_from_slice(&v.to_le_bytes());
        }
        p = swap32(s);
        let c = crc32(&p[r..r + SUM]);
        p[r + SUM..r + REC].copy_from_slice(&c.to_be_bytes());
        *s = swap32(&p);
    }
    if touched.iter().any(|&t| (GLOBAL..GLOBAL_SUM).contains(&t)) {
        let mut p = swap32(s);
        let c = crc32(&p[GLOBAL..GLOBAL_SUM]);
        p[GLOBAL_SUM..GLOBAL_SUM + 4].copy_from_slice(&c.to_be_bytes());
        *s = swap32(&p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut p = vec![0u8; EEP];
        let r = REC;
        p[r + 0x19e] = 8;
        p[r + 0x1a0] = 1 << 2;
        let c = crc32(&p[r..r + SUM]);
        p[r + SUM..r + REC].copy_from_slice(&c.to_be_bytes());
        let c = crc32(&p[GLOBAL..GLOBAL_SUM]);
        p[GLOBAL_SUM..GLOBAL_SUM + 4].copy_from_slice(&c.to_be_bytes());
        p
    }

    #[test]
    fn edits_values_across_bytes_and_keeps_the_crcs() {
        let p = sample();
        assert!(looks(&p));
        let mut doc = parse(&p).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), p);
        doc.set("/Slot 2/General/Playtime (seconds)", 3600.into()).unwrap();
        doc.set("/Slot 2/Characters/Donkey Kong/Banana Coins", 50.into()).unwrap();
        let gb = dk::GOLDEN[0];
        let (f, bit) = gb.2[0];
        let path = dk::SLOT.iter().find(|x| x.at == f && x.shift == bit && x.width == 1).unwrap().path.join("/");
        doc.set(&format!("/Slot 2/{path}"), "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let r = REC;
        // The game's own reading of a number that runs across words (after the template's getMergedInt).
        let merged = |o: usize, n: usize| -> u32 {
            let mut v = 0u32;
            let mut at = o;
            for i in 0..n {
                v = (v << 8) | out[at] as u32;
                at = if (o + i + 1) % 4 == 0 { o - (o % 4) - 4 } else { at + 1 };
            }
            v
        };
        assert_eq!((merged(r + 0x1a3, 4) >> 4) & 0x3f_ffff, 3600);
        assert_eq!((merged(r + 0x14f, 2) >> 1) & 0xff, 50);
        assert_eq!(be(&out, r + SUM), crc32(&out[r..r + SUM]));
        let s = swap32(&out);
        assert_eq!(u16::from_le_bytes([s[r + gb.0], s[r + gb.0 + 1]]) >> gb.1 & 7, 1, "the count follows the flag");
        assert!(looks(&out));
    }
}
