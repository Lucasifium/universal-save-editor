//! Pathfinder: Kingmaker and Wrath of the Righteous (Owlcat `.zks`, shown as one JSON
//! document with `header`, `player` and `party`). The JSON keeps .NET references: any
//! object can be `{"$ref": "N"}` pointing at the object whose `$id` is N in the same file,
//! so values are found through those links.

use crate::i18n::{tr};
use crate::widgets::ListBox;
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
            let name = custom.or(bp).unwrap_or_else(|| crate::i18n::trf("Companion {0}", &[&(members.len() + 1)]));
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
        ui.weak(tr("No party members found in this save."));
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
                let rows: Vec<(String, String)> = ABILITIES
                    .iter()
                    .filter_map(|(k, short)| {
                        let s = stats.as_deref().and_then(|s| child(cx.root, s, k))?;
                        let p = format!("{s}/m_BaseValue");
                        cx.get(&p).is_some().then(|| (format!("{k} ({short})"), p))
                    })
                    .collect();
cx.open_attributes(ui, &rows);
                ui.weak(tr("Base scores, before race, items and buffs."));
            });
        } else {
            panel(ui, "Progression", |ui| {
                // One row style for the editable values: name on the left, the box on the right.
                let mut rows: Vec<(&str, String, f32)> = vec![];
                if let Some(p) = &prog {
                    rows.push(("Experience", format!("{p}/Experience"), 120.0));
                    rows.push(("Mythic rank", format!("{p}/MythicExperience"), 120.0));
                }
                if let Some(hp) = stats.as_deref().and_then(|s| child(cx.root, s, "HitPoints")) {
                    rows.push(("Hit points (base)", format!("{hp}/m_BaseValue"), 80.0));
                }
                let rows: Vec<(&str, String, f32)> = rows.into_iter().filter(|r| cx.get(&r.1).is_some()).collect();
                super::row_columns(ui, 300.0, rows.len(), |ui, range, _| {
                    for (label, q, w) in &rows[range] {
                        cx.row_fixed(ui, label, q, 110.0, *w);
                    }
                });
                let classes = prog.as_ref().and_then(|p| cx.get(&format!("{p}/Classes"))).and_then(Value::as_array).cloned().unwrap_or_default();
                if !classes.is_empty() {
                    ui.add_space(6.0);
                    let names: Vec<String> = classes
                        .iter()
                        .map(|c| {
                            let g = c.get("CharacterClass").and_then(Value::as_str).unwrap_or_default();
                            let name = guid_name(g, wotr).unwrap_or_else(|| crate::i18n::tr("Class").into());
                            let lv = c.get("Level").and_then(Value::as_i64).unwrap_or(0);
                            format!("{name} {lv}")
                        })
                        .collect();
                    ui.label(names.join("  \u{00B7}  ")); // i18n-ok: class names and their levels
                }
                ui.weak(tr("Experience is shared by the party; raising it gives level-ups to pick in the game."));
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
    panel(ui, &crate::i18n::trf("Story flags ({0})", &[&rows.len()]), |ui| {
        super::screens2::story_guard(ui, |ui| {
            let q = super::screens2::filter_box(ui, "pf_flags", "e.g. Romance, Amiri, Unlock");
            egui::ScrollArea::vertical().show_list(ui, "pf_flags_scroll", 340.0, |ui| {
                for (n, p) in rows.iter().filter(|r| q.is_empty() || r.0.to_lowercase().contains(&q)).take(400) {
                    cx.row_fixed(ui, n, p, 320.0, 90.0);
                }
            });
            ui.weak(tr("The game's own story and unlock flags: most are 0 or 1, some count up. Shows 400 at a time: use the filter."));
        });
    });
}
