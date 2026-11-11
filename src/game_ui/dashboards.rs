//! Screens for games whose saves use general-purpose formats (JSON, XML, Lua, NBT…):
//! the values players care about, laid out like the game shows them. Everything is
//! looked up by name and simply left out when a save doesn't have it.

use super::{cell_grid, columns, fact, fmt_num, hero, meter, panel, select, selection, tabs, words, Cell, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Color32, RichText, Ui};
use serde_json::Value;

fn keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

fn join(base: &str, key: &str) -> String {
    format!("{base}/{}", pointer_escape(key))
}

/// Rows of (label, pointer) as a two-column grid.
fn grid(ui: &mut Ui, cx: &mut GameCtx, id: &str, rows: &[(&str, String)]) {
    egui::Grid::new(id).num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
        for (label, p) in rows {
            cx.grid_row(ui, label, p, 160.0);
        }
    });
}

/// Every scalar value of an object, as a grid spread over columns.
fn object_columns(ui: &mut Ui, cx: &mut GameCtx, id: &str, base: &str, label: impl Fn(&str) -> String) {
    let ks: Vec<String> = keys(cx, base).into_iter().filter(|k| !matches!(cx.get(&join(base, k)), Some(Value::Object(_) | Value::Array(_)))).collect();
    let cols = ((ui.available_width() / 360.0).floor() as usize).clamp(1, 3);
    let per = ks.len().div_ceil(cols);
    ui.columns(cols, |cs| {
        for (c, chunk) in ks.chunks(per.max(1)).enumerate() {
            let ui = &mut cs[c];
            ui.push_id((id, c), |ui| {
                let label_w = (ui.available_width() - 170.0).max(80.0);
                for k in chunk {
                    cx.row_fixed(ui, &label(k), &join(base, k), label_w, 80.0);
                }
            });
        }
    });
}

// ------------------------------------------------------------------ Skyrim / Fallout 4

pub fn bethesda(ui: &mut Ui, cx: &mut GameCtx) {
    let fallout = cx.tree.is_some_and(|t| t.kind_name.starts_with("Fallout"));
    let accent = if fallout { Color32::from_rgb(90, 190, 110) } else { Color32::from_rgb(150, 170, 200) };
    let name = cx.text("/Save/Player name").filter(|n| !n.is_empty()).unwrap_or_else(|| if fallout { "Sole Survivor".into() } else { "Dragonborn".into() });
    let sub: Vec<String> = ["/Save/Race", "/Save/Sex", "/Save/Location"]
        .iter()
        .filter_map(|p| cx.text(p))
        .map(|s| s.strip_suffix("Race").map(words).unwrap_or(s))
        .filter(|s| !s.is_empty())
        .collect();
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        if let Some(l) = cx.int("/Save/Level") {
            fact(ui, "Level", &l.to_string(), accent);
        }
        if let (Some(x), Some(n)) = (cx.num("/Save/Experience"), cx.num("/Save/Experience for next level")) {
            fact(ui, "Experience", &format!("{} / {}", fmt_num(x.round()), fmt_num(n.round())), accent);
        }
        // Stored like "0d.2h.13m.0 days.2 hours.13 minutes".
        if let Some(t) = cx.text("/Save/Play time") {
            fact(ui, "Play time", &t.split_whitespace().next().unwrap_or("").split('.').take(3).collect::<Vec<_>>().join(" "), accent);
        }
    });
    ui.weak("The character's name, level and location are shown as the game recorded them; the values below can be edited.");
    ui.add_space(6.0);
    if cx.get("/Game time").is_some() {
        panel(ui, "Game time", |ui| object_columns(ui, cx, "beth_time", "/Game time", |k| k.to_string()));
    }
    if cx.get("/Misc stats").is_some() {
        panel(ui, "Statistics", |ui| object_columns(ui, cx, "beth_stats", "/Misc stats", |k| k.to_string()));
    }
}

// ------------------------------------------------------------------ Hades / Hades II

/// Friendly names for Hades and Hades II resources (the rest are spelled out).
fn hades_name(key: &str, two: bool) -> Option<&'static str> {
    Some(match key {
        "GiftPoints" => "Nectar",
        "SuperGiftPoints" => "Ambrosia",
        _ if two => match key {
            "MetaCurrency" => "Bones",
            "MetaCardPointsCommon" => "Ash",
            "MemPointsCommon" => "Psyche",
            "CardUpgradePoints" => "Moon Dust",
            "MetaFabric" => "Silk",
            "Money" => "Gold (this run)",
            _ => return None,
        },
        "MetaPoints" => "Darkness",
        "Gems" => "Gemstones",
        "LockKeys" => "Chthonic Keys",
        "SuperLockKeys" => "Titan Blood",
        "SuperGems" => "Diamonds",
        _ => return None,
    })
}

pub fn hades(ui: &mut Ui, cx: &mut GameCtx) {
    let res = "/GameState/Resources";
    let two = cx.get(&join(res, "MemPointsCommon")).is_some() || cx.get(&join(res, "MetaCurrency")).is_some();
    let accent = if two { Color32::from_rgb(160, 110, 220) } else { Color32::from_rgb(220, 80, 60) };
    let mut sub = vec![(if two { "Hades II" } else { "Hades" }).to_string()];
    if let Some(n) = cx.int("/GameState/CompletedRunsCache") {
        sub.push(format!("{n} nights"));
    }
    if let Some(n) = cx.int("/GameState/ClearedRunsCache") {
        sub.push(format!("{n} cleared"));
    }
    hero(ui, accent, if two { "Melino\u{00EB}" } else { "Zagreus" }, &sub.join("  \u{00B7}  "), |ui| {
        let main: &[&str] = if two { &["MetaCardPointsCommon", "MemPointsCommon", "MetaCurrency", "CardUpgradePoints", "GiftPoints"] } else { &["MetaPoints", "Gems", "LockKeys", "GiftPoints", "SuperLockKeys", "SuperGems", "SuperGiftPoints"] };
        for k in main {
            cx.tile(ui, hades_name(k, two).unwrap_or(k), &join(res, k), accent);
        }
    });
    let facts: Vec<(&str, String)> = [
        ("Play time", cx.num("/GameState/GameplayTime").map(|s| format!("{:.1} hours", s / 3600.0))),
        ("Deepest run", cx.int("/GameState/HighestRunDepthCache").map(|n| format!("{n} rooms"))),
        ("Last killed by", cx.text("/GameState/CauseOfDeathDisplay").or_else(|| cx.text("/GameState/CauseOfDeath"))),
        ("Weapon", cx.text("/GameState/PrimaryWeaponName").map(|w| words(w.trim_start_matches("Weapon")))),
        ("Familiar", cx.text("/GameState/EquippedFamiliar").map(|f| words(f.trim_end_matches("Familiar")))),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|v| (k, v)))
    .collect();
    if !facts.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for (k, v) in &facts {
                fact(ui, k, v, accent);
            }
        });
        ui.add_space(8.0);
    }
    // Resources, grouped the way the game's inventory groups them.
    let all = keys(cx, res);
    let group = |k: &str| -> usize {
        if k.starts_with("Plant") || k.starts_with("Seed") {
            1
        } else if k.starts_with("Ore") {
            2
        } else if k.starts_with("Fish") {
            3
        } else if k.starts_with("Mixer") {
            4
        } else {
            0
        }
    };
    let names = ["Currencies", "Plants & seeds", "Ores", "Fish", "Reagents"];
    let present: Vec<usize> = (0..names.len()).filter(|g| all.iter().any(|k| group(k) == *g)).collect();
    if present.is_empty() {
        return;
    }
    panel(ui, "Resources", |ui| {
        let labels: Vec<String> = present.iter().map(|&g| names[g].to_string()).collect();
        let t = present[tabs(ui, "hades_res_tab", &labels)];
        let mut ks: Vec<&String> = all.iter().filter(|k| group(k) == t).collect();
        ks.sort_by_key(|k| (hades_name(k, two).is_none(), hades_name(k, two).map(str::to_string).unwrap_or_else(|| words(k))));
        let cols = ((ui.available_width() / 330.0).floor() as usize).clamp(1, 3);
        let per = ks.len().div_ceil(cols).max(1);
        ui.columns(cols, |cs| {
            for (c, chunk) in ks.chunks(per).enumerate() {
                let ui = &mut cs[c];
                let label_w = (ui.available_width() - 170.0).max(80.0);
                for k in chunk {
                    let label = hades_name(k, two).map(str::to_string).unwrap_or_else(|| words(k));
                    cx.row_fixed(ui, &label, &join(res, k), label_w, 80.0);
                }
            }
        });
    });
}

// ------------------------------------------------------------------ Stardew Valley

/// Stardew Valley: total skill XP for levels 1–10.
const SDV_XP: [i64; 10] = [100, 380, 770, 1300, 2150, 3300, 4800, 6900, 10_000, 15_000];

pub fn stardew(ui: &mut Ui, cx: &mut GameCtx) {
    let p = "/SaveGame/player";
    let g = "/SaveGame";
    let accent = Color32::from_rgb(110, 170, 70);
    let name = cx.text(&join(p, "name")).unwrap_or_else(|| "Farmer".into());
    let mut sub = vec![];
    if let Some(f) = cx.text(&join(p, "farmName")) {
        sub.push(format!("{f} Farm"));
    }
    if let (Some(s), Some(d), Some(y)) = (cx.text(&join(g, "currentSeason")), cx.int(&join(g, "dayOfMonth")), cx.int(&join(g, "year"))) {
        let mut season = s.clone();
        if let Some(c) = season.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        sub.push(format!("{season} {d}, Year {y}"));
    }
    hero(ui, accent, &name, &sub.join("  \u{00B7}  "), |ui| {
        for (cap, k) in [("Gold", "money"), ("Qi Gems", "qiGems"), ("Casino coins", "clubCoins")] {
            cx.tile(ui, cap, &join(p, k), accent);
        }
        cx.tile(ui, "Golden Walnuts", &join(g, "goldenWalnuts"), accent);
    });
    let wide = ui.available_width() > 760.0;
    let left = |ui: &mut Ui, cx: &mut GameCtx| {
        panel(ui, "Health & energy", |ui| {
            if let (Some(h), Some(m)) = (cx.num(&join(p, "health")), cx.num(&join(p, "maxHealth"))) {
                meter(ui, "Health", h, m, Color32::from_rgb(200, 60, 60));
            }
            if let (Some(h), Some(m)) = (cx.num(&join(p, "stamina")), cx.num(&join(p, "maxStamina"))) {
                meter(ui, "Energy", h, m, Color32::from_rgb(220, 180, 50));
            }
            grid(ui, cx, "sdv_health", &[("Max health", join(p, "maxHealth")), ("Max energy", join(p, "maxStamina")), ("Backpack slots", join(p, "maxItems")), ("House upgrades", join(p, "houseUpgradeLevel")), ("Deepest mine level", join(p, "deepestMineLevel"))]);
        });
    };
    let right = |ui: &mut Ui, cx: &mut GameCtx| {
        panel(ui, "Skills", |ui| {
            // (name, level key, index in experiencePoints, colour).
            for (label, k, xi, c) in [
                ("Farming", "farmingLevel", 0, Color32::from_rgb(110, 170, 70)),
                ("Mining", "miningLevel", 3, Color32::from_rgb(150, 110, 80)),
                ("Foraging", "foragingLevel", 2, Color32::from_rgb(70, 140, 90)),
                ("Fishing", "fishingLevel", 1, Color32::from_rgb(70, 130, 200)),
                ("Combat", "combatLevel", 4, Color32::from_rgb(200, 70, 70)),
                ("Luck", "luckLevel", 5, Color32::from_rgb(200, 170, 60)),
            ] {
                cx.attribute(ui, label, &join(p, k), 10.0, c);
                let xp = format!("{p}/experiencePoints/int/{xi}");
                if cx.get(&xp).is_some() {
                    ui.horizontal(|ui| {
                        ui.add_space(118.0);
                        cx.label(ui, "XP", &xp);
                        cx.editor(ui, &xp, 80.0);
                        let have = cx.int(&xp).unwrap_or(0);
                        match SDV_XP.iter().find(|&&t| t > have) {
                            Some(t) => ui.weak(format!("level {} at {}", SDV_XP.iter().position(|x| x == t).unwrap() + 1, fmt_num(*t as f64))),
                            None => ui.weak("max"),
                        };
                    });
                }
            }
            ui.weak(
                "Levels 0\u{2013}10. Raising XP is the natural way: the game levels the skill up overnight and lets you \
                 pick professions at 5 and 10. Setting the level directly skips those choices.",
            );
        });
    };
    if wide {
        ui.columns(2, |c| {
            left(&mut c[0], cx);
            right(&mut c[1], cx);
        });
    } else {
        left(ui, cx);
        right(ui, cx);
    }
    panel(ui, "Farm", |ui| {
        grid(ui, cx, "sdv_farm", &[("Total earned", join(p, "totalMoneyEarned")), ("Favorite thing", join(p, "favoriteThing")), ("Raining", join(g, "isRaining")), ("Daily luck", join(g, "dailyLuck"))]);
    });
    if let Ok(list) = super::crafting::stacks(super::Game::Stardew, cx) {
        panel(ui, "Crafting materials in the backpack", |ui| {
            if list.is_empty() {
                ui.weak("No wood, stone, ores, bars or other crafting materials in the backpack.");
            } else {
                egui::Grid::new("sdv_mats").num_columns(2).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                    for s in &list {
                        ui.label(&s.name);
                        cx.compact(ui, &s.paths[0], 80.0);
                        ui.end_row();
                    }
                });
            }
            ui.add_space(4.0);
            super::crafting::button(super::Game::Stardew, ui, cx);
        });
    }
}

// ------------------------------------------------------------------ Minecraft

pub fn minecraft(ui: &mut Ui, cx: &mut GameCtx) {
    let d = "/Data";
    let p = "/Data/Player";
    let accent = Color32::from_rgb(100, 160, 70);
    let world = cx.text(&join(d, "LevelName")).unwrap_or_else(|| "Minecraft world".into());
    let mode = |n: Option<i64>| match n {
        Some(0) => "Survival",
        Some(1) => "Creative",
        Some(2) => "Adventure",
        Some(3) => "Spectator",
        _ => "",
    };
    let mut sub = vec![mode(cx.int(&join(p, "playerGameType")).or(cx.int(&join(d, "GameType")))).to_string()];
    if let Some(v) = cx.text("/Data/Version/Name") {
        sub.push(format!("Java {v}"));
    }
    if cx.flag(&join(d, "hardcore")) {
        sub.push("Hardcore".into());
    }
    sub.retain(|s| !s.is_empty());
    hero(ui, accent, &world, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "XP level", &join(p, "XpLevel"), accent);
        cx.tile(ui, "Health", &join(p, "Health"), accent);
        cx.tile(ui, "Food", &join(p, "foodLevel"), accent);
    });
    let wide = ui.available_width() > 760.0;
    let left = |ui: &mut Ui, cx: &mut GameCtx| {
        panel(ui, "Player", |ui| {
            if let Some(h) = cx.num(&join(p, "Health")) {
                meter(ui, "Health", h, 20.0, Color32::from_rgb(200, 60, 60));
            }
            if let Some(f) = cx.num(&join(p, "foodLevel")) {
                meter(ui, "Food", f, 20.0, Color32::from_rgb(190, 130, 60));
            }
            grid(ui, cx, "mc_player", &[("Game mode", join(p, "playerGameType")), ("XP progress", join(p, "XpP")), ("Total XP", join(p, "XpTotal")), ("Score", join(p, "Score")), ("Saturation", join(p, "foodSaturationLevel"))]);
            ui.weak("Game mode: 0 Survival, 1 Creative, 2 Adventure, 3 Spectator. Health and food go up to 20.");
        });
    };
    let right = |ui: &mut Ui, cx: &mut GameCtx| {
        panel(ui, "World", |ui| {
            grid(ui, cx, "mc_world", &[("Difficulty", join(d, "Difficulty")), ("Cheats", join(d, "allowCommands")), ("Day time", join(d, "DayTime")), ("Raining", join(d, "raining")), ("Thundering", join(d, "thundering"))]);
            ui.weak("Difficulty: 0 Peaceful, 1 Easy, 2 Normal, 3 Hard. Day time 0 is sunrise, 6000 noon, 13000 night.");
        });
    };
    if wide {
        ui.columns(2, |c| {
            left(&mut c[0], cx);
            right(&mut c[1], cx);
        });
    } else {
        left(ui, cx);
        right(ui, cx);
    }
    for (title, list, id) in [("Inventory", join(p, "Inventory"), "mc_inv"), ("Ender chest", join(p, "EnderItems"), "mc_ender")] {
        let Some(Value::Array(items)) = cx.get(&list).cloned() else { continue };
        panel(ui, title, |ui| {
            // Slots by number: hotbar 0–8, backpack 9–35, armor 100–103, off hand −106.
            let order: Vec<i64> = if id == "mc_inv" { (0..36).chain([103, 102, 101, 100, -106]).collect() } else { (0..27).collect() };
            let at = |slot: i64| items.iter().position(|it| it.get("Slot").and_then(Value::as_i64) == Some(slot));
            let cells: Vec<Cell> = order
                .iter()
                .map(|&s| match at(s) {
                    Some(i) => {
                        let ip = format!("{list}/{i}");
                        let id_s = cx.text(&format!("{ip}/id")).unwrap_or_default();
                        Cell {
                            title: words(id_s.trim_start_matches("minecraft:")),
                            count: cx.int(&format!("{ip}/Count")).or(cx.int(&format!("{ip}/count"))),
                            empty: false,
                            modified: cx.modified(&ip),
                            tint: None,
                        }
                    }
                    None => Cell { title: String::new(), count: None, empty: true, modified: false, tint: None },
                })
                .collect();
            let sel = selection(ui, id);
            if let Some(i) = cell_grid(ui, &cells, 9, sel, accent) {
                select(ui, id, if sel == Some(i) || cells[i].empty { None } else { Some(i) });
            }
            match selection(ui, id).and_then(|i| order.get(i)).and_then(|&s| at(s)) {
                Some(i) => {
                    let ip = format!("{list}/{i}");
                    grid(ui, cx, &format!("{id}_sel"), &[("Item", format!("{ip}/id")), ("Count", format!("{ip}/Count")), ("Count", format!("{ip}/count"))]);
                }
                None => {
                    ui.weak("Click an item to change it. Empty slots can't be filled from here (use the Tree tab).");
                }
            }
            if id == "mc_inv" {
                ui.add_space(4.0);
                super::crafting::button(super::Game::Minecraft, ui, cx);
            }
        });
    }
}

// ------------------------------------------------------------------ Balatro

pub fn balatro(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(230, 70, 90);
    let g = "/GAME";
    let mut sub = vec![];
    if let Some(a) = cx.int("/GAME/round_resets/ante") {
        sub.push(format!("Ante {a}"));
    }
    if let Some(r) = cx.int("/GAME/round") {
        sub.push(format!("Round {r}"));
    }
    if let Some(s) = cx.int("/GAME/stake") {
        sub.push(format!("Stake {s}"));
    }
    if let Some(b) = cx.text("/GAME/blind_on_deck") {
        sub.push(format!("{b} blind next"));
    }
    hero(ui, accent, "Balatro run", &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Money ($)", &join(g, "dollars"), accent);
        cx.tile(ui, "Ante", "/GAME/round_resets/ante", accent);
        cx.tile(ui, "Hands per round", "/GAME/round_resets/hands", accent);
        cx.tile(ui, "Discards per round", "/GAME/round_resets/discards", accent);
    });
    panel(ui, "This round", |ui| {
        grid(ui, cx, "bal_round", &[
            ("Hands left", "/GAME/current_round/hands_left".into()),
            ("Discards left", "/GAME/current_round/discards_left".into()),
            ("Reroll cost", "/GAME/current_round/reroll_cost".into()),
            ("Interest cap", join(g, "interest_cap")),
            ("Joker slots", "/cardAreas/jokers/config/card_limit".into()),
            ("Consumable slots", "/cardAreas/consumeables/config/card_limit".into()),
        ]);
    });
    for (title, area) in [("Jokers", "/cardAreas/jokers/cards"), ("Consumables", "/cardAreas/consumeables/cards")] {
        let cards = keys(cx, area);
        panel(ui, title, |ui| {
            if cards.is_empty() {
                ui.weak("None right now.");
            }
            egui::Grid::new(("bal_cards", title)).num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                for c in &cards {
                    let cp = join(area, c);
                    ui.label(RichText::new(cx.text(&format!("{cp}/label")).unwrap_or_else(|| c.clone())).strong());
                    ui.horizontal(|ui| {
                        ui.weak("sells for");
                        cx.editor(ui, &format!("{cp}/sell_cost"), 50.0);
                    });
                    ui.horizontal(|ui| {
                        if cx.get(&format!("{cp}/edition")).is_some() {
                            ui.weak("edition");
                            cx.editor(ui, &format!("{cp}/edition/type"), 90.0);
                        }
                    });
                    ui.end_row();
                }
            });
        });
    }
    let hands = keys(cx, "/GAME/hands");
    if !hands.is_empty() {
        panel(ui, "Poker hands", |ui| {
            egui::Grid::new("bal_hands").num_columns(4).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                ui.weak("Hand");
                ui.weak("Level");
                ui.weak("Chips");
                ui.weak("Mult");
                ui.end_row();
                for h in &hands {
                    let hp = join("/GAME/hands", h);
                    ui.label(h.as_str());
                    for f in ["level", "chips", "mult"] {
                        ui.horizontal(|ui| cx.editor(ui, &format!("{hp}/{f}"), 60.0));
                    }
                    ui.end_row();
                }
            });
        });
    }
    let deck = keys(cx, "/cardAreas/deck/cards").len();
    if deck > 0 {
        ui.weak(format!("{deck} cards in the deck (edit them in the Tree tab)."));
    }
}

// ------------------------------------------------------------------ Hollow Knight

const HK_CHARMS: [&str; 40] = [
    "Gathering Swarm", "Wayward Compass", "Grubsong", "Stalwart Shell", "Baldur Shell", "Fury of the Fallen", "Quick Focus", "Lifeblood Heart",
    "Lifeblood Core", "Defender's Crest", "Flukenest", "Thorns of Agony", "Mark of Pride", "Steady Body", "Heavy Blow", "Sharp Shadow",
    "Spore Shroom", "Longnail", "Shaman Stone", "Soul Catcher", "Soul Eater", "Glowing Womb", "Fragile Heart", "Fragile Greed",
    "Fragile Strength", "Nailmaster's Glory", "Joni's Blessing", "Shape of Unn", "Hiveblood", "Dream Wielder", "Dashmaster", "Quick Slash",
    "Spell Twister", "Deep Focus", "Grubberfly's Elegy", "Kingsoul", "Sprintmaster", "Dreamshield", "Weaversong", "Grimmchild",
];

pub fn hollow_knight(ui: &mut Ui, cx: &mut GameCtx) {
    let p = "/playerData";
    let accent = Color32::from_rgb(150, 170, 200);
    let mut sub = vec![];
    if let Some(c) = cx.num(&join(p, "completionPercentage")) {
        sub.push(format!("{c}% complete"));
    }
    if let Some(t) = cx.num(&join(p, "playTime")) {
        sub.push(format!("{:.1} hours", t / 3600.0));
    }
    if cx.int(&join(p, "permadeathMode")).is_some_and(|m| m > 0) {
        sub.push("Steel Soul".into());
    }
    hero(ui, accent, "The Knight", &sub.join("  \u{00B7}  "), |ui| {
        for (cap, k) in [("Geo", "geo"), ("Masks", "maxHealthBase"), ("Charm notches", "charmSlots"), ("Pale Ore", "ore"), ("Simple Keys", "simpleKeys"), ("Rancid Eggs", "rancidEggs"), ("Essence", "dreamOrbs")] {
            cx.tile(ui, cap, &join(p, k), accent);
        }
    });
    if cx.get(&join(p, "ore")).is_some() && super::materials_button(ui, "6 Pale Ore", "The game has 6 Pale Ore for nail upgrades; this gives all 6. More than you have already is left alone.") {
        if cx.int(&join(p, "ore")).unwrap_or(0) < 6 {
            cx.set(&join(p, "ore"), serde_json::Value::from(6));
        }
    }
    panel(ui, "Nail & soul", |ui| {
        grid(ui, cx, "hk_nail", &[("Nail upgrades", join(p, "nailSmithUpgrades")), ("Nail damage", join(p, "nailDamage")), ("Max soul", join(p, "maxMP")), ("Soul vessels", join(p, "MPReserveMax")), ("Health", join(p, "health")), ("Max health", join(p, "maxHealth"))]);
    });
    panel(ui, "Abilities", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (label, k) in [
                ("Mothwing Cloak", "hasDash"),
                ("Mantis Claw", "hasWalljump"),
                ("Crystal Heart", "hasSuperDash"),
                ("Monarch Wings", "hasDoubleJump"),
                ("Shade Cloak", "hasShadowDash"),
                ("Isma's Tear", "hasAcidArmour"),
                ("Dream Nail", "hasDreamNail"),
                ("Lumafly Lantern", "hasLantern"),
                ("King's Brand", "hasKingsBrand"),
                ("City Crest", "hasCityKey"),
                ("Tram Pass", "hasTramPass"),
                ("Love Key", "hasLoveKey"),
            ] {
                cx.chip(ui, label, &join(p, k), accent);
            }
        });
    });
    panel(ui, "Charms", |ui| {
        ui.weak("Charms you own.");
        ui.horizontal_wrapped(|ui| {
            for (i, name) in HK_CHARMS.iter().enumerate() {
                cx.chip(ui, name, &join(p, &format!("gotCharm_{}", i + 1)), Color32::from_rgb(110, 130, 200));
            }
        });
    });
    panel(ui, "Relics", |ui| {
        grid(ui, cx, "hk_relics", &[("Wanderer's Journals", join(p, "trinket1")), ("Hallownest Seals", join(p, "trinket2")), ("King's Idols", join(p, "trinket3")), ("Arcane Eggs", join(p, "trinket4"))]);
    });
}

// ------------------------------------------------------------------ Slay the Spire

pub fn slay_the_spire(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 90, 60);
    let who = cx.text("/name").map(|n| words(&n.to_lowercase())).unwrap_or_else(|| "Run".into());
    let mut sub = vec![];
    if let Some(a) = cx.int("/act_num") {
        sub.push(format!("Act {a}"));
    }
    if let Some(f) = cx.int("/floor_num") {
        sub.push(format!("Floor {f}"));
    }
    if let Some(a) = cx.int("/ascension_level").filter(|&a| a > 0) {
        sub.push(format!("Ascension {a}"));
    }
    hero(ui, accent, &who, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Gold", "/gold", accent);
        cx.tile(ui, "HP", "/current_health", accent);
        cx.tile(ui, "Max HP", "/max_health", accent);
        cx.tile(ui, "Potion slots", "/potion_slots", accent);
    });
    if let (Some(h), Some(m)) = (cx.num("/current_health"), cx.num("/max_health")) {
        meter(ui, "HP", h, m, Color32::from_rgb(200, 60, 60));
        ui.add_space(8.0);
    }
    let lists: [(&str, &str); 3] = [("Deck", "/cards"), ("Relics", "/relics"), ("Potions", "/potions")];
    let mut change: Option<(String, Value)> = None;
    columns(ui, 280.0, lists.len(), |ui, i| {
        let (title, p) = lists[i];
        let Some(Value::Array(items)) = cx.get(p).cloned() else { return };
        panel(ui, &format!("{title} ({})", items.len()), |ui| {
            // The name column keeps its width: a grid otherwise sizes it from the narrowest first frame.
            egui::Grid::new(("sts", title)).num_columns(3).striped(true).spacing([8.0, 3.0]).min_col_width(150.0).show(ui, |ui| {
                for (n, it) in items.iter().enumerate() {
                    let ip = format!("{p}/{n}");
                    if it.is_object() {
                        cx.editor(ui, &format!("{ip}/id"), 150.0);
                        ui.horizontal(|ui| {
                            ui.weak("+");
                            cx.editor(ui, &format!("{ip}/upgrades"), 34.0);
                        });
                    } else {
                        cx.editor(ui, &ip, 150.0);
                        ui.label("");
                    }
                    if ui.small_button("\u{2715}").on_hover_text("Remove").clicked() {
                        let mut left = items.clone();
                        left.remove(n);
                        change = Some((p.to_string(), Value::Array(left)));
                        if p == "/relics" {
                            if let Some(Value::Array(mut counters)) = cx.get("/relic_counters").cloned() {
                                if n < counters.len() {
                                    counters.remove(n);
                                    cx.set("/relic_counters", Value::Array(counters));
                                }
                            }
                        }
                    }
                    ui.end_row();
                }
            });
            let id = egui::Id::new(("sts_add", title));
            let mut name = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut name).hint_text("internal id, e.g. Strike_R").desired_width(150.0));
                if ui.button("Add").clicked() && !name.trim().is_empty() {
                    let mut more = items.clone();
                    more.push(match i {
                        0 => serde_json::json!({"id": name.trim(), "upgrades": 0, "misc": 0}),
                        _ => Value::String(name.trim().to_string()),
                    });
                    change = Some((p.to_string(), Value::Array(more)));
                    if p == "/relics" {
                        if let Some(Value::Array(mut counters)) = cx.get("/relic_counters").cloned() {
                            counters.push(Value::from(-1));
                            cx.set("/relic_counters", Value::Array(counters));
                        }
                    }
                    name.clear();
                }
            });
            ui.data_mut(|d| d.insert_temp(id, name));
        });
    });
    if let Some((p, v)) = change {
        cx.set(&p, v);
    }
    let flags: Vec<&str> = ["has_emerald_key", "has_ruby_key", "has_sapphire_key", "is_ascension_mode", "is_final_act_on"].into_iter().filter(|k| cx.get(&format!("/{k}")).is_some()).collect();
    if !flags.is_empty() || cx.get("/ascension_level").is_some() {
        panel(ui, "Run settings", |ui| {
            egui::Grid::new("sts_run").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                for (label, k) in [("Ascension level", "ascension_level"), ("Floor", "floor_num"), ("Act", "act_num"), ("Shop card removal cost", "purgeCost")] {
                    if cx.get(&format!("/{k}")).is_some() {
                        cx.grid_row(ui, label, &format!("/{k}"), 80.0);
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for k in &flags {
                    cx.chip(ui, &words(k.trim_start_matches("has_")), &format!("/{k}"), accent);
                }
            });
            ui.weak("The three keys unlock the Heart; Final act lets the run go on to Act 4. Ids must be the game's own (a wrong one can stop the run loading); removing a relic also drops its counter.");
        });
    }
}

// ------------------------------------------------------------------ RPG Maker MV / MZ

/// Names of the eight parameters `_paramPlus` adds to (the engine's order).
const RM_PARAMS: [&str; 8] = ["Max HP", "Max MP", "Attack", "Defense", "M. Attack", "M. Defense", "Agility", "Luck"];

/// RPG Maker MV / MZ: the party members' level, EXP, HP/MP, stat bonuses and skills.
/// Item, class and skill names live in the game's own data files, not the save.
pub fn rpg_maker(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(120, 140, 220);
    let party: Vec<i64> = match cx.get("/party/_actors/@a") {
        Some(Value::Array(a)) => a.iter().filter_map(Value::as_i64).collect(),
        _ => vec![],
    };
    let mut sub = vec![format!("{} in the party", party.len())];
    if let Some(s) = cx.int("/party/_steps") {
        sub.push(format!("{} steps", fmt_num(s as f64)));
    }
    if let Some(f) = cx.num("/system/_framesOnSave") {
        sub.push(format!("{:.1} hours played", f / 60.0 / 3600.0));
    }
    let leader = party.first().and_then(|id| cx.text(&format!("/actors/_data/@a/{id}/_name"))).unwrap_or_else(|| "The party".into());
    hero(ui, accent, &leader, &sub.join("  \u{00B7}  "), |ui| {
        cx.tile(ui, "Gold", "/party/_gold", accent);
        for &id in &party {
            let a = format!("/actors/_data/@a/{id}");
            let name = cx.text(&format!("{a}/_name")).unwrap_or_else(|| format!("Actor {id}"));
            cx.tile(ui, &format!("{name}: level"), &format!("{a}/_level"), accent);
        }
    });
    // Everyone the game has made, party members first.
    let mut ids = party.clone();
    if let Some(Value::Array(all)) = cx.get("/actors/_data/@a") {
        ids.extend((0..all.len() as i64).filter(|i| all[*i as usize].is_object() && !party.contains(i)));
    }
    let names: Vec<String> = ids
        .iter()
        .map(|id| {
            let n = cx.text(&format!("/actors/_data/@a/{id}/_name")).filter(|n| !n.is_empty()).unwrap_or_else(|| format!("Actor {id}"));
            if party.contains(id) {
                n
            } else {
                format!("{n} (not in party)")
            }
        })
        .collect();
    if names.is_empty() {
        return;
    }
    panel(ui, "Characters", |ui| {
        let t = tabs(ui, "rm_actor", &names);
        let a = format!("/actors/_data/@a/{}", ids[t]);
        let class = cx.int(&format!("{a}/_classId")).unwrap_or(0);
        let exp = format!("{a}/_exp/{class}");
        columns(ui, 320.0, 2, |ui, col| {
            if col == 0 {
                ui.label(RichText::new("Level and experience").strong());
                grid(ui, cx, "rm_lv", &[("Name", format!("{a}/_name")), ("Nickname", format!("{a}/_nickname")), ("Level", format!("{a}/_level")), ("EXP", exp.clone())]);
                ui.weak(format!(
                    "Class {class}. The game works the level out from EXP the next time EXP changes, \
                     so raise both: the EXP a level needs depends on the class's curve in the game's data."
                ));
                ui.add_space(6.0);
                grid(ui, cx, "rm_hp", &[("HP", format!("{a}/_hp")), ("MP", format!("{a}/_mp")), ("TP", format!("{a}/_tp"))]);
            } else {
                ui.label(RichText::new("Stat bonuses").strong());
                ui.weak("Added on top of the class's stats (what seeds and level-ups give).");
                let rows: Vec<(&str, String)> = RM_PARAMS.iter().enumerate().map(|(i, n)| (*n, format!("{a}/_paramPlus/@a/{i}"))).collect();
                grid(ui, cx, "rm_params", &rows);
                if let Some(Value::Array(s)) = cx.get(&format!("{a}/_skills/@a")) {
                    let s: Vec<String> = s.iter().filter_map(Value::as_i64).map(|i| format!("#{i}")).collect();
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Skills learned: {}", s.len())).strong());
                    ui.weak(if s.is_empty() { "None.".into() } else { s.join(", ") });
                    ui.weak("Skill names are in the game's data files; the numbers are their ids.");
                }
            }
        });
    });
}

/// Where an RE Engine save keeps its save slots: the first array of objects with `_Data`
/// directly under a top-level object (Onimusha: Way of the Sword).
pub fn ree_slots(root: &Value) -> Option<String> {
    root.as_object()?.iter().find_map(|(top, v)| {
        v.as_object()?.iter().find_map(|(k, a)| {
            let a = a.as_array()?;
            a.iter().any(|s| s.pointer("/_Data/_ResourceData").is_some()).then(|| format!("/{}/{}", pointer_escape(top), pointer_escape(k)))
        })
    })
}

/// Reads an "id<TAB>name" table (lines starting with '#' are comments).
fn id_names(src: &'static str) -> std::collections::HashMap<i64, &'static str> {
    src.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let (id, name) = l.split_once('\t')?;
            Some((id.parse().ok()?, name))
        })
        .collect()
}

/// English item name for an Onimusha ItemID (from the game's own item text).
fn onimusha_item(id: i64) -> Option<&'static str> {
    static T: std::sync::OnceLock<std::collections::HashMap<i64, &'static str>> = std::sync::OnceLock::new();
    T.get_or_init(|| id_names(include_str!("../fmt/data/onimusha_items.tsv"))).get(&id).copied()
}

/// English charm name for an Onimusha charm ID; the game's text uses the unsigned form.
fn onimusha_charm(id: i64) -> Option<&'static str> {
    static T: std::sync::OnceLock<std::collections::HashMap<i64, &'static str>> = std::sync::OnceLock::new();
    T.get_or_init(|| id_names(include_str!("../fmt/data/onimusha_charms.tsv"))).get(&i64::from(id as u32)).copied()
}

pub fn onimusha(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(190, 50, 60);
    let Some(base) = ree_slots(cx.root) else { return };
    let count = cx.get(&base).and_then(Value::as_array).map_or(0, Vec::len);
    // Slots in use have played time; the game writes "chapter\tplace" and
    // "date\tplay time\tdifficulty\tobjective" for its load screen.
    let used: Vec<usize> = (0..count).filter(|i| cx.int(&format!("{base}/{i}/_Data/_ResourceData/PlayTime")).unwrap_or(0) > 0).collect();
    if used.is_empty() {
        hero(ui, accent, "Onimusha: Way of the Sword", "No saved games in this file yet", |_| {});
        return;
    }
    let label = |cx: &GameCtx, i: usize| {
        let sub = cx.text(&format!("{base}/{i}/Subtitle")).unwrap_or_default();
        let place = sub.split('\t').nth(1).filter(|s| !s.is_empty());
        match place {
            Some(p) => format!("Slot {} \u{2014} {p}", i + 1),
            None if i + 1 == count => "Autosave".to_string(),
            None => format!("Slot {}", i + 1),
        }
    };
    let names: Vec<String> = used.iter().map(|&i| label(cx, i)).collect();
    let t = tabs(ui, "oni_slot", &names);
    let i = used[t];
    let s = format!("{base}/{i}");
    let d = format!("{s}/_Data");
    let detail = cx.text(&format!("{s}/Detail")).unwrap_or_default();
    let parts: Vec<&str> = detail.split('\t').collect();
    let mut sub = vec![];
    if let Some(p) = cx.text(&format!("{s}/Subtitle")).and_then(|t| t.split('\t').nth(1).map(str::to_string)).filter(|p| !p.is_empty()) {
        sub.push(p);
    }
    if let Some(time) = parts.get(1) {
        sub.push(format!("{} played", time.trim_start_matches('0').trim_start_matches(':')));
    }
    if let Some(diff) = parts.get(2).filter(|x| !x.is_empty()) {
        sub.push(format!("{diff} difficulty"));
    }
    if let Some(when) = parts.first().filter(|x| !x.is_empty()) {
        sub.push(format!("saved {when}"));
    }
    hero(ui, accent, "Onimusha: Way of the Sword", &sub.join("  \u{00B7}  "), |ui| {
        // Red souls are kept multiplied by a key; the parser shows them as a plain number.
        let souls = format!("{d}/_ResourceData/SoulAmount");
        if cx.get(&souls).is_some_and(Value::is_number) {
            cx.tile(ui, "Red souls", &souls, accent);
        }
        cx.tile(ui, "Money", &format!("{d}/_ResourceData/Money"), accent);
        cx.tile(ui, "Health", &format!("{d}/_PlayerStatus/Health"), accent);
        cx.tile(ui, "Oni energy", &format!("{d}/_PlayerStatus/OniEnergy"), accent);
    });
    if let Some(goal) = parts.get(3).filter(|x| !x.is_empty()) {
        ui.label(RichText::new(format!("Objective: {goal}")).weak());
    }
    // Items: 500 entries, unused ones hold item 46. EquipNum is what's on hand, BoxNum what's
    // in storage (matched against the game's item screen).
    let items = format!("{d}/_Items");
    let used: Vec<(usize, i64)> = cx
        .get(&items)
        .and_then(Value::as_array)
        .map(|a| a.iter().enumerate().filter_map(|(k, e)| e.get("ItemID")?.as_i64().filter(|&id| id != 46).map(|id| (k, id))).collect())
        .unwrap_or_default();
    if !used.is_empty() {
        panel(ui, "Items", |ui| {
            egui::Grid::new("oni_items").num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                ui.strong("Item");
                ui.strong("On hand");
                ui.strong("In storage");
                ui.end_row();
                for (k, id) in &used {
                    let p = format!("{items}/{k}");
                    let name = onimusha_item(*id).map_or_else(|| format!("Item {id}"), str::to_string);
                    ui.label(name);
                    cx.cell(ui, &format!("{p}/EquipNum"), 70.0);
                    cx.cell(ui, &format!("{p}/BoxNum"), 70.0);
                    ui.end_row();
                }
            });
            ui.weak("The game shows these counts as On Hand and In Storage. Key items (weapons, the bow) have no count.");
            ui.add_space(4.0);
            super::crafting::offer(super::Game::Onimusha, ui, cx);
        });
    }
    // Charms: unused entries hold the "no charm" ID 325704896. Level 0 is the
    // first level, and EquipSlot is -1 when the charm isn't worn.
    let charms = format!("{d}/#de498466");
    let owned: Vec<(usize, i64)> = cx
        .get(&charms)
        .and_then(Value::as_array)
        .map(|a| a.iter().enumerate().filter_map(|(k, e)| e.get("#37ba528f")?.as_i64().filter(|&id| id != 325704896 && id != 0).map(|id| (k, id))).collect())
        .unwrap_or_default();
    if !owned.is_empty() {
        panel(ui, "Charms", |ui| {
            egui::Grid::new("oni_charms").num_columns(3).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
                ui.strong("Charm");
                ui.strong("Level");
                ui.strong("Worn");
                ui.end_row();
                for (k, id) in &owned {
                    let p = format!("{charms}/{k}");
                    ui.label(onimusha_charm(*id).map_or_else(|| format!("Charm {id}"), str::to_string));
                    cx.cell(ui, &format!("{p}/Level"), 50.0);
                    ui.label(if cx.int(&format!("{p}/EquipSlot")).is_some_and(|n| n >= 0) { "Yes" } else { "No" });
                    ui.end_row();
                }
            });
            ui.weak("Level 0 is the charm's first level; each charm has three.");
        });
    }
    // Equipment that has gained experience (the rest of the 32 entries are unused).
    let eq = format!("{d}/_Equipments");
    let rows: Vec<usize> = cx
        .get(&eq)
        .and_then(Value::as_array)
        .map(|a| a.iter().enumerate().filter(|(_, e)| e.as_object().is_some_and(|o| o.values().any(|v| v.as_i64().is_some_and(|n| n != 0)))).map(|(k, _)| k).collect())
        .unwrap_or_default();
    panel(ui, "Equipment", |ui| {
        if rows.is_empty() {
            ui.weak("No equipment with experience yet.");
            return;
        }
        egui::Grid::new("oni_eq").num_columns(4).striped(true).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.strong("Equipment");
            ui.strong("Experience");
            ui.strong("Total experience");
            ui.end_row();
            for k in &rows {
                let p = format!("{eq}/{k}");
                ui.label(format!("#{}", cx.int(&format!("{p}/EquipmentID")).unwrap_or(0)));
                cx.cell(ui, &format!("{p}/Exp"), 90.0);
                cx.cell(ui, &format!("{p}/TotalExp"), 90.0);
                ui.end_row();
            }
        });
    });
    ui.add_space(4.0);
    ui.weak(
        "Each save slot is edited separately; the game keeps the slot you load. Item and equipment names aren't known \
         yet \u{2014} the Values tab lists everything.",
    );
}

/// Stellaris, Hearts of Iron IV and other Paradox text saves.
pub fn paradox(ui: &mut Ui, cx: &mut GameCtx, kind: &str) {
    let accent = Color32::from_rgb(70, 140, 200);
    let date = cx.text("/date").unwrap_or_default();
    if kind.starts_with("Stellaris") {
        let id = cx.get("/player/0/country").and_then(Value::as_i64).unwrap_or(0);
        let c = format!("/country/{id}");
        let res = format!("{c}/modules/standard_economy_module/resources");
        let name = cx.text("/name").unwrap_or_else(|| "Stellaris empire".into());
        let ruler = cx.text("/player/0/name").unwrap_or_default();
        hero(ui, accent, &name, &format!("{date}  \u{00B7}  played by {ruler}  \u{00B7}  Stellaris"), |ui| {
            for (label, k) in [("Energy", "energy"), ("Minerals", "minerals"), ("Food", "food"), ("Alloys", "alloys"), ("Consumer goods", "consumer_goods"), ("Influence", "influence"), ("Unity", "unity")] {
                if cx.get(&join(&res, k)).is_some() {
                    cx.tile(ui, label, &join(&res, k), accent);
                }
            }
        });
        panel(ui, "All resources", |ui| {
            if cx.get(&res).is_none() {
                ui.weak("This empire has no stockpiles in the save.");
                return;
            }
            object_columns(ui, cx, "st_res", &res, words);
        });
        ui.weak("Research totals are progress toward the current projects. The empire is the one you play (player 1).");
        return;
    }
    if kind.starts_with("Hearts of Iron") {
        let tag = cx.text("/player").unwrap_or_default();
        let c = format!("/countries/{tag}");
        let mut sub = vec![date.clone()];
        if let Some(i) = cx.text("/ideology") {
            sub.push(words(&i));
        }
        if let Some(d) = cx.text("/difficulty") {
            sub.push(format!("{d} difficulty"));
        }
        if cx.text("/ironman").is_some_and(|s| !s.is_empty()) {
            sub.push("ironman".into());
        }
        hero(ui, accent, &tag, &sub.join("  \u{00B7}  "), |ui| {
            cx.tile(ui, "Political power", &format!("{c}/politics/political_power"), accent);
            cx.tile(ui, "Stability", &format!("{c}/stability"), accent);
            cx.tile(ui, "War support", &format!("{c}/war_support"), accent);
            cx.tile(ui, "Command power", &format!("{c}/command_power"), accent);
        });
        panel(ui, "Country", |ui| {
            grid(ui, cx, "hoi_c", &[
                ("Army experience", format!("{c}/army_experience")),
                ("Navy experience", format!("{c}/navy_experience")),
                ("Air experience", format!("{c}/air_experience")),
                ("Research slots", format!("{c}/research_slot")),
                ("Capital state", format!("{c}/capital")),
            ]);
            ui.weak("Stability and war support run from 0 to 1 (0.5 is 50%).");
        });
        return;
    }
    if kind.starts_with("Europa Universalis IV") {
        return super::screens2::eu4(ui, cx);
    }
    if kind.starts_with("Crusader Kings II ") {
        return super::screens2::ck2(ui, cx);
    }
    hero(ui, accent, kind, &date, |_| {});
    panel(ui, "Save", |ui| {
        object_columns(ui, cx, "pdx_top", "", words);
    });
}

pub fn rimworld(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(200, 150, 60);
    let colony = cx.text("/Colony/Faction").unwrap_or_else(|| "Colony".into());
    let n = cx.int("/Colony/Colonists").unwrap_or(0);
    let version = cx.text("/Colony/Game version").unwrap_or_default();
    hero(ui, accent, &colony, &format!("{n} colonist{}  \u{00B7}  RimWorld {version}", if n == 1 { "" } else { "s" }), |ui| {
        fact(ui, "Silver (all stacks)", &fmt_num(cx.num("/Colony/Silver (all stacks)").unwrap_or(0.0)), accent);
    });
    let names = keys(cx, "/Colonists");
    panel(ui, "Colonists", |ui| {
        if names.is_empty() {
            ui.weak("No colonists found in this save.");
            return;
        }
        let t = tabs(ui, "rw_pawn", &names);
        let base = join("/Colonists", &names[t]);
        if let Some(full) = cx.text(&format!("{base}/Name")) {
            ui.weak(full);
        }
        let skills = keys(cx, &format!("{base}/Skills"));
        egui::Grid::new(("rw_skills", t)).num_columns(3).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.strong("Skill");
            ui.strong("Level");
            ui.strong("Passion");
            ui.end_row();
            for s in &skills {
                let p = join(&format!("{base}/Skills"), s);
                ui.label(words(s));
                cx.cell(ui, &format!("{p}/Level"), 80.0);
                cx.cell(ui, &format!("{p}/Passion"), 170.0);
                ui.end_row();
            }
        });
        ui.weak("Levels run 0\u{2013}20 in the base game. A passion makes the colonist learn that skill faster.");
    });
    let stacks = keys(cx, "/Silver");
    panel(ui, "Silver stacks", |ui| {
        if stacks.is_empty() {
            ui.weak("No silver on the map.");
            return;
        }
        egui::ScrollArea::vertical().max_height(220.0).id_salt("rw_silver").show(ui, |ui| {
            egui::Grid::new("rw_silver_g").num_columns(2).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                for s in &stacks {
                    ui.label(s);
                    cx.editor(ui, &join("/Silver", s), 100.0);
                    ui.end_row();
                }
            });
        });
        ui.weak("A stack holds up to 500 in the base game.");
    });
}

/// Subnautica and Below Zero: a slot's `gameinfo.json`.
pub fn subnautica(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(40, 160, 190);
    const MODES: [&str; 4] = ["Survival", "Freedom", "Hardcore", "Creative"];
    let mode = cx.int("/gameMode").unwrap_or(0);
    let secs = cx.num("/gameTime").unwrap_or(0.0);
    let title = if cx.get("/userName").is_some() { "Subnautica: Below Zero" } else { "Subnautica" };
    let name = MODES.get(mode as usize).copied().unwrap_or("Custom");
    hero(ui, accent, title, &format!("{name} mode  \u{00B7}  {:.1} hours in game", secs / 3600.0), |ui| {
        fact(ui, "Game mode", name, accent);
    });
    panel(ui, "Game mode", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, m) in MODES.iter().enumerate() {
                if ui.selectable_label(mode == i as i64, *m).clicked() && mode != i as i64 {
                    cx.set("/gameMode", Value::from(i as i64));
                }
            }
        });
        ui.weak("Takes effect the next time you load this slot. Freedom turns off food and water; Creative also turns off oxygen and building costs. Changing mode may turn achievements off.");
    });
    panel(ui, "In this slot", |ui| {
        ui.horizontal_wrapped(|ui| {
            for (k, label) in [("seamothPresent", "Seamoth"), ("exosuitPresent", "Prawn Suit"), ("cyclopsPresent", "Cyclops"), ("rocketPresent", "Neptune rocket"), ("basePresent", "Base")] {
                if cx.get(&format!("/{k}")).is_some() {
                    let have = cx.flag(&format!("/{k}"));
                    ui.label(RichText::new(format!("{} {label}", if have { "\u{2713}" } else { "\u{2717}" })).color(if have { theme_ok(ui) } else { ui.visuals().weak_text_color() }));
                }
            }
        });
        ui.weak("Inventory, health and blueprints are in global-objects.bin next to this file, which isn't editable yet.");
    });
}

fn theme_ok(ui: &Ui) -> Color32 {
    crate::theme::fg(ui, crate::theme::OK)
}

pub fn oxygen_not_included(ui: &mut Ui, cx: &mut GameCtx) {
    let accent = Color32::from_rgb(110, 180, 90);
    let colony = cx.text("/Colony/Colony").unwrap_or_else(|| "Colony".into());
    let cycles = cx.int("/Colony/Cycles").unwrap_or(0);
    let n = cx.int("/Colony/Duplicants").unwrap_or(0);
    hero(ui, accent, &colony, &format!("Cycle {cycles}  \u{00B7}  {n} duplicants  \u{00B7}  Oxygen Not Included"), |_| {});
    if cx.get("/Worlds").is_some() {
        let pages: Vec<String> = ["Duplicants", "Map", "Worlds & rockets"].iter().map(|s| s.to_string()).collect();
        match tabs(ui, "oni_tab", &pages) {
            1 => return super::oni_map::map_tab(ui, cx),
            2 => return super::oni_map::worlds_tab(ui, cx, accent),
            _ => {}
        }
    }
    let names = keys(cx, "/Duplicants");
    panel(ui, "Duplicants", |ui| {
        if names.is_empty() {
            ui.weak("No duplicants found in this save.");
            return;
        }
        let t = tabs(ui, "oni_dupe", &names);
        let base = join("/Duplicants", &names[t]);
        let levels = format!("{base}/Klei.AI.AttributeLevels/saveLoadLevels");
        let count = cx.get(&levels).and_then(Value::as_array).map_or(0, Vec::len);
        ui.label(RichText::new("Attributes").strong());
        egui::Grid::new(("oni_attr", t)).num_columns(3).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.strong("Attribute");
            ui.strong("Level");
            ui.strong("Experience");
            ui.end_row();
            for i in 0..count {
                let p = format!("{levels}/{i}");
                ui.label(words(&cx.text(&format!("{p}/attributeId")).unwrap_or_default()));
                cx.cell(ui, &format!("{p}/level"), 70.0);
                cx.cell(ui, &format!("{p}/experience"), 110.0);
                ui.end_row();
            }
        });
        let resume = format!("{base}/MinionResume");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Total experience (skill points come from this):");
            cx.editor(ui, &format!("{resume}/totalExperienceGained"), 130.0);
        });
        let skills = keys(cx, &format!("{resume}/MasteryBySkillID"));
        if !skills.is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new("Skills learned").strong());
            ui.horizontal_wrapped(|ui| {
                for s in &skills {
                    let p = join(&format!("{resume}/MasteryBySkillID"), s);
                    let mut on = cx.flag(&p);
                    if ui.checkbox(&mut on, words(s)).changed() {
                        cx.set(&p, Value::Bool(on));
                    }
                }
            });
        }
        ui.weak("Attribute levels top out at 20 without mods. Everything else about a duplicant is in the Values tab.");
    });
}
