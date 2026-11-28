//! Layout checks for every game screen. Each screen is drawn off-screen from a made-up save the
//! way the window draws it (a scroll area as wide as the window; every panel at the full width it
//! has when its tab is open, all shown at once; each page of a screen with pages), at a desktop
//! width (1150 px) and a phone width (390 px). A check fails on:
//!
//! * text wider than the box it is drawn in (cut off, or spilling into its neighbour);
//! * anything drawn past the right edge of the window, so the screen has to be scrolled sideways
//!   to see it (a table inside its own sideways-scrolling area is fine);
//! * at the desktop width, a panel of three or more rows whose content uses under half of its
//!   width: such a panel should lay its rows out in columns.
//!
//! Made-up saves: the table-driven screens fill their own spec; a hand-written screen opens the
//! sample a reader's tests build where one exists, and otherwise a save made of the values the
//! screen asked for while it was drawn (repeated until it asks for nothing new).

use crate::formats::{decode_with, Content, Decoded, Options};
use crate::game_ui::{self, Game, GameCtx};
use crate::tree::TreeDoc;
use crate::widgets::ScrollLock;
use eframe::egui;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// A panel as it was drawn: the room inside it and what its content took.
#[derive(Clone, Debug)]
struct PanelSeen {
    title: String,
    inner: egui::Rect,
    content: egui::Rect,
}

thread_local! {
    /// Pointers the screen read, while a save is being made up for it.
    static READS: RefCell<Option<BTreeSet<String>>> = const { RefCell::new(None) };
    /// Panels drawn, while a frame is being checked.
    static PANELS: RefCell<Option<Vec<PanelSeen>>> = const { RefCell::new(None) };
    /// Every tab drawn at once, each laid out as when it is the open one (see `columns`).
    static STACKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Read by `game_ui::columns`: panels go one under the other, as tabs do, though all are shown.
pub(crate) fn stacking() -> bool {
    STACKING.with(|s| s.get())
}

/// Called by `GameCtx::get`: notes the pointer while a save is being made up.
pub(crate) fn note_read(p: &str) {
    READS.with(|r| {
        if let Some(s) = r.borrow_mut().as_mut() {
            if s.len() < 5000 {
                s.insert(p.to_string());
            }
        }
    });
}

/// Called by `game_ui::panel` at the end of its card: notes the card's room and content.
pub(crate) fn note_panel(ui: &egui::Ui, title: &str) {
    if !ui.is_visible() {
        return;
    }
    PANELS.with(|p| {
        if let Some(v) = p.borrow_mut().as_mut() {
            v.push(PanelSeen { title: title.to_string(), inner: ui.max_rect(), content: ui.min_rect() });
        }
    });
}

const WIDE: f32 = 1150.0;
const PHONE: f32 = 390.0;
const HEIGHT: f32 = 800.0;

/// Every hand-written screen (the table-driven ones are `Game::Simple`).
const HAND: &[Game] = &[
    Game::EldenRing, Game::DarkSouls3, Game::DarkSouls2, Game::DarkSouls1, Game::Sekiro, Game::Terraria, Game::TerrariaWorld, Game::Pokemon,
    Game::InfiniteFusion, Game::Bethesda, Game::Hades, Game::Stardew, Game::Minecraft, Game::Balatro, Game::HollowKnight, Game::SlayTheSpire,
    Game::RpgMaker, Game::Onimusha, Game::Paradox, Game::RimWorld, Game::Subnautica, Game::OxygenNotIncluded, Game::GreenHell, Game::ReSlot,
    Game::Hogwarts, Game::RenPy, Game::MassEffect, Game::Xcom, Game::Xcom2, Game::Civ6, Game::Diablo2, Game::Pathfinder, Game::Ff7,
    Game::Ff7Remake, Game::HogwartsDb, Game::Ff9, Game::DyingLight2, Game::DyingLight, Game::Witcher3, Game::Cyberpunk, Game::Kenshi,
    Game::KenshiSquad, Game::Palworld, Game::P3r, Game::P4g, Game::Ffx2, Game::P5r, Game::Ffx, Game::Metro, Game::Yakuza0, Game::YakuzaDe,
    Game::Kiwami, Game::Ffxv, Game::DeathStranding2, Game::DeathStranding1, Game::Andromeda, Game::Gow2018, Game::GowRagnarok, Game::Cairn,
    Game::Borderlands, Game::Borderlands3, Game::Wonderlands, Game::TruckSim, Game::X4, Game::Mgsv, Game::TotalWar, Game::GrimDawn,
    Game::TitanQuest, Game::Torchlight2, Game::Fallout12, Game::DeusEx, Game::Kotor, Game::Nwn, Game::Infinity, Game::DragonAge,
    Game::HalfLife, Game::HalfLife2, Game::Silksong, Game::Undertale, Game::Deltarune, Game::VampireSurvivors, Game::DarkestDungeon,
    Game::DarkestDungeon2, Game::Octopath, Game::Dos2, Game::Bg3, Game::Wasteland3, Game::Valheim, Game::GoldenSun, Game::Torment,
    Game::Starbound, Game::Expedition33, Game::Phasmophobia, Game::LethalCompany, Game::Repo, Game::Fallout3Nv, Game::Nier,
    Game::WorldOfHorror, Game::Sims1, Game::Sims2, Game::Sims4, Game::Sims3, Game::DeadRising, Game::DeadRising2, Game::LegoBatman,
    Game::PlanetCrafter, Game::FfResonance, Game::Ff16,
];

/// A new variant must be added to `HAND` (this match has no catch-all, so it won't build otherwise).
#[allow(dead_code)]
fn listed(g: Game) -> bool {
    use Game::*;
    match g {
        EldenRing | DarkSouls3 | DarkSouls2 | DarkSouls1 | Sekiro | Terraria | TerrariaWorld | Pokemon | InfiniteFusion | Bethesda | Hades | Stardew
        | Minecraft | Balatro | HollowKnight | SlayTheSpire | RpgMaker | Onimusha | Paradox | RimWorld | Subnautica | OxygenNotIncluded | GreenHell
        | ReSlot | Hogwarts | RenPy | MassEffect | Xcom | Xcom2 | Civ6 | Diablo2 | Pathfinder | Ff7 | Ff7Remake | HogwartsDb | Ff9 | DyingLight2
        | DyingLight | Witcher3 | Cyberpunk | Kenshi | KenshiSquad | Palworld | P3r | P4g | Ffx2 | P5r | Ffx | Metro | Yakuza0 | YakuzaDe | Kiwami
        | Ffxv | DeathStranding2 | DeathStranding1 | Andromeda | Gow2018 | GowRagnarok | Cairn | Borderlands | Borderlands3 | Wonderlands
        | TruckSim | X4 | Mgsv | TotalWar | GrimDawn | TitanQuest | Torchlight2 | Fallout12 | DeusEx | Kotor | Nwn | Infinity | DragonAge
        | HalfLife | HalfLife2 | Silksong | Undertale | Deltarune | VampireSurvivors | DarkestDungeon | DarkestDungeon2 | Octopath | Dos2 | Bg3
        | Wasteland3 | Valheim | GoldenSun | Torment | Starbound | Expedition33 | Phasmophobia | LethalCompany | Repo | Fallout3Nv | Nier
        | WorldOfHorror | Sims1 | Sims2 | Sims4 | Sims3 | DeadRising | DeadRising2 | LegoBatman | PlanetCrafter | FfResonance | Ff16 => true,
        Simple(_) => false,
    }
}

/// Problems that are not problems, each with why: (screen, text in the problem, reason).
const ALLOW: &[(&str, &str, &str)] = &[
    // The sample has one coin stack: one row and a one-line note, which is all the panel holds.
    ("Dos2", "panel \"Gold\"", "one row and a note"),
    // The made-up save has one character: one table row and a one-line note.
    ("ReSlot", "panel \"Health\"", "one row and a note"),
    // Read-only lines about the last run that wrap at the panel's width when they are long.
    ("WorldOfHorror", "panel \"Last run\"", "short read-only lines"),
    // Column headings of tables that scroll sideways on a phone, cut by that table's own edge.
    ("HalfLife2", "\"SMG rounds, SMG grenades\"", "heading of a table that scrolls sideways"),
    ("NieR Replicant ver.1.22474487139...", "\"Level (shown minus one)\"", "heading of a table that scrolls sideways"),
    ("Slay the Spire 2", "\"Preferred Ascension\"", "heading of a table that scrolls sideways"),
];

/// One screen with its made-up save.
struct Fixture {
    game: Game,
    name: String,
    /// Where the save came from, for the report: "spec", "sample", "made up" or "empty".
    source: &'static str,
    doc: Option<Decoded>,
    root: Value,
}

impl Fixture {
    fn parts(&self) -> (&Value, Option<&TreeDoc>) {
        match self.doc.as_ref().map(|d| &d.content) {
            Some(Content::Tree(t)) => (&t.view, Some(&**t)),
            Some(Content::Json { value, .. }) => (value, None),
            _ => (&self.root, None),
        }
    }
}

fn screen_name(g: Game) -> String {
    match g {
        Game::Simple(i) => game_ui::simple::all()[i as usize].name.to_string(),
        other => format!("{other:?}"),
    }
}

/// One drawn frame and where its content area was.
struct Drawn {
    out: egui::FullOutput,
    /// The clip rect of the screen's own scroll area: shapes clipped to anything else are inside
    /// a nested area (a table that scrolls sideways, a text box).
    clip: egui::Rect,
    /// The right edge of the width the screen is laid out to.
    right: f32,
}

fn new_ctx() -> egui::Context {
    let ctx = egui::Context::default();
    crate::theme::install_fonts(&ctx);
    crate::theme::apply(&ctx, true);
    ctx
}

/// Draws one frame the way the window's game tab does: with `active`, that tab open; with
/// `None`, every tab shown at once, each laid out as it is when open.
fn draw(ctx: &egui::Context, f: &Fixture, w: f32, active: Option<&str>) -> Drawn {
    STACKING.with(|s| s.set(active.is_none()));
    let (root, tree) = f.parts();
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, HEIGHT))), ..Default::default() };
    let (mut clip, mut right) = (egui::Rect::NOTHING, 0.0);
    let out = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let sb = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin + ui.spacing().scroll.bar_outer_margin;
            let cw = (ui.available_width() - sb).max(100.0);
            game_ui::set_active_section(ui.ctx(), active);
            egui::ScrollArea::both().auto_shrink([false, false]).max_width(cw + sb).show_locked(ui, |ui| {
                ui.set_max_width(cw);
                clip = ui.clip_rect();
                right = ui.max_rect().left() + cw;
                let mut cx = GameCtx { root, baseline: root, tree, edits: vec![], archive: None, open_entry: None };
                game_ui::show(f.game, ui, &mut cx);
            });
        });
    });
    STACKING.with(|s| s.set(false));
    Drawn { out, clip, right }
}

/// "<text> spans A..B but is clipped to C..D" for text that starts in the last 120 px of its
/// area: a column of a table that scrolls sideways, mostly out of view.
fn scrolled_out(t: &str) -> bool {
    let nums: Vec<f32> = t.rsplit("spans ").next().unwrap_or("").split(|c: char| !(c.is_ascii_digit() || c == '-')).filter_map(|x| x.parse().ok()).collect();
    nums.len() >= 4 && nums[0] > nums[3] - 120.0
}

fn same_rect(a: egui::Rect, b: egui::Rect) -> bool {
    (a.min - b.min).length() < 0.5 && (a.max - b.max).length() < 0.5
}

/// What is wrong with one drawn frame, one line per problem.
fn problems(d: &Drawn, panels: &[PanelSeen], wide: bool) -> Vec<String> {
    let mut bad: Vec<String> = crate::app::layout_tests::cut_off_text(&d.out)
        .into_iter()
        // egui's own debug-build warnings, painted over the window, are not the screen's text.
        .filter(|t| !t.starts_with("\"\u{1F525}") && !t.starts_with("\"egui ") && !scrolled_out(t))
        .map(|t| format!("cut off: {t}"))
        .collect();
    // Past the right edge, outside any nested area that scrolls or clips by itself.
    let mut past: Option<(f32, bool, String)> = None;
    for cs in &d.out.shapes {
        if !same_rect(cs.clip_rect, d.clip) {
            continue;
        }
        let mut stack = vec![&cs.shape];
        while let Some(sh) = stack.pop() {
            if let egui::epaint::Shape::Vec(v) = sh {
                stack.extend(v.iter());
                continue;
            }
            let r = sh.visual_bounding_rect();
            if !r.is_finite() || !r.is_positive() || r.right() <= d.right + 1.0 {
                continue;
            }
            // Named by its text where there is one; a box or a line by where it is.
            let (text, what) = match sh {
                egui::epaint::Shape::Text(t) => (true, format!("text {:?} at y {:.0}", t.galley.text().chars().take(40).collect::<String>(), r.top())),
                _ => (false, format!("a {:.0}x{:.0} shape at x {:.0}, y {:.0}", r.width(), r.height(), r.left(), r.top())),
            };
            let better = match &past {
                None => true,
                Some((x, was_text, _)) => (text && !was_text) || (text == *was_text && r.right() > *x),
            };
            if better {
                let inside = panels.iter().filter(|p| p.inner.expand(16.0).contains(r.center_top())).last().map(|p| format!(" in panel {:?}", p.title)).unwrap_or_default();
                past = Some((r.right(), text, format!("{what}{inside}")));
            }
        }
    }
    if let Some((x, _, what)) = past {
        bad.push(format!("past the right edge: {what} reaches {x:.0}, the window's content ends at {:.0}", d.right));
    }
    if wide {
        for p in panels {
            let rows = ((p.content.height() - 24.0) / 24.0).floor();
            let used = p.content.right() - p.inner.left();
            if rows >= 3.0 && p.inner.width() >= 400.0 && used < 0.5 * p.inner.width() {
                bad.push(format!("half empty: panel {:?} uses {used:.0} of {:.0} px over about {rows} rows", p.title, p.inner.width()));
            }
        }
    }
    bad
}

/// Draws a screen (every tab at once, then page by page) at one width; returns its problems and
/// how many panels were seen.
fn check_screen(ctx: &egui::Context, f: &Fixture, w: f32) -> (Vec<String>, usize) {
    ctx.memory_mut(|m| m.data = Default::default());
    let wide = w > 640.0;
    let mut bad = vec![];
    let mut seen = 0;
    let mut check = |d: &Drawn, label: &str, bad: &mut Vec<String>| {
        let panels = PANELS.with(|p| p.borrow_mut().take()).unwrap_or_default();
        seen += panels.len();
        for p in problems(d, &panels, wide) {
            bad.push(format!("[{label}] {p}"));
        }
    };
    // Once as the window opens it (to learn the screen's pages), then every tab at once.
    draw(ctx, f, w, Some(""));
    let pages = game_ui::page_tabs(ctx);
    draw(ctx, f, w, None);
    PANELS.with(|p| *p.borrow_mut() = Some(vec![]));
    let d = draw(ctx, f, w, None);
    check(&d, "first page", &mut bad);
    if let Some((id, names)) = pages {
        for (i, n) in names.iter().enumerate().skip(1) {
            game_ui::open_page(ctx, &id, i);
            draw(ctx, f, w, None);
            PANELS.with(|p| *p.borrow_mut() = Some(vec![]));
            let d = draw(ctx, f, w, None);
            check(&d, n, &mut bad);
        }
    }
    bad.sort();
    bad.dedup();
    (bad, seen)
}

/// Decodes a reader's test sample; `None` when it opens on no hand-written screen.
fn sample(bytes: Vec<u8>, file: &str) -> Option<(Game, Decoded)> {
    let d = decode_with(&bytes, &Options::for_path(std::path::Path::new(file)));
    let game = match &d.content {
        Content::Tree(t) => game_ui::detect(&t.kind_name, &t.view),
        Content::Json { value, .. } => game_ui::detect("JSON", value),
        _ => None,
    }?;
    (!matches!(game, Game::Simple(_))).then_some((game, d))
}

/// The samples the readers' own tests build: (bytes, a file name to open them under).
fn samples() -> Vec<(Vec<u8>, &'static str)> {
    use crate::fmt::*;
    vec![
        (d2s::tests::sample(), "hero.d2s"),
        (starbound::tests::sample(), "player.player"),
        (grimdawn::tests::sample(), "player.gdc"),
        (torchlight2::tests::sample(), "hero.svt"),
        (dragonage::tests::sample(), "save.das"),
        (infinity::tests::sample(), "BALDUR.gam"),
        (xcom2::tests::sample(), "save_1"),
        (valheim::tests::synthetic(37), "hero.fch"),
        (half_life::tests::goldsrc(), "save.sav"),
        (half_life::tests::source(), "save.sav"),
        (sims1::tests::synthetic(), "User00000.iff"),
        (sims2::tests::sample(), "Neighborhood.package"),
        (sims3::tests::package(), "save.nhd"),
        (sims4::tests::package(), "Slot_00000001.save"),
        (fos::tests::synthetic(), "Save 1.fos"),
        (nier::tests::synthetic(), "GameData.dat"),
        (mgsv::tests::sample(), "PERSONAL_DATA"),
        (mea::tests::synthetic(), "Career.sav"),
        (ffxv::tests::synthetic(), "game.save"),
        (ff7r::tests::remake_slot(), "ff7remake000.sav"),
        (wasteland3::tests::synthetic_xml(), "save.xml"),
        (pokemon3::tests::synthetic(), "game.sav"),
        (pokemon12::tests::red_save(), "red.sav"),
        (larian::tests::dos2_save(), "save.lsv"),
        (kenshi::tests::sample(), "quick.save"),
        (civ6::tests::sample(), "AutoSave.Civ6Save"),
        (titanquest::tests::sample(), "Player.chr"),
        (borderlands3::tests::sample(), "1.sav"),
        (nwn::tests::save(), "savegame.sav"),
        (darkest::tests::estate(), "persist.estate.json"),
        (marshal::tests::essentials_sample(), "Game.rxdata"),
        (me1::tests::sample(), "save.MassEffectSave"),
        (pdxbin::tests::eu4(), "save.eu4"),
        (ds2::tests::data(), "checkpoint.dat"),
    ]
}

/// Keys a screen looks for before it reads anything else (so a made-up save reaches its panels).
fn seed(g: Game) -> Value {
    match g {
        Game::Ff7 => json!({"Slot 1": {"Location": "Midgar", "Party": {"Cloud": {}, "Barret": {}, "Tifa": {}}}}),
        Game::Ff9 => json!({"Slot 1": {"Gil": 900, "Party": {"Zidane": {}, "Vivi": {}, "Steiner": {}}}}),
        Game::DeathStranding1 => json!({"Facilities": {"Capital Knot City": {}, "Lake Knot City": {}, "Port Knot City": {}}}),
        Game::DyingLight2 => json!({"Bucket": {"Combat": {}, "Parkour": {}}}),
        Game::DyingLight => json!({"Skill trees": {"Agility": 5, "Power": 3, "Survivor": 2}}),
        Game::Metro => json!({"Inventory": {"Military-grade rounds (money)": 100, "Revolver rounds": 30, "Shotgun shells": 12, "Filters (crafting)": 4}}),
        Game::GreenHell => json!([{"PCM_HP": 80.0, "Day": 3}]),
        Game::ReSlot => json!({"Info": {"_SaveCount": 3}, "Player": {"_CurrentMaxHP": 100, "_CurrentHP": 80}}),
        Game::Xcom => json!({"Headquarters": {"m_iCash": 500}}),
        Game::Gow2018 => json!({"Slot 1": {"Saved": "Today", "Resources": {"Dew": 3, "Ore": 5, "Leather": 3, "Ember": 2}, "Equipment owned": {"Armor": ["Chest", "Wrist"]}}}),
        Game::GoldenSun => json!({"File 1": {"Name": "Isaac"}}),
        Game::Torment => json!({"Party members": {"The Nameless One": {"Level": 3, "Fighter level": 3}, "Morte": {"Level": 2}}, "Party": {}}),
        Game::TotalWar => json!({"Factions": {"rome": {}, "carthage": {}, "egypt": {}}}),
        Game::RpgMaker => json!({"actors": {"_data": {"@a": [null, {"_name": "Hero", "_level": 5, "_hp": 100, "_mp": 20}]}}, "party": {"_actors": {"@a": [1]}, "_gold": 100}}),
        Game::Onimusha => json!({"Slots": {"Data": [{"_Data": {"_ResourceData": {"PlayTime": 100}}, "Subtitle": "Chapter 1\tCastle"}]}}),
        Game::RenPy => json!({"Variables": {"money": 10, "name": "Player", "love": 3}}),
        Game::HogwartsDb => json!({
            "MiscDataDynamic": [{"DataOwner": "ExperienceManager", "DataName": "ExperiencePoints", "DataValue": "100"}, {"DataOwner": "Player0", "DataName": "PerkPoints", "DataValue": "2"}],
            "InventoryDynamic": [{"CharacterID": "Player0", "HolderID": "ResourceInventory", "ItemID": "Knuts", "Count": 500}]
        }),
        Game::DarkestDungeon2 => json!({"m_ProfileValues": {"m_Values": {}}, "m_UnlockContainer": {}}),
        Game::LegoBatman => json!({"Game": {"StudsCollected": 100, "TotalPlaytime": 1000000000}}),
        Game::Pathfinder => json!({
            "player": {"Money": 100, "Chapter": 2, "m_MainCharacter": "u1", "PartyCharacters": ["u1", "u2"]},
            "party": {"m_EntityData": [
                {"UniqueId": "u1", "Descriptor": {"CustomName": "Hero", "Stats": {
                    "Strength": {"m_BaseValue": 14}, "Dexterity": {"m_BaseValue": 12}, "Constitution": {"m_BaseValue": 13},
                    "Intelligence": {"m_BaseValue": 10}, "Wisdom": {"m_BaseValue": 8}, "Charisma": {"m_BaseValue": 16}, "HitPoints": {"m_BaseValue": 20}},
                    "Progression": {"Experience": 1000, "MythicExperience": 2, "Classes": [{"CharacterClass": "x", "Level": 3}]}}},
                {"UniqueId": "u2", "Descriptor": {"CustomName": "Friend", "Stats": {"Strength": {"m_BaseValue": 10}}}}
            ]}
        }),
        _ => json!({}),
    }
}

/// A save made of what the screen reads: drawn, and every pointer it asked for filled in, until
/// it asks for nothing new.
fn made_up(ctx: &egui::Context, g: Game, seed: Value) -> Value {
    let mut root = seed;
    for _ in 0..6 {
        ctx.memory_mut(|m| m.data = Default::default());
        READS.with(|r| *r.borrow_mut() = Some(BTreeSet::new()));
        let f = Fixture { game: g, name: String::new(), source: "", doc: None, root: root.clone() };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            draw(ctx, &f, WIDE, Some(""));
            draw(ctx, &f, WIDE, None);
        }));
        // Deepest first, so a value read both as a group and as a number becomes the group.
        let mut reads: Vec<String> = READS.with(|r| r.borrow_mut().take()).unwrap_or_default().into_iter().collect();
        reads.sort_by_key(|p| std::cmp::Reverse(p.matches('/').count()));
        let mut grew = false;
        for (i, p) in reads.iter().enumerate() {
            if p.is_empty() || !p.starts_with('/') || root.pointer(p).is_some() {
                continue;
            }
            // Under a value that is already there as a plain value: leave it, or the screen loses it.
            let mut parent = p.as_str();
            let mut blocked = false;
            while let Some(cut) = parent.rfind('/') {
                parent = &parent[..cut];
                if let Some(v) = root.pointer(parent) {
                    blocked = !v.is_object() && !v.is_array();
                    break;
                }
            }
            if blocked {
                continue;
            }
            crate::synthetic::set(&mut root, p, crate::synthetic::value_for(p, i));
            grew = true;
        }
        if !grew {
            break;
        }
    }
    root
}

fn leaves(v: &Value) -> usize {
    match v {
        Value::Object(o) => o.values().map(leaves).sum(),
        Value::Array(a) => a.iter().map(leaves).sum(),
        _ => 1,
    }
}

fn fixtures(ctx: &egui::Context) -> Vec<Fixture> {
    let mut out = vec![];
    for (i, s) in game_ui::simple::all().iter().enumerate() {
        let g = Game::Simple(i as u16);
        // A spec with steps that pick by key or hash: what the screen reads is filled in on top.
        let plain = game_ui::simple::tests::synthetic(s);
        let root = match crate::synthetic::build(s) {
            Some(mut v) => {
                // A list or table whose path is also a marker was filled with a number: give it rows.
                let leaf = |v: &Value, p: &str| v.pointer(p).is_some_and(|x| !x.is_object() && !x.is_array());
                for l in s.lists {
                    if leaf(&v, l.path) {
                        if let Some(items) = plain.pointer(l.path) {
                            crate::synthetic::set(&mut v, l.path, items.clone());
                        }
                    }
                }
                for t in s.tables {
                    if leaf(&v, t.path) {
                        let rows: Vec<Value> = (0..3)
                            .map(|r| {
                                let mut row = json!({});
                                for n in t.name {
                                    crate::synthetic::set(&mut row, n, json!(format!("Member {}", r + 1)));
                                }
                                for (j, c) in t.cols.iter().enumerate() {
                                    crate::synthetic::set(&mut row, c.1, crate::synthetic::value_for(c.1, r * 10 + j));
                                }
                                row
                            })
                            .collect();
                        crate::synthetic::set(&mut v, t.path, Value::Array(rows));
                    }
                }
                v
            }
            None => made_up(ctx, g, plain),
        };
        out.push(Fixture { game: g, name: s.name.to_string(), source: "spec", doc: None, root });
    }
    let mut decoded: Vec<(Game, Decoded)> = samples().into_iter().filter_map(|(b, n)| sample(b, n)).collect();
    for &g in HAND {
        match decoded.iter().position(|(dg, _)| *dg == g) {
            Some(k) => {
                let (_, d) = decoded.remove(k);
                out.push(Fixture { game: g, name: screen_name(g), source: "sample", doc: Some(d), root: Value::Null });
            }
            None => {
                let root = made_up(ctx, g, seed(g));
                let source = if leaves(&root) >= 3 { "made up" } else { "empty" };
                out.push(Fixture { game: g, name: screen_name(g), source, doc: None, root });
            }
        }
    }
    out
}

fn allowed(name: &str, problem: &str) -> bool {
    ALLOW.iter().any(|(s, needle, _)| *s == name && problem.contains(needle))
}

fn sweep(only: Option<&str>) {
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::i18n::set(Some("en"));
    let start = std::time::Instant::now();
    let ctx = new_ctx();
    let mut all = fixtures(&ctx);
    if let Some(o) = only {
        all.retain(|f| f.name.contains(o));
    }
    let made = start.elapsed();
    // LAYOUT_DUMP=<screen> prints the made-up save of a screen, to see what it lacks.
    if let Ok(name) = std::env::var("LAYOUT_DUMP") {
        for f in all.iter().filter(|f| f.name == name) {
            println!("{}: {}", f.name, serde_json::to_string(f.parts().0).unwrap_or_default());
        }
    }
    let results: Vec<(usize, f32, Vec<String>, usize, f32)> = std::thread::scope(|sc| {
        let jobs: Vec<_> = [WIDE, PHONE]
            .into_iter()
            .map(|w| {
                let all = &all;
                sc.spawn(move || {
                    crate::i18n::set(Some("en"));
                    let ctx = new_ctx();
                    let mut res = vec![];
                    for (k, f) in all.iter().enumerate() {
                        let t = std::time::Instant::now();
                        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_screen(&ctx, f, w)));
                        let (bad, seen) = r.unwrap_or_else(|_| (vec!["panicked while drawing".to_string()], 0));
                        res.push((k, w, bad, seen, t.elapsed().as_secs_f32()));
                    }
                    res
                })
            })
            .collect();
        jobs.into_iter().flat_map(|j| j.join().unwrap_or_default()).collect()
    });
    let mut report = vec![];
    let mut no_panels = BTreeSet::new();
    let mut slow: Vec<(f32, String)> = results.iter().map(|(k, w, _, _, t)| (*t, format!("{} at {w}", all[*k].name))).collect();
    slow.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("slowest: {:?}", slow.iter().take(12).map(|(t, n)| format!("{n} {t:.2}s")).collect::<Vec<_>>());
    for (k, w, bad, seen, _) in &results {
        let f = &all[*k];
        if *seen == 0 && *w == WIDE {
            no_panels.insert(format!("{} ({})", f.name, f.source));
        }
        for b in bad {
            if !allowed(&f.name, b) {
                report.push(format!("{} at {w}: {b}", f.name));
            }
        }
    }
    let count = |s: &str| all.iter().filter(|f| f.source == s).count();
    println!(
        "layout check: {} screens ({} from specs, {} from samples, {} made up, {} empty) in {:.1} s (saves made in {:.1} s)",
        all.len(),
        count("spec"),
        count("sample"),
        count("made up"),
        count("empty"),
        start.elapsed().as_secs_f64(),
        made.as_secs_f64()
    );
    println!("screens with no panel drawn: {}: {:?}", no_panels.len(), no_panels);
    println!("empty saves: {:?}", all.iter().filter(|f| f.source == "empty").map(|f| f.name.as_str()).collect::<Vec<_>>());
    report.sort();
    assert!(report.is_empty(), "{} layout problems:\n{}", report.len(), report.iter().take(300).cloned().collect::<Vec<_>>().join("\n"));
}

/// Every game screen, at a desktop and a phone width. `LAYOUT_ONLY=<part of a screen's name>`
/// checks only those screens.
///
///     cargo test --lib every_screen_lays_out_evenly -- --nocapture
#[test]
fn every_screen_lays_out_evenly() {
    sweep(std::env::var("LAYOUT_ONLY").ok().as_deref());
}
