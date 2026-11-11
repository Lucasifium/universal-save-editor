//! Reusable editing widgets. Editors never mutate data directly: they return the
//! new value so the app can record it for undo/redo and change tracking.

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
    s.replace('\n', "\u{23CE}").replace('\r', "\u{240D}")
}

fn unescape_breaks(s: &str) -> String {
    s.replace('\u{23CE}', "\n").replace('\u{240D}', "\r")
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
        let r = ui.add(TextEdit::singleline(&mut filter).hint_text("Type to filter…").desired_width(f32::INFINITY));
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

pub fn value_editor(ui: &mut Ui, id_src: &str, v: &Value, editable: bool, width: f32) -> Option<Value> {
    if !editable {
        ui.label(RichText::new(display(v)).weak().italics()).on_hover_text("Read-only: this value can't be edited safely");
        return None;
    }
    match v {
        Value::Bool(b) => {
            let mut on = *b;
            let changed = toggle(ui, &mut on).changed();
            ui.label(if on { "On" } else { "Off" });
            changed.then_some(Value::Bool(on))
        }
        Value::String(s) => {
            // Line breaks are shown as visible symbols so a one-line box can't
            // mangle multi-line text (e.g. dialogue); they're restored on edit.
            let mut text = escape_breaks(s);
            let id = ui.make_persistent_id(("str", id_src));
            let resp = ui.add(TextEdit::singleline(&mut text).id(id).desired_width(width));
            let resp = if s.contains(['\n', '\r']) { resp.on_hover_text("\u{23CE} marks a line break") } else { resp };
            resp.changed().then(|| Value::String(unescape_breaks(&text)))
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
            let mut edit = TextEdit::singleline(&mut buf).id(id).desired_width(width.min(160.0));
            if !valid {
                edit = edit.text_color(theme::ERR);
            }
            let mut resp = ui.add(edit);
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
            }
            let _ = resp;
            if integer && !COMPACT.with(|c| c.get()) {
                if let Some(i) = n.as_i64() {
                    if ui.small_button("\u{2212}").on_hover_text("Decrease by 1").clicked() {
                        out = Some(Value::from(i.saturating_sub(1)));
                    }
                    if ui.small_button("+").on_hover_text("Increase by 1").clicked() {
                        out = Some(Value::from(i.saturating_add(1)));
                    }
                }
            }
            out
        }
        Value::Null => {
            ui.label(RichText::new("empty").weak().italics());
            None
        }
        _ => None,
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
