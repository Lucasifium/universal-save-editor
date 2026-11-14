//! Rayman (PlayStation 1) saves: one 8 KiB block per save. Offsets follow the MIT-licensed save
//! editor in RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md). The save has no
//! checksum; the per-level totals and the completion rate in the title are derived from the
//! unlock and cage flags and are written again.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, opts, title};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Rayman save";

pub fn looks(d: &[u8]) -> bool {
    is_save(d, 1) && title(d).starts_with("Rayman pas")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Rayman save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Info", "Save title"], title(d));
    b.ranged(&["General", "Continues"], 0x200, Kind::U8, 0, 99);
    b.ranged(&["General", "Lives"], 0x400, Kind::U8, 0, 99);
    b.pick(&["General", "Maximum health"], 0x409, Kind::U8, &opts(&[("3 points", 2), ("5 points", 4)]));
    b.bits(&["General", "Health"], 0x480, Kind::U8, 0, 8, 1, 1, 5);
    b.ranged(&["General", "Tings"], 0x406, Kind::U8, 0, 99);
    for (n, at) in [("Punch", (0x300, 0)), ("Hang", (0x300, 1)), ("Helicopter", (0x300, 2)), ("Grab", (0x300, 7)), ("Run", (0x301, 0))] {
        flag(&mut b, &["Powers", n], at.0, at.1);
    }
    for (name, unlock, cages) in LEVELS {
        flag(&mut b, &["Levels", name, "Unlocked"], *unlock, 0);
        for (i, (at, bit)) in cages.iter().enumerate() {
            flag(&mut b, &["Levels", name, &format!("Electoon cage {}", i + 1)], *at, *bit);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. The level totals and the completion rate in the save's title are written again from the unlock and cage flags.")))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() < 0x2000 {
        return;
    }
    let mut cages = 0u32;
    for (_, unlock, list) in LEVELS.iter().filter(|l| !l.2.is_empty()) {
        let n = list.iter().filter(|(at, bit)| d[*at] >> bit & 1 == 1).count() as u32;
        cages += n;
        d[*unlock] = (d[*unlock] & 1) | (n as u8 * 4);
    }
    let pct = format!("{:03}", cages * 100 / 102);
    d[0xf..0x12].copy_from_slice(pct.as_bytes());
}

const LEVELS: &[(&str, usize, &[(usize, u32)])] = &[
    ("Pink Plant Woods", 0x280, &[(0x521, 6), (0x529, 2), (0x529, 4), (0x529, 5), (0x56a, 5), (0x56a, 7)]),
    ("Anguish Lagoon", 0x281, &[(0x583, 0), (0x584, 3), (0x584, 5), (0x584, 6), (0x584, 7), (0x5c3, 6)]),
    ("The Swamps of Forgetfulness", 0x282, &[(0x600, 4), (0x600, 5), (0x626, 1), (0x627, 7), (0x648, 1), (0x648, 2)]),
    ("Moskito's Nest", 0x283, &[(0x667, 2), (0x667, 3), (0x667, 6), (0x689, 0), (0x689, 1), (0x689, 3)]),
    ("Bongo Hills", 0x284, &[(0x7aa, 0), (0x7d3, 5), (0x7e8, 3), (0x810, 3), (0x832, 0), (0x832, 1)]),
    ("Allegro Presto", 0x285, &[(0x860, 2), (0x860, 4), (0x860, 5), (0x88f, 7), (0x8a1, 0), (0x8a1, 2)]),
    ("Gong Heights", 0x286, &[(0x900, 2), (0x900, 4), (0x900, 5), (0x900, 7), (0x920, 3), (0x920, 6)]),
    ("Mr Sax's Hullaballoo", 0x287, &[(0x954, 2), (0x954, 3), (0x954, 4), (0x954, 5), (0x954, 6), (0x954, 7)]),
    ("Twilight Gulch", 0x288, &[(0x9ea, 1), (0x9ea, 2), (0x9eb, 4), (0x9eb, 5), (0x9eb, 6), (0x9eb, 7)]),
    ("The Hard Rocks", 0x289, &[(0xa37, 6), (0xa44, 5), (0xa44, 6), (0xa69, 0), (0xa69, 1), (0xa6a, 6)]),
    ("Mr Stone's Peaks", 0x28a, &[(0xa90, 0), (0xa90, 1), (0xaa5, 7), (0xac6, 1), (0xaf3, 0), (0xaf4, 7)]),
    ("Eraser Plains", 0x28b, &[(0xb93, 3), (0xb93, 4), (0xbb2, 3), (0xbcf, 1), (0xbcf, 2), (0xbd0, 7)]),
    ("Pencil Pentathlon", 0x28c, &[(0xc17, 0), (0xc18, 7), (0xc2a, 0), (0xc2a, 2), (0xc4b, 1), (0xc4c, 7)]),
    ("Space Mama's Crater", 0x28d, &[(0xc68, 2), (0xc68, 3), (0xc81, 0), (0xc81, 2), (0xcb3, 4), (0xcb3, 5)]),
    ("Crystal Palace", 0x28e, &[(0xd2c, 1), (0xd2c, 4), (0xd2c, 5), (0xd4b, 0), (0xd4c, 6), (0xd4c, 7)]),
    ("Eat At Joe's", 0x28f, &[(0xd87, 3), (0xdb1, 1), (0xdb1, 3), (0xdb1, 4), (0xde0, 6), (0xe07, 4)]),
    ("Mr Skops' Stalactites", 0x290, &[(0xe20, 1), (0xe20, 2), (0xe20, 3), (0xe20, 4), (0xe20, 5), (0xe20, 6)]),
    ("Mr Dark's Dare", 0x291, &[]),
    ("The Dream Forest (Save)", 0x292, &[]),
    ("Band Land (Save)", 0x293, &[]),
    ("Blue Mountains (Save)", 0x294, &[]),
    ("Picture City (Save)", 0x295, &[]),
    ("The Caves of Skops (Save)", 0x296, &[]),
    ("Candy Chateau (Save)", 0x297, &[]),
];
