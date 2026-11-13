//! Total War (Warhammer II, Troy) campaign saves: every faction's treasury, yours first.

use super::{fact, hero, panel, GameCtx};
use eframe::egui::{self, Color32, Ui};
use serde_json::{json, Value};

const ACCENT: Color32 = Color32::from_rgb(190, 140, 50);

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let player = cx.text("/Save/Player faction").unwrap_or_default();
    let build = cx.text("/Save/Build").unwrap_or_default();
    let Some(Value::Object(f)) = cx.get("/Factions") else { return };
    let keys: Vec<String> = f.keys().cloned().collect();
    let mine = keys.iter().find(|k| **k == player).cloned();
    hero(ui, ACCENT, "Total War campaign", &player, |ui| {
        if let Some(k) = &mine {
            cx.tile(ui, "Your treasury", &format!("/Factions/{}/Treasury", k.replace('~', "~0").replace('/', "~1")), ACCENT);
        }
        if !build.is_empty() {
            fact(ui, "Build", build.trim(), ACCENT);
        }
    });
    panel(ui, &format!("Factions ({})", keys.len()), |ui| {
        if let Some(k) = &mine {
            if super::quick_button(ui, "Set your treasury to 9,999,999", "") {
                cx.set(&format!("/Factions/{}/Treasury", k.replace('~', "~0").replace('/', "~1")), json!(9_999_999));
            }
        }
        egui::Grid::new("tw_factions").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
            for k in &keys {
                let p = format!("/Factions/{}/Treasury", k.replace('~', "~0").replace('/', "~1"));
                cx.grid_row(ui, k, &p, 120.0);
            }
        });
        ui.weak("Treasuries are whole numbers up to 2,147,483,647. The save is packed again on write (it gets larger). Not yet tried in the game.");
    });
}
