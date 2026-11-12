//! Final Fantasy VII Remake (Intergrade) and Rebirth slot saves (`ff7remake000.sav`,
//! `ff7remakeplus000.sav`, `ff7rebirth000.sav` …). Neither is encrypted or checksummed.
//! After a 16-byte header and the "RESD" marker comes the party: a count, then one record
//! per character (0x40 bytes in Remake, 0x100 in Rebirth) starting with the level, with
//! current/max HP, current/max MP and experience at +0x10.
//!
//! The inventory is a fixed table of 2048 records further in (see `Table`); gil is item 20
//! in Remake, and items 1, 2 and 11 are gil, Moogle Medals and Gold Saucer Points in Rebirth.
//! Remake characters wear armor and accessories by inventory slot number.
//!
//! Worked out from real saves (current HP below max where a slot was saved hurt, levels
//! matching the game's caps, and currency amounts matching what a memory table showed for
//! the same slot); only these values are offered.

use super::ff7r_items as items;
use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const MARK: &[u8] = b"RESDRESDRESDRESD";
const FIRST: usize = 0x30;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The record size that makes every used record look like a character, if one does.
fn layout(d: &[u8]) -> Option<(usize, usize)> {
    if d.len() < 0x200 || d[..4] != [0, 0, 1, 0] || &d[0x10..0x20] != MARK {
        return None;
    }
    let n = u32_at(d, 0x20)? as usize;
    if !(1..=64).contains(&n) {
        return None;
    }
    [0x40usize, 0x100].into_iter().find(|&stride| {
        let mut used = 0;
        for k in 0..n {
            let r = FIRST + k * stride;
            let (Some(hp), Some(max), Some(mp), Some(mmp)) = (u32_at(d, r + 0x10), u32_at(d, r + 0x14), u32_at(d, r + 0x18), u32_at(d, r + 0x1C)) else {
                return false;
            };
            if max == 0 {
                continue; // not joined yet
            }
            // Current values above the maximum are allowed (an edit, or a temporary boost).
            if !(1..=99).contains(&d[r]) || max > 99_999 || hp > 99_999 || mp > 99_999 || mmp > 99_999 {
                return false;
            }
            used += 1;
        }
        used > 0
    }).map(|s| (s, n))
}

// ------------------------------------------------------------------ inventory

/// Where the inventory table is and how its records are laid out.
#[derive(Clone, Copy, PartialEq)]
enum Table {
    /// 0x18-byte records: timestamp (u64), id (i32, -1 = empty), quantity, category, 0.
    Remake,
    /// 0x20-byte records: u16, type (u8, 0xFF = empty), u8, running number from 100000,
    /// id (i32, -1 = empty), quantity, acquisition order, then zeros.
    Rebirth,
}

const SLOTS: usize = 2048;
const REMAKE_BASE: usize = 0x34DD0;
const REBIRTH_BASE: usize = 0x4FA68;
/// Remake: each character's equipment (weapon, armor, accessory) as inventory slot numbers.
const REMAKE_EQUIP: usize = 0x1740;

impl Table {
    fn of(d: &[u8], rebirth: bool) -> Option<Table> {
        let t = if rebirth { Table::Rebirth } else { Table::Remake };
        let ok = match t {
            Table::Remake => d.len() >= REMAKE_BASE + SLOTS * 0x18 && (0..SLOTS).all(|k| u32_at(d, REMAKE_BASE + k * 0x18 + 0x14) == Some(0)),
            Table::Rebirth => (0..SLOTS).all(|k| u32_at(d, REBIRTH_BASE + k * 0x20 + 4) == Some(100_000 + k as u32)),
        };
        ok.then_some(t)
    }
    fn rec(self, k: usize) -> usize {
        match self {
            Table::Remake => REMAKE_BASE + k * 0x18,
            Table::Rebirth => REBIRTH_BASE + k * 0x20,
        }
    }
    fn id_at(self, k: usize) -> usize {
        self.rec(k) + 8
    }
    fn id(self, d: &[u8], k: usize) -> i32 {
        u32_at(d, self.id_at(k)).map_or(-1, |v| v as i32)
    }
    /// Category (Remake) or type (Rebirth) of slot `k`.
    fn kind(self, d: &[u8], k: usize) -> u32 {
        match self {
            Table::Remake => u32_at(d, self.rec(k) + 0x10).unwrap_or(0),
            Table::Rebirth => u32::from(d[self.rec(k) + 2]),
        }
    }
    fn empty(self, d: &[u8], k: usize) -> bool {
        let id = self.id(d, k);
        id == -1 || (self == Table::Remake && id == 0)
    }
    fn names(self) -> &'static [(u32, &'static str, u8)] {
        match self {
            Table::Remake => items::REMAKE,
            Table::Rebirth => items::REBIRTH,
        }
    }
    fn group(self, kind: u32) -> &'static str {
        match (self, kind) {
            (Table::Remake, 0) | (Table::Remake, 1) => "Items",
            (Table::Remake, 2) => "Key items",
            (Table::Remake, 8) | (Table::Rebirth, 4) => "Armor",
            (Table::Remake, 9) | (Table::Rebirth, 5) => "Accessories",
            (Table::Remake, 10) => "Music",
            (Table::Rebirth, 0) => "Items",
            (Table::Rebirth, 1) => "Materials",
            (Table::Rebirth, 2) => "Key items",
            (Table::Rebirth, 8) => "Currencies",
            (Table::Rebirth, 9) => "Outfits",
            (Table::Rebirth, 10) => "Transmuter chips",
            (Table::Rebirth, 11) => "Folios and manuscripts",
            (Table::Rebirth, 12) => "Chocobo gear",
            (Table::Rebirth, 13) => "Queen's Blood cards",
            (Table::Rebirth, 14) => "Treasure Trove",
            _ => "Other",
        }
    }
    /// Kinds whose amount is a stack (up to 99) rather than one piece.
    fn stacks(self, kind: u32) -> bool {
        match self {
            Table::Remake => kind == 0,
            Table::Rebirth => kind <= 1,
        }
    }
    /// Kinds that can be added or removed: no key items, story items, currencies or trophies.
    fn free(self, kind: u32) -> bool {
        match self {
            Table::Remake => matches!(kind, 0 | 8 | 9 | 10),
            Table::Rebirth => matches!(kind, 0 | 1 | 12 | 13),
        }
    }
    /// Kinds whose amount can be edited (Rebirth's armor and accessories too, but not to 0:
    /// characters wear them by slot and the slot must stay).
    fn countable(self, kind: u32) -> bool {
        self.free(kind) || (self == Table::Rebirth && matches!(kind, 4 | 5))
    }
}

/// The quantity offset of the first record with this id (and Remake category).
fn find_item(d: &[u8], t: Table, id: i32, kind: Option<u32>) -> Option<usize> {
    (0..SLOTS).find(|&k| t.id(d, k) == id && kind.is_none_or(|c| t.kind(d, k) == c)).map(|k| t.id_at(k) + 4)
}

/// Remake: how many characters wear the armor or accessory in slot `k`.
fn worn(d: &[u8], k: usize) -> i64 {
    (0..8).map(|c| [4, 8].iter().filter(|&&o| u32_at(d, REMAKE_EQUIP + c * 0x20 + o).map(|v| v as i32) == Some(k as i32)).count() as i64).sum()
}

fn add_choices(stack: bool) -> Vec<String> {
    let v: &[&str] = if stack { &["Don't add", "Add 1", "Add 10", "Add 99"] } else { &["Don't add", "Add"] };
    v.iter().map(|s| s.to_string()).collect()
}

fn inventory(b: &mut Builder, d: &[u8], t: Table) {
    let names: std::collections::HashMap<u32, &str> = t.names().iter().map(|&(i, n, _)| (i, n)).collect();
    let mut owned = std::collections::HashSet::new();
    let mut used = std::collections::HashSet::new();
    for k in 0..SLOTS {
        if t.empty(d, k) {
            continue;
        }
        let (id, kind) = (t.id(d, k), t.kind(d, k));
        owned.insert(id as u32);
        // Currencies are offered by name at the top.
        if (t == Table::Remake && id == 20) || (t == Table::Rebirth && kind == 8) {
            continue;
        }
        let group = t.group(kind);
        let base = names.get(&(id as u32)).map_or_else(|| format!("{group} {id}"), |n| n.to_string());
        // A second entry of the same item is named by its slot, which stays the same when others change.
        let name = if used.insert((group, base.clone())) { base } else { format!("{base} (slot {})", k + 1) };
        let qty = t.id_at(k) + 4;
        let path = ["Inventory", group, name.as_str()];
        if !t.countable(kind) {
            b.info(&path, u32_at(d, qty).unwrap_or(0));
            continue;
        }
        let wear = if t == Table::Remake && matches!(kind, 8 | 9) { worn(d, k) } else { 0 };
        let lo = if t.free(kind) { wear } else { 1 };
        let have = i64::from(u32_at(d, qty).unwrap_or(0));
        // An amount above 99 (from another editor) stays allowed.
        b.ranged(&path, qty, Kind::U32, lo.min(have), 99.max(have));
        if t.free(kind) && wear == 0 {
            b.option(&["Remove items", group, &name], vec!["Keep".into(), "Remove".into()], format!("{}R{k}", tag(t)));
        }
    }
    for &(id, name, kind) in t.names() {
        let kind = u32::from(kind);
        if owned.contains(&id) || !t.free(kind) {
            continue;
        }
        b.option(&["Add items", t.group(kind), name], add_choices(t.stacks(kind)), format!("{}A{id}:{kind}", tag(t)));
    }
}

fn tag(t: Table) -> char {
    if t == Table::Rebirth { 'B' } else { 'M' }
}

/// Saves the items chosen under "Add items" (into free slots) and "Remove items", and the
/// materia chosen to add or remove.
fn post(out: &mut Vec<u8>, chosen: &[(std::sync::Arc<str>, String)]) -> Result<(), String> {
    post_materia(out, chosen)?;
    let put = |out: &mut Vec<u8>, at: usize, v: u32| out[at..at + 4].copy_from_slice(&v.to_le_bytes());
    for (tag, choice) in chosen {
        let t = if tag.starts_with('B') { Table::Rebirth } else { Table::Remake };
        let rest = &tag[1..];
        if let Some(k) = rest.strip_prefix('R') {
            if choice != "Remove" {
                continue;
            }
            let k: usize = k.parse().map_err(|_| "bad request")?;
            let r = t.rec(k);
            match t {
                Table::Remake => {
                    out[r..r + 0x18].fill(0);
                    put(out, r + 8, u32::MAX);
                }
                Table::Rebirth => {
                    out[r + 2] = 0xFF;
                    put(out, r + 8, u32::MAX);
                    out[r + 0xC..r + 0x20].fill(0);
                }
            }
        }
    }
    for (tag, choice) in chosen {
        let t = if tag.starts_with('B') { Table::Rebirth } else { Table::Remake };
        let Some(rest) = tag[1..].strip_prefix('A') else { continue };
        let qty: u32 = match choice.as_str() {
            "Add" | "Add 1" => 1,
            "Add 10" => 10,
            "Add 99" => 99,
            _ => continue,
        };
        let (id, kind) = rest.split_once(':').ok_or("bad request")?;
        let id: u32 = id.parse().map_err(|_| "bad request")?;
        let kind: u32 = kind.parse().map_err(|_| "bad request")?;
        if (0..SLOTS).any(|k| !t.empty(out, k) && t.id(out, k) == id as i32) {
            continue;
        }
        let k = (0..SLOTS).find(|&k| t.empty(out, k)).ok_or("the inventory has no free slot left; remove something first")?;
        let r = t.rec(k);
        match t {
            Table::Remake => {
                // The game stamps each new entry with the time; one after the newest keeps the order.
                let newest = (0..SLOTS).filter_map(|j| u32_at(out, t.rec(j))).max().unwrap_or(0);
                out[r..r + 0x18].fill(0);
                put(out, r, newest + 1);
                put(out, r + 8, id);
                put(out, r + 0xC, qty);
                put(out, r + 0x10, kind);
            }
            Table::Rebirth => {
                let order = (0..SLOTS).filter_map(|j| u32_at(out, t.rec(j) + 0x10)).max().unwrap_or(0);
                out[r + 2] = kind as u8;
                put(out, r + 8, id);
                put(out, r + 0xC, qty);
                put(out, r + 0x10, order + 1);
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ materia

/// Remake: 1000 materia records of 0x20 bytes: level, wearer (9 = nobody), owned flag, 0,
/// id, AP, 0, timestamp, 0, slot number + 1, 0. Characters, weapons and summon slots point at
/// a record by its slot number (tables below).
const REMAKE_MATERIA: usize = 0x1840;
const REMAKE_CHAR_MATERIA: usize = 0x40DD0; // 8 × 0x30, slots at +0x10
const REMAKE_WEAPON_MATERIA: usize = 0x40F50; // 0x78 × 0x30, slots at +0x10
const REMAKE_SUMMONS: usize = 0x44ED4; // 8 slot numbers
/// Rebirth: 1000 inventory records like the item table's (type 6) and, in the same order,
/// 1000 detail records: instance number, id, AP, 0, level, wearer (16 = nobody).
const REBIRTH_MATERIA_INV: usize = 0x60A68;
const REBIRTH_MATERIA: usize = 0xBFA68;
const MATERIA_SLOTS: usize = 1000;

fn materia_ok(d: &[u8], rebirth: bool) -> bool {
    if rebirth {
        (0..MATERIA_SLOTS).all(|k| u32_at(d, REBIRTH_MATERIA_INV + k * 0x20 + 4) == Some(0x927C0 + k as u32))
    } else {
        d.len() >= REMAKE_SUMMONS + 32
            && (0..MATERIA_SLOTS).all(|k| {
                let r = REMAKE_MATERIA + k * 0x20;
                let idx = u32_at(d, r + 0x18).unwrap_or(u32::MAX);
                idx == 0 || idx == k as u32 + 1
            })
    }
}

/// Remake: materia slot numbers someone or some weapon holds.
fn remake_slotted(d: &[u8]) -> std::collections::HashSet<i32> {
    let mut held = std::collections::HashSet::new();
    let mut take = |at: usize, n: usize| {
        for j in 0..n {
            if let Some(v) = u32_at(d, at + 4 * j) {
                held.insert(v as i32);
            }
        }
    };
    for c in 0..8 {
        take(REMAKE_CHAR_MATERIA + c * 0x30 + 0x10, 8);
    }
    for w in 0..0x78 {
        take(REMAKE_WEAPON_MATERIA + w * 0x30 + 0x10, 8);
    }
    take(REMAKE_SUMMONS, 8);
    held.remove(&-1);
    held
}

fn materia(b: &mut Builder, d: &[u8], rebirth: bool) {
    let held = if rebirth { Default::default() } else { remake_slotted(d) };
    let mut owned = std::collections::HashSet::new();
    for k in 0..MATERIA_SLOTS {
        let (id, level, free, ap_at, lv_at) = if rebirth {
            let r = REBIRTH_MATERIA + k * 0x20;
            let id = u32_at(d, r + 4).map_or(-1, |v| v as i32);
            (id, d[r + 0x10], d[r + 0x11] == 16, r + 8, r + 0x10)
        } else {
            let r = REMAKE_MATERIA + k * 0x20;
            let id = u32_at(d, r + 4).map_or(0, |v| v as i32);
            let id = if d[r + 2] == 0 { 0 } else { id };
            (id, d[r], d[r + 1] == 9 && !held.contains(&(k as i32)), r + 8, r)
        };
        if id <= 0 {
            continue;
        }
        owned.insert(id as u32);
        let base = if rebirth {
            items::REBIRTH_MATERIA.iter().find(|m| m.0 == id as u32).map(|m| m.1.to_string())
        } else {
            items::REMAKE_MATERIA.iter().find(|m| m.0 == id as u32).map(|m| m.1.to_string())
        }
        .unwrap_or_else(|| format!("Materia {id}"));
        // Keyed by slot so a name always means the same record (copies are common).
        let name = format!("{base} #{}", k + 1);
        let ap_table = if rebirth { items::REBIRTH_MATERIA.iter().find(|m| m.0 == id as u32).map(|m| m.3) } else { None };
        match ap_table {
            Some(ap) if !ap.is_empty() => {
                let top = ap.len() as i64;
                b.ranged(&["Materia", &name, "Level"], lv_at, Kind::U8, 1, top.max(i64::from(level)));
                let max_ap = i64::from(*ap.last().unwrap_or(&0));
                let have = i64::from(u32_at(d, ap_at).unwrap_or(0));
                b.ranged(&["Materia", &name, "AP"], ap_at, Kind::U32, 0, max_ap.max(have));
            }
            _ => {
                b.info(&["Materia", &name, "Level"], level);
                b.info(&["Materia", &name, "AP"], u32_at(d, ap_at).unwrap_or(0));
            }
        }
        b.info(&["Materia", &name, "Equipped"], if free { "No" } else { "Yes" });
        if free {
            b.option(&["Remove materia", &name], vec!["Keep".into(), "Remove".into()], format!("{}r{k}", if rebirth { 'B' } else { 'M' }));
        }
    }
    let list: Vec<(u32, &str, u8)> =
        if rebirth { items::REBIRTH_MATERIA.iter().map(|m| (m.0, m.1, m.2)).collect() } else { items::REMAKE_MATERIA.to_vec() };
    for (id, name, kind) in list {
        // One of each summon.
        let summon = kind == 4;
        if summon && owned.contains(&id) {
            continue;
        }
        let choices: Vec<String> = if summon { vec!["Don't add".into(), "Add".into()] } else { ["Don't add", "Add 1", "Add 2", "Add 3", "Add 5"].iter().map(|s| s.to_string()).collect() };
        b.option(&["Add materia", items::MATERIA_KINDS[usize::from(kind).min(4)], name], choices, format!("{}m{id}", if rebirth { 'B' } else { 'M' }));
    }
}

/// Saves the materia chosen under "Remove materia" and "Add materia".
fn post_materia(out: &mut [u8], chosen: &[(std::sync::Arc<str>, String)]) -> Result<(), String> {
    let put = |out: &mut [u8], at: usize, v: u32| out[at..at + 4].copy_from_slice(&v.to_le_bytes());
    for (tag, choice) in chosen {
        let rebirth = tag.starts_with('B');
        let Some(k) = tag[1..].strip_prefix('r') else { continue };
        if choice != "Remove" {
            continue;
        }
        let k: usize = k.parse().map_err(|_| "bad request")?;
        if rebirth {
            let a = REBIRTH_MATERIA_INV + k * 0x20;
            out[a + 2] = 0xFF;
            put(out, a + 8, u32::MAX);
            out[a + 0xC..a + 0x20].fill(0);
            let r = REBIRTH_MATERIA + k * 0x20;
            out[r..r + 0x20].fill(0);
            put(out, r, u32::MAX);
            put(out, r + 4, u32::MAX);
            out[r + 0x10] = 1;
            out[r + 0x11] = 16;
        } else {
            let r = REMAKE_MATERIA + k * 0x20;
            out[r..r + 0x20].fill(0);
            out[r + 1] = 9;
        }
    }
    for (tag, choice) in chosen {
        let rebirth = tag.starts_with('B');
        let Some(id) = tag[1..].strip_prefix('m') else { continue };
        let n: usize = match choice.as_str() {
            "Add" | "Add 1" => 1,
            "Add 2" => 2,
            "Add 3" => 3,
            "Add 5" => 5,
            _ => continue,
        };
        let id: u32 = id.parse().map_err(|_| "bad request")?;
        for _ in 0..n {
            if rebirth {
                let k = (0..MATERIA_SLOTS).find(|&k| u32_at(out, REBIRTH_MATERIA_INV + k * 0x20 + 8) == Some(u32::MAX)).ok_or("no free materia slot left; remove some materia first")?;
                let order = (0..MATERIA_SLOTS).filter_map(|j| u32_at(out, REBIRTH_MATERIA_INV + j * 0x20 + 0x10)).max().unwrap_or(0);
                let a = REBIRTH_MATERIA_INV + k * 0x20;
                out[a + 2] = 6;
                put(out, a + 8, id);
                put(out, a + 0xC, 1);
                put(out, a + 0x10, order + 1);
                let r = REBIRTH_MATERIA + k * 0x20;
                out[r..r + 0x20].fill(0);
                put(out, r, 0x927C0 + k as u32);
                put(out, r + 4, id);
                out[r + 0x10] = 1;
                out[r + 0x11] = 16;
            } else {
                let k = (0..MATERIA_SLOTS).find(|&k| out[REMAKE_MATERIA + k * 0x20 + 2] == 0 && u32_at(out, REMAKE_MATERIA + k * 0x20 + 4) == Some(0)).ok_or("no free materia slot left; remove some materia first")?;
                let newest = (0..MATERIA_SLOTS).filter_map(|j| u32_at(out, REMAKE_MATERIA + j * 0x20 + 0x10)).max().unwrap_or(0);
                let r = REMAKE_MATERIA + k * 0x20;
                out[r..r + 0x20].fill(0);
                out[r] = 1;
                out[r + 1] = 9;
                out[r + 2] = 1;
                put(out, r + 4, id);
                put(out, r + 0x10, newest + 1);
                put(out, r + 0x18, k as u32 + 1);
            }
        }
    }
    Ok(())
}

pub fn looks(d: &[u8]) -> bool {
    layout(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (stride, n) = layout(d).ok_or("not a Final Fantasy VII Remake/Rebirth slot")?;
    let rebirth = stride == 0x100;
    let mut b = Builder::new(d);
    let game = if rebirth { "Final Fantasy VII Rebirth" } else { "Final Fantasy VII Remake" };
    b.info(&["Game"], game);
    let cap = if rebirth { 70 } else { 50 };
    let table = Table::of(d, rebirth);
    if let Some(t) = table {
        let money: &[(&str, i32, Option<u32>, i64)] = if rebirth {
            &[("Gil", 1, None, 9_999_999), ("Moogle Medals", 2, None, 9_999_999), ("Gold Saucer Points", 11, None, 9_999_999)]
        } else {
            &[("Gil", 20, Some(1), 99_999_999)]
        };
        for &(name, id, kind, max) in money {
            if let Some(at) = find_item(d, t, id, kind) {
                b.ranged(&[name], at, Kind::U32, 0, max);
            }
        }
    }
    for k in 0..n {
        let r = FIRST + k * stride;
        if u32_at(d, r + 0x14).unwrap_or(0) == 0 {
            continue;
        }
        let who = format!("Character {}", k + 1);
        let w = who.as_str();
        // Levels above the game's cap are kept as they are (they can come from other editors).
        b.ranged(&["Party", w, "Level"], r, Kind::U8, 1, i64::from(d[r].max(cap)));
        b.ranged(&["Party", w, "HP"], r + 0x10, Kind::U32, 0, 99_999);
        b.ranged(&["Party", w, "Max HP"], r + 0x14, Kind::U32, 1, 99_999);
        b.ranged(&["Party", w, "MP"], r + 0x18, Kind::U32, 0, 99_999);
        b.ranged(&["Party", w, "Max MP"], r + 0x1C, Kind::U32, 0, 99_999);
        b.ranged(&["Party", w, "Experience"], r + 0x20, Kind::U32, 0, 99_999_999);
    }
    if let Some(t) = table {
        inventory(&mut b, d, t);
    }
    if materia_ok(d, rebirth) {
        materia(&mut b, d, rebirth);
    }
    Ok(b.finish_with(&format!("{game} save"), Checksum::None, Some(post)).with_note(
        "Gil, the inventory and materia (add, remove and change amounts), party levels, HP, MP and experience. The game \
         may recalculate max HP and MP from level and equipment when it loads. Weapons aren't listed.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_slot() {
        let mut d = vec![0u8; 0x400];
        d[..4].copy_from_slice(&[0, 0, 1, 0]);
        d[0x10..0x20].copy_from_slice(MARK);
        d[0x20] = 2;
        for (k, (lv, hp, max)) in [(40u8, 900u32, 1000u32), (12, 300, 300)].into_iter().enumerate() {
            let r = FIRST + k * 0x40;
            d[r] = lv;
            d[r + 0x10..r + 0x14].copy_from_slice(&hp.to_le_bytes());
            d[r + 0x14..r + 0x18].copy_from_slice(&max.to_le_bytes());
        }
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Game").unwrap(), "Final Fantasy VII Remake");
        assert_eq!(doc.view.pointer("/Party/Character 1/HP").unwrap(), 900);
        assert!(doc.set("/Party/Character 1/Level", serde_json::Value::from(51)).is_err());
        doc.set("/Party/Character 2/Experience", serde_json::Value::from(12345)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, FIRST + 0x40 + 0x20), Some(12345));
    }

    fn remake_slot() -> Vec<u8> {
        let mut d = vec![0u8; 0x41000];
        d[..4].copy_from_slice(&[0, 0, 1, 0]);
        d[0x10..0x20].copy_from_slice(MARK);
        d[0x20] = 1;
        d[FIRST] = 10;
        d[FIRST + 0x14] = 100;
        for k in 0..SLOTS {
            let r = REMAKE_BASE + k * 0x18;
            d[r + 8..r + 12].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        // Nobody wears anything.
        for c in 0..8 {
            d[REMAKE_EQUIP + c * 0x20..REMAKE_EQUIP + c * 0x20 + 12].fill(0xFF);
        }
        d
    }

    fn put_item(d: &mut [u8], t: Table, k: usize, id: u32, qty: u32, kind: u32) {
        let r = t.rec(k);
        d[r + 8..r + 12].copy_from_slice(&id.to_le_bytes());
        d[r + 12..r + 16].copy_from_slice(&qty.to_le_bytes());
        match t {
            Table::Remake => d[r + 0x10..r + 0x14].copy_from_slice(&kind.to_le_bytes()),
            Table::Rebirth => d[r + 2] = kind as u8,
        }
    }

    #[test]
    fn remake_inventory() {
        let t = Table::Remake;
        let mut d = remake_slot();
        put_item(&mut d, t, 0, 9001, 3, 8); // Bronze Bangle, worn by the first character
        put_item(&mut d, t, 3, 20, 1234, 1); // gil
        put_item(&mut d, t, 4, 1, 5, 0); // Potion
        put_item(&mut d, t, 5, 100, 1, 2); // a key item
        d[REMAKE_EQUIP + 4..REMAKE_EQUIP + 8].copy_from_slice(&0u32.to_le_bytes());
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Gil").unwrap(), 1234);
        assert_eq!(doc.view.pointer("/Inventory/Items/Potion").unwrap(), 5);
        assert!(doc.view.pointer("/Inventory/Key items").is_some());
        // Worn armor can't drop below what's worn or be removed.
        assert!(doc.set("/Inventory/Armor/Bronze Bangle", serde_json::Value::from(0)).is_err());
        assert!(doc.view.pointer("/Remove items/Armor").is_none());
        doc.set("/Gil", serde_json::Value::from(99_999_999)).unwrap();
        assert!(doc.set("/Gil", serde_json::Value::from(100_000_000)).is_err());
        doc.set("/Remove items/Items/Potion", serde_json::Value::from("Remove")).unwrap();
        doc.set("/Add items/Items/Elixir", serde_json::Value::from("Add 99")).unwrap();
        doc.set("/Add items/Accessories/Talisman", serde_json::Value::from("Add")).unwrap();
        assert!(doc.view.pointer("/Add items/Key items").is_none());
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, t.id_at(3) + 4), Some(99_999_999));
        let doc = parse(&out).unwrap();
        assert!(doc.view.pointer("/Inventory/Items/Potion").is_none());
        assert_eq!(doc.view.pointer("/Inventory/Items/Elixir").unwrap(), 99);
        assert_eq!(doc.view.pointer("/Inventory/Accessories/Talisman").unwrap(), 1);
        let k = (0..SLOTS).find(|&k| t.id(&out, k) == 9020).unwrap();
        assert_eq!(t.kind(&out, k), 9);
    }

    #[test]
    fn remake_materia() {
        let mut d = remake_slot();
        d.resize(0x45000, 0);
        for k in 0..MATERIA_SLOTS {
            d[REMAKE_MATERIA + k * 0x20 + 1] = 9;
        }
        for (at, n) in [(REMAKE_CHAR_MATERIA, 8), (REMAKE_WEAPON_MATERIA, 0x78)] {
            for j in 0..n {
                d[at + j * 0x30 + 0x10..at + j * 0x30 + 0x30].fill(0xFF);
            }
        }
        d[REMAKE_SUMMONS..REMAKE_SUMMONS + 32].fill(0xFF);
        // Fire in slot 0 (held by a weapon), Healing in slot 1 (free).
        for (k, id) in [(0usize, 10004u32), (1, 10001)] {
            let r = REMAKE_MATERIA + k * 0x20;
            d[r] = 2;
            d[r + 2] = 1;
            d[r + 4..r + 8].copy_from_slice(&id.to_le_bytes());
            d[r + 0x18..r + 0x1C].copy_from_slice(&(k as u32 + 1).to_le_bytes());
        }
        d[REMAKE_WEAPON_MATERIA + 0x10..REMAKE_WEAPON_MATERIA + 0x14].copy_from_slice(&0u32.to_le_bytes());
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Materia/Fire #1/Equipped").unwrap(), "Yes");
        assert!(doc.view.pointer("/Remove materia/Fire #1").is_none());
        doc.set("/Remove materia/Healing #2", serde_json::Value::from("Remove")).unwrap();
        doc.set("/Add materia/Summon/Shiva", serde_json::Value::from("Add")).unwrap();
        doc.set("/Add materia/Magic/Ice", serde_json::Value::from("Add 2")).unwrap();
        let doc = parse(&doc.to_bytes().unwrap()).unwrap();
        let mut names: Vec<String> = doc.view["Materia"].as_object().unwrap().keys().map(|k| k.split(" #").next().unwrap().to_string()).collect();
        names.sort();
        assert_eq!(names, ["Fire", "Ice", "Ice", "Shiva"]);
        assert!(doc.view.pointer("/Add materia/Summon/Shiva").is_none());
    }

    #[test]
    fn rebirth_inventory() {
        let t = Table::Rebirth;
        let mut d = vec![0u8; 0x60000];
        d[..4].copy_from_slice(&[0, 0, 1, 0]);
        d[0x10..0x20].copy_from_slice(MARK);
        d[0x20] = 2;
        d[FIRST] = 10;
        d[FIRST + 0x14] = 100;
        d[FIRST + 0x54..FIRST + 0x58].copy_from_slice(&[0xFF; 4]); // rules out 0x40 records
        for k in 0..SLOTS {
            let r = REBIRTH_BASE + k * 0x20;
            d[r + 2] = 0xFF;
            d[r + 4..r + 8].copy_from_slice(&(100_000 + k as u32).to_le_bytes());
            d[r + 8..r + 12].copy_from_slice(&u32::MAX.to_le_bytes());
        }
        put_item(&mut d, t, 4, 1, 500, 8);
        put_item(&mut d, t, 5, 300, 7, 1); // Sage, a material
        put_item(&mut d, t, 6, 2001, 1, 4); // Leather Bangle
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Game").unwrap(), "Final Fantasy VII Rebirth");
        assert_eq!(doc.view.pointer("/Gil").unwrap(), 500);
        assert!(doc.view.pointer("/Moogle Medals").is_none());
        assert_eq!(doc.view.pointer("/Inventory/Materials/Sage").unwrap(), 7);
        assert!(doc.set("/Inventory/Armor/Leather Bangle", serde_json::Value::from(0)).is_err());
        doc.set("/Inventory/Armor/Leather Bangle", serde_json::Value::from(2)).unwrap();
        doc.set("/Gil", serde_json::Value::from(9_999_999)).unwrap();
        doc.set("/Add items/Items/Potion", serde_json::Value::from("Add 10")).unwrap();
        doc.set("/Remove items/Materials/Sage", serde_json::Value::from("Remove")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, t.id_at(4) + 4), Some(9_999_999));
        // Removed: the slot is empty again and keeps its running number.
        assert!(t.empty(&out, 5));
        assert_eq!(out[t.rec(5) + 2], 0xFF);
        assert_eq!(u32_at(&out, t.rec(5) + 4), Some(100_005));
        let doc = parse(&out).unwrap();
        assert_eq!(doc.view.pointer("/Inventory/Items/Potion").unwrap(), 10);
        assert_eq!(doc.view.pointer("/Inventory/Armor/Leather Bangle").unwrap(), 2);
        assert!(doc.view.pointer("/Inventory/Materials").is_none());
    }

    /// Research aid: SAVE_EDITOR_FF7R_DIR = a folder of Remake/Rebirth saves (read only).
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("SAVE_EDITOR_FF7R_DIR").unwrap();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            match parse(&d) {
                Ok(doc) => {
                    assert_eq!(doc.to_bytes().unwrap(), d);
                    let lv: Vec<String> = doc.view["Party"].as_object().unwrap().values().map(|c| c["Level"].to_string()).collect();
                    let inv: Vec<String> = doc.view["Inventory"].as_object().map(|m| m.iter().map(|(k, v)| format!("{k} {}", v.as_object().map_or(0, |o| o.len()))).collect()).unwrap_or_default();
                    let add = doc.view["Add items"].as_object().map_or(0, |m| m.values().map(|v| v.as_object().map_or(0, |o| o.len())).sum::<usize>());
                    println!("{:?}: {} levels {:?} gil {} inventory {inv:?} addable {add}", e.file_name(), doc.kind_name, lv, doc.view["Gil"]);
                }
                Err(err) => println!("{:?}: {err}", e.file_name()),
            }
        }
    }
}
