//! Terraria player: life and mana, permanent upgrades, and the inventory, equipment
//! and storage as item grids.

use super::{cell_grid, hero, meter, panel, select, selection, tabs, Cell, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};

const ACCENT: Color32 = Color32::from_rgb(90, 170, 90);
const EMPTY: &str = "(empty)";

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let name = cx.text("/Character/Name").unwrap_or_else(|| "Terrarian".into());
    let mut sub = vec![];
    if let Some(d) = cx.text("/Character/Difficulty") {
        sub.push(format!("{d} character"));
    }
    if let Some(h) = cx.num("/Character/Play time (hours)") {
        sub.push(format!("{h:.1} hours played"));
    }
    hero(ui, ACCENT, &name, &sub.join("  \u{00B7}  "), |ui| {
        for (cap, p) in [("Max life", "/Character/Max life"), ("Max mana", "/Character/Max mana"), ("Life", "/Character/Life"), ("Mana", "/Character/Mana")] {
            cx.tile(ui, cap, p, ACCENT);
        }
    });

    let wide = ui.available_width() > 760.0;
    let left = |ui: &mut Ui, cx: &mut GameCtx| {
        panel(ui, "Health", |ui| {
            if let (Some(l), Some(m)) = (cx.num("/Character/Life"), cx.num("/Character/Max life")) {
                meter(ui, "Life", l, m.max(l), Color32::from_rgb(200, 60, 70));
            }
            if let (Some(l), Some(m)) = (cx.num("/Character/Mana"), cx.num("/Character/Max mana")) {
                meter(ui, "Mana", l, m.max(l), Color32::from_rgb(70, 110, 220));
            }
            ui.weak("Max life goes up to 500 with Life Crystals and Life Fruit (more with buffs); max mana to 200.");
            let deaths: Vec<String> = [("Deaths (enemies)", "by enemies"), ("Deaths (players)", "by players")]
                .iter()
                .filter_map(|(k, what)| cx.int(&format!("/Character/{k}")).map(|n| format!("{n} {what}")))
                .collect();
            if !deaths.is_empty() {
                ui.weak(format!("Deaths: {}", deaths.join(", ")));
            }
        });
        if let Some(serde_json::Value::Object(m)) = cx.get("/Permanent upgrades").cloned() {
            panel(ui, "Permanent upgrades", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for k in m.keys() {
                        cx.chip(ui, k, &format!("/Permanent upgrades/{}", pointer_escape(k)), ACCENT);
                    }
                });
            });
        }
    };
    let right = |ui: &mut Ui, cx: &mut GameCtx| {
        panel(ui, "Equipment", |ui| {
            let groups: [(&str, Vec<String>); 4] = [
                ("Armor & accessories", ["Helmet", "Chestplate", "Leggings"].iter().map(|s| s.to_string()).chain((1..=7).map(|i| format!("Accessory {i}"))).collect()),
                ("Vanity", ["Vanity Helmet", "Vanity Chestplate", "Vanity Leggings"].iter().map(|s| s.to_string()).chain((1..=7).map(|i| format!("Vanity Accessory {i}"))).collect()),
                ("Dyes", ["Dye Helmet", "Dye Chestplate", "Dye Leggings"].iter().map(|s| s.to_string()).chain((1..=7).map(|i| format!("Dye Accessory {i}"))).collect()),
                ("Pets, mounts & hook", ["Pet", "Pet dye", "Light pet", "Light pet dye", "Minecart", "Minecart dye", "Mount", "Mount dye", "Hook", "Hook dye"].iter().map(|s| s.to_string()).collect()),
            ];
            let names: Vec<String> = groups.iter().map(|g| g.0.to_string()).collect();
            let t = tabs(ui, "terraria_equip_tab", &names);
            slots(ui, cx, &format!("terraria_equip_{t}"), "/Equipment", &groups[t].1, 10);
        });
    };
    if wide {
        ui.columns(2, |c| {
            left(&mut c[0], cx);
            right(&mut c[1], cx);
        });
    } else {
        left(ui, cx);
        right(ui, cx);
    }

    panel(ui, "Inventory", |ui| {
        let inv: Vec<String> = (1..=50).map(|i| format!("Slot {i}")).collect();
        slots(ui, cx, "terraria_inv", "/Inventory", &inv, 10);
        ui.add_space(6.0);
        ui.label(RichText::new("Coins and ammo").strong());
        let ca: Vec<String> = (1..=4).map(|i| format!("Coin slot {i}")).chain((1..=4).map(|i| format!("Ammo slot {i}"))).collect();
        slots(ui, cx, "terraria_coins", "/Coins and ammo", &ca, 8);
    });

    let stores: Vec<String> = ["Piggy bank", "Safe", "Defender's Forge", "Void Vault"].iter().filter(|s| cx.get(&format!("/{}", pointer_escape(s))).is_some()).map(|s| s.to_string()).collect();
    if !stores.is_empty() {
        panel(ui, "Storage", |ui| {
            let t = tabs(ui, "terraria_store_tab", &stores);
            let keys: Vec<String> = (1..=40).map(|i| format!("Slot {i}")).collect();
            slots(ui, cx, &format!("terraria_store_{t}"), &format!("/{}", pointer_escape(&stores[t])), &keys, 10);
        });
    }
}

/// A grid of item slots under `base`, with the selected slot's editor below it.
pub(super) fn slots(ui: &mut Ui, cx: &mut GameCtx, id: &str, base: &str, keys: &[String], cols: usize) {
    let present: Vec<&String> = keys.iter().filter(|k| cx.get(&format!("{base}/{}", pointer_escape(k))).is_some()).collect();
    let cells: Vec<Cell> = present
        .iter()
        .map(|k| {
            let p = format!("{base}/{}", pointer_escape(k));
            let item = cx.text(&format!("{p}/Item")).unwrap_or_default();
            let empty = item.is_empty() || item == EMPTY;
            let prefix = cx.text(&format!("{p}/Prefix")).filter(|x| x != "(none)" && !x.is_empty());
            let title = if empty { String::new() } else { prefix.map_or(item.clone(), |px| format!("{px} {item}")) };
            Cell {
                title,
                count: cx.int(&format!("{p}/Stack")),
                empty,
                modified: ["Item", "Stack", "Prefix", "Favorite"].iter().any(|f| cx.modified(&format!("{p}/{f}"))),
                tint: cx.flag(&format!("{p}/Favorite")).then(|| Color32::from_rgb(70, 64, 30)),
            }
        })
        .collect();
    let sel = selection(ui, id);
    if let Some(i) = cell_grid(ui, &cells, cols, sel, ACCENT) {
        select(ui, id, if sel == Some(i) { None } else { Some(i) });
    }
    let Some(i) = selection(ui, id).filter(|&i| i < present.len()) else {
        ui.weak("Click a slot to change its item, stack size or modifier.");
        return;
    };
    let p = format!("{base}/{}", pointer_escape(present[i]));
    egui::Frame::none().fill(super::inset_fill(ui, ACCENT)).rounding(8.0).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(present[i].as_str()).strong());
            if ui.small_button("Close").clicked() {
                select(ui, id, None);
            }
        });
        egui::Grid::new((id, "slot")).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            cx.grid_row(ui, "Item", &format!("{p}/Item"), 240.0);
            cx.grid_row(ui, "Stack", &format!("{p}/Stack"), 100.0);
            cx.grid_row(ui, "Modifier", &format!("{p}/Prefix"), 200.0);
            cx.grid_row(ui, "Favorite", &format!("{p}/Favorite"), 100.0);
        });
    });
}
