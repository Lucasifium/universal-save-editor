//! MessagePack saves. There's no magic number, so a file is only accepted when the
//! whole of it decodes to a single map with text keys.

use crate::tree::{child, int_in, num, unique_key, Backend, TreeDoc};
use rmpv::Value as M;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct MsgpackBackend {
    raw: Vec<u8>,
    tree: M,
    /// Path of indices: map entry index or array index at each level.
    paths: Vec<Vec<usize>>,
}

fn get_mut<'a>(v: &'a mut M, path: &[usize]) -> Option<&'a mut M> {
    let mut cur = v;
    for &i in path {
        cur = match cur {
            M::Map(m) => &mut m.get_mut(i)?.1,
            M::Array(a) => a.get_mut(i)?,
            _ => return None,
        };
    }
    Some(cur)
}

fn convert(orig: &M, v: &Value) -> Result<M, String> {
    Ok(match orig {
        M::Boolean(_) => M::Boolean(v.as_bool().ok_or("must be on/off")?),
        M::String(_) => M::from(v.as_str().ok_or("must be text")?),
        M::F32(_) => M::F32(v.as_f64().ok_or("must be a number")? as f32),
        M::F64(_) => M::F64(v.as_f64().ok_or("must be a number")?),
        M::Integer(_) => {
            let n = int_in(v, i64::MIN as i128, u64::MAX as i128, "this number")?;
            if n < 0 { M::from(n as i64) } else { M::from(n as u64) }
        }
        _ => return Err("this value can't be edited".into()),
    })
}

impl Backend for MsgpackBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let mut tree = self.tree.clone();
        for (id, v) in changes {
            let slot = get_mut(&mut tree, &self.paths[*id]).ok_or("couldn't locate an edited value")?;
            *slot = convert(slot, v)?;
        }
        let mut out = Vec::new();
        rmpv::encode::write_value(&mut out, &tree).map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if {
            let mut t = self.tree.clone();
            matches!(get_mut(&mut t, &self.paths[leaf]), Some(M::F32(_)))
        } {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let mut t = self.tree.clone();
        convert(get_mut(&mut t, &self.paths[leaf]).ok_or("missing value")?, v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn key_text(k: &M) -> String {
    match k {
        M::String(s) => s.as_str().unwrap_or("?").to_string(),
        other => other.to_string(),
    }
}

struct B {
    paths: Vec<Vec<usize>>,
    editable: HashMap<String, usize>,
}

impl B {
    fn view(&mut self, v: &M, ptr: &str, path: &mut Vec<usize>) -> Value {
        let leaf = |b: &mut Self, out: Value| {
            b.editable.insert(ptr.to_string(), b.paths.len());
            b.paths.push(path.clone());
            out
        };
        match v {
            M::Nil => Value::Null,
            M::Boolean(b) => leaf(self, Value::Bool(*b)),
            M::Integer(i) => leaf(self, i.as_i64().map(Value::from).or_else(|| i.as_u64().map(Value::from)).unwrap_or(Value::Null)),
            M::F32(f) => leaf(self, crate::tree::f32_value(*f)),
            M::F64(f) => leaf(self, num(*f)),
            M::String(s) => match s.as_str() {
                Some(t) => leaf(self, Value::String(t.to_string())),
                None => Value::String("<invalid text>".into()),
            },
            M::Binary(b) => Value::String(format!("<{} bytes of data>", b.len())),
            M::Ext(t, b) => Value::String(format!("<extension {t}: {} bytes>", b.len())),
            M::Array(a) => Value::Array(
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
            M::Map(m) => {
                let mut out = Map::new();
                for (i, (k, c)) in m.iter().enumerate() {
                    let key = unique_key(&out, &key_text(k));
                    path.push(i);
                    let r = self.view(c, &child(ptr, &key), path);
                    path.pop();
                    out.insert(key, r);
                }
                Value::Object(out)
            }
        }
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut rd = d;
    let tree = rmpv::decode::read_value(&mut rd).map_err(|e| e.to_string())?;
    let strict = rd.is_empty() && matches!(&tree, M::Map(m) if m.len() >= 2 && m.iter().all(|(k, _)| k.is_str()));
    if !strict {
        return Err("not a MessagePack document".into());
    }
    let mut b = B { paths: vec![], editable: HashMap::new() };
    let view = b.view(&tree, "", &mut vec![]);
    let backend = MsgpackBackend { raw: d.to_vec(), tree, paths: b.paths };
    Ok(TreeDoc::new(view, b.editable, "MessagePack", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let tree = M::Map(vec![
            (M::from("gold"), M::from(1500)),
            (M::from("name"), M::from("Ash")),
            (M::from("speed"), M::F32(1.5)),
            (M::from("flags"), M::Array(vec![M::Boolean(true), M::Boolean(false)])),
        ]);
        let mut raw = Vec::new();
        rmpv::encode::write_value(&mut raw, &tree).unwrap();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        doc.set("/gold", Value::from(70000)).unwrap();
        doc.set("/flags/1", Value::Bool(true)).unwrap();
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(again.view.pointer("/gold").unwrap(), 70000);
        assert_eq!(again.view.pointer("/flags/1").unwrap(), true);
        assert!(parse(b"\x93\x01\x02\x03").is_err(), "a bare array isn't accepted");
    }
}
