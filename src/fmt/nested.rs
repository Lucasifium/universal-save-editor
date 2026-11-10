//! JSON inside JSON strings (Unity's `JsonUtility` saves keep each part of the game as a
//! string holding more JSON, sometimes several levels deep). The strings are opened into
//! real values so their numbers can be edited, and written back as strings.

use serde_json::Value;

/// The pointers (parents before their children) of the strings opened, and the document.
pub struct Opened {
    pub paths: Vec<String>,
    pub pretty: bool,
    pub bytes: Vec<u8>,
}

/// A string that holds a JSON object or array.
fn inner_json(s: &str) -> Option<Value> {
    let t = s.trim();
    let ok = (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'));
    if !ok || t.len() < 3 {
        return None;
    }
    match serde_json::from_str::<Value>(t).ok()? {
        v @ (Value::Object(_) | Value::Array(_)) => Some(v),
        _ => None,
    }
}

fn walk(v: &mut Value, ptr: &str, paths: &mut Vec<String>, depth: usize) {
    if depth > 8 {
        return;
    }
    match v {
        Value::String(s) => {
            if let Some(mut inner) = inner_json(s) {
                paths.push(ptr.to_string());
                walk(&mut inner, ptr, paths, depth + 1);
                *v = inner;
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter_mut().enumerate() {
                walk(x, &format!("{ptr}/{i}"), paths, depth);
            }
        }
        Value::Object(m) => {
            for (k, x) in m.iter_mut() {
                walk(x, &format!("{ptr}/{}", crate::json_view::pointer_escape(k)), paths, depth);
            }
        }
        _ => {}
    }
}

/// Opens the nested strings of a JSON document; `None` when it has none (or isn't JSON).
pub fn open(data: &[u8]) -> Option<Opened> {
    // Cheap test first: a string holding JSON has an escaped quote right after its brace or bracket.
    if data.len() > 64 << 20 || !data.windows(3).any(|w| w == b"{\\\"" || w == b"[\\\"") {
        return None;
    }
    let text = std::str::from_utf8(data).ok()?;
    let t = text.trim_start();
    if !(t.starts_with('{') || t.starts_with('[')) {
        return None;
    }
    let mut v: Value = serde_json::from_str(t).ok()?;
    let mut paths = vec![];
    walk(&mut v, "", &mut paths, 0);
    if paths.is_empty() {
        return None;
    }
    let pretty = t.lines().nth(1).is_some_and(|l| l.starts_with([' ', '\t']));
    let bytes = if pretty { serde_json::to_vec_pretty(&v).ok()? } else { serde_json::to_vec(&v).ok()? };
    Some(Opened { paths, pretty, bytes })
}

/// Turns the opened values back into strings (deepest first).
pub fn close(data: &[u8], paths: &[String], pretty: bool) -> Result<Vec<u8>, String> {
    let mut v: Value = serde_json::from_slice(data).map_err(|e| format!("not JSON: {e}"))?;
    for p in paths.iter().rev() {
        let Some(slot) = v.pointer_mut(p) else { continue };
        if matches!(slot, Value::Object(_) | Value::Array(_)) {
            let text = serde_json::to_string(slot).map_err(|e| e.to_string())?;
            *slot = Value::String(text);
        }
    }
    if pretty { serde_json::to_vec_pretty(&v) } else { serde_json::to_vec(&v) }.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_and_closes_nested_strings() {
        let inner = r#"{"Hp":12,"Items":[1,2],"Deep":"{\"A\":1.50}"}"#;
        let outer = serde_json::json!({"Version": 1, "State": inner, "Note": "{not json}", "List": ["[\"x\",\"y\"]"]});
        let bytes = serde_json::to_vec(&outer).unwrap();
        let o = open(&bytes).unwrap();
        let v: Value = serde_json::from_slice(&o.bytes).unwrap();
        assert_eq!(v["State"]["Hp"], 12);
        assert_eq!(v["State"]["Deep"]["A"].to_string(), "1.50");
        assert_eq!(v["Note"], "{not json}");
        assert_eq!(v["List"][0][1], "y");
        let mut edited = v.clone();
        edited["State"]["Hp"] = Value::from(99);
        let back = close(&serde_json::to_vec(&edited).unwrap(), &o.paths, o.pretty).unwrap();
        let back: Value = serde_json::from_slice(&back).unwrap();
        let state: Value = serde_json::from_str(back["State"].as_str().unwrap()).unwrap();
        assert_eq!(state["Hp"], 99);
        assert_eq!(state["Deep"].as_str().map(|s| serde_json::from_str::<Value>(s).unwrap()["A"].to_string()), Some("1.50".into()));
        // Unedited, the bytes come back the same.
        assert_eq!(close(&o.bytes, &o.paths, o.pretty).unwrap(), bytes);
    }
}
