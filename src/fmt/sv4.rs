//! RollerCoaster Tycoon (1, with Added Attractions and Loopy Landscapes) saved games (`.SV4`).
//! The whole file is run-length packed (a byte under 128 copies that many plus one bytes, a byte
//! from 129 repeats the next byte `257 - n` times) and followed by four more bytes. The packed
//! data is a fixed 2,065,676-byte park record. Field positions follow OpenRCT2's `S4` structure
//! (GPL-3.0; see THIRD_PARTY_NOTICES.md). Money is kept in tenths of a currency unit.
//!
//! The four bytes after the packed data are not the byte sum OpenRCT2 writes (none of 85 public
//! saves matches it), so they are kept as they were.

use super::fields::{Builder, Checksum, Kind};
use super::sv6::{rle_decode, rle_encode};
use crate::tree::TreeDoc;
use std::sync::Arc;

pub const KIND: &str = "RollerCoaster Tycoon save";
const SIZE: usize = 2_065_676;
const CASH: usize = 0x19_8834;
const LOAN: usize = 0x19_8838;
const FEE: usize = 0x19_8840;
const MAX_LOAN: usize = 0x19_9548;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    tail: [u8; 4],
    original: Arc<Vec<u8>>,
    file: Arc<Vec<u8>>,
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Cheap test before any unpacking.
pub fn maybe(d: &[u8]) -> bool {
    d.len() > 1_000 && d.len() < 4_000_000
}

fn plausible(inner: &[u8]) -> bool {
    inner.len() == SIZE
        && matches!(i32_at(inner, CASH), Some(c) if c.abs() < 1_000_000_000)
        && matches!(i32_at(inner, LOAN), Some(l) if (0..=1_000_000_000).contains(&l))
        && matches!(inner.get(FEE..FEE + 2), Some(f) if u16::from_le_bytes([f[0], f[1]]) <= 3_000)
}

pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !maybe(d) {
        return None;
    }
    let body = d.len() - 4;
    let inner = rle_decode(&d[..body])?;
    if !plausible(&inner) {
        return None;
    }
    Some((Packed { tail: d[body..].try_into().ok()?, original: Arc::new(inner.clone()), file: Arc::new(d.to_vec()) }, inner))
}

pub fn close(p: &Packed, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() != SIZE {
        return Err("the park data has the wrong size".into());
    }
    if data == p.original.as_slice() {
        return Ok((*p.file).clone());
    }
    let mut out = rle_encode(data);
    out.extend_from_slice(&p.tail);
    Ok(out)
}

pub fn looks(d: &[u8]) -> bool {
    plausible(d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !plausible(d) {
        return Err("not a RollerCoaster Tycoon save".into());
    }
    let mut b = Builder::new(d);
    let money = |b: &mut Builder, name: &str, at: usize, hi: i64| {
        b.ranged(&["Park", name], at, Kind::TenthsI32, -100_000, hi);
    };
    money(&mut b, "Cash", CASH, 200_000_000);
    money(&mut b, "Bank loan", LOAN, 200_000_000);
    money(&mut b, "Maximum loan", MAX_LOAN, 200_000_000);
    b.ranged(&["Park", "Entrance fee (tenths of a unit)"], FEE, Kind::I16, 0, 3_000);
    let note = crate::i18n::tx("Close the game first. Cash, loan and fees are in whole currency units with one decimal (the game keeps tenths). A scenario you play for a score may not record it after its cash was changed.");
    Ok(b.finish(KIND, Checksum::None).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn park() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        d[CASH..CASH + 4].copy_from_slice(&176_226i32.to_le_bytes());
        d[LOAN..LOAN + 4].copy_from_slice(&150_000i32.to_le_bytes());
        d[FEE..FEE + 2].copy_from_slice(&400u16.to_le_bytes());
        d[MAX_LOAN..MAX_LOAN + 4].copy_from_slice(&200_000i32.to_le_bytes());
        d.iter_mut().skip(100).take(3000).enumerate().for_each(|(i, b)| *b = (i % 251) as u8);
        d
    }

    fn file(inner: &[u8]) -> Vec<u8> {
        let mut f = rle_encode(inner);
        f.extend_from_slice(&[0x6a, 0xbe, 0xc9, 0x04]);
        f
    }

    #[test]
    fn reads_edits_and_keeps_the_tail() {
        let f = file(&park());
        let (p, inner) = open(&f).unwrap();
        let mut doc = parse(&inner).unwrap();
        assert_eq!(doc.view.pointer("/Park/Cash"), Some(&serde_json::json!(17622.6)));
        assert_eq!(doc.view.pointer("/Park/Maximum loan"), Some(&serde_json::json!(20000.0)));
        assert_eq!(close(&p, &doc.to_bytes().unwrap()).unwrap(), f);
        doc.set("/Park/Cash", serde_json::json!(99999.5)).unwrap();
        let out = close(&p, &doc.to_bytes().unwrap()).unwrap();
        assert_eq!(&out[out.len() - 4..], &[0x6a, 0xbe, 0xc9, 0x04]);
        let (_, again) = open(&out).unwrap();
        assert_eq!(i32_at(&again, CASH), Some(999_995));
        assert_eq!(i32_at(&again, LOAN), Some(150_000));
    }

    #[test]
    fn other_files_are_not_a_park() {
        assert!(open(&vec![0u8; 6_000]).is_none());
        assert!(!looks(&[0u8; 100]));
    }

    /// `SV4_SAVE=<file.SV4> cargo test --lib sv4 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_save_round_trips() {
        let Ok(p) = std::env::var("SV4_SAVE") else { return };
        let f = std::fs::read(p).unwrap();
        let (pk, inner) = open(&f).unwrap();
        let doc = parse(&inner).unwrap();
        println!("{}", doc.view);
        assert_eq!(close(&pk, &doc.to_bytes().unwrap()).unwrap(), f);
    }
}
