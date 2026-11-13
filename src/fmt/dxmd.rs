//! Deus Ex: Mankind Divided saves (`DXNGsavegame0000.dat` ...).
//!
//! ```text
//! 0x00  "SGD6", u16 1, u16 entry count, u32 size of everything after this field up to the
//!       end of the last entry
//! 0x0c  entries of (u64 index, u64 offset, u64 size); entry 4 is the zlib-packed game state
//! end   the save's type table, stored as it is (names, then each type's members by the CRC-32
//!       of their names)
//! ```
//!
//! The game state starts with a few singletons and then one record per scene context:
//! u64 id, u64, u32, u32, a u16-length name, u64, then the static and the dynamic entities.
//! Each entity is a u64 resource id, three u32, a u16-length name, a u64-counted list of
//! sub-entities (u32, u32 path and u32 start in the value list) and a u64-counted list of u32
//! values. The values of a sub-entity follow its class's stored save order.
//!
//! The global scene holds the player: its sub-entity for experience keeps
//! [XP toward the next Praxis kit, Praxis kits, total XP, level], and the inventory's
//! sub-entity keeps the credits third. Inventory stacks are dynamic entities of the global
//! scene whose first value is the count and fourth the inventory that holds them; the
//! player's inventory is the one most stacks point at. Values are changed in place, so the
//! game state keeps its length and only its packed size and the header sizes change.
//! Worked out from saves.

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};

pub const KIND: &str = "Deus Ex: Mankind Divided save";

/// Id of the global scene context.
const GLOBAL: u64 = 2_866_095_916;
/// Resource id of the entity that holds the player's progression and inventory component.
const PLAYER: u64 = 50_714_423_580_617_650;
/// Sub-entity paths inside it.
const XP: (u32, u32) = (2_482_625_284, 1_791_586_679);
const INVENTORY: (u32, u32) = (580_489_732, 1_466_939_605);
/// Resource ids of the stack templates: ammunition, and the group holding crafting parts.
const AMMO: u64 = 28_436_858_646_147_798;
const PARTS: u64 = 58_420_311_722_090_081;

/// Most crafting parts a stack holds: 999 by community reports (an estimate).
pub const PARTS_CAP: i64 = 999;
const CREDITS_MAX: i64 = 999_999_999;
const PRAXIS_MAX: i64 = 999;
const AMMO_MAX: i64 = 9_999;

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}
fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}
fn u64_at(d: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(o..o + 8)?.try_into().ok()?))
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 12 + 24 * 5 && d.starts_with(b"SGD6\x01\x00")
}

/// Where the packed game state is: (table slot, offset, size).
fn state_entry(d: &[u8]) -> Option<(usize, usize, usize)> {
    let n = u16_at(d, 6)? as usize;
    if !(5..=16).contains(&n) {
        return None;
    }
    let mut end = 0usize;
    let mut found = None;
    for k in 0..n {
        let e = 12 + 24 * k;
        let (idx, off, size) = (u64_at(d, e)?, u64_at(d, e + 8)? as usize, u64_at(d, e + 16)? as usize);
        let stop = off.checked_add(size)?;
        if stop > d.len() || off < 12 + 24 * n {
            return None;
        }
        end = end.max(stop);
        if idx == 4 {
            found = Some((k, off, size));
        }
    }
    let (k, off, size) = found?;
    // The game state must be the last entry and the header's size must match.
    (off + size == end && u32_at(d, 8)? as usize == end - 12).then_some((k, off, size))
}

struct Entity {
    rid: u64,
    subs: Vec<(u32, u32, u32)>,
    /// Byte offset of the first value, and the number of values.
    vals: usize,
    count: usize,
}

fn entity(b: &[u8], mut o: usize) -> Option<(usize, Entity)> {
    let rid = u64_at(b, o)?;
    o += 20;
    o += 2 + u16_at(b, o)? as usize;
    let n = u64_at(b, o)? as usize;
    o += 8;
    if n > b.len() / 12 {
        return None;
    }
    let mut subs = Vec::with_capacity(n);
    for k in 0..n {
        let s = o + 12 * k;
        subs.push((u32_at(b, s)?, u32_at(b, s + 4)?, u32_at(b, s + 8)?));
    }
    o += 12 * n;
    let count = u64_at(b, o)? as usize;
    o += 8;
    if count > b.len() / 4 || o + 4 * count > b.len() {
        return None;
    }
    let vals = o;
    Some((vals + 4 * count, Entity { rid, subs, vals, count }))
}

fn entities(b: &[u8], mut o: usize) -> Option<(usize, Vec<Entity>)> {
    let n = u64_at(b, o)? as usize;
    o += 8;
    if n > b.len() / 40 {
        return None;
    }
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let (next, e) = entity(b, o)?;
        out.push(e);
        o = next;
    }
    Some((o, out))
}

/// The static and dynamic entities of the scene context starting at `o`.
fn scene(b: &[u8], mut o: usize) -> Option<(Vec<Entity>, Vec<Entity>)> {
    if u64_at(b, o)? != GLOBAL {
        return None;
    }
    o += 24;
    o += 2 + u16_at(b, o)? as usize + 8;
    let (o, st) = entities(b, o)?;
    let (_, dy) = entities(b, o)?;
    Some((st, dy))
}

/// Byte offsets of the values the screen shows.
#[derive(Clone, Debug, Default)]
struct Spots {
    /// XP toward the next kit, Praxis kits, total XP, level.
    xp: usize,
    credits: usize,
    parts: Option<usize>,
    ammo: Vec<usize>,
}

fn val(b: &[u8], e: &Entity, i: usize) -> Option<u32> {
    (i < e.count).then(|| u32_at(b, e.vals + 4 * i)).flatten()
}

fn locate(b: &[u8]) -> Option<Spots> {
    let key = GLOBAL.to_le_bytes();
    let mut from = 0;
    while let Some(p) = b.get(from..)?.windows(8).position(|w| w == key) {
        let at = from + p;
        from = at + 1;
        let Some((st, dy)) = scene(b, at) else { continue };
        let Some(player) = st.iter().find(|e| e.rid == PLAYER) else { continue };
        let start = |path: (u32, u32)| player.subs.iter().find(|s| (s.0, s.1) == path).map(|s| s.2 as usize);
        let (xp, inv) = (start(XP)?, start(INVENTORY)?);
        if xp + 4 > player.count || inv + 3 > player.count {
            return None;
        }
        // Inventory stacks: one sub-entity, 6 or 7 values, the second and fifth flags.
        let stacks: Vec<&Entity> = dy
            .iter()
            .filter(|e| {
                e.subs.len() == 1
                    && (e.count == 6 || e.count == 7)
                    && val(b, e, 1).is_some_and(|v| v <= 1)
                    && val(b, e, 4).is_some_and(|v| v <= 1)
            })
            .collect();
        let mut owners: HashMap<u32, usize> = HashMap::new();
        for e in &stacks {
            *owners.entry(val(b, e, 3)?).or_default() += 1;
        }
        let owner = owners.iter().max_by_key(|(k, n)| (**n, std::cmp::Reverse(**k))).map(|(k, _)| *k);
        let mine = |rid: u64| stacks.iter().filter(move |e| e.rid == rid && owner.is_some() && val(b, e, 3) == owner);
        let parts = mine(PARTS).max_by_key(|e| val(b, e, 0).unwrap_or(0)).map(|e| e.vals);
        let ammo = mine(AMMO).map(|e| e.vals).collect();
        return Some(Spots { xp: player.vals + 4 * xp, credits: player.vals + 4 * (inv + 2), parts, ammo });
    }
    None
}

fn inflate(d: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(d).take(256 << 20).read_to_end(&mut out).ok()?;
    Some(out)
}

#[derive(Clone, Debug)]
struct Save {
    file: Vec<u8>,
    slot: usize,
    off: usize,
    size: usize,
    state: Vec<u8>,
    spots: Spots,
}

fn open(d: &[u8]) -> Option<Save> {
    if !looks(d) {
        return None;
    }
    let (slot, off, size) = state_entry(d)?;
    let state = inflate(&d[off..off + size])?;
    let spots = locate(&state)?;
    Some(Save { file: d.to_vec(), slot, off, size, state, spots })
}

/// Editable leaves: credits, Praxis kits, crafting parts, then each ammo stack.
const CREDITS: usize = 0;
const PRAXIS: usize = 1;
const PARTS_LEAF: usize = 2;
const AMMO0: usize = 3;

impl Save {
    fn spot(&self, leaf: usize) -> Option<usize> {
        match leaf {
            CREDITS => Some(self.spots.credits),
            PRAXIS => Some(self.spots.xp + 4),
            PARTS_LEAF => self.spots.parts,
            n => self.spots.ammo.get(n - AMMO0).copied(),
        }
    }
    fn get(&self, at: usize) -> u32 {
        u32_at(&self.state, at).unwrap_or(0)
    }
}

fn max_of(leaf: usize) -> i64 {
    match leaf {
        CREDITS => CREDITS_MAX,
        PRAXIS => PRAXIS_MAX,
        PARTS_LEAF => PARTS_CAP,
        _ => AMMO_MAX,
    }
}

#[derive(Clone, Debug)]
struct B(Save);

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let s = &self.0;
        if changes.is_empty() {
            return Ok(s.file.clone());
        }
        let mut state = s.state.clone();
        for (leaf, v) in changes {
            self.check(*leaf, v)?;
            let at = s.spot(*leaf).ok_or("Not editable")?;
            let n = v.as_i64().ok_or("Must be a whole number")? as u32;
            state[at..at + 4].copy_from_slice(&n.to_le_bytes());
        }
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&state).map_err(|e| e.to_string())?;
        let packed = z.finish().map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(s.file.len() + packed.len());
        out.extend_from_slice(&s.file[..s.off]);
        out.extend_from_slice(&packed);
        out.extend_from_slice(&s.file[s.off + s.size..]);
        let e = 12 + 24 * s.slot;
        out[e + 16..e + 24].copy_from_slice(&(packed.len() as u64).to_le_bytes());
        let end = s.off + packed.len();
        let total = u32::try_from(end - 12).map_err(|_| "Save too large")?;
        out[8..12].copy_from_slice(&total.to_le_bytes());
        Ok(out)
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let max = max_of(leaf);
        match v.as_i64() {
            Some(n) if (0..=max).contains(&n) => Ok(()),
            _ => Err(format!("Must be a whole number from 0 to {max}")),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let s = open(d)?;
    let mut edit = HashMap::new();
    let mut adam = Map::new();
    adam.insert("Credits".into(), Value::from(s.get(s.spots.credits) as i32));
    edit.insert("/Adam/Credits".to_string(), CREDITS);
    adam.insert("Praxis kits".into(), Value::from(s.get(s.spots.xp + 4)));
    edit.insert("/Adam/Praxis kits".to_string(), PRAXIS);
    adam.insert("XP toward next Praxis kit".into(), Value::from(s.get(s.spots.xp)));
    adam.insert("Total XP".into(), Value::from(s.get(s.spots.xp + 8)));
    let mut root = Map::new();
    root.insert("Adam".into(), Value::Object(adam));
    if let Some(at) = s.spots.parts {
        let mut m = Map::new();
        m.insert("Crafting parts".into(), Value::from(s.get(at)));
        edit.insert("/Crafting/Crafting parts".to_string(), PARTS_LEAF);
        root.insert("Crafting".into(), Value::Object(m));
    }
    if !s.spots.ammo.is_empty() {
        let mut m = Map::new();
        for (i, &at) in s.spots.ammo.iter().enumerate() {
            let name = format!("Ammo stack {}", i + 1);
            m.insert(name.clone(), Value::from(s.get(at)));
            edit.insert(format!("/Ammo/{name}"), AMMO0 + i);
        }
        root.insert("Ammo".into(), Value::Object(m));
    }
    Some(TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B(s))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(rid: u64, subs: &[(u32, u32, u32)], vals: &[u32]) -> Vec<u8> {
        let mut v = rid.to_le_bytes().to_vec();
        v.extend([0u8; 12]);
        v.extend(0u16.to_le_bytes());
        v.extend((subs.len() as u64).to_le_bytes());
        for s in subs {
            v.extend(s.0.to_le_bytes());
            v.extend(s.1.to_le_bytes());
            v.extend(s.2.to_le_bytes());
        }
        v.extend((vals.len() as u64).to_le_bytes());
        for x in vals {
            v.extend(x.to_le_bytes());
        }
        v
    }

    /// A save with the player entity, two stacks of each kind in the player's inventory and
    /// one stack held elsewhere.
    fn sample() -> Vec<u8> {
        let mut st = vec![0u8; 40];
        st.extend(GLOBAL.to_le_bytes());
        st.extend([0u8; 16]);
        st.extend(4u16.to_le_bytes());
        st.extend(b"glob");
        st.extend(0u64.to_le_bytes());
        st.extend(1u64.to_le_bytes());
        let mut vals = vec![0u32; 12];
        vals[2..6].copy_from_slice(&[1815, 11, 148_815, 27]);
        vals[8] = 9_958_111;
        st.extend(ent(PLAYER, &[(XP.0, XP.1, 2), (INVENTORY.0, INVENTORY.1, 6)], &vals));
        let stacks = [
            (PARTS, [3, 0, 1, 631, 1, 9]),
            (PARTS, [107, 0, 2, 631, 1, 9]),
            (AMMO, [18, 0, 3, 631, 1, 9]),
            (AMMO, [132, 0, 4, 631, 1, 9]),
            (AMMO, [500, 0, 5, 700, 1, 9]),
        ];
        st.extend((stacks.len() as u64).to_le_bytes());
        for (rid, v) in stacks {
            st.extend(ent(rid, &[(0, 0, 0)], &v));
        }
        st.extend([0u8; 12]);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        z.write_all(&st).unwrap();
        let packed = z.finish().unwrap();
        let mut f = b"SGD6\x01\x00\x05\x00\x00\x00\x00\x00".to_vec();
        let mut off = 12 + 24 * 5;
        let mut body = Vec::new();
        for k in 0..4u64 {
            f.extend(k.to_le_bytes());
            f.extend((off as u64).to_le_bytes());
            f.extend(3u64.to_le_bytes());
            body.extend([k as u8; 3]);
            off += 3;
        }
        f.extend(4u64.to_le_bytes());
        f.extend((off as u64).to_le_bytes());
        f.extend((packed.len() as u64).to_le_bytes());
        f.extend(body);
        f.extend(&packed);
        let total = (f.len() - 12) as u32;
        f[8..12].copy_from_slice(&total.to_le_bytes());
        f.extend(b"type table");
        f
    }

    #[test]
    fn reads_and_edits_values() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Adam/Credits").unwrap(), 9_958_111);
        assert_eq!(doc.view.pointer("/Adam/Praxis kits").unwrap(), 11);
        assert_eq!(doc.view.pointer("/Adam/Total XP").unwrap(), 148_815);
        assert_eq!(doc.view.pointer("/Crafting/Crafting parts").unwrap(), 107);
        assert_eq!(doc.view.pointer("/Ammo/Ammo stack 1").unwrap(), 18);
        assert!(doc.view.pointer("/Ammo/Ammo stack 3").is_none());
        doc.set("/Adam/Credits", Value::from(123_456)).unwrap();
        doc.set("/Adam/Praxis kits", Value::from(40)).unwrap();
        doc.set("/Crafting/Crafting parts", Value::from(999)).unwrap();
        doc.set("/Ammo/Ammo stack 2", Value::from(300)).unwrap();
        assert!(doc.set("/Crafting/Crafting parts", Value::from(1000)).is_err());
        let out = doc.to_bytes().unwrap();
        assert!(out.ends_with(b"type table"));
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Adam/Credits").unwrap(), 123_456);
        assert_eq!(back.view.pointer("/Adam/Praxis kits").unwrap(), 40);
        assert_eq!(back.view.pointer("/Adam/Total XP").unwrap(), 148_815);
        assert_eq!(back.view.pointer("/Crafting/Crafting parts").unwrap(), 999);
        assert_eq!(back.view.pointer("/Ammo/Ammo stack 1").unwrap(), 18);
        assert_eq!(back.view.pointer("/Ammo/Ammo stack 2").unwrap(), 300);
    }

    /// Reads real saves named in `DXMD_SAVES` (separated by `|`), edits every value and
    /// checks that everything else in the game state is unchanged.
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(list) = std::env::var("DXMD_SAVES") else { return };
        for p in list.split('|') {
            let d = std::fs::read(p).unwrap();
            let mut doc = parse(&d).unwrap_or_else(|| panic!("{p}"));
            println!("{p}: {}", doc.view);
            assert_eq!(doc.to_bytes().unwrap(), d);
            doc.set("/Adam/Credits", Value::from(1_000_000)).unwrap();
            doc.set("/Crafting/Crafting parts", Value::from(PARTS_CAP)).unwrap();
            let out = doc.to_bytes().unwrap();
            let (a, b) = (open(&d).unwrap(), open(&out).unwrap());
            assert_eq!(a.state.len(), b.state.len());
            let diff = a.state.iter().zip(&b.state).filter(|(x, y)| x != y).count();
            assert!(diff <= 8, "{diff} bytes changed");
            let back = parse(&out).unwrap();
            assert_eq!(back.view.pointer("/Adam/Credits").unwrap(), 1_000_000);
            assert_eq!(back.view.pointer("/Crafting/Crafting parts").unwrap(), PARTS_CAP);
        }
    }
}
