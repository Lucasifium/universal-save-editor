//! Final Fantasy Pixel Remaster saves (FF I–VI, PC): a slot file is base64 (with a UTF-8 BOM and a
//! line break) of Rijndael-256-CBC of a raw deflate stream of JSON. Key and IV come from PBKDF2
//! over two fixed strings; the JSON keeps most of its data as more JSON inside strings
//! (`userData`, the characters of `ownedCharacterList`, …), opened here as one tree and made
//! into strings again on save.
//!
//! The cipher parameters are as published in ff_pr_save_tool.py (ReverseTelnet, GPL-3.0) and
//! ffpr-save-cypher (KiameV, MIT). The Rijndael with a 256-bit block is written here from the
//! specification, since AES only has 128-bit blocks.

use super::tld::{as_json, esc, expand, restring};
use base64::Engine;
use serde_json::Value;
use std::io::{Read, Write};

const PASSWORD: &[u8] = b"TKX73OHHK1qMonoICbpVT0hIDGe7SkW0";
const SALT: &[u8] = b"71Ba2p0ULBGaE6oJ7TjCqwsls1jBKmRL";
const BOM: &[u8] = b"\xEF\xBB\xBF";
const NB: usize = 8; // 256-bit block: 8 columns of 4 bytes
const NK: usize = 8; // 256-bit key
const NR: usize = 14;

pub use super::tld::Wrapped;

// ------------------------------------------------------------------ Rijndael-256

fn xtime(a: u8) -> u8 {
    (a << 1) ^ if a & 0x80 != 0 { 0x1B } else { 0 }
}
fn gmul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0;
    while b != 0 {
        if b & 1 != 0 {
            p ^= a;
        }
        a = xtime(a);
        b >>= 1;
    }
    p
}

fn sboxes() -> ([u8; 256], [u8; 256]) {
    let mut s = [0u8; 256];
    let mut inv = [0u8; 256];
    // Multiplicative inverse by search, then the affine map.
    for x in 0..256usize {
        let mut y = 0u8;
        if x != 0 {
            y = (1..=255u8).find(|&c| gmul(x as u8, c) == 1).unwrap();
        }
        let mut b = y;
        let mut r = y;
        for _ in 0..4 {
            b = b.rotate_left(1);
            r ^= b;
        }
        let v = r ^ 0x63;
        s[x] = v;
        inv[v as usize] = x as u8;
    }
    (s, inv)
}

struct Rijndael256 {
    rk: Vec<[u8; 4]>,
    sb: [u8; 256],
    isb: [u8; 256],
}

impl Rijndael256 {
    fn new(key: &[u8; 32]) -> Self {
        let (sb, isb) = sboxes();
        let total = NB * (NR + 1);
        let mut w: Vec<[u8; 4]> = key.chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect();
        let mut rcon = 1u8;
        for i in NK..total {
            let mut t = w[i - 1];
            if i % NK == 0 {
                t = [sb[t[1] as usize] ^ rcon, sb[t[2] as usize], sb[t[3] as usize], sb[t[0] as usize]];
                rcon = xtime(rcon);
            } else if i % NK == 4 {
                t = [sb[t[0] as usize], sb[t[1] as usize], sb[t[2] as usize], sb[t[3] as usize]];
            }
            let p = w[i - NK];
            w.push([p[0] ^ t[0], p[1] ^ t[1], p[2] ^ t[2], p[3] ^ t[3]]);
        }
        Self { rk: w, sb, isb }
    }

    fn add_key(&self, st: &mut [u8; 32], round: usize) {
        for c in 0..NB {
            for r in 0..4 {
                st[c * 4 + r] ^= self.rk[round * NB + c][r];
            }
        }
    }

    const SHIFT: [usize; 4] = [0, 1, 3, 4];

    fn shift_rows(st: &mut [u8; 32], inverse: bool) {
        let mut out = [0u8; 32];
        for c in 0..NB {
            for r in 0..4 {
                let from = if inverse { (c + NB - Self::SHIFT[r]) % NB } else { (c + Self::SHIFT[r]) % NB };
                out[c * 4 + r] = st[from * 4 + r];
            }
        }
        *st = out;
    }

    fn mix(st: &mut [u8; 32], inverse: bool) {
        let m: [u8; 4] = if inverse { [14, 11, 13, 9] } else { [2, 3, 1, 1] };
        for c in 0..NB {
            let col = [st[c * 4], st[c * 4 + 1], st[c * 4 + 2], st[c * 4 + 3]];
            for r in 0..4 {
                let mut v = 0;
                for k in 0..4 {
                    v ^= gmul(col[k], m[(k + 4 - r) % 4]);
                }
                st[c * 4 + r] = v;
            }
        }
    }

    fn encrypt_block(&self, b: &mut [u8; 32]) {
        self.add_key(b, 0);
        for round in 1..=NR {
            for x in b.iter_mut() {
                *x = self.sb[*x as usize];
            }
            Self::shift_rows(b, false);
            if round != NR {
                Self::mix(b, false);
            }
            self.add_key(b, round);
        }
    }

    fn decrypt_block(&self, b: &mut [u8; 32]) {
        self.add_key(b, NR);
        for round in (0..NR).rev() {
            Self::shift_rows(b, true);
            for x in b.iter_mut() {
                *x = self.isb[*x as usize];
            }
            self.add_key(b, round);
            if round != 0 {
                Self::mix(b, true);
            }
        }
    }
}

fn cipher() -> (Rijndael256, [u8; 32]) {
    let mut dk = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(PASSWORD, SALT, 10, &mut dk);
    let key: [u8; 32] = dk[..32].try_into().unwrap();
    let iv: [u8; 32] = dk[32..].try_into().unwrap();
    (Rijndael256::new(&key), iv)
}

fn cbc_decrypt(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() || data.len() % 32 != 0 {
        return None;
    }
    let (c, mut prev) = cipher();
    let mut out = Vec::with_capacity(data.len());
    for blk in data.chunks(32) {
        let mut b: [u8; 32] = blk.try_into().unwrap();
        c.decrypt_block(&mut b);
        for i in 0..32 {
            b[i] ^= prev[i];
        }
        prev = blk.try_into().unwrap();
        out.extend_from_slice(&b);
    }
    Some(out)
}

fn cbc_encrypt(data: &[u8]) -> Vec<u8> {
    let (c, mut prev) = cipher();
    let mut padded = data.to_vec();
    while padded.len() % 32 != 0 {
        padded.push(0);
    }
    let mut out = Vec::with_capacity(padded.len());
    for blk in padded.chunks(32) {
        let mut b: [u8; 32] = blk.try_into().unwrap();
        for i in 0..32 {
            b[i] ^= prev[i];
        }
        c.encrypt_block(&mut b);
        prev = b;
        out.extend_from_slice(&b);
    }
    out
}

// ------------------------------------------------------------------ the save

fn plausible(d: &[u8]) -> bool {
    let t = d.strip_prefix(BOM).unwrap_or(d);
    t.len() >= 64 && t.len() < (64 << 20) && t.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'\r' | b'\n'))
}

/// Unpacks a PC slot file into one JSON text.
pub fn open(d: &[u8]) -> Option<(Wrapped, Vec<u8>)> {
    if !plausible(d) {
        return None;
    }
    let t = d.strip_prefix(BOM).unwrap_or(d);
    let b64: Vec<u8> = t.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
    let enc = base64::engine::general_purpose::STANDARD.decode(&b64).ok()?;
    let packed = cbc_decrypt(&enc)?;
    let mut json = Vec::new();
    flate2::read::DeflateDecoder::new(&packed[..]).read_to_end(&mut json).ok()?;
    let text = String::from_utf8(json).ok()?;
    let mut root: Value = serde_json::from_str(&text).ok()?;
    if !root.get("userData").is_some_and(Value::is_string) {
        return None;
    }
    let mut ptrs: Vec<(String, bool)> = vec![];
    // Each top-level string that is JSON is opened, with the strings inside it.
    let keys: Vec<String> = root.as_object()?.keys().cloned().collect();
    for k in keys {
        let p = format!("/{}", esc(&k));
        let slot = root.pointer_mut(&p)?;
        if let Value::String(s) = slot {
            if let Some(mut inner) = as_json(s) {
                expand(&mut inner, &p, &mut ptrs);
                ptrs.push((p, false));
                *slot = inner;
            }
        }
    }
    Some((Wrapped(ptrs), serde_json::to_vec_pretty(&root).ok()?))
}

/// Makes the strings again, packs, encrypts and encodes.
pub fn pack(w: &Wrapped, bom: bool, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut root: Value = serde_json::from_slice(data).map_err(|e| format!("The save is no longer valid JSON: {e}"))?;
    restring(&mut root, &w.0)?;
    let text = serde_json::to_vec(&root).map_err(|e| e.to_string())?;
    let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&text).map_err(|e| e.to_string())?;
    let packed = z.finish().map_err(|e| e.to_string())?;
    let mut out = if bom { BOM.to_vec() } else { vec![] };
    out.extend_from_slice(base64::engine::general_purpose::STANDARD.encode(cbc_encrypt(&packed)).as_bytes());
    out.extend_from_slice(b"\r\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cipher_round_trips() {
        let data: Vec<u8> = (0..200u32).map(|i| (i * 7 % 251) as u8).collect();
        let enc = cbc_encrypt(&data);
        assert_eq!(enc.len() % 32, 0);
        let dec = cbc_decrypt(&enc).unwrap();
        assert_eq!(&dec[..200], &data[..]);
    }

    #[test]
    fn sbox_is_aes() {
        let (s, i) = sboxes();
        assert_eq!((s[0], s[1], s[0x53], s[255]), (0x63, 0x7c, 0xed, 0x16));
        assert_eq!(i[0x63], 0);
    }

    #[test]
    fn real_saves_open() {
        let Ok(dir) = std::env::var("FFPR_SAMPLES") else { return };
        for g in ["ff1_pr", "ff2_pr", "ff3_pr", "ff4_pr"] {
            let enc = std::fs::read(format!("{dir}/{g}/encrypted/Rl18osV3e9kPX9SMWQj8mqShFpTUmu1lf6Mb=FVVfqk=")).unwrap();
            let want: Value = serde_json::from_slice(&std::fs::read(format!("{dir}/{g}/decrypted/slot21.json")).unwrap()).unwrap();
            let (w, json) = open(&enc).unwrap_or_else(|| panic!("{g} opens"));
            let got: Value = serde_json::from_slice(&json).unwrap();
            assert!(got.pointer("/userData/owendGil").is_some(), "{g}");
            let again = pack(&w, enc.starts_with(BOM), &json).unwrap();
            let (_, back) = open(&again).unwrap();
            let back: Value = serde_json::from_slice(&back).unwrap();
            assert_eq!(back, got, "{g} round trip");
            // The reference tool's own output is the same data as ours, once its strings are made again.
            let mut re = got.clone();
            restring(&mut re, &w.0).unwrap();
            assert_eq!(re, want, "{g} matches the reference decryption");
        }
    }
}
