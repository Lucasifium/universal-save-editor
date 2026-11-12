//! Borderlands 2, The Pre-Sequel and Borderlands 3: the vault hunter, currencies, ammo
//! with its SDU upgrades, and the loot carried (shown, not edited).

use super::{bar, blend, fmt_num, panel, tabs, GameCtx};
use crate::fmt::borderlands::{ammo_capacity, MAX_AMMO_SDU};
use crate::json_view::pointer_escape;
use crate::theme;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

const YELLOW: Color32 = Color32::from_rgb(250, 196, 22);
const MONEY: Color32 = Color32::from_rgb(110, 190, 70);
const ERIDIUM: Color32 = Color32::from_rgb(170, 90, 230);
const MOONSTONE: Color32 = Color32::from_rgb(110, 190, 235);
const SERAPH: Color32 = Color32::from_rgb(235, 100, 170);
const TORGUE: Color32 = Color32::from_rgb(240, 130, 40);

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

/// The vault hunter banner: dark plate, yellow edge, hazard stripes, name in yellow.
fn banner(ui: &mut Ui, title: &str, subtitle: &str, add: impl FnOnce(&mut Ui)) {
    let dark = ui.visuals().dark_mode;
    let plate = if dark { Color32::from_rgb(28, 27, 24) } else { blend(ui.visuals().faint_bg_color, YELLOW, 0.16) };
    let resp = egui::Frame::none()
        .fill(plate)
        .stroke(egui::Stroke::new(1.5_f32, YELLOW.gamma_multiply(0.8)))
        .rounding(6.0)
        .inner_margin(egui::Margin { left: 22.0, right: 16.0, top: 18.0, bottom: 12.0 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title.to_uppercase()).family(theme::semibold_family()).size(30.0).color(theme::fg(ui, YELLOW)));
            if !subtitle.is_empty() {
                ui.label(RichText::new(subtitle.to_uppercase()).size(13.5).color(ui.visuals().strong_text_color()));
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(add);
        });
    let r = resp.response.rect;
    let p = ui.painter().with_clip_rect(r.shrink(1.0));
    // Yellow edge on the left and hazard stripes along the top right.
    p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(8.0, r.height())), egui::Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }, YELLOW);
    let band = egui::Rect::from_min_max(egui::pos2(r.max.x - r.width().min(260.0), r.min.y), egui::pos2(r.max.x, r.min.y + 9.0));
    p.rect_filled(band, 0.0, YELLOW);
    let stripe = Color32::from_rgb(28, 27, 24);
    let mut x = band.min.x - 9.0;
    while x < band.max.x {
        p.add(egui::Shape::convex_polygon(
            vec![egui::pos2(x, band.max.y), egui::pos2(x + 9.0, band.min.y), egui::pos2(x + 16.0, band.min.y), egui::pos2(x + 7.0, band.max.y)],
            stripe,
            egui::Stroke::NONE,
        ));
        x += 18.0;
    }
    ui.add_space(10.0);
}

fn currency_color(name: &str) -> Color32 {
    match name {
        "Money" => MONEY,
        "Eridium" => ERIDIUM,
        "Moonstone" => MOONSTONE,
        "Seraph crystals" => SERAPH,
        "Torgue tokens" => TORGUE,
        _ => YELLOW,
    }
}

fn header(ui: &mut Ui, cx: &mut GameCtx, game: &str) {
    let name = cx.text("/Character/Name").filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Vault Hunter".into());
    let mut sub = vec![];
    if let Some(l) = cx.int("/Character/Level") {
        sub.push(format!("Level {l}"));
    }
    if let Some(c) = cx.text("/Character/Class").filter(|c| !c.is_empty()) {
        sub.push(c);
    }
    if let Some(v) = cx.text("/Character/Vault hunter").filter(|c| !c.is_empty() && !c.eq_ignore_ascii_case(&name)) {
        sub.push(v);
    }
    sub.push(game.to_string());
    if let Some(p) = cx.int("/Character/Playthroughs completed").filter(|&p| p > 0) {
        sub.push(format!("{p} playthrough{} done", if p == 1 { "" } else { "s" }));
    }
    banner(ui, &name, &sub.join("  \u{00B7}  "), |ui| {
        for (label, p) in [("Level", "/Character/Level"), ("Experience", "/Character/Experience"), ("Skill points", "/Character/Skill points")] {
            cx.tile(ui, label, p, YELLOW);
        }
    });
    panel(ui, "Currency", |ui| {
        ui.horizontal_wrapped(|ui| {
            for k in keys(cx, "/Currency") {
                cx.tile(ui, &k, &join("/Currency", &k), currency_color(&k));
            }
        });
    });
}

/// Ammo per weapon type next to its SDU level; with `capacity`, a bar of how full it is.
fn ammo_panel(ui: &mut Ui, cx: &mut GameCtx, capacity: bool, note: &str) {
    let ammo = keys(cx, "/Ammo");
    let sdus = keys(cx, "/SDU upgrades");
    let mut names: Vec<String> = ammo.clone();
    names.extend(sdus.iter().filter(|s| !ammo.contains(s)).cloned());
    if names.is_empty() {
        return;
    }
    panel(ui, "Ammo and SDUs", |ui| {
        let narrow = ui.available_width() < 560.0;
        egui::Grid::new("bl_ammo").num_columns(if narrow { 3 } else { 4 }).spacing([14.0, 6.0]).striped(true).show(ui, |ui| {
            ui.strong("");
            ui.strong("Ammo");
            ui.strong("SDU level");
            if !narrow {
                ui.strong("");
            }
            ui.end_row();
            for n in &names {
                let ap = join("/Ammo", n);
                let sp = join("/SDU upgrades", n);
                ui.label(RichText::new(n).color(ui.visuals().strong_text_color()));
                ui.horizontal(|ui| cx.compact(ui, &ap, 70.0));
                ui.horizontal(|ui| cx.compact(ui, &sp, 44.0));
                if !narrow {
                    let cap = if capacity { cx.int(&sp).and_then(|lv| ammo_capacity(n, lv)) } else { None };
                    match (cap, cx.num(&ap)) {
                        (Some(cap), Some(a)) => bar(ui, 200.0, (a / cap as f64) as f32, YELLOW.gamma_multiply(0.85), Some(&format!("{} / {}", fmt_num(a), fmt_num(cap as f64)))),
                        _ => {
                            ui.label("");
                        }
                    }
                }
                ui.end_row();
            }
        });
        if capacity {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("Fill all ammo").on_hover_text("Set each ammo type to what its SDU level holds (without relic or class mod bonuses).").clicked() {
                    for n in &ammo {
                        let ap = join("/Ammo", n);
                        if let Some(cap) = cx.int(&join("/SDU upgrades", n)).and_then(|lv| ammo_capacity(n, lv)) {
                            if cx.can_edit(&ap) && cx.num(&ap) != Some(f64::from(cap)) {
                                cx.set(&ap, cap.into());
                            }
                        }
                    }
                }
                if ui.button(format!("Max ammo SDUs ({MAX_AMMO_SDU})")).on_hover_text("Every ammo SDU to the highest black market level.").clicked() {
                    for n in &sdus {
                        let sp = join("/SDU upgrades", n);
                        if cx.can_edit(&sp) && cx.int(&sp) != Some(MAX_AMMO_SDU) {
                            cx.set(&sp, MAX_AMMO_SDU.into());
                        }
                    }
                }
            });
        }
        ui.weak(note);
    });
}

/// A read-only table of loot: one row per entry, the given columns.
fn loot_table(ui: &mut Ui, cx: &GameCtx, id: &str, base: &str, cols: &[&str]) {
    let n = cx.get(base).and_then(Value::as_array).map_or(0, Vec::len);
    if n == 0 {
        ui.weak("Nothing here.");
        return;
    }
    egui::ScrollArea::vertical().id_salt(id).max_height(360.0).show(ui, |ui| {
        egui::Grid::new((id, "grid")).num_columns(cols.len() + 1).spacing([16.0, 4.0]).striped(true).show(ui, |ui| {
            ui.strong("#");
            for c in cols {
                ui.strong(*c);
            }
            ui.end_row();
            for i in 0..n {
                ui.label(format!("{}", i + 1));
                for c in cols {
                    let v = cx.get(&format!("{base}/{i}/{}", pointer_escape(c)));
                    let text = match (c, v) {
                        (&"Level", Some(v)) if v.as_i64() == Some(0) => "\u{2014}".to_string(),
                        (&"Quick slot", Some(v)) if v.as_i64() == Some(0) => "backpack".to_string(),
                        (_, Some(Value::Bool(b))) => if *b { "yes" } else { "" }.to_string(),
                        (_, Some(Value::String(s))) => s.clone(),
                        (_, Some(v)) => v.to_string(),
                        (_, None) => String::new(),
                    };
                    let color = match (c, text.as_str()) {
                        (&"Serial check", "OK") => theme::OK,
                        (&"Serial check", _) => theme::ERR,
                        (&"Slot", s) if s != "Backpack" => theme::fg(ui, YELLOW),
                        (&"Quick slot", s) if s != "backpack" => theme::fg(ui, YELLOW),
                        (&"Equipped", "yes") => theme::fg(ui, YELLOW),
                        _ => ui.visuals().text_color(),
                    };
                    ui.label(RichText::new(text).color(color));
                }
                ui.end_row();
            }
        });
    });
}

/// Borderlands 2 and The Pre-Sequel.
pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let game = cx.text("/Character/Game").unwrap_or_else(|| "Borderlands 2".into());
    header(ui, cx, &game);
    ammo_panel(
        ui,
        cx,
        true,
        "Bars show the ammo each SDU level holds before relic and class mod bonuses (the game allows more with those).",
    );
    if cx.get("/SDU upgrades/Backpack").is_some() || cx.get("/Character/Weapon slots").is_some() {
        panel(ui, "Storage and playthroughs", |ui| {
            egui::Grid::new("bl2_storage").num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
                for (label, p, room, per) in [("Backpack SDU", "/SDU upgrades/Backpack", "/Character/Backpack room", "slots"), ("Bank SDU", "/SDU upgrades/Bank", "/Character/Bank room", "slots")] {
                    if cx.get(p).is_some() {
                        cx.label(ui, label, p);
                        ui.horizontal(|ui| cx.editor(ui, p, 56.0));
                        let lv = cx.int(p).unwrap_or(0);
                        let now = if label.starts_with("Backpack") { crate::fmt::borderlands::backpack_slots(lv) } else { crate::fmt::borderlands::bank_slots(lv) };
                        let saved = cx.int(room);
                        ui.weak(if cx.modified(p) || saved.is_none() { format!("{now} {per} when saved") } else { format!("{} {per}", saved.unwrap()) });
                        ui.end_row();
                    }
                }
                cx.grid_row(ui, "Weapon slots", "/Character/Weapon slots", 56.0);
                cx.grid_row(ui, "Playthroughs completed", "/Character/Playthroughs completed", 56.0);
            });
            ui.weak(format!(
                "Backpack and bank SDUs go from 0 to {}: the backpack holds 12 items plus 3 a level, the bank 6 plus 2 a level. Weapon slots go from 2 to 4. \
                 Playthroughs completed 1 opens True Vault Hunter Mode and 2 opens Ultimate Vault Hunter Mode.",
                crate::fmt::borderlands::MAX_STORAGE_SDU
            ));
        });
    }
    let nw = cx.get("/Weapons").and_then(Value::as_array).map_or(0, Vec::len);
    let ni = cx.get("/Items").and_then(Value::as_array).map_or(0, Vec::len);
    panel(ui, "Loot", |ui| {
        let t = tabs(ui, "bl2_loot", &[format!("Weapons ({nw})"), format!("Items ({ni})")]);
        if t == 0 {
            loot_table(ui, cx, "bl2_w", "/Weapons", &["Level", "Quick slot", "Serial check"]);
        } else {
            loot_table(ui, cx, "bl2_i", "/Items", &["Level", "Equipped", "Quantity", "Serial check"]);
        }
        let bank = cx.int("/Bank").unwrap_or(0);
        ui.weak(format!(
            "Gear is read from each item's serial (level and its check); names need the game's part tables, so they aren't shown, and gear isn't edited. {bank} item(s) in the bank."
        ));
    });
}

/// Borderlands 3.
pub fn show3(ui: &mut Ui, cx: &mut GameCtx) {
    header(ui, cx, "Borderlands 3");
    ammo_panel(ui, cx, false, "SDU levels go up to 10 (13 for the backpack, sniper rifles and heavy weapons). Ammo above what the SDUs hold is cut back by the game.");
    let n = cx.get("/Inventory").and_then(Value::as_array).map_or(0, Vec::len);
    panel(ui, &format!("Loot ({n})"), |ui| {
        loot_table(ui, cx, "bl3_inv", "/Inventory", &["Slot", "Favorite", "Junk", "Serial version", "Serial check"]);
        ui.weak("Gear is shown with its slot, marks and serial check; its parts need the game's item database, so it isn't edited here. Level follows experience: changing one sets the other.");
    });
}

/// Tiny Tina's Wonderlands.
pub fn show_wl(ui: &mut Ui, cx: &mut GameCtx) {
    header(ui, cx, "Tiny Tina's Wonderlands");
    ammo_panel(ui, cx, false, "Ammo above what the character can hold is cut back by the game. Level goes up to 40.");
    let n = cx.get("/Inventory").and_then(Value::as_array).map_or(0, Vec::len);
    panel(ui, &format!("Loot ({n})"), |ui| {
        loot_table(ui, cx, "wl_inv", "/Inventory", &["Slot", "Favorite", "Junk", "Serial version", "Serial check"]);
        ui.weak("Gear is shown with its slot, marks and serial check; its parts need the game's item database, so it isn't edited here. Level follows experience: changing one sets the other.");
    });
}
