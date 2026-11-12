//! Sid Meier's Civilization VII saves (`.Civ7Save`): a `CIV7` header (which names the leaders),
//! the game as one zlib stream cut into chunks that each start with their length, and a footer.
//!
//! The container, the markers that lead to each player's gold and influence, and the 24-bit way
//! those two are stored are as in civ7-save-editor (iqqmuT, MIT) and civ7-save-parser
//! (playyourdamnturn.com, MIT).
//!
//! The unpacked form is `CV7U`, the header's length (u32), the header, then the game data.

use crate::tree::{Backend, Patches, TreeDoc};
use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Civilization VII save";
const START: [u8; 6] = [0, 0, 1, 0, 0x78, 0x9c];
const GOLD: [u8; 4] = [0x35, 0xcf, 0xc8, 0x6e];
const INFLUENCE: [u8; 4] = [0x50, 0x3c, 0xa8, 0x4a];
const LEADER: [u8; 4] = [0x0f, 0xfb, 0x8c, 0xc1];
const SLOTS: [[u8; 4]; 8] = [[0xb8, 0x61, 0xf0, 0xf4], [0x2e, 0x51, 0xf7, 0x83], [0xd4, 0xab, 0x9f, 0x19], [0x02, 0x30, 0xf9, 0x6d], [0xa1, 0xa5, 0x9d, 0xf3], [0x37, 0x95, 0x9a, 0x84], [0x8d, 0xc4, 0x93, 0x1d], [0x1b, 0xf4, 0x94, 0x6a]];
const TAG: &[u8; 4] = b"CV7U";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    /// The bytes after the last chunk, starting with the zero-length chunk marker.
    pub footer: Vec<u8>,
    pub chunk: u32,
}

fn find(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    h.get(from..)?.windows(n.len()).position(|w| w == n).map(|p| p + from)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

pub fn open(d: &[u8]) -> Option<(Info, Vec<u8>)> {
    if !d.starts_with(b"CIV7") {
        return None;
    }
    let start = find(d, &START, 0)?;
    let rest = &d[start..];
    let mut pos = 0;
    let chunk = u32le(rest, 0)?;
    let mut comp = Vec::new();
    let mut size = chunk;
    pos += 4;
    while size > 1 {
        comp.extend_from_slice(rest.get(pos..pos + size as usize)?);
        pos += size as usize;
        size = u32le(rest, pos)?;
        pos += 4;
    }
    pos -= 4;
    let mut z = Decompress::new(true);
    let mut body: Vec<u8> = Vec::with_capacity(comp.len() * 4);
    let mut consumed = 0usize;
    loop {
        if body.capacity() - body.len() < 1 << 20 {
            body.reserve(1 << 22);
        }
        let before_in = z.total_in();
        let before_out = z.total_out();
        z.decompress_vec(&comp[consumed..], &mut body, FlushDecompress::Sync).ok()?;
        consumed += (z.total_in() - before_in) as usize;
        if consumed >= comp.len() || (z.total_in() == before_in && z.total_out() == before_out) {
            break;
        }
    }
    let mut out = TAG.to_vec();
    out.extend_from_slice(&(start as u32).to_le_bytes());
    out.extend_from_slice(&d[..start]);
    out.extend_from_slice(&body);
    Some((Info { footer: rest[pos..].to_vec(), chunk }, out))
}

pub fn pack(info: &Info, data: &[u8]) -> Result<Vec<u8>, String> {
    if !data.starts_with(TAG) {
        return Err("Not an unpacked Civilization VII save".into());
    }
    let hl = u32le(data, 4).ok_or("short")? as usize;
    let header = data.get(8..8 + hl).ok_or("short")?;
    let body = &data[8 + hl..];
    let mut z = Compress::new(Compression::default(), true);
    let mut comp: Vec<u8> = Vec::with_capacity(body.len() / 2 + 1024);
    loop {
        comp.reserve(1 << 20);
        z.compress_vec(&body[z.total_in() as usize..], &mut comp, FlushCompress::Sync).map_err(|e| e.to_string())?;
        if z.total_in() as usize >= body.len() && comp.len() < comp.capacity() {
            break;
        }
    }
    let mut out = header.to_vec();
    let size = info.chunk.max(2) as usize;
    let mut at = 0;
    while at + size < comp.len() {
        out.extend_from_slice(&(size as u32).to_le_bytes());
        out.extend_from_slice(&comp[at..at + size]);
        at += size;
    }
    out.extend_from_slice(&((comp.len() - at) as u32).to_le_bytes());
    out.extend_from_slice(&comp[at..]);
    out.extend_from_slice(&info.footer);
    Ok(out)
}

fn read24(body: &[u8], at: usize) -> Option<i64> {
    let v = u32le(body, at)?;
    let main = (v >> 8) as i64;
    Some(if v & 0xff == 0xff { main + 1 } else { main })
}

#[derive(Clone, Debug)]
struct Civ7Backend {
    raw: Vec<u8>,
    /// Offset in `raw` of each editable value.
    at: Vec<usize>,
}

impl Backend for Civ7Backend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let n = v.as_i64().ok_or("Gold and influence are whole numbers")?;
            if !(0..=0x80_0000).contains(&n) {
                return Err("Must be between 0 and 8388608".into());
            }
            let word: u32 = if n < 0x80_0000 { (n as u32) << 8 } else { 0x7fff_ffff };
            p.add(self.at[*id], self.at[*id] + 4, word.to_le_bytes().to_vec());
        }
        p.apply(&self.raw)
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        match v.as_i64() {
            Some(n) if (0..=0x80_0000).contains(&n) => Ok(()),
            _ => Err("Must be a whole number between 0 and 8388608".into()),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// The players' gold and influence of an unpacked save.
pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    if !d.starts_with(TAG) {
        return None;
    }
    let hl = u32le(d, 4)? as usize;
    let header = d.get(8..8 + hl)?;
    let body_at = 8 + hl;
    let body = &d[body_at..];
    let positions = |marker: &[u8]| -> Vec<usize> {
        let mut v = vec![];
        let mut p = 0;
        while v.len() < 8 {
            let Some(i) = find(body, marker, p) else { break };
            v.push(i + 24);
            p = i + 24;
        }
        v
    };
    let (gold, infl) = (positions(&GOLD), positions(&INFLUENCE));
    let mut players = Map::new();
    let mut editable = HashMap::new();
    let mut at = vec![];
    for (idx, m) in SLOTS.iter().enumerate() {
        let Some(pos) = find(header, m, 0) else { continue };
        let Some(lp) = find(header, &LEADER, pos) else { continue };
        let ls = lp + 20;
        let Some(end) = header.get(ls..).and_then(|h| h.iter().position(|b| *b == 0)).map(|e| e + ls) else { continue };
        let name = String::from_utf8_lossy(&header[ls..end]).trim_start_matches("LEADER_").to_string();
        let mut o = Map::new();
        for (label, list) in [("Gold", &gold), ("Influence", &infl)] {
            let Some(&p) = list.get(idx) else { continue };
            let Some(v) = read24(body, p) else { continue };
            // Saves of other game versions keep these elsewhere: junk is not offered for editing.
            if !(0..=200_000).contains(&v) {
                return None;
            }
            o.insert(label.into(), Value::from(v));
            editable.insert(crate::tree::child(&crate::tree::child("/Players", &format!("{name} (slot {})", idx + 1)), label), at.len());
            at.push(body_at + p);
        }
        players.insert(format!("{name} (slot {})", idx + 1), Value::Object(o));
    }
    if players.is_empty() {
        return None;
    }
    let mut root = Map::new();
    root.insert("Players".into(), Value::Object(players));
    Some(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(Civ7Backend { raw: d.to_vec(), at })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_saves() {
        let Ok(dir) = std::env::var("CIV7_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().and_then(|e| e.to_str()) != Some("Civ7Save") {
                continue;
            }
            let d = std::fs::read(&p).unwrap();
            let (info, un) = open(&d).unwrap_or_else(|| panic!("{p:?} opens"));
            let again = pack(&info, &un).unwrap();
            let (_, un2) = open(&again).unwrap_or_else(|| panic!("{p:?} reopens"));
            assert_eq!(un, un2, "{p:?}");
            let doc = parse(&un);
            println!("{p:?}: {} bytes, {}", un.len(), doc.map(|t| t.view.to_string()).unwrap_or_default());
        }
    }
}
