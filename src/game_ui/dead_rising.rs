//! Dead Rising (2016 PC), Dead Rising 2 and Off the Record: the survivor's level, PP and
//! level-up stats.

use crate::i18n::{tr, trf};
use super::{hero, panel, GameCtx};
use crate::fmt::dead_rising::{DR1_ALL_SKILLS, DR2_STAT_NAMES, KIND_OTR};
use eframe::egui::{Color32, Ui};
use serde_json::Value;

const ACCENT: Color32 = Color32::from_rgb(200, 45, 40);

pub fn dead_rising(ui: &mut Ui, cx: &mut GameCtx) {
    let sub = cx.text("/Save/Area").map(|a| format!("Saved in area {a}")).unwrap_or_default();
    hero(ui, ACCENT, "Dead Rising", &sub, |ui| {
        cx.tile(ui, "Level", "/Frank/Level", ACCENT);
        cx.tile(ui, "PP", "/Frank/PP", ACCENT);
    });
    panel(ui, "Frank", |ui| {
        cx.row_fixed(ui, "Max health", "/Frank/Max health", 120.0, 90.0);
        ui.weak(tr("1000 per health block: 4000 at level 1, 12000 at level 50."));
        let skills = cx.int("/Frank/Skills").unwrap_or(0);
        ui.label(trf("Skills learned: {0} of 21", &[&(skills.count_ones())]));
        if ui.add_enabled(skills != DR1_ALL_SKILLS && cx.can_edit("/Frank/Skills"), eframe::egui::Button::new(tr("Learn all skills"))).clicked() {
            cx.set("/Frank/Skills", Value::from(DR1_ALL_SKILLS));
        }
        if super::quick_button(ui, tr("Level 50 with full health"), "") {
            cx.set("/Frank/Level", Value::from(50));
            cx.set("/Frank/Max health", Value::from(12_000));
            cx.set("/Frank/Skills", Value::from(DR1_ALL_SKILLS));
        }
        ui.weak(tr("Level and PP are kept separately in the save, so change them together."));
    });
}

pub fn dead_rising2(ui: &mut Ui, cx: &mut GameCtx) {
    let otr = cx.tree.is_some_and(|t| t.kind_name == KIND_OTR);
    let (title, who) = if otr { ("Dead Rising 2: Off the Record", "/Frank") } else { ("Dead Rising 2", "/Chuck") };
    let p = |k: &str| format!("{who}/{k}");
    hero(ui, ACCENT, title, "", |ui| {
        for k in ["Level", "PP", "Money"] {
            cx.tile(ui, k, &p(k), ACCENT);
        }
    });
    panel(ui, "Level-up stats", |ui| {
        for (k, _, hi) in DR2_STAT_NAMES.iter().skip(3) {
            cx.attribute(ui, k, &p(k), *hi as f64, ACCENT);
        }
        if super::quick_button(ui, tr("Max level and stats"), "") {
            for (k, _, hi) in DR2_STAT_NAMES {
                if k != "PP" && k != "Money" {
                    cx.set(&p(k), Value::from(hi));
                }
            }
        }
        ui.weak(tr("Health and item slots count blocks and slots; attack, speed and throwing count upgrades. The game's own maximum is the end of each bar."));
    });
}
