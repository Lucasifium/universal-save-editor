//! Samurai Warriors 2 (the original PC release, `save.dat`, exactly 0x22EB4 bytes): a plain
//! little-endian save with revision 2 at offset 4 and an additive checksum, the sum of every
//! byte before it, in the dword at 0x22E94. Offsets follow the Universal Koei Tecmo Save Editor
//! by Sanrenan (MIT), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Samurai Warriors 2 save";
const SIZE: usize = 0x22EB4;
const CK: usize = 0x22E94;
const NAMES: [&str; 26] = [
    "Yukimura Sanada", "Mitsuhide Akechi", "Nobunaga Oda", "Keiji Maeda", "Kenshin Uesugi", "Oichi", "Okuni", "Magoichi Saika", "Shingen Takeda", "Masamune Date", "Noh",
    "Hanzo Hattori", "Ranmaru Mori", "Hideyoshi Toyotomi", "Ieyasu Tokugawa", "Tadakatsu Honda", "Ina", "Mitsunari Ishida", "Nagamasa Azai", "Sakon Shima", "Yoshihiro Shimazu",
    "Ginchiyo Tachibana", "Kanetsugu Naoe", "Nene", "Kotaro Fuma", "Musashi Miyamoto",
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
    d.len() == SIZE && u16::from_le_bytes([d[4], d[5]]) == 2 && u32::from_le_bytes([d[CK], d[CK + 1], d[CK + 2], d[CK + 3]]) == sum(d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Samurai Warriors 2 save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Army", "Money"], 0x2150, Kind::U32, 0, 99_999_999);
    for (i, name) in NAMES.iter().enumerate() {
        // The ownership bitset at 0x214C says which officers are unlocked.
        if d[0x214C + i / 8] & (1 << (i % 8)) == 0 {
            continue;
        }
        let at = 0xC + i * 0xEC;
        for (k, stat) in ["Life", "Musou", "Attack", "Defense", "Riding", "Speed", "Dexterity", "Luck"].iter().enumerate() {
            b.ranged(&["Officers", name, stat], at + 4 * k, Kind::U32, 0, 999);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the game first. The additive checksum is written again. Only unlocked officers are listed; the stat limits are estimates, not the game's own.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_fixes_sum() {
        let mut d = vec![0u8; SIZE];
        d[4] = 2;
        d[0x214C] = 1;
        d[0xC..0x10].copy_from_slice(&289u32.to_le_bytes());
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Army/Money", 99_999_999.into()).unwrap();
        doc.set("/Officers/Yukimura Sanada/Life", 400.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(u32::from_le_bytes([out[0x2150], out[0x2151], out[0x2152], out[0x2153]]), 99_999_999);
    }
}
