//! Tony Hawk's Skateboarding (PlayStation 1) saves: one 8 KiB block per save. Offsets follow the
//! MIT-licensed save editor in RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).
//! A 16-bit sum of every byte plus one over 0x280..0xc80 sits at 0x236; the tape count of each
//! skater is derived from the tape flags and written again.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, opts, title};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Tony Hawk's Skateboarding save";

fn sum(d: &[u8]) -> u16 {
    d[0x280..0xc80].iter().fold(0u32, |a, &b| a + b as u32 + 1) as u16
}

pub fn looks(d: &[u8]) -> bool {
    is_save(d, 1) && title(d).starts_with("THPS") && u16::from_le_bytes([d[0x236], d[0x237]]) == sum(d)
}

const SKATERS: [&str; 11] = ["Tony Hawk", "Bob Burnquist", "Geoff Rowley", "Bucky Lasek", "Chad Muska", "Kareem Campbell", "Andrew Reynolds", "Rune Glifberg", "Jamie Thomas", "Elissa Steamer", "Officer Dick"];
const COURSES: [&str; 9] = ["Warehouse", "School", "Mall", "Skate Park", "Downtown", "Downhill Jam", "Burnside", "Streets", "Roswell"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Tony Hawk's Skateboarding save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Info", "Save title"], title(d));
    let medals = opts(&[("-", 0), ("Gold", 1), ("Silver", 2), ("Bronze", 4)]);
    for (i, n) in SKATERS.iter().enumerate() {
        let s = 0xe * i;
        for (level, tasks) in LEVELS {
            for (at, bit, label) in *tasks {
                flag(&mut b, &[n, level, label], s + at, *bit);
            }
        }
        for (course, at) in [("Skate Park", 0x28f), ("Burnside", 0x292), ("Roswell", 0x294)] {
            b.pick_bits(&["Competitions", n, course], s + at, Kind::U8, 5, 3, &medals);
        }
    }
    for (i, c) in COURSES.iter().enumerate() {
        b.ranged(&["High scores", c], 0x32c + 0x60 * i, Kind::U32, 0, 999_999);
    }
    b.ranged(&["Options", "Sound effects level"], 0xab0, Kind::U8, 0, 12);
    b.ranged(&["Options", "Music level"], 0xab4, Kind::U8, 0, 12);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. The tape count of each skater and the save's checksum are written again.")))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() < 0xc80 {
        return;
    }
    for i in 0..11 {
        let s = 0xe * i;
        let n: u32 = (0..8).map(|k| (d[s + 0x28c + k] & 0x1f).count_ones()).sum();
        d[s + 0x288] = n as u8;
    }
    let c = sum(d);
    d[0x236..0x238].copy_from_slice(&c.to_le_bytes());
}

const LEVELS: &[(&str, &[(usize, u32, &str)])] = &[
    ("Warehouse", &[(0x28c, 0, "5,000 Points"), (0x28c, 1, "Smash the Boxes"), (0x28c, 2, "Get S-K-A-T-E"), (0x28c, 3, "Hidden Tape"), (0x28c, 4, "15,000 Points")]),
    ("School", &[(0x28d, 0, "7,500 Points"), (0x28d, 1, "Grind 5 Tables"), (0x28d, 2, "Get S-K-A-T-E"), (0x28d, 3, "Hidden Tape"), (0x28d, 4, "25,000 Points")]),
    ("Mall", &[(0x28e, 0, "10,000 Points"), (0x28e, 1, "Destroy 5 Directories"), (0x28e, 2, "Get S-K-A-T-E"), (0x28e, 3, "Hidden Tape"), (0x28e, 4, "30,000 Points")]),
    ("Downtown", &[(0x290, 0, "15,000 Points"), (0x290, 1, "Break 5 No Skating Signs"), (0x290, 2, "Get S-K-A-T-E"), (0x290, 3, "Hidden Tape"), (0x290, 4, "40,000 Points")]),
    ("Downhill Jam", &[(0x291, 0, "20,000 Points"), (0x291, 1, "Open 5 Valves"), (0x291, 2, "Get S-K-A-T-E"), (0x291, 3, "Hidden Tape"), (0x291, 4, "40,000 Points")]),
    ("Streets", &[(0x293, 0, "25,000 Points"), (0x293, 1, "Wreck 5 Cop Cars"), (0x293, 2, "Get S-K-A-T-E"), (0x293, 3, "Hidden Tape"), (0x293, 4, "50,000 Points")]),
];
