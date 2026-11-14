//! Crash Bandicoot (PlayStation 1, the 1996 game) saves: one 8 KiB block per save. Offsets
//! follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md). The save keeps a checksum at 0x17c of its game data and shows the
//! completion rate in the save's title.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{extra_frames, is_save, title};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Crash Bandicoot save";

/// Offset of the game data in the block: after the header and the icon frames.
fn base(d: &[u8]) -> usize {
    extra_frames(d) * 0x80
}

fn sum(d: &[u8], s: usize) -> u32 {
    let mut c: u32 = 0x1234_5678;
    for (i, &x) in d[s + 0x100..s + 0x180].iter().enumerate() {
        let x = if (0x7c..0x80).contains(&i) { 0 } else { x };
        c = c.wrapping_add(x as u32).rotate_left(3);
    }
    c
}

pub fn looks(d: &[u8]) -> bool {
    if !is_save(d, 1) || !title(d).starts_with("Crash") {
        return false;
    }
    let s = base(d);
    d.len() >= s + 0x180 && u32::from_le_bytes(d[s + 0x17c..s + 0x180].try_into().unwrap()) == sum(d, s)
}

const LEVELS: [&str; 31] = [
    "N. Sanity Beach", "Jungle Rollers", "The Great Gate", "Boulders", "Upstream", "Papu Papu", "Rolling Stones", "Hog Wild", "Native Fortress",
    "Up the Creek", "Ripper Roo", "The Lost City", "Temple Ruins", "Road to Nowhere", "Boulder Dash", "Sunset Vista", "Koala Kong",
    "Heavy Machinery", "Cortex Power", "Generator Room", "Toxic Waste", "Pinstripe", "The High Road", "Slippery Climb", "Lights Out",
    "Jaws of Darkness", "Castle Machinery", "Nitrus Brio", "The Lab", "The Great Hall", "Dr. Neo Cortex",
];

/// Offset (relative to the data) and bit of the gem of each level, then of the two bonus gems.
fn gem(i: usize) -> (usize, u32) {
    match i {
        0..=30 => (0x11c + (i + 1) / 8, ((i + 1) % 8) as u32),
        31 => (0x122, 2),
        _ => (0x121, 0),
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Crash Bandicoot save".into());
    }
    let s = base(d);
    let mut b = Builder::new(d);
    b.info(&["Info", "Save title"], title(d));
    b.ranged(&["General", "Level reached (1 = just started)"], s + 0x104, Kind::U32, 1, 30);
    for (i, n) in LEVELS.iter().enumerate().take(30) {
        // Bosses and the last level have no gem.
        if [5, 10, 16, 21, 27, 29].contains(&i) {
            continue;
        }
        let (o, bit) = gem(i);
        let name = format!("{n} gem");
        flag(&mut b, &["Gems", &name], s + o, bit);
    }
    flag(&mut b, &["Gems", "Whole Dog gem"], s + 0x122, 2);
    flag(&mut b, &["Gems", "Fumbling in the Dark gem"], s + 0x121, 0);
    flag(&mut b, &["Keys", "Whole Dog key"], s + 0x122, 4);
    flag(&mut b, &["Keys", "Fumbling in the Dark key"], s + 0x121, 2);
    b.ranged(&["Options", "Effects volume"], s + 0x114, Kind::U32, 0, 255);
    b.ranged(&["Options", "Music volume"], s + 0x118, Kind::U32, 0, 255);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tx("Close the emulator first. The level reached is the number of the level you can play next (the game counts finished levels from it). The gem and key counts, the completion rate in the save's title and the checksum are written again.")))
}

fn fix(d: &mut Vec<u8>) {
    if d.len() < 0x2000 {
        return;
    }
    let s = base(d);
    let bit = |d: &[u8], i: usize| {
        let (o, b) = gem(i);
        (d[s + o] >> b) & 1
    };
    let gems: u32 = (0..32).map(|i| bit(d, i) as u32).sum();
    let keys = ((d[s + 0x122] >> 4) & 1) as u32 + ((d[s + 0x121] >> 2) & 1) as u32;
    let level = u32::from_le_bytes(d[s + 0x104..s + 0x108].try_into().unwrap()).min(31);
    // The preview in the save list: level (5 bits), gems (5 bits), keys (2 bits).
    let p = u16::from_le_bytes([d[s + 0x100], d[s + 0x101]]);
    let p = (p & !0x03ff) | level as u16 | ((gems.min(31) as u16) << 5);
    let p = (p & !0x0c00) | ((keys as u16) << 10);
    d[s + 0x100..s + 0x102].copy_from_slice(&p.to_le_bytes());
    // The completion rate in the title: whole percent of 88 points.
    let pct = ((level.saturating_sub(1)).min(28) * 2 + gems + keys * 3) * 100 / 88;
    let mut w: Vec<u16> = pct.to_string().bytes().map(|c| 0x824f + (c - b'0') as u16).collect();
    w.extend([0x8193, 0x816a, 0]);
    for (i, x) in w.iter().enumerate() {
        d[0x12 + 2 * i..0x14 + 2 * i].copy_from_slice(&x.to_be_bytes());
    }
    let c = sum(d, s);
    d[s + 0x17c..s + 0x180].copy_from_slice(&c.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_and_fixes_checksum() {
        let mut d = vec![0u8; 0x2000];
        d[..2].copy_from_slice(b"SC");
        d[2] = 0x11;
        // "Crash" in full-width letters.
        for (i, c) in "Crash".chars().enumerate() {
            let w = if c == 'C' { 0x8262u16 } else { 0x8281 + (c as u16 - 'a' as u16) };
            d[4 + 2 * i..6 + 2 * i].copy_from_slice(&w.to_be_bytes());
        }
        d[0x104] = 2;
        let c = sum(&d, 0);
        d[0x17c..0x180].copy_from_slice(&c.to_le_bytes());
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Gems/Boulders gem", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(out[0x11c + 0] & 0x10, 0x10);
        assert_eq!(u16::from_le_bytes([out[0x100], out[0x101]]) >> 5 & 31, 1);
    }
}
