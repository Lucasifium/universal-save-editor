//! Grand Theft Auto III and Vice City (PC): `GTA3sf1.b` … and `GTAVCsf1.b` …, 22 and 24 blocks, each `[u32 size][data]`,
//! with the sum of every byte but the last four stored there as a u32 (little-endian).
//!
//! From public notes on the save format (facts): block 16 is the player info (money at 8
//! bytes into the block, on-screen money at 23, packages collected and total at 27 and 31) and
//! block 17 the statistics.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "GTA III save";
pub const KIND_VC: &str = "GTA Vice City save";

/// What differs between the two games: the block count, where the player info and the
/// statistics sit, their sizes, and the statistics' offsets (from the block's data).
struct Layout {
    kind: &'static str,
    blocks: usize,
    player: usize,
    player_size: usize,
    stats: usize,
    stats_size: usize,
    /// (label, offset, float, maximum)
    fields: &'static [(&'static str, usize, bool, i64)],
}

const GTA3: Layout = Layout {
    kind: KIND,
    blocks: 22,
    player: 16,
    player_size: 0x140,
    stats: 17,
    stats_size: 0x1A8,
    fields: &[
        ("Progress made", 112, false, 154),
        ("Total progress in the game", 116, false, 154),
        ("Days passed", 148, false, 99_999),
        ("Unique stunt jumps found", 176, false, 20),
        ("Total unique stunt jumps", 180, false, 20),
        ("Missions given", 184, false, 999),
        ("Missions passed", 188, false, 999),
        ("Times arrested", 140, false, 99_999),
        ("Times died", 144, false, 99_999),
        ("People killed by the player", 0, false, 9_999_999),
        ("Cars exploded", 8, false, 9_999_999),
        ("Rounds fired", 12, false, 99_999_999),
        ("Helicopters destroyed", 108, false, 99_999),
    ],
};

const VC: Layout = Layout {
    kind: KIND_VC,
    blocks: 24,
    player: 18,
    player_size: 372,
    stats: 19,
    stats_size: 600,
    fields: &[
        ("Progress made", 120, true, 154),
        ("Total progress in the game", 124, true, 154),
        ("Days passed", 156, false, 99_999),
        ("Unique stunt jumps found", 188, false, 36),
        ("Total unique stunt jumps", 192, false, 36),
        ("Missions given", 196, false, 999),
        ("Times arrested", 148, false, 99_999),
        ("Times died", 152, false, 99_999),
        ("People killed by the player", 0, false, 9_999_999),
        ("Cars exploded", 8, false, 9_999_999),
        ("Boats exploded", 12, false, 9_999_999),
        ("Rounds fired", 20, false, 99_999_999),
        ("Helicopters destroyed", 116, false, 99_999),
    ],
};

fn u32_at(d: &[u8], at: usize) -> usize {
    d.get(at..at + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
}

/// Start offsets of the outer blocks when they tile the file exactly up to the checksum
/// (a few padding blocks may follow the game's own).
fn blocks(d: &[u8], count: usize) -> Option<Vec<usize>> {
    let end = d.len().checked_sub(4)?;
    let mut at = 0;
    let mut v = Vec::new();
    while at + 4 <= end {
        let n = u32_at(d, at);
        if n == 0 || at + 4 + n > end {
            return None;
        }
        v.push(at);
        at += 4 + n;
    }
    (at == end && v.len() >= count && v.len() <= count + 4).then_some(v)
}

fn layout(d: &[u8]) -> Option<(&'static Layout, Vec<usize>)> {
    [&GTA3, &VC].into_iter().find_map(|l| {
        let b = blocks(d, l.blocks)?;
        (u32_at(d, b[l.player]) == l.player_size && u32_at(d, b[l.stats]) == l.stats_size && u32_at(d, b[l.player] + 4) == l.player_size - 4).then_some((l, b))
    })
}

pub fn looks(d: &[u8]) -> bool {
    layout(d).is_some()
}

fn fix_sum(d: &mut Vec<u8>) {
    let n = d.len() - 4;
    let sum = d[..n].iter().fold(0u32, |a, &b| a.wrapping_add(u32::from(b)));
    d[n..].copy_from_slice(&sum.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (l, bl) = layout(d).ok_or("not a GTA III or Vice City save")?;
    let p = bl[l.player] + 8;
    let s = bl[l.stats] + 8;
    let mut b = Builder::new(d);
    let g = "Player";
    b.ranged(&[g, "Money"], p, Kind::I32, -99_999_999, 99_999_999);
    b.ranged(&[g, "Money shown on screen"], p + 15, Kind::I32, -99_999_999, 99_999_999);
    b.ranged(&[g, "Hidden packages collected"], p + 19, Kind::I32, 0, 100);
    b.ranged(&[g, "Hidden packages in the game"], p + 23, Kind::I32, 0, 100);
    for &(label, off, float, max) in l.fields {
        let kind = if float { Kind::F32Int } else { Kind::I32 };
        b.ranged(&["Statistics", label], s + off, kind, 0, max);
    }
    Ok(b.finish(l.kind, Checksum::Custom(fix_sum)).with_note("Close the game first. The save's checksum is rewritten. Money is kept in two places and both should match; the on-screen figure is what counts up and down."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample() -> Vec<u8> {
        let mut d = Vec::new();
        for i in 0..GTA3.blocks {
            let n = match i {
                16 => 0x13C,
                17 => 0x1A4,
                _ => 8,
            };
            d.extend_from_slice(&(n as u32 + 4).to_le_bytes());
            d.extend_from_slice(&(n as u32).to_le_bytes());
            d.extend(std::iter::repeat(0).take(n));
        }
        d.extend_from_slice(&[0; 4]);
        fix_sum(&mut d);
        d
    }

    #[test]
    fn money_is_written_and_the_sum_fixed() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Player/Money", Value::from(5_000_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        let at = blocks(&out, 22).unwrap()[16] + 8;
        assert_eq!(u32_at(&out, at), 5_000_000);
        let n = out.len() - 4;
        assert_eq!(out[..n].iter().fold(0u32, |a, &b| a.wrapping_add(u32::from(b))), u32_at(&out, n) as u32);
    }
}
