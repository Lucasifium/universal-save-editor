//! Templates the player writes for games the editor doesn't know: a JSON file in the
//! settings folder's `templates` folder that says how to recognise the game's saves and
//! where each value lives. A save that matches opens as a Tree of those values, edited in
//! place, with the checksum (if the template names one) written again on save.
//!
//! ```json
//! {
//!   "game": "My Game",
//!   "match": { "extensions": ["sav"], "size": 4096, "magic": [{ "at": 0, "hex": "53 41 56 45" }] },
//!   "big_endian": false,
//!   "checksum": { "algorithm": "CRC-32", "at": "0x08", "over": "after" },
//!   "fields": [
//!     { "path": "Player/Gold", "at": "0x40", "type": "u32", "min": 0, "max": 999999 },
//!     { "path": "Player/Difficulty", "at": 68, "type": "u8", "names": ["Easy", "Normal", "Hard"] }
//!   ]
//! }
//! ```

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use super::fields::{Builder, Checksum, Kind};
use super::sumfind::{Span, Sum, ALGOS};
use crate::tree::TreeDoc;
use serde_json::Value;

pub const README: &str = r#"Templates for games the editor doesn't know
============================================

Put one .json file per game in this folder. When a save matches a template it opens as a
list of the values the template names, and each one can be changed in place. Restart the
editor or open the save again after changing a template.

{
  "game": "My Game",
  "match": { "extensions": ["sav"], "size": 4096, "magic": [{ "at": 0, "hex": "53 41 56 45" }] },
  "big_endian": false,
  "checksum": { "algorithm": "CRC-32", "at": "0x08", "over": "after" },
  "fields": [
    { "path": "Player/Gold", "at": "0x40", "type": "u32", "min": 0, "max": 999999 },
    { "path": "Player/Difficulty", "at": 68, "type": "u8", "names": ["Easy", "Normal", "Hard"] },
    { "path": "Player/Gold (backup copy)", "at": "0x80", "type": "u32", "same_as": "Player/Gold" }
  ]
}

match: every part given must hold. "size" is the exact file size ("min_size" and "max_size"
  also work), "magic" lists bytes (in hex) that must be at an offset, and "extensions" lists
  file extensions. At least one of them is needed.
fields: "path" is where the value shows (parts separated by /), "at" the offset (a number or
  "0x…"), "type" one of u8, i16, u16, i32, u32, u64, f32, bool8, bool32. "min"/"max" limit
  what can be entered, "names" turns the value into a drop-down (0, 1, 2…), "big_endian"
  overrides the template's byte order, and "same_as" writes the value to a second place
  too (instead of showing it twice).
checksum (optional): "algorithm" is one of CRC-32, CRC-32C, Adler-32, Fletcher-32,
  FNV-1a (32-bit), sum of bytes (32-bit), sum of 32-bit words, CRC-16/CCITT, CRC-16/ARC,
  sum of bytes (16-bit), sum of bytes (8-bit), XOR of bytes, MD5, SHA-1, SHA-256 (the names
  Tools > Find checksum shows). "over" is "after" (the bytes after it), "before" (the bytes
  before it) or "whole" (the whole file with itself as zero). Tools > Find checksum can
  find these for you.
"#;

#[derive(Clone, Debug)]
struct Field {
    path: Vec<String>,
    at: usize,
    kind: Kind,
    big_endian: bool,
    range: Option<(i64, i64)>,
    names: Vec<String>,
    same_as: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct Template {
    pub game: String,
    size: (usize, usize),
    magic: Vec<(usize, Vec<u8>)>,
    extensions: Vec<String>,
    sum: Option<Sum>,
    fields: Vec<Field>,
}

fn offset(v: &Value) -> Option<usize> {
    match v {
        Value::Number(n) => n.as_u64().map(|n| n as usize),
        Value::String(s) => {
            let s = s.trim();
            match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                Some(h) => usize::from_str_radix(h, 16).ok(),
                None => s.parse().ok(),
            }
        }
        _ => None,
    }
}

fn hex(s: &str) -> Option<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() || s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

fn kind(s: &str) -> Option<Kind> {
    Some(match s.to_ascii_lowercase().as_str() {
        "u8" | "byte" => Kind::U8,
        "i16" => Kind::I16,
        "u16" => Kind::U16,
        "i32" | "int" => Kind::I32,
        "u32" => Kind::U32,
        "u64" => Kind::U64,
        "f32" | "float" => Kind::F32,
        "bool8" | "bool" => Kind::B8,
        "bool32" => Kind::B32,
        _ => return None,
    })
}

fn path(s: &str) -> Vec<String> {
    s.split('/').map(str::trim).filter(|p| !p.is_empty()).map(String::from).collect()
}

/// Reads one template; the error says what's wrong with it.
pub fn from_json(text: &str) -> Result<Template, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
    let game = v["game"].as_str().filter(|s| !s.trim().is_empty()).ok_or("\"game\" (the game's name) is missing")?.trim().to_string();
    let m = &v["match"];
    let mut size = (0, usize::MAX);
    if let Some(n) = m.get("size") {
        let n = offset(n).ok_or("\"size\" must be a number")?;
        size = (n, n);
    }
    if let Some(n) = m.get("min_size") {
        size.0 = offset(n).ok_or("\"min_size\" must be a number")?;
    }
    if let Some(n) = m.get("max_size") {
        size.1 = offset(n).ok_or("\"max_size\" must be a number")?;
    }
    let mut magic = Vec::new();
    for x in m["magic"].as_array().into_iter().flatten() {
        let at = offset(&x["at"]).ok_or("each \"magic\" needs an \"at\" offset")?;
        let bytes = x["hex"].as_str().and_then(hex).ok_or("each \"magic\" needs \"hex\" bytes, like \"53 41 56 45\"")?;
        magic.push((at, bytes));
    }
    let extensions: Vec<String> =
        m["extensions"].as_array().into_iter().flatten().filter_map(|e| e.as_str()).map(|e| e.trim_start_matches('.').to_lowercase()).collect();
    if size == (0, usize::MAX) && magic.is_empty() && extensions.is_empty() {
        return Err("\"match\" needs a size, magic bytes or extensions, so the template doesn't claim every file".into());
    }
    let be = v["big_endian"].as_bool().unwrap_or(false);
    let sum = match v.get("checksum") {
        None | Some(Value::Null) => None,
        Some(c) => {
            let name = c["algorithm"].as_str().ok_or("the checksum needs an \"algorithm\"")?;
            let algo = ALGOS.iter().copied().find(|a| a.name().eq_ignore_ascii_case(name)).ok_or_else(|| format!("unknown checksum algorithm \"{name}\""))?;
            let at = offset(&c["at"]).ok_or("the checksum needs an \"at\" offset")?;
            let span = match c["over"].as_str().unwrap_or("after") {
                "after" => Span::After,
                "before" => Span::Before,
                "whole" => Span::Zeroed,
                o => return Err(format!("the checksum's \"over\" is \"{o}\"; use after, before or whole")),
            };
            Some(Sum { algo, at, be: c["big_endian"].as_bool().unwrap_or(be), span })
        }
    };
    let mut fields = Vec::new();
    for (i, f) in v["fields"].as_array().ok_or("\"fields\" (the list of values) is missing")?.iter().enumerate() {
        let n = i + 1;
        let p = path(f["path"].as_str().ok_or_else(|| format!("field {n} has no \"path\""))?);
        if p.is_empty() {
            return Err(format!("field {n} has an empty \"path\""));
        }
        let at = offset(&f["at"]).ok_or_else(|| format!("field {n} needs an \"at\" offset"))?;
        let ty = f["type"].as_str().ok_or_else(|| format!("field {n} needs a \"type\""))?;
        let kind = kind(ty).ok_or_else(|| format!("field {n}: unknown type \"{ty}\""))?;
        let range = match (f["min"].as_i64(), f["max"].as_i64()) {
            (None, None) => None,
            (lo, hi) => Some((lo.unwrap_or(i64::MIN), hi.unwrap_or(i64::MAX))),
        };
        let names = f["names"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
        let same_as = f["same_as"].as_str().map(path);
        fields.push(Field { path: p, at, kind, big_endian: f["big_endian"].as_bool().unwrap_or(be), range, names, same_as });
    }
    if fields.is_empty() {
        return Err("\"fields\" is empty".into());
    }
    Ok(Template { game, size, magic, extensions, sum, fields })
}

impl Template {
    pub fn matches(&self, d: &[u8], ext: &str) -> bool {
        (self.size.0..=self.size.1).contains(&d.len())
            && self.magic.iter().all(|(at, b)| d.get(*at..at + b.len()) == Some(&b[..]))
            && (self.extensions.is_empty() || self.extensions.iter().any(|e| e == ext))
            && self.sum.is_none_or(|s| s.holds(d))
    }

    /// The checksum to write again on save, if the template has one.
    pub fn sum(&self) -> Option<Sum> {
        self.sum
    }

    pub fn parse(&self, d: &[u8], file: &str) -> TreeDoc {
        let mut b = Builder::new(d);
        for f in self.fields.iter().filter(|f| f.same_as.is_none()) {
            let p: Vec<&str> = f.path.iter().map(String::as_str).collect();
            b.big_endian = f.big_endian;
            if f.names.is_empty() {
                match f.range {
                    Some((lo, hi)) => b.ranged(&p, f.at, f.kind, lo, hi),
                    None => b.field(&p, f.at, f.kind),
                };
            } else {
                let names: Vec<&str> = f.names.iter().map(String::as_str).collect();
                b.named(&p, f.at, f.kind, &names);
            }
        }
        for f in &self.fields {
            if let Some(to) = &f.same_as {
                let p: Vec<&str> = to.iter().map(String::as_str).collect();
                b.mirror(&p, f.at);
            }
        }
        let mut note = format!("Opened with your template {file}: the values it names, changed in place.");
        if let Some(s) = self.sum {
            note.push_str(&format!(" Its checksum ({}) is written again on save.", s.describe()));
        }
        b.finish(&format!("{} save (your template)", self.game), Checksum::None).with_note(note)
    }
}

/// The folder templates are read from.
#[cfg(not(target_arch = "wasm32"))]
pub fn dir() -> Option<std::path::PathBuf> {
    crate::platform::config_dir().map(|d| d.join("templates"))
}

/// The folder, created with a short guide in it the first time.
#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_dir() -> Result<std::path::PathBuf, String> {
    let d = dir().ok_or("no settings folder")?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let guide = d.join("README.txt");
    if !guide.exists() {
        std::fs::write(&guide, README).map_err(|e| e.to_string())?;
    }
    Ok(d)
}

/// The first template that matches, with its file name.
#[cfg(not(target_arch = "wasm32"))]
pub fn find(d: &[u8], ext: &str) -> Option<(Template, String)> {
    let mut files: Vec<_> = std::fs::read_dir(dir()?)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")))
        .collect();
    files.sort();
    files.into_iter().find_map(|p| {
        let t = from_json(&std::fs::read_to_string(&p).ok()?).ok()?;
        t.matches(d, ext).then(|| (t, p.file_name().unwrap_or_default().to_string_lossy().into_owned()))
    })
}

#[cfg(target_arch = "wasm32")]
pub fn find(_d: &[u8], _ext: &str) -> Option<(Template, String)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: &str = r#"{
        "game": "Test Quest",
        "match": { "extensions": ["sav"], "size": 64, "magic": [{ "at": 0, "hex": "54 51" }] },
        "checksum": { "algorithm": "CRC-32", "at": "0x3C", "over": "before" },
        "fields": [
            { "path": "Hero/Gold", "at": "0x10", "type": "u32", "min": 0, "max": 99999 },
            { "path": "Hero/Mode", "at": 20, "type": "u8", "names": ["Easy", "Normal", "Hard"] },
            { "path": "Hero/HP", "at": "0x18", "type": "u16", "big_endian": true },
            { "path": "Gold copy", "at": "0x20", "type": "u32", "same_as": "Hero/Gold" }
        ]
    }"#;

    fn save() -> Vec<u8> {
        let mut d = vec![0u8; 64];
        d[..2].copy_from_slice(b"TQ");
        d[0x10..0x14].copy_from_slice(&500u32.to_le_bytes());
        d[0x20..0x24].copy_from_slice(&500u32.to_le_bytes());
        d[20] = 1;
        d[0x18..0x1A].copy_from_slice(&300u16.to_be_bytes());
        let s = from_json(T).unwrap().sum.unwrap();
        s.fix(&mut d).unwrap();
        d
    }

    #[test]
    fn reads_matches_and_edits() {
        let t = from_json(T).unwrap();
        let d = save();
        assert!(t.matches(&d, "sav"));
        assert!(!t.matches(&d, "dat"));
        assert!(!t.matches(&d[..63], "sav"));
        let mut doc = t.parse(&d, "test.json");
        assert_eq!(doc.view.pointer("/Hero/Gold").unwrap(), 500);
        assert_eq!(doc.view.pointer("/Hero/Mode").unwrap(), "Normal");
        assert_eq!(doc.view.pointer("/Hero/HP").unwrap(), 300);
        assert!(doc.view.pointer("/Gold copy").is_none());
        assert!(doc.set("/Hero/Gold", Value::from(100_000)).is_err());
        doc.set("/Hero/Gold", Value::from(77_777)).unwrap();
        doc.set("/Hero/Mode", Value::from("Hard")).unwrap();
        let mut out = doc.to_bytes().unwrap();
        assert_eq!(&out[0x20..0x24], &77_777u32.to_le_bytes());
        assert_eq!(out[20], 2);
        t.sum().unwrap().fix(&mut out).unwrap();
        assert!(t.matches(&out, "sav"));
    }

    #[test]
    fn explains_mistakes() {
        assert!(from_json("{").unwrap_err().contains("JSON"));
        assert!(from_json(r#"{"game":"x","fields":[]}"#).unwrap_err().contains("match"));
        let e = from_json(r#"{"game":"x","match":{"size":4},"fields":[{"path":"a","at":0,"type":"u24"}]}"#).unwrap_err();
        assert!(e.contains("u24"), "{e}");
        let e = from_json(r#"{"game":"x","match":{"size":4},"checksum":{"algorithm":"CRC-99","at":0},"fields":[{"path":"a","at":0,"type":"u8"}]}"#).unwrap_err();
        assert!(e.contains("CRC-99"), "{e}");
    }

    /// The guide's example is itself a valid template.
    #[test]
    fn guide_example_reads() {
        let start = README.find('{').unwrap();
        let end = README.find("\n}\n").unwrap() + 2;
        from_json(&README[start..end]).unwrap();
    }
}
