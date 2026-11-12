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
//! * LEGO Star Wars II: The Original Trilogy `savegame.bin` (1008 bytes): the same plain sum in
//!   the last u32; the stud counter is the u32 at 0x383, right before the u16 that `code.bin`
//!   copies (which isn't the studs here, so `code.bin` is left alone).
//! * The later games of that engine (`GAME1.<game>SaveGameData`: Harry Potter Years 1-4 and
//!   5-7, Star Wars III, Pirates of the Caribbean, Batman 2, City Undercover): the same `HMGR`
//!   header, then a byte-packed save block with no checksum. The stud wallet is a u64 at a
//!   fixed place, followed by two u16 counters. Pirates keeps five copies of the block, all
//!   written. Marvel Super Heroes and The Lord of the Rings have the same file size as Batman 2,
//!   so the title in the header tells them apart.
//! * The games after those (The Hobbit, Jurassic World, Batman 3, The Force Awakens, The
//!   Incredibles, DC Super-Villains, Marvel Super Heroes 2) drop the header; see `BARE`.

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
const LSW2_LEN: usize = 1008;
const LSW2_STUDS: usize = 0x383;

/// (title, file size, offset of the studs from the start of the file, bytes from the studs
/// to the completion points)
const GAMES: [(&str, usize, usize, usize); 4] = [
    ("LEGO Star Wars: The Complete Saga", 40572, HEADER + 0x7c14, 4),
    ("LEGO Batman: The Videogame", 40520, HEADER + 0x7bd0, 4),
    ("LEGO Indiana Jones: The Original Adventures", 40880, HEADER + 0x7d38, 4),
    ("LEGO Indiana Jones 2: The Adventure Continues", 537600, 537600 - 8 - 394240 - 8, 8),
];

const NEWER_MAX_STUDS: i64 = 999_999_999_999;

/// (title, a word from the title in the header, file size, offset of the studs from the end of
/// the header, copies, bytes between copies)
type Newer = (&'static str, &'static str, usize, usize, usize, usize);
const NEWER: [Newer; 8] = [
    ("LEGO Harry Potter: Years 1-4", "Harry Potter", 397364, 0x1a84, 1, 0),
    ("LEGO Harry Potter: Years 5-7", "Harry Potter", 353936, 0x1a80, 1, 0),
    ("LEGO Star Wars III: The Clone Wars", "Star Wars", 110648, 0x6240, 1, 0),
    ("LEGO Pirates of the Caribbean", "Pirates", 315448, 0x80b0, 5, 0xdd50),
    ("LEGO Batman 2: DC Super Heroes", "Batman", 97336, 0xa958, 1, 0),
    ("LEGO City Undercover", "City Undercover", 2579512, 0xfb20, 1, 0),
    ("LEGO Marvel Super Heroes", "MARVEL", 97336, 0x4508, 1, 0),
    ("LEGO The Lord of the Rings", "Lord of the Rings", 97336, 0xaa90, 1, 0),
];

/// The games after those drop the `HMGR` header: the file starts with an 8-byte engine id.
/// The stud wallet is a u64 at a fixed place; in the tagged ones it is written twice in a row
/// between two field tags (the second one is always `6e 48 54 eb`), and the whole block is
/// saved more than once. (title, file size, engine id, offset of the (second) u64, copies,
/// bytes between copies, tagged)
type Bare = (&'static str, usize, [u8; 8], usize, usize, usize, bool);
const V1: [u8; 8] = [1, 0, 0, 0, 0x5b, 0xf2, 0x12, 0x8f];
const V2: [u8; 8] = [2, 0, 0, 0, 0x7a, 0xd8, 0x5e, 0xd2];
const V3: [u8; 8] = [3, 0, 0, 0, 0xc5, 0x8b, 0x45, 0x11];
const TAG_END: [u8; 4] = [0x6e, 0x48, 0x54, 0xeb];
const BARE: [Bare; 7] = [
    ("LEGO The Hobbit", 262164, V1, 0xd19c, 2, 0x20a64, false),
    ("LEGO Jurassic World", 163860, V1, 0x260f, 2, 0x24493, true),
    ("LEGO Batman 3: Beyond Gotham", 307220, V1, 0xf7d4, 4, 0x11833, true),
    ("LEGO Star Wars: The Force Awakens", 307224, V2, 0x8dbf, 4, 0x13a47, true),
    ("LEGO The Incredibles", 358424, V1, 0x1c35b, 2, 0x1e6df, true),
    ("LEGO DC Super-Villains", 358424, V1, 0x22cca, 2, 0x241fb, true),
    ("LEGO Marvel Super Heroes 2", 358424, V3, 0x22582, 2, 0x23a1b, true),
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

fn header_title(d: &[u8]) -> String {
    let w: Vec<u16> = d[0x28..0x228].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
    String::from_utf16_lossy(&w)
}

fn newer_game(d: &[u8]) -> Option<&'static Newer> {
    if d.len() < HEADER + 16 || !d.starts_with(b"HMGR") || u32at(d, 8) as usize != HEADER {
        return None;
    }
    // The title in the header is UTF-16 and starts with "LEGO".
    if d[0x28..0x30] != *b"L\0E\0G\0O\0" {
        return None;
    }
    let title = header_title(d);
    let g = NEWER.iter().find(|g| g.2 == d.len() && title.contains(g.1))?;
    // Every copy holds the same wallet.
    let first = &d[HEADER + g.3..HEADER + g.3 + 8];
    (1..g.4).all(|k| &d[HEADER + g.3 + k * g.5..HEADER + g.3 + k * g.5 + 8] == first).then_some(g)
}

/// Every place a bare save keeps its wallet, if all of them hold the same number.
fn bare_game(d: &[u8]) -> Option<(&'static str, Vec<usize>)> {
    if d.len() < 16 {
        return None;
    }
    BARE.iter().find_map(|&(title, len, id, at, copies, stride, tagged)| {
        if d.len() != len || d[..8] != id {
            return None;
        }
        let mut places = vec![];
        for k in 0..copies {
            let p = at + k * stride;
            if tagged {
                if d[p + 8..p + 12] != TAG_END || d[p - 8..p] != d[p..p + 8] {
                    return None;
                }
                places.push(p - 8);
            }
            places.push(p);
        }
        places.iter().all(|&p| d[p..p + 8] == d[at..at + 8]).then_some((title, places))
    })
}

fn is_lsw1(d: &[u8]) -> bool {
    d.len() == LSW1_LEN && u32at(d, LSW1_LEN - 4) == words_sum(&d[..LSW1_LEN - 4]) && u32at(d, LSW1_STUDS) <= LSW1_MAX_STUDS as u32
}

fn is_lsw2(d: &[u8]) -> bool {
    d.len() == LSW2_LEN && u32at(d, LSW2_LEN - 4) == words_sum(&d[..LSW2_LEN - 4])
}

pub fn looks(d: &[u8]) -> bool {
    hmgr_game(d).is_some() || newer_game(d).is_some() || bare_game(d).is_some() || is_lsw1(d) || is_lsw2(d)
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
    if let Some((title, places)) = bare_game(d) {
        b.ranged(&["Studs"], places[0], Kind::U64, 0, NEWER_MAX_STUDS);
        for &p in &places[1..] {
            b.mirror(&["Studs"], p);
        }
        b.info(&["Game"], title);
        return Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. The game keeps the stud wallet in several places; all of them are written."));
    }
    if let Some(&(title, _, _, at, copies, stride)) = newer_game(d) {
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
    if is_lsw2(d) {
        b.ranged(&["Studs"], LSW2_STUDS, Kind::U32, 0, MAX_STUDS);
        b.info(&["Game"], "LEGO Star Wars II: The Original Trilogy");
        return Ok(b.finish(KIND, Checksum::Custom(fix_lsw1)).with_note("Close the game first. The checksum is rewritten."));
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
        for &(title, key, len, at, copies, stride) in &NEWER {
            let mut d = vec![0u8; len];
            d[..4].copy_from_slice(b"HMGR");
            d[8..12].copy_from_slice(&(HEADER as u32).to_le_bytes());
            for (i, c) in format!("LEGO® {key}").encode_utf16().enumerate() {
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
    fn bare_studs_every_place() {
        for &(title, len, id, at, copies, stride, tagged) in &BARE {
            let mut d = vec![0u8; len];
            d[..8].copy_from_slice(&id);
            let mut places = vec![];
            for k in 0..copies {
                let p = at + k * stride;
                if tagged {
                    d[p + 8..p + 12].copy_from_slice(&TAG_END);
                    places.push(p - 8);
                }
                places.push(p);
            }
            for &p in &places {
                d[p..p + 8].copy_from_slice(&10_212_618_240u64.to_le_bytes());
            }
            assert!(looks(&d), "{title}");
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.view.pointer("/Game"), Some(&Value::from(title)));
            assert_eq!(doc.view.pointer("/Studs"), Some(&Value::from(10_212_618_240u64)));
            assert_eq!(doc.to_bytes().unwrap(), d);
            doc.set("/Studs", Value::from(777_777_770u64)).unwrap();
            let out = doc.to_bytes().unwrap();
            for &p in &places {
                assert_eq!(out[p..p + 8], 777_777_770u64.to_le_bytes(), "{title}");
            }
            d[places[places.len() - 1]] ^= 1;
            assert!(!looks(&d), "{title}");
        }
    }

    #[test]
    fn lsw2_studs_and_checksum() {
        let mut d = vec![0u8; LSW2_LEN];
        d[0x10] = 9;
        d[LSW2_STUDS..LSW2_STUDS + 4].copy_from_slice(&1_361_272_840u32.to_le_bytes());
        d[0x388] = 0xa2;
        fix_lsw1(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Studs"), Some(&Value::from(1_361_272_840u32)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Studs", Value::from(4_000_000_000u64)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32at(&out, LSW2_STUDS), 4_000_000_000);
        assert_eq!(out[0x388], 0xa2);
        assert_eq!(u32at(&out, LSW2_LEN - 4), words_sum(&out[..LSW2_LEN - 4]));
        assert!(companions(std::path::Path::new("savegame.bin"), &out).is_empty());
        d[0x10] ^= 1;
        assert!(!looks(&d));
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
