//! Plants vs. Zombies (2009, Game of the Year Edition): `userN.dat`.
//!
//! A flat table of little-endian 32-bit values, no compression or checksum: the version (10,
//! 11 or 12) at 0, the Adventure level at 4, money divided by 10 at 8, adventure completions at
//! 12, Survival progress at 0x10–0x37, mini-game trophies at 0x4C–0xD0, puzzle trophies at
//! 0xD8–0x12C, shop purchases at 0x1A0–0x214 (one flag each; fertilizer, bug spray, chocolate
//! and tree food are stored as count + 1000), mode unlocks at 0x300/0x304, then the Zen Garden
//! (plant count at 0x330, 88-byte records from 0x334). Layout from the Plants vs. Zombies
//! wiki's "User file format" page (facts), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Plants vs. Zombies profile";
const PLANTS: usize = 0x330;
const RECORDS: usize = 0x334;
const RECORD: usize = 0x58;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

pub fn looks(d: &[u8]) -> bool {
    let (Some(v), Some(n)) = (u32_at(d, 0), u32_at(d, PLANTS)) else { return false };
    (10..=12).contains(&v) && n <= 200 && d.len() >= RECORDS + n as usize * RECORD && d.len() < RECORDS + n as usize * RECORD + 4096 && u32_at(d, 4).is_some_and(|l| l < 1000)
}

const SURVIVAL: [(&str, usize, i64); 10] = [
    ("Day", 0x10, 5),
    ("Night", 0x14, 5),
    ("Pool", 0x18, 5),
    ("Fog", 0x1C, 5),
    ("Roof", 0x20, 5),
    ("Day (Hard)", 0x24, 10),
    ("Night (Hard)", 0x28, 10),
    ("Pool (Hard)", 0x2C, 10),
    ("Fog (Hard)", 0x30, 10),
    ("Roof (Hard)", 0x34, 10),
];
const ENDLESS: [(&str, usize); 5] = [("Day", 0x38), ("Night", 0x3C), ("Pool", 0x40), ("Fog", 0x44), ("Roof", 0x48)];
const MINIGAMES: [&str; 34] = [
    "ZomBotany", "Wall-nut Bowling", "Slot Machine", "It's Raining Seeds", "Beghouled", "Invisi-ghoul", "Seeing Stars", "Zombiquarium", "Beghouled Twist",
    "Big Trouble Little Zombie", "Portal Combat", "Column Like You See 'Em", "Bobsled Bonanza", "Zombie Nimble Zombie Quick", "Whack a Zombie", "Last Stand",
    "ZomBotany 2", "Wall-nut Bowling 2", "Pogo Party", "Dr. Zomboss's Revenge", "Art Challenge Wall-nut", "Sunny Day", "Unsodded", "Big Time",
    "Art Challenge Sunflower", "Air Raid", "Ice Level", "Zen Garden trophy", "High Gravity", "Grave Danger", "Can You Dig It? (mini-game)", "Dark Stormy Night",
    "Bungee Blitz", "Squirrel",
];
const PUZZLES: [&str; 19] = [
    "Vasebreaker", "To the Left", "Third Vase", "Chain Reaction", "M is for Metal", "Scary Potter", "Hokey Pokey", "Another Chain Reaction", "Ace of Vase",
    "Vasebreaker Endless streak", "I, Zombie", "I, Zombie Too", "Can You Dig It? (puzzle)", "Totally Nuts", "Dead Zeppelin", "Me Smash!", "ZomBoogie",
    "Three Hit Wonder", "All your brainz",
];
const SHOP: [(&str, usize); 17] = [
    ("Gatling Pea", 0x1A0),
    ("Twin Sunflower", 0x1A4),
    ("Gloom-shroom", 0x1A8),
    ("Cattail", 0x1AC),
    ("Winter Melon", 0x1B0),
    ("Gold Magnet", 0x1B4),
    ("Spikerock", 0x1B8),
    ("Cob Cannon", 0x1BC),
    ("Imitater", 0x1C0),
    ("Golden Watering Can", 0x1D4),
    ("Phonograph", 0x1E0),
    ("Gardening Glove", 0x1E4),
    ("Mushroom Garden", 0x1E8),
    ("Wheel Barrow", 0x1EC),
    ("Pool Cleaner", 0x1F8),
    ("Roof Cleaner", 0x1FC),
    ("Aquarium Garden", 0x204),
];
const PLANT_NAMES: [&str; 49] = [
    "Peashooter", "Sunflower", "Cherry Bomb", "Wall-nut", "Potato Mine", "Snow Pea", "Chomper", "Repeater", "Puff-shroom", "Sun-shroom", "Fume-shroom",
    "Grave Buster", "Hypno-shroom", "Scaredy-shroom", "Ice-shroom", "Doom-shroom", "Lily Pad", "Squash", "Threepeater", "Tangle Kelp", "Jalapeno",
    "Spikeweed", "Torchwood", "Tall-nut", "Sea-shroom", "Plantern", "Cactus", "Blover", "Split Pea", "Starfruit", "Pumpkin", "Magnet-shroom",
    "Cabbage-pult", "Flower Pot", "Kernel-pult", "Coffee Bean", "Garlic", "Umbrella Leaf", "Marigold", "Melon-pult", "Gatling Pea", "Twin Sunflower",
    "Gloom-shroom", "Cattail", "Winter Melon", "Gold Magnet", "Spikerock", "Cob Cannon", "Imitater",
];
const PLACES: [&str; 4] = ["Zen Garden", "Mushroom Garden", "Wheel Barrow", "Aquarium"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Plants vs. Zombies profile".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Adventure", "Current level (1–50)"], 4, Kind::U32, 1, 999);
    b.ranged(&["Adventure", "Money (the game shows this × 10)"], 8, Kind::U32, 0, 9999);
    b.ranged(&["Adventure", "Times completed"], 0xC, Kind::U32, 0, 999);
    b.ranged(&["Adventure", "Tree of Wisdom height (feet)"], 0xD4, Kind::U32, 0, 9999);
    b.ranged(&["Adventure", "Seed slot upgrades"], 0x1F4, Kind::U32, 0, 4);
    b.ranged(&["Adventure", "Mini-games unlocked (1 = yes)"], 0x300, Kind::U32, 0, 1);
    b.ranged(&["Adventure", "Puzzle mode unlocked (1 = yes)"], 0x304, Kind::U32, 0, 1);
    for (name, at, max) in SURVIVAL {
        b.ranged(&["Survival", &format!("{name}: rounds completed")], at, Kind::U32, 0, max);
    }
    for (name, at) in ENDLESS {
        b.ranged(&["Survival", &format!("Endless {name}: best streak")], at, Kind::U32, 0, 9999);
    }
    for (i, name) in MINIGAMES.iter().enumerate() {
        b.ranged(&["Trophies", "Mini-games", name], 0x4C + 4 * i, Kind::U32, 0, 1);
    }
    for (i, name) in PUZZLES.iter().enumerate() {
        let at = 0xD8 + 4 * i;
        // The two streak counters sit among the trophies.
        if *name == "Vasebreaker Endless streak" {
            b.ranged(&["Trophies", "Puzzle", name], at, Kind::U32, 0, 9999);
        } else {
            b.ranged(&["Trophies", "Puzzle", name], at, Kind::U32, 0, 1);
        }
    }
    b.ranged(&["Trophies", "Puzzle", "I, Zombie Endless streak"], 0x124, Kind::U32, 0, 9999);
    for (name, at) in SHOP {
        b.ranged(&["Shop", name, "Owned (1 = yes)"], at, Kind::U32, 0, 1);
    }
    for (name, at) in [("Fertilizer", 0x1D8), ("Bug spray", 0x1DC), ("Chocolate", 0x208), ("Tree food", 0x210)] {
        b.ranged(&["Shop", name, "Amount + 1000 (0 hides it)"], at, Kind::U32, 0, 1999);
    }
    b.ranged(&["Shop", "Garden rake", "Uses left"], 0x200, Kind::U32, 0, 3);
    let n = u32_at(d, PLANTS).unwrap_or(0) as usize;
    b.info(&["Zen Garden", "Plants"], n as i64);
    for i in 0..n {
        let r = RECORDS + i * RECORD;
        let ty = u32_at(d, r).unwrap_or(0) as usize;
        let place = u32_at(d, r + 4).unwrap_or(0) as usize;
        let name = format!("{:02} {}", i + 1, PLANT_NAMES.get(ty).copied().unwrap_or("Plant"));
        b.info(&["Zen Garden", &name, "Where"], PLACES.get(place).copied().unwrap_or("?"));
        b.ranged(&["Zen Garden", &name, "Times fertilized (up to 3)"], r + 0x24, Kind::U32, 0, 3);
        b.ranged(&["Zen Garden", &name, "Times watered (up to 6)"], r + 0x28, Kind::U32, 0, 6);
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Money is stored divided by 10; the game's cap is 99,990. Achievements aren't shown."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; RECORDS + 2 * RECORD + 80];
        let mut put = |at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(0, 12);
        put(4, 13);
        put(8, 1);
        put(0x1A0, 1);
        put(0x1D8, 1017);
        put(PLANTS, 2);
        put(RECORDS, 5);
        put(RECORDS + RECORD, 1);
        put(RECORDS + RECORD + 4, 1);
        d
    }

    #[test]
    fn reads_and_edits() {
        let d = sample();
        assert!(looks(&d));
        assert!(!looks(&d[..100]));
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Adventure/Current level (1–50)"), Some(Value::from(13)));
        assert_eq!(v(&doc, "/Shop/Gatling Pea/Owned (1 = yes)"), Some(Value::from(1)));
        assert_eq!(v(&doc, "/Shop/Fertilizer/Amount + 1000 (0 hides it)"), Some(Value::from(1017)));
        assert_eq!(v(&doc, "/Zen Garden/01 Snow Pea/Where"), Some(Value::from("Zen Garden")));
        assert_eq!(v(&doc, "/Zen Garden/02 Sunflower/Where"), Some(Value::from("Mushroom Garden")));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Adventure/Money (the game shows this × 10)", Value::from(9999)).unwrap();
        assert!(doc.set("/Adventure/Money (the game shows this × 10)", Value::from(10_000)).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, 8), Some(9999));
        assert_eq!(out.len(), d.len());
    }

    /// Research aid: SAVE_EDITOR_PVZ = `userN.dat` files separated by `|`.
    #[test]
    #[ignore]
    fn real_files() {
        for path in std::env::var("SAVE_EDITOR_PVZ").unwrap().split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            let doc = parse(&d).unwrap();
            println!("{}", serde_json::to_string(&doc.view["Zen Garden"]).unwrap());
            assert_eq!(doc.to_bytes().unwrap(), d);
        }
    }
}
