//! LEGO Star Wars: The Skywalker Saga, `SLOTn\COLLECTABLES.BLOB`: a TT Games object stream
//! (`StreamInfo`, `TypeList`, `ClassList`, then objects) holding one `collectables::GameSaveTable`.
//! Each entry names a table, an entry and a column, then a value object and its "permanent"
//! copy. The stud wallet is `StudCollectables` / `MainTotal` / `TotalValue`, a 64-bit
//! `UInt64Val`; both copies are written. The file has no checksum and its size doesn't change.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "LEGO Star Wars: The Skywalker Saga collectables";
const CLASS: &[u8] = b"collectables::GameSaveTable\0";
/// What the screen accepts (the game keeps more than 50 billion in public saves).
pub const MAX_STUDS: i64 = 99_999_999_999;

fn str32(name: &str) -> Vec<u8> {
    let mut v = ((name.len() + 1) as u32).to_le_bytes().to_vec();
    v.extend_from_slice(name.as_bytes());
    v.push(0);
    v
}

fn u32at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 0x400
        && d.get(8..18) == Some(b"StreamInfo")
        && d[..d.len().min(0x1000)]
            .windows(CLASS.len())
            .any(|w| w == CLASS)
}

/// Reads one value object after `at`: `str32 name`, a present flag, the class id, then a
/// `MOBJ` block of 0x19 bytes whose last eight are the number. Returns (number offset, end).
fn u64_value(d: &[u8], at: usize) -> Option<(usize, usize)> {
    let name = str32("No name");
    if d.get(at..at + name.len())? != name.as_slice() || *d.get(at + name.len())? != 1 {
        return None;
    }
    let block = at + name.len() + 5;
    if u32at(d, block)? != 0x19 || d.get(block + 4..block + 13)? != str32("MOBJ").as_slice() {
        return None;
    }
    Some((block + 0x11, block + 0x19))
}

/// The two places (value, permanent value) a `table` / `entry` / `TotalValue` number lives.
fn total(d: &[u8], table: &str, entry: &str) -> Option<(usize, usize)> {
    let mut key = str32(table);
    key.extend(str32(entry));
    key.extend(str32("TotalValue"));
    let start = d.windows(key.len()).position(|w| w == key.as_slice())? + key.len() + 4;
    let (value, end) = u64_value(d, start)?;
    let (permanent, _) = u64_value(d, end)?;
    Some((value, permanent))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Skywalker Saga COLLECTABLES.BLOB".into());
    }
    let (value, permanent) =
        total(d, "StudCollectables", "MainTotal").ok_or("the stud total isn't in this save")?;
    let mut b = Builder::new(d);
    b.ranged(&["Studs"], value, Kind::U64, 0, MAX_STUDS);
    b.mirror(&["Studs"], permanent);
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Close the game first. This is COLLECTABLES.BLOB in a SLOTn folder; the stud wallet is saved twice and both copies are written.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn value(n: u64) -> Vec<u8> {
        let mut v = str32("No name");
        v.push(1);
        v.extend(7u32.to_le_bytes());
        v.extend(0x19u32.to_le_bytes());
        v.extend(str32("MOBJ"));
        v.extend([0; 4]);
        v.extend(n.to_le_bytes());
        v
    }

    fn sample() -> Vec<u8> {
        let mut d = vec![0x17, 0, 0, 0, 0x0b, 0, 0, 0];
        d.extend(b"StreamInfo\0");
        d.extend(vec![0u8; 0x300]);
        d.extend(str32("Class"));
        d.extend(str32("collectables::GameSaveTable"));
        d.extend(vec![0u8; 0x100]);
        for (entry, n) in [("TotalP1", 0u64), ("MainTotal", 51_111_592_606)] {
            d.extend(str32("StudCollectables"));
            d.extend(str32(entry));
            d.extend(str32("TotalValue"));
            d.extend(8u32.to_le_bytes());
            d.extend(value(n));
            d.extend(value(n - n / 1000));
            d.push(1);
        }
        d
    }

    #[test]
    fn studs_written_in_both_copies() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(
            doc.view.pointer("/Studs"),
            Some(&Value::from(51_111_592_606u64))
        );
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Studs", Value::from(1_000_000_000u64)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let doc = parse(&out).unwrap();
        assert_eq!(
            doc.view.pointer("/Studs"),
            Some(&Value::from(1_000_000_000u64))
        );
        let (v, p) = total(&out, "StudCollectables", "MainTotal").unwrap();
        assert_eq!(out[v..v + 8], out[p..p + 8]);
    }
}
