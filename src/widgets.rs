//! Reusable editing widgets. Editors never mutate data directly: they return the
//! new value so the app can record it for undo/redo and change tracking.

use crate::i18n::tr;
use crate::theme;
use eframe::egui::{self, Color32, Response, RichText, Sense, TextEdit, Ui, WidgetText};
use serde_json::{Number, Value};

/// iOS-style on/off switch.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let size = egui::vec2(2.0, 1.0) * (ui.spacing().interact_size.y - 4.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool(resp.id, *on);
        let radius = 0.5 * rect.height();
        let off_fill = ui.visuals().widgets.inactive.bg_fill;
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        let fill = Color32::from_rgb(
            mix(off_fill.r(), theme::ACCENT.r()),
            mix(off_fill.g(), theme::ACCENT.g()),
            mix(off_fill.b(), theme::ACCENT.b()),
        );
        ui.painter().rect_filled(rect, radius, fill);
        let cx = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(egui::pos2(cx, rect.center().y), radius - 3.0, Color32::WHITE);
    }
    resp
}

/// A toggle switch with a clickable label. Returns true when flipped.
pub fn labeled_toggle(ui: &mut Ui, on: &mut bool, text: &str, tip: &str) -> bool {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let mut changed = toggle(ui, on).on_hover_text(tip).changed();
        if ui.add(egui::Label::new(text).sense(Sense::click())).on_hover_text(tip).clicked() {
            *on = !*on;
            changed = true;
        }
        changed
    })
    .inner
}

/// Icon (if the icon font is present) followed by text.
pub fn icon_text(ui: &Ui, has_icons: bool, icon: &str, text: &str) -> WidgetText {
    icon_text_colored(ui, has_icons, icon, text, ui.visuals().text_color())
}

pub fn icon_text_colored(ui: &Ui, has_icons: bool, icon: &str, text: &str, color: Color32) -> WidgetText {
    if !has_icons {
        return RichText::new(text).color(color).into();
    }
    let font = egui::TextStyle::Button.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    job.append(icon, 0.0, egui::TextFormat::simple(egui::FontId::new(font.size - 1.0, theme::icons_family()), color));
    if !text.is_empty() {
        job.append(text, 6.0, egui::TextFormat::simple(font, color));
    }
    job.into()
}

pub fn icon(has_icons: bool, glyph: &str, fallback: &str) -> RichText {
    if has_icons {
        RichText::new(glyph).family(theme::icons_family())
    } else {
        RichText::new(fallback)
    }
}

fn escape_breaks(s: &str) -> String {
    s.replace('\n', "\u{21A9}").replace('\r', "\u{00B6}")
}

fn unescape_breaks(s: &str) -> String {
    s.replace('\u{21A9}', "\n").replace('\u{00B6}', "\r")
}

/// Parse user input for a number, keeping integers as integers.
pub fn parse_number(text: &str, integer: bool) -> Option<Value> {
    let t: String = text.chars().filter(|c| !matches!(c, ' ' | ',' | '_')).collect();
    if t.is_empty() {
        return None;
    }
    if integer {
        if let Ok(i) = t.parse::<i64>() {
            return Some(Value::from(i));
        }
        return t.parse::<u64>().ok().map(Value::from);
    }
    let f: f64 = t.parse().ok()?;
    Number::from_f64(f).map(Value::Number)
}

pub fn display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => (if *b { "On" } else { "Off" }).into(),
        Value::Null => "empty".into(),
        Value::Array(a) => format!("list of {}", a.len()),
        Value::Object(m) => format!("group of {}", m.len()),
    }
}

pub fn type_name(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "text",
        Value::Number(n) if n.as_i64().is_some() || n.as_u64().is_some() => "whole number",
        Value::Number(_) => "decimal",
        Value::Bool(_) => "on/off",
        Value::Null => "empty",
        _ => "",
    }
}

/// Editor for a scalar value. Returns `Some(new)` when the user changed it.
/// A drop-down for fields with a fixed list of values (Pokémon, items, moves…),
/// with a filter box so long lists stay usable.
pub fn choice_editor(ui: &mut Ui, id_src: &str, current: &str, list: &[String], width: f32) -> Option<Value> {
    let mut out = None;
    let id = ui.make_persistent_id(("choice", id_src));
    let fid = id.with("filter");
    egui::ComboBox::from_id_salt(id).selected_text(current).width(width.min(260.0)).height(340.0).show_ui(ui, |ui| {
        let mut filter = ui.data(|d| d.get_temp::<String>(fid)).unwrap_or_default();
        let r = ui.add(TextEdit::singleline(&mut filter).hint_text(tr("Type to filter…")).desired_width(f32::INFINITY));
        if ui.memory(|m| m.focused().is_none()) {
            r.request_focus();
        }
        let f = filter.trim().to_lowercase();
        for name in list.iter().filter(|n| f.is_empty() || n.to_lowercase().contains(&f)) {
            if ui.selectable_label(name == current, name).clicked() && name != current {
                out = Some(Value::String(name.clone()));
            }
        }
        if out.is_some() {
            filter.clear();
        }
        ui.data_mut(|d| d.insert_temp(fid, filter));
    });
    out
}

/// A box for typing an id with a filterable drop-down of known ones beside it. The typed text
/// is always accepted, so ids the list doesn't know can still be entered.
pub fn id_picker(ui: &mut Ui, id_src: &str, text: &mut String, known: &[String], hint: &str, width: f32) {
    ui.add(TextEdit::singleline(text).hint_text(hint).desired_width(width));
    if known.is_empty() {
        return;
    }
    let shown = text.trim().to_lowercase();
    let list: Vec<&String> = known.iter().filter(|k| shown.is_empty() || k.to_lowercase().contains(&shown)).collect();
    egui::ComboBox::from_id_salt(("id_picker", id_src)).selected_text("From this save").width(140.0).height(300.0).show_ui(ui, |ui| {
        if list.is_empty() {
            ui.weak(tr("Nothing in this save matches"));
        }
        for k in list {
            if ui.selectable_label(false, k).clicked() {
                *text = k.clone();
            }
        }
    });
}

thread_local! {
    static COMPACT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with number boxes drawn without their −/+ buttons (for tight tables).
pub fn compact<R>(f: impl FnOnce() -> R) -> R {
    COMPACT.with(|c| c.set(true));
    let r = f();
    COMPACT.with(|c| c.set(false));
    r
}

/// What the editors may offer beyond typing: the format's accepted range, the opened
/// value (for Revert) and the name to copy. All optional.
#[derive(Default, Clone, Copy)]
pub struct EditOpts<'a> {
    pub tree: Option<&'a crate::tree::TreeDoc>,
    pub baseline: Option<&'a Value>,
    pub name: Option<&'a str>,
    /// The kind of save ("Elden Ring character"), to pick risk notes.
    pub kind: Option<&'a str>,
    /// Offer "Show in Tree" in the right-click menu.
    pub show_in_tree: bool,
}

/// An editor's result: the new value if it changed, and whether "Show in Tree" was chosen.
#[derive(Default)]
pub struct EditOut {
    pub edit: Option<Value>,
    pub go_to_tree: bool,
}

fn copy_form(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The right-click menu of a value: copy, range shortcuts, revert, show in Tree.
fn value_menu(resp: &Response, ptr: &str, v: &Value, editable: bool, opts: &EditOpts, out: &mut EditOut) {
    resp.context_menu(|ui| {
        if ui.button(tr("Copy value")).clicked() {
            ui.ctx().copy_text(copy_form(v));
            ui.close_menu();
        }
        if let Some(n) = opts.name {
            if ui.button(tr("Copy name")).clicked() {
                ui.ctx().copy_text(n.to_string());
                ui.close_menu();
            }
        }
        if !ptr.is_empty() && ui.button(tr("Copy path")).clicked() {
            ui.ctx().copy_text(ptr.to_string());
            ui.close_menu();
        }
        if editable {
            if let Some((lo, hi)) = opts.tree.and_then(|t| t.range(ptr)) {
                ui.separator();
                if &hi != v && ui.button(format!("Set to maximum ({})", display(&hi))).on_hover_text(tr("The largest value this field accepts")).clicked() {
                    out.edit = Some(hi);
                    ui.close_menu();
                }
                if &lo != v && ui.button(format!("Set to minimum ({})", display(&lo))).on_hover_text(tr("The smallest value this field accepts")).clicked() {
                    out.edit = Some(lo);
                    ui.close_menu();
                }
            }
            if let Some(orig) = opts.baseline.and_then(|b| b.pointer(ptr)).filter(|o| *o != v) {
                ui.separator();
                if ui.button(format!("Revert to original ({})", display(orig))).clicked() {
                    out.edit = Some(orig.clone());
                    ui.close_menu();
                }
            }
        }
        if opts.show_in_tree && !ptr.is_empty() {
            ui.separator();
            if ui.button(tr("Show in Tree")).clicked() {
                out.go_to_tree = true;
                ui.close_menu();
            }
        }
    });
}

/// Hover text for a value: the range the save file accepts (numbers) and a risk note.
fn hover_extra(opts: &EditOpts, ptr: &str, v: &Value) -> Option<String> {
    let mut lines: Vec<String> = vec![];
    if matches!(v, Value::Number(_)) {
        if let Some((lo, hi)) = opts.tree.and_then(|t| t.range(ptr)) {
            lines.push(format!("The save file accepts {} to {} (the game may expect less). Right-click for Max.", display(&lo), display(&hi)));
        }
    }
    if let Some(note) = opts.kind.and_then(|k| crate::risks::note(k, ptr)) {
        lines.push(format!("Note: {note}"));
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// `value_editor` with the right-click menu and range hint (`id_src` is the value's pointer).
pub fn value_editor_ex(ui: &mut Ui, id_src: &str, v: &Value, editable: bool, width: f32, opts: &EditOpts) -> EditOut {
    let mut out = EditOut::default();
    let (edit, resp) = editor_body(ui, id_src, v, editable, width, opts);
    out.edit = edit;
    if let Some(r) = resp {
        value_menu(&r, id_src, v, editable, opts, &mut out);
    }
    out
}

fn editor_body(ui: &mut Ui, id_src: &str, v: &Value, editable: bool, width: f32, opts: &EditOpts) -> (Option<Value>, Option<Response>) {
    if !editable {
        let why = opts.tree.map(|t| t.read_only_reason(id_src)).filter(|w| !w.is_empty()).unwrap_or_else(|| "Read-only: this value can't be edited safely".into());
        let r = ui.label(RichText::new(display(v)).weak().italics()).on_hover_text(why);
        return (None, Some(r));
    }
    match v {
        Value::Bool(b) => {
            let mut on = *b;
            let mut r = toggle(ui, &mut on);
            if let Some(t) = r.hovered().then(|| hover_extra(opts, id_src, v)).flatten() {
                r = r.on_hover_text(t);
            }
            let changed = r.changed();
            ui.label(if on { "On" } else { "Off" });
            (changed.then_some(Value::Bool(on)), Some(r))
        }
        Value::String(s) => {
            // Line breaks are shown as visible symbols so a one-line box can't
            // mangle multi-line text (e.g. dialogue); they're restored on edit.
            let mut text = escape_breaks(s);
            let id = ui.make_persistent_id(("str", id_src));
            let resp = ui.add_sized([width + 8.0, ui.spacing().interact_size.y], TextEdit::singleline(&mut text).id(id));
            let resp = if s.contains(['\n', '\r']) { resp.on_hover_text(tr("\u{21A9} marks a line break (\u{00B6} a carriage return)")) } else { resp };
            let resp = match resp.hovered().then(|| hover_extra(opts, id_src, v)).flatten() {
                Some(t) => resp.on_hover_text(t),
                None => resp,
            };
            (resp.changed().then(|| Value::String(unescape_breaks(&text))), Some(resp))
        }
        Value::Number(n) => {
            let integer = n.as_i64().is_some() || n.as_u64().is_some();
            let id = ui.make_persistent_id(("num", id_src));
            let focused = ui.memory(|m| m.has_focus(id));
            // The typing buffer only lives while the box has focus; otherwise it could
            // resurface stale text after the value changed via −/+, undo or revert.
            if !focused {
                ui.data_mut(|d| d.remove::<String>(id));
            }
            let mut buf = if focused {
                ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| n.to_string())
            } else {
                n.to_string()
            };
            let valid = parse_number(&buf, integer).is_some();
            // Wide enough for what it holds: a fixed box cut "3270" down to "327".
            let text_w = ui.fonts(|f| f.layout_no_wrap(buf.clone(), egui::TextStyle::Body.resolve(ui.style()), Color32::WHITE).size().x);
            let w = width.min(160.0).max((text_w + 20.0).min(260.0));
            let mut edit = TextEdit::singleline(&mut buf).id(id).desired_width(w);
            if !valid {
                edit = edit.text_color(theme::ERR);
            }
            // add_sized: a bare desired_width is cut down to what the cell has left (in a grid, a few pixels).
            let mut resp = ui.add_sized([w + 8.0, ui.spacing().interact_size.y], edit);
            let mut out = None;
            if resp.changed() {
                ui.data_mut(|d| d.insert_temp(id, buf.clone()));
                if let Some(nv) = parse_number(&buf, integer) {
                    if nv != *v {
                        out = Some(nv);
                    }
                }
            }
            if !valid {
                resp = resp.on_hover_text(if integer { "Enter a whole number, e.g. 1500" } else { "Enter a number, e.g. 2.5" });
            } else if resp.hovered() {
                if let Some(hint) = hover_extra(opts, id_src, v) {
                    resp = resp.on_hover_text(hint);
                }
            }
            // On a phone-width screen the −/+ would be pushed off its edge: type the number instead.
            if integer && !COMPACT.with(|c| c.get()) && ui.ctx().screen_rect().width() >= 520.0 {
                // Big values (experience, gold) are typed, not stepped.
                if let Some(i) = n.as_i64().filter(|i| i.unsigned_abs() <= 5000) {
                    let step = ui.input(|inp| if inp.modifiers.ctrl || inp.modifiers.command { 100 } else if inp.modifiers.shift { 10 } else { 1 });
                    if ui.small_button("\u{2212}").on_hover_text(tr("Decrease by 1 (Shift: 10, Ctrl: 100)")).clicked() {
                        out = Some(Value::from(i.saturating_sub(step)));
                    }
                    if ui.small_button("+").on_hover_text(tr("Increase by 1 (Shift: 10, Ctrl: 100)")).clicked() {
                        out = Some(Value::from(i.saturating_add(step)));
                    }
                }
            }
            (out, Some(resp))
        }
        Value::Null => {
            let r = ui.label(RichText::new(tr("empty")).weak().italics());
            (None, Some(r))
        }
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_breaks_round_trip() {
        for s in ["one line", "two\nlines", "windows\r\nbreaks", "\n"] {
            let shown = escape_breaks(s);
            assert!(!shown.contains(['\n', '\r']), "one-line box must not see raw breaks");
            assert_eq!(unescape_breaks(&shown), s);
        }
    }

    #[test]
    fn numbers_parse_strictly() {
        assert_eq!(parse_number("2,500", true), Some(Value::from(2500)));
        assert_eq!(parse_number("-7", true), Some(Value::from(-7)));
        assert_eq!(parse_number("1.5", true), None, "whole-number fields reject decimals");
        assert_eq!(parse_number("", true), None);
        assert_eq!(parse_number("abc", false), None);
        assert_eq!(parse_number("inf", false), None, "JSON can't store infinity");
        assert!(parse_number("2.25", false).is_some());
    }
}

/// A scroll area that keeps the mouse wheel to itself while the pointer is over it and it has
/// something to scroll: at its top or bottom the wheel does nothing, instead of moving the page
/// behind it (egui passes the rest of the wheel on to the surrounding area).
pub trait ScrollLock {
    fn show_locked<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R>;
    fn show_rows_locked<R>(self, ui: &mut Ui, row_height: f32, rows: usize, add: impl FnOnce(&mut Ui, std::ops::Range<usize>) -> R) -> egui::scroll_area::ScrollAreaOutput<R>;
}

fn lock_wheel(ui: &Ui, content: egui::Vec2, inner: egui::Rect) {
    let (ov_y, ov_x) = (content.y > inner.height() + 0.5, content.x > inner.width() + 0.5);
    if (ov_y || ov_x) && ui.rect_contains_pointer(inner) {
        ui.ctx().input_mut(|i| {
            if ov_y {
                i.smooth_scroll_delta[1] = 0.0;
            }
            if ov_x {
                i.smooth_scroll_delta[0] = 0.0;
            }
        });
    }
}

impl ScrollLock for egui::ScrollArea {
    fn show_locked<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
        let out = self.show(ui, add);
        lock_wheel(ui, out.content_size, out.inner_rect);
        out
    }
    fn show_rows_locked<R>(self, ui: &mut Ui, row_height: f32, rows: usize, add: impl FnOnce(&mut Ui, std::ops::Range<usize>) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
        let out = self.show_rows(ui, row_height, rows, add);
        lock_wheel(ui, out.content_size, out.inner_rect);
        out
    }
}

/// Column headings of a table that scrolls up and down: draws them as the grid's first row and,
/// once the table has scrolled them out of view, [`sticky_headers`] repeats them at the top.
pub fn header_row<S: AsRef<str>>(ui: &mut Ui, key: &str, heads: impl IntoIterator<Item = S>) {
    let mut seen: Vec<(egui::Rect, String)> = vec![];
    for h in heads {
        let h = h.as_ref();
        let r = ui.strong(h);
        seen.push((r.rect, h.to_string()));
    }
    ui.end_row();
    ui.data_mut(|d| d.insert_temp(egui::Id::new(("sticky_head", key)), seen));
}

/// Paints the headings recorded by [`header_row`] over the top of the scroll area they belong to
/// (call it right after the area, with the area's `inner_rect`).
pub fn sticky_headers(ui: &mut Ui, key: &str, area: egui::Rect) {
    let Some(seen) = ui.data(|d| d.get_temp::<Vec<(egui::Rect, String)>>(egui::Id::new(("sticky_head", key)))) else { return };
    let Some(first) = seen.first() else { return };
    // Still in view: the real row is showing.
    if first.0.top() >= area.top() - 0.5 || first.0.top() > area.bottom() {
        return;
    }
    let band = egui::Rect::from_min_max(area.left_top(), egui::pos2(area.right(), area.top() + first.0.height() + 6.0));
    let painter = ui.painter_at(band);
    painter.rect_filled(band, 0.0, ui.visuals().faint_bg_color);
    painter.line_segment([band.left_bottom(), band.right_bottom()], egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color));
    let color = ui.visuals().strong_text_color();
    for (r, text) in &seen {
        painter.text(egui::pos2(r.left(), band.top() + 3.0), egui::Align2::LEFT_TOP, text, egui::TextStyle::Body.resolve(ui.style()), color);
    }
}
