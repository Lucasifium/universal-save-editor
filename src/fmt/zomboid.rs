//! Project Zomboid (Build 42): `players.db` in the save's folder. It is an SQLite file whose
//! `localPlayers` (single player and the host of co-op) and `networkPlayers` (dedicated servers)
//! tables hold one big-endian binary record per character (the `data` column), with the death
//! flag in `isDead`.
//!
//! The record layout follows the public notes of alexjthomson's project-zomboid-save-editor (no
//! licence given, so only the facts are used; the code here is new). The record is walked from
//! the front and only accepted when the walk ends exactly on its last byte, which proves the
//! layout for that file. Only fixed-size values are edited, in place, so nothing moves.

use super::fields::{Builder, Checksum, Kind};
use super::sqlite::Temp;
use crate::tree::TreeDoc;
use rusqlite::{Connection, OpenFlags};

pub const KIND: &str = "Project Zomboid players.db";
const TABLES: [&str; 2] = ["localPlayers", "networkPlayers"];

/// The 17 body parts, in the order the game saves them.
const PARTS: [&str; 17] = [
    "Left hand", "Right hand", "Left forearm", "Right forearm", "Left upper arm", "Right upper arm", "Upper torso", "Lower torso", "Head",
    "Neck", "Groin", "Left upper leg", "Right upper leg", "Left lower leg", "Right lower leg", "Left foot", "Right foot",
];

/// The 24 stats, in the order the game saves them.
const STATS: [&str; 24] = [
    "Anger", "Boredom", "Discomfort", "Endurance", "Fatigue", "Fitness", "FoodSickness", "Hunger", "Idleness", "Intoxication", "Morale",
    "NicotineWithdrawal", "Pain", "Panic", "Poison", "Sanity", "Sickness", "Stress", "Temperature", "Thirst", "Unhappiness", "Wetness",
    "ZombieFever", "ZombieInfection",
];

struct R<'a> {
    d: &'a [u8],
    at: usize,
}

impl R<'_> {
    fn skip(&mut self, n: usize) -> Option<()> {
        self.at = self.at.checked_add(n).filter(|e| *e <= self.d.len())?;
        Some(())
    }
    fn byte(&mut self) -> Option<u8> {
        let b = *self.d.get(self.at)?;
        self.at += 1;
        Some(b)
    }
    /// A one-byte true/false (0 or 1 only).
    fn flag(&mut self) -> Option<bool> {
        match self.byte()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn i16(&mut self) -> Option<i16> {
        let v = i16::from_be_bytes(self.d.get(self.at..self.at + 2)?.try_into().ok()?);
        self.at += 2;
        Some(v)
    }
    fn i32(&mut self) -> Option<i32> {
        let v = i32::from_be_bytes(self.d.get(self.at..self.at + 4)?.try_into().ok()?);
        self.at += 4;
        Some(v)
    }
    /// A count that must be small enough to be believable.
    fn count(&mut self, max: i32) -> Option<usize> {
        let n = self.i32()?;
        (0..=max).contains(&n).then_some(n as usize)
    }
    /// A string: 16-bit byte length, then UTF-8.
    fn text(&mut self) -> Option<String> {
        let n = self.i16()?;
        if n < 0 {
            return None;
        }
        let s = self.d.get(self.at..self.at + n as usize)?;
        self.at += n as usize;
        String::from_utf8(s.to_vec()).ok()
    }
    fn texts(&mut self, n: usize) -> Option<()> {
        (0..n).try_for_each(|_| self.text().map(|_| ()))
    }
    fn color(&mut self) -> Option<()> {
        self.skip(3)
    }
    /// A Kahlua table: count, then (type, key) and (type, value) pairs.
    fn table(&mut self, depth: usize) -> Option<()> {
        if depth > 8 {
            return None;
        }
        for _ in 0..self.count(100_000)? {
            for is_value in [false, true] {
                match self.byte()? {
                    0 => {
                        self.text()?;
                    }
                    1 => self.skip(8)?,
                    2 if is_value => self.table(depth + 1)?,
                    3 if is_value => {
                        self.flag()?;
                    }
                    _ => return None,
                }
            }
        }
        Some(())
    }
    fn item_visual(&mut self) -> Option<()> {
        let f = self.byte()?;
        self.texts(3)?;
        if f & 1 != 0 {
            self.color()?;
        }
        for bit in [2, 4] {
            if f & bit != 0 {
                self.skip(1)?;
            }
        }
        if f & 8 != 0 {
            self.skip(4)?;
        }
        if f & 0x10 != 0 {
            self.text()?;
        }
        for _ in 0..6 {
            self.byte_array()?;
        }
        Some(())
    }
    fn byte_array(&mut self) -> Option<()> {
        let n = self.byte()?;
        self.skip(n as usize)
    }
}

struct Walk {
    name: String,
    profession: String,
    stats: usize,
    parts: Vec<usize>,
    hours: usize,
    kills: usize,
    survivor_kills: usize,
    weight: usize,
    traits: Vec<String>,
    /// (perk id, offset of its level, offset of its experience).
    perks: Vec<(String, Option<usize>, Option<usize>)>,
}

/// Walks a whole record; `None` unless every byte is accounted for.
fn walk(d: &[u8], ver: i32) -> Option<Walk> {
    if !(217..=400).contains(&ver) {
        return None;
    }
    let mut r = R { d, at: 0 };
    r.skip(2 + 8 + 12 + 4)?; // two type bytes, offsets, position, direction
    if r.flag()? {
        r.table(0)?; // mod data
    }
    let (mut forename, mut surname, mut profession) = (String::new(), String::new(), String::new());
    if r.flag()? {
        r.i32()?;
        forename = r.text()?;
        surname = r.text()?;
        r.text()?; // torso model
        r.i32()?; // gender
        profession = r.text()?;
        if r.i32()? == 1 {
            let n = r.count(1000)?;
            r.texts(n)?;
        }
        for _ in 0..r.count(1000)? {
            r.text()?;
            r.i32()?;
        }
        r.text()?;
        r.skip(8)?; // voice pitch and type
    }
    // How the character looks.
    let f = r.byte()?;
    for bit in [4, 2, 8] {
        if f & bit != 0 {
            r.color()?;
        }
    }
    r.skip(3)?;
    for bit in [0x40, 0x10, 0x20] {
        if f & bit != 0 {
            r.text()?;
        }
    }
    for _ in 0..3 {
        r.byte_array()?;
    }
    for _ in 0..r.byte()? {
        r.item_visual()?;
    }
    r.text()?;
    let f2 = r.byte()?;
    for bit in [4, 2] {
        if f2 & bit != 0 {
            r.color()?;
        }
    }
    // The inventory: each item has its own length, so items are stepped over.
    r.text()?;
    r.flag()?;
    for _ in 0..r.i16().filter(|n| *n >= 0)? {
        let same = r.i32().filter(|n| *n >= 1)? as usize;
        let len = r.i32().filter(|n| *n >= 7)? as usize;
        r.skip(len)?;
        r.skip((same - 1).checked_mul(4)?)?;
    }
    r.flag()?;
    r.skip(4)?;
    r.flag()?; // asleep
    r.skip(4)?;
    let stats = r.at;
    r.skip(24 * 4)?;
    // The body.
    let mut parts = Vec::new();
    for _ in 0..PARTS.len() {
        let mut flags = [false; 8];
        for f in &mut flags {
            *f = r.flag()?;
        }
        parts.push(r.at);
        r.skip(4)?;
        if flags[3] {
            r.skip(4)?; // bandage life
        }
        if r.flag()? {
            r.skip(4)?; // wound infection level
        }
        r.skip(7 * 4)?;
        r.skip(3)?; // glass, bandage experience, stitched
        r.skip(4)?;
        r.skip(2)?;
        r.skip(4)?;
        if r.flag()? {
            r.skip(4)?; // splint factor
        }
        r.flag()?; // bullet
        r.skip(4)?;
        r.flag()?;
        r.skip(4)?;
        r.texts(2)?;
        r.skip(12)?;
        if ver >= 227 {
            r.skip(12)?;
        }
    }
    r.skip(4)?;
    r.flag()?;
    r.skip(4)?;
    if ver >= 222 {
        r.skip(4)?;
    }
    r.flag()?;
    r.skip(6 * 4)?;
    if r.flag()? {
        // Body temperature.
        r.skip(2 * 4)?;
        if ver >= 243 {
            r.skip(4)?;
        }
        r.skip(5 * 4)?;
        if ver >= 249 {
            r.skip(4)?;
        }
        for _ in 0..r.count(100)? {
            r.skip(6 * 4)?;
            if ver >= 241 {
                r.skip(4)?;
            }
            if ver >= 243 {
                r.skip(12)?;
            }
        }
    }
    // Traits and skills.
    let n = r.count(1000)?;
    let mut traits = Vec::new();
    for _ in 0..n {
        traits.push(r.text()?);
    }
    r.skip(12)?; // total experience, level, last level
    let mut perks: Vec<(String, Option<usize>, Option<usize>)> = Vec::new();
    let slot = |id: String, perks: &mut Vec<(String, Option<usize>, Option<usize>)>| -> usize {
        match perks.iter().position(|p| p.0 == id) {
            Some(i) => i,
            None => {
                perks.push((id, None, None));
                perks.len() - 1
            }
        }
    };
    for _ in 0..r.count(1000)? {
        let id = r.text()?;
        let i = slot(id, &mut perks);
        perks[i].2 = Some(r.at);
        r.skip(4)?;
    }
    for _ in 0..r.count(1000)? {
        let id = r.text()?;
        let i = slot(id, &mut perks);
        perks[i].1 = Some(r.at);
        r.skip(4)?;
    }
    for _ in 0..r.count(1000)? {
        r.text()?;
        r.skip(6)?;
    }
    // The rest of the character.
    r.skip(8)?;
    r.flag()?;
    r.skip(8 * 4)?;
    for _ in 0..r.count(100_000)? {
        r.text()?;
        r.skip(4)?;
    }
    r.skip(4)?;
    let n = r.count(100_000)?;
    r.texts(n)?;
    r.skip(4 * 4)?;
    for _ in 0..6 {
        r.flag()?;
    }
    for _ in 0..[(202, 1), (217, 2)].iter().filter(|(v, _)| ver >= *v).map(|(_, n)| n).sum::<usize>() {
        r.flag()?;
    }
    r.flag()?;
    r.flag()?;
    if ver >= 230 {
        r.flag()?;
        r.flag()?;
    }
    r.flag()?;
    r.flag()?;
    for _ in 0..r.count(100_000)? {
        r.text()?;
        r.skip(4)?;
    }
    if ver >= 222 {
        let n = r.count(100_000)?;
        r.texts(n)?;
    }
    r.skip(8)?;
    if ver >= 231 {
        let n = r.count(10_000)?;
        r.skip(n)?;
    }
    let hours = r.at;
    r.skip(8)?;
    let kills = r.at;
    r.skip(4)?;
    for _ in 0..r.byte()? {
        r.text()?;
        r.skip(2)?;
    }
    r.skip(4)?;
    let survivor_kills = r.at;
    r.skip(4)?;
    let weight = r.at + 16;
    r.skip(5 * 4)?;
    r.flag()?;
    r.text()?;
    r.skip(12)?;
    r.text()?;
    r.flag()?;
    r.flag()?;
    if ver >= 239 {
        r.flag()?;
    }
    r.skip(1)?;
    if r.flag()? {
        r.skip(10)?;
    }
    for _ in 0..r.count(100_000)? {
        r.skip(16)?;
    }
    // Fitness: four lists of (name, 4 bytes), a list of names, and a list of (name, 8 bytes).
    for (named, size) in [(true, 4usize), (true, 4), (true, 4), (false, 0), (true, 8)] {
        for _ in 0..r.count(100_000)? {
            r.text()?;
            if named {
                r.skip(size)?;
            }
        }
    }
    let n = r.i16().filter(|n| *n >= 0)? as usize;
    r.skip(n * 2)?;
    for _ in 0..r.i16().filter(|n| *n >= 0)? {
        r.text()?;
    }
    if ver >= 203 {
        r.skip(1)?;
    }
    if ver >= 228 {
        for _ in 0..r.count(1_000_000)? {
            let chars = r.count(10_000)?;
            r.skip(chars * 2 + 12)?;
        }
    }
    (r.at == d.len()).then_some(Walk { name: format!("{forename} {surname}").trim().to_string(), profession, stats, parts, hours, kills, survivor_kills, weight, traits, perks })
}

struct Row {
    table: &'static str,
    id: i64,
    version: i32,
    data: Vec<u8>,
    dead: bool,
    account: String,
}

/// Every character row, local players first, each table by id.
fn rows(d: &[u8]) -> Option<Vec<Row>> {
    let tmp = Temp::new(d).ok()?;
    let conn = Connection::open_with_flags(&tmp.0, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let mut out = Vec::new();
    for table in TABLES {
        let account = if table == "networkPlayers" { "COALESCE(username, '')" } else { "''" };
        let sql = format!("SELECT id, worldversion, data, isDead, {account} FROM {table} ORDER BY id");
        let Ok(mut st) = conn.prepare(&sql) else { continue };
        let found = st
            .query_map([], |r| {
                Ok(Row {
                    table,
                    id: r.get(0)?,
                    version: r.get::<_, i64>(1)? as i32,
                    data: r.get(2)?,
                    dead: r.get::<_, i64>(3).unwrap_or(0) != 0,
                    account: r.get(4)?,
                })
            })
            .ok()?
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        out.extend(found);
    }
    drop(conn);
    Some(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"SQLite format 3\0") && parse(d).is_ok()
}

/// Puts the edited records back: the shown bytes are every record followed by its death flag.
/// Returns nothing when that fails, so a failed save is reported and never looks like a saved one.
fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let go = || -> Option<Vec<u8>> {
        let found = rows(outer)?;
        let tmp = Temp::new(outer).ok()?;
        {
            let conn = Connection::open(&tmp.0).ok()?;
            conn.execute_batch("BEGIN").ok()?;
            let mut at = 0;
            for r in &found {
                let new = inner.get(at..at + r.data.len())?;
                let dead = *inner.get(at + r.data.len())? != 0;
                at += r.data.len() + 1;
                if new != r.data || dead != r.dead {
                    let n = conn.execute(&format!("UPDATE {} SET data = ?1, isDead = ?2 WHERE id = ?3", r.table), rusqlite::params![new, dead, r.id]).ok()?;
                    if n != 1 {
                        return None;
                    }
                }
            }
            conn.execute_batch("COMMIT").ok()?;
        }
        std::fs::read(&tmp.0).ok()
    };
    // An empty answer tells the caller the edits could not be written (never the unchanged file).
    go().unwrap_or_default()
}

/// A skill id such as `base:woodwork` as a name to show.
fn skill_name(id: &str) -> String {
    let id = id.rsplit(':').next().unwrap_or(id).replace('_', " ");
    let mut c = id.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    build(d).ok_or_else(|| "not a Project Zomboid players.db this editor can read".to_string())
}

fn build(d: &[u8]) -> Option<TreeDoc> {
    let found = rows(d).filter(|r| !r.is_empty())?;
    let mut walks = Vec::new();
    for r in &found {
        walks.push(walk(&r.data, r.version)?);
    }
    // The records and their death flags, one after another.
    let mut buf = Vec::new();
    let mut base = Vec::new();
    for r in &found {
        base.push(buf.len());
        buf.extend(&r.data);
        buf.push(u8::from(r.dead));
    }
    let mut b = Builder::new(&buf);
    b.big_endian = true;
    b.inside(d, put);
    let many = found.len() > 1;
    b.info(&["About", "Characters"], found.len());
    b.info(&["About", "World version"], found[0].version);
    for (i, (w, r)) in walks.iter().zip(&found).enumerate() {
        let o = base[i];
        let who = if many { format!("{} - {}", i + 1, w.name) } else { w.name.clone() };
        let who = if who.is_empty() { format!("Character {}", i + 1) } else { who };
        if !w.profession.is_empty() {
            b.info(&["Characters", &who, "Job"], w.profession.trim_start_matches("base:").to_string());
        }
        if !r.account.is_empty() {
            b.info(&["Characters", &who, "Account"], r.account.clone());
        }
        b.info(&["Characters", &who, "Traits"], w.traits.iter().map(|t| skill_name(t)).collect::<Vec<_>>().join(", "));
        b.ranged(&["Characters", &who, "Hours survived"], o + w.hours, Kind::F64Int, 0, 999_999)?;
        b.ranged(&["Characters", &who, "Zombie kills"], o + w.kills, Kind::I32, 0, 999_999)?;
        b.ranged(&["Characters", &who, "Survivor kills"], o + w.survivor_kills, Kind::I32, 0, 999_999)?;
        b.ranged(&["Characters", &who, "Weight (kg)"], o + w.weight, Kind::F32, 30, 150)?;
        b.field(&["Characters", &who, "Dead"], o + r.data.len(), Kind::B8);
        let stat = |name: &str| STATS.iter().position(|s| *s == name).map(|n| o + w.stats + n * 4);
        for (group, list) in [
            ("Needs", &[("Hunger", 0, 1), ("Thirst", 0, 1), ("Fatigue", 0, 1), ("Endurance", 0, 1), ("Stress", 0, 1), ("Morale", 0, 1), ("Sanity", 0, 1), ("Boredom", 0, 100), ("Unhappiness", 0, 100)][..]),
            ("Condition", &[("Pain", 0, 100), ("Panic", 0, 100), ("Sickness", 0, 1), ("FoodSickness", 0, 100), ("Poison", 0, 100), ("Intoxication", 0, 100), ("Wetness", 0, 100), ("Temperature", 20, 40), ("ZombieInfection", 0, 100), ("ZombieFever", 0, 100)][..]),
        ] {
            for &(name, lo, hi) in list {
                let label = if name == "FoodSickness" { "Food sickness" } else if name == "ZombieInfection" { "Zombie infection" } else if name == "ZombieFever" { "Zombie fever" } else { name };
                if let Some(at) = stat(name) {
                    b.ranged(&[group, &who, label], at, Kind::F32, lo, hi)?;
                }
            }
        }
        for (n, part) in PARTS.iter().enumerate() {
            b.ranged(&["Body health", &who, part], o + w.parts[n], Kind::F32, 0, 100)?;
        }
        for (id, level, xp) in &w.perks {
            let key = if many { format!("{} - {}", i + 1, skill_name(id)) } else { skill_name(id) };
            if let Some(l) = level {
                b.ranged(&["Skills", &key, "Level"], o + l, Kind::I32, 0, 10)?;
            }
            if let Some(x) = xp {
                b.ranged(&["Skills", &key, "XP"], o + x, Kind::F32, 0, 999_999)?;
            }
        }
    }
    Some(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real players.db: `ZOMBOID_DB=path cargo test --release --lib zomboid_real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn zomboid_real() {
        let d = std::fs::read(std::env::var("ZOMBOID_DB").unwrap()).unwrap();
        assert!(looks(&d));
        let t = parse(&d).unwrap();
        println!("{}", serde_json::to_string_pretty(&t.view).unwrap());
        assert_eq!(t.to_bytes().unwrap(), d);
        // Edits land in the right character's record and nothing else moves.
        let mut t = t;
        let who = "/Needs/2 - Olin Parrott/Hunger";
        t.set(who, serde_json::json!(0.5)).unwrap();
        t.set("/Characters/2 - Olin Parrott/Zombie kills", serde_json::json!(999)).unwrap();
        t.set("/Characters/2 - Olin Parrott/Dead", serde_json::json!(true)).unwrap();
        assert!(t.set("/Needs/2 - Olin Parrott/Thirst", serde_json::json!(2)).is_err());
        let out = t.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer(who).unwrap(), 0.5);
        assert_eq!(back.view.pointer("/Characters/2 - Olin Parrott/Zombie kills").unwrap(), 999);
        assert_eq!(back.view.pointer("/Characters/2 - Olin Parrott/Dead").unwrap(), true);
        assert_eq!(back.view.pointer("/Characters/1 - Alex Thomson/Zombie kills").unwrap(), 2602);
        assert_eq!(rows(&out).unwrap().iter().map(|r| r.dead).collect::<Vec<_>>(), [false, true, false]);
    }

    #[test]
    fn rejects_other_files() {
        assert!(!looks(&[0u8; 400]));
        assert!(parse(&[]).is_err());
    }
}
