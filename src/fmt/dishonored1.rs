//! Dishonored (2012) player state, inside a save already unpacked by the `ue3pkg` layer.
//!
//! The game's objects are listed as 7-byte records: an object number (u16, ascending), a value
//! (u32) and a flag (0 or 1). Inventory counts are such records; the object numbers differ from
//! save to save, so the coins are found by their surroundings: the record three object numbers
//! before is a small count (the rewire tools, runes or similar) and the one after is an
//! ordinary 1. Only a single clear candidate is offered, and the value is checked against the
//! game's own display (coins 77342 in one save, 74375 in another).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Dishonored save";
pub const COINS_MAX: i64 = 999_999;

fn rec(d: &[u8], o: usize) -> Option<(u16, u32, u8)> {
    let b = d.get(o..o + 7)?;
    Some((u16::from_le_bytes([b[0], b[1]]), u32::from_le_bytes([b[2], b[3], b[4], b[5]]), b[6]))
}

/// The offset of the coin count, when exactly one record fits.
fn coins(d: &[u8]) -> Option<usize> {
    let mut found = Vec::new();
    for o in 35..d.len().saturating_sub(14) {
        let Some((id, v, f)) = rec(d, o) else { break };
        if f > 1 || !(100..=5_000_000).contains(&v) || id < 100 {
            continue;
        }
        let (Some(b), Some(a)) = (rec(d, o - 7), rec(d, o + 7)) else { continue };
        let ascending = (1..5).all(|j| rec(d, o - 7 * j).zip(rec(d, o - 7 * (j + 1))).is_some_and(|(x, y)| y.0 < x.0));
        let (id, bi, ai) = (id as u32, b.0 as u32, a.0 as u32);
        if bi + 3 == id && (ai == id + 2 || ai == id + 3) && b.2 <= 1 && a.2 <= 1 && b.1 <= 30 && a.1 <= 30 && ascending {
            found.push(o + 2);
        }
    }
    (found.len() == 1).then(|| found[0])
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let at = coins(d).ok_or("not a Dishonored save")?;
    let mut b = Builder::new(d);
    b.ranged(&["Corvo", "Coins"], at, Kind::U32, 0, COINS_MAX);
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Coins. Tried in the game on a save from the end of a mission. Back up the save first: runes, rewire tools and elixirs aren't located yet.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u16, v: u32, f: u8) -> Vec<u8> {
        let mut r = id.to_le_bytes().to_vec();
        r.extend(v.to_le_bytes());
        r.push(f);
        r
    }

    fn sample(coins: u32) -> Vec<u8> {
        let mut d = vec![0u8; 40];
        for (id, v) in [(240u16, 1u32), (245, 1), (250, 2), (262, 1), (272, 9), (275, coins), (277, 1), (280, 1)] {
            d.extend(record(id, v, 0));
        }
        d.extend(vec![0u8; 64]);
        d
    }

    #[test]
    fn finds_the_coins() {
        let d = sample(77342);
        let at = coins(&d).unwrap();
        assert_eq!(u32::from_le_bytes(d[at..at + 4].try_into().unwrap()), 77342);
        assert!(parse(&d).is_ok());
    }

    #[test]
    fn refuses_unclear_data() {
        assert!(coins(&vec![0u8; 400]).is_none());
    }

    /// Research aid: SAVE_EDITOR_DIS1 = a Dishonored `.sav`, SAVE_EDITOR_DIS1_COINS = its coins.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_DIS1").unwrap()).unwrap();
        let (_, v) = crate::fmt::ue3pkg::unpack(&d).unwrap();
        let at = coins(&v).unwrap();
        let n = u32::from_le_bytes(v[at..at + 4].try_into().unwrap());
        assert_eq!(n.to_string(), std::env::var("SAVE_EDITOR_DIS1_COINS").unwrap());
        // Optional: write a copy with 99999 coins to try in the game.
        if let Ok(out) = std::env::var("SAVE_EDITOR_DIS1_OUT") {
            let (l, mut v) = crate::fmt::ue3pkg::unpack(&d).unwrap();
            v[at..at + 4].copy_from_slice(&99_999u32.to_le_bytes());
            let file = crate::fmt::ue3pkg::pack(&l, &v).unwrap();
            let (_, back) = crate::fmt::ue3pkg::unpack(&file).unwrap();
            assert_eq!(coins(&back), Some(at));
            assert_eq!(&back[at..at + 4], &99_999u32.to_le_bytes());
            std::fs::write(out, file).unwrap();
        }
    }
}
