//! Capcom RE Engine "Mandarin" saves (`DSSS` files in `remote\win64_save`): Onimusha: Way of
//! the Sword, Dragon's Dogma 2, Monster Hunter Wilds and Stories 3, Pragmata, Resident Evil
//! Requiem and others.
//!
//! Ported from mi5hmash's MandarinJuice (MIT, github.com/mi5hmash/MandarinJuice), whose game
//! profiles supply each game's seed and how it turns the Steam account into the key.
//!
//! A file is a 16-byte header ("DSSS", 2, flavor, 0), the encrypted data, and a 0x8C-byte
//! footer (0x80 random bytes, the plain length, a MurmurHash3 of everything before it). The
//! data is cut into slices of 16 KiB × (1…8), each after a 528-byte slice header. A
//! SplitMix64 stream seeded from the game's seed and the user id masks the slice headers and
//! gives each slice a 32-byte AES-128 key and starting block (OFB mode); the header carries
//! that key wrapped with modular exponentiation over the three fixed 256-bit numbers below,
//! so it unwraps only with the right user id. The first slice's header also holds a
//! checksum of its plain data, which tells a right id from a wrong one.
//!
//! The "compressible" flavor (Monster Hunter Wilds) deflates the data first, behind a
//! 24-byte block header.

use aes::cipher::{BlockEncrypt, KeyInit};
use num_bigint::BigUint;
use std::io::{Read, Write};
use std::sync::OnceLock;

/// What a game's profile says: its Steam app id (or Epic namespace), seed, how the user id
/// is formed, and whether the data is compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Game {
    pub title: &'static str,
    pub app: &'static str,
    pub epic: bool,
    pub seed: u64,
    pub variant: u8,
    pub compressed: bool,
}

const fn g(title: &'static str, app: &'static str, epic: bool, seed: u64, variant: u8, flavor: u32) -> Game {
    Game { title, app, epic, seed, variant, compressed: flavor == 0x18 }
}

/// From MandarinJuice v1.4.1's `_profiles`.
pub static GAMES: &[Game] = &[
    g("Onimusha: Way of the Sword", "2638890", false, 11688424886205786022, 1, 16),
    g("Onimusha: Way of the Sword", "onimusha-way-of-the-sword-3c2acc", true, 11688424886205786022, 2, 16),
    g("Dead Rising Deluxe Remaster", "2527390", false, 3460376763846700646, 3, 16),
    g("Dragon's Dogma 2", "2054970", false, 6829223510001954331, 1, 16),
    g("Kunitsu-Gami: Path of the Goddess", "2510710", false, 5816692738637946535, 1, 16),
    g("Mega Man Star Force Legacy Collection", "3500390", false, 10988124340104319346, 1, 16),
    g("Monster Hunter Stories 3: Twisted Reflection", "2852190", false, 11817790605178805979, 2, 16),
    g("Monster Hunter Wilds", "2246340", false, 8806390302218022125, 2, 24),
    g("Pragmata", "3357650", false, 9052880243776943063, 2, 16),
    g("Resident Evil Requiem", "3764200", false, 7058977378613153274, 2, 16),
    g("Resident Evil Requiem", "resident-evil-requiem-4ead6d", true, 7058977378613153274, 1, 16),
    g("Resident Evil Requiem Demo", "4459100", false, 16977276748297568783, 2, 16),
    g("Resident Evil Requiem Demo", "resident-evil-requiem-resident-evil-requiem-demo-68cecd", true, 16977276748297568783, 1, 16),
];

const MAGIC: &[u8; 4] = b"DSSS";
const HEADER: usize = 0x10;
const FOOTER: usize = 0x8C;
const SLICE_HEADER: usize = 2 * 4 * 8 * 8 + 0x10;
const SLICE_SHIFT: u32 = 14;
const KEY_TYPE: u64 = 0x14;

const P1: &str = "8ztvuXKgtyUV5Fw5GCnhgq2Km9wKZNNETXnIEKuGNxc=";
const P2: &str = "+Z23XDnQ25IKcq4cjJRwwVbFTW4FsmmipjxkiFXDmws=";
const P3: &str = "5m9USvzOaMXvB7mgeyd1hTRKHbYTdugx9zufvV9E9xU=";

struct Keys {
    p1: BigUint,
    p2: BigUint,
    p3: BigUint,
    /// P3 ^ 0x14 mod P1: the first half of every wrapped key.
    header_key: BigUint,
}

fn keys() -> &'static Keys {
    static K: OnceLock<Keys> = OnceLock::new();
    K.get_or_init(|| {
        use base64::Engine;
        let n = |s: &str| BigUint::from_bytes_le(&base64::engine::general_purpose::STANDARD.decode(s).unwrap());
        let (p1, p2, p3) = (n(P1), n(P2), n(P3));
        let header_key = create_key(&p1, &p2, &p3, KEY_TYPE);
        Keys { p1, p2, p3, header_key }
    })
}

fn create_key(p1: &BigUint, p2: &BigUint, p3: &BigUint, seed: u64) -> BigUint {
    p3.modpow(&(BigUint::from(seed) % p2), p1)
}

fn splitmix(state: &mut u64) {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    *state = (*state ^ (*state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    *state = (*state ^ (*state >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    *state ^= *state >> 31;
}

/// Slice capacities (in 16 KiB units) for `len` bytes, advancing `state`.
fn slices(state: &mut u64, mut len: u32) -> Vec<usize> {
    let mut out = vec![];
    for _ in 0..(len >> SLICE_SHIFT) + 1 {
        if len > 0 {
            let cap = (*state as u8 & 7) + 1;
            out.push(cap as usize);
            len = len.saturating_sub((cap as u32) << SLICE_SHIFT);
        }
        splitmix(state);
    }
    out
}

fn mask_bytes(state: &mut u64, data: &mut [u8]) {
    for b in data {
        splitmix(state);
        *b ^= *state as u8;
    }
}

/// The 32 bytes of key material for one slice (AES key, then the starting block).
fn slice_key(state: &mut u64) -> [u8; 32] {
    let mut k = [0u8; 32];
    for i in 0..16 {
        splitmix(state);
        k[i] = *state as u8;
        k[16 + i] = (*state >> 8) as u8;
    }
    k
}

/// AES-128 in OFB mode (the same both ways).
fn ofb(data: &mut [u8], key: &[u8; 32]) {
    let aes = aes::Aes128::new(key[..16].into());
    let mut block = aes::Block::clone_from_slice(&key[16..]);
    for chunk in data.chunks_mut(16) {
        aes.encrypt_block(&mut block);
        for (d, k) in chunk.iter_mut().zip(block.iter()) {
            *d ^= k;
        }
    }
}

fn limbs(n: &BigUint, count: usize) -> Vec<u8> {
    let mut b = n.to_bytes_le();
    b.resize(count * 8, 0);
    b.truncate(count * 8);
    b
}

/// Unwraps the slice key from a (unmasked) slice header.
fn unwrap_key(header: &[u8], user: u64) -> [u8; 32] {
    let k = keys();
    let mut out = [0u8; 32];
    for i in 0..4 {
        let at = i * 128;
        let a = BigUint::from_bytes_le(&header[at..at + 64]).modpow(&BigUint::from(user), &k.p1);
        let d = BigUint::from_bytes_le(&header[at + 64..at + 128]);
        let q = if a == BigUint::ZERO { BigUint::ZERO } else { d / a };
        out[i * 8..i * 8 + 8].copy_from_slice(&limbs(&q, 1));
    }
    out
}

/// Wraps a slice key into a slice header, for `personal` = create_key(user id).
fn wrap_key(header: &mut [u8], key: &[u8; 32], personal: &BigUint) {
    let k = keys();
    let a = personal.modpow(&BigUint::from(KEY_TYPE), &k.p1);
    for i in 0..4 {
        let at = i * 128;
        header[at..at + 64].copy_from_slice(&limbs(&k.header_key, 8));
        let part = u64::from_le_bytes(key[i * 8..i * 8 + 8].try_into().unwrap());
        header[at + 64..at + 128].copy_from_slice(&limbs(&(&a * BigUint::from(part)), 8));
    }
}

fn u64s(d: &[u8]) -> Vec<u64> {
    d.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
}

/// The checksum over a slice's plain data (a CityHash64-style hash).
pub fn checksum(d: &[u8]) -> u64 {
    const SHIFT: u32 = 0x2F;
    const S0: u64 = 0xB492_B66F_BE98_F273;
    const S1: u64 = 0x9AE1_6A3B_2F90_404F;
    let len = d.len();
    let w = u64s(d);
    let tail = |n: usize| u64s(&d[len - n..]);
    let (rl, rr) = (u64::rotate_left, u64::rotate_right);
    match len {
        65.. => {
            let m: u64 = 0x9DDF_EA08_EB38_2D69;
            let lb = tail(64);
            let l = |k: usize| lb[8 - k];
            let x0 = l(6).wrapping_add(len as u64) ^ l(3);
            let x = (x0.wrapping_mul(m) >> SHIFT) ^ x0.wrapping_mul(m) ^ l(3);
            let mut ca = (((x.wrapping_mul(m)) >> SHIFT) ^ x.wrapping_mul(m)).wrapping_mul(m);
            let mut sum0 = (len as u64).wrapping_add(l(8));
            let mut sum = sum0.wrapping_add(l(7)).wrapping_add(l(6));
            let mut left = rl(sum, 0x14);
            sum = sum0.wrapping_add(l(5)).wrapping_add(ca);
            let mut right = rr(sum, 0x15);
            let mut cb = sum0.wrapping_add(left).wrapping_add(right);
            sum0 = S0.wrapping_add(l(7)).wrapping_add(l(4)).wrapping_add(l(2));
            sum = sum0.wrapping_add(l(3)).wrapping_add(l(2));
            left = rl(sum, 0x14);
            sum = sum0.wrapping_add(l(5)).wrapping_add(l(1));
            right = rr(sum, 0x15);
            let mut cc = sum0.wrapping_add(left).wrapping_add(right);
            let mut cd = sum0.wrapping_add(l(3)).wrapping_add(l(2)).wrapping_add(l(1));
            let mut ce = (len as u64).wrapping_add(l(8)).wrapping_add(l(7)).wrapping_add(l(6)).wrapping_add(l(5));
            let mut cf = S0.wrapping_mul(l(5)).wrapping_add(w[0]);
            let mut cg = l(7).wrapping_add(l(2));
            let mut ch = 0u64;
            for i in 0..(len - 1) / 64 {
                let p = i * 8;
                sum = ce.wrapping_add(cf).wrapping_add(cg).wrapping_add(w[p + 1]);
                ch = S0.wrapping_mul(rl(sum, 0x1B)) ^ cc;
                sum = cb.wrapping_add(cg).wrapping_add(w[p + 6]);
                cg = S0.wrapping_mul(rl(sum, 0x16)).wrapping_add(w[p + 5]).wrapping_add(ce);
                sum = ca.wrapping_add(cd);
                cf = S0.wrapping_mul(rl(sum, 0x1F));
                let mul = S0.wrapping_mul(cb);
                sum = w[p + 2].wrapping_add(w[p + 1]).wrapping_add(w[p]);
                ce = mul.wrapping_add(w[p + 3]).wrapping_add(sum);
                left = rl(mul.wrapping_add(sum), 0x14);
                right = rr(mul.wrapping_add(w[p + 3]).wrapping_add(w[p]).wrapping_add(cd).wrapping_add(ch), 0x15);
                cb = mul.wrapping_add(left).wrapping_add(right).wrapping_add(w[p]);
                cc = cc.wrapping_add(w[p + 4]);
                sum0 = cc.wrapping_add(cf);
                cd = w[p + 7].wrapping_add(w[p + 6]).wrapping_add(w[p + 5]).wrapping_add(sum0);
                sum = w[p + 6].wrapping_add(w[p + 5]).wrapping_add(sum0);
                left = rl(sum, 0x14);
                sum = w[p + 7].wrapping_add(w[p + 2]).wrapping_add(sum0).wrapping_add(cg);
                right = rr(sum, 0x15);
                cc = left.wrapping_add(right).wrapping_add(sum0);
                ca = ch;
            }
            ca = (cb ^ cc).wrapping_mul(m);
            cb = ((ca >> SHIFT) ^ ca ^ cc).wrapping_mul(m);
            ca = (cd ^ ce).wrapping_mul(m);
            cc = ((ca >> SHIFT) ^ ca ^ cd).wrapping_mul(m);
            ca = ((cg >> SHIFT) ^ cg).wrapping_mul(S0).wrapping_add(ch);
            cd = ((cb >> SHIFT) ^ cb).wrapping_mul(m).wrapping_add(cf);
            cb = ((((cc >> SHIFT) ^ cc).wrapping_mul(m).wrapping_add(ca)) ^ cd).wrapping_mul(m);
            ca = ((cb >> SHIFT) ^ cb ^ cd).wrapping_mul(m);
            ((ca >> SHIFT) ^ ca).wrapping_mul(m)
        }
        33..=64 => {
            let m = (len as u64).wrapping_mul(2).wrapping_add(S1);
            let lb = tail(32);
            let l = |k: usize| lb[4 - k];
            let ca = w[3].wrapping_mul(9).wrapping_add(S1.wrapping_mul(w[2]));
            let cb = rl(ca, 0x16).wrapping_add(l(3));
            let cc = ca.wrapping_add(l(3));
            let cd = S1.wrapping_mul(w[0]).wrapping_add(l(1));
            let ce = w[3].wrapping_mul(9).wrapping_add(cd ^ l(4)).wrapping_add(1);
            let mut bs = m
                .wrapping_mul(rl(cd, 0x15).wrapping_add(rr(w[1], 0x1E).wrapping_add(l(3)).wrapping_mul(9)).wrapping_add(ce))
                .swap_bytes();
            bs = m.wrapping_mul(bs.wrapping_add(ce).wrapping_add(m.wrapping_mul(l(2)))).swap_bytes();
            bs = m.wrapping_mul(bs.wrapping_add(cb).wrapping_add(cc).wrapping_add(l(1))).swap_bytes();
            bs = m.wrapping_mul(bs.wrapping_add(cc).wrapping_add(w[1])).wrapping_add(m.wrapping_mul(l(2))).wrapping_add(l(4));
            m.wrapping_mul((bs >> SHIFT) ^ bs).wrapping_add(cb)
        }
        17..=32 => {
            let m = (len as u64).wrapping_mul(2).wrapping_add(S1);
            let lb = tail(16);
            let ca = rr(w[1].wrapping_add(S1), 0x12).wrapping_add(m.wrapping_mul(lb[1])).wrapping_add(w[0].wrapping_mul(S0));
            let cb = m.wrapping_mul(
                (rl(w[1].wrapping_add(w[0].wrapping_mul(S0)), 0x15).wrapping_add(rr(m.wrapping_mul(lb[1]), 0x1E)).wrapping_add(S1.wrapping_mul(lb[0]))) ^ ca,
            );
            let cc = m.wrapping_mul((cb >> SHIFT) ^ ca ^ cb);
            ((cc >> SHIFT) ^ cc).wrapping_mul(m)
        }
        8..=16 => {
            let m = (len as u64).wrapping_mul(2).wrapping_add(S1);
            let last = tail(8)[0];
            let ca = w[0].wrapping_sub(0x651E_95C4_D06F_BFB1);
            let cb = m.wrapping_mul(rr(ca, 0x19).wrapping_add(last));
            let cc = m.wrapping_mul(m.wrapping_mul(rl(last, 0x1B)).wrapping_add(ca) ^ cb);
            let cd = m.wrapping_mul((cc >> SHIFT) ^ cc ^ cb);
            ((cd >> SHIFT) ^ cd).wrapping_mul(m)
        }
        4..=7 => {
            let m = (len as u64).wrapping_mul(2).wrapping_add(S1);
            let first = u32::from_le_bytes(d[..4].try_into().unwrap()) as u64;
            let last = u32::from_le_bytes(d[len - 4..].try_into().unwrap()) as u64;
            let ca = m.wrapping_mul((first.wrapping_mul(8).wrapping_add(len as u64)) ^ last);
            let cb = m.wrapping_mul((ca >> SHIFT) ^ ca ^ last);
            m.wrapping_mul((cb >> SHIFT) ^ cb)
        }
        1..=3 => {
            let (m0, m): (u64, u64) = (0xC3A5_C85C_97CB_3127, 0xE16A_3B2F_9040_4F00);
            let ca = (m.wrapping_mul(d[len >> 1] as u64).wrapping_add(S1.wrapping_mul(d[0] as u64)))
                ^ m0.wrapping_mul((len as u64).wrapping_add((d[len - 1] as u64).wrapping_mul(4)));
            ((ca >> SHIFT) ^ ca).wrapping_mul(S1)
        }
        0 => S1,
    }
}

/// MurmurHash3 (32-bit, whole words only) as the files use it.
fn murmur3(words: &[u32], mut seed: u32) -> u32 {
    const H0: u32 = 0x1B87_3593;
    const H1: u32 = 0xCC9E_2D51;
    for &e in words {
        seed = 5u32.wrapping_mul((H0.wrapping_mul(H1.wrapping_mul(e).rotate_left(15)) ^ seed).rotate_left(13).wrapping_sub(0x0522_50EC));
    }
    let basis = (words.len() as u32 * 4) ^ seed;
    let x = 0x85EB_CA6Bu32.wrapping_mul(basis ^ (basis >> 16));
    let y = 0xC2B2_AE35u32.wrapping_mul(x ^ (x >> 13));
    y ^ (y >> 16)
}

fn sign(file: &mut [u8]) {
    let n = file.len() / 4;
    let words: Vec<u32> = file[..(n - 1) * 4].chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    let s = murmur3(&words, 0xFFFF_FFFF);
    file[(n - 1) * 4..n * 4].copy_from_slice(&s.to_le_bytes());
}

/// The user id a game keys its saves with, from the Steam account number (the folder under
/// `Steam\userdata`) or, for Epic, the 64-bit id.
pub fn user_id(game: &Game, account: u64) -> u64 {
    let steam64 = if game.epic || account > u32::MAX as u64 { account } else { 0x0110_0001_0000_0000 | account };
    let acct = account as u32;
    match game.variant {
        1 => !(acct as u64) | 0xFFFF_FFFF_0000_0000,
        2 => !steam64,
        3 => {
            let s = steam64 ^ 0x1A3B_5C7D_D0C2_B4A8;
            let b = |sh: u32| (s >> sh) & 0xFF;
            !(b(32) | b(40) << 8 | b(48) << 16 | b(56) << 24 | b(0) << 32 | b(8) << 40 | b(16) << 48 | b(24) << 56)
        }
        _ => steam64,
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > HEADER + FOOTER && d.starts_with(MAGIC) && d[4..8] == [2, 0, 0, 0] && matches!(d[8], 0x10 | 0x18)
}

fn plain_len(d: &[u8]) -> usize {
    u64::from_le_bytes(d[d.len() - 12..d.len() - 4].try_into().unwrap()) as usize
}

/// Cheap check of a user id against a file, without decrypting it: the first 8 bytes of
/// the masked slice header are the fixed first half of the wrapped key.
pub fn id_matches(d: &[u8], game: &Game, user: u64) -> bool {
    if !looks(d) || (d[8] == 0x18) != game.compressed {
        return false;
    }
    let mut state = game.seed;
    slices(&mut state, plain_len(d) as u32);
    state = state.wrapping_add(user);
    slice_key(&mut state);
    let mut first = d[HEADER..HEADER + 8].to_vec();
    mask_bytes(&mut state, &mut first);
    first == limbs(&keys().header_key, 1)
}

/// The data inside, for `user`; `None` unless the first slice's checksum matches.
pub fn decrypt(d: &[u8], game: &Game, user: u64) -> Option<Vec<u8>> {
    if !looks(d) {
        return None;
    }
    let len = plain_len(d);
    let data = &d[HEADER..d.len() - FOOTER];
    let mut state = game.seed;
    let caps = slices(&mut state, len as u32);
    state = state.wrapping_add(user);
    // The key stream runs through the slices in order (cheap); unwrapping each slice's key
    // and decrypting it (the slow part) then happen on all cores.
    let mut slices = vec![];
    let mut at = 0;
    let mut left = len;
    for cap in &caps {
        let slice_len = cap << SLICE_SHIFT;
        let slice = data.get(at..at + SLICE_HEADER + slice_len)?;
        slice_key(&mut state);
        let mut header = slice[..SLICE_HEADER].to_vec();
        mask_bytes(&mut state, &mut header);
        let n = slice_len.min(left);
        left -= n;
        slices.push((header, &slice[SLICE_HEADER..], n));
        at += SLICE_HEADER + slice_len;
    }
    let plain = crate::par::map(&slices, |(header, enc, n)| {
        let key = unwrap_key(header, user);
        let mut body = enc.to_vec();
        ofb(&mut body, &key);
        body.truncate(*n);
        body
    });
    if checksum(&plain[0]) != u64::from_le_bytes(slices[0].0[512..520].try_into().unwrap()) {
        return None;
    }
    let out: Vec<u8> = plain.concat();
    if game.compressed {
        let mut plain = vec![];
        flate2::read::DeflateDecoder::new(out.get(24..)?).read_to_end(&mut plain).ok()?;
        return Some(plain);
    }
    Some(out)
}

/// A file holding `plain`, for `user`. `like` (the file it was read from) gives the header
/// and the footer's random bytes, so an unchanged save comes back byte for byte.
pub fn encrypt(plain: &[u8], like: &[u8], game: &Game, user: u64) -> Result<Vec<u8>, String> {
    let packed;
    let data = if game.compressed {
        let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        e.write_all(plain).map_err(|e| e.to_string())?;
        let c = e.finish().map_err(|e| e.to_string())?;
        let mut b = Vec::with_capacity(24 + c.len());
        b.extend((c.len() as u64 + 0x10).to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend((c.len() as u32).to_le_bytes());
        b.extend((plain.len() as u64).to_le_bytes());
        b.extend(c);
        packed = b;
        &packed[..]
    } else {
        plain
    };
    let mut state = game.seed;
    let caps = slices(&mut state, data.len() as u32);
    state = state.wrapping_add(user);
    let k = keys();
    let personal = create_key(&k.p1, &k.p2, &k.p3, user);
    // Per slice, in order: its key and the mask for its header (the key stream); then the
    // headers and bodies are built on all cores.
    let mut plan = vec![];
    let mut done = 0;
    for cap in caps {
        let slice_len = cap << SLICE_SHIFT;
        let remaining = data.len() - done;
        let key = slice_key(&mut state);
        let mut mask = vec![0u8; SLICE_HEADER];
        mask_bytes(&mut state, &mut mask);
        let n = slice_len.min(remaining);
        plan.push((key, mask, done, n, slice_len, remaining));
        done += n;
    }
    let slices = crate::par::map(&plan, |(key, mask, from, n, slice_len, remaining)| {
        let mut header = vec![0u8; SLICE_HEADER];
        wrap_key(&mut header, key, &personal);
        let mut body = vec![0u8; *slice_len];
        body[..*n].copy_from_slice(&data[*from..from + n]);
        header[512..520].copy_from_slice(&checksum(&body[..*n]).to_le_bytes());
        let mut r = *remaining as u64;
        splitmix(&mut r);
        header[520..528].copy_from_slice(&r.to_le_bytes());
        for (h, m) in header.iter_mut().zip(mask) {
            *h ^= m;
        }
        ofb(&mut body, key);
        [header, body].concat()
    });
    let mut out = like[..HEADER].to_vec();
    for sl in slices {
        out.extend(sl);
    }
    let mut footer = like[like.len() - FOOTER..].to_vec();
    footer[0x80..0x88].copy_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend(footer);
    sign(&mut out);
    Ok(out)
}

/// The game and user id that open this file: games whose app id is in `path` first, then
/// every game, each with the Steam accounts in `accounts`.
pub fn find_key(d: &[u8], path_hint: &str, accounts: &[u64]) -> Option<(&'static Game, u64)> {
    let hint = path_hint.replace('\\', "/").to_lowercase();
    let named = |g: &&Game| hint.contains(&format!("/{}/", g.app));
    let order = GAMES.iter().filter(named).chain(GAMES.iter().filter(|g| !named(g)));
    order.into_iter().find_map(|g: &'static Game| {
        accounts.iter().map(|&a| user_id(g, a)).find(|&u| id_matches(d, g, u)).map(|u| (g, u))
    })
}

/// Steam account numbers worth trying: the one in the save's own path (`userdata\<n>\`),
/// then every account that has played something on this computer.
pub fn accounts_for(path: &std::path::Path) -> Vec<u64> {
    let mut out = vec![];
    let comps: Vec<String> = path.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
    for w in comps.windows(2) {
        if w[0] == "userdata" {
            if let Ok(n) = w[1].parse::<u64>() {
                out.push(n);
            }
        }
    }
    for dir in crate::platform::steam_userdata_dirs() {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if let Ok(n) = e.file_name().to_string_lossy().parse::<u64>() {
                    if !out.contains(&n) {
                        out.push(n);
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tv(name: &str) -> Option<Vec<u8>> {
        let dir = std::env::var("MANDARIN_VECTORS").ok()?;
        std::fs::read(std::path::Path::new(&dir).join(name)).ok()
    }

    #[test]
    fn hashes() {
        // MurmurHash3_x86_32 reference values don't apply (the seed mix differs); check the
        // parts against each other instead: same input, same output, length matters.
        assert_ne!(checksum(b"abcdefgh"), checksum(b"abcdefghi"));
        assert_eq!(checksum(&[]), 0x9AE1_6A3B_2F90_404F);
        let mut s = 0u64;
        splitmix(&mut s);
        assert_eq!(s, 0xE220_A839_7B1D_CDAF);
    }

    #[test]
    fn round_trip_synthetic() {
        let game = &GAMES[0];
        let user = user_id(game, 83_920_375);
        let plain: Vec<u8> = (0..40_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut like = vec![0u8; HEADER + FOOTER + 16];
        like[..16].copy_from_slice(&[b'D', b'S', b'S', b'S', 2, 0, 0, 0, 0x10, 0, 0, 0, 0, 0, 0, 0]);
        let file = encrypt(&plain, &like, game, user).unwrap();
        assert!(looks(&file));
        assert!(id_matches(&file, game, user));
        assert!(!id_matches(&file, game, user ^ 1));
        assert_eq!(decrypt(&file, game, user).unwrap(), plain);
        assert!(decrypt(&file, game, user ^ 1).is_none());
        assert_eq!(find_key(&file, "", &[5, 83_920_375]).map(|(g, u)| (g.app, u)), Some(("2638890", user)));
        // The 64-bit Steam ID works too (typed in when Steam's folders aren't there).
        assert_eq!(find_key(&file, "", &[76_561_197_960_265_728 + 83_920_375]).map(|(_, u)| u), Some(user));
    }

    /// MandarinJuice's own test files (QualityControl.xUnit/Resources), if MANDARIN_VECTORS
    /// points at them: Steam id 76561197960265729 (account 1).
    #[test]
    #[ignore]
    fn mandarinjuice_vectors() {
        let plain = tv("decrypted.bin").expect("MANDARIN_VECTORS");
        let games = [
            ("encrypted-v1-steam.bin", g("v1", "", false, 11688424886205786022, 1, 16)),
            ("encrypted-v2-steam.bin", g("v2", "", false, 16977276748297568783, 2, 16)),
            ("encrypted-v3-steam.bin", g("v3", "", false, 3460376763846700646, 3, 16)),
        ];
        for (file, game) in games {
            let d = tv(file).unwrap();
            let user = user_id(&game, 1);
            assert!(id_matches(&d, &game, user), "{file}: id check");
            let got = decrypt(&d, &game, user).unwrap_or_else(|| panic!("{file}: decrypt"));
            assert_eq!(got, plain, "{file}");
            let again = encrypt(&got, &d, &game, user).unwrap();
            assert_eq!(decrypt(&again, &game, user).unwrap(), plain, "{file}: re-encrypted");
            println!("{file}: OK ({} bytes)", got.len());
        }
    }

    /// Research aid: decrypts MANDARIN_SAVE (found the way the app does) and writes the
    /// data next to it as .dec, then checks re-encrypting gives the file back.
    #[test]
    #[ignore]
    fn real_save() {
        let p = std::path::PathBuf::from(std::env::var("MANDARIN_SAVE").expect("MANDARIN_SAVE"));
        let d = std::fs::read(&p).unwrap();
        let hint = std::env::var("MANDARIN_HINT").unwrap_or_else(|_| p.to_string_lossy().to_string());
        let (game, user) = find_key(&d, &hint, &accounts_for(std::path::Path::new(&hint))).expect("no key");
        let plain = decrypt(&d, game, user).expect("decrypt");
        println!("{}: user {user:#x}, {} bytes plain, starts {:02x?}", game.title, plain.len(), &plain[..64.min(plain.len())]);
        std::fs::write(p.with_extension("dec"), &plain).unwrap();
        let again = encrypt(&plain, &d, game, user).unwrap();
        println!("re-encrypted identical: {} (len {} vs {})", again == d, again.len(), d.len());
        let diffs: Vec<usize> = (0..again.len().min(d.len())).filter(|&i| again[i] != d[i]).collect();
        println!("{} bytes differ, first {:x?}, last {:x?}", diffs.len(), diffs.first(), diffs.last());
        assert_eq!(decrypt(&again, game, user).unwrap(), plain);
    }
}
