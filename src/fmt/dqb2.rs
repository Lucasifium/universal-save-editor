//! Dragon Quest Builders 2 (`CMNDAT.BIN`, the common data of a save slot): an `aerC` header of
//! 0x2A444 bytes followed by one zlib stream. The builder's level, experience, hit points, attack
//! and defence, the mini medals and the item slots of the pockets and the bag are edited in place.
//! The island files (`STGDAT*.BIN`) are far too large and are left alone.
//!
//! Offsets follow DQB2 (turtle-insect, GPL-3.0).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{Read, Write};

pub const KIND: &str = "Dragon Quest Builders 2 common data";
const HEADER: usize = 0x2A444;
const MAX_PLAIN: u64 = 64 << 20;

/// Opens the file: (the header, the plain data).
pub fn unpack(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if d.len() < HEADER + 8 || !d.starts_with(b"aerC") || d[HEADER] != 0x78 {
        return None;
    }
    let mut plain = Vec::new();
    ZlibDecoder::new(&d[HEADER..]).take(MAX_PLAIN).read_to_end(&mut plain).ok()?;
    looks(&plain).then(|| (d[..HEADER].to_vec(), plain))
}

/// Closes the plain data again: compresses it and records the new file length in the header.
pub fn pack(plain: &[u8], header: &[u8]) -> Result<Vec<u8>, String> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(plain).map_err(|e| e.to_string())?;
    let body = z.finish().map_err(|e| e.to_string())?;
    let mut out = header.to_vec();
    out.extend_from_slice(&body);
    let n = out.len() as u32;
    out[0x10..0x14].copy_from_slice(&n.to_le_bytes());
    Ok(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 0x55B9A0 && d.len() < 32 << 20 && (1..=99).contains(&d[0x6A9CF])
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not Dragon Quest Builders 2 common data".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Builder", "Level"], 0x6A9CF, Kind::U8, 1, 99);
    b.ranged(&["Builder", "Experience"], 0x6A9D1, Kind::U32, 0, 9_999_999);
    b.ranged(&["Builder", "Hit points"], 0x6A890, Kind::U16, 1, 999);
    b.ranged(&["Builder", "Hit points bonus"], 0x6A892, Kind::U16, 0, 999);
    b.ranged(&["Builder", "Attack"], 0x6A898, Kind::U16, 1, 9_999);
    b.ranged(&["Builder", "Defence"], 0x6A89A, Kind::U16, 1, 9_999);
    b.ranged(&["Mini medals", "Held"], 0x226E40, Kind::U8, 0, 90);
    b.ranged(&["Mini medals", "Deposited"], 0x226E44, Kind::U8, 0, 90);
    for (name, at) in [("Weapon", 0x55B959), ("Hammer", 0x55B95D), ("Shield", 0x55B985), ("Armour", 0x55B989)] {
        b.ranged(&["Equipment", name, "Item id"], at, Kind::U16, 0, 9_999);
    }
    let slot = |b: &mut Builder, group: &str, i: usize, at: usize| {
        let id = u16::from_le_bytes([d[at], d[at + 1]]);
        if id == 0 {
            return;
        }
        let label = format!("Slot {} (item {id})", i + 1);
        b.ranged(&[group, &label, "Item id"], at, Kind::U16, 0, 9_999);
        b.ranged(&[group, &label, "Quantity"], at + 2, Kind::U16, 0, 999);
    };
    for i in 0..15 {
        slot(&mut b, "Pockets", i, 0x55B28D + i * 4);
    }
    for i in 0..420 {
        slot(&mut b, "Bag", i, 0x55B2C9 + i * 4);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("Builder level, experience and stats, mini medals, equipment and the item slots of the pockets and the bag (item ids only; no name list is available). The island files are not opened.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_with_the_new_length() {
        let mut plain = vec![0u8; 0x55C000];
        plain[0x6A9CF] = 5;
        let mut file = b"aerC".to_vec();
        file.resize(HEADER, 0);
        let packed = pack(&plain, &file).unwrap();
        assert_eq!(u32::from_le_bytes(packed[0x10..0x14].try_into().unwrap()) as usize, packed.len());
        let (h, p) = unpack(&packed).unwrap();
        assert_eq!(p, plain);
        assert_eq!(pack(&p, &h).unwrap(), packed);
    }
}
