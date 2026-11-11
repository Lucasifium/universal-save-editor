//! Hollow Knight: Silksong, Undertale, Deltarune and Vampire Survivors. As elsewhere,
//! values are found by name and simply left out when a save doesn't have them.

use super::{bar, columns, fact, fmt_num, hero, meter, panel, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

/// Label + editor rows in a two-column grid (rows the save doesn't have are left out).
fn rows(ui: &mut Ui, cx: &mut GameCtx, id: &str, list: &[(&str, String)], width: f32) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([14.0, 5.0])
        .show(ui, |ui| {
            for (label, p) in list {
                cx.grid_row(ui, label, p, width);
            }
        });
}

/// A filter box remembered under `id`; returns the lowercase query.
fn filter(ui: &mut Ui, id: &str) -> String {
    let sid = egui::Id::new(("indie_filter", id));
    let mut q = ui.data(|d| d.get_temp::<String>(sid)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.add(egui::TextEdit::singleline(&mut q).desired_width(200.0));
    });
    ui.data_mut(|d| d.insert_temp(sid, q.clone()));
    q.to_lowercase()
}

fn hours(frames_or_secs: f64, per_sec: f64) -> String {
    let s = (frames_or_secs / per_sec).max(0.0) as u64;
    format!("{}:{:02}", s / 3600, s / 60 % 60)
}

// ------------------------------------------------------------------ Hollow Knight: Silksong

/// Silksong keeps Hollow Knight's `playerData` (rosaries are still `geo`), plus silk and shell shards.
pub fn is_silksong(root: &Value) -> bool {
    root.pointer("/playerData/silkMax").is_some()
        || root.pointer("/playerData/ShellShards").is_some()
}

const SS_ABILITIES: [(&str, &str); 9] = [
    ("Swift Step", "hasDash"),
    ("Cling Grip", "hasWalljump"),
    ("Drifter's Cloak", "hasBrolly"),
    ("Needolin", "hasNeedolin"),
    ("Clawline", "hasHarpoonDash"),
    ("Faydown Cloak", "hasDoubleJump"),
    ("Needle Strike", "hasChargeSlash"),
    ("Silk Soar", "hasSuperJump"),
    ("Sylphsong", "HasBoundCrestUpgrader"),
];

/// Crests whose name in the game differs from the save's id.
fn crest_name(id: &str) -> String {
    match id {
        "Warrior" => "Beast".into(),
        "Toolmaster" => "Architect".into(),
        "Spell" => "Shaman".into(),
        _ => id.replace('_', " "),
    }
}

/// `(index, Name)` of the entries of a `{savedData: [{Name, Data}]}` list.
fn saved(cx: &GameCtx, base: &str) -> Vec<(usize, String)> {
    match cx.get(&format!("{base}/savedData")) {
        Some(Value::Array(a)) => a
            .iter()
            .enumerate()
            .filter_map(|(i, e)| Some((i, e.get("Name")?.as_str()?.to_string())))
            .collect(),
        _ => vec![],
    }
}

pub fn silksong(ui: &mut Ui, cx: &mut GameCtx) {
    let p = "/playerData";
    let accent = Color32::from_rgb(200, 40, 62);
    let silk = Color32::from_rgb(232, 228, 240);
    let k = |key: &str| join(p, key);
    let mut sub = vec![];
    if let Some(c) = cx.num(&k("completionPercentage")) {
        sub.push(format!("{c}% complete"));
    }
    if let Some(t) = cx.num(&k("playTime")) {
        sub.push(format!("{} played", hours(t, 1.0)));
    }
    if cx.int(&k("permadeathMode")).is_some_and(|m| m > 0) {
        sub.push("Steel Soul".into());
    }
    if let Some(v) = cx.text(&k("version")) {
        sub.push(format!("version {v}"));
    }
    hero(ui, accent, "Hornet", &sub.join("  \u{00B7}  "), |ui| {
        for (cap, key) in [
            ("Rosaries", "geo"),
            ("Shell Shards", "ShellShards"),
            ("Masks", "maxHealthBase"),
            ("Silk spool", "silkMax"),
            ("Needle upgrades", "nailUpgrades"),
            ("Silk hearts", "silkRegenMax"),
            ("Tool pouch upgrades", "ToolPouchUpgrades"),
            ("Crafting kit upgrades", "ToolKitUpgrades"),
        ] {
            cx.tile(ui, cap, &k(key), accent);
        }
    });
    if let (Some(h), Some(m)) = (cx.num(&k("health")), cx.num(&k("maxHealth"))) {
        meter(ui, "Masks", h, m, accent);
    }
    if let (Some(s), Some(m)) = (cx.num(&k("silk")), cx.num(&k("silkMax"))) {
        meter(ui, "Silk", s, m, silk.gamma_multiply(0.8));
    }
    ui.add_space(8.0);
    columns(ui, 340.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Health and silk", |ui| {
                rows(
                    ui,
                    cx,
                    "ss_health",
                    &[
                        ("Health", k("health")),
                        ("Max health", k("maxHealth")),
                        ("Base masks", k("maxHealthBase")),
                        ("Mask shards", k("heartPieces")),
                        ("Silk", k("silk")),
                        ("Silk spool", k("silkMax")),
                        ("Spool fragments", k("silkSpoolParts")),
                        ("Silk hearts", k("silkRegenMax")),
                        ("Rosaries on your cocoon", k("HeroCorpseMoneyPool")),
                    ],
                    110.0,
                );
                ui.weak("Masks: change Max health and Base masks together.");
            });
        } else {
            panel(ui, "Abilities", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (label, key) in SS_ABILITIES {
                        cx.chip(ui, label, &k(key), accent);
                    }
                });
            });
            let crests = saved(cx, &k("ToolEquips"));
            if !crests.is_empty() {
                panel(ui, "Crests", |ui| {
                    let cur_p = k("CurrentCrestID");
                    if let Some(cur) = cx.text(&cur_p) {
                        ui.horizontal(|ui| {
                            cx.label(ui, "Equipped crest", &cur_p);
                            let mut pick = None;
                            egui::ComboBox::from_id_salt("ss_crest")
                                .selected_text(crest_name(&cur))
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    for (_, id) in &crests {
                                        if ui.selectable_label(*id == cur, crest_name(id)).clicked()
                                        {
                                            pick = Some(id.clone());
                                        }
                                    }
                                });
                            if let Some(id) = pick.filter(|id| *id != cur && cx.can_edit(&cur_p)) {
                                cx.set(&cur_p, Value::String(id));
                            }
                        });
                    }
                    ui.weak("Crests you have:");
                    ui.horizontal_wrapped(|ui| {
                        for (i, id) in &crests {
                            cx.chip(
                                ui,
                                &crest_name(id),
                                &format!("{}/savedData/{i}/Data/IsUnlocked", k("ToolEquips")),
                                accent,
                            );
                        }
                    });
                });
            }
        }
    });
    let tools = saved(cx, &k("Tools"));
    if !tools.is_empty() {
        panel(ui, &format!("Tools ({})", tools.len()), |ui| {
            let q = filter(ui, "ss_tools");
            let shown: Vec<&(usize, String)> = tools
                .iter()
                .filter(|(_, n)| q.is_empty() || n.to_lowercase().contains(&q))
                .collect();
            let base = format!("{}/savedData", k("Tools"));
            columns(ui, 300.0, shown.len(), |ui, j| {
                let (i, name) = shown[j];
                ui.horizontal(|ui| {
                    cx.chip(ui, name, &format!("{base}/{i}/Data/IsUnlocked"), accent);
                    let amount = format!("{base}/{i}/Data/AmountLeft");
                    if cx.int(&amount).is_some_and(|n| n > 0) || cx.modified(&amount) {
                        ui.weak("\u{00D7}");
                        cx.compact(ui, &amount, 48.0);
                    }
                });
            });
            ui.weak("Lit = you have the tool. The number is how many uses it has left (for tools that run out).");
        });
    }
    let liquids = saved(cx, &k("ToolLiquids"));
    let items = saved(cx, &k("Collectables"));
    columns(ui, 340.0, 2, |ui, i| {
        if i == 0 && !liquids.is_empty() {
            panel(ui, "Refills", |ui| {
                let list: Vec<(&str, String)> = liquids
                    .iter()
                    .map(|(j, n)| {
                        (
                            n.as_str(),
                            format!("{}/savedData/{j}/Data/RefillsLeft", k("ToolLiquids")),
                        )
                    })
                    .collect();
                rows(ui, cx, "ss_liquids", &list, 90.0);
            });
            panel(ui, "Where you are", |ui| {
                rows(
                    ui,
                    cx,
                    "ss_where",
                    &[("Bench scene", k("respawnScene"))],
                    140.0,
                );
            });
        }
        if i == 1 && !items.is_empty() {
            panel(ui, "Items", |ui| {
                let q = filter(ui, "ss_items");
                let names: Vec<(String, String)> = items
                    .iter()
                    .filter(|(_, n)| q.is_empty() || n.to_lowercase().contains(&q))
                    .map(|(j, n)| {
                        (
                            n.replace('_', " "),
                            format!("{}/savedData/{j}/Data/Amount", k("Collectables")),
                        )
                    })
                    .collect();
                egui::ScrollArea::vertical()
                    .id_salt("ss_items_scroll")
                    .max_height(320.0)
                    .show(ui, |ui| {
                        let list: Vec<(&str, String)> =
                            names.iter().map(|(n, p)| (n.as_str(), p.clone())).collect();
                        rows(ui, cx, "ss_items", &list, 90.0);
                    });
            });
        }
    });
}

// ------------------------------------------------------------------ Undertale

const UT_YELLOW: Color32 = Color32::from_rgb(255, 222, 0);

/// A black box with a thick white border, like the game's own menus.
fn ut_box(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(Color32::BLACK)
        .stroke(egui::Stroke::new(4.0_f32, Color32::WHITE))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        });
    ui.add_space(10.0);
}

fn mono(t: impl Into<String>, size: f32, c: Color32) -> RichText {
    RichText::new(t).monospace().size(size).color(c)
}

pub fn undertale(ui: &mut Ui, cx: &mut GameCtx) {
    let c = "/Character";
    let k = |key: &str| join(c, key);
    let name = cx
        .text(&k("Name"))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Human".into());
    let lv = cx.int(&k("LV")).unwrap_or(1);
    let max_hp = cx.num(&k("Max HP")).unwrap_or(20.0);
    ut_box(ui, |ui| {
        ui.label(mono(format!("{name}   LV {lv}"), 26.0, Color32::WHITE));
        ui.horizontal(|ui| {
            ui.label(mono("HP", 16.0, Color32::WHITE));
            let w = (ui.available_width() - 8.0).clamp(80.0, 260.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 20.0), egui::Sense::hover());
            // Full: the game refills HP when a save is loaded.
            ui.painter().rect_filled(rect, 0.0, UT_YELLOW);
            ui.label(mono(
                format!("{} / {}", fmt_num(max_hp), fmt_num(max_hp)),
                16.0,
                Color32::WHITE,
            ));
        });
        let mut line = vec![];
        for (cap, key) in [("G", "Gold"), ("EXP", "EXP"), ("KILLS", "Kills")] {
            if let Some(n) = cx.num(&k(key)) {
                line.push(format!("{cap} {}", fmt_num(n)));
            }
        }
        if let Some(t) = cx.num("/Progress/Play time (frames)") {
            line.push(format!("TIME {}", hours(t, 30.0)));
        }
        ui.label(mono(line.join("   "), 16.0, UT_YELLOW));
    });
    columns(ui, 330.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Stats", |ui| {
                let list: Vec<(&str, String)> = [
                    "Name",
                    "LV",
                    "Max HP",
                    "AT",
                    "Weapon AT",
                    "DF",
                    "Armor DF",
                    "Speed",
                    "EXP",
                    "Gold",
                    "Kills",
                ]
                .iter()
                .map(|n| (*n, k(n)))
                .collect();
                rows(ui, cx, "ut_stats", &list, 120.0);
                if (1..=19).contains(&lv) && cx.can_edit(&k("Max HP")) && cx.can_edit(&k("AT")) {
                    let (hp, at) = (16 + 4 * lv, 10 + 2 * (lv - 1));
                    let tip = "Max HP 16 + 4 \u{00D7} LV and AT 10 + 2 \u{00D7} (LV \u{2212} 1), as the game sets them when you level up (LV 1 to 19).";
                    if ui
                        .button(format!("Match Max HP and AT to LV {lv} ({hp} HP, {at} AT)"))
                        .on_hover_text(tip)
                        .clicked()
                    {
                        cx.set(&k("Max HP"), Value::from(hp));
                        cx.set(&k("AT"), Value::from(at));
                    }
                }
                ui.weak("AT and DF here are your own; Weapon AT and Armor DF come from what you have equipped.");
            });
        } else {
            panel(ui, "Equipment", |ui| {
                rows(
                    ui,
                    cx,
                    "ut_equip",
                    &[
                        ("Weapon", "/Equipment/Weapon".into()),
                        ("Armor", "/Equipment/Armor".into()),
                    ],
                    170.0,
                );
            });
            panel(ui, "Items", |ui| {
                let list: Vec<(String, String)> = (1..=8)
                    .map(|n| (format!("{n}"), format!("/Items/Slot {n}")))
                    .collect();
                let list: Vec<(&str, String)> =
                    list.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
                rows(ui, cx, "ut_items", &list, 170.0);
            });
        }
    });
    columns(ui, 330.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Phone", |ui| {
                let list: Vec<(String, String)> = (1..=8)
                    .map(|n| (format!("{n}"), format!("/Phone/Slot {n}")))
                    .collect();
                let list: Vec<(&str, String)> =
                    list.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
                rows(ui, cx, "ut_phone", &list, 170.0);
            });
        } else {
            panel(ui, "Progress", |ui| {
                rows(
                    ui,
                    cx,
                    "ut_progress",
                    &[
                        ("Room", "/Progress/Room".into()),
                        ("Play time (frames)", "/Progress/Play time (frames)".into()),
                    ],
                    120.0,
                );
                ui.weak("Play time counts frames (30 a second). The story flags are under Other lines (Advanced).");
            });
        }
    });
    ui.weak("The load menu shows the name, LV and time from undertale.ini in the same folder; the game itself loads file0.");
}

// ------------------------------------------------------------------ Deltarune

pub fn deltarune(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(60, 110, 235);
    let ch1 = cx.get("/Inventory").is_some();
    let name = cx
        .text("/Player/Name")
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Vessel".into());
    let mut sub = vec![if ch1 {
        "Chapter 1".to_string()
    } else {
        "Chapter 2, 3 or 4".to_string()
    }];
    if let Some(t) = cx.num("/Progress/Play time (frames)") {
        sub.push(format!("{} played", hours(t, 30.0)));
    }
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Dark Dollars", "/Player/Dark Dollars", accent);
        if ch1 {
            cx.tile(ui, "TP", "/Tension/TP", accent);
        }
    });
    panel(ui, "Party", |ui| {
        ui.horizontal_wrapped(|ui| {
            for n in 1..=3 {
                let p = format!("/Player/Party member {n}");
                cx.label(ui, &format!("{n}."), &p);
                cx.editor(ui, &p, 110.0);
                ui.add_space(10.0);
            }
        });
    });
    let members: Vec<String> = match cx.get("/Party") {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    };
    columns(ui, 300.0, members.len(), |ui, i| {
        let base = join("/Party", &members[i]);
        let k = |key: &str| join(&base, key);
        panel(ui, &members[i], |ui| {
            if let (Some(h), Some(m)) = (cx.num(&k("HP")), cx.num(&k("Max HP"))) {
                meter(ui, "HP", h, m, Color32::from_rgb(230, 190, 40));
                ui.add_space(4.0);
            }
            for (label, key) in [
                ("Attack", "Attack"),
                ("Defense", "Defense"),
                ("Magic", "Magic"),
                ("Guts", "Guts"),
            ] {
                cx.attribute(ui, label, &k(key), 30.0, accent);
            }
            ui.add_space(4.0);
            let list: Vec<(&str, String)> = ["HP", "Max HP", "Weapon", "Armor 1", "Armor 2"]
                .iter()
                .map(|n| (*n, k(n)))
                .collect();
            rows(ui, cx, &format!("dr_{i}"), &list, 90.0);
        });
    });
    if ch1 {
        panel(ui, "Inventory", |ui| {
            let kinds = ["Items", "Key items", "Weapons", "Armor"];
            egui::Grid::new("dr_inv")
                .num_columns(5)
                .striped(true)
                .spacing([10.0, 3.0])
                .show(ui, |ui| {
                    ui.label("");
                    for kind in kinds {
                        ui.strong(kind);
                    }
                    ui.end_row();
                    for n in 1..=13 {
                        ui.weak(n.to_string());
                        for kind in kinds {
                            cx.compact(ui, &format!("/Inventory/{kind}/Slot {n}"), 56.0);
                        }
                        ui.end_row();
                    }
                });
            ui.weak("Game item numbers; 0 is an empty slot.");
        });
        columns(ui, 330.0, 2, |ui, i| {
            if i == 0 {
                panel(ui, "Light World", |ui| {
                    let list: Vec<(&str, String)> = [
                        "LV", "EXP", "Money", "HP", "Max HP", "AT", "DF", "Weapon", "Armor",
                    ]
                    .iter()
                    .map(|n| (*n, join("/Light World", n)))
                    .collect();
                    rows(ui, cx, "dr_light", &list, 100.0);
                });
            } else {
                panel(ui, "Light World items", |ui| {
                    let list: Vec<(String, String)> = (1..=8)
                        .map(|n| (format!("{n}"), format!("/Light World items/Slot {n}")))
                        .collect();
                    let list: Vec<(&str, String)> =
                        list.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
                    rows(ui, cx, "dr_light_items", &list, 100.0);
                });
                panel(ui, "Tension", |ui| {
                    rows(
                        ui,
                        cx,
                        "dr_tp",
                        &[
                            ("TP", "/Tension/TP".into()),
                            ("Max TP", "/Tension/Max TP".into()),
                        ],
                        100.0,
                    );
                });
            }
        });
    }
    panel(ui, "Progress", |ui| {
        rows(
            ui,
            cx,
            "dr_progress",
            &[
                ("Plot", "/Progress/Plot".into()),
                ("Room", "/Progress/Room".into()),
                ("Play time (frames)", "/Progress/Play time (frames)".into()),
            ],
            120.0,
        );
    });
    if !ch1 {
        ui.weak("Chapters 2\u{2013}4: the inventory's place in the file isn't mapped yet; every line is under Other lines (Advanced).");
    }
    ui.weak("The load menu shows what dr.ini (same folder) says; the game itself loads this file.");
}

// ------------------------------------------------------------------ Vampire Survivors

/// Power-up ids with their names and highest rank.
const VS_POWERUPS: [(&str, &str, Option<usize>); 20] = [
    ("POWER", "Might", Some(5)),
    ("ARMOR", "Armor", Some(3)),
    ("MAXHEALTH", "Max Health", Some(3)),
    ("REGEN", "Recovery", Some(5)),
    ("COOLDOWN", "Cooldown", Some(2)),
    ("AREA", "Area", Some(2)),
    ("SPEED", "Speed", Some(2)),
    ("DURATION", "Duration", Some(2)),
    ("AMOUNT", "Amount", Some(1)),
    ("MOVESPEED", "MoveSpeed", Some(2)),
    ("MAGNET", "Magnet", Some(2)),
    ("LUCK", "Luck", Some(3)),
    ("GROWTH", "Growth", Some(5)),
    ("GREED", "Greed", Some(5)),
    ("CURSE", "Curse", Some(5)),
    ("REVIVAL", "Revival", Some(1)),
    ("SHIELD", "Shield", Some(1)),
    ("REROLL", "Reroll", None),
    ("SKIP", "Skip", None),
    ("BANISH", "Banish", None),
];

pub fn vampire_survivors(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(214, 168, 58);
    let red = Color32::from_rgb(170, 30, 40);
    let mut sub = vec![];
    if let Some(v) = cx.text("/Version") {
        sub.push(format!("save version {v}"));
    }
    if let Some(c) = cx.text("/SelectedCharacter") {
        sub.push(c);
    }
    if let Some(s) = cx.text("/SelectedStage") {
        sub.push(s);
    }
    hero(
        ui,
        red,
        "Vampire Survivors",
        &sub.join("  \u{00B7}  "),
        |ui| {
            cx.tile(ui, "Gold coins", "/Coins", accent);
            cx.tile(ui, "Lifetime coins", "/LifetimeCoins", accent);
            cx.tile(ui, "Total coins", "/TotalCoins", accent);
        },
    );
    ui.weak("Lifetime coins counts all coins ever earned; raise it with your coins to keep the stats in line.");
    ui.add_space(6.0);
    if let Some(Value::Array(bought)) = cx.get("/BoughtPowerups").cloned() {
        let p = "/BoughtPowerups";
        let editable = cx.can_edit(p);
        let ids: Vec<String> = bought
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        let count = |id: &str| ids.iter().filter(|x| *x == id).count();
        // The known power-ups, then any other id this save has.
        let mut list: Vec<(String, String, Option<usize>)> = VS_POWERUPS
            .iter()
            .map(|(i, n, m)| (i.to_string(), n.to_string(), *m))
            .collect();
        for id in &ids {
            if !list.iter().any(|e| e.0 == *id) {
                list.push((id.clone(), id.clone(), None));
            }
        }
        panel(ui, "Power-ups", |ui| {
            let mut new: Option<Vec<String>> = None;
            columns(ui, 320.0, list.len(), |ui, i| {
                let (id, name, max) = &list[i];
                let n = count(id);
                ui.horizontal(|ui| {
                    let c = if cx.modified(p) {
                        crate::theme::MODIFIED
                    } else {
                        ui.visuals().strong_text_color()
                    };
                    ui.allocate_ui_with_layout(
                        egui::vec2(96.0, 22.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_width(96.0);
                            ui.add(egui::Label::new(RichText::new(name).color(c)).truncate())
                                .on_hover_text(id);
                        },
                    );
                    ui.add_enabled_ui(editable, |ui| {
                        if ui.small_button("\u{2212}").clicked() && n > 0 {
                            let mut v = ids.clone();
                            if let Some(at) = v.iter().rposition(|x| x == id) {
                                v.remove(at);
                            }
                            new = Some(v);
                        }
                        ui.label(RichText::new(n.to_string()).strong());
                        if ui.small_button("+").clicked() && max.is_none_or(|m| n < m) {
                            let mut v = ids.clone();
                            v.push(id.clone());
                            new = Some(v);
                        }
                    });
                    if let Some(m) = max {
                        let w = (ui.available_width() - 8.0).clamp(30.0, 120.0);
                        bar(ui, w, n as f32 / *m as f32, accent, None);
                    }
                });
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        editable,
                        egui::Button::new("All known power-ups to their highest rank"),
                    )
                    .clicked()
                {
                    let mut v = ids.clone();
                    for (id, _, max) in VS_POWERUPS {
                        if let Some(m) = max {
                            let have = v.iter().filter(|x| *x == id).count();
                            v.extend(std::iter::repeat_n(id.to_string(), m.saturating_sub(have)));
                        }
                    }
                    new = Some(v);
                }
            });
            ui.weak("A rank is bought once per entry in the save's list; coins spent on them aren't changed.");
            if let Some(v) = new.filter(|v| *v != ids) {
                cx.set(p, Value::Array(v.into_iter().map(Value::String).collect()));
            }
        });
    }
    panel(ui, "Collection", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (cap, key) in [
                ("Characters bought", "BoughtCharacters"),
                ("Characters unlocked", "UnlockedCharacters"),
                ("Weapons unlocked", "UnlockedWeapons"),
                ("Weapons found", "CollectedWeapons"),
                ("Items found", "CollectedItems"),
                ("Stages unlocked", "UnlockedStages"),
                ("Hyper modes", "UnlockedHypers"),
                ("Arcanas", "UnlockedArcanas"),
                ("Achievements", "Achievements"),
            ] {
                if let Some(Value::Array(a)) = cx.get(&join("", key)) {
                    fact(ui, cap, &a.len().to_string(), accent);
                }
            }
        });
        ui.weak("The lists themselves are in the Tree tab.");
    });
    columns(ui, 330.0, 2, |ui, i| {
        if i == 0 {
            panel(ui, "Records", |ui| {
                let list: Vec<(&str, String)> = [
                    ("Seconds survived", "LifetimeSurvived"),
                    ("HP healed", "LifetimeHeal"),
                    ("Beginner's luck", "BeginnersLuck"),
                    ("Highest fever", "HighestFever"),
                    ("Longest fever", "LongestFever"),
                    ("Completed hurries", "CompletedHurries"),
                ]
                .iter()
                .map(|(a, b)| (*a, join("", b)))
                .collect();
                rows(ui, cx, "vs_records", &list, 110.0);
            });
        } else {
            panel(ui, "Run options", |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (label, key) in [
                        ("Hyper", "SelectedHyper"),
                        ("Hurry", "SelectedHurry"),
                        ("Arcanas", "SelectedMazzo"),
                        ("Golden eggs", "SelectedGoldenEggs"),
                        ("Limit break", "SelectedLimitBreak"),
                    ] {
                        cx.chip(ui, label, &join("", key), red);
                    }
                });
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{decode_with, encode, Content, Options};
    use serde_json::json;

    #[test]
    fn silksong_and_hollow_knight_are_told_apart() {
        assert!(is_silksong(
            &json!({"playerData": {"geo": 5, "silkMax": 9}})
        ));
        assert!(is_silksong(
            &json!({"playerData": {"geo": 5, "ShellShards": 0}})
        ));
        assert!(!is_silksong(
            &json!({"playerData": {"geo": 5, "MPCharge": 0}})
        ));
        assert_eq!(crest_name("Toolmaster"), "Architect");
        assert_eq!(crest_name("Hunter_v2"), "Hunter v2");
    }

    /// Real saves: `SILKSONG_SAVES` = `user*.dat` files or folders separated by `|`.
    #[test]
    #[ignore]
    fn research_silksong_saves() {
        fn dats(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if p.is_dir() {
                for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
                    dats(&e.path(), out);
                }
            } else if p.extension().is_some_and(|e| e == "dat") {
                out.push(p.to_path_buf());
            }
        }
        let Ok(list) = std::env::var("SILKSONG_SAVES") else {
            return;
        };
        let mut paths = vec![];
        for p in list.split('|').filter(|s| !s.is_empty()) {
            dats(std::path::Path::new(p), &mut paths);
        }
        for path in &paths {
            let raw = std::fs::read(path).unwrap();
            let d = decode_with(&raw, &Options::for_path(path));
            let Content::Json { value, pretty } = &d.content else {
                panic!("{path:?}: not JSON")
            };
            assert_eq!(
                super::super::detect("JSON", value),
                Some(super::super::Game::Silksong),
                "{path:?}"
            );
            assert_eq!(
                encode(&d.layers, &d.content).unwrap(),
                raw,
                "{path:?}: round trip"
            );
            let mut v = value.clone();
            let geo = v["playerData"]["geo"].as_i64().unwrap();
            v["playerData"]["geo"] = json!(geo + 1);
            let out = encode(
                &d.layers,
                &Content::Json {
                    value: v.clone(),
                    pretty: *pretty,
                },
            )
            .unwrap();
            let Content::Json { value: b, .. } = decode_with(&out, &Options::default()).content
            else {
                panic!("{path:?}: edited save isn't JSON")
            };
            assert_eq!(b, v, "{path:?}: only rosaries changed");
        }
        eprintln!("{} Silksong saves checked", paths.len());
    }
}
