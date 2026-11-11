//! Star Wars: Knights of the Old Republic 1 and 2. `SAVEGAME.sav` (and an auto-save's
//! `pifo.ifo`) shows the player and the party members; `PARTYTABLE.res` shows credits, party
//! XP and KOTOR II's influence, components and chemicals. Values come from `fmt::kotor`.

use super::{columns, hero, panel, tabs, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

/// Lightsaber gold on the hero band, a cool blue for the bars.
const GOLD: Color32 = Color32::from_rgb(230, 190, 40);
const BLUE: Color32 = Color32::from_rgb(70, 150, 230);
const RED: Color32 = Color32::from_rgb(200, 60, 50);

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn list(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    }
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    if cx.get("/Party/Credits").is_some() {
        party_table(ui, cx);
    } else {
        save(ui, cx);
    }
}

fn short_game(cx: &GameCtx) -> String {
    let g = cx.text("/Save/Game").unwrap_or_default();
    g.trim_start_matches("Star Wars: ").to_string()
}

fn party_table(ui: &mut Ui, cx: &mut GameCtx) {
    let mut sub = vec![];
    if let Some(p) = cx.text("/Save/Player") {
        sub.push(p);
    }
    if let Some(h) = cx.num("/Save/Hours played") {
        sub.push(format!("{h:.1} hours"));
    }
    sub.push("Party table".into());
    hero(ui, GOLD, &short_game(cx), &sub.join("  \u{00B7}  "), |ui| {
        for (label, p) in [
            ("Credits", "/Party/Credits"),
            ("Party XP", "/Party/Party XP"),
            ("Components", "/Party/Components"),
            ("Chemicals", "/Party/Chemicals"),
        ] {
            cx.tile(ui, label, p, GOLD);
        }
    });
    super::crafting::offer(super::Game::Kotor, ui, cx);
    let names = keys(cx, "/Influence");
    if !names.is_empty() {
        panel(ui, "Influence", |ui| {
            columns(ui, 320.0, names.len(), |ui, i| {
                let p = join("/Influence", &names[i]);
                if cx.get(&p).is_some_and(Value::is_number) {
                    cx.attribute(ui, &names[i], &p, 100.0, BLUE);
                } else {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [110.0, 22.0],
                            egui::Label::new(RichText::new(&names[i]).weak()).truncate(),
                        );
                        ui.weak("not met yet");
                    });
                }
            });
            ui.weak("0\u{2013}100. High influence opens a companion's conversations and can turn them towards your side of the Force.");
        });
    }
    let members = list(cx, "/Party/In the party");
    let avail = list(cx, "/Party/Available");
    panel(ui, "Party", |ui| {
        ui.label(format!(
            "With you: {}",
            if members.is_empty() {
                "nobody".into()
            } else {
                members.join(", ")
            }
        ));
        if !avail.is_empty() {
            ui.label(format!("Can join: {}", avail.join(", ")));
        }
    });
    ui.weak(
        "Stats of the player and the party members are in SAVEGAME.sav in the same save folder.",
    );
}

fn save(ui: &mut Ui, cx: &mut GameCtx) {
    let mut people: Vec<(String, String)> = vec![];
    if cx.get("/Player").is_some() {
        let n = cx
            .text("/Player/Name")
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Player".into());
        people.push((n, "/Player".into()));
    }
    for k in keys(cx, "/Party") {
        people.push((k.clone(), join("/Party", &k)));
    }
    let mut sub = vec![short_game(cx)];
    for p in ["/Save/Area", "/Save/Module"] {
        if let Some(t) = cx.text(p).filter(|t| !t.is_empty()) {
            sub.push(t);
            break;
        }
    }
    let title = cx
        .text("/Player/Name")
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Your party".into());
    hero(ui, GOLD, &title, &sub.join("  \u{00B7}  "), |ui| {
        for (label, p) in [
            ("Experience", "/Player/Experience"),
            ("Skill points", "/Player/Skill points"),
            ("Level", "/Player/Level"),
        ] {
            if cx.can_edit(p) {
                cx.tile(ui, label, p, GOLD);
            } else if let Some(v) = cx.text(p) {
                super::fact(ui, label, &v, GOLD);
            }
        }
    });
    if cx.get("/Player").is_none() && cx.text("/Save/Player file").is_some() {
        panel(ui, "Player", |ui| {
            ui.label("This auto-save keeps the player in pifo.ifo, which the game loads instead of the copies in SAVEGAME.sav. Open pifo.ifo from the same folder to edit the player.");
        });
    }
    if people.is_empty() {
        return;
    }
    let names: Vec<String> = people.iter().map(|p| p.0.clone()).collect();
    let t = tabs(ui, "kotor_who", &names);
    let base = people[t].1.clone();
    sheet(ui, cx, &base, t);
    let items = list(cx, "/Inventory");
    if !items.is_empty() {
        egui::CollapsingHeader::new(
            RichText::new(format!("Party inventory ({} kinds)", items.len())).strong(),
        )
        .id_salt("kotor_inv")
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("kotor_inv_s")
                .max_height(260.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for it in &items {
                            ui.label(RichText::new(it).monospace().size(12.0));
                            ui.add_space(8.0);
                        }
                    });
                });
            ui.weak("Item tags as the game stores them; items can't be added or removed here.");
        });
    }
    ui.add_space(6.0);
    ui.weak("Credits, party XP, influence, components and chemicals are in PARTYTABLE.res in the same save folder.");
}

fn sheet(ui: &mut Ui, cx: &mut GameCtx, base: &str, id: usize) {
    ui.push_id(("kotor_sheet", id), |ui| {
        let classes = cx.text(&join(base, "Classes")).unwrap_or_default();
        let level = cx.int(&join(base, "Level"));
        ui.horizontal_wrapped(|ui| {
            if !classes.is_empty() {
                ui.label(RichText::new(&classes).strong().color(crate::theme::fg(ui, GOLD)));
            }
            if let Some(l) = level {
                ui.label(format!("Level {l}"));
            }
        });
        ui.add_space(4.0);
        if base != "/Player" {
            ui.horizontal_wrapped(|ui| {
                cx.tile(ui, "Experience", &join(base, "Experience"), GOLD);
                cx.tile(ui, "Skill points", &join(base, "Skill points"), GOLD);
            });
            ui.add_space(6.0);
        }
        columns(ui, 360.0, 2, |ui, i| {
            if i == 0 {
                panel(ui, "Attributes", |ui| {
                    for k in ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"] {
                        cx.attribute(ui, k, &join(&join(base, "Attributes"), k), 30.0, BLUE);
                    }
                    ui.add_space(4.0);
                    cx.attribute(ui, "Alignment", &join(base, "Alignment"), 100.0, BLUE);
                    ui.weak("Alignment: 0 is the dark side, 100 the light side.");
                });
            } else {
                panel(ui, "Vitality and the Force", |ui| {
                    let h = join(base, "Health");
                    egui::Grid::new("kotor_hp").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("");
                        for h in ["Current", "Max", "Base"] {
                            ui.weak(h);
                        }
                        ui.end_row();
                        for (a, b, c) in [("Current HP", "Max HP", "Base HP"), ("Current FP", "Max FP", "Base FP")] {
                            ui.label(RichText::new(&a[8..]).strong().color(if a.ends_with("HP") { RED } else { BLUE }));
                            for k in [a, b, c] {
                                cx.compact(ui, &join(&h, k), 70.0);
                            }
                            ui.end_row();
                        }
                    });
                    ui.weak("Hit points and Force points; max includes bonuses, base is before them.");
                });
            }
        });
        let sk = join(base, "Skills");
        let names = keys(cx, &sk);
        if !names.is_empty() {
            panel(ui, "Skills", |ui| {
                columns(ui, 300.0, names.len(), |ui, i| {
                    cx.attribute(ui, &names[i], &join(&sk, &names[i]), 30.0, GOLD);
                });
                ui.weak("Ranks put into each skill (attribute bonuses and items add to them in the game).");
            });
        }
    });
}
