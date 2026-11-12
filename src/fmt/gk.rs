//! Graveyard Keeper saves (`*.dat`): a typed tree of values keyed by 32-bit name hashes, with a
//! table of strings (each byte XOR 0x6D) at the end.
//!
//! ```text
//! i64 string-table offset | i64 header offset | i32 version | 15 x i32 | tree | string table
//! tree   = i32 count, then count x (i32 hash, typed value)       (count -1: nothing)
//! typed  = u8 type, then the value; types 16-24 are short forms of 0, 1, (0,0), (1,1)…
//! ```
//!
//! The layout and the names of the hashes are as published by Graveyard-Keeper-Savefile-Editor
//! (NetroScript, MIT). Hashes it has no name for are shown by number. A change keeps each
//! value's type, except that a number that was stored as a short form (0 or 1) or an empty text
//! is stored in full when it changes.

use crate::tree::{child, f32_value, insert_unique, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

pub const KIND: &str = "Graveyard Keeper save";
const HEADER: usize = 80;

fn names() -> &'static HashMap<i32, &'static str> {
    static N: OnceLock<HashMap<i32, &'static str>> = OnceLock::new();
    N.get_or_init(|| {
        let lines: Vec<&str> = include_str!("data/gk_hashes.txt").lines().map(str::trim).collect();
        let mut m = HashMap::new();
        for p in lines.chunks(2) {
            if let [name, h] = p {
                if let (false, Ok(h)) = (name.is_empty(), h.parse::<i32>()) {
                    m.insert(h, *name);
                }
            }
        }
        m
    })
}

#[derive(Clone, Debug, PartialEq)]
struct Str {
    /// The bytes as the text uses them (already decoded for the string table).
    bytes: Vec<u8>,
    /// The length the file declares, in characters.
    n: i32,
}

impl Str {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
    fn from_text(t: &str) -> Self {
        Self { bytes: t.as_bytes().to_vec(), n: t.chars().count() as i32 }
    }
}

#[derive(Clone, Debug)]
enum Val {
    Null,
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    U8(u8),
    Char(u8),
    Str(Option<Str>),
    /// An entry of the string table: its index, and the text it had.
    Idx(usize, Option<Str>),
}

#[derive(Clone, Debug)]
enum Node {
    /// type 250; `None`: the count was -1.
    Obj(Option<Vec<(i32, Node)>>),
    List(u8, Vec<Node>),
    Bytes(Vec<u8>),
    /// A vector or quaternion (type 13, 14 or 15): the leaves of its components.
    Vec(u8, Vec<usize>),
    /// A value, with the type byte it was stored with.
    Leaf(usize),
}

struct Reader<'a> {
    d: &'a [u8],
    pos: usize,
    table: Vec<Option<Str>>,
    leaves: Vec<Val>,
    types: Vec<u8>,
}

fn dec(b: u8) -> u8 {
    if b == 0 || b == 109 {
        b
    } else {
        b ^ 0x6D
    }
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.d.get(self.pos..self.pos + n).ok_or("The save ends early")?;
        self.pos += n;
        Ok(s)
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn string(&mut self, encrypted: bool) -> Result<Option<Str>, String> {
        let n = self.i32()?;
        if n == -1 {
            return Ok(None);
        }
        if n < 0 || n as usize > self.d.len() {
            return Err("Bad text length".into());
        }
        let mut bytes = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let b = self.u8()?;
            let b = if encrypted { dec(b) } else { b };
            bytes.push(b);
            let extra = match b {
                0xC0..=0xDF => 1,
                0xE0..=0xEF => 2,
                0xF0..=0xF7 => 3,
                _ => 0,
            };
            for _ in 0..extra {
                let c = self.u8()?;
                bytes.push(if encrypted { dec(c) } else { c });
            }
        }
        Ok(Some(Str { bytes, n }))
    }
    fn leaf(&mut self, ty: u8, v: Val) -> Node {
        self.leaves.push(v);
        self.types.push(ty);
        Node::Leaf(self.leaves.len() - 1)
    }
    fn f32leaf(&mut self, v: f32) -> usize {
        self.leaves.push(Val::F32(v));
        self.types.push(5);
        self.leaves.len() - 1
    }
    fn object(&mut self, depth: usize) -> Result<Option<Vec<(i32, Node)>>, String> {
        if depth > 200 {
            return Err("Too deep".into());
        }
        let n = self.i32()?;
        if n == -1 {
            return Ok(None);
        }
        if n < 0 || n as usize > self.d.len() {
            return Err("Bad count".into());
        }
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let h = self.i32()?;
            let v = self.value(depth)?;
            out.push((h, v));
        }
        Ok(Some(out))
    }
    fn value(&mut self, depth: usize) -> Result<Node, String> {
        let ty = self.u8()?;
        Ok(match ty {
            250 => Node::Obj(self.object(depth + 1)?),
            0 => self.leaf(ty, Val::Null),
            1 => self.leaf(ty, Val::Bool(true)),
            2 => self.leaf(ty, Val::Bool(false)),
            3 => {
                let v = self.i32()?;
                self.leaf(ty, Val::I32(v))
            }
            4 => {
                let v = i64::from_le_bytes(self.take(8)?.try_into().unwrap());
                self.leaf(ty, Val::I64(v))
            }
            5 => {
                let v = f32::from_le_bytes(self.take(4)?.try_into().unwrap());
                self.leaf(ty, Val::F32(v))
            }
            6 => {
                let v = f64::from_le_bytes(self.take(8)?.try_into().unwrap());
                self.leaf(ty, Val::F64(v))
            }
            7 => {
                let v = self.u8()?;
                self.leaf(ty, Val::U8(v))
            }
            8 => {
                let v = self.u8()?;
                self.leaf(ty, Val::Char(v))
            }
            9 => {
                let s = self.string(false)?;
                self.leaf(ty, Val::Str(s))
            }
            10 => {
                let i = self.i32()?;
                let t = self.table.get(i as usize).ok_or("A text index is out of range")?.clone();
                self.leaf(ty, Val::Idx(i as usize, t))
            }
            11 => self.leaf(ty, Val::Str(Some(Str { bytes: vec![], n: 0 }))),
            16 => self.leaf(ty, Val::I32(0)),
            17 => self.leaf(ty, Val::I32(1)),
            18 => self.leaf(ty, Val::F32(0.0)),
            19 => self.leaf(ty, Val::F32(1.0)),
            13 | 14 | 15 | 20 | 21 | 22 | 23 | 24 => {
                let (base, count) = match ty {
                    13 | 20 | 21 => (13, 2),
                    14 | 22 | 23 => (14, 3),
                    _ => (15, 4),
                };
                let comps: Vec<f32> = match ty {
                    13 | 14 | 15 => (0..count).map(|_| self.take(4).map(|b| f32::from_le_bytes(b.try_into().unwrap()))).collect::<Result<_, _>>()?,
                    20 | 22 => vec![0.0; count],
                    21 | 23 => vec![1.0; count],
                    _ => vec![0.0, 0.0, 0.0, 1.0],
                };
                let ids = comps.into_iter().map(|c| self.f32leaf(c)).collect();
                Node::Vec(base, ids)
            }
            100 | 101 => {
                let n = self.i32()?;
                if n < 0 || n as usize > self.d.len() {
                    return Err("Bad list length".into());
                }
                let mut v = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    v.push(self.value(depth + 1)?);
                }
                Node::List(ty, v)
            }
            102 => {
                let n = self.i32()?;
                if n < 0 {
                    return Err("Bad byte array".into());
                }
                Node::Bytes(self.take(n as usize)?.to_vec())
            }
            _ => return Err(format!("Unknown value type {ty}")),
        })
    }
}

pub fn looks(d: &[u8]) -> bool {
    if d.len() < HEADER + 8 {
        return false;
    }
    let pos = i64::from_le_bytes(d[..8].try_into().unwrap());
    pos > HEADER as i64 && (pos as usize) < d.len() - 3 && i32::from_le_bytes(d[HEADER - 4 + 4..HEADER].try_into().unwrap_or([0; 4])) >= -1
}

struct Parsed {
    root: Node,
    leaves: Vec<Val>,
    types: Vec<u8>,
    table: Vec<Option<Str>>,
}

fn read_all(d: &[u8]) -> Result<Parsed, String> {
    if !looks(d) {
        return Err("not a Graveyard Keeper save".into());
    }
    let tpos = i64::from_le_bytes(d[..8].try_into().unwrap()) as usize;
    let mut r = Reader { d, pos: tpos, table: vec![], leaves: vec![], types: vec![] };
    let n = r.i32()?;
    if n < 0 || n as usize > d.len() {
        return Err("bad string table".into());
    }
    let mut table = Vec::with_capacity(n as usize);
    for _ in 0..n {
        table.push(r.string(true)?);
    }
    if r.pos != d.len() {
        return Err("data after the string table".into());
    }
    r.table = table;
    r.pos = HEADER;
    let root = r.object(0)?;
    if r.pos != tpos {
        return Err("the tree does not end where the string table starts".into());
    }
    Ok(Parsed { root: Node::Obj(root), leaves: r.leaves, types: r.types, table: r.table })
}

// ------------------------------------------------------------------ the tree shown

fn leaf_view(v: &Val) -> Value {
    match v {
        Val::Null => Value::Null,
        Val::Bool(b) => Value::Bool(*b),
        Val::I32(i) => Value::from(*i),
        Val::I64(i) => Value::from(*i),
        Val::F32(f) => f32_value(*f),
        Val::F64(f) => serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number),
        Val::U8(b) | Val::Char(b) => Value::from(*b),
        Val::Str(s) | Val::Idx(_, s) => s.as_ref().map_or(Value::Null, |s| Value::String(s.text())),
    }
}

fn editable_leaf(v: &Val) -> bool {
    !matches!(v, Val::Null | Val::Str(None) | Val::Idx(_, None))
}

fn label(h: i32) -> String {
    names().get(&h).map_or_else(|| h.to_string(), |s| s.to_string())
}

fn build(n: &Node, ptr: &str, p: &Parsed, edit: &mut HashMap<String, usize>) -> Value {
    match n {
        Node::Obj(None) => Value::Null,
        Node::Obj(Some(items)) => {
            let mut m = Map::new();
            for (h, v) in items {
                let k = unique_key(&m, &label(*h));
                let cp = child(ptr, &k);
                let val = build(v, &cp, p, edit);
                insert_unique(&mut m, &k, val);
            }
            Value::Object(m)
        }
        Node::List(_, items) => Value::Array(items.iter().enumerate().map(|(i, v)| build(v, &child(ptr, &i.to_string()), p, edit)).collect()),
        Node::Bytes(b) => Value::String(format!("({} bytes)", b.len())),
        Node::Vec(kind, ids) => {
            let names: &[&str] = match kind {
                13 => &["x", "y"],
                14 => &["x", "y", "z"],
                _ => &["x", "y", "z", "w"],
            };
            let mut m = Map::new();
            for (nm, id) in names.iter().zip(ids) {
                m.insert((*nm).to_string(), leaf_view(&p.leaves[*id]));
                edit.insert(child(ptr, nm), *id);
            }
            Value::Object(m)
        }
        Node::Leaf(id) => {
            if editable_leaf(&p.leaves[*id]) {
                edit.insert(ptr.to_string(), *id);
            }
            leaf_view(&p.leaves[*id])
        }
    }
}

// ------------------------------------------------------------------ writing

#[derive(Clone, Debug)]
struct GkBackend {
    head: Vec<u8>,
    root: Node,
    leaves: Vec<Val>,
    types: Vec<u8>,
    table: Vec<Option<Str>>,
}

struct Writer<'a> {
    out: Vec<u8>,
    leaves: &'a [Val],
    types: &'a [u8],
    table: Vec<Option<Str>>,
}

impl Writer<'_> {
    fn i32(&mut self, v: i32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn string(out: &mut Vec<u8>, s: &Option<Str>, encrypted: bool) {
        match s {
            None => out.extend_from_slice(&(-1i32).to_le_bytes()),
            Some(s) => {
                out.extend_from_slice(&s.n.to_le_bytes());
                out.extend(s.bytes.iter().map(|b| if encrypted { dec(*b) } else { *b }));
            }
        }
    }
    fn object(&mut self, o: &Option<Vec<(i32, Node)>>) {
        match o {
            None => self.i32(-1),
            Some(items) => {
                self.i32(items.len() as i32);
                for (h, v) in items {
                    self.i32(*h);
                    self.value(v);
                }
            }
        }
    }
    fn value(&mut self, n: &Node) {
        match n {
            Node::Obj(o) => {
                self.out.push(250);
                self.object(o);
            }
            Node::List(ty, items) => {
                self.out.push(*ty);
                self.i32(items.len() as i32);
                for i in items {
                    self.value(i);
                }
            }
            Node::Bytes(b) => {
                self.out.push(102);
                self.i32(b.len() as i32);
                self.out.extend_from_slice(b);
            }
            Node::Vec(kind, ids) => {
                let c: Vec<f32> = ids.iter().map(|i| if let Val::F32(f) = self.leaves[*i] { f } else { 0.0 }).collect();
                let all = |x: f32| c.iter().all(|v| *v == x);
                let ty = match kind {
                    13 if all(0.0) => 20,
                    13 if all(1.0) => 21,
                    14 if all(0.0) => 22,
                    14 if all(1.0) => 23,
                    15 if c == [0.0, 0.0, 0.0, 1.0] => 24,
                    k => *k,
                };
                self.out.push(ty);
                if matches!(ty, 13 | 14 | 15) {
                    for f in &c {
                        self.out.extend_from_slice(&f.to_le_bytes());
                    }
                }
            }
            Node::Leaf(id) => {
                let orig = self.types[*id];
                match &self.leaves[*id] {
                    Val::Null => self.out.push(0),
                    Val::Bool(b) => self.out.push(if *b { 1 } else { 2 }),
                    Val::I32(v) => {
                        let ty = if matches!(orig, 16 | 17) { if *v == 0 { 16 } else if *v == 1 { 17 } else { 3 } } else { 3 };
                        self.out.push(ty);
                        if ty == 3 {
                            self.i32(*v);
                        }
                    }
                    Val::I64(v) => {
                        self.out.push(4);
                        self.out.extend_from_slice(&v.to_le_bytes());
                    }
                    Val::F32(v) => {
                        let ty = if matches!(orig, 18 | 19) { if *v == 0.0 { 18 } else if *v == 1.0 { 19 } else { 5 } } else { 5 };
                        self.out.push(ty);
                        if ty == 5 {
                            self.out.extend_from_slice(&v.to_le_bytes());
                        }
                    }
                    Val::F64(v) => {
                        self.out.push(6);
                        self.out.extend_from_slice(&v.to_le_bytes());
                    }
                    Val::U8(v) => {
                        self.out.push(7);
                        self.out.push(*v);
                    }
                    Val::Char(v) => {
                        self.out.push(8);
                        self.out.push(*v);
                    }
                    Val::Str(s) => {
                        if orig == 11 && s.as_ref().is_some_and(|s| s.bytes.is_empty()) {
                            self.out.push(11);
                        } else {
                            self.out.push(9);
                            Self::string(&mut self.out, s, false);
                        }
                    }
                    Val::Idx(idx, s) => {
                        self.out.push(10);
                        let same = self.table.get(*idx).is_some_and(|t| t == s);
                        let at = if same {
                            *idx
                        } else if let Some(p) = self.table.iter().position(|t| t == s) {
                            p
                        } else {
                            self.table.push(s.clone());
                            self.table.len() - 1
                        };
                        self.i32(at as i32);
                    }
                }
            }
        }
    }
}

impl Backend for GkBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut leaves = self.leaves.clone();
        for (id, v) in changes {
            leaves[*id] = apply(&leaves[*id], v)?;
        }
        let mut w = Writer { out: self.head.clone(), leaves: &leaves, types: &self.types, table: self.table.clone() };
        w.value_root(&self.root);
        let tpos = w.out.len() as i64;
        w.out[..8].copy_from_slice(&tpos.to_le_bytes());
        let n = w.table.len() as i32;
        w.i32(n);
        let table = std::mem::take(&mut w.table);
        for s in &table {
            Writer::string(&mut w.out, s, true);
        }
        Ok(w.out)
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        apply(&self.leaves[leaf], v).map(|_| ())
    }
    fn normalize(&self, leaf: usize, v: Value) -> Value {
        match (&self.leaves[leaf], &v) {
            (Val::F32(_), Value::Number(n)) => n.as_f64().map_or(v.clone(), |f| f32_value(f as f32)),
            _ => v,
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

impl Writer<'_> {
    /// The root is written as a bare object (no type byte).
    fn value_root(&mut self, n: &Node) {
        if let Node::Obj(o) = n {
            self.object(o);
        }
    }
}

fn apply(old: &Val, v: &Value) -> Result<Val, String> {
    let whole = |what: &str| v.as_i64().ok_or_else(|| format!("{what} must be a whole number"));
    Ok(match old {
        Val::Bool(_) => Val::Bool(v.as_bool().ok_or("Must be true or false")?),
        Val::I32(_) => Val::I32(i32::try_from(whole("This value")?).map_err(|_| "Out of range for a 32-bit number")?),
        Val::I64(_) => Val::I64(whole("This value")?),
        Val::F32(_) => Val::F32(v.as_f64().ok_or("Must be a number")? as f32),
        Val::F64(_) => Val::F64(v.as_f64().ok_or("Must be a number")?),
        Val::U8(_) => Val::U8(u8::try_from(whole("This value")?).map_err(|_| "Must be 0 to 255")?),
        Val::Char(_) => Val::Char(u8::try_from(whole("This value")?).map_err(|_| "Must be 0 to 255")?),
        Val::Str(_) => Val::Str(Some(Str::from_text(v.as_str().ok_or("Must be text")?))),
        Val::Idx(i, _) => Val::Idx(*i, Some(Str::from_text(v.as_str().ok_or("Must be text")?))),
        Val::Null => return Err("This value can't be changed".into()),
    })
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let p = read_all(d)?;
    let mut edit = HashMap::new();
    let view = build(&p.root, "", &p, &mut edit);
    let backend = GkBackend { head: d[..HEADER].to_vec(), root: p.root, leaves: p.leaves, types: p.types, table: p.table };
    Ok(TreeDoc::new(view, edit, KIND, Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small save built by hand: `{_inventory: {_params: {_money: 12.5f, _hp: 0, tag: "x" (indexed)}}}`.
    fn sample() -> Vec<u8> {
        let h = |n: &str| -> i32 { names().iter().find(|(_, v)| **v == n).map(|(k, _)| *k).unwrap_or(1) };
        let enc = |s: &str| -> Vec<u8> { s.bytes().map(dec).collect() };
        let mut body = vec![];
        let i32b = |v: i32| v.to_le_bytes();
        body.extend(i32b(1));
        body.extend(i32b(h("_inventory")));
        body.push(250);
        body.extend(i32b(1));
        body.extend(i32b(h("_params")));
        body.push(250);
        body.extend(i32b(4));
        body.extend(i32b(h("_money")));
        body.push(5);
        body.extend(12.5f32.to_le_bytes());
        body.extend(i32b(h("_hp")));
        body.push(18);
        body.extend(i32b(h("id")));
        body.push(10);
        body.extend(i32b(0));
        body.extend(i32b(h("pos")));
        body.push(22);
        let mut f = vec![0u8; HEADER];
        f.extend(body);
        let tpos = f.len() as i64;
        f[..8].copy_from_slice(&tpos.to_le_bytes());
        f.extend(i32b(1));
        f.extend(i32b(5));
        f.extend(enc("flour"));
        f
    }

    #[test]
    fn round_trip_and_edit() {
        let raw = sample();
        let mut doc = parse(&raw).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), raw);
        assert_eq!(doc.view.pointer("/_inventory/_params/_money").unwrap(), 12.5);
        assert_eq!(doc.view.pointer("/_inventory/_params/id").unwrap(), "flour");
        doc.set("/_inventory/_params/_hp", Value::from(100)).unwrap();
        doc.set("/_inventory/_params/_money", Value::from(5000)).unwrap();
        doc.set("/_inventory/_params/id", Value::from("rye")).unwrap();
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/_inventory/_params/_hp").unwrap().as_f64(), Some(100.0));
        assert_eq!(back.view.pointer("/_inventory/_params/_money").unwrap().as_f64(), Some(5000.0));
        assert_eq!(back.view.pointer("/_inventory/_params/id").unwrap(), "rye");
    }

    #[test]
    fn real_save() {
        let Ok(p) = std::env::var("GK_SAMPLE") else { return };
        let raw = std::fs::read(p).unwrap();
        let doc = parse(&raw).unwrap();
        let d = doc.clone();
        assert_eq!(d.to_bytes().unwrap(), raw);
        println!("{}", doc.view.pointer("/_inventory/_params/_money").map(|v| v.to_string()).unwrap_or_default());
    }
}
