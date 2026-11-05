//! TOML saves, edited with `toml_edit` so comments and formatting are preserved.

use crate::tree::{child, int_in, num, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use toml_edit::{DocumentMut, Item};

#[derive(Clone, Debug)]
enum Seg {
    Key(String),
    Idx(usize),
}

#[derive(Clone, Debug)]
struct TomlBackend {
    raw: String,
    paths: Vec<Vec<Seg>>,
}

fn to_toml(v: &Value) -> Result<toml_edit::Value, String> {
    Ok(match v {
        Value::Bool(b) => (*b).into(),
        Value::String(s) => s.as_str().into(),
        Value::Number(n) if n.as_i64().is_some() => n.as_i64().unwrap().into(),
        Value::Number(n) => n.as_f64().ok_or("bad number")?.into(),
        _ => return Err("unsupported value".into()),
    })
}

fn replace_keep_decor(v: &mut toml_edit::Value, new: &toml_edit::Value) {
    let decor = v.decor().clone();
    *v = new.clone();
    *v.decor_mut() = decor;
}

fn set_in_value(v: &mut toml_edit::Value, path: &[Seg], new: &toml_edit::Value) -> bool {
    let Some((first, rest)) = path.split_first() else {
        replace_keep_decor(v, new);
        return true;
    };
    match (v, first) {
        (toml_edit::Value::InlineTable(t), Seg::Key(k)) => t.get_mut(k).is_some_and(|v| set_in_value(v, rest, new)),
        (toml_edit::Value::Array(a), Seg::Idx(i)) => a.get_mut(*i).is_some_and(|v| set_in_value(v, rest, new)),
        _ => false,
    }
}

fn set_in_item(item: &mut Item, path: &[Seg], new: &toml_edit::Value) -> bool {
    match item {
        Item::Value(v) => set_in_value(v, path, new),
        Item::Table(t) => match path.split_first() {
            Some((Seg::Key(k), rest)) => t.get_mut(k).is_some_and(|i| set_in_item(i, rest, new)),
            _ => false,
        },
        Item::ArrayOfTables(a) => match path.split_first() {
            Some((Seg::Idx(i), rest)) => match (a.get_mut(*i), rest.split_first()) {
                (Some(t), Some((Seg::Key(k), rest))) => t.get_mut(k).is_some_and(|i| set_in_item(i, rest, new)),
                _ => false,
            },
            _ => false,
        },
        Item::None => false,
    }
}

impl Backend for TomlBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone().into_bytes());
        }
        let mut doc: DocumentMut = self.raw.parse().map_err(|e: toml_edit::TomlError| e.to_string())?;
        for (id, v) in changes {
            let new = to_toml(v)?;
            if !set_in_item(doc.as_item_mut(), &self.paths[*id], &new) {
                return Err("couldn't locate an edited value".into());
            }
        }
        Ok(doc.to_string().into_bytes())
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.as_u64().is_some_and(|u| u > i64::MAX as u64) {
            int_in(v, i64::MIN as i128, i64::MAX as i128, "TOML integers")?;
        }
        Ok(())
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
    fn value(&mut self, v: &toml_edit::Value, ptr: &str, path: &mut Vec<Seg>) -> Value {
        let leaf = |b: &mut Self, out: Value| {
            b.editable.insert(ptr.to_string(), b.paths.len());
            b.paths.push(path.clone());
            out
        };
        match v {
            toml_edit::Value::String(s) => leaf(self, Value::String(s.value().clone())),
            toml_edit::Value::Integer(i) => leaf(self, Value::from(*i.value())),
            toml_edit::Value::Float(f) => leaf(self, num(*f.value())),
            toml_edit::Value::Boolean(b) => leaf(self, Value::Bool(*b.value())),
            toml_edit::Value::Datetime(d) => Value::String(d.value().to_string()),
            toml_edit::Value::Array(a) => Value::Array(
                a.iter()
                    .enumerate()
                    .map(|(i, x)| {
                        path.push(Seg::Idx(i));
                        let r = self.value(x, &format!("{ptr}/{i}"), path);
                        path.pop();
                        r
                    })
                    .collect(),
            ),
            toml_edit::Value::InlineTable(t) => {
                let mut m = Map::new();
                for (k, x) in t.iter() {
                    path.push(Seg::Key(k.to_string()));
                    m.insert(k.to_string(), self.value(x, &child(ptr, k), path));
                    path.pop();
                }
                Value::Object(m)
            }
        }
    }

    fn item(&mut self, item: &Item, ptr: &str, path: &mut Vec<Seg>) -> Value {
        match item {
            Item::Value(v) => self.value(v, ptr, path),
            Item::Table(t) => {
                let mut m = Map::new();
                for (k, i) in t.iter() {
                    path.push(Seg::Key(k.to_string()));
                    m.insert(k.to_string(), self.item(i, &child(ptr, k), path));
                    path.pop();
                }
                Value::Object(m)
            }
            Item::ArrayOfTables(a) => Value::Array(
                a.iter()
                    .enumerate()
                    .map(|(i, t)| {
                        path.push(Seg::Idx(i));
                        let mut m = Map::new();
                        for (k, it) in t.iter() {
                            path.push(Seg::Key(k.to_string()));
                            m.insert(k.to_string(), self.item(it, &child(&format!("{ptr}/{i}"), k), path));
                            path.pop();
                        }
                        path.pop();
                        Value::Object(m)
                    })
                    .collect(),
            ),
            Item::None => Value::Null,
        }
    }
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    let doc: DocumentMut = text.parse().map_err(|e: toml_edit::TomlError| e.to_string())?;
    let mut b = B { paths: vec![], editable: HashMap::new() };
    let view = b.item(doc.as_item(), "", &mut vec![]);
    let backend = TomlBackend { raw: text.to_string(), paths: b.paths };
    Ok(TreeDoc::new(view, b.editable, "TOML", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_comments() {
        let src = "# save\n[player]\nname = \"Ash\" # hero\ngold = 100\nspeed = 1.5\n\n[[party]]\nhp = 10\n[[party]]\nhp = 20\n";
        let mut doc = parse(src).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), src.as_bytes());
        doc.set("/player/gold", Value::from(999)).unwrap();
        doc.set("/party/1/hp", Value::from(55)).unwrap();
        doc.set("/player/name", Value::from("Misty")).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, src.replace("gold = 100", "gold = 999").replace("hp = 20", "hp = 55").replace("\"Ash\"", "\"Misty\""));
    }
}
