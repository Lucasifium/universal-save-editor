//! JSON written by Unity / .NET code can hold the bare words `Infinity`, `-Infinity` and `NaN`
//! as numbers, which standard JSON doesn't allow. They are shown as marker strings and written
//! back as the bare words.

use serde_json::Value;

const INF: &str = "\"\\u0001Infinity\"";
const NINF: &str = "\"\\u0001-Infinity\"";
const NAN: &str = "\"\\u0001NaN\"";

/// The text with the bare words replaced, when that makes it valid JSON.
pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(data).ok()?;
    let t = text.trim_start();
    if !(t.starts_with('{') || t.starts_with('[')) || !(text.contains("Infinity") || text.contains("NaN")) {
        return None;
    }
    if serde_json::from_str::<Value>(text).is_ok() {
        return None;
    }
    let (mut out, mut in_str, mut esc) = (String::with_capacity(text.len() + 64), false, false);
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if in_str {
            out.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            rest = &rest[c.len_utf8()..];
            continue;
        }
        if c == '"' {
            in_str = true;
        } else if let Some(r) = rest.strip_prefix("-Infinity") {
            out.push_str(NINF);
            rest = r;
            continue;
        } else if let Some(r) = rest.strip_prefix("Infinity") {
            out.push_str(INF);
            rest = r;
            continue;
        } else if let Some(r) = rest.strip_prefix("NaN") {
            out.push_str(NAN);
            rest = r;
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    serde_json::from_str::<Value>(&out).ok()?;
    Some(out.into_bytes())
}

pub fn pack(data: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(data).map_err(|e| e.to_string())?;
    Ok(text.replace(NINF, "-Infinity").replace(INF, "Infinity").replace(NAN, "NaN").into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let t = br#"{"a":Infinity,"b":-Infinity,"c":[NaN,1.5],"d":"Infinity in text"}"#;
        let json = open(t).unwrap();
        assert!(serde_json::from_slice::<Value>(&json).is_ok());
        assert_eq!(pack(&json).unwrap(), t.to_vec());
    }
}
