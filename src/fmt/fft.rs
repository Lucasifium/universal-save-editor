//! Final Fantasy Tactics (PlayStation 1) saves: one 8 KiB block per save. Offsets, ids and names
//! follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md). The save keeps a 64-bit parity checksum at 0x118.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, title};
use crate::tree::TreeDoc;

pub const KIND: &str = "Final Fantasy Tactics save";

pub fn looks(d: &[u8]) -> bool {
    is_save(d, 1) && title(d).starts_with("FFT")
}

fn list(l: &[(u8, &str)], extra: &[(&str, u64)], plus: u64) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = extra.iter().map(|(n, x)| (n.to_string(), *x)).collect();
    v.extend(l.iter().map(|(i, n)| (n.to_string(), *i as u64 + plus)));
    v
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Final Fantasy Tactics save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Info", "Save title"], title(d));
    b.ranged(&["General", "War funds"], 0x1934, Kind::U32, 0, 99_999_999);
    b.ranged(&["General", "Deaths of units (Brave Story)"], 0x1a0c, Kind::U32, 0, 9999);
    b.ranged(&["General", "Injured (Brave Story)"], 0x1a08, Kind::U32, 0, 9999);
    let mut jobs: Vec<(String, u64)> = JOBS.iter().map(|(i, n)| (n.to_string(), *i as u64)).collect();
    jobs.extend(MONSTERS.iter().map(|(i, n)| (n.to_string(), *i as u64)));
    let hands = list(&WEAPONS.iter().filter(|(_, n)| !n.is_empty()).copied().collect::<Vec<_>>(), &[("Empty", 0), ("-", 255)], 1);
    let heads = list(HEADS, &[("Empty", 0), ("-", 255)], 1);
    let bodies = list(ARMOR, &[("Empty", 0), ("-", 255)], 1);
    let accs = list(ACCESSORIES, &[("Empty", 0), ("-", 255)], 1);
    for u in 0..20 {
        let s = 0xe0 * u;
        let n = format!("Unit {}", u + 1);
        b.pick(&["Party", &n, "Job"], s + 0x486, Kind::U8, &jobs);
        b.ranged(&["Party", &n, "Level"], s + 0x49a, Kind::U8, 1, 99);
        b.ranged(&["Party", &n, "Experience"], s + 0x499, Kind::U8, 0, 99);
        b.ranged(&["Party", &n, "Brave"], s + 0x49b, Kind::U8, 0, 100);
        b.ranged(&["Party", &n, "Faith"], s + 0x49c, Kind::U8, 0, 100);
        b.pick(&["Party", &n, "Right hand"], s + 0x495, Kind::U8, &hands);
        b.pick(&["Party", &n, "Left hand"], s + 0x497, Kind::U8, &hands);
        b.pick(&["Party", &n, "Head"], s + 0x492, Kind::U8, &heads);
        b.pick(&["Party", &n, "Body"], s + 0x493, Kind::U8, &bodies);
        b.pick(&["Party", &n, "Accessory"], s + 0x494, Kind::U8, &accs);
    }
    let mut seen = std::collections::HashSet::new();
    for (cat, items) in [("Weapons", WEAPONS), ("Heads", HEADS), ("Armor", ARMOR), ("Accessories", ACCESSORIES), ("Items", ITEMS)] {
        for (i, name) in items {
            let mut key = name.to_string();
            if !seen.insert(key.clone()) {
                key = format!("{name} (2)");
                seen.insert(key.clone());
            }
            b.ranged(&["Inventory", cat, &key], 0x1605 + *i as usize, Kind::U8, 0, 99);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. Level, experience, Brave, Faith and equipment are edited as the game stores them (experience and level are separate bytes: set both). The save's checksum is written again.")))
}

/// 60 parity bits, one per 1024 bits of the first 0x1e00 bytes (the checksum's own bytes counted as zero).
fn checksum(d: &[u8]) -> [u8; 8] {
    let mut buf = [0xffu8; 8];
    for i in 0..60 {
        let ones: u32 = d[i * 128..(i + 1) * 128].iter().map(|b| b.count_ones()).sum();
        let m = 1u8 << (7 - (i % 8));
        buf[i / 8] &= !m;
        if ones & 1 != 0 {
            buf[i / 8] |= m;
        }
    }
    buf
}

fn fix(d: &mut Vec<u8>) {
    d[0x118..0x120].fill(0);
    let c = checksum(d);
    d[0x118..0x120].copy_from_slice(&c);
}

const JOBS: &[(u8, &str)] = &[(1, "Squire (Ramza)"), (2, "Squire (Ramza)"), (3, "Squire (Ramza)"), (4, "Squire (Delita)"), (5, "Holy Knight (Delita)"), (6, "Arc Knight (Delita)"), (7, "Squire (Algus)"), (8, "Arc Knight (Zalbag)"), (9, "Lune Knight (Dycedarg)"), (10, "Duke (Larg)"), (11, "Duke (Goltana)"), (12, "Princess (Ovelia)"), (13, "Holy Swordsman (Orlandu)"), (14, "High Priest (Funeral)"), (15, "Dragoner (Reis)"), (16, "Holy Priest (Zalmo)"), (17, "Dark Knight (Gafgarion)"), (18, "Hell Knight (Malak)"), (19, "Bishop (Simon)"), (20, "Cleric (Alma)"), (21, "Astrologist (Olan)"), (22, "Engineer (Mustadio)"), (23, "Dark Knight (Gafgarion)"), (24, "Cardinal (Draclau)"), (25, "Heaven Knight (Rafa)"), (26, "Hell Knight (Malak)"), (27, "Arc Knight (Elmdor)"), (28, "Delita's Sis (Teta)"), (29, "Arc Duke (Barinten)"), (30, "Holy Knight (Agrias)"), (31, "Temple Knight (Beowulf)"), (32, "White Knight (Wiegraf)"), (33, "Arc Witch (Balmafula)"), (34, "Engineer (Mustadio)"), (35, "Bi-Count (Rudvich)"), (36, "Divine Knight (Vormav)"), (37, "Divine Knight (Rofel)"), (38, "Knight Blade (Izlude)"), (39, "Sorceror (Kletian)"), (40, "White Knight (Wiegraf)"), (41, "Heaven Knight (Rafa)"), (42, "Divine Knight (Meliadoul)"), (43, "Engineer (Balk)"), (44, "Cleric (Alma)"), (45, "Assassin (Celia)"), (46, "Assassin (Lede)"), (47, "Divine Knight (Meliadoul)"), (48, "Cleric (Alma)"), (49, "Phony Saint (Ajora)"), (50, "Soldier (Cloud)"), (51, "Arc Knight (Zalbag)"), (52, "Holy Knight (Agrias)"), (60, "Warlock (Velius)"), (62, "Angel of Death (Zalera)"), (64, "Regulator (Hashmalum)"), (65, "Holy Angel (Altima)"), (67, "Impure King (Queklain)"), (69, "Ghost of Fury (Adramelk)"), (72, "Holy Dragon (Holy Dragon)"), (73, "Arch Angel (Arch Angel)"), (74, "Squire"), (75, "Chemist"), (76, "Knight"), (77, "Archer"), (78, "Monk"), (79, "Priest"), (80, "Wizard"), (81, "Time Mage"), (82, "Summoner"), (83, "Thief"), (84, "Mediator"), (85, "Oracle"), (86, "Geomancer"), (87, "Lancer"), (88, "Samurai"), (89, "Ninja"), (90, "Calculator"), (91, "Bard"), (92, "Dancer"), (93, "Mime"), ];
const MONSTERS: &[(u8, &str)] = &[(94, "Chocobo"), (95, "Black Chocobo"), (96, "Red Chocobo"), (97, "Goblin"), (98, "Black Goblin"), (99, "Gobbledeguck"), (100, "Bomb"), (101, "Grenade"), (102, "Explosive"), (103, "Red Panther"), (104, "Cuar"), (105, "Vampire"), (106, "Pisco Demon"), (107, "Squidlarkin"), (108, "Mindflare"), (109, "Skeleton"), (110, "Bone Snatch"), (111, "Living Bone"), (112, "Ghoul"), (113, "Gust"), (114, "Revnant"), (115, "Flotiball"), (116, "Ahriman"), (117, "Plague"), (118, "Juravis"), (119, "Steel Hawk"), (120, "Cocatoris"), (121, "Uribo"), (122, "Porky"), (123, "Wildbow"), (124, "Woodman"), (125, "Trent"), (126, "Taiju"), (127, "Bull Demon"), (128, "Minitaurus"), (129, "Sacred"), (130, "Morbol"), (131, "Ochu"), (132, "Great Morbol"), (133, "Behemoth"), (134, "King Behemoth"), (135, "Dark Behemoth"), (136, "Dragon"), (137, "Blue Dragon"), (138, "Red Dragon"), (139, "Hyudra"), (140, "Hydra"), (141, "Tiamat"), (144, "Byblos"), (145, "Steel Giant"), (150, "Apanda"), (151, "Serpentarius"), (152, "Holy Dragon (unused)"), (153, "Archaic Demon"), (154, "Ultima Demon"), ];
/// Weapons: (id, name); the count is at 0x1605 + id.
const WEAPONS: &[(u8, &str)] = &[(0, "Dagger"), (1, "Mythril Knife"), (2, "Blind Knife"), (3, "Mage Masher"), (4, "Platina Dagger"), (5, "Main Gauche"), (6, "Orichalcum"), (7, "Assassin Dagger"), (8, "Air Knife"), (9, "Zorlin Shape"), (10, "Hidden Knife"), (11, "Ninja Knife"), (12, "Short Edge"), (13, "Ninja Edge"), (14, "Spell Edge"), (15, "Sasuke Knife"), (16, "Iga Knife"), (17, "Koga Knife"), (18, "Broad Sword"), (19, "Long Sword"), (20, "Iron Sword"), (21, "Mythril Sword"), (22, "Blood Sword"), (23, "Coral Sword"), (24, "Ancient Sword"), (25, "Sleep Sword"), (26, "Platinum Sword"), (27, "Diamond Sword"), (28, "Ice Brand"), (29, "Rune Blade"), (30, "Nagrarock"), (31, "Materia Blade"), (32, "Defender"), (33, "Save the Queen"), (34, "Excalibur"), (35, "Ragnarok"), (36, "Chaos Blade"), (37, "Asura Knife"), (38, "Koutetsu Knife"), (39, "Bizen Boat"), (40, "Murasame"), (41, "Heaven's Cloud"), (42, "Kiyomori"), (43, "Muramasa"), (44, "Kikuichimoji"), (45, "Masamune"), (46, "Chirijiraden"), (47, "Battle Axe"), (48, "Giant Axe"), (49, "Slasher"), (50, "Rod"), (51, "Thunder Rod"), (52, "Flame Rod"), (53, "Ice Rod"), (54, "Poison Rod"), (55, "Wizard Rod"), (56, "Dragon Rod"), (57, "Faith Rod"), (58, "Oak Staff"), (59, "White Staff"), (60, "Healing Staff"), (61, "Rainbow Staff"), (62, "Wizard Staff"), (63, "Gold Staff"), (64, "Mace of Zeus"), (65, "Sage Staff"), (66, "Flail"), (67, "Flame Whip"), (68, "Morning Star"), (69, "Scorpion Tail"), (70, "Romanda Gun"), (71, "Mythril Gun"), (72, "Stone Gun"), (73, "Blaze Gun"), (74, "Glacier Gun"), (75, "Blast Gun"), (76, "Bow Gun"), (77, "Night Killer"), (78, "Cross Bow"), (79, "Poison Bow"), (80, "Hunting Bow"), (81, "Gastrafitis"), (82, "Long Bow"), (83, "Silver Bow"), (84, "Ice Bow"), (85, "Lightning Bow"), (86, "Windslash Bow"), (87, "Mythril Bow"), (88, "Ultimus Bow"), (89, "Yoichi Bow"), (90, "Perseus Bow"), (91, "Ramia Harp"), (92, "Bloody Strings"), (93, "Fairy Harp"), (94, "Battle Dict"), (95, "Monster Dict"), (96, "Papyrus Plate"), (97, "Madlemgen"), (98, "Javelin"), (99, "Spear"), (100, "Mythril Spear"), (101, "Partisan"), (102, "Oberisk"), (103, "Holy Lance"), (104, "Dragon Whisker"), (105, "Javelin"), (106, "Cypress Rod"), (107, "Battle Bamboo"), (108, "Musk Rod"), (109, "Iron Fan"), (110, "Gokuu Rod"), (111, "Ivory Rod"), (112, "Octagon Rod"), (113, "Whale Whisker"), (114, "C Bag"), (115, "FS Bag"), (116, "P Bag"), (117, "H Bag"), (118, "Persia"), (119, "Cashmere"), (120, "Ryozan Silk"), (121, "Shuriken"), (122, "Magic Shuriken"), (123, "Yagyu Darkness"), (124, "Fire Ball"), (125, "Water Ball"), (126, "Lightning Ball"), (127, "Escutcheon"), (128, "Buckler"), (129, "Bronze Shield"), (130, "Round Shield"), (131, "Mythril Shield"), (132, "Gold Shield"), (133, "Ice Shield"), (134, "Flame Shield"), (135, "Aegis Shield"), (136, "Diamond Shield"), (137, "Platina Shield"), (138, "Crystal Shield"), (139, "Genji Shield"), (140, "Kaiser Plate"), (141, "Venetian Shield"), (142, "Escutcheon"), ];
/// Heads: (id, name); the count is at 0x1605 + id.
const HEADS: &[(u8, &str)] = &[(143, "Leather Helmet"), (144, "Bronze Helmet"), (145, "Iron Helmet"), (146, "Barbuta"), (147, "Mythril Helmet"), (148, "Gold Helmet"), (149, "Cross Helmet"), (150, "Diamond Helmet"), (151, "Platina Helmet"), (152, "Circlet"), (153, "Crystal Helmet"), (154, "Genji Helmet"), (155, "Grand Helmet"), (156, "Leather Hat"), (157, "Feather Hat"), (158, "Red Hood"), (159, "Headgear"), (160, "Triangle Hat"), (161, "Green Beret"), (162, "Twist Headband"), (163, "Holy Miter"), (164, "Black Hood"), (165, "Golden Hairpin"), (166, "Flash Hat"), (167, "Thief Hat"), (168, "Cachusha"), (169, "Barette"), (170, "Ribbon"), ];
/// Armor: (id, name); the count is at 0x1605 + id.
const ARMOR: &[(u8, &str)] = &[(171, "Leather Armor"), (172, "Linen Cuirass"), (173, "Bronze Armor"), (174, "Chain Mail"), (175, "Mythril Armor"), (176, "Plate Mail"), (177, "Gold Armor"), (178, "Diamond Armor"), (179, "Platina Armor"), (180, "Carabini Mail"), (181, "Crystal Mail"), (182, "Genji Armor"), (183, "Reflect Mail"), (184, "Maximillian"), (185, "Clothes"), (186, "Leather Outfit"), (187, "Leather Vest"), (188, "Chain Vest"), (189, "Mythril Vest"), (190, "Adaman Vest"), (191, "Wizard Outfit"), (192, "Brigandine"), (193, "Judo Outfit"), (194, "Power Sleeve"), (195, "Earth Clothes"), (196, "Secret Clothes"), (197, "Black Costume"), (198, "Rubber Costume"), (199, "Linen Robe"), (200, "Silk Robe"), (201, "Wizard Robe"), (202, "Chameleon Robe"), (203, "White Robe"), (204, "Black Robe"), (205, "Light Robe"), (206, "Robe of Lords"), ];
/// Accessories: (id, name); the count is at 0x1605 + id.
const ACCESSORIES: &[(u8, &str)] = &[(207, "Battle Boots"), (208, "Spike Shoes"), (209, "Germinas Boots"), (210, "Rubber Shoes"), (211, "Feather Boots"), (212, "Sprint Shoes"), (213, "Red Shoes"), (214, "Power Wrist"), (215, "Genji Gauntlet"), (216, "Magic Gauntlet"), (217, "Bracer"), (218, "Reflect Ring"), (219, "Defense Ring"), (220, "Magic Ring"), (221, "Cursed Ring"), (222, "Angel Ring"), (223, "Diamond Armlet"), (224, "Jade Armlet"), (225, "108 Gems"), (226, "N-Kai Armlet"), (227, "Defense Armlet"), (228, "Small Mantle"), (229, "Leather Mantle"), (230, "Wizard Mantle"), (231, "Elf Mantle"), (232, "Dracula Mantle"), (233, "Feather Mantle"), (234, "Vanish Mantle"), (235, "Chantage"), (236, "Cherche"), (237, "Setiemson"), (238, "Salty Rage"), ];
/// Items: (id, name); the count is at 0x1605 + id.
const ITEMS: &[(u8, &str)] = &[(239, "Potion"), (240, "Hi-Potion"), (241, "X-Potion"), (242, "Ether"), (243, "Hi-Ether"), (244, "Elixir"), (245, "Antidote"), (246, "Eye Drop"), (247, "Echo Grass"), (248, "Maiden's Kiss"), (249, "Soft"), (250, "Holy Water"), (251, "Remedy"), (252, "Phoenix Down"), ];
