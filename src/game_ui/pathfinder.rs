//! Pathfinder: Kingmaker and Wrath of the Righteous (Owlcat `.zks`, shown as one JSON
//! document with `header`, `player` and `party`). The JSON keeps .NET references: any
//! object can be `{"$ref": "N"}` pointing at the object whose `$id` is N in the same file,
//! so values are found through those links.

use super::{columns, fact, hero, panel, tabs, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, Ui};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

const ABILITIES: [(&str, &str); 6] = [
    ("Strength", "STR"),
    ("Dexterity", "DEX"),
    ("Constitution", "CON"),
    ("Intelligence", "INT"),
    ("Wisdom", "WIS"),
    ("Charisma", "CHA"),
];

pub fn is_pathfinder(root: &Value) -> bool {
    root.pointer("/party/m_EntityData").is_some() && root.pointer("/player/Money").is_some()
}

thread_local! {
    /// `$id` → pointer under `/party`, for the document last indexed (by address and size).
    static IDS: RefCell<(usize, usize, HashMap<String, String>)> = RefCell::new((0, 0, HashMap::new()));
}

fn index(v: &Value, p: &mut String, out: &mut HashMap<String, String>) {
    match v {
        Value::Object(m) => {
            if let Some(Value::String(id)) = m.get("$id") {
                out.insert(id.clone(), p.clone());
            }
            for (k, x) in m {
                if matches!(x, Value::Object(_) | Value::Array(_)) {
                    let n = p.len();
                    p.push('/');
                    p.push_str(&pointer_escape(k));
                    index(x, p, out);
                    p.truncate(n);
                }
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                if matches!(x, Value::Object(_) | Value::Array(_)) {
                    let n = p.len();
                    p.push('/');
                    p.push_str(&i.to_string());
                    index(x, p, out);
                    p.truncate(n);
                }
            }
        }
        _ => {}
    }
}

/// The pointer of the object at `p` under `/party`, following a `$ref` if it is one.
fn resolve(root: &Value, p: &str) -> Option<String> {
    let v = root.pointer(p)?;
    let Some(Value::String(r)) = v.get("$ref") else { return Some(p.to_string()) };
    let party = root.get("party")?;
    IDS.with(|c| {
        let mut c = c.borrow_mut();
        let key = (party as *const Value as usize, party.get("m_EntityData").and_then(Value::as_array).map_or(0, Vec::len));
        let stale = (c.0, c.1) != key || c.2.get(r).and_then(|q| root.pointer(&format!("/party{q}"))).and_then(|o| o.get("$id")).and_then(Value::as_str) != Some(r);
        if stale {
            let mut m = HashMap::new();
            index(party, &mut String::new(), &mut m);
            *c = (key.0, key.1, m);
        }
        c.2.get(r).map(|q| format!("/party{q}"))
    })
}

fn child(root: &Value, base: &str, key: &str) -> Option<String> {
    resolve(root, &format!("{base}/{}", pointer_escape(key)))
}

fn unit_id(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(m) => m.get("m_UniqueId").and_then(Value::as_str).map(str::to_string),
        _ => None,
    }
}

fn guid_name(g: &str, wotr: bool) -> Option<String> {
    crate::fmt::owlcat::name(g, wotr)
}

pub fn show(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(190, 150, 70);
    let root = cx.root;
    let wotr = root.pointer("/player/Corruption").is_some() || root.pointer("/player/m_LeadersManager").is_some();
    let title = if wotr { "Pathfinder: Wrath of the Righteous" } else { "Pathfinder: Kingmaker" };
    let save = cx.text("/header/Name").unwrap_or_default();
    let main = root.pointer("/player/m_MainCharacter").and_then(unit_id);
    let party: Vec<String> = root.pointer("/player/PartyCharacters").and_then(Value::as_array).map(|a| a.iter().filter_map(unit_id).collect()).unwrap_or_default();
    // Party members: (label, pointer of the unit's Descriptor).
    let mut members: Vec<(String, String)> = vec![];
    if let Some(units) = root.pointer("/party/m_EntityData").and_then(Value::as_array) {
        for (i, u) in units.iter().enumerate() {
            let Some(id) = u.get("UniqueId").and_then(Value::as_str) else { continue };
            if !party.iter().any(|p| p == id) && main.as_deref() != Some(id) {
                continue;
            }
            let Some(desc) = child(root, &format!("/party/m_EntityData/{i}"), "Descriptor") else { continue };
            let custom = root.pointer(&format!("{desc}/CustomName")).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
            let bp = root.pointer(&format!("{desc}/Blueprint")).and_then(Value::as_str).and_then(|g| guid_name(g, wotr)).filter(|n| n != "Player" && n != "Custom");
            let name = custom.or(bp).unwrap_or_else(|| format!("Companion {}", members.len() + 1));
            if main.as_deref() == Some(id) {
                members.insert(0, (name, desc));
            } else {
                members.push((name, desc));
            }
        }
    }
    hero(ui, accent, title, &save, |ui| {
        cx.tile(ui, "Money", "/player/Money", accent);
        if let Some(c) = cx.int("/player/Chapter") {
            fact(ui, "Chapter", &c.to_string(), accent);
        }
        fact(ui, "Party", &members.len().to_string(), accent);
    });
    if members.is_empty() {
        ui.weak("No party members found in this save.");
        return;
    }
    let names: Vec<String> = members.iter().map(|m| m.0.clone()).collect();
    let t = tabs(ui, "pf_party", &names);
    let desc = members[t].1.clone();
    let stats = child(root, &desc, "Stats");
    let prog = child(root, &desc, "Progression");
    columns(ui, 360.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Ability scores", |ui| {
                for (k, short) in ABILITIES {
                    let Some(s) = stats.as_deref().and_then(|s| child(cx.root, s, k)) else { continue };
                    let p = format!("{s}/m_BaseValue");
                    if cx.get(&p).is_some() {
                        cx.attribute(ui, &format!("{k} ({short})"), &p, 30.0, accent);
                    }
                }
                ui.weak("Base scores, before race, items and buffs.");
            });
        } else {
            panel(ui, "Progression", |ui| {
                if let Some(p) = &prog {
                    for (label, k) in [("Experience", "Experience"), ("Mythic rank", "MythicExperience")] {
                        let q = format!("{p}/{k}");
                        if cx.get(&q).is_some() {
                            cx.grid_row(ui, label, &q, 120.0);
                        }
                    }
                    if let Some(classes) = cx.get(&format!("{p}/Classes")).and_then(Value::as_array).cloned() {
                        ui.add_space(6.0);
                        for c in classes {
                            let g = c.get("CharacterClass").and_then(Value::as_str).unwrap_or_default();
                            let name = guid_name(g, wotr).unwrap_or_else(|| "Class".into());
                            let lv = c.get("Level").and_then(Value::as_i64).unwrap_or(0);
                            ui.label(format!("{name} {lv}"));
                        }
                    }
                }
                if let Some(hp) = stats.as_deref().and_then(|s| child(cx.root, s, "HitPoints")) {
                    let q = format!("{hp}/m_BaseValue");
                    if cx.get(&q).is_some() {
                        ui.add_space(6.0);
                        cx.grid_row(ui, "Hit points (base)", &q, 80.0);
                    }
                }
                ui.weak("Experience is shared by the party; raising it gives level-ups to pick in the game.");
            });
        }
    });
    story_flags(ui, cx, wotr);
}

/// Kingmaker's story flags (`player.m_UnlockableFlags`): each a blueprint GUID and a number,
/// named from the game's blueprint list. Wrath's flag names aren't known, so it has none.
fn story_flags(ui: &mut Ui, cx: &mut GameCtx, wotr: bool) {
    const AT: &str = "/player/m_UnlockableFlags/m_UnlockedFlags";
    let Some(Value::Array(list)) = cx.get(AT) else { return };
    let mut rows: Vec<(String, String)> = list
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let name = if wotr { None } else { e.get("Key").and_then(Value::as_str).and_then(crate::fmt::owlcat::kingmaker_flag) }?;
            e.get("Value").filter(|v| v.is_i64() || v.is_u64())?;
            Some((name.to_string(), format!("{AT}/{i}/Value")))
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    rows.sort_by_key(|r| r.0.to_lowercase());
    panel(ui, &format!("Story flags ({})", rows.len()), |ui| {
        super::screens2::story_guard(ui, |ui| {
            let q = super::screens2::filter_box(ui, "pf_flags", "e.g. Romance, Amiri, Unlock");
            egui::ScrollArea::vertical().id_salt("pf_flags_scroll").max_height(340.0).show(ui, |ui| {
                for (n, p) in rows.iter().filter(|r| q.is_empty() || r.0.to_lowercase().contains(&q)).take(400) {
                    cx.row_fixed(ui, n, p, 320.0, 90.0);
                }
            });
            ui.weak("The game's own story and unlock flags: most are 0 or 1, some count up. Shows 400 at a time: use the filter.");
        });
    });
}
