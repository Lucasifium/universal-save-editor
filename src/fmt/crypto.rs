//! Encrypted save wrappers.
//!
//! * Easy Save 3 (Unity): `IV(16) || AES-128-CBC(PKCS7)`, key = PBKDF2-HMAC-SHA1(password,
//!   salt = IV, 100 iterations, 16 bytes). Many games keep ES3's default password "password".
//! * Godot `open_encrypted_with_pass`: "GDEC" magic, mode, MD5 of the plaintext, length,
//!   [IV in Godot 4], then AES-256 data (CFB in Godot 4, ECB in Godot 3) zero-padded to
//!   16 bytes. Key = the 32 hex characters of md5(password). The stored MD5 lets us
//!   confirm a password is right.

use aes::cipher::{
    block_padding::Pkcs7, AsyncStreamCipher, BlockDecrypt, BlockDecryptMut, BlockEncrypt,
    BlockEncryptMut, KeyInit, KeyIvInit,
};
use md5::{Digest, Md5};

pub const ES3_DEFAULT_PASSWORD: &str = "password";

/// Passwords of games known to use Easy Save 3 encryption with their own password, as
/// published by those games' open-source save editors (Phasmophobia, Lethal Company, R.E.P.O.).
pub const ES3_GAME_PASSWORDS: &[&str] = &[
    "t36gref9u84y7f43g",
    "lcslime14a5",
    "Why would you want to cheat?... :o It's no fun. :') :'D",
];

fn es3_key(password: &str, iv: &[u8]) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(password.as_bytes(), iv, 100, &mut key);
    key
}

pub fn es3_decrypt(data: &[u8], password: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    if data.len() < 32 || (data.len() - 16) % 16 != 0 {
        return None;
    }
    let (iv, body) = data.split_at(16);
    let key = es3_key(password, iv);
    let plain = cbc::Decryptor::<aes::Aes128>::new(&key.into(), iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(body)
        .ok()?;
    Some((iv.to_vec(), plain))
}

/// Try the player's password (if any), ES3's default and the known games' passwords; accept only a result that
/// `accept` recognises (JSON, gzip, …) so random data is never "decrypted".
pub fn es3_decrypt_any(
    data: &[u8],
    password: Option<&str>,
    accept: fn(&[u8]) -> bool,
) -> Option<(String, Vec<u8>, Vec<u8>)> {
    if std::str::from_utf8(data).is_ok() {
        return None;
    }
    let mut candidates: Vec<&str> = password.into_iter().collect();
    for pw in std::iter::once(&ES3_DEFAULT_PASSWORD).chain(ES3_GAME_PASSWORDS) {
        if !candidates.contains(pw) {
            candidates.push(pw);
        }
    }
    for pw in candidates {
        if let Some((iv, plain)) = es3_decrypt(data, pw) {
            if accept(&plain) {
                return Some((pw.to_string(), iv, plain));
            }
        }
    }
    None
}

pub fn es3_encrypt(plain: &[u8], password: &str, iv: &[u8]) -> Result<Vec<u8>, String> {
    let key = es3_key(password, iv);
    let iv16: [u8; 16] = iv.try_into().map_err(|_| "bad IV")?;
    let body = cbc::Encryptor::<aes::Aes128>::new(&key.into(), &iv16.into())
        .encrypt_padded_vec_mut::<Pkcs7>(plain);
    Ok([iv, &body].concat())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GodotEnc {
    pub password: String,
    /// Godot 4 stores an IV (CFB mode); Godot 3 has none (ECB mode).
    pub iv: Option<Vec<u8>>,
    pub mode: u32,
}

fn godot_key(password: &str) -> [u8; 32] {
    let hex: String = Md5::digest(password.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    hex.as_bytes().try_into().unwrap()
}

fn padded(len: usize) -> usize {
    len.div_ceil(16) * 16
}

pub fn godot_decrypt(data: &[u8], password: Option<&str>) -> Option<(GodotEnc, Vec<u8>)> {
    let password = password?;
    if !data.starts_with(b"GDEC") || data.len() < 32 {
        return None;
    }
    let mode = u32::from_le_bytes(data[4..8].try_into().ok()?);
    let md5: [u8; 16] = data[8..24].try_into().ok()?;
    let length = u64::from_le_bytes(data[24..32].try_into().ok()?) as usize;
    let key = godot_key(password);
    // Godot 4: IV then CFB data.
    if let Some(iv) = data.get(32..48) {
        if let Some(enc) = data.get(48..48 + padded(length)) {
            let mut buf = enc.to_vec();
            cfb_mode::Decryptor::<aes::Aes256>::new(&key.into(), iv.into()).decrypt(&mut buf);
            buf.truncate(length);
            if Md5::digest(&buf).as_slice() == md5 {
                return Some((
                    GodotEnc {
                        password: password.into(),
                        iv: Some(iv.to_vec()),
                        mode,
                    },
                    buf,
                ));
            }
        }
    }
    // Godot 3: ECB data straight after the header.
    if let Some(enc) = data.get(32..32 + padded(length)) {
        let cipher = aes::Aes256::new(&key.into());
        let mut buf = enc.to_vec();
        for block in buf.chunks_mut(16) {
            cipher.decrypt_block(block.into());
        }
        buf.truncate(length);
        if Md5::digest(&buf).as_slice() == md5 {
            return Some((
                GodotEnc {
                    password: password.into(),
                    iv: None,
                    mode,
                },
                buf,
            ));
        }
    }
    None
}

pub fn godot_encrypt(plain: &[u8], info: &GodotEnc) -> Result<Vec<u8>, String> {
    let key = godot_key(&info.password);
    let mut out = b"GDEC".to_vec();
    out.extend_from_slice(&info.mode.to_le_bytes());
    out.extend_from_slice(&Md5::digest(plain));
    out.extend_from_slice(&(plain.len() as u64).to_le_bytes());
    let mut buf = plain.to_vec();
    buf.resize(padded(plain.len()), 0);
    match &info.iv {
        Some(iv) => {
            out.extend_from_slice(iv);
            let iv16: [u8; 16] = iv.as_slice().try_into().map_err(|_| "bad IV")?;
            cfb_mode::Encryptor::<aes::Aes256>::new(&key.into(), &iv16.into()).encrypt(&mut buf);
        }
        None => {
            let cipher = aes::Aes256::new(&key.into());
            for block in buf.chunks_mut(16) {
                cipher.encrypt_block(block.into());
            }
        }
    }
    out.extend_from_slice(&buf);
    Ok(out)
}

/// High-entropy data with no recognisable header: probably encrypted.
pub fn looks_encrypted(d: &[u8]) -> bool {
    if d.len() < 64 {
        return false;
    }
    let mut counts = [0usize; 256];
    for &b in d {
        counts[b as usize] += 1;
    }
    let n = d.len() as f64;
    let entropy: f64 = counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum();
    entropy > 7.2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn es3_round_trip() {
        let plain = br#"{"gold":{"__type":"int","value":1500}}"#;
        let iv = [7u8; 16];
        let enc = es3_encrypt(plain, ES3_DEFAULT_PASSWORD, &iv).unwrap();
        let (got_iv, got) = es3_decrypt(&enc, ES3_DEFAULT_PASSWORD).unwrap();
        assert_eq!((got_iv.as_slice(), got.as_slice()), (&iv[..], &plain[..]));
        assert!(es3_decrypt(&enc, "wrong").is_none_or(|(_, p)| p != plain));
    }

    #[test]
    fn godot_round_trip_both_versions() {
        let plain = b"{\"level\": 3, \"name\": \"Ash\"}";
        for iv in [Some(vec![9u8; 16]), None] {
            let info = GodotEnc {
                password: "secret".into(),
                iv,
                mode: 1,
            };
            let enc = godot_encrypt(plain, &info).unwrap();
            let (got_info, got) = godot_decrypt(&enc, Some("secret")).unwrap();
            assert_eq!(got, plain);
            assert_eq!(got_info, info);
            assert!(
                godot_decrypt(&enc, Some("nope")).is_none(),
                "wrong password must be rejected via MD5"
            );
        }
    }
}

// ------------------------------------------------------------------ fixed game keys

/// Terraria `.plr`: AES-128-CBC with key = IV = "h3y_gUyZ" as UTF-16LE.
fn terraria_key() -> [u8; 16] {
    let mut k = [0u8; 16];
    for (i, u) in "h3y_gUyZ".encode_utf16().enumerate() {
        k[i * 2..i * 2 + 2].copy_from_slice(&u.to_le_bytes());
    }
    k
}

pub fn terraria_decrypt(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 32 || data.len() % 16 != 0 {
        return None;
    }
    let k = terraria_key();
    let plain = cbc::Decryptor::<aes::Aes128>::new(&k.into(), &k.into())
        .decrypt_padded_vec_mut::<Pkcs7>(data)
        .ok()?;
    // Version, then the "relogic" file signature with file type 3 (player).
    (plain.len() > 12 && &plain[4..11] == b"relogic" && plain[11] == 3).then_some(plain)
}

pub fn terraria_encrypt(plain: &[u8]) -> Vec<u8> {
    let k = terraria_key();
    cbc::Encryptor::<aes::Aes128>::new(&k.into(), &k.into()).encrypt_padded_vec_mut::<Pkcs7>(plain)
}

/// Hollow Knight `user*.dat`: a .NET BinaryFormatter string holding base64 of
/// AES-256-ECB(PKCS7) JSON with a fixed key.
const HK_KEY: &[u8; 32] = b"UKu52ePUBwetZ9wNX88o54dnfKRu0T1l";
const HK_HEADER: [u8; 22] = [
    0, 1, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 1, 0, 0, 0, 0, 0, 0, 0, 6, 1, 0, 0, 0,
];

pub fn hollow_knight_decrypt(data: &[u8]) -> Option<Vec<u8>> {
    let rest = data.strip_prefix(&HK_HEADER[..])?;
    let (mut len, mut shift, mut i) = (0usize, 0, 0);
    loop {
        let b = *rest.get(i)?;
        len |= ((b & 0x7F) as usize) << shift;
        i += 1;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift > 28 {
            return None;
        }
    }
    let text = rest.get(i..i + len)?;
    if rest.get(i + len..) != Some(&[0x0B][..]) {
        return None;
    }
    use base64::Engine;
    let mut buf = base64::engine::general_purpose::STANDARD
        .decode(text)
        .ok()?;
    if buf.is_empty() || buf.len() % 16 != 0 {
        return None;
    }
    let cipher = aes::Aes256::new(HK_KEY.into());
    for block in buf.chunks_exact_mut(16) {
        cipher.decrypt_block(block.into());
    }
    let pad = *buf.last()? as usize;
    if !(1..=16).contains(&pad) || buf[buf.len() - pad..].iter().any(|&b| b as usize != pad) {
        return None;
    }
    buf.truncate(buf.len() - pad);
    Some(buf)
}

pub fn hollow_knight_encrypt(plain: &[u8]) -> Vec<u8> {
    let pad = 16 - plain.len() % 16;
    let mut buf = plain.to_vec();
    buf.extend(std::iter::repeat(pad as u8).take(pad));
    let cipher = aes::Aes256::new(HK_KEY.into());
    for block in buf.chunks_exact_mut(16) {
        cipher.encrypt_block(block.into());
    }
    use base64::Engine;
    let text = base64::engine::general_purpose::STANDARD.encode(&buf);
    let mut out = HK_HEADER.to_vec();
    let mut n = text.len();
    while n >= 0x80 {
        out.push((n as u8 & 0x7F) | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(text.as_bytes());
    out.push(0x0B);
    out
}

/// Fallout Shelter `Vault<n>.sav`: base64 of AES-256-CBC (PKCS7, fixed key and IV) JSON, as
/// published by the game's open-source save editors.
const FS_KEY: [u8; 32] = [
    0xa7, 0xca, 0x9f, 0x33, 0x66, 0xd8, 0x92, 0xc2, 0xf0, 0xbe, 0xf4, 0x17, 0x34, 0x1c, 0xa9, 0x71,
    0xb6, 0x9a, 0xe9, 0xf7, 0xba, 0xcc, 0xcf, 0xfc, 0xf4, 0x3c, 0x62, 0xd1, 0xd7, 0xd0, 0x21, 0xf9,
];
const FS_IV: &[u8; 16] = b"tu89geji340t89u2";

pub fn fallout_shelter_decrypt(data: &[u8]) -> Option<Vec<u8>> {
    use base64::Engine;
    let text = std::str::from_utf8(data).ok()?.trim();
    if text.len() < 24
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
    {
        return None;
    }
    let raw = base64::engine::general_purpose::STANDARD
        .decode(text)
        .ok()?;
    if raw.is_empty() || raw.len() % 16 != 0 {
        return None;
    }
    let plain = cbc::Decryptor::<aes::Aes256>::new(&FS_KEY.into(), FS_IV.into())
        .decrypt_padded_vec_mut::<Pkcs7>(&raw)
        .ok()?;
    plain.first().is_some_and(|&b| b == b'{').then_some(plain)
}

pub fn fallout_shelter_encrypt(plain: &[u8]) -> Vec<u8> {
    use base64::Engine;
    let body = cbc::Encryptor::<aes::Aes256>::new(&FS_KEY.into(), FS_IV.into())
        .encrypt_padded_vec_mut::<Pkcs7>(plain);
    base64::engine::general_purpose::STANDARD
        .encode(body)
        .into_bytes()
}
