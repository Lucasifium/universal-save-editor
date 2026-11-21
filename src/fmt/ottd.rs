//! OpenTTD saved games (`.sav`): a four-letter compression tag (`OTTZ` zlib, `OTTX` xz, `OTTS`
//! zstd, `OTTN` none), the savegame version, then a list of chunks. Packed files are shown as
//! the same file with the tag `OTTN` and the data unpacked, and packed the same way again on
//! saving.
//!
//! Every chunk can be walked without knowing its contents (a RIFF chunk carries its length,
//! the array and table kinds carry a length for each record). Only the company chunk `PLYR` is
//! read further: money, loan and the maximum loan are fixed-size integers changed in place, so
//! no length ever changes. Saves from version 295 on describe their records in a table header,
//! so fields are found by name; older ones are read by the documented field order (version 84
//! onward). The chunk format and field order are those of OpenTTD (GPL-2.0, facts only; see
//! THIRD_PARTY_NOTICES.md).

use crate::tree::{Backend, TreeDoc};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;

pub const KIND: &str = "OpenTTD save";
const LIMIT: u64 = 512 << 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    tag: [u8; 4],
    original: Arc<Vec<u8>>,
    file: Arc<Vec<u8>>,
}

pub fn maybe(d: &[u8]) -> bool {
    d.len() > 16 && matches!(&d[..4], b"OTTZ" | b"OTTX" | b"OTTS")
}

/// A writer that refuses more than its limit, so a small xz stream can't fill the memory.
struct Capped<'a> {
    out: &'a mut Vec<u8>,
    left: usize,
}

impl std::io::Write for Capped<'_> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if b.len() > self.left {
            return Err(std::io::Error::other("too large"));
        }
        self.left -= b.len();
        self.out.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !maybe(d) {
        return None;
    }
    let tag: [u8; 4] = d[..4].try_into().ok()?;
    let mut body = Vec::new();
    match &tag {
        b"OTTZ" => {
            ZlibDecoder::new(&d[8..]).take(LIMIT).read_to_end(&mut body).ok()?;
        }
        b"OTTX" => {
            let mut sink = Capped { out: &mut body, left: LIMIT as usize };
            lzma_rs::xz_decompress(&mut &d[8..], &mut sink).ok()?;
        }
        _ => {
            zstd::stream::read::Decoder::new(&d[8..]).ok()?.take(LIMIT).read_to_end(&mut body).ok()?;
        }
    }
    let mut inner = Vec::with_capacity(body.len() + 8);
    inner.extend_from_slice(b"OTTN");
    inner.extend_from_slice(&d[4..8]);
    inner.extend_from_slice(&body);
    if !looks(&inner) {
        return None;
    }
    Some((Packed { tag, original: Arc::new(inner.clone()), file: Arc::new(d.to_vec()) }, inner))
}

pub fn close(p: &Packed, data: &[u8]) -> Result<Vec<u8>, String> {
    if data == p.original.as_slice() {
        return Ok((*p.file).clone());
    }
    if data.len() < 8 {
        return Err("the saved game is cut off".into());
    }
    let mut out = p.tag.to_vec();
    out.extend_from_slice(&data[4..8]);
    let body = &data[8..];
    match &p.tag {
        b"OTTZ" => {
            let mut z = ZlibEncoder::new(out, Compression::default());
            z.write_all(body).map_err(|e| e.to_string())?;
            z.finish().map_err(|e| e.to_string())
        }
        b"OTTX" => {
            lzma_rs::xz_compress(&mut &body[..], &mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        _ => {
            out.extend(zstd::encode_all(body, 3).map_err(|e| e.to_string())?);
            Ok(out)
        }
    }
}

fn version(d: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(4..8)?.try_into().ok()?) >> 16)
}

/// The size-prefix scheme of OpenTTD: one to five bytes, the top bits of the first say how many.
fn gamma(d: &[u8], i: &mut usize) -> Option<u32> {
    let mut next = |i: &mut usize| -> Option<u32> {
        let b = *d.get(*i)?;
        *i += 1;
        Some(u32::from(b))
    };
    let mut v = next(i)?;
    if v & 0x80 != 0 {
        v &= !0x80;
        if v & 0x40 != 0 {
            v &= !0x40;
            if v & 0x20 != 0 {
                v &= !0x20;
                if v & 0x10 != 0 {
                    v &= !0x10;
                    if v & 0x08 != 0 {
                        return None;
                    }
                    v = next(i)?;
                }
                v = (v << 8) | next(i)?;
            }
            v = (v << 8) | next(i)?;
        }
        v = (v << 8) | next(i)?;
    }
    Some(v)
}

struct Chunk {
    tag: [u8; 4],
    kind: u8,
    /// Records of an array or table chunk: offset and length (a gap has length 0). For a table
    /// the first entry is the header. A RIFF chunk is one entry.
    records: Vec<(usize, usize)>,
}

fn chunks(d: &[u8]) -> Option<Vec<Chunk>> {
    if !d.starts_with(b"OTTN") || d.len() < 12 {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 8;
    loop {
        let tag: [u8; 4] = d.get(i..i + 4)?.try_into().ok()?;
        i += 4;
        if tag == [0; 4] {
            break;
        }
        if !tag.iter().all(|b| b.is_ascii_alphanumeric() || *b == b' ' || *b == b'_') {
            return None;
        }
        let m = *d.get(i)?;
        i += 1;
        let kind = m & 0x0f;
        let mut records = Vec::new();
        match kind {
            0 => {
                let len = ((usize::from(m >> 4)) << 24) + (usize::from(*d.get(i)?) << 16) + (usize::from(*d.get(i + 1)?) << 8) + usize::from(*d.get(i + 2)?);
                i += 3;
                d.get(i..i + len)?;
                records.push((i, len));
                i += len;
            }
            1..=4 => loop {
                let g = gamma(d, &mut i)? as usize;
                if g == 0 {
                    break;
                }
                let len = g - 1;
                d.get(i..i + len)?;
                records.push((i, len));
                i += len;
            },
            _ => return None,
        }
        out.push(Chunk { tag, kind, records });
    }
    // Nothing may follow the end marker.
    (i == d.len()).then_some(out)
}

pub fn looks(d: &[u8]) -> bool {
    version(d).is_some() && chunks(d).is_some_and(|c| !c.is_empty())
}

/// One field of a record described by a table header.
#[derive(Clone, Debug)]
struct Field {
    key: String,
    ty: u8,
    array: bool,
    sub: Option<Vec<Field>>,
}

fn header(d: &[u8], i: &mut usize, end: usize, depth: u8) -> Option<Vec<Field>> {
    if depth > 6 {
        return None;
    }
    let mut fields = Vec::new();
    loop {
        let t = *d.get(*i)?;
        *i += 1;
        if t == 0 {
            break;
        }
        let n = gamma(d, i)? as usize;
        let key = String::from_utf8_lossy(d.get(*i..*i + n)?).into_owned();
        *i += n;
        let (ty, array) = (t & 0x0f, t & 0x10 != 0);
        if !(1..=11).contains(&ty) || *i > end {
            return None;
        }
        fields.push(Field { key, ty, array, sub: None });
    }
    for f in fields.iter_mut() {
        if f.ty == 11 {
            let sub = header(d, i, end, depth + 1)?;
            // An empty struct would consume no bytes however often it repeats.
            if sub.is_empty() {
                return None;
            }
            f.sub = Some(sub);
        }
    }
    Some(fields)
}

#[derive(Clone, Copy, Debug)]
struct Int {
    at: usize,
    /// Size in bytes and whether it is signed.
    size: u8,
    signed: bool,
}

fn int_size(ty: u8) -> Option<(u8, bool)> {
    Some(match ty {
        1 => (1, true),
        2 => (1, false),
        3 => (2, true),
        4 | 9 => (2, false),
        5 => (4, true),
        6 => (4, false),
        7 => (8, true),
        8 => (8, false),
        _ => return None,
    })
}

fn read_int(d: &[u8], at: usize, size: u8, signed: bool) -> Option<i128> {
    // OpenTTD writes every number big-endian.
    let b = d.get(at..at + usize::from(size))?;
    let mut raw = [0u8; 8];
    raw[8 - b.len()..].copy_from_slice(b);
    let u = u64::from_be_bytes(raw);
    Some(if signed {
        let shift = 64 - 8 * u32::from(size);
        i128::from(((u << shift) as i64) >> shift)
    } else {
        i128::from(u)
    })
}

/// Reads one record by its header, collecting the fixed-size numbers by key path.
fn read_record(d: &[u8], i: &mut usize, end: usize, fields: &[Field], path: &str, out: &mut Map<String, Value>, ints: &mut Vec<(String, Int)>) -> Option<()> {
    for f in fields {
        let p = format!("{path}/{}", f.key);
        let count = if f.array { gamma(d, i)? as usize } else { 1 };
        if count > 1_000_000 {
            return None;
        }
        match f.ty {
            10 => {
                // A string: a length and the bytes (text of one-element fields is shown).
                for _ in 0..count {
                    let n = gamma(d, i)? as usize;
                    let s = String::from_utf8_lossy(d.get(*i..*i + n)?).into_owned();
                    *i += n;
                    if !f.array {
                        out.insert(f.key.clone(), Value::String(s));
                    }
                }
            }
            11 => {
                let sub = f.sub.as_ref()?;
                let mut list = Vec::new();
                for k in 0..count {
                    let mut m = Map::new();
                    read_record(d, i, end, sub, &format!("{p}/{k}"), &mut m, ints)?;
                    list.push(Value::Object(m));
                }
                if f.array {
                    out.insert(f.key.clone(), Value::Array(list));
                } else if let Some(Value::Object(m)) = list.into_iter().next() {
                    out.insert(f.key.clone(), Value::Object(m));
                }
            }
            ty => {
                let (size, signed) = int_size(ty)?;
                let mut vals = Vec::new();
                for k in 0..count {
                    let v = read_int(d, *i, size, signed)?;
                    if !f.array && k == 0 {
                        ints.push((p.clone(), Int { at: *i, size, signed }));
                    }
                    *i += usize::from(size);
                    vals.push(Value::from(v as i64));
                }
                if *i > end {
                    return None;
                }
                if f.array {
                    // Arrays of numbers (yearly figures) are not shown.
                    out.insert(f.key.clone(), Value::String(format!("{} values", vals.len())));
                } else {
                    out.insert(f.key.clone(), vals.into_iter().next().unwrap_or(Value::Null));
                }
            }
        }
    }
    Some(())
}

#[derive(Clone, Debug)]
struct OttdBackend {
    data: Arc<Vec<u8>>,
    ints: Vec<Int>,
}

impl Backend for OttdBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = (*self.data).clone();
        for (leaf, v) in changes {
            let int = self.ints[*leaf];
            let n = v.as_i64().ok_or("expected a whole number")?;
            let bytes = (n as i128).to_be_bytes();
            let size = usize::from(int.size);
            let dst = out.get_mut(int.at..int.at + size).ok_or("the save is cut off")?;
            dst.copy_from_slice(&bytes[16 - size..]);
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let int = self.ints[leaf];
        let n = v.as_i64().ok_or("expected a whole number")?;
        let bits = 8 * u32::from(int.size);
        let (lo, hi): (i128, i128) = if int.signed { (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1) } else { (0, (1i128 << bits) - 1) };
        // Money stays far below what the game's own sums could overflow.
        let (lo, hi) = (lo.max(-10_000_000_000_000), hi.min(10_000_000_000_000));
        if (n as i128) < lo || (n as i128) > hi {
            return Err(format!("must be between {lo} and {hi}"));
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// A company record of a save made before table headers (version 84 to 294).
fn legacy_company(d: &[u8], at: usize, len: usize, ver: u32, ints: &mut Vec<(String, Int)>, idx: usize, number: usize) -> Option<Map<String, Value>> {
    if !(84..295).contains(&ver) {
        return None;
    }
    let end = at + len;
    let mut i = at + 4 + 2; // name_2, name_1
    let n = gamma(d, &mut i)? as usize;
    let name = String::from_utf8_lossy(d.get(i..i + n)?).into_owned();
    i += n;
    i += 2 + 4; // president_name_1, president_name_2
    let n = gamma(d, &mut i)? as usize;
    i += n;
    i += 4; // face
    let money_at = i;
    let money = read_int(d, money_at, 8, true)?;
    // The loan is 64 bits from version 65 on.
    let loan = read_int(d, money_at + 8, 8, true)?;
    if money_at + 16 > end || !(-10_000_000_000_000_000..=10_000_000_000_000_000).contains(&money) || !(0..=10_000_000_000_000).contains(&loan) {
        return None;
    }
    let mut m = Map::new();
    m.insert("name".into(), Value::String(if name.is_empty() { format!("Company {}", number + 1) } else { name }));
    m.insert("money".into(), Value::from(money as i64));
    m.insert("current_loan".into(), Value::from(loan as i64));
    ints.push((format!("/Companies/{idx}/money"), Int { at: money_at, size: 8, signed: true }));
    ints.push((format!("/Companies/{idx}/current_loan"), Int { at: money_at + 8, size: 8, signed: true }));
    Some(m)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let ver = version(d).ok_or("not an OpenTTD save")?;
    let cs = chunks(d).ok_or("not an OpenTTD save")?;
    let plyr = cs.iter().find(|c| &c.tag == b"PLYR").ok_or("no companies in this save")?;
    let mut companies = Vec::new();
    let mut ints: Vec<(String, Int)> = Vec::new();
    let table = matches!(plyr.kind, 3 | 4);
    let mut records = plyr.records.iter();
    let desc = if table {
        let &(at, len) = records.next().ok_or("a table without a header")?;
        let mut i = at;
        Some(header(d, &mut i, at + len, 0).ok_or("the company table header is not readable")?)
    } else {
        None
    };
    let sparse = matches!(plyr.kind, 2 | 4);
    for (n, &(at, len)) in records.enumerate() {
        if len == 0 {
            continue;
        }
        let (mut idx, mut start) = (n, at);
        if sparse {
            let mut i = at;
            idx = gamma(d, &mut i).ok_or("bad company index")? as usize;
            start = i;
        }
        let end = at + len;
        let m = match &desc {
            Some(fields) => {
                let mut m = Map::new();
                let mut i = start;
                let mut found = Vec::new();
                if read_record(d, &mut i, end, fields, &format!("/Companies/{}", companies.len()), &mut m, &mut found).is_none() {
                    continue;
                }
                // Only the figures worth editing, kept at the top of the record.
                let keep = ["money", "current_loan", "max_loan"];
                for (p, int) in found {
                    if keep.iter().any(|k| p.ends_with(&format!("/{k}"))) && p.matches('/').count() == 3 {
                        ints.push((p, int));
                    }
                }
                let name = m.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                m.insert("name".into(), Value::String(if name.is_empty() { format!("Company {}", idx + 1) } else { name }));
                Some(m)
            }
            None => legacy_company(d, start, end - start, ver, &mut ints, companies.len(), idx),
        };
        if let Some(mut m) = m {
            m.insert("company".into(), Value::from(idx as i64 + 1));
            companies.push(Value::Object(m));
        }
    }
    if companies.is_empty() {
        return Err("no readable company".into());
    }
    let mut editable = HashMap::new();
    let mut leaves = Vec::new();
    for (p, int) in ints {
        editable.insert(p, leaves.len());
        leaves.push(int);
    }
    let mut root = Map::new();
    root.insert("Savegame version".into(), Value::from(ver));
    root.insert("Companies".into(), Value::Array(companies));
    let note = crate::i18n::tx("Close the game first. Money is in the game's own currency units (pounds). The save is packed the same way it was before.");
    Ok(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(OttdBackend { data: Arc::new(d.to_vec()), ints: leaves })).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_gamma(out: &mut Vec<u8>, v: usize) {
        if v < 0x80 {
            out.push(v as u8);
        } else {
            out.push(0x80 | (v >> 8) as u8);
            out.push(v as u8);
        }
    }

    fn text(out: &mut Vec<u8>, s: &str) {
        put_gamma(out, s.len());
        out.extend_from_slice(s.as_bytes());
    }

    /// A version 196 company record.
    fn old_company(name: &str, money: i64, loan: i64) -> Vec<u8> {
        let mut r = vec![0, 0, 0, 0, 0x12, 0x34];
        text(&mut r, name);
        r.extend_from_slice(&[0, 0, 5, 6, 7, 8]);
        text(&mut r, "");
        r.extend_from_slice(&[1, 2, 3, 4]);
        r.extend_from_slice(&money.to_be_bytes());
        r.extend_from_slice(&loan.to_be_bytes());
        r.extend_from_slice(&[0; 20]);
        r
    }

    fn stream(version: u32, chunks: &[(&[u8; 4], u8, Vec<Vec<u8>>)]) -> Vec<u8> {
        let mut d = b"OTTN".to_vec();
        d.extend_from_slice(&(version << 16).to_be_bytes());
        for (tag, kind, recs) in chunks {
            d.extend_from_slice(*tag);
            d.push(*kind);
            if *kind == 0 {
                let r = &recs[0];
                d.extend_from_slice(&[(r.len() >> 16) as u8, (r.len() >> 8) as u8, r.len() as u8]);
                d.extend_from_slice(r);
            } else {
                for r in recs {
                    put_gamma(&mut d, r.len() + 1);
                    d.extend_from_slice(r);
                }
                d.push(0);
            }
        }
        d.extend_from_slice(&[0; 4]);
        d
    }

    #[test]
    fn an_old_save_is_read_and_edited_in_place() {
        let d = stream(196, &[(b"MAPT", 0, vec![vec![1, 2, 3, 4, 5]]), (b"PLYR", 1, vec![old_company("Acme Rail", 130_000, 100_000), vec![], old_company("", -500, 0)])]);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Companies/0/money").unwrap(), 130_000);
        assert_eq!(doc.view.pointer("/Companies/0/name").unwrap(), "Acme Rail");
        assert_eq!(doc.view.pointer("/Companies/1/name").unwrap(), "Company 3");
        assert_eq!(doc.view.pointer("/Companies/1/money").unwrap(), -500);
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Companies/1/money", serde_json::json!(9_000_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(parse(&out).unwrap().view.pointer("/Companies/1/money").unwrap(), 9_000_000);
        assert_eq!(parse(&out).unwrap().view.pointer("/Companies/0/money").unwrap(), 130_000);
    }

    #[test]
    fn a_table_save_is_read_by_field_names() {
        let mut head = Vec::new();
        for (t, k) in [(10u8, "name"), (7, "money"), (7, "current_loan"), (7, "max_loan"), (4, "face")] {
            head.push(t);
            text(&mut head, k);
        }
        head.push(0);
        let mut rec = Vec::new();
        text(&mut rec, "Rail Co");
        rec.extend_from_slice(&250_000i64.to_be_bytes());
        rec.extend_from_slice(&0i64.to_be_bytes());
        rec.extend_from_slice(&300_000i64.to_be_bytes());
        rec.extend_from_slice(&7u16.to_be_bytes());
        let d = stream(300, &[(b"PLYR", 3, vec![head, rec])]);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Companies/0/money").unwrap(), 250_000);
        assert_eq!(doc.view.pointer("/Companies/0/max_loan").unwrap(), 300_000);
        doc.set("/Companies/0/current_loan", serde_json::json!(100_000)).unwrap();
        doc.set("/Companies/0/max_loan", serde_json::json!(500_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Companies/0/current_loan").unwrap(), 100_000);
        assert_eq!(again.view.pointer("/Companies/0/max_loan").unwrap(), 500_000);
        assert!(doc.set("/Companies/0/money", serde_json::json!(1_000_000_000_000_000i64)).is_err());
    }

    #[test]
    fn packed_files_come_back_the_same_way() {
        let d = stream(196, &[(b"PLYR", 1, vec![old_company("Acme", 5, 0)])]);
        let mut z = b"OTTZ".to_vec();
        z.extend_from_slice(&d[4..8]);
        let mut e = ZlibEncoder::new(z, Compression::fast());
        e.write_all(&d[8..]).unwrap();
        let file = e.finish().unwrap();
        let (p, inner) = open(&file).unwrap();
        assert_eq!(inner, d);
        assert_eq!(close(&p, &inner).unwrap(), file);
        let mut doc = parse(&inner).unwrap();
        doc.set("/Companies/0/money", serde_json::json!(77)).unwrap();
        let out = close(&p, &doc.to_bytes().unwrap()).unwrap();
        assert!(out.starts_with(b"OTTZ"));
        let (_, again) = open(&out).unwrap();
        assert_eq!(parse(&again).unwrap().view.pointer("/Companies/0/money").unwrap(), 77);
    }

    #[test]
    fn other_data_is_not_taken() {
        assert!(!looks(b"OTTN\x00\x04\x01\x00ZZZZ"));
        assert!(open(&[0u8; 40]).is_none());
    }
}

