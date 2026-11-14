//! Neverwinter Nights: the player of a module save (`<Module>.sav`) or a character file
//! (`.bic`). Values come from `fmt::nwn`.

use crate::i18n::{tr};
use crate::widgets::ScrollLock;
use super::{columns, hero, panel, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

/// Old gold and oxblood, like a tavern's sign.
const GOLD: Color32 = Color32::from_rgb(196, 150, 60);
const BLOOD: Color32 = Color32::from_rgb(150, 45, 40);

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let c = "/Character";
    let name = cx.text("/Character/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Adventurer".into());
    let mut sub = vec![];
    for p in ["/Character/Race", "/Character/Classes"] {
        if let Some(t) = cx.text(p) {
            sub.push(t);
        }
    }
    if let Some(l) = cx.int("/Character/Level") {
        sub.push(format!("Level {l}"));
    }
    for p in ["/Save/Area", "/Save/Module"] {
        if let Some(t) = cx.text(p).filter(|t| !t.is_empty()) {
            sub.push(t);
            break;
        }
    }
    hero(ui, GOLD, &name, &sub.join("  \u{00B7}  "), |ui| {
        for (label, k) in [("Gold", "Gold"), ("Experience", "Experience"), ("Skill points", "Skill points")] {
            cx.tile(ui, label, &join(c, k), GOLD);
        }
    });
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Ability scores", |ui| {
                for k in ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"] {
                    cx.attribute(ui, k, &join("/Character/Attributes", k), 30.0, BLOOD);
                }
            });
        } else {
            panel(ui, "Hit points and alignment", |ui| {
                egui::Grid::new("nwn_hp").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    for k in ["Current HP", "Max HP", "Base HP"] {
                        cx.grid_row(ui, k, &join("/Character/Health", k), 80.0);
                    }
                });
                ui.weak(tr("Max HP includes bonuses; base HP is before them."));
                ui.add_space(6.0);
                cx.attribute(ui, "Good\u{2013}evil", "/Character/Alignment/Good\u{2013}evil", 100.0, GOLD);
                cx.attribute(ui, "Law\u{2013}chaos", "/Character/Alignment/Law\u{2013}chaos", 100.0, GOLD);
                ui.weak(tr("100 is good / lawful, 0 evil / chaotic."));
            });
        }
    });
    let sk = "/Character/Skills";
    let names: Vec<String> = match cx.get(sk) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !names.is_empty() {
        panel(ui, "Skills", |ui| {
            columns(ui, 300.0, names.len(), |ui, i| {
                cx.attribute(ui, &names[i], &join(sk, &names[i]), 40.0, GOLD);
            });
            ui.weak(tr("Ranks bought with skill points (ability bonuses and items add to them in the game)."));
        });
    }
    let items: Vec<String> = match cx.get("/Inventory") {
        Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => vec![],
    };
    if !items.is_empty() {
        egui::CollapsingHeader::new(RichText::new(format!("Backpack ({} kinds)", items.len())).strong()).id_salt("nwn_inv").show(ui, |ui| {
        let stacks: Vec<String> = match cx.get("/Items") {
            Some(Value::Object(m)) => m.keys().cloned().collect(),
            _ => vec![],
        };
        if stacks.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for it in &items {
                    ui.label(RichText::new(it).monospace().size(12.0));
                    ui.add_space(8.0);
                }
            });
        } else {
            egui::ScrollArea::vertical().id_salt("nwn_inv_s").max_height(300.0).show_locked(ui, |ui| {
                super::columns(ui, 280.0, stacks.len(), |ui, i| {
                    cx.row_fixed(ui, &stacks[i], &format!("/Items/{}", crate::json_view::pointer_escape(&stacks[i])), 180.0, 56.0);
                });
            });
        }
        ui.weak(tr("How many of each item is in the stack, by the item's tag. The game may cut a stack back to what the item allows; items can't be added or removed here."));
        });
    }
    ui.add_space(6.0);
    if cx.text("/Save/Area").is_some() || cx.text("/Save/Module").is_some() {
        ui.weak(tr("Saving also updates player.bic in the same folder, the copy the game shows on the load screen."));
    } else {
        ui.weak(tr("Servers with character checks (ELC/ILR) may refuse a character edited past the rules."));
    }
}
