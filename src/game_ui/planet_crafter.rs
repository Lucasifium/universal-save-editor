//! The Planet Crafter: Terra tokens, terraformation levels, your gauges, the game's
//! settings and the raw materials in the backpack.

use crate::i18n::{tr};
use super::{fact, hero, panel, GameCtx};
use crate::fmt::planet_crafter::{free_slots, MATERIALS};
use eframe::egui::{Color32, Ui};
use serde_json::Value;

const ACCENT: Color32 = Color32::from_rgb(224, 120, 60);

pub const ABOUT: &str = "Adds your free backpack slots to the raw materials (iron, cobalt, silicon, titanium, magnesium, aluminium, \
iridium, super alloy, zeolite, sulfur, osmium, uranim, obsidian, selenium, phosphorus, dolomite, bauxite, uraninite, ice), \
shared equally. The game has no stack size: every item takes one backpack slot, so the free slots are the real limit. \
Seeds, food, fuses, tokens, key cards and chips are left alone.";

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let sub = cx.text("/Player/On planet").map(|p| crate::i18n::trf("{0} on {1}", &[&cx.text("/Player/Name").unwrap_or_default(), &p])).unwrap_or_default();
    hero(ui, ACCENT, "The Planet Crafter", &sub, |ui| {
        cx.tile(ui, "Terra tokens", "/Progress/Terra tokens", ACCENT);
        cx.tile(ui, "Earned in all", "/Progress/Terra tokens earned in all", ACCENT);
        cx.tile(ui, "Objects crafted", "/Stats/Objects crafted", ACCENT);
        if let Some(n) = cx.int("/Progress/Blueprints unlocked") {
            fact(ui, "Blueprints unlocked", &n.to_string(), ACCENT);
        }
    });
    let planets: Vec<String> = match cx.get("") {
        Some(Value::Object(m)) => m.keys().filter(|k| k.starts_with("Planet ")).cloned().collect(),
        _ => vec![],
    };
    for pl in planets {
        panel(ui, &crate::i18n::trf("Terraformation: {0}", &[&pl.trim_start_matches("Planet ")]), |ui| {
            for k in ["Oxygen", "Heat", "Pressure", "Plants", "Insects", "Animals"] {
                cx.row_fixed(ui, k, &format!("/{pl}/{k}"), 120.0, 190.0);
            }
            ui.weak(tr("Terraformation units; the planet's stage follows them when the save loads."));
        });
    }
    panel(ui, "You", |ui| {
        for k in ["Oxygen", "Thirst", "Health"] {
            cx.row_fixed(ui, k, &format!("/Player/{k}"), 120.0, 90.0);
        }
    });
    panel(ui, "Game settings", |ui| {
        for k in [
            "Free crafting", "Everything unlocked", "Space trading unlocked", "Ore extractors unlocked", "Teleporters unlocked",
            "Drones unlocked", "Autocrafter unlocked",
        ] {
            let p = format!("/Settings/{k}");
            if cx.get(&p).is_some() {
                cx.row_fixed(ui, k, &p, 200.0, 60.0);
            }
        }
        for k in ["Mode", "When you die", "Start location"] {
            if let Some(v) = cx.text(&format!("/Settings/{k}")) {
                fact(ui, k, &v, ACCENT);
            }
        }
    });
    panel(ui, "Backpack materials", |ui| {
        let free = cx.get("").map_or(0, free_slots);
        ui.weak(crate::i18n::trf(
            "{0} slots, {1} carried, {2} free. Raising a count adds items to the backpack; lowering it removes some.",
            &[&cx.int("/Backpack/Slots").unwrap_or(0), &cx.int("/Backpack/Items carried").unwrap_or(0), &free],
        ));
        for (_, name) in MATERIALS {
            cx.row_fixed(ui, name, &format!("/Backpack/{name}"), 120.0, 80.0);
        }
        let enabled = free >= MATERIALS.len() as i64 && MATERIALS.iter().all(|(_, n)| cx.can_edit(&format!("/Backpack/{n}")));
        if ui.add_enabled(enabled, eframe::egui::Button::new(tr("Max crafting materials"))).on_hover_text(ABOUT).clicked() {
            let share = free / MATERIALS.len() as i64;
            for (_, name) in MATERIALS {
                let p = format!("/Backpack/{name}");
                let have = cx.int(&p).unwrap_or(0);
                cx.set(&p, Value::from(have + share));
            }
        }
        if !enabled {
            ui.weak(tr("Free at least one backpack slot per material to use the button."));
        }
    });
}
