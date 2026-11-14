//! Baldur's Gate / Icewind Dale (Infinity Engine, Enhanced Editions): the party's gold and
//! reputation, and each party member's sheet (experience, hit points, levels, abilities,
//! alignment and item stacks).

use crate::widgets::ScrollLock;
use super::{columns, fact, hero, panel, tabs, GameCtx};
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
    let accent = Color32::from_rgb(176, 128, 48);
    let members = keys(cx, "/Party members");
    let leader = members.first().cloned().unwrap_or_else(|| "Party".into());
    let mut sub = vec!["Baldur's Gate / Icewind Dale (Enhanced Edition)".to_string()];
    if let Some(c) = cx.int("/Party/Chapter") {
        sub.push(format!("Chapter {c}"));
    }
    if let Some(a) = cx.text("/Party/Area").filter(|a| !a.is_empty()) {
        sub.push(a);
    }
    hero(ui, accent, &leader, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Party gold", "/Party/Gold", accent);
        cx.tile(ui, "Reputation", "/Party/Reputation", accent);
        cx.tile(ui, "Reputation (x10)", "/Party/Reputation (x10)", accent);
        fact(ui, "Party members", &members.len().to_string(), accent);
    });
    if members.is_empty() {
        return;
    }
    let t = tabs(ui, "ie_party", &members);
    let base = join("/Party members", &members[t]);
    character(ui, cx, &base, accent);
    let globals = keys(cx, "/Global variables");
    if !globals.is_empty() {
        super::screens2::panel_globals(ui, cx, &globals);
    }
}

fn character(ui: &mut Ui, cx: &mut GameCtx, base: &str, accent: Color32) {
    let line: Vec<String> = ["Class", "Race", "Creature"].iter().filter_map(|k| cx.text(&join(base, k))).filter(|s| !s.is_empty()).collect();
    ui.label(RichText::new(line.join("  \u{00B7}  ")).size(14.0).color(ui.visuals().weak_text_color()));
    ui.add_space(4.0);
    let levels: Vec<String> = keys(cx, base).into_iter().filter(|k| k == "Level" || k.ends_with(" level")).collect();
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Character", |ui| {
                egui::Grid::new(("ie_char", base)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    cx.grid_row(ui, "Experience", &join(base, "Experience"), 110.0);
                    for l in &levels {
                        cx.grid_row(ui, l, &join(base, l), 70.0);
                    }
                    cx.grid_row(ui, "Hit points", &join(base, "Hit points"), 70.0);
                    cx.grid_row(ui, "Max hit points", &join(base, "Max hit points"), 70.0);
                    cx.grid_row(ui, "Alignment", &join(base, "Alignment"), 150.0);
                });
                ui.weak("Levels don't follow experience by themselves: set both. Hit points above the maximum are the game's own temporary bonus.");
            });
        } else {
            panel(ui, "Abilities", |ui| {
                for k in ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"] {
                    cx.attribute(ui, k, &join(base, k), 25.0, accent);
                }
                cx.attribute(ui, "Strength 18/xx", &join(base, "Strength bonus (18/xx)"), 100.0, accent);
                ui.weak("The 18/xx bonus only counts for warriors with Strength 18.");
            });
        }
    });
    let items_base = join(base, "Items");
    let items = keys(cx, &items_base);
    if !items.is_empty() {
        panel(ui, "Items", |ui| {
            egui::ScrollArea::vertical().id_salt(("ie_items", base)).max_height(380.0).show_locked(ui, |ui| {
                egui::Grid::new(("ie_items_g", base)).num_columns(4).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                    ui.strong("Item");
                    ui.strong("Quantity / charges");
                    ui.strong("Charges 2");
                    ui.strong("Charges 3");
                    ui.end_row();
                    for it in &items {
                        let p = join(&items_base, it);
                        ui.label(it.as_str());
                        for k in ["Quantity / charges", "Charges 2", "Charges 3"] {
                            let q = join(&p, k);
                            if cx.can_edit(&q) {
                                cx.compact(ui, &q, 70.0);
                            } else {
                                ui.weak("\u{2013}");
                            }
                        }
                        ui.end_row();
                    }
                });
            });
            ui.weak("Items by their game resource name. Stack sizes (arrows, potions) and charges (wands) can be changed; items without either show \u{2013}. Items can't be added here.");
        });
    }
}
