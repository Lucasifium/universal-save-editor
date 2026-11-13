//! Final Fantasy Resonance (Steam) `GameSaveData<n>.sav`: a 32-byte header (`SVHD`, two 16-bit
//! version fields, the SHA-1 of the body, four zero bytes) followed by an Unreal Engine 5.5
//! save game encrypted with AES-256-ECB under a fixed key. The decrypted body is a GVAS file
//! (read by the generic Unreal reader) padded with zeros to the AES block size.
//!
//! Container layout and key as documented by the MIT-licensed community tools
//! ffr-save-editor (Jesús Seijas) and ffr-save-tracker (zabo-dev); see THIRD_PARTY_NOTICES.md.

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use sha1::{Digest, Sha1};

pub const KIND: &str = "Final Fantasy Resonance save";
const HEAD: usize = 32;
const KEY: &[u8; 32] = b"wwrqPkdwzPvgfchQ8kTYHVbn4umTf3d7";

pub fn looks(d: &[u8]) -> bool {
    d.len() > HEAD && d.starts_with(b"SVHD") && (d.len() - HEAD) % 16 == 0
}

/// The header and the decrypted GVAS body, when the checksum matches.
pub fn unpack(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if !looks(d) {
        return None;
    }
    let body = &d[HEAD..];
    if Sha1::digest(body).as_slice() != &d[8..28] {
        return None;
    }
    let cipher = aes::Aes256::new(KEY.into());
    let mut plain = body.to_vec();
    for block in plain.chunks_exact_mut(16) {
        cipher.decrypt_block(block.into());
    }
    if !plain.starts_with(b"GVAS") {
        return None;
    }
    Some((d[..HEAD].to_vec(), plain))
}

/// Encrypts the body again (zero-padded to a whole block) and fixes the checksum.
pub fn pack(head: &[u8], plain: &[u8]) -> Vec<u8> {
    let mut buf = plain.to_vec();
    buf.resize(buf.len().div_ceil(16) * 16, 0);
    let cipher = aes::Aes256::new(KEY.into());
    for block in buf.chunks_exact_mut(16) {
        cipher.encrypt_block(block.into());
    }
    let mut out = head[..HEAD].to_vec();
    out[8..28].copy_from_slice(&Sha1::digest(&buf));
    out.extend(buf);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_is_byte_identical() {
        let sample = std::fs::read("tests/fixtures/ffr_GameSaveData0.sav").unwrap();
        let (head, plain) = unpack(&sample).expect("unpack");
        assert!(plain.starts_with(b"GVAS"));
        assert_eq!(pack(&head, &plain), sample);
    }

    #[test]
    fn tree_shape() {
        let sample = std::fs::read("tests/fixtures/ffr_GameSaveData0.sav").unwrap();
        let (_, plain) = unpack(&sample).unwrap();
        let t = crate::fmt::parse_binary(&plain).unwrap();
        assert_eq!(t.kind_name, KIND);
        assert_eq!(t.view.pointer("/Player/currentMoney"), Some(&serde_json::json!(1234)));
    }

    #[test]
    fn edit_and_save_round_trip() {
        let sample = std::fs::read("tests/fixtures/ffr_GameSaveData0.sav").unwrap();
        let (head, plain) = unpack(&sample).unwrap();
        let mut t = crate::fmt::parse_binary(&plain).unwrap();
        t.set("/Player/currentMoney", serde_json::json!(99999)).unwrap().expect("editable");
        let out = pack(&head, &t.to_bytes().unwrap());
        assert_eq!(out.len(), sample.len());
        let (_, plain2) = unpack(&out).unwrap();
        let t2 = crate::fmt::parse_binary(&plain2).unwrap();
        assert_eq!(t2.view.pointer("/Player/currentMoney"), Some(&serde_json::json!(99999)));
        assert_eq!(t2.view.pointer("/inventory"), t.view.pointer("/inventory"));
    }

    #[test]
    fn bad_checksum_is_refused() {
        let mut sample = std::fs::read("tests/fixtures/ffr_GameSaveData0.sav").unwrap();
        *sample.last_mut().unwrap() ^= 1;
        assert!(unpack(&sample).is_none());
    }
}
