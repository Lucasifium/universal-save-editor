//! Black Myth: Wukong saves (`ArchiveSaveFile.<n>.sav`): a protobuf whose field 1 is a plain
//! header holding a checksum and whose field 2 is the XOR-encrypted (8-byte repeating key)
//! protobuf body. The checksum is the lower-case hex MD5 of the encrypted body followed by a
//! fixed salt, checked when the game loads the save. Only the numbers the screen shows are
//! rewritten; every other field is carried over byte for byte.
//!
//! Key, salt and field numbers follow the MIT-licensed WukongMM project's format notes.

use crate::tree::{Backend, TreeDoc};
use md5::{Digest, Md5};
use serde_json::{json, Value};
use std::collections::HashMap;

pub const KIND: &str = "Black Myth: Wukong save";
const KEY: [u8; 8] = [0x7B, 0x5C, 0xDA, 0x91, 0x3E, 0xFC, 0xDA, 0x37];
const SALT: &[u8] = b"lhx2tkh6lj1wj8jmrgs3k1xb2brusehx";

fn xor(d: &mut [u8]) {
    for (i, b) in d.iter_mut().enumerate() {
        *b ^= KEY[i & 7];
    }
}

fn md5hex(enc: &[u8]) -> String {
    let mut h = Md5::new();
    h.update(enc);
    h.update(SALT);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn read_varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let (mut x, mut s) = (0u64, 0);
    loop {
        let c = *b.get(*i)?;
        *i += 1;
        if s >= 64 {
            return None;
        }
        x |= ((c & 0x7F) as u64) << s;
        s += 7;
        if c < 0x80 {
            return Some(x);
        }
    }
}

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let c = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(c);
            return;
        }
        out.push(c | 0x80);
    }
}

/// One field: number, wire type, and the byte range of its payload (for length-delimited the
/// bytes after the length) and of the whole field.
#[derive(Clone, Debug)]
struct F {
    num: u32,
    wire: u8,
    start: usize,
    pay: (usize, usize),
    end: usize,
}

fn fields(b: &[u8]) -> Option<Vec<F>> {
    let mut i = 0;
    let mut out = vec![];
    while i < b.len() {
        let start = i;
        let k = read_varint(b, &mut i)?;
        let (num, wire) = ((k >> 3) as u32, (k & 7) as u8);
        let pay = match wire {
            0 => {
                let s = i;
                read_varint(b, &mut i)?;
                (s, i)
            }
            1 => {
                i += 8;
                (i - 8, i)
            }
            5 => {
                i += 4;
                (i - 4, i)
            }
            2 => {
                let l = read_varint(b, &mut i)? as usize;
                i += l;
                (i - l, i)
            }
            _ => return None,
        };
        if i > b.len() {
            return None;
        }
        out.push(F { num, wire, start, pay, end: i });
    }
    Some(out)
}

#[derive(Clone, Copy, Debug)]
struct Step {
    field: u32,
    nth: usize,
}

fn nth_field(b: &[u8], s: Step) -> Option<F> {
    fields(b)?.into_iter().filter(|f| f.num == s.field).nth(s.nth)
}

/// Replace (or add) a varint / float leaf at `path` + `last` inside `b`, resizing the
/// length prefixes of the messages on the way.
fn set(b: &[u8], path: &[Step], last: u32, wire: u8, payload: &[u8]) -> Option<Vec<u8>> {
    if let Some((first, rest)) = path.split_first() {
        let f = nth_field(b, *first)?;
        if f.wire != 2 {
            return None;
        }
        let inner = set(&b[f.pay.0..f.pay.1], rest, last, wire, payload)?;
        let mut out = b[..f.start].to_vec();
        put_varint(&mut out, ((f.num as u64) << 3) | 2);
        put_varint(&mut out, inner.len() as u64);
        out.extend(&inner);
        out.extend(&b[f.end..]);
        return Some(out);
    }
    let found = fields(b)?.into_iter().find(|f| f.num == last && f.wire == wire);
    let mut out;
    match found {
        Some(f) => {
            out = b[..f.pay.0].to_vec();
            out.extend(payload);
            out.extend(&b[f.pay.1..]);
        }
        None => {
            out = b.to_vec();
            put_varint(&mut out, ((last as u64) << 3) | wire as u64);
            out.extend(payload);
        }
    }
    Some(out)
}

fn get_varint(b: &[u8], path: &[Step], last: u32) -> Option<u64> {
    let mut cur = b;
    for s in path {
        let f = nth_field(cur, *s)?;
        cur = &cur[f.pay.0..f.pay.1];
    }
    match fields(cur)?.into_iter().find(|f| f.num == last && f.wire == 0) {
        Some(f) => {
            let mut i = f.pay.0;
            read_varint(cur, &mut i)
        }
        None => Some(0),
    }
}

fn st(f: u32, n: usize) -> Step {
    Step { field: f, nth: n }
}

#[derive(Clone, Debug)]
struct Leaf {
    path: Vec<Step>,
    last: u32,
    float: bool,
}

#[derive(Clone, Debug)]
struct WukongBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

/// Split a file into (header message bytes, encrypted body).
fn split(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let f = fields(d)?;
    let h = f.iter().find(|x| x.num == 1 && x.wire == 2)?;
    let b = f.iter().find(|x| x.num == 2 && x.wire == 2)?;
    Some((d[h.pay.0..h.pay.1].to_vec(), d[b.pay.0..b.pay.1].to_vec()))
}

fn join(header: &[u8], body: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    put_varint(&mut out, (1 << 3) | 2);
    put_varint(&mut out, header.len() as u64);
    out.extend(header);
    put_varint(&mut out, (2 << 3) | 2);
    put_varint(&mut out, body.len() as u64);
    out.extend(body);
    out
}

impl Backend for WukongBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let (header, enc) = split(&self.raw).ok_or("not a Black Myth: Wukong save")?;
        let mut body = enc;
        xor(&mut body);
        for (id, v) in changes {
            let l = &self.leaves[*id];
            let (wire, payload) = if l.float {
                (5u8, (v.as_f64().ok_or("a number is needed")? as f32).to_le_bytes().to_vec())
            } else {
                let n = v.as_u64().or_else(|| v.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64)).ok_or("a whole number of 0 or more is needed")?;
                let mut p = vec![];
                put_varint(&mut p, n);
                (0u8, p)
            };
            body = set(&body, &l.path, l.last, wire, &payload).ok_or("the save's layout is not what this editor expects")?;
        }
        xor(&mut body);
        let sum = md5hex(&body);
        // The checksum is field 1 of the header (32 characters, same length either way).
        let hf = fields(&header).ok_or("bad header")?;
        let c = hf.iter().find(|f| f.num == 1 && f.wire == 2).ok_or("the header has no checksum")?;
        let mut h = header.clone();
        h[c.pay.0..c.pay.1].copy_from_slice(sum.as_bytes());
        Ok(join(&h, &body))
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.as_f64().is_some_and(|f| f < 0.0) {
            return Err("This value can't be negative".into());
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Does the file carry a header whose checksum matches its body?
pub fn looks(d: &[u8]) -> bool {
    let Some((h, enc)) = split(d) else { return false };
    let Some(hf) = fields(&h) else { return false };
    let Some(c) = hf.iter().find(|f| f.num == 1 && f.wire == 2) else { return false };
    &h[c.pay.0..c.pay.1] == md5hex(&enc).as_bytes()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (_, enc) = split(d).ok_or("not a Black Myth: Wukong save")?;
    let mut body = enc;
    xor(&mut body);
    let role = [st(1, 0), st(1, 0)];
    let at = |extra: &[Step]| -> Vec<Step> { role.iter().chain(extra).copied().collect() };
    // Role base (1), bag (2), actor (3) inside the role data.
    let base = at(&[st(1, 0)]);
    let level = get_varint(&body, &base, 4).filter(|_| nth_field(&body, st(1, 0)).is_some()).ok_or("this file holds no character")?;
    let bag = at(&[st(2, 0)]);
    // The money entry of type 1 (the game's only currency).
    let bagf = {
        let mut cur: &[u8] = &body;
        for s in &bag {
            let f = nth_field(cur, *s).ok_or("no inventory in this file")?;
            cur = &cur[f.pay.0..f.pay.1];
        }
        cur.to_vec()
    };
    let money_idx = fields(&bagf)
        .ok_or("bad inventory")?
        .into_iter()
        .filter(|f| f.num == 1)
        .position(|f| {
            let mut i = f.pay.0 + 0;
            let inner = &bagf[f.pay.0..f.pay.1];
            let _ = &mut i;
            fields(inner).is_some_and(|ff| ff.iter().any(|x| x.num == 1 && x.wire == 0 && inner[x.pay.0] == 1))
        })
        .ok_or("no Will entry")?;
    let mut will_path = bag.clone();
    will_path.push(st(1, money_idx));
    let prog = at(&[st(3, 0), st(2, 0)]);
    let mut leaves = vec![];
    let mut editable = HashMap::new();
    let mut view = serde_json::Map::new();
    let mut add = |name: &str, path: Vec<Step>, last: u32, val: Value, float: bool| {
        editable.insert(format!("/{name}"), leaves.len());
        leaves.push(Leaf { path, last, float });
        view.insert(name.to_string(), val);
    };
    add("Level", base.clone(), 4, json!(level), false);
    add("Will", will_path.clone(), 2, json!(get_varint(&body, &will_path, 2).unwrap_or(0)), false);
    add("Experience (this level)", prog.clone(), 1, json!(get_varint(&body, &prog, 1).unwrap_or(0)), false);
    add("Sparks", prog.clone(), 2, json!(get_varint(&body, &prog, 2).unwrap_or(0)), false);
    add("Experience (total)", prog.clone(), 7, json!(get_varint(&body, &prog, 7).unwrap_or(0)), false);
    let pt = at(&[]);
    let _ = pt;
    // Total play time (seconds): a float in 1.2.1.1 (field 2).
    let tp = [st(1, 0), st(2, 0), st(1, 0), st(1, 0)];
    let mut cur: &[u8] = &body;
    let mut ok = true;
    for s in &tp {
        match nth_field(cur, *s) {
            Some(f) => cur = &cur[f.pay.0..f.pay.1],
            None => {
                ok = false;
                break;
            }
        }
    }
    if ok {
        if let Some(f) = fields(cur).and_then(|ff| ff.into_iter().find(|f| f.num == 2 && f.wire == 5)) {
            let v = f32::from_le_bytes(cur[f.pay.0..f.pay.1].try_into().unwrap());
            add("Play time (seconds)", tp.to_vec(), 2, json!(v as f64), true);
        }
    }
    Ok(TreeDoc::new(Value::Object(view), editable, KIND, Box::new(WukongBackend { raw: d.to_vec(), leaves })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        // money entry, base, progress, built by hand
        let mut money = vec![];
        put_varint(&mut money, 1 << 3);
        put_varint(&mut money, 1);
        put_varint(&mut money, 2 << 3);
        put_varint(&mut money, 300);
        let mut bag = vec![];
        put_varint(&mut bag, (1 << 3) | 2);
        put_varint(&mut bag, money.len() as u64);
        bag.extend(&money);
        let mut base = vec![];
        put_varint(&mut base, 4 << 3);
        put_varint(&mut base, 92);
        let mut prog = vec![];
        put_varint(&mut prog, 1 << 3);
        put_varint(&mut prog, 4695);
        let mut actor = vec![];
        put_varint(&mut actor, (2 << 3) | 2);
        put_varint(&mut actor, prog.len() as u64);
        actor.extend(&prog);
        let mut cs = vec![];
        for (n, m) in [(1u64, &base), (2, &bag), (3, &actor)] {
            put_varint(&mut cs, (n << 3) | 2);
            put_varint(&mut cs, m.len() as u64);
            cs.extend(m.iter());
        }
        let mut role = vec![];
        put_varint(&mut role, (1 << 3) | 2);
        put_varint(&mut role, cs.len() as u64);
        role.extend(&cs);
        let mut data = vec![];
        put_varint(&mut data, (1 << 3) | 2);
        put_varint(&mut data, role.len() as u64);
        data.extend(&role);
        xor(&mut data);
        let mut header = vec![];
        put_varint(&mut header, (1 << 3) | 2);
        put_varint(&mut header, 32);
        header.extend(md5hex(&data).as_bytes());
        join(&header, &data)
    }

    #[test]
    fn edit_resizes_and_resigns() {
        let raw = sample();
        assert!(looks(&raw));
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        assert_eq!(doc.view.pointer("/Level").unwrap(), 92);
        assert_eq!(doc.view.pointer("/Will").unwrap(), 300);
        doc.set("/Will", Value::from(99_999_999u64)).unwrap();
        doc.set("/Sparks", Value::from(5)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out), "the checksum is written again");
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Will").unwrap(), 99_999_999u64);
        assert_eq!(again.view.pointer("/Sparks").unwrap(), 5);
        assert_eq!(again.view.pointer("/Level").unwrap(), 92);
    }

    /// A real save, if WUKONG_SAVE points at one: no-change round trip and a Will edit.
    #[test]
    #[ignore]
    fn real_save() {
        let raw = std::fs::read(std::env::var("WUKONG_SAVE").expect("WUKONG_SAVE")).unwrap();
        assert!(looks(&raw));
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        println!("{}", doc.view);
        doc.set("/Will", Value::from(9_999_999u64)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(parse(&out).unwrap().view.pointer("/Will").unwrap(), 9_999_999u64);
    }
}
