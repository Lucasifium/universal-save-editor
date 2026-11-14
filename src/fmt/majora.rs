//! The Legend of Zelda: Majora's Mask (Nintendo 64 and GameCube) FlashRAM saves: 128 KiB in
//! which each save slot is 0x2000 bytes followed by a backup copy, and (outside Japan) each slot
//! may also have an owl-statue save of 0x4000 bytes, again with a backup. Every save ends its
//! data with a 16-bit sum of its bytes. The file may be a plain `.fla`, one with its 32-bit
//! words swapped (Project64), an emulator's combined `.srm`, or the GameCube `.gci` of the
//! Collector's Edition disc, which keeps the FlashRAM in 0x2000-byte blocks.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, mm, swap32};
use crate::tree::TreeDoc;

pub const KIND: &str = "Majora's Mask save";
const FLA: usize = 0x20000;
const SRM_FLA: usize = 0x28800;
const GCI_AT: usize = 0x6044;
/// Where the sum sits: Japanese saves keep it 0x384 bytes further on.
const SUM_WEST: usize = 0x100a;
const SUM_JAPAN: usize = 0x138e;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Src {
    Plain(usize),
    Swapped(usize),
    Gci,
}

fn sum_of(s: &[u8], base: usize, len: usize, at: usize) -> u16 {
    let all: u32 = s[base..base + len].iter().map(|&x| x as u32).sum();
    (all - s[base + at] as u32 - s[base + at + 1] as u32) as u16
}

/// Length of the save at `base` that the sum covers (an owl save is twice as long).
fn span(s: &[u8], base: usize) -> usize {
    if s[base + 0x23] != 0 { 0x4000 } else { 0x2000 }
}

fn save_ok(s: &[u8], base: usize, at: usize) -> bool {
    let len = span(s, base);
    base + len <= s.len() && &s[base + 0x24..base + 0x2a] == b"ZELDA3" && s[base + at..base + at + 2] == sum_of(s, base, len, at).to_be_bytes()
}

/// The sum's place for this FlashRAM (`None`: no valid save in it).
fn region(s: &[u8]) -> Option<usize> {
    if s.len() != FLA {
        return None;
    }
    let bases = (0..FLA).step_by(0x2000);
    [SUM_WEST, SUM_JAPAN].into_iter().find(|&at| bases.clone().any(|b| save_ok(s, b, at)))
}

fn from_gci(d: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(FLA);
    let mut at = GCI_AT;
    while out.len() < FLA {
        let n = (FLA - out.len()).min(0x1ffc);
        out.extend_from_slice(d.get(at..at + n)?);
        at += 0x2000;
    }
    Some(out)
}

fn into_gci(d: &mut [u8], s: &[u8]) {
    let (mut at, mut done) = (GCI_AT, 0);
    while done < FLA {
        let n = (FLA - done).min(0x1ffc);
        d[at..at + n].copy_from_slice(&s[done..done + n]);
        let sum = d[at..at + 0x1ffc].chunks(4).fold(0u32, |a, c| a.wrapping_add(u32::from_be_bytes([c[0], c[1], c[2], c[3]])));
        d[at - 4..at].copy_from_slice(&(if sum == 0 { 1 } else { sum }).to_be_bytes());
        at += 0x2000;
        done += n;
    }
}

fn find(d: &[u8]) -> Option<(Src, Vec<u8>)> {
    if d.len() == 172_096 && d.get(..3) == Some(b"PZL") && d.get(8..14) == Some(b"ZELDA2") {
        return from_gci(d).filter(|s| region(s) == Some(SUM_WEST)).map(|s| (Src::Gci, s));
    }
    let base = match d.len() {
        FLA => 0,
        296_960 => SRM_FLA,
        _ => return None,
    };
    let raw = &d[base..base + FLA];
    if region(raw).is_some() {
        return Some((Src::Plain(base), raw.to_vec()));
    }
    let sw = swap32(raw);
    region(&sw).is_some().then_some((Src::Swapped(base), sw))
}

pub fn looks(d: &[u8]) -> bool {
    find(d).is_some()
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let mut o = outer.to_vec();
    match find(outer).map(|x| x.0) {
        Some(Src::Plain(b)) => o[b..b + FLA].copy_from_slice(inner),
        Some(Src::Swapped(b)) => o[b..b + FLA].copy_from_slice(&swap32(inner)),
        Some(Src::Gci) => into_gci(&mut o, inner),
        None => {}
    }
    o
}

/// The saves the game reads for each slot: `(slot, start)`. Outside Japan an owl-statue save
/// is what the game loads when one exists, so that one is edited.
fn saves(s: &[u8], at: usize) -> Vec<(usize, usize)> {
    pick(s, |b| save_ok(s, b, at), at)
}

/// Like `saves`, by the header alone (after an edit the sums are not yet right).
fn edited(s: &[u8], at: usize) -> Vec<(usize, usize)> {
    pick(s, |b| &s[b + 0x24..b + 0x2a] == b"ZELDA3", at)
}

fn pick(s: &[u8], save_ok: impl Fn(usize) -> bool, at: usize) -> Vec<(usize, usize)> {
    let mut out = vec![];
    if at == SUM_JAPAN {
        for i in 0..3 {
            if save_ok(i * 0x4000) {
                out.push((i, i * 0x4000));
            }
        }
        return out;
    }
    for i in 0..2 {
        let owl = 0x8000 + i * 0x8000;
        if save_ok(owl) && s[owl + 0x23] != 0 {
            out.push((i, owl));
        } else if save_ok(i * 0x4000) {
            out.push((i, i * 0x4000));
        }
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (_, s) = find(d).ok_or("not a Majora's Mask save")?;
    let at = region(&s).ok_or("not a Majora's Mask save")?;
    let mut b = Builder::new(&s);
    b.inside(d, put);
    let list = saves(&s, at);
    for &(slot, base) in &list {
        let name = format!("Slot {}", slot + 1);
        add(&mut b, &[&name], base, if at == SUM_JAPAN { mm::SLOT_JP } else { mm::SLOT });
        b.mirror(&[&name, "General", "Owl File", "Day"], base + 0x1c);
        b.info(&[&name, "Owl save"], if base >= 0x8000 { "Yes" } else { "No" });
    }
    b.info(&["Info", "Slots in use"], list.len());
    if list.is_empty() {
        return Err("no Majora's Mask slot has been started".into());
    }
    let fix = if at == SUM_JAPAN { fix_japan } else { fix_west };
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note(crate::i18n::tr("Close the emulator first. Health is in sixteenths of a heart. When a slot has an owl-statue save, that is the save the game loads, so it is the one shown and edited. Each edited save's sum is made again and its backup copy is updated.")))
}

fn fix_west(d: &mut Vec<u8>, touched: &[usize]) {
    fix(d, touched, SUM_WEST)
}

fn fix_japan(d: &mut Vec<u8>, touched: &[usize]) {
    fix(d, touched, SUM_JAPAN)
}

/// Keeps the values the game derives from others in step, then the sum and the backup copy.
fn fix(d: &mut Vec<u8>, touched: &[usize], at: usize) {
    let list = edited(d, at);
    for (_, base) in list {
        let len = span(d, base);
        if !touched.iter().any(|&t| (base..base + len).contains(&t)) {
            continue;
        }
        let has = |o: usize| touched.contains(&(base + o));
        let u16_at = |d: &[u8], o: usize| u16::from_be_bytes([d[base + o], d[base + o + 1]]);
        if has(0x38) {
            let level = d[base + 0x38];
            d[base + 0x40] = u8::from(level > 0);
            d[base + 0x41] = u8::from(level == 2);
        }
        if has(0x38) || has(0x39) {
            let cap = match d[base + 0x38] { 0 => 0, 1 => 48, _ => 96 };
            d[base + 0x39] = d[base + 0x39].min(cap);
        }
        if has(0x34) || has(0x42) {
            d[base + 0xd3] = if d[base + 0x42] != 0 { (u16_at(d, 0x34) / 16) as u8 } else { 0 };
        }
        // Rupees, bombs and arrows above what the wallet, bag and quiver hold.
        let wallet = [99u16, 200, 500][((d[base + 0xba] >> 4) & 3).min(2) as usize];
        if has(0x3a) || has(0x3b) || has(0xba) {
            let rupees = u16_at(d, 0x3a).min(wallet);
            d[base + 0x3a..base + 0x3c].copy_from_slice(&rupees.to_be_bytes());
        }
        let bag = [0u8, 20, 30, 40][((d[base + 0xbb] >> 3) & 3) as usize];
        let quiver = [0u8, 30, 40, 50][(d[base + 0xbb] & 3) as usize];
        if has(0xa6) || has(0xbb) {
            d[base + 0xa6] = d[base + 0xa6].min(bag);
        }
        if has(0xa1) || has(0xbb) {
            d[base + 0xa1] = d[base + 0xa1].min(quiver);
        }
        let sum = sum_of(d, base, len, at).to_be_bytes();
        d[base + at..base + at + 2].copy_from_slice(&sum);
        let backup = if at == SUM_JAPAN { base + 0x10000 } else { base + len };
        if backup + len <= d.len() {
            d.copy_within(base..base + len, backup);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut s = vec![0xffu8; FLA];
        for b in [0usize, 0x2000] {
            s[b..b + 0x2000].fill(0);
            s[b + 0x24..b + 0x2a].copy_from_slice(b"ZELDA3");
            s[b + 0x34..b + 0x36].copy_from_slice(&48u16.to_be_bytes());
            s[b + 0xba] = 0x10;
            s[b + 0x70..b + 0xa0].fill(0xff);
        }
        let v = sum_of(&s, 0, 0x2000, SUM_WEST).to_be_bytes();
        s[SUM_WEST..SUM_WEST + 2].copy_from_slice(&v);
        s[0x2000 + SUM_WEST..0x2000 + SUM_WEST + 2].copy_from_slice(&v);
        s
    }

    #[test]
    fn plain_swapped_and_gci_round_trip_with_sums() {
        let s = sample();
        assert!(looks(&s));
        let swapped = swap32(&s);
        let mut doc = parse(&swapped).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), swapped);
        assert_eq!(doc.view.pointer("/Slot 1/General/Max Health").unwrap(), 48);
        doc.set("/Slot 1/General/Rupees", 9999.into()).unwrap();
        doc.set("/Slot 1/General/Magic Level", "Double".into()).unwrap();
        let out = swap32(&doc.to_bytes().unwrap());
        assert_eq!(u16::from_be_bytes([out[0x3a], out[0x3b]]), 200, "capped by the adult wallet");
        assert_eq!((out[0x40], out[0x41]), (1, 1));
        assert!(save_ok(&out, 0, SUM_WEST));
        assert_eq!(out[..0x2000], out[0x2000..0x4000]);
        // The same save inside a GameCube file.
        let mut g = vec![0u8; 172_096];
        g[..6].copy_from_slice(b"PZLE01");
        g[8..14].copy_from_slice(b"ZELDA2");
        into_gci(&mut g, &s);
        let mut doc = parse(&g).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), g);
        doc.set("/Slot 1/Inventory/Masks/Deku Mask", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(from_gci(&out).unwrap()[0x8d], 0x32);
        assert_eq!(out.len(), g.len());
    }

    /// `SAVE_SAMPLES=<dir>`: every real save opens, writes back unchanged, and a no-op fix keeps it.
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            let Some((_, s)) = find(&d) else { continue };
            let doc = parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d, "{:?}", e.path());
            let at = region(&s).unwrap();
            let mut x = s.clone();
            let all: Vec<usize> = saves(&s, at).iter().map(|&(_, b)| b + 0x24).collect();
            fix(&mut x, &all, at);
            for &(_, b) in &saves(&s, at) {
                let len = span(&s, b);
                let diff: Vec<usize> = (b..b + len).filter(|&i| x[i] != s[i]).map(|i| i - b).take(8).collect();
                assert!(diff.is_empty(), "{:?} save at {b:#x}: {diff:x?}", e.path());
            }
            println!("{:?}: {} slot(s)", e.file_name(), saves(&s, at).len());
            n += 1;
        }
        println!("{n} saves");
    }
}
