//! Games that encrypt a JSON save with a fixed key built into the game.
//!
//! - Megabonk (`progression.json`, `stats.json`): base64 of AES-256-CBC (the base64 is its own
//!   layer). Key and IV as published by Megabonk-Save-Decryptor (GuardianN06), up to version
//!   1.0.17.
//! - Eiyuden Chronicle: Hundred Heroes (`UserDataN.dat`): Triple-DES-CBC, the key and IV as
//!   published by Eiyuden-Chronicle-Save-Editor (TheSparda).
//! - Suikoden I & II HD Remaster: `GR_DATA:` then base64 of a 16-byte salt and AES-256-CBC, the
//!   key and IV made from the salt and a password with SHA-256 (as SuikodenSaveDecrypter by
//!   d3xMachina, MIT, does it). The salt is kept, so an unchanged save comes back the same.
//!
//! A cipher is only used when it gives back JSON, so other files are never taken for these.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::Engine;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wrap {
    Megabonk,
    Eiyuden,
    /// The salt in front of the data.
    Suikoden(Vec<u8>),
}

impl Wrap {
    pub fn label(&self) -> &'static str {
        match self {
            Wrap::Megabonk => "Megabonk save encryption (AES)",
            Wrap::Eiyuden => "Eiyuden Chronicle save encryption (Triple-DES)",
            Wrap::Suikoden(_) => "Suikoden I & II HD save encryption (AES)",
        }
    }
    pub fn game(&self) -> &'static str {
        match self {
            Wrap::Megabonk => "Megabonk",
            Wrap::Eiyuden => "Eiyuden Chronicle: Hundred Heroes",
            Wrap::Suikoden(_) => "Suikoden I & II HD Remaster",
        }
    }
}

const MEGABONK_KEY: [u8; 32] = hex32("D940840D5AE7C7907B092437BC0C5B44AAF70E273E12D0FB4DA2B8C767CC911D");
const MEGABONK_IV: [u8; 16] = hex16("37864EF15C24BC0ACBC60E3978EF1F06");
const EIYUDEN_KEY: [u8; 24] = hex24("b3ba76ead29507bad9e68bab87b6e920fe5193bdce92a870");
const EIYUDEN_IV: [u8; 8] = hex8("2f6e9693c9779505");
const SUIKODEN_PASSWORD: &[u8] = b"auDotXSC3fCBCwQk1nQ3aT7Xe1Vk3BmG";
const SUIKODEN_HEAD: &[u8] = b"GR_DATA:";

const fn nib(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => c - b'A' + 10,
    }
}
const fn hexn<const N: usize>(s: &str) -> [u8; N] {
    let b = s.as_bytes();
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = nib(b[i * 2]) << 4 | nib(b[i * 2 + 1]);
        i += 1;
    }
    out
}
const fn hex32(s: &str) -> [u8; 32] {
    hexn::<32>(s)
}
const fn hex24(s: &str) -> [u8; 24] {
    hexn::<24>(s)
}
const fn hex16(s: &str) -> [u8; 16] {
    hexn::<16>(s)
}
const fn hex8(s: &str) -> [u8; 8] {
    hexn::<8>(s)
}

fn looks_json(d: &[u8]) -> bool {
    let d = d.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(d);
    let t = d.iter().position(|b| !b.is_ascii_whitespace()).map(|i| &d[i..]).unwrap_or(&[]);
    (t.starts_with(b"{") || t.starts_with(b"[")) && std::str::from_utf8(d).is_ok()
}

/// Suikoden's key and IV: SHA-256 blocks of (previous block, password, salt), as OpenSSL's
/// EVP_BytesToKey makes them.
fn suikoden_key(salt: &[u8]) -> ([u8; 32], [u8; 16]) {
    let mut out = Vec::with_capacity(64);
    let mut prev: Vec<u8> = vec![];
    while out.len() < 48 {
        let h = Sha256::new().chain_update(&prev).chain_update(SUIKODEN_PASSWORD).chain_update(salt).finalize();
        prev = h.to_vec();
        out.extend_from_slice(&h);
    }
    (out[..32].try_into().unwrap(), out[32..48].try_into().unwrap())
}

fn aes256_dec(key: &[u8; 32], iv: &[u8; 16], d: &[u8]) -> Option<Vec<u8>> {
    cbc::Decryptor::<aes::Aes256>::new(key.into(), iv.into()).decrypt_padded_vec_mut::<Pkcs7>(d).ok()
}
fn aes256_enc(key: &[u8; 32], iv: &[u8; 16], d: &[u8]) -> Vec<u8> {
    cbc::Encryptor::<aes::Aes256>::new(key.into(), iv.into()).encrypt_padded_vec_mut::<Pkcs7>(d)
}

pub fn open(d: &[u8]) -> Option<(Wrap, Vec<u8>)> {
    if let Some(rest) = d.strip_prefix(SUIKODEN_HEAD) {
        let text: Vec<u8> = rest.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect();
        let raw = base64::engine::general_purpose::STANDARD.decode(text).ok()?;
        if raw.len() < 32 || (raw.len() - 16) % 16 != 0 {
            return None;
        }
        let (key, iv) = suikoden_key(&raw[..16]);
        let plain = aes256_dec(&key, &iv, &raw[16..])?;
        return looks_json(&plain).then(|| (Wrap::Suikoden(raw[..16].to_vec()), plain));
    }
    if d.len() >= 16 && d.len() % 16 == 0 {
        if let Some(p) = aes256_dec(&MEGABONK_KEY, &MEGABONK_IV, d).filter(|p| looks_json(p)) {
            return Some((Wrap::Megabonk, p));
        }
    }
    if d.len() >= 8 && d.len() % 8 == 0 {
        let p = cbc::Decryptor::<des::TdesEde3>::new(&EIYUDEN_KEY.into(), &EIYUDEN_IV.into()).decrypt_padded_vec_mut::<Pkcs7>(d).ok();
        if let Some(p) = p.filter(|p| looks_json(p)) {
            return Some((Wrap::Eiyuden, p));
        }
    }
    None
}

pub fn close(w: &Wrap, plain: &[u8]) -> Vec<u8> {
    match w {
        Wrap::Megabonk => aes256_enc(&MEGABONK_KEY, &MEGABONK_IV, plain),
        Wrap::Eiyuden => cbc::Encryptor::<des::TdesEde3>::new(&EIYUDEN_KEY.into(), &EIYUDEN_IV.into()).encrypt_padded_vec_mut::<Pkcs7>(plain),
        Wrap::Suikoden(salt) => {
            let (key, iv) = suikoden_key(salt);
            let mut raw = salt.clone();
            raw.extend(aes256_enc(&key, &iv, plain));
            [SUIKODEN_HEAD, base64::engine::general_purpose::STANDARD.encode(raw).as_bytes()].concat()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let json = br#"{"gold": 123, "name": "x"}"#;
        for w in [Wrap::Megabonk, Wrap::Eiyuden, Wrap::Suikoden(vec![7; 16])] {
            let enc = close(&w, json);
            let (w2, plain) = open(&enc).unwrap();
            assert_eq!(w2, w);
            assert_eq!(plain, json);
            assert_eq!(close(&w2, &plain), enc);
        }
        assert!(open(&[0u8; 64]).is_none());
    }

    #[test]
    fn suikoden_key_matches_openssl_layout() {
        // D1 = SHA-256(password || salt); the key is D1, the IV the first half of D2.
        let salt = [1u8; 16];
        let d1 = Sha256::new().chain_update(SUIKODEN_PASSWORD).chain_update(salt).finalize();
        let d2 = Sha256::new().chain_update(d1).chain_update(SUIKODEN_PASSWORD).chain_update(salt).finalize();
        let (k, iv) = suikoden_key(&salt);
        assert_eq!(&k[..], &d1[..]);
        assert_eq!(&iv[..], &d2[..16]);
    }
}
