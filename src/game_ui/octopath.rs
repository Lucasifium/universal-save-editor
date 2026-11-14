//! Octopath Traveler and Octopath Traveler II (Unreal GVAS; names found by `fmt::octopath`):
//! money and the eight travellers' level, EXP, job points, and current HP/SP.

use crate::i18n::{tr, tr_any, trf};
use crate::widgets::ScrollLock;
use super::{fact, hero, panel, GameCtx};
use crate::fmt::octopath::{self as ot, OT1};
use eframe::egui::{self, Color32, RichText, Ui};

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let kind = cx.tree.map_or_else(String::new, |t| t.kind_name.clone());
    let first = kind == OT1;
    let accent = if first { Color32::from_rgb(176, 140, 64) } else { Color32::from_rgb(70, 120, 170) };
    let root = cx.root;
    let money = ot::ptr(root, "", "PlayerBackpack").and_then(|b| ot::ptr(root, &b, "Money"));
    let secs = ot::ptr(root, "", "GamePlaySecond").and_then(|p| cx.int(&p));
    let sub = secs.map_or_else(String::new, |s| trf("{0} played", &[&format!("{}:{:02}", s / 3600, s / 60 % 60)]));
    hero(ui, accent, if first { "Octopath Traveler" } else { "Octopath Traveler II" }, &sub, |ui| {
        if let Some(p) = &money {
            cx.tile(ui, "Money (leaves)", p, accent);
        }
    });
    // Slots of the front row, as indexes into the member list.
    let party: Vec<i64> = ot::ptr(root, "", "PlayerParty").map(|p| ot::ints(root, &p, "MainMemberID")).unwrap_or_default();
    let members: Vec<(usize, String, i64)> = ot::members(root).into_iter().enumerate().filter(|(_, m)| (1..=8).contains(&m.1)).map(|(i, (p, id))| (i, p, id)).collect();
    panel(ui, "Travellers", |ui| {
        if members.is_empty() {
            ui.weak(tr("No travellers in this save."));
            return;
        }
        egui::ScrollArea::horizontal().id_salt("ot_party").show_locked(ui, |ui| {
            egui::Grid::new("ot_party_grid").num_columns(7).spacing([12.0, 5.0]).striped(true).show(ui, |ui| {
                for h in ["Traveller", "Job", "Level", "EXP", "JP", "HP", "SP"] {
                    ui.label(RichText::new(tr_any(h)).strong());
                }
                ui.end_row();
                for (slot, p, id) in &members {
                    let name = ot::character(&kind, *id).unwrap_or("?");
                    let mark = if party.contains(&(*slot as i64)) { "  \u{2605}" } else { "" };
                    ui.label(RichText::new(format!("{name}{mark}")).color(ui.visuals().strong_text_color())); // i18n-ok
                    let job = ot::ptr(root, p, "FirstJobID").and_then(|q| cx.int(&q)).map(ot::job).unwrap_or_default();
                    ui.label(job);
                    for (k, w) in [("Level", 44.0), ("Exp", 84.0), ("JobPoint", 70.0), ("RawHP", 60.0), ("RawMP", 52.0)] {
                        match ot::ptr(root, p, k) {
                            Some(q) => cx.compact(ui, &q, w),
                            None => {
                                ui.label("");
                            }
                        }
                    }
                    ui.end_row();
                }
            });
        });
        ui.weak(tr("\u{2605} marks the front row. Level goes from 1 to 99; EXP isn't worked out from the level, so set both. HP and SP are current values (the maximum comes from level, job and gear)."));
        if first {
            ui.weak(tr("Octopath Traveler (2018): these values are read from the save as it is; changing them hasn't been tried in the game yet."));
        }
    });
    if let Some(n) = ot::ptr(root, "", "FirstSelectCharacterID").and_then(|p| cx.int(&p)).filter(|_| !first) {
        if let Some(name) = ot::character(&kind, n) {
            ui.horizontal_wrapped(|ui| fact(ui, "Started with", name, accent));
        }
    }
}
