//! Ruby Marshal 4.8 — RPG Maker XP (`.rxdata`), VX (`.rvdata`) and VX Ace (`.rvdata2`)
//! saves, which are several Marshal dumps written one after another.
//!
//! Like the Ren'Py reader this patches in place: each editable scalar keeps the byte
//! range of its token, and saving rewrites only those tokens. Ruby's object table
//! (used by `@` back-references) is never disturbed because a token is only ever
//! replaced by a token of the same kind (a Bignum stays a Bignum, etc.).

use crate::tree::{child, int_in, num, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Bool,
    Fixnum,
    Bignum,
    Float,
    Str,
}

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    kind: Kind,
}

struct P<'a> {
    d: &'a [u8],
    pos: usize,
    syms: Vec<String>,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
}

fn w_long(x: i64) -> Vec<u8> {
    if x == 0 {
        return vec![0];
    }
    if 0 < x && x < 123 {
        return vec![(x + 5) as u8];
    }
    if -124 < x && x < 0 {
        return vec![((x - 5) & 0xff) as u8];
    }
    let mut buf = vec![0u8];
    let mut v = x;
    for i in 1..=8 {
        buf.push((v & 0xff) as u8);
        v >>= 8;
        if v == 0 {
            buf[0] = i as u8;
            break;
        }
        if v == -1 {
            buf[0] = (-(i as i8)) as u8;
            break;
        }
    }
    buf
}

impl P<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.d.get(self.pos).ok_or("unexpected end of data")?;
        self.pos += 1;
        Ok(b)
    }
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.d.len()).ok_or("unexpected end of data")?;
        let s = &self.d[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn long(&mut self) -> Result<i64, String> {
        let c = self.byte()? as i8;
        Ok(match c {
            0 => 0,
            5..=127 => c as i64 - 5,
            -128..=-5 => c as i64 + 5,
            1..=4 => {
                let mut x: i64 = 0;
                for i in 0..c {
                    x |= (self.byte()? as i64) << (8 * i);
                }
                x
            }
            _ => {
                let n = -c;
                let mut x: i64 = -1;
                for i in 0..n {
                    x &= !(0xff << (8 * i));
                    x |= (self.byte()? as i64) << (8 * i);
                }
                x
            }
        })
    }
    fn count(&mut self) -> Result<usize, String> {
        let n = self.long()?;
        if n < 0 || n as usize > self.d.len() {
            return Err("bad length".into());
        }
        Ok(n as usize)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, String> {
        let n = self.count()?;
        Ok(self.take(n)?.to_vec())
    }
    fn symbol(&mut self) -> Result<String, String> {
        match self.byte()? {
            b':' => {
                let s = String::from_utf8_lossy(&self.bytes()?).into_owned();
                self.syms.push(s.clone());
                Ok(s)
            }
            b';' => {
                let i = self.count()?;
                self.syms.get(i).cloned().ok_or_else(|| "bad symbol link".into())
            }
            // Symbols with an encoding ivar: I : name, count, (sym, value)…
            b'I' => {
                let s = self.symbol()?;
                let n = self.count()?;
                for _ in 0..n {
                    self.symbol()?;
                    self.value(None, 0)?;
                }
                Ok(s)
            }
            t => Err(format!("expected a symbol, found 0x{t:02X}")),
        }
    }
    fn leaf(&mut self, ptr: Option<&str>, start: usize, kind: Kind, v: Value) -> Value {
        if let Some(p) = ptr {
            self.editable.insert(p.to_string(), self.leaves.len());
            self.leaves.push(Leaf { start, end: self.pos, kind });
        }
        v
    }
    fn key_text(v: &Value) -> String {
        match v {
            Value::String(s) => s.strip_prefix(':').unwrap_or(s).to_string(),
            Value::Null => "nil".into(),
            other => other.to_string(),
        }
    }
    /// Parse one value. `ptr` = where it goes in the view (None: parse but don't expose).
    fn value(&mut self, ptr: Option<&str>, depth: usize) -> Result<Value, String> {
        if depth > 400 {
            return Err("nested too deep".into());
        }
        let start = self.pos;
        let sub = |key: &str| ptr.map(|p| child(p, key));
        Ok(match self.byte()? {
            b'0' => Value::Null,
            b'T' => self.leaf(ptr, start, Kind::Bool, Value::Bool(true)),
            b'F' => self.leaf(ptr, start, Kind::Bool, Value::Bool(false)),
            b'i' => {
                let n = self.long()?;
                self.leaf(ptr, start, Kind::Fixnum, Value::from(n))
            }
            b'l' => {
                let sign = self.byte()?;
                let shorts = self.count()?;
                let mag = self.take(shorts * 2)?.to_vec();
                let mut x: i128 = 0;
                if mag.len() <= 15 {
                    for (i, b) in mag.iter().enumerate() {
                        x |= (*b as i128) << (8 * i);
                    }
                    if sign == b'-' {
                        x = -x;
                    }
                }
                match i64::try_from(x).ok().filter(|_| mag.len() <= 15) {
                    Some(v) => self.leaf(ptr, start, Kind::Bignum, Value::from(v)),
                    None => Value::String("<very large integer>".into()),
                }
            }
            b'f' => {
                let s = String::from_utf8_lossy(&self.bytes()?).into_owned();
                match s.parse::<f64>() {
                    Ok(f) if f.is_finite() => self.leaf(ptr, start, Kind::Float, num(f)),
                    _ => Value::String(s),
                }
            }
            b'"' => {
                let raw = self.bytes()?;
                match String::from_utf8(raw) {
                    Ok(s) => self.leaf(ptr, start, Kind::Str, Value::String(s)),
                    Err(e) => Value::String(format!("<{} bytes of binary data>", e.as_bytes().len())),
                }
            }
            b':' | b';' => {
                self.pos -= 1;
                Value::String(format!(":{}", self.symbol()?))
            }
            b'I' => {
                // Value with instance variables (usually a String + its encoding).
                let v = self.value(ptr, depth + 1)?;
                let n = self.count()?;
                for _ in 0..n {
                    self.symbol()?;
                    self.value(None, depth + 1)?;
                }
                v
            }
            b'[' => {
                let n = self.count()?;
                let mut out = Vec::with_capacity(n.min(4096));
                for i in 0..n {
                    out.push(self.value(ptr.map(|p| format!("{p}/{i}")).as_deref(), depth + 1)?);
                }
                Value::Array(out)
            }
            t @ (b'{' | b'}') => {
                let n = self.count()?;
                let mut m = Map::new();
                for _ in 0..n {
                    let k = Self::key_text(&self.value(None, depth + 1)?);
                    let key = unique_key(&m, &k);
                    let v = self.value(sub(&key).as_deref(), depth + 1)?;
                    m.insert(key, v);
                }
                if t == b'}' {
                    self.value(None, depth + 1)?; // default value
                }
                Value::Object(m)
            }
            t @ (b'o' | b'S') => {
                let class = self.symbol()?;
                let n = self.count()?;
                let mut m = Map::new();
                m.insert("__class__".into(), Value::String(class));
                for _ in 0..n {
                    let k = self.symbol()?;
                    let key = unique_key(&m, if t == b'o' { &k } else { k.as_str() });
                    let v = self.value(sub(&key).as_deref(), depth + 1)?;
                    m.insert(key, v);
                }
                Value::Object(m)
            }
            b'u' => {
                let class = self.symbol()?;
                let data = self.bytes()?;
                Value::String(format!("<{class}: {} bytes>", data.len()))
            }
            b'U' => {
                let class = self.symbol()?;
                let mut m = Map::new();
                m.insert("__class__".into(), Value::String(class));
                let v = self.value(sub("data").as_deref(), depth + 1)?;
                m.insert("data".into(), v);
                Value::Object(m)
            }
            b'e' | b'C' => {
                self.symbol()?;
                self.value(ptr, depth + 1)?
            }
            b'c' | b'm' | b'M' => Value::String(format!("<class {}>", String::from_utf8_lossy(&self.bytes()?))),
            b'/' => {
                let src = self.bytes()?;
                self.byte()?;
                Value::String(format!("/{}/", String::from_utf8_lossy(&src)))
            }
            b'd' => {
                let class = self.symbol()?;
                self.value(None, depth + 1)?;
                Value::String(format!("<{class} data>"))
            }
            b'@' => {
                let i = self.long()?;
                Value::String(format!("<same object as #{i}>"))
            }
            t => return Err(format!("unknown Marshal type 0x{t:02X} at byte {start}")),
        })
    }
}

#[derive(Clone, Debug)]
struct MarshalBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

fn encode(kind: Kind, v: &Value) -> Result<Vec<u8>, String> {
    Ok(match kind {
        Kind::Bool => vec![if v.as_bool().ok_or("must be on/off")? { b'T' } else { b'F' }],
        Kind::Fixnum => {
            let n = int_in(v, i32::MIN as i128, i32::MAX as i128, "This number")? as i64;
            [vec![b'i'], w_long(n)].concat()
        }
        Kind::Bignum => {
            let n = int_in(v, i64::MIN as i128, i64::MAX as i128, "This number")? as i64;
            let mut mag = n.unsigned_abs().to_le_bytes().to_vec();
            while mag.len() > 2 && mag[mag.len() - 1] == 0 && mag[mag.len() - 2] == 0 {
                mag.truncate(mag.len() - 2);
            }
            if mag.len() % 2 == 1 {
                mag.push(0);
            }
            let mut out = vec![b'l', if n < 0 { b'-' } else { b'+' }];
            out.extend(w_long((mag.len() / 2) as i64));
            out.extend(mag);
            out
        }
        Kind::Float => {
            let f = v.as_f64().ok_or("must be a number")?;
            if !f.is_finite() {
                return Err("must be a finite number".into());
            }
            let s = format!("{f}");
            [vec![b'f'], w_long(s.len() as i64), s.into_bytes()].concat()
        }
        Kind::Str => {
            let s = v.as_str().ok_or("must be text")?;
            [vec![b'"'], w_long(s.len() as i64), s.as_bytes().to_vec()].concat()
        }
    })
}

impl Backend for MarshalBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            p.add(l.start, l.end, encode(l.kind, v)?);
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode(self.leaves[leaf].kind, v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut p = P { d, pos: 0, syms: vec![], leaves: vec![], editable: HashMap::new() };
    let mut parts = Vec::new();
    while p.pos < d.len() {
        if p.take(2)? != [4, 8] {
            return Err("not Ruby Marshal 4.8".into());
        }
        p.syms.clear();
        let ptr = format!("/Part {}", parts.len() + 1);
        parts.push(p.value(Some(&ptr), 0)?);
    }
    if parts.is_empty() {
        return Err("empty".into());
    }
    let mut root = Map::new();
    for (i, v) in parts.into_iter().enumerate() {
        root.insert(format!("Part {}", i + 1), v);
    }
    let backend = MarshalBackend { raw: d.to_vec(), leaves: p.leaves };
    Ok(TreeDoc::new(Value::Object(root), p.editable, "RPG Maker XP/VX/Ace (Ruby Marshal)", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Vec<u8> {
        // Ruby: I"..." with E: true (UTF-8 marker)
        [b"I\"".to_vec(), w_long(x.len() as i64), x.as_bytes().to_vec(), b"\x06:\x06ET".to_vec()].concat()
    }

    /// Hand-built VX Ace style save: header hash, then contents with a Game_Party object.
    fn sample() -> Vec<u8> {
        let mut v = vec![4, 8];
        // { :playtime => 3600 }
        v.extend(b"{\x06:\x0dplaytime");
        v.extend([b'i']);
        v.extend(w_long(3600));
        v.extend([4, 8]);
        // { :party => #<Game_Party @gold=1500, @name="Ash", @steps=bignum, @rate=1.5, @flag=true>, :vars => [nil, 7, -300] }
        v.extend(b"{\x07:\x0aparty");
        v.extend(b"o:\x0fGame_Party\x0a");
        v.extend(b":\x0a@gold");
        v.extend([b'i']);
        v.extend(w_long(1500));
        v.extend(b":\x0a@name");
        v.extend(s("Ash"));
        v.extend(b":\x0b@steps");
        v.extend(encode(Kind::Bignum, &Value::from(5_000_000_000_i64)).unwrap());
        v.extend(b":\x0a@rate");
        v.extend(encode(Kind::Float, &Value::from(1.5)).unwrap());
        v.extend(b":\x0a@flag");
        v.extend(b"T");
        v.extend(b":\x09vars");
        v.extend(b"[\x080");
        v.extend([b'i']);
        v.extend(w_long(7));
        v.extend([b'i']);
        v.extend(w_long(-300));
        v
    }

    #[test]
    fn long_encoding_round_trips() {
        for x in [0, 1, 122, 123, -1, -123, -124, 255, 256, -256, 65535, -65536, 1 << 20, i32::MAX as i64, i32::MIN as i64] {
            let b = w_long(x);
            let mut p = P { d: &b, pos: 0, syms: vec![], leaves: vec![], editable: HashMap::new() };
            assert_eq!(p.long().unwrap(), x, "value {x}");
            assert_eq!(p.pos, b.len());
        }
    }

    #[test]
    fn edit_vx_ace_style_save() {
        let raw = sample();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        assert_eq!(doc.view.pointer("/Part 1/playtime").unwrap(), 3600);
        assert_eq!(doc.view.pointer("/Part 2/party/@gold").unwrap(), 1500);
        assert_eq!(doc.view.pointer("/Part 2/party/@name").unwrap(), "Ash");
        assert_eq!(doc.view.pointer("/Part 2/party/@steps").unwrap(), 5_000_000_000_i64);
        doc.set("/Part 2/party/@gold", Value::from(999_999)).unwrap();
        doc.set("/Part 2/party/@name", Value::from("Misty Waterflower")).unwrap();
        doc.set("/Part 2/party/@steps", Value::from(42)).unwrap();
        doc.set("/Part 2/party/@rate", Value::from(2.25)).unwrap();
        doc.set("/Part 2/party/@flag", Value::Bool(false)).unwrap();
        doc.set("/Part 2/vars/2", Value::from(1)).unwrap();
        assert!(doc.set("/Part 2/vars/1", Value::from(1_i64 << 40)).is_err(), "fixnum range enforced");
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(again.view.pointer("/Part 2/party/@gold").unwrap(), 999_999);
        assert_eq!(again.view.pointer("/Part 2/party/@name").unwrap(), "Misty Waterflower");
        assert_eq!(again.view.pointer("/Part 2/party/@steps").unwrap(), 42);
        assert_eq!(again.view.pointer("/Part 2/party/@rate").unwrap().as_f64(), Some(2.25));
        assert_eq!(again.view.pointer("/Part 2/party/@flag").unwrap(), false);
        assert_eq!(again.view.pointer("/Part 2/vars/2").unwrap(), 1);
    }
}
