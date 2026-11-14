//! Ocarina of Time (Nintendo 64 and GameCube) SRAM saves: 32 KiB of three 0x1450-byte files
//! followed by three backup copies, each file ending in a 16-bit sum of its 16-bit words.
//! The file may be a plain `.sra`, one with its 32-bit words swapped (Project64), an emulator's
//! combined `.srm`, or a GameCube `.gci` that keeps the SRAM in 0x2000-byte blocks.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, oot};
use crate::tree::TreeDoc;

pub const KIND: &str = "Ocarina of Time save";
const SRAM: usize = 0x8000;
const FILE: usize = 0x1450;
const BACKUP: usize = 3 * FILE;

/// Where the SRAM sits in the file and how it is stored.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Src {
    Plain(usize),
    Swapped(usize),
    Gci(usize),
}

fn swap32(b: &[u8]) -> Vec<u8> {
    b.chunks(4).flat_map(|c| c.iter().rev().copied().collect::<Vec<u8>>()).collect()
}

fn sum_of(s: &[u8], b: usize) -> u16 {
    s[b + 0x20..b + 0x1372].chunks(2).fold(0u16, |a, c| a.wrapping_add(u16::from_be_bytes([c[0], c[1]])))
}

fn file_ok(s: &[u8], b: usize) -> bool {
    s[b + 0x3c] == 0x5a && s[b + 0x1372..b + 0x1374] == sum_of(s, b).to_be_bytes()
}

fn valid(s: &[u8]) -> bool {
    s.len() >= SRAM && (0..3).any(|i| file_ok(s, i * FILE))
}

/// The SRAM bytes of a GameCube save: each 0x2000-byte block holds 0x1ffc of them after a sum.
fn from_gci(d: &[u8], off: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(SRAM);
    let mut at = off;
    while out.len() < SRAM {
        let n = (SRAM - out.len()).min(0x1ffc);
        out.extend_from_slice(d.get(at..at + n)?);
        at += 0x2000;
    }
    Some(out)
}

fn into_gci(d: &mut [u8], off: usize, s: &[u8]) {
    let (mut at, mut done) = (off, 0);
    while done < SRAM {
        let n = (SRAM - done).min(0x1ffc);
        d[at..at + n].copy_from_slice(&s[done..done + n]);
        let sum = d[at..at + 0x1ffc].chunks(4).fold(0u32, |a, c| a.wrapping_add(u32::from_be_bytes([c[0], c[1], c[2], c[3]])));
        d[at - 4..at].copy_from_slice(&(if sum == 0 { 1 } else { sum }).to_be_bytes());
        at += 0x2000;
        done += n;
    }
}

fn find(d: &[u8]) -> Option<(Src, Vec<u8>)> {
    let gci = |off: usize| from_gci(d, off).filter(|s| valid(s)).map(|s| (Src::Gci(off), s));
    match d.len() {
        73_792 | 122_944 if d.get(0x30f0..0x30f6) == Some(b"ZELDA\0") => gci(0x6044),
        73_792 | 122_944 if d.get(0x3111..0x3117) == Some(b"ZELDA\0") => gci(0x12044),
        32_768 | 296_960 => {
            let base = if d.len() == SRAM { 0 } else { 0x20800 };
            let raw = &d[base..base + SRAM];
            if valid(raw) {
                Some((Src::Plain(base), raw.to_vec()))
            } else {
                let sw = swap32(raw);
                valid(&sw).then_some((Src::Swapped(base), sw))
            }
        }
        _ => None,
    }
}

pub fn looks(d: &[u8]) -> bool {
    find(d).is_some()
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let mut o = outer.to_vec();
    match find(outer).map(|x| x.0) {
        Some(Src::Plain(b)) => o[b..b + SRAM].copy_from_slice(inner),
        Some(Src::Swapped(b)) => o[b..b + SRAM].copy_from_slice(&swap32(inner)),
        Some(Src::Gci(off)) => into_gci(&mut o, off, inner),
        None => {}
    }
    o
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (_, sram) = find(d).ok_or("not an Ocarina of Time save")?;
    let mut b = Builder::new(&sram);
    b.inside(d, put);
    let mut used = 0;
    for i in 0..3 {
        if sram[i * FILE + 0x3c] != 0x5a {
            continue;
        }
        used += 1;
        add(&mut b, &[&format!("File {}", i + 1)], i * FILE, oot::SLOT);
    }
    b.info(&["Info", "Files in use"], used);
    if used == 0 {
        return Err("no Ocarina of Time file has been started".into());
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note("Close the emulator first. Health is in sixteenths of a heart. Each edited file's sum is made again and its backup copy is updated."))
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    let mut files: Vec<usize> = touched.iter().filter(|&&t| t < 3 * FILE).map(|t| t / FILE * FILE).collect();
    files.sort_unstable();
    files.dedup();
    for b in files {
        let has = |o: usize| touched.contains(&(b + o));
        if has(0x52) || has(0x53) {
            let level = d[b + 0x52];
            d[b + 0x5a] = u8::from(level > 0);
            d[b + 0x5c] = u8::from(level == 2);
            d[b + 0x53] = d[b + 0x53].min(match level { 0 => 0, 1 => 48, _ => 96 });
        }
        if has(0x4e) || has(0x5d) {
            d[b + 0xef] = if d[b + 0x5d] != 0 { (u16::from_be_bytes([d[b + 0x4e], d[b + 0x4f]]) / 16) as u8 } else { 0 };
        }
        if has(0x91) {
            d[b + 0x88] = match d[b + 0x91] & 0xf {
                0 => 0xff,
                1 => 0x3b,
                2 => 0x3c,
                _ => 0x3d,
            };
        }
        let sum = sum_of(d, b).to_be_bytes();
        d[b + 0x1372..b + 0x1374].copy_from_slice(&sum);
        let (from, to) = (b + 0x20, b + 0x1374);
        d.copy_within(from..to, from + BACKUP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; SRAM];
        d[0x3c] = 0x5a;
        d[0x50..0x52].copy_from_slice(&48u16.to_be_bytes());
        fix(&mut d, &[0x50]);
        d
    }

    #[test]
    fn plain_swapped_and_gci_round_trip_with_sums() {
        let d = sample();
        assert!(looks(&d));
        let swapped = swap32(&d);
        assert!(looks(&swapped));
        let mut doc = parse(&swapped).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), swapped);
        assert_eq!(doc.view.pointer("/File 1/General/Health (sixteenths of a heart)").unwrap(), 48);
        doc.set("/File 1/General/Rupees", 500.into()).unwrap();
        doc.set("/File 1/General/Magic Level", "Double".into()).unwrap();
        let out = swap32(&doc.to_bytes().unwrap());
        assert_eq!(u16::from_be_bytes([out[0x54], out[0x55]]), 500);
        assert_eq!(out[0x5c], 1);
        assert!(file_ok(&out, 0));
        assert_eq!(out[0x20..0x1374], out[BACKUP + 0x20..BACKUP + 0x1374]);
        // The same save inside a GameCube file.
        let mut g = vec![0u8; 73_792];
        g[0x30f0..0x30f6].copy_from_slice(b"ZELDA\0");
        into_gci(&mut g, 0x6044, &d);
        assert!(looks(&g));
        let mut doc = parse(&g).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), g);
        doc.set("/File 1/General/Rupees", 99.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(from_gci(&out, 0x6044).unwrap()[0x54..0x56], 99u16.to_be_bytes());
        assert_eq!(out.len(), g.len());
    }

    /// `SAVE_SAMPLES=<dir>`: fixing up a real save leaves it exactly as the game wrote it.
    #[test]
    #[ignore]
    fn fix_matches_real_saves() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            let Some((_, inner)) = find(&d) else { continue };
            let mut x = inner.clone();
            let all: Vec<usize> = (0..3).filter(|&i| x[i * FILE + 0x3c] == 0x5a).map(|i| i * FILE + 0x91).collect();
            fix(&mut x, &all);
            let diff: Vec<usize> = (0..x.len()).filter(|&i| x[i] != inner[i]).take(8).collect();
            // The equipped-sword byte is derived from the owned sword; only it may differ.
            assert!(diff.iter().all(|&i| i % FILE == 0x88), "{:?}: {:x?}", e.path(), diff);
            assert_eq!(put(&d, &inner), d, "{:?}", e.path());
            n += 1;
        }
        println!("{n} saves checked");
    }
}
