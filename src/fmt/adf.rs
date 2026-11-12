//! theHunter: Call of the Wild profile (`thp_player_profile_adf`): an Avalanche ADF file (the
//! type definitions travel with the data), here with a five-byte prefix. The profile is the
//! first instance; its struct's members give each value's offset, so the screen edits the
//! level, XP, skill and perk points and cash in place. Nothing is checksummed in this layout.
//!
//! Layout (offsets from the " FDA" magic): version, instance count/offset, type-definition
//! count/offset, string-hash count/offset, name count/offset, total size; each instance is
//! (name hash, type hash, offset, size, name index) and each struct member is (name index u64,
//! type hash, size, offset, default). Names are length-prefixed in one table.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "theHunter: Call of the Wild profile";

fn u32at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}
fn u64at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Where the ADF header starts (the magic is " FDA" in the file).
fn base(d: &[u8]) -> Option<usize> {
    [0usize, 5].into_iter().find(|&b| d.get(b..b + 4) == Some(b" FDA"))
}

/// (member name, offset inside the profile struct, size) of the first struct, and where the
/// profile's data starts in the file.
fn members(d: &[u8]) -> Option<(Vec<(String, usize, usize)>, usize)> {
    let b = base(d)?;
    if u32at(d, b + 4)? != 4 {
        return None;
    }
    let (ic, io, tc, to, nc, no) = (u32at(d, b + 8)?, u32at(d, b + 12)?, u32at(d, b + 16)?, u32at(d, b + 20)?, u32at(d, b + 32)?, u32at(d, b + 36)?);
    if ic == 0 || tc == 0 || nc > 4096 {
        return None;
    }
    let n0 = b + no as usize;
    let lens = d.get(n0..n0 + nc as usize)?;
    let mut p = n0 + nc as usize;
    let mut names = vec![];
    for &l in lens {
        names.push(String::from_utf8_lossy(d.get(p..p + l as usize)?).to_string());
        p += l as usize + 1;
    }
    let inst = b + io as usize;
    let type_hash = u32at(d, inst + 4)?;
    let data_at = b + u32at(d, inst + 8)? as usize;
    // Walk the type definitions (36 bytes each, then a struct's members, 32 bytes each).
    let mut p = b + to as usize;
    for _ in 0..tc {
        let (ty, hash, name_ix) = (u32at(d, p)?, u32at(d, p + 12)?, u64at(d, p + 16)?);
        let _ = name_ix;
        p += 36;
        match ty {
            1 => {
                let mc = u32at(d, p)? as usize;
                p += 4;
                let mut out = vec![];
                for _ in 0..mc {
                    let (ni, size, off) = (u64at(d, p)? as usize, u32at(d, p + 12)? as usize, u32at(d, p + 16)? as usize & 0xFF_FFFF);
                    out.push((names.get(ni)?.clone(), off, size));
                    p += 32;
                }
                if hash == type_hash {
                    return Some((out, data_at));
                }
            }
            // Arrays, pointers and the like carry one more 8-byte field each in this version.
            _ => return None,
        }
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    members(d).is_some_and(|(m, _)| m.iter().any(|(n, ..)| n == "Cash") && m.iter().any(|(n, ..)| n == "SkillPoints"))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (m, at) = members(d).ok_or("not a theHunter profile")?;
    if !m.iter().any(|(n, ..)| n == "Cash") {
        return Err("not a theHunter profile".into());
    }
    let mut b = Builder::new(d);
    for (label, key) in [("Cash", "Cash"), ("Level", "Level"), ("XP", "Xp"), ("Skill points", "SkillPoints"), ("Perk points", "PerkPoints")] {
        if let Some((_, off, 4)) = m.iter().find(|(n, ..)| n == key) {
            b.field(&["Hunter", label], at + off, Kind::U32);
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. The skill points already spent are stored per skill tree and are left as they are."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// A tiny version-4 ADF: one struct type (Cash, Level, SkillPoints, PerkPoints, Xp) and one instance.
    fn profile() -> Vec<u8> {
        let names = ["0", "Prof", "Cash", "Level", "SkillPoints", "PerkPoints", "Xp"];
        let mut table: Vec<u8> = names.iter().map(|n| n.len() as u8).collect();
        for n in names {
            table.extend_from_slice(n.as_bytes());
            table.push(0);
        }
        let hash = 0x1234_5678u32;
        let mut ty = vec![];
        for v in [1u32, 20, 4, hash] {
            ty.extend_from_slice(&v.to_le_bytes());
        }
        ty.extend_from_slice(&1u64.to_le_bytes());
        ty.extend_from_slice(&[0u8; 12]);
        ty.extend_from_slice(&5u32.to_le_bytes());
        for (i, off) in [(2u64, 0u32), (3, 4), (4, 8), (5, 12), (6, 16)] {
            ty.extend_from_slice(&i.to_le_bytes());
            ty.extend_from_slice(&0x075e_4e4fu32.to_le_bytes());
            ty.extend_from_slice(&4u32.to_le_bytes());
            ty.extend_from_slice(&off.to_le_bytes());
            ty.extend_from_slice(&[0u8; 12]);
        }
        let data_len = 20usize;
        let inst_off = 0x40 + data_len;
        let ty_off = inst_off + 24;
        let name_off = ty_off + ty.len();
        let mut f = vec![1, 1, 0, 0, 0];
        f.extend_from_slice(b" FDA");
        for v in [4u32, 1, inst_off as u32, 1, ty_off as u32, 0, 0, 1 + names.len() as u32 - 1, name_off as u32, 0] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.resize(5 + 0x40, 0);
        let mut data = vec![0u8; data_len];
        data[0..4].copy_from_slice(&1500u32.to_le_bytes());
        data[4..8].copy_from_slice(&60u32.to_le_bytes());
        f.extend_from_slice(&data);
        for v in [0u32, hash, 0x40, data_len as u32] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(&[0u8; 8]);
        f.extend_from_slice(&ty);
        f.extend_from_slice(&table);
        f
    }

    #[test]
    fn cash_and_level_edit_in_place() {
        let d = profile();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Hunter/Cash"), Some(&Value::from(1500)));
        assert_eq!(doc.view.pointer("/Hunter/Level"), Some(&Value::from(60)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Hunter/Cash", Value::from(999_999)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes(out[5 + 0x40..5 + 0x44].try_into().unwrap()), 999_999);
        assert_eq!(out.len(), d.len());
    }
}
