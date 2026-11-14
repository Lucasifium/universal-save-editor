//! Deus Ex (2000): JC Denton from a map's .dxs file, or where to find him from SaveInfo.dxs.

use crate::i18n::{tr};
use super::{columns, hero, panel, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

const GOLD: Color32 = Color32::from_rgb(214, 160, 60);
const HEALTH: Color32 = Color32::from_rgb(200, 70, 60);

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn subtitle(cx: &GameCtx) -> String {
    let mut sub = vec![];
    for p in ["/Save/Location", "/Save/Map", "/Save/Saved on"] {
        if let Some(t) = cx.text(p).filter(|t| !t.is_empty()) {
            if !sub.contains(&t) {
                sub.push(t);
            }
        }
    }
    sub.join("  \u{00B7}  ")
}

fn notice(ui: &mut Ui, text: &str) {
    egui::Frame::none().fill(super::blend(ui.visuals().faint_bg_color, GOLD, 0.25)).rounding(8.0).inner_margin(egui::Margin::same(12.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(text).size(14.0).color(ui.visuals().strong_text_color()));
    });
    ui.add_space(8.0);
}

/// "Label [editor]" rows for every key under `base`, in columns.
fn rows(ui: &mut Ui, cx: &mut GameCtx, base: &str, id: &str) {
    let ks = keys(cx, base);
    ui.push_id(id, |ui| {
        columns(ui, 300.0, ks.len(), |ui, i| {
            let label_w = (ui.available_width() - 120.0).clamp(90.0, 200.0);
            cx.row_fixed(ui, &ks[i], &join(base, &ks[i]), label_w, 90.0);
        });
    });
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    if let Some(file) = cx.text("/Save/Open to edit JC") {
        hero(ui, GOLD, &cx.text("/Save/Description").unwrap_or_else(|| "Deus Ex save".into()), &subtitle(cx), |ui| {
            for (label, p) in [("Save count", "/Save/Save count"), ("Play time (seconds)", "/Save/Play time (seconds)")] {
                if let Some(v) = cx.text(p) {
                    super::fact(ui, label, &v, GOLD);
                }
            }
        });
        notice(ui, &format!("This is SaveInfo.dxs, the save's label. JC Denton is in {file} in the same Save folder: open that file to change his credits, skills and inventory."));
        return;
    }
    let live = cx.flag("/Save/Live player");
    if !live {
        notice(
            ui,
            "This map file only holds the copy of JC Denton from when he last left this map; the game loads him from the map you were on when saving \
             (its name is in SaveInfo.dxs in the same folder). Open that map's .dxs file to edit him; this copy is shown read-only.",
        );
    }
    hero(ui, GOLD, "JC Denton", &subtitle(cx), |ui| {
        for (label, p) in [
            ("Credits", "/JC Denton/Credits"),
            ("Skill points", "/JC Denton/Skill points"),
            ("Skill points earned", "/JC Denton/Skill points earned"),
            ("Bioelectric energy", "/JC Denton/Bioelectric energy"),
            ("Max bioelectric energy", "/JC Denton/Bioelectric energy max"),
        ] {
            if cx.get(p).is_some_and(Value::is_number) {
                cx.tile(ui, label, p, GOLD);
            } else if let Some(t) = cx.text(p) {
                super::fact(ui, label, &t, GOLD);
            }
        }
    });

    columns(ui, 380.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Health", |ui| {
                for k in keys(cx, "/Health") {
                    cx.attribute(ui, &k, &join("/Health", &k), 100.0, HEALTH);
                }
                if keys(cx, "/Health").iter().any(|k| !cx.can_edit(&join("/Health", k))) && live {
                    ui.weak(tr("Body parts at full health (100) aren't written in the save, so they can't be changed here."));
                }
            });
            panel(ui, "Items", |ui| {
                rows(ui, cx, "/Items", "dx_items");
                ui.weak(tr("Medkits, lockpicks, multitools, bioelectric cells and other items carried. A single item's count isn't written in the save and can't be changed."));
            });
        } else {
            panel(ui, "Skills", |ui| {
                rows(ui, cx, "/Skills", "dx_skills");
                if live {
                    ui.weak(tr("Untrained skills aren't written in the save, so they can't be raised here (spend skill points in the game)."));
                }
            });
            panel(ui, "Augmentations", |ui| {
                if keys(cx, "/Augmentations").is_empty() {
                    ui.label(tr("None installed."));
                }
                rows(ui, cx, "/Augmentations", "dx_augs");
                if live {
                    ui.weak(tr("Installed augmentations. Level 1 isn't written in the save, so those can't be upgraded here."));
                }
            });
        }
    });
    panel(ui, "Ammo", |ui| {
        rows(ui, cx, "/Ammo", "dx_ammo");
        ui.weak(tr("Rounds carried for each ammo type. Amounts the game left at their default aren't in the save and can't be changed."));
    });
    if let Some(w) = cx.text("/Weapons") {
        panel(ui, "Weapons", |ui| {
            ui.label(w);
        });
    }
}
