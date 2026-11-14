//! Advanced tree view of structured data (JSON, Ren'Py). Read-only traversal;
//! edits are returned through `TreeCtx::edits`.

use crate::names::{self, Names};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, text::LayoutJob, CollapsingHeader, Color32, FontId, TextFormat, TextStyle, Ui};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

/// Containers with more children than this are split into collapsible pages.
const PAGE: usize = 100;

pub fn pointer_escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

pub struct TreeCtx<'a> {
    pub names: &'a Names,
    pub editable: Option<&'a HashMap<String, usize>>,
    pub baseline: &'a Value,
    pub show_raw_keys: bool,
    /// Screen x where value editors start, so they line up at every depth.
    pub value_x: f32,
    pub edits: Vec<(String, Value)>,
    /// The structured document, for value ranges and read-only reasons.
    pub tree: Option<&'a crate::tree::TreeDoc>,
    /// With a search: the pointers to show (matches and the groups holding them).
    pub visible: Option<&'a HashSet<String>>,
    /// A value to reveal: its groups are opened and it is highlighted.
    pub reveal: Option<&'a str>,
    /// Open the groups leading to `reveal` and scroll there (the first moments only).
    pub reveal_scroll: bool,
}

fn leads_to(reveal: Option<&str>, ptr: &str) -> bool {
    reveal.is_some_and(|r| r.len() > ptr.len() && r.starts_with(ptr) && r.as_bytes()[ptr.len()] == b'/')
}

/// The pointers a tree search keeps: every value whose name, key or shown value contains
/// `needle` (lower case), every group whose name does (with everything inside), and the
/// groups on the way to a match.
pub fn filter_ptrs(root: &Value, names: &Names, needle: &str) -> HashSet<String> {
    fn walk(names: &Names, needle: &str, key: &str, v: &Value, ptr: &str, force: bool, out: &mut HashSet<String>) -> bool {
        if names.hide_meta {
            if let Value::Object(map) = v {
                let meta = |k: &String| k == "@c" || k == "@";
                if map.contains_key("@a") && map.keys().all(|k| k == "@a" || meta(k)) {
                    return walk(names, needle, key, &map["@a"], &format!("{ptr}/@a"), force, out);
                }
                if map.contains_key("_data") && map.keys().all(|k| k == "_data" || meta(k)) {
                    return walk(names, needle, key, &map["_data"], &format!("{ptr}/_data"), force, out);
                }
            }
        }
        let label = names::node_label(names, key, ptr, v).to_lowercase();
        let named = force || label.contains(needle) || key.to_lowercase().contains(needle);
        let hidden = |k: &str| names.hide_meta && (k == "@c" || k == "@");
        let mut any = false;
        match v {
            Value::Object(map) => {
                for (k, c) in map.iter().filter(|(k, _)| !hidden(k)) {
                    any |= walk(names, needle, k, c, &format!("{ptr}/{}", pointer_escape(k)), named, out);
                }
            }
            Value::Array(arr) => {
                for (i, c) in arr.iter().enumerate() {
                    any |= walk(names, needle, &i.to_string(), c, &format!("{ptr}/{i}"), named, out);
                }
            }
            scalar => any = widgets::display(scalar).to_lowercase().contains(needle),
        }
        let keep = any || named;
        if keep {
            out.insert(ptr.to_string());
        }
        keep
    }
    let mut out = HashSet::new();
    let needle = needle.trim().to_lowercase();
    match root {
        Value::Object(map) => {
            for (k, c) in map.iter().filter(|(k, _)| !(names.hide_meta && (*k == "@c" || *k == "@"))) {
                walk(names, &needle, k, c, &format!("/{}", pointer_escape(k)), false, &mut out);
            }
        }
        Value::Array(arr) => {
            for (i, c) in arr.iter().enumerate() {
                walk(names, &needle, &i.to_string(), c, &format!("/{i}"), false, &mut out);
            }
        }
        other => {
            walk(names, &needle, "value", other, "", false, &mut out);
        }
    }
    out
}

fn items(n: usize) -> String {
    if n == 1 { "1 item".into() } else { format!("{n} items") }
}

impl TreeCtx<'_> {
    fn hidden(&self, key: &str) -> bool {
        self.names.hide_meta && (key == "@c" || key == "@")
    }
}

fn label_job(ui: &Ui, main: &str, raw: Option<&str>, suffix: &str, color: Color32) -> LayoutJob {
    let font = TextStyle::Body.resolve(ui.style());
    let small = FontId::new(font.size * 0.85, font.family.clone());
    let mut job = LayoutJob::default();
    job.append(main, 0.0, TextFormat::simple(font, color));
    if !suffix.is_empty() {
        job.append(suffix, 6.0, TextFormat::simple(small.clone(), ui.visuals().weak_text_color()));
    }
    if let Some(raw) = raw {
        job.append(raw, 8.0, TextFormat::simple(small, Color32::from_rgb(120, 150, 185)));
    }
    job
}

/// Render the children of the root directly (no redundant root node).
pub fn tree_ui(ui: &mut Ui, root: &Value, ctx: &mut TreeCtx) {
    match root {
        Value::Object(map) => object_children(ui, map, "", ctx),
        Value::Array(arr) => {
            for (i, c) in arr.iter().enumerate() {
                node_ui(ui, &i.to_string(), c, &format!("/{i}"), ctx);
            }
        }
        other => node_ui(ui, "value", other, "", ctx),
    }
}

fn object_children(ui: &mut Ui, map: &Map<String, Value>, ptr: &str, ctx: &mut TreeCtx) {
    let entries: Vec<(&String, &Value)> = map.iter().filter(|(k, _)| !ctx.hidden(k)).collect();
    let on_path = |&(k, _): &(&String, &Value)| format!("{ptr}/{}", pointer_escape(k));
    paged(ui, ptr, &entries, ctx, on_path, |ui, (k, child), ctx| {
        node_ui(ui, k, child, &format!("{ptr}/{}", pointer_escape(k)), ctx);
    });
}

fn node_ui(ui: &mut Ui, key: &str, v: &Value, ptr: &str, ctx: &mut TreeCtx) {
    if ctx.names.hide_meta {
        if let Value::Object(map) = v {
            let meta = |k: &String| k == "@c" || k == "@";
            if map.contains_key("@a") && map.keys().all(|k| k == "@a" || meta(k)) {
                return node_ui(ui, key, &map["@a"], &format!("{ptr}/@a"), ctx);
            }
            // RPG Maker's {"_data": [...]} wrappers add a level with no meaning.
            if map.contains_key("_data") && map.keys().all(|k| k == "_data" || meta(k)) {
                return node_ui(ui, key, &map["_data"], &format!("{ptr}/_data"), ctx);
            }
        }
    }
    if ctx.visible.is_some_and(|vis| !vis.contains(ptr)) {
        return;
    }
    let label = names::node_label(ctx.names, key, ptr, v);
    let raw = (ctx.show_raw_keys && names::raw_differs(key, &label)).then_some(key);
    match v {
        Value::Object(map) => {
            let n = map.keys().filter(|k| !ctx.hidden(k)).count();
            let job = label_job(ui, &label, raw, &items(n), ui.visuals().strong_text_color());
            let open = open_state(ctx, ptr);
            CollapsingHeader::new(job).id_salt(ptr).open(open).show(ui, |ui| object_children(ui, map, ptr, ctx));
        }
        Value::Array(arr) => {
            let job = label_job(ui, &label, raw, &items(arr.len()), ui.visuals().strong_text_color());
            let open = open_state(ctx, ptr);
            CollapsingHeader::new(job).id_salt(ptr).open(open).show(ui, |ui| {
                let entries: Vec<(usize, &Value)> = arr.iter().enumerate().collect();
                paged(ui, ptr, &entries, ctx, |&(i, _): &(usize, &Value)| format!("{ptr}/{i}"), |ui, (i, child), ctx| node_ui(ui, &i.to_string(), child, &format!("{ptr}/{i}"), ctx));
            });
        }
        _ => {
            let editable = ctx.editable.is_none_or(|e| e.contains_key(ptr));
            let modified = ctx.baseline.pointer(ptr) != Some(v);
            let revealed = ctx.reveal == Some(ptr);
            let row = ui.horizontal(|ui| {
                let color = if modified { theme::MODIFIED } else { ui.visuals().text_color() };
                let label_w = (ctx.value_x - ui.cursor().min.x).max(120.0);
                let job = label_job(ui, &label, raw, "", color);
                ui.allocate_ui_with_layout(egui::vec2(label_w, ui.spacing().interact_size.y), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_width(label_w);
                    ui.add(egui::Label::new(job).truncate()).on_hover_text(crate::i18n::trf("key: {0}\npath: {1}", &[&key, &ptr]));
                });
                let opts = widgets::EditOpts { tree: ctx.tree, baseline: Some(ctx.baseline), name: Some(&label), kind: ctx.tree.map(|t| t.kind_name.as_str()), show_in_tree: false };
                if let Some(nv) = widgets::value_editor_ex(ui, ptr, v, editable, 320.0, &opts).edit {
                    ctx.edits.push((ptr.to_string(), nv));
                }
            });
            if revealed {
                let rect = row.response.rect;
                ui.painter().rect_stroke(rect.expand(2.0), 4.0, egui::Stroke::new(1.5_f32, theme::ACCENT));
                if ctx.reveal_scroll {
                    ui.scroll_to_rect(rect, Some(egui::Align::Center));
                }
            }
        }
    }
}

/// Search and "Show in Tree" keep the groups they need open.
fn open_state(ctx: &TreeCtx, ptr: &str) -> Option<bool> {
    if ctx.visible.is_some() || (ctx.reveal_scroll && leads_to(ctx.reveal, ptr)) {
        Some(true)
    } else {
        None
    }
}

fn paged<T: Copy>(ui: &mut Ui, ptr: &str, entries: &[T], ctx: &mut TreeCtx, child_ptr: impl Fn(&T) -> String, mut row: impl FnMut(&mut Ui, T, &mut TreeCtx)) {
    if entries.len() <= PAGE || ctx.visible.is_some() {
        for &e in entries {
            row(ui, e, ctx);
        }
        return;
    }
    let total = entries.len();
    for (page_idx, chunk) in entries.chunks(PAGE).enumerate() {
        let start = page_idx * PAGE;
        let end = (start + PAGE).min(total) - 1;
        let holds_reveal = ctx.reveal_scroll && chunk.iter().any(|e| {
            let c = child_ptr(e);
            ctx.reveal == Some(c.as_str()) || leads_to(ctx.reveal, &c)
        });
        CollapsingHeader::new(format!("{start} \u{2013} {end}"))
            .id_salt(format!("{ptr}#page{start}"))
            .open(holds_reveal.then_some(true))
            .show(ui, |ui| {
                for &e in chunk {
                    row(ui, e, ctx);
                }
            });
    }
}
