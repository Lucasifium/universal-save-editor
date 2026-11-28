//! Action RPG character screens: Grim Dawn, Titan Quest and Torchlight II.
//!
//! Skills and items in Grim Dawn and Titan Quest are shown by the game's internal record
//! names (made readable by the reader); their real names live in the game's own database.

use crate::i18n::{tr, trf};
use crate::widgets::ListBox;
use super::{columns, fact, fmt_num, hero, panel, tabs, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, Ui};
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

/// A text filter remembered per screen.
fn filter_box(ui: &mut Ui, id: &str, hint: &str) -> String {
    let sid = egui::Id::new(("arpg_filter", id));
    let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(tr("Filter:"));
        ui.add(egui::TextEdit::singleline(&mut q).hint_text(hint).desired_width(220.0));
    });
    ui.data_mut(|d| d.insert_temp(sid, q.clone()));
    q.to_lowercase()
}

/// Label + editor rows, spread over as many columns as fit.
fn rows(ui: &mut Ui, cx: &mut GameCtx, id: &str, rows: &[(String, String)]) {
    if rows.is_empty() {
        ui.weak(tr("Nothing here."));
        return;
    }
    super::fit_rows(ui, cx, id, rows, 80.0, 84.0);
}

/// A filterable list of the values under `base` (skills, a bag's stacks).
fn list(ui: &mut Ui, cx: &mut GameCtx, id: &str, base: &str, hint: &str) {
    let all = keys(cx, base);
    let q = if all.len() > 12 { filter_box(ui, id, hint) } else { String::new() };
    let shown: Vec<(String, String)> = all.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).map(|k| (k.clone(), join(base, k))).collect();
    egui::ScrollArea::vertical().show_list(ui, (id, "scroll"), 420.0, |ui| rows(ui, cx, id, &shown));
}

/// Tabs over the groups of item stacks that exist in this save.
fn stacks(ui: &mut Ui, cx: &mut GameCtx, id: &str, groups: &[String], note: &str) {
    if groups.is_empty() {
        return;
    }
    panel(ui, "Item stacks", |ui| {
        let labels: Vec<String> = groups.iter().map(|g| format!("{g} ({})", keys(cx, &join("", g)).len())).collect();
        let t = tabs(ui, id, &labels);
        list(ui, cx, &format!("{id}{t}"), &join("", &groups[t]), "item name");
        ui.weak(note); // i18n-ok: a name or value from the save, or already translated
    });
}

fn subtitle(parts: Vec<String>) -> String {
    parts.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("  \u{00B7}  ")
}

// ------------------------------------------------------------------ Grim Dawn

pub fn grim_dawn(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(201, 124, 44);
    let name = cx.text("/Character/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Character".into());
    let mut sub = vec![cx.text("/Character/Class tag").unwrap_or_default()];
    if cx.flag("/Character/Hardcore") {
        sub.push(crate::i18n::tr("Hardcore").into());
    }
    sub.push(cx.text("/Character/Difficulty").map(|d| crate::i18n::trf("{0} difficulty", &[&d])).unwrap_or_default());
    hero(ui, accent, &name, &subtitle(sub), |ui| {
        for (label, p) in [("Level", "/Character/Level"), ("Experience", "/Character/Experience"), ("Iron bits", "/Character/Money"), ("Tributes", "/Character/Tributes"), ("Difficulty", "/Character/Difficulty"), ("Highest difficulty unlocked", "/Character/Highest difficulty")] {
            cx.tile(ui, label, p, accent);
        }
    });
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Attributes", |ui| {
                let names = ["Physique", "Cunning", "Spirit"];
                let mut rows: Vec<(&str, String)> = names.into_iter().map(|k| (k, join("/Attributes", k))).collect();
                rows.extend([("Base health", "/Attributes/Health".to_string()), ("Base energy", "/Attributes/Energy".to_string())]);
                cx.open_attributes(ui, &rows);
                ui.weak(tr("Base values before items and skills."));
            });
        } else {
            panel(ui, "Unspent points", |ui| {
                let names = ["Attribute points", "Skill points", "Devotion points"];
                let rows: Vec<(&str, String)> = names.into_iter().map(|k| (k, join("/Points", k))).collect();
                cx.open_attributes(ui, &rows);
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if let Some(n) = cx.int("/Points/Devotion points unlocked") {
                        fact(ui, "Devotion points unlocked", &fmt_num(n as f64), accent);
                    }
                    if let Some(n) = cx.int("/Statistics/Play time (seconds)") {
                        fact(ui, "Played", &crate::i18n::trf("{0} hours", &[&format!("{:.1}", n as f64 / 3600.0)]), accent);
                    }
                    if let Some(n) = cx.int("/Statistics/Kills") {
                        fact(ui, "Kills", &fmt_num(n as f64), accent);
                    }
                    if let Some(n) = cx.int("/Statistics/Deaths") {
                        fact(ui, "Deaths", &fmt_num(n as f64), accent);
                    }
                });
            });
        }
    });
    for (base, title) in [("/Skills", "Skills"), ("/Devotions", "Devotions")] {
        if !keys(cx, base).is_empty() {
            panel(ui, title, |ui| {
                list(ui, cx, &format!("gd{base}"), base, "skill name");
                ui.weak(tr("Ranks put into each skill. Names are the game's internal records (class folder: file name)."));
            });
        }
    }
    let groups: Vec<String> = keys(cx, "").into_iter().filter(|k| k == "Inventory" || k.starts_with("Bag ") || k.starts_with("Stash ")).collect();
    stacks(ui, cx, "gd_items", &groups, crate::i18n::tr("How many are in each stack (items that don't stack are 1). Names are the item's record (folder: file name)."));
    if let Some(Value::Array(eq)) = cx.get("/Character/Equipped").cloned() {
        if !eq.is_empty() {
            panel(ui, "Equipped", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for e in eq.iter().filter_map(Value::as_str) {
                        ui.label(egui::RichText::new(e).color(crate::theme::fg(ui, accent)));
                        ui.add_space(8.0);
                    }
                });
            });
        }
    }
    let factions = match cx.get("/Factions") {
        Some(Value::Object(m)) => m.keys().cloned().collect::<Vec<_>>(),
        _ => vec![],
    };
    if !factions.is_empty() {
        panel(ui, "Factions", |ui| {
            for f in &factions {
                let base = join("/Factions", f);
                ui.horizontal(|ui| {
                    ui.add_sized([150.0, 18.0], egui::Label::new(f));
                    cx.compact(ui, &join(&base, "Reputation"), 90.0);
                    cx.compact(ui, &join(&base, "Unlocked"), 70.0);
                });
            }
            ui.weak(tr("Reputation runs from -25000 (nemesis) to 25000 (revered, the most). \"Faction N\" are factions the editor has no name for."));
        });
    }
}

// ------------------------------------------------------------------ Titan Quest

pub fn titan_quest(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(64, 128, 196);
    let name = cx.text("/Character/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Hero".into());
    let sub = vec![cx.text("/Character/Class").unwrap_or_default(), cx.text("/Character/Class tag").unwrap_or_default()];
    hero(ui, accent, &name, &subtitle(sub), |ui| {
        for (label, p) in [("Level", "/Character/Level"), ("Experience", "/Character/Experience"), ("Gold", "/Character/Money")] {
            cx.tile(ui, label, p, accent);
        }
    });
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Attributes", |ui| {
                let names = ["Strength", "Dexterity", "Intelligence", "Health", "Energy"];
                let rows: Vec<(&str, String)> = names.into_iter().map(|k| (k, join("/Attributes", k))).collect();
                cx.open_attributes(ui, &rows);
                ui.weak(tr("Base values before items and skills."));
            });
        } else {
            panel(ui, "Unspent points", |ui| {
                cx.open_attributes(ui, &[("Attribute points", "/Points/Attribute points"), ("Skill points", "/Points/Skill points")]);
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if let Some(n) = cx.int("/Statistics/Play time (seconds)") {
                        fact(ui, "Played", &crate::i18n::trf("{0} hours", &[&format!("{:.1}", n as f64 / 3600.0)]), accent);
                    }
                    for k in ["Kills", "Deaths"] {
                        if let Some(n) = cx.int(&join("/Statistics", k)) {
                            fact(ui, k, &fmt_num(n as f64), accent);
                        }
                    }
                });
            });
        }
    });
    if !keys(cx, "/Skills").is_empty() {
        panel(ui, "Skills", |ui| {
            list(ui, cx, "tq_skills", "/Skills", "skill name");
            ui.weak(tr("Each mastery's level and the points in each skill. Names are the game's internal records (mastery folder: file name)."));
        });
    }
}

// ------------------------------------------------------------------ Torchlight II

pub fn torchlight2(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(46, 160, 150);
    let name = cx.text("/Hero/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Hero".into());
    let mut sub = vec![cx.text("/Hero/Class").unwrap_or_default(), cx.text("/Hero/Difficulty").unwrap_or_default()];
    if cx.flag("/Hero/Hardcore") {
        sub.push(crate::i18n::tr("Hardcore").into());
    }
    if let Some(n) = cx.int("/Hero/New game plus").filter(|&n| n > 0) {
        sub.push(format!("NG+{n}"));
    }
    if let Some(s) = cx.num("/Hero/Play time (seconds)") {
        sub.push(crate::i18n::trf("{0} hours", &[&format!("{:.1}", s / 3600.0)]));
    }
    hero(ui, accent, &name, &subtitle(sub), |ui| {
        for (label, p) in [("Level", "/Progress/Level"), ("Experience", "/Progress/Experience"), ("Gold", "/Progress/Gold"), ("Fame level", "/Progress/Fame level"), ("Fame", "/Progress/Fame"), ("Difficulty", "/Hero/Difficulty")] {
            cx.tile(ui, label, p, accent);
        }
    });
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Stats", |ui| {
                let names = ["Strength", "Dexterity", "Vitality", "Focus"];
                let rows: Vec<(&str, String)> = names.into_iter().map(|k| (k, join("/Stats", k))).collect();
                cx.open_attributes(ui, &rows);
            });
        } else {
            panel(ui, "Points, health and mana", |ui| {
                cx.open_attributes(ui, &[("Attribute points", "/Points/Attribute points"), ("Skill points", "/Points/Skill points"), ("Health", "/Health and mana/Health"), ("Mana", "/Health and mana/Mana")]);
                let spells: Vec<String> = keys(cx, "/Spells");
                if !spells.is_empty() {
                    ui.add_space(6.0);
                    ui.strong(tr("Spells"));
                    for s in spells {
                        let lvl = cx.int(&join("/Spells", &s)).unwrap_or(0);
                        ui.label(trf("{0}  (level {1})", &[&s, &lvl]));
                    }
                }
            });
        }
    });
    stacks(ui, cx, "tl2_items", &["Items".to_string()].into_iter().filter(|g| cx.get(&join("", g)).is_some()).collect::<Vec<_>>(), crate::i18n::tr("How many are in each stack: equipped, carried and stashed items, by the name the save keeps for them (items that don't stack are 1)."));
}
