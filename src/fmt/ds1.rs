//! Death Stranding (PC and Director's Cut) `checkpoint.dat`.
//!
//! Container: a 24-byte header of three decimal sizes in ASCII (8 bytes each, cut at the first
//! NUL): the slot description (plain text), the PNG thumbnail and the game data, which follow in
//! that order. The game data starts with the magic `1D 1E 39 C0` and a u32 seed; the rest is
//! XORed with the same 16-byte key as Death Stranding 2's saves (MurmurHash3 of the seed and fixed
//! key material). Under the XOR: the account id (u32 length + text, "steam_…"), then the save
//! itself, Oodle Kraken compressed in 256 KiB pieces to a fixed 12 MiB image.
//!
//! The image is Decima game data like Death Stranding 2's: an MD5 of its data block at 0x1EC and
//! the block's size at 0x1FC. Facilities are a table of 152-byte records after the marker
//! `D5 98 82 17 05 F1 C4 89 55 29` (+13 bytes): the six material stocks (u32, 8 bytes apart) at
//! +0x34, the facility code at +0x94, then its index (u16) and connection level (u16). The table
//! layout and the facilities' order follow the public PS4 save patches (bucanero/apollo-patches,
//! facts only); the key derivation is from mi5hmash/DeathStrandingSaveDataResigner (MIT). Checked
//! on public PC saves.
//!
//! Oodle isn't part of this program: the saves are read and written with the Oodle library a
//! game on this computer already has (see `crate::oodle`).

use super::ds2::{key, xor};
use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::Arc;

pub const KIND: &str = "Death Stranding save";

const MAGIC: [u8; 4] = [0x1D, 0x1E, 0x39, 0xC0];
const PIECE: usize = 256 * 1024;
const FACILITY_MARK: [u8; 10] = [0xD5, 0x98, 0x82, 0x17, 0x05, 0xF1, 0xC4, 0x89, 0x55, 0x29];
const RECORD: usize = 152;

pub use super::ds2::MATERIALS;

/// The facilities in the order the save keeps them (from the PS4 save patches' list). Entries
/// the list doesn't name are shown by the game's facility code.
const FACILITIES: [&str; 41] = [
    "Capital Knot City",
    "Waystation West of Capital Knot City",
    "Distribution Center West of Capital Knot City",
    "Wind Farm",
    "Port Knot City",
    "The Ludens Fan",
    "Musician",
    "Engineer",
    "Elder",
    "Craftsman",
    "Timefall Farm",
    "Junk Dealer",
    "Chiral Artist's Studio",
    "Waystation North of Mountain Knot City",
    "Roboticist",
    "The Mountaineer",
    "The Photographer",
    "Spiritualist",
    "Doctor",
    "Geologist",
    "Paleontologist",
    "Veteran Porter",
    "Evo-devo Biologist",
    "Peter Englert",
    "Film Director",
    "Cosplayer",
    "First Prepper",
    "Collector",
    "Novelist's Son",
    "Lake Knot City",
    "Distribution Center South of Lake Knot City",
    "Weather Station",
    "South Knot City",
    "Mama's Lab",
    "Distribution Center North of Mountain Knot City",
    "Mountain Knot City",
    "",
    "",
    "Distribution Center North of Edge Knot City",
    "Edge Knot City",
    "",
];

// ------------------------------------------------------------------ container

/// What's needed to write a save back.
pub struct Wrap {
    original: Arc<Vec<u8>>,
    /// The header's first two sizes and the bytes they cover (slot description and thumbnail).
    meta: Vec<u8>,
    png: Vec<u8>,
    seed: [u8; 4],
    /// The account id with its length, as stored before the compressed data.
    account: Vec<u8>,
    plain: Arc<Vec<u8>>,
}

impl PartialEq for Wrap {
    fn eq(&self, o: &Self) -> bool {
        Arc::ptr_eq(&self.original, &o.original) && Arc::ptr_eq(&self.plain, &o.plain)
    }
}
impl Eq for Wrap {}

impl std::fmt::Debug for Wrap {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Ds1 {{ account: {} bytes, data: {} bytes }}", self.account.len(), self.plain.len())
    }
}

fn size_field(b: &[u8]) -> Option<usize> {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let s = std::str::from_utf8(&b[..n]).ok()?;
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// The three header sizes, when they add up to the file.
fn sizes(d: &[u8]) -> Option<(usize, usize, usize)> {
    if d.len() < 32 {
        return None;
    }
    let (m, p, g) = (size_field(&d[..8])?, size_field(&d[8..16])?, size_field(&d[16..24])?);
    (24 + m + p + g == d.len() && d.get(24 + m + p..24 + m + p + 4) == Some(&MAGIC[..])).then_some((m, p, g))
}

pub fn looks(d: &[u8]) -> bool {
    sizes(d).is_some()
}

/// The slot's description text (title with the play time, location), as the game shows it.
pub fn description(d: &[u8]) -> Option<String> {
    let (m, _, _) = sizes(d)?;
    Some(String::from_utf8_lossy(&d[24..24 + m]).into_owned())
}

/// The parts of a file: description, thumbnail, seed, account id (with its length) and the
/// compressed stream.
struct Parts {
    meta: Vec<u8>,
    png: Vec<u8>,
    seed: [u8; 4],
    account: Vec<u8>,
    stream: Vec<u8>,
}

fn split(d: &[u8]) -> Option<Parts> {
    let (m, p, _) = sizes(d)?;
    let data = &d[24 + m + p..];
    let seed: [u8; 4] = data[4..8].try_into().ok()?;
    let body = xor(&data[8..], &key(seed));
    let n = u32::from_le_bytes(body.get(..4)?.try_into().ok()?) as usize;
    if n > 255 || 4 + n > body.len() {
        return None;
    }
    Some(Parts {
        meta: d[24..24 + m].to_vec(),
        png: d[24 + m..24 + m + p].to_vec(),
        seed,
        account: body[..4 + n].to_vec(),
        stream: body[4 + n..].to_vec(),
    })
}

fn join(meta: &[u8], png: &[u8], seed: [u8; 4], account: &[u8], stream: &[u8]) -> Vec<u8> {
    let mut body = account.to_vec();
    body.extend_from_slice(stream);
    let mut data = MAGIC.to_vec();
    data.extend(seed);
    data.extend(xor(&body, &key(seed)));
    let mut out = vec![0u8; 24];
    for (i, n) in [meta.len(), png.len(), data.len()].into_iter().enumerate() {
        let s = n.to_string();
        out[8 * i..8 * i + s.len()].copy_from_slice(s.as_bytes());
    }
    out.extend_from_slice(meta);
    out.extend_from_slice(png);
    out.extend(data);
    out
}

/// Number of 256 KiB pieces in a Kraken stream: each starts with a 2-byte block header and a
/// 3-byte (big-endian) piece header whose low 18 bits are the packed size - 1; all ones there
/// means a piece filled with the one byte that follows.
fn pieces(s: &[u8]) -> Option<usize> {
    let (mut at, mut n) = (0usize, 0usize);
    while at < s.len() {
        let h = s.get(at..at + 5)?;
        if h[0] & 0x7F != 0x0C {
            return None;
        }
        let v = (h[2] as usize) << 16 | (h[3] as usize) << 8 | h[4] as usize;
        at += if v & 0x3FFFF == 0x3FFFF { 6 } else { 5 + (v & 0x3FFFF) + 1 };
        n += 1;
    }
    (at == s.len() && n > 0).then_some(n)
}

/// Why a file couldn't be opened, when it's this game's but Oodle is missing.
pub fn needs_oodle(d: &[u8]) -> bool {
    split(d).and_then(|p| pieces(&p.stream)).is_some() && crate::oodle::location().is_none()
}

pub fn unpack(d: &[u8]) -> Option<(Wrap, Vec<u8>)> {
    let p = split(d)?;
    let n = pieces(&p.stream)?;
    let plain = crate::oodle::decompress(&p.stream, n * PIECE)?;
    let wrap = Wrap { original: Arc::new(d.to_vec()), meta: p.meta, png: p.png, seed: p.seed, account: p.account, plain: Arc::new(plain.clone()) };
    Some((wrap, plain))
}

/// The MD5 of the data block at 0x1EC (the size of the block at 0x1FC).
fn fix_md5(p: &mut [u8]) {
    if p.len() < 0x200 {
        return;
    }
    let n = u32::from_le_bytes(p[0x1FC..0x200].try_into().unwrap()) as usize;
    if 0x200 + n > p.len() {
        return;
    }
    use md5::Digest;
    let h: [u8; 16] = md5::Md5::digest(&p[0x200..0x200 + n]).into();
    p[0x1EC..0x1FC].copy_from_slice(&h);
}

pub fn pack(w: &Wrap, data: &[u8]) -> Result<Vec<u8>, String> {
    if data == w.plain.as_slice() {
        return Ok(w.original.as_ref().clone());
    }
    if data.len() != w.plain.len() {
        return Err("the game data changed size".into());
    }
    let mut p = data.to_vec();
    fix_md5(&mut p);
    let stream = crate::oodle::compress(&p).ok_or("Oodle couldn't pack the save again (no Oodle library found)")?;
    Ok(join(&w.meta, &w.png, w.seed, &w.account, &stream))
}

// ------------------------------------------------------------------ game data

fn facility_table(p: &[u8]) -> Option<usize> {
    let at = p.windows(FACILITY_MARK.len()).position(|w| w == FACILITY_MARK)?;
    Some(at + 13)
}

pub fn looks_data(p: &[u8]) -> bool {
    p.len() > 0x200 && u32::from_le_bytes(p[0x1FC..0x200].try_into().unwrap()) as usize + 0x200 <= p.len() && {
        let mut q = p[..0x200 + u32::from_le_bytes(p[0x1FC..0x200].try_into().unwrap()) as usize].to_vec();
        let before = q[0x1EC..0x1FC].to_vec();
        fix_md5(&mut q);
        q[0x1EC..0x1FC] == before[..]
    } && facility_table(p).is_some()
}

/// Facility code, index and connection level of record `k`, if it is one.
fn record(p: &[u8], table: usize, k: usize) -> Option<(u32, u16, u16)> {
    let at = table + RECORD * k + 0x94;
    let b = p.get(at..at + 8)?;
    let code = u32::from_le_bytes(b[..4].try_into().unwrap());
    let index = u16::from_le_bytes(b[4..6].try_into().unwrap());
    let level = u16::from_le_bytes(b[6..8].try_into().unwrap());
    ((100..1000).contains(&code) && index as usize == k && level <= 5).then_some((code, index, level))
}

/// Facility name for record `k` (the game's code when the list has none).
pub fn facility_name(k: usize, code: u32) -> String {
    match FACILITIES.get(k) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => format!("Facility {code}"),
    }
}

pub fn parse(p: &[u8]) -> Result<TreeDoc, String> {
    let table = facility_table(p).ok_or("not Death Stranding game data")?;
    let mut b = Builder::new(p);
    let mut connected = 0;
    for k in 0..64 {
        let Some((code, _, level)) = record(p, table, k) else { break };
        let at = table + RECORD * k + 0x34;
        let stocks: Vec<u32> = (0..6).map(|i| u32::from_le_bytes(p[at + 8 * i..at + 8 * i + 4].try_into().unwrap())).collect();
        if level == 0 && stocks.iter().all(|&s| s == 0) {
            continue;
        }
        if level > 0 {
            connected += 1;
        }
        let name = facility_name(k, code);
        b.info(&["Facilities", &name, "Connection level"], level as u32);
        for (i, m) in MATERIALS.iter().enumerate() {
            b.ranged(&["Facilities", &name, m], at + 8 * i, Kind::U32, 0, 9_999_999);
        }
    }
    b.info(&["Save", "Connected facilities"], connected);
    Ok(b.finish(KIND, Checksum::None).with_note("Each facility's material stocks; the data's MD5 is written again on save."))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A data image with an MD5 header and a facility table of three records.
    pub(super) fn image() -> Vec<u8> {
        let mut p = vec![0u8; 0x200 + 4096];
        let n = 4000u32;
        p[0x1FC..0x200].copy_from_slice(&n.to_le_bytes());
        let mark = 0x400;
        p[mark..mark + 10].copy_from_slice(&FACILITY_MARK);
        let table = mark + 13;
        for (k, (code, level, metals)) in [(102u32, 3u16, 2855u32), (104, 0, 0), (105, 4, 4934)].into_iter().enumerate() {
            let r = table + RECORD * k;
            p[r + 0x94..r + 0x98].copy_from_slice(&code.to_le_bytes());
            p[r + 0x98..r + 0x9A].copy_from_slice(&(k as u16).to_le_bytes());
            p[r + 0x9A..r + 0x9C].copy_from_slice(&level.to_le_bytes());
            p[r + 0x34 + 16..r + 0x34 + 20].copy_from_slice(&metals.to_le_bytes());
        }
        fix_md5(&mut p);
        p
    }

    #[test]
    fn reads_facilities_and_rewrites_the_md5() {
        let p = image();
        assert!(looks_data(&p));
        let mut doc = parse(&p).unwrap();
        assert_eq!(doc.view.pointer("/Facilities/Capital Knot City/Metals").unwrap(), 2855);
        assert_eq!(doc.view.pointer("/Facilities/Capital Knot City/Connection level").unwrap(), 3);
        assert_eq!(doc.view.pointer("/Facilities/Distribution Center West of Capital Knot City/Metals").unwrap(), 4934);
        assert!(doc.view.pointer("/Facilities/Waystation West of Capital Knot City").is_none(), "unconnected and empty");
        assert_eq!(doc.view.pointer("/Save/Connected facilities").unwrap(), 2);
        doc.set("/Facilities/Capital Knot City/Metals", serde_json::Value::from(4000)).unwrap();
        let mut edited = doc.to_bytes().unwrap();
        assert!(!looks_data(&edited), "the MD5 no longer matches until it is written again");
        fix_md5(&mut edited);
        assert!(looks_data(&edited));
        assert_eq!(parse(&edited).unwrap().view.pointer("/Facilities/Capital Knot City/Metals").unwrap(), 4000);
    }

    #[test]
    fn container_splits_and_joins_back() {
        // A stream of two pieces: one packed (5-byte header + 3 bytes), one filled with zeros.
        let stream = [0x8C, 0x06, 0x00, 0x00, 0x02, 1, 2, 3, 0x0C, 0x06, 0x07, 0xFF, 0xFF, 0x00];
        let account = [&5u32.to_le_bytes()[..], b"steam"].concat();
        let file = join(b"[Slot]\r\nTitle = x - 01:02:03\r\n", b"\x89PNG....", [1, 2, 3, 4], &account, &stream);
        assert!(looks(&file));
        assert!(description(&file).unwrap().contains("01:02:03"));
        let p = split(&file).unwrap();
        assert_eq!(p.account, account);
        assert_eq!(p.stream, stream);
        assert_eq!(pieces(&p.stream), Some(2));
        assert_eq!(join(&p.meta, &p.png, p.seed, &p.account, &p.stream), file);
        // The game leaves bytes after the NUL in the size fields; they are ignored.
        let mut f = file.clone();
        f[7] = b'x';
        let digits = file[..8].iter().position(|&c| c == 0).unwrap();
        assert!(digits < 7 && looks(&f));
    }

    /// `SAVE_EDITOR_DS1_IMAGE=<decompressed image>`: prints what's read and checks the MD5.
    #[test]
    #[ignore]
    fn research_real_image() {
        let path = std::env::var("SAVE_EDITOR_DS1_IMAGE").expect("set SAVE_EDITOR_DS1_IMAGE");
        let p = std::fs::read(path).unwrap();
        assert!(looks_data(&p), "MD5 and facility table found");
        let doc = parse(&p).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view).unwrap());
    }

    /// `SAVE_EDITOR_DS1=<checkpoint.dat>`: splits the container and checks the piece headers.
    #[test]
    #[ignore]
    fn research_real_container() {
        let path = std::env::var("SAVE_EDITOR_DS1").expect("set SAVE_EDITOR_DS1");
        let d = std::fs::read(path).unwrap();
        let p = split(&d).expect("splits");
        let n = pieces(&p.stream).expect("Kraken pieces");
        println!("{} pieces, account id {} bytes", n, p.account.len());
        // The size fields can hold leftover bytes after their NUL; everything else is the same.
        let back = join(&p.meta, &p.png, p.seed, &p.account, &p.stream);
        assert_eq!(sizes(&back), sizes(&d));
        assert_eq!(back[24..], d[24..]);
    }
}
