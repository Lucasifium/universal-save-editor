//! Dead Space 2 (PC) saves (`ds_slot_NN.deadspace2saved`, about 420 KB): an "RGMH" header with the
//! slot's title as UTF-16 at 0x28, then an "MC02" container at 0x2834 whose two chunks hold the
//! game data. Inside it sit tagged blocks (16 zero bytes, a 4-byte tag, 0xFFFFFFFF, a size and a
//! count): the inventory (credits, then 24-byte entries of item id, slot and quantity), the
//! weapons, the safe, the store and the player's misc values (health, stasis, air, power nodes).
//!
//! Values are edited in place and never move. The game checks two sums, which are made again on
//! save: a multiply-and-add hash of the data at 0x2854 and three MC02 checksums at 0x2844.
//!
//! The block tags, the item ids and the sum routines were read from the GPL-2.0 Dead Space 2 save
//! editor by VakhtinAndrey as facts only (see THIRD_PARTY_NOTICES.md); the code is written fresh and the
//! sums are checked on public saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::OnceLock;

pub const KIND: &str = "Dead Space 2 save";

const MC02: usize = 0x2834;
const HASHED: usize = 0x2854;

fn tag(t: [u8; 4]) -> Vec<u8> {
    let mut v = vec![0u8; 16];
    v.extend(t);
    v.extend([0xff; 4]);
    v
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn find(d: &[u8], pat: &[u8]) -> Option<usize> {
    d.windows(pat.len()).position(|w| w == pat)
}

fn items() -> &'static Vec<([u8; 16], String)> {
    static N: OnceLock<Vec<([u8; 16], String)>> = OnceLock::new();
    N.get_or_init(|| {
        include_str!("data/ds2_items.txt")
            .lines()
            .filter_map(|l| {
                let mut p = l.splitn(3, ':');
                let hex = p.next()?;
                let mut id = [0u8; 16];
                for (i, b) in id.iter_mut().enumerate() {
                    *b = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
                }
                p.next()?;
                Some((id, p.next()?.to_string()))
            })
            .collect()
    })
}

pub fn looks(d: &[u8]) -> bool {
    let title: Vec<u8> = "Dead Space 2".encode_utf16().flat_map(u16::to_le_bytes).collect();
    d.len() > 0x2a00 && d.starts_with(b"RGMH") && d.get(0x28..0x28 + title.len()) == Some(&title[..]) && d.get(MC02..MC02 + 4) == Some(&b"20CM"[..])
}

/// The two MC02 chunk sizes, when they add up.
fn chunks(d: &[u8]) -> Option<(usize, usize)> {
    let (total, c0, c1) = (u32le(d, MC02 + 4)? as usize, u32le(d, MC02 + 8)? as usize, u32le(d, MC02 + 12)? as usize);
    (total == c0.checked_add(c1)?.checked_add(0x1c)? && (MC02 + 0x1c).checked_add(c0)?.checked_add(c1)? <= d.len()).then_some((c0, c1))
}

/// The checksum of the MC02 container: a CRC-like sum with the table of the CRC-32/MPEG-2 polynomial.
fn mc02_sum(b: &[u8]) -> u32 {
    static T: OnceLock<[u32; 256]> = OnceLock::new();
    let t = T.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = (i as u32) << 24;
            for _ in 0..8 {
                c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04c1_1db7 } else { c << 1 };
            }
            *e = c;
        }
        t
    });
    if b.len() < 4 {
        return 0;
    }
    let (b0, b1, b2, b3) = (u32::from(b[0]), u32::from(b[1]), u32::from(b[2]), u32::from(b[3]));
    let mut s = !((b2 << 8) | ((b0 << 24) | (b0 >> 8)) | ((b1 << 16) | (b1 >> 16)) | b3);
    for &x in &b[4..] {
        s = t[((s >> 24) & 0xff) as usize] ^ (u32::from(x) | (s << 8));
    }
    !s
}

fn hash(d: &[u8], at: usize, len: usize, init: u32) -> u32 {
    d[at..at + len].iter().fold(init, |r, &b| r.wrapping_mul(0x1003f).wrapping_add(u32::from(b)))
}

/// Makes both sums again after values changed.
fn fix_sums(d: &mut Vec<u8>) {
    let Some((c0, c1)) = chunks(d) else { return };
    let Some(len1) = u32le(d, 0x28bc).map(|n| n as usize) else { return };
    if 0x29c0usize.checked_add(len1).is_none_or(|e| e > d.len()) {
        return;
    }
    d[HASHED..HASHED + 4].fill(0);
    let h = hash(d, 0x29c0, len1, hash(d, 0x2850, 0x170, 0));
    d[HASHED..HASHED + 4].copy_from_slice(&h.to_le_bytes());
    let start = MC02 + 0x1c;
    let s0 = mc02_sum(&d[start..start + c0]);
    let s1 = mc02_sum(&d[start + c0..start + c0 + c1]);
    d[MC02 + 16..MC02 + 20].copy_from_slice(&s0.to_le_bytes());
    d[MC02 + 20..MC02 + 24].copy_from_slice(&s1.to_le_bytes());
    let s2 = mc02_sum(&d[MC02..MC02 + 0x18]);
    d[MC02 + 24..MC02 + 28].copy_from_slice(&s2.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) || chunks(d).is_none() {
        return Err("not a Dead Space 2 save".into());
    }
    let mut b = Builder::new(d);
    let after = |t: [u8; 4]| find(d, &tag(t)).map(|p| p + 24);
    if let Some(m) = after([0x3d, 0x8d, 0xe8, 0x25]) {
        // size u16, count u16, then health, stasis, air (f32), power nodes (u32).
        b.field(&["Player", "Health"], m + 4, Kind::F32);
        b.field(&["Player", "Stasis"], m + 8, Kind::F32);
        b.field(&["Player", "Air"], m + 12, Kind::F32);
        b.field(&["Player", "Power nodes"], m + 16, Kind::U32);
    }
    if let Some(i) = after([0xe1, 0x70, 0xa2, 0xa3]) {
        let size = d.get(i..i + 2).map(|s| u16::from_le_bytes([s[0], s[1]]) as usize).unwrap_or(0);
        b.field(&["Player", "Credits"], i + 8, Kind::U32);
        let names = items();
        for n in 0..size.saturating_sub(8) / 24 {
            let at = i + 12 + n * 24;
            let Some(id) = d.get(at..at + 16) else { break };
            let name = names.iter().find(|(g, _)| g[..] == *id).map(|(_, n)| n.clone()).unwrap_or_else(|| format!("Item {}", id.iter().take(4).map(|x| format!("{x:02x}")).collect::<String>()));
            let slot = u32le(d, at + 16).unwrap_or(0);
            b.field(&["Inventory", &format!("{name} (slot {})", u64::from(slot) + 1)], at + 20, Kind::U32);
        }
    }
    if b.is_empty() {
        return Err("no Dead Space 2 values found".into());
    }
    Ok(b.finish(KIND, Checksum::Custom(fix_sums)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small made-up save with the container, an inventory and the player block.
    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; 0x29c0];
        d[..4].copy_from_slice(b"RGMH");
        for (i, u) in "Dead Space 2".encode_utf16().enumerate() {
            d[0x28 + i * 2..0x2a + i * 2].copy_from_slice(&u.to_le_bytes());
        }
        let mut body = tag([0xe1, 0x70, 0xa2, 0xa3]);
        body.extend([8 + 24, 0, 8, 0]); // size (header + one item), count
        body.extend(7i32.to_le_bytes());
        body.extend(1234u32.to_le_bytes()); // credits
        body.extend(hex("368cc74b2213659d43454e544b4f5753")); // plasma energy
        body.extend(2u32.to_le_bytes());
        body.extend(25u32.to_le_bytes());
        body.extend(tag([0x3d, 0x8d, 0xe8, 0x25]));
        body.extend([140, 0, 8, 0]);
        body.extend(100f32.to_le_bytes());
        body.extend(40f32.to_le_bytes());
        body.extend(120f32.to_le_bytes());
        body.extend(3u32.to_le_bytes());
        body.extend([0u8; 100]);
        d.extend(&body);
        let c1 = d.len() - 0x29c0;
        d[0x28bc..0x28c0].copy_from_slice(&(c1 as u32).to_le_bytes());
        d[MC02..MC02 + 4].copy_from_slice(b"20CM");
        d[MC02 + 8..MC02 + 12].copy_from_slice(&0x170u32.to_le_bytes());
        let c1_len = (d.len() - 0x2850 - 0x170) as u32;
        d[MC02 + 12..MC02 + 16].copy_from_slice(&c1_len.to_le_bytes());
        let total = (d.len() - MC02) as u32;
        d[MC02 + 4..MC02 + 8].copy_from_slice(&total.to_le_bytes());
        fix_sums(&mut d);
        d
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap()).collect()
    }

    #[test]
    fn reads_edits_and_keeps_sums_valid() {
        let d = synthetic();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Player/Credits"), Some(&serde_json::Value::from(1234)));
        assert_eq!(doc.view.pointer("/Player/Power nodes"), Some(&serde_json::Value::from(3)));
        assert_eq!(doc.view.pointer("/Inventory/Plasma Energy (slot 3)"), Some(&serde_json::Value::from(25)));
        assert_eq!(doc.to_bytes().unwrap(), d, "an untouched save is written back unchanged");
        doc.set("/Player/Credits", serde_json::Value::from(99999)).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = {
            let mut e = out.clone();
            fix_sums(&mut e);
            e
        };
        assert_eq!(out, again, "the sums are current after an edit");
        assert_ne!(out, d);
    }

    #[test]
    fn other_files_are_refused() {
        assert!(!looks(b"RGMH"));
        assert!(parse(&vec![0u8; 0x3000]).is_err());
    }
}
