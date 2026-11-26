//! Attack on Titan (`atwin0000.dat`, 769,568 bytes) and Attack on Titan 2: Final Battle
//! (`Attack on Titan2_AOT2_PK_WIN0000.dat`, 3,687,964 bytes) on Windows. Both start with a
//! little-endian 16-bit word sum and 16-bit seed, then a DWORD XOR stream (state = state *
//! 0x5B1A7851 + 0xCE4E, advanced three times per word in the first game and once in the second)
//! over the rest of the file. The sum is the 16-bit word sum of the decrypted body. The decrypted
//! body opens with the completion figures as text, and the same figures follow as numbers at body
//! offset 0x604, which is how the layouts are recognised. The first game's body is the PS Vita
//! layout behind a 0x61C-byte header (money and item quantity rows as published in the public
//! Vita patch lists, checked on a public save); the second game's gift table and its offset
//! match the public PS4 patch list shifted by 0x114740, checked on a public save. The second
//! game's regiment funds (0x114D74, with the total acquired beside it) and Wings of Freedom
//! (0x247A4C) were matched to values read in the game from a save of its own, and so was the
//! crafting material table (0x146E98, 999 each).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND1: &str = "Attack on Titan save";
pub const KIND2: &str = "Attack on Titan 2 save";
const SIZE1: usize = 0x000B_BE20;
const SIZE2: usize = 0x0038_461C;
const P: usize = 4;

const MONEY1: usize = P + 0x650;
const ITEMS1: usize = P + 0x362BC;
const ITEM_ROWS1: usize = 81;
const ITEM_STRIDE1: usize = 0x30;
const FUNDS2: usize = P + 0x114D74;
const WINGS2: usize = P + 0x247A4C;
const EXP2: usize = P + 0x22C6B8;
/// Player level, stored one lower than shown (0 = level 1).
const LEVEL2: usize = P + 0x22C6BC;
/// Experience at level 120 (identical in two level-120 saves).
const EXP2_CAP: u32 = 605_770;
const MATERIALS2: usize = P + 0x146E98;
/// Crafting material rows. Rows 53-55, 73-74 and 115 on are never listed: a 193-hour save that
/// owns every other material still marks them differently, so what they hold is unknown. Row 72 is
/// marked like a material but never appears in the game's Materials list (an unused item).
const MATERIAL_ROWS2: usize = 115;
const MATERIAL_SKIP2: [usize; 6] = [53, 54, 55, 72, 73, 74];
const GIFTS2: usize = P + 0x14B698;
const GIFT_ROWS2: usize = 50;
const GIFT_STRIDE2: usize = 0x30;
/// Equipped skills: one bit per skill (bit n = byte n / 8, bit n % 8), from body 0x22C638.
/// Matched by loading saves with different sets of bits in the game and reading the Equip Skill
/// list. About Titans, Friendship, Upstanding Soldier and Negotiating Skill showed as equipped
/// whatever these bits held, so they are kept elsewhere; bit 21 and bits 0, 86-99, 172-174 and
/// 187 on matched no skill.
const SKILLS2: usize = P + 0x22C638;
const SKILL_BITS2: [(usize, &str); 168] = [
    (1, "Swoop Attack"), (2, "Spinning Slash"), (3, "Multi-Strike"), (4, "Wings of Freedom"), (5, "Multi-Dash"),
    (6, "Mid-Air Blade Swap"), (7, "Mid-Air First-Aid"), (8, "Air Move"), (9, "Blade Guard"),
    (10, "Lightning Speed"), (11, "Anchor Spear"), (12, "Free Shooter"), (13, "Extra Bullets"),
    (14, "Showdown Equipment Handling"), (15, "Mid-Air Gas Swap"), (16, "Exaltation"), (17, "Fresh Start"),
    (18, "Lone Wolf"), (19, "Careful Selection"), (20, "Revenge"), (22, "Auto Reload"), (23, "Full Power"),
    (24, "Weapons Inspection"), (25, "Elimination Bounty"), (26, "Wings of Hope"), (27, "Master Rider"),
    (28, "Point-Blank"), (29, "Reload Ace"), (30, "Willpower"), (31, "Predicament"), (32, "Survival Instincts"),
    (33, "Alms of the Goddess"), (34, "Goddess Grin"), (35, "Ready For Anything"), (36, "Well-Prepared"),
    (37, "Appeal"), (38, "Stealth"), (39, "Secret Maneuver"), (40, "Secret Maneuver Lv. 2"),
    (41, "Secret Maneuver Lv. 3"), (42, "Sudden Death"), (43, "Sudden Death Lv. 2"), (44, "Sudden Death Lv. 3"),
    (45, "Dueler"), (46, "Dueler Lv. 2"), (47, "Dueler Lv. 3"), (48, "Surprise Attack"),
    (49, "Surprise Attack Lv. 2"), (50, "Surprise Attack Lv. 3"), (51, "Strong Bonds"), (52, "Strength of Unity"),
    (53, "Siege Formation"), (54, "Natural Tactics"), (55, "Motivator"), (56, "Motivator Lv. 2"),
    (57, "Motivator Lv. 3"), (58, "Chase Down"), (59, "Chase Down Lv. 2"), (60, "Chase Down Lv. 3"),
    (61, "Decisive Battle"), (62, "Devotee"), (63, "Research Grounding"), (64, "Research Heart"),
    (65, "Item Meister"), (66, "Item Meister Lv. 2"), (67, "Item Meister Lv. 3"), (68, "Tail-End Rise"),
    (69, "Easily Pleased"), (70, "Blade Art"), (71, "Ultimate Blade Art"), (72, "Back From The Brink"),
    (73, "Never Say Die"), (74, "Never Say Die Lv. 2"), (75, "Never Say Die Lv. 3"), (76, "Advanced ODM Skills"),
    (77, "Advanced ODM Skills Lv. 2"), (78, "Advanced ODM Skills Lv. 3"), (79, "Gas Saver"), (80, "Gas Saver Lv. 2"),
    (81, "Gas Saver Lv. 3"), (82, "Blade Saver"), (83, "Blade Saver Lv. 2"), (84, "Blade Saver Lv. 3"),
    (85, "Titan Hunter"), (100, "Strength +3 (1)"), (101, "Strength +3 (2)"), (102, "Strength +3 (3)"),
    (103, "Strength +3 (4)"), (104, "Strength +3 (5)"), (105, "Strength +5 (1)"), (106, "Strength +5 (2)"),
    (107, "Strength +5 (3)"), (108, "Strength +5 (4)"), (109, "Strength +5 (5)"), (110, "Strength +10 (1)"),
    (111, "Strength +10 (2)"), (112, "Dexterity +3 (1)"), (113, "Dexterity +3 (2)"), (114, "Dexterity +3 (3)"),
    (115, "Dexterity +3 (4)"), (116, "Dexterity +3 (5)"), (117, "Dexterity +5 (1)"), (118, "Dexterity +5 (2)"),
    (119, "Dexterity +5 (3)"), (120, "Dexterity +5 (4)"), (121, "Dexterity +5 (5)"), (122, "Dexterity +10 (1)"),
    (123, "Dexterity +10 (2)"), (124, "Concentration +3 (1)"), (125, "Concentration +3 (2)"),
    (126, "Concentration +3 (3)"), (127, "Concentration +3 (4)"), (128, "Concentration +3 (5)"),
    (129, "Concentration +5 (1)"), (130, "Concentration +5 (2)"), (131, "Concentration +5 (3)"),
    (132, "Concentration +5 (4)"), (133, "Concentration +5 (5)"), (134, "Concentration +10 (1)"),
    (135, "Concentration +10 (2)"), (136, "Agility +3 (1)"), (137, "Agility +3 (2)"), (138, "Agility +3 (3)"),
    (139, "Agility +3 (4)"), (140, "Agility +3 (5)"), (141, "Agility +5 (1)"), (142, "Agility +5 (2)"),
    (143, "Agility +5 (3)"), (144, "Agility +5 (4)"), (145, "Agility +5 (5)"), (146, "Agility +10 (1)"),
    (147, "Agility +10 (2)"), (148, "Health +3 (1)"), (149, "Health +3 (2)"), (150, "Health +3 (3)"),
    (151, "Health +3 (4)"), (152, "Health +3 (5)"), (153, "Health +5 (1)"), (154, "Health +5 (2)"),
    (155, "Health +5 (3)"), (156, "Health +5 (4)"), (157, "Health +5 (5)"), (158, "Health +10 (1)"),
    (159, "Health +10 (2)"), (160, "Leadership +3 (1)"), (161, "Leadership +3 (2)"), (162, "Leadership +3 (3)"),
    (163, "Leadership +3 (4)"), (164, "Leadership +3 (5)"), (165, "Leadership +5 (1)"), (166, "Leadership +5 (2)"),
    (167, "Leadership +5 (3)"), (168, "Leadership +5 (4)"), (169, "Leadership +5 (5)"), (170, "Leadership +10 (1)"),
    (171, "Leadership +10 (2)"), (175, "Good Treatment"), (176, "Good Treatment Lv. 2"),
    (177, "Good Treatment Lv. 3"), (178, "Courageousness"), (179, "Courageousness Lv. 2"),
    (180, "Courageousness Lv. 3"), (181, "Treasure Drive"), (182, "Treasure Drive Lv. 2"),
    (183, "Treasure Drive Lv. 3"), (184, "Robbery"), (185, "Robbery Lv. 2"), (186, "Robbery Lv. 3"),
];
/// Friendship records (0x180 bytes each): u16 points, then the level (0..=10) the People Log shows.
const FRIENDS2: usize = P + 0x166978;
const FRIEND_STRIDE2: usize = 0x180;
/// Record number of each character, matched by loading test levels in the game.
const FRIEND_NAMES2: [(usize, &str); 39] = [
    (0, "Eren Jaeger"), (1, "Mikasa Ackermann"), (2, "Armin Arlelt"), (3, "Reiner Braun"), (4, "Bertholdt Hoover"),
    (5, "Annie Leonhart"), (6, "Jean Kirschtein"), (7, "Conny Springer"), (8, "Sasha Braus"), (9, "Christa Lenz"),
    (10, "Ymir"), (11, "Levi"), (12, "Hange Zo\u{eb}"), (13, "Erwin Smith"), (14, "Marco Bodt"), (15, "Thomas Wagner"),
    (16, "Mina Carolina"), (17, "Miche Zacharius"), (18, "Oruo Bozad"), (19, "Petra Ral"), (20, "Gunther Schultz"),
    (21, "Eld Gin"), (22, "Moblit"), (23, "Ian Dietrich"), (24, "Rico Brzenska"), (25, "Mitabi Jarnach"),
    (26, "Nile Dawk"), (27, "Marlo Freudenberg"), (28, "Hitch"), (29, "Hannes"), (30, "Dot Pyxis"), (31, "Keith Sadies"),
    (32, "Kitz Woermann"), (33, "Dhalis Zachary"), (34, "Nanaba"), (35, "Gelgar"), (36, "Daz"), (47, "Pastor Nick"),
    (48, "Merchant Association Boss"),
];

/// Training Field courses, one 0x50-byte record each: u32 training points, then the training
/// level (one per 100 points). Matched by training once in each course in the game and comparing
/// the save before and after; a completed save holds 3,000 points and level 30 in each.
const TRAINING2: usize = P + 0x18FA68;
const TRAINING_STRIDE2: usize = 0x50;
const TRAINING_POINTS_PER_LEVEL2: u32 = 100;
const TRAINING_MAX2: u8 = 30;
/// The byte after the level, 5 in every course record seen.
const TRAINING_TAG2: u8 = 5;
/// Titan Research Room: u32 EXP, then the room's level, matched to the game's own screen (level 1 at
/// 1,550 EXP, level 4 at 7,750); a completed save holds level 10 with 33,000 EXP.
const RESEARCH2: usize = P + 0x1859B8;
const RESEARCH_EXP_MAX2: u32 = 33_000;
const RESEARCH_LEVEL_MAX2: u8 = 10;
const TRAINING_NAMES2: [&str; 3] = ["Omni-direction Mobility Training", "Hand-to-hand Combat Training", "Lecture"];

/// Owned gear: Blades, Scabbards and ODM gear, 99 rows of 0x40 bytes each (the rows' offsets
/// match the public PS4 patch list shifted by 0x114740). A row holds the piece's number at +0x18
/// (0xFF: free) and its upgrade at +0x19 (u16). The game lists the equipped piece first, then the
/// other rows in order, and has no separate count: filling the first free row adds a piece, which
/// it shows as new. Names and upgrade values were read in the game from saves of its own: 1 to 99
/// show as "+1" to "+99" and 0x1C6 as "+99" with a star, the value of every fully upgraded piece.
const GEAR2: [(&str, usize); 3] = [("Blades", P + 0x1363C0), ("Scabbards", P + 0x137C80), ("ODM gear", P + 0x139540)];
const GEAR_ROWS2: usize = 99;
const GEAR_STRIDE2: usize = 0x40;
const GEAR_FREE2: u8 = 0xFF;
/// `(number, blade, scabbard, ODM gear)`; an empty name is a piece not seen in that list.
const GEAR_NAMES2: [(u8, &str, &str, &str); 21] = [
    (51, "Shadow Falcon", "", ""),
    (87, "Perfected Blade - Ultimate", "Perfected Scabbard - Ultimate", "Perfected Gear - Ultimate"),
    (92, "Evil Edge", "Evil Coffin", "Evil Drive"),
    (93, "Chrono Cutter I", "Chrono Booster I", "Chrono Reaper I"),
    (97, "Phos Chrono Cutter", "Phos Chrono Booster", "Phos Chrono Reaper"),
    (102, "Fury Blade - Ultimate", "Fury Scabbard - Ultimate", "Fury Gear - Ultimate"),
    (103, "Divine Reinforced Blade", "Divine Reinforced Scabbard", "Divine Reinforced Gear"),
    (104, "Koenigmesser II", "Koenigscheide II", "Koenighaken II"),
    (105, "Kusanagi Zero", "Tsunami Zero", "Brilliance Zero"),
    (106, "Blood Rapier", "Blood Wing", "Blood Crest"),
    (107, "Ultimate Duster", "Heavy Scabbard Ver.6 - Red Hare", "Heavy Gear Ver.6 - Bright Burn"),
    (108, "Heavy Blade Ver.6 - Crimson Blossom", "Crescent Sheath II", "Crescent Gear II"),
    (109, "Crescent Blade II", "Shadow Hurricane", "Shadow Moonbow"),
    (110, "Shadow Kestrel", "Platinum Fliegen", "Platinum Motor"),
    (111, "Platinum Dolch", "Demon God Scabbard Mk II", "Demon God Gear Mk II"),
    (112, "Demon God Blade Mk II", "Onogoro Hyaku-Shiki Mk II", "Kagutsuchi Hyaku-Shiki Mk II"),
    (113, "", "Helleisen II", "Hellbogen II"),
    (114, "Hellklinge II", "Ultimate Dream Sheath", "Soaring Assault Gear"),
    (115, "Explosive Skysplitter Gunblade", "Blazing Axel", "Blazing Machinery"),
    (116, "Blazing Blade", "Omega Sol Fuel", "Omega Sol Unit"),
    (117, "Omega Sol Slicer", "", ""),
];
/// The upgrade of a fully upgraded piece, shown as "+99" with a star.
const GEAR_UPGRADE_MAX2: u64 = 0x1C6;

fn w16(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

fn w32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

fn stream(body: &mut [u8], seed: u32, advances: usize) {
    let mut st = seed;
    for c in body.chunks_exact_mut(4) {
        for _ in 0..advances {
            st = st.wrapping_mul(0x5B1A_7851).wrapping_add(0xCE4E);
        }
        let w = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) ^ st;
        c.copy_from_slice(&w.to_le_bytes());
    }
}

fn word_sum(body: &[u8]) -> u16 {
    body.chunks_exact(2).fold(0u16, |a, c| a.wrapping_add(u16::from_le_bytes([c[0], c[1]])))
}

fn advances(len: usize) -> Option<usize> {
    match len {
        SIZE1 => Some(3),
        SIZE2 => Some(1),
        _ => None,
    }
}

/// The completion figures at the top of the body: text (UTF-16 in the first game, ASCII in the
/// second) that must equal the numbers stored from body offset 0x604.
fn figures_agree(p: &[u8], first: bool) -> bool {
    let text: String = if first {
        p[P + 0x200..P + 0x240].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).filter_map(|u| char::from_u32(u32::from(u))).collect()
    } else {
        p[P..P + 0x40].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect()
    };
    if !text.chars().all(|c| c.is_ascii_digit() || c == ' ') {
        return false;
    }
    let mut nums: Vec<u32> = text.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    if first && nums.len() == 7 {
        nums.remove(0); // the leading 0000000 block
    }
    nums.len() >= 5 && nums.iter().take(5).enumerate().all(|(i, &n)| w32(p, P + 0x604 + 4 * i) == n)
}

fn unpack(d: &[u8]) -> Option<(Vec<u8>, bool)> {
    let adv = advances(d.len())?;
    let mut p = d.to_vec();
    stream(&mut p[P..], u32::from(w16(d, 2)), adv);
    let first = d.len() == SIZE1;
    (word_sum(&p[P..]) == w16(d, 0) && figures_agree(&p, first)).then_some((p, first))
}

/// The flag byte after a material's quantity, for an owned row given the flag it has while unowned.
/// Pairs seen across a new save and two completed ones: 0x0C and 0x04 become 0x21, 0x0D becomes 0x29.
fn owned_flag(flag: u8) -> Option<u8> {
    match flag {
        0x0C | 0x04 => Some(0x21),
        0x0D => Some(0x29),
        _ => None,
    }
}

fn material_rows() -> impl Iterator<Item = usize> {
    (0..MATERIAL_ROWS2).filter(|i| !MATERIAL_SKIP2.contains(i))
}

/// A Training Field course record: the tag byte the game writes, a level up to the top one and
/// no more points than the top level. The screen lists exactly these, and only these are synced
/// on save, so nothing the screen doesn't show is changed.
fn training_course(p: &[u8], at: usize) -> bool {
    p[at + 5] == TRAINING_TAG2 && p[at + 4] <= TRAINING_MAX2 && w32(p, at) <= u32::from(TRAINING_MAX2) * TRAINING_POINTS_PER_LEVEL2
}

fn pack(p: &mut Vec<u8>) {
    let Some(adv) = advances(p.len()) else { return };
    if p.len() == SIZE2 {
        // A course whose training points were changed gets the level those points reach (one per
        // 100), as in every save the game writes; an untouched course already agrees.
        for k in 0..TRAINING_NAMES2.len() {
            let at = TRAINING2 + k * TRAINING_STRIDE2;
            let level = w32(p, at) / TRAINING_POINTS_PER_LEVEL2;
            if training_course(p, at) && level != u32::from(p[at + 4]) {
                p[at + 4] = level as u8;
            }
        }
        // A material given a quantity it didn't have is marked as owned, as the game does.
        for i in material_rows() {
            let at = MATERIALS2 + i * GIFT_STRIDE2;
            if w16(p, at) != 0 {
                if let Some(f) = owned_flag(p[at + 2]) {
                    p[at + 2] = f;
                }
            }
        }
    }
    let ck = word_sum(&p[P..]);
    p[..2].copy_from_slice(&ck.to_le_bytes());
    let seed = u32::from(w16(p, 2));
    stream(&mut p[P..], seed, adv);
}

/// The gear you own in each list, plus the first free row for adding a piece. A piece's number
/// can only be set to a name seen in that list; an upgrade value the game hasn't been seen to show
/// is listed by its number.
fn gear(b: &mut Builder, p: &[u8]) {
    for (list, (title, start)) in GEAR2.iter().enumerate() {
        let names: Vec<(String, u64)> = GEAR_NAMES2
            .iter()
            .map(|g| (g.0, [g.1, g.2, g.3][list]))
            .filter(|(_, n)| !n.is_empty())
            .map(|(id, n)| (n.to_string(), u64::from(id)))
            .collect();
        let mut free_shown = false;
        for i in 0..GEAR_ROWS2 {
            let at = start + i * GEAR_STRIDE2;
            let id = p[at + 0x18];
            let free = id == GEAR_FREE2;
            if free && free_shown {
                continue;
            }
            let mut items = names.clone();
            if free {
                free_shown = true;
                items.insert(0, ("(free slot)".to_string(), u64::from(GEAR_FREE2)));
            } else if !items.iter().any(|(_, v)| *v == u64::from(id)) {
                items.insert(0, (format!("Item {id}"), u64::from(id)));
            }
            let row = format!("Slot {:02}", i + 1);
            b.pick(&[title, &row, "Piece"], at + 0x18, Kind::U8, &items);
            let up = u64::from(w16(p, at + 0x19));
            let mut ups: Vec<(String, u64)> = std::iter::once(("None".to_string(), 0))
                .chain((1..=99).map(|n| (format!("+{n}"), n)))
                .chain(std::iter::once(("+99 with star".to_string(), GEAR_UPGRADE_MAX2)))
                .collect();
            if !ups.iter().any(|(_, v)| *v == up) {
                ups.insert(0, (format!("Value {up}"), up));
            }
            b.pick(&[title, &row, "Upgrade"], at + 0x19, Kind::U16, &ups);
        }
    }
}

pub fn looks(d: &[u8]) -> bool {
    unpack(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (p, first) = unpack(d).ok_or("not an Attack on Titan save")?;
    let mut b = Builder::new(&p);
    if first {
        b.ranged(&["Resources", "Funds"], MONEY1, Kind::U32, 0, 99_999_999);
        for i in 0..ITEM_ROWS1 {
            let at = ITEMS1 + i * ITEM_STRIDE1;
            if w32(&p, at) != 0 && w32(&p, at) <= 999 {
                b.ranged(&["Items", &format!("Item {:02}", i + 1)], at, Kind::U16, 0, 999);
            }
        }
        Ok(b.finish(KIND1, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. The stream cipher and the word checksum are written again. Items are shown by their number in the save, not by name; only items you own are listed.")))
    } else {
        // Current funds sit beside the total acquired; both stay within the game's cap. Funds edited
        // above the recorded total must still read back, so the two are not compared.
        if w32(&p, FUNDS2) <= 99_999_999 && w32(&p, FUNDS2 + 4) <= 99_999_999 {
            b.ranged(&["Resources", "Regiment funds"], FUNDS2, Kind::U32, 0, 99_999_999);
        }
        if w32(&p, WINGS2) <= 999 {
            b.ranged(&["Resources", "Wings of Freedom"], WINGS2, Kind::U32, 0, 999);
        }
        if p[LEVEL2] < 120 {
            b.bits(&["Player", "Level"], LEVEL2, Kind::U8, 0, 8, 1, 1, 120);
        }
        if w32(&p, EXP2) <= EXP2_CAP {
            b.ranged(&["Player", "Experience"], EXP2, Kind::U32, 0, EXP2_CAP as i64);
        }
        if w32(&p, RESEARCH2) <= RESEARCH_EXP_MAX2 && p[RESEARCH2 + 4] <= RESEARCH_LEVEL_MAX2 {
            b.ranged(&["Titan Research Room", "Level"], RESEARCH2 + 4, Kind::U8, 0, i64::from(RESEARCH_LEVEL_MAX2));
            b.ranged(&["Titan Research Room", "EXP"], RESEARCH2, Kind::U32, 0, i64::from(RESEARCH_EXP_MAX2));
        }
        for (k, name) in TRAINING_NAMES2.iter().enumerate() {
            let at = TRAINING2 + k * TRAINING_STRIDE2;
            if training_course(&p, at) {
                b.ranged(&["Training", name, "Training points"], at, Kind::U32, 0, i64::from(TRAINING_MAX2) * i64::from(TRAINING_POINTS_PER_LEVEL2));
            }
        }
        // Every material is listed, owned or not; one you don't own yet is marked as owned on save.
        for i in material_rows() {
            let at = MATERIALS2 + i * GIFT_STRIDE2;
            let flag = p[at + 2];
            let known = p[at + 3] == 0 && (matches!(flag, 0x21 | 0x29) || owned_flag(flag).is_some());
            if known && w16(&p, at) <= 999 {
                b.ranged(&["Crafting materials", &format!("Material {:02}", i + 1)], at, Kind::U16, 0, 999);
            }
        }
        // Only characters you've met (points or a level set) are listed.
        for (k, name) in FRIEND_NAMES2 {
            let at = FRIENDS2 + k * FRIEND_STRIDE2;
            if (w16(&p, at) != 0 || p[at + 2] != 0) && p[at + 2] <= 10 {
                b.ranged(&["Friendship", name], at + 2, Kind::U8, 0, 10);
            }
            // Points add up over every level (1,000 at level 10); the bar is the share toward the next level.
            if (w16(&p, at) != 0 || p[at + 2] != 0) && w16(&p, at) <= 1000 {
                b.ranged(&["Friendship points", name], at, Kind::U16, 0, 1000);
            }
        }
        for (bit, name) in SKILL_BITS2 {
            super::retro::flag(&mut b, &["Skills", name, "Equipped"], SKILLS2 + bit / 8, (bit % 8) as u32);
        }
        gear(&mut b, &p);
        for i in 0..GIFT_ROWS2 {
            let at = GIFTS2 + i * GIFT_STRIDE2;
            // A gift row the game has set up (flag 0x01, or 0x41) is listed even at 0: giving it a
            // quantity is all the game needs to show it.
            if w16(&p, at) <= 99 && (w16(&p, at) != 0 || matches!(p[at + 2], 0x01 | 0x41)) {
                b.ranged(&["Gifts", &format!("Gift {:02}", i + 1)], at, Kind::U16, 0, 99);
            }
        }
        Ok(b.finish(KIND2, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. The stream cipher and the word checksum are written again. Every material and gift is listed; giving one you don't own a quantity adds it. Equipping skills doesn't check your SP.")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(first: bool) -> Vec<u8> {
        let size = if first { SIZE1 } else { SIZE2 };
        let mut p = vec![0u8; size];
        let nums = [4u32, 6, 29, 40, 3, 51];
        if first {
            let text = "0000000 004 06 29 040 003 051";
            for (i, u) in text.encode_utf16().enumerate() {
                p[P + 0x200 + 2 * i..P + 0x202 + 2 * i].copy_from_slice(&u.to_le_bytes());
            }
        } else {
            p[P..P + 17].copy_from_slice(b"004 06 29 040 003");
        }
        for (i, n) in nums.iter().take(if first { 6 } else { 5 }).enumerate() {
            p[P + 0x604 + 4 * i..P + 0x608 + 4 * i].copy_from_slice(&n.to_le_bytes());
        }
        p[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        p
    }

    /// Owned gear is named and its upgrade shown; the first free row takes a new piece.
    #[test]
    fn second_game_gear() {
        let mut p = blank(false);
        for (_, start) in GEAR2 {
            for i in 0..GEAR_ROWS2 {
                p[start + i * GEAR_STRIDE2 + 0x18] = GEAR_FREE2;
            }
        }
        let blades = GEAR2[0].1;
        p[blades + 0x18..blades + 0x1B].copy_from_slice(&[87, 0xC6, 0x01]);
        p[blades + GEAR_STRIDE2 + 0x18..blades + GEAR_STRIDE2 + 0x1B].copy_from_slice(&[105, 0x63, 0]);
        p[blades + 2 * GEAR_STRIDE2 + 0x18..blades + 2 * GEAR_STRIDE2 + 0x1B].copy_from_slice(&[70, 0x63, 0x01]);
        pack(&mut p);
        let d = p;
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Blades/Slot 01/Piece").unwrap(), "Perfected Blade - Ultimate");
        assert_eq!(doc.view.pointer("/Blades/Slot 01/Upgrade").unwrap(), "+99 with star");
        assert_eq!(doc.view.pointer("/Blades/Slot 02/Piece").unwrap(), "Kusanagi Zero");
        assert_eq!(doc.view.pointer("/Blades/Slot 02/Upgrade").unwrap(), "+99");
        // A piece or upgrade value not seen in the game is shown by its number.
        assert_eq!(doc.view.pointer("/Blades/Slot 03/Piece").unwrap(), "Item 70");
        assert_eq!(doc.view.pointer("/Blades/Slot 03/Upgrade").unwrap(), "Value 355");
        doc.set("/Blades/Slot 03/Upgrade", "+16".into()).unwrap();
        // Only the first free row is listed; a list with nothing owned shows just that row.
        assert_eq!(doc.view.pointer("/Blades/Slot 04/Piece").unwrap(), "(free slot)");
        assert!(doc.view.pointer("/Blades/Slot 05").is_none());
        assert_eq!(doc.view.pointer("/ODM gear").unwrap().as_object().unwrap().len(), 1);
        // A blade name can't go in the scabbard list, and an owned piece can't be freed.
        assert!(doc.set("/Scabbards/Slot 01/Piece", "Kusanagi Zero".into()).is_err());
        assert!(doc.set("/Blades/Slot 01/Piece", "(free slot)".into()).is_err());
        doc.set("/Blades/Slot 04/Piece", "Blood Rapier".into()).unwrap();
        doc.set("/Blades/Slot 02/Upgrade", "+99 with star".into()).unwrap();
        doc.set("/ODM gear/Slot 01/Piece", "Crescent Gear II".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        let (q, _) = unpack(&out).unwrap();
        assert_eq!(&q[blades + GEAR_STRIDE2 + 0x18..][..3], &[105, 0xC6, 0x01]);
        assert_eq!(&q[blades + 3 * GEAR_STRIDE2 + 0x18..][..3], &[106, 0, 0]);
        assert_eq!(q[GEAR2[2].1 + 0x18], 108);
        assert_eq!(&q[blades + 2 * GEAR_STRIDE2 + 0x18..][..3], &[70, 16, 0]);
        assert_eq!(q[blades + 4 * GEAR_STRIDE2 + 0x18], GEAR_FREE2);
    }

    #[test]
    fn first_game_round_trip() {
        let mut p = blank(true);
        p[MONEY1..MONEY1 + 4].copy_from_slice(&1234u32.to_le_bytes());
        p[ITEMS1 + 7 * ITEM_STRIDE1..ITEMS1 + 7 * ITEM_STRIDE1 + 4].copy_from_slice(&392u32.to_le_bytes());
        pack(&mut p);
        let d = p;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Resources/Funds", 99_999_999.into()).unwrap();
        doc.set("/Items/Item 08", 999.into()).unwrap();
        assert!(doc.set("/Items/Item 08", 1000.into()).is_err());
        assert!(doc.view.pointer("/Items/Item 01").is_none());
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(parse(&out).unwrap().view.pointer("/Resources/Funds").unwrap(), 99_999_999);
    }

    #[test]
    fn second_game_training() {
        let mut p = blank(false);
        let course = |k: usize| TRAINING2 + k * TRAINING_STRIDE2;
        // Mid-way through level 0 in the first course, level 10 in the second, a third the game
        // wouldn't write (more points than the top level).
        p[course(0)..course(0) + 12].copy_from_slice(&[70, 0, 0, 0, 0, 5, 0, 0, 3, 0, 0, 0]);
        p[course(1)..course(1) + 6].copy_from_slice(&[0xE8, 0x03, 0, 0, 10, 5]);
        p[course(2)..course(2) + 6].copy_from_slice(&[0x88, 0x13, 0, 0, 7, 5]);
        pack(&mut p);
        let mut doc = parse(&p).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), p, "untouched training round-trips");
        assert_eq!(doc.view.pointer("/Training/Omni-direction Mobility Training/Training points").unwrap(), 70);
        assert_eq!(doc.view.pointer("/Training/Hand-to-hand Combat Training/Training points").unwrap(), 1000);
        assert!(doc.view.pointer("/Training/Lecture").is_none(), "a record the game wouldn't write is not shown");
        assert!(doc.set("/Training/Hand-to-hand Combat Training/Training points", 3001.into()).is_err());
        doc.set("/Training/Omni-direction Mobility Training/Training points", 3000.into()).unwrap();
        doc.set("/Training/Hand-to-hand Combat Training/Training points", 455.into()).unwrap();
        let (q, _) = unpack(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(&q[course(0)..course(0) + 12], &[0xB8, 0x0B, 0, 0, 30, 5, 0, 0, 3, 0, 0, 0], "3,000 points, level 30, sessions kept");
        assert_eq!(&q[course(1)..course(1) + 6], &[0xC7, 0x01, 0, 0, 4, 5], "455 points is level 4");
        assert_eq!(&q[course(2)..course(2) + 6], &[0x88, 0x13, 0, 0, 7, 5], "a course not shown is left alone");
        // Points and level that disagree (never seen in a save the game wrote): the level follows
        // the points on save.
        let mut p = blank(false);
        p[course(0)..course(0) + 6].copy_from_slice(&[0xE8, 0x03, 0, 0, 7, 5]);
        pack(&mut p);
        let doc = parse(&p).unwrap();
        assert_eq!(doc.view.pointer("/Training/Omni-direction Mobility Training/Training points").unwrap(), 1000);
        let (q, _) = unpack(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(&q[course(0)..course(0) + 6], &[0xE8, 0x03, 0, 0, 10, 5]);
    }

    #[test]
    fn second_game_round_trip() {
        let mut p = blank(false);
        p[GIFTS2 + 3 * GIFT_STRIDE2..GIFTS2 + 3 * GIFT_STRIDE2 + 2].copy_from_slice(&7u16.to_le_bytes());
        p[MATERIALS2 + 3 * GIFT_STRIDE2..MATERIALS2 + 3 * GIFT_STRIDE2 + 4].copy_from_slice(&[0x61, 0x03, 0x29, 0x00]);
        p[FRIENDS2 + 29 * FRIEND_STRIDE2..FRIENDS2 + 29 * FRIEND_STRIDE2 + 3].copy_from_slice(&[0x0F, 0x27, 3]);
        p[FRIENDS2..FRIENDS2 + 3].copy_from_slice(&[40, 0, 0]);
        p[GIFTS2 + 9 * GIFT_STRIDE2 + 2] = 0x01;
        // Unowned materials as a new game marks them.
        p[MATERIALS2 + 5 * GIFT_STRIDE2 + 2] = 0x0D;
        p[MATERIALS2 + 6 * GIFT_STRIDE2 + 2] = 0x0C;
        p[MATERIALS2 + 53 * GIFT_STRIDE2 + 2] = 0x0D;
        pack(&mut p);
        let d = p;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Gifts/Gift 04", 99.into()).unwrap();
        assert_eq!(doc.view.pointer("/Gifts/Gift 10").unwrap(), 0);
        assert!(doc.view.pointer("/Gifts/Gift 11").is_none());
        // An all-zero row is not a material layout the game writes, so it stays hidden.
        assert!(doc.view.pointer("/Crafting materials/Material 01").is_none());
        doc.set("/Crafting materials/Material 04", 999.into()).unwrap();
        assert!(doc.set("/Crafting materials/Material 04", 1000.into()).is_err());
        doc.set("/Resources/Regiment funds", 5000.into()).unwrap();
        doc.set("/Resources/Regiment funds", 12_345.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(parse(&out).unwrap().view.pointer("/Resources/Regiment funds").unwrap(), 12_345);
        doc.set("/Resources/Wings of Freedom", 999.into()).unwrap();
        assert!(doc.set("/Resources/Wings of Freedom", 1000.into()).is_err());
        assert!(doc.view.pointer("/Friendship/Levi").is_none());
        assert_eq!(doc.view.pointer("/Friendship/Hannes").unwrap(), 3);
        doc.set("/Friendship/Hannes", 10.into()).unwrap();
        assert!(doc.set("/Friendship/Hannes", 11.into()).is_err());
        doc.set("/Titan Research Room/Level", 10.into()).unwrap();
        assert!(doc.set("/Titan Research Room/Level", 11.into()).is_err());
        doc.set("/Titan Research Room/EXP", 33_000.into()).unwrap();
        assert!(doc.set("/Titan Research Room/EXP", 33_001.into()).is_err());
        doc.set("/Player/Level", 120.into()).unwrap();
        assert!(doc.set("/Player/Level", 121.into()).is_err());
        doc.set("/Player/Experience", 605_770.into()).unwrap();
        assert!(doc.set("/Player/Experience", 605_771.into()).is_err());
        assert!(doc.set("/Gifts/Gift 04", 100.into()).is_err());
        // Friendship points: Hannes's 9,999 is past the game's 1,000, so only Eren's are listed.
        assert!(doc.view.pointer("/Friendship points/Hannes").is_none());
        assert_eq!(doc.view.pointer("/Friendship points/Eren Jaeger").unwrap(), 40);
        doc.set("/Friendship points/Eren Jaeger", 1000.into()).unwrap();
        assert!(doc.set("/Friendship points/Eren Jaeger", 1001.into()).is_err());
        // Unowned materials are listed at 0 and become owned once given a quantity.
        assert_eq!(doc.view.pointer("/Crafting materials/Material 06").unwrap(), 0);
        assert!(doc.view.pointer("/Crafting materials/Material 54").is_none());
        doc.set("/Crafting materials/Material 06", 50.into()).unwrap();
        doc.set("/Crafting materials/Material 07", 3.into()).unwrap();
        // Equipped skills: bit 30 is Willpower, bit 185 Robbery Lv. 2.
        assert_eq!(doc.view.pointer("/Skills/Willpower/Equipped").unwrap(), "No");
        doc.set("/Skills/Willpower/Equipped", "Yes".into()).unwrap();
        doc.set("/Skills/Robbery Lv. 2/Equipped", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        let (q, _) = unpack(&out).unwrap();
        assert_eq!(q[SKILLS2 + 3], 1 << 6);
        assert_eq!(q[SKILLS2 + 23], 1 << 1);
        assert_eq!(&q[MATERIALS2 + 5 * GIFT_STRIDE2..][..4], &[50, 0, 0x29, 0]);
        assert_eq!(&q[MATERIALS2 + 6 * GIFT_STRIDE2..][..4], &[3, 0, 0x21, 0]);
        assert_eq!(&q[MATERIALS2 + 53 * GIFT_STRIDE2..][..4], &[0, 0, 0x0D, 0]);
        assert_eq!(&q[FRIENDS2..][..2], &1000u16.to_le_bytes());
    }

    #[test]
    fn funds_edit_changes_only_the_funds_bytes() {
        let mut p = blank(false);
        p[MATERIALS2 + 3 * GIFT_STRIDE2..MATERIALS2 + 3 * GIFT_STRIDE2 + 4].copy_from_slice(&[0x61, 0x03, 0x29, 0x00]);
        p[MATERIALS2 + 5 * GIFT_STRIDE2 + 2] = 0x0D;
        pack(&mut p);
        let (before, _) = unpack(&p).unwrap();
        let mut doc = parse(&p).unwrap();
        doc.set("/Resources/Regiment funds", 777.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let (after, _) = unpack(&out).unwrap();
        let changed: Vec<usize> = (P..before.len()).filter(|&i| before[i] != after[i]).collect();
        assert!(!changed.is_empty() && changed.iter().all(|i| (FUNDS2..FUNDS2 + 4).contains(i)), "{changed:?}");
    }

    #[test]
    fn rejects_other_data() {
        assert!(!looks(&vec![0u8; SIZE1]));
        let mut d = blank(true);
        pack(&mut d);
        d[100] ^= 1;
        assert!(!looks(&d));
    }

    /// Research: real saves from SAVE_EDITOR_AOT (files separated by `|`) go through the whole
    /// decode chain, open on their own screen and survive an edit.
    #[test]
    #[ignore]
    fn real_saves_decode_and_edit() {
        use crate::formats::{decode_with, Content, Options};
        let list = std::env::var("SAVE_EDITOR_AOT").expect("SAVE_EDITOR_AOT");
        for f in list.split('|').filter(|s| !s.is_empty()) {
            let path = std::path::Path::new(f);
            let raw = std::fs::read(path).unwrap();
            let d = decode_with(&raw, &Options::for_path(path));
            let Content::Tree(mut t) = d.content else { panic!("{f}: not a tree") };
            println!("{f}: {} {}", t.kind_name, t.view);
            assert_eq!(t.to_bytes().unwrap(), raw, "{f}: unchanged round trip");
            let first = t.kind_name == KIND1;
            let (path_, hi) = if first { ("/Resources/Funds", 99_999_999) } else { ("/Gifts/Gift 01", 99) };
            t.set(path_, (hi - 1).into()).unwrap();
            let out = t.to_bytes().unwrap();
            assert_eq!(out.len(), raw.len());
            assert!(looks(&out));
        }
    }
}
