//! Minecraft NBT: Java Edition (big-endian, usually gzipped — `level.dat`,
//! `playerdata/*.dat`) and Bedrock Edition (little-endian, `level.dat` with an
//! 8-byte header). Parsed into typed tags and written back exactly; a file is only
//! accepted if re-writing it unchanged reproduces it byte-for-byte.

use crate::tree::{child, int_in, num, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(u8, Vec<Tag>),
    Compound(Vec<(String, Tag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    fn id(&self) -> u8 {
        match self {
            Tag::Byte(_) => 1,
            Tag::Short(_) => 2,
            Tag::Int(_) => 3,
            Tag::Long(_) => 4,
            Tag::Float(_) => 5,
            Tag::Double(_) => 6,
            Tag::ByteArray(_) => 7,
            Tag::String(_) => 8,
            Tag::List(..) => 9,
            Tag::Compound(_) => 10,
            Tag::IntArray(_) => 11,
            Tag::LongArray(_) => 12,
        }
    }
}

struct R<'a> {
    d: &'a [u8],
    pos: usize,
    le: bool,
}

macro_rules! rd {
    ($self:ident, $t:ty) => {{
        const N: usize = std::mem::size_of::<$t>();
        let b: [u8; N] = $self.take(N)?.try_into().unwrap();
        if $self.le {
            <$t>::from_le_bytes(b)
        } else {
            <$t>::from_be_bytes(b)
        }
    }};
}

impl R<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.d.len())
            .ok_or("unexpected end of data")?;
        let s = &self.d[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn len(&mut self) -> Result<usize, String> {
        let n = rd!(self, i32);
        if n < 0 || n as usize > self.d.len() {
            return Err("bad length".into());
        }
        Ok(n as usize)
    }
    fn string(&mut self) -> Result<String, String> {
        let n = rd!(self, u16) as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "invalid text".to_string())
    }
    fn payload(&mut self, id: u8, depth: usize) -> Result<Tag, String> {
        if depth > 512 {
            return Err("nested too deep".into());
        }
        Ok(match id {
            1 => Tag::Byte(rd!(self, i8)),
            2 => Tag::Short(rd!(self, i16)),
            3 => Tag::Int(rd!(self, i32)),
            4 => Tag::Long(rd!(self, i64)),
            5 => Tag::Float(rd!(self, f32)),
            6 => Tag::Double(rd!(self, f64)),
            7 => {
                let n = self.len()?;
                Tag::ByteArray(self.take(n)?.iter().map(|&b| b as i8).collect())
            }
            8 => Tag::String(self.string()?),
            9 => {
                let elem = rd!(self, u8);
                let n = self.len()?;
                if elem == 0 && n > 0 {
                    return Err("list of End tags".into());
                }
                let mut items = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    items.push(self.payload(elem, depth + 1)?);
                }
                Tag::List(elem, items)
            }
            10 => {
                let mut fields = Vec::new();
                loop {
                    let t = rd!(self, u8);
                    if t == 0 {
                        break;
                    }
                    let name = self.string()?;
                    fields.push((name, self.payload(t, depth + 1)?));
                }
                Tag::Compound(fields)
            }
            11 => {
                let n = self.len()?;
                Tag::IntArray(
                    (0..n)
                        .map(|_| Ok(rd!(self, i32)))
                        .collect::<Result<_, String>>()?,
                )
            }
            12 => {
                let n = self.len()?;
                Tag::LongArray(
                    (0..n)
                        .map(|_| Ok(rd!(self, i64)))
                        .collect::<Result<_, String>>()?,
                )
            }
            _ => return Err(format!("unknown tag {id}")),
        })
    }
}

struct W {
    out: Vec<u8>,
    le: bool,
}

macro_rules! wr {
    ($self:ident, $v:expr) => {{
        let v = $v;
        let b = if $self.le {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        };
        $self.out.extend_from_slice(&b);
    }};
}

impl W {
    fn string(&mut self, s: &str) {
        wr!(self, s.len() as u16);
        self.out.extend_from_slice(s.as_bytes());
    }
    fn payload(&mut self, t: &Tag) {
        match t {
            Tag::Byte(v) => wr!(self, *v),
            Tag::Short(v) => wr!(self, *v),
            Tag::Int(v) => wr!(self, *v),
            Tag::Long(v) => wr!(self, *v),
            Tag::Float(v) => wr!(self, *v),
            Tag::Double(v) => wr!(self, *v),
            Tag::ByteArray(a) => {
                wr!(self, a.len() as i32);
                self.out.extend(a.iter().map(|&b| b as u8));
            }
            Tag::String(s) => self.string(s),
            Tag::List(elem, items) => {
                wr!(self, *elem);
                wr!(self, items.len() as i32);
                for i in items {
                    self.payload(i);
                }
            }
            Tag::Compound(fields) => {
                for (name, v) in fields {
                    wr!(self, v.id());
                    self.string(name);
                    self.payload(v);
                }
                wr!(self, 0u8);
            }
            Tag::IntArray(a) => {
                wr!(self, a.len() as i32);
                for v in a {
                    wr!(self, *v);
                }
            }
            Tag::LongArray(a) => {
                wr!(self, a.len() as i32);
                for v in a {
                    wr!(self, *v);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Nbt {
    le: bool,
    /// Bedrock level.dat header: storage version.
    header: Option<i32>,
    root_name: String,
    root: Tag,
}

fn read(d: &[u8], le: bool, header: bool) -> Result<Nbt, String> {
    let (version, body) = if header {
        if d.len() < 9 {
            return Err("too short".into());
        }
        let len = i32::from_le_bytes(d[4..8].try_into().unwrap()) as usize;
        if len != d.len() - 8 {
            return Err("header length mismatch".into());
        }
        (
            Some(i32::from_le_bytes(d[0..4].try_into().unwrap())),
            &d[8..],
        )
    } else {
        (None, d)
    };
    let mut r = R {
        d: body,
        pos: 0,
        le,
    };
    if rd!(r, u8) != 10 {
        return Err("not NBT".into());
    }
    let root_name = r.string()?;
    let root = r.payload(10, 0)?;
    if r.pos != body.len() {
        return Err("extra data after NBT".into());
    }
    Ok(Nbt {
        le,
        header: version,
        root_name,
        root,
    })
}

fn write(n: &Nbt) -> Vec<u8> {
    let mut w = W {
        out: Vec::new(),
        le: n.le,
    };
    wr!(w, 10u8);
    w.string(&n.root_name);
    w.payload(&n.root);
    match n.header {
        Some(v) => {
            let mut out = v.to_le_bytes().to_vec();
            out.extend_from_slice(&(w.out.len() as i32).to_le_bytes());
            out.extend(w.out);
            out
        }
        None => w.out,
    }
}

#[derive(Clone, Debug)]
struct NbtBackend {
    nbt: Nbt,
    paths: Vec<Vec<usize>>,
}

fn get_mut<'a>(t: &'a mut Tag, path: &[usize]) -> Option<&'a mut Tag> {
    let mut cur = t;
    for &i in path {
        cur = match cur {
            Tag::Compound(f) => &mut f.get_mut(i)?.1,
            Tag::List(_, items) => items.get_mut(i)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// Apply a value to a tag; `idx` addresses an element of a numeric array tag.
fn set(tag: &mut Tag, idx: Option<usize>, v: &Value) -> Result<(), String> {
    let int = |lo: i128, hi: i128, what: &str| int_in(v, lo, hi, what);
    match (tag, idx) {
        (Tag::Byte(x), None) => *x = int(-128, 127, "A byte value")? as i8,
        (Tag::Short(x), None) => {
            *x = int(i16::MIN as i128, i16::MAX as i128, "A short value")? as i16
        }
        (Tag::Int(x), None) => *x = int(i32::MIN as i128, i32::MAX as i128, "An int value")? as i32,
        (Tag::Long(x), None) => {
            *x = int(i64::MIN as i128, i64::MAX as i128, "A long value")? as i64
        }
        (Tag::Float(x), None) => *x = v.as_f64().ok_or("must be a number")? as f32,
        (Tag::Double(x), None) => *x = v.as_f64().ok_or("must be a number")?,
        (Tag::String(x), None) => {
            let s = v.as_str().ok_or("must be text")?;
            if s.len() > u16::MAX as usize {
                return Err("text is too long for NBT".into());
            }
            *x = s.to_string();
        }
        (Tag::ByteArray(a), Some(i)) => {
            *a.get_mut(i).ok_or("bad index")? = int(-128, 127, "A byte value")? as i8
        }
        (Tag::IntArray(a), Some(i)) => {
            *a.get_mut(i).ok_or("bad index")? =
                int(i32::MIN as i128, i32::MAX as i128, "An int value")? as i32
        }
        (Tag::LongArray(a), Some(i)) => {
            *a.get_mut(i).ok_or("bad index")? =
                int(i64::MIN as i128, i64::MAX as i128, "A long value")? as i64
        }
        _ => return Err("this value can't be edited".into()),
    }
    Ok(())
}

/// A leaf path addresses a tag; for an element of a numeric array tag (ByteArray /
/// IntArray / LongArray) its last entry is the element index, which is split off here.
fn split(path: &[usize], tree: &Tag) -> (Vec<usize>, Option<usize>) {
    let mut t = tree;
    for (n, &i) in path.iter().enumerate() {
        match t {
            Tag::Compound(f) => t = &f[i].1,
            Tag::List(_, items) => t = &items[i],
            Tag::ByteArray(_) | Tag::IntArray(_) | Tag::LongArray(_) => {
                return (path[..n].to_vec(), Some(i))
            }
            _ => break,
        }
    }
    (path.to_vec(), None)
}

impl Backend for NbtBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut nbt = self.nbt.clone();
        for (id, v) in changes {
            let (path, idx) = split(&self.paths[*id], &nbt.root);
            set(
                get_mut(&mut nbt.root, &path).ok_or("couldn't locate an edited value")?,
                idx,
                v,
            )?;
        }
        Ok(write(&nbt))
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if {
            let mut root = self.nbt.root.clone();
            let (path, idx) = split(&self.paths[leaf], &root);
            idx.is_none() && matches!(get_mut(&mut root, &path), Some(Tag::Float(_)))
        } {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let mut root = self.nbt.root.clone();
        let (path, idx) = split(&self.paths[leaf], &root);
        set(get_mut(&mut root, &path).ok_or("missing value")?, idx, v)
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
    fn leaf(&mut self, ptr: &str, path: &[usize], v: Value) -> Value {
        self.editable.insert(ptr.to_string(), self.paths.len());
        self.paths.push(path.to_vec());
        v
    }

    fn view(&mut self, t: &Tag, ptr: &str, path: &mut Vec<usize>) -> Value {
        let arr = |b: &mut Self, vals: Vec<Value>, path: &mut Vec<usize>| {
            Value::Array(
                vals.into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        path.push(i);
                        let r = b.leaf(&format!("{ptr}/{i}"), path, v);
                        path.pop();
                        r
                    })
                    .collect(),
            )
        };
        match t {
            Tag::Byte(v) => self.leaf(ptr, path, Value::from(*v)),
            Tag::Short(v) => self.leaf(ptr, path, Value::from(*v)),
            Tag::Int(v) => self.leaf(ptr, path, Value::from(*v)),
            Tag::Long(v) => self.leaf(ptr, path, Value::from(*v)),
            Tag::Float(v) => self.leaf(ptr, path, crate::tree::f32_value(*v)),
            Tag::Double(v) => self.leaf(ptr, path, num(*v)),
            Tag::String(s) => self.leaf(ptr, path, Value::String(s.clone())),
            Tag::ByteArray(a) => arr(self, a.iter().map(|&x| Value::from(x)).collect(), path),
            Tag::IntArray(a) => arr(self, a.iter().map(|&x| Value::from(x)).collect(), path),
            Tag::LongArray(a) => arr(self, a.iter().map(|&x| Value::from(x)).collect(), path),
            Tag::List(_, items) => Value::Array(
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
            Tag::Compound(fields) => {
                let mut m = Map::new();
                for (i, (name, c)) in fields.iter().enumerate() {
                    let key = unique_key(&m, name);
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

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let candidates = [(false, false), (true, true), (true, false)];
    for (le, header) in candidates {
        if !header && d.first() != Some(&10) {
            continue;
        }
        let Ok(nbt) = read(d, le, header) else {
            continue;
        };
        // Only trust it if writing it back reproduces the file exactly.
        if write(&nbt) != d {
            continue;
        }
        let mut b = B {
            paths: vec![],
            editable: HashMap::new(),
        };
        let view = b.view(&nbt.root, "", &mut vec![]);
        let name = if le {
            "Minecraft NBT (Bedrock)"
        } else {
            "Minecraft NBT (Java)"
        };
        return Ok(TreeDoc::new(
            view,
            b.editable,
            name,
            Box::new(NbtBackend {
                nbt,
                paths: b.paths,
            }),
        ));
    }
    Err("not NBT".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(le: bool, header: bool) -> Vec<u8> {
        let root = Tag::Compound(vec![(
            "Data".into(),
            Tag::Compound(vec![
                ("LevelName".into(), Tag::String("My World".into())),
                ("Time".into(), Tag::Long(123456)),
                ("hardcore".into(), Tag::Byte(0)),
                ("SpawnX".into(), Tag::Int(-40)),
                (
                    "Pos".into(),
                    Tag::List(
                        6,
                        vec![Tag::Double(1.5), Tag::Double(64.0), Tag::Double(-3.25)],
                    ),
                ),
                ("Inv".into(), Tag::IntArray(vec![1, 2, 3])),
            ]),
        )]);
        write(&Nbt {
            le,
            header: header.then_some(10),
            root_name: String::new(),
            root,
        })
    }

    /// The "Max crafting materials" button on a made-up Java player: materials go to 64,
    /// progression items and single items without a count stay as they are.
    #[test]
    fn minecraft_crafting_materials_round_trip() {
        let item = |slot: i8, id: &str, count: Option<Tag>| {
            let mut c = vec![
                ("Slot".to_string(), Tag::Byte(slot)),
                ("id".to_string(), Tag::String(id.into())),
            ];
            if let Some(n) = count {
                c.push(("Count".to_string(), n));
            }
            Tag::Compound(c)
        };
        let ender = Tag::Compound(vec![
            ("Slot".into(), Tag::Byte(0)),
            ("id".into(), Tag::String("minecraft:string".into())),
            ("count".into(), Tag::Int(5)),
        ]);
        let root = Tag::Compound(vec![(
            "Data".into(),
            Tag::Compound(vec![(
                "Player".into(),
                Tag::Compound(vec![
                    (
                        "Inventory".into(),
                        Tag::List(
                            10,
                            vec![
                                item(0, "minecraft:iron_ingot", Some(Tag::Byte(9))),
                                item(1, "minecraft:ender_pearl", Some(Tag::Byte(3))),
                                item(2, "minecraft:eye_of_ender", Some(Tag::Byte(2))),
                                item(3, "minecraft:diamond", None),
                            ],
                        ),
                    ),
                    ("EnderItems".into(), Tag::List(10, vec![ender])),
                ]),
            )]),
        )]);
        let raw = write(&Nbt {
            le: false,
            header: None,
            root_name: String::new(),
            root,
        });
        let mut doc = parse(&raw).unwrap();
        let view = doc.view.clone();
        let mut cx = crate::game_ui::GameCtx {
            root: &view,
            baseline: &view,
            tree: Some(&doc),
            edits: vec![],
            archive: None,
            open_entry: None,
        };
        assert_eq!(
            crate::game_ui::max_crafting_materials(crate::game_ui::Game::Minecraft, &mut cx),
            2
        );
        for (p, v) in cx.edits {
            doc.set(&p, v).unwrap().expect("editable");
        }
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        let v = |p: &str| again.view.pointer(p).cloned().unwrap();
        assert_eq!(v("/Data/Player/Inventory/0/Count"), 64);
        assert_eq!(v("/Data/Player/Inventory/1/Count"), 3);
        assert_eq!(v("/Data/Player/Inventory/2/Count"), 2);
        assert!(again
            .view
            .pointer("/Data/Player/Inventory/3/Count")
            .is_none());
        assert_eq!(v("/Data/Player/EnderItems/0/count"), 64);
    }

    #[test]
    fn java_and_bedrock() {
        for (le, header) in [(false, false), (true, true)] {
            let raw = sample(le, header);
            let mut doc = parse(&raw).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), raw);
            assert_eq!(doc.view.pointer("/Data/LevelName").unwrap(), "My World");
            doc.set("/Data/Time", Value::from(999)).unwrap();
            doc.set("/Data/Pos/1", Value::from(80.0)).unwrap();
            doc.set("/Data/Inv/2", Value::from(7)).unwrap();
            assert!(
                doc.set("/Data/hardcore", Value::from(300)).is_err(),
                "byte range enforced"
            );
            let again = parse(&doc.to_bytes().unwrap()).unwrap();
            assert_eq!(again.view.pointer("/Data/Time").unwrap(), 999);
            assert_eq!(
                again.view.pointer("/Data/Pos/1").unwrap().as_f64(),
                Some(80.0)
            );
            assert_eq!(again.view.pointer("/Data/Inv/2").unwrap(), 7);
        }
    }
}
