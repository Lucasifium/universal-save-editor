//! Salt and Sanctuary `datN.slv` character saves: a .NET `BinaryWriter` stream, no checksum.
//! After the version number 29 come the name (7-bit length prefix; newer builds write a version
//! marker string first), hardcore, kills, deaths, sanctuary rests, corruption, challenge type,
//! the play time in ticks, playthrough, last sanctuary, current creed, two respawn points, then
//! the level and its item-class and skill-tree arrays (30 + 500 + 2 numbers), ten creed
//! standings, ten creeds of four unlocks, ten creed levels, 32 NPC standings, and the
//! expunged count, salt and gold. The layout is the one the game's own reader uses, as
//! described by the public editors of this game; everything is read in place and edited in
//! place.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Salt and Sanctuary character";

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// A .NET string at `at`: (text, position after it).
fn net_string(d: &[u8], at: usize) -> Option<(String, usize)> {
    let mut len = 0usize;
    let mut p = at;
    for shift in (0..35).step_by(7) {
        let b = *d.get(p)?;
        p += 1;
        len |= usize::from(b & 0x7F) << shift;
        if b & 0x80 == 0 {
            break;
        }
    }
    let s = d.get(p..p.checked_add(len)?)?;
    let text = std::str::from_utf8(s).ok()?;
    (len <= 64 && !text.chars().any(char::is_control)).then(|| (text.to_string(), p + len))
}

struct Layout {
    name: String,
    hardcore: usize,
    stats: usize,
    after_creeds: usize,
}

fn layout(d: &[u8]) -> Option<Layout> {
    // 29 in current builds; some older saves carry a smaller number and the same layout.
    if !(0..=29).contains(&i32_at(d, 0)?) {
        return None;
    }
    let (first, p) = net_string(d, 4)?;
    // Newer builds write a version marker, then the name.
    for (name, hardcore) in [(Some(first.clone()), p), (net_string(d, p).map(|x| x.0), net_string(d, p).map_or(0, |x| x.1))] {
        let Some(name) = name else { continue };
        let flag = *d.get(hardcore)?;
        let stats = hardcore + 1 + 5 * 4 + 8 + 3 * 4 + 16;
        let level = i32_at(d, stats)?;
        let after = stats + 4 + 4 * 30 + 4 * 500 + 8 + 4 * (10 + 40 + 10);
        let (salt, gold) = (i32_at(d, after + 4 * 33)?, i32_at(d, after + 4 * 34)?);
        if flag <= 1 && (1..=400).contains(&level) && salt >= 0 && gold >= 0 && d.len() > after + 4 * 40 {
            return Some(Layout { name, hardcore, stats, after_creeds: after });
        }
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 2500 && layout(d).is_some_and(|l| !l.name.is_empty())
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let l = layout(d).ok_or("not a Salt and Sanctuary save")?;
    let mut b = Builder::new(d);
    b.info(&["Character", "Name"], l.name.clone());
    let at = |n: usize| l.hardcore + 1 + 4 * n;
    b.field(&["Character", "Hardcore"], l.hardcore, Kind::B8);
    b.ranged(&["Character", "Level"], l.stats, Kind::I32, 1, 400);
    b.ranged(&["Character", "Salt"], l.after_creeds + 4 * 33, Kind::I32, 0, 99_999_999);
    b.ranged(&["Character", "Gold"], l.after_creeds + 4 * 34, Kind::I32, 0, 99_999_999);
    b.ranged(&["Character", "Kills"], at(0), Kind::I32, 0, 99_999_999);
    b.ranged(&["Character", "Deaths"], at(1), Kind::I32, 0, 99_999_999);
    b.ranged(&["Character", "Sanctuary rests"], at(2), Kind::I32, 0, 99_999_999);
    b.ranged(&["Character", "Corruption"], at(3), Kind::I32, 0, 99_999_999);
    b.ranged(&["Character", "Playthrough"], at(5) + 8, Kind::I32, 0, 99);
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Salt and gold are what you carry; items and the skill tree are not edited yet."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = 29i32.to_le_bytes().to_vec();
        d.push(4);
        d.extend(b"Grim");
        d.push(0);
        d.extend([1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
        d.extend([0u8; 8 + 12 + 16]);
        let stats = d.len();
        d.extend(45i32.to_le_bytes());
        d.extend(vec![0u8; 4 * 30 + 4 * 500 + 8 + 4 * 60]);
        let after = d.len();
        d.extend(vec![0u8; 4 * 40]);
        d[after + 4 * 33..after + 4 * 33 + 4].copy_from_slice(&5214i32.to_le_bytes());
        d[after + 4 * 34..after + 4 * 34 + 4].copy_from_slice(&9293i32.to_le_bytes());
        assert!(stats > 0);
        d.extend(vec![0u8; 2500]);
        d
    }

    #[test]
    fn reads_and_edits_salt_and_gold() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/Salt").unwrap(), 5214);
        assert_eq!(doc.view.pointer("/Character/Gold").unwrap(), 9293);
        assert_eq!(doc.view.pointer("/Character/Kills").unwrap(), 1);
        assert_eq!(doc.view.pointer("/Character/Level").unwrap(), 45);
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Character/Gold", 77.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(parse(&out).unwrap().view.pointer("/Character/Gold").unwrap(), 77);
    }
}
