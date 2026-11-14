//! Paper Mario (Nintendo 64) FlashRAM saves: 128 KiB of eight 0x4000-byte records, each one a
//! save of one of the four files ("Mario Story 006" header, file number at 0x38 and a running
//! save count at 0x3c). The game loads the newest record of each file. A record's first 0x1380
//! bytes are covered by two 32-bit sums at 0x30 and 0x34. The file may be a plain `.fla`, one
//! with its 32-bit words swapped (Project64) or an emulator's combined `.srm`.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, pm, swap32};
use crate::tree::TreeDoc;

pub const KIND: &str = "Paper Mario save";
const FLA: usize = 0x20000;
const SRM_FLA: usize = 0x28800;
const REC: usize = 0x4000;
const SPAN: usize = 0x1380;
const MAGIC: &[u8] = b"Mario Story 006\0";
/// Badges that raise the maximum HP and FP by five each.
const HP_PLUS: u16 = 0x14a;
const FP_PLUS: u16 = 0x14d;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Src {
    Plain(usize),
    Swapped(usize),
}

fn word(s: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([s[at], s[at + 1], s[at + 2], s[at + 3]])
}

fn sums(s: &[u8], b: usize) -> (u32, u32) {
    let mut c = 0xffff_ffffu32;
    for i in (0..SPAN).step_by(4) {
        if i != 0x30 && i != 0x34 {
            c = c.wrapping_add(word(s, b + i));
        }
    }
    (c, !c)
}

fn record_ok(s: &[u8], b: usize) -> bool {
    &s[b..b + MAGIC.len()] == MAGIC && word(s, b + 0x80) != 0xffff_ffff && sums(s, b) == (word(s, b + 0x30), word(s, b + 0x34))
}

fn valid(s: &[u8]) -> bool {
    s.len() == FLA && (0..8).any(|r| record_ok(s, r * REC))
}

fn find(d: &[u8]) -> Option<(Src, Vec<u8>)> {
    let base = match d.len() {
        FLA => 0,
        296_960 => SRM_FLA,
        _ => return None,
    };
    let raw = &d[base..base + FLA];
    if valid(raw) {
        return Some((Src::Plain(base), raw.to_vec()));
    }
    let sw = swap32(raw);
    valid(&sw).then_some((Src::Swapped(base), sw))
}

pub fn looks(d: &[u8]) -> bool {
    find(d).is_some()
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let mut o = outer.to_vec();
    match find(outer).map(|x| x.0) {
        Some(Src::Plain(b)) => o[b..b + FLA].copy_from_slice(inner),
        Some(Src::Swapped(b)) => o[b..b + FLA].copy_from_slice(&swap32(inner)),
        None => {}
    }
    o
}

/// The newest record of each file: `(file, start)`.
fn newest(s: &[u8]) -> Vec<(usize, usize)> {
    newest_by(s, record_ok)
}

/// The record header alone (after an edit the sums are not yet right).
fn header_ok(s: &[u8], b: usize) -> bool {
    &s[b..b + MAGIC.len()] == MAGIC && word(s, b + 0x80) != 0xffff_ffff
}

fn newest_by(s: &[u8], ok: fn(&[u8], usize) -> bool) -> Vec<(usize, usize)> {
    let mut out = vec![];
    for file in 0..4u32 {
        let best = (0..8).map(|r| r * REC).filter(|&b| ok(s, b) && word(s, b + 0x38) == file).max_by_key(|&b| word(s, b + 0x3c));
        if let Some(b) = best {
            out.push((file as usize, b));
        }
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (_, s) = find(d).ok_or("not a Paper Mario save")?;
    let mut b = Builder::new(&s);
    b.inside(d, put);
    let list = newest(&s);
    for &(file, base) in &list {
        let name = format!("File {}", file + 1);
        add(&mut b, &[&name], base, pm::SAVE);
        // The file-select screen keeps its own copies of these.
        b.mirror(&[&name, "General", "Star Spirits saved"], base + 0x12f0);
        b.mirror(&[&name, "General", "Level"], base + 0x12f2);
        b.mirror(&[&name, "General", "Playtime (1/60 s)"], base + 0x12ec);
    }
    b.info(&["Info", "Files in use"], list.len());
    if list.is_empty() {
        return Err("no Paper Mario file has been started".into());
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note(crate::i18n::tr("Close the emulator first. The newest save of each file is the one shown. Max HP and Max FP include the HP Plus and FP Plus badges worn; the base values are worked out from them on save. Both sums of each edited save are made again.")))
}

fn equipped(d: &[u8], b: usize, id: u16) -> u8 {
    (0..32).filter(|i| u16::from_be_bytes([d[b + 0x248 + i * 2], d[b + 0x249 + i * 2]]) == id).count() as u8
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    for (_, b) in newest_by(d, header_ok) {
        if !touched.iter().any(|&t| (b..b + REC).contains(&t)) {
            continue;
        }
        let has = |o: usize| touched.contains(&(b + o));
        if has(0x43) {
            d[b + 0x44] = d[b + 0x43].saturating_sub(5 * equipped(d, b, HP_PLUS)).max(1);
        }
        if has(0x46) {
            d[b + 0x47] = d[b + 0x46].saturating_sub(5 * equipped(d, b, FP_PLUS));
        }
        d[b + 0x42] = d[b + 0x42].min(d[b + 0x43]);
        d[b + 0x45] = d[b + 0x45].min(d[b + 0x46]);
        d[b + 0x2d0] = d[b + 0x2d0].min(d[b + 0x2ce]);
        let (c1, c2) = sums(d, b);
        d[b + 0x30..b + 0x34].copy_from_slice(&c1.to_be_bytes());
        d[b + 0x34..b + 0x38].copy_from_slice(&c2.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut s = vec![0xffu8; FLA];
        for (r, count) in [(0usize, 1u32), (1, 2)] {
            let b = r * REC;
            s[b..b + REC].fill(0);
            s[b..b + MAGIC.len()].copy_from_slice(MAGIC);
            s[b + 0x3c..b + 0x40].copy_from_slice(&count.to_be_bytes());
            s[b + 0x42..b + 0x48].copy_from_slice(&[10, 10, 10, 5, 5, 5]);
            let (c1, c2) = sums(&s, b);
            s[b + 0x30..b + 0x34].copy_from_slice(&c1.to_be_bytes());
            s[b + 0x34..b + 0x38].copy_from_slice(&c2.to_be_bytes());
        }
        s
    }

    #[test]
    fn edits_the_newest_record_and_keeps_its_sums() {
        let s = sample();
        let swapped = swap32(&s);
        assert!(looks(&swapped));
        let mut doc = parse(&swapped).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), swapped);
        doc.set("/File 1/General/Coins", 999.into()).unwrap();
        doc.set("/File 1/General/Level", 27.into()).unwrap();
        doc.set("/File 1/Stats/Max HP", 50.into()).unwrap();
        let out = swap32(&doc.to_bytes().unwrap());
        let b = REC;
        assert_eq!(u16::from_be_bytes([out[b + 0x4c], out[b + 0x4d]]), 999);
        assert_eq!((out[b + 0x49], out[b + 0x12f2]), (27, 27));
        assert_eq!((out[b + 0x43], out[b + 0x44]), (50, 50));
        assert!(record_ok(&out, b));
        assert_eq!(out[..REC], s[..REC], "the older record is left alone");
    }

    /// `SAVE_SAMPLES=<dir>`: every real save opens and writes back unchanged.
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let doc = parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d, "{:?}", e.path());
            println!("{:?}: {}", e.file_name(), doc.view.pointer("/Info/Files in use").unwrap());
        }
    }
}
