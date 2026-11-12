//! Like a Dragon: Ishin! (PC) `data.sav`, once the Dragon Engine wrapper is taken off (see
//! `rgg.rs`): a 193,792-byte image of the game's save data, laid out like the game's memory.
//! The distance between two values in the save is the distance between their addresses in
//! the running game, so a cheat table that lists the in-game addresses of the items, the
//! style experience and the orbs (the public one in niemasd/Game-Saves) pins all of them
//! once one is known; the whole table was checked against a real save in which these are
//! at their maximum. Item stacks cap at 999.
//!
//! Ryo (the money) is not mapped: no sample holds a Ryo figure that isn't maxed and the
//! table doesn't list it, so it stays in the Tree/Hex view.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Like a Dragon: Ishin! save";
const LEN: usize = 193_792;

/// Style experience and training orbs (Gunman, Wild Dancer, Brawler, Training).
const EXPERIENCE: [(&str, usize); 4] = [("Swordsman", 0x7340), ("Gunman", 0x7344), ("Wild Dancer", 0x7348), ("Brawler", 0x734c)];
const ORBS: [(&str, usize); 4] = [("Gunman", 0x7411), ("Wild Dancer", 0x7412), ("Brawler", 0x7413), ("Training", 0x7414)];

const FOOD: &[(&str, usize)] = &[
    ("Potato", 0x1265a),
    ("Daikon", 0x12672),
    ("Carrot", 0x1268a),
    ("Spinach", 0x126a2),
    ("Sweet Potato", 0x126ba),
    ("Broad Bean", 0x126d2),
    ("Onion", 0x126ea),
    ("Eggplant", 0x12702),
    ("Green Onion", 0x1271a),
    ("Turnip", 0x12732),
    ("Pumpkin", 0x1274a),
    ("Burdock Root", 0x12762),
    ("Ginger", 0x1277a),
    ("Garlic", 0x12792),
    ("Bok Choy", 0x127aa),
    ("Hot Pepper", 0x127c2),
    ("Strawberry", 0x127da),
    ("Cabbage", 0x127f2),
    ("Taro", 0x1280a),
    ("Tomato", 0x12822),
    ("Ginseng", 0x1283a),
    ("Egg", 0x12852),
    ("Udon Flour", 0x1286a),
    ("Honey", 0x12882),
    ("Ginkgo", 0x1289a),
    ("Out-of-season Plum", 0x128b2),
];

const SEAFOOD: &[(&str, usize)] = &[
    ("Whitebait", 0x1295a),
    ("Goby", 0x12972),
    ("Porcupinefish", 0x1298a),
    ("Crucian Carp", 0x129a2),
    ("Sweetfish", 0x129ba),
    ("Squid", 0x129d2),
    ("Freshwater Eel", 0x129ea),
    ("Electric Catfish", 0x12a02),
    ("Thread-Sail Filefish", 0x12a1a),
    ("Tiger Prawn", 0x12a32),
    ("Carp", 0x12a4a),
    ("Common Octopus", 0x12a62),
    ("Conger Eel", 0x12a7a),
    ("Scorpionfish", 0x12a92),
    ("Righteye Flounder", 0x12aaa),
    ("Softshell Turtle", 0x12ac2),
    ("Spider Crab", 0x12ada),
    ("Nishikigoi", 0x12af2),
    ("Rainbow Trout", 0x12b0a),
    ("Salmon", 0x12b22),
    ("Japanese Puffer", 0x12b3a),
    ("Great White Shark", 0x12b52),
    ("Sea Breem", 0x12b6a),
    ("Stringfish", 0x12b82),
    ("Marlin", 0x12b9a),
    ("Ghost Koi", 0x12bb2),
    ("Bluefin Tuna", 0x12bca),
    ("Oarfish", 0x12be2),
];

const MATERIALS: &[(&str, usize)] = &[
    ("Metal Sheet", 0x12dda),
    ("Precious Steel", 0x12df2),
    ("Silver Scrap", 0x12e0a),
    ("Gold Nugget", 0x12e22),
    ("Gold Chunk", 0x12e3a),
    ("Platinum Filings", 0x12e52),
    ("Platinum Chunk", 0x12e6a),
    ("Rainbow Shard", 0x12e82),
    ("Misshapen Screw", 0x12e9a),
    ("New Screw", 0x12eb2),
    ("Worn Out Cogwheel", 0x12eca),
    ("Exquisite Cogwheel", 0x12ee2),
    ("Rusty Spring", 0x12efa),
    ("Small Wind-Up Device", 0x12f12),
    ("Mechanical Parts", 0x12f2a),
    ("Broken Compass", 0x12f42),
    ("Precise Machinery", 0x12f5a),
    ("Dead Branch", 0x12f72),
    ("Solid Wood", 0x12f8a),
    ("Soft Lumber", 0x12fa2),
    ("Sturdy Lumber", 0x12fba),
    ("Driftwood", 0x12fd2),
    ("Thousand-Year Tree Branch", 0x12fea),
    ("Sacred Wood", 0x13002),
    ("Quality Hide", 0x1301a),
    ("Beautiful Pelt", 0x13032),
    ("Wild Silk", 0x1304a),
    ("Beast's Tail", 0x13062),
    ("Fine Horse's Mane", 0x1307a),
    ("Vermilion Fabric", 0x13092),
    ("Fine Silk Fabric", 0x130aa),
    ("European Fabric", 0x130c2),
    ("Rainbow Fabric", 0x130da),
    ("Iron Hammer", 0x130f2),
    ("Bronze Hammer", 0x1310a),
    ("Silver Hammer", 0x13122),
    ("Gold Hammer", 0x1313a),
    ("Kiwami Hammer", 0x13152),
    ("Brown Metal Dust", 0x1316a),
    ("Green Metal Dust", 0x13182),
    ("Grey Metal Dust", 0x1319a),
    ("Black Metal Dust", 0x131b2),
    ("White Metal Dust", 0x131ca),
    ("Bronze Metal Dust", 0x131e2),
    ("Silver Metal Dust", 0x131fa),
    ("Gold Metal Dust", 0x13212),
    ("Grinding Stone", 0x1322a),
    ("Charcoal", 0x13242),
    ("Volcanic Rock Fragment", 0x1325a),
    ("Ceramic Sake Bottle", 0x13272),
    ("Crooked Magnet", 0x1328a),
    ("Cloudy Hand Mirror", 0x132a2),
    ("Cracked Cutting Board", 0x132ba),
    ("Lime Powder", 0x132d2),
    ("Beautiful Leaf", 0x132ea),
    ("Amber", 0x13302),
    ("Picture of a Ghost", 0x1331a),
    ("Gold Dust", 0x13332),
    ("Lead Chunk", 0x1334a),
    ("Emerald", 0x13362),
    ("Pearl", 0x1337a),
    ("Elegant Hair Ornament", 0x13392),
    ("Obsidian Chunk", 0x133aa),
    ("Carved Wooden Bear", 0x133c2),
    ("Ominous Prayer Beads", 0x133da),
    ("Fluorite", 0x133f2),
    ("Broken Awl", 0x1340a),
    ("Ronin's Hair Tie", 0x13422),
    ("Straw Effigy", 0x1343a),
    ("Water God Stone", 0x13452),
    ("Flint Stone", 0x1346a),
    ("Jet Black Belt", 0x13482),
    ("Killer's Handcuffs", 0x1349a),
    ("Marksman's Fan", 0x134b2),
    ("Bloodstained Cloth", 0x134ca),
    ("Aquamarine", 0x134e2),
    ("Gold Seal", 0x134fa),
    ("Charming Doll", 0x13512),
    ("Tear of the Dragon", 0x1352a),
    ("Crystal", 0x13542),
    ("Phoenix Feather", 0x1355a),
    ("Ancient Amulet", 0x13572),
    ("Crimson Bead", 0x1358a),
    ("Eye of the Dragon", 0x135a2),
    ("Golden Medicine Case", 0x135ba),
    ("Great Serpent Skin", 0x135d2),
    ("Godslayer's Talisman", 0x135ea),
    ("Dragon Whisker", 0x13602),
    ("Dragon Fang", 0x1361a),
    ("Provincial Dojo Certificate", 0x13632),
    ("Snake Eyepatch", 0x1364a),
    ("Torn Haori", 0x13662),
    ("Firework Ball", 0x1367a),
    ("Sphere Seal: Educated", 0x13692),
    ("Sphere Seal: Loyalty", 0x136aa),
    ("Sphere Seal: Corporal", 0x136c2),
    ("Sphere Seal: Attack", 0x136da),
    ("Sphere Seal: Fierce Attack", 0x136f2),
    ("Sphere Seal: Pierce", 0x1370a),
    ("Sphere Seal: Crush", 0x13722),
    ("Sphere Seal: Madness", 0x1373a),
    ("Square Seal: Raging Flame", 0x13752),
    ("Square Seal: Champion", 0x1376a),
    ("Sword Seal: Mighty", 0x13782),
    ("Sword Seal: Composure", 0x1379a),
    ("Sword Seal: Fierce Deity", 0x137b2),
    ("Gun Seal: Double Shot", 0x137ca),
    ("Exclusive Seal: Rapid Fire", 0x137e2),
    ("Exclusive Seal: Flash Fire", 0x137fa),
    ("Square Seal: Brawl", 0x13812),
    ("Square Seal: Fierce Battle", 0x1382a),
    ("Exclusive Seal: Grip", 0x13842),
    ("Sphere Seal: Drunken Craze", 0x1385a),
    ("Sphere Seal: Fighting Spirit", 0x13872),
    ("Sphere Seal: On the Brink", 0x1388a),
    ("Sphere Seal: Special Moves", 0x138a2),
    ("Square Seal: Drunken Craze", 0x138ba),
    ("Square Seal: Fighting Spirit", 0x138d2),
    ("Square Seal: On the Brink", 0x138ea),
    ("Square Seal: Special Moves", 0x13902),
    ("Exclusive Seal: Calm", 0x1391a),
    ("Gun Seal: Sure Shot", 0x13932),
    ("Exclusive Seal: Tiger", 0x1394a),
    ("Exclusive Seal: Kiku", 0x13962),
    ("Exclusive Seal: Ujishige", 0x1397a),
    ("Sphere Seal: Elements", 0x13992),
    ("Sphere Seal: Bewitched", 0x139aa),
    ("Exclusive Seal: Flame", 0x139c2),
    ("Exclusive Seal: Wind", 0x139da),
    ("Exclusive Seal: Thunder", 0x139f2),
    ("Exclusive Seal: Light", 0x13a0a),
    ("Exclusive Seal: Poison", 0x13a22),
    ("Square Seal: Reversal", 0x13a3a),
    ("Square Seal: Bleached Cloth", 0x13a52),
    ("Square Seal: Chain Mail", 0x13a6a),
    ("Sphere Seal: Endurance", 0x13a82),
    ("Sphere Seal: Adamantine", 0x13a9a),
    ("Exclusive Seal: Bullets", 0x13ab2),
    ("Exclusive Seal: Blades", 0x13aca),
    ("Square Seal: Durable", 0x13ae2),
    ("Square Seal: Fireproof", 0x13afa),
    ("Square Seal: Antidote", 0x13b12),
    ("Square Seal: Hemostasis", 0x13b2a),
    ("Square Seal: Awakened", 0x13b42),
    ("Sphere Seal: Experience", 0x13b5a),
    ("Square Seal: Experience", 0x13b72),
    ("Sphere Seal: Greed", 0x13b8a),
    ("Exclusive Seal: Damned", 0x13ba2),
    ("Square Seal: Heavy Drinker", 0x13bba),
    ("Square Seal: Reverance", 0x13bd2),
    ("Square Seal: Virtuous", 0x13bea),
    ("Exclusive Seal: Sense", 0x13c02),
    ("Exclusive Seal: Perception", 0x13c1a),
    ("Exclusive Seal: Scout", 0x13c32),
    ("Exclusive Seal: Conflict", 0x13c4a),
    ("Exclusive Seal: Coin", 0x13c62),
    ("Sphere Seal: Repentance", 0x13c7a),
    ("Square Seal: Blessing", 0x13c92),
    ("Sphere Seal: Kiwami", 0x13caa),
    ("Square Seal: Kiwami", 0x13cc2),
];

const VALUABLES: &[(&str, usize)] = &[
    ("Wooden Tags", 0x17700),
    ("Mahjong Sticks", 0x17708),
    ("Shogi Points", 0x1770c),
    ("Virtue", 0x17720),
    ("Prize Tickets", 0x1772c),
];

const AMMO: &[(&str, usize)] = &[
    ("Enhanced Rounds", 0xdb5a),
    ("Flash Rounds", 0xdb72),
    ("Thunder Rounds", 0xdb8a),
    ("Acid Rounds", 0xdba2),
    ("Poison Rounds", 0xdbba),
];

pub const MAX_STACK: i64 = 999;

/// The crafting materials the "Max" button of the Materials list raises.
pub const MATERIAL_NAMES: &[&str] = &[
    "Metal Sheet",
    "Precious Steel",
    "Silver Scrap",
    "Gold Nugget",
    "Gold Chunk",
    "Platinum Filings",
    "Platinum Chunk",
    "Rainbow Shard",
    "Misshapen Screw",
    "New Screw",
    "Worn Out Cogwheel",
    "Exquisite Cogwheel",
    "Rusty Spring",
    "Small Wind-Up Device",
    "Mechanical Parts",
    "Broken Compass",
    "Precise Machinery",
    "Dead Branch",
    "Solid Wood",
    "Soft Lumber",
    "Sturdy Lumber",
    "Driftwood",
    "Thousand-Year Tree Branch",
    "Sacred Wood",
    "Quality Hide",
    "Beautiful Pelt",
    "Wild Silk",
    "Beast's Tail",
    "Fine Horse's Mane",
    "Vermilion Fabric",
    "Fine Silk Fabric",
    "European Fabric",
    "Rainbow Fabric",
    "Iron Hammer",
    "Bronze Hammer",
    "Silver Hammer",
    "Gold Hammer",
    "Kiwami Hammer",
    "Brown Metal Dust",
    "Green Metal Dust",
    "Grey Metal Dust",
    "Black Metal Dust",
    "White Metal Dust",
    "Bronze Metal Dust",
    "Silver Metal Dust",
    "Gold Metal Dust",
    "Grinding Stone",
    "Charcoal",
    "Volcanic Rock Fragment",
    "Ceramic Sake Bottle",
    "Crooked Magnet",
    "Cloudy Hand Mirror",
    "Cracked Cutting Board",
    "Lime Powder",
    "Beautiful Leaf",
    "Amber",
    "Picture of a Ghost",
    "Gold Dust",
    "Lead Chunk",
    "Emerald",
    "Pearl",
    "Elegant Hair Ornament",
    "Obsidian Chunk",
    "Carved Wooden Bear",
    "Ominous Prayer Beads",
    "Fluorite",
    "Broken Awl",
    "Ronin's Hair Tie",
    "Straw Effigy",
    "Water God Stone",
    "Flint Stone",
    "Jet Black Belt",
    "Killer's Handcuffs",
    "Marksman's Fan",
    "Bloodstained Cloth",
    "Aquamarine",
    "Gold Seal",
    "Charming Doll",
    "Tear of the Dragon",
    "Crystal",
    "Phoenix Feather",
    "Ancient Amulet",
    "Crimson Bead",
    "Eye of the Dragon",
    "Golden Medicine Case",
    "Great Serpent Skin",
    "Godslayer's Talisman",
    "Dragon Whisker",
    "Dragon Fang",
    "Provincial Dojo Certificate",
    "Snake Eyepatch",
    "Torn Haori",
    "Firework Ball",
    "Sphere Seal: Educated",
    "Sphere Seal: Loyalty",
    "Sphere Seal: Corporal",
    "Sphere Seal: Attack",
    "Sphere Seal: Fierce Attack",
    "Sphere Seal: Pierce",
    "Sphere Seal: Crush",
    "Sphere Seal: Madness",
    "Square Seal: Raging Flame",
    "Square Seal: Champion",
    "Sword Seal: Mighty",
    "Sword Seal: Composure",
    "Sword Seal: Fierce Deity",
    "Gun Seal: Double Shot",
    "Exclusive Seal: Rapid Fire",
    "Exclusive Seal: Flash Fire",
    "Square Seal: Brawl",
    "Square Seal: Fierce Battle",
    "Exclusive Seal: Grip",
    "Sphere Seal: Drunken Craze",
    "Sphere Seal: Fighting Spirit",
    "Sphere Seal: On the Brink",
    "Sphere Seal: Special Moves",
    "Square Seal: Drunken Craze",
    "Square Seal: Fighting Spirit",
    "Square Seal: On the Brink",
    "Square Seal: Special Moves",
    "Exclusive Seal: Calm",
    "Gun Seal: Sure Shot",
    "Exclusive Seal: Tiger",
    "Exclusive Seal: Kiku",
    "Exclusive Seal: Ujishige",
    "Sphere Seal: Elements",
    "Sphere Seal: Bewitched",
    "Exclusive Seal: Flame",
    "Exclusive Seal: Wind",
    "Exclusive Seal: Thunder",
    "Exclusive Seal: Light",
    "Exclusive Seal: Poison",
    "Square Seal: Reversal",
    "Square Seal: Bleached Cloth",
    "Square Seal: Chain Mail",
    "Sphere Seal: Endurance",
    "Sphere Seal: Adamantine",
    "Exclusive Seal: Bullets",
    "Exclusive Seal: Blades",
    "Square Seal: Durable",
    "Square Seal: Fireproof",
    "Square Seal: Antidote",
    "Square Seal: Hemostasis",
    "Square Seal: Awakened",
    "Sphere Seal: Experience",
    "Square Seal: Experience",
    "Sphere Seal: Greed",
    "Exclusive Seal: Damned",
    "Square Seal: Heavy Drinker",
    "Square Seal: Reverance",
    "Square Seal: Virtuous",
    "Exclusive Seal: Sense",
    "Exclusive Seal: Perception",
    "Exclusive Seal: Scout",
    "Exclusive Seal: Conflict",
    "Exclusive Seal: Coin",
    "Sphere Seal: Repentance",
    "Square Seal: Blessing",
    "Sphere Seal: Kiwami",
    "Square Seal: Kiwami",
];

pub fn looks(d: &[u8]) -> bool {
    // The wrapper's plain data starts with the counts 6, 23, 20, 21.
    d.len() == LEN && d.starts_with(&[6, 0, 0, 0, 0x17, 0, 0, 0, 0x14, 0, 0, 0, 0x15, 0, 0, 0])
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Like a Dragon: Ishin! save".into());
    }
    let mut b = Builder::new(d);
    for (name, at) in EXPERIENCE {
        b.ranged(&["Style experience", name], at, Kind::U32, 0, 999_999);
    }
    for (name, at) in ORBS {
        b.ranged(&["Orbs", name], at, Kind::U8, 0, 99);
    }
    for (group, items) in [("Food", FOOD), ("Seafood", SEAFOOD), ("Materials", MATERIALS)] {
        for &(name, at) in items {
            b.ranged(&[group, name], at, Kind::U16, 0, MAX_STACK);
        }
    }
    for &(name, at) in VALUABLES {
        b.ranged(&["Valuables", name], at, Kind::U32, 0, 999_999_999);
    }
    for &(name, at) in AMMO {
        b.ranged(&["Ammo", name], at, Kind::U8, 0, 99);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Food, seafood and crafting materials (up to 999 each), the valuables (virtue, tokens, tickets), ammunition, style experience and training orbs. The wrapper's checksum is rewritten.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> Vec<u8> {
        let mut d = vec![0u8; LEN];
        d[..16].copy_from_slice(&[6, 0, 0, 0, 0x17, 0, 0, 0, 0x14, 0, 0, 0, 0x15, 0, 0, 0]);
        d
    }

    #[test]
    fn stacks_and_wallet() {
        let mut d = image();
        let (_, at) = MATERIALS[0];
        d[at..at + 2].copy_from_slice(&12u16.to_le_bytes());
        let mut doc = parse(&d).unwrap();
        let p = format!("/Materials/{}", MATERIALS[0].0);
        assert_eq!(doc.view.pointer(&p).unwrap(), 12);
        doc.set(&p, serde_json::Value::from(999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u16::from_le_bytes([out[at], out[at + 1]]), 999);
        assert!(doc.set(&p, serde_json::Value::from(1000)).is_err());
    }

    #[test]
    fn rejects_other_images() {
        assert!(!looks(&vec![0u8; LEN]));
        assert!(!looks(&image()[..1000]));
    }

    #[test]
    fn offsets_inside_the_image() {
        for &(_, at) in FOOD.iter().chain(SEAFOOD).chain(MATERIALS).chain(VALUABLES).chain(AMMO) {
            assert!(at + 4 <= LEN);
        }
    }
}
