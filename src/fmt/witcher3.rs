//! The Witcher 3 game data (after the CD Projekt LZ4 layer). Layout from the MIT-licensed
//! W3SavegameEditor (Atvaark): "SAV3", …, a name table (`NM` + `MANU` strings), a variable
//! table, and a footer ending in the variable table's offset and "SE". Offsets count from
//! the start of the original file, so the length of what came before this data is worked
//! out from where the name table must be.
//!
//! Script objects store properties as (name index, type index, size, value). The player's
//! `levelManager` (type `handle:W3LevelManager`) appears once: it holds `level` and the
//! `points` array (skill points, then experience; each {free, used}). Geralt's money is the
//! first "Crowns" item after it: name index, id, flags, two dye names, then the quantity.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

fn u32_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"SAV3") && d.ends_with(b"SE")
}

struct Names {
    list: Vec<String>,
    /// Where the name table starts (end of the searchable game data).
    at: usize,
}

impl Names {
    fn index(&self, n: &str) -> Option<u16> {
        self.list.iter().position(|x| x == n).map(|i| (i + 1) as u16)
    }
}

fn names(d: &[u8]) -> Option<Names> {
    let vt = u32_at(d, d.len() - 6)?;
    // The offsets count from the original file's start: find the shift that makes the
    // footer point at the "NM" marker.
    for base in 0..(1 << 21).min(vt) {
        let Some(sf) = (vt - base).checked_sub(10) else { break };
        let Some(nm) = u32_at(d, sf).and_then(|n| n.checked_sub(base)) else { continue };
        if d.get(nm..nm + 6) == Some(b"NMMANU") {
            let count = u32_at(d, nm + 6)?;
            let mut p = nm + 14;
            let mut list = Vec::with_capacity(count);
            for _ in 0..count {
                let n = *d.get(p)? as usize;
                list.push(d.get(p + 1..p + 1 + n)?.iter().map(|&c| c as char).collect());
                p += 1 + n;
            }
            return Some(Names { list, at: nm });
        }
    }
    None
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not Witcher 3 game data".into());
    }
    let nt = names(d).ok_or("the name table wasn't found")?;
    let ix = |n: &str| nt.index(n).ok_or_else(|| format!("no \u{201C}{n}\u{201D} in this save"));
    let int32 = ix("Int32")?;
    let pair = |a: u16, b: u16| [a.to_le_bytes(), b.to_le_bytes()].concat();
    let lm = find(&d[..nt.at], &pair(ix("levelManager")?, ix("handle:W3LevelManager")?)).ok_or("the player's level data wasn't found")?;
    let lm_size = u32_at(d, lm + 4).ok_or("bad level data")?;
    let lm_end = (lm + 4 + lm_size).min(nt.at);
    let block = &d[lm..lm_end];
    let mut b = Builder::new(d);
    // level (Int32): name, type, size 8, value.
    let level = pair(ix("level")?, int32);
    if let Some(o) = find(block, &[&level[..], &8u32.to_le_bytes()].concat()) {
        b.ranged(&["Geralt", "Level"], lm + o + 8, Kind::I32, 1, 100);
    }
    // points: an array of {free, used} — skill points, then experience. Fields at their
    // default (0) aren't stored, so each element is read on its own.
    let (free, used) = (ix("free")?, ix("used")?);
    let arr = pair(ix("points")?, ix("array:2,0,SSpendablePoints")?);
    if let Some(o) = find(block, &arr) {
        let start = lm + o;
        let end = (start + 4 + u32_at(d, start + 4).unwrap_or(0)).min(lm_end);
        let count = u32_at(d, start + 8).unwrap_or(0);
        let mut q = start + 12;
        for (n, (label, a, c)) in [("Skill points", "Unspent", "Spent"), ("Experience", "Toward next level", "Total")].into_iter().enumerate() {
            if n >= count || q >= end {
                break;
            }
            q += 1; // each element starts with a 0 byte
            // Properties until a 0 name.
            while q + 2 <= end {
                let name = u32_at(d, q).map(|x| x as u16).unwrap_or(0);
                if name == 0 {
                    q += 2;
                    break;
                }
                let size = u32_at(d, q + 4).unwrap_or(0);
                if size < 4 {
                    q = end;
                    break;
                }
                let ty = (u32_at(d, q).unwrap_or(0) >> 16) as u16;
                if ty == int32 && size == 8 {
                    let which = if name == free { Some(a) } else if name == used { Some(c) } else { None };
                    if let Some(w) = which {
                        b.ranged(&["Geralt", label, w], q + 8, Kind::I32, 0, i32::MAX as i64);
                    }
                }
                q += 4 + size;
            }
        }
    }
    // Crowns: the first stack after the player's data.
    if let (Ok(crowns), Ok(dye1), Ok(dye2)) = (ix("Crowns"), ix("Dye Default"), ix("dye_default")) {
        let dyes = pair(dye1, dye2);
        let mut o = lm_end;
        while let Some(i) = find(&d[o..nt.at], &crowns.to_le_bytes()).map(|i| i + o) {
            if d.get(i + 13..i + 17) == Some(&dyes[..]) {
                b.ranged(&["Geralt", "Crowns"], i + 17, Kind::U32, 0, i32::MAX as u32 as i64);
                break;
            }
            o = i + 2;
            if o > lm_end + (4 << 20) {
                break;
            }
        }
    }
    Ok(b.finish("The Witcher 3 game data", Checksum::None).with_note(
        "Geralt's crowns, level, and unspent skill points and experience. The game works out the level from the \
         experience, so raise both together.",
    ))
}

#[cfg(test)]
mod tests {
    /// Research aid: SAVE_EDITOR_W3 = unpacked Witcher 3 game data.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_W3").unwrap()).unwrap();
        let t = std::time::Instant::now();
        let doc = super::parse(&d).unwrap();
        println!("{:?} {}", t.elapsed(), serde_json::to_string_pretty(&doc.view).unwrap());
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}
