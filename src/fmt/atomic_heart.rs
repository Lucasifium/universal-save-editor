//! Atomic Heart `<n>.sav` (SaveExtension plugin slot files, Unreal 4.27).
//!
//! ```text
//! u32 1, u32 2, GVAS-style header (engine version, custom versions)   no "GVAS" magic
//! FString /Script/SaveExtension.SlotInfo, then its properties         readable as they are
//! FString /Script/SaveExtension.SlotData, FString "Oodle",
//! u32 n, then n bytes of blocks, each block holding up to 128 KiB:
//!   i64 0x9E2A83C1, i64 131072, i64 packed, i64 raw, i64 packed, i64 raw, packed bytes (Oodle)
//! 20 bytes: SHA-1 of everything before them
//! ```
//!
//! The blocks are unpacked with the Oodle library the game itself installs (see `oodle.rs`);
//! packing again re-blocks the stream, rewrites the length and makes the SHA-1 again.

use sha1::{Digest, Sha1};

const MARKER: &[u8] = b"/Script/SaveExtension.SlotData";
const TAG: i64 = 0x9E2A_83C1;
const CHUNK: usize = 131_072;

/// The file around the unpacked stream: everything before the block length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    head: Vec<u8>,
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 4096
        && d.starts_with(&[1, 0, 0, 0, 2, 0, 0, 0])
        && d[..4096.min(d.len())].windows(30).any(|w| w == b"/Script/SaveExtension.SlotInfo")
}

fn i64_at(d: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Where the block length is: right after the "Oodle" compressor name that follows the SlotData class.
fn start(d: &[u8]) -> Option<usize> {
    let m = d.windows(MARKER.len()).position(|w| w == MARKER)? + MARKER.len() + 1;
    let name = b"\x06\x00\x00\x00Oodle\x00";
    (d.get(m..m + name.len())? == name).then_some(m + name.len())
}

/// Splits the file into the part before the blocks and the unpacked stream.
pub fn unpack(d: &[u8]) -> Result<(Layout, Vec<u8>), String> {
    if !looks(d) {
        return Err("not an Atomic Heart save".into());
    }
    let at = start(d).ok_or("this slot file's data isn't Oodle-packed")?;
    let total = u32::from_le_bytes(d.get(at..at + 4).ok_or("cut off")?.try_into().unwrap()) as usize;
    let first = at + 4;
    let end = first.checked_add(total).filter(|&e| e + 20 == d.len()).ok_or("the save's block length doesn't match its size")?;
    if Sha1::digest(&d[..end]).as_slice() != &d[end..] {
        return Err("the save's SHA-1 doesn't match".into());
    }
    let mut blocks: Vec<(&[u8], usize)> = vec![];
    let mut p = first;
    while p < end {
        let (tag, size, sc, su, cc, cu) = (i64_at(d, p), i64_at(d, p + 8), i64_at(d, p + 16), i64_at(d, p + 24), i64_at(d, p + 32), i64_at(d, p + 40));
        let (Some(tag), Some(size), Some(sc), Some(su), Some(cc), Some(cu)) = (tag, size, sc, su, cc, cu) else { return Err("cut off".into()) };
        if tag != TAG || size != CHUNK as i64 || sc != cc || su != cu || cc < 0 || !(0..=CHUNK as i64).contains(&cu) {
            return Err("unexpected block header".into());
        }
        let body = d.get(p + 48..p + 48 + cc as usize).filter(|_| p + 48 + cc as usize <= end).ok_or("cut off")?;
        blocks.push((body, cu as usize));
        p += 48 + cc as usize;
    }
    let mut out = Vec::with_capacity(blocks.len() * CHUNK);
    for b in crate::par::map(&blocks, |&(b, n)| crate::oodle::decompress(b, n)) {
        out.extend(b.ok_or("Oodle couldn't unpack the save's data")?);
    }
    Ok((Layout { head: d[..at].to_vec() }, out))
}

pub fn pack(l: &Layout, stream: &[u8]) -> Result<Vec<u8>, String> {
    let parts: Vec<&[u8]> = stream.chunks(CHUNK).collect();
    let mut blocks = vec![];
    for p in crate::par::map(&parts, |p| crate::oodle::compress(p)) {
        blocks.push(p.ok_or("Oodle couldn't pack the save again (no Oodle library found)")?);
    }
    let mut body = vec![];
    for (raw, packed) in parts.iter().zip(&blocks) {
        for v in [TAG, CHUNK as i64, packed.len() as i64, raw.len() as i64, packed.len() as i64, raw.len() as i64] {
            body.extend(v.to_le_bytes());
        }
        body.extend(packed);
    }
    let mut out = l.head.clone();
    out.extend((body.len() as u32).to_le_bytes());
    out.extend(body);
    let sum = Sha1::digest(&out);
    out.extend(sum);
    Ok(out)
}
