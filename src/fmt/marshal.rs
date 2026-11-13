//! Ruby Marshal 4.8 — RPG Maker XP (`.rxdata`), VX (`.rvdata`) and VX Ace (`.rvdata2`)
//! saves, which are several Marshal dumps written one after another.
//!
//! Like the Ren'Py reader this patches in place: each editable scalar keeps the byte
//! range of its token, and saving rewrites only those tokens. Ruby's object table
//! (used by `@` back-references) is never disturbed because a token is only ever
//! replaced by a token of the same kind (a Bignum stays a Bignum, etc.).

use super::essentials;
use crate::tree::{child, int_in, num, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Bool,
    Fixnum,
    Bignum,
    Float,
    Str,
    /// A nil offered as on/off (it can become true or false, or go back to nil).
    NilBool,
    /// A symbol. Changing one renumbers the symbol links after it, see `relink`.
    Sym,
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
            b'0' => match ptr {
                Some(p) if essentials::nil_editable(p) => self.leaf(ptr, start, Kind::NilBool, Value::Null),
                _ => Value::Null,
            },
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
                let s = Value::String(format!(":{}", self.symbol()?));
                match ptr {
                    Some(p) if essentials::sym_editable(p) => self.leaf(ptr, start, Kind::Sym, s),
                    _ => s,
                }
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
    ess: Option<Arc<essentials::Info>>,
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
        Kind::NilBool => match v {
            Value::Null => vec![b'0'],
            _ => vec![if v.as_bool().ok_or("must be on/off")? { b'T' } else { b'F' }],
        },
        Kind::Sym => {
            let s = v.as_str().and_then(|s| s.strip_prefix(':')).filter(|s| !s.is_empty()).ok_or("must be a :symbol")?;
            [vec![b':'], w_long(s.len() as i64), s.as_bytes().to_vec()].concat()
        }
    })
}

impl Backend for MarshalBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut scalars = HashMap::new();
        let mut syms = HashMap::new();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            match l.kind {
                Kind::Sym => {
                    let name = v.as_str().and_then(|s| s.strip_prefix(':')).ok_or("must be a :symbol")?;
                    syms.insert(l.start, name.to_string());
                }
                k => {
                    scalars.insert(l.start, (l.end, encode(k, v)?));
                }
            }
        }
        if syms.is_empty() {
            let mut p = Patches::default();
            for (start, (end, bytes)) in scalars {
                p.add(start, end, bytes);
            }
            return p.apply(&self.raw);
        }
        relink(&self.raw, &scalars, &syms)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode(self.leaves[leaf].kind, v)?;
        match &self.ess {
            Some(e) => e.check(leaf, v),
            None => Ok(()),
        }
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        self.ess.as_ref().map_or_else(Vec::new, |e| e.derived(leaf, view))
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        self.ess.as_ref()?.choices(leaf)
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Copies a Marshal stream token by token, replacing the scalars in `scalars` (start →
/// (end, new bytes)) and renaming the symbols in `syms` (start → new name). Symbols are
/// written once and then referred to by number, so a changed symbol shifts the numbers of
/// the symbols after it: every symbol is written out again, as a definition the first time
/// its name appears and as a link after that. Objects are untouched, so `@` links stay right.
fn relink(d: &[u8], scalars: &HashMap<usize, (usize, Vec<u8>)>, syms: &HashMap<usize, String>) -> Result<Vec<u8>, String> {
    struct W<'a> {
        p: P<'a>,
        out: Vec<u8>,
        scalars: &'a HashMap<usize, (usize, Vec<u8>)>,
        syms: &'a HashMap<usize, String>,
        new_syms: HashMap<String, usize>,
    }
    impl W<'_> {
        fn copy_from(&mut self, a: usize) {
            self.out.extend_from_slice(&self.p.d[a..self.p.pos]);
        }
        fn long(&mut self) -> Result<i64, String> {
            let a = self.p.pos;
            let n = self.p.long()?;
            self.copy_from(a);
            Ok(n)
        }
        fn bytes(&mut self) -> Result<(), String> {
            let a = self.p.pos;
            self.p.bytes()?;
            self.copy_from(a);
            Ok(())
        }
        fn emit_sym(&mut self, name: &str) {
            match self.new_syms.get(name) {
                Some(&i) => {
                    self.out.push(b';');
                    self.out.extend(w_long(i as i64));
                }
                None => {
                    self.new_syms.insert(name.to_string(), self.new_syms.len());
                    self.out.push(b':');
                    self.out.extend(w_long(name.len() as i64));
                    self.out.extend_from_slice(name.as_bytes());
                }
            }
        }
        fn symbol(&mut self) -> Result<(), String> {
            let at = self.p.pos;
            match self.p.d.get(at) {
                Some(b':') | Some(b';') => {
                    let name = self.p.symbol()?;
                    let name = self.syms.get(&at).cloned().unwrap_or(name);
                    self.emit_sym(&name);
                }
                Some(b'I') => {
                    // A symbol with an encoding: always a definition, and Ruby refuses a
                    // link in its place, so its name must still be new at this point.
                    self.p.pos += 1;
                    self.out.push(b'I');
                    let inner = self.p.pos;
                    let name = self.p.symbol()?;
                    if self.p.d[inner] != b':' || self.new_syms.contains_key(&name) {
                        return Err("can't rewrite this save's symbols".into());
                    }
                    self.emit_sym(&name);
                    let n = self.long()?;
                    for _ in 0..n {
                        self.symbol()?;
                        self.value(0)?;
                    }
                }
                _ => return Err("expected a symbol".into()),
            }
            Ok(())
        }
        fn value(&mut self, depth: usize) -> Result<(), String> {
            if depth > 400 {
                return Err("nested too deep".into());
            }
            let start = self.p.pos;
            if let Some((end, bytes)) = self.scalars.get(&start) {
                self.out.extend_from_slice(bytes);
                self.p.pos = *end;
                return Ok(());
            }
            let t = self.p.byte()?;
            if matches!(t, b':' | b';') {
                self.p.pos = start;
                return self.symbol();
            }
            self.out.push(t);
            match t {
                b'0' | b'T' | b'F' => {}
                b'i' | b'@' => {
                    self.long()?;
                }
                b'l' => {
                    let a = self.p.pos;
                    self.p.byte()?;
                    let shorts = self.p.count()?;
                    self.p.take(shorts * 2)?;
                    self.copy_from(a);
                }
                b'f' | b'"' | b'c' | b'm' | b'M' => self.bytes()?,
                b'/' => {
                    self.bytes()?;
                    let a = self.p.pos;
                    self.p.byte()?;
                    self.copy_from(a);
                }
                b'I' => {
                    self.value(depth + 1)?;
                    let n = self.long()?;
                    for _ in 0..n {
                        self.symbol()?;
                        self.value(depth + 1)?;
                    }
                }
                b'[' => {
                    let n = self.long()?;
                    for _ in 0..n {
                        self.value(depth + 1)?;
                    }
                }
                b'{' | b'}' => {
                    let n = self.long()?;
                    for _ in 0..n * 2 {
                        self.value(depth + 1)?;
                    }
                    if t == b'}' {
                        self.value(depth + 1)?;
                    }
                }
                b'o' | b'S' => {
                    self.symbol()?;
                    let n = self.long()?;
                    for _ in 0..n {
                        self.symbol()?;
                        self.value(depth + 1)?;
                    }
                }
                b'u' => {
                    self.symbol()?;
                    self.bytes()?;
                }
                b'U' | b'e' | b'C' | b'd' => {
                    self.symbol()?;
                    self.value(depth + 1)?;
                }
                t => return Err(format!("unknown Marshal type 0x{t:02X} at byte {start}")),
            }
            Ok(())
        }
    }
    let mut w = W { p: P { d, pos: 0, syms: vec![], leaves: vec![], editable: HashMap::new() }, out: Vec::with_capacity(d.len() + 64), scalars, syms, new_syms: HashMap::new() };
    while w.p.pos < d.len() {
        if w.p.take(2)? != [4, 8] {
            return Err("not Ruby Marshal 4.8".into());
        }
        w.out.extend([4, 8]);
        w.p.syms.clear();
        w.new_syms.clear();
        w.value(0)?;
    }
    Ok(w.out)
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
    let view = Value::Object(root);
    let (ess, kind) = if essentials::looks(&view) {
        (Some(Arc::new(essentials::Info::build(&view, &p.editable))), essentials::KIND)
    } else {
        (None, "RPG Maker XP/VX/Ace (Ruby Marshal)")
    };
    let backend = MarshalBackend { raw: d.to_vec(), leaves: p.leaves, ess };
    Ok(TreeDoc::new(view, p.editable, kind, Box::new(backend)))
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

    /// Minimal Essentials v19 save: two party Pokémon sharing a nature symbol, a bag, a PC.
    fn essentials_sample() -> Vec<u8> {
        fn sym(v: &mut Vec<u8>, s: &str) {
            v.push(b':');
            v.extend(w_long(s.len() as i64));
            v.extend(s.as_bytes());
        }
        fn int(v: &mut Vec<u8>, n: i64) {
            v.push(b'i');
            v.extend(w_long(n));
        }
        // Symbol table as written: player(0) Player(1) @money(2) @party(3) Pokemon(4) @level(5)
        // @exp(6) @nature(7) SERIOUS(8) @shiny(9) @iv(10) HP(11) bag(12) …
        let mut v = vec![4, 8, b'{'];
        v.extend(w_long(3));
        sym(&mut v, "player");
        v.push(b'o');
        sym(&mut v, "Player");
        v.extend(w_long(2));
        sym(&mut v, "@money");
        int(&mut v, 500);
        sym(&mut v, "@party");
        v.push(b'[');
        v.extend(w_long(2));
        for (i, lv) in [(0, 5), (1, 10)] {
            v.push(b'o');
            if i == 0 {
                sym(&mut v, "Pokemon");
            } else {
                v.extend([b';', 4 + 5]);
            }
            v.extend(w_long(5));
            if i == 0 {
                sym(&mut v, "@level");
            } else {
                v.extend([b';', 5 + 5]);
            }
            int(&mut v, lv);
            if i == 0 {
                sym(&mut v, "@exp");
            } else {
                v.extend([b';', 6 + 5]);
            }
            int(&mut v, [125, 1100][i]);
            if i == 0 {
                sym(&mut v, "@nature");
                sym(&mut v, "SERIOUS");
            } else {
                v.extend([b';', 7 + 5, b';', 8 + 5]);
            }
            if i == 0 {
                sym(&mut v, "@shiny");
            } else {
                v.extend([b';', 9 + 5]);
            }
            v.push(b'0');
            if i == 0 {
                sym(&mut v, "@iv");
            } else {
                v.extend([b';', 10 + 5]);
            }
            v.push(b'{');
            v.extend(w_long(1));
            if i == 0 {
                sym(&mut v, "HP");
            } else {
                v.extend([b';', 11 + 5]);
            }
            int(&mut v, 20);
        }
        sym(&mut v, "bag");
        v.push(b'o');
        sym(&mut v, "PokemonBag");
        v.extend(w_long(1));
        sym(&mut v, "@pockets");
        v.extend(b"[\x00");
        sym(&mut v, "storage_system");
        v.push(b'o');
        sym(&mut v, "PokemonStorage");
        v.extend(w_long(1));
        sym(&mut v, "@boxes");
        v.extend(b"[\x00");
        v
    }

    #[test]
    fn essentials_nature_change_relinks_symbols() {
        let raw = essentials_sample();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.kind_name, essentials::KIND);
        assert_eq!(doc.to_bytes().unwrap(), raw);
        let a = "/Part 1/player/@party/0";
        let b = "/Part 1/player/@party/1";
        assert_eq!(doc.view.pointer(&format!("{b}/@nature")).unwrap(), ":SERIOUS");
        assert!(doc.choices(&format!("{a}/@nature")).is_some_and(|c| c.len() == 25));
        assert!(doc.set(&format!("{a}/@nature"), Value::from(":SILLY")).is_err());
        assert!(doc.set(&format!("{a}/@iv/HP"), Value::from(32)).is_err(), "IVs stop at 31");
        // The first Pokémon defines :SERIOUS and the second links to it; change the first.
        doc.set(&format!("{a}/@nature"), Value::from(":ADAMANT")).unwrap();
        doc.set(&format!("{b}/@shiny"), Value::Bool(true)).unwrap();
        // 1,100 Exp. at level 10 is only Medium Fast, where level 12 starts at 1,728. The first
        // Pokémon's 125 Exp. at level 5 fits Medium Fast (6 starts at 216) and Fast (6 starts
        // at 172), so level 6 is 216 Exp. under either; level 30 (27,000 or 21,600 Exp.) can't
        // be both.
        assert!(doc.set(&format!("{a}/@level"), Value::from(30)).is_err());
        doc.set(&format!("{a}/@level"), Value::from(6)).unwrap();
        assert_eq!(doc.view.pointer(&format!("{a}/@exp")).unwrap(), 216);
        doc.set(&format!("{b}/@level"), Value::from(12)).unwrap();
        assert_eq!(doc.view.pointer(&format!("{b}/@exp")).unwrap(), 1728);
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(again.view.pointer(&format!("{a}/@nature")).unwrap(), ":ADAMANT");
        assert_eq!(again.view.pointer(&format!("{b}/@nature")).unwrap(), ":SERIOUS");
        assert_eq!(again.view.pointer(&format!("{b}/@shiny")).unwrap(), true);
        assert_eq!(again.view.pointer(&format!("{a}/@shiny")).unwrap(), &Value::Null);
        assert_eq!(again.view.pointer(&format!("{b}/@level")).unwrap(), 12);
        assert_eq!(again.view.pointer(&format!("{b}/@exp")).unwrap(), 1728);
        assert_eq!(again.view.pointer(&format!("{b}/@iv/HP")).unwrap(), 20);
        assert_eq!(again.view.pointer("/Part 1/player/@money").unwrap(), 500);
        assert_eq!(again.view.pointer("/Part 1/storage_system/__class__").unwrap(), "PokemonStorage");
    }
}
