//! Three small Nintendo saves with fixed offsets, after marcrobledo/savegame-editors (MIT, see
//! THIRD_PARTY_NOTICES.md): Super Smash Bros. Ultimate `system_data.bin` (Switch, 5,982,968 bytes),
//! Mario Kart 7 `system0.dat` (3DS, 20,692 bytes, CRC-32 in the last four bytes) and Super Kirby
//! Clash `savedata.dat` (3DS, 54,344 bytes).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const SMASH_KIND: &str = "Super Smash Bros. Ultimate save";
pub const MK7_KIND: &str = "Mario Kart 7 save";
pub const SKC_KIND: &str = "Super Kirby Clash save";
const SMASH_SIZE: usize = 5_982_968;
const MK7_SIZE: usize = 20_692;
const SKC_SIZE: usize = 54_344;

pub fn smash_looks(d: &[u8]) -> bool {
    d.len() == SMASH_SIZE
}
pub fn mk7_looks(d: &[u8]) -> bool {
    d.len() == MK7_SIZE && crc32(&d[..MK7_SIZE - 4]).to_le_bytes() == d[MK7_SIZE - 4..]
}
pub fn skc_looks(d: &[u8]) -> bool {
    d.len() == SKC_SIZE && d[0x3ef0..0x3ef4] != [0xff; 4]
}

fn crc32(d: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in d {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    !crc
}

pub fn parse_smash(d: &[u8]) -> Result<TreeDoc, String> {
    if !smash_looks(d) {
        return Err("not a Super Smash Bros. Ultimate system_data.bin".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Main", "Tickets"], 0x5506cc, Kind::U8, 0, 99);
    b.ranged(&["Main", "Gold"], 0x5506dc, Kind::U32, 0, 999_999);
    b.ranged(&["Main", "Hammers"], 0x555e5c, Kind::U8, 0, 9);
    b.ranged(&["Spirits", "Snacks (S)"], 0x4831ce, Kind::U16, 0, 999);
    b.ranged(&["Spirits", "Snacks (M)"], 0x4831d0, Kind::U16, 0, 999);
    b.ranged(&["Spirits", "Snacks (L)"], 0x4831d2, Kind::U16, 0, 999);
    b.ranged(&["Spirits", "SP"], 0x4831e4, Kind::U32, 0, 999_999);
    let items = [
        ("Shuffle all", 0x4831c0), ("All primaries", 0x4831c1), ("All supports", 0x4831c2), ("Filler", 0x4831c3), ("Rematch", 0x4831c4),
        ("50% damage", 0x4831c6), ("Slow FS charging", 0x4831c7), ("Weaken minions", 0x4831c8), ("Health drain", 0x4831c9),
        ("Disable items", 0x4831ca), ("Shield spacer", 0x4831cb), ("Sluggish shield", 0x4831cc),
    ];
    for (name, at) in items {
        b.ranged(&["Spirit board items", name], at, Kind::U8, 0, 99);
    }
    Ok(b.finish(SMASH_KIND, Checksum::None).with_note("Close the emulator first. The save is the decrypted system_data.bin."))
}

pub fn parse_mk7(d: &[u8]) -> Result<TreeDoc, String> {
    if !mk7_looks(d) {
        return Err("not a Mario Kart 7 system0.dat".into());
    }
    let mut b = Builder::new(d);
    let name: String = d[0x22..0x36].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).filter_map(|u| char::from_u32(u.into())).collect();
    b.info(&["Player", "Name"], name);
    b.bits(&["Player", "Coins"], 0x4c28, Kind::U32, 0, 24, 0, 0, 99_999);
    b.bits(&["Player", "StreetPass"], 0x4e24, Kind::U32, 0, 24, 0, 0, 99_999);
    b.ranged(&["Player", "Online wins"], 0x4e34, Kind::U16, 0, 99_999);
    b.ranged(&["Player", "Online losses"], 0x4e38, Kind::U16, 0, 99_999);
    b.ranged(&["Player", "VR points"], 0x4e3c, Kind::U16, 0, 99_999);
    Ok(b.finish(MK7_KIND, Checksum::Custom(fix_mk7)).with_note("Close the emulator first. The save is the decrypted system0.dat."))
}

fn fix_mk7(d: &mut Vec<u8>) {
    if d.len() == MK7_SIZE {
        let crc = crc32(&d[..MK7_SIZE - 4]);
        d[MK7_SIZE - 4..].copy_from_slice(&crc.to_le_bytes());
    }
}

pub fn parse_skc(d: &[u8]) -> Result<TreeDoc, String> {
    if !skc_looks(d) {
        return Err("not a Super Kirby Clash savedata.dat".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Resources", "Apple gems"], 0x3ef0, Kind::U32, 0, 99_999);
    b.ranged(&["Resources", "Apple gems bought"], 0x3f00, Kind::U16, 0, 5000);
    b.ranged(&["Resources", "Red shards"], 0x3f20, Kind::U16, 0, 9999);
    b.ranged(&["Resources", "Blue shards"], 0x3f24, Kind::U16, 0, 9999);
    b.ranged(&["Resources", "Yellow shards"], 0x3f28, Kind::U16, 0, 9999);
    b.ranged(&["Resources", "Rare shards"], 0x3f2c, Kind::U16, 0, 9999);
    Ok(b.finish(SKC_KIND, Checksum::None).with_note("Close the emulator first. The save is the decrypted savedata.dat."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mk7_keeps_its_crc() {
        let mut d = vec![0u8; MK7_SIZE];
        d[0x4c28] = 100;
        fix_mk7(&mut d);
        assert!(mk7_looks(&d));
        let mut doc = parse_mk7(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Player/Coins", 20000.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes([out[0x4c28], out[0x4c29], out[0x4c2a], 0]), 20000);
        assert!(mk7_looks(&out));
    }

    #[test]
    fn smash_and_kirby_edit_in_place() {
        let mut d = vec![0u8; SMASH_SIZE];
        d[0x5506dc] = 5;
        let mut doc = parse_smash(&d).unwrap();
        doc.set("/Main/Gold", 999_999.into()).unwrap();
        doc.set("/Main/Hammers", 9.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes([out[0x5506dc], out[0x5506dd], out[0x5506de], out[0x5506df]]), 999_999);
        let d = vec![0u8; SKC_SIZE];
        let mut doc = parse_skc(&d).unwrap();
        doc.set("/Resources/Red shards", 9999.into()).unwrap();
        assert_eq!(u16::from_le_bytes([doc.to_bytes().unwrap()[0x3f20], doc.to_bytes().unwrap()[0x3f21]]), 9999);
    }
}
