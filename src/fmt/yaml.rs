//! YAML saves (trusted by extension only — almost any text is valid YAML).
//! Saving rebuilds the document, so comments and custom formatting are not kept.

use crate::tree::{Backend, TreeDoc};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct YamlBackend {
    raw: Vec<u8>,
}

/// A JSON value as a YAML value. (Serialising the JSON value directly would write numbers kept
/// at full precision as a `$serde_json::private::Number` mapping.)
pub fn to_yaml(v: &Value) -> serde_yaml::Value {
    use serde_yaml::Value as Y;
    match v {
        Value::Null => Y::Null,
        Value::Bool(b) => Y::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Y::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                Y::Number(u.into())
            } else {
                Y::Number(serde_yaml::Number::from(n.as_f64().unwrap_or(0.0)))
            }
        }
        Value::String(s) => Y::String(s.clone()),
        Value::Array(a) => Y::Sequence(a.iter().map(to_yaml).collect()),
        Value::Object(m) => Y::Mapping(m.iter().map(|(k, x)| (Y::String(k.clone()), to_yaml(x))).collect()),
    }
}

impl Backend for YamlBackend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        serde_yaml::to_string(&to_yaml(view)).map(String::into_bytes).map_err(|e| e.to_string())
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
    let backend = YamlBackend { raw: text.as_bytes().to_vec() };
    Ok(TreeDoc::new(view, editable, "YAML", Box::new(backend))
        .with_note("YAML: saving rewrites the file, so comments and custom formatting are not kept."))
}
