//! Fallout and Fallout 2 (SAVE.DAT): the character sheet on a Pip-Boy green screen.

use super::{columns, panel, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

const GREEN: Color32 = Color32::from_rgb(80, 235, 100);
const DIM: Color32 = Color32::from_rgb(46, 150, 60);
const SCREEN: Color32 = Color32::from_rgb(6, 20, 8);
const CARD: Color32 = Color32::from_rgb(12, 36, 15);

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

/// Green-on-black colours for everything drawn inside.
fn pipboy_visuals(ui: &mut Ui) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(GREEN);
    v.faint_bg_color = CARD;
    v.extreme_bg_color = Color32::from_rgb(3, 12, 4);
    v.panel_fill = SCREEN;
    v.window_fill = SCREEN;
    v.selection.bg_fill = Color32::from_rgb(30, 100, 40);
    v.selection.stroke.color = GREEN;
    v.hyperlink_color = GREEN;
    let w = &mut v.widgets;
    for (s, fill) in [(&mut w.noninteractive, CARD), (&mut w.inactive, Color32::from_rgb(16, 52, 20)), (&mut w.hovered, Color32::from_rgb(26, 80, 32)), (&mut w.active, Color32::from_rgb(34, 104, 42)), (&mut w.open, Color32::from_rgb(20, 64, 26))] {
        s.bg_fill = fill;
        s.weak_bg_fill = fill;
        s.fg_stroke.color = GREEN;
        s.bg_stroke.color = DIM;
    }
    w.noninteractive.fg_stroke.color = GREEN;
    ui.style_mut().visuals = v;
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    egui::Frame::none().fill(SCREEN).stroke(egui::Stroke::new(2.0_f32, DIM)).rounding(12.0).inner_margin(egui::Margin::same(14.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        pipboy_visuals(ui);
        sheet(ui, cx);
    });
}

fn sheet(ui: &mut Ui, cx: &mut GameCtx) {
    let game = cx.text("/Save/Game").unwrap_or_else(|| "Fallout".into());
    let name = cx.text("/Save/Character name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Vault Dweller".into());
    let mut sub = vec![game.clone()];
    for p in ["/Character/Gender", "/Save/Map", "/Save/Game date"] {
        if let Some(t) = cx.text(p).filter(|t| !t.is_empty()) {
            sub.push(t);
        }
    }
    if let Some(a) = cx.int("/Character/Age") {
        sub.insert(2, format!("age {a}"));
    }
    ui.label(RichText::new(format!("{game} \u{2014} PERSONNEL RECORD")).monospace().size(12.0).color(DIM));
    ui.label(RichText::new(name.to_uppercase()).monospace().size(28.0).strong());
    ui.label(RichText::new(sub.join("  \u{00B7}  ")).monospace().size(13.0));
    if let Some(d) = cx.text("/Save/Description").filter(|d| !d.is_empty()) {
        ui.label(RichText::new(format!("\u{201C}{d}\u{201D}")).monospace().size(12.0).color(DIM));
    }
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        for (label, p) in [("Level", "/Character/Level"), ("Experience", "/Character/Experience"), ("Skill points", "/Character/Skill points"), ("Hit points", "/Character/Hit points"), ("Bottle caps", "/Money/Bottle caps")] {
            cx.tile(ui, label, p, GREEN);
        }
    });
    if cx.get("/Money/Bottle caps").is_none() {
        ui.label(RichText::new("No bottle caps in the inventory: there's no stack to change (the game adds one when you pick some up).").size(12.0).color(DIM));
    }
    ui.add_space(8.0);

    columns(ui, 380.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "S.P.E.C.I.A.L.", |ui| {
                for k in keys(cx, "/S.P.E.C.I.A.L.") {
                    let p = join("/S.P.E.C.I.A.L.", &k);
                    let bonus = cx.int(&join("/S.P.E.C.I.A.L. bonus", &k)).unwrap_or(0);
                    let label = if bonus != 0 { format!("{k} ({bonus:+})") } else { k.clone() };
                    cx.attribute(ui, &label, &p, 10.0, GREEN);
                }
                ui.add_space(4.0);
                ui.label(RichText::new("Base values, 1\u{2013}10. The bonus in brackets comes from traits, perks, drugs and armour and is kept separately.").size(12.0).color(DIM));
            });
            panel(ui, "Status", |ui| {
                egui::Grid::new("fo_status").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    cx.grid_row(ui, "Max hit points (base)", "/Character/Max hit points (base)", 70.0);
                    for (label, p) in [("Max hit points", "/Character/Max hit points"), ("Radiation", "/Character/Radiation"), ("Poison", "/Character/Poison")] {
                        if let Some(v) = cx.text(p) {
                            ui.label(label);
                            ui.label(v);
                            ui.end_row();
                        }
                    }
                });
                ui.label(
                    RichText::new("Derived values (max hit points, action points, carry weight\u{2026}) are stored as their own numbers: changing S.P.E.C.I.A.L. doesn't change them.")
                        .size(12.0)
                        .color(DIM),
                );
            });
        } else {
            let tagged = cx.text("/Tagged skills").unwrap_or_default();
            panel(ui, "Skills", |ui| {
                for k in keys(cx, "/Skills") {
                    let tag = tagged.split(", ").any(|t| t == k);
                    let label = if tag { format!("\u{25C6} {k}") } else { k.clone() };
                    cx.row_fixed(ui, &label, &join("/Skills", &k), 150.0, 70.0);
                }
                ui.add_space(4.0);
                ui.label(RichText::new("Points put into each skill (the sheet adds the starting value from S.P.E.C.I.A.L. on top; tagged skills \u{25C6} count them twice).").size(12.0).color(DIM));
            });
        }
    });

    columns(ui, 380.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Traits", |ui| {
                let t = cx.text("/Traits").unwrap_or_default();
                chips(ui, &if t.is_empty() { vec![] } else { t.split(", ").map(str::to_string).collect() }, "No traits");
            });
        } else {
            panel(ui, "Perks", |ui| {
                let perks: Vec<String> = keys(cx, "/Perks")
                    .into_iter()
                    .map(|k| match cx.int(&join("/Perks", &k)) {
                        Some(r) if r > 1 => format!("{k} ({r})"),
                        _ => k,
                    })
                    .collect();
                chips(ui, &perks, "No perks yet");
            });
        }
    });
    ui.label(RichText::new("Traits and perks are shown only: the game applies their effects when they're picked, so they're not changed here.").size(12.0).color(DIM));
    ui.add_space(8.0);

    let items = keys(cx, "/Inventory");
    panel(ui, &format!("Inventory ({} stacks)", items.len()), |ui| {
        if items.is_empty() {
            ui.label("Nothing carried.");
            return;
        }
        let q = filter(ui);
        egui::ScrollArea::vertical().id_salt("fo_inv").max_height(420.0).show(ui, |ui| {
            egui::Grid::new("fo_inv_g").num_columns(3).striped(true).spacing([16.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new("Item").strong());
                ui.label(RichText::new("Count").strong());
                ui.label(RichText::new("Loaded").strong());
                ui.end_row();
                for k in items.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)) {
                    let base = join("/Inventory", k);
                    let mut label = k.clone();
                    if let Some(w) = cx.text(&join(&base, "Loaded with")) {
                        label = format!("{label}  [{w}]");
                    }
                    ui.label(label);
                    cx.compact(ui, &join(&base, "Count"), 90.0);
                    if cx.get(&join(&base, "Loaded rounds")).is_some() {
                        cx.compact(ui, &join(&base, "Loaded rounds"), 60.0);
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
        });
        ui.label(
            RichText::new(
                "Item names come from a short list the game's own code refers to; other items show their prototype number (the names are in the game's data files). \u{21B3} marks items inside a container above.",
            )
            .size(12.0)
            .color(DIM),
        );
    });

    let kills = keys(cx, "/Kills");
    if !kills.is_empty() {
        egui::CollapsingHeader::new(RichText::new("Kills").strong()).id_salt("fo_kills").show(ui, |ui| {
            columns(ui, 260.0, kills.len(), |ui, i| {
                cx.row_fixed(ui, &kills[i], &join("/Kills", &kills[i]), 120.0, 70.0);
            });
        });
    }
}

fn chips(ui: &mut Ui, list: &[String], empty: &str) {
    if list.is_empty() {
        ui.label(RichText::new(empty).color(DIM));
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for t in list {
            egui::Frame::none().fill(Color32::from_rgb(20, 64, 26)).rounding(10.0).inner_margin(egui::Margin::symmetric(8.0, 3.0)).show(ui, |ui| {
                ui.label(RichText::new(t).size(13.0));
            });
        }
    });
}

fn filter(ui: &mut Ui) -> String {
    let id = egui::Id::new("fo_inv_filter");
    let mut q = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add(egui::TextEdit::singleline(&mut q).hint_text("item name").desired_width(200.0));
    });
    ui.data_mut(|d| d.insert_temp(id, q.clone()));
    q.to_lowercase()
}
