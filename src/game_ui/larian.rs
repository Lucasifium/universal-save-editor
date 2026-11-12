//! Larian saves: Divinity: Original Sin 2 (the party, editable) and Baldur's Gate 3 (the
//! save's party summary, and gold and stack sizes of what each companion carries). See `fmt/larian.rs`.

use super::{columns, fact, fmt_num, hero, panel, tabs, GameCtx};
use crate::fmt::larian::ATTRIBUTES;
use crate::tree::esc;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn gold_of(cx: &GameCtx, base: &str) -> i64 {
    keys(cx, &format!("{base}/Gold")).iter().filter_map(|k| cx.int(&format!("{base}/Gold/{}", esc(k)))).sum()
}

pub fn dos2(ui: &mut Ui, cx: &mut GameCtx) {
    // Teal and gold, like the game's menus.
    let teal = Color32::from_rgb(38, 150, 145);
    let gold = Color32::from_rgb(205, 165, 70);
    let members = keys(cx, "/Party");
    let mut sub = vec![];
    for k in ["Region", "Game version"] {
        if let Some(s) = cx.text(&format!("/Save/{k}")) {
            sub.push(s);
        }
    }
    let total: i64 = members.iter().map(|m| gold_of(cx, &format!("/Party/{}", esc(m)))).sum();
    hero(ui, teal, "Divinity: Original Sin 2", &sub.join("  \u{00B7}  "), |ui| {
        fact(ui, "Party", &members.len().to_string(), teal);
        fact(ui, "Gold carried", &fmt_num(total as f64), gold);
    });
    if members.is_empty() {
        ui.weak("No player characters found in this save.");
        return;
    }
    let t = tabs(ui, "dos2_party", &members);
    let base = format!("/Party/{}", esc(&members[t]));
    let at = |k: &str| format!("{base}/{k}");
    let mut line = vec![];
    for k in ["Race", "Origin"] {
        if let Some(s) = cx.text(&at(k)).filter(|s| !s.is_empty() && s != "Generic" && !s.starts_with("Generic")) {
            line.push(s);
        }
    }
    line.push(if cx.flag(&at("In party")) { "In the party".into() } else { "Not in the party".into() });
    ui.label(RichText::new(line.join("  \u{00B7}  ")).color(theme_fg(ui, teal)));
    ui.add_space(6.0);
    columns(ui, 360.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Attributes", |ui| {
                for a in ATTRIBUTES {
                    cx.attribute(ui, a, &format!("{base}/Attributes/{a}"), 40.0, gold);
                }
                ui.weak("Base points (10 is the starting value), before items and talents.");
            });
        } else {
            panel(ui, "Character", |ui| {
                egui::Grid::new(("dos2_char", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    cx.grid_row(ui, "Experience", &at("Experience"), 120.0);
                    cx.grid_row(ui, "Vitality", &at("Vitality"), 100.0);
                    for k in ["Attribute points", "Combat ability points", "Civil ability points", "Talent points"] {
                        cx.grid_row(ui, k, &format!("{base}/Points/{k}"), 80.0);
                    }
                });
                ui.weak("The level follows from experience; points not yet spent can be spent in the game. Vitality is the current value.");
            });
        }
    });
    let piles = keys(cx, &at("Gold"));
    panel(ui, "Gold", |ui| {
        if piles.is_empty() {
            ui.weak("This character carries no gold.");
            return;
        }
        egui::Grid::new(("dos2_gold", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
            for k in &piles {
                cx.grid_row(ui, k, &format!("{base}/Gold/{}", esc(k)), 120.0);
            }
        });
        ui.label(RichText::new(format!("{} gold on {}", fmt_num(gold_of(cx, &base) as f64), members[t])).color(theme_fg(ui, gold)));
        ui.weak("Each line is a stack of coins in this character's inventory (named by its stats entry).");
    });
    let things = keys(cx, &at("Inventory"));
    if !things.is_empty() {
        panel(ui, &format!("Inventory ({} stacks)", things.len()), |ui| {
            egui::ScrollArea::vertical().id_salt(("dos2_inv", t)).max_height(320.0).show(ui, |ui| {
                columns(ui, 300.0, things.len(), |ui, i| {
                    cx.row_fixed(ui, &things[i].replace('_', " "), &format!("{base}/Inventory/{}", esc(&things[i])), 190.0, 60.0);
                });
            });
            ui.weak(
                "How many of each item this character carries directly (items inside bags aren't listed), named by the game's item entry. \
                 Change a stack size; adding or removing items isn't possible here, and the game may trim a stack to what the item allows.",
            );
        });
    }
}

fn theme_fg(ui: &Ui, c: Color32) -> Color32 {
    crate::theme::fg(ui, c)
}

pub fn bg3(ui: &mut Ui, cx: &mut GameCtx) {
    let gold = Color32::from_rgb(200, 160, 80);
    let mut sub = vec![];
    for k in ["Save name", "Difficulty"] {
        if let Some(s) = cx.text(&format!("/Save/{k}")).filter(|s| !s.is_empty()) {
            sub.push(s);
        }
    }
    hero(ui, gold, "Baldur's Gate 3", &sub.join("  \u{00B7}  "), |ui| {
        for k in ["Leader", "Region", "Game version"] {
            if let Some(s) = cx.text(&format!("/Save/{k}")) {
                fact(ui, k, &s, gold);
            }
        }
    });
    let party = cx.get("/Party").and_then(Value::as_array).cloned().unwrap_or_default();
    panel(ui, "Party (read-only)", |ui| {
        if party.is_empty() {
            ui.weak("The save's summary lists no party.");
            return;
        }
        let s = |c: &Value, k: &str| match c.get(k) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(v) => v.to_string(),
        };
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("bg3_party").num_columns(5).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                for h in ["Origin", "Race", "Class", "Level", "Experience"] {
                    ui.label(RichText::new(h).strong().color(theme_fg(ui, gold)));
                }
                ui.end_row();
                for c in &party {
                    for k in ["Origin", "Race", "Class", "Level", "Experience"] {
                        ui.label(s(c, k));
                    }
                    ui.end_row();
                }
            });
        });
        ui.add_space(6.0);
        ui.weak("From the save's own summary.");
    });
    bg3_carried(ui, cx, gold);
}

/// Gold and stack sizes of what each companion carries (see `fmt::larian::bg3_carried`).
fn bg3_carried(ui: &mut Ui, cx: &mut GameCtx, gold: Color32) {
    let people = keys(cx, "/Characters");
    if people.is_empty() {
        ui.weak("Gold and items aren't readable in this save, so nothing here can be changed.");
        return;
    }
    let t = tabs(ui, "bg3_who", &people);
    let base = format!("/Characters/{}", esc(&people[t]));
    let piles = keys(cx, &format!("{base}/Gold"));
    let sum: i64 = piles.iter().filter_map(|k| cx.int(&format!("{base}/Gold/{}", esc(k)))).sum();
    panel(ui, "Gold", |ui| {
        ui.label(RichText::new(format!("{} gold on {}", fmt_num(sum as f64), people[t])).strong().color(theme_fg(ui, gold)));
        if piles.is_empty() {
            ui.weak("No coins on this character.");
            return;
        }
        egui::Grid::new(("bg3_gold", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
            for k in &piles {
                cx.grid_row(ui, k, &format!("{base}/Gold/{}", esc(k)), 120.0);
            }
        });
        ui.weak("Each line is one stack of coins in this character's inventory.");
    });
    let things = keys(cx, &format!("{base}/Items"));
    if !things.is_empty() {
        panel(ui, &format!("Stacked items ({})", things.len()), |ui| {
            let q = super::screens2::filter_box(ui, &format!("bg3_items{t}"), "potion, arrow, scroll\u{2026}");
            let shown: Vec<&String> = things.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).collect();
            egui::ScrollArea::vertical().id_salt(("bg3_inv", t)).max_height(360.0).show(ui, |ui| {
                columns(ui, 300.0, shown.len(), |ui, i| {
                    cx.row_fixed(ui, &shown[i].replace('_', " "), &format!("{base}/Items/{}", esc(shown[i])), 190.0, 60.0);
                });
            });
            ui.weak(
                "Stack sizes of what this character carries, by the game's item name. Items can't be added or removed here. \
                 A stack the game split just before saving shows as read-only.",
            );
        });
    }
}
