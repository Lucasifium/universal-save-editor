//! Capcom RE Engine "Citrus" saves: Monster Hunter Rise (Steam), `data00?Slot.bin`.
//!
//! Ported from ChevalGrand's mhrise-save-editor (MIT, github.com/ChevalGrand/mhrise-save-editor).
//!
//! A file is a 16-byte header ("DSSS", 2, flags with the Citrus bit, padding), the encrypted
//! blocks, a 0x1000-byte tail, the plain length (u64) and a MurmurHash3 of everything before
//! it. Each block holds a 16-byte key and IV, 0x200 bytes of wrapped keys (AES-128-CBC with
//! that key, after an XOR of key ^ IV over every block but the first), 0x40000 bytes of data
//! encrypted the same way with the unwrapped key, and the SHA3-256 of the key, IV, wrapped
//! keys and plain data. The data key and IV are four 64-bit words, each the x coordinate (÷100)
//! of an EC-ElGamal message point; the private key is the bitwise NOT of the 64-bit Steam ID,
//! and the curve is one of 128 fixed ones (found by trying them).
//!
//! Writing keeps each block's keys (so nothing has to be wrapped again), re-encrypts the data
//! and recomputes the block hashes, the length and the signature.

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use sha3::{Digest, Sha3_256};

const HEADER: usize = 0x10;
const KEYS: usize = 0x200;
const DATA: usize = 0x40000;
const HASH: usize = 0x20;
const BLOCK: usize = 0x20 + KEYS + DATA + HASH;
const TAIL: usize = 0x1000;
const CITRUS: u32 = 0x04;
const DEFLATE: u32 = 0x08;

include!("citrus_curves.rs");

#[derive(Clone)]
struct Curve {
    p: BigInt,
    a: BigInt,
    #[cfg_attr(not(test), allow(dead_code))]
    g: (BigInt, BigInt),
}

fn int(hex: &str) -> BigInt {
    let b: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap()).collect();
    BigInt::from_bytes_le(Sign::Plus, &b)
}

fn curve(i: usize) -> Curve {
    let c = &CURVES[i];
    Curve { p: int(c[0]), a: int(c[1]), g: (int(c[3]), int(c[4])) }
}

type Point = (BigInt, BigInt);

fn inv(v: &BigInt, p: &BigInt) -> BigInt {
    v.modpow(&(p - BigInt::from(2)), p)
}

fn add(a: Option<Point>, b: Option<Point>, c: &Curve) -> Option<Point> {
    let Some((x1, y1)) = a else { return b };
    let Some((x2, y2)) = b else { return Some((x1, y1)) };
    let p = &c.p;
    if x1 == x2 && (y1 != y2 || y1 == BigInt::ZERO) {
        return None;
    }
    let slope = if x1 == x2 {
        (BigInt::from(3) * &x1 * &x1 + &c.a).mod_floor(p) * inv(&(BigInt::from(2) * &y1).mod_floor(p), p)
    } else {
        (&y2 - &y1).mod_floor(p) * inv(&(&x2 - &x1).mod_floor(p), p)
    }
    .mod_floor(p);
    let x3 = (&slope * &slope - &x1 - &x2).mod_floor(p);
    let y3 = (&slope * (&x1 - &x3) - &y1).mod_floor(p);
    Some((x3, y3))
}

fn mul(k: &BigInt, pt: Point, c: &Curve) -> Option<Point> {
    let mut result = None;
    let mut addend = Some(pt);
    let mut f = k.clone();
    while f > BigInt::ZERO {
        if f.is_odd() {
            result = add(result, addend.clone(), c);
        }
        addend = addend.and_then(|v| add(Some(v.clone()), Some(v), c));
        f >>= 1;
    }
    result
}

fn le32(n: &BigInt) -> [u8; 32] {
    let mut out = [0u8; 32];
    let b = n.to_bytes_le().1;
    let l = b.len().min(32);
    out[..l].copy_from_slice(&b[..l]);
    out
}

/// One 64-bit word of the data key from its 128-byte EC-ElGamal pair.
fn unwrap_word(seg: &[u8], d: &BigInt, c: &Curve) -> Option<u64> {
    let n = |r: std::ops::Range<usize>| BigInt::from_bytes_le(Sign::Plus, &seg[r]);
    let c1 = (n(0..32), n(32..64));
    let c2 = (n(64..96), n(96..128));
    let (sx, sy) = mul(d, c1, c)?;
    let m = add(Some(c2), Some((sx, (-sy).mod_floor(&c.p))), c)?;
    Some(u64::from_le_bytes(le32(&(m.0 / 100))[..8].try_into().unwrap()))
}

fn unwrap_keys(keys: &[u8], d: &BigInt, c: &Curve) -> Option<([u8; 16], [u8; 16])> {
    let mut w = [0u64; 4];
    for (i, x) in w.iter_mut().enumerate() {
        *x = unwrap_word(&keys[i * 128..i * 128 + 128], d, c)?;
    }
    let mut key = [0u8; 16];
    let mut iv = [0u8; 16];
    key[..8].copy_from_slice(&w[0].to_le_bytes());
    key[8..].copy_from_slice(&w[1].to_le_bytes());
    iv[..8].copy_from_slice(&w[2].to_le_bytes());
    iv[8..].copy_from_slice(&w[3].to_le_bytes());
    Some((key, iv))
}

/// AES-128-CBC with the game's twist: every block after the first is XORed with key ^ IV.
fn cbc_decrypt(buf: &mut [u8], key: &[u8; 16], iv: &[u8; 16]) {
    let aes = aes::Aes128::new(key.into());
    let mut prev = *iv;
    for chunk in buf.chunks_exact_mut(16) {
        let ct: [u8; 16] = chunk.try_into().unwrap();
        let mut b = aes::Block::clone_from_slice(chunk);
        aes.decrypt_block(&mut b);
        for i in 0..16 {
            chunk[i] = b[i] ^ prev[i];
        }
        prev = ct;
    }
    patch(buf, key, iv);
}

fn cbc_encrypt(buf: &mut [u8], key: &[u8; 16], iv: &[u8; 16]) {
    patch(buf, key, iv);
    let aes = aes::Aes128::new(key.into());
    let mut prev = *iv;
    for chunk in buf.chunks_exact_mut(16) {
        let mut b = aes::Block::clone_from_slice(chunk);
        for i in 0..16 {
            b[i] ^= prev[i];
        }
        aes.encrypt_block(&mut b);
        chunk.copy_from_slice(&b);
        prev = chunk.try_into().unwrap();
    }
}

fn patch(buf: &mut [u8], key: &[u8; 16], iv: &[u8; 16]) {
    for chunk in buf.chunks_exact_mut(16).skip(1) {
        for i in 0..16 {
            chunk[i] ^= key[i] ^ iv[i];
        }
    }
}

pub fn looks(d: &[u8]) -> bool {
    if d.len() < HEADER + BLOCK + TAIL + 12 || !d.starts_with(b"DSSS") || d[4..8] != [2, 0, 0, 0] {
        return false;
    }
    let flags = u32::from_le_bytes(d[8..12].try_into().unwrap());
    flags & (CITRUS | DEFLATE) == CITRUS && (d.len() - HEADER - TAIL - 12) % BLOCK == 0
}

fn blocks(d: &[u8]) -> impl Iterator<Item = &[u8]> {
    d[HEADER..d.len() - TAIL - 12].chunks_exact(BLOCK)
}

fn plain_len(d: &[u8]) -> usize {
    u64::from_le_bytes(d[d.len() - 12..d.len() - 4].try_into().unwrap()) as usize
}

/// The private key for a Steam ID (the 64-bit id; an account number is widened).
fn private(steam: u64) -> BigInt {
    let id = if steam > u32::MAX as u64 { steam } else { 0x0110_0001_0000_0000 | steam };
    BigInt::from(!id)
}

/// The keys a block's data is encrypted with, and the block hash check.
fn block_keys(b: &[u8], d: &BigInt, c: &Curve) -> Option<([u8; 16], [u8; 16], Vec<u8>)> {
    let key: [u8; 16] = b[..16].try_into().unwrap();
    let iv: [u8; 16] = b[16..32].try_into().unwrap();
    let mut wrapped = b[32..32 + KEYS].to_vec();
    cbc_decrypt(&mut wrapped, &key, &iv);
    let (k, v) = unwrap_keys(&wrapped, d, c)?;
    Some((k, v, wrapped))
}

fn decrypt_block(b: &[u8], d: &BigInt, c: &Curve) -> Option<Vec<u8>> {
    let (k, v, wrapped) = block_keys(b, d, c)?;
    let mut data = b[32 + KEYS..32 + KEYS + DATA].to_vec();
    cbc_decrypt(&mut data, &k, &v);
    let mut h = Sha3_256::new();
    h.update(&b[..32]);
    h.update(&wrapped);
    h.update(&data[..DATA - HASH]);
    (h.finalize()[..] == b[BLOCK - HASH..]).then_some(data)
}

/// The curve and Steam ID (of `accounts`) that open this file.
pub fn find_key(d: &[u8], accounts: &[u64]) -> Option<(usize, u64)> {
    let first = blocks(d).next()?;
    for &a in accounts {
        let pk = private(a);
        for i in 0..CURVES.len() {
            let c = curve(i);
            if decrypt_block(first, &pk, &c).is_some() {
                return Some((i, a));
            }
        }
    }
    None
}

pub fn decrypt(d: &[u8], curve_index: usize, steam: u64) -> Option<Vec<u8>> {
    if !looks(d) {
        return None;
    }
    let (pk, c) = (private(steam), curve(curve_index));
    let bs: Vec<&[u8]> = blocks(d).collect();
    let parts = crate::par::map(&bs, |b| decrypt_block(b, &pk, &c));
    let mut out = Vec::with_capacity(bs.len() * DATA);
    for p in parts {
        out.extend(p?);
    }
    let n = plain_len(d);
    if n > out.len() {
        return None;
    }
    out.truncate(n);
    Some(out)
}

/// A file holding `plain`, built from `like` (the file it was read from): its header, tail
/// and each block's keys are kept.
pub fn encrypt(plain: &[u8], like: &[u8], curve_index: usize, steam: u64) -> Result<Vec<u8>, String> {
    if !looks(like) {
        return Err("not a Citrus save".into());
    }
    let (pk, c) = (private(steam), curve(curve_index));
    let old: Vec<&[u8]> = blocks(like).collect();
    let count = plain.len().div_ceil(DATA).max(1);
    let plan: Vec<(usize, &[u8])> = (0..count).map(|i| (i, old[i.min(old.len() - 1)])).collect();
    let built = crate::par::map(&plan, |(i, model)| -> Result<Vec<u8>, String> {
        let (k, v, wrapped) = block_keys(model, &pk, &c).ok_or("the save's keys don't open with this account")?;
        let mut data = vec![0u8; DATA];
        let from = i * DATA;
        let to = (from + DATA).min(plain.len());
        data[..to - from].copy_from_slice(&plain[from..to]);
        let mut h = Sha3_256::new();
        h.update(&model[..32]);
        h.update(&wrapped);
        h.update(&data[..DATA - HASH]);
        let sum = h.finalize();
        cbc_encrypt(&mut data, &k, &v);
        Ok([&model[..32 + KEYS], &data[..], &sum[..]].concat())
    });
    let mut out = like[..HEADER].to_vec();
    for b in built {
        out.extend(b?);
    }
    out.extend(&like[like.len() - TAIL - 12..like.len() - 12]);
    out.extend((plain.len() as u64).to_le_bytes());
    out.extend([0u8; 4]);
    super::mandarin::sign(&mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps a 64-bit word the way the game does (EC-ElGamal with ephemeral `k`).
    fn wrap_word(value: u64, k: u64, steam: u64, c: &Curve) -> Vec<u8> {
        let x: BigInt = BigInt::from(value) * BigInt::from(100);
        let rhs = (x.modpow(&BigInt::from(3), &c.p) + &c.a * &x + int(CURVES_B(c))).mod_floor(&c.p);
        let y = rhs.modpow(&((&c.p + BigInt::from(1)) / BigInt::from(4)), &c.p);
        let q = mul(&private(steam), c.g.clone(), c).unwrap();
        let k = BigInt::from(k);
        let c1 = mul(&k, c.g.clone(), c).unwrap();
        let c2 = add(Some((x, y)), mul(&k, q, c), c).unwrap();
        [le32(&c1.0), le32(&c1.1), le32(&c2.0), le32(&c2.1)].concat()
    }

    #[allow(non_snake_case)]
    fn CURVES_B(c: &Curve) -> &'static str {
        CURVES.iter().find(|r| int(r[0]) == c.p).unwrap()[2]
    }

    #[test]
    fn synthetic_round_trip() {
        let steam = 76_561_198_000_000_042u64;
        let ci = 3;
        let c = curve(ci);
        let (key, iv) = ([1u8; 16], [2u8; 16]);
        let (dk, dv) = ([0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 9, 10, 11, 12, 13, 14, 15, 16], [7u8; 16]);
        let words = [&dk[..8], &dk[8..], &dv[..8], &dv[8..]].map(|w| u64::from_le_bytes(w.try_into().unwrap()));
        let mut wrapped: Vec<u8> = words.iter().enumerate().flat_map(|(i, &w)| wrap_word(w, 1000 + i as u64, steam, &c)).collect();
        assert_eq!(unwrap_keys(&wrapped, &private(steam), &c), Some((dk, dv)));
        cbc_encrypt(&mut wrapped, &key, &iv);
        let mut like = b"DSSS\x02\0\0\0\x04\0\0\0\0\0\0\0".to_vec();
        like.extend(key);
        like.extend(iv);
        like.extend(&wrapped);
        like.extend(vec![0u8; DATA + HASH + TAIL]);
        like.extend(0u64.to_le_bytes());
        like.extend([0u8; 4]);
        assert!(looks(&like));
        let plain: Vec<u8> = (0..300_000u32).map(|i| (i % 253) as u8).collect();
        let file = encrypt(&plain, &like, ci, steam).unwrap();
        assert_eq!(find_key(&file, &[5, steam]), Some((ci, steam)));
        assert_eq!(decrypt(&file, ci, steam).unwrap(), plain);
        assert!(decrypt(&file, ci, steam + 1).is_none());
        // Unchanged data gives the same file back.
        assert_eq!(encrypt(&plain, &file, ci, steam).unwrap(), file);
    }
}
