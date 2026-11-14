//! PAYDAY 2 `saveNNN.sav`: the whole file is XORed with a fixed key stream, and inside it is a
//! version number, three blocks (header, game data, footer) that each end in an MD5 of their
//! bytes, and a closing hash. The game data is a serialized dictionary of typed values; some
//! strings are stored "encoded" (every second byte is a fixed padding byte), which the game uses
//! for numbers it does not want to be found by Value search.
//!
//! The tree is shown as JSON: encoded numbers as numbers, dictionaries as objects. On save the
//! file is rebuilt from the bytes it was opened from, so every value keeps the type the game wrote.

use md5::{Digest, Md5};
use serde_json::{Map, Value};

const XOR_KEY: [u8; 34] = [
    0x74, 0x3E, 0x3F, 0xA4, 0x32, 0x43, 0x26, 0x2E, 0x23, 0x36, 0x37, 0x6A, 0x6D, 0x3A, 0x48, 0x47, 0x3D, 0x53, 0x2D, 0x63, 0x41, 0x6B, 0x29, 0x38,
    0x6A, 0x68, 0x5F, 0x4D, 0x4A, 0x68, 0x3C, 0x6E, 0x66, 0xF6,
];
const HASH_KEY: [u8; 4] = [0x1A, 0x1F, 0x32, 0x2C];
const PADDING: [u8; 12] = [0xDF, 0xC1, 0xA3, 0x85, 0x67, 0x49, 0x2B, 0x0D, 0xED, 0xCF, 0xB1, 0x93];
const VERSION: u32 = 10;

fn transform(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    data.iter().enumerate().map(|(i, b)| b ^ (XOR_KEY[((n + i) * 7) % 34].wrapping_mul((n - i) as u8))).collect()
}

fn md5(d: &[u8]) -> [u8; 16] {
    Md5::digest(d).into()
}

fn final_hash(body: &[u8]) -> [u8; 16] {
    let t: Vec<u8> = body.iter().enumerate().map(|(i, &b)| if b.wrapping_add(HASH_KEY[i % 4]) % 2 != 0 { (i % 7) as u8 } else { b }).collect();
    md5(&t)
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Str { s: String, enc: bool },
    Float(f32),
    Null,
    Byte(u8),
    Short(i16),
    Bool(bool),
    Dict(Vec<(Node, Node)>),
}

struct Rd<'a> {
    d: &'a [u8],
    p: usize,
}

impl Rd<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.d.get(self.p..self.p.checked_add(n)?)?;
        self.p += n;
        Some(s)
    }
    fn node(&mut self, depth: usize) -> Option<Node> {
        if depth > 64 {
            return None;
        }
        Some(match self.take(1)?[0] {
            1 => {
                let end = self.p + self.d.get(self.p..)?.iter().position(|&b| b == 0)?;
                let raw = &self.d[self.p..end];
                self.p = end + 1;
                let enc = !raw.is_empty() && raw.len() % 2 == 0 && raw.len() <= 24 && (1..raw.len()).step_by(2).all(|i| raw[i] == PADDING[(i - 1) / 2]);
                if enc {
                    Node::Str { s: (0..raw.len()).step_by(2).map(|i| char::from(0xFEu8.wrapping_sub(raw[i]))).collect(), enc: true }
                } else {
                    Node::Str { s: raw.iter().map(|&b| char::from(b)).collect(), enc: false }
                }
            }
            2 => Node::Float(f32::from_le_bytes(self.take(4)?.try_into().ok()?)),
            3 => Node::Null,
            4 => Node::Byte(self.take(1)?[0]),
            5 => Node::Short(i16::from_le_bytes(self.take(2)?.try_into().ok()?)),
            6 => Node::Bool(self.take(1)?[0] != 0),
            7 => {
                let n = u32::from_le_bytes(self.take(4)?.try_into().ok()?) as usize;
                let mut v = Vec::new();
                for _ in 0..n {
                    let k = self.node(depth + 1)?;
                    let x = self.node(depth + 1)?;
                    v.push((k, x));
                }
                Node::Dict(v)
            }
            _ => return None,
        })
    }
}

fn put(n: &Node, out: &mut Vec<u8>) {
    match n {
        Node::Str { s, enc } => {
            out.push(1);
            if *enc {
                for (i, c) in s.chars().enumerate() {
                    out.push(0xFEu8.wrapping_sub(c as u32 as u8));
                    out.push(PADDING[i % 12]);
                }
            } else {
                out.extend(s.chars().map(|c| c as u32 as u8));
            }
            out.push(0);
        }
        Node::Float(f) => {
            out.push(2);
            out.extend(f.to_le_bytes());
        }
        Node::Null => out.push(3),
        Node::Byte(b) => out.extend([4, *b]),
        Node::Short(s) => {
            out.push(5);
            out.extend(s.to_le_bytes());
        }
        Node::Bool(b) => out.extend([6, *b as u8]),
        Node::Dict(v) => {
            out.push(7);
            out.extend((v.len() as u32).to_le_bytes());
            for (k, x) in v {
                put(k, out);
                put(x, out);
            }
        }
    }
}

fn key_text(n: &Node) -> String {
    match n {
        Node::Str { s, .. } => s.clone(),
        Node::Float(f) => f.to_string(),
        Node::Byte(b) => b.to_string(),
        Node::Short(s) => s.to_string(),
        Node::Bool(b) => b.to_string(),
        _ => "null".into(),
    }
}

fn number(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

fn to_json(n: &Node) -> Option<Value> {
    Some(match n {
        Node::Str { s, enc: true } => match s.parse::<f64>() {
            Ok(f) if f.is_finite() && !s.is_empty() && s.trim() == s => number(&format!("{}", f)),
            _ => Value::String(s.clone()),
        },
        Node::Str { s, .. } => Value::String(s.clone()),
        Node::Float(f) => number(&f.to_string()),
        Node::Null => Value::Null,
        Node::Byte(b) => Value::from(*b),
        Node::Short(s) => Value::from(*s),
        Node::Bool(b) => Value::Bool(*b),
        Node::Dict(v) => {
            let mut m = Map::new();
            for (k, x) in v {
                if m.insert(key_text(k), to_json(x)?).is_some() {
                    return None;
                }
            }
            Value::Object(m)
        }
    })
}

fn enc_number(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

fn infer(v: &Value) -> Node {
    match v {
        Value::Null => Node::Null,
        Value::Bool(b) => Node::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) if i16::try_from(i).is_ok() => Node::Short(i as i16),
            _ => Node::Float(n.as_f64().unwrap_or(0.0) as f32),
        },
        Value::String(s) => Node::Str { s: s.clone(), enc: false },
        Value::Object(m) => Node::Dict(m.iter().map(|(k, x)| (Node::Str { s: k.clone(), enc: false }, infer(x))).collect()),
        Value::Array(a) => Node::Dict(a.iter().enumerate().map(|(i, x)| (Node::Str { s: i.to_string(), enc: false }, infer(x))).collect()),
    }
}

fn rebuild(skel: &Node, v: &Value) -> Node {
    match (skel, v) {
        (Node::Dict(old), Value::Object(m)) => Node::Dict(
            m.iter()
                .map(|(k, x)| match old.iter().find(|(ok, _)| key_text(ok) == *k) {
                    Some((ok, ov)) => (ok.clone(), rebuild(ov, x)),
                    None => (Node::Str { s: k.clone(), enc: false }, infer(x)),
                })
                .collect(),
        ),
        // An untouched number keeps the text the game wrote (it may be rounded, such as "1.16413e+008").
        (Node::Str { s: old, enc: true }, Value::Number(n)) => {
            let f = n.as_f64().unwrap_or(0.0);
            let same = old.parse::<f64>().is_ok_and(|o| format!("{o}") == format!("{f}"));
            Node::Str { s: if same { old.clone() } else { enc_number(f) }, enc: true }
        }
        (Node::Str { enc, .. }, Value::String(s)) => Node::Str { s: s.clone(), enc: *enc },
        (Node::Float(_), Value::Number(n)) => Node::Float(n.as_f64().unwrap_or(0.0) as f32),
        (Node::Byte(_), Value::Number(n)) => Node::Byte(n.as_f64().unwrap_or(0.0).clamp(0.0, 255.0) as u8),
        (Node::Short(_), Value::Number(n)) => Node::Short(n.as_f64().unwrap_or(0.0).clamp(-32768.0, 32767.0) as i16),
        (Node::Bool(_), Value::Bool(b)) => Node::Bool(*b),
        (Node::Null, Value::Null) => Node::Null,
        _ => infer(v),
    }
}

/// The three blocks' data and the offset after them, if `p` is a PAYDAY 2 file in the clear.
fn blocks(p: &[u8]) -> Option<[Vec<u8>; 3]> {
    if p.len() < 4 + 3 * 20 + 32 || u32::from_le_bytes(p[..4].try_into().ok()?) != VERSION {
        return None;
    }
    let mut o = 4;
    let mut out: [Vec<u8>; 3] = Default::default();
    for b in out.iter_mut() {
        let size = u32::from_le_bytes(p.get(o..o + 4)?.try_into().ok()?) as usize;
        if size < 20 || u32::from_le_bytes(p.get(o + 4..o + 8)?.try_into().ok()?) != VERSION {
            return None;
        }
        let data = p.get(o + 8..o + 4 + size - 16)?;
        if md5(data) != p.get(o + 4 + size - 16..o + 4 + size)? {
            return None;
        }
        *b = data.to_vec();
        o += 4 + size;
    }
    (p.len() == o + 32).then_some(out)
}

/// The decoded file (kept to rebuild the save) and the JSON text of its game data.
pub fn open(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if data.len() < 128 {
        return None;
    }
    let plain = transform(data);
    let b = blocks(&plain)?;
    let mut rd = Rd { d: &b[1], p: 0 };
    let root = rd.node(0)?;
    if rd.p != b[1].len() || !matches!(root, Node::Dict(_)) {
        return None;
    }
    let json = serde_json::to_vec_pretty(&to_json(&root)?).ok()?;
    Some((plain, json))
}

pub fn pack(original: &[u8], json: &[u8]) -> Result<Vec<u8>, String> {
    let b = blocks(original).ok_or("The PAYDAY 2 save this was opened from is not valid")?;
    let skel = Rd { d: &b[1], p: 0 }.node(0).ok_or("The PAYDAY 2 save data could not be read")?;
    let v: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let mut game = Vec::new();
    put(&rebuild(&skel, &v), &mut game);
    let mut out = VERSION.to_le_bytes().to_vec();
    for d in [&b[0], &game, &b[2]] {
        out.extend(((d.len() + 20) as u32).to_le_bytes());
        out.extend(VERSION.to_le_bytes());
        out.extend(d.iter());
        out.extend(md5(d));
    }
    out.extend([0u8; 16]);
    let h = final_hash(&out);
    out.extend(h);
    Ok(transform(&out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let game = Node::Dict(vec![
            (Node::Str { s: "MoneyManager".into(), enc: false }, Node::Dict(vec![(Node::Str { s: "total".into(), enc: false }, Node::Str { s: "1500".into(), enc: true })])),
            (Node::Str { s: "version".into(), enc: false }, Node::Byte(5)),
            (Node::Byte(3), Node::Float(2.5)),
        ]);
        let mut g = Vec::new();
        put(&game, &mut g);
        let mut out = VERSION.to_le_bytes().to_vec();
        for d in [&[1u8, 2, 3][..], &g, &[][..]] {
            out.extend(((d.len() + 20) as u32).to_le_bytes());
            out.extend(VERSION.to_le_bytes());
            out.extend(d);
            out.extend(md5(d));
        }
        out.extend([0u8; 16]);
        let h = final_hash(&out);
        out.extend(h);
        transform(&out)
    }

    #[test]
    fn round_trip_and_edit() {
        let file = sample();
        let (plain, json) = open(&file).unwrap();
        assert_eq!(pack(&plain, &json).unwrap(), file);
        let mut v: Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(v["MoneyManager"]["total"], 1500);
        v["MoneyManager"]["total"] = Value::from(99999999);
        let edited = pack(&plain, &serde_json::to_vec(&v).unwrap()).unwrap();
        let (_, json2) = open(&edited).unwrap();
        let v2: Value = serde_json::from_slice(&json2).unwrap();
        assert_eq!(v2["MoneyManager"]["total"], 99999999);
        assert_eq!(v2["3"], 2.5);
    }
}
