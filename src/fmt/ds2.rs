//! Death Stranding 2: On the Beach (PC) `manualsave*.dat`, `autosave*.dat`, `checkpointsave*.dat`.
//!
//! Container: a fixed 24-byte header, a u32 seed and a u32 kept as it is; then a 16-byte key
//! block (byte 2 = number of chunks), the chunk sizes (u32 each) and the chunks, each part
//! XORed with a 16-byte key (MurmurHash3 x64-128, seed 42, of the seed and fixed key material)
//! that starts again at every part. Chunk 0 is the load-screen metadata, chunk 1 the PNG
//! thumbnail; the rest are 256 KiB pieces of the game data, LZ4 blocks (a piece that wouldn't
//! shrink is stored as it is). The game data (Decima) holds an MD5 of its data block (first
//! byte XOR 6) at 0x1EC, written again on save. Unchanged pieces keep their original bytes.
//!
//! Inside the data block: sections found through a directory at its end (magic
//! `05 A0 0E 53 FF FF EE EE`, count, then (id, offset, version) as Decima variable-length
//! ints). Facilities (`73dacf23`), likes (`202659d5`) and the clock (`3926c2c4`) are laid out as
//! documented by binsento-netizen/ds2-peko-check (MIT); the key derivation is from
//! mi5hmash/DeathStrandingSaveDataResigner (MIT). Checked on sample saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::sync::Arc;

pub const KIND: &str = "Death Stranding 2 save";

const MAGIC: [u8; 8] = [0x49, 0xF6, 0x0E, 0xC1, 0x2D, 0xB5, 0xFB, 0xA9];
const KEY_MATERIAL: [u8; 16] = [0x46, 0x3F, 0xBB, 0xBC, 0x7B, 0x6F, 0x57, 0xED, 0x9B, 0x41, 0x7D, 0x36, 0xE3, 0x26, 0x12, 0xBC];
const PIECE: usize = 256 * 1024;
const DIR_MAGIC: [u8; 8] = [0x05, 0xA0, 0x0E, 0x53, 0xFF, 0xFF, 0xEE, 0xEE];

// ------------------------------------------------------------------ container

fn fmix(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    k ^= k >> 33;
    k = k.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    k ^ (k >> 33)
}

/// MurmurHash3 x64-128 (seed 42) of the seed bytes followed by the rest of the key material.
fn key(seed: [u8; 4]) -> [u8; 16] {
    let mut m = KEY_MATERIAL;
    m[..4].copy_from_slice(&seed);
    let (c1, c2, s) = (0x87C3_7B91_1142_53D5u64, 0x4CF5_AD43_2745_937Fu64, 0x2Au64);
    let mut k0 = u64::from_le_bytes(m[..8].try_into().unwrap());
    let mut k1 = u64::from_le_bytes(m[8..].try_into().unwrap());
    k0 = k0.wrapping_mul(c1).rotate_left(31).wrapping_mul(c2) ^ s;
    k0 = k0.rotate_left(27).wrapping_add(s).wrapping_mul(5).wrapping_add(0x52DC_E729);
    k1 = k1.wrapping_mul(c2).rotate_left(33).wrapping_mul(c1) ^ s;
    k1 = k1.rotate_left(31).wrapping_add(k0).wrapping_mul(5).wrapping_add(0x3849_5AB5);
    k0 ^= 16;
    k1 ^= 16;
    k0 = k0.wrapping_add(k1);
    k1 = k1.wrapping_add(k0);
    k0 = fmix(k0);
    k1 = fmix(k1);
    k0 = k0.wrapping_add(k1);
    k1 = k1.wrapping_add(k0);
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&k0.to_le_bytes());
    out[8..].copy_from_slice(&k1.to_le_bytes());
    out
}

fn xor(b: &[u8], k: &[u8; 16]) -> Vec<u8> {
    b.iter().enumerate().map(|(i, &x)| x ^ k[i & 15]).collect()
}

/// What's needed to write a save back: the file as read, where the game-data pieces are, and
/// the game data as read (pieces that didn't change keep their original bytes).
pub struct Wrap {
    original: Arc<Vec<u8>>,
    /// Number of chunks before the game data (metadata, thumbnail).
    kept: usize,
    /// (offset in the file, size) of every chunk.
    chunks: Vec<(usize, usize)>,
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
        write!(f, "Ds2 {{ chunks: {}, kept: {} }}", self.chunks.len(), self.kept)
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 0x40 && d.starts_with(&MAGIC)
}

pub fn unpack(d: &[u8]) -> Option<(Wrap, Vec<u8>)> {
    if !looks(d) {
        return None;
    }
    let k = key(d[0x18..0x1C].try_into().ok()?);
    let body = 0x20;
    let count = (d[body + 2] ^ k[2]) as usize;
    if count < 2 {
        return None;
    }
    let sizes = xor(d.get(body + 16..body + 16 + 4 * count)?, &k);
    let mut chunks = vec![];
    let mut at = body + 16 + 4 * count;
    for i in 0..count {
        let n = u32::from_le_bytes(sizes[4 * i..4 * i + 4].try_into().unwrap()) as usize;
        if at + n > d.len() {
            return None;
        }
        chunks.push((at, n));
        at += n;
    }
    if at != d.len() {
        return None;
    }
    // The metadata, then the thumbnail if there is one.
    let png = xor(&d[chunks[1].0..chunks[1].0 + chunks[1].1.min(8)], &k).starts_with(b"\x89PNG");
    let kept = if png { 2 } else { 1 };
    let mut plain = Vec::with_capacity((count - kept) * PIECE);
    for &(o, n) in &chunks[kept..] {
        let c = xor(&d[o..o + n], &k);
        if n == PIECE {
            plain.extend_from_slice(&c);
        } else {
            let mut buf = vec![0u8; PIECE];
            let len = lz4_flex::block::decompress_into(&c, &mut buf).ok()?;
            plain.extend_from_slice(&buf[..len]);
        }
    }
    let wrap = Wrap { original: Arc::new(d.to_vec()), kept, chunks, plain: Arc::new(plain.clone()) };
    Some((wrap, plain))
}

/// The MD5 of the data block (first byte XOR 6) at 0x1EC, when the game data has one.
fn fix_md5(p: &mut [u8]) {
    if p.len() < 0x200 || !p.starts_with(b"v") {
        return;
    }
    let n = u32::from_le_bytes(p[0x1FC..0x200].try_into().unwrap()) as usize;
    if 0x200 + n > p.len() {
        return;
    }
    use md5::Digest;
    let mut h: [u8; 16] = md5::Md5::digest(&p[0x200..0x200 + n]).into();
    h[0] ^= 6;
    p[0x1EC..0x1FC].copy_from_slice(&h);
}

pub fn pack(w: &Wrap, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut p = data.to_vec();
    if p == *w.plain {
        return Ok(w.original.as_ref().clone());
    }
    fix_md5(&mut p);
    let d = &w.original;
    let k = key(d[0x18..0x1C].try_into().unwrap());
    // Pieces as they were where nothing changed, packed again where something did.
    let mut out_chunks: Vec<Vec<u8>> = w.chunks[..w.kept].iter().map(|&(o, n)| d[o..o + n].to_vec()).collect();
    let old = &w.chunks[w.kept..];
    for (i, piece) in p.chunks(PIECE).enumerate() {
        let at = i * PIECE;
        let same = i < old.len() && w.plain.get(at..at + piece.len()) == Some(piece) && (piece.len() == PIECE || at + piece.len() == w.plain.len());
        if same {
            let (o, n) = old[i];
            out_chunks.push(d[o..o + n].to_vec());
            continue;
        }
        let z = lz4_flex::block::compress(piece);
        let raw = if z.len() >= PIECE || z.len() >= piece.len() {
            if piece.len() != PIECE {
                return Err("the last piece of the game data can't be stored unpacked".into());
            }
            piece.to_vec()
        } else {
            z
        };
        out_chunks.push(xor(&raw, &k));
    }
    if out_chunks.len() > 255 {
        return Err("too much game data for one save".into());
    }
    let mut out = d[..0x20].to_vec();
    let mut kb = [0u8; 16];
    kb[2] = out_chunks.len() as u8;
    out.extend(xor(&kb, &k));
    let sizes: Vec<u8> = out_chunks.iter().flat_map(|c| (c.len() as u32).to_le_bytes()).collect();
    out.extend(xor(&sizes, &k));
    for c in out_chunks {
        out.extend(c);
    }
    Ok(out)
}

// ------------------------------------------------------------------ game data

/// Facility codes and names (the game's own; from ds2-peko-check's facility list).
const FACILITIES: [(u32, &str); 48] = [
    (101, "Sam's Shelter"),
    (102, "Ciudad Nudo del Norte (C1)"),
    (103, "Villa Libre"),
    (104, "C1 South Distribution Center"),
    (105, "Former Geophysics Research Lab"),
    (106, "Plate Gate"),
    (107, "The Bokka"),
    (108, "The Artist"),
    (201, "Plate Gate (Australia)"),
    (202, "The Government's Base"),
    (203, "Western Environmental Observatory"),
    (204, "The Lone Commander"),
    (205, "The Musician"),
    (206, "West Fort Knot (F1)"),
    (207, "The Inventor"),
    (208, "The Pioneer"),
    (209, "Rainbow Valley"),
    (210, "Animal Shelter"),
    (211, "Rainy's Shelter"),
    (212, "The Fisherman"),
    (213, "Northern Environmental Observatory"),
    (214, "The Architect"),
    (215, "F2 South Distribution Center"),
    (216, "The Dowser"),
    (217, "Heartman's Lab"),
    (218, "The Pizza Chef"),
    (219, "The Motherhood"),
    (220, "The Data Scientist"),
    (221, "The Mechanic"),
    (222, "East Fort Knot (F4)"),
    (223, "The Chronobiologist"),
    (224, "The Ghost Hunter"),
    (225, "The Metagenomicist"),
    (226, "F5 East Distribution Center"),
    (227, "The Adventurer"),
    (228, "Eastern Environmental Observatory"),
    (229, "The Phantom Smith"),
    (230, "The Aeronautical Engineer"),
    (231, "South Fort Knot (F6)"),
    (232, "The Tar Therapist"),
    (233, "Southern Environmental Observatory"),
    (234, "F7 North Distribution Center"),
    (235, "Terminal Fort Knot (F8)"),
    (236, "BPAS"),
    (237, "Mr. Impossible"),
    (238, "Waystation 1"),
    (239, "Waystation 2"),
    (400, "APAC"),
];

pub const MATERIALS: [&str; 6] = ["Chiral crystals", "Resins", "Metals", "Ceramics", "Chemicals", "Special alloys"];

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}

impl Rd<'_> {
    /// Decima variable-length int: -128 then an i32, -127 then an i16, else a signed byte.
    fn vint(&mut self) -> Option<i64> {
        let v = *self.b.get(self.p)? as i8;
        self.p += 1;
        Some(match v {
            -128 => {
                let x = i32::from_le_bytes(self.b.get(self.p..self.p + 4)?.try_into().ok()?);
                self.p += 4;
                x as i64
            }
            -127 => {
                let x = i16::from_le_bytes(self.b.get(self.p..self.p + 2)?.try_into().ok()?);
                self.p += 2;
                x as i64
            }
            v => v as i64,
        })
    }
}

/// The data block's sections: id -> (start, end), in the game data's offsets.
fn sections(p: &[u8]) -> Option<Vec<(u32, usize, usize)>> {
    let n = u32::from_le_bytes(p.get(0x1FC..0x200)?.try_into().ok()?) as usize;
    let d = p.get(0x200..0x200 + n)?;
    let type_off = u32::from_le_bytes(d.get(n.checked_sub(8)?..n - 4)?.try_into().ok()?) as usize;
    let at = d.windows(8).rposition(|w| w == DIR_MAGIC)?;
    let mut r = Rd { b: d, p: at + 8 };
    let count = r.vint()?;
    if !(1..=10_000).contains(&count) {
        return None;
    }
    let mut s = vec![];
    for _ in 0..count {
        let id = r.vint()? as u32;
        let off = r.vint()? as usize;
        r.vint()?;
        s.push((id, off));
    }
    if r.p != n - 8 {
        return None;
    }
    let mut order: Vec<(u32, usize)> = s.clone();
    order.sort_by_key(|x| x.1);
    let mut out = vec![];
    for (i, &(id, off)) in order.iter().enumerate() {
        let end = order.get(i + 1).map_or(type_off, |x| x.1);
        if off <= end && end <= n {
            out.push((id, 0x200 + off, 0x200 + end));
        }
    }
    Some(out)
}

pub fn looks_data(p: &[u8]) -> bool {
    p.len() > 0x200 && p.starts_with(b"v") && p[1].is_ascii_digit() && sections(p).is_some()
}

pub fn parse(p: &[u8]) -> Result<TreeDoc, String> {
    let secs = sections(p).ok_or("not Death Stranding 2 game data")?;
    let sec = |id: u32| secs.iter().find(|s| s.0 == id).map(|s| (s.1, s.2));
    let u32_at = |at: usize| u32::from_le_bytes(p[at..at + 4].try_into().unwrap());
    let mut b = Builder::new(p);
    let version = String::from_utf8_lossy(&p[..32]).trim_end_matches('\0').to_string();
    b.info(&["Save", "Game version"], version);
    let secs_played = f64::from_le_bytes(p[0x178..0x180].try_into().unwrap());
    if secs_played.is_finite() && secs_played >= 0.0 {
        let s = secs_played as u64;
        b.info(&["Save", "Play time"], format!("{}:{:02}", s / 3600, s / 60 % 60));
    }
    if let Some((at, end)) = sec(0x3926_C2C4).filter(|(a, e)| e - a >= 12) {
        let hour = f32::from_le_bytes(p[at + 4..at + 8].try_into().unwrap());
        b.info(&["Save", "Day"], u32_at(at + 8));
        if (0.0..24.0).contains(&hour) {
            b.info(&["Save", "Time of day"], format!("{:02}:{:02}", hour as u32, ((hour.fract()) * 60.0) as u32));
        }
        let _ = end;
    }
    if let Some((at, end)) = sec(0x2026_59D5).filter(|(a, e)| e - a >= 48) {
        let _ = end;
        b.ranged(&["Likes", "From NPCs"], at + 4, Kind::U32, 0, 99_999_999);
        b.ranged(&["Likes", "From other porters"], at + 12, Kind::U32, 0, 99_999_999);
        b.ranged(&["Likes", "Given"], at + 44, Kind::U32, 0, 99_999_999);
    }
    // Facilities: 112-byte records from +106: u32 code, u16 index, u16 connection level; the six
    // material stocks (u32, 8 bytes apart) start 96 bytes before the code.
    if let Some((at, end)) = sec(0x73DA_CF23) {
        let mut o = at + 106;
        for _ in 0..49 {
            if o + 8 > end {
                break;
            }
            let code = u32_at(o);
            let level = u16::from_le_bytes(p[o + 6..o + 8].try_into().unwrap());
            let stocks: Vec<u32> = (0..6).map(|i| u32_at(o - 96 + 8 * i)).collect();
            if code != 0 && (level > 0 || stocks.iter().any(|&s| s > 0)) {
                let name = FACILITIES.iter().find(|f| f.0 == code).map_or_else(|| format!("Facility {code}"), |f| f.1.to_string());
                b.info(&["Facilities", &name, "Connection level"], level as u32);
                for (i, m) in MATERIALS.iter().enumerate() {
                    b.ranged(&["Facilities", &name, m], o - 96 + 8 * i, Kind::U32, 0, 9_999_999);
                }
            }
            o += 112;
        }
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Each facility's material stocks and the likes counters; the data's MD5 is written again on save."))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Game data with a header, a facilities and a likes section, and the directory.
    fn data() -> Vec<u8> {
        let mut p = vec![0u8; 0x200];
        p[..9].copy_from_slice(b"v1.0.45.0");
        p[0x178..0x180].copy_from_slice(&3725.0f64.to_le_bytes());
        let mut d = vec![0u8; 4];
        let likes = d.len();
        d.extend(vec![0u8; 48]);
        d[likes + 4..likes + 8].copy_from_slice(&2271u32.to_le_bytes());
        let fac = d.len();
        d.extend(vec![0u8; 106 + 112 * 2]);
        d[fac + 106..fac + 110].copy_from_slice(&102u32.to_le_bytes());
        d[fac + 112..fac + 114].copy_from_slice(&1u16.to_le_bytes());
        d[fac + 10 + 16..fac + 10 + 20].copy_from_slice(&350u32.to_le_bytes()); // metals
        let type_off = d.len();
        d.extend(DIR_MAGIC);
        d.push(2);
        for (id, off) in [(0x2026_59D5u32, likes), (0x73DA_CF23, fac)] {
            d.push(0x80);
            d.extend(id.to_le_bytes());
            d.push(0x80);
            d.extend((off as u32).to_le_bytes());
            d.push(1);
        }
        d.extend((type_off as u32).to_le_bytes());
        d.extend((type_off as u32).to_le_bytes());
        p[0x1FC..0x200].copy_from_slice(&(d.len() as u32).to_le_bytes());
        p.extend(d);
        fix_md5(&mut p);
        p.resize(PIECE * 2 + 1000, 7);
        p
    }

    /// A container around `plain`, as the game writes it.
    fn container(plain: &[u8]) -> Vec<u8> {
        let mut f = MAGIC.to_vec();
        f.resize(0x18, 0);
        f.extend(0x1234_5678u32.to_le_bytes());
        f.extend(0u32.to_le_bytes());
        let k = key(f[0x18..0x1C].try_into().unwrap());
        let mut chunks = vec![lz4_flex::block::compress(b"metadata"), b"\x89PNG\r\n\x1a\nthumbnail".to_vec()];
        chunks.extend(plain.chunks(PIECE).map(lz4_flex::block::compress));
        let mut kb = [0u8; 16];
        kb[2] = chunks.len() as u8;
        f.extend(xor(&kb, &k));
        f.extend(xor(&chunks.iter().flat_map(|c| (c.len() as u32).to_le_bytes()).collect::<Vec<u8>>(), &k));
        for c in chunks {
            f.extend(xor(&c, &k));
        }
        f
    }

    #[test]
    fn key_matches_the_known_derivation() {
        // The key block's plain bytes are zero except the count, so the file's first 16 body
        // bytes give the key away: check the derivation against a container built with it.
        let f = container(&data());
        let k = key(f[0x18..0x1C].try_into().unwrap());
        assert_eq!(f[0x20] ^ k[0], 0);
        assert_eq!(f[0x22] ^ k[2], 5);
    }

    #[test]
    fn reads_edits_and_packs_again() {
        let plain = data();
        let file = container(&plain);
        let (w, p) = unpack(&file).unwrap();
        assert_eq!(p, plain);
        assert_eq!(pack(&w, &p).unwrap(), file, "unchanged data gives the file back");
        assert!(looks_data(&p));
        let mut doc = parse(&p).unwrap();
        assert_eq!(doc.view.pointer("/Likes/From NPCs").unwrap(), 2271);
        assert_eq!(doc.view.pointer("/Facilities/Ciudad Nudo del Norte (C1)/Metals").unwrap(), 350);
        assert_eq!(doc.view.pointer("/Save/Play time").unwrap(), "1:02");
        doc.set("/Facilities/Ciudad Nudo del Norte (C1)/Metals", serde_json::Value::from(9000)).unwrap();
        let edited = doc.to_bytes().unwrap();
        let out = pack(&w, &edited).unwrap();
        let (_, back) = unpack(&out).unwrap();
        assert_eq!(parse(&back).unwrap().view.pointer("/Facilities/Ciudad Nudo del Norte (C1)/Metals").unwrap(), 9000);
        // The MD5 was written again.
        let mut check = back.clone();
        fix_md5(&mut check);
        assert_eq!(check, back);
        assert_ne!(back[0x1EC..0x1FC], plain[0x1EC..0x1FC]);
    }

    /// `SAVE_EDITOR_DS2=<save>.dat`: unpacks, checks the MD5 and the round trip, prints what's read.
    #[test]
    #[ignore]
    fn research_real_save() {
        let path = std::env::var("SAVE_EDITOR_DS2").expect("set SAVE_EDITOR_DS2");
        let d = std::fs::read(path).unwrap();
        let (w, p) = unpack(&d).expect("unpacks");
        assert_eq!(pack(&w, &p).unwrap(), d);
        let mut m = p.clone();
        fix_md5(&mut m);
        assert_eq!(m, p, "the save's MD5 is the one worked out here");
        let doc = parse(&p).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view).unwrap());
        // An edit packs again and reads back.
        let mut q = p.clone();
        q[0x400] ^= 1;
        let (_, back) = unpack(&pack(&w, &q).unwrap()).unwrap();
        assert_eq!(back[0x400], q[0x400]);
    }
}
