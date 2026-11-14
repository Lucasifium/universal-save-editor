//! Screens for Green Hell, RE Engine game slots, Hogwarts Legacy, Ren'Py, Europa
//! Universalis IV and Crusader Kings II. As elsewhere, values are found by name and
//! simply left out when a save doesn't have them.

use crate::i18n::tr;
use crate::widgets::ScrollLock;
use super::{columns, crafting, fact, hero, panel, tabs, words, Game, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

/// Pointers (at most `limit`, depth-first, in file order) of objects that have `key`.
pub fn objects_with(root: &Value, key: &str, limit: usize) -> Vec<String> {
    fn walk(v: &Value, p: &mut String, key: &str, out: &mut Vec<String>, limit: usize, depth: usize) {
        if out.len() >= limit || depth > 24 {
            return;
        }
        match v {
            Value::Object(m) => {
                if m.contains_key(key) {
                    out.push(p.clone());
                }
                for (k, x) in m {
                    if matches!(x, Value::Object(_) | Value::Array(_)) {
                        let n = p.len();
                        p.push('/');
                        p.push_str(&pointer_escape(k));
                        walk(x, p, key, out, limit, depth + 1);
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
                        walk(x, p, key, out, limit, depth + 1);
                        p.truncate(n);
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    walk(root, &mut String::new(), key, &mut out, limit, 0);
    out
}

/// A filter box remembered under `id`; returns the lowercase query.
pub(super) fn filter_box(ui: &mut Ui, id: &str, hint: &str) -> String {
    let sid = egui::Id::new(("screen_filter", id));
    let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(tr("Filter:"));
        ui.add(egui::TextEdit::singleline(&mut q).hint_text(hint).desired_width(220.0));
    });
    ui.data_mut(|d| d.insert_temp(sid, q.clone()));
    q.to_lowercase()
}

/// Label + editor rows for the given (label, pointer) pairs, spread over columns.
pub(super) fn value_rows(ui: &mut Ui, cx: &mut GameCtx, id: &str, rows: &[(String, String)]) {
    let rows: Vec<&(String, String)> = rows.iter().filter(|(_, p)| cx.get(p).is_some()).collect();
    if rows.is_empty() {
        return;
    }
    let cols = ((ui.available_width() / 340.0).floor() as usize).clamp(1, 3);
    let per = rows.len().div_ceil(cols);
    ui.columns(cols, |cs| {
        for (c, chunk) in rows.chunks(per.max(1)).enumerate() {
            let ui = &mut cs[c];
            ui.push_id((id, c), |ui| {
                let label_w = (ui.available_width() - 230.0).clamp(70.0, 220.0);
                for (label, p) in chunk {
                    cx.row_fixed(ui, label, p, label_w, 90.0);
                }
            });
        }
    });
}

/// Every scalar (non-object) value of an object as (label, pointer) rows.
fn scalar_rows(cx: &GameCtx, base: &str, keep: impl Fn(&str, &Value) -> bool) -> Vec<(String, String)> {
    match cx.get(base) {
        Some(Value::Object(m)) => m
            .iter()
            .filter(|(k, v)| !matches!(v, Value::Object(_) | Value::Array(_)) && keep(k, v))
            .map(|(k, _)| (words(k), join(base, k)))
            .collect(),
        _ => vec![],
    }
}

// ------------------------------------------------------------------ Green Hell

/// The message of a Green Hell save that holds `key` (the save is a run of messages).
fn gh_part(root: &Value, key: &str) -> Option<String> {
    root.as_array()?.iter().position(|m| m.get(key).is_some()).map(|i| format!("/{i}"))
}

pub fn is_green_hell(root: &Value) -> bool {
    gh_part(root, "PCM_HP").is_some()
}

pub fn green_hell(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(70, 150, 70);
    let Some(body) = gh_part(cx.root, "PCM_HP") else { return };
    let world = gh_part(cx.root, "Day").unwrap_or_else(|| body.clone());
    let day = cx.int(&format!("{world}/Day")).unwrap_or(0);
    let month = cx.int(&format!("{world}/Month")).unwrap_or(0);
    let year = cx.int(&format!("{world}/Year")).unwrap_or(0);
    let hour = cx.int(&format!("{body}/Hour"));
    let played = cx.num(&format!("{body}/StatPlayTime")).unwrap_or(0.0);
    let mut sub = vec![];
    if year > 0 {
        sub.push(format!("{day}/{month}/{year}{}", hour.map(|h| format!(", {h}:00")).unwrap_or_default()));
    }
    if played > 0.0 {
        sub.push(format!("{:.1} hours played", played / 3600.0));
    }
    hero(ui, accent, "Green Hell", &sub.join("  \u{00B7}  "), |ui| {
        if let Some(d) = cx.int(&format!("{world}/StatDaysSurvived")) {
            fact(ui, "Days survived", &d.to_string(), accent);
        }
        cx.tile(ui, "Sanity", &format!("{world}/Sanity"), accent);
    });
    panel(ui, "Condition", |ui| {
        let rows = [
            ("Health", "PCM_HP", Color32::from_rgb(200, 60, 60)),
            ("Energy", "PCM_Energy", Color32::from_rgb(220, 180, 50)),
            ("Hydration", "PCM_Hydration", Color32::from_rgb(60, 140, 220)),
            ("Carbohydrates", "PCM_NutritionCarbo", Color32::from_rgb(120, 190, 80)),
            ("Fat", "PCM_NutritionFat", Color32::from_rgb(230, 200, 90)),
            ("Proteins", "PCM_NutritionProteins", Color32::from_rgb(200, 110, 80)),
            ("Dirtiness", "PCM_Dirtiness", Color32::from_rgb(140, 110, 80)),
        ];
        for (name, key, color) in rows {
            cx.attribute(ui, name, &format!("{body}/{key}"), 100.0, color);
        }
        ui.weak("Each runs 0\u{2013}100 on normal settings. Low nutrition shrinks the most health you can have.");
    });
    let skills: Vec<(String, String)> = match cx.get(&body) {
        Some(Value::Object(m)) => m
            .keys()
            .filter_map(|k| k.strip_prefix("Skill").and_then(|s| s.strip_suffix("Skill")).map(|s| (words(s), join(&body, k))))
            .collect(),
        _ => vec![],
    };
    if !skills.is_empty() {
        panel(ui, "Skills", |ui| {
            columns(ui, 300.0, skills.len(), |ui, i| {
                let (name, p) = &skills[i];
                cx.attribute(ui, name, p, 100.0, accent);
            });
            ui.weak("Skills run 0\u{2013}100.");
        });
    }
    panel(ui, "Statistics", |ui| {
        let mut rows = scalar_rows(cx, &world, |k, _| k.starts_with("Stat"));
        rows.extend(scalar_rows(cx, &body, |k, _| k.starts_with("Stat")));
        for r in &mut rows {
            r.0 = r.0.trim_start_matches("Stat ").to_string();
        }
        value_rows(ui, cx, "gh_stats", &rows);
    });
    ui.weak("Items, constructions and quests are in the Values tab.");
}

// ------------------------------------------------------------------ RE Engine game slots

/// An RE Engine save with a player inventory or character health (Resident Evil
/// Requiem and other recent RE Engine games).
pub fn is_re_slot(root: &Value) -> bool {
    !objects_with(root, "_InventoryData", 1).is_empty() || !objects_with(root, "_CurrentMaxHP", 1).is_empty()
}

pub fn re_slot(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(160, 40, 40);
    let info = objects_with(cx.root, "_SaveCount", 1).pop();
    let mut sub = vec![];
    if let Some(i) = &info {
        if let Some(n) = cx.int(&format!("{i}/_SaveCount")) {
            sub.push(format!("saved {n} times"));
        }
        if cx.flag(&format!("{i}/_IsNewGamePlus")) {
            sub.push("New Game+".into());
        }
    }
    hero(ui, accent, "Game slot", &sub.join("  \u{00B7}  "), |ui| {
        if let Some(i) = &info {
            cx.tile(ui, "Save count", &format!("{i}/_SaveCount"), accent);
        }
    });
    // Characters with health: the first is normally the player.
    let hp: Vec<String> = objects_with(cx.root, "_CurrentMaxHP", 12);
    if !hp.is_empty() {
        panel(ui, "Health", |ui| {
            egui::Grid::new("re_hp").num_columns(3).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                ui.strong("Character");
                ui.strong("Health");
                ui.strong("Most health");
                ui.end_row();
                for (n, p) in hp.iter().enumerate() {
                    ui.label(if n == 0 { "Player".to_string() } else { format!("Character {}", n + 1) });
                    cx.cell(ui, &format!("{p}/_CurrentHP"), 100.0);
                    cx.cell(ui, &format!("{p}/_CurrentMaxHP"), 100.0);
                    ui.end_row();
                }
            });
            ui.weak("Enemies kept in the save show here too; the player is usually first.");
        });
    }
    for inv_base in objects_with(cx.root, "_InventoryData", 4) {
        let p = format!("{inv_base}/_InventoryData");
        let boards = cx.get(&p).and_then(Value::as_array).map_or(0, Vec::len);
        if boards == 0 {
            continue;
        }
        let names: Vec<String> = (0..boards)
            .map(|i| cx.text(&format!("{p}/{i}/_BoardTypeName")).or_else(|| cx.text(&format!("{p}/{i}/_UserName"))).map(|s| words(&s)).unwrap_or_else(|| format!("Board {}", i + 1)))
            .collect();
        panel(ui, "Inventory", |ui| {
            let t = tabs(ui, &format!("re_inv{inv_base}"), &names);
            let b = format!("{p}/{t}");
            ui.horizontal(|ui| {
                if cx.get(&format!("{b}/_Moneys")).is_some_and(|m| m.as_i64() != Some(-1)) {
                    cx.label(ui, "Money", &format!("{b}/_Moneys"));
                    cx.editor(ui, &format!("{b}/_Moneys"), 110.0);
                }
            });
            for list in ["_PanelItems", "_Items", "_ArchiveItems"] {
                let lp = format!("{b}/{list}");
                let n = cx.get(&lp).and_then(Value::as_array).map_or(0, Vec::len);
                if n == 0 {
                    continue;
                }
                ui.label(RichText::new(match list {
                    "_ArchiveItems" => "Files and key items",
                    _ => "Items",
                }).strong());
                egui::Grid::new(("re_items", &lp)).num_columns(3).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                    ui.strong(tr("Item"));
                    ui.strong(tr("Count"));
                    ui.strong("Held inside");
                    ui.end_row();
                    for i in 0..n {
                        let ip = format!("{lp}/{i}");
                        let id = cx.int(&format!("{ip}/_ItemIDHash")).unwrap_or(0);
                        if id == 0 {
                            continue;
                        }
                        ui.label(format!("#{id:08x}"));
                        cx.cell(ui, &format!("{ip}/_Stock"), 80.0);
                        // Ammunition in a weapon, or a pouch's contents.
                        let inner = format!("{ip}/_StorageItems/0/_AmountSaveData/_Stock");
                        if cx.get(&inner).is_some() {
                            cx.cell(ui, &inner, 80.0);
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
            }
        });
    }
    ui.weak("Items are listed by their ID because the game's item names aren't in the save. Everything else is in the Values tab.");
}

// ------------------------------------------------------------------ Hogwarts Legacy

pub fn is_hogwarts(root: &Value) -> bool {
    root.pointer("/CharacterSaveGameInfo/CharacterHouse").is_some()
}

pub fn hogwarts(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = match cx.text("/CharacterSaveGameInfo/CharacterHouse").as_deref() {
        Some("Gryffindor") => Color32::from_rgb(170, 40, 40),
        Some("Slytherin") => Color32::from_rgb(40, 120, 70),
        Some("Ravenclaw") => Color32::from_rgb(50, 80, 170),
        Some("Hufflepuff") => Color32::from_rgb(210, 170, 40),
        _ => Color32::from_rgb(120, 90, 160),
    };
    let c = "/CharacterSaveGameInfo";
    let name = cx.text(&format!("{c}/CharacterName")).unwrap_or_else(|| "Student".into());
    let mut sub = vec![];
    if let Some(h) = cx.text(&format!("{c}/CharacterHouse")) {
        sub.push(h);
    }
    if let Some(m) = cx.text("/DirectoryEntry/CurrentMap") {
        sub.push(words(m.trim_start_matches("RegionName")));
    }
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Level (load screen)", &format!("{c}/CharacterLevel"), accent);
        cx.tile(ui, "Difficulty", &format!("{c}/CharacterGameDifficulty"), accent);
    });
    if let Some(q) = cx.text("/DirectoryEntry/CurrentAction") {
        ui.label(RichText::new(format!("Current quest: {}", words(q.trim_end_matches("_QuestTitle")))).weak());
    }
    panel(ui, "Character", |ui| {
        value_rows(ui, cx, "hl_c", &[
            ("Name".into(), format!("{c}/CharacterName")),
            ("House".into(), format!("{c}/CharacterHouse")),
            ("Voice".into(), format!("{c}/CharacterVoice")),
            ("Pronouns".into(), format!("{c}/CharacterPronoun")),
            ("Intro finished".into(), "/bCompletedIntro".into()),
        ]);
        ui.weak(
            "Level and difficulty here are what the load screen shows. Gold, experience, talent points and the \
             inventory are kept in the game's compressed database inside this save, which can't be edited yet.",
        );
    });
}

// ------------------------------------------------------------------ Ren'Py

pub fn is_renpy(kind: &str, root: &Value) -> bool {
    kind == "Ren'Py save" && root.get("Variables").is_some_and(Value::is_object)
}

pub fn renpy(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 90, 140);
    let version = cx.text("/Ren'Py internals/_version").unwrap_or_default();
    // Every plain value in the story variables, including those inside objects (`mc.money`), as (label, pointer, value).
    fn collect(label: String, ptr: String, v: &Value, depth: usize, out: &mut Vec<(String, String, Value)>) {
        if out.len() > 5000 {
            return;
        }
        match v {
            Value::Object(m) if depth < 3 => {
                for (k, c) in m {
                    if !k.starts_with("__") {
                        collect(format!("{label}.{k}"), join(&ptr, k), c, depth + 1, out);
                    }
                }
            }
            Value::Number(_) | Value::Bool(_) | Value::String(_) => out.push((label, ptr, v.clone())),
            _ => {}
        }
    }
    let mut entries: Vec<(String, String, Value)> = vec![];
    if let Some(Value::Object(m)) = cx.get("/Variables") {
        for (k, v) in m {
            collect(k.clone(), join("/Variables", k), v, 0, &mut entries);
        }
    }
    let vars: Vec<(String, Value)> = entries.iter().map(|(l, _, v)| (l.clone(), v.clone())).collect();
    let ptr_of: std::collections::HashMap<String, String> = entries.iter().map(|(l, p, _)| (l.clone(), p.clone())).collect();
    let numbers: Vec<&String> = vars.iter().filter(|(_, v)| v.is_number()).map(|(k, _)| k).collect();
    let flags: Vec<&String> = vars.iter().filter(|(_, v)| v.is_boolean()).map(|(k, _)| k).collect();
    let texts: Vec<&String> = vars.iter().filter(|(_, v)| v.as_str().is_some_and(|s| !s.starts_with('<'))).map(|(k, _)| k).collect();
    let sub = format!("{} numbers  \u{00B7}  {} flags{}", numbers.len(), flags.len(), if version.is_empty() { String::new() } else { format!("  \u{00B7}  saved with game version {version}") });
    hero(ui, accent, "Story variables", &sub, |ui| {
        // Money and the like, if the game calls them something common.
        for (k, _) in &vars {
            let l = k.to_lowercase();
            if ["money", "gold", "cash", "coins", "credits", "affection", "love", "points"].iter().any(|w| l == *w || l.ends_with(&format!("_{w}")) || l.ends_with(&format!(".{w}"))) {
                cx.tile(ui, &words(k), &ptr_of[k], accent);
            }
        }
    });
    story_guard(ui, |ui| {
        let q = filter_box(ui, "renpy", "e.g. money, love, day");
        let keep = |k: &str| q.is_empty() || k.to_lowercase().contains(&q);
        panel(ui, "Numbers", |ui| {
            let rows: Vec<(String, String)> = numbers.iter().filter(|k| keep(k)).map(|k| ((*k).clone(), ptr_of[*k].clone())).collect();
            if rows.is_empty() {
                ui.weak(tr("None match."));
            }
            egui::ScrollArea::vertical().id_salt("renpy_num").max_height(360.0).show_locked(ui, |ui| value_rows(ui, cx, "renpy_n", &rows));
        });
        panel(ui, "Flags", |ui| {
            let shown: Vec<&&String> = flags.iter().filter(|k| keep(k)).collect();
            if shown.is_empty() {
                ui.weak(tr("None match."));
            }
            egui::ScrollArea::vertical().id_salt("renpy_flags").max_height(260.0).show_locked(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for k in shown {
                        cx.chip(ui, k, &ptr_of[*k], accent);
                    }
                });
            });
        });
        panel(ui, "Text", |ui| {
            let rows: Vec<(String, String)> = texts.iter().filter(|k| keep(k)).map(|k| ((*k).clone(), ptr_of[*k].clone())).collect();
            if rows.is_empty() {
                ui.weak(tr("None match."));
            }
            egui::ScrollArea::vertical().id_salt("renpy_txt").max_height(220.0).show_locked(ui, |ui| value_rows(ui, cx, "renpy_t", &rows));
        });
    });
    ui.weak("These are the game's own variable names; values inside characters or other objects are shown as name.value. Each save keeps its own copy; the Values tab also shows lists.");
}

// ------------------------------------------------------------------ EU4 / CK2

const EU4_TILES: [(&str, &str); 10] = [
    ("Treasury", "treasury"),
    ("Manpower", "manpower"),
    ("Sailors", "sailors"),
    ("Prestige", "prestige"),
    ("Stability", "stability"),
    ("Legitimacy", "legitimacy"),
    ("Army tradition", "army_tradition"),
    ("Navy tradition", "navy_tradition"),
    ("Corruption", "corruption"),
    ("Inflation", "inflation"),
];

pub fn eu4(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(150, 110, 60);
    let tag = cx.text("/player").unwrap_or_default();
    let c = join("/countries", &tag);
    let date = cx.text("/date").unwrap_or_default();
    let name = cx.text(&format!("{c}/name")).unwrap_or_else(|| tag.clone());
    hero(ui, accent, &name, &format!("{date}  \u{00B7}  {tag}  \u{00B7}  Europa Universalis IV"), |ui| {
        for (label, k) in EU4_TILES {
            cx.tile(ui, label, &join(&c, k), accent);
        }
    });
    // Monarch power: "powers = { adm dip mil }".
    let pw = format!("{c}/powers");
    if cx.get(&pw).and_then(Value::as_array).is_some_and(|a| a.len() >= 3) {
        panel(ui, "Monarch power", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, n) in ["Administrative", "Diplomatic", "Military"].iter().enumerate() {
                    cx.label(ui, n, &format!("{pw}/{i}"));
                    cx.editor(ui, &format!("{pw}/{i}"), 80.0);
                    ui.add_space(10.0);
                }
            });
        });
    }
    panel(ui, "Country", |ui| {
        if cx.get(&c).is_none() {
            ui.weak("The player's country isn't in this save.");
            return;
        }
        let q = filter_box(ui, "eu4", "e.g. mercantilism");
        let rows = scalar_rows(cx, &c, |k, v| v.is_number() && (q.is_empty() || k.contains(&q)));
        egui::ScrollArea::vertical().id_salt("eu4_c").max_height(360.0).show_locked(ui, |ui| value_rows(ui, cx, "eu4_rows", &rows));
    });
    ui.weak("Stability runs \u{2212}3 to 3; prestige and legitimacy up to 100. Ironman saves are edited in place and stay ironman; the game may stop awarding achievements for an edited campaign.");
}

pub fn ck2(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(140, 50, 60);
    let id = cx.int("/player/id").unwrap_or(0);
    let ch = format!("/character/{id}");
    let date = cx.text("/date").unwrap_or_default();
    let name = cx.text(&format!("{ch}/bn")).or_else(|| cx.text(&format!("{ch}/birth_name"))).unwrap_or_else(|| format!("Character {id}"));
    hero(ui, accent, &name, &format!("{date}  \u{00B7}  Crusader Kings II"), |ui| {
        for (label, ks) in [("Wealth", ["wealth", "wea"]), ("Prestige", ["prestige", "prs"]), ("Piety", ["piety", "piety"])] {
            if let Some(k) = ks.iter().find(|k| cx.get(&join(&ch, k)).is_some()) {
                cx.tile(ui, label, &join(&ch, k), accent);
            }
        }
    });
    panel(ui, "Ruler", |ui| {
        if cx.get(&ch).is_none() {
            ui.weak("The player's character isn't in this save.");
            return;
        }
        let rows = scalar_rows(cx, &ch, |_, v| v.is_number() || v.is_string());
        value_rows(ui, cx, "ck2_rows", &rows);
        if let Some(Value::Array(a)) = cx.get(&format!("{ch}/att")) {
            let n = a.len().min(5);
            ui.add_space(6.0);
            ui.label(RichText::new("Attributes").strong());
            ui.horizontal_wrapped(|ui| {
                for (i, label) in ["Diplomacy", "Martial", "Stewardship", "Intrigue", "Learning"].iter().take(n).enumerate() {
                    cx.label(ui, label, &format!("{ch}/att/{i}"));
                    cx.editor(ui, &format!("{ch}/att/{i}"), 60.0);
                    ui.add_space(8.0);
                }
            });
        }
    });
}

// ------------------------------------------------------------------ Mass Effect 2 / 3

pub fn mass_effect(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 60, 50);
    let first = cx.text("/Shepard/First name").filter(|s| !s.is_empty()).unwrap_or_else(|| "Shepard".into());
    let class = cx.text("/Shepard/Class").unwrap_or_default();
    let game = cx.text("/Save/Game").unwrap_or_default();
    let played = cx.num("/Save/Seconds played").unwrap_or(0.0);
    let mut sub = vec![];
    if !class.is_empty() {
        sub.push(words(class.trim_end_matches("NonCombat")));
    }
    if let Some(o) = cx.text("/Shepard/Origin") {
        sub.push(o);
    }
    if let Some(r) = cx.text("/Shepard/Reputation") {
        sub.push(r);
    }
    sub.push(format!("{:.1} hours", played / 3600.0));
    sub.push(game);
    hero(ui, accent, &format!("{first} Shepard"), &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Level", "/Shepard/Level", accent);
        cx.tile(ui, "Experience", "/Shepard/Experience", accent);
        cx.tile(ui, "Talent points", "/Shepard/Talent points", accent);
        cx.tile(ui, "Difficulty", "/Save/Difficulty", accent);
        cx.tile(ui, "Paragon", "/Shepard/Paragon", accent);
        cx.tile(ui, "Renegade", "/Shepard/Renegade", accent);
        cx.tile(ui, "Reputation points", "/Shepard/Reputation points", accent);
    });
    panel(ui, "Resources", |ui| {
        let rows: Vec<(String, String)> = match cx.get("/Resources") {
            Some(Value::Object(m)) => m.keys().map(|k| (k.clone(), join("/Resources", k))).collect(),
            _ => vec![],
        };
        value_rows(ui, cx, "me_res", &rows);
        if cx.get("/Resources/Omni-gel").is_none() {
            ui.weak("Element zero, iridium, palladium and platinum are Mass Effect 2's minerals; Mass Effect 3 keeps them at 0.");
        }
    });
    power_list(ui, cx, "/Powers", "me_pow", accent);
    let weapons: Vec<String> = match cx.get("/Weapons") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !weapons.is_empty() {
        panel(ui, "Weapons", |ui| {
            let rows: Vec<(String, String)> = weapons.iter().map(|w| (words(w), format!("{}/Spare ammo", join("/Weapons", w)))).collect();
            value_rows(ui, cx, "me_w", &rows);
            ui.weak("Spare ammo for each weapon Shepard carries.");
        });
    }
    let parts: Vec<String> = match cx.get("/Items") {
        Some(Value::Object(m)) => m.iter().filter(|(_, v)| v.as_object().is_some_and(|o| !o.is_empty())).map(|(k, _)| k.clone()).collect(),
        _ => vec![],
    };
    if !parts.is_empty() {
        panel(ui, "Inventory", |ui| {
            let t = tabs(ui, "me_items", &parts);
            let base = join("/Items", &parts[t]);
            let q = filter_box(ui, "me_items_q", "item name");
            let rows: Vec<(String, String)> = match cx.get(&base) {
                Some(Value::Object(m)) => m.keys().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).map(|k| (k.clone(), join(&base, k))).collect(),
                _ => vec![],
            };
            egui::ScrollArea::vertical().id_salt("me_items_s").max_height(380.0).show_locked(ui, |ui| value_rows(ui, cx, &format!("me_it{t}"), &rows));
            ui.weak("Each item's level (I\u{2013}X). Items without a name are upgrades and ammo, shown by the game's item number.");
        });
    }
    let squad: Vec<String> = match cx.get("/Squad") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !squad.is_empty() {
        panel(ui, "Squad", |ui| {
            let t = tabs(ui, "me_squad", &squad);
            let base = join("/Squad", &squad[t]);
            ui.horizontal_wrapped(|ui| {
                for (label, k) in [("Level", "Level"), ("Talent points", "Talent points"), ("Grenades", "Grenades")] {
                    let p = join(&base, k);
                    if cx.get(&p).is_some() {
                        cx.label(ui, label, &p);
                        cx.editor(ui, &p, 70.0);
                        ui.add_space(10.0);
                    }
                }
            });
            power_list(ui, cx, &format!("{base}/Powers"), &format!("me_sq{t}"), accent);
            power_list(ui, cx, &format!("{base}/Talents"), &format!("me_sqt{t}"), accent);
        });
    }
    ui.weak("Power ranks run 0\u{2013}4 in Mass Effect 2 and 0\u{2013}6 in Mass Effect 3 and up to 12 in Mass Effect 1; an evolved power shows up as its own entry.");
    story(ui, cx, accent);
}

/// Mass Effect 2 and 3 story flags (loyalty, romance, who lived, imported choices) and the
/// numbers beside them, by section.
pub(super) fn story(ui: &mut Ui, cx: &mut GameCtx, accent: Color32) {
    let flags = keys_of(cx, "/Story flags");
    let numbers = keys_of(cx, "/Story numbers");
    if flags.is_empty() && numbers.is_empty() {
        return;
    }
    panel(ui, "Story", |ui| story_guard(ui, |ui| {
        let q = filter_box(ui, "me_story", "loyal, romance, dead\u{2026}");
        let mut sections = flags.clone();
        for n in &numbers {
            if !sections.contains(n) {
                sections.push(n.clone());
            }
        }
        for sec in &sections {
            let fb = join("/Story flags", sec);
            let nb = join("/Story numbers", sec);
            let hit = |k: &String| q.is_empty() || k.to_lowercase().contains(&q) || sec.to_lowercase().contains(&q);
            let f: Vec<String> = keys_of(cx, &fb).into_iter().filter(hit).collect();
            let n: Vec<String> = keys_of(cx, &nb).into_iter().filter(hit).collect();
            if f.is_empty() && n.is_empty() {
                continue;
            }
            let on = f.iter().filter(|k| cx.flag(&join(&fb, k))).count();
            let title = if f.is_empty() { sec.clone() } else { format!("{sec}  ({on} of {} on)", f.len()) };
            egui::CollapsingHeader::new(title).id_salt(("me_story", sec)).open(if q.is_empty() { None } else { Some(true) }).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for k in &f {
                        cx.chip(ui, k, &join(&fb, k), accent);
                    }
                });
                if !n.is_empty() {
                    let rows: Vec<(String, String)> = n.iter().map(|k| (k.clone(), join(&nb, k))).collect();
                    value_rows(ui, cx, &format!("me_story_{sec}"), &rows);
                }
            });
        }
        ui.weak(
            "Click a flag to turn it on or off. These are the game's own story records: changing one doesn't replay the scene behind it, \
             and flags that contradict each other (alive and dead, two romances) can confuse later scenes, so change few at a time and keep the backup.",
        );
    }));
}

fn power_list(ui: &mut Ui, cx: &mut GameCtx, base: &str, id: &str, accent: Color32) {
    let names: Vec<String> = match cx.get(base) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => return,
    };
    if names.is_empty() {
        return;
    }
    let game = cx.text("/Save/Game").unwrap_or_default();
    let max = if game.contains("ME1") || game.contains("2008") { 12.0 } else if game.contains('3') { 6.0 } else { 4.0 };
    ui.push_id(id, |ui| {
        panel(ui, "Powers", |ui| {
            columns(ui, 320.0, names.len(), |ui, i| {
                cx.attribute(ui, &words(&names[i]), &join(base, &names[i]), max, accent);
            });
        });
    });
}

// ------------------------------------------------------------------ XCOM: Enemy Unknown / Within

pub fn xcom(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(60, 120, 200);
    let hq = objects_with(cx.root, "m_iCash", 1).pop();
    let mut soldiers: Vec<(String, &'static str, &'static str)> = vec![];
    for (s, c) in [("m_kSoldier", "m_kChar"), ("kSoldier", "kChar")] {
        for p in objects_with(cx.root, s, 400) {
            soldiers.push((p, s, c));
        }
    }
    let n = soldiers.len();
    hero(ui, accent, "XCOM", &format!("{n} soldier record{}", if n == 1 { "" } else { "s" }), |ui| {
        if let Some(h) = &hq {
            cx.tile(ui, "Money (\u{00A7})", &format!("{h}/m_iCash"), accent);
        }
    });
    if !soldiers.is_empty() {
        panel(ui, "Soldiers", |ui| {
            let q = filter_box(ui, "xcom_sold", "name or class");
            let stat = |base: &str, c: &str, i: usize| format!("{base}/{c}/aStats[{i}]");
            let area = egui::ScrollArea::both().id_salt("xcom_s").max_height(420.0).show_locked(ui, |ui| {
                egui::Grid::new("xcom_grid").num_columns(9).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    crate::widgets::header_row(ui, "xcom_s", ["Soldier", "Class", "Rank", "XP", "Kills", "HP", "Aim", "Mobility", "Will"]);
                    // The living first, then the fallen.
                    let mut order: Vec<(bool, &(String, &str, &str))> =
                        soldiers.iter().map(|x| (cx.text(&format!("{}/m_strKIAReport", x.0)).is_some_and(|r| !r.is_empty()), x)).collect();
                    order.sort_by_key(|(kia, _)| *kia);
                    for (kia, (base, s, c)) in order {
                        let sp = format!("{base}/{s}");
                        let first = cx.text(&format!("{sp}/strFirstName")).unwrap_or_default();
                        let last = cx.text(&format!("{sp}/strLastName")).unwrap_or_default();
                        let nick = cx.text(&format!("{sp}/strNickName")).filter(|n| !n.is_empty()).map(|n| format!(" \u{201C}{n}\u{201D}")).unwrap_or_default();
                        let class = cx.text(&format!("{sp}/kClass/strName")).unwrap_or_default();
                        let name = format!("{first}{nick} {last}");
                        if !q.is_empty() && !name.to_lowercase().contains(&q) && !class.to_lowercase().contains(&q) {
                            continue;
                        }
                        ui.label(if kia { RichText::new(format!("{name} (KIA)")).weak() } else { RichText::new(name) });
                        ui.label(class);
                        cx.compact(ui, &format!("{sp}/iRank"), 36.0);
                        cx.compact(ui, &format!("{sp}/iXP"), 64.0);
                        cx.compact(ui, &format!("{sp}/iNumKills"), 52.0);
                        for i in [0, 1, 3, 7] {
                            if cx.get(&stat(base, c, i)).is_some() {
                                cx.compact(ui, &stat(base, c, i), 56.0);
                            } else {
                                ui.label("");
                            }
                        }
                        ui.end_row();
                    }
                });
            });
            crate::widgets::sticky_headers(ui, "xcom_s", area.inner_rect);
            ui.weak("Rank 0 is Squaddie\u{2026} 7 is Colonel. Stats are the soldier's own, before armour and items.");
        });
    }
    let store = objects_with(cx.root, "m_arrItems", 1).pop();
    if let Some(sp) = store {
        let base = format!("{sp}/m_arrItems");
        let rows: Vec<(String, String)> = match cx.get(&base) {
            Some(Value::Object(m)) => m.iter().filter(|(_, v)| v.as_i64().is_some_and(|n| n > 0)).map(|(k, _)| (k.clone(), join(&base, k))).collect(),
            _ => vec![],
        };
        panel(ui, "Storage", |ui| {
            value_rows(ui, cx, "xcom_store", &rows);
            ui.weak("Items HQ holds, by the game's item number (only ones you have are listed). Elerium, alloys and Meld are among them \u{2014} change one and check it in the game.");
        });
    }
}

// ------------------------------------------------------------------ XCOM 2 / War of the Chosen

const XCOM2_RANKS: [&str; 9] = ["Rookie", "Squaddie", "Corporal", "Sergeant", "Lieutenant", "Captain", "Major", "Colonel", "Brigadier"];

pub fn xcom2(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(70, 150, 210);
    let soldiers: Vec<String> = match cx.get("/Soldiers") {
        Some(Value::Object(m)) => m.keys().map(|k| join("/Soldiers", k)).collect(),
        _ => vec![],
    };
    let n = soldiers.len();
    hero(ui, accent, "XCOM 2", &format!("{n} soldier{}", if n == 1 { "" } else { "s" }), |ui| {
        for (_, label) in crate::fmt::xcom2::RESOURCES {
            cx.tile(ui, label, &join("/Resources", label), accent);
        }
    });
    if !soldiers.is_empty() {
        let stats = ["HP", "Aim", "Mobility", "Will", "Hacking", "Defense", "Dodge", "Psi", "Armor", "Shield", "Sight"];
        let shown: Vec<&str> = stats.iter().copied().filter(|s| soldiers.iter().any(|b| cx.get(&format!("{b}/Stats/{s}")).is_some())).collect();
        panel(ui, "Soldiers", |ui| {
            let q = filter_box(ui, "xcom2_sold", "name or class");
            let area = egui::ScrollArea::both().id_salt("xcom2_s").max_height(460.0).show_locked(ui, |ui| {
                egui::Grid::new("xcom2_grid").num_columns(6 + shown.len()).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    crate::widgets::header_row(ui, "xcom2_s", ["Soldier", "Class", "Rank", "Kills", "Missions", "Ability points"].into_iter().chain(shown.iter().map(|s| s.as_ref())));
                    for b in &soldiers {
                        let first = cx.text(&format!("{b}/First name")).unwrap_or_default();
                        let last = cx.text(&format!("{b}/Last name")).unwrap_or_default();
                        let nick = cx.text(&format!("{b}/Nickname")).filter(|n| !n.is_empty()).map(|n| format!(" \u{201C}{n}\u{201D}")).unwrap_or_default();
                        let class = cx.text(&format!("{b}/Class")).unwrap_or_default();
                        let name = format!("{first}{nick} {last}");
                        if !q.is_empty() && !name.to_lowercase().contains(&q) && !class.to_lowercase().contains(&q) {
                            continue;
                        }
                        ui.label(name);
                        ui.label(words(&class));
                        let rank = format!("{b}/Rank");
                        ui.horizontal(|ui| {
                            cx.compact(ui, &rank, 36.0);
                            if let Some(r) = cx.int(&rank).and_then(|r| XCOM2_RANKS.get(r as usize)) {
                                ui.weak(*r);
                            }
                        });
                        for f in ["Kills", "Missions", "Ability points"] {
                            let p = format!("{b}/{f}");
                            if cx.get(&p).is_some() {
                                cx.compact(ui, &p, 44.0);
                            } else {
                                ui.label("");
                            }
                        }
                        for s in &shown {
                            let p = format!("{b}/Stats/{s}");
                            if cx.get(&p).is_some() {
                                cx.compact(ui, &p, 44.0);
                            } else {
                                ui.label("");
                            }
                        }
                        ui.end_row();
                    }
                });
            });
            crate::widgets::sticky_headers(ui, "xcom2_s", area.inner_rect);
            ui.weak(
                "Rank 0 is Rookie \u{2026} 7 is Colonel (8 Brigadier with some classes). A stat sets the soldier's current, maximum and \
                 base value together, so a wounded soldier is healed to it. A blank cell is a value the game left at its default.",
            );
        });
    }
    let rows: Vec<(String, String)> = match cx.get("/Storage") {
        Some(Value::Object(m)) => m.iter().filter(|(_, v)| v.as_i64().is_some_and(|n| n > 0)).map(|(k, _)| (words(k), join("/Storage", k))).collect(),
        _ => vec![],
    };
    if !rows.is_empty() {
        panel(ui, "Storage", |ui| {
            value_rows(ui, cx, "xcom2_store", &rows);
            ui.weak("Items HQ holds, by the game's template name.");
        });
    }
}

// ------------------------------------------------------------------ Civilization VI

pub fn civ6(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 160, 60);
    let turn = cx.int("/Game/Turn").map(|t| format!("Turn {t}")).unwrap_or_default();
    let mut sub = vec![turn];
    for k in ["Speed", "Map size", "Map"] {
        if let Some(v) = cx.text(&format!("/Game/{k}")) {
            sub.push(format!("{k}: {v}"));
        }
    }
    let civs: Vec<String> = match cx.get("/Civilizations") {
        Some(Value::Object(m)) => m.keys().map(|k| join("/Civilizations", k)).collect(),
        _ => vec![],
    };
    hero(ui, accent, "Civilization VI", &sub.join("  \u{00B7}  "), |ui| {
        let humans = civs.iter().filter(|c| cx.text(&format!("{c}/Played by")).as_deref() == Some("Human")).count();
        fact(ui, "Civilizations", &civs.len().to_string(), accent);
        fact(ui, "Human players", &humans.to_string(), accent);
    });
    panel(ui, "Civilizations", |ui| {
        egui::Grid::new("civ6_grid").num_columns(6).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
            for h in ["Civilization", "Leader", "Player", "Played by", "Their turn", "Alive"] {
                ui.strong(h);
            }
            ui.end_row();
            for c in &civs {
                ui.label(cx.text(&format!("{c}/Civilization")).unwrap_or_default());
                ui.label(cx.text(&format!("{c}/Leader")).unwrap_or_default());
                ui.label(cx.text(&format!("{c}/Player")).unwrap_or_default());
                cx.compact(ui, &format!("{c}/Played by"), 90.0);
                let t = format!("{c}/Current turn");
                if cx.get(&t).is_some() {
                    cx.compact(ui, &t, 40.0);
                } else {
                    ui.label("");
                }
                let alive = cx.get(&format!("{c}/Alive")).and_then(Value::as_bool);
                ui.label(match alive {
                    Some(true) => "Yes",
                    Some(false) => "Defeated",
                    None => "",
                });
                ui.end_row();
            }
        });
        ui.weak(
            "Hand a civilisation to a human or the AI, or change whose turn it is \u{2014} for hotseat or play-by-cloud games when someone \
             drops out. Gold, faith, cities and units are in the packed part of the save, which isn't mapped yet.",
        );
    });
    let mods: Vec<String> = match cx.get("/Mods") {
        Some(Value::Object(m)) => m.values().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => vec![],
    };
    if !mods.is_empty() {
        panel(ui, "Mods and DLC", |ui| {
            for m in mods {
                ui.label(m);
            }
        });
    }
}

// ------------------------------------------------------------------ Diablo II

pub fn diablo2(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(170, 40, 30);
    let name = cx.text("/Character/Name").filter(|n| !n.is_empty()).unwrap_or_else(|| "Hero".into());
    let class = cx.text("/Character/Class").unwrap_or_default();
    let mut sub = vec![class];
    if cx.flag("/Character/Hardcore") {
        sub.push("Hardcore".into());
    }
    sub.push(if cx.flag("/Character/Expansion") { "Lord of Destruction".into() } else { "Classic".into() });
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        for (label, p) in [("Level", "/Progress/Level"), ("Experience", "/Progress/Experience"), ("Gold", "/Progress/Gold"), ("Stash gold", "/Progress/Stash gold")] {
            cx.tile(ui, label, p, accent);
        }
    });
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Attributes", |ui| {
                for k in ["Strength", "Dexterity", "Vitality", "Energy"] {
                    cx.attribute(ui, k, &join("/Attributes", k), 1023.0, accent);
                }
                ui.add_space(6.0);
                cx.grid_row(ui, "Stat points", "/Attributes/Stat points", 80.0);
                cx.grid_row(ui, "Skill points", "/Attributes/Skill points", 80.0);
            });
        } else {
            panel(ui, "Life and mana", |ui| {
                egui::Grid::new("d2_life").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("");
                    ui.weak("Current");
                    ui.label("");
                    ui.weak(tr("Base max")).on_hover_text("The maximum before items and bonuses; the game adds those on top");
                    ui.end_row();
                    for (a, b) in [("Life", "Max life"), ("Mana", "Max mana"), ("Stamina", "Max stamina")] {
                        ui.label(a);
                        cx.compact(ui, &join("/Life and mana", a), 84.0);
                        ui.label("/");
                        cx.compact(ui, &join("/Life and mana", b), 84.0);
                        ui.end_row();
                    }
                });
                ui.weak("Max life and mana here are the base values, before items.");
            });
        }
    });
    let skills: Vec<String> = match cx.get("/Skills") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !skills.is_empty() {
        panel(ui, "Skills", |ui| {
            columns(ui, 300.0, skills.len(), |ui, i| {
                cx.attribute(ui, &skills[i], &join("/Skills", &skills[i]), 20.0, accent);
            });
            ui.weak("Hard points put into each skill (20 is the most the game allows from points; items add more on top).");
        });
    }
    let ways: Vec<String> = ["Normal", "Nightmare", "Hell"].iter().map(|d| format!("/Waypoints: {d}")).filter(|p| cx.get(p).is_some()).collect();
    if !ways.is_empty() {
        panel(ui, "Waypoints", |ui| {
            for base in &ways {
                let keys = keys_of(cx, base);
                let on = keys.iter().filter(|k| cx.flag(&join(base, k))).count();
                egui::CollapsingHeader::new(format!("{}  ({on} of {})", base.trim_start_matches("/Waypoints: "), keys.len())).id_salt(("d2_wp", base)).show(ui, |ui| {
                    if super::quick_button(ui, "Open every waypoint", "") {
                        for k in &keys {
                            cx.set(&join(base, k), Value::from(1));
                        }
                    }
                    ui.horizontal_wrapped(|ui| {
                        for k in &keys {
                            cx.chip(ui, k, &join(base, k), accent);
                        }
                    });
                });
            }
            ui.weak("A waypoint you can use in that difficulty. The character still has to be able to reach the act.");
        });
    }
    ui.weak("Changing the level doesn't change experience: set both. Gold on the character can be at most 10,000 per level.");
}

// ------------------------------------------------------------------ Final Fantasy VII (PC)

pub fn ff7(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(60, 110, 180);
    let slots: Vec<String> = match cx.root {
        Value::Object(m) => m.keys().filter(|k| k.starts_with("Slot ")).cloned().collect(),
        _ => vec![],
    };
    if slots.is_empty() {
        return;
    }
    let names: Vec<String> = slots
        .iter()
        .map(|s| match cx.text(&format!("{}/Location", join("", s))) {
            Some(l) if !l.is_empty() => format!("{s} \u{2014} {l}"),
            _ => s.clone(),
        })
        .collect();
    let t = tabs(ui, "ff7_slot", &names);
    let base = join("", &slots[t]);
    let secs = cx.int(&format!("{base}/Play time (seconds)")).unwrap_or(0);
    let loc = cx.text(&format!("{base}/Location")).unwrap_or_default();
    hero(ui, accent, "Final Fantasy VII", &format!("{loc}  \u{00B7}  {}:{:02} played", secs / 3600, secs / 60 % 60), |ui| {
        cx.tile(ui, "Gil", &format!("{base}/Gil"), accent);
        cx.tile(ui, "Play time (seconds)", &format!("{base}/Play time (seconds)"), accent);
    });
    let party = format!("{base}/Party");
    let members: Vec<String> = match cx.get(&party) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    panel(ui, "Characters", |ui| {
        let c = tabs(ui, &format!("ff7_char{t}"), &members);
        let Some(who) = members.get(c) else { return };
        let p = join(&party, who);
        let at = |k: &str| join(&p, k);
        egui::Grid::new(("ff7_top", t, c)).num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
            for k in ["Level", "Experience", "Kills", "Limit level", "Limit gauge"] {
                cx.grid_row(ui, k, &at(k), if k == "Experience" { 100.0 } else { 56.0 });
            }
        });
        ui.add_space(6.0);
        egui::Grid::new(("ff7_hp", t, c)).num_columns(4).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.label("");
            ui.strong("Now");
            ui.strong("Max");
            ui.strong(tr("Base max"));
            ui.end_row();
            for (n, cur, max, base) in [("HP", "HP", "Max HP", "Base max HP"), ("MP", "MP", "Max MP", "Base max MP")] {
                ui.label(n);
                cx.cell(ui, &at(cur), 70.0);
                cx.cell(ui, &at(max), 70.0);
                if cx.get(&at(base)).is_some() {
                    cx.cell(ui, &at(base), 70.0);
                } else {
                    ui.label("");
                }
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Stats").strong());
        for k in ["Strength", "Vitality", "Magic", "Spirit", "Dexterity", "Luck"] {
            cx.attribute(ui, k, &at(k), 255.0, accent);
        }
        ui.weak("Stats go up to 255. The game works out max HP/MP again from the base max and equipment, so raise the base max to make a higher maximum stick.");
    });
}

// ------------------------------------------------------------------ FF7 Remake / Rebirth

pub fn ff7_remake(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(70, 150, 120);
    let game = cx.text("/Game").unwrap_or_else(|| "Final Fantasy VII Remake".into());
    let members: Vec<String> = match cx.get("/Party") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    hero(ui, accent, &game, &format!("{} party members in this slot", members.len()), |ui| {
        for k in ["Gil", "Moogle Medals", "Gold Saucer Points"] {
            cx.tile(ui, k, &format!("/{k}"), accent);
        }
    });
    panel(ui, "Party", |ui| {
        egui::ScrollArea::horizontal().id_salt("ff7r_scroll").show_locked(ui, |ui| {
        egui::Grid::new("ff7r_party").num_columns(7).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
            for h in ["", "Level", "HP", "Max HP", "MP", "Max MP", "Experience"] {
                ui.strong(h);
            }
            ui.end_row();
            for m in &members {
                let p = join("/Party", m);
                ui.label(m);
                cx.compact(ui, &join(&p, "Level"), 40.0);
                for k in ["HP", "Max HP", "MP", "Max MP"] {
                    cx.compact(ui, &join(&p, k), 60.0);
                }
                cx.compact(ui, &join(&p, "Experience"), 90.0);
                ui.end_row();
            }
        });
        });
        ui.weak(
            "Characters are numbered in the order the save keeps them (Cloud is usually first). The game can recalculate \
             max HP and MP from level and equipment, so raise the level and experience together for lasting changes.",
        );
    });
    ff7r_inventory(ui, cx);
}

/// FF7 Remake / Rebirth: owned materia (level, AP, remove) and materia to add.
fn ff7r_materia(ui: &mut Ui, cx: &mut GameCtx) {
    let all = keys_of(cx, "/Materia");
    if all.is_empty() {
        return;
    }
    panel(ui, &format!("Materia ({})", all.len()), |ui| {
        let q = filter_box(ui, "ff7r_materia", "e.g. Fire, Magnify");
        let shown: Vec<&String> = all.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).collect();
        let area = egui::ScrollArea::vertical().id_salt("ff7r_materia_scroll").max_height(360.0).show_locked(ui, |ui| {
            egui::Grid::new("ff7r_materia_grid").num_columns(5).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                crate::widgets::header_row(ui, "ff7r_materia", ["", "Level", "AP", "Equipped", ""]);
                for m in shown {
                    let p = join("/Materia", m);
                    ui.label(m.as_str());
                    cx.compact(ui, &join(&p, "Level"), 40.0);
                    cx.compact(ui, &join(&p, "AP"), 70.0);
                    ui.label(cx.text(&join(&p, "Equipped")).unwrap_or_default());
                    let rm = join("/Remove materia", m);
                    if cx.get(&rm).is_some() {
                        cx.compact(ui, &rm, 100.0);
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
        });
        crate::widgets::sticky_headers(ui, "ff7r_materia", area.inner_rect);
        ui.weak(
            "Each materia is listed with its slot number, since a save often has several of one kind. Materia someone \
             has equipped can't be removed. In Rebirth, level and AP can be changed up to the materia's top level.",
        );
    });
    let add = "/Add materia";
    let kinds = keys_of(cx, add);
    if kinds.is_empty() {
        return;
    }
    panel(ui, "Add materia", |ui| {
        let q = filter_box(ui, "ff7r_add_materia", "e.g. Fire, Ifrit");
        for c in &kinds {
            let p = join(add, c);
            let rows: Vec<(String, String)> = keys_of(cx, &p).into_iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).map(|k| (k.clone(), join(&p, &k))).collect();
            if rows.is_empty() {
                continue;
            }
            let chosen = rows.iter().filter(|(_, p)| cx.text(p).is_some_and(|v| v != "Don't add")).count();
            let title = if chosen > 0 { format!("{c} materia ({}, {chosen} to add)", rows.len()) } else { format!("{c} materia ({})", rows.len()) };
            egui::CollapsingHeader::new(title).id_salt(("ff7r_add_materia", c.as_str())).default_open(!q.is_empty()).show(ui, |ui| {
                value_rows(ui, cx, &format!("ff7r_add_materia_{c}"), &rows);
            });
        }
        ui.weak("New materia start at level 1 with no AP and go into free materia slots when you save. Summons you already own aren't listed.");
    });
}

/// FF7 Remake / Rebirth: the inventory by category (amounts, remove) and items to add.
fn ff7r_inventory(ui: &mut Ui, cx: &mut GameCtx) {
    let groups = keys_of(cx, "/Inventory");
    if groups.is_empty() {
        return;
    }
    panel(ui, "Inventory", |ui| {
        let labels: Vec<String> = groups.iter().map(|g| format!("{g} ({})", keys_of(cx, &join("/Inventory", g)).len())).collect();
        let t = tabs(ui, "ff7r_inv_tab", &labels);
        let g = &groups[t.min(groups.len() - 1)];
        let q = filter_box(ui, "ff7r_inv", "e.g. Potion, Bangle");
        let base = join("/Inventory", g);
        let names: Vec<String> = keys_of(cx, &base).into_iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).collect();
        egui::ScrollArea::vertical().id_salt("ff7r_inv_scroll").max_height(360.0).show_locked(ui, |ui| {
            egui::Grid::new(("ff7r_inv_grid", g.as_str())).num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                for n in &names {
                    ui.label(n);
                    cx.compact(ui, &join(&base, n), 70.0);
                    let rm = join(&join("/Remove items", g), n);
                    if cx.get(&rm).is_some() {
                        cx.compact(ui, &rm, 100.0);
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
        });
        crafting::offer(Game::Ff7Remake, ui, cx);
        ui.weak(
            "Items and materials go up to 99. Removing frees the slot when you save. Key items, currencies, outfits, \
             folios and trophies are shown but can't be removed. Armor and accessories someone is wearing can't be removed \
             (in Rebirth they can't be removed at all, only changed in number).",
        );
    });
    ff7r_materia(ui, cx);
    let add = "/Add items";
    let cats = keys_of(cx, add);
    if cats.is_empty() {
        return;
    }
    panel(ui, "Add items", |ui| {
        let q = filter_box(ui, "ff7r_add", "e.g. Elixir, Ring");
        for c in &cats {
            let p = join(add, c);
            let rows: Vec<(String, String)> = keys_of(cx, &p).into_iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).map(|k| (k.clone(), join(&p, &k))).collect();
            if rows.is_empty() {
                continue;
            }
            let chosen = rows.iter().filter(|(_, p)| cx.text(p).is_some_and(|v| v != "Don't add")).count();
            let title = if chosen > 0 { format!("{c} ({}, {chosen} to add)", rows.len()) } else { format!("{c} ({})", rows.len()) };
            egui::CollapsingHeader::new(title).id_salt(("ff7r_add", c.as_str())).default_open(!q.is_empty()).show(ui, |ui| {
                value_rows(ui, cx, &format!("ff7r_add_{c}"), &rows);
            });
        }
        ui.weak("Only items this save doesn't have yet are listed. Each goes into a free inventory slot when you save; reopen the file to change its amount afterwards.");
    });
}

// ------------------------------------------------------------------ Hogwarts Legacy (database)

/// Hogwarts Legacy's game database (unpacked from the save).
pub fn is_hogwarts_db(root: &Value) -> bool {
    root.get("MiscDataDynamic").is_some() && root.get("InventoryDynamic").is_some() && root.get("PerkDynamic").is_some()
}

/// Index of the first row of `table` whose columns equal the given values.
fn row_where(root: &Value, table: &str, cols: &[(&str, &str)]) -> Option<usize> {
    root.get(table)?.as_array()?.iter().position(|r| cols.iter().all(|(k, v)| r.get(*k).and_then(Value::as_str) == Some(*v)))
}

pub fn hogwarts_db(ui: &mut Ui, cx: &mut GameCtx) {
    let misc = |cx: &GameCtx, owner: &str, name: &str| row_where(cx.root, "MiscDataDynamic", &[("DataOwner", owner), ("DataName", name)]).map(|i| format!("/MiscDataDynamic/{i}/DataValue"));
    let text = |cx: &GameCtx, owner: &str, name: &str| misc(cx, owner, name).and_then(|p| cx.text(&p)).unwrap_or_default();
    let house = text(cx, "Player", "HouseID");
    let accent = match house.as_str() {
        "Gryffindor" => Color32::from_rgb(170, 40, 40),
        "Slytherin" => Color32::from_rgb(40, 120, 70),
        "Ravenclaw" => Color32::from_rgb(50, 80, 170),
        "Hufflepuff" => Color32::from_rgb(210, 170, 40),
        _ => Color32::from_rgb(120, 90, 160),
    };
    let name = format!("{} {}", text(cx, "Player", "PlayerFirstName"), text(cx, "Player", "PlayerLastName")).trim().to_string();
    let year = text(cx, "Player", "Year");
    let sub = [house.clone(), if year.is_empty() { String::new() } else { format!("Year {year}") }].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("  \u{00B7}  ");
    let gold = row_where(cx.root, "InventoryDynamic", &[("CharacterID", "Player0"), ("HolderID", "ResourceInventory"), ("ItemID", "Knuts")]).map(|i| format!("/InventoryDynamic/{i}/Count"));
    hero(ui, accent, if name.is_empty() { "Hogwarts Legacy" } else { &name }, &sub, |ui| {
        if let Some(g) = &gold {
            cx.tile(ui, "Gold", g, accent);
        }
        if let Some(x) = misc(cx, "ExperienceManager", "ExperiencePoints") {
            cx.tile(ui, "Experience", &x, accent);
        }
        if let Some(t) = misc(cx, "Player0", "PerkPoints") {
            cx.tile(ui, "Talent points", &t, accent);
        }
    });
    // Everything the player holds in their resource bag (ingredients, Moonstone…), and gear storage.
    let rows: Vec<(String, String)> = cx
        .get("/InventoryDynamic")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .enumerate()
                .filter(|(_, r)| r.get("CharacterID").and_then(Value::as_str) == Some("Player0") && r.get("HolderID").and_then(Value::as_str) == Some("ResourceInventory") && r.get("ItemID").and_then(Value::as_str) != Some("Knuts"))
                .map(|(i, r)| (words(r.get("ItemID").and_then(Value::as_str).unwrap_or("").trim_end_matches("_Byproduct")), format!("/InventoryDynamic/{i}/Count")))
                .collect()
        })
        .unwrap_or_default();
    panel(ui, "Resources", |ui| {
        if rows.is_empty() {
            ui.weak("No resources in the bag.");
        }
        value_rows(ui, cx, "hl_res", &rows);
        super::crafting::offer(super::Game::HogwartsDb, ui, cx);
        ui.weak("How many of each ingredient and material you carry. The game caps most stacks at its own limits.");
    });
    let perks: Vec<(String, String)> = cx
        .get("/PerkDynamic")
        .and_then(Value::as_array)
        .map(|a| a.iter().enumerate().map(|(i, r)| (words(r.get("PerkID").and_then(Value::as_str).unwrap_or("").trim_start_matches("Talent_")), format!("/PerkDynamic/{i}/Level"))).collect())
        .unwrap_or_default();
    if !perks.is_empty() {
        panel(ui, "Talents", |ui| {
            value_rows(ui, cx, "hl_perks", &perks);
            ui.weak("Talents you've unlocked and their level. Give yourself talent points above to unlock more in the game.");
        });
    }
    ui.weak("Experience decides your level when the game loads. The whole database (quests, collections, gear) is in the Values tab.");
}

// ------------------------------------------------------------------ Final Fantasy IX

pub fn ff9(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(110, 90, 170);
    let slots: Vec<String> = match cx.root {
        Value::Object(m) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if slots.is_empty() {
        return;
    }
    let t = tabs(ui, "ff9_slot", &slots);
    let base = join("", &slots[t]);
    hero(ui, accent, "Final Fantasy IX", &slots[t], |ui| {
        cx.tile(ui, "Gil", &format!("{base}/Gil"), accent);
    });
    let party = format!("{base}/Party");
    let members: Vec<String> = match cx.get(&party) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    const COLS: [&str; 10] = ["Level", "Experience", "HP", "Max HP", "MP", "Max MP", "Strength", "Magic", "Spirit", "Speed"];
    panel(ui, "Party", |ui| {
        egui::ScrollArea::horizontal().id_salt("ff9_scroll").show_locked(ui, |ui| {
            egui::Grid::new(("ff9_party", t)).num_columns(COLS.len() + 1).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.label("");
                for h in COLS {
                    ui.strong(h);
                }
                ui.end_row();
                for m in &members {
                    let p = join(&party, m);
                    ui.label(m);
                    for k in COLS {
                        cx.compact(ui, &join(&p, k), if k == "Experience" { 80.0 } else { 48.0 });
                    }
                    ui.end_row();
                }
            });
        });
        ui.weak("Strength, Magic, Spirit and Speed are the base values before equipment and abilities. Level 99, HP 9999 and MP 999 are the game's limits.");
    });
}

// ------------------------------------------------------------------ Dying Light 2

pub fn dying_light2(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 120, 40);
    let difficulty = cx.text("/State/#5/Current").unwrap_or_default();
    hero(ui, accent, "Dying Light 2", &if difficulty.is_empty() { String::new() } else { format!("{difficulty} difficulty") }, |ui| {
        cx.tile(ui, "Money", "/Group/Main/Cash", accent);
        cx.tile(ui, "Health", "/HealthState/#1/Health", accent);
    });
    let buckets: Vec<String> = match cx.get("/Bucket") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !buckets.is_empty() {
        panel(ui, "Progression", |ui| {
            egui::Grid::new("dl2_prog").num_columns(3).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                ui.label("");
                ui.strong(tr("Level"));
                ui.strong("Points");
                ui.end_row();
                for b in &buckets {
                    let p = join("/Bucket", b);
                    ui.label(words(b));
                    cx.cell(ui, &join(&p, "Level"), 70.0);
                    cx.cell(ui, &join(&p, "Points"), 100.0);
                    ui.end_row();
                }
            });
            ui.weak("Traversal and Combat are the two skill trees; their points fill the bar to the next level. Inhibitors raise health and stamina.");
        });
    }
    let skills: Vec<(String, String)> = match cx.get("/SkillInstance") {
        Some(Value::Object(m)) => m.keys().map(|k| (words(k), format!("{}/SkillLevel", join("/SkillInstance", k)))).collect(),
        _ => vec![],
    };
    if !skills.is_empty() {
        panel(ui, "Skills", |ui| {
            let q = filter_box(ui, "dl2_sk", "e.g. dash");
            let rows: Vec<(String, String)> = skills.into_iter().filter(|(n, _)| q.is_empty() || n.to_lowercase().contains(&q)).collect();
            egui::ScrollArea::vertical().id_salt("dl2_sk_s").max_height(320.0).show_locked(ui, |ui| value_rows(ui, cx, "dl2_skills", &rows));
            ui.weak("1 means learned. Everything else in the save (items, map, quests) is in the Values tab.");
        });
    }
}

// ------------------------------------------------------------------ Dying Light (2015)

pub fn dying_light(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(190, 100, 40);
    hero(ui, accent, "Dying Light", "Skill trees", |ui| {
        if let Some(Value::Object(m)) = cx.get("/Skill trees") {
            for k in m.keys().cloned().collect::<Vec<_>>() {
                cx.tile(ui, &k, &join("/Skill trees", &k), accent);
            }
        }
    });
    ui.weak("Agility, Power and Survivor go up to 24\u{2013}25 (Legend higher). Inventory and money aren't decoded yet; use Value search for those.");
}

// ------------------------------------------------------------------ The Witcher 3

pub fn witcher3(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(170, 30, 30);
    let level = cx.int("/Geralt/Level").map(|l| format!("Level {l}")).unwrap_or_default();
    hero(ui, accent, "Geralt of Rivia", &level, |ui| {
        cx.tile(ui, "Crowns", "/Geralt/Crowns", accent);
        cx.tile(ui, "Level", "/Geralt/Level", accent);
        cx.tile(ui, "Unspent skill points", "/Geralt/Skill points/Unspent", accent);
    });
    panel(ui, "Progress", |ui| {
        value_rows(ui, cx, "w3_prog", &[
            ("Skill points spent".into(), "/Geralt/Skill points/Spent".into()),
            ("Experience toward next level".into(), "/Geralt/Experience/Toward next level".into()),
            ("Total experience".into(), "/Geralt/Experience/Total".into()),
        ]);
        if cx.get("/Geralt/Skill points/Unspent").is_none() {
            ui.weak("This save has no unspent skill points: the game doesn't store a 0, so there's nothing here to raise \u{2014} earn one in the game first.");
        }
        ui.weak("The game decides the level from the total experience when it loads, so change both together.");
    });
    story(ui, cx, accent);
}

// ------------------------------------------------------------------ Cyberpunk 2077

pub fn cyberpunk(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(230, 210, 40);
    hero(ui, accent, "Cyberpunk 2077", "V", |ui| {
        cx.tile(ui, "Eurodollars", "/V/Eurodollars", accent);
    });
    ui.weak("The eddies in V's own inventory. Everything else in this save is in the Hex view: use Value search with a number you know.");
}

// ------------------------------------------------------------------ Kenshi

pub fn kenshi(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(170, 130, 80);
    hero(ui, accent, "Kenshi", "Your faction", |ui| {
        cx.tile(ui, "Cats", "/Player/Money (cats)", accent);
    });
    ui.weak("Each squad's characters (attributes, skills, health) are in the save's platoon folder: open the file named after your faction, such as platoon\\Nameless_0.platoon, for its Squad screen.");
}

fn kenshi_keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

/// Every attribute and skill of one member to 100 (the game's own cap).
fn kenshi_max(cx: &mut GameCtx, base: &str) -> usize {
    let mut n = 0;
    for group in ["Attributes", "Skills"] {
        let g = join(base, group);
        for k in kenshi_keys(cx, &g) {
            let p = join(&g, &k);
            if cx.can_edit(&p) && cx.num(&p).is_some_and(|v| v < 100.0) {
                cx.set(&p, Value::from(100.0));
                n += 1;
            }
        }
    }
    n
}

pub fn kenshi_squad(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(170, 130, 80);
    let members = kenshi_keys(cx, "/Members");
    let player = cx.flag("/Squad/Player's squad");
    let sub = if player { "Your squad" } else { "A squad of another faction" };
    hero(ui, accent, "Kenshi", sub, |ui| {
        fact(ui, "Members", &members.len().to_string(), accent);
        if super::quick_button(ui, "Max everyone's stats", "Every attribute and skill of every member in this file to 100, the most the game allows.") {
            for m in &members {
                kenshi_max(cx, &join("/Members", m));
            }
        }
    });
    if members.is_empty() {
        return;
    }
    let t = tabs(ui, "kenshi_squad", &members);
    let base = join("/Members", &members[t]);
    let mut line = vec![];
    if cx.flag(&join(&base, "Squad leader")) {
        line.push("Squad leader");
    }
    if !cx.flag(&join(&base, "Player's")) {
        line.push("Not in your faction");
    }
    if !line.is_empty() {
        ui.weak(line.join("  \u{00B7}  "));
    }
    let attrs = join(&base, "Attributes");
    let skills = join(&base, "Skills");
    columns(ui, 360.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Attributes", |ui| {
                for k in kenshi_keys(cx, &attrs) {
                    cx.attribute(ui, &k, &join(&attrs, &k), 100.0, accent);
                }
                if ui.button("Max this character").on_hover_text("Every attribute and skill to 100.").clicked() {
                    kenshi_max(cx, &base);
                }
            });
            kenshi_health(ui, cx, &base);
        } else {
            panel(ui, "Skills", |ui| {
                for k in kenshi_keys(cx, &skills) {
                    cx.attribute(ui, &k, &join(&skills, &k), 100.0, accent);
                }
                ui.weak("0\u{2013}100. Skills never trained sit a little below zero; the game levels them up from there.");
            });
        }
    });
}

fn kenshi_health(ui: &mut Ui, cx: &mut GameCtx, base: &str) {
    let health = join(base, "Health");
    let parts = kenshi_keys(cx, &health);
    if parts.is_empty() {
        return;
    }
    let limbs = join(base, "Limbs");
    panel(ui, "Health", |ui| {
        egui::Grid::new(("kenshi_hp", base)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            for p in &parts {
                cx.grid_row(ui, p, &join(&health, p), 90.0);
            }
            let cond = join(base, "Condition");
            for k in kenshi_keys(cx, &cond) {
                cx.grid_row(ui, &k, &join(&cond, &k), 90.0);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let status = join(base, "Status");
            for k in kenshi_keys(cx, &status) {
                cx.chip(ui, &k, &join(&status, &k), Color32::from_rgb(190, 70, 60));
            }
        });
        // A part's most health comes from the race in the game's data, not from the save, so
        // there is no "full health" to fill up to here: parts are typed in.
        let tip = "Clears dead, unconscious, coma and incapacitated and ends bleeding. Raise any part below zero above it too, or the character falls again.";
        if ui.button("Revive").on_hover_text(tip).clicked() {
            let bleed = join(&join(base, "Condition"), "Bleeding");
            if cx.num(&bleed).is_some_and(|v| v > 0.0) {
                cx.set(&bleed, Value::from(0.0));
            }
            let status = join(base, "Status");
            for k in kenshi_keys(cx, &status) {
                if cx.flag(&join(&status, &k)) {
                    cx.set(&join(&status, &k), Value::Bool(false));
                }
            }
        }
        ui.weak("Each part's health. The game keeps the most a part can have in its race data, not in the save, so it isn't shown here; healthy humans have the same value on every part.");
        let limb_names = kenshi_keys(cx, &limbs);
        if !limb_names.is_empty() {
            ui.add_space(6.0);
            egui::Grid::new(("kenshi_limbs", base)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for l in &limb_names {
                    cx.grid_row(ui, l, &join(&limbs, l), 110.0);
                }
            });
            ui.weak("A lost limb also needs its health above zero to come back. Robotic limbs are read from very few saves.");
        }
    });
}

// ------------------------------------------------------------------ Palworld (player file)

pub fn palworld(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(70, 170, 210);
    hero(ui, accent, "Palworld", "Player", |ui| {
        cx.tile(ui, "Technology points", "/SaveData/TechnologyPoint", accent);
        cx.tile(ui, "Ancient technology points", "/SaveData/bossTechnologyPoint", accent);
        cx.tile(ui, "Lifmunk effigies", "/SaveData/RecordData/RelicPossessNum", accent);
    });
    let rows: Vec<(String, String)> = match cx.get("/SaveData/RecordData") {
        Some(Value::Object(m)) => m.iter().filter(|(_, v)| v.is_number()).map(|(k, _)| (words(k), join("/SaveData/RecordData", k))).collect(),
        _ => vec![],
    };
    if !rows.is_empty() {
        panel(ui, "Records", |ui| {
            value_rows(ui, cx, "pal_rec", &rows);
            ui.weak("Counters the game keeps for this player (captures, dungeons, effigies). Level and Pals are in Level.sav in the world folder.");
        });
    }
}

// ------------------------------------------------------------------ Persona 3 Reload

pub fn p3r(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(40, 150, 230);
    hero(ui, accent, "Persona 3 Reload", "Protagonist", |ui| {
        cx.tile(ui, "Yen", "/Protagonist/Yen", accent);
        cx.tile(ui, "Level", "/Party/Protagonist/Level", accent);
        cx.tile(ui, "Experience", "/Party/Protagonist/Experience", accent);
    });
    panel(ui, "Social stats", |ui| {
        let rows: Vec<(String, String)> = ["Academics", "Charm", "Courage"].iter().map(|s| (s.to_string(), join("/Social stats", s))).collect();
        value_rows(ui, cx, "p3r_soc", &rows);
    });
    let members = ["Protagonist", "Yukari", "Junpei", "Akihiko", "Mitsuru", "Fuuka", "Aigis", "Koromaru", "Ken", "Shinjiro"];
    for name in members {
        let base = join("/Party", name);
        if cx.get(&join(&base, "Level")).is_none() {
            continue;
        }
        panel(ui, name, |ui| {
            let rows: Vec<(String, String)> = ["Level", "Experience", "HP", "SP"].iter().map(|k| (k.to_string(), join(&base, k))).collect();
            value_rows(ui, cx, &format!("p3r_{name}"), &rows);
        });
    }
    panel(ui, "Social Links", |ui| {
        let keys: Vec<String> = cx.get("/Social links").and_then(|v| v.as_object()).map(|m| m.keys().cloned().collect()).unwrap_or_default();
        let rows: Vec<(String, String)> = keys.iter().map(|k| (k.clone(), join("/Social links", k))).collect();
        value_rows(ui, cx, "p3r_links", &rows);
    });
}

// ------------------------------------------------------------------ Persona 4 Golden

pub fn p4g(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(230, 190, 30);
    hero(ui, accent, "Persona 4 Golden", "Protagonist", |ui| {
        cx.tile(ui, "Yen", "/Protagonist/Yen", accent);
        cx.tile(ui, "Level", "/Protagonist/Level", accent);
        cx.tile(ui, "Total experience", "/Protagonist/Total experience", accent);
    });
    panel(ui, "Social stats", |ui| {
        let rows: Vec<(String, String)> = ["Knowledge", "Courage", "Diligence", "Understanding", "Expression"].iter().map(|s| (s.to_string(), join("/Social stats", s))).collect();
        value_rows(ui, cx, "p4g_soc", &rows);
        ui.weak("These are the points behind each stat's rank. The level follows the experience when the game loads it, so change both together.");
    });
}

// ------------------------------------------------------------------ Final Fantasy X-2

pub fn ffx2(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(220, 80, 140);
    hero(ui, accent, "Final Fantasy X-2", "Gullwings", |ui| {
        cx.tile(ui, "Gil", "/Gullwings/Gil", accent);
    });
    const COLS: [&str; 14] = ["Level", "Experience", "HP", "Max HP", "MP", "Max MP", "Strength", "Defense", "Magic", "Magic defense", "Agility", "Accuracy", "Evasion", "Luck"];
    panel(ui, "Party", |ui| {
        egui::ScrollArea::horizontal().id_salt("ffx2_scroll").show_locked(ui, |ui| {
            egui::Grid::new("ffx2_party").num_columns(COLS.len() + 1).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.label("");
                for h in COLS {
                    ui.strong(h);
                }
                ui.end_row();
                for m in ["Yuna", "Rikku", "Paine"] {
                    let p = join("/Party", m);
                    ui.label(m);
                    for k in COLS {
                        cx.compact(ui, &join(&p, k), if k == "Experience" { 80.0 } else { 52.0 });
                    }
                    ui.end_row();
                }
            });
        });
        ui.weak("Stats are the base values before the dressphere and accessories.");
    });
}

// ------------------------------------------------------------------ Persona 5 Royal

pub fn p5r(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(220, 30, 40);
    hero(ui, accent, "Persona 5 Royal", "Joker", |ui| {
        cx.tile(ui, "Money", "/Joker/Money", accent);
        cx.tile(ui, "Level", "/Joker/Level", accent);
        cx.tile(ui, "Experience", "/Joker/Experience", accent);
        cx.tile(ui, "HP", "/Joker/HP", accent);
        cx.tile(ui, "SP", "/Joker/SP", accent);
    });
    const STATS: [&str; 7] = ["Level", "Experience", "Strength", "Magic", "Endurance", "Agility", "Luck"];
    let grid = |ui: &mut Ui, cx: &mut GameCtx, id: &str, rows: &[(String, String)], hp: bool| {
        egui::ScrollArea::horizontal().id_salt((id, "s")).show_locked(ui, |ui| {
            egui::Grid::new(id).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.label("");
                if hp {
                    ui.strong("HP");
                    ui.strong("SP");
                }
                for h in STATS {
                    ui.strong(h);
                }
                ui.end_row();
                for (label, base) in rows {
                    ui.label(label);
                    if hp {
                        let who = base.trim_end_matches("/Persona");
                        cx.compact(ui, &join(who, "HP"), 52.0);
                        cx.compact(ui, &join(who, "SP"), 52.0);
                    }
                    for k in STATS {
                        cx.compact(ui, &join(base, k), if k == "Experience" { 90.0 } else { 44.0 });
                    }
                    ui.end_row();
                }
            });
        });
    };
    let party: Vec<(String, String)> = ["Ryuji", "Morgana", "Ann", "Yusuke", "Makoto", "Haru", "Futaba", "Akechi", "Kasumi"]
        .iter()
        .filter(|w| cx.get(&format!("/{w}/Persona")).is_some())
        .map(|w| (w.to_string(), format!("/{w}/Persona")))
        .collect();
    panel(ui, "Party", |ui| {
        grid(ui, cx, "p5r_party", &party, true);
        ui.weak("A party member's level and stats are their Persona's.");
    });
    let personas: Vec<(String, String)> = match cx.get("/Joker") {
        Some(Value::Object(m)) => m.keys().filter(|k| k.starts_with("Persona ")).map(|k| (k.clone(), join("/Joker", k))).collect(),
        _ => vec![],
    };
    if !personas.is_empty() {
        panel(ui, "Joker's Personas", |ui| {
            grid(ui, cx, "p5r_personas", &personas, false);
            ui.weak("The number after each slot is the Persona's id in the game's compendium.");
        });
    }
}

// ------------------------------------------------------------------ Final Fantasy X

pub fn ffx(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(60, 140, 220);
    hero(ui, accent, "Final Fantasy X", "Party", |ui| {
        cx.tile(ui, "Gil", "/Party/Gil", accent);
    });
    const COLS: [&str; 12] = ["HP", "Max HP", "MP", "Max MP", "Strength", "Defense", "Magic", "Magic defense", "Agility", "Luck", "Evasion", "Accuracy"];
    const BASE: [&str; 10] = [
        "Base max HP", "Base max MP", "Base strength", "Base defense", "Base magic", "Base magic defense", "Base agility", "Base luck",
        "Base evasion", "Base accuracy",
    ];
    const GROW: [&str; 4] = ["AP", "Sphere levels", "Overdrive gauge", "Overdrive mode"];
    let pages = ["Party".to_string(), "Abilities".into(), "Overdrives".into(), "Items".into(), "Monster Arena".into()];
    match tabs(ui, "ffx_tab", &pages) {
        0 => {
            for group in ["Characters", "Aeons"] {
                let who = keys_of(cx, &join("", group));
                if who.is_empty() {
                    continue;
                }
                for (title, cols) in [(group.to_string(), &COLS[..]), (format!("{group}: without equipment"), &BASE[..]), (format!("{group}: growth"), &GROW[..])] {
                    panel(ui, &title, |ui| {
                        egui::ScrollArea::horizontal().id_salt(("ffx_s", &title)).show_locked(ui, |ui| {
                            egui::Grid::new(("ffx_g", &title)).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                                ui.label("");
                                for h in cols {
                                    ui.strong(h.trim_start_matches("Base ").to_string());
                                }
                                ui.end_row();
                                for w in &who {
                                    let p = join(&join("", group), w);
                                    ui.label(w);
                                    for k in cols {
                                        let w = if k.contains("mode") { 110.0 } else if k.contains("HP") || *k == "AP" { 72.0 } else { 46.0 };
                                        if cx.get(&join(&p, k)).is_some() {
                                            cx.compact(ui, &join(&p, k), w);
                                        } else {
                                            ui.label("");
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                    });
                }
            }
            ui.weak(
                "Stats are as the Sphere Grid has raised them (255 is the most). HP above 9999 needs Break HP Limit. The game works the totals \
                 out again from the \"without equipment\" numbers, so change both. Sphere levels are the moves left to spend on the Sphere Grid.",
            );
        }
        1 => ffx_flags(ui, cx, "/Abilities", "ability", accent, true),
        2 => {
            ffx_flags(ui, cx, "/Overdrives", "overdrive", accent, false);
            ffx_flags(ui, cx, "/Overdrive modes", "overdrive mode", accent, false);
        }
        3 => {
            panel(ui, "Items", |ui| {
                let slots = keys_of(cx, "/Items");
                if super::quick_button(ui, "Set every stack to 99", "Raises the count of each item you already hold to 99.") {
                    for s in &slots {
                        let p = join("/Items", s);
                        if cx.text(&join(&p, "Item")).is_some_and(|t| t != "(empty)") {
                            cx.set(&join(&p, "Count"), Value::from(99));
                        }
                    }
                }
                let cols = ((ui.available_width() / 330.0).floor() as usize).clamp(1, 3);
                let per = slots.len().div_ceil(cols).max(1);
                ui.columns(cols, |cs| {
                    for (c, chunk) in slots.chunks(per).enumerate() {
                        let ui = &mut cs[c];
                        egui::Grid::new(("ffx_items", c)).striped(true).spacing([8.0, 3.0]).show(ui, |ui| {
                            for s in chunk {
                                let p = join("/Items", s);
                                ui.weak(s.trim_start_matches("Slot "));
                                cx.compact(ui, &join(&p, "Item"), 190.0);
                                cx.compact(ui, &join(&p, "Count"), 46.0);
                                ui.end_row();
                            }
                        });
                    }
                });
                ui.weak("Pick an item for an empty slot and give it a count to add it. Keep each item in one slot only.");
            });
            let keys = keys_of(cx, "/Key items");
            if !keys.is_empty() {
                panel(ui, "Key items", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for k in &keys {
                            cx.chip(ui, k, &join("/Key items", k), accent);
                        }
                    });
                    ui.weak("Story items: holding one early doesn't play the scene that normally gives it. The Al Bhed primers aren't listed.");
                });
            }
        }
        _ => {
            panel(ui, "Monster Arena", |ui| {
                let regions = keys_of(cx, "/Monster Arena");
                if super::quick_button(ui, "Capture 10 of every fiend", "Sets every area's fiends to 10 captured. The arena's own creations are left alone.") {
                    for r in &regions {
                        if r.contains("Conquest") || r == "Original" {
                            continue;
                        }
                        for m in keys_of(cx, &join("/Monster Arena", r)) {
                            cx.set(&join(&join("/Monster Arena", r), &m), Value::from(10));
                        }
                    }
                }
                for r in &regions {
                    let base = join("/Monster Arena", r);
                    let rows: Vec<(String, String)> = keys_of(cx, &base).into_iter().map(|m| (m.clone(), join(&base, &m))).collect();
                    egui::CollapsingHeader::new(r).id_salt(("ffx_arena", r)).show(ui, |ui| value_rows(ui, cx, &format!("ffx_arena_{r}"), &rows));
                }
                ui.weak("How many of each fiend the Monster Arena holds (10 at most). The conquest and original entries are the arena's creations: 1 means unlocked.");
            });
        }
    }
}

/// Chips for on/off records grouped as `base/<who>/<name>` or `base/<who>/<group>/<name>`.
fn ffx_flags(ui: &mut Ui, cx: &mut GameCtx, base: &str, what: &str, accent: Color32, nested: bool) {
    let who = keys_of(cx, base);
    if who.is_empty() {
        return;
    }
    panel(ui, base.trim_start_matches('/'), |ui| {
        for w in &who {
            let wb = join(base, w);
            let groups: Vec<(String, String)> =
                if nested { keys_of(cx, &wb).into_iter().map(|g| (g.clone(), join(&wb, &g))).collect() } else { vec![(String::new(), wb.clone())] };
            let all: Vec<String> = groups.iter().flat_map(|(_, gb)| keys_of(cx, gb).into_iter().map(|k| join(gb, &k)).collect::<Vec<_>>()).collect();
            let on = all.iter().filter(|p| cx.flag(p)).count();
            egui::CollapsingHeader::new(format!("{w}  ({on} of {})", all.len())).id_salt((base, w)).show(ui, |ui| {
                if ui.small_button(format!("Learn every {what}")).clicked() {
                    for p in &all {
                        cx.set(p, Value::from(1));
                    }
                }
                for (g, gb) in &groups {
                    if !g.is_empty() {
                        ui.strong(g);
                    }
                    ui.horizontal_wrapped(|ui| {
                        for k in keys_of(cx, gb) {
                            cx.chip(ui, &k, &join(gb, &k), accent);
                        }
                    });
                }
            });
        }
    });
}

// ------------------------------------------------------------------ Metro

pub fn metro(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 160, 60);
    const MONEY: &str = "/Inventory/Military-grade rounds (money)";
    hero(ui, accent, "Metro", "Artyom", |ui| {
        cx.tile(ui, "Military-grade rounds", MONEY, accent);
    });
    let rows: Vec<(String, String)> = match cx.get("/Inventory") {
        Some(Value::Object(m)) => m.keys().filter(|k| !k.starts_with("Military")).map(|k| (k.clone(), join("/Inventory", k))).collect(),
        _ => vec![],
    };
    panel(ui, "Ammunition", |ui| {
        value_rows(ui, cx, "metro_ammo", &rows);
        if rows.iter().any(|(k, _)| k.contains("(crafting)")) {
            super::crafting::offer(super::Game::Metro, ui, cx);
        }
        ui.weak("What Artyom carries at the start of this level (chapter select loads it). The game may cap a count at what Artyom can carry.");
    });
}

// ------------------------------------------------------------------ Yakuza 0

pub fn yakuza0(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 40, 50);
    let sub = [cx.text("/Game/Saved").map(|t| format!("Saved {t}")), cx.text("/Game/Playing as").map(|w| format!("playing as {w}"))]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("  \u{00B7}  ");
    hero(ui, accent, "Yakuza 0", &sub, |ui| {
        cx.tile(ui, "Kiryu's money (\u{00A5})", "/Kiryu/Money", accent);
        cx.tile(ui, "Majima's money (\u{00A5})", "/Majima/Money", accent);
    });
    panel(ui, "Kiryu", |ui| {
        let rows = vec![
            ("CP".to_string(), "/Kiryu/CP".to_string()),
            ("Real estate takings (Kamurocho)".to_string(), "/Kiryu/Real estate takings (Kamurocho)".to_string()),
        ];
        value_rows(ui, cx, "y0_kiryu", &rows);
    });
    panel(ui, "Majima", |ui| {
        let rows = vec![
            ("CP".to_string(), "/Majima/CP".to_string()),
            ("Cabaret takings (Sotenbori)".to_string(), "/Majima/Cabaret takings (Sotenbori)".to_string()),
        ];
        value_rows(ui, cx, "y0_majima", &rows);
    });
    panel(ui, "Game", |ui| {
        value_rows(ui, cx, "y0_game", &[("Difficulty".to_string(), "/Game/Difficulty".to_string())]);
        ui.weak("Takings are what the businesses have earned and you haven't collected yet. Money tops out at \u{00A5}9,999,999,999,999.");
    });
}

// ------------------------------------------------------------------ Yakuza (Dragon Engine: Kiwami 2, Like a Dragon, …)

pub fn yakuza_de(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(210, 160, 40);
    let y6 = cx.get("/base/m_title").is_none() && cx.get("/mg_spearfishing").is_some();
    let title = cx.text("/base/m_title").unwrap_or_else(|| if y6 { "Yakuza 6: The Song of Life".into() } else { "Yakuza".into() });
    let sub = cx.text("/base/m_subtitle").unwrap_or_default();
    let party = cx.get("/party/m_chara").and_then(|v| v.as_array().map(Vec::len)).unwrap_or(0);
    // Kiwami 3 keeps its points in lists (the first is Kiryu's) and its HP per character.
    let money = if cx.get("/player/point/m_point/1").is_some() { "/player/point/m_point/1" } else { "/player/m_points/m_list/0/m_point/1" };
    let chars = cx.get("/player/m_chara").and_then(|v| v.as_array().map(Vec::len)).unwrap_or(0);
    hero(ui, accent, &title, &sub, |ui| {
        cx.tile(ui, "Money (\u{00A5})", money, accent);
        if cx.get("/player/hp/m_max").is_some() {
            cx.tile(ui, "Max HP", "/player/hp/m_max", accent);
            cx.tile(ui, "HP", "/player/hp/m_now", accent);
        } else if chars > 0 {
            cx.tile(ui, "Kiryu's max HP", "/player/m_chara/0/hp/m_max", accent);
            cx.tile(ui, "Kiryu's HP", "/player/m_chara/0/hp/m_now", accent);
        }
    });
    if party == 0 && chars > 0 {
        panel(ui, "Characters", |ui| {
            egui::Grid::new("yde_chara").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in ["", "HP", "Max HP", "Heat", "Max heat"] {
                    ui.strong(h);
                }
                ui.end_row();
                for i in 0..chars {
                    let base = format!("/player/m_chara/{i}");
                    if cx.get(&format!("{base}/hp/m_max")).and_then(|v| v.as_i64()).unwrap_or(0) == 0 {
                        continue;
                    }
                    ui.label(if i == 0 { "Kiryu".to_string() } else { format!("Character {}", i + 1) });
                    for k in ["hp/m_now", "hp/m_max", "heat/m_now", "heat/m_max"] {
                        cx.compact(ui, &format!("{base}/{k}"), 70.0);
                    }
                    ui.end_row();
                }
            });
            ui.weak("Every character the save keeps stats for, in the save's order; the first is Kiryu.");
        });
    }
    // Kiwami 2 and Yakuza 6 keep Kiryu's five experience pools next to the money.
    if party == 0 && !title.starts_with("Judgment") && cx.get("/player/point/m_point/6").is_some() {
        panel(ui, "Experience", |ui| {
            let rows: Vec<(String, String)> = (2..=6).map(|i| (format!("Experience pool {}", i - 1), format!("/player/point/m_point/{i}"))).collect();
            value_rows(ui, cx, "yde_exp", &rows);
            ui.weak("The five kinds of experience you spend in the Growth menu, in the order the save keeps them.");
        });
    }
    if party > 0 {
        const COLS: [(&str, &str); 6] = [
            ("Level", "exp/m_level"),
            ("Experience", "exp/m_exp"),
            ("HP", "hp/m_now"),
            ("Max HP", "hp/m_max"),
            ("MP", "heat/m_now"),
            ("Max MP", "heat/m_max"),
        ];
        panel(ui, "Party", |ui| {
            egui::ScrollArea::horizontal().id_salt("yde_party_s").show_locked(ui, |ui| {
                egui::Grid::new("yde_party").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label("");
                    for (h, _) in COLS {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for i in 0..party {
                        let base = format!("/party/m_chara/{i}");
                        ui.label(if i == 0 { "Ichiban".to_string() } else { format!("Member {}", i + 1) });
                        for (h, k) in COLS {
                            cx.compact(ui, &format!("{base}/{k}"), if h == "Experience" { 120.0 } else { 56.0 });
                        }
                        ui.end_row();
                    }
                });
            });
            ui.weak("Every party character the save knows about, in the save's order; the first is Ichiban.");
        });
    }
    ui.weak("The save's checksum is updated when you save.");
}

// ------------------------------------------------------------------ Yakuza Kiwami

pub fn kiwami(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(190, 40, 60);
    let sub = cx.text("/Game/Saved").map(|t| format!("Saved {t}")).unwrap_or_default();
    hero(ui, accent, "Yakuza Kiwami", &sub, |ui| {
        cx.tile(ui, "Money (\u{00A5})", "/Kiryu/Money", accent);
        cx.tile(ui, "Experience to spend", "/Kiryu/Experience to spend", accent);
    });
    ui.weak("Experience is what Kiryu has earned and not yet spent in the Abilities menu (99,999,999 at most). Money tops out at \u{00A5}9,999,999,999.");
}

// ------------------------------------------------------------------ Final Fantasy XV

pub fn ffxv(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(70, 110, 190);
    let mut sub = vec![];
    if let Some(c) = cx.int("/Save/Chapter") {
        sub.push(format!("Chapter {c}"));
    }
    if let Some(l) = cx.int("/Save/Level shown") {
        sub.push(format!("Level {l}"));
    }
    if let Some(t) = cx.text("/Save/Play time") {
        sub.push(format!("{t} played"));
    }
    hero(ui, accent, "Final Fantasy XV", &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Gil", "/Party/Gil", accent);
        cx.tile(ui, "Ability points", "/Party/Ability points", accent);
        cx.tile(ui, "Medals", "/Party/Medals", accent);
    });
    const COLS: [&str; 12] = ["Level", "Experience", "Total experience", "HP", "Max HP", "MP", "Max MP", "Strength", "Vitality", "Magic", "Spirit", "Kills"];
    for group in ["Party", "Others"] {
        let base = format!("/Characters/{group}");
        let who: Vec<String> = match cx.get(&base) {
            Some(Value::Object(m)) => m.keys().cloned().collect(),
            _ => continue,
        };
        let title = if group == "Party" { "Party" } else { "Guests and other characters" };
        panel(ui, title, |ui| {
            egui::ScrollArea::horizontal().id_salt(("ffxv_s", group)).show_locked(ui, |ui| {
                egui::Grid::new(("ffxv_g", group)).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label("");
                    for h in COLS {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for w in &who {
                        let p = join(&base, w);
                        ui.label(w);
                        for k in COLS {
                            cx.compact(ui, &join(&p, k), if k.contains("xperience") { 90.0 } else { 52.0 });
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }
    ui.weak("Experience is what a character has earned since the last rest; the game banks it, and raises the level, when the party rests at a camp or hotel. Levels, HP, MP and stats are shown as saved.");
    let lists: Vec<String> = match cx.get("/Items") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !lists.is_empty() {
        panel(ui, "Items", |ui| {
            let i = tabs(ui, "ffxv_items", &lists);
            let p = join("/Items", &lists[i]);
            let rows: Vec<(String, String)> = match cx.get(&p) {
                Some(Value::Object(m)) => m.keys().map(|k| (k.clone(), join(&p, k))).collect(),
                _ => vec![],
            };
            value_rows(ui, cx, &format!("ffxv_{i}"), &rows);
            if cx.get("/Items/Treasures").is_some() {
                super::crafting::offer(super::Game::Ffxv, ui, cx);
            }
            ui.weak("Items are listed by the game's item number (names aren't known yet); 999 is the most the editor lets you set.");
        });
    }
    ui.weak("Not yet tried in the game.");
}

// ------------------------------------------------------------------ Death Stranding 2

pub fn death_stranding2(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 150, 60);
    let mut sub = vec![];
    if let Some(d) = cx.int("/Save/Day") {
        sub.push(format!("Day {d}{}", cx.text("/Save/Time of day").map(|t| format!(", {t}")).unwrap_or_default()));
    }
    if let Some(t) = cx.text("/Save/Play time") {
        sub.push(format!("{t} played"));
    }
    hero(ui, accent, "Death Stranding 2", &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Likes from NPCs", "/Likes/From NPCs", accent);
        cx.tile(ui, "Likes from other porters", "/Likes/From other porters", accent);
        cx.tile(ui, "Likes given", "/Likes/Given", accent);
    });
    let names: Vec<String> = match cx.get("/Facilities") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if names.is_empty() {
        return;
    }
    panel(ui, "Facility material stocks", |ui| {
        egui::ScrollArea::horizontal().id_salt("ds2_s").show_locked(ui, |ui| {
            egui::Grid::new("ds2_g").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.strong(tr("Facility"));
                ui.strong(tr("Connection"));
                for m in crate::fmt::ds2::MATERIALS {
                    ui.strong(m);
                }
                ui.end_row();
                for n in &names {
                    let p = join("/Facilities", n);
                    ui.label(n);
                    cx.compact(ui, &join(&p, "Connection level"), 40.0);
                    for m in crate::fmt::ds2::MATERIALS {
                        cx.compact(ui, &join(&p, m), 72.0);
                    }
                    ui.end_row();
                }
            });
        });
        ui.weak("Facilities you have found or connected, with the materials stored at each. Connection level runs 0\u{2013}5.");
        super::crafting::offer(super::Game::DeathStranding2, ui, cx);
    });
    ui.weak("The save's MD5 is written again when you save. Not yet tried in the game.");
}

// ------------------------------------------------------------------ Death Stranding

pub fn death_stranding1(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 150, 60);
    let sub = cx.int("/Save/Connected facilities").map(|n| format!("{n} facilities connected")).unwrap_or_default();
    hero(ui, accent, "Death Stranding", &sub, |ui| {
        cx.tile(ui, "Likes received", "/Likes/Likes received", accent);
    });
    let names: Vec<String> = match cx.get("/Facilities") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if names.is_empty() {
        return;
    }
    panel(ui, "Facility material stocks", |ui| {
        egui::ScrollArea::horizontal().id_salt("ds1_s").show_locked(ui, |ui| {
            egui::Grid::new("ds1_g").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                ui.strong(tr("Facility"));
                ui.strong(tr("Connection"));
                for m in crate::fmt::ds1::MATERIALS {
                    ui.strong(m);
                }
                ui.end_row();
                for n in &names {
                    let p = join("/Facilities", n);
                    ui.label(n);
                    cx.compact(ui, &join(&p, "Connection level"), 40.0);
                    for m in crate::fmt::ds1::MATERIALS {
                        cx.compact(ui, &join(&p, m), 72.0);
                    }
                    ui.end_row();
                }
            });
        });
        ui.weak("Facilities you have found or connected, with the materials stored at each. Connection level runs 0\u{2013}5.");
        super::crafting::offer(super::Game::DeathStranding1, ui, cx);
    });
    ui.weak("Needs the Oodle library that many games install (found automatically). The save's MD5 is written again when you save. Not yet tried in the game.");
}

// ------------------------------------------------------------------ Mass Effect: Andromeda

pub fn andromeda(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(60, 170, 200);
    let mut sub = vec![];
    if let Some(l) = cx.text("/Save/Location") {
        sub.push(l);
    }
    if let Some(c) = cx.num("/Save/Completion %") {
        sub.push(format!("{c}% complete"));
    }
    if let Some(h) = cx.num("/Save/Play time (hours)") {
        sub.push(format!("{h:.1} hours played"));
    }
    hero(ui, accent, "Mass Effect: Andromeda", &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Credits", "/Ryder/Credits", accent);
        cx.tile(ui, "Ryder's level", "/Ryder/Level", accent);
        cx.tile(ui, "Skill points", "/Ryder/Skill points", accent);
    });
    let squad: Vec<String> = match cx.get("/Squad") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !squad.is_empty() {
        panel(ui, "Squad", |ui| {
            egui::Grid::new("mea_squad").striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                ui.strong("");
                ui.strong(tr("Level"));
                ui.strong("Skill points");
                ui.end_row();
                for n in &squad {
                    let p = join("/Squad", n);
                    ui.label(n);
                    cx.compact(ui, &join(&p, "Level"), 52.0);
                    cx.compact(ui, &join(&p, "Skill points"), 60.0);
                    ui.end_row();
                }
            });
        });
    }
    let groups: Vec<String> = match cx.get("/Resources") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    if !groups.is_empty() {
        panel(ui, "Resources", |ui| {
            let i = tabs(ui, "mea_res", &groups);
            let p = join("/Resources", &groups[i]);
            let rows: Vec<(String, String)> = match cx.get(&p) {
                Some(Value::Object(m)) => m.keys().map(|k| (k.clone(), join(&p, k))).collect(),
                _ => vec![],
            };
            value_rows(ui, cx, &format!("mea_{i}"), &rows);
            super::crafting::offer(super::Game::Andromeda, ui, cx);
            ui.weak("Research data, minerals, salvage and the other stacks in Ryder's inventory, named from the save itself.");
        });
    }
    ui.weak("A level change is copied to the character's progression too; the load screen keeps showing the old level until the game saves again. Both checksums are written again on save. Not yet tried in the game.");
}

// ------------------------------------------------------------------ God of War (2018)

/// "Add equipment": a drop-down per piece the save doesn't have, grouped by type.
fn gow_add_panel(ui: &mut Ui, cx: &mut GameCtx, base: &str, id: &str, note: &str) {
    let add = join(base, "Add equipment");
    let cats = keys_of(cx, &add);
    if cats.is_empty() {
        return;
    }
    panel(ui, "Add equipment", |ui| {
        for c in &cats {
            let p = join(&add, c);
            let rows: Vec<(String, String)> = keys_of(cx, &p).into_iter().map(|k| (k.clone(), join(&p, &k))).collect();
            let chosen = rows.iter().filter(|(_, p)| cx.text(p).is_some_and(|v| v != "Don't add")).count();
            let title = if chosen > 0 { format!("{c} ({}, {chosen} to add)", rows.len()) } else { format!("{c} ({})", rows.len()) };
            egui::CollapsingHeader::new(title).id_salt((id, c.as_str())).show(ui, |ui| {
                value_rows(ui, cx, &format!("{id}_{c}"), &rows);
            });
        }
        ui.weak(note);
    });
}

/// Asked of the app: open the "Back up this save" dialog (set by [`story_guard`]).
pub const STORY_BACKUP_REQUEST: &str = "story_backup_request";

/// The warning shown above every story and choice editor, and its lock: the controls inside
/// stay disabled until the person keeps a named backup or says they accept the risk (once per
/// run of the editor).
pub(crate) fn story_guard(ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
    let ack = egui::Id::new("story_edits_acknowledged");
    let ok = ui.ctx().data(|d| d.get_temp::<bool>(ack)).unwrap_or(false);
    let warn = crate::theme::fg(ui, crate::theme::WARN);
    egui::Frame::group(ui.style()).stroke(egui::Stroke::new(1.0_f32, warn)).show(ui, |ui| {
        ui.label(RichText::new("Experimental: editing story decisions").strong().color(warn));
        ui.label(
            "Changing story flags and choices can break quests, soft-lock the game or stop it from being completed. Treat it as experimental, \
             with no guarantee the save can still be finished, and use it only if you know what you're doing. Keep a backup first so you can \
             go back (Backups \u{203A} Open save manager restores it). The file as it was is also backed up automatically when you save.",
        );
        if !ok {
            ui.horizontal_wrapped(|ui| {
                if !cfg!(target_arch = "wasm32") && ui.button("Back up this save first\u{2026}").clicked() {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new(STORY_BACKUP_REQUEST), true);
                        d.insert_temp(ack, true);
                    });
                }
                if ui.button("I understand, let me edit").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(ack, true));
                }
            });
        }
    });
    ui.add_enabled_ui(ok, body);
}

/// The keys of the object at `p`, in order.
/// A filterable list of an Infinity Engine save's global variables.
pub(super) fn panel_globals(ui: &mut Ui, cx: &mut GameCtx, names: &[String]) {
    panel(ui, &format!("Global variables ({})", names.len()), |ui| story_guard(ui, |ui| {
        let q = filter_box(ui, "ie_globals", "e.g. CHAPTER, JOINED, QUEST");
        let rows: Vec<(String, String)> = names.iter().filter(|k| q.is_empty() || k.to_lowercase().contains(&q)).take(400).map(|k| (k.clone(), join("/Global variables", k))).collect();
        egui::ScrollArea::vertical().id_salt("ie_globals_scroll").max_height(320.0).show_locked(ui, |ui| value_rows(ui, cx, "ie_g", &rows));
        ui.weak("The game's own story counters: quest stages, who has joined or died, what was said. Most are 0 or 1, some count up. Changing one can skip or repeat a scene, so change few and keep the backup. Shows 400 at a time: use the filter.");
    }));
}

fn keys_of(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

pub fn gow2018(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(170, 40, 30);
    let slots = keys_of(cx, "");
    if slots.is_empty() {
        return;
    }
    let labels: Vec<String> = slots.iter().map(|s| format!("{s}: {}", cx.text(&join(&join("", s), "Saved")).unwrap_or_default())).collect();
    let t = tabs(ui, "gow_slot", &labels);
    let base = join("", &slots[t]);
    let title = cx.text(&join(&base, "Title")).unwrap_or_default();
    hero(ui, accent, "God of War", &title, |ui| {
        cx.tile(ui, "Hacksilver", &join(&base, "Hacksilver"), accent);
        cx.tile(ui, "Experience", &join(&base, "Experience"), accent);
    });
    let res = join(&base, "Resources");
    let rows: Vec<(String, String)> = keys_of(cx, &res).into_iter().map(|k| (k.clone(), join(&res, &k))).collect();
    if !rows.is_empty() {
        panel(ui, "Resources", |ui| {
            value_rows(ui, cx, "gow_res", &rows);
            super::crafting::offer(super::Game::Gow2018, ui, cx);
            ui.weak("Includes Yggdrasil's dew (the stat boosts). Resources this save hasn't found yet aren't listed.");
        });
    }
    let eq = join(&base, "Equipment owned");
    let cats = keys_of(cx, &eq);
    if !cats.is_empty() {
        panel(ui, "Equipment owned", |ui| {
            for c in &cats {
                let list: Vec<String> = cx
                    .get(&join(&eq, c))
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                egui::CollapsingHeader::new(format!("{c} ({})", list.len())).id_salt(("gow_eq", c)).show(ui, |ui| {
                    for n in list {
                        ui.label(n);
                    }
                });
            }
            ui.weak("Each upgrade level is its own entry.");
        });
    }
    gow_add_panel(
        ui,
        cx,
        &base,
        &format!("gow_add_{t}"),
        "Pieces this save doesn't have at any level. They're added at their base level when you save; upgrade them at a \
         shop in the game. Up to 32 at a time. Keep a copy of game.sav until you've loaded the save in the game.",
    );
}

// ------------------------------------------------------------------ God of War Ragnarök

pub fn gow_ragnarok(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(60, 120, 190);
    let title = cx.text("/Save/Title").unwrap_or_default();
    let mode = cx.text("/Save/Mode").unwrap_or_default();
    hero(ui, accent, "God of War Ragnar\u{00F6}k", &format!("{title}  \u{00B7}  {mode}"), |ui| {
        cx.tile(ui, "Hacksilver", "/Currency/Hacksilver", accent);
        cx.tile(ui, "Health", "/Kratos/Health", accent);
        cx.tile(ui, "Rage", "/Kratos/Rage", accent);
    });
    for (group, id, note) in [
        ("Experience", "gowr_xp", ""),
        ("Currency", "gowr_cur", ""),
        ("Yggdrasil's Dew", "gowr_dew", "Each dew adds to one of Kratos's stats."),
        ("Resources", "gowr_res", "Resources this save hasn't found yet aren't listed."),
    ] {
        let p = join("", group);
        let rows: Vec<(String, String)> = keys_of(cx, &p).into_iter().map(|k| (k.clone(), join(&p, &k))).collect();
        if rows.is_empty() {
            continue;
        }
        panel(ui, group, |ui| {
            value_rows(ui, cx, id, &rows);
            if group == "Resources" {
                super::crafting::offer(super::Game::GowRagnarok, ui, cx);
            }
            if !note.is_empty() {
                ui.weak(note);
            }
        });
    }
    let cats = keys_of(cx, "/Equipment");
    if cats.is_empty() {
        if mode == "Valhalla" {
            ui.weak("Equipment levels are edited in campaign saves.");
        }
        return;
    }
    panel(ui, "Equipment levels", |ui| {
        for c in &cats {
            let p = join("/Equipment", c);
            let rows: Vec<(String, String)> = keys_of(cx, &p).into_iter().map(|k| (k.clone(), join(&p, &k))).collect();
            egui::CollapsingHeader::new(format!("{c} ({})", rows.len())).id_salt(("gowr_eq", c)).show(ui, |ui| {
                value_rows(ui, cx, &format!("gowr_eq_{c}"), &rows);
            });
        }
        ui.weak("Each piece keeps its place, its links and what it's equipped to; only its level changes. Levels another piece of yours already has aren't offered.");
    });
    gow_add_panel(
        ui,
        cx,
        "",
        "gowr_add",
        "Pieces you don't have yet, at the level you pick, for each type you already own a piece of (that piece's record \
         is the model; it stays as it is). Added when you save, up to 32 at a time. Keep a copy of the save until you've \
         loaded it in the game.",
    );
}
