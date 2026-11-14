//! Adding items to a Baldur's Gate 3 character's bag.
//!
//! In BG3 an item lives in two places in `Globals.lsf`:
//!
//! - an `Items/ItemFactory/Items/Item` node (template, `Stats` name, position) with, at the same
//!   place in the list, an `Items/ItemFactory/Creators/Creator` naming the item's entity;
//! - the entity store in `NewAge` (see `lsmf.rs`): the item's components, and the inventory
//!   records that say where it is.
//!
//! The entity store: an entity table (the GUID of every entity, its index being the entity's
//! number) and, per component type that entities can have, a list of the entities that have
//! it, in the order of the component's rows. The `core.v0.Level` row gives both: the table's
//! range, then the range of 32-byte ownership records `{list begin, list end, component,
//! count}` (all-`FF` and 0 for none). Every other component (an `EntityId` holding a GUID,
//! `MemberData`, `ContainerSlotData` and so on) is a pool that rows point into; a reference to
//! another entity is a pointer to a copy of its GUID in the `EntityId` pool.
//!
//! An item in a bag: its `MemberComponent` points to `{inventory, handle}` where the handle's
//! low 16 bits are its key in that inventory; the inventory's `ContainerComponent` is a map of
//! keys to `ContainerSlotData` rows `{item, grid cell, generation}`; `ContainerDataComponent`
//! keeps the lowest free key and the grid's extent. The character's `OwnerComponent` lists its
//! inventories, the bag first; its owner sets (`IsCurrentOwnerComponent` and so on) list the
//! items it owns.
//!
//! A new item is a copy of an item already in a party member's bag: a new entity with every
//! component row of the original (references to the original holder and inventory pointed at
//! the new ones), put into the target's bag at the first free key and grid cell, and added to
//! the target's owner sets where the original is in its holder's. Nothing already in the store
//! moves: the new entity table, the grown component columns and lists, and the new records
//! are appended before the store's directory, and the few pointers to them are updated in
//! place. Stackable items, equipped items, containers, keys and story items aren't copied.

use super::larian_add::{bytes, clone_node, random_guid};
use super::lsf::{Lsf, Writer};
use std::collections::{BTreeMap, HashMap, HashSet};

const BASE: usize = 48;
const FF: u64 = u64::MAX;
/// The "worn" bit of an item node's `Flags`.
const WORN: i64 = 0x0400_0000;

const LEVEL: &str = "core.v0.Level";
const ENTITY_ID: &str = "core.v0.EntityId";
const MEMBER: &str = "game.inventory.v0.MemberComponent";
const MEMBER_DATA: &str = "game.inventory.v0.MemberData";
const OWNER: &str = "game.inventory.v0.OwnerComponent";
const CONTAINER: &str = "game.inventory.v1.ContainerComponent";
const CONTAINER_DATA: &str = "game.inventory.v1.ContainerDataComponent";
const SLOT_DATA: &str = "game.inventory.v0.ContainerSlotData";
const OWNER_SETS: [&str; 3] = ["game.v0.IsCurrentOwnerComponent", "game.v0.IsLatestOwnerComponent", "game.v0.IsOriginalOwnerComponent"];
/// An item with any of these isn't copied: stacks, worn items, containers, keys, story items
/// and owners (whose records are edited in place).
const NOT_COPIED: [&str; 14] = [
    "game.inventory.v0.StackMemberComponent",
    "game.inventory.v0.NewStackComponent",
    "game.inventory.v0.WieldingComponent",
    "game.item.v0.IsStoryItemComponent",
    "game.lock.v0.KeyComponent",
    OWNER,
    CONTAINER,
    CONTAINER_DATA,
    "game.inventory.v1.IsOwnedComponent",
    "game.v0.IsCurrentOwnerComponent",
    "game.v0.IsLatestOwnerComponent",
    "game.v0.IsOriginalOwnerComponent",
    "game.v1.IsPreviousOwnerComponent",
    "game.v1.IsPreviousLatestOwnerComponent",
];
/// Most copies of one item per save.
pub const COPIES: [u32; 5] = [1, 2, 3, 5, 10];

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at.checked_add(2)?)?.try_into().ok()?))
}
fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at.checked_add(4)?)?.try_into().ok()?))
}
fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at.checked_add(8)?)?.try_into().ok()?))
}
fn abs(v: u64) -> Option<usize> {
    usize::try_from(v).ok()?.checked_add(BASE)
}
fn rel(at: usize) -> u64 {
    (at - BASE) as u64
}

struct Comp {
    name: String,
    elem: usize,
    rows: usize,
    /// Where the rows start (absolute).
    off: usize,
    /// Where the descriptor is (absolute).
    desc: usize,
}

struct Rec {
    at: usize,
    start: usize,
    count: usize,
}

/// A parsed entity store.
pub struct Store<'a> {
    b: &'a [u8],
    comps: Vec<Comp>,
    by: HashMap<String, usize>,
    names_off: usize,
    level: usize,
    ent: usize,
    n: usize,
    idx: HashMap<[u8; 16], usize>,
    recs: HashMap<usize, Rec>,
    own: Vec<Vec<(usize, usize)>>,
}

/// An inventory's slot map.
struct Cont {
    row: usize,
    keys: Vec<u16>,
    vals: Vec<u64>,
    /// (item entity, grid cell, generation) per key.
    items: Vec<(Option<usize>, u32, u32)>,
}

impl<'a> Store<'a> {
    pub fn read(b: &'a [u8]) -> Option<Self> {
        if b.get(0..4)? != b"LSMF" {
            return None;
        }
        let names_off = abs(u64_at(b, 16)?)?;
                let desc = names_off.checked_add(u32_at(b, 32)? as usize)?;
        let count = u16_at(b, 36)? as usize;
        if names_off % 8 != 0 || desc.checked_add(count * 48)? > b.len() {
            return None;
        }
        let mut comps = Vec::with_capacity(count);
        let mut by = HashMap::new();
        for i in 0..count {
            let at = desc + 48 * i;
            let (no, nl) = (u64_at(b, at)? as usize, u32_at(b, at + 8)? as usize);
            let name = std::str::from_utf8(b.get(names_off.checked_add(no)?..names_off.checked_add(no)?.checked_add(nl)?)?).ok()?.to_string();
            let (elem, rows, off) = (u32_at(b, at + 24)? as usize, u64_at(b, at + 32)? as usize, abs(u64_at(b, at + 40)?)?);
            if rows.checked_mul(elem)?.checked_add(off)? > names_off || by.insert(name.clone(), i).is_some() {
                return None;
            }
            comps.push(Comp { name, elem, rows, off, desc: at });
        }
        if [MEMBER_DATA, SLOT_DATA].iter().any(|n| by.get(*n).is_some_and(|&i| comps[i].elem != 16)) {
            return None;
        }
        let lv = &comps[*by.get(LEVEL)?];
        let eid = &comps[*by.get(ENTITY_ID)?];
        if lv.elem != 32 || lv.rows != 1 || eid.elem != 16 {
            return None;
        }
        let level = lv.off;
        let (ent, ent_end) = (abs(u64_at(b, level)?)?, abs(u64_at(b, level + 8)?)?);
        let (rec_lo, rec_hi) = (abs(u64_at(b, level + 16)?)?, abs(u64_at(b, level + 24)?)?);
        if ent_end < ent || (ent_end - ent) % 16 != 0 || rec_hi < rec_lo || (rec_hi - rec_lo) % 32 != 0 || rec_hi > names_off {
            return None;
        }
        let n = (ent_end - ent) / 16;
        // The table is the start of the `EntityId` pool until an edit moves it.
        if ent != eid.off && ent + 16 * n > names_off {
            return None;
        }
        let mut idx = HashMap::with_capacity(n);
        for e in 0..n {
            if idx.insert(b[ent + 16 * e..ent + 16 * e + 16].try_into().ok()?, e).is_some() {
                return None;
            }
        }
        let mut recs = HashMap::new();
        let mut own = vec![vec![]; n];
        for at in (rec_lo..rec_hi).step_by(32) {
            let (s, e, c, k) = (u64_at(b, at)?, u64_at(b, at + 8)?, u64_at(b, at + 16)?, u64_at(b, at + 24)?);
            if (s, e, c, k) == (FF, FF, FF, 0) {
                continue;
            }
            let (c, k) = (usize::try_from(c).ok()?, usize::try_from(k).ok()?);
            let (start, end) = (abs(s)?, abs(e)?);
            if c >= comps.len() || end < start || end - start != 4 * k || k != comps[c].rows || end > names_off || recs.contains_key(&c) {
                return None;
            }
            for p in 0..k {
                let who = u32_at(b, start + 4 * p)? as usize;
                own.get_mut(who)?.push((c, p));
            }
            recs.insert(c, Rec { at, start, count: k });
        }
        Some(Store { b, comps, by, names_off, level, ent, n, idx, recs, own })
    }

    fn comp(&self, name: &str) -> Option<usize> {
        self.by.get(name).copied()
    }

    pub fn entity(&self, guid: &[u8]) -> Option<usize> {
        self.idx.get(guid).copied()
    }

    pub fn guid(&self, e: usize) -> [u8; 16] {
        self.b[self.ent + 16 * e..self.ent + 16 * e + 16].try_into().unwrap()
    }

    fn has(&self, e: usize, name: &str) -> Option<usize> {
        let c = self.comp(name)?;
        self.own.get(e)?.iter().find(|o| o.0 == c).map(|o| o.1)
    }

    fn row_at(&self, c: usize, p: usize) -> usize {
        self.comps[c].off + p * self.comps[c].elem
    }

    fn row(&self, c: usize, p: usize) -> &[u8] {
        let at = self.row_at(c, p);
        &self.b[at..at + self.comps[c].elem]
    }

    fn named_row(&self, e: usize, name: &str) -> Option<(usize, &[u8])> {
        let c = self.comp(name)?;
        let p = self.has(e, name)?;
        Some((self.row_at(c, p), self.row(c, p)))
    }

    /// The entity a reference (a pointer to an `EntityId` row) names, and whether the row is a
    /// copy (not in the entity table itself).
    fn referent(&self, v: u64) -> Option<(usize, bool)> {
        // Pointers go into the `EntityId` pool, or (after an edit) to the rows appended.
        let a = self.pool_row(v, 16)?;
        Some((self.entity(&self.b[a..a + 16])?, a < self.ent || a >= self.ent + 16 * self.n))
    }

    /// Where a pointer to a pool row of `len` bytes leads: 8-aligned, inside the data.
    fn pool_row(&self, v: u64, len: usize) -> Option<usize> {
        let a = abs(v)?;
        (a >= BASE && a % 8 == 0 && a.checked_add(len)? <= self.names_off).then_some(a)
    }

    /// A character's bag: the first inventory its `OwnerComponent` lists.
    pub fn bag(&self, ch: usize) -> Option<usize> {
        let (_, r) = self.named_row(ch, OWNER)?;
        let (lo, hi) = (abs(u64_at(r, 0)?)?, abs(u64_at(r, 8)?)?);
        if hi <= lo || (hi - lo) % 8 != 0 {
            return None;
        }
        let first = self.referent(u64_at(self.b, lo)?)?.0;
        (self.referent(u64_at(r, 16)?)?.0 == first).then_some(first)
    }

    fn container(&self, inv: usize) -> Option<Cont> {
        let (row, r) = self.named_row(inv, CONTAINER)?;
        let q = [u64_at(r, 0)?, u64_at(r, 8)?, u64_at(r, 16)?, u64_at(r, 24)?];
        if q == [FF; 4] {
            return Some(Cont { row, keys: vec![], vals: vec![], items: vec![] });
        }
        let (k0, k1, v0, v1) = (abs(q[0])?, abs(q[1])?, abs(q[2])?, abs(q[3])?);
        if k1 < k0 || v1 < v0 || (k1 - k0) % 2 != 0 || (v1 - v0) % 8 != 0 || (k1 - k0) / 2 != (v1 - v0) / 8 || v1 > self.b.len() || k1 > self.b.len() {
            return None;
        }
        let k = (k1 - k0) / 2;
        let mut c = Cont { row, keys: vec![], vals: vec![], items: vec![] };
        for i in 0..k {
            c.keys.push(u16_at(self.b, k0 + 2 * i)?);
            let v = u64_at(self.b, v0 + 8 * i)?;
            let a = self.pool_row(v, 16)?;
            c.vals.push(v);
            c.items.push((self.referent(u64_at(self.b, a)?).map(|r| r.0), u32_at(self.b, a + 8)?, u32_at(self.b, a + 12)?));
        }
        let mut seen = HashSet::new();
        c.keys.iter().all(|k| seen.insert(*k)).then_some(c)
    }

    /// The items in a character's bag.
    pub fn bag_items(&self, ch: usize) -> Vec<usize> {
        let Some(c) = self.bag(ch).and_then(|b| self.container(b)) else { return vec![] };
        c.items.iter().filter_map(|i| i.0).collect()
    }

    /// An item's inventory record: (where its `MemberData` is, the inventory, the handle).
    fn member(&self, item: usize) -> Option<(usize, usize, u64)> {
        let (_, r) = self.named_row(item, MEMBER)?;
        let a = self.pool_row(u64_at(r, 0)?, 16)?;
        Some((a, self.referent(u64_at(self.b, a)?)?.0, u64_at(self.b, a + 8)?))
    }

    /// Whether `item` (in a bag) can be copied.
    pub fn copyable(&self, item: usize) -> bool {
        let Some(own) = self.own.get(item) else { return false };
        !own.is_empty()
            && NOT_COPIED.iter().all(|n| self.has(item, n).is_none())
            && own.iter().all(|&(c, _)| self.comps[c].elem > 0 && self.comps[c].name != LEVEL && self.comps[c].name != ENTITY_ID)
            && self.member(item).is_some_and(|(_, inv, h)| h & 0xFFFF == self.container(inv).and_then(|c| c.items.iter().position(|i| i.0 == Some(item)).map(|p| u64::from(c.keys[p]))).unwrap_or(u64::MAX))
    }

    /// What isn't pointers but could pass for them: the Level row, rows of sizes that can't
    /// hold an aligned pointer, every inventory's keys (16-bit numbers) and the entity lists.
    fn plain_ranges(&self) -> Vec<(usize, usize)> {
        let mut out = vec![(self.level, self.level + 32)];
        // Rows that can't hold an aligned pointer: sizes that aren't a multiple of 8.
        out.extend(self.comps.iter().filter(|c| c.elem % 8 != 0).map(|c| (c.off, c.off + c.rows * c.elem)));
        out.extend(self.recs.values().map(|r| (r.start, r.start + 4 * r.count)));
        if let Some(c) = self.comp(CONTAINER) {
            for p in 0..self.comps[c].rows {
                // Only the map form {keys begin, end, values begin, end}; a small inventory
                // can keep its entries in the row itself.
                let r = self.row(c, p);
                let q: Vec<usize> = (0..4).filter_map(|i| u64_at(r, 8 * i).and_then(abs)).collect();
                if let [k0, k1, v0, v1] = q[..] {
                    if k0 <= k1 && v0 <= v1 && (k1 - k0) % 2 == 0 && (v1 - v0) % 8 == 0 && (k1 - k0) / 2 == (v1 - v0) / 8 && k1 <= v0 && v1 <= self.names_off {
                        out.push((k0, k1));
                    }
                }
            }
        }
        out.sort();
        out
    }

    /// The owner sets of `ch` (component, row, members).
    fn set(&self, ch: usize, name: &str) -> Option<(usize, Vec<u64>)> {
        let (row, r) = self.named_row(ch, name)?;
        if r.len() != 16 {
            return None;
        }
        let (lo, hi) = (u64_at(r, 0)?, u64_at(r, 8)?);
        if (lo, hi) == (FF, FF) {
            return Some((row, vec![]));
        }
        let (lo, hi) = (abs(lo)?, abs(hi)?);
        if hi < lo || (hi - lo) % 8 != 0 || hi > self.b.len() {
            return None;
        }
        Some((row, (lo..hi).step_by(8).map(|a| u64_at(self.b, a)).collect::<Option<_>>()?))
    }
}

/// New bytes appended before the directory.
struct Area {
    base: usize,
    data: Vec<u8>,
    /// Ranges (absolute) that hold no pointers.
    plain: Vec<(usize, usize)>,
}

impl Area {
    /// Appends `bytes` aligned to `align`, and returns where they are.
    fn place(&mut self, bytes: &[u8], align: usize) -> usize {
        while (self.base + self.data.len()) % align != 0 {
            self.data.push(0);
        }
        let at = self.base + self.data.len();
        self.data.extend_from_slice(bytes);
        at
    }

    /// Appends `bytes` and returns the pointer to them.
    fn put(&mut self, bytes: &[u8], align: usize) -> u64 {
        rel(self.place(bytes, align))
    }

    /// Like `put`, for bytes that aren't pointers.
    fn put_plain(&mut self, bytes: &[u8], align: usize) -> u64 {
        let at = self.place(bytes, align);
        self.plain.push((at, at + bytes.len()));
        rel(at)
    }

    fn write(&mut self, at: usize, bytes: &[u8]) {
        let o = at - self.base;
        self.data[o..o + bytes.len()].copy_from_slice(bytes);
    }
}

/// One item to copy: the entity copied, the character receiving it, how many copies.
#[derive(Clone, Copy, Debug)]
pub struct Dup {
    pub source: usize,
    pub target: usize,
    pub copies: usize,
}

/// A pool whose rows the edit adds to: the rows are moved, grown, to the new area, and every
/// pointer to an old row is shifted to its new place.
#[derive(Clone, Copy)]
struct Pool {
    /// The rows that move (absolute range).
    lo: usize,
    hi: usize,
    /// Where they go.
    to: usize,
}

impl Pool {
    fn shift(&self, v: u64, elem: usize) -> Option<u64> {
        let a = abs(v)?;
        (a >= self.lo && a < self.hi && (a - self.lo) % elem == 0).then(|| rel(a - self.lo + self.to))
    }
}

/// Where pointers can be: every 8-byte word from `BASE` to `to`, aligned to its column's rows
/// inside a column whose rows are a multiple of 8 bytes, aligned to the file elsewhere, and
/// not overlapping `skip`.
fn words(s: &Store, skip: &[(usize, usize)], to: usize) -> Vec<usize> {
    let mut cols: Vec<(usize, usize)> = s.comps.iter().filter(|c| c.elem % 8 == 0 && c.rows > 0).map(|c| (c.off, c.off + c.rows * c.elem)).collect();
    cols.sort();
    let mut skip: Vec<(usize, usize)> = skip.iter().copied().filter(|r| r.0 < r.1).collect();
    skip.sort();
    let mut merged: Vec<(usize, usize)> = vec![];
    for r in skip {
        match merged.last_mut() {
            Some(m) if r.0 <= m.1 => m.1 = m.1.max(r.1),
            _ => merged.push(r),
        }
    }
    let mut out = vec![];
    let mut j = 0;
    let mut push = |at: usize, out: &mut Vec<usize>| {
        while j < merged.len() && merged[j].1 <= at {
            j += 1;
        }
        if !(j < merged.len() && merged[j].0 < at + 8) {
            out.push(at);
        }
    };
    let mut at = BASE;
    for (lo, hi) in cols {
        if hi <= at {
            continue;
        }
        let mut h = at.next_multiple_of(8);
        while h + 8 <= lo.min(to) {
            push(h, &mut out);
            h += 8;
        }
        let mut c = lo.max(at);
        c += (lo % 8 + 8 - c % 8) % 8;
        while c + 8 <= hi.min(to) {
            push(c, &mut out);
            c += 8;
        }
        at = hi;
    }
    let mut h = at.next_multiple_of(8);
    while h + 8 <= to {
        push(h, &mut out);
        h += 8;
    }
    out
}

/// How many of the words at `at` lead to each row of a pool's range `lo..hi`.
fn pointer_counts(b: &[u8], at: &[usize], lo: usize, hi: usize, elem: usize) -> Vec<u32> {
    let mut n = vec![0u32; (hi - lo) / elem];
    for &w in at {
        let v = u64::from_le_bytes(b[w..w + 8].try_into().unwrap());
        if let Some(a) = abs(v).filter(|&a| a >= lo && a < hi && (a - lo) % elem == 0) {
            n[(a - lo) / elem] += 1;
        }
    }
    n
}

/// Adds copies of items into characters' bags. Returns the new store and, per new entity,
/// (the entity copied, the new GUID, the character receiving it).
///
/// Every row the copies need is kept inside its own pool: the entity table with the `EntityId`
/// pool it starts, `ContainerSlotData` and `MemberData` are moved to the new area, grown, and
/// every pointer to their old rows is shifted. Pointers are 8-byte-aligned values; what can't
/// be one (see `plain_ranges`) isn't looked at. In the saves this was built on, every row of
/// those pools is pointed to, nearly always once (hotbar slots and a few others point to rows
/// that something else points to as well); the result is checked to point to every old row at
/// least as often as before, and to every new one.
pub fn add(b: &[u8], adds: &[Dup]) -> Result<(Vec<u8>, Vec<(usize, [u8; 16], usize)>), String> {
    let s = Store::read(b).ok_or("the entity store doesn't read")?;
    let (c_member, c_eid, c_csd, c_md) = (
        s.comp(MEMBER).ok_or("no inventory records")?,
        s.comp(ENTITY_ID).ok_or("no entity ids")?,
        s.comp(SLOT_DATA).ok_or("no inventory slots")?,
        s.comp(MEMBER_DATA).ok_or("no inventory records")?,
    );
    let (eid, csd, md) = (&s.comps[c_eid], &s.comps[c_csd], &s.comps[c_md]);
    if s.ent != eid.off || [csd.elem, md.elem] != [16, 16] {
        return Err("the entity table isn't where it's expected".into());
    }
    let level_row = s.plain_ranges();
    let (refs_lo, refs_hi) = (s.ent + 16 * s.n, eid.off + 16 * eid.rows);
    let mut taken: HashSet<[u8; 16]> = s.idx.keys().copied().collect();
    let mut news = vec![];
    for a in adds {
        if !s.copyable(a.source) {
            return Err("an item can't be copied".into());
        }
        for _ in 0..a.copies {
            let g = random_guid(&taken);
            taken.insert(g);
            news.push((a.source, g, a.target));
        }
    }
    if news.is_empty() {
        return Ok((b.to_vec(), vec![]));
    }
    let k = news.len();
    let m = eid.rows - s.n;
    // At most one new reference per pointer-sized field of each copied row, and a few more.
    let most: usize = news.iter().map(|n| s.own[n.0].iter().map(|&(c, _)| s.comps[c].elem / 8).sum::<usize>() + 2 + OWNER_SETS.len()).sum();
    let mut r = Area { base: s.names_off, data: vec![], plain: vec![] };
    // A gap first: a range ending at the end of the old data must not seem to lead into the new.
    r.put_plain(&[0; 16], 16);
    let e_at = r.place(&vec![0; 16 * (s.n + k + m + most)], 16);
    r.plain.push((e_at, e_at + 16 * (s.n + k + m + most)));
    let c_at = r.place(&vec![0; 16 * (csd.rows + k)], 16);
    let m_at = r.place(&vec![0; 16 * (md.rows + k)], 16);
    let pools = [
        (Pool { lo: refs_lo, hi: refs_hi, to: e_at + 16 * (s.n + k) }, 16usize),
        (Pool { lo: csd.off, hi: csd.off + 16 * csd.rows, to: c_at }, 16),
        (Pool { lo: md.off, hi: md.off + 16 * md.rows, to: m_at }, 16),
    ];
    // New references: rows after the old ones in the grown pool.
    let mut refs: Vec<[u8; 16]> = vec![];
    let mut new_ref = |r: &mut Area, g: [u8; 16]| -> Result<u64, String> {
        if refs.len() >= most {
            return Err("too many references".into());
        }
        let at = e_at + 16 * (s.n + k + m + refs.len());
        r.write(at, &g);
        refs.push(g);
        Ok(rel(at))
    };
    let mut patches: Vec<(usize, Vec<u8>)> = vec![];
    // Arrays replaced by grown copies.
    let mut dead: Vec<(usize, usize)> = vec![];
    let mut conts: BTreeMap<usize, Cont> = BTreeMap::new();
    let mut sets: BTreeMap<(usize, usize), Vec<u64>> = BTreeMap::new();
    let mut rows: BTreeMap<usize, Vec<(u32, Vec<u8>)>> = BTreeMap::new();
    for (j, &(src, guid, target)) in news.iter().enumerate() {
        let me = (s.n + j) as u32;
        let (_, src_inv, handle) = s.member(src).ok_or("an item's inventory record doesn't read")?;
        let holder = s.named_row(src_inv, "game.inventory.v1.IsOwnedComponent").and_then(|(_, r)| s.referent(u64_at(r, 0)?)).map(|r| r.0).ok_or("an item's holder isn't known")?;
        let bag = s.bag(target).ok_or("the character's bag isn't known")?;
        let gen = s.container(src_inv).and_then(|c| c.items.iter().find(|i| i.0 == Some(src)).map(|i| i.2)).unwrap_or(0);
        if let std::collections::btree_map::Entry::Vacant(v) = conts.entry(bag) {
            v.insert(s.container(bag).ok_or("the character's bag doesn't read")?);
        }
        let cont = conts.get_mut(&bag).unwrap();
        let key = (0..=u16::MAX).find(|k| !cont.keys.contains(k)).ok_or("the bag is full")?;
        let cell = (0..).find(|c| !cont.items.iter().any(|i| i.1 == *c)).unwrap();
        let mut slot = new_ref(&mut r, guid)?.to_le_bytes().to_vec();
        slot.extend(cell.to_le_bytes());
        slot.extend(gen.to_le_bytes());
        let slot_at = c_at + 16 * (csd.rows + j);
        r.write(slot_at, &slot);
        cont.keys.push(key);
        cont.vals.push(rel(slot_at));
        cont.items.push((None, cell, gen));

        let bag_guid = s.guid(bag);
        let target_guid = s.guid(target);
        let (src_guid, holder_guid) = (s.guid(src), s.guid(holder));
        for &(c, p) in &s.own[src] {
            let mut row = s.row(c, p).to_vec();
            if c == c_member {
                let mut data = new_ref(&mut r, bag_guid)?.to_le_bytes().to_vec();
                data.extend(((handle & !0xFFFF) | u64::from(key)).to_le_bytes());
                let at = m_at + 16 * (md.rows + j);
                r.write(at, &data);
                row[..8].copy_from_slice(&rel(at).to_le_bytes());
            } else {
                // Each reference gets its own copy of the GUID, as the game writes them;
                // references to the original's holder and inventory name the new ones.
                for o in (0..row.len().saturating_sub(7)).step_by(8) {
                    let v = u64::from_le_bytes(row[o..o + 8].try_into().unwrap());
                    let Some(a) = abs(v).filter(|&a| a >= refs_lo && a < refs_hi && (a - refs_lo) % 16 == 0) else { continue };
                    let g: [u8; 16] = b[a..a + 16].try_into().unwrap();
                    let to = match g {
                        g if g == src_guid => guid,
                        g if g == holder_guid => target_guid,
                        g if g == s.guid(src_inv) => bag_guid,
                        g => g,
                    };
                    row[o..o + 8].copy_from_slice(&new_ref(&mut r, to)?.to_le_bytes());
                }
                for o in (0..row.len().saturating_sub(15)).step_by(4) {
                    if row[o..o + 16] == src_guid {
                        row[o..o + 16].copy_from_slice(&guid);
                    } else if row[o..o + 16] == holder_guid {
                        row[o..o + 16].copy_from_slice(&target_guid);
                    }
                }
            }
            rows.entry(c).or_default().push((me, row));
        }
        for name in OWNER_SETS {
            let Some((_, members)) = s.set(holder, name) else { continue };
            if !members.iter().any(|&m| s.referent(m).is_some_and(|r| r.0 == src)) {
                continue;
            }
            let Some(c) = s.comp(name) else { continue };
            if s.set(target, name).is_none() {
                continue;
            }
            let p = new_ref(&mut r, guid)?;
            sets.entry((target, c)).or_default().push(p);
        }
    }
    // The grown pools: the entity table, then the old references (new ones are in place).
    let mut table = b[s.ent..s.ent + 16 * s.n].to_vec();
    for n in &news {
        table.extend_from_slice(&n.1);
    }
    r.write(e_at, &table);
    r.write(e_at + table.len(), &b[refs_lo..refs_hi]);
    r.write(c_at, &b[csd.off..csd.off + 16 * csd.rows]);
    r.write(m_at, &b[md.off..md.off + 16 * md.rows]);
    let mut desc: Vec<(usize, Vec<u8>)> = vec![
        (eid.desc + 32, [((s.n + k + m + refs.len()) as u64).to_le_bytes(), rel(e_at).to_le_bytes()].concat()),
        (csd.desc + 32, [((csd.rows + k) as u64).to_le_bytes(), rel(c_at).to_le_bytes()].concat()),
        (md.desc + 32, [((md.rows + k) as u64).to_le_bytes(), rel(m_at).to_le_bytes()].concat()),
    ];
    let level = [rel(e_at), rel(e_at + table.len())];
    // The bags' slot maps and their lowest free key and grid extent.
    for (bag, cont) in &conts {
        for (lo, hi) in [(0, 8), (16, 24)] {
            if let (Some(a), Some(z)) = (u64_at(b, cont.row + lo).and_then(abs), u64_at(b, cont.row + hi).and_then(abs)) {
                if a <= z && z <= s.names_off {
                    dead.push((a, z));
                }
            }
        }
        let keys: Vec<u8> = cont.keys.iter().flat_map(|k| k.to_le_bytes()).collect();
        let vals: Vec<u8> = cont.vals.iter().flat_map(|v| v.to_le_bytes()).collect();
        let k0 = r.put_plain(&keys, 8);
        let v0 = r.put(&vals, 8);
        patches.push((cont.row, [k0, k0 + keys.len() as u64, v0, v0 + vals.len() as u64].iter().flat_map(|v| v.to_le_bytes()).collect()));
        let (at, d) = s.named_row(*bag, CONTAINER_DATA).ok_or("the bag's record doesn't read")?;
        if d.len() != 8 {
            return Err("the bag's record is an unexpected size".into());
        }
        let free = (0..=u16::MAX).find(|k| !cont.keys.contains(k)).unwrap_or(u16::MAX);
        let extent = cont.items.iter().map(|i| i.1 + 1).max().unwrap_or(0).max(u32_at(d, 4).unwrap_or(0));
        patches.push((at, free.to_le_bytes().to_vec()));
        patches.push((at + 4, extent.to_le_bytes().to_vec()));
    }
    // The owner sets.
    for (&(ch, c), extra) in &sets {
        let name = &s.comps[c].name;
        let (row, mut members) = s.set(ch, name).ok_or("an owner record doesn't read")?;
        if !members.is_empty() {
            let a = abs(u64_at(b, row).unwrap_or(FF)).unwrap_or(0);
            dead.push((a, a + 8 * members.len()));
        }
        members.extend(extra);
        let bytes: Vec<u8> = members.iter().flat_map(|v| v.to_le_bytes()).collect();
        let lo = r.put(&bytes, 8);
        patches.push((row, [lo, lo + bytes.len() as u64].iter().flat_map(|v| v.to_le_bytes()).collect()));
    }
    // Component columns and their entity lists, grown.
    for (c, new) in &rows {
        let comp = &s.comps[*c];
        let rec = s.recs.get(c).ok_or("a component has no entity list")?;
        let mut col = b[comp.off..comp.off + comp.rows * comp.elem].to_vec();
        let mut list = b[rec.start..rec.start + 4 * rec.count].to_vec();
        for (e, row) in new {
            col.extend_from_slice(row);
            list.extend(e.to_le_bytes());
        }
        let off = if comp.elem % 8 == 0 { r.put(&col, 16) } else { r.put_plain(&col, 16) };
        let lo = r.put_plain(&list, 8);
        let count = (rec.count + new.len()) as u64;
        desc.push((comp.desc + 32, [count.to_le_bytes(), off.to_le_bytes()].concat()));
        patches.push((rec.at, [lo, lo + list.len() as u64, *c as u64, count].iter().flat_map(|v| v.to_le_bytes()).collect()));
    }
    while r.data.len() % 16 != 0 {
        r.data.push(0);
    }
    let grow = r.data.len();
    let mut out = Vec::with_capacity(b.len() + grow);
    out.extend_from_slice(&b[..s.names_off]);
    out.extend_from_slice(&r.data);
    out.extend_from_slice(&b[s.names_off..]);
    for (at, bytes) in patches {
        out[at..at + bytes.len()].copy_from_slice(&bytes);
    }
    // Shift every pointer to a moved row, except in what moved (the old copies are left as
    // they were, pointing at each other, and nothing points at them).
    let mut skip = r.plain.clone();
    skip.extend(level_row);
    skip.extend([(eid.off, refs_hi), (csd.off, csd.off + 16 * csd.rows), (md.off, md.off + 16 * md.rows)]);
    skip.extend(rows.keys().map(|&c| (s.comps[c].off, s.comps[c].off + s.comps[c].rows * s.comps[c].elem)));
    skip.extend(dead);
    for w in words(&s, &skip, s.names_off + grow) {
        let v = u64::from_le_bytes(out[w..w + 8].try_into().unwrap());
        if let Some(nv) = pools.iter().find_map(|(p, elem)| p.shift(v, *elem)) {
            out[w..w + 8].copy_from_slice(&nv.to_le_bytes());
        }
    }
    out[s.level..s.level + 16].copy_from_slice(&[level[0].to_le_bytes(), level[1].to_le_bytes()].concat());
    let dir = u64_at(b, 16).unwrap() + grow as u64;
    out[16..24].copy_from_slice(&dir.to_le_bytes());
    for (at, bytes) in desc {
        out[at + grow..at + grow + bytes.len()].copy_from_slice(&bytes);
    }
    verify(b, &out, &news)?;
    Ok((out, news))
}

/// A row with pointers replaced by what they lead to (a GUID, or the slot or inventory record
/// and the GUID it names), so rows compare across a move of the pools.
fn resolved(s: &Store, row: &[u8]) -> Vec<u64> {
    let pool = |name: &str| s.comp(name).map(|c| &s.comps[c]).map(|p| (p.off, p.off + p.rows * p.elem));
    // References are to the rows after the entity table.
    let eid = pool(ENTITY_ID).map(|(lo, hi)| (lo.max(s.ent + 16 * s.n).min(hi), hi));
    let others = [pool(SLOT_DATA), pool(MEMBER_DATA)];
    if row.len() % 8 != 0 {
        return row.iter().map(|&x| u64::from(x)).collect();
    }
    let inside = |v: u64, r: Option<(usize, usize)>| abs(v).filter(|&a| r.is_some_and(|(lo, hi)| a >= lo && a < hi && (a - lo) % 16 == 0));
    let guid = |v: u64| inside(v, eid).map(|a| [u64_at(s.b, a).unwrap_or(0), u64_at(s.b, a + 8).unwrap_or(0)]);
    let mut out = vec![];
    for w in row.chunks(8) {
        let Ok(w) = <[u8; 8]>::try_from(w) else {
            out.extend(w.iter().map(|&x| u64::from(x)));
            continue;
        };
        let v = u64::from_le_bytes(w);
        if let Some(g) = guid(v) {
            out.extend([1 << 62, g[0], g[1]]);
        } else if let Some(a) = others.iter().find_map(|&r| inside(v, r)) {
            let first = u64_at(s.b, a).unwrap_or(0);
            out.push(1 << 61);
            out.extend(guid(first).unwrap_or([first, 0]));
            out.push(u64_at(s.b, a + 8).unwrap_or(0));
        } else {
            out.push(v);
        }
    }
    out
}

/// Re-reads the edited store: the old entities unchanged, each new one with its original's
/// components, in the target's bag with its own key, and no row of the grown pools pointed to
/// twice.
fn verify(old: &[u8], new: &[u8], news: &[(usize, [u8; 16], usize)]) -> Result<(), String> {
    let bad = |m: &str| format!("the edited entity store doesn't check out ({m})");
    let (a, b) = (Store::read(old).ok_or_else(|| bad("old"))?, Store::read(new).ok_or_else(|| bad("doesn't read"))?);
    if b.n != a.n + news.len() || (0..a.n).any(|e| a.guid(e) != b.guid(e)) {
        return Err(bad("entities"));
    }
    let rows = |s: &Store, e: usize| s.own[e].iter().map(|&(c, p)| (s.comps[c].name.clone(), resolved(s, s.row(c, p)))).collect::<Vec<_>>();
    for e in 0..a.n {
        let (x, y) = (rows(&a, e), rows(&b, e));
        let edited = |n: &str| n == CONTAINER || n == CONTAINER_DATA || OWNER_SETS.contains(&n);
        if x.len() != y.len() || x.iter().zip(&y).any(|(p, q)| p.0 != q.0 || (p.1 != q.1 && !edited(&p.0))) {
            return Err(bad("an existing entity changed"));
        }
    }
    for (k, &(src, guid, target)) in news.iter().enumerate() {
        let e = a.n + k;
        if b.guid(e) != guid || b.own[e].len() != a.own[src].len() {
            return Err(bad("a new entity's components"));
        }
        let (_, inv, h) = b.member(e).ok_or_else(|| bad("a new item's record"))?;
        let bag = b.bag(target).ok_or_else(|| bad("bag"))?;
        let c = b.container(bag).ok_or_else(|| bad("the bag"))?;
        let p = c.items.iter().position(|i| i.0 == Some(e)).ok_or_else(|| bad("the new item isn't in the bag"))?;
        if inv != bag || u64::from(c.keys[p]) != h & 0xFFFF {
            return Err(bad("the new item's key"));
        }
    }
    // Every old row of the grown pools is pointed to as often as before, each new one too.
    for name in [ENTITY_ID, SLOT_DATA, MEMBER_DATA] {
        let count = |s: &Store, data: &[u8], table: usize| {
            let p = &s.comps[s.comp(name).unwrap()];
            pointer_counts(data, &words(s, &s.plain_ranges(), s.names_off), p.off + 16 * table, p.off + p.rows * p.elem, p.elem)
        };
        let table = |s: &Store| if name == ENTITY_ID { s.n } else { 0 };
        let (was, now) = (count(&a, old, table(&a)), count(&b, new, table(&b)));
        // (A number that happens to equal a new row's place counts too, so only "at least".)
        if now.len() < was.len() || was.iter().zip(&now).any(|(w, n)| n < w) || now[was.len()..].iter().any(|&c| c == 0) {
            return Err(bad("pointers"));
        }
    }
    Ok(())
}

/// The party members' items that can be copied, by `Stats` name: (item entity, item node,
/// creator node), the first found of each name.
pub fn offers(l: &Lsf, s: &Store, party: &[(String, usize)]) -> BTreeMap<String, (usize, usize, usize)> {
    let items = l.at_path(&["Items", "ItemFactory", "Items", "Item"]);
    let creators = l.at_path(&["Items", "ItemFactory", "Creators", "Creator"]);
    let mut out = BTreeMap::new();
    if items.len() != creators.len() {
        return out;
    }
    let mut node: HashMap<[u8; 16], usize> = HashMap::new();
    for (k, &c) in creators.iter().enumerate() {
        if let Some(g) = bytes(l, c, "Entity").and_then(|g| <[u8; 16]>::try_from(g).ok()) {
            node.insert(g, k);
        }
    }
    let chars = characters(l);
    for (_, ch_node) in party {
        let Some(ch) = chars.iter().find(|c| c.0 == *ch_node).and_then(|c| s.entity(&c.1)) else { continue };
        for item in s.bag_items(ch) {
            let Some(&k) = node.get(&s.guid(item)) else { continue };
            let (i, c) = (items[k], creators[k]);
            let Some(stats) = l.string_of(i, "Stats").filter(|t| !t.is_empty()) else { continue };
            if out.contains_key(&stats) || !plain_item(l, i) || !l.string_of(i, "Level").is_some_and(|v| v.is_empty()) || l.int_of(i, "Flags").is_none_or(|f| f & WORN != 0) || bytes(l, c, "UUID").is_some_and(|u| u.len() != 16) || !s.copyable(item) {
                continue;
            }
            out.insert(stats, (item, i, c));
        }
    }
    out
}

/// An item node with nothing inside its machine, variables or statuses, and a name that
/// doesn't look like a key or a story item.
fn plain_item(l: &Lsf, i: usize) -> bool {
    let stats = l.string_of(i, "Stats").unwrap_or_default().to_lowercase();
    let story = stats.split('_').any(|w| w.starts_with("key") || matches!(w, "quest" | "story" | "tadpole" | "netherstone" | "orphic" | "astral"));
    !story && l.children(i).all(|c| matches!(l.node_name(c), "ItemMachine" | "VariableManager" | "StatusManager") && l.children(c).next().is_none())
}

/// Character nodes with their entity GUIDs (`Characters` and its `Creators` are in step).
fn characters(l: &Lsf) -> Vec<(usize, [u8; 16])> {
    let chars = l.at_path(&["Characters", "CharacterFactory", "Characters", "Character"]);
    let creators = l.at_path(&["Characters", "CharacterFactory", "Creators", "Creator"]);
    if chars.len() != creators.len() {
        return vec![];
    }
    chars.iter().zip(&creators).filter_map(|(&n, &c)| Some((n, bytes(l, c, "Entity")?.try_into().ok()?))).collect()
}

/// The `NewAge` attribute holding the entity store.
pub fn store_attr(l: &Lsf) -> Option<super::lsf::Attr> {
    let na = *l.at_path(&["NewAge"]).first()?;
    l.attr(na, "NewAge").filter(|a| a.ty == 25 && a.at + a.len <= l.d.len())
}

/// Adds items to the bag of the character at node `who`: each (stats name, copies), copied
/// from the party's items of that name. Returns the new unpacked `Globals.lsf`.
pub fn add_items(plain: &[u8], party: &[(String, usize)], who: usize, adds: &[(String, u32)]) -> Result<Vec<u8>, String> {
    let bad = |m: &str| format!("This save's item records aren't laid out the way adding items needs ({m}), so nothing was added.");
    let l = Lsf::read(plain).ok_or_else(|| bad("it doesn't read"))?;
    let a = store_attr(&l).ok_or_else(|| bad("no entity store"))?;
    let blob = &plain[a.at..a.at + a.len];
    let s = Store::read(blob).ok_or_else(|| bad("the entity store"))?;
    let chars = characters(&l);
    let (_, who_guid) = *chars.iter().find(|c| c.0 == who).ok_or_else(|| bad("the character"))?;
    let target = s.entity(&who_guid).ok_or_else(|| bad("the character's entity"))?;
    let offers = offers(&l, &s, party);
    let mut copies = vec![];
    for (stats, n) in adds {
        if *n == 0 {
            continue;
        }
        let o = offers.get(stats).ok_or_else(|| bad("an item to copy"))?;
        copies.push((Dup { source: o.0, target, copies: *n as usize }, o.1, o.2));
    }
    if copies.is_empty() {
        return Ok(plain.to_vec());
    }
    let list: Vec<Dup> = copies.iter().map(|c| c.0).collect();
    let (new_blob, news) = add(blob, &list).map_err(|e| bad(&e))?;
    let items_parent = l.at_path(&["Items", "ItemFactory", "Items"]).first().copied().ok_or_else(|| bad("items"))?;
    let creators_parent = l.at_path(&["Items", "ItemFactory", "Creators"]).first().copied().ok_or_else(|| bad("items"))?;
    let here = bytes(&l, who, "Translate").filter(|t| t.len() == 12).ok_or_else(|| bad("the character's position"))?.to_vec();
    let mut wr = Writer::new(plain).ok_or_else(|| bad("it can't be written"))?;
    wr.replace_value(&a, &new_blob)?;
    let mut uuids: HashSet<[u8; 16]> = HashSet::new();
    for (src, guid, _) in &news {
        let &(_, item, creator) = copies.iter().find(|c| c.0.source == *src).ok_or_else(|| bad("copies"))?;
        let uuid = random_guid(&uuids);
        uuids.insert(uuid);
        clone_node(&l, &mut wr, creator, creators_parent, &[], &|n| match n {
            "Entity" => Some(guid.to_vec()),
            "UUID" => Some(uuid.to_vec()),
            _ => None,
        })?;
        let flags = l.int_of(item, "Flags").unwrap_or(0) & !WORN;
        let flags_len = l.attr(item, "Flags").map_or(8, |f| f.len);
        clone_node(&l, &mut wr, item, items_parent, &[], &|n| match n {
            "Translate" => Some(here.clone()),
            "Flags" => Some(flags.to_le_bytes()[..flags_len.min(8)].to_vec()),
            _ => None,
        })?;
    }
    let out = wr.finish();
    // Read it all back.
    let l2 = Lsf::read(&out).ok_or_else(|| bad("the result doesn't read"))?;
    let a2 = store_attr(&l2).ok_or_else(|| bad("the result's entity store"))?;
    let creators = l2.at_path(&["Items", "ItemFactory", "Creators", "Creator"]);
    let items = l2.at_path(&["Items", "ItemFactory", "Items", "Item"]);
    if out[a2.at..a2.at + a2.len] != new_blob[..] || creators.len() != items.len() || items.len() != l.at_path(&["Items", "ItemFactory", "Items", "Item"]).len() + news.len() {
        return Err(bad("the result doesn't check out"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a small store: entities 0 (a character), 1 (its bag), 2 (an item in the bag),
    /// with the inventory records, an owner set and one plain component.
    pub(crate) fn store() -> Vec<u8> {
        // Components: Level, EntityId, MemberData, ContainerSlotData, Member, Owner,
        // Container, ContainerData, IsOwned, IsCurrentOwner, Plain.
        let names = [LEVEL, ENTITY_ID, MEMBER_DATA, SLOT_DATA, MEMBER, OWNER, CONTAINER, CONTAINER_DATA, "game.inventory.v1.IsOwnedComponent", OWNER_SETS[0], "game.test.v0.PlainComponent"];
        let elems = [32usize, 16, 16, 16, 8, 24, 32, 8, 8, 16, 16];
        let g = |e: u8| [e + 1; 16];
        let mut b = vec![0u8; 48];
        b.extend([0xFF; 8]);
        let mut cols: Vec<(usize, usize)> = vec![]; // (off, rows)
        let col = |b: &mut Vec<u8>, data: &[u8], elem: usize| {
            while b.len() % 8 != 0 {
                b.push(0);
            }
            let off = b.len();
            b.extend_from_slice(data);
            (off, data.len() / elem)
        };
        let level_at = b.len();
        cols.push(col(&mut b, &[0; 32], 32));
        // EntityId: table (0, 1, 2), then one reference copy per pointer: 3 → bag, 4 → char,
        // 5 → item, 6 → char, 7 → item, 8 → bag, 9 → bag.
        let eid: Vec<u8> = [0, 1, 2, 1, 0, 2, 0, 2, 1, 1].iter().flat_map(|&e| g(e)).collect();
        cols.push(col(&mut b, &eid, 16));
        let eid_off = cols[1].0;
        let eref = move |i: usize| rel(eid_off + 16 * i);
        let md: Vec<u8> = [eref(3), 0x7000_0000_0000].iter().flat_map(|v| v.to_le_bytes()).collect();
        cols.push(col(&mut b, &md, 16));
        let csd: Vec<u8> = [eref(5).to_le_bytes().to_vec(), 0u32.to_le_bytes().to_vec(), 0u32.to_le_bytes().to_vec()].concat();
        cols.push(col(&mut b, &csd, 16));
        let mdp = rel(cols[2].0);
        cols.push(col(&mut b, &mdp.to_le_bytes(), 8));
        // Heap: the owner's inventory list, the bag's keys and values, the owner set.
        let inv_list = b.len();
        b.extend(eref(8).to_le_bytes());
        let keys_at = b.len();
        b.extend(0u16.to_le_bytes());
        while b.len() % 8 != 0 {
            b.push(0);
        }
        let vals_at = b.len();
        b.extend(rel(cols[3].0).to_le_bytes());
        let set_at = b.len();
        b.extend(eref(7).to_le_bytes());
        let owner: Vec<u8> = [rel(inv_list), rel(inv_list + 8), eref(9)].iter().flat_map(|v| v.to_le_bytes()).collect();
        cols.push(col(&mut b, &owner, 24));
        let cont: Vec<u8> = [rel(keys_at), rel(keys_at + 2), rel(vals_at), rel(vals_at + 8)].iter().flat_map(|v| v.to_le_bytes()).collect();
        cols.push(col(&mut b, &cont, 32));
        cols.push(col(&mut b, &[1, 0, 0xF3, 0x92, 1, 0, 0, 0], 8));
        cols.push(col(&mut b, &eref(4).to_le_bytes(), 8));
        cols.push(col(&mut b, &[rel(set_at), rel(set_at + 8)].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(), 16));
        let plain: Vec<u8> = [eref(6).to_le_bytes(), 42u64.to_le_bytes()].concat();
        cols.push(col(&mut b, &plain, 16));
        // Entity lists, then the records.
        let owners: [(usize, &[u32]); 7] = [(4, &[2]), (5, &[0]), (6, &[1]), (7, &[1]), (8, &[1]), (9, &[0]), (10, &[2])];
        let mut lists = vec![];
        for (c, l) in owners {
            let at = b.len();
            for e in l {
                b.extend(e.to_le_bytes());
            }
            lists.push((c, at, l.len()));
        }
        while b.len() % 8 != 0 {
            b.push(0);
        }
        let rec_lo = b.len();
        for c in 0..names.len() {
            match lists.iter().find(|l| l.0 == c) {
                Some(&(_, at, k)) => b.extend([rel(at), rel(at + 4 * k), c as u64, k as u64].iter().flat_map(|v| v.to_le_bytes())),
                None => b.extend([FF, FF, FF, 0].iter().flat_map(|v| v.to_le_bytes())),
            }
        }
        let rec_hi = b.len();
        let lv: Vec<u8> = [rel(cols[1].0), rel(cols[1].0 + 48), rel(rec_lo), rel(rec_hi)].iter().flat_map(|v| v.to_le_bytes()).collect();
        b[level_at..level_at + 32].copy_from_slice(&lv);
        while b.len() % 16 != 0 {
            b.push(0);
        }
        let names_off = b.len();
        let mut no = vec![];
        for n in names {
            no.push(b.len() - names_off);
            b.extend(n.as_bytes());
        }
        let names_size = b.len() - names_off;
        while b.len() % 8 != 0 {
            b.push(0);
        }
        let desc_rel = b.len() - names_off;
        for (i, n) in names.iter().enumerate() {
            b.extend((no[i] as u64).to_le_bytes());
            b.extend((n.len() as u32).to_le_bytes());
            b.extend([0; 4]);
            b.extend([0; 8]);
            b.extend((elems[i] as u32).to_le_bytes());
            b.extend([0; 4]);
            b.extend((cols[i].1 as u64).to_le_bytes());
            b.extend(rel(cols[i].0).to_le_bytes());
        }
        b[0..8].copy_from_slice(b"LSMF\x01\x01\x00\x08");
        b[16..24].copy_from_slice(&rel(names_off).to_le_bytes());
        b[24..32].copy_from_slice(&(names_size as u64).to_le_bytes());
        b[32..36].copy_from_slice(&(desc_rel as u32).to_le_bytes());
        b[36..38].copy_from_slice(&(names.len() as u16).to_le_bytes());
        b
    }

    #[test]
    fn reads_the_inventory_web() {
        let b = store();
        let s = Store::read(&b).unwrap();
        assert_eq!(s.n, 3);
        assert_eq!(s.bag(0), Some(1));
        assert_eq!(s.bag_items(0), vec![2]);
        assert!(s.copyable(2));
        assert!(!s.copyable(0));
    }

    #[test]
    fn copies_an_item_into_the_bag() {
        let b = store();
        let (out, news) = add(&b, &[Dup { source: 2, target: 0, copies: 2 }]).unwrap();
        assert_eq!(news.len(), 2);
        let s = Store::read(&out).unwrap();
        assert_eq!(s.n, 5);
        assert_eq!(s.bag_items(0), vec![2, 3, 4]);
        let c = s.container(1).unwrap();
        assert_eq!(c.keys, vec![0, 1, 2]);
        assert_eq!(c.items.iter().map(|i| i.1).collect::<Vec<_>>(), vec![0, 1, 2]);
        // Lowest free key and the grid's extent.
        let (_, d) = s.named_row(1, CONTAINER_DATA).unwrap();
        assert_eq!((u16_at(d, 0), u32_at(d, 4)), (Some(3), Some(3)));
        // The plain component's reference to the holder still names the holder; its value is kept.
        for e in [3, 4] {
            let (_, row) = s.named_row(e, "game.test.v0.PlainComponent").unwrap();
            assert_eq!(s.referent(u64_at(row, 0).unwrap()).map(|r| r.0), Some(0));
            assert_eq!(u64_at(row, 8), Some(42));
            assert_eq!(s.member(e).map(|m| (m.1, m.2 & 0xFFFF)), Some((1, e as u64 - 2)));
        }
        let (_, set) = s.set(0, OWNER_SETS[0]).unwrap();
        assert_eq!(set.iter().map(|&m| s.referent(m).unwrap().0).collect::<Vec<_>>(), vec![2, 3, 4]);
        // An edited store can be edited again.
        let (again, _) = add(&out, &[Dup { source: 2, target: 0, copies: 1 }]).unwrap();
        assert_eq!(Store::read(&again).unwrap().bag_items(0), vec![2, 3, 4, 5]);
    }

    /// A `Globals.lsf` around `store()`: the character (entity 0, a player) and the item
    /// (entity 2) it carries.
    fn globals() -> (Vec<u8>, usize) {
        let g = |e: u8| vec![e + 1; 16];
        let s = |v: &str| [v.as_bytes(), &[0]].concat();
        let here: Vec<u8> = [1.5f32, 2.0, -3.25].iter().flat_map(|f| f.to_le_bytes()).collect();
        let nodes: Vec<(&str, i32, Vec<(&str, u32, Vec<u8>)>)> = vec![
            ("Characters", -1, vec![]),
            ("CharacterFactory", 0, vec![]),
            ("Characters", 1, vec![]),
            ("Character", 2, vec![("Translate", 12, here.clone()), ("Flags", 24, 0x13u64.to_le_bytes().to_vec())]),
            ("PlayerData", 3, vec![]),
            ("Creators", 1, vec![]),
            ("Creator", 5, vec![("Entity", 31, g(0))]),
            ("Items", -1, vec![]),
            ("ItemFactory", 7, vec![]),
            ("Items", 8, vec![]),
            ("Item", 9, vec![("Translate", 12, here), ("Flags", 24, 12u64.to_le_bytes().to_vec()), ("Level", 22, s("")), ("Stats", 22, s("WPN_Test_Sword"))]),
            ("ItemMachine", 10, vec![]),
            ("Creators", 8, vec![]),
            ("Creator", 12, vec![("Entity", 31, g(2)), ("TemplateType", 1, vec![0]), ("TemplateID", 31, vec![9; 16]), ("UUID", 31, vec![7; 16])]),
            ("NewAge", -1, vec![("NewAge", 25, store())]),
        ];
        (crate::fmt::lsf::unpack(&crate::fmt::lsf::tests::build(&nodes, true)).unwrap(), 3)
    }

    #[test]
    fn adds_items_to_the_save() {
        let (plain, who) = globals();
        let l = Lsf::read(&plain).unwrap();
        let party = vec![("Your character".to_string(), who)];
        let s = Store::read(&plain[store_attr(&l).unwrap().at..][..store_attr(&l).unwrap().len]).unwrap();
        assert_eq!(offers(&l, &s, &party).keys().collect::<Vec<_>>(), ["WPN_Test_Sword"]);
        let out = add_items(&plain, &party, who, &[("WPN_Test_Sword".into(), 2)]).unwrap();
        let l2 = Lsf::read(&out).unwrap();
        let items = l2.at_path(&["Items", "ItemFactory", "Items", "Item"]);
        let creators = l2.at_path(&["Items", "ItemFactory", "Creators", "Creator"]);
        assert_eq!((items.len(), creators.len()), (3, 3));
        let a = store_attr(&l2).unwrap();
        let s2 = Store::read(&out[a.at..a.at + a.len]).unwrap();
        for k in 1..3 {
            assert_eq!(l2.string_of(items[k], "Stats").as_deref(), Some("WPN_Test_Sword"));
            assert_eq!(bytes(&l2, items[k], "Translate"), bytes(&l2, who, "Translate"));
            let e = s2.entity(bytes(&l2, creators[k], "Entity").unwrap()).unwrap();
            assert_eq!(e, 2 + k);
            assert_ne!(bytes(&l2, creators[k], "UUID"), Some(&[7u8; 16][..]));
            assert_eq!(bytes(&l2, creators[k], "TemplateID"), Some(&[9u8; 16][..]));
        }
        assert_eq!(s2.bag_items(0), vec![2, 3, 4]);
        // Nothing chosen, nothing changed.
        assert_eq!(add_items(&plain, &party, who, &[]).unwrap(), plain);
    }

    #[test]
    fn refuses_what_it_cannot_copy() {
        let b = store();
        assert!(add(&b, &[Dup { source: 0, target: 0, copies: 1 }]).is_err());
        let mut bad = b.clone();
        bad[0] = b'X';
        assert!(add(&bad, &[Dup { source: 2, target: 0, copies: 1 }]).is_err());
    }
}
