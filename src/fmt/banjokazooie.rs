//! Banjo-Kazooie (Nintendo 64) EEPROM saves (`.eep`, 2048 bytes, or an emulator's combined
//! `.srm`): four 0x78-byte records (three save slots and a spare; byte 1 says which slot a
//! record is) and the Stop 'n' Swop block at 0x1e0. Each ends in a Rare 32-bit sum.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, bk};
use super::rare;
use crate::tree::TreeDoc;

pub const KIND: &str = "Banjo-Kazooie save";
const REC: usize = 0x78;
const SWOP: usize = 0x1e0;

fn stored(d: &[u8], from: usize, to: usize) -> [u8; 4] {
    ((rare::sums(&d[from..to]).1 & 0xffff_ffff) as u32).to_be_bytes()
}

pub fn looks(d: &[u8]) -> bool {
    matches!(d.len(), 2048 | 296_960) && d[SWOP + 0x1c..SWOP + 0x20] == stored(d, SWOP, SWOP + 0x1c)
}

/// The record holding save slot `slot` (0 to 2).
fn record(d: &[u8], slot: usize) -> Option<usize> {
    (0..4).map(|i| i * REC).find(|&s| {
        let id = match d[s + 1] {
            2 => 3,
            3 => 2,
            x => x,
        };
        id as usize == slot + 1
    })
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Banjo-Kazooie save".into());
    }
    let mut b = Builder::new(d);
    let mut used = 0;
    for slot in 0..3 {
        let Some(s) = record(d, slot) else { continue };
        if d[s] == 0 || d[s] == 0xff {
            continue;
        }
        used += 1;
        add(&mut b, &[&format!("Slot {}", slot + 1)], s, bk::SLOT);
    }
    add(&mut b, &[], 0, bk::SWOP);
    b.info(&["Info", "Slots in use"], used);
    if used == 0 {
        return Err("no Banjo-Kazooie slot has been started".into());
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note("Close the emulator first. Times are in seconds. The game keeps its own totals of jiggies and notes in the flags, so change the flags and the counts together."))
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    let mut recs: Vec<usize> = touched.iter().filter(|&&t| t < SWOP).map(|t| t / REC * REC).collect();
    recs.sort_unstable();
    recs.dedup();
    for s in recs {
        let v = stored(d, s, s + 0x74);
        d[s + 0x74..s + 0x78].copy_from_slice(&v);
    }
    if touched.iter().any(|&t| (SWOP..SWOP + 0x1c).contains(&t)) {
        let v = stored(d, SWOP, SWOP + 0x1c);
        d[SWOP + 0x1c..SWOP + 0x20].copy_from_slice(&v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_counts_flags_and_sums() {
        let mut d = vec![0u8; 2048];
        d[1] = 2;
        d[REC + 1] = 3;
        d[2 * REC] = 0x11;
        d[2 * REC + 1] = 1;
        fix(&mut d, &[0, REC, 2 * REC, 3 * REC, SWOP]);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/General/Jiggies", 60.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let s = 2 * REC;
        assert_eq!(out[s + 0x69], 60);
        assert_eq!(out[s + 0x74..s + 0x78], stored(&out, s, s + 0x74));
        assert!(looks(&out));
    }
}
