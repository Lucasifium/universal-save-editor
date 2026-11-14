//! Silent Hill (PlayStation 1) saves: one 8 KiB block per memory card file, holding up to eleven
//! save slots. Offsets and item names follow the MIT-licensed save editor in
//! RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md). Every part of the block has
//! its own checksum (all bytes XORed, the result stored in both bytes of a 16-bit field), which
//! is written again on save, together with the copy of the save count and location in the
//! slot's preview.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, opts, title};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Silent Hill save";

const PREVIEWS: usize = 0x204;
const OPTIONS: usize = 0x300;
const SLOTS: usize = 0x380;
const SLOT_LEN: usize = 0x280;

fn xor16(d: &[u8], from: usize, to: usize) -> [u8; 2] {
    let x = d[from..to].iter().fold(0u8, |a, b| a ^ b);
    [x, x]
}

fn count(d: &[u8]) -> usize {
    (0..11).take_while(|i| d[PREVIEWS + i * 0xc..PREVIEWS + i * 0xc + 4] != [0; 4]).count()
}

pub fn looks(d: &[u8]) -> bool {
    if !is_save(d, 1) {
        return false;
    }
    // "ＳＩＬＥＮＴ" in full-width Latin letters, or the Japanese "サイレント".
    let t = &d[4..16];
    if !(t.starts_with(&[0x82, 0x72, 0x82, 0x68, 0x82, 0x6b]) || t.starts_with(&[0x83, 0x54, 0x83, 0x43, 0x83, 0x8c])) {
        return false;
    }
    d[OPTIONS + 0x7c..OPTIONS + 0x7e] == xor16(d, OPTIONS, OPTIONS + 0x7c) && count(d) > 0
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Silent Hill save".into());
    }
    let mut b = Builder::new(d);
    let items: Vec<(String, u64)> = ITEMS.iter().map(|(i, n)| (n.to_string(), *i as u64)).chain([("-".to_string(), 0xff)]).collect();
    b.info(&["Info", "Save title"], title(d));
    b.ranged(&["Options", "Brightness level"], OPTIONS + 0x22, Kind::U8, 0, 7);
    b.pick(&["Options", "Sound"], OPTIONS + 0x1e, Kind::U8, &opts(&[("Stereo", 0), ("Monaural", 1)]));
    b.pick(&["Options", "Blood color"], OPTIONS + 0x24, Kind::U8, &opts(&[("Normal", 0), ("Green", 2), ("Violet", 5), ("Black", 0xb)]));
    b.pick(&["Options", "Bullet adjust"], OPTIONS + 0x2d, Kind::U8, &opts(&[("x1", 0), ("x2", 1), ("x3", 2), ("x4", 3), ("x5", 4), ("x6", 5)]));
    for (i, n) in ["Good+", "Good", "Bad+", "Bad", "UFO"].iter().enumerate() {
        flag(&mut b, &["Unlocked endings", n], OPTIONS + 0x27, i as u32);
    }
    for i in 0..count(d) {
        let s = SLOTS + i * SLOT_LEN;
        let slot = format!("Slot {}", i + 1);
        let g = |n: &'static str| -> [String; 3] { [slot.clone(), "General".to_string(), n.to_string()] };
        let nums: [(&str, usize, Kind, i64, i64); 9] = [
            ("Health", 0x242, Kind::U8, 0, 6),
            ("Saves", 0xa6, Kind::U16, 0, 999),
            ("Continues", 0x27b, Kind::U8, 0, 99),
            ("Game clear", 0x24a, Kind::U8, 0, 99),
            ("Inventory slots", 0xab, Kind::U8, 8, 40),
            ("Total shots", 0x264, Kind::U16, 0, 50000),
            ("Short range shots", 0x266, Kind::U16, 0, 50000),
            ("Middle range shots", 0x268, Kind::U16, 0, 50000),
            ("Long range shots", 0x26a, Kind::U16, 0, 50000),
        ];
        let d3 = g("Difficulty");
        b.pick(&[&d3[0], &d3[1], &d3[2]], s + 0x263, Kind::U8, &opts(&[("Easy", 0xf0), ("Normal", 0), ("Hard", 0x10)]));
        for (n, at, k, lo, hi) in nums {
            let q = g(n);
            b.ranged(&[&q[0], &q[1], &q[2]], s + at, k, lo, hi);
        }
        for j in 0..40 {
            let n = format!("Slot {}", j + 1);
            b.pick(&[&slot, "Inventory", &n, "Item"], s + 0xac + j * 4, Kind::U8, &items);
            b.ranged(&[&slot, "Inventory", &n, "Quantity"], s + 0xad + j * 4, Kind::U8, 0, 255);
        }
        for (_sec, list) in FLAGS {
            for (n, at, bit) in *list {
                flag(&mut b, &[&slot, "Extra items", n], s + at, *bit);
            }
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. The checksums of the options, the slots and the save previews are written again on save.")))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() < 0x2000 {
        return;
    }
    let n = count(d);
    for i in 0..n {
        let s = SLOTS + i * SLOT_LEN;
        let pv = PREVIEWS + i * 0xc;
        // The preview repeats the save count and the save-point location.
        let (a, b) = (d[s + 0xa6], d[s + 0xa7]);
        d[pv + 8] = a;
        d[pv + 9] = b;
        d[pv + 0xa] = d[s + 0xa8];
        let c = xor16(d, s, s + 0x27c);
        d[s + 0x27c..s + 0x27e].copy_from_slice(&c);
    }
    let c = xor16(d, PREVIEWS, PREVIEWS + 0xf8);
    d[PREVIEWS + 0xf8..PREVIEWS + 0xfa].copy_from_slice(&c);
    let c = xor16(d, OPTIONS, OPTIONS + 0x7c);
    d[OPTIONS + 0x7c..OPTIONS + 0x7e].copy_from_slice(&c);
}

const FLAGS: &[(&str, &[(&str, usize, u32)])] = &[("Extra items", &[
    ("Katana room unlocked", 0x16f, 1),
    ("Gasoline tank available", 0x16f, 6),
    ("Channeling stone available", 0x16f, 2),
    ("Hyper Blaster available", 0x24b, 4),
])];

const ITEMS: &[(u8, &str)] = &[
    (0x20, "Health drink"),
    (0x21, "First aid kit"),
    (0x22, "Ampoule"),
    (0xa0, "Handgun"),
    (0x80, "Kitchen knife"),
    (0x81, "Steel pipe"),
    (0x86, "Katana"),
    (0xa2, "Shotgun"),
    (0x85, "Chainsaw"),
    (0x82, "Rock drill"),
    (0x84, "Hammer"),
    (0x87, "Axe"),
    (0xa1, "Hunting rifle"),
    (0xa3, "Hyper Blaster"),
    (0xc0, "Handgun bullets"),
    (0xc2, "Shotgun shells"),
    (0xc1, "Rifle shells"),
    (0xe0, "Flashlight"),
    (0xe1, "Pocket radio"),
    (0x53, "A note \"To school\""),
    (0x54, "A note \"Doghouse\""),
    (0x41, "House key"),
    (0x42, "Key of \"Lion\""),
    (0x43, "Key of \"Woodman\""),
    (0x44, "Key of \"Scarecrow\""),
    (0x60, "Chemical"),
    (0x61, "Gold medallion"),
    (0x62, "Silver medallion"),
    (0x63, "Rubber ball"),
    (0x55, "Picture card"),
    (0x45, "Library reserve key"),
    (0x46, "Classroom key"),
    (0x47, "K. Gordon key"),
    (0x64, "\"Flauros\""),
    (0x48, "Drawbridge key"),
    (0x49, "Basement key"),
    (0x65, "Plastic bottle"),
    (0x66, "Unknown liquid"),
    (0x67, "Plate of \"Turtle\""),
    (0x68, "Plate of \"Hatter\""),
    (0x69, "Plate of \"Cat\""),
    (0x6a, "Plate of \"Queen\""),
    (0x6b, "Blood pack"),
    (0x6d, "Lighter"),
    (0x6c, "Disinfecting alcohol"),
    (0x4a, "Basement storeroom key"),
    (0x6e, "Video tape"),
    (0x4b, "Examination room key"),
    (0x4c, "Antique shop key"),
    (0x4d, "Sewer key"),
    (0x57, "Sewer exit key"),
    (0x70, "Kaufmann key"),
    (0x71, "Receipt"),
    (0x72, "Safe key"),
    (0x73, "Magnet"),
    (0x74, "Motorcycle key"),
    (0x77, "Screwdriver"),
    (0x76, "Pliers"),
    (0x4e, "Key of \"Ophiel\""),
    (0x4f, "Key of \"Hagith\""),
    (0x50, "Key of \"Phaleg\""),
    (0x51, "Key of \"Bethor\""),
    (0x52, "Key of \"Aratron\""),
    (0x7a, "Stone of time"),
    (0x79, "Ring of contract"),
    (0x78, "Camera"),
    (0x75, "Bird cage key"),
    (0x40, "Lobby key"),
    (0x7b, "Amulet of Solomon"),
    (0x7c, "Crest of Mercury"),
    (0x7d, "Ankh"),
    (0x7e, "Dagger of Melchior"),
    (0x7f, "Disk of Ouroboros"),
    (0x58, "Channeling stone"),
    (0xe2, "Gasoline tank"),
];
