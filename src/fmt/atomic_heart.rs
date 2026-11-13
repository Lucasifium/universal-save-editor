//! Atomic Heart `<n>.sav` (SaveExtension plugin slot files, Unreal 4.27).
//!
//! ```text
//! u32 1, u32 2, GVAS-style header (engine version, custom versions)   no "GVAS" magic
//! FString /Script/SaveExtension.SlotInfo, then its properties         readable as they are
//! FString /Script/SaveExtension.SlotData, FString "Oodle",
//! u32 n, then n bytes of blocks, each block holding up to 128 KiB:
//!   i64 0x9E2A83C1, i64 131072, i64 packed, i64 raw, i64 packed, i64 raw, packed bytes (Oodle)
//! 20 bytes: SHA-1 of everything before them
//! ```
//!
//! The blocks are unpacked with the Oodle library the game itself installs (see `oodle.rs`);
//! packing again re-blocks the stream, rewrites the length and makes the SHA-1 again.

use sha1::{Digest, Sha1};

const MARKER: &[u8] = b"/Script/SaveExtension.SlotData";
const TAG: i64 = 0x9E2A_83C1;
const CHUNK: usize = 131_072;

/// The file around the unpacked stream: everything before the block length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    head: Vec<u8>,
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 4096
        && d.starts_with(&[1, 0, 0, 0, 2, 0, 0, 0])
        && d[..4096.min(d.len())].windows(30).any(|w| w == b"/Script/SaveExtension.SlotInfo")
}

fn i64_at(d: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Where the block length is: right after the "Oodle" compressor name that follows the SlotData class.
fn start(d: &[u8]) -> Option<usize> {
    let m = d.windows(MARKER.len()).position(|w| w == MARKER)? + MARKER.len() + 1;
    let name = b"\x06\x00\x00\x00Oodle\x00";
    (d.get(m..m + name.len())? == name).then_some(m + name.len())
}

/// Splits the file into the part before the blocks and the unpacked stream.
pub fn unpack(d: &[u8]) -> Result<(Layout, Vec<u8>), String> {
    if !looks(d) {
        return Err("not an Atomic Heart save".into());
    }
    let at = start(d).ok_or("this slot file's data isn't Oodle-packed")?;
    let total = u32::from_le_bytes(d.get(at..at + 4).ok_or("cut off")?.try_into().unwrap()) as usize;
    let first = at + 4;
    let end = first.checked_add(total).filter(|&e| e + 20 == d.len()).ok_or("the save's block length doesn't match its size")?;
    if Sha1::digest(&d[..end]).as_slice() != &d[end..] {
        return Err("the save's SHA-1 doesn't match".into());
    }
    let mut blocks: Vec<(&[u8], usize)> = vec![];
    let mut p = first;
    while p < end {
        let (tag, size, sc, su, cc, cu) = (i64_at(d, p), i64_at(d, p + 8), i64_at(d, p + 16), i64_at(d, p + 24), i64_at(d, p + 32), i64_at(d, p + 40));
        let (Some(tag), Some(size), Some(sc), Some(su), Some(cc), Some(cu)) = (tag, size, sc, su, cc, cu) else { return Err("cut off".into()) };
        if tag != TAG || size != CHUNK as i64 || sc != cc || su != cu || cc < 0 || !(0..=CHUNK as i64).contains(&cu) {
            return Err("unexpected block header".into());
        }
        let body = d.get(p + 48..p + 48 + cc as usize).filter(|_| p + 48 + cc as usize <= end).ok_or("cut off")?;
        blocks.push((body, cu as usize));
        p += 48 + cc as usize;
    }
    let mut out = Vec::with_capacity(blocks.len() * CHUNK);
    for b in crate::par::map(&blocks, |&(b, n)| crate::oodle::decompress(b, n)) {
        out.extend(b.ok_or("Oodle couldn't unpack the save's data")?);
    }
    Ok((Layout { head: d[..at].to_vec() }, out))
}

pub fn pack(l: &Layout, stream: &[u8]) -> Result<Vec<u8>, String> {
    let parts: Vec<&[u8]> = stream.chunks(CHUNK).collect();
    let mut blocks = vec![];
    for p in crate::par::map(&parts, |p| crate::oodle::compress(p)) {
        blocks.push(p.ok_or("Oodle couldn't pack the save again (no Oodle library found)")?);
    }
    let mut body = vec![];
    for (raw, packed) in parts.iter().zip(&blocks) {
        for v in [TAG, CHUNK as i64, packed.len() as i64, raw.len() as i64, packed.len() as i64, raw.len() as i64] {
            body.extend(v.to_le_bytes());
        }
        body.extend(packed);
    }
    let mut out = l.head.clone();
    out.extend((body.len() as u32).to_le_bytes());
    out.extend(body);
    let sum = Sha1::digest(&out);
    out.extend(sum);
    Ok(out)
}

// ---------------------------------------------------------------------------- the stream

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Atomic Heart save";

/// The unpacked stream starts with its own length, then the first property (`Map`).
pub fn stream_looks(d: &[u8]) -> bool {
    d.len() > 4096
        && i32_at(d, 0) == Some(d.len() as i32 - 4)
        && d[4..].starts_with(b"\x04\x00\x00\x00Map\x00\x0d\x00\x00\x00NameProperty\x00")
        && d.windows(24).take(65536).any(|w| w == b"/Script/AtomicHeart.AHWo")
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The inventory lists of the player (the first set that follows `BP_PlayerInventory_C`):
/// (list name, group shown on the screen).
const LISTS: [(&str, &str); 2] = [("ResourcesList", "Resources"), ("SpecialResourcesList", "Special")];

/// (asset name, name shown, plain crafting material the max button may raise).
pub const ITEMS: &[(&str, &str, bool)] = &[
    ("DA_Item_Resources_Metal_Parts", "Metal parts", true),
    ("DA_Item_Synthetic_Material_Resource", "Synthetic material", true),
    ("DA_Item_Resource_Biomaterial", "Biomaterial", true),
    ("DA_Item_Resource_Superconductor", "Superconductors", true),
    ("DA_Item_Resources_Chemicals", "Chemicals", true),
    ("DA_Item_Energy_Cell_Resource", "Energy cells", true),
    ("DA_Item_Resource_Microelectronics", "Microelectronics", true),
    ("DA_Item_Resource_NeuralModule", "Neural modules", false),
    ("DA_Item_Jelly_Resource", "Neuropolymer", false),
];

/// One stack: where its count is, and which item it is.
pub struct Stack {
    pub group: &'static str,
    pub name: &'static str,
    pub at: usize,
}

/// Finds the player's stacks of resources and of the special resource (polymer jelly).
pub fn scan(d: &[u8]) -> Vec<Stack> {
    let Some(inv) = d.windows(22).position(|w| w == b"BP_PlayerInventory_C_0") else { return vec![] };
    let amount: &[u8] = b"\x07\x00\x00\x00Amount\x00\x0c\x00\x00\x00IntProperty\x00\x04\x00\x00\x00\x00\x00\x00\x00\x00";
    let mut out = vec![];
    for (list, group) in LISTS {
        let mut tag = vec![];
        tag.extend((list.len() as u32 + 1).to_le_bytes());
        tag.extend(list.as_bytes());
        tag.extend(b"\x00\x0f\x00\x00\x00StructProperty\x00");
        let Some(at) = d[inv..].windows(tag.len()).position(|w| w == tag.as_slice()).map(|p| p + inv) else { continue };
        let Some(size) = i32_at(d, at + tag.len()).filter(|s| *s > 0) else { continue };
        let (start, end) = (at + tag.len(), (at + tag.len() + size as usize + 16).min(d.len()));
        let mut prev = start;
        let mut p = start;
        while let Some(i) = d[p..end].windows(amount.len()).position(|w| w == amount) {
            let a = p + i;
            let seg = &d[prev..a];
            if let Some(k) = seg.windows(14).rposition(|w| w == b"ItemDataAsset\0") {
                let path = &seg[k + 14..];
                if let Some(s) = path.windows(6).position(|w| w == b"/Game/") {
                    let text = String::from_utf8_lossy(&path[s..]);
                    let asset = text.split('\0').next().unwrap_or("").rsplit('.').next().unwrap_or("").to_string();
                    if let Some((_, name, _)) = ITEMS.iter().find(|(id, ..)| *id == asset) {
                        if a + amount.len() + 4 <= d.len() {
                            out.push(Stack { group, name, at: a + amount.len() });
                        }
                    }
                }
            }
            p = a + amount.len() + 4;
            prev = p;
        }
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !stream_looks(d) {
        return Err("not an Atomic Heart save stream".into());
    }
    let stacks = scan(d);
    if stacks.is_empty() {
        return Err("no inventory found in this save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Save", "Stacks found"], stacks.len());
    let mut seen = std::collections::HashSet::new();
    for s in &stacks {
        // The first stack of each item is the one the screen edits.
        if seen.insert(s.name) {
            b.ranged(&[s.group, s.name], s.at, Kind::I32, 0, 99_999);
        }
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs(t: &str) -> Vec<u8> {
        let mut v = (t.len() as i32 + 1).to_le_bytes().to_vec();
        v.extend(t.as_bytes());
        v.push(0);
        v
    }

    /// A small stream shaped like the real one: the player's inventory actor, then a
    /// resources list whose slots hold an item path, an Amount and an ItemId.
    fn stream(slots: &[(&str, i32)]) -> Vec<u8> {
        let mut b = vec![0u8; 4];
        b.extend(fs("Map"));
        b.extend(fs("NameProperty"));
        b.extend([0u8; 40]);
        b.extend(fs("/Script/AtomicHeart.AHWorldSettings"));
        b.extend(vec![0u8; 4096]);
        b.extend(fs("BP_PlayerInventory_C_0"));
        for (list, group) in [("ResourcesList", &slots[..slots.len().min(2)]), ("SpecialResourcesList", &slots[slots.len().min(2)..])] {
            let mut body = vec![];
            for (asset, n) in group {
                body.extend(fs("ItemDataAsset"));
                body.extend(fs("ObjectProperty"));
                body.extend([0u8; 9]);
                body.extend(fs(&format!("/Game/Core/Objects/Items/Resourses/{asset}.{asset}")));
                body.extend(b"\x07\x00\x00\x00Amount\x00\x0c\x00\x00\x00IntProperty\x00\x04\x00\x00\x00\x00\x00\x00\x00\x00");
                body.extend(n.to_le_bytes());
                body.extend(fs("ItemId"));
            }
            b.extend(fs(list));
            b.extend(fs("StructProperty"));
            b.extend((body.len() as i32).to_le_bytes());
            b.extend(body);
        }
        let n = b.len() as i32 - 4;
        b[..4].copy_from_slice(&n.to_le_bytes());
        b
    }

    #[test]
    fn reads_and_edits_stacks_in_place() {
        let s = stream(&[("DA_Item_Resources_Metal_Parts", 33), ("DA_Item_Synthetic_Material_Resource", 51), ("DA_Item_Jelly_Resource", 32)]);
        assert!(stream_looks(&s));
        let mut doc = parse(&s).unwrap();
        assert_eq!(doc.view.pointer("/Resources/Metal parts"), Some(&serde_json::Value::from(33)));
        assert_eq!(doc.view.pointer("/Resources/Synthetic material"), Some(&serde_json::Value::from(51)));
        assert_eq!(doc.view.pointer("/Special/Neuropolymer"), Some(&serde_json::Value::from(32)));
        doc.set("/Resources/Metal parts", serde_json::Value::from(999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), s.len());
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Resources/Metal parts"), Some(&serde_json::Value::from(999)));
        assert_eq!(again.view.pointer("/Resources/Synthetic material"), Some(&serde_json::Value::from(51)));
    }

    #[test]
    fn slot_files_are_recognised() {
        let mut d = vec![1, 0, 0, 0, 2, 0, 0, 0];
        d.extend(vec![0u8; 100]);
        d.extend(fs("/Script/SaveExtension.SlotInfo"));
        d.extend(vec![0u8; 4096]);
        assert!(looks(&d));
        assert!(!looks(b"GVAS"));
    }
}
