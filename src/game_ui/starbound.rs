//! Starbound characters (`.player`): pixels and other currencies, the item bags with
//! each stack's count, vitals, ship upgrades and the record. Items are shown by the
//! game's item name (or the name a generated weapon carries in the save).
//! Values are found by name and left out when a save doesn't have them.

use crate::i18n::{tr, trf};
use super::{columns, fact, fmt_num, hero, panel, tabs, words, GameCtx};
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

/// Friendly names for the bags and currencies the base game has.
fn label(key: &str) -> String {
    match key {
        "money" => "Pixels".into(),
        "essence" => "Ancient essence".into(),
        "mainBag" | "bag" => "Main bag".into(),
        "materialBag" | "tileBag" => "Materials".into(),
        "objectBag" => "Objects".into(),
        "foodBag" => "Food".into(),
        "reagentBag" => "Reagents".into(),
        "actionBar" => "Action bar".into(),
        "essentialBar" => "Essentials".into(),
        k => words(k),
    }
}

/// Where an item stack's fields are: 1.0 and later wrap them in `content`.
fn stack_base(cx: &GameCtx, slot: &str) -> Option<String> {
    let c = join(slot, "content");
    if cx.get(&join(&c, "name")).is_some() {
        Some(c)
    } else if cx.get(&join(slot, "name")).is_some() {
        Some(slot.to_string())
    } else {
        None
    }
}

/// An item's name: the one a generated item carries, else the game's item id.
fn item_name(cx: &GameCtx, base: &str) -> String {
    let id = cx.text(&join(base, "name")).unwrap_or_default();
    for params in ["parameters", "data"] {
        if let Some(s) = cx.text(&format!("{base}/{params}/shortdescription")).filter(|s| !s.is_empty()) {
            return format!("{} ({id})", strip_markup(&s));
        }
    }
    id
}

/// Drops Starbound's colour codes ("^orange;Name^reset;").
fn strip_markup(s: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for c in s.chars() {
        match c {
            '^' => skip = true,
            ';' if skip => skip = false,
            _ if !skip => out.push(c),
            _ => {}
        }
    }
    out
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    // Deep space blue and pixel gold.
    let accent = Color32::from_rgb(70, 96, 186);
    let gold = Color32::from_rgb(214, 170, 60);
    let name = cx.text("/identity/name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Starbound character".into());
    let mut sub = vec![];
    for p in ["/identity/species", "/identity/gender"] {
        if let Some(s) = cx.text(p).filter(|s| !s.is_empty()) {
            sub.push(words(&s));
        }
    }
    if let Some(m) = cx.text("/modeType").filter(|s| !s.is_empty()) {
        sub.push(format!("{} mode", words(&m)));
    }
    if let Some(t) = cx.num("/log/playTime") {
        let mins = (t / 60.0) as i64;
        sub.push(format!("{}h {:02}m played", mins / 60, mins % 60));
    }
    let money = ["/inventory/currencies/money", "/inventory/money"].into_iter().find(|p| cx.get(p).is_some());
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        match money {
            Some(p) => cx.tile(ui, "Pixels", p, gold),
            None => fact(ui, "Pixels", "\u{2014}", gold),
        }
        cx.tile(ui, "Ancient essence", "/inventory/currencies/essence", gold);
        cx.tile(ui, "Ship level", "/shipUpgrades/shipLevel", accent);
        cx.tile(ui, "Deaths", "/log/deathCount", accent);
    });

    let names: Vec<String> = ["Inventory", "Character & ship", "Currencies", "Techs & progress"].map(String::from).to_vec();
    match tabs(ui, "sb_tab", &names) {
        0 => inventory(ui, cx),
        1 => character(ui, cx),
        2 => currencies(ui, cx),
        _ => progress(ui, cx),
    }
}

fn inventory(ui: &mut Ui, cx: &mut GameCtx) {
    // 1.0 and later: named bags under itemBags; early-access saves: lists on the inventory.
    let mut bags: Vec<String> = keys(cx, "/inventory/itemBags").into_iter().map(|k| join("/inventory/itemBags", &k)).collect();
    if bags.is_empty() {
        for k in ["bag", "tileBag", "actionBar", "essentialBar", "equipment", "wieldable"] {
            if matches!(cx.get(&join("/inventory", k)), Some(Value::Array(_))) {
                bags.push(join("/inventory", k));
            }
        }
    }
    let worn: Vec<(String, String)> = keys(cx, "/inventory")
        .into_iter()
        .filter(|k| k.ends_with("Slot") || matches!(k.as_str(), "beamAxe" | "wireTool" | "paintTool" | "inspectionTool"))
        .filter_map(|k| {
            let p = join("/inventory", &k);
            stack_base(cx, &p).map(|b| (words(&k), item_name(cx, &b)))
        })
        .collect();
    if !worn.is_empty() {
        panel(ui, "Equipped", |ui| {
            columns(ui, 300.0, worn.len(), |ui, i| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&worn[i].0).color(ui.visuals().weak_text_color()));
                    ui.label(&worn[i].1);
                });
            });
        });
    }
    if bags.is_empty() {
        panel(ui, "Bags", |ui| {
            ui.weak(tr("No item bags in this save."));
        });
        return;
    }
    let labels: Vec<String> = bags
        .iter()
        .map(|b| {
            let key = b.rsplit('/').next().unwrap_or(b).replace("~1", "/").replace("~0", "~");
            let used = match cx.get(b) {
                Some(Value::Array(a)) => a.iter().filter(|v| !v.is_null()).count(),
                _ => 0,
            };
            format!("{} ({used})", label(&key))
        })
        .collect();
    let bag = bags[tabs(ui, "sb_bag", &labels)].clone();
    panel(ui, "Items", |ui| {
        let q = filter(ui);
        let n = match cx.get(&bag) {
            Some(Value::Array(a)) => a.len(),
            _ => 0,
        };
        let mut shown = 0;
        egui::Grid::new(("sb_items", &bag)).num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.strong(tr("Slot"));
            ui.strong(tr("Item"));
            ui.strong(tr("Count"));
            ui.end_row();
            for i in 0..n {
                let slot = format!("{bag}/{i}");
                let Some(base) = stack_base(cx, &slot) else { continue };
                let title = item_name(cx, &base);
                if !q.is_empty() && !title.to_lowercase().contains(&q) {
                    continue;
                }
                shown += 1;
                ui.label(RichText::new(format!("{}", i + 1)).color(ui.visuals().weak_text_color()));
                let count = join(&base, "count");
                let c = if cx.modified(&count) { crate::theme::MODIFIED } else { ui.visuals().text_color() };
                ui.label(RichText::new(title).color(c));
                cx.cell(ui, &count, 80.0);
                ui.end_row();
            }
        });
        if shown == 0 {
            ui.weak(if q.is_empty() { "This bag is empty." } else { "No item matches." });
        }
        ui.add_space(4.0);
        ui.weak(tr("Each item has its own largest stack in the game (1000 for most materials); the game may cut a bigger count when it loads."));
        super::crafting::offer(super::Game::Starbound, ui, cx);
    });
}

fn filter(ui: &mut Ui) -> String {
    let sid = egui::Id::new("sb_filter");
    let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(tr("Filter:"));
        ui.add(egui::TextEdit::singleline(&mut q).hint_text(tr("item name")).desired_width(220.0));
    });
    ui.data_mut(|d| d.insert_temp(sid, q.clone()));
    q.to_lowercase()
}

fn character(ui: &mut Ui, cx: &mut GameCtx) {
    columns(ui, 340.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Vitals", |ui| {
                let base = "/statusController/resourceValues";
                if cx.get(base).is_none() {
                    ui.weak(tr("This save keeps no vitals."));
                    return;
                }
                egui::Grid::new("sb_vitals").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    for (k, l) in [("health", "Health"), ("energy", "Energy"), ("food", "Food"), ("breath", "Breath")] {
                        cx.grid_row(ui, l, &join(base, k), 100.0);
                    }
                });
                ui.weak(tr("These are the current amounts; the most a character can have comes from its armour and food, and the game keeps the current amount within it."));
            });
        } else {
            panel(ui, "Ship", |ui| {
                let base = "/shipUpgrades";
                if cx.get(base).is_none() {
                    ui.weak(tr("This save has no ship upgrades."));
                    return;
                }
                egui::Grid::new("sb_ship").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    for (k, l) in [("shipLevel", "Ship level"), ("crewSize", "Crew size"), ("maxFuel", "Fuel tank"), ("fuelEfficiency", "Fuel efficiency"), ("shipSpeed", "Ship speed")] {
                        cx.grid_row(ui, l, &join(base, k), 100.0);
                    }
                });
                if let Some(Value::Array(c)) = cx.get("/shipUpgrades/capabilities") {
                    let c: Vec<String> = c.iter().filter_map(Value::as_str).map(words).collect();
                    if !c.is_empty() {
                        ui.label(trf("Can: {0}", &[&(c.join(", "))]));
                    }
                }
            });
        }
    });
    panel(ui, "Record", |ui| {
        egui::Grid::new("sb_log").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
            cx.grid_row(ui, "Deaths", "/log/deathCount", 100.0);
            cx.grid_row(ui, "Play time (seconds)", "/log/playTime", 140.0);
            cx.grid_row(ui, "Intro done", "/log/introComplete", 100.0);
        });
        for (p, what) in [("/log/scannedObjects", "objects scanned"), ("/log/cinematics", "cinematics seen"), ("/log/radioMessages", "radio messages heard")] {
            if let Some(Value::Array(a)) = cx.get(p) {
                ui.label(format!("{} {what}", fmt_num(a.len() as f64)));
            }
        }
    });
}

fn currencies(ui: &mut Ui, cx: &mut GameCtx) {
    panel(ui, "Currencies", |ui| {
        let ks = keys(cx, "/inventory/currencies");
        if ks.is_empty() {
            if cx.get("/inventory/money").is_some() {
                egui::Grid::new("sb_cur_old").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    cx.grid_row(ui, "Pixels", "/inventory/money", 120.0);
                });
            } else {
                ui.weak(tr("No currencies in this save."));
            }
            return;
        }
        egui::Grid::new("sb_cur").num_columns(2).spacing([14.0, 4.0]).striped(true).show(ui, |ui| {
            for k in &ks {
                cx.grid_row(ui, &label(k), &join("/inventory/currencies", k), 120.0);
            }
        });
        ui.weak(tr("Each currency has a most the game allows; a larger amount may be cut when it loads. Currencies past pixels and ancient essence come from mods."));
    });
}

fn progress(ui: &mut Ui, cx: &mut GameCtx) {
    let list = |cx: &GameCtx, p: &str| -> Vec<String> {
        match cx.get(p) {
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).map(str::to_string).collect(),
            _ => vec![],
        }
    };
    columns(ui, 340.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Techs", |ui| {
                let avail = list(cx, "/techs/availableTechs");
                let enabled = list(cx, "/techs/enabledTechs");
                if avail.is_empty() {
                    ui.weak(tr("No techs yet."));
                }
                for t in &avail {
                    let on = enabled.contains(t);
                    ui.label(if on { RichText::new(format!("{t}  (enabled)")).strong() } else { RichText::new(t) });
                }
                for k in keys(cx, "/techs/equippedTechs") {
                    if let Some(t) = cx.text(&join("/techs/equippedTechs", &k)) {
                        ui.label(format!("{k}: {t}"));
                    }
                }
            });
        } else {
            panel(ui, "Discoveries", |ui| {
                let known = match cx.get("/blueprints/knownBlueprints") {
                    Some(Value::Array(a)) => a.len(),
                    _ => 0,
                };
                ui.label(trf("{0} blueprints known", &[&(fmt_num(known as f64))]));
                ui.label(trf("{0} codex entries", &[&(fmt_num(keys(cx, "/codexes").len() as f64))]));
                let quests = keys(cx, "/quests/quests");
                let done = quests.iter().filter(|q| cx.text(&format!("/quests/quests/{}/content/state", pointer_escape(q))).is_some_and(|s| s == "Complete")).count();
                ui.label(trf("{0} quests, {1} complete", &[&(quests.len()), &done]));
                if let Some(Value::Array(m)) = cx.get("/aiState/completedMissions") {
                    ui.label(trf("{0} missions done", &[&(m.len())]));
                }
            });
        }
    });
}
