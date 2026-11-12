//! The Long Dark saves (slot files without an extension, e.g. `sandbox1`).
//!
//! The file is a LibLZF stream of a JSON object `{"m_Timestamp": …, "m_Dict": {"boot": …,
//! "global": …, …}}`. Each `boot`/`global` entry is base64 of another LZF stream of JSON, and
//! inside those many values (`m_Condition_Serialized`, …) are again JSON in a string. All of
//! it is opened as one tree here, and every level is packed again on save. The layering is as
//! The Long Dark Save Editor (MIT, FINDarkside) reads it.

use super::wasteland3::{lzf_compress, lzf_inflate};
use base64::Engine;
use serde_json::Value;

const LIMIT: usize = 512 << 20;

/// What to turn back into a string again, deepest first: `(pointer, packed with LZF + base64)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrapped(pub Vec<(String, bool)>);

pub(crate) fn esc(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}

pub(crate) fn as_json(s: &str) -> Option<Value> {
    let t = s.trim_start();
    if !(t.starts_with('{') || t.starts_with('[')) {
        return None;
    }
    serde_json::from_str::<Value>(s).ok().filter(|v| v.is_object() || v.is_array())
}

pub(crate) fn expand(v: &mut Value, ptr: &str, out: &mut Vec<(String, bool)>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m.iter_mut() {
                let p = format!("{ptr}/{}", esc(k));
                if let Value::String(s) = x {
                    if let Some(mut inner) = as_json(s) {
                        expand(&mut inner, &p, out);
                        out.push((p, false));
                        *x = inner;
                    }
                } else {
                    expand(x, &p, out);
                }
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter_mut().enumerate() {
                let p = format!("{ptr}/{i}");
                if let Value::String(s) = x {
                    if let Some(mut inner) = as_json(s) {
                        expand(&mut inner, &p, out);
                        out.push((p, false));
                        *x = inner;
                    }
                } else {
                    expand(x, &p, out);
                }
            }
        }
        _ => {}
    }
}

fn unpack_entry(s: &str) -> Option<Value> {
    let raw = base64::engine::general_purpose::STANDARD.decode(s.trim()).ok()?;
    let text = String::from_utf8(lzf_inflate(&raw, LIMIT).ok()?).ok()?;
    as_json(&text)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 16 && d[0] < 32
}

/// Unpacks a slot file into one JSON text.
pub fn open(d: &[u8]) -> Option<(Wrapped, Vec<u8>)> {
    if !looks(d) {
        return None;
    }
    let text = String::from_utf8(lzf_inflate(d, LIMIT).ok()?).ok()?;
    if !text.contains("\"m_Dict\"") {
        return None;
    }
    let mut root: Value = serde_json::from_str(&text).ok()?;
    let mut out = vec![];
    let keys: Vec<String> = root.get("m_Dict")?.as_object()?.keys().cloned().collect();
    for k in keys {
        let p = format!("/m_Dict/{}", esc(&k));
        let Some(Value::String(s)) = root.pointer(&p).cloned() else { continue };
        if s.len() < 24 {
            continue;
        }
        let Some(mut inner) = unpack_entry(&s) else { continue };
        expand(&mut inner, &p, &mut out);
        out.push((p.clone(), true));
        *root.pointer_mut(&p)? = inner;
    }
    if !out.iter().any(|(_, packed)| *packed) {
        return None;
    }
    Some((Wrapped(out), serde_json::to_vec_pretty(&root).ok()?))
}

/// Turns the opened strings back into strings (and LZF + base64 where marked), deepest first.
pub(crate) fn restring(root: &mut Value, list: &[(String, bool)]) -> Result<(), String> {
    let mut order: Vec<&(String, bool)> = list.iter().collect();
    // Deepest first, so inner strings are made before the entries that hold them.
    order.sort_by_key(|(p, _)| std::cmp::Reverse(p.matches('/').count()));
    for (p, packed) in order {
        let Some(slot) = root.pointer_mut(p) else { continue };
        let text = serde_json::to_string(slot).map_err(|e| e.to_string())?;
        *slot = Value::String(if *packed { base64::engine::general_purpose::STANDARD.encode(lzf_compress(text.as_bytes())) } else { text });
    }
    Ok(())
}

/// Packs every level again.
pub fn pack(w: &Wrapped, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut root: Value = serde_json::from_slice(data).map_err(|e| format!("The save is no longer valid JSON: {e}"))?;
    restring(&mut root, &w.0)?;
    let text = serde_json::to_string(&root).map_err(|e| e.to_string())?;
    Ok(lzf_compress(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_levels_round_trip() {
        let cond = r#"{"m_CurrentHPProxy":87.5,"m_NeverDieProxy":false}"#;
        let global = serde_json::json!({"m_Version": 5, "m_Condition_Serialized": cond, "m_Inventory_Serialized": "not json"}).to_string();
        let boot = serde_json::json!({"m_SceneName": "RuralRegion"}).to_string();
        let enc = |s: &str| base64::engine::general_purpose::STANDARD.encode(lzf_compress(s.as_bytes()));
        let slot = serde_json::json!({"m_Timestamp": "2024-01-01 00:00:00", "m_Dict": {"boot": enc(&boot), "global": enc(&global)}}).to_string();
        let file = lzf_compress(slot.as_bytes());
        let (w, json) = open(&file).expect("opens");
        let v: Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(v.pointer("/m_Dict/global/m_Condition_Serialized/m_CurrentHPProxy").unwrap(), 87.5);
        assert_eq!(v.pointer("/m_Dict/boot/m_SceneName").unwrap(), "RuralRegion");
        let mut v2 = v.clone();
        *v2.pointer_mut("/m_Dict/global/m_Condition_Serialized/m_CurrentHPProxy").unwrap() = Value::from(100);
        let again = pack(&w, &serde_json::to_vec(&v2).unwrap()).unwrap();
        let (_, back) = open(&again).expect("reopens");
        let b: Value = serde_json::from_slice(&back).unwrap();
        assert_eq!(b.pointer("/m_Dict/global/m_Condition_Serialized/m_CurrentHPProxy").unwrap(), 100);
        assert_eq!(b.pointer("/m_Dict/global/m_Inventory_Serialized").unwrap(), "not json");
    }
}
