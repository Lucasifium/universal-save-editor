//! Mario Kart 64 (Nintendo 64) EEPROM saves (`.eep`, 512 or 2048 bytes, or an emulator's
//! combined `.srm`, whose EEPROM comes first): Grand Prix cups, Time Trial records (five
//! places and a best lap for each of the sixteen courses) and the sound mode.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).
//! Each record is three little-endian bytes: the time in hundredths of a second in the low 20
//! bits and the character in the next three; 0x927c0 means "no record".

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Mario Kart 64 save";
const NONE: u32 = 0x9_27c0;
const CHARS: [&str; 8] = ["Mario", "Luigi", "Yoshi", "Toad", "D.K.", "Wario", "Peach", "Bowser"];
const COURSES: [&str; 16] = [
    "Luigi Raceway", "Moo Moo Farm", "Koopa Troopa Beach", "Kalimari Desert", "Toad's Turnpike", "Frappe Snowland", "Choco Mountain",
    "Mario Raceway", "Wario Stadium", "Sherbet Land", "Royal Raceway", "Bowser's Castle", "D.K.'s Jungle Parkway", "Yoshi Valley",
    "Banshee Boardwalk", "Rainbow Road",
];
const CLASSES: [&str; 4] = ["50cc", "100cc", "150cc", "Extra"];
const CUPS: [&str; 4] = ["Mushroom Cup", "Flower Cup", "Star Cup", "Special Cup"];
const PLACES: [&str; 5] = ["1st place", "2nd place", "3rd place", "4th place", "5th place"];

fn stamp(x: u32) -> u16 {
    (((x & 0xff) << 8) | ((0x5a + x) & 0xff)) as u16
}

fn gp_sum(d: &[u8]) -> u16 {
    stamp(d[0x180..0x185].iter().enumerate().map(|(i, &b)| (b as u32 + 1) * (i as u32 + 1) + i as u32).sum())
}

fn best_sum(d: &[u8], from: usize) -> u16 {
    stamp(d[from..from + 0x33].iter().enumerate().map(|(i, &b)| (b as u32 + 1) * (i as u32 / 0x11 + 1) + i as u32 % 0x11).sum())
}

fn course_sum(d: &[u8], s: usize) -> u8 {
    d[s..s + 0x15].iter().enumerate().fold(0u32, |a, (i, &b)| a + b as u32 * (i as u32 % 3 + 1) + i as u32 / 3) as u8
}

pub fn looks(d: &[u8]) -> bool {
    matches!(d.len(), 512 | 2048 | 296_960) && d[0x186..0x188] == gp_sum(d).to_be_bytes()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Mario Kart 64 save".into());
    }
    let chars: Vec<(String, u64)> = CHARS.iter().enumerate().map(|(i, n)| (n.to_string(), i as u64)).collect();
    let mut b = Builder::new(d);
    let modes = [("Stereo".to_string(), 0), ("Headphones".to_string(), 1), ("Mono".to_string(), 3)];
    b.pick(&["Options", "Sound mode"], 0x184, Kind::U8, &modes);
    let medals = [("None".to_string(), 0), ("Bronze".to_string(), 1), ("Silver".to_string(), 2), ("Gold".to_string(), 3)];
    for (c, class) in CLASSES.iter().enumerate() {
        for (n, cup) in CUPS.iter().enumerate() {
            b.pick_bits(&["Grand Prix", class, cup], 0x180 + c, Kind::U8, n as u32 * 2, 2, &medals);
        }
    }
    let record = |b: &mut Builder, course: &str, place: &str, at: usize, mirror: Option<usize>| {
        let (t, w) = (format!("{place} time"), format!("{place} character"));
        let time = ["Time Trials", course, t.as_str()];
        let who = ["Time Trials", course, w.as_str()];
        b.bits(&time, at, Kind::U24, 0, 20, 0, 0, NONE as i64);
        b.pick_bits(&who, at, Kind::U24, 20, 3, &chars);
        if let Some(m) = mirror {
            b.mirror(&time, m);
            b.mirror(&who, m);
        }
    };
    for (c, course) in COURSES.iter().enumerate() {
        let s = c * 0x18;
        let best = if c < 8 { 0x188 } else { 0x1c0 } + (c % 8) * 3;
        for (p, place) in PLACES.iter().enumerate() {
            record(&mut b, course, place, s + p * 3, (p == 0).then_some(best));
        }
        record(&mut b, course, "Best lap", s + 0xf, Some(best + 0x18));
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. A time of 600000 (100 minutes) means no record. The first place and best lap of each course are copied to the game's best-records list."))
}

fn fix(d: &mut Vec<u8>) {
    for c in 0..16 {
        let s = c * 0x18;
        let any = (0..6).any(|i| {
            let o = s + if i == 5 { 0xf } else { i * 3 };
            (u32::from_le_bytes([d[o], d[o + 1], d[o + 2], 0])) != NONE
        });
        d[s + 0x12] = u8::from(any);
        d[s + 0x17] = course_sum(d, s);
    }
    let g = gp_sum(d).to_be_bytes();
    d[0x186..0x188].copy_from_slice(&g);
    for (from, at) in [(0x188, 0x1be), (0x1c0, 0x1f6)] {
        let v = best_sum(d, from).to_be_bytes();
        d[at..at + 2].copy_from_slice(&v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_sums_and_copy_best_records() {
        let mut d = vec![0u8; 2048];
        for c in 0..16 {
            for i in 0..6 {
                let o = c * 0x18 + i * 3;
                d[o..o + 3].copy_from_slice(&NONE.to_le_bytes()[..3]);
            }
        }
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Time Trials/Mario Raceway/1st place time", 5432.into()).unwrap();
        doc.set("/Time Trials/Mario Raceway/1st place character", "Wario".into()).unwrap();
        doc.set("/Grand Prix/150cc/Star Cup", "Gold".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let s = 7 * 0x18;
        assert_eq!(u32::from_le_bytes([out[s], out[s + 1], out[s + 2], 0]), 5432 | 5 << 20);
        assert_eq!(out[s + 0x12], 1);
        assert_eq!(out[s + 0x17], course_sum(&out, s));
        assert_eq!(&out[0x188 + 21..0x188 + 24], &out[s..s + 3]);
        assert_eq!(out[0x182] >> 4 & 3, 3);
        assert!(looks(&out));
        assert_eq!(out[0x1be..0x1c0], best_sum(&out, 0x188).to_be_bytes());
        assert!(parse(&out).is_ok());
    }

    /// `SAVE_SAMPLES=<dir>`: the sums computed here match those of real saves.
    #[test]
    #[ignore]
    fn sums_match_real_saves() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if looks(&d) {
                let mut x = d.clone();
                fix(&mut x);
                let diff: Vec<usize> = (0..d.len()).filter(|&i| x[i] != d[i]).collect();
                assert!(diff.is_empty(), "{:?}: {:x?}", e.path(), diff);
            }
        }
    }
}
