//! Trails to Azure (NISA, PC) and Ao no Kiseki KAI (Clouded Leopard, PC) saves.
//!
//! The NISA file is one zstd frame holding a 0x2643C-byte save (or that save as it is); the
//! last eight bytes are two 32-bit checksums. The KAI file is one AES-128-CBC stream (PKCS7)
//! over a length and a 0x27448-byte payload; it is converted to the NISA layout in memory
//! and back on save, keeping the bytes only KAI has from the original file.
//!
//! Offsets and the checksum follow the BZH Ao no Kiseki save editor by 424778940z (GPL-3.0),
//! in the form adapted for these two releases by lmaple0's ao-save-editor (facts only).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use sha2::{Digest, Sha256};

pub const SIZE: usize = 0x2643C;
pub const CLE_SIZE: usize = 0x27448;
const SCENE: (usize, usize) = (0x4B64, 0x4B68);
const ACTORS: (usize, usize) = (0x4C04, 0x4C10);
const ACTOR_SIZE: (usize, usize) = (0x2A0, 0x2C0);
const THREAD_SIZE: (usize, usize) = (0xA0, 0xA8);
const TAIL: (usize, usize) = (0x19C04, 0x1AC10);

fn sums(d: &[u8]) -> (u32, u32) {
    let n = (d.len() - 8) / 4;
    let total = d[..n * 4].chunks_exact(4).fold(0u32, |a, w| a.wrapping_add(u32::from_le_bytes([w[0], w[1], w[2], w[3]])));
    (total, 0u32.wrapping_sub(n as u32).wrapping_sub(total))
}

pub fn fix_sums(d: &mut Vec<u8>) {
    if d.len() == SIZE {
        let (a, b) = sums(d);
        let n = d.len();
        d[n - 8..n - 4].copy_from_slice(&a.to_le_bytes());
        d[n - 4..].copy_from_slice(&b.to_le_bytes());
    }
}

/// A NISA-layout save with its checksums right.
pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && {
        let (a, b) = sums(d);
        d[SIZE - 8..SIZE - 4] == a.to_le_bytes() && d[SIZE - 4..] == b.to_le_bytes()
    }
}

fn digest() -> [u8; 32] {
    let mut d = b"5e84w@ypz p@kgpg te".to_vec();
    for _ in 0..3 {
        d = Sha256::digest(&d).to_vec();
    }
    d.try_into().unwrap()
}

fn threads() -> impl Iterator<Item = (usize, usize)> {
    std::iter::once(SCENE).chain((0..128).flat_map(|a| {
        (0..4).map(move |t| (ACTORS.0 + a * ACTOR_SIZE.0 + 0x20 + t * THREAD_SIZE.0, ACTORS.1 + a * ACTOR_SIZE.1 + 0x20 + t * THREAD_SIZE.1))
    }))
}

fn cle_to_nisa(p: &[u8]) -> Vec<u8> {
    let mut r = vec![0u8; SIZE];
    r[..SCENE.0].copy_from_slice(&p[..SCENE.0]);
    for a in 0..128 {
        let (n, c) = (ACTORS.0 + a * ACTOR_SIZE.0, ACTORS.1 + a * ACTOR_SIZE.1);
        r[n..n + 0x20].copy_from_slice(&p[c..c + 0x20]);
    }
    for (n, c) in threads() {
        r[n..n + 0x3C].copy_from_slice(&p[c..c + 0x3C]);
        r[n + 0x3C..n + 0x96].copy_from_slice(&p[c + 0x40..c + 0x9A]);
    }
    r[TAIL.0..SIZE - 8].copy_from_slice(&p[TAIL.1..CLE_SIZE - 8]);
    fix_sums(&mut r);
    r
}

fn nisa_to_cle(d: &[u8], original: &[u8]) -> Vec<u8> {
    let mut r = original.to_vec();
    r[..SCENE.0].copy_from_slice(&d[..SCENE.0]);
    for a in 0..128 {
        let (n, c) = (ACTORS.0 + a * ACTOR_SIZE.0, ACTORS.1 + a * ACTOR_SIZE.1);
        r[c..c + 0x20].copy_from_slice(&d[n..n + 0x20]);
    }
    for (n, c) in threads() {
        r[c..c + 0x3C].copy_from_slice(&d[n..n + 0x3C]);
        r[c + 0x40..c + 0x9A].copy_from_slice(&d[n + 0x3C..n + 0x96]);
    }
    r[TAIL.1..CLE_SIZE - 8].copy_from_slice(&d[TAIL.0..SIZE - 8]);
    r
}

/// Opens a NISA zstd file: (plain save).
pub fn unzstd(d: &[u8]) -> Option<Vec<u8>> {
    if d.get(..4)? != [0x28, 0xB5, 0x2F, 0xFD] {
        return None;
    }
    let plain = zstd::stream::decode_all(d).ok()?;
    looks(&plain).then_some(plain)
}

pub fn zstd_pack(plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut p = plain.to_vec();
    fix_sums(&mut p);
    zstd::stream::encode_all(&p[..], 3).map_err(|e| e.to_string())
}

/// Opens a KAI file: (plain save in the NISA layout, the original payload).
pub fn uncle(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if d.len() != ((CLE_SIZE + 4) / 16 + 1) * 16 {
        return None;
    }
    let dg = digest();
    let plain = cbc::Decryptor::<aes::Aes128>::new((&dg[..16]).into(), (&dg[16..]).into()).decrypt_padded_vec_mut::<Pkcs7>(d).ok()?;
    let declared = u32::from_le_bytes(plain.get(..4)?.try_into().ok()?) as usize;
    if declared != CLE_SIZE || plain.len() != CLE_SIZE + 4 {
        return None;
    }
    let payload = plain[4..].to_vec();
    Some((cle_to_nisa(&payload), payload))
}

pub fn cle_pack(plain: &[u8], original: &[u8]) -> Result<Vec<u8>, String> {
    if plain.len() != SIZE || original.len() != CLE_SIZE {
        return Err("a Trails to Azure save keeps its size".into());
    }
    let payload = nisa_to_cle(plain, original);
    let mut buf = (CLE_SIZE as u32).to_le_bytes().to_vec();
    buf.extend_from_slice(&payload);
    let dg = digest();
    Ok(cbc::Encryptor::<aes::Aes128>::new((&dg[..16]).into(), (&dg[16..]).into()).encrypt_padded_vec_mut::<Pkcs7>(&buf))
}

const CHARACTERS: [(&str, usize); 11] = [
    ("Lloyd", 0x94),
    ("Elie", 0xC8),
    ("Tio", 0xFC),
    ("Randy", 0x130),
    ("Wazy", 0x164),
    ("Rixia", 0x198),
    ("Zeit", 0x1CC),
    ("Arios", 0x200),
    ("Noel", 0x234),
    ("Dudley", 0x268),
    ("Garcia", 0x29C),
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Trails to Azure save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Party", "Mira"], 0x19C2C, Kind::U32, 0, 9_999_999);
    b.ranged(&["Party", "Diamond points (DP)"], 0x19C1C, Kind::U32, 0, 9_999_999);
    b.ranged(&["Party", "Medals"], 0x19C34, Kind::U32, 0, 9_999_999);
    b.named(&["Party", "Difficulty"], 0x1F36D, Kind::U8, &["Normal", "Hard", "Nightmare", "Easy"]);
    b.ranged(&["Party", "Play time (seconds)"], 0x19C84, Kind::U32, 0, u32::MAX as i64);
    for (i, s) in ["Earth", "Water", "Fire", "Wind", "Time", "Space", "Mirage"].iter().enumerate() {
        b.ranged(&["Sepith", s], 0x19C3C + 4 * i, Kind::U32, 0, 9_999);
    }
    for (name, at) in CHARACTERS {
        b.ranged(&["Characters", name, "Level"], at + 0x08, Kind::U16, 1, 99);
        b.ranged(&["Characters", name, "Max HP"], at, Kind::U32, 1, 999_999);
        b.ranged(&["Characters", name, "HP"], at + 0x04, Kind::U32, 0, 999_999);
        b.ranged(&["Characters", name, "Max EP"], at + 0x0A, Kind::U16, 0, 9_999);
        b.ranged(&["Characters", name, "EP"], at + 0x0C, Kind::U16, 0, 9_999);
        b.ranged(&["Characters", name, "CP"], at + 0x0E, Kind::U16, 0, 200);
        b.ranged(&["Characters", name, "Experience"], at + 0x10, Kind::U32, 0, 99_999_999);
        for (k, s) in ["Strength", "Defence", "Arts strength", "Arts defence"].iter().enumerate() {
            b.ranged(&["Characters", name, s], at + 0x14 + 2 * k, Kind::U16, 0, 9_999);
        }
    }
    for (i, n) in ["Lloyd", "Elie", "Tio", "Randy", "Noel", "Wazy", "Rixia", "Dudley", "Ilia", "Cecile", "Fran", "Sully"].iter().enumerate() {
        b.ranged(&["Bonding", n], 0x1B334 + i, Kind::U8, 0, 255);
    }
    Ok(b.finish("Trails to Azure save", Checksum::Custom(fix_sums)).with_note(
        "Mira, DP, medals, Sepith, play time, difficulty, every character's stats and the bonding values. Both checksums are updated when you save.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nisa() -> Vec<u8> {
        let mut d = vec![0u8; SIZE];
        d[0x19C2C..0x19C30].copy_from_slice(&1234u32.to_le_bytes());
        d[0x94 + 8] = 30;
        fix_sums(&mut d);
        d
    }

    #[test]
    fn zstd_round_trip() {
        let d = nisa();
        let z = zstd_pack(&d).unwrap();
        assert_eq!(unzstd(&z).unwrap(), d);
        let mut t = parse(&d).unwrap();
        t.set("/Party/Mira", serde_json::json!(5)).unwrap();
        let out = t.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(&out[0x19C2C..0x19C30], &5u32.to_le_bytes());
    }

    #[test]
    fn kai_round_trip_keeps_its_own_bytes() {
        let mut orig = vec![0u8; CLE_SIZE];
        orig[0x10] = 7;
        orig[ACTORS.1 + 0x20 + 0x3C] = 0xAB; // a byte only KAI has
        let plain = cle_to_nisa(&orig);
        let packed = cle_pack(&plain, &orig).unwrap();
        let (again, payload) = uncle(&packed).unwrap();
        assert_eq!(again, plain);
        assert_eq!(payload[ACTORS.1 + 0x20 + 0x3C], 0xAB);
        assert_eq!(payload[0x10], 7);
    }
}
