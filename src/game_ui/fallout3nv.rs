//! Fallout 3 and Fallout: New Vegas (`.fos`): the character's bottle caps, experience,
//! karma and S.P.E.C.I.A.L. Values the save doesn't have (some saves lack a readable
//! inventory or base form) are left out.

use super::{columns, fact, hero, panel, GameCtx};
use eframe::egui::{self, Color32, Ui};

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(120, 160, 90);
    let game = cx.text("/Character/Game").unwrap_or_else(|| "Fallout".into());
    let mut sub = vec![game];
    for (k, f) in [("Place", "{}"), ("Play time", "{} played")] {
        if let Some(v) = cx.text(&format!("/Character/{k}")).filter(|v| !v.is_empty()) {
            sub.push(f.replace("{}", &v));
        }
    }
    let name = cx.text("/Character/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Character".into());
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Bottle caps", "/Character/Bottle caps", accent);
        cx.tile(ui, "Experience", "/Character/Experience", accent);
        cx.tile(ui, "Karma", "/Character/Karma", accent);
        if let Some(l) = cx.int("/Character/Level") {
            fact(ui, "Level", &l.to_string(), accent);
        }
    });
    columns(ui, 340.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "S.P.E.C.I.A.L.", |ui| {
                egui::Grid::new("fo_special").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    for n in ["Strength", "Perception", "Endurance", "Charisma", "Intelligence", "Agility", "Luck"] {
                        cx.grid_row(ui, n, &format!("/S.P.E.C.I.A.L./{n}"), 70.0);
                    }
                });
                ui.weak("The perk and trait bonuses on top come from the game.");
            });
        } else {
            panel(ui, "About the numbers", |ui| {
                ui.weak("Experience sets the level the game works out; the level shown is from the save's header and catches up when you next gain experience. Karma runs from \u{2212}1000 to 1000.");
            });
        }
    });
}
