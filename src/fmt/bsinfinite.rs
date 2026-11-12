//! BioShock Infinite's player state, inside a save already unpacked by the `bioshock` layer:
//! a block that starts with its length and 0xA13580C2; from byte 45 a table of persistent
//! variables (16 bytes each), then the inventory (u64 id, i32 count, u8 flag, i32, i32 k and
//! k more bytes). Infusions are inventory items whose count is the upgrade level; Silver
//! Eagles, the current salts and lockpicks sit in one block after a marker. Layout, item
//! ids and limits from Baistus's bioshock-infinite-profile-manager (MIT).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "BioShock Infinite save";
const MAGIC: u32 = 0xA135_80C2;
const VARS_AT: usize = 45;
const CURRENCY: [u8; 4] = [0xB2, 0x81, 0x25, 0xC1];
/// (item id, name).
const INFUSIONS: [(u64, &str); 3] = [(276, "Health"), (329, "Shield"), (235, "Salts")];
pub const INFUSION_MAX: i64 = 10;
pub const EAGLES_MAX: i64 = 99_999;
pub const LOCKPICKS_MAX: i64 = 999;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    u32_at(d, at).map(|v| v as i32)
}

struct Player {
    /// (item id, offset of its count).
    items: Vec<(u64, usize)>,
    currency: Option<usize>,
}

/// The player state starting at `s`, if it reads cleanly.
fn player_at(d: &[u8], s: usize) -> Option<Player> {
    let len = u32_at(d, s)? as usize;
    if u32_at(d, s + 4)? != MAGIC || len < VARS_AT + 8 {
        return None;
    }
    let end = s.checked_add(4 + len).filter(|&e| e <= d.len())?;
    let b = &d[s..end];
    let mut p = VARS_AT;
    let vars = u32_at(b, p)? as usize;
    p += 4;
    if vars > 20_000 || p + 16 * vars + 4 > b.len() || (0..vars).any(|i| u32_at(b, p + 16 * i + 4) != Some(0)) {
        return None;
    }
    p += 16 * vars;
    let n = u32_at(b, p)? as usize;
    p += 4;
    if n > 20_000 {
        return None;
    }
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        let id = u64::from_le_bytes(b.get(p..p + 8)?.try_into().ok()?);
        let flag = *b.get(p + 12)?;
        let extra = u32_at(b, p + 17)? as usize;
        if flag > 1 || extra > 64 || p + 21 + extra > b.len() {
            return None;
        }
        items.push((id, s + p + 8));
        p += 21 + extra;
    }
    // The currency block: only when exactly one plausible copy follows the inventory.
    let mut found = Vec::new();
    let mut i = p;
    while let Some(k) = b.get(i..).and_then(|r| r.windows(4).position(|w| w == CURRENCY)).map(|k| k + i) {
        let v: Option<Vec<i32>> = (0..9).map(|j| i32_at(b, k + 4 + 4 * j)).collect();
        if let Some(v) = v {
            if v[0] >= 0 && v[1..5].iter().all(|&z| z == 0) && v[8] == 0 && (0..=1000).contains(&v[5]) && (0..=100_000).contains(&v[6]) && (0..=100_000).contains(&v[7]) {
                found.push(s + k + 4);
            }
        }
        i = k + 1;
    }
    let currency = (found.len() == 1).then(|| found[0]);
    Some(Player { items, currency })
}

fn find(d: &[u8]) -> Option<Player> {
    let m = MAGIC.to_le_bytes();
    let mut i = 4;
    while let Some(k) = d.get(i..).and_then(|r| r.windows(4).position(|w| w == m)).map(|k| k + i) {
        if let Some(p) = player_at(d, k - 4) {
            if p.currency.is_some() || p.items.iter().any(|(id, _)| INFUSIONS.iter().any(|(x, _)| x == id)) {
                return Some(p);
            }
        }
        i = k + 1;
    }
    None
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let p = find(d).ok_or("not a BioShock Infinite save")?;
    let mut b = Builder::new(d);
    if let Some(c) = p.currency {
        b.ranged(&["Booker", "Silver Eagles"], c, Kind::I32, 0, EAGLES_MAX);
        b.ranged(&["Booker", "Salts"], c + 20, Kind::I32, 0, 1000);
        b.ranged(&["Booker", "Lockpicks"], c + 28, Kind::I32, 0, LOCKPICKS_MAX);
    }
    for (id, name) in INFUSIONS {
        if let Some(&(_, at)) = p.items.iter().find(|(x, _)| *x == id) {
            b.ranged(&["Infusions", name], at, Kind::I32, 0, INFUSION_MAX);
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Silver Eagles, salts and lockpicks, and the infusion levels you've picked (a kind you've never picked has no entry yet). The save's MD5 is written again on save. Not yet tried in the game.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn player(eagles: i32, health: i32) -> Vec<u8> {
        let mut b = vec![0u8; VARS_AT];
        b[4..8].copy_from_slice(&MAGIC.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend([7u8; 4].iter().chain([0u8; 4].iter()).chain([9u8; 8].iter()));
        b.extend(2u32.to_le_bytes());
        for (id, n) in [(276u64, health), (999, 3)] {
            b.extend(id.to_le_bytes());
            b.extend(n.to_le_bytes());
            b.push(1);
            b.extend(0i32.to_le_bytes());
            b.extend(2i32.to_le_bytes());
            b.extend([0xAA, 0xBB]);
        }
        b.extend([1, 2, 3]);
        b.extend(CURRENCY);
        for v in [eagles, 0, 0, 0, 0, 40, 7, 12, 0] {
            b.extend(v.to_le_bytes());
        }
        let len = (b.len() - 4) as u32;
        b[..4].copy_from_slice(&len.to_le_bytes());
        b
    }

    #[test]
    fn reads_and_edits_the_player() {
        let mut d = b"header bytes".to_vec();
        d.extend(player(1234, 4));
        d.extend(b"level states");
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Booker/Silver Eagles").unwrap(), 1234);
        assert_eq!(doc.view.pointer("/Booker/Lockpicks").unwrap(), 12);
        assert_eq!(doc.view.pointer("/Booker/Salts").unwrap(), 40);
        assert_eq!(doc.view.pointer("/Infusions/Health").unwrap(), 4);
        assert!(doc.view.pointer("/Infusions/Shield").is_none());
        assert!(doc.set("/Infusions/Health", serde_json::Value::from(11)).is_err());
        doc.set("/Booker/Silver Eagles", serde_json::Value::from(99_999)).unwrap();
        doc.set("/Infusions/Health", serde_json::Value::from(10)).unwrap();
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(again.view.pointer("/Booker/Silver Eagles").unwrap(), 99_999);
        assert_eq!(again.view.pointer("/Infusions/Health").unwrap(), 10);
        assert!(parse(b"nothing like a save here, not at all").is_err());
    }
}
