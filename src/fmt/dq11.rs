//! Dragon Quest XI S (PC `data000.sav` and friends).
//!
//! The file is an 8-byte header and AES-256-ECB data. The plain data starts with `SQEX` and ends
//! with its own length, a CRC-32 (the usual 0xEDB88320 polynomial) of everything before that
//! length, and padding. The body is a stream of Unreal properties; the party's stats and the item
//! bags are found by their markers, so an edit never changes the file's size.
//!
//! Layout facts and the key follow turtle-insect/DQ11_NS (GPL-3.0).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};

pub const KIND: &str = "Dragon Quest XI S save";
const KEY: &[u8; 32] = b"C5VbD9SJxe4FhK7wnWxy_LVSuHfbQjAU";
const CHARACTER: &[u8] = b"JackFriendGameCharacter\0";

fn crc32(d: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn crc_ok(plain: &[u8]) -> bool {
    let n = plain.len();
    if n < 32 || &plain[..4] != b"SQEX" {
        return false;
    }
    match (u32_at(plain, n - 16), u32_at(plain, n - 12)) {
        (Some(size), Some(crc)) => (size as usize) <= n - 16 && crc32(&plain[..size as usize]) == crc,
        _ => false,
    }
}

/// Opens an encrypted file: (8-byte header, plain data).
pub fn unpack(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if d.len() < 40 || (d.len() - 8) % 16 != 0 {
        return None;
    }
    let cipher = aes::Aes256::new(GenericArray::from_slice(KEY));
    let mut plain = d[8..].to_vec();
    for blk in plain.chunks_exact_mut(16) {
        cipher.decrypt_block(GenericArray::from_mut_slice(blk));
    }
    crc_ok(&plain).then(|| (d[..8].to_vec(), plain))
}

/// Closes the plain data again: fixes the CRC, encrypts, puts the header back.
pub fn pack(plain: &[u8], header: &[u8]) -> Result<Vec<u8>, String> {
    let mut p = plain.to_vec();
    fix(&mut p);
    let cipher = aes::Aes256::new(GenericArray::from_slice(KEY));
    for blk in p.chunks_exact_mut(16) {
        cipher.encrypt_block(GenericArray::from_mut_slice(blk));
    }
    let mut out = header.to_vec();
    out.extend_from_slice(&p);
    Ok(out)
}

fn fix(d: &mut Vec<u8>) {
    let n = d.len();
    if n >= 32 {
        if let Some(size) = u32_at(d, n - 16) {
            if (size as usize) <= n - 16 {
                let c = crc32(&d[..size as usize]);
                d[n - 12..n - 8].copy_from_slice(&c.to_le_bytes());
            }
        }
    }
}

pub fn looks(d: &[u8]) -> bool {
    crc_ok(d)
}

/// An item bag starting at `p`: (end, [(id, count offset, max)]).
fn bag(d: &[u8], p: usize) -> Option<(usize, Vec<(String, usize, u32)>)> {
    let c = u32_at(d, p)? as usize;
    if !(1..=700).contains(&c) {
        return None;
    }
    let mut q = p + 4;
    let mut out = vec![];
    for _ in 0..c {
        let l = u32_at(d, q)? as usize;
        if !(4..=40).contains(&l) || q + 4 + l + 9 > d.len() {
            return None;
        }
        let s = &d[q + 4..q + 4 + l];
        let id = &s[..l - 1];
        if s[l - 1] != 0 || id.len() < 3 || !id[0].is_ascii_alphabetic() || id[1] != b'_' || !id.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_') {
            return None;
        }
        let (cnt, mx) = (u32_at(d, q + 4 + l)?, u32_at(d, q + 8 + l)?);
        if cnt > mx.max(99) {
            return None;
        }
        out.push((String::from_utf8_lossy(id).into_owned(), q + 4 + l, mx));
        q += l + 13;
    }
    Some((q, out))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Dragon Quest XI S save".into());
    }
    let mut b = Builder::new(d);
    let mut seen = 0;
    let mut p = 0;
    while p + CHARACTER.len() + 40 < d.len() {
        if &d[p..p + CHARACTER.len()] != CHARACTER {
            p += 1;
            continue;
        }
        let at = p + 24;
        let n = u32_at(d, at).unwrap_or(0) as usize;
        let v = at + 4 + n + 6;
        if (2..=40).contains(&n) && v + 60 <= d.len() && d[at + 4 + n - 1] == 0 {
            let name = String::from_utf8_lossy(&d[at + 4..at + 3 + n]).into_owned();
            let name = if seen > 0 && name.is_empty() { format!("Character {}", seen + 1) } else { name };
            let stats: [(&str, usize, i64, i64); 13] = [
                ("Level", 0, 1, 99),
                ("HP", 4, 0, 999),
                ("MP", 8, 0, 999),
                ("Attack", 12, 0, 999),
                ("Defence", 16, 0, 999),
                ("Agility", 20, 0, 999),
                ("Deftness", 24, 0, 999),
                ("Charm", 28, 0, 999),
                ("Magical might", 32, 0, 999),
                ("Magical mending", 36, 0, 999),
                ("Max HP", 40, 1, 999),
                ("Max MP", 44, 0, 999),
                ("Experience", 56, 0, 9_999_999),
            ];
            for (label, off, lo, hi) in stats.iter() {
                b.ranged(&["Party", &name, label], v + off, Kind::U32, *lo, *hi);
            }
            seen += 1;
        }
        p += CHARACTER.len();
    }
    let mut p = 0;
    let mut n = 0;
    while p + 8 < d.len() {
        match bag(d, p) {
            Some((end, items)) if items.len() >= 3 => {
                n += 1;
                let title = format!("Bag {n}");
                let mut used: std::collections::HashMap<String, u32> = Default::default();
                for (id, at, mx) in items {
                    let k = used.entry(id.clone()).or_insert(0);
                    *k += 1;
                    let key = if *k > 1 { format!("{id} ({k})") } else { id };
                    b.ranged(&["Item bags", &title, &key], at, Kind::U32, 0, mx.clamp(1, 999) as i64);
                }
                p = end;
            }
            _ => p += 1,
        }
    }
    if b.is_empty() {
        return Err("no characters or item bags found".into());
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(crate::i18n::tr("Each party member's level, HP, MP, stats and experience, and the item counts in every bag (up to the item's own maximum). The CRC is updated when you save.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Vec<u8> {
        let mut d = b"SQEX".to_vec();
        d.extend_from_slice(&[0, 0, 0, 0]);
        d.extend_from_slice(CHARACTER);
        d.extend_from_slice(&6u32.to_le_bytes());
        d.extend_from_slice(b"Niema\0");
        d.extend_from_slice(&[1, 1, 1, 1, 5, 0]);
        for v in [43u32, 384, 166, 146, 61, 131, 82, 113, 141, 144, 364, 166, 7, 5, 515051, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        let size = d.len() as u32;
        d.extend_from_slice(&[0; 16]);
        let n = d.len();
        d[n - 16..n - 12].copy_from_slice(&size.to_le_bytes());
        fix(&mut d);
        d
    }

    #[test]
    fn encrypts_and_edits() {
        let d = plain();
        assert!(looks(&d));
        let enc = pack(&d, b"HEADER12").unwrap();
        let (h, p) = unpack(&enc).unwrap();
        assert_eq!(h, b"HEADER12");
        assert_eq!(p, d);
        let mut t = parse(&d).unwrap();
        assert_eq!(t.view.pointer("/Party/Niema/Level"), Some(&serde_json::json!(43)));
        t.set("/Party/Niema/Level", serde_json::json!(99)).unwrap();
        let out = t.to_bytes().unwrap();
        assert!(looks(&out));
    }
}
