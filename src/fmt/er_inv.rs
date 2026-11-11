//! Elden Ring's item lists: what the character carries, the key items, and the storage box.
//!
//! Each list is a fixed array of 12-byte entries (handle, quantity, acquisition order),
//! after a count of the used ones; carried and key items share the two counters that
//! follow them (next equip index, next acquisition order), and storage has its own. The
//! handle's top four bits give the kind: 0xB… goods, 0xA… talismans (both are the item id
//! itself), and 0x8… weapons, 0x9… armor, 0xC… Ashes of War, which point into the slot's
//! item table ("gaitem" map, 0x1400 entries at 0x20). There a weapon takes 21 bytes
//! (handle, item id = base + affinity × 100 + level, two values, its Ash of War, a byte),
//! armor 16 and anything else 8, so adding a weapon or armor makes the table longer and
//! moves everything after it; the slot keeps its size by giving up unused bytes at its end
//! (past the last section, which editors write as zeros). An unused list entry is all
//! zeros; an unused table entry is handle 0, id 0xFFFFFFFF.
//!
//! An Ash of War's table entry is 8 bytes (handle 0xC…, id 0x8000_0000 + its id); a weapon
//! with one fitted points to it, so a fitted ash isn't touched.
//!
//! Sites of Grace and bosses are bits in the event flags, which follow the items ever
//! acquired and the tutorial data (see `walk`).
//!
//! The carried list sits a fixed distance after the attributes; storage follows the
//! equipment, the variable-length ammunition list and the face data. Checked against real
//! saves, ClayAmore's ER-Save-Editor and alfizari's Elden-Ring-Save-Editor.

use super::er_equip::{ARMOR, ASHES, TALISMANS, WEAPONS};
use super::er_flags::{BASE_GRACES, BOSSES, GRACES};
use super::er_items::GOODS;
use super::sekiro_inv::EMPTY;
use crate::tree::int_in;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

const ENTRY: usize = 12;
const CARRIED_CAP: usize = 0xA80;
const KEY_CAP: usize = 0x180;
const STORAGE_CAP: usize = 0x780;
const STORAGE_KEY_CAP: usize = 0x80;
/// From Vigor to the carried items' count.
const INV_FROM_VIGOR: usize = 0x370;
/// From Vigor to the equipment the character wears (equip data and two copies of it).
const WORN_FROM_VIGOR: usize = 0x24C;
const WORN_LEN: usize = 0x58 + 0x74 + 0x58;
/// From the end of the carried lists: equipped spells, equipped items (quick slots and
/// pouch), gestures, then the ammunition count.
const SPELLS_LEN: usize = 0x74;
const EQUIP_ITEMS_LEN: usize = 0x8C;
const GESTURES_LEN: usize = 0x18;
/// After the ammunition: equipped items again, physick, a value and the face data.
const EQUIPPED_LEN: usize = 0x9C;
const AFTER_EQUIPPED: usize = 8 + 4 + 0x12F;
const GA_START: usize = 0x20;
const GA_COUNT: usize = 0x1400;
const GOOD: u32 = 0xB000_0000;
/// Bare fists, kept as a weapon.
const UNARMED: u32 = 110_000;
const TALISMAN: u32 = 0xA000_0000;
const ARMOR_ID: u32 = 0x1000_0000;
const ASH_ID: u32 = 0x8000_0000;
/// Maximum carried or stored of any good.
pub const MAX_QTY: i128 = 999;
pub const AFFINITIES: [&str; 13] = [
    "Standard",
    "Heavy",
    "Keen",
    "Quality",
    "Fire",
    "Flame Art",
    "Lightning",
    "Sacred",
    "Magic",
    "Cold",
    "Poison",
    "Blood",
    "Occult",
];
/// Shown as the affinity of weapons that can't have one.
pub const NO_AFFINITY: &str = "\u{2014}";

fn u32_at(d: &[u8], o: usize) -> u32 {
    d.get(o..o + 4)
        .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn put(d: &mut [u8], o: usize, v: u32) {
    d[o..o + 4].copy_from_slice(&v.to_le_bytes());
}

fn entry(d: &[u8], o: usize) -> [u32; 3] {
    [u32_at(d, o), u32_at(d, o + 4), u32_at(d, o + 8)]
}

/// Used entries in a list, or `None` if one doesn't look like an entry.
fn used(d: &[u8], at: usize, cap: usize) -> Option<usize> {
    let mut n = 0;
    for i in 0..cap {
        let e = entry(d, at + i * ENTRY);
        match e[0] >> 28 {
            0 if e == [0; 3] => {}
            0x8..=0xC => n += 1,
            _ => return None,
        }
    }
    Some(n)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Good,
    Weapon,
    Armor,
    Talisman,
    Ash,
}

impl Kind {
    fn of(handle: u32) -> Option<Kind> {
        Some(match handle >> 28 {
            0xB => Kind::Good,
            0x8 => Kind::Weapon,
            0x9 => Kind::Armor,
            0xA => Kind::Talisman,
            0xC => Kind::Ash,
            _ => return None,
        })
    }
}

/// Names by id and ids by name for one kind; names used twice get their id added.
pub(super) struct Names {
    pub(super) by_id: HashMap<u32, String>,
    pub(super) by_name: HashMap<String, u32>,
    /// "(empty)" first, then every name, sorted.
    pub(super) all: Arc<Vec<String>>,
}

impl Names {
    pub(super) fn new<'a>(items: impl Iterator<Item = (u32, &'a str)> + Clone) -> Names {
        let mut count: HashMap<&str, usize> = HashMap::new();
        for (_, n) in items.clone() {
            *count.entry(n).or_default() += 1;
        }
        let mut by_id = HashMap::new();
        let mut by_name = HashMap::new();
        for (id, n) in items {
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
    }

    pub(super) fn name(&self, id: u32, what: &str) -> String {
        self.by_id
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("{what} #{id}"))
    }
}

fn names(kind: Kind) -> &'static Names {
    static N: OnceLock<[Names; 5]> = OnceLock::new();
    let n = N.get_or_init(|| {
        [
            Names::new(GOODS.iter().map(|&(i, n)| (i, n))),
            Names::new(
                WEAPONS
                    .iter()
                    .filter(|w| w.0 != UNARMED)
                    .map(|&(i, n, _, _)| (i, n)),
            ),
            Names::new(ARMOR.iter().map(|&(i, n)| (i, n))),
            Names::new(TALISMANS.iter().map(|&(i, n)| (i, n))),
            Names::new(ASHES.iter().map(|&(i, n)| (i, n))),
        ]
    });
    &n[kind as usize]
}

fn kind_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Good => "Item",
        Kind::Weapon => "Weapon",
        Kind::Armor => "Armor",
        Kind::Talisman => "Talisman",
        Kind::Ash => "Ash of War",
    }
}

/// The id of a good by the name the editor shows for it.
#[cfg(test)]
pub fn good_id(name: &str) -> Option<u32> {
    names(Kind::Good).by_name.get(name).copied()
}

/// A weapon's highest upgrade level and affinity mask, if known.
fn weapon_info(base: u32) -> Option<(Option<u8>, u16)> {
    static W: OnceLock<HashMap<u32, (Option<u8>, u16)>> = OnceLock::new();
    W.get_or_init(|| WEAPONS.iter().map(|&(i, _, m, a)| (i, (m, a))).collect())
        .get(&base)
        .copied()
}

fn is_ammo(base: u32) -> bool {
    base >= 50_000_000
}

/// The affinities a weapon can have, by name ("—" alone when it can't have one).
fn affinity_names(base: Option<u32>) -> Vec<String> {
    let mask = base.and_then(weapon_info).map_or(0, |(_, m)| m);
    let v: Vec<String> = AFFINITIES
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, a)| a.to_string())
        .collect();
    if v.is_empty() {
        vec![NO_AFFINITY.into()]
    } else {
        v
    }
}

#[derive(Clone, Debug)]
struct List {
    section: &'static str,
    start: usize,
    cap: usize,
    count_at: usize,
    /// Next equip index, then next acquisition order (shared by carried and key items).
    next_at: usize,
}

/// An entry of the item table.
#[derive(Clone, Copy, Debug)]
struct Ga {
    off: usize,
    len: usize,
    id: u32,
    /// The weapon's own Ash of War (0 or 0xFFFFFFFF for its default skill).
    aow: u32,
}

#[derive(Clone, Debug)]
struct Row {
    list: usize,
    kind: Kind,
    off: usize,
    /// Pointers: item, quantity, level, affinity (the last three only where the kind has them).
    ptrs: [Option<String>; 4],
    /// Handle when the file was read.
    handle: Option<u32>,
    orig_name: String,
    ga: Option<Ga>,
    /// Worn, held, or in a quick slot or the pouch.
    equipped: bool,
    /// One of the empty entries reserved for new items of this kind (see `OFFERED`).
    offered: bool,
}

/// Item table facts needed to add weapons and armor.
#[derive(Clone, Debug)]
struct Table {
    /// Unused entries after the first weapon (where the game puts new ones).
    free: Vec<usize>,
    /// Highest handle number in use (the low 16 bits are shared by every kind).
    last_handle: u32,
    /// Bytes that can be given up at the end of the slot.
    spare: usize,
}

/// One grace or boss: its pointer in the view, the byte in the slot and the bit.
#[derive(Clone, Debug)]
struct Flag {
    ptr: String,
    at: usize,
    bit: u8,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    lists: Vec<List>,
    rows: Vec<Row>,
    first_leaf: usize,
    table: Option<Table>,
    /// Leaves after the rows' (four per row).
    flags: Vec<Flag>,
}

/// Unused entries reserved for each kind at the end of each list (carried, key, storage),
/// counted back from the end: goods, weapons, armor, talismans, ashes. They're always the same
/// entries, so a list reads back the same after saving. Lists are nowhere near full in
/// sample saves. Items elsewhere can't be removed here (they'd drop out of the list).
const OFFERED: [[usize; 5]; 3] = [[64, 32, 32, 16, 16], [16, 0, 0, 0, 0], [32, 0, 0, 0, 0]];

fn offered_kind(list: usize, cap: usize, i: usize) -> Option<Kind> {
    let mut end = cap;
    for (k, &n) in [
        Kind::Good,
        Kind::Weapon,
        Kind::Armor,
        Kind::Talisman,
        Kind::Ash,
    ]
    .iter()
    .zip(&OFFERED[list])
    {
        if i < end && i + n >= end {
            return Some(*k);
        }
        end -= n;
    }
    None
}

struct Located {
    lists: Vec<List>,
    /// Handles and ids of what's worn, held and in quick slots.
    equipped: BTreeSet<u32>,
    /// End of the last known section.
    used_end: Option<usize>,
    /// Start of the event flags.
    flags_at: Option<usize>,
}

/// Where the lists are, from the offset of Vigor; `None` unless the counts match.
fn locate(d: &[u8], vigor: usize) -> Option<Located> {
    let inv = vigor + INV_FROM_VIGOR;
    let key = inv + 4 + CARRIED_CAP * ENTRY;
    let next = key + 4 + KEY_CAP * ENTRY;
    let end = next + 8;
    if end > d.len()
        || used(d, inv + 4, CARRIED_CAP)? != u32_at(d, inv) as usize
        || used(d, key + 4, KEY_CAP)? != u32_at(d, key) as usize
    {
        return None;
    }
    let mut lists = vec![
        List {
            section: "Items",
            start: inv + 4,
            cap: CARRIED_CAP,
            count_at: inv,
            next_at: next,
        },
        List {
            section: "Key items",
            start: key + 4,
            cap: KEY_CAP,
            count_at: key,
            next_at: next,
        },
    ];
    let equip_items = end + SPELLS_LEN;
    let ammo = equip_items + EQUIP_ITEMS_LEN + GESTURES_LEN;
    let n_ammo = u32_at(d, ammo) as usize;
    let equipped_at = ammo + 4 + 8 * n_ammo;
    let mut equipped: BTreeSet<u32> = BTreeSet::new();
    for (at, len) in [
        (vigor + WORN_FROM_VIGOR, WORN_LEN),
        (equip_items, EQUIP_ITEMS_LEN),
        (equipped_at, EQUIPPED_LEN),
    ] {
        for i in 0..len / 4 {
            let v = u32_at(d, at + i * 4);
            if matches!(v >> 28, 0x4 | 0x8 | 0x9 | 0xA | 0xB) {
                equipped.insert(v);
                // Goods may be referred to by id (0x4…) rather than handle.
                if v >> 28 == 0x4 {
                    equipped.insert(GOOD | (v & 0x0FFF_FFFF));
                }
            }
        }
    }
    let st = equipped_at + EQUIPPED_LEN + AFTER_EQUIPPED;
    let st_key = st + 4 + STORAGE_CAP * ENTRY;
    let st_next = st_key + 4 + STORAGE_KEY_CAP * ENTRY;
    let mut used_end = None;
    let mut flags_at = None;
    if n_ammo < 0x1000
        && st_next + 8 <= d.len()
        && used(d, st + 4, STORAGE_CAP) == Some(u32_at(d, st) as usize)
        && used(d, st_key + 4, STORAGE_KEY_CAP) == Some(u32_at(d, st_key) as usize)
    {
        lists.push(List {
            section: "Storage",
            start: st + 4,
            cap: STORAGE_CAP,
            count_at: st,
            next_at: st_next,
        });
        if let Some((f, e)) = walk(d, st_next + 8) {
            flags_at = Some(f);
            used_end = Some(e);
        }
    }
    Some(Located {
        lists,
        equipped,
        used_end,
        flags_at,
    })
}

/// From the end of storage to the end of the last known section, checking the one marker
/// on the way (a value the game always writes as 2 on a used slot). Returns where the event
/// flags start and where the sections end.
fn walk(d: &[u8], mut o: usize) -> Option<(usize, usize)> {
    o += 0x40 * 4; // gestures
    o += 4 + 4 * u32_at(d, o) as usize; // unlocked regions
    o += 0x28 + 1 + 0x40 + 12 + 0x1008 + 0x34; // horse, menu and trophy data
    o += 8 + 0x1B58 * 16; // items ever acquired
    o += 0x408 + 0x1D; // tutorial data, deaths, online state, play time
    let flags = o;
    o += 0x1B_F99F + 1; // event flags
    for _ in 0..5 {
        o += 4 + u32_at(d, o) as usize;
        if o > d.len() {
            return None;
        }
    }
    o += 0x3D + 0xF; // position
    if u32_at(d, o) != 2 {
        return None;
    }
    o += 4 + 0x2_0000 + 12 + 12 + 0x10 + 8 + 0x20 + 0x32 + 0x80;
    (o <= d.len()).then_some((flags, o))
}

/// The item table, if it ends where the character data begins.
fn read_table(d: &[u8], vigor: usize) -> Option<(HashMap<u32, Ga>, Vec<usize>, usize, u32)> {
    let mut o = GA_START;
    let mut by_handle = HashMap::new();
    let mut empties = vec![];
    let mut first_weapon = None;
    let mut last = 0;
    for _ in 0..GA_COUNT {
        let (h, id) = (u32_at(d, o), u32_at(d, o + 4));
        let len = match h >> 28 {
            _ if h == 0 => 8,
            0x8 => 21,
            0x9 => 16,
            _ => 8,
        };
        if h == 0 {
            if id == u32::MAX {
                empties.push(o);
            }
        } else {
            last = last.max(h & 0xFFFF);
            if h >> 28 == 0x8 && first_weapon.is_none() {
                first_weapon = Some(o);
            }
            let aow = if len == 21 { u32_at(d, o + 16) } else { 0 };
            by_handle.insert(
                h,
                Ga {
                    off: o,
                    len,
                    id,
                    aow,
                },
            );
        }
        o += len;
        if o > d.len() {
            return None;
        }
    }
    (o + 0x34 == vigor).then(|| {
        let after = first_weapon.unwrap_or(GA_START);
        (
            by_handle,
            empties.into_iter().filter(|&e| e > after).collect(),
            o,
            last,
        )
    })
}

/// Adds the item sections to `root` (leaves numbered from `first_leaf`).
pub fn build(
    d: &[u8],
    vigor: usize,
    first_leaf: usize,
    root: &mut Map<String, Value>,
    editable: &mut HashMap<String, usize>,
) -> Option<Inventory> {
    let loc = locate(d, vigor)?;
    let table = read_table(d, vigor);
    let mut inv = Inventory {
        lists: loc.lists.clone(),
        rows: vec![],
        first_leaf,
        table: match (&table, loc.used_end) {
            (Some((_, free, _, last)), Some(end)) => Some(Table {
                free: free.clone(),
                last_handle: *last,
                spare: d.len() - end,
            }),
            _ => None,
        },
        flags: vec![],
    };
    let ga = table.map(|t| t.0).unwrap_or_default();
    let mut loc = loc;
    // An ash fitted to a weapon stays put.
    loc.equipped.extend(
        ga.values()
            .filter(|g| g.len == 21 && g.aow != 0 && g.aow != u32::MAX)
            .map(|g| g.aow),
    );
    let mut sections: Vec<(&str, Map<String, Value>)> = [
        "Items",
        "Key items",
        "Storage",
        "Weapons",
        "Armor",
        "Talismans",
        "Ashes of War",
    ]
    .iter()
    .map(|s| (*s, Map::new()))
    .collect();
    for (li, l) in inv.lists.iter().enumerate() {
        for i in 0..l.cap {
            let off = l.start + i * ENTRY;
            let e = entry(d, off);
            let offered = offered_kind(li, l.cap, i);
            let kind = match (Kind::of(e[0]), offered) {
                (Some(k), _) => k,
                (None, Some(k)) if e == [0; 3] => k,
                _ => continue,
            };
            // Goods keep their lists; equipment from every list is gathered by kind.
            let (sec_i, key) = match kind {
                Kind::Good => (li, format!("Slot {}", i + 1)),
                k => {
                    let s = 2 + k as usize;
                    (
                        s,
                        if li == 0 {
                            format!("Slot {}", i + 1)
                        } else {
                            format!("Storage {}", i + 1)
                        },
                    )
                }
            };
            if kind != Kind::Good && li == 1 {
                continue; // equipment isn't kept with key items
            }
            let handle = (e[0] != 0).then_some(e[0]);
            let g = handle.and_then(|h| ga.get(&h).copied());
            if matches!(kind, Kind::Weapon | Kind::Armor | Kind::Ash)
                && handle.is_some()
                && g.is_none_or(|g| g.id == UNARMED)
            {
                continue; // not in the item table, or bare fists: leave it alone
            }
            let (name, level, affinity) = match (kind, handle) {
                (_, None) => (EMPTY.to_string(), 0, NO_AFFINITY.to_string()),
                (Kind::Good, Some(h)) | (Kind::Talisman, Some(h)) => (
                    names(kind).name(h & 0x0FFF_FFFF, kind_word(kind)),
                    0,
                    NO_AFFINITY.to_string(),
                ),
                (Kind::Armor | Kind::Ash, Some(_)) => (
                    names(kind).name(g.unwrap().id & 0x0FFF_FFFF, kind_word(kind)),
                    0,
                    NO_AFFINITY.to_string(),
                ),
                (Kind::Weapon, Some(_)) => {
                    let id = g.unwrap().id;
                    let base = id - id % 10_000;
                    let aff = ((id / 100) % 100) as usize;
                    let affinity = if weapon_info(base).is_some_and(|(_, m)| m != 0) {
                        AFFINITIES.get(aff).map_or(NO_AFFINITY, |a| *a)
                    } else {
                        NO_AFFINITY
                    };
                    (
                        names(kind).name(base, "Weapon"),
                        id % 100,
                        affinity.to_string(),
                    )
                }
            };
            let base = format!("/{}/{key}", sections[sec_i].0);
            let mut slot = Map::new();
            let mut ptrs: [Option<String>; 4] = Default::default();
            slot.insert("Item".into(), Value::from(name.clone()));
            ptrs[0] = Some(format!("{base}/Item"));
            if matches!(kind, Kind::Good | Kind::Weapon) {
                slot.insert(
                    "Quantity".into(),
                    Value::from(if handle.is_some() { e[1] } else { 0 }),
                );
                ptrs[1] = Some(format!("{base}/Quantity"));
            }
            if kind == Kind::Weapon {
                slot.insert("Level".into(), Value::from(level));
                slot.insert("Affinity".into(), Value::from(affinity));
                ptrs[2] = Some(format!("{base}/Level"));
                ptrs[3] = Some(format!("{base}/Affinity"));
            }
            sections[sec_i].1.insert(key, Value::Object(slot));
            let r = inv.rows.len();
            for (f, p) in ptrs.iter().enumerate() {
                if let Some(p) = p {
                    editable.insert(p.clone(), first_leaf + r * 4 + f);
                }
            }
            inv.rows.push(Row {
                list: li,
                kind,
                off,
                ptrs,
                handle,
                orig_name: name,
                ga: g,
                equipped: handle.is_some_and(|h| loc.equipped.contains(&h)),
                offered: offered == Some(kind),
            });
        }
    }
    for (s, m) in sections {
        if s != "Storage" || inv.lists.len() > 2 {
            root.insert(s.into(), Value::Object(m));
        }
    }
    if let Some(at) = loc.flags_at {
        let leaf0 = first_leaf + inv.rows.len() * 4;
        for (section, list) in [("Sites of Grace", GRACES), ("Bosses defeated", BOSSES)] {
            let mut m = Map::new();
            for &(name, off, bit) in list {
                let o = at + off as usize;
                let Some(&b) = d.get(o) else { continue };
                let ptr = format!("/{section}/{}", name.replace('~', "~0").replace('/', "~1"));
                m.insert(name.to_string(), Value::Bool(b & (1 << bit) != 0));
                editable.insert(ptr.clone(), leaf0 + inv.flags.len());
                inv.flags.push(Flag { ptr, at: o, bit });
            }
            root.insert(section.into(), Value::Object(m));
        }
    }
    Some(inv)
}

/// The Shadow of the Erdtree graces, by name (the rest are the base game's).
pub fn dlc_graces() -> impl Iterator<Item = &'static str> {
    GRACES[BASE_GRACES..].iter().map(|g| g.0)
}

impl Inventory {
    /// (row, field: 0 item, 1 quantity, 2 level, 3 affinity) for one of these leaves.
    fn row(&self, leaf: usize) -> Option<(&Row, usize)> {
        let i = leaf.checked_sub(self.first_leaf)?;
        self.rows.get(i / 4).map(|r| (r, i % 4))
    }

    fn flag(&self, leaf: usize) -> Option<&Flag> {
        let i = leaf.checked_sub(self.first_leaf + self.rows.len() * 4)?;
        self.flags.get(i)
    }

    pub fn owns(&self, leaf: usize) -> bool {
        self.row(leaf).is_some() || self.flag(leaf).is_some()
    }

    fn get<'a>(row: &Row, view: &'a Value, f: usize) -> Option<&'a Value> {
        row.ptrs[f].as_ref().and_then(|p| view.pointer(p))
    }

    fn id_of(kind: Kind, name: &str) -> Option<u32> {
        names(kind).by_name.get(name).copied()
    }

    pub fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        if self.flag(leaf).is_some() {
            return v
                .as_bool()
                .map(|_| ())
                .ok_or_else(|| "tick or untick it".to_string());
        }
        let (row, f) = self.row(leaf).ok_or("unknown value")?;
        match f {
            1 => return int_in(v, 0, MAX_QTY, "Quantity").map(|_| ()),
            2 => return int_in(v, 0, 25, "Level").map(|_| ()),
            3 => {
                let had = row.ga.map(|g| {
                    AFFINITIES
                        .get(((g.id / 100) % 100) as usize)
                        .copied()
                        .unwrap_or(NO_AFFINITY)
                });
                if row.ga.is_some_and(|g| g.aow != 0 && g.aow != u32::MAX) && v.as_str() != had {
                    return Err("this weapon has an Ash of War on it, which sets its affinity \u{2014} change it in the game".into());
                }
                return Ok(());
            }
            _ => {}
        }
        let name = v.as_str().ok_or("pick an item from the list")?;
        let what = kind_word(row.kind).to_lowercase();
        if name != row.orig_name && name != EMPTY && Self::id_of(row.kind, name).is_none() {
            return Err(format!("\u{201C}{name}\u{201D} isn't an Elden Ring {what}"));
        }
        if row.equipped && name != row.orig_name {
            return Err(format!(
                "this {what} is equipped or in a quick slot \u{2014} take it off in the game first"
            ));
        }
        if row.kind == Kind::Weapon
            && name != row.orig_name
            && row.ga.is_some_and(|g| g.aow != 0 && g.aow != u32::MAX)
        {
            return Err(
                "this weapon has an Ash of War on it \u{2014} take it off in the game first".into(),
            );
        }
        if name == EMPTY && row.handle.is_some() && !row.offered {
            return Err(format!(
                "this {what} can't be removed here \u{2014} drop or sell it in the game"
            ));
        }
        if name != EMPTY
            && row.handle.is_none()
            && matches!(row.kind, Kind::Weapon | Kind::Armor | Kind::Ash)
            && self.table.is_none()
        {
            return Err("new weapons, armor and Ashes of War can't be added to this save".into());
        }
        Ok(())
    }

    /// Keeps a row consistent: an empty slot holds nothing; a weapon's level and affinity
    /// stay within what it can have.
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
        match row.kind {
            Kind::Good => set(1, Value::from(if empty { 0 } else { qty.max(1) })),
            Kind::Weapon => {
                let base = Self::id_of(Kind::Weapon, name).or_else(|| {
                    (name == row.orig_name)
                        .then(|| row.ga.map(|g| g.id - g.id % 10_000))
                        .flatten()
                });
                let ammo = base.is_some_and(is_ammo);
                set(
                    1,
                    Value::from(if empty {
                        0
                    } else if ammo {
                        qty.clamp(1, 999)
                    } else {
                        1
                    }),
                );
                let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0);
                let max = match base.and_then(weapon_info) {
                    Some((Some(m), _)) => m as u64,
                    // Unknown: keep the level it had.
                    _ if name == row.orig_name => row.ga.map_or(0, |g| (g.id % 100) as u64),
                    _ => 0,
                };
                set(2, Value::from(if empty { 0 } else { level.min(max) }));
                let allowed = affinity_names(if empty { None } else { base });
                let aff = Self::get(row, view, 3)
                    .and_then(Value::as_str)
                    .unwrap_or(NO_AFFINITY);
                if !allowed.iter().any(|a| a == aff) {
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
                let name = Self::get(row, view, 0)
                    .and_then(Value::as_str)
                    .unwrap_or(EMPTY);
                let base = Self::id_of(Kind::Weapon, name).or_else(|| {
                    (name == row.orig_name)
                        .then(|| row.ga.map(|g| g.id - g.id % 10_000))
                        .flatten()
                });
                return Some(Arc::new(affinity_names(if name == EMPTY {
                    None
                } else {
                    base
                })));
            }
            _ => return None,
        }
        // Goods and talismans are listed once per list; weapons and armor can repeat.
        let taken: BTreeSet<&str> = if matches!(row.kind, Kind::Good | Kind::Talisman) {
            self.rows
                .iter()
                .filter(|r| r.list == row.list && r.kind == row.kind && r.ptrs[0] != row.ptrs[0])
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
        if row.handle.is_some() && !row.offered {
            all.retain(|n| n != EMPTY);
        }
        if row.orig_name != EMPTY && Self::id_of(row.kind, &row.orig_name).is_none() {
            all.insert(0, row.orig_name.clone());
        }
        Some(Arc::new(all))
    }

    /// Writes the changed rows (and the lists' counters) into `out`; adding or removing a
    /// weapon or armor resizes the item table.
    pub fn write(
        &self,
        out: &mut Vec<u8>,
        view: &Value,
        changes: &[(usize, Value)],
    ) -> Result<(), String> {
        // Flags first: resizing the item table below moves them along with everything else.
        for (leaf, _) in changes {
            if let Some(f) = self.flag(*leaf) {
                let on = view
                    .pointer(&f.ptr)
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if on {
                    out[f.at] |= 1 << f.bit;
                } else {
                    out[f.at] &= !(1 << f.bit);
                }
            }
        }
        let touched: BTreeSet<usize> = changes
            .iter()
            .filter_map(|(l, _)| l.checked_sub(self.first_leaf).map(|i| i / 4))
            .filter(|&r| r < self.rows.len())
            .collect();
        if touched.is_empty() {
            return Ok(());
        }
        let mut next: HashMap<usize, (u32, u32)> = self
            .lists
            .iter()
            .map(|l| {
                (
                    l.next_at,
                    (u32_at(out, l.next_at), u32_at(out, l.next_at + 4)),
                )
            })
            .collect();
        let mut delta = vec![0i64; self.lists.len()];
        let mut added: Vec<(usize, Kind, u32)> = vec![];
        // Item table changes, applied last (they move everything after them).
        let mut table_ops: Vec<(usize, usize, Vec<u8>)> = vec![];
        let mut free = self
            .table
            .as_ref()
            .map(|t| t.free.iter())
            .into_iter()
            .flatten();
        let mut last_handle = self.table.as_ref().map_or(0, |t| t.last_handle);
        for r in touched {
            let row = &self.rows[r];
            let name = Self::get(row, view, 0)
                .and_then(Value::as_str)
                .unwrap_or(EMPTY);
            let qty = Self::get(row, view, 1).and_then(Value::as_u64).unwrap_or(1) as u32;
            let level = Self::get(row, view, 2).and_then(Value::as_u64).unwrap_or(0) as u32;
            let aff = Self::get(row, view, 3)
                .and_then(Value::as_str)
                .unwrap_or(NO_AFFINITY);
            let aff_n = AFFINITIES.iter().position(|a| *a == aff).unwrap_or(0) as u32;
            let what = kind_word(row.kind).to_lowercase();
            let id =
                if name == EMPTY {
                    None
                } else if name == row.orig_name && row.handle.is_some() {
                    Some(match (row.kind, row.ga) {
                        (Kind::Weapon, Some(g)) => g.id - g.id % 10_000,
                        (Kind::Armor | Kind::Ash, Some(g)) => g.id & 0x0FFF_FFFF,
                        _ => row.handle.unwrap() & 0x0FFF_FFFF,
                    })
                } else {
                    Some(Self::id_of(row.kind, name).ok_or_else(|| {
                        format!("\u{201C}{name}\u{201D} isn't an Elden Ring {what}")
                    })?)
                };
            // What the item table entry should hold for this row.
            let ga_id = id.map(|id| match row.kind {
                Kind::Weapon => id + aff_n * 100 + level,
                Kind::Armor => ARMOR_ID | id,
                Kind::Ash => ASH_ID | id,
                _ => id,
            });
            let o = row.off;
            let l = &self.lists[row.list];
            match (row.handle, ga_id) {
                (None, None) => {}
                (Some(_), None) => {
                    out[o..o + ENTRY].fill(0);
                    delta[row.list] -= 1;
                    if let Some(g) = row.ga {
                        let mut empty = vec![0u8; 8];
                        empty[4..].copy_from_slice(&u32::MAX.to_le_bytes());
                        table_ops.push((g.off, g.len, empty));
                    }
                }
                (None, Some(gid)) => {
                    let handle = match row.kind {
                        Kind::Good => GOOD | gid,
                        Kind::Talisman => TALISMAN | gid,
                        k => {
                            let at = *free.next().ok_or("the item table is full")?;
                            last_handle += 1;
                            let h = match k {
                                Kind::Weapon => 0x8080_0000,
                                Kind::Armor => 0x9080_0000,
                                _ => 0xC080_0000,
                            } | last_handle;
                            let mut e = Vec::with_capacity(21);
                            e.extend(h.to_le_bytes());
                            e.extend(gid.to_le_bytes());
                            e.resize(
                                match k {
                                    Kind::Weapon => 21,
                                    Kind::Armor => 16,
                                    _ => 8,
                                },
                                0,
                            );
                            table_ops.push((at, 8, e));
                            h
                        }
                    };
                    added.push((row.list, row.kind, handle));
                    let n = next.get_mut(&l.next_at).unwrap();
                    put(out, o, handle);
                    put(
                        out,
                        o + 4,
                        if row.kind == Kind::Weapon || row.kind == Kind::Good {
                            qty.max(1)
                        } else {
                            1
                        },
                    );
                    put(out, o + 8, n.1);
                    n.0 += 1;
                    n.1 += 1;
                    delta[row.list] += 1;
                }
                (Some(h), Some(gid)) => match (row.kind, row.ga) {
                    (Kind::Weapon | Kind::Armor | Kind::Ash, Some(g)) => {
                        put(out, g.off + 4, gid);
                        if row.kind == Kind::Weapon {
                            put(out, o + 4, qty.max(1));
                        }
                    }
                    (Kind::Good | Kind::Talisman, _) => {
                        let nh = if row.kind == Kind::Good {
                            GOOD
                        } else {
                            TALISMAN
                        } | gid;
                        if nh != h {
                            added.push((row.list, row.kind, nh));
                            put(out, o, nh);
                        }
                        if row.kind == Kind::Good {
                            put(out, o + 4, qty.max(1));
                        }
                    }
                    _ => return Err(format!("this {what} isn't in the item table")),
                },
            }
        }
        for (i, l) in self.lists.iter().enumerate() {
            for &(_, kind, h) in added
                .iter()
                .filter(|(li, k, _)| *li == i && matches!(k, Kind::Good | Kind::Talisman))
            {
                let n = (0..l.cap)
                    .filter(|k| u32_at(out, l.start + k * ENTRY) == h)
                    .count();
                if n > 1 {
                    return Err(format!(
                        "{} already has {} \u{2014} change its quantity instead",
                        l.section,
                        names(kind).name(h & 0x0FFF_FFFF, kind_word(kind))
                    ));
                }
            }
            let n = (u32_at(out, l.count_at) as i64 + delta[i]) as u32;
            put(out, l.count_at, n);
            let (eq, acq) = next[&l.next_at];
            put(out, l.next_at, eq);
            put(out, l.next_at + 4, acq);
        }
        if !table_ops.is_empty() {
            let len = out.len();
            let grow: i64 = table_ops
                .iter()
                .map(|(_, old, new)| new.len() as i64 - *old as i64)
                .sum();
            let spare = self.table.as_ref().map_or(0, |t| t.spare) as i64;
            if grow > spare {
                return Err("there's no room left in this save for more weapons and armor".into());
            }
            table_ops.sort_by(|a, b| b.0.cmp(&a.0));
            for (at, old, new) in table_ops {
                out.splice(at..at + old, new);
            }
            // The slot keeps its size: unused bytes at the end make up the difference.
            out.resize(len, 0);
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Lays out empty lists after Vigor at `vigor` with `ammo` arrows registered.
    pub fn blank(d: &mut [u8], vigor: usize, ammo: u32) {
        let inv = vigor + INV_FROM_VIGOR;
        let next = inv + 4 + CARRIED_CAP * ENTRY + 4 + KEY_CAP * ENTRY;
        put(d, next, 10);
        put(d, next + 4, 100);
        let ammo_at = next + 8 + SPELLS_LEN + EQUIP_ITEMS_LEN + GESTURES_LEN;
        put(d, ammo_at, ammo);
    }

    /// An item table of unused entries with `weapons` (handle, id) first, ending where the
    /// character data at the returned Vigor offset starts; the slot's end marker is set.
    pub fn with_table(d: &mut Vec<u8>, weapons: &[(u32, u32)]) -> usize {
        let mut o = GA_START;
        for &(h, id) in weapons {
            put(d, o, h);
            put(d, o + 4, id);
            o += 21;
        }
        for _ in weapons.len()..GA_COUNT {
            put(d, o, 0);
            put(d, o + 4, u32::MAX);
            o += 8;
        }
        o + 0x34
    }

    /// Writes the "2" that `walk_to_end` checks (after the lists are laid out).
    pub fn mark_end(d: &mut [u8], vigor: usize) {
        let loc = locate(d, vigor).unwrap();
        let st = &loc.lists[2];
        let mut o = st.next_at + 8 + 0x40 * 4;
        o += 4 + 4 * u32_at(d, o) as usize;
        o += 0x28 + 1 + 0x40 + 12 + 0x1008 + 0x34 + 8 + 0x1B58 * 16 + 0x408 + 0x1D + 0x1B_F99F + 1;
        for _ in 0..5 {
            o += 4 + u32_at(d, o) as usize;
        }
        o += 0x3D + 0xF;
        put(d, o, 2);
    }

    /// Adds goods `id` to list 0 (carried), 1 (key) or 2 (storage) at `slot`.
    pub fn add(d: &mut [u8], vigor: usize, list: usize, slot: usize, id: u32, qty: u32) {
        add_handle(d, vigor, list, slot, GOOD | id, qty);
    }

    pub fn add_handle(d: &mut [u8], vigor: usize, list: usize, slot: usize, handle: u32, qty: u32) {
        let l = &locate(d, vigor).unwrap().lists[list];
        let o = l.start + slot * ENTRY;
        put(d, o, handle);
        put(d, o + 4, qty);
        put(d, o + 8, 50 + slot as u32);
        put(d, l.count_at, u32_at(d, l.count_at) + 1);
    }

    /// Puts goods `id` in the first quick slot.
    pub fn quick(d: &mut [u8], vigor: usize, id: u32) {
        let next = vigor + INV_FROM_VIGOR + 4 + CARRIED_CAP * ENTRY + 4 + KEY_CAP * ENTRY;
        put(d, next + 8 + SPELLS_LEN, 0x4000_0000 | id);
    }

    /// Wears the item with this handle (first right-hand weapon slot).
    pub fn wear(d: &mut [u8], vigor: usize, handle: u32) {
        put(d, vigor + WORN_FROM_VIGOR + 4, handle);
    }

    pub fn storage_at(d: &[u8], vigor: usize) -> Option<usize> {
        locate(d, vigor).and_then(|l| l.lists.get(2).map(|s| s.count_at))
    }
}
