//! Total War campaign saves (Warhammer II, Troy: Creative Assembly's "ESF" `ABCA` files).
//!
//! The file is a tree of typed nodes: records (a name from the name table, a version, a
//! size and children), record blocks (arrays of sized entries) and values, many of them
//! stored in the shortest form that fits (an `i32` may take zero to four bytes). The header
//! holds the file's edit time and the position of the name tables, which follow the tree.
//! The campaign itself is a second ESF file, LZMA-compressed inside the first one's
//! `COMPRESSED_DATA` record. A faction's treasury is the first integer after the `history`
//! block of its `FACTION_ECONOMICS` record.
//!
//! Layout after Creative Assembly's own file structure as documented by the PackFileManager
//! "EsfLib" and the MIT-licensed total-war-save-parser (IMRZ, MrSoul42); this is a new
//! implementation. Edits change a value's size, so every size field above it is recomputed
//! and the compressed block is made again.

use crate::tree::{Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub const KIND: &str = "Total War campaign save";
const HEADER: usize = 16;

fn varint(d: &[u8], mut p: usize) -> Option<(usize, usize)> {
    let start = p;
    let mut v = 0usize;
    loop {
        let b = *d.get(p)?;
        p += 1;
        v = v.checked_mul(128)? + (b & 0x7f) as usize;
        if b & 0x80 == 0 {
            return Some((v, p - start));
        }
        if p - start > 8 {
            return None;
        }
    }
}

fn put_varint(mut v: usize) -> Vec<u8> {
    let mut groups = vec![(v & 0x7f) as u8];
    v >>= 7;
    while v > 0 {
        groups.push((v & 0x7f) as u8 | 0x80);
        v >>= 7;
    }
    groups.reverse();
    groups
}

fn u16_at(d: &[u8], p: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(p..p + 2)?.try_into().ok()?) as usize)
}
fn u32_at(d: &[u8], p: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(p..p + 4)?.try_into().ok()?) as usize)
}

/// The name tables at the end of an ESF file.
#[allow(dead_code)]
struct Esf<'a> {
    d: &'a [u8],
    names: Vec<String>,
    ascii: HashMap<usize, String>,
    /// Where the tree ends (the name tables start).
    end: usize,
}

fn open(d: &[u8]) -> Option<Esf<'_>> {
    if u32_at(d, 0)? != 0xABCA {
        return None;
    }
    let off = u32_at(d, 12)?;
    let mut p = off;
    let n = u16_at(d, p)?;
    p += 2;
    let mut names = Vec::with_capacity(n);
    for _ in 0..n {
        let l = u16_at(d, p)?;
        names.push(String::from_utf8_lossy(d.get(p + 2..p + 2 + l)?).into_owned());
        p += 2 + l;
    }
    let c = u32_at(d, p)?;
    p += 4;
    for _ in 0..c {
        let l = u16_at(d, p)?;
        p += 2 + 2 * l + 4;
    }
    let c = u32_at(d, p)?;
    p += 4;
    let mut ascii = HashMap::new();
    for _ in 0..c {
        let l = u16_at(d, p)?;
        let s = String::from_utf8_lossy(d.get(p + 2..p + 2 + l)?).into_owned();
        p += 2 + l;
        ascii.insert(u32_at(d, p)?, s);
        p += 4;
    }
    (p <= d.len()).then_some(Esf { d, names, ascii, end: off })
}

#[derive(Clone, Copy, Debug)]
struct Rec {
    start: usize,
    name: usize,
    size_at: usize,
    size_len: usize,
    body: usize,
    end: usize,
    block: bool,
}

#[derive(Clone, Copy, Debug)]
enum Node {
    Rec(Rec),
    Val { start: usize, end: usize, t: u8 },
}

fn rec_at(d: &[u8], start: usize) -> Option<Rec> {
    let t = *d.get(start)?;
    if t & 0x80 == 0 {
        return None;
    }
    let mut p = start + 1;
    let name = if start == HEADER || t & 0x20 != 0 {
        let n = u16_at(d, p)?;
        p += 3;
        n
    } else {
        let n = ((t as usize & 1) << 8) | *d.get(p)? as usize;
        p += 1;
        n
    };
    let (size, len) = varint(d, p)?;
    let body = p + len;
    // A block's size doesn't count its entry count, which comes first.
    let count_len = if t & 0x40 != 0 { varint(d, body)?.1 } else { 0 };
    let end = body.checked_add(count_len)?.checked_add(size)?;
    (end <= d.len()).then_some(Rec { start, name, size_at: p, size_len: len, body, end, block: t & 0x40 != 0 })
}

/// End of the value of type `t` whose payload starts at `p`.
fn skip_val(d: &[u8], p: usize, t: u8) -> Option<usize> {
    Some(match t {
        0x01 | 0x02 | 0x06 | 0x16 | 0x1a | 0x23 => p + 1,
        0x03 | 0x07 | 0x10 | 0x17 | 0x1b | 0x24 => p + 2,
        0x18 | 0x1c => p + 3,
        0x04 | 0x08 | 0x0a | 0x0e | 0x0f | 0x21 | 0x25 => p + 4,
        0x05 | 0x09 | 0x0b | 0x0c => p + 8,
        0x0d => p + 12,
        0x12 | 0x13 | 0x14 | 0x15 | 0x19 | 0x1d => p,
        0x26 => {
            let f = *d.get(p)?;
            p + 1 + if f == 16 { 16 } else if f == 8 { 8 } else { 7 }
        }
        0x41..=0x7f => {
            let (n, l) = varint(d, p)?;
            p + l + n
        }
        _ => return None,
    })
}

fn children(d: &[u8], mut p: usize, end: usize) -> Option<Vec<Node>> {
    let mut v = vec![];
    while p < end {
        let t = *d.get(p)?;
        if t & 0x80 != 0 {
            let r = rec_at(d, p)?;
            p = r.end;
            v.push(Node::Rec(r));
        } else {
            let e = match skip_val(d, p + 1, t) {
                Some(e) => e,
                None => {
                    #[cfg(test)]
                    eprintln!("unknown value type {t:#x} at {p}");
                    return None;
                }
            };
            v.push(Node::Val { start: p, end: e, t });
            p = e;
        }
        if p > end {
            return None;
        }
    }
    Some(v)
}

/// The entries of a record block: `(size_at, size_len, body, end)` of each.
fn block_items(d: &[u8], r: &Rec) -> Option<Vec<(usize, usize, usize, usize)>> {
    let (n, l) = varint(d, r.body)?;
    let mut p = r.body + l;
    let mut v = Vec::with_capacity(n.min(1 << 20));
    for _ in 0..n {
        let (s, sl) = varint(d, p)?;
        v.push((p, sl, p + sl, p + sl + s));
        p += sl + s;
    }
    (p == r.end).then_some(v)
}

fn child_rec(e: &Esf, kids: &[Node], name: &str) -> Option<Rec> {
    kids.iter().find_map(|n| match n {
        Node::Rec(r) if e.names.get(r.name).is_some_and(|s| s == name) => Some(*r),
        _ => None,
    })
}

/// Reads a value node as an integer, with its type.
fn int_val(d: &[u8], start: usize, t: u8) -> Option<i64> {
    let p = start + 1;
    Some(match t {
        0x19 | 0x14 => 0,
        0x15 => 1,
        0x1a => d[p] as i8 as i64,
        0x1b => i16::from_le_bytes(d.get(p..p + 2)?.try_into().ok()?) as i64,
        0x1c => {
            let b = d.get(p..p + 3)?;
            let v = (((b[0] & 0x7f) as i64) << 16) | ((b[1] as i64) << 8) | b[2] as i64;
            if b[0] & 0x80 != 0 {
                -v
            } else {
                v
            }
        }
        0x04 => i32::from_le_bytes(d.get(p..p + 4)?.try_into().ok()?) as i64,
        0x02 => d[p] as i8 as i64,
        0x03 => i16::from_le_bytes(d.get(p..p + 2)?.try_into().ok()?) as i64,
        _ => return None,
    })
}

fn is_signed_int(t: u8) -> bool {
    matches!(t, 0x19 | 0x1a | 0x1b | 0x1c | 0x04)
}

/// One faction's treasury, with every size field above it.
struct Treasury {
    name: String,
    val: (usize, usize, u8),
    /// `(size_at, size_len)` of each enclosing size field, outermost first.
    chain: Vec<(usize, usize)>,
}

fn rec_chain(r: &Rec) -> (usize, usize) {
    (r.size_at, r.size_len)
}

fn treasuries(d: &[u8]) -> Result<(Vec<Treasury>, Option<String>), String> {
    let e = open(d).ok_or("not a Total War save")?;
    let root = rec_at(d, HEADER).ok_or("no root record")?;
    let top = children(d, root.body, root.end).ok_or("the save's tree couldn't be read")?;
    let env = child_rec(&e, &top, "CAMPAIGN_ENV").ok_or("no campaign in this save")?;
    let mut chain = vec![rec_chain(&root), rec_chain(&env)];
    let kids = children(d, env.body, env.end).ok_or("bad campaign")?;
    let model = child_rec(&e, &kids, "CAMPAIGN_MODEL").ok_or("no campaign model")?;
    chain.push(rec_chain(&model));
    let kids = children(d, model.body, model.end).ok_or("bad campaign model")?;
    let world = child_rec(&e, &kids, "WORLD").ok_or("no world")?;
    chain.push(rec_chain(&world));
    let kids = children(d, world.body, world.end).ok_or("bad world")?;
    let fa = child_rec(&e, &kids, "FACTION_ARRAY").ok_or("no factions")?;
    chain.push(rec_chain(&fa));
    let items = block_items(d, &fa).ok_or("bad faction list")?;
    let mut out = vec![];
    for (sa, sl, body, end) in items {
        let ks = children(d, body, end).ok_or("bad faction")?;
        let Some(f) = child_rec(&e, &ks, "FACTION") else { continue };
        let fk = children(d, f.body, f.end).ok_or("bad faction")?;
        // Three Kingdoms keeps the economics inside a FACTION_ECONOMICS_MANAGER record.
        let mut manager = None;
        let eco = match child_rec(&e, &fk, "FACTION_ECONOMICS") {
            Some(r) => r,
            None => {
                let Some(m) = child_rec(&e, &fk, "FACTION_ECONOMICS_MANAGER") else { continue };
                let Some(mk) = children(d, m.body, m.end) else { continue };
                let Some(r) = child_rec(&e, &mk, "FACTION_ECONOMICS") else { continue };
                manager = Some(m);
                r
            }
        };
        let name = fk
            .iter()
            .find_map(|n| match n {
                Node::Val { start, t: 0x0f | 0x21 | 0x25, .. } => u32_at(d, start + 1).and_then(|i| e.ascii.get(&i).cloned()),
                _ => None,
            })
            .unwrap_or_default();
        let ek = children(d, eco.body, eco.end).ok_or("bad economics")?;
        let mut after_block = false;
        let mut found = None;
        for n in &ek {
            match n {
                Node::Rec(r) if r.block => after_block = true,
                Node::Val { start, end, t } if after_block && is_signed_int(*t) => {
                    found = Some((*start, *end, *t));
                    break;
                }
                _ => {}
            }
        }
        let Some(val) = found else { continue };
        let mut c = chain.clone();
        c.push((sa, sl));
        c.push(rec_chain(&f));
        if let Some(m) = &manager {
            c.push(rec_chain(m));
        }
        c.push(rec_chain(&eco));
        out.push(Treasury { name, val, chain: c });
    }
    if out.is_empty() {
        return Err("no faction treasuries found".into());
    }
    // The player's faction: the text value of the save header that names a faction (Three Kingdoms
    // starts its header with the game version, which is not one).
    let player = child_rec(&e, &top, "SAVE_GAME_HEADER").and_then(|h| {
        let texts: Vec<String> = children(d, h.body, h.end)?
            .iter()
            .filter_map(|n| match n {
                Node::Val { start, t: 0x0f | 0x21 | 0x25, .. } => u32_at(d, start + 1).and_then(|i| e.ascii.get(&i).cloned()),
                _ => None,
            })
            .collect();
        texts.iter().find(|t| out.iter().any(|f| &f.name == *t)).cloned()
    });
    Ok((out, player))
}

/// Sets the value of treasury `i`, resizing every record above it.
fn set_treasury(d: &[u8], i: usize, v: i32) -> Result<Vec<u8>, String> {
    let (ts, _) = treasuries(d)?;
    let t = ts.get(i).ok_or("faction not found")?;
    let mut new = vec![0x04];
    new.extend(v.to_le_bytes());
    let delta = new.len() as i64 - (t.val.1 - t.val.0) as i64;
    let mut p = Patches::default();
    p.add(t.val.0, t.val.1, new);
    let mut acc = delta;
    for &(at, len) in t.chain.iter().rev() {
        let (old, _) = varint(d, at).ok_or("bad size")?;
        let ns = (old as i64 + acc) as usize;
        let enc = put_varint(ns);
        acc += enc.len() as i64 - len as i64;
        p.add(at, at + len, enc);
    }
    let mut out = p.apply(d)?;
    let off = u32_at(&out, 12).ok_or("bad header")? as i64 + acc;
    out[12..16].copy_from_slice(&(off as u32).to_le_bytes());
    Ok(out)
}

// ---------------------------------------------------------------- outer wrapper

struct Wrapped {
    inner: Vec<u8>,
}

/// The campaign ESF inside the outer file's LZMA block.
fn unwrap(d: &[u8]) -> Result<Wrapped, String> {
    let e = open(d).ok_or("not a Total War save")?;
    let root = rec_at(d, HEADER).ok_or("no root record")?;
    let top = children(d, root.body, root.end).ok_or("bad save")?;
    let cd = child_rec(&e, &top, "COMPRESSED_DATA").ok_or("this save has no compressed campaign")?;
    let kids = children(d, cd.body, cd.end).ok_or("bad compressed block")?;
    let (Some(Node::Val { start: a0, end: a1, .. }), Some(info)) = (kids.first().copied(), child_rec(&e, &kids, "COMPRESSED_DATA_INFO")) else {
        return Err("bad compressed block".into());
    };
    let (n, l) = varint(d, a0 + 1).ok_or("bad data")?;
    let data = d.get(a0 + 1 + l..a0 + 1 + l + n).ok_or("bad data")?;
    let _ = a1;
    let ik = children(d, info.body, info.end).ok_or("bad info")?;
    let size = match ik.first() {
        Some(Node::Val { start, t, .. }) => match t {
            0x08 => u32_at(d, start + 1).ok_or("bad size")?,
            0x14 => 0,
            0x15 => 1,
            0x16 => d[start + 1] as usize,
            0x17 => u16_at(d, start + 1).ok_or("bad size")?,
            0x18 => ((d[start + 1] as usize) << 16) | ((d[start + 2] as usize) << 8) | d[start + 3] as usize,
            _ => return Err("unknown size encoding".into()),
        },
        _ => return Err("no size".into()),
    };
    let props = match ik.get(1) {
        Some(Node::Val { start, .. }) => {
            let (n, l) = varint(d, start + 1).ok_or("bad props")?;
            d.get(start + 1 + l..start + 1 + l + n).ok_or("bad props")?
        }
        _ => return Err("no properties".into()),
    };
    if props.len() != 5 {
        return Err("unexpected compression properties".into());
    }
    let mut stream = props.to_vec();
    stream.extend((size as u64).to_le_bytes());
    stream.extend(data);
    let mut inner = Vec::with_capacity(size);
    lzma_rs::lzma_decompress(&mut &stream[..], &mut inner).map_err(|e| format!("the campaign couldn't be decompressed: {e}"))?;
    if inner.len() != size {
        return Err("the campaign has the wrong size".into());
    }
    Ok(Wrapped { inner })
}

/// The outer file with its compressed block replaced by `inner`, compressed again.
fn rewrap(d: &[u8], inner: &[u8]) -> Result<Vec<u8>, String> {
    let e = open(d).ok_or("not a Total War save")?;
    let root = rec_at(d, HEADER).ok_or("no root record")?;
    let top = children(d, root.body, root.end).ok_or("bad save")?;
    let cd = child_rec(&e, &top, "COMPRESSED_DATA").ok_or("no compressed block")?;
    let kids = children(d, cd.body, cd.end).ok_or("bad compressed block")?;
    let info = child_rec(&e, &kids, "COMPRESSED_DATA_INFO").ok_or("no info")?;
    let ik = children(d, info.body, info.end).ok_or("bad info")?;
    let (Some(Node::Val { start: a0, .. }), Some(Node::Val { start: s0, end: s1, .. }), Some(Node::Val { start: p0, end: p1, .. })) = (kids.first().copied(), ik.first().copied(), ik.get(1).copied()) else {
        return Err("bad compressed block".into());
    };
    let mut packed = Vec::new();
    lzma_rs::lzma_compress_with_options(&mut &inner[..], &mut packed, &lzma_rs::compress::Options { unpacked_size: lzma_rs::compress::UnpackedSize::WriteToHeader(Some(inner.len() as u64)) })
        .map_err(|e| format!("the campaign couldn't be compressed: {e}"))?;
    let (props, data) = (&packed[..5], &packed[13..]);
    // New COMPRESSED_DATA_INFO body: size, then the properties array as it was.
    let _ = (p1, s1);
    let mut info_body = vec![0x08];
    info_body.extend((inner.len() as u32).to_le_bytes());
    info_body.push(d[*&p0]);
    info_body.extend(put_varint(5));
    info_body.extend(props);
    let _ = s0;
    let mut info_rec = d[info.start..info.size_at].to_vec();
    info_rec.extend(put_varint(info_body.len()));
    info_rec.extend(info_body);
    let mut arr = vec![d[a0]];
    arr.extend(put_varint(data.len()));
    arr.extend(data);
    let mut cd_body = arr;
    cd_body.extend(info_rec);
    let mut cd_rec = d[cd.start..cd.size_at].to_vec();
    cd_rec.extend(put_varint(cd_body.len()));
    cd_rec.extend(cd_body);
    let delta = cd_rec.len() as i64 - (cd.end - cd.start) as i64;
    let mut p = Patches::default();
    p.add(cd.start, cd.end, cd_rec);
    let (old, _) = varint(d, root.size_at).ok_or("bad size")?;
    let enc = put_varint((old as i64 + delta) as usize);
    let acc = delta + enc.len() as i64 - root.size_len as i64;
    p.add(root.size_at, root.size_at + root.size_len, enc);
    let mut out = p.apply(d)?;
    let off = u32_at(&out, 12).ok_or("bad header")? as i64 + acc;
    out[12..16].copy_from_slice(&(off as u32).to_le_bytes());
    Ok(out)
}

// ---------------------------------------------------------------- document

#[derive(Clone, Debug)]
struct TwBackend {
    outer: Arc<Vec<u8>>,
    inner: Arc<Vec<u8>>,
    /// Faction index of each editable leaf.
    leaves: Vec<usize>,
}

impl Backend for TwBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.outer.to_vec());
        }
        let mut inner = self.inner.to_vec();
        for (leaf, v) in changes {
            let n = v.as_i64().ok_or("expected a whole number")?;
            if !(0..=i64::from(i32::MAX)).contains(&n) {
                return Err("must be between 0 and 2147483647".into());
            }
            inner = set_treasury(&inner, self.leaves[*leaf], n as i32)?;
        }
        rewrap(&self.outer, &inner)
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        let n = v.as_i64().ok_or("expected a whole number")?;
        if !(0..=i64::from(i32::MAX)).contains(&n) {
            return Err("must be between 0 and 2147483647".into());
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && u32_at(d, 0) == Some(0xABCA) && open(d).is_some_and(|e| e.names.first().is_some_and(|n| n == "CAMPAIGN_SAVE_GAME") || e.names.iter().any(|n| n == "COMPRESSED_DATA"))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let w = unwrap(d)?;
    let (ts, player) = treasuries(&w.inner)?;
    let e = open(&w.inner).ok_or("bad campaign")?;
    let mut view = Map::new();
    let mut save = Map::new();
    if let Some(root) = rec_at(&w.inner, HEADER) {
        if let Some(kids) = children(&w.inner, root.body, root.end) {
            if let Some(b) = child_rec(&e, &kids, "BUILD") {
                if let Some(Node::Val { start, .. }) = children(&w.inner, b.body, b.end).and_then(|k| k.first().copied()) {
                    let (_, l) = varint(&w.inner, start + 1).unwrap_or((0, 0));
                    if let Some(t) = u32_at(&w.inner, start + 1 + l).and_then(|i| e.ascii.get(&i)) {
                        save.insert("Build".into(), t.clone().into());
                    }
                }
            }
        }
    }
    save.insert("Player faction".into(), player.clone().unwrap_or_default().into());
    view.insert("Save".into(), Value::Object(save));
    let mut factions = Map::new();
    let mut editable = HashMap::new();
    let mut leaves = vec![];
    let mut order: Vec<usize> = (0..ts.len()).collect();
    order.sort_by_key(|&i| (Some(&ts[i].name) != player.as_ref(), i));
    for i in order {
        let t = &ts[i];
        let amount = int_val(&w.inner, t.val.0, t.val.2).unwrap_or(0);
        let mut f = Map::new();
        f.insert("Treasury".into(), amount.into());
        let key = crate::tree::unique_key(&factions, &t.name);
        editable.insert(format!("/Factions/{}/Treasury", crate::tree::esc(&key)), leaves.len());
        leaves.push(i);
        factions.insert(key, Value::Object(f));
    }
    view.insert("Factions".into(), Value::Object(factions));
    let backend = TwBackend { outer: Arc::new(d.to_vec()), inner: Arc::new(w.inner), leaves };
    Ok(TreeDoc::new(Value::Object(view), editable, KIND, Box::new(backend)).with_note(
        "Each faction's treasury can be changed (the first one is yours). The save is repacked on write: the campaign is compressed again, so the file gets larger. Not yet tried in the game.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints() {
        for v in [0usize, 1, 127, 128, 300, 16383, 16384, 1 << 28] {
            let e = put_varint(v);
            assert_eq!(varint(&e, 0), Some((v, e.len())));
        }
    }

    /// Needs a real save: TWSAVE=<file> cargo test --release --lib totalwar -- --ignored
    #[test]
    #[ignore]
    fn real_save() {
        let path = std::env::var("TWSAVE").unwrap();
        let d = std::fs::read(path).unwrap();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        println!("{}", serde_json::to_string(&doc.view["Save"]).unwrap());
        let (key, first) = {
            let f = doc.view["Factions"].as_object().unwrap();
            let (k, v) = f.iter().next().unwrap();
            (k.clone(), v["Treasury"].as_i64().unwrap())
        };
        println!("{key}: {first}");
        assert_eq!(doc.to_bytes().unwrap(), d, "unchanged save is written back as it was");
        let p = format!("/Factions/{}/Treasury", crate::tree::esc(&key));
        doc.set(&p, 1_234_567.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer(&p), Some(&1_234_567.into()));
        let (a, _) = treasuries(&unwrap(&d).unwrap().inner).unwrap();
        let (b, _) = treasuries(&unwrap(&out).unwrap().inner).unwrap();
        assert_eq!(a.len(), b.len());
        println!("{} -> {} bytes", d.len(), out.len());
    }
}
