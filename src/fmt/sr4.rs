//! Saints Row IV (`sr4save_N.sr4s_pc`): a small header, then a run of sections, each a
//! `(id, version, size)` triple of u32s followed by its bytes, ended by an empty id-0 section.
//! The u32 at offset 8 is a CRC-32 (zero start, no final inversion) of everything from offset 12.
//! Section 0x0C starts with the player record: cash in cents, data clusters, total respect,
//! respect inside the current level and the level, in that order (the same order Saints Row:
//! The Third uses). Worked out from public practice saves taken across the whole story.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Saints Row IV save";
const MAGIC: u32 = 0x050C_0914;
const PLAYER: u32 = 0x0C;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn crc(d: &[u8]) -> u32 {
    let mut c = 0u32;
    for &b in &d[12..] {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg());
        }
    }
    c
}

/// Start and size of every section, if the whole file walks cleanly to its end.
fn sections(d: &[u8]) -> Option<Vec<(u32, usize, usize)>> {
    if u32_at(d, 0)? != MAGIC {
        return None;
    }
    let mut at = 24usize.checked_add(u32_at(d, 20)? as usize)?;
    let mut out = Vec::new();
    loop {
        let id = u32_at(d, at)?;
        let size = u32_at(d, at + 8)? as usize;
        let start = at + 12;
        let end = start.checked_add(size)?;
        if end > d.len() {
            return None;
        }
        if id == 0 && size == 0 && end == d.len() {
            return Some(out);
        }
        out.push((id, start, size));
        at = end;
    }
}

fn player(d: &[u8]) -> Option<usize> {
    sections(d)?.into_iter().find(|&(id, _, size)| id == PLAYER && size >= 24).map(|(_, start, _)| start)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && u32_at(d, 8) == Some(crc(d)) && player(d).is_some()
}

fn fix(d: &mut Vec<u8>) {
    let c = crc(d);
    d[8..12].copy_from_slice(&c.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let Some(p) = player(d).filter(|_| looks(d)) else {
        return Err("not a Saints Row IV save".into());
    };
    let mut b = Builder::new(d);
    // Cash is kept in hundredths of a dollar, as in Saints Row: The Third.
    b.ranged(&["Player", "Cash (in cents)"], p + 4, Kind::U32, 0, 2_000_000_000);
    b.ranged(&["Player", "Data clusters"], p + 8, Kind::U32, 0, 999_999);
    b.ranged(&["Player", "Respect level"], p + 20, Kind::U32, 0, 49);
    b.ranged(&["Player", "Respect in this level"], p + 16, Kind::U32, 0, 99_999_999);
    b.ranged(&["Player", "Total respect"], p + 12, Kind::U32, 0, 99_999_999);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the game first. Cash is in cents (500000 is $5,000). The level is stored counting from 0, so 49 is the top level. Changing the level alone doesn't change the respect the game thinks you have earned, so set total respect with it. The save's checksum is rewritten."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample() -> Vec<u8> {
        let mut d = Vec::new();
        for v in [MAGIC, 1, 0, 1, 1, 4, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        for (id, body) in [(0x1Du32, vec![0u8; 8]), (PLAYER, vec![0u8; 40])] {
            for v in [id, 1, body.len() as u32] {
                d.extend_from_slice(&v.to_le_bytes());
            }
            d.extend_from_slice(&body);
        }
        d.extend_from_slice(&[0u8; 12]);
        fix(&mut d);
        d
    }

    #[test]
    fn cash_is_written_and_the_checksum_fixed() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Player/Cash (in cents)", Value::from(123_400_000u64)).unwrap();
        let out = doc.to_bytes().unwrap();
        let p = player(&out).unwrap();
        assert_eq!(u32_at(&out, p + 4), Some(123_400_000));
        assert!(looks(&out));
    }

    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("SR4_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            assert!(looks(&d), "{:?}", e.path());
            let doc = parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d);
        }
    }
}
