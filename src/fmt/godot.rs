//! Godot 4 binary Variant saves (`FileAccess.store_var` / `var_to_bytes`).
//!
//! A file is either one Variant, or a sequence of `store_var` records (u32 length +
//! Variant). Values are parsed into a typed tree and re-encoded the way Godot does
//! (ints/floats use 64-bit only when needed). Files are only accepted if re-encoding
//! them unchanged reproduces them byte-for-byte, so an unsupported variant of the
//! format is shown as hex instead of being saved wrongly.

use crate::tree::{child, int_in, num, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

const FLAG_64: u32 = 1 << 16;

#[derive(Clone, Debug, PartialEq)]
enum V {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    StringName(String),
    /// Fixed-size math types: (type id, flags, f64 components or i32 components)
    Floats(u32, bool, Vec<f64>),
    Ints(u32, Vec<i32>),
    Dict(Vec<(V, V)>),
    Array(Vec<V>),
    PackedBytes(Vec<u8>),
    PackedI32(Vec<i32>),
    PackedI64(Vec<i64>),
    PackedF32(Vec<f32>),
    PackedF64(Vec<f64>),
    PackedStr(Vec<String>),
    /// Packed vectors/colors: (type id, components per item, f32 values)
    PackedVec(u32, usize, Vec<f32>),
}

/// (type id, number of float components) for float-based math types.
const FLOAT_TYPES: &[(u32, usize)] = &[(5, 2), (7, 4), (9, 3), (11, 6), (12, 4), (14, 4), (15, 4), (16, 6), (17, 9), (18, 12), (19, 16)];
const INT_TYPES: &[(u32, usize)] = &[(6, 2), (8, 4), (10, 3), (13, 4)];
const COLOR: u32 = 20;

struct R<'a> {
    d: &'a [u8],
    pos: usize,
}

impl R<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.d.len()).ok_or("unexpected end of data")?;
        let s = &self.d[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn count(&mut self) -> Result<usize, String> {
        let n = (self.u32()? & 0x7FFF_FFFF) as usize;
        if n > self.d.len() {
            return Err("bad count".into());
        }
        Ok(n)
    }
    fn string(&mut self) -> Result<String, String> {
        let n = self.count()?;
        let s = String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "invalid text")?;
        self.take((4 - n % 4) % 4)?;
        Ok(s)
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn value(&mut self, depth: usize) -> Result<V, String> {
        if depth > 256 {
            return Err("nested too deep".into());
        }
        let header = self.u32()?;
        let ty = header & 0xFFFF;
        let flags = header & !0xFFFF;
        let wide = flags & FLAG_64 != 0;
        if flags & !FLAG_64 != 0 {
            return Err("unsupported Variant flags".into());
        }
        Ok(match ty {
            0 => V::Nil,
            1 => V::Bool(self.u32()? != 0),
            2 => V::Int(if wide { i64::from_le_bytes(self.take(8)?.try_into().unwrap()) } else { self.u32()? as i32 as i64 }),
            3 => V::Float(if wide { f64::from_le_bytes(self.take(8)?.try_into().unwrap()) } else { self.f32()? as f64 }),
            4 => V::Str(self.string()?),
            21 => V::StringName(self.string()?),
            COLOR => V::Floats(COLOR, false, (0..4).map(|_| self.f32().map(f64::from)).collect::<Result<_, _>>()?),
            27 => {
                let n = self.count()?;
                let mut out = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    let k = self.value(depth + 1)?;
                    let v = self.value(depth + 1)?;
                    out.push((k, v));
                }
                V::Dict(out)
            }
            28 => {
                let n = self.count()?;
                V::Array((0..n).map(|_| self.value(depth + 1)).collect::<Result<_, _>>()?)
            }
            29 => {
                let n = self.count()?;
                let b = self.take(n)?.to_vec();
                self.take((4 - n % 4) % 4)?;
                V::PackedBytes(b)
            }
            30 => {
                let n = self.count()?;
                V::PackedI32((0..n).map(|_| self.u32().map(|x| x as i32)).collect::<Result<_, _>>()?)
            }
            31 => {
                let n = self.count()?;
                V::PackedI64((0..n).map(|_| Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))).collect::<Result<_, String>>()?)
            }
            32 => {
                let n = self.count()?;
                V::PackedF32((0..n).map(|_| self.f32()).collect::<Result<_, _>>()?)
            }
            33 => {
                let n = self.count()?;
                V::PackedF64((0..n).map(|_| Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))).collect::<Result<_, String>>()?)
            }
            34 => {
                let n = self.count()?;
                V::PackedStr((0..n).map(|_| self.string()).collect::<Result<_, _>>()?)
            }
            35 | 36 | 37 | 38 => {
                let per = match ty {
                    35 => 2,
                    36 => 3,
                    _ => 4,
                };
                let n = self.count()?;
                V::PackedVec(ty, per, (0..n * per).map(|_| self.f32()).collect::<Result<_, _>>()?)
            }
            t => {
                if let Some(&(_, n)) = FLOAT_TYPES.iter().find(|(id, _)| *id == t) {
                    let comps = (0..n)
                        .map(|_| if wide { Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap())) } else { self.f32().map(f64::from) })
                        .collect::<Result<_, String>>()?;
                    V::Floats(t, wide, comps)
                } else if let Some(&(_, n)) = INT_TYPES.iter().find(|(id, _)| *id == t) {
                    V::Ints(t, (0..n).map(|_| self.u32().map(|x| x as i32)).collect::<Result<_, _>>()?)
                } else {
                    return Err(format!("unsupported Variant type {t}"));
                }
            }
        })
    }
}

fn put_string(out: &mut Vec<u8>, s: &str) {
    out.extend((s.len() as u32).to_le_bytes());
    out.extend(s.as_bytes());
    out.extend(std::iter::repeat_n(0, (4 - s.len() % 4) % 4));
}

fn encode(v: &V, out: &mut Vec<u8>) {
    let hdr = |out: &mut Vec<u8>, t: u32| out.extend(t.to_le_bytes());
    match v {
        V::Nil => hdr(out, 0),
        V::Bool(b) => {
            hdr(out, 1);
            out.extend((*b as u32).to_le_bytes());
        }
        V::Int(i) => {
            if i32::try_from(*i).is_ok() {
                hdr(out, 2);
                out.extend((*i as i32).to_le_bytes());
            } else {
                hdr(out, 2 | FLAG_64);
                out.extend(i.to_le_bytes());
            }
        }
        V::Float(f) => {
            if (*f as f32) as f64 == *f || f.is_nan() {
                hdr(out, 3);
                out.extend((*f as f32).to_le_bytes());
            } else {
                hdr(out, 3 | FLAG_64);
                out.extend(f.to_le_bytes());
            }
        }
        V::Str(s) => {
            hdr(out, 4);
            put_string(out, s);
        }
        V::StringName(s) => {
            hdr(out, 21);
            put_string(out, s);
        }
        V::Floats(t, wide, comps) => {
            hdr(out, t | if *wide { FLAG_64 } else { 0 });
            for c in comps {
                if *wide {
                    out.extend(c.to_le_bytes());
                } else {
                    out.extend((*c as f32).to_le_bytes());
                }
            }
        }
        V::Ints(t, comps) => {
            hdr(out, *t);
            comps.iter().for_each(|c| out.extend(c.to_le_bytes()));
        }
        V::Dict(pairs) => {
            hdr(out, 27);
            out.extend((pairs.len() as u32).to_le_bytes());
            for (k, v) in pairs {
                encode(k, out);
                encode(v, out);
            }
        }
        V::Array(items) => {
            hdr(out, 28);
            out.extend((items.len() as u32).to_le_bytes());
            items.iter().for_each(|i| encode(i, out));
        }
        V::PackedBytes(b) => {
            hdr(out, 29);
            out.extend((b.len() as u32).to_le_bytes());
            out.extend(b);
            out.extend(std::iter::repeat_n(0, (4 - b.len() % 4) % 4));
        }
        V::PackedI32(a) => {
            hdr(out, 30);
            out.extend((a.len() as u32).to_le_bytes());
            a.iter().for_each(|x| out.extend(x.to_le_bytes()));
        }
        V::PackedI64(a) => {
            hdr(out, 31);
            out.extend((a.len() as u32).to_le_bytes());
            a.iter().for_each(|x| out.extend(x.to_le_bytes()));
        }
        V::PackedF32(a) => {
            hdr(out, 32);
            out.extend((a.len() as u32).to_le_bytes());
            a.iter().for_each(|x| out.extend(x.to_le_bytes()));
        }
        V::PackedF64(a) => {
            hdr(out, 33);
            out.extend((a.len() as u32).to_le_bytes());
            a.iter().for_each(|x| out.extend(x.to_le_bytes()));
        }
        V::PackedStr(a) => {
            hdr(out, 34);
            out.extend((a.len() as u32).to_le_bytes());
            a.iter().for_each(|s| put_string(out, s));
        }
        V::PackedVec(t, per, vals) => {
            hdr(out, *t);
            out.extend(((vals.len() / per) as u32).to_le_bytes());
            vals.iter().for_each(|x| out.extend(x.to_le_bytes()));
        }
    }
}

#[derive(Clone, Debug)]
struct GodotDoc {
    /// true: sequence of store_var records; false: a single bare Variant.
    records: bool,
    values: Vec<V>,
}

fn serialize(doc: &GodotDoc) -> Vec<u8> {
    let mut out = Vec::new();
    for v in &doc.values {
        let mut body = Vec::new();
        encode(v, &mut body);
        if doc.records {
            out.extend((body.len() as u32).to_le_bytes());
        }
        out.extend(body);
    }
    out
}

/// Path into the typed tree: indexes into dict entries / arrays / components.
#[derive(Clone, Debug)]
struct GodotBackend {
    doc: GodotDoc,
    paths: Vec<Vec<usize>>,
}

fn apply(v: &mut V, path: &[usize], new: &Value) -> Result<(), String> {
    match (v, path) {
        (V::Bool(b), []) => *b = new.as_bool().ok_or("must be on/off")?,
        (V::Int(i), []) => *i = int_in(new, i64::MIN as i128, i64::MAX as i128, "This number")? as i64,
        (V::Float(f), []) => *f = new.as_f64().ok_or("must be a number")?,
        (V::Str(s), []) => *s = new.as_str().ok_or("must be text")?.to_string(),
        (V::Floats(_, _, c), [i]) => *c.get_mut(*i).ok_or("bad index")? = new.as_f64().ok_or("must be a number")?,
        (V::Ints(_, c), [i]) => *c.get_mut(*i).ok_or("bad index")? = int_in(new, i32::MIN as i128, i32::MAX as i128, "This number")? as i32,
        (V::PackedBytes(a), [i]) => *a.get_mut(*i).ok_or("bad index")? = int_in(new, 0, 255, "A byte")? as u8,
        (V::PackedI32(a), [i]) => *a.get_mut(*i).ok_or("bad index")? = int_in(new, i32::MIN as i128, i32::MAX as i128, "This number")? as i32,
        (V::PackedI64(a), [i]) => *a.get_mut(*i).ok_or("bad index")? = int_in(new, i64::MIN as i128, i64::MAX as i128, "This number")? as i64,
        (V::PackedF32(a), [i]) => *a.get_mut(*i).ok_or("bad index")? = new.as_f64().ok_or("must be a number")? as f32,
        (V::PackedF64(a), [i]) => *a.get_mut(*i).ok_or("bad index")? = new.as_f64().ok_or("must be a number")?,
        (V::PackedStr(a), [i]) => *a.get_mut(*i).ok_or("bad index")? = new.as_str().ok_or("must be text")?.to_string(),
        (V::PackedVec(_, _, a), [i]) => *a.get_mut(*i).ok_or("bad index")? = new.as_f64().ok_or("must be a number")? as f32,
        (V::Dict(pairs), [i, rest @ ..]) => apply(&mut pairs.get_mut(*i).ok_or("bad index")?.1, rest, new)?,
        (V::Array(items), [i, rest @ ..]) => apply(items.get_mut(*i).ok_or("bad index")?, rest, new)?,
        _ => return Err("this value can't be edited".into()),
    }
    Ok(())
}

impl Backend for GodotBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut doc = self.doc.clone();
        for (id, v) in changes {
            let path = &self.paths[*id];
            apply(&mut doc.values[path[0]], &path[1..], v)?;
        }
        Ok(serialize(&doc))
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let path = &self.paths[leaf];
        let mut value = self.doc.values[path[0]].clone();
        apply(&mut value, &path[1..], v)
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn key_text(k: &V) -> String {
    match k {
        V::Str(s) | V::StringName(s) => s.clone(),
        V::Int(i) => i.to_string(),
        V::Float(f) => f.to_string(),
        V::Bool(b) => b.to_string(),
        V::Nil => "null".into(),
        _ => "<key>".into(),
    }
}

struct B {
    paths: Vec<Vec<usize>>,
    editable: HashMap<String, usize>,
}

impl B {
    fn leaf(&mut self, ptr: &str, path: &[usize], v: Value) -> Value {
        self.editable.insert(ptr.to_string(), self.paths.len());
        self.paths.push(path.to_vec());
        v
    }

    fn list(&mut self, ptr: &str, path: &mut Vec<usize>, vals: Vec<Value>) -> Value {
        Value::Array(
            vals.into_iter()
                .enumerate()
                .map(|(i, v)| {
                    path.push(i);
                    let r = self.leaf(&format!("{ptr}/{i}"), path, v);
                    path.pop();
                    r
                })
                .collect(),
        )
    }

    fn view(&mut self, v: &V, ptr: &str, path: &mut Vec<usize>) -> Value {
        match v {
            V::Nil => Value::Null,
            V::Bool(b) => self.leaf(ptr, path, Value::Bool(*b)),
            V::Int(i) => self.leaf(ptr, path, Value::from(*i)),
            V::Float(f) => self.leaf(ptr, path, num(*f)),
            V::Str(s) => self.leaf(ptr, path, Value::String(s.clone())),
            V::StringName(s) => Value::String(format!("&\"{s}\"")),
            V::Floats(_, _, c) => self.list(ptr, path, c.iter().map(|&x| num(x)).collect()),
            V::Ints(_, c) => self.list(ptr, path, c.iter().map(|&x| Value::from(x)).collect()),
            V::PackedBytes(a) => self.list(ptr, path, a.iter().map(|&x| Value::from(x)).collect()),
            V::PackedI32(a) => self.list(ptr, path, a.iter().map(|&x| Value::from(x)).collect()),
            V::PackedI64(a) => self.list(ptr, path, a.iter().map(|&x| Value::from(x)).collect()),
            V::PackedF32(a) => self.list(ptr, path, a.iter().map(|&x| num(x as f64)).collect()),
            V::PackedF64(a) => self.list(ptr, path, a.iter().map(|&x| num(x)).collect()),
            V::PackedStr(a) => self.list(ptr, path, a.iter().map(|x| Value::String(x.clone())).collect()),
            V::PackedVec(_, _, a) => self.list(ptr, path, a.iter().map(|&x| num(x as f64)).collect()),
            V::Array(items) => Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        path.push(i);
                        let r = self.view(c, &format!("{ptr}/{i}"), path);
                        path.pop();
                        r
                    })
                    .collect(),
            ),
            V::Dict(pairs) => {
                let mut m = Map::new();
                for (i, (k, c)) in pairs.iter().enumerate() {
                    let key = unique_key(&m, &key_text(k));
                    path.push(i);
                    let r = self.view(c, &child(ptr, &key), path);
                    path.pop();
                    m.insert(key, r);
                }
                Value::Object(m)
            }
        }
    }
}

fn read_doc(d: &[u8]) -> Result<GodotDoc, String> {
    // A sequence of store_var records…
    let mut r = R { d, pos: 0 };
    let mut values = Vec::new();
    let as_records = (|| -> Result<Vec<V>, String> {
        while r.pos < d.len() {
            let len = r.u32()? as usize;
            let end = r.pos + len;
            let v = r.value(0)?;
            if r.pos != end {
                return Err("record length mismatch".into());
            }
            values.push(v);
        }
        Ok(std::mem::take(&mut values))
    })();
    if let Ok(vals) = as_records {
        if !vals.is_empty() {
            return Ok(GodotDoc { records: true, values: vals });
        }
    }
    // …or one bare Variant.
    let mut r = R { d, pos: 0 };
    let v = r.value(0)?;
    if r.pos != d.len() {
        return Err("extra data".into());
    }
    Ok(GodotDoc { records: false, values: vec![v] })
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if d.len() < 8 {
        return Err("too short".into());
    }
    let doc = read_doc(d)?;
    let structured = doc.values.iter().any(|v| matches!(v, V::Dict(_) | V::Array(_)));
    if !structured || serialize(&doc) != d {
        return Err("not a Godot Variant file".into());
    }
    let mut b = B { paths: vec![], editable: HashMap::new() };
    let view = if doc.values.len() == 1 {
        b.view(&doc.values[0], "", &mut vec![0])
    } else {
        let mut m = Map::new();
        for (i, v) in doc.values.iter().enumerate() {
            let key = format!("Value {}", i + 1);
            let r = b.view(v, &child("", &key), &mut vec![i]);
            m.insert(key, r);
        }
        Value::Object(m)
    };
    Ok(TreeDoc::new(view, b.editable, "Godot binary save (store_var)", Box::new(GodotBackend { doc, paths: b.paths })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_round_trip_and_edit() {
        let doc = GodotDoc {
            records: true,
            values: vec![
                V::Dict(vec![
                    (V::Str("gold".into()), V::Int(1500)),
                    (V::Str("name".into()), V::Str("Ash".into())),
                    (V::Str("pos".into()), V::Floats(5, false, vec![1.5, -2.0])),
                    (V::Str("big".into()), V::Int(1 << 40)),
                    (V::Str("ratio".into()), V::Float(0.1)),
                    (V::Str("inv".into()), V::PackedI32(vec![3, 4])),
                ]),
                V::Int(42),
            ],
        };
        let raw = serialize(&doc);
        let mut t = parse(&raw).unwrap();
        assert_eq!(t.to_bytes().unwrap(), raw);
        assert_eq!(t.view.pointer("/Value 1/ratio").unwrap().as_f64(), Some(0.1), "doubles kept exactly");
        t.set("/Value 1/gold", Value::from(3_000_000_000_i64)).unwrap();
        t.set("/Value 1/name", Value::from("Misty")).unwrap();
        t.set("/Value 1/pos/1", Value::from(8.25)).unwrap();
        t.set("/Value 1/inv/0", Value::from(9)).unwrap();
        let again = parse(&t.to_bytes().unwrap()).unwrap();
        assert_eq!(again.view.pointer("/Value 1/gold").unwrap(), 3_000_000_000_i64);
        assert_eq!(again.view.pointer("/Value 1/name").unwrap(), "Misty");
        assert_eq!(again.view.pointer("/Value 1/pos/1").unwrap().as_f64(), Some(8.25));
        assert_eq!(again.view.pointer("/Value 1/inv/0").unwrap(), 9);
    }
}
