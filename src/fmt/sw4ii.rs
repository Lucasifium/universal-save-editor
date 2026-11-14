//! Samurai Warriors 4-II (`SAVEDATA000n.dat`, exactly 0xC8210 bytes, payload revision 0x31A4):
//! a 0x20C-byte header, then a little-endian 16-bit word checksum and 16-bit seed, then a
//! 0xC8000-byte payload run through a DWORD XOR stream (the state advances three times per word:
//! state = state * 0x5B1A7851 + 0xCE4E). Four section sums are stored inside the payload and
//! all are written again. Envelope, section sums and field offsets follow the Universal Koei
//! Tecmo Save Editor by Sanrenan (MIT), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Samurai Warriors 4-II save";
const SIZE: usize = 0xC8210;
const CK: usize = 0x20C;
const P: usize = 0x210; // payload start in the file

fn w16(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

fn w32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

fn xor_stream(payload: &mut [u8], seed: u32) {
    let mut st = seed;
    for c in payload.chunks_exact_mut(4) {
        for _ in 0..3 {
            st = st.wrapping_mul(0x5B1A_7851).wrapping_add(0xCE4E);
        }
        let w = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) ^ st;
        c.copy_from_slice(&w.to_le_bytes());
    }
}

fn word_sum(p: &[u8]) -> u16 {
    p.chunks_exact(2).fold(0u16, |a, c| a.wrapping_add(u16::from_le_bytes([c[0], c[1]])))
}

/// The four section values stored in the payload: (a, b, c, d, e, f sums) → four dwords.
fn sections(pl: &[u8]) -> [u32; 4] {
    let bs = |a: usize, b: usize| pl[a..b].iter().fold(0u32, |s, &x| s.wrapping_add(u32::from(x)));
    let (a, b, c, d, e, f) = (bs(0x8, 0xA8), bs(0xAC, 0xA0C), bs(0xA0C, 0x104E), bs(0x1052, 0xCC36), bs(0xCC36, 0xD92E), bs(0xD92E, 0x5D86E));
    let c1 = (d.wrapping_add(a).wrapping_mul(b).wrapping_add(e)) & 0x7FFF_FFFF;
    let c2 = (d.wrapping_add(f).wrapping_add(b).wrapping_mul(a)) & 0x7FFF_FFFF;
    let c3 = a.wrapping_add(b).wrapping_add(c).wrapping_add(d) & 0x7FFF_FFFF;
    let c4 = c1.wrapping_add(c2).wrapping_add(c3) & 0x7FFF_FFFF;
    [c1, c2, c3, c4]
}

const SECTION_AT: [usize; 4] = [0x4, 0xA8, 0x104E, 0x5D86E];

fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() != SIZE {
        return None;
    }
    let mut p = d.to_vec();
    let seed = u32::from(w16(d, CK + 2));
    xor_stream(&mut p[P..], seed);
    if word_sum(&p[P..]) != w16(d, CK) {
        return None;
    }
    let s = sections(&p[P..]);
    if SECTION_AT.iter().zip(s).any(|(&at, v)| w32(&p, P + at) != v) {
        return None;
    }
    Some(p)
}

fn pack(p: &mut Vec<u8>) {
    if p.len() != SIZE {
        return;
    }
    let s = sections(&p[P..]);
    for (&at, v) in SECTION_AT.iter().zip(s) {
        p[P + at..P + at + 4].copy_from_slice(&v.to_le_bytes());
    }
    let ck = word_sum(&p[P..]);
    p[CK..CK + 2].copy_from_slice(&ck.to_le_bytes());
    let seed = u32::from(w16(p, CK + 2));
    xor_stream(&mut p[P..], seed);
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && unpack(d).is_some_and(|p| w32(&p, P) != 0 || w32(&p, P + 0x1052) != 0)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let p = unpack(d).ok_or("not a Samurai Warriors 4-II save")?;
    let mut b = Builder::new(&p);
    b.ranged(&["Army", "Gold"], P + 0xCB82, Kind::U32, 0, 999_999);
    for (i, c) in ["Red", "Green", "Blue", "Yellow", "Purple"].iter().enumerate() {
        b.ranged(&["Strategy tomes", c], P + 0xCC16 + 2 * i, Kind::U16, 0, 999);
    }
    for i in 0..56usize {
        let at = P + 0x1052 + i * 0x8C;
        let level = w32(&p, at + 0xC);
        if !(1..=50).contains(&level) {
            continue;
        }
        let n = format!("Officer {}", i + 1);
        b.info(&["Officers", &n, "Level"], i64::from(level));
        b.info(&["Officers", &n, "Experience"], i64::from(w32(&p, at + 8)));
        for (k, stat) in ["Health", "Attack", "Defense", "Riding", "Speed"].iter().enumerate() {
            b.ranged(&["Officers", &n, stat], at + 0x20 + 2 * k, Kind::U16, 0, 1000);
        }
    }
    Ok(b.finish(KIND, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. The stream cipher, the four section sums and the word checksum are written again. Officers are shown by their number in the save; levels and experience are read only.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_edits() {
        let mut p = vec![0u8; SIZE];
        p[CK + 2..CK + 4].copy_from_slice(&0xE01Cu16.to_le_bytes());
        p[P + 0xCB82..P + 0xCB86].copy_from_slice(&1234u32.to_le_bytes());
        let o = P + 0x1052;
        p[o..o + 4].copy_from_slice(&5u32.to_le_bytes());
        p[o + 0xC] = 20;
        p[o + 0x20] = 100;
        pack(&mut p);
        let d = p;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Army/Gold").unwrap(), 1234);
        doc.set("/Army/Gold", 999_999.into()).unwrap();
        doc.set("/Officers/Officer 1/Health", 350.into()).unwrap();
        assert!(doc.set("/Strategy tomes/Red", 5000.into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert!(unpack(&out).is_some());
        assert_eq!(parse(&out).unwrap().view.pointer("/Army/Gold").unwrap(), 999_999);
    }
}
