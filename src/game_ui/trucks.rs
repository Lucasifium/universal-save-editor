//! Euro Truck Simulator 2 and American Truck Simulator (`game.sii`): money, experience,
//! skills and the trucks' fuel and wear.

use super::{fact, hero, panel, GameCtx};
use eframe::egui::{Color32, Ui};
use serde_json::{json, Value};

const ACCENT: Color32 = Color32::from_rgb(232, 142, 36);

const SKILLS: [(&str, &str, f64); 5] =
    [("Long distance", "long_dist", 6.0), ("Heavy cargo", "heavy", 6.0), ("Fragile cargo", "fragile", 6.0), ("Urgent delivery", "urgent", 6.0), ("Mechanical", "mechanical", 6.0)];

const WEAR: [&str; 8] =
    ["engine_wear", "transmission_wear", "cabin_wear", "chassis_wear", "engine_wear_unfixable", "transmission_wear_unfixable", "cabin_wear_unfixable", "chassis_wear_unfixable"];

fn esc(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}

/// Units whose type is `ty` ("vehicle : _nameless.…").
fn units(cx: &GameCtx, ty: &str) -> Vec<String> {
    let Some(Value::Object(m)) = cx.get("") else { return vec![] };
    m.keys().filter(|k| k.as_str() == ty || k.starts_with(&format!("{ty} : "))).map(|k| format!("/{}", esc(k))).collect()
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let sub = cx.text("/driver_player/experience_points").map_or_else(String::new, |x| format!("{x} XP"));
    hero(ui, ACCENT, "Truck simulator profile", &sub, |ui| {
        cx.tile(ui, "Money", "/bank/money_account", ACCENT);
        cx.tile(ui, "Experience", "/driver_player/experience_points", ACCENT);
        cx.tile(ui, "Loan limit", "/bank/loan_limit", ACCENT);
    });
    if cx.get("/driver_player").is_some() {
        panel(ui, "Skills", |ui| {
            for (name, key, max) in SKILLS {
                cx.attribute(ui, name, &format!("/driver_player/{key}"), max, ACCENT);
            }
            ui.horizontal(|ui| {
                cx.label(ui, "ADR classes (bitmask 0\u{2013}63)", "/driver_player/adr");
                cx.editor(ui, "/driver_player/adr", 60.0);
            });
            if super::quick_button(ui, "Max all skills", "") {
                for (_, key, max) in SKILLS {
                    cx.set(&format!("/driver_player/{key}"), json!(max as i64));
                }
                cx.set("/driver_player/adr", json!(63));
            }
            ui.weak("Skills go from 0 to 6; ADR is the six hazardous-goods classes as bits (63 = all). Experience sets the driver level; the game works it out on load.");
        });
    }
    let trucks = units(cx, "vehicle");
    if !trucks.is_empty() {
        panel(ui, &format!("Vehicles ({})", trucks.len()), |ui| {
            for t in &trucks {
                ui.horizontal_wrapped(|ui| {
                    let plate = cx.text(&format!("{t}/license_plate")).unwrap_or_default();
                    fact(ui, "Plate", &plate, ACCENT);
                    for (cap, key) in [("Odometer (km)", "odometer"), ("Fuel (0\u{2013}1)", "fuel_relative")] {
                        let p = format!("{t}/{key}");
                        if cx.get(&p).is_some() {
                            cx.label(ui, cap, &p);
                            cx.editor(ui, &p, 80.0);
                        }
                    }
                });
            }
            ui.horizontal_wrapped(|ui| {
                if super::quick_button(ui, "Fill every tank", "") {
                    for t in &trucks {
                        let p = format!("{t}/fuel_relative");
                        if cx.get(&p).is_some() {
                            cx.set(&p, json!(1.0));
                        }
                    }
                }
                if super::quick_button(ui, "Repair every vehicle", "") {
                    for t in units(cx, "vehicle").into_iter().chain(units(cx, "trailer")) {
                        for w in WEAR.iter().chain(&["body_wear", "body_wear_unfixable", "wheels_wear"]) {
                            let p = format!("{t}/{w}");
                            if cx.get(&p).is_some() {
                                cx.set(&p, json!(0.0));
                            }
                        }
                    }
                }
            });
            ui.weak("Vehicle units are named by their id in the Tree. Wear and fuel are fractions (0 = new, 1 = worn out; fuel 1 = full).");
        });
    }
    ui.weak("Packed saves are opened and packed again for you. In the game's config.cfg, g_save_format 2 makes it write plain text saves.");
}
