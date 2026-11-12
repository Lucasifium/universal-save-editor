//! FANTASIAN Neo Dimension (Steam): `root.json`.
//!
//! `root.json` is `{"dataString": "<records JSON as a string>"}`; each record has a `path`
//! (`Data/GameData0.json` …) and an `encryptedString`: base64 of AES-128-CBC (a fixed key and IV
//! built into the game) over JSON `{"keys": [...], "values": [...]}` written with 4-space
//! indents. Each value is itself a JSON string (`GameSystemInfo`, `PlayerInfo` …).
//!
//! One record is open at a time. It is shown as one object (`{key: value}`) when writing it
//! back gives exactly the text the game wrote; otherwise it is shown as the game wrote it. On
//! save, only that record's base64 is replaced in the file; everything else stays as it was.
//!
//! Format, key and IV from FantasianND-Save-Editor (mathcodergamer) and
//! Fantasian-Save-Transfer (RobThePCGuy), both GPL-3.0.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::Engine;
use serde_json::Value;

const KEY: &[u8; 16] = b"yrhWj8EiU83kXupm";
const IV: &[u8; 16] = b"Nq4G3pTQFLTCeiB7";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub name: String,
    /// Byte range of the base64 text in the file (escaped `\/` included).
    pub span: (usize, usize),
    /// The base64 writes `/` as `\/` here.
    pub escaped_slash: bool,
    /// The record is shown as one `{key: value}` object.
    pub merged: bool,
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn decrypt(d: &[u8]) -> Option<Vec<u8>> {
    cbc::Decryptor::<aes::Aes128>::new(KEY.into(), IV.into()).decrypt_padded_vec_mut::<Pkcs7>(d).ok()
}
fn encrypt(d: &[u8]) -> Vec<u8> {
    cbc::Encryptor::<aes::Aes128>::new(KEY.into(), IV.into()).encrypt_padded_vec_mut::<Pkcs7>(d)
}

/// The base64 text after each `encryptedString` key in the raw file, in order.
fn spans(text: &str) -> Vec<(usize, usize, bool)> {
    let b = text.as_bytes();
    let mut out = vec![];
    let mut from = 0;
    while let Some(i) = text[from..].find("encryptedString") {
        let mut p = from + i + "encryptedString".len();
        // Past the key's closing quote, the colon and the value's opening quote, escaped or not.
        while p < b.len() && matches!(b[p], b'\\' | b'"' | b':' | b' ' | b'\t' | b'\r' | b'\n') {
            p += 1;
        }
        let start = p;
        let mut escaped = false;
        while p < b.len() {
            match b[p] {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => p += 1,
                b'\\' if b.get(p + 1) == Some(&b'/') => {
                    escaped = true;
                    p += 2
                }
                _ => break,
            }
        }
        out.push((start, p, escaped));
        from = p;
    }
    out
}

/// JSON text the way Python's `json.dumps(..., indent=4)` writes it.
fn py_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{:04x}", u));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn py_dump(keys: &[String], values: &[String]) -> String {
    let mut out = String::from("{\n    \"keys\": [");
    for (i, k) in keys.iter().enumerate() {
        out.push_str(if i == 0 { "\n        " } else { ",\n        " });
        py_string(k, &mut out);
    }
    out.push_str(if keys.is_empty() { "],\n    \"values\": [" } else { "\n    ],\n    \"values\": [" });
    for (i, v) in values.iter().enumerate() {
        out.push_str(if i == 0 { "\n        " } else { ",\n        " });
        py_string(v, &mut out);
    }
    out.push_str(if values.is_empty() { "]\n}" } else { "\n    ]\n}" });
    out
}

/// The record's keys and values when all of them are strings.
fn pairs(plain: &[u8]) -> Option<(Vec<String>, Vec<String>)> {
    let v: Value = serde_json::from_slice(plain).ok()?;
    let strs = |k: &str| -> Option<Vec<String>> { v.get(k)?.as_array()?.iter().map(|x| x.as_str().map(String::from)).collect() };
    let (k, vals) = (strs("keys")?, strs("values")?);
    (k.len() == vals.len()).then_some((k, vals))
}

pub fn looks(d: &[u8]) -> bool {
    d.first() == Some(&b'{') && d.len() < 64 << 20 && memchr(d, b"encryptedString") && memchr(d, b"dataString")
}

fn memchr(d: &[u8], pat: &[u8]) -> bool {
    d.windows(pat.len()).any(|w| w == pat)
}

/// Lists the records; `None` if this isn't a Fantasian `root.json`.
pub fn records(d: &[u8]) -> Option<Vec<Record>> {
    let text = std::str::from_utf8(d).ok()?;
    let root: Value = serde_json::from_str(text).ok()?;
    let inner: Value = serde_json::from_str(root.get("dataString")?.as_str()?).ok()?;
    let recs = inner.get("records")?.as_array()?;
    let found = spans(text);
    if found.len() != recs.len() {
        return None;
    }
    let mut out = vec![];
    for (i, (r, &(a, b, esc))) in recs.iter().zip(&found).enumerate() {
        let enc = r.get("encryptedString")?.as_str()?;
        // The text found in the file must be the same base64 the parser read.
        if text[a..b].replace("\\/", "/") != enc {
            return None;
        }
        let name = r.get("path").and_then(|p| p.as_str()).map(String::from).unwrap_or_else(|| format!("record {i}"));
        out.push(Record { name, span: (a, b), escaped_slash: esc, merged: false });
    }
    Some(out)
}

/// Decrypts one record. Returns the record (with `merged` set) and the text shown.
pub fn open(d: &[u8], mut r: Record) -> Option<(Record, Vec<u8>)> {
    let text = std::str::from_utf8(d).ok()?;
    let enc = text[r.span.0..r.span.1].replace("\\/", "/");
    if enc.is_empty() {
        return Some((r, vec![]));
    }
    let plain = decrypt(&b64().decode(enc).ok()?)?;
    if let Some((k, v)) = pairs(&plain) {
        if py_dump(&k, &v).as_bytes() == plain {
            let mut obj = serde_json::Map::new();
            for (k, v) in k.into_iter().zip(v) {
                obj.insert(k, Value::String(v));
            }
            if obj.len() == pairs(&plain)?.0.len() {
                r.merged = true;
                return Some((r, serde_json::to_vec_pretty(&Value::Object(obj)).ok()?));
            }
        }
    }
    Some((r, plain))
}

/// Writes one record back into the file.
pub fn close(d: &[u8], r: &Record, shown: &[u8]) -> Result<Vec<u8>, String> {
    let plain = if r.merged {
        let v: Value = serde_json::from_slice(shown).map_err(|e| format!("the save's data isn't JSON any more: {e}"))?;
        let obj = v.as_object().ok_or("the save's data must stay one JSON object")?;
        let mut keys = vec![];
        let mut values = vec![];
        for (k, v) in obj {
            keys.push(k.clone());
            values.push(v.as_str().map(String::from).ok_or_else(|| format!("{k} must stay text (a JSON string)"))?);
        }
        py_dump(&keys, &values).into_bytes()
    } else {
        shown.to_vec()
    };
    let mut enc = b64().encode(encrypt(&plain));
    if r.escaped_slash {
        enc = enc.replace('/', "\\/");
    }
    Ok([&d[..r.span.0], enc.as_bytes(), &d[r.span.1..]].concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_text(keys: &[&str], values: &[&str]) -> String {
        let k: Vec<String> = keys.iter().map(|s| s.to_string()).collect();
        let v: Vec<String> = values.iter().map(|s| s.to_string()).collect();
        b64().encode(encrypt(py_dump(&k, &v).as_bytes()))
    }

    #[test]
    fn records_open_and_write_back() {
        let r0 = record_text(&["GameSystemInfo", "Name"], &[r#"{"_money":120,"_playTimeSec":5}"#, "Kina é"]);
        let r1 = record_text(&["GameSystemInfo"], &[r#"{"_money":7}"#]);
        let inner = serde_json::json!({"records": [
            {"path": "Data/GameData0.json", "encryptedString": r0},
            {"path": "Data/GameData10.json", "encryptedString": r1},
        ]});
        let root = serde_json::to_string_pretty(&serde_json::json!({"dataString": inner.to_string()})).unwrap();
        let d = root.as_bytes();
        assert!(looks(d));
        let recs = records(d).unwrap();
        assert_eq!(recs.len(), 2);
        let (r, shown) = open(d, recs[0].clone()).unwrap();
        assert!(r.merged);
        let v: Value = serde_json::from_slice(&shown).unwrap();
        assert_eq!(v["Name"], "Kina é");
        // Unchanged: the same file.
        assert_eq!(close(d, &r, &shown).unwrap(), d);
        // Changed: only this record changes, and it reads back.
        let edited = String::from_utf8(shown).unwrap().replace("120", "999999");
        let out = close(d, &r, edited.as_bytes()).unwrap();
        let recs2 = records(&out).unwrap();
        let (_, again) = open(&out, recs2[0].clone()).unwrap();
        assert!(String::from_utf8(again).unwrap().contains("999999"));
        assert_eq!(open(&out, recs2[1].clone()).unwrap().1, open(d, recs[1].clone()).unwrap().1);
    }

    #[test]
    fn opens_through_decode() {
        let r0 = record_text(&["GameSystemInfo"], &[r#"{"_money":120}"#]);
        let inner = serde_json::json!({"records": [{"path": "Data/GameData0.json", "encryptedString": r0}]});
        let root = serde_json::to_string_pretty(&serde_json::json!({"dataString": inner.to_string()})).unwrap();
        let d = crate::formats::decode(root.as_bytes());
        assert!(matches!(d.layers.first(), Some(crate::formats::Layer::Fantasian { .. })), "{:?}", d.layers);
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), root.as_bytes());
    }
}
