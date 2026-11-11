//! Pokémon Red/Blue/Yellow, Gold/Silver/Crystal and Ruby/Sapphire/Emerald/FireRed/
//! LeafGreen: trainer card, badges, party, PC boxes and bag.

use super::{cell_grid, columns, fact, hero, meter, panel, select, selection, tabs, Cell, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

const ACCENT: Color32 = Color32::from_rgb(220, 70, 70);
const NONE: &str = "(none)";

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
    let name = cx.text("/Trainer/Name").unwrap_or_else(|| "Trainer".into());
    let mut sub = vec![];
    if let Some(id) = cx.int("/Trainer/Trainer ID") {
        sub.push(format!("ID No. {id:05}"));
    }
    if let Some(g) = cx.text("/Trainer/Gender") {
        sub.push(g);
    }
    let time: Vec<i64> = ["hours", "minutes"].iter().filter_map(|u| cx.int(&format!("/Trainer/Play time ({u})"))).collect();
    if let [h, m] = time[..] {
        sub.push(format!("{h}:{m:02} played"));
    }
    if let Some(k) = cx.tree.map(|t| t.kind_name.clone()) {
        sub.push(k);
    }
    hero(ui, ACCENT, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Money", "/Trainer/Money", ACCENT);
        cx.tile(ui, "Coins", "/Trainer/Coins", ACCENT);
        let n = keys(cx, "/Party").len();
        fact(ui, "Party", &format!("{n} / 6"), ACCENT);
    });

    let mut rest: Vec<String> = keys(cx, "/Trainer").into_iter().filter(|k| !matches!(k.as_str(), "Name" | "Trainer ID" | "Gender" | "Money" | "Coins")).collect();
    rest.insert(0, "Name".into());
    panel(ui, "Trainer", |ui| {
        egui::Grid::new("pk_trainer").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
            for k in ["Name", "Gender"].iter().map(|s| s.to_string()).chain(rest.into_iter().skip(1)) {
                cx.grid_row(ui, &k, &join("/Trainer", &k), 160.0);
            }
        });
    });

    let badges = keys(cx, "/Badges");
    if !badges.is_empty() {
        panel(ui, "Badges", |ui| {
            ui.horizontal_wrapped(|ui| {
                for b in &badges {
                    cx.chip(ui, b, &join("/Badges", b), Color32::from_rgb(200, 160, 40));
                }
            });
        });
    }

    let party = keys(cx, "/Party");
    panel(ui, "Party", |ui| {
        if party.is_empty() {
            ui.weak("No Pokémon in the party.");
        }
        columns(ui, 330.0, party.len(), |ui, i| {
            mon_card(ui, cx, &join("/Party", &party[i]), &format!("party{i}"));
        });
    });

    let boxes = keys(cx, "/PC boxes");
    if !boxes.is_empty() {
        panel(ui, "PC boxes", |ui| {
            let labels: Vec<String> = boxes
                .iter()
                .map(|b| {
                    let n = keys(cx, &join("/PC boxes", b)).iter().filter(|k| k.starts_with("Slot") && !mon_empty(cx, &join(&join("/PC boxes", b), k))).count();
                    let cur = cx.flag(&format!("{}/Current box", join("/PC boxes", b)));
                    format!("{b}{}{}", if n > 0 { format!(" ({n})") } else { String::new() }, if cur { " \u{2022}" } else { "" })
                })
                .collect();
            let t = tabs(ui, "pk_box_tab", &labels);
            let bp = join("/PC boxes", &boxes[t]);
            let slots: Vec<String> = keys(cx, &bp).into_iter().filter(|k| k.starts_with("Slot")).collect();
            if slots.is_empty() {
                ui.weak("This box is empty.");
                return;
            }
            let cells: Vec<Cell> = slots
                .iter()
                .map(|s| {
                    let p = join(&bp, s);
                    let empty = mon_empty(cx, &p);
                    let title = if empty { String::new() } else { format!("{}\nLv {}", cx.text(&format!("{p}/Species")).unwrap_or_default(), cx.int(&format!("{p}/Level")).unwrap_or(0)) };
                    Cell { title, count: None, empty, modified: false, tint: cx.flag(&format!("{p}/Shiny")).then(|| Color32::from_rgb(80, 70, 20)) }
                })
                .collect();
            let id = format!("pk_box_sel_{t}");
            let sel = selection(ui, &id);
            if let Some(i) = cell_grid(ui, &cells, 6, sel, ACCENT) {
                select(ui, &id, if sel == Some(i) { None } else { Some(i) });
            }
            match selection(ui, &id).filter(|&i| i < slots.len()) {
                Some(i) => mon_card(ui, cx, &join(&bp, &slots[i]), &id),
                None => {
                    ui.weak("Click a Pokémon to edit it.");
                }
            }
        });
    }

    let pockets = keys(cx, "/Bag");
    if !pockets.is_empty() {
        panel(ui, "Bag", |ui| {
            let t = tabs(ui, "pk_bag_tab", &pockets);
            bag_pocket(ui, cx, &join("/Bag", &pockets[t]), t);
        });
    }
}

fn mon_empty(cx: &GameCtx, p: &str) -> bool {
    cx.text(&format!("{p}/Species")).is_none_or(|s| s.is_empty() || s == NONE)
}

/// One Pokémon: species, level and nickname up top, then moves and stats.
fn mon_card(ui: &mut Ui, cx: &mut GameCtx, p: &str, id: &str) {
    let sub = |k: &str| format!("{p}/{}", pointer_escape(k));
    let sub2 = |a: &str, b: &str| format!("{p}/{}/{}", pointer_escape(a), pointer_escape(b));
    egui::Frame::none().fill(super::inset_fill(ui, ACCENT)).rounding(10.0).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        if mon_empty(cx, p) {
            ui.weak("(empty slot)");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.style_mut().override_font_id = Some(egui::FontId::proportional(16.0));
            cx.editor(ui, &sub("Species"), 170.0);
            ui.label("Lv");
            cx.editor(ui, &sub("Level"), 44.0);
        });
        ui.horizontal_wrapped(|ui| {
            for (k, c) in [("Shiny", Color32::from_rgb(200, 160, 40)), ("Egg", Color32::from_rgb(120, 160, 120))] {
                if cx.get(&sub(k)).is_some() {
                    cx.chip(ui, k, &sub(k), c);
                }
            }
            for k in ["Gender", "Type", "Nature", "Ability"] {
                if let Some(v) = cx.text(&sub(k)) {
                    if !cx.can_edit(&sub(k)) {
                        ui.label(RichText::new(v).size(12.5).color(ui.visuals().weak_text_color()));
                    }
                }
            }
        });
        if let (Some(c), Some(m)) = (cx.num(&sub2("Stats", "Current HP")), cx.num(&sub2("Stats", "Max HP")).filter(|&m| m > 0.0)) {
            meter(ui, "HP", c, m, Color32::from_rgb(80, 180, 90));
        }
        egui::Grid::new((id, "main")).num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            cx.grid_row(ui, "Nickname", &sub("Nickname"), 150.0);
            cx.grid_row(ui, "Held item", &sub("Held item"), 150.0);
            for k in ["Gender", "Nature", "Ability"] {
                if cx.can_edit(&sub(k)) {
                    cx.grid_row(ui, k, &sub(k), 150.0);
                }
            }
        });
        // Moves: name, PP and PP Ups side by side.
        if cx.get(&sub("Moves")).is_some() {
            ui.add_space(4.0);
            ui.label(RichText::new("Moves").strong());
            egui::Grid::new((id, "moves")).num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                for i in 1..=4 {
                    let mv = sub2("Moves", &format!("Move {i}"));
                    if cx.get(&mv).is_none() {
                        continue;
                    }
                    cx.editor(ui, &mv, 140.0);
                    ui.horizontal(|ui| {
                        ui.weak("PP");
                        cx.editor(ui, &sub2("Moves", &format!("PP {i}")), 36.0);
                    });
                    ui.horizontal(|ui| {
                        if cx.get(&sub2("Moves", &format!("PP Ups {i}"))).is_some() {
                            ui.weak("Ups");
                            cx.editor(ui, &sub2("Moves", &format!("PP Ups {i}")), 30.0);
                        }
                    });
                    ui.end_row();
                }
            });
        }
        egui::CollapsingHeader::new("Stats, IVs and more").id_salt((id, "more")).show(ui, |ui| {
            let groups: Vec<String> = keys(cx, p).into_iter().filter(|k| matches!(cx.get(&sub(k)), Some(Value::Object(_))) && k != "Moves").collect();
            for g in groups {
                ui.label(RichText::new(&g).strong());
                egui::Grid::new((id, g.as_str())).num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                    for k in keys(cx, &sub(&g)) {
                        cx.grid_row(ui, &k, &format!("{}/{}", sub(&g), pointer_escape(&k)), 90.0);
                    }
                });
            }
            let skip = ["Species", "Level", "Nickname", "Held item", "Shiny", "Egg", "Gender", "Type", "Nature", "Ability"];
            let other: Vec<String> = keys(cx, p).into_iter().filter(|k| !skip.contains(&k.as_str()) && !matches!(cx.get(&sub(k)), Some(Value::Object(_)))).collect();
            if !other.is_empty() {
                ui.label(RichText::new("Other").strong());
                egui::Grid::new((id, "other")).num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                    for k in other {
                        cx.grid_row(ui, &k, &sub(&k), 120.0);
                    }
                });
            }
        });
    });
    ui.add_space(6.0);
}

/// A bag pocket: item slots (item + quantity), or TMs (name → count).
fn bag_pocket(ui: &mut Ui, cx: &mut GameCtx, p: &str, t: usize) {
    let entries = keys(cx, p);
    let slots: Vec<&String> = entries.iter().filter(|k| matches!(cx.get(&join(p, k)), Some(Value::Object(_)))).collect();
    if slots.is_empty() {
        // TMs & HMs and similar: a count per name.
        egui::Grid::new(("pk_bag_counts", t)).num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
            for (i, k) in entries.iter().enumerate() {
                cx.label(ui, k, &join(p, k));
                cx.editor(ui, &join(p, k), 44.0);
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        return;
    }
    // The games keep pockets gap-free, so show the filled slots and the first free one.
    let filled = slots.iter().take_while(|k| cx.text(&format!("{}/Item", join(p, k))).is_some_and(|i| i != NONE)).count();
    egui::Grid::new(("pk_bag", t)).num_columns(3).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
        for k in slots.iter().take(filled + 1) {
            let sp = join(p, k);
            ui.weak(k.as_str());
            cx.editor(ui, &format!("{sp}/Item"), 200.0);
            ui.horizontal(|ui| {
                if cx.get(&format!("{sp}/Quantity")).is_some() {
                    ui.weak("\u{00D7}");
                    cx.editor(ui, &format!("{sp}/Quantity"), 50.0);
                }
            });
            ui.end_row();
        }
    });
    ui.weak(format!("{filled} of {} slots used. Pick an item in the free slot to add one.", slots.len()));
}
