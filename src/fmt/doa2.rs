//! Dead or Alive 2 (PlayStation 2) saves: the 6,521-byte save file of the save folder. Its last byte is
//! the XOR of the bytes before it. Offsets and names follow the MIT-licensed save editor in
//! RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Dead or Alive 2 (PlayStation 2) save";

const LEN: usize = 6521;

fn xor(d: &[u8]) -> u8 {
    d[..LEN - 1].iter().fold(0, |a, b| a ^ b)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == LEN && d[LEN - 1] == xor(d) && d.iter().any(|b| *b != 0) && d[0x1816] != 0xff
}

/// (name, number of costumes) in the game's own order; the costume byte of the fighter is at
/// 0x932 + index, the story completion byte at 0x1808 + index, the "completed once" flag at 0x940 + index.
const FIGHTERS: [(&str, usize); 14] = [
    ("Zack", 6), ("Tina Armstrong", 7), ("Jann Lee", 6), ("Ein", 5), ("Ryu Hayabusa", 6), ("Kasumi", 8), ("Gen Fu", 6),
    ("Helena", 7), ("Leon", 5), ("Bass Armstrong", 5), ("Leifang", 8), ("Ayane", 8), ("Tengu", 2), ("Bayman", 4),
];
const COLLECTION: [&str; 50] = ["Ice Lolly", "Soft Ice Cream", "Ice Cream (Double)", "Egg", "Chick", "Chicken", "French Fries", "Shake", "Hamburger", "Shuriken", "Kunai", "Katana", "Sushi (Kappa Maki)", "Sushi (Tamago)", "Sushi (Toro)", "Strawberry", "Rice Ball", "Cherry Blossom", "Mahjong Tile (Red)", "Mahjong Tile (Green)", "Mahjong Tile (White)", "Tropical Juice 1", "Tropical Juice 2", "Tropical Juice 3", "Knife", "Gun", "Tank", "Meat", "Mixed Juice", "The Stars and Stripes", "Pancake", "Chocolate Cake", "Shortcake", "Pudding", "Ribbon", "Ring", "Three", "Mountain", "Mt.Fuji", "Bayman's Knife", "Bayman's Gun", "Bayman's Tank", "Lipstick", "Carrot", "Money", "Melon", "False Suezo", "Suezo", "Star", "Medal"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Dead or Alive 2 save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["General", "Points"], 0x1816, Kind::U8, 0, 255);
    b.ranged(&["General", "Total games played"], 0x1817, Kind::U8, 0, 255);
    b.ranged(&["General", "Play time (seconds)"], 0x92c, Kind::U32, 0, 359_999);
    b.ranged(&["General", "Story mode plays"], 0xaf4, Kind::U32, 0, 999_999);
    b.ranged(&["General", "Time attack mode plays"], 0xb04, Kind::U32, 0, 999_999);
    b.ranged(&["General", "Survival mode plays"], 0xb0c, Kind::U32, 0, 999_999);
    b.ranged(&["General", "Tag battle mode plays"], 0xafc, Kind::U32, 0, 999_999);
    flag(&mut b, &["Unlocks", "Tengu"], 0x1805, 0);
    flag(&mut b, &["Unlocks", "Bayman"], 0x1806, 0);
    flag(&mut b, &["Unlocks", "CG Gallery menu"], 0x1807, 0);
    flag(&mut b, &["Unlocks", "Collection menu"], 0x1818, 0);
    for (i, (n, costumes)) in FIGHTERS.iter().enumerate() {
        b.ranged(&["Fighters", n, "Costumes unlocked"], 0x932 + i, Kind::U8, 1, *costumes as i64);
        b.ranged(&["Fighters", n, "Story completions"], 0x1808 + i, Kind::U8, 0, 255);
    }
    for (i, n) in COLLECTION.iter().enumerate() {
        b.ranged(&["Collection", n], 0x182a + i * 2, Kind::U16, 0, 9999);
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note(crate::i18n::tx("Close the emulator first. A fighter's story is counted as completed once it has been finished at least once; the game's own list of obtained collection items and the save's checksum are written again.")))
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    for t in touched {
        if (0x1808..0x1808 + 14).contains(t) {
            let done = d[*t] > 0;
            let at = 0x940 + (*t - 0x1808);
            d[at] = d[at] & !1 | done as u8;
        }
        if (0x182a..0x182a + 100).contains(t) {
            let i = (*t - 0x182a) / 2;
            let have = u16::from_le_bytes([d[0x182a + i * 2], d[0x182a + i * 2 + 1]]) > 0;
            let at = 0x181a + i / 8;
            d[at] = d[at] & !(1 << (i % 8)) | (have as u8) << (i % 8);
        }
    }
    let x = xor(d);
    d[LEN - 1] = x;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_flags_and_checksum() {
        let mut d = vec![0u8; LEN];
        d[0x1816] = 5;
        d[LEN - 1] = xor(&d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Collection/Egg", serde_json::json!(3)).unwrap();
        doc.set("/Fighters/Kasumi/Story completions", serde_json::json!(2)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0x182a + 3 * 2], 3);
        assert_eq!(out[0x181a] >> 3 & 1, 1);
        assert_eq!(out[0x1808 + 5], 2);
        assert_eq!(out[0x940 + 5] & 1, 1);
        assert_eq!(out[LEN - 1], xor(&out));
    }
}
