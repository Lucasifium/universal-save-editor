//! Dark Souls (Remastered)'s item list: 2048 entries of 28 bytes at a fixed place, key
//! items in the first 64. An entry is: kind (top four bits: 0 weapons, shields and
//! ammunition, 1 armor, 2 rings, 4 goods), item id (a weapon's is base + infusion × 100 +
//! level), quantity, order (the game's sort key × 4096 + the entry's position), an in-use
//! flag, durability and a zero. An unused entry is 0xFF with the flag 0 (or all zeros).
//! Two counts follow the equipment (everything else, then key items), and after the list
//! the highest position in use. What's equipped is kept as positions in this list plus a
//! copy of each item's id, which follows the item's id here.
//!
//! New items copy the sort key and durability of the same item if the character has one
//! (else the sort key is 0, which only moves it in the menu, and the durability the lowest
//! of that kind, so it never exceeds the item's maximum). Layout from er-save-manager's
//! notes and alfizari's Dark-Souls-Remastered-Save-Editor (offsets here are 0x10 lower:
//! the first decrypted block isn't part of the data).

use super::ds1_items::{ARMOR, GOODS, RINGS, WEAPONS};
use super::er_inv::{Names, NO_AFFINITY};
use super::sekiro_inv::EMPTY;
use crate::tree::int_in;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

const LIST: usize = 0x360;
const ENTRY: usize = 28;
const COUNT: usize = 2048;
const KEY_SLOTS: usize = 64;
/// The equipped items' positions (20 u32), then the copy of their ids (20 u32).
const EQUIPPED: usize = 0x298;
const EQUIPPED_IDS: usize = 0x304;
const EQUIP_SLOTS: usize = 20;
/// Item counts (everything but key items, then key items) and the highest position in use.
const COMMON_COUNT: usize = 0x354;
const KEY_COUNT: usize = 0x358;
const HIGHEST: usize = 0xE360;
const MAX_QTY: i128 = 99;
const MAX_AMMO: i128 = 999;
/// Durability for new equipment when the character has nothing to copy it from.
const DEFAULT_DURABILITY: u32 = 200;
pub const INFUSIONS: [&str; 10] = ["Normal", "Crystal", "Lightning", "Raw", "Magic", "Enchanted", "Divine", "Occult", "Fire", "Chaos"];

fn u32_at(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
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

    /// The entry's kind bits.
    fn bits(self) -> u32 {
        match self {
            Kind::Weapon => 0,
            Kind::Armor => 1 << 28,
            Kind::Ring => 2 << 28,
            Kind::Good | Kind::Key => 4 << 28,
        }
    }

    fn stacks(self) -> bool {
        matches!(self, Kind::Good | Kind::Key)
    }
}

fn names(kind: Kind) -> &'static Names {
    static N: OnceLock<[Names; 4]> = OnceLock::new();
    let i = match kind {
        Kind::Good | Kind::Key => 0,
        k => k as usize,
    };
    &N.get_or_init(|| [GOODS, WEAPONS, ARMOR, RINGS].map(|t| Names::new(t.iter().map(|&(i, n)| (i, n)))))[i]
}

/// Arrows and bolts: weapons with a quantity, no level.
fn is_ammo(id: u32) -> bool {
    (2_000_000..2_200_000).contains(&id)
}

/// A weapon's base id (ammunition ids are whole items).
fn base_of(id: u32) -> u32 {
    if is_ammo(id) {
        id
    } else {
        id - id % 1000
    }
}

/// The id that names an item: a weapon's base, anything else's own.
fn item_of(kind: Kind, id: u32) -> u32 {
    if kind == Kind::Weapon {
        base_of(id)
    } else {
        id
    }
}

#[derive(Clone, Debug)]
struct Row {
    kind: Kind,
    slot: usize,
    off: usize,
    /// Pointers: item, quantity, level (the last two only where the kind has them).
    ptrs: [Option<String>; 3],
    /// Id when the file was read (a weapon's full id).
    id: Option<u32>,
    orig_name: String,
    /// Copies of this entry's id among the equipped items (offsets).
    equipped_ids: Vec<usize>,
    offered: bool,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    rows: Vec<Row>,
    first_leaf: usize,
    /// (sort key, durability) of an owned copy of an item, by (kind bits, base id).
    copy_of: HashMap<(u32, u32), (u32, u32)>,
    /// The lowest durability owned, by kind bits.
    lowest: HashMap<u32, u32>,
}

/// Unused entries offered for new items: (kind, first position, count). Always the same
/// entries, so the list reads back the same after saving.
const OFFERED: [(Kind, usize, usize); 5] =
    [(Kind::Key, 48, 16), (Kind::Ring, 1904, 16), (Kind::Armor, 1920, 32), (Kind::Weapon, 1952, 32), (Kind::Good, 1984, 64)];

fn offered_kind(i: usize) -> Option<Kind> {
    OFFERED.iter().find(|(_, s, n)| (*s..s + n).contains(&i)).map(|(k, _, _)| *k)
}

fn in_use(d: &[u8], off: usize) -> bool {
    u32_at(d, off + 16) == 1
}

pub fn build(d: &[u8], first_leaf: usize, root: &mut Map<String, Value>, editable: &mut HashMap<String, usize>) -> Option<Inventory> {
    if d.len() < HIGHEST + 4 {
        return None;
    }
    let equipped: Vec<(usize, usize)> = (0..EQUIP_SLOTS).map(|i| (u32_at(d, EQUIPPED + i * 4) as usize, EQUIPPED_IDS + i * 4)).collect();
    let mut inv = Inventory { rows: vec![], first_leaf, copy_of: HashMap::new(), lowest: HashMap::new() };
    let mut sections: Vec<(&str, Map<String, Value>)> = ["Items", "Key items", "Weapons", "Armor", "Rings"].iter().map(|s| (*s, Map::new())).collect();
    for i in 0..COUNT {
        let off = LIST + i * ENTRY;
        let (cat, id, qty) = (u32_at(d, off), u32_at(d, off + 4), u32_at(d, off + 8));
        let used = in_use(d, off);
        let offered = offered_kind(i);
        let kind = match (used, offered) {
            (true, _) => match cat >> 28 {
                0x4 if i < KEY_SLOTS => Kind::Key,
                0x4 => Kind::Good,
                0x0 => Kind::Weapon,
                0x1 => Kind::Armor,
                0x2 => Kind::Ring,
                _ => continue,
            },
            (false, Some(k)) => k,
            (false, None) => continue,
        };
        if used {
            let (order, dur) = (u32_at(d, off + 12), u32_at(d, off + 20));
            inv.copy_of.entry((kind.bits(), item_of(kind, id))).or_insert((order >> 12, dur));
            if !kind.stacks() {
                inv.lowest.entry(kind.bits()).and_modify(|l| *l = (*l).min(dur)).or_insert(dur);
            }
        }
        let sec = match kind {
            Kind::Good => 0,
            Kind::Key => 1,
            k => k as usize + 1,
        };
        let key = format!("Slot {}", i + 1);
        let base = format!("/{}/{key}", sections[sec].0);
        let mut slot = Map::new();
        let mut ptrs: [Option<String>; 3] = Default::default();
        let name = match (used, kind) {
            (false, _) => EMPTY.to_string(),
            (true, Kind::Weapon) => names(kind).name(base_of(id), "Weapon"),
            (true, k) => names(k).name(id, k.word()),
        };
        slot.insert("Item".into(), Value::from(name.clone()));
        ptrs[0] = Some(format!("{base}/Item"));
        if kind.stacks() || kind == Kind::Weapon {
            slot.insert("Quantity".into(), Value::from(if used { qty } else { 0 }));
            ptrs[1] = Some(format!("{base}/Quantity"));
        }
        if kind == Kind::Weapon {
            let plain = used && !is_ammo(id);
            slot.insert("Level".into(), Value::from(if plain { id % 100 } else { 0 }));
            let inf = if plain { INFUSIONS.get(((id % 1000) / 100) as usize).copied().unwrap_or("?") } else { NO_AFFINITY };
            slot.insert("Infusion".into(), Value::from(inf));
            ptrs[2] = Some(format!("{base}/Level"));
        }
        sections[sec].1.insert(key, Value::Object(slot));
        let r = inv.rows.len();
        for (f, p) in ptrs.iter().enumerate() {
            if let Some(p) = p {
                editable.insert(p.clone(), first_leaf + r * 3 + f);
            }
        }
        let equipped_ids = equipped.iter().filter(|(p, _)| used && *p == i).map(|(_, at)| *at).collect();
        inv.rows.push(Row { kind, slot: i, off, ptrs, id: used.then_some(id), orig_name: name, equipped_ids, offered: offered == Some(kind) });
    }
    for (s, m) in sections {
        root.insert(s.into(), Value::Object(m));
    }
    Some(inv)
}

impl Inventory {
    fn row(&self, leaf: usize) -> Option<(&Row, usize)> {
        let i = leaf.checked_sub(self.first_leaf)?;
        self.rows.get(i / 3).map(|r| (r, i % 3))
    }

    pub fn owns(&self, leaf: usize) -> bool {
        self.row(leaf).is_some()
    }

    fn get<'a>(row: &Row, view: &'a Value, f: usize) -> Option<&'a Value> {
        row.ptrs[f].as_ref().and_then(|p| view.pointer(p))
    }

    /// The base id a row shows: by name, or the one it had.
    fn id_in(row: &Row, name: &str) -> Option<u32> {
        if name == row.orig_name {
            if let Some(id) = row.id {
                return Some(item_of(row.kind, id));
            }
        }
        names(row.kind).by_name.get(name).copied()
    }

    pub fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let (row, f) = self.row(leaf).ok_or("unknown value")?;
        match f {
            1 => return int_in(v, 0, if row.kind == Kind::Weapon { MAX_AMMO } else { MAX_QTY }, "Quantity").map(|_| ()),
            2 => return int_in(v, 0, 15, "Level").map(|_| ()),
            _ => {}
        }
        let name = v.as_str().ok_or("pick an item from the list")?;
        let what = row.kind.word().to_lowercase();
        if name != row.orig_name && name != EMPTY && !names(row.kind).by_name.contains_key(name) {
            return Err(format!("\u{201C}{name}\u{201D} isn't a Dark Souls {what}"));
        }
        if !row.equipped_ids.is_empty() && name != row.orig_name {
            return Err(format!("this {what} is equipped \u{2014} take it off in the game first"));
        }
        // Items elsewhere would drop out of the list (only the reserved entries are listed empty).
        if name == EMPTY && row.id.is_some() && !row.offered {
            return Err(format!("this {what} can't be removed here \u{2014} drop or sell it in the game"));
        }
        Ok(())
    }

    /// An empty slot holds nothing; ammunition has a quantity and no level; other weapons
    /// hold one and stay within +5, or the level they had.
    pub fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Some((row, _)) = self.row(leaf) else { return vec![] };
        let name = Self::get(row, view, 0).and_then(Value::as_str).unwrap_or(EMPTY);
        let empty = name == EMPTY;
        let mut out = vec![];
        let mut set = |p: &str, v: Value| {
            if view.pointer(p) != Some(&v) {
                out.push((p.to_string(), v));
            }
        };
        let qty = Self::get(row, view, 1).and_then(Value::as_u64).unwrap_or(0);
        let base = if empty { None } else { Self::id_in(row, name) };
        let ammo = row.kind == Kind::Weapon && base.is_some_and(is_ammo);
        if let Some(p) = &row.ptrs[1] {
            let q = match (empty, row.kind) {
                (true, _) => 0,
                (_, Kind::Weapon) if !ammo => 1,
                _ => qty.max(1),
            };
            set(p, Value::from(q));
        }
        if let Some(p) = &row.ptrs[2] {
            let same = name == row.orig_name && row.id.is_some();
            let had = if same { row.id.map_or(0, |id| id % 100) } else { 0 };
            let max = if empty || ammo { 0 } else { had.max(5) } as u64;
            let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0);
            set(p, Value::from(level.min(max)));
            let inf = p.replace("/Level", "/Infusion");
            let v = if empty || ammo {
                Value::from(NO_AFFINITY)
            } else if same {
                Value::from(INFUSIONS.get(((row.id.unwrap() % 1000) / 100) as usize).copied().unwrap_or("?"))
            } else {
                Value::from(INFUSIONS[0])
            };
            set(&inf, v);
        }
        out
    }

    pub fn choices(&self, leaf: usize, view: &Value) -> Option<Arc<Vec<String>>> {
        let (row, f) = self.row(leaf)?;
        if f != 0 {
            return None;
        }
        // Goods stack: each once. Weapons, armor and rings can repeat.
        let taken: BTreeSet<&str> = if row.kind.stacks() {
            self.rows
                .iter()
                .filter(|r| r.kind == row.kind && r.ptrs[0] != row.ptrs[0])
                .filter_map(|r| Self::get(r, view, 0).and_then(Value::as_str))
                .filter(|n| *n != EMPTY)
                .collect()
        } else {
            BTreeSet::new()
        };
        let mut all: Vec<String> = names(row.kind).all.iter().filter(|n| !taken.contains(n.as_str())).cloned().collect();
        if !row.equipped_ids.is_empty() || (row.id.is_some() && !row.offered) {
            all.retain(|n| n != EMPTY);
        }
        if row.orig_name != EMPTY && !names(row.kind).by_name.contains_key(&row.orig_name) {
            all.insert(0, row.orig_name.clone());
        }
        Some(Arc::new(all))
    }

    pub fn write(&self, out: &mut [u8], view: &Value, changes: &[(usize, Value)]) -> Result<(), String> {
        let touched: BTreeSet<usize> = changes.iter().filter_map(|(l, _)| l.checked_sub(self.first_leaf).map(|i| i / 3)).filter(|&r| r < self.rows.len()).collect();
        let (mut common, mut key) = (0i64, 0i64);
        let mut highest = u32_at(out, HIGHEST);
        let mut added = vec![];
        for r in touched {
            let row = &self.rows[r];
            let name = Self::get(row, view, 0).and_then(Value::as_str).unwrap_or(EMPTY);
            let qty = Self::get(row, view, 1).and_then(Value::as_u64).unwrap_or(1) as u32;
            let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0) as u32;
            let what = row.kind.word().to_lowercase();
            let base = if name == EMPTY { None } else { Some(Self::id_in(row, name).ok_or_else(|| format!("\u{201C}{name}\u{201D} isn't a Dark Souls {what}"))?) };
            // The full id: a weapon keeps its infusion unless it's a different weapon now.
            let full = base.map(|b| match row.kind {
                Kind::Weapon if is_ammo(b) => b,
                Kind::Weapon => {
                    let inf = row.id.filter(|&id| base_of(id) == b).map_or(0, |id| id % 1000 / 100);
                    b + inf * 100 + level
                }
                _ => b,
            });
            let o = row.off;
            let count = if row.slot < KEY_SLOTS { &mut key } else { &mut common };
            match (row.id, full) {
                (None, None) => {}
                (Some(_), None) => {
                    out[o..o + ENTRY].fill(0xFF);
                    put(out, o + 16, 0);
                    *count -= 1;
                }
                (was, Some(id)) => {
                    let same_item = was.is_some_and(|w| item_of(row.kind, w) == item_of(row.kind, id));
                    if !same_item {
                        if row.kind.stacks() {
                            added.push((row.kind, id));
                        }
                        let durability = if row.kind.stacks() { 0 } else { self.lowest.get(&row.kind.bits()).copied().unwrap_or(DEFAULT_DURABILITY) };
                        let (sort, durability) = self.copy_of.get(&(row.kind.bits(), item_of(row.kind, id))).copied().unwrap_or((0, durability));
                        put(out, o, row.kind.bits());
                        put(out, o + 12, (sort << 12) | row.slot as u32);
                        put(out, o + 16, 1);
                        put(out, o + 20, durability);
                        put(out, o + 24, 0);
                        if was.is_none() {
                            *count += 1;
                            highest = highest.max(row.slot as u32);
                        }
                    }
                    let q = if row.kind.stacks() || (row.kind == Kind::Weapon && is_ammo(id)) { qty.max(1) } else { 1 };
                    put(out, o + 4, id);
                    put(out, o + 8, q);
                    for &at in &row.equipped_ids {
                        put(out, at, id);
                    }
                }
            }
        }
        for (kind, id) in added {
            if self.rows.iter().filter(|r| r.kind == kind && in_use(out, r.off) && u32_at(out, r.off + 4) == id).count() > 1 {
                return Err(format!("{} is there already \u{2014} change its quantity instead", names(kind).name(id, kind.word())));
            }
        }
        for (at, d) in [(COMMON_COUNT, common), (KEY_COUNT, key)] {
            if d != 0 {
                let n = (u32_at(out, at) as i64 + d).max(0) as u32;
                put(out, at, n);
            }
        }
        put(out, HIGHEST, highest);
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Marks every entry unused, as the game does.
    pub fn blank(d: &mut [u8]) {
        for i in 0..COUNT {
            let o = LIST + i * ENTRY;
            d[o..o + ENTRY].fill(0xFF);
            put(d, o + 16, 0);
        }
    }

    /// Puts an item in the list: `cat` is the kind's top four bits.
    pub fn add(d: &mut [u8], slot: usize, cat: u32, id: u32, qty: u32) {
        let o = LIST + slot * ENTRY;
        put(d, o, cat << 28);
        put(d, o + 4, id);
        put(d, o + 8, qty);
        put(d, o + 12, (7 << 12) | slot as u32);
        put(d, o + 16, 1);
        put(d, o + 20, 200);
        put(d, o + 24, 0);
        let at = if slot < KEY_SLOTS { KEY_COUNT } else { COMMON_COUNT };
        let n = u32_at(d, at) + 1;
        put(d, at, n);
        let h = u32_at(d, HIGHEST).max(slot as u32);
        put(d, HIGHEST, h);
    }

    /// Equips the item at `slot` in the first right-hand slot.
    pub fn equip(d: &mut [u8], slot: usize) {
        let id = u32_at(d, LIST + slot * ENTRY + 4);
        put(d, EQUIPPED + 4, slot as u32);
        put(d, EQUIPPED_IDS + 4, id);
    }

    /// (common count, key count, highest position in use).
    pub fn counts(d: &[u8]) -> (u32, u32, u32) {
        (u32_at(d, COMMON_COUNT), u32_at(d, KEY_COUNT), u32_at(d, HIGHEST))
    }
}
