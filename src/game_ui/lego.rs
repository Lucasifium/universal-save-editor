//! LEGO Batman: Legacy of the Dark Knight (decrypted by `fmt::ttgames`): the studs wallet.
//! The game keeps studs in `StudsCollected` and in `Saved_Total` fields, so one box writes them all.

use super::{fact, hero, panel, GameCtx};
use crate::fmt::ttgames::find_all;
use eframe::egui::{Color32, Ui};
use serde_json::json;

const ACCENT: Color32 = Color32::from_rgb(230, 190, 40);
const MAX_STUDS: i64 = 9_999_999;

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let mut studs = find_all(cx.root, "StudsCollected");
    studs.extend(find_all(cx.root, "Saved_Total"));
    let secs = find_all(cx.root, "TotalPlaytime").first().and_then(|p| cx.int(p)).map(|t| t / 10_000_000);
    let sub = secs.map_or_else(String::new, |s| format!("{}:{:02} played", s / 3600, s / 60 % 60));
    hero(ui, ACCENT, "LEGO Batman: Legacy of the Dark Knight", &sub, |ui| {
        if let Some(p) = studs.first() {
            cx.tile(ui, "Studs", p, ACCENT);
        }
        let n = find_all(cx.root, "StudsCollected").len();
        fact(ui, "Wallet fields", &format!("{} in this save", studs.len().max(n)), ACCENT);
    });
    panel(ui, "Studs", |ui| {
        let total = studs.first().and_then(|p| cx.int(p)).unwrap_or(0);
        ui.label(format!("Studs now: {total}. The game keeps the wallet in {} places; the buttons write all of them.", studs.len()));
        ui.horizontal_wrapped(|ui| {
            if super::quick_button(ui, format!("Set studs to {MAX_STUDS}"), "") {
                for p in &studs {
                    cx.set(p, json!(MAX_STUDS));
                }
            }
            if ui.button("Copy the first value to every wallet field").clicked() {
                for p in studs.iter().skip(1) {
                    cx.set(p, json!(total));
                }
            }
        });
        ui.weak("Collectible counters (gold bricks, chips, tokens) count entries in the save, not their state, so they are in the Tree. Save slot files come with a BackupCopy file; edit both. Not yet tried in the game.");
    });
}
