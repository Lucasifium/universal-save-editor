//! Wreckfest profile files (`career.cres`, `cars5.ccrs`, ...): a 12-byte header (the number 7,
//! a reversed four-letter tag, a version) followed by blocks of (compressed size, CRC-32C of the
//! decompressed data, an LZ4 block). Each block holds up to 64 KB and may refer back into the
//! earlier blocks. Only the career file (tag `serc`) is opened on a screen; it holds the driver
//! name, credits and experience, and the tournament fame counter.
//!
//! The container and the career layout follow RSWreckGarage (RootSwitch, Unlicense).
//!
//! The opened data is the header followed by the decompressed stream.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Wreckfest career";
const BLOCK: usize = 65536;
const CAREER_TAG: &[u8; 4] = b"serc";
/// The empty tournament-events chunk that ends the 'leot' block, right before the fame counter.
const NEOT_EMPTY: &[u8] = b"neot\x06\x00\x00\x00\x00\x00\x00\x00";

fn crc32c(d: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in d {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0x82F6_3B78 } else { crc >> 1 };
        }
    }
    !crc
}

/// Appends the content of one LZ4 block to `out`; matches may reach back into earlier blocks.
fn lz4_into(out: &mut Vec<u8>, src: &[u8]) -> Option<()> {
    let (mut i, n) = (0usize, src.len());
    while i < n {
        let token = *src.get(i)?;
        i += 1;
        let mut lit = (token >> 4) as usize;
        if lit == 15 {
            loop {
                let b = *src.get(i)? as usize;
                i += 1;
                lit += b;
                if b != 255 {
                    break;
                }
            }
        }
        out.extend_from_slice(src.get(i..i + lit)?);
        i += lit;
        if i >= n {
            break;
        }
        let off = *src.get(i)? as usize | (*src.get(i + 1)? as usize) << 8;
        i += 2;
        let mut ml = (token & 15) as usize;
        if ml == 15 {
            loop {
                let b = *src.get(i)? as usize;
                i += 1;
                ml += b;
                if b != 255 {
                    break;
                }
            }
        }
        ml += 4;
        if off == 0 || off > out.len() || out.len() + ml > 1 << 28 {
            return None;
        }
        let s = out.len() - off;
        for k in 0..ml {
            out.push(out[s + k]);
        }
    }
    Some(())
}

fn u32at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The header and the decompressed stream of a career file, checked block by block.
pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 12 || u32at(data, 0)? != 7 || data.get(4..8)? != CAREER_TAG {
        return None;
    }
    let mut out = data[..12].to_vec();
    let mut stream = Vec::new();
    let mut p = 12;
    while p < data.len() {
        let (csize, crc) = (u32at(data, p)? as usize, u32at(data, p + 4)?);
        p += 8;
        let before = stream.len();
        lz4_into(&mut stream, data.get(p..p + csize)?)?;
        p += csize;
        if stream.len() - before > BLOCK || crc32c(&stream[before..]) != crc {
            return None;
        }
    }
    out.extend_from_slice(&stream);
    Some(out)
}

/// Blocks of independent LZ4 data (valid for a linked reader as well).
pub fn pack(plain: &[u8]) -> Result<Vec<u8>, String> {
    let (head, stream) = plain.split_at(12.min(plain.len()));
    let mut out = head.to_vec();
    for chunk in stream.chunks(BLOCK) {
        let comp = lz4_flex::block::compress(chunk);
        out.extend_from_slice(&(comp.len() as u32).to_le_bytes());
        out.extend_from_slice(&crc32c(chunk).to_le_bytes());
        out.extend_from_slice(&comp);
    }
    Ok(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 20 && u32at(d, 0) == Some(7) && d.get(4..8) == Some(CAREER_TAG)
}

fn fame_at(d: &[u8]) -> Option<usize> {
    let mut end = d.len();
    while let Some(pos) = d[..end].windows(NEOT_EMPTY.len()).rposition(|w| w == NEOT_EMPTY) {
        let off = pos + NEOT_EMPTY.len();
        if d.get(off + 5..off + 9) == Some(b"nrpu") && d[..pos].windows(4).any(|w| w == b"leot") {
            return Some(off);
        }
        end = pos + NEOT_EMPTY.len() - 1;
    }
    None
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Wreckfest career".into());
    }
    let bad = || "unexpected Wreckfest career layout".to_string();
    let n = u32at(d, 12).ok_or_else(bad)? as usize;
    let name = String::from_utf8_lossy(d.get(16..16 + n).ok_or_else(bad)?).to_string();
    let p = 16 + n;
    let mut b = Builder::new(d);
    b.info(&["Driver", "Name"], name);
    b.ranged(&["Driver", "Credits"], p, Kind::U32, 0, 2_000_000_000).ok_or_else(bad)?;
    b.ranged(&["Driver", "Experience"], p + 4, Kind::U32, 0, 2_000_000_000).ok_or_else(bad)?;
    if let Some(at) = fame_at(d) {
        b.ranged(&["Driver", "Fame"], at, Kind::U32, 0, 2_000_000_000);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("Close the game first. The file is written again in its block format with fresh checksums.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn career() -> Vec<u8> {
        let mut s = vec![];
        s.extend_from_slice(&6u32.to_le_bytes());
        s.extend_from_slice(b"Driver");
        s.extend_from_slice(&10_000u32.to_le_bytes());
        s.extend_from_slice(&77u32.to_le_bytes());
        s.extend_from_slice(b"leot");
        s.extend_from_slice(NEOT_EMPTY);
        s.extend_from_slice(&5u32.to_le_bytes());
        s.push(0xff);
        s.extend_from_slice(b"nrpu");
        let mut plain = vec![7, 0, 0, 0];
        plain.extend_from_slice(b"serc");
        plain.extend_from_slice(&40u32.to_le_bytes());
        plain.extend_from_slice(&s);
        pack(&plain).unwrap()
    }

    #[test]
    fn credits_and_fame_edit_and_pack_again() {
        let file = career();
        let plain = open(&file).unwrap();
        assert!(looks(&plain));
        let mut t = parse(&plain).unwrap();
        assert_eq!(t.view.pointer("/Driver/Credits"), Some(&serde_json::Value::from(10_000)));
        assert_eq!(t.view.pointer("/Driver/Fame"), Some(&serde_json::Value::from(5)));
        t.set("/Driver/Credits", serde_json::Value::from(99_999_999)).unwrap();
        let out = pack(&t.to_bytes().unwrap()).unwrap();
        let again = open(&out).unwrap();
        assert_eq!(u32at(&again, 16 + 6), Some(99_999_999));
    }

    #[test]
    fn a_wrong_checksum_is_refused() {
        let mut file = career();
        file[16] ^= 1;
        assert!(open(&file).is_none());
    }
}
