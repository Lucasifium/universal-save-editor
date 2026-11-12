//! Ori and the Blind Forest (Definitive Edition and the original) `saveFile<n>.sav`.
//!
//! A .NET `BinaryWriter` file: "OriSave", version, a header the load menu shows (area name,
//! completion, health, max health, energy, max energy, hours, minutes, seconds, world
//! progression, completed, scene id, debug, order, difficulty, was killed, completed with
//! everything, lowest difficulty, trial), then "SaveGameData", a data version and every
//! scene: a 16-byte id and its objects, each a 16-byte id and a length-prefixed blob. Ori's
//! own state lives in the master scene (all-zero id). Object ids and blob layouts as used by
//! LiveSplit.OriDE's save manager (credited in THIRD_PARTY_NOTICES.md). Nothing is
//! checksummed; the header's health and energy are written again from the edited values.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Ori and the Blind Forest save";

/// Object ids, written as LiveSplit.OriDE writes them (two 64-bit halves in hex).
const SEIN_LEVEL: &str = "44D72845AB790E021CC526C2EEAC82A5";
const SEIN_INVENTORY: &str = "444B746E7F7A0AE0CC208C40DE7CD9B1";
const ABILITIES: &str = "4B417974465D2E6BE06A6D8DBD0F0BB3";
const HEALTH: &str = "4544D2E4CB117A965E43AAEB8BFE26AA";
const ENERGY: &str = "4DE917EC7C3072427FEBCC44A35880B9";
const DEATHS: &str = "48B63153499A49ACEFDD874FE2C88CBC";

/// One byte each, in this order.
const ABILITY_NAMES: [&str; 43] = [
    "Bash", "Charge Flame", "Wall Jump", "Stomp", "Double Jump", "Charge Jump", "Magnet", "Ultra Magnet", "Climb", "Glide",
    "Spirit Flame", "Rapid Flame", "Spirit Efficiency", "Water Breath", "Charge Flame Blast", "Charge Flame Burn", "Triple Jump",
    "Bash Buff", "Ultra Defense", "Life Efficiency", "Sense", "Ultra Stomp", "Quick Flame", "Map Markers", "Energy Efficiency",
    "Life Markers", "Energy Markers", "Ability Markers", "Rekindle", "Regroup", "Charge Flame Efficiency", "Ultra Soul Link",
    "Soul Link Efficiency", "Split Flame", "Spark Flame", "Cinder Flame", "Ultra Split Flame", "Dash", "Light Burst",
    "Ultra Light Burst", "Charge Dash", "Air Dash", "Light Burst Efficiency",
];

/// Skills found in the world (the rest come from the ability tree).
const SKILLS: [usize; 11] = [10, 2, 1, 4, 0, 3, 9, 8, 5, 37, 38];

fn guid(hex: &str) -> [u8; 16] {
    let l = u64::from_str_radix(&hex[..16], 16).unwrap_or(0);
    let r = u64::from_str_radix(&hex[16..], 16).unwrap_or(0);
    let mut g = [0u8; 16];
    g[..8].copy_from_slice(&l.to_le_bytes());
    g[8..].copy_from_slice(&r.to_le_bytes());
    g
}

struct Cur<'a> {
    d: &'a [u8],
    p: usize,
}

impl Cur<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.d.get(self.p..self.p.checked_add(n)?)?;
        self.p += n;
        Some(s)
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn str(&mut self) -> Option<&[u8]> {
        let (mut n, mut sh) = (0usize, 0);
        loop {
            let b = *self.take(1)?.first()?;
            n |= usize::from(b & 0x7F) << sh;
            sh += 7;
            if b < 0x80 {
                break;
            }
            if sh > 28 {
                return None;
            }
        }
        self.take(n)
    }
}

/// Where things are: header fields and the master scene's objects (offset, length).
struct Layout {
    area: String,
    /// Offset of the header's first int (completion).
    head: usize,
    objects: Vec<([u8; 16], usize, usize)>,
}

impl Layout {
    fn obj(&self, hex: &str, min: usize) -> Option<usize> {
        let g = guid(hex);
        self.objects.iter().find(|(id, _, len)| *id == g && *len >= min).map(|o| o.1)
    }
}

fn layout(d: &[u8]) -> Option<Layout> {
    let mut c = Cur { d, p: 0 };
    if c.str()? != b"OriSave" {
        return None;
    }
    let version = c.i32()?;
    if !(10..=20).contains(&version) {
        return None;
    }
    let area = String::from_utf8_lossy(c.str()?).into_owned();
    let head = c.p;
    // 9 ints, completed, scene id, debug, order, difficulty, was killed, everything, lowest, trial.
    c.take(36 + 1 + 16 + 1 + 4 + 4 + 1 + 1 + 4 + 1)?;
    if c.str()? != b"SaveGameData" {
        return None;
    }
    c.i32()?;
    let scenes = c.i32()?;
    let mut objects = vec![];
    for _ in 0..scenes.max(0) {
        let master = c.take(16)?.iter().all(|&b| b == 0);
        let n = c.i32()?;
        for _ in 0..n.max(0) {
            let id: [u8; 16] = c.take(16)?.try_into().ok()?;
            let len = usize::try_from(c.i32()?).ok()?;
            let at = c.p;
            c.take(len)?;
            if master {
                objects.push((id, at, len));
            }
        }
    }
    (c.p == d.len()).then_some(Layout { area, head, objects })
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"\x07OriSave") && layout(d).is_some()
}

fn f32at(d: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

/// The load menu's copy of health and energy (whole cells), from the edited values.
fn fix_header(d: &mut Vec<u8>) {
    let Some(l) = layout(d) else { return };
    let mut vals: Vec<(usize, i32)> = vec![];
    if let Some(h) = l.obj(HEALTH, 8) {
        vals.push((1, (f32at(d, h) / 4.0).round() as i32));
        vals.push((2, i32::from_le_bytes(d[h + 4..h + 8].try_into().unwrap()) / 4));
    }
    if let Some(e) = l.obj(ENERGY, 8) {
        vals.push((3, f32at(d, e).round() as i32));
        vals.push((4, f32at(d, e + 4).round() as i32));
    }
    for (i, v) in vals {
        let at = l.head + 4 * i;
        d[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let l = layout(d).ok_or("not an Ori and the Blind Forest save")?;
    let mut b = Builder::new(d);
    let i32at = |at: usize| i32::from_le_bytes(d[at..at + 4].try_into().unwrap());
    b.info(&["Save", "Area"], l.area.clone());
    b.info(&["Save", "Completion (%)"], i32at(l.head));
    b.info(&["Save", "Play time"], format!("{}:{:02}:{:02}", i32at(l.head + 20), i32at(l.head + 24), i32at(l.head + 28)));
    if let Some(o) = l.obj(SEIN_LEVEL, 12) {
        b.ranged(&["Ori", "Level"], o, Kind::I32, 0, 999);
        b.ranged(&["Ori", "Experience"], o + 4, Kind::I32, 0, 9_999_999);
        b.ranged(&["Ori", "Ability points"], o + 8, Kind::I32, 0, 999);
    }
    if let Some(o) = l.obj(HEALTH, 8) {
        b.ranged(&["Ori", "Life (4 per cell)"], o, Kind::F32Int, 0, 80);
        b.ranged(&["Ori", "Max life (4 per cell)"], o + 4, Kind::I32, 4, 80);
    }
    if let Some(o) = l.obj(ENERGY, 8) {
        b.ranged(&["Ori", "Energy"], o, Kind::F32Int, 0, 20);
        b.ranged(&["Ori", "Max energy"], o + 4, Kind::F32Int, 0, 20);
    }
    if let Some(o) = l.obj(SEIN_INVENTORY, 8) {
        b.ranged(&["Ori", "Keystones"], o, Kind::I32, 0, 99);
        b.ranged(&["Ori", "Mapstones"], o + 4, Kind::I32, 0, 99);
    }
    if let Some(o) = l.obj(DEATHS, 4) {
        b.ranged(&["Ori", "Deaths"], o, Kind::I32, 0, 999_999);
    }
    if let Some(o) = l.obj(ABILITIES, ABILITY_NAMES.len()) {
        for &i in &SKILLS {
            b.field(&["Skills", ABILITY_NAMES[i]], o + i, Kind::B8);
        }
        for (i, n) in ABILITY_NAMES.iter().enumerate() {
            if !SKILLS.contains(&i) && i != 17 && i != 42 {
                b.field(&["Ability tree", n], o + i, Kind::B8);
            }
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix_header)).with_note(
        "Close the game first. Life counts quarters: 4 is one cell (the game's most is 15 cells, 60). The load menu's copy of life and energy is written again for you.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn s(out: &mut Vec<u8>, t: &[u8]) {
        out.push(t.len() as u8);
        out.extend_from_slice(t);
    }

    fn sample() -> Vec<u8> {
        let mut d = vec![];
        s(&mut d, b"OriSave");
        d.extend_from_slice(&13i32.to_le_bytes());
        s(&mut d, b"sunkenGlades");
        for v in [40, 3, 3, 1, 1, 0, 5, 9, 10] {
            d.extend_from_slice(&(v as i32).to_le_bytes());
        }
        d.extend_from_slice(&[0u8; 1 + 16 + 1 + 4 + 4 + 1 + 1 + 4 + 1]);
        s(&mut d, b"SaveGameData");
        d.extend_from_slice(&1i32.to_le_bytes());
        d.extend_from_slice(&1i32.to_le_bytes());
        d.extend_from_slice(&[0u8; 16]);
        let objs: Vec<(&str, Vec<u8>)> = vec![
            (SEIN_LEVEL, [2i32, 100, 1].iter().flat_map(|v| v.to_le_bytes()).chain([1]).collect()),
            (HEALTH, 12f32.to_le_bytes().into_iter().chain(12i32.to_le_bytes()).collect()),
            (ENERGY, 1f32.to_le_bytes().into_iter().chain(1f32.to_le_bytes()).collect()),
            (ABILITIES, vec![0u8; 43]),
        ];
        d.extend_from_slice(&(objs.len() as i32).to_le_bytes());
        for (id, blob) in objs {
            d.extend_from_slice(&guid(id));
            d.extend_from_slice(&(blob.len() as i32).to_le_bytes());
            d.extend_from_slice(&blob);
        }
        d
    }

    #[test]
    fn reads_edits_and_fixes_header() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Ori/Level"), Some(&Value::from(2)));
        assert_eq!(doc.view.pointer("/Ori/Max life (4 per cell)"), Some(&Value::from(12)));
        assert_eq!(doc.view.pointer("/Skills/Bash"), Some(&Value::Bool(false)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Ori/Max life (4 per cell)", Value::from(40)).unwrap();
        doc.set("/Ori/Life (4 per cell)", Value::from(40)).unwrap();
        doc.set("/Ori/Max energy", Value::from(7)).unwrap();
        doc.set("/Skills/Bash", Value::Bool(true)).unwrap();
        let out = doc.to_bytes().unwrap();
        let l = layout(&out).unwrap();
        let hd = |i: usize| i32::from_le_bytes(out[l.head + 4 * i..l.head + 4 * i + 4].try_into().unwrap());
        assert_eq!((hd(1), hd(2), hd(4)), (10, 10, 7));
        assert_eq!(out[l.obj(ABILITIES, 43).unwrap()], 1);
        assert!(!looks(&d[..d.len() - 1]));
    }
}
