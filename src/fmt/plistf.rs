//! Apple property lists (binary `bplist00` and XML), used by Mac/iOS ports and some
//! Unity games. Types are preserved: an Integer stays an Integer, a Real stays a Real.

use crate::tree::{child, int_in, num, Backend, TreeDoc};
use plist::Value as P;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum Seg {
    Key(String),
    Idx(usize),
}

#[derive(Clone, Debug)]
struct PlistBackend {
    raw: Vec<u8>,
    tree: P,
    binary: bool,
    paths: Vec<Vec<Seg>>,
}

fn get_mut<'a>(v: &'a mut P, path: &[Seg]) -> Option<&'a mut P> {
    let mut cur = v;
    for s in path {
        cur = match (cur, s) {
            (P::Dictionary(d), Seg::Key(k)) => d.get_mut(k)?,
            (P::Array(a), Seg::Idx(i)) => a.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// Convert an edited value back to the plist type it had originally.
fn convert(orig: &P, v: &Value) -> Result<P, String> {
    Ok(match orig {
        P::Boolean(_) => P::Boolean(v.as_bool().ok_or("must be on/off")?),
        P::String(_) => P::String(v.as_str().ok_or("must be text")?.to_string()),
        P::Real(_) => P::Real(v.as_f64().ok_or("must be a number")?),
        P::Integer(i) if i.as_signed().is_none() => {
            let n = int_in(v, 0, u64::MAX as i128, "this number")?;
            P::Integer((n as u64).into())
        }
        P::Integer(_) => {
            let n = int_in(v, i64::MIN as i128, i64::MAX as i128, "this number")?;
            P::Integer((n as i64).into())
        }
        _ => return Err("this value can't be edited".into()),
    })
}

impl Backend for PlistBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let mut tree = self.tree.clone();
        for (id, v) in changes {
            let slot =
                get_mut(&mut tree, &self.paths[*id]).ok_or("couldn't locate an edited value")?;
            *slot = convert(slot, v)?;
        }
        let mut out = Vec::new();
        if self.binary {
            tree.to_writer_binary(&mut out).map_err(|e| e.to_string())?;
        } else {
            tree.to_writer_xml(&mut out).map_err(|e| e.to_string())?;
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let mut t = self.tree.clone();
        let orig = get_mut(&mut t, &self.paths[leaf]).ok_or("missing value")?;
        convert(orig, v).map(|_| ())
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
    fn view(&mut self, v: &P, ptr: &str, path: &mut Vec<Seg>) -> Value {
        let leaf = |b: &mut Self, path: &[Seg], out: Value| {
            b.editable.insert(ptr.to_string(), b.paths.len());
            b.paths.push(path.to_vec());
            out
        };
        match v {
            P::Dictionary(d) => {
                let mut m = Map::new();
                for (k, c) in d.iter() {
                    path.push(Seg::Key(k.clone()));
                    m.insert(k.clone(), self.view(c, &child(ptr, k), path));
                    path.pop();
                }
                Value::Object(m)
            }
            P::Array(a) => Value::Array(
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
            P::Boolean(b) => leaf(self, path, Value::Bool(*b)),
            P::String(s) => leaf(self, path, Value::String(s.clone())),
            P::Real(f) => leaf(self, path, num(*f)),
            P::Integer(i) => {
                let n = i
                    .as_signed()
                    .map(Value::from)
                    .or_else(|| i.as_unsigned().map(Value::from))
                    .unwrap_or(Value::Null);
                leaf(self, path, n)
            }
            P::Date(d) => Value::String(format!("{d:?}")),
            P::Data(d) => Value::String(format!("<{} bytes of data>", d.len())),
            P::Uid(u) => Value::String(format!("<uid {}>", u.get())),
            _ => Value::Null,
        }
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let tree = P::from_reader(std::io::Cursor::new(d)).map_err(|e| e.to_string())?;
    if !matches!(tree, P::Dictionary(_) | P::Array(_)) {
        return Err("not a property list document".into());
    }
    let binary = d.starts_with(b"bplist");
    let mut b = B {
        paths: vec![],
        editable: HashMap::new(),
    };
    let view = b.view(&tree, "", &mut vec![]);
    let name = if binary {
        "Property list (binary)"
    } else {
        "Property list (XML)"
    };
    let backend = PlistBackend {
        raw: d.to_vec(),
        tree,
        binary,
        paths: b.paths,
    };
    Ok(TreeDoc::new(view, b.editable, name, Box::new(backend)))
}
