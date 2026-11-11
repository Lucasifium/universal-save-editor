//! Oxygen Not Included saves (`.sav`).
//!
//! A header (build, header size, header version, compressed flag), a JSON summary, then
//! type templates: for every serialised class its fields and properties with their types.
//! The body (zlib when compressed) is "world", the world root and settings by template,
//! simulation data, "KSAV" and the version, then game objects in groups by prefab, each
//! group and each behaviour of an object carrying its length. Only duplicants (prefab
//! `Minion`) are read here: every behaviour's template data, with its numbers editable in
//! place; everything else is skipped by length and kept byte for byte. Layout as documented
//! by RoboPhred's oni-save-parser (MIT).

use super::oni_map::{self, OniMap, Poi, World};
use crate::tree::{esc, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct TypeInfo {
    code: u8,
    value_type: bool,
    template: Option<String>,
    sub: Vec<TypeInfo>,
}

#[derive(Clone, Debug)]
struct Template {
    members: Vec<(String, TypeInfo)>,
}

struct R<'a> {
    d: &'a [u8],
    pos: usize,
}

impl R<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let b = self
            .d
            .get(self.pos..self.pos + n)
            .ok_or("unexpected end of save")?;
        self.pos += n;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<Option<String>, String> {
        let n = self.i32()?;
        if n < 0 {
            return Ok(None);
        }
        Ok(Some(
            String::from_utf8_lossy(self.take(n as usize)?).into_owned(),
        ))
    }
    fn name(&mut self) -> Result<String, String> {
        self.string()?.ok_or_else(|| "missing name".to_string())
    }
}

fn type_info(r: &mut R) -> Result<TypeInfo, String> {
    let info = r.u8()?;
    let code = info & 63;
    let template = if code == 0 || code == 13 {
        Some(r.name()?)
    } else {
        None
    };
    let mut sub = vec![];
    if info & 128 != 0 {
        for _ in 0..r.u8()? {
            sub.push(type_info(r)?);
        }
    } else if code == 17 {
        sub.push(type_info(r)?);
    }
    Ok(TypeInfo {
        code,
        value_type: info & 64 != 0,
        template,
        sub,
    })
}

fn templates(r: &mut R) -> Result<HashMap<String, Template>, String> {
    let n = r.i32()?;
    if !(0..100_000).contains(&n) {
        return Err("bad template count".into());
    }
    let mut out = HashMap::new();
    for _ in 0..n {
        let name = r.name()?;
        let fields = r.i32()?;
        let props = r.i32()?;
        let mut members = vec![];
        for _ in 0..fields.max(0) + props.max(0) {
            let m = r.name()?;
            members.push((m, type_info(r)?));
        }
        out.insert(name, Template { members });
    }
    Ok(out)
}

/// An editable number in the decompressed body.
#[derive(Clone, Copy, Debug)]
struct Leaf {
    at: usize,
    code: u8,
    /// For text: where the lengths of the behaviour and the object group holding it are.
    lens: [usize; 2],
}

/// Leaf codes beyond the save's own type codes.
const TEXT: u8 = 250;
const MAP_EDITS: u8 = 251;

fn size_of(code: u8) -> usize {
    match code {
        1..=3 => 1,
        4 | 5 => 2,
        6 | 7 | 10 | 13 => 4,
        8 | 9 | 11 => 8,
        _ => 0,
    }
}

struct Reader<'a> {
    r: R<'a>,
    t: &'a HashMap<String, Template>,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
    /// Every text read: pointer and where its length is.
    strs: Vec<(String, usize)>,
    /// Every byte array read: pointer, where its bytes start and how many.
    blobs: Vec<(String, usize, usize)>,
}

impl<'a> Reader<'a> {
    fn new(d: &'a [u8], pos: usize, t: &'a HashMap<String, Template>) -> Self {
        Reader {
            r: R { d, pos },
            t,
            leaves: vec![],
            editable: HashMap::new(),
            strs: vec![],
            blobs: vec![],
        }
    }

    fn number(&mut self, code: u8, ptr: &str) -> Result<Value, String> {
        let at = self.r.pos;
        let n = size_of(code);
        let b = self.r.take(n)?;
        let v = match code {
            1 => Value::from(b[0] as i8),
            2 => Value::from(b[0]),
            3 => Value::Bool(b[0] != 0),
            4 => Value::from(i16::from_le_bytes([b[0], b[1]])),
            5 => Value::from(u16::from_le_bytes([b[0], b[1]])),
            6 | 13 => Value::from(i32::from_le_bytes(b.try_into().unwrap())),
            7 => Value::from(u32::from_le_bytes(b.try_into().unwrap())),
            8 => Value::from(i64::from_le_bytes(b.try_into().unwrap())),
            9 => Value::from(u64::from_le_bytes(b.try_into().unwrap())),
            10 => crate::tree::f32_value(f32::from_le_bytes(b.try_into().unwrap())),
            _ => serde_json::Number::from_f64(f64::from_le_bytes(b.try_into().unwrap()))
                .map_or(Value::Null, Value::Number),
        };
        if !(code == 3 && b[0] > 1) && !v.is_null() {
            self.editable.insert(ptr.to_string(), self.leaves.len());
            self.leaves.push(Leaf {
                at,
                code,
                lens: [0; 2],
            });
        }
        Ok(v)
    }

    fn template(&mut self, name: &str, ptr: &str) -> Result<Value, String> {
        let t = self
            .t
            .get(name)
            .ok_or_else(|| format!("no template {name}"))?
            .clone();
        let mut m = Map::new();
        for (k, ty) in &t.members {
            let key = unique_key(&m, k);
            let v = self.value(ty, &format!("{ptr}/{}", esc(&key)))?;
            m.insert(key, v);
        }
        Ok(Value::Object(m))
    }

    fn value(&mut self, ty: &TypeInfo, ptr: &str) -> Result<Value, String> {
        match ty.code {
            1..=11 | 13 => self.number(ty.code, ptr),
            12 => {
                self.strs.push((ptr.to_string(), self.r.pos));
                Ok(self.r.string()?.map_or(Value::Null, Value::from))
            }
            14 => Ok(Value::from(vec![self.r.i32()?, self.r.i32()?])),
            15 => Ok(Value::from(
                (0..2)
                    .map(|_| self.r.f32().map(|f| f as f64))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            16 => Ok(Value::from(
                (0..3)
                    .map(|_| self.r.f32().map(|f| f as f64))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            23 => Ok(Value::from(
                self.r
                    .take(4)?
                    .iter()
                    .map(|b| format!("{b:02X}"))
                    .collect::<String>(),
            )),
            0 => {
                let name = ty.template.clone().unwrap_or_default();
                let len = self.r.i32()?;
                if len < 0 {
                    return Ok(Value::Null);
                }
                let start = self.r.pos;
                let v = self.template(&name, ptr)?;
                if self.r.pos != start + len as usize {
                    return Err(format!("{name}: length mismatch"));
                }
                Ok(v)
            }
            17 | 20 | 21 | 22 => {
                let _size = self.r.i32()?;
                let n = self.r.i32()?;
                if n < 0 {
                    return Ok(Value::Null);
                }
                let el = ty.sub.first().ok_or("array without element type")?;
                if el.code == 2 {
                    self.blobs.push((ptr.to_string(), self.r.pos, n as usize));
                    self.r.take(n as usize)?;
                    return Ok(Value::from(format!("({n} bytes)")));
                }
                let mut out = Vec::with_capacity((n as usize).min(4096));
                for i in 0..n as usize {
                    let p = format!("{ptr}/{i}");
                    out.push(if el.value_type && el.code == 0 {
                        self.template(el.template.as_deref().unwrap_or(""), &p)?
                    } else {
                        self.value(el, &p)?
                    });
                }
                Ok(Value::Array(out))
            }
            18 => {
                let len = self.r.i32()?;
                if len < 0 {
                    return Ok(Value::Null);
                }
                let (k, v) = (ty.sub.first().ok_or("pair")?, ty.sub.get(1).ok_or("pair")?);
                let key = self.value(k, &format!("{ptr}/key"))?;
                let val = self.value(v, &format!("{ptr}/value"))?;
                Ok(serde_json::json!({ "key": key, "value": val }))
            }
            19 => {
                let _size = self.r.i32()?;
                let n = self.r.i32()?;
                if n < 0 {
                    return Ok(Value::Null);
                }
                let (kt, vt) = (
                    ty.sub.first().ok_or("dict")?.clone(),
                    ty.sub.get(1).ok_or("dict")?.clone(),
                );
                // Values come first, then the keys; numbered until the keys are known.
                let mut vals = vec![];
                for i in 0..n as usize {
                    vals.push(self.value(&vt, &format!("{ptr}/\u{1}{i}"))?);
                }
                let mut keys = vec![];
                for i in 0..n as usize {
                    keys.push(self.value(&kt, &format!("{ptr}/\u{2}{i}"))?);
                }
                // Re-point the values' leaves at their keys.
                let mut m = Map::new();
                for (i, (k, v)) in keys.into_iter().zip(vals).enumerate() {
                    let name = match &k {
                        Value::String(s) => s.clone(),
                        Value::Object(o) => o
                            .get("hash")
                            .map_or_else(|| k.to_string(), |h| h.to_string()),
                        x => x.to_string(),
                    };
                    let key = unique_key(&m, &name);
                    let old = format!("{ptr}/\u{1}{i}");
                    let new = format!("{ptr}/{}", esc(&key));
                    let moved: Vec<(String, usize)> = self
                        .editable
                        .iter()
                        .filter(|(p, _)| {
                            p.starts_with(&old)
                                && (p.len() == old.len() || p.as_bytes()[old.len()] == b'/')
                        })
                        .map(|(p, l)| (p.clone(), *l))
                        .collect();
                    for (p, l) in moved {
                        self.editable.remove(&p);
                        self.editable.insert(format!("{new}{}", &p[old.len()..]), l);
                    }
                    let under = |p: &str| {
                        p.starts_with(&old)
                            && (p.len() == old.len() || p.as_bytes()[old.len()] == b'/')
                    };
                    for (p, _) in self.strs.iter_mut().filter(|(p, _)| under(p)) {
                        *p = format!("{new}{}", &p[old.len()..]);
                    }
                    for (p, _, _) in self.blobs.iter_mut().filter(|(p, _, _)| under(p)) {
                        *p = format!("{new}{}", &p[old.len()..]);
                    }
                    m.insert(key, v);
                }
                let prefix = format!("{ptr}/\u{2}");
                self.editable.retain(|p, _| !p.starts_with(&prefix));
                Ok(Value::Object(m))
            }
            c => Err(format!("unknown type {c}")),
        }
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 32
        && d.get(16..18) == Some(b"{\"")
        && d[16..d.len().min(4096)]
            .windows(18)
            .any(|w| w == b"numberOfDuplicants")
}

fn skip_template(r: &mut R, t: &HashMap<String, Template>, name: &str) -> Result<(), String> {
    let mut rd = Reader::new(r.d, r.pos, t);
    rd.template(name, "")?;
    r.pos = rd.r.pos;
    Ok(())
}

#[derive(Clone, Debug)]
struct OniBackend {
    prefix: Arc<Vec<u8>>,
    body: Arc<Vec<u8>>,
    compressed: bool,
    original: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
    map: Option<Arc<OniMap>>,
}

impl OniBackend {
    fn encode(leaf: &Leaf, v: &Value) -> Result<Vec<u8>, String> {
        use crate::tree::int_in;
        Ok(match leaf.code {
            1 => vec![int_in(v, i8::MIN as i128, i8::MAX as i128, "This value")? as i8 as u8],
            2 => vec![int_in(v, 0, 255, "This value")? as u8],
            3 => vec![u8::from(v.as_bool().ok_or("tick or untick it")?)],
            4 => (int_in(v, i16::MIN as i128, i16::MAX as i128, "This value")? as i16)
                .to_le_bytes()
                .to_vec(),
            5 => (int_in(v, 0, u16::MAX as i128, "This value")? as u16)
                .to_le_bytes()
                .to_vec(),
            6 | 13 => (int_in(v, i32::MIN as i128, i32::MAX as i128, "This value")? as i32)
                .to_le_bytes()
                .to_vec(),
            7 => (int_in(v, 0, u32::MAX as i128, "This value")? as u32)
                .to_le_bytes()
                .to_vec(),
            8 => (int_in(v, i64::MIN as i128, i64::MAX as i128, "This value")? as i64)
                .to_le_bytes()
                .to_vec(),
            9 => (int_in(v, 0, u64::MAX as i128, "This value")? as u64)
                .to_le_bytes()
                .to_vec(),
            10 => (v
                .as_f64()
                .filter(|f| f.is_finite())
                .ok_or("expected a number")? as f32)
                .to_le_bytes()
                .to_vec(),
            TEXT => {
                let s = v.as_str().ok_or("expected text")?.trim();
                if s.is_empty() {
                    return Err("the name can't be empty".into());
                }
                if s.chars().count() > 60 {
                    return Err("names can be at most 60 characters".into());
                }
                let mut b = (s.len() as i32).to_le_bytes().to_vec();
                b.extend_from_slice(s.as_bytes());
                b
            }
            MAP_EDITS => return Err("map edits are applied separately".into()),
            _ => v
                .as_f64()
                .filter(|f| f.is_finite())
                .ok_or("expected a number")?
                .to_le_bytes()
                .to_vec(),
        })
    }
}

impl Backend for OniBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.original.as_ref().clone());
        }
        let mut body = self.body.as_ref().clone();
        let mut texts = vec![];
        for (l, v) in changes {
            let leaf = &self.leaves[*l];
            match leaf.code {
                MAP_EDITS => {
                    let ops = oni_map::parse_ops(v.as_str().ok_or("expected text")?)?;
                    if let Some(map) = &self.map {
                        map.apply_to_body(&mut body, &ops)?;
                    }
                }
                TEXT => texts.push((leaf, Self::encode(leaf, v)?)),
                _ => {
                    let b = Self::encode(leaf, v)?;
                    body[leaf.at..leaf.at + b.len()].copy_from_slice(&b);
                }
            }
        }
        // Text changes the body's length: the last first, so earlier places stay put, and
        // the behaviour and group lengths before each grow or shrink with it.
        texts.sort_by_key(|(leaf, _)| std::cmp::Reverse(leaf.at));
        for (leaf, b) in texts {
            let old =
                i32::from_le_bytes(body[leaf.at..leaf.at + 4].try_into().unwrap()).max(0) as usize;
            body.splice(leaf.at..leaf.at + 4 + old, b.iter().copied());
            let delta = b.len() as i64 - (4 + old) as i64;
            for &at in &leaf.lens {
                let n = i32::from_le_bytes(body[at..at + 4].try_into().unwrap()) as i64 + delta;
                body[at..at + 4].copy_from_slice(&(n as i32).to_le_bytes());
            }
        }
        let mut out = self.prefix.as_ref().clone();
        if self.compressed {
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            e.write_all(&body).map_err(|e| e.to_string())?;
            out.extend(e.finish().map_err(|e| e.to_string())?);
        } else {
            out.extend(body);
        }
        Ok(out)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        match self.leaves[leaf].code {
            10 => crate::tree::round_f32(v),
            TEXT => v.as_str().map_or(v.clone(), |s| Value::from(s.trim())),
            _ => v,
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        if self.leaves[leaf].code == MAP_EDITS {
            let map = self
                .map
                .as_ref()
                .ok_or("this save's map couldn't be read")?;
            return oni_map::parse_ops(v.as_str().ok_or("expected text")?)?
                .iter()
                .try_for_each(|op| oni_map::op_ok(op, map));
        }
        Self::encode(&self.leaves[leaf], v).map(|_| ())
    }

    fn reshapes(&self, leaf: usize) -> bool {
        matches!(self.leaves[leaf].code, TEXT | MAP_EDITS)
    }

    fn attachment(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        self.map
            .clone()
            .map(|m| m as Arc<dyn std::any::Any + Send + Sync>)
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// World fields shown and editable: (field, label).
const WORLD_FIELDS: [(&str, &str); 4] = [
    ("isDiscovered", "Discovered"),
    ("isSurfaceRevealed", "Surface revealed"),
    ("sunlight", "Sunlight (lux)"),
    ("cosmicRadiation", "Cosmic radiation (rads)"),
];

fn world_title(world_name: &str) -> String {
    let last = world_name.rsplit('/').next().unwrap_or(world_name);
    let mut out = String::new();
    let mut prev_lower = false;
    for c in last.chars() {
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        out.push(c);
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut r = R { d, pos: 0 };
    let _build = r.i32()?;
    let header_size = r.i32()? as usize;
    let header_version = r.i32()?;
    let compressed = header_version >= 1 && r.i32()? != 0;
    let info: Value = serde_json::from_slice(r.take(header_size)?).map_err(|e| e.to_string())?;
    let t = templates(&mut r)?;
    let prefix = d[..r.pos].to_vec();
    let body = if compressed {
        let mut out = vec![];
        flate2::read::ZlibDecoder::new(&d[r.pos..])
            .read_to_end(&mut out)
            .map_err(|e| e.to_string())?;
        out
    } else {
        d[r.pos..].to_vec()
    };

    let mut b = R { d: &body, pos: 0 };
    if b.name()? != "world" {
        return Err("no world in this save".into());
    }
    let root_type = b.name()?;
    // The world root: its size and the map's discovered cells.
    let mut root_rd = Reader::new(&body, b.pos, &t);
    let root_v = root_rd.template(&root_type, "")?;
    b.pos = root_rd.r.pos;
    let settings_type = b.name()?;
    skip_template(&mut b, &t, &settings_type)?;
    let sim = b.i32()?;
    let sim_at = b.pos;
    b.take(sim.max(0) as usize)?;
    if b.take(4)? != b"KSAV" {
        return Err("KSAV marker missing".into());
    }
    b.take(8)?;

    let mut dupes = Map::new();
    let mut worlds_v = Map::new();
    let mut rockets_v = Map::new();
    let mut worlds: Vec<World> = vec![];
    let mut pois: Vec<Poi> = vec![];
    let mut starmap: Vec<(String, usize)> = vec![];
    let mut tiles: Vec<(i32, i32)> = vec![];
    let mut rd = Reader::new(&body, b.pos, &t);
    let mut moved: HashMap<String, usize> = HashMap::new();
    let groups = rd.r.i32()?;
    for _ in 0..groups {
        let prefab = rd.r.name()?;
        let count = rd.r.i32()?;
        let group_len_at = rd.r.pos;
        let len = rd.r.i32()? as usize;
        let start = rd.r.pos;
        let poi = oni_map::poi_of(&prefab);
        if let Some(name) = oni_map::starmap_name(&prefab) {
            match starmap.iter_mut().find(|(n, _)| *n == name) {
                Some(e) => e.1 += count.max(0) as usize,
                None => starmap.push((name, count.max(0) as usize)),
            }
        }
        let tile = oni_map::is_tile_building(&prefab);
        for i in 0..count {
            let x = rd.r.f32()?;
            let y = rd.r.f32()?;
            rd.r.take(4 * 8 + 1)?; // z, rotation, scale, folder
            let behaviors = rd.r.i32()?;
            let mut raw: Vec<(String, usize, usize)> = vec![];
            for _ in 0..behaviors {
                let name = rd.r.name()?;
                let blen = rd.r.i32()? as usize;
                raw.push((name, rd.r.pos, blen));
                rd.r.pos += blen;
            }
            let end = rd.r.pos;
            let (cx, cy) = (x.floor() as i32, (y + 0.01).floor() as i32);
            if tile {
                tiles.push((cx, cy));
            }
            if let Some((kind, name)) = &poi {
                if prefab != "Minion" {
                    pois.push(Poi {
                        kind: *kind,
                        name: name.clone(),
                        x: cx,
                        y: cy,
                    });
                }
            }
            if prefab == "Minion" {
                let name = raw
                    .iter()
                    .find(|b| b.0 == "MinionIdentity")
                    .and_then(|b| {
                        let mut s = Reader::new(&body, b.1, &t);
                        s.template("MinionIdentity", "")
                            .ok()
                            .and_then(|v| v["name"].as_str().map(str::to_string))
                    })
                    .unwrap_or_else(|| format!("Duplicant {}", i + 1));
                pois.push(Poi {
                    kind: 6,
                    name: name.clone(),
                    x: cx,
                    y: cy,
                });
                let key = unique_key(&dupes, &name);
                let base = format!("/Duplicants/{}", esc(&key));
                let mut m = Map::new();
                for (bname, at, blen) in &raw {
                    if !t.contains_key(bname) {
                        continue;
                    }
                    rd.r.pos = *at;
                    let bkey = unique_key(&m, bname);
                    match rd.template(bname, &format!("{base}/{}", esc(&bkey))) {
                        Ok(v) if rd.r.pos <= at + blen => {
                            m.insert(bkey, v);
                        }
                        _ => {}
                    }
                }
                dupes.insert(key, Value::Object(m));
            } else if raw
                .iter()
                .any(|b| b.0 == "WorldContainer" || b.0 == "Clustercraft")
            {
                // Worlds (asteroids, rocket interiors) and rockets: their name is the
                // `m_name` of a behaviour (Clustercraft for rockets).
                let mut name: Option<(String, Leaf)> = None;
                for (bname, at, blen) in &raw {
                    let has_name = t.get(bname).is_some_and(|tp| {
                        tp.members
                            .iter()
                            .any(|(m, ty)| m == "m_name" && ty.code == 12)
                    });
                    if !has_name || name.is_some() {
                        continue;
                    }
                    let mut s2 = Reader::new(&body, *at, &t);
                    if let Ok(v) = s2.template(bname, "") {
                        if let (Some(n), Some((_, sat))) = (
                            v["m_name"].as_str(),
                            s2.strs.iter().find(|(p, _)| p == "/m_name"),
                        ) {
                            if s2.r.pos <= at + blen && !n.trim().is_empty() {
                                name = Some((
                                    n.to_string(),
                                    Leaf {
                                        at: *sat,
                                        code: TEXT,
                                        lens: [at - 4, group_len_at],
                                    },
                                ));
                            }
                        }
                    }
                }
                let wc = raw.iter().find(|b| b.0 == "WorldContainer").cloned();
                let world = wc.and_then(|(bname, at, blen)| {
                    rd.r.pos = at;
                    let tmp = format!("\u{3}{}", worlds.len());
                    let v = rd
                        .template(&bname, &tmp)
                        .ok()
                        .filter(|_| rd.r.pos <= at + blen)?;
                    Some((v, tmp))
                });
                let is_craft = raw.iter().any(|b| b.0 == "Clustercraft");
                if let Some((v, tmp)) = world {
                    let pair = |k: &str| {
                        v[k].as_array().map(|a| {
                            (
                                a.first().and_then(Value::as_i64).unwrap_or(0),
                                a.get(1).and_then(Value::as_i64).unwrap_or(0),
                            )
                        })
                    };
                    let (Some((ox, oy)), Some((w, h))) = (pair("worldOffset"), pair("worldSize"))
                    else {
                        continue;
                    };
                    let interior = v["isModuleInterior"].as_bool().unwrap_or(false);
                    let id = v["id"].as_i64().unwrap_or(-1) as i32;
                    let title = name.as_ref().map(|n| n.0.clone()).unwrap_or_else(|| {
                        let wn = v["worldName"].as_str().unwrap_or("");
                        if wn.is_empty() {
                            format!("World {id}")
                        } else {
                            world_title(wn)
                        }
                    });
                    let key = unique_key(&worlds_v, &title);
                    let base = format!("/Worlds/{}", esc(&key));
                    let mut m = Map::new();
                    if let Some((n, leaf)) = &name {
                        m.insert("Name".into(), Value::from(n.clone()));
                        moved.insert(format!("{base}/Name"), rd.leaves.len());
                        rd.leaves.push(*leaf);
                    }
                    m.insert(
                        "Type".into(),
                        Value::from(if interior {
                            "Rocket interior"
                        } else {
                            "Asteroid"
                        }),
                    );
                    if let Some(wn) = v["worldName"].as_str().filter(|s| !s.is_empty()) {
                        m.insert("Kind".into(), Value::from(world_title(wn)));
                    }
                    m.insert("Size".into(), Value::from(format!("{w} \u{00D7} {h}")));
                    m.insert(
                        "Start world".into(),
                        Value::Bool(v["isStartWorld"].as_bool().unwrap_or(false)),
                    );
                    for (f, label) in WORLD_FIELDS {
                        if let Some(l) = rd.editable.get(&format!("{tmp}/{f}")) {
                            moved.insert(format!("{base}/{label}"), *l);
                            m.insert(label.into(), v[f].clone());
                        }
                    }
                    worlds_v.insert(key.clone(), Value::Object(m));
                    worlds.push(World {
                        id,
                        name: title,
                        rocket: interior,
                        x: ox.max(0) as usize,
                        y: oy.max(0) as usize,
                        w: w.max(0) as usize,
                        h: h.max(0) as usize,
                        discovered: v["isDiscovered"].as_bool().unwrap_or(false),
                        start: v["isStartWorld"].as_bool().unwrap_or(false),
                        key,
                    });
                    rd.editable.retain(|p, _| !p.starts_with(&tmp));
                }
                if is_craft {
                    if let Some((n, leaf)) = &name {
                        let key = unique_key(&rockets_v, n);
                        let base = format!("/Rockets/{}", esc(&key));
                        moved.insert(format!("{base}/Name"), rd.leaves.len());
                        rd.leaves.push(*leaf);
                        let mut m = Map::new();
                        m.insert("Name".into(), Value::from(n.clone()));
                        rockets_v.insert(key, Value::Object(m));
                    }
                }
            }
            rd.r.pos = end;
        }
        if rd.r.pos != start + len {
            return Err("game object data didn't add up".into());
        }
    }
    let mut colony = Map::new();
    for (k, label) in [
        ("baseName", "Colony"),
        ("numberOfCycles", "Cycles"),
        ("numberOfDuplicants", "Duplicants"),
        ("clusterId", "Cluster"),
    ] {
        if let Some(v) = info.get(k) {
            colony.insert(label.into(), v.clone());
        }
    }

    // The map.
    let width = root_v["WidthInCells"].as_i64().unwrap_or(0).max(0) as usize;
    let height = root_v["HeightInCells"].as_i64().unwrap_or(0).max(0) as usize;
    let map = OniMap::read(&body, sim_at, sim.max(0) as usize)
        .filter(|(_, _, sw, sh)| *sw == width + 2 && *sh == height + 2 && !worlds.is_empty())
        .map(|(cells, radiation, sw, sh)| {
            let mut protected = vec![false; cells.len()];
            for (x, y) in tiles {
                if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                    protected[y as usize * width + x as usize] = true;
                }
            }
            let visible_at = root_rd
                .blobs
                .iter()
                .find(|(p, _, n)| p.ends_with("/GridVisible") && *n == width * height)
                .map(|b| b.1);
            let visible = visible_at.map(|at| body[at..at + width * height].to_vec());
            let mut present: Vec<u32> = cells.iter().map(|c| c.element).collect();
            present.sort_unstable();
            present.dedup();
            worlds.sort_by_key(|w| (w.rocket, w.id));
            pois.retain(|p| p.x >= 0 && p.y >= 0);
            starmap.sort();
            Arc::new(OniMap {
                width,
                height,
                cells,
                radiation,
                protected,
                visible,
                worlds,
                pois,
                starmap,
                present,
                sim_at,
                sim_w: sw,
                sim_h: sh,
                visible_at,
            })
        });

    let mut root = Map::new();
    root.insert("Colony".into(), Value::Object(colony));
    root.insert("Duplicants".into(), Value::Object(dupes));
    if !worlds_v.is_empty() {
        root.insert("Worlds".into(), Value::Object(worlds_v));
    }
    if !rockets_v.is_empty() {
        root.insert("Rockets".into(), Value::Object(rockets_v));
    }
    let (mut leaves, mut editable) = (rd.leaves, rd.editable);
    editable.extend(moved);
    if map.is_some() {
        root.insert("Map edits".into(), Value::from(""));
        editable.insert(oni_map::EDITS.into(), leaves.len());
        leaves.push(Leaf {
            at: 0,
            code: MAP_EDITS,
            lens: [0; 2],
        });
    }
    let backend = OniBackend {
        prefix: Arc::new(prefix),
        body: Arc::new(body),
        compressed,
        original: Arc::new(d.to_vec()),
        leaves,
        map,
    };
    Ok(TreeDoc::new(Value::Object(root), editable, "Oxygen Not Included save", Box::new(backend))
        .with_note("Duplicants, the map of every asteroid and rocket interior, world settings and rocket names can be changed; the rest of the colony is kept as it is."))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Research aid: parses ONI_SAVE and prints the duplicants' behaviours.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("ONI_SAVE").expect("ONI_SAVE")).unwrap();
        let t = std::time::Instant::now();
        let doc = parse(&d).unwrap();
        println!(
            "{:?}: {} editable; {}",
            t.elapsed(),
            doc.editable.len(),
            doc.view["Colony"]
        );
        for (k, v) in doc.view["Duplicants"].as_object().unwrap().iter().take(2) {
            println!(
                "{k}: {:?}",
                v.as_object().unwrap().keys().collect::<Vec<_>>()
            );
            for b in ["Klei.AI.AttributeLevels", "MinionResume"] {
                let s = v.get(b).map(|x| x.to_string()).unwrap_or_default();
                println!("  {b}: {}", &s[..s.len().min(1200)]);
            }
        }
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}

#[cfg(test)]
mod research {
    use super::*;

    /// Research aid: ONI_SAVE = a save; writes ONI_OUT.body (decompressed), ONI_OUT.root.json
    /// (world root and settings by template) and ONI_OUT.sim (the simulation block).
    #[test]
    #[ignore]
    fn dump_world() {
        let d = std::fs::read(std::env::var("ONI_SAVE").unwrap()).unwrap();
        let out = std::env::var("ONI_OUT").unwrap();
        let mut r = R { d: &d, pos: 0 };
        r.i32().unwrap();
        let hs = r.i32().unwrap() as usize;
        let hv = r.i32().unwrap();
        let compressed = hv >= 1 && r.i32().unwrap() != 0;
        r.take(hs).unwrap();
        let t = templates(&mut r).unwrap();
        let mut body = vec![];
        if compressed {
            flate2::read::ZlibDecoder::new(&d[r.pos..])
                .read_to_end(&mut body)
                .unwrap();
        } else {
            body = d[r.pos..].to_vec();
        }
        std::fs::write(format!("{out}.body"), &body).unwrap();
        let mut b = R { d: &body, pos: 0 };
        assert_eq!(b.name().unwrap(), "world");
        let mut rd = Reader::new(&body, 0, &t);
        rd.r.pos = b.pos;
        let root_type = rd.r.name().unwrap();
        let root = rd.template(&root_type, "").unwrap();
        let settings_type = rd.r.name().unwrap();
        let settings = rd.template(&settings_type, "").unwrap();
        let sim_len = rd.r.i32().unwrap() as usize;
        let sim_at = rd.r.pos;
        std::fs::write(format!("{out}.sim"), &body[sim_at..sim_at + sim_len]).unwrap();
        let j = serde_json::json!({"root_type": root_type, "root": root, "settings_type": settings_type, "settings": settings, "sim_at": sim_at, "sim_len": sim_len});
        std::fs::write(
            format!("{out}.root.json"),
            serde_json::to_string_pretty(&j).unwrap(),
        )
        .unwrap();
        println!("body {} bytes, sim at {sim_at} len {sim_len}", body.len());
        // Game objects: every prefab with its count; WorldContainer data in full.
        rd.r.pos = sim_at + sim_len;
        assert_eq!(rd.r.take(4).unwrap(), b"KSAV");
        rd.r.take(8).unwrap();
        let groups = rd.r.i32().unwrap();
        let mut list = vec![];
        let mut worlds = vec![];
        for _ in 0..groups {
            let prefab = rd.r.name().unwrap();
            let count = rd.r.i32().unwrap();
            let len = rd.r.i32().unwrap() as usize;
            let start = rd.r.pos;
            list.push(format!("{prefab}	{count}"));
            for _ in 0..count {
                let pos: Vec<f32> = (0..3).map(|_| rd.r.f32().unwrap()).collect();
                rd.r.take(4 * 7 + 1).unwrap();
                let n = rd.r.i32().unwrap();
                for _ in 0..n {
                    let name = rd.r.name().unwrap();
                    let blen = rd.r.i32().unwrap() as usize;
                    let at = rd.r.pos;
                    if name == "WorldContainer"
                        || name == "ClusterGridEntity"
                        || name == "Clustercraft"
                    {
                        let v = rd.template(&name, "").unwrap_or(Value::Null);
                        worlds.push(serde_json::json!({"prefab": prefab, "behaviour": name, "pos": pos, "data": v}));
                    }
                    rd.r.pos = at + blen;
                }
            }
            assert_eq!(rd.r.pos, start + len, "{prefab}");
        }
        std::fs::write(
            format!("{out}.prefabs.txt"),
            list.join(
                "
",
            ),
        )
        .unwrap();
        std::fs::write(
            format!("{out}.worlds.json"),
            serde_json::to_string_pretty(&worlds).unwrap(),
        )
        .unwrap();
        println!("{} prefabs, {} world records", list.len(), worlds.len());
    }
}

#[cfg(test)]
mod map_tests {
    use super::*;

    /// Research aid: ONI_SAVE = a copy of a save. Reads the map, paints, renames a rocket and
    /// reads the result back.
    #[test]
    #[ignore]
    fn map_round_trip() {
        let d = std::fs::read(std::env::var("ONI_SAVE").expect("ONI_SAVE")).unwrap();
        let t0 = std::time::Instant::now();
        let mut doc = parse(&d).unwrap();
        println!("parsed in {:?}", t0.elapsed());
        assert_eq!(doc.to_bytes().unwrap(), d);
        let map = doc.attachment().unwrap().downcast::<OniMap>().unwrap();
        println!(
            "grid {}x{}, visible {:?}, protected {}",
            map.width,
            map.height,
            map.visible
                .as_ref()
                .map(|v| v.iter().filter(|&&b| b > 0).count()),
            map.protected.iter().filter(|&&p| p).count()
        );
        for w in &map.worlds {
            println!(
                "world {} {:?} rocket={} at {},{} {}x{} disc={} start={}",
                w.id, w.name, w.rocket, w.x, w.y, w.w, w.h, w.discovered, w.start
            );
        }
        let mut kinds = std::collections::BTreeMap::new();
        for p in &map.pois {
            *kinds.entry((p.kind, p.name.clone())).or_insert(0) += 1;
        }
        println!("pois {kinds:?}");
        println!("starmap {:?}", map.starmap);
        // Tile buildings should sit on solid cells.
        let solid = map
            .protected
            .iter()
            .enumerate()
            .filter(|(i, &p)| {
                p && oni_map::element(map.cells[*i].element)
                    .is_some_and(|e| e.state == oni_map::State::Solid)
            })
            .count();
        println!("protected cells that are solid: {solid}");
        println!(
            "view keys {:?}",
            doc.view.as_object().unwrap().keys().collect::<Vec<_>>()
        );
        println!("worlds {}", doc.view["Worlds"]);
        // Paint a 3x3 of granite in the middle of the start world, rename a rocket.
        let w = map.worlds.iter().find(|w| w.start).unwrap();
        let (cx, cy) = ((w.x + w.w / 2) as u32, (w.y + w.h / 2) as u32);
        let op = oni_map::Op {
            mode: oni_map::PAINT,
            world: w.id as u16,
            element: oni_map::hash("Granite"),
            mass: 1234.5,
            temp: 300.0,
            size: 3,
            points: vec![(cx, cy)],
        };
        let rev = oni_map::Op {
            mode: oni_map::REVEAL,
            world: w.id as u16,
            element: 0,
            mass: 0.0,
            temp: 0.0,
            size: 0,
            points: vec![],
        };
        doc.set(
            oni_map::EDITS,
            Value::from(format!("{};{}", op.to_text(), rev.to_text())),
        )
        .unwrap();
        let rocket = doc.view["Rockets"]
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .clone();
        doc.set(
            &format!("/Rockets/{}/Name", esc(&rocket)),
            Value::from("Renamed Rocket With A Longer Name"),
        )
        .unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        let m2 = again.attachment().unwrap().downcast::<OniMap>().unwrap();
        let i = cy as usize * map.width + cx as usize;
        println!("before {:?} after {:?}", map.cells[i], m2.cells[i]);
        assert!(
            m2.cells[i].element == oni_map::hash("Granite")
                || map.protected[i]
                || map.cells[i].element == oni_map::neutronium()
        );
        let changed = map
            .cells
            .iter()
            .zip(&m2.cells)
            .filter(|(a, b)| a != b)
            .count();
        println!("cells changed {changed}");
        assert!(changed <= 9);
        assert!(again.view["Rockets"]
            .get("Renamed Rocket With A Longer Name")
            .is_some());
        let v2 = m2.visible.as_ref().unwrap();
        assert!(v2[(w.y + 1) * map.width + w.x + 1] == 255);
        assert_eq!(
            again.view["Duplicants"].as_object().unwrap().len(),
            doc.view["Duplicants"].as_object().unwrap().len()
        );
    }
}
