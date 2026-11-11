//! NieR:Automata slot saves (`SlotData_N.dat`, 235,980 bytes, no checksum).
//!
//! Fixed places: play time (seconds) at 0x24, chapter at 0x2C, the name (UTF-16) at 0x34,
//! the world state as text from 0x7C (`version v0.0.2 …`), money at 0x3056C, the
//! inventory at 0x30570 (256 slots of item id, status 0x70000 when used, count; a second
//! 256 for the items left on your body after a death follow at 0x31170), the 39 weapons
//! at 0x31D70 (id, level 1–4, three counters) and experience at 0x3871C.
//!
//! Layout, item and weapon names and the experience table from NieR.Automata.Editor
//! (Alexandre Quoniou, MIT), matching the offsets in atilion's save editor (MIT) and
//! Aemony's level editor (MIT); checked on a public real save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "NieR:Automata save";
pub const SIZE: usize = 0x399CC;
const WORLD: usize = 0x7C;
const MONEY: usize = 0x3056C;
const ITEMS: usize = 0x30570;
const WEAPONS: usize = 0x31D70;
const XP: usize = 0x3871C;
const USED: u32 = 0x0007_0000;

/// Experience needed for levels 1 to 99.
pub const LEVEL_XP: [i64; 99] = [
    0, 48, 139, 294, 525, 843, 1259, 1782, 2421, 3184, 4080, 5116, 6300, 7638, 9139, 10809, 12654,
    14682, 16898, 19309, 21920, 24739, 27770, 31019, 34493, 38196, 42134, 46312, 50736, 55412,
    60343, 65536, 70994, 76724, 82730, 89017, 95590, 102453, 109611, 117070, 124832, 132904,
    141288, 149991, 159016, 168368, 178051, 188068, 198426, 209127, 220177, 231578, 243336, 255454,
    267937, 280788, 294011, 307611, 321591, 335956, 350709, 365854, 381395, 397336, 413680, 430431,
    447594, 465171, 483167, 501585, 520429, 539702, 559408, 579552, 600135, 621162, 642637, 664562,
    686942, 709780, 733079, 756843, 781075, 805779, 830958, 856615, 882754, 909379, 936491, 964096,
    992196, 1020794, 1049894, 1079499, 1109612, 1140237, 1171376, 1203033, 1235211,
];

/// The level a total of experience gives.
pub fn level_for(xp: i64) -> usize {
    LEVEL_XP.iter().filter(|&&n| n <= xp).count().max(1)
}

/// Item ids and names.
pub const ITEM_NAMES: &[(u32, &str)] = &[
    (0, "Small Recovery"),
    (1, "Medium Recovery"),
    (2, "Large Recovery"),
    (3, "Full Recovery"),
    (50, "Volt-Proof Salve"),
    (60, "Visual Cure"),
    (70, "Aural Cure"),
    (75, "Cure Manipulation"),
    (80, "Cure All Status"),
    (90, "Cure All + Heal All"),
    (100, "Melee Attack Up (S)"),
    (102, "Melee Attack Up (L)"),
    (110, "Ranged Attack Up (S)"),
    (112, "Ranged Attack Up (L)"),
    (120, "Melee Defense Up (S)"),
    (122, "Melee Defense Up (L)"),
    (130, "Ranged Defense Up (S)"),
    (132, "Ranged Defense Up (L)"),
    (140, "Skill Salve (S)"),
    (142, "Skill Salve (L)"),
    (150, "Impact Bracer (S)"),
    (152, "Impact Bracer (L)"),
    (160, "Speed Salve (S)"),
    (162, "Speed Salve (L)"),
    (170, "Popola's Booze"),
    (300, "Animal Bait"),
    (330, "Small G Luck+"),
    (332, "Large G Luck+"),
    (400, "E-Drug"),
    (410, "Forbidden Fruit"),
    (510, "Beast Hide"),
    (515, "Boar Meat"),
    (516, "Moose Meat"),
    (517, "W. Boar Meat"),
    (518, "W. Moose Meat"),
    (520, "Stripped Screw"),
    (521, "Pristine Screw"),
    (525, "Small Gear"),
    (526, "Large Gear"),
    (530, "Rusted Clump"),
    (531, "Meteorite Shard"),
    (535, "Titanium Alloy"),
    (536, "Memory Alloy"),
    (540, "Broken Key"),
    (545, "Broken Battery"),
    (546, "Large Battery"),
    (550, "Severed Cable"),
    (551, "Pristine Cable"),
    (555, "New Bolt"),
    (560, "Clean Nut"),
    (570, "Machine Arm"),
    (571, "Machine Leg"),
    (572, "Machine Torso"),
    (573, "Machine Head"),
    (580, "Simple Gadget"),
    (581, "Elaborate Gadget"),
    (582, "Complex Gadget"),
    (583, "Powerup Part S"),
    (584, "Powerup Part M"),
    (585, "Powerup Part L"),
    (600, "Copper Ore"),
    (601, "Iron Ore"),
    (602, "Silver Ore"),
    (603, "Gold Ore"),
    (610, "Dented Plate"),
    (621, "Plant Seed"),
    (622, "Tree Sap"),
    (623, "Pure Water"),
    (624, "Tree Seed"),
    (625, "Desert Rose"),
    (626, "Mushroom"),
    (627, "Torn Book"),
    (628, "Machine Oil"),
    (629, "Tanning Agent"),
    (630, "Filler Metal"),
    (631, "Dye"),
    (632, "Pyrite"),
    (633, "Amber"),
    (634, "Natural Rubber"),
    (635, "Rusty Bolt"),
    (636, "Crushed Nut"),
    (637, "Giant Egg"),
    (638, "Eagle Eggs"),
    (639, "Black Pearl"),
    (640, "Shattered Earring"),
    (641, "Drab Bracelet"),
    (642, "Sturdy Socket"),
    (643, "Warped Wire"),
    (644, "Stretched Coil"),
    (645, "Dented Socket"),
    (646, "Broken Circuit"),
    (680, "Crystal"),
    (681, "Lovely Choker"),
    (682, "Ancient Mask"),
    (683, "Precious Earrings"),
    (684, "Tech Manual"),
    (685, "Thick Dictionary"),
    (686, "Pearl"),
    (687, "Moldavite"),
    (688, "Meteorite"),
    (690, "Machine Core"),
    (691, "Pascal's Core"),
    (692, "Children's Cores"),
    (693, "Pascal's Book"),
    (700, "Letter for Sartre"),
    (701, "Beautiful Glass"),
    (702, "Machine Fossil"),
    (703, "Sartre's Letter"),
    (715, "Memory Chip"),
    (716, "Man's Journal"),
    (720, "Data Chip A"),
    (721, "Data Chip B"),
    (722, "Data Chip C"),
    (723, "Data Chip D"),
    (724, "Data Chip E"),
    (740, "Rusty Dog Tag"),
    (741, "Dirty Dog Tag"),
    (742, "Damaged Dog Tag"),
    (743, "Bloody Dog Tag"),
    (750, "Rusty Music Box"),
    (760, "Desert Photo"),
    (761, "Amusement Park Photo"),
    (762, "Forest Photo"),
    (775, "Package"),
    (780, "Broken Toy"),
    (781, "Accounting Book"),
    (782, "Small Shoe"),
    (790, "Tri-color Cable"),
    (791, "Four-color Cable"),
    (792, "Five-color Cable"),
    (793, "Toothbrush"),
    (794, "Cosmetics"),
    (795, "Dietary Goods"),
    (796, "Writing Implement"),
    (797, "Medical Journal"),
    (820, "Forest Access Key"),
    (821, "Ocean Access Key"),
    (822, "Park Access Key"),
    (832, "Machine Part"),
    (833, "Stamp"),
    (834, "Stamp Card"),
    (835, "Aged Stick"),
    (836, "Commandment Slab"),
    (837, "Filthy Mask"),
    (838, "Statue of a Girl"),
    (839, "Toy Material"),
    (840, "Storage Element"),
    (841, "Plug-in Chip"),
    (842, "Rigid Tree Bark"),
    (843, "Membership Card"),
    (844, "Dirty Bag"),
    (845, "Elevator Key"),
    (850, "Book"),
    (851, "Fuel Filter"),
    (852, "Viscous Oil"),
    (870, "Valve: Head"),
    (871, "Camouflage Goggles"),
    (872, "A2 Wig"),
    (873, "Pink Ribbon"),
    (874, "Blue Ribbon"),
    (875, "Proof of Oath"),
    (876, "Masamune Mask"),
    (877, "Matsuda Mask"),
    (878, "Sato Mask"),
    (880, "White Hair"),
    (881, "Black Hair"),
    (882, "Brown Hair"),
    (883, "Red Hair"),
    (884, "Blue Hair"),
    (885, "Green Hair"),
    (886, "Purple Hair"),
    (887, "Ash Grey Hair"),
    (888, "Golden Hair"),
    (889, "Pastel Pink Hair"),
    (890, "Light Blue Hair"),
    (891, "Lime Green Hair"),
    (892, "Light Purple Hair"),
    (893, "Neon White Hair"),
    (894, "Neon Yellow Hair"),
    (895, "Neon Pink Hair"),
    (896, "Neon Blue Hair"),
    (897, "Neon Green Hair"),
    (898, "Neon Purple Hair"),
    (920, "Sachet"),
    (921, "Quality Sachet"),
    (922, "Choice Sachet"),
    (930, "Sound Data 1"),
    (931, "Sound Data 2"),
    (932, "Sound Data 3"),
    (933, "Sound Data 4"),
    (934, "Sound Data 5"),
    (935, "Record: 3C3C1D119440927"),
    (936, "Record: CEO"),
    (950, "Play System Pod"),
    (951, "Grimoire Weiss"),
    (952, "Cardboard Pod"),
    (953, "Blue Stripes Pod"),
    (954, "Retro Red Pod"),
    (955, "Retro White Pod"),
    (956, "Retro Black Pod"),
    (957, "Retro Grey Pod"),
    (958, "Retro Purple Pod"),
    (959, "amazarashi Head"),
    (960, "Emil Bullets"),
    (961, "CEO Bullets"),
    (980, "Dress Module"),
    (981, "Heavy Armor A"),
    (982, "Heavy Armor B"),
    (983, "Revealing Outfit"),
    (984, "Young Man's Outfit"),
    (985, "Destroyer Outfit"),
    (990, "Machine Mask"),
    (991, "Emil's Head"),
    (992, "Lunar Tear"),
    (993, "Emil Mask"),
    (994, "Sand Mask"),
    (995, "Adam's Glasses"),
    (996, "Alien Mask"),
    (997, "Valve: Left Eye"),
    (998, "Valve: Right Eye"),
    (999, "Valve: Both Eyes"),
];

/// The items in the game's Materials list (upgrade and selling materials), less Viscous
/// Oil (a quest item kept there) and the Children's Cores (a story item).
pub const MATERIALS: [u32; 69] = [
    510, 515, 516, 517, 518, 520, 521, 525, 526, 530, 531, 535, 536, 540, 545, 546, 550, 551, 555,
    560, 570, 571, 572, 573, 580, 581, 582, 583, 584, 585, 600, 601, 602, 603, 610, 621, 622, 623,
    624, 625, 626, 627, 628, 629, 630, 631, 632, 633, 634, 635, 636, 637, 638, 639, 640, 641, 642,
    643, 644, 645, 646, 680, 681, 683, 684, 685, 686, 687, 688,
];

/// Weapon ids and names, in the order of the save's weapon list.
pub const WEAPON_NAMES: [(u32, &str); 39] = [
    (0x3EB, "Faith"),
    (0x3F5, "Iron Pipe"),
    (0x3FC, "Beastbane"),
    (0x410, "Phoenix Dagger"),
    (0x406, "Ancient Overlord"),
    (0x41A, "Type-40 Sword"),
    (0x424, "Type-3 Sword"),
    (0x42E, "Virtuous Contract"),
    (0x42F, "Cruel Oath"),
    (0x438, "YoRHa-issue Blade"),
    (0x442, "Machine Sword"),
    (0x4B3, "Iron Will"),
    (0x4BD, "Fang of the Twins"),
    (0x4C4, "Beastlord"),
    (0x4CE, "Phoenix Sword"),
    (0x4D8, "Type-40 Blade"),
    (0x4E2, "Type-3 Blade"),
    (0x4EC, "Virtuous Treaty"),
    (0x4ED, "Cruel Blood Oath"),
    (0x4F6, "Machine Axe"),
    (0x578, "Phoenix Lance"),
    (0x58C, "Beastcurse"),
    (0x596, "Dragoon Lance"),
    (0x5A0, "Spear of the Usurper"),
    (0x5AA, "Type-40 Lance"),
    (0x5B4, "Type-3 Lance"),
    (0x5BE, "Virtuous Dignity"),
    (0x5BF, "Cruel Arrogance"),
    (0x5C8, "Machine Spear"),
    (0x668, "Angel's Folly"),
    (0x65E, "Demon's Cry"),
    (0x64A, "Type-40 Fists"),
    (0x640, "Type-3 Fists"),
    (0x654, "Virtuous Grief"),
    (0x655, "Cruel Lament"),
    (0x672, "Machine Heads"),
    (0x753, "Engine Blade"),
    (0x754, "Cypress Stick"),
    (0x755, "Emil Heads"),
];

pub fn item_name(id: u32) -> String {
    ITEM_NAMES
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| format!("Item {id}"))
}

fn u32_at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && d[WORLD..].starts_with(b"version v")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a NieR:Automata slot save".into());
    }
    let mut b = Builder::new(d);
    let name: Vec<u16> = d[0x34..0x34 + 70]
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();
    b.info(&["Save", "Name"], String::from_utf16_lossy(&name));
    let secs = u32_at(d, 0x24);
    b.info(
        &["Save", "Play time"],
        format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60),
    );
    let chapter = u32_at(d, 0x2C) as i32;
    if chapter >= 0 {
        b.info(&["Save", "Chapter"], chapter);
    }
    b.ranged(&["Player", "Money"], MONEY, Kind::U32, 0, 9_999_999);
    b.ranged(&["Player", "Experience"], XP, Kind::U32, 0, 9_999_999);
    b.info(&["Player", "Level"], level_for(u32_at(d, XP) as i64) as i64);
    let mut seen = std::collections::HashSet::new();
    for slot in 0..256 {
        let at = ITEMS + slot * 12;
        let (id, status) = (u32_at(d, at), u32_at(d, at + 4));
        if id == u32::MAX || status != USED {
            continue;
        }
        let name = item_name(id);
        let group = if MATERIALS.contains(&id) {
            "Materials"
        } else {
            "Items"
        };
        // The same item can take two slots; the second gets its slot number.
        let key = if seen.insert((group, name.clone())) {
            name
        } else {
            format!("{name} (slot {})", slot + 1)
        };
        b.ranged(&[group, &key], at + 8, Kind::U32, 1, 99);
    }
    for (i, (id, name)) in WEAPON_NAMES.iter().enumerate() {
        let at = WEAPONS + i * 20;
        if u32_at(d, at) == *id {
            b.ranged(&["Weapons", name], at + 4, Kind::U32, 1, 4);
        }
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        d[0x24..0x28].copy_from_slice(&3725u32.to_le_bytes());
        d[0x2C..0x30].copy_from_slice(&2u32.to_le_bytes());
        for (i, c) in "2B".encode_utf16().enumerate() {
            d[0x34 + 2 * i..0x36 + 2 * i].copy_from_slice(&c.to_le_bytes());
        }
        d[WORLD..WORLD + 14].copy_from_slice(b"version v0.0.2");
        d[MONEY..MONEY + 4].copy_from_slice(&1500u32.to_le_bytes());
        d[XP..XP + 4].copy_from_slice(&600u32.to_le_bytes());
        for slot in 0..512 {
            let at = ITEMS + slot * 12;
            d[at..at + 12].copy_from_slice(&[0xFF; 12]);
        }
        for (slot, (id, n)) in [(0u32, 5u32), (600, 12), (690, 1)].iter().enumerate() {
            let at = ITEMS + slot * 12;
            d[at..at + 4].copy_from_slice(&id.to_le_bytes());
            d[at + 4..at + 8].copy_from_slice(&USED.to_le_bytes());
            d[at + 8..at + 12].copy_from_slice(&n.to_le_bytes());
        }
        for i in 0..39 {
            d[WEAPONS + i * 20..WEAPONS + i * 20 + 4].copy_from_slice(&[0xFF; 4]);
        }
        d[WEAPONS + 9 * 20..WEAPONS + 9 * 20 + 4].copy_from_slice(&0x438u32.to_le_bytes());
        d[WEAPONS + 9 * 20 + 4..WEAPONS + 9 * 20 + 8].copy_from_slice(&2u32.to_le_bytes());
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = synthetic();
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Save/Name"), Some(Value::from("2B")));
        assert_eq!(v(&doc, "/Save/Play time"), Some(Value::from("1:02:05")));
        assert_eq!(v(&doc, "/Player/Level"), Some(Value::from(5)));
        assert_eq!(v(&doc, "/Items/Small Recovery"), Some(Value::from(5)));
        assert_eq!(v(&doc, "/Materials/Copper Ore"), Some(Value::from(12)));
        // Machine Core is a material; Children's Cores (692) would not be.
        assert_eq!(v(&doc, "/Materials/Machine Core"), None);
        assert_eq!(v(&doc, "/Items/Machine Core"), Some(Value::from(1)));
        assert_eq!(v(&doc, "/Weapons/YoRHa-issue Blade"), Some(Value::from(2)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Player/Money", Value::from(9_999_999)).unwrap();
        doc.set("/Materials/Copper Ore", Value::from(99)).unwrap();
        doc.set("/Weapons/YoRHa-issue Blade", Value::from(4))
            .unwrap();
        assert!(doc.set("/Materials/Copper Ore", Value::from(100)).is_err());
        assert!(doc
            .set("/Weapons/YoRHa-issue Blade", Value::from(5))
            .is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), SIZE);
        let again = parse(&out).unwrap();
        assert_eq!(v(&again, "/Player/Money"), Some(Value::from(9_999_999)));
        assert_eq!(v(&again, "/Materials/Copper Ore"), Some(Value::from(99)));
        assert_eq!(
            v(&again, "/Weapons/YoRHa-issue Blade"),
            Some(Value::from(4))
        );
        assert!(parse(&d[..SIZE - 1]).is_err());
        assert_eq!(level_for(0), 1);
        assert_eq!(level_for(1_235_211), 99);
    }

    /// Research aid: SAVE_EDITOR_NIER = `SlotData_N.dat` files separated by `|`.
    #[test]
    #[ignore]
    fn real_files() {
        for path in std::env::var("SAVE_EDITOR_NIER")
            .unwrap()
            .split('|')
            .filter(|s| !s.is_empty())
        {
            let d = std::fs::read(path).unwrap();
            let doc = parse(&d).unwrap();
            println!("{}", serde_json::to_string(&doc.view).unwrap());
            assert_eq!(doc.to_bytes().unwrap(), d, "round trip");
        }
    }
}
