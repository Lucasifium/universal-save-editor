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
        let b = self.d.get(self.pos..self.pos + n).ok_or("unexpected end of save")?;
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
        Ok(Some(String::from_utf8_lossy(self.take(n as usize)?).into_owned()))
    }
    fn name(&mut self) -> Result<String, String> {
        self.string()?.ok_or_else(|| "missing name".to_string())
    }
}

fn type_info(r: &mut R) -> Result<TypeInfo, String> {
    let info = r.u8()?;
    let code = info & 63;
    let template = if code == 0 || code == 13 { Some(r.name()?) } else { None };
    let mut sub = vec![];
    if info & 128 != 0 {
        for _ in 0..r.u8()? {
            sub.push(type_info(r)?);
        }
    } else if code == 17 {
        sub.push(type_info(r)?);
    }
    Ok(TypeInfo { code, value_type: info & 64 != 0, template, sub })
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
}

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
}

impl Reader<'_> {
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
            _ => serde_json::Number::from_f64(f64::from_le_bytes(b.try_into().unwrap())).map_or(Value::Null, Value::Number),
        };
        if !(code == 3 && b[0] > 1) && !v.is_null() {
            self.editable.insert(ptr.to_string(), self.leaves.len());
            self.leaves.push(Leaf { at, code });
        }
        Ok(v)
    }

    fn template(&mut self, name: &str, ptr: &str) -> Result<Value, String> {
        let t = self.t.get(name).ok_or_else(|| format!("no template {name}"))?.clone();
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
            12 => Ok(self.r.string()?.map_or(Value::Null, Value::from)),
            14 => Ok(Value::from(vec![self.r.i32()?, self.r.i32()?])),
            15 => Ok(Value::from((0..2).map(|_| self.r.f32().map(|f| f as f64)).collect::<Result<Vec<_>, _>>()?)),
            16 => Ok(Value::from((0..3).map(|_| self.r.f32().map(|f| f as f64)).collect::<Result<Vec<_>, _>>()?)),
            23 => Ok(Value::from(self.r.take(4)?.iter().map(|b| format!("{b:02X}")).collect::<String>())),
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
                    self.r.take(n as usize)?;
                    return Ok(Value::from(format!("({n} bytes)")));
                }
                let mut out = Vec::with_capacity((n as usize).min(4096));
                for i in 0..n as usize {
                    let p = format!("{ptr}/{i}");
                    out.push(if el.value_type && el.code == 0 { self.template(el.template.as_deref().unwrap_or(""), &p)? } else { self.value(el, &p)? });
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
                let (kt, vt) = (ty.sub.first().ok_or("dict")?.clone(), ty.sub.get(1).ok_or("dict")?.clone());
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
                        Value::Object(o) => o.get("hash").map_or_else(|| k.to_string(), |h| h.to_string()),
                        x => x.to_string(),
                    };
                    let key = unique_key(&m, &name);
                    let old = format!("{ptr}/\u{1}{i}");
                    let new = format!("{ptr}/{}", esc(&key));
                    let moved: Vec<(String, usize)> = self.editable.iter().filter(|(p, _)| p.starts_with(&old) && (p.len() == old.len() || p.as_bytes()[old.len()] == b'/')).map(|(p, l)| (p.clone(), *l)).collect();
                    for (p, l) in moved {
                        self.editable.remove(&p);
                        self.editable.insert(format!("{new}{}", &p[old.len()..]), l);
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
    d.len() > 32 && d.get(16..18) == Some(b"{\"") && d[16..d.len().min(4096)].windows(18).any(|w| w == b"numberOfDuplicants")
}

#[derive(Clone, Debug)]
struct OniBackend {
    prefix: Arc<Vec<u8>>,
    body: Arc<Vec<u8>>,
    compressed: bool,
    original: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
}

impl OniBackend {
    fn encode(leaf: &Leaf, v: &Value) -> Result<Vec<u8>, String> {
        use crate::tree::int_in;
        Ok(match leaf.code {
            1 => vec![int_in(v, i8::MIN as i128, i8::MAX as i128, "This value")? as i8 as u8],
            2 => vec![int_in(v, 0, 255, "This value")? as u8],
            3 => vec![u8::from(v.as_bool().ok_or("tick or untick it")?)],
            4 => (int_in(v, i16::MIN as i128, i16::MAX as i128, "This value")? as i16).to_le_bytes().to_vec(),
            5 => (int_in(v, 0, u16::MAX as i128, "This value")? as u16).to_le_bytes().to_vec(),
            6 | 13 => (int_in(v, i32::MIN as i128, i32::MAX as i128, "This value")? as i32).to_le_bytes().to_vec(),
            7 => (int_in(v, 0, u32::MAX as i128, "This value")? as u32).to_le_bytes().to_vec(),
            8 => (int_in(v, i64::MIN as i128, i64::MAX as i128, "This value")? as i64).to_le_bytes().to_vec(),
            9 => (int_in(v, 0, u64::MAX as i128, "This value")? as u64).to_le_bytes().to_vec(),
            10 => (v.as_f64().filter(|f| f.is_finite()).ok_or("expected a number")? as f32).to_le_bytes().to_vec(),
            _ => v.as_f64().filter(|f| f.is_finite()).ok_or("expected a number")?.to_le_bytes().to_vec(),
        })
    }
}

impl Backend for OniBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.original.as_ref().clone());
        }
        let mut body = self.body.as_ref().clone();
        for (l, v) in changes {
            let leaf = &self.leaves[*l];
            let b = Self::encode(leaf, v)?;
            body[leaf.at..leaf.at + b.len()].copy_from_slice(&b);
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
        if self.leaves[leaf].code == 10 {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        Self::encode(&self.leaves[leaf], v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn skip_template(r: &mut R, t: &HashMap<String, Template>, name: &str) -> Result<(), String> {
    let mut rd = Reader { r: R { d: r.d, pos: r.pos }, t, leaves: vec![], editable: HashMap::new() };
    rd.template(name, "")?;
    r.pos = rd.r.pos;
    Ok(())
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
        flate2::read::ZlibDecoder::new(&d[r.pos..]).read_to_end(&mut out).map_err(|e| e.to_string())?;
        out
    } else {
        d[r.pos..].to_vec()
    };

    let mut b = R { d: &body, pos: 0 };
    if b.name()? != "world" {
        return Err("no world in this save".into());
    }
    let root_type = b.name()?;
    skip_template(&mut b, &t, &root_type)?;
    let settings_type = b.name()?;
    skip_template(&mut b, &t, &settings_type)?;
    let sim = b.i32()?;
    b.take(sim.max(0) as usize)?;
    if b.take(4)? != b"KSAV" {
        return Err("KSAV marker missing".into());
    }
    b.take(8)?;

    let mut dupes = Map::new();
    let mut rd = Reader { r: R { d: &body, pos: b.pos }, t: &t, leaves: vec![], editable: HashMap::new() };
    let groups = rd.r.i32()?;
    for _ in 0..groups {
        let prefab = rd.r.name()?;
        let count = rd.r.i32()?;
        let len = rd.r.i32()? as usize;
        let start = rd.r.pos;
        if prefab != "Minion" {
            rd.r.pos = start + len;
            continue;
        }
        for i in 0..count {
            rd.r.take(4 * 10 + 1)?; // position, rotation, scale, folder
            let behaviors = rd.r.i32()?;
            let mut raw: Vec<(String, usize, usize)> = vec![];
            for _ in 0..behaviors {
                let name = rd.r.name()?;
                let blen = rd.r.i32()? as usize;
                raw.push((name, rd.r.pos, blen));
                rd.r.pos += blen;
            }
            let name = raw
                .iter()
                .find(|b| b.0 == "MinionIdentity")
                .and_then(|b| {
                    let mut s = Reader { r: R { d: &body, pos: b.1 }, t: &t, leaves: vec![], editable: HashMap::new() };
                    s.template("MinionIdentity", "").ok().and_then(|v| v["name"].as_str().map(str::to_string))
                })
                .unwrap_or_else(|| format!("Duplicant {}", i + 1));
            let key = unique_key(&dupes, &name);
            let base = format!("/Duplicants/{}", esc(&key));
            let mut m = Map::new();
            let end = rd.r.pos;
            for (bname, at, blen) in raw {
                if !t.contains_key(&bname) {
                    continue;
                }
                rd.r.pos = at;
                let bkey = unique_key(&m, &bname);
                match rd.template(&bname, &format!("{base}/{}", esc(&bkey))) {
                    Ok(v) if rd.r.pos <= at + blen => {
                        m.insert(bkey, v);
                    }
                    _ => {}
                }
            }
            rd.r.pos = end;
            dupes.insert(key, Value::Object(m));
        }
        if rd.r.pos != start + len {
            return Err("duplicant data didn't add up".into());
        }
    }
    let mut colony = Map::new();
    for (k, label) in [("baseName", "Colony"), ("numberOfCycles", "Cycles"), ("numberOfDuplicants", "Duplicants"), ("clusterId", "Cluster")] {
        if let Some(v) = info.get(k) {
            colony.insert(label.into(), v.clone());
        }
    }
    let mut root = Map::new();
    root.insert("Colony".into(), Value::Object(colony));
    root.insert("Duplicants".into(), Value::Object(dupes));
    let (leaves, editable) = (rd.leaves, rd.editable);
    let backend = OniBackend { prefix: Arc::new(prefix), body: Arc::new(body), compressed, original: Arc::new(d.to_vec()), leaves };
    Ok(TreeDoc::new(Value::Object(root), editable, "Oxygen Not Included save", Box::new(backend))
        .with_note("Your duplicants' attributes, skills and other values can be changed; the rest of the colony is kept as it is."))
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
        println!("{:?}: {} editable; {}", t.elapsed(), doc.editable.len(), doc.view["Colony"]);
        for (k, v) in doc.view["Duplicants"].as_object().unwrap().iter().take(2) {
            println!("{k}: {:?}", v.as_object().unwrap().keys().collect::<Vec<_>>());
            for b in ["Klei.AI.AttributeLevels", "MinionResume"] {
                let s = v.get(b).map(|x| x.to_string()).unwrap_or_default();
                println!("  {b}: {}", &s[..s.len().min(1200)]);
            }
        }
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}
