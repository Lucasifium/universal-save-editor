//! Fixed-size values at known places in a binary file: a little builder for the view and
//! a backend that writes them back in place, then fixes the file's checksum.
//!
//! Used by formats that are read with a hand-written walker (Mass Effect, Final Fantasy
//! VII, XCOM…): the walker finds where each value lives; this module edits it.

use crate::tree::{esc, int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    U8,
    I16,
    U16,
    I32,
    U32,
    /// Unsigned 64-bit (money in some games).
    U64,
    F32,
    /// A 32-bit true/false.
    B32,
    /// A one-byte true/false.
    B8,
}

impl Kind {
    pub fn size(self) -> usize {
        match self {
            Kind::U8 | Kind::B8 => 1,
            Kind::I16 | Kind::U16 => 2,
            Kind::U64 => 8,
            _ => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Leaf {
    pub at: usize,
    pub kind: Kind,
    pub big_endian: bool,
    /// Lowest and highest value the game accepts (on top of the type's own range).
    pub range: Option<(i64, i64)>,
    /// Names for the values 0, 1, 2… (shown as a drop-down).
    pub names: Option<Arc<Vec<String>>>,
    /// Other places that hold a copy of the same value (written too).
    pub also: Vec<usize>,
}

/// How the file's checksum is kept right after an edit.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Checksum {
    None,
    /// CRC-32/BZIP2 of everything before the last four bytes, stored there (little-endian).
    TrailingBzip2,
    /// Custom: called with the whole file after the values are written.
    Custom(fn(&mut Vec<u8>)),
}

pub fn read(d: &[u8], at: usize, kind: Kind, be: bool) -> Option<Value> {
    let b = d.get(at..at + kind.size())?;
    let u = |n: usize| -> u64 {
        let mut x = 0u64;
        for i in 0..n {
            let byte = if be { b[i] } else { b[n - 1 - i] };
            x = (x << 8) | u64::from(byte);
        }
        x
    };
    Some(match kind {
        Kind::U8 => Value::from(b[0]),
        Kind::I16 => Value::from(u(2) as u16 as i16),
        Kind::U16 => Value::from(u(2) as u16),
        Kind::I32 => Value::from(u(4) as u32 as i32),
        Kind::U32 => Value::from(u(4) as u32),
        Kind::U64 => Value::from(u(8)),
        Kind::F32 => crate::tree::round_f32(Value::from(f64::from(f32::from_bits(u(4) as u32)))),
        Kind::B32 => Value::Bool(u(4) != 0),
        Kind::B8 => Value::Bool(b[0] != 0),
    })
}

fn encode(l: &Leaf, v: &Value) -> Result<Vec<u8>, String> {
    let n: u64 = match l.kind {
        Kind::F32 => {
            let f = v.as_f64().filter(|f| f.is_finite()).ok_or("expected a number")?;
            if let Some((lo, hi)) = l.range {
                if f < lo as f64 || f > hi as f64 {
                    return Err(format!("must be between {lo} and {hi}"));
                }
            }
            u64::from((f as f32).to_bits())
        }
        Kind::B32 | Kind::B8 => u64::from(v.as_bool().ok_or("tick or untick it")?),
        k => {
            let v = match (&l.names, v) {
                (Some(names), Value::String(s)) => Value::from(names.iter().position(|n| n == s).ok_or("pick one from the list")?),
                _ => v.clone(),
            };
            let (tmin, tmax): (i128, i128) = match k {
                Kind::U8 => (0, 255),
                Kind::I16 => (i16::MIN.into(), i16::MAX.into()),
                Kind::U16 => (0, u16::MAX.into()),
                Kind::I32 => (i32::MIN.into(), i32::MAX.into()),
                Kind::U64 => (0, u64::MAX.into()),
                _ => (0, u32::MAX.into()),
            };
            let (min, max) = l.range.map_or((tmin, tmax), |(a, b)| (i128::from(a).max(tmin), i128::from(b).min(tmax)));
            int_in(&v, min, max, "This value")? as u64
        }
    };
    let size = l.kind.size();
    let mut out: Vec<u8> = (0..size).map(|i| (n >> (8 * i)) as u8).collect();
    if l.big_endian {
        out.reverse();
    }
    Ok(out)
}

#[derive(Clone, Debug)]
struct FieldsBackend {
    data: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
    checksum: Checksum,
}

impl Backend for FieldsBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = self.data.as_ref().clone();
        for (leaf, v) in changes {
            let l = &self.leaves[*leaf];
            let bytes = encode(l, v)?;
            for at in std::iter::once(l.at).chain(l.also.iter().copied()) {
                out[at..at + l.kind.size()].copy_from_slice(&bytes);
            }
        }
        if !changes.is_empty() {
            match self.checksum {
                Checksum::None => {}
                Checksum::TrailingBzip2 => {
                    let n = out.len() - 4;
                    let crc = super::crc32_bzip2(&out[..n]);
                    out[n..].copy_from_slice(&crc.to_le_bytes());
                }
                Checksum::Custom(f) => f(&mut out),
            }
        }
        Ok(out)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if self.leaves[leaf].kind == Kind::F32 {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        encode(&self.leaves[leaf], v).map(|_| ())
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        self.leaves[leaf].names.clone()
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Builds the view as nested objects while recording where each editable value lives.
pub struct Builder<'a> {
    pub d: &'a [u8],
    pub big_endian: bool,
    view: Map<String, Value>,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
}

impl<'a> Builder<'a> {
    pub fn new(d: &'a [u8]) -> Self {
        Self { d, big_endian: false, view: Map::new(), leaves: vec![], editable: HashMap::new() }
    }

    fn slot(&mut self, path: &[&str]) -> Option<&mut Map<String, Value>> {
        let mut m = &mut self.view;
        for k in &path[..path.len() - 1] {
            m = m.entry(k.to_string()).or_insert_with(|| Value::Object(Map::new())).as_object_mut()?;
        }
        Some(m)
    }

    /// A read-only value.
    pub fn info(&mut self, path: &[&str], v: impl Into<Value>) {
        let last = path[path.len() - 1].to_string();
        if let Some(m) = self.slot(path) {
            m.insert(last, v.into());
        }
    }

    /// An editable value at `at`; returns it (None if it's outside the file).
    pub fn field(&mut self, path: &[&str], at: usize, kind: Kind) -> Option<Value> {
        self.field_with(path, at, kind, None, None)
    }

    pub fn ranged(&mut self, path: &[&str], at: usize, kind: Kind, lo: i64, hi: i64) -> Option<Value> {
        self.field_with(path, at, kind, Some((lo, hi)), None)
    }

    pub fn named(&mut self, path: &[&str], at: usize, kind: Kind, names: &[&str]) -> Option<Value> {
        let names: Arc<Vec<String>> = Arc::new(names.iter().map(|s| s.to_string()).collect());
        self.field_with(path, at, kind, Some((0, names.len() as i64 - 1)), Some(names))
    }

    fn field_with(&mut self, path: &[&str], at: usize, kind: Kind, range: Option<(i64, i64)>, names: Option<Arc<Vec<String>>>) -> Option<Value> {
        let raw = read(self.d, at, kind, self.big_endian)?;
        let shown = match (&names, raw.as_u64()) {
            (Some(n), Some(i)) if (i as usize) < n.len() => Value::String(n[i as usize].clone()),
            (Some(_), Some(_)) => {
                // An out-of-list value stays a plain number (and read-only).
                self.info(path, raw.clone());
                return Some(raw);
            }
            _ => raw.clone(),
        };
        let ptr: String = path.iter().map(|k| format!("/{}", esc(k))).collect();
        let id = self.leaves.len();
        self.leaves.push(Leaf { at, kind, big_endian: self.big_endian, range, names, also: vec![] });
        self.editable.insert(ptr, id);
        let last = path[path.len() - 1].to_string();
        self.slot(path)?.insert(last, shown);
        Some(raw)
    }

    /// Writes the value last added at `path` to `at` as well (a copy the game keeps).
    pub fn mirror(&mut self, path: &[&str], at: usize) {
        let ptr: String = path.iter().map(|k| format!("/{}", esc(k))).collect();
        if let Some(&id) = self.editable.get(&ptr) {
            if at + self.leaves[id].kind.size() <= self.d.len() {
                self.leaves[id].also.push(at);
            }
        }
    }

    pub fn finish(self, kind_name: &str, checksum: Checksum) -> TreeDoc {
        let backend = FieldsBackend { data: Arc::new(self.d.to_vec()), leaves: self.leaves, checksum };
        TreeDoc::new(Value::Object(self.view), self.editable, kind_name, Box::new(backend))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_in_place_and_fixes_crc() {
        let mut d = vec![1u8, 0, 0, 0, 0, 0, 0x80, 0x3F, 2, 0, 0, 0, 0];
        let n = d.len() - 4;
        let crc = crate::fmt::crc32_bzip2(&d[..n]);
        d[n..].copy_from_slice(&crc.to_le_bytes());
        let mut b = Builder::new(&d);
        assert_eq!(b.field(&["A", "n"], 0, Kind::I32), Some(Value::from(1)));
        b.field(&["A", "f"], 4, Kind::F32);
        b.named(&["mode"], 8, Kind::U8, &["Easy", "Normal", "Hard"]);
        let mut doc = b.finish("test", Checksum::TrailingBzip2);
        assert_eq!(doc.view.pointer("/mode"), Some(&Value::from("Hard")));
        assert!(doc.set("/mode", Value::from("Sideways")).is_err());
        doc.set("/A/n", Value::from(500)).unwrap();
        doc.set("/mode", Value::from("Easy")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[..4], &500i32.to_le_bytes());
        assert_eq!(out[8], 0);
        let crc = crate::fmt::crc32_bzip2(&out[..n]);
        assert_eq!(&out[n..], &crc.to_le_bytes());
    }
}
