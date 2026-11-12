//! Final Fantasy XIII (PC, Steam): `ff13-NN.dat`, 300,600 bytes. A plain 0x208-byte header
//! (the in-game title at 0x08), then the game data in 8-byte blocks, each encrypted on its own
//! with a key that depends on the block number.
//!
//! The cipher and its key schedule are the documented ones from the public FFXIII
//! save-editor notes (facts only; written here from the algorithm). The decrypted data
//! starts with `Sound::Snapshot`; its last four bytes (little-endian) hold the sum of every
//! fourth byte before the last eight. The gil is a big-endian u32 at 0x46B4 into the data.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Final Fantasy XIII save";
pub const HEAD: usize = 0x208;
const SIZE: usize = 300_600;
const CONST: u32 = 0xA165_2347;
const GIL: usize = 0x46B4;

struct Keys {
    ab: [(u32, u32); 32],
    sum: [u8; 32],
}

fn keys() -> Keys {
    let mut ab = [(0u32, 0u32); 32];
    let mut sum = [0u8; 32];
    let mut k: u64 = 0xC9FB_2201_23D7_E0EF;
    for i in 0..32 {
        let bytes = k.to_le_bytes();
        ab[i] = (u32::from_le_bytes(bytes[..4].try_into().unwrap()), u32::from_le_bytes(bytes[4..].try_into().unwrap()));
        let s: u32 = bytes.iter().map(|&b| u32::from(b)).sum();
        sum[i] = (8u32 * 0x78).wrapping_sub(s) as u8;
        k = k.wrapping_mul(5);
    }
    Keys { ab, sum }
}

/// The two gears of a block from its byte offset.
fn gears(bc: u64) -> (u32, u32) {
    let v = bc << 20;
    let (lo, hi) = (v as u32, (v >> 32) as u32);
    let g1 = lo | ((bc << 10) as u32) | (bc as u32);
    let (g1, cf) = {
        let (r, o) = g1.overflowing_add(CONST);
        (r, u32::from(o))
    };
    let g2 = (((bc / 8) * 2) as u32 | hi).wrapping_add(cf);
    (g1, g2)
}

pub fn decrypt(src: &[u8]) -> Vec<u8> {
    let k = keys();
    let mut out = vec![0u8; src.len()];
    for blk in 0..src.len() / 8 {
        let bc = blk * 8;
        let ks = k.sum[blk & 31];
        let mut b = [0u8; 8];
        for p in 0..8 {
            let c = src[bc + p];
            let x = if p == 0 { 0x45 ^ (blk as u8) ^ c } else { c ^ src[bc + p - 1] };
            b[p] = x.wrapping_add(ks);
        }
        let ta = u32::from_le_bytes(b[..4].try_into().unwrap());
        let tb = u32::from_le_bytes(b[4..].try_into().unwrap());
        let (ka, kb) = k.ab[blk & 31];
        let (g1, g2) = gears(bc as u64);
        let cf2 = u32::from(ta < ka);
        let na = ka ^ g1 ^ ta.wrapping_sub(ka);
        let nb = kb ^ g2 ^ tb.wrapping_sub(kb).wrapping_sub(cf2);
        out[bc..bc + 4].copy_from_slice(&nb.to_le_bytes());
        out[bc + 4..bc + 8].copy_from_slice(&na.to_le_bytes());
    }
    out
}

pub fn encrypt(plain: &[u8]) -> Vec<u8> {
    let k = keys();
    let mut out = vec![0u8; plain.len()];
    for blk in 0..plain.len() / 8 {
        let bc = blk * 8;
        let nb = u32::from_le_bytes(plain[bc..bc + 4].try_into().unwrap());
        let na = u32::from_le_bytes(plain[bc + 4..bc + 8].try_into().unwrap());
        let (ka, kb) = k.ab[blk & 31];
        let (g1, g2) = gears(bc as u64);
        let ta = (na ^ ka ^ g1).wrapping_add(ka);
        let cf2 = u32::from(ta < ka);
        let tb = (nb ^ kb ^ g2).wrapping_add(kb).wrapping_add(cf2);
        let mut b = [0u8; 8];
        b[..4].copy_from_slice(&ta.to_le_bytes());
        b[4..].copy_from_slice(&tb.to_le_bytes());
        let ks = k.sum[blk & 31];
        for p in 0..8 {
            let x = b[p].wrapping_sub(ks);
            out[bc + p] = if p == 0 { x ^ 0x45 ^ (blk as u8) } else { x ^ out[bc + p - 1] };
        }
    }
    out
}

fn checksum(d: &[u8]) -> u32 {
    let end = d.len().saturating_sub(8);
    (0..end).step_by(4).fold(0u32, |a, i| a.wrapping_add(u32::from(d[i])))
}

fn fix_sum(d: &mut Vec<u8>) {
    let n = d.len() - 4;
    let s = checksum(d);
    d[n..].copy_from_slice(&s.to_le_bytes());
}

/// The header and the decrypted data when this is an FFXIII save (the checksum must match).
pub fn unpack(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if d.len() != SIZE {
        return None;
    }
    let body = decrypt(&d[HEAD..]);
    let stored = u32::from_le_bytes(body[body.len() - 4..].try_into().ok()?);
    (body.starts_with(b"Sound::Snapshot") && checksum(&body) == stored).then(|| (d[..HEAD].to_vec(), body))
}

pub fn pack(head: &[u8], data: &[u8]) -> Vec<u8> {
    let mut out = head.to_vec();
    out.extend(encrypt(data));
    out
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE - HEAD && d.starts_with(b"Sound::Snapshot") && u32::from_le_bytes(d[d.len() - 4..].try_into().unwrap()) == checksum(d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not Final Fantasy XIII save data".into());
    }
    let mut b = Builder::new(d);
    b.big_endian = true;
    b.ranged(&["Party", "Gil"], GIL, Kind::U32, 0, 9_999_999);
    Ok(b.finish(KIND, Checksum::Custom(fix_sum)).with_note("Close the game first (and turn off Steam cloud saves). Only gil is shown so far."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cipher_round_trips() {
        let plain: Vec<u8> = (0..4096u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect();
        assert_eq!(decrypt(&encrypt(&plain)), plain);
    }
}
