//! Ren'Py 8.1+ save signing ("save tokens").
//!
//! Ren'Py signs every save with a key stored on this PC in
//! `%APPDATA%\RenPy\tokens\security_keys.txt`. When it loads a save whose
//! signature doesn't match the data, it asks the player, and then *still refuses*
//! if the save carries a (now wrong) signature. So after editing we re-sign with
//! the player's own key — exactly what the game itself does when saving — which
//! lets the game load the edited save normally.
//!
//! Format (from renpy/savetoken.py): lines of `kind base64 [base64]`.
//!   signing-key   <SEC1 DER private key> <SubjectPublicKeyInfo DER public key>
//!   signature     <public key DER>       <64-byte r||s signature>
//! Signatures are python-ecdsa defaults: NIST P-256 over the SHA-1 digest.

use base64::{engine::general_purpose::STANDARD, Engine};
use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::ecdsa::{Signature, SigningKey};
use sha1::{Digest, Sha1};
use std::path::PathBuf;

/// Where Ren'Py keeps the keys. `SAVE_EDITOR_RENPY_KEYS` overrides it (used by tests).
pub fn keys_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SAVE_EDITOR_RENPY_KEYS") {
        return Some(PathBuf::from(p));
    }
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("RenPy").join("tokens").join("security_keys.txt"))
}

struct Key {
    signing: SigningKey,
    public_der: Vec<u8>,
}

fn load_keys() -> Vec<Key> {
    let Some(text) = keys_path().and_then(|p| std::fs::read_to_string(p).ok()) else { return vec![] };
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next()? != "signing-key" {
                return None;
            }
            let private = STANDARD.decode(parts.next()?).ok()?;
            let public_der = STANDARD.decode(parts.next()?).ok()?;
            let secret = p256::SecretKey::from_sec1_der(&private).ok()?;
            Some(Key { signing: SigningKey::from(secret), public_der })
        })
        .collect()
}

/// True if this PC has a Ren'Py signing key we can use.
pub fn can_sign() -> bool {
    !load_keys().is_empty()
}

/// Produce the contents of a `signatures` entry for `data`, like Ren'Py's
/// `sign_data`. Returns None if no signing key is available.
pub fn sign(data: &[u8]) -> Option<String> {
    let keys = load_keys();
    if keys.is_empty() {
        return None;
    }
    let digest = Sha1::digest(data);
    let mut out = String::new();
    for k in keys {
        let sig: Signature = k.signing.sign_prehash(&digest).ok()?;
        out.push_str(&format!("signature {} {}\n", STANDARD.encode(&k.public_der), STANDARD.encode(sig.to_bytes())));
    }
    Some(out)
}

/// Does `signatures` contain a valid signature of `data`? (Any key.)
#[cfg(test)]
pub fn verify(data: &[u8], signatures: &str) -> bool {
    use p256::ecdsa::signature::hazmat::PrehashVerifier;
    use p256::ecdsa::VerifyingKey;
    use p256::pkcs8::DecodePublicKey;
    let digest = Sha1::digest(data);
    signatures.lines().any(|line| {
        let mut parts = line.split_whitespace();
        if parts.next() != Some("signature") {
            return false;
        }
        let (Some(k), Some(s)) = (parts.next(), parts.next()) else { return false };
        let (Ok(k), Ok(s)) = (STANDARD.decode(k), STANDARD.decode(s)) else { return false };
        let (Ok(vk), Ok(sig)) = (VerifyingKey::from_public_key_der(&k), Signature::from_slice(&s)) else { return false };
        vk.verify_prehash(&digest, &sig).is_ok()
    })
}

/// Write a fresh test key file in Ren'Py's format.
#[cfg(test)]
pub fn write_test_keys(path: &std::path::Path) {
    use p256::pkcs8::EncodePublicKey;
    let secret = p256::SecretKey::random(&mut rand_core::OsRng);
    let private = secret.to_sec1_der().unwrap();
    let public = secret.public_key().to_public_key_der().unwrap();
    let line = format!("signing-key {} {}\n", STANDARD.encode(&*private), STANDARD.encode(public.as_bytes()));
    std::fs::write(path, line).unwrap();
}
