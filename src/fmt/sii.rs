//! SCS Software saves (Euro Truck Simulator 2, American Truck Simulator): `game.sii`,
//! `profile.sii`, `info.sii`.
//!
//! The files are `SiiNunit` text: units (`bank : _nameless.7646.a028.4150 { … }`) holding
//! `key: value` lines, where an array is a count line plus `key[0]`, `key[1]`, … lines and
//! a float may be written as `&` and its eight hex digits. The game keeps most saves in
//! an "ScsC" wrapper: a 56-byte header (magic, HMAC-SHA256, IV, size), then the text
//! zlib-compressed and AES-256-CBC encrypted with a key every install shares. The text
//! is edited in place; the wrapper is made again on save.
//!
//! Wrapper and key layout from SII_Decrypt (Mario Gruber's original, and its MIT C++
//! port by liam-dong); unit and property names from public format notes (facts only).

use crate::tree::{child, esc, text_to_value, value_to_text, Backend, Patches, TreeDoc};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;
use std::collections::HashMap;

pub const KIND: &str = "SCS truck simulator save";

const KEY: [u8; 32] = [
    0x2a, 0x5f, 0xcb, 0x17, 0x91, 0xd2, 0x2f, 0xb6, 0x02, 0x45, 0xb3, 0xd8, 0x36, 0x9e, 0xd0, 0xb2, 0xc2, 0x73, 0x71, 0x56, 0x3f, 0xbf, 0x1f, 0x3c, 0x9e, 0xdf, 0x6b, 0x11, 0x82, 0x5a, 0x5d,
    0x0a,
];

pub fn looks_encrypted(d: &[u8]) -> bool {
    d.len() > 56 && d.starts_with(b"ScsC")
}

/// Decrypts an "ScsC" file to its text (or whatever the game packed inside).
pub fn decrypt(d: &[u8]) -> Result<Vec<u8>, String> {
    if !looks_encrypted(d) {
        return Err("not an ScsC file".into());
    }
    let iv: [u8; 16] = d[36..52].try_into().unwrap();
    let size = u32::from_le_bytes(d[52..56].try_into().unwrap()) as usize;
    let mut body = d[56..].to_vec();
    body.truncate(body.len() / 16 * 16);
    let plain = cbc::Decryptor::<aes::Aes256>::new(&KEY.into(), &iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::NoPadding>(&mut body)
        .map_err(|_| "the save couldn't be decrypted".to_string())?
        .to_vec();
    let mut out = Vec::new();
    {
        use std::io::Read;
        let mut z = flate2::read::ZlibDecoder::new(&plain[..]);
        z.read_to_end(&mut out).map_err(|e| format!("the decrypted save isn't zlib data: {e}"))?;
    }
    if out.len() != size {
        return Err("the decrypted save has the wrong size".into());
    }
    Ok(out)
}

/// Packs text into an "ScsC" file again (zlib, AES-256-CBC, HMAC-SHA256 over IV and data).
pub fn encrypt(text: &[u8], iv: [u8; 16]) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    z.write_all(text).map_err(|e| e.to_string())?;
    let packed = z.finish().map_err(|e| e.to_string())?;
    let body = cbc::Encryptor::<aes::Aes256>::new(&KEY.into(), &iv.into()).encrypt_padded_vec_mut::<Pkcs7>(&packed);
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&KEY).map_err(|e| e.to_string())?;
    mac.update(&iv);
    mac.update(&body);
    let mut out = b"ScsC".to_vec();
    out.extend(mac.finalize().into_bytes());
    out.extend(iv);
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(body);
    Ok(out)
}

pub fn looks(text: &str) -> bool {
    text.trim_start_matches('\u{feff}').trim_start().starts_with("SiiNunit")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Style {
    Plain,
    Quoted,
    /// `&` and the eight hex digits of an f32.
    HexFloat,
}

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    style: Style,
    orig: String,
}

#[derive(Clone, Debug)]
struct SiiBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for SiiBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            let t = match l.style {
                Style::HexFloat => {
                    let f = v.as_f64().ok_or("expected a number")? as f32;
                    format!("&{:08x}", f.to_bits())
                }
                Style::Quoted => value_to_text(&l.orig, v).replace('"', "\\\""),
                Style::Plain => value_to_text(&l.orig, v),
            };
            p.add(l.start, l.end, t.into_bytes());
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let l = &self.leaves[leaf];
        if v.as_str().is_some_and(|s| s.contains(['\n', '\r'])) {
            return Err("values can't contain line breaks".into());
        }
        if l.style == Style::HexFloat && v.as_f64().is_none() {
            return Err("expected a number".into());
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn hex_float(v: &str) -> Option<f64> {
    let h = v.strip_prefix('&')?;
    if h.len() != 8 {
        return None;
    }
    let bits = u32::from_str_radix(h, 16).ok()?;
    let f = f32::from_bits(bits);
    f.is_finite().then(|| {
        // Shortest decimal that is the same f32, so the screen shows 0.5 and not 0.5000000298.
        format!("{f}").parse::<f64>().unwrap_or(f as f64)
    })
}

fn is_key(k: &str) -> bool {
    !k.is_empty() && k.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'[' | b']'))
}

/// Splits `economy : _nameless.1.2 {` (or without the brace) into type and name.
fn unit_header(t: &str) -> Option<(String, String)> {
    let t = t.strip_suffix('{').unwrap_or(t).trim();
    let (ty, name) = t.split_once(':')?;
    let (ty, name) = (ty.trim(), name.trim());
    (is_key(ty) && !ty.contains('[') && !name.is_empty() && !name.contains(char::is_whitespace)).then(|| (ty.to_string(), name.to_string()))
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not an SII file".into());
    }
    // First pass: how many units of each type (a type seen once is named by its type alone).
    let mut counts: HashMap<String, usize> = HashMap::new();
    {
        let mut depth = 0;
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('#') || t.starts_with("//") || t.is_empty() {
                continue;
            }
            if depth == 1 {
                if let Some((ty, _)) = unit_header(t) {
                    *counts.entry(ty).or_default() += 1;
                }
            }
            if t == "{" || t.ends_with('{') {
                depth += 1;
            }
            if t == "}" {
                depth -= 1;
            }
        }
    }

    let mut root = Map::new();
    let mut leaves: Vec<Leaf> = Vec::new();
    let mut editable: HashMap<String, usize> = HashMap::new();
    let mut depth = 0;
    let mut unit: Option<(String, Map<String, Value>, HashMap<String, usize>)> = None;
    let mut offset = 0;
    let mut pending: Option<(String, String)> = None;
    let finish = |unit: &mut Option<(String, Map<String, Value>, HashMap<String, usize>)>, root: &mut Map<String, Value>, editable: &mut HashMap<String, usize>| {
        if let Some((key, map, leafs)) = unit.take() {
            for (k, id) in leafs {
                editable.insert(k.replace("\u{0}", &esc(&key)), id);
            }
            root.insert(key, Value::Object(map));
        }
    };
    for line in text.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let t = body.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") || t.starts_with('@') {
            continue;
        }
        if depth == 0 {
            if t == "{" {
                depth = 1;
            }
            continue;
        }
        if depth == 1 {
            if t == "}" {
                depth = 0;
                continue;
            }
            if t == "{" {
                if let Some((ty, name)) = pending.take() {
                    depth = 2;
                    start_unit(&mut unit, &counts, ty, name);
                }
                continue;
            }
            if let Some((ty, name)) = unit_header(t) {
                if t.ends_with('{') {
                    depth = 2;
                    start_unit(&mut unit, &counts, ty, name);
                } else {
                    pending = Some((ty, name));
                }
            }
            continue;
        }
        // Inside a unit.
        if t == "}" {
            finish(&mut unit, &mut root, &mut editable);
            depth = 1;
            continue;
        }
        let Some(colon) = body.find(':') else { continue };
        let key = body[..colon].trim();
        if !is_key(key) {
            continue;
        }
        let raw_val = &body[colon + 1..];
        let lead = raw_val.len() - raw_val.trim_start().len();
        let mut vs = line_start + colon + 1 + lead;
        let mut val = raw_val.trim();
        let mut ve = vs + val.len();
        let Some((_, map, leafs)) = unit.as_mut() else { continue };
        let (style, shown) = if val.len() >= 2 && val.starts_with('"') && val.ends_with('"') {
            vs += 1;
            ve -= 1;
            val = &val[1..val.len() - 1];
            (Style::Quoted, Value::String(val.replace("\\\"", "\"")))
        } else if let Some(f) = hex_float(val) {
            (Style::HexFloat, serde_json::Number::from_f64(f).map_or_else(|| Value::String(val.into()), Value::Number))
        } else {
            (Style::Plain, text_to_value(val))
        };
        let id = leaves.len();
        leaves.push(Leaf { start: vs, end: ve, style, orig: val.to_string() });
        if let Some(open) = key.find('[') {
            // `name[3]: v` joins the array `name`; its count line stops being a leaf.
            let (name, idx) = (&key[..open], &key[open + 1..key.len().saturating_sub(1)]);
            if idx.is_empty() || idx.bytes().all(|b| b.is_ascii_digit()) {
                let slot = map.entry(name.to_string()).or_insert(Value::Null);
                if !slot.is_array() {
                    *slot = Value::Array(vec![]);
                    leafs.remove(&child("/\u{0}", name));
                }
                let arr = slot.as_array_mut().unwrap();
                arr.push(shown);
                leafs.insert(child(&child("/\u{0}", name), &(arr.len() - 1).to_string()), id);
                continue;
            }
        }
        match map.get(key) {
            Some(_) => {
                // A repeated plain key: keep the first.
                continue;
            }
            None => {
                map.insert(key.to_string(), shown);
                leafs.insert(child("/\u{0}", key), id);
            }
        }
    }
    finish(&mut unit, &mut root, &mut editable);
    if root.is_empty() {
        return Err("no units in this SII file".into());
    }
    let backend = SiiBackend { raw: text.as_bytes().to_vec(), leaves };
    Ok(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(backend)).with_note(
        "Euro Truck Simulator 2 / American Truck Simulator save. Money is in `bank`, experience and skills in `driver_player`. \
         Floats written as `&3f000000` are shown as numbers. Saved again in the game's own packed form when it came packed. Turn off Steam Cloud first, or the old save may come back.",
    ))
}

fn start_unit(unit: &mut Option<(String, Map<String, Value>, HashMap<String, usize>)>, counts: &HashMap<String, usize>, ty: String, name: String) {
    let key = if counts.get(&ty).copied().unwrap_or(0) <= 1 { ty } else { format!("{ty} : {name}") };
    *unit = Some((key, Map::new(), HashMap::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "SiiNunit\n{\neconomy : _nameless.7646.a03e.f410 {\n bank: _nameless.7646.a028.4150\n companies: 2\n companies[0]: company.volatile.bhv.berlin\n companies[1]: company.volatile.eurogoodies.paris\n}\n\nbank : _nameless.7646.a028.4150 {\n money_account: 100\n loan_limit: 0\n}\n\ndriver_player : driver.125 {\n experience_points: 0\n adr: 0\n long_dist: 0\n}\n\nvehicle : _nameless.4e43.34c0 {\n license_plate: \"ABC-1234\"\n odometer: 45231\n fuel_relative: &3f000000\n}\n\nvehicle : _nameless.4e43.34c1 {\n license_plate: \"XYZ\"\n}\n}\n";

    #[test]
    fn reads_and_edits() {
        let mut doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        assert_eq!(doc.view.pointer("/bank/money_account"), Some(&100.into()));
        assert_eq!(doc.view.pointer("/economy/companies/1"), Some(&"company.volatile.eurogoodies.paris".into()));
        assert_eq!(doc.view.pointer("/driver_player/experience_points"), Some(&0.into()));
        let v = doc.view.pointer("/vehicle : _nameless.4e43.34c0/fuel_relative").unwrap().as_f64().unwrap();
        assert!((v - 0.5).abs() < 1e-9);
        doc.set("/bank/money_account", 99_000_000.into()).unwrap();
        doc.set("/driver_player/long_dist", 6.into()).unwrap();
        doc.set("/vehicle : _nameless.4e43.34c0/fuel_relative", serde_json::json!(1.0)).unwrap();
        doc.set("/vehicle : _nameless.4e43.34c0/license_plate", "NEW-1".into()).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("money_account: 99000000"));
        assert!(out.contains("long_dist: 6"));
        assert!(out.contains("fuel_relative: &3f800000"));
        assert!(out.contains("license_plate: \"NEW-1\""));
        assert!(out.contains("companies: 2"), "array counts are untouched");
    }

    #[test]
    fn braces_on_their_own_line() {
        let t = "SiiNunit\n{\n\nbank: .bank\n{\n\tmoney_account: 0\n\tapp_enabled: false\n}\n\n@include \"x.sui\"\n}\n";
        let doc = parse(t).unwrap();
        assert_eq!(doc.view.pointer("/bank/money_account"), Some(&0.into()));
        assert_eq!(doc.view.pointer("/bank/app_enabled"), Some(&false.into()));
    }

    #[test]
    fn scsc_round_trip() {
        let packed = encrypt(SAMPLE.as_bytes(), [9; 16]).unwrap();
        assert!(looks_encrypted(&packed));
        assert_eq!(decrypt(&packed).unwrap(), SAMPLE.as_bytes());
    }
}
