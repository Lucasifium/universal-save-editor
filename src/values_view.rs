//! The friendly "Values" view: every editable value in one aligned, searchable
//! table, grouped into collapsible sections.

use crate::i18n::{tr, trf};
use crate::json_view::pointer_escape;
use crate::names::{self, Names};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, text::LayoutJob, Align, FontId, Layout, RichText, Sense, TextFormat, Ui};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::Arc;

/// A row's label and raw key, shared by every row with the same key (array indices and
/// field names repeat across thousands of rows).
struct Label {
    name: String,
    raw_key: String,
    /// `"{name}\n{raw_key}"` lower-cased, for search.
    lc: String,
    /// Where the lower-cased name ends in `lc`.
    name_lc_len: usize,
}

/// The "where" text of a row ("Party › Actor 1"), shared by all scalars of one container.
struct Context {
    text: String,
    lc: String,
}

pub struct Row {
    pub ptr: Arc<str>,
    pub section: usize,
    pub editable: bool,
    label: Arc<Label>,
    context: Arc<Context>,
}

impl Row {
    pub fn name(&self) -> &str {
        &self.label.name
    }

    pub fn context(&self) -> &str {
        &self.context.text
    }

    pub fn raw_key(&self) -> &str {
        &self.label.raw_key
    }
}

pub struct Section {
    pub title: String,
    pub advanced: bool,
    lc: String,
}

#[derive(Default)]
pub struct Model {
    pub sections: Vec<Section>,
    pub rows: Vec<Row>,
    pub by_ptr: HashMap<Arc<str>, usize>,
}

#[derive(Clone, Copy)]
pub enum Disp {
    Header(usize, usize),
    Item(usize),
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Object(_) | Value::Array(_))
}

struct Builder<'a> {
    names: &'a Names,
    editable: Option<&'a HashMap<String, usize>>,
    model: Model,
    /// The pointer of the node being visited; children append to it and truncate back.
    ptr: String,
    /// Labels of the containers above the node being visited.
    context: Vec<String>,
    /// The shared context text for each depth of `context`, made when its first row needs it.
    context_text: Vec<Option<Arc<Context>>>,
    /// Labels by raw key, for rows whose label comes from the key alone.
    labels: HashMap<String, Arc<Label>>,
}

fn new_label(name: String, raw_key: &str) -> Arc<Label> {
    // Lower-casing the parts apart gives the same text as lower-casing them joined by a line break.
    let mut lc = name.to_lowercase();
    let name_lc_len = lc.len();
    lc.push('\n');
    lc.push_str(&raw_key.to_lowercase());
    Arc::new(Label { lc, name_lc_len, name, raw_key: raw_key.to_string() })
}

impl Builder<'_> {
    fn section(&mut self, title: String, advanced: bool) -> usize {
        self.model.sections.push(Section { lc: title.to_lowercase(), title, advanced });
        self.model.sections.len() - 1
    }

    fn push_context(&mut self, label: String) {
        self.context.push(label);
        self.context_text.push(None);
    }

    fn pop_context(&mut self) {
        self.context.pop();
        self.context_text.pop();
    }

    /// Visit `v`, whose key is `key` and whose pointer is `self.ptr`.
    fn collect(&mut self, v: &Value, key: &str, section: usize, is_section_root: bool) {
        let hide = self.names.hide_meta;
        if hide && (key == "@c" || key == "@") {
            return;
        }
        // RPG Maker wraps lists in a "_data" field; it adds nothing for the reader.
        let is_section_root = is_section_root || (hide && key == "_data");
        let len = self.ptr.len();
        match v {
            Value::Object(map) => {
                if hide && map.contains_key("@a") && map.keys().all(|k| k == "@a" || k == "@c" || k == "@") {
                    self.ptr.push_str("/@a");
                    self.collect(&map["@a"], key, section, is_section_root);
                    self.ptr.truncate(len);
                    return;
                }
                if !is_section_root {
                    let label = names::node_label(self.names, key, &self.ptr, v);
                    self.push_context(label);
                }
                for (k, c) in map {
                    self.ptr.push('/');
                    push_escaped(&mut self.ptr, k);
                    self.collect(c, k, section, false);
                    self.ptr.truncate(len);
                }
                if !is_section_root {
                    self.pop_context();
                }
            }
            Value::Array(arr) => {
                if !is_section_root {
                    let label = names::node_label(self.names, key, &self.ptr, v);
                    self.push_context(label);
                }
                let mut index = String::new();
                for (i, c) in arr.iter().enumerate() {
                    index.clear();
                    let _ = write!(index, "{i}");
                    self.ptr.push('/');
                    self.ptr.push_str(&index);
                    self.collect(c, &index, section, false);
                    self.ptr.truncate(len);
                }
                if !is_section_root {
                    self.pop_context();
                }
            }
            Value::Null => {}
            _ => {
                let label = match self.names.by_ptr.contains_key(self.ptr.as_str()) {
                    true => new_label(names::node_label(self.names, key, &self.ptr, v), key),
                    // Without a name for this exact pointer, a scalar's label depends on its key alone.
                    false => match self.labels.get(key) {
                        Some(l) => l.clone(),
                        None => {
                            let l = new_label(names::node_label(self.names, key, &self.ptr, v), key);
                            self.labels.insert(key.to_string(), l.clone());
                            l
                        }
                    },
                };
                let depth = self.context.len();
                let context = match &self.context_text[depth] {
                    Some(c) => c.clone(),
                    None => {
                        let text = self.context.join(" \u{203A} ");
                        let c = Arc::new(Context { lc: text.to_lowercase(), text });
                        self.context_text[depth] = Some(c.clone());
                        c
                    }
                };
                let editable = self.editable.is_none_or(|e| e.contains_key(self.ptr.as_str()));
                let ptr: Arc<str> = Arc::from(self.ptr.as_str());
                self.model.by_ptr.insert(ptr.clone(), self.model.rows.len());
                self.model.rows.push(Row { ptr, section, editable, label, context });
            }
        }
    }
}

/// Append `key` to a JSON pointer, escaped as `pointer_escape` does.
fn push_escaped(ptr: &mut String, key: &str) {
    if key.contains(['~', '/']) {
        ptr.push_str(&pointer_escape(key));
    } else {
        ptr.push_str(key);
    }
}

pub fn build(root: &Value, names: &Names, editable: Option<&HashMap<String, usize>>) -> Model {
    let mut b = Builder { names, editable, model: Model::default(), ptr: String::new(), context: Vec::new(), context_text: vec![None], labels: HashMap::new() };
    match root {
        Value::Object(map) => {
            if map.values().any(is_scalar) {
                let general = b.section("General".into(), false);
                for (k, v) in map.iter().filter(|(_, v)| is_scalar(v)) {
                    b.ptr.clear();
                    b.ptr.push('/');
                    push_escaped(&mut b.ptr, k);
                    b.collect(v, k, general, false);
                }
            }
            let mut keys: Vec<&String> = map.iter().filter(|(_, v)| !is_scalar(v)).map(|(k, _)| k).collect();
            keys.sort_by_key(|k| names.priority.iter().position(|p| p == *k).unwrap_or(usize::MAX));
            for k in keys {
                if names.hide_meta && (k == "@c" || k == "@") {
                    continue;
                }
                let ptr = format!("/{}", pointer_escape(k));
                let title = names::node_label(names, k, &ptr, &map[k]);
                let s = b.section(title, names.advanced.iter().any(|a| a == k));
                b.ptr = ptr;
                b.collect(&map[k], k, s, true);
            }
        }
        other => {
            let s = b.section("Values".into(), false);
            b.ptr.clear();
            b.collect(other, "", s, true);
        }
    }
    b.model
}

impl Row {
    /// Does the row's name, context, key or section title contain `needle` (lower-cased)?
    fn text_matches(&self, section: &Section, needle: &str) -> bool {
        if needle.contains('\n') {
            // Matches may span the line breaks between the parts; keep the joined text's answer.
            let (name, key) = self.label.lc.split_at(self.label.name_lc_len);
            return format!("{name}\n{}{key}\n{}", self.context.lc, section.lc).contains(needle);
        }
        self.label.lc.contains(needle) || self.context.lc.contains(needle) || section.lc.contains(needle)
    }
}

/// Rows to display after search / section collapse / advanced filtering.
pub fn filter(model: &Model, root: &Value, needle: &str, advanced: bool, collapsed: &HashSet<usize>) -> Vec<Disp> {
    let needle = needle.trim().to_lowercase();
    let mut out = Vec::new();
    let mut start = 0;
    while start < model.rows.len() {
        let sec = model.rows[start].section;
        let end = model.rows[start..].iter().position(|r| r.section != sec).map_or(model.rows.len(), |p| start + p);
        let section = &model.sections[sec];
        if advanced || !section.advanced {
            let hits: Vec<usize> = (start..end)
                .filter(|&i| {
                    let r = &model.rows[i];
                    (advanced || r.editable)
                        && (needle.is_empty()
                            || r.text_matches(section, &needle)
                            || root.pointer(&r.ptr).is_some_and(|v| widgets::display(v).to_lowercase().contains(&needle)))
                })
                .collect();
            if !hits.is_empty() {
                out.push(Disp::Header(sec, hits.len()));
                if !collapsed.contains(&sec) || !needle.is_empty() {
                    out.extend(hits.into_iter().map(Disp::Item));
                }
            }
        }
        start = end;
    }
    out
}

pub struct ViewCtx<'a> {
    pub root: &'a Value,
    /// The structured document, for fields with a fixed list of choices.
    pub tree: Option<&'a crate::tree::TreeDoc>,
    pub baseline: &'a Value,
    pub show_raw_keys: bool,
    pub has_icons: bool,
    pub edits: Vec<(String, Value)>,
    pub toggle_section: Option<usize>,
    pub scroll_to: Option<usize>,
    /// Bulk edit: the rows ticked so far (`None` when bulk edit is off) and the rows whose
    /// tick was clicked this frame.
    pub ticked: Option<&'a HashSet<usize>>,
    pub toggle_tick: Vec<usize>,
    /// "Show in Tree" was chosen for this pointer.
    pub show_in_tree: Option<String>,
    /// A single copied value is waiting: offer "Paste here" on each value.
    pub can_paste_one: bool,
    /// Asked to copy: what it is and the (pointer, value) pairs.
    pub copy: Option<(String, Vec<(String, Value)>)>,
    /// Asked to paste the copied value into this pointer.
    pub paste_at: Option<String>,
}

/// What bulk edit does to each ticked value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BulkOp {
    Set,
    Add,
    Multiply,
}

impl BulkOp {
    pub const ALL: [BulkOp; 3] = [BulkOp::Set, BulkOp::Add, BulkOp::Multiply];

    pub fn name(self) -> &'static str {
        match self {
            BulkOp::Set => tr("Set to"),
            BulkOp::Add => tr("Add"),
            BulkOp::Multiply => tr("Multiply by"),
        }
    }
}

/// The new value bulk edit gives `cur`, or why it can't (`Ok(None)`: left as it is). Set
/// only changes values of the kind typed: a number sets numbers, true/false sets switches, and
/// other text sets text, so ticking a whole section never turns a name into "99".
pub fn bulk_value(cur: &Value, op: BulkOp, arg: &str) -> Result<Option<Value>, String> {
    let arg = arg.trim();
    let parsed = arg.replace(',', "").parse::<f64>();
    let num = || parsed.clone().map_err(|_| format!("\u{201C}{arg}\u{201D} is not a number"));
    let switch = match arg.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    };
    if op == BulkOp::Set && !matches!(cur, Value::Number(_)) {
        return Ok(match cur {
            Value::Bool(_) => switch.map(Value::Bool),
            Value::String(_) if parsed.is_err() && switch.is_none() => Some(Value::String(arg.to_string())),
            _ => None,
        });
    }
    if op == BulkOp::Set && (switch.is_some() || parsed.is_err()) {
        return Ok(None);
    }
    match (cur, op) {
        (Value::Number(n), _) => {
            let x = num()?;
            let old = n.as_f64().unwrap_or(0.0);
            let new = match op {
                BulkOp::Set => x,
                BulkOp::Add => old + x,
                BulkOp::Multiply => old * x,
            };
            if !new.is_finite() {
                return Err("the result is too big".into());
            }
            let whole = n.is_i64() || n.is_u64();
            Ok(Some(if whole {
                let r = new.round();
                if r < 0.0 && n.is_u64() && !n.is_i64() {
                    Value::from(0u64)
                } else if r >= 0.0 && r <= u64::MAX as f64 {
                    Value::from(r as u64)
                } else if r >= i64::MIN as f64 {
                    Value::from(r as i64)
                } else {
                    return Err("the result is too big".into());
                }
            } else {
                serde_json::Number::from_f64(new).map(Value::Number).ok_or("the result is not a number")?
            }))
        }
        _ => Ok(None),
    }
}

pub const ROW_H: f32 = 34.0;

pub fn show(ui: &mut Ui, model: &Model, disp: &[Disp], collapsed: &HashSet<usize>, ctx: &mut ViewCtx) {
    let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
    if let Some(target) = ctx.scroll_to.take() {
        if let Some(pos) = disp.iter().position(|d| matches!(d, Disp::Item(i) if *i == target)) {
            let spacing = ui.spacing().item_spacing.y;
            area = area.vertical_scroll_offset(((pos as f32) - 3.0).max(0.0) * (ROW_H + spacing));
        }
    }
    area.show_rows(ui, ROW_H, disp.len(), |ui, range| {
        let full_w = ui.available_width();
        let name_w = (full_w * 0.4).clamp(110.0, 460.0);
        for idx in range {
            let row_rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(full_w, ROW_H));
            let name_w = if ctx.ticked.is_some() { (name_w - 28.0).max(80.0) } else { name_w };
            match disp[idx] {
                Disp::Header(sec, count) => {
                    let s = &model.sections[sec];
                    let open = !collapsed.contains(&sec);
                    let (row_rect, resp) = ui.allocate_exact_size(egui::vec2(full_w, ROW_H), Sense::click());
                    // A painted row: tell screen readers it's a heading that opens and closes (Enter or Space work as a click).
                    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::CollapsingHeader, true, open, &s.title));
                    let painter = ui.painter();
                    let fill = if resp.hovered() { ui.visuals().widgets.hovered.weak_bg_fill } else { ui.visuals().faint_bg_color };
                    painter.rect_filled(row_rect.shrink2(egui::vec2(0.0, 2.0)), 6.0, fill);
                    let c = ui.visuals().strong_text_color();
                    if ctx.has_icons {
                        let chevron = if open { theme::icon::DOWN } else { theme::icon::RIGHT };
                        painter.text(row_rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, chevron, FontId::new(11.0, theme::icons_family()), c);
                    } else {
                        // No icon font (Linux, macOS, the browser): a drawn triangle, like the tree's, not a letter "v".
                        let m = row_rect.left_center() + egui::vec2(17.0, 0.0);
                        let pts = if open {
                            vec![m + egui::vec2(-4.5, -2.5), m + egui::vec2(4.5, -2.5), m + egui::vec2(0.0, 3.0)]
                        } else {
                            vec![m + egui::vec2(-2.5, -4.5), m + egui::vec2(3.0, 0.0), m + egui::vec2(-2.5, 4.5)]
                        };
                        painter.add(egui::Shape::convex_polygon(pts, c, egui::Stroke::NONE));
                    }
                    painter.text(row_rect.left_center() + egui::vec2(32.0, 0.0), egui::Align2::LEFT_CENTER, &s.title, FontId::new(16.0, theme::semibold_family()), c);
                    let mut right = format!("{count}");
                    if s.advanced {
                        right = format!("{}  \u{00B7}  {right}", tr("advanced"));
                    }
                    painter.text(row_rect.right_center() - egui::vec2(12.0, 0.0), egui::Align2::RIGHT_CENTER, right, FontId::new(13.0, egui::FontFamily::Proportional), ui.visuals().weak_text_color());
                    if resp.clicked() {
                        ctx.toggle_section = Some(sec);
                    }
                    resp.context_menu(|ui| {
                        if ui.button(crate::i18n::trf("Copy all of {0} for another save", &[&s.title])).clicked() {
                            let values: Vec<(String, Value)> = model
                                .rows
                                .iter()
                                .filter(|r| r.section == sec && r.editable)
                                .filter_map(|r| Some((r.ptr.to_string(), ctx.root.pointer(&r.ptr)?.clone())))
                                .collect();
                            ctx.copy = Some((s.title.clone(), values));
                            ui.close_menu();
                        }
                    });
                }
                Disp::Item(i) => {
                    let r = &model.rows[i];
                    let Some(v) = ctx.root.pointer(&r.ptr) else {
                        ui.allocate_exact_size(egui::vec2(full_w, ROW_H), Sense::hover());
                        continue;
                    };
                    let modified = ctx.baseline.pointer(&r.ptr) != Some(v);
                    let hovered = ui.rect_contains_pointer(row_rect);
                    if hovered {
                        ui.painter().rect_filled(row_rect, 6.0, ui.visuals().faint_bg_color);
                    }
                    if modified {
                        let bar = egui::Rect::from_min_size(row_rect.min + egui::vec2(2.0, 6.0), egui::vec2(3.0, ROW_H - 12.0));
                        ui.painter().rect_filled(bar, 2.0, theme::fg(ui, theme::MODIFIED));
                    }
                    ui.allocate_ui_with_layout(egui::vec2(full_w, ROW_H), Layout::left_to_right(Align::Center), |ui| {
                        ui.set_min_size(egui::vec2(full_w, ROW_H));
                        {
                            ui.add_space(14.0);
                            if let Some(t) = ctx.ticked {
                                let mut on = t.contains(&i);
                                let tip = if r.editable { tr("Tick to include in Bulk edit") } else { tr("Read-only: bulk edit leaves it alone") };
                                if crate::widgets::square(ui, |ui| ui.add_enabled(r.editable, egui::Checkbox::without_text(&mut on))).on_hover_text(tip).clicked() {
                                    ctx.toggle_tick.push(i);
                                }
                            }
                            // Name column.
                            ui.allocate_ui_with_layout(egui::vec2(name_w, ROW_H), Layout::left_to_right(Align::Center), |ui| {
                                ui.set_width(name_w);
                                let font = egui::TextStyle::Body.resolve(ui.style());
                                let mut job = LayoutJob::default();
                                if !r.context().is_empty() {
                                    // The name is what tells rows apart, so where the column is too narrow for
                                    // both it is the "where" text in front that is cut short, from its start.
                                    let room = name_w - text_width(ui, r.name(), &font) - 4.0;
                                    let shown = fit_context(r.context(), room, |t| text_width(ui, t, &font));
                                    job.append(&shown, 0.0, TextFormat::simple(font.clone(), ui.visuals().weak_text_color()));
                                }
                                let name_color = if modified { theme::fg(ui, theme::MODIFIED) } else { ui.visuals().strong_text_color() };
                                job.append(r.name(), 0.0, TextFormat::simple(font.clone(), name_color));
                                if ctx.show_raw_keys && names::raw_differs(r.raw_key(), r.name()) {
                                    let small = FontId::new(font.size * 0.82, font.family.clone());
                                    job.append(r.raw_key(), 8.0, TextFormat::simple(small, theme::fg(ui, theme::RAW_KEY)));
                                }
                                let label = ui.add(egui::Label::new(job).truncate().sense(Sense::click()));
                                label.context_menu(|ui| {
                                    if ui.button(tr("Copy for another save")).on_hover_text(tr("Then paste it into another open save (or another field)")).clicked() {
                                        ctx.copy = Some((r.name().to_string(), vec![(r.ptr.to_string(), v.clone())]));
                                        ui.close_menu();
                                    }
                                    if ctx.can_paste_one && r.editable && ui.button(tr("Paste here")).clicked() {
                                        ctx.paste_at = Some(r.ptr.to_string());
                                        ui.close_menu();
                                    }
                                });
                                label.on_hover_ui(|ui| {
                                    ui.label(RichText::new(r.name()).strong());
                                    if !r.context().is_empty() {
                                        ui.label(trf("in {0}", &[&r.context()]));
                                    }
                                    ui.weak(trf("key: {0}   ({1})", &[&r.raw_key(), &widgets::type_name(v)]));
                                    ui.weak(trf("path: {0}", &[&r.ptr]));
                                    if let Some(note) = ctx.tree.and_then(|t| crate::risks::note(&t.kind_name, &r.ptr)) {
                                        ui.label(trf("Note: {0}", &[&note]));
                                    }
                                });
                            });
                            // Value column.
                            let w = (ui.available_width() - 120.0).clamp(120.0, 420.0);
                            let choices = if r.editable { ctx.tree.and_then(|t| t.choices(&r.ptr)) } else { None };
                            let opts = widgets::EditOpts { tree: ctx.tree, baseline: Some(ctx.baseline), name: Some(r.name()), kind: ctx.tree.map(|t| t.kind_name.as_str()), show_in_tree: true };
                            let edit = match (choices, v) {
                                (Some(list), Value::String(cur)) => widgets::choice_editor(ui, &r.ptr, cur, &list, w),
                                _ => {
                                    let o = widgets::value_editor_ex(ui, &r.ptr, v, r.editable, w, &opts);
                                    if o.go_to_tree {
                                        ctx.show_in_tree = Some(r.ptr.to_string());
                                    }
                                    o.edit
                                }
                            };
                            if let Some(nv) = edit {
                                ctx.edits.push((r.ptr.to_string(), nv));
                            }
                            if modified {
                                let old = ctx.baseline.pointer(&r.ptr).map(widgets::display).unwrap_or_default();
                                let tip = trf("Revert to original: {0}", &[&old]);
                                let btn = widgets::icon_button(ui, widgets::icon(ctx.has_icons, theme::icon::REVERT, tr("Revert")).size(12.0), &tip, &tip);
                                if btn.clicked() {
                                    if let Some(orig) = ctx.baseline.pointer(&r.ptr) {
                                        ctx.edits.push((r.ptr.to_string(), orig.clone()));
                                    }
                                }
                            }
                        }
                    });
                }
            }
        }
    });
}

/// How wide `text` is drawn in `font`.
fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), egui::Color32::WHITE).size().x)
}

/// A row's "where" text with its separator ("Party › Actor 1 › "), shortened from the front
/// ("… › Actor 1 › ", then "… › ") until it is at most `room` wide.
fn fit_context(context: &str, room: f32, width: impl Fn(&str) -> f32) -> String {
    const SEP: &str = " \u{203A} ";
    let full = format!("{context}{SEP}");
    if width(&full) <= room {
        return full;
    }
    let parts: Vec<&str> = context.split(SEP).collect();
    for from in 1..parts.len() {
        let t = format!("\u{2026}{SEP}{}{SEP}", parts[from..].join(SEP));
        if width(&t) <= room {
            return t;
        }
    }
    format!("\u{2026}{SEP}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn narrow_name_column_keeps_the_end_of_the_path() {
        let w = |t: &str| t.chars().count() as f32;
        assert_eq!(fit_context("Members \u{203A} #0 Hero", 40.0, w), "Members \u{203A} #0 Hero \u{203A} ");
        assert_eq!(fit_context("Members \u{203A} #0 Hero", 14.0, w), "\u{2026} \u{203A} #0 Hero \u{203A} ");
        assert_eq!(fit_context("Members \u{203A} #0 Hero", 5.0, w), "\u{2026} \u{203A} ");
        assert_eq!(fit_context("Members", 3.0, w), "\u{2026} \u{203A} ");
    }

    use super::*;
    use serde_json::json;

    /// A large nested document with keys that repeat across many rows.
    fn big_doc() -> Value {
        let actors: Vec<Value> = (0..300)
            .map(|i| json!({"name": format!("Hero {i}"), "level": i % 50, "maxHp": 100 + i, "equips": [i, i + 1, null, {"slot/one": "Sword~1"}]}))
            .collect();
        let mut sw = serde_json::Map::new();
        for i in 0..200 {
            sw.insert(format!("switch_{i}"), json!(i % 3 == 0));
        }
        json!({"gold": 1234, "playTime": 5.5, "actors": actors, "switches": sw, "map": {"mapId": 7, "events": [[1, 2], [3, "ΣΑΣ"]]}})
    }

    #[test]
    fn model_rows_and_search() {
        let doc = big_doc();
        let model = build(&doc, &Names::default(), None);
        assert_eq!(model.rows.len(), 2 + 300 * 6 + 200 + 5);
        assert_eq!(model.sections.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["General", "Actors", "Switches", "Map"]);
        let show = |i: usize| {
            let r = &model.rows[i];
            format!("{}|{}|{}|{}|{}|{}", r.ptr, r.section, r.context(), r.name(), r.raw_key(), r.editable)
        };
        assert_eq!(show(0), "/gold|0||Gold|gold|true");
        assert_eq!(show(1), "/playTime|0||Play Time|playTime|true");
        assert_eq!(show(2), "/actors/0/name|1|#0 Hero 0|Name|name|true");
        assert_eq!(show(5), "/actors/0/equips/0|1|#0 Hero 0 \u{203A} Equips|#0|0|true");
        assert_eq!(show(7), "/actors/0/equips/3/slot~1one|1|#0 Hero 0 \u{203A} Equips \u{203A} #3|Slot/one|slot/one|true");
        assert_eq!(show(2 + 299 * 6 + 2), "/actors/299/maxHp|1|#299 Hero 299|Max HP|maxHp|true");
        assert_eq!(show(2 + 1800), "/switches/switch_0|2||Switch 0|switch_0|true");
        assert_eq!(show(2 + 1800 + 200 + 4), "/map/events/1/1|3|Events \u{203A} #1|#1|1|true");
        for (i, r) in model.rows.iter().enumerate() {
            assert_eq!(model.by_ptr[&r.ptr], i);
            assert_eq!(doc.pointer(&r.ptr).is_some(), true, "{}", r.ptr);
        }
        // Rows with the same key share one label.
        assert!(Arc::ptr_eq(&model.rows[2 + 3].label, &model.rows[2 + 6 + 3].label));

        let items = |needle: &str, advanced: bool| -> Vec<String> {
            filter(&model, &doc, needle, advanced, &HashSet::new())
                .into_iter()
                .map(|d| match d {
                    Disp::Header(s, n) => format!("[{} {n}]", model.sections[s].title),
                    Disp::Item(i) => model.rows[i].ptr.to_string(),
                })
                .collect()
        };
        // Name, context, raw key, section title and value all match.
        assert_eq!(items("hero 17 ", true)[..3], ["[Actors 66]", "/actors/17/name", "/actors/17/level"]);
        assert_eq!(items("MAX hp", true).len(), 301);
        assert_eq!(items("maxhp", true).len(), 301);
        assert_eq!(items("slot/", true).len(), 301);
        assert_eq!(items("sword~1", true).len(), 301);
        assert_eq!(items("switches", true).len(), 201);
        assert_eq!(items("\u{3c3}\u{3b1}\u{3c2}", true), ["[Map 1]", "/map/events/1/1"]);
        assert_eq!(items("events \u{203A} #0", true), ["[Map 2]", "/map/events/0/0", "/map/events/0/1"]);
        assert_eq!(items("1234", true), ["[General 1]", "/gold"]);
        assert!(items("no such text", true).is_empty());
        assert_eq!(items("", true).len(), model.rows.len() + 4);
        // Text spanning the parts matches only in the order name, context, key, section.
        assert!(items("name\nname", true).is_empty());
        assert_eq!(items("hp\n#4 hero 4\nmaxhp\nactors", true), ["[Actors 1]", "/actors/4/maxHp"]);
        assert_eq!(items("name\n#0 hero 0\nname", true), ["[Actors 1]", "/actors/0/name"]);
        assert_eq!(items("switch_5\nswitches", true), ["[Switches 1]", "/switches/switch_5"]);

        // Read-only rows are left out unless advanced rows are shown.
        let editable: HashMap<String, usize> = [("/gold".to_string(), 0)].into_iter().collect();
        let ro = build(&doc, &Names::default(), Some(&editable));
        assert_eq!(ro.rows.iter().filter(|r| r.editable).count(), 1);
        assert_eq!(filter(&ro, &doc, "", false, &HashSet::new()).len(), 2);
        assert_eq!(filter(&ro, &doc, "", true, &HashSet::new()).len(), model.rows.len() + 4);
    }

    #[test]
    fn bulk_values() {
        assert_eq!(bulk_value(&json!(10), BulkOp::Set, "99").unwrap(), Some(json!(99)));
        assert_eq!(bulk_value(&json!(10), BulkOp::Add, "5").unwrap(), Some(json!(15)));
        assert_eq!(bulk_value(&json!(10), BulkOp::Multiply, "1.5").unwrap(), Some(json!(15)));
        assert_eq!(bulk_value(&json!(10), BulkOp::Add, "-25").unwrap(), Some(json!(-15)));
        assert_eq!(bulk_value(&json!(1.5), BulkOp::Multiply, "2").unwrap(), Some(json!(3.0)));
        assert_eq!(bulk_value(&json!(10), BulkOp::Set, "1,000").unwrap(), Some(json!(1000)));
        assert_eq!(bulk_value(&json!(10), BulkOp::Set, "lots").unwrap(), None);
        assert!(bulk_value(&json!(10), BulkOp::Add, "lots").is_err());
        // Set changes only values of the kind typed.
        assert_eq!(bulk_value(&json!(true), BulkOp::Set, "off").unwrap(), Some(json!(false)));
        assert_eq!(bulk_value(&json!(true), BulkOp::Set, "0").unwrap(), None);
        assert_eq!(bulk_value(&json!(10), BulkOp::Set, "true").unwrap(), None);
        assert_eq!(bulk_value(&json!("a"), BulkOp::Set, "99").unwrap(), None);
        assert_eq!(bulk_value(&json!("a"), BulkOp::Set, "b").unwrap(), Some(json!("b")));
        assert_eq!(bulk_value(&json!(true), BulkOp::Add, "1").unwrap(), None);
        assert_eq!(bulk_value(&json!("a"), BulkOp::Multiply, "2").unwrap(), None);
    }
}
