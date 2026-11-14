//! Darkest Dungeon (the estate and the roster of a profile, binary DSON read by
//! `fmt::darkest`) and Darkest Dungeon II (plain JSON: the profile, a run and its heroes).
//! One file at a time, as the games keep them: the screen shows what the open file holds.

use crate::widgets::ScrollLock;
use super::{columns, fact, fmt_num, hero, panel, words, GameCtx};
use crate::json_view::pointer_escape;
use crate::theme;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

/// Old blood and candle light.
const BLOOD: Color32 = Color32::from_rgb(140, 28, 24);
const TORCH: Color32 = Color32::from_rgb(196, 120, 40);

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

/// A thin rule with a diamond in the middle, between sections.
fn rule(ui: &mut Ui, accent: Color32) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(egui::vec2(w, 14.0), egui::Sense::hover());
    let p = ui.painter();
    let y = r.center().y;
    let stroke = egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.7));
    p.line_segment([egui::pos2(r.left() + 8.0, y), egui::pos2(r.center().x - 9.0, y)], stroke);
    p.line_segment([egui::pos2(r.center().x + 9.0, y), egui::pos2(r.right() - 8.0, y)], stroke);
    let c = r.center();
    let d = 5.0;
    p.add(egui::Shape::convex_polygon(vec![egui::pos2(c.x, c.y - d), egui::pos2(c.x + d, c.y), egui::pos2(c.x, c.y + d), egui::pos2(c.x - d, c.y)], accent, egui::Stroke::NONE));
}

fn title(ui: &mut Ui, text: &str, accent: Color32) {
    ui.label(RichText::new(text).family(theme::semibold_family()).size(17.0).color(theme::fg(ui, accent)));
}

fn object_keys(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

// ------------------------------------------------------------------ Darkest Dungeon

pub fn is_dd1(kind: &str) -> bool {
    kind == "Darkest Dungeon estate" || kind == "Darkest Dungeon roster"
}

pub fn show_dd1(ui: &mut Ui, cx: &mut GameCtx) {
    if cx.get("/base_root/wallet").is_some() {
        estate(ui, cx);
    } else {
        roster(ui, cx);
    }
}

/// What the wallet's `type` names are called in the game.
fn currency(t: &str) -> String {
    match t {
        "gold" => "Gold".into(),
        "bust" => "Busts".into(),
        "portrait" => "Portraits".into(),
        "deed" => "Deeds".into(),
        "crest" => "Crests".into(),
        "blueprint" => "Blueprints".into(),
        "shard" => "Shards".into(),
        "memory" => "Memories".into(),
        t => words(t),
    }
}

fn estate(ui: &mut Ui, cx: &mut GameCtx) {
    let wallet: Vec<(String, String)> = object_keys(cx.get("/base_root/wallet"))
        .into_iter()
        .filter_map(|k| {
            let base = join("/base_root/wallet", &k);
            let t = cx.text(&join(&base, "type"))?;
            Some((currency(&t), join(&base, "amount")))
        })
        .collect();
    hero(ui, BLOOD, "Darkest Dungeon", "The Estate \u{2014} its coffers and heirlooms", |ui| {
        for (label, p) in &wallet {
            if label == "Gold" {
                cx.tile(ui, label, p, BLOOD);
            }
        }
    });
    if wallet.iter().any(|w| w.0 != "Gold") {
        panel(ui, "Heirlooms", |ui| {
            ui.horizontal_wrapped(|ui| {
                for (label, p) in wallet.iter().filter(|w| w.0 != "Gold") {
                    cx.tile(ui, label, p, BLOOD);
                }
            });
            ui.weak("Busts, portraits, deeds and crests pay for the Hamlet's buildings; spend them in the game.");
        });
    }
    rule(ui, BLOOD);
    let lists = [("Trinkets", "/base_root/trinkets/items"), ("Estate items", "/base_root/estate_items/items")];
    columns(ui, 340.0, lists.len(), |ui, i| {
        let (name, base) = lists[i];
        let items = object_keys(cx.get(base));
        panel(ui, &format!("{name} ({})", items.len()), |ui| {
            if items.is_empty() {
                ui.weak("None.");
                return;
            }
            egui::ScrollArea::vertical().id_salt(base).max_height(320.0).show_locked(ui, |ui| {
                egui::Grid::new(("dd_items", base)).num_columns(2).spacing([16.0, 4.0]).striped(true).show(ui, |ui| {
                    for k in &items {
                        let at = join(base, k);
                        let id = cx.text(&join(&at, "id")).unwrap_or_default();
                        ui.label(words(&id));
                        let ap = join(&at, "amount");
                        if cx.can_edit(&ap) {
                            cx.compact(ui, &ap, 52.0);
                        } else {
                            let n = cx.int(&ap).unwrap_or(0);
                            ui.label(RichText::new(format!("\u{00D7}{n}")).color(ui.visuals().weak_text_color()));
                        }
                        ui.end_row();
                    }
                });
            });
        });
    });
    if cx.flag("/base_root/tampering/tampering_manager/foundLocalTamperedFile") || cx.flag("/base_root/tampering/tampering_manager/foundGlobalTamperedFile") {
        ui.colored_label(theme::fg(ui, BLOOD), "The game has marked this profile as tampered with.");
    }
}

fn roster(ui: &mut Ui, cx: &mut GameCtx) {
    let ids = object_keys(cx.get("/base_root/heroes"));
    // (pointer of the hero's own data, name, class)
    let heroes: Vec<(String, String, String)> = ids
        .iter()
        .filter_map(|id| {
            let b = format!("{}/hero_file_data/raw_data/base_root", join("/base_root/heroes", id));
            cx.get(&b)?;
            let name = cx.text(&format!("{b}/actor/name")).unwrap_or_else(|| format!("Hero {id}"));
            let class = cx.text(&format!("{b}/heroClass")).map(|c| words(&c)).unwrap_or_default();
            Some((b, name, class))
        })
        .collect();
    hero(ui, BLOOD, "Darkest Dungeon", "The Roster \u{2014} heroes of the Hamlet", |ui| {
        fact(ui, "Heroes", &heroes.len().to_string(), BLOOD);
        if let Some(x) = cx.int("/base_root/highest_resolve_xp") {
            fact(ui, "Highest resolve XP", &fmt_num(x as f64), BLOOD);
        }
    });
    if heroes.is_empty() {
        ui.weak("No heroes in this roster.");
        return;
    }
    panel(ui, "Heroes", |ui| {
        egui::ScrollArea::horizontal().id_salt("dd_roster").show_locked(ui, |ui| {
            egui::Grid::new("dd_roster_grid").num_columns(7).spacing([12.0, 5.0]).striped(true).show(ui, |ui| {
                for h in ["Hero", "Class", "Resolve XP", "Stress", "HP", "Weapon", "Armour"] {
                    ui.label(RichText::new(h).strong());
                }
                ui.end_row();
                for (b, name, class) in &heroes {
                    ui.label(RichText::new(name).color(ui.visuals().strong_text_color()));
                    ui.label(class);
                    for (k, w) in [("resolveXp", 56.0), ("m_Stress", 64.0), ("actor/current_hp", 64.0), ("weapon_rank", 40.0), ("armour_rank", 40.0)] {
                        cx.compact(ui, &format!("{b}/{k}"), w);
                    }
                    ui.end_row();
                }
            });
        });
        ui.weak("Stress 0\u{2013}200 (100 brings an affliction or virtue). Weapon and armour ranks count from 0 (a recruit's gear) to 4 (fully upgraded). HP is the value stored for the hero now. The level follows resolve XP.");
    });
    rule(ui, BLOOD);
    // One hero in detail.
    let id = egui::Id::new("dd_hero_pick");
    let mut sel = ui.data(|d| d.get_temp::<usize>(id)).unwrap_or(0).min(heroes.len() - 1);
    ui.horizontal(|ui| {
        title(ui, "Hero", BLOOD);
        egui::ComboBox::from_id_salt("dd_hero_combo").selected_text(format!("{} \u{2014} {}", heroes[sel].1, heroes[sel].2)).show_ui(ui, |ui| {
            for (i, h) in heroes.iter().enumerate() {
                ui.selectable_value(&mut sel, i, format!("{} \u{2014} {}", h.1, h.2));
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(id, sel));
    let b = heroes[sel].0.clone();
    columns(ui, 340.0, 2, |ui, col| {
        if col == 0 {
            panel(ui, "Quirks", |ui| {
                let quirks = object_keys(cx.get(&format!("{b}/quirks")));
                if quirks.is_empty() {
                    ui.weak("None.");
                }
                for q in quirks {
                    let p = join(&format!("{b}/quirks"), &q);
                    ui.horizontal(|ui| {
                        ui.label(words(&q));
                        if cx.get(&join(&p, "is_locked")).is_some() {
                            cx.chip(ui, "Locked in", &join(&p, "is_locked"), BLOOD);
                        }
                    });
                }
            });
        } else {
            panel(ui, "Skills", |ui| {
                for (label, k) in [("Combat", "selected_combat_skills"), ("Camping", "selected_camping_skills")] {
                    let base = format!("{b}/skills/{k}");
                    let skills = object_keys(cx.get(&base));
                    if skills.is_empty() {
                        continue;
                    }
                    ui.label(RichText::new(format!("{label} (upgrade level)")).strong());
                    egui::Grid::new(("dd_skills", &base)).num_columns(2).spacing([16.0, 3.0]).show(ui, |ui| {
                        for s in skills {
                            cx.grid_row(ui, &words(&s), &join(&base, &s), 44.0);
                        }
                    });
                    ui.add_space(4.0);
                }
            });
        }
    });
}

// ------------------------------------------------------------------ Darkest Dungeon II

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dd2 {
    Profile,
    Run,
    Heroes,
}

fn actor_keys(root: &Value) -> Vec<String> {
    match root {
        Value::Object(m) => m.iter().filter(|(k, v)| k.starts_with("ActorGUID") && v.get("m_ActorDataIds").is_some()).map(|(k, _)| k.clone()).collect(),
        _ => vec![],
    }
}

/// Which Darkest Dungeon II file this is, if it is one.
pub fn dd2_file(root: &Value) -> Option<Dd2> {
    let has = |p: &str| root.pointer(p).is_some();
    if has("/m_ProfileValues/m_Values") && has("/m_UnlockContainer") {
        Some(Dd2::Profile)
    } else if has("/run_values/values") && has("/inventory/m_items") {
        Some(Dd2::Run)
    } else if !actor_keys(root).is_empty() {
        Some(Dd2::Heroes)
    } else {
        None
    }
}

pub fn show_dd2(ui: &mut Ui, cx: &mut GameCtx) {
    match dd2_file(cx.root) {
        Some(Dd2::Profile) => dd2_profile(ui, cx),
        Some(Dd2::Run) => dd2_run(ui, cx),
        Some(Dd2::Heroes) => dd2_heroes(ui, cx),
        None => {}
    }
    ui.weak("Read from saves made before the Kingdoms update; Kingdoms keeps its own files, which aren't covered.");
}

/// Pointer of a `[name, value]` pair's value in a list of pairs.
fn pair(cx: &GameCtx, list: &str, name: &str) -> Option<String> {
    let a = cx.get(list)?.as_array()?;
    let i = a.iter().position(|e| e.get(0).and_then(Value::as_str) == Some(name))?;
    Some(format!("{list}/{i}/1"))
}

fn dd2_profile(ui: &mut Ui, cx: &mut GameCtx) {
    let name = cx.text("/m_Name").unwrap_or_default();
    let candles = pair(cx, "/m_ProfileValues/m_Values", "candles");
    hero(ui, TORCH, "Darkest Dungeon II", &format!("Profile \u{2014} {name}"), |ui| {
        if let Some(p) = &candles {
            cx.tile(ui, "Candles of Hope", p, TORCH);
        }
        if let Some(n) = cx.int("/m_HopeCollected") {
            fact(ui, "Hope collected", &fmt_num(n as f64), TORCH);
        }
        if let Some(n) = cx.num("/m_TotalDistanceDriven") {
            fact(ui, "Distance driven", &fmt_num(n.round()), TORCH);
        }
    });
    if candles.is_none() {
        ui.weak("This profile has no candles yet.");
    }
    rule(ui, TORCH);
    columns(ui, 340.0, 2, |ui, col| {
        if col == 0 {
            let unlocks: Vec<String> = cx.get("/m_UnlockContainer/m_Unlocks").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(words).collect()).unwrap_or_default();
            panel(ui, &format!("Unlocked at the Altar of Hope ({})", unlocks.len()), |ui| {
                egui::ScrollArea::vertical().id_salt("dd2_unlocks").max_height(300.0).show_locked(ui, |ui| {
                    for u in &unlocks {
                        ui.label(u);
                    }
                });
            });
        } else {
            panel(ui, "Confessions", |ui| {
                egui::Grid::new("dd2_bosses").num_columns(3).spacing([14.0, 4.0]).show(ui, |ui| {
                    ui.label(RichText::new("Chapter").strong());
                    ui.label(RichText::new("Chosen").strong());
                    ui.label(RichText::new("Won").strong());
                    ui.end_row();
                    let won: Vec<Value> = cx.get("/m_BossVictoryCounts").and_then(Value::as_array).cloned().unwrap_or_default();
                    for e in cx.get("/m_BossSelectCounts").and_then(Value::as_array).cloned().unwrap_or_default() {
                        let boss = e.get(0).and_then(Value::as_str).unwrap_or_default().to_string();
                        let n = e.get(1).map(crate::widgets::display).unwrap_or_default();
                        let w = won.iter().find(|x| x.get(0).and_then(Value::as_str) == Some(&boss)).and_then(|x| x.get(1)).map(crate::widgets::display).unwrap_or_else(|| "0".into());
                        ui.label(words(&boss));
                        ui.label(n);
                        ui.label(w);
                        ui.end_row();
                    }
                });
            });
        }
    });
}

fn dd2_run(ui: &mut Ui, cx: &mut GameCtx) {
    let items: Vec<(usize, String, String)> = cx
        .get("/inventory/m_items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .enumerate()
                .filter_map(|(i, it)| {
                    let def = it.get("def")?.as_str()?.to_string();
                    let name = it.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| words(&def));
                    Some((i, def, name))
                })
                .collect()
        })
        .unwrap_or_default();
    let relics = items.iter().find(|i| i.1 == "gold").map(|i| format!("/inventory/m_items/{}/qty", i.0));
    hero(ui, TORCH, "Darkest Dungeon II", "The Expedition \u{2014} this run", |ui| {
        if let Some(p) = &relics {
            cx.tile(ui, "Relics", p, TORCH);
        }
        cx.tile(ui, "Torch", "/run_values/values/torch", TORCH);
        for (label, k) in [("Stagecoach armour", "stage_coach_armor"), ("Stagecoach wheels", "stage_coach_wheels")] {
            if let Some(n) = cx.num(&format!("/run_values/values/{k}")) {
                fact(ui, label, &fmt_num(n), TORCH);
            }
        }
    });
    rule(ui, TORCH);
    panel(ui, &format!("Inventory ({} of {} slots)", items.len(), cx.get("/inventory/m_items").and_then(Value::as_array).map_or(0, Vec::len)), |ui| {
        if items.is_empty() {
            ui.weak("Empty.");
        }
        columns(ui, 300.0, items.len(), |ui, n| {
            let (i, _, name) = &items[n];
            cx.row_fixed(ui, name, &format!("/inventory/m_items/{i}/qty"), 150.0, 70.0);
        });
        ui.weak("Relics are the run's money. Torch goes up to 100.");
    });
}

fn dd2_heroes(ui: &mut Ui, cx: &mut GameCtx) {
    let actors = actor_keys(cx.root);
    hero(ui, TORCH, "Darkest Dungeon II", "The Heroes of this run", |ui| {
        fact(ui, "Heroes", &actors.len().to_string(), TORCH);
    });
    columns(ui, 330.0, actors.len(), |ui, n| {
        let b = join("", &actors[n]);
        let name = cx.text(&format!("{b}/m_ActorName")).unwrap_or_else(|| actors[n].clone());
        let class = cx.get(&format!("{b}/m_ActorDataIds")).and_then(|v| v.get(0)).and_then(Value::as_str).map(words).unwrap_or_default();
        let mut tags = vec![class];
        if cx.flag(&format!("{b}/m_IsInParty")) {
            tags.push("In the party".into());
        }
        if cx.get(&format!("{b}/m_IsLiving")).is_some() && !cx.flag(&format!("{b}/m_IsLiving")) {
            tags.push("Dead".into());
        }
        panel(ui, &name, |ui| {
            ui.label(RichText::new(tags.join("  \u{00B7}  ")).color(theme::fg(ui, TORCH)));
            cx.row_fixed(ui, "HP", &format!("{b}/m_Hp"), 70.0, 80.0);
            cx.row_fixed(ui, "Stress", &format!("{b}/m_Stress"), 70.0, 80.0);
            let quirks: Vec<String> = cx.get(&format!("{b}/QuirkContainer")).and_then(Value::as_array).map(|a| a.iter().filter_map(|q| q.get("m_Definition")?.as_str()).map(|q| words(q.trim_start_matches("quirk_"))).collect()).unwrap_or_default();
            if !quirks.is_empty() {
                ui.label(RichText::new("Quirks").strong());
                ui.label(quirks.join(", "));
            }
            let skills: Vec<String> = cx
                .get(&format!("{b}/m_CombatSkills"))
                .and_then(Value::as_array)
                .map(|a| a.iter().filter(|s| s.get("m_IsEquipped").and_then(Value::as_bool) == Some(true)).filter_map(|s| s.get("m_SkillId")?.as_str()).map(words).collect())
                .unwrap_or_default();
            if !skills.is_empty() {
                ui.label(RichText::new("Equipped skills").strong());
                ui.label(skills.join(", "));
            }
        });
    });
    ui.weak("HP and stress as they are now (stress goes up to 10). Max HP comes from the hero's path and level.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tells_darkest_dungeon_2_files_apart() {
        assert_eq!(dd2_file(&json!({"m_ProfileValues": {"m_Values": [["candles", 5.0]]}, "m_UnlockContainer": {}})), Some(Dd2::Profile));
        assert_eq!(dd2_file(&json!({"run_values": {"values": {"torch": 50.0}}, "inventory": {"m_items": [null]}})), Some(Dd2::Run));
        assert_eq!(dd2_file(&json!({"ActorGUID3": {"m_ActorDataIds": ["x"], "m_Hp": 20.0}})), Some(Dd2::Heroes));
        assert_eq!(dd2_file(&json!({"ActorGUID3": 1})), None);
        assert_eq!(currency("crest"), "Crests");
    }

    /// Research aid: every .json under SAVE_EDITOR_DD2 (a profiles folder) that is a profile,
    /// run or heroes file opens as JSON on this screen, is written back the same (bar line
    /// endings), and takes an edit with nothing else changing.
    #[test]
    #[ignore]
    fn real_dd2_files() {
        use crate::formats::{decode_with, Content, Options};
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "json") {
                    out.push(p);
                }
            }
        }
        let mut files = vec![];
        walk(std::path::Path::new(&std::env::var("SAVE_EDITOR_DD2").unwrap()), &mut files);
        let mut seen = [0; 3];
        for path in files {
            let raw = std::fs::read(&path).unwrap();
            let d = decode_with(&raw, &Options::for_path(&path));
            let Content::Json { value, pretty } = &d.content else { continue };
            let Some(kind) = dd2_file(value) else { continue };
            assert_eq!(crate::game_ui::detect("JSON", value), Some(crate::game_ui::Game::DarkestDungeon2));
            let plain: Vec<u8> = raw.iter().copied().filter(|&b| b != b'\r').collect();
            assert_eq!(String::from_utf8(d.content.to_bytes().unwrap()).unwrap().trim_end(), String::from_utf8(plain).unwrap().trim_end(), "{}", path.display());
            let p = match kind {
                Dd2::Profile => {
                    seen[0] += 1;
                    let cx = GameCtx { root: value, baseline: value, tree: None, edits: vec![], archive: None, open_entry: None };
                    pair(&cx, "/m_ProfileValues/m_Values", "candles")
                }
                Dd2::Run => {
                    seen[1] += 1;
                    let items = value.pointer("/inventory/m_items").and_then(Value::as_array).unwrap();
                    items.iter().position(|i| i.get("def").and_then(Value::as_str) == Some("gold")).map(|i| format!("/inventory/m_items/{i}/qty"))
                }
                Dd2::Heroes => {
                    seen[2] += 1;
                    Some(format!("/{}/m_Stress", actor_keys(value)[0]))
                }
            };
            let Some(p) = p else { continue };
            let n = value.pointer(&p).and_then(Value::as_f64).unwrap();
            let mut edited = value.clone();
            *edited.pointer_mut(&p).unwrap() = serde_json::json!(n + 1.0);
            let bytes = Content::Json { value: edited, pretty: *pretty }.to_bytes().unwrap();
            let mut again: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(again.pointer(&p).and_then(Value::as_f64), Some(n + 1.0));
            *again.pointer_mut(&p).unwrap() = value.pointer(&p).unwrap().clone();
            assert_eq!(&again, value);
            println!("{}: {kind:?}, {p} {n} -> {}", path.display(), n + 1.0);
        }
        println!("profiles {}, runs {}, heroes {}", seen[0], seen[1], seen[2]);
        assert!(seen.iter().all(|&n| n > 0));
    }
}
