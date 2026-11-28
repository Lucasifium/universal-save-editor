//! Synthetic saves: plain-JSON saves built from a table-driven screen's own spec, with made-up
//! neutral values. They exist for screenshots and for checking that a screen opens, shows its
//! values and writes back. They are NOT real saves: they say nothing about what the game
//! accepts, and the real-save status in `tests/screen-samples.tsv` is not affected.
//!
//!     SAVE_EDITOR_SYNTH_OUT=dir cargo test --release write_synthetic_saves -- --ignored --nocapture

use crate::formats::{decode_with, encode, same_data, Content, Options};
use crate::game_ui::simple::{self, Shape, Spec};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

pub(crate) fn set(root: &mut Value, ptr: &str, v: Value) {
    let segs: Vec<&str> = ptr.split('/').skip(1).collect();
    let mut cur = root;
    for (i, seg) in segs.iter().enumerate() {
        let seg = seg.replace("~1", "/").replace("~0", "~");
        let last = i + 1 == segs.len();
        let next_is_idx = !last && segs[i + 1].parse::<usize>().is_ok();
        if let Ok(ix) = seg.parse::<usize>() {
            if !cur.is_array() {
                *cur = json!([]);
            }
            let a = cur.as_array_mut().unwrap();
            while a.len() <= ix {
                a.push(Value::Null);
            }
            if last {
                a[ix] = v;
                return;
            }
            if a[ix].is_null() {
                a[ix] = if next_is_idx { json!([]) } else { json!({}) };
            }
            cur = &mut a[ix];
        } else {
            if !cur.is_object() {
                *cur = json!({});
            }
            let o = cur.as_object_mut().unwrap();
            if last {
                o.insert(seg, v);
                return;
            }
            cur = o.entry(seg).or_insert_with(|| if next_is_idx { json!([]) } else { json!({}) });
            if cur.is_null() {
                *cur = if next_is_idx { json!([]) } else { json!({}) };
            }
        }
    }
}

/// Whether the field holds words (a name, a type, an id) rather than a number: judged by the last word
/// of its key ("PlayerName", "item_id"), so "maple" or "budget" are not taken for "map" or "id".
fn is_text(ptr: &str) -> bool {
    let key = ptr.rsplit('/').next().unwrap_or("");
    let mut words: Vec<String> = vec![];
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in key.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            words.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let last = words.last().map(String::as_str).unwrap_or("");
    key.to_lowercase().ends_with("guid") || ["id", "guid", "name", "title", "type", "class", "job", "map", "location", "version", "nickname"].contains(&last)
}

/// A made-up, plausible mid-game figure that differs between fields.
fn number(ptr: &str, salt: usize) -> i64 {
    let h = ptr.bytes().fold(salt as u64 + 7, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)) >> 3;
    let l = ptr.rsplit('/').next().unwrap_or("").to_lowercase();
    let has = |ks: &[&str]| ks.iter().any(|k| l.contains(k));
    let r = |lo: u64, hi: u64| (lo + h % (hi - lo + 1)) as i64;
    if l == "max" || l.starts_with("max") {
        r(100, 150)
    } else if has(&["current", "cur"]) {
        r(40, 99)
    } else if has(&["node", "city", "area", "chapter", "act", "difficulty"]) {
        r(1, 6)
    } else if has(&["level", "lvl", "rank", "tier", "grade"]) {
        r(12, 34)
    } else if has(&["health", "hp", "life", "energy", "mana", "stamina", "food", "water", "sleep", "hunger"]) {
        r(60, 100)
    } else if !l.contains("golden") && has(&["gold", "money", "cash", "credit", "coin", "ruble", "unit", "stardust", "gem", "crystal", "token", "fame", "currency"]) {
        r(2_000, 48_000)
    } else if has(&["exp", "xp", "score"]) {
        r(3_000, 42_000)
    } else if has(&["day", "season", "week", "month"]) {
        r(14, 80)
    } else if has(&["death", "kill", "win", "loss", "run", "streak", "count", "found", "complet", "finish", "visit"]) {
        r(8, 160)
    } else if has(&["point", "perk", "skill", "stat", "attribute", "strength", "agility", "intellect", "endurance", "perception", "luck", "charisma"]) {
        r(4, 40)
    } else {
        r(10, 90)
    }
}

pub(crate) fn value_for(ptr: &str, salt: usize) -> Value {
    if is_text(ptr) {
        json!("Player")
    } else {
        json!(number(ptr, salt))
    }
}

fn supported(p: &str) -> bool {
    !p.contains("/=") && !p.contains("/#")
}

/// A JSON save that the spec recognises, filled from the spec's own pointers.
pub fn build(s: &Spec) -> Option<Value> {
    let mut v = json!({});
    let mut ptrs: Vec<&str> = vec![];
    ptrs.extend(s.markers.iter().copied());
    ptrs.extend(s.tiles.iter().map(|t| t.1));
    for sec in s.sections {
        ptrs.extend(sec.rows.iter().map(|r| r.1));
    }
    if !ptrs.iter().all(|p| supported(p)) {
        return None;
    }
    for (i, p) in ptrs.iter().enumerate() {
        if v.pointer(p).is_none() {
            set(&mut v, p, value_for(p, i));
        }
    }
    for l in s.lists {
        if !supported(l.path) {
            return None;
        }
        let names: Vec<&str> = l.names.iter().take(12).map(|n| n.0).collect();
        let ids: Vec<&str> = if names.is_empty() { vec!["item_1", "item_2", "item_3"] } else { names };
        let body = match &l.shape {
            Shape::Array { id_key, qty_keys } => Value::Array(
                ids.iter()
                    .enumerate()
                    .map(|(i, id)| {
                        let mut o = Map::new();
                        o.insert((*id_key).into(), id.parse::<i64>().map(Value::from).unwrap_or_else(|_| json!(id)));
                        for k in qty_keys.iter() {
                            o.insert((*k).into(), json!(3 + i as i64 * 7));
                        }
                        Value::Object(o)
                    })
                    .collect(),
            ),
            Shape::Map => Value::Object(ids.iter().enumerate().map(|(i, id)| (id.to_string(), json!(3 + i as i64 * 7))).collect()),
            Shape::Records { qty_key } => Value::Object(ids.iter().enumerate().map(|(i, id)| (id.to_string(), json!({ *qty_key: 3 + i as i64 * 7 }))).collect()),
        };
        if v.pointer(l.path).is_none() {
            set(&mut v, l.path, body);
        }
    }
    for t in s.tables {
        if !supported(t.path) || t.name.iter().chain(t.cols.iter().map(|c| &c.1)).any(|p| !supported(p)) {
            return None;
        }
        let mut rows = vec![];
        for r in 0..3 {
            let mut row = json!({});
            for (j, n) in t.name.iter().enumerate() {
                set(&mut row, n, json!(format!("Member {}{}", r + 1, if j > 0 { "b" } else { "" })));
            }
            for (j, c) in t.cols.iter().enumerate() {
                set(&mut row, c.1, value_for(c.1, r * 10 + j));
            }
            rows.push(row);
        }
        if v.pointer(t.path).is_none() {
            set(&mut v, t.path, Value::Array(rows));
        }
    }
    Some(v)
}

/// The text formats a made-up save can be written in: (kind, file name to open it under).
fn file_name(s: &Spec) -> Option<&'static str> {
    match s.kind {
        None => Some("Save01.json"),
        Some("XML") => Some("Save01.xml"),
        Some("Lua table") => Some("Save01.lua"),
        Some("INI / config") => Some("Save01.ini"),
        Some(k) if k.starts_with("Unreal Engine save (") => Some("Save01.sav"),
        Some(k) if k.starts_with(".NET BinaryFormatter") => Some("Save01.dat"),
        _ => None,
    }
}

fn jsonish(s: &Spec) -> bool {
    file_name(s).is_some()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn scalar_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn xml_elem(name: &str, v: &Value, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    match v {
        Value::Object(m) => {
            let attrs: String = m.iter().filter(|(k, _)| k.starts_with('@')).map(|(k, v)| format!(" {}=\"{}\"", &k[1..], esc(&scalar_text(v)))).collect();
            let kids: Vec<(&String, &Value)> = m.iter().filter(|(k, _)| !k.starts_with('@') && *k != "#text").collect();
            let text = m.get("#text").map(scalar_text);
            if kids.is_empty() {
                match text {
                    Some(t) => out.push_str(&format!("{pad}<{name}{attrs}>{}</{name}>\n", esc(&t))),
                    None => out.push_str(&format!("{pad}<{name}{attrs} />\n")),
                }
            } else {
                out.push_str(&format!("{pad}<{name}{attrs}>\n"));
                for (k, c) in kids {
                    xml_elem(k, c, depth + 1, out);
                }
                out.push_str(&format!("{pad}</{name}>\n"));
            }
        }
        Value::Array(a) => {
            for it in a {
                xml_elem(name, it, depth, out);
            }
        }
        other => out.push_str(&format!("{pad}<{name}>{}</{name}>\n", esc(&scalar_text(other)))),
    }
}

fn lua_val(v: &Value, out: &mut String) {
    match v {
        Value::Object(m) => {
            out.push('{');
            for (k, c) in m {
                match k.parse::<i64>() {
                    Ok(n) => out.push_str(&format!("[{n}]=")),
                    Err(_) => out.push_str(&format!("[{k:?}]=")),
                }
                lua_val(c, out);
                out.push(',');
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push('{');
            for c in a {
                lua_val(c, out);
                out.push(',');
            }
            out.push('}');
        }
        Value::String(s) => out.push_str(&format!("{s:?}")),
        other => out.push_str(&other.to_string()),
    }
}

fn fstr(s: &str) -> Vec<u8> {
    let mut o = ((s.len() + 1) as i32).to_le_bytes().to_vec();
    o.extend_from_slice(s.as_bytes());
    o.push(0);
    o
}

/// One Unreal property: name, type, size, array index, tag data, no GUID, value.
fn gvas_prop(name: &str, ty: &str, tag: &[u8], value: &[u8]) -> Vec<u8> {
    let mut v = fstr(name);
    v.extend(fstr(ty));
    v.extend((value.len() as i32).to_le_bytes());
    v.extend(0i32.to_le_bytes());
    v.extend(tag);
    v.push(0);
    v.extend(value);
    v
}

fn gvas_props(m: &Map<String, Value>) -> Option<Vec<u8>> {
    let mut o = vec![];
    for (k, v) in m {
        o.extend(gvas_value(k, v)?);
    }
    o.extend(fstr("None"));
    Some(o)
}

fn gvas_value(name: &str, v: &Value) -> Option<Vec<u8>> {
    Some(match v {
        Value::Number(n) => match n.as_i64()? {
            i if i32::try_from(i).is_ok() => gvas_prop(name, "IntProperty", &[], &(i as i32).to_le_bytes()),
            i => gvas_prop(name, "Int64Property", &[], &i.to_le_bytes()),
        },
        Value::String(t) => gvas_prop(name, "StrProperty", &[], &fstr(t)),
        Value::Bool(b) => {
            let mut o = fstr(name);
            o.extend(fstr("BoolProperty"));
            o.extend(0i32.to_le_bytes());
            o.extend(0i32.to_le_bytes());
            o.push(*b as u8);
            o.push(0);
            o
        }
        Value::Object(m) => {
            let mut tag = fstr(name);
            tag.extend([0u8; 16]);
            gvas_prop(name, "StructProperty", &tag, &gvas_props(m)?)
        }
        Value::Array(a) => match a.first() {
            Some(Value::Object(_)) => {
                let mut elems = vec![];
                for it in a {
                    elems.extend(gvas_props(it.as_object()?)?);
                }
                let mut arr = (a.len() as i32).to_le_bytes().to_vec();
                arr.extend(fstr(name));
                arr.extend(fstr("StructProperty"));
                arr.extend((elems.len() as i32).to_le_bytes());
                arr.extend(0i32.to_le_bytes());
                arr.extend(fstr("Elem"));
                arr.extend([0u8; 16]);
                arr.push(0);
                arr.extend(elems);
                gvas_prop(name, "ArrayProperty", &fstr("StructProperty"), &arr)
            }
            Some(Value::Number(_)) => {
                let mut arr = (a.len() as i32).to_le_bytes().to_vec();
                for it in a {
                    arr.extend((it.as_i64()? as i32).to_le_bytes());
                }
                gvas_prop(name, "ArrayProperty", &fstr("IntProperty"), &arr)
            }
            Some(Value::String(_)) => {
                let mut arr = (a.len() as i32).to_le_bytes().to_vec();
                for it in a {
                    arr.extend(fstr(it.as_str()?));
                }
                gvas_prop(name, "ArrayProperty", &fstr("StrProperty"), &arr)
            }
            _ => return None,
        },
        _ => return None,
    })
}

fn gvas(class: &str, v: &Value) -> Option<Vec<u8>> {
    let mut o = b"GVAS".to_vec();
    o.extend(2i32.to_le_bytes());
    o.extend(522i32.to_le_bytes());
    o.extend([4, 0, 27, 0, 2, 0, 1, 2, 3, 4]);
    o.extend(fstr("++UE4+Release-4.27"));
    o.extend(3i32.to_le_bytes());
    o.extend(0i32.to_le_bytes());
    o.extend(fstr(&format!("/Script/Game.{class}")));
    o.extend(gvas_props(v.as_object()?)?);
    o.extend(0i32.to_le_bytes());
    Some(o)
}

fn lps(s: &str) -> Vec<u8> {
    let mut o = vec![];
    let mut n = s.len();
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            o.push(b);
            break;
        }
        o.push(b | 0x80);
    }
    o.extend_from_slice(s.as_bytes());
    o
}

struct Nrbf {
    out: Vec<u8>,
    next: i32,
}

impl Nrbf {
    fn id(&mut self) -> i32 {
        self.next += 1;
        self.next
    }

    fn num(v: &Value) -> Option<i64> {
        v.as_i64()
    }

    /// A class record (type 5) for an object, member values following the metadata.
    fn class(&mut self, name: &str, m: &Map<String, Value>) -> Option<()> {
        let id = self.id();
        // (name, binary type, extra) per member: 0 prim Int32/Int64, 1 string, 4 class, 5 object array, 7 primitive array.
        let mut kinds = vec![];
        for (k, v) in m {
            let bt = match v {
                Value::Number(n) => match n.as_i64()? {
                    i if i32::try_from(i).is_ok() => (0u8, 8u8),
                    _ => (0, 9),
                },
                Value::Bool(_) => (0, 1),
                Value::String(_) => (1, 0),
                Value::Object(_) => (4, 0),
                Value::Array(a) => match a.first() {
                    Some(Value::Number(_)) => (7, 8),
                    Some(Value::Object(_)) => (5, 0),
                    _ => return None,
                },
                _ => return None,
            };
            kinds.push((k, v, bt));
        }
        self.out.push(5);
        self.out.extend(id.to_le_bytes());
        self.out.extend(lps(name));
        self.out.extend((kinds.len() as i32).to_le_bytes());
        for (k, _, _) in &kinds {
            self.out.extend(lps(k));
        }
        for (_, _, (bt, _)) in &kinds {
            self.out.push(*bt);
        }
        for (k, _, (bt, extra)) in &kinds {
            match bt {
                0 | 7 => self.out.push(*extra),
                4 => {
                    self.out.extend(lps(&format!("Game.{k}")));
                    self.out.extend(100i32.to_le_bytes());
                }
                _ => {}
            }
        }
        self.out.extend(100i32.to_le_bytes());
        for (k, v, (bt, extra)) in kinds {
            match (v, bt) {
                (Value::Number(n), 0) => {
                    let i = n.as_i64()?;
                    if extra == 8 {
                        self.out.extend((i as i32).to_le_bytes());
                    } else {
                        self.out.extend(i.to_le_bytes());
                    }
                }
                (Value::Bool(b), 0) => self.out.push(*b as u8),
                (Value::String(t), 1) => {
                    let sid = self.id();
                    self.out.push(6);
                    self.out.extend(sid.to_le_bytes());
                    self.out.extend(lps(t));
                }
                (Value::Object(o), 4) => self.class(&format!("Game.{k}"), o)?,
                (Value::Array(a), 7) => {
                    let aid = self.id();
                    self.out.push(15);
                    self.out.extend(aid.to_le_bytes());
                    self.out.extend((a.len() as i32).to_le_bytes());
                    self.out.push(8);
                    for it in a {
                        self.out.extend((Self::num(it)? as i32).to_le_bytes());
                    }
                }
                (Value::Array(a), 5) => {
                    let aid = self.id();
                    self.out.push(16);
                    self.out.extend(aid.to_le_bytes());
                    self.out.extend((a.len() as i32).to_le_bytes());
                    for it in a {
                        self.class(&format!("Game.{k}Item"), it.as_object()?)?;
                    }
                }
                _ => return None,
            }
        }
        Some(())
    }
}

fn nrbf_message(class: &str, m: &Map<String, Value>) -> Option<Vec<u8>> {
    let mut n = Nrbf { out: vec![], next: 1 };
    n.out.push(0);
    n.out.extend(2i32.to_le_bytes());
    n.out.extend((-1i32).to_le_bytes());
    n.out.extend(1i32.to_le_bytes());
    n.out.extend(0i32.to_le_bytes());
    n.out.push(12);
    n.out.extend(100i32.to_le_bytes());
    n.out.extend(lps("Assembly-CSharp"));
    n.class(&format!("Game.{class}"), m)?;
    n.out.push(11);
    Some(n.out)
}

fn nrbf(kind: &str, v: &Value) -> Option<Vec<u8>> {
    if let Value::Array(a) = v {
        let mut o = vec![];
        for it in a {
            o.extend(nrbf_message("Message", it.as_object()?)?);
        }
        return Some(o);
    }
    let class = kind.trim_start_matches(".NET BinaryFormatter (").trim_end_matches(')');
    nrbf_message(class, v.as_object()?)
}

/// The save's bytes in the screen's own text format (`None` when the layout can't be written).
pub fn render(s: &Spec, v: &Value) -> Option<Vec<u8>> {
    let text = match s.kind {
        None => return serde_json::to_vec_pretty(v).ok(),
        Some("XML") => {
            let (root, body) = v.as_object()?.iter().next()?;
            let mut o = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
            xml_elem(root, body, 0, &mut o);
            o
        }
        Some("Lua table") => {
            let mut o = String::from("return ");
            lua_val(v, &mut o);
            o.push('\n');
            o
        }
        Some("INI / config") => {
            let mut top = String::new();
            let mut secs = String::new();
            for (k, c) in v.as_object()? {
                match c {
                    Value::Object(m) => {
                        secs.push_str(&format!("[{k}]\n"));
                        for (k2, c2) in m {
                            if c2.is_object() || c2.is_array() {
                                return None;
                            }
                            secs.push_str(&format!("{k2}={}\n", scalar_text(c2)));
                        }
                    }
                    Value::Array(_) => return None,
                    other => top.push_str(&format!("{k}={}\n", scalar_text(other))),
                }
            }
            format!("{top}{secs}")
        }
        Some(k) if k.starts_with("Unreal Engine save (") => {
            let class = k.trim_start_matches("Unreal Engine save (").trim_end_matches(')');
            return gvas(class, v);
        }
        Some(k) if k.starts_with(".NET BinaryFormatter") => return nrbf(k, v),
        _ => return None,
    };
    Some(text.into_bytes())
}

fn status() -> BTreeSet<String> {
    include_str!("../tests/screen-samples.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            let (n, st) = (f.next()?, f.next()?);
            (st == "none").then(|| n.to_string())
        })
        .collect()
}

/// Every JSON screen without a real sample: the synthetic save must open on that very screen,
/// hold editable values and write back the same data.
#[test]
fn synthetic_saves_open_on_their_screens() {
    let (mut ok, mut skipped) = (0, vec![]);
    let mut families: std::collections::BTreeMap<&str, usize> = Default::default();
    for s in simple::all().iter().filter(|s| jsonish(s)) {
        let Some(v) = build(s) else { skipped.push(s.name); continue };
        let Some(bytes) = render(s, &v) else { skipped.push(s.name); continue };
        let opts = Options::for_path(std::path::Path::new(file_name(s).unwrap()));
        let d = decode_with(&bytes, &opts);
        let got = crate::sample_check::screen_of(&d);
        if got.as_deref() != Some(s.name) {
            skipped.push(s.name);
            continue;
        }
        let out = encode(&d.layers, &d.content).expect("encode");
        let again = decode_with(&out, &opts);
        assert!(same_data(&again.content, &d.content), "{} does not write back", s.name);
        assert!(matches!(d.content, Content::Json { .. } | Content::Tree(_)));
        ok += 1;
        let family = match s.kind {
            None => "JSON",
            Some("XML") => "XML",
            Some("Lua table") => "Lua",
            Some("INI / config") => "INI",
            Some(k) if k.starts_with("Unreal") => "Unreal",
            _ => ".NET",
        };
        *families.entry(family).or_default() += 1;
    }
    println!("synthetic: {ok} open on their own screen, skipped {}: {:?}", skipped.len(), skipped);
    assert!(ok >= 60, "only {ok} synthetic saves open on their own screen");
    // Every writer must keep working, not just the JSON one.
    for (family, least) in [("JSON", 60), ("XML", 10), ("Lua", 2), ("INI", 1), ("Unreal", 15), (".NET", 8)] {
        let got = families.get(family).copied().unwrap_or(0);
        assert!(got >= least, "only {got} {family} synthetic saves open on their own screen (need {least}): {skipped:?}");
    }
}

#[test]
#[ignore]
fn write_synthetic_saves() {
    let Ok(dir) = std::env::var("SAVE_EDITOR_SYNTH_OUT") else { return };
    let none = status();
    std::fs::create_dir_all(&dir).unwrap();
    for s in simple::all().iter().filter(|s| jsonish(s) && none.contains(s.name)) {
        let Some(v) = build(s) else { println!("skip {} (unsupported pointers)", s.name); continue };
        let Some(bytes) = render(s, &v) else { println!("skip {} (cannot write the format)", s.name); continue };
        let ext = file_name(s).unwrap().rsplit('.').next().unwrap();
        let d = decode_with(&bytes, &Options::for_path(std::path::Path::new(&format!("Save01.{ext}"))));
        if crate::sample_check::screen_of(&d).as_deref() != Some(s.name) {
            println!("skip {} (opens elsewhere)", s.name);
            continue;
        }
        let slug: String = s.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        std::fs::write(format!("{dir}/{slug}.{ext}"), bytes).unwrap();
        println!("wrote {}", s.name);
    }
}

#[test]
#[ignore]
fn list_synthetic_subtitles() {
    let none = status();
    for s in simple::all().iter().filter(|s| jsonish(s) && none.contains(s.name)) {
        println!("## {} :: {}", s.name, s.subtitle);
    }
}

#[test]
#[ignore]
fn list_kind_specs() {
    let none = status();
    for s in simple::all().iter().filter(|s| s.kind.is_some() && none.contains(s.name)) {
        let n: usize = s.sections.iter().map(|x| x.rows.len()).sum();
        println!("{} :: {} :: markers {:?} rows {} lists {} tables {}", s.name, s.kind.unwrap(), s.markers, n, s.lists.len(), s.tables.len());
    }
}

#[test]
#[ignore]
fn list_hard_screens() {
    let none = status();
    let simple_names: BTreeSet<&str> = simple::all().iter().map(|s| s.name).collect();
    for s in simple::all().iter().filter(|s| none.contains(s.name) && !jsonish(s)) {
        println!("HARD-SIMPLE\t{}\t{}", s.name, s.kind.unwrap_or(""));
    }
    for n in none.iter().filter(|n| !simple_names.contains(n.as_str())) {
        println!("HARD-BESPOKE\t{n}");
    }
}
