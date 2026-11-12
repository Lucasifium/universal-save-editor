//! Ace Combat 8: Wings of Theve. `Campaign.sav` and `System.sav` are GVAS files holding
//! three properties: `SavedVersion`, `PackedData` (a byte array of property lists, one after
//! another) and `Checksum`, a CRC-32 of the packed bytes started from the CRC of a salt
//! string fed in as four bytes per character. Layout and salt from RivaTesu's
//! ac8-save-editor (MIT).
//!
//! The packed bytes are opened behind the file's own GVAS header, so the Tree shows each
//! list as "Section n"; on save they go back into the array with its sizes and checksum.

use std::sync::Arc;

const SALT: &str = "XnMVqmFJnH!2";

fn update(mut c: u32, bytes: impl IntoIterator<Item = u8>) -> u32 {
    c = !c;
    for b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg());
        }
    }
    !c
}

fn checksum(packed: &[u8]) -> u32 {
    let seed = update(0, SALT.chars().flat_map(|c| (c as u32).to_le_bytes()));
    update(seed, packed.iter().copied())
}

/// Where the parts sit in the original file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    pub original: Arc<Vec<u8>>,
    /// End of the GVAS header (where `SavedVersion` starts).
    header: usize,
    /// The property's value size, then the array's element count, then the bytes.
    size_at: usize,
    count_at: usize,
    start: usize,
    end: usize,
    sum_at: usize,
}

fn fstring(name: &str) -> Vec<u8> {
    let mut v = ((name.len() + 1) as i32).to_le_bytes().to_vec();
    v.extend_from_slice(name.as_bytes());
    v.push(0);
    v
}

fn find(d: &[u8], what: &[u8], from: usize) -> Option<usize> {
    d.get(from..)?.windows(what.len()).position(|w| w == what).map(|i| i + from)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The packed bytes behind the GVAS header, when the stored checksum matches.
pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !d.starts_with(b"GVAS") || d.len() < 64 {
        return None;
    }
    let header = find(d, &fstring("SavedVersion"), 0)?;
    let name = find(d, &fstring("PackedData"), header)?;
    // The value: size S, a flags byte, then the element count S - 4 and the bytes.
    let (size_at, count_at, start, end) = (name..name + 128).find_map(|p| {
        let s = u32_at(d, p)? as usize;
        let n = u32_at(d, p + 5)? as usize;
        (s >= 4 && n == s - 4 && p + 9 + n <= d.len()).then_some((p, p + 5, p + 9, p + 9 + n))
    })?;
    // The file ends with Checksum's value, then "None" and a zero.
    let tail = fstring("None").len() + 4;
    let sum_at = d.len().checked_sub(tail + 4)?;
    if sum_at < end || find(d, &fstring("Checksum"), end).is_none() || u32_at(d, sum_at)? != checksum(&d[start..end]) {
        return None;
    }
    let mut inner = d[..header].to_vec();
    inner.extend_from_slice(&d[start..end]);
    let original = Arc::new(d.to_vec());
    Some((Packed { original, header, size_at, count_at, start, end, sum_at }, inner))
}

pub fn close(p: &Packed, inner: &[u8]) -> Result<Vec<u8>, String> {
    let packed = inner.get(p.header..).ok_or("the edited save lost its header")?;
    let o = &p.original;
    let mut out = o[..p.start].to_vec();
    out.extend_from_slice(packed);
    out.extend_from_slice(&o[p.end..]);
    let n = packed.len() as u32;
    out[p.size_at..p.size_at + 4].copy_from_slice(&(n + 4).to_le_bytes());
    out[p.count_at..p.count_at + 4].copy_from_slice(&n.to_le_bytes());
    let sum_at = p.sum_at + packed.len() - (p.end - p.start);
    out[sum_at..sum_at + 4].copy_from_slice(&checksum(packed).to_le_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up save shaped like the game's: header, SavedVersion, PackedData, Checksum.
    pub fn sample(packed: &[u8]) -> Vec<u8> {
        let mut d = b"GVAS".to_vec();
        d.extend(3i32.to_le_bytes());
        d.extend(522i32.to_le_bytes());
        d.extend(1012i32.to_le_bytes());
        d.extend([5, 0, 4, 0, 0, 0, 1, 2, 3, 4]);
        d.extend(fstring("++UE5+Release-5.4"));
        d.extend(3i32.to_le_bytes());
        d.extend(0i32.to_le_bytes());
        d.extend(fstring("/Script/AC8.LiveSaveGame"));
        d.push(0);
        d.extend(fstring("SavedVersion"));
        d.extend(fstring("IntProperty"));
        d.extend(0i32.to_le_bytes());
        d.extend(4i32.to_le_bytes());
        d.push(0);
        d.extend(7i32.to_le_bytes());
        d.extend(fstring("PackedData"));
        d.extend(fstring("ArrayProperty"));
        d.extend(1i32.to_le_bytes());
        d.extend(fstring("ByteProperty"));
        d.extend(0i32.to_le_bytes());
        d.extend(((packed.len() + 4) as i32).to_le_bytes());
        d.push(0);
        d.extend((packed.len() as i32).to_le_bytes());
        d.extend(packed);
        d.extend(fstring("Checksum"));
        d.extend(fstring("UInt32Property"));
        d.extend(0i32.to_le_bytes());
        d.extend(4i32.to_le_bytes());
        d.push(0);
        d.extend(checksum(packed).to_le_bytes());
        d.extend(fstring("None"));
        d.extend(0i32.to_le_bytes());
        d
    }

    /// The whole path: packed lists opened as a Tree, edited, packed again with the checksum.
    #[test]
    fn edits_through_the_layers() {
        let inner = crate::fmt::gvas::tests::sections5();
        let at = find(&inner, &fstring("Gold"), 0).unwrap();
        let raw = sample(&inner[at..]);
        let mut d = crate::formats::decode(&raw);
        assert!(matches!(d.layers.as_slice(), [crate::formats::Layer::Ac8(_)]), "{:?}", d.layers);
        let crate::formats::Content::Tree(t) = &mut d.content else { panic!("not a tree") };
        assert_eq!(t.view.pointer("/Section 2/Rank").unwrap(), 3);
        t.set("/Section 2/Rank", serde_json::Value::from(12)).unwrap();
        let out = crate::formats::encode(&d.layers, &d.content).unwrap();
        let (_, again) = open(&out).expect("checksum written again");
        assert_eq!(crate::fmt::gvas::parse(&again).unwrap().view.pointer("/Section 2/Rank").unwrap(), 12);
    }

    #[test]
    fn salt_seed_is_the_wide_string_crc() {
        // Four bytes per character, as FCrc::StrCrc32 feeds them.
        let mut wide = Vec::new();
        for c in SALT.chars() {
            wide.extend((c as u32).to_le_bytes());
        }
        assert_eq!(update(0, wide.iter().copied()), update(0, SALT.chars().flat_map(|c| (c as u32).to_le_bytes())));
        let mut h = flate2::Crc::new();
        h.update(&wide);
        assert_eq!(h.sum(), update(0, wide));
    }

    #[test]
    fn packed_round_trip() {
        let raw = sample(b"\x05\x00\x00\x00None\x00\x05\x00\x00\x00None\x00");
        let (p, inner) = open(&raw).unwrap();
        assert!(inner.starts_with(b"GVAS"));
        assert_eq!(close(&p, &inner).unwrap(), raw);
        let longer = [&inner[..], b"\x05\x00\x00\x00None\x00"].concat();
        let out = close(&p, &longer).unwrap();
        let (_, again) = open(&out).unwrap();
        assert_eq!(again, longer);
        let mut bad = raw.clone();
        bad[p.start + 2] ^= 1;
        assert!(open(&bad).is_none());
    }
}
