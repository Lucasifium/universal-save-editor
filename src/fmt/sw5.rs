//! Samurai Warriors 5 (`SAVEDATA00.BIN`, 4,493,236 bytes, stream revision 39): AES-128-CBC over
//! everything but the last four bytes, which hold the seed of the IV. The key comes from the
//! Steam account number: ten steps of x = (3.66 * x) * (1 - x) from 1/12, the low 32 bits of the
//! result XOR the account, then an LCG (`* 0x343FD + 0x269EC3`, high byte). The decrypted data
//! ends in `01Finger2Print34`; there is no other checksum. Envelope and field offsets follow the
//! Universal Koei Tecmo Save Editor by Sanrenan (MIT), checked on a public save.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

pub const KIND: &str = "Samurai Warriors 5 save";
pub const SIZE: usize = 4_493_236;
const FINGERPRINT: &[u8; 16] = b"01Finger2Print34";
const PAD_START: usize = 4_493_211;
const MANAGER_END: usize = 0x18CF9B;
const SECOND_RESUME: usize = 0x2EAF9B;

fn key(account: u32) -> [u8; 16] {
    let mut x = 0.5_f64 / 6.0;
    x = (1.0 - x) * (x * 3.66);
    for _ in 0..9 {
        x = (x * 3.66) * (1.0 - x);
    }
    let mut s = (x.to_bits() as u32) ^ account;
    let mut k = [0u8; 16];
    for b in &mut k {
        s = s.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        *b = (s >> 24) as u8;
    }
    k
}

fn iv(seed: u32) -> [u8; 16] {
    let mut s = seed;
    let mut v = [0u8; 16];
    for b in &mut v {
        s = s.wrapping_mul(0x015A_4E35).wrapping_add(1);
        *b = (s >> 16) as u8;
    }
    v
}

fn seed_of(d: &[u8]) -> u32 {
    u32::from_le_bytes([d[SIZE - 4], d[SIZE - 3], d[SIZE - 2], d[SIZE - 1]])
}

/// The decrypted data (without the seed) when the account opens the file.
pub fn decrypt(d: &[u8], account: u32) -> Option<Vec<u8>> {
    if d.len() != SIZE {
        return None;
    }
    // Only the last two blocks are needed to test a key: the fingerprint is the final block.
    let (k, v) = (key(account), iv(seed_of(d)));
    let tail = &d[SIZE - 4 - 32..SIZE - 4];
    // CBC: plain block = D(cipher block) xor previous cipher block.
    let mut blk = [0u8; 16];
    blk.copy_from_slice(&tail[16..]);
    cbc::Decryptor::<aes::Aes128>::new(&k.into(), &[0u8; 16].into()).decrypt_padded_mut::<NoPadding>(&mut blk).ok()?;
    for (b, p) in blk.iter_mut().zip(&tail[..16]) {
        *b ^= p;
    }
    if &blk != FINGERPRINT {
        return None;
    }
    let mut p = d[..SIZE - 4].to_vec();
    cbc::Decryptor::<aes::Aes128>::new(&k.into(), &v.into()).decrypt_padded_mut::<NoPadding>(&mut p).ok()?;
    valid(&p).then_some(p)
}

pub fn find(d: &[u8], accounts: &[u32]) -> Option<(u32, Vec<u8>)> {
    accounts.iter().find_map(|&a| decrypt(d, a).map(|p| (a, p)))
}

pub fn encrypt(plain: &[u8], account: u32, seed: u32) -> Result<Vec<u8>, String> {
    if plain.len() != SIZE - 4 {
        return Err("not a Samurai Warriors 5 save".into());
    }
    let mut p = plain.to_vec();
    let n = p.len();
    cbc::Encryptor::<aes::Aes128>::new(&key(account).into(), &iv(seed).into()).encrypt_padded_mut::<NoPadding>(&mut p, n).map_err(|e| e.to_string())?;
    p.extend(seed.to_le_bytes());
    Ok(p)
}

/// The seed stored at the end of the encrypted file.
pub fn seed(raw: &[u8]) -> u32 {
    seed_of(raw)
}

fn valid(p: &[u8]) -> bool {
    p.len() == SIZE - 4
        && p[p.len() - 16..] == *FINGERPRINT
        && p[..8] == [39, 0, 0, 0, 39, 0, 0, 0]
        && p[PAD_START..p.len() - 16].iter().all(|&b| b == 0)
        && p[MANAGER_END] <= 1
        && p[SECOND_RESUME] == 0
}

/// The decrypted data of a Samurai Warriors 5 save.
pub fn looks(d: &[u8]) -> bool {
    valid(d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !valid(d) {
        return Err("not a Samurai Warriors 5 save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Resources", "Gold"], 0xD4, Kind::U32, 0, 9_999_999);
    b.ranged(&["Dojo", "Stock EXP"], 0xDC, Kind::U32, 0, 9_999_999);
    b.ranged(&["Dojo", "Shared skill points"], 0xE0, Kind::U32, 0, 99_999);
    b.ranged(&["Dojo", "Stock weapon mastery"], 0xE4, Kind::U32, 0, 9_999_999);
    b.ranged(&["Building materials", "Lumber"], 0x2A48, Kind::U16, 0, 9_999);
    b.ranged(&["Building materials", "Mineral"], 0x2A4A, Kind::U16, 0, 9_999);
    b.ranged(&["Building materials", "Fabric"], 0x2A4C, Kind::U16, 0, 9_999);
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the game first. The save is decrypted with your Steam account number and encrypted again on save. Only saves made in My Castle (not in a battle) are opened.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_edits() {
        let mut p = vec![0u8; SIZE - 4];
        p[..8].copy_from_slice(&[39, 0, 0, 0, 39, 0, 0, 0]);
        let n = p.len();
        p[n - 16..].copy_from_slice(FINGERPRINT);
        p[0xD4..0xD8].copy_from_slice(&500u32.to_le_bytes());
        let raw = encrypt(&p, 123_456, 0xDEAD_BEEF).unwrap();
        assert_eq!(raw.len(), SIZE);
        assert!(decrypt(&raw, 123_457).is_none());
        let (acct, plain) = find(&raw, &[1, 123_456]).unwrap();
        assert_eq!(acct, 123_456);
        assert_eq!(plain, p);
        let mut doc = parse(&plain).unwrap();
        doc.set("/Resources/Gold", 9_999_999.into()).unwrap();
        assert!(doc.set("/Building materials/Lumber", 10_000.into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(encrypt(&out, acct, seed(&raw)).unwrap().len(), SIZE);
    }
}
