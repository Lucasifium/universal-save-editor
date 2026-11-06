//! Fallout 3 and Fallout: New Vegas saves (`.fos`, magic `FO3SAVEGAME`).
//!
//! A header (save number, character name, level, place, play time) and a screenshot, the
//! plugin list, a table of section offsets, then the "changed forms": one record per game
//! object that differs from the master files. Nearly every field is followed by a `|` byte.
//! Three records are read here: the player's reference (form 0x14), which holds the actor
//! value arrays (experience, karma) and the inventory (bottle caps), and the player's base
//! form (0x7), which holds S.P.E.C.I.A.L.
//!
//! Everything is a fixed-size value changed in place; there is no checksum. Layout facts
//! from the NVForever project's clean-room save notes (facts only) and xEdit's save
//! definitions (facts only), checked on real Fallout 3 and New Vegas saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Fallout 3 / New Vegas save";
const MAGIC: &[u8] = b"FO3SAVEGAME";
const PLAYER_REF: u32 = 0x14;
const PLAYER_BASE: u32 = 0x07;
const CAPS: u32 = 0x0F;
const ACHR: u8 = 1;
const NPC: u8 = 10;
/// Actor value codes in the second array.
const AV_KARMA: usize = 23;
const AV_XP: usize = 24;

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(MAGIC)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

/// A cursor over the `|`-separated header fields.
struct Cur<'a> {
    d: &'a [u8],
    o: usize,
}

impl Cur<'_> {
    fn pipe(&mut self) -> Option<()> {
        (*self.d.get(self.o)? == 0x7C).then(|| self.o += 1)
    }
    fn u32(&mut self) -> Option<u32> {
        let v = u32_at(self.d, self.o)?;
        self.o += 4;
        self.pipe()?;
        Some(v)
    }
    fn text(&mut self) -> Option<String> {
        let n = u16::from_le_bytes(self.d.get(self.o..self.o + 2)?.try_into().ok()?) as usize;
        self.o += 2;
        self.pipe()?;
        let s = self.d.get(self.o..self.o.checked_add(n)?)?;
        self.o += n;
        self.pipe()?;
        // Saves from localised games hold single-byte text; show it byte for byte.
        Some(s.iter().map(|&b| b as char).collect())
    }
}

/// The form ID a 3-byte reference points at.
fn resolve(r: [u8; 3], fids: &[u32]) -> u32 {
    let v = (((r[0] & 0x3F) as u32) << 16) | ((r[1] as u32) << 8) | r[2] as u32;
    match r[0] >> 6 {
        0 => v.checked_sub(1).and_then(|i| fids.get(i as usize)).copied().unwrap_or(0),
        1 => v,
        _ => 0xFF00_0000 | v,
    }
}

struct Record {
    flags: u32,
    at: usize,
    len: usize,
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    parse_inner(d).ok_or_else(|| "not a readable Fallout 3 / New Vegas save".to_string())
}

fn parse_inner(d: &[u8]) -> Option<TreeDoc> {
    if !looks(d) {
        return None;
    }
    let hs = u32_at(d, 0x0B)? as usize;
    let mut c = Cur { d, o: 0x0F };
    c.u32()?;
    // New Vegas adds a 64-byte language field here; Fallout 3 goes straight to the width.
    let fnv = *d.get(c.o + 4)? != 0x7C;
    if fnv {
        c.o += 64;
        c.pipe()?;
    }
    let (w, h) = (c.u32()? as usize, c.u32()? as usize);
    let save_no = c.u32()?;
    let name = c.text()?;
    let karma_title = c.text()?;
    let level = c.u32()?;
    let place = c.text()?;
    let played = c.text()?;
    if c.o != 0x0F + hs || w > 4096 || h > 4096 {
        return None;
    }
    // Screenshot, form version, plugin block, then the table of section offsets.
    let mut o = 0x0F + hs + w * h * 3 + 1;
    o += 4 + u32_at(d, o)? as usize;
    let tab = |i: usize| u32_at(d, o + 4 * i).map(|v| v as usize);
    let (fid_off, forms_off, forms_end, forms_n) = (tab(0)?, tab(3)?, tab(4)?, tab(7)?);
    let n_fids = u32_at(d, fid_off)? as usize;
    let fids: Vec<u32> = (0..n_fids).map(|i| u32_at(d, fid_off + 4 + 4 * i)).collect::<Option<_>>()?;
    // Walk the changed forms for the two records needed.
    let (mut player, mut base) = (None, None);
    let mut p = forms_off;
    for _ in 0..forms_n {
        let r: [u8; 3] = d.get(p..p + 3)?.try_into().ok()?;
        let flags = u32_at(d, p + 3)?;
        let ty = *d.get(p + 7)?;
        let (width, mut at) = (ty >> 6, p + 9);
        let len = match width {
            0 => *d.get(at)? as usize,
            1 => u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize,
            _ => u32_at(d, at)? as usize,
        };
        at += [1, 2, 4][width.min(2) as usize];
        if at.checked_add(len)? > d.len() {
            return None;
        }
        match (resolve(r, &fids), ty & 63) {
            (PLAYER_REF, ACHR) => player = Some(Record { flags, at, len }),
            (PLAYER_BASE, NPC) => base = Some(Record { flags, at, len }),
            _ => {}
        }
        p = at + len;
    }
    if p != forms_end {
        return None;
    }
    let player = player?;

    let mut b = Builder::new(d);
    b.info(&["Character", "Game"], if fnv { "Fallout: New Vegas" } else { "Fallout 3" });
    b.info(&["Character", "Name"], name);
    b.info(&["Character", "Level"], level);
    b.info(&["Character", "Karma title"], karma_title);
    b.info(&["Character", "Place"], place);
    b.info(&["Character", "Play time"], played);
    b.info(&["Character", "Save number"], save_no);

    // The player's record: a place (reference + six floats), then three arrays of actor
    // values (77 in New Vegas, 73 in Fallout 3) of f32 + `|`. The second holds the
    // permanent changes: experience and karma.
    let n_av = if fnv { 77 } else { 73 };
    let av = player.at + 28;
    let slot = |array: usize, code: usize| av + (array * n_av + code) * 5;
    if player.len >= 28 + 3 * n_av * 5 && d[slot(0, 0) + 4] == 0x7C && d[slot(2, n_av - 1) + 4] == 0x7C {
        b.ranged(&["Character", "Experience"], slot(1, AV_XP), Kind::F32Int, 0, 10_000_000);
        b.ranged(&["Character", "Karma"], slot(1, AV_KARMA), Kind::F32Int, -1000, 1000);
    }
    // Bottle caps are the inventory entry for form 0x0F: reference, `|`, i32, `|`. The
    // reference can be written two ways; accept the one match inside the player's record.
    let body = &d[player.at..player.at + player.len];
    let from = 28 + 3 * n_av * 5;
    let mut refs = vec![[0x40u8, 0, 0x0F]];
    refs.extend(fids.iter().enumerate().filter(|(_, &f)| f == CAPS).map(|(i, _)| {
        let v = i as u32 + 1;
        [(v >> 16) as u8 & 0x3F, (v >> 8) as u8, v as u8]
    }));
    let hits: Vec<usize> = (from..body.len().saturating_sub(8))
        .filter(|&i| refs.iter().any(|r| body[i..i + 3] == r[..]) && body[i + 3] == 0x7C && body[i + 8] == 0x7C)
        .collect();
    if let [i] = hits[..] {
        b.ranged(&["Character", "Bottle caps"], player.at + i + 4, Kind::I32, 0, i32::MAX as i64);
    }

    // S.P.E.C.I.A.L. in the player's base form: base data (24 bytes), the factions
    // (count, then a reference each), a byte, then the seven values.
    if let Some(base) = base.filter(|r| r.flags & 0b110 == 0b110) {
        let special = (|| {
            let mut c = Cur { d, o: base.at + 24 };
            c.pipe()?;
            let first = *d.get(c.o)?;
            let (n, w) = match first & 3 {
                0 => (first as usize >> 2, 1),
                1 => (u16::from_le_bytes(d.get(c.o..c.o + 2)?.try_into().ok()?) as usize >> 2, 2),
                _ => return None,
            };
            c.o += w;
            c.pipe()?;
            for _ in 0..n {
                c.o += 3;
                c.pipe()?;
            }
            c.o += 1;
            c.pipe()?;
            let at = c.o;
            let vals = d.get(at..at + 7)?;
            (vals.iter().all(|v| (1..=10).contains(v)) && *d.get(at + 7)? == 0x7C && at + 8 <= base.at + base.len).then_some(at)
        })();
        if let Some(at) = special {
            for (i, n) in ["Strength", "Perception", "Endurance", "Charisma", "Intelligence", "Agility", "Luck"].iter().enumerate() {
                b.ranged(&["S.P.E.C.I.A.L.", n], at + i, Kind::U8, 1, 10);
            }
        }
    }
    Some(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn pipe(v: &mut Vec<u8>) {
        v.push(0x7C);
    }
    fn u32p(v: &mut Vec<u8>, x: u32) {
        v.extend(x.to_le_bytes());
        pipe(v);
    }
    fn text(v: &mut Vec<u8>, s: &str) {
        v.extend((s.len() as u16).to_le_bytes());
        pipe(v);
        v.extend(s.as_bytes());
        pipe(v);
    }

    /// A small New Vegas-shaped save: a 1×1 screenshot, one plugin, the player's reference
    /// (experience, karma, caps) and the player's base form (S.P.E.C.I.A.L.).
    fn synthetic() -> Vec<u8> {
        let mut h = vec![];
        u32p(&mut h, 48);
        h.extend([b'E', b'N', b'G', b'L', b'I', b'S', b'H']);
        h.resize(h.len() + 57, 0);
        pipe(&mut h);
        for x in [1, 1, 7] {
            u32p(&mut h, x);
        }
        text(&mut h, "Courier");
        text(&mut h, "Drifter");
        u32p(&mut h, 12);
        text(&mut h, "Goodsprings");
        text(&mut h, "001.02.03");
        let mut d = b"FO3SAVEGAME".to_vec();
        d.extend((h.len() as u32).to_le_bytes());
        d.extend(&h);
        d.extend([1, 2, 3]); // screenshot
        d.push(27);
        let mut plug = vec![1];
        pipe(&mut plug);
        plug.extend(5u16.to_le_bytes());
        pipe(&mut plug);
        plug.extend(b"a.esm");
        pipe(&mut plug);
        d.extend((plug.len() as u32).to_le_bytes());
        d.extend(&plug);
        let table_at = d.len();
        d.extend([0u8; 32 + 78]);
        // Changed forms: the player's reference ...
        let forms_off = d.len();
        let mut ach = vec![0x40, 0, 0x14];
        ach.extend([0u8; 20]);
        ach.extend(0f32.to_le_bytes());
        pipe(&mut ach);
        for array in 0..3 {
            for code in 0..77 {
                let v: f32 = match (array, code) {
                    (1, AV_XP) => 5000.0,
                    (1, AV_KARMA) => 250.0,
                    _ => 0.0,
                };
                ach.extend(v.to_le_bytes());
                pipe(&mut ach);
            }
        }
        ach.extend([0, 0, 0, 0, 0x7C, 0x00, 0x7C]);
        ach.extend([0x40, 0, 0x0F, 0x7C]);
        ach.extend(1234i32.to_le_bytes());
        ach.push(0x7C);
        d.extend([0x40, 0, 0x14]);
        d.extend(0u32.to_le_bytes());
        d.extend([1 | (1 << 6), 27]);
        d.extend((ach.len() as u16).to_le_bytes());
        d.extend(&ach);
        // ... and the player's base form.
        let mut npc = vec![0u8; 24];
        pipe(&mut npc);
        npc.push(4); // one faction
        pipe(&mut npc);
        npc.extend([0x40, 0x12, 0x34]);
        pipe(&mut npc);
        npc.push(0);
        pipe(&mut npc);
        npc.extend([5, 6, 5, 7, 7, 5, 5]);
        pipe(&mut npc);
        d.extend([0x40, 0, 0x07]);
        d.extend(0b110u32.to_le_bytes());
        d.extend([10, 27, npc.len() as u8]);
        d.extend(&npc);
        let forms_end = d.len();
        let fid_off = d.len();
        d.extend(0u32.to_le_bytes());
        let tail = d.len();
        d.extend([5, 0, 0, 0, 0, 0, 0, 0, 0x7C]);
        for (i, v) in [fid_off, tail, 0, forms_off, forms_end, 0, 0, 2].iter().enumerate() {
            d[table_at + 4 * i..table_at + 4 * i + 4].copy_from_slice(&(*v as u32).to_le_bytes());
        }
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = synthetic();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/Name"), Some(&Value::from("Courier")));
        assert_eq!(doc.view.pointer("/Character/Level"), Some(&Value::from(12)));
        assert_eq!(doc.view.pointer("/Character/Game"), Some(&Value::from("Fallout: New Vegas")));
        assert_eq!(doc.view.pointer("/Character/Experience"), Some(&Value::from(5000)));
        assert_eq!(doc.view.pointer("/Character/Karma"), Some(&Value::from(250)));
        assert_eq!(doc.view.pointer("/Character/Bottle caps"), Some(&Value::from(1234)));
        assert_eq!(doc.view.pointer("/S.P.E.C.I.A.L./Intelligence"), Some(&Value::from(7)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Character/Bottle caps", Value::from(99999)).unwrap();
        doc.set("/S.P.E.C.I.A.L./Luck", Value::from(10)).unwrap();
        doc.set("/Character/Experience", Value::from(60000)).unwrap();
        assert!(doc.set("/S.P.E.C.I.A.L./Luck", Value::from(11)).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Character/Bottle caps"), Some(&Value::from(99999)));
        assert_eq!(again.view.pointer("/S.P.E.C.I.A.L./Luck"), Some(&Value::from(10)));
        assert_eq!(again.view.pointer("/Character/Experience"), Some(&Value::from(60000)));
        assert_eq!(again.view.pointer("/Character/Name"), Some(&Value::from("Courier")));
        assert!(parse(&d[..d.len() - 20]).is_err());
    }

    /// Research aid: SAVE_EDITOR_FOS = `.fos` files separated by `|`. Each must read, write
    /// back byte for byte and take an edit that reads back.
    #[test]
    #[ignore]
    fn real_files() {
        for path in std::env::var("SAVE_EDITOR_FOS").unwrap().split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            let mut doc = match parse(&d) {
                Ok(t) => t,
                Err(e) => {
                    println!("{path}: {e}");
                    continue;
                }
            };
            println!("{}", serde_json::to_string(&doc.view).unwrap().chars().take(400).collect::<String>());
            assert_eq!(doc.to_bytes().unwrap(), d, "round trip");
            let caps = doc.view.pointer("/Character/Bottle caps").and_then(Value::as_i64);
            if let Some(c) = caps {
                doc.set("/Character/Bottle caps", Value::from(c + 1)).unwrap();
                let out = doc.to_bytes().unwrap();
                assert_eq!(parse(&out).unwrap().view.pointer("/Character/Bottle caps"), Some(&Value::from(c + 1)));
            }
        }
    }
}
