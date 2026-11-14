//! Final Fantasy XVI (slot pictures `0001.png`…, `auto.png`): gil, Ability Points, consumables
//! and materials, with a "Max crafting materials" button.

use crate::i18n::tr;
use super::screens2::value_rows;
use super::{fact, hero, panel, tabs, GameCtx};
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
    let accent = Color32::from_rgb(196, 64, 52);
    let level = cx.int("/Clive/Level");
    let sub = match level {
        Some(l) => format!("Final Fantasy XVI  \u{00B7}  level {l}"),
        None => "Final Fantasy XVI".into(),
    };
    hero(ui, accent, "Clive", &sub, |ui| {
        cx.tile(ui, "Gil", "/Clive/Gil", accent);
        cx.tile(ui, "Ability Points", "/Clive/Ability Points", accent);
        if let Some(hp) = cx.int("/Clive/Max HP") {
            fact(ui, "Max HP", &hp.to_string(), accent);
        }
    });
    let lists = ["Materials", "Consumables"];
    let t = tabs(ui, "ff16_tabs", &lists.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let base = format!("/{}", lists[t]);
    let r = rows(cx, &base);
    panel(ui, lists[t], |ui| {
        if lists[t] == "Materials" {
            super::crafting::button(super::Game::Ff16, ui, cx);
            ui.add_space(4.0);
        }
        if r.is_empty() {
            ui.weak(tr("None in this save."));
            return;
        }
        value_rows(ui, cx, &format!("ff16_{t}"), &r);
        ui.weak(match lists[t] {
            "Materials" => "Materials you have picked up at least once. Most hold 99; Desert Rose, Valley Madder, Briar \
                            Clam Shell, Clutchmine and Iron Filings hold 999, and Wyrrite, Sharp Fang, Steelsilk, \
                            Bloody Hide, Magicked Ash and Rhema hold 9999.",
            _ => "Potions and tonics in your satchel, up to the most the game shows for each with the satchel fully expanded.",
        });
    });
    ui.weak(
        "Level and HP come from experience and gear, so they are shown but not edited. Materials you have never \
         picked up aren't in the save yet. The save's checksum is written again when you save. Tried in the game.",
    );
}
