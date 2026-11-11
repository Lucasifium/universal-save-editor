//! Capcom RE Engine "Lime" saves (the first `DSSS` encryption with a per-user key): Resident
//! Evil 4 (2023) on Steam.
//!
//! Ported from mi5hmash's LimebrellaSharp (MIT, github.com/mi5hmash/LimebrellaSharp).
//!
//! A file is a 16-byte header ("DSSS", 2, 0x10, 0), then segments of 0x1220 bytes, then the
//! same 0x8C-byte footer as the later "Mandarin" saves (random bytes, the plain length and a
//! MurmurHash3 of everything before it). Each segment holds four key banks (a 64-byte header
//! and a 64-byte key fragment each), 0x1000 bytes of data encrypted with AES-128 in OFB mode,
//! and the SHA3-256 of the plain data. A bank's 64-bit key word is the fragment divided by
//! the bank header raised to the bitwise NOT of the Steam account number, modulo a fixed
//! 256-bit prime: so only the right account opens the save, and the first segment's
//! checksum tells a right account from a wrong one.
//!
//! Writing keeps every segment's key banks (so each keeps its key) and recomputes the
//! checksums, the footer length and the signature.

use super::mandarin;
use sha3::{Digest, Sha3_256};

const HEADER: usize = 0x10;
const FOOTER: usize = 0x8C;
const BANKS: usize = 4 * 0x80;
const DATA: usize = 0x1000;
const SEGMENT: usize = BANKS + DATA + 0x20;
/// The first bank header of every segment (the fixed "header key").
const FIRST_BANK: [u64; 4] = [0x5B49_D502_17C8_39BB, 0x772B_EEF5_D244_1867, 0x6E23_6B07_6EEB_11B8, 0x1216_F542_E37C_EE41];

pub fn looks(d: &[u8]) -> bool {
    d.len() >= HEADER + SEGMENT + FOOTER
        && (d.len() - HEADER - FOOTER) % SEGMENT == 0
        && d.starts_with(b"DSSS")
        && d[4..8] == [2, 0, 0, 0]
        && d[8..12] == [0x10, 0, 0, 0]
        && (0..4).all(|i| d[HEADER + i * 8..HEADER + i * 8 + 8] == FIRST_BANK[i].to_le_bytes())
}

fn plain_len(d: &[u8]) -> usize {
    u64::from_le_bytes(d[d.len() - 12..d.len() - 4].try_into().unwrap()) as usize
}

/// The user id the game uses: the Steam account number (the folder under `Steam\userdata`),
/// bitwise NOT. A 64-bit Steam ID is cut to its account number.
pub fn user_id(account: u64) -> u64 {
    !(account & 0xFFFF_FFFF)
}

fn segments(d: &[u8]) -> impl Iterator<Item = &[u8]> {
    d[HEADER..d.len() - FOOTER].chunks_exact(SEGMENT)
}

fn decrypt_segment(seg: &[u8], user: u64) -> Vec<u8> {
    let key = mandarin::unwrap_key(&seg[..BANKS], user);
    let mut body = seg[BANKS..BANKS + DATA].to_vec();
    mandarin::ofb(&mut body, &key);
    body
}

/// Does `user` open this file? (Decrypts and checks the first segment only.)
pub fn id_matches(d: &[u8], user: u64) -> bool {
    let Some(seg) = segments(d).next() else { return false };
    Sha3_256::digest(decrypt_segment(seg, user))[..] == seg[BANKS + DATA..]
}

/// The data inside, for `user`; `None` unless the first segment's checksum matches.
pub fn decrypt(d: &[u8], user: u64) -> Option<Vec<u8>> {
    if !looks(d) || !id_matches(d, user) {
        return None;
    }
    let segs: Vec<&[u8]> = segments(d).collect();
    let mut out = crate::par::map(&segs, |s| decrypt_segment(s, user)).concat();
    let n = plain_len(d);
    if n > out.len() {
        return None;
    }
    out.truncate(n);
    Some(out)
}

/// A file holding `plain`, built from `like` (the file it was read from): its header, its
/// segments' key banks and its footer's random bytes are kept.
pub fn encrypt(plain: &[u8], like: &[u8], user: u64) -> Result<Vec<u8>, String> {
    if !looks(like) {
        return Err("not a Lime save".into());
    }
    let old: Vec<&[u8]> = segments(like).collect();
    let count = plain.len().div_ceil(DATA).max(1);
    let plan: Vec<(usize, &[u8])> = (0..count).map(|i| (i, old[i.min(old.len() - 1)])).collect();
    let built = crate::par::map(&plan, |(i, model)| {
        let key = mandarin::unwrap_key(&model[..BANKS], user);
        let mut body = vec![0u8; DATA];
        let from = i * DATA;
        let to = (from + DATA).min(plain.len());
        body[..to - from].copy_from_slice(&plain[from..to]);
        let sum = Sha3_256::digest(&body);
        mandarin::ofb(&mut body, &key);
        [&model[..BANKS], &body[..], &sum[..]].concat()
    });
    let mut out = like[..HEADER].to_vec();
    for b in built {
        out.extend(b);
    }
    let mut footer = like[like.len() - FOOTER..].to_vec();
    footer[0x80..0x88].copy_from_slice(&(plain.len() as u64).to_le_bytes());
    out.extend(footer);
    mandarin::sign(&mut out);
    Ok(out)
}

/// The account (of `accounts`) that opens this file, as a user id.
pub fn find_user(d: &[u8], accounts: &[u64]) -> Option<u64> {
    accounts.iter().map(|&a| user_id(a)).find(|&u| id_matches(d, u))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LimebrellaSharp's own test files (QualityControl.xUnit/Resources), if LIME_VECTORS
    /// points at them: Steam id 76561197960265729 (account 1).
    #[test]
    #[ignore]
    fn limebrella_vectors() {
        let dir = std::path::PathBuf::from(std::env::var("LIME_VECTORS").expect("LIME_VECTORS"));
        let enc = std::fs::read(dir.join("encrypted.bin")).unwrap();
        let plain = std::fs::read(dir.join("decrypted.bin")).unwrap();
        assert!(looks(&enc));
        assert_eq!(find_user(&enc, &[5, 76_561_197_960_265_729]), Some(user_id(1)));
        assert_eq!(decrypt(&enc, user_id(1)).unwrap(), plain);
        let again = encrypt(&plain, &enc, user_id(1)).unwrap();
        assert_eq!(again, enc, "unchanged data must give the file back byte for byte");
        let mut edited = plain.clone();
        edited[100] ^= 0xFF;
        edited.extend([7u8; 5000]);
        let e = encrypt(&edited, &enc, user_id(1)).unwrap();
        assert_eq!(decrypt(&e, user_id(1)).unwrap(), edited);
    }

    #[test]
    fn synthetic_round_trip() {
        // A file whose banks wrap a known key for account 42, built the way the game does.
        use num_bigint::BigUint;
        let k = mandarin::keys();
        let user = user_id(42);
        let mut banks = vec![0u8; BANKS];
        let hk = &k.header_key;
        let a = hk.modpow(&BigUint::from(user), &k.p1);
        for (i, part) in [11u64, 22, 33, 44].into_iter().enumerate() {
            banks[i * 128..i * 128 + 64].copy_from_slice(&mandarin::limbs(hk, 8));
            banks[i * 128 + 64..i * 128 + 128].copy_from_slice(&mandarin::limbs(&(&a * BigUint::from(part)), 8));
        }
        let mut like = b"DSSS\x02\0\0\0\x10\0\0\0\0\0\0\0".to_vec();
        like.extend(&banks);
        like.extend(vec![0u8; DATA + 0x20]);
        like.extend(vec![0u8; FOOTER]);
        assert!(looks(&like), "header key bank");
        let plain: Vec<u8> = (0..9000u32).map(|i| (i % 251) as u8).collect();
        let file = encrypt(&plain, &like, user).unwrap();
        assert!(looks(&file));
        assert_eq!(find_user(&file, &[1, 42]), Some(user));
        assert_eq!(decrypt(&file, user).unwrap(), plain);
        assert!(decrypt(&file, user_id(43)).is_none());
    }
}
