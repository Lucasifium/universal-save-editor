//! Dark Souls II: Scholar of the First Sin's item lists, at fixed places in the character
//! slot: everything owned (carried, and in Bonfire storage, marked by a flag) and the key
//! items. An entry is 16 bytes: item id, flags (0x100: in the item box), quantity (for
//! weapons, armor and rings the durability, as a float), and the upgrade level (low byte;
//! a weapon's infusion in the next). An unused entry has id 0. Equipment is referred to by
//! its position in the list (a table of 16-bit positions), so entries never move here.
//! Checked against a sample save, alfizari's Dark-Souls-2-Save-Editor and er-save-manager's
//! notes on the format.

use super::ds2_items::{ARMOR, GOODS, KEY_ITEMS, RINGS, WEAPONS};
use super::er_inv::{Names, NO_AFFINITY};
use super::sekiro_inv::EMPTY;
use crate::tree::int_in;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

const ENTRY: usize = 16;
/// (start, end) of the main list and the key items (after the slot's 4-byte length).
const MAIN: (usize, usize) = (0x1E30, 0x10E20);
const KEY: (usize, usize) = (0x10E34, 0x11DF4);
/// The positions equipment uses (u16 each, 0xFFFF for none): weapons, armor, rings,
/// ammunition, belt items and spells.
const EQUIP_INDEX: usize = 0x11E34;
const EQUIP_INDEX_COUNT: usize = 42;
const IN_BOX: u32 = 0x100;
const MAX_CARRIED: i128 = 99;
const MAX_BOXED: i128 = 999;
pub const INFUSIONS: [&str; 10] = [
    "Normal",
    "Fire",
    "Magic",
    "Lightning",
    "Dark",
    "Poison",
    "Bleed",
    "Raw",
    "Enchanted",
    "Mundane",
];
/// Durability for new equipment when the save has nothing to copy it from (low enough to
/// stay under any item's maximum; the game repairs it at a bonfire).
const DEFAULT_DURABILITY: [f32; 3] = [30.0, 35.0, 70.0];

fn u32_at(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4)
        .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn put(d: &mut [u8], o: usize, v: u32) {
    d[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Good,
    Weapon,
    Armor,
    Ring,
    Key,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Good => "Item",
            Kind::Weapon => "Weapon",
            Kind::Armor => "Armor",
            Kind::Ring => "Ring",
            Kind::Key => "Key item",
        }
    }

    fn equipment(self) -> bool {
        matches!(self, Kind::Weapon | Kind::Armor | Kind::Ring)
    }
}

fn names(kind: Kind) -> &'static Names {
    static N: OnceLock<[Names; 5]> = OnceLock::new();
    &N.get_or_init(|| {
        [GOODS, WEAPONS, ARMOR, RINGS, KEY_ITEMS]
            .map(|t| Names::new(t.iter().map(|&(i, n)| (i, n))))
    })[kind as usize]
}

/// An item's kind by id (the save doesn't say; ids of different kinds overlap in range).
fn kind_of(id: u32) -> Option<Kind> {
    static K: OnceLock<HashMap<u32, Kind>> = OnceLock::new();
    K.get_or_init(|| {
        let mut m = HashMap::new();
        for (t, k) in [
            (GOODS, Kind::Good),
            (KEY_ITEMS, Kind::Key),
            (RINGS, Kind::Ring),
            (ARMOR, Kind::Armor),
            (WEAPONS, Kind::Weapon),
        ] {
            for &(i, _) in t {
                m.insert(i, k);
            }
        }
        m
    })
    .get(&id)
    .copied()
}

#[derive(Clone, Debug)]
struct Row {
    kind: Kind,
    off: usize,
    ptrs: [Option<String>; 4],
    id: Option<u32>,
    orig_name: String,
    /// The level it had (the highest it may go when that's above +5).
    level: u32,
    boxed: bool,
    equipped: bool,
    offered: bool,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    rows: Vec<Row>,
    first_leaf: usize,
    /// Durability to give new equipment, by id (an owned copy's) and by kind (the lowest owned).
    durability: HashMap<u32, u32>,
    durability_kind: [u32; 3],
}

/// Unused entries offered for new items, as (kind, first position, count): far enough into
/// the list that the game can still put items it moves to the box after them.
const OFFERED: [(Kind, usize, usize); 4] = [
    (Kind::Ring, 1904, 16),
    (Kind::Armor, 1920, 32),
    (Kind::Weapon, 1952, 32),
    (Kind::Good, 1984, 64),
];
const OFFERED_KEYS: usize = 16;

fn offered_kind(i: usize) -> Option<Kind> {
    OFFERED
        .iter()
        .find(|(_, s, n)| (*s..s + n).contains(&i))
        .map(|(k, _, _)| *k)
}

pub fn build(
    d: &[u8],
    first_leaf: usize,
    root: &mut Map<String, Value>,
    editable: &mut HashMap<String, usize>,
) -> Option<Inventory> {
    if d.len() < EQUIP_INDEX + EQUIP_INDEX_COUNT * 2 {
        return None;
    }
    let equipped: BTreeSet<usize> = (0..EQUIP_INDEX_COUNT)
        .map(|i| u16::from_le_bytes([d[EQUIP_INDEX + i * 2], d[EQUIP_INDEX + i * 2 + 1]]))
        .filter(|&p| p != 0xFFFF)
        .map(|p| p as usize)
        .collect();
    let mut inv = Inventory {
        rows: vec![],
        first_leaf,
        durability: HashMap::new(),
        durability_kind: DEFAULT_DURABILITY.map(f32::to_bits),
    };
    let mut lowest: [Option<f32>; 3] = [None; 3];
    let mut sections: Vec<(&str, Map<String, Value>)> = [
        "Items",
        "Item box",
        "Weapons",
        "Armor",
        "Rings",
        "Key items",
    ]
    .iter()
    .map(|s| (*s, Map::new()))
    .collect();
    let n_main = (MAIN.1 - MAIN.0) / ENTRY;
    let n_key = (KEY.1 - KEY.0) / ENTRY;
    for (list, (start, n)) in [(MAIN.0, n_main), (KEY.0, n_key)].into_iter().enumerate() {
        for i in 0..n {
            let off = start + i * ENTRY;
            let (id, flags, qty, extra) = (
                u32_at(d, off),
                u32_at(d, off + 4),
                u32_at(d, off + 8),
                u32_at(d, off + 12),
            );
            let offered = if list == 0 {
                offered_kind(i)
            } else {
                (i >= n - OFFERED_KEYS).then_some(Kind::Key)
            };
            let kind = match (id, offered) {
                (0, Some(k)) => k,
                (0, None) => continue,
                (id, _) if list == 1 => {
                    kind_of(id).filter(|k| *k == Kind::Key).unwrap_or(Kind::Key)
                }
                (id, _) => match kind_of(id) {
                    Some(Kind::Key) | None => continue, // unknown: left alone
                    Some(k) => k,
                },
            };
            let boxed = flags & IN_BOX != 0;
            if id != 0 && kind.equipment() {
                let e = kind as usize - 1;
                let f = f32::from_bits(qty);
                inv.durability
                    .entry(id)
                    .and_modify(|q| *q = (*q).max(qty))
                    .or_insert(qty);
                if f.is_finite() && f > 0.0 && lowest[e].is_none_or(|l| f < l) {
                    lowest[e] = Some(f);
                }
            }
            let sec = match kind {
                Kind::Good if boxed => 1,
                Kind::Good => 0,
                Kind::Key => 5,
                k => k as usize + 1,
            };
            let key = format!("Slot {}", i + 1);
            let name = if id == 0 {
                EMPTY.to_string()
            } else {
                names(kind).name(id, kind.word())
            };
            let base = format!("/{}/{key}", sections[sec].0);
            let mut slot = Map::new();
            let mut ptrs: [Option<String>; 4] = Default::default();
            slot.insert("Item".into(), Value::from(name.clone()));
            ptrs[0] = Some(format!("{base}/Item"));
            if matches!(kind, Kind::Good | Kind::Key) {
                slot.insert(
                    "Quantity".into(),
                    Value::from(if id == 0 { 0 } else { qty }),
                );
                ptrs[1] = Some(format!("{base}/Quantity"));
            }
            if kind == Kind::Weapon {
                slot.insert("Level".into(), Value::from(extra & 0xFF));
                let inf = if id == 0 {
                    NO_AFFINITY
                } else {
                    INFUSIONS
                        .get(((extra >> 8) & 0xFF) as usize)
                        .copied()
                        .unwrap_or(NO_AFFINITY)
                };
                slot.insert("Infusion".into(), Value::from(inf));
                ptrs[2] = Some(format!("{base}/Level"));
            }
            if boxed && kind != Kind::Good {
                slot.insert("In the item box".into(), Value::from(true));
            }
            sections[sec].1.insert(key, Value::Object(slot));
            let r = inv.rows.len();
            for (f, p) in ptrs.iter().enumerate() {
                if let Some(p) = p {
                    editable.insert(p.clone(), first_leaf + r * 4 + f);
                }
            }
            inv.rows.push(Row {
                kind,
                off,
                ptrs,
                id: (id != 0).then_some(id),
                orig_name: name,
                level: extra & 0xFF,
                boxed,
                equipped: list == 0 && equipped.contains(&i),
                offered: offered == Some(kind),
            });
        }
    }
    for (e, l) in lowest.iter().enumerate() {
        if let Some(l) = l {
            inv.durability_kind[e] = l.to_bits();
        }
    }
    for (s, m) in sections {
        root.insert(s.into(), Value::Object(m));
    }
    Some(inv)
}

impl Inventory {
    fn row(&self, leaf: usize) -> Option<(&Row, usize)> {
        let i = leaf.checked_sub(self.first_leaf)?;
        self.rows.get(i / 4).map(|r| (r, i % 4))
    }

    pub fn owns(&self, leaf: usize) -> bool {
        self.row(leaf).is_some()
    }

    fn get<'a>(row: &Row, view: &'a Value, f: usize) -> Option<&'a Value> {
        row.ptrs[f].as_ref().and_then(|p| view.pointer(p))
    }

    pub fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let (row, f) = self.row(leaf).ok_or("unknown value")?;
        match f {
            1 => {
                return int_in(
                    v,
                    0,
                    if row.boxed { MAX_BOXED } else { MAX_CARRIED },
                    "Quantity",
                )
                .map(|_| ())
            }
            2 => return int_in(v, 0, 10, "Level").map(|_| ()),
            _ => {}
        }
        let name = v.as_str().ok_or("pick an item from the list")?;
        let what = row.kind.word().to_lowercase();
        if name != row.orig_name && name != EMPTY && !names(row.kind).by_name.contains_key(name) {
            return Err(format!(
                "\u{201C}{name}\u{201D} isn't a Dark Souls II {what}"
            ));
        }
        if row.equipped && name != row.orig_name {
            return Err(format!(
                "this {what} is equipped \u{2014} take it off in the game first"
            ));
        }
        if name == EMPTY && row.id.is_some() && !row.offered {
            return Err(format!(
                "this {what} can't be removed here \u{2014} drop or sell it in the game"
            ));
        }
        Ok(())
    }

    /// Weapons stay within +5, or the level they had if that's higher (unique weapons stop
    /// at +5 and the save doesn't say which those are).
    pub fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Some((row, _)) = self.row(leaf) else {
            return vec![];
        };
        let name = Self::get(row, view, 0)
            .and_then(Value::as_str)
            .unwrap_or(EMPTY);
        let empty = name == EMPTY;
        let mut out = vec![];
        let mut set = |f: usize, v: Value| {
            if let Some(p) = &row.ptrs[f] {
                if view.pointer(p) != Some(&v) {
                    out.push((p.clone(), v));
                }
            }
        };
        let qty = Self::get(row, view, 1).and_then(Value::as_u64).unwrap_or(0);
        set(1, Value::from(if empty { 0 } else { qty.max(1) }));
        if row.kind == Kind::Weapon {
            let max = if name == row.orig_name {
                row.level.max(5)
            } else {
                5
            } as u64;
            let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0);
            set(2, Value::from(if empty { 0 } else { level.min(max) }));
            if row.id.is_none() || name != row.orig_name {
                if let Some(p) = &row.ptrs[0] {
                    let inf = p.replace("/Item", "/Infusion");
                    let v = Value::from(if empty { NO_AFFINITY } else { INFUSIONS[0] });
                    if view.pointer(&inf) != Some(&v) {
                        out.push((inf, v));
                    }
                }
            }
        }
        out
    }

    pub fn choices(&self, leaf: usize, view: &Value) -> Option<Arc<Vec<String>>> {
        let (row, f) = self.row(leaf)?;
        if f != 0 {
            return None;
        }
        let taken: BTreeSet<&str> = if matches!(row.kind, Kind::Good | Kind::Key) {
            self.rows
                .iter()
                .filter(|r| r.kind == row.kind && r.boxed == row.boxed && r.ptrs[0] != row.ptrs[0])
                .filter_map(|r| Self::get(r, view, 0).and_then(Value::as_str))
                .filter(|n| *n != EMPTY)
                .collect()
        } else {
            BTreeSet::new()
        };
        let mut all: Vec<String> = names(row.kind)
            .all
            .iter()
            .filter(|n| !taken.contains(n.as_str()))
            .cloned()
            .collect();
        if row.id.is_some() && !row.offered {
            all.retain(|n| n != EMPTY);
        }
        if row.orig_name != EMPTY && !names(row.kind).by_name.contains_key(&row.orig_name) {
            all.insert(0, row.orig_name.clone());
        }
        Some(Arc::new(all))
    }

    pub fn write(
        &self,
        out: &mut [u8],
        view: &Value,
        changes: &[(usize, Value)],
    ) -> Result<(), String> {
        let touched: BTreeSet<usize> = changes
            .iter()
            .filter_map(|(l, _)| l.checked_sub(self.first_leaf).map(|i| i / 4))
            .filter(|&r| r < self.rows.len())
            .collect();
        let mut added = vec![];
        for r in touched {
            let row = &self.rows[r];
            let name = Self::get(row, view, 0)
                .and_then(Value::as_str)
                .unwrap_or(EMPTY);
            let qty = Self::get(row, view, 1).and_then(Value::as_u64).unwrap_or(1) as u32;
            let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0) as u32;
            let what = row.kind.word().to_lowercase();
            let id = if name == EMPTY {
                None
            } else if name == row.orig_name {
                row.id
            } else {
                Some(*names(row.kind).by_name.get(name).ok_or_else(|| {
                    format!("\u{201C}{name}\u{201D} isn't a Dark Souls II {what}")
                })?)
            };
            let o = row.off;
            match (row.id, id) {
                (None, None) => {}
                (Some(_), None) => out[o..o + ENTRY].fill(0),
                (was, Some(id)) => {
                    let new = was != Some(id);
                    if new && matches!(row.kind, Kind::Good | Kind::Key) {
                        added.push((row.kind, row.boxed, id));
                    }
                    put(out, o, id);
                    if was.is_none() {
                        put(out, o + 4, 0);
                    }
                    match row.kind {
                        Kind::Good | Kind::Key => put(out, o + 8, qty.max(1)),
                        k if new => put(
                            out,
                            o + 8,
                            self.durability
                                .get(&id)
                                .copied()
                                .unwrap_or(self.durability_kind[k as usize - 1]),
                        ),
                        _ => {}
                    }
                    if row.kind == Kind::Weapon {
                        let extra = if new { 0 } else { u32_at(out, o + 12) & !0xFF };
                        put(out, o + 12, extra | (level & 0xFF));
                    } else if new {
                        put(out, o + 12, 0);
                    }
                }
            }
        }
        for (kind, boxed, id) in added {
            let n = self
                .rows
                .iter()
                .filter(|r| r.kind == kind && r.boxed == boxed && u32_at(out, r.off) == id)
                .count();
            if n > 1 {
                return Err(format!(
                    "{} is there already \u{2014} change its quantity instead",
                    names(kind).name(id, kind.word())
                ));
            }
        }
        Ok(())
    }
}
