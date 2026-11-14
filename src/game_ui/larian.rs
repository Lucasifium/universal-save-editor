//! Larian saves: Divinity: Original Sin 2 (the party, editable) and Baldur's Gate 3 (the
//! save's party summary, and gold and stack sizes of what each companion carries). See `fmt/larian.rs`.

use crate::i18n::{tr, trf};
use crate::widgets::ScrollLock;
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
        ui.weak(tr("No player characters found in this save."));
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
    line.push(if cx.flag(&at("In party")) { tr("In the party").into() } else { tr("Not in the party").into() });
    ui.label(RichText::new(line.join("  \u{00B7}  ")).color(theme_fg(ui, teal)));
    ui.add_space(6.0);
    columns(ui, 360.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Attributes", |ui| {
                for a in ATTRIBUTES {
                    cx.attribute(ui, a, &format!("{base}/Attributes/{a}"), 40.0, gold);
                }
                ui.weak(tr("Base points (10 is the starting value), before items and talents."));
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
                ui.weak(tr("The level follows from experience; points not yet spent can be spent in the game. Vitality is the current value."));
            });
        }
    });
    let piles = keys(cx, &at("Gold"));
    panel(ui, "Gold", |ui| {
        if piles.is_empty() {
            ui.weak(tr("This character carries no gold."));
            return;
        }
        egui::Grid::new(("dos2_gold", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
            for k in &piles {
                cx.grid_row(ui, k, &format!("{base}/Gold/{}", esc(k)), 120.0);
            }
        });
        ui.label(RichText::new(trf("{0} gold on {1}", &[&fmt_num(gold_of(cx, &base) as f64), &members[t]])).color(theme_fg(ui, gold)));
        ui.weak(tr("Each line is a stack of coins in this character's inventory (named by its stats entry)."));
    });
    let things = keys(cx, &at("Inventory"));
    if !things.is_empty() {
        panel(ui, &trf("Inventory ({0} stacks)", &[&things.len()]), |ui| {
            egui::ScrollArea::vertical().id_salt(("dos2_inv", t)).max_height(320.0).show_locked(ui, |ui| {
                columns(ui, 360.0, things.len(), |ui, i| {
                    cx.row_fixed(ui, &things[i].replace('_', " "), &format!("{base}/Inventory/{}", esc(&things[i])), 190.0, 60.0);
                });
            });
            ui.weak(
                tr("How many of each item this character carries directly (items inside bags aren't listed), named by the game's item entry. Change a stack size, or add items below; the game may trim a stack to what the item allows."),
            );
        });
    }
    add_items(ui, cx, tr("Every kind of item found in an inventory in this save can be added, as a copy of one the game made. Items go into the chosen character's bag when you save; where they already carry a stack of the same item, that stack grows instead. Keys, containers and story items aren't offered. Keep a backup: this hasn't been tried in the game yet."));
}

/// "Add items": who gets them, and an amount per item kind (see `fmt::larian_add` and
/// `fmt::bg3_add`), with `hint` under the list.
fn add_items(ui: &mut Ui, cx: &mut GameCtx, hint: &str) {
    let kinds: Vec<String> = keys(cx, "/Add items").into_iter().filter(|k| k != "Add to").collect();
    if kinds.is_empty() {
        return;
    }
    panel(ui, "Add items", |ui| {
        cx.row_fixed(ui, "Add to", "/Add items/Add to", 120.0, 200.0);
        ui.add_space(4.0);
        let q = super::screens2::filter_box(ui, "dos2_add", tr("potion, arrow, scroll…"));
        let shown: Vec<(String, String)> = kinds
            .iter()
            .filter(|k| q.is_empty() || k.to_lowercase().contains(&q) || k.replace('_', " ").to_lowercase().contains(&q))
            .map(|k| (k.replace('_', " "), format!("/Add items/{}", esc(k))))
            .collect();
        let chosen = kinds.iter().filter(|k| cx.text(&format!("/Add items/{}", esc(k))).is_some_and(|v| v != "Don't add")).count();
        if chosen > 0 {
            ui.label(RichText::new(trf("{0} kinds of item to add", &[&chosen])).strong());
        }
        if shown.len() > 300 {
            ui.weak(trf("{0} items match; type to narrow the list.", &[&shown.len()]));
        }
        egui::ScrollArea::vertical().id_salt("dos2_add_list").max_height(360.0).show_locked(ui, |ui| {
            super::screens2::value_rows(ui, cx, "dos2_add_rows", &shown[..shown.len().min(300)]);
        });
        ui.weak(hint);
    });
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
            ui.weak(tr("The save's summary lists no party."));
            return;
        }
        let s = |c: &Value, k: &str| match c.get(k) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(v) => v.to_string(),
        };
        egui::ScrollArea::horizontal().show_locked(ui, |ui| {
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
        ui.weak(tr("From the save's own summary."));
    });
    bg3_carried(ui, cx, gold);
    add_items(ui, cx, tr("Every kind of item in your party's bags can be added, as a copy of one the game made. The copies go into the chosen character's bag when you save. Items kept in a stack, worn gear, containers, keys and story items aren't offered; a stack's size can be changed under Stacked items. Keep a backup: this hasn't been tried in the game yet."));
    super::screens2::story(ui, cx, gold);
}

/// Gold and stack sizes of what each companion carries (see `fmt::larian::bg3_carried`).
fn bg3_carried(ui: &mut Ui, cx: &mut GameCtx, gold: Color32) {
    let people = keys(cx, "/Characters");
    if people.is_empty() {
        ui.weak(tr("Gold and items aren't readable in this save, so nothing here can be changed."));
        return;
    }
    let t = tabs(ui, "bg3_who", &people);
    let base = format!("/Characters/{}", esc(&people[t]));
    let piles = keys(cx, &format!("{base}/Gold"));
    let sum: i64 = piles.iter().filter_map(|k| cx.int(&format!("{base}/Gold/{}", esc(k)))).sum();
    panel(ui, "Gold", |ui| {
        ui.label(RichText::new(trf("{0} gold on {1}", &[&fmt_num(sum as f64), &people[t]])).strong().color(theme_fg(ui, gold)));
        if piles.is_empty() {
            ui.weak(tr("No coins on this character."));
            return;
        }
        egui::Grid::new(("bg3_gold", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
            for k in &piles {
                cx.grid_row(ui, k, &format!("{base}/Gold/{}", esc(k)), 120.0);
            }
        });
        ui.weak(tr("Each line is one stack of coins in this character's inventory."));
    });
    let things = keys(cx, &format!("{base}/Items"));
    if !things.is_empty() {
        panel(ui, &trf("Stacked items ({0})", &[&things.len()]), |ui| {
            let q = super::screens2::filter_box(ui, &format!("bg3_items{t}"), tr("potion, arrow, scroll…"));
            let shown: Vec<&String> = things.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).collect();
            egui::ScrollArea::vertical().id_salt(("bg3_inv", t)).max_height(360.0).show_locked(ui, |ui| {
                columns(ui, 300.0, shown.len(), |ui, i| {
                    cx.row_fixed(ui, &shown[i].replace('_', " "), &format!("{base}/Items/{}", esc(shown[i])), 190.0, 60.0);
                });
            });
            ui.weak(
                tr("Stack sizes of what this character carries, by the game's item name. Items can't be removed here; Add items adds new ones. A stack the game split just before saving shows as read-only."),
            );
        });
    }
}
