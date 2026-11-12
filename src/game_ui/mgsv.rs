//! Metal Gear Solid V: Ground Zeroes / The Phantom Pain (`TPP_GAME_DATA`): GMP, heroism,
//! Mother Base switches and the lifetime statistics.

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
                if ui.button("Max GMP").clicked() {
                    cx.set("/Player/GMP", json!(GMP_MAX));
                    if cx.get("/Player/FOB wallet GMP").is_some() {
                        cx.set("/Player/FOB wallet GMP", json!(GMP_MAX));
                    }
                }
                if ui.button("Clear demon points").clicked() {
                    cx.set("/Player/Demon points", json!(0));
                }
            });
            grid(ui, cx, "Player", "mgsv_player");
            ui.weak("GMP is capped at 99,999,999 in the game. Close the game and turn Steam Cloud off for it before editing.");
        });
    }
    resources(ui, cx);
    staff(ui, cx);
    let flags: Vec<String> = names(cx, "Mother Base").into_iter().filter(|n| n.starts_with("mbmIsOpened") && n.ends_with("Section")).collect();
    if !flags.is_empty() || cx.get("/Mother Base").is_some() {
        panel(ui, "Mother Base", |ui| {
            if ui.button("Open every Mother Base section").clicked() {
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

fn resources(ui: &mut Ui, cx: &mut GameCtx) {
    let rs = names(cx, "Resources");
    if rs.is_empty() {
        return;
    }
    panel(ui, "Mother Base resources", |ui| {
        if ui.button("Max crafting materials").on_hover_text("Fuel, biological, metals and plants to 999,999 usable; fuel, biological and metals also to 1,000,000 processed (an estimate of the cap)").clicked() {
            for (i, n) in rs.iter().enumerate() {
                if i < 13 {
                    cx.set(&format!("/Resources/{}/Usable", esc(n)), json!(999_999));
                }
                if i < 5 {
                    cx.set(&format!("/Resources/{}/Processing", esc(n)), json!(1_000_000));
                }
            }
        }
        Grid::new("mgsv_res").num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.strong("Resource");
            ui.strong("Usable");
            ui.strong("Processing");
            ui.end_row();
            for n in &rs {
                ui.label(n);
                cx.editor(ui, &format!("/Resources/{}/Usable", esc(n)), 90.0);
                cx.editor(ui, &format!("/Resources/{}/Processing", esc(n)), 90.0);
                ui.end_row();
            }
        });
        ui.weak("Counts as the save stores them. Check them against the in-game resource screen before relying on them.");
    });
}

fn staff(ui: &mut Ui, cx: &mut GameCtx) {
    let all = names(cx, "Staff");
    if all.is_empty() {
        return;
    }
    panel(ui, &format!("Staff ({})", all.len()), |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Everyone to S++").clicked() {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Rank", esc(n)), json!("S++"));
                }
            }
            if ui.button("Max stat bonus").clicked() {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Stat bonus", esc(n)), json!(3));
                }
            }
            if ui.button("Max morale").clicked() {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Morale", esc(n)), json!(15));
                }
            }
            if ui.button("Max proficiency").clicked() {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Proficiency", esc(n)), json!(15));
                }
            }
            if ui.button("Heal everyone").clicked() {
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
        ScrollArea::vertical().max_height(360.0).id_salt("mgsv_staff").show_rows(ui, 26.0, shown.len(), |ui, range| {
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
