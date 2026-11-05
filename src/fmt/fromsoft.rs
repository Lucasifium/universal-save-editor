//! FromSoftware character slots (one `USER_DATA00x` entry of an `.sl2` save):
//! Elden Ring, Dark Souls III, Dark Souls II: Scholar of the First Sin and Sekiro.
//!
//! A slot is a large binary blob; each game's player block holds the attributes, level
//! and currency at fixed distances from each other. The block is found by its own
//! consistency (level = attributes − a class constant, a valid name nearby), and only
//! those values are rewritten, in place.
//!
//! Layouts were checked against sample saves and published editors: souls_givifier
//! (Dark Souls III / Elden Ring souls next to the name), alfizari's Dark Souls II and
//! Sekiro editors (stat offsets). Items are in `sekiro_inv` and `er_inv`.

use super::{ds1_inv, ds2_inv, ds3_inv, er_inv, sekiro_inv};

/// A game's item lists (their leaves come after the numeric fields).
#[derive(Clone, Debug)]
enum Inventory {
    Sekiro(sekiro_inv::Inventory),
    EldenRing(er_inv::Inventory),
    DarkSouls3(ds3_inv::Inventory),
    DarkSouls2(ds2_inv::Inventory),
    DarkSouls1(ds1_inv::Inventory),
}

macro_rules! each {
    ($s:expr, $i:ident => $e:expr) => {
        match $s {
            Inventory::Sekiro($i) => $e,
            Inventory::EldenRing($i) => $e,
            Inventory::DarkSouls3($i) => $e,
            Inventory::DarkSouls2($i) => $e,
            Inventory::DarkSouls1($i) => $e,
        }
    };
}

impl Inventory {
    fn owns(&self, leaf: usize) -> bool {
        each!(self, i => i.owns(leaf))
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        each!(self, i => i.check(leaf, v))
    }
    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        each!(self, i => i.derived(leaf, view))
    }
    fn choices(&self, leaf: usize, view: &Value) -> Option<Arc<Vec<String>>> {
        each!(self, i => i.choices(leaf, view))
    }
    fn write(&self, out: &mut Vec<u8>, view: &Value, changes: &[(usize, Value)]) -> Result<(), String> {
        each!(self, i => i.write(out, view, changes))
    }
}
use crate::tree::{int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    EldenRing,
    DarkSouls3,
    DarkSouls2,
    DarkSouls1,
    Sekiro,
}

impl Game {
    pub fn name(self) -> &'static str {
        match self {
            Game::EldenRing => "Elden Ring character",
            Game::DarkSouls3 => "Dark Souls III character",
            Game::DarkSouls2 => "Dark Souls II character",
            Game::DarkSouls1 => "Dark Souls character",
            Game::Sekiro => "Sekiro character",
        }
    }
}

pub const ER_SLOT: usize = 0x28_0000;
const DS3_SLOT: usize = 0xC_0010;
const SEKIRO_SLOT: usize = 0x10_0000;
/// Dark Souls Remastered, decrypted (a length, the data, padding).
const DS1_SLOT: usize = 0x6_0010;

const ER_ATTRS: [&str; 8] = ["Vigor", "Mind", "Endurance", "Strength", "Dexterity", "Intelligence", "Faith", "Arcane"];
const ER_CLASSES: [&str; 10] = ["Vagabond", "Warrior", "Hero", "Bandit", "Astrologer", "Prophet", "Confessor", "Samurai", "Prisoner", "Wretch"];
/// Dark Souls III: (name, offset from Vigor). Two unused values sit before Vitality.
const DS3_ATTRS: [(&str, usize); 9] = [
    ("Vigor", 0x00),
    ("Attunement", 0x04),
    ("Endurance", 0x08),
    ("Vitality", 0x28),
    ("Strength", 0x0C),
    ("Dexterity", 0x10),
    ("Intelligence", 0x14),
    ("Faith", 0x18),
    ("Luck", 0x1C),
];
/// Dark Souls (Remastered): (name, offset); one byte each, eight bytes apart.
const DS1_ATTRS: [(&str, usize); 8] = [
    ("Vitality", 0x90),
    ("Attunement", 0x98),
    ("Endurance", 0xA0),
    ("Strength", 0xA8),
    ("Dexterity", 0xB0),
    ("Resistance", 0xD8),
    ("Intelligence", 0xB8),
    ("Faith", 0xC0),
];
pub const DS1_CLASSES: [&str; 10] = ["Warrior", "Knight", "Wanderer", "Thief", "Bandit", "Hunter", "Sorcerer", "Pyromancer", "Cleric", "Deprived"];
/// Each class's attribute total minus its starting soul level: soul level = total − this.
const DS1_LEVEL_MINUS: [u32; 10] = [82, 81, 83, 81, 82, 82, 79, 83, 82, 82];
/// Dark Souls II: (name, offset in the slot); 16-bit, stored in this order (not the menu's).
const DS2_ATTRS: [(&str, usize); 9] = [
    ("Vigor", 0x24),
    ("Endurance", 0x26),
    ("Vitality", 0x28),
    ("Attunement", 0x2A),
    ("Strength", 0x2C),
    ("Dexterity", 0x2E),
    ("Adaptability", 0x34),
    ("Intelligence", 0x30),
    ("Faith", 0x32),
];

fn u32_at(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    d.get(o..o + 2).map_or(0, |b| u16::from_le_bytes(b.try_into().unwrap()))
}

fn name_at(d: &[u8], o: usize) -> Option<String> {
    let units: Vec<u16> = d.get(o..o + 32)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
    let s = String::from_utf16(&units).ok()?;
    (!s.trim().is_empty() && s.chars().all(|c| !c.is_control())).then(|| s.trim_end().to_string())
}

/// Candidate offsets for a block of `n` u32 attributes: every byte where they're all 1..=99.
fn attr_blocks(d: &[u8], n: usize, extra: usize) -> impl Iterator<Item = usize> + '_ {
    (0..d.len().saturating_sub(n * 4 + extra)).filter(move |&o| (0..n).all(|i| (1..=99).contains(&u32_at(d, o + i * 4))))
}

/// Elden Ring: offset of Vigor (the block isn't always 4-byte aligned).
fn find_er(d: &[u8]) -> Option<usize> {
    if d.len() != ER_SLOT {
        return None;
    }
    attr_blocks(d, 8, 0x100).find(|&o| {
        let sum: u32 = (0..8).map(|i| u32_at(d, o + i * 4)).sum();
        o >= 0x40 && u32_at(d, o + 0x2C) + 79 == sum && name_at(d, o + 0x60).is_some()
    })
}

/// Dark Souls III: offset of Vigor.
fn find_ds3(d: &[u8]) -> Option<usize> {
    if d.len() != DS3_SLOT {
        return None;
    }
    attr_blocks(d, 8, 0x100).find(|&o| {
        let vit = u32_at(d, o + 0x28);
        let sum: u32 = (0..8).map(|i| u32_at(d, o + i * 4)).sum::<u32>() + vit;
        o >= 0x40
            && (1..=99).contains(&vit) && u32_at(d, o + 0x2C) + 89 == sum && name_at(d, o + 0x44).is_some()
    })
}

fn is_ds2(d: &[u8]) -> bool {
    if d.len() < 0x1000 || u32_at(d, 0) as usize != d.len() - 4 {
        return false;
    }
    let attrs: Vec<u16> = DS2_ATTRS.iter().map(|(_, o)| u16_at(d, *o)).collect();
    attrs.iter().all(|a| (1..=99).contains(a)) && u32_at(d, 0x3C) + 53 == attrs.iter().map(|&a| a as u32).sum::<u32>()
}

/// Dark Souls: the character's class, if the slot holds one (level = attributes − class constant).
fn ds1_class(d: &[u8]) -> Option<usize> {
    if d.len() != DS1_SLOT {
        return None;
    }
    let class = *d.get(0x11E)? as usize;
    let attrs: Vec<u32> = DS1_ATTRS.iter().map(|(_, o)| d[*o] as u32).collect();
    (class < 10
        && attrs.iter().all(|a| (1..=99).contains(a))
        && u16_at(d, 0xE0) as u32 + DS1_LEVEL_MINUS[class] == attrs.iter().sum::<u32>()
        && name_at(d, 0xF8).is_some())
    .then_some(class)
}

/// Sekiro: offset of current HP (HP, max HP and base max HP, then posture the same way).
fn find_sekiro(d: &[u8]) -> Option<usize> {
    if d.len() != SEKIRO_SLOT {
        return None;
    }
    (0..d.len() - 0x200).step_by(4).find(|&o| {
        let hp = [u32_at(d, o), u32_at(d, o + 4), u32_at(d, o + 8)];
        let posture = [u32_at(d, o + 0x1C), u32_at(d, o + 0x20), u32_at(d, o + 0x24)];
        (100..=20_000).contains(&hp[1])
            && (1..=hp[1]).contains(&hp[0])
            && (100..=hp[1]).contains(&hp[2])
            && (50..=5_000).contains(&posture[1])
            && posture[0] <= posture[1]
            && (50..=posture[1]).contains(&posture[2])
            && (1..=99).contains(&u32_at(d, o + 0x30))
    })
}

/// Which game's character this slot holds, if any.
pub fn detect(d: &[u8]) -> Option<Game> {
    match d.len() {
        ER_SLOT => find_er(d).map(|_| Game::EldenRing),
        DS3_SLOT => find_ds3(d).map(|_| Game::DarkSouls3),
        SEKIRO_SLOT => find_sekiro(d).map(|_| Game::Sekiro),
        DS1_SLOT => ds1_class(d).map(|_| Game::DarkSouls1),
        _ => is_ds2(d).then_some(Game::DarkSouls2),
    }
}

/// An editable number stored at one or more places (`offs`) with `width` bytes.
#[derive(Clone, Debug)]
struct Field {
    offs: Vec<usize>,
    width: usize,
    min: i128,
    max: i128,
    what: &'static str,
    /// Follows other values (level); edited only through them.
    derived: bool,
}

/// Level = sum of these leaves − `minus`.
#[derive(Clone, Debug)]
struct LevelRule {
    level_ptr: String,
    attr_ptrs: Vec<String>,
    minus: u64,
}

#[derive(Clone, Debug)]
struct SlotBackend {
    raw: Vec<u8>,
    fields: Vec<Field>,
    level: Option<LevelRule>,
    /// Leaves that make up the level.
    attr_leaves: Vec<usize>,
    /// Item lists (leaves after `fields`).
    inv: Option<Inventory>,
}

impl SlotBackend {
    fn inv(&self, leaf: usize) -> Option<&Inventory> {
        self.inv.as_ref().filter(|i| i.owns(leaf))
    }
}

impl Backend for SlotBackend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = self.raw.clone();
        for (leaf, v) in changes {
            if self.inv(*leaf).is_some() {
                continue;
            }
            let f = self.fields.get(*leaf).ok_or("unknown value")?;
            let n = int_in(v, f.min, f.max, f.what)? as u64;
            for &o in &f.offs {
                out[o..o + f.width].copy_from_slice(&n.to_le_bytes()[..f.width]);
            }
        }
        // Items last: adding a weapon or armor moves everything after the item table.
        if let Some(inv) = &self.inv {
            inv.write(&mut out, view, changes)?;
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        if let Some(inv) = self.inv(leaf) {
            return inv.check(leaf, v);
        }
        let f = self.fields.get(leaf).ok_or("unknown value")?;
        if f.derived {
            return Err("your level follows your attributes \u{2014} change those instead".into());
        }
        int_in(v, f.min, f.max, f.what).map(|_| ())
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        if let Some(inv) = self.inv(leaf) {
            return inv.derived(leaf, view);
        }
        let Some(rule) = &self.level else { return vec![] };
        if !self.attr_leaves.contains(&leaf) {
            return vec![];
        }
        let sum: u64 = rule.attr_ptrs.iter().map(|p| view.pointer(p).and_then(Value::as_u64).unwrap_or(0)).sum();
        vec![(rule.level_ptr.clone(), Value::from(sum.saturating_sub(rule.minus)))]
    }

    fn choices(&self, leaf: usize, view: &Value) -> Option<Arc<Vec<String>>> {
        self.inv(leaf)?.choices(leaf, view)
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Builds the view and the backend's field list together.
struct Builder<'a> {
    d: &'a [u8],
    root: Map<String, Value>,
    editable: HashMap<String, usize>,
    fields: Vec<Field>,
    inv: Option<Inventory>,
}

impl<'a> Builder<'a> {
    fn new(d: &'a [u8]) -> Self {
        Self { d, root: Map::new(), editable: HashMap::new(), fields: vec![], inv: None }
    }

    fn section(&mut self, s: &str) -> &mut Map<String, Value> {
        self.root.entry(s.to_string()).or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap()
    }

    fn read(&self, off: usize, width: usize) -> u64 {
        match width {
            1 => self.d.get(off).copied().unwrap_or(0) as u64,
            2 => u16_at(self.d, off) as u64,
            _ => u32_at(self.d, off) as u64,
        }
    }

    /// A value shown but not editable.
    fn show(&mut self, sec: &str, key: &str, v: impl Into<Value>) {
        self.section(sec).insert(key.into(), v.into());
    }

    /// An editable number (written to every offset in `offs`). Returns its leaf.
    fn num(&mut self, sec: &str, key: &str, offs: &[usize], width: usize, (min, max): (i128, i128), what: &'static str) -> usize {
        let v = self.read(offs[0], width);
        self.section(sec).insert(key.into(), Value::from(v));
        self.editable.insert(format!("/{sec}/{key}"), self.fields.len());
        self.fields.push(Field { offs: offs.to_vec(), width, min, max, what, derived: false });
        self.fields.len() - 1
    }

    fn finish(mut self, game: Game, level: Option<(&str, &str, u64)>, attr_leaves: Vec<usize>, note: &str) -> TreeDoc {
        let level = level.map(|(sec, key, minus)| {
            let p = format!("/{sec}/{key}");
            if let Some(&leaf) = self.editable.get(&p) {
                self.fields[leaf].derived = true;
            }
            let attr_ptrs = attr_leaves.iter().filter_map(|l| self.editable.iter().find(|(_, v)| *v == l).map(|(k, _)| k.clone())).collect();
            LevelRule { level_ptr: p, attr_ptrs, minus }
        });
        let backend = SlotBackend { raw: self.d.to_vec(), fields: self.fields, level, attr_leaves, inv: self.inv };
        TreeDoc::new(Value::Object(self.root), self.editable, game.name(), Box::new(backend)).with_note(note)
    }
}

const ATTR: (i128, i128) = (1, 99);
const CURRENCY: (i128, i128) = (0, 999_999_999);

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    match detect(d).ok_or("no FromSoftware character in this slot")? {
        Game::EldenRing => parse_er(d),
        Game::DarkSouls3 => parse_ds3(d),
        Game::DarkSouls2 => parse_ds2(d),
        Game::DarkSouls1 => parse_ds1(d),
        Game::Sekiro => parse_sekiro(d),
    }
}

/// HP / FP / Stamina: current, maximum and base maximum, 4 bytes apart.
fn pools(b: &mut Builder, pools: &[(&str, usize)]) {
    for &(name, o) in pools {
        b.show("Status", name, b.read(o, 4));
        b.show("Status", &format!("Max {name}"), b.read(o + 4, 4));
    }
}

fn parse_er(d: &[u8]) -> Result<TreeDoc, String> {
    let base = find_er(d).ok_or("no Elden Ring character in this slot")?;
    let mut b = Builder::new(d);
    b.show("Character", "Name", name_at(d, base + 0x60).unwrap_or_default());
    let class = d.get(base + 0x60 + 0x23).copied().unwrap_or(255) as usize;
    b.show("Character", "Starting class", ER_CLASSES.get(class).copied().unwrap_or("Unknown"));
    b.num("Character", "Level", &[base + 0x2C], 4, (1, 713), "Level");
    b.num("Character", "Runes", &[base + 0x30], 4, CURRENCY, "Runes");
    b.show("Character", "Rune memory", b.read(base + 0x34, 4));
    let attrs: Vec<usize> = ER_ATTRS.iter().enumerate().map(|(i, a)| b.num("Attributes", a, &[base + i * 4], 4, ATTR, "Attributes")).collect();
    pools(&mut b, &[("HP", base - 0x2C), ("FP", base - 0x20), ("Stamina", base - 0x10)]);
    b.inv = er_inv::build(d, base, b.fields.len(), &mut b.root, &mut b.editable).map(Inventory::EldenRing);
    let items = if b.inv.is_some() {
        "Items, Key items and Storage list your goods (items, materials, keys): change an item or its quantity (up to 999), \
         or pick an item for one of the empty slots at the end to add it. Weapons, armor, talismans and Ashes of War are counted, not edited. "
    } else {
        ""
    };
    let note = format!(
        "{items}Level is worked out from your attributes, and HP, FP and Stamina from those when the game loads. \
         The load screen keeps showing the old level until you save in-game. Close the game before saving. \
         Use Tools \u{203A} Open another file in this archive to pick another character slot."
    );
    Ok(b.finish(Game::EldenRing, Some(("Character", "Level", 79)), attrs, &note))
}

fn parse_ds3(d: &[u8]) -> Result<TreeDoc, String> {
    let base = find_ds3(d).ok_or("no Dark Souls III character in this slot")?;
    let mut b = Builder::new(d);
    b.show("Character", "Name", name_at(d, base + 0x44).unwrap_or_default());
    b.num("Character", "Soul level", &[base + 0x2C], 4, (1, 802), "Soul level");
    b.num("Character", "Souls", &[base + 0x30], 4, CURRENCY, "Souls");
    b.show("Character", "Soul memory", b.read(base + 0x34, 4));
    let attrs: Vec<usize> = DS3_ATTRS.iter().map(|(a, o)| b.num("Attributes", a, &[base + o], 4, ATTR, "Attributes")).collect();
    pools(&mut b, &[("HP", base - 0x2C), ("FP", base - 0x20), ("Stamina", base - 0x10)]);
    b.inv = ds3_inv::build(d, base, b.fields.len(), &mut b.root, &mut b.editable).map(Inventory::DarkSouls3);
    let items = if b.inv.is_some() {
        "Items, Storage, Weapons, Armor and Rings list what you have: change an item or its quantity (99 carried, 600 stored), \
         a weapon's level and infusion, or pick something for one of the empty slots at the end to add it. "
    } else {
        ""
    };
    let note = format!(
        "{items}Soul level is worked out from your attributes, and HP, FP and Stamina from those when the game loads. \
         The load screen keeps showing the old level until you save in-game. Close the game before saving. \
         Use Tools \u{203A} Open another file in this archive to pick another character slot."
    );
    Ok(b.finish(Game::DarkSouls3, Some(("Character", "Soul level", 89)), attrs, &note))
}

fn parse_ds2(d: &[u8]) -> Result<TreeDoc, String> {
    if !is_ds2(d) {
        return Err("no Dark Souls II character in this slot".into());
    }
    let mut b = Builder::new(d);
    b.show("Character", "Name", name_at(d, 0x3C4).unwrap_or_default());
    b.num("Character", "Soul level", &[0x3C], 4, (1, 838), "Soul level");
    b.num("Character", "Souls", &[0x40], 4, CURRENCY, "Souls");
    b.show("Character", "Soul memory", b.read(0x44, 4));
    let attrs: Vec<usize> = DS2_ATTRS.iter().map(|(a, o)| b.num("Attributes", a, &[*o], 2, ATTR, "Attributes")).collect();
    b.show("Status", "HP", b.read(0x4C, 4));
    b.inv = ds2_inv::build(d, b.fields.len(), &mut b.root, &mut b.editable).map(Inventory::DarkSouls2);
    Ok(b.finish(
        Game::DarkSouls2,
        Some(("Character", "Soul level", 53)),
        attrs,
        "Items, Item box, Weapons, Armor, Rings and Key items list what you have: change an item or its quantity \
         (99 carried, 999 in the box), a weapon's level (up to +5, or more if it's there already), \
         or pick something for one of the empty slots at the end to add it. \
         Soul level is worked out from your attributes. The load screen keeps showing the old values until you rest at a bonfire. \
         Close the game before saving. Use Tools \u{203A} Open another file in this archive to pick another character slot.",
    ))
}

fn parse_ds1(d: &[u8]) -> Result<TreeDoc, String> {
    let class = ds1_class(d).ok_or("no Dark Souls character in this slot")?;
    let mut b = Builder::new(d);
    b.show("Character", "Name", name_at(d, 0xF8).unwrap_or_default());
    b.show("Character", "Starting class", DS1_CLASSES[class]);
    b.num("Character", "Soul level", &[0xE0], 2, (1, 713), "Soul level");
    b.num("Character", "Souls", &[0xE4], 4, CURRENCY, "Souls");
    b.num("Character", "Humanity", &[0xD4], 1, (0, 99), "Humanity");
    let attrs: Vec<usize> = DS1_ATTRS.iter().map(|(a, o)| b.num("Attributes", a, &[*o], 1, ATTR, "Attributes")).collect();
    pools(&mut b, &[("HP", 0x64), ("Stamina", 0x80)]);
    b.inv = ds1_inv::build(d, b.fields.len(), &mut b.root, &mut b.editable).map(Inventory::DarkSouls1);
    Ok(b.finish(
        Game::DarkSouls1,
        Some(("Character", "Soul level", DS1_LEVEL_MINUS[class] as u64)),
        attrs,
        "Items, Key items, Weapons, Armor and Rings list what you have: change an item or its quantity (up to 99), \
         a weapon's level (up to +5, or more if it's there already), or pick something for one of the empty slots at the end to add it. \
         Soul level is worked out from your attributes, and HP and Stamina from those when the game loads. \
         Close the game before saving. Use Tools \u{203A} Open another file in this archive to pick another character slot.",
    ))
}

fn parse_sekiro(d: &[u8]) -> Result<TreeDoc, String> {
    let o = find_sekiro(d).ok_or("no Sekiro character in this slot")?;
    let mut b = Builder::new(d);
    b.num("Character", "Sen", &[o + 0x64], 4, CURRENCY, "Sen");
    b.num("Character", "Attack Power", &[o + 0x30], 4, (1, 99), "Attack Power");
    // How many Spirit Emblems can be carried; those carried are an item (Items, Spirit Emblem).
    b.num("Character", "Spirit Emblem capacity", &[o + 0x12E], 1, (0, 99), "Spirit Emblem capacity");
    // Vitality and posture are stored three times (current, maximum, base); edit all together.
    b.num("Status", "Vitality", &[o, o + 4, o + 8], 4, (1, 99_999), "Vitality");
    b.num("Status", "Posture", &[o + 0x1C, o + 0x20, o + 0x24], 4, (1, 99_999), "Posture");
    // Skills: experience toward the next skill point (the game hands out a point once it's
    // full), how many points have been earned and all experience ever gained. Points to
    // spend are an item (Items, Skill Point).
    b.num("Skills", "Skill experience", &[o + 0x148], 4, (0, 999_999_999), "Skill experience");
    b.show("Skills", "Skill points earned", b.read(o + 0x13C, 4));
    b.show("Skills", "Total experience", b.read(o + 0x140, 4));
    b.inv = sekiro_inv::build(d, b.fields.len(), &mut b.root, &mut b.editable).map(Inventory::Sekiro);
    let items = if b.inv.is_some() {
        "Items, Key items and Storage list what you have: change an item or its quantity, or pick an item for an empty slot to add it. \
         The game caps what you carry at each item's limit and keeps the rest in Storage. \
         Key items belong in Key items. "
    } else {
        ""
    };
    let note = format!(
        "{items}Vitality and Posture are the bars' sizes (Prayer Beads and Gourd Seeds raise them in the game). \
         Close the game before saving. Use Tools \u{203A} Open another file in this archive to pick another save slot."
    );
    Ok(b.finish(Game::Sekiro, None, vec![], &note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put32(d: &mut [u8], o: usize, v: u32) {
        d[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn put_name(d: &mut [u8], o: usize, name: &str) {
        for (i, u) in name.encode_utf16().enumerate() {
            d[o + i * 2..o + i * 2 + 2].copy_from_slice(&u.to_le_bytes());
        }
    }

    #[test]
    fn elden_ring_unaligned_block() {
        let mut d = vec![0u8; ER_SLOT];
        let base = 0xAFC5; // as in a sample save: not 4-byte aligned
        for i in 0..8 {
            put32(&mut d, base + i * 4, 10);
        }
        put32(&mut d, base + 0x2C, 1);
        put32(&mut d, base + 0x30, 500);
        put32(&mut d, base - 0x28, 400);
        put_name(&mut d, base + 0x60, "Tarnished");
        d[base + 0x60 + 0x23] = 9;
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Elden Ring character");
        assert_eq!(doc.view.pointer("/Character/Name").unwrap(), "Tarnished");
        assert_eq!(doc.view.pointer("/Character/Starting class").unwrap(), "Wretch");
        assert_eq!(doc.view.pointer("/Status/Max HP").unwrap(), 400);
        assert!(doc.set("/Character/Level", Value::from(50)).is_err());
        doc.set("/Attributes/Vigor", Value::from(40)).unwrap();
        assert_eq!(doc.view.pointer("/Character/Level").unwrap(), 31);
        doc.set("/Character/Runes", Value::from(1_000_000)).unwrap();
        assert!(doc.set("/Attributes/Mind", Value::from(100)).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, base + 0x2C), 31);
        assert_eq!(parse(&out).unwrap().view, doc.view);
    }

    #[test]
    fn elden_ring_items() {
        use super::er_inv::tests::{add, blank, quick, storage_at};
        let mut d = vec![0u8; ER_SLOT];
        let base = 0xAFC5;
        for i in 0..8 {
            put32(&mut d, base + i * 4, 10);
        }
        put32(&mut d, base + 0x2C, 1);
        put_name(&mut d, base + 0x60, "Tarnished");
        blank(&mut d, base, 3);
        add(&mut d, base, 0, 0, 10100, 10); // Smithing Stone [1]
        add(&mut d, base, 0, 1, 2900, 5); // Golden Rune [1], in a quick slot
        quick(&mut d, base, 2900);
        add(&mut d, base, 1, 0, 8000, 2); // Stonesword Key
        add(&mut d, base, 2, 0, 10100, 400);
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
        assert_eq!(v(&doc, "/Items/Slot 1/Item"), "Smithing Stone [1]");
        assert_eq!(v(&doc, "/Key items/Slot 1/Item"), "Stonesword Key");
        assert_eq!(v(&doc, "/Storage/Slot 1/Quantity"), 400);
        assert!(v(&doc, "/Items/Slot 3").is_null(), "only the empty slots at the end are offered");
        assert_eq!(v(&doc, "/Items/Slot 2625/Item"), sekiro_inv::EMPTY);

        assert!(doc.set("/Items/Slot 1/Item", Value::from(sekiro_inv::EMPTY)).is_err(), "not one of the offered slots");
        assert!(doc.set("/Items/Slot 2/Item", Value::from("Smithing Stone [2]")).is_err(), "in a quick slot");
        assert!(doc.set("/Items/Slot 1/Quantity", Value::from(1000)).is_err());
        doc.set("/Items/Slot 2/Quantity", Value::from(20)).unwrap();
        doc.set("/Storage/Slot 1/Quantity", Value::from(999)).unwrap();

        // The "Max upgrade materials" button.
        let root = doc.view.clone();
        let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: Some(&doc), edits: vec![] };
        crate::game_ui::give_materials(crate::game_ui::Game::EldenRing, &mut cx);
        let edits = cx.edits;
        for (p, val) in edits {
            doc.set(&p, val).unwrap();
        }
        assert_eq!(v(&doc, "/Items/Slot 1/Quantity"), 999);
        assert_eq!(v(&doc, "/Items/Slot 2625/Item"), "Smithing Stone [2]");
        assert_eq!(v(&doc, "/Items/Slot 2662/Item"), "Great Ghost Glovewort");

        let out = doc.to_bytes().unwrap();
        assert_eq!(parse(&out).unwrap().view, doc.view, "saving reads back exactly what's on screen");
        let inv = base + 0x370;
        assert_eq!(u32_at(&out, inv), 2 + 38, "carried count");
        let e = inv + 4 + 2624 * 12;
        assert_eq!([u32_at(&out, e), u32_at(&out, e + 4), u32_at(&out, e + 8)], [0xB000_0000 | 10101, 999, 100]);
        let next = inv + 4 + 0xA80 * 12 + 4 + 0x180 * 12;
        assert_eq!([u32_at(&out, next), u32_at(&out, next + 4)], [10 + 38, 100 + 38]);
        let st = storage_at(&out, base).unwrap();
        assert_eq!(u32_at(&out, st), 1);
        assert_eq!(u32_at(&out, st + 8), 999);
    }

    #[test]
    fn elden_ring_weapons_and_armor() {
        use crate::fmt::er_equip::{ARMOR, TALISMANS, WEAPONS};
        use crate::fmt::er_inv::tests::{add_handle, blank, mark_end, wear, with_table};
        let mut d = vec![0u8; ER_SLOT];
        // Heavy Dagger +5, and a Longsword that's worn.
        let base = with_table(&mut d, &[(0x8080_0010, 1_000_000 + 100 + 5), (0x8080_0011, 2_000_000)]);
        for i in 0..8 {
            put32(&mut d, base + i * 4, 10);
        }
        put32(&mut d, base + 0x2C, 1);
        put_name(&mut d, base + 0x60, "Tarnished");
        blank(&mut d, base, 0);
        add_handle(&mut d, base, 0, 0, 0x8080_0010, 1);
        add_handle(&mut d, base, 0, 1, 0x8080_0011, 1);
        wear(&mut d, base, 0x8080_0011);
        mark_end(&mut d, base);
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
        assert_eq!(v(&doc, "/Weapons/Slot 1/Item"), "Dagger");
        assert_eq!(v(&doc, "/Weapons/Slot 1/Level"), 5);
        assert_eq!(v(&doc, "/Weapons/Slot 1/Affinity"), "Heavy");
        assert_eq!(v(&doc, "/Weapons/Slot 2/Item"), "Longsword");

        doc.set("/Weapons/Slot 1/Level", Value::from(25)).unwrap();
        doc.set("/Weapons/Slot 1/Affinity", Value::from("Keen")).unwrap();
        assert!(doc.set("/Weapons/Slot 1/Level", Value::from(26)).is_err());
        assert!(doc.set("/Weapons/Slot 2/Item", Value::from("Dagger")).is_err(), "worn");
        assert!(doc.set("/Weapons/Slot 1/Item", Value::from(sekiro_inv::EMPTY)).is_err(), "not added here");

        // Adding: a weapon that can take affinities, armor and a talisman.
        let (&(_, sword, _, _), somber) = (
            WEAPONS.iter().find(|w| w.2 == Some(25) && w.3 != 0).unwrap(),
            WEAPONS.iter().find(|w| w.2 == Some(10) && w.3 == 0).unwrap(),
        );
        doc.set("/Weapons/Slot 2593/Item", Value::from(sword)).unwrap();
        assert_eq!(v(&doc, "/Weapons/Slot 2593/Affinity"), "Standard");
        doc.set("/Weapons/Slot 2593/Level", Value::from(25)).unwrap();
        doc.set("/Weapons/Slot 2594/Item", Value::from(somber.1)).unwrap();
        doc.set("/Weapons/Slot 2594/Level", Value::from(25)).unwrap();
        assert_eq!(v(&doc, "/Weapons/Slot 2594/Level"), 10, "somber weapons stop at +10");
        assert_eq!(v(&doc, "/Weapons/Slot 2594/Affinity"), super::er_inv::NO_AFFINITY);
        doc.set("/Armor/Slot 2561/Item", Value::from(ARMOR[0].1)).unwrap();
        doc.set("/Talismans/Slot 2545/Item", Value::from(TALISMANS[0].1)).unwrap();

        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap();
        assert_eq!(back.view, doc.view, "saving reads back exactly what's on screen");
        // The character data moved by the two new weapons and the armor (13 + 13 + 8 bytes).
        let moved = super::find_er(&out).unwrap();
        assert_eq!(moved, base + 34);
        assert_eq!(u32_at(&out, moved + 0x370), 2 + 4, "carried count");
        assert_eq!(u32_at(&out, 0x20 + 4), 1_000_000 + 200 + 25, "Keen Dagger +25");

        // Removing what was added puts the item table back as it was.
        let mut doc2 = back;
        for p in ["/Weapons/Slot 2593/Item", "/Weapons/Slot 2594/Item", "/Armor/Slot 2561/Item", "/Talismans/Slot 2545/Item"] {
            doc2.set(p, Value::from(sekiro_inv::EMPTY)).unwrap();
        }
        let out2 = doc2.to_bytes().unwrap();
        assert_eq!(parse(&out2).unwrap().view, doc2.view);
        assert_eq!(super::find_er(&out2), Some(base));
        assert_eq!(out2[0x20 + 42..base], d[0x20 + 42..base], "the free table entries are back");
        assert_eq!(u32_at(&out2, base + 0x370), 2);
    }

    #[test]
    fn dark_souls_3_items() {
        use crate::fmt::ds3_inv::tests::{add, wear, with_table};
        let mut d = vec![0u8; DS3_SLOT];
        // A Long Sword +3 in the item table, worn.
        let base = with_table(&mut d, &[(0x8081_0010, 2_000_003)]);
        for (i, v) in [12u32, 10, 11, 13, 12, 9, 9, 7].iter().enumerate() {
            put32(&mut d, base + i * 4, *v);
        }
        put32(&mut d, base + 0x28, 15);
        put32(&mut d, base + 0x2C, 9);
        put_name(&mut d, base + 0x44, "Ash");
        add(&mut d, base, 0, 0, 0x8081_0010, 2_000_003, 1);
        add(&mut d, base, 0, 1, 0xB000_03E8, 0x4000_03E8, 4); // Titanite Shard
        wear(&mut d, base, 0x8081_0010);
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
        assert_eq!(v(&doc, "/Weapons/Slot 1/Level"), 3);
        assert_eq!(v(&doc, "/Items/Slot 2/Item"), "Titanite Shard");
        assert!(doc.set("/Weapons/Slot 1/Item", Value::from("Dagger")).is_err(), "worn");
        doc.set("/Weapons/Slot 1/Level", Value::from(10)).unwrap();
        doc.set("/Items/Slot 2/Quantity", Value::from(99)).unwrap();
        assert!(doc.set("/Items/Slot 2/Quantity", Value::from(100)).is_err());
        // A new weapon in the reserved slots (0x780 - 64 - 32 = 1824).
        doc.set("/Weapons/Slot 1825/Item", Value::from("Dagger")).unwrap();
        doc.set("/Weapons/Slot 1825/Infusion", Value::from("Sharp")).unwrap();
        assert!(doc.set("/Weapons/Slot 1825/Level", Value::from(11)).is_err());
        doc.set("/Weapons/Slot 1825/Level", Value::from(10)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(&out[out.len() - 13..], &d[d.len() - 13..], "the slot's last bytes stay");
        let back = parse(&out).unwrap();
        assert_eq!(back.view, doc.view);
        assert_eq!(super::find_ds3(&out), Some(base + 52), "the item table grew by one weapon");
    }

    #[test]
    fn dark_souls_2_items() {
        let mut d = vec![0u8; 0x1B2C0];
        let n = d.len() as u32 - 4;
        put32(&mut d, 0, n);
        for (i, v) in [7u16, 6, 6, 5, 15, 11, 5, 5, 5].iter().enumerate() {
            d[0x24 + i * 2..0x26 + i * 2].copy_from_slice(&v.to_le_bytes());
        }
        put32(&mut d, 0x3C, 12);
        d[0x11E34..0x11E34 + 84].fill(0xFF);
        // Broadsword +2 (durability 60.0), worn; 3 Lifegems.
        let e = |d: &mut Vec<u8>, slot: usize, vals: [u32; 4]| {
            for (k, v) in vals.iter().enumerate() {
                put32(d, 0x1E30 + slot * 16 + k * 4, *v);
            }
        };
        e(&mut d, 0, [1_230_000, 0, 60f32.to_bits(), 2]);
        e(&mut d, 1, [60_010_000, 0, 3, 0]);
        d[0x11E34..0x11E36].copy_from_slice(&0u16.to_le_bytes());
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
        assert_eq!(v(&doc, "/Weapons/Slot 1/Item"), "Broadsword");
        assert_eq!(v(&doc, "/Weapons/Slot 1/Level"), 2);
        assert_eq!(v(&doc, "/Items/Slot 2/Item"), "Lifegem");
        assert!(doc.set("/Weapons/Slot 1/Item", Value::from("Dagger")).is_err(), "worn");
        doc.set("/Weapons/Slot 1/Level", Value::from(9)).unwrap();
        assert_eq!(v(&doc, "/Weapons/Slot 1/Level"), 5, "+5 at most");
        doc.set("/Weapons/Slot 1953/Item", Value::from("Dagger")).unwrap();
        doc.set("/Rings/Slot 1905/Item", Value::from("Life Ring")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(parse(&out).unwrap().view, doc.view);
        let o = 0x1E30 + 1952 * 16;
        assert_eq!(u32_at(&out, o + 8), 60f32.to_bits(), "a new weapon gets an owned weapon's durability");
    }

    #[test]
    fn dark_souls_1_character() {
        use crate::fmt::ds1_inv::tests::{add, blank, counts, equip};
        let mut d = vec![0u8; DS1_SLOT];
        blank(&mut d);
        // A Knight: Vit 14, Att 10, End 10, Str 11, Dex 11, Res 10, Int 9, Fth 11 → SL 5.
        for ((_, o), v) in DS1_ATTRS.iter().zip([14u8, 10, 10, 11, 11, 10, 9, 11]) {
            d[*o] = v;
        }
        d[0x11E] = 1;
        d[0xE0] = 5;
        put32(&mut d, 0xE4, 1000);
        put_name(&mut d, 0xF8, "Oscar");
        add(&mut d, 64, 0, 201_003, 1); // Longsword +3
        equip(&mut d, 64);
        add(&mut d, 65, 4, 1000, 7); // Titanite Shard
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Dark Souls character");
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
        assert_eq!(v(&doc, "/Character/Starting class"), "Knight");
        assert_eq!(v(&doc, "/Items/Slot 66/Item"), "Titanite Shard");
        doc.set("/Attributes/Strength", Value::from(20)).unwrap();
        assert_eq!(v(&doc, "/Character/Soul level"), 14);
        doc.set("/Items/Slot 66/Quantity", Value::from(99)).unwrap();
        doc.set("/Weapons/Slot 65/Level", Value::from(6)).unwrap();
        assert_eq!(v(&doc, "/Weapons/Slot 65/Level"), 5, "+5 at most here");
        doc.set("/Weapons/Slot 65/Level", Value::from(5)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(parse(&out).unwrap().view, doc.view);
        assert_eq!(out[0xA8], 20);
        assert_eq!(u32_at(&out, 0x360 + 64 * 28 + 4), 201_005);
        assert_eq!(u32_at(&out, 0x304 + 4), 201_005, "the equipped copy follows");
        assert!(doc.set("/Weapons/Slot 65/Item", Value::from("Dagger")).is_err(), "equipped");
        assert!(doc.set("/Items/Slot 66/Item", Value::from("(empty)")).is_err(), "not one of the slots for new items");

        // Adding: a second Longsword (copies the first's sort key and durability), a ring,
        // armor and a material, in the slots kept for new items.
        let mut doc = parse(&out).unwrap();
        assert_eq!(counts(&out), (2, 0, 65));
        doc.set("/Weapons/Slot 1953/Item", Value::from("Longsword")).unwrap();
        doc.set("/Weapons/Slot 1953/Level", Value::from(4)).unwrap();
        assert_eq!(v(&doc, "/Weapons/Slot 1953/Infusion"), "Normal");
        doc.set("/Rings/Slot 1905/Item", Value::from("Havel's Ring")).unwrap();
        doc.set("/Armor/Slot 1921/Item", Value::from("Catarina Helm")).unwrap();
        doc.set("/Items/Slot 1985/Item", Value::from("Green Titanite Shard")).unwrap();
        doc.set("/Items/Slot 1985/Quantity", Value::from(12)).unwrap();
        assert!(!doc.choices("/Items/Slot 1986/Item").unwrap().contains(&"Titanite Shard".to_string()), "already carried");
        let added = doc.to_bytes().unwrap();
        assert_eq!(parse(&added).unwrap().view, doc.view, "saving reads back what's on screen");
        assert_eq!(counts(&added), (6, 0, 1984));
        let e = |d: &[u8], slot: usize| (0..7).map(|k| u32_at(d, 0x360 + slot * 28 + k * 4)).collect::<Vec<u32>>();
        assert_eq!(e(&added, 1952), vec![0, 201_004, 1, (7 << 12) | 1952, 1, 200, 0]);
        assert_eq!(e(&added, 1984)[..5], [4 << 28, 1020, 12, 1984, 1]);
        assert_eq!(e(&added, 1904)[..2], [2 << 28, 100]);

        // Removing what was added.
        let mut doc = parse(&added).unwrap();
        for p in ["/Weapons/Slot 1953/Item", "/Rings/Slot 1905/Item", "/Armor/Slot 1921/Item", "/Items/Slot 1985/Item"] {
            doc.set(p, Value::from("(empty)")).unwrap();
        }
        let removed = doc.to_bytes().unwrap();
        assert_eq!(parse(&removed).unwrap().view, doc.view);
        assert_eq!(counts(&removed).0, 2);
        assert_eq!(e(&removed, 1952)[4], 0, "unused again");
    }

    #[test]
    fn dark_souls_3_knight() {
        let mut d = vec![0u8; DS3_SLOT];
        let base = 0x115F4;
        // Knight: Vig 12, Att 10, End 11, Str 13, Dex 12, Int 9, Fth 9, Lck 7, Vit 15 → SL 9.
        for (i, v) in [12u32, 10, 11, 13, 12, 9, 9, 7].iter().enumerate() {
            put32(&mut d, base + i * 4, *v);
        }
        put32(&mut d, base + 0x28, 15);
        put32(&mut d, base + 0x2C, 9);
        put32(&mut d, base + 0x30, 60);
        put_name(&mut d, base + 0x44, "Ash");
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Dark Souls III character");
        assert_eq!(doc.view.pointer("/Attributes/Vitality").unwrap(), 15);
        doc.set("/Attributes/Luck", Value::from(17)).unwrap();
        assert_eq!(doc.view.pointer("/Character/Soul level").unwrap(), 19);
        doc.set("/Character/Souls", Value::from(123_456)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, base + 0x1C), 17);
        assert_eq!(u32_at(&out, base + 0x2C), 19);
        assert_eq!(u32_at(&out, base + 0x30), 123_456);
        assert_eq!(parse(&out).unwrap().view, doc.view);
    }

    #[test]
    fn dark_souls_2_stat_order() {
        let mut d = vec![0u8; 0x1B2C0];
        let n = d.len() as u32 - 4;
        put32(&mut d, 0, n);
        // Warrior: Vig 7, End 6, Vit 6, Att 5, Str 15, Dex 11, Int 5, Fth 5, Adp 5 → SL 12.
        for (i, v) in [7u16, 6, 6, 5, 15, 11, 5, 5, 5].iter().enumerate() {
            d[0x24 + i * 2..0x26 + i * 2].copy_from_slice(&v.to_le_bytes());
        }
        put32(&mut d, 0x3C, 12);
        put_name(&mut d, 0x3C4, "Bearer");
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Dark Souls II character");
        assert_eq!(doc.view.pointer("/Attributes/Strength").unwrap(), 15);
        doc.set("/Attributes/Adaptability", Value::from(25)).unwrap();
        assert_eq!(doc.view.pointer("/Character/Soul level").unwrap(), 32);
        let out = doc.to_bytes().unwrap();
        assert_eq!(u16_at(&out, 0x34), 25, "Adaptability is stored after Faith");
        assert_eq!(u32_at(&out, 0x3C), 32);
    }

    #[test]
    fn sekiro_bars_change_together() {
        let mut d = vec![0u8; SEKIRO_SLOT];
        let o = 0x3446C;
        for k in 0..3 {
            put32(&mut d, o + k * 4, 960);
            put32(&mut d, o + 0x1C + k * 4, 360);
        }
        put32(&mut d, o + 0x30, 27);
        put32(&mut d, o + 0x64, 850);
        d[o + 0x12E] = 19;
        put32(&mut d, o + 0x13C, 115); // skill points earned
        put32(&mut d, o + 0x148, 46_244); // skill experience
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Sekiro character");
        assert_eq!(doc.view.pointer("/Skills/Skill points earned").unwrap(), 115);
        doc.set("/Skills/Skill experience", Value::from(90_000)).unwrap();
        assert_eq!(doc.view.pointer("/Character/Spirit Emblem capacity").unwrap(), 19);
        assert!(doc.view.pointer("/Items").is_none(), "no item tables in this buffer");
        doc.set("/Status/Vitality", Value::from(1200)).unwrap();
        doc.set("/Character/Sen", Value::from(9999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!([u32_at(&out, o), u32_at(&out, o + 4), u32_at(&out, o + 8)], [1200; 3]);
        assert_eq!(u32_at(&out, o + 0x64), 9999);
        assert_eq!(out[o + 0x12E], 19);
        assert_eq!(u32_at(&out, o + 0x148), 90_000);
        assert_eq!(u32_at(&out, o + 0x13C), 115);
    }

    fn sekiro_slot(t: usize) -> Vec<u8> {
        use super::sekiro_inv::tests::{add, blank, quick};
        let mut d = blank(SEKIRO_SLOT, t);
        let o = 0x3446C;
        for k in 0..3 {
            put32(&mut d, o + k * 4, 960);
            put32(&mut d, o + 0x1C + k * 4, 360);
        }
        put32(&mut d, o + 0x30, 27);
        add(&mut d, t, 0, 0, 1000, 17); // Spirit Emblem
        add(&mut d, t, 0, 1, 3000, 4); // Healing Gourd, in a quick-item slot
        quick(&mut d, t, 3000);
        add(&mut d, t, 1, 0, 2300, 1); // Kusabimaru
        add(&mut d, t, 2, 0, 1000, 500);
        d
    }

    #[test]
    fn sekiro_items() {
        let t = 0x8F70C;
        let d = sekiro_slot(t);
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
        assert_eq!(v(&doc, "/Items/Slot 1/Item"), "Spirit Emblem");
        assert_eq!(v(&doc, "/Items/Slot 1/Quantity"), 17);
        assert_eq!(v(&doc, "/Key items/Slot 1/Item"), "Kusabimaru");
        assert_eq!(v(&doc, "/Storage/Slot 1/Quantity"), 500);
        assert_eq!(v(&doc, "/Items/Slot 3/Item"), sekiro_inv::EMPTY);
        assert_eq!(v(&doc, "/Items/Slot 320/Item"), sekiro_inv::EMPTY);
        assert!(v(&doc, "/Items/Slot 321").is_null(), "a fixed number of slots is listed");

        // The gourd is on a quick-item slot: its quantity can change, the item can't.
        assert!(doc.set("/Items/Slot 2/Item", Value::from("Oil")).is_err());
        doc.set("/Items/Slot 2/Quantity", Value::from(9)).unwrap();
        // Adding: an empty slot gets an item and at least one of it.
        doc.set("/Items/Slot 3/Item", Value::from("Oil")).unwrap();
        assert_eq!(v(&doc, "/Items/Slot 3/Quantity"), 1);
        doc.set("/Items/Slot 3/Quantity", Value::from(12)).unwrap();
        assert!(!doc.choices("/Items/Slot 4/Item").unwrap().contains(&"Oil".to_string()), "already carried");
        assert!(doc.choices("/Key items/Slot 2/Item").unwrap().contains(&"Oil".to_string()), "other tables are separate");
        assert!(doc.set("/Items/Slot 4/Item", Value::from("Excalibur")).is_err());
        // Removing.
        doc.set("/Items/Slot 1/Item", Value::from(sekiro_inv::EMPTY)).unwrap();
        assert_eq!(v(&doc, "/Items/Slot 1/Quantity"), 0);
        doc.set("/Storage/Slot 1/Quantity", Value::from(999)).unwrap();
        assert!(doc.set("/Storage/Slot 1/Quantity", Value::from(10_000)).is_err());

        let out = doc.to_bytes().unwrap();
        let entry = |o: usize| [u32_at(&out, o), u32_at(&out, o + 4), u32_at(&out, o + 8), u32_at(&out, o + 12)];
        assert_eq!(entry(t), [0, u32::MAX, 0, 0]);
        assert_eq!(entry(t + 16)[2], 9);
        let oil = entry(t + 32);
        assert_eq!(oil[..3], [0xB000_0000 | 3520, 0x4000_0000 | 3520, 12]);
        assert_eq!(oil[3], 0x102, "the next index, after the two already given out");
        assert_eq!(u32_at(&out, t - 0x10), 0x102, "last index");
        assert_eq!(u32_at(&out, t - 0xC), 2, "carried count: one added, one removed");
        assert_eq!(u32_at(&out, t - 8), 1, "key items unchanged");
        // Saving re-reads the file and needs exactly what's on screen.
        assert_eq!(parse(&out).unwrap().view, doc.view);
        // Nothing outside the edited entries and headers moved.
        let changed: Vec<usize> = (0..d.len()).filter(|&i| d[i] != out[i]).collect();
        assert!(changed.iter().all(|&i| (t - 0x10..t + 0x30).contains(&i) || (t + 0x9094..t + 0x90A4).contains(&i)), "{changed:x?}");
    }

    #[test]
    fn sekiro_items_found_when_shifted() {
        let t = 0x8E6FC; // where a real slot keeps them
        let doc = parse(&sekiro_slot(t)).unwrap();
        assert_eq!(doc.view.pointer("/Items/Slot 2/Item").unwrap(), "Healing Gourd");
        assert_eq!(doc.to_bytes().unwrap(), sekiro_slot(t));
    }

    /// Real slots dumped by `bughunt::dump_fromsoft_slots` (SAVE_EDITOR_SLOTS = folder).
    #[test]
    #[ignore]
    fn real_slots() {
        let dir = std::env::var("SAVE_EDITOR_SLOTS").expect("SAVE_EDITOR_SLOTS");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if let Ok(doc) = parse(&d) {
                println!("{}: {}\n  {}", e.file_name().to_string_lossy(), doc.kind_name, doc.view);
                assert_eq!(doc.to_bytes().unwrap(), d, "unchanged slot must round-trip");
            }
        }
    }
}
