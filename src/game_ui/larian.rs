//! Larian saves: Divinity: Original Sin 2 (the party, editable) and Baldur's Gate 3 (the
//! save's party summary, gold and stack sizes of what each companion carries, and the story's
//! global flags with the risky ones marked). See `fmt/larian.rs` and `fmt/bg3_flags.rs`.

use crate::i18n::{tr, trf};
use crate::widgets::{ListBox, ScrollLock};
use super::{columns, fact, fmt_num, hero, panel, tabs, GameCtx};
use crate::fmt::larian::ATTRIBUTES;
use crate::tree::esc;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn gold_of(cx: &GameCtx, base: &str) -> i64 {
    keys(cx, &format!("{base}/Gold")).iter().filter_map(|k| cx.int(&format!("{base}/Gold/{}", esc(k)))).sum()
}

pub fn dos2(ui: &mut Ui, cx: &mut GameCtx) {
    // Teal and gold, like the game's menus.
    let teal = Color32::from_rgb(38, 150, 145);
    let gold = Color32::from_rgb(205, 165, 70);
    let members = keys(cx, "/Party");
    let mut sub = vec![];
    for k in ["Region", "Game version"] {
        if let Some(s) = cx.text(&format!("/Save/{k}")) {
            sub.push(s);
        }
    }
    let total: i64 = members.iter().map(|m| gold_of(cx, &format!("/Party/{}", esc(m)))).sum();
    hero(ui, teal, "Divinity: Original Sin 2", &sub.join("  \u{00B7}  "), |ui| {
        fact(ui, "Party", &members.len().to_string(), teal);
        fact(ui, "Gold carried", &fmt_num(total as f64), gold);
    });
    if members.is_empty() {
        ui.weak(tr("No player characters found in this save."));
        return;
    }
    let t = tabs(ui, "dos2_party", &members);
    let base = format!("/Party/{}", esc(&members[t]));
    let at = |k: &str| format!("{base}/{k}");
    let mut line = vec![];
    for k in ["Race", "Origin"] {
        if let Some(s) = cx.text(&at(k)).filter(|s| !s.is_empty() && s != "Generic" && !s.starts_with("Generic")) {
            line.push(s);
        }
    }
    line.push(if cx.flag(&at("In party")) { tr("In the party").into() } else { tr("Not in the party").into() });
    ui.label(RichText::new(line.join("  \u{00B7}  ")).color(theme_fg(ui, teal)));
    ui.add_space(6.0);
    columns(ui, 360.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Attributes", |ui| {
                if !ATTRIBUTES.iter().any(|a| cx.num(&format!("{base}/Attributes/{a}")).is_some()) {
                    // Without this the panel showed only the note about base points, over nothing.
                    ui.weak(tr("This save doesn't list this character's attributes."));
                    return;
                }
                for a in ATTRIBUTES {
                    cx.attribute(ui, a, &format!("{base}/Attributes/{a}"), 40.0, gold);
                }
                ui.weak(tr("Base points (10 is the starting value), before items and talents."));
            });
        } else {
            panel(ui, "Character", |ui| {
                egui::Grid::new(("dos2_char", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
                    cx.grid_row(ui, "Experience", &at("Experience"), 120.0);
                    cx.grid_row(ui, "Vitality", &at("Vitality"), 100.0);
                    for k in ["Attribute points", "Combat ability points", "Civil ability points", "Talent points"] {
                        cx.grid_row(ui, k, &format!("{base}/Points/{k}"), 80.0);
                    }
                });
                ui.weak(tr("The level follows from experience; points not yet spent can be spent in the game. Vitality is the current value."));
            });
        }
    });
    let piles = keys(cx, &at("Gold"));
    panel(ui, "Gold", |ui| {
        if piles.is_empty() {
            ui.weak(tr("This character carries no gold."));
            return;
        }
        egui::Grid::new(("dos2_gold", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
            for k in &piles {
                cx.grid_row(ui, k, &format!("{base}/Gold/{}", esc(k)), 120.0);
            }
        });
        ui.label(RichText::new(trf("{0} gold on {1}", &[&fmt_num(gold_of(cx, &base) as f64), &members[t]])).color(theme_fg(ui, gold)));
        ui.weak(tr("Each line is a stack of coins in this character's inventory (named by its stats entry)."));
    });
    let things = keys(cx, &at("Inventory"));
    if !things.is_empty() {
        panel(ui, &trf("Inventory ({0} stacks)", &[&things.len()]), |ui| {
            egui::ScrollArea::vertical().show_list(ui, ("dos2_inv", t), 320.0, |ui| {
                columns(ui, 360.0, things.len(), |ui, i| {
                    cx.row_fixed(ui, &things[i].replace('_', " "), &format!("{base}/Inventory/{}", esc(&things[i])), 190.0, 60.0);
                });
            });
            ui.weak(
                tr("How many of each item this character carries directly (items inside bags aren't listed), named by the game's item entry. Change a stack size, or add items below; the game may trim a stack to what the item allows."),
            );
        });
    }
    add_items(ui, cx, tr("Every kind of item found in an inventory in this save can be added, as a copy of one the game made. Items go into the chosen character's bag when you save; where they already carry a stack of the same item, that stack grows instead. Keys, containers and story items aren't offered. Keep a backup: this hasn't been tried in the game yet."));
}

/// "Add items": who gets them, and an amount per item kind (see `fmt::larian_add` and
/// `fmt::bg3_add`), with `hint` under the list.
fn add_items(ui: &mut Ui, cx: &mut GameCtx, hint: &str) {
    let kinds: Vec<String> = keys(cx, "/Add items").into_iter().filter(|k| k != "Add to").collect();
    if kinds.is_empty() {
        return;
    }
    panel(ui, "Add items", |ui| {
        cx.row_fixed(ui, "Add to", "/Add items/Add to", 120.0, 200.0);
        ui.add_space(4.0);
        let q = super::screens2::filter_box(ui, "dos2_add", tr("potion, arrow, scroll…"));
        let shown: Vec<(String, String)> = kinds
            .iter()
            .filter(|k| q.is_empty() || k.to_lowercase().contains(&q) || k.replace('_', " ").to_lowercase().contains(&q))
            .map(|k| (k.replace('_', " "), format!("/Add items/{}", esc(k))))
            .collect();
        let chosen = kinds.iter().filter(|k| cx.text(&format!("/Add items/{}", esc(k))).is_some_and(|v| v != "Don't add")).count();
        if chosen > 0 {
            ui.label(RichText::new(trf("{0} kinds of item to add", &[&chosen])).strong());
        }
        if shown.len() > 300 {
            ui.weak(trf("{0} items match; type to narrow the list.", &[&shown.len()]));
        }
        egui::ScrollArea::vertical().show_list(ui, "dos2_add_list", 360.0, |ui| {
            super::screens2::value_rows(ui, cx, "dos2_add_rows", &shown[..shown.len().min(300)]);
        });
        ui.weak(hint);
    });
}

fn theme_fg(ui: &Ui, c: Color32) -> Color32 {
    crate::theme::fg(ui, c)
}

pub fn bg3(ui: &mut Ui, cx: &mut GameCtx) {
    let gold = Color32::from_rgb(200, 160, 80);
    let mut sub = vec![];
    for k in ["Save name", "Difficulty"] {
        if let Some(s) = cx.text(&format!("/Save/{k}")).filter(|s| !s.is_empty()) {
            sub.push(s);
        }
    }
    hero(ui, gold, "Baldur's Gate 3", &sub.join("  \u{00B7}  "), |ui| {
        for k in ["Leader", "Region", "Game version"] {
            if let Some(s) = cx.text(&format!("/Save/{k}")) {
                fact(ui, k, &s, gold);
            }
        }
    });
    let party = cx.get("/Party").and_then(Value::as_array).cloned().unwrap_or_default();
    panel(ui, "Party (read-only)", |ui| {
        if party.is_empty() {
            ui.weak(tr("The save's summary lists no party."));
            return;
        }
        let s = |c: &Value, k: &str| match c.get(k) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(v) => v.to_string(),
        };
        egui::ScrollArea::horizontal().auto_shrink([false, true]).show_locked(ui, |ui| {
            egui::Grid::new("bg3_party").num_columns(5).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                for h in ["Origin", "Race", "Class", "Level", "Experience"] {
                    ui.label(RichText::new(h).strong().color(theme_fg(ui, gold)));
                }
                ui.end_row();
                for c in &party {
                    for k in ["Origin", "Race", "Class", "Level", "Experience"] {
                        ui.label(s(c, k));
                    }
                    ui.end_row();
                }
            });
        });
        ui.add_space(6.0);
        ui.weak(tr("From the save's own summary."));
    });
    bg3_carried(ui, cx, gold);
    add_items(ui, cx, tr("Every kind of item in your party's bags can be added, as a copy of one the game made. The copies go into the chosen character's bag when you save. Items kept in a stack, worn gear, containers, keys and story items aren't offered; a stack's size can be changed under Stacked items. Keep a backup: this hasn't been tried in the game yet."));
    bg3_story(ui, cx, gold);
}

/// Gold and stack sizes of what each companion carries (see `fmt::larian::bg3_carried`).
fn bg3_carried(ui: &mut Ui, cx: &mut GameCtx, gold: Color32) {
    let people = keys(cx, "/Characters");
    if people.is_empty() {
        ui.weak(tr("Gold and items aren't readable in this save, so nothing here can be changed."));
        return;
    }
    let t = tabs(ui, "bg3_who", &people);
    let base = format!("/Characters/{}", esc(&people[t]));
    let piles = keys(cx, &format!("{base}/Gold"));
    let sum: i64 = piles.iter().filter_map(|k| cx.int(&format!("{base}/Gold/{}", esc(k)))).sum();
    panel(ui, "Gold", |ui| {
        ui.label(RichText::new(trf("{0} gold on {1}", &[&fmt_num(sum as f64), &people[t]])).strong().color(theme_fg(ui, gold)));
        if piles.is_empty() {
            ui.weak(tr("No coins on this character."));
            return;
        }
        egui::Grid::new(("bg3_gold", t)).num_columns(2).spacing([14.0, 6.0]).show(ui, |ui| {
            for k in &piles {
                cx.grid_row(ui, k, &format!("{base}/Gold/{}", esc(k)), 120.0);
            }
        });
        ui.weak(tr("Each line is one stack of coins in this character's inventory."));
    });
    let things = keys(cx, &format!("{base}/Items"));
    if !things.is_empty() {
        panel(ui, &trf("Stacked items ({0})", &[&things.len()]), |ui| {
            let q = super::screens2::filter_box(ui, &format!("bg3_items{t}"), tr("potion, arrow, scroll…"));
            let shown: Vec<&String> = things.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).collect();
            egui::ScrollArea::vertical().show_list(ui, ("bg3_inv", t), 360.0, |ui| {
                columns(ui, 300.0, shown.len(), |ui, i| {
                    cx.row_fixed(ui, &shown[i].replace('_', " "), &format!("{base}/Items/{}", esc(shown[i])), 190.0, 60.0);
                });
            });
            ui.weak(
                tr("Stack sizes of what this character carries, by the game's item name. Items can't be removed here; Add items adds new ones. A stack the game split just before saving shows as read-only."),
            );
        });
    }
}

/// A story section's name in the chosen language (see `fmt::bg3_flags::section`). Place and
/// companion names stay as the game spells them.
fn section_label(sec: &str) -> String {
    // The section names `fmt::bg3_flags::section` builds around a companion or an unknown prefix.
    let companion = sec.starts_with("Companion: ").then(|| &sec[11..]);
    let other = (sec.starts_with("Other (") && sec.ends_with(')')).then(|| &sec[7..sec.len() - 1]);
    if let Some(c) = companion {
        return trf("Companion: {0}", &[&c]);
    }
    if let Some(o) = other {
        return trf("Other ({0})", &[&o]);
    }
    match sec {
        "General" => tr("General"),
        "Camp" => tr("Camp"),
        "Camp nights" => tr("Camp nights"),
        "Companions" => tr("Companions"),
        "Regions" => tr("Regions"),
        "Hirelings" => tr("Hirelings"),
        "Debug switches" => tr("Debug switches"),
        "Nautiloid crash site" => tr("Nautiloid crash site"),
        "Chapel ruins" => tr("Chapel ruins"),
        "Forest and Blighted Village" => tr("Forest and Blighted Village"),
        "Wilderness" => tr("Wilderness"),
        "Goblin camp" => tr("Goblin camp"),
        "Githyanki creche" => tr("Githyanki creche"),
        _ => return sec.to_string(),
    }
    .to_string()
}

/// A risk mark's short name.
fn risk_tag(r: crate::fmt::bg3_flags::Risk) -> &'static str {
    use crate::fmt::bg3_flags::Risk;
    match r {
        Risk::Debug => tr("testing switch"),
        Risk::Scene => tr("scene in progress"),
        Risk::Mode => tr("game mode"),
        Risk::Life => tr("life or death"),
        Risk::Party => tr("party"),
        Risk::Romance => tr("romance"),
        Risk::Progress => tr("story progress"),
        Risk::Tadpole => tr("tadpole"),
    }
}

/// What a risk mark says (see `fmt::bg3_flags::Risk`).
fn risk_text(r: crate::fmt::bg3_flags::Risk) -> &'static str {
    use crate::fmt::bg3_flags::Risk;
    match r {
        Risk::Debug => tr("A developer's testing switch. Leave it as it is."),
        Risk::Scene => tr("A script sets and clears this while a scene runs; changing it can leave the scene stuck."),
        Risk::Mode => tr("The game mode (difficulty and the like) is also kept elsewhere in the save; this flag alone doesn't change it."),
        Risk::Life => tr("Someone lives or dies by this flag. The character itself isn't brought back or killed, so scenes can contradict what you see."),
        Risk::Party => tr("Who is in the party or at camp is also kept elsewhere in the save; changing only the flag can leave a companion half in, half out."),
        Risk::Romance => tr("Romance state: later scenes check it, and contradicting flags can confuse them."),
        Risk::Progress => tr("Marks story progress; setting it early can skip scenes or lock quests."),
        Risk::Tadpole => tr("The tadpole and mind flayer story; key scenes depend on it."),
    }
}

/// The story's global flags by area and companion, each a check box, with risky ones marked
/// (see `fmt::bg3_flags`).
fn bg3_story(ui: &mut Ui, cx: &mut GameCtx, _accent: Color32) {
    let sections = keys(cx, "/Story flags");
    if sections.is_empty() {
        return;
    }
    let flags: Vec<(String, Vec<String>)> = sections.iter().map(|s| (s.clone(), keys(cx, &format!("/Story flags/{}", esc(s))))).collect();
    let path = |s: &str, k: &str| format!("/Story flags/{}/{}", esc(s), esc(k));
    let total: usize = flags.iter().map(|f| f.1.len()).sum();
    let on: usize = flags.iter().map(|(s, ks)| ks.iter().filter(|k| cx.flag(&path(s, k))).count()).sum();
    let changed: usize = flags.iter().map(|(s, ks)| ks.iter().filter(|k| cx.modified(&path(s, k))).count()).sum();
    let unnamed = cx.int("/Story flags unnamed").unwrap_or(0);
    let warn = theme_fg(ui, crate::theme::WARN);
    panel(ui, &trf("Story flags ({0})", &[&total]), |ui| super::screens2::story_guard(ui, |ui| {
        ui.label(trf("{0} named flags, {1} of them on.", &[&total, &on]));
        if unnamed > 0 {
            ui.weak(trf("{0} more flags have no name in this save and aren't listed.", &[&unnamed]));
        }
        if changed > 0 {
            ui.label(RichText::new(trf("{0} flags changed", &[&changed])).strong().color(theme_fg(ui, crate::theme::MODIFIED)));
        }
        let q = super::screens2::filter_box(ui, "bg3_story", tr("grove, Astarion, dead\u{2026}"));
        let sid = egui::Id::new("bg3_story_show");
        let mut show = ui.data(|d| d.get_temp::<u8>(sid)).unwrap_or(0);
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Show:"));
            ui.selectable_value(&mut show, 0, tr("All"));
            ui.selectable_value(&mut show, 1, tr("On"));
            ui.selectable_value(&mut show, 2, tr("Off"));
            ui.selectable_value(&mut show, 3, tr("Changed"));
            ui.selectable_value(&mut show, 4, RichText::new(tr("Risky")).color(warn));
            ui.selectable_value(&mut show, 5, tr("Not risky"));
        });
        ui.data_mut(|d| d.insert_temp(sid, show));
        let mut any = false;
        egui::ScrollArea::vertical().show_list(ui, "bg3_story_list", 520.0, |ui| {
            for (sec, ks) in &flags {
                let label = section_label(sec);
                let shown: Vec<&String> = ks
                    .iter()
                    .filter(|k| {
                        let p = path(sec, k);
                        let risky = crate::fmt::bg3_flags::risk(k).is_some();
                        let keep = match show {
                            1 => cx.flag(&p),
                            2 => !cx.flag(&p),
                            3 => cx.modified(&p),
                            4 => risky,
                            5 => !risky,
                            _ => true,
                        };
                        keep && (q.is_empty() || k.to_lowercase().contains(&q) || label.to_lowercase().contains(&q) || sec.to_lowercase().contains(&q))
                    })
                    .collect();
                if shown.is_empty() {
                    continue;
                }
                any = true;
                let n_on = ks.iter().filter(|k| cx.flag(&path(sec, k))).count();
                let title = trf("{0}  ({1} of {2} on)", &[&label, &n_on, &ks.len()]);
                let open = if q.is_empty() && show == 0 { None } else { Some(true) };
                egui::CollapsingHeader::new(title).id_salt(("bg3_story", sec.as_str())).open(open).show(ui, |ui| {
                    for k in shown {
                        let p = path(sec, k);
                        let mut v = cx.flag(&p);
                        let modified = cx.modified(&p);
                        ui.horizontal(|ui| {
                            let mut text = RichText::new(k.as_str());
                            if modified {
                                text = text.color(theme_fg(ui, crate::theme::MODIFIED));
                            }
                            let r = ui.add_enabled(cx.can_edit(&p), egui::Checkbox::new(&mut v, text));
                            super::find_mark(ui, r.rect, k);
                            if r.changed() {
                                cx.set(&p, Value::from(i64::from(v)));
                            }
                            if let Some(risk) = crate::fmt::bg3_flags::risk(k) {
                                ui.label(RichText::new(format!("\u{26A0} {}", risk_tag(risk))).small().color(warn)).on_hover_text(risk_text(risk)); // i18n-ok
                            }
                        });
                    }
                });
            }
        });
        if !any {
            ui.weak(tr("No flag matches."));
        }
        ui.add_space(4.0);
        ui.weak(tr("Tick a flag to set it, untick it to clear it. These are the story's global flags, named as the game's scripts name them; the story script's own list of set flags is kept in step when you save. Changing a flag doesn't replay the scene behind it. Flags marked \u{26A0} are riskier than most, so change few at a time and keep the backup."));
    }));
}
