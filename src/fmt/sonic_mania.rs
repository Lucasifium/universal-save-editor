//! Sonic Mania (PC): `SaveData.bin`, 65,536 bytes. Eight Mania Mode slots from 0x0000, the
//! Extras block at 0x2400 and three Encore Mode slots from 0x2800, each slot 1,024 bytes.
//! Nothing is checksummed.
//!
//! Slot layout (facts from the public MIT-licensed save format notes of the Sonic Mania
//! save editor by jcfieldsdev): state 0x58, character 0x5C, stage 0x60, lives 0x64, score
//! (u32) 0x68, Chaos Emeralds bit field 0x70, continues 0x74, next special stage 0x7C.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Sonic Mania save";
const SIZE: usize = 65_536;
const SLOTS: [(&str, usize); 11] = [
    ("Mania slot 1", 0x0000), ("Mania slot 2", 0x0400), ("Mania slot 3", 0x0800), ("Mania slot 4", 0x0C00),
    ("Mania slot 5", 0x1000), ("Mania slot 6", 0x1400), ("Mania slot 7", 0x1800), ("Mania slot 8", 0x1C00),
    ("Encore slot 1", 0x2800), ("Encore slot 2", 0x2C00), ("Encore slot 3", 0x3000),
];
const STATES: [&str; 3] = ["New", "In progress", "Cleared"];
const CHARACTERS: [&str; 7] = ["Sonic & Tails", "Sonic", "Tails", "Knuckles", "Mighty", "Ray", "Encore team"];
const STAGES: [&str; 13] = [
    "Green Hill", "Chemical Plant", "Studiopolis", "Flying Battery", "Press Garden", "Stardust Speedway", "Hydrocity",
    "Mirage Saloon", "Oil Ocean", "Lava Reef", "Metallic Madness", "Titanic Monarch", "Egg Reverie (cleared)",
];

/// 64 KiB with the unused blocks empty and every slot's header bytes and values in range.
pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE
        && d[0x2000..0x2400].iter().all(|&b| b == 0)
        && d[0x3400..].iter().all(|&b| b == 0)
        && SLOTS.iter().all(|&(_, o)| d[o..o + 0x58].iter().all(|&b| b == 0) && d[o + 0x58] <= 2 && d[o + 0x60] <= 12 && d[o + 0x64] <= 99 && d[o + 0x5C] <= 6)
        && SLOTS.iter().any(|&(_, o)| d[o + 0x58] != 0)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Sonic Mania save".into());
    }
    let mut b = Builder::new(d);
    for (name, o) in SLOTS {
        b.named(&[name, "State"], o + 0x58, Kind::U8, &STATES);
        b.named(&[name, "Character"], o + 0x5C, Kind::U8, &CHARACTERS);
        b.named(&[name, "Zone"], o + 0x60, Kind::U8, &STAGES);
        b.ranged(&[name, "Lives"], o + 0x64, Kind::U8, 0, 99);
        b.ranged(&[name, "Score"], o + 0x68, Kind::U32, 0, 9_999_999);
        b.ranged(&[name, "Chaos Emeralds (127 = all seven)"], o + 0x70, Kind::U8, 0, 127);
        b.ranged(&[name, "Continues"], o + 0x74, Kind::U8, 0, 99);
        b.ranged(&[name, "Next special stage"], o + 0x7C, Kind::U8, 0, 6);
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. The Chaos Emerald number is a bit field: Green 1, Yellow 2, Purple 4, Pink 8, Grey 16, Blue 32, Red 64 (127 = all). Play times and the extras are not shown."))
}
