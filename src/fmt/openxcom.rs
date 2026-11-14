//! OpenXcom / OpenXcom Extended saves (`.sav`): YAML text in two documents, a header
//! (`name: SaveGame`, `version`, `engine`, `time`, `mods`) and the game data after a `---` line.
//!
//! The game's data is shown with two changes to make it easy to edit, undone on saving: `funds`
//! (one entry per month, the last is the money you have now) becomes `Current` and
//! `Earlier months`, and every base's soldiers are gathered into one `Soldiers` list (each with
//! the name of its base). Saving writes the header back unchanged and rebuilds the data
//! document, so comments and spacing in it are not kept.
//!
//! The layout (funds, soldier stats, base stores) is the game's own and is documented by the
//! OpenXcom project (GPL-3.0) and by aaronjacobs-chelt/openxcom-save-editor (MIT); see
//! THIRD_PARTY_NOTICES.md.

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "OpenXcom save";

pub fn looks(text: &str) -> bool {
    let t = text.trim_start_matches('\u{feff}');
    t.starts_with("name: SaveGame") && t.contains("\n---\n") && t.contains("\nfunds:") && t.contains("\nbases:")
}

#[derive(Clone, Debug)]
struct Backing {
    header: String,
    raw: Vec<u8>,
}

impl Backend for Backing {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let data = restore(view)?;
        let body = serde_yaml::to_string(&super::yaml::to_yaml(&data)).map_err(|e| e.to_string())?;
        Ok(format!("{}---\n{}", self.header, body).into_bytes())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn base_name(b: &Value) -> String {
    b.get("name").and_then(Value::as_str).unwrap_or("").to_string()
}

/// The data document as shown: `Current` funds and one flat list of soldiers.
fn present(mut data: Value) -> Value {
    let Some(root) = data.as_object_mut() else { return data };
    if let Some(Value::Array(f)) = root.get("funds").cloned() {
        if let Some((last, rest)) = f.split_last() {
            let mut m = Map::new();
            m.insert("Current".into(), last.clone());
            m.insert("Earlier months".into(), Value::Array(rest.to_vec()));
            root.insert("funds".into(), Value::Object(m));
        }
    }
    let mut flat = Vec::new();
    if let Some(Value::Array(bases)) = root.get_mut("bases") {
        for b in bases.iter_mut() {
            let name = base_name(b);
            if let Some(Value::Array(s)) = b.as_object_mut().and_then(|o| o.remove("soldiers")) {
                for mut sol in s {
                    if let Some(o) = sol.as_object_mut() {
                        o.insert("base".into(), Value::String(name.clone()));
                    }
                    flat.push(sol);
                }
            }
        }
    }
    root.insert("Soldiers".into(), Value::Array(flat));
    data
}

fn restore(view: &Value) -> Result<Value, String> {
    let mut data = view.clone();
    let root = data.as_object_mut().ok_or("not an OpenXcom save")?;
    if let Some(Value::Object(m)) = root.get("funds").cloned() {
        let mut f: Vec<Value> = match m.get("Earlier months") {
            Some(Value::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        f.push(m.get("Current").cloned().unwrap_or(Value::from(0)));
        root.insert("funds".into(), Value::Array(f));
    }
    let Some(Value::Array(flat)) = root.remove("Soldiers") else { return Ok(data) };
    // The soldiers go back to the base whose name they carry (the order inside a base is kept).
    let mut rest: Vec<Value> = flat;
    if let Some(Value::Array(bases)) = root.get_mut("bases") {
        for b in bases.iter_mut() {
            let name = base_name(b);
            let (mine, other): (Vec<Value>, Vec<Value>) = rest
                .into_iter()
                .partition(|s| s.get("base").and_then(Value::as_str) == Some(name.as_str()));
            rest = other;
            let mut mine = mine;
            for s in mine.iter_mut() {
                if let Some(o) = s.as_object_mut() {
                    o.remove("base");
                }
            }
            if !mine.is_empty() {
                if let Some(o) = b.as_object_mut() {
                    o.insert("soldiers".into(), Value::Array(mine));
                }
            }
        }
    }
    Ok(data)
}

fn register(v: &Value, ptr: &str, editable: &mut HashMap<String, usize>) {
    match v {
        Value::Object(m) => {
            for (k, c) in m {
                register(c, &crate::tree::child(ptr, k), editable);
            }
        }
        Value::Array(a) => {
            for (i, c) in a.iter().enumerate() {
                register(c, &format!("{ptr}/{i}"), editable);
            }
        }
        Value::Null => {}
        _ => {
            let n = editable.len();
            editable.insert(ptr.to_string(), n);
        }
    }
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not an OpenXcom save".into());
    }
    let t = text.trim_start_matches('\u{feff}');
    let at = t.find("\n---\n").ok_or("no data document")? + 1;
    let header = t[..at].to_string();
    let data: Value = serde_yaml::from_str(&t[at + 4..]).map_err(|e| e.to_string())?;
    if !data.is_object() {
        return Err("not an OpenXcom save".into());
    }
    let view = present(data);
    let mut editable = HashMap::new();
    register(&view, "", &mut editable);
    Ok(TreeDoc::new(view, editable, KIND, Box::new(Backing { header, raw: text.as_bytes().to_vec() })).with_note(
        "Close OpenXcom first. Saving rewrites the data part of the file, so its comments and spacing are not kept. \
         The money is Funds > Current; the soldiers of every base are in one list, each with the name of its base.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAVE: &str = "name: SaveGame\nversion: 1.0\nengine: x\n---\nfunds:\n- 10\n- 20\nbases:\n- name: A\n  soldiers:\n  - id: 1\n    name: S1\n- name: B\n  soldiers:\n  - id: 2\n    name: S2\n  - id: 3\n    name: S3\n";

    #[test]
    fn edit_round_trip() {
        assert!(looks(SAVE));
        let mut doc = parse(SAVE).unwrap();
        assert_eq!(doc.view.pointer("/funds/Current").and_then(Value::as_i64), Some(20));
        assert_eq!(doc.view.pointer("/Soldiers/2/base").and_then(Value::as_str), Some("B"));
        assert_eq!(doc.to_bytes().unwrap(), SAVE.as_bytes());
        doc.set("/funds/Current", Value::from(99)).unwrap();
        doc.set("/Soldiers/1/name", Value::from("Renamed")).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.starts_with("name: SaveGame\nversion: 1.0\nengine: x\n---\n"));
        let d: Value = serde_yaml::from_str(out.split("---\n").nth(1).unwrap()).unwrap();
        assert_eq!(d["funds"], serde_json::json!([10, 99]));
        assert_eq!(d["bases"][1]["soldiers"][0]["name"], "Renamed");
        assert_eq!(d["bases"][0]["soldiers"].as_array().unwrap().len(), 1);
        assert!(d.get("Soldiers").is_none());
    }
}
