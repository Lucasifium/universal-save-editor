//! No Man's Sky `save*.hg` (Steam and GOG).
//!
//! The file is a run of chunks: the bytes `E5 A1 ED FE`, the packed size, the unpacked size
//! (0x80000 except in the last chunk) and a zero, then one LZ4 block. Together the chunks are
//! one JSON text followed by a zero byte. The object keys are scrambled to three characters
//! (`"F2P"` for `"Version"`); the table that turns them back is `data/nms_keys.tsv`, built from
//! the key mapping of libMBIN (MBINCompiler, LGPL-3.0, whose scrambling was worked out by
//! AndASM; see THIRD_PARTY_NOTICES.md). Keys are shown by their real names and scrambled again
//! on save; a key the table lacks is left as it is.
//!
//! Each save has a metadata file beside it (`mf_save*.hg`): XXTEA-encrypted words, with the
//! key made from the slot number as published by nmssavetool (MIT). Its unpacked-size word is
//! made again after a save is written (see `companions`); the older 2018 layout that also holds
//! a SpookyHash and SHA-256 is left alone.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const CHUNK_MAGIC: u32 = 0xFEED_A1E5;
const CHUNK: usize = 0x80000;
const META_MAGIC: u32 = 0xEEEE_EEBE;
const KEYS: &str = include_str!("data/nms_keys.tsv");

struct Names {
    /// Scrambled key → real name.
    plain: HashMap<&'static str, &'static str>,
    /// Real name → scrambled key.
    scrambled: HashMap<&'static str, &'static str>,
}

fn names() -> &'static Names {
    static N: OnceLock<Names> = OnceLock::new();
    N.get_or_init(|| {
        let mut plain = HashMap::new();
        let mut scrambled = HashMap::new();
        for l in KEYS.lines() {
            if let Some((k, v)) = l.split_once('\t') {
                plain.insert(k, v);
                scrambled.insert(v, k);
            }
        }
        Names { plain, scrambled }
    })
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 16 && d[..4] == CHUNK_MAGIC.to_le_bytes()
}

/// The unpacked JSON text of a save, or `None` when this is not one.
pub fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    if !looks(d) {
        return None;
    }
    let (mut at, mut out) = (0usize, Vec::new());
    while at < d.len() {
        let h = d.get(at..at + 16)?;
        if h[..4] != CHUNK_MAGIC.to_le_bytes() {
            return None;
        }
        let packed = u32::from_le_bytes(h[4..8].try_into().ok()?) as usize;
        let size = u32::from_le_bytes(h[8..12].try_into().ok()?) as usize;
        if size > CHUNK || packed > d.len() {
            return None;
        }
        let block = d.get(at + 16..at + 16 + packed)?;
        out.extend(lz4_flex::block::decompress(block, size).ok()?);
        at += 16 + packed;
    }
    while out.last() == Some(&0) {
        out.pop();
    }
    (out.first() == Some(&b'{')).then_some(out)
}

pub fn pack_chunks(text: &[u8]) -> Vec<u8> {
    let mut all = text.to_vec();
    all.push(0);
    let mut out = Vec::with_capacity(all.len() / 3);
    for part in all.chunks(CHUNK) {
        let block = lz4_flex::block::compress(part);
        out.extend_from_slice(&CHUNK_MAGIC.to_le_bytes());
        out.extend_from_slice(&(block.len() as u32).to_le_bytes());
        out.extend_from_slice(&(part.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&block);
    }
    out
}

fn rename(v: &mut serde_json::Value, map: &HashMap<&str, &str>) {
    match v {
        serde_json::Value::Object(o) => {
            let old = std::mem::take(o);
            let keys: std::collections::HashSet<String> = old.keys().cloned().collect();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for (k, mut val) in old {
                rename(&mut val, map);
                // Never merge two keys into one.
                let new = match map.get(k.as_str()) {
                    Some(n) if !keys.contains(*n) && !seen.contains(*n) => n.to_string(),
                    _ => k,
                };
                seen.insert(new.clone());
                o.insert(new, val);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(|x| rename(x, map)),
        _ => {}
    }
}

/// The save's JSON with real key names, and whether its keys were scrambled (so `pack` scrambles
/// them again); `None` when this is not a save.
pub fn open(d: &[u8]) -> Option<(Vec<u8>, bool)> {
    let text = unpack(d)?;
    let mut v: serde_json::Value = serde_json::from_slice(&text).ok()?;
    // Saves with real names already (older builds) have nothing to rename.
    let scrambled = v.get("F2P").is_some();
    if scrambled {
        rename(&mut v, &names().plain);
    }
    Some((serde_json::to_vec(&v).ok()?, scrambled))
}

/// The file bytes for the JSON text shown in the editor.
pub fn pack(json: &[u8], scrambled: bool) -> Result<Vec<u8>, String> {
    let mut v: serde_json::Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    if scrambled {
        rename(&mut v, &names().scrambled);
    }
    Ok(pack_chunks(&serde_json::to_vec(&v).map_err(|e| e.to_string())?))
}

// ---------------------------------------------------------------- metadata (mf_save*.hg)

const DELTA: u32 = 0x9E37_79B9;

fn mx(sum: u32, y: u32, z: u32, p: usize, e: u32, key: &[u32; 4]) -> u32 {
    (((z >> 5) ^ (y << 2)).wrapping_add((y >> 3) ^ (z << 4))) ^ ((sum ^ y).wrapping_add(key[(p & 3) ^ e as usize] ^ z))
}

fn xxtea_decrypt(w: &mut [u32], key: &[u32; 4], rounds: u32) {
    let n = w.len();
    let mut sum = rounds.wrapping_mul(DELTA);
    let mut y = w[0];
    for _ in 0..rounds {
        let e = (sum >> 2) & 3;
        for p in (1..n).rev() {
            let z = w[p - 1];
            w[p] = w[p].wrapping_sub(mx(sum, y, z, p, e, key));
            y = w[p];
        }
        let z = w[n - 1];
        w[0] = w[0].wrapping_sub(mx(sum, y, z, 0, e, key));
        y = w[0];
        sum = sum.wrapping_sub(DELTA);
    }
}

fn xxtea_encrypt(w: &mut [u32], key: &[u32; 4], rounds: u32) {
    let n = w.len();
    let mut sum = 0u32;
    let mut z = w[n - 1];
    for _ in 0..rounds {
        sum = sum.wrapping_add(DELTA);
        let e = (sum >> 2) & 3;
        for p in 0..n - 1 {
            let y = w[p + 1];
            w[p] = w[p].wrapping_add(mx(sum, y, z, p, e, key));
            z = w[p];
        }
        let y = w[0];
        w[n - 1] = w[n - 1].wrapping_add(mx(sum, y, z, n - 1, e, key));
        z = w[n - 1];
    }
}

/// The key of the metadata file with storage index `slot` (save.hg is 2, save2.hg is 3, …).
fn meta_key(slot: u32) -> [u32; 4] {
    let k1 = slot ^ 0x1422_CB8C;
    let h1 = k1.rotate_left(13).wrapping_mul(5).wrapping_add(0xE654_6B64);
    let mut kb = *b"NAESEVADNAYRTNRG";
    kb[..4].copy_from_slice(&h1.to_le_bytes());
    let mut key = [0u32; 4];
    for (i, k) in key.iter_mut().enumerate() {
        *k = u32::from_le_bytes(kb[i * 4..i * 4 + 4].try_into().unwrap());
    }
    key
}

fn meta_rounds(len: usize) -> u32 {
    if len == 0x68 {
        8
    } else {
        6
    }
}

/// Storage index of a save file name: `save.hg` 2, `save2.hg` 3, `save3.hg` 4…
fn slot_of(name: &str) -> Option<u32> {
    let rest = name.strip_prefix("save")?.strip_suffix(".hg")?;
    if rest.is_empty() {
        return Some(2);
    }
    rest.parse::<u32>().ok().filter(|n| (2..=30).contains(n)).map(|n| n + 1)
}

/// The metadata file with its unpacked-size word made again for `new_text_len` bytes of JSON.
pub fn updated_meta(meta: &[u8], slot: u32, text_len: usize, packed_len: usize) -> Option<Vec<u8>> {
    if !matches!(meta.len(), 0x68 | 0x168 | 0x180 | 0x1B0) {
        return None;
    }
    let rounds = meta_rounds(meta.len());
    let words: Vec<u32> = meta.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    // The slot number is what the game used; try the others in case the file was renamed.
    let (key_slot, mut plain) = std::iter::once(slot)
        .chain((2..=31).filter(move |s| *s != slot))
        .find_map(|s| {
            let mut w = words.clone();
            xxtea_decrypt(&mut w, &meta_key(s), rounds);
            (w[0] == META_MAGIC).then_some((s, w))
        })?;
    // Format 2001 also holds a SpookyHash and a SHA-256 of the save: not made here.
    if plain[1] == 0x7D1 {
        return None;
    }
    let before = plain.clone();
    plain[14] = (text_len + 1) as u32;
    if plain[15] != 0 {
        plain[15] = packed_len as u32;
    }
    if plain == before {
        return None;
    }
    xxtea_encrypt(&mut plain, &meta_key(key_slot), rounds);
    Some(plain.iter().flat_map(|w| w.to_le_bytes()).collect())
}

/// After `saveN.hg` is written: the matching `mf_saveN.hg` is made to agree with it.
pub fn companions(path: &Path, bytes: &[u8]) -> Vec<(PathBuf, Vec<u8>)> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return vec![] };
    let Some(slot) = slot_of(name) else { return vec![] };
    let Some(text) = unpack(bytes) else { return vec![] };
    let mf = path.with_file_name(format!("mf_{name}"));
    let Ok(meta) = std::fs::read(&mf) else { return vec![] };
    updated_meta(&meta, slot, text.len(), bytes.len()).map(|m| vec![(mf, m)]).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"F2P":4650,"8>q":"Win|Final","6f=":{"wGS":123,";l5":{":No":[{"b2n":"^FUEL1","1o9":50,"F9q":250}]}}}"#;

    #[test]
    fn chunks_round_trip_across_a_chunk_boundary() {
        let text = format!("{{\"a\":\"{}\"}}", "xyz".repeat(300_000));
        let packed = pack_chunks(text.as_bytes());
        assert!(packed.len() > 16);
        assert_eq!(unpack(&packed).unwrap(), text.as_bytes());
    }

    #[test]
    fn keys_are_shown_by_name_and_scrambled_again() {
        let file = pack_chunks(SAMPLE.as_bytes());
        let (shown, was) = open(&file).unwrap();
        assert!(was);
        let shown = String::from_utf8(shown).unwrap();
        assert!(shown.contains("\"Version\":4650") && shown.contains("\"PlayerStateData\"") && shown.contains("\"Units\":123"));
        assert!(shown.contains("\"Amount\":50") && shown.contains("\"MaxAmount\":250"));
        let back = pack(shown.as_bytes(), true).unwrap();
        assert_eq!(unpack(&back).unwrap(), SAMPLE.as_bytes());
    }

    #[test]
    fn a_name_is_never_merged_with_another_key() {
        let mut v: serde_json::Value = serde_json::from_str(r#"{"F2P":1,"Version":2}"#).unwrap();
        rename(&mut v, &names().plain);
        assert_eq!(v.as_object().unwrap().len(), 2);
    }

    #[test]
    fn xxtea_round_trips_and_the_metadata_size_follows_the_save() {
        for len in [0x68usize, 0x168] {
            let mut plain = vec![0u32; len / 4];
            plain[0] = META_MAGIC;
            plain[1] = 0x7D2;
            plain[14] = 10;
            plain[15] = if len == 0x168 { 7 } else { 0 };
            plain[17] = 0x102A;
            let mut enc = plain.clone();
            xxtea_encrypt(&mut enc, &meta_key(4), meta_rounds(len));
            assert_ne!(enc, plain);
            let mut dec = enc.clone();
            xxtea_decrypt(&mut dec, &meta_key(4), meta_rounds(len));
            assert_eq!(dec, plain);
            let bytes: Vec<u8> = enc.iter().flat_map(|w| w.to_le_bytes()).collect();
            let out = updated_meta(&bytes, 4, 999, 500).unwrap();
            let mut w: Vec<u32> = out.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
            xxtea_decrypt(&mut w, &meta_key(4), meta_rounds(len));
            assert_eq!(w[0], META_MAGIC);
            assert_eq!(w[14], 1000);
            assert_eq!(w[15], if len == 0x168 { 500 } else { 0 });
            assert_eq!(w[17], 0x102A);
        }
    }

    #[test]
    fn save_file_names_give_storage_slots() {
        assert_eq!(slot_of("save.hg"), Some(2));
        assert_eq!(slot_of("save3.hg"), Some(4));
        assert_eq!(slot_of("accountdata.hg"), None);
    }

    #[test]
    fn a_metadata_file_is_rewritten_beside_the_save() {
        let dir = std::env::temp_dir().join(format!("nms_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut plain = vec![0u32; 0x68 / 4];
        plain[0] = META_MAGIC;
        plain[1] = 0x7D2;
        plain[14] = 1;
        xxtea_encrypt(&mut plain, &meta_key(4), 8);
        std::fs::write(dir.join("mf_save3.hg"), plain.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>()).unwrap();
        let save = pack_chunks(SAMPLE.as_bytes());
        let out = companions(&dir.join("save3.hg"), &save);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, dir.join("mf_save3.hg"));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Public sample saves (not in the repository): `NMS_SAMPLES=<folder with save*.hg>`.
    #[test]
    #[ignore]
    fn public_samples_open_edit_and_save_again() {
        let Ok(dir) = std::env::var("NMS_SAMPLES") else { return };
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if slot_of(&name).is_none() {
                continue;
            }
            let raw = std::fs::read(e.path()).unwrap();
            let mut d = crate::formats::decode(&raw);
            assert!(d.layers.iter().any(|l| matches!(l, crate::formats::Layer::Nms { .. })), "{name}");
            let crate::formats::Content::Json { value, .. } = &mut d.content else { panic!("{name} is not JSON") };
            assert!(value.pointer("/PlayerStateData/Inventory/Slots/0/Amount").is_some(), "{name}");
            let game = crate::game_ui::detect("JSON", value);
            assert!(game.is_some(), "{name}: no screen");
            *value.pointer_mut("/PlayerStateData/Units").unwrap() = serde_json::json!(4242);
            let out = crate::formats::encode(&d.layers, &d.content).unwrap();
            let back = crate::formats::decode(&out);
            let crate::formats::Content::Json { value: v2, .. } = &back.content else { panic!() };
            assert_eq!(v2.pointer("/PlayerStateData/Units").unwrap(), &serde_json::json!(4242));
            // Nothing else moved: the same text once the one edit is undone.
            let orig = unpack(&raw).unwrap();
            let mut v: serde_json::Value = serde_json::from_slice(&orig).unwrap();
            *v.pointer_mut("/6f=/wGS").unwrap() = serde_json::json!(4242);
            assert_eq!(unpack(&out).unwrap(), serde_json::to_vec(&v).unwrap(), "{name}");
            // The game's own metadata already agrees with its own save: nothing to rewrite.
            if let Ok(mf) = std::fs::read(e.path().with_file_name(format!("mf_{name}"))) {
                assert!(updated_meta(&mf, slot_of(&name).unwrap(), orig.len(), raw.len()).is_none(), "{name}: metadata size");
            }
            n += 1;
        }
        assert!(n > 0);
    }
}
