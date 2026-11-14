//! Dead Island (Definitive Edition and Riptide) saves, once the gzip is off: a flat binary that
//! starts `FF FF FF FF 00 00 00 00 05 00 00 00`. Only the level, the two health values and the
//! money are shown; every other byte is kept as it was. Offsets follow the layout notes of the
//! public dead-island-definitive-save-editor project (facts only; the code here is new).

use serde_json::{json, Value};

struct Spots {
    level: usize,
    max_health: usize,
    health: usize,
    money: usize,
}

fn u32_at(d: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(p..p + 4)?.try_into().ok()?))
}

fn wstr(d: &[u8], p: &mut usize) -> Option<()> {
    let n = u16::from_le_bytes(d.get(*p..*p + 2)?.try_into().ok()?) as usize;
    d.get(*p + 2..*p + 2 + n)?;
    *p += 2 + n;
    Some(())
}

fn spots(d: &[u8]) -> Option<Spots> {
    if d.len() < 0x40 || u32_at(d, 0)? != u32::MAX || u32_at(d, 4)? != 0 || u32_at(d, 8)? != 5 {
        return None;
    }
    let mut p = 0x30;
    for _ in 0..3 {
        wstr(d, &mut p)?; // map, checkpoint, spawn point
    }
    p += 4;
    wstr(d, &mut p)?; // character key
    p += 4 + 8;
    wstr(d, &mut p)?; // second checkpoint
    p += 8; // two unknown values
    u32_at(d, p)?;
    let level = u32_at(d, 0x14)?;
    if !(1..=200).contains(&level) {
        return None;
    }
    Some(Spots { level: 0x14, max_health: 0x18, health: 0x1C, money: p })
}

pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    let s = spots(data)?;
    let v = json!({
        "Level": u32_at(data, s.level)?,
        "Maximum health": u32_at(data, s.max_health)?,
        "Health": u32_at(data, s.health)?,
        "Money": u32_at(data, s.money)?,
    });
    serde_json::to_vec_pretty(&v).ok()
}

pub fn pack(original: &[u8], json: &[u8]) -> Result<Vec<u8>, String> {
    let s = spots(original).ok_or("The Dead Island save this was opened from is not valid")?;
    let v: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let mut out = original.to_vec();
    for (key, at) in [("Level", s.level), ("Maximum health", s.max_health), ("Health", s.health), ("Money", s.money)] {
        let n = v.get(key).and_then(Value::as_u64).filter(|n| *n <= u32::MAX as u64).ok_or(format!("{key} must be a whole number"))?;
        out[at..at + 4].copy_from_slice(&(n as u32).to_le_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0xFF; 4];
        d.extend([0, 0, 0, 0, 5, 0, 0, 0, 12, 0, 0, 0, 5, 0, 0, 0]);
        d.extend([101u32, 171, 167, 0, 2, 12, 3].iter().flat_map(|n| n.to_le_bytes()));
        for s in ["ACT1A", "Hub", "Spawn"] {
            d.extend((s.len() as u16).to_le_bytes());
            d.extend(s.as_bytes());
        }
        d.extend([4, 0, 0, 0]);
        d.extend([9, 0]);
        d.extend(b"Type;SamB");
        d.extend([0u8; 12]);
        d.extend([3, 0]);
        d.extend(b"abc");
        d.extend([0u8; 8]);
        d.extend(1234u32.to_le_bytes());
        d.extend([0u8; 40]);
        d
    }

    #[test]
    fn edits_money_in_place() {
        let d = sample();
        let json = open(&d).unwrap();
        let mut v: Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(v["Money"], 1234);
        v["Money"] = json!(99);
        let out = pack(&d, &serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(out.len(), d.len());
        let v2: Value = serde_json::from_slice(&open(&out).unwrap()).unwrap();
        assert_eq!(v2["Money"], 99);
        assert_eq!(v2["Level"], 101);
    }
}
