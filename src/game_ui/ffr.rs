//! Final Fantasy Resonance (`GameSaveData<n>.sav`, an Unreal Engine save): Gil and play time,
//! each character's experience, HP, MP and bonus stats, each Vision's awakening and mastery
//! points, item quantities and the artifacts you hold.
//!
//! Item, character and level-curve data (`src/fmt/data/ffr.json`) comes from the MIT-licensed
//! ffr-save-editor (Jesús Seijas); see THIRD_PARTY_NOTICES.md.

use crate::i18n::{tr, trf};
use super::{columns, fact, fmt_num, hero, panel, tabs, GameCtx};
use eframe::egui::{self, Color32, Ui};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

struct Data {
    units: HashMap<String, String>,
    items: HashMap<String, String>,
    consumables: Vec<i64>,
    exp: HashMap<String, Vec<i64>>,
}

fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| {
        let v: Value = serde_json::from_str(include_str!("../fmt/data/ffr.json")).unwrap_or_default();
        let strs = |k: &str| -> HashMap<String, String> {
            v[k].as_object().map(|m| m.iter().filter_map(|(a, b)| Some((a.clone(), b.as_str()?.to_string()))).collect()).unwrap_or_default()
        };
        Data {
            units: strs("units"),
            items: strs("items"),
            consumables: v["consumables"].as_array().map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default(),
            exp: v["levelExp"]
                .as_object()
                .map(|m| m.iter().map(|(k, a)| (k.clone(), a.as_array().map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default())).collect())
                .unwrap_or_default(),
        }
    })
}

/// A stack in the game never holds more than this many consumables.
const STACK: i64 = 99;

fn unit_name(id: i64) -> String {
    data().units.get(&id.to_string()).cloned().unwrap_or_else(|| format!("Character {id}"))
}

fn item_name(id: i64) -> String {
    data().items.get(&id.to_string()).cloned().unwrap_or_else(|| format!("Item {id}"))
}

/// The level an experience total gives, when the character's curve is known.
fn level(unit: i64, exp: i64) -> Option<usize> {
    let curve = data().exp.get(&unit.to_string())?;
    Some(curve.iter().take_while(|&&need| exp >= need).count().max(1))
}

fn count(cx: &GameCtx, p: &str) -> usize {
    match cx.get(p) {
        Some(Value::Array(a)) => a.len(),
        _ => 0,
    }
}

/// Raises every consumable stack you already hold to 99; returns how many changed.
pub fn max_consumables(cx: &mut GameCtx) -> usize {
    let mut n = 0;
    for i in 0..count(cx, "/inventory") {
        let (Some(id), Some(num)) = (cx.int(&format!("/inventory/{i}/ItemId")), cx.int(&format!("/inventory/{i}/Num"))) else { continue };
        if data().consumables.contains(&id) && num < STACK {
            cx.set(&format!("/inventory/{i}/Num"), Value::from(STACK));
            n += 1;
        }
    }
    n
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(90, 130, 210);
    let units = count(cx, "/unitParameterList");
    let visions = count(cx, "/visionList");
    let hours = cx.num("/Player/PlayTime").map(|s| format!("{:.1} h played", s / 3600.0));
    hero(ui, accent, "Final Fantasy Resonance", &hours.unwrap_or_default(), |ui| {
        cx.tile(ui, "Gil", "/Player/currentMoney", accent);
        fact(ui, "Characters", &units.to_string(), accent);
        fact(ui, "Visions", &visions.to_string(), accent);
    });
    let mut names: Vec<String> = (0..units).map(|i| unit_name(cx.int(&format!("/unitParameterList/{i}/UnitId")).unwrap_or(0))).collect();
    let (vis_tab, item_tab, art_tab) = (names.len(), names.len() + 1, names.len() + 2);
    names.push("Visions".into());
    names.push("Items".into());
    if matches!(cx.get("/artifactList"), Some(Value::Object(m)) if !m.is_empty()) {
        names.push("Artifacts".into());
    }
    let t = tabs(ui, "ffr_tabs", &names);
    if t < units {
        let base = format!("/unitParameterList/{t}");
        let uid = cx.int(&format!("{base}/UnitId")).unwrap_or(0);
        columns(ui, 360.0, 2, |ui, i| {
            if i == 0 {
                panel(ui, "Level and vitals", |ui| {
                    if let (Some(exp), Some(lv)) = (cx.int(&format!("{base}/Experience")), cx.int(&format!("{base}/Experience")).and_then(|e| level(uid, e))) {
                        let top = data().exp.get(&uid.to_string()).map_or(0, Vec::len);
                        ui.label(trf("Level {1} ({0} experience)", &[&(fmt_num(exp as f64)), &lv]));
                        if top > 0 && ui.button(trf("Max level ({0})", &[&top])).clicked() {
                            let need = data().exp[&uid.to_string()][top - 1];
                            cx.set(&format!("{base}/Experience"), Value::from(need));
                        }
                    }
                    egui::Grid::new(("ffr_char", t)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        for (label, key) in [("Experience", "Experience"), ("HP", "HitPoint"), ("MP", "MagicPoint"), ("Enemies defeated", "defeatUnitNum")] {
                            cx.grid_row(ui, label, &format!("{base}/{key}"), 110.0);
                        }
                    });
                    ui.weak(tr("The game may recalculate level and stats from experience."));
                });
            } else {
                panel(ui, "Equipment and bonus stats", |ui| {
                    egui::Grid::new(("ffr_equip", t)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        for (label, key) in [("Weapon", "EquipWeaponId"), ("Armor", "EquipArmorId"), ("Vision", "equipVisionId")] {
                            let p = format!("{base}/{key}");
                            if let Some(id) = cx.int(&p) {
                                cx.label(ui, label, &p);
                                ui.horizontal(|ui| {
                                    cx.editor(ui, &p, 90.0);
                                    if id > 0 {
                                        ui.weak(item_name(id));
                                    }
                                });
                                ui.end_row();
                            }
                        }
                        for j in 0..count(cx, &format!("{base}/baseParameterList")) {
                            cx.grid_row(ui, &format!("Bonus stat {j}"), &format!("{base}/baseParameterList/{j}"), 90.0);
                        }
                    });
                });
            }
        });
    } else if t == vis_tab {
        for i in 0..visions {
            let base = format!("/visionList/{i}");
            let id = cx.int(&format!("{base}/ID")).unwrap_or(0);
            panel(ui, &unit_name(id), |ui| {
                egui::Grid::new(("ffr_vis", i)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    cx.grid_row(ui, "Awakening (0 to 3)", &format!("{base}/awakeningPoint"), 90.0);
                    if let Some(Value::Object(m)) = cx.get(&format!("{base}/synchroPoints")) {
                        let keys: Vec<String> = m.keys().cloned().collect();
                        for k in keys {
                            let who = k.parse().map_or(k.clone(), unit_name);
                            cx.grid_row(ui, &format!("Mastery points: {who}"), &format!("{base}/synchroPoints/{k}"), 90.0);
                        }
                    }
                });
            });
        }
        if visions == 0 {
            ui.weak(tr("No Visions in this save."));
        }
    } else if t == item_tab {
        panel(ui, "Items", |ui| {
            if super::quick_button(ui, format!("Max consumables ({STACK} each)"), "Raises the consumable stacks you already carry to 99. Key items are left alone.") {
                max_consumables(cx);
            }
            egui::Grid::new("ffr_items").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for i in 0..count(cx, "/inventory") {
                    let id = cx.int(&format!("/inventory/{i}/ItemId")).unwrap_or(0);
                    cx.grid_row(ui, &item_name(id), &format!("/inventory/{i}/Num"), 90.0);
                }
            });
            ui.weak(tr("Quantities of items you already have; new item types can't be added here."));
        });
    } else if t == art_tab {
        panel(ui, "Artifacts", |ui| {
            let ids: Vec<String> = match cx.get("/artifactList") {
                Some(Value::Object(m)) => m.keys().cloned().collect(),
                _ => vec![],
            };
            ui.horizontal_wrapped(|ui| {
                for id in ids {
                    let label = id.parse().map_or(id.clone(), item_name);
                    cx.chip(ui, &label, &format!("/artifactList/{id}/IsActive"), accent);
                }
            });
            ui.weak(tr("Click an artifact to switch it on or off."));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn data_is_embedded() {
        assert_eq!(unit_name(1001), "Rain");
        assert_eq!(item_name(1), "Potion");
        assert_eq!(level(1001, 0), Some(1));
        assert_eq!(level(1001, 52), Some(2));
        assert!(level(1001, i64::MAX).unwrap() >= 99);
    }

    #[test]
    fn max_consumables_only_raises_consumables() {
        let root = json!({"inventory": [{"ItemId": 1, "Num": 5}, {"ItemId": 25004, "Num": 2}, {"ItemId": 2, "Num": 99}]});
        let mut cx = GameCtx { root: &root, baseline: &root, tree: None, edits: vec![], archive: None, open_entry: None };
        assert_eq!(max_consumables(&mut cx), 1);
        assert_eq!(cx.edits, vec![("/inventory/0/Num".to_string(), json!(99))]);
    }
}
