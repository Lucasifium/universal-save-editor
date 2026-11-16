//! Baldur's Gate 3's packed entity store: the `NewAge` blob (magic "LSMF") inside a save's
//! `Globals.lsf`. Only what the editor needs is read: stack sizes.
//!
//! Header (little-endian): at 16 the directory offset and at 24 the size of the component
//! name block, which starts 48 bytes after the directory offset; at 32 (u32) the descriptor
//! table, relative to the name block; at 36 (u16) the number of component types. Each
//! descriptor is 48 bytes: name offset (u64, into the name block) and length (u32; newer
//! saves keep other bits in the next four bytes), a hash,
//! then at 24 the row size (u32), at 32 the row count and at 40 where the component's rows
//! start (u64 each; a pointer, like those below).
//!
//! Pointers stored inside rows are absolute offsets minus 48.
//!
//! The u64 at 8 is a checksum the game checks on load ("server shutting down" when it is
//! wrong): the first half of MurmurHash3 x64_128, both halves seeded with
//! 0xf0015eed5eedf001, over the bytes from 48 to 48 + directory offset + name block size.
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
const SEED: u64 = 0xf001_5eed_5eed_f001;

/// MurmurHash3 x64_128 with `seed` in both halves; the first half.
fn murmur3_h1(data: &[u8], seed: u64) -> u64 {
    const C1: u64 = 0x87c3_7b91_1142_53d5;
    const C2: u64 = 0x4cf5_ad43_2745_937f;
    fn fmix(mut k: u64) -> u64 {
        k ^= k >> 33;
        k = k.wrapping_mul(0xff51_afd7_ed55_8ccd);
        k ^= k >> 33;
        k = k.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        k ^ (k >> 33)
    }
    let (mut h1, mut h2) = (seed, seed);
    let mut blocks = data.chunks_exact(16);
    for b in &mut blocks {
        let k1 = u64::from_le_bytes(b[..8].try_into().unwrap());
        let k2 = u64::from_le_bytes(b[8..].try_into().unwrap());
        h1 ^= k1.wrapping_mul(C1).rotate_left(31).wrapping_mul(C2);
        h1 = h1.rotate_left(27).wrapping_add(h2).wrapping_mul(5).wrapping_add(0x52dc_e729);
        h2 ^= k2.wrapping_mul(C2).rotate_left(33).wrapping_mul(C1);
        h2 = h2.rotate_left(31).wrapping_add(h1).wrapping_mul(5).wrapping_add(0x3849_5ab5);
    }
    let tail = blocks.remainder();
    let mut pad = [0u8; 16];
    pad[..tail.len()].copy_from_slice(tail);
    if tail.len() > 8 {
        let k2 = u64::from_le_bytes(pad[8..].try_into().unwrap());
        h2 ^= k2.wrapping_mul(C2).rotate_left(33).wrapping_mul(C1);
    }
    if !tail.is_empty() {
        let k1 = u64::from_le_bytes(pad[..8].try_into().unwrap());
        h1 ^= k1.wrapping_mul(C1).rotate_left(31).wrapping_mul(C2);
    }
    let len = data.len() as u64;
    h1 ^= len;
    h2 ^= len;
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    h1 = fmix(h1);
    h2 = fmix(h2);
    h1.wrapping_add(h2)
}

/// The checksum a blob should carry at 8, if its header reads.
pub fn checksum(b: &[u8]) -> Option<u64> {
    if b.get(0..4)? != b"LSMF" {
        return None;
    }
    let end = u64_at(b, 16)?.checked_add(u64_at(b, 24)?)?.checked_add(BASE)?;
    Some(murmur3_h1(b.get(BASE as usize..usize::try_from(end).ok()?)?, SEED))
}

/// Writes the blob's checksum again after an edit.
pub fn fix_checksum(b: &mut [u8]) -> bool {
    match checksum(b) {
        Some(h) => {
            b[8..16].copy_from_slice(&h.to_le_bytes());
            true
        }
        None => false,
    }
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}
fn u64_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// One component type: row size, row count and where its rows start (in the blob).
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
        let Some(off) = usize::try_from(off).ok().and_then(|o| o.checked_add(BASE as usize)) else { continue };
        let c = Component { elem: elem as usize, rows: rows as usize, off };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_murmur3_over_the_data_and_directory() {
        // Header: directory at 60, name block 15 bytes, so the hash covers 75 bytes from 48.
        let mut b = b"LSMF".to_vec();
        b.extend([0u8; 12]);
        b.extend(60u64.to_le_bytes());
        b.extend(15u64.to_le_bytes());
        b.extend([0u8; 16]);
        b.extend((0..75u32).map(|i| (i * 7 % 256) as u8));
        // Reference value from a separate MurmurHash3 x64_128 implementation.
        assert_eq!(checksum(&b), Some(0x920a_aab5_04a6_9a2a));
        assert!(fix_checksum(&mut b));
        assert_eq!(&b[8..16], &0x920a_aab5_04a6_9a2au64.to_le_bytes());
        // The bytes it covers change it; the header's own bytes don't.
        b[100] ^= 1;
        assert_ne!(checksum(&b), Some(0x920a_aab5_04a6_9a2a));
        assert_eq!(checksum(b"LSMX"), None);
    }
}
