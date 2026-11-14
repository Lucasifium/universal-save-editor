//! Half-Life (GoldSrc) and Half-Life 2 (Source) saves: the player's HEV readout — health,
//! armour, suit power, ammo by type and the weapons carried with their magazines. Also
//! Black Mesa (Source) and Half-Life: Alyx (Source 2: health, resin, ammo, weapons).

use crate::i18n::{tr};
use crate::widgets::ScrollLock;
use super::{fact, hero, panel, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
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

/// The HUD colour of each game.
fn accent(game: &str) -> Color32 {
    match game {
        "Half-Life: Opposing Force" => Color32::from_rgb(70, 170, 60),
        "Half-Life: Blue Shift" => Color32::from_rgb(70, 140, 230),
        "Half-Life: Alyx" => Color32::from_rgb(40, 190, 200),
        g if g.starts_with("Half-Life 2") => Color32::from_rgb(235, 180, 40),
        _ => Color32::from_rgb(250, 150, 20),
    }
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let game = cx.text("/Save/Game").unwrap_or_else(|| "Half-Life".into());
    let accent = accent(&game);
    let alyx = game == "Half-Life: Alyx";
    let mut sub = vec![];
    if let Some(m) = cx.text("/Save/Map") {
        sub.push(crate::i18n::trf("Map {0}", &[&m]));
    }
    if let Some(v) = cx.text("/Save/Chapter") {
        sub.push(v);
    }
    if let Some(w) = cx.text("/Player/Weapon in hand") {
        sub.push(crate::i18n::trf("holding the {0}", &[&w]));
    }
    let missing: Vec<String> = match cx.get("/Not in this save") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => vec![],
    };
    hero(ui, accent, &game, &sub.join("  \u{00B7}  "), |ui| {
        for label in ["Health", "Armor", "Suit power", "Flashlight", "Max health", "Resin"] {
            let p = join("/Player", label);
            if cx.get(&p).is_some() {
                cx.tile(ui, label, &p, accent);
            } else if missing.iter().any(|m| m.starts_with(&format!("{label}:"))) {
                fact(ui, label, "0 \u{2014} not stored", accent);
            }
        }
        if let Some(s) = cx.text("/Player/HEV suit") {
            fact(ui, "HEV suit", &s, accent);
        }
        cx.tile(ui, "Difficulty", "/Save/Difficulty", accent);
    });
    if alyx {
        super::crafting::offer(super::Game::HalfLife2, ui, cx);
    }

    let ammo = keys(cx, "/Ammo");
    panel(ui, "Ammunition", |ui| {
        if ammo.is_empty() {
            ui.weak(tr("No ammunition in this save."));
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for a in &ammo {
                cx.tile(ui, a, &join("/Ammo", a), accent);
            }
        });
        if alyx {
            ui.weak(tr("Spare ammunition stored in the backpack, as the wrist display counts it."));
        } else {
            ui.weak(tr("Spare ammunition by type, as the HUD counts it. Names come from the weapons that use each type; \"Ammo type N\" is one no weapon in this map names. The game caps ammo at each type's maximum when you pick more up."));
        }
    });

    let weapons = keys(cx, "/Weapons");
    panel(ui, "Weapons", |ui| {
        if weapons.is_empty() {
            ui.weak(tr("The player carries no weapons yet."));
            return;
        }
        if alyx {
            ui.horizontal_wrapped(|ui| {
                for w in &weapons {
                    fact(ui, w, "carried", accent);
                }
            });
            ui.weak(tr("The weapons Alyx has. Their upgrades and loaded rounds aren't edited here."));
            return;
        }
        // Only Source weapons have a second magazine.
        let second = weapons.iter().any(|w| cx.get(&join(&join("/Weapons", w), "Second magazine")).is_some());
        let mags: &[&str] = if second { &["Magazine", "Second magazine"] } else { &["Magazine"] };
        egui::ScrollArea::horizontal().id_salt("hl_weapons_s").show_locked(ui, |ui| {
            egui::Grid::new("hl_weapons").num_columns(mags.len() + 2).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
                for h in std::iter::once("Weapon").chain(mags.iter().copied()).chain(["Ammo"]) {
                    ui.strong(h);
                }
                ui.end_row();
                for w in &weapons {
                    let base = join("/Weapons", w);
                    let held = cx.text(&join(&base, "In hand")).is_some();
                    let title = RichText::new(w).strong();
                    ui.horizontal(|ui| {
                        ui.label(if held { title.color(crate::theme::fg(ui, accent)) } else { title });
                        if held {
                            ui.label(RichText::new(tr("in hand")).small().color(ui.visuals().weak_text_color()));
                        }
                    })
                    .response
                    .on_hover_text(cx.text(&join(&base, "Entity")).unwrap_or_default());
                    for k in mags {
                        let p = join(&base, k);
                        match cx.get(&p) {
                            Some(Value::String(s)) => {
                                ui.weak(s.clone());
                            }
                            Some(_) => {
                                ui.horizontal(|ui| cx.cell(ui, &p, 64.0));
                            }
                            None => {
                                ui.label("");
                            }
                        }
                    }
                    ui.label(cx.text(&join(&base, "Uses")).unwrap_or_default());
                    ui.end_row();
                }
            });
        });
        ui.weak(tr("Rounds loaded in each weapon. An empty magazine isn't stored by the game, so it can't be filled here: fire it empty and reload, or give spare ammo above."));
    });

    if !missing.is_empty() {
        panel(ui, "Not in this save", |ui| {
            for m in &missing {
                ui.label(format!("\u{2022} {m}"));
            }
            ui.weak(tr("The game leaves out values that are zero, so there is nothing in the file to change for these. Pick up some armour (or ammo) in the game and save again."));
        });
    }
}
