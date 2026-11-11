//! BSON documents (binary JSON). Int32 stays Int32 (range-checked), etc.

use crate::tree::{child, int_in, num, Backend, TreeDoc};
use bson::{Bson, Document};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum Seg {
    Key(String),
    Idx(usize),
}

#[derive(Clone, Debug)]
struct BsonBackend {
    raw: Vec<u8>,
    doc: Document,
    paths: Vec<Vec<Seg>>,
}

fn get_mut<'a>(doc: &'a mut Document, path: &[Seg]) -> Option<&'a mut Bson> {
    let (Seg::Key(first), rest) = path.split_first()? else {
        return None;
    };
    let mut cur = doc.get_mut(first)?;
    for s in rest {
        cur = match (cur, s) {
            (Bson::Document(d), Seg::Key(k)) => d.get_mut(k)?,
            (Bson::Array(a), Seg::Idx(i)) => a.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(cur)
}

fn convert(orig: &Bson, v: &Value) -> Result<Bson, String> {
    Ok(match orig {
        Bson::Boolean(_) => Bson::Boolean(v.as_bool().ok_or("must be on/off")?),
        Bson::String(_) => Bson::String(v.as_str().ok_or("must be text")?.to_string()),
        Bson::Double(_) => Bson::Double(v.as_f64().ok_or("must be a number")?),
        Bson::Int32(_) => {
            Bson::Int32(int_in(v, i32::MIN as i128, i32::MAX as i128, "this number")? as i32)
        }
        Bson::Int64(_) => {
            Bson::Int64(int_in(v, i64::MIN as i128, i64::MAX as i128, "this number")? as i64)
        }
        _ => return Err("this value can't be edited".into()),
    })
}

impl Backend for BsonBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let mut doc = self.doc.clone();
        for (id, v) in changes {
            let slot =
                get_mut(&mut doc, &self.paths[*id]).ok_or("couldn't locate an edited value")?;
            *slot = convert(slot, v)?;
        }
        let mut out = Vec::new();
        doc.to_writer(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let mut d = self.doc.clone();
        convert(
            get_mut(&mut d, &self.paths[leaf]).ok_or("missing value")?,
            v,
        )
        .map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

struct B {
    paths: Vec<Vec<Seg>>,
    editable: HashMap<String, usize>,
}

impl B {
    fn doc(&mut self, d: &Document, ptr: &str, path: &mut Vec<Seg>) -> Value {
        let mut m = Map::new();
        for (k, v) in d {
            path.push(Seg::Key(k.clone()));
            m.insert(k.clone(), self.view(v, &child(ptr, k), path));
            path.pop();
        }
        Value::Object(m)
    }

    fn view(&mut self, v: &Bson, ptr: &str, path: &mut Vec<Seg>) -> Value {
        let leaf = |b: &mut Self, out: Value| {
            b.editable.insert(ptr.to_string(), b.paths.len());
            b.paths.push(path.clone());
            out
        };
        match v {
            Bson::Document(d) => self.doc(d, ptr, path),
            Bson::Array(a) => Value::Array(
                a.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        path.push(Seg::Idx(i));
                        let r = self.view(c, &format!("{ptr}/{i}"), path);
                        path.pop();
                        r
                    })
                    .collect(),
            ),
            Bson::Boolean(b) => leaf(self, Value::Bool(*b)),
            Bson::String(s) => leaf(self, Value::String(s.clone())),
            Bson::Double(f) => leaf(self, num(*f)),
            Bson::Int32(i) => leaf(self, Value::from(*i)),
            Bson::Int64(i) => leaf(self, Value::from(*i)),
            Bson::Null | Bson::Undefined => Value::Null,
            other => Value::String(other.to_string()),
        }
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 5
        && i32::from_le_bytes([d[0], d[1], d[2], d[3]]) as usize == d.len()
        && d.last() == Some(&0)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let doc = Document::from_reader(&mut std::io::Cursor::new(d)).map_err(|e| e.to_string())?;
    if doc.is_empty() {
        return Err("empty BSON document".into());
    }
    let mut b = B {
        paths: vec![],
        editable: HashMap::new(),
    };
    let view = b.doc(&doc, "", &mut vec![]);
    let backend = BsonBackend {
        raw: d.to_vec(),
        doc,
        paths: b.paths,
    };
    Ok(TreeDoc::new(view, b.editable, "BSON", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_types() {
        let doc = bson::doc! { "gold": 1500_i32, "xp": 123456789012_i64, "name": "Ash", "hp": 97.5, "alive": true };
        let mut raw = Vec::new();
        doc.to_writer(&mut raw).unwrap();
        assert!(looks(&raw));
        let mut t = parse(&raw).unwrap();
        assert_eq!(t.to_bytes().unwrap(), raw);
        assert!(
            t.set("/gold", Value::from(5_000_000_000_i64)).is_err(),
            "Int32 is range-checked"
        );
        t.set("/gold", Value::from(99)).unwrap();
        let out = Document::from_reader(&mut std::io::Cursor::new(t.to_bytes().unwrap())).unwrap();
        assert_eq!(out.get("gold"), Some(&Bson::Int32(99)));
        assert_eq!(out.get("xp"), Some(&Bson::Int64(123456789012)));
    }
}
