//! F-Zero X (Nintendo 64) SRAM saves (`.sra`, 32 KiB, or an emulator's combined `.srm`; the
//! words may be swapped as Project64 writes them). The file starts with "F-ZERO X"; each block
//! (header, Death Race, the 24 Time Attack courses and the cups won per machine) ends or starts
//! with a big-endian 16-bit byte sum.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::n64data::{sram, sram_put};
use crate::tree::TreeDoc;

pub const KIND: &str = "F-Zero X save";

/// A Time Attack time of 0x36ee7f ms (59:59.999) means the course has no record.
const NO_TIME: i64 = 0x36_ee7f;

const COURSES: [&str; 24] = [
    "Mute City", "Silence", "Sand Ocean", "Devil's Forest", "Big Blue", "Port Town", "Sector Alpha", "Red Canyon",
    "Devil's Forest 2", "Mute City 2", "Big Blue 2", "White Land", "Fire Field", "Silence 2", "Sector Beta", "Red Canyon 2",
    "White Land 2", "Mute City 3", "Rainbow Road", "Devil's Forest 3", "Space Plant", "Sand Ocean 2", "Port Town 2", "Big Hand",
];
const CUPS: [&str; 5] = ["Jack Cup", "Queen Cup", "King Cup", "Joker Cup", "X Cup"];
const CLASSES: [&str; 4] = ["Novice", "Standard", "Expert", "Master"];
const PLACES: [&str; 5] = ["1st place", "2nd place", "3rd place", "4th place", "5th place"];

/// Each machine's five "cup won" bits (byte inside a class's 0x14-byte block, bit).
const MACHINES: [(&str, [(usize, u32); 5]); 30] = [
    ("Blue Falcon", [(0x11, 0), (0x11, 1), (0x11, 2), (0x11, 3), (0x11, 4)]),
    ("Golden Fox", [(0x11, 5), (0x11, 6), (0x11, 7), (0x10, 0), (0x10, 1)]),
    ("Wild Goose", [(0x10, 2), (0x10, 3), (0x10, 4), (0x10, 5), (0x10, 6)]),
    ("Fire Stingray", [(0x13, 0), (0x13, 1), (0x13, 2), (0x13, 3), (0x13, 4)]),
    ("White Cat", [(0x13, 5), (0x13, 6), (0x13, 7), (0x12, 0), (0x12, 1)]),
    ("Red Gazelle", [(0x12, 2), (0x12, 3), (0x12, 4), (0x12, 5), (0x12, 6)]),
    ("Iron Tiger", [(0x15, 5), (0x15, 6), (0x15, 7), (0x14, 0), (0x14, 1)]),
    ("Deep Claw", [(0x14, 2), (0x14, 3), (0x14, 4), (0x14, 5), (0x14, 6)]),
    ("Crazy Bear", [(0x1e, 2), (0x1e, 3), (0x1e, 4), (0x1e, 5), (0x1e, 6)]),
    ("Great Star", [(0x15, 0), (0x15, 1), (0x15, 2), (0x15, 3), (0x15, 4)]),
    ("Big Fang", [(0x21, 5), (0x21, 6), (0x21, 7), (0x20, 0), (0x20, 1)]),
    ("Mad Wolf", [(0x23, 0), (0x23, 1), (0x23, 2), (0x23, 3), (0x23, 4)]),
    ("Night Thunder", [(0x21, 0), (0x21, 1), (0x21, 2), (0x21, 3), (0x21, 4)]),
    ("Twin Noritta", [(0x17, 0), (0x17, 1), (0x17, 2), (0x17, 3), (0x17, 4)]),
    ("Wonder Wasp", [(0x1c, 2), (0x1c, 3), (0x1c, 4), (0x1c, 5), (0x1c, 6)]),
    ("Queen Meteor", [(0x1d, 5), (0x1d, 6), (0x1d, 7), (0x1c, 0), (0x1c, 1)]),
    ("Blood Hawk", [(0x22, 2), (0x22, 3), (0x22, 4), (0x22, 5), (0x22, 6)]),
    ("Astro Robin", [(0x1a, 2), (0x1a, 3), (0x1a, 4), (0x1a, 5), (0x1a, 6)]),
    ("Little Wyvern", [(0x19, 0), (0x19, 1), (0x19, 2), (0x19, 3), (0x19, 4)]),
    ("Death Anchor", [(0x1f, 5), (0x1f, 6), (0x1f, 7), (0x1e, 0), (0x1e, 1)]),
    ("Wild Boar", [(0x1b, 5), (0x1b, 6), (0x1b, 7), (0x1a, 0), (0x1a, 1)]),
    ("King Meteor", [(0x1d, 0), (0x1d, 1), (0x1d, 2), (0x1d, 3), (0x1d, 4)]),
    ("Super Piranha", [(0x17, 5), (0x17, 6), (0x17, 7), (0x16, 0), (0x16, 1)]),
    ("Mighty Hurricane", [(0x16, 2), (0x16, 3), (0x16, 4), (0x16, 5), (0x16, 6)]),
    ("Space Angler", [(0x19, 5), (0x19, 6), (0x19, 7), (0x18, 0), (0x18, 1)]),
    ("Mighty Typhoon", [(0x20, 2), (0x20, 3), (0x20, 4), (0x20, 5), (0x20, 6)]),
    ("Hyper Speeder", [(0x1f, 0), (0x1f, 1), (0x1f, 2), (0x1f, 3), (0x1f, 4)]),
    ("Green Panther", [(0x18, 2), (0x18, 3), (0x18, 4), (0x18, 5), (0x18, 6)]),
    ("Black Bull", [(0x1b, 0), (0x1b, 1), (0x1b, 2), (0x1b, 3), (0x1b, 4)]),
    ("Sonic Phantom", [(0x23, 5), (0x23, 6), (0x23, 7), (0x22, 0), (0x22, 1)]),
];

fn sum16(d: &[u8]) -> [u8; 2] {
    (d.iter().map(|&b| u32::from(b)).sum::<u32>() as u16).to_be_bytes()
}

fn valid(s: &[u8]) -> bool {
    s.starts_with(b"F-ZERO X") && s[0xe..0x10] == sum16(&s[..0xe])
}

pub fn looks(d: &[u8]) -> bool {
    sram(d, valid).is_some()
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    sram_put(outer, inner, valid)
}

fn yes_no() -> Vec<(String, u64)> {
    vec![("No".into(), 0), ("Yes".into(), 1)]
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (_, s) = sram(d, valid).ok_or("not an F-Zero X save")?;
    let mut b = Builder::new(&s);
    b.inside(d, put);
    b.big_endian = true;
    let with = vec![("Without".to_string(), 0), ("With".to_string(), 1)];
    b.pick_bits(&["Options", "Everything unlocked"], 0x8, Kind::U8, 3, 1, &yes_no());
    b.pick_bits(&["Options", "VS COM (2P, 3P)"], 0x8, Kind::U8, 0, 1, &with);
    b.pick_bits(&["Options", "VS slot"], 0x8, Kind::U8, 1, 1, &with);
    b.pick_bits(&["Options", "VS handicap"], 0x8, Kind::U8, 4, 2, &[("Without".into(), 0), ("+1".into(), 1), ("+2".into(), 2)]);
    b.pick_bits(&["Options", "Sound mode"], 0x8, Kind::U8, 2, 1, &[("Stereo".into(), 0), ("Mono".into(), 1)]);
    let progress: Vec<(String, u64)> =
        ["-", "Novice", "Standard", "Expert", "Master"].iter().enumerate().map(|(i, n)| (n.to_string(), i as u64)).collect();
    for (i, cup) in CUPS[..4].iter().enumerate() {
        b.pick(&["GP Race", cup], 0xa + i, Kind::U8, &progress);
    }
    b.ranged(&["Death Race", "Time (ms)"], 0x14, Kind::U32, 0, NO_TIME);
    for (c, course) in COURSES.iter().enumerate() {
        let base = c * 0x110;
        b.ranged(&["Time Attack", course, "Best lap (ms)"], base + 0x50, Kind::U32, 0, NO_TIME);
        for (p, place) in PLACES.iter().enumerate() {
            b.ranged(&["Time Attack", course, &format!("{place} (ms)")], base + 0x24 + p * 4, Kind::U32, 0, NO_TIME);
        }
    }
    for (k, class) in CLASSES.iter().enumerate() {
        for (machine, bits) in MACHINES.iter() {
            for (cup, (at, bit)) in CUPS.iter().zip(bits.iter()) {
                b.pick_bits(&["Cups won", class, machine, cup], 0x7f80 + k * 0x14 + at, Kind::U8, *bit, 1, &yes_no());
            }
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. Times are in milliseconds; 3599999 (59:59.999) means the course has no record.")))
}

fn fix(d: &mut Vec<u8>) {
    let s = sum16(&d[..0xe]);
    d[0xe..0x10].copy_from_slice(&s);
    let s = sum16(&d[0x12..0x20]);
    d[0x10..0x12].copy_from_slice(&s);
    for c in 0..24 {
        let base = c * 0x110;
        let s = sum16(&d[base + 0x22..base + 0x130]);
        d[base + 0x20..base + 0x22].copy_from_slice(&s);
    }
    let s = sum16(&d[0x7f82..0x8000]);
    d[0x7f80..0x7f82].copy_from_slice(&s);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::n64data::swap32;

    fn blank() -> Vec<u8> {
        let mut s = vec![0u8; 0x8000];
        s[..8].copy_from_slice(b"F-ZERO X");
        for c in 0..24 {
            for p in 0..5 {
                let at = c * 0x110 + 0x24 + p * 4;
                s[at..at + 4].copy_from_slice(&(NO_TIME as u32).to_be_bytes());
            }
        }
        fix(&mut s);
        s
    }

    #[test]
    fn edits_a_swapped_save_and_its_sums() {
        let s = blank();
        assert!(valid(&s));
        let file = swap32(&s);
        assert!(looks(&file));
        let mut doc = parse(&file).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), file);
        doc.set("/Options/Everything unlocked", "Yes".into()).unwrap();
        doc.set("/Time Attack/Big Blue/1st place (ms)", 61_234.into()).unwrap();
        doc.set("/Cups won/Master/Golden Fox/X Cup", "Yes".into()).unwrap();
        let out = swap32(&doc.to_bytes().unwrap());
        assert_eq!(out[0x8] & 8, 8);
        let at = 4 * 0x110 + 0x24;
        assert_eq!(u32::from_be_bytes(out[at..at + 4].try_into().unwrap()), 61_234);
        assert_eq!(out[0x7f80 + 3 * 0x14 + 0x10] & 2, 2);
        let mut again = out.clone();
        fix(&mut again);
        assert_eq!(again, out);
        assert!(valid(&out));
    }

    /// `SAVE_SAMPLES=<dir>`: real saves open, keep their sums and write back unchanged.
    #[test]
    #[ignore]
    fn real_saves_round_trip() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut seen = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if let Some((_, s)) = sram(&d, valid) {
                let mut x = s.clone();
                fix(&mut x);
                assert_eq!(x, s, "{:?}", e.path());
                let doc = parse(&d).unwrap();
                assert_eq!(doc.to_bytes().unwrap(), d, "{:?}", e.path());
                seen += 1;
            }
        }
        assert!(seen > 0);
    }
}
