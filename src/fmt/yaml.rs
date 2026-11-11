//! YAML saves (trusted by extension only — almost any text is valid YAML).
//! Saving rebuilds the document, so comments and custom formatting are not kept.

use crate::tree::{Backend, TreeDoc};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct YamlBackend {
    raw: Vec<u8>,
}

impl Backend for YamlBackend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        serde_yaml::to_string(view)
            .map(String::into_bytes)
            .map_err(|e| e.to_string())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
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
    let view: Value = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
    if !(view.is_object() || view.is_array()) {
        return Err("not a YAML document".into());
    }
    let mut editable = HashMap::new();
    register(&view, "", &mut editable);
    let backend = YamlBackend {
        raw: text.as_bytes().to_vec(),
    };
    Ok(
        TreeDoc::new(view, editable, "YAML", Box::new(backend)).with_note(
            "YAML: saving rewrites the file, so comments and custom formatting are not kept.",
        ),
    )
}
