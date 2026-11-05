//! Elden Ring, Dark Souls III, II and I, and Sekiro: a character sheet and the inventory.

use super::{cell_grid, fact, materials_button, fmt_num, hero, inset_fill, meter, panel, select, selection, tabs, Cell, GameCtx, Game};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};

struct Look {
    accent: Color32,
    /// What the game calls its currency and level.
    currency: &'static str,
    level: &'static str,
    memory: &'static str,
    attrs: &'static [&'static str],
    default_name: &'static str,
}

fn look(g: Game) -> Look {
    match g {
        Game::EldenRing => Look {
            accent: Color32::from_rgb(201, 168, 92),
            currency: "Runes",
            level: "Level",
            memory: "Rune memory",
            attrs: &["Vigor", "Mind", "Endurance", "Strength", "Dexterity", "Intelligence", "Faith", "Arcane"],
            default_name: "Tarnished",
        },
        Game::DarkSouls3 => Look {
            accent: Color32::from_rgb(222, 120, 60),
            currency: "Souls",
            level: "Soul level",
            memory: "Soul memory",
            attrs: &["Vigor", "Attunement", "Endurance", "Vitality", "Strength", "Dexterity", "Intelligence", "Faith", "Luck"],
            default_name: "Ashen One",
        },
        Game::DarkSouls2 => Look {
            accent: Color32::from_rgb(110, 150, 210),
            currency: "Souls",
            level: "Soul level",
            memory: "Soul memory",
            attrs: &["Vigor", "Endurance", "Vitality", "Attunement", "Strength", "Dexterity", "Adaptability", "Intelligence", "Faith"],
            default_name: "Bearer of the Curse",
        },
        Game::DarkSouls1 => Look {
            accent: Color32::from_rgb(190, 150, 90),
            currency: "Souls",
            level: "Soul level",
            memory: "",
            attrs: &["Vitality", "Attunement", "Endurance", "Strength", "Dexterity", "Resistance", "Intelligence", "Faith"],
            default_name: "Chosen Undead",
        },
        _ => Look {
            accent: Color32::from_rgb(200, 60, 60),
            currency: "Sen",
            level: "",
            memory: "",
            attrs: &[],
            default_name: "Wolf",
        },
    }
}

/// Dark Souls and Dark Souls III: souls needed to go from `level` to the next one (a table
/// up to 12, then the game's formula, rounded down).
fn ds_soul_cost(level: i64) -> i64 {
    const LOW: [i64; 11] = [673, 689, 706, 723, 740, 757, 775, 793, 811, 829, 847];
    match level {
        1..=11 => LOW[level as usize - 1],
        _ => {
            let x = (level + 1) as f64;
            (0.02 * x * x * x + 3.06 * x * x + 105.6 * x - 895.0).floor() as i64
        }
    }
}

/// Elden Ring: runes needed to go from `level` to the next one.
fn er_rune_cost(level: i64) -> i64 {
    let l = (level + 81) as f64;
    let x = ((l - 92.0) * 0.02).max(0.0);
    ((x + 0.1) * l * l).floor() as i64 + 1
}

pub fn show(g: Game, ui: &mut Ui, cx: &mut GameCtx) {
    let lk = look(g);
    let name = cx.text("/Character/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| lk.default_name.to_string());
    let mut sub = Vec::new();
    if let Some(class) = cx.text("/Character/Starting class") {
        sub.push(class);
    }
    if !lk.level.is_empty() {
        if let Some(l) = cx.int(&format!("/Character/{}", lk.level)) {
            sub.push(format!("{} {l}", lk.level));
        }
    }
    let game_name = match g {
        Game::EldenRing => "Elden Ring",
        Game::DarkSouls3 => "Dark Souls III",
        Game::DarkSouls2 => "Dark Souls II",
        Game::DarkSouls1 => "Dark Souls",
        _ => "Sekiro: Shadows Die Twice",
    };
    sub.push(game_name.to_string());

    hero(ui, lk.accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, lk.currency, &format!("/Character/{}", lk.currency), lk.accent);
        if g == Game::Sekiro {
            cx.tile(ui, "Attack Power", "/Character/Attack Power", lk.accent);
            if let Some(p) = slot_of(cx, "/Items", "Spirit Emblem") {
                cx.tile(ui, "Spirit Emblems", &format!("{p}/Quantity"), lk.accent);
            }
            match slot_of(cx, "/Items", "Skill Point") {
                Some(p) => cx.tile(ui, "Skill points", &format!("{p}/Quantity"), lk.accent),
                None => fact(ui, "Skill points", "0", lk.accent),
            }
        } else {
            if let Some(l) = cx.int(&format!("/Character/{}", lk.level)) {
                fact(ui, lk.level, &l.to_string(), lk.accent);
                match g {
                    Game::EldenRing => fact(ui, "Runes to next level", &fmt_num(er_rune_cost(l) as f64), lk.accent),
                    Game::DarkSouls3 | Game::DarkSouls1 => fact(ui, "Souls to next level", &fmt_num(ds_soul_cost(l) as f64), lk.accent),
                    _ => {}
                }
            }
            if let Some(m) = cx.num(&format!("/Character/{}", lk.memory)) {
                fact(ui, lk.memory, &fmt_num(m), lk.accent);
            }
            cx.tile(ui, "Humanity", "/Character/Humanity", lk.accent);
        }
    });

    let wide = ui.available_width() > 760.0;
    let attributes = |ui: &mut Ui, cx: &mut GameCtx| {
        if lk.attrs.is_empty() {
            return;
        }
        panel(ui, "Attributes", |ui| {
            for a in lk.attrs {
                cx.attribute(ui, a, &format!("/Attributes/{a}"), 99.0, lk.accent);
            }
            ui.add_space(4.0);
            ui.weak(format!("{} follows these: each point is one level.", lk.level));
        });
    };
    let status = |ui: &mut Ui, cx: &mut GameCtx| {
        panel(ui, "Status", |ui| match g {
            Game::Sekiro => {
                ui.weak("The size of each bar. Prayer Beads and Gourd Seeds raise them in the game.");
                egui_grid(ui, cx, &[("Vitality", "/Status/Vitality"), ("Posture", "/Status/Posture"), ("Spirit Emblem capacity", "/Character/Spirit Emblem capacity")]);
                if cx.get("/Skills").is_some() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Skills").strong());
                    egui_grid(ui, cx, &[("Skill experience", "/Skills/Skill experience")]);
                    ui.horizontal_wrapped(|ui| {
                        for k in ["Skill points earned", "Total experience"] {
                            if let Some(n) = cx.num(&format!("/Skills/{k}")) {
                                fact(ui, k, &fmt_num(n), lk.accent);
                            }
                        }
                    });
                    ui.weak(if slot_of(cx, "/Items", "Skill Point").is_some() {
                        "Skill points to spend are the tile above. Skill experience fills toward the next point; the game hands it out once it's full."
                    } else {
                        "No skill points to spend right now: add a Skill Point in Items below to give some. \
                         Skill experience fills toward the next point; the game hands it out once it's full."
                    });
                }
            }
            _ => {
                for (pool, color) in [("HP", Color32::from_rgb(190, 60, 60)), ("FP", Color32::from_rgb(70, 110, 210)), ("Stamina", Color32::from_rgb(80, 160, 80))] {
                    let cur = cx.num(&format!("/Status/{pool}"));
                    let max = cx.num(&format!("/Status/Max {pool}")).or(cur);
                    if let (Some(c), Some(m)) = (cur, max) {
                        meter(ui, pool, c, m, color);
                    }
                }
                ui.add_space(4.0);
                ui.weak("Worked out by the game from your attributes when it loads.");
            }
        });
    };
    if wide && !lk.attrs.is_empty() {
        ui.columns(2, |c| {
            attributes(&mut c[0], cx);
            status(&mut c[1], cx);
        });
    } else {
        attributes(ui, cx);
        status(ui, cx);
    }
    progress(ui, cx);
    item_lists(g, ui, cx, lk.accent);
    if g != Game::Sekiro {
        ui.label(RichText::new(format!("Raise attributes here and your {} follows; change your {} in the tile above.", lk.level.to_lowercase(), lk.currency.to_lowercase())).weak());
    }
}

/// Upgrade materials that stack, and the most of each the game lets you carry.
fn materials(g: Game) -> (Vec<String>, i64, &'static str) {
    match g {
        Game::Sekiro => (
            [
                "Scrap Iron",
                "Scrap Magnetite",
                "Adamantite Scrap",
                "Black Gunpowder",
                "Yellow Gunpowder",
                "Fulminated Mercury",
                "Lump of Fat Wax",
                "Lump of Grave Wax",
                "Lapis Lazuli",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            99,
            "Scrap Iron, Black and Yellow Gunpowder, Scrap Magnetite, Adamantite Scrap, Fulminated Mercury, \
             Fat and Grave Wax and Lapis Lazuli, each to the 99 you can hold.",
        ),
        Game::DarkSouls3 => (
            ["Titanite Shard", "Large Titanite Shard", "Titanite Chunk", "Titanite Slab", "Twinkling Titanite", "Titanite Scale"].iter().map(|s| s.to_string()).collect(),
            99,
            "Titanite Shards, Large Shards, Chunks and Slabs, Twinkling Titanite and Titanite Scales, each to the 99 you can carry.",
        ),
        Game::DarkSouls2 => (
            ["Titanite Shard", "Large Titanite Shard", "Titanite Chunk", "Titanite Slab", "Twinkling Titanite", "Petrified Dragon Bone"].iter().map(|s| s.to_string()).collect(),
            99,
            "Titanite Shards, Large Shards, Chunks and Slabs, Twinkling Titanite and Petrified Dragon Bones, each to the 99 you can carry.",
        ),
        Game::DarkSouls1 => (
            [
                "Titanite Shard", "Large Titanite Shard", "Green Titanite Shard", "Titanite Chunk", "Blue Titanite Chunk", "White Titanite Chunk", "Red Titanite Chunk",
                "Titanite Slab", "Blue Titanite Slab", "White Titanite Slab", "Red Titanite Slab", "Dragon Scale", "Demon Titanite", "Twinkling Titanite",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            99,
            "Every kind of titanite, Dragon Scales and Demon Titanite, each to the 99 you can carry.",
        ),
        _ => {
            let mut m: Vec<String> = (1..=8).map(|i| format!("Smithing Stone [{i}]")).collect();
            m.push("Ancient Dragon Smithing Stone".into());
            m.extend((1..=9).map(|i| format!("Somber Smithing Stone [{i}]")));
            m.push("Somber Ancient Dragon Smithing Stone".into());
            m.extend((1..=9).map(|i| format!("Grave Glovewort [{i}]")));
            m.push("Great Grave Glovewort".into());
            m.extend((1..=9).map(|i| format!("Ghost Glovewort [{i}]")));
            m.push("Great Ghost Glovewort".into());
            (
                m,
                999,
                "Every Smithing Stone and Somber Smithing Stone (Ancient Dragon ones too) for weapons, \
                 and every Grave and Ghost Glovewort for Spirit Ashes, each to the 999 you can hold.",
            )
        }
    }
}

/// Brings every upgrade material up to the most you can carry: raises the ones carried and
/// puts the missing ones in empty slots.
pub(super) fn give_materials(g: Game, cx: &mut GameCtx) {
    let (list, max, _) = materials(g);
    let Some(serde_json::Value::Object(items)) = cx.get("/Items").cloned() else { return };
    let empty = crate::fmt::sekiro_inv::EMPTY;
    let mut free = items.iter().filter(|(_, v)| v.get("Item").and_then(|x| x.as_str()) == Some(empty)).map(|(k, _)| k.clone());
    for m in &list {
        let p = match slot_of(cx, "/Items", m) {
            Some(p) => p,
            None => {
                let Some(k) = free.next() else { return };
                let p = format!("/Items/{}", pointer_escape(&k));
                cx.set(&format!("{p}/Item"), serde_json::Value::from(m.as_str()));
                p
            }
        };
        let q = format!("{p}/Quantity");
        if cx.int(&q).unwrap_or(0) < max {
            cx.set(&q, serde_json::Value::from(max));
        }
    }
}

/// The slot under `base` holding `item`, as a pointer.
fn slot_of(cx: &GameCtx, base: &str, item: &str) -> Option<String> {
    let Some(serde_json::Value::Object(m)) = cx.get(base) else { return None };
    m.iter().find(|(_, v)| v.get("Item").and_then(|x| x.as_str()) == Some(item)).map(|(k, _)| format!("{base}/{}", pointer_escape(k)))
}

/// Carried items, key items and storage as grids (and Sekiro's learned skills).
fn item_lists(g: Game, ui: &mut Ui, cx: &mut GameCtx, accent: Color32) {
    if cx.get("/Items").is_none() {
        return;
    }
    panel(ui, "Inventory", |ui| {
        let mut names: Vec<String> = ["Items", "Key items", "Item box", "Storage", "Weapons", "Armor", "Talismans", "Ashes of War", "Rings"].iter().filter(|s| cx.get(&format!("/{s}")).is_some()).map(|s| s.to_string()).collect();
        let skills = g == Game::Sekiro;
        if skills {
            names.push("Skills & tools".into());
        }
        let sid = egui::Id::new("fs_inv_search");
        let mut query: String = ui.data(|d| d.get_temp(sid)).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Search every list:");
            let r = ui.add(egui::TextEdit::singleline(&mut query).hint_text("e.g. Smithing").desired_width(200.0));
            if !query.is_empty() && ui.small_button("Clear").clicked() {
                query.clear();
            }
            if r.changed() || query.is_empty() {
                select(ui, "fs_inv_found", None);
            }
        });
        ui.data_mut(|d| d.insert_temp(sid, query.clone()));
        let query = query.trim().to_lowercase();
        let t = if query.is_empty() { tabs(ui, "fs_inv_tab", &names) } else { usize::MAX };
        ui.add_space(4.0);
        let (_, max, tip) = materials(g);
        if materials_button(ui, &format!("{max} of each"), &format!("{tip} Ones you already have that many of are left alone.")) {
            give_materials(g, cx);
        }
        ui.add_space(4.0);
        if skills && t == names.len() - 1 && query.is_empty() {
            if let Some(serde_json::Value::Array(a)) = cx.get("/Learned/Skills & prosthetic tools").cloned() {
                ui.horizontal_wrapped(|ui| {
                    for n in a.iter().filter_map(|v| v.as_str()) {
                        egui::Frame::none().fill(inset_fill(ui, accent)).rounding(10.0).inner_margin(egui::Margin::symmetric(8.0, 3.0)).show(ui, |ui| {
                            ui.label(RichText::new(n).size(13.0));
                        });
                    }
                });
                ui.weak("Skills, combat arts and prosthetic tools are listed here; learn them in the game.");
            }
            return;
        }
        // The slots shown, as (list, slot name). Searching: every matching item in every list.
        let found: Vec<(String, String)> = names
            .iter()
            .filter(|n| !query.is_empty() && cx.get(&format!("/{}", pointer_escape(n))).is_some_and(|v| v.is_object()))
            .flat_map(|n| {
                let Some(serde_json::Value::Object(m)) = cx.get(&format!("/{}", pointer_escape(n))) else { return vec![] };
                m.iter()
                    .filter(|(_, v)| {
                        let item = v.get("Item").and_then(|x| x.as_str()).unwrap_or_default();
                        item != crate::fmt::sekiro_inv::EMPTY && item.to_lowercase().contains(&query)
                    })
                    .map(|(k, _)| (n.clone(), k.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        if !query.is_empty() && found.is_empty() {
            ui.weak("Nothing you have matches. To add it, clear the search and click an empty slot.");
            return;
        }
        let list = if query.is_empty() { names[t].clone() } else { String::new() };
        let base = format!("/{}", pointer_escape(&list));
        // Filled slots, then a few empty ones (the save lists more). Measured against the file
        // as read, so the grid holds still while editing.
        let keys: Vec<String> = match (cx.get(&base), cx.baseline.pointer(&base)) {
            _ if !query.is_empty() => vec![],
            (Some(serde_json::Value::Object(m)), Some(serde_json::Value::Object(orig))) => {
                let is_empty = |v: &serde_json::Value| v.get("Item").and_then(|x| x.as_str()) == Some(crate::fmt::sekiro_inv::EMPTY);
                let last = orig.iter().rposition(|(_, v)| !is_empty(v)).map_or(0, |i| i + 1);
                let mut free = 0;
                m.keys()
                    .zip(orig.values())
                    .enumerate()
                    .filter(|(i, (_, v))| {
                        let e = is_empty(v);
                        free += (e && *i >= last) as usize;
                        !e || *i < last || free <= 8
                    })
                    .map(|(_, (k, _))| k.clone())
                    .collect()
            }
            _ => vec![],
        };
        if g == Game::EldenRing {
            if let Some(serde_json::Value::Object(eq)) = cx.get("/Equipment (carried and stored)") {
                let parts: Vec<String> = eq.iter().filter_map(|(k, v)| v.as_u64().filter(|&n| n > 0).map(|n| format!("{n} {}", k.to_lowercase()))).collect();
                if !parts.is_empty() {
                    ui.weak(format!("Also in these lists (not edited here): {}.", parts.join(", ")));
                }
            }
        }
        let slots: Vec<(String, String)> = if query.is_empty() { keys.into_iter().map(|k| (list.clone(), k)).collect() } else { found };
        let id = if query.is_empty() { format!("fs_inv_{t}") } else { "fs_inv_found".to_string() };
        let cells: Vec<Cell> = slots
            .iter()
            .map(|(l, k)| {
                let p = format!("/{}/{}", pointer_escape(l), pointer_escape(k));
                let item = cx.text(&format!("{p}/Item")).unwrap_or_default();
                let empty = item == crate::fmt::sekiro_inv::EMPTY;
                // Weapons: "Heavy Dagger +25".
                let affinity = cx.text(&format!("{p}/Affinity")).or_else(|| cx.text(&format!("{p}/Infusion")));
                let title = match (cx.int(&format!("{p}/Level")), affinity) {
                    (Some(l), Some(a)) => {
                        let a = if ["Standard", "Normal", crate::fmt::er_inv::NO_AFFINITY].contains(&a.as_str()) { String::new() } else { format!("{a} ") };
                        format!("{a}{item}{}", if l > 0 { format!(" +{l}") } else { String::new() })
                    }
                    _ => item,
                };
                Cell {
                    title: if empty { String::new() } else { title },
                    count: cx.int(&format!("{p}/Quantity")),
                    empty,
                    modified: ["Item", "Quantity", "Level", "Affinity", "Infusion"].iter().any(|f| cx.modified(&format!("{p}/{f}"))),
                    tint: None,
                }
            })
            .collect();
        let sel = selection(ui, &id);
        if let Some(i) = cell_grid(ui, &cells, 10, sel, accent) {
            select(ui, &id, if sel == Some(i) { None } else { Some(i) });
        }
        let Some(i) = selection(ui, &id).filter(|&i| i < slots.len()) else {
            ui.weak(if query.is_empty() {
                "Click an item to change it or its quantity, or an empty slot to add one."
            } else {
                "Everything you have that matches, from every list. Click one to change it."
            });
            return;
        };
        let (l, k) = &slots[i];
        let p = format!("/{}/{}", pointer_escape(l), pointer_escape(k));
        egui::Frame::none().fill(inset_fill(ui, accent)).rounding(8.0).inner_margin(10.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(if query.is_empty() { k.clone() } else { format!("{l} \u{203A} {k}") }).strong());
                if ui.small_button("Close").clicked() {
                    select(ui, &id, None);
                }
            });
            egui::Grid::new((id.as_str(), "slot")).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                cx.grid_row(ui, "Item", &format!("{p}/Item"), 260.0);
                cx.grid_row(ui, "Quantity", &format!("{p}/Quantity"), 100.0);
                cx.grid_row(ui, "Level", &format!("{p}/Level"), 100.0);
                cx.grid_row(ui, "Affinity", &format!("{p}/Affinity"), 160.0);
                cx.grid_row(ui, "Infusion", &format!("{p}/Infusion"), 160.0);
            });
            ui.weak(match g {
                Game::Sekiro => "Pick (empty) to remove an item. Items set in a quick-item slot can't be swapped here.",
                _ => {
                    "Pick (empty) to remove something you added here. What's equipped or in a quick slot can't be swapped, \
                     and a weapon's level stays within what it can have."
                }
            });
        });
    });
}

/// Elden Ring: Sites of Grace and bosses, as ticks.
fn progress(ui: &mut Ui, cx: &mut GameCtx) {
    let lists = ["Sites of Grace", "Bosses defeated"];
    if cx.get(lists[0]).is_none() && cx.get(&format!("/{}", lists[0])).is_none() {
        return;
    }
    panel(ui, "Progress", |ui| {
        let names: Vec<String> = lists.iter().map(|s| s.to_string()).collect();
        let t = tabs(ui, "er_progress_tab", &names);
        let base = format!("/{}", lists[t]);
        let Some(serde_json::Value::Object(m)) = cx.get(&base).cloned() else { return };
        let on = m.values().filter(|v| v.as_bool() == Some(true)).count();
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{on} of {}", m.len()));
            let dlc: std::collections::HashSet<&str> = crate::fmt::er_inv::dlc_graces().collect();
            let tick = |cx: &mut GameCtx, pick: &dyn Fn(&str) -> bool| {
                for k in m.keys().filter(|k| pick(k)) {
                    cx.set(&format!("{base}/{}", pointer_escape(k)), serde_json::Value::Bool(true));
                }
            };
            if t == 0 {
                if ui.button("Unlock base game graces").clicked() {
                    tick(cx, &|k| !dlc.contains(k));
                }
                if ui.button("Unlock Shadow of the Erdtree graces").on_hover_text("Only useful if you own the DLC").clicked() {
                    tick(cx, &|k| dlc.contains(k));
                }
            } else if ui.button("Mark every boss defeated").on_hover_text("Bosses marked defeated stay gone; their rewards aren't given.").clicked() {
                tick(cx, &|_| true);
            }
        });
        egui::ScrollArea::vertical().max_height(320.0).id_salt(("er_progress", t)).show(ui, |ui| {
            for (k, v) in &m {
                let p = format!("{base}/{}", pointer_escape(k));
                let mut b = v.as_bool().unwrap_or(false);
                let text = if cx.modified(&p) { RichText::new(k).color(crate::theme::MODIFIED) } else { RichText::new(k) };
                if ui.checkbox(&mut b, text).changed() {
                    cx.set(&p, serde_json::Value::Bool(b));
                }
            }
        });
        ui.weak("Graces you unlock show up on the map for fast travel. Shadow of the Erdtree graces need the DLC.");
    });
}

fn egui_grid(ui: &mut Ui, cx: &mut GameCtx, rows: &[(&str, &str)]) {
    eframe::egui::Grid::new(("fs_grid", rows.len())).num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
        for (label, p) in rows {
            cx.grid_row(ui, label, p, 120.0);
        }
    });
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn sekiro_materials_fill_to_max() {
        let mut items = serde_json::Map::new();
        items.insert("Slot 1".into(), json!({"Item": "Scrap Iron", "Quantity": 99}));
        items.insert("Slot 2".into(), json!({"Item": "Black Gunpowder", "Quantity": 3}));
        for i in 3..=12 {
            items.insert(format!("Slot {i}"), json!({"Item": "(empty)", "Quantity": 0}));
        }
        let root = json!({ "Items": items });
        let mut cx = super::GameCtx { root: &root, baseline: &root, tree: None, edits: vec![], archive: None, open_entry: None };
        super::give_materials(super::Game::Sekiro, &mut cx);
        let e: std::collections::HashMap<String, serde_json::Value> = cx.edits.iter().cloned().collect();
        assert!(!e.contains_key("/Items/Slot 1/Quantity"), "already at the maximum");
        assert_eq!(e["/Items/Slot 2/Quantity"], 99);
        assert_eq!(e["/Items/Slot 3/Item"], "Scrap Magnetite");
        assert_eq!(e["/Items/Slot 3/Quantity"], 99);
        assert_eq!(e["/Items/Slot 9/Item"], "Lapis Lazuli");
        assert!(!e.contains_key("/Items/Slot 10/Item"));
    }

    #[test]
    fn dark_souls_materials_are_real_items() {
        for (g, name) in [(super::Game::DarkSouls3, "Dark Souls III"), (super::Game::DarkSouls2, "Dark Souls II"), (super::Game::DarkSouls1, "Dark Souls")] {
            let known = crate::fmt::known_goods(name);
            let (list, max, _) = super::materials(g);
            assert_eq!(max, 99);
            for m in &list {
                assert!(known.contains(m.as_str()), "{name}: {m}");
            }
        }
    }

    #[test]
    fn dark_souls_level_costs() {
        assert_eq!(super::ds_soul_cost(1), 673);
        assert_eq!(super::ds_soul_cost(11), 847);
        assert_eq!(super::ds_soul_cost(12), 1038);
        assert_eq!(super::ds_soul_cost(100), 61_591);
    }

    #[test]
    fn elden_ring_materials_are_real_items() {
        let (list, max, _) = super::materials(super::Game::EldenRing);
        assert_eq!((list.len(), max), (39, 999));
        for m in &list {
            assert!(crate::fmt::er_inv::good_id(m).is_some(), "{m}");
        }
    }

    #[test]
    fn elden_ring_level_costs() {
        // The game's table: 1 → 2 costs 673 runes, 2 → 3 costs 689.
        assert_eq!(super::er_rune_cost(1), 673);
        assert_eq!(super::er_rune_cost(2), 689);
    }
}
