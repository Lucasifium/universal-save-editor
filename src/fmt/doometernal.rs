//! DOOM Eternal (Steam) save files (`profile.bin`, `game.details`, `game_duration.dat` and their
//! `-BACKUP` copies in `Steam\userdata\<account>\782330\remote`): AES-128-GCM, laid out as the
//! 12-byte nonce, the encrypted data and the 16-byte tag. The key is the first 16 bytes of the
//! SHA-256 of `<SteamID64>MANCUBUS<file name>`, and the same text is the associated data, so the
//! file must keep its own name and the account must be known.
//!
//! `profile.bin` opens as plain data and `game.details` as text. The level save
//! (`game_duration.dat`) is packed again inside with a compressor this editor doesn't include,
//! so only its outer layer is opened.

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use sha2::{Digest, Sha256};

/// Smallest file: nonce, tag and at least one byte.
const MIN: usize = 12 + 16 + 1;

pub const FILES: [&str; 6] = ["profile.bin", "game.details", "game_duration.dat", "profile.bin-BACKUP", "game.details-BACKUP", "game_duration.dat-BACKUP"];

/// Only files named like DOOM Eternal's are tried (the name is part of the key).
pub fn maybe(d: &[u8], name: &str) -> bool {
    d.len() >= MIN && FILES.iter().any(|f| f.eq_ignore_ascii_case(name))
}

fn key(steam: u64, name: &str) -> ([u8; 16], Vec<u8>) {
    let text = format!("{steam}MANCUBUS{name}").into_bytes();
    let h = Sha256::digest(&text);
    let mut k = [0u8; 16];
    k.copy_from_slice(&h[..16]);
    (k, text)
}

/// Multiplication in GF(2^128) as GCM defines it (blocks read big-endian).
fn gmul(x: u128, y: u128) -> u128 {
    let mut z = 0u128;
    let mut v = y;
    for i in 0..128 {
        if (x >> (127 - i)) & 1 == 1 {
            z ^= v;
        }
        v = if v & 1 == 1 { (v >> 1) ^ (0xE1u128 << 120) } else { v >> 1 };
    }
    z
}

fn ghash(h: u128, aad: &[u8], ct: &[u8]) -> u128 {
    let mut x = 0u128;
    for part in [aad, ct] {
        for c in part.chunks(16) {
            let mut b = [0u8; 16];
            b[..c.len()].copy_from_slice(c);
            x = gmul(x ^ u128::from_be_bytes(b), h);
        }
    }
    let lens = ((aad.len() as u128 * 8) << 64) | (ct.len() as u128 * 8);
    gmul(x ^ lens, h)
}

fn block(c: &Aes128, v: u128) -> u128 {
    let mut b = v.to_be_bytes().into();
    c.encrypt_block(&mut b);
    u128::from_be_bytes(b.into())
}

/// CTR keystream from the counter after J0, XORed over `data`.
fn ctr(c: &Aes128, j0: u128, data: &mut [u8]) {
    let mut counter = j0;
    for chunk in data.chunks_mut(16) {
        counter = (counter & !0xFFFF_FFFFu128) | u128::from((counter as u32).wrapping_add(1));
        let ks = block(c, counter).to_be_bytes();
        for (b, k) in chunk.iter_mut().zip(ks) {
            *b ^= k;
        }
    }
}

fn j0(nonce: &[u8]) -> u128 {
    let mut b = [0u8; 16];
    b[..12].copy_from_slice(nonce);
    b[15] = 1;
    u128::from_be_bytes(b)
}

pub fn decrypt(d: &[u8], steam: u64, name: &str) -> Option<Vec<u8>> {
    if d.len() < MIN {
        return None;
    }
    let (k, aad) = key(steam, name);
    let c = Aes128::new(&k.into());
    let (nonce, rest) = d.split_at(12);
    let (ct, tag) = rest.split_at(rest.len() - 16);
    let h = block(&c, 0);
    let j = j0(nonce);
    let want = block(&c, j) ^ ghash(h, &aad, ct);
    if want.to_be_bytes() != tag {
        return None;
    }
    let mut out = ct.to_vec();
    ctr(&c, j, &mut out);
    Some(out)
}

pub fn encrypt(plain: &[u8], nonce: &[u8; 12], steam: u64, name: &str) -> Vec<u8> {
    let (k, aad) = key(steam, name);
    let c = Aes128::new(&k.into());
    let j = j0(nonce);
    let mut ct = plain.to_vec();
    ctr(&c, j, &mut ct);
    let tag = block(&c, j) ^ ghash(block(&c, 0), &aad, &ct);
    let mut out = nonce.to_vec();
    out.extend(ct);
    out.extend(tag.to_be_bytes());
    out
}

/// SteamID64 values to try for account numbers (or ids) found for the file.
pub fn ids(accounts: &[u64]) -> Vec<u64> {
    let mut out = vec![];
    for &a in accounts {
        let id = if a < 1 << 32 { a + 76_561_197_960_265_728 } else { a };
        if !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// The account that opens the file, its nonce and the plain data.
pub fn find(d: &[u8], name: &str, accounts: &[u64]) -> Option<(u64, [u8; 12], Vec<u8>)> {
    let nonce: [u8; 12] = d.get(..12)?.try_into().ok()?;
    ids(accounts).into_iter().find_map(|s| decrypt(d, s, name).map(|p| (s, nonce, p)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NIST GCM test case 4 (AES-128, 60-byte plaintext, 20-byte associated data), run through
    /// the same pieces with the key and data given directly.
    #[test]
    fn gcm_matches_the_standard_vector() {
        let hex = |s: &str| (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect::<Vec<u8>>();
        let key: [u8; 16] = hex("feffe9928665731c6d6a8f9467308308").try_into().unwrap();
        let pt = hex("d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39");
        let aad = hex("feedfacedeadbeeffeedfacedeadbeefabaddad2");
        let nonce = hex("cafebabefacedbaddecaf888");
        let c = Aes128::new(&key.into());
        let j = j0(&nonce);
        let mut ct = pt.clone();
        ctr(&c, j, &mut ct);
        assert_eq!(ct, hex("42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091"));
        let tag = block(&c, j) ^ ghash(block(&c, 0), &aad, &ct);
        assert_eq!(tag.to_be_bytes().to_vec(), hex("5bc94fbc3221a5db94fae95ae7121a47"));
    }

    #[test]
    fn round_trips_with_the_right_account_only() {
        let plain = b"checksum=1\ncompleted=0\ndifficulty=2\n".to_vec();
        let d = encrypt(&plain, &[7; 12], 76_561_197_960_265_729, "game.details");
        assert!(maybe(&d, "game.details"));
        assert!(!maybe(&d, "other.bin"));
        let (id, nonce, p) = find(&d, "game.details", &[5, 1]).unwrap();
        assert_eq!((id, nonce, p.clone()), (76_561_197_960_265_729, [7; 12], plain.clone()));
        assert_eq!(encrypt(&p, &nonce, id, "game.details"), d);
        assert!(find(&d, "game.details", &[2]).is_none());
        assert!(find(&d, "game.details-BACKUP", &[1]).is_none(), "the name is part of the key");
    }
}
