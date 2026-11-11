//! The checksums Apollo patch scripts can compute, ported from apollo-lib's `crc_util.c`
//! (GPL-3.0-or-later, Damian Parrino; the CRC routines are Michael Barr's public-domain code,
//! the CRC forcer is Project Nayuki's, lookup3 and one-at-a-time are Bob Jenkins's).

/// A CRC described by its parameters (width 16, 32 or 64).
#[derive(Clone, Copy, Default, Debug)]
pub struct Crc {
    pub width: u32,
    pub poly: u64,
    pub init: u64,
    pub xor_out: u64,
    pub ref_in: bool,
    pub ref_out: bool,
}

pub const CRC32_POLY: u64 = 0x04C1_1DB7;

fn reflect(mut v: u64, bits: u32) -> u64 {
    let mut r = 0;
    for b in 0..bits {
        if v & 1 != 0 {
            r |= 1 << (bits - 1 - b);
        }
        v >>= 1;
    }
    r
}

/// The bitwise CRC of `width` bits (16, 32 or 64); other fields of `c` as given.
pub fn crc(d: &[u8], c: &Crc, width: u32) -> u64 {
    let mask = if width == 64 { u64::MAX } else { (1u64 << width) - 1 };
    let top = 1u64 << (width - 1);
    let poly = c.poly & mask;
    let mut crc = c.init & mask;
    for &b in d {
        let b = if c.ref_in { reflect(b as u64, 8) } else { b as u64 };
        crc ^= (b << (width - 8)) & mask;
        for _ in 0..8 {
            crc = if crc & top != 0 { ((crc << 1) ^ poly) & mask } else { (crc << 1) & mask };
        }
    }
    if c.ref_out {
        crc = reflect(crc, width);
    }
    (crc ^ c.xor_out) & mask
}

// --- CRC-32 forcer ---

fn multiply_mod(mut x: u64, mut y: u64) -> u64 {
    let mut z = 0;
    while y != 0 {
        z ^= x.wrapping_mul(y & 1);
        y >>= 1;
        x <<= 1;
        if (x >> 32) & 1 != 0 {
            x ^= CRC32_POLY + 0x1_0000_0000;
        }
    }
    z
}

fn pow_mod(mut x: u64, mut y: u64) -> u64 {
    let mut z = 1;
    while y != 0 {
        if y & 1 != 0 {
            z = multiply_mod(z, x);
        }
        x = multiply_mod(x, x);
        y >>= 1;
    }
    z
}

fn degree(mut x: u64) -> i32 {
    let mut r = -1;
    while x != 0 {
        x >>= 1;
        r += 1;
    }
    r
}

fn divmod(mut x: u64, y: u64) -> (u64, u64) {
    if y == 0 || x == 0 {
        return (0, x);
    }
    let yd = degree(y);
    let mut z = 0;
    let mut i = degree(x) - yd;
    while i >= 0 {
        if (x >> (i + yd)) & 1 != 0 {
            x ^= y << i;
            z |= 1 << i;
        }
        i -= 1;
    }
    (z, x)
}

fn reciprocal_mod(y0: u64) -> u64 {
    let (mut x, mut y) = (CRC32_POLY + 0x1_0000_0000, y0);
    let (mut a, mut b) = (0u64, 1u64);
    while y != 0 {
        let (q, r) = divmod(x, y);
        let c = a ^ multiply_mod(q, b);
        x = y;
        y = r;
        a = b;
        b = c;
    }
    if x == 1 {
        a
    } else {
        0
    }
}

/// The four bytes (big-endian) to put at `offset` so the CRC-32 of `d` becomes `newcrc`.
pub fn force_crc32(d: &[u8], offset: u32, newcrc: u32) -> u32 {
    let len = d.len() as u32;
    if len < 4 || offset > len - 4 {
        return 0;
    }
    let cfg = Crc { width: 32, poly: CRC32_POLY, init: 0xFFFF_FFFF, xor_out: 0xFFFF_FFFF, ref_in: true, ref_out: false };
    let mut delta = (reflect(newcrc as u64, 32) as u32) ^ (crc(d, &cfg, 32) as u32);
    delta = multiply_mod(reciprocal_mod(pow_mod(2, (len - offset) as u64 * 8)), delta as u64) as u32;
    delta = reflect(delta as u64, 32) as u32;
    let mut r = 0u32;
    for i in 0..4 {
        r |= ((d[(offset + i) as usize] ^ ((delta >> (i * 8)) & 0xFF) as u8) as u32) << (24 - i * 8);
    }
    r
}

fn table32(poly: u32) -> [u32; 256] {
    let mut t = [0u32; 256];
    for (i, e) in t.iter_mut().enumerate() {
        let mut r = (i as u32) << 24;
        for _ in 0..8 {
            r = if r & 0x8000_0000 != 0 { (r << 1) ^ poly } else { r << 1 };
        }
        *e = r;
    }
    t
}

/// Kingdom Hearts HD 2.5 (its table loop runs 255 times, as the game does).
pub fn kh25(d: &[u8]) -> u32 {
    let mut t = [0u32; 256];
    for (x, e) in t.iter_mut().enumerate() {
        let mut r = (x as i32) << 24;
        for _ in 0..0xff {
            r = (r << 1) ^ if r < 0 { CRC32_POLY as i32 } else { 0 };
        }
        *e = r as u32;
    }
    let mut crc = 0xFFFF_FFFFu32;
    let step = |crc: u32, b: u8| (crc << 8) ^ t[(((crc >> 24) ^ b as u32) & 0xFF) as usize];
    let n = d.len();
    for &b in d.iter().take(8.min(n)) {
        crc = step(crc, b);
    }
    // The checksum's own bytes (0x8..0xB) are skipped.
    for &b in d.iter().skip(12) {
        crc = step(crc, b);
    }
    !crc
}

/// Kingdom Hearts Re:Chain of Memories.
pub fn khcom(d: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32 as i32;
    for &b in d {
        c ^= ((b as u32) << 31) as i32;
        c = (c << 1) ^ if c < 0 { CRC32_POLY as i32 } else { 0 };
    }
    !(c as u32)
}

/// Electronic Arts "MC02".
pub fn mc02(d: &[u8]) -> u32 {
    if d.len() < 4 {
        return 0;
    }
    let t = table32(CRC32_POLY as u32);
    let p = |i: usize| d[i] as u32;
    let third = (p(2) << 8) | (p(2) >> 24);
    let first = ((p(0) << 24) | (p(0) >> 8)) | ((p(1) << 16) | (p(1) >> 16));
    let mut seed = !((third | first) | p(3));
    for &b in &d[4..] {
        let look = t[(((seed >> 22) & 0x3FC) >> 2) as usize];
        seed = look ^ (b as u32 | (seed << 8));
    }
    !seed
}

pub fn djb2(d: &[u8]) -> u32 {
    d.iter().fold(5381u32, |h, &b| (h << 5).wrapping_add(h).wrapping_add(b as u32))
}

pub fn sdbm(d: &[u8], init: u32) -> u32 {
    d.iter().fold(init, |h, &b| h.wrapping_mul(0x1003f).wrapping_add(b as u32))
}

pub fn fnv1(d: &[u8], init: u32) -> u32 {
    d.iter().fold(init, |h, &b| h.wrapping_mul(0x0100_0193) ^ b as u32)
}

pub fn checksum32(d: &[u8]) -> u32 {
    d.iter().fold(0u32, |s, &b| s.wrapping_add(b as i8 as i32 as u32))
}

/// Final Fantasy X / X-2 (CRC-16 CCITT with the last table entry zeroed, as the game has it).
pub fn ffx(d: &[u8]) -> u16 {
    let mut t = [0u16; 256];
    for (i, e) in t.iter_mut().enumerate() {
        let mut r = (i as u16) << 8;
        for _ in 0..8 {
            r = if r & 0x8000 != 0 { (r << 1) ^ 0x1021 } else { r << 1 };
        }
        *e = r;
    }
    t[0xFF] = 0;
    let mut c = 0xFFFFu16;
    for &b in d {
        c = (c << 8) ^ t[(((c >> 8) ^ b as u16) & 0xFF) as usize];
    }
    !c
}

fn rot(x: u32, k: u32) -> u32 {
    x.rotate_left(k)
}

/// Bob Jenkins's lookup3 `hashlittle2`, byte by byte. Returns (c, b).
pub fn lookup3_little2(k: &[u8], pc: u32, pb: u32) -> (u32, u32) {
    let mut a = 0xDEAD_BEEFu32.wrapping_add(k.len() as u32).wrapping_add(pc);
    let mut b = a;
    let mut c = a.wrapping_add(pb);
    let w = |s: &[u8], i: usize| -> u32 { s.get(i).copied().unwrap_or(0) as u32 };
    let mut k = k;
    while k.len() > 12 {
        a = a.wrapping_add(w(k, 0) | w(k, 1) << 8 | w(k, 2) << 16 | w(k, 3) << 24);
        b = b.wrapping_add(w(k, 4) | w(k, 5) << 8 | w(k, 6) << 16 | w(k, 7) << 24);
        c = c.wrapping_add(w(k, 8) | w(k, 9) << 8 | w(k, 10) << 16 | w(k, 11) << 24);
        a = a.wrapping_sub(c); a ^= rot(c, 4); c = c.wrapping_add(b);
        b = b.wrapping_sub(a); b ^= rot(a, 6); a = a.wrapping_add(c);
        c = c.wrapping_sub(b); c ^= rot(b, 8); b = b.wrapping_add(a);
        a = a.wrapping_sub(c); a ^= rot(c, 16); c = c.wrapping_add(b);
        b = b.wrapping_sub(a); b ^= rot(a, 19); a = a.wrapping_add(c);
        c = c.wrapping_sub(b); c ^= rot(b, 4); b = b.wrapping_add(a);
        k = &k[12..];
    }
    if k.is_empty() {
        return (c, b);
    }
    for i in (0..k.len()).rev() {
        let v = (k[i] as u32) << ((i % 4) * 8);
        match i / 4 {
            0 => a = a.wrapping_add(v),
            1 => b = b.wrapping_add(v),
            _ => c = c.wrapping_add(v),
        }
    }
    c ^= b; c = c.wrapping_sub(rot(b, 14));
    a ^= c; a = a.wrapping_sub(rot(c, 11));
    b ^= a; b = b.wrapping_sub(rot(a, 25));
    c ^= b; c = c.wrapping_sub(rot(b, 16));
    a ^= c; a = a.wrapping_sub(rot(c, 4));
    b ^= a; b = b.wrapping_sub(rot(a, 14));
    c ^= b; c = c.wrapping_sub(rot(b, 24));
    (c, b)
}

pub fn add(d: &[u8]) -> u32 {
    d.iter().fold(0u32, |s, &b| s.wrapping_add(b as u32))
}

pub fn wadd(d: &[u8], le: bool) -> u32 {
    d.chunks_exact(2).fold(0u32, |s, w| {
        s.wrapping_add(if le { u16::from_le_bytes([w[0], w[1]]) } else { u16::from_be_bytes([w[0], w[1]]) } as u32)
    })
}

pub fn dwadd(d: &[u8], le: bool) -> u32 {
    d.chunks_exact(4).fold(0u32, |s, w| {
        let a = [w[0], w[1], w[2], w[3]];
        s.wrapping_add(if le { u32::from_le_bytes(a) } else { u32::from_be_bytes(a) })
    })
}

/// Sums the high (big-endian, second) dword of each 8-byte word.
pub fn qwadd(d: &[u8]) -> u32 {
    d.chunks_exact(8).fold(0u32, |s, w| s.wrapping_add(u32::from_be_bytes([w[4], w[5], w[6], w[7]])))
}

pub fn wsub(d: &[u8]) -> u32 {
    d.chunks_exact(2).fold(0u32, |s, w| s.wrapping_sub(u16::from_be_bytes([w[0], w[1]]) as u32))
}

/// Samurai Warriors 4: four sums over fixed regions of the save.
pub fn sw4(d: &[u8]) -> [u32; 4] {
    const OFF1: usize = 0x4;
    const OFF2: usize = 0xA8;
    const OFF3: usize = 0xC16;
    const JP: usize = 0x58298;
    if d.len() < JP {
        return [0; 4];
    }
    let s = |a: usize, b: usize| add(d.get(a..b).unwrap_or(&[]));
    let jp = d[JP..JP + 4] != [0, 0, 0, 0];
    let n2 = s(OFF1 + 4, OFF2);
    let n3 = s(OFF2 + 4, 2284);
    let (n1, n4, n5, n6) = if jp {
        (s(OFF3 + 4, 30294), s(33630, 361106), s(30294, 33631), s(OFF1 + 4, 30294))
    } else {
        (s(OFF3 + 4, 30934), s(34270, 361778), s(30934, 34271), s(OFF1 + 4, 30934))
    };
    let c0 = 0x7FFF_FFFF & n5.wrapping_add(n1.wrapping_add(n2).wrapping_mul(n3));
    let c1 = 0x7FFF_FFFF & n2.wrapping_mul(n1.wrapping_add(n3).wrapping_add(n4));
    [c0, c1, n6, 0x7FFF_FFFF & c0.wrapping_add(c1).wrapping_add(n6)]
}

/// Metal Gear Solid 2.
pub fn mgs2(d: &[u8]) -> u32 {
    let mut c: i32 = -1;
    for &b in d {
        c ^= b as i8 as i32;
        for _ in 0..8 {
            let n = c & 1;
            c = (c >> 1) & 0x7fff_ffff;
            if n != 0 {
                c ^= 0xEDB8_8320u32 as i32;
            }
        }
    }
    !(c as u32)
}

pub fn tiara2(d: &[u8]) -> u32 {
    let (mut c, mut a) = (1u32, 0x3428u32);
    for &b in d {
        a = a.wrapping_add(b as u32);
        c = c.wrapping_mul(a).wrapping_add(a);
    }
    c
}

fn sha1_parts(parts: &[&[u8]]) -> [u8; 20] {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}

/// Tales of Zestiria.
pub fn toz(d: &[u8]) -> [u8; 20] {
    const SALT: [&[u8]; 8] = [b"SRA", b"ROS", b"MIC", b"LAI", b"EDN", b"DEZ", b"ZAB", b"ALI"];
    let mut h = sha1_parts(&[d, b"TO12"]);
    for i in 0..100 {
        h = sha1_parts(&[&h, SALT[i % 8]]);
    }
    h
}

pub fn adler16(d: &[u8]) -> u16 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in d {
        a = (a + x as u32) % 251;
        b = (b + a) % 251;
    }
    ((b << 8) | a) as u16
}

/// zlib's Adler-32, continuing from `start` (1 for a fresh sum).
pub fn adler32(start: u32, d: &[u8]) -> u32 {
    let (mut a, mut b) = (start & 0xFFFF, start >> 16);
    for c in d.chunks(5552) {
        for &x in c {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

pub fn fletcher16(d: &[u8]) -> u16 {
    let (mut c0, mut c1) = (0u32, 0u32);
    for blk in d.chunks(5802) {
        for &x in blk {
            c0 += x as u32;
            c1 += c0;
        }
        c0 %= 255;
        c1 %= 255;
    }
    (c1 << 8 | c0) as u16
}

/// Fletcher-32 over little-endian 16-bit words (an odd last byte is zero-padded).
pub fn fletcher32(d: &[u8]) -> u32 {
    let (mut c0, mut c1) = (0u32, 0u32);
    let words: Vec<u32> = d.chunks(2).map(|w| w[0] as u32 | (*w.get(1).unwrap_or(&0) as u32) << 8).collect();
    for blk in words.chunks(360) {
        for &w in blk {
            c0 += w;
            c1 += c0;
        }
        c0 %= 65535;
        c1 %= 65535;
    }
    c1 << 16 | c0
}

/// Dead Rising: rewrites the 4-byte sum after every 16 bytes, in place.
pub fn deadrising(d: &mut [u8]) {
    let mut s = 0;
    while s < d.len() {
        let (mut lo, mut hi) = (0u16, 0u16);
        for n in 0..16 {
            let b = *d.get(s).unwrap_or(&0) as u16;
            lo = lo.wrapping_add(b);
            hi = hi.wrapping_add(if n % 2 == 0 { b } else { b.wrapping_neg() });
            s += 1;
        }
        if s + 4 <= d.len() {
            d[s..s + 4].copy_from_slice(&[lo as u8, (lo >> 8) as u8, hi as u8, (hi >> 8) as u8]);
        }
        s += 4;
    }
}

/// Castlevania: Lords of Shadow.
pub fn castlevania(d: &[u8]) -> u32 {
    let (mut a, mut b) = (0i32, 0i32);
    let mut i = 0;
    while i < d.len() {
        a = a.wrapping_add((d[i] as i32) ^ (i as i32 & 255));
        b = b.wrapping_add((*d.get(i + 1).unwrap_or(&0) as i32) ^ ((i as i32 + 1) & 255));
        i += 2;
    }
    a.wrapping_add(b) as u32
}

/// Dragon Ball Xenoverse 2: the eight one-byte sums, packed so `out[7]` is the high byte.
pub fn dbzxv2(d: &[u8]) -> u64 {
    let pack = |o: &[u8; 8]| o.iter().rev().fold(0u64, |r, &b| (r << 8) | b as u64);
    let g = |i: usize| *d.get(i).unwrap_or(&0);
    let h = |i: usize| g(0x20 + i);
    let mut out = [0u8; 8];
    let blocks = d.len() / 0x20;
    if (0..8).all(|i| h(0x14 + i) == 0) {
        out[7] = 0xFF;
        for i in 5..blocks {
            out[1] = out[1].wrapping_add(g(i * 0x20));
        }
        return pack(&out);
    }
    out[1] = h(0x1A);
    for i in 0..14 {
        out[6] = out[6].wrapping_add(h(0x6 + i));
    }
    for i in 0..8 {
        out[5] = out[5].wrapping_add(h(0x1C + i * 4));
        out[4] = out[4].wrapping_add(h(0x4C + i * 4));
    }
    for i in 0..4 {
        out[3] = out[3].wrapping_add(h(0x3C + i * 4));
        out[2] = out[2].wrapping_add(h(0x6C + i * 4));
    }
    for i in 5..blocks {
        out[0] = out[0].wrapping_add(g(i * 0x20));
    }
    out[7] = h(0x5);
    for i in 0..7 {
        out[7] = out[7].wrapping_add(out[i]);
    }
    pack(&out)
}

pub fn jenkins_oaat(d: &[u8], mut h: u32) -> u32 {
    for &b in d {
        h = h.wrapping_add(b as i8 as i32 as u32);
        h = h.wrapping_add(h << 10);
        h ^= h >> 6;
    }
    h = h.wrapping_add(h << 3);
    h ^= h >> 11;
    h.wrapping_add(h << 15)
}

pub fn murmur3_32(key: &[u8], mut h: u32) -> u32 {
    let scramble = |k: u32| k.wrapping_mul(0xcc9e_2d51).rotate_left(15).wrapping_mul(0x1b87_3593);
    let mut it = key.chunks_exact(4);
    for c in &mut it {
        h ^= scramble(u32::from_le_bytes([c[0], c[1], c[2], c[3]]));
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe654_6b64);
    }
    let rest = it.remainder();
    let mut k = 0u32;
    for i in (0..rest.len()).rev() {
        k = (k << 8) | rest[i] as u32;
    }
    h ^= scramble(k);
    h ^= key.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^ (h >> 16)
}

/// The Linux kernel's old `jhash`.
pub fn jhash(k: &[u8], init: u32) -> u32 {
    let (mut a, mut b, mut c) = (0x9e37_79b9u32, 0x9e37_79b9u32, init);
    let mix = |a: &mut u32, b: &mut u32, c: &mut u32| {
        *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 13);
        *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 8);
        *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 13);
        *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 12);
        *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 16);
        *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 5);
        *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 3);
        *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 10);
        *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 15);
    };
    let le = |s: &[u8]| u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    let mut k2 = k;
    while k2.len() >= 12 {
        a = a.wrapping_add(le(&k2[0..4]));
        b = b.wrapping_add(le(&k2[4..8]));
        c = c.wrapping_add(le(&k2[8..12]));
        mix(&mut a, &mut b, &mut c);
        k2 = &k2[12..];
    }
    c = c.wrapping_add(k.len() as u32);
    for i in (0..k2.len()).rev() {
        let v = k2[i] as u32;
        match i {
            8..=10 => c = c.wrapping_add(v << ((i - 7) * 8)),
            4..=7 => b = b.wrapping_add(v << ((i - 4) * 8)),
            _ => a = a.wrapping_add(v << (i * 8)),
        }
    }
    mix(&mut a, &mut b, &mut c);
    c
}

/// MD5 folded to 32 bits: the four little-endian words XORed together.
pub fn md5_xor(d: &[u8]) -> u32 {
    use md5::{Digest, Md5};
    let h: [u8; 16] = Md5::digest(d).into();
    h.chunks(4).fold(0, |x, w| x ^ u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
}

/// SHA-1 folded to 64 bits: its three big-endian 8-byte words (the last zero-padded) XORed.
pub fn sha1_xor64(d: &[u8]) -> u64 {
    let mut h = [0u8; 24];
    h[..20].copy_from_slice(&sha1_parts(&[d]));
    h.chunks(8).fold(0, |x, w| x ^ u64::from_be_bytes(w.try_into().unwrap()))
}

/// Final Fantasy XIII: the first byte of every dword, summed.
pub fn ff13(d: &[u8]) -> u32 {
    d.chunks_exact(4).fold(0u32, |s, w| s.wrapping_add(w[0] as u32))
}

/// Metal Gear Solid: Peace Walker.
pub fn mgspw(d: &[u8]) -> u32 {
    let t = &super::keys::MGS_PW_TABLE;
    let mut c = 0xFFFF_FFFFu32;
    for &b in d {
        c = t[(b ^ c as u8) as usize] ^ (c >> 8) ^ t[0];
    }
    !c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        let s = b"123456789";
        let c32 = Crc { width: 32, poly: CRC32_POLY, init: 0xFFFF_FFFF, xor_out: 0xFFFF_FFFF, ref_in: true, ref_out: true };
        assert_eq!(crc(s, &c32, 32), 0xCBF4_3926);
        let bzip = Crc { ref_in: false, ref_out: false, ..c32 };
        assert_eq!(crc(s, &bzip, 32), 0xFC89_1918);
        let x = Crc { width: 16, poly: 0x1021, ..Default::default() };
        assert_eq!(crc(s, &x, 16), 0x31C3);
        let ecma = Crc { width: 64, poly: 0x42F0_E1EB_A9EA_3693, ..Default::default() };
        assert_eq!(crc(s, &ecma, 64), 0x6C40_DF5F_0B49_7347);
        assert_eq!(adler32(1, b"Wikipedia"), 0x11E6_0398);
        assert_eq!(fletcher16(b"abcde"), 0xC8F0);
        assert_eq!(fletcher32(b"abcde"), 0xF04F_C729);
        assert_eq!(fletcher32(b"abcdef"), 0x5650_2D2A);
        // Forcing the CRC: patch the bytes in, and the file's CRC-32 is the one asked for.
        let mut d = b"hello world, this is a save".to_vec();
        let want = 0x1234_5678;
        let p = force_crc32(&d, 4, want);
        d[4..8].copy_from_slice(&p.to_be_bytes());
        assert_eq!(crc(&d, &c32, 32), want as u64);
    }
}
