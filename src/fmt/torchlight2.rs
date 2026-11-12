//! Torchlight II characters (`.SVB`, also the game's `.restore` and `backup.tmp` copies).
//!
//! The file: u32 version (0x44), a byte (1 = scrambled), u32 checksum of the plain body,
//! the scrambled body, and u32 file size. Scrambling has no key: plain byte `i` is the high
//! nibble of byte `i` and the low nibble of the byte mirrored from the end, inverted unless
//! it is 0x00 or 0xFF. The checksum starts at 0x14D3 and does `c = c * 33 + byte`.
//! `unpack` / `pack` are the save's layer; this reader works on the plain body.
//!
//! The body: class, difficulty, hardcore, NG+, mod lists, then the hero block (appearance,
//! name, level, experience, fame, health, mana, unspent points, skills, spells, the four
//! stats, gold, then the items). Layout facts from Scepheo's tl2-save-edit (facts only).
//! Older saves (version 0x40) are laid out differently and aren't read.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

const VERSION: u32 = 0x44;
const DIFFICULTY: [&str; 4] = ["Casual", "Normal", "Veteran", "Elite"];

fn invert(b: u8) -> u8 {
    if b == 0 || b == 0xFF {
        b
    } else {
        b ^ 0xFF
    }
}

fn checksum(d: &[u8]) -> u32 {
    d.iter().fold(0x14D3u32, |c, &b| c.wrapping_mul(33).wrapping_add(u32::from(b)))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The plain body of a current (0x44) save whose size and checksum are right.
pub fn unpack(file: &[u8]) -> Option<Vec<u8>> {
    if file.len() < 13 || u32_at(file, 0)? != VERSION || file[4] != 1 || u32_at(file, file.len() - 4)? as usize != file.len() {
        return None;
    }
    let body = &file[9..file.len() - 4];
    let n = body.len();
    let plain: Vec<u8> = (0..n).map(|i| invert((body[i] >> 4) | (body[n - 1 - i] << 4))).collect();
    (checksum(&plain) == u32_at(file, 5)?).then_some(plain)
}

/// An older (0x40) save: recognised so the player can be told it isn't supported.
pub fn is_old(file: &[u8]) -> bool {
    file.len() > 16 && u32_at(file, 0) == Some(0x40) && file[4] == 1 && u32_at(file, file.len() - 4).is_some_and(|n| n as usize + 4 == file.len())
}

/// The save file for this plain body.
pub fn pack(plain: &[u8]) -> Vec<u8> {
    let n = plain.len();
    let mut body = vec![0u8; n];
    let (mut i, mut j) = (0usize, n.wrapping_sub(1));
    while n > 0 && i <= j {
        let (s, e) = (invert(plain[i]), invert(plain[j]));
        body[i] = (s << 4) | (e >> 4);
        body[j] = (s >> 4) | (e << 4);
        i += 1;
        if j == 0 {
            break;
        }
        j -= 1;
    }
    let mut out = Vec::with_capacity(n + 13);
    out.extend(VERSION.to_le_bytes());
    out.push(1);
    out.extend(checksum(plain).to_le_bytes());
    out.extend(body);
    out.extend(((n + 13) as u32).to_le_bytes());
    out
}

type R<T> = Result<T, String>;

struct Rd<'a> {
    d: &'a [u8],
    pos: usize,
}

impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> R<&'a [u8]> {
        let s = self.d.get(self.pos..self.pos + n).ok_or("the save ends early")?;
        self.pos += n;
        Ok(s)
    }
    fn i32(&mut self) -> R<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }
    fn i16(&mut self) -> R<i16> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn count(&mut self, max: i32) -> R<usize> {
        let n = self.i32()?;
        if !(0..=max).contains(&n) {
            return Err(format!("bad count {n} at {}", self.pos - 4));
        }
        Ok(n as usize)
    }
    /// i16 length, then UTF-16 text.
    fn text(&mut self) -> R<String> {
        let n = self.i16()?;
        if !(0..=1024).contains(&n) {
            return Err(format!("bad text length at {}", self.pos - 2));
        }
        let b = self.take(2 * n as usize)?;
        Ok(String::from_utf16_lossy(&b.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>()))
    }
    fn ff(&mut self, n: usize) -> R<()> {
        if self.take(n)?.iter().all(|&b| b == 0xFF) {
            Ok(())
        } else {
            Err(format!("marker missing at {}", self.pos - n))
        }
    }
}

/// Header and the start of the hero block, up to the name; returns the class, difficulty,
/// hardcore flag and NG+ cycle.
fn header(r: &mut Rd) -> R<(String, i32, usize, u8, i32)> {
    let class = r.text()?;
    if class.is_empty() || !class.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("not a Torchlight II character".into());
    }
    let diff_at = r.pos;
    let diff = r.i32()?;
    let hc = r.u8()?;
    let ng = r.i32()?;
    r.take(5)?;
    let n = r.count(10_000)?;
    r.take(n * 12)?;
    for _ in 0..3 {
        let n = r.count(10_000)?;
        r.take(n * 10)?;
    }
    let hero = r.pos;
    let len = r.count(i32::MAX)?;
    if hero + len > r.d.len() {
        return Err("hero block runs past the end".into());
    }
    r.take(11)?;
    r.ff(8)?;
    Ok((class, diff, diff_at, hc, ng))
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 400 && d[1] == 0 && (1..=64).contains(&d[0]) && header(&mut Rd { d, pos: 0 }).is_ok()
}

/// "Hum_Engineer_M" → "Engineer (male)".
fn class_name(s: &str) -> String {
    let parts: Vec<&str> = s.split('_').collect();
    match parts.as_slice() {
        [_, mid @ .., sex] if !mid.is_empty() && (*sex == "M" || *sex == "F") => {
            format!("{} ({})", mid.join(" "), if *sex == "M" { "male" } else { "female" })
        }
        _ => s.to_string(),
    }
}

fn modifier(r: &mut Rd) -> R<()> {
    let flags = r.i32()?;
    r.text()?;
    if flags & 0x100 != 0 {
        r.text()?;
    }
    if flags & 0x1000 != 0 {
        r.text()?;
    }
    if flags & 0x2000 != 0 {
        r.take(8)?;
    }
    let n = r.u8()? as usize;
    r.take(4 * n)?;
    let n = r.i16()?;
    if n < 0 {
        return Err("bad modifier".into());
    }
    r.take(12 * n as usize)?;
    r.take(32)?;
    if flags & 0x20000 != 0 {
        let n = r.u8()? as usize;
        r.take(2 * n)?;
    }
    Ok(())
}

/// One item: its name and where its stack size is.
fn item(r: &mut Rd) -> R<(String, usize)> {
    r.u8()?;
    r.take(8)?;
    let name = r.text()?;
    r.text()?;
    r.text()?;
    r.take(24)?;
    let n = r.count(1000)?;
    r.take(8 * n)?;
    r.take(29 + 8 + 95 + 4)?; // …, enchantments, stash position, …, level
    let stack = r.pos;
    r.take(8)?; // stack, sockets
    for _ in 0..r.count(100)? {
        item(r)?;
    }
    r.take(4 + 12 + 12)?;
    let n = r.i16()?;
    if n < 0 {
        return Err("bad item".into());
    }
    r.take(12 * n as usize)?;
    for _ in 0..3 {
        for _ in 0..r.count(1000)? {
            modifier(r)?;
        }
    }
    for _ in 0..r.count(1000)? {
        r.text()?;
    }
    let n = r.count(100_000)?;
    r.take(12 * n)?;
    Ok((name, stack))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut b = Builder::new(d);
    let mut r = Rd { d, pos: 0 };
    let (class, diff, diff_at, hc, ng) = header(&mut r)?;
    let hero_end = {
        // The hero block's length is the i32 just before the 11 + 8 bytes read last.
        let at = r.pos - 23;
        at + u32_at(d, at).unwrap_or(0) as usize
    };
    b.info(&["Hero", "Class"], class_name(&class));
    b.info(&["Hero", "Class id"], class);
    if (0..DIFFICULTY.len() as i32).contains(&diff) {
        b.named(&["Hero", "Difficulty"], diff_at, Kind::I32, &DIFFICULTY);
    }
    b.info(&["Hero", "Hardcore"], hc != 0);
    b.info(&["Hero", "New game plus"], ng);
    r.take(10 + 12 + 43 + 4 + 46)?;
    let name = r.text()?;
    b.info(&["Hero", "Name"], name);
    r.take(2)?;
    r.text()?;
    r.take(84)?;
    let i32_max = i32::MAX as i64;
    for (k, lo, hi) in [("Level", 1, 100), ("Experience", 0, i32_max), ("Fame level", 0, 100), ("Fame", 0, i32_max)] {
        b.ranged(&["Progress", k], r.pos, Kind::I32, lo, hi);
        r.take(4)?;
    }
    b.ranged(&["Health and mana", "Health"], r.pos, Kind::F32, 1, 1_000_000);
    r.take(4 + 4 + 4)?;
    b.ranged(&["Health and mana", "Mana"], r.pos, Kind::F32, 0, 1_000_000);
    r.take(4 + 4 + 12)?;
    let play = f32::from_le_bytes(r.take(4)?.try_into().unwrap());
    b.info(&["Hero", "Play time (seconds)"], play.round() as i64);
    r.take(4)?;
    for k in ["Skill points", "Attribute points"] {
        b.ranged(&["Points", k], r.pos, Kind::I32, 0, 10_000);
        r.take(4)?;
    }
    r.take(48)?;
    let skills = r.count(10_000)?;
    r.take(12 * skills)?;
    b.info(&["Hero", "Skills learned"], skills);
    for _ in 0..4 {
        let name = r.text()?;
        let level = r.i32()?;
        if !name.is_empty() {
            b.info(&["Spells", &name], level);
        }
    }
    r.take(28)?;
    for k in ["Strength", "Dexterity", "Vitality", "Focus"] {
        b.ranged(&["Stats", k], r.pos, Kind::I32, 0, 100_000);
        r.take(4)?;
    }
    b.ranged(&["Progress", "Gold"], r.pos, Kind::I32, 0, i32_max);
    r.take(4 + 4)?;
    r.ff(13)?;
    let n = r.count(100_000)?;
    r.take(8 * n)?;
    let mut items = vec![];
    let mut whole = true;
    for _ in 0..r.count(100_000)? {
        match item(&mut r) {
            Ok(it) if r.pos <= hero_end => items.push(it),
            _ => {
                whole = false;
                break;
            }
        }
    }
    // Item stacks only when every item was read (so each stack is where it should be).
    if whole {
        let mut seen: HashMap<String, usize> = HashMap::new();
        for (name, at) in items {
            let name = if name.trim().is_empty() { "Item".to_string() } else { name };
            let n = seen.entry(name.clone()).or_default();
            *n += 1;
            let label = if *n > 1 { format!("{name} ({n})") } else { name };
            b.ranged(&["Items", &label], at, Kind::I32, 1, 100_000);
        }
    }
    Ok(b.finish("Torchlight II character", Checksum::None).with_note(
        "Level, experience, fame, gold, unspent points, the four stats, health, mana and item stacks. The save is \
         scrambled again and its checksum updated when you save.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(d: &mut Vec<u8>, s: &str) {
        let u: Vec<u16> = s.encode_utf16().collect();
        d.extend((u.len() as i16).to_le_bytes());
        d.extend(u.iter().flat_map(|c| c.to_le_bytes()));
    }

    fn item(d: &mut Vec<u8>, name: &str, stack: i32) {
        d.push(2);
        d.extend([0x11; 8]);
        text(d, name);
        text(d, "");
        text(d, "");
        d.extend([0; 24]);
        d.extend(0i32.to_le_bytes());
        d.extend([0; 29 + 8 + 95]);
        d.extend(5i32.to_le_bytes());
        d.extend(stack.to_le_bytes());
        d.extend(0i32.to_le_bytes());
        d.extend(0i32.to_le_bytes()); // gems
        d.extend([0; 28]);
        d.extend(0i16.to_le_bytes());
        d.extend([0; 4 * 3]); // modifier lists
        d.extend(0i32.to_le_bytes()); // augments
        d.extend(0i32.to_le_bytes());
    }

    /// A synthetic plain body.
    pub fn sample() -> Vec<u8> {
        let mut d = vec![];
        text(&mut d, "Hum_Engineer_M");
        d.extend(2i32.to_le_bytes());
        d.push(0);
        d.extend(0i32.to_le_bytes());
        d.extend([0; 5]);
        d.extend(0i32.to_le_bytes());
        for _ in 0..3 {
            d.extend(0i32.to_le_bytes());
        }
        let hero = d.len();
        d.extend(0i32.to_le_bytes());
        d.extend([0; 11]);
        d.extend([0xFF; 8]);
        d.extend([0; 10 + 12 + 43 + 4 + 46]);
        text(&mut d, "Tester");
        d.extend([0; 2]);
        text(&mut d, "PLAYER");
        d.extend([0; 84]);
        for v in [20, 12345, 3, 999] {
            d.extend((v as i32).to_le_bytes());
        }
        d.extend(500.0f32.to_le_bytes());
        d.extend([0; 8]);
        d.extend(120.0f32.to_le_bytes());
        d.extend([0; 4 + 12]);
        d.extend(3600.0f32.to_le_bytes());
        d.extend([0; 4]);
        d.extend(4i32.to_le_bytes());
        d.extend(5i32.to_le_bytes());
        d.extend([0; 48]);
        d.extend(1i32.to_le_bytes());
        d.extend([7; 8]);
        d.extend(2i32.to_le_bytes());
        for s in ["HEAL SELF", "", "", ""] {
            text(&mut d, s);
            d.extend(1i32.to_le_bytes());
        }
        d.extend([0; 28]);
        for v in [10, 20, 30, 40, 777] {
            d.extend((v as i32).to_le_bytes());
        }
        d.extend([0; 4]);
        d.extend([0xFF; 13]);
        d.extend(0i32.to_le_bytes());
        d.extend(2i32.to_le_bytes());
        item(&mut d, "Health Potion", 5);
        item(&mut d, "Health Potion", 1);
        d.extend([0; 40]);
        let len = (d.len() - hero) as i32;
        d[hero..hero + 4].copy_from_slice(&len.to_le_bytes());
        d.extend([0; 300]);
        d
    }

    #[test]
    fn scrambles_and_reads() {
        let plain = sample();
        let file = pack(&plain);
        assert_eq!(unpack(&file).unwrap(), plain);
        let mut bad = file.clone();
        bad[20] ^= 0x10;
        assert!(unpack(&bad).is_none());
        // Odd and even lengths both survive.
        for n in [0usize, 1, 2, 7] {
            let p: Vec<u8> = (0..n as u8).map(|i| i.wrapping_mul(37)).collect();
            assert_eq!(unpack(&pack(&p)).unwrap(), p);
        }
        assert!(looks(&plain));
        let mut doc = parse(&plain).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Hero/Class"), Some(&"Engineer (male)".into()));
        assert_eq!(v.pointer("/Hero/Name"), Some(&"Tester".into()));
        assert_eq!(v.pointer("/Hero/Difficulty"), Some(&"Veteran".into()));
        assert_eq!(v.pointer("/Progress/Level"), Some(&20.into()));
        assert_eq!(v.pointer("/Progress/Gold"), Some(&777.into()));
        assert_eq!(v.pointer("/Stats/Focus"), Some(&40.into()));
        assert_eq!(v.pointer("/Health and mana/Mana"), Some(&120.0.into()));
        assert_eq!(v.pointer("/Points/Attribute points"), Some(&5.into()));
        assert_eq!(v.pointer("/Items/Health Potion"), Some(&5.into()));
        assert_eq!(v.pointer("/Items/Health Potion (2)"), Some(&1.into()));
        doc.set("/Progress/Gold", 778.into()).unwrap();
        doc.set("/Items/Health Potion", 9.into()).unwrap();
        let out = pack(&doc.to_bytes().unwrap());
        let again = parse(&unpack(&out).unwrap()).unwrap();
        assert_eq!(again.view.pointer("/Progress/Gold"), Some(&778.into()));
        assert_eq!(again.view.pointer("/Items/Health Potion"), Some(&9.into()));
        assert_eq!(again.view.pointer("/Stats"), doc.view.pointer("/Stats"));
    }

    /// SAVE_EDITOR_TL2 = a folder of .svb (and .restore / backup.tmp) files.
    #[test]
    #[ignore]
    fn real_saves() {
        let want = |n: &str| {
            let n = n.to_lowercase();
            n.ends_with(".svb") || n.ends_with(".restore") || n == "backup.tmp"
        };
        crate::fmt::grimdawn::tests::research("SAVE_EDITOR_TL2", want, "Torchlight II character", &["/Progress/Gold", "/Stats/Strength", "/Health and mana/Mana", "/Progress/Level"]);
    }
}
