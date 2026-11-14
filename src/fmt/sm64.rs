//! Super Mario 64 (Nintendo 64) EEPROM saves (`.eep`, 512 or 2048 bytes): four files of two
//! identical 0x38-byte copies each, then the menu's own record at 0x1c0. Each copy ends in a
//! 16-bit big-endian sum of its bytes; the menu record has the same kind of sum at 0x1de.
//!
//! Layout after the SM64 decompilation project's `SaveFile` and MIT-licensed save editors
//! (MaikelChan/SM64SaveEditor, RyudoSynbios/game-tools-collection; see THIRD_PARTY_NOTICES.md).
//! Each course keeps its star bits in one byte (bits 0-6) and its cannon bit in the next one.

use super::fields::{Builder, Checksum, Kind};
use super::retro::{byte_sum, flag};
use crate::tree::TreeDoc;

pub const KIND: &str = "Super Mario 64 save";
const SLOT: usize = 0x70;
const COPY: usize = 0x38;
const MENU: usize = 0x1c0;
const FILES: [&str; 4] = ["File A", "File B", "File C", "File D"];

pub fn looks(d: &[u8]) -> bool {
    (d.len() == 512 || d.len() == 2048) && d[0x34..0x36] == *b"DA" && d[MENU + 0x1c..MENU + 0x1e] == *b"HI"
}

/// Course names in save order (star byte 0xc + n).
const COURSES: [&str; 15] = [
    "Bob-omb Battlefield", "Whomp's Fortress", "Jolly Roger Bay", "Cool, Cool Mountain", "Big Boo's Haunt", "Hazy Maze Cave",
    "Lethal Lava Land", "Shifting Sand Land", "Dire, Dire Docks", "Snowman's Land", "Wet-Dry World", "Tall, Tall Mountain",
    "Tiny-Huge Island", "Tick Tock Clock", "Rainbow Ride",
];
/// Courses whose cannon bit is kept (course number, 1 based).
const CANNONS: [usize; 10] = [1, 2, 3, 4, 8, 10, 11, 12, 13, 15];
const SECRETS: [(&str, usize); 8] = [
    ("Bowser in the Dark World (red coins)", 0x1b), ("Bowser in the Fire Sea (red coins)", 0x1c), ("Bowser in the Sky (red coins)", 0x1d),
    ("Cavern of the Metal Cap", 0x1f), ("Tower of the Wing Cap", 0x20), ("Vanish Cap Under the Moat", 0x21),
    ("Wing Mario Over the Rainbow", 0x22), ("The Secret Aquarium", 0x23),
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Super Mario 64 save".into());
    }
    let mut b = Builder::new(d);
    b.big_endian = true;
    let mut found = 0;
    for (n, file) in FILES.iter().enumerate() {
        let s = n * SLOT;
        if d.len() < s + SLOT || d[s + 0xb] & 1 == 0 || d[s + 0x34..s + 0x36] != *b"DA" {
            continue;
        }
        found += 1;
        let at = |b: &mut Builder, path: &[&str], off: usize, bit: u32| {
            flag(b, path, s + off, bit);
            b.mirror(path, s + COPY + off);
        };
        let mut stars = 0;
        for (c, _) in COURSES.iter().enumerate() {
            stars += (d[s + 0xc + c] & 0x7f).count_ones();
        }
        for (_, off) in SECRETS {
            stars += (d[s + off] & 1) as u32;
        }
        stars += ((d[s + 0x1e] >> 1) & 1) as u32;
        b.info(&[file, "Stars collected"], stars);
        for (name, bit) in [("Wing Cap switch", 1), ("Metal Cap switch", 2), ("Vanish Cap switch", 3), ("Basement key", 4), ("Gallery key", 5), ("Basement key door open", 6), ("Gallery key door open", 7)] {
            at(&mut b, &[file, name], 0xb, bit);
        }
        at(&mut b, &[file, "Moat drained"], 0xa, 1);
        for (name, bit) in [("Toad (Hazy Maze Cave)", 0), ("Toad (Gallery)", 1), ("Toad (Clock Room)", 2), ("Mips (first star)", 3), ("Mips (second star)", 4)] {
            at(&mut b, &[file, name], 8, bit);
        }
        at(&mut b, &[file, "Secret slide star"], 0x1e, 0);
        at(&mut b, &[file, "Secret slide star (under 21 s)"], 0x1e, 1);
        for (name, off) in SECRETS {
            at(&mut b, &[file, name], off, 0);
        }
        for (c, course) in COURSES.iter().enumerate() {
            for bit in 0..7 {
                let label = if bit == 6 { "100 coins".to_string() } else { format!("Star {}", bit + 1) };
                at(&mut b, &[file, "Courses", course, &label], 0xc + c, bit);
            }
            if CANNONS.contains(&(c + 1)) {
                at(&mut b, &[file, "Courses", course, "Cannon open"], 0xd + c, 7);
            }
            let p = [*file, "Courses", course, "Coin record"];
            b.ranged(&p, s + 0x25 + c, Kind::U8, 0, 255);
            b.mirror(&p, s + COPY + 0x25 + c);
        }
    }
    b.info(&["Info", "Files in use"], found);
    if found == 0 {
        return Err("no Super Mario 64 file in this save has been started".into());
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the emulator first. Star bits are kept per course; a course's six stars and its 100-coin star are separate bits."))
}

fn fix(d: &mut Vec<u8>) {
    for n in 0..4 {
        for copy in 0..2 {
            let s = n * SLOT + copy * COPY;
            if s + COPY <= d.len() && d[s + 0x34..s + 0x36] == *b"DA" {
                let sum = byte_sum(d, s, s + 0x36) as u16;
                d[s + 0x36..s + 0x38].copy_from_slice(&sum.to_be_bytes());
            }
        }
    }
    let sum = byte_sum(d, MENU, MENU + 0x1e) as u16;
    d[MENU + 0x1e..MENU + 0x20].copy_from_slice(&sum.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 512];
        d[0xb] = 1;
        d[0xc] = 0x20;
        for s in [0, COPY] {
            d[s + 0xb] = 1;
            d[s + 0xc] = 0x20;
            d[s + 0x34..s + 0x36].copy_from_slice(b"DA");
        }
        d[MENU + 0x1c..MENU + 0x1e].copy_from_slice(b"HI");
        fix(&mut d);
        d
    }

    #[test]
    fn edits_both_copies_and_checksums() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/File A/Courses/Bob-omb Battlefield/Star 6").unwrap(), "Yes");
        doc.set("/File A/Courses/Whomp's Fortress/Star 1", "Yes".into()).unwrap();
        doc.set("/File A/Courses/Bob-omb Battlefield/Coin record", 120.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0xd] & 1, 1);
        assert_eq!(out[COPY + 0xd] & 1, 1);
        assert_eq!(out[0x25], 120);
        assert_eq!(out[COPY + 0x25], 120);
        let sum = byte_sum(&out, 0, 0x36) as u16;
        assert_eq!(u16::from_be_bytes([out[0x36], out[0x37]]), sum);
        assert_eq!(u16::from_be_bytes([out[COPY + 0x36], out[COPY + 0x37]]), sum);
        assert!(parse(&out).is_ok());
    }
}
