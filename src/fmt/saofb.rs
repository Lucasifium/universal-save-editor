//! Sword Art Online: Fatal Bullet (`SaveData.sav`): every 16-byte block is AES-256-ECB encrypted
//! with a fixed key; newer game versions end the plain data with a 4-byte marker, a length of 20 and
//! an HMAC-SHA1 of everything before it. The plain data is a serialized object stream, so the
//! inventory is found by scanning for the length-prefixed item codes, each followed by its quantity.
//!
//! The key, the checksum recipe and the slot checks follow Sword-Art-Online-FB-Inventory-Editor
//! (alfizari, Apache-2.0); the item names come from that project's list.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const KIND: &str = "Sword Art Online: Fatal Bullet save";
const KEY: &[u8; 32] = b"A3fzg98u0cPO4dgrnjE0o78ie43g75w2";
const HMAC_KEY: &[u8] = b"1FB00CC8D8D94CD0A94C847C2F04A921";
const MARK: [u8; 8] = [0, 0, 0, 0, 20, 0, 0, 0];
const MAX_CHIP: u8 = 60;

fn names() -> &'static HashMap<&'static str, &'static str> {
    static T: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    T.get_or_init(|| include_str!("data/saofb_items.tsv").lines().filter_map(|l| l.split_once('\t')).collect())
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The item code stored as a length-prefixed string at `at`: (code, offset just after it).
fn code_at(d: &[u8], at: usize) -> Option<(&str, usize)> {
    let len = u32_at(d, at)? as usize;
    if !(4..=64).contains(&len) {
        return None;
    }
    let raw = d.get(at + 4..at + 4 + len)?;
    if *raw.last()? != 0 {
        return None;
    }
    let s = std::str::from_utf8(&raw[..len - 1]).ok()?;
    s.is_ascii().then_some((s, at + 4 + len))
}

fn category(code: &str) -> Option<&'static str> {
    Some(match code.as_bytes()[0] {
        b'W' => "Weapons",
        b'C' => "Costumes",
        b'A' => "Accessories",
        b'B' => "Bullets",
        b'M' => "Materials",
        _ => return None,
    })
}

fn min_body(cat: &str) -> usize {
    match cat {
        "Weapons" | "Costumes" | "Bullets" => 71,
        "Accessories" => 51,
        _ => 9,
    }
}

fn valid_slot(d: &[u8], base: usize) -> bool {
    let chips = base + 4 + 0x23;
    if chips + 40 > d.len() {
        return false;
    }
    if let Some(q) = u32_at(d, base) {
        if (4..=64).contains(&q) {
            if let Some((c, _)) = code_at(d, base) {
                if names().contains_key(c) {
                    return false;
                }
            }
        }
    }
    if (0..8).any(|i| d[chips + i * 5] > MAX_CHIP) {
        return false;
    }
    let skip = &d[base + 4..chips];
    !names().keys().any(|k| skip.windows(k.len()).any(|w| w == k.as_bytes()))
}

/// Inventory slots: (category, code, offset of the quantity).
fn slots(d: &[u8]) -> Vec<(&'static str, String, usize)> {
    let ff = [0xffu8; 16];
    let mut start = d.windows(16).position(|w| w == ff).unwrap_or(0);
    if start < 0x20000 {
        start = 0x20000;
    }
    let mut out = Vec::new();
    let end_zero = |from: usize| d[from..].windows(0x30).position(|w| w.iter().all(|b| *b == 0)).map_or(d.len(), |p| from + p);
    // The first known code after the start marks where the inventory begins; its end is the next long run of zeros.
    let mut first = None;
    let mut p = start;
    while p + 8 < d.len() {
        if let Some((c, _)) = code_at(d, p) {
            if names().contains_key(c) {
                first = Some(p);
                break;
            }
        }
        p += 1;
    }
    let Some(first) = first else { return out };
    let end = end_zero(first);
    let mut pos = start;
    while pos + 4 < end {
        let Some((code, base)) = code_at(&d[..end], pos) else {
            pos += 1;
            continue;
        };
        let Some(cat) = category(code) else {
            pos += 1;
            continue;
        };
        if !valid_slot(&d[..end], base) {
            pos += 1;
            continue;
        }
        out.push((cat, code.to_string(), base));
        let min_next = base + min_body(cat);
        let mut next = pos + 1;
        while next + 4 < end {
            if let Some((c, _)) = code_at(&d[..end], next) {
                if category(c).is_some() && next >= min_next {
                    break;
                }
            }
            next += 1;
        }
        pos = next;
    }
    out
}

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 0x20100 && slots(d).len() >= 3
}

fn cipher() -> aes::Aes256 {
    aes::Aes256::new(GenericArray::from_slice(KEY))
}

/// Opens an encrypted file: (the bytes after the last whole block, plain data).
pub fn unpack(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if d.len() < 0x20100 || d.len() > 64 << 20 {
        return None;
    }
    let n = d.len() / 16 * 16;
    let c = cipher();
    let mut plain = d[..n].to_vec();
    for blk in plain.chunks_exact_mut(16) {
        c.decrypt_block(GenericArray::from_mut_slice(blk));
    }
    looks(&plain).then(|| (d[n..].to_vec(), plain))
}

/// Closes the plain data again: refreshes the HMAC when the save carries one, then encrypts.
pub fn pack(plain: &[u8], tail: &[u8]) -> Result<Vec<u8>, String> {
    let mut p = plain.to_vec();
    let from = p.len().saturating_sub(0x1000);
    if let Some(i) = p[from..].windows(8).position(|w| w == MARK) {
        let cut = from + i + 4;
        if p.len() - cut == 24 {
            let mut m = <Hmac<Sha1> as Mac>::new_from_slice(HMAC_KEY).map_err(|e| e.to_string())?;
            m.update(&p[..cut]);
            let sum = m.finalize().into_bytes();
            p[cut + 4..].copy_from_slice(&sum);
        }
    }
    let c = cipher();
    for blk in p.chunks_exact_mut(16) {
        c.encrypt_block(GenericArray::from_mut_slice(blk));
    }
    p.extend_from_slice(tail);
    Ok(p)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let found = slots(d);
    if found.len() < 3 {
        return Err("no inventory found".into());
    }
    let mut b = Builder::new(d);
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (cat, code, at) in &found {
        let name = names().get(code.as_str()).map_or_else(|| code.clone(), |n| format!("{n} ({code})"));
        let n = seen.entry(name.clone()).or_insert(0);
        *n += 1;
        let label = if *n > 1 { format!("{name} #{n}") } else { name };
        b.ranged(&["Inventory", cat, &label, "Quantity"], *at, Kind::U32, 0, 99_999);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("Quantity of every weapon, costume, accessory, bullet and material in the inventory, found by scanning for the item codes. Weapon memory chips are not editable yet.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_and_hmac_round_trip() {
        let mut plain = vec![0u8; 0x20200];
        let n = plain.len();
        plain[n - 28..n - 20].copy_from_slice(&MARK);
        plain[n - 30] = 9;
        let enc = pack(&plain, &[7, 8]).unwrap();
        assert_eq!(&enc[enc.len() - 2..], &[7, 8]);
        let c = cipher();
        let mut back = enc[..enc.len() - 2].to_vec();
        for blk in back.chunks_exact_mut(16) {
            c.decrypt_block(GenericArray::from_mut_slice(blk));
        }
        assert_eq!(&back[..n - 20], &plain[..n - 20]);
        assert_ne!(&back[n - 20..], &plain[n - 20..]);
    }
}
