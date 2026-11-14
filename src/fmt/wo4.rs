//! Warriors Orochi 4 (`SAVEDATA.BIN`, revision 20180720) and Warriors Orochi 4 Ultimate
//! (`SAVEDATAU.BIN`, revisions 20191201 and 20200101), 927,576 bytes: two AES-128-CBC blocks
//! (system, then gameplay), each followed by a four-byte seed that makes its IV. The key comes
//! from ten steps of x = (3.66 * x) * (1 - x) starting at (block + 1) / 4, then an LCG. A block
//! ends in the marker `FingerPrint01234`; there is no other checksum. Envelope and field
//! offsets follow the Universal Koei Tecmo Save Editor by Sanrenan (MIT), checked on public
//! saves of both editions.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

pub const KIND: &str = "Warriors Orochi 4 save";
const SIZE: usize = 927_576;
const SYS: usize = 0x6E734;
const MARK: &[u8; 16] = b"FingerPrint01234";
const GAME: usize = SYS; // gameplay block starts here; officers at GAME + id * 0x30C

fn key_iv(block: u32, seed: u32) -> ([u8; 16], [u8; 16]) {
    let mut x = f64::from(block + 1) * 0.5 * 0.5;
    for _ in 0..10 {
        x = (3.66 * x) * (1.0 - x);
    }
    let mut s = (x.to_bits() & 0xFFFF_FFFF) as u32;
    let mut key = [0u8; 16];
    for k in &mut key {
        s = s.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        *k = (s >> 24) as u8;
    }
    let mut t = seed;
    let mut iv = [0u8; 16];
    for v in &mut iv {
        t = t.wrapping_mul(0x015A_4E35).wrapping_add(1);
        *v = (t >> 16) as u8;
    }
    (key, iv)
}

fn le32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

fn le16(d: &[u8], at: usize) -> u32 {
    u32::from(u16::from_le_bytes([d[at], d[at + 1]]))
}

fn blocks() -> [(u32, usize, usize); 2] {
    [(1, 0, SYS), (0, SYS, SIZE)]
}

/// The file with both blocks decrypted in place (their seeds stay where they are).
fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() != SIZE {
        return None;
    }
    let mut p = d.to_vec();
    for (idx, a, b) in blocks() {
        let seed = le32(d, b - 4);
        let (k, iv) = key_iv(idx, seed);
        cbc::Decryptor::<aes::Aes128>::new(&k.into(), &iv.into()).decrypt_padded_mut::<NoPadding>(&mut p[a..b - 4]).ok()?;
        if &p[b - 20..b - 4] != MARK {
            return None;
        }
    }
    Some(p)
}

fn pack(p: &mut Vec<u8>) {
    if p.len() != SIZE {
        return;
    }
    for (idx, a, b) in blocks() {
        let seed = le32(p, b - 4);
        let (k, iv) = key_iv(idx, seed);
        let n = b - 4 - a;
        let _ = cbc::Encryptor::<aes::Aes128>::new(&k.into(), &iv.into()).encrypt_padded_mut::<NoPadding>(&mut p[a..b - 4], n);
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && unpack(d).is_some_and(|p| matches!(le32(&p, 0), 20_180_720 | 20_191_201 | 20_200_101) && le32(&p, 4) == 0 && le32(&p, 8) == 0)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let p = unpack(d).ok_or("not a Warriors Orochi 4 save")?;
    let rev = le32(&p, 0);
    if !matches!(rev, 20_180_720 | 20_191_201 | 20_200_101) {
        return Err("a Warriors Orochi 4 save revision this editor doesn't know".into());
    }
    let ultimate = rev != 20_180_720;
    let mut b = Builder::new(&p);
    b.info(&["Save", "Edition"], if ultimate { "Ultimate" } else { "Original" });
    b.info(&["Save", "Revision"], rev.to_string());
    b.ranged(&["Resources", "Gems"], 0x274, Kind::U32, 0, 9_999_999);
    b.ranged(&["Resources", "Crystals"], 0x27C, Kind::U16, 0, 999);
    b.ranged(&["Resources", "Growth Points"], 0x280, Kind::U32, 0, if ultimate { 99_999_999 } else { 999_999 });
    let orbs = le16(&p, 0x30E);
    if (1..=999).contains(&orbs) {
        b.ranged(&["Resources", "Skill Orbs (can only be lowered)"], 0x30E, Kind::U16, 0, i64::from(orbs));
    }
    let cap = if ultimate { 9999 } else { 999 };
    for id in 1..64usize {
        let q = le16(&p, 0x28E + 2 * id);
        if (1..=cap).contains(&q) {
            b.ranged(&["Weapon elements", &format!("Element {id}")], 0x28E + 2 * id, Kind::U16, 1, i64::from(cap));
        }
    }
    // Officers' unspent skill points (original edition; Ultimate's officer records are not mapped).
    if !ultimate {
        for id in 0..200usize {
            let at = GAME + id * 0x30C;
            let exp = le32(&p, at);
            if (1..=999_999).contains(&exp) {
                b.info(&["Officers", &format!("Officer {}", id + 1), "Experience"], i64::from(exp));
                b.ranged(&["Officers", &format!("Officer {}", id + 1), "Skill points"], at + 20, Kind::U8, 0, 99);
            }
        }
    }
    Ok(b.finish(KIND, Checksum::Repack(pack)).with_note(crate::i18n::tx("Close the game first. Both encrypted blocks are written again with their own seeds. Weapon elements and officers are shown by number; only existing elements (1 or more) and progressed officers are listed.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut p = vec![0u8; SIZE];
        p[..4].copy_from_slice(&20_180_720u32.to_le_bytes());
        p[0x274..0x278].copy_from_slice(&5000u32.to_le_bytes());
        p[0x28E + 4..0x28E + 6].copy_from_slice(&7u16.to_le_bytes());
        p[GAME..GAME + 4].copy_from_slice(&1234u32.to_le_bytes());
        p[GAME + 20] = 3;
        for (_, _, b) in blocks() {
            p[b - 20..b - 4].copy_from_slice(MARK);
            p[b - 4..b].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        }
        pack(&mut p);
        p
    }

    #[test]
    fn round_trips_and_edits() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Resources/Gems").unwrap(), 5000);
        assert_eq!(doc.view.pointer("/Weapon elements/Element 2").unwrap(), 7);
        doc.set("/Resources/Gems", 9_999_999.into()).unwrap();
        doc.set("/Officers/Officer 1/Skill points", 99.into()).unwrap();
        assert!(doc.set("/Resources/Crystals", 1000.into()).is_err());
        let out = doc.to_bytes().unwrap();
        let p = unpack(&out).unwrap();
        assert_eq!(le32(&p, 0x274), 9_999_999);
        assert_eq!(p[GAME + 20], 99);
    }
}
