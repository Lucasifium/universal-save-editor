//! NieR:Automata (`SlotData_N.dat`): money, experience, item and material counts and weapon
//! levels, with a "Max crafting materials" button for the Materials list.

use crate::i18n::tr;
use super::screens2::value_rows;
use super::{fact, fmt_num, hero, panel, tabs, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{Color32, Ui};
use serde_json::Value;

fn rows(cx: &GameCtx, base: &str) -> Vec<(String, String)> {
    match cx.get(base) {
        Some(Value::Object(m)) => m.keys().map(|k| (k.clone(), format!("{base}/{}", pointer_escape(k)))).collect(),
        _ => vec![],
    }
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 190, 160);
    let name = cx.text("/Save/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "NieR:Automata".into());
    let mut sub = vec!["NieR:Automata".to_string()];
    if let Some(c) = cx.int("/Save/Chapter") {
        sub.push(format!("chapter {c}"));
    }
    if let Some(t) = cx.text("/Save/Play time") {
        sub.push(format!("{t} played"));
    }
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Money (G)", "/Player/Money", accent);
        cx.tile(ui, "Experience", "/Player/Experience", accent);
        if let Some(xp) = cx.int("/Player/Experience") {
            fact(ui, "Level", &crate::fmt::nier::level_for(xp).to_string(), accent);
        }
    });
    let lists = ["Items", "Materials", "Weapons"];
    let t = tabs(ui, "nier_tabs", &lists.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let base = format!("/{}", lists[t]);
    let r = rows(cx, &base);
    panel(ui, lists[t], |ui| {
        if lists[t] == "Materials" {
            super::crafting::button(super::Game::Nier, ui, cx);
            ui.add_space(4.0);
        }
        if r.is_empty() {
            ui.weak(tr("None in this save."));
            return;
        }
        value_rows(ui, cx, &format!("nier_{t}"), &r);
        ui.weak(match lists[t] {
            "Weapons" => "Weapons you own and their level (1\u{2013}4); upgrading needs no materials once set here.",
            "Materials" => "Upgrade and selling materials; 99 is the most of one the game holds.",
            _ => "Consumables, key items and cosmetics you carry; 99 at most of each.",
        });
    });
    ui.weak(format!(
        "Level comes from experience ({} for level 99). The save has no checksum. Not yet tried in the game.",
        fmt_num(crate::fmt::nier::LEVEL_XP[98] as f64)
    ));
}
