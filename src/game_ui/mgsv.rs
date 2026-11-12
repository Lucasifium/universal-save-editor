//! Metal Gear Solid V: Ground Zeroes / The Phantom Pain (`TPP_GAME_DATA`): GMP, heroism,
//! Mother Base switches and the lifetime statistics.

use super::{hero, panel, GameCtx};
use eframe::egui::{Color32, Grid, Ui};
use serde_json::{json, Value};

const ACCENT: Color32 = Color32::from_rgb(214, 170, 60);
const GMP_MAX: i64 = 99_999_999;

fn names(cx: &GameCtx, group: &str) -> Vec<String> {
    match cx.get(&format!("/{group}")) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn esc(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}

fn grid(ui: &mut Ui, cx: &mut GameCtx, group: &str, id: &str) {
    Grid::new(id).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
        for n in names(cx, group) {
            ui.label(&n);
            cx.editor(ui, &format!("/{group}/{}", esc(&n)), 110.0);
            ui.end_row();
        }
    });
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    hero(ui, ACCENT, "Mother Base commander", "Metal Gear Solid V save", |ui| {
        cx.tile(ui, "GMP", "/Player/GMP", ACCENT);
        cx.tile(ui, "Heroism", "/Player/Heroism", ACCENT);
        cx.tile(ui, "Demon points", "/Player/Demon points", ACCENT);
    });
    if cx.get("/Player").is_some() {
        panel(ui, "Player", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Max GMP").clicked() {
                    cx.set("/Player/GMP", json!(GMP_MAX));
                    if cx.get("/Player/FOB wallet GMP").is_some() {
                        cx.set("/Player/FOB wallet GMP", json!(GMP_MAX));
                    }
                }
                if ui.button("Clear demon points").clicked() {
                    cx.set("/Player/Demon points", json!(0));
                }
            });
            grid(ui, cx, "Player", "mgsv_player");
            ui.weak("GMP is capped at 99,999,999 in the game. Close the game and turn Steam Cloud off for it before editing.");
        });
    }
    let flags: Vec<String> = names(cx, "Mother Base").into_iter().filter(|n| n.starts_with("mbmIsOpened") && n.ends_with("Section")).collect();
    if !flags.is_empty() || cx.get("/Mother Base").is_some() {
        panel(ui, "Mother Base", |ui| {
            if ui.button("Open every Mother Base section").clicked() {
                for n in &flags {
                    let p = format!("/Mother Base/{}", esc(n));
                    let on = if cx.get(&p).map_or(false, Value::is_boolean) { json!(true) } else { json!(1) };
                    cx.set(&p, on);
                }
            }
            ui.weak("The names are the game's own variables; hover over a section's flag to guess what it does, or leave the rest alone.");
            grid(ui, cx, "Mother Base", "mgsv_mb");
        });
    }
    if cx.get("/Statistics").is_some() {
        panel(ui, "Statistics", |ui| grid(ui, cx, "Statistics", "mgsv_stats"));
    }
    ui.weak("Every variable in the save, named by the game's own names, is in the Tree under Variables.");
}
