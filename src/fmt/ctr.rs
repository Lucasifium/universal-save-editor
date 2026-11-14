//! Crash Team Racing (PlayStation 1) saves: one 8 KiB block per save, four adventure slots in it.
//! Offsets and names follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection
//! (see THIRD_PARTY_NOTICES.md). A 16-bit CRC sits at 0x177e and covers 0x100..0x1780 (its own
//! two bytes counted as zero).

use super::fields::{Builder, Checksum};
use super::ps1::{is_save, title};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Crash Team Racing save";

fn crc(d: &[u8]) -> u16 {
    let mut c: u32 = 0;
    for (i, &b) in d[0x100..0x1780].iter().enumerate() {
        let b = if i + 0x100 >= 0x177e { 0 } else { b };
        for j in (0..8).rev() {
            c <<= 1;
            let x = c & 0x10000 != 0;
            c |= ((b >> j) & 1) as u32;
            if x {
                c ^= 0x11021;
            }
        }
    }
    (c as u16).swap_bytes()
}

pub fn looks(d: &[u8]) -> bool {
    is_save(d, 1) && (title(d).starts_with("CTR") || d[4..6] == [0x83, 0x4e]) && u16::from_le_bytes([d[0x177e], d[0x177f]]) == crc(d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Crash Team Racing save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Info", "Save title"], title(d));
    for (sec, list) in GENERAL {
        for (at, bit, label) in *list {
            flag(&mut b, &["Unlocked", sec, label], *at, *bit);
        }
    }
    for slot in 0..4 {
        let s = 0x50 * slot;
        let sn = format!("Slot {}", slot + 1);
        for (label, at, bit) in [("Gem Stone Valley", 0x110, 4), ("Gem Races", 0x110, 5), ("Glacier Park (from N.Sanity Beach)", 0x110, 6), ("Glacier Park (from The Lost Ruins)", 0x110, 7), ("Citadel City", 0x111, 0)] {
            flag(&mut b, &[&sn, "World doors", label], s + at, bit);
        }
        for (world, group, list) in EVENTS {
            let kind = if list.len() == 5 && list[1].2 == "Sapphire Relic" { "Races" } else { "Other" };
            // The bonus rounds and boss keys repeat in every world.
            let key = if matches!(*group, "Bonus Round" | "Boss Key") { format!("{group} ({world})") } else { group.to_string() };
            for (at, bit, label) in *list {
                flag(&mut b, &[&sn, kind, &key, label], s + at, *bit);
            }
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. Giving a Gold or Platinum relic also gives the lower relics, as the game expects; the save's checksum is written again.")))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() < 0x1780 {
        return;
    }
    for slot in 0..4 {
        let s = 0x50 * slot;
        for (_, _, list) in EVENTS {
            let get = |d: &Vec<u8>, i: usize| d[s + list[i].0] >> list[i].1 & 1 == 1;
            if list.len() == 5 && list[1].2 == "Sapphire Relic" {
                // Platinum implies gold implies sapphire.
                for (hi, lo) in [(3usize, 2usize), (2, 1)] {
                    if get(d, hi) && !get(d, lo) {
                        d[s + list[lo].0] |= 1 << list[lo].1;
                    }
                }
            }
        }
    }
    let c = crc(d);
    d[0x177e..0x1780].copy_from_slice(&c.to_le_bytes());
}

const GENERAL: &[(&str, &[(usize, u32, &str)])] = &[
    ("Unlocked Characters", &[(0x248, 5, "Dr. N. Tropy"), (0x248, 6, "Penta Penguin"), (0x248, 7, "Ripper Roo"), (0x249, 0, "Papu Papu"), (0x249, 1, "Komodo Joe"), (0x249, 2, "Pinstripe"), (0x249, 3, "Fake Crash")]),
    ("Unlocked Levels", &[(0x248, 1, "Turbo Track Unlocked"), (0x248, 2, "Parking Lot"), (0x248, 3, "The North Bowl"), (0x248, 4, "Lab Basement")]),
    ("Unlocked Menus", &[(0x24c, 4, "Scrapbook")]),
];

const EVENTS: &[(&str, &str, &[(usize, u32, &str)])] = &[
    ("N.Sanity Beach", "Crash Cove", &[(0x105, 1, "Trophy"), (0x107, 1, "Sapphire Relic"), (0x109, 3, "Gold Relic"), (0x10b, 5, "Platinum Relic"), (0x10d, 7, "CTR Token")]),
    ("N.Sanity Beach", "Roo's Tubes", &[(0x105, 4, "Trophy"), (0x107, 4, "Sapphire Relic"), (0x109, 6, "Gold Relic"), (0x10c, 0, "Platinum Relic"), (0x10e, 2, "CTR Token")]),
    ("N.Sanity Beach", "Sewer Speedway", &[(0x105, 6, "Trophy"), (0x107, 6, "Sapphire Relic"), (0x10a, 0, "Gold Relic"), (0x10c, 2, "Platinum Relic"), (0x10e, 4, "CTR Token")]),
    ("N.Sanity Beach", "Mystery Caves", &[(0x105, 7, "Trophy"), (0x107, 7, "Sapphire Relic"), (0x10a, 1, "Gold Relic"), (0x10c, 3, "Platinum Relic"), (0x10e, 5, "CTR Token")]),
    ("N.Sanity Beach", "Bonus Round", &[(0x111, 7, "Skull Rock")]),
    ("N.Sanity Beach", "Boss Key", &[(0x10f, 6, "Ripper Roo")]),
    ("The Lost Ruins", "Dingo Canyon", &[(0x104, 6, "Trophy"), (0x106, 6, "Sapphire Relic"), (0x109, 0, "Gold Relic"), (0x10b, 2, "Platinum Relic"), (0x10d, 4, "CTR Token")]),
    ("The Lost Ruins", "Tiger Temple", &[(0x105, 2, "Trophy"), (0x107, 2, "Sapphire Relic"), (0x109, 4, "Gold Relic"), (0x10b, 6, "Platinum Relic"), (0x10e, 0, "CTR Token")]),
    ("The Lost Ruins", "Papu's Pyramid", &[(0x105, 3, "Trophy"), (0x107, 3, "Sapphire Relic"), (0x109, 5, "Gold Relic"), (0x10b, 7, "Platinum Relic"), (0x10e, 1, "CTR Token")]),
    ("The Lost Ruins", "Coco Park", &[(0x106, 4, "Trophy"), (0x108, 4, "Sapphire Relic"), (0x10a, 6, "Gold Relic"), (0x10d, 0, "Platinum Relic"), (0x10f, 2, "CTR Token")]),
    ("The Lost Ruins", "Bonus Round", &[(0x112, 0, "Rampage Ruins")]),
    ("The Lost Ruins", "Boss Key", &[(0x10f, 7, "Papu Papu")]),
    ("Glacier Park", "Dragon Mines", &[(0x104, 7, "Trophy"), (0x106, 7, "Sapphire Relic"), (0x109, 1, "Gold Relic"), (0x10b, 3, "Platinum Relic"), (0x10d, 5, "CTR Token")]),
    ("Glacier Park", "Blizzard Bluff", &[(0x105, 0, "Trophy"), (0x107, 0, "Sapphire Relic"), (0x109, 2, "Gold Relic"), (0x10b, 4, "Platinum Relic"), (0x10d, 6, "CTR Token")]),
    ("Glacier Park", "Polar Pass", &[(0x106, 2, "Trophy"), (0x108, 2, "Sapphire Relic"), (0x10a, 4, "Gold Relic"), (0x10c, 6, "Platinum Relic"), (0x10f, 0, "CTR Token")]),
    ("Glacier Park", "Tiny Arena", &[(0x106, 5, "Trophy"), (0x108, 5, "Sapphire Relic"), (0x10a, 7, "Gold Relic"), (0x10d, 1, "Platinum Relic"), (0x10f, 3, "CTR Token")]),
    ("Glacier Park", "Bonus Round", &[(0x112, 1, "Rocky Road")]),
    ("Glacier Park", "Boss Key", &[(0x110, 0, "Komodo Joe")]),
    ("Citadel City", "Hot Air Skyway", &[(0x105, 5, "Trophy"), (0x107, 5, "Sapphire Relic"), (0x109, 7, "Gold Relic"), (0x10c, 1, "Platinum Relic"), (0x10e, 3, "CTR Token")]),
    ("Citadel City", "Cortex Castle", &[(0x106, 0, "Trophy"), (0x108, 0, "Sapphire Relic"), (0x10a, 2, "Gold Relic"), (0x10c, 4, "Platinum Relic"), (0x10e, 6, "CTR Token")]),
    ("Citadel City", "N.Gin Labs", &[(0x106, 1, "Trophy"), (0x108, 1, "Sapphire Relic"), (0x10a, 3, "Gold Relic"), (0x10c, 5, "Platinum Relic"), (0x10e, 7, "CTR Token")]),
    ("Citadel City", "Oxide Station", &[(0x106, 3, "Trophy"), (0x108, 3, "Sapphire Relic"), (0x10a, 5, "Gold Relic"), (0x10c, 7, "Platinum Relic"), (0x10f, 1, "CTR Token")]),
    ("Citadel City", "Bonus Round", &[(0x112, 2, "Nitro Court")]),
    ("Citadel City", "Boss Key", &[(0x110, 1, "Pinstripe")]),
    ("Gem Stone Valley", "Slide Coliseum", &[(0x108, 6, "Sapphire Relic"), (0x10b, 0, "Gold Relic"), (0x10d, 2, "Platinum Relic")]),
    ("Gem Stone Valley", "Turbo Track", &[(0x108, 7, "Sapphire Relic"), (0x10b, 1, "Gold Relic"), (0x10d, 3, "Platinum Relic")]),
    ("Gem Stone Valley", "Gems", &[(0x111, 2, "Red Gem"), (0x111, 3, "Green Gem"), (0x111, 4, "Cyan Gem"), (0x111, 5, "Yellow Gem"), (0x111, 6, "Magenta Gem")]),
    ("Gem Stone Valley", "Boss Challenges", &[(0x110, 2, "N. Oxide's Challenge"), (0x112, 3, "N. Oxide's Challenge (percent)"), (0x110, 3, "N. Oxide's Final Challenge"), (0x112, 4, "N. Oxide's Final Challenge (percent)")]),
];
