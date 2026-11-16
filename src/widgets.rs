//! Reusable editing widgets. Editors never mutate data directly: they return the
//! new value so the app can record it for undo/redo and change tracking.

use crate::i18n::{tr, trf};
use crate::theme;
use eframe::egui::{self, Color32, Response, RichText, Sense, TextEdit, Ui, WidgetText};
use serde_json::{Number, Value};

/// Covers the screen under a dialog or while a file is busy: takes clicks so they don't reach
/// what's behind, without being a stop for Tab.
pub const BLOCK_CLICKS: Sense = Sense { click: true, drag: false, focusable: false };

/// Draws what `add` adds with nearly square corners. A tick box drawn with the theme's round
/// button corners is a circle, the same shape as a radio button, so the two could not be told
/// apart; tick boxes are drawn through this (or [`checkbox`]).
pub fn square<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let old = ui.visuals().widgets.clone();
    let w = &mut ui.visuals_mut().widgets;
    for v in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        v.rounding = egui::Rounding::same(3.0);
    }
    let r = add(ui);
    ui.visuals_mut().widgets = old;
    r
}

/// A tick box with a label (see [`square`]).
pub fn checkbox(ui: &mut Ui, on: &mut bool, text: impl Into<WidgetText>) -> Response {
    square(ui, |ui| ui.checkbox(on, text))
}

/// Gives a control the name a screen reader says for it. A control drawn with only an icon glyph
/// (a private-use character of the icon font, which reads as nothing) or a symbol such as × or +

/// needs one; so does a button whose text starts with an icon glyph.
pub fn named(resp: Response, typ: egui::WidgetType, name: &str) -> Response {
    let enabled = resp.enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(typ, enabled, name));
    resp
}

/// A small frameless button that shows only an icon (or `fallback` text without the icon font):
/// `name` is what a screen reader says for it, and `tip` shows on hover.
pub fn icon_button(ui: &mut Ui, glyph: RichText, name: &str, tip: &str) -> Response {
    let btn = egui::Button::new(glyph).frame(false).min_size(egui::vec2(MIN_TARGET, MIN_TARGET));
    named(ui.add(btn), egui::WidgetType::Button, name).on_hover_text(tip)
}

/// The smallest a pointer target should be (WCAG 2.5.8).
pub const MIN_TARGET: f32 = 24.0;

/// A small button showing a symbol (× + −): at least [`MIN_TARGET`] square, named for screen
/// readers (the symbol alone reads as "multiplication sign" and the like), with `tip` on hover.
pub fn symbol_button(ui: &mut Ui, symbol: &str, name: &str, tip: &str) -> Response {
    let btn = egui::Button::new(symbol).small().min_size(egui::vec2(MIN_TARGET, MIN_TARGET));
    named(ui.add(btn), egui::WidgetType::Button, name).on_hover_text(tip)
}

/// iOS-style on/off switch, with the name a screen reader says for it.
pub fn toggle_named(ui: &mut Ui, on: &mut bool, name: &str) -> Response {
    let size = egui::vec2(2.0, 1.0) * (ui.spacing().interact_size.y - 2.0).max(MIN_TARGET);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let enabled = resp.enabled();
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, *on, name));
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool(resp.id, *on);
        let radius = 0.5 * rect.height();
        // Off: the button grey, with the edge and grey knob below so it shows in both themes.
        let off_fill = ui.visuals().widgets.inactive.bg_fill;
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        let fill = Color32::from_rgb(
            mix(off_fill.r(), theme::ACCENT.r()),
            mix(off_fill.g(), theme::ACCENT.g()),
            mix(off_fill.b(), theme::ACCENT.b()),
        );
        ui.painter().rect_filled(rect, radius, fill);
        // Off, the track gets an edge and a grey knob so the switch stands out from the panel.
        let edge = theme::edge_color(ui.visuals().dark_mode);
        if t < 1.0 {
            ui.painter().rect_stroke(rect, radius, egui::Stroke::new(1.0_f32, edge.gamma_multiply(1.0 - t)));
        }
        let knob = Color32::from_rgb(mix(edge.r(), 255), mix(edge.g(), 255), mix(edge.b(), 255));
        let cx = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(egui::pos2(cx, rect.center().y), radius - 3.0, knob);
    }
    resp
}

/// A toggle switch with a clickable label. Returns true when flipped.
pub fn labeled_toggle(ui: &mut Ui, on: &mut bool, text: &str, tip: &str) -> bool {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let mut changed = toggle_named(ui, on, text).on_hover_text(tip).changed();
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

/// "Label: text" as one wrapping paragraph. Two labels in `horizontal_wrapped` put the
/// text's second line a row's height below its first, further apart than the rest.
pub fn label_text(ui: &mut Ui, label: &str, text: &str, weak: bool) -> Response {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let strong = ui.visuals().strong_text_color();
    let rest = if weak { ui.visuals().weak_text_color() } else { ui.visuals().text_color() };
    let mut job = egui::text::LayoutJob::default();
    job.append(label, 0.0, egui::TextFormat::simple(font.clone(), strong));
    job.append(text, 6.0, egui::TextFormat::simple(font, rest));
    job.wrap.max_width = ui.available_width();
    ui.add(egui::Label::new(job).wrap())
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
        Value::Bool(b) => (if *b { tr("On") } else { tr("Off") }).into(),
        Value::Null => tr("empty").into(),
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
        let shown: Vec<&String> = list.iter().filter(|n| f.is_empty() || n.to_lowercase().contains(&f)).collect();
        long_list(ui, id.with("rows"), shown.len(), |ui, i| {
            let name = shown[i];
            if ui.selectable_label(name == current, name).clicked() && name != current {
                out = Some(Value::String(name.clone()));
            }
        });
        if out.is_some() {
            filter.clear();
        }
        ui.data_mut(|d| d.insert_temp(fid, filter));
    });
    out
}

/// Past this many rows a drop-down list only lays out the rows in view.
const LONG_LIST: usize = 200;

/// `n` same-height selectable rows. Short lists are drawn whole; long ones in a scroll area that
/// only lays out the visible rows, at the width the drop-down already has.
fn long_list(ui: &mut Ui, id: egui::Id, n: usize, mut row: impl FnMut(&mut Ui, usize)) {
    if n <= LONG_LIST {
        (0..n).for_each(|i| row(ui, i));
        return;
    }
    let w = ui.available_width();
    let row_h = ui.spacing().interact_size.y;
    egui::ScrollArea::vertical().id_salt(id).max_height(300.0).auto_shrink([false, true]).show_rows_locked(ui, row_h, n, |ui, range| {
        ui.set_min_width(w);
        for i in range {
            row(ui, i);
        }
    });
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
    egui::ComboBox::from_id_salt(("id_picker", id_src)).selected_text(tr("From this save")).width(140.0).height(300.0).show_ui(ui, |ui| {
        if list.is_empty() {
            ui.weak(tr("Nothing in this save matches"));
        }
        long_list(ui, egui::Id::new(("id_picker_rows", id_src)), list.len(), |ui, i| {
            if ui.selectable_label(false, list[i]).clicked() {
                *text = list[i].clone();
            }
        });
    });
}

thread_local! {
    static COMPACT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

thread_local! {
    static STEPPERS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with −/+ buttons kept even on a phone-width screen, for boxes that have the room
/// (a header tile, an attribute line).
pub fn with_steppers<R>(f: impl FnOnce() -> R) -> R {
    STEPPERS.with(|c| c.set(true));
    let r = f();
    STEPPERS.with(|c| c.set(false));
    r
}

/// Whether a whole number gets −/+ buttons: small ones do, big ones (experience, gold) are typed.
pub fn steps(n: &Value) -> bool {
    n.as_i64().is_some_and(|i| i.unsigned_abs() <= STEP_MAX)
}

const STEP_MAX: u64 = 5000;

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

/// A typed number pulled back into the range the format accepts, so a value can never go
/// outside it (the typing box keeps what was typed; the save gets the nearest allowed value).
pub fn clamp_to(range: Option<(Value, Value)>, v: Value) -> Value {
    let Some((lo, hi)) = range else { return v };
    let (Some(x), Some(l), Some(h)) = (v.as_f64(), lo.as_f64(), hi.as_f64()) else { return v };
    if x < l {
        lo
    } else if x > h {
        hi
    } else {
        v
    }
}

/// `v` moved by `steps` whole units (the step of the -/+ buttons is 1; decimal fields use 1 too),
/// pulled into the field's range. `None` if the result can't be stored as a number.
fn step_number(v: &Value, steps: i64, range: Option<(Value, Value)>) -> Option<Value> {
    let nv = if let Some(i) = v.as_i64() {
        Value::from(i.saturating_add(steps))
    } else if let Some(u) = v.as_u64() {
        let x = i128::from(u) + i128::from(steps);
        Value::from(u64::try_from(x.max(0)).unwrap_or(u64::MAX))
    } else {
        serde_json::Number::from_f64(v.as_f64()? + steps as f64).map(Value::Number)?
    };
    Some(clamp_to(range, nv))
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
                if &hi != v && ui.button(trf("Set to maximum ({0})", &[&display(&hi)])).on_hover_text(tr("The largest value this field accepts")).clicked() {
                    out.edit = Some(hi);
                    ui.close_menu();
                }
                if &lo != v && ui.button(trf("Set to minimum ({0})", &[&display(&lo)])).on_hover_text(tr("The smallest value this field accepts")).clicked() {
                    out.edit = Some(lo);
                    ui.close_menu();
                }
            }
            if let Some(orig) = opts.baseline.and_then(|b| b.pointer(ptr)).filter(|o| *o != v) {
                ui.separator();
                if ui.button(trf("Revert to original ({0})", &[&display(orig)])).clicked() {
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
            lines.push(trf("The save file accepts {0} to {1} (the game may expect less). Right-click for Max.", &[&display(&lo), &display(&hi)]));
        }
    }
    if let Some(note) = opts.kind.and_then(|k| crate::risks::note(k, ptr)) {
        lines.push(trf("Note: {0}", &[&note]));
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

/// A value's typing box, named after the value for screen readers (its name is a separate label).
fn name_box(resp: Response, opts: &EditOpts) -> Response {
    match opts.name {
        Some(n) if !n.is_empty() => named(resp, egui::WidgetType::TextEdit, n),
        _ => resp,
    }
}

fn editor_body(ui: &mut Ui, id_src: &str, v: &Value, editable: bool, width: f32, opts: &EditOpts) -> (Option<Value>, Option<Response>) {
    if !editable {
        let why = opts.tree.map(|t| t.read_only_reason(id_src)).filter(|w| !w.is_empty()).unwrap_or_else(|| tr("Read-only: this value can't be edited safely").into());
        let r = ui.label(RichText::new(display(v)).weak().italics()).on_hover_text(why);
        return (None, Some(r));
    }
    match v {
        Value::Bool(b) => {
            let mut on = *b;
            let mut r = toggle_named(ui, &mut on, opts.name.unwrap_or_default());
            if let Some(t) = r.hovered().then(|| hover_extra(opts, id_src, v)).flatten() {
                r = r.on_hover_text(t);
            }
            let changed = r.changed();
            ui.label(if on { tr("On") } else { tr("Off") });
            (changed.then_some(Value::Bool(on)), Some(r))
        }
        Value::String(s) => {
            // Line breaks are shown as visible symbols so a one-line box can't
            // mangle multi-line text (e.g. dialogue); they're restored on edit.
            let mut text = escape_breaks(s);
            let id = ui.make_persistent_id(("str", id_src));
            let resp = ui.add_sized([width + 8.0, ui.spacing().interact_size.y], TextEdit::singleline(&mut text).id(id));
            let resp = name_box(resp, opts);
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
            // Up/Down step the value (Shift: 10 steps) through the same path as typing and −/+.
            let mut keyed = None;
            if focused {
                let steps = ui.input_mut(|i| {
                    let mut k = 0i64;
                    for (key, dir) in [(egui::Key::ArrowUp, 1), (egui::Key::ArrowDown, -1)] {
                        while i.consume_key(egui::Modifiers::SHIFT, key) {
                            k += 10 * dir;
                        }
                        while i.consume_key(egui::Modifiers::NONE, key) {
                            k += dir;
                        }
                    }
                    k
                });
                if steps != 0 {
                    let base = parse_number(&buf, integer).unwrap_or_else(|| v.clone());
                    if let Some(nv) = step_number(&base, steps, opts.tree.and_then(|t| t.range(id_src))) {
                        buf = nv.to_string();
                        ui.data_mut(|d| d.insert_temp(id, buf.clone()));
                        keyed = Some(nv);
                    }
                }
            }
            let valid = parse_number(&buf, integer).is_some();
            // Wide enough for what it holds: a fixed box cut "3270" down to "327".
            let text_w = ui.fonts(|f| f.layout_no_wrap(buf.clone(), egui::TextStyle::Body.resolve(ui.style()), Color32::WHITE).size().x);
            let w = width.min(160.0).max((text_w + 20.0).min(260.0));
            let mut edit = TextEdit::singleline(&mut buf).id(id).desired_width(w);
            if !valid {
                edit = edit.text_color(theme::fg(ui, theme::ERR));
            }
            // add_sized: a bare desired_width is cut down to what the cell has left (in a grid, a few pixels).
            let mut resp = name_box(ui.add_sized([w + 8.0, ui.spacing().interact_size.y], edit), opts);
            let mut out = keyed.filter(|nv| nv != v);
            if resp.changed() {
                ui.data_mut(|d| d.insert_temp(id, buf.clone()));
                if let Some(nv) = parse_number(&buf, integer) {
                    let nv = clamp_to(opts.tree.and_then(|t| t.range(id_src)), nv);
                    if nv != *v {
                        out = Some(nv);
                    }
                }
            }
            if !valid {
                resp = resp.on_hover_text(if integer { tr("Enter a whole number, e.g. 1500") } else { tr("Enter a number, e.g. 2.5") });
            } else if resp.hovered() {
                if let Some(hint) = hover_extra(opts, id_src, v) {
                    resp = resp.on_hover_text(hint);
                }
            }
            // On a phone-width screen the −/+ would be pushed off its edge: type the number instead.
            if integer && !COMPACT.with(|c| c.get()) && (ui.ctx().screen_rect().width() >= 520.0 || STEPPERS.with(|c| c.get())) {
                // Big values (experience, gold) are typed, not stepped.
                if let Some(i) = n.as_i64().filter(|i| i.unsigned_abs() <= STEP_MAX) {
                    let step = ui.input(|inp| if inp.modifiers.ctrl || inp.modifiers.command { 100 } else if inp.modifiers.shift { 10 } else { 1 });
                    // As tall as the number box beside them: small buttons were hard to hit.
                    let side = ui.spacing().interact_size.y.max(MIN_TARGET);
                    let tip = tr("Decrease by 1 (Shift: 10, Ctrl: 100)");
                    if named(ui.add_sized([side, side], egui::Button::new("\u{2212}")), egui::WidgetType::Button, tip).on_hover_text(tip).clicked() {
                        out = Some(clamp_to(opts.tree.and_then(|t| t.range(id_src)), Value::from(i.saturating_sub(step))));
                    }
                    let tip = tr("Increase by 1 (Shift: 10, Ctrl: 100)");
                    if named(ui.add_sized([side, side], egui::Button::new("+")), egui::WidgetType::Button, tip).on_hover_text(tip).clicked() {
                        out = Some(clamp_to(opts.tree.and_then(|t| t.range(id_src)), Value::from(i.saturating_add(step))));
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
    fn numbers_clamp_into_range() {
        let r = Some((Value::from(0), Value::from(99)));
        assert_eq!(clamp_to(r.clone(), Value::from(150)), Value::from(99));
        assert_eq!(clamp_to(r.clone(), Value::from(-3)), Value::from(0));
        assert_eq!(clamp_to(r.clone(), Value::from(42)), Value::from(42));
        assert_eq!(clamp_to(None, Value::from(1_000_000)), Value::from(1_000_000));
    }

    #[test]
    fn stepping_respects_range_and_type() {
        let r = Some((Value::from(0), Value::from(20)));
        assert_eq!(step_number(&Value::from(5), 1, r.clone()), Some(Value::from(6)));
        assert_eq!(step_number(&Value::from(5), -10, r.clone()), Some(Value::from(0)));
        assert_eq!(step_number(&Value::from(15), 10, r), Some(Value::from(20)));
        assert_eq!(step_number(&Value::from(u64::MAX), 1, None), Some(Value::from(u64::MAX)));
        assert_eq!(step_number(&Value::from(2.5), 1, None), Some(Value::from(3.5)));
    }

    fn key(k: egui::Key, shift: bool) -> egui::Event {
        egui::Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE } }
    }

    #[test]
    fn arrow_keys_step_a_focused_number_box() {
        let ctx = egui::Context::default();
        let mut v = Value::from(5);
        let frame = |ctx: &egui::Context, v: &mut Value, events: Vec<egui::Event>, focus: bool| {
            let input = egui::RawInput { events, ..Default::default() };
            let mut edit = None;
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if focus {
                        let id = ui.make_persistent_id(("num", "/n"));
                        ui.memory_mut(|m| m.request_focus(id));
                    }
                    edit = value_editor_ex(ui, "/n", v, true, 80.0, &EditOpts::default()).edit;
                });
            });
            if let Some(e) = edit {
                *v = e;
            }
        };
        frame(&ctx, &mut v, vec![], true);
        frame(&ctx, &mut v, vec![], false);
        frame(&ctx, &mut v, vec![key(egui::Key::ArrowUp, false)], false);
        assert_eq!(v, Value::from(6));
        frame(&ctx, &mut v, vec![key(egui::Key::ArrowUp, true)], false);
        assert_eq!(v, Value::from(16));
        frame(&ctx, &mut v, vec![key(egui::Key::ArrowDown, false)], false);
        assert_eq!(v, Value::from(15));
    }

    #[test]
    fn switch_flips_with_space_and_icon_buttons_are_big_enough() {
        let ctx = egui::Context::default();
        let mut on = false;
        let mut ids = (egui::Id::NULL, egui::Rect::NOTHING, egui::Rect::NOTHING);
        for (n, events) in [vec![], vec![], vec![key(egui::Key::Space, false)]].into_iter().enumerate() {
            let _ = ctx.run(egui::RawInput { events, ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let r = toggle_named(ui, &mut on, "Infinite health");
                    if n == 0 {
                        r.request_focus();
                    }
                    let b = symbol_button(ui, "\u{00D7}", "Close tab: a.sav", "Close");
                    let i = icon_button(ui, RichText::new("x"), "Forget", "Forget");
                    ids = (r.id, b.rect, i.rect);
                });
            });
        }
        assert!(on, "Space on the focused switch turns it on");
        assert!(ctx.memory(|m| m.has_focus(ids.0)));
        for r in [ids.1, ids.2] {
            assert!(r.width() >= MIN_TARGET && r.height() >= MIN_TARGET, "target {r:?} is under 24 by 24");
        }
    }

    #[test]
    fn long_lists_only_lay_out_visible_rows() {
        let ctx = egui::Context::default();
        let mut drawn = [0usize; 2];
        for (k, n) in [50usize, 5000].into_iter().enumerate() {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| long_list(ui, egui::Id::new("t"), n, |ui, _| {
                    let _ = ui.selectable_label(false, "row");
                    drawn[k] += 1;
                }));
            });
        }
        assert_eq!(drawn[0], 50, "short lists are drawn whole");
        assert!(drawn[1] > 0 && drawn[1] < 100, "long list drew {} rows", drawn[1]);
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

/// Height for a list that scrolls inside a screen: the given size on a small window, more of the
/// window on a tall one, so a short fixed box doesn't leave the rest of the screen empty.
pub fn list_height(ui: &Ui, base: f32) -> f32 {
    (ui.ctx().screen_rect().height() * 0.55).clamp(base.min(200.0), base.max(640.0))
}

/// A list or table that scrolls inside a game screen, kept at one steady size.
///
/// A plain `ScrollArea` with a maximum height shrinks to whatever it holds, so typing in its
/// filter box, picking a row or loading another save made the box (and everything below it)
/// jump. This box fills the width it is given and keeps the tallest content it has shown, up to
/// [`list_height`], for as long as it stays on screen; it starts over once it is not drawn for a
/// frame (another section, file or save), so it never keeps a stale gap.
pub trait ListBox {
    fn show_list<R>(self, ui: &mut Ui, key: impl std::hash::Hash, base: f32, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R>;
    /// Same, for a dialog or window that sizes itself to its contents: the box keeps its height
    /// but takes only the width its rows need, so it doesn't push the window wider.
    fn show_list_fit<R>(self, ui: &mut Ui, key: impl std::hash::Hash, base: f32, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R>;
}

impl ListBox for egui::ScrollArea {
    fn show_list<R>(self, ui: &mut Ui, key: impl std::hash::Hash, base: f32, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
        list_box(self, ui, key, base, true, add)
    }
    fn show_list_fit<R>(self, ui: &mut Ui, key: impl std::hash::Hash, base: f32, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
        list_box(self, ui, key, base, false, add)
    }
}

fn list_box<R>(this: egui::ScrollArea, ui: &mut Ui, key: impl std::hash::Hash, base: f32, fill: bool, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
    {
        let self_ = this;
        let key = egui::Id::new(key);
        let mem = ui.make_persistent_id(("list_box", key));
        let frame = ui.ctx().cumulative_pass_nr();
        let cap = list_height(ui, base);
        let kept: Option<f32> = ui.data(|d| d.get_temp::<(f32, u64)>(mem)).filter(|&(h, f)| h > 1.0 && frame.saturating_sub(f) <= 2).map(|(h, _)| h);
        let area = self_.id_salt(key);
        let area = match kept {
            Some(h) => area.auto_shrink([!fill, false]).min_scrolled_height(h.min(cap)).max_height(h.min(cap)),
            None => area.auto_shrink([!fill, true]).min_scrolled_height(0.0).max_height(cap),
        };
        let out = area.show_locked(ui, add);
        let tallest = kept.unwrap_or(0.0).max(out.content_size.y);
        ui.data_mut(|d| d.insert_temp(mem, (tallest, frame)));
        if kept.is_none_or(|k| tallest > k + 0.5) {
            ui.ctx().request_repaint();
        }
        out
    }
}

#[cfg(test)]
mod list_box_tests {
    use super::*;

    /// Draws one frame holding a list box of `rows` rows (or nothing, when `None`) and returns
    /// the height the box took.
    fn frame(ctx: &egui::Context, rows: Option<usize>, w: f32, h: f32) -> f32 {
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() };
        let mut height = 0.0;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(n) = rows {
                    let top = ui.cursor().top();
                    egui::ScrollArea::vertical().show_list(ui, "t", 300.0, |ui| {
                        for i in 0..n {
                            ui.label(format!("row {i}"));
                        }
                    });
                    height = ui.cursor().top() - top;
                }
            });
        });
        height
    }

    /// Filtering or selecting must not make the box jump: it keeps its size while on screen, and
    /// starts over when another screen has been shown in between.
    #[test]
    fn list_box_keeps_its_size_while_content_changes() {
        for (w, h) in [(1100.0, 900.0), (380.0, 700.0)] {
            let ctx = egui::Context::default();
            crate::theme::install_fonts(&ctx);
            let mut full = 0.0;
            for _ in 0..4 {
                full = frame(&ctx, Some(40), w, h);
            }
            assert!(full >= 150.0, "box too short to be usable: {full} at {w}x{h}");
            for n in [3, 0, 12, 40, 1] {
                for _ in 0..3 {
                    assert_eq!(frame(&ctx, Some(n), w, h), full, "box resized with {n} rows at {w}x{h}");
                }
            }
            // Another screen is shown for a while, then a short list: no stale gap.
            for _ in 0..4 {
                frame(&ctx, None, w, h);
            }
            let mut short = 0.0;
            for _ in 0..4 {
                short = frame(&ctx, Some(3), w, h);
            }
            assert!(short < full, "box kept a stale size ({short} vs {full})");
        }
    }

    /// Every list that scrolls inside a screen goes through `show_list`; a bare ScrollArea with a
    /// maximum height shrinks and grows with its rows.
    #[test]
    fn scroll_areas_with_a_max_height_use_the_shared_box() {
        fn walk(dir: &std::path::Path, bad: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, bad);
                } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "widgets.rs") {
                    let text = std::fs::read_to_string(&p).unwrap();
                    for (i, l) in text.lines().enumerate() {
                        if l.contains("ScrollArea::") && l.contains(".max_height(") && !l.contains("auto_shrink([false, false])") && !l.contains("auto_shrink([true, true])") {
                            bad.push(format!("{}:{}", p.display(), i + 1));
                        }
                    }
                }
            }
        }
        let mut bad = vec![];
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut bad);
        assert!(bad.is_empty(), "use ScrollArea::..show_list instead of max_height at: {bad:?}");
    }
}
