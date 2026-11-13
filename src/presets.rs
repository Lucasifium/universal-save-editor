//! Presets: a set of changes saved from one save and replayed on another (another slot,
//! another character, a friend's save of the same game). A preset is a small JSON file:
//!
//! ```json
//! {
//!   "preset": "Max gold and potions",
//!   "game": "Slay the Spire run",
//!   "values": [ { "path": "/gold", "name": "Gold", "value": 9999 } ],
//!   "bytes":  [ { "offset": "0x40", "name": "Gold", "hex": "0F 27 00 00" } ]
//! }
//! ```
//!
//! "values" are for saves the editor reads as a list of values (JSON pointers into the
//! Values/Tree data), "bytes" for saves shown as hex. A preset can hold either or both.

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct ValueChange {
    pub path: String,
    pub name: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ByteChange {
    pub offset: usize,
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Preset {
    pub name: String,
    pub game: String,
    pub values: Vec<ValueChange>,
    pub bytes: Vec<ByteChange>,
}

fn hex_text(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

fn parse_hex(s: &str) -> Option<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() || s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
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

impl Preset {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.bytes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.values.len() + self.bytes.len()
    }

    pub fn to_json(&self) -> String {
        let mut v = json!({ "preset": self.name, "game": self.game });
        if !self.values.is_empty() {
            v["values"] = self.values.iter().map(|c| json!({ "path": c.path, "name": c.name, "value": c.value })).collect();
        }
        if !self.bytes.is_empty() {
            v["bytes"] =
                self.bytes.iter().map(|c| json!({ "offset": format!("0x{:X}", c.offset), "name": c.name, "hex": hex_text(&c.bytes) })).collect();
        }
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }

    /// Reads a preset; the error says what's wrong with it.
    pub fn from_json(text: &str) -> Result<Preset, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
        let mut p = Preset {
            name: v["preset"].as_str().unwrap_or("Preset").trim().to_string(),
            game: v["game"].as_str().unwrap_or("").trim().to_string(),
            ..Default::default()
        };
        for (i, c) in v["values"].as_array().into_iter().flatten().enumerate() {
            let path = c["path"].as_str().ok_or_else(|| format!("value {} has no \"path\"", i + 1))?;
            if !path.is_empty() && !path.starts_with('/') {
                return Err(format!("value {}: \"path\" must start with /", i + 1));
            }
            let value = c.get("value").filter(|x| !x.is_object() && !x.is_array() && !x.is_null()).ok_or_else(|| format!("value {} needs a plain \"value\"", i + 1))?;
            p.values.push(ValueChange { path: path.to_string(), name: c["name"].as_str().unwrap_or(path).to_string(), value: value.clone() });
        }
        for (i, c) in v["bytes"].as_array().into_iter().flatten().enumerate() {
            let offset = offset(&c["offset"]).ok_or_else(|| format!("byte change {} needs an \"offset\"", i + 1))?;
            let bytes = c["hex"].as_str().and_then(parse_hex).ok_or_else(|| format!("byte change {} needs \"hex\" bytes, like \"0F 27\"", i + 1))?;
            let name = c["name"].as_str().map(String::from).unwrap_or_else(|| format!("Offset 0x{offset:X}"));
            p.bytes.push(ByteChange { offset, name, bytes });
        }
        if p.is_empty() {
            return Err("it has no changes in it".into());
        }
        Ok(p)
    }

    /// One line per change, for showing before it is applied.
    pub fn lines(&self) -> Vec<String> {
        let mut out: Vec<String> = self.values.iter().map(|c| format!("{}: {}", c.name, crate::widgets::display(&c.value))).collect();
        out.extend(self.bytes.iter().map(|c| format!("{} (0x{:X}): {}", c.name, c.offset, hex_text(&c.bytes))));
        out
    }
}

/// A file name for the preset made from its name: letters, digits, spaces and dashes.
pub fn file_name(name: &str) -> String {
    let clean: String = name.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
    let clean = clean.trim().trim_matches('_').to_string();
    format!("{}.json", if clean.is_empty() { "preset".into() } else { clean })
}

/// The folder presets are kept in.
#[cfg(not(target_arch = "wasm32"))]
pub fn dir() -> Option<std::path::PathBuf> {
    crate::platform::config_dir().map(|d| d.join("presets"))
}

/// Keeps a preset in the presets folder. Refuses to overwrite one with the same name unless
/// `replace`.
#[cfg(not(target_arch = "wasm32"))]
pub fn keep(p: &Preset, replace: bool) -> Result<std::path::PathBuf, String> {
    let d = dir().ok_or("no settings folder")?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let path = d.join(file_name(&p.name));
    if path.exists() && !replace {
        return Err(format!("a preset called \u{201C}{}\u{201D} already exists", p.name));
    }
    std::fs::write(&path, p.to_json()).map_err(|e| e.to_string())?;
    Ok(path)
}

/// The presets in the folder, by name, with their files.
#[cfg(not(target_arch = "wasm32"))]
pub fn list() -> Vec<(std::path::PathBuf, Preset)> {
    let Some(d) = dir() else { return vec![] };
    let Ok(rd) = std::fs::read_dir(d) else { return vec![] };
    let mut out: Vec<_> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")))
        .filter_map(|p| Some((p.clone(), Preset::from_json(&std::fs::read_to_string(&p).ok()?).ok()?)))
        .collect();
    out.sort_by_key(|(_, p)| p.name.to_lowercase());
    out
}

#[cfg(target_arch = "wasm32")]
pub fn list() -> Vec<(std::path::PathBuf, Preset)> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = Preset {
            name: "Rich".into(),
            game: "Test Quest".into(),
            values: vec![ValueChange { path: "/hero/gold".into(), name: "Gold".into(), value: json!(9999) }],
            bytes: vec![ByteChange { offset: 0x40, name: "Ammo".into(), bytes: vec![0x0F, 0x27, 0, 0] }],
        };
        let back = Preset::from_json(&p.to_json()).unwrap();
        assert_eq!(back, p);
        assert_eq!(back.len(), 2);
        assert!(back.lines()[1].contains("0x40"));
    }

    #[test]
    fn explains_mistakes() {
        assert!(Preset::from_json("{").unwrap_err().contains("JSON"));
        assert!(Preset::from_json(r#"{"preset":"x"}"#).unwrap_err().contains("no changes"));
        assert!(Preset::from_json(r#"{"values":[{"path":"gold","value":1}]}"#).unwrap_err().contains("/"));
        assert!(Preset::from_json(r#"{"values":[{"path":"/gold","value":{}}]}"#).unwrap_err().contains("plain"));
        assert!(Preset::from_json(r#"{"bytes":[{"offset":"0x10","hex":"ABC"}]}"#).unwrap_err().contains("hex"));
        let p = Preset::from_json(r#"{"bytes":[{"offset":16,"hex":"ab cd"}]}"#).unwrap();
        assert_eq!(p.bytes[0].bytes, vec![0xAB, 0xCD]);
        assert_eq!(p.bytes[0].name, "Offset 0x10");
    }

    #[test]
    fn safe_file_names() {
        assert_eq!(file_name("Max gold / items"), "Max gold _ items.json");
        assert_eq!(file_name("../.."), "preset.json");
        assert_eq!(file_name(""), "preset.json");
    }
}
