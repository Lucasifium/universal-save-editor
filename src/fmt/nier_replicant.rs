//! NieR Replicant ver.1.22474487139... (`GAMEDATA`, 295,424 bytes): a 33,120-byte header and
//! seven 37,472-byte save slots (0-2 NieR, 3-5 Kaine, 6 the saved ending D). A slot's last
//! checksum is the plain byte sum of its first 3,104 bytes, stored as a u32 at slot offset
//! 37,456.
//!
//! The slot layout (health, magic, level, experience, money, item and material counts) and
//! the item names are as published by nier_replicant (dskrypa, MIT), which credits Acurisu's
//! NieR-Replicant editor for most of the field decoding; the code is a new implementation.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "NieR Replicant save";
const SIZE: usize = 295_424;
const HEAD: usize = 33_120;
const SLOT: usize = 37_472;
const SUM_LEN: usize = 3_104;
const SUM_AT: usize = 37_456;

const RECOVERY: &[(&str, usize)] = &[("Medicinal Herb", 192), ("Health Salve", 193), ("Recovery Potion", 194), ("Strength Drop", 213), ("Strength Capsule", 214), ("Magic Drop", 215), ("Magic Capsule", 216), ("Defense Drop", 217), ("Defense Capsule", 218), ("Spirit Drop", 219), ("Spirit Capsule", 220), ("Antidotal Weed", 223), ("Smelling Salts", 225)];

const MATERIALS: &[(&str, usize)] = &[("Aquatic Plant", 323), ("Deadwood", 324), ("Rusty Bucket", 325), ("Empty Can", 326), ("Gold Ore", 330), ("Silver Ore", 331), ("Copper Ore", 332), ("Iron Ore", 333), ("Crystal", 334), ("Pyrite", 335), ("Moldavite", 336), ("Meteorite", 337), ("Amber", 338), ("Fluorite", 339), ("Clay", 340), ("Berries", 345), ("Royal Fern", 346), ("Tree Branch", 347), ("Log", 348), ("Natural Rubber", 349), ("Ivy", 350), ("Lichen", 351), ("Mushroom", 352), ("Sap", 353), ("Mutton", 359), ("Boar Meat", 360), ("Wool", 361), ("Boar Hide", 362), ("Wolf Hide", 363), ("Wolf Fang", 364), ("Giant Spider Silk", 365), ("Bat Fang", 366), ("Bat Wing", 367), ("Goat Meat", 368), ("Goat Hide", 369), ("Venison", 370), ("Rainbow Spider Silk", 371), ("Boar Liver", 372), ("Scorpion Claw", 373), ("Scorpion Tail", 374), ("Dented Metal Board", 375), ("Stripped Bolt", 376), ("Broken Lens", 377), ("Severed Cable", 378), ("Broken Arm", 379), ("Broken Antenna", 380), ("Broken Motor", 381), ("Broken Battery", 382), ("Mysterious Switch", 383), ("Large Gear", 384), ("Titanium Alloy", 385), ("Memory Alloy", 386), ("Rusted Clump", 387), ("Machine Oil", 388), ("Forlorn Necklace", 393), ("Twisted Ring", 394), ("Broken Earring", 395), ("Pretty Choker", 396), ("Metal Piercing", 397), ("Subdued Bracelet", 398), ("Technical Guide", 399), ("Grubby Book", 400), ("Thick Dictionary", 401), ("Closed Book", 402), ("Used Coloring Book", 403), ("Old Schoolbook", 404), ("Dirty Bag", 405), ("Flashy Hat", 406), ("Leather Gloves", 407), ("Silk Handkerchief", 408), ("Leather Boots", 409), ("Complex Machine", 410), ("Elaborate Machine", 411), ("Simple Machine", 412), ("Stopped Clock", 413), ("Broken Wristwatch", 414), ("Rusty Kitchen Knife", 415), ("Broken Saw", 416), ("Dented Metal Bat", 417), ("Shell", 419), ("Gastropod", 420), ("Bivalve", 421), ("Seaweed", 422), ("Empty Bottle", 423), ("Driftwood", 424), ("Pearl", 425), ("Black Pearl", 426), ("Crab", 427), ("Starfish", 428), ("Sea Turtle Egg", 434), ("Broken Pottery", 435), ("Desert Rose", 436), ("Giant Egg", 437), ("Damascus Steel", 438), ("Eagle Egg", 439), ("Chicken Egg", 440), ("Mouse Tail", 442), ("Lizard Tail", 443), ("Deer Antler", 447)];

fn sum_ok(d: &[u8], slot: usize) -> bool {
    let b = HEAD + slot * SLOT;
    let s: u32 = d[b..b + SUM_LEN].iter().map(|&x| u32::from(x)).sum();
    d[b + SUM_AT..b + SUM_AT + 4] == s.to_le_bytes()
}

fn rd(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && (0..7).all(|i| rd(d, HEAD + i * SLOT) == 200 && sum_ok(d, i))
}

/// Writes the checksum of every slot that was touched.
fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    for slot in 0..7 {
        let b = HEAD + slot * SLOT;
        if touched.iter().any(|&t| t >= b && t < b + SLOT) {
            let s: u32 = d[b..b + SUM_LEN].iter().map(|&x| u32::from(x)).sum();
            d[b + SUM_AT..b + SUM_AT + 4].copy_from_slice(&s.to_le_bytes());
        }
    }
}

const SLOTS: [&str; 7] = ["NieR 1", "NieR 2", "NieR 3", "Kaine 1", "Kaine 2", "Kaine 3", "NieR (after ending D)"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a NieR Replicant save".into());
    }
    let mut b = Builder::new(d);
    for (i, name) in SLOTS.iter().enumerate() {
        let o = HEAD + i * SLOT;
        let used = rd(d, o + 112) > 0 || rd(d, o + 188) > 0 || rd(d, o + 100) > 0;
        if !used {
            continue;
        }
        b.ranged(&[name, "Health"], o + 76, Kind::I32, 1, 99_999);
        b.ranged(&[name, "Health (Kaine)"], o + 80, Kind::I32, 1, 99_999);
        b.ranged(&[name, "Health (Emil)"], o + 84, Kind::I32, 1, 99_999);
        b.ranged(&[name, "Magic"], o + 88, Kind::F32, 0, 999);
        b.ranged(&[name, "Level (shown level minus one)"], o + 100, Kind::I32, 0, 98);
        b.ranged(&[name, "Experience"], o + 112, Kind::I32, 0, 99_999_999);
        b.ranged(&[name, "Money"], o + 188, Kind::I32, 0, 999_999_999);
        for (item, at) in RECOVERY {
            b.ranged(&[name, "Recovery items", item], o + at, Kind::U8, 0, 99);
        }
        for (item, at) in MATERIALS {
            b.ranged(&[name, "Materials", item], o + at, Kind::U8, 0, 99);
        }
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note("Close the game first. Slot checksums are written again on save. Level is stored one lower than the level the game shows."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        for i in 0..7 {
            let b = HEAD + i * SLOT;
            d[b..b + 4].copy_from_slice(&200u32.to_le_bytes());
            d[b + 188..b + 192].copy_from_slice(&1000u32.to_le_bytes());
            let s: u32 = d[b..b + SUM_LEN].iter().map(|&x| u32::from(x)).sum();
            d[b + SUM_AT..b + SUM_AT + 4].copy_from_slice(&s.to_le_bytes());
        }
        d
    }

    #[test]
    fn edit_keeps_the_checksum_right() {
        let d = synthetic();
        assert!(looks(&d));
        let mut t = parse(&d).unwrap();
        t.set("/NieR 1/Money", serde_json::Value::from(5000)).unwrap();
        t.set("/NieR 1/Materials/Aquatic Plant", serde_json::Value::from(99)).unwrap();
        let out = t.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(rd(&out, HEAD + 188), 5000);
    }
}
