//! Dynasty Warriors 7: Xtreme Legends Definitive Edition (`save.dat`, exactly 470,307 bytes):
//! a little-endian 16-bit word checksum and 16-bit seed, then the payload run through a DWORD
//! XOR stream (the state advances twice per word: state = state * 0x5B1A7851 + 0xCE4E) with a
//! separate byte stream for the last odd bytes. The checksum is the word sum of the payload
//! without its final byte. Envelope and field offsets follow the Universal Koei Tecmo Save
//! Editor by Sanrenan (MIT), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Dynasty Warriors 7: Xtreme Legends save";
const SIZE: usize = 0x72D23;
const P: usize = 4; // payload start in the file
const MAGIC: [u8; 4] = [0x00, 0x02, 0x08, 0x11];

fn w16(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

fn stream(payload: &mut [u8], seed: u32) {
    let end = payload.len() - payload.len() % 4;
    let mut st = seed;
    for c in payload[..end].chunks_exact_mut(4) {
        for _ in 0..2 {
            st = st.wrapping_mul(0x5B1A_7851).wrapping_add(0xCE4E);
        }
        let w = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) ^ st;
        c.copy_from_slice(&w.to_le_bytes());
    }
    let mut t = seed & 0xFF;
    for b in &mut payload[end..] {
        t = (t.wrapping_mul(0x51).wrapping_add(0x4E)).wrapping_mul(0x51).wrapping_add(0x4E) & 0xFF;
        *b ^= t as u8;
    }
}

fn word_sum(p: &[u8]) -> u16 {
    p[..p.len() - 1].chunks_exact(2).fold(0u16, |a, c| a.wrapping_add(u16::from_le_bytes([c[0], c[1]])))
}

fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() != SIZE {
        return None;
    }
    let mut p = d.to_vec();
    stream(&mut p[P..], u32::from(w16(d, 2)));
    (word_sum(&p[P..]) == w16(d, 0) && p[P..P + 4] == MAGIC).then_some(p)
}

fn pack(p: &mut Vec<u8>) {
    if p.len() != SIZE {
        return;
    }
    let ck = word_sum(&p[P..]);
    p[..2].copy_from_slice(&ck.to_le_bytes());
    let seed = u32::from(w16(p, 2));
    stream(&mut p[P..], seed);
}

pub fn looks(d: &[u8]) -> bool {
    unpack(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let p = unpack(d).ok_or("not a Dynasty Warriors 7 Definitive Edition save")?;
    let mut b = Builder::new(&p);
    b.ranged(&["Army", "Gold"], P + 0xC7C, Kind::U32, 0, 999_999);
    for i in 0..65usize {
        let at = P + 0x2039 + i * 0x80;
        if w16(&p, at + 6) == 0 {
            continue;
        }
        let n = format!("Officer {:02}", i + 1);
        b.ranged(&["Officers", &n, "Health"], at + 6, Kind::U16, 0, 1000);
        b.ranged(&["Officers", &n, "Attack"], at + 8, Kind::U16, 1, 1400);
        b.ranged(&["Officers", &n, "Defense"], at + 10, Kind::U16, 1, 1400);
        b.ranged(&["Officers", &n, "Power"], at + 12, Kind::U16, 0, 100);
        b.ranged(&["Officers", &n, "Speed"], at + 14, Kind::U16, 0, 100);
        b.ranged(&["Officers", &n, "Skill points"], at + 26, Kind::U16, 0, 9999);
    }
    Ok(b.finish(KIND, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. The stream cipher and the word checksum are written again. Officers are shown by their number in the save, not by name.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_edits() {
        let mut p = vec![0u8; SIZE];
        p[2..4].copy_from_slice(&0x1234u16.to_le_bytes());
        p[P..P + 4].copy_from_slice(&MAGIC);
        p[P + 0xC7C..P + 0xC80].copy_from_slice(&1234u32.to_le_bytes());
        let o = P + 0x2039;
        p[o + 6..o + 8].copy_from_slice(&300u16.to_le_bytes());
        pack(&mut p);
        let d = p;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Army/Gold", 999_999.into()).unwrap();
        doc.set("/Officers/Officer 01/Attack", 1400.into()).unwrap();
        assert!(doc.set("/Officers/Officer 01/Power", 500.into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert!(unpack(&out).is_some());
        assert_eq!(parse(&out).unwrap().view.pointer("/Army/Gold").unwrap(), 999_999);
    }
}
