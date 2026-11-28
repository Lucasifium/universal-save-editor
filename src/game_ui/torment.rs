//! Planescape: Torment: Enhanced Edition (`BALDUR.gam`, same file layout as the other
//! Enhanced Editions): the party's coppers, the Nameless One and his companions, each with
//! experience, level, hit points, abilities, alignment and inventory stacks.

use crate::i18n::tr;
use crate::widgets::ListBox;
use super::{columns, fact, fmt_num, hero, panel, tabs, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(150, 110, 170);
    let members = keys(cx, "/Party members");
    let mut sub = vec!["Planescape: Torment (Enhanced Edition)".to_string()];
    if let Some(a) = cx.text("/Party/Area").filter(|a| !a.is_empty()) {
        sub.push(a);
    }
    if let Some(h) = cx.int("/Party/Game hours") {
        sub.push(crate::i18n::trf("{0} h of game time", &[&h]));
    }
    let title = members.first().cloned().unwrap_or_else(|| "The Nameless One".into());
    hero(ui, accent, &title, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Coppers", "/Party/Gold", accent);
        let total: i64 = members.iter().filter_map(|m| cx.int(&join(&join("/Party members", m), "Experience"))).sum();
        fact(ui, "Party experience", &fmt_num(total as f64), accent);
        fact(ui, "Companions", &members.len().saturating_sub(1).to_string(), accent);
    });
    if members.is_empty() {
        return;
    }
    let t = tabs(ui, "pst_party", &members);
    let base = join("/Party members", &members[t]);
    let line: Vec<String> = ["Class", "Race"].iter().filter_map(|k| cx.text(&join(&base, k))).filter(|s| !s.is_empty()).collect();
    ui.label(RichText::new(line.join("  \u{00B7}  ")).size(crate::theme::size::BODY).color(ui.visuals().weak_text_color()));
    ui.add_space(4.0);
    let levels: Vec<String> = keys(cx, &base).into_iter().filter(|k| k == "Level" || k.ends_with(" level")).collect();
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Life and growth", |ui| {
                let first = "Experience";
                let mut rows: Vec<(String, String, f32)> = vec![(first.into(), join(&base, first), 110.0)];
                rows.extend(levels.iter().map(|l| (l.clone(), join(&base, l), 70.0)));
                let more = [("Hit points", 70.0), ("Max hit points", 70.0), ("Alignment", 150.0)];
                rows.extend(more.map(|(k, w)| (k.to_string(), join(&base, k), w)));
                cx.value_grid(ui, "pst_char", &rows);
                ui.weak(tr("Levels don't follow experience by themselves: set both."));
            });
        } else {
            panel(ui, "Abilities", |ui| {
                for k in ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"] {
                    // cap: Planescape: Torment ability scores stop at 25
                    cx.capped_attribute(ui, k, &join(&base, k), 25.0, accent);
                }
            });
        }
    });
    let items_base = join(&base, "Items");
    let items = keys(cx, &items_base);
    if !items.is_empty() {
        panel(ui, "Inventory", |ui| {
            egui::ScrollArea::vertical().show_list(ui, ("pst_items", &base), 360.0, |ui| {
                egui::Grid::new(("pst_items_g", &base)).num_columns(2).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                    ui.strong(tr("Item"));
                    ui.strong(tr("Count / charges"));
                    ui.end_row();
                    for it in &items {
                        let q = join(&join(&items_base, it), "Quantity / charges");
                        ui.label(it.as_str());
                        if cx.can_edit(&q) {
                            cx.compact(ui, &q, 70.0);
                        } else {
                            ui.weak("\u{2013}");
                        }
                        ui.end_row();
                    }
                });
            });
            ui.weak(tr("Items by their game resource name; stacks (charms, bandages) and charges can be changed. Items can't be added here."));
        });
    }
}
