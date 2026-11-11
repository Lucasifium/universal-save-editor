//! INI / CFG style saves (`[Section]` + `key=value`), including Godot ConfigFile.
//! Only value spans are rewritten, so comments, ordering and line endings survive.

use crate::tree::{child, insert_unique, text_to_value, unique_key, value_to_text, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    quoted: bool,
    orig: String,
}

#[derive(Clone, Debug)]
struct IniBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for IniBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            let mut t = value_to_text(&l.orig, v);
            if l.quoted {
                t = t.replace('"', "\\\"");
            }
            p.add(l.start, l.end, t.into_bytes());
        }
        p.apply(&self.raw)
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.as_str().is_some_and(|s| s.contains(['\n', '\r'])) {
            return Err("INI values can't contain line breaks".into());
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

const INI_EXTS: &[&str] = &["ini", "cfg", "conf", "inf", "properties", "config", "settings"];

pub fn parse(text: &str, ext: &str) -> Result<TreeDoc, String> {
    let mut root = Map::new();
    let mut section: Option<String> = None;
    let mut leaves = Vec::new();
    let mut editable = HashMap::new();
    let (mut kv, mut sections, mut other) = (0, 0, 0);

    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let t = body.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            let name = t[1..t.len() - 1].trim().to_string();
            let key = unique_key(&root, &name);
            root.insert(key.clone(), Value::Object(Map::new()));
            section = Some(key);
            sections += 1;
            continue;
        }
        let Some(eq) = body.find('=') else {
            other += 1;
            continue;
        };
        let key = body[..eq].trim();
        if key.is_empty() || key.contains(char::is_whitespace) && !key.contains('.') && key.split_whitespace().count() > 3 {
            other += 1;
            continue;
        }
        kv += 1;
        // Value span: trimmed, and inside quotes if quoted.
        let raw_val = &body[eq + 1..];
        let lead = raw_val.len() - raw_val.trim_start().len();
        let mut vs = line_start + eq + 1 + lead;
        let mut val = raw_val.trim();
        let mut ve = vs + val.len();
        let quoted = val.len() >= 2 && val.starts_with('"') && val.ends_with('"');
        if quoted {
            vs += 1;
            ve -= 1;
            val = &val[1..val.len() - 1];
        }
        let shown = if quoted { Value::String(val.replace("\\\"", "\"")) } else { text_to_value(val) };
        let map = match &section {
            Some(s) => root.get_mut(s).and_then(Value::as_object_mut).unwrap(),
            None => &mut root,
        };
        let k = insert_unique(map, key, shown);
        let ptr = match &section {
            Some(s) => child(&child("", s), &k),
            None => child("", &k),
        };
        editable.insert(ptr, leaves.len());
        leaves.push(Leaf { start: vs, end: ve, quoted, orig: val.to_string() });
    }

    let by_ext = INI_EXTS.contains(&ext);
    let looks_ini = kv >= 2 && (sections >= 1 || kv >= 3) && kv * 10 >= (kv + other) * 7;
    if kv == 0 || !(by_ext || looks_ini) {
        return Err("not an INI file".into());
    }
    let backend = IniBackend { raw: text.as_bytes().to_vec(), leaves };
    Ok(TreeDoc::new(Value::Object(root), editable, "INI / config", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; game settings\r\n[Player]\r\nName = \"Ash Ketchum\"\r\nGold=1500\r\nHardMode=False\r\n\r\n[World]\r\nDay = 12 ; inline\r\nSeed=0042\r\n";

    #[test]
    fn edit_values_keep_everything_else() {
        let mut doc = parse(SAMPLE, "").unwrap();
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        assert_eq!(doc.view.pointer("/Player/Gold").unwrap(), 1500);
        assert_eq!(doc.view.pointer("/Player/Name").unwrap(), "Ash Ketchum");
        assert_eq!(doc.view.pointer("/World/Seed").unwrap(), "0042", "leading zeros stay text");
        doc.set("/Player/Gold", Value::from(25)).unwrap();
        doc.set("/Player/HardMode", Value::Bool(true)).unwrap();
        doc.set("/Player/Name", Value::from("Misty")).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, SAMPLE.replace("Gold=1500", "Gold=25").replace("HardMode=False", "HardMode=True").replace("Ash Ketchum", "Misty"));
        assert!(doc.set("/Player/Name", Value::from("two\nlines")).is_err());
    }

    #[test]
    fn plain_prose_is_not_ini() {
        assert!(parse("Hello there.\nThis is a note = not config.\nJust text here.\n", "").is_err());
    }
}
