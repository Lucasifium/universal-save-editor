//! Valheim characters (`.fch`).
//!
//! The file is an i32 length N, N bytes of profile data, then an i32 64 and the SHA-512 of
//! those N bytes. All little-endian; text is a .NET string (7-bit length, then UTF-8).
//!
//! Profile data (versions 33–37, with player data 24–26 and inventory 101–106): version,
//! kills / deaths / crafts / builds, the worlds the character has been in (spawn points
//! and a mini-map each), name, player id, start seed, then the player data as one block:
//! health, stamina, guardian power, inventory (prefab name, stack, durability, grid
//! position, equipped, quality…), known recipes / stations / materials / biomes / texts,
//! looks, foods, and skills (id, level, progress). Gold is the stack of `Coins` items.
//!
//! Numbers are edited in place and the SHA-512 written again. Newer characters (profile 38
//! and up, a packed inventory) haven't been checked on a sample save, so they open read-only.
//!
//! Format facts from cquaid/valheim-fch-editor (MIT) and Kakoen/valheim-save-tools and
//! bkaznowski/valheim-character-editor (facts only), checked against sample characters.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use serde_json::{Map, Value};
use sha2::{Digest, Sha512};

/// Skill ids (the game's `Skills.SkillType`).
pub const SKILLS: [(i32, &str); 24] = [
    (1, "Swords"),
    (2, "Knives"),
    (3, "Clubs"),
    (4, "Polearms"),
    (5, "Spears"),
    (6, "Blocking"),
    (7, "Axes"),
    (8, "Bows"),
    (9, "Elemental Magic"),
    (10, "Blood Magic"),
    (11, "Unarmed"),
    (12, "Pickaxes"),
    (13, "Wood Cutting"),
    (14, "Crossbows"),
    (100, "Jump"),
    (101, "Sneak"),
    (102, "Run"),
    (103, "Swim"),
    (104, "Fishing"),
    (105, "Cooking"),
    (106, "Farming"),
    (107, "Crafting"),
    (108, "Dodge"),
    (110, "Ride"),
];

pub fn skill_name(id: i32) -> String {
    SKILLS.iter().find(|s| s.0 == id).map_or_else(|| format!("Skill {id}"), |s| s.1.to_string())
}

/// Profile versions whose layout has been checked on sample characters.
const PROFILE_OK: std::ops::RangeInclusive<i32> = 33..=37;
const PLAYER_OK: std::ops::RangeInclusive<i32> = 24..=26;
const INVENTORY_OK: std::ops::RangeInclusive<i32> = 101..=106;

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

pub fn looks(d: &[u8]) -> bool {
    let Some(n) = i32_at(d, 0).filter(|&n| n > 8) else { return false };
    let n = n as usize;
    d.len() == n + 4 + 4 + 64 && i32_at(d, 4 + n) == Some(64) && i32_at(d, 4).is_some_and(|v| (1..=200).contains(&v))
}

fn hash_ok(d: &[u8]) -> bool {
    let n = i32_at(d, 0).unwrap_or(0) as usize;
    d.get(4..4 + n).is_some_and(|p| Sha512::digest(p)[..] == d[8 + n..])
}

/// Writes the SHA-512 of the profile data after it.
fn rehash(out: &mut Vec<u8>) {
    let n = i32_at(out, 0).unwrap_or(0) as usize;
    if out.len() == n + 72 {
        let h = Sha512::digest(&out[4..4 + n]);
        out[8 + n..].copy_from_slice(&h);
    }
}

struct R<'a, 'b> {
    b: &'b mut Builder<'a>,
    pos: usize,
    /// End of the block being read.
    end: usize,
}

impl R<'_, '_> {
    fn take(&mut self, n: usize) -> Result<usize, String> {
        let at = self.pos;
        if at + n > self.end {
            return Err("the character ends early".into());
        }
        self.pos += n;
        Ok(at)
    }
    fn i32(&mut self) -> Result<i32, String> {
        let at = self.take(4)?;
        Ok(i32_at(self.b.d, at).unwrap())
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(self.i32()? as u32))
    }
    fn u8(&mut self) -> Result<u8, String> {
        let at = self.take(1)?;
        Ok(self.b.d[at])
    }
    fn count(&mut self) -> Result<usize, String> {
        let n = self.i32()?;
        if n < 0 || n as usize > self.end - self.pos {
            return Err("a list in the character is too long".into());
        }
        Ok(n as usize)
    }
    fn string(&mut self) -> Result<String, String> {
        let mut n = 0usize;
        for shift in (0..35).step_by(7) {
            let b = self.u8()?;
            n |= usize::from(b & 0x7F) << shift;
            if b & 0x80 == 0 {
                let at = self.take(n)?;
                return Ok(String::from_utf8_lossy(&self.b.d[at..at + n]).into_owned());
            }
        }
        Err("bad text length".into())
    }
    fn strings(&mut self) -> Result<Vec<String>, String> {
        let n = self.count()?;
        (0..n).map(|_| self.string()).collect()
    }
    fn pairs(&mut self) -> Result<(), String> {
        for _ in 0..self.count()? {
            self.string()?;
            self.string()?;
        }
        Ok(())
    }
}

/// A character this editor doesn't know the layout of: shown read-only with a reason.
fn read_only(d: &[u8], version: i32, why: String) -> TreeDoc {
    let mut b = Builder::new(d);
    b.info(&["Character", "Profile version"], version);
    b.info(&["Character", "Checksum"], if hash_ok(d) { "SHA-512 matches" } else { "SHA-512 doesn't match" });
    b.info(&["Character", "Read-only"], why.clone());
    b.finish("Valheim character", Checksum::None).with_note(why)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Valheim character".into());
    }
    let n = i32_at(d, 0).unwrap() as usize;
    let version = i32_at(d, 4).unwrap();
    if !PROFILE_OK.contains(&version) {
        let why = if version > *PROFILE_OK.end() {
            format!(
                "This character was saved by a newer Valheim (profile version {version}). Only versions 33 to 37 have been checked \
                 on sample characters, so it's shown read-only rather than risk a guessed layout."
            )
        } else {
            format!("This character is from an old Valheim (profile version {version}); only versions 33 to 37 are supported, so it's shown read-only.")
        };
        return Ok(read_only(d, version, why));
    }
    let mut b = Builder::new(d);
    let mut r = R { b: &mut b, pos: 8, end: 4 + n };
    let note = read_profile(&mut r, version)?;
    if let Some(why) = note {
        return Ok(read_only(d, version, why));
    }
    let ok = hash_ok(d);
    b.info(&["Character", "Profile version"], version);
    b.info(&["Character", "Checksum"], if ok { "SHA-512 matches (written again on save)" } else { "SHA-512 didn't match (written again on save)" });
    Ok(b.finish("Valheim character", Checksum::Custom(rehash)))
}

/// Reads the profile into the builder. `Ok(Some(reason))`: a part whose layout isn't known.
fn read_profile(r: &mut R, version: i32) -> Result<Option<String>, String> {
    for k in ["Kills", "Deaths", "Crafts", "Builds"] {
        let at = r.take(4)?;
        r.b.ranged(&["Stats", k], at, Kind::I32, 0, i32::MAX as i64);
    }
    for _ in 0..r.count()? {
        r.take(8)?; // world id
        for _ in 0..2 {
            r.u8()?; // custom spawn point / logout point (flag + position)
            r.take(12)?;
        }
        if version >= 30 {
            r.u8()?; // death point
            r.take(12)?;
        }
        r.take(12)?; // home point
        if version >= 29 && r.u8()? != 0 {
            let m = r.count()?;
            r.take(m)?; // mini-map
        }
    }
    let name = r.string()?;
    r.b.info(&["Character", "Name"], name);
    r.take(8)?; // player id
    r.string()?; // start seed
    if r.u8()? == 0 {
        return Ok(Some("This character has no player data yet (never entered a world), so there's nothing to edit.".into()));
    }
    let len = r.count()?;
    let (start, outer_end) = (r.pos, r.end);
    r.end = start + len;
    let note = read_player(r)?;
    if note.is_none() && r.pos != r.end {
        return Err("the player data has more in it than expected".into());
    }
    r.pos = start + len;
    r.end = outer_end;
    if note.is_none() && r.pos != r.end {
        return Err("the character has more in it than expected".into());
    }
    Ok(note)
}

fn read_player(r: &mut R) -> Result<Option<String>, String> {
    let pv = r.i32()?;
    if !PLAYER_OK.contains(&pv) {
        return Ok(Some(format!("Its player data has version {pv}; only 24 to 26 have been checked on sample characters, so it's shown read-only.")));
    }
    let max = r.f32()?;
    r.b.info(&["Vitals", "Max health"], crate::tree::f32_value(max));
    let at = r.take(4)?;
    r.b.ranged(&["Vitals", "Health"], at, Kind::F32, 0, 100_000);
    let at = r.take(4)?;
    r.b.ranged(&["Vitals", "Stamina"], at, Kind::F32, 0, 100_000);
    r.u8()?; // first spawn (player data versions 8 to 27)
    r.f32()?; // time since death
    let gp = r.string()?;
    r.b.info(&["Vitals", "Guardian power"], gp);
    r.f32()?; // guardian power cooldown

    let iv = r.i32()?;
    if !INVENTORY_OK.contains(&iv) {
        return Ok(Some(format!("Its inventory has version {iv}; only 101 to 106 are known, so it's shown read-only.")));
    }
    let mut keys = Map::new();
    for _ in 0..r.count()? {
        let name = r.string()?;
        let key = crate::tree::unique_key(&keys, &name);
        keys.insert(key.clone(), Value::Null);
        let at = r.take(4)?;
        r.b.ranged(&["Inventory", &key, "Stack"], at, Kind::I32, 0, i32::MAX as i64);
        let at = r.take(4)?;
        r.b.ranged(&["Inventory", &key, "Durability"], at, Kind::F32, 0, 1_000_000);
        let (x, y) = (r.i32()?, r.i32()?);
        r.b.info(&["Inventory", &key, "Slot"], format!("{x},{y}"));
        let eq = r.u8()? != 0;
        r.b.info(&["Inventory", &key, "Equipped"], eq);
        if iv >= 101 {
            let at = r.take(4)?;
            r.b.ranged(&["Inventory", &key, "Quality"], at, Kind::I32, 1, 100);
        }
        if iv >= 102 {
            r.i32()?; // variant
        }
        if iv >= 103 {
            r.take(8)?;
            let crafter = r.string()?;
            if !crafter.is_empty() {
                r.b.info(&["Inventory", &key, "Crafted by"], crafter);
            }
        }
        if iv >= 104 {
            r.pairs()?;
        }
        if iv >= 105 {
            r.i32()?; // world level
        }
        if iv >= 106 {
            r.u8()?; // picked up
        }
    }
    let recipes = r.strings()?.len();
    let stations = r.count()?;
    for _ in 0..stations {
        r.string()?;
        r.i32()?;
    }
    let materials = r.strings()?.len();
    r.strings()?; // tutorials shown
    r.strings()?; // uniques
    let trophies = r.strings()?;
    // Known biomes: ids in these versions.
    let biomes = r.count()?;
    r.take(biomes * 4)?;
    r.pairs()?; // known texts
    for (k, v) in [("Known recipes", recipes), ("Known materials", materials), ("Trophies", trophies.len()), ("Biomes found", biomes)] {
        r.b.info(&["Progress", k], v);
    }
    r.string()?; // beard
    r.string()?; // hair
    r.take(24)?; // skin and hair colour
    r.i32()?; // model
    let mut foods = vec![];
    for _ in 0..r.count()? {
        foods.push(Value::String(r.string()?));
        r.take(if pv >= 25 { 4 } else { 8 })?;
    }
    r.b.info(&["Vitals", "Foods eaten"], Value::Array(foods));
    let sv = r.i32()?;
    for _ in 0..r.count()? {
        let id = r.i32()?;
        let name = skill_name(id);
        let at = r.take(4)?;
        r.b.ranged(&["Skills", &name], at, Kind::F32, 0, 100);
        if sv >= 2 {
            r.take(4)?; // progress towards the next level
        }
    }
    if pv >= 26 {
        r.pairs()?; // custom data
        r.take(12)?; // stamina, max eitr, eitr
    }
    Ok(None)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn s(out: &mut Vec<u8>, t: &str) {
        out.push(t.len() as u8);
        out.extend(t.as_bytes());
    }
    fn i(out: &mut Vec<u8>, v: i32) {
        out.extend(v.to_le_bytes());
    }
    fn f(out: &mut Vec<u8>, v: f32) {
        out.extend(v.to_le_bytes());
    }

    /// A made-up profile-37 character with two items and two skills.
    pub fn synthetic(version: i32) -> Vec<u8> {
        synthetic_with(version, &[("Coins", 120, 1), ("SwordTest", 1, 3)])
    }

    /// The same character carrying `items` (prefab, stack, quality).
    pub fn synthetic_with(version: i32, items: &[(&str, i32, i32)]) -> Vec<u8> {
        let mut p = vec![];
        i(&mut p, 26); // player data version
        for v in [100.0, 75.5, 80.0] {
            f(&mut p, v);
        }
        p.push(0); // first spawn
        f(&mut p, 1.0);
        s(&mut p, "GP_Test");
        f(&mut p, 0.0);
        i(&mut p, 104);
        i(&mut p, items.len() as i32);
        for &(name, stack, q) in items {
            s(&mut p, name);
            i(&mut p, stack);
            f(&mut p, 50.0);
            i(&mut p, 1);
            i(&mut p, 0);
            p.push(0);
            i(&mut p, q);
            i(&mut p, 0);
            p.extend(0i64.to_le_bytes());
            s(&mut p, "");
            i(&mut p, 0);
        }
        i(&mut p, 1); // recipes
        s(&mut p, "Recipe_Test");
        i(&mut p, 1); // stations
        s(&mut p, "station");
        i(&mut p, 1);
        for _ in 0..4 {
            i(&mut p, 0);
        }
        i(&mut p, 1); // biomes
        i(&mut p, 1);
        i(&mut p, 0); // texts
        s(&mut p, "Beard1");
        s(&mut p, "Hair1");
        p.extend([0u8; 24]);
        i(&mut p, 0);
        i(&mut p, 1); // foods
        s(&mut p, "FoodTest");
        f(&mut p, 600.0);
        i(&mut p, 2); // skills
        i(&mut p, 2);
        for (id, lv) in [(1, 12.5f32), (102, 30.0)] {
            i(&mut p, id);
            f(&mut p, lv);
            f(&mut p, 0.25);
        }
        i(&mut p, 0); // custom data
        for v in [50.0, 0.0, 0.0] {
            f(&mut p, v);
        }

        let mut d = vec![];
        i(&mut d, version);
        for v in [1, 2, 3, 4] {
            i(&mut d, v);
        }
        i(&mut d, 1); // worlds
        d.extend(7i64.to_le_bytes());
        for _ in 0..3 {
            d.push(0);
            d.extend([0u8; 12]);
        }
        d.extend([0u8; 12]);
        d.push(1);
        i(&mut d, 3);
        d.extend([9, 9, 9]);
        s(&mut d, "Tester");
        d.extend(1i64.to_le_bytes());
        s(&mut d, "");
        d.push(1);
        i(&mut d, p.len() as i32);
        d.extend(p);

        let mut out = vec![];
        i(&mut out, d.len() as i32);
        out.extend(&d);
        i(&mut out, 64);
        out.extend(Sha512::digest(&d));
        out
    }

    #[test]
    fn reads_and_edits_a_synthetic_character() {
        let d = synthetic(37);
        assert!(looks(&d) && hash_ok(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Valheim character");
        let v = &doc.view;
        assert_eq!(v.pointer("/Character/Name"), Some(&Value::from("Tester")));
        assert_eq!(v.pointer("/Stats/Deaths"), Some(&Value::from(2)));
        assert_eq!(v.pointer("/Vitals/Health"), Some(&Value::from(75.5)));
        assert_eq!(v.pointer("/Inventory/Coins/Stack"), Some(&Value::from(120)));
        assert_eq!(v.pointer("/Inventory/SwordTest/Quality"), Some(&Value::from(3)));
        assert_eq!(v.pointer("/Skills/Run"), Some(&Value::from(30.0)));
        assert_eq!(v.pointer("/Progress/Known recipes"), Some(&Value::from(1)));
        assert!(doc.set("/Skills/Swords", Value::from(150.0)).is_err());
        doc.set("/Inventory/Coins/Stack", Value::from(999)).unwrap();
        doc.set("/Skills/Swords", Value::from(100.0)).unwrap();
        doc.set("/Vitals/Health", Value::from(100.0)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert!(hash_ok(&out));
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
        // Unchanged: the same bytes.
        assert_eq!(parse(&d).unwrap().to_bytes().unwrap(), d);
    }

    #[test]
    fn newer_characters_open_read_only() {
        let mut d = synthetic(37);
        d[4..8].copy_from_slice(&41i32.to_le_bytes());
        rehash(&mut d);
        let doc = parse(&d).unwrap();
        assert!(doc.editable.is_empty());
        assert!(doc.note.as_deref().unwrap().contains("profile version 41"));
        assert_eq!(doc.view.pointer("/Character/Profile version"), Some(&Value::from(41)));
    }

    /// VALHEIM_SAVES = real `.fch` files separated by `|`: each opens, writes back unchanged,
    /// and an edit (coins or a stack + 1, a skill) reads back with the rest unchanged and a
    /// valid SHA-512.
    #[test]
    #[ignore]
    fn real_characters() {
        let list = std::env::var("VALHEIM_SAVES").expect("set VALHEIM_SAVES");
        for path in list.split('|').filter(|s| !s.is_empty()) {
            let raw = std::fs::read(path).unwrap();
            let dec = crate::formats::decode(&raw);
            let crate::formats::Content::Tree(t) = &dec.content else { panic!("{path}: not a tree") };
            assert_eq!(t.kind_name, "Valheim character", "{path}");
            assert!(!t.editable.is_empty(), "{path}: read-only: {:?}", t.note);
            assert_eq!(crate::formats::encode(&dec.layers, &dec.content).unwrap(), raw, "{path}: unchanged differs");
            let mut doc = (**t).clone();
            let item = doc.view.get("Inventory").and_then(Value::as_object).and_then(|m| m.keys().find(|k| k.starts_with("Coins")).or_else(|| m.keys().next()).cloned());
            if let Some(item) = &item {
                let p = format!("/Inventory/{}/Stack", crate::tree::esc(item));
                let n = doc.view.pointer(&p).and_then(Value::as_i64).unwrap();
                doc.set(&p, Value::from(n + 1)).unwrap();
            }
            let skill = doc.view.get("Skills").and_then(Value::as_object).and_then(|m| m.keys().next().cloned());
            if let Some(sk) = &skill {
                doc.set(&format!("/Skills/{}", crate::tree::esc(sk)), Value::from(42.5)).unwrap();
            }
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), raw.len());
            assert!(hash_ok(&out), "{path}: hash");
            let diff = raw.iter().zip(&out).filter(|(a, b)| a != b).count();
            let back = parse(&out).unwrap();
            assert_eq!(back.view, doc.view, "{path}: reads back differently");
            println!("{path}: {} items, {} skills, edited {item:?} and {skill:?}: {diff} bytes differ (incl. 64-byte hash)", doc.view.get("Inventory").and_then(Value::as_object).map_or(0, |m| m.len()), doc.view.get("Skills").and_then(Value::as_object).map_or(0, |m| m.len()));
        }
    }
}
