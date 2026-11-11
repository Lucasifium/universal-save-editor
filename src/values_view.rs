//! The friendly "Values" view: every editable value in one aligned, searchable
//! table, grouped into collapsible sections.

use crate::json_view::pointer_escape;
use crate::names::{self, Names};
use crate::theme;
use crate::widgets;
use eframe::egui::{self, text::LayoutJob, Align, Color32, FontId, Layout, RichText, Sense, TextFormat, Ui};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub struct Row {
    pub ptr: String,
    pub section: usize,
    pub name: String,
    pub context: String,
    pub raw_key: String,
    pub editable: bool,
    haystack: String,
}

pub struct Section {
    pub title: String,
    pub advanced: bool,
}

#[derive(Default)]
pub struct Model {
    pub sections: Vec<Section>,
    pub rows: Vec<Row>,
    pub by_ptr: HashMap<String, usize>,
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
}

impl Builder<'_> {
    fn section(&mut self, title: String, advanced: bool) -> usize {
        self.model.sections.push(Section { title, advanced });
        self.model.sections.len() - 1
    }

    fn collect(&mut self, v: &Value, key: &str, ptr: &str, context: &mut Vec<String>, section: usize, is_section_root: bool) {
        let hide = self.names.hide_meta;
        if hide && (key == "@c" || key == "@") {
            return;
        }
        // RPG Maker wraps lists in a "_data" field; it adds nothing for the reader.
        let is_section_root = is_section_root || (hide && key == "_data");
        match v {
            Value::Object(map) => {
                if hide && map.contains_key("@a") && map.keys().all(|k| k == "@a" || k == "@c" || k == "@") {
                    return self.collect(&map["@a"], key, &format!("{ptr}/@a"), context, section, is_section_root);
                }
                if !is_section_root {
                    context.push(names::node_label(self.names, key, ptr, v));
                }
                for (k, c) in map {
                    self.collect(c, k, &format!("{ptr}/{}", pointer_escape(k)), context, section, false);
                }
                if !is_section_root {
                    context.pop();
                }
            }
            Value::Array(arr) => {
                if !is_section_root {
                    context.push(names::node_label(self.names, key, ptr, v));
                }
                for (i, c) in arr.iter().enumerate() {
                    self.collect(c, &i.to_string(), &format!("{ptr}/{i}"), context, section, false);
                }
                if !is_section_root {
                    context.pop();
                }
            }
            Value::Null => {}
            _ => {
                let name = names::node_label(self.names, key, ptr, v);
                let ctx = context.join(" \u{203A} ");
                let section_title = &self.model.sections[section].title;
                let haystack = format!("{name}\n{ctx}\n{key}\n{section_title}").to_lowercase();
                let editable = self.editable.is_none_or(|e| e.contains_key(ptr));
                self.model.by_ptr.insert(ptr.to_string(), self.model.rows.len());
                self.model.rows.push(Row { ptr: ptr.to_string(), section, name, context: ctx, raw_key: key.to_string(), editable, haystack });
            }
        }
    }
}

pub fn build(root: &Value, names: &Names, editable: Option<&HashMap<String, usize>>) -> Model {
    let mut b = Builder { names, editable, model: Model::default() };
    let mut ctx = Vec::new();
    match root {
        Value::Object(map) => {
            if map.values().any(is_scalar) {
                let general = b.section("General".into(), false);
                for (k, v) in map.iter().filter(|(_, v)| is_scalar(v)) {
                    b.collect(v, k, &format!("/{}", pointer_escape(k)), &mut ctx, general, false);
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
                b.collect(&map[k], k, &ptr, &mut ctx, s, true);
            }
        }
        other => {
            let s = b.section("Values".into(), false);
            b.collect(other, "", "", &mut ctx, s, true);
        }
    }
    b.model
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
                            || r.haystack.contains(&needle)
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
            match disp[idx] {
                Disp::Header(sec, count) => {
                    let s = &model.sections[sec];
                    let open = !collapsed.contains(&sec);
                    let (row_rect, resp) = ui.allocate_exact_size(egui::vec2(full_w, ROW_H), Sense::click());
                    let painter = ui.painter();
                    let fill = if resp.hovered() { ui.visuals().widgets.hovered.weak_bg_fill } else { ui.visuals().faint_bg_color };
                    painter.rect_filled(row_rect.shrink2(egui::vec2(0.0, 2.0)), 6.0, fill);
                    let chevron = if ctx.has_icons { if open { theme::icon::DOWN } else { theme::icon::RIGHT } } else if open { "v" } else { ">" };
                    let fam = if ctx.has_icons { theme::icons_family() } else { egui::FontFamily::Proportional };
                    let c = ui.visuals().strong_text_color();
                    painter.text(row_rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, chevron, FontId::new(11.0, fam), c);
                    painter.text(row_rect.left_center() + egui::vec2(32.0, 0.0), egui::Align2::LEFT_CENTER, &s.title, FontId::new(16.0, theme::semibold_family()), c);
                    let mut right = format!("{count}");
                    if s.advanced {
                        right = format!("advanced  \u{00B7}  {right}");
                    }
                    painter.text(row_rect.right_center() - egui::vec2(12.0, 0.0), egui::Align2::RIGHT_CENTER, right, FontId::new(13.0, egui::FontFamily::Proportional), ui.visuals().weak_text_color());
                    if resp.clicked() {
                        ctx.toggle_section = Some(sec);
                    }
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
                        ui.painter().rect_filled(bar, 2.0, theme::MODIFIED);
                    }
                    ui.allocate_ui_with_layout(egui::vec2(full_w, ROW_H), Layout::left_to_right(Align::Center), |ui| {
                        ui.set_min_size(egui::vec2(full_w, ROW_H));
                        {
                            ui.add_space(14.0);
                            // Name column.
                            ui.allocate_ui_with_layout(egui::vec2(name_w, ROW_H), Layout::left_to_right(Align::Center), |ui| {
                                ui.set_width(name_w);
                                let font = egui::TextStyle::Body.resolve(ui.style());
                                let mut job = LayoutJob::default();
                                if !r.context.is_empty() {
                                    job.append(&format!("{} \u{203A} ", r.context), 0.0, TextFormat::simple(font.clone(), ui.visuals().weak_text_color()));
                                }
                                let name_color = if modified { theme::MODIFIED } else { ui.visuals().strong_text_color() };
                                job.append(&r.name, 0.0, TextFormat::simple(font.clone(), name_color));
                                if ctx.show_raw_keys && names::raw_differs(&r.raw_key, &r.name) {
                                    let small = FontId::new(font.size * 0.82, font.family.clone());
                                    job.append(&r.raw_key, 8.0, TextFormat::simple(small, Color32::from_rgb(120, 150, 185)));
                                }
                                ui.add(egui::Label::new(job).truncate()).on_hover_ui(|ui| {
                                    ui.label(RichText::new(&r.name).strong());
                                    if !r.context.is_empty() {
                                        ui.label(format!("in {}", r.context));
                                    }
                                    ui.weak(format!("key: {}   ({})", r.raw_key, widgets::type_name(v)));
                                    ui.weak(format!("path: {}", r.ptr));
                                });
                            });
                            // Value column.
                            let w = (ui.available_width() - 120.0).clamp(120.0, 420.0);
                            let choices = if r.editable { ctx.tree.and_then(|t| t.choices(&r.ptr)) } else { None };
                            let edit = match (choices, v) {
                                (Some(list), Value::String(cur)) => widgets::choice_editor(ui, &r.ptr, cur, &list, w),
                                _ => widgets::value_editor(ui, &r.ptr, v, r.editable, w),
                            };
                            if let Some(nv) = edit {
                                ctx.edits.push((r.ptr.clone(), nv));
                            }
                            if modified {
                                let old = ctx.baseline.pointer(&r.ptr).map(widgets::display).unwrap_or_default();
                                let btn = ui.add(egui::Button::new(widgets::icon(ctx.has_icons, theme::icon::REVERT, "undo").size(12.0)).frame(false));
                                if btn.on_hover_text(format!("Revert to original: {old}")).clicked() {
                                    if let Some(orig) = ctx.baseline.pointer(&r.ptr) {
                                        ctx.edits.push((r.ptr.clone(), orig.clone()));
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
