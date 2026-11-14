//! Cairn: the run's statistics, the saves in the slot, and what the climber (Aava or Marco) carries (the
//! backpack, the chalk pouch and the piton belt) as item grids.

use crate::i18n::tr;
use crate::widgets::ScrollLock;
use super::{cell_grid, hero, panel, select, selection, Cell, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

const ACCENT: Color32 = Color32::from_rgb(214, 120, 70);
const NONE: &str = "(none)";

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let current = cx.archive.map(|(c, _, _)| c.to_string()).unwrap_or_default();
    let sub = crate::app::cairn_save_label(&current).unwrap_or_else(|| current.clone());
    let carried = |cx: &GameCtx, list: &str| {
        cx.get(&format!("/{}", pointer_escape(list))).and_then(Value::as_object).map_or(0, |m| m.values().filter(|r| r.get("Item").and_then(Value::as_str).is_some_and(|s| s != NONE)).count())
    };
    let (bag, pouch, belt) = (carried(cx, "Backpack"), carried(cx, "Chalk pouch"), carried(cx, "Piton belt"));
    hero(ui, ACCENT, "Cairn", &sub, |ui| {
        if let Some(s) = cx.int("/Run/Play time (seconds)") {
            super::fact(ui, "Play time", &format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60), ACCENT);
        }
        cx.tile(ui, "Climber", "/Run/Climber", ACCENT);
        cx.tile(ui, "Falls", "/Run/Falls", ACCENT);
        cx.tile(ui, "Deaths", "/Run/Deaths", ACCENT);
        super::fact(ui, "Carried", &format!("{bag} in the bag, {belt} pitons, {pouch} chalk"), ACCENT);
    });

    if let Some((cur, entries, newest)) = cx.archive {
        let newest = newest.map(str::to_string);
        if newest.as_deref().is_some_and(|n| n != cur) {
            ui.label(
                RichText::new(tr("This isn't the save the game continues from: Cairn loads the newest save in the slot (marked in the list below), so edits here won't show up in the game."))
                    .color(crate::theme::MODIFIED),
            );
        }
        let mut saves: Vec<(String, String)> = entries.iter().filter_map(|e| crate::app::cairn_save_label(e).map(|l| (e.clone(), l))).collect();
        // The save the game continues from first, then the dated autosaves newest first, then the rest.
        saves.sort_by(|a, b| {
            let resume = |e: &str| newest.as_deref() == Some(e);
            let dated = |l: &str| l.starts_with(|c: char| c.is_ascii_digit());
            resume(&b.0).cmp(&resume(&a.0)).then(dated(&b.1).cmp(&dated(&a.1))).then(b.1.cmp(&a.1))
        });
        saves.dedup_by(|a, b| a.0 == b.0);
        let cur = cur.to_string();
        let mut open = None;
        egui::CollapsingHeader::new(format!("Saves in this slot ({})", saves.len())).id_salt("cairn_saves").show(ui, |ui| {
            ui.weak(tr("The dated saves are the game's autosaves. Cairn continues from the newest save in the slot; pick another to open it instead (unsaved changes are dropped)."));
            egui::ScrollArea::vertical().id_salt("cairn_save_list").max_height(220.0).show_locked(ui, |ui| {
                for (entry, label) in &saves {
                    let shown = *entry == cur;
                    let resume = if newest.as_deref() == Some(entry.as_str()) { "  \u{2014} the game continues from this one" } else { "" };
                    let text = if shown { RichText::new(format!("{label}  (open){resume}")).strong() } else { RichText::new(format!("{label}{resume}")) };
                    if ui.selectable_label(shown, text).on_hover_text(entry).clicked() && !shown {
                        open = Some(entry.clone());
                    }
                }
            });
        });
        if open.is_some() {
            cx.open_entry = open;
        }
        ui.add_space(6.0);
    }

    if cx.get("/Buffs").is_some() {
        panel(ui, "Buffs", |ui| {
            egui::Grid::new("cairn_buffs").num_columns(4).striped(true).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.strong(tr("Buff"));
                ui.strong(tr("Seconds left"));
                ui.strong(tr("From"));
                ui.strong(tr("Strength"));
                ui.end_row();
                for name in crate::fmt::cairn::BUFFS {
                    let base = format!("/Buffs/{}", pointer_escape(name));
                    let secs = cx.int(&format!("{base}/Seconds left")).unwrap_or(0);
                    let stacked = cx.int(&format!("{base}/Stacked")).unwrap_or(0);
                    cx.label(ui, name, &format!("{base}/Seconds left"));
                    ui.horizontal(|ui| {
                        cx.compact(ui, &format!("{base}/Seconds left"), 80.0);
                        if secs > 0 {
                            let shown = format!("{}:{:02}", secs / 60, secs % 60);
                            ui.weak(if stacked > 1 { format!("{shown} ({stacked} stacked)") } else { shown });
                        }
                    });
                    cx.compact(ui, &format!("{base}/From"), 200.0);
                    cx.compact(ui, &format!("{base}/Strength"), 70.0);
                    ui.end_row();
                }
            });
            ui.weak(
                "Seconds left is the time the game shows for the buff (10:00 is 600). Set it to 0 to end a buff, or give an empty one time to start it \
                 with its usual source and strength. Sips taken one after another stack and run in turn; a changed buff is saved as one with the whole time. \
                 Stamina and Strength show on the game's Grip line (it shows the longest of them and Grip). Extra grip comes from chalk and \
                 doesn't run down off the wall (the chalk count on the HUD is the chalk itself: 12 grips a charge). \
                 Strength and Stoked! can only be changed when the save already has them.",
            );
        });
    }

    panel(ui, "Backpack", |ui| {
        grid(ui, cx, "cairn_bag", "Backpack", 8);
        ui.weak(
            "Count is the uses left (chalk, climbing tape) or the sips left in a bottle; Contents is what a bottle, flask or thermos holds. \
             Pick an item in an empty square to add it: it goes at the bottom of the bag, and \u{201C}Shake up bag\u{201D} in the game settles everything. \
             Set an item to (none) to take it out.",
        );
    });
    panel(ui, "Piton belt", |ui| {
        grid(ui, cx, "cairn_belt", "Piton belt", 10);
        ui.weak(tr("The pitons hanging under the bag. The game shows how many the belt holds (6 at first)."));
    });
    panel(ui, "Chalk pouch", |ui| {
        grid(ui, cx, "cairn_pouch", "Chalk pouch", 10);
        ui.weak(
            "The chalk on the climber's belt; Count is its uses. The pouch holds only as many uses in all as the game shows after the slash \
             (3/3 at first, more with upgrades): put in more and the game drops the extra chalk.",
        );
    });
}

/// The rows of one list as a grid of squares, with the chosen one's editor under it.
fn grid(ui: &mut Ui, cx: &mut GameCtx, id: &str, list: &str, cols: usize) {
    let base = format!("/{}", pointer_escape(list));
    let keys: Vec<String> = cx.get(&base).and_then(Value::as_object).map(|m| m.keys().cloned().collect()).unwrap_or_default();
    let cells: Vec<Cell> = keys
        .iter()
        .map(|k| {
            let p = format!("{base}/{}", pointer_escape(k));
            let item = cx.text(&format!("{p}/Item")).unwrap_or_default();
            let empty = item.is_empty() || item == NONE;
            let contents = cx.text(&format!("{p}/Contents")).filter(|c| c != NONE && !c.is_empty());
            Cell {
                title: if empty { String::new() } else { contents.map_or(item.clone(), |c| format!("{item} ({c})")) },
                count: (!empty).then(|| cx.int(&format!("{p}/Count"))).flatten(),
                empty,
                modified: ["Item", "Contents", "Count"].iter().any(|f| cx.modified(&format!("{p}/{f}"))),
                tint: None,
            }
        })
        .collect();
    let sel = selection(ui, id);
    if let Some(i) = cell_grid(ui, &cells, cols, sel, ACCENT) {
        select(ui, id, if sel == Some(i) { None } else { Some(i) });
    }
    let Some(i) = selection(ui, id).filter(|&i| i < keys.len()) else {
        ui.weak(tr("Click a square to change it."));
        return;
    };
    let p = format!("{base}/{}", pointer_escape(&keys[i]));
    egui::Frame::none().fill(super::inset_fill(ui, ACCENT)).rounding(8.0).inner_margin(10.0).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(keys[i].as_str()).strong());
            if ui.small_button(tr("Close")).clicked() {
                select(ui, id, None);
            }
        });
        egui::Grid::new((id, "row")).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            cx.grid_row(ui, "Item", &format!("{p}/Item"), 260.0);
            cx.grid_row(ui, "Count", &format!("{p}/Count"), 100.0);
            cx.grid_row(ui, "Contents", &format!("{p}/Contents"), 260.0);
        });
    });
}
