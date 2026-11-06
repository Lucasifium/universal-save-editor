//! Wasteland 3 (the party of rangers, money and inventory) and Valheim characters
//! (vitals, skills and inventory). Values are found by name and left out when a save
//! doesn't have them.

use super::{columns, fact, fmt_num, hero, panel, tabs, GameCtx};
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

/// A filter box remembered under `id`; returns the lowercase query.
fn filter(ui: &mut Ui, id: &str, hint: &str) -> String {
    let sid = egui::Id::new(("survival_filter", id));
    let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add(egui::TextEdit::singleline(&mut q).hint_text(hint).desired_width(220.0));
    });
    ui.data_mut(|d| d.insert_temp(sid, q.clone()));
    q.to_lowercase()
}

/// "Name  [quantity]" rows for every entry of the object at `base`, in columns.
fn amount_rows(ui: &mut Ui, cx: &mut GameCtx, id: &str, base: &str, query: &str) {
    let names: Vec<String> = keys(cx, base).into_iter().filter(|k| query.is_empty() || k.to_lowercase().contains(query)).collect();
    if names.is_empty() {
        ui.weak("Nothing here.");
        return;
    }
    let cols = ((ui.available_width() / 360.0).floor() as usize).clamp(1, 3);
    let per = names.len().div_ceil(cols);
    ui.columns(cols, |cs| {
        for (c, chunk) in names.chunks(per.max(1)).enumerate() {
            let ui = &mut cs[c];
            ui.push_id((id, c), |ui| {
                let label_w = (ui.available_width() - 150.0).clamp(80.0, 260.0);
                for n in chunk {
                    cx.row_fixed(ui, n, &join(base, n), label_w, 80.0);
                }
            });
        }
    });
}

// ------------------------------------------------------------------ Wasteland 3

const W3_ATTRIBUTES: [&str; 7] = ["Coordination", "Luck", "Awareness", "Strength", "Speed", "Intelligence", "Charisma"];

pub fn wasteland3(ui: &mut Ui, cx: &mut GameCtx) {
    // Desert sand and ranger rust.
    let accent = Color32::from_rgb(201, 122, 52);
    let place = cx.text("/Save/Location").unwrap_or_default();
    let version = cx.text("/Save/Version").map(|v| format!("version {v}")).unwrap_or_default();
    let sub: Vec<String> = [place, version].into_iter().filter(|s| !s.is_empty()).collect();
    // Rangers in the party first, then any on the bench.
    let mut members: Vec<(String, String)> = keys(cx, "/Rangers").into_iter().map(|k| (k.clone(), join("/Rangers", &k))).collect();
    members.extend(keys(cx, "/Bench").into_iter().map(|k| (format!("{k} (bench)"), join("/Bench", &k))));
    hero(ui, accent, "Wasteland 3", &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Money (scrap)", "/Party/Money", accent);
        fact(ui, "Rangers", &members.len().to_string(), accent);
        let items = keys(cx, "/Inventory").len() + keys(cx, "/Party stash").len();
        fact(ui, "Item stacks", &fmt_num(items as f64), accent);
    });
    if !members.is_empty() {
        let names: Vec<String> = members.iter().map(|m| m.0.clone()).collect();
        let t = tabs(ui, "w3_ranger", &names);
        let base = members[t].1.clone();
        let at = |k: &str| join(&base, k);
        columns(ui, 360.0, 2, |ui, col| {
            if col == 0 {
                panel(ui, "Attributes", |ui| {
                    for a in W3_ATTRIBUTES {
                        cx.attribute(ui, a, &at(a), 10.0, accent);
                    }
                    ui.add_space(4.0);
                    ui.weak("Attributes run from 1 to 10 in the game.");
                });
            } else {
                panel(ui, "Ranger", |ui| {
                    egui::Grid::new(("w3_prog", t)).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                        for k in ["Level", "XP", "Hit points", "Attribute points", "Skill points", "Perk points"] {
                            cx.grid_row(ui, k, &at(k), if k == "XP" { 110.0 } else { 70.0 });
                        }
                    });
                    ui.add_space(4.0);
                    ui.weak("Hit points are the current value; the game works out the maximum from level and attributes. Unspent points are spent in the game.");
                });
            }
        });
        let skills = keys(cx, &at("Skills"));
        if !skills.is_empty() {
            panel(ui, "Skills", |ui| {
                columns(ui, 300.0, skills.len(), |ui, i| {
                    cx.attribute(ui, &skills[i], &join(&at("Skills"), &skills[i]), 10.0, accent);
                });
                ui.weak("Skills are listed by the game's skill number (their names aren't in the save).");
            });
        }
        if let Some(Value::Array(perks)) = cx.get(&at("Perks")).cloned() {
            let list: Vec<String> = perks.iter().filter_map(Value::as_str).map(str::to_string).collect();
            if !list.is_empty() {
                panel(ui, "Perks and quirks", |ui| {
                    ui.label(RichText::new(list.join(", ")).color(ui.visuals().weak_text_color()));
                });
            }
        }
        if !keys(cx, &at("Equipment")).is_empty() {
            panel(ui, "Equipped", |ui| amount_rows(ui, cx, &format!("w3_eq{t}"), &at("Equipment"), ""));
        }
    }
    for (title, base) in [("Inventory", "/Inventory"), ("Party stash", "/Party stash")] {
        if cx.get(base).is_none() {
            continue;
        }
        panel(ui, title, |ui| {
            let q = filter(ui, base, "item name");
            amount_rows(ui, cx, base, base, &q);
        });
    }
    ui.weak("Items are shown by their template name. Changing money, levels or XP updates the save only; the save list's summary updates the next time the game saves.");
}

// ------------------------------------------------------------------ Valheim

pub fn valheim(ui: &mut Ui, cx: &mut GameCtx) {
    // Fjord blue and rune gold.
    let accent = Color32::from_rgb(72, 118, 150);
    let gold = Color32::from_rgb(196, 160, 72);
    let name = cx.text("/Character/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Viking".into());
    let mut sub = vec![];
    if let Some(v) = cx.int("/Character/Profile version") {
        sub.push(format!("profile version {v}"));
    }
    if let Some(gp) = cx.text("/Vitals/Guardian power").filter(|s| !s.is_empty()) {
        sub.push(gp.trim_start_matches("GP_").to_string());
    }
    let items = keys(cx, "/Inventory");
    let coins: Vec<String> = items.iter().filter(|k| *k == "Coins" || k.starts_with("Coins (")).map(|k| join(&join("/Inventory", k), "Stack")).collect();
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        match coins.len() {
            0 => fact(ui, "Gold (Coins)", "0", gold),
            1 => cx.tile(ui, "Gold (Coins)", &coins[0], gold),
            _ => {
                let total: i64 = coins.iter().filter_map(|p| cx.int(p)).sum();
                fact(ui, "Gold (all Coins stacks)", &fmt_num(total as f64), gold);
            }
        }
        cx.tile(ui, "Health", "/Vitals/Health", accent);
        cx.tile(ui, "Stamina", "/Vitals/Stamina", accent);
    });
    if let Some(why) = cx.text("/Character/Read-only") {
        panel(ui, "Read-only", |ui| {
            ui.label(why);
        });
        return;
    }
    columns(ui, 340.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Vitals", |ui| {
                egui::Grid::new("vh_vitals").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    cx.grid_row(ui, "Health", "/Vitals/Health", 90.0);
                    cx.grid_row(ui, "Stamina", "/Vitals/Stamina", 90.0);
                });
                if let Some(m) = cx.num("/Vitals/Max health") {
                    ui.label(format!("Max health {}", fmt_num(m)));
                }
                if let Some(Value::Array(f)) = cx.get("/Vitals/Foods eaten") {
                    let f: Vec<&str> = f.iter().filter_map(Value::as_str).collect();
                    if !f.is_empty() {
                        ui.label(format!("Foods: {}", f.join(", ")));
                    }
                }
                ui.weak("Max health and stamina come from the food eaten; the game works them out again on load.");
            });
        } else {
            panel(ui, "Record", |ui| {
                egui::Grid::new("vh_stats").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    for k in ["Kills", "Deaths", "Crafts", "Builds"] {
                        cx.grid_row(ui, k, &join("/Stats", k), 90.0);
                    }
                });
                for k in ["Known recipes", "Known materials", "Trophies", "Biomes found"] {
                    if let Some(n) = cx.int(&join("/Progress", k)) {
                        ui.label(format!("{k}: {n}"));
                    }
                }
            });
        }
    });
    let skills = keys(cx, "/Skills");
    if !skills.is_empty() {
        panel(ui, "Skills", |ui| {
            columns(ui, 300.0, skills.len(), |ui, i| {
                cx.attribute(ui, &skills[i], &join("/Skills", &skills[i]), 100.0, accent);
            });
            ui.weak("Skill levels go from 0 to 100.");
        });
    }
    panel(ui, "Inventory", |ui| {
        if items.is_empty() {
            ui.weak("The inventory is empty.");
            return;
        }
        let q = filter(ui, "vh_inv", "item name");
        let narrow = ui.available_width() < 560.0;
        egui::Grid::new("vh_inv_grid").num_columns(if narrow { 3 } else { 5 }).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
            ui.strong("Item");
            ui.strong("Stack");
            ui.strong("Quality");
            if !narrow {
                ui.strong("Durability");
                ui.strong("Slot");
            }
            ui.end_row();
            for k in items.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)) {
                let base = join("/Inventory", k);
                let equipped = cx.flag(&join(&base, "Equipped"));
                let label = if equipped { format!("{k} (equipped)") } else { k.clone() };
                let c = if cx.modified(&join(&base, "Stack")) || cx.modified(&join(&base, "Quality")) { crate::theme::MODIFIED } else { ui.visuals().text_color() };
                ui.label(RichText::new(label).color(c));
                cx.compact(ui, &join(&base, "Stack"), 70.0);
                cx.compact(ui, &join(&base, "Quality"), 50.0);
                if !narrow {
                    cx.compact(ui, &join(&base, "Durability"), 80.0);
                    ui.weak(cx.text(&join(&base, "Slot")).unwrap_or_default());
                }
                ui.end_row();
            }
        });
        ui.weak("Items are shown by their prefab name. Each item has its own top stack size and quality in the game; going past them may be undone when it loads.");
    });
}
