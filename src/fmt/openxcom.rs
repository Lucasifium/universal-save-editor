//! OpenXcom and OpenXcom Extended saves (`*.sav`): two YAML documents, a short header and the
//! game. The header is kept byte for byte; the game document is read as a tree and written
//! again as YAML on save. Layout follows the OpenXcom Save Editor by aaronjacobs-chelt (MIT).

use crate::tree::{Backend, TreeDoc};
use serde_json::Value;
use std::collections::HashMap;

pub const KIND: &str = "OpenXcom save";

#[derive(Clone, Debug)]
struct Oxc {
    header: String,
    raw: Vec<u8>,
}

impl Backend for Oxc {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let body = serde_yaml::to_string(&to_yaml(view)).map_err(|e| e.to_string())?;
        Ok(format!("{}---\n{}", self.header, body).into_bytes())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// The tree as YAML values (numbers are written as plain numbers).
fn to_yaml(v: &Value) -> serde_yaml::Value {
    use serde_yaml::Value as Y;
    match v {
        Value::Null => Y::Null,
        Value::Bool(b) => Y::Bool(*b),
        Value::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) => Y::Number(i.into()),
            (None, Some(u)) => Y::Number(u.into()),
            _ => Y::Number(n.as_f64().unwrap_or(0.0).into()),
        },
        Value::String(s) => Y::String(s.clone()),
        Value::Array(a) => Y::Sequence(a.iter().map(to_yaml).collect()),
        Value::Object(m) => Y::Mapping(m.iter().map(|(k, c)| (Y::String(k.clone()), to_yaml(c))).collect()),
    }
}

fn register(v: &Value, ptr: &str, editable: &mut HashMap<String, usize>) {
    match v {
        Value::Object(m) => m.iter().for_each(|(k, c)| register(c, &crate::tree::child(ptr, k), editable)),
        Value::Array(a) => a.iter().enumerate().for_each(|(i, c)| register(c, &format!("{ptr}/{i}"), editable)),
        Value::Null => {}
        _ => {
            let n = editable.len();
            editable.insert(ptr.to_string(), n);
        }
    }
}

pub fn looks(text: &str) -> bool {
    text.starts_with("name:") && text.contains("\n---\n") && text.contains("\nfunds:") && text.contains("\nbases:") && text[..text.len().min(400)].contains("\nengine:")
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not an OpenXcom save".into());
    }
    let at = text.find("\n---\n").ok_or("no game document")? + 1;
    let (header, body) = (&text[..at], &text[at + 4..]);
    let view: Value = serde_yaml::from_str(body).map_err(|e| e.to_string())?;
    if view.get("funds").is_none() {
        return Err("not an OpenXcom save".into());
    }
    let mut editable = HashMap::new();
    register(&view, "", &mut editable);
    Ok(TreeDoc::new(view, editable, KIND, Box::new(Oxc { header: header.to_string(), raw: text.as_bytes().to_vec() }))
        .with_note("Close the game first. Saving writes the game document out again as YAML (the header is kept as it is); funds[0] is this month's funds."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_funds_and_keeps_header() {
        let t = "name: SaveGame\nversion: Extended 8\nengine: Extended\n---\ndifficulty: 1\nfunds:\n- 100\n- 50\nbases:\n- name: B\n  soldiers: []\n";
        let mut doc = parse(t).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), t.as_bytes());
        doc.set("/funds/0", 9_999_999.into()).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.starts_with("name: SaveGame\nversion: Extended 8\nengine: Extended\n---\n"));
        assert!(parse(&out).unwrap().view.pointer("/funds/0").unwrap() == 9_999_999);
    }
}
