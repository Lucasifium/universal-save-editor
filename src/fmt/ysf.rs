//! Ys: The Oath in Felghana (PC) `ysf_NN.nacci`: a 7,208-byte file with a plain header and two
//! blocks hidden by Falcom's "Nacci" cipher (each byte has a keystream byte taken away; the
//! stream comes from a seed that is multiplied by 0x3D09 for every byte). The first block holds
//! the character header and the seed and CRC-32 of the second; both CRCs are taken over the
//! cipher text. Cipher by Luigi Auriemma's `ysnacci` (GPL-2.0-or-later); field layout and the
//! difficulty table from geq991/ysf-save-tools (GPL-2.0-or-later).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::Arc;

pub const KIND: &str = "Ys: Oath in Felghana save";
const SIZE: usize = 7208;
const B1: usize = 0x30;
const B1_LEN: usize = 0x8C;
const B2: usize = 0xBC;

fn le32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

fn put32(d: &mut [u8], at: usize, v: u32) {
    d[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn crc32(d: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in d {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

/// Takes the keystream off (`sign` = -1) or puts it back on (`sign` = 1).
fn nacci(buf: &mut [u8], seed: u32, add: bool) {
    let mut n = seed;
    for b in buf {
        n = n.wrapping_mul(0x3D09);
        let k = (n >> 16) as u8;
        *b = if add { b.wrapping_add(k) } else { b.wrapping_sub(k) };
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && d.starts_with(b"Ys3\0") && &d[0x27..0x30] == b"Nacci,Thx" && le32(d, 8) as usize == SIZE
}

/// The plain save, or why it can't be read.
fn unpack(d: &[u8]) -> Result<Vec<u8>, String> {
    if !looks(d) {
        return Err("not a Ys: The Oath in Felghana save".into());
    }
    if crc32(&d[B1..B1 + 0x30]) != le32(d, 0xC) {
        return Err("the first block's checksum is wrong".into());
    }
    let mut p = d.to_vec();
    nacci(&mut p[B1..B1 + B1_LEN], le32(d, 0x10), false);
    if crc32(&d[B2..]) != le32(&p, 0xB0) {
        return Err("the second block's checksum is wrong".into());
    }
    let seed2 = le32(&p, 0xAC);
    nacci(&mut p[B2..], seed2, false);
    Ok(p)
}

/// Back to the file's own form.
fn pack(p: &mut Vec<u8>) {
    if p.len() != SIZE {
        return;
    }
    let seed2 = le32(p, 0xAC);
    nacci(&mut p[B2..], seed2, true);
    let crc2 = crc32(&p[B2..]);
    put32(p, 0xB0, crc2);
    let seed1 = le32(p, 0x10);
    nacci(&mut p[B1..B1 + B1_LEN], seed1, true);
    let crc1 = crc32(&p[B1..B1 + 0x30]);
    put32(p, 0xC, crc1);
}

const LEVELS: [&str; 6] = ["Very Easy", "Easy", "Normal", "Hard", "Nightmare", "Inferno"];

/// What each difficulty writes: 0x50, 0x1530, 0x58, 0x152C, 0x1AC0, 0x5C, 0x60 (+ the copies
/// at 0x1618 / 0x1620) and the multiplier at 0x1BDC.
fn signature(level: usize) -> ([u32; 6], [u32; 2], f32) {
    // [0x50, 0x58, 0x5C, 0x60, 0x1AC0, 0x1530], [0x5C, 0x60] is repeated for the copies.
    match level {
        0 => ([1, 1, 1, 0, 0, 0], [1, 0], 1.0),
        1 => ([1, 1, 0, 0, 0, 0], [0, 0], 1.0),
        2 => ([1, 0, 0, 0, 1, 0], [0, 0], 1.0),
        3 => ([2, 0, 0, 0, 1, 1], [0, 0], 1.5),
        4 => ([3, 0, 0, 0, 1, 2], [0, 0], 2.0),
        _ => ([3, 0, 0, 1, 1, 2], [0, 1], 2.0),
    }
}

fn set_level(p: &mut [u8], level: usize) {
    let (a, _, mult) = signature(level);
    put32(p, 0x50, a[0]);
    put32(p, 0x58, a[1]);
    put32(p, 0x5C, a[2]);
    put32(p, 0x60, a[3]);
    put32(p, 0x1AC0, a[4]);
    put32(p, 0x1530, a[5]);
    put32(p, 0x152C, a[1]);
    put32(p, 0x1620, a[2]);
    put32(p, 0x1618, a[3]);
    p[0x1BDC..0x1BE0].copy_from_slice(&mult.to_le_bytes());
}

fn current_level(p: &[u8]) -> Option<usize> {
    (0..6).find(|&l| {
        let (a, _, mult) = signature(l);
        le32(p, 0x50) == a[0]
            && le32(p, 0x58) == a[1]
            && le32(p, 0x5C) == a[2]
            && le32(p, 0x60) == a[3]
            && le32(p, 0x1AC0) == a[4]
            && le32(p, 0x1530) == a[5]
            && f32::from_bits(le32(p, 0x1BDC)) == mult
    })
}

fn post(p: &mut Vec<u8>, chosen: &[(Arc<str>, String)]) -> Result<(), String> {
    for (tag, name) in chosen {
        if &**tag == "difficulty" {
            let l = LEVELS.iter().position(|n| n == name).ok_or("pick a difficulty from the list")?;
            set_level(p, l);
        }
    }
    Ok(())
}

fn text(d: &[u8], at: usize, n: usize) -> String {
    let s = &d[at..at + n];
    let end = s.iter().position(|&c| c == 0).unwrap_or(n);
    String::from_utf8_lossy(&s[..end]).into_owned()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let p = unpack(d)?;
    let mut b = Builder::new(&p);
    b.ranged(&["Adol", "Level"], 0x34, Kind::U32, 1, 99);
    b.mirror(&["Adol", "Level"], 0x298);
    b.ranged(&["Adol", "Max HP"], 0x38, Kind::U32, 1, 99_999);
    b.mirror(&["Adol", "Max HP"], 0x40);
    b.ranged(&["Adol", "Experience"], 0x48, Kind::U32, 0, 99_999_999);
    b.ranged(&["Adol", "Gold"], 0x4C, Kind::U32, 0, 99_999_999);
    b.mirror(&["Adol", "Gold"], 0x294);
    b.info(&["Adol", "Play time (frames, 60 per second)"], le32(&p, 0x64));
    b.info(&["Save", "Date"], text(&p, 0x68, 11));
    b.info(&["Save", "Time"], text(&p, 0x73, 9));
    b.info(&["Save", "Place"], text(&p, 0x7C, 48));
    match current_level(&p) {
        Some(l) => b.info(&["Difficulty", "Now"], LEVELS[l]),
        None => b.info(&["Difficulty", "Now"], "custom (not one of the six)"),
    }
    b.option(&["Difficulty", "Change to"], LEVELS.iter().map(|s| s.to_string()).collect(), "difficulty".into());
    Ok(b.finish_with(KIND, Checksum::Repack(pack), Some(post)).with_note(crate::i18n::tx("Close the game and turn off Steam cloud saves first. The game's two checksums are rewritten. Choosing a difficulty writes every place the game keeps it.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut p = vec![0u8; SIZE];
        p[..4].copy_from_slice(b"Ys3\0");
        put32(&mut p, 4, 1);
        put32(&mut p, 8, SIZE as u32);
        put32(&mut p, 0x10, 0x1234_5678);
        p[0x27..0x30].copy_from_slice(b"Nacci,Thx");
        put32(&mut p, 0x34, 17);
        put32(&mut p, 0x38, 400);
        put32(&mut p, 0x4C, 7817);
        put32(&mut p, 0x50, 1);
        put32(&mut p, 0x294, 7817);
        put32(&mut p, 0x298, 17);
        put32(&mut p, 0xAC, 0x0BAD_F00D);
        p[0x1BDC..0x1BE0].copy_from_slice(&1.0f32.to_le_bytes());
        pack(&mut p);
        p
    }

    #[test]
    fn round_trips_and_edits() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Adol/Gold").unwrap(), 7817);
        doc.set("/Adol/Gold", 5_000_000.into()).unwrap();
        doc.set("/Adol/Level", 40.into()).unwrap();
        doc.set("/Difficulty/Change to", "Inferno".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let p = unpack(&out).unwrap();
        assert_eq!(le32(&p, 0x4C), 5_000_000);
        assert_eq!(le32(&p, 0x294), 5_000_000);
        assert_eq!(le32(&p, 0x298), 40);
        assert_eq!(current_level(&p), Some(5));
        assert_eq!(parse(&out).unwrap().view.pointer("/Difficulty/Now").unwrap(), "Inferno");
    }
}
