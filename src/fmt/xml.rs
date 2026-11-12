//! XML saves (Stardew Valley, .NET XmlSerializer, many Unity games).
//!
//! A small span-tracking parser: every editable text node and attribute value keeps
//! its exact byte range, and saving only rewrites those ranges — declarations,
//! comments, whitespace and attribute order stay byte-for-byte identical.

use crate::tree::{child, text_to_value, value_to_text, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum Leaf {
    /// Text between tags (`cdata` = inside <![CDATA[ ]]>, written unescaped).
    Text { start: usize, end: usize, cdata: bool, orig: String },
    /// Attribute value between its quotes.
    Attr { start: usize, end: usize, quote: u8, orig: String },
    /// `<name/>` with no attributes — becomes `<name>text</name>` when given a value.
    Empty { start: usize, end: usize, name: String },
}

struct Elem {
    name: String,
    attrs: Vec<(String, usize, usize, u8, Option<String>)>,
    children: Vec<Elem>,
    /// (start, end, cdata, unescaped text — None if it has unknown entities)
    texts: Vec<(usize, usize, bool, Option<String>)>,
    self_closing: bool,
    tag_start: usize,
    tag_end: usize,
    content_start: usize,
}

struct P<'a> {
    s: &'a str,
    b: &'a [u8],
    pos: usize,
}

impl P<'_> {
    fn starts(&self, t: &str) -> bool {
        self.b[self.pos..].starts_with(t.as_bytes())
    }
    fn skip_until(&mut self, t: &str) -> Result<(), String> {
        let rest = &self.s[self.pos..];
        let i = rest.find(t).ok_or_else(|| format!("unterminated {t}"))?;
        self.pos += i + t.len();
        Ok(())
    }
    fn ws(&mut self) {
        while self.pos < self.b.len() && self.b[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
    fn name(&mut self) -> Result<String, String> {
        let start = self.pos;
        while self.pos < self.b.len() && !self.b[self.pos].is_ascii_whitespace() && !b"/>=".contains(&self.b[self.pos]) {
            self.pos += 1;
        }
        if start == self.pos {
            return Err(format!("expected a name at byte {start}"));
        }
        Ok(self.s[start..self.pos].to_string())
    }
    /// Comments, processing instructions and DOCTYPE outside the root.
    fn misc(&mut self) -> Result<(), String> {
        loop {
            self.ws();
            if self.starts("<?") {
                self.skip_until("?>")?;
            } else if self.starts("<!--") {
                self.skip_until("-->")?;
            } else if self.starts("<!DOCTYPE") || self.starts("<!doctype") {
                let mut depth = 0;
                while self.pos < self.b.len() {
                    match self.b[self.pos] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        b'>' if depth == 0 => {
                            self.pos += 1;
                            break;
                        }
                        _ => {}
                    }
                    self.pos += 1;
                }
            } else {
                return Ok(());
            }
        }
    }
    fn element(&mut self, depth: usize) -> Result<Elem, String> {
        if depth > 512 {
            return Err("nested too deep".into());
        }
        let tag_start = self.pos;
        if !self.starts("<") {
            return Err(format!("expected '<' at byte {}", self.pos));
        }
        self.pos += 1;
        let name = self.name()?;
        let mut attrs = Vec::new();
        loop {
            self.ws();
            if self.starts("/>") {
                self.pos += 2;
                return Ok(Elem { name, attrs, children: vec![], texts: vec![], self_closing: true, tag_start, tag_end: self.pos, content_start: self.pos });
            }
            if self.starts(">") {
                self.pos += 1;
                break;
            }
            let an = self.name()?;
            self.ws();
            if !self.starts("=") {
                return Err(format!("expected '=' after attribute {an}"));
            }
            self.pos += 1;
            self.ws();
            let q = *self.b.get(self.pos).ok_or("unexpected end")?;
            if q != b'"' && q != b'\'' {
                return Err("attribute value must be quoted".into());
            }
            self.pos += 1;
            let vs = self.pos;
            while self.pos < self.b.len() && self.b[self.pos] != q {
                self.pos += 1;
            }
            if self.pos >= self.b.len() {
                return Err("unterminated attribute".into());
            }
            let ve = self.pos;
            self.pos += 1;
            attrs.push((an, vs, ve, q, unescape(&self.s[vs..ve])));
        }
        let content_start = self.pos;
        let mut children = Vec::new();
        let mut texts = Vec::new();
        loop {
            if self.pos >= self.b.len() {
                return Err(format!("unclosed <{name}>"));
            }
            if self.starts("</") {
                self.pos += 2;
                let end_name = self.name()?;
                if end_name != name {
                    return Err(format!("</{end_name}> doesn't match <{name}>"));
                }
                self.ws();
                if !self.starts(">") {
                    return Err("expected '>'".into());
                }
                self.pos += 1;
                return Ok(Elem { name, attrs, children, texts, self_closing: false, tag_start, tag_end: self.pos, content_start });
            } else if self.starts("<!--") {
                self.skip_until("-->")?;
            } else if self.starts("<![CDATA[") {
                let start = self.pos + 9;
                self.skip_until("]]>")?;
                let end = self.pos - 3;
                texts.push((start, end, true, Some(self.s[start..end].to_string())));
            } else if self.starts("<?") {
                self.skip_until("?>")?;
            } else if self.starts("<") {
                children.push(self.element(depth + 1)?);
            } else {
                let start = self.pos;
                while self.pos < self.b.len() && self.b[self.pos] != b'<' {
                    self.pos += 1;
                }
                texts.push((start, self.pos, false, unescape(&self.s[start..self.pos])));
            }
        }
    }
}

/// Decode XML entities; None if there's an entity we don't know (kept read-only).
fn unescape(s: &str) -> Option<String> {
    if !s.contains('&') {
        return Some(s.to_string());
    }
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let semi = rest[i..].find(';')? + i;
        let ent = &rest[i + 1..semi];
        let ch = match ent {
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            _ if ent.starts_with("#x") || ent.starts_with("#X") => char::from_u32(u32::from_str_radix(&ent[2..], 16).ok()?)?,
            _ if ent.starts_with('#') => char::from_u32(ent[1..].parse().ok()?)?,
            _ => return None,
        };
        out.push(ch);
        rest = &rest[semi + 1..];
    }
    out.push_str(rest);
    Some(out)
}

fn escape(s: &str, quote: Option<u8>) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quote == Some(b'"') => out.push_str("&quot;"),
            '\'' if quote == Some(b'\'') => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

fn parse_root(s: &str) -> Result<Elem, String> {
    let mut p = P { s, b: s.as_bytes(), pos: 0 };
    p.misc()?;
    let root = p.element(0)?;
    p.misc()?;
    p.ws();
    if p.pos != p.b.len() {
        return Err("extra content after the root element".into());
    }
    Ok(root)
}

pub fn looks_like_xml(t: &str) -> bool {
    t.trim_start().starts_with('<') && parse_root(t.trim()).is_ok()
}

struct Builder {
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
    /// Element names that are always shown as a list, even when there is only one.
    lists: &'static [&'static str],
}

impl Builder {
    fn leaf(&mut self, ptr: &str, leaf: Leaf, v: Value, editable: bool) -> Value {
        if editable {
            self.editable.insert(ptr.to_string(), self.leaves.len());
            self.leaves.push(leaf);
        }
        v
    }

    fn view(&mut self, e: &Elem, ptr: &str) -> Value {
        let meaningful: Vec<&(usize, usize, bool, Option<String>)> =
            e.texts.iter().filter(|t| t.2 || t.3.as_deref().is_none_or(|s| !s.trim().is_empty())).collect();
        if e.attrs.is_empty() && e.children.is_empty() {
            return match (e.texts.as_slice(), e.self_closing) {
                ([], true) => self.leaf(ptr, Leaf::Empty { start: e.tag_start, end: e.tag_end, name: e.name.clone() }, Value::String(String::new()), true),
                ([], false) => self.leaf(
                    ptr,
                    Leaf::Text { start: e.content_start, end: e.content_start, cdata: false, orig: String::new() },
                    Value::String(String::new()),
                    true,
                ),
                ([(s, en, cdata, Some(t))], _) => {
                    let v = if *cdata { Value::String(t.clone()) } else { text_to_value(t) };
                    self.leaf(ptr, Leaf::Text { start: *s, end: *en, cdata: *cdata, orig: t.clone() }, v, true)
                }
                _ => {
                    let joined: String = e.texts.iter().map(|t| t.3.clone().unwrap_or_default()).collect();
                    Value::String(joined)
                }
            };
        }
        let mut map = Map::new();
        for (an, vs, ve, q, val) in &e.attrs {
            let key = format!("@{an}");
            let p = child(ptr, &key);
            let v = match val {
                Some(t) => self.leaf(&p, Leaf::Attr { start: *vs, end: *ve, quote: *q, orig: t.clone() }, text_to_value(t), true),
                None => Value::String(String::new()),
            };
            map.insert(key, v);
        }
        if let Some((s, en, cdata, Some(t))) = meaningful.first().map(|t| (*t).clone()) {
            let p = child(ptr, "#text");
            let v = self.leaf(&p, Leaf::Text { start: s, end: en, cdata, orig: t.clone() }, if cdata { Value::String(t) } else { text_to_value(&t) }, true);
            map.insert("#text".into(), v);
        }
        // Group children by name, keeping first-appearance order.
        let mut order: Vec<&str> = Vec::new();
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for c in &e.children {
            let n = counts.entry(&c.name).or_insert(0);
            if *n == 0 {
                order.push(&c.name);
            }
            *n += 1;
        }
        for name in order {
            let p = child(ptr, name);
            if counts[name] == 1 && !self.lists.contains(&name) {
                let c = e.children.iter().find(|c| c.name == name).unwrap();
                let v = self.view(c, &p);
                map.insert(name.to_string(), v);
            } else {
                let items: Vec<Value> =
                    e.children.iter().filter(|c| c.name == name).enumerate().map(|(i, c)| self.view(c, &format!("{p}/{i}"))).collect();
                map.insert(name.to_string(), Value::Array(items));
            }
        }
        Value::Object(map)
    }
}

#[derive(Clone, Debug)]
struct XmlBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for XmlBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            match &self.leaves[*id] {
                Leaf::Text { start, end, cdata, orig } => {
                    let t = value_to_text(orig, v);
                    p.add(*start, *end, if *cdata { t } else { escape(&t, None) }.into_bytes());
                }
                Leaf::Attr { start, end, quote, orig } => {
                    p.add(*start, *end, escape(&value_to_text(orig, v), Some(*quote)).into_bytes());
                }
                Leaf::Empty { start, end, name } => {
                    let t = value_to_text("", v);
                    if !t.is_empty() {
                        p.add(*start, *end, format!("<{name}>{}</{name}>", escape(&t, None)).into_bytes());
                    }
                }
            }
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        if let (Leaf::Text { cdata: true, .. }, Some(s)) = (&self.leaves[leaf], v.as_str()) {
            if s.contains("]]>") {
                return Err("CDATA text can't contain \"]]>\"".into());
            }
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    parse_with(text, text.as_bytes(), &[])
}

/// Reads `text` (the file's text, possibly with a few characters swapped for same-length ones
/// so it reads as XML) but writes edits into `raw`, the file as it was; `lists` names the
/// elements to show as lists even when they occur once.
pub fn parse_with(text: &str, raw: &[u8], lists: &'static [&'static str]) -> Result<TreeDoc, String> {
    let root = parse_root(text)?;
    let mut b = Builder { leaves: vec![], editable: HashMap::new(), lists };
    let mut map = Map::new();
    let ptr = child("", &root.name);
    let v = b.view(&root, &ptr);
    map.insert(root.name.clone(), v);
    let backend = XmlBackend { raw: raw.to_vec(), leaves: b.leaves };
    Ok(TreeDoc::new(Value::Object(map), b.editable, "XML", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!-- Stardew-like save -->
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <player>
    <name>Robin &amp; Co</name>
    <money>1500</money>
    <isMale>true</isMale>
    <stamina>270.5</stamina>
    <farmName />
    <items><Item q="1">Parsnip</Item><Item q="5">Hoe</Item></items>
  </player>
  <notes><![CDATA[a < b]]></notes>
</SaveGame>
"#;

    fn edit(doc: &mut TreeDoc, ptr: &str, v: Value) {
        doc.set(ptr, v).unwrap().expect("pointer exists");
    }

    #[test]
    fn unchanged_round_trip_is_exact() {
        let doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        assert_eq!(doc.view.pointer("/SaveGame/player/money").unwrap(), 1500);
        assert_eq!(doc.view.pointer("/SaveGame/player/name").unwrap(), "Robin & Co");
        assert_eq!(doc.view.pointer("/SaveGame/player/items/Item/1/@q").unwrap(), 5);
    }

    #[test]
    fn edits_only_touch_values() {
        let mut doc = parse(SAMPLE).unwrap();
        edit(&mut doc, "/SaveGame/player/money", Value::from(99999));
        edit(&mut doc, "/SaveGame/player/name", Value::from("Ann <3"));
        edit(&mut doc, "/SaveGame/player/isMale", Value::Bool(false));
        edit(&mut doc, "/SaveGame/player/farmName", Value::from("Sunny"));
        edit(&mut doc, "/SaveGame/player/items/Item/1/@q", Value::from(9));
        edit(&mut doc, "/SaveGame/player/items/Item/0/#text", Value::from("Melon"));
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        let expected = SAMPLE
            .replace("<money>1500</money>", "<money>99999</money>")
            .replace("Robin &amp; Co", "Ann &lt;3")
            .replace("<isMale>true</isMale>", "<isMale>false</isMale>")
            .replace("<farmName />", "<farmName>Sunny</farmName>")
            .replace("q=\"5\">Hoe", "q=\"9\">Hoe")
            .replace(">Parsnip<", ">Melon<");
        assert_eq!(out, expected);
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/SaveGame/player/name").unwrap(), "Ann <3");
    }

    #[test]
    fn rejects_broken_xml() {
        assert!(parse("<a><b></a>").is_err());
        assert!(parse("<a>").is_err());
        assert!(parse("not xml").is_err());
    }
}
