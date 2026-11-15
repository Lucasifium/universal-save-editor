//! The Legend of Zelda: Oracle of Ages and Oracle of Seasons (Game Boy Color) battery saves
//! (`.sav`, 8 KiB): three slots of 0x550 bytes. A used slot has the text "Z21216-0" (Ages) or
//! "Z11216-0" (Seasons) at +0x12 and a 16-bit little-endian sum of the words from +0x12 to
//! +0x560 stored at +0x10. Rupees, bombs and seeds are binary-coded decimal; health is kept in
//! quarter hearts.
//!
//! Layout follows the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md); the sums are checked on public saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND_AGES: &str = "Oracle of Ages (Game Boy Color) save";
pub const KIND_SEASONS: &str = "Oracle of Seasons (Game Boy Color) save";
const SLOT: usize = 0x550;
const AGES: &[u8; 8] = b"Z21216-0";
const SEASONS: &[u8; 8] = b"Z11216-0";

fn sum(d: &[u8], s: usize) -> u16 {
    (s + 0x12..s + 0x560).step_by(2).fold(0u16, |a, i| a.wrapping_add(u16::from_le_bytes([d[i], d[i + 1]])))
}

fn valid(d: &[u8], n: usize, magic: &[u8; 8]) -> bool {
    let s = n * SLOT;
    &d[s + 0x12..s + 0x1a] == magic && sum(d, s) == u16::from_le_bytes([d[s + 0x10], d[s + 0x11]])
}

fn game(d: &[u8]) -> Option<bool> {
    if d.len() != 0x2000 {
        return None;
    }
    for (magic, ages) in [(AGES, true), (SEASONS, false)] {
        if (0..3).any(|n| valid(d, n, magic)) {
            return Some(ages);
        }
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    game(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let ages = game(d).ok_or("not an Oracle of Ages / Seasons save")?;
    let magic = if ages { AGES } else { SEASONS };
    let mut b = Builder::new(d);
    let mut found = 0;
    for n in 0..3 {
        if !valid(d, n, magic) {
            continue;
        }
        found += 1;
        let s = n * SLOT;
        let slot = ["Slot 1", "Slot 2", "Slot 3"][n];
        // Offsets differ between the two games up to the shared seed block.
        let (hp, pieces, rupees, bombs, sword, shield, chus, seeds) = if ages {
            (0x10a, 0x10c, 0x10d, 0x110, 0x112, 0x10f, 0x113, 0x119)
        } else {
            (0x102, 0x104, 0x105, 0x10a, 0x10c, 0x109, 0x10d, 0x115)
        };
        b.ranged(&[slot, "Game", "Health (quarter hearts)"], s + hp, Kind::U8, 1, 64);
        b.ranged(&[slot, "Game", "Max health (quarter hearts)"], s + hp + 1, Kind::U8, 4, 64);
        b.ranged(&[slot, "Game", "Heart pieces"], s + pieces, Kind::U8, 0, 3);
        b.ranged(&[slot, "Game", "Rupees"], s + rupees, Kind::Bcd16, 0, 999);
        b.ranged(&[slot, "Game", "Deaths"], s + 0x7e, Kind::U16, 0, 999);
        b.ranged(&[slot, "Items", "Sword level"], s + sword, Kind::U8, 0, 3);
        b.ranged(&[slot, "Items", "Shield level"], s + shield, Kind::U8, 0, 3);
        b.ranged(&[slot, "Items", "Bombs"], s + bombs, Kind::Bcd8, 0, 99);
        b.ranged(&[slot, "Items", "Max bombs"], s + bombs + 1, Kind::Bcd8, 0, 99);
        b.ranged(&[slot, "Items", "Bombchus"], s + chus, Kind::Bcd8, 0, 99);
        for (i, name) in ["Ember seeds", "Scent seeds", "Pegasus seeds", "Gale seeds", "Mystery seeds"].iter().enumerate() {
            b.ranged(&[slot, "Items", name], s + seeds + i, Kind::Bcd8, 0, 99);
        }
        if ages {
            b.ranged(&[slot, "Items", "Bracelet level"], s + 0x118, Kind::U8, 0, 2);
            b.ranged(&[slot, "Items", "Hook level"], s + 0x116, Kind::U8, 0, 2);
        } else {
            b.ranged(&[slot, "Game", "Ore chunks"], s + 0x107, Kind::Bcd16, 0, 999);
            b.ranged(&[slot, "Items", "Boomerang level"], s + 0x111, Kind::U8, 0, 2);
            b.ranged(&[slot, "Items", "Slingshot level"], s + 0x113, Kind::U8, 0, 2);
        }
    }
    b.info(&["Info", "Slots in use"], found);
    let kind = if ages { KIND_AGES } else { KIND_SEASONS };
    Ok(b.finish(kind, Checksum::Custom(fix)).with_note("Close the emulator first. Health counts quarter hearts (4 = one heart). Each slot's sum is rewritten on save."))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..3 {
        let s = n * SLOT;
        if d[s + 0x12..s + 0x1a] == *AGES || d[s + 0x12..s + 0x1a] == *SEASONS {
            let v = sum(d, s);
            d[s + 0x10..s + 0x12].copy_from_slice(&v.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_bcd_values() {
        for (magic, hp, rupees) in [(AGES, 0x10a, 0x10d), (SEASONS, 0x102, 0x105)] {
            let mut d = vec![0u8; 0x2000];
            d[0x12..0x1a].copy_from_slice(magic);
            d[hp] = 12;
            d[hp + 1] = 12;
            d[rupees] = 0x34;
            d[rupees + 1] = 0x01;
            fix(&mut d);
            assert!(valid(&d, 0, magic));
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.view.pointer("/Slot 1/Game/Rupees").and_then(|v| v.as_i64()), Some(134));
            assert_eq!(doc.to_bytes().unwrap(), d);
            doc.set("/Slot 1/Game/Rupees", 999.into()).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(&out[rupees..rupees + 2], &[0x99, 0x09]);
            assert!(valid(&out, 0, magic));
        }
    }
}
