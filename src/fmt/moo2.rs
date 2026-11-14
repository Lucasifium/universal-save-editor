//! Master of Orion II saves (`SAVE1.GAM` to `SAVE10.GAM`, 208000 bytes). The file is the game's
//! memory written block by block with no headers, so every value sits at a fixed offset. The
//! layout is from the OpenOrion2 project's format notes (GPL-3.0), checked on public saves.
//! Values are edited in place and the length never changes; no checksum is kept.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Master of Orion II save";

const SIZE: usize = 208_000;
const NUM_COLONIES: usize = 0x25b;
const COLONY: usize = 0x25d;
const COLONY_LEN: usize = 361;
const PLANET: usize = 0x162e9;
const PLANET_LEN: usize = 17;
const STAR: usize = 0x17ad3;
const STAR_LEN: usize = 113;
const NUM_STARS: usize = 0x17ad1;
const NUM_PLAYERS: usize = 0x1aa0c;
const PLAYER: usize = 0x1aa0e;
const PLAYER_LEN: usize = 3753;

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && d[..4] == [0xE0, 0, 0, 0] && u16le(d, NUM_PLAYERS).is_some_and(|n| (1..=8).contains(&n))
}

fn u16le(d: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn text(d: &[u8], at: usize, len: usize) -> String {
    let raw = &d[at..at + len];
    let end = raw.iter().position(|&c| c == 0).unwrap_or(len);
    raw[..end].iter().map(|&c| if (0x20..0x7f).contains(&c) { c as char } else { '?' }).collect()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Master of Orion II save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Game", "Save name"], text(d, 4, 37));
    b.info(&["Game", "Stardate"], format!("{:.1}", i32::from_le_bytes(d[0x29..0x2d].try_into().unwrap()) as f64 / 10.0));
    let players = u16le(d, NUM_PLAYERS).unwrap_or(0);
    let mut names: Vec<String> = Vec::new();
    for i in 0..players {
        let at = PLAYER + i * PLAYER_LEN;
        let emperor = text(d, at + 1, 20);
        let race = text(d, at + 0x15, 15);
        let human = d[at + 0x28] == 100;
        let row = format!("{}. {} ({}{})", i + 1, emperor, race, if human { ", you" } else { "" });
        names.push(row.clone());
        let row = row.as_str();
        b.ranged(&["Empires", row, "BC reserve"], at + 0x32, Kind::I32, -2_000_000_000, 2_000_000_000);
        b.ranged(&["Empires", row, "Research points"], at + 0x1eb, Kind::I32, 0, 2_000_000_000);
        b.ranged(&["Empires", row, "Fuel range (parsecs)"], at + 0x324, Kind::I16, 0, 99);
    }
    let colonies = u16le(d, NUM_COLONIES).unwrap_or(0).min(250);
    let stars = u16le(d, NUM_STARS).unwrap_or(0).min(72);
    for i in 0..colonies {
        let at = COLONY + i * COLONY_LEN;
        let owner = d[at] as i8;
        let planet = i16::from_le_bytes([d[at + 2], d[at + 3]]);
        if owner < 0 || owner as usize >= players || planet < 0 || planet as usize >= 360 || d[at + 6] == 1 {
            continue; // an empty slot or an outpost
        }
        let p = PLANET + planet as usize * PLANET_LEN;
        let star = d[p + 2] as usize;
        let orbit = d[p + 3] as usize + 1;
        let star_name = if star < stars { text(d, STAR + star * STAR_LEN, 15) } else { String::from("?") };
        let row = format!("{star_name} {orbit} ({})", names.get(owner as usize).and_then(|n| n.split(' ').next()).unwrap_or("?").trim_end_matches('.'));
        let row = row.as_str();
        b.ranged(&["Colonies", row, "Production stored"], at + 0x125, Kind::I16, 0, 32_000);
        b.ranged(&["Colonies", row, "Marines"], at + 0x130, Kind::I16, 0, 99);
        b.ranged(&["Colonies", row, "Tanks"], at + 0x132, Kind::I16, 0, 99);
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Edit a copy of a SAVEn.GAM, then load it in the game. The treasury and research points are the values the game itself keeps."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(d: &mut [u8], at: usize, bytes: &[u8]) {
        d[at..at + bytes.len()].copy_from_slice(bytes);
    }

    /// A save with two empires, one star with one colonised planet and one outpost.
    fn fake() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        put(&mut d, 0, &[0xE0, 0, 0, 0]);
        put(&mut d, 4, b"Test game");
        put(&mut d, 0x29, &35_000i32.to_le_bytes());
        put(&mut d, NUM_PLAYERS, &2u16.to_le_bytes());
        for (i, (emperor, race, human)) in [("Ann", "Human", true), ("Bob", "Alkari", false)].into_iter().enumerate() {
            let at = PLAYER + i * PLAYER_LEN;
            put(&mut d, at + 1, emperor.as_bytes());
            put(&mut d, at + 0x15, race.as_bytes());
            d[at + 0x28] = if human { 100 } else { 3 };
            put(&mut d, at + 0x32, &(50 + i as i32).to_le_bytes());
            put(&mut d, at + 0x1eb, &(100i32).to_le_bytes());
        }
        put(&mut d, NUM_STARS, &1u16.to_le_bytes());
        put(&mut d, STAR, b"Sol");
        put(&mut d, NUM_COLONIES, &2u16.to_le_bytes());
        for (i, outpost) in [0u8, 1].into_iter().enumerate() {
            let at = COLONY + i * COLONY_LEN;
            d[at] = 0;
            put(&mut d, at + 2, &(i as i16).to_le_bytes());
            d[at + 6] = outpost;
            put(&mut d, at + 0x130, &3i16.to_le_bytes());
        }
        for i in 0..2 {
            let p = PLANET + i * PLANET_LEN;
            d[p + 2] = 0;
            d[p + 3] = i as u8 + 2;
        }
        d
    }

    #[test]
    fn reads_and_edits_empires_and_colonies() {
        let d = fake();
        assert!(looks(&d));
        let mut t = parse(&d).unwrap();
        assert_eq!(t.view.pointer("/Game/Stardate").unwrap(), "3500.0");
        assert_eq!(t.view.pointer("/Empires/1. Ann (Human, you)/BC reserve").unwrap(), 50);
        assert_eq!(t.view.pointer("/Empires/2. Bob (Alkari)/BC reserve").unwrap(), 51);
        assert_eq!(t.view.pointer("/Colonies/Sol 3 (1)/Marines").unwrap(), 3);
        assert!(t.view.pointer("/Colonies/Sol 4 (1)").is_none(), "an outpost is not listed");
        t.set("/Empires/1. Ann (Human, you)/BC reserve", 999_999.into()).unwrap();
        t.set("/Colonies/Sol 3 (1)/Marines", 12.into()).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(out.len(), SIZE);
        let t2 = parse(&out).unwrap();
        assert_eq!(t2.view.pointer("/Empires/1. Ann (Human, you)/BC reserve").unwrap(), 999_999);
        assert_eq!(t2.view.pointer("/Colonies/Sol 3 (1)/Marines").unwrap(), 12);
        assert_eq!(t2.view.pointer("/Empires/2. Bob (Alkari)/BC reserve").unwrap(), 51);
    }

    /// `MOO2_SAMPLES` files: open through the whole pipeline, show the values, change the treasury.
    #[test]
    #[ignore]
    fn real_samples() {
        let Ok(list) = std::env::var("MOO2_SAMPLES") else { return };
        for f in list.split('|') {
            let raw = std::fs::read(f).unwrap();
            let mut d = crate::formats::decode(&raw);
            let crate::formats::Content::Tree(t) = &mut d.content else {
                println!("{f}: not a tree");
                continue;
            };
            println!("{f}: {}", t.view);
            let first = t.view.pointer("/Empires").and_then(|e| e.as_object()).and_then(|o| o.keys().next().cloned()).unwrap();
            let p = format!("/Empires/{first}/BC reserve");
            t.set(&p, 123_456.into()).unwrap();
            let out = crate::formats::encode(&d.layers, &d.content).unwrap();
            assert_eq!(out.len(), raw.len());
            assert_eq!(out.iter().zip(&raw).filter(|(a, b)| a != b).count(), 4 - raw[PLAYER + 0x32..PLAYER + 0x36].iter().zip(123_456i32.to_le_bytes()).filter(|(a, b)| **a == *b).count());
            let d2 = crate::formats::decode(&out);
            let crate::formats::Content::Tree(t2) = &d2.content else { panic!("{f}: reopened as hex") };
            assert_eq!(t2.view.pointer(&p).unwrap(), 123_456);
        }
    }
}
