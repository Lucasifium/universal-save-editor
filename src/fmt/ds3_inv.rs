//! Dark Souls III's item lists: what the character carries, and the storage box.
//!
//! Like Elden Ring: the slot starts with an item table (0x1800 entries at 0x70; weapons
//! and armor take 60 bytes, anything else 8, an unused entry is handle 0, id 0xFFFFFFFF)
//! that ends 0x34 bytes before Vigor. The carried list (0x780 entries of 16 bytes: handle,
//! item id, quantity, index) sits 0x2E8 bytes after Vigor, after a 16-bit count; a second
//! counter follows the list. Storage comes after a variable-length table, with its own
//! count before it. Goods (0xB… handle, 0x4… id) and rings (0xA…, 0x2…) are the item
//! itself; weapons (0x8…) and armor (0x9…) point into the item table, where a weapon's id
//! is base + infusion × 100 + level. Adding a weapon or armor makes the table 52 bytes
//! longer; the slot keeps its size by giving up bytes from the random filler the game
//! writes at its end (the last 13 bytes are kept). Checked against a sample save and
//! alfizari's Dark-Souls-3-Save-Editor.

use super::ds3_items::{ARMOR, GOODS, RINGS, WEAPONS};
use super::er_inv::Names;
use super::sekiro_inv::EMPTY;
use crate::tree::int_in;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

const ENTRY: usize = 16;
const CAP: usize = 0x780;
const GA_START: usize = 0x70;
const GA_COUNT: usize = 0x1800;
const GA_EQUIP_LEN: usize = 60;
/// From Vigor: worn equipment, then the carried items' count and list.
const WORN_FROM_VIGOR: usize = 0x288;
const INV_FROM_VIGOR: usize = 0x2E8;
/// From the carried list: the second counter.
const COUNTER2: usize = 0x8808;
/// From the second counter: the table before storage (u32 count, 8 bytes each).
const TABLE_FROM_COUNTER2: usize = 0x11C;
const STORAGE_AFTER_TABLE: usize = 0x18C + 4;
/// The slot's last bytes, kept when the item table grows.
const KEEP_AT_END: usize = 13;
/// How much the item table may grow (the filler at the end is far larger).
const MAX_GROWTH: usize = 0x4000;
const UNARMED: u32 = 110_000;
pub const INFUSIONS: [&str; 16] = [
    "Standard", "Heavy", "Sharp", "Refined", "Simple", "Crystal", "Fire", "Chaos", "Lightning", "Deep", "Dark", "Poison", "Blood", "Raw", "Blessed", "Hollow",
];
pub use super::er_inv::NO_AFFINITY;
/// Most of an item carried; storage holds more.
const MAX_CARRIED: i128 = 99;
const MAX_STORED: i128 = 600;
/// New table entries' tails (after handle and id), as the save editor writes them.
const WEAPON_TAIL: [u8; 52] = tail(0x4B);
const ARMOR_TAIL: [u8; 52] = tail(0x168);
/// Upper half of a new entry's index, by kind, as the save editor writes it.
const INDEX_HIGH: [u32; 4] = [0x1FCF, 0xFB18, 0xFE65, 0x1FCF];

const fn tail(durability: u32) -> [u8; 52] {
    let mut t = [0u8; 52];
    let d = durability.to_le_bytes();
    t[0] = d[0];
    t[1] = d[1];
    t[8] = 1;
    let mut i = 15;
    while i < 52 {
        t[i] = 0x80;
        i += 8;
    }
    t
}

fn u32_at(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    d.get(o..o + 2).map_or(0, |b| u16::from_le_bytes(b.try_into().unwrap()))
}

fn put(d: &mut [u8], o: usize, v: u32) {
    d[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn entry(d: &[u8], o: usize) -> [u32; 4] {
    [u32_at(d, o), u32_at(d, o + 4), u32_at(d, o + 8), u32_at(d, o + 12)]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Good,
    Weapon,
    Armor,
    Ring,
}

impl Kind {
    fn of(handle: u32) -> Option<Kind> {
        Some(match handle >> 28 {
            0xB => Kind::Good,
            0x8 => Kind::Weapon,
            0x9 => Kind::Armor,
            0xA => Kind::Ring,
            _ => return None,
        })
    }

    fn word(self) -> &'static str {
        match self {
            Kind::Good => "Item",
            Kind::Weapon => "Weapon",
            Kind::Armor => "Armor",
            Kind::Ring => "Ring",
        }
    }

    /// The item id's kind bits.
    fn id_bits(self) -> u32 {
        match self {
            Kind::Good => 0x4000_0000,
            Kind::Weapon => 0,
            Kind::Armor => 0x1000_0000,
            Kind::Ring => 0x2000_0000,
        }
    }
}

fn names(kind: Kind) -> &'static Names {
    static N: OnceLock<[Names; 4]> = OnceLock::new();
    &N.get_or_init(|| {
        [
            Names::new(GOODS.iter().map(|&(i, n)| (i, n))),
            Names::new(WEAPONS.iter().filter(|w| w.0 != UNARMED).map(|&(i, n, _)| (i, n))),
            Names::new(ARMOR.iter().map(|&(i, n)| (i, n))),
            Names::new(RINGS.iter().map(|&(i, n)| (i, n))),
        ]
    })[kind as usize]
}

fn infusable(base: u32) -> bool {
    static W: OnceLock<HashMap<u32, bool>> = OnceLock::new();
    W.get_or_init(|| WEAPONS.iter().map(|&(i, _, f)| (i, f)).collect()).get(&base).copied().unwrap_or(false)
}

fn is_ammo(base: u32) -> bool {
    base < 1_000_000
}

/// A weapon's base id (ammunition ids are whole items).
fn base_of(id: u32) -> u32 {
    if is_ammo(id) {
        id
    } else {
        id - id % 10_000
    }
}

/// Highest level: +10 for weapons that take infusions, +5 for the rest (unique and boss
/// weapons stop there; a few plain ones can go further, but +5 is always valid), or the
/// level it has if that's higher. Ammunition has none.
fn max_level(base: u32, has: u32) -> u32 {
    if is_ammo(base) {
        0
    } else if infusable(base) {
        10
    } else {
        has.max(5)
    }
}

fn infusion_names(base: Option<u32>) -> Vec<String> {
    match base {
        Some(b) if infusable(b) => INFUSIONS.iter().map(|s| s.to_string()).collect(),
        _ => vec![NO_AFFINITY.into()],
    }
}

#[derive(Clone, Debug)]
struct List {
    section: &'static str,
    start: usize,
    count_at: usize,
    /// The 16-bit count (carried) or 32-bit (storage), and the carried list's second counter.
    wide_count: bool,
    counter2: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct Ga {
    off: usize,
    id: u32,
}

#[derive(Clone, Debug)]
struct Row {
    list: usize,
    kind: Kind,
    off: usize,
    ptrs: [Option<String>; 4],
    handle: Option<u32>,
    orig_name: String,
    ga: Option<Ga>,
    equipped: bool,
    offered: bool,
}

#[derive(Clone, Debug)]
struct Table {
    free: Vec<usize>,
    last_handle: u32,
    /// Upper 16 bits of new weapon and armor handles (as the save already uses).
    prefix: [u32; 2],
}

#[derive(Clone, Debug)]
pub struct Inventory {
    lists: Vec<List>,
    rows: Vec<Row>,
    first_leaf: usize,
    table: Option<Table>,
}

/// Unused entries reserved at the end of each list (carried, storage) per kind: goods,
/// weapons, armor, rings (see Elden Ring's `OFFERED`).
const OFFERED: [[usize; 4]; 2] = [[64, 32, 32, 16], [32, 0, 0, 0]];

fn offered_kind(list: usize, i: usize) -> Option<Kind> {
    let mut end = CAP;
    for (k, &n) in [Kind::Good, Kind::Weapon, Kind::Armor, Kind::Ring].iter().zip(&OFFERED[list]) {
        if i < end && i + n >= end {
            return Some(*k);
        }
        end -= n;
    }
    None
}

/// An unused list entry: the game writes `0, 0xFFFFFFFF, 0, 0`; never-used ones are zeros.
fn is_free(e: [u32; 4]) -> bool {
    e == [0; 4] || e == [0, u32::MAX, 0, 0]
}

fn valid(e: [u32; 4]) -> bool {
    is_free(e) || matches!(e[0] >> 28, 0x8..=0xB)
}

/// The item table, if it ends where the character data begins.
fn read_table(d: &[u8], vigor: usize) -> Option<(HashMap<u32, Ga>, Table)> {
    let mut o = GA_START;
    let mut by_handle = HashMap::new();
    let mut free = vec![];
    let mut last = 0;
    let mut prefix = [0x8080, 0x9080];
    for _ in 0..GA_COUNT {
        let (h, id) = (u32_at(d, o), u32_at(d, o + 4));
        if h == 0 {
            if id == u32::MAX {
                free.push(o);
            }
            o += 8;
        } else {
            last = last.max(h & 0xFFFF);
            match h >> 28 {
                0x8 | 0x9 => {
                    prefix[(h >> 28 == 0x9) as usize] = h >> 16;
                    by_handle.insert(h, Ga { off: o, id });
                    o += GA_EQUIP_LEN;
                }
                _ => o += 8,
            }
        }
        if o > d.len() {
            return None;
        }
    }
    (o + 0x34 == vigor).then_some((by_handle, Table { free, last_handle: last, prefix }))
}

/// Adds the item sections to `root` (leaves numbered from `first_leaf`). `None` if the
/// carried list isn't where it should be.
pub fn build(d: &[u8], vigor: usize, first_leaf: usize, root: &mut Map<String, Value>, editable: &mut HashMap<String, usize>) -> Option<Inventory> {
    let inv = vigor + INV_FROM_VIGOR;
    let c2 = inv + COUNTER2;
    if c2 + 4 > d.len() || !(0..CAP).all(|i| valid(entry(d, inv + i * ENTRY))) {
        return None;
    }
    let mut lists = vec![List { section: "Items", start: inv, count_at: inv - 4, wide_count: false, counter2: Some(c2) }];
    let t_at = c2 + TABLE_FROM_COUNTER2;
    let n = u32_at(d, t_at) as usize;
    let st = t_at + 4 + n * 8 + STORAGE_AFTER_TABLE;
    if n < 0x1000 && st + CAP * ENTRY <= d.len() && (0..CAP).all(|i| valid(entry(d, st + i * ENTRY))) {
        lists.push(List { section: "Storage", start: st, count_at: st - 4, wide_count: true, counter2: None });
    }
    let (ga, table) = match read_table(d, vigor) {
        Some((g, t)) => (g, Some(t)),
        None => (HashMap::new(), None),
    };
    let equipped: BTreeSet<u32> = (vigor + WORN_FROM_VIGOR..inv - 4).step_by(4).map(|o| u32_at(d, o)).filter(|v| matches!(v >> 28, 0x8..=0xB)).collect();
    let mut out = Inventory { lists, rows: vec![], first_leaf, table };
    let mut sections: Vec<(&str, Map<String, Value>)> = ["Items", "Storage", "Weapons", "Armor", "Rings"].iter().map(|s| (*s, Map::new())).collect();
    for (li, l) in out.lists.iter().enumerate() {
        for i in 0..CAP {
            let off = l.start + i * ENTRY;
            let e = entry(d, off);
            let offered = offered_kind(li, i);
            let kind = match (Kind::of(e[0]), offered) {
                (Some(k), _) if e[1] != u32::MAX => k,
                (None, Some(k)) if is_free(e) => k,
                _ => continue,
            };
            let handle = (e[0] != 0).then_some(e[0]);
            let g = handle.and_then(|h| ga.get(&h).copied());
            if matches!(kind, Kind::Weapon | Kind::Armor) && handle.is_some() && g.is_none_or(|g| g.id == UNARMED) {
                continue;
            }
            let (sec_i, key) = match kind {
                Kind::Good => (li, format!("Slot {}", i + 1)),
                k => (1 + k as usize, if li == 0 { format!("Slot {}", i + 1) } else { format!("Storage {}", i + 1) }),
            };
            let id = g.map_or(e[1], |g| g.id);
            let (name, level, infusion) = match (kind, handle) {
                (_, None) => (EMPTY.to_string(), 0, NO_AFFINITY.to_string()),
                (Kind::Weapon, Some(_)) => {
                    let base = base_of(id);
                    let inf = if infusable(base) { INFUSIONS.get(((id % 10_000) / 100) as usize).copied().unwrap_or(NO_AFFINITY) } else { NO_AFFINITY };
                    (names(kind).name(base, "Weapon"), if is_ammo(base) { 0 } else { id % 100 }, inf.to_string())
                }
                (k, Some(_)) => (names(k).name(id & 0x0FFF_FFFF, k.word()), 0, NO_AFFINITY.to_string()),
            };
            let base = format!("/{}/{key}", sections[sec_i].0);
            let mut slot = Map::new();
            let mut ptrs: [Option<String>; 4] = Default::default();
            slot.insert("Item".into(), Value::from(name.clone()));
            ptrs[0] = Some(format!("{base}/Item"));
            if matches!(kind, Kind::Good | Kind::Weapon) {
                slot.insert("Quantity".into(), Value::from(if handle.is_some() { e[2] } else { 0 }));
                ptrs[1] = Some(format!("{base}/Quantity"));
            }
            if kind == Kind::Weapon {
                slot.insert("Level".into(), Value::from(level));
                slot.insert("Infusion".into(), Value::from(infusion));
                ptrs[2] = Some(format!("{base}/Level"));
                ptrs[3] = Some(format!("{base}/Infusion"));
            }
            sections[sec_i].1.insert(key, Value::Object(slot));
            let r = out.rows.len();
            for (f, p) in ptrs.iter().enumerate() {
                if let Some(p) = p {
                    editable.insert(p.clone(), first_leaf + r * 4 + f);
                }
            }
            out.rows.push(Row { list: li, kind, off, ptrs, handle, orig_name: name, ga: g, equipped: handle.is_some_and(|h| equipped.contains(&h)), offered: offered == Some(kind) });
        }
    }
    for (s, m) in sections {
        if s != "Storage" || out.lists.len() > 1 {
            root.insert(s.into(), Value::Object(m));
        }
    }
    Some(out)
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

    fn id_of(kind: Kind, name: &str) -> Option<u32> {
        names(kind).by_name.get(name).copied()
    }

    /// The weapon base a row shows (by name, or the one it had).
    fn base_in(row: &Row, name: &str) -> Option<u32> {
        Self::id_of(Kind::Weapon, name).or_else(|| (name == row.orig_name).then(|| row.ga.map(|g| base_of(g.id))).flatten())
    }

    pub fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let (row, f) = self.row(leaf).ok_or("unknown value")?;
        let max_qty = if row.list == 0 { MAX_CARRIED } else { MAX_STORED };
        match f {
            1 => return int_in(v, 0, max_qty, "Quantity").map(|_| ()),
            2 => return int_in(v, 0, 10, "Level").map(|_| ()),
            3 => return Ok(()),
            _ => {}
        }
        let name = v.as_str().ok_or("pick an item from the list")?;
        let what = row.kind.word().to_lowercase();
        if name != row.orig_name && name != EMPTY && Self::id_of(row.kind, name).is_none() {
            return Err(format!("\u{201C}{name}\u{201D} isn't a Dark Souls III {what}"));
        }
        if row.equipped && name != row.orig_name {
            return Err(format!("this {what} is equipped \u{2014} take it off in the game first"));
        }
        if name == EMPTY && row.handle.is_some() && !row.offered {
            return Err(format!("this {what} can't be removed here \u{2014} drop or sell it in the game"));
        }
        if name != EMPTY && row.handle.is_none() && matches!(row.kind, Kind::Weapon | Kind::Armor) && self.table.is_none() {
            return Err("new weapons and armor can't be added to this save".into());
        }
        Ok(())
    }

    pub fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Some((row, _)) = self.row(leaf) else { return vec![] };
        let name = Self::get(row, view, 0).and_then(Value::as_str).unwrap_or(EMPTY);
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
        match row.kind {
            Kind::Good => set(1, Value::from(if empty { 0 } else { qty.max(1) })),
            Kind::Weapon => {
                let base = if empty { None } else { Self::base_in(row, name) };
                let ammo = base.is_some_and(is_ammo);
                set(1, Value::from(if empty { 0 } else if ammo { qty.clamp(1, 99) } else { 1 }));
                let has = if name == row.orig_name { row.ga.map_or(0, |g| g.id % 100) } else { 0 };
                let max = base.map_or(0, |b| max_level(b, has)) as u64;
                let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0);
                set(2, Value::from(level.min(max)));
                let allowed = infusion_names(base);
                let inf = Self::get(row, view, 3).and_then(Value::as_str).unwrap_or(NO_AFFINITY);
                if !allowed.iter().any(|a| a == inf) {
                    set(3, Value::from(allowed[0].clone()));
                }
            }
            _ => {}
        }
        out
    }

    pub fn choices(&self, leaf: usize, view: &Value) -> Option<Arc<Vec<String>>> {
        let (row, f) = self.row(leaf)?;
        match f {
            0 => {}
            3 => {
                let name = Self::get(row, view, 0).and_then(Value::as_str).unwrap_or(EMPTY);
                return Some(Arc::new(infusion_names(if name == EMPTY { None } else { Self::base_in(row, name) })));
            }
            _ => return None,
        }
        let taken: BTreeSet<&str> = if matches!(row.kind, Kind::Good | Kind::Ring) {
            self.rows
                .iter()
                .filter(|r| r.list == row.list && r.kind == row.kind && r.ptrs[0] != row.ptrs[0])
                .filter_map(|r| Self::get(r, view, 0).and_then(Value::as_str))
                .filter(|n| *n != EMPTY)
                .collect()
        } else {
            BTreeSet::new()
        };
        let mut all: Vec<String> = names(row.kind).all.iter().filter(|n| !taken.contains(n.as_str())).cloned().collect();
        if row.handle.is_some() && !row.offered {
            all.retain(|n| n != EMPTY);
        }
        if row.orig_name != EMPTY && Self::id_of(row.kind, &row.orig_name).is_none() {
            all.insert(0, row.orig_name.clone());
        }
        Some(Arc::new(all))
    }

    pub fn write(&self, out: &mut Vec<u8>, view: &Value, changes: &[(usize, Value)]) -> Result<(), String> {
        let touched: BTreeSet<usize> = changes.iter().filter_map(|(l, _)| l.checked_sub(self.first_leaf).map(|i| i / 4)).filter(|&r| r < self.rows.len()).collect();
        if touched.is_empty() {
            return Ok(());
        }
        // Next index per list: one past the highest running number in use.
        let mut next_idx: Vec<u32> = self.lists.iter().map(|l| (0..CAP).map(|i| u32_at(out, l.start + i * ENTRY + 12) & 0xFFF).max().unwrap_or(0) + 1).collect();
        let mut delta = vec![0i64; self.lists.len()];
        let mut added: Vec<(usize, Kind, u32)> = vec![];
        let mut table_ops: Vec<(usize, usize, Vec<u8>)> = vec![];
        let mut free = self.table.as_ref().map(|t| t.free.iter()).into_iter().flatten();
        let mut last_handle = self.table.as_ref().map_or(0, |t| t.last_handle);
        for r in touched {
            let row = &self.rows[r];
            let name = Self::get(row, view, 0).and_then(Value::as_str).unwrap_or(EMPTY);
            let qty = Self::get(row, view, 1).and_then(Value::as_u64).unwrap_or(1) as u32;
            let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0) as u32;
            let inf = Self::get(row, view, 3).and_then(Value::as_str).unwrap_or(NO_AFFINITY);
            let inf_n = INFUSIONS.iter().position(|a| *a == inf).unwrap_or(0) as u32;
            let what = row.kind.word().to_lowercase();
            let id = if name == EMPTY {
                None
            } else if name == row.orig_name && row.handle.is_some() {
                Some(match (row.kind, row.ga) {
                    (Kind::Weapon, Some(g)) => base_of(g.id),
                    (_, Some(g)) => g.id & 0x0FFF_FFFF,
                    _ => u32_at(out, row.off + 4) & 0x0FFF_FFFF,
                })
            } else {
                Some(Self::id_of(row.kind, name).ok_or_else(|| format!("\u{201C}{name}\u{201D} isn't a Dark Souls III {what}"))?)
            };
            // The full item id, as the list and the item table hold it.
            let full = id.map(|id| match row.kind {
                Kind::Weapon if is_ammo(id) => id,
                Kind::Weapon => id + inf_n * 100 + level,
                k => k.id_bits() | id,
            });
            let o = row.off;
            match (row.handle, full) {
                (None, None) => {}
                (Some(_), None) => {
                    out[o..o + ENTRY].fill(0);
                    put(out, o + 4, u32::MAX);
                    delta[row.list] -= 1;
                    if let Some(g) = row.ga {
                        let mut empty = vec![0u8; 8];
                        empty[4..].copy_from_slice(&u32::MAX.to_le_bytes());
                        table_ops.push((g.off, GA_EQUIP_LEN, empty));
                    }
                }
                (None, Some(fid)) => {
                    let handle = match row.kind {
                        Kind::Good => 0xB000_0000 | (fid & 0x00FF_FFFF),
                        Kind::Ring => 0xA000_0000 | (fid & 0x00FF_FFFF),
                        k => {
                            let t = self.table.as_ref().ok_or("new weapons and armor can't be added to this save")?;
                            let at = *free.next().ok_or("the item table is full")?;
                            last_handle += 1;
                            let h = (t.prefix[(k == Kind::Armor) as usize] << 16) | (last_handle & 0xFFFF);
                            let mut e = Vec::with_capacity(GA_EQUIP_LEN);
                            e.extend(h.to_le_bytes());
                            e.extend(fid.to_le_bytes());
                            e.extend(if k == Kind::Weapon { WEAPON_TAIL } else { ARMOR_TAIL });
                            table_ops.push((at, 8, e));
                            h
                        }
                    };
                    added.push((row.list, row.kind, handle));
                    put(out, o, handle);
                    put(out, o + 4, fid);
                    put(out, o + 8, if matches!(row.kind, Kind::Good | Kind::Weapon) { qty.max(1) } else { 1 });
                    put(out, o + 12, (INDEX_HIGH[row.kind as usize] << 16) | (next_idx[row.list] & 0xFFF));
                    next_idx[row.list] += 1;
                    delta[row.list] += 1;
                }
                (Some(h), Some(fid)) => {
                    match (row.kind, row.ga) {
                        (Kind::Weapon | Kind::Armor, Some(g)) => put(out, g.off + 4, fid),
                        (Kind::Good | Kind::Ring, _) => {
                            let nh = if row.kind == Kind::Good { 0xB000_0000 } else { 0xA000_0000 } | (fid & 0x00FF_FFFF);
                            if nh != h {
                                added.push((row.list, row.kind, nh));
                                put(out, o, nh);
                            }
                        }
                        _ => return Err(format!("this {what} isn't in the item table")),
                    }
                    put(out, o + 4, fid);
                    if matches!(row.kind, Kind::Good | Kind::Weapon) {
                        put(out, o + 8, qty.max(1));
                    }
                }
            }
        }
        for (i, l) in self.lists.iter().enumerate() {
            for &(_, kind, h) in added.iter().filter(|(li, k, _)| *li == i && matches!(k, Kind::Good | Kind::Ring)) {
                if (0..CAP).filter(|k| u32_at(out, l.start + k * ENTRY) == h).count() > 1 {
                    return Err(format!("{} already has {} \u{2014} change its quantity instead", l.section, names(kind).name(h & 0x00FF_FFFF, kind.word())));
                }
            }
            if delta[i] != 0 {
                if l.wide_count {
                    let n = (u32_at(out, l.count_at) as i64 + delta[i]) as u32;
                    put(out, l.count_at, n);
                } else {
                    let n = (u16_at(out, l.count_at) as i64 + delta[i]) as u16;
                    out[l.count_at..l.count_at + 2].copy_from_slice(&n.to_le_bytes());
                }
            }
            let adds = added.iter().filter(|a| a.0 == i).count() as u16;
            if let (Some(c), true) = (l.counter2, adds > 0) {
                let n = u16_at(out, c).wrapping_add(adds);
                out[c..c + 2].copy_from_slice(&n.to_le_bytes());
            }
        }
        if !table_ops.is_empty() {
            let len = out.len();
            let grow: i64 = table_ops.iter().map(|(_, old, new)| new.len() as i64 - *old as i64).sum();
            if grow > MAX_GROWTH as i64 {
                return Err("that's too many new weapons and armor at once \u{2014} save, then add more".into());
            }
            table_ops.sort_by(|a, b| b.0.cmp(&a.0));
            for (at, old, new) in table_ops {
                out.splice(at..at + old, new);
            }
            // Keep the size: take (or give back) filler just before the slot's last bytes.
            let keep: Vec<u8> = out[out.len() - KEEP_AT_END..].to_vec();
            out.truncate(out.len() - KEEP_AT_END);
            out.resize(len - KEEP_AT_END, 0);
            out.extend(keep);
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// An item table of unused entries with `weapons` (handle, id) first; returns where
    /// Vigor goes. The carried list after it is empty.
    pub fn with_table(d: &mut [u8], weapons: &[(u32, u32)]) -> usize {
        let mut o = GA_START;
        for &(h, id) in weapons {
            put(d, o, h);
            put(d, o + 4, id);
            d[o + 8..o + GA_EQUIP_LEN].copy_from_slice(&WEAPON_TAIL);
            o += GA_EQUIP_LEN;
        }
        for _ in weapons.len()..GA_COUNT {
            put(d, o, 0);
            put(d, o + 4, u32::MAX);
            o += 8;
        }
        o + 0x34
    }

    /// Puts an entry in the carried list (0) or storage (1) at `slot`.
    pub fn add(d: &mut [u8], vigor: usize, list: usize, slot: usize, handle: u32, id: u32, qty: u32) {
        let inv = vigor + INV_FROM_VIGOR;
        let (start, count_at) = if list == 0 {
            (inv, inv - 4)
        } else {
            let t = inv + COUNTER2 + TABLE_FROM_COUNTER2;
            let st = t + 4 + u32_at(d, t) as usize * 8 + STORAGE_AFTER_TABLE;
            (st, st - 4)
        };
        let o = start + slot * ENTRY;
        put(d, o, handle);
        put(d, o + 4, id);
        put(d, o + 8, qty);
        put(d, o + 12, 0x80 + slot as u32);
        put(d, count_at, u32_at(d, count_at) + 1);
    }

    pub fn wear(d: &mut [u8], vigor: usize, handle: u32) {
        put(d, vigor + WORN_FROM_VIGOR + 4, handle);
    }
}
