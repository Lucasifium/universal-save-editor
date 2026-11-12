//! The older TT Games LEGO saves on PC, with the stud counter.
//!
//! * LEGO Star Wars: The Complete Saga, LEGO Batman: The Videogame and LEGO Indiana Jones:
//!   The Original Adventures and 2 (`SaveGameN.<game>_SavedGame`): an `HMGR` header of 0x2028 bytes
//!   (title, thumbnail), then the game's save block, then a u32 checksum and a u32 with the
//!   completion points. The checksum is `0x5c0999 + the sum of every little-endian u32` from
//!   0x2028 up to it (the same function the Android port of The Complete Saga ships). The stud
//!   counter is the u32 that sits right before the u16 completion points.
//! * LEGO Star Wars: The Video Game (2005) `LSW.BIN` (532 bytes): the stud counter is the u32
//!   at 0x204, the last u32 is the plain sum of all the words before it, and the game keeps
//!   the same stud count in `CODE.BIN` beside it, which is rewritten too.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::path::{Path, PathBuf};

pub const KIND: &str = "LEGO classic save";
const HEADER: usize = 0x2028;
const SEED: u32 = 0x5c0999;
const MAX_STUDS: i64 = 4_000_000_000;
const LSW1_LEN: usize = 532;
const LSW1_STUDS: usize = 0x204;
const LSW1_MAX_STUDS: i64 = 9_999_999;

/// (title, file size, offset of the studs from the start of the file, bytes from the studs
/// to the completion points)
const GAMES: [(&str, usize, usize, usize); 4] = [
    ("LEGO Star Wars: The Complete Saga", 40572, HEADER + 0x7c14, 4),
    ("LEGO Batman: The Videogame", 40520, HEADER + 0x7bd0, 4),
    ("LEGO Indiana Jones: The Original Adventures", 40880, HEADER + 0x7d38, 4),
    ("LEGO Indiana Jones 2: The Adventure Continues", 537600, 537600 - 8 - 394240 - 8, 8),
];

const NEWER_MAX_STUDS: i64 = 999_999_999_999;

/// (title, file size, offset of the studs from the end of the header, copies, bytes between
/// copies)
const NEWER: [(&str, usize, usize, usize, usize); 5] = [
    ("LEGO Harry Potter: Years 1-4", 397364, 0x1a84, 1, 0),
    ("LEGO Harry Potter: Years 5-7", 353936, 0x1a80, 1, 0),
    ("LEGO Star Wars III: The Clone Wars", 110648, 0x6240, 1, 0),
    ("LEGO Pirates of the Caribbean", 315448, 0x80b0, 5, 0xdd50),
    ("LEGO Batman 2: DC Super Heroes", 97336, 0xa958, 1, 0),
];

fn words_sum(d: &[u8]) -> u32 {
    d.chunks_exact(4).fold(0u32, |a, c| a.wrapping_add(u32::from_le_bytes(c.try_into().unwrap())))
}

fn u32at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

fn hmgr_game(d: &[u8]) -> Option<(&'static str, usize)> {
    if d.len() < HEADER + 16 || !d.starts_with(b"HMGR") || u32at(d, 8) as usize != HEADER {
        return None;
    }
    let g = GAMES.iter().find(|g| g.1 == d.len())?;
    let n = d.len();
    let sum = words_sum(&d[HEADER..n - 8]).wrapping_add(SEED);
    // The checksum matches, and the completion points follow the stud counter.
    let completion = u32at(d, n - 4);
    let at = g.2;
    (u32at(d, n - 8) == sum && u32::from(u16::from_le_bytes([d[at + g.3], d[at + g.3 + 1]])) == completion).then_some((g.0, at))
}

fn newer_game(d: &[u8]) -> Option<&'static (&'static str, usize, usize, usize, usize)> {
    if d.len() < HEADER + 16 || !d.starts_with(b"HMGR") || u32at(d, 8) as usize != HEADER {
        return None;
    }
    // The title in the header is UTF-16 and starts with "LEGO".
    if d[0x28..0x30] != *b"L\0E\0G\0O\0" {
        return None;
    }
    let g = NEWER.iter().find(|g| g.1 == d.len())?;
    // Every copy holds the same wallet.
    let first = &d[HEADER + g.2..HEADER + g.2 + 8];
    (1..g.3).all(|k| &d[HEADER + g.2 + k * g.4..HEADER + g.2 + k * g.4 + 8] == first).then_some(g)
}

fn is_lsw1(d: &[u8]) -> bool {
    d.len() == LSW1_LEN && u32at(d, LSW1_LEN - 4) == words_sum(&d[..LSW1_LEN - 4]) && u32at(d, LSW1_STUDS) <= LSW1_MAX_STUDS as u32
}

pub fn looks(d: &[u8]) -> bool {
    hmgr_game(d).is_some() || newer_game(d).is_some() || is_lsw1(d)
}

fn fix_hmgr(d: &mut Vec<u8>) {
    let n = d.len();
    let sum = words_sum(&d[HEADER..n - 8]).wrapping_add(SEED);
    d[n - 8..n - 4].copy_from_slice(&sum.to_le_bytes());
}

fn fix_lsw1(d: &mut Vec<u8>) {
    let n = d.len();
    let sum = words_sum(&d[..n - 4]);
    d[n - 4..].copy_from_slice(&sum.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut b = Builder::new(d);
    if let Some((title, at)) = hmgr_game(d) {
        b.ranged(&["Studs"], at, Kind::U32, 0, MAX_STUDS);
        b.info(&["Game"], title);
        b.info(&["Completion points"], u32at(d, d.len() - 4));
        return Ok(b.finish(KIND, Checksum::Custom(fix_hmgr)).with_note(
            "Close the game first. The checksum is rewritten.",
        ));
    }
    if let Some(&(title, _, at, copies, stride)) = newer_game(d) {
        b.ranged(&["Studs"], HEADER + at, Kind::U64, 0, NEWER_MAX_STUDS);
        for k in 1..copies {
            b.mirror(&["Studs"], HEADER + at + k * stride);
        }
        b.info(&["Game"], title);
        return Ok(b.finish(KIND, Checksum::None).with_note("Close the game first."));
    }
    if is_lsw1(d) {
        b.ranged(&["Studs"], LSW1_STUDS, Kind::U32, 0, LSW1_MAX_STUDS);
        b.info(&["Game"], "LEGO Star Wars: The Video Game");
        return Ok(b.finish(KIND, Checksum::Custom(fix_lsw1)).with_note(
            "Close the game first. The checksum is rewritten, and CODE.BIN beside LSW.BIN is rewritten with the stud count.",
        ));
    }
    Err("not an older LEGO save".into())
}

/// After `LSW.BIN` is written: `CODE.BIN` beside it holds the same stud count.
pub fn companions(path: &Path, bytes: &[u8]) -> Vec<(PathBuf, Vec<u8>)> {
    if !is_lsw1(bytes) {
        return vec![];
    }
    let Some(file) = super::kotor::sibling(Some(path), "CODE.BIN") else { return vec![] };
    let new = bytes[LSW1_STUDS..LSW1_STUDS + 4].to_vec();
    match std::fs::read(&file) {
        Ok(old) if old != new => vec![(file, new)],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn hmgr(len: usize, at: usize, gap: usize, studs: u32) -> Vec<u8> {
        let mut d = vec![0u8; len];
        d[..4].copy_from_slice(b"HMGR");
        d[8..12].copy_from_slice(&(HEADER as u32).to_le_bytes());
        d[HEADER + 100] = 7;
        d[at..at + 4].copy_from_slice(&studs.to_le_bytes());
        d[at + gap..at + gap + 2].copy_from_slice(&0x330u16.to_le_bytes());
        let n = len;
        d[n - 4..].copy_from_slice(&0x330u32.to_le_bytes());
        fix_hmgr(&mut d);
        d
    }

    #[test]
    fn hmgr_studs_and_checksum() {
        for (_, len, at, gap) in GAMES {
            let d = hmgr(len, at, gap, 123_456);
            assert!(looks(&d));
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.view.pointer("/Studs"), Some(&Value::from(123_456)));
            assert_eq!(doc.to_bytes().unwrap(), d);
            doc.set("/Studs", Value::from(4_000_000_000u64)).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(u32at(&out, at), 4_000_000_000);
            assert_eq!(u32at(&out, len - 8), words_sum(&out[HEADER..len - 8]).wrapping_add(SEED));
            assert!(doc.set("/Studs", Value::from(5_000_000_000u64)).is_err());
        }
        let mut bad = hmgr(GAMES[0].1, GAMES[0].2, 4, 5);
        bad[HEADER + 100] ^= 1;
        assert!(!looks(&bad));
    }

    #[test]
    fn newer_studs_every_copy() {
        for &(title, len, at, copies, stride) in &NEWER {
            let mut d = vec![0u8; len];
            d[..4].copy_from_slice(b"HMGR");
            d[8..12].copy_from_slice(&(HEADER as u32).to_le_bytes());
            for (i, c) in "LEGO".encode_utf16().enumerate() {
                d[0x28 + 2 * i..0x2a + 2 * i].copy_from_slice(&c.to_le_bytes());
            }
            for k in 0..copies {
                let p = HEADER + at + k * stride;
                d[p..p + 8].copy_from_slice(&5_978_596_090u64.to_le_bytes());
            }
            assert!(looks(&d), "{title}");
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.view.pointer("/Studs"), Some(&Value::from(5_978_596_090u64)));
            assert_eq!(doc.view.pointer("/Game"), Some(&Value::from(title)));
            assert_eq!(doc.to_bytes().unwrap(), d);
            doc.set("/Studs", Value::from(999_999_999_999u64)).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), len);
            for k in 0..copies {
                let p = HEADER + at + k * stride;
                assert_eq!(out[p..p + 8], 999_999_999_999u64.to_le_bytes());
            }
            assert!(doc.set("/Studs", Value::from(1_000_000_000_000u64)).is_err());
            if copies > 1 {
                d[HEADER + at + stride] ^= 1;
                assert!(!looks(&d));
            }
        }
    }

    #[test]
    fn lsw1_studs_and_companion() {
        let mut d = vec![0u8; LSW1_LEN];
        d[0] = 0;
        d[1] = 5;
        d[LSW1_STUDS..LSW1_STUDS + 4].copy_from_slice(&1_872_940u32.to_le_bytes());
        fix_lsw1(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Studs", Value::from(9_999_999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32at(&out, LSW1_STUDS), 9_999_999);
        assert_eq!(u32at(&out, LSW1_LEN - 4), words_sum(&out[..LSW1_LEN - 4]));
        assert!(doc.set("/Studs", Value::from(10_000_000)).is_err());

        let dir = std::env::temp_dir().join(format!("lego_lsw1_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("code.bin"), 1_872_940u32.to_le_bytes()).unwrap();
        let c = companions(&dir.join("LSW.BIN"), &out);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].1, 9_999_999u32.to_le_bytes());
        std::fs::remove_dir_all(&dir).ok();
    }
}
