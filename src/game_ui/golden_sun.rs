//! Golden Sun and Golden Sun: The Lost Age (GBA): each save file's coins, the party's
//! stat cards and inventories, and a Djinn grid per element.

use super::{columns, fact, hero, inset_fill, meter, panel, tabs, GameCtx};
use crate::fmt::goldensun::{djinn_name, ELEMENTS, KIND_GS2};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

const ACCENT: Color32 = Color32::from_rgb(214, 160, 40);
/// Venus, Mercury, Mars, Jupiter.
const ELEMENT_COLORS: [Color32; 4] =
    [Color32::from_rgb(176, 128, 56), Color32::from_rgb(52, 128, 205), Color32::from_rgb(208, 72, 48), Color32::from_rgb(150, 92, 200)];

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let lost_age = cx.tree.is_some_and(|t| t.kind_name == KIND_GS2);
    let files: Vec<String> = keys(cx, "").into_iter().filter(|k| k.starts_with("File ")).collect();
    if files.is_empty() {
        ui.weak("No save file in this flash.");
        return;
    }
    let labels: Vec<String> = files
        .iter()
        .map(|f| {
            let leader = cx.text(&format!("{}/Leader", join("", f))).unwrap_or_default();
            if leader.is_empty() { f.clone() } else { format!("{f} \u{2014} {leader}") }
        })
        .collect();
    let t = if files.len() > 1 { tabs(ui, "gs_file", &labels) } else { 0 };
    let fp = join("", &files[t]);
    let sub = |k: &str| format!("{fp}/{k}");

    let title = if lost_age { "Golden Sun: The Lost Age" } else { "Golden Sun" };
    let mut line = vec![files[t].clone()];
    for k in ["Leader", "Version"] {
        if let Some(v) = cx.text(&sub(k)).filter(|v| !v.is_empty()) {
            line.push(v);
        }
    }
    if let Some(p) = cx.text(&sub("Play time")) {
        line.push(format!("{p} played"));
    }
    if let Some(n) = cx.int(&sub("Save count")) {
        line.push(format!("saved {n} times"));
    }
    let party = keys(cx, &sub("Party"));
    let per = if lost_age { 18 } else { 7 };
    let djinn_total: usize = ELEMENTS.iter().enumerate().map(|(e, el)| (0..per).filter(|&bit| djinn_holder(cx, &sub("Party"), &party, el, &djinn_name(e, bit)).is_some()).count()).sum();
    hero(ui, ACCENT, title, &line.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Coins", &sub("Coins"), ACCENT);
        fact(ui, "Party", &party.len().to_string(), ACCENT);
        fact(ui, "Djinn", &djinn_total.to_string(), ACCENT);
    });

    if party.is_empty() {
        ui.weak("Nobody is in the party in this file.");
        return;
    }
    let names: Vec<String> = party
        .iter()
        .map(|m| {
            let p = join(&sub("Party"), m);
            let n = cx.text(&format!("{p}/Name")).filter(|n| !n.is_empty()).unwrap_or_else(|| m.clone());
            format!("{n}  Lv {}", cx.int(&format!("{p}/Level")).unwrap_or(0))
        })
        .collect();
    panel(ui, "Party", |ui| {
        let i = tabs(ui, &format!("gs_member_{t}"), &names);
        let p = join(&sub("Party"), &party[i]);
        member(ui, cx, &p, &party[i]);
    });

    let story = keys(cx, &sub("Story"));
    if !story.is_empty() {
        panel(ui, "Story", |ui| {
            super::screens2::story_guard(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for k in &story {
                        cx.chip(ui, k, &join(&sub("Story"), k), ACCENT);
                    }
                });
                ui.weak("The choices the password carries into The Lost Age. Click one to turn it on or off; the scene behind it isn't replayed.");
            });
        });
    }

    panel(ui, "Djinn", |ui| djinn_grid(ui, cx, &sub("Party"), &party, if lost_age { 18 } else { 7 }));
    ui.weak(
        "Pick who holds each Djinni, set or on standby, or nobody. The game works out class and Psynergy from the set Djinn the next time you change Djinn in its own menu, \
         so open the Djinn menu and set one again after loading. Djinn the story hands out are not marked as collected in the world: the game may offer them again.",
    );
}

/// One party member: class and level, HP/PP, base and shown stats, and items.
fn member(ui: &mut Ui, cx: &mut GameCtx, p: &str, key: &str) {
    let at = |k: &str| format!("{p}/{k}");
    let elem = match key {
        "Isaac" | "Felix" => 0,
        "Mia" | "Piers" => 1,
        "Garet" | "Jenna" => 2,
        _ => 3,
    };
    let color = ELEMENT_COLORS[elem];
    let name = cx.text(&at("Name")).filter(|n| !n.is_empty()).unwrap_or_else(|| key.to_string());
    let class = cx.text(&at("Class")).unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&name).size(20.0).strong());
        ui.label(RichText::new(format!("{class}  \u{00B7}  {} adept", ELEMENTS[elem])).color(crate::theme::fg(ui, color)));
    });
    ui.horizontal_wrapped(|ui| {
        cx.tile(ui, "Level", &at("Level"), color);
        cx.tile(ui, "Experience", &at("Experience"), color);
    });
    ui.add_space(4.0);
    columns(ui, 330.0, 2, |ui, c| {
        egui::Frame::none().fill(inset_fill(ui, color)).rounding(10.0).inner_margin(10.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if c == 0 {
                ui.label(RichText::new("HP and PP").strong());
                for (k, max, bar) in [("HP", "Max HP", Color32::from_rgb(70, 170, 90)), ("PP", "Max PP", Color32::from_rgb(70, 120, 210))] {
                    let (cur, m) = (cx.num(&at(k)).unwrap_or(0.0), cx.num(&at(max)).unwrap_or(0.0));
                    meter(ui, k, cur, m, bar);
                    cx.row_fixed(ui, &format!("Current {k}"), &at(k), 110.0, 70.0);
                }
                ui.add_space(6.0);
                ui.label(RichText::new("Stats in the menu").strong());
                egui::Grid::new(("gs_shown", p)).num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                    for k in ["Attack", "Defense", "Agility", "Luck"] {
                        ui.label(k);
                        ui.label(cx.text(&format!("{p}/Stats/{k}")).unwrap_or_default());
                        ui.end_row();
                    }
                });
            } else {
                ui.label(RichText::new("Base stats").strong());
                for (k, max) in [("Max HP", 999.0), ("Max PP", 400.0), ("Attack", 400.0), ("Defense", 200.0), ("Agility", 400.0), ("Luck", 50.0)] {
                    cx.attribute(ui, k, &format!("{p}/Base stats/{k}"), max, color);
                }
                ui.weak("The game builds the menu stats from these, adding class, Djinn and equipment.");
            }
        });
    });
    ui.add_space(8.0);
    inventory(ui, cx, &at("Items"), color);
}

fn inventory(ui: &mut Ui, cx: &mut GameCtx, p: &str, color: Color32) {
    ui.label(RichText::new("Items").strong());
    let slots = keys(cx, p);
    egui::Grid::new(("gs_items", p)).num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
        for s in &slots {
            let sp = join(p, s);
            let item = format!("{sp}/Item");
            let equipped = cx.flag(&format!("{sp}/Equipped"));
            let mark = if equipped { "E" } else if cx.flag(&format!("{sp}/Broken")) { "B" } else { "" };
            ui.label(RichText::new(mark).strong().color(crate::theme::fg(ui, color))).on_hover_text(if equipped { "Equipped" } else { "" });
            cx.editor(ui, &item, 220.0);
            if cx.get(&format!("{sp}/Quantity")).is_some() && cx.text(&item).as_deref() != Some(crate::fmt::goldensun::EMPTY) {
                ui.horizontal(|ui| {
                    ui.label("\u{00D7}");
                    cx.editor(ui, &format!("{sp}/Quantity"), 48.0);
                });
            } else {
                ui.label("");
            }
            ui.end_row();
        }
    });
    ui.weak("E = equipped (not swapped here). Consumables can be swapped for other consumables and gear for other gear; the first empty place takes any item.");
}

/// Who has the Djinni at `bit` of element `el`, and whether it is set.
fn djinn_holder(cx: &GameCtx, party_p: &str, party: &[String], el: &str, dn: &str) -> Option<(usize, bool)> {
    party.iter().enumerate().find_map(|(i, m)| {
        let b = format!("{}/Djinn/{el}/{}", join(party_p, m), pointer_escape(dn));
        (cx.int(&format!("{b}/Owned")) == Some(1)).then(|| (i, cx.int(&format!("{b}/Set")) == Some(1)))
    })
}

/// For each element, every Djinn of the game: who has it and whether it is set, with a
/// list to hand it to someone (set or on standby) or to nobody.
fn djinn_grid(ui: &mut Ui, cx: &mut GameCtx, party_p: &str, party: &[String], per: usize) {
    let names: Vec<String> = party.iter().map(|m| cx.text(&format!("{}/Name", join(party_p, m))).filter(|n| !n.is_empty()).unwrap_or_else(|| m.clone())).collect();
    columns(ui, 250.0, 4, |ui, e| {
        let color = ELEMENT_COLORS[e];
        let el = ELEMENTS[e];
        egui::Frame::none().fill(inset_fill(ui, color)).rounding(8.0).inner_margin(8.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let owned = (0..per).filter(|&bit| djinn_holder(cx, party_p, party, el, &djinn_name(e, bit)).is_some()).count();
            ui.label(RichText::new(format!("{el}  {owned} / {per}")).strong().color(crate::theme::fg(ui, color)));
            for bit in 0..per {
                let dn = djinn_name(e, bit);
                let holder = djinn_holder(cx, party_p, party, el, &dn);
                let path = |m: &String, k: &str| format!("{}/Djinn/{el}/{}/{k}", join(party_p, m), pointer_escape(&dn));
                let changed = party.iter().any(|m| cx.modified(&path(m, "Owned")) || cx.modified(&path(m, "Set")));
                let mut pick = holder;
                ui.horizontal(|ui| {
                    let (fill, text) = match &holder {
                        Some((_, true)) => (color, Color32::WHITE),
                        Some(_) => (color.gamma_multiply(0.45), Color32::WHITE),
                        None => (ui.visuals().extreme_bg_color, ui.visuals().weak_text_color()),
                    };
                    egui::Frame::none().fill(fill).rounding(10.0).inner_margin(egui::Margin::symmetric(8.0, 2.0)).show(ui, |ui| {
                        ui.label(RichText::new(&dn).size(13.0).color(text));
                    });
                    let shown = match holder {
                        Some((i, set)) => format!("{} \u{00B7} {}", names[i], if set { "set" } else { "standby" }),
                        None => "\u{2014}".to_string(),
                    };
                    let shown = if changed { RichText::new(shown).size(12.0).color(crate::theme::MODIFIED) } else { RichText::new(shown).size(12.0) };
                    egui::ComboBox::from_id_salt(("gs_djinn", e, bit)).selected_text(shown).width(120.0).show_ui(ui, |ui| {
                        ui.selectable_value(&mut pick, None, "\u{2014} nobody");
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(&mut pick, Some((i, true)), format!("{n} \u{00B7} set"));
                            ui.selectable_value(&mut pick, Some((i, false)), format!("{n} \u{00B7} standby"));
                        }
                    });
                });
                if pick != holder {
                    // A Djinni belongs to one character at most.
                    for (i, m) in party.iter().enumerate() {
                        let (own, set) = match pick {
                            Some((j, set)) if j == i => (1, i64::from(set)),
                            _ => (0, 0),
                        };
                        for (k, v) in [("Owned", own), ("Set", set)] {
                            let p = path(m, k);
                            if cx.int(&p) != Some(v) && cx.can_edit(&p) {
                                cx.set(&p, Value::from(v));
                            }
                        }
                    }
                }
            }
        });
    });
}
