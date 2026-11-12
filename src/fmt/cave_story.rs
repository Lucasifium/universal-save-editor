//! Cave Story `Profile.dat`: the freeware release keeps one 0x604-byte profile; Cave Story+
//! (PC) keeps three 0x620-byte slots at the start of a 0x20020-byte file (the rest holds
//! settings and records and is left alone). Nothing is checksummed.
//!
//! Profile layout as read by doukutsu-rs (MIT, `src/game/profile.rs`): magic "Do041220",
//! stage 0x08, max health (u16) 0x1C, Whimsical Stars 0x1E, health 0x20, current weapon 0x24,
//! equipment bits 0x2C, eight weapons of five u32 (type, level, energy, max ammo, ammo) from
//! 0x38, 32 item ids from 0xD8, "FLAG" at 0x218. Cave Story+ stores the difficulty
//! (0 Normal, 2 Easy, 4 Hard) at 0x610 of each slot, as published in speedrunners' notes.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Cave Story save";
const MAGIC: &[u8] = b"Do041220";
const FREEWARE: usize = 0x604;
const PLUS: usize = 0x20020;
const SLOT: usize = 0x620;

const WEAPONS: [&str; 14] = [
    "None", "Snake", "Polar Star", "Fireball", "Machine Gun", "Missile Launcher", "(unused 6)", "Bubbler", "(unused 8)", "Blade",
    "Super Missile Launcher", "(unused 11)", "Nemesis", "Spur",
];

const ITEMS: [&str; 40] = [
    "", "Arthur's Key", "Map System", "Santa's Key", "Silver Locket", "Beast Fang", "Life Capsule", "ID Card", "Jellyfish Juice",
    "Rusty Key", "Gum Key", "Gum Base", "Charcoal", "Explosive", "Puppy", "Life Pot", "Cure-All", "Clinic Key", "Booster 0.8",
    "Arms Barrier", "Turbocharge", "Curly's Air Tank", "Nikumaru Counter", "Booster 2.0", "Mimiga Mask", "Teleporter Room Key",
    "Sue's Letter", "Controller", "Broken Sprinkler", "Sprinkler", "Tow Rope", "Clay Figure Medal", "Little Man",
    "Mushroom Badge", "Ma Pignon", "Curly's Underwear", "Alien Medal", "Chaco's Lipstick", "Whimsical Star", "Iron Bond",
];

const EQUIPMENT: [&str; 9] =
    ["Booster 0.8", "Map System", "Arms Barrier", "Turbocharge", "Air Tank", "Booster 2.0", "Mimiga Mask", "Whimsical Star", "Nikumaru Counter"];

fn slots(d: &[u8]) -> Vec<usize> {
    match d.len() {
        FREEWARE => vec![0],
        PLUS => (0..3).map(|i| i * SLOT).filter(|&o| d[o..o + 8] == *MAGIC).collect(),
        _ => vec![],
    }
}

fn u32at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

pub fn looks(d: &[u8]) -> bool {
    (d.len() == FREEWARE || d.len() == PLUS) && d.starts_with(MAGIC) && slots(d).iter().all(|&o| &d[o + 0x218..o + 0x21C] == b"FLAG")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Cave Story save".into());
    }
    let plus = d.len() == PLUS;
    let difficulties = [("Normal".to_string(), 0u64), ("Easy".to_string(), 2), ("Hard".to_string(), 4)];
    let mut b = Builder::new(d);
    for o in slots(d) {
        let slot = if plus { format!("Slot {}", o / SLOT + 1) } else { "Profile".to_string() };
        let s = slot.as_str();
        if plus {
            b.pick(&[s, "Difficulty"], o + 0x610, Kind::U8, &difficulties);
        }
        b.ranged(&[s, "Health"], o + 0x20, Kind::U16, 1, 999);
        b.ranged(&[s, "Max health"], o + 0x1C, Kind::U16, 1, 999);
        b.ranged(&[s, "Whimsical stars"], o + 0x1E, Kind::U16, 0, 3);
        b.info(&[s, "Stage number"], u32at(d, o + 0x08));
        let eq = u32at(d, o + 0x2C);
        let worn: Vec<&str> = EQUIPMENT.iter().enumerate().filter(|(i, _)| eq & (1 << i) != 0).map(|(_, n)| *n).collect();
        b.info(&[s, "Equipment"], if worn.is_empty() { "none".to_string() } else { worn.join(", ") });
        let items: Vec<String> = (0..32)
            .map(|i| u32at(d, o + 0xD8 + 4 * i) as usize)
            .filter(|&id| id != 0)
            .map(|id| ITEMS.get(id).map_or_else(|| format!("Item {id}"), |n| n.to_string()))
            .collect();
        b.info(&[s, "Items"], if items.is_empty() { "none".to_string() } else { items.join(", ") });
        for w in 0..8 {
            let at = o + 0x38 + 20 * w;
            let key = format!("Weapon {}", w + 1);
            let p = |f: &'static str| [s, "Weapons", key.as_str(), f];
            b.named(&p("Type"), at, Kind::U32, &WEAPONS);
            b.ranged(&p("Level"), at + 4, Kind::U32, 0, 3);
            b.ranged(&p("Energy"), at + 8, Kind::U32, 0, 100);
            b.ranged(&p("Max ammo (0 = unlimited)"), at + 12, Kind::U32, 0, 999);
            b.ranged(&p("Ammo"), at + 16, Kind::U32, 0, 999);
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Close the game first. Weapons fill the slots from Weapon 1 with no gaps: to add one, pick its type in the first empty slot and set Level to 1.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn profile() -> Vec<u8> {
        let mut p = vec![0u8; SLOT];
        p[..8].copy_from_slice(MAGIC);
        p[0x1C] = 3;
        p[0x20] = 3;
        p[0x2C] = 0x22;
        p[0x38] = 2;
        p[0x3C] = 1;
        p[0xD8] = 2;
        p[0x218..0x21C].copy_from_slice(b"FLAG");
        p
    }

    #[test]
    fn plus_slots_and_freeware() {
        let mut d = vec![0u8; PLUS];
        d[..SLOT].copy_from_slice(&profile());
        d[0x610] = 4;
        d[2 * SLOT..3 * SLOT].copy_from_slice(&profile());
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Slot 1/Difficulty"), Some(&Value::from("Hard")));
        assert!(doc.view.pointer("/Slot 2").is_none());
        assert_eq!(doc.view.pointer("/Slot 3/Weapons/Weapon 1/Type"), Some(&Value::from("Polar Star")));
        assert_eq!(doc.view.pointer("/Slot 1/Equipment"), Some(&Value::from("Map System, Booster 2.0")));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Difficulty", Value::from("Easy")).unwrap();
        doc.set("/Slot 3/Max health", Value::from(50)).unwrap();
        doc.set("/Slot 3/Weapons/Weapon 2/Type", Value::from("Spur")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0x610], 2);
        assert_eq!(out[2 * SLOT + 0x1C], 50);
        assert_eq!(out[2 * SLOT + 0x38 + 20], 13);
        assert!(doc.set("/Slot 3/Health", Value::from(0)).is_err());

        let f = profile()[..FREEWARE].to_vec();
        let doc = parse(&f).unwrap();
        assert_eq!(doc.view.pointer("/Profile/Health"), Some(&Value::from(3)));
        assert!(doc.view.pointer("/Profile/Difficulty").is_none());
    }
}
