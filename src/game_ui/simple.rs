//! Table-driven screens for games whose save is a plain tree (JSON and the like): each game is
//! a `Spec` (how to recognise its save, which values go in which panel, which item lists to
//! show with names, and which stacks a "Max crafting materials" button may raise).
//!
//! Values are found by pointer and simply left out when a save doesn't have them.

use crate::widgets::ScrollLock;
use super::{columns, hero, panel, GameCtx};
use eframe::egui::{self, Color32, Ui};
use serde_json::Value;

mod specs;
pub use specs::SPECS;

/// A panel of labelled values: `(label, pointer)`.
pub struct Section {
    pub title: &'static str,
    pub rows: &'static [(&'static str, &'static str)],
}

/// How the items of a list are stored.
#[allow(dead_code)]
pub enum Shape {
    /// An array of objects: `id_key` names the item, `qty_keys[0]` is the count the screen
    /// edits and the others (a reserved or unreserved copy) are kept equal to it.
    Array { id_key: &'static str, qty_keys: &'static [&'static str] },
    /// An object of `name: count` (the name is the id).
    Map,
    /// An object of records keyed by id, each holding its count under `qty_key`.
    Records { qty_key: &'static str },
}

/// The "Max crafting materials" button of a list.
pub struct Max {
    /// Ids (or names, for a map) of the plain crafting materials the button may raise.
    pub ids: &'static [&'static str],
    pub cap: i64,
    /// Caps of single ids that differ from `cap`.
    pub caps: &'static [(&'static str, i64)],
    /// True when no public source states the cap and it is an estimate.
    pub estimate: bool,
    /// What the button does, for its hover text.
    pub about: &'static str,
}

pub struct List {
    pub title: &'static str,
    pub path: &'static str,
    pub shape: Shape,
    /// `(id, name)`; ids without a name are shown as "Item <id>".
    pub names: &'static [(&'static str, &'static str)],
    pub max: Option<Max>,
}

/// An array of objects shown as a table, one row each (dwellers, party members…).
pub struct Table {
    pub title: &'static str,
    pub path: &'static str,
    /// Pointers inside an entry whose text makes up the row's name.
    pub name: &'static [&'static str],
    /// Editable columns: `(heading, pointer inside the entry)`.
    pub cols: &'static [(&'static str, &'static str)],
    /// Only rows whose name is one of these (empty: every row).
    pub only: &'static [&'static str],
    /// `(text, name)`: names for rows whose name text is an id.
    pub names: &'static [(&'static str, &'static str)],
}

pub struct Spec {
    pub name: &'static str,
    pub tab: &'static str,
    pub accent: (u8, u8, u8),
    /// The kind name the reader gives the save (`None`: a plain JSON save).
    pub kind: Option<&'static str>,
    /// Pointers that must all exist for the save to be this game's.
    pub markers: &'static [&'static str],
    pub subtitle: &'static str,
    /// Big editable figures at the top: `(caption, pointer)`.
    pub tiles: &'static [(&'static str, &'static str)],
    pub sections: &'static [Section],
    pub lists: &'static [List],
    pub tables: &'static [Table],
}

impl Spec {
    fn matches(&self, kind: &str, root: &Value) -> bool {
        let kind_ok = match self.kind {
            Some(k) => kind == k,
            None => kind == "JSON",
        };
        kind_ok && self.markers.iter().all(|m| root.pointer(&resolve(root, m)).is_some())
    }
}

/// Index of the spec that recognises this save.
pub fn detect(kind: &str, root: &Value) -> Option<u16> {
    SPECS.iter().position(|s| s.matches(kind, root)).map(|i| i as u16)
}

pub fn game_name(i: u16) -> &'static str {
    SPECS[i as usize].name
}

pub fn tab_label(i: u16) -> &'static str {
    SPECS[i as usize].tab
}

/// Resolves `@<hash>` steps in a spec pointer: inside an array of managers that each carry a
/// `_Hash` (RE Engine saves, whose lists change order between saves), `@123` means the entry
/// whose `_Hash` is 123.
pub fn resolve(root: &Value, p: &str) -> String {
    if !p.contains("/@") && !p.contains("/=") {
        return p.to_string();
    }
    let mut out = String::new();
    let mut cur = Some(root);
    for seg in p.split('/').skip(1) {
        let seg = match (seg.strip_prefix('@').and_then(|h| h.parse::<u64>().ok()), cur) {
            (Some(h), Some(Value::Array(a))) => match a.iter().position(|e| e.get("_Hash").and_then(Value::as_u64) == Some(h)) {
                Some(i) => i.to_string(),
                None => return "/\u{0}no such entry".to_string(),
            },
            // `=name`: the entry whose `key` is that name (database tables of key/data rows).
            (None, Some(Value::Array(a))) if seg.starts_with('=') => match a.iter().position(|e| e.get("key").and_then(Value::as_str) == Some(&seg[1..])) {
                Some(i) => i.to_string(),
                None => return "/\u{0}no such entry".to_string(),
            },
            _ => seg.to_string(),
        };
        cur = match cur {
            Some(Value::Array(a)) => seg.parse::<usize>().ok().and_then(|i| a.get(i)),
            Some(v) => v.get(seg.replace("~1", "/").replace("~0", "~")),
            None => None,
        };
        out.push('/');
        out.push_str(&seg);
    }
    out
}

fn qty_path(base: &str, i: usize, key: &str) -> String {
    // A key that starts with "/" is a pointer inside the entry (an item record nested in the entry).
    if key.starts_with('/') {
        return format!("{base}/{i}{key}");
    }
    format!("{base}/{i}/{}", crate::json_view::pointer_escape(key))
}

/// A field of an array entry: plain key, or a pointer inside the entry when it starts with "/".
fn field<'a>(e: &'a Value, key: &str) -> Option<&'a Value> {
    if key.starts_with('/') {
        e.pointer(key)
    } else {
        e.get(key)
    }
}

/// Puts a field into a new array entry, nesting objects for a pointer key.
#[cfg(test)]
fn put_field(o: &mut serde_json::Map<String, Value>, key: &str, v: Value) {
    match key.strip_prefix('/').and_then(|k| k.split_once('/')) {
        Some((head, rest)) => {
            let inner = o.entry(head.to_string()).or_insert_with(|| serde_json::json!({}));
            put_field(inner.as_object_mut().unwrap(), &format!("/{rest}"), v);
        }
        None => {
            o.insert(key.trim_start_matches('/').to_string(), v);
        }
    }
}

/// What one list entry is: (id as text, pointer of its count).
fn entries(cx: &GameCtx, l: &List) -> Vec<(String, String)> {
    let path = resolve(cx.root, l.path);
    let l_path = path.as_str();
    let Some(v) = cx.get(l_path) else { return vec![] };
    match (&l.shape, v) {
        (Shape::Array { id_key, qty_keys }, Value::Array(a)) => a
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let id = match field(e, id_key)? {
                    Value::String(s) => s.clone(),
                    n => n.as_i64().map(|n| n.to_string()).or_else(|| n.as_f64().map(|f| f.to_string()))?,
                };
                field(e, qty_keys[0])?.as_f64()?;
                Some((id, qty_path(l_path, i, qty_keys[0])))
            })
            .collect(),
        (Shape::Map, Value::Object(m)) => m
            .iter()
            // "@c" is RPG Maker MV's object-reference counter, not an item.
            .filter(|(k, v)| v.is_number() && k.as_str() != "@c")
            .map(|(k, _)| (k.clone(), format!("{}/{}", l_path, crate::json_view::pointer_escape(k))))
            .collect(),
        (Shape::Records { qty_key }, Value::Object(m)) => m
            .iter()
            .filter(|(_, v)| v.get(*qty_key).is_some_and(|n| n.is_number()))
            .map(|(k, _)| (k.clone(), format!("{}/{}/{}", l_path, crate::json_view::pointer_escape(k), crate::json_view::pointer_escape(qty_key))))
            .collect(),
        _ => vec![],
    }
}

fn name_of(l: &List, id: &str) -> String {
    // A map keyed by names (a binary save read by name) needs no table of names.
    if l.names.is_empty() && (matches!(l.shape, Shape::Map) || id.parse::<f64>().is_err()) {
        return id.to_string();
    }
    // Text ids (a game's own item names) read fine as they are; numeric ids get "Item <id>".
    l.names.iter().find(|(i, _)| *i == id).map_or_else(
        || if id.parse::<f64>().is_ok() { format!("Item {id}") } else { id.to_string() },
        |(_, n)| n.to_string(),
    )
}

/// Sets a list entry's count, and its mirrored copies.
fn set_count(cx: &mut GameCtx, l: &List, count_path: &str, v: Value) {
    if let Shape::Array { qty_keys, .. } = &l.shape {
        if qty_keys[0].starts_with('/') {
            // Pointer keys: a copy follows the count only while it equals it (a grenade's
            // charges are not its count).
            let old = cx.get(count_path).cloned();
            if let Some(base) = count_path.strip_suffix(qty_keys[0]) {
                for k in qty_keys.iter().skip(1) {
                    let p = format!("{base}{k}");
                    if cx.get(&p).is_some() && cx.get(&p).cloned() == old {
                        cx.set(&p, v.clone());
                    }
                }
            }
        } else if let Some((base, _)) = count_path.rsplit_once('/') {
            for k in qty_keys.iter().skip(1) {
                let p = format!("{base}/{}", crate::json_view::pointer_escape(k));
                if cx.get(&p).is_some() {
                    cx.set(&p, v.clone());
                }
            }
        }
    }
    cx.set(count_path, v);
}

/// What "Max crafting materials" does on one list; returns the stacks raised.
pub fn apply_max(cx: &mut GameCtx, l: &List) -> usize {
    let Some(m) = &l.max else { return 0 };
    let mut n = 0;
    for (id, p) in entries(cx, l) {
        if !m.ids.contains(&id.as_str()) {
            continue;
        }
        let cap = m.caps.iter().find(|(c, _)| *c == id).map_or(m.cap, |(_, c)| *c);
        if cx.num(&p).is_some_and(|have| have < cap as f64) {
            set_count(cx, l, &p, Value::from(cap));
            n += 1;
        }
    }
    n
}

fn filter(ui: &mut Ui, id: &str) -> String {
    let sid = egui::Id::new(("simple_filter", id));
    let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add(egui::TextEdit::singleline(&mut q).desired_width(200.0));
    });
    ui.data_mut(|d| d.insert_temp(sid, q.clone()));
    q.to_lowercase()
}

pub fn show(i: u16, ui: &mut Ui, cx: &mut GameCtx) {
    let s = &SPECS[i as usize];
    let accent = Color32::from_rgb(s.accent.0, s.accent.1, s.accent.2);
    let tiles: Vec<(&str, String)> = s.tiles.iter().map(|(c, p)| (*c, resolve(cx.root, p))).collect();
    hero(ui, accent, s.name, s.subtitle, |ui| {
        for (cap, p) in &tiles {
            if cx.get(p).is_some() {
                cx.tile(ui, cap, p, accent);
            }
        }
    });
    let rows_of = |sec: &Section| -> Vec<(&'static str, String)> { sec.rows.iter().map(|(l, p)| (*l, resolve(cx.root, p))).collect() };
    let visible: Vec<(&Section, Vec<(&str, String)>)> =
        s.sections.iter().map(|sec| (sec, rows_of(sec))).filter(|(_, rows)| rows.iter().any(|(_, p)| cx.get(p).is_some())).collect();
    columns(ui, 360.0, visible.len(), |ui, k| {
        let (sec, rows) = &visible[k];
        panel(ui, sec.title, |ui| {
            egui::Grid::new(("simple_grid", i, k)).num_columns(2).spacing([14.0, 5.0]).show(ui, |ui| {
                for (label, p) in rows {
                    cx.grid_row(ui, label, p, 110.0);
                }
            });
        });
    });
    for (ti, t) in s.tables.iter().enumerate() {
        // Rows of an array are numbered; rows of an object (the nodes of a text save, or the
        // named records of a binary one) are its keys, which name the row when `name` is empty.
        let t_path = resolve(cx.root, t.path);
        let t_path = t_path.as_str();
        let rows: Vec<(String, String)> = match cx.get(t_path) {
            Some(Value::Array(a)) => (0..a.len()).map(|i| (i.to_string(), String::new())).collect(),
            Some(Value::Object(o)) if !o.is_empty() && o.iter().all(|(k, v)| v.is_object() && !k.starts_with('@')) => {
                o.keys().map(|k| (k.replace('~', "~0").replace('/', "~1"), k.clone())).collect()
            }
            _ => Vec::new(),
        };
        let n = rows.len();
        if n == 0 {
            continue;
        }
        panel(ui, &format!("{} ({n})", t.title), |ui| {
            let q = filter(ui, &format!("t{i}/{ti}"));
            egui::ScrollArea::horizontal().id_salt(("simple_table_scroll", i, ti)).show_locked(ui, |ui| {
                egui::Grid::new(("simple_table", i, ti)).num_columns(t.cols.len() + 1).spacing([8.0, 4.0]).striped(true).show(ui, |ui| {
                    ui.label("");
                    for (h, _) in t.cols {
                        ui.label(egui::RichText::new(*h).strong());
                    }
                    ui.end_row();
                    for (r, (key, key_name)) in rows.iter().enumerate() {
                        let name = if t.name.is_empty() {
                            key_name.clone()
                        } else {
                            t.name.iter().filter_map(|k| cx.text(&format!("{t_path}/{key}{k}"))).collect::<Vec<_>>().join(" ")
                        };
                        if !q.is_empty() && !name.to_lowercase().contains(&q) {
                            continue;
                        }
                        if !t.only.is_empty() && !t.only.contains(&name.as_str()) {
                            continue;
                        }
                        let name = t.names.iter().find(|(id, _)| *id == name).map_or(name, |(_, n)| n.to_string());
                        ui.label(if name.is_empty() { format!("#{}", r + 1) } else { name });
                        for (_, k) in t.cols {
                            let p = format!("{t_path}/{key}{k}");
                            if cx.get(&p).is_some() {
                                cx.cell(ui, &p, 64.0);
                            } else {
                                ui.label("");
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }
    for (li, l) in s.lists.iter().enumerate() {
        let list = entries(cx, l);
        if list.is_empty() {
            continue;
        }
        panel(ui, l.title, |ui| {
            if let Some(m) = &l.max {
                let tip = format!("{}{}", m.about, if m.estimate { " The cap is an estimate from community guides." } else { "" });
                // With several lists, the label names which one the action is for.
                let many = s.lists.iter().filter(|x| x.max.is_some()).count() > 1;
                let label = if many { format!("Max crafting materials: {} ({})", l.title, m.cap) } else { format!("Max crafting materials ({})", m.cap) };
                if super::quick_button(ui, label, tip) {
                    apply_max(cx, l);
                }
            }
            let q = filter(ui, &format!("{i}/{li}"));
            egui::Grid::new(("simple_list", i, li)).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                for (id, p) in &list {
                    let name = name_of(l, id);
                    if !q.is_empty() && !name.to_lowercase().contains(&q) {
                        continue;
                    }
                    cx.label(ui, &name, p);
                    let n = cx.edits.len();
                    ui.horizontal(|ui| cx.editor(ui, p, 110.0));
                    if cx.edits.len() > n {
                        if let Some((_, v)) = cx.edits.last().cloned() {
                            cx.edits.pop();
                            set_count(cx, l, p, v);
                        }
                    }
                    ui.end_row();
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn put(root: &mut Value, ptr: &str, v: Value) {
        let mut cur = root;
        let parts: Vec<String> = ptr.split('/').skip(1).map(|s| s.replace("~1", "/").replace("~0", "~")).collect();
        for (i, k) in parts.iter().enumerate() {
            if !cur.is_object() {
                *cur = json!({});
            }
            let o = cur.as_object_mut().unwrap();
            if i + 1 == parts.len() {
                o.entry(k.clone()).or_insert(v);
                return;
            }
            cur = o.entry(k.clone()).or_insert(json!({}));
        }
    }

    /// A made-up save with every value of the spec in it.
    fn synthetic(s: &Spec) -> Value {
        let mut root = json!({});
        for m in s.markers {
            put(&mut root, m, json!(5));
        }
        for (_, p) in s.tiles {
            put(&mut root, p, json!(5));
        }
        for sec in s.sections {
            for (_, p) in sec.rows {
                put(&mut root, p, json!(5));
            }
        }
        for l in s.lists {
            let mut ids: Vec<&str> = l.max.as_ref().map_or(vec![], |m| m.ids.to_vec());
            ids.push("999999");
            match &l.shape {
                Shape::Array { id_key, qty_keys } => {
                    let arr: Vec<Value> = ids
                        .iter()
                        .map(|id| {
                            let mut o = serde_json::Map::new();
                            put_field(&mut o, id_key, id.parse::<i64>().map_or_else(|_| json!(id), |n| json!(n)));
                            let low = l.max.as_ref().is_some_and(|m| m.ids.contains(id));
                            for k in *qty_keys {
                                put_field(&mut o, k, json!(if low { 0 } else { 3 }));
                            }
                            Value::Object(o)
                        })
                        .collect();
                    root.pointer_mut("/").map(|_| ());
                    set_path(&mut root, l.path, Value::Array(arr));
                }
                Shape::Map => {
                    let o: serde_json::Map<String, Value> =
                        ids.iter().map(|id| (id.to_string(), json!(if l.max.as_ref().is_some_and(|m| m.ids.contains(id)) { 0 } else { 3 }))).collect();
                    set_path(&mut root, l.path, Value::Object(o));
                }
                Shape::Records { qty_key } => {
                    let o: serde_json::Map<String, Value> = ids
                        .iter()
                        .map(|id| (id.to_string(), json!({ *qty_key: if l.max.as_ref().is_some_and(|m| m.ids.contains(id)) { 0 } else { 3 } })))
                        .collect();
                    set_path(&mut root, l.path, Value::Object(o));
                }
            }
        }
        root
    }

    fn set_path(root: &mut Value, ptr: &str, v: Value) {
        let parts: Vec<&str> = ptr.split('/').skip(1).collect();
        let mut cur = root;
        for (i, k) in parts.iter().enumerate() {
            if !cur.is_object() {
                *cur = json!({});
            }
            let o = cur.as_object_mut().unwrap();
            if i + 1 == parts.len() {
                o.insert(k.to_string(), v);
                return;
            }
            cur = o.entry(k.to_string()).or_insert(json!({}));
        }
    }

    #[test]
    fn hash_steps_find_the_entry_wherever_it_sits() {
        let a = json!({"m": [{"_Hash": 5, "x": 1}, {"_Hash": 9, "x": 2}]});
        let b = json!({"m": [{"_Hash": 9, "x": 2}, {"_Hash": 7}, {"_Hash": 5, "x": 1}]});
        for root in [&a, &b] {
            assert_eq!(root.pointer(&resolve(root, "/m/@9/x")).and_then(Value::as_i64), Some(2));
            assert_eq!(root.pointer(&resolve(root, "/m/@5/x")).and_then(Value::as_i64), Some(1));
        }
        assert!(a.pointer(&resolve(&a, "/m/@7/x")).is_none());
    }

    #[test]
    fn every_spec_detects_draws_and_maxes() {
        for (i, s) in SPECS.iter().enumerate() {
            let root = synthetic(s);
            let kind = s.kind.unwrap_or("JSON");
            let found = super::super::detect(kind, &root);
            assert_eq!(found, Some(super::super::Game::Simple(i as u16)), "{} not detected", s.name);
            // Draw it.
            let ctx = egui::Context::default();
            crate::theme::install_fonts(&ctx);
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 800.0))), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut cx = GameCtx { root: &root, baseline: &root, tree: None, edits: vec![], archive: None, open_entry: None };
                    show(i as u16, ui, &mut cx);
                });
            });
            // The max button raises its materials, mirrors the copies and leaves others alone.
            for l in s.lists {
                let Some(m) = &l.max else { continue };
                let mut cx = GameCtx { root: &root, baseline: &root, tree: None, edits: vec![], archive: None, open_entry: None };
                let n = apply_max(&mut cx, l);
                assert_eq!(n, m.ids.len(), "{}: {}", s.name, l.title);
                let mut after = root.clone();
                for (p, v) in &cx.edits {
                    *after.pointer_mut(p).unwrap() = v.clone();
                }
                let arr = after.pointer(l.path).unwrap();
                match &l.shape {
                    Shape::Array { id_key, qty_keys } => {
                        for e in arr.as_array().unwrap() {
                            let id = match field(e, id_key).unwrap() { Value::String(s) => s.clone(), n => n.to_string() };
                            let cap = m.caps.iter().find(|(c, _)| *c == id).map_or(m.cap, |(_, c)| *c);
                            let want = if m.ids.contains(&id.as_str()) { cap } else { 3 };
                            for k in *qty_keys {
                                assert_eq!(field(e, k).unwrap().as_i64(), Some(want), "{} {id} {k}", s.name);
                            }
                        }
                    }
                    Shape::Map => {
                        for (id, v) in arr.as_object().unwrap() {
                            let cap = m.caps.iter().find(|(c, _)| *c == id.as_str()).map_or(m.cap, |(_, c)| *c);
                            let want = if m.ids.contains(&id.as_str()) { cap } else { 3 };
                            assert_eq!(v.as_i64(), Some(want), "{} {id}", s.name);
                        }
                    }
                    Shape::Records { qty_key } => {
                        for (id, v) in arr.as_object().unwrap() {
                            let cap = m.caps.iter().find(|(c, _)| *c == id.as_str()).map_or(m.cap, |(_, c)| *c);
                            let want = if m.ids.contains(&id.as_str()) { cap } else { 3 };
                            assert_eq!(v[*qty_key].as_i64(), Some(want), "{} {id}", s.name);
                        }
                    }
                }
            }
        }
    }

    /// Research: a real save from SAVE_EDITOR_SIMPLE (files separated by `|`) opens on a simple screen.
    #[test]
    #[ignore]
    fn real_saves_open_on_simple_screens() {
        use crate::formats::{decode_with, Content, Options};
        let list = std::env::var("SAVE_EDITOR_SIMPLE").expect("SAVE_EDITOR_SIMPLE");
        for f in list.split('|').filter(|s| !s.is_empty()) {
            let path = std::path::Path::new(f);
            let mut o = Options::for_path(path);
            o.password = std::env::var("SAVE_EDITOR_PASSWORD").ok();
            let d = decode_with(&std::fs::read(path).unwrap(), &o);
            let (kind, root) = match &d.content {
                Content::Tree(t) => (t.kind_name.clone(), t.view.clone()),
                Content::Json { value, .. } => ("JSON".to_string(), value.clone()),
                _ => panic!("{f}: not a tree"),
            };
            let g = super::super::detect(&kind, &root);
            let Some(super::super::Game::Simple(i)) = g else { panic!("{f}: kind {kind:?} detected as {g:?}") };
            let s = &SPECS[i as usize];
            let shown = s.sections.iter().flat_map(|sec| sec.rows).filter(|(_, p)| root.pointer(&resolve(&root, p)).is_some()).count();
            let total: usize = s.sections.iter().map(|sec| sec.rows.len()).sum();
            println!("{f}: {} ({shown} of {total} values present)", s.name);
        }
    }
}

#[cfg(test)]
mod uty_tests {
    #[test]
    #[ignore]
    fn undertale_yellow_sample() {
        let Ok(f) = std::env::var("UTY_SAMPLE") else { return };
        let text = std::fs::read_to_string(f).unwrap();
        let mut t = crate::fmt::ini::parse(&text, "sav").unwrap();
        assert_eq!(super::game_name(super::detect(&t.kind_name, &t.view).expect("spec matches")), "Undertale Yellow");
        t.set("/Save1/Gold", 9999.into()).unwrap();
        let out = String::from_utf8(t.to_bytes().unwrap()).unwrap();
        assert!(out.contains("Gold=\"9999\"") || out.contains("Gold=9999"), "{}", out.lines().find(|l| l.starts_with("Gold")).unwrap_or(""));
    }
}
