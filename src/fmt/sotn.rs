//! Castlevania: Symphony of the Night (PlayStation 1) saves: one 8 KiB block per save. Offsets,
//! item ids and names follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection
//! (see THIRD_PARTY_NOTICES.md). Every region but Europe has two more icon frames in the header,
//! which moves the data down by 0x100.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{extra_frames, is_save, opts, title};
use crate::tree::TreeDoc;

pub const KIND: &str = "Castlevania: Symphony of the Night save";

pub fn looks(d: &[u8]) -> bool {
    if !is_save(d, 1) {
        return false;
    }
    let t = title(d);
    t.starts_with("CASTLEVANIA") || d[4..14] == [0x83, 0x68, 0x83, 0x89, 0x83, 0x4c, 0x83, 0x85, 0x83, 0x89]
}

fn names(list: &[(u8, u8, &str)], kind: u8, extra: &[(&str, u64)]) -> Vec<(String, u64)> {
    let mut v = opts(extra);
    v.extend(list.iter().filter(|(_, k, _)| *k == kind).map(|(i, _, n)| (n.to_string(), *i as u64)));
    v
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Symphony of the Night save".into());
    }
    let s = extra_frames(d) * 0x80;
    let mut b = Builder::new(d);
    let u32n = |b: &mut Builder, path: &[&str], at: usize, hi: i64| {
        b.ranged(path, s + at, Kind::U32, 0, hi);
    };
    b.info(&["Info", "Save title"], title(d));
    u32n(&mut b, &["General", "Gold"], 0x3c4, 999_999);
    b.mirror(&["General", "Gold"], s + 0x110);
    u32n(&mut b, &["General", "Kills"], 0x3c8, 999_999);
    u32n(&mut b, &["General", "Play time hours"], 0x404, 99);
    b.mirror(&["General", "Play time hours"], s + 0x114);
    u32n(&mut b, &["General", "Play time minutes"], 0x408, 59);
    b.mirror(&["General", "Play time minutes"], s + 0x118);
    u32n(&mut b, &["General", "Play time seconds"], 0x40c, 59);
    b.mirror(&["General", "Play time seconds"], s + 0x11c);
    b.pick(&["General", "Progression"], s + 0x124, Kind::U8, &opts(&[("-", 0), ("Clear", 1), ("Replay", 2), ("Replay Clear", 3)]));
    b.pick(&["General", "Character"], s + 0x130, Kind::U32, &opts(&[("Alucard", 0), ("Richter", 1)]));
    b.ranged(&["Status", "Level"], s + 0x3bc, Kind::U32, 1, 99);
    u32n(&mut b, &["Status", "Experience"], 0x3c0, 999_999);
    for (n, at) in [("HP", 0x374), ("MP", 0x384), ("Hearts", 0x37c)] {
        let cur = format!("{n} (now)");
        let max = format!("{n} (max)");
        u32n(&mut b, &["Status", &cur], at, 9999);
        u32n(&mut b, &["Status", &max], at + 4, 9999);
    }
    for (n, at) in [("Strength", 0x38c), ("Constitution", 0x390), ("Intelligence", 0x394), ("Luck", 0x398)] {
        b.ranged(&["Status", n], s + at, Kind::U32, 1, 99);
    }
    b.pick(
        &["Status", "Subweapon"],
        s + 0x3d0,
        Kind::U8,
        &opts(&[("-", 0), ("Knife", 1), ("Axe", 2), ("Holy Water", 3), ("Cross", 4), ("Book", 5), ("Clock", 6), ("Diamond", 7), ("Fist", 8), ("Gun", 9)]),
    );
    let hands = |first: bool| {
        let mut v = opts(&[("-", 0)]);
        let _ = first;
        v.extend(HANDS.iter().map(|(i, _, n)| (n.to_string(), *i as u64)));
        v
    };
    b.pick(&["Equipped", "Right hand"], s + 0x3d4, Kind::U32, &hands(true));
    b.pick(&["Equipped", "Left hand"], s + 0x3d8, Kind::U32, &hands(false));
    b.pick(&["Equipped", "Head"], s + 0x3dc, Kind::U32, &names(EQUIP, 0, &[("-", 0x1a)]));
    b.pick(&["Equipped", "Body"], s + 0x3e0, Kind::U32, &names(EQUIP, 1, &[("-", 0)]));
    b.pick(&["Equipped", "Cloak"], s + 0x3e4, Kind::U32, &names(EQUIP, 2, &[("-", 0x30)]));
    b.pick(&["Equipped", "Other 1"], s + 0x3e8, Kind::U32, &names(EQUIP, 3, &[("-", 0x39)]));
    b.pick(&["Equipped", "Other 2"], s + 0x3ec, Kind::U32, &names(EQUIP, 3, &[("-", 0x39)]));
    for (n, i) in HANDS.iter().map(|(i, _, n)| (n, *i)) {
        b.ranged(&["Hand items", n], s + 0x15e + i as usize, Kind::U8, 0, 99);
    }
    for (n, i) in EQUIP.iter().map(|(i, _, n)| (n, *i)) {
        b.ranged(&["Equipment", n], s + 0x207 + i as usize, Kind::U8, 0, 99);
    }
    let spells = opts(&[("-", 0), ("Dark Metamorphosis", 0x80), ("Summon Spirit", 0x81), ("Hellfire", 0x82), ("Tetra Spirit", 0x83), ("Wolf Charge", 0x84), ("Soul Steal", 0x85), ("Wing Smash", 0x86), ("Sword Brothers", 0x87)]);
    for n in 0..7 {
        let name = format!("Spell {}", n + 1);
        b.pick(&["Spells", &name], s + 0x156 + n, Kind::U8, &spells);
    }
    let relic = opts(&[("-", 0), ("Off", 1), ("On", 3)]);
    for &(n, at) in RELICS {
        b.pick(&["Relics", n], s + at, Kind::U8, &relic);
    }
    for n in 0..7 {
        let name = ["Bat", "Ghost", "Faerie", "Demon", "Sword", "Faerie (JP)", "Demon (JP)"][n];
        b.ranged(&["Familiars", name, "Level"], s + 0x418 + 0xc * n, Kind::U32, 1, 99);
        b.ranged(&["Familiars", name, "Experience"], s + 0x41c + 0xc * n, Kind::U32, 0, 9899);
    }
    for &(n, at, bit) in WARP {
        super::retro::flag(&mut b, &["Warp rooms", n], s + at, bit);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. Gold, play time and the other numbers are limited to what the game can show; the save's memory-card preview (name, gold and time) is kept in step.")))
}

const RELICS: &[(&str, usize)] = &[
    ("Soul of Bat", 0x138), ("Fire of Bat", 0x139), ("Echo of Bat", 0x13a), ("Force of Echo", 0x13b),
    ("Soul of Wolf", 0x13c), ("Power of Wolf", 0x13d), ("Skill of Wolf", 0x13e),
    ("Form of Mist", 0x13f), ("Power of Mist", 0x140), ("Gas Cloud", 0x141),
    ("Cube of Zoe", 0x142), ("Spirit Orb", 0x143), ("Gravity Boots", 0x144), ("Leap Stone", 0x145),
    ("Holy Symbol", 0x146), ("Faerie Scroll", 0x147), ("Jewel of Open", 0x148), ("Merman Statue", 0x149),
    ("Bat Card", 0x14a), ("Ghost Card", 0x14b), ("Faerie Card", 0x14c), ("Demon Card", 0x14d), ("Sword Card", 0x14e),
    ("Faerie Card (JP)", 0x14f), ("Demon Card (JP)", 0x150),
    ("Heart of Vlad", 0x151), ("Tooth of Vlad", 0x152), ("Rib of Vlad", 0x153), ("Ring of Vlad", 0x154), ("Eye of Vlad", 0x155),
];

const WARP: &[(&str, usize, u32)] = &[
    ("Entrance", 0x698, 0), ("Abandoned Mine", 0x698, 1), ("Outer Wall", 0x698, 2), ("Castle Keep", 0x698, 3), ("Olrox's Quarters", 0x698, 4),
    ("Reverse Entrance", 0x699, 0), ("Cave", 0x699, 1), ("Reverse Outer Wall", 0x699, 2), ("Reverse Keep", 0x699, 3), ("Death Wing's Lair", 0x699, 4),
];

/// (id, kind, name) of the hand items (counts at 0x15e + id).
const HANDS: &[(u8, u8, &str)] = &[
    (0x01, 8, "Monster vial 1"),
    (0x02, 8, "Monster vial 2"),
    (0x03, 8, "Monster vial 3"),
    (0x04, 4, "Shield rod"),
    (0x05, 9, "Leather shield"),
    (0x06, 9, "Knight shield"),
    (0x07, 9, "Iron shield"),
    (0x08, 9, "AxeLord shield"),
    (0x09, 9, "Herald shield"),
    (0x0a, 9, "Dark shield"),
    (0x0b, 9, "Goddess shield"),
    (0x0c, 9, "Shaman shield"),
    (0x0d, 9, "Medusa shield"),
    (0x0e, 9, "Skull shield"),
    (0x0f, 9, "Fire shield"),
    (0x10, 9, "Alucard shield"),
    (0x11, 5, "Sword of Dawn"),
    (0x12, 0, "Basilard"),
    (0x13, 0, "Short sword"),
    (0x14, 0, "Combat knife"),
    (0x15, 5, "Nunchaku"),
    (0x16, 0, "Were Bane"),
    (0x17, 0, "Rapier"),
    (0x18, 7, "Karma Coin"),
    (0x19, 8, "Magic Missile"),
    (0x1a, 5, "Red Rust"),
    (0x1b, 5, "Takemitsu"),
    (0x1c, 0, "Shotel"),
    (0x1d, 6, "Orange"),
    (0x1e, 6, "Apple"),
    (0x1f, 6, "Banana"),
    (0x20, 6, "Grapes"),
    (0x21, 6, "Strawberry"),
    (0x22, 6, "Pineapple"),
    (0x23, 6, "Peanuts"),
    (0x24, 6, "Toadstool"),
    (0x25, 6, "Shiitake"),
    (0x26, 6, "Cheesecake"),
    (0x27, 6, "Shortcake"),
    (0x28, 6, "Tart"),
    (0x29, 6, "Parfait"),
    (0x2a, 6, "Pudding"),
    (0x2b, 6, "Ice cream"),
    (0x2c, 6, "Frankfurter"),
    (0x2d, 6, "Hamburger"),
    (0x2e, 6, "Pizza"),
    (0x2f, 6, "Cheese"),
    (0x30, 6, "Ham and eggs"),
    (0x31, 6, "Omelette"),
    (0x32, 6, "Morning set"),
    (0x33, 6, "Lunch A"),
    (0x34, 6, "Lunch B"),
    (0x35, 6, "Curry rice"),
    (0x36, 6, "Gyros plate"),
    (0x37, 6, "Spaghetti"),
    (0x38, 6, "Grape juice"),
    (0x39, 6, "Barley tea"),
    (0x3a, 6, "Green tea"),
    (0x3b, 6, "Natou"),
    (0x3c, 6, "Ramen"),
    (0x3d, 6, "Miso soup"),
    (0x3e, 6, "Sushi"),
    (0x3f, 6, "Pork bun"),
    (0x40, 6, "Red bean bun"),
    (0x41, 6, "Chinese bun"),
    (0x42, 6, "Dim Sum set"),
    (0x43, 6, "Pot roast"),
    (0x44, 6, "Sirloin"),
    (0x45, 6, "Turkey"),
    (0x46, 6, "Meal ticket"),
    (0x47, 7, "Neutron bomb"),
    (0x48, 7, "Power of Sire"),
    (0x49, 7, "Pentagram"),
    (0x4a, 7, "Bat Pentagram"),
    (0x4b, 8, "Shuriken"),
    (0x4c, 8, "Cross shuriken"),
    (0x4d, 8, "Buffalo star"),
    (0x4e, 8, "Flame star"),
    (0x4f, 8, "TNT"),
    (0x50, 8, "Bwaka knife"),
    (0x51, 8, "Boomerang"),
    (0x52, 8, "Javelin"),
    (0x53, 1, "Tyrfing"),
    (0x54, 5, "Namakura"),
    (0x55, 3, "Knuckle duster"),
    (0x56, 1, "Gladius"),
    (0x57, 1, "Scimitar"),
    (0x58, 1, "Cutlass"),
    (0x59, 1, "Saber"),
    (0x5a, 1, "Falchion"),
    (0x5b, 1, "Broadsword"),
    (0x5c, 1, "Bekatowa"),
    (0x5d, 1, "Damascus sword"),
    (0x5e, 1, "Hunter sword"),
    (0x5f, 5, "Estoc"),
    (0x60, 1, "Bastard sword"),
    (0x61, 3, "Jewel knuckles"),
    (0x62, 5, "Claymore"),
    (0x63, 1, "Talwar"),
    (0x64, 5, "Katana"),
    (0x65, 5, "Flamberge"),
    (0x66, 3, "Iron Fist"),
    (0x67, 5, "Zwei hander"),
    (0x68, 1, "Sword of Hador"),
    (0x69, 1, "Luminus"),
    (0x6a, 1, "Harper"),
    (0x6b, 5, "Obsidian sword"),
    (0x6c, 1, "Gram"),
    (0x6d, 0, "Jewel sword"),
    (0x6e, 1, "Mormegil"),
    (0x6f, 1, "Firebrand"),
    (0x70, 1, "Thunderbrand"),
    (0x71, 1, "Icebrand"),
    (0x72, 0, "Stone sword"),
    (0x73, 1, "Holy sword"),
    (0x74, 1, "Terminus Est"),
    (0x75, 1, "Marsil"),
    (0x76, 1, "Dark Blade"),
    (0x77, 2, "Heaven sword"),
    (0x78, 3, "Fist of Tulkas"),
    (0x79, 1, "Gurthang"),
    (0x7a, 1, "Mourneblade"),
    (0x7b, 1, "Alucard sword"),
    (0x7c, 1, "Mablung Sword"),
    (0x7d, 1, "Badelaire"),
    (0x7e, 1, "Sword Familiar"),
    (0x7f, 5, "Great Sword"),
    (0x80, 4, "Mace"),
    (0x81, 4, "Morningstar"),
    (0x82, 4, "Holy rod"),
    (0x83, 4, "Star flail"),
    (0x84, 4, "Moon rod"),
    (0x85, 2, "Chakram"),
    (0x86, 8, "Fire boomerang"),
    (0x87, 8, "Iron ball"),
    (0x88, 0, "Holbein dagger"),
    (0x89, 3, "Blue knuckles"),
    (0x8a, 8, "Dynamite"),
    (0x8b, 5, "Osafune katana"),
    (0x8c, 5, "Masamune"),
    (0x8d, 5, "Muramasa"),
    (0x8e, 10, "Heart Refresh"),
    (0x8f, 2, "Runesword"),
    (0x90, 10, "Antivenom"),
    (0x91, 10, "Uncurse"),
    (0x92, 10, "Life apple"),
    (0x93, 10, "Hammer"),
    (0x94, 10, "Str. potion"),
    (0x95, 10, "Luck potion"),
    (0x96, 10, "Smart potion"),
    (0x97, 10, "Attack potion"),
    (0x98, 10, "Shield potion"),
    (0x99, 10, "Resist fire"),
    (0x9a, 10, "Resist thunder"),
    (0x9b, 10, "Resist ice"),
    (0x9c, 10, "Resist stone"),
    (0x9d, 10, "Resist holy"),
    (0x9e, 10, "Resist dark"),
    (0x9f, 10, "Potion"),
    (0xa0, 10, "High potion"),
    (0xa1, 10, "Elixir"),
    (0xa2, 10, "Manna prism"),
    (0xa3, 1, "Vorpal blade"),
    (0xa4, 1, "Crissaegrim"),
    (0xa5, 5, "Yasutsuna"),
    (0xa6, 10, "Library card"),
    (0xa7, 9, "Alucart shield"),
    (0xa8, 1, "Alucart sword"),
];
/// (id, kind, name) of the equipment (counts at 0x207 + id).
const EQUIP: &[(u8, u8, &str)] = &[
    (0x01, 1, "Cloth tunic"),
    (0x02, 1, "Hide cuirass"),
    (0x03, 1, "Bronze cuirass"),
    (0x04, 1, "Iron cuirass"),
    (0x05, 1, "Steel cuirass"),
    (0x06, 1, "Silver plate"),
    (0x07, 1, "Gold plate"),
    (0x08, 1, "Platinum mail"),
    (0x09, 1, "Diamond plate"),
    (0x0a, 1, "Fire mail"),
    (0x0b, 1, "Lightning mail"),
    (0x0c, 1, "Ice mail"),
    (0x0d, 1, "Mirror cuirass"),
    (0x0e, 1, "Spike Breaker"),
    (0x0f, 1, "Alucard mail"),
    (0x10, 1, "Dark armor"),
    (0x11, 1, "Healing mail"),
    (0x12, 1, "Holy mail"),
    (0x13, 1, "Walk armor"),
    (0x14, 1, "Brilliant mail"),
    (0x15, 1, "Mojo mail"),
    (0x16, 1, "Fury plate"),
    (0x17, 1, "Dracula tunic"),
    (0x18, 1, "God's Garb"),
    (0x19, 1, "Axe Lord armor"),
    (0x1b, 0, "Sunglasses"),
    (0x1c, 0, "Ballroom mask"),
    (0x1d, 0, "Bandanna"),
    (0x1e, 0, "Felt hat"),
    (0x1f, 0, "Velvet hat"),
    (0x20, 0, "Goggles"),
    (0x21, 0, "Leather hat"),
    (0x22, 0, "Holy glasses"),
    (0x23, 0, "Steel helm"),
    (0x24, 0, "Stone mask"),
    (0x25, 0, "Circlet"),
    (0x26, 0, "Gold circlet"),
    (0x27, 0, "Ruby circlet"),
    (0x28, 0, "Opal circlet"),
    (0x29, 0, "Topaz circlet"),
    (0x2a, 0, "Beryl circlet"),
    (0x2b, 0, "Cat-eye circl."),
    (0x2c, 0, "Coral circlet"),
    (0x2d, 0, "Dragon helm"),
    (0x2e, 0, "Silver crown"),
    (0x2f, 0, "Wizard hat"),
    (0x31, 2, "Cloth cape"),
    (0x32, 2, "Reverse cloak"),
    (0x33, 2, "Elven cloak"),
    (0x34, 2, "Crystal cloak"),
    (0x35, 2, "Royal cloak"),
    (0x36, 2, "Blood cloak"),
    (0x37, 2, "Joseph's cloak"),
    (0x38, 2, "Twilight cloak"),
    (0x3a, 3, "Moonstone"),
    (0x3b, 3, "Sunstone"),
    (0x3c, 3, "Bloodstone"),
    (0x3d, 3, "Staurolite"),
    (0x3e, 3, "Ring of Pales"),
    (0x3f, 3, "Zircon"),
    (0x40, 3, "Aquamarine"),
    (0x41, 3, "Turquoise"),
    (0x42, 3, "Onyx"),
    (0x43, 3, "Garnet"),
    (0x44, 3, "Opal"),
    (0x45, 3, "Diamond"),
    (0x46, 3, "Lapis lazuli"),
    (0x47, 3, "Ring of Ares"),
    (0x48, 3, "Gold Ring"),
    (0x49, 3, "Silver Ring"),
    (0x4a, 3, "Ring of Varda"),
    (0x4b, 3, "Ring of Arcana"),
    (0x4c, 3, "Mystic pendant"),
    (0x4d, 3, "Heart broach"),
    (0x4e, 3, "Necklace of J"),
    (0x4f, 3, "Gauntlet"),
    (0x50, 3, "Ankh of Life"),
    (0x51, 3, "Ring of Feanor"),
    (0x52, 3, "Medal"),
    (0x53, 3, "Talisman"),
    (0x54, 3, "Duplicator"),
    (0x55, 3, "King's stone"),
    (0x56, 3, "Covenant stone"),
    (0x57, 3, "Nauglamir"),
    (0x58, 3, "Secret boots"),
    (0x59, 1, "Alucart mail"),
];
const HAND_TYPES: &[&str] = &["Short Swords", "Swords", "Throws 1", "Fists", "Clubs", "Two-hands", "Foods", "Bombs", "Throws 2", "Shields", "Medicines"];
const EQUIP_TYPES: &[&str] = &["Headgears", "Armor", "Cloaks", "Accessories"];
