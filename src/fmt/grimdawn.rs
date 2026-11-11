//! Grim Dawn characters (`player.gdc`).
//!
//! The file is a rolling-XOR stream: the first word is the seed (masked with 0x55555555);
//! a 256-word table is made from it, and every ciphertext byte read changes the key
//! (`key ^= table[byte]`). A 32-bit value is XORed with the whole key, a single byte with
//! its low byte. Blocks are an id, a length (XORed but not fed to the key), the payload and
//! a checksum: the key at that point, stored as is. Nested blocks (bags, stash pages) work
//! the same way, and the header ends with a bare checksum.
//!
//! Since the key depends on every byte before it, an edit changes all that follows: the
//! layer (`decrypt` / `encrypt`) walks the whole file once, remembering which bytes were
//! read as words, which as bytes and where the lengths and checksums are, so the edited
//! data can be encrypted again with the same seed and fresh checksums. The reader then
//! works on the plain data (same layout, starting at the "GDCX" magic).
//!
//! Block layouts ported from kraken444444's grim-dawn-save-editor (MIT), with the Fangs of
//! Asterkarn item, stash-tab, item-skill and hotbar details from gd-edit (facts only); cipher
//! as in GDParser (Chris Elison, MIT).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use serde_json::Value;
use std::collections::HashMap;

const SEED_MASK: u32 = 0x5555_5555;
const MAGIC: u32 = u32::from_le_bytes(*b"GDCX");
const DIFFICULTY: [&str; 3] = ["Normal", "Elite", "Ultimate"];

/// How a run of the plain data is encrypted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seg {
    /// This many 32-bit values.
    Words(u32),
    /// This many single bytes.
    Bytes(u32),
    /// A block length: XORed with the key, but not fed to it.
    Len,
    /// A checksum: the key itself, unencrypted.
    Sum,
}

fn table(seed: u32) -> Box<[u32; 256]> {
    let mut t = Box::new([0u32; 256]);
    let mut v = seed;
    for e in t.iter_mut() {
        v = v.rotate_right(1).wrapping_mul(39_916_801);
        *e = v;
    }
    t
}

type R<T> = Result<T, String>;

/// What the walk found: values for the view.
enum Rec {
    Info(Vec<String>, Value),
    Field(Vec<String>, usize, Kind, i64, i64),
    /// Write the field at this path to `at` as well.
    Mirror(Vec<String>, usize),
}

/// Reads either the encrypted file (building the plain data and the plan as it goes) or
/// the plain data.
struct Cur<'a> {
    d: &'a [u8],
    pos: usize,
    /// Offset of the plain data in `d` (4 for the encrypted file: the seed comes first).
    base: usize,
    crypt: Option<(Box<[u32; 256]>, u32)>,
    plain: Vec<u8>,
    plan: Vec<Seg>,
    recs: Vec<Rec>,
}

fn p(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| s.to_string()).collect()
}

impl<'a> Cur<'a> {
    fn need(&self, n: usize) -> R<&'a [u8]> {
        self.d
            .get(self.pos..self.pos + n)
            .ok_or_else(|| format!("the save ends early (at {})", self.pos))
    }
    /// Where the next value is in the plain data.
    fn at(&self) -> usize {
        self.pos - self.base
    }
    fn left(&self) -> usize {
        self.d.len() - self.pos
    }
    fn seg(&mut self, s: Seg) {
        match (self.plan.last_mut(), s) {
            (Some(Seg::Words(n)), Seg::Words(m)) => *n += m,
            (Some(Seg::Bytes(n)), Seg::Bytes(m)) => *n += m,
            _ => self.plan.push(s),
        }
    }
    fn word(&mut self, absorb: bool, seg: Seg) -> R<u32> {
        let raw = self.need(4)?;
        let c = u32::from_le_bytes(raw.try_into().unwrap());
        self.pos += 4;
        let Some((t, k)) = &mut self.crypt else {
            return Ok(c);
        };
        let v = c ^ *k;
        if absorb {
            for &b in raw {
                *k ^= t[b as usize];
            }
        }
        self.plain.extend(v.to_le_bytes());
        self.seg(seg);
        Ok(v)
    }
    fn u32(&mut self) -> R<u32> {
        self.word(true, Seg::Words(1))
    }
    fn bytes(&mut self, n: usize) -> R<Vec<u8>> {
        let raw = self.need(n)?;
        self.pos += n;
        let Some((t, k)) = &mut self.crypt else {
            return Ok(raw.to_vec());
        };
        let mut out = Vec::with_capacity(n);
        for &c in raw {
            out.push(c ^ *k as u8);
            *k ^= t[c as usize];
        }
        self.plain.extend(&out);
        self.seg(Seg::Bytes(n as u32));
        Ok(out)
    }
    fn byte(&mut self) -> R<u8> {
        Ok(self.bytes(1)?[0])
    }
    fn count(&mut self, each: usize) -> R<usize> {
        let n = self.u32()? as usize;
        if n.saturating_mul(each) > self.left() {
            return Err(format!("bad count {n} at {}", self.pos - 4));
        }
        Ok(n)
    }
    fn str(&mut self) -> R<String> {
        let n = self.count(1)?;
        Ok(self.bytes(n)?.iter().map(|&b| b as char).collect())
    }
    fn wstr(&mut self) -> R<String> {
        let n = self.count(2)?;
        let b = self.bytes(2 * n)?;
        Ok(String::from_utf16_lossy(
            &b.chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect::<Vec<_>>(),
        ))
    }
    fn skip_words(&mut self, n: usize) -> R<()> {
        for _ in 0..n {
            self.u32()?;
        }
        Ok(())
    }
    /// A checksum: the key as it is now.
    fn sum(&mut self) -> R<()> {
        let raw = self.need(4)?;
        self.pos += 4;
        if let Some((_, k)) = &self.crypt {
            if u32::from_le_bytes(raw.try_into().unwrap()) != *k {
                return Err(format!("checksum mismatch at {}", self.pos - 4));
            }
            self.plain.extend(raw);
            self.plan.push(Seg::Sum);
        }
        Ok(())
    }
    /// Block id and where its payload ends.
    fn begin(&mut self) -> R<(u32, usize)> {
        let id = self.u32()?;
        let len = self.word(false, Seg::Len)? as usize;
        let end = self
            .pos
            .checked_add(len)
            .filter(|&e| e + 4 <= self.d.len())
            .ok_or_else(|| format!("block {id} runs past the end"))?;
        Ok((id, end))
    }
    fn end(&mut self, id: u32, end: usize) -> R<()> {
        if self.pos != end {
            return Err(format!("block {id}: read to {}, expected {end}", self.pos));
        }
        self.sum()
    }
    /// Fields a newer version added at the end of a block: kept as 32-bit values.
    fn rest(&mut self, end: usize) -> R<()> {
        if self.pos > end {
            return Err("a block was read past its end".into());
        }
        let n = end - self.pos;
        if n % 4 != 0 {
            return Err(format!("{n} unknown bytes at {}", self.pos));
        }
        self.skip_words(n / 4)
    }
    fn field(&mut self, path: &[&str], kind: Kind, lo: i64, hi: i64) -> R<u32> {
        let at = self.at();
        let v = self.u32()?;
        self.recs.push(Rec::Field(p(path), at, kind, lo, hi));
        Ok(v)
    }
    fn info(&mut self, path: &[&str], v: impl Into<Value>) {
        self.recs.push(Rec::Info(p(path), v.into()));
    }
}

/// Numbers a repeated label: "Potion", "Potion (2)"…
fn unique(seen: &mut HashMap<String, usize>, name: String) -> String {
    let n = seen.entry(name.clone()).or_default();
    *n += 1;
    if *n > 1 {
        format!("{name} ({n})")
    } else {
        name
    }
}

/// "StormMastery" / "compa_scrapmetal" → "Storm mastery" / "Compa scrapmetal".
pub(crate) fn words(key: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in key.chars() {
        if c == '_' || c == ' ' {
            if !out.ends_with(' ') && !out.is_empty() {
                out.push(' ');
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        out.push(if out.is_empty() {
            c.to_ascii_uppercase()
        } else {
            c.to_ascii_lowercase()
        });
    }
    out.trim_end().to_string()
}

/// A database record path made readable: "records/skills/playerclass01/blitz1.dbr" →
/// "Playerclass01: Blitz1" (the folder it's in, then its file name). Real names live in
/// the game's own database, which isn't shipped here.
pub(crate) fn record_name(path: &str) -> String {
    let parts: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    let Some(last) = parts.last() else {
        return String::new();
    };
    let stem = last
        .strip_suffix(".dbr")
        .or_else(|| last.strip_suffix(".DBR"))
        .unwrap_or(last);
    match parts.len() {
        0 | 1 => words(stem),
        n => format!("{}: {}", words(parts[n - 2]), words(stem)),
    }
}

/// One item: its record path and where its stack count is.
fn item(c: &mut Cur, v2: bool) -> R<(String, usize)> {
    let base = c.str()?;
    for _ in 0..4 {
        c.str()?; // prefix, suffix, modifier, transmute
    }
    c.u32()?; // seed
    c.str()?; // relic
    c.str()?; // relic bonus
    c.u32()?; // relic seed
    c.str()?; // augment
    c.str()?; // augment bonus
    c.u32()?; // augment seed
    if v2 {
        c.str()?; // ascended affix
        c.str()?; // its bonus
    }
    c.u32()?; // relic completion
    let stack = c.at();
    c.u32()?;
    if v2 {
        c.skip_words(2)?; // reroll counts
    }
    Ok((base, stack))
}

fn stack_field(
    c: &mut Cur,
    group: &str,
    seen: &mut HashMap<String, usize>,
    (base, at): (String, usize),
) {
    if base.is_empty() {
        return;
    }
    let label = unique(seen, record_name(&base));
    c.recs
        .push(Rec::Field(p(&[group, &label]), at, Kind::U32, 1, 100_000));
}

fn uid_lists(c: &mut Cur, n: usize) -> R<()> {
    for _ in 0..n {
        let k = c.count(16)?;
        c.bytes(16 * k)?;
    }
    Ok(())
}

fn block(c: &mut Cur, id: u32, end: usize, header_level: usize) -> R<()> {
    let v = c.u32()?;
    match id {
        1 => {
            c.bytes(2)?;
            let last = c.byte()?;
            let most = c.byte()?;
            for (k, d) in [("Difficulty", last), ("Highest difficulty", most)] {
                if let Some(n) = DIFFICULTY.get(d as usize) {
                    c.info(&["Character", k], *n);
                }
            }
            c.field(&["Character", "Money"], Kind::U32, 0, i32::MAX as i64)?;
            c.byte()?;
            c.field(&["Character", "Tributes"], Kind::U32, 0, i32::MAX as i64)?;
            c.byte()?;
            if (2..=4).contains(&v) {
                c.u32()?;
            }
            c.bytes(3)?;
            c.str()?;
            if v >= 5 {
                let n = c.count(1)?;
                c.bytes(n)?;
            }
        }
        2 => {
            c.field(&["Character", "Level"], Kind::U32, 1, 100)?;
            c.recs
                .push(Rec::Mirror(p(&["Character", "Level"]), header_level));
            c.field(&["Character", "Experience"], Kind::U32, 0, i32::MAX as i64)?;
            for k in ["Attribute points", "Skill points", "Devotion points"] {
                c.field(&["Points", k], Kind::U32, 0, 10_000)?;
            }
            let total = c.u32()?;
            c.info(&["Points", "Devotion points unlocked"], total);
            for k in ["Physique", "Cunning", "Spirit", "Health", "Energy"] {
                let at = c.at();
                c.u32()?;
                c.recs
                    .push(Rec::Field(p(&["Attributes", k]), at, Kind::F32, 0, 100_000));
            }
        }
        3 => {
            let v2 = v >= 11;
            if c.byte()? != 0 {
                let sacks = c.count(4)?;
                c.skip_words(2)?;
                let mut seen = HashMap::new();
                for i in 0..sacks {
                    let (sid, send) = c.begin()?;
                    c.byte()?;
                    let group = if i == 0 {
                        "Inventory".to_string()
                    } else {
                        format!("Bag {i}")
                    };
                    for _ in 0..c.count(4)? {
                        let it = item(c, v2)?;
                        stack_field(c, &group, &mut seen, it);
                        c.skip_words(2)?;
                    }
                    c.end(sid, send)?;
                }
                c.byte()?;
                let mut equipped = vec![];
                for _ in 0..12 {
                    equipped.push(item(c, v2)?.0);
                    c.byte()?;
                }
                for _ in 0..2 {
                    c.byte()?;
                    for _ in 0..2 {
                        equipped.push(item(c, v2)?.0);
                        c.byte()?;
                    }
                }
                let names: Vec<Value> = equipped
                    .iter()
                    .filter(|b| !b.is_empty())
                    .map(|b| Value::from(record_name(b)))
                    .collect();
                c.info(&["Character", "Equipped"], names);
            }
        }
        4 => {
            let v2 = v >= 11;
            for page in 0..c.count(4)? {
                let (sid, send) = c.begin()?;
                c.skip_words(2)?;
                let group = format!("Stash {}", page + 1);
                let mut seen = HashMap::new();
                for _ in 0..c.count(4)? {
                    let it = item(c, v2)?;
                    stack_field(c, &group, &mut seen, it);
                    c.skip_words(2)?;
                }
                if v2 {
                    c.skip_words(4)?; // tab icon and colours
                    c.wstr()?; // tab name
                }
                c.end(sid, send)?;
            }
        }
        5 => {
            uid_lists(c, 3)?;
            c.bytes(48)?;
        }
        6 | 7 => uid_lists(c, 3)?,
        17 => uid_lists(c, 6)?,
        8 => {
            let v2 = v >= 8;
            let mut seen = HashMap::new();
            for _ in 0..c.count(4)? {
                let name = c.str()?;
                let at = c.at();
                c.u32()?;
                let group = if name.to_ascii_lowercase().contains("/devotion/") {
                    "Devotions"
                } else {
                    "Skills"
                };
                let label = unique(&mut seen, record_name(&name));
                c.recs
                    .push(Rec::Field(p(&[group, &label]), at, Kind::U32, 0, 100));
                c.byte()?;
                if v2 {
                    c.byte()?;
                }
                c.skip_words(3)?;
                c.bytes(2)?;
                c.str()?;
                c.str()?;
            }
            let m = c.u32()?;
            c.info(&["Character", "Masteries allowed"], m);
            c.skip_words(2)?;
            for _ in 0..c.count(4)? {
                for _ in 0..3 {
                    c.str()?;
                }
                c.u32()?; // slot of the item granting it
                c.str()?;
            }
            if v >= 6 {
                for _ in 0..c.count(16)? {
                    for _ in 0..4 {
                        c.str()?;
                    }
                }
            }
        }
        10 => {
            for _ in 0..3 {
                for _ in 0..c.count(4)? {
                    c.str()?;
                }
            }
        }
        12 => {
            for _ in 0..c.count(4)? {
                c.str()?;
            }
        }
        13 => {
            c.u32()?;
            for _ in 0..c.count(14)? {
                c.bytes(2)?;
                c.skip_words(3)?;
            }
        }
        14 => {
            c.byte()?;
            c.u32()?;
            c.byte()?;
            for _ in 0..5 {
                c.str()?;
                c.str()?;
                c.byte()?;
            }
            if v >= 7 {
                // One bar of 46 slots per form the character has taken.
                for _ in 0..c.count(8)? {
                    c.skip_words(2)?;
                    for _ in 0..46 {
                        hotslot(c)?;
                    }
                }
                c.u32()?;
            } else {
                for _ in 0..if v == 4 { 36 } else { 46 } {
                    hotslot(c)?;
                }
                c.u32()?;
                if v == 6 {
                    c.u32()?;
                }
            }
        }
        15 => {
            let n = c.count(4)?;
            c.skip_words(n)?;
        }
        16 => {
            let play = c.u32()?;
            let deaths = c.u32()?;
            let kills = c.u32()?;
            c.info(&["Statistics", "Play time (seconds)"], play);
            c.info(&["Statistics", "Deaths"], deaths);
            c.info(&["Statistics", "Kills"], kills);
            c.skip_words(9)?;
            for _ in 0..3 {
                c.str()?;
                c.skip_words(2)?;
                c.str()?;
                c.str()?;
            }
            c.skip_words(4 + 8 + 3 + 4)?;
            if v >= 11 {
                for _ in 0..c.count(8)? {
                    c.str()?;
                    c.u32()?;
                }
                c.skip_words(2)?;
                c.byte()?;
            }
            c.skip_words(2)?;
            if v >= 12 {
                c.skip_words(2)?;
            }
        }
        _ => {}
    }
    c.rest(end)
}

fn hotslot(c: &mut Cur) -> R<()> {
    match c.u32()? {
        0 => {
            c.str()?;
            c.byte()?;
            c.str()?;
            c.u32()?;
        }
        4 => {
            c.str()?;
            c.str()?;
            c.str()?;
            c.wstr()?;
        }
        _ => {}
    }
    Ok(())
}

fn walk(c: &mut Cur) -> R<()> {
    if c.u32()? != MAGIC {
        return Err("not a Grim Dawn character".into());
    }
    let pre = c.u32()?;
    let name = c.wstr()?;
    c.info(&["Character", "Name"], name);
    let sex = c.byte()?;
    c.info(
        &["Character", "Sex"],
        if sex == 1 { "Male" } else { "Female" },
    );
    let class = c.str()?;
    c.info(&["Character", "Class tag"], class);
    let header_level = c.at();
    c.u32()?;
    let hc = c.byte()?;
    c.info(&["Character", "Hardcore"], hc != 0);
    if pre >= 2 {
        c.byte()?; // expansions owned
    }
    c.sum()?;
    c.u32()?; // data version
    c.bytes(16)?;
    while c.pos < c.d.len() {
        let (id, end) = c.begin()?;
        block(c, id, end, header_level)?;
        c.end(id, end)?;
    }
    Ok(())
}

/// The seed, the plain data (from the "GDCX" magic on) and how to encrypt it again.
/// `Err` when the file starts like a character but its layout isn't one this reader knows.
pub fn decrypt(file: &[u8]) -> Option<Result<(u32, Vec<u8>, Vec<Seg>), String>> {
    let seed = u32::from_le_bytes(file.get(..4)?.try_into().ok()?) ^ SEED_MASK;
    if u32::from_le_bytes(file.get(4..8)?.try_into().ok()?) ^ seed != MAGIC {
        return None;
    }
    let mut c = Cur {
        d: file,
        pos: 4,
        base: 4,
        crypt: Some((table(seed), seed)),
        plain: Vec::with_capacity(file.len()),
        plan: vec![],
        recs: vec![],
    };
    Some(walk(&mut c).map(|()| (seed, c.plain, c.plan)))
}

/// The file again, from (edited) plain data: same seed, fresh lengths' encryption and checksums.
pub fn encrypt(seed: u32, plan: &[Seg], plain: &[u8]) -> Result<Vec<u8>, String> {
    let t = table(seed);
    let mut k = seed;
    let mut out = Vec::with_capacity(plain.len() + 4);
    out.extend((seed ^ SEED_MASK).to_le_bytes());
    let mut i = 0;
    let short = || "this save's data can't change size".to_string();
    let word = |i: usize| {
        plain
            .get(i..i + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(short)
    };
    for s in plan {
        match *s {
            Seg::Words(n) => {
                for _ in 0..n {
                    let cb = (word(i)? ^ k).to_le_bytes();
                    out.extend(cb);
                    for b in cb {
                        k ^= t[b as usize];
                    }
                    i += 4;
                }
            }
            Seg::Bytes(n) => {
                for _ in 0..n {
                    let c = plain.get(i).ok_or_else(short)? ^ k as u8;
                    out.push(c);
                    k ^= t[c as usize];
                    i += 1;
                }
            }
            Seg::Len => {
                out.extend((word(i)? ^ k).to_le_bytes());
                i += 4;
            }
            Seg::Sum => {
                word(i)?;
                out.extend(k.to_le_bytes());
                i += 4;
            }
        }
    }
    if i != plain.len() {
        return Err(short());
    }
    Ok(out)
}

/// Plain character data (what the encryption layer hands over).
pub fn looks(d: &[u8]) -> bool {
    d.len() > 32 && d.starts_with(b"GDCX")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Grim Dawn character".into());
    }
    let mut c = Cur {
        d,
        pos: 0,
        base: 0,
        crypt: None,
        plain: vec![],
        plan: vec![],
        recs: vec![],
    };
    walk(&mut c)?;
    let mut b = Builder::new(d);
    for r in c.recs {
        match r {
            Rec::Info(path, v) => b.info(&path.iter().map(String::as_str).collect::<Vec<_>>(), v),
            Rec::Field(path, at, kind, lo, hi) => {
                b.ranged(
                    &path.iter().map(String::as_str).collect::<Vec<_>>(),
                    at,
                    kind,
                    lo,
                    hi,
                );
            }
            Rec::Mirror(path, at) => {
                b.mirror(&path.iter().map(String::as_str).collect::<Vec<_>>(), at)
            }
        }
    }
    Ok(b.finish("Grim Dawn character", Checksum::None).with_note(
        "Money, level, experience, unspent points, attributes, skill ranks and item stacks. Skills and items show the \
         game's internal record names (the real names are in the game's own database). The save is encrypted again \
         and its checksums updated when you save.",
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds an encrypted file the way the game writes it.
    pub struct W {
        t: Box<[u32; 256]>,
        k: u32,
        pub out: Vec<u8>,
        stack: Vec<(usize, u32)>,
    }

    impl W {
        pub fn new(seed: u32) -> Self {
            W {
                t: table(seed),
                k: seed,
                out: (seed ^ SEED_MASK).to_le_bytes().to_vec(),
                stack: vec![],
            }
        }
        pub fn u32(&mut self, v: u32) {
            let cb = (v ^ self.k).to_le_bytes();
            self.out.extend(cb);
            for b in cb {
                self.k ^= self.t[b as usize];
            }
        }
        pub fn f32(&mut self, v: f32) {
            self.u32(v.to_bits());
        }
        pub fn bytes(&mut self, d: &[u8]) {
            for &b in d {
                let c = b ^ self.k as u8;
                self.out.push(c);
                self.k ^= self.t[c as usize];
            }
        }
        pub fn str(&mut self, s: &str) {
            self.u32(s.len() as u32);
            self.bytes(s.as_bytes());
        }
        pub fn wstr(&mut self, s: &str) {
            let u: Vec<u16> = s.encode_utf16().collect();
            self.u32(u.len() as u32);
            self.bytes(&u.iter().flat_map(|c| c.to_le_bytes()).collect::<Vec<_>>());
        }
        pub fn sum(&mut self) {
            self.out.extend(self.k.to_le_bytes());
        }
        pub fn begin(&mut self, id: u32) {
            self.u32(id);
            self.stack.push((self.out.len(), self.k));
            self.out.extend([0; 4]);
        }
        pub fn end(&mut self) {
            let (at, k) = self.stack.pop().unwrap();
            let len = (self.out.len() - at - 4) as u32;
            self.out[at..at + 4].copy_from_slice(&(len ^ k).to_le_bytes());
            self.sum();
        }
        fn item(&mut self, base: &str, stack: u32) {
            self.str(base);
            for _ in 0..4 {
                self.str("");
            }
            self.u32(7);
            self.str("");
            self.str("");
            self.u32(0);
            self.str("");
            for _ in 0..3 {
                self.u32(0);
            }
            self.u32(stack);
        }
    }

    /// A small synthetic character: header, blocks 1–4, 8 and 16.
    pub fn sample() -> Vec<u8> {
        let mut w = W::new(0x1234_5678);
        w.u32(MAGIC);
        w.u32(2);
        w.wstr("Tester");
        w.bytes(&[1]);
        w.str("tagSkillClassName01");
        w.u32(12);
        w.bytes(&[0, 1]);
        w.sum();
        w.u32(8);
        w.bytes(&[9; 16]);
        w.begin(1);
        w.u32(4);
        w.bytes(&[1, 1, 1, 1]);
        w.u32(5000); // money
        w.bytes(&[0]);
        w.u32(3);
        w.bytes(&[0]);
        w.u32(0);
        w.bytes(&[0, 0, 0]);
        w.str("");
        w.end();
        w.begin(2);
        w.u32(8);
        for v in [12, 34_567, 2, 3, 1, 4] {
            w.u32(v);
        }
        for f in [90.0, 60.0, 50.0, 400.0, 300.0] {
            w.f32(f);
        }
        w.end();
        w.begin(3);
        w.u32(4);
        w.bytes(&[1]);
        w.u32(1);
        w.u32(0);
        w.u32(0);
        w.begin(0);
        w.bytes(&[0]);
        w.u32(1);
        w.item("records/items/materia/compa_scrapmetal.dbr", 40);
        w.u32(1);
        w.u32(2);
        w.end();
        w.bytes(&[0]);
        for _ in 0..12 {
            w.item("", 1);
            w.bytes(&[0]);
        }
        for _ in 0..2 {
            w.bytes(&[0]);
            for _ in 0..2 {
                w.item("", 1);
                w.bytes(&[0]);
            }
        }
        w.end();
        w.begin(4);
        w.u32(5);
        w.u32(1);
        w.begin(0);
        w.u32(10);
        w.u32(18);
        w.u32(1);
        w.item("records/items/misc/potions/potion_healtha01.dbr", 9);
        w.f32(1.0);
        w.f32(2.0);
        w.end();
        w.end();
        w.begin(8);
        w.u32(7);
        w.u32(1);
        w.str("records/skills/playerclass01/blitz1.dbr");
        w.u32(5);
        w.bytes(&[1]);
        w.u32(0);
        w.u32(0);
        w.u32(0);
        w.bytes(&[0, 0]);
        w.str("");
        w.str("");
        w.u32(2);
        w.u32(0);
        w.u32(0);
        w.u32(0);
        w.u32(0);
        w.end();
        w.out
    }

    #[test]
    fn decrypts_edits_and_encrypts_again() {
        let file = sample();
        let (seed, plain, plan) = decrypt(&file).unwrap().unwrap();
        assert_eq!(encrypt(seed, &plan, &plain).unwrap(), file);
        let mut doc = parse(&plain).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Character/Name"), Some(&"Tester".into()));
        assert_eq!(v.pointer("/Character/Money"), Some(&5000.into()));
        assert_eq!(v.pointer("/Character/Level"), Some(&12.into()));
        assert_eq!(v.pointer("/Attributes/Physique"), Some(&90.0.into()));
        assert_eq!(
            v.pointer("/Inventory/Materia: Compa scrapmetal"),
            Some(&40.into())
        );
        assert_eq!(
            v.pointer("/Stash 1/Potions: Potion healtha01"),
            Some(&9.into())
        );
        assert_eq!(v.pointer("/Skills/Playerclass01: Blitz1"), Some(&5.into()));
        doc.set("/Character/Money", 5001.into()).unwrap();
        doc.set("/Character/Level", 13.into()).unwrap();
        doc.set("/Attributes/Cunning", 75.0.into()).unwrap();
        doc.set("/Skills/Playerclass01: Blitz1", 6.into()).unwrap();
        let edited = doc.to_bytes().unwrap();
        let out = encrypt(seed, &plan, &edited).unwrap();
        assert_eq!(out.len(), file.len());
        // It decrypts again (every checksum is right) with the edits in place.
        let (_, plain2, _) = decrypt(&out).unwrap().unwrap();
        let again = parse(&plain2).unwrap();
        assert_eq!(again.view.pointer("/Character/Money"), Some(&5001.into()));
        assert_eq!(again.view.pointer("/Character/Level"), Some(&13.into()));
        assert_eq!(
            again.view.pointer("/Attributes/Cunning"),
            Some(&75.0.into())
        );
        assert_eq!(
            again.view.pointer("/Skills/Playerclass01: Blitz1"),
            Some(&6.into())
        );
        assert_eq!(
            again.view.pointer("/Inventory"),
            doc.view.pointer("/Inventory")
        );
        // The header's level follows.
        let lvl_at = plain
            .windows(19)
            .position(|w| w == b"tagSkillClassName01")
            .unwrap()
            + 19;
        assert_eq!(&plain2[lvl_at..lvl_at + 4], &13u32.to_le_bytes());
        // A damaged checksum is refused.
        let mut bad = file.clone();
        let n = bad.len();
        bad[n - 1] ^= 1;
        assert!(decrypt(&bad).unwrap().is_err());
    }

    /// Research aid shared by the action RPG readers: every file under the folder in `var`
    /// whose name passes `want` is decoded through its layers, saved unchanged (must be
    /// byte-identical), then saved with each of `edits` raised by one and read back.
    pub(crate) fn research(var: &str, want: fn(&str) -> bool, kind: &str, edits: &[&str]) {
        use crate::formats::{decode_with, encode, Content, Options};
        fn walk(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(p).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else {
                    out.push(p);
                }
            }
        }
        let mut files = vec![];
        walk(
            std::path::Path::new(&std::env::var(var).unwrap()),
            &mut files,
        );
        files.sort();
        let mut tried = 0;
        for f in files
            .iter()
            .filter(|f| want(&f.file_name().unwrap().to_string_lossy()))
        {
            let raw = std::fs::read(f).unwrap();
            let opts = Options::for_path(f);
            let d = decode_with(&raw, &opts);
            let Content::Tree(doc) = &d.content else {
                println!("{}: not read ({:?})", f.display(), d.hint);
                continue;
            };
            assert_eq!(doc.kind_name, kind, "{}", f.display());
            tried += 1;
            let v = &doc.view;
            let short = |p: &str| v.pointer(p).map(|x| x.to_string()).unwrap_or_default();
            println!(
                "{}: {}",
                f.display(),
                edits
                    .iter()
                    .map(|p| format!("{p}={}", short(p)))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            for g in ["/Inventory", "/Items"] {
                if let Some(Value::Object(m)) = v.pointer(g) {
                    let big: Vec<String> = m
                        .iter()
                        .filter(|(_, n)| n.as_i64().unwrap_or(0) > 1)
                        .take(3)
                        .map(|(k, n)| format!("{k} x{n}"))
                        .collect();
                    println!("    {g}: {} items, stacks e.g. {big:?}", m.len());
                }
            }
            assert_eq!(
                encode(&d.layers, &d.content).unwrap(),
                raw,
                "unchanged save differs: {}",
                f.display()
            );
            let mut doc = doc.clone();
            let mut expect = doc.view.clone();
            for p in edits {
                let Some(old) = doc.view.pointer(p).and_then(Value::as_f64) else {
                    continue;
                };
                let new = if doc.view.pointer(p).unwrap().is_f64() {
                    Value::from(old + 1.0)
                } else {
                    Value::from(old as i64 + 1)
                };
                doc.set(p, new.clone())
                    .unwrap_or_else(|e| panic!("{p}: {e}"));
                *expect.pointer_mut(p).unwrap() = new;
            }
            let out = encode(&d.layers, &Content::Tree(doc)).unwrap();
            assert_eq!(out.len(), raw.len());
            let again = decode_with(&out, &opts);
            let Content::Tree(a) = &again.content else {
                panic!(
                    "edited save no longer reads: {} {:?}",
                    f.display(),
                    again.hint
                )
            };
            assert_eq!(a.view, expect, "{}", f.display());
        }
        assert!(tried > 0, "no saves were read");
    }

    /// SAVE_EDITOR_GD = a folder of player.gdc files.
    #[test]
    #[ignore]
    fn real_saves() {
        research(
            "SAVE_EDITOR_GD",
            |n| n.ends_with(".gdc"),
            "Grim Dawn character",
            &[
                "/Character/Money",
                "/Attributes/Physique",
                "/Character/Experience",
                "/Points/Skill points",
            ],
        );
    }

    #[test]
    fn readable_record_names() {
        assert_eq!(
            record_name("records/skills/playerclass01/blitz1.dbr"),
            "Playerclass01: Blitz1"
        );
        assert_eq!(
            record_name(r"Records\Skills\Storm\IceShard.dbr"),
            "Storm: Ice shard"
        );
        assert_eq!(record_name("x.dbr"), "X");
    }
}
