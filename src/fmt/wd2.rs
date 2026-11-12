//! Watch Dogs 2 (`1.save`, `2.save` … in the Ubisoft Connect save folder): the launcher's
//! 0x228-byte header, a short block of the game's own, then an `nbCF` object tree as used by
//! Ubisoft's Disrupt engine. Each node is: child count, name hash, value count, then for each
//! value its name hash, size and bytes; counts below 0xFE are one byte, 0xFF is followed by a
//! u32, and 0xFE marks a u32 reference to data written earlier. Names are CRC-32s of the
//! property names, so `Money` and `Followers` are found by hashing those words. Values are
//! changed in place; the file has no checksum over the tree.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Watch Dogs 2 save";

const PLAYER_PROGRESSION: u32 = 0x4163_991B;
const FIELDS: &[(u32, &str, &str)] = &[
    (0x761E_3CE0, "Player", "Money"),
    (0x06F9_7F04, "Player", "Followers"),
    (0x235D_3359, "Player", "Research points"),
    (0x1A7E_DCD6, "Totals", "Money earned in total"),
    (0x5BA2_4E0D, "Totals", "Followers gained in total"),
];

struct R<'a> {
    d: &'a [u8],
    p: usize,
}

impl R<'_> {
    fn u8(&mut self) -> Option<u8> {
        let b = *self.d.get(self.p)?;
        self.p += 1;
        Some(b)
    }
    fn u32(&mut self) -> Option<u32> {
        let v = u32::from_le_bytes(self.d.get(self.p..self.p + 4)?.try_into().ok()?);
        self.p += 4;
        Some(v)
    }
    /// A count; true when it is a reference to earlier data instead.
    fn count(&mut self) -> Option<(usize, bool)> {
        match self.u8()? {
            b if b < 0xFE => Some((b as usize, false)),
            0xFE => Some((self.u32()? as usize, true)),
            _ => Some((self.u32()? as usize, false)),
        }
    }
}

/// Walks one node; records every value of a `PlayerProgression` node as (hash, at, size).
fn node(r: &mut R, depth: usize, out: &mut Vec<(u32, usize, usize)>) -> Option<()> {
    if depth > 64 {
        return None;
    }
    let (children, is_ref) = r.count()?;
    if is_ref {
        return Some(());
    }
    let hash = r.u32()?;
    let (values, _) = r.count()?;
    for _ in 0..values {
        let name = r.u32()?;
        let (size, is_ref) = r.count()?;
        if !is_ref {
            if r.p + size > r.d.len() {
                return None;
            }
            if hash == PLAYER_PROGRESSION {
                out.push((name, r.p, size));
            }
            r.p += size;
        }
    }
    for _ in 0..children {
        node(r, depth + 1, out)?;
    }
    Some(())
}

fn tree(d: &[u8]) -> Option<Vec<(u32, usize, usize)>> {
    if d.len() < 0x280 || d[..4] != [0x24, 0x02, 0, 0] {
        return None;
    }
    let at = d[0x228..d.len().min(0x300)].windows(4).position(|w| w == b"nbCF")? + 0x228;
    let mut r = R { d, p: at + 16 };
    let mut out = Vec::new();
    node(&mut r, 0, &mut out)?;
    (r.p == d.len()).then_some(out)
}

pub fn looks(d: &[u8]) -> bool {
    tree(d).is_some_and(|v| !v.is_empty())
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let values = tree(d).ok_or("not a Watch Dogs 2 save")?;
    if values.is_empty() {
        return Err("this Watch Dogs 2 file has no player progress (it may be the options file)".into());
    }
    let mut b = Builder::new(d);
    for &(hash, group, label) in FIELDS {
        if let Some(&(_, at, 4)) = values.iter().find(|v| v.0 == hash) {
            b.ranged(&[group, label], at, Kind::I32, 0, 999_999_999);
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game and turn off Ubisoft Connect cloud saves first. Followers raise your level; research points are only saved while you have some unspent, so they can be edited only then."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 0x268];
        d[..4].copy_from_slice(&0x224u32.to_le_bytes());
        d.extend(b"nbCF");
        d.extend([3, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0]);
        // root: one child, no values
        d.push(1);
        d.extend(0x1234_5678u32.to_le_bytes());
        d.push(0);
        // PlayerProgression: no children, two values
        d.push(0);
        d.extend(PLAYER_PROGRESSION.to_le_bytes());
        d.push(2);
        d.extend(0x761E_3CE0u32.to_le_bytes());
        d.push(4);
        d.extend(183_874i32.to_le_bytes());
        d.extend(0x06F9_7F04u32.to_le_bytes());
        d.push(4);
        d.extend(1_030_072i32.to_le_bytes());
        d
    }

    #[test]
    fn money_and_followers_are_written_in_place() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Player/Money", Value::from(5_000_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let doc = parse(&out).unwrap();
        let mut doc = doc;
        assert_eq!(doc.view_and_editable().0.pointer("/Player/Money"), Some(&Value::from(5_000_000)));
        assert_eq!(doc.view_and_editable().0.pointer("/Player/Followers"), Some(&Value::from(1_030_072)));
    }

    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("WD2_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let mut doc = parse(&d).unwrap();
            println!("{:?} {}", e.path(), doc.view_and_editable().0);
            assert_eq!(doc.to_bytes().unwrap(), d);
        }
    }
}
