//! Dead Rising (2016 PC release) `DeadRising_0N_Main`, Dead Rising 2 `DR2SAVE.DR2S` and
//! Dead Rising 2: Off the Record `DCSAV0N.DSF`.
//!
//! Dead Rising: 508000 bytes in 20-byte blocks, each 16 bytes of data followed by two
//! 16-bit sums of those bytes (plain, and with every other byte subtracted); the scheme is
//! from Philymaster's security notes as used by bucanero's save-decrypters (GPL, facts only).
//! Worked out from public speedrun practice saves, in the data with the sums taken out:
//! level at 0x1C, maximum health at 0x20 (1000 per block), the skills learned as a bit mask
//! at 0x24 and the PP total at 0x90.
//!
//! Dead Rising 2 and Off the Record: the layout follows the PlayStation 3 cheat codes in
//! bucanero's apollo-patches (GPL, facts only), moved to where the PC files keep it and
//! checked on public speedrun saves: level, PP, money, then the attack, speed, health,
//! item slots and throwing levels, one 32-bit value each. Sections of the file carry a
//! CRC-32 (polynomial 0xEDB88320 run most-significant bit first, each byte bit-reversed,
//! starting at 0) of what's between them, stored before and after. Off the Record also
//! keeps level, PP and money in the load screen's header at 0x161.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND_DR1: &str = "Dead Rising save";
pub const KIND_DR2: &str = "Dead Rising 2 save";
pub const KIND_OTR: &str = "Dead Rising 2: Off the Record save";

// ------------------------------------------------------------------ Dead Rising

const DR1_LEN: usize = 508_000;

fn block_sums(b: &[u8]) -> (u16, u16) {
    let mut l = 0u16;
    let mut h = 0u16;
    for (i, &x) in b.iter().enumerate() {
        l = l.wrapping_add(u16::from(x));
        h = if i % 2 == 0 { h.wrapping_add(u16::from(x)) } else { h.wrapping_sub(u16::from(x)) };
    }
    (l, h)
}

fn block_ok(d: &[u8], at: usize) -> bool {
    let (l, h) = block_sums(&d[at..at + 16]);
    d[at + 16..at + 18] == l.to_le_bytes() && d[at + 18..at + 20] == h.to_le_bytes()
}

pub fn looks_dr1(d: &[u8]) -> bool {
    d.len() == DR1_LEN && d[..4] == [0x01, 0x10, 0, 0] && (0..d.len()).step_by(20).all(|b| block_ok(d, b))
}

fn fix_dr1(out: &mut Vec<u8>) {
    for b in (0..out.len()).step_by(20) {
        let (l, h) = block_sums(&out[b..b + 16]);
        out[b + 16..b + 18].copy_from_slice(&l.to_le_bytes());
        out[b + 18..b + 20].copy_from_slice(&h.to_le_bytes());
    }
}

/// Where a place in the data (sums taken out) is in the file.
fn raw(p: usize) -> usize {
    p / 16 * 20 + p % 16
}

/// All 21 skills learned (what a level 50 save holds).
pub const DR1_ALL_SKILLS: i64 = 0x1F_FFFF;

pub fn parse_dr1(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_dr1(d) {
        return Err("not a Dead Rising save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Frank", "Level"], raw(0x1C), Kind::U32, 1, 50);
    b.ranged(&["Frank", "PP"], raw(0x90), Kind::U32, 0, 999_999_999);
    b.ranged(&["Frank", "Max health"], raw(0x20), Kind::U32, 1000, 12_000);
    b.ranged(&["Frank", "Skills"], raw(0x24), Kind::U32, 0, DR1_ALL_SKILLS);
    let area: String = (0xC..0x10).map(|i| d[raw(i)]).take_while(|&c| c.is_ascii_alphanumeric()).map(char::from).collect();
    if !area.is_empty() {
        b.info(&["Save", "Area"], area);
    }
    Ok(b.finish(KIND_DR1, Checksum::Custom(fix_dr1)).with_note(
        "Frank's level, PP, maximum health and skills. The block checksums are updated when you save.",
    ))
}

// ------------------------------------------------------------------ Dead Rising 2 / Off the Record

const DR2_LEN: usize = 303_724;
const DR2_SECTIONS: &[(usize, usize)] = &[(0, 0x4A104)];
const DR2_STATS: usize = 0xD24;

const OTR_LEN: usize = 257_294;
const OTR_SECTIONS: &[(usize, usize)] = &[(0, 0x199), (0x6E8, 0x108D), (0x1770, 0x20E1), (0xA028, 0x24AE6), (0x29BF8, 0x3EBAE)];
const OTR_STATS: usize = 0x18D0;
const OTR_HEADER: usize = 0x161;

/// CRC-32 with the polynomial 0xEDB88320 run most-significant bit first over bit-reversed
/// bytes, starting at 0 (what Apollo calls reflected input, unreflected output).
pub fn crc_dr2(d: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = (i as u32) << 24;
            for _ in 0..8 {
                c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0xEDB8_8320 } else { c << 1 };
            }
            *e = c;
        }
        t
    });
    d.iter().fold(0u32, |c, &b| (c << 8) ^ t[((c >> 24) as u8 ^ b.reverse_bits()) as usize])
}

fn sections_ok(d: &[u8], sections: &[(usize, usize)]) -> bool {
    sections.iter().all(|&(a, z)| {
        let s = &d[a..a + 4];
        s == &d[z..z + 4] && u32::from_le_bytes(s.try_into().unwrap()) == crc_dr2(&d[a + 4..z])
    })
}

fn fix_sections(out: &mut [u8], sections: &[(usize, usize)]) {
    for &(a, z) in sections {
        let c = crc_dr2(&out[a + 4..z]).to_le_bytes();
        out[a..a + 4].copy_from_slice(&c);
        out[z..z + 4].copy_from_slice(&c);
    }
}

pub fn looks_dr2(d: &[u8]) -> bool {
    d.len() == DR2_LEN && sections_ok(d, DR2_SECTIONS)
}

pub fn looks_otr(d: &[u8]) -> bool {
    d.len() == OTR_LEN && sections_ok(d, OTR_SECTIONS)
}

fn fix_dr2(out: &mut Vec<u8>) {
    fix_sections(out, DR2_SECTIONS);
}

fn fix_otr(out: &mut Vec<u8>) {
    fix_sections(out, OTR_SECTIONS);
}

/// The eight values from level to throwing, with the highest level the game reaches.
pub const DR2_STAT_NAMES: [(&str, i64, i64); 8] = [
    ("Level", 1, 50),
    ("PP", 0, 999_999_999),
    ("Money", 0, 1_000_000_000),
    ("Attack", 0, 7),
    ("Speed", 0, 3),
    ("Health", 1, 12),
    ("Item slots", 1, 12),
    ("Throwing", 0, 5),
];

fn stats(b: &mut Builder, who: &str, at: usize) {
    for (i, (k, lo, hi)) in DR2_STAT_NAMES.into_iter().enumerate() {
        b.ranged(&[who, k], at + 4 * i, Kind::I32, lo, hi);
    }
}

pub fn parse_dr2(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_dr2(d) {
        return Err("not a Dead Rising 2 save".into());
    }
    let mut b = Builder::new(d);
    stats(&mut b, "Chuck", DR2_STATS);
    Ok(b.finish(KIND_DR2, Checksum::Custom(fix_dr2)).with_note(
        "Chuck's level, PP, money and the five level-up stats. The checksum is updated when you save.",
    ))
}

pub fn parse_otr(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_otr(d) {
        return Err("not a Dead Rising 2: Off the Record save".into());
    }
    let mut b = Builder::new(d);
    stats(&mut b, "Frank", OTR_STATS);
    for (i, k) in ["Level", "PP", "Money"].into_iter().enumerate() {
        b.mirror(&["Frank", k], OTR_HEADER + 4 * i);
    }
    Ok(b.finish(KIND_OTR, Checksum::Custom(fix_otr)).with_note(
        "Frank's level, PP, money and the five level-up stats (the load screen's copy is kept in step). The \
         checksums are updated when you save.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn crc_matches_a_bitwise_run() {
        let data = b"123456789";
        let direct = data.iter().fold(0u32, |mut c, &b| {
            c ^= u32::from(b.reverse_bits()) << 24;
            for _ in 0..8 {
                c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0xEDB8_8320 } else { c << 1 };
            }
            c
        });
        assert_eq!(crc_dr2(data), direct);
    }

    #[test]
    fn dead_rising_blocks_are_rewritten() {
        let mut d = vec![0u8; DR1_LEN];
        let mut put = |p: usize, v: u32| {
            for (i, x) in v.to_le_bytes().into_iter().enumerate() {
                d[raw(p + i)] = x;
            }
        };
        put(0, 0x1001);
        put(0x1C, 3);
        put(0x20, 5000);
        put(0x90, 50_000);
        d[raw(0xC)..raw(0xC) + 4].copy_from_slice(b"s100");
        fix_dr1(&mut d);
        assert!(looks_dr1(&d));
        let mut doc = parse_dr1(&d).unwrap();
        assert_eq!(doc.view.pointer("/Frank/Level"), Some(&Value::from(3)));
        assert_eq!(doc.view.pointer("/Frank/PP"), Some(&Value::from(50_000)));
        assert_eq!(doc.view.pointer("/Save/Area"), Some(&Value::from("s100")));
        assert!(doc.set("/Frank/Level", Value::from(51)).is_err());
        doc.set("/Frank/Level", Value::from(50)).unwrap();
        doc.set("/Frank/Skills", Value::from(DR1_ALL_SKILLS)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks_dr1(&out));
        assert_eq!(out[raw(0x1C)], 50);
        assert_eq!(&out[raw(0x24)..raw(0x24) + 4], &0x1F_FFFFu32.to_le_bytes());
    }

    fn synthetic(len: usize, sections: &[(usize, usize)]) -> Vec<u8> {
        let mut d = vec![0u8; len];
        for (i, x) in d.iter_mut().enumerate().skip(0x2000).step_by(97) {
            *x = (i % 251) as u8;
        }
        fix_sections(&mut d, sections);
        d
    }

    #[test]
    fn dead_rising_2_stats_and_crc() {
        let mut d = synthetic(DR2_LEN, DR2_SECTIONS);
        for (i, v) in [7i32, 9000, 1234, 2, 1, 6, 5, 1].into_iter().enumerate() {
            d[DR2_STATS + 4 * i..DR2_STATS + 4 * i + 4].copy_from_slice(&v.to_le_bytes());
        }
        fix_sections(&mut d, DR2_SECTIONS);
        let mut doc = parse_dr2(&d).unwrap();
        assert_eq!(doc.view.pointer("/Chuck/Money"), Some(&Value::from(1234)));
        assert!(doc.set("/Chuck/Attack", Value::from(8)).is_err());
        doc.set("/Chuck/Attack", Value::from(7)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks_dr2(&out));
        assert_eq!(out[DR2_STATS + 12], 7);
    }

    #[test]
    fn off_the_record_keeps_the_header_copy() {
        let d = synthetic(OTR_LEN, OTR_SECTIONS);
        let mut doc = parse_otr(&d).unwrap();
        doc.set("/Frank/PP", Value::from(5_000_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks_otr(&out));
        assert_eq!(&out[OTR_STATS + 4..OTR_STATS + 8], &5_000_000i32.to_le_bytes());
        assert_eq!(&out[OTR_HEADER + 4..OTR_HEADER + 8], &5_000_000i32.to_le_bytes());
    }

    /// `DR_SAVES="a|b|c"`: each real save opens and writes back byte for byte.
    #[test]
    #[ignore]
    fn real_saves_round_trip() {
        let Ok(list) = std::env::var("DR_SAVES") else { return };
        for p in list.split('|') {
            let d = std::fs::read(p).unwrap();
            let doc = if looks_dr1(&d) {
                parse_dr1(&d)
            } else if looks_dr2(&d) {
                parse_dr2(&d)
            } else {
                parse_otr(&d)
            }
            .unwrap_or_else(|e| panic!("{p}: {e}"));
            println!("{p}: {}", doc.view);
            assert_eq!(doc.to_bytes().unwrap(), d, "{p}");
        }
    }
}
