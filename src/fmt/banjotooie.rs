//! Banjo-Tooie (Nintendo 64) EEPROM saves (`.eep`, 2048 bytes, or an emulator's combined
//! `.srm`): four 0x1c0-byte records from 0x100 ("KHJC", the slot number at +0xa), each ending
//! in a Rare 64-bit sum of the record.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, bt};
use super::rare;
use crate::tree::TreeDoc;

pub const KIND: &str = "Banjo-Tooie save";
const FIRST: usize = 0x100;
const REC: usize = 0x1c0;
const SUM: usize = 0x1b8;

fn record_ok(d: &[u8], s: usize) -> bool {
    &d[s..s + 4] == b"KHJC" && d[s + SUM..s + REC] == rare::stored(&d[s..s + SUM])
}

pub fn looks(d: &[u8]) -> bool {
    matches!(d.len(), 2048 | 296_960) && (0..4).any(|i| record_ok(d, FIRST + i * REC))
}

/// The record holding save slot `slot` (1 to 3).
fn record(d: &[u8], slot: u8, ok: fn(&[u8], usize) -> bool) -> Option<usize> {
    (0..4).map(|i| FIRST + i * REC).find(|&s| ok(d, s) && d[s + 0xa] == slot)
}

fn header_ok(d: &[u8], s: usize) -> bool {
    &d[s..s + 4] == b"KHJC"
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Banjo-Tooie save".into());
    }
    let mut b = Builder::new(d);
    let mut used = 0;
    for slot in 1..=3u8 {
        let Some(s) = record(d, slot, record_ok) else { continue };
        used += 1;
        add(&mut b, &[&format!("Slot {slot}")], s, bt::SLOT);
    }
    b.info(&["Info", "Slots in use"], used);
    if used == 0 {
        return Err("no Banjo-Tooie slot has been started".into());
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note(crate::i18n::tr("Close the emulator first. Times are in seconds. The jiggies, notes and other world flags are left out: the American release keeps them in other places than the European, Australian and Japanese ones, and the save does not say which one wrote it. Each edited slot's Rare sum is made again.")))
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    for slot in 1..=3u8 {
        let Some(s) = record(d, slot, header_ok) else { continue };
        if touched.iter().any(|&t| (s..s + SUM).contains(&t)) {
            let v = rare::stored(&d[s..s + SUM]);
            d[s + SUM..s + REC].copy_from_slice(&v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_counts_and_moves_and_keeps_the_sum() {
        let mut d = vec![0u8; 2048];
        let s = FIRST + 2 * REC;
        d[s..s + 4].copy_from_slice(b"KHJC");
        d[s + 0xa] = 1;
        fix(&mut d, &[s]);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Inventory/Eggs", 999.into()).unwrap();
        doc.set("/Slot 1/Special Moves/Advanced Moves/Glide", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u16::from_be_bytes([out[s + 0x2b], out[s + 0x2c]]), 999);
        assert_eq!(out[s + 0xa3] >> 7, 1);
        assert!(record_ok(&out, s));
    }
}
