//! X4: Foundations (`.xml.gz` saves, opened as XML): the player's credits. The game keeps
//! them in three places (the save's info line, the player faction's account and the
//! `money_player` statistic); the box below writes all three.

use super::{fact, hero, panel, GameCtx};
use eframe::egui::{Color32, Ui};
use serde_json::{json, Value};

const ACCENT: Color32 = Color32::from_rgb(80, 190, 230);

pub fn is_x4(root: &Value) -> bool {
    root.pointer("/savegame/info/player/@money").is_some() && root.pointer("/savegame/info/game/@build").is_some() && root.pointer("/savegame/factions").is_some()
}

/// Pointers of every place that holds the player's credits.
fn money_paths(cx: &GameCtx) -> Vec<String> {
    let mut v = vec!["/savegame/info/player/@money".to_string()];
    let factions = cx.get("/savegame/factions/faction");
    match factions {
        Some(Value::Array(a)) => {
            for (i, f) in a.iter().enumerate() {
                if f.pointer("/@id").and_then(Value::as_str) == Some("player") {
                    v.push(format!("/savegame/factions/faction/{i}/account/@amount"));
                }
            }
        }
        Some(f) if f.pointer("/@id").and_then(Value::as_str) == Some("player") => v.push("/savegame/factions/faction/account/@amount".to_string()),
        _ => {}
    }
    match cx.get("/savegame/statistics/stat") {
        Some(Value::Array(a)) => {
            for (i, s) in a.iter().enumerate() {
                if s.pointer("/@id").and_then(Value::as_str) == Some("money_player") {
                    v.push(format!("/savegame/statistics/stat/{i}/@value"));
                }
            }
        }
        Some(s) if s.pointer("/@id").and_then(Value::as_str) == Some("money_player") => v.push("/savegame/statistics/stat/@value".to_string()),
        _ => {}
    }
    v.retain(|p| cx.get(p).is_some());
    v
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let name = cx.text("/savegame/info/player/@name").unwrap_or_default();
    let sub = cx.text("/savegame/info/save/@name").unwrap_or_default();
    hero(ui, ACCENT, if name.is_empty() { "X4: Foundations" } else { &name }, &sub, |ui| {
        cx.tile(ui, "Credits (info line)", "/savegame/info/player/@money", ACCENT);
        if let Some(b) = cx.text("/savegame/info/game/@version") {
            fact(ui, "Game version", &b, ACCENT);
        }
    });
    let paths = money_paths(cx);
    if paths.is_empty() {
        return;
    }
    panel(ui, "Credits", |ui| {
        let now = cx.int(&paths[0]).unwrap_or(0);
        ui.label(format!("The save keeps the player's credits in {} places; these are the values as the game stores them.", paths.len()));
        let mut buf = ui.memory_mut(|m| *m.data.get_temp_mut_or_insert_with(ui.id().with("x4money"), || now));
        ui.horizontal(|ui| {
            ui.add(eframe::egui::DragValue::new(&mut buf).speed(1000.0).range(0..=i64::MAX / 4));
            if ui.button("Set credits everywhere").clicked() {
                for p in &paths {
                    cx.set(p, json!(buf));
                }
            }
        });
        ui.memory_mut(|m| m.data.insert_temp(ui.id().with("x4money"), buf));
        for p in paths.iter().skip(1) {
            ui.horizontal(|ui| {
                ui.label(p.trim_start_matches("/savegame/"));
                cx.editor(ui, p, 120.0);
            });
        }
        ui.weak("Everything else (ships, stations, wares) is in the Tree. Not yet tried on a save of the game itself.");
    });
}
