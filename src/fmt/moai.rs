//! Saves written by the MOAI game framework's object serializer (Invisible, Inc.): a Lua file
//! whose `init` function sets every field of every serialized table with lines such as
//! `table [ "xp" ] = 5030`, and links tables to each other by id (`objects [ 0x00000002 ]`).
//! The file is read into a tree that follows the links from the returned object. Numbers and
//! booleans are edited in place; every other byte of the file stays as it was.

use crate::tree::{child, insert_unique, num, unique_key, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

pub const KIND: &str = "MOAI Lua save";

#[derive(Clone, Copy, Debug)]
enum Kind {
    Num,
    Bool,
}

#[derive(Clone, Debug)]
struct Span {
    start: usize,
    end: usize,
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Val {
    Ref(u32),
    Num(Value, Span),
    Bool(bool, Span),
    Text(String),
}

pub fn looks(text: &str) -> bool {
    let t = text.trim_start_matches('\u{feff}');
    t.starts_with("--MOAI") && t.contains("MOAIDeserializer") && t.contains("local function init")
}

struct Scan<'a> {
    s: &'a [u8],
    i: usize,
}

impl Scan<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn starts(&self, t: &str) -> bool {
        self.s[self.i..].starts_with(t.as_bytes())
    }

    fn eat(&mut self, t: &str) -> bool {
        if self.starts(t) {
            self.i += t.len();
            true
        } else {
            false
        }
    }

    fn skip_line(&mut self) {
        while self.i < self.s.len() && self.s[self.i] != b'\n' {
            self.i += 1;
        }
    }

    /// `[ 0x0000001F ]` after `objects `: the id.
    fn object_id(&mut self) -> Result<u32, String> {
        self.ws();
        if !self.eat("[") {
            return Err("expected [".into());
        }
        self.ws();
        if !self.eat("0x") {
            return Err("expected an object id".into());
        }
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_hexdigit() {
            self.i += 1;
        }
        let id = u32::from_str_radix(std::str::from_utf8(&self.s[start..self.i]).unwrap_or(""), 16).map_err(|_| "bad object id")?;
        self.ws();
        if !self.eat("]") {
            return Err("expected ]".into());
        }
        Ok(id)
    }

    /// A key: a quoted string or a number.
    fn key(&mut self) -> Result<String, String> {
        self.ws();
        if self.s.get(self.i) == Some(&b'"') {
            Ok(self.quoted()?)
        } else {
            let start = self.i;
            while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || b"+-.eE".contains(&self.s[self.i])) {
                self.i += 1;
            }
            if self.i == start {
                return Err("bad key".into());
            }
            Ok(String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
        }
    }

    fn quoted(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = Vec::new();
        while let Some(&c) = self.s.get(self.i) {
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).map_err(|_| "bad text".to_string()),
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("bad escape")?;
                    self.i += 1;
                    out.push(match e {
                        b'n' => b'\n',
                        b't' => b'\t',
                        b'r' => b'\r',
                        other => other,
                    });
                }
                _ => out.push(c),
            }
        }
        Err("unterminated string".into())
    }

    fn value(&mut self) -> Result<Val, String> {
        self.ws();
        let start = self.i;
        if self.eat("objects") {
            return Ok(Val::Ref(self.object_id()?));
        }
        if self.starts("[==[") {
            self.i += 4;
            let from = self.i;
            let rest = &self.s[self.i..];
            let at = rest.windows(4).position(|w| w == b"]==]").ok_or("unterminated long string")?;
            self.i += at + 4;
            return Ok(Val::Text(String::from_utf8_lossy(&self.s[from..from + at]).into_owned()));
        }
        if self.s.get(self.i) == Some(&b'"') {
            return Ok(Val::Text(self.quoted()?));
        }
        if self.eat("true") {
            return Ok(Val::Bool(true, Span { start, end: self.i, kind: Kind::Bool }));
        }
        if self.eat("false") {
            return Ok(Val::Bool(false, Span { start, end: self.i, kind: Kind::Bool }));
        }
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || b"+-.".contains(&self.s[self.i])) {
            self.i += 1;
        }
        let t = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| "bad number")?;
        let span = Span { start, end: self.i, kind: Kind::Num };
        if let Ok(n) = t.parse::<i64>() {
            return Ok(Val::Num(Value::from(n), span));
        }
        match t.parse::<f64>() {
            Ok(f) if f.is_finite() => Ok(Val::Num(num(f), span)),
            _ => Err(format!("unsupported value {t}")),
        }
    }
}

struct Objects {
    /// Fields of each serialized table, in file order.
    tables: HashMap<u32, Vec<(String, Val)>>,
    root: u32,
}

fn scan(text: &str) -> Result<Objects, String> {
    let s = text.as_bytes();
    let init = text.find("local function init").ok_or("no init function")?;
    let mut sc = Scan { s, i: init };
    sc.skip_line();
    let mut tables: HashMap<u32, Vec<(String, Val)>> = HashMap::new();
    let mut cur: Option<u32> = None;
    loop {
        sc.ws();
        if sc.i >= s.len() {
            return Err("the init function does not end".into());
        }
        if sc.starts("--") || sc.starts("local table") {
            sc.skip_line();
        } else if sc.starts("table") {
            sc.i += 5;
            sc.ws();
            if sc.eat("=") {
                sc.ws();
                if !sc.eat("objects") {
                    return Err("expected objects".into());
                }
                let id = sc.object_id()?;
                tables.entry(id).or_default();
                cur = Some(id);
            } else if sc.eat("[") {
                let key = sc.key()?;
                sc.ws();
                if !sc.eat("]") {
                    return Err("expected ]".into());
                }
                sc.ws();
                if !sc.eat("=") {
                    return Err("expected =".into());
                }
                let v = sc.value()?;
                tables.get_mut(&cur.ok_or("a field before any table")?).ok_or("unknown table")?.push((key, v));
            } else {
                return Err("unexpected text in init".into());
            }
        } else if sc.starts("end") {
            break;
        } else {
            return Err("unexpected text in init".into());
        }
    }
    // The returned object is the root.
    let ret = text.rfind("return objects").ok_or("no return")?;
    let mut sc = Scan { s, i: ret + "return ".len() };
    sc.eat("objects");
    let root = sc.object_id()?;
    if !tables.contains_key(&root) {
        return Err("the returned table is missing".into());
    }
    Ok(Objects { tables, root })
}

struct Build<'a> {
    o: &'a Objects,
    spans: Vec<Span>,
    editable: HashMap<String, usize>,
    seen: HashSet<u32>,
}

impl Build<'_> {
    fn table(&mut self, id: u32, ptr: &str, depth: usize) -> Value {
        let mut map = Map::new();
        if depth > 64 || !self.seen.insert(id) {
            return Value::String("(the same table as another entry)".into());
        }
        let fields: Vec<(String, Val)> = self.o.tables.get(&id).cloned().unwrap_or_default();
        for (key, v) in fields {
            let k = unique_key(&map, &key);
            let p = child(ptr, &k);
            let value = match v {
                Val::Ref(r) if self.o.tables.contains_key(&r) => self.table(r, &p, depth + 1),
                Val::Ref(_) => Value::Null,
                Val::Num(n, span) => {
                    self.editable.insert(p, self.spans.len());
                    self.spans.push(span);
                    n
                }
                Val::Bool(b, span) => {
                    self.editable.insert(p, self.spans.len());
                    self.spans.push(span);
                    Value::Bool(b)
                }
                Val::Text(t) => Value::String(t),
            };
            insert_unique(&mut map, &key, value);
        }
        Value::Object(map)
    }
}

#[derive(Clone, Debug)]
struct MoaiBackend {
    raw: String,
    spans: Vec<Span>,
}

impl Backend for MoaiBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (leaf, v) in changes {
            let s = &self.spans[*leaf];
            let text = match (s.kind, v) {
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
            (Kind::Num, Value::Number(_)) | (Kind::Bool, Value::Bool(_)) => Ok(()),
            _ => Err("this value has to keep its type".into()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not a MOAI save".into());
    }
    let objects = scan(text)?;
    let mut b = Build { o: &objects, spans: vec![], editable: HashMap::new(), seen: HashSet::new() };
    let root = b.table(objects.root, "", 0);
    let backend = MoaiBackend { raw: text.to_string(), spans: b.spans };
    Ok(TreeDoc::new(root, b.editable, KIND, Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "--MOAI\nserializer = ... or MOAIDeserializer.new ()\n\nlocal function init ( objects )\n\n\t--Initializing Tables\n\tlocal table\n\n\ttable = objects [ 0x00000001 ]\n\ttable [ \"saveSlots\" ] = objects [ 0x00000002 ]\n\ttable [ \"xp\" ] = 5030\n\ttable [ \"retail\" ] = true\n\ttable [ \"name\" ] = [==[default]==]\n\n\ttable = objects [ 0x00000002 ]\n\ttable [ 1 ]\t= objects [ 0x00000003 ]\n\n\ttable = objects [ 0x00000003 ]\n\ttable [ \"cash\" ] = 1000\n\ttable [ \"drift\" ] = -7.6293918027659e-006\n\ttable [ \"text\" ] = [==[line one\nline two]==]\nend\n\n--Declaring Objects\nlocal objects = {\n\t[ 0x00000001 ] = {},\n}\n\ninit ( objects )\n\n--Returning Tables\nreturn objects [ 0x00000001 ]\n";

    #[test]
    fn reads_follows_links_and_edits_in_place() {
        let mut doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.view.pointer("/xp").unwrap(), 5030);
        assert_eq!(doc.view.pointer("/saveSlots/1/cash").unwrap(), 1000);
        assert_eq!(doc.view.pointer("/name").unwrap(), "default");
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        doc.set("/saveSlots/1/cash", Value::from(99999)).unwrap();
        doc.set("/retail", Value::Bool(false)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("[ \"cash\" ] = 99999") && out.contains("[ \"retail\" ] = false"));
        assert!(out.contains("line one\nline two"));
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/saveSlots/1/cash").unwrap(), 99999);
        assert!(doc.set("/xp", Value::from("text")).is_err());
    }

    #[test]
    fn a_plain_lua_table_is_not_taken() {
        assert!(!looks("return { [\"a\"] = 1 }"));
    }
}
