//! Borderlands (2009) character saves (`Save0001.sav` ...): the letters `WSG`, the version 2,
//! then a `PLYR` block of little-endian values and length-prefixed strings. Level, experience,
//! skill points, money, skills and ammunition are read in place by walking the block; items
//! and weapons are left as they are. Nothing is checksummed on PC.
//!
//! Layout follows Gibbed.Borderlands (Rick, zlib licence).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Borderlands save";

fn u32at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// A length-prefixed string (a negative length counts UTF-16 characters); returns it and the end.
fn string(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?);
    let at = at + 4;
    if n == 0 {
        return Some((String::new(), at));
    }
    if n < 0 {
        let len = (-(n as i64)) as usize * 2;
        if len > 1 << 20 {
            return None;
        }
        let units: Vec<u16> = d.get(at..at + len)?.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Some((String::from_utf16_lossy(&units).trim_end_matches('\0').to_string(), at + len))
    } else {
        let len = n as usize;
        if len > 1 << 20 {
            return None;
        }
        Some((String::from_utf8_lossy(d.get(at..at + len)?).trim_end_matches('\0').to_string(), at + len))
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && &d[..3] == b"WSG" && u32at(d, 3) == Some(2) && &d[7..11] == b"PLYR" && u32at(d, 11) == Some(35)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Borderlands save".into());
    }
    let bad = || "unexpected Borderlands save layout".to_string();
    let (class, mut p) = string(d, 15).ok_or_else(bad)?;
    let mut b = Builder::new(d);
    b.info(&["Character", "Class"], class);
    b.ranged(&["Character", "Level"], p, Kind::U32, 1, 69).ok_or_else(bad)?;
    b.ranged(&["Character", "Experience"], p + 4, Kind::U32, 0, 2_000_000_000).ok_or_else(bad)?;
    b.ranged(&["Character", "Skill points"], p + 8, Kind::U32, 0, 999).ok_or_else(bad)?;
    b.ranged(&["Character", "Money"], p + 16, Kind::U32, 0, 2_000_000_000).ok_or_else(bad)?;
    p += 24;
    // Skills: name, level, experience, artifact mode.
    let n = u32at(d, p).ok_or_else(bad)? as usize;
    p += 4;
    if n > 1000 {
        return Err(bad());
    }
    for _ in 0..n {
        let (name, q) = string(d, p).ok_or_else(bad)?;
        let name = name.rsplit('.').next().unwrap_or(&name).to_string();
        b.ranged(&["Skills", &name, "Level"], q, Kind::U32, 0, 10);
        p = q + 12;
    }
    p += 16;
    // Ammunition pools: name, pool, quantity (float), upgrade level.
    let n = u32at(d, p).ok_or_else(bad)? as usize;
    p += 4;
    if n > 1000 {
        return Err(bad());
    }
    for _ in 0..n {
        let (_, q) = string(d, p).ok_or_else(bad)?;
        let (pool, q) = string(d, q).ok_or_else(bad)?;
        let pool = pool.rsplit('.').next().unwrap_or(&pool).to_string();
        b.ranged(&["Ammunition", &pool, "Quantity"], q, Kind::F32Int, 0, 9_999);
        b.ranged(&["Ammunition", &pool, "Upgrade level"], q + 4, Kind::I32, 0, 9);
        p = q + 8;
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Items and weapons are left as they are."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(out: &mut Vec<u8>, t: &str) {
        out.extend_from_slice(&((t.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(t.as_bytes());
        out.push(0);
    }

    #[test]
    fn level_money_and_ammo_edit_in_place() {
        let mut d = b"WSG".to_vec();
        d.extend_from_slice(&2u32.to_le_bytes());
        d.extend_from_slice(b"PLYR");
        d.extend_from_slice(&35u32.to_le_bytes());
        s(&mut d, "gd_siren.Character");
        for v in [20u32, 5000, 3, 0, 1234, 1] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&1u32.to_le_bytes());
        s(&mut d, "gd_siren.Skills.Phasewalk");
        for v in [2u32, 10, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&[0u8; 16]);
        d.extend_from_slice(&1u32.to_le_bytes());
        s(&mut d, "name");
        s(&mut d, "d_resources.AmmoPools.Pistol");
        d.extend_from_slice(&80f32.to_le_bytes());
        d.extend_from_slice(&1i32.to_le_bytes());
        d.resize(d.len() + 40, 0);
        assert!(looks(&d));
        let mut t = parse(&d).unwrap();
        assert_eq!(t.view.pointer("/Character/Money"), Some(&serde_json::Value::from(1234)));
        assert_eq!(t.view.pointer("/Skills/Phasewalk/Level"), Some(&serde_json::Value::from(2)));
        assert_eq!(t.view.pointer("/Ammunition/Pistol/Quantity"), Some(&serde_json::Value::from(80)));
        t.set("/Character/Money", serde_json::Value::from(999_999)).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
    }
}
