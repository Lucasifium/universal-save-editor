//! Golden Sun and Golden Sun: The Lost Age (Game Boy Advance) battery saves, as emulators
//! write them (`.sav`, 64 KiB flash).
//!
//! The flash holds blocks that start with "CAMELOT", a slot number, a 16-bit checksum (the
//! sum of every byte after the 16-byte header) and a save counter. Golden Sun uses 4 KiB
//! blocks (slots 0–2 hold the party, 3–5 the second half of each file); The Lost Age uses
//! 12 KiB blocks. Old copies of a slot stay in the flash: the game loads the copy with the
//! highest counter, so that copy is the one shown and edited. Each edited block gets its
//! checksum back.
//!
//! Layout facts from Dyrati's obababot (`gsfuncs.py`, MIT) and Hambaka's Golden Sun save
//! tools (MIT); name tables from obababot's data. Checked against sample saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::OnceLock;

pub const KIND_GS1: &str = "Golden Sun save";
pub const KIND_GS2: &str = "Golden Sun: The Lost Age save";

const FLASH: usize = 0x10000;
/// Build dates stored in each file; they tell the game and its language apart.
const GS1_DATES: [(u16, &str); 6] =
    [(0x159C, "Japan"), (0x1652, "USA / Europe"), (0x1849, "Germany"), (0x1885, "Spain"), (0x1713, "France"), (0x1886, "Italy")];
const GS2_DATES: [(u16, &str); 6] =
    [(0x198A, "Japan"), (0x1C85, "USA / Europe"), (0x1D97, "Germany"), (0x1DC7, "Spain"), (0x1D98, "France"), (0x1DC8, "Italy")];
pub const MEMBERS: [&str; 8] = ["Isaac", "Garet", "Ivan", "Mia", "Felix", "Jenna", "Sheba", "Piers"];
pub const ELEMENTS: [&str; 4] = ["Venus", "Mercury", "Mars", "Jupiter"];
/// Size of one party member's record.
const PC: usize = 0x14C;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Game {
    GoldenSun,
    LostAge,
}

impl Game {
    fn block(self) -> usize {
        match self {
            Game::GoldenSun => 0x1000,
            Game::LostAge => 0x3000,
        }
    }
    /// The Lost Age fits five 12 KiB blocks in the flash.
    fn end(self) -> usize {
        match self {
            Game::GoldenSun => FLASH,
            Game::LostAge => 5 * 0x3000,
        }
    }
    fn dates(self) -> &'static [(u16, &'static str); 6] {
        match self {
            Game::GoldenSun => &GS1_DATES,
            Game::LostAge => &GS2_DATES,
        }
    }
    /// The Lost Age moves the party data 0x20 bytes on.
    fn shift(self) -> usize {
        match self {
            Game::GoldenSun => 0,
            Game::LostAge => 0x20,
        }
    }
    /// Djinn of each element in the game.
    pub fn djinn_per_element(self) -> usize {
        match self {
            Game::GoldenSun => 7,
            Game::LostAge => 18,
        }
    }
    fn members(self) -> usize {
        match self {
            Game::GoldenSun => 7,
            Game::LostAge => 8,
        }
    }
    pub fn kind(self) -> &'static str {
        match self {
            Game::GoldenSun => KIND_GS1,
            Game::LostAge => KIND_GS2,
        }
    }
}

/// The copy of one file (slot 0–2) the game would load.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub file: u8,
    pub at: usize,
    pub count: u16,
    pub date: u16,
}

fn u16_at(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

fn u32_at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

/// Sum of the block's bytes after its header, as stored at offset 8.
pub fn block_sum(d: &[u8], at: usize, size: usize) -> u16 {
    d[at + 16..at + size].iter().fold(0u16, |s, &b| s.wrapping_add(u16::from(b)))
}

fn slots_of(d: &[u8], g: Game) -> Vec<Slot> {
    let size = g.block();
    let mut best: Vec<Slot> = vec![];
    let mut at = 0;
    while at + size <= g.end() {
        let h = &d[at..at + 16];
        let date = u16_at(d, at + 0x36) & 0x7FFF;
        if &h[..7] == b"CAMELOT" && h[7] <= 2 && u16_at(d, at + 8) == block_sum(d, at, size) && g.dates().iter().any(|x| x.0 == date) {
            let s = Slot { file: h[7], at, count: u16_at(d, at + 10), date };
            match best.iter_mut().find(|b| b.file == s.file) {
                Some(b) if b.count < s.count => *b = s,
                Some(_) => {}
                None => best.push(s),
            }
        }
        at += size;
    }
    // Most saved file first.
    best.sort_by(|a, b| b.count.cmp(&a.count).then(a.file.cmp(&b.file)));
    best
}

/// Which game wrote this flash, and its files.
pub fn scan(d: &[u8]) -> Option<(Game, Vec<Slot>)> {
    // 64 KiB; some emulators write a 128 KiB flash or append a few bytes.
    if d.len() < FLASH || d.len() > 2 * FLASH + 0x400 || d.get(..7) != Some(b"CAMELOT".as_slice()) && !(1..16).any(|i| d[i * 0x1000..].starts_with(b"CAMELOT")) {
        return None;
    }
    for g in [Game::LostAge, Game::GoldenSun] {
        let s = slots_of(d, g);
        if !s.is_empty() {
            return Some((g, s));
        }
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    scan(d).is_some()
}

// ------------------------------------------------------------------ names

pub struct Tables {
    pub items: [Vec<String>; 2],
    pub stackable: [Vec<u16>; 2],
    pub classes: [Vec<String>; 2],
    /// 20 per element (Venus, Mercury, Mars, Jupiter).
    pub djinn: Vec<String>,
}

pub fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let src = include_str!("data/goldensun.txt");
        let line = |k: &str| src.lines().find_map(|l| l.strip_prefix(k)).unwrap_or("");
        let names = |k: &str| line(k).split('|').map(str::to_string).collect::<Vec<_>>();
        let ids = |k: &str| line(k).split(',').filter_map(|x| x.parse().ok()).collect::<Vec<u16>>();
        Tables {
            items: [names("IT1="), names("IT2=")],
            stackable: [ids("ST1="), ids("ST2=")],
            classes: [names("CL1="), names("CL2=")],
            djinn: names("DJ="),
        }
    })
}

fn gi(g: Game) -> usize {
    match g {
        Game::GoldenSun => 0,
        Game::LostAge => 1,
    }
}

pub const EMPTY: &str = "(empty)";

/// An item's name; ids without a usable name (or with a repeated one) get `#id`.
pub fn item_name(g: Game, id: u16) -> String {
    let t = &tables().items[gi(g)];
    match t.get(id as usize) {
        _ if id == 0 => EMPTY.into(),
        Some(n) if !n.is_empty() && !n.starts_with('?') => {
            if t.iter().filter(|x| *x == n).count() > 1 {
                format!("{n} #{id}")
            } else {
                n.clone()
            }
        }
        _ => format!("#{id}"),
    }
}

pub fn is_stackable(g: Game, id: u16) -> bool {
    tables().stackable[gi(g)].contains(&id)
}

/// Every named item of one kind (stackable or not), as pick-list options.
fn item_options(g: Game, stackable: bool) -> Vec<(String, u64)> {
    let t = &tables().items[gi(g)];
    (1..t.len() as u16)
        .filter(|&i| !t[i as usize].starts_with('?') && !t[i as usize].is_empty() && is_stackable(g, i) == stackable)
        .map(|i| (item_name(g, i), u64::from(i)))
        .collect()
}

fn class_name(g: Game, id: u8) -> String {
    match tables().classes[gi(g)].get(id as usize) {
        Some(n) if !n.is_empty() => n.clone(),
        _ => format!("#{id}"),
    }
}

pub fn djinn_name(e: usize, bit: usize) -> String {
    tables().djinn.get(e * 20 + bit).cloned().unwrap_or_else(|| format!("#{bit}"))
}

/// A name as the game stores it: Latin-1 for the western games, half-width katakana in
/// the Japanese ones.
fn text(b: &[u8], japanese: bool) -> String {
    b.iter()
        .take_while(|&&c| c != 0)
        .map(|&c| match c {
            0xA1..=0xDF if japanese => char::from_u32(0xFF61 + u32::from(c - 0xA1)).unwrap_or('?'),
            0x20..=0x7E | 0xA0..=0xFF => c as char,
            _ => '?',
        })
        .collect()
}

// ------------------------------------------------------------------ reader

/// Gives each block that was written to its checksum back.
fn fix(out: &mut [u8], touched: &[usize], size: usize) {
    let mut blocks: Vec<usize> = touched.iter().map(|&a| a - a % size).collect();
    blocks.sort_unstable();
    blocks.dedup();
    for b in blocks {
        if b + size <= out.len() && out[b..b + 7] == *b"CAMELOT" {
            let s = block_sum(out, b, size);
            out[b + 8..b + 10].copy_from_slice(&s.to_le_bytes());
        }
    }
}

/// After a Djinn edit: the counts the game keeps beside the bits (owned and set, per member
/// and element) and the totals on the file-select screen are made to match. Only files whose
/// Djinn words were written are touched. Layout from game-tools-collection (RyudoSynbios, MIT).
fn fix_djinn(out: &mut [u8], touched: &[usize], g: Game, size: usize) {
    let mut blocks: Vec<usize> = touched.iter().map(|&a| a - a % size).collect();
    blocks.sort_unstable();
    blocks.dedup();
    for blk in blocks {
        let base = blk + 0x10;
        let party = base + 0x500 + g.shift();
        if party + PC * g.members() > out.len() || out[blk..blk + 7] != *b"CAMELOT" {
            continue;
        }
        let djinn = |a: usize| (party..party + PC * g.members()).contains(&a) && (0xF8..0x118).contains(&((a - party) % PC));
        if !touched.iter().any(|&a| a - a % size == blk && djinn(a)) {
            continue;
        }
        for p in 0..g.members() {
            let r = party + PC * p;
            for e in 0..4 {
                let owned = u32_at(out, r + 0xF8 + 4 * e);
                // A Djinni that isn't owned can't be set.
                let set = u32_at(out, r + 0x108 + 4 * e) & owned;
                out[r + 0x108 + 4 * e..r + 0x10C + 4 * e].copy_from_slice(&set.to_le_bytes());
                out[r + 0x118 + e] = owned.count_ones() as u8;
                out[r + 0x11C + e] = set.count_ones() as u8;
            }
        }
        let n = out[base + 0x40].count_ones() as usize;
        let mut all = [0u32; 4];
        for j in 0..n {
            let who = out[base + 0x438 + g.shift() + j] as usize;
            if who < g.members() {
                for (e, a) in all.iter_mut().enumerate() {
                    *a |= u32_at(out, party + PC * who + 0xF8 + 4 * e);
                }
            }
        }
        for (e, a) in all.iter().enumerate() {
            out[base + 0x18 + e] = a.count_ones() as u8;
        }
    }
}

fn fix_gs1(out: &mut Vec<u8>, touched: &[usize]) {
    fix_djinn(out, touched, Game::GoldenSun, 0x1000);
    fix(out, touched, 0x1000);
}

fn fix_gs2(out: &mut Vec<u8>, touched: &[usize]) {
    fix_djinn(out, touched, Game::LostAge, 0x3000);
    fix(out, touched, 0x3000);
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (g, slots) = scan(d).ok_or("not a Golden Sun save")?;
    let mut b = Builder::new(d);
    for s in &slots {
        let file = format!("File {}", s.file + 1);
        let f = file.as_str();
        let base = s.at + 0x10;
        let japanese = s.date == g.dates()[0].0;
        let lang = g.dates().iter().find(|x| x.0 == s.date).map_or("?", |x| x.1);
        b.info(&[f, "Version"], lang);
        b.info(&[f, "Leader"], text(&d[base..base + 12], japanese));
        b.info(&[f, "Save count"], s.count);
        let secs = u32_at(d, base + 0x244) / 60;
        b.info(&[f, "Play time"], format!("{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60));
        b.ranged(&[f, "Coins"], base + 0x250, Kind::U32, 0, 99_999_999);

        // Party, in marching order.
        let mask = d[base + 0x40];
        let n = mask.count_ones() as usize;
        let mut order: Vec<usize> = (0..n).map(|j| d[base + 0x438 + g.shift() + j] as usize).collect();
        let mut seen = order.clone();
        seen.sort_unstable();
        seen.dedup();
        if seen.len() != n || order.iter().any(|&p| p >= g.members() || mask >> p & 1 == 0) {
            order = (0..g.members()).filter(|&p| mask >> p & 1 != 0).collect();
        }
        for p in order {
            let r = base + 0x500 + g.shift() + PC * p;
            let m = MEMBERS[p];
            let path = |k: &'static str| -> [&str; 4] { [f, "Party", m, k] };
            b.info(&path("Name"), text(&d[r..r + 15], japanese));
            b.info(&path("Class"), class_name(g, d[r + 0x129]));
            b.ranged(&path("Level"), r + 0xF, Kind::U8, 1, 99);
            b.field(&path("Experience"), r + 0x124, Kind::U32);
            let (max_hp, max_pp) = (u16_at(d, r + 0x34), u16_at(d, r + 0x36));
            b.ranged(&path("HP"), r + 0x38, Kind::U16, 0, i64::from(max_hp));
            b.info(&path("Max HP"), max_hp);
            b.ranged(&path("PP"), r + 0x3A, Kind::U16, 0, i64::from(max_pp));
            b.info(&path("Max PP"), max_pp);
            for (k, off, kind, lo, hi) in [
                ("Max HP", 0x10, Kind::U16, 1, 9999),
                ("Max PP", 0x12, Kind::U16, 0, 9999),
                ("Attack", 0x18, Kind::U16, 0, 9999),
                ("Defense", 0x1A, Kind::U16, 0, 9999),
                ("Agility", 0x1C, Kind::U16, 0, 9999),
                ("Luck", 0x1E, Kind::U8, 0, 255),
            ] {
                b.ranged(&[f, "Party", m, "Base stats", k], r + off, kind, lo, hi);
            }
            for (k, off) in [("Attack", 0x3C), ("Defense", 0x3E), ("Agility", 0x40)] {
                b.info(&[f, "Party", m, "Stats", k], u16_at(d, r + off));
            }
            b.info(&[f, "Party", m, "Stats", "Luck"], d[r + 0x42]);

            // Djinn: a bit per Djinni for "has it" and another for "set" (the rest are on standby).
            for (e, el) in ELEMENTS.iter().enumerate() {
                for bit in 0..g.djinn_per_element() {
                    let name = djinn_name(e, bit);
                    b.bits(&[f, "Party", m, "Djinn", el, &name, "Owned"], r + 0xF8 + 4 * e, Kind::U32, bit as u32, 1, 0, 0, 1);
                    b.bits(&[f, "Party", m, "Djinn", el, &name, "Set"], r + 0x108 + 4 * e, Kind::U32, bit as u32, 1, 0, 0, 1);
                }
            }

            // Items: the list has no gaps, so only the first empty place takes a new item.
            for j in 0..15 {
                let at = r + 0xD8 + 2 * j;
                let v = u16_at(d, at);
                let id = v & 0x1FF;
                let key = format!("Slot {:02}", j + 1);
                let ip = |k: &'static str| -> [&str; 6] { [f, "Party", m, "Items", key.as_str(), k] };
                if id == 0 {
                    let mut opts = vec![(EMPTY.to_string(), 0)];
                    opts.extend(item_options(g, false));
                    opts.extend(item_options(g, true));
                    opts.sort_by(|a, b| (a.1 != 0).cmp(&(b.1 != 0)).then(a.0.to_lowercase().cmp(&b.0.to_lowercase())));
                    b.pick_bits(&ip("Item"), at, Kind::U16, 0, 11, &opts);
                    break;
                }
                let stack = is_stackable(g, id);
                let qty = i64::from(v >> 11) + 1;
                let equipped = v & 0x200 != 0;
                let broken = v & 0x400 != 0;
                // An item swaps only for one of the same kind, so its quantity stays meaningful.
                let mut opts = item_options(g, stack);
                if !opts.iter().any(|o| o.1 == u64::from(id)) {
                    opts.push((item_name(g, id), u64::from(id)));
                }
                opts.sort_by_key(|o| o.0.to_lowercase());
                if equipped || broken || b.pick_bits(&ip("Item"), at, Kind::U16, 0, 11, &opts).is_none() {
                    b.info(&ip("Item"), item_name(g, id));
                }
                if stack && qty <= 30 {
                    b.bits(&ip("Quantity"), at, Kind::U16, 11, 5, 1, 1, 30);
                } else {
                    b.info(&ip("Quantity"), qty);
                }
                b.info(&ip("Equipped"), equipped);
                if broken {
                    b.info(&ip("Broken"), true);
                }
            }
        }
    }
    let fixer = match g {
        Game::GoldenSun => fix_gs1,
        Game::LostAge => fix_gs2,
    };
    Ok(b.finish(g.kind(), Checksum::Touched(fixer)).with_note(
        "Each file is the copy the game loads (the one saved most often). Base stats are what the game builds a character's stats from; class, Djinn and equipment are applied on top.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// A synthetic flash: one file in the given game, with one party member.
    fn flash(g: Game, file: u8, block_index: usize, count: u16) -> Vec<u8> {
        let mut d = vec![0xFFu8; FLASH];
        put_block(&mut d, g, file, block_index, count, 1234);
        d
    }

    fn put_block(d: &mut [u8], g: Game, file: u8, block_index: usize, count: u16, coins: u32) {
        let size = g.block();
        let at = block_index * size;
        d[at..at + size].fill(0);
        d[at..at + 7].copy_from_slice(b"CAMELOT");
        d[at + 7] = file;
        d[at + 10..at + 12].copy_from_slice(&count.to_le_bytes());
        let base = at + 0x10;
        d[base..base + 4].copy_from_slice(b"Hero");
        d[base + 0x26..base + 0x28].copy_from_slice(&g.dates()[1].0.to_le_bytes());
        d[base + 0x244..base + 0x248].copy_from_slice(&(3600u32 * 60 + 61 * 60).to_le_bytes());
        d[base + 0x250..base + 0x254].copy_from_slice(&coins.to_le_bytes());
        let lead = if g == Game::LostAge { 4 } else { 0 };
        d[base + 0x40] = 1 << lead;
        d[base + 0x438 + g.shift()] = lead as u8;
        let r = base + 0x500 + g.shift() + PC * lead;
        d[r..r + 4].copy_from_slice(b"Hero");
        d[r + 0xF] = 12;
        d[r + 0x10..r + 0x12].copy_from_slice(&120u16.to_le_bytes());
        d[r + 0x18..r + 0x1A].copy_from_slice(&50u16.to_le_bytes());
        d[r + 0x34..r + 0x36].copy_from_slice(&150u16.to_le_bytes());
        d[r + 0x38..r + 0x3A].copy_from_slice(&90u16.to_le_bytes());
        d[r + 0x124..r + 0x128].copy_from_slice(&5000u32.to_le_bytes());
        d[r + 0x129] = 1;
        d[r + 0xF8] = 0b101; // Flint and Quartz owned…
        d[r + 0x108] = 0b001; // …Flint set.
        // Long Sword (equipped), 3 Herbs, then nothing.
        d[r + 0xD8..r + 0xDA].copy_from_slice(&(1u16 | 0x200).to_le_bytes());
        d[r + 0xDA..r + 0xDC].copy_from_slice(&(180u16 | 2 << 11).to_le_bytes());
        let s = block_sum(d, at, size);
        d[at + 8..at + 10].copy_from_slice(&s.to_le_bytes());
    }

    #[test]
    fn reads_and_edits_a_golden_sun_flash() {
        let mut d = flash(Game::GoldenSun, 0, 3, 7);
        // An older copy of the same file, and a second half (slot 3): both left alone.
        put_block(&mut d, Game::GoldenSun, 0, 9, 6, 1);
        put_block(&mut d, Game::GoldenSun, 3, 10, 7, 1);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, KIND_GS1);
        let v = &doc.view;
        assert_eq!(v.pointer("/File 1/Coins"), Some(&Value::from(1234)));
        assert_eq!(v.pointer("/File 1/Play time"), Some(&Value::from("1:01:01")));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Level"), Some(&Value::from(12)));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Djinn/Venus/Flint/Owned"), Some(&Value::from(1)));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Djinn/Venus/Flint/Set"), Some(&Value::from(1)));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Djinn/Venus/Quartz/Owned"), Some(&Value::from(1)));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Djinn/Venus/Quartz/Set"), Some(&Value::from(0)));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Djinn/Venus/Granite/Owned"), Some(&Value::from(0)));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Items/Slot 01/Item"), Some(&Value::from("Long Sword")));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Items/Slot 02/Quantity"), Some(&Value::from(3)));
        assert_eq!(v.pointer("/File 1/Party/Isaac/Items/Slot 03/Item"), Some(&Value::from(EMPTY)));
        assert!(v.pointer("/File 1/Party/Isaac/Items/Slot 04").is_none());
        // An equipped item isn't swapped; a stackable one only for another stackable one.
        assert!(!doc.editable.contains_key("/File 1/Party/Isaac/Items/Slot 01/Item"));
        assert!(doc.set("/File 1/Party/Isaac/Items/Slot 02/Item", Value::from("Long Sword")).is_err());
        assert!(doc.set("/File 1/Party/Isaac/HP", Value::from(151)).is_err());
        doc.set("/File 1/Coins", Value::from(1235)).unwrap();
        doc.set("/File 1/Party/Isaac/Base stats/Attack", Value::from(60)).unwrap();
        doc.set("/File 1/Party/Isaac/Items/Slot 02/Quantity", Value::from(30)).unwrap();
        doc.set("/File 1/Party/Isaac/Items/Slot 02/Item", Value::from("Nut")).unwrap();
        doc.set("/File 1/Party/Isaac/Items/Slot 03/Item", Value::from("Broad Sword")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let at = 3 * 0x1000;
        assert_eq!(u16_at(&out, at + 8), block_sum(&out, at, 0x1000));
        assert_eq!(u32_at(&out, at + 0x10 + 0x250), 1235);
        let r = at + 0x10 + 0x500;
        assert_eq!(u16_at(&out, r + 0x18), 60);
        assert_eq!(u16_at(&out, r + 0xDA), 181 | 29 << 11);
        assert_eq!(u16_at(&out, r + 0xDC), 2);
        // Only the edited block changed.
        let changed: Vec<usize> = (0..out.len()).filter(|&i| out[i] != d[i]).collect();
        assert!(changed.iter().all(|&i| (at..at + 0x1000).contains(&i)));
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/File 1/Party/Isaac/Items/Slot 02/Item"), Some(&Value::from("Nut")));
        assert_eq!(again.view.pointer("/File 1/Party/Isaac/Items/Slot 03/Item"), Some(&Value::from("Broad Sword")));
        // Giving a Djinni and setting it: the counts beside the bits and the file-select totals follow.
        let mut doc = again;
        doc.set("/File 1/Party/Isaac/Djinn/Venus/Granite/Owned", Value::from(1)).unwrap();
        doc.set("/File 1/Party/Isaac/Djinn/Venus/Granite/Set", Value::from(1)).unwrap();
        doc.set("/File 1/Party/Isaac/Djinn/Mars/Forge/Set", Value::from(1)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, r + 0xF8), 0b111);
        assert_eq!(u32_at(&out, r + 0x108), 0b011);
        assert_eq!(u32_at(&out, r + 0x108 + 8), 0, "a Djinni that isn't owned stays unset");
        assert_eq!((out[r + 0x118], out[r + 0x11C], out[r + 0x11C + 2]), (3, 2, 0));
        assert_eq!(out[at + 0x10 + 0x18], 3);
        assert_eq!(u16_at(&out, at + 8), block_sum(&out, at, 0x1000));
    }

    #[test]
    fn reads_the_lost_age_and_orders_files() {
        let mut d = flash(Game::LostAge, 0, 1, 3);
        put_block(&mut d, Game::LostAge, 2, 4, 40, 99);
        let (g, slots) = scan(&d).unwrap();
        assert_eq!(g, Game::LostAge);
        assert_eq!(slots.iter().map(|s| s.file).collect::<Vec<_>>(), vec![2, 0]);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, KIND_GS2);
        let keys: Vec<&String> = doc.view.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["File 3", "File 1"]);
        assert_eq!(doc.view.pointer("/File 3/Party/Felix/Name"), Some(&Value::from("Hero")));
        doc.set("/File 3/Party/Felix/Level", Value::from(13)).unwrap();
        let out = doc.to_bytes().unwrap();
        let at = 4 * 0x3000;
        assert_eq!(out[at + 0x10 + 0x520 + PC * 4 + 0xF], 13);
        assert_eq!(u16_at(&out, at + 8), block_sum(&out, at, 0x3000));
        // A broken checksum hides that copy; other data isn't a Golden Sun save.
        d[at + 0x100] ^= 1;
        assert_eq!(scan(&d).unwrap().1.len(), 1);
        assert!(!looks(&vec![0u8; FLASH]));
        assert!(!looks(&d[..0x8000]));
        assert!(!crate::fmt::pokemon3::looks(&d));
    }

    #[test]
    fn name_tables() {
        assert_eq!(item_name(Game::LostAge, 180), "Herb");
        assert!(is_stackable(Game::GoldenSun, 183));
        assert_eq!(djinn_name(3, 0), "Gust");
        assert_eq!(text(&[0xDB, 0xCB, 0xDE, 0xDD, 0], true), "\u{FF9B}\u{FF8B}\u{FF9E}\u{FF9D}");
        assert_eq!(text(b"F\xe9lix\0", false), "F\u{e9}lix");
        // Every option in a pick list is unique.
        for g in [Game::GoldenSun, Game::LostAge] {
            let mut all: Vec<String> = item_options(g, false).into_iter().chain(item_options(g, true)).map(|o| o.0).collect();
            let n = all.len();
            all.sort();
            all.dedup();
            assert_eq!(all.len(), n);
        }
    }

    /// Research: every sample save in GOLDEN_SUN_SAVES (a folder) parses, writes back
    /// unchanged, and takes an edit with valid checksums and nothing else changed.
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("GOLDEN_SUN_SAVES").expect("GOLDEN_SUN_SAVES");
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let d = std::fs::read(&p).unwrap();
            let Some((g, slots)) = scan(&d) else {
                println!("{}: not recognised", p.display());
                continue;
            };
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d, "{}", p.display());
            let s = slots[0];
            let f = format!("/File {}", s.file + 1);
            let coins = doc.view.pointer(&format!("{f}/Coins")).and_then(Value::as_u64).unwrap();
            let new_coins = if coins >= 99_999_999 { coins - 1 } else { coins + 1 };
            doc.set(&format!("{f}/Coins"), Value::from(new_coins)).unwrap();
            let party: Vec<String> = doc.view.pointer(&format!("{f}/Party")).and_then(Value::as_object).map(|m| m.keys().cloned().collect()).unwrap_or_default();
            let who = &party[0];
            let lp = format!("{f}/Party/{who}/Level");
            let level = doc.view.pointer(&lp).and_then(Value::as_u64).unwrap();
            let new_level = if level >= 99 { 98 } else { level + 1 };
            doc.set(&lp, Value::from(new_level)).unwrap();
            let ap = format!("{f}/Party/{who}/Base stats/Attack");
            let atk = doc.view.pointer(&ap).and_then(Value::as_u64).unwrap();
            doc.set(&ap, Value::from(atk + 1)).unwrap();
            let out = doc.to_bytes().unwrap();
            let (g2, slots2) = scan(&out).expect("still a save");
            assert_eq!(g2, g);
            assert_eq!(slots2.len(), slots.len(), "{}", p.display());
            let size = g.block();
            assert_eq!(u16_at(&out, s.at + 8), block_sum(&out, s.at, size));
            assert!((0..out.len()).filter(|&i| out[i] != d[i]).all(|i| (s.at..s.at + size).contains(&i)), "{}", p.display());
            let back = parse(&out).unwrap();
            assert_eq!(back.view.pointer(&format!("{f}/Coins")).and_then(Value::as_u64), Some(new_coins));
            assert_eq!(back.view.pointer(&lp).and_then(Value::as_u64), Some(new_level));
            assert_eq!(back.view.pointer(&ap).and_then(Value::as_u64), Some(atk + 1));
            // Everything else in the view is as it was.
            let mut b2 = back.view.clone();
            let mut o = parse(&d).unwrap().view;
            for q in [format!("{f}/Coins"), lp.clone(), ap.clone()] {
                *o.pointer_mut(&q).unwrap() = Value::Null;
                *b2.pointer_mut(&q).unwrap() = Value::Null;
            }
            assert_eq!(o, b2, "{}", p.display());
            // Djinn: flipping one and flipping it back gives the file as it was, so the counts
            // the editor works out are the ones the game wrote.
            let dj = doc.view.pointer(&format!("{f}/Party/{who}/Djinn/Venus")).and_then(Value::as_object).unwrap().keys().next().unwrap().clone();
            let dp = format!("{f}/Party/{who}/Djinn/Venus/{dj}/Owned");
            let mut d1 = parse(&d).unwrap();
            let was = d1.view.pointer(&dp).and_then(Value::as_u64).unwrap();
            let set_was = d1.view.pointer(&dp.replace("/Owned", "/Set")).and_then(Value::as_u64).unwrap();
            d1.set(&dp, Value::from(1 - was)).unwrap();
            let mid = d1.to_bytes().unwrap();
            let mut d2 = parse(&mid).unwrap();
            d2.set(&dp, Value::from(was)).unwrap();
            d2.set(&dp.replace("/Owned", "/Set"), Value::from(set_was)).unwrap();
            let round = d2.to_bytes().unwrap();
            let diff: Vec<usize> = (0..d.len()).filter(|&i| round[i] != d[i]).collect();
            assert!(diff.is_empty(), "{}: Djinn round trip differs at {:x?}", p.display(), &diff[..diff.len().min(12)]);
            println!("{}: {:?}, files {:?}, coins {coins}, {who} level {level}", p.file_name().unwrap().to_string_lossy(), g, slots.iter().map(|s| s.file + 1).collect::<Vec<_>>());
            n += 1;
        }
        println!("{n} saves checked");
        assert!(n > 0);
    }
}
