//! Kingdom Hearts II (PlayStation 2, USA and Europe releases) saves: the game's own 46,304-byte
//! save file inside a `.psu`, `.psv` or memory card folder. Offsets, ids and names follow the
//! MIT-licensed save editor in RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).
//! The Japanese and Final Mix saves lay their data out differently and are not read yet.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::opts;
use crate::tree::TreeDoc;

pub const KIND: &str = "Kingdom Hearts II save";
const SIZE: usize = 46_304;
const CHARACTERS: [&str; 13] = ["Sora", "Donald", "Goofy", "Mickey", "Auron", "Mulan", "Aladdin", "Jack Sparrow", "Beast", "Jack Skellington", "Simba", "Tron", "Riku"];

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && &d[..3] == b"KH2" && (d[3] == b'U' || d[3] == b'E')
}

fn magic(base: &str) -> Vec<(String, u64)> {
    let a = ["", "a", "ga"];
    let base_names: &[&str] = match base {
        "Fire" => &["Fire", "Fira", "Firaga"],
        "Blizzard" => &["Blizzard", "Blizzara", "Blizzaga"],
        "Thunder" => &["Thunder", "Thundara", "Thundaga"],
        "Cure" => &["Cure", "Cura", "Curaga"],
        "Magnet" => &["Magnet", "Magnera", "Magnega"],
        _ => &["Reflect", "Reflera", "Reflega"],
    };
    let _ = a;
    let mut v = opts(&[("-", 0)]);
    v.extend(base_names.iter().enumerate().map(|(i, n)| (n.to_string(), i as u64 + 1)));
    v
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Kingdom Hearts II save".into());
    }
    let mut b = Builder::new(d);
    b.pick(&["General", "Difficulty"], 0x1658, Kind::U8, &opts(&[("Beginner Mode", 0), ("Standard Mode", 1), ("Proud Mode", 2)]));
    b.ranged(&["General", "Munny"], 0x1600, Kind::U32, 0, 999_999);
    for (n, at) in [("Fire", 0x249c), ("Blizzard", 0x249d), ("Thunder", 0x249e), ("Cure", 0x249f), ("Magnet", 0x24d7), ("Reflect", 0x24d8)] {
        b.pick(&["Magic", n], at, Kind::U8, &magic(n));
    }
    for (i, name) in CHARACTERS.iter().enumerate() {
        let s = 0xf4 * i;
        b.ranged(&["Party", name, "Level"], s + 0x166f, Kind::U8, 1, 99);
        b.ranged(&["Party", name, "Experience"], s + 0x25e8, Kind::U32, 0, 9_999_999);
        b.ranged(&["Party", name, "HP"], s + 0x1664, Kind::U8, 1, 255);
        b.ranged(&["Party", name, "MP"], s + 0x1666, Kind::U8, 0, 255);
        b.ranged(&["Party", name, "Bonus Strength"], s + 0x1669, Kind::U8, 0, 100);
        b.ranged(&["Party", name, "Bonus Magic"], s + 0x166a, Kind::U8, 0, 100);
        b.ranged(&["Party", name, "Bonus Defense"], s + 0x166b, Kind::U8, 0, 100);
        b.ranged(&["Party", name, "Bonus AP"], s + 0x1668, Kind::U8, 0, 100);
    }
    let mut seen = std::collections::HashSet::new();
    for (cat, items) in [("Items", ITEMS), ("Weapons", WEAPONS), ("Armor", ARMOR), ("Accessories", ACCESSORIES), ("Key items", KEY_ITEMS), ("Synthesis materials", MATERIALS)] {
        for (i, name) in items {
            let mut key = name.to_string();
            if !seen.insert(key.clone()) {
                key = format!("{name} ({cat})");
                seen.insert(key.clone());
            }
            b.ranged(&["Stock", cat, &key], 0x2488 + *i as usize, Kind::U8, 0, 99);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. The save's checksum is written again. Only the USA and Europe releases are read; the stock holds at most 99 of each item."))
}

fn fix(d: &mut Vec<u8>) {
    let mut c = 0xffff_ffffu32;
    for (i, &x) in d[..0xb4e0].iter().enumerate() {
        if !(8..12).contains(&i) {
            c = (c << 8) ^ CRC[((x as u32 ^ (c >> 24)) & 0xff) as usize];
        }
    }
    d[8..12].copy_from_slice(&(!c).to_le_bytes());
}

const CRC: [u32; 256] = [
    0x00000000, 0x8b17e770, 0x12eed357, 0x99f93427, 0x25dda6ae, 0xaeca41de,
    0x373375f9, 0xbc249289, 0x4bbb4d5c, 0xc0acaa2c, 0x59559e0b, 0xd242797b,
    0x6e66ebf2, 0xe5710c82, 0x7c8838a5, 0xf79fdfd5, 0x97769ab8, 0x1c617dc8,
    0x859849ef, 0x0e8fae9f, 0xb2ab3c16, 0x39bcdb66, 0xa045ef41, 0x2b520831,
    0xdccdd7e4, 0x57da3094, 0xce2304b3, 0x4534e3c3, 0xf910714a, 0x7207963a,
    0xebfea21d, 0x60e9456d, 0x2a2c28c7, 0xa13bcfb7, 0x38c2fb90, 0xb3d51ce0,
    0x0ff18e69, 0x84e66919, 0x1d1f5d3e, 0x9608ba4e, 0x6197659b, 0xea8082eb,
    0x7379b6cc, 0xf86e51bc, 0x444ac335, 0xcf5d2445, 0x56a41062, 0xddb3f712,
    0xbd5ab27f, 0x364d550f, 0xafb46128, 0x24a38658, 0x988714d1, 0x1390f3a1,
    0x8a69c786, 0x017e20f6, 0xf6e1ff23, 0x7df61853, 0xe40f2c74, 0x6f18cb04,
    0xd33c598d, 0x582bbefd, 0xc1d28ada, 0x4ac56daa, 0x5458518e, 0xdf4fb6fe,
    0x46b682d9, 0xcda165a9, 0x7185f720, 0xfa921050, 0x636b2477, 0xe87cc307,
    0x1fe31cd2, 0x94f4fba2, 0x0d0dcf85, 0x861a28f5, 0x3a3eba7c, 0xb1295d0c,
    0x28d0692b, 0xa3c78e5b, 0xc32ecb36, 0x48392c46, 0xd1c01861, 0x5ad7ff11,
    0xe6f36d98, 0x6de48ae8, 0xf41dbecf, 0x7f0a59bf, 0x8895866a, 0x0382611a,
    0x9a7b553d, 0x116cb24d, 0xad4820c4, 0x265fc7b4, 0xbfa6f393, 0x34b114e3,
    0x7e747949, 0xf5639e39, 0x6c9aaa1e, 0xe78d4d6e, 0x5ba9dfe7, 0xd0be3897,
    0x49470cb0, 0xc250ebc0, 0x35cf3415, 0xbed8d365, 0x2721e742, 0xac360032,
    0x101292bb, 0x9b0575cb, 0x02fc41ec, 0x89eba69c, 0xe902e3f1, 0x62150481,
    0xfbec30a6, 0x70fbd7d6, 0xccdf455f, 0x47c8a22f, 0xde319608, 0x55267178,
    0xa2b9aead, 0x29ae49dd, 0xb0577dfa, 0x3b409a8a, 0x87640803, 0x0c73ef73,
    0x958adb54, 0x1e9d3c24, 0xa8b0a31c, 0x23a7446c, 0xba5e704b, 0x3149973b,
    0x8d6d05b2, 0x067ae2c2, 0x9f83d6e5, 0x14943195, 0xe30bee40, 0x681c0930,
    0xf1e53d17, 0x7af2da67, 0xc6d648ee, 0x4dc1af9e, 0xd4389bb9, 0x5f2f7cc9,
    0x3fc639a4, 0xb4d1ded4, 0x2d28eaf3, 0xa63f0d83, 0x1a1b9f0a, 0x910c787a,
    0x08f54c5d, 0x83e2ab2d, 0x747d74f8, 0xff6a9388, 0x6693a7af, 0xed8440df,
    0x51a0d256, 0xdab73526, 0x434e0101, 0xc859e671, 0x829c8bdb, 0x098b6cab,
    0x9072588c, 0x1b65bffc, 0xa7412d75, 0x2c56ca05, 0xb5affe22, 0x3eb81952,
    0xc927c687, 0x423021f7, 0xdbc915d0, 0x50def2a0, 0xecfa6029, 0x67ed8759,
    0xfe14b37e, 0x7503540e, 0x15ea1163, 0x9efdf613, 0x0704c234, 0x8c132544,
    0x3037b7cd, 0xbb2050bd, 0x22d9649a, 0xa9ce83ea, 0x5e515c3f, 0xd546bb4f,
    0x4cbf8f68, 0xc7a86818, 0x7b8cfa91, 0xf09b1de1, 0x696229c6, 0xe275ceb6,
    0xfce8f292, 0x77ff15e2, 0xee0621c5, 0x6511c6b5, 0xd935543c, 0x5222b34c,
    0xcbdb876b, 0x40cc601b, 0xb753bfce, 0x3c4458be, 0xa5bd6c99, 0x2eaa8be9,
    0x928e1960, 0x1999fe10, 0x8060ca37, 0x0b772d47, 0x6b9e682a, 0xe0898f5a,
    0x7970bb7d, 0xf2675c0d, 0x4e43ce84, 0xc55429f4, 0x5cad1dd3, 0xd7bafaa3,
    0x20252576, 0xab32c206, 0x32cbf621, 0xb9dc1151, 0x05f883d8, 0x8eef64a8,
    0x1716508f, 0x9c01b7ff, 0xd6c4da55, 0x5dd33d25, 0xc42a0902, 0x4f3dee72,
    0xf3197cfb, 0x780e9b8b, 0xe1f7afac, 0x6ae048dc, 0x9d7f9709, 0x16687079,
    0x8f91445e, 0x0486a32e, 0xb8a231a7, 0x33b5d6d7, 0xaa4ce2f0, 0x215b0580,
    0x41b240ed, 0xcaa5a79d, 0x535c93ba, 0xd84b74ca, 0x646fe643, 0xef780133,
    0x76813514, 0xfd96d264, 0x0a090db1, 0x811eeac1, 0x18e7dee6, 0x93f03996,
    0x2fd4ab1f, 0xa4c34c6f, 0x3d3a7848, 0xb62d9f38,
];
const ITEMS: &[(u16, &str)] = &[(0, "Potion"), (1, "Hi-Potion"), (2, "Ether"), (3, "Elixir"), (4, "Mega-Potion"), (5, "Mega-Ether"), (6, "Megalixir"), (97, "Tent"), (228, "Drive Recovery"), (229, "High Drive Recovery"), (230, "Power Boost"), (231, "Magic Boost"), (232, "Defense Boost"), (233, "AP Boost"), ];
const WEAPONS: &[(u16, &str)] = &[(33, "Kingdom Key"), (34, "Oathkeeper"), (35, "Oblivion"), (36, "Detection Saber"), (37, "Edge of Ultima"), (41, "Knight's Shield"), (42, "Detection Shield"), (43, "Test the King"), (46, "Battlefields of War"), (47, "Sword of the Ancestor"), (51, "Beast's Claw"), (52, "Bone Fist"), (53, "Proud Fang"), (54, "Skill and Crossbones"), (63, "Antiform Dummy"), (64, "Scimitar"), (65, "Way to the Dawn"), (66, "Identity Disk"), (67, "Mage's Staff"), (73, "FAKE"), (102, "Adamant Shield"), (103, "Chain Gear"), (104, "Ogre Shield"), (105, "Falling Star"), (106, "Dreamcloud"), (107, "Knight Defender"), (108, "Genji Shield"), (109, "Akashic Record"), (110, "Nobody Guard"), (111, "Hammer Staff"), (112, "Victory Bell"), (113, "Meteor Staff"), (114, "Comet Staff"), (115, "Lord's Broom"), (116, "Wisdom Wand"), (117, "Rising Dragon"), (118, "Nobody Lance"), (119, "Shaman's Relic"), (122, "Staff of Detection"), (251, "Star Seeker"), (252, "Hidden Dragon"), (253, "Save the Queen"), (254, "Save the King"), (255, "Hero's Crest"), (256, "Monochrome"), (257, "Follow the Wind"), (258, "Circle of Life"), (259, "Photon Debugger"), (260, "Gull Wing"), (261, "Rumbling Rose"), (262, "Guardian Soul"), (263, "Wishing Lamp"), (264, "Decisive Pumpkin"), (265, "Sleeping Lion"), (266, "Sweet Memories"), (267, "Mysterious Abyss"), (268, "Fatal Crest"), (269, "Bond of Flame"), (270, "Fenrir"), (271, "Ultima Weapon"), (274, "Save the Queen+"), (275, "Save the King+"), ];
const ARMOR: &[(u16, &str)] = &[(60, "Elven Bandanna"), (61, "Divine Bandanna"), (62, "Power Band"), (70, "Buster Band"), (71, "Protect Belt"), (74, "Gaia Belt"), (81, "Cosmic Belt"), (87, "Fire Bangle"), (88, "Fira Bangle"), (89, "Firaga Bangle"), (90, "Firagun Bangle"), (92, "Blizzard Armlet"), (93, "Blizzara Armlet"), (94, "Blizzaga Armlet"), (95, "Blizzagun Armlet"), (98, "Thunder Trinket"), (99, "Thundara Trinket"), (100, "Thundaga Trinket"), (101, "Thundagun Trinket"), (121, "Shadow Anklet"), (123, "Dark Anklet"), (124, "Midnight Anklet"), (125, "Chaos Anklet"), (127, "Abas Chain"), (128, "Aegis Chain"), (129, "Acrisius"), (130, "Ribbon"), (131, "Champion Belt"), (132, "Petit Ribbon"), (133, "Acrisius+"), (134, "Cosmic Chain"), ];
const ACCESSORIES: &[(u16, &str)] = &[(7, "Ability Ring"), (8, "Engineer's Ring"), (9, "Technician's Ring"), (10, "Expert's Ring"), (11, "Sardonyx Ring"), (12, "Tourmaline Ring"), (13, "Aquamarine Ring"), (14, "Garnet Ring"), (15, "Diamond Ring"), (16, "Silver Ring"), (17, "Gold Ring"), (18, "Platinum Ring"), (19, "Mythril Ring"), (26, "Orichalcum Ring"), (27, "Master's Ring"), (28, "Moon Amulet"), (30, "Star Charm"), (31, "Skill Ring"), (32, "Skillful Ring"), (38, "Soldier Earring"), (39, "Fencer Earring"), (40, "Mage Earring"), (44, "Slayer Earring"), (45, "Cosmic Ring"), (48, "Medal"), (49, "Cosmic Arts"), (56, "Lucky Ring"), (58, "Draw Ring"), ];
const KEY_ITEMS: &[(u16, &str)] = &[(24, "Torn Pages"), (188, "Munny Pouch"), (189, "Crystal Orb"), (190, "Seifer's Trophy"), (191, "Tournament Poster"), (192, "Poster"), (193, "Letter"), (194, "Naminé's Sketches"), (195, "Membership Card"), (196, "Olympus Stone"), (197, "Auron's Statue"), (198, "Cursed Medallion"), (199, "Present"), (200, "Decoy Presents"), (201, "Ice Cream"), (202, "Picture"), (208, "Mega-Recipe"), (210, "Star Recipe"), (211, "Recovery Recipe"), (212, "Skill Recipe"), (213, "Guard Recipe"), (246, "Style Recipe"), (247, "Moon Recipe"), (248, "Queen Recipe"), (249, "King Recipe"), (250, "Ultimate Recipe"), (276, "Promise Charm"), (277, "Munny Pouch"), (278, "Hades Cup Trophy"), ];
const MATERIALS: &[(u16, &str)] = &[(234, "Dark Shard"), (235, "Dark Stone"), (236, "Dark Gem"), (237, "Dark Crystal"), (143, "Blazing Shard"), (144, "Blazing Stone"), (145, "Blazing Gem"), (146, "Blazing Crystal"), (151, "Lightning Shard"), (152, "Lightning Stone"), (153, "Lightning Gem"), (154, "Lightning Crystal"), (155, "Power Shard"), (156, "Power Stone"), (157, "Power Gem"), (158, "Power Crystal"), (159, "Lucid Shard"), (160, "Lucid Stone"), (161, "Lucid Gem"), (162, "Lucid Crystal"), (163, "Dense Shard"), (164, "Dense Stone"), (165, "Dense Gem"), (166, "Dense Crystal"), (167, "Twilight Shard"), (168, "Twilight Stone"), (169, "Twilight Gem"), (170, "Twilight Crystal"), (171, "Mythril Shard"), (172, "Mythril Stone"), (173, "Mythril Gem"), (174, "Mythril Crystal"), (175, "Bright Shard"), (176, "Bright Stone"), (177, "Bright Gem"), (178, "Bright Crystal"), (179, "Energy Shard"), (180, "Energy Stone"), (181, "Energy Gem"), (182, "Energy Crystal"), (183, "Serenity Shard"), (184, "Serenity Stone"), (185, "Serenity Gem"), (186, "Serenity Crystal"), (187, "Orichalcum+"), (203, "Orichalcum"), (204, "Frost Shard"), (205, "Frost Stone"), (206, "Frost Gem"), (207, "Frost Crystal"), ];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_fixes_checksum() {
        let mut d = vec![0u8; SIZE];
        d[..4].copy_from_slice(b"KH2U");
        fix(&mut d);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/General/Munny", 5000.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes(out[0x1600..0x1604].try_into().unwrap()), 5000);
        let mut again = out.clone();
        fix(&mut again);
        assert_eq!(again, out);
    }
}
