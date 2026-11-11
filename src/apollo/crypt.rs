//! The ciphers Apollo patch scripts can run (`encrypt` / `decrypt`), ported from apollo-lib's
//! `decrypt.c` (GPL-3.0-or-later, Damian Parrino). Each works in place on the part of the save
//! the script selected; like the original, a key or length it can't use leaves the data alone.

use super::keys::*;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Encrypt,
    Decrypt,
}
use Mode::*;

fn ecb<C: BlockEncrypt + BlockDecrypt>(c: &C, mode: Mode, d: &mut [u8], bs: usize) {
    for b in d.chunks_exact_mut(bs) {
        let blk = aes::cipher::generic_array::GenericArray::from_mut_slice(b);
        match mode {
            Encrypt => c.encrypt_block(blk),
            Decrypt => c.decrypt_block(blk),
        }
    }
}

fn cbc<C: BlockEncrypt + BlockDecrypt>(c: &C, mode: Mode, d: &mut [u8], iv: &[u8], bs: usize) {
    let mut prev = iv.to_vec();
    for b in d.chunks_exact_mut(bs) {
        match mode {
            Encrypt => {
                for (x, p) in b.iter_mut().zip(&prev) {
                    *x ^= p;
                }
                c.encrypt_block(aes::cipher::generic_array::GenericArray::from_mut_slice(b));
                prev.copy_from_slice(b);
            }
            Decrypt => {
                let ct = b.to_vec();
                c.decrypt_block(aes::cipher::generic_array::GenericArray::from_mut_slice(b));
                for (x, p) in b.iter_mut().zip(&prev) {
                    *x ^= p;
                }
                prev = ct;
            }
        }
    }
}

macro_rules! by_key_size {
    ($key:expr, $f:expr, $( $n:literal => $t:ty ),+) => {
        match $key.len() {
            $( $n => { if let Ok(c) = <$t>::new_from_slice($key) { $f(&c) } } )+
            _ => {}
        }
    };
}

pub fn aes_ecb(mode: Mode, d: &mut [u8], key: &[u8]) {
    by_key_size!(key, |c| ecb(c, mode, d, 16), 16 => aes::Aes128, 24 => aes::Aes192, 32 => aes::Aes256);
}

pub fn aes_cbc(mode: Mode, d: &mut [u8], key: &[u8], iv: &[u8]) {
    if iv.len() != 16 {
        return;
    }
    let n = d.len() & !15;
    let d = &mut d[..n];
    by_key_size!(key, |c| cbc(c, mode, d, iv, 16), 16 => aes::Aes128, 24 => aes::Aes192, 32 => aes::Aes256);
}

/// AES in counter mode, the whole 16-byte IV counting up big-endian (as PolarSSL does).
pub fn aes_ctr(d: &mut [u8], key: &[u8], iv: &[u8]) {
    if iv.len() != 16 {
        return;
    }
    let n = d.len() & !15;
    let d = &mut d[..n];
    let mut run = |c: &dyn Fn(&mut [u8; 16])| {
        let mut ctr: [u8; 16] = iv.try_into().unwrap();
        for b in d.chunks_exact_mut(16) {
            let mut ks = ctr;
            c(&mut ks);
            for (x, k) in b.iter_mut().zip(ks) {
                *x ^= k;
            }
            for i in (0..16).rev() {
                ctr[i] = ctr[i].wrapping_add(1);
                if ctr[i] != 0 {
                    break;
                }
            }
        }
    };
    macro_rules! go {
        ($t:ty) => {
            if let Ok(c) = <$t>::new_from_slice(key) {
                run(&|b: &mut [u8; 16]| c.encrypt_block(b.into()))
            }
        };
    }
    match key.len() {
        16 => go!(aes::Aes128),
        24 => go!(aes::Aes192),
        32 => go!(aes::Aes256),
        _ => {}
    }
}

pub fn blowfish_ecb(mode: Mode, d: &mut [u8], key: &[u8]) {
    if let Ok(c) = Bf::new_from_slice(key) {
        ecb(&c, mode, d, 8);
    }
}

pub fn blowfish_cbc(mode: Mode, d: &mut [u8], key: &[u8], iv: &[u8]) {
    if iv.len() != 8 {
        return;
    }
    let n = d.len() & !7;
    if let Ok(c) = Bf::new_from_slice(key) {
        cbc(&c, mode, &mut d[..n], iv, 8);
    }
}

/// Blowfish with the standard big-endian word order.
type Bf = blowfish::Blowfish;

pub fn camellia_ecb(mode: Mode, d: &mut [u8], key: &[u8]) {
    by_key_size!(key, |c| ecb(c, mode, d, 16), 16 => camellia::Camellia128, 24 => camellia::Camellia192, 32 => camellia::Camellia256);
}

pub fn des3_ecb(mode: Mode, d: &mut [u8], key: &[u8]) {
    if key.len() != 24 {
        return;
    }
    if let Ok(c) = des::TdesEde3::new_from_slice(key) {
        ecb(&c, mode, d, 8);
    }
}

pub fn des3_cbc(mode: Mode, d: &mut [u8], key: &[u8], iv: &[u8]) {
    if key.len() != 24 || iv.len() != 8 {
        return;
    }
    let n = d.len() & !7;
    if let Ok(c) = des::TdesEde3::new_from_slice(key) {
        cbc(&c, mode, &mut d[..n], iv, 8);
    }
}

/// Diablo III (PS3/PS4).
pub fn diablo3(mode: Mode, d: &mut [u8]) {
    let (mut k1, mut k2) = (0x305f_92d8u32, 0x2ec9_a01bu32);
    for b in d.iter_mut() {
        let tmp = match mode {
            Decrypt => {
                *b ^= k1 as u8;
                *b as u32 ^ k1
            }
            Encrypt => {
                let t = *b as u32 ^ k1;
                *b ^= k1 as u8;
                t
            }
        };
        k1 = k1 >> 8 | k2 << 24;
        k2 = k2 >> 8 | tmp << 24;
    }
}

fn md5_16(d: &[u8]) -> [u8; 16] {
    use md5::{Digest, Md5};
    Md5::digest(d).into()
}

/// Need for Speed: Undercover.
pub fn nfs_undercover(mode: Mode, d: &mut [u8]) {
    if d.len() < 16 {
        return;
    }
    let mut key = NFS_XOR_KEY;
    for (k, x) in key.iter_mut().zip(&d[..16]) {
        *k ^= x;
    }
    key = md5_16(&key);
    let blocks = d.len() / 16;
    for i in 1..blocks {
        let b = &mut d[i * 16..i * 16 + 16];
        match mode {
            Decrypt => {
                let next = md5_16(b);
                for (x, k) in b.iter_mut().zip(&key) {
                    *x ^= k;
                }
                key = next;
            }
            Encrypt => {
                for (x, k) in b.iter_mut().zip(&key) {
                    *x ^= k;
                }
                key = md5_16(b);
            }
        }
    }
}

/// Silent Hill 3 (HD Collection).
pub fn silent_hill3(mode: Mode, d: &mut [u8]) {
    const K1: u64 = 0x5b6c_3a2a;
    const K2: u64 = 0x1_0000_0000;
    let mut key2 = K2;
    for w in d.chunks_exact_mut(4) {
        let input = u32::from_be_bytes([w[0], w[1], w[2], w[3]]);
        let out = ((input as u64 ^ key2.wrapping_sub(K1)) & 0xFFFF_FFFF) as u32;
        w.copy_from_slice(&out.to_be_bytes());
        let basis = if mode == Decrypt { input } else { out };
        key2 = (basis.rotate_left(5) as u64).wrapping_add(K2);
    }
}

fn ff13_key_table(game: u32, kdata: &[u8]) -> [[u8; 8]; 32] {
    let mut ff_key: u64 = 0x1317_fb09_b9b4_2080;
    if game > 1 {
        let k0 = u64::from_le_bytes(kdata[0..8].try_into().unwrap());
        let k1 = u64::from_le_bytes(kdata[8..16].try_into().unwrap());
        ff_key = (k0 ^ k1).swap_bytes() | 1;
        ff_key ^= if game == 2 { 0x9B1F_0101_1A64_38B0 } else { 0x3654_5e6c_eb9a_705f };
    }
    ff_key = ((ff_key & 0xFF00_0000_0000_0000) >> 16) | ((ff_key & 0x0000_FF00_0000_0000) << 16) | (ff_key & 0x00FF_00FF_FFFF_FFFF);
    ff_key = ((ff_key & 0x0000_0000_FF00_FF00) >> 8) | ((ff_key & 0x0000_0000_00FF_00FF) << 8) | (ff_key & 0xFFFF_FFFF_0000_0000);
    let mut t = [[0u8; 8]; 32];
    t[0] = ff_key.to_le_bytes();
    t[0][0] = t[0][0].wrapping_add(0x45);
    for j in 1..8 {
        let i0 = t[0][j - 1] as u32 + t[0][j] as u32;
        let i1 = ((t[0][j - 1] as u32) << 2) & 0x3FC;
        t[0][j] = ((((i0 + 0xD4) ^ i1) & 0xFF) ^ 0x45) as u8;
    }
    for j in 0..31 {
        let k = u64::from_le_bytes(t[j]);
        let k = k.wrapping_add((k << 2) & 0xFFFF_FFFF_FFFF_FFFC);
        t[j + 1] = k.to_le_bytes();
    }
    t
}

const FF13_CONST: u32 = 0xA165_2347;

fn ff13_gears(byte_counter: u32, block: u32) -> (u32, u32) {
    let cog = (byte_counter as u64) << 0x14;
    let (lo, hi) = (cog as u32, (cog >> 32) as u32);
    let g1 = lo | (byte_counter << 0x0A) | byte_counter;
    let carry = (g1 > !FF13_CONST) as u32;
    (g1.wrapping_add(FF13_CONST), ((block.wrapping_mul(2)) | hi).wrapping_add(carry))
}

/// Final Fantasy XIII (game 1), XIII-2 (2) and Lightning Returns (3); the later two need the
/// 16-byte key the patch reads out of the save.
pub fn final_fantasy13(mode: Mode, game: u32, d: &mut [u8], key: &[u8]) {
    if game != 1 && key.len() != 16 {
        return;
    }
    let kt = ff13_key_table(game, key);
    let le = |s: &[u8]| u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    let size = d.len() as u32;
    let mut bc: u32 = 0;
    let mut block: u32 = 0;
    let mut kc = 0usize;
    while bc < size {
        if kc > 31 {
            kc = 0;
        }
        let (g1, g2) = ff13_gears(bc, block);
        let at = bc as usize;
        if at + 8 > d.len() {
            break;
        }
        let ka = le(&kt[kc][0..4]);
        let kb = le(&kt[kc][4..8]);
        match mode {
            Decrypt => {
                let mut old = 0u8;
                for k in 0..8 {
                    let p = at + k;
                    if k == 0 {
                        old = d[p];
                        d[p] ^= 0x45 ^ block as u8;
                    } else {
                        let cur = d[p];
                        d[p] ^= old;
                        old = cur;
                    }
                    for i in 1..9 {
                        d[p] = 0x78u8.wrapping_add(d[p]).wrapping_sub(kt[kc][i - 1]);
                    }
                }
                let (ta, tb) = (le(&d[at..]), le(&d[at + 4..]));
                let carry = (ta < ka) as u32;
                let na = ka ^ g1 ^ ta.wrapping_sub(ka);
                let nb = kb ^ g2 ^ tb.wrapping_sub(kb).wrapping_sub(carry);
                d[at..at + 4].copy_from_slice(&nb.to_le_bytes());
                d[at + 4..at + 8].copy_from_slice(&na.to_le_bytes());
            }
            Encrypt => {
                let mut tb = le(&d[at..]);
                let mut ta = le(&d[at + 4..]);
                tb ^= kb ^ g2;
                ta ^= ka ^ g1;
                let carry = (ta > !ka) as u32;
                tb = tb.wrapping_add(kb.wrapping_add(carry));
                ta = ta.wrapping_add(ka);
                d[at..at + 4].copy_from_slice(&ta.to_le_bytes());
                d[at + 4..at + 8].copy_from_slice(&tb.to_le_bytes());
                let mut old = 0u8;
                for k in 0..8 {
                    let p = at + k;
                    for i in (1..9).rev() {
                        d[p] = d[p].wrapping_add(kt[kc][i - 1]).wrapping_sub(0x78);
                    }
                    if k == 0 {
                        d[p] ^= 0x45 ^ block as u8;
                    } else {
                        d[p] ^= old;
                    }
                    old = d[p];
                }
            }
        }
        bc += 8;
        block = block.wrapping_add(1);
        kc += 1;
    }
}

/// Metal Gear Solid HD Collection: an additive chain under a text key.
pub fn mgs(mode: Mode, d: &mut [u8], key: &[u8]) {
    if d.is_empty() || key.is_empty() {
        return;
    }
    let k = |i: usize| key[i % key.len()];
    match mode {
        Decrypt => {
            for i in (1..d.len()).rev() {
                d[i] = d[i].wrapping_sub(k(i).wrapping_add(d[i - 1]));
            }
            d[0] = d[0].wrapping_sub(key[0]);
        }
        Encrypt => {
            d[0] = d[0].wrapping_add(key[0]);
            for i in 1..d.len() {
                d[i] = d[i].wrapping_add(k(i).wrapping_add(d[i - 1]));
            }
        }
    }
}

/// Metal Gear Solid 2/3 HD's 32-byte base64-like save codes.
pub fn mgs_base64(mode: Mode, d: &mut [u8]) {
    if d.len() != 32 {
        return;
    }
    match mode {
        Encrypt => {
            let ty = d[31];
            let chars = if ty == 2 { MGS2_ALPHABET } else { MGS3_ALPHABET };
            d[31] = 0;
            d[20] = 0;
            for j in 0..20 {
                d[20] ^= d[j];
            }
            let mut tmp = [0u8; 28];
            let (mut i, mut k) = (0usize, 0i32);
            for t in tmp.iter_mut() {
                *t = if k == 0 || k == 1 {
                    d[i] >> (2 - k)
                } else if k == 2 {
                    d[i] & 63
                } else {
                    (d[i + 1] >> (10 - k)) | ((d[i] & ((1u16 << (8 - k)) - 1) as u8) << (k - 2))
                };
                k += 6;
                if k >= 8 {
                    k -= 8;
                    i += 1;
                }
            }
            d[0] = if ty == 2 { 68 } else { 0x5F };
            let mut j = 0usize;
            for (i, &t) in tmp.iter().enumerate() {
                d[i + 1] = chars[(t as usize + j + 7 * (i % 4)) & 63];
                if (i + 1) % 4 == 0 {
                    j += 28;
                }
            }
        }
        Decrypt => {
            let ty = (d[0] == 0x5F) as u8 + 2;
            let chars = if ty == 2 { MGS2_ALPHABET } else { MGS3_ALPHABET };
            let mut table = [0xFFu8; 256];
            for (k, &c) in chars.iter().enumerate() {
                table[c as usize] = k as u8;
            }
            let mut j = 0i32;
            for m in 0..28 {
                let k = table[d[1 + m] as usize];
                if k == 0xFF {
                    return;
                }
                d[m] = ((k as i32 - j) & 63) as u8;
                j += 7;
            }
            let mut tmp = [0u8; 32];
            let (mut j, mut k) = (0i32, 0usize);
            for t in tmp.iter_mut().take(21) {
                if j <= 5 {
                    *t = ((d[k] as i32 & (63 >> (j & 31))) << ((j + 2) & 31)) as u8;
                }
                let mut i = !j + 7;
                k += 1;
                if i < 2 {
                    *t |= ((d[k] as i32) << ((!i + 3) & 31)) as u8;
                    i += 6;
                    k += 1;
                }
                i -= 2;
                j = 0;
                if i == 0 {
                    *t |= d[k];
                    k += 1;
                } else if i <= 5 {
                    j = 6 - i;
                    *t |= (d[k] as i32 >> (i & 31)) as u8;
                }
            }
            d.copy_from_slice(&tmp);
            d[31] = ty;
        }
    }
}

// --- Metal Gear Solid: Peace Walker ---

const PW_R1_STD: usize = 0x1AF24;
const PW_R1_JP: usize = 0x1AF14;
const PW_R2: usize = 0x1C00;
const PW_R3: usize = 0x18E68;
const PW_B1_OFF: usize = 0x40;
const PW_B1_TAIL: usize = 8;
const PW_H2_OFF: usize = 0xD676;
const PW_B2_OFF: usize = 0xD686;
const PW_B2_SIZE: usize = 0xF0D0;

fn be32(d: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

fn pw_swap(d: &mut [u8], words: usize) {
    for w in d.chunks_exact_mut(4).take(words) {
        w.reverse();
    }
}

fn pw_crypt(d: &mut [u8], words: usize, salts: &mut [u32; 2]) {
    for w in d.chunks_exact_mut(4).take(words) {
        let v = u32::from_be_bytes([w[0], w[1], w[2], w[3]]) ^ salts[0];
        w.copy_from_slice(&v.to_be_bytes());
        salts[0] = salts[0].wrapping_mul(0x2e9_0edd).wrapping_add(salts[1]);
    }
}

fn pw_salts(d: &[u8]) -> Option<[u32; 2]> {
    if d.len() < 8 {
        return None;
    }
    let off = ((be32(d, 4) | 0xAD47_DE8F) ^ be32(d, 0)) as u64;
    if (off + 8) * 4 > d.len() as u64 {
        return None;
    }
    let off = off as usize;
    let mut s0 = (be32(d, (off + 2) * 4) ^ 0x1327_de73) ^ (be32(d, (off + 3) * 4) ^ 0x2d71_d26c);
    let s1 = s0.wrapping_mul(be32(d, (off + 7) * 4) ^ 0xBC4D_EFA2);
    s0 = (s0 ^ 0x6576) << 16 | s0;
    Some([s0, s1])
}

/// Peace Walker saves: type 0 is the PS3 HD Edition, 1 PSP (US/EU), 2 PSP Japanese download.
pub fn mgs_pw(mode: Mode, d: &mut [u8], ty: i32) {
    let r1 = if ty == 2 { PW_R1_JP } else { PW_R1_STD };
    let b1 = 0x44 + r1 + PW_R2 + PW_R3 + PW_B1_TAIL - PW_B1_OFF;
    let ps3 = ty == 0;
    let b1_words = b1 / 4;
    let swap_words = PW_B1_OFF / 4 + b1_words;
    let min = if ps3 { PW_B2_OFF * 4 + PW_B2_SIZE } else { PW_B1_OFF + b1 };
    if d.len() < min {
        return;
    }
    match mode {
        Decrypt => {
            pw_swap(d, swap_words);
            let Some(mut s) = pw_salts(d) else { return };
            pw_crypt(&mut d[PW_B1_OFF..], b1_words, &mut s);
            if ps3 {
                let Some(mut s) = pw_salts(&d[PW_H2_OFF * 4..]) else { return };
                pw_crypt(&mut d[PW_B2_OFF * 4..], PW_B2_SIZE / 4, &mut s);
            }
            pw_swap(&mut d[0x44..], swap_words - 0x11);
        }
        Encrypt => {
            pw_swap(&mut d[0x44..], swap_words - 0x11);
            if ps3 {
                let Some(mut s) = pw_salts(&d[PW_H2_OFF * 4..]) else { return };
                pw_crypt(&mut d[PW_B2_OFF * 4..], PW_B2_SIZE / 4, &mut s);
            }
            let Some(mut s) = pw_salts(d) else { return };
            pw_crypt(&mut d[PW_B1_OFF..], b1_words, &mut s);
            pw_swap(d, swap_words);
        }
    }
}

/// Dynasty Warriors 8: Xtreme Legends (the same stream both ways).
pub fn dw8xl(d: &mut [u8]) {
    let mut k = 0x1310_0200u32;
    for b in d.iter_mut() {
        k = k.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        *b ^= (k >> 16) as u8;
    }
}

/// Borderlands 3 (PS4): `is_data` picks the save keys over the profile keys.
pub fn borderlands3(mode: Mode, d: &mut [u8], is_data: bool) {
    let (xor, pre) = if is_data { (&BL3_DATA_XOR_PS4, &BL3_DATA_PREFIX_PS4) } else { (&BL3_PROFILE_XOR_PS4, &BL3_PROFILE_PREFIX_PS4) };
    let step = |d: &mut [u8], i: usize| {
        let b = if i < 32 { pre[i] } else { d[i - 32] } ^ xor[i % 32];
        d[i] ^= b;
    };
    match mode {
        Decrypt => (0..d.len()).rev().for_each(|i| step(d, i)),
        Encrypt => (0..d.len()).for_each(|i| step(d, i)),
    }
}

/// Metal Gear Solid V: The Phantom Pain (the same stream both ways).
pub fn mgs5_tpp(d: &mut [u8], mut key: u32, big_endian: bool) {
    for w in d.chunks_exact_mut(4) {
        key ^= key << 13;
        key ^= key >> 7;
        key ^= key << 5;
        let a = [w[0], w[1], w[2], w[3]];
        let v = if big_endian { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) } ^ key;
        w.copy_from_slice(&if big_endian { v.to_be_bytes() } else { v.to_le_bytes() });
    }
}

fn mh_xor(d: &mut [u8], seed: u32) {
    let mut k = [seed >> 16, seed & 0xffff];
    if k[0] == 0 {
        k[0] = 0xdfa3;
    }
    if k[1] == 0 {
        k[1] = 0x215f;
    }
    for w in d.chunks_exact_mut(4) {
        k[0] = k[0].wrapping_mul(0xdfa3) % 0xffef;
        k[1] = k[1].wrapping_mul(0x215f) % 0xff8f;
        let key = (k[0] << 16).wrapping_add(k[1]);
        let v = u32::from_le_bytes([w[0], w[1], w[2], w[3]]) ^ key;
        w.copy_from_slice(&v.to_le_bytes());
    }
}

/// Monster Hunter Freedom Unite / Portable 3rd (PSP): `ver` 3 is Portable 3rd.
pub fn monster_hunter(mode: Mode, d: &mut [u8], ver: i32) {
    let n = d.len();
    if n < 4 {
        return;
    }
    let dec = if ver == 3 { &MH3_DEC_TABLE } else { &MH2_DEC_TABLE };
    let tr = |d: &mut [u8], t: &[u8; 256]| d.iter_mut().for_each(|b| *b = t[*b as usize]);
    match mode {
        Decrypt => {
            tr(&mut d[n - 4..], dec);
            tr(d, dec);
            let seed = u32::from_le_bytes(d[n - 4..].try_into().unwrap());
            mh_xor(&mut d[..n - 4], seed);
            tr(&mut d[..n - 4], dec);
        }
        Encrypt => {
            let mut enc = [0u8; 256];
            for (i, &v) in dec.iter().enumerate() {
                enc[v as usize] = i as u8;
            }
            let seed = u32::from_le_bytes(d[n - 4..].try_into().unwrap());
            tr(&mut d[..n - 4], &enc);
            mh_xor(&mut d[..n - 4], seed);
            tr(d, &enc);
            tr(&mut d[n - 4..], &enc);
        }
    }
}

/// Yakuza / RGG Studio games: XOR with a repeating key.
pub fn rgg_studio(d: &mut [u8], key: &[u8]) {
    if key.is_empty() {
        return;
    }
    for (i, b) in d.iter_mut().enumerate() {
        *b ^= key[i % key.len()];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(f: impl Fn(Mode, &mut Vec<u8>)) {
        let orig: Vec<u8> = (0..96u32).map(|i| (i * 37 + 11) as u8).collect();
        let mut d = orig.clone();
        f(Encrypt, &mut d);
        assert_ne!(d, orig);
        f(Decrypt, &mut d);
        assert_eq!(d, orig);
    }

    #[test]
    fn round_trips() {
        let k16 = [7u8; 16];
        let k24: Vec<u8> = (0..24).collect();
        let iv16 = [3u8; 16];
        round(|m, d| aes_ecb(m, d, &k16));
        round(|m, d| aes_cbc(m, d, &k24[..16], &iv16));
        round(|m, d| blowfish_ecb(m, d, b"secret key"));
        round(|m, d| blowfish_cbc(m, d, b"secret key", &[1; 8]));
        round(|m, d| camellia_ecb(m, d, &k16));
        round(|m, d| des3_ecb(m, d, &k24));
        round(|m, d| des3_cbc(m, d, &k24, &[9; 8]));
        round(|m, d| diablo3(m, d));
        round(|m, d| nfs_undercover(m, d));
        round(|m, d| silent_hill3(m, d));
        round(|m, d| final_fantasy13(m, 1, d, &[]));
        round(|m, d| final_fantasy13(m, 2, d, &k16));
        round(|m, d| mgs(m, d, b"2MetalOfSolidSonsLibertyGear"));
        round(|m, d| borderlands3(m, d, true));
        round(|m, d| monster_hunter(m, d, 3));
        let mut d = vec![5u8; 64];
        aes_ctr(&mut d, &k16, &iv16);
        aes_ctr(&mut d, &k16, &iv16);
        assert_eq!(d, vec![5u8; 64]);
        // Blowfish's published test vector (all-zero key and block).
        let mut z = [0u8; 8];
        blowfish_ecb(Encrypt, &mut z, &[0; 8]);
        assert_eq!(z, [0x4E, 0xF9, 0x97, 0x45, 0x61, 0x98, 0xDD, 0x78]);
    }
}
