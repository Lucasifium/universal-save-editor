//! Final Fantasy XVI (PC) saves: `0001.png`…, `auto.png` and `system.png`.
//!
//! Each slot is a real PNG (the slot's picture) with the save in a private `faTh` chunk. The
//! chunk is a small archive: `u64` header size, `UMIF`, `u32` entry count, then per entry
//! `u32` name length, `u32` packed length, `u64` name offset, `u64` unpacked length and `u64`
//! data offset; names, then data, each padded to four bytes. `auto.png` holds the rotating
//! autosaves as ten entries ("game-save-data00"…), a slot file holds one.
//!
//! Names and data are each XORed with a fixed 8-byte key. The data is then a zlib stream
//! without its two header bytes that uses a preset 32 KiB dictionary: four bytes of dictionary
//! id (its Adler-32), deflate, and the Adler-32 of the unpacked save. The dictionary is part of
//! the game, so it is never shipped here: it is read from the player's own `ffxvi.exe` (found
//! by its Adler-32) when a save is opened. Writing needs no dictionary: the new stream simply
//! never refers to it.
//!
//! Worked out independently from saves and the game's own program; no third-party Final
//! Fantasy XVI tools or their documentation were used.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

pub const CHUNK: [u8; 4] = *b"faTh";
const MAGIC: &[u8; 4] = b"UMIF";
const KEY: [u8; 8] = [0xf3, 0xc4, 0x1f, 0x5f, 0xfe, 0x80, 0x3f, 0x0f];
const DICT_LEN: usize = 32768;

/// The game's XOR layer: 8-byte blocks, then a 4-, 2- and 1-byte tail, each from the key's start.
pub fn xor(d: &mut [u8]) {
    let mut i = 0;
    while i + 8 <= d.len() {
        for (b, k) in d[i..i + 8].iter_mut().zip(KEY) {
            *b ^= k;
        }
        i += 8;
    }
    for w in [4, 2, 1] {
        if d.len() & w != 0 {
            for (b, k) in d[i..i + w].iter_mut().zip(KEY) {
                *b ^= k;
            }
            i += w;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub unpacked: u64,
    /// The stored bytes, still XORed.
    pub data: Vec<u8>,
    /// The stored name bytes (XORed), kept as they were.
    name_raw: Vec<u8>,
}

fn u32_at(d: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(p..p + 4)?.try_into().ok()?))
}

fn u64_at(d: &[u8], p: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(p..p + 8)?.try_into().ok()?))
}

pub fn looks_archive(c: &[u8]) -> bool {
    c.len() >= 0x30 && &c[8..12] == MAGIC
}

/// The entries of a `faTh` chunk.
pub fn entries(c: &[u8]) -> Result<Vec<Entry>, String> {
    let bad = || "the Final Fantasy XVI save archive is damaged".to_string();
    if !looks_archive(c) {
        return Err(bad());
    }
    let n = u32_at(c, 12).ok_or_else(bad)? as usize;
    if n == 0 || n > 64 {
        return Err(bad());
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let e = 0x10 + i * 0x20;
        let name_len = u32_at(c, e).ok_or_else(bad)? as usize;
        let data_len = u32_at(c, e + 4).ok_or_else(bad)? as usize;
        let name_at = usize::try_from(u64_at(c, e + 8).ok_or_else(bad)?).map_err(|_| bad())?;
        let unpacked = u64_at(c, e + 16).ok_or_else(bad)?;
        let data_at = usize::try_from(u64_at(c, e + 24).ok_or_else(bad)?).map_err(|_| bad())?;
        let name_raw = c
            .get(name_at..name_at.checked_add(name_len).ok_or_else(bad)?)
            .ok_or_else(bad)?
            .to_vec();
        let data = c
            .get(data_at..data_at.checked_add(data_len).ok_or_else(bad)?)
            .ok_or_else(bad)?
            .to_vec();
        let mut name = name_raw.clone();
        xor(&mut name);
        let name =
            String::from_utf8_lossy(name.split(|&b| b == 0).next().unwrap_or(&[])).into_owned();
        out.push(Entry {
            name,
            unpacked,
            data,
            name_raw,
        });
    }
    Ok(out)
}

/// Builds a `faTh` chunk again, laid out as the game lays it out.
pub fn build(entries: &[Entry]) -> Vec<u8> {
    let pad = |v: &mut Vec<u8>| v.resize((v.len() + 3) & !3, 0);
    let head = 0x10 + entries.len() * 0x20;
    let mut body = vec![0u8; head];
    let mut names = vec![];
    for e in entries {
        names.push(body.len());
        body.extend(&e.name_raw);
        pad(&mut body);
    }
    let mut datas = vec![];
    for e in entries {
        datas.push(body.len());
        body.extend(&e.data);
        pad(&mut body);
    }
    body[0..8].copy_from_slice(&(head as u64).to_le_bytes());
    body[8..12].copy_from_slice(MAGIC);
    body[12..16].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    for (i, e) in entries.iter().enumerate() {
        let p = 0x10 + i * 0x20;
        body[p..p + 4].copy_from_slice(&(e.name_raw.len() as u32).to_le_bytes());
        body[p + 4..p + 8].copy_from_slice(&(e.data.len() as u32).to_le_bytes());
        body[p + 8..p + 16].copy_from_slice(&(names[i] as u64).to_le_bytes());
        body[p + 16..p + 24].copy_from_slice(&e.unpacked.to_le_bytes());
        body[p + 24..p + 32].copy_from_slice(&(datas[i] as u64).to_le_bytes());
    }
    body
}

fn adler32(d: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in d.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// Finds the 32 KiB window of `exe` whose Adler-32 is `id`.
pub fn find_dictionary(exe: &[u8], id: u32) -> Option<Vec<u8>> {
    const M: u64 = 65521;
    if exe.len() < DICT_LEN {
        return None;
    }
    let (mut a, mut b) = (1u64, 0u64);
    for &x in &exe[..DICT_LEN] {
        a = (a + x as u64) % M;
        b = (b + a) % M;
    }
    let want_a = (id & 0xffff) as u64;
    let want_b = (id >> 16) as u64;
    let n = DICT_LEN as u64 % M;
    let mut i = 0;
    loop {
        if a == want_a && b == want_b && adler32(&exe[i..i + DICT_LEN]) == id {
            return Some(exe[i..i + DICT_LEN].to_vec());
        }
        if i + DICT_LEN >= exe.len() {
            return None;
        }
        let out = exe[i] as u64;
        let inn = exe[i + DICT_LEN] as u64;
        a = (a + M - out + inn) % M;
        b = (b + M * M - (n * out) % M + a + M - 1) % M;
        i += 1;
    }
}

/// Where `ffxvi.exe` may be, best guess first.
fn exe_candidates() -> Vec<PathBuf> {
    let mut out = vec![];
    if let Some(p) = std::env::var_os("SAVE_EDITOR_FF16_EXE") {
        out.push(PathBuf::from(p));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(m) = memo_file().and_then(|m| std::fs::read_to_string(m).ok()) {
            out.push(PathBuf::from(m.trim()));
        }
        for lib in crate::games::steam_libraries() {
            out.push(
                lib.join("steamapps")
                    .join("common")
                    .join("FINAL FANTASY XVI")
                    .join("ffxvi.exe"),
            );
        }
        for var in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(pf) = std::env::var_os(var) {
                out.push(
                    PathBuf::from(pf)
                        .join("Epic Games")
                        .join("FINAL FANTASY XVI")
                        .join("ffxvi.exe"),
                );
            }
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn memo_file() -> Option<PathBuf> {
    crate::platform::config_dir().map(|d| d.join("ff16_game.txt"))
}

static FOUND: Mutex<Vec<(u32, Vec<u8>)>> = Mutex::new(Vec::new());

/// Why a save can't be opened when the game isn't found.
pub const MISSING: &str = "Final Fantasy XVI saves are packed with a dictionary that is part of the game, so the \
editor reads it from your own copy of ffxvi.exe. The game wasn't found in your Steam or Epic folders; set \
SAVE_EDITOR_FF16_EXE to the path of ffxvi.exe, or open the save on the computer the game is installed on.";

/// The game's dictionary with Adler-32 `id`, read from the player's installed game.
pub fn dictionary(id: u32) -> Result<Vec<u8>, String> {
    if let Some((_, d)) = FOUND
        .lock()
        .map_err(|_| MISSING.to_string())?
        .iter()
        .find(|(i, _)| *i == id)
    {
        return Ok(d.clone());
    }
    for p in exe_candidates() {
        let Ok(exe) = std::fs::read(&p) else { continue };
        if let Some(d) = find_dictionary(&exe, id) {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(m) = memo_file() {
                let _ = std::fs::create_dir_all(m.parent().unwrap_or(&m));
                let _ = std::fs::write(&m, p.to_string_lossy().as_bytes());
            }
            if let Ok(mut f) = FOUND.lock() {
                f.push((id, d.clone()));
            }
            return Ok(d);
        }
    }
    Err(MISSING.into())
}

/// Lets a known dictionary be used without looking for the game (tests, other tools).
pub fn remember_dictionary(dict: Vec<u8>) {
    let id = adler32(&dict);
    if let Ok(mut f) = FOUND.lock() {
        if !f.iter().any(|(i, _)| *i == id) {
            f.push((id, dict));
        }
    }
}

/// Unpacks one entry's data (still XORed) with the given dictionary.
pub fn unpack_with(stored: &[u8], dict: &[u8]) -> Result<Vec<u8>, String> {
    let mut d = stored.to_vec();
    xor(&mut d);
    if d.len() < 8 {
        return Err("the save is too short".into());
    }
    let id = u32::from_be_bytes(d[0..4].try_into().unwrap());
    if adler32(dict) != id {
        return Err("the save was packed with a different dictionary".into());
    }
    // A raw deflate stream that first writes the dictionary as stored blocks (not final),
    // then continues with the save's own blocks: the save's back-references into the
    // dictionary then resolve exactly as zlib's preset dictionary would.
    let mut s = Vec::with_capacity(dict.len() + d.len() + 16);
    for part in dict.chunks(0xffff) {
        s.push(0);
        s.extend((part.len() as u16).to_le_bytes());
        s.extend((!(part.len() as u16)).to_le_bytes());
        s.extend(part);
    }
    let body = &d[4..d.len() - 4];
    s.extend(body);
    let mut out = Vec::new();
    flate2::read::DeflateDecoder::new(&s[..])
        .read_to_end(&mut out)
        .map_err(|e| format!("the save couldn't be unpacked: {e}"))?;
    if out.len() < dict.len() {
        return Err("the save couldn't be unpacked".into());
    }
    let out = out.split_off(dict.len());
    let sum = u32::from_be_bytes(d[d.len() - 4..].try_into().unwrap());
    if adler32(&out) != sum {
        return Err("the save's checksum doesn't match".into());
    }
    Ok(out)
}

/// Packs data for an entry: dictionary id, deflate (not using the dictionary), Adler-32, XOR.
pub fn pack_with_id(data: &[u8], id: u32) -> Result<Vec<u8>, String> {
    let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(data).map_err(|e| e.to_string())?;
    let body = e.finish().map_err(|e| e.to_string())?;
    let mut out = id.to_be_bytes().to_vec();
    out.extend(body);
    out.extend(adler32(data).to_be_bytes());
    xor(&mut out);
    Ok(out)
}

/// The dictionary id an entry was packed with.
pub fn dictionary_id(stored: &[u8]) -> Option<u32> {
    let mut d = stored.get(0..4)?.to_vec();
    xor(&mut d);
    Some(u32::from_be_bytes(d[..4].try_into().ok()?))
}

/// The `faTh` chunk of a PNG.
pub fn chunk(png: &[u8]) -> Option<&[u8]> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut i = 8;
    while i + 12 <= png.len() {
        let len = u32::from_be_bytes(png[i..i + 4].try_into().ok()?) as usize;
        let kind = &png[i + 4..i + 8];
        let data = png.get(i + 8..i + 8 + len)?;
        if kind == CHUNK {
            return Some(data);
        }
        i += 12 + len;
    }
    None
}

pub fn looks(png: &[u8]) -> bool {
    chunk(png).is_some_and(looks_archive)
}

/// Unpacks entry `pick` (by name) or the first one: (entry index, every entry name, data).
pub fn unpack(png: &[u8], pick: Option<&str>) -> Result<(usize, Vec<String>, Vec<u8>), String> {
    let c = chunk(png).ok_or("no Final Fantasy XVI save data in this picture")?;
    let all = entries(c)?;
    let i = pick
        .and_then(|n| all.iter().position(|e| e.name == n))
        .unwrap_or(0);
    let id = dictionary_id(&all[i].data).ok_or("the save is too short")?;
    let dict = dictionary(id)?;
    let data = unpack_with(&all[i].data, &dict)?;
    if data.len() as u64 != all[i].unpacked {
        return Err("the save unpacked to an unexpected size".into());
    }
    Ok((i, all.iter().map(|e| e.name.clone()).collect(), data))
}

/// The unpacked game and system saves start with a 16-byte header: `u32` 16, the CRC-32 of
/// everything after the header, and a format version. The game skips a save whose CRC is wrong,
/// so it is written again here.
pub fn with_crc(data: &[u8]) -> Vec<u8> {
    let mut d = data.to_vec();
    if d.len() > 16 && d[0..4] == 16u32.to_le_bytes() {
        let mut h = flate2::Crc::new();
        h.update(&d[16..]);
        d[4..8].copy_from_slice(&h.sum().to_le_bytes());
    }
    d
}

/// Whether the save's own CRC-32 matches its data.
pub fn crc_ok(data: &[u8]) -> bool {
    data.len() <= 16 || data[0..4] != 16u32.to_le_bytes() || with_crc(data) == data
}

/// The new `faTh` chunk with entry `index` replaced by `data`.
pub fn repack_chunk(c: &[u8], index: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut all = entries(c)?;
    let e = all
        .get_mut(index)
        .ok_or("the save is missing from its archive")?;
    let id = dictionary_id(&e.data).ok_or("the save is too short")?;
    let data = with_crc(data);
    e.data = pack_with_id(&data, id)?;
    e.unpacked = data.len() as u64;
    Ok(build(&all))
}

// ------------------------------------------------------------------ the unpacked game save

pub const KIND: &str = "Final Fantasy XVI save";
const VERSION: u32 = 2019111101;
/// Item records (id, count, new-flag, 0), 16 bytes each, for gil, Ability Points, consumables,
/// materials, gear and curiosities alike.
const ITEMS: usize = 0x3C50;
const ITEM_SLOTS: usize = 1200;
const ITEM_COUNT: usize = 0x9A10;
const LEVEL: usize = 0x20;
const MAX_HP: usize = 0x30;
pub const GIL_MAX: i64 = 9_999_999;
pub const AP_MAX: i64 = 999_999;

/// Consumables: id, name, the most the satchel holds as the game showed it.
pub const CONSUMABLES: &[(u32, &str, i64)] =
    &[(5, "Potion", 8), (6, "High Potion", 5), (7, "Elixir", 2), (8, "Strength Tonic", 4), (9, "Stoneskin Tonic", 4)];

/// Materials: id, name and the stack size the Items menu shows for it.
pub const MATERIALS: &[(u32, &str, i64)] = &[
    (21, "Meteorite", 99), (31, "Scarletite", 99), (32, "Orichalcum", 99),
    (101, "Wyrrite", 9999), (102, "Sharp Fang", 9999), (103, "Steelsilk", 9999), (104, "Bloody Hide", 9999),
    (105, "Magicked Ash", 9999), (106, "Desert Rose", 999), (107, "Valley Madder", 999), (108, "Briar Clam Shell", 999),
    (109, "Clutchmine", 999), (110, "Iron Filings", 999),
    (151, "Gil Bug", 99), (152, "Black Blood", 99), (153, "Goblin Coin", 99), (154, "Empty Shard", 99), (155, "Amber", 99),
    (156, "Fallen Enigma", 99), (158, "Silver Penning", 99), (160, "Rhema", 9999),
    (201, "Wind Shard", 99), (202, "Fire Shard", 99), (203, "Lightning Shard", 99), (204, "Earth Shard", 99),
    (205, "Light Shard", 99), (206, "Water Shard", 99), (207, "Ice Shard", 99), (208, "Dark Shard", 99),
    (301, "Frozen Tear", 99), (302, "Gnarled Scales", 99), (303, "Clouded Eye", 99), (304, "Liquid Flame", 99),
    (305, "Earthen Fury", 99), (306, "Whitewyrm Bone", 99), (307, "Sanguine Insignia", 99), (308, "Behemoth Shackle", 99),
    (320, "Living Tissue", 99), (321, "Aqueous Humours", 99), (322, "Omega Nodule", 99), (326, "Aqueous Orb", 99),
    (327, "Bilious Scales", 99), (328, "Timeless Gardbrace", 99), (335, "Aquamarine", 99), (336, "Ageless Tallow", 99),
    (337, "Ageless Odorament", 99), (338, "Ageless Oilstone", 99), (339, "Ageless Millet", 99), (340, "Eludium", 99),
    (342, "Fool's Gold", 99),
    (1001, "Dragon Talon", 99), (1003, "Minotaur Mane", 99), (1005, "Imperial Link", 99), (1007, "Fallen Iron", 99),
    (1009, "Coeurl Whisker", 99), (1011, "Bone Necklace", 99),
    (1101, "Comet Feather", 99), (1103, "Grimalkin Hide", 99), (1105, "Bomb Ember", 99), (1107, "Darksteel", 99),
    (1109, "Morbol Flower", 99), (1111, "Stone Tongue", 99), (1113, "Stained Loincloth", 99), (1115, "Electrum", 99),
    (1117, "Gelatinous Mass", 99), (1123, "Primitive Battlehorn", 99), (1125, "Aqueous Membrane", 99),
    (1127, "Notched Tang", 99),
];

/// The stack size of the material called `name`.
pub fn material_cap(name: &str) -> Option<i64> {
    MATERIALS.iter().find(|m| m.1 == name).map(|m| m.2)
}

fn le32(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

/// The game save inside a slot picture (after `unpack`).
pub fn looks_save(d: &[u8]) -> bool {
    d.len() > ITEM_COUNT + 4 && le32(d, 0) == 16 && le32(d, 8) == VERSION
}

pub fn parse_save(d: &[u8]) -> Result<crate::tree::TreeDoc, String> {
    use super::fields::{Builder, Checksum, Kind};
    if !looks_save(d) {
        return Err("not a Final Fantasy XVI game save".into());
    }
    let mut b = Builder::new(d);
    b.info(&["Clive", "Level"], le32(d, LEVEL) as i64);
    b.info(&["Clive", "Max HP"], le32(d, MAX_HP) as i64);
    let n = (le32(d, ITEM_COUNT) as usize).min(ITEM_SLOTS);
    for i in 0..n {
        let at = ITEMS + i * 16;
        let id = le32(d, at);
        let (group, name, hi) = match id {
            1 => ("Clive", "Gil", GIL_MAX),
            3 => ("Clive", "Ability Points", AP_MAX),
            _ => {
                if let Some(c) = CONSUMABLES.iter().find(|c| c.0 == id) {
                    ("Consumables", c.1, c.2)
                } else if let Some(m) = MATERIALS.iter().find(|m| m.0 == id) {
                    ("Materials", m.1, m.2)
                } else {
                    continue;
                }
            }
        };
        b.ranged(&[group, name], at + 4, Kind::U32, 0, hi);
    }
    Ok(b.finish(KIND, Checksum::Custom(|d: &mut Vec<u8>| *d = with_crc(d))))
}

#[cfg(test)]
pub(crate) fn test_dictionary() -> Vec<u8> {
    (0..DICT_LEN)
        .map(|i| if i % 97 < 5 { (i * 31 % 251) as u8 } else { 0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_is_its_own_inverse_and_matches_the_tail_rule() {
        for n in [0, 1, 2, 3, 7, 8, 9, 15, 29506] {
            let d: Vec<u8> = (0..n).map(|i| (i * 7) as u8).collect();
            let mut x = d.clone();
            xor(&mut x);
            if n == 15 {
                // 8 + 4 + 2 + 1: the tail pieces restart at the key's first byte.
                assert_eq!(x[8] ^ d[8], KEY[0]);
                assert_eq!(x[12] ^ d[12], KEY[0]);
                assert_eq!(x[14] ^ d[14], KEY[0]);
            }
            xor(&mut x);
            assert_eq!(x, d);
        }
    }

    #[test]
    fn crc_is_written_again() {
        let mut d = vec![0u8; 64];
        d[0] = 16;
        d[20] = 5;
        assert!(!crc_ok(&d));
        let fixed = with_crc(&d);
        assert!(crc_ok(&fixed));
        let mut h = flate2::Crc::new();
        h.update(&d[16..]);
        assert_eq!(fixed[4..8], h.sum().to_le_bytes());
        // Data without the header is left alone.
        assert_eq!(with_crc(&[1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn rolling_search_finds_the_dictionary() {
        let dict = test_dictionary();
        let mut exe: Vec<u8> = (0..100_000u32)
            .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
            .collect();
        exe.splice(40_001..40_001, dict.iter().copied());
        assert_eq!(find_dictionary(&exe, adler32(&dict)), Some(dict));
    }

    fn archive(entries_data: &[(&str, &[u8])], id: u32) -> Vec<u8> {
        let all: Vec<Entry> = entries_data
            .iter()
            .map(|(n, d)| {
                let mut name_raw = n.as_bytes().to_vec();
                name_raw.push(0);
                xor(&mut name_raw);
                Entry {
                    name: n.to_string(),
                    unpacked: d.len() as u64,
                    data: pack_with_id(d, id).unwrap(),
                    name_raw,
                }
            })
            .collect();
        build(&all)
    }

    #[test]
    fn layout_matches_the_game() {
        let c = archive(&[("game-save-data", &[1, 2, 3])], 0x1234_5678);
        // One entry: names at 0x30 (15 bytes, padded to 16), data at 0x40.
        assert_eq!(u64_at(&c, 0), Some(0x30));
        assert_eq!(u64_at(&c, 0x18), Some(0x30));
        assert_eq!(u64_at(&c, 0x28), Some(0x40));
        assert_eq!(c.len() % 4, 0);
        let e = entries(&c).unwrap();
        assert_eq!(e[0].name, "game-save-data");
        assert_eq!(build(&e), c);
    }

    #[test]
    fn unpacks_a_stream_that_uses_the_dictionary() {
        let dict = test_dictionary();
        // 300 bytes copied from the dictionary's start, then "FF16".
        let mut want = dict[..300].to_vec();
        want.extend(b"FF16");
        // Build that stream by hand: a fixed-Huffman block with one long match into the dictionary.
        let stream = hand_stream_with_dictionary_match(&dict, &want);
        let mut stored = adler32(&dict).to_be_bytes().to_vec();
        stored.extend(stream);
        stored.extend(adler32(&want).to_be_bytes());
        xor(&mut stored);
        assert_eq!(unpack_with(&stored, &dict).unwrap(), want);
    }

    /// Fixed-Huffman deflate: copies 258+42 bytes from distance 32768 (the dictionary's start),
    /// then the literals "FF16".
    fn hand_stream_with_dictionary_match(dict: &[u8], want: &[u8]) -> Vec<u8> {
        struct W {
            out: Vec<u8>,
            acc: u64,
            n: u32,
        }
        impl W {
            fn bits(&mut self, v: u32, n: u32) {
                self.acc |= (v as u64) << self.n;
                self.n += n;
                while self.n >= 8 {
                    self.out.push(self.acc as u8);
                    self.acc >>= 8;
                    self.n -= 8;
                }
            }
            fn code(&mut self, c: u32, n: u32) {
                let r = (0..n).fold(0, |a, i| a | (((c >> i) & 1) << (n - 1 - i)));
                self.bits(r, n);
            }
        }
        let mut w = W {
            out: vec![],
            acc: 0,
            n: 0,
        };
        w.bits(1, 1);
        w.bits(1, 2);
        let lit = |w: &mut W, b: u8| {
            if b < 144 {
                w.code(0x30 + b as u32, 8)
            } else {
                w.code(0x190 + (b as u32 - 144), 9)
            }
        };
        let dist32768 = |w: &mut W, back: usize| {
            // Distance code 29: base 24577, 13 extra bits.
            w.code(29, 5);
            w.bits((back - 24577) as u32, 13);
        };
        // Length 258 (code 285, 8 bits: 11000101), then length 42 (code 273, base 35, 3 extra bits).
        w.code(0b11000101, 8);
        dist32768(&mut w, DICT_LEN);
        w.code(273 - 256, 7);
        w.bits(42 - 35, 3);
        dist32768(&mut w, DICT_LEN);
        for &b in &want[300..] {
            lit(&mut w, b);
        }
        w.code(0, 7);
        w.bits(0, 7);
        assert_eq!(&want[..300], &dict[..300]);
        w.out
    }

    #[test]
    fn repacked_entries_unpack_again() {
        let dict = test_dictionary();
        let id = adler32(&dict);
        let a: Vec<u8> = (0..5000u32).map(|i| (i % 13) as u8).collect();
        let b: Vec<u8> = vec![7; 999];
        let c = archive(&[("game-save-data00", &a), ("game-save-data01", &b)], id);
        let changed: Vec<u8> = a.iter().map(|x| x ^ 1).collect();
        let c2 = repack_chunk(&c, 1, &changed).unwrap();
        let e = entries(&c2).unwrap();
        assert_eq!(e[0].name, "game-save-data00");
        assert_eq!(unpack_with(&e[0].data, &dict).unwrap(), a);
        assert_eq!(unpack_with(&e[1].data, &dict).unwrap(), changed);
        assert_eq!(e[1].unpacked, changed.len() as u64);
    }

    #[test]
    fn reads_and_edits_the_game_save() {
        let mut d = vec![0u8; 0x10000];
        d[0..4].copy_from_slice(&16u32.to_le_bytes());
        d[8..12].copy_from_slice(&VERSION.to_le_bytes());
        d[LEVEL..LEVEL + 4].copy_from_slice(&49u32.to_le_bytes());
        let recs = [(1u32, 402883u32), (3, 5563), (5, 8), (109, 97), (1007, 6), (100045, 1)];
        for (i, (id, n)) in recs.iter().enumerate() {
            d[ITEMS + i * 16..ITEMS + i * 16 + 4].copy_from_slice(&id.to_le_bytes());
            d[ITEMS + i * 16 + 4..ITEMS + i * 16 + 8].copy_from_slice(&n.to_le_bytes());
        }
        d[ITEM_COUNT..ITEM_COUNT + 4].copy_from_slice(&(recs.len() as u32).to_le_bytes());
        let d = with_crc(&d);
        let mut doc = parse_save(&d).unwrap();
        let v = |doc: &crate::tree::TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Clive/Gil"), Some(402883.into()));
        assert_eq!(v(&doc, "/Clive/Ability Points"), Some(5563.into()));
        assert_eq!(v(&doc, "/Clive/Level"), Some(49.into()));
        assert_eq!(v(&doc, "/Consumables/Potion"), Some(8.into()));
        assert_eq!(v(&doc, "/Materials/Clutchmine"), Some(97.into()));
        assert_eq!(v(&doc, "/Materials/Fallen Iron"), Some(6.into()));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Materials/Clutchmine", 999.into()).unwrap();
        assert!(doc.set("/Materials/Fallen Iron", 100.into()).is_err());
        doc.set("/Clive/Gil", 111111.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(crc_ok(&out));
        let again = parse_save(&out).unwrap();
        assert_eq!(v(&again, "/Materials/Clutchmine"), Some(999.into()));
        assert_eq!(v(&again, "/Clive/Gil"), Some(111111.into()));
        assert_eq!(material_cap("Rhema"), Some(9999));
    }

    /// Real saves, kept out of the repository: FF16_SAMPLES = folder of slot PNGs, with
    /// SAVE_EDITOR_FF16_EXE pointing at the game's ffxvi.exe.
    #[test]
    #[ignore]
    fn real_saves_round_trip() {
        let Some(dir) = std::env::var_os("FF16_SAMPLES") else {
            return;
        };
        let mut n = 0;
        for f in std::fs::read_dir(dir).unwrap().flatten() {
            let png = std::fs::read(f.path()).unwrap();
            if !looks(&png) {
                continue;
            }
            let c = chunk(&png).unwrap();
            let all = entries(c).unwrap();
            assert_eq!(build(&all), c, "layout differs for {:?}", f.path());
            for (i, e) in all.iter().enumerate() {
                let (_, _, data) = unpack(&png, Some(&e.name)).unwrap();
                assert!(crc_ok(&data), "stored CRC-32 doesn't match for {:?}", f.path());
                if looks_save(&data) {
                    let doc = parse_save(&data).unwrap();
                    assert_eq!(doc.to_bytes().unwrap(), data);
                }
                let mut edited = data.clone();
                edited[0x24] ^= 1;
                let c2 = repack_chunk(c, i, &edited).unwrap();
                if let Some(out) = std::env::var_os("FF16_OUT") {
                    std::fs::write(std::path::Path::new(&out).join(format!("{n}.fath")), &c2)
                        .unwrap();
                }
                let back = entries(&c2).unwrap();
                let dict = dictionary(dictionary_id(&back[i].data).unwrap()).unwrap();
                let again = unpack_with(&back[i].data, &dict).unwrap();
                assert!(crc_ok(&again));
                assert_eq!(again, with_crc(&edited));
                n += 1;
            }
        }
        assert!(n > 0);
    }
}
