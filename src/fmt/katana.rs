//! Koei Tecmo's "Katana" engine saves (Steam): Nioh, Nioh 2 and Nioh 3 (a custom AES in CTR
//! mode whose keys sit, themselves encrypted, in the header), Wo Long: Fallen Dynasty and Fatal
//! Frame II: Crimson Butterfly Remake (a table-driven block cipher over JSON, with two 32-byte
//! checksums) and Stranger of Paradise: Final Fantasy Origin (AES-128-CBC).
//!
//! Ported from KatanaSaveDataResigner by Michał Gębicki (MIT). Saving keeps each file's own
//! data keys, so a save written back unchanged is byte-for-byte the original.

use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::Engine;
use std::sync::OnceLock;

const SBOX: &str = "HC8DU6MBSdqmzeCKGacE1AYa2kkI4vaynuEiSc57fl6gCSpjr0nOcHs8I4D6F0fyYmJsWRDMKZy1RljHRBPnONWvJ4PU1aCe43Y7hQTZ1phgZtR4U+rKDo1WU0Ti772pmxAKoROT8EMLfDmKR9/TxQ40MaauWrjn5jFDwKoP4IISTNHfi6WscMU9G46TF015Ts5jxDMOFFfw2Blbm2Fx8iszfv0sC7YjILnUkRmUBKQwE4rx0AXsXqxK1NalF3/55fYAKdeTLV4s8YGjt2M5V8Izhy2oPwLMCGd0YNjw2mdAZIdVu3/yEMkDFLWAZsuR9h95WIi8lcIGX+kJMu2bhQ==";
const HEADER_KEY1: &str = "NTEfzfhSTqJ5zFGSS1hI/g==";
const HEADER_KEY2: &str = "DoyVzWMp9sJT7Po01bR/Xg==";
const HEADER_STATE1: &str = "G9/dVyfLzoc66sKe4FspJQ==";
const HEADER_STATE2: &str = "/UxAoo7OGYt3AbwMFLdXvQ==";
const DATA_KEY0: &str = "VFtevf14Q1V8fXxydMdEkQ==";
const DATA_STATE0: &str = "Vwd4cd/s6OSHYsd+afkEXQ==";
const WL_IV: &str = "u0CECl+cfUDBjpAo++5Rjw==";
const WL_E0: &str = "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";
const WL_D0: &str = "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";
const WL_E1: &str = "UMTjOub+TYrdP+abcohmx3ol95XwaAlza442rqzovtz8tGw5DNxlSmdSU+TLuu04+6uYaPd3/SKQJa7GW59D/kCSQ3q35b5YJ8AQnnxfU2CQgoyHJ2cy3wCnIkF8+HEhbZLNBEr1/9tKUt2aNqqsu4eXYdXNYp4OhzBDlLGa7y+SX9mKXz1HhNgNBBBpl+s/56ZReLibFvxglhLsCQH509ctp4ErOzw5xymqWRTQq1A=";
const WL_D1: &str = "1y2ngSs7PDnHKapZFNCrUCl0T3plgUJv+NqasGJd1suXpftXTPUNFZ1b2N+ah0x7ZVfQRttQ9kLRrtXKB9yUpJSEWH6+ByYECv4jiNZyQW4fYzFUKoN+erT5BYzcjGLmylIEdzXgTy6eenv2aHVnaob9vmX/sktZq5o02PYPHJzu00RkeU/1PFQof4FdlShEc6OEaZecsVgtZ4q9Cb1XxVDE4zrm/k2K3T/mm3KIZsc=";
const WL_C: &str = "810MZdvnQKKZE6sJA39HX4KEyulhUPTGw22xuVNUrMT/q3esR76Rn5X2+FllTqVaTyLL9AlKCpns4x3+52cWsiRwBOBRghtdheKWV86G1ZSTvzrAsbWkyES8zNdrfi6IfSsDF0npaQC0XBnqIPLK7pecbDI0qI+dcQboOwx8vbBy84tiJXscutEm9wV/P5KAjl9QTMZY4bahdd7H8YHNI9vcJ91Vatr8QqanowGz0O12+ku3Dbh05m6gjThz0y8weIctUm85hD1AVCELFTNW5EHB8MJG+8/v2cR6udZgEWY2ihq7mir9N8UUmDWiEIxtwwLJ+YNDra6vBw4pD5BT2JsTPtRNY1vrHvUfnuVFMRJkaCypPAjfedIoiV6qYUgY";
const SOP_KEY: &str = "CpBtJ2lVBrWMGiKdxHH0VA==";
const SOP_IV: &str = "qBvSUQRCr6JPWzcVDDvPig==";

fn b64(s: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD.decode(s).unwrap_or_default()
}

struct Tables {
    sbox: Vec<u8>,
    hk1: Vec<u8>,
    hk2: Vec<u8>,
    hs1: Vec<u8>,
    hs2: Vec<u8>,
    dk0: Vec<u8>,
    ds0: Vec<u8>,
    wl_iv: Vec<u8>,
    e0: Vec<u8>,
    d0: Vec<u8>,
    e1: Vec<u8>,
    d1: Vec<u8>,
    c: Vec<u8>,
}

fn t() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| Tables {
        sbox: b64(SBOX),
        hk1: b64(HEADER_KEY1),
        hk2: b64(HEADER_KEY2),
        hs1: b64(HEADER_STATE1),
        hs2: b64(HEADER_STATE2),
        dk0: b64(DATA_KEY0),
        ds0: b64(DATA_STATE0),
        wl_iv: b64(WL_IV),
        e0: b64(WL_E0),
        d0: b64(WL_D0),
        e1: b64(WL_E1),
        d1: b64(WL_D1),
        c: b64(WL_C),
    })
}

// ---- Nioh: AES-128 with the game's own S-box, in CTR mode --------------------------------

fn expand(key: &[u8], sbox: &[u8]) -> [u32; 44] {
    const RCON: [u32; 11] = [0, 0x0100_0000, 0x0200_0000, 0x0400_0000, 0x0800_0000, 0x1000_0000, 0x2000_0000, 0x4000_0000, 0x8000_0000, 0x1B00_0000, 0x3600_0000];
    let mut w = [0u32; 44];
    for i in 0..4 {
        w[i] = u32::from_be_bytes([key[i * 4], key[i * 4 + 1], key[i * 4 + 2], key[i * 4 + 3]]);
    }
    for i in 4..44 {
        let mut temp = w[i - 1];
        if i & 3 == 0 {
            temp = temp.rotate_left(8);
            let b = temp.to_be_bytes();
            temp = u32::from_be_bytes([sbox[b[0] as usize], sbox[b[1] as usize], sbox[b[2] as usize], sbox[b[3] as usize]]) ^ RCON[i >> 2];
        }
        w[i] = w[i - 4] ^ temp;
    }
    w
}

fn xtime(x: u8) -> u8 {
    (x << 1) ^ if x & 0x80 != 0 { 0x1B } else { 0 }
}

fn encrypt_block(s: &mut [u8; 16], rk: &[u32; 44], sbox: &[u8]) {
    let add = |s: &mut [u8; 16], round: usize| {
        for i in 0..4 {
            let w = rk[round * 4 + i].to_be_bytes();
            for j in 0..4 {
                s[i * 4 + j] ^= w[j];
            }
        }
    };
    add(s, 0);
    for round in 1..=10 {
        for b in s.iter_mut() {
            *b = sbox[*b as usize];
        }
        // ShiftRows
        let t = *s;
        for c in 0..4 {
            for r in 0..4 {
                s[c * 4 + r] = t[((c + r) % 4) * 4 + r];
            }
        }
        if round < 10 {
            for c in 0..4 {
                let (a0, a1, a2, a3) = (s[c * 4], s[c * 4 + 1], s[c * 4 + 2], s[c * 4 + 3]);
                s[c * 4] = a2 ^ a3 ^ a1 ^ xtime(a0 ^ a1);
                s[c * 4 + 1] = a0 ^ a3 ^ a2 ^ xtime(a1 ^ a2);
                s[c * 4 + 2] = a0 ^ a1 ^ a3 ^ xtime(a2 ^ a3);
                s[c * 4 + 3] = a2 ^ a0 ^ a1 ^ xtime(a3 ^ a0);
            }
        }
        add(s, round);
    }
}

/// CTR: the counter block's first 12 bytes, then a big-endian counter.
fn ctr(data: &mut [u8], counter: &[u8], rk: &[u32; 44], sbox: &[u8]) {
    let mut n = u32::from_be_bytes([counter[12], counter[13], counter[14], counter[15]]);
    for chunk in data.chunks_mut(16) {
        let mut block = [0u8; 16];
        block[..12].copy_from_slice(&counter[..12]);
        block[12..].copy_from_slice(&n.to_be_bytes());
        encrypt_block(&mut block, rk, sbox);
        chunk.iter_mut().zip(block).for_each(|(d, k)| *d ^= k);
        n = n.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Nioh and Nioh 2.
    Nioh,
    Nioh3,
    WoLong,
    FatalFrame2,
    StrangerOfParadise,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Nioh => "Nioh / Nioh 2",
            Kind::Nioh3 => "Nioh 3",
            Kind::WoLong => "Wo Long: Fallen Dynasty",
            Kind::FatalFrame2 => "Fatal Frame II Remake",
            Kind::StrangerOfParadise => "Stranger of Paradise",
        }
    }

    fn header(self) -> usize {
        match self {
            Kind::Nioh => 0x148,
            Kind::Nioh3 => 0x158,
            _ => 0x100,
        }
    }

    fn key_offsets(self) -> [usize; 4] {
        if self == Kind::Nioh3 { [0x49, 0x59, 0x69, 0x79] } else { [0x40, 0x50, 0x60, 0x70] }
    }

    fn json_at(self) -> usize {
        if self == Kind::FatalFrame2 { 0x110 } else { 0x108 }
    }

    fn sums(self) -> (usize, usize) {
        if self == Kind::FatalFrame2 { (0x50, 0x70) } else { (0x3C, 0x5C) }
    }
}

fn header_crypt(h: &mut [u8]) {
    let t = t();
    ctr(h, &t.hs1, &expand(&t.hk1, &t.sbox), &t.sbox);
    ctr(h, &t.hs2, &expand(&t.hk2, &t.sbox), &t.sbox);
}

/// Decrypts the data keys in a decrypted header (in place) and the data.
fn nioh_decrypt(kind: Kind, plain: &mut [u8]) {
    let t = t();
    let (h, data) = plain.split_at_mut(kind.header());
    let [k1, s1, k2, s2] = kind.key_offsets();
    let rk0 = expand(&t.dk0, &t.sbox);
    ctr(&mut h[k1..k1 + 16], &t.ds0, &rk0, &t.sbox);
    ctr(&mut h[s1..s1 + 16], &t.ds0, &rk0, &t.sbox);
    let rk1 = expand(&h[k1..k1 + 16], &t.sbox);
    let state1 = h[s1..s1 + 16].to_vec();
    ctr(&mut h[k2..k2 + 16], &state1, &rk1, &t.sbox);
    ctr(&mut h[s2..s2 + 16], &state1, &rk1, &t.sbox);
    let rk2 = expand(&h[k2..k2 + 16], &t.sbox);
    ctr(data, &h[s2..s2 + 16], &rk2, &t.sbox);
    ctr(data, &state1, &rk1, &t.sbox);
}

/// The reverse of [nioh_decrypt] followed by the header encryption.
fn nioh_encrypt(kind: Kind, plain: &mut [u8]) {
    let t = t();
    let (h, data) = plain.split_at_mut(kind.header());
    let [k1, s1, k2, s2] = kind.key_offsets();
    let rk1 = expand(&h[k1..k1 + 16], &t.sbox);
    let rk2 = expand(&h[k2..k2 + 16], &t.sbox);
    let state1 = h[s1..s1 + 16].to_vec();
    ctr(data, &h[s2..s2 + 16], &rk2, &t.sbox);
    ctr(data, &state1, &rk1, &t.sbox);
    ctr(&mut h[k2..k2 + 16], &state1, &rk1, &t.sbox);
    ctr(&mut h[s2..s2 + 16], &state1, &rk1, &t.sbox);
    let rk0 = expand(&t.dk0, &t.sbox);
    ctr(&mut h[k1..k1 + 16], &t.ds0, &rk0, &t.sbox);
    ctr(&mut h[s1..s1 + 16], &t.ds0, &rk0, &t.sbox);
    header_crypt(h);
}

// ---- Wo Long and Fatal Frame II: table cipher, chained like CBC --------------------------

const SEQ: [usize; 7] = [1, 2, 3, 0, 1, 2, 3];

fn rd(t: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([t[off], t[off + 1], t[off + 2], t[off + 3]])
}

fn b(x: u32, n: u32) -> usize {
    ((x >> (8 * n)) & 0xFF) as usize
}

fn wl_decrypt(data: &mut [u8]) {
    let t = t();
    let (d0, d1) = (&t.d0[..], &t.d1[..]);
    let u = |base: usize, k: usize| rd(d0, base + 4 * k);
    let mut c: [u32; 4] = std::array::from_fn(|i| rd(&t.wl_iv, 4 * i));
    for blk in data.chunks_exact_mut(16) {
        let orig: [u32; 4] = std::array::from_fn(|i| rd(blk, 4 * i));
        let mut a: [u32; 4] = std::array::from_fn(|i| orig[i] ^ rd(d1, 4 * i));
        let mut bb = [0u32; 4];
        for i in 0..4 {
            bb[i] = rd(d1, 4 * (4 + i)) ^ u(1, 2 * b(a[SEQ[i + 2]], 1)) ^ u(2, 2 * b(a[SEQ[i + 1]], 2)) ^ u(3, 2 * b(a[SEQ[i]], 3)) ^ u(4, 2 * b(a[SEQ[i + 3]], 0));
        }
        for i in 0..4 {
            for j in 0..4 {
                a[j] = rd(d1, 32 + 4 * (i * 8 + SEQ[j + 3])) ^ u(1, 2 * b(bb[SEQ[j + 2]], 2)) ^ u(2, 2 * b(bb[SEQ[j + 1]], 1)) ^ u(3, 2 * b(bb[SEQ[j]], 0)) ^ u(4, 2 * b(bb[SEQ[j + 3]], 3));
            }
            for j in 0..4 {
                bb[j] = rd(d1, 32 + 4 * (i * 8 + SEQ[j + 3] + 4)) ^ u(1, 2 * b(a[SEQ[j + 2]], 2)) ^ u(2, 2 * b(a[SEQ[j + 1]], 1)) ^ u(3, 2 * b(a[SEQ[j]], 0)) ^ u(4, 2 * b(a[SEQ[j + 3]], 3));
            }
        }
        for i in 0..4 {
            let bytes = [d0[8 * b(bb[SEQ[i + 3]], 3)], d0[8 * b(bb[SEQ[i + 2]], 2)], d0[8 * b(bb[SEQ[i + 1]], 1)], d0[8 * b(bb[SEQ[i]], 0)]];
            a[i] = u32::from_le_bytes(bytes) ^ rd(d1, 160 + 4 * i) ^ c[i];
        }
        c = orig;
        for i in 0..4 {
            blk[4 * i..4 * i + 4].copy_from_slice(&a[i].to_le_bytes());
        }
    }
}

fn wl_encrypt(data: &mut [u8]) {
    let t = t();
    let (e0, e1) = (&t.e0[..], &t.e1[..]);
    let u = |base: usize, k: usize| rd(e0, base + 4 * k);
    let mut c: [u32; 4] = std::array::from_fn(|i| rd(&t.wl_iv, 4 * i));
    for blk in data.chunks_exact_mut(16) {
        let mut a: [u32; 4] = std::array::from_fn(|i| rd(blk, 4 * i) ^ c[i] ^ rd(e1, 4 * i));
        let mut bb = [0u32; 4];
        for i in 0..4 {
            bb[i] = rd(e1, 4 * (4 + i)) ^ u(1, 2 * b(a[SEQ[i]], 1)) ^ u(2, 2 * b(a[SEQ[i + 1]], 2)) ^ u(3, 2 * b(a[SEQ[i + 2]], 3)) ^ u(4, 2 * b(a[SEQ[i + 3]], 0));
        }
        for i in 0..4 {
            for j in 0..4 {
                a[j] = rd(e1, 32 + 4 * (i * 8 + SEQ[j])) ^ u(1, 2 * b(bb[SEQ[j + 1]], 2)) ^ u(2, 2 * b(bb[SEQ[j + 2]], 1)) ^ u(3, 2 * b(bb[SEQ[j + 3]], 0)) ^ u(4, 2 * b(bb[SEQ[j]], 3));
            }
            for j in 0..4 {
                bb[j] = rd(e1, 32 + 4 * (i * 8 + SEQ[j + 3] + 4)) ^ u(1, 2 * b(a[SEQ[j + 3]], 2)) ^ u(2, 2 * b(a[SEQ[j]], 1)) ^ u(3, 2 * b(a[SEQ[j + 1]], 0)) ^ u(4, 2 * b(a[SEQ[j + 2]], 3));
            }
        }
        for i in 0..4 {
            let bytes = [e0[1 + 8 * b(bb[SEQ[i + 3]], 3)], e0[1 + 8 * b(bb[SEQ[i]], 2)], e0[1 + 8 * b(bb[SEQ[i + 1]], 1)], e0[1 + 8 * b(bb[SEQ[i + 2]], 0)]];
            c[i] = u32::from_le_bytes(bytes) ^ rd(e1, 160 + 4 * i);
            blk[4 * i..4 * i + 4].copy_from_slice(&c[i].to_le_bytes());
        }
    }
}

/// The game's 32-byte checksum of `data`, added onto `out` (which starts as zeros).
fn wl_sum(out: &mut [u8], data: &[u8]) {
    let tc = &t().c;
    let add = |out: &mut [u8], p: usize, x: u8| {
        let v = x as u32 + out[p] as u32 + ((out[p + 1] as u32) << 8);
        out[p] = v as u8;
        out[p + 1] = (v >> 8) as u8;
    };
    for (i, &x) in data.iter().enumerate() {
        add(out, 0, x);
        add(out, 2 * ((i >> 2) & 1) + 2, x);
        add(out, 2 * (i & 3) + 6, x);
    }
    for i in 0..31 {
        out[i + 1] = out[i + 1].wrapping_add((2 * i as u8).wrapping_add(out[i]));
    }
    for i in 0..32 {
        out[i] = tc[32 + out[i] as usize].wrapping_add(tc[i]);
    }
}

fn wl_sums(kind: Kind, plain: &mut [u8]) {
    let (ds, hs) = kind.sums();
    let (h, data) = plain.split_at_mut(0x100);
    h[ds..ds + 32].fill(0);
    wl_sum(&mut h[ds..ds + 32], data);
    h[hs..hs + 32].fill(0);
    let mut s = [0u8; 32];
    wl_sum(&mut s, h);
    h[hs..hs + 32].copy_from_slice(&s);
}

fn header_sum_ok(kind: Kind, plain: &[u8]) -> bool {
    let (_, hs) = kind.sums();
    let mut h = plain[..0x100].to_vec();
    h[hs..hs + 32].fill(0);
    let mut s = [0u8; 32];
    wl_sum(&mut s, &h);
    s[..] == plain[hs..hs + 32]
}

// ---- Stranger of Paradise: AES-128-CBC ----------------------------------------------------

fn sop_key() -> ([u8; 16], [u8; 16]) {
    let (k, iv) = (b64(SOP_KEY), b64(SOP_IV));
    (k.try_into().unwrap_or([0; 16]), iv.try_into().unwrap_or([0; 16]))
}

// ---- Layer ----------------------------------------------------------------------------------

fn magic(d: &[u8]) -> bool {
    [&b"NIOHUSR"[..], b"NIOHSYS", b"RNNUSR", b"RNNSYS", b"WLNUSR", b"WLNSYS"].iter().any(|m| d.starts_with(m))
}

/// The whole save decrypted (header included) and which game it is.
pub fn open(d: &[u8]) -> Option<(Kind, Vec<u8>)> {
    if d.len() < 0x200 {
        return None;
    }
    // Wo Long and Fatal Frame II: plain header, the data's first 8 bytes repeat header bytes 8..16.
    if d.starts_with(b"WLNUSR") || d.starts_with(b"WLNSYS") {
        if d[8..16] == d[0x100..0x108] {
            return None; // already decrypted
        }
        let mut plain = d.to_vec();
        wl_decrypt(&mut plain[0x100..]);
        if plain[8..16] != plain[0x100..0x108] {
            return None;
        }
        // The JSON starts at 0x108 (Wo Long) or 0x110 (Fatal Frame II); a valid header checksum
        // settles it when both could fit.
        let kind = match (plain[0x108] == b'{', plain[0x110] == b'{') {
            (true, false) => Kind::WoLong,
            (false, true) => Kind::FatalFrame2,
            _ => [Kind::WoLong, Kind::FatalFrame2].into_iter().find(|&k| header_sum_ok(k, &plain))?,
        };
        return Some((kind, plain));
    }
    if magic(d) {
        return None;
    }
    // Nioh: the header decrypts to its magic.
    let mut h = d[..0x158].to_vec();
    header_crypt(&mut h);
    let kind = if h.starts_with(b"NIOHUSR") || h.starts_with(b"NIOHSYS") {
        Some(Kind::Nioh)
    } else if h.starts_with(b"RNNUSR") || h.starts_with(b"RNNSYS") {
        Some(Kind::Nioh3)
    } else {
        None
    };
    if let Some(kind) = kind {
        let mut plain = d.to_vec();
        header_crypt(&mut plain[..kind.header()]);
        nioh_decrypt(kind, &mut plain);
        return Some((kind, plain));
    }
    // Stranger of Paradise: the whole file, AES-CBC.
    if d.len() % 16 == 0 {
        let (k, iv) = sop_key();
        let mut first = d[..16].to_vec();
        cbc::Decryptor::<aes::Aes128>::new(&k.into(), &iv.into()).decrypt_padded_mut::<NoPadding>(&mut first).ok()?;
        if first.starts_with(b"RNNUSR") || first.starts_with(b"RNNSYS") {
            let mut plain = d.to_vec();
            cbc::Decryptor::<aes::Aes128>::new(&k.into(), &iv.into()).decrypt_padded_mut::<NoPadding>(&mut plain).ok()?;
            return Some((Kind::StrangerOfParadise, plain));
        }
    }
    None
}

/// What the editor shows: the JSON for Wo Long and Fatal Frame II, the data after the header
/// for the others.
pub fn content(kind: Kind, plain: &[u8]) -> Vec<u8> {
    match kind {
        Kind::WoLong | Kind::FatalFrame2 => {
            let d = &plain[kind.json_at()..];
            // Where the JSON value ends (anything after it is padding).
            let mut it = serde_json::Deserializer::from_slice(d).into_iter::<serde_json::Value>();
            if let Some(Ok(_)) = it.next() {
                return d[..it.byte_offset()].to_vec();
            }
            let end = match d.iter().position(|&x| x == 0) {
                Some(z) if z > 0 && d[z - 1] == b'}' => z,
                _ => d.iter().rposition(|&x| x == b'}').map_or(0, |p| p + 1),
            };
            d[..end].to_vec()
        }
        _ => plain[kind.header()..].to_vec(),
    }
}

/// The encrypted save for the (edited) content, keeping `plain`'s header and keys.
pub fn close(kind: Kind, plain: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let mut p = plain.to_vec();
    match kind {
        Kind::WoLong | Kind::FatalFrame2 => {
            let room = &mut p[kind.json_at()..];
            if data.len() > room.len() {
                return Err(format!("the edited data is {} bytes longer than this save has room for", data.len() - room.len()));
            }
            room.fill(0);
            room[..data.len()].copy_from_slice(data);
            wl_sums(kind, &mut p);
            wl_encrypt(&mut p[0x100..]);
        }
        Kind::Nioh | Kind::Nioh3 => {
            if data.len() != p.len() - kind.header() {
                return Err("this save's data can't change size".into());
            }
            p[kind.header()..].copy_from_slice(data);
            nioh_encrypt(kind, &mut p);
        }
        Kind::StrangerOfParadise => {
            if data.len() != p.len() - 0x100 {
                return Err("this save's data can't change size".into());
            }
            p[0x100..].copy_from_slice(data);
            let (k, iv) = sop_key();
            let n = p.len();
            cbc::Encryptor::<aes::Aes128>::new(&k.into(), &iv.into()).encrypt_padded_mut::<NoPadding>(&mut p, n).map_err(|e| e.to_string())?;
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KATANA_SAMPLES = KatanaSaveDataResigner's QualityControl.xUnit/Resources: each encrypted
    /// file decrypts to its decrypted twin (Wo Long's data part) and encrypts back unchanged.
    #[test]
    #[ignore]
    fn test_vectors() {
        let dir = std::path::PathBuf::from(std::env::var("KATANA_SAMPLES").unwrap());
        for name in ["nioh", "nioh2", "nioh3", "sopffo", "wolong-dummy", "ff2cbr-dummy"] {
            let enc = std::fs::read(dir.join(format!("{name}-encrypted.bin"))).unwrap();
            let dec = std::fs::read(dir.join(format!("{name}-decrypted.bin"))).unwrap();
            let (kind, plain) = open(&enc).unwrap_or_else(|| panic!("{name} opens"));
            println!("{name}: {kind:?}");
            // Nioh's tool re-keys on encrypt, so only the data part is compared there.
            assert_eq!(plain[kind.header()..], dec[kind.header()..], "{name} data");
            let c = content(kind, &plain);
            let out = close(kind, &plain, &c).unwrap();
            if matches!(kind, Kind::WoLong | Kind::FatalFrame2) {
                // The test files' checksums and the bytes after their JSON weren't made by the
                // game; what matters is that the JSON comes back.
                let (k2, p2) = open(&out).expect("reopens");
                assert!(k2 == kind && content(k2, &p2) == c, "{name} round trip");
            } else {
                assert!(out == enc, "{name} round trip");
            }
        }
    }

    #[test]
    fn wolong_round_trip() {
        let mut plain = vec![0u8; 0x400];
        plain[..6].copy_from_slice(b"WLNSYS");
        plain[8..16].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        plain[0x100..0x108].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let json = br#"{"a":1}"#;
        plain[0x108..0x108 + json.len()].copy_from_slice(json);
        let file = close(Kind::WoLong, &plain, json).unwrap();
        let (kind, p2) = open(&file).expect("opens");
        assert_eq!(kind, Kind::WoLong);
        assert_eq!(content(kind, &p2), json);
        let edited = close(kind, &p2, br#"{"a":22}"#).unwrap();
        assert_eq!(content(Kind::WoLong, &open(&edited).unwrap().1), br#"{"a":22}"#);
    }
}
