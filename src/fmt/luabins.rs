//! Supergiant Games saves (Hades, Hades II `.sav`).
//!
//! `SGB1`, an Adler-32 checksum of everything after it, a header (version, location,
//! run counters, map names…), then the length-prefixed, LZ4-compressed game state in
//! "luabins" format (tagged Lua values: `T` table, `S` string, `N` number, `0`/`1`
//! booleans, `-` nil). The container layer lives in `formats.rs`; this module
//! handles the checksum/framing and edits luabins values in place.

use crate::tree::{child, insert_unique, num, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

// ------------------------------------------------------------------ container

pub fn adler32(d: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in d.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// Split a save into (header before the state length, decompressed state).
pub fn open(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if !d.starts_with(b"SGB1")
        || d.len() < 16
        || adler32(&d[8..]) != u32::from_le_bytes(d[4..8].try_into().ok()?)
    {
        return None;
    }
    // The state is the last field: a u32 length covering the rest of the file.
    let p = (8..d.len().min(1 << 16) - 4).find(|&p| {
        u32::from_le_bytes(d[p..p + 4].try_into().unwrap()) as usize == d.len() - p - 4
    })?;
    let body = &d[p + 4..];
    let mut out = vec![0u8; (body.len() * 40).clamp(1 << 20, 256 << 20)];
    let n = lz4_flex::block::decompress_into(body, &mut out).ok()?;
    out.truncate(n);
    Some((d[..p].to_vec(), out))
}

pub fn close(header: &[u8], state: &[u8]) -> Vec<u8> {
    let body = lz4_flex::block::compress(state);
    let mut out = header.to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    let sum = adler32(&out[8..]);
    out[4..8].copy_from_slice(&sum.to_le_bytes());
    out
}

// ------------------------------------------------------------------ luabins

#[derive(Clone, Copy, Debug)]
enum Kind {
    Num,
    Bool,
    Str,
}

#[derive(Clone, Debug)]
struct Span {
    start: usize,
    end: usize,
    kind: Kind,
}

struct P<'a> {
    d: &'a [u8],
    i: usize,
    spans: Vec<Span>,
    editable: HashMap<String, usize>,
    depth: usize,
}

impl P<'_> {
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.d.get(self.i..self.i + 4).ok_or("unexpected end")?;
        self.i += 4;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    }

    fn str_body(&mut self) -> Result<String, String> {
        let n = self.u32()? as usize;
        let b = self.d.get(self.i..self.i + n).ok_or("unexpected end")?;
        self.i += n;
        Ok(String::from_utf8_lossy(b).into_owned())
    }

    /// Read one value; `ptr` is where it goes in the view (None for table keys).
    fn value(&mut self, ptr: Option<&str>) -> Result<Value, String> {
        let start = self.i;
        let tag = *self.d.get(self.i).ok_or("unexpected end")?;
        self.i += 1;
        let (v, kind) = match tag {
            b'-' => (Value::Null, None),
            b'0' | b'1' => (Value::Bool(tag == b'1'), Some(Kind::Bool)),
            b'N' => {
                let b = self.d.get(self.i..self.i + 8).ok_or("unexpected end")?;
                self.i += 8;
                let f = f64::from_le_bytes(b.try_into().unwrap());
                let v = if f.fract() == 0.0 && f.abs() < 9e15 {
                    Value::from(f as i64)
                } else {
                    num(f)
                };
                (v, Some(Kind::Num))
            }
            b'S' => (Value::String(self.str_body()?), Some(Kind::Str)),
            b'T' => {
                self.depth += 1;
                if self.depth > 250 {
                    return Err("tables nested too deeply".into());
                }
                let n = self.u32()? as usize + self.u32()? as usize;
                let mut map = Map::new();
                for _ in 0..n {
                    let key = match self.value(None)? {
                        Value::String(s) => s,
                        Value::Null => "nil".into(),
                        other => other.to_string(),
                    };
                    let k = crate::tree::unique_key(&map, &key);
                    let p = ptr.map(|p| child(p, &k));
                    let v = self.value(p.as_deref())?;
                    insert_unique(&mut map, &key, v);
                }
                self.depth -= 1;
                return Ok(Value::Object(map));
            }
            _ => return Err(format!("unknown value tag {tag:#x}")),
        };
        if let (Some(p), Some(kind)) = (ptr, kind) {
            self.editable.insert(p.to_string(), self.spans.len());
            self.spans.push(Span {
                start,
                end: self.i,
                kind,
            });
        }
        Ok(v)
    }
}

#[derive(Clone, Debug)]
struct LuabinsBackend {
    raw: Vec<u8>,
    spans: Vec<Span>,
}

impl Backend for LuabinsBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (leaf, v) in changes {
            let s = &self.spans[*leaf];
            let bytes = match (s.kind, v) {
                (Kind::Num, Value::Number(n)) => {
                    [&[b'N'][..], &n.as_f64().ok_or("bad number")?.to_le_bytes()].concat()
                }
                (Kind::Bool, Value::Bool(b)) => vec![if *b { b'1' } else { b'0' }],
                (Kind::Str, Value::String(t)) => {
                    [&[b'S'][..], &(t.len() as u32).to_le_bytes(), t.as_bytes()].concat()
                }
                _ => return Err("this value has to keep its type".into()),
            };
            p.add(s.start, s.end, bytes);
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match (self.spans[leaf].kind, v) {
            (Kind::Num, Value::Number(_))
            | (Kind::Bool, Value::Bool(_))
            | (Kind::Str, Value::String(_)) => Ok(()),
            _ => Err("this value has to keep its type".into()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let count = *d.first().ok_or("empty")? as usize;
    let mut p = P {
        d,
        i: 1,
        spans: vec![],
        editable: HashMap::new(),
        depth: 0,
    };
    let mut root = Map::new();
    for n in 0..count {
        let key = if count == 1 {
            "Game state".to_string()
        } else {
            format!("Value {}", n + 1)
        };
        let v = p.value(Some(&child("", &key)))?;
        root.insert(key, v);
    }
    if p.i != d.len() {
        return Err("unexpected data after the game state".into());
    }
    let backend = LuabinsBackend {
        raw: d.to_vec(),
        spans: p.spans,
    };
    // Unwrap a single table so its fields become the top-level sections.
    let (view, editable) = match root.into_iter().next() {
        Some((k, Value::Object(m))) if count == 1 => {
            let prefix = child("", &k);
            let editable = p
                .editable
                .into_iter()
                .map(|(ptr, l)| (ptr[prefix.len()..].to_string(), l))
                .collect();
            (Value::Object(m), editable)
        }
        Some((k, v)) => (Value::Object(Map::from_iter([(k, v)])), p.editable),
        None => (Value::Object(Map::new()), p.editable),
    };
    Ok(TreeDoc::new(view, editable, "Supergiant game state", Box::new(backend)).with_note(
        "Close the game before saving. Search for a resource or upgrade by name (Ctrl+F) \u{2014} the game state is large.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(t: &str) -> Vec<u8> {
        [&[b'S'][..], &(t.len() as u32).to_le_bytes(), t.as_bytes()].concat()
    }

    #[test]
    fn edit_luabins_and_container() {
        let mut st = vec![1u8, b'T', 0, 0, 0, 0, 2, 0, 0, 0];
        st.extend(s("Gold"));
        st.push(b'N');
        st.extend(250f64.to_le_bytes());
        st.extend(s("Name"));
        st.extend(s("Mel"));
        let mut doc = parse(&st).unwrap();
        assert_eq!(doc.view.pointer("/Gold").unwrap(), 250);
        assert_eq!(doc.to_bytes().unwrap(), st);
        doc.set("/Gold", Value::from(9999)).unwrap();
        doc.set("/Name", Value::from("Melinoë")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(parse(&out).unwrap().view, doc.view);

        let mut header = b"SGB1\0\0\0\0".to_vec();
        header.extend_from_slice(&[18, 0, 1, 0]);
        let file = close(&header, &out);
        let (h, back) = open(&file).unwrap();
        assert_eq!((&h[8..], back), (&header[8..], out));
    }

    #[test]
    #[ignore]
    fn real_hades_saves() {
        let dir = std::env::var("HADES_SAVES").unwrap_or_else(|_| {
            format!(
                "{}\\Saved Games\\Hades II",
                std::env::var("USERPROFILE").unwrap()
            )
        });
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if !p.to_string_lossy().ends_with(".sav") {
                continue;
            }
            let d = std::fs::read(&p).unwrap();
            let Some((h, state)) = open(&d) else {
                println!("{} not a full save", p.display());
                continue;
            };
            let doc = parse(&state).unwrap();
            println!(
                "{} header {} state {} leaves {}",
                p.display(),
                h.len(),
                state.len(),
                doc.editable.len()
            );
            assert_eq!(doc.to_bytes().unwrap(), state);
            let again = open(&close(&h, &state)).unwrap();
            assert_eq!(again.1, state);
        }
    }
}
