//! Borderlands 4 saves (`*.sav`): AES-256-ECB under a key made from the Steam or Epic account id,
//! around a zlib stream, an Adler-32 and the length, holding a YAML document.
//! The key construction and trailer follow the layout documented by the
//! Borderlands4SaveDataResigner project (MIT).

use crate::tree::{child, insert_unique, text_to_value, value_to_text, Backend, Patches, TreeDoc};
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};

const PUBLIC_KEY: [u8; 32] = [
    0x35, 0xEC, 0x33, 0x77, 0xF3, 0x5D, 0xB0, 0xEA, 0xBE, 0x6B, 0x83, 0x11, 0x54, 0x03, 0xEB, 0xFB, 0x27, 0x25, 0x64, 0x2E, 0xD5, 0x49, 0x06, 0x29,
    0x05, 0x78, 0xBD, 0x60, 0xBA, 0x4A, 0xA7, 0x87,
];

/// The bytes of an account that are mixed into the key: a Steam id as 8 little-endian bytes, an
/// Epic account id (32 hex characters) as UTF-16LE text, of which the first 32 bytes count.
pub fn steam_id(id: u64) -> Vec<u8> {
    id.to_le_bytes().to_vec()
}

pub fn epic_id(id: &str) -> Vec<u8> {
    id.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn key_for(id: &[u8]) -> [u8; 32] {
    let mut k = PUBLIC_KEY;
    for (x, b) in k.iter_mut().zip(id) {
        *x ^= b;
    }
    k
}

fn adler32(d: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &c in d {
        a = (a + c as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn ecb(data: &[u8], id: &[u8], enc: bool) -> Vec<u8> {
    let c = Aes256::new(GenericArray::from_slice(&key_for(id)));
    let mut out = data.to_vec();
    for blk in out.chunks_exact_mut(16) {
        let b = GenericArray::from_mut_slice(blk);
        if enc {
            c.encrypt_block(b)
        } else {
            c.decrypt_block(b)
        }
    }
    out
}

fn unpack(body: &[u8]) -> Option<Vec<u8>> {
    if body.len() < 10 {
        return None;
    }
    let n = body.len();
    let len = u32::from_le_bytes(body[n - 4..].try_into().ok()?) as usize;
    let sum = u32::from_be_bytes(body[n - 8..n - 4].try_into().ok()?);
    let mut plain = Vec::new();
    flate2::read::ZlibDecoder::new(&body[..n - 8]).read_to_end(&mut plain).ok()?;
    // Some tools write the checksum little-endian; the length and a clean zlib stream still vouch for it.
    let a = adler32(&plain);
    (plain.len() == len && (a == sum || a == sum.swap_bytes())).then_some(plain)
}

/// The YAML text of a file the account `id` opens.
pub fn decrypt(data: &[u8], id: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 32 || data.len() % 16 != 0 {
        return None;
    }
    let first = ecb(&data[..16], id, false);
    if first[0] != 0x78 {
        return None;
    }
    let dec = ecb(data, id, false);
    let pad = *dec.last()? as usize;
    if (1..=16).contains(&pad) && dec.len() > pad && dec[dec.len() - pad..].iter().all(|&b| b as usize == pad) {
        if let Some(p) = unpack(&dec[..dec.len() - pad]) {
            return Some(p);
        }
    }
    unpack(&dec)
}

/// Accounts named by the folders of the path: Steam ids (17 digits) and Epic account ids
/// (32 hex characters, tried as written and in lower case).
pub fn ids_in_path(path: &std::path::Path) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for s in path.components().filter_map(|c| c.as_os_str().to_str()) {
        if let Some(n) = s.parse::<u64>().ok().filter(|n| *n > 76_561_190_000_000_000) {
            out.push(steam_id(n));
        } else if s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
            out.push(epic_id(s));
            out.push(epic_id(&s.to_ascii_lowercase()));
        }
    }
    out
}

/// What a typed-in id means: digits are a Steam id, anything else an Epic account id.
pub fn typed_id(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else if let Ok(n) = s.parse::<u64>() {
        Some(steam_id(n))
    } else if s.len() >= 10 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Some(epic_id(s))
    } else {
        None
    }
}

pub fn find(data: &[u8], accounts: &[Vec<u8>]) -> Option<(Vec<u8>, Vec<u8>)> {
    accounts.iter().find_map(|a| decrypt(data, a).map(|p| (a.clone(), p)))
}

pub fn encrypt(plain: &[u8], id: &[u8]) -> Result<Vec<u8>, String> {
    let text = String::from_utf8_lossy(plain).replace("\r\n", "\n");
    let plain = text.as_bytes();
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(plain).map_err(|e| e.to_string())?;
    let mut body = z.finish().map_err(|e| e.to_string())?;
    body.extend(adler32(plain).to_be_bytes());
    body.extend((plain.len() as u32).to_le_bytes());
    let pad = 16 - body.len() % 16;
    body.extend(std::iter::repeat(pad as u8).take(pad));
    Ok(ecb(&body, id, true))
}

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    single: bool,
    orig: String,
}

#[derive(Clone, Debug)]
struct YamlBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for YamlBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            let mut t = value_to_text(&l.orig, v);
            if l.single {
                t = t.replace('\'', "''");
            }
            p.add(l.start, l.end, t.into_bytes());
        }
        p.apply(&self.raw)
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.as_str().is_some_and(|s| s.contains(['\n', '\r'])) {
            return Err("This value can't contain line breaks".into());
        }
        Ok(())
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

struct Line<'a> {
    indent: usize,
    off: usize,
    text: &'a str,
}

fn split_key(t: &str) -> Option<(&str, &str)> {
    let i = t.find(':')?;
    let (k, rest) = (&t[..i], &t[i + 1..]);
    if k.is_empty() || k.starts_with(['\'', '"', '[', '{']) || k.contains(' ') && !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    (rest.is_empty() || rest.starts_with(' ')).then_some((k, rest))
}

struct P<'a> {
    lines: Vec<Line<'a>>,
    pos: usize,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
}

impl P<'_> {
    fn scalar(&mut self, off: usize, rest: &str, ptr: &str) -> Value {
        let lead = rest.len() - rest.trim_start().len();
        let val = rest.trim();
        let start = off + lead;
        let end = start + val.len();
        let (v, single, s, e, orig) = if val.len() >= 2 && val.starts_with('\'') && val.ends_with('\'') {
            let inner = val[1..val.len() - 1].replace("''", "'");
            (Value::String(inner.clone()), true, start + 1, end - 1, inner)
        } else if val.starts_with(['"', '[', '{', '|', '>', '&', '*']) {
            return Value::String(val.to_string());
        } else {
            (text_to_value(val), false, start, end, val.to_string())
        };
        self.editable.insert(ptr.to_string(), self.leaves.len());
        self.leaves.push(Leaf { start: s, end: e, single, orig });
        v
    }

    /// A block of lines at `indent`: a mapping, or a sequence.
    fn block(&mut self, indent: usize, ptr: &str, virt: Option<(usize, usize, &str)>) -> Value {
        let first_is_seq = virt.is_none() && self.lines.get(self.pos).is_some_and(|l| l.indent == indent && l.text.starts_with("- "));
        if first_is_seq {
            return self.seq(indent, ptr);
        }
        let mut map = Map::new();
        let mut virt = virt;
        loop {
            let (ind, off, text) = if let Some(v) = virt.take() {
                (v.0, v.1, v.2)
            } else {
                match self.lines.get(self.pos) {
                    Some(l) if l.indent == indent && !l.text.starts_with("- ") => {
                        let r = (l.indent, l.off, l.text);
                        self.pos += 1;
                        r
                    }
                    _ => break,
                }
            };
            let _ = ind;
            let Some((k, rest)) = split_key(text) else { continue };
            let key = insert_unique(&mut map, k, Value::Null);
            let p = child(ptr, &key);
            let rest_t = rest.trim();
            let v = if rest_t.is_empty() || rest_t.starts_with("!") {
                match self.lines.get(self.pos) {
                    Some(l) if l.indent > indent || (l.indent == indent && l.text.starts_with("- ")) => {
                        let ni = l.indent;
                        self.block(ni, &p, None)
                    }
                    _ => Value::Null,
                }
            } else {
                self.scalar(off + k.len() + 1, rest, &p)
            };
            map.insert(key, v);
        }
        Value::Object(map)
    }

    fn seq(&mut self, indent: usize, ptr: &str) -> Value {
        let mut arr = Vec::new();
        while let Some(l) = self.lines.get(self.pos) {
            if l.indent != indent || !l.text.starts_with("- ") {
                break;
            }
            let (off, text) = (l.off + 2, &l.text[2..]);
            let text = text.trim_start();
            let off = off + (l.text.len() - 2 - text.len());
            self.pos += 1;
            let p = child(ptr, &arr.len().to_string());
            if split_key(text).is_some() {
                let v = self.block(indent + 2, &p, Some((indent + 2, off, text)));
                arr.push(v);
            } else {
                let v = self.scalar(off, text, &p);
                arr.push(v);
            }
        }
        Value::Array(arr)
    }
}

/// Is this text a Borderlands 4 profile or character?
pub fn looks(t: &str) -> bool {
    let t = t.trim_start();
    (t.starts_with("state:") && t.contains("char_guid:")) || (t.starts_with("inputprefs:") && t.contains("onlineprefs:"))
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    let mut lines = Vec::new();
    let mut offset = 0;
    for l in text.split_inclusive('\n') {
        let start = offset;
        offset += l.len();
        let body = l.trim_end_matches(['\n', '\r']);
        let t = body.trim_start();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let indent = body.len() - t.len();
        lines.push(Line { indent, off: start + indent, text: t });
    }
    let mut p = P { lines, pos: 0, leaves: vec![], editable: HashMap::new() };
    let root = p.block(0, "", None);
    if !root.is_object() || root.as_object().is_some_and(|m| m.is_empty()) {
        return Err("not a Borderlands 4 save".into());
    }
    let backend = YamlBackend { raw: text.as_bytes().to_vec(), leaves: p.leaves };
    Ok(TreeDoc::new(root, p.editable, "Borderlands 4 save", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_edit() {
        let y = "state: \n  char_name: Vex\n  currencies: \n    cash: 29749\n    eridium: 51\n  experience: \n  - type: Character\n    level: 5\n  pips: !tags\n  - a\n";
        let mut doc = parse(y).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), y.as_bytes());
        assert_eq!(doc.view.pointer("/state/currencies/cash").unwrap(), 29749);
        assert_eq!(doc.view.pointer("/state/experience/0/level").unwrap(), 5);
        doc.set("/state/currencies/cash", Value::from(999)).unwrap();
        doc.set("/state/experience/0/level", Value::from(50)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, y.replace("29749", "999").replace("level: 5", "level: 50"));
        let me = steam_id(76561197960265729);
        let enc = encrypt(y.as_bytes(), &me).unwrap();
        assert_eq!(decrypt(&enc, &me).unwrap(), y.as_bytes());
        assert!(decrypt(&enc, &steam_id(76561197960265730)).is_none());
        let p = std::path::Path::new("C:/Users/x/Documents/My Games/Borderlands 4/Saved/SaveGames/76561197960265729/1.sav");
        assert_eq!(ids_in_path(p), vec![me.clone()]);
        let opts = crate::formats::Options::for_path(p);
        let d = crate::formats::decode_with(&enc, &opts);
        assert!(!d.layers.is_empty(), "the account in the path opens the file");
    }

    #[test]
    fn epic_account_ids_open_saves() {
        let y = "state: \n  char_guid: 1\n  char_name: Vex\n  currencies: \n    cash: 5\n";
        let id = "0123456789abcdef0123456789ABCDEF";
        let key = epic_id(id);
        assert_eq!(key.len(), 64);
        let enc = encrypt(y.as_bytes(), &key).unwrap();
        assert_eq!(decrypt(&enc, &key).unwrap(), y.as_bytes());
        assert!(decrypt(&enc, &steam_id(76561197960265729)).is_none());
        let p = std::path::Path::new("C:/Saved/SaveGames/0123456789abcdef0123456789ABCDEF/Profiles/client/1.sav");
        let d = crate::formats::decode_with(&enc, &crate::formats::Options::for_path(p));
        assert!(!d.layers.is_empty(), "the Epic id in the path opens the file");
        assert_eq!(typed_id("0123456789abcdef0123456789ABCDEF"), Some(key));
        assert_eq!(typed_id("76561197960265729"), Some(steam_id(76561197960265729)));
        assert_eq!(typed_id("bad id!"), None);
    }

    /// The resigner project's own sample files, if BL4_SAMPLES holds that folder.
    #[test]
    #[ignore]
    fn sample_files() {
        let dir = std::path::PathBuf::from(std::env::var("BL4_SAMPLES").expect("BL4_SAMPLES"));
        for (e, d) in [("1.sav", "1.yaml"), ("profile.sav", "profile.yaml")] {
            let enc = std::fs::read(dir.join("encrypted/76561197960265729").join(e)).unwrap();
            let plain = std::fs::read(dir.join("decrypted").join(d)).unwrap();
            let got = decrypt(&enc, &steam_id(76561197960265729)).unwrap();
            assert_eq!(String::from_utf8_lossy(&got).replace("\r\n", "\n"), String::from_utf8_lossy(&plain).replace("\r\n", "\n"));
            let doc = parse(&String::from_utf8_lossy(&got)).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), got);
        }
    }
}
