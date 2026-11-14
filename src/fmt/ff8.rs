//! Final Fantasy VIII (PlayStation 1) saves: one 8 KiB block per save. Offsets, ids and names
//! follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md). The save keeps a CRC-16 of 0x1d0..0x1520 at 0x180 and 0x1520.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, opts, title};
use crate::tree::TreeDoc;

pub const KIND: &str = "Final Fantasy VIII save";

pub fn looks(d: &[u8]) -> bool {
    is_save(d, 1) && title(d).starts_with("FF8")
}

const CHARACTERS: &[(&str, u64)] = &[
    ("Squall", 0), ("Zell", 1), ("Irvine", 2), ("Quistis", 3), ("Rinoa", 4), ("Selphie", 5), ("Seifer", 6), ("Edea", 7),
    ("Laguna", 8), ("Kiros", 9), ("Ward", 10), ("-", 255),
];
const GFS: [&str; 16] = [
    "Quezacotl", "Shiva", "Ifrit", "Siren", "Brothers", "Diablos", "Carbuncle", "Leviathan", "Pandemona", "Cerberus",
    "Alexander", "Doomtrain", "Bahamut", "Cactuar", "Tonberry", "Eden",
];

fn ids(list: &[(u8, &str)]) -> Vec<(String, u64)> {
    list.iter().map(|(i, n)| (n.to_string(), *i as u64)).collect()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Final Fantasy VIII save".into());
    }
    let mut b = Builder::new(d);
    let items = ids(ITEMS);
    let magics = ids(MAGICS);
    let chars = opts(CHARACTERS);
    b.info(&["Info", "Save title"], title(d));
    b.ranged(&["General", "Gil (Squall)"], 0xc8c, Kind::U32, 0, 99_999_999);
    b.mirror(&["General", "Gil (Squall)"], 0x18c);
    b.ranged(&["General", "Gil (Laguna)"], 0xc90, Kind::U32, 0, 99_999_999);
    b.ranged(&["General", "Story progression"], 0xfe0, Kind::U16, 0, 4050);
    b.ranged(&["General", "Triple Triad wins"], 0x14d4, Kind::U16, 0, 65535);
    b.ranged(&["General", "Triple Triad losses"], 0x14d6, Kind::U16, 0, 65535);
    b.ranged(&["General", "Triple Triad draws"], 0x14d8, Kind::U16, 0, 65535);
    for i in 0..3 {
        let n = format!("Character {}", i + 1);
        b.pick(&["Formation", &n], 0xc74 + i, Kind::U8, &chars);
    }
    for p in 0..8 {
        let s = 0x98 * p;
        let slot = format!("Slot {}", p + 1);
        b.pick(&["Party", &slot, "Character"], s + 0x618, Kind::U8, &chars);
        b.ranged(&["Party", &slot, "Experience"], s + 0x614, Kind::U32, 0, 99_000);
        b.ranged(&["Party", &slot, "HP"], s + 0x610, Kind::U16, 0, 9999);
        b.ranged(&["Party", &slot, "Bonus HP"], s + 0x612, Kind::U16, 0, 9999);
        for (n, at) in [("Bonus Strength", 0x61a), ("Bonus Vitality", 0x61b), ("Bonus Magic", 0x61c), ("Bonus Spirit", 0x61d), ("Bonus Speed", 0x61e), ("Bonus Luck", 0x61f)] {
            b.ranged(&["Party", &slot, n], s + at, Kind::U8, 0, 255);
        }
        for j in 0..32 {
            let m = format!("Magic {}", j + 1);
            b.pick(&["Magic", &slot, &m, "Magic"], s + 0x620 + j * 2, Kind::U8, &magics);
            b.ranged(&["Magic", &slot, &m, "Quantity"], s + 0x621 + j * 2, Kind::U8, 0, 100);
        }
    }
    for (g, name) in GFS.iter().enumerate() {
        let s = 0x44 * g;
        b.pick(&["Guardian Forces", name, "Obtained"], s + 0x1e1, Kind::U8, &opts(&[("No", 0), ("Yes", 1)]));
        b.ranged(&["Guardian Forces", name, "Experience"], s + 0x1dc, Kind::U32, 0, 49_500);
        b.ranged(&["Guardian Forces", name, "HP"], s + 0x1e2, Kind::U16, 0, 9999);
    }
    for i in 0..198 {
        let n = format!("Slot {}", i + 1);
        b.pick(&["Inventory", &n, "Item"], 0xcc4 + i * 2, Kind::U8, &items);
        b.ranged(&["Inventory", &n, "Quantity"], 0xcc5 + i * 2, Kind::U8, 0, 100);
    }
    for (i, n) in COMMON_CARDS.iter().enumerate() {
        b.bits(&["Cards", n], 0x1460 + i, Kind::U8, 0, 7, 0, 0, 100);
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Levels follow experience (1000 per level for the party, 500 for guardian forces). The save's CRC-16 is written again; the story progression is left to you, since a wrong value can break the story."))
}

/// CRC-16 (polynomial 0x1021, start 0xffff, complemented) with the game's own table, whose last
/// entry is 0.
fn crc(d: &[u8]) -> u16 {
    let mut t = [0u16; 256];
    for (i, e) in t.iter_mut().enumerate().take(255) {
        let mut c = (i as u16) << 8;
        for _ in 0..8 {
            c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
        }
        *e = c;
    }
    let mut c = 0xffffu16;
    for &x in d {
        c = t[(x ^ (c >> 8) as u8) as usize] ^ (c << 8);
    }
    !c
}

fn fix(d: &mut Vec<u8>) {
    let c = crc(&d[0x1d0..0x1520]).to_le_bytes();
    d[0x180..0x182].copy_from_slice(&c);
    d[0x1520..0x1522].copy_from_slice(&c);
}

const ITEMS: &[(u8, &str)] = &[
    (0, "-"),
    (1, "Potion"),
    (2, "Potion+"),
    (3, "Hi-Potion"),
    (4, "Hi-Potion+"),
    (5, "X-Potion"),
    (6, "Mega-Potion"),
    (7, "Phoenix Down"),
    (8, "Mega Phoenix"),
    (9, "Elixir"),
    (10, "Megalixir"),
    (11, "Antidote"),
    (12, "Soft"),
    (13, "Eye Drops"),
    (14, "Echo Screen"),
    (15, "Holy Water"),
    (16, "Remedy"),
    (17, "Remedy+"),
    (18, "Hero-trial"),
    (19, "Hero"),
    (20, "Holy War-trial"),
    (21, "Holy War"),
    (22, "Shell Stone"),
    (23, "Protect Stone"),
    (24, "Aura Stone"),
    (25, "Death Stone"),
    (26, "Holy Stone"),
    (27, "Flare Stone"),
    (28, "Meteor Stone"),
    (29, "Ultima Stone"),
    (30, "Gysahl Greens"),
    (31, "Phoenix Pinion"),
    (32, "Friendship"),
    (33, "Tent"),
    (34, "Pet House"),
    (35, "Cottage"),
    (36, "G-Potion"),
    (37, "G-Hi-Potion"),
    (38, "G-Mega-Potion"),
    (39, "G-Returner"),
    (40, "Rename Card"),
    (41, "Amnesia Greens"),
    (42, "HP-J Scroll"),
    (43, "Str-J Scroll"),
    (44, "Vit-J Scroll"),
    (45, "Mag-J Scroll"),
    (46, "Spr-J Scroll"),
    (47, "Spd-J Scroll"),
    (48, "Luck-J Scroll"),
    (49, "Aegis Amulet"),
    (50, "Elem Atk"),
    (51, "Elem Guard"),
    (52, "Status Atk"),
    (53, "Status Guard"),
    (54, "Rosetta Stone"),
    (55, "Magic Scroll"),
    (56, "GF Scroll"),
    (57, "Draw Scroll"),
    (58, "Item Scroll"),
    (59, "Gambler Spirit"),
    (60, "Healing Ring"),
    (61, "Phoenix Spirit"),
    (62, "Med Kit"),
    (63, "Bomb Spirit"),
    (64, "Hungry Cookpot"),
    (65, "Mog's Amulet"),
    (66, "Steel Pipe"),
    (67, "Star Fragment"),
    (68, "Energy Crystal"),
    (69, "Samantha Soul"),
    (70, "Healing Mail"),
    (71, "Silver Mail"),
    (72, "Gold Armor"),
    (73, "Diamond Armor"),
    (74, "Regen Ring"),
    (75, "Giant's Ring"),
    (76, "Gaea's Ring"),
    (77, "Strength Love"),
    (78, "Power Wrist"),
    (79, "Hyper Wrist"),
    (80, "Turtle Shell"),
    (81, "Orihalcon"),
    (82, "Adamantine"),
    (83, "Rune Armlet"),
    (84, "Force Armlet"),
    (85, "Magic Armlet"),
    (86, "Circlet"),
    (87, "Hypno Crown"),
    (88, "Royal Crown"),
    (89, "Jet Engine"),
    (90, "Rocket Engine"),
    (91, "Moon Curtain"),
    (92, "Steel Curtain"),
    (93, "Glow Curtain"),
    (94, "Accelerator"),
    (95, "Monk's Code"),
    (96, "Knight's Code"),
    (97, "Doc's Code"),
    (98, "Hundred Needles"),
    (99, "Three Stars"),
    (100, "Ribbon"),
    (101, "Normal Ammo"),
    (102, "Shotgun Ammo"),
    (103, "Dark Ammo"),
    (104, "Fire Ammo"),
    (105, "Demolition Ammo"),
    (106, "Fast Ammo"),
    (107, "AP Ammo"),
    (108, "Pulse Ammo"),
    (109, "M-Stone Piece"),
    (110, "Magic Stone"),
    (111, "Wizard Stone"),
    (112, "Ochu Tentacle"),
    (113, "Healing Water"),
    (114, "Cockatrice Pinion"),
    (115, "Zombie Powder"),
    (116, "Lightweight"),
    (117, "Sharp Spike"),
    (118, "Screw"),
    (119, "Saw Blade"),
    (120, "Mesmerize Blade"),
    (121, "Vampire Fang"),
    (122, "Fury Fragment"),
    (123, "Betrayal Sword"),
    (124, "Sleep Powder"),
    (125, "Life Ring"),
    (126, "Dragon Fang"),
    (127, "Spider Web"),
    (128, "Coral Fragment"),
    (129, "Curse Spike"),
    (130, "Black Hole"),
    (131, "Water Crystal"),
    (132, "Missile"),
    (133, "Mystery Fluid"),
    (134, "Running Fire"),
    (135, "Inferno Fang"),
    (136, "Malboro Tentacle"),
    (137, "Whisper"),
    (138, "Laser Cannon"),
    (139, "Barrier"),
    (140, "Power Generator"),
    (141, "Dark Matter"),
    (142, "Bomb Fragment"),
    (143, "Red Fang"),
    (144, "Arctic Wind"),
    (145, "North Wind"),
    (146, "Dynamo Stone"),
    (147, "Shear Feather"),
    (148, "Venom Fang"),
    (149, "Steel Orb"),
    (150, "Moon Stone"),
    (151, "Dino Bone"),
    (152, "Windmill"),
    (153, "Dragon Skin"),
    (154, "Fish Fin"),
    (155, "Dragon Fin"),
    (156, "Silence Powder"),
    (157, "Poison Powder"),
    (158, "Dead Spirit"),
    (159, "Chef's Knife"),
    (160, "Cactus Thorn"),
    (161, "Shaman Stone"),
    (162, "Fuel"),
    (163, "Girl Next Door"),
    (164, "Sorceress' Letter"),
    (165, "Chocobo's Tag"),
    (166, "Pet Nametag"),
    (167, "Solomon Ring"),
    (168, "Magical Lamp"),
    (169, "HP Up"),
    (170, "Str Up"),
    (171, "Vit Up"),
    (172, "Mag Up"),
    (173, "Spr Up"),
    (174, "Spd Up"),
    (175, "Luck Up"),
    (176, "LuvLuv G"),
    (177, "Weapons Mon 1st"),
    (178, "Weapons Mon Mar"),
    (179, "Weapons Mon Apr"),
    (180, "Weapons Mon May"),
    (181, "Weapons Mon Jun"),
    (182, "Weapons Mon Jul"),
    (183, "Weapons Mon Aug"),
    (184, "Combat King 001"),
    (185, "Combat King 002"),
    (186, "Combat King 003"),
    (187, "Combat King 004"),
    (188, "Combat King 005"),
    (189, "Pet Pals Vol.1"),
    (190, "Pet Pals Vol.2"),
    (191, "Pet Pals Vol.3"),
    (192, "Pet Pals Vol.4"),
    (193, "Pet Pals Vol.5"),
    (194, "Pet Pals Vol.6"),
    (195, "Occult Fan I"),
    (196, "Occult Fan II"),
    (197, "Occult Fan III"),
    (198, "Occult Fan IV"),
];
const MAGICS: &[(u8, &str)] = &[
    (0, "-"),
    (1, "Fire"),
    (2, "Fira"),
    (3, "Firaga"),
    (4, "Blizzard"),
    (5, "Blizzara"),
    (6, "Blizzaga"),
    (7, "Thunder"),
    (8, "Thundara"),
    (9, "Thundaga"),
    (10, "Water"),
    (11, "Aero"),
    (12, "Bio"),
    (13, "Demi"),
    (14, "Holy"),
    (15, "Flare"),
    (16, "Meteor"),
    (17, "Quake"),
    (18, "Tornado"),
    (19, "Ultima"),
    (20, "Apocalypse"),
    (21, "Cure"),
    (22, "Cura"),
    (23, "Curaga"),
    (24, "Life"),
    (25, "Full-life"),
    (26, "Regen"),
    (27, "Esuna"),
    (28, "Dispel"),
    (29, "Protect"),
    (30, "Shell"),
    (31, "Reflect"),
    (32, "Aura"),
    (33, "Double"),
    (34, "Triple"),
    (35, "Haste"),
    (36, "Slow"),
    (37, "Stop"),
    (38, "Blind"),
    (39, "Confuse"),
    (40, "Sleep"),
    (41, "Silence"),
    (42, "Break"),
    (43, "Death"),
    (44, "Drain"),
    (45, "Pain"),
    (46, "Berserk"),
    (47, "Float"),
    (48, "Zombie"),
    (49, "Meltdown"),
    (50, "Scan"),
];
const COMMON_CARDS: &[&str] = &["Geezard", "Funguar", "Bite Bug", "Red Bat", "Blobra", "Gayla", "Gesper", "Fastitocalon-F", "Blood Soul", "Caterchipillar", "Cockatrice", "Grat", "Buel", "Mesmerize", "Glacial Eye", "Belhelmel", "Thrustaevis", "Anacondaur", "Creeps", "Grendel", "Jelleye", "Grand Mantis", "Forbidden", "Armadodo", "Tri-Face", "Fastitocalon", "Snow Lion", "Ochu", "SAM08G", "Death Claw", "Cactuar", "Tonberry", "Abyss Worm", "Turtapod", "Vysage", "T-Rexaur", "Bomb", "Blitz", "Wendigo", "Torama", "Imp", "Blue Dragon", "Adamantoise", "Hexadragon", "Iron Giant", "Behemoth", "Chimera", "PuPu", "Elastoid", "GIM47N", "Malboro", "Ruby Dragon", "Elnoyle", "Tonberry King", "Wedge, Biggs", "Fujin, Raijin", "Elvoret", "X-ATM092", "Granaldo", "Gerogero", "Iguion", "Abadon", "Trauma", "Oilboyle", "Shumi Tribe", "Krysta", "Propagator", "Jumbo Cactuar", "Tri-Point", "Gargantua", "Mobile Type 8", "Sphinxara", "Tiamat", "BGH251F2", "Red Giant", "Catoblepas", "Ultima Weapon"];
