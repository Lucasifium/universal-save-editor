//! Kingdom Hearts (PlayStation 2, the original and Final Mix) saves: the game's own 93,184-byte
//! save file inside a `.psu`, `.psv` or memory card folder. Offsets, ids and names follow the
//! MIT-licensed save editor in RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).
//! The save has no checksum. The play time lives in a separate file of the save and is not edited.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::opts;
use crate::tree::TreeDoc;

pub const KIND: &str = "Kingdom Hearts save";
const SIZE: usize = 93_184;
const CHARACTERS: [&str; 10] = ["Sora", "Donald", "Goofy", "Tarzan", "Pooh", "Aladdin", "Ariel", "Jack", "Peter Pan", "Beast"];

pub fn looks(d: &[u8]) -> bool {
    // No magic: the layout has to read sensibly (version word 4 or 5, Sora's level, HP and AP).
    d.len() == SIZE
        && (4..=5).contains(&d[0])
        && d[1..4] == [0, 0, 0]
        && (1..=100).contains(&d[4])
        && d[5] >= 1
        && d[6] >= d[5].min(1)
        && d[9] >= 1
        && d[0x1641f] == 0
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Kingdom Hearts save".into());
    }
    let mut b = Builder::new(d);
    if d[0] == 5 {
        b.pick(&["General", "Difficulty"], 0x1642c, Kind::U32, &opts(&[("Final Mix: Beginner", 0), ("Final Mix", 1), ("Final Mix: Proud", 2)]));
    } else {
        b.pick(&["General", "Difficulty (not in the Japanese release)"], 0x16418, Kind::U32, &opts(&[("Normal", 0), ("Expert", 1)]));
    }
    b.ranged(&["General", "Munny"], 0x1641c, Kind::U32, 0, 99_999);
    for (i, n) in ["Fire", "Blizzard", "Thunder", "Cure", "Gravity", "Stop", "Aero"].iter().enumerate() {
        b.ranged(&["Magic", n], 0x492 + i, Kind::U8, 0, 3);
    }
    let weapons: Vec<(String, u64)> = std::iter::once(("-".to_string(), 0)).chain(ITEMS.iter().filter(|x| x.1 >> 4 == 5).map(|x| (x.3.to_string(), x.0 as u64))).collect();
    for (i, n) in CHARACTERS.iter().enumerate() {
        let s = 0x74 * i;
        b.ranged(&["Party", n, "Level"], s + 4, Kind::U8, 1, 100);
        b.ranged(&["Party", n, "Experience"], s + 0x40, Kind::U32, 0, 999_999);
        b.ranged(&["Party", n, "HP"], s + 5, Kind::U8, 0, 255);
        b.ranged(&["Party", n, "Max HP"], s + 6, Kind::U8, 1, 255);
        b.ranged(&["Party", n, "MP"], s + 7, Kind::U8, 0, 255);
        b.ranged(&["Party", n, "Max MP"], s + 8, Kind::U8, 0, 255);
        b.ranged(&["Party", n, "Max AP"], s + 9, Kind::U8, 0, 100);
        b.ranged(&["Party", n, "Strength"], s + 10, Kind::U8, 0, 100);
        b.ranged(&["Party", n, "Defense"], s + 11, Kind::U8, 0, 100);
        b.pick(&["Party", n, "Weapon"], s + 0x36, Kind::U8, &weapons);
    }
    let mut seen = std::collections::HashSet::new();
    for (ty, cat) in [(0u8, "Recovery items"), (1, "Boosting items"), (2, "Rare drops"), (3, "Synthesis materials"), (4, "Key items"), (5, "Weapons"), (6, "Accessories")] {
        for (i, t, _, name) in ITEMS {
            if t >> 4 != ty {
                continue;
            }
            let mut key = name.to_string();
            if !seen.insert(format!("{cat}/{key}")) {
                key = format!("{name} (2)");
                seen.insert(format!("{cat}/{key}"));
            }
            b.ranged(&["Stock", cat, &key], 0x499 + *i as usize, Kind::U8, 0, if ty == 4 { 1 } else { 99 });
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. A stock number is how many you own (99 at most; key items 0 or 1). The Final Mix only items (synthesis materials) are listed for every release but only matter in Final Mix.")))
}

const ITEMS: &[(u8, u8, bool, &str)] = &[
    (1, 0x00, false, "Potion"),
    (2, 0x00, false, "Hi-Potion"),
    (3, 0x00, false, "Ether"),
    (4, 0x00, false, "Elixir"),
    (6, 0x00, false, "Mega-Potion"),
    (7, 0x00, false, "Mega-Ether"),
    (8, 0x00, false, "Megalixir"),
    (142, 0x01, false, "Tent"),
    (143, 0x01, false, "Camping Set"),
    (144, 0x01, false, "Cottage"),
    (152, 0x10, false, "Power Up"),
    (153, 0x10, false, "Defense Up"),
    (154, 0x10, false, "AP Up"),
    (158, 0x20, false, "Fire Arts"),
    (159, 0x20, false, "Blizzard Arts"),
    (160, 0x20, false, "Thunder Arts"),
    (161, 0x20, false, "Cure Arts"),
    (162, 0x20, false, "Gravity Arts"),
    (163, 0x20, false, "Stop Arts"),
    (164, 0x20, false, "Aero Arts"),
    (165, 0x20, false, "Shiitake Rank"),
    (166, 0x20, false, "Matsutake Rank"),
    (167, 0x20, false, "Mystery Mold"),
    (233, 0x30, false, "Lucid Shard"),
    (234, 0x30, false, "Lucid Gem"),
    (235, 0x30, false, "Lucid Crystal"),
    (236, 0x30, false, "Spirit Shard"),
    (237, 0x30, false, "Spirit Gem"),
    (9, 0x30, true, "Fury Stone"),
    (238, 0x30, false, "Power Shard"),
    (239, 0x30, false, "Power Gem"),
    (10, 0x30, true, "Power Stone"),
    (240, 0x30, false, "Power Crystal"),
    (11, 0x30, true, "Energy Stone"),
    (241, 0x30, false, "Blaze Shard"),
    (242, 0x30, false, "Blaze Gem"),
    (12, 0x30, true, "Blazing Stone"),
    (243, 0x30, false, "Frost Shard"),
    (244, 0x30, false, "Frost Gem"),
    (13, 0x30, true, "Frost Stone"),
    (245, 0x30, false, "Thunder Shard"),
    (246, 0x30, false, "Thunder Gem"),
    (14, 0x30, true, "Lightning Stone"),
    (247, 0x30, false, "Shiny Crystal"),
    (15, 0x30, true, "Dazzling Stone"),
    (248, 0x30, false, "Bright Shard"),
    (249, 0x30, false, "Bright Gem"),
    (250, 0x30, false, "Bright Crystal"),
    (251, 0x30, false, "Mystery Goo"),
    (155, 0x30, true, "Serenity Power"),
    (252, 0x30, false, "Gale"),
    (16, 0x30, true, "Stormy Stone"),
    (253, 0x30, false, "Mythril Shard"),
    (254, 0x30, false, "Mythril"),
    (157, 0x30, true, "Mythril Stone"),
    (255, 0x30, false, "Orichalcum"),
    (156, 0x30, true, "Dark Matter"),
    (168, 0x40, false, "Ansem Report 1"),
    (169, 0x40, false, "Ansem Report 2"),
    (170, 0x40, false, "Ansem Report 3"),
    (171, 0x40, false, "Ansem Report 4"),
    (172, 0x40, false, "Ansem Report 5"),
    (173, 0x40, false, "Ansem Report 6"),
    (174, 0x40, false, "Ansem Report 7"),
    (175, 0x40, false, "Ansem Report 8"),
    (176, 0x40, false, "Ansem Report 9"),
    (177, 0x40, false, "Ansem Report 10"),
    (149, 0x40, true, "Ansem Report 11"),
    (150, 0x40, true, "Ansem Report 12"),
    (151, 0x40, true, "Ansem Report 13"),
    (206, 0x41, false, "Watergleam"),
    (207, 0x41, false, "Naturespark"),
    (208, 0x41, false, "Fireglow"),
    (209, 0x41, false, "Earthshine"),
    (200, 0x41, false, "Navi-G Piece"),
    (201, 0x41, false, "Navi-G Piece"),
    (202, 0x41, false, "Navi-Gummi"),
    (203, 0x41, false, "Navi-G Piece"),
    (204, 0x41, false, "Navi-G Piece"),
    (205, 0x41, false, "Navi-G Gummi"),
    (187, 0x41, false, "Old Book"),
    (186, 0x41, false, "Empty Bottle"),
    (192, 0x41, false, "Log"),
    (193, 0x41, false, "Cloth"),
    (194, 0x41, false, "Rope"),
    (195, 0x41, false, "Seagull Egg"),
    (196, 0x41, false, "Fish"),
    (197, 0x41, false, "Mushroom"),
    (198, 0x41, false, "Coconut"),
    (199, 0x41, false, "Drinking Water"),
    (178, 0x41, false, "Khama vol. 8"),
    (179, 0x41, false, "Salegg vol. 6"),
    (180, 0x41, false, "Azal vol. 3"),
    (181, 0x41, false, "Mava vol. 3"),
    (182, 0x41, false, "Mava vol. 6"),
    (183, 0x41, false, "Theon vol. 6"),
    (184, 0x41, false, "Nahara vol. 5"),
    (185, 0x41, false, "Hafet vol. 4"),
    (188, 0x41, false, "Emblem Piece"),
    (189, 0x41, false, "Emblem Piece"),
    (190, 0x41, false, "Emblem Piece"),
    (191, 0x41, false, "Emblem Piece"),
    (210, 0x41, false, "Crystal Trident"),
    (211, 0x41, false, "Postcard"),
    (212, 0x41, false, "Torn Page"),
    (213, 0x41, false, "Torn Page"),
    (214, 0x41, false, "Torn Page"),
    (215, 0x41, false, "Torn Page"),
    (216, 0x41, false, "Torn Page"),
    (217, 0x41, false, "Slide 1"),
    (218, 0x41, false, "Slide 2"),
    (219, 0x41, false, "Slide 3"),
    (220, 0x41, false, "Slide 4"),
    (221, 0x41, false, "Slide 5"),
    (222, 0x41, false, "Slide 6"),
    (223, 0x41, false, "Footprints"),
    (224, 0x41, false, "Claw Marks"),
    (225, 0x41, false, "Stench"),
    (226, 0x41, false, "Antenna"),
    (227, 0x41, false, "Forget-Me-Not"),
    (228, 0x41, false, "Jack-in-the-Box"),
    (229, 0x41, false, "Entry Pass"),
    (230, 0x41, false, "Hero License"),
    (231, 0x41, false, "Pretty Stone"),
    (81, 0x50, false, "Kingdom Key"),
    (82, 0x50, false, "Dream Sword"),
    (83, 0x50, false, "Dream Shield"),
    (84, 0x50, false, "Dream Rod"),
    (85, 0x50, false, "Wooden Sword"),
    (86, 0x50, false, "Jungle King"),
    (87, 0x50, false, "Three Wishes"),
    (90, 0x50, false, "Crabclaw"),
    (89, 0x50, false, "Pumpkinhead"),
    (88, 0x50, false, "Fairy Harp"),
    (99, 0x50, false, "Wishing Star"),
    (92, 0x50, false, "Spellbinder"),
    (95, 0x50, false, "Metal Chocobo"),
    (93, 0x50, false, "Olympia"),
    (94, 0x50, false, "Lionheart"),
    (98, 0x50, false, "Lady Luck"),
    (91, 0x50, false, "Divine Rose"),
    (96, 0x50, false, "Oathkeeper"),
    (97, 0x50, false, "Oblivion"),
    (101, 0x50, true, "Diamond Dust"),
    (102, 0x50, true, "One-Winged Angel"),
    (100, 0x50, false, "Ultima Weapon"),
    (103, 0x51, false, "Mage's Staff"),
    (104, 0x51, false, "Morning Star"),
    (105, 0x51, false, "Shooting Star"),
    (106, 0x51, false, "Magus Staff"),
    (107, 0x51, false, "Wisdom Staff"),
    (108, 0x51, false, "Warhammer"),
    (109, 0x51, false, "Silver Mallet"),
    (110, 0x51, false, "Grand Mallet"),
    (111, 0x51, false, "Lord Fortune"),
    (112, 0x51, false, "Violetta"),
    (115, 0x51, false, "Wizard’s Relic"),
    (113, 0x51, false, "Dream Rod"),
    (116, 0x51, true, "Meteor Strike"),
    (117, 0x51, true, "Fantasista"),
    (114, 0x51, false, "Save the Queen"),
    (119, 0x52, false, "Knight’s Shield"),
    (120, 0x52, false, "Mythril Shield"),
    (121, 0x52, false, "Onyx Shield"),
    (125, 0x52, false, "Smasher"),
    (126, 0x52, false, "Gigas Fist"),
    (122, 0x52, false, "Stout Shield"),
    (123, 0x52, false, "Golem Shield"),
    (124, 0x52, false, "Adamant Shield"),
    (128, 0x52, false, "Herc’s Shield"),
    (127, 0x52, false, "Genji Shield"),
    (131, 0x52, false, "Defender"),
    (129, 0x52, false, "Dream Shield"),
    (132, 0x52, true, "Mighty Shield"),
    (133, 0x52, true, "Seven Elements"),
    (130, 0x52, false, "Save the King"),
    (135, 0x53, false, "Spear (Tarzan)"),
    (136, 0x53, false, "No Weapon (Pooh)"),
    (137, 0x53, false, "Sword (Aladdin)"),
    (138, 0x53, false, "No Weapon (Ariel)"),
    (139, 0x53, false, "No Weapon (Jack)"),
    (140, 0x53, false, "Dagger (Peter Pan)"),
    (141, 0x53, false, "Claws (Beast)"),
    (17, 0x60, false, "Protect Chain"),
    (18, 0x60, false, "Protera Chain"),
    (19, 0x60, false, "Protega Chain"),
    (20, 0x60, false, "Fire Ring"),
    (21, 0x60, false, "Fira Ring"),
    (22, 0x60, false, "Firaga Ring"),
    (61, 0x60, true, "Firagun Band"),
    (23, 0x60, false, "Blizzard Ring"),
    (24, 0x60, false, "Blizzara Ring"),
    (25, 0x60, false, "Blizzaga Ring"),
    (62, 0x60, true, "Blizzagun Band"),
    (26, 0x60, false, "Thunder Ring"),
    (27, 0x60, false, "Thundara Ring"),
    (28, 0x60, false, "Thundaga Ring"),
    (63, 0x60, true, "Thundagun Band"),
    (32, 0x60, false, "Chaos Ring"),
    (33, 0x60, false, "Dark Ring"),
    (34, 0x60, false, "Element Ring"),
    (35, 0x60, false, "Three Stars"),
    (71, 0x60, true, "Obsidian Ring"),
    (29, 0x60, false, "Ability Stud"),
    (30, 0x60, false, "Guard Earring"),
    (31, 0x60, false, "Master Earring"),
    (36, 0x60, false, "Power Chain"),
    (37, 0x60, false, "Golem Chain"),
    (38, 0x60, false, "Titan Chain"),
    (39, 0x60, false, "Energy Bangle"),
    (40, 0x60, false, "Angel Bangle"),
    (41, 0x60, false, "Gaia Bangle"),
    (42, 0x60, false, "Magic Armlet"),
    (43, 0x60, false, "Rune Armlet"),
    (44, 0x60, false, "Atlas Armlet"),
    (67, 0x60, true, "Moogle Badge"),
    (68, 0x60, true, "Cosmic Arts"),
    (48, 0x60, false, "Brave Warrior"),
    (49, 0x60, false, "Ifrit's Horn"),
    (50, 0x60, false, "Inferno Band"),
    (51, 0x60, false, "White Fang"),
    (52, 0x60, false, "Ray of Light"),
    (53, 0x60, false, "Holy Circlet"),
    (54, 0x60, false, "Raven's Claw"),
    (55, 0x60, false, "Omega Arts"),
    (45, 0x60, false, "Heartguard"),
    (46, 0x60, false, "Ribbon"),
    (47, 0x60, false, "Crystal Crown"),
    (69, 0x60, true, "Royal Crown"),
    (70, 0x60, true, "Prime Cap"),
    (64, 0x60, true, "Ifrit Belt"),
    (65, 0x60, true, "Shiva Belt"),
    (66, 0x60, true, "Ramuh Belt"),
    (56, 0x60, true, "EXP Earring"),
    (58, 0x60, true, "EXP Ring"),
    (59, 0x60, true, "EXP Bracelet"),
    (60, 0x60, true, "EXP Necklace"),
];

#[allow(dead_code)]
const ABILITIES: &[(u8, &str)] = &[
    (1, "High Jump"),
    (2, "Mermaid Kick"),
    (3, "Glide"),
    (4, "Superglide"),
    (5, "Treasure Magnet"),
    (6, "Combo Plus"),
    (7, "Air Combo Plus"),
    (8, "Critical Plus"),
    (9, "Second Wind"),
    (10, "Scan"),
    (11, "Sonic Blade"),
    (12, "Ars Arcanum"),
    (13, "Strike Raid"),
    (14, "Ragnarok"),
    (15, "Trinity Limit"),
    (16, "Cheer"),
    (17, "Vortex"),
    (18, "Aerial Sweep"),
    (19, "Counterattack"),
    (20, "Blitz"),
    (21, "Guard"),
    (22, "Dodge Roll"),
    (23, "MP Haste"),
    (24, "MP Rage"),
    (25, "Second Chance"),
    (26, "Berserk"),
    (27, "Jackpot"),
    (28, "Lucky Strike"),
    (29, "Charge"),
    (30, "Rocket"),
    (31, "Tornado"),
    (32, "MP Gift"),
    (33, "Raging Boar"),
    (34, "Asp’s Bite"),
    (35, "Healing Herb"),
    (36, "Wind Armor"),
    (37, "Crescent"),
    (38, "Sandstorm"),
    (39, "Applause!"),
    (40, "Blazing Fury"),
    (41, "Icy Terror"),
    (42, "Bolts of Sorrow"),
    (43, "Ghostly Scream"),
    (44, "Hummingbird"),
    (45, "Time-Out"),
    (46, "Storm’s Eye"),
    (47, "Ferocious Lunge"),
    (48, "Furious Bellow"),
    (49, "Spiral Wave"),
    (50, "Thunder Potion"),
    (51, "Cure Potion"),
    (52, "Aero Potion"),
    (53, "Slapshot"),
    (54, "Sliding Dash"),
    (55, "Hurricane Blast"),
    (56, "Ripple Drive"),
    (57, "Stun Impact"),
    (58, "Gravity Break"),
    (59, "Zantetsuken"),
    (60, "Tech Boost"),
    (61, "Encounter Plus"),
    (62, "Leaf Bracer"),
    (63, "Evolution"),
    (64, "EXP Zero"),
    (65, "Combo Master"),
];
