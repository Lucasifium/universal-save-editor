//! The Elder Scrolls IV: Oblivion saves (`.ess`).
//!
//! ```text
//! "TES4SAVEGAME" major minor exe-time(16) header-version header-size
//! header: save number, name (u8 length), level u16, place (u8 length), days f32, ticks u32,
//!         game time(16), screenshot (size, width, height, pixels)
//! plugin list, then: form-id table offset, record count, 32 bytes, globals (u16 count of
//! (id, f32)), four u16-sized blocks, 4 bytes, created forms (u32 count of 20-byte headers with
//! a body), four u16-sized blocks, then the change records
//!   id u32, kind u8, flags u32, version u8, size u16, data
//! ```
//!
//! Nothing is compressed or checksummed, so values are patched in place. The player is two
//! change records: form 7 (the `NPC_` base, kind 35) holds the attributes (8 bytes), the base
//! data (flags, base spell, fatigue, barter gold, level offset, calc min/max), the base health
//! and the 21 skills; form 0x14 (the player reference, kind 50) holds the inventory, whose
//! entries are `item index u32, count u32, extra count u32`. The gold is the item whose form is
//! 0x0000000F. The record layout of the NPC change record follows the community documentation
//! of the format (Wrye Bash, GPL-3.0); the gold entry was found on public saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Oblivion save";

const ATTRIBUTES: [&str; 8] = ["Strength", "Intelligence", "Willpower", "Agility", "Speed", "Endurance", "Personality", "Luck"];
const SKILLS: [&str; 21] = [
    "Armorer", "Athletics", "Blade", "Block", "Blunt", "Hand to Hand", "Heavy Armor", "Alchemy", "Alteration", "Conjuration", "Destruction",
    "Illusion", "Mysticism", "Restoration", "Acrobatics", "Light Armor", "Marksman", "Mercantile", "Security", "Sneak", "Speechcraft",
];

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"TES4SAVEGAME")
}

struct Cur<'a> {
    d: &'a [u8],
    i: usize,
}

impl Cur<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let v = *self.d.get(self.i).ok_or("truncated save")?;
        self.i += 1;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16, String> {
        let b = self.d.get(self.i..self.i + 2).ok_or("truncated save")?;
        self.i += 2;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.d.get(self.i..self.i + 4).ok_or("truncated save")?;
        self.i += 4;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn skip(&mut self, n: usize) -> Result<(), String> {
        self.i = self.i.checked_add(n).filter(|&e| e <= self.d.len()).ok_or("truncated save")?;
        Ok(())
    }
    fn text(&mut self) -> Result<String, String> {
        let n = self.u8()? as usize;
        let b = self.d.get(self.i..self.i + n).ok_or("truncated save")?;
        self.i += n;
        let b = &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())];
        Ok(b.iter().map(|&c| super::bethesda_cp1252(c)).collect())
    }
}

/// A change record's data range and flags.
struct Change {
    flags: u32,
    at: usize,
    len: usize,
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not an Oblivion save".into());
    }
    let mut c = Cur { d, i: 30 };
    let _header_version = c.u32()?;
    let header_size = c.u32()? as usize;
    let header_end = 38usize.checked_add(header_size).ok_or("bad header")?;
    c.u32()?; // save number
    let name = c.text()?;
    let level_at = c.i;
    let level = c.u16()?;
    let place = c.text()?;
    c.i = header_end;
    let plugins = c.u8()? as usize;
    for _ in 0..plugins {
        let n = c.u8()? as usize;
        c.skip(n)?;
    }
    c.skip(8 + 32)?;
    let globals = c.u16()? as usize;
    c.skip(globals * 8)?;
    for _ in 0..4 {
        let n = c.u16()? as usize;
        c.skip(n)?;
    }
    c.skip(4)?;
    let created = c.u32()? as usize;
    for _ in 0..created {
        c.skip(4)?;
        let size = c.u32()? as usize;
        c.skip(12 + size)?;
    }
    for _ in 0..4 {
        let n = c.u16()? as usize;
        c.skip(n)?;
    }
    // The record count sits right after the plugin list's table offset.
    let mut h = Cur { d, i: header_end + 1 };
    for _ in 0..plugins {
        let n = h.u8()? as usize;
        h.skip(n)?;
    }
    let _fid_table = h.u32()?;
    let records = h.u32()? as usize;

    let (mut npc, mut player) = (None, None);
    for _ in 0..records {
        let id = c.u32()?;
        let kind = c.u8()?;
        let flags = c.u32()?;
        c.u8()?;
        let len = c.u16()? as usize;
        let at = c.i;
        c.skip(len)?;
        match (id, kind) {
            (7, 35) if npc.is_none() => npc = Some(Change { flags, at, len }),
            (0x14, 50) if player.is_none() => player = Some(Change { flags, at, len }),
            _ => {}
        }
    }
    let npc = npc.ok_or("the save has no player record")?;
    // Form ids: the item index of the gold.
    let tmp = c.u32()? as usize;
    c.skip(tmp)?;
    let count = c.u32()? as usize;
    let mut gold_index = None;
    for i in 0..count {
        if c.u32()? == 0x0F {
            gold_index = Some(i as u32);
            break;
        }
    }

    // The NPC change record, in the order its flags are stored.
    let f = npc.flags;
    let mut p = npc.at;
    if f & 1 != 0 {
        p += 4;
    }
    let attrs = (f & 1 << 3 != 0).then(|| {
        let a = p;
        p += 8;
        a
    });
    let acbs = (f & 1 << 4 != 0).then(|| {
        let a = p;
        p += 16;
        a
    });
    if f & 1 << 6 != 0 {
        let n = d.get(p..p + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or("truncated record")? as usize;
        p += 2 + 5 * n;
    }
    if f & 1 << 5 != 0 {
        let n = d.get(p..p + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or("truncated record")? as usize;
        p += 2 + 4 * n;
    }
    if f & 1 << 8 != 0 {
        p += 4;
    }
    let health = (f & 1 << 2 != 0).then(|| {
        let a = p;
        p += 4;
        a
    });
    if f & 1 << 28 != 0 {
        let n = d.get(p..p + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or("truncated record")? as usize;
        p += 2 + 5 * n;
    }
    if f & 1 << 7 != 0 {
        p += 1 + *d.get(p).ok_or("truncated record")? as usize;
    }
    let skills = (f & 1 << 9 != 0).then_some(p);
    if let Some(s) = skills {
        p = s + 21;
    }
    if p != npc.at + npc.len {
        return Err("the player record has an unexpected layout".into());
    }

    let mut b = Builder::new(d);
    if !name.is_empty() {
        b.info(&["Character", "Name"], name);
    }
    if !place.is_empty() {
        b.info(&["Character", "Location"], place);
    }
    if let Some(a) = acbs {
        // The level is also in the file's header (the load screen).
        if level >= 1 && u16::from_le_bytes([d[a + 10], d[a + 11]]) == level {
            b.ranged(&["Character", "Level"], a + 10, Kind::I16, 1, 255);
            b.mirror(&["Character", "Level"], level_at);
        }
    }
    if let (Some(g), Some(pl)) = (gold_index, &player) {
        if pl.flags & (1 << 27) != 0 {
            let data = &d[pl.at..pl.at + pl.len];
            let hit = (0..data.len().saturating_sub(11)).find(|&o| {
                let w = |k: usize| u32::from_le_bytes(data[o + k..o + k + 4].try_into().unwrap());
                w(0) == g && (1..=i32::MAX as u32).contains(&w(4)) && w(8) == 0
            });
            if let Some(o) = hit {
                b.ranged(&["Character", "Gold"], pl.at + o + 4, Kind::U32, 0, i32::MAX as i64);
            }
        }
    }
    if let Some(a) = attrs {
        for (i, n) in ATTRIBUTES.iter().enumerate() {
            b.ranged(&["Attributes", n], a + i, Kind::U8, 0, 100);
        }
    }
    if let Some(s) = skills {
        for (i, n) in SKILLS.iter().enumerate() {
            b.ranged(&["Skills", n], s + i, Kind::U8, 0, 100);
        }
    }
    if let Some(h) = health {
        b.ranged(&["Character", "Base health"], h, Kind::U16, 1, 9999);
    }
    if b.is_empty() {
        return Err("nothing to edit in this save".into());
    }
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Level, gold, attributes, skills and base health of an Oblivion .ess save. Checked on public saves, not yet tried in the game. Back up the save first.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bz(s: &str) -> Vec<u8> {
        let mut v = vec![s.len() as u8 + 1];
        v.extend(s.as_bytes());
        v.push(0);
        v
    }

    /// A small save with the player's NPC record and an inventory holding gold.
    pub fn synthetic(level: u16, gold: u32) -> Vec<u8> {
        let mut header = 1u32.to_le_bytes().to_vec();
        header.extend(bz("Tester"));
        header.extend(level.to_le_bytes());
        header.extend(bz("Cheydinhal"));
        header.extend([0u8; 4 + 4 + 16]);
        header.extend([0u8; 12]);
        let mut d = b"TES4SAVEGAME".to_vec();
        d.extend([0, 125]);
        d.extend([0u8; 16]);
        d.extend(125u32.to_le_bytes());
        d.extend((header.len() as u32).to_le_bytes());
        d.extend(header);
        d.push(1);
        d.push(12);
        d.extend(b"Oblivion.esm");
        d.extend(0u32.to_le_bytes()); // form table offset (unused here)
        d.extend(2u32.to_le_bytes()); // records
        d.extend([0u8; 32]);
        d.extend(1u16.to_le_bytes());
        d.extend([1, 0, 0, 0, 0, 0, 0x80, 0x3F]);
        for _ in 0..4 {
            d.extend(0u16.to_le_bytes());
        }
        d.extend([0u8; 4]);
        d.extend(0u32.to_le_bytes()); // created
        for _ in 0..4 {
            d.extend(0u16.to_le_bytes());
        }
        // NPC record: attributes, base data, health, skills.
        let mut npc = vec![];
        npc.extend([40u8, 41, 42, 43, 44, 45, 46, 47]);
        let mut acbs = vec![0u8; 16];
        acbs[10..12].copy_from_slice(&level.to_le_bytes());
        npc.extend(acbs);
        npc.extend([0x72, 0x01, 0, 0]);
        npc.extend((0..21u8).map(|i| 10 + i));
        let flags = (1u32 << 3) | (1 << 4) | (1 << 2) | (1 << 9);
        d.extend(7u32.to_le_bytes());
        d.push(35);
        d.extend(flags.to_le_bytes());
        d.push(125);
        d.extend((npc.len() as u16).to_le_bytes());
        d.extend(npc);
        // Player reference: some bytes, then the inventory.
        let mut pl = vec![0u8; 8];
        pl.extend(1u32.to_le_bytes()); // gold's item index is 1
        pl.extend(gold.to_le_bytes());
        pl.extend(0u32.to_le_bytes());
        pl.extend([0u8; 12]);
        d.extend(0x14u32.to_le_bytes());
        d.push(50);
        d.extend((1u32 << 27).to_le_bytes());
        d.push(125);
        d.extend((pl.len() as u16).to_le_bytes());
        d.extend(pl);
        d.extend(0u32.to_le_bytes()); // temp effects
        d.extend(3u32.to_le_bytes()); // form ids: null, gold, junk
        d.extend([0, 0, 0, 0, 0x0F, 0, 0, 0, 7, 0, 0, 0]);
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = synthetic(17, 4321);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/Level"), Some(&serde_json::json!(17)));
        assert_eq!(doc.view.pointer("/Character/Gold"), Some(&serde_json::json!(4321)));
        assert_eq!(doc.view.pointer("/Character/Base health"), Some(&serde_json::json!(370)));
        assert_eq!(doc.view.pointer("/Attributes/Speed"), Some(&serde_json::json!(44)));
        assert_eq!(doc.view.pointer("/Skills/Speechcraft"), Some(&serde_json::json!(30)));
        doc.set("/Character/Gold", serde_json::json!(500_000)).unwrap();
        doc.set("/Character/Level", serde_json::json!(30)).unwrap();
        doc.set("/Skills/Blade", serde_json::json!(100)).unwrap();
        assert!(doc.set("/Skills/Blade", serde_json::json!(300)).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Character/Gold"), Some(&serde_json::json!(500_000)));
        assert_eq!(back.view.pointer("/Character/Level"), Some(&serde_json::json!(30)));
        assert_eq!(back.view.pointer("/Skills/Blade"), Some(&serde_json::json!(100)));
        // The header's copy of the level follows (it is what the load screen shows).
        let mut c = Cur { d: &out, i: 38 + 4 };
        c.text().unwrap();
        assert_eq!(c.u16().unwrap(), 30);
    }

    #[test]
    fn refuses_junk_and_cut_off_saves() {
        assert!(parse(b"TES4SAVEGAME").is_err());
        let d = synthetic(5, 10);
        assert!(parse(&d[..d.len() / 2]).is_err());
        assert!(parse(b"not an oblivion save at all").is_err());
    }

    /// Research aid: SAVE_EDITOR_OB = an Oblivion `.ess`.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_OB").unwrap()).unwrap();
        let mut doc = parse(&d).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view["Character"]).unwrap());
        assert_eq!(doc.to_bytes().unwrap(), d, "an unedited save is written back byte for byte");
        doc.set("/Character/Gold", serde_json::json!(777_777)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(parse(&out).unwrap().view.pointer("/Character/Gold"), Some(&serde_json::json!(777_777)));
    }
}
