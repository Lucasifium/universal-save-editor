//! Cyberpunk 2077 game data. The file's uncompressed tail holds a table of named sections
//! ("EDON", packed count, then per section: name, next, child, offset, size; offsets count
//! from the file start). The `inventory` section starts with a size, the number of
//! sub-inventories, then the first one's id (1 = the player) and item count; its items are
//! the `itemData` sections that follow it in the table. An item starts with a counter,
//! then its TweakDB id (CRC-32 of the name, and the name's length), seed, flags, creation
//! time, and — for stackable items like money — the quantity.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// TweakDB id of an item name.
fn tdbid(name: &str) -> u64 {
    let mut h = flate2::Crc::new();
    h.update(name.as_bytes());
    u64::from(h.sum()) | ((name.len() as u64) << 32)
}

struct Section {
    name: String,
    offset: usize,
    size: usize,
}

/// The section table from the file's tail (after the compressed data).
fn sections(tail: &[u8]) -> Option<Vec<Section>> {
    if tail.len() < 12 || &tail[tail.len() - 4..] != b"ENOD" {
        return None;
    }
    // The table's position is stored just before "DONE" — as a file offset; find "EDON".
    let at = tail.windows(4).position(|w| w == b"EDON")?;
    let mut q = at + 4;
    let packed = |q: &mut usize| -> Option<i64> {
        let b = *tail.get(*q)?;
        *q += 1;
        let mut v = i64::from(b & 0x3F);
        let mut shift = 6;
        if b & 0x40 != 0 {
            loop {
                let c = *tail.get(*q)?;
                *q += 1;
                v |= i64::from(c & 0x7F) << shift;
                shift += 7;
                if c & 0x80 == 0 {
                    break;
                }
            }
        }
        Some(if b & 0x80 != 0 { -v } else { v })
    };
    let n = packed(&mut q)?;
    if !(1..200_000).contains(&n) {
        return None;
    }
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let len = (*tail.get(q)? & 0x7F) as usize;
        let name = String::from_utf8_lossy(tail.get(q + 1..q + 1 + len)?).into_owned();
        q += 1 + len;
        let offset = u32_at(tail, q + 8)? as usize;
        let size = u32_at(tail, q + 12)? as usize;
        q += 16;
        out.push(Section { name, offset, size });
    }
    Some(out)
}

/// `head` is the file before the compressed data (section offsets count from its start).
pub fn parse(data: &[u8], head: usize, tail: &[u8]) -> Option<TreeDoc> {
    let secs = sections(tail)?;
    let inv = secs.iter().position(|s| s.name == "inventory")?;
    let at = secs[inv].offset.checked_sub(head)?;
    let count = u32_at(data, at + 4)?;
    let first_id = u64::from(u32_at(data, at + 8)?) | (u64::from(u32_at(data, at + 12)?) << 32);
    let items = u32_at(data, at + 16)? as usize;
    if count == 0 || first_id != 1 || items > 100_000 {
        return None;
    }
    let money = tdbid("Items.money").to_le_bytes();
    let mut b = Builder::new(data);
    let mut found = false;
    for s in secs[inv + 1..].iter().filter(|s| s.name == "itemData").take(items) {
        let o = s.offset.checked_sub(head)?;
        if data.get(o + 4..o + 12) == Some(&money[..]) && s.size >= 29 {
            b.ranged(&["V", "Eurodollars"], o + 25, Kind::U32, 0, i32::MAX as i64);
            found = true;
            break;
        }
    }
    if !found {
        return None;
    }
    Some(b.finish("Cyberpunk 2077 game data", Checksum::None).with_note(
        "V's eurodollars, from the player's own inventory. Other values: use Value search with a number you know.",
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn tweakdb_id() {
        // Items.money: CRC-32 0xF5E188EC, length 11.
        assert_eq!(super::tdbid("Items.money"), 0x0B_F5E1_88EC);
    }
}
