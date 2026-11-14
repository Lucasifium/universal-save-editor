//! Paradox (Clausewitz / Jomini) text saves: Stellaris (`gamestate` inside the .sav zip),
//! Hearts of Iron IV, Europa Universalis IV, Crusader Kings II/III and Victoria text saves.
//!
//! The text is `key=value` pairs and `{ … }` blocks, which can also hold plain lists; keys
//! repeat freely. Values are edited by replacing their text in place, so everything else
//! in the file stays byte for byte. These files are huge (tens of MB), so each top-level
//! section shows at most `SECTION_BUDGET` values; the player's own country is always shown
//! in full. Binary (ironman) saves are read by `pdxbin`.

use crate::tree::{esc, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

const SECTION_BUDGET: usize = 6_000;
const FOCUS_BUDGET: usize = 400_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tok {
    Open,
    Close,
    Op,
    Quoted,
    Word,
}

#[derive(Clone, Copy, Debug)]
struct Token {
    kind: Tok,
    start: usize,
    end: usize,
}

fn tokenize(d: &[u8]) -> Vec<Token> {
    let mut out = Vec::with_capacity(d.len() / 6);
    let mut i = 0;
    while i < d.len() {
        let c = d[i];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' | b';' => i += 1,
            b'#' => {
                while i < d.len() && d[i] != b'\n' {
                    i += 1;
                }
            }
            b'{' => {
                out.push(Token { kind: Tok::Open, start: i, end: i + 1 });
                i += 1;
            }
            b'}' => {
                out.push(Token { kind: Tok::Close, start: i, end: i + 1 });
                i += 1;
            }
            b'=' | b'<' | b'>' | b'!' | b'?' => {
                let s = i;
                i += 1;
                if i < d.len() && d[i] == b'=' {
                    i += 1;
                }
                out.push(Token { kind: Tok::Op, start: s, end: i });
            }
            b'"' => {
                let s = i;
                i += 1;
                while i < d.len() && d[i] != b'"' {
                    if d[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i = (i + 1).min(d.len());
                out.push(Token { kind: Tok::Quoted, start: s, end: i });
            }
            _ => {
                let s = i;
                while i < d.len() && !matches!(d[i], b' ' | b'\t' | b'\r' | b'\n' | b'{' | b'}' | b'=' | b'"' | b'<' | b'>' | b'#' | b';') {
                    i += 1;
                }
                out.push(Token { kind: Tok::Word, start: s, end: i });
            }
        }
    }
    out
}

/// One editable scalar: where its text is and how to write it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Int,
    Float { decimals: usize },
    Bool,
    Quoted,
    Word,
}

#[derive(Clone, Copy, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    kind: Kind,
}

struct Builder<'a> {
    d: &'a [u8],
    toks: Vec<Token>,
    pos: usize,
    utf8: bool,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
    /// Paths always shown in full (the player's country).
    focus: Vec<String>,
    budget: usize,
    /// Pointer of each leaf (same order as `leaves`).
    leaf_ptrs: Vec<String>,
}

fn decode(b: &[u8], utf8: bool) -> String {
    if utf8 {
        String::from_utf8_lossy(b).into_owned()
    } else {
        b.iter().map(|&c| super::bethesda_cp1252(c)).collect()
    }
}

impl Builder<'_> {
    fn text(&self, t: Token) -> String {
        decode(&self.d[t.start..t.end], self.utf8)
    }

    fn scalar(&mut self, t: Token, ptr: &str) -> Value {
        let raw = self.text(t);
        let (v, kind) = match t.kind {
            Tok::Quoted => {
                let inner = raw.get(1..raw.len().saturating_sub(1)).unwrap_or("").replace("\\\"", "\"");
                (Value::from(inner), Kind::Quoted)
            }
            _ if raw == "yes" || raw == "no" => (Value::Bool(raw == "yes"), Kind::Bool),
            _ => {
                let numeric = !raw.is_empty() && raw.bytes().enumerate().all(|(i, c)| c.is_ascii_digit() || (c == b'-' && i == 0) || c == b'.') && raw.bytes().filter(|&c| c == b'.').count() <= 1 && raw != "-" && raw != ".";
                if numeric && !raw.contains('.') {
                    match raw.parse::<i64>() {
                        Ok(n) => (Value::from(n), Kind::Int),
                        Err(_) => (Value::from(raw.clone()), Kind::Word),
                    }
                } else if numeric {
                    let decimals = raw.len() - raw.find('.').unwrap() - 1;
                    match raw.parse::<f64>().ok().and_then(serde_json::Number::from_f64) {
                        Some(n) => (Value::Number(n), Kind::Float { decimals }),
                        None => (Value::from(raw.clone()), Kind::Word),
                    }
                } else {
                    (Value::from(raw.clone()), Kind::Word)
                }
            }
        };
        self.editable.insert(ptr.to_string(), self.leaves.len());
        self.leaf_ptrs.push(ptr.to_string());
        self.leaves.push(Leaf { start: t.start, end: t.end, kind });
        v
    }

    fn peek(&self, k: usize) -> Option<Token> {
        self.toks.get(self.pos + k).copied()
    }

    /// Skips a block whose `{` was just taken; returns how many values it held.
    fn skip_block(&mut self) -> usize {
        let mut depth = 1;
        let mut n = 0;
        while let Some(t) = self.peek(0) {
            self.pos += 1;
            match t.kind {
                Tok::Open => depth += 1,
                Tok::Close => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Tok::Word | Tok::Quoted => n += 1,
                Tok::Op => {}
            }
        }
        n
    }

    /// A value after `key=` (or a list item): a scalar, or a block (possibly tagged, as in
    /// `rgb { 1 2 3 }`).
    fn value(&mut self, ptr: &str) -> Value {
        let Some(t) = self.peek(0) else { return Value::Null };
        match t.kind {
            Tok::Open => {
                self.pos += 1;
                self.block(ptr, false)
            }
            Tok::Word if self.peek(1).is_some_and(|n| n.kind == Tok::Open) && !self.peek(2).is_some_and(|n| n.kind == Tok::Op) => {
                // A tagged block: rgb { … }, hsv { … }, LIST { … }.
                self.pos += 2;
                self.block(ptr, false)
            }
            Tok::Close | Tok::Op => {
                self.pos += 1;
                Value::Null
            }
            _ => {
                self.pos += 1;
                self.scalar(t, ptr)
            }
        }
    }

    fn focused(&self, ptr: &str) -> bool {
        self.focus.iter().any(|f| ptr == f || ptr.starts_with(&format!("{f}/")) || f.starts_with(&format!("{ptr}/")))
    }

    /// The items of a block (its `{` already taken), up to the closing `}` (or the end of
    /// the file for the top level).
    fn block(&mut self, ptr: &str, top: bool) -> Value {
        let mut kv = Map::new();
        let mut list: Vec<Value> = vec![];
        let mut mixed = false;
        let mut hidden = 0usize;
        // First leaf of each plain list item (to re-point them if the block turns out mixed).
        let mut item_leaves: Vec<usize> = vec![];
        loop {
            let Some(t) = self.peek(0) else { break };
            if t.kind == Tok::Close {
                self.pos += 1;
                if top {
                    continue;
                }
                break;
            }
            let keyed = matches!(t.kind, Tok::Word | Tok::Quoted) && self.peek(1).is_some_and(|n| n.kind == Tok::Op);
            if top {
                self.budget = SECTION_BUDGET;
            }
            if keyed {
                let raw = self.text(t);
                let name = raw.trim_matches('"').to_string();
                let key = unique_key(&kv, &name);
                let p = format!("{ptr}/{}", esc(&key));
                self.pos += 2;
                let focus = self.focused(&p);
                if !focus && self.budget == 0 {
                    hidden += 1;
                    if self.peek(0).is_some_and(|n| n.kind == Tok::Open) {
                        self.pos += 1;
                        hidden += self.skip_block();
                    } else {
                        self.pos += 1;
                    }
                    continue;
                }
                let saved = self.budget;
                if focus {
                    self.budget = FOCUS_BUDGET;
                }
                self.budget = self.budget.saturating_sub(1);
                let v = self.value(&p);
                if focus {
                    self.budget = saved;
                }
                kv.insert(key, v);
            } else {
                mixed |= !kv.is_empty();
                let p = format!("{ptr}/{}", list.len());
                if self.budget == 0 && !self.focused(&p) {
                    hidden += 1;
                    if t.kind == Tok::Open {
                        self.pos += 1;
                        hidden += self.skip_block();
                    } else {
                        self.pos += 1;
                    }
                    item_leaves.push(self.leaves.len());
                    list.push(Value::Null);
                    continue;
                }
                self.budget = self.budget.saturating_sub(1);
                item_leaves.push(self.leaves.len());
                let v = self.value(&p);
                list.push(v);
            }
        }
        if hidden > 0 {
            let key = unique_key(&kv, "(not shown)");
            kv.insert(key, Value::from(format!("{hidden} more values \u{2014} see the file itself")));
        }
        if kv.is_empty() {
            return Value::Array(list);
        }
        if !list.is_empty() {
            // Keys and plain values together: keep the plain ones under numbered keys.
            let _ = mixed;
            let end = self.leaves.len();
            for (i, v) in list.into_iter().enumerate() {
                let key = unique_key(&kv, &format!("#{}", i + 1));
                // The item's values were recorded under "{ptr}/{i}"; move them to the key shown.
                let (from, to) = (format!("{ptr}/{i}"), format!("{ptr}/{}", esc(&key)));
                let last = item_leaves.get(i + 1).copied().unwrap_or(end);
                for leaf in item_leaves[i]..last {
                    let old = &self.leaf_ptrs[leaf];
                    if old == &from || old.starts_with(&format!("{from}/")) {
                        let new = format!("{to}{}", &old[from.len()..]);
                        self.editable.remove(old);
                        self.editable.insert(new.clone(), leaf);
                        self.leaf_ptrs[leaf] = new;
                    }
                }
                kv.insert(key, v);
            }
        }
        Value::Object(kv)
    }
}

/// Text saves this module reads, by their first bytes (and what the file is).
fn magic(d: &[u8]) -> Option<(&'static str, usize)> {
    for (m, name) in [(&b"HOI4txt"[..], "Hearts of Iron IV save"), (b"EU4txt", "Europa Universalis IV save"), (b"CK2txt", "Crusader Kings II save"), (b"CK3txt", "Crusader Kings III save")] {
        if d.starts_with(m) {
            return Some((name, m.len()));
        }
    }
    // The newer games' plain text saves (debug mode): a `SAV…` header line, then the text.
    if let Some(h) = super::pdxbin::SavHeader::read(d).filter(|h| h.kind == 0) {
        return Some(("Paradox save", h.len));
    }
    None
}

/// Binary (ironman) saves by their first bytes, to name the game when one can't be read.
pub fn binary_kind(d: &[u8]) -> Option<&'static str> {
    if d.starts_with(b"HOI4bin") {
        return Some("Hearts of Iron IV");
    }
    if d.starts_with(b"EU4bin") {
        return Some("Europa Universalis IV");
    }
    if d.len() > 24 && d.starts_with(b"SAV") && d[3..23].iter().all(u8::is_ascii_hexdigit) && matches!(&d[5..7], b"01" | b"03" | b"05") {
        return Some("Paradox (Crusader Kings III / Europa Universalis V / Victoria 3)");
    }
    None
}

/// Stellaris' gamestate and meta, and other headerless Paradox text: `version="…"` first,
/// with a date near the top.
/// Europa Universalis III text saves: no header, `date="…"` then `player="TAG"` first.
pub fn is_eu3(d: &[u8]) -> bool {
    let head = &d[..d.len().min(2048)];
    head.starts_with(b"date=\"") && head.windows(8).any(|w| w == b"\nplayer=") && head.windows(9).any(|w| w == b"\nmonarch=" || w == b"\ncardinal") || (head.starts_with(b"date=\"") && head.windows(8).any(|w| w == b"\nplayer=") && head.windows(9).any(|w| w == b"\npapacy={"))
}

pub fn looks(d: &[u8]) -> bool {
    if magic(d).is_some() || is_eu3(d) {
        return true;
    }
    let head = &d[..d.len().min(4096)];
    head.starts_with(b"version=\"") && (head.windows(6).any(|w| w == b"\ndate=") || head.windows(6).any(|w| w == b"\nname="))
}

#[derive(Clone, Debug)]
struct PdxBackend {
    data: Arc<Vec<u8>>,
    utf8: bool,
    leaves: Vec<Leaf>,
    /// A `SAV…` text file: where its header keeps the metadata length (8 hex digits) and
    /// where the metadata ends, so the length follows an edit inside it.
    meta: Option<(usize, usize)>,
}

impl PdxBackend {
    fn encode(&self, leaf: &Leaf, v: &Value) -> Result<Vec<u8>, String> {
        let text = match leaf.kind {
            Kind::Int => crate::tree::int_in(v, i64::MIN as i128, i64::MAX as i128, "This value")?.to_string(),
            Kind::Float { decimals } => {
                let f = v.as_f64().filter(|f| f.is_finite()).ok_or("expected a number")?;
                format!("{f:.prec$}", prec = decimals.max(1))
            }
            Kind::Bool => (if v.as_bool().ok_or("tick or untick it")? { "yes" } else { "no" }).to_string(),
            Kind::Quoted => format!("\"{}\"", v.as_str().ok_or("expected text")?.replace('"', "\\\"")),
            Kind::Word => {
                let s = match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => return Err("expected a word".into()),
                };
                if s.is_empty() || s.bytes().any(|c| c.is_ascii_whitespace() || b"{}=\"#".contains(&c)) {
                    return Err("this value can't contain spaces, quotes or braces".into());
                }
                s
            }
        };
        Ok(if self.utf8 {
            text.into_bytes()
        } else {
            text.chars().map(|c| super::bethesda_cp1252_encode(c).unwrap_or(b'?')).collect()
        })
    }
}

impl Backend for PdxBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.data.as_ref().clone());
        }
        let mut edits: Vec<(Leaf, Vec<u8>)> = changes.iter().map(|(l, v)| Ok((self.leaves[*l], self.encode(&self.leaves[*l], v)?))).collect::<Result<_, String>>()?;
        edits.sort_by_key(|e| e.0.start);
        let mut out = Vec::with_capacity(self.data.len() + 64);
        let (mut at, mut meta_delta) = (0, 0i64);
        for (leaf, bytes) in edits {
            if self.meta.is_some_and(|(_, end)| leaf.start < end) {
                meta_delta += bytes.len() as i64 - (leaf.end - leaf.start) as i64;
            }
            out.extend_from_slice(&self.data[at..leaf.start]);
            out.extend(bytes);
            at = leaf.end;
        }
        out.extend_from_slice(&self.data[at..]);
        if let (Some((field, _)), true) = (self.meta, meta_delta != 0) {
            let old = std::str::from_utf8(&out[field..field + 8]).ok().and_then(|h| i64::from_str_radix(h, 16).ok()).ok_or("the save's header can't be read")?;
            out[field..field + 8].copy_from_slice(format!("{:08x}", old + meta_delta).as_bytes());
        }
        Ok(out)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        // Show what will be written (floats keep the file's number of decimals).
        match (self.leaves[leaf].kind, v.as_f64()) {
            (Kind::Float { decimals }, Some(f)) => format!("{f:.prec$}", prec = decimals.max(1)).parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(v, Value::Number),
            _ => v,
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        self.encode(&self.leaves[leaf], v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// The player's country, as paths to show in full: Stellaris `player={ { country=N } }`
/// → /country/N; Hearts of Iron IV / EU4 `player="TAG"` → /countries/TAG.
fn focus_paths(d: &[u8], utf8: bool) -> Vec<String> {
    let head = decode(&d[..d.len().min(64 * 1024)], utf8);
    let mut out = vec![];
    if let Some(i) = head.find("\nplayer=") {
        let rest = &head[i + 8..];
        let rest = rest.trim_start();
        if let Some(tag) = rest.strip_prefix('"').and_then(|r| r.split('"').next()) {
            out.push(if is_eu3(d) { format!("/{tag}") } else { format!("/countries/{tag}") });
        } else if let Some(c) = rest.find("country=") {
            let id: String = rest[c + 8..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if !id.is_empty() {
                out.push(format!("/country/{id}"));
            }
        }
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (name, skip) = magic(d).unwrap_or((if is_eu3(d) { "Europa Universalis III save" } else { "Paradox save" }, 0));
    let utf8 = std::str::from_utf8(d).is_ok();
    let toks = tokenize(&d[skip..]).into_iter().map(|t| Token { start: t.start + skip, end: t.end + skip, ..t }).collect();
    let mut b = Builder { d, toks, pos: 0, utf8, leaves: vec![], editable: HashMap::new(), focus: focus_paths(d, utf8), budget: SECTION_BUDGET, leaf_ptrs: vec![] };
    let view = b.block("", true);
    if view.as_object().is_none_or(|m| m.len() < 2) {
        return Err("not a Paradox save".into());
    }
    let kind = if view.pointer("/player_countries").is_some() || name.starts_with("Hearts") {
        "Hearts of Iron IV save"
    } else if view.pointer("/species_db").is_some() || view.pointer("/galaxy").is_some() || view.pointer("/required_dlcs").is_some() && view.pointer("/version").and_then(Value::as_str).is_some_and(|v| !v.contains('.')) {
        "Stellaris save"
    } else {
        name
    };
    let meta = super::pdxbin::SavHeader::read(d).filter(|h| h.kind == 0 && h.meta_len > 0).map(|h| (15, h.len + h.meta_len));
    let backend = PdxBackend { data: Arc::new(d.to_vec()), utf8, leaves: b.leaves, meta };
    Ok(TreeDoc::new(view, b.editable, kind, Box::new(backend)).with_note(
        "Big sections show their first few thousand values; your own country is shown in full. \
         The game may stop awarding achievements for an edited ironman campaign.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "version=\"Test v1\"\ndate=\"2206.04.01\"\nplayer=\n{\n\t{\n\t\tname=\"Ash\"\n\t\tcountry=0\n\t}\n}\n\
        country=\n{\n\t0=\n\t{\n\t\tresources=\n\t\t{\n\t\t\tenergy=145000\n\t\t\tminerals=111590.1424\n\t\t}\n\t\tis_ai=no\n\t\tcolor=rgb { 10 20 30 }\n\t\tflags={ a b c }\n\t}\n}\n";

    #[test]
    fn reads_and_edits_in_place() {
        let d = SAMPLE.as_bytes();
        assert!(looks(d));
        let mut doc = parse(d).unwrap();
        assert_eq!(doc.kind_name, "Paradox save");
        assert_eq!(doc.view["date"], "2206.04.01");
        assert_eq!(doc.view["country"]["0"]["resources"]["energy"], 145000);
        assert_eq!(doc.view["country"]["0"]["resources"]["minerals"], 111590.1424);
        assert_eq!(doc.view["country"]["0"]["is_ai"], false);
        assert_eq!(doc.view["country"]["0"]["flags"][1], "b");
        assert_eq!(doc.view["player"][0]["name"], "Ash");
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/country/0/resources/energy", Value::from(999999)).unwrap();
        doc.set("/country/0/resources/minerals", Value::from(5.5)).unwrap();
        doc.set("/player/0/name", Value::from("Ada \"A\"")).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("energy=999999\n"));
        assert!(out.contains("minerals=5.5000\n"));
        assert!(out.contains("name=\"Ada \\\"A\\\"\""));
        let back = parse(out.as_bytes()).unwrap();
        assert_eq!(back.view, doc.view);
    }

    #[test]
    fn sav_text_header_follows_the_metadata() {
        let meta = "meta_data={\n\tname=\"Ada\"\n}\n";
        let d = format!("SAV0100aabbccdd{:08x}\n{meta}gold=5\n", meta.len());
        assert!(looks(d.as_bytes()));
        let mut doc = parse(d.as_bytes()).unwrap();
        assert_eq!(doc.view["gold"], 5);
        doc.set("/meta_data/name", Value::from("Adelaide")).unwrap();
        doc.set("/gold", Value::from(123456)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.starts_with(&format!("SAV0100aabbccdd{:08x}\n", meta.len() + 5)), "{out}");
        assert!(out.ends_with("gold=123456\n"));
    }

    #[test]
    fn focus_finds_the_player() {
        assert_eq!(focus_paths(SAMPLE.as_bytes(), true), vec!["/country/0"]);
        assert_eq!(focus_paths(b"HOI4txt\nplayer=\"FRA\"\n", true), vec!["/countries/FRA"]);
    }

    /// Research aid: parses PDX_SAVE (a text save, or a Stellaris gamestate) and prints
    /// the top-level keys and timings.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("PDX_SAVE").expect("PDX_SAVE")).unwrap();
        let t = std::time::Instant::now();
        let doc = parse(&d).unwrap();
        println!("{} in {:?}: {} editable, top {:?}", doc.kind_name, t.elapsed(), doc.editable.len(), doc.view.as_object().unwrap().keys().take(40).collect::<Vec<_>>());
        assert_eq!(doc.to_bytes().unwrap(), d);
        for p in std::env::var("PDX_SHOW").unwrap_or_default().split(',').filter(|p| !p.is_empty()) {
            let v = doc.view.pointer(p).cloned().unwrap_or_default();
            let s = v.to_string();
            println!("{p}: {}", &s[..s.len().min(1500)]);
        }
    }
}
