//! Baldur's Gate 3's packed entity store: the `NewAge` blob (magic "LSMF") inside a save's
//! `Globals.lsf`. Only what the editor needs is read: stack sizes.
//!
//! Header (little-endian): at 16 the directory offset and at 24 the size of the component
//! name block, which starts 48 bytes after the directory offset; at 32 (u32) the descriptor
//! table, relative to the name block; at 36 (u16) the number of component types. Each
//! descriptor is 48 bytes: name offset (u64, into the name block) and length (u32; newer
//! saves keep other bits in the next four bytes), a hash,
//! then at 24 the row size (u32), at 32 the row count and at 40 the absolute offset of the
//! component's rows (u64 each).
//!
//! Pointers stored inside rows are absolute offsets minus 48.
//!
//! - `core.v0.EntityId` rows are entity GUIDs (16 bytes); the LSF `Creators` table gives each
//!   `Item` node its entity GUID.
//! - Each `game.inventory.v0.NewStackComponent` row is a pointer to a stack record:
//!   `{members begin, members end, entries begin, entries end}` (u64 pointers). Members are
//!   pointers to `EntityId` rows; entries are 8-byte `{u16 member index, u16, u32 amount}` in
//!   `game.inventory.v0.StackEntry`. A member's stack size is the sum of its entries; an item
//!   without an entry is a single item.
//!
//! Layout facts from public research on BG3 saves (the unlicensed bg3-savefile-parser notes,
//! used as facts only), checked here against public sample saves.

use std::collections::HashMap;

const BASE: u64 = 48;

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}
fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// One component type: row size, row count and where its rows start.
#[derive(Clone, Copy, Debug)]
pub struct Component {
    pub elem: usize,
    pub rows: usize,
    pub off: usize,
}

/// The component types of a blob, by name.
pub fn components(b: &[u8]) -> Option<HashMap<String, Component>> {
    if b.get(0..4)? != b"LSMF" {
        return None;
    }
    let dir = u64_at(b, 16)?;
    let names_size = u64_at(b, 24)? as usize;
    let names_off = usize::try_from(dir.checked_add(BASE)?).ok()?;
    let desc_rel = u32_at(b, 32)? as usize;
    let count = u16_at(b, 36)? as usize;
    if names_off >= b.len() || count == 0 || count >= 4000 {
        return None;
    }
    let names = b.get(names_off..names_off.checked_add(names_size)?)?;
    let desc = names_off.checked_add(desc_rel)?;
    let mut out = HashMap::new();
    for i in 0..count {
        let at = desc + i * 48;
        let (Some(no), Some(nl), Some(elem), Some(rows), Some(off)) = (u64_at(b, at), u32_at(b, at + 8).map(u64::from), u32_at(b, at + 24), u64_at(b, at + 32), u64_at(b, at + 40)) else { break };
        let (no, nl) = (no as usize, nl as usize);
        if nl == 0 || nl > 200 {
            continue;
        }
        let Some(name) = names.get(no..no + nl).and_then(|s| std::str::from_utf8(s).ok()) else { continue };
        let c = Component { elem: elem as usize, rows: rows as usize, off: off as usize };
        if c.rows.checked_mul(c.elem).and_then(|n| n.checked_add(c.off)).is_none_or(|end| end > b.len()) {
            continue;
        }
        out.entry(name.to_string()).or_insert(c);
    }
    Some(out)
}

/// Where one item's stack size is kept: the offsets (in the blob) of its entries' amounts.
#[derive(Clone, Debug, Default)]
pub struct Stack {
    pub amounts: Vec<usize>,
}

impl Stack {
    pub fn total(&self, b: &[u8]) -> u64 {
        self.amounts.iter().filter_map(|&a| u32_at(b, a)).map(u64::from).sum()
    }
}

/// Stack sizes by entity GUID (the 16 bytes as stored).
pub fn stacks(b: &[u8]) -> HashMap<[u8; 16], Stack> {
    let mut out: HashMap<[u8; 16], Stack> = HashMap::new();
    let Some(c) = components(b) else { return out };
    let (Some(ns), Some(se), Some(eid)) = (c.get("game.inventory.v0.NewStackComponent"), c.get("game.inventory.v0.StackEntry"), c.get("core.v0.EntityId")) else { return out };
    if eid.elem != 16 || ns.elem < 8 || se.elem != 8 {
        return out;
    }
    let ptr = |v: u64| -> Option<usize> { usize::try_from(v.checked_add(BASE)?).ok().filter(|&p| p < b.len()) };
    let (se0, se1) = (se.off, se.off + se.rows * 8);
    let (e0, e1) = (eid.off, eid.off + eid.rows * 16);
    for k in 0..ns.rows {
        let Some(rec) = u64_at(b, ns.off + k * ns.elem).and_then(ptr) else { continue };
        let (Some(mlo), Some(mhi), Some(slo), Some(shi)) = (u64_at(b, rec), u64_at(b, rec + 8), u64_at(b, rec + 16), u64_at(b, rec + 24)) else { continue };
        if mhi <= mlo || (mhi - mlo) % 8 != 0 || (mhi - mlo) / 8 > 256 {
            continue;
        }
        let n = ((mhi - mlo) / 8) as usize;
        let Some(m0) = ptr(mlo) else { continue };
        let (Some(a0), Some(a1)) = (ptr(slo), shi.checked_add(BASE).and_then(|v| usize::try_from(v).ok())) else { continue };
        if !(se0 <= a0 && a0 < a1 && a1 <= se1) || (a1 - a0) % 8 != 0 {
            continue;
        }
        let members: Vec<Option<[u8; 16]>> = (0..n)
            .map(|i| {
                let a = u64_at(b, m0 + 8 * i).and_then(ptr)?;
                (a >= e0 && a < e1 && (a - e0) % 16 == 0).then(|| b[a..a + 16].try_into().ok()).flatten()
            })
            .collect();
        let mut j = a0;
        while j < a1 {
            let i = u16_at(b, j).unwrap_or(u16::MAX) as usize;
            if let Some(Some(g)) = members.get(i) {
                out.entry(*g).or_default().amounts.push(j + 4);
            }
            j += 8;
        }
    }
    out
}
