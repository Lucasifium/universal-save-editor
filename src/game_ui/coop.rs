//! Co-op horror games that keep their saves in Easy Save 3 files: Phasmophobia
//! (`SaveFile.txt`), Lethal Company (`LCSaveFile1…`, `LCGeneralSaveData`) and R.E.P.O.
//! (`<save>.es3`). Every value sits in an ES3 `{"__type", "value"}` pair.

use super::screens2::{filter_box, value_rows};
use super::{fact, hero, panel, tabs, words, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{Color32, Ui};
use serde_json::Value;

/// The pointer of an ES3 entry's value.
fn val(key: &str) -> String {
    format!("/{}/value", pointer_escape(key))
}

/// (label, pointer) rows for the top-level ES3 entries `pick` gives a label to.
fn entries(cx: &GameCtx, pick: impl Fn(&str, &Value) -> Option<String>) -> Vec<(String, String)> {
    let Some(Value::Object(m)) = cx.get("") else { return vec![] };
    m.iter().filter_map(|(k, e)| pick(k, e.get("value")?).map(|label| (label, val(k)))).collect()
}

fn filtered(rows: Vec<(String, String)>, q: &str) -> Vec<(String, String)> {
    rows.into_iter().filter(|(l, _)| q.is_empty() || l.to_lowercase().contains(q)).collect()
}

// ------------------------------------------------------------------ Phasmophobia

pub fn is_phasmophobia(root: &Value) -> bool {
    root.pointer("/PlayersMoney/value").is_some() && (root.pointer("/NewLevel/value").is_some() || root.pointer("/Experience/value").is_some())
}

pub fn phasmophobia(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(120, 190, 150);
    let mut sub = vec!["Phasmophobia".to_string()];
    if let Some(n) = cx.int(&val("ghostsIdentifiedAmount")) {
        sub.push(format!("{n} ghosts identified"));
    }
    hero(ui, accent, "Ghost hunter", &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Money ($)", &val("PlayersMoney"), accent);
        cx.tile(ui, "Level", &val("NewLevel"), accent);
        cx.tile(ui, "Prestige", &val("Prestige"), accent);
        cx.tile(ui, "Experience", &val("Experience"), accent);
    });
    // The lobby badge follows the prestige number.
    if let (Some(p), Some(i)) = (cx.int(&val("Prestige")), cx.int(&val("PrestigeIndex"))) {
        if p != i && cx.modified(&val("Prestige")) {
            cx.set(&val("PrestigeIndex"), Value::from(p));
        }
    }
    let names = ["Equipment", "Tiers", "Unlocks", "Statistics"];
    let t = tabs(ui, "phasmo_tabs", &names.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    panel(ui, names[t], |ui| {
        let q = filter_box(ui, "phasmo_filter", "name");
        ui.add_space(4.0);
        match t {
            0 => {
                let rows = entries(cx, |k, v| (v.is_number() && !k.starts_with("committed")).then(|| k.strip_suffix("Inventory").map(words)).flatten());
                value_rows(ui, cx, "phasmo_eq", &filtered(rows, &q));
                ui.weak("How many of each item you own. The shop sells them one at a time; the save itself takes any number.");
            }
            1 => {
                let rows = entries(cx, |k, v| v.is_number().then(|| k.strip_suffix("Tier").map(words)).flatten());
                value_rows(ui, cx, "phasmo_tier", &filtered(rows, &q));
                ui.weak("The tier of each item in your loadout: 0, 1 or 2 for tiers I, II and III. A tier also has to be unlocked (next tab).");
            }
            2 => {
                let rows = entries(cx, |k, v| v.is_boolean().then(|| k.strip_suffix("UnlockOwned").map(words)).flatten());
                let rows = filtered(rows, &q);
                ui.horizontal_wrapped(|ui| {
                    for (label, p) in &rows {
                        cx.chip(ui, label, p, accent);
                    }
                });
                if rows.is_empty() {
                    ui.weak("No unlock flags in this save.");
                }
            }
            _ => {
                let skip = ["PlayersMoney", "NewLevel", "Prestige", "PrestigeIndex", "Experience"];
                let rows = entries(cx, |k, v| {
                    let other = k.ends_with("Inventory") || k.ends_with("Tier") || k.ends_with("Seed") || k.starts_with("daily") || k.starts_with("weekly") || skip.contains(&k);
                    (v.is_number() && !other).then(|| words(k))
                });
                value_rows(ui, cx, "phasmo_stats", &filtered(rows, &q));
                ui.weak("Counters from the journal's statistics pages and the other numbers the save keeps.");
            }
        }
    });
    ui.weak("Close the game before saving here, and turn Steam Cloud off for one start, or the game may bring the old save back. Checked on public test saves, not yet tried in the game.");
}

// ------------------------------------------------------------------ Lethal Company

pub fn is_lethal_company(root: &Value) -> bool {
    let has = |k: &str| root.pointer(&val(k)).is_some();
    (has("GroupCredits") && has("ProfitQuota")) || (has("PlayerXPNum") && has("PlayerLevel"))
}

pub fn lethal_company(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(230, 110, 40);
    if cx.get(&val("GroupCredits")).is_none() {
        // LCGeneralSaveData: the player's own rank, shared by every save file.
        hero(ui, accent, "Employee record", "Lethal Company  \u{00B7}  LCGeneralSaveData", |ui| {
            cx.tile(ui, "Experience", &val("PlayerXPNum"), accent);
            cx.tile(ui, "Rank", &val("PlayerLevel"), accent);
            cx.tile(ui, "Times landed", &val("TimesLanded"), accent);
        });
        ui.weak("Rank goes from 0 (Intern) up; the game works it out again from experience after a round. Credits, the quota and the ship are in LCSaveFile1 to LCSaveFile3. Checked on public saves, not yet tried in the game.");
        return;
    }
    let mut sub = vec!["Lethal Company".to_string()];
    if let Some(d) = cx.int(&val("Stats_DaysSpent")) {
        sub.push(format!("day {d}"));
    }
    if let Some(v) = cx.int(&val("FileGameVers")) {
        sub.push(format!("version {v}"));
    }
    hero(ui, accent, "The crew", &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Credits", &val("GroupCredits"), accent);
        cx.tile(ui, "Profit quota", &val("ProfitQuota"), accent);
        cx.tile(ui, "Quota fulfilled", &val("QuotaFulfilled"), accent);
        cx.tile(ui, "Quotas passed", &val("QuotasPassed"), accent);
        cx.tile(ui, "Deadline (minutes)", &val("DeadlineTime"), accent);
        if let Some(t) = cx.num(&val("DeadlineTime")) {
            fact(ui, "Days left", &format!("{:.0}", (t / 1080.0).floor()), accent);
        }
    });
    panel(ui, "Ship and moon", |ui| {
        let rows: Vec<(String, String)> = [("Current moon (number)", "CurrentPlanetID"), ("Random seed", "RandomSeed"), ("Time of day", "GlobalTime")].iter().map(|(l, k)| (l.to_string(), val(k))).collect();
        value_rows(ui, cx, "lc_ship", &rows);
        ui.weak("The deadline counts in-game minutes: a day is 1,080, so 3,240 is the full three days.");
    });
    let stored = entries(cx, |k, v| v.is_boolean().then(|| k.strip_prefix("ShipUnlockStored_").map(str::to_string)).flatten());
    if !stored.is_empty() {
        panel(ui, "Ship upgrades put in storage", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, p) in &stored {
                    cx.chip(ui, label, p, accent);
                }
            });
            ui.weak("On means the item sits in the ship's storage instead of standing in the ship.");
        });
    }
    let stats = entries(cx, |k, v| v.is_number().then(|| k.strip_prefix("Stats_").map(words)).flatten());
    if !stats.is_empty() {
        panel(ui, "Statistics", |ui| value_rows(ui, cx, "lc_stats", &stats));
    }
    ui.weak("Checked on public saves, not yet tried in the game.");
}

// ------------------------------------------------------------------ R.E.P.O.

const REPO: &str = "/dictionaryOfDictionaries/value";

pub fn is_repo(root: &Value) -> bool {
    root.pointer("/dictionaryOfDictionaries/value/runStats").is_some()
}

fn dict_rows(cx: &GameCtx, name: &str, label: impl Fn(&str) -> String) -> Vec<(String, String)> {
    let base = format!("{REPO}/{name}");
    match cx.get(&base) {
        Some(Value::Object(m)) => m.keys().map(|k| (label(k), format!("{base}/{}", pointer_escape(k)))).collect(),
        _ => vec![],
    }
}

pub fn repo(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(240, 200, 60);
    let team = cx.text("/teamName/value").filter(|t| !t.is_empty()).unwrap_or_else(|| "R.E.P.O.".into());
    let mut sub = vec!["R.E.P.O.".to_string()];
    if let Some(d) = cx.text("/dateAndTime/value") {
        sub.push(d);
    }
    if let Some(t) = cx.num("/timePlayed/value") {
        sub.push(format!("{:.1} hours", t / 3600.0));
    }
    hero(ui, accent, &team, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Level", &format!("{REPO}/runStats/level"), accent);
        cx.tile(ui, "Money ($K)", &format!("{REPO}/runStats/currency"), accent);
        cx.tile(ui, "Lives", &format!("{REPO}/runStats/lives"), accent);
        cx.tile(ui, "Charging station", &format!("{REPO}/runStats/chargingStationCharge"), accent);
        cx.tile(ui, "Total haul ($K)", &format!("{REPO}/runStats/totalHaul"), accent);
    });
    // One card per player: health and every upgrade bought.
    let players: Vec<String> = match cx.get(&format!("{REPO}/playerHealth")) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    let upgrades: Vec<String> = match cx.get(REPO) {
        Some(Value::Object(m)) => m.keys().filter(|k| k.starts_with("playerUpgrade")).cloned().collect(),
        _ => vec![],
    };
    for id in &players {
        let name = cx.text(&format!("/playerNames/value/{}", pointer_escape(id))).unwrap_or_else(|| "Player".into());
        panel(ui, &name, |ui| {
            let mut rows = vec![("Health".to_string(), format!("{REPO}/playerHealth/{}", pointer_escape(id)))];
            for u in &upgrades {
                rows.push((words(u.trim_start_matches("playerUpgrade")), format!("{REPO}/{u}/{}", pointer_escape(id))));
            }
            rows.push(("Has the crown (0 or 1)".to_string(), format!("{REPO}/playerHasCrown/{}", pointer_escape(id))));
            value_rows(ui, cx, &format!("repo_{id}"), &rows);
        });
    }
    let lists = [("In the truck", "itemsPurchased"), ("Bought in total", "itemsPurchasedTotal"), ("Item upgrades", "itemsUpgradesPurchased")];
    let t = tabs(ui, "repo_items", &lists.iter().map(|l| l.0.to_string()).collect::<Vec<_>>());
    panel(ui, lists[t].0, |ui| {
        let q = filter_box(ui, "repo_filter", "item");
        ui.add_space(4.0);
        let rows = filtered(dict_rows(cx, lists[t].1, |k| k.trim_start_matches("Item ").to_string()), &q);
        value_rows(ui, cx, &format!("repo_items_{t}"), &rows);
        if t == 0 {
            ui.weak("The items the team owns for the next level. Each health upgrade is worth 20 health on top of 100.");
        }
    });
    ui.weak("Only the host's save counts in a multiplayer game. Checked on a public save, not yet tried in the game.");
}
