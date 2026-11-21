//! Rogue Legacy (2013): `RogueLegacyPlayer.rcdat` (the character) and `RogueLegacyBP.rcdat`
//! (blueprints, runes and the manor). Both are raw .NET `BinaryWriter` output with no header,
//! so a file is recognised by checking that every field of its fixed front is plausible.
//! Field order and names follow the MIT-licensed editor listed in THIRD_PARTY_NOTICES.md
//! (facts only, no code); everything after the documented front is kept as it is.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const PLAYER_KIND: &str = "Rogue Legacy character";
pub const MANOR_KIND: &str = "Rogue Legacy manor";

const CLASSES: &[&str] = &[
    "Knight",
    "Mage",
    "Barbarian",
    "Knave",
    "Shinobi",
    "Miner",
    "Spellthief",
    "Lich",
    "Paladin",
    "Archmage",
    "Barbarian King",
    "Assassin",
    "Hokage",
    "Spelunker",
    "Spellsword",
    "Lich King",
    "Dragon",
    "Traitor",
];

const SPELLS: &[(u64, &str)] = &[
    (0, "None"),
    (1, "Dagger"),
    (2, "Axe"),
    (3, "Bomb"),
    (4, "Time Stop"),
    (5, "Crow Storm"),
    (6, "Quantum Translocator"),
    (7, "Displacer"),
    (8, "Chakram"),
    (9, "Scythe"),
    (10, "Blade Wall"),
    (11, "Flame Barrier"),
    (12, "Conflux"),
    (13, "Dragon Fire A"),
    (14, "Rapid Dagger"),
    (15, "Dragon Fire B"),
    (100, "B.E.A.M"),
];

const ITEMS: &[(u64, &str)] = &[
    (0, "None"),
    (1, "Charon's Obol"),
    (2, "Hedgehog's Curse"),
    (3, "Hyperion's Ring"),
    (4, "Hermes' Boots"),
    (5, "Helios' Blessing"),
    (6, "Calypso's Compass"),
    (8, "Nerdy Glasses"),
    (9, "Khidr's Obol"),
    (10, "Alexander's Obol"),
    (11, "Ponce de Leon's Obol"),
    (12, "Herodotus' Obol"),
    (13, "Traitor's Obol"),
];

const TRAITS: &[(u64, &str)] = &[
    (0, "None"),
    (1, "Color blind"),
    (2, "Gay"),
    (3, "Near-sighted"),
    (4, "Far-sighted"),
    (5, "Dyslexia"),
    (6, "Gigantism"),
    (7, "Dwarfism"),
    (8, "Baldness"),
    (9, "Endomorph"),
    (10, "Ectomorph"),
    (11, "Alzheimer's"),
    (12, "Dextrocardia"),
    (13, "Coprolalia"),
    (14, "ADHD"),
    (15, "O.C.D."),
    (16, "Hypergonadism"),
    (17, "Muscle weakness"),
    (18, "Stereo blind"),
    (19, "I.B.S."),
    (20, "Vertigo"),
    (21, "Tunnel vision"),
    (22, "Ambilevous"),
    (23, "P.A.D."),
    (24, "Alektorophobia"),
    (25, "Hypochondriac"),
    (26, "Dementia"),
    (27, "Flexible"),
    (28, "Eidetic memory"),
    (29, "Nostalgic"),
    (30, "C.I.P."),
    (31, "Savant"),
    (32, "The One"),
    (33, "Clumsy"),
    (34, "EHS"),
    (35, "Glaucoma"),
    (100, "Adopted"),
];

/// Manor skills in file order: label and the highest level the game allows.
const SKILLS: &[(&str, i64)] = &[
    ("Health Up", 75),
    ("Invulnerability Time Up", 5),
    ("Death Defy", 10),
    ("Attack Up", 75),
    ("Down Strike Up", 5),
    ("Crit Chance Up", 25),
    ("Crit Damage Up", 25),
    ("Magic Damage Up", 75),
    ("Mana Up", 75),
    ("Mana Cost Down", 5),
    ("Smithy", 1),
    ("Enchantress", 1),
    ("Architect", 1),
    ("Equipment Up", 50),
    ("Armor Up", 50),
    ("Gold Gain Up", 5),
    ("Haggle", 5),
    ("Potion Up", 5),
    ("Randomize Children", 1),
    ("Unlock Lich", 1),
    ("Unlock Miner", 1),
    ("Unlock Spellthief", 1),
    ("Unlock Shinobi", 1),
    ("Upgrade Knight", 1),
    ("Upgrade Mage", 1),
    ("Upgrade Knave", 1),
    ("Upgrade Miner", 1),
    ("Upgrade Barbarian", 1),
    ("Upgrade Lich", 1),
    ("Upgrade Shinobi", 1),
    ("Upgrade Spellthief", 1),
    ("Beastiality", 1),
];

const SETS: &[&str] = &["Squire", "Silver", "Guardian", "Imperial", "Royal", "Knight", "Ranger", "Sky", "Dragon", "Slayer", "Blood", "Sage", "Retribution", "Holy", "Dark"];
const PARTS: &[&str] = &["Sword", "Helm", "Chestplate", "Bracers", "Cape"];
const RUNES: &[&str] = &["Vault", "Sprint", "Vampire", "Sky", "Siphon", "Retaliation", "Bounty", "Haste", "Curse", "Grace", "Balance"];
const STATES: &[&str] = &["Not found", "Found", "Seen", "Purchased"];

/// Where each field of the character file lies, found by walking the fixed front.
struct Player {
    gold: usize,
    health: usize,
    mana: usize,
    age: usize,
    child_age: usize,
    spell: usize,
    class: usize,
    item: usize,
    trait_a: usize,
    trait_b: usize,
    name: String,
    /// The first byte after the name.
    after_name: usize,
    len_front: usize,
}

fn varint(d: &[u8], at: usize) -> Option<(usize, usize)> {
    let mut v = 0usize;
    for i in 0..5 {
        let b = *d.get(at + i)?;
        v |= usize::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            return Some((v, i + 1));
        }
    }
    None
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn f32_at(d: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Bytes after the name: three equipment pieces, the diary entry, nine counters (bonus stats and
/// the lich's), a float, six boss flags, two counters, twelve flags, the death count, four more
/// flags, the hours played (float) and three wizard spells.
const AFTER_NAME: usize = 3 + 1 + 8 * 4 + 4 + 6 + 8 + 12 + 4 + 4 + 4 + 3;

fn player(d: &[u8]) -> Option<Player> {
    if d.len() < 20 + AFTER_NAME {
        return None;
    }
    let (gold, health, mana) = (i32_at(d, 0)?, i32_at(d, 4)?, i32_at(d, 8)?);
    if !(0..=2_000_000_000).contains(&gold) || !(0..=1_000_000).contains(&health) || !(0..=1_000_000).contains(&mana) {
        return None;
    }
    let (class, spell, item, ta, tb) = (d[15], d[14], d[16], d[17], d[18]);
    if d[12] == 0 {
        return None;
    }
    if usize::from(class) >= CLASSES.len() || !SPELLS.iter().any(|(v, _)| *v == u64::from(spell)) || !ITEMS.iter().any(|(v, _)| *v == u64::from(item)) {
        return None;
    }
    if !TRAITS.iter().any(|(v, _)| *v == u64::from(ta)) || !TRAITS.iter().any(|(v, _)| *v == u64::from(tb)) {
        return None;
    }
    let (n, used) = varint(d, 19)?;
    if n > 64 || d.len() < 19 + used + n + AFTER_NAME {
        return None;
    }
    let name = std::str::from_utf8(d.get(19 + used..19 + used + n)?).ok()?;
    if name.chars().any(char::is_control) {
        return None;
    }
    let after = 19 + used + n;
    let front_end = after + AFTER_NAME;
    // The flags are single bytes holding 0 or 1.
    let flags = after + 4 + 8 * 4 + 4;
    if d[flags..flags + 6].iter().any(|&b| b > 1) {
        return None;
    }
    let hours = f32_at(d, front_end - 7)?;
    if !hours.is_finite() || !(0.0..=100_000.0).contains(&hours) {
        return None;
    }
    Some(Player { gold: 0, health: 4, mana: 8, age: 12, child_age: 13, spell: 14, class: 15, item: 16, trait_a: 17, trait_b: 18, name: name.to_string(), after_name: after, len_front: front_end })
}

pub fn looks_player(d: &[u8]) -> bool {
    player(d).is_some()
}

const MANOR_FRONT: usize = 5 * 15 + 5 * 11 + 5 + 5 + 32 * 4;

fn manor_ok(d: &[u8]) -> bool {
    if d.len() < MANOR_FRONT || d.len() > MANOR_FRONT + 1024 {
        return false;
    }
    if d[..130].iter().any(|&b| b > 3) {
        return false;
    }
    if d[130..140].iter().any(|&b| b != 0xff && b > 14) {
        return false;
    }
    (0..32).all(|i| i32_at(d, 140 + i * 4).is_some_and(|v| (0..=1000).contains(&v)))
}

pub fn looks_manor(d: &[u8]) -> bool {
    manor_ok(d)
}

fn spell_options() -> Vec<(String, u64)> {
    SPELLS.iter().map(|(v, n)| (n.to_string(), *v)).collect()
}

pub fn parse_player(d: &[u8]) -> Result<TreeDoc, String> {
    let p = player(d).ok_or("not a Rogue Legacy character")?;
    let mut b = Builder::new(d);
    b.field(&["Character", "Gold"], p.gold, Kind::I32);
    b.ranged(&["Character", "Health"], p.health, Kind::I32, 0, 1_000_000);
    b.ranged(&["Character", "Mana"], p.mana, Kind::I32, 0, 1_000_000);
    b.ranged(&["Character", "Age"], p.age, Kind::U8, 0, 255);
    b.ranged(&["Character", "Child age"], p.child_age, Kind::U8, 0, 255);
    b.info(&["Character", "Name"], p.name.clone());
    b.named(&["Character", "Class"], p.class, Kind::U8, CLASSES);
    b.pick(&["Character", "Special item"], p.item, Kind::U8, &ITEMS.iter().map(|(v, n)| (n.to_string(), *v)).collect::<Vec<_>>());
    let traits: Vec<(String, u64)> = TRAITS.iter().map(|(v, n)| (n.to_string(), *v)).collect();
    b.pick(&["Character", "Trait 1"], p.trait_a, Kind::U8, &traits);
    b.pick(&["Character", "Trait 2"], p.trait_b, Kind::U8, &traits);
    let a = p.after_name;
    b.ranged(&["Character", "Bonus health"], a + 4, Kind::I32, 0, 100_000);
    b.ranged(&["Character", "Bonus strength"], a + 8, Kind::I32, 0, 100_000);
    b.ranged(&["Character", "Bonus mana"], a + 12, Kind::I32, 0, 100_000);
    b.ranged(&["Character", "Bonus defense"], a + 16, Kind::I32, 0, 100_000);
    b.ranged(&["Character", "Bonus weight"], a + 20, Kind::I32, 0, 100_000);
    b.ranged(&["Character", "Bonus magic"], a + 24, Kind::I32, 0, 100_000);
    b.ranged(&["Character", "Lich health"], a + 28, Kind::I32, 0, 1_000_000);
    b.ranged(&["Character", "Lich mana"], a + 32, Kind::I32, 0, 1_000_000);
    let bosses = a + 4 + 8 * 4 + 4;
    for (i, label) in ["Boss: newest", "Boss: Khidr", "Boss: Alexander", "Boss: Ponce de Leon", "Boss: Herodotus", "Boss: last"].iter().enumerate() {
        b.field(&["Progress", label], bosses + i, Kind::B8);
    }
    b.ranged(&["Progress", "Times the castle was beaten"], bosses + 6, Kind::I32, 0, 1_000_000);
    b.ranged(&["Progress", "Enemies beaten"], bosses + 10, Kind::I32, 0, 2_000_000_000);
    let flags = bosses + 14;
    for (i, label) in [
        "Tutorial complete",
        "Character found",
        "Start in the starting room",
        "Castle locked",
        "Spoke to the blacksmith",
        "Spoke to the enchantress",
        "Spoke to the architect",
        "Spoke to the toll collector",
        "Dead",
        "Final door opened",
        "Children re-rolled",
        "Female",
    ]
    .iter()
    .enumerate()
    {
        b.field(&["Progress", label], flags + i, Kind::B8);
    }
    let dead = flags + 12;
    b.ranged(&["Progress", "Times died"], dead, Kind::I32, 0, 1_000_000);
    for (i, label) in ["Architect free", "Last diary read", "Spoke to the last boss", "Hardcore mode"].iter().enumerate() {
        b.field(&["Progress", label], dead + 4 + i, Kind::B8);
    }
    let hours = dead + 8;
    b.field(&["Progress", "Hours played"], hours, Kind::F32);
    // The first wizard spell is also kept in the older single-spell field.
    let spells = hours + 4;
    let opts = spell_options();
    b.pick(&["Spells", "Spell 1"], spells, Kind::U8, &opts);
    b.mirror(&["Spells", "Spell 1"], p.spell);
    b.pick(&["Spells", "Spell 2"], spells + 1, Kind::U8, &opts);
    b.pick(&["Spells", "Spell 3"], spells + 2, Kind::U8, &opts);
    debug_assert_eq!(spells + 3, p.len_front);
    let note = crate::i18n::tx("Close the game first. If the profile folder holds AutoSave_ copies of these files the game loads those: edit them too, or remove them. Gold shows in the castle (the toll takes it when a run begins).");
    Ok(b.finish(PLAYER_KIND, Checksum::None).with_note(note))
}

pub fn parse_manor(d: &[u8]) -> Result<TreeDoc, String> {
    if !manor_ok(d) {
        return Err("not a Rogue Legacy manor file".into());
    }
    let mut b = Builder::new(d);
    for (i, (name, max)) in SKILLS.iter().enumerate() {
        b.ranged(&["Manor", name], 140 + i * 4, Kind::I32, 0, *max);
    }
    let states: Vec<(String, u64)> = STATES.iter().enumerate().map(|(i, s)| (s.to_string(), i as u64)).collect();
    for (c, part) in PARTS.iter().enumerate() {
        for (s, set) in SETS.iter().enumerate() {
            b.pick(&["Blueprints", &format!("{part}: {set}")], c * 15 + s, Kind::U8, &states);
        }
    }
    for (c, part) in PARTS.iter().enumerate() {
        for (r, rune) in RUNES.iter().enumerate() {
            b.pick(&["Runes", &format!("{part}: {rune}")], 75 + c * 11 + r, Kind::U8, &states);
        }
    }
    let note = crate::i18n::tx("Close the game first. A skill's level can't go above what the manor allows. If the profile folder holds an AutoSave_RogueLegacyBP.rcdat the game loads that copy: edit it too, or remove it.");
    Ok(b.finish(MANOR_KIND, Checksum::None).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character() -> Vec<u8> {
        let mut d = Vec::new();
        for v in [61_630i32, 110, 120] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&[30, 5, 1, 0, 0, 0, 0]);
        d.push(3);
        d.extend_from_slice(b"Lee");
        d.extend_from_slice(&[1, 2, 3, 0]);
        for v in 0..8i32 {
            d.extend_from_slice(&(v * 2).to_le_bytes());
        }
        d.extend_from_slice(&1.5f32.to_le_bytes());
        d.extend_from_slice(&[1, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&7i32.to_le_bytes());
        d.extend_from_slice(&900i32.to_le_bytes());
        d.extend_from_slice(&[1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1]);
        d.extend_from_slice(&4i32.to_le_bytes());
        d.extend_from_slice(&[0, 1, 0, 0]);
        d.extend_from_slice(&12.5f32.to_le_bytes());
        d.extend_from_slice(&[1, 2, 3]);
        d.extend_from_slice(&[9, 9, 9, 9, 9, 9]); // newer data after the front
        d
    }

    #[test]
    fn reads_edits_and_keeps_the_rest() {
        let d = character();
        assert!(looks_player(&d) && !looks_manor(&d));
        let mut doc = parse_player(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/Gold").unwrap(), 61_630);
        assert_eq!(doc.view.pointer("/Character/Name").unwrap(), "Lee");
        assert_eq!(doc.view.pointer("/Character/Class").unwrap(), "Knight");
        assert_eq!(doc.view.pointer("/Progress/Times died").unwrap(), 4);
        assert_eq!(doc.view.pointer("/Progress/Hours played").unwrap(), 12.5);
        assert_eq!(doc.view.pointer("/Spells/Spell 1").unwrap(), "Dagger");
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Character/Gold", serde_json::json!(5_000_000)).unwrap();
        doc.set("/Spells/Spell 1", serde_json::json!("Axe")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(i32_at(&out, 0), Some(5_000_000));
        assert_eq!(out[14], 2, "the old spell field follows the first wizard spell");
        assert_eq!(&out[out.len() - 6..], &[9; 6]);
        assert_eq!(out.len(), d.len());
    }

    #[test]
    fn a_manor_file() {
        let mut d = vec![0u8; MANOR_FRONT];
        d[130..140].fill(0xff);
        d[140..144].copy_from_slice(&75i32.to_le_bytes());
        assert!(looks_manor(&d));
        let mut doc = parse_manor(&d).unwrap();
        assert_eq!(doc.view.pointer("/Manor/Health Up").unwrap(), 75);
        doc.set("/Manor/Mana Up", serde_json::json!(40)).unwrap();
        assert!(doc.set("/Manor/Smithy", serde_json::json!(2)).is_err());
        assert_eq!(i32_at(&doc.to_bytes().unwrap(), 140 + 8 * 4), Some(40));
        assert_eq!(parse_manor(&d).unwrap().to_bytes().unwrap(), d);
        assert!(!looks_player(&d));
    }

    #[test]
    fn other_files_are_neither() {
        assert!(!looks_player(&[0u8; 300]) || parse_player(&[0u8; 300]).is_ok());
        assert!(!looks_manor(&[7u8; 300]));
    }
}
