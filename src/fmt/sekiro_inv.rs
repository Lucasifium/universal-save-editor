//! Sekiro's item tables: what the Wolf carries, the key items, and the surplus the game
//! moves to storage once an item's carry limit is reached.
//!
//! Each table is a run of 16-byte entries: a handle (the item kind in the top four bits,
//! 0xB… for goods), the item id (0x4… for goods), the quantity, and an index (a running
//! number in the low 12 bits, the game's sort order above that). Empty entries read
//! `0, 0xFFFFFFFF, 0, 0`. A header before the carried items counts the entries of the
//! carried and key tables and holds the last index given out; storage has its own.
//!
//! Only goods are edited here. Skills, combat arts and prosthetic tools are also kept
//! in the carried table (as "weapons"), but they point into another table, so they're
//! listed, not changed. Checked against sample saves and alfizari's Sekiro-Save-Editor.

use super::sekiro_items::{GOODS, WEAPONS};
use crate::tree::int_in;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

pub const EMPTY: &str = "(empty)";
const ENTRY: usize = 16;
const CARRIED_CAP: usize = 0x700;
const KEY_CAP: usize = 0x200;
const STORAGE_CAP: usize = 0x700;
/// From the carried table's start: the key table, the quick-item slots and storage's header.
const KEY_AT: usize = CARRIED_CAP * ENTRY;
const QUICK_AT: usize = 0x9000;
const QUICK_LEN: usize = 0x84;
const STORAGE_HDR: usize = 0x9084;
const STORAGE_AT: usize = 0x9094;
/// Slots listed from the start of each table (carried, key, storage), empty or not, so the
/// list doesn't depend on what's in them: filling or emptying one gives the same list when
/// the saved file is read back. Real saves use under 270, 75 and 45. Goods further on are
/// listed too but can't be emptied (they'd drop out of the list).
const LISTED: [usize; 3] = [320, 128, 128];
const GOOD: u32 = 0xB000_0000;
const GOOD_ID: u32 = 0x4000_0000;

fn u32_at(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4)
        .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn put(d: &mut [u8], o: usize, v: u32) {
    d[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn entry(d: &[u8], o: usize) -> [u32; 4] {
    [
        u32_at(d, o),
        u32_at(d, o + 4),
        u32_at(d, o + 8),
        u32_at(d, o + 12),
    ]
}

fn is_empty(e: [u32; 4]) -> bool {
    e == [0, u32::MAX, 0, 0]
}

fn is_good(e: [u32; 4]) -> bool {
    e[0] & 0xF000_0000 == GOOD && e[1] == (e[0] & 0x0FFF_FFFF) | GOOD_ID
}

fn valid(e: [u32; 4]) -> bool {
    is_empty(e) || (matches!(e[0] >> 28, 0x8..=0xB) && (e[0] >> 28 != 0xB || is_good(e)))
}

/// Number of used entries in a table, or `None` if something in it doesn't look like an entry.
fn used(d: &[u8], at: usize, cap: usize) -> Option<usize> {
    let mut n = 0;
    for i in 0..cap {
        let e = entry(d, at + i * ENTRY);
        if !valid(e) {
            return None;
        }
        n += !is_empty(e) as usize;
    }
    Some(n)
}

/// Offset of the carried-items table: it follows its header (last index, carried count,
/// key count, 0), and both counts match the tables.
pub fn find(d: &[u8]) -> Option<usize> {
    let end = d.len().checked_sub(STORAGE_AT + STORAGE_CAP * ENTRY)?;
    (0x10..end).step_by(4).find(|&t| {
        let (last, carried, key) = (
            u32_at(d, t - 0x10),
            u32_at(d, t - 0xC) as usize,
            u32_at(d, t - 8) as usize,
        );
        u32_at(d, t - 4) == 0
            && (0x100..0x1000).contains(&last)
            && (1..=CARRIED_CAP).contains(&carried)
            && key <= KEY_CAP
            && valid(entry(d, t))
            && used(d, t, CARRIED_CAP) == Some(carried)
            && used(d, t + KEY_AT, KEY_CAP) == Some(key)
    })
}

struct Names {
    by_id: HashMap<u32, String>,
    by_name: HashMap<String, u32>,
    /// "(empty)" and every good, sorted.
    all: Arc<Vec<String>>,
}

/// Goods by id and by name; names the game uses twice get their id added.
fn names() -> &'static Names {
    static N: OnceLock<Names> = OnceLock::new();
    N.get_or_init(|| {
        let mut count: HashMap<&str, usize> = HashMap::new();
        for (_, n) in GOODS {
            *count.entry(n).or_default() += 1;
        }
        let mut by_id = HashMap::new();
        let mut by_name = HashMap::new();
        for &(id, n) in GOODS {
            let shown = if count[n] > 1 {
                format!("{n} (#{id})")
            } else {
                n.to_string()
            };
            by_name.insert(shown.clone(), id);
            by_id.insert(id, shown);
        }
        let mut all: Vec<String> = by_name.keys().cloned().collect();
        all.sort_by_key(|s| s.to_lowercase());
        all.insert(0, EMPTY.into());
        Names {
            by_id,
            by_name,
            all: Arc::new(all),
        }
    })
}

fn good_name(id: u32) -> String {
    names()
        .by_id
        .get(&id)
        .cloned()
        .unwrap_or_else(|| format!("Item #{id}"))
}

#[derive(Clone, Debug)]
struct Table {
    section: &'static str,
    start: usize,
    cap: usize,
    count_at: usize,
    /// Where the last index given out is kept (the key table has none: it uses the highest one).
    last_at: Option<usize>,
}

#[derive(Clone, Debug)]
struct Row {
    table: usize,
    off: usize,
    item_ptr: String,
    qty_ptr: String,
    /// The good here when the file was read.
    orig: Option<u32>,
    orig_name: String,
    /// Set in a quick-item slot, which refers to it by its handle.
    quick: bool,
    /// Within the listed part of its table (see `LISTED`).
    listed: bool,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    tables: Vec<Table>,
    rows: Vec<Row>,
    first_leaf: usize,
    /// The game's sort order for goods that are somewhere in this save (copied to new entries).
    sort: HashMap<u32, u32>,
}

/// Adds the item sections to `root` (editable leaves numbered from `first_leaf`) and the
/// list of learned skills and tools. `None` if the tables aren't where they should be.
pub fn build(
    d: &[u8],
    first_leaf: usize,
    root: &mut Map<String, Value>,
    editable: &mut HashMap<String, usize>,
) -> Option<Inventory> {
    let t = find(d)?;
    let mut tables = vec![
        Table {
            section: "Items",
            start: t,
            cap: CARRIED_CAP,
            count_at: t - 0xC,
            last_at: Some(t - 0x10),
        },
        Table {
            section: "Key items",
            start: t + KEY_AT,
            cap: KEY_CAP,
            count_at: t - 8,
            last_at: None,
        },
    ];
    let storage_count = u32_at(d, t + STORAGE_HDR + 4) as usize;
    if used(d, t + STORAGE_AT, STORAGE_CAP) == Some(storage_count) {
        tables.push(Table {
            section: "Storage",
            start: t + STORAGE_AT,
            cap: STORAGE_CAP,
            count_at: t + STORAGE_HDR + 4,
            last_at: Some(t + STORAGE_HDR),
        });
    }
    let quick: BTreeSet<u32> = (0..QUICK_LEN / 4)
        .map(|i| u32_at(d, t + QUICK_AT + i * 4))
        .filter(|h| h >> 28 == 0xB)
        .collect();
    let weapons: HashMap<u32, &str> = WEAPONS.iter().copied().collect();

    let mut inv = Inventory {
        tables,
        rows: vec![],
        first_leaf,
        sort: HashMap::new(),
    };
    let mut learned = BTreeSet::new();
    for (ti, tb) in inv.tables.iter().enumerate() {
        let mut sec = Map::new();
        for i in 0..tb.cap {
            let listed = i < LISTED[ti];
            let off = tb.start + i * ENTRY;
            let e = entry(d, off);
            if e[0] >> 28 == 0x8 {
                if let Some(n) = weapons
                    .get(&e[1])
                    .filter(|n| !n.starts_with("Virtual Weapon"))
                {
                    learned.insert(n.to_string());
                }
            }
            let good = is_good(e).then_some(e[0] & 0x0FFF_FFFF);
            if let Some(id) = good {
                inv.sort.entry(id).or_insert(e[3] & !0xFFF);
            } else if !is_empty(e) || !listed {
                continue;
            }
            let key = format!("Slot {}", i + 1);
            let name = good.map_or(EMPTY.to_string(), good_name);
            let qty = if good.is_some() { e[2] } else { 0 };
            let mut slot = Map::new();
            slot.insert("Item".into(), Value::from(name.clone()));
            slot.insert("Quantity".into(), Value::from(qty));
            sec.insert(key.clone(), Value::Object(slot));
            let base = format!("/{}/{key}", tb.section);
            let row = Row {
                table: ti,
                off,
                item_ptr: format!("{base}/Item"),
                qty_ptr: format!("{base}/Quantity"),
                orig: good,
                orig_name: name,
                quick: quick.contains(&e[0]) && good.is_some(),
                listed,
            };
            editable.insert(row.item_ptr.clone(), first_leaf + inv.rows.len() * 2);
            editable.insert(row.qty_ptr.clone(), first_leaf + inv.rows.len() * 2 + 1);
            inv.rows.push(row);
        }
        root.insert(tb.section.into(), Value::Object(sec));
    }
    let mut skills = Map::new();
    skills.insert(
        "Skills & prosthetic tools".into(),
        Value::Array(learned.into_iter().map(Value::from).collect()),
    );
    root.insert("Learned".into(), Value::Object(skills));
    Some(inv)
}

impl Inventory {
    /// (row, is the quantity) for one of this inventory's leaves.
    fn row(&self, leaf: usize) -> Option<(&Row, bool)> {
        let i = leaf.checked_sub(self.first_leaf)?;
        self.rows.get(i / 2).map(|r| (r, i % 2 == 1))
    }

    pub fn owns(&self, leaf: usize) -> bool {
        self.row(leaf).is_some()
    }

    pub fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let (row, is_qty) = self.row(leaf).ok_or("unknown value")?;
        if is_qty {
            return int_in(v, 0, 9_999, "Quantity").map(|_| ());
        }
        let name = v.as_str().ok_or("pick an item from the list")?;
        if name != row.orig_name && name != EMPTY && !names().by_name.contains_key(name) {
            return Err(format!("\u{201C}{name}\u{201D} isn't a Sekiro item"));
        }
        if row.quick && name != row.orig_name {
            return Err("this item is set in a quick-item slot \u{2014} take it off there in the game first".into());
        }
        if !row.listed && name == EMPTY {
            return Err("items this far down the list can't be removed here \u{2014} set the quantity instead".into());
        }
        Ok(())
    }

    /// An empty slot holds nothing; an item holds at least one.
    pub fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Some((row, _)) = self.row(leaf) else {
            return vec![];
        };
        let empty = view.pointer(&row.item_ptr).and_then(Value::as_str) == Some(EMPTY);
        let qty = view
            .pointer(&row.qty_ptr)
            .and_then(Value::as_u64)
            .unwrap_or(0);
        match (empty, qty) {
            (true, q) if q != 0 => vec![(row.qty_ptr.clone(), Value::from(0))],
            (false, 0) => vec![(row.qty_ptr.clone(), Value::from(1))],
            _ => vec![],
        }
    }

    /// Every item, less those already in another slot of the same table.
    pub fn choices(&self, leaf: usize, view: &Value) -> Option<Arc<Vec<String>>> {
        let (row, is_qty) = self.row(leaf)?;
        if is_qty {
            return None;
        }
        let taken: BTreeSet<&str> = self
            .rows
            .iter()
            .filter(|r| r.table == row.table && r.item_ptr != row.item_ptr)
            .filter_map(|r| view.pointer(&r.item_ptr).and_then(Value::as_str))
            .filter(|n| *n != EMPTY)
            .collect();
        let mut all: Vec<String> = names()
            .all
            .iter()
            .filter(|n| !taken.contains(n.as_str()))
            .cloned()
            .collect();
        if !names().by_name.contains_key(&row.orig_name) && row.orig_name != EMPTY {
            all.insert(1, row.orig_name.clone());
        }
        Some(Arc::new(all))
    }

    /// Writes the changed slots (and the tables' counts) into `out`.
    pub fn write(
        &self,
        out: &mut [u8],
        view: &Value,
        changes: &[(usize, Value)],
    ) -> Result<(), String> {
        let touched: BTreeSet<usize> = changes
            .iter()
            .filter_map(|(l, _)| l.checked_sub(self.first_leaf).map(|i| i / 2))
            .filter(|&r| r < self.rows.len())
            .collect();
        if touched.is_empty() {
            return Ok(());
        }
        let mut last: Vec<u32> = self
            .tables
            .iter()
            .map(|t| match t.last_at {
                Some(a) => u32_at(out, a),
                None => (0..t.cap)
                    .map(|i| entry(out, t.start + i * ENTRY))
                    .filter(|e| !is_empty(*e))
                    .map(|e| e[3] & 0xFFF)
                    .max()
                    .unwrap_or(0),
            })
            .collect();
        let mut delta = vec![0i64; self.tables.len()];
        // Goods this edit puts into a slot (the game itself sometimes lists one twice).
        let mut added: Vec<(usize, u32)> = vec![];
        for r in touched {
            let row = &self.rows[r];
            let name = view
                .pointer(&row.item_ptr)
                .and_then(Value::as_str)
                .unwrap_or(EMPTY);
            let qty = view
                .pointer(&row.qty_ptr)
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            let new = if name == EMPTY {
                None
            } else if name == row.orig_name {
                row.orig
            } else {
                Some(
                    *names()
                        .by_name
                        .get(name)
                        .ok_or_else(|| format!("\u{201C}{name}\u{201D} isn't a Sekiro item"))?,
                )
            };
            let o = row.off;
            match (row.orig, new) {
                (None, None) => {}
                (Some(_), None) => {
                    put(out, o, 0);
                    put(out, o + 4, u32::MAX);
                    put(out, o + 8, 0);
                    put(out, o + 12, 0);
                    delta[row.table] -= 1;
                }
                (None, Some(id)) => {
                    added.push((row.table, id));
                    last[row.table] += 1;
                    put(out, o, GOOD | id);
                    put(out, o + 4, GOOD_ID | id);
                    put(out, o + 8, qty.max(1));
                    put(
                        out,
                        o + 12,
                        (last[row.table] & 0xFFF) | self.sort.get(&id).copied().unwrap_or(0),
                    );
                    delta[row.table] += 1;
                }
                (Some(was), Some(id)) => {
                    if was != id {
                        added.push((row.table, id));
                        let idx = u32_at(out, o + 12);
                        put(out, o, GOOD | id);
                        put(out, o + 4, GOOD_ID | id);
                        put(
                            out,
                            o + 12,
                            (idx & 0xFFF) | self.sort.get(&id).copied().unwrap_or(idx & !0xFFF),
                        );
                    }
                    put(out, o + 8, qty.max(1));
                }
            }
        }
        for (i, t) in self.tables.iter().enumerate() {
            for &(_, id) in added.iter().filter(|(li, _)| *li == i) {
                let n = (0..t.cap)
                    .map(|k| entry(out, t.start + k * ENTRY))
                    .filter(|e| is_good(*e) && e[0] & 0x0FFF_FFFF == id)
                    .count();
                if n > 1 {
                    return Err(format!(
                        "{} already has {} \u{2014} change its quantity instead",
                        t.section,
                        good_name(id)
                    ));
                }
            }
            let n = u32_at(out, t.count_at) as i64 + delta[i];
            put(out, t.count_at, n as u32);
            if let Some(a) = t.last_at {
                put(out, a, last[i]);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A slot-sized buffer with an empty set of tables at `t`.
    pub fn blank(len: usize, t: usize) -> Vec<u8> {
        let mut d = vec![0u8; len];
        for (at, cap) in [
            (t, CARRIED_CAP),
            (t + KEY_AT, KEY_CAP),
            (t + STORAGE_AT, STORAGE_CAP),
        ] {
            for i in 0..cap {
                put(&mut d, at + i * ENTRY + 4, u32::MAX);
            }
        }
        put(&mut d, t - 0x10, 0xFF);
        put(&mut d, t + STORAGE_HDR, 0xFF);
        d
    }

    /// Adds a good to a table (0 carried, 1 key, 2 storage), keeping the headers right.
    pub fn add(d: &mut [u8], t: usize, table: usize, slot: usize, id: u32, qty: u32) {
        let (at, count, last) = match table {
            0 => (t, t - 0xC, Some(t - 0x10)),
            1 => (t + KEY_AT, t - 8, None),
            _ => (t + STORAGE_AT, t + STORAGE_HDR + 4, Some(t + STORAGE_HDR)),
        };
        let idx = match last {
            Some(a) => {
                let n = u32_at(d, a) + 1;
                put(d, a, n);
                n
            }
            None => slot as u32,
        };
        let o = at + slot * ENTRY;
        put(d, o, GOOD | id);
        put(d, o + 4, GOOD_ID | id);
        put(d, o + 8, qty);
        put(d, o + 12, idx | (id << 12));
        let c = u32_at(d, count) + 1;
        put(d, count, c);
    }

    pub fn quick(d: &mut [u8], t: usize, id: u32) {
        put(d, t + QUICK_AT + 4, GOOD | id);
    }
}
