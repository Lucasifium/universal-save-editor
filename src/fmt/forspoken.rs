//! Forspoken `auto.save` / `slotNN.save`. The outer layer is the one Final Fantasy XV uses
//! (see `ffxv`) with Forspoken's own AES key; inside, a 24-byte header, a 24-byte checksum
//! block (the MD5 of the payload in its first 16 bytes) and the payload (its size is the u64
//! at 40). The layer's key and the layout of the save information come from LuminousSaveCrypt
//! by Rinual (GPL-3.0), after Surihix's FFXVSaveCrypt.

use super::fields::Builder;
use super::fields::Checksum;
use crate::tree::TreeDoc;
use md5::{Digest, Md5};

const KEY: [u8; 24] = [
    0xEE, 0xD1, 0x9D, 0xE5, 0x59, 0x07, 0xDA, 0xC3, 0x64, 0x94, 0x66, 0xB4, 0x31, 0x0E, 0xE4, 0x79, 0x29, 0xB9, 0x3F, 0xFA, 0x09, 0x9D, 0x4F, 0x9E,
];

fn payload(d: &[u8]) -> Option<&[u8]> {
    let size = u64::from_le_bytes(d.get(40..48)?.try_into().ok()?) as usize;
    d.get(48..48usize.checked_add(size)?)
}

/// Whether the decrypted data is a Forspoken save: its stored MD5 matches the payload.
pub fn looks(d: &[u8]) -> bool {
    payload(d).is_some_and(|p| !p.is_empty() && Md5::digest(p)[..] == d[24..40])
}

pub fn decrypt(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let (plain, foot) = super::ffxv::decrypt_with(&KEY, d)?;
    looks(&plain).then_some((plain, foot))
}

pub fn encrypt(plain: &[u8], foot: &[u8]) -> Result<Vec<u8>, String> {
    let mut d = plain.to_vec();
    let p = payload(&d).ok_or("the Forspoken save lost its payload")?;
    let sum = Md5::digest(p);
    d[24..40].copy_from_slice(&sum);
    super::ffxv::encrypt_with(&KEY, &d, foot)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Forspoken save".into());
    }
    let u32_at = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let mut b = Builder::new(d);
    // Save information: structure id and version, the thumbnail (a PNG), the position and
    // camera vectors, the root package path, then the slot, the play time and the chapter.
    let info = (|| {
        let mut o = 48 + 12;
        o += 4 + u32_at(o)?;
        o += 32 + 16 + 16 + 32;
        o += 4 + u32_at(o)?;
        let slot = u32_at(o)? as i32;
        let secs = f64::from_le_bytes(d.get(o + 4..o + 12)?.try_into().ok()?);
        let chapter = u32_at(o + 12 + 8 + 4 + 4)? as i32;
        Some((slot, secs, chapter))
    })();
    if let Some((slot, secs, chapter)) = info {
        b.info(&["Save", "Slot"], slot);
        b.info(&["Save", "Chapter"], chapter);
        b.info(&["Save", "Play time (hours)"], (secs / 360.0).round() / 10.0);
    }
    Ok(b.finish("Forspoken save", Checksum::None).with_note(
        "Decrypted for you; the save's MD5 is worked out again and it is encrypted again on save. Edit values with Value search.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_refreshes_the_md5() {
        let mut plain = vec![0u8; 48 + 160];
        plain[40..48].copy_from_slice(&160u64.to_le_bytes());
        for (i, b) in plain[48..].iter_mut().enumerate() {
            *b = i as u8;
        }
        let sum = Md5::digest(&plain[48..]);
        plain[24..40].copy_from_slice(&sum);
        let mut foot = vec![0u8; 53];
        for (i, b) in foot[..36].iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
        foot[52] = 2;
        let enc = encrypt(&plain, &foot).unwrap();
        let (dec, f2) = decrypt(&enc).unwrap();
        assert_eq!(dec, plain);
        assert_eq!(f2, foot);
        // An edited payload gets its MD5 back on save.
        let mut edited = dec.clone();
        edited[100] ^= 0xFF;
        let again = decrypt(&encrypt(&edited, &foot).unwrap()).unwrap().0;
        assert_eq!(&again[48..], &edited[48..]);
        assert!(looks(&again));
    }
}
