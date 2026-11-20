//! Metal Gear Solid V: Ground Zeroes / The Phantom Pain (`TPP_GAME_DATA`): GMP, heroism,
//! Mother Base switches and the lifetime statistics.

use crate::i18n::{tr, trf};
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
            ui.label(&n); // i18n-ok: a name or value from the save, or already translated
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
                if super::quick_button(ui, tr("Max GMP"), "") {
                    cx.set("/Player/GMP", json!(GMP_MAX));
                    if cx.get("/Player/FOB wallet GMP").is_some() {
                        cx.set("/Player/FOB wallet GMP", json!(GMP_MAX));
                    }
                }
                if super::quick_button(ui, tr("Max heroism"), "") {
                    cx.set("/Player/Heroism", json!(99_999_999));
                }
                if ui.button(tr("Clear demon points")).clicked() {
                    cx.set("/Player/Demon points", json!(0));
                }
            });
            grid(ui, cx, "Player", "mgsv_player");
            ui.weak(tr("GMP is capped at 99,999,999 in the game. Close the game and turn Steam Cloud off for it before editing."));
        });
    }
    bases(ui, cx);
    resources(ui, cx);
    research(ui, cx);
    staff(ui, cx);
    let flags: Vec<String> = names(cx, "Mother Base").into_iter().filter(|n| n.starts_with("mbmIsOpened") && n.ends_with("Section")).collect();
    if !flags.is_empty() || cx.get("/Mother Base").is_some() {
        panel(ui, "Mother Base", |ui| {
            if super::quick_button(ui, tr("Open every Mother Base section"), "") {
                for n in &flags {
                    let p = format!("/Mother Base/{}", esc(n));
                    let on = if cx.get(&p).map_or(false, Value::is_boolean) { json!(true) } else { json!(1) };
                    cx.set(&p, on);
                }
            }
            ui.weak(tr("The names are the game's own variables; hover over a section's flag to guess what it does, or leave the rest alone."));
            grid(ui, cx, "Mother Base", "mgsv_mb");
        });
    }
    if cx.get("/Statistics").is_some() {
        panel(ui, "Statistics", |ui| grid(ui, cx, "Statistics", "mgsv_stats"));
    }
    ui.weak(tr("Every variable in the save, named by the game's own names, is in the Tree under Variables."));
}

/// Mother Base's online caps since 1.07: 1,000,000 of each material (processed and unprocessed
/// alike) and 30,000 of each plant. Offline storage holds 500,000 and 6,000 on top.
const MATERIAL_ONLINE_CAP: i64 = 1_000_000;
const PLANT_ONLINE_CAP: i64 = 30_000;
const MATERIAL_LOCAL_CAP: i64 = 500_000;
const PLANT_LOCAL_CAP: i64 = 6_000;

fn resources(ui: &mut Ui, cx: &mut GameCtx) {
    let rs = names(cx, "Resources");
    if rs.is_empty() {
        return;
    }
    panel(ui, "Mother Base resources", |ui| {
        if super::quick_button(ui, tr("Max crafting materials"), tr("Fuel, biological and the three metals to 1,000,000 online and 500,000 local, processed too; the eight medicinal plants to 30,000 online and 6,000 local (the game's own caps)")) {
            for (i, n) in rs.iter().enumerate() {
                if i < 13 {
                    let cap = if i < 5 { MATERIAL_ONLINE_CAP } else { PLANT_ONLINE_CAP };
                    cx.set(&format!("/Resources/{}/Usable (online)", esc(n)), json!(cap));
                    let local = if i < 5 { MATERIAL_LOCAL_CAP } else { PLANT_LOCAL_CAP };
                    cx.set(&format!("/Resources/{}/Usable (local)", esc(n)), json!(local));
                }
                if i < 5 {
                    cx.set(&format!("/Resources/{}/Processed (online)", esc(n)), json!(MATERIAL_ONLINE_CAP));
                }
            }
        }
        Grid::new("mgsv_res").num_columns(5).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.strong(tr("Resource"));
            ui.strong(tr("Owned"));
            ui.strong(tr("Online"));
            ui.strong(tr("Local"));
            ui.strong(tr("Processed (online)"));
            ui.end_row();
            for n in &rs {
                ui.label(n); // i18n-ok: a name or value from the save, or already translated
                let p = |f: &str| format!("/Resources/{}/{f}", esc(n));
                let total = cx.int(&p("Usable (online)")).unwrap_or(0) + cx.int(&p("Usable (local)")).unwrap_or(0);
                ui.label(total.to_string());
                cx.editor(ui, &p("Usable (online)"), 90.0);
                cx.editor(ui, &p("Usable (local)"), 90.0);
                cx.editor(ui, &p("Processed (online)"), 90.0);
                ui.end_row();
            }
        });
        ui.weak(tr("The game shows Owned = online + local (the number with the globe is the online part). Checked against the in-game vehicle, walker gear and placed-weapon screens."));
    });
}

/// The development type of a research row, translated.
fn kind_label(kind: &str) -> &'static str {
    match kind {
        "Prosthetic arm" => tr("Prosthetic arm"),
        "Assault rifle" => tr("Assault rifle"),
        "Battle Gear" => tr("Battle Gear"),
        "D-Dog" => tr("D-Dog"),
        "Equipment" => tr("Equipment"),
        "Grenade launcher" => tr("Grenade launcher"),
        "Helicopter" => tr("Helicopter"),
        "Machine gun" => tr("Machine gun"),
        "Placed weapon" => tr("Placed weapon"),
        "Security gadget" => tr("Security gadget"),
        "Sniper rifle" => tr("Sniper rifle"),
        "Submachine gun" => tr("Submachine gun"),
        "Throwing weapon" => tr("Throwing weapon"),
        "Walker Gear" => tr("Walker Gear"),
        "Handgun" => tr("Handgun"),
        "Horse" => tr("Horse"),
        "Missile" => tr("Missile"),
        "Quiet" => tr("Quiet"),
        "Shield" => tr("Shield"),
        "Shotgun" => tr("Shotgun"),
        "Suit" => tr("Suit"),
        "Tool" => tr("Tool"),
        _ => tr("Other"),
    }
}

/// Mother Base R&D: which weapons, items, suits and support gear are developed.
fn research(ui: &mut Ui, cx: &mut GameCtx) {
    use crate::fmt::mgsv::{dev_key, dev_rows};
    if cx.get("/Research").is_none() {
        return;
    }
    let rows = dev_rows();
    let state = |cx: &GameCtx, i: usize| -> Option<(bool, bool)> {
        let p = format!("/Research/{}", esc(&dev_key(&rows[i])));
        Some((cx.get(&format!("{p}/Developed"))?.as_bool()?, cx.get(&format!("{p}/Available"))?.as_bool()?))
    };
    let shown_rows: Vec<usize> = (0..rows.len()).filter(|&i| !rows[i].hidden && state(cx, i).is_some()).collect();
    let developed = shown_rows.iter().filter(|&&i| state(cx, i).map_or(false, |s| s.0)).count();
    panel(ui, "Research and development", |ui| {
        ui.horizontal_wrapped(|ui| {
            if super::quick_button(ui, tr("Develop everything available"), tr("Marks developed every item of the R&D menu that can be developed now, with the earlier grades of its line. Left alone: DLC, enemy and unused entries, items that need a blueprint you haven't earned, and lines whose first grade is still locked by the story.")) {
                let plan = crate::fmt::mgsv::develop_plan(&|i| state(cx, i).unwrap_or((false, false)));
                for i in plan {
                    let p = format!("/Research/{}", esc(&dev_key(&rows[i])));
                    cx.set(&format!("{p}/Available"), json!(true));
                    cx.set(&format!("{p}/Developed"), json!(true));
                }
            }
        });
        let filter_id = ui.id().with("mgsv_research_kind");
        let mut filter: String = ui.memory_mut(|m| m.data.get_temp::<String>(filter_id)).unwrap_or_default();
        let mut kinds: Vec<&'static str> = vec![];
        for &i in &shown_rows {
            if !kinds.contains(&rows[i].kind) {
                kinds.push(rows[i].kind);
            }
        }
        kinds.sort();
        ui.horizontal(|ui| {
            ui.label(tr("Show"));
            let current = if filter.is_empty() { tr("All types") } else { kind_label(&filter) };
            eframe::egui::ComboBox::from_id_salt("mgsv_research_filter").selected_text(current).show_ui(ui, |ui| {
                ui.selectable_value(&mut filter, String::new(), tr("All types"));
                for k in &kinds {
                    ui.selectable_value(&mut filter, k.to_string(), kind_label(k));
                }
            });
        });
        ui.memory_mut(|m| m.data.insert_temp(filter_id, filter.clone()));
        let shown: Vec<usize> = shown_rows.iter().copied().filter(|&i| filter.is_empty() || rows[i].kind == filter).collect();
        ui.label(trf("{0} of {1} developed. Items are listed by their development number; the game's item names are not in the save.", &[&developed, &shown_rows.len()]));
        const ROW: f32 = 24.0;
        let cell = |ui: &mut Ui, w: f32, add: &mut dyn FnMut(&mut Ui)| {
            ui.allocate_ui_with_layout(eframe::egui::vec2(w, ROW), eframe::egui::Layout::left_to_right(eframe::egui::Align::Center), |ui| {
                ui.set_min_width(w);
                add(ui);
            });
        };
        let widths = [170.0, 70.0, 110.0, 110.0];
        let heads = [tr("Item"), tr("Grade"), tr("Developed"), tr("Available")];
        ScrollArea::horizontal().auto_shrink([false, true]).id_salt("mgsv_research_h").show_locked(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (w, h) in widths.iter().zip(heads) {
                    cell(ui, *w, &mut |ui| {
                        ui.strong(h); // i18n-ok: translated above
                    });
                }
            });
            let cap = crate::widgets::list_height(ui, 360.0);
            let height = (shown_rows.len() as f32 * ROW + 6.0).clamp(ROW * 5.0, cap.max(ROW * 5.0));
            ScrollArea::vertical().auto_shrink([false, false]).max_height(height).min_scrolled_height(height).id_salt("mgsv_research").show_rows_locked(ui, ROW, shown.len(), |ui, range| {
                for k in range {
                    let i = shown[k];
                    let row = &rows[i];
                    let p = format!("/Research/{}", esc(&dev_key(row)));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        cell(ui, widths[0], &mut |ui| {
                            ui.label(format!("{} {}", kind_label(row.kind), row.id)); // i18n-ok: a translated type and a number
                        });
                        cell(ui, widths[1], &mut |ui| {
                            ui.label(row.grade.to_string());
                        });
                        cell(ui, widths[2], &mut |ui| {
                            let n = cx.edits.len();
                            cx.editor(ui, &format!("{p}/Developed"), 40.0);
                            // A developed item is also marked available, as the game does.
                            let dev_path = format!("{p}/Developed");
                            if cx.edits.len() > n && cx.edits[n..].iter().any(|(q, v)| *q == dev_path && v.as_bool() == Some(true)) {
                                cx.set(&format!("{p}/Available"), json!(true));
                            }
                        });
                        cell(ui, widths[3], &mut |ui| cx.editor(ui, &format!("{p}/Available"), 40.0));
                    });
                }
            });
        });
        ui.weak(tr("Developed items appear in the iDroid as ready to use; develop an earlier grade before a later one. Items still being developed keep their timer. Close the game and turn Steam Cloud off before editing."));
    });
}

/// The single-team stat type (2 Combat … 7 Medical) a specialist stat type (8–25) is switched
/// to by "Everyone to S++": in game, specialist types keep the other team stats below S++.
/// Other types (single-team ones, and the unique staff's own) are left alone.
fn team_distribution(dist: u64, section: &str) -> Option<u64> {
    if !(8..=25).contains(&dist) {
        return None;
    }
    Some(match section {
        "R&D" => 3,
        "Base Development" => 4,
        "Support" => 5,
        "Intel" => 6,
        "Medical" | "Sickbay" => 7,
        _ => 2,
    })
}

fn staff(ui: &mut Ui, cx: &mut GameCtx) {
    let all = names(cx, "Staff");
    if all.is_empty() {
        return;
    }
    panel(ui, &trf("Staff ({0})", &[&all.len()]), |ui| {
        ui.horizontal_wrapped(|ui| {
            if super::quick_button(ui, tr("Everyone to S++"), tr("Rank S++, top stat bonus, all three medals, morale and proficiency at 15, the stat cap removed, and specialist stat types switched to the member's own team: in game this gives S++ in all six stats. Rank alone leaves some stats lower.")) {
                for n in &all {
                    let s = format!("/Staff/{}", esc(n));
                    cx.set(&format!("{s}/Rank"), json!("S++"));
                    cx.set(&format!("{s}/Stat bonus"), json!(3));
                    for medal in ["DS medal", "DS cross", "Honor medal"] {
                        cx.set(&format!("{s}/{medal}"), json!(true));
                    }
                    cx.set(&format!("{s}/Morale"), json!(15));
                    cx.set(&format!("{s}/Proficiency"), json!(15));
                    cx.set(&format!("{s}/Stats suppressed"), json!(false));
                    let dist = cx.get(&format!("{s}/Stat distribution")).and_then(Value::as_u64);
                    if let Some(team) = dist.and_then(|d| team_distribution(d, cx.get(&format!("{s}/Section")).and_then(Value::as_str).unwrap_or(""))) {
                        cx.set(&format!("{s}/Stat distribution"), json!(team));
                    }
                }
            }
            if super::quick_button(ui, tr("Max stat bonus"), "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Stat bonus", esc(n)), json!(3));
                }
            }
            if super::quick_button(ui, tr("Max morale"), "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Morale", esc(n)), json!(15));
                }
            }
            if super::quick_button(ui, tr("Max proficiency"), "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Proficiency", esc(n)), json!(15));
                }
            }
            if super::quick_button(ui, tr("Heal everyone"), "") {
                for n in &all {
                    cx.set(&format!("/Staff/{}/Symptomatic", esc(n)), json!(false));
                }
            }
        });
        let filter_id = ui.id().with("mgsv_section");
        let mut filter: String = ui.memory_mut(|m| m.data.get_temp::<String>(filter_id)).unwrap_or_else(|| "All sections".into());
        ui.horizontal(|ui| {
            ui.label(tr("Show"));
            eframe::egui::ComboBox::from_id_salt("mgsv_sec_filter").selected_text(crate::i18n::tr_any(&filter)).show_ui(ui, |ui| {
                for s in ["All sections", "Combat", "R&D", "Base Development", "Support", "Intel", "Medical", "Security", "Sickbay", "Brig", "Unassigned"] {
                    ui.selectable_value(&mut filter, s.to_string(), crate::i18n::tr_any(s));
                }
            });
        });
        ui.memory_mut(|m| m.data.insert_temp(filter_id, filter.clone()));
        let shown: Vec<&String> = all.iter().filter(|n| filter == "All sections" || cx.text(&format!("/Staff/{}/Section", esc(n))).as_deref() == Some(filter.as_str())).collect();
        ui.label(trf("{0} shown. Staff have no names in the save; they are listed by slot.", &[&(shown.len())]));
        // Fixed-size box: every cell has a set width and the height follows the whole staff
        // list (not the filtered one), so neither changes while filtering, editing or scrolling.
        // Column headings sit above the rows instead of repeating a label in every row.
        const ROW: f32 = 24.0;
        let cell = |ui: &mut Ui, w: f32, add: &mut dyn FnMut(&mut Ui)| {
            ui.allocate_ui_with_layout(eframe::egui::vec2(w, ROW), eframe::egui::Layout::left_to_right(eframe::egui::Align::Center), |ui| {
                ui.set_min_width(w);
                add(ui);
            });
        };
        let widths = [84.0, 156.0, 156.0, 156.0, 92.0, 92.0, 92.0];
        let heads = ["Slot", "Section", "Rank", "Skill", "Morale", "Proficiency", "Stat bonus"];
        ScrollArea::horizontal().auto_shrink([false, true]).id_salt("mgsv_staff_h").show_locked(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (w, h) in widths.iter().zip(heads) {
                    cell(ui, *w, &mut |ui| {
                        ui.strong(crate::i18n::tr_any(h));
                    });
                }
            });
            let cap = crate::widgets::list_height(ui, 360.0);
            let height = (all.len() as f32 * ROW + 6.0).clamp(ROW * 5.0, cap.max(ROW * 5.0));
            ScrollArea::vertical().auto_shrink([false, false]).max_height(height).min_scrolled_height(height).id_salt("mgsv_staff").show_rows_locked(ui, ROW, shown.len(), |ui, range| {
                for i in range {
                    let n = shown[i];
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        cell(ui, widths[0], &mut |ui| {
                            ui.label(n.as_str());
                        });
                        for (k, f) in ["Section", "Rank", "Skill"].into_iter().enumerate() {
                            cell(ui, widths[1 + k], &mut |ui| cx.editor(ui, &format!("/Staff/{}/{}", esc(n), f), 150.0));
                        }
                        for (k, f) in ["Morale", "Proficiency", "Stat bonus"].into_iter().enumerate() {
                            cell(ui, widths[4 + k], &mut |ui| cx.compact(ui, &format!("/Staff/{}/{}", esc(n), f), 56.0));
                        }
                    });
                }
            });
        });
        ui.weak(tr("Rank is the staff member's peak rank (S++ is the top). The six team stats follow from it, the stat distribution and bonus, medals and morale. The rest of each member's fields are in the Tree."));
    });
}

fn bases(ui: &mut Ui, cx: &mut GameCtx) {
    let bs = names(cx, "Bases");
    if bs.is_empty() {
        return;
    }
    panel(ui, "Mother Base and FOB platforms", |ui| {
        if super::quick_button(ui, tr("Finish every platform (grade 4)"), "") {
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
                        ui.label(&pl); // i18n-ok: a name or value from the save, or already translated
                        let p = format!("/Bases/{}/{}", esc(b), esc(&pl));
                        cx.editor(ui, &format!("{p}/Grade (0-4)"), 50.0);
                        cx.editor(ui, &format!("{p}/Under construction"), 50.0);
                        ui.end_row();
                    }
                });
            });
        }
        ui.weak(tr("The save lists 35 platform slots in order: Mother Base, then FOB 1 to 4, each Command, Combat, R&D, Base Development, Support, Intel, Medical (matches the in-game base screens). Not every FOB platform exists until built in the game."));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    fn root(skill: &str, rank: &str, level: i64) -> Value {
        let mut staff = serde_json::Map::new();
        for i in 0..80 {
            staff.insert(format!("Staff {:04}", i + 1), json!({"Section": "Base Development", "Rank": rank, "Skill": skill, "Morale": level, "Proficiency": level, "Stat bonus": 3}));
        }
        json!({"Player": {"GMP": 5}, "Staff": staff})
    }

    fn size_of(root: &Value, w: f32) -> egui::Vec2 {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let mut size = egui::Vec2::ZERO;
        for _ in 0..3 {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 900.0))), ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut cx = GameCtx { root, baseline: root, tree: None, edits: vec![], archive: None, open_entry: None };
                    staff(ui, &mut cx);
                    size = ui.min_rect().size();
                });
            });
        }
        size
    }

    #[test]
    fn specialist_stat_types_switch_to_the_team_type() {
        assert_eq!(team_distribution(18, "Intel"), Some(6));
        assert_eq!(team_distribution(8, "Combat"), Some(2));
        assert_eq!(team_distribution(25, "Sickbay"), Some(7));
        assert_eq!(team_distribution(12, "Brig"), Some(2));
        for keep in [2, 6, 7, 30, 35, 63] {
            assert_eq!(team_distribution(keep, "R&D"), None, "{keep}");
        }
    }

    /// The staff box keeps one size whatever the values in it (short or long skill names, small or
    /// large numbers), so the screen doesn't grow or shrink as the list scrolls.
    #[test]
    fn staff_box_keeps_one_size() {
        for w in [1100.0, 380.0] {
            let a = size_of(&root("Ranger", "E", 1), w);
            let b = size_of(&root("Intelligence Specialist Lv3", "S++", 15), w);
            assert_eq!(a, b, "size changed with the values at width {w}");
            assert!(a.y >= 380.0, "staff box too short: {a:?}");
        }
    }
}
