//! Tekken, Tekken 2 and Tekken 3 (PlayStation 1) saves: one 8 KiB block per save. Offsets
//! follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md). Tekken 2 and 3 keep their data in the 512 bytes at 0x200, scrambled
//! byte by byte and guarded by a negated byte sum in the first byte; the editor works on the
//! unscrambled bytes and scrambles them again, with a new sum, whenever it writes the save.

use super::fields::{Builder, Checksum, Kind};
use super::ps1::{is_save, opts};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Tekken save";
pub const KIND2: &str = "Tekken 2 save";
pub const KIND3: &str = "Tekken 3 save";

#[derive(Clone, Copy, PartialEq)]
pub enum Game {
    One,
    Two,
    Three,
}

fn prefix(d: &[u8]) -> Option<&[u8]> {
    d.get(4..24)
}

pub fn game(d: &[u8]) -> Option<Game> {
    if !is_save(d, 1) {
        return None;
    }
    let t = prefix(d)?;
    if t.starts_with(&[0x82, 0x73, 0x82, 0x92, 0x82, 0x95, 0x82, 0x85, 0x81, 0x40, 0x82, 0x86]) || t.starts_with(&[0x90, 0x5e, 0x82, 0xcc, 0x8a, 0x69, 0x93, 0xac]) {
        return Some(Game::One);
    }
    let ok = |d: &[u8]| {
        let a = unscramble(d);
        a[0x200] == checksum(&a)
    };
    if t.starts_with(&[0x81, 0x6d, 0x82, 0x73, 0x82, 0x64, 0x82, 0x6a, 0x82, 0x6a, 0x82, 0x64, 0x82, 0x6d]) {
        let g = if t[14..16] == [0x82, 0x51] { Game::Two } else if t[14..18] == [0x81, 0x40, 0x82, 0x52] { Game::Three } else { return None };
        return ok(d).then_some(g);
    }
    if t.starts_with(&[0x8c, 0x83, 0x93, 0xac, 0x8d, 0xc4, 0x8a, 0x4a]) {
        return ok(d).then_some(Game::Two);
    }
    if t.starts_with(&[0x90, 0x56, 0x82, 0xbd, 0x82, 0xc8, 0x82, 0xe9, 0x93, 0xac, 0x82, 0xa2]) {
        return ok(d).then_some(Game::Three);
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    game(d).is_some()
}

fn unscramble(d: &[u8]) -> Vec<u8> {
    let mut a = d.to_vec();
    let mut prev = a[0x200];
    for i in 0x201..0x400 {
        let v = a[i].wrapping_sub(prev.wrapping_mul(5).wrapping_add(1));
        prev = a[i];
        a[i] = v;
    }
    a
}

fn checksum(a: &[u8]) -> u8 {
    a[0x201..0x400].iter().fold(0u8, |s, b| s.wrapping_add(*b)).wrapping_neg()
}

/// Writes the checksum and scrambles the data again.
fn repack(a: &mut Vec<u8>) {
    if a.len() < 0x2000 {
        return;
    }
    a[0x200] = checksum(a);
    let mut prev = a[0x200];
    for i in 0x201..0x400 {
        a[i] = a[i].wrapping_add(prev.wrapping_mul(5).wrapping_add(1));
        prev = a[i];
    }
}

const T1: &[&str] = &["Kazuya", "Paul", "Law", "Jack", "Nina", "King", "Yoshimitsu", "Michelle", "Wang", "Heihachi", "Lee", "Prototype Jack", "Armor King", "Anna", "Ganryu", "Kuma", "Kunimitsu"];
const T2: &[&str] = &[
    "Kazuya", "Paul", "Law", "Jack-2", "Nina", "King", "Yoshimitsu", "Michelle", "Wang", "Heihachi", "Lee", "Prototype Jack", "Armor King", "Anna", "Ganryu", "Kuma", "Kunimitsu", "Devil", "Lei", "Jun", "Bruce", "Baek", "Roger",
];
const T3: &[&str] = &[
    "Paul", "Law", "Lei", "King", "Yoshimitsu", "Nina", "Hwoarang", "Xiaoyu", "Eddy", "Jin", "Julia", "Kuma", "Bryan", "Heihachi", "Ogre", "Mokujin", "Gun Jack", "Gon", "Anna", "Doctor B.", "True Ogre",
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let g = game(d).ok_or("not a Tekken save")?;
    let data = if g == Game::One { d.to_vec() } else { unscramble(d) };
    let mut b = Builder::new(&data);
    let o = if g == Game::One { 0 } else { 0x200 };
    let names = match g {
        Game::One => T1,
        Game::Two => T2,
        Game::Three => T3,
    };
    let (unlocked, done, difficulty, fights, rtime, cont, speaker, bgm, win) = match g {
        Game::One => (0x20c, 0x210, 0x203, 0x204, 0x205, 0x207, 0x201, 0x202, Some(0x206)),
        Game::Two => (0x1, 0x5, 0xf, 0x10, 0x11, 0x14, 0xd, 0xe, Some(0x13)),
        Game::Three => (0x1, 0x9, 0x17, 0x18, 0x19, 0x1b, 0x15, 0x16, None),
    };
    for (i, n) in names.iter().enumerate() {
        flag(&mut b, &["Unlocked characters", n], o + unlocked + i / 8, (i % 8) as u32);
    }
    match g {
        Game::Two => {
            // The "defeated" bits start one bit in (Kazuya's sits at the end of the third byte).
            for (i, n) in names.iter().enumerate() {
                let (at, bit) = if i == 0 { (o + 0x7, 7) } else { (o + done + i / 8, (i % 8) as u32) };
                flag(&mut b, &["Arcade completed", n], at, bit);
            }
        }
        _ => {
            for (i, n) in names.iter().enumerate() {
                flag(&mut b, &["Arcade completed", n], o + done + i / 8, (i % 8) as u32);
            }
        }
    }
    if g == Game::Three {
        for (i, n) in names.iter().enumerate() {
            flag(&mut b, &["Unlocked costumes", n], o + 0x5 + i / 8, (i % 8) as u32);
        }
        flag(&mut b, &["Unlocked extras", "Tekken Ball mode"], o + 0x37, 0);
        flag(&mut b, &["Unlocked extras", "Theatre mode"], o + 0x38, 0);
        flag(&mut b, &["Unlocked extras", "Panda movie"], o + 0xe, 3);
        flag(&mut b, &["Unlocked extras", "Tiger movie"], o + 0xe, 0);
        flag(&mut b, &["Unlocked extras", "Gun Jack movie"], o + 0xf, 0);
        b.pick(&["Tekken Force", "Key"], o + 0x29, Kind::U8, &opts(&[("-", 0), ("Copper key", 1), ("Silver key", 2), ("Gold key", 3)]));
        b.ranged(&["Tekken Force", "High score"], o + 0x25, Kind::U32, 0, 99_999_999);
    }
    let diffs: &[(&str, u64)] = if g == Game::One { &[("Easy", 0), ("Medium", 1), ("Hard", 2), ("Very Hard", 3), ("Ultra Hard", 4)] } else { &[("Easy", 0), ("Medium", 1), ("Hard", 2)] };
    b.pick(&["Options", "Difficulty"], o + difficulty, Kind::U8, &opts(diffs));
    b.bits(&["Options", "Fights per match"], o + fights, Kind::U8, 0, 8, 1, 1, if g == Game::Two { 4 } else { 5 });
    b.pick(&["Options", "Round time"], o + rtime, Kind::U8, &opts(&[("20", 0), ("30", 1), ("40", 2), ("50", 3), ("60", 4), ("Unlimited", 5)]));
    b.pick(&["Options", "Change character at continue"], o + cont, Kind::U8, &opts(&[("No", 0), ("Yes", 1)]));
    b.pick(&["Options", "Sound output"], o + speaker, Kind::U8, &opts(&[("Mono", 0), ("Stereo", 1)]));
    let bgms: &[(&str, u64)] = match g {
        Game::Three => &[("Remixed", 0), ("Arcade", 1), ("Silent", 2)],
        _ => &[("Arranged", 0), ("Original", 1), ("Silent", 2)],
    };
    b.pick(&["Options", "Music"], o + bgm, Kind::U8, &opts(bgms));
    if let Some(w) = win {
        b.pick(&["Options", "Match wins shown as"], o + w, Kind::U8, &opts(&[("Fruit", 0), ("Number", 1)]));
    }
    if g != Game::One {
        let (guard, quick, auto) = if g == Game::Two { (0x12, 0x16, 0x18) } else { (0x1a, 0x21, 0x22) };
        let yn = opts(&[("No", 0), ("Yes", 1)]);
        b.pick(&["Options", "Guard damage"], o + guard, Kind::U8, &yn);
        b.pick(&["Options", "Quick select"], o + quick, Kind::U8, &yn);
        b.pick(&["Options", "Auto save"], o + auto, Kind::U8, &yn);
        let base = if g == Game::Two { 0x13d } else { 0x149 };
        for (i, n) in names.iter().enumerate() {
            let at = o + base + i * 8;
            b.ranged(&["Characters", n, "Arcade plays"], at, Kind::U16, 0, 9999);
            b.ranged(&["Characters", n, "Versus wins"], at + 2, Kind::U16, 0, 9999);
            b.ranged(&["Characters", n, "Versus losses"], at + 4, Kind::U16, 0, 9999);
            b.ranged(&["Characters", n, "Versus draws"], at + 6, Kind::U16, 0, 9999);
        }
    }
    let note = crate::i18n::tx("Close the emulator first. Tekken 2 and 3 keep their data scrambled and guarded by a checksum; both are written again on save.");
    let kind = match g {
        Game::One => KIND,
        Game::Two => KIND2,
        Game::Three => KIND3,
    };
    Ok(if g == Game::One { b.finish(kind, Checksum::None) } else { b.finish(kind, Checksum::Repack(repack)) }.with_note(note))
}
