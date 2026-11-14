//! Dynasty Warriors 4 Hyper (the PC release, `save.dat`, exactly 0x10FC0 bytes): a plain
//! little-endian save whose checksum is the sum of every byte before 0x10FA8, stored there as a
//! dword; the last 20 bytes are zero. Offsets follow the Universal Koei Tecmo Save Editor by
//! Sanrenan (MIT) and talkative-platano's DW4 Hyper save editor, checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Dynasty Warriors 4 Hyper save";
const SIZE: usize = 0x10FC0;
const CK: usize = 0x10FA8;
const NAMES: [&str; 42] = [
    "Zhao Yun", "Guan Yu", "Zhang Fei", "Xiahou Dun", "Dian Wei", "Xu Zhu", "Zhou Yu", "Lu Xun", "Taishi Ci", "Diao Chan", "Zhuge Liang", "Cao Cao", "Lu Bu", "Sun Shang Xiang",
    "Liu Bei", "Sun Jian", "Sun Quan", "Dong Zhuo", "Yuan Shao", "Ma Chao", "Huang Zhong", "Xiahou Yuan", "Zhang Liao", "Sima Yi", "Lu Meng", "Gan Ning", "Jiang Wei",
    "Zhang Jiao", "Xu Huang", "Zhang He", "Zhen Ji", "Huang Gai", "Sun Ce", "Wei Yan", "Pang Tong", "Meng Huo", "Zhu Rong", "Da Qiao", "Xiao Qiao", "Cao Ren", "Zhou Tai", "Yue Ying",
];
const ITEMS: [&str; 32] = [
    "Peacock Urn", "Dragon Amulet", "Tiger Amulet", "Tortoise Amulet", "Speed Scroll", "Wing Boots", "Huang's Bow", "Nanman Armor", "Horned Helm", "Cavalry Armor", "Seven Star Sash",
    "Elixir", "Herbal Remedy", "Fire Orb", "Lightning Orb", "Vorpal Orb", "Ice Orb", "Blast Orb", "Poison Orb", "Red Hare Harness", "Hex Mark Harness", "Storm Harness", "Shadow Harness",
    "Elephant Harness", "Art of War", "Survival Guide", "Bodyguard Manual", "Way of Musou", "Power Scroll", "Wind Scroll", "Fire Arrows", "Charge Bracer",
];

fn sum(d: &[u8]) -> u32 {
    d[..CK].iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b)))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() == SIZE {
        let s = sum(d);
        d[CK..CK + 4].copy_from_slice(&s.to_le_bytes());
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && d[CK + 4..].iter().all(|&b| b == 0) && u32::from_le_bytes([d[CK], d[CK + 1], d[CK + 2], d[CK + 3]]) == sum(d) && d[0x9A] <= 2 && (0..42).all(|i| d[0xB8 + i * 24] <= 1)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Dynasty Warriors 4 Hyper save".into());
    }
    let mut b = Builder::new(d);
    b.pick(&["Difficulty"], 0x9A, Kind::U8, &[("Easy".to_string(), 0), ("Normal".to_string(), 1), ("Hard".to_string(), 2)]);
    for (i, name) in NAMES.iter().enumerate() {
        let at = 0xB8 + i * 24;
        b.pick(&["Officers", name, "Playable"], at, Kind::U8, &[("Locked".to_string(), 0), ("Unlocked".to_string(), 1)]);
        for (k, stat) in ["Life", "Musou", "Attack", "Defense"].iter().enumerate() {
            b.ranged(&["Officers", name, stat], at + 1 + k, Kind::U8, 0, 255);
        }
        b.ranged(&["Officers", name, "Character EXP"], at + 16, Kind::U16, 0, 65_535);
        b.ranged(&["Officers", name, "Weapon EXP"], 0x798 + 2 * i, Kind::U16, 0, 36_001);
    }
    for (i, item) in ITEMS.iter().enumerate() {
        // 0xFF is locked; 0 to max - 1 are levels 1 to max. Items from the 20th are owned or not.
        let at = 0x7F6 + i;
        let max = if i < 13 { 20u64 } else if i < 19 { 4 } else { 1 };
        let mut options = vec![("Locked".to_string(), 0xFFu64)];
        if max == 1 {
            options.push(("Owned".to_string(), 0));
        } else {
            options.extend((0..max).map(|l| (format!("Level {}", l + 1), l)));
        }
        b.pick(&["Items", item], at, Kind::U8, &options);
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the game first. The additive checksum is written again. Weapon EXP 36,001 is a special weapon at level 10; items show their level or whether you own them.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_fixes_sum() {
        let mut d = vec![0u8; SIZE];
        d[0xB8] = 1;
        d[0xB8 + 1] = 100;
        d[0x7F6] = 3;
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Officers/Zhao Yun/Life", 255.into()).unwrap();
        doc.set("/Items/Peacock Urn", "Level 20".into()).unwrap();
        doc.set("/Difficulty", "Hard".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(out[0xB8 + 1], 255);
        assert_eq!(out[0x7F6], 19);
        assert_eq!(out[0x9A], 2);
    }
}
