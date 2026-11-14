//! Resident Evil (PlayStation 1, Director's Cut included) saves: one 8 KiB block per save.
//! Offsets and item names follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection
//! (see THIRD_PARTY_NOTICES.md). The save has no checksum.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, opts, title};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Resident Evil save";

pub fn looks(d: &[u8]) -> bool {
    if !is_save(d, 1) {
        return false;
    }
    let t = title(d);
    if !(t.starts_with("RESIDENT EVIL") || t.starts_with("BIO") || t.starts_with("R?E?DC")) {
        return false;
    }
    // Resident Evil 2 and 3 saves carry similar titles; this layout has to read sensibly.
    let hp = u16::from_le_bytes([d[0x21e], d[0x21f]]);
    (1..=8).contains(&d[0x207]) && (1..=140).contains(&hp) && (0..8).all(|i| d[0x324 + 2 * i] <= 75)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Resident Evil save".into());
    }
    let mut b = Builder::new(d);
    let items: Vec<(String, u64)> = ITEMS.iter().map(|(i, n)| (n.to_string(), *i as u64)).collect();
    b.info(&["Info", "Save title"], title(d));
    b.pick_bits(&["General", "Character"], 0x22b, Kind::U8, 0, 1, &opts(&[("Chris Redfield", 0), ("Jill Valentine", 1)]));
    b.ranged(&["General", "Health"], 0x21e, Kind::U16, 1, 140);
    b.ranged(&["General", "Ink ribbons used"], 0x228, Kind::U8, 0, 99);
    b.pick_bits(&["General", "Condition"], 0x232, Kind::U8, 1, 1, &opts(&[("Healthy", 0), ("Poisoned", 1)]));
    b.ranged(&["General", "Inventory slots"], 0x207, Kind::U8, 1, 8);
    flag(&mut b, &["General", "Communication radio"], 0x2ac, 0);
    flag(&mut b, &["General", "Alternative outfit"], 0x2a6, 5);
    flag(&mut b, &["General", "Unlimited rocket launcher"], 0x2ac, 1);
    for i in 0..8 {
        let n = format!("Slot {}", i + 1);
        b.pick(&["Inventory", &n, "Item"], 0x324 + i * 2, Kind::U8, &items);
        b.ranged(&["Inventory", &n, "Quantity"], 0x325 + i * 2, Kind::U8, 0, 250);
    }
    for i in 0..48 {
        let n = format!("Slot {}", i + 1);
        b.pick(&["Item box", &n, "Item"], 0x2c4 + i * 2, Kind::U8, &items);
        b.ranged(&["Item box", &n, "Quantity"], 0x2c5 + i * 2, Kind::U8, 0, 250);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. Health is capped at 140, the game's own maximum; the quantity is the ammunition or the number of uses.")))
}

const ITEMS: &[(u8, &str)] = &[
    (0, "-"),
    (1, "Combat Knife"),
    (2, "Beretta"),
    (3, "Shotgun"),
    (4, "Colt Python (Dumdum Rounds)"),
    (5, "Colt Python (Magnum Rounds)"),
    (6, "Flamethrower"),
    (7, "Bazooka (Explosive R.)"),
    (8, "Bazooka (Acid Rounds)"),
    (9, "Bazooka (Flame Rounds)"),
    (10, "R. Launcher"),
    (11, "Clip"),
    (12, "Shells"),
    (13, "Dumdum Rounds"),
    (14, "Magnum Rounds"),
    (15, "Fuel"),
    (16, "Explosive R."),
    (17, "Acid Rounds"),
    (18, "Flame Rounds"),
    (19, "Empty Bottle"),
    (20, "Water"),
    (21, "UMB No. 2"),
    (22, "UMB No. 4"),
    (23, "UMB No. 7"),
    (24, "UMB No. 13"),
    (25, "Yellow-6"),
    (26, "NP-003"),
    (27, "V-Jolt"),
    (28, "Broken Shotgun"),
    (29, "Square Crank"),
    (30, "Hex. Crank"),
    (31, "Emblem"),
    (32, "Gold Emblem"),
    (33, "Blue Jewel"),
    (34, "Red Jewel"),
    (35, "Music Notes"),
    (36, "Wolf Medal"),
    (37, "Eagle Medal"),
    (38, "Herbicide"),
    (39, "Battery"),
    (40, "MO Disk"),
    (41, "Wind Crest"),
    (42, "Flare"),
    (43, "Slides"),
    (44, "Moon Crest"),
    (45, "Star Crest"),
    (46, "Sun Crest"),
    (47, "Ink Ribbon"),
    (48, "Lighter"),
    (49, "Lockpick"),
    (51, "Sword Key"),
    (52, "Armor Key"),
    (53, "Shield Key"),
    (54, "Helmet Key"),
    (55, "Master Key"),
    (56, "Closet Key"),
    (57, "002 Key"),
    (58, "003 Key"),
    (59, "C. Room Key"),
    (60, "P. Room Key"),
    (61, "Desk Key"),
    (62, "Blank Book"),
    (63, "Doom Book 2"),
    (64, "Doom Book 1"),
    (65, "F.-Aid Spray"),
    (66, "Serum"),
    (67, "Red Herb"),
    (68, "Green Herb"),
    (69, "Blue Herb"),
    (70, "Mixed Herbs (Green + Red)"),
    (71, "Mixed Herbs (Green + Green)"),
    (72, "Mixed Herbs (Green + Blue)"),
    (73, "Mixed Herbs (Green + Red + Blue)"),
    (74, "Mixed Herbs (Green + Green + Green)"),
    (75, "Mixed Herbs (Green + Green + Blue)"),
];
