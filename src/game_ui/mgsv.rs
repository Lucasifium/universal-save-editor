//! Metal Gear Solid V: Ground Zeroes / The Phantom Pain (`TPP_GAME_DATA`): GMP, heroism,
//! Mother Base switches and the lifetime statistics.

use crate::widgets::ScrollLock;
use super::{hero, panel, GameCtx};
use eframe::egui::{Color32, Grid, ScrollArea, Ui};
use serde_json::{json, Value};

const ACCENT: Color32 = Color32::from_rgb(214, 170, 60);
const GMP_MAX: i64 = 99_999_999;

fn names(cx: &GameCtx, group: &str) -> Vec<String> {
    match cx.get(&format!("/{group}")) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn esc(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}

fn grid(ui: &mut Ui, cx: &mut GameCtx, group: &str, id: &str) {
    Grid::new(id).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
        for n in names(cx, group) {
            ui.label(&n);
            cx.editor(ui, &format!("/{group}/{}", esc(&n)), 110.0);
            ui.end_row();
        }
    });
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    hero(ui, ACCENT, "Mother Base commander", "Metal Gear Solid V save", |ui| {
        cx.tile(ui, "GMP", "/Player/GMP", ACCENT);
        cx.tile(ui, "Heroism", "/Player/Heroism", ACCENT);
        cx.tile(ui, "Demon points", "/Player/Demon points", ACCENT);
    });
    if cx.get("/Player").is_some() {
        panel(ui, "Player", |ui| {
            ui.horizontal(|ui| {
                if super::quick_button(ui, "Max GMP", "") {
                    cx.set("/Player/GMP", json!(GMP_MAX));
                    if cx.get("/Player/FOB wallet GMP").is_some() {
                        cx.set("/Player/FOB wallet GMP", json!(GMP_MAX));
                    }
                }
                if super::quick_button(ui, "Max heroism", "") {
                    cx.set("/Player/Heroism", json!(99_999_999));
                }
                if ui.button("Clear demon points").clicked() {
                    cx.set("/Player/Demon points", json!(0));
                }
            });
            grid(ui, cx, "Player", "mgsv_player");
            ui.weak("GMP is capped at 99,999,999 in the game. Close the game and turn Steam Cloud off for it before editing.");
        });
    }
    bases(ui, cx);
    resources(ui, cx);
    staff(ui, cx);
    let flags: Vec<String> = names(cx, "Mother Base").into_iter().filter(|n| n.starts_with("mbmIsOpened") && n.ends_with("Section")).collect();
    if !flags.is_empty() || cx.get("/Mother Base").is_some() {
        panel(ui, "Mother Base", |ui| {
            if super::quick_button(ui, "Open every Mother Base section", "") {
                for n in &flags {
                    let p = format!("/Mother Base/{}", esc(n));
                    let on = if cx.get(&p).map_or(false, Value::is_boolean) { json!(true) } else { json!(1) };
                    cx.set(&p, on);
                }
            }
            ui.weak("The names are the game's own variables; hover over a section's flag to guess what it does, or leave the rest alone.");
            grid(ui, cx, "Mother Base", "mgsv_mb");
        });
    }
    if cx.get("/Statistics").is_some() {
        panel(ui, "Statistics", |ui| grid(ui, cx, "Statistics", "mgsv_stats"));
    }
    ui.weak("Every variable in the save, named by the game's own names, is in the Tree under Variables.");
}

/// Mother Base's online caps since 1.07: 1,000,000 of each material (processed and unprocessed
/// alike) and 30,000 of each plant. Offline storage holds 500,000 and 6,000 on top.
const MATERIAL_ONLINE_CAP: i64 = 1_000_000;
const PLANT_ONLINE_CAP: i64 = 30_000;

fn resources(ui: &mut Ui, cx: &mut GameCtx) {
    let rs = names(cx, "Resources");
    if rs.is_empty() {
        return;
    }
    panel(ui, "Mother Base resources", |ui| {
        if super::quick_button(ui, "Max crafting materials", "Fuel, biological and the three metals to 1,000,000 online, processed too; the eight medicinal plants to 30,000 online (the game's own online caps)") {
            for (i, n) in rs.iter().enumerate() {
                if i < 13 {
                    let cap = if i < 5 { MATERIAL_ONLINE_CAP } else { PLANT_ONLINE_CAP };
                    cx.set(&format!("/Resources/{}/Usable (online)", esc(n)), json!(cap));
                }
                if i < 5 {
                    cx.set(&format!("/Resources/{}/Processed (online)", esc(n)), json!(MATERIAL_ONLINE_CAP));
                }
            }
        }
        Grid::new("mgsv_res").num_columns(5).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.strong("Resource");
            ui.strong("Owned");
            ui.strong("Online");
            ui.strong("Local");
            ui.strong("Processed (online)");
            ui.end_row();
            for n in &rs {
                ui.label(n);
                let p = |f: &str| format!("/Resources/{}/{f}", esc(n));
                let total = cx.int(&p("Usable (online)")).unwrap_or(0) + cx.int(&p("Usable (local)")).unwrap_or(0);
                ui.label(total.to_string());
                cx.editor(ui, &p("Usable (online)"), 90.0);
                cx.editor(ui, &p("Usable (local)"), 90.0);
                cx.editor(ui, &p("Processed (online)"), 90.0);
                ui.end_row();
            }
        });
        ui.weak("The game shows Owned = online + local (the number with the globe is the online part). Checked against the in-game vehicle, walker gear and placed-weapon screens.");
    });
}

fn staff(ui: &mut Ui, cx: &mut GameCtx) {
    let all = names(cx, "Staff");
    if all.is_empty() {
        return;
    }
    panel(ui, &format!("Staff ({})", all.len()), |ui| {
        ui.horizontal_wrapped(|ui| {
            if super::quick_button(ui, "Everyone to S++", "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Rank", esc(n)), json!("S++"));
                }
            }
            if super::quick_button(ui, "Max stat bonus", "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Stat bonus", esc(n)), json!(3));
                }
            }
            if super::quick_button(ui, "Max morale", "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Morale", esc(n)), json!(15));
                }
            }
            if super::quick_button(ui, "Max proficiency", "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Proficiency", esc(n)), json!(15));
                }
            }
            if super::quick_button(ui, "Heal everyone", "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Symptomatic", esc(n)), json!(false));
                }
            }
        });
        let filter_id = ui.id().with("mgsv_section");
        let mut filter: String = ui.memory_mut(|m| m.data.get_temp::<String>(filter_id)).unwrap_or_else(|| "All sections".into());
        ui.horizontal(|ui| {
            ui.label("Show");
            eframe::egui::ComboBox::from_id_salt("mgsv_sec_filter").selected_text(&filter).show_ui(ui, |ui| {
                for s in ["All sections", "Combat", "R&D", "Base Development", "Support", "Intel", "Medical", "Security", "Sickbay", "Brig", "Unassigned"] {
                    ui.selectable_value(&mut filter, s.to_string(), s);
                }
            });
        });
        ui.memory_mut(|m| m.data.insert_temp(filter_id, filter.clone()));
        let shown: Vec<&String> = all.iter().filter(|n| filter == "All sections" || cx.text(&format!("/Staff/{}/Section", esc(n))).as_deref() == Some(filter.as_str())).collect();
        ui.label(format!("{} shown. Staff have no names in the save; they are listed by slot.", shown.len()));
        ScrollArea::vertical().max_height(360.0).id_salt("mgsv_staff").show_rows_locked(ui, 26.0, shown.len(), |ui, range| {
            for i in range {
                let n = shown[i];
                ui.horizontal(|ui| {
                    ui.label(n.as_str());
                    for f in ["Section", "Rank", "Skill"] {
                        cx.editor(ui, &format!("/Staff/{}/{}", esc(n), f), 150.0);
                    }
                    for f in ["Morale", "Proficiency", "Stat bonus"] {
                        ui.label(f);
                        cx.editor(ui, &format!("/Staff/{}/{}", esc(n), f), 50.0);
                    }
                });
            }
        });
        ui.weak("Rank is the staff member's stat rank (S++ is the top); every other stat follows from rank and stat distribution. The rest of each member's fields are in the Tree.");
    });
}

fn bases(ui: &mut Ui, cx: &mut GameCtx) {
    let bs = names(cx, "Bases");
    if bs.is_empty() {
        return;
    }
    panel(ui, "Mother Base and FOB platforms", |ui| {
        if super::quick_button(ui, "Finish every platform (grade 4)", "") {
            for b in &bs {
                for pl in names(cx, &format!("Bases/{}", esc(b))) {
                    let p = format!("/Bases/{}/{}", esc(b), esc(&pl));
                    cx.set(&format!("{p}/Grade (0-4)"), json!(4));
                    cx.set(&format!("{p}/Under construction"), json!(false));
                }
            }
        }
        for b in &bs {
            ui.collapsing(b.as_str(), |ui| {
                Grid::new(format!("mgsv_base_{b}")).num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
                    for pl in names(cx, &format!("Bases/{}", esc(b))) {
                        ui.label(&pl);
                        let p = format!("/Bases/{}/{}", esc(b), esc(&pl));
                        cx.editor(ui, &format!("{p}/Grade (0-4)"), 50.0);
                        cx.editor(ui, &format!("{p}/Under construction"), 50.0);
                        ui.end_row();
                    }
                });
            });
        }
        ui.weak("The save lists 35 platform slots in order: Mother Base, then FOB 1 to 4, each Command, Combat, R&D, Base Development, Support, Intel, Medical (matches the in-game base screens). Not every FOB platform exists until built in the game.");
    });
}
