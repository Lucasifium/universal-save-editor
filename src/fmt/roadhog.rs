//! The text save format of the RoadHog engine (Shadow Warrior 2, Hard Reset Redux): a script-like
//! tree, `return ( { Key = value, … } );`, with `[ … ]` lists, `@"names"`, and constructors such as
//! `::Vector( x, y, z )`. In the game's save folder each `.bin` is a zip holding the `meta` entry and
//! this text as `bin`. Numbers, true/false and names are edited in place; the rest of the text is
//! written back exactly as it was.

use crate::tree::{child, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "RoadHog engine save";

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    /// Written with a decimal point in the file (`1.0`): keep it that way.
    float: bool,
    name: bool,
}

#[derive(Clone, Debug)]
struct Back {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for Back {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            let text = match v {
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => {
                    let mut s = n.to_string();
                    if l.float && !s.contains(['.', 'e', 'E']) {
                        s.push_str(".0");
                    }
                    s
                }
                Value::String(s) if l.name => s.clone(),
                _ => return Err("this value can't be written".into()),
            };
            p.add(l.start, l.end, text.into_bytes());
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match v {
            Value::String(s) if self.leaves[leaf].name && !s.contains(['"', '\n', '\r']) => Ok(()),
            Value::String(_) => Err("a name can't contain quotes or line breaks".into()),
            _ => Ok(()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"// This is RoadHog auto-generated savegame file")
}

struct P<'a> {
    d: &'a [u8],
    pos: usize,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
    depth: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        loop {
            while self.pos < self.d.len() && self.d[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.d[self.pos..].starts_with(b"//") {
                while self.pos < self.d.len() && self.d[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.d.get(self.pos).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.pos))
        }
    }

    fn string(&mut self) -> Result<(usize, usize), String> {
        // at the opening quote
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.d.len() && self.d[self.pos] != b'"' {
            if self.d[self.pos] == b'\\' {
                self.pos += 1;
            }
            self.pos += 1;
        }
        if self.pos >= self.d.len() {
            return Err("unterminated text".into());
        }
        let end = self.pos;
        self.pos += 1;
        Ok((start, end))
    }

    /// A key: a bare name, `@"name"`, `"name"` or `[ number or @"name" ]`.
    fn key(&mut self) -> Result<String, String> {
        self.ws();
        match self.peek() {
            Some(b'[') => {
                self.pos += 1;
                self.ws();
                let k = if matches!(self.peek(), Some(b'@' | b'"')) {
                    if self.peek() == Some(b'@') {
                        self.pos += 1;
                    }
                    let (s, e) = self.string()?;
                    String::from_utf8_lossy(&self.d[s..e]).into_owned()
                } else {
                    let s = self.pos;
                    while self.peek().is_some_and(|c| c != b']') {
                        self.pos += 1;
                    }
                    String::from_utf8_lossy(&self.d[s..self.pos]).trim().to_string()
                };
                self.expect(b']')?;
                Ok(k)
            }
            Some(b'@' | b'"') => {
                if self.peek() == Some(b'@') {
                    self.pos += 1;
                }
                let (s, e) = self.string()?;
                Ok(String::from_utf8_lossy(&self.d[s..e]).into_owned())
            }
            _ => {
                let s = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.' || c == b':') {
                    self.pos += 1;
                }
                if s == self.pos {
                    return Err(format!("expected a key at byte {}", self.pos));
                }
                Ok(String::from_utf8_lossy(&self.d[s..self.pos]).into_owned())
            }
        }
    }

    fn value(&mut self, ptr: &str) -> Result<Value, String> {
        self.ws();
        self.depth += 1;
        if self.depth > 200 {
            return Err("nested too deeply".into());
        }
        let v = match self.peek().ok_or("unexpected end")? {
            b'{' => {
                self.pos += 1;
                let mut m = Map::new();
                loop {
                    self.ws();
                    if self.peek() == Some(b'}') {
                        self.pos += 1;
                        break;
                    }
                    let k = self.key()?;
                    self.expect(b'=')?;
                    let key = crate::tree::unique_key(&m, &k);
                    let v = self.value(&child(ptr, &key))?;
                    m.insert(key, v);
                    self.ws();
                    if self.peek() == Some(b',') {
                        self.pos += 1;
                    }
                }
                Value::Object(m)
            }
            b'[' => {
                self.pos += 1;
                let mut a = Vec::new();
                loop {
                    self.ws();
                    if self.peek() == Some(b']') {
                        self.pos += 1;
                        break;
                    }
                    let v = self.value(&child(ptr, &a.len().to_string()))?;
                    a.push(v);
                    self.ws();
                    if self.peek() == Some(b',') {
                        self.pos += 1;
                    }
                }
                Value::Array(a)
            }
            b'@' | b'"' => {
                let name = self.peek() == Some(b'@');
                if name {
                    self.pos += 1;
                }
                let (s, e) = self.string()?;
                let text = String::from_utf8_lossy(&self.d[s..e]).into_owned();
                if name && !text.contains('\\') {
                    self.editable.insert(ptr.to_string(), self.leaves.len());
                    self.leaves.push(Leaf { start: s, end: e, float: false, name: true });
                }
                Value::String(text)
            }
            b':' => {
                // `::Vector( 1, 2, 3 )`: kept as read-only text.
                let s = self.pos;
                while self.peek().is_some_and(|c| c != b'(') {
                    self.pos += 1;
                }
                let mut depth = 0;
                while let Some(c) = self.peek() {
                    self.pos += 1;
                    if c == b'(' {
                        depth += 1;
                    } else if c == b')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                Value::String(String::from_utf8_lossy(&self.d[s..self.pos]).into_owned())
            }
            _ => {
                let s = self.pos;
                while self.peek().is_some_and(|c| !matches!(c, b',' | b'}' | b']' | b')') && !c.is_ascii_whitespace()) {
                    self.pos += 1;
                }
                let t = std::str::from_utf8(&self.d[s..self.pos]).map_err(|_| "not text")?;
                match t {
                    "true" | "false" => {
                        self.editable.insert(ptr.to_string(), self.leaves.len());
                        self.leaves.push(Leaf { start: s, end: self.pos, float: false, name: false });
                        Value::Bool(t == "true")
                    }
                    "null" => Value::Null,
                    _ => match t.parse::<f64>() {
                        Ok(f) if f.is_finite() => {
                            self.editable.insert(ptr.to_string(), self.leaves.len());
                            self.leaves.push(Leaf { start: s, end: self.pos, float: t.contains('.'), name: false });
                            match t.parse::<i64>() {
                                Ok(i) => Value::from(i),
                                // A whole float (`1451541.0`) is shown as a whole number.
                                Err(_) if f.fract() == 0.0 && f.abs() < 1e15 => Value::from(f as i64),
                                Err(_) => crate::tree::num(f),
                            }
                        }
                        _ => return Err(format!("unknown value '{t}' at byte {s}")),
                    },
                }
            }
        };
        self.depth -= 1;
        Ok(v)
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a RoadHog save".into());
    }
    let mut p = P { d, pos: 0, leaves: vec![], editable: HashMap::new(), depth: 0 };
    p.ws();
    if !p.d[p.pos..].starts_with(b"return") {
        return Err("no return statement".into());
    }
    p.pos += 6;
    p.expect(b'(')?;
    let root = p.value("")?;
    p.expect(b')')?;
    Ok(TreeDoc::new(root, p.editable, KIND, Box::new(Back { raw: d.to_vec(), leaves: p.leaves })))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "// This is RoadHog auto-generated savegame file. Plz don't modify it.\n\nreturn (\n\n\t{\n\t\tPlayerTemplate = \n\t\t{\n\t\t\tStats = \n\t\t\t{\n\t\t\t\tLevel = 3157,\n\t\t\t\tMoney = 1451541.0,\n\t\t\t\tName = @\"Roa\",\n\t\t\t\tPos = ::Vector( 1.5, 2, 3 ),\n\t\t\t\tList = \n\t\t\t\t[\n\t\t\t\t\t@\"a\",\n\t\t\t\t],\n\t\t\t\t[ 0 ]  = 1,\n\t\t\t\tOn = true,\n\t\t\t},\n\t\t},\n\t}\n\n);\n";

    #[test]
    fn reads_and_edits_in_place() {
        let mut doc = parse(SAMPLE.as_bytes()).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        assert_eq!(doc.view.pointer("/PlayerTemplate/Stats/Level").unwrap(), 3157);
        assert_eq!(doc.view.pointer("/PlayerTemplate/Stats/Name").unwrap(), "Roa");
        doc.set("/PlayerTemplate/Stats/Money", Value::from(99)).unwrap();
        doc.set("/PlayerTemplate/Stats/Level", Value::from(5)).unwrap();
        doc.set("/PlayerTemplate/Stats/On", Value::Bool(false)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, SAMPLE.replace("3157", "5").replace("1451541.0", "99.0").replace("On = true", "On = false"));
        assert!(parse(&out.into_bytes()).is_ok());
    }
}
