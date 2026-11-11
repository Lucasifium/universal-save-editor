//! Windows registry exports (`.reg`, "Windows Registry Editor Version 5.00"), as Unity's
//! PlayerPrefs keeps them on Windows: `HKCU\Software\<company>\<product>`, one value per
//! preference, its name followed by `_h` and a hash.
//!
//! Unity writes an int as a REG_DWORD, a float as a REG_DWORD holding the 8 bytes of a double
//! (exported as `hex(4):`), and a string as REG_BINARY UTF-8 text ending in a zero byte. The
//! values open as a JSON object keyed by the preference name (without the hash); everything
//! else in the file (other value types, comments) is written back exactly as it was.
//!
//! Layout facts from sabresaurus's PlayerPrefsEditor (MIT).

use serde_json::{Map, Number, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Int,
    Float,
    Text,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Copied as it was.
    Raw(String),
    /// A value shown for editing: (JSON key, registry name, kind).
    Value {
        key: String,
        name: String,
        kind: Kind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Meta {
    pub lines: Vec<Line>,
}

const HEAD: &str = "Windows Registry Editor Version 5.00";

/// The preference name without Unity's `_h<hash>` suffix.
fn pref_name(reg: &str) -> &str {
    match reg.rsplit_once("_h") {
        Some((n, h)) if !n.is_empty() && !h.is_empty() && h.bytes().all(|b| b.is_ascii_digit()) => {
            n
        }
        _ => reg,
    }
}

fn unquote(s: &str) -> Option<String> {
    let s = s.strip_prefix('"')?.strip_suffix('"')?;
    Some(s.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    s.split(',')
        .map(|b| b.trim())
        .filter(|b| !b.is_empty())
        .map(|b| u8::from_str_radix(b, 16).ok())
        .collect()
}

fn hex_text(prefix: &str, name: &str, bytes: &[u8]) -> String {
    let mut out = format!("{}={prefix}", quote(name));
    let mut col = out.len();
    for (i, b) in bytes.iter().enumerate() {
        let item = if i + 1 < bytes.len() {
            format!("{b:02x},")
        } else {
            format!("{b:02x}")
        };
        if col + item.len() > 78 && i + 1 < bytes.len() {
            out.push_str("\\\r\n  ");
            col = 2;
        }
        out.push_str(&item);
        col += item.len();
    }
    out
}

/// Opens a `.reg` export: the JSON object of the values, and how to write them back.
pub fn open(text: &str) -> Option<(Meta, Vec<u8>)> {
    let text = text.trim_start_matches('\u{FEFF}');
    if !text.starts_with(HEAD) {
        return None;
    }
    // Join continuation lines ("\" at the end).
    let mut logical: Vec<String> = vec![];
    let mut cur = String::new();
    for line in text.lines() {
        let l = line.trim_end_matches('\r');
        if !cur.is_empty() {
            cur.push_str(l.trim_start());
        } else {
            cur.push_str(l);
        }
        if cur.ends_with('\\') && !cur.ends_with("\\\\\"") {
            cur.pop();
            continue;
        }
        logical.push(std::mem::take(&mut cur));
    }
    if !cur.is_empty() {
        logical.push(cur);
    }
    let mut meta = Meta::default();
    let mut map = Map::new();
    for l in logical {
        let parsed = (|| {
            if !l.starts_with('"') {
                return None;
            }
            // The name ends at the first unescaped quote followed by '='.
            let mut end = None;
            let b = l.as_bytes();
            let mut i = 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if b[i] == b'"' {
                    end = Some(i);
                    break;
                }
                i += 1;
            }
            let end = end?;
            let name = unquote(&l[..=end])?;
            let data = l[end + 1..].strip_prefix('=')?;
            let (kind, value) = if let Some(h) = data.strip_prefix("dword:") {
                (
                    Kind::Int,
                    Value::from(u32::from_str_radix(h, 16).ok()? as i32),
                )
            } else if let Some(h) = data.strip_prefix("hex(4):") {
                let b = hex_bytes(h)?;
                match b.len() {
                    8 => (
                        Kind::Float,
                        Value::Number(Number::from_f64(f64::from_le_bytes(b.try_into().ok()?))?),
                    ),
                    4 => (
                        Kind::Int,
                        Value::from(i32::from_le_bytes(b.try_into().ok()?)),
                    ),
                    _ => return None,
                }
            } else if let Some(h) = data.strip_prefix("hex:") {
                let b = hex_bytes(h)?;
                let t = String::from_utf8(b.strip_suffix(&[0]).unwrap_or(&b).to_vec()).ok()?;
                (Kind::Text, Value::String(t))
            } else {
                return None;
            };
            Some((name, kind, value))
        })();
        match parsed {
            Some((name, kind, value)) => {
                let mut key = pref_name(&name).to_string();
                if map.contains_key(&key) {
                    key = name.clone();
                }
                map.insert(key.clone(), value);
                meta.lines.push(Line::Value { key, name, kind });
            }
            None => meta.lines.push(Line::Raw(l)),
        }
    }
    if map.is_empty() {
        return None;
    }
    Some((meta, serde_json::to_vec_pretty(&Value::Object(map)).ok()?))
}

/// The `.reg` text for edited values (CRLF lines, as regedit writes them).
pub fn save(meta: &Meta, json: &[u8]) -> Result<Vec<u8>, String> {
    let v: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let obj = v.as_object().ok_or("the preferences must stay an object")?;
    let mut out = vec![];
    for l in &meta.lines {
        out.push(match l {
            Line::Raw(r) => r.clone(),
            Line::Value { key, name, kind } => {
                let val = obj.get(key).ok_or_else(|| format!("{key} was removed"))?;
                match kind {
                    Kind::Int => {
                        let n = val
                            .as_i64()
                            .filter(|n| *n >= i32::MIN as i64 && *n <= u32::MAX as i64)
                            .ok_or_else(|| format!("{key} must be a whole number"))?;
                        format!("{}=dword:{:08x}", quote(name), n as u32)
                    }
                    Kind::Float => {
                        let f = val
                            .as_f64()
                            .ok_or_else(|| format!("{key} must be a number"))?;
                        hex_text("hex(4):", name, &f.to_le_bytes())
                    }
                    Kind::Text => {
                        let s = val.as_str().ok_or_else(|| format!("{key} must be text"))?;
                        let mut b = s.as_bytes().to_vec();
                        b.push(0);
                        hex_text("hex:", name, &b)
                    }
                }
            }
        });
    }
    let mut text = out.join("\r\n");
    if !text.ends_with("\r\n") {
        text.push_str("\r\n");
    }
    Ok(text.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_prefs_round_trip() {
        let reg = "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Software\\Studio\\Game]\r\n\
            \"Coins_h1234567\"=dword:000003e8\r\n\
            \"Volume_h99\"=hex(4):00,00,00,00,00,00,e0,3f\r\n\
            \"PlayerName_h42\"=hex:41,6e,6e,00\r\n\
            \"Other\"=\"plain string\"\r\n";
        let (meta, json) = open(reg).unwrap();
        let v: Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(v["Coins"], 1000);
        assert_eq!(v["Volume"], 0.5);
        assert_eq!(v["PlayerName"], "Ann");
        assert_eq!(String::from_utf8(save(&meta, &json).unwrap()).unwrap(), reg);
        let edited = serde_json::to_vec(&serde_json::json!({"Coins": 99999, "Volume": 0.25, "PlayerName": "A long player name that wraps onto a second line of hex"})).unwrap();
        let back = String::from_utf8(save(&meta, &edited).unwrap()).unwrap();
        assert!(back.contains("\"Coins_h1234567\"=dword:0001869f"));
        let (_, again) = open(&back).unwrap();
        let v: Value = serde_json::from_slice(&again).unwrap();
        assert_eq!(
            v["PlayerName"],
            "A long player name that wraps onto a second line of hex"
        );
        assert_eq!(v["Volume"], 0.25);
    }
}

#[cfg(test)]
mod layer_tests {
    #[test]
    fn utf16_export_opens_and_saves_back() {
        let reg = "Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Software\\S\\G]\r\n\"Gems_h7\"=dword:00000005\r\n";
        let mut raw = vec![0xFF, 0xFE];
        raw.extend(reg.encode_utf16().flat_map(|u| u.to_le_bytes()));
        let d = crate::formats::decode_with(&raw, &Default::default());
        assert!(
            d.layers
                .iter()
                .any(|l| matches!(l, crate::formats::Layer::RegFile(_))),
            "{:?}",
            d.layers.iter().map(|l| l.name()).collect::<Vec<_>>()
        );
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), raw);
    }
}
