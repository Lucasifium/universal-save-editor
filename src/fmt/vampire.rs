//! Vampire Survivors `SaveData` / `SaveData.sav`: one line of JSON whose `checksum` is the
//! SHA-256 (lowercase hex) of the same JSON written with `checksum` set to "".
//!
//! The game checks it by writing its data out again, so a save must be written exactly as
//! the game's serializer would: keys in the same order, no spaces, and numbers in the
//! build's style. The older JavaScript builds write whole floats as `946`; the Unity
//! builds keep `946.0` on a float and use `1E+21` exponents. Values that weren't edited
//! keep their exact text; an edited number is written in the save's style (facts from
//! chrisdfennell/vampire-survivors-save-editor, MIT).
//!
//! The whole document is written again when anything changes, so screens may also
//! replace a whole list (bought power-ups are a list where an id repeats once per rank).

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub const KIND: &str = "Vampire Survivors save";

#[derive(Clone, Debug)]
struct VsBackend {
    raw: Arc<Vec<u8>>,
    /// Where `checksum` goes among the top-level keys (it isn't shown: it's worked out on save).
    at: usize,
    /// Unity (C#) build: whole floats keep ".0".
    dotnet: bool,
    /// Per leaf: was the number written as a float (`0.5`, `1.0`, `1E+21`)?
    float: Vec<bool>,
}

/// SHA-256 (FIPS 180-4).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, b) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    let mut out = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

fn hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Compact JSON as the game writes it; numbers keep their text (see `normalize`).
fn write_json(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => quote(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                quote(k, out);
                out.push(':');
                write_json(x, out);
            }
            out.push('}');
        }
    }
}

fn serialize(v: &Value) -> String {
    let mut s = String::new();
    write_json(v, &mut s);
    s
}

/// The object with `checksum` put back at its place among the top-level keys.
fn with_field(view: &Value, at: usize, sum: &str) -> Value {
    let mut m = Map::new();
    let put = |m: &mut Map<String, Value>| {
        m.insert("checksum".into(), Value::String(sum.into()));
    };
    if let Value::Object(o) = view {
        for (i, (k, v)) in o.iter().enumerate() {
            if i == at {
                put(&mut m);
            }
            m.insert(k.clone(), v.clone());
        }
        if at >= o.len() {
            put(&mut m);
        }
    }
    Value::Object(m)
}

/// The save as the game writes it: the data, with its checksum at `at`.
fn file_text(view: &Value, at: usize) -> String {
    let sum = hex(&sha256(serialize(&with_field(view, at, "")).as_bytes()));
    serialize(&with_field(view, at, &sum))
}

fn is_float_text(t: &str) -> bool {
    t.contains(['.', 'e', 'E'])
}

/// JavaScript `Number.prototype.toString` for a finite number.
fn js_number(f: f64) -> String {
    if f == 0.0 {
        return "0".into();
    }
    // Shortest round-trip digits and exponent: "d.ddde±x".
    let e = format!("{:e}", f.abs());
    let (mant, exp) = e.split_once('e').unwrap();
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let k = digits.len() as i32;
    let n = exp.parse::<i32>().unwrap() + 1;
    let sign = if f < 0.0 { "-" } else { "" };
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let m = if k > 1 { format!("{}.{}", &digits[..1], &digits[1..]) } else { digits.clone() };
        format!("{m}e{}{}", if n - 1 >= 0 { "+" } else { "-" }, (n - 1).abs())
    };
    format!("{sign}{body}")
}

/// .NET round-trip text for a float: always a decimal point or an exponent (`5.0`, `1E+21`).
fn dotnet_number(f: f64) -> String {
    let e = format!("{:e}", f);
    let (mant, exp) = e.split_once('e').unwrap();
    let x: i32 = exp.parse().unwrap();
    if (-5..15).contains(&x) {
        let s = format!("{f}");
        if s.contains('.') {
            s
        } else {
            format!("{s}.0")
        }
    } else {
        format!("{mant}E{}{:02}", if x >= 0 { "+" } else { "-" }, x.abs())
    }
}

fn number_from(text: String) -> Value {
    serde_json::from_str::<Number>(&text).map(Value::Number).unwrap_or(Value::Null)
}

impl Backend for VsBackend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.to_vec());
        }
        Ok(file_text(view, self.at).into_bytes())
    }

    /// An edited number in the style the game itself writes it, so the checksum matches.
    fn normalize(&self, leaf: usize, v: Value) -> Value {
        let Value::Number(n) = &v else { return v };
        let Some(f) = n.as_f64().filter(|f| f.is_finite()) else { return v };
        let int = n.as_i64().is_some() || n.as_u64().is_some();
        let float = self.float.get(leaf).copied().unwrap_or(false);
        if self.dotnet {
            if float {
                number_from(dotnet_number(f))
            } else if int {
                v
            } else {
                Value::from(f.round() as i64)
            }
        } else if int {
            v
        } else {
            number_from(js_number(f))
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// A Vampire Survivors save: a JSON object with a `checksum` key.
pub fn looks(d: &[u8]) -> bool {
    d.first() == Some(&b'{') && d.len() < 64 << 20 && d.windows(12).any(|w| w == b"\"checksum\":\"")
}

fn register(v: &Value, ptr: &mut String, editable: &mut HashMap<String, usize>, float: &mut Vec<bool>) {
    let mut add = |p: &str, f: bool| {
        editable.insert(p.to_string(), float.len());
        float.push(f);
    };
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let n = ptr.len();
                ptr.push('/');
                ptr.push_str(&crate::tree::esc(k));
                register(x, ptr, editable, float);
                ptr.truncate(n);
            }
        }
        Value::Array(a) => {
            // The whole list can be replaced (bought power-ups), and each entry edited.
            add(ptr, false);
            for (i, x) in a.iter().enumerate() {
                let n = ptr.len();
                ptr.push('/');
                ptr.push_str(&i.to_string());
                register(x, ptr, editable, float);
                ptr.truncate(n);
            }
        }
        Value::Number(n) => add(ptr, is_float_text(&n.to_string())),
        _ => add(ptr, false),
    }
}

fn any_dotnet_float(v: &Value) -> bool {
    match v {
        Value::Object(m) => m.values().any(any_dotnet_float),
        Value::Array(a) => a.iter().any(any_dotnet_float),
        Value::Number(n) => {
            let t = n.to_string();
            t.ends_with(".0") || t.contains('E')
        }
        _ => false,
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let text = std::str::from_utf8(d).map_err(|e| e.to_string())?;
    let file: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let obj: &Map<String, Value> = file.as_object().ok_or("not an object")?;
    let at = obj.keys().position(|k| k == "checksum").ok_or("no checksum")?;
    if !obj["checksum"].is_string() || !obj.contains_key("Coins") {
        return Err("not a Vampire Survivors save".into());
    }
    let view = Value::Object(obj.iter().filter(|(k, _)| *k != "checksum").map(|(k, v)| (k.clone(), v.clone())).collect());
    // Only a save this writer reproduces exactly (checksum included) is opened this way:
    // then an edit gets a checksum the game accepts.
    if file_text(&view, at).as_bytes() != d {
        return Err("the checksum doesn't match this data".into());
    }
    let version = obj.get("Version").and_then(Value::as_f64).unwrap_or(0.0);
    let dotnet = any_dotnet_float(&view) || version >= 15000.0;
    let mut editable = HashMap::new();
    let mut float = Vec::new();
    register(&view, &mut String::new(), &mut editable, &mut float);
    let backend = VsBackend { raw: Arc::new(d.to_vec()), at, dotnet, float };
    let note = format!(
        "Vampire Survivors save ({} build): the checksum is worked out again when you save. The game reads the copy in \
         Steam\\userdata\\<account>\\1794680\\remote; close the game (and let Steam Cloud finish) before editing.",
        if dotnet { "Unity" } else { "JavaScript" }
    );
    Ok(TreeDoc::new(view, editable, KIND, Box::new(backend)).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic save with a correct checksum.
    fn save(body: &str) -> Vec<u8> {
        let v: Value = serde_json::from_str(body).unwrap();
        let o = v.as_object().unwrap();
        let at = o.keys().position(|k| k == "checksum").unwrap();
        let rest = Value::Object(o.iter().filter(|(k, _)| *k != "checksum").map(|(k, v)| (k.clone(), v.clone())).collect());
        file_text(&rest, at).into_bytes()
    }

    #[test]
    fn sha256_known_values() {
        assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let long = vec![b'a'; 1000];
        assert_eq!(hex(&sha256(&long)), "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3");
    }

    #[test]
    fn number_styles() {
        assert_eq!(js_number(946.0), "946");
        assert_eq!(js_number(0.7), "0.7");
        assert_eq!(js_number(1e21), "1e+21");
        assert_eq!(js_number(1e-7), "1e-7");
        assert_eq!(js_number(-12.5), "-12.5");
        assert_eq!(dotnet_number(5.0), "5.0");
        assert_eq!(dotnet_number(0.25), "0.25");
        assert_eq!(dotnet_number(1e21), "1E+21");
    }

    #[test]
    fn edit_keeps_order_and_fixes_checksum() {
        let raw = save(r#"{"SelectedCharacter":"ANTONIO","Version":7220,"Coins":120,"LifetimeCoins":300,"SoundsVolume":0.7,"BoughtPowerups":["POWER","POWER"],"checksum":"","Name":"café \"x\""}"#);
        assert!(looks(&raw));
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.kind_name, KIND);
        assert_eq!(doc.to_bytes().unwrap(), raw);
        assert!(doc.view.get("checksum").is_none(), "worked out on save, not shown");
        doc.set("/Coins", Value::from(121)).unwrap();
        doc.set("/SoundsVolume", serde_json::from_str::<Value>("1.0").unwrap()).unwrap();
        assert_eq!(doc.view["SoundsVolume"].to_string(), "1", "JavaScript writes whole floats without .0");
        doc.set("/BoughtPowerups", serde_json::json!(["POWER", "POWER", "ARMOR"])).unwrap();
        let out = doc.to_bytes().unwrap();
        let text = std::str::from_utf8(&out).unwrap();
        assert!(text.starts_with(r#"{"SelectedCharacter":"ANTONIO","Version":7220,"Coins":121,"LifetimeCoins":300,"SoundsVolume":1,"BoughtPowerups":["POWER","POWER","ARMOR"],"checksum":""#));
        let back = parse(&out).expect("checksum is right");
        assert_eq!(back.view["Coins"], 121);
        assert_eq!(back.view, doc.view);
        // A wrong checksum: not opened as this format (plain JSON instead).
        let bad = String::from_utf8(raw.clone()).unwrap().replace("\"Coins\":120", "\"Coins\":999");
        assert!(parse(bad.as_bytes()).is_err());
    }

    #[test]
    fn unity_style_keeps_floats() {
        let raw = save(r#"{"Version":15101,"Coins":250.0,"Kills":3,"checksum":""}"#);
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        doc.set("/Coins", Value::from(300)).unwrap();
        doc.set("/Kills", Value::from(4)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.starts_with(r#"{"Version":15101,"Coins":300.0,"Kills":4,"checksum":""#), "{out}");
        assert!(parse(out.as_bytes()).is_ok());
    }

    /// Real saves: `VS_SAVES` = files separated by `|`.
    #[test]
    #[ignore]
    fn research_real_saves() {
        let Ok(list) = std::env::var("VS_SAVES") else { return };
        for path in list.split('|').filter(|s| !s.is_empty()) {
            let raw = std::fs::read(path).unwrap();
            assert!(looks(&raw), "{path}");
            let mut doc = parse(&raw).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(doc.to_bytes().unwrap(), raw, "{path}: round trip");
            let before = doc.view.clone();
            let c = doc.view["Coins"].as_f64().unwrap();
            doc.set("/Coins", serde_json::json!(c as i64 + 1)).unwrap();
            let out = doc.to_bytes().unwrap();
            let back = parse(&out).unwrap_or_else(|e| panic!("{path}: edited save: {e}"));
            assert_eq!(back.view["Coins"].as_f64(), Some(c + 1.0));
            let mut b = back.view.clone();
            b["Coins"] = before["Coins"].clone();
            assert_eq!(b, before, "{path}: nothing else changes");
            eprintln!("{path}: Version {} coins {c} -> ok, {} bytes", before["Version"], out.len());
        }
    }
}
