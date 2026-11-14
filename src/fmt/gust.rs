//! Atelier Sophie and Atelier Firis (original Steam PC, `GAMEDATA00`..., no extension).
//! A 32-byte header (01 33 79 C9 for Sophie, 01 33 A1 3D for Firis), then root blocks that each
//! start with a little-endian size and a zero-terminated name; inside them, tagged nodes keep
//! big-endian values. There is no checksum. Layout follows the Universal Koei Tecmo Save Editor
//! by Sanrenan (MIT) and was checked on public saves of both games.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const SOPHIE: &str = "Atelier Sophie save";
pub const FIRIS: &str = "Atelier Firis save";

fn le32(d: &[u8], at: usize) -> usize {
    d.get(at..at + 4).map_or(usize::MAX, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

/// The two games: (header magic, file size, name).
fn game(d: &[u8]) -> Option<(&'static str, bool)> {
    match (d.get(..4)?, d.len()) {
        ([0x01, 0x33, 0x79, 0xC9], 768_000) => Some((SOPHIE, true)),
        ([0x01, 0x33, 0xA1, 0x3D], 1_228_800) => Some((FIRIS, false)),
        _ => None,
    }
}

/// Root blocks by name → (start, end), walking from byte 32.
fn roots(d: &[u8]) -> Option<Vec<(String, usize, usize)>> {
    let mut out = vec![];
    let mut cur = 32;
    while cur + 8 < d.len() {
        let sz = le32(d, cur);
        if sz < 8 || cur + sz > d.len() {
            break;
        }
        let end = d[cur + 4..cur + sz].iter().position(|&b| b == 0)? + cur + 4;
        out.push((String::from_utf8(d[cur + 4..end].to_vec()).ok()?, cur, cur + sz));
        cur += sz;
    }
    (cur == d.len() || cur + 8 >= d.len() || out.len() > 25).then_some(out)
}

/// Data start of the node `name` inside `s..e` (the node is preceded by its own little-endian size).
fn node(d: &[u8], name: &str, s: usize, e: usize) -> Option<usize> {
    let m: Vec<u8> = name.bytes().chain([0]).collect();
    let hay = d.get(s + 4..e)?;
    let first = hay.windows(m.len()).position(|w| w == m)?;
    if hay[first + 1..].windows(m.len()).any(|w| w == m) {
        return None;
    }
    Some(s + 4 + first + m.len())
}

fn locate(d: &[u8]) -> Option<(&'static str, usize, Option<usize>)> {
    let (name, sophie) = game(d)?;
    let r = roots(d)?;
    let find = |n: &str| r.iter().find(|x| x.0 == n).map(|x| (x.1, x.2));
    let (ms, me) = find("money")?;
    let cole = node(d, "money", ms + 10, me)?;
    if cole + 4 > me || le32(d, cole - 10) != 18 {
        return None;
    }
    let tickets = if sophie {
        let (qs, qe) = find("quest")?;
        let t = node(d, "numOfTickets", qs, qe)?;
        ((le32(d, t) as u32).swap_bytes() == 4).then_some(t + 4)
    } else {
        None
    };
    Some((name, cole, tickets))
}

pub fn looks(d: &[u8]) -> bool {
    locate(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (name, cole, tickets) = locate(d).ok_or("not an Atelier Sophie or Firis save")?;
    let mut b = Builder::new(d);
    b.big_endian = true;
    b.info(&["Save", "Game"], if tickets.is_some() { "Atelier Sophie" } else { "Atelier Firis" });
    b.ranged(&["Resources", "Cole"], cole, Kind::U32, 0, 99_999_999);
    if let Some(t) = tickets {
        b.ranged(&["Resources", "Tess exchange tickets"], t, Kind::U32, 0, 9_999);
    }
    Ok(b.finish(name, Checksum::None).with_note(crate::i18n::tx("Close the game first. The saves have no checksum; item qualities and recipes are not touched.")))
}
