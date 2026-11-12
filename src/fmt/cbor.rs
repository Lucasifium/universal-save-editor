//! CBOR saves. There's no magic number, so a file is only accepted when the whole of it
//! decodes to one map with text keys and encoding that again gives back the same bytes
//! (so an edit changes only the values edited).

use crate::tree::{child, int_in, num, unique_key, Backend, TreeDoc};
use ciborium::Value as C;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct CborBackend {
    raw: Vec<u8>,
    tree: C,
    paths: Vec<Vec<usize>>,
}

fn get_mut<'a>(v: &'a mut C, path: &[usize]) -> Option<&'a mut C> {
    let mut cur = v;
    for &i in path {
        cur = match cur {
            C::Map(m) => &mut m.get_mut(i)?.1,
            C::Array(a) => a.get_mut(i)?,
            C::Tag(_, inner) if i == 0 => inner.as_mut(),
            _ => return None,
        };
    }
    Some(cur)
}

fn convert(orig: &C, v: &Value) -> Result<C, String> {
    Ok(match orig {
        C::Bool(_) => C::Bool(v.as_bool().ok_or("must be on/off")?),
        C::Text(_) => C::Text(v.as_str().ok_or("must be text")?.to_string()),
        C::Float(_) => C::Float(v.as_f64().ok_or("must be a number")?),
        C::Integer(_) => {
            let n = int_in(v, i64::MIN as i128, u64::MAX as i128, "this number")?;
            C::Integer(ciborium::value::Integer::try_from(n).map_err(|_| "out of range")?)
        }
        _ => return Err("this value can't be edited".into()),
    })
}

fn encode(v: &C) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    ciborium::into_writer(v, &mut out).ok()?;
    Some(out)
}

impl Backend for CborBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let mut tree = self.tree.clone();
        for (id, v) in changes {
            let slot = get_mut(&mut tree, &self.paths[*id]).ok_or("couldn't locate an edited value")?;
            *slot = convert(slot, v)?;
        }
        encode(&tree).ok_or_else(|| "couldn't write the CBOR document".into())
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let mut t = self.tree.clone();
        convert(get_mut(&mut t, &self.paths[leaf]).ok_or("missing value")?, v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

struct B {
    paths: Vec<Vec<usize>>,
    editable: HashMap<String, usize>,
}

impl B {
    fn view(&mut self, v: &C, ptr: &str, path: &mut Vec<usize>) -> Value {
        let leaf = |b: &mut Self, out: Value| {
            b.editable.insert(ptr.to_string(), b.paths.len());
            b.paths.push(path.clone());
            out
        };
        match v {
            C::Null => Value::Null,
            C::Bool(b) => leaf(self, Value::Bool(*b)),
            C::Integer(i) => {
                let n: i128 = (*i).into();
                leaf(self, i64::try_from(n).map(Value::from).or_else(|_| u64::try_from(n).map(Value::from)).unwrap_or(Value::Null))
            }
            C::Float(f) => leaf(self, num(*f)),
            C::Text(s) => leaf(self, Value::String(s.clone())),
            C::Bytes(b) => Value::String(format!("<{} bytes of data>", b.len())),
            C::Tag(t, inner) => {
                path.push(0);
                let r = self.view(inner, &child(ptr, &format!("tag {t}")), path);
                path.pop();
                let mut m = Map::new();
                m.insert(format!("tag {t}"), r);
                Value::Object(m)
            }
            C::Array(a) => Value::Array(
                a.iter()
                    .enumerate()
                    .map(|(i, c)| {
                        path.push(i);
                        let r = self.view(c, &format!("{ptr}/{i}"), path);
                        path.pop();
                        r
                    })
                    .collect(),
            ),
            C::Map(m) => {
                let mut out = Map::new();
                for (i, (k, c)) in m.iter().enumerate() {
                    let name = match k {
                        C::Text(s) => s.clone(),
                        C::Integer(n) => i128::from(*n).to_string(),
                        _ => "?".into(),
                    };
                    let key = unique_key(&out, &name);
                    path.push(i);
                    let r = self.view(c, &child(ptr, &key), path);
                    path.pop();
                    out.insert(key, r);
                }
                Value::Object(out)
            }
            _ => Value::Null,
        }
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    // A CBOR map starts with major type 5.
    if d.len() < 3 || d[0] >> 5 != 5 {
        return Err("not a CBOR document".into());
    }
    let tree: C = ciborium::from_reader(d).map_err(|e| e.to_string())?;
    let strict = matches!(&tree, C::Map(m) if m.len() >= 2 && m.iter().all(|(k, _)| k.is_text()));
    if !strict || encode(&tree).as_deref() != Some(d) {
        return Err("not a CBOR document".into());
    }
    let mut b = B { paths: vec![], editable: HashMap::new() };
    let view = b.view(&tree, "", &mut vec![]);
    let backend = CborBackend { raw: d.to_vec(), tree, paths: b.paths };
    Ok(TreeDoc::new(view, b.editable, "CBOR", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let tree = C::Map(vec![
            (C::Text("gold".into()), C::Integer(1500.into())),
            (C::Text("name".into()), C::Text("Ash".into())),
            (C::Text("speed".into()), C::Float(1.5)),
            (C::Text("flags".into()), C::Array(vec![C::Bool(true), C::Bool(false)])),
        ]);
        let raw = encode(&tree).unwrap();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        doc.set("/gold", Value::from(70000)).unwrap();
        doc.set("/flags/1", Value::Bool(true)).unwrap();
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(again.view.pointer("/gold").unwrap(), 70000);
        assert_eq!(again.view.pointer("/flags/1").unwrap(), true);
        assert!(parse(b"\x83\x01\x02\x03").is_err());
    }
}
