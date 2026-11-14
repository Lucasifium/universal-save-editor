//! Lua table literals (`return { ["money"] = 50, ... }`), as saved by Balatro (`.jkr`
//! after its deflate layer), Don't Starve, many LÖVE games and other Lua-scripted games.
//! Values are edited in place; everything else (key order, spacing) is kept.

use crate::tree::{child, insert_unique, num, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Str,
    Num,
    Bool,
}

#[derive(Clone, Debug)]
struct Span {
    start: usize,
    end: usize,
    kind: Kind,
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
    spans: Vec<Span>,
    editable: HashMap<String, usize>,
    depth: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        loop {
            while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            // -- comments
            if self.s[self.i..].starts_with(b"--") {
                while self.i < self.s.len() && self.s[self.i] != b'\n' {
                    self.i += 1;
                }
                continue;
            }
            break;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> Result<String, String> {
        let q = self.s[self.i];
        self.i += 1;
        let mut out = Vec::new();
        while let Some(&c) = self.s.get(self.i) {
            self.i += 1;
            match c {
                _ if c == q => return String::from_utf8(out).map_err(|_| "bad text".into()),
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("bad escape")?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'\\' | b'"' | b'\'' | b'\n' => out.push(e),
                        b'0'..=b'9' => {
                            let mut n = (e - b'0') as u32;
                            for _ in 0..2 {
                                match self.s.get(self.i) {
                                    Some(d @ b'0'..=b'9') => {
                                        n = n * 10 + (d - b'0') as u32;
                                        self.i += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push(u8::try_from(n).map_err(|_| "bad escape")?);
                        }
                        _ => return Err("unsupported escape".into()),
                    }
                }
                b'\n' => return Err("line break in string".into()),
                _ => out.push(c),
            }
        }
        Err("unterminated string".into())
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || b"+-.".contains(&self.s[self.i])) {
            // Stop at a sign that isn't part of an exponent.
            if (self.s[self.i] == b'-' || self.s[self.i] == b'+') && self.i > start && !matches!(self.s[self.i - 1], b'e' | b'E') {
                break;
            }
            self.i += 1;
        }
        let t = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        if let Ok(i) = t.parse::<i64>() {
            return Ok(Value::from(i));
        }
        match t.parse::<f64>() {
            Ok(f) if f.is_finite() && !t.contains(['x', 'X']) => Ok(num(f)),
            _ => Err(format!("unsupported number {t}")),
        }
    }

    fn ident(&mut self) -> Option<String> {
        let start = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || self.s[self.i] == b'_') {
            self.i += 1;
        }
        (self.i > start && !self.s[start].is_ascii_digit()).then(|| String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
    }

    fn value(&mut self, ptr: &str) -> Result<Value, String> {
        self.ws();
        let start = self.i;
        let c = *self.s.get(self.i).ok_or("unexpected end")?;
        let (v, kind) = match c {
            b'{' => return self.table(ptr),
            b'"' | b'\'' => (Value::String(self.string()?), Some(Kind::Str)),
            b'-' | b'.' | b'0'..=b'9' => (self.number()?, Some(Kind::Num)),
            _ => match self.ident().as_deref() {
                Some("true") => (Value::Bool(true), Some(Kind::Bool)),
                Some("false") => (Value::Bool(false), Some(Kind::Bool)),
                Some("nil") => (Value::Null, None),
                _ => return Err("unsupported value".into()),
            },
        };
        if let Some(kind) = kind {
            self.editable.insert(ptr.to_string(), self.spans.len());
            self.spans.push(Span { start, end: self.i, kind });
        }
        Ok(v)
    }

    fn table(&mut self, ptr: &str) -> Result<Value, String> {
        self.depth += 1;
        if self.depth > 200 {
            return Err("tables nested too deeply".into());
        }
        self.i += 1; // {
        let mut map = Map::new();
        let mut pos = 1;
        loop {
            self.ws();
            if self.eat(b'}') {
                break;
            }
            let save = self.i;
            let key = if self.eat(b'[') {
                self.ws();
                let k = match self.s.get(self.i) {
                    Some(b'"' | b'\'') => self.string()?,
                    _ => match self.number()? {
                        Value::Number(n) => n.to_string(),
                        _ => unreachable!(),
                    },
                };
                if !self.eat(b']') || !self.eat(b'=') {
                    return Err("expected ] =".into());
                }
                Some(k)
            } else {
                match self.ident() {
                    Some(id) if self.eat(b'=') => Some(id),
                    _ => {
                        self.i = save;
                        None
                    }
                }
            };
            let key = key.unwrap_or_else(|| {
                pos += 1;
                (pos - 1).to_string()
            });
            let k = crate::tree::unique_key(&map, &key);
            let v = self.value(&child(ptr, &k))?;
            insert_unique(&mut map, &key, v);
            self.ws();
            if !self.eat(b',') && !self.eat(b';') {
                self.ws();
                if !self.eat(b'}') {
                    return Err("expected , or }".into());
                }
                break;
            }
        }
        self.depth -= 1;
        Ok(Value::Object(map))
    }
}

fn lua_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[derive(Clone, Debug)]
struct LuaBackend {
    raw: String,
    spans: Vec<Span>,
}

impl Backend for LuaBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (leaf, v) in changes {
            let s = &self.spans[*leaf];
            let text = match (s.kind, v) {
                (Kind::Str, Value::String(t)) => lua_string(t),
                (Kind::Num, Value::Number(n)) => n.to_string(),
                (Kind::Bool, Value::Bool(b)) => b.to_string(),
                _ => return Err("this value has to keep its type".into()),
            };
            p.add(s.start, s.end, text.into_bytes());
        }
        p.apply(self.raw.as_bytes())
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match (self.spans[leaf].kind, v) {
            (Kind::Str, Value::String(_)) | (Kind::Num, Value::Number(_)) | (Kind::Bool, Value::Bool(_)) => Ok(()),
            _ => Err("this value has to keep its type".into()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Where the table starts and the name a trailing `return <name>` must repeat, for the two
/// forms: `return { ... }` and `local <name> = { ... }` followed by `return <name>` (Kingdom Rush).
fn head(text: &str) -> Option<(usize, Option<&str>)> {
    let t = text.trim_start();
    let lead = text.len() - t.len();
    if let Some(r) = t.strip_prefix("return") {
        return r.trim_start().starts_with('{').then_some((lead + 6, None));
    }
    let r = t.strip_prefix("local")?;
    if !r.starts_with(char::is_whitespace) {
        return None;
    }
    let r = r.trim_start();
    let end = r.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    let name = &r[..end];
    let after = r[end..].trim_start().strip_prefix('=')?;
    let skip = text.len() - after.len();
    after.trim_start().starts_with('{').then_some((skip, Some(name)))
}

pub fn looks(text: &str) -> bool {
    head(text).is_some()
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    let Some((start, name)) = head(text) else {
        return Err("not a Lua table".into());
    };
    let s = text.as_bytes();
    let mut p = P { s, i: start, spans: vec![], editable: HashMap::new(), depth: 0 };
    p.ws();
    let root = p.table("")?;
    p.ws();
    if let Some(name) = name {
        // `return <name>` closes the `local` form.
        let rest = &text[p.i..];
        let tail = rest.strip_prefix("return").map(str::trim).ok_or("unexpected text after the table")?;
        if tail != name {
            return Err("unexpected text after the table".into());
        }
        p.i = s.len();
    }
    if p.i != s.len() {
        return Err("unexpected text after the table".into());
    }
    let backend = LuaBackend { raw: text.to_string(), spans: p.spans };
    Ok(TreeDoc::new(root, p.editable, "Lua table", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balatro_style() {
        let src = r#"return {["GAME"]={["dollars"]=4,["chips"]=0.5,["won"]=false,["name"]="Red \"Deck\"",[1]="a",},["x"]={1,2,3},y = -2e-05, -- note
}"#;
        let mut doc = parse(src).unwrap();
        assert_eq!(doc.view.pointer("/GAME/dollars").unwrap(), 4);
        assert_eq!(doc.view.pointer("/GAME/name").unwrap(), "Red \"Deck\"");
        assert_eq!(doc.view.pointer("/x/3").unwrap(), 3);
        assert_eq!(doc.to_bytes().unwrap(), src.as_bytes());
        doc.set("/GAME/dollars", Value::from(9999)).unwrap();
        doc.set("/GAME/won", Value::Bool(true)).unwrap();
        doc.set("/GAME/name", Value::from("Blue\nDeck")).unwrap();
        assert!(doc.set("/GAME/chips", Value::from("text")).is_err());
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains(r#"["dollars"]=9999"#) && out.contains(r#""Blue\nDeck""#));
        assert_eq!(parse(&out).unwrap().view, doc.view);
    }

    #[test]
    fn local_form() {
        let src = "local obj1 = {\n\t[\"gems\"] = 5;\n\t[\"h\"] = {\n\t\t[\"xp\"] = 0;\n\t};\n}\nreturn obj1\n";
        let mut doc = parse(src).unwrap();
        assert_eq!(doc.view.pointer("/gems").unwrap(), 5);
        assert_eq!(doc.to_bytes().unwrap(), src.as_bytes());
        doc.set("/gems", Value::from(99)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("= 99;") && out.ends_with("return obj1\n"));
        assert!(parse("local x = {}\nreturn y").is_err());
    }
}
