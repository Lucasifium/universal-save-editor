//! Clair Obscur: Expedition 33 (`EXPEDITION_*.sav`, an Unreal Engine save): gold, play time,
//! each party member's level, experience, attribute points and Lumina from Colours of
//! Lumina, each weapon's level, and the item counts.
//! Property names carry a generated suffix (`CurrentLevel_49_97AB…`), so values are found by
//! the part before it.

use super::{columns, fact, fmt_num, hero, panel, tabs, words, GameCtx};
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

/// The full key under `base` that starts with `prefix` followed by `_` and a number.
fn find(cx: &GameCtx, base: &str, prefix: &str) -> Option<String> {
    keys(cx, base).into_iter().find(|k| {
        k.strip_prefix(prefix)
            .is_some_and(|r| r.starts_with('_') && r[1..].starts_with(|c: char| c.is_ascii_digit()))
    })
}

/// Cosmetic unlocks sit in the item list too; they aren't things to count.
fn is_cosmetic(name: &str) -> bool {
    name.starts_with("Skin")
        || name.starts_with("Face")
        || name.starts_with("Hair")
        || name.starts_with("Outfit")
}

fn attribute(key: &str) -> String {
    // The game stores these as "ECharacterAttribute::NewEnumeratorN"; the names behind N
    // aren't mapped, so they show by number.
    match key
        .rsplit("NewEnumerator")
        .next()
        .and_then(|n| n.parse::<u32>().ok())
    {
        Some(n) => format!("Attribute {}", n + 1),
        None => words(key),
    }
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(190, 150, 90);
    let members = keys(cx, "/CharactersCollection");
    let mut sub = vec!["Clair Obscur: Expedition 33".to_string()];
    if let Some(m) = cx.text("/CurrentCharacterWorld").filter(|m| !m.is_empty()) {
        sub.push(format!("playing {m}"));
    }
    if let Some(Value::Number(t)) = cx.get("/TimePlayed") {
        if let Some(s) = t.as_f64() {
            sub.push(format!("{:.0} h played", s / 3600.0));
        }
    }
    hero(
        ui,
        accent,
        "Expedition 33",
        &sub.join("  \u{00B7}  "),
        |ui| {
            cx.tile(ui, "Gold", "/Gold", accent);
            fact(ui, "Party members", &members.len().to_string(), accent);
        },
    );
    let mut names = members.clone();
    names.push("Items".into());
    let weapons = match cx.get("/WeaponProgressions") {
        Some(Value::Array(a)) => a.len(),
        _ => 0,
    };
    if weapons > 0 {
        names.push("Weapons".into());
    }
    let t = tabs(ui, "e33_tabs", &names);
    if t < members.len() {
        let base = join("/CharactersCollection", &members[t]);
        columns(ui, 360.0, 2, |ui, i| {
            if i == 0 {
                panel(ui, "Level and vitals", |ui| {
                    egui::Grid::new(("e33_char", &base))
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            for (label, prefix) in [
                                ("Level", "CurrentLevel"),
                                ("Experience", "CurrentExperience"),
                                ("Available action points", "AvailableActionPoints"),
                                ("Current HP", "CurrentHP"),
                                ("Current MP", "CurrentMP"),
                                ("Lumina from Colours of Lumina", "LuminaFromConsumables"),
                            ] {
                                if let Some(k) = find(cx, &base, prefix) {
                                    cx.grid_row(ui, label, &join(&base, &k), 110.0);
                                }
                            }
                        });
                    ui.weak("Levels don't follow experience by themselves: set both.");
                });
            } else if let Some(k) = find(cx, &base, "AssignedAttributePoints") {
                let ab = join(&base, &k);
                panel(ui, "Attribute points", |ui| {
                    egui::Grid::new(("e33_attr", &base))
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            for a in keys(cx, &ab) {
                                cx.grid_row(ui, &attribute(&a), &join(&ab, &a), 90.0);
                            }
                        });
                });
            }
        });
    } else if t == members.len() + 1 {
        panel(ui, "Weapon levels", |ui| {
            egui::Grid::new("e33_weapons")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for i in 0..weapons {
                        let base = format!("/WeaponProgressions/{i}");
                        let (Some(id), Some(lv)) = (
                            find(cx, &base, "DefinitionID"),
                            find(cx, &base, "CurrentLevel"),
                        ) else {
                            continue;
                        };
                        let name = cx.text(&join(&base, &id)).unwrap_or_default();
                        cx.grid_row(ui, &words(&name), &join(&base, &lv), 90.0);
                    }
                });
            ui.weak("Each weapon you own and the level it has been upgraded to.");
        });
    } else {
        panel(ui, "Items", |ui| {
            egui::Grid::new("e33_items")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for k in keys(cx, "/InventoryItems")
                        .into_iter()
                        .filter(|k| !is_cosmetic(k))
                    {
                        cx.grid_row(ui, &words(&k), &join("/InventoryItems", &k), 90.0);
                    }
                });
            ui.weak("Counts only; weapons and picto sit in the list at their owned count.");
            super::crafting::offer(super::Game::Expedition33, ui, cx);
        });
    }
    let _ = fmt_num;
}
