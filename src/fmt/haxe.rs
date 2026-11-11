//! Haxe serialization: the save format of OpenFL and HaxeFlixel games (`FlxSave`, OpenFL's
//! `SharedObject` on desktop, whose `.sol` files hold this text and not Flash's AMF).
//!
//! The grammar is the one in the Haxe manual ("Serialization format"). The file is read into a
//! tree that keeps every token, opens as JSON, and is written back with only the edited
//! numbers, flags and text changed. Things JSON can't show (dates, bytes, class and enum
//! references, back-references to shared objects) are kept as they were.

use serde_json::{Map, Number, Value as Json};

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Null,
    True,
    False,
    Zero,
    Int(i64),
    /// The float as written (digits), or `k`, `m`, `p` for NaN, -Infinity, +Infinity.
    Float(String),
    /// A string: its text, and whether the file wrote it as a back-reference (`R`).
    Str { text: String, raw: Option<String>, was_ref: Option<usize> },
    /// `o … g` (anonymous object) or `c<class> … g` (class instance).
    Object { class: Option<Box<Node>>, fields: Vec<(Node, Node)> },
    /// `a … h`; `nulls` items stand for `u<n>` runs.
    Array(Vec<Item>),
    /// `l … h`
    List(Vec<Node>),
    /// `b … h` (string keys), `q … h` (`:int` keys), `M … h` (any keys).
    StrMap(Vec<(Node, Node)>),
    IntMap(Vec<(i64, Node)>),
    ObjMap(Vec<(Node, Node)>),
    /// `w<name><ctor>:<n>args` or `j<name>:<index>:<n>args`.
    Enum { name: Box<Node>, ctor: EnumCtor, args: Vec<Node> },
    /// `C<class> … g` (custom serializer): the values it wrote.
    Custom { class: Box<Node>, values: Vec<Node> },
    /// `x<value>`
    Exception(Box<Node>),
    /// Tokens kept as they are: `r<n>`, `v<date>`, `s<len>:<base64>`, `A<name>`, `B<name>`.
    Opaque { tag: u8, text: String, inner: Option<Box<Node>> },
}

#[derive(Clone, Debug, PartialEq)]
enum EnumCtor {
    Name(Box<Node>),
    Index(i64),
}

#[derive(Clone, Debug, PartialEq)]
enum Item {
    Value(Node),
    /// `u<n>`
    Nulls(usize),
}

struct Reader<'a> {
    s: &'a [u8],
    at: usize,
    strings: Vec<String>,
}

fn url_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
                out.push(u8::from_str_radix(h, 16).ok()?);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// What `StringTools.urlEncode` writes (JavaScript's `encodeURIComponent`).
fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for &c in s.as_bytes() {
        if c.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&c) {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{c:02X}"));
        }
    }
    out
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.at).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.at += 1;
        Some(c)
    }

    fn eat(&mut self, c: u8) -> Option<()> {
        (self.next()? == c).then_some(())
    }

    fn take_while(&mut self, f: impl Fn(u8) -> bool) -> &str {
        let start = self.at;
        while self.peek().is_some_and(&f) {
            self.at += 1;
        }
        std::str::from_utf8(&self.s[start..self.at]).unwrap_or("")
    }

    fn int(&mut self) -> Option<i64> {
        let neg = self.peek() == Some(b'-');
        if neg {
            self.at += 1;
        }
        let d = self.take_while(|c| c.is_ascii_digit());
        if d.is_empty() || d.len() > 18 {
            return None;
        }
        let n: i64 = d.parse().ok()?;
        Some(if neg { -n } else { n })
    }

    fn count(&mut self) -> Option<usize> {
        usize::try_from(self.int()?).ok()
    }

    fn float_text(&mut self) -> Option<String> {
        let t = self.take_while(|c| c.is_ascii_digit() || b"+-.eE".contains(&c)).to_string();
        t.parse::<f64>().ok()?;
        Some(t)
    }

    /// A string token (`y` or `R`), as object keys and names are written.
    fn string(&mut self) -> Option<Node> {
        match self.next()? {
            b'y' => {
                let n = self.count()?;
                self.eat(b':')?;
                let raw = std::str::from_utf8(self.s.get(self.at..self.at + n)?).ok()?.to_string();
                self.at += n;
                let text = url_decode(&raw)?;
                self.strings.push(text.clone());
                Some(Node::Str { text, raw: Some(raw), was_ref: None })
            }
            b'R' => {
                let k = self.count()?;
                Some(Node::Str { text: self.strings.get(k)?.clone(), raw: None, was_ref: Some(k) })
            }
            _ => None,
        }
    }

    fn pairs(&mut self, end: u8, key_is_string: bool, d: usize) -> Option<Vec<(Node, Node)>> {
        let mut v = vec![];
        while self.peek()? != end {
            let k = if key_is_string { self.string()? } else { self.value(d)? };
            v.push((k, self.value(d)?));
        }
        self.at += 1;
        Some(v)
    }

    fn value(&mut self, depth: usize) -> Option<Node> {
        if depth > 200 {
            return None;
        }
        let d = depth + 1;
        let tag = self.peek()?;
        if tag == b'y' || tag == b'R' {
            return self.string();
        }
        self.at += 1;
        Some(match tag {
            b'n' => Node::Null,
            b't' => Node::True,
            b'f' => Node::False,
            b'z' => Node::Zero,
            b'i' => Node::Int(self.int()?),
            b'd' => Node::Float(self.float_text()?),
            b'k' | b'm' | b'p' => Node::Float((tag as char).to_string()),
            b'o' => Node::Object { class: None, fields: self.pairs(b'g', true, d)? },
            b'c' => {
                let class = Box::new(self.string()?);
                Node::Object { class: Some(class), fields: self.pairs(b'g', true, d)? }
            }
            b'a' => {
                let mut items = vec![];
                loop {
                    match self.peek()? {
                        b'h' => {
                            self.at += 1;
                            break;
                        }
                        b'u' => {
                            self.at += 1;
                            items.push(Item::Nulls(self.count()?));
                        }
                        _ => items.push(Item::Value(self.value(d)?)),
                    }
                }
                Node::Array(items)
            }
            b'l' => Node::List(self.until_depth(b'h', d)?),
            b'b' => Node::StrMap(self.pairs(b'h', true, d)?),
            b'q' => {
                let mut v = vec![];
                while self.peek()? != b'h' {
                    self.eat(b':')?;
                    let k = self.int()?;
                    v.push((k, self.value(d)?));
                }
                self.at += 1;
                Node::IntMap(v)
            }
            b'M' => Node::ObjMap(self.pairs(b'h', false, d)?),
            b'w' | b'j' => {
                let name = Box::new(self.string()?);
                let ctor = if tag == b'w' {
                    EnumCtor::Name(Box::new(self.string()?))
                } else {
                    self.eat(b':')?;
                    EnumCtor::Index(self.int()?)
                };
                self.eat(b':')?;
                let n = self.count()?;
                let args = (0..n).map(|_| self.value(d)).collect::<Option<Vec<_>>>()?;
                Node::Enum { name, ctor, args }
            }
            b'C' => {
                let class = Box::new(self.string()?);
                Node::Custom { class, values: self.until_depth(b'g', d)? }
            }
            b'x' => Node::Exception(Box::new(self.value(d)?)),
            b'r' => Node::Opaque { tag, text: self.count()?.to_string(), inner: None },
            b'v' => {
                let s = &self.s[self.at..];
                let text = if s.len() >= 19 && s[..4].iter().all(u8::is_ascii_digit) && s[4] == b'-' {
                    let t = std::str::from_utf8(&s[..19]).ok()?.to_string();
                    self.at += 19;
                    t
                } else {
                    self.float_text()?
                };
                Node::Opaque { tag, text, inner: None }
            }
            b's' => {
                let n = self.count()?;
                self.eat(b':')?;
                let b = self.s.get(self.at..self.at + n)?;
                if !b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'%' || *c == b':') {
                    return None;
                }
                self.at += n;
                Node::Opaque { tag, text: format!("{n}:{}", std::str::from_utf8(b).ok()?), inner: None }
            }
            b'A' | b'B' => Node::Opaque { tag, text: String::new(), inner: Some(Box::new(self.string()?)) },
            _ => return None,
        })
    }

    fn until_depth(&mut self, end: u8, d: usize) -> Option<Vec<Node>> {
        let mut v = vec![];
        while self.peek()? != end {
            v.push(self.value(d)?);
        }
        self.at += 1;
        Some(v)
    }
}

struct Writer {
    out: String,
    strings: Vec<String>,
}

impl Writer {
    fn string(&mut self, text: &str, raw: &Option<String>, was_ref: Option<usize>) {
        if let Some(k) = was_ref {
            if self.strings.get(k).map(String::as_str) == Some(text) {
                self.out.push_str(&format!("R{k}"));
                return;
            }
            if let Some(i) = self.strings.iter().position(|s| s == text) {
                self.out.push_str(&format!("R{i}"));
                return;
            }
        }
        let enc = match raw {
            Some(r) if url_decode(r).as_deref() == Some(text) => r.clone(),
            _ => url_encode(text),
        };
        self.out.push_str(&format!("y{}:{enc}", enc.len()));
        self.strings.push(text.to_string());
    }

    fn node(&mut self, n: &Node) {
        match n {
            Node::Null => self.out.push('n'),
            Node::True => self.out.push('t'),
            Node::False => self.out.push('f'),
            Node::Zero => self.out.push('z'),
            Node::Int(i) => self.out.push_str(&format!("i{i}")),
            Node::Float(t) if t.len() == 1 && "kmp".contains(t.as_str()) => self.out.push_str(t),
            Node::Float(t) => self.out.push_str(&format!("d{t}")),
            Node::Str { text, raw, was_ref } => self.string(text, raw, *was_ref),
            Node::Object { class, fields } => {
                match class {
                    Some(c) => {
                        self.out.push('c');
                        self.node(c);
                    }
                    None => self.out.push('o'),
                }
                for (k, v) in fields {
                    self.node(k);
                    self.node(v);
                }
                self.out.push('g');
            }
            Node::Array(items) => {
                self.out.push('a');
                for i in items {
                    match i {
                        Item::Value(v) => self.node(v),
                        Item::Nulls(n) => self.out.push_str(&format!("u{n}")),
                    }
                }
                self.out.push('h');
            }
            Node::List(v) => {
                self.out.push('l');
                v.iter().for_each(|x| self.node(x));
                self.out.push('h');
            }
            Node::StrMap(v) | Node::ObjMap(v) => {
                self.out.push(if matches!(n, Node::StrMap(_)) { 'b' } else { 'M' });
                for (k, x) in v {
                    self.node(k);
                    self.node(x);
                }
                self.out.push('h');
            }
            Node::IntMap(v) => {
                self.out.push('q');
                for (k, x) in v {
                    self.out.push_str(&format!(":{k}"));
                    self.node(x);
                }
                self.out.push('h');
            }
            Node::Enum { name, ctor, args } => {
                match ctor {
                    EnumCtor::Name(c) => {
                        self.out.push('w');
                        self.node(name);
                        self.node(c);
                    }
                    EnumCtor::Index(i) => {
                        self.out.push('j');
                        self.node(name);
                        self.out.push_str(&format!(":{i}"));
                    }
                }
                self.out.push_str(&format!(":{}", args.len()));
                args.iter().for_each(|a| self.node(a));
            }
            Node::Custom { class, values } => {
                self.out.push('C');
                self.node(class);
                values.iter().for_each(|x| self.node(x));
                self.out.push('g');
            }
            Node::Exception(v) => {
                self.out.push('x');
                self.node(v);
            }
            Node::Opaque { tag, text, inner } => {
                self.out.push(*tag as char);
                self.out.push_str(text);
                if let Some(i) = inner {
                    self.node(i);
                }
            }
        }
    }
}

fn parse(d: &[u8]) -> Option<Node> {
    let mut r = Reader { s: d, at: 0, strings: vec![] };
    let n = r.value(0)?;
    (r.at == d.len()).then_some(n)
}

fn write(n: &Node) -> String {
    let mut w = Writer { out: String::new(), strings: vec![] };
    w.node(n);
    w.out
}

/// True for data that is a whole Haxe-serialized object, map or list (not a lone value, so
/// that ordinary text is never mistaken for it).
pub fn looks(d: &[u8]) -> bool {
    d.len() > 3 && b"ocbqMal".contains(&d[0]) && d.is_ascii() && parse(d).is_some()
}

fn key_text(k: &Node) -> String {
    match k {
        Node::Str { text, .. } => text.clone(),
        other => placeholder(other),
    }
}

fn placeholder(n: &Node) -> String {
    match n {
        Node::Opaque { tag: b'v', text, .. } => format!("\u{2039}date {text}\u{203A}"),
        Node::Opaque { tag: b's', text, .. } => format!("\u{2039}{} bytes\u{203A}", text.split(':').nth(1).map_or(0, |b| b.len() * 3 / 4)),
        Node::Opaque { tag: b'r', text, .. } => format!("\u{2039}same as object #{text}\u{203A}"),
        Node::Opaque { inner: Some(i), .. } => format!("\u{2039}type {}\u{203A}", key_text(i)),
        Node::Enum { ctor: EnumCtor::Name(c), args, .. } if args.is_empty() => key_text(c),
        Node::Enum { ctor: EnumCtor::Index(i), args, .. } if args.is_empty() => format!("\u{2039}enum #{i}\u{203A}"),
        _ => "\u{2039}kept as it is\u{203A}".into(),
    }
}

fn float_json(t: &str) -> Json {
    match t {
        "k" => Json::String("NaN".into()),
        "m" => Json::String("-Infinity".into()),
        "p" => Json::String("Infinity".into()),
        _ => t.parse::<Number>().map(Json::Number).unwrap_or_else(|_| Json::String(t.into())),
    }
}

fn pairs_json(v: &[(Node, Node)]) -> Json {
    Json::Object(v.iter().map(|(k, x)| (key_text(k), to_json(x))).collect())
}

fn to_json(n: &Node) -> Json {
    match n {
        Node::Null => Json::Null,
        Node::True => Json::Bool(true),
        Node::False => Json::Bool(false),
        Node::Zero => Json::from(0),
        Node::Int(i) => Json::from(*i),
        Node::Float(t) => float_json(t),
        Node::Str { text, .. } => Json::String(text.clone()),
        Node::Object { fields, .. } => pairs_json(fields),
        Node::StrMap(v) => pairs_json(v),
        Node::IntMap(v) => Json::Object(v.iter().map(|(k, x)| (k.to_string(), to_json(x))).collect()),
        Node::ObjMap(v) => Json::Array(v.iter().map(|(k, x)| Json::Array(vec![to_json(k), to_json(x)])).collect()),
        Node::Array(items) => Json::Array(
            items
                .iter()
                .flat_map(|i| match i {
                    Item::Value(v) => vec![to_json(v)],
                    Item::Nulls(n) => vec![Json::Null; *n],
                })
                .collect(),
        ),
        Node::List(v) | Node::Custom { values: v, .. } => Json::Array(v.iter().map(to_json).collect()),
        Node::Enum { ctor, args, .. } if !args.is_empty() => {
            let name = match ctor {
                EnumCtor::Name(c) => key_text(c),
                EnumCtor::Index(i) => format!("#{i}"),
            };
            let mut m = Map::new();
            m.insert(name, Json::Array(args.iter().map(to_json).collect()));
            Json::Object(m)
        }
        Node::Exception(v) => to_json(v),
        other => Json::String(placeholder(other)),
    }
}

fn number(j: &Json, path: &str) -> Result<f64, String> {
    match j.as_str() {
        Some("NaN") => Ok(f64::NAN),
        Some("Infinity") => Ok(f64::INFINITY),
        Some("-Infinity") => Ok(f64::NEG_INFINITY),
        _ => j.as_f64().ok_or_else(|| format!("{path} must be a number")),
    }
}

/// The float token for `f` (`d…`, or `k`/`m`/`p`).
fn float_token(f: f64) -> String {
    if f.is_nan() {
        "k".into()
    } else if f.is_infinite() {
        if f > 0.0 { "p" } else { "m" }.into()
    } else {
        format!("{f}")
    }
}

fn whole(j: &Json) -> Option<i64> {
    j.as_i64().or_else(|| j.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15).map(|f| f as i64))
}

fn set_number(n: &mut Node, j: &Json, path: &str) -> Result<(), String> {
    let old = to_json(n);
    if &old == j {
        return Ok(());
    }
    let f = number(j, path)?;
    if matches!(n, Node::Float(_)) {
        if old.as_f64() == Some(f) {
            return Ok(());
        }
        *n = Node::Float(float_token(f));
        return Ok(());
    }
    // An Int stays an Int while it fits (Haxe ints are 32 bits); otherwise it becomes a Float.
    *n = match whole(j).and_then(|i| i32::try_from(i).ok()) {
        Some(0) => Node::Zero,
        Some(i) => Node::Int(i.into()),
        None => Node::Float(float_token(f)),
    };
    Ok(())
}

fn apply_pairs(v: &mut [(Node, Node)], j: &Json, path: &str) -> Result<(), String> {
    let m = j.as_object().ok_or_else(|| format!("{path} must stay an object"))?;
    for (k, x) in v {
        let name = key_text(k);
        if let Some(n) = m.get(&name) {
            apply(x, n, &format!("{path}/{name}"))?;
        }
    }
    Ok(())
}

fn list<'a>(j: &'a Json, path: &str) -> Result<&'a Vec<Json>, String> {
    j.as_array().ok_or_else(|| format!("{path} must stay a list"))
}

fn apply_list(v: &mut [Node], j: &Json, path: &str) -> Result<(), String> {
    for (i, (x, n)) in v.iter_mut().zip(list(j, path)?).enumerate() {
        apply(x, n, &format!("{path}/{i}"))?;
    }
    Ok(())
}

/// Puts edited JSON back into the tree, keeping each value's own kind.
fn apply(n: &mut Node, j: &Json, path: &str) -> Result<(), String> {
    match n {
        Node::Zero | Node::Int(_) | Node::Float(_) => set_number(n, j, path)?,
        Node::True | Node::False => *n = if j.as_bool().ok_or_else(|| format!("{path} must be true or false"))? { Node::True } else { Node::False },
        Node::Str { text, .. } => *text = j.as_str().ok_or_else(|| format!("{path} must be text"))?.to_string(),
        Node::Object { fields, .. } => apply_pairs(fields, j, path)?,
        Node::StrMap(v) => apply_pairs(v, j, path)?,
        Node::IntMap(v) => {
            let m = j.as_object().ok_or_else(|| format!("{path} must stay an object"))?;
            for (k, x) in v {
                if let Some(n) = m.get(&k.to_string()) {
                    apply(x, n, &format!("{path}/{k}"))?;
                }
            }
        }
        Node::ObjMap(v) => {
            for ((_, x), n) in v.iter_mut().zip(list(j, path)?) {
                if let Some(p) = n.as_array().filter(|p| p.len() == 2) {
                    apply(x, &p[1], path)?;
                }
            }
        }
        Node::Array(items) => {
            let l = list(j, path)?;
            let mut at = 0;
            for it in items {
                match it {
                    Item::Value(v) => {
                        if let Some(n) = l.get(at) {
                            apply(v, n, &format!("{path}/{at}"))?;
                        }
                        at += 1;
                    }
                    Item::Nulls(c) => {
                        if l.iter().skip(at).take(*c).any(|x| !x.is_null()) {
                            return Err(format!("{path}: an empty entry can't be given a value"));
                        }
                        at += *c;
                    }
                }
            }
        }
        Node::List(v) | Node::Custom { values: v, .. } => apply_list(v, j, path)?,
        Node::Enum { args, .. } if !args.is_empty() => {
            let m = j.as_object().filter(|m| m.len() == 1).ok_or_else(|| format!("{path} must stay as it is"))?;
            apply_list(args, m.values().next().unwrap_or(&Json::Null), path)?;
        }
        Node::Exception(v) => apply(v, j, path)?,
        Node::Null => {
            if !j.is_null() {
                return Err(format!("{path} is empty in the save and can't be given a value"));
            }
        }
        other => {
            if j.as_str() != Some(&placeholder(other)) {
                return Err(format!("{path} can't be edited"));
            }
        }
    }
    Ok(())
}

/// The values of a Haxe-serialized save as JSON (only when it writes back exactly).
pub fn open(d: &[u8]) -> Option<Vec<u8>> {
    if !looks(d) {
        return None;
    }
    let n = parse(d)?;
    if write(&n).as_bytes() != d {
        return None;
    }
    serde_json::to_vec_pretty(&to_json(&n)).ok()
}

/// `original` with the edited values from `json`.
pub fn save(original: &[u8], json: &[u8]) -> Result<Vec<u8>, String> {
    let mut n = parse(original).ok_or("the original save no longer parses")?;
    let j: Json = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    apply(&mut n, &j, "")?;
    Ok(write(&n).into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_round_trip() {
        // A file using most tags: strings and back-references, arrays with null runs, maps,
        // enums, dates and bytes, floats and NaN.
        let files = [
            "oy5:coinsi250y4:namey7:Ann%20Ly5:speedd1.5y5:flagsatfnu3zhy6:levelsby3:onei3R0i12hg",
            "cy6:Playery2:hpi-5y3:bagly1:aR3hy4:kindwy4:Kindy4:Fire:1d2.25y4:seenv2024-01-02 03:04:05y3:rawi1y4:dataq:1n:2zhg",
            "ay4:wavekpmjy1:E:3:2i1i2s4:AAECr0h",
        ];
        for f in files {
            let n = parse(f.as_bytes()).unwrap_or_else(|| panic!("parses {f}"));
            assert_eq!(write(&n), f);
        }
    }

    #[test]
    fn edit_keeps_kinds_and_back_references() {
        let file = b"oy5:coinsi250y4:namey3:Anny5:otherR2y5:speedd1.5y4:hardfg";
        let json = open(file).expect("opens");
        let v: Json = serde_json::from_slice(&json).unwrap();
        assert_eq!(v, serde_json::json!({"coins": 250, "name": "Ann", "other": "Ann", "speed": 1.5, "hard": false}));
        assert_eq!(save(file, &json).unwrap(), file);
        let out = save(file, br#"{"coins": 99999999999, "name": "Bo Bo", "other": "Ann", "speed": 2, "hard": true}"#).unwrap();
        let back: Json = serde_json::from_slice(&open(&out).unwrap()).unwrap();
        assert_eq!(back, serde_json::json!({"coins": 99999999999u64, "name": "Bo Bo", "other": "Ann", "speed": 2, "hard": true}));
        // "other" still says Ann although the string it pointed at changed.
        assert!(String::from_utf8(out).unwrap().contains("y5:other"));
    }

    #[test]
    fn plain_text_is_not_haxe() {
        for t in [&b"ok then"[..], b"abc", b"opening night", b"{\"a\":1}", b"list of things"] {
            assert!(!looks(t));
        }
    }

    /// HAXE_SAMPLES = a folder of Haxe-serialized saves: each must open and write back unchanged.
    #[test]
    #[ignore]
    fn sample_files() {
        let dir = std::env::var("HAXE_SAMPLES").expect("HAXE_SAMPLES");
        for e in std::fs::read_dir(dir).unwrap().flatten().filter(|e| e.path().is_file()) {
            let d = std::fs::read(e.path()).unwrap();
            let json = open(&d).unwrap_or_else(|| panic!("{:?} doesn't open", e.path()));
            assert_eq!(save(&d, &json).unwrap(), d, "{:?}", e.path());
            println!("{:?}: {} bytes of JSON", e.path(), json.len());
        }
    }
}
