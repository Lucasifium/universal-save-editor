//! Dragon Quest VIII: Journey of the Cursed King (PlayStation 2) saves: the 23,776-byte save file
//! of the save folder. A 32-bit checksum at offset 8 sums one byte out of every 64 (each taken
//! modulo 255) of 0x40..0x2e80. Offsets and item names follow the MIT-licensed save editor in
//! RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Dragon Quest VIII save";

fn sum(d: &[u8]) -> u32 {
    (0x40..0x2e80).step_by(0x40).fold(0u32, |s, i| s.wrapping_add((d[i] % 0xff) as u32))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() == 0x5ce0 {
        let s = sum(d);
        d[8..12].copy_from_slice(&s.to_le_bytes());
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == 0x5ce0 && d[..4] == [0x10, 0, 0, 0] && u32::from_le_bytes([d[8], d[9], d[10], d[11]]) == sum(d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Dragon Quest VIII save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["General", "Save count"], 0x18, Kind::U32, 0, 999_999);
    b.ranged(&["General", "Gold coins"], 0xa40, Kind::U32, 0, 99_999_999);
    b.ranged(&["Miscellaneous", "Bank vault"], 0x1494, Kind::U32, 0, 99_999_999);
    b.ranged(&["Miscellaneous", "Casino tokens"], 0x5c50, Kind::U32, 0, 99_999_999);
    b.ranged(&["Miscellaneous", "Mini medals given to Minnie"], 0x1ac, Kind::U16, 0, 115);
    for (n, at, bit) in WARPS {
        flag(&mut b, &["Warp places", n], *at, *bit);
    }
    let stats: [(&str, usize, Kind, i64, i64); 4] = [("HP", 0xa74, Kind::U16, 1, 999), ("MP", 0xa7c, Kind::U16, 0, 999), ("Experience", 0xa88, Kind::U32, 0, 16_777_215), ("Strength", 0xaa0, Kind::U16, 0, 999)];
    let more: [(&str, usize); 3] = [("Agility", 0xaa2), ("Resilience", 0xaa4), ("Wisdom", 0xaa6)];
    for (c, who) in ["Hero", "Yangus", "Jessica", "Angelo"].iter().enumerate() {
        let o = c * 0x68;
        b.bits(&["Party", who, "Level"], 0xa80 + o, Kind::U16, 0, 16, 1, 1, 99);
        for (n, at, k, lo, hi) in stats {
            b.ranged(&["Party", who, n], at + o, k, lo, hi);
        }
        for (n, at) in more {
            b.ranged(&["Party", who, n], at + o, Kind::U16, 0, 999);
        }
        for a in 0..5 {
            b.ranged(&["Party", who, &format!("Attribute {}", a + 1)], 0xaaa + a * 2 + o, Kind::U16, 0, 100);
        }
    }
    let items: Vec<(String, u64)> = std::iter::once(("-".to_string(), 0u64)).chain(ITEMS.iter().map(|(i, n)| (n.to_string(), *i as u64))).collect();
    for i in 0..396 {
        let slot = format!("Slot {:03}", i + 1);
        b.pick(&["Bag", &slot, "Item"], 0xcd0 + i * 4, Kind::U16, &items);
        b.ranged(&["Bag", &slot, "Quantity"], 0xcd2 + i * 4, Kind::U16, 0, 999);
    }
    let rec: [(&str, usize); 7] = [("Battles", 0x1740), ("Victories", 0x174c), ("Times fled", 0x1754), ("Times wiped out", 0x1758), ("Monsters defeated", 0x1744), ("Monsters intimidated", 0x57b4), ("Total gold obtained", 0x175c)];
    for (n, at) in rec {
        b.ranged(&["Battle records", n], at, Kind::U32, 0, 99_999_999);
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. The save's checksum is written again when you save.")))
}

const WARPS: &[(&str, usize, u32)] = &[("Farebury", 0x290, 0), ("Alexandria", 0x290, 1), ("Port Prospect", 0x290, 2), ("Peregrin Quay", 0x290, 3), ("Maella Abbey", 0x290, 4), ("Simpleton", 0x290, 5), ("Ruined Abbey", 0x290, 6), ("Riverside Chapel", 0x290, 7), ("Ascantha", 0x291, 0), ("Monster Arena", 0x291, 1), ("Pickham", 0x291, 2), ("Swordsman's Labyrinth", 0x291, 3), ("Hilltop Hut", 0x291, 4), ("Trodain", 0x291, 5), ("Princess Minnie's Castle", 0x291, 6), ("Neos", 0x291, 7), ("Savella Cathedral", 0x292, 0), ("Seaview Church", 0x292, 1), ("Baccarat", 0x292, 2), ("Chateau Felix", 0x292, 3), ("Mystical Spring", 0x292, 4), ("Argonia", 0x292, 5), ("Royal Hunting Ground", 0x292, 6), ("Dark Ruins", 0x292, 7), ("Desert Chapel", 0x293, 0), ("Arcadia", 0x293, 1), ("Rydon's Tower", 0x293, 2), ("Marta's Cottage", 0x293, 3), ("Orkutsk", 0x293, 4), ("Herb Grotto", 0x293, 5), ("Pirate's Cove", 0x293, 6), ("Empycchu", 0x293, 7), ("Tryan Gully", 0x294, 0), ("Dragovian Sanctuary", 0x294, 1)];
const ITEMS: &[(u16, &str)] = &[(0x1, "Cypress stick"), (0x2, "Soldier's sword"), (0x3, "Copper sword"), (0x4, "Steel broadsword"), (0x5, "Platinum sword"), (0x6, "Dream blade"), (0x7, "Falcon blade"), (0x8, "Über falcon blade"), (0x9, "Zombiesbane"), (0xa, "Zombie slayer"), (0xb, "Dragonsbane"), (0xc, "Dragon slayer"), (0xd, "Blizzard blade"), (0xe, "Miracle sword"), (0xf, "Über miracle sword"), (0x10, "Bastard sword"), (0x11, "Liquid metal sword"), (0x12, "Double-edged sword"), (0x13, "Über double-edge"), (0x14, "Rusty old sword"), (0x15, "Dragovian sword"), (0x16, "Dragovian king sword"), (0x17, "Rapier"), (0x18, "Templar's sword"), (0x19, "Hell sabre"), (0x1a, "Holy silver rapier"), (0x1b, "Fallen angel rapier"), (0x1c, "Shamshir of light"), (0x1d, "Mercury's rapier"), (0x1e, "Bronze knife"), (0x1f, "Dagger"), (0x20, "Poison moth knife"), (0x21, "Assassin's dagger"), (0x22, "Icicle dirk"), (0x23, "Poison needle"), (0x24, "Falcon knife"), (0x25, "Imp knife"), (0x26, "Sword breaker"), (0x27, "Eagle dagger"), (0x28, "Iron lance"), (0x29, "Holy lance"), (0x2a, "Long spear"), (0x2b, "Partisan"), (0x2c, "Battle fork"), (0x2d, "Sandstorm spear"), (0x2e, "Demon spear"), (0x2f, "Hero spear"), (0x30, "Metal king spear"), (0x31, "Boomerang"), (0x32, "Reinforced boomerang"), (0x33, "Edged boomerang"), (0x34, "Metal wing boomerang"), (0x35, "Razor wing boomerang"), (0x36, "Swallowtail"), (0x37, "Flametang boomerang"), (0x38, "Stone axe"), (0x39, "Iron axe"), (0x3a, "King axe"), (0x3b, "Moon axe"), (0x3c, "Bandit axe"), (0x3d, "Golden axe"), (0x3e, "Battle-axe"), (0x3f, "Conqueror's axe"), (0x40, "Oaken club"), (0x41, "Giant mallet"), (0x42, "Sledgehammer"), (0x43, "War hammer"), (0x44, "Über war hammer"), (0x45, "DUMMY HAMMER"), (0x46, "Megaton hammer"), (0x47, "Flail of fury"), (0x48, "Flail of destruction"), (0x49, "Thorn whip"), (0x4a, "Leather whip"), (0x4b, "Snakeskin whip"), (0x4c, "Chain whip"), (0x4d, "Dragontail whip"), (0x4e, "Spiked steel whip"), (0x4f, "Demon whip"), (0x50, "Scourge whip"), (0x51, "Gringham whip"), (0x52, "Farmer's scythe"), (0x53, "DUMMY SICKLE"), (0x54, "Heavy hatchet"), (0x55, "Bardiche of binding"), (0x56, "Hell scythe"), (0x57, "Steel scythe"), (0x58, "Wizard's staff"), (0x59, "Lightning staff"), (0x5a, "Staff of divine wrath"), (0x5b, "Magma staff"), (0x5c, "DUMMY WAND"), (0x5d, "Staff of antimagic"), (0x5e, "Rune staff"), (0x5f, "Staff of resurrection"), (0x60, "Magical mace"), (0x61, "DUMMY STAFF"), (0x62, "Hunter's bow"), (0x63, "Great bow"), (0x64, "Short bow"), (0x65, "Odin's bow"), (0x66, "Cheiron's bow"), (0x67, "Eros' bow"), (0x68, "Plain clothes"), (0x69, "Bandit's grass skirt"), (0x6a, "Wayfarer's clothes"), (0x6b, "Boxer shorts"), (0x6c, "Dancer's costume"), (0x6d, "Leather kilt"), (0x6e, "Leather dress"), (0x6f, "Templar's uniform"), (0x70, "Fur poncho"), (0x71, "Cloak of evasion"), (0x72, "Bunny suit"), (0x73, "Silk bustier"), (0x74, "Magic bikini"), (0x75, "Spangled dress"), (0x76, "Posh waistcoat"), (0x77, "Magical skirt"), (0x78, "Shimmering dress"), (0x79, "Dark robe"), (0x7a, "DUMMY DRESS"), (0x7b, "Dangerous bustier"), (0x7c, "Divine bustier"), (0x7d, "Jessica's outfit"), (0x7e, "Leather armour"), (0x7f, "Scale armour"), (0x80, "Chain mail"), (0x81, "Bronze armour"), (0x82, "Iron cuirass"), (0x83, "Iron armour"), (0x84, "Tortoise shell"), (0x85, "Full plate armour"), (0x86, "Silver cuirass"), (0x87, "Dancer's mail"), (0x88, "Silver mail"), (0x89, "Dragon mail"), (0x8a, "Magic armour"), (0x8b, "Zombie mail"), (0x8c, "Heavy armour"), (0x8d, "Spiked armour"), (0x8e, "Platinum mail"), (0x8f, "Bandit mail"), (0x90, "Sacred armour"), (0x91, "DUMMY ARMOUR"), (0x92, "Gigant armour"), (0x93, "Mirror armour"), (0x94, "Liquid metal armour"), (0x95, "Dragovian armour"), (0x96, "Metal king armour"), (0x97, "Silk robe"), (0x98, "Leather cape"), (0x99, "Robe of serenity"), (0x9a, "Magic vestment"), (0x9b, "Flowing dress"), (0x9c, "Sage's robe"), (0x9d, "Angel's robe"), (0x9e, "Velvet cape"), (0x9f, "Crimson robe"), (0xa0, "Princess's robe"), (0xa1, "Dragon robe"), (0xa2, "Pot lid"), (0xa3, "Leather shield"), (0xa4, "Scale shield"), (0xa5, "Silver platter"), (0xa6, "Kitty shield"), (0xa7, "Bronze shield"), (0xa8, "Templar's shield"), (0xa9, "Iron shield"), (0xaa, "Light shield"), (0xab, "White shield"), (0xac, "Steel shield"), (0xad, "Magic shield"), (0xae, "Dragon shield"), (0xaf, "DUMMY SHIELD"), (0xb0, "Power shield"), (0xb1, "Saintess shield"), (0xb2, "Bone shield"), (0xb3, "Mirror shield"), (0xb4, "Ogre shield"), (0xb5, "Silver shield"), (0xb6, "Big boss shield"), (0xb7, "Ruinous shield"), (0xb8, "Thanatos' shield"), (0xb9, "Goddess shield"), (0xba, "Dragovian shield"), (0xbb, "Metal king shield"), (0xbc, "Bandana"), (0xbd, "Leather hat"), (0xbe, "Hairband"), (0xbf, "Pointy hat"), (0xc0, "Feathered cap"), (0xc1, "Turban"), (0xc2, "Silver tiara"), (0xc3, "Bunny ears"), (0xc4, "Stone hardhat"), (0xc5, "Iron helmet"), (0xc6, "Fur hood"), (0xc7, "Bronze helmet"), (0xc8, "Coral hairpin"), (0xc9, "Pirate's hat"), (0xca, "Iron mask"), (0xcb, "Hermes' hat"), (0xcc, "Platinum headgear"), (0xcd, "Magical hat"), (0xce, "Happy hat"), (0xcf, "Hades' helm"), (0xd0, "Slime crown"), (0xd1, "Mythril helm"), (0xd2, "Mercury's bandana"), (0xd3, "Scholar's cap"), (0xd4, "DUMMY CAP"), (0xd5, "Thinking cap"), (0xd6, "Iron headgear"), (0xd7, "Raging bull helm"), (0xd8, "Golden tiara"), (0xd9, "Great helm"), (0xda, "Phantom mask"), (0xdb, "Skull helm"), (0xdc, "Dragovian helm"), (0xdd, "Sun crown"), (0xde, "Metal king helm"), (0xdf, "DUMMY FLEE RING"), (0xe0, "Agility ring"), (0xe1, "Meteorite bracer"), (0xe2, "Strength ring"), (0xe3, "Gold ring"), (0xe4, "Prayer ring"), (0xe5, "Recovery ring"), (0xe6, "Goddess ring"), (0xe7, "Gospel ring"), (0xe8, "Life bracer"), (0xe9, "Mighty armlet"), (0xea, "Gold bracer"), (0xeb, "DUMMY BRACELET"), (0xec, "DUMMY BRACER"), (0xed, "Ruby of protection"), (0xee, "Titan belt"), (0xef, "Scholar's specs"), (0xf0, "Tough guy tattoo"), (0xf1, "Slime earrings"), (0xf2, "Devil's tail"), (0xf3, "Fishnet stockings"), (0xf4, "Garter"), (0xf5, "Lady's ring"), (0xf6, "Gold rosary"), (0xf7, "DUMMY BOOTS"), (0xf8, "Sorcerer's ring"), (0xf9, "Skull ring"), (0xfa, "Holy talisman"), (0xfb, "Ring of awakening"), (0xfc, "Full moon ring"), (0xfd, "Ring of clarity"), (0xfe, "Ring of truth"), (0xff, "Ring of immunity"), (0x100, "Catholicon ring"), (0x101, "DUMMY SANDGLASS"), (0x102, "DUMMY HOURGLASS"), (0x103, "Dragon scale"), (0x104, "Elevating shoes"), (0x105, "Medicinal herb"), (0x106, "Antidotal herb"), (0x107, "Moonwort bulb"), (0x108, "Strong medicine"), (0x109, "Special medicine"), (0x10a, "Lesser panacea"), (0x10b, "Greater panacea"), (0x10c, "Rose-root"), (0x10d, "Rose-wort"), (0x10e, "Strong antidote"), (0x10f, "Special antidote"), (0x110, "Moon's mercy"), (0x111, "Mystifying mixture"), (0x112, "Yggdrasil leaf"), (0x113, "Yggdrasil dew"), (0x114, "Amor seco essence"), (0x115, "Chimaera wing"), (0x116, "Holy water"), (0x117, "Magic water"), (0x118, "Elfin elixir"), (0x119, "Rockbomb shard"), (0x11a, "Sage's stone"), (0x11b, "DUMMY SAND OF TIME"), (0x11c, "Seed of strength"), (0x11d, "Seed of agility"), (0x11e, "Seed of defence"), (0x11f, "Seed of wisdom"), (0x120, "Seed of skill"), (0x121, "Seed of life"), (0x122, "Seed of magic"), (0x123, "Mini medal"), (0x124, "DUMMY BESTIARY"), (0x125, "Thief's key"), (0x126, "Magic key"), (0x127, "Ultimate key"), (0x128, "Orichalcum"), (0x129, "Bunny tail"), (0x12a, "Timbrel of tension"), (0x12b, "DUMMY CYMBAL"), (0x12c, "DUMMY CONCH"), (0x12d, "Baumren's bell"), (0x12e, "Gold nugget"), (0x12f, "Iron nail"), (0x130, "Cowpat"), (0x131, "Dragon dung"), (0x132, "Rock salt"), (0x133, "DUMMY LIZARD DUNG"), (0x134, "Saint's ashes"), (0x135, "Wing of bat"), (0x136, "Magic beast hide"), (0x137, "Fresh milk"), (0x138, "Rennet powder"), (0x139, "Red mould"), (0x13a, "Waterweed mould"), (0x13b, "Premium mould"), (0x13c, "Plain cheese"), (0x13d, "DUMMY CHEESE #1"), (0x13e, "Spicy cheese"), (0x13f, "Super spicy cheese"), (0x140, "Scorching cheese"), (0x141, "DUMMY CHEESE #2"), (0x142, "Cool cheese"), (0x143, "Chilly cheese"), (0x144, "Cold cheese"), (0x145, "C-c-cold cheese"), (0x146, "DUMMY CHEESE #3"), (0x147, "DUMMY CHEESE #4"), (0x148, "DUMMY CHEESE #5"), (0x149, "DUMMY CHEESE #6"), (0x14a, "DUMMY CHEESE #7"), (0x14b, "Mild cheese"), (0x14c, "Cured cheese"), (0x14d, "Angel cheese"), (0x14e, "Hard cheese"), (0x14f, "Soft cheese"), (0x150, "Chunky cheese"), (0x151, "Highly-strung cheese"), (0x152, "DUMMY CHEESE #8"), (0x153, "Crystal ball"), (0x154, "Tool bag"), (0x155, "Jessica's letter"), (0x156, "Templar's ring"), (0x157, "World map"), (0x158, "Venus' tear"), (0x159, "Moonshadow harp"), (0x15a, "Sand of serenity"), (0x15b, "Lizard humour"), (0x15c, "DUMMY BAG"), (0x15d, "Argon heart"), (0x15e, "Great big Argon heart"), (0x15f, "Magic mirror"), (0x160, "Sun mirror"), (0x161, "Stone sword"), (0x162, "Kran Spinels"), (0x163, "'The Big Book of Barriers'"), (0x164, "Marta's bag"), (0x165, "Illuminated sea chart"), (0x166, "Nook grass"), (0x167, "Godbird's soulstone"), (0x168, "Darktree leaf"), (0x169, "Templar Captain's ring"), (0x16a, "DUMMY RING"), (0x16b, "Argon ring"), (0x16c, "Echo flute"), (0x16d, "Eros' bow recipe"), (0x16e, "Imp knife recipe"), (0x16f, "Dragon slayer recipe"), (0x170, "Thief's key recipe"), (0x171, "Morrie's memo #1"), (0x172, "Morrie's memo #2"), (0x173, "Morrie's memo #3"), (0x174, "Monster Arena key"), (0x175, "Godbird Sceptre"), (0x176, "DUMMY CARD"), (0x177, "Gold orb"), (0x178, "Silver orb"), (0x179, "Red orb"), (0x17a, "Blue orb"), (0x17b, "Green orb"), (0x17c, "Yellow orb"), (0x17d, "Purple orb"), (0x17e, "Copper monster coin"), (0x17f, "Silver monster coin"), (0x180, "Gold monster coin"), (0x181, "Flame shield"), (0x182, "Ice shield"), (0x183, "MIMIC STONE"), (0x184, "BLARNEY STONE")];

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("DQ8_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            assert!(super::looks(&d));
            let mut doc = super::parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d);
        }
    }
}
