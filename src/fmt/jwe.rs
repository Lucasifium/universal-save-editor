//! Jurassic World Evolution (1, 2 and 3) career and sandbox saves: a ZIP of Frontier's "Cobra"
//! files (`island1` …, `careerc`, `careerdb`, `metadata`). Each Cobra file starts with
//! `ff 00 fe 01`, a CRC-32 of everything after the 16-byte header (stored big-endian, no
//! final inversion), `00 00 00 01` and the body length. The ZIP's own directory has no size
//! (the game writes a zero), so the entries are walked from their local headers.
//!
//! `careerc` keeps the career-wide values. `GlobalMoney` is a table of one amount per island
//! (a key that is an index into the file's string table or an inline name, then a 28-bit amount
//! as `e0|high nibble` and three more bytes). Only that table is edited, and only when every
//! amount uses that four-byte form, so nothing in the file changes size. Everything else is
//! copied byte for byte.

use crate::tree::{int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};

pub const KIND: &str = "Jurassic World Evolution career";
/// The most any island held in the public career checked (the file's 28 bits would hold more).
const MAX_MONEY: i128 = 100_000_000;

fn u16le(d: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(i..i + 2)?.try_into().ok()?) as usize)
}
fn u32le(d: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(i..i + 4)?.try_into().ok()?) as usize)
}

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    /// The whole local record: header, name, extra field and data.
    start: usize,
    end: usize,
    header_len: usize,
    method: usize,
    /// The matching record in the central directory.
    central: usize,
    central_len: usize,
}

/// The entries, and where the end-of-directory record starts.
fn entries(d: &[u8]) -> Option<(Vec<Entry>, usize)> {
    let mut out = Vec::new();
    let mut p = 0;
    while d.get(p..p + 4)? == b"PK\x03\x04" {
        let flag = u16le(d, p + 6)?;
        let method = u16le(d, p + 8)?;
        let (cs, nl, el) = (u32le(d, p + 18)?, u16le(d, p + 26)?, u16le(d, p + 28)?);
        if flag & 8 != 0 || !(method == 0 || method == 8) {
            return None;
        }
        let name = String::from_utf8(d.get(p + 30..p + 30 + nl)?.to_vec()).ok()?;
        let header_len = 30 + nl + el;
        let end = p.checked_add(header_len)?.checked_add(cs)?;
        if end > d.len() {
            return None;
        }
        out.push(Entry { name, start: p, end, header_len, method, central: 0, central_len: 0 });
        p = end;
    }
    // End of central directory: the last 22 bytes (no comment); the directory starts where the
    // local records end.
    let e = d.len().checked_sub(22)?;
    if d.get(e..e + 4)? != b"PK\x05\x06" || u32le(d, e + 16)? != p || u16le(d, e + 10)? != out.len() {
        return None;
    }
    let mut c = p;
    for en in &mut out {
        if d.get(c..c + 4)? != b"PK\x01\x02" || u32le(d, c + 42)? != en.start {
            return None;
        }
        let len = 46 + u16le(d, c + 28)? + u16le(d, c + 30)? + u16le(d, c + 32)?;
        if d.get(c + 46..c + 46 + en.name.len())? != en.name.as_bytes() {
            return None;
        }
        en.central = c;
        en.central_len = len;
        c += len;
    }
    (c == e).then_some((out, e))
}

fn read(d: &[u8], e: &Entry) -> Option<Vec<u8>> {
    let data = &d[e.start + e.header_len..e.end];
    match e.method {
        0 => Some(data.to_vec()),
        _ => {
            let mut v = Vec::new();
            flate2::read::DeflateDecoder::new(data).read_to_end(&mut v).ok()?;
            Some(v)
        }
    }
}

/// The CRC-32 the Cobra header stores: no initial or final inversion, big-endian.
fn cobra_crc(body: &[u8]) -> u32 {
    super::crc32_raw(0, body)
}

fn cobra_ok(d: &[u8]) -> bool {
    d.len() >= 16 && d[..4] == [0xff, 0x00, 0xfe, 0x01] && d[4..8] == cobra_crc(&d[16..]).to_be_bytes()
}

/// (island name, offset of its four amount bytes) for each row of `GlobalMoney`.
fn money(c: &[u8]) -> Option<Vec<(String, usize)>> {
    // The string table: `Strings>`, a count, then `f3 name 00` for each.
    let s = c.windows(8).position(|w| w == b"Strings>")? + 8;
    let n = *c.get(s)? as usize;
    if n >= 0x80 {
        return None;
    }
    let mut p = s + 1;
    let mut names = Vec::new();
    for _ in 0..n {
        if *c.get(p)? != 0xf3 {
            return None;
        }
        let end = p + 1 + c.get(p + 1..)?.iter().position(|&b| b == 0)?;
        names.push(String::from_utf8(c[p + 1..end].to_vec()).ok()?);
        p = end + 1;
    }
    let key = b"\xf3GlobalMoney\x00";
    let g = c.windows(key.len()).position(|w| w == key)? + key.len();
    if *c.get(g)? != 0x02 {
        return None;
    }
    let count = *c.get(g + 1)? as usize;
    let mut p = g + 2;
    let mut rows = Vec::new();
    for _ in 0..count {
        let name = match *c.get(p)? {
            0xf3 => {
                let end = p + 1 + c.get(p + 1..)?.iter().position(|&b| b == 0)?;
                let n = String::from_utf8(c[p + 1..end].to_vec()).ok()?;
                p = end + 1;
                n
            }
            i if (i as usize) < names.len() => {
                p += 1;
                names[i as usize].clone()
            }
            _ => return None,
        };
        if c.get(p)? & 0xf0 != 0xe0 || p + 4 > c.len() {
            return None;
        }
        rows.push((name, p));
        p += 4;
    }
    Some(rows)
}

fn amount(c: &[u8], p: usize) -> i64 {
    (((c[p] & 0x0f) as i64) << 24) | ((c[p + 1] as i64) << 16) | ((c[p + 2] as i64) << 8) | c[p + 3] as i64
}

pub fn looks(d: &[u8]) -> bool {
    let Some((es, _)) = entries(d) else { return false };
    let has = |n: &str| es.iter().any(|e| e.name == n);
    has("careerc") && has("careerdb") && has("metadata") && es.iter().any(|e| e.name.starts_with("island"))
}

#[derive(Clone, Debug)]
struct Jwe {
    raw: Vec<u8>,
    /// Offsets of the amounts inside `careerc`, one per editable leaf.
    offsets: Vec<usize>,
}

impl Backend for Jwe {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let d = &self.raw;
        let (es, eocd) = entries(d).ok_or("the save's layout changed")?;
        let ci = es.iter().position(|e| e.name == "careerc").ok_or("no careerc in the save")?;
        let mut body = read(d, &es[ci]).ok_or("careerc could not be unpacked")?;
        for (leaf, v) in changes {
            let n = int_in(v, 0, MAX_MONEY, "This amount")? as u32;
            let p = self.offsets[*leaf];
            body[p..p + 4].copy_from_slice(&(n | 0xe000_0000).to_be_bytes());
        }
        let crc = cobra_crc(&body[16..]);
        body[4..8].copy_from_slice(&crc.to_be_bytes());
        let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&body).map_err(|e| e.to_string())?;
        let packed = z.finish().map_err(|e| e.to_string())?;
        let zip_crc = !super::crc32_raw(!0, &body);

        let mut out = Vec::with_capacity(d.len() + 64);
        let mut starts = Vec::new();
        for (i, e) in es.iter().enumerate() {
            starts.push(out.len());
            if i == ci {
                let mut h = d[e.start..e.start + e.header_len].to_vec();
                h[8..10].copy_from_slice(&8u16.to_le_bytes());
                h[14..18].copy_from_slice(&zip_crc.to_le_bytes());
                h[18..22].copy_from_slice(&(packed.len() as u32).to_le_bytes());
                h[22..26].copy_from_slice(&(body.len() as u32).to_le_bytes());
                out.extend_from_slice(&h);
                out.extend_from_slice(&packed);
            } else {
                out.extend_from_slice(&d[e.start..e.end]);
            }
        }
        let new_cd = out.len();
        for (i, e) in es.iter().enumerate() {
            let mut c = d[e.central..e.central + e.central_len].to_vec();
            if i == ci {
                c[10..12].copy_from_slice(&8u16.to_le_bytes());
                c[16..20].copy_from_slice(&zip_crc.to_le_bytes());
                c[20..24].copy_from_slice(&(packed.len() as u32).to_le_bytes());
                c[24..28].copy_from_slice(&(body.len() as u32).to_le_bytes());
            }
            c[42..46].copy_from_slice(&(starts[i] as u32).to_le_bytes());
            out.extend_from_slice(&c);
        }
        let mut tail = d[eocd..].to_vec();
        tail[16..20].copy_from_slice(&(new_cd as u32).to_le_bytes());
        out.extend_from_slice(&tail);
        Ok(out)
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        int_in(v, 0, MAX_MONEY, "This amount").map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Jurassic World Evolution save".into());
    }
    let (es, _) = entries(d).ok_or("bad layout")?;
    let get = |n: &str| es.iter().find(|e| e.name == n).and_then(|e| read(d, e));
    let careerc = get("careerc").filter(|c| cobra_ok(c)).ok_or("careerc is damaged")?;
    let mut root = Map::new();
    if let Some(meta) = get("metadata").filter(|m| m.len() > 16) {
        if let Ok(Value::Object(m)) = serde_json::from_slice::<Value>(&meta[16..]) {
            let mut c = Map::new();
            if let Some(v) = m.get("career") {
                c.insert("Career".into(), v.clone());
            }
            if let Some(v) = m.get("island") {
                c.insert("Island".into(), v.clone());
            }
            if let Some(p) = m.get("playtime") {
                c.insert("Hours played".into(), p.get("hours").cloned().unwrap_or(Value::Null));
                c.insert("Minutes played".into(), p.get("mins").cloned().unwrap_or(Value::Null));
            }
            root.insert("Career".into(), Value::Object(c));
        }
    }
    let mut editable = HashMap::new();
    let mut offsets = Vec::new();
    if let Some(rows) = money(&careerc) {
        let mut mo = Map::new();
        for (name, p) in rows {
            editable.insert(format!("/Money/{}", crate::tree::esc(&name)), offsets.len());
            mo.insert(name, Value::from(amount(&careerc, p)));
            offsets.push(p);
        }
        root.insert("Money".into(), Value::Object(mo));
    }
    let note = crate::i18n::tx("Close the game first. Each island's money is the amount the career keeps for it; the game may still show the island's own balance when you open it.");
    Ok(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(Jwe { raw: d.to_vec(), offsets })).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cobra(body: &[u8]) -> Vec<u8> {
        let mut v = vec![0xff, 0, 0xfe, 1];
        v.extend_from_slice(&cobra_crc(body).to_be_bytes());
        v.extend_from_slice(&[0, 0, 0, 1]);
        v.extend_from_slice(&(body.len() as u32).to_be_bytes());
        v.extend_from_slice(body);
        v
    }

    fn zip(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (n, data) in files {
            let mut z = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            z.write_all(data).unwrap();
            let packed = z.finish().unwrap();
            let crc = !super::super::crc32_raw(!0, data);
            let off = out.len() as u32;
            let mut h = b"PK\x03\x04\x14\0\0\0\x08\0\0\0\0\0".to_vec();
            h.extend_from_slice(&crc.to_le_bytes());
            h.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(n.len() as u16).to_le_bytes());
            h.extend_from_slice(&[0, 0]);
            h.extend_from_slice(n.as_bytes());
            out.extend_from_slice(&h);
            out.extend_from_slice(&packed);
            let mut c = b"PK\x01\x02\x14\0\x14\0\0\0\x08\0\0\0\0\0".to_vec();
            c.extend_from_slice(&crc.to_le_bytes());
            c.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            c.extend_from_slice(&(data.len() as u32).to_le_bytes());
            c.extend_from_slice(&(n.len() as u16).to_le_bytes());
            c.extend_from_slice(&[0; 12]);
            c.extend_from_slice(&off.to_le_bytes());
            c.extend_from_slice(n.as_bytes());
            central.extend_from_slice(&c);
        }
        let cd = out.len() as u32;
        out.extend_from_slice(&central);
        out.extend_from_slice(b"PK\x05\x06\0\0\0\0");
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]); // the game writes a zero size
        out.extend_from_slice(&cd.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    fn save() -> Vec<u8> {
        let mut c = b"CobraSav\x03\x04\xf3[1.4.3.37142F]\x00\x01Header>IDs>\x00Strings>\x03".to_vec();
        for n in ["Tacano", "Matanceros", "Nublar"] {
            c.push(0xf3);
            c.extend_from_slice(n.as_bytes());
            c.push(0);
        }
        c.extend_from_slice(b"WString>\x00Hierarchy>\x00ClientSizes>\x02$\x1f<HeaderClient>>");
        c.extend_from_slice(b"\xf3GlobalMoney\x00\x02\x04");
        c.extend_from_slice(&[0, 0xe4, 0x2b, 0x7b, 0xf0, 1, 0xe5, 0xf5, 0xe0, 0xff, 2, 0xe0, 0x00, 0x13, 0x88]);
        c.extend_from_slice(b"\xf3Pena\x00\xe0\x95\x58\x3e><<Client\xff");
        zip(&[
            ("island1", cobra(b"island data")),
            ("metadata", cobra(br#"{"version":3,"career":"Campaign","playtime":{"hours":42,"mins":21},"island":"Nublar"}"#)),
            ("careerc", cobra(&c)),
            ("careerdb", cobra(b"db")),
        ])
    }

    #[test]
    fn reads_and_edits_island_money_keeping_the_rest() {
        let d = save();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Money/Tacano"), Some(&Value::from(69_958_640)));
        assert_eq!(doc.view.pointer("/Money/Nublar"), Some(&Value::from(5000)));
        assert_eq!(doc.view.pointer("/Money/Pena"), Some(&Value::from(9_787_454)));
        assert_eq!(doc.view.pointer("/Career/Hours played"), Some(&Value::from(42)));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Money/Matanceros", Value::from(100_000_000)).unwrap();
        assert!(doc.set("/Money/Tacano", Value::from(0x1000_0000)).is_err());
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        let re = parse(&out).unwrap();
        assert_eq!(re.view.pointer("/Money/Matanceros"), Some(&Value::from(100_000_000)));
        assert_eq!(re.view.pointer("/Money/Tacano"), Some(&Value::from(69_958_640)));
        // The other entries are untouched, byte for byte.
        let (a, _) = entries(&d).unwrap();
        let (b, _) = entries(&out).unwrap();
        for i in [0, 1, 3] {
            assert_eq!(&d[a[i].start..a[i].end], &out[b[i].start..b[i].end]);
        }
        let careerc = read(&out, &b[2]).unwrap();
        assert!(cobra_ok(&careerc), "header sum rewritten");
    }

    #[test]
    fn other_zips_are_not_this_game() {
        let z = zip(&[("a.txt", b"x".to_vec())]);
        assert!(!looks(&z));
        assert!(!looks(b"PK"));
    }

    /// `JWE_SAVE=<file.career> cargo test --lib jwe -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_save_is_recognised() {
        let Ok(p) = std::env::var("JWE_SAVE") else { return };
        let d = std::fs::read(p).unwrap();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        println!("{}", doc.view);
        assert_eq!(doc.to_bytes().unwrap(), d);
        if doc.view.pointer("/Money").is_some() {
            let first = doc.view["Money"].as_object().unwrap().keys().next().unwrap().clone();
            doc.set(&format!("/Money/{first}"), Value::from(99_999_999)).unwrap();
            let out = doc.to_bytes().unwrap();
            let re = parse(&out).unwrap();
            assert_eq!(re.view.pointer(&format!("/Money/{first}")), Some(&Value::from(99_999_999)));
            println!("rewritten {} -> {} bytes", d.len(), out.len());
        }
    }
}
