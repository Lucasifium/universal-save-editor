//! Warriors All-Stars (`SAVEDATA.BIN`, 4,955,400 bytes): ten AES-128-CBC blocks, the system
//! block first (key 9) and then the nine campaign slots (keys 0 to 8). Each block ends in a
//! four-byte seed that makes its IV, and its decrypted data ends in `FingerPrint01234`; there is
//! no other checksum. Keys are LCG streams from ten fixed seeds. Envelope and field offsets follow
//! the Universal Koei Tecmo Save Editor by Sanrenan (MIT), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

pub const KIND: &str = "Warriors All-Stars save";
const SB: usize = 0x79B84; // system block, encrypted size
const LB: usize = 0x78E64; // slot block, encrypted size
const SIZE: usize = SB + 9 * LB;
const MARK: &[u8; 16] = b"FingerPrint01234";
const REV: u32 = 0x1703_02F4;
const SEEDS: [u32; 10] = [0x27C6_BAFB, 0xB3C7_CC4C, 0x8999_30C6, 0x7242_DA7E, 0x7731_8BB8, 0x9223_B091, 0x7E03_1D1B, 0x3743_9CE5, 0xFF8F_999F, 0x3A21_1944];
pub const MATERIAL_IDS: usize = 45;

fn lcg16(seed: u32, mul: u32, inc: u32, shift: u32) -> [u8; 16] {
    let mut s = seed;
    let mut out = [0u8; 16];
    for b in &mut out {
        s = s.wrapping_mul(mul).wrapping_add(inc);
        *b = (s >> shift) as u8;
    }
    out
}

/// (key index, start, end) of the ten blocks in file order.
fn blocks() -> Vec<(usize, usize, usize)> {
    let mut v = vec![(9, 0, SB)];
    v.extend((0..9).map(|i| (i, SB + i * LB, SB + (i + 1) * LB)));
    v
}

fn le32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

fn le16(d: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([d[at], d[at + 1]])
}

/// The file with every block decrypted in place (the seeds stay where they are).
fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() != SIZE {
        return None;
    }
    let mut p = d.to_vec();
    for (k, a, b) in blocks() {
        let key = lcg16(SEEDS[k], 0x343FD, 0x269EC3, 24);
        let iv = lcg16(le32(d, b - 4), 0x015A_4E35, 1, 16);
        cbc::Decryptor::<aes::Aes128>::new(&key.into(), &iv.into()).decrypt_padded_mut::<NoPadding>(&mut p[a..b - 4]).ok()?;
        if &p[b - 20..b - 4] != MARK {
            return None;
        }
    }
    (le32(&p, 0) == REV).then_some(p)
}

fn pack(p: &mut Vec<u8>) {
    if p.len() != SIZE {
        return;
    }
    for (k, a, b) in blocks() {
        let key = lcg16(SEEDS[k], 0x343FD, 0x269EC3, 24);
        let iv = lcg16(le32(p, b - 4), 0x015A_4E35, 1, 16);
        let n = b - 4 - a;
        let _ = cbc::Encryptor::<aes::Aes128>::new(&key.into(), &iv.into()).encrypt_padded_mut::<NoPadding>(&mut p[a..b - 4], n);
    }
}

pub fn looks(d: &[u8]) -> bool {
    unpack(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let p = unpack(d).ok_or("not a Warriors All-Stars save")?;
    let mut b = Builder::new(&p);
    for s in 0..9usize {
        let base = SB + s * LB;
        let gold = le32(&p, base + 0x2F6A);
        let owned = (0..MATERIAL_IDS).any(|i| le16(&p, base + 0x2F10 + 2 * i) > 0);
        if gold == 0 && !owned {
            continue;
        }
        let slot = format!("Slot {}", s + 1);
        b.ranged(&[&slot, "Gold"], base + 0x2F6A, Kind::U32, 0, 9_999_999);
        for i in 0..MATERIAL_IDS {
            let at = base + 0x2F10 + 2 * i;
            if (1..=9_999).contains(&le16(&p, at)) {
                b.ranged(&[&slot, "Materials", &format!("Material ID {i}")], at, Kind::U16, 0, 9_999);
            }
        }
    }
    if b.is_empty() {
        return Err("no campaign slot in use".into());
    }
    Ok(b.finish(KIND, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. All ten encrypted blocks are written again with their own seeds. Only slots in use are listed; materials are shown by their number in the save and only the ones you own.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_edits() {
        let mut p = vec![0u8; SIZE];
        for (_, _, b) in blocks() {
            p[b - 20..b - 4].copy_from_slice(MARK);
            p[b - 4..b].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        }
        p[..4].copy_from_slice(&REV.to_le_bytes());
        p[SB + 0x2F6A..SB + 0x2F6E].copy_from_slice(&1000u32.to_le_bytes());
        p[SB + 0x2F10 + 4..SB + 0x2F10 + 6].copy_from_slice(&7u16.to_le_bytes());
        pack(&mut p);
        let d = p;
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/Gold", 9_999_999.into()).unwrap();
        doc.set("/Slot 1/Materials/Material ID 2", 9999.into()).unwrap();
        assert!(doc.set("/Slot 1/Materials/Material ID 2", 10_000.into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert!(unpack(&out).is_some());
    }
}
