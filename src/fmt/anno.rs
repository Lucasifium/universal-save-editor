//! Anno 1800 saves (`.a7s`): an RDA 2.2 archive ("Resource File V2.2") holding `meta.a7s`,
//! `header.a7s`, `gamesetup.a7s` and `data.a7s`, each a zlib stream followed by an empty zlib
//! stream and the unpacked size. Every inner file is a FileDB (BBDom V3) document:
//!
//! ```text
//! node   = i32 size, i32 id;  id >= 0x8000: an attribute with `size` bytes (padded to 8),
//!          0 < id < 0x8000: a tag that opens, id <= 0: the end of the open tag
//! tail   = tag names, attribute names (i32 count, u16 ids, NUL-ended names),
//!          i32 tags offset, i32 attributes offset, i32 8, i32 -3
//! ```
//!
//! The player's coins are in `data.a7s` under `MetaGameManager/EconomyManager/Pair` (the pair
//! whose participant is 0) `/MetaEconomy/StrgLrg`: a storage of the first GUID, the gaps to the
//! next GUIDs (each minus one), then one i32 amount per GUID. Coins are GUID 1010017. The load
//! screen's copy is `CorporationProfile/TotalMoney` in `header.a7s`, kept equal.
//!
//! The RDA and FileDB layouts are as documented by RDAExplorer and FileDBReader (facts only; neither has a licence file);
//! the storage encoding was checked on a public save. Amounts are changed in place, so the
//! documents keep their layout; the changed files are packed again and the archive rebuilt.

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};

pub const KIND: &str = "Anno 1800 save";
const MAGIC: &[u8] = b"Resource File V2.2";
const HEADER: usize = 792;
const DIR_ENTRY: usize = 560;
const COINS: u32 = 1_010_017;
const EMPTY_ZLIB: [u8; 8] = [0x78, 0xDA, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01];

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    /// The block header (32 bytes) and the directory entry, as the file has them.
    block: [u8; 32],
    dir: Vec<u8>,
    data: (usize, usize),
}

fn u32at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}
fn u64at(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// The files of an RDA 2.2 archive whose directories are stored plainly (as saves are).
fn rda(d: &[u8]) -> Option<Vec<Entry>> {
    if !d.starts_with(MAGIC) {
        return None;
    }
    let mut out = vec![];
    let mut off = u64at(d, HEADER - 8)? as usize;
    for _ in 0..64 {
        let flags = u32at(d, off)?;
        let n = u32at(d, off + 4)? as usize;
        let dir_size = u64at(d, off + 8)? as usize;
        let next = u64at(d, off + 24)? as usize;
        if flags != 0 || dir_size != n * DIR_ENTRY || dir_size > off {
            return None;
        }
        let dir = d.get(off - dir_size..off)?;
        for e in dir.chunks(DIR_ENTRY) {
            let name: String = char::decode_utf16(e[..520].chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|c| *c != 0)).filter_map(Result::ok).collect();
            let at = u64at(e, 520)? as usize;
            let size = u64at(e, 528)? as usize;
            d.get(at..at + size)?;
            out.push(Entry { name, block: d[off..off + 32].try_into().ok()?, dir: e.to_vec(), data: (at, size) });
        }
        if next >= d.len() || next <= off {
            return (next == d.len()).then_some(out);
        }
        off = next;
    }
    None
}

/// Rebuilds the archive with some files replaced (one file per block, each block's files
/// followed by its directory and block header, as the game writes saves).
fn rda_write(d: &[u8], entries: &[Entry], replace: &HashMap<String, Vec<u8>>) -> Vec<u8> {
    let mut out = d[..HEADER].to_vec();
    let mut first = true;
    for (i, e) in entries.iter().enumerate() {
        let body: &[u8] = replace.get(&e.name).map_or(&d[e.data.0..e.data.0 + e.data.1], |v| v.as_slice());
        let at = out.len();
        out.extend_from_slice(body);
        let mut dir = e.dir.clone();
        dir[520..528].copy_from_slice(&(at as u64).to_le_bytes());
        dir[528..536].copy_from_slice(&(body.len() as u64).to_le_bytes());
        dir[536..544].copy_from_slice(&(body.len() as u64).to_le_bytes());
        out.extend_from_slice(&dir);
        let header_at = out.len();
        if first {
            out[HEADER - 8..HEADER].copy_from_slice(&(header_at as u64).to_le_bytes());
            first = false;
        }
        let mut block = e.block;
        let next = match entries.get(i + 1) {
            Some(n) => header_at + 32 + replace.get(&n.name).map_or(n.data.1, Vec::len) + DIR_ENTRY,
            None => header_at + 32,
        };
        block[24..32].copy_from_slice(&(next as u64).to_le_bytes());
        out.extend_from_slice(&block);
    }
    out
}

/// An inner file: zlib streams, then the unpacked size.
fn unpack(b: &[u8]) -> Option<Vec<u8>> {
    let size = u32at(b, b.len().checked_sub(4)?)? as usize;
    let mut out = Vec::with_capacity(size);
    let mut rest = &b[..b.len() - 4];
    while !rest.is_empty() {
        let mut z = flate2::read::ZlibDecoder::new(rest);
        z.read_to_end(&mut out).ok()?;
        let used = z.total_in() as usize;
        if used == 0 {
            return None;
        }
        rest = &rest[used..];
    }
    (out.len() == size).then_some(out)
}

fn pack(u: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::with_capacity(u.len() / 20), flate2::Compression::best());
    z.write_all(u).expect("writing to memory");
    let mut out = z.finish().expect("writing to memory");
    out.extend_from_slice(&EMPTY_ZLIB);
    out.extend_from_slice(&(u.len() as u32).to_le_bytes());
    out
}

/// The tag and attribute names of a FileDB V3 document, and where its nodes end.
fn names(d: &[u8]) -> Option<(HashMap<i32, String>, HashMap<i32, String>, usize)> {
    let n = d.len();
    if n < 16 || d[n - 8..] != [8, 0, 0, 0, 0xFD, 0xFF, 0xFF, 0xFF] {
        return None;
    }
    let dict = |base: usize| -> Option<HashMap<i32, String>> {
        let count = u32at(d, base)? as usize;
        let mut p = base + 4 + 2 * count;
        let mut m = HashMap::new();
        for i in 0..count {
            let id = u16::from_le_bytes(d.get(base + 4 + 2 * i..base + 6 + 2 * i)?.try_into().ok()?) as i32;
            let end = p + d.get(p..)?.iter().position(|b| *b == 0)?;
            m.insert(id, String::from_utf8_lossy(&d[p..end]).into_owned());
            p = end + 1;
        }
        Some(m)
    };
    let to = u32at(d, n - 16)? as usize;
    let ao = u32at(d, n - 12)? as usize;
    Some((dict(to)?, dict(ao)?, to.min(ao)))
}

/// Calls `f(path of open tags, attribute name, content range)` for every attribute.
fn walk(d: &[u8], mut f: impl FnMut(&[&str], &str, usize, usize)) -> Option<()> {
    let (tags, attrs, end) = names(d)?;
    let mut stack: Vec<&str> = vec![];
    let mut p = 0;
    while p + 8 <= end {
        let size = u32at(d, p)? as usize;
        let id = u32at(d, p + 4)? as i32;
        p += 8;
        if id >= 0x8000 {
            if p + size > end {
                return None;
            }
            f(&stack, attrs.get(&id).map_or("", String::as_str), p, size);
            p += size.div_ceil(8) * 8;
        } else if id > 0 {
            stack.push(tags.get(&id).map_or("", String::as_str));
        } else {
            stack.pop();
        }
    }
    Some(())
}

/// The player's storage (`StrgLrg` of participant 0) in `data.a7s`: (offset, size).
fn player_storage(d: &[u8]) -> Option<(usize, usize)> {
    let mut participant: Option<u16> = None;
    let mut found = None;
    walk(d, |path, name, at, size| {
        if path.ends_with(&["EconomyManager", "Pair", "ParticipantID", "value"]) && name == "id" && size == 2 {
            participant = Some(u16::from_le_bytes([d[at], d[at + 1]]));
        } else if path.ends_with(&["EconomyManager", "Pair", "MetaEconomy"]) && name == "StrgLrg" && participant == Some(0) && found.is_none() {
            found = Some((at, size));
        }
    })?;
    found
}

fn storage(b: &[u8]) -> Option<Vec<(u32, i32)>> {
    if b.len() < 8 || b.len() % 8 != 0 {
        return None;
    }
    let n = b.len() / 8;
    let mut guid = u32at(b, 0)?;
    let mut out = vec![];
    for i in 0..n {
        if i > 0 {
            guid = guid.checked_add(u32at(b, i * 4)?)?.checked_add(1)?;
        }
        out.push((guid, u32at(b, (n + i) * 4)? as i32));
    }
    Some(out)
}

/// `TotalMoney` in `header.a7s` (the load screen's figure).
fn total_money(h: &[u8]) -> Option<usize> {
    let mut found = None;
    walk(h, |path, name, at, size| {
        if name == "TotalMoney" && size >= 4 && path.last() == Some(&"CorporationProfile") && found.is_none() {
            found = Some(at);
        }
    })?;
    found
}

fn guid_name(g: u32) -> String {
    match g {
        COINS => "Coins".into(),
        1_010_190 => "Influence".into(),
        _ => format!("GUID {g}"),
    }
}

#[derive(Clone, Debug)]
struct B {
    raw: std::sync::Arc<Vec<u8>>,
    /// GUIDs of the storage, in order (leaf id = index).
    guids: Vec<u32>,
}

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.to_vec());
        }
        let d = &self.raw[..];
        let entries = rda(d).ok_or("the save archive no longer reads")?;
        let get = |name: &str| entries.iter().find(|e| e.name == name).ok_or(format!("{name} is missing"));
        let de = get("data.a7s")?;
        let mut data = unpack(&d[de.data.0..de.data.0 + de.data.1]).ok_or("data.a7s doesn't unpack")?;
        let (at, size) = player_storage(&data).ok_or("the player's storage wasn't found")?;
        let n = size / 8;
        let mut coins = None;
        for (id, v) in changes {
            let a = v.as_i64().filter(|a| i32::try_from(*a).is_ok()).ok_or("Must be a whole number")? as i32;
            if *id >= n {
                return Err("unknown value".into());
            }
            let p = at + (n + id) * 4;
            data[p..p + 4].copy_from_slice(&a.to_le_bytes());
            if self.guids.get(*id) == Some(&COINS) {
                coins = Some(a);
            }
        }
        let mut replace = HashMap::from([("data.a7s".to_string(), pack(&data))]);
        if let Some(c) = coins {
            let he = get("header.a7s")?;
            let mut h = unpack(&d[he.data.0..he.data.0 + he.data.1]).ok_or("header.a7s doesn't unpack")?;
            if let Some(p) = total_money(&h) {
                h[p..p + 4].copy_from_slice(&c.to_le_bytes());
                replace.insert("header.a7s".into(), pack(&h));
            }
        }
        Ok(rda_write(d, &entries, &replace))
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        match v.as_i64() {
            Some(n) if (0..=i32::MAX as i64).contains(&n) => Ok(()),
            _ => Err("Must be a whole number from 0 to 2,147,483,647".into()),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(MAGIC) && rda(d).is_some_and(|e| e.iter().any(|e| e.name == "data.a7s"))
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let entries = rda(d)?;
    let de = entries.iter().find(|e| e.name == "data.a7s")?;
    let data = unpack(&d[de.data.0..de.data.0 + de.data.1])?;
    let (at, size) = player_storage(&data)?;
    let items = storage(&data[at..at + size])?;
    drop(data);
    let mut store = Map::new();
    let mut edit = HashMap::new();
    for (i, (g, a)) in items.iter().enumerate() {
        let key = guid_name(*g);
        edit.insert(format!("/Player/{key}"), i);
        store.insert(key, Value::from(*a));
    }
    let mut root = Map::new();
    root.insert("Player".into(), Value::Object(store));
    let guids = items.iter().map(|x| x.0).collect();
    let mut doc = TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B { raw: std::sync::Arc::new(d.to_vec()), guids }));
    doc.note = Some("Anno 1800 save: the game data is unpacked to read your coins and packed again when you save, which takes a few seconds on a large save.".into());
    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A FileDB V3 document from (depth-first) nodes.
    fn filedb(nodes: &[(i32, &[u8])], tags: &[(u16, &str)], attrs: &[(u16, &str)]) -> Vec<u8> {
        let mut d = vec![];
        for (id, body) in nodes {
            d.extend((body.len() as u32).to_le_bytes());
            d.extend(id.to_le_bytes());
            if *id >= 0x8000 {
                d.extend(*body);
                d.resize(d.len().div_ceil(8) * 8, 0);
            }
        }
        let dict = |d: &mut Vec<u8>, m: &[(u16, &str)]| {
            let at = d.len();
            d.extend((m.len() as u32).to_le_bytes());
            for (id, _) in m {
                d.extend(id.to_le_bytes());
            }
            for (_, n) in m {
                d.extend(n.as_bytes());
                d.push(0);
            }
            at
        };
        let to = dict(&mut d, tags);
        let ao = dict(&mut d, attrs);
        d.extend((to as u32).to_le_bytes());
        d.extend((ao as u32).to_le_bytes());
        d.extend([8, 0, 0, 0, 0xFD, 0xFF, 0xFF, 0xFF]);
        d
    }

    fn archive(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        d.resize(HEADER, 0);
        let entries: Vec<Entry> = files
            .iter()
            .map(|(n, _)| {
                let mut dir = vec![0u8; DIR_ENTRY];
                for (i, c) in n.encode_utf16().enumerate() {
                    dir[i * 2..i * 2 + 2].copy_from_slice(&c.to_le_bytes());
                }
                let mut block = [0u8; 32];
                block[4] = 1;
                block[8..16].copy_from_slice(&(DIR_ENTRY as u64).to_le_bytes());
                block[16..24].copy_from_slice(&(DIR_ENTRY as u64).to_le_bytes());
                Entry { name: n.to_string(), block, dir, data: (0, 0) }
            })
            .collect();
        let replace: HashMap<String, Vec<u8>> = files.iter().map(|(n, b)| (n.to_string(), pack(b))).collect();
        rda_write(&d, &entries, &replace)
    }

    fn sample(coins: i32) -> Vec<u8> {
        let mut strg = vec![];
        strg.extend(119_392u32.to_le_bytes());
        strg.extend((COINS - 119_392 - 1).to_le_bytes());
        strg.extend(0i32.to_le_bytes());
        strg.extend(coins.to_le_bytes());
        let p0 = 0u16.to_le_bytes();
        let data = filedb(
            &[(1, b""), (2, b""), (3, b""), (4, b""), (5, b""), (0x8001, &p0), (0, b""), (0, b""), (6, b""), (0x8002, &strg), (0, b""), (0, b""), (0, b""), (0, b"")],
            &[(1, "MetaGameManager"), (2, "EconomyManager"), (3, "Pair"), (4, "ParticipantID"), (5, "value"), (6, "MetaEconomy")],
            &[(0x8001, "id"), (0x8002, "StrgLrg")],
        );
        let mut money = coins.to_le_bytes().to_vec();
        money.extend([0xFF; 12]);
        let header = filedb(&[(1, b""), (0x8001, &money), (0, b"")], &[(1, "CorporationProfile")], &[(0x8001, "TotalMoney")]);
        archive(&[("meta.a7s", filedb(&[], &[], &[])), ("header.a7s", header), ("data.a7s", data)])
    }

    #[test]
    fn reads_and_edits_coins() {
        let d = sample(5000);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Player/Coins").unwrap(), 5000);
        doc.set("/Player/Coins", Value::from(2_000_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Player/Coins").unwrap(), 2_000_000);
        assert_eq!(back.view.pointer("/Player/GUID 119392").unwrap(), 0);
        let he = rda(&out).unwrap().into_iter().find(|e| e.name == "header.a7s").unwrap();
        let h = unpack(&out[he.data.0..he.data.0 + he.data.1]).unwrap();
        assert_eq!(u32at(&h, total_money(&h).unwrap()), Some(2_000_000));
        assert_eq!(rda(&out).unwrap().len(), 3);
    }
}
