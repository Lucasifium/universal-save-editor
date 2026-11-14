//! Game screens: a layout made for one game's save (a character sheet, an inventory
//! grid, a party of Pokémon…) on top of the same data the Values tab edits.
//!
//! Screens never change data themselves: like every other view they push
//! `(pointer, new value)` edits, which the app applies with undo, range checks,
//! dependent values (a level following attributes) and checksums.

mod arpg;
mod borderlands;
mod cairn;
mod coop;
pub mod crafting;
mod darkest;
mod dead_rising;
mod dashboards;
mod deus_ex;
mod dragon_age;
mod fallout12;
mod fromsoft;
mod golden_sun;
mod half_life;
mod indie;
mod infinity;
mod kotor;
mod larian;
mod nwn;
mod lego;
mod octopath;
mod oni_map;
mod pathfinder;
mod planet_crafter;
mod infinite_fusion;
mod pokemon;
mod expedition33;
mod ffr;
mod fallout3nv;
mod ff16;
mod nier;
mod sims;
mod screens2;
pub use screens2::STORY_BACKUP_REQUEST;

/// Temp-data keys of the screen find box: needle, current match, running count, last count.
pub const FIND_NEEDLE: &str = "game_find_needle";
pub const FIND_TARGET: &str = "game_find_target";
pub const FIND_SEEN: &str = "game_find_seen";
pub const FIND_COUNT: &str = "game_find_count";

/// Starts a frame of the screen find box.
pub fn begin_find(ctx: &egui::Context, needle: &str, target: usize) {
    let previous = ctx.data(|d| d.get_temp::<usize>(egui::Id::new(FIND_SEEN))).unwrap_or(0);
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new(FIND_COUNT), previous);
        d.insert_temp(egui::Id::new(FIND_NEEDLE), needle.trim().to_lowercase());
        d.insert_temp(egui::Id::new(FIND_TARGET), target);
        d.insert_temp(egui::Id::new(FIND_SEEN), 0usize);
    });
}

/// A label that matches the find box gets a highlight, and the current match is scrolled to.
/// Call with the rectangle the label occupies.
pub fn find_mark(ui: &Ui, rect: egui::Rect, text: &str) {
    let ctx = ui.ctx();
    let needle = ctx.data(|d| d.get_temp::<String>(egui::Id::new(FIND_NEEDLE))).unwrap_or_default();
    if needle.is_empty() || !text.to_lowercase().contains(&needle) {
        return;
    }
    let (target, index) = ctx.data_mut(|d| {
        let seen = d.get_temp::<usize>(egui::Id::new(FIND_SEEN)).unwrap_or(0);
        d.insert_temp(egui::Id::new(FIND_SEEN), seen + 1);
        (d.get_temp::<usize>(egui::Id::new(FIND_TARGET)).unwrap_or(0), seen)
    });
    let current = index == target;
    let color = if current { theme::MODIFIED } else { theme::ACCENT };
    ui.painter().rect_stroke(rect.expand(2.0), 4.0, egui::Stroke::new(if current { 2.0 } else { 1.0 }, color));
    if current {
        // Scroll once per search / Next press, not on every frame.
        let key = format!("{needle}#{target}");
        let id = egui::Id::new("game_find_scrolled");
        if ctx.data(|d| d.get_temp::<String>(id)).as_deref() != Some(&key) {
            ui.scroll_to_rect(rect, Some(egui::Align::Center));
            ctx.data_mut(|d| d.insert_temp(id, key));
        }
    }
}

/// Temp-data key where a value's "Show in Tree" request (its pointer) is left for the app.
pub const SHOW_IN_TREE: &str = "show_in_tree_request";
mod simple;

/// Texts of the table-driven screens, for the translation check.
pub fn spec_texts() -> Vec<&'static str> {
    simple::texts()
}
mod starbound;
mod survival;
mod terraria;
mod terraria_world;
mod torment;
mod totalwar;
mod trucks;
mod x4;
mod mgsv;
mod woh;

use crate::i18n::{tr, tr_any, trf};
use crate::theme;
use crate::tree::TreeDoc;
use crate::widgets;
use eframe::egui::{self, Color32, FontId, RichText, Sense, Ui};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Game {
    EldenRing,
    DarkSouls3,
    DarkSouls2,
    DarkSouls1,
    Sekiro,
    Terraria,
    TerrariaWorld,
    Pokemon,
    InfiniteFusion,
    Bethesda,
    Hades,
    Stardew,
    Minecraft,
    Balatro,
    HollowKnight,
    SlayTheSpire,
    RpgMaker,
    Onimusha,
    Paradox,
    RimWorld,
    Subnautica,
    OxygenNotIncluded,
    GreenHell,
    ReSlot,
    Hogwarts,
    RenPy,
    MassEffect,
    Xcom,
    Xcom2,
    Civ6,
    Diablo2,
    Pathfinder,
    Ff7,
    Ff7Remake,
    HogwartsDb,
    Ff9,
    DyingLight2,
    DyingLight,
    Witcher3,
    Cyberpunk,
    Kenshi,
    KenshiSquad,
    Palworld,
    P3r,
    P4g,
    Ffx2,
    P5r,
    Ffx,
    Metro,
    Yakuza0,
    YakuzaDe,
    Kiwami,
    Ffxv,
    DeathStranding2,
    DeathStranding1,
    Andromeda,
    Gow2018,
    GowRagnarok,
    Cairn,
    Borderlands,
    Borderlands3,
    Wonderlands,
    TruckSim,
    X4,
    Mgsv,
    TotalWar,
    GrimDawn,
    TitanQuest,
    Torchlight2,
    Fallout12,
    DeusEx,
    Kotor,
    Nwn,
    Infinity,
    DragonAge,
    HalfLife,
    HalfLife2,
    Silksong,
    Undertale,
    Deltarune,
    VampireSurvivors,
    DarkestDungeon,
    DarkestDungeon2,
    Octopath,
    Dos2,
    Bg3,
    Wasteland3,
    Valheim,
    GoldenSun,
    Torment,
    Starbound,
    Expedition33,
    Phasmophobia,
    LethalCompany,
    Repo,
    Fallout3Nv,
    Nier,
    WorldOfHorror,
    Sims1,
    Sims2,
    Sims4,
    Sims3,
    DeadRising,
    DeadRising2,
    LegoBatman,
    PlanetCrafter,
    FfResonance,
    Ff16,
    /// A table-driven screen (see `simple.rs`).
    Simple(u16),
}

impl Game {
    /// The game's name, for saves whose format is generic (JSON, XML, a database) so the
    /// file-type chip can say which game it is. `None` where the format already names it.
    pub fn name(self) -> Option<&'static str> {
        Some(match self {
            Game::Hades => "Hades",
            Game::InfiniteFusion => "Pok\u{00E9}mon Infinite Fusion",
            Game::Stardew => "Stardew Valley",
            Game::Minecraft => "Minecraft",
            Game::Balatro => "Balatro",
            Game::Silksong => "Hollow Knight: Silksong",
            Game::HollowKnight => "Hollow Knight",
            Game::SlayTheSpire => "Slay the Spire",
            Game::Pathfinder => "Pathfinder",
            Game::RenPy => "Ren'Py",
            Game::Phasmophobia => "Phasmophobia",
            Game::LethalCompany => "Lethal Company",
            Game::Repo => "R.E.P.O.",
            Game::Subnautica => "Subnautica",
            Game::DarkestDungeon2 => "Darkest Dungeon II",
            Game::GreenHell => "Green Hell",
            Game::X4 => "X4: Foundations",
            Game::Simple(i) => simple::game_name(i),
            _ => return None,
        })
    }

    /// Name of the tab that shows the screen.
    pub fn tab_label(self) -> &'static str {
        match self {
            Game::Pokemon | Game::InfiniteFusion => "Trainer",
            Game::TerrariaWorld => "World",
            Game::Terraria | Game::Minecraft | Game::Stardew => "Player",
            Game::Balatro | Game::SlayTheSpire => "Run",
            Game::Hades => "House",
            Game::RpgMaker => "Party",
            Game::Paradox => "Empire",
            Game::Civ6 => "Civilizations",
            Game::Diablo2 => "Character",
            Game::Pathfinder => "Party",
            Game::RimWorld | Game::OxygenNotIncluded => "Colony",
            Game::Subnautica | Game::ReSlot => "Slot",
            Game::GreenHell => "Survivor",
            Game::RenPy => "Story",
            Game::MassEffect => "Shepard",
            Game::Xcom | Game::Xcom2 => "HQ",
            Game::Ff7 | Game::Ff7Remake | Game::Ff9 => "Party",
            Game::HogwartsDb => "Character",
            Game::DyingLight2 | Game::DyingLight => "Survivor",
            Game::Witcher3 => "Geralt",
            Game::Cyberpunk => "V",
            Game::Kenshi => "Faction",
            Game::KenshiSquad => "Squad",
            Game::Palworld => "Player",
            Game::P3r => "Protagonist",
            Game::P4g => "Protagonist",
            Game::Ffx2 => "Gullwings",
            Game::P5r => "Phantom Thieves",
            Game::Ffx => "Party",
            Game::Metro => "Artyom",
            Game::Yakuza0 => "Kiryu & Majima",
            Game::YakuzaDe => "Player",
            Game::Kiwami => "Kiryu",
            Game::Ffxv => "Party",
            Game::DeathStranding2 => "Facilities",
            Game::DeathStranding1 => "Facilities",
            Game::Andromeda => "Pathfinder",
            Game::Gow2018 => "Kratos",
            Game::GowRagnarok => "Kratos",
            Game::Cairn => "Climber",
            Game::Borderlands | Game::Borderlands3 => "Vault Hunter",
            Game::Wonderlands => "Hero",
            Game::TruckSim => "Driver",
            Game::X4 => "Pilot",
            Game::Mgsv => "Commander",
            Game::TotalWar => "Campaign",
            Game::GrimDawn | Game::TitanQuest | Game::Torchlight2 => "Character",
            Game::Fallout12 => "Pip-Boy",
            Game::DeusEx => "JC Denton",
            Game::Kotor => "Party",
            Game::Infinity => "Party",
            Game::DragonAge => "Warden",
            Game::HalfLife | Game::HalfLife2 => "Player",
            Game::Silksong => "Hornet",
            Game::Deltarune => "Party",
            Game::VampireSurvivors => "Collection",
            Game::DarkestDungeon => "Hamlet",
            Game::DarkestDungeon2 => "Expedition",
            Game::Octopath => "Travellers",
            Game::Dos2 | Game::Bg3 => "Party",
            Game::Wasteland3 => "Rangers",
            Game::Valheim => "Viking",
            Game::GoldenSun => "Party",
            Game::Torment => "Party",
            Game::Starbound => "Character",
            Game::Expedition33 => "Party",
            Game::Fallout3Nv => "Character",
            Game::Nier => "YoRHa",
            Game::WorldOfHorror => "Unlocks",
            Game::Sims1 | Game::Sims2 => "Families",
            Game::Sims4 | Game::Sims3 => "Households",
            Game::Phasmophobia => "Ghost hunter",
            Game::LethalCompany => "Crew",
            Game::Repo => "Team",
            Game::DeadRising | Game::DeadRising2 => "Survivor",
            Game::LegoBatman => "Studs",
            Game::PlanetCrafter => "Terraformer",
            Game::FfResonance => "Party",
            Game::Ff16 => "Clive",
            Game::Simple(i) => simple::tab_label(i),
            _ => "Character",
        }
    }
}

/// The screen for this save, if it's a game that has one.
pub fn detect(kind: &str, root: &Value) -> Option<Game> {
    let has = |p: &str| root.pointer(p).is_some();
    Some(match kind {
        "Elden Ring character" => Game::EldenRing,
        "Dark Souls III character" => Game::DarkSouls3,
        "Dark Souls II character" => Game::DarkSouls2,
        "Dark Souls character" => Game::DarkSouls1,
        "Sekiro character" => Game::Sekiro,
        "Terraria player" => Game::Terraria,
        "Terraria world" => Game::TerrariaWorld,
        k if k.starts_with("Pok\u{00E9}mon") => Game::Pokemon,
        crate::fmt::essentials::KIND => Game::InfiniteFusion,
        "Fallout 1 save" | "Fallout 2 save" => Game::Fallout12,
        "Deus Ex map save" | "Deus Ex save info" => Game::DeusEx,
        k if (k.starts_with("Skyrim") || k.starts_with("Fallout") || k.starts_with("Enderal")) && has("/Save") => Game::Bethesda,
        _ if pathfinder::is_pathfinder(root) => Game::Pathfinder,
        _ if has("/GameState/Resources") => Game::Hades,
        _ if has("/SaveGame/player/farmName") => Game::Stardew,
        _ if has("/Data/Player/Inventory") => Game::Minecraft,
        _ if has("/GAME/dollars") && has("/cardAreas") => Game::Balatro,
        _ if indie::is_silksong(root) => Game::Silksong,
        _ if has("/playerData/geo") => Game::HollowKnight,
        _ if has("/current_health") && has("/relics") && has("/cards") => Game::SlayTheSpire,
        _ if has("/party/_stressEnergyCount") && simple::detect(kind, root).is_some() => return simple::detect(kind, root).map(Game::Simple),
        _ if has("/actors/_data/@a") && has("/party/_actors/@a") => Game::RpgMaker,
        "RE Engine save" if dashboards::ree_slots(root).is_some() => Game::Onimusha,
        "RE Engine save" if screens2::is_re_slot(root) => Game::ReSlot,
        ".NET BinaryFormatter stream" if screens2::is_green_hell(root) => Game::GreenHell,
        "XML" if x4::is_x4(root) => Game::X4,
        k if k == crate::fmt::mgsv::KIND => Game::Mgsv,
        _ if screens2::is_hogwarts(root) => Game::Hogwarts,
        "SQLite database" if screens2::is_hogwarts_db(root) => Game::HogwartsDb,
        k if screens2::is_renpy(k, root) => Game::RenPy,
        k if k.starts_with("Mass Effect") && has("/Shepard") => Game::MassEffect,
        "XCOM: Enemy Unknown game data" => Game::Xcom,
        "XCOM 2 game data" => Game::Xcom2,
        "Civilization VI save" => Game::Civ6,
        "Diablo II character" => Game::Diablo2,
        "Undertale save" => Game::Undertale,
        k if k.starts_with("Deltarune save") => Game::Deltarune,
        "Vampire Survivors save" => Game::VampireSurvivors,
        "Final Fantasy VII save" => Game::Ff7,
        "Final Fantasy IX save" => Game::Ff9,
        "Dying Light save" if has("/Group/Main/Cash") || has("/Bucket") => Game::DyingLight2,
        "Dying Light save (2015)" => Game::DyingLight,
        "The Witcher 3 game data" => Game::Witcher3,
        "Cyberpunk 2077 game data" => Game::Cyberpunk,
        "Kenshi save" => Game::Kenshi,
        "Kenshi squad" => Game::KenshiSquad,
        "Persona 3 Reload save" => Game::P3r,
        "Persona 4 Golden save" => Game::P4g,
        "Final Fantasy X-2 save" => Game::Ffx2,
        "Persona 5 Royal save" => Game::P5r,
        "Final Fantasy X save" => Game::Ffx,
        "Metro player save" => Game::Metro,
        "Yakuza 0 save" => Game::Yakuza0,
        "Yakuza Kiwami save" => Game::Kiwami,
        crate::fmt::ffxv::KIND => Game::Ffxv,
        crate::fmt::ds2::KIND => Game::DeathStranding2,
        crate::fmt::ds1::KIND => Game::DeathStranding1,
        crate::fmt::mea::KIND => Game::Andromeda,
        "God of War save" => Game::Gow2018,
        "God of War Ragnarök save" => Game::GowRagnarok,
        "Cairn save" => Game::Cairn,
        crate::fmt::borderlands::KIND_BL2 | crate::fmt::borderlands::KIND_TPS => Game::Borderlands,
        crate::fmt::borderlands3::KIND => Game::Borderlands3,
        crate::fmt::borderlands3::KIND_WL => Game::Wonderlands,
        crate::fmt::sii::KIND => Game::TruckSim,
        crate::fmt::totalwar::KIND => Game::TotalWar,
        "Grim Dawn character" => Game::GrimDawn,
        "Titan Quest character" => Game::TitanQuest,
        "Torchlight II character" => Game::Torchlight2,
        k if k.starts_with("Star Wars: KOTOR") => Game::Kotor,
        k if k.starts_with("Neverwinter Nights") => Game::Nwn,
        crate::fmt::infinity::KIND => Game::Infinity,
        crate::fmt::infinity::KIND_PST => Game::Torment,
        crate::fmt::dragonage::KIND => Game::DragonAge,
        crate::fmt::half_life::KIND_GOLDSRC => Game::HalfLife,
        crate::fmt::half_life::KIND_SOURCE => Game::HalfLife2,
        crate::fmt::half_life::KIND_ALYX => Game::HalfLife2,
        k if darkest::is_dd1(k) => Game::DarkestDungeon,
        "JSON" if darkest::dd2_file(root).is_some() => Game::DarkestDungeon2,
        crate::fmt::octopath::OT1 | crate::fmt::octopath::OT2 => Game::Octopath,
        crate::fmt::larian::DOS2 => Game::Dos2,
        crate::fmt::larian::BG3 => Game::Bg3,
        "Wasteland 3 save" => Game::Wasteland3,
        "Valheim character" => Game::Valheim,
        crate::fmt::starbound::KIND_PLAYER => Game::Starbound,
        crate::fmt::fos::KIND => Game::Fallout3Nv,
        crate::fmt::nier::KIND => Game::Nier,
        k if woh::is_woh(k, root) => Game::WorldOfHorror,
        crate::fmt::sims1::KIND => Game::Sims1,
        crate::fmt::sims2::KIND => Game::Sims2,
        crate::fmt::sims4::KIND => Game::Sims4,
        crate::fmt::sims3::KIND => Game::Sims3,
        crate::fmt::dead_rising::KIND_DR1 => Game::DeadRising,
        crate::fmt::dead_rising::KIND_DR2 | crate::fmt::dead_rising::KIND_OTR => Game::DeadRising2,
        crate::fmt::ttgames::KIND => Game::LegoBatman,
        crate::fmt::planet_crafter::KIND => Game::PlanetCrafter,
        crate::fmt::ffr::KIND => Game::FfResonance,
        crate::fmt::ff16::KIND => Game::Ff16,
        crate::fmt::goldensun::KIND_GS1 | crate::fmt::goldensun::KIND_GS2 => Game::GoldenSun,
        _ if coop::is_phasmophobia(root) => Game::Phasmophobia,
        _ if coop::is_lethal_company(root) => Game::LethalCompany,
        _ if coop::is_repo(root) => Game::Repo,
        _ if has("/base/m_title") && has("/player") => Game::YakuzaDe,
        // Yakuza 6 has no title in its save; its spearfishing minigame marks it.
        _ if has("/base/m_subtitle") && has("/player/point/m_point") && has("/mg_spearfishing") => Game::YakuzaDe,
        k if k.starts_with("Unreal Engine save (BP_SaveGameObject") && has("/CharactersCollection") && has("/Gold") => Game::Expedition33,
        "Unreal Engine save (PalWorldPlayerSaveGame)" if has("/SaveData/TechnologyPoint") => Game::Palworld,
        "Final Fantasy VII Remake save" | "Final Fantasy VII Rebirth save" => Game::Ff7Remake,
        "RimWorld save" => Game::RimWorld,
        "Oxygen Not Included save" => Game::OxygenNotIncluded,
        _ if has("/gameMode") && has("/changeSet") && has("/session") => Game::Subnautica,
        "Stellaris save" | "Hearts of Iron IV save" | "Europa Universalis IV save" | "Europa Universalis III save" | "Crusader Kings II save" | "Crusader Kings III save" | "Paradox save" => Game::Paradox,
        k if k.ends_with(" save (binary)") && crate::fmt::pdxbin::is_kind(k) => Game::Paradox,
        _ => return simple::detect(kind, root).map(Game::Simple),
    })
}

/// One save-wide action offered in the strip at the top of a screen.
#[derive(Clone, PartialEq)]
struct QuickAction {
    label: String,
    tip: String,
    enabled: bool,
    why_off: String,
}

const QA_NOW: &str = "quick_actions_now";
const QA_SHOWN: &str = "quick_actions_shown";
const QA_CLICK: &str = "quick_actions_click";

/// A save-wide action such as "Max crafting materials". It doesn't draw where it is called:
/// every screen lists these in one "Quick actions" strip at its top, so the player learns
/// where to look once. Returns true on the frame the player clicked it.
pub fn quick_button(ui: &Ui, label: impl AsRef<str>, tip: impl AsRef<str>) -> bool {
    quick_action(ui, label.as_ref(), tip.as_ref(), true, "")
}

/// Like [`quick_button`], greyed out (with `why_off` as the hover text) when `enabled` is false.
pub fn quick_action(ui: &Ui, label: &str, tip: &str, enabled: bool, why_off: &str) -> bool {
    let ctx = ui.ctx();
    let action = QuickAction { label: label.to_string(), tip: tip.to_string(), enabled, why_off: why_off.to_string() };
    ctx.data_mut(|d| {
        let list = d.get_temp_mut_or_default::<Vec<QuickAction>>(egui::Id::new(QA_NOW));
        if !list.iter().any(|a| a.label == action.label) {
            list.push(action);
        }
    });
    if !enabled {
        return false;
    }
    let id = egui::Id::new(QA_CLICK);
    let hit = ctx.data(|d| d.get_temp::<String>(id)).as_deref() == Some(label);
    if hit {
        ctx.data_mut(|d| d.remove_temp::<String>(id));
    }
    hit
}

/// Draws the strip from the actions the screen registered on the previous frame (a screen
/// registers them as it draws, so they show one frame later).
fn quick_strip(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let now: Vec<QuickAction> = ctx.data_mut(|d| d.remove_temp::<Vec<QuickAction>>(egui::Id::new(QA_NOW))).unwrap_or_default();
    let shown: Vec<QuickAction> = ctx.data(|d| d.get_temp(egui::Id::new(QA_SHOWN))).unwrap_or_default();
    // Anything not consumed last frame was for a button that is gone.
    ctx.data_mut(|d| d.remove_temp::<String>(egui::Id::new(QA_CLICK)));
    if now != shown {
        ctx.request_repaint();
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(QA_SHOWN), now));
    let list: Vec<QuickAction> = ctx.data(|d| d.get_temp(egui::Id::new(QA_SHOWN))).unwrap_or_default();
    if list.is_empty() {
        return;
    }
    theme::card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Quick actions")).family(theme::semibold_family()));
            for a in &list {
                let mut r = ui.add_enabled(a.enabled, egui::Button::new(tr_any(&a.label)));
                if !a.tip.is_empty() {
                    r = r.on_hover_text(tr_any(&a.tip));
                }
                if !a.enabled && !a.why_off.is_empty() {
                    r = r.on_disabled_hover_text(tr_any(&a.why_off));
                }
                if r.clicked() {
                    ctx.data_mut(|d| d.insert_temp(egui::Id::new(QA_CLICK), a.label.clone()));
                }
            }
        });
    });
    ui.add_space(8.0);
}

pub fn show(game: Game, ui: &mut Ui, cx: &mut GameCtx) {
    quick_strip(ui);
    show_screen(game, ui, cx);
}

fn show_screen(game: Game, ui: &mut Ui, cx: &mut GameCtx) {
    match game {
        Game::EldenRing | Game::DarkSouls3 | Game::DarkSouls2 | Game::DarkSouls1 | Game::Sekiro => fromsoft::show(game, ui, cx),
        Game::Terraria => terraria::show(ui, cx),
        Game::TerrariaWorld => terraria_world::show(ui, cx),
        Game::Pokemon => pokemon::show(ui, cx),
        Game::InfiniteFusion => infinite_fusion::show(ui, cx),
        Game::Bethesda => dashboards::bethesda(ui, cx),
        Game::Hades => dashboards::hades(ui, cx),
        Game::Stardew => dashboards::stardew(ui, cx),
        Game::Minecraft => dashboards::minecraft(ui, cx),
        Game::Balatro => dashboards::balatro(ui, cx),
        Game::HollowKnight => dashboards::hollow_knight(ui, cx),
        Game::SlayTheSpire => dashboards::slay_the_spire(ui, cx),
        Game::RpgMaker => dashboards::rpg_maker(ui, cx),
        Game::Onimusha => dashboards::onimusha(ui, cx),
        Game::RimWorld => dashboards::rimworld(ui, cx),
        Game::Subnautica => dashboards::subnautica(ui, cx),
        Game::OxygenNotIncluded => dashboards::oxygen_not_included(ui, cx),
        Game::GreenHell => screens2::green_hell(ui, cx),
        Game::ReSlot => screens2::re_slot(ui, cx),
        Game::Hogwarts => screens2::hogwarts(ui, cx),
        Game::HogwartsDb => screens2::hogwarts_db(ui, cx),
        Game::Ff9 => screens2::ff9(ui, cx),
        Game::DyingLight2 => screens2::dying_light2(ui, cx),
        Game::DyingLight => screens2::dying_light(ui, cx),
        Game::Witcher3 => screens2::witcher3(ui, cx),
        Game::Cyberpunk => screens2::cyberpunk(ui, cx),
        Game::Kenshi => screens2::kenshi(ui, cx),
        Game::KenshiSquad => screens2::kenshi_squad(ui, cx),
        Game::Palworld => screens2::palworld(ui, cx),
        Game::P3r => screens2::p3r(ui, cx),
        Game::P4g => screens2::p4g(ui, cx),
        Game::Ffx2 => screens2::ffx2(ui, cx),
        Game::P5r => screens2::p5r(ui, cx),
        Game::Ffx => screens2::ffx(ui, cx),
        Game::Metro => screens2::metro(ui, cx),
        Game::Yakuza0 => screens2::yakuza0(ui, cx),
        Game::YakuzaDe => screens2::yakuza_de(ui, cx),
        Game::Kiwami => screens2::kiwami(ui, cx),
        Game::Ffxv => screens2::ffxv(ui, cx),
        Game::DeathStranding2 => screens2::death_stranding2(ui, cx),
        Game::DeathStranding1 => screens2::death_stranding1(ui, cx),
        Game::Andromeda => screens2::andromeda(ui, cx),
        Game::Gow2018 => screens2::gow2018(ui, cx),
        Game::GowRagnarok => screens2::gow_ragnarok(ui, cx),
        Game::Cairn => cairn::show(ui, cx),
        Game::Borderlands => borderlands::show(ui, cx),
        Game::Borderlands3 => borderlands::show3(ui, cx),
        Game::Wonderlands => borderlands::show_wl(ui, cx),
        Game::TruckSim => trucks::show(ui, cx),
        Game::X4 => x4::show(ui, cx),
        Game::Mgsv => mgsv::show(ui, cx),
        Game::TotalWar => totalwar::show(ui, cx),
        Game::GrimDawn => arpg::grim_dawn(ui, cx),
        Game::TitanQuest => arpg::titan_quest(ui, cx),
        Game::Torchlight2 => arpg::torchlight2(ui, cx),
        Game::Fallout12 => fallout12::show(ui, cx),
        Game::DeusEx => deus_ex::show(ui, cx),
        Game::Kotor => kotor::show(ui, cx),
        Game::Nwn => nwn::show(ui, cx),
        Game::Infinity => infinity::show(ui, cx),
        Game::Torment => torment::show(ui, cx),
        Game::DragonAge => dragon_age::show(ui, cx),
        Game::HalfLife | Game::HalfLife2 => half_life::show(ui, cx),
        Game::DarkestDungeon => darkest::show_dd1(ui, cx),
        Game::DarkestDungeon2 => darkest::show_dd2(ui, cx),
        Game::Octopath => octopath::show(ui, cx),
        Game::Dos2 => larian::dos2(ui, cx),
        Game::Bg3 => larian::bg3(ui, cx),
        Game::Wasteland3 => survival::wasteland3(ui, cx),
        Game::Valheim => survival::valheim(ui, cx),
        Game::GoldenSun => golden_sun::show(ui, cx),
        Game::Starbound => starbound::show(ui, cx),
        Game::Expedition33 => expedition33::show(ui, cx),
        Game::Fallout3Nv => fallout3nv::show(ui, cx),
        Game::Nier => nier::show(ui, cx),
        Game::WorldOfHorror => woh::show(ui, cx),
        Game::Sims1 => sims::show(ui, cx, false),
        Game::Sims2 => sims::show(ui, cx, true),
        Game::Sims4 => sims::show4(ui, cx),
        Game::Sims3 => sims::show3(ui, cx),
        Game::Phasmophobia => coop::phasmophobia(ui, cx),
        Game::LethalCompany => coop::lethal_company(ui, cx),
        Game::Repo => coop::repo(ui, cx),
        Game::DeadRising => dead_rising::dead_rising(ui, cx),
        Game::DeadRising2 => dead_rising::dead_rising2(ui, cx),
        Game::LegoBatman => lego::show(ui, cx),
        Game::PlanetCrafter => planet_crafter::show(ui, cx),
        Game::FfResonance => ffr::show(ui, cx),
        Game::Ff16 => ff16::show(ui, cx),
        Game::Simple(i) => simple::show(i, ui, cx),
        Game::RenPy => screens2::renpy(ui, cx),
        Game::MassEffect => screens2::mass_effect(ui, cx),
        Game::Xcom => screens2::xcom(ui, cx),
        Game::Xcom2 => screens2::xcom2(ui, cx),
        Game::Civ6 => screens2::civ6(ui, cx),
        Game::Diablo2 => screens2::diablo2(ui, cx),
        Game::Pathfinder => pathfinder::show(ui, cx),
        Game::Silksong => indie::silksong(ui, cx),
        Game::Undertale => indie::undertale(ui, cx),
        Game::Deltarune => indie::deltarune(ui, cx),
        Game::VampireSurvivors => indie::vampire_survivors(ui, cx),
        Game::Ff7 => screens2::ff7(ui, cx),
        Game::Ff7Remake => screens2::ff7_remake(ui, cx),
        Game::Paradox => {
            let kind = cx.tree.map_or_else(String::new, |t| t.kind_name.clone());
            dashboards::paradox(ui, cx, &kind)
        }
    }
}

/// What a screen reads and where it puts its edits.
pub struct GameCtx<'a> {
    pub root: &'a Value,
    pub baseline: &'a Value,
    /// The structured document (editable values, drop-down lists); `None` for plain JSON,
    /// where every value can be edited.
    pub tree: Option<&'a TreeDoc>,
    pub edits: Vec<(String, Value)>,
    /// For saves inside an archive (a Cairn slot pack): the entry open now and all entries.
    /// The third item is the entry the game itself would load (Cairn: the newest save).
    pub archive: Option<(&'a str, &'a [String], Option<&'a str>)>,
    /// An entry of that archive the screen asked to open instead.
    pub open_entry: Option<String>,
}

impl GameCtx<'_> {
    pub fn get(&self, p: &str) -> Option<&Value> {
        self.root.pointer(p)
    }

    pub fn int(&self, p: &str) -> Option<i64> {
        let v = self.get(p)?;
        v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    }

    pub fn num(&self, p: &str) -> Option<f64> {
        let v = self.get(p)?;
        v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    }

    pub fn text(&self, p: &str) -> Option<String> {
        match self.get(p)? {
            Value::String(s) => Some(s.clone()),
            Value::Null => None,
            v => Some(widgets::display(v)),
        }
    }

    pub fn flag(&self, p: &str) -> bool {
        match self.get(p) {
            Some(Value::Bool(b)) => *b,
            Some(v) => v.as_i64().is_some_and(|n| n != 0) || v.as_str().is_some_and(|s| s == "true"),
            None => false,
        }
    }

    pub fn can_edit(&self, p: &str) -> bool {
        match self.tree {
            Some(t) => t.editable.contains_key(p),
            None => self.get(p).is_some_and(|v| !matches!(v, Value::Object(_) | Value::Array(_))),
        }
    }

    pub fn modified(&self, p: &str) -> bool {
        self.baseline.pointer(p) != self.root.pointer(p)
    }

    pub fn set(&mut self, p: &str, v: Value) {
        self.edits.push((p.to_string(), v));
    }

    /// Editor for one value (drop-down when the field has a fixed list). Nothing is shown
    /// when the value isn't in this save.
    pub fn editor(&mut self, ui: &mut Ui, p: &str, width: f32) {
        let Some(v) = self.get(p).cloned() else { return };
        let editable = self.can_edit(p);
        let choices = if editable { self.tree.and_then(|t| t.choices(p)) } else { None };
        // One group, so a number's box and its −/+ stay in one cell of a grid.
        let opts = widgets::EditOpts { tree: self.tree, baseline: Some(self.baseline), name: None, kind: self.tree.map(|t| t.kind_name.as_str()), show_in_tree: true };
        let out = ui
            .horizontal(|ui| match (&choices, &v) {
                (Some(list), Value::String(cur)) => widgets::choice_editor(ui, p, cur, list, width),
                _ => {
                    let o = widgets::value_editor_ex(ui, p, &v, editable, width, &opts);
                    if o.go_to_tree {
                        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(SHOW_IN_TREE), p.to_string()));
                    }
                    o.edit
                }
            })
            .inner;
        if let Some(nv) = out {
            self.set(p, nv);
        }
    }

    /// For table cells: the normal editor, or the compact one on a narrow screen.
    pub fn cell(&mut self, ui: &mut Ui, p: &str, width: f32) {
        if ui.ctx().screen_rect().width() < 640.0 {
            self.compact(ui, p, width.min(72.0));
        } else {
            self.editor(ui, p, width);
        }
    }

    /// An editor without −/+ buttons, for tables with many columns.
    pub fn compact(&mut self, ui: &mut Ui, p: &str, width: f32) {
        widgets::compact(|| self.editor(ui, p, width));
    }

    /// "Label   [editor]" for use inside a two-column `egui::Grid`.
    pub fn grid_row(&mut self, ui: &mut Ui, label: &str, p: &str, width: f32) {
        if self.get(p).is_none() {
            return;
        }
        self.label(ui, label, p);
        ui.horizontal(|ui| self.editor(ui, p, width));
        ui.end_row();
    }

    /// "Label [editor]" with the label cut to `label_w`, so columns of them line up.
    pub fn row_fixed(&mut self, ui: &mut Ui, label: &str, p: &str, label_w: f32, width: f32) {
        let label = tr_any(label);
        if self.get(p).is_none() {
            return;
        }
        // Never wider than the editor and its −/+ leave room for, so a narrow column doesn't push them out.
        let label_w = label_w.min((ui.available_width() - width - 84.0).max(56.0));
        ui.horizontal(|ui| {
            let c = if self.modified(p) { theme::MODIFIED } else { ui.visuals().text_color() };
            ui.allocate_ui_with_layout(egui::vec2(label_w, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_width(label_w);
                let r = ui.add(egui::Label::new(RichText::new(label).color(c)).truncate());
                find_mark(ui, r.rect, label);
            });
            self.editor(ui, p, width);
        });
    }

    /// A field's label, orange when it was changed.
    pub fn label(&self, ui: &mut Ui, label: &str, p: &str) {
        let label = tr_any(label);
        let c = if self.modified(p) { theme::MODIFIED } else { ui.visuals().text_color() };
        let r = ui.label(RichText::new(label).color(c));
        find_mark(ui, r.rect, label);
    }

    /// A compact tile: small caption, big value with its editor.
    pub fn tile(&mut self, ui: &mut Ui, caption: &str, p: &str, accent: Color32) {
        let caption = tr_any(caption);
        if self.get(p).is_none() {
            return;
        }
        let modified = self.modified(p);
        // Long numbers (money in the trillions) get a wider tile instead of being cut off.
        let digits = self.get(p).map(|v| v.to_string().len()).unwrap_or(0) as f32;
        let need = digits * 11.0 + 18.0;
        tile_frame_min(ui, accent, modified, need + 76.0, |ui| {
            let r = ui.add(egui::Label::new(RichText::new(caption).size(12.5).color(ui.visuals().weak_text_color())).truncate());
            find_mark(ui, r.rect, caption);
            ui.horizontal(|ui| {
                ui.style_mut().override_font_id = Some(FontId::proportional(18.0));
                // Whole numbers leave room for −/+; decimals use the whole width.
                let whole = self.get(p).is_some_and(|v| v.as_i64().is_some() || v.as_u64().is_some());
                let w = if whole { (ui.available_width() - 76.0).clamp(50.0, 104.0f32.max(need)) } else { (ui.available_width() - 8.0).clamp(60.0, 160.0f32.max(need)) };
                self.editor(ui, p, w);
            });
        });
    }

    /// An attribute line: name, value with −/+, and a bar towards `max`.
    pub fn attribute(&mut self, ui: &mut Ui, name: &str, p: &str, max: f64, color: Color32) {
        let name = tr_any(name);
        let Some(n) = self.num(p) else { return };
        ui.horizontal(|ui| {
            let c = if self.modified(p) { theme::MODIFIED } else { ui.visuals().strong_text_color() };
            ui.allocate_ui_with_layout(egui::vec2(110.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_width(110.0);
                let r = ui.add(egui::Label::new(RichText::new(name).color(c)).truncate());
                find_mark(ui, r.rect, name);
            });
            // Decimals need a wider box than whole numbers.
            self.editor(ui, p, if n.fract().abs() > 1e-9 { 84.0 } else { 52.0 });
            let w = (ui.available_width() - 8.0).max(30.0);
            bar(ui, w, (n / max) as f32, color, None);
        });
    }

    /// A difficulty (or game mode) selector: one button per level, writing the value in the
    /// type the save already uses (number or text). `levels` are (name, stored value).
    pub fn difficulty(&mut self, ui: &mut Ui, caption: &str, p: &str, levels: &[(&str, Value)]) {
        let caption = tr_any(caption);
        let Some(cur) = self.get(p).cloned() else { return };
        let editable = self.can_edit(p);
        let same = |a: &Value, b: &Value| match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => x == y,
            _ => a == b,
        };
        let modified = self.modified(p);
        let mut pick = None;
        ui.horizontal_wrapped(|ui| {
            let c = if modified { theme::MODIFIED } else { ui.visuals().text_color() };
            let r = ui.label(RichText::new(caption).color(c));
            find_mark(ui, r.rect, caption);
            for (name, v) in levels {
                let on = same(&cur, v);
                if ui.add_enabled(editable, egui::SelectableLabel::new(on, tr_any(name))).clicked() && !on {
                    pick = Some(v.clone());
                }
            }
            if !levels.iter().any(|(_, v)| same(&cur, v)) {
                ui.weak(trf("(now {0})", &[&(widgets::display(&cur))]));
            }
        });
        if let Some(v) = pick {
            self.set(p, v);
        }
    }

    /// A toggle chip for an on/off value.
    pub fn chip(&mut self, ui: &mut Ui, label: &str, p: &str, color: Color32) {
        let label = tr_any(label);
        let Some(v) = self.get(p).cloned() else { return };
        let on = self.flag(p);
        let editable = self.can_edit(p);
        let modified = self.modified(p);
        let fill = if on { color.gamma_multiply(0.85) } else { ui.visuals().widgets.inactive.bg_fill };
        let text_c = if on { Color32::WHITE } else { ui.visuals().weak_text_color() };
        let mut btn = egui::Button::new(RichText::new(label).color(text_c).size(13.0)).fill(fill).rounding(12.0);
        if modified {
            btn = btn.stroke(egui::Stroke::new(1.5_f32, theme::MODIFIED));
        }
        let r = ui.add_enabled(editable, btn);
        find_mark(ui, r.rect, label);
        if r.clicked() {
            // Keep the value's own type: true/false, 1/0 or "true"/"false".
            let nv = match v {
                Value::Bool(_) => Value::Bool(!on),
                Value::String(_) => Value::String((!on).to_string()),
                _ => Value::from(i64::from(!on)),
            };
            self.set(p, nv);
        }
    }
}

// ------------------------------------------------------------------ layout pieces

/// The header card: a coloured band with the character's name and a line under it.
pub fn hero(ui: &mut Ui, accent: Color32, title: &str, subtitle: &str, add: impl FnOnce(&mut Ui)) {
    let subtitle = tr_any(subtitle);
    let fill = blend(ui.visuals().faint_bg_color, accent, 0.18);
    let resp = egui::Frame::none().fill(fill).rounding(10.0).inner_margin(egui::Margin::symmetric(18.0, 14.0)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let r = ui.label(RichText::new(title).family(theme::semibold_family()).size(26.0).color(ui.visuals().strong_text_color()));
        find_mark(ui, r.rect, title);
        if !subtitle.is_empty() {
            ui.label(RichText::new(subtitle).size(14.0).color(theme::fg(ui, accent)));
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(add);
    });
    let r = resp.response.rect;
    ui.painter().rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(5.0, r.height())), egui::Rounding { nw: 10.0, sw: 10.0, ne: 0.0, se: 0.0 }, accent);
    ui.add_space(10.0);
}

/// A titled card.
pub fn panel(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    let title = tr_any(title);
    theme::card(ui, |ui| {
        let r = ui.add(egui::Label::new(RichText::new(title).family(theme::semibold_family()).size(16.0)).truncate());
        find_mark(ui, r.rect, title);
        ui.add_space(4.0);
        add(ui);
    });
    ui.add_space(8.0);
}

/// Background for boxes inside a panel: distinct from both the panel and the text fields.
pub fn inset_fill(ui: &Ui, accent: Color32) -> Color32 {
    blend(ui.visuals().faint_bg_color, accent, 0.10)
}

const TILE_W: f32 = 196.0;

pub fn tile_frame(ui: &mut Ui, accent: Color32, modified: bool, add: impl FnOnce(&mut Ui)) {
    tile_frame_min(ui, accent, modified, 0.0, add)
}

/// A tile at least `min` wide inside (when the row has room).
pub fn tile_frame_min(ui: &mut Ui, accent: Color32, modified: bool, min: f32, add: impl FnOnce(&mut Ui)) {
    // At least two tiles per row, even on a phone.
    let row = ui.max_rect().width();
    let tile_w = TILE_W.max(min).min((row / 2.0 - 30.0).max(110.0).max(min.min(row - 40.0)));
    // Tiles flow in rows; start a new row when this one is full (frames can't wrap by themselves).
    if ui.layout().main_wrap() && ui.available_size_before_wrap().x < tile_w + 24.0 {
        ui.end_row();
    }
    let stroke = if modified { egui::Stroke::new(1.5_f32, theme::MODIFIED) } else { egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.45)) };
    egui::Frame::none()
        .fill(inset_fill(ui, accent))
        .stroke(stroke)
        .rounding(8.0)
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.set_width(tile_w.min(ui.available_width()));
            ui.vertical(add);
        });
}

/// A read-only figure in the same style as `tile`.
pub fn fact(ui: &mut Ui, caption: &str, value: &str, accent: Color32) {
    let caption = tr_any(caption);
    tile_frame(ui, accent, false, |ui| {
        let r = ui.add(egui::Label::new(RichText::new(caption).size(12.5).color(ui.visuals().weak_text_color())).truncate());
        find_mark(ui, r.rect, caption);
        ui.label(RichText::new(value).size(18.0).color(ui.visuals().strong_text_color()));
    });
}

/// A horizontal meter (0..=1), with optional text drawn on it.
pub fn bar(ui: &mut Ui, width: f32, frac: f32, color: Color32, text: Option<&str>) {
    let h = if text.is_some() { 18.0 } else { 8.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, h), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, h / 2.0, ui.visuals().extreme_bg_color);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        p.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * f, h)), h / 2.0, color);
    }
    if let Some(t) = text {
        p.text(rect.center(), egui::Align2::CENTER_CENTER, t, FontId::proportional(12.0), Color32::WHITE);
    }
}

/// A labelled meter: "HP  [=====    ] 450 / 600".
pub fn meter(ui: &mut Ui, label: &str, cur: f64, max: f64, color: Color32) {
    let label = tr_any(label);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(70.0, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_width(70.0);
            let r = ui.label(RichText::new(label).strong());
            find_mark(ui, r.rect, label);
        });
        let w = (ui.available_width() - 4.0).max(60.0);
        let frac = if max > 0.0 { (cur / max) as f32 } else { 0.0 };
        bar(ui, w, frac, color, Some(&format!("{} / {}", fmt_num(cur), fmt_num(max))));
    });
}

/// Lays `n` items out in as many columns of at least `min_w` as fit.
pub fn columns(ui: &mut Ui, min_w: f32, n: usize, mut add: impl FnMut(&mut Ui, usize)) {
    if n == 0 {
        return;
    }
    let cols = ((ui.available_width() / min_w).floor() as usize).clamp(1, n);
    ui.columns(cols, |cs| {
        for i in 0..n {
            add(&mut cs[i % cols], i);
        }
    });
}

pub fn fmt_num(n: f64) -> String {
    if n.fract().abs() > 1e-9 {
        return format!("{n:.1}");
    }
    let s = (n.abs() as u64).to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0.0 {
        out.insert(0, '-');
    }
    out
}

pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// "MetaCardPointsCommon" → "Meta card points common".
pub fn words(key: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in key.chars() {
        if c == '_' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        out.push(if out.is_empty() { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() });
    }
    out
}

/// What a game screen's "Give upgrade materials" button does (for tests).
#[cfg(test)]
pub fn give_materials(game: Game, cx: &mut GameCtx) {
    fromsoft::give_materials(game, cx);
}

/// What a game screen's "Max crafting materials" button does (for tests); returns the
/// number of stacks raised.
#[cfg(test)]
pub fn max_crafting_materials(game: Game, cx: &mut GameCtx) -> usize {
    crafting::apply(game, cx)
}

/// The "max upgrade materials" button: `what` names the amount ("99 of each").
pub fn materials_button(ui: &mut Ui, what: &str, tip: &str) -> bool {
    quick_button(ui, trf("Max upgrade materials ({0})", &[&tr_any(what)]), tip)
}

// ------------------------------------------------------------------ item grids

/// One cell of an inventory grid.
pub struct Cell {
    pub title: String,
    pub count: Option<i64>,
    pub empty: bool,
    pub modified: bool,
    pub tint: Option<Color32>,
}

/// A grid of item cells; returns the index clicked. `selected` is highlighted.
pub fn cell_grid(ui: &mut Ui, cells: &[Cell], cols: usize, selected: Option<usize>, accent: Color32) -> Option<usize> {
    let gap = 4.0;
    let cols = cols.max(1).min(((ui.available_width() + gap) / 58.0).floor().max(1.0) as usize);
    let size = ((ui.available_width() - gap * (cols as f32 - 1.0)) / cols as f32).clamp(52.0, 96.0);
    let mut clicked = None;
    for (r, row) in cells.chunks(cols).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (c, cell) in row.iter().enumerate() {
                let i = r * cols + c;
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), Sense::click());
                let p = ui.painter();
                let base = cell.tint.unwrap_or(ui.visuals().extreme_bg_color);
                let fill = if resp.hovered() { blend(base, accent, 0.25) } else { base };
                p.rect_filled(rect, 6.0, fill);
                let stroke = if selected == Some(i) {
                    egui::Stroke::new(2.0_f32, accent)
                } else if cell.modified {
                    egui::Stroke::new(1.5_f32, theme::MODIFIED)
                } else {
                    egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color)
                };
                p.rect_stroke(rect, 6.0, stroke);
                if !cell.empty {
                    let galley = fit_title(ui, &cell.title, size);
                    let pos = rect.center() - galley.size() / 2.0 - egui::vec2(0.0, 4.0);
                    ui.painter().with_clip_rect(rect.shrink(2.0)).galley(pos, galley, Color32::WHITE);
                    if let Some(n) = cell.count.filter(|&n| n > 1) {
                        ui.painter().text(rect.right_bottom() - egui::vec2(4.0, 3.0), egui::Align2::RIGHT_BOTTOM, fmt_num(n as f64), FontId::proportional(11.0), theme::fg(ui, accent));
                    }
                }
                if !cell.empty {
                    find_mark(ui, rect, &cell.title);
                }
                let resp = resp.on_hover_text(if cell.empty { tr("(empty)").to_string() } else { cell.title.clone() });
                if resp.clicked() {
                    clicked = Some(i);
                }
            }
        });
        ui.add_space(gap - ui.spacing().item_spacing.y);
    }
    clicked
}

/// A cell's title, in the largest size (11 down to 8) where every word fits on a line and
/// the text leaves room for the count below it.
fn fit_title(ui: &Ui, title: &str, size: f32) -> std::sync::Arc<egui::Galley> {
    let color = ui.visuals().strong_text_color();
    let width = size - 6.0;
    let mut last = None;
    for pt in [11.0, 10.0, 9.0, 8.0] {
        let font = FontId::proportional(pt);
        let longest = title.split_whitespace().map(|w| ui.fonts(|f| f.layout_no_wrap(w.to_string(), font.clone(), color).size().x)).fold(0.0, f32::max);
        let galley = ui.painter().layout(title.to_string(), font, color, width);
        if longest <= width && galley.size().y <= size - 16.0 {
            return galley;
        }
        last = Some(galley);
    }
    last.unwrap()
}

/// Remembered selection for a grid (per save view).
pub fn selection(ui: &Ui, id: &str) -> Option<usize> {
    ui.data(|d| d.get_temp::<usize>(egui::Id::new(("game_sel", id))))
}

pub fn select(ui: &Ui, id: &str, i: Option<usize>) {
    ui.data_mut(|d| match i {
        Some(i) => d.insert_temp(egui::Id::new(("game_sel", id)), i),
        None => d.remove::<usize>(egui::Id::new(("game_sel", id))),
    });
}

/// A row of tabs; returns the chosen index (remembered under `id`).
pub fn tabs(ui: &mut Ui, id: &str, names: &[String]) -> usize {
    let mut cur = selection(ui, id).unwrap_or(0).min(names.len().saturating_sub(1));
    ui.horizontal_wrapped(|ui| {
        for (i, n) in names.iter().enumerate() {
            if ui.selectable_label(cur == i, tr_any(n)).clicked() {
                cur = i;
            }
        }
    });
    select(ui, id, Some(cur));
    cur
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_games_from_their_data() {
        assert_eq!(detect("Elden Ring character", &json!({})), Some(Game::EldenRing));
        assert_eq!(detect("Pok\u{00E9}mon Crystal", &json!({})), Some(Game::Pokemon));
        assert_eq!(detect("Lua table", &json!({"GAME": {"dollars": 4}, "cardAreas": {}})), Some(Game::Balatro));
        assert_eq!(detect("JSON", &json!({"playerData": {"geo": 1}})), Some(Game::HollowKnight));
        assert_eq!(detect("JSON", &json!({"playerData": {"geo": 1, "silkMax": 9, "ShellShards": 0}})), Some(Game::Silksong));
        assert_eq!(detect("Undertale save", &json!({})), Some(Game::Undertale));
        assert_eq!(detect("Deltarune save (Chapter 1)", &json!({})), Some(Game::Deltarune));
        assert_eq!(detect("Vampire Survivors save", &json!({})), Some(Game::VampireSurvivors));
        assert_eq!(detect("XML", &json!({"SaveGame": {"player": {"farmName": "x"}}})), Some(Game::Stardew));
        assert_eq!(detect("JSON", &json!({"actors": {"_data": {"@a": [null, {}]}}, "party": {"_actors": {"@a": [1]}}})), Some(Game::RpgMaker));
        assert_eq!(detect("JSON", &json!({"gold": 1})), None);
        assert_eq!(detect(crate::fmt::nier::KIND, &json!({})), Some(Game::Nier));
        assert_eq!(detect(crate::fmt::sims1::KIND, &json!({})), Some(Game::Sims1));
        assert_eq!(detect(crate::fmt::sims2::KIND, &json!({})), Some(Game::Sims2));
        assert_eq!(detect(crate::fmt::sims4::KIND, &json!({})), Some(Game::Sims4));
        assert_eq!(detect(crate::fmt::sims3::KIND, &json!({})), Some(Game::Sims3));
        let es3 = |n: i64| json!({"__type": "int", "value": n});
        assert_eq!(detect(crate::fmt::es3::KIND_PHASMOPHOBIA, &json!({"PlayersMoney": es3(5), "NewLevel": es3(2)})), Some(Game::Phasmophobia));
        assert_eq!(detect("JSON", &json!({"GroupCredits": es3(60), "ProfitQuota": es3(130)})), Some(Game::LethalCompany));
        assert_eq!(detect("JSON", &json!({"PlayerXPNum": es3(60), "PlayerLevel": es3(1)})), Some(Game::LethalCompany));
        assert_eq!(detect("JSON", &json!({"dictionaryOfDictionaries": {"__type": "d", "value": {"runStats": {"level": 1}}}})), Some(Game::Repo));
        assert_eq!(detect("JSON", &json!({"GroupCredits": 60})), None);
        let y6 = json!({"base": {"m_subtitle": "chapter1"}, "player": {"point": {"m_point": [0, 5000]}}, "mg_spearfishing": {}});
        assert_eq!(detect("Yakuza save (Dragon Engine: Yakuza 6)", &y6), Some(Game::YakuzaDe));
        assert_eq!(detect("JSON", &json!({"base": {"m_subtitle": "x"}, "player": {"point": {"m_point": []}}})), None);
        assert_eq!(detect("Mass Effect 3 save", &json!({"Shepard": {"Level": 60}})), Some(Game::MassEffect));
        assert_eq!(detect("XCOM: Enemy Unknown game data", &json!({})), Some(Game::Xcom));
        assert_eq!(detect("XCOM 2 game data", &json!({})), Some(Game::Xcom2));
        assert_eq!(detect("Civilization VI save", &json!({})), Some(Game::Civ6));
        assert_eq!(detect("Diablo II character", &json!({})), Some(Game::Diablo2));
        assert_eq!(detect("Divinity: Original Sin 2 save", &json!({})), Some(Game::Dos2));
        assert_eq!(detect("Baldur's Gate 3 save", &json!({})), Some(Game::Bg3));
        assert_eq!(detect("JSON", &json!({"player": {"Money": 1}, "party": {"m_EntityData": []}})), Some(Game::Pathfinder));
        assert_eq!(detect("Borderlands: The Pre-Sequel character", &json!({})), Some(Game::Borderlands));
        assert_eq!(detect("Borderlands 3 character", &json!({})), Some(Game::Borderlands3));
        assert_eq!(detect("Tiny Tina's Wonderlands character", &json!({})), Some(Game::Wonderlands));
        assert_eq!(detect(crate::fmt::sii::KIND, &json!({})), Some(Game::TruckSim));
        assert_eq!(detect(crate::fmt::mgsv::KIND, &json!({})), Some(Game::Mgsv));
        assert_eq!(detect(crate::fmt::totalwar::KIND, &json!({})), Some(Game::TotalWar));
        assert_eq!(detect("XML", &json!({"savegame": {"info": {"player": {"@money": 1}, "game": {"@build": 1}}, "factions": {}}})), Some(Game::X4));
        assert_eq!(detect("Fallout 2 save", &json!({"Save": {}})), Some(Game::Fallout12));
        assert_eq!(detect("Deus Ex map save", &json!({})), Some(Game::DeusEx));
        assert_eq!(detect(crate::fmt::infinity::KIND, &json!({})), Some(Game::Infinity));
        assert_eq!(detect(crate::fmt::infinity::KIND_PST, &json!({})), Some(Game::Torment));
        assert_eq!(detect(crate::fmt::dragonage::KIND, &json!({})), Some(Game::DragonAge));
        assert_eq!(detect(crate::fmt::half_life::KIND_GOLDSRC, &json!({})), Some(Game::HalfLife));
        assert_eq!(detect(crate::fmt::half_life::KIND_SOURCE, &json!({})), Some(Game::HalfLife2));
        assert_eq!(detect(crate::fmt::half_life::KIND_ALYX, &json!({})), Some(Game::HalfLife2));
        assert_eq!(detect("Final Fantasy VII save", &json!({})), Some(Game::Ff7));
        assert_eq!(detect("Final Fantasy VII Rebirth save", &json!({})), Some(Game::Ff7Remake));
        assert_eq!(detect(".NET BinaryFormatter stream", &json!([{"m_Major": 1}, {"PCM_HP": 90.0}])), Some(Game::GreenHell));
        assert_eq!(detect("RE Engine save", &json!({"#1": {"_Data": {"_InventoryData": []}}})), Some(Game::ReSlot));
        assert_eq!(detect("Unreal Engine save (PersistentGameData)", &json!({"CharacterSaveGameInfo": {"CharacterHouse": "Ravenclaw"}})), Some(Game::Hogwarts));
        assert_eq!(detect("Ren'Py save", &json!({"Variables": {"money": 5}})), Some(Game::RenPy));
        assert_eq!(detect("Grim Dawn character", &json!({})), Some(Game::GrimDawn));
        assert_eq!(detect("Titan Quest character", &json!({})), Some(Game::TitanQuest));
        assert_eq!(detect("Torchlight II character", &json!({})), Some(Game::Torchlight2));
        assert_eq!(detect("Star Wars: KOTOR II party table", &json!({})), Some(Game::Kotor));
        assert_eq!(detect("Neverwinter Nights character", &json!({})), Some(Game::Nwn));
        assert_eq!(detect("Darkest Dungeon estate", &json!({})), Some(Game::DarkestDungeon));
        assert_eq!(detect("Darkest Dungeon save file", &json!({})), None);
        assert_eq!(detect("JSON", &json!({"run_values": {"values": {}}, "inventory": {"m_items": []}})), Some(Game::DarkestDungeon2));
        assert_eq!(detect("Octopath Traveler II save", &json!({})), Some(Game::Octopath));
        assert_eq!(detect("Wasteland 3 save", &json!({})), Some(Game::Wasteland3));
        assert_eq!(detect("Valheim character", &json!({})), Some(Game::Valheim));
        assert_eq!(detect("Golden Sun save", &json!({})), Some(Game::GoldenSun));
        assert_eq!(detect("Golden Sun: The Lost Age save", &json!({})), Some(Game::GoldenSun));
    }

    #[test]
    fn quick_actions_register_and_report_clicks() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("en"));
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let run = |ctx: &egui::Context, click: Option<&str>| {
            let mut got = false;
            let _ = ctx.run(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    quick_strip(ui);
                    if let Some(c) = click {
                        ctx.data_mut(|d| d.insert_temp(egui::Id::new(QA_CLICK), c.to_string()));
                    }
                    got = quick_button(ui, tr("Max gold"), tr("tip"));
                });
            });
            got
        };
        assert!(!run(&ctx, None), "no click, no result");
        let shown: Vec<QuickAction> = ctx.data(|d| d.get_temp(egui::Id::new(QA_NOW))).unwrap_or_default();
        assert_eq!(shown.len(), 1, "the screen registered its action");
        assert!(run(&ctx, Some("Max gold")), "a click on the strip reaches the screen's button");
        assert!(!run(&ctx, None), "and is reported once");
    }

    #[test]
    fn find_marks_count_matches() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        let _ = ctx.run(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() }, |ctx| {
            begin_find(ctx, " Lock", 0);
            egui::CentralPanel::default().show(ctx, |ui| {
                for label in ["Lockpick", "Sneak", "Unlocking tools"] {
                    let r = ui.label(label);
                    find_mark(ui, r.rect, label);
                }
            });
        });
        assert_eq!(ctx.data(|d| d.get_temp::<usize>(egui::Id::new(FIND_SEEN))), Some(2));
        begin_find(&ctx, "", 0);
        assert_eq!(ctx.data(|d| d.get_temp::<usize>(egui::Id::new(FIND_COUNT))), Some(2), "last frame's count is kept for the find box");
    }

    #[test]
    fn friendly_words() {
        assert_eq!(words("MetaCardPointsCommon"), "Meta card points common");
        assert_eq!(words("current_health"), "Current health");
        assert_eq!(fmt_num(1234567.0), "1,234,567");
        assert_eq!(fmt_num(-1500.0), "-1,500");
    }
}
