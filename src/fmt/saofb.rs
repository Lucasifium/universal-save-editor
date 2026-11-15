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

/// The lengths of the item codes, shortest first.
fn code_lens() -> &'static [usize] {
    static T: OnceLock<Vec<usize>> = OnceLock::new();
    T.get_or_init(|| {
        let mut v: Vec<usize> = names().keys().map(|k| k.len()).collect();
        v.sort_unstable();
        v.dedup();
        v
    })
}

/// Whether a code of `len` bytes starting with `first` can be in the table: a cheap test made
/// before the code is read and looked up.
fn may_be_known(len: usize, first: u8) -> bool {
    static FIRST: OnceLock<[bool; 256]> = OnceLock::new();
    let firsts = FIRST.get_or_init(|| {
        let mut t = [false; 256];
        names().keys().filter_map(|k| k.bytes().next()).for_each(|b| t[usize::from(b)] = true);
        t
    });
    firsts[usize::from(first)] && code_lens().binary_search(&len).is_ok()
}

/// Whether one of the item codes appears anywhere in `skip`.
fn holds_a_code(skip: &[u8]) -> bool {
    code_lens().iter().any(|&n| skip.windows(n).any(|w| std::str::from_utf8(w).is_ok_and(|s| names().contains_key(s))))
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
    !holds_a_code(&d[base + 4..chips])
}

/// Inventory slots: (category, code, offset of the quantity).
fn slots(d: &[u8]) -> Vec<(&'static str, String, usize)> {
    let mut start = super::find_bytes(d, &[0xff; 16]).unwrap_or(0);
    if start < 0x20000 {
        start = 0x20000;
    }
    let mut out = Vec::new();
    let end_zero = |from: usize| super::find_bytes(&d[from..], &[0; 0x30]).map_or(d.len(), |p| from + p);
    // The first known code after the start marks where the inventory begins; its end is the next long run of zeros.
    let mut first = None;
    let mut p = start;
    while p + 8 < d.len() {
        let w = u64::from_le_bytes(d[p..p + 8].try_into().unwrap());
        // Eight zero bytes start no length of 4 to 64; and as such a length has three zero
        // bytes on top, a byte that isn't zero at p + 3 rules out p, p + 1 and p + 2.
        if w == 0 {
            p += 8;
            continue;
        }
        if (w >> 24) & 0xFF != 0 {
            p += 3;
            continue;
        }
        let len = w as u32;
        if !(4..=64).contains(&len) || !may_be_known(len as usize - 1, d[p + 4]) {
            p += 1;
            continue;
        }
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
        // As above: no length at pos, pos + 1 or pos + 2.
        if d[pos + 3] != 0 {
            pos += 3;
            continue;
        }
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
            if d[next + 3] != 0 {
                next += 3;
                continue;
            }
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

    /// The scan as first written, byte by byte: `slots` must find the same.
    fn slots_by_byte(d: &[u8]) -> Vec<(&'static str, String, usize)> {
        let valid = |d: &[u8], base: usize| {
            let chips = base + 4 + 0x23;
            if chips + 40 > d.len() || code_at(d, base).is_some_and(|(c, _)| names().contains_key(c)) || (0..8).any(|i| d[chips + i * 5] > MAX_CHIP) {
                return false;
            }
            !names().keys().any(|k| d[base + 4..chips].windows(k.len()).any(|w| w == k.as_bytes()))
        };
        let start = d.windows(16).position(|w| w == [0xff; 16]).unwrap_or(0).max(0x20000);
        let Some(first) = (start..d.len().saturating_sub(8)).find(|&p| code_at(d, p).is_some_and(|(c, _)| names().contains_key(c))) else { return vec![] };
        let end = d[first..].windows(0x30).position(|w| w.iter().all(|b| *b == 0)).map_or(d.len(), |p| first + p);
        let (mut out, mut pos) = (Vec::new(), start);
        while pos + 4 < end {
            let Some((code, base)) = code_at(&d[..end], pos).filter(|(c, b)| category(c).is_some() && valid(&d[..end], *b)) else {
                pos += 1;
                continue;
            };
            let cat = category(code).unwrap();
            out.push((cat, code.to_string(), base));
            let min_next = base + min_body(cat);
            let mut next = pos + 1;
            while next + 4 < end && !(code_at(&d[..end], next).is_some_and(|(c, _)| category(c).is_some()) && next >= min_next) {
                next += 1;
            }
            pos = next;
        }
        out
    }

    #[test]
    fn slots_match_the_byte_by_byte_scan() {
        let mut x = 0x2545_F491u32;
        let mut rnd = move || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        };
        let mut codes: Vec<&str> = names().keys().copied().collect();
        codes.sort_unstable();
        for round in 0..6 {
            // Mostly small numbers and zeros, as in the game's data, with item codes spread in.
            let mut d: Vec<u8> = (0..0x24000).map(|_| match rnd() % 16 { 0..=11 => 0, 12..=14 => (rnd() % 70) as u8, _ => rnd() as u8 }).collect();
            if round % 2 == 1 {
                d[0x1F000..0x1F010].fill(0xff);
            }
            let mut at = 0x20010 + (rnd() % 64) as usize;
            for _ in 0..(6 + round * 4) {
                let code = codes[rnd() as usize % codes.len()];
                let code = if rnd() % 5 == 0 { "WXX_NOT_AN_ITEM" } else { code };
                d[at..at + 4].copy_from_slice(&(code.len() as u32 + 1).to_le_bytes());
                d[at + 4..at + 4 + code.len()].copy_from_slice(code.as_bytes());
                d[at + 4 + code.len()] = 0;
                at += 4 + code.len() + 1 + 60 + (rnd() % 120) as usize;
            }
            if round == 4 {
                d[at + 200..at + 300].fill(0);
            }
            let want = slots_by_byte(&d);
            assert_eq!(slots(&d), want, "round {round}");
            if round == 0 {
                assert!(!want.is_empty(), "the test data should hold slots");
            }
        }
        let zeros = vec![0u8; 0x30000];
        assert_eq!(slots(&zeros), slots_by_byte(&zeros));
    }
}
