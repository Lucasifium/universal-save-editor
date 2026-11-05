//! Screens for Green Hell, RE Engine game slots, Hogwarts Legacy, Ren'Py, Europa
//! Universalis IV and Crusader Kings II. As elsewhere, values are found by name and
//! simply left out when a save doesn't have them.

use super::{columns, fact, hero, panel, tabs, words, GameCtx};
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
fn filter_box(ui: &mut Ui, id: &str, hint: &str) -> String {
    let sid = egui::Id::new(("screen_filter", id));
    let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add(egui::TextEdit::singleline(&mut q).hint_text(hint).desired_width(220.0));
    });
    ui.data_mut(|d| d.insert_temp(sid, q.clone()));
    q.to_lowercase()
}

/// Label + editor rows for the given (label, pointer) pairs, spread over columns.
fn value_rows(ui: &mut Ui, cx: &mut GameCtx, id: &str, rows: &[(String, String)]) {
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
                    ui.strong("Item");
                    ui.strong("Count");
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
    let vars: Vec<(String, Value)> = match cx.get("/Variables") {
        Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        _ => vec![],
    };
    let numbers: Vec<&String> = vars.iter().filter(|(_, v)| v.is_number()).map(|(k, _)| k).collect();
    let flags: Vec<&String> = vars.iter().filter(|(_, v)| v.is_boolean()).map(|(k, _)| k).collect();
    let texts: Vec<&String> = vars.iter().filter(|(_, v)| v.as_str().is_some_and(|s| !s.starts_with('<'))).map(|(k, _)| k).collect();
    let sub = format!("{} numbers  \u{00B7}  {} flags{}", numbers.len(), flags.len(), if version.is_empty() { String::new() } else { format!("  \u{00B7}  saved with game version {version}") });
    hero(ui, accent, "Story variables", &sub, |ui| {
        // Money and the like, if the game calls them something common.
        for (k, _) in &vars {
            let l = k.to_lowercase();
            if ["money", "gold", "cash", "coins", "credits", "affection", "love", "points"].iter().any(|w| l == *w || l.ends_with(&format!("_{w}")) || l.ends_with(&format!(".{w}"))) {
                cx.tile(ui, &words(k), &join("/Variables", k), accent);
            }
        }
    });
    let q = filter_box(ui, "renpy", "e.g. money, love, day");
    let keep = |k: &str| q.is_empty() || k.to_lowercase().contains(&q);
    panel(ui, "Numbers", |ui| {
        let rows: Vec<(String, String)> = numbers.iter().filter(|k| keep(k)).map(|k| ((*k).clone(), join("/Variables", k))).collect();
        if rows.is_empty() {
            ui.weak("None match.");
        }
        egui::ScrollArea::vertical().id_salt("renpy_num").max_height(360.0).show(ui, |ui| value_rows(ui, cx, "renpy_n", &rows));
    });
    panel(ui, "Flags", |ui| {
        let shown: Vec<&&String> = flags.iter().filter(|k| keep(k)).collect();
        if shown.is_empty() {
            ui.weak("None match.");
        }
        egui::ScrollArea::vertical().id_salt("renpy_flags").max_height(260.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for k in shown {
                    cx.chip(ui, k, &join("/Variables", k), accent);
                }
            });
        });
    });
    panel(ui, "Text", |ui| {
        let rows: Vec<(String, String)> = texts.iter().filter(|k| keep(k)).map(|k| ((*k).clone(), join("/Variables", k))).collect();
        if rows.is_empty() {
            ui.weak("None match.");
        }
        egui::ScrollArea::vertical().id_salt("renpy_txt").max_height(220.0).show(ui, |ui| value_rows(ui, cx, "renpy_t", &rows));
    });
    ui.weak("These are the game's own variable names. Each save keeps its own copy; the Values tab also shows lists and objects.");
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
        egui::ScrollArea::vertical().id_salt("eu4_c").max_height(360.0).show(ui, |ui| value_rows(ui, cx, "eu4_rows", &rows));
    });
    ui.weak("Stability runs \u{2212}3 to 3; prestige and legitimacy up to 100. Ironman saves are binary and can't be edited.");
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
    });
    panel(ui, "Resources", |ui| {
        let rows: Vec<(String, String)> = match cx.get("/Resources") {
            Some(Value::Object(m)) => m.keys().map(|k| (k.clone(), join("/Resources", k))).collect(),
            _ => vec![],
        };
        value_rows(ui, cx, "me_res", &rows);
        ui.weak("Element zero, iridium, palladium and platinum are Mass Effect 2's minerals; Mass Effect 3 keeps them at 0.");
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
        });
    }
    ui.weak("Power ranks run 0\u{2013}4 in Mass Effect 2 and 0\u{2013}6 in Mass Effect 3; an evolved power shows up as its own entry.");
}

fn power_list(ui: &mut Ui, cx: &mut GameCtx, base: &str, id: &str, accent: Color32) {
    let names: Vec<String> = match cx.get(base) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => return,
    };
    if names.is_empty() {
        return;
    }
    let max = if cx.text("/Save/Game").is_some_and(|g| g.contains('3')) { 6.0 } else { 4.0 };
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
            egui::ScrollArea::both().id_salt("xcom_s").max_height(420.0).show(ui, |ui| {
                egui::Grid::new("xcom_grid").num_columns(9).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    for h in ["Soldier", "Class", "Rank", "XP", "Kills", "HP", "Aim", "Mobility", "Will"] {
                        ui.strong(h);
                    }
                    ui.end_row();
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
                        cx.compact(ui, &format!("{sp}/iXP"), 60.0);
                        cx.compact(ui, &format!("{sp}/iNumKills"), 44.0);
                        for i in [0, 1, 3, 7] {
                            if cx.get(&stat(base, c, i)).is_some() {
                                cx.compact(ui, &stat(base, c, i), 44.0);
                            } else {
                                ui.label("");
                            }
                        }
                        ui.end_row();
                    }
                });
            });
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
        egui::Grid::new(("ff7_hp", t, c)).num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.label("");
            ui.strong("Now");
            ui.strong("Max");
            ui.end_row();
            for (n, cur, max) in [("HP", "HP", "Max HP"), ("MP", "MP", "Max MP")] {
                ui.label(n);
                cx.cell(ui, &at(cur), 70.0);
                cx.cell(ui, &at(max), 70.0);
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Stats").strong());
        for k in ["Strength", "Vitality", "Magic", "Spirit", "Dexterity", "Luck"] {
            cx.attribute(ui, k, &at(k), 255.0, accent);
        }
        ui.weak("Stats go up to 255. The game recalculates max HP/MP from level and equipment, so change base max HP/MP in Values to make it stick.");
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
    hero(ui, accent, &game, &format!("{} party members in this slot", members.len()), |_| {});
    panel(ui, "Party", |ui| {
        egui::ScrollArea::horizontal().id_salt("ff7r_scroll").show(ui, |ui| {
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
        egui::ScrollArea::horizontal().id_salt("ff9_scroll").show(ui, |ui| {
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
                ui.strong("Level");
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
            egui::ScrollArea::vertical().id_salt("dl2_sk_s").max_height(320.0).show(ui, |ui| value_rows(ui, cx, "dl2_skills", &rows));
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
    ui.weak("Characters' stats are in the platoon files next to this save; open one of those with Open\u{2026} and use Value search.");
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
        egui::ScrollArea::horizontal().id_salt("ffx2_scroll").show(ui, |ui| {
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
        egui::ScrollArea::horizontal().id_salt((id, "s")).show(ui, |ui| {
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
    for group in ["Characters", "Aeons"] {
        let who: Vec<String> = match cx.get(&format!("/{group}")) {
            Some(Value::Object(m)) => m.keys().cloned().collect(),
            _ => continue,
        };
        panel(ui, group, |ui| {
            egui::ScrollArea::horizontal().id_salt(("ffx_s", group)).show(ui, |ui| {
                egui::Grid::new(("ffx_g", group)).striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                    ui.label("");
                    for h in COLS {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for w in &who {
                        let p = join(&join("", group), w);
                        ui.label(w);
                        for k in COLS {
                            cx.compact(ui, &join(&p, k), if k.contains("HP") { 72.0 } else { 46.0 });
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }
    ui.weak("Stats are as the Sphere Grid has raised them (255 is the most). HP above 9999 needs Break HP Limit.");
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
        ui.weak("What Artyom carries at the start of this level (chapter select loads it). The game may cap a count at what Artyom can carry.");
    });
}
