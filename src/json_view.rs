//! Advanced tree view of structured data (JSON, Ren'Py). Read-only traversal;
//! edits are returned through `TreeCtx::edits`.

use crate::names::{self, Names};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, text::LayoutJob, CollapsingHeader, Color32, FontId, TextFormat, TextStyle, Ui};
use serde_json::{Map, Value};
use std::collections::HashMap;

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
    paged(ui, ptr, &entries, |ui, (k, child)| {
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
    let label = names::node_label(ctx.names, key, ptr, v);
    let raw = (ctx.show_raw_keys && names::raw_differs(key, &label)).then_some(key);
    match v {
        Value::Object(map) => {
            let n = map.keys().filter(|k| !ctx.hidden(k)).count();
            let job = label_job(ui, &label, raw, &items(n), ui.visuals().strong_text_color());
            CollapsingHeader::new(job).id_salt(ptr).show(ui, |ui| object_children(ui, map, ptr, ctx));
        }
        Value::Array(arr) => {
            let job = label_job(ui, &label, raw, &items(arr.len()), ui.visuals().strong_text_color());
            CollapsingHeader::new(job).id_salt(ptr).show(ui, |ui| {
                let entries: Vec<(usize, &Value)> = arr.iter().enumerate().collect();
                paged(ui, ptr, &entries, |ui, (i, child)| node_ui(ui, &i.to_string(), child, &format!("{ptr}/{i}"), ctx));
            });
        }
        _ => {
            let editable = ctx.editable.is_none_or(|e| e.contains_key(ptr));
            let modified = ctx.baseline.pointer(ptr) != Some(v);
            ui.horizontal(|ui| {
                let color = if modified { theme::MODIFIED } else { ui.visuals().text_color() };
                let label_w = (ctx.value_x - ui.cursor().min.x).max(120.0);
                let job = label_job(ui, &label, raw, "", color);
                ui.allocate_ui_with_layout(egui::vec2(label_w, ui.spacing().interact_size.y), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_width(label_w);
                    ui.add(egui::Label::new(job).truncate()).on_hover_text(format!("key: {key}\npath: {ptr}"));
                });
                if let Some(nv) = widgets::value_editor(ui, ptr, v, editable, 320.0) {
                    ctx.edits.push((ptr.to_string(), nv));
                }
            });
        }
    }
}

fn paged<T: Copy>(ui: &mut Ui, ptr: &str, entries: &[T], mut row: impl FnMut(&mut Ui, T)) {
    if entries.len() <= PAGE {
        for &e in entries {
            row(ui, e);
        }
        return;
    }
    let total = entries.len();
    for (page_idx, chunk) in entries.chunks(PAGE).enumerate() {
        let start = page_idx * PAGE;
        let end = (start + PAGE).min(total) - 1;
        CollapsingHeader::new(format!("{start} \u{2013} {end}"))
            .id_salt(format!("{ptr}#page{start}"))
            .show(ui, |ui| {
                for &e in chunk {
                    row(ui, e);
                }
            });
    }
}
