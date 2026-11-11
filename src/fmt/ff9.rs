//! Final Fantasy IX (Steam) `SavedData_ww.dat`: a header, 150 load-screen previews, the
//! autosave and 150 save slots, each encrypted on its own (AES-256-CBC; key and IV from
//! PBKDF2-SHA1 of a fixed password and salt) and stored in a fixed-size space.
//!
//! The encryption facts and the slot layout were checked against the game's own files;
//! field positions inside a slot were matched value by value against a decoded slot.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

pub const FILE_LEN: usize = 320 + 150 * 1024 + 151 * 18432;
const META: (usize, usize) = (288, 320);
const PREVIEW: (usize, usize) = (965, 1024);
const SLOT: (usize, usize) = (17970, 18432);
const PLAIN_LEN: usize = META.0 + 150 * PREVIEW.0 + 151 * SLOT.0;

fn key_iv() -> ([u8; 32], [u8; 16]) {
    let mut kv = [0u8; 48];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(
        b"System.Security.SecureString",
        &[3, 3, 1, 4, 7, 0, 9, 7],
        1000,
        &mut kv,
    );
    let mut k = [0u8; 32];
    let mut iv = [0u8; 16];
    k.copy_from_slice(&kv[..32]);
    iv.copy_from_slice(&kv[32..]);
    (k, iv)
}

/// The sections: (start in the file, plain size, space it takes).
fn sections() -> Vec<(usize, usize, usize)> {
    let mut out = vec![(0, META.0, META.1)];
    let mut at = META.1;
    for _ in 0..150 {
        out.push((at, PREVIEW.0, PREVIEW.1));
        at += PREVIEW.1;
    }
    for _ in 0..151 {
        out.push((at, SLOT.0, SLOT.1));
        at += SLOT.1;
    }
    out
}

fn cipher_len(n: usize) -> usize {
    n + 16 - n % 16
}

/// Decrypts every section into one buffer.
pub fn decrypt(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() != FILE_LEN {
        return None;
    }
    let (k, iv) = key_iv();
    let mut plain = Vec::with_capacity(PLAIN_LEN);
    for (at, n, space) in sections() {
        let c = cipher_len(n);
        // The rest of a section's space is always empty.
        if d[at + c..at + space].iter().any(|&b| b != 0) {
            return None;
        }
        let mut buf = d[at..at + c].to_vec();
        let p = cbc::Decryptor::<aes::Aes256>::new(&k.into(), &iv.into())
            .decrypt_padded_mut::<Pkcs7>(&mut buf)
            .ok()?;
        if p.len() != n {
            return None;
        }
        plain.extend_from_slice(p);
    }
    plain[..4].iter().copied().eq(*b"SAVE").then_some(plain)
}

pub fn encrypt(plain: &[u8]) -> Result<Vec<u8>, String> {
    if plain.len() != PLAIN_LEN {
        return Err("this save's data can't change size".into());
    }
    let (k, iv) = key_iv();
    let mut out = vec![0u8; FILE_LEN];
    let mut p = 0;
    for (at, n, _) in sections() {
        let c = cbc::Encryptor::<aes::Aes256>::new(&k.into(), &iv.into())
            .encrypt_padded_vec_mut::<Pkcs7>(&plain[p..p + n]);
        out[at..at + c.len()].copy_from_slice(&c);
        p += n;
    }
    Ok(out)
}

/// The decrypted data, as `decrypt` makes it.
pub fn looks(plain: &[u8]) -> bool {
    plain.len() == PLAIN_LEN && plain.starts_with(b"SAVE")
}

const GIL: usize = 5235;
const FIRST_NAME: usize = 5808;
const MEMBER: usize = 244;

pub fn parse(plain: &[u8]) -> Result<TreeDoc, String> {
    if !looks(plain) {
        return Err("not Final Fantasy IX data".into());
    }
    let mut b = Builder::new(plain);
    let slots_at = META.0 + 150 * PREVIEW.0;
    let mut used = 0;
    for i in 0..151 {
        let s = slots_at + i * SLOT.0;
        if &plain[s..s + 4] != b"SAVE" {
            continue;
        }
        used += 1;
        let slot = if i == 0 {
            "Autosave".to_string()
        } else {
            format!("Slot {}", (i - 1) / 15 + 1) + &format!(" file {}", (i - 1) % 15 + 1)
        };
        let sl = slot.as_str();
        b.ranged(&[sl, "Gil"], s + GIL, Kind::U32, 0, 9_999_999);
        for m in 0..9 {
            let r = s + FIRST_NAME + m * MEMBER;
            let name: String = plain[r..r + 12]
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c as char)
                .collect();
            if name.is_empty() {
                continue;
            }
            let who = name.as_str();
            let mut f = |k: &str, off: isize, kind: Kind, lo: i64, hi: i64| {
                b.ranged(
                    &[sl, "Party", who, k],
                    (r as isize + off) as usize,
                    kind,
                    lo,
                    hi,
                );
            };
            f("Level", -9, Kind::U8, 1, 99);
            f("Experience", -19, Kind::U32, 0, 9_999_999);
            f("HP", -36, Kind::U16, 0, 9999);
            f("Max HP", -4, Kind::U16, 1, 9999);
            f("MP", -34, Kind::U16, 0, 999);
            f("Max MP", -2, Kind::U16, 0, 999);
            f("Speed", -57, Kind::U8, 1, 50);
            f("Strength", -51, Kind::U8, 1, 99);
            f("Magic", -52, Kind::U8, 1, 99);
            f("Spirit", -50, Kind::U8, 1, 50);
        }
    }
    if used == 0 {
        return Err("no saved games in this file".into());
    }
    Ok(b.finish("Final Fantasy IX save", Checksum::None).with_note(
        "Gil and each character's level, experience, HP/MP and base stats, for every saved game in the file. \
         The load screen keeps its own summary, which updates the next time you save in the game.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_round_trips() {
        // A file made of empty slots ("NONE") with one used slot, written and read back.
        let mut plain = vec![0u8; PLAIN_LEN];
        plain[..4].copy_from_slice(b"SAVE");
        let slots_at = META.0 + 150 * PREVIEW.0;
        for i in 0..151 {
            plain[slots_at + i * SLOT.0..slots_at + i * SLOT.0 + 4].copy_from_slice(b"NONE");
        }
        let s = slots_at + 2 * SLOT.0;
        plain[s..s + 4].copy_from_slice(b"SAVE");
        plain[s + GIL..s + GIL + 4].copy_from_slice(&1234u32.to_le_bytes());
        plain[s + FIRST_NAME..s + FIRST_NAME + 6].copy_from_slice(b"Zidane");
        plain[s + FIRST_NAME - 9] = 12;
        let file = encrypt(&plain).unwrap();
        assert_eq!(file.len(), FILE_LEN);
        assert_eq!(decrypt(&file).unwrap(), plain);
        let mut doc = parse(&plain).unwrap();
        assert_eq!(doc.view.pointer("/Slot 1 file 2/Gil").unwrap(), 1234);
        assert_eq!(
            doc.view
                .pointer("/Slot 1 file 2/Party/Zidane/Level")
                .unwrap(),
            12
        );
        doc.set("/Slot 1 file 2/Gil", serde_json::Value::from(99999))
            .unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(
            u32::from_le_bytes(out[s + GIL..s + GIL + 4].try_into().unwrap()),
            99999
        );
    }

    /// Research aid: SAVE_EDITOR_FF9 = a SavedData_ww.dat.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_FF9").unwrap()).unwrap();
        let plain = decrypt(&d).expect("decrypt");
        assert_eq!(encrypt(&plain).unwrap(), d, "re-encrypting changes nothing");
        let doc = parse(&plain).unwrap();
        println!(
            "{}",
            serde_json::to_string_pretty(&doc.view)
                .unwrap()
                .chars()
                .take(1500)
                .collect::<String>()
        );
    }
}
