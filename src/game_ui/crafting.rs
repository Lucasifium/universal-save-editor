//! "Max crafting materials": raises every crafting material already in the save to the
//! most one stack holds in that game. Nothing new is added and nothing else is touched.
//!
//! Each game has its own list, made from public sources (the games' wikis) and kept to
//! plain materials: no quest or story items, no boss summons or boss drops, no keys,
//! no critters, nothing whose possession the game reacts to. Where no public source states a
//! game's stack size, the cap is an estimate from community guides and editors; each such
//! cap says so where it is defined, and the button's hover text says so too.

use super::{Game, GameCtx};
use crate::json_view::pointer_escape;
use eframe::egui::{self, Ui};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// One stack of a crafting material: where its count is (some saves keep it twice) and
/// how far it can go.
#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    pub paths: Vec<String>,
    pub name: String,
    pub have: i64,
    pub cap: i64,
}

/// What the button does in `game`, for its hover text and the catalog.
pub fn about(game: Game) -> &'static str {
    match game {
        Game::Terraria => {
            "Raises wood, ores, bars, gems, monster drops, souls, fragments, herbs and other crafting materials \
             you already carry or store to 9999, Terraria's stack size for them since 1.4.4. Critters, fish, \
             boss summons and coins are left alone."
        }
        Game::Stardew => {
            "Raises the wood, stone, hardwood, fiber, clay, sap, coal, ores, bars, refined quartz, battery packs, \
             bone fragments and monster loot (slime, bat wings, solar and void essence, bug meat) in your \
             backpack to 999, the game's stack size. Quest items, minerals, gems and artifacts are left alone."
        }
        Game::Valheim => {
            "Raises wood, stone, flint, resin, coal, ores, scrap, metal bars, hides, pelts and other plain \
             materials in the inventory to their stack size in the current game (30 for ores and metals, 50 \
             for most others). Boss drops, summoning items, key fragments and food are left alone. Full stacks \
             of heavy materials can leave you over-encumbered."
        }
        Game::Minecraft => {
            "Raises ingots, nuggets, raw ores, gems, redstone, coal, string, leather, feathers, bones, gunpowder, \
             blaze rods and other crafting materials in the inventory and ender chest to 64. Ender pearls, eyes \
             of ender, nether stars and other progression items are left alone. Stacks of one item may have no \
             count in newer saves and aren't changed."
        }
        Game::Nier => {
            "Raises every material in the Materials list (ores, alloys, screws, gears, cables, batteries, machine parts, \
             gems, gadgets, seeds, meats and the rest) to 99, the most one slot holds. Viscous Oil (a quest item) and \
             the Children's Cores (a story item) aren't in the list and are left alone, as are recovery items and key items."
        }
        Game::EldenRing => {
            "Raises the crafting materials you carry or keep in the chest (meat, bones, flowers, mushrooms, \
             butterflies, stones and the rest of the base game's cookbook materials) to 999, the most a slot \
             holds. Smithing stones and gloveworts have their own button; key items are left alone."
        }
        Game::HogwartsDb => {
            "Raises every potion ingredient, plant ingredient, beast by-product and Moonstone in your resource \
             bag to 999 (an estimate: the game shows no stack limit above that). Gold, seeds and potions are \
             left alone."
        }
        Game::Gow2018 => {
            "Raises the crafting resources this save has found (Dust of the Realms, the Svartalfheim steels, \
             realm-specific ores and remnants and the rest) to 999 in each save slot, an estimate since the \
             game states no limit. Frozen Flame and Chaos Flame are story-locked and left alone, as are \
             Yggdrasil's Dew and other stat items."
        }
        Game::GowRagnarok => {
            "Raises the crafting resources this save has found (embers, timber, metals, remnants, crystals, \
             leather, bones and the realm materials) to 999, an estimate since the game states no limit. \
             Yggdrasil's Dew, currencies and quest items are left alone."
        }
        Game::Andromeda => {
            "Raises the minerals, biological materials and faction materials in Ryder's inventory to 999, \
             the resource limit in the game's settings. Research data, salvage and augmentations are left alone."
        }
        Game::DeathStranding2 => {
            "Raises the chiral crystals, resins, metals, ceramics, chemicals and special alloys stored at every \
             facility you've found to 9999 each, an estimate since the game states no facility limit."
        }
        Game::Metro => {
            "Raises Artyom's crafting metal and chemicals (Metro Exodus) to 150 each, an estimate of the most \
             he can carry on Normal; the game may lower them to its own limit when it loads."
        }
        Game::Ffxv => {
            "Raises every treasure (the materials used for elemancy and weapon upgrades) to 99, the most one \
             item stacks to. Key items, food and recovery items are left alone."
        }
        Game::Kotor => {
            "Raises KOTOR II's components and chemicals (what the workbench and lab station craft with) to \
             9999 each, an estimate since the game states no limit."
        }
        Game::HalfLife2 => {
            "Raises Alyx's resin (what the upgrade stations build with) to 999, an estimate since the game \
             states no limit."
        }
        Game::Starbound => {
            "Raises ores, bars, fabric, leather, silk, plant fibre, wire, glass and other crafting materials in \
             every bag to 1000, the game's usual stack size for them. Quest items, fuel and currencies are left alone."
        }
        Game::Expedition33 => {
            "Raises every kind of Chroma Catalyst (the weapon upgrade materials) to 999, an estimate since the \
             game states no limit. Lumina, Recoat and story items are left alone."
        }
        _ => "",
    }
}

/// The material stacks in this save, or why the button isn't offered for it.
pub fn stacks(game: Game, cx: &GameCtx) -> Result<Vec<Stack>, &'static str> {
    match game {
        Game::Terraria => terraria(cx),
        Game::Stardew => Ok(stardew(cx)),
        Game::Valheim => Ok(valheim(cx)),
        Game::Minecraft => Ok(minecraft(cx)),
        Game::Nier => Ok(nier(cx)),
        Game::EldenRing => Ok(elden_ring(cx)),
        Game::HogwartsDb => Ok(hogwarts(cx)),
        Game::Gow2018 => Ok(gow2018(cx)),
        Game::GowRagnarok => Ok(gow_ragnarok(cx)),
        Game::Andromeda => Ok(andromeda(cx)),
        Game::DeathStranding2 => Ok(death_stranding2(cx)),
        Game::Metro => Ok(metro(cx)),
        Game::Ffxv => Ok(ffxv(cx)),
        Game::Kotor => some(fixed(cx, &[("/Party/Components", "Components"), ("/Party/Chemicals", "Chemicals")], KOTOR2_CAP), "Only KOTOR II's party table has components and chemicals."),
        Game::HalfLife2 => some(fixed(cx, &[("/Player/Resin", "Resin")], ALYX_RESIN_CAP), "Only Half-Life: Alyx has a crafting material (resin)."),
        Game::Starbound => Ok(starbound(cx)),
        Game::Expedition33 => Ok(expedition33(cx)),
        _ => Err("This game has no crafting materials list."),
    }
}

/// Raises every material stack below its cap. Returns how many stacks changed.
pub fn apply(game: Game, cx: &mut GameCtx) -> usize {
    let Ok(list) = stacks(game, cx) else { return 0 };
    let mut n = 0;
    for s in list.iter().filter(|s| s.have < s.cap) {
        for p in &s.paths {
            cx.set(p, Value::from(s.cap));
        }
        n += 1;
    }
    n
}

/// The "Max crafting materials" button (or a note on why it isn't offered).
pub fn button(game: Game, ui: &mut Ui, cx: &mut GameCtx) {
    match stacks(game, cx) {
        Err(why) => {
            ui.weak(why);
        }
        Ok(list) => {
            let low = list.iter().filter(|s| s.have < s.cap).count();
            let label = match low {
                0 => "Max crafting materials".to_string(),
                1 => "Max crafting materials (1 stack)".to_string(),
                n => format!("Max crafting materials ({n} stacks)"),
            };
            let why_off = if list.is_empty() { "No crafting materials in this save." } else { "Every crafting material is already a full stack." };
            let r = ui.add_enabled(low > 0, egui::Button::new(label)).on_hover_text(about(game)).on_disabled_hover_text(why_off);
            if r.clicked() {
                apply(game, cx);
            }
        }
    }
}

/// The button, only where this save has the game's crafting materials list (nothing otherwise).
pub fn offer(game: Game, ui: &mut Ui, cx: &mut GameCtx) {
    if stacks(game, cx).is_ok() {
        button(game, ui, cx);
    }
}

fn object_keys(cx: &GameCtx, p: &str) -> Vec<String> {
    match cx.get(p) {
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        _ => vec![],
    }
}

// ------------------------------------------------------------------ Terraria

/// Terraria's first file version with 9999-item stacks for materials (1.4.4).
pub const TERRARIA_9999: i64 = 269;

/// Item ids of plain crafting materials (the wiki's items whose only type is crafting
/// material, ore, bar or gem), less critters, fish and unobtainable or inactive items;
/// plus the seven woods.
const TERRARIA_IDS: [i32; 144] = [
    9, 619, 620, 621, 911, 2503, 2504, 11, 12, 13, 14, 19, 20, 21, 22, 38, 56, 57, 60, 68, 69, 86, 116, 117, 118, 173, 174, 175, 177, 178, 179, 180, 181, 182, 209, 210, 225, 236, 254, 255,
    259, 313, 314, 315, 316, 317, 318, 319, 320, 323, 324, 331, 362, 364, 365, 366, 381, 382, 391, 501, 520, 521, 522, 526, 527, 528, 531, 547, 548, 549,
    575, 699, 700, 701, 702, 703, 704, 705, 706, 880, 947, 981, 999, 1006, 1104, 1105, 1106, 1113, 1115, 1116, 1117, 1118, 1119, 1184, 1191, 1198, 1225,
    1257, 1328, 1329, 1330, 1332, 1339, 1345, 1346, 1347, 1348, 1432, 1508, 1516, 1517, 1518, 1519, 1520, 1521, 1552, 1570, 1611, 1811, 1831, 2161, 2218,
    2358, 2431, 2607, 2766, 2887, 3111, 3186, 3354, 3355, 3356, 3456, 3457, 3458, 3459, 3460, 3467, 3783, 3794, 4412, 4413, 4414, 5070,
];

fn terraria_names() -> &'static HashSet<String> {
    static S: OnceLock<HashSet<String>> = OnceLock::new();
    S.get_or_init(|| TERRARIA_IDS.iter().map(|&i| crate::fmt::terraria::item_name(i)).collect())
}

fn terraria(cx: &GameCtx) -> Result<Vec<Stack>, &'static str> {
    match cx.int("/Character/File version") {
        Some(v) if v >= TERRARIA_9999 => {}
        _ => return Err("Max crafting materials needs a character saved by Terraria 1.4.4 or later (older versions stack materials lower)."),
    }
    let names = terraria_names();
    let mut out = vec![];
    for store in ["Inventory", "Piggy bank", "Safe", "Defender's Forge", "Void Vault"] {
        let base = format!("/{}", pointer_escape(store));
        for k in object_keys(cx, &base) {
            let p = format!("{base}/{}", pointer_escape(&k));
            let item = cx.text(&format!("{p}/Item")).unwrap_or_default();
            let stack = format!("{p}/Stack");
            if let (true, Some(have)) = (names.contains(&item), cx.int(&stack)) {
                if have > 0 {
                    out.push(Stack { paths: vec![stack], name: item, have, cap: 9999 });
                }
            }
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ Stardew Valley

/// Materials by name, with the category the game files them under (checked on saves
/// from 1.5); all stack to 999.
const STARDEW: [(&str, i64); 23] = [
    ("Wood", -16),
    ("Stone", -16),
    ("Hardwood", -16),
    ("Fiber", -16),
    ("Clay", -16),
    ("Battery Pack", -16),
    ("Sap", -81),
    ("Coal", -15),
    ("Copper Ore", -15),
    ("Iron Ore", -15),
    ("Gold Ore", -15),
    ("Iridium Ore", -15),
    ("Copper Bar", -15),
    ("Iron Bar", -15),
    ("Gold Bar", -15),
    ("Iridium Bar", -15),
    ("Refined Quartz", -15),
    ("Bone Fragment", -15),
    ("Slime", -28),
    ("Bat Wing", -28),
    ("Solar Essence", -28),
    ("Void Essence", -28),
    ("Bug Meat", -28),
];

fn stardew(cx: &GameCtx) -> Vec<Stack> {
    let base = "/SaveGame/player/items/Item";
    let slots: Vec<String> = match cx.get(base) {
        Some(Value::Array(a)) => (0..a.len()).map(|i| format!("{base}/{i}")).collect(),
        Some(Value::Object(_)) => vec![base.to_string()],
        _ => vec![],
    };
    let mut out = vec![];
    for p in slots {
        if cx.text(&format!("{p}/@xsi:type")).as_deref() != Some("Object") {
            continue;
        }
        let name = cx.text(&format!("{p}/name")).unwrap_or_default();
        let Some(&(_, cat)) = STARDEW.iter().find(|(n, _)| *n == name) else { continue };
        if cx.int(&format!("{p}/category")).is_some_and(|c| c != cat) || cx.flag(&format!("{p}/questItem")) || cx.flag(&format!("{p}/bigCraftable")) {
            continue;
        }
        let paths: Vec<String> = ["stack", "Stack"].iter().map(|k| format!("{p}/{k}")).filter(|q| cx.int(q).is_some()).collect();
        let Some(have) = paths.first().and_then(|q| cx.int(q)) else { continue };
        out.push(Stack { paths, name, have, cap: 999 });
    }
    out
}

// ------------------------------------------------------------------ NieR:Automata

/// The reader already files only plain materials under "Materials" (see `fmt::nier::MATERIALS`).
fn nier(cx: &GameCtx) -> Vec<Stack> {
    object_keys(cx, "/Materials")
        .into_iter()
        .filter_map(|k| {
            let p = format!("/Materials/{}", pointer_escape(&k));
            Some(Stack { have: cx.int(&p)?, paths: vec![p], name: k, cap: 99 })
        })
        .collect()
}

/// One stack per value at `p` that holds a number.
fn single(cx: &GameCtx, p: String, name: String, cap: i64) -> Option<Stack> {
    Some(Stack { have: cx.int(&p)?, paths: vec![p], name, cap })
}

/// The number values under the object at `base` whose key passes `keep`.
fn counts_under(cx: &GameCtx, base: &str, cap: i64, keep: impl Fn(&str) -> bool) -> Vec<Stack> {
    object_keys(cx, base)
        .into_iter()
        .filter(|k| keep(k))
        .filter_map(|k| single(cx, format!("{base}/{}", pointer_escape(&k)), k, cap))
        .collect()
}

/// `list`, or `why` when it's empty (the save has none of the game's fixed material counts).
fn some(list: Vec<Stack>, why: &'static str) -> Result<Vec<Stack>, &'static str> {
    if list.is_empty() {
        Err(why)
    } else {
        Ok(list)
    }
}

/// Fixed paths that hold one count each.
fn fixed(cx: &GameCtx, paths: &[(&str, &str)], cap: i64) -> Vec<Stack> {
    paths.iter().filter_map(|(p, n)| single(cx, p.to_string(), n.to_string(), cap)).collect()
}

// ------------------------------------------------------------------ Elden Ring

/// The base game's crafting materials (cookbook ingredients). Elden Ring lets a slot hold 999
/// of each, in the pouch and in the chest.
const ELDEN_RING: [&str; 67] = [
    "Sliver of Meat", "Beast Liver", "Lump of Flesh", "Beast Blood", "Old Fang", "Budding Horn", "Flight Pinion",
    "Four-Toed Fowl Foot", "Turtle Neck Meat", "Human Bone Shard", "Great Dragonfly Head", "Slumbering Egg", "Crab Eggs",
    "Land Octopus Ovary", "Miranda Powder", "Strip of White Flesh", "Thin Beast Bones", "Hefty Beast Bone", "String",
    "Living Jar Shard", "Albinauric Bloodclot", "Stormhawk Feather", "Poisonbloom", "Trina's Lily", "Fulgurbloom",
    "Miquella's Lily", "Grave Violet", "Faded Erdleaf Flower", "Erdleaf Flower", "Altus Bloom", "Fire Blossom",
    "Golden Sunflower", "Tarnished Golden Sunflower", "Herba", "Arteria Leaf", "Dewkissed Herba", "Rowa Fruit",
    "Golden Rowa", "Rimed Rowa", "Bloodrose", "Eye of Yelough", "Crystal Bud", "Rimed Crystal Bud", "Sacramental Bud",
    "Mushroom", "Melted Mushroom", "Toxic Mushroom", "Root Resin", "Cracked Crystal", "Sanctuary Stone",
    "Nascent Butterfly", "Aeonian Butterfly", "Smoldering Butterfly", "Silver Firefly", "Gold Firefly",
    "Glintstone Firefly", "Golden Centipede", "Silver Tear Husk", "Gold-Tinged Excrement", "Blood-Tainted Excrement",
    "Cave Moss", "Budding Cave Moss", "Crystal Cave Moss", "Yellow Ember", "Volcanic Stone", "Formic Rock",
    "Gravel Stone",
];

fn elden_ring(cx: &GameCtx) -> Vec<Stack> {
    let mut out = vec![];
    for list in ["/Items", "/Storage"] {
        for k in object_keys(cx, list) {
            let p = format!("{list}/{}", pointer_escape(&k));
            let item = cx.text(&format!("{p}/Item")).unwrap_or_default();
            if !ELDEN_RING.contains(&item.as_str()) {
                continue;
            }
            if let Some(s) = single(cx, format!("{p}/Quantity"), item, 999).filter(|s| s.have > 0) {
                out.push(s);
            }
        }
    }
    out
}

// ------------------------------------------------------------------ Hogwarts Legacy

/// Estimate: no public source states a stack limit for ingredients or Moonstone, and the
/// inventory shows counts up to 999.
const HOGWARTS_CAP: i64 = 999;

fn hogwarts(cx: &GameCtx) -> Vec<Stack> {
    let Some(Value::Array(rows)) = cx.get("/InventoryDynamic") else { return vec![] };
    let field = |r: &Value, k: &str| r.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    rows.iter()
        .enumerate()
        .filter(|(_, r)| field(r, "CharacterID") == "Player0" && field(r, "HolderID") == "ResourceInventory")
        .filter_map(|(i, r)| {
            let id = field(r, "ItemID");
            let low = id.to_ascii_lowercase();
            if id.is_empty() || low == "knuts" || ["seed", "potion", "quest"].iter().any(|w| low.contains(w)) {
                return None;
            }
            single(cx, format!("/InventoryDynamic/{i}/Count"), id, HOGWARTS_CAP)
        })
        .collect()
}

// ------------------------------------------------------------------ God of War (2018)

/// Estimate: the game states no limit for crafting resources.
const GOW_CAP: i64 = 999;

/// Resources the 2018 game files with the crafting materials but that aren't plain materials:
/// the two story-locked upgrade flames and the stat-boosting dews.
fn gow2018_skip(name: &str) -> bool {
    matches!(name, "Frozen Flame" | "Chaos Flame") || name.starts_with("Yggdrasil's Dew")
}

fn gow2018(cx: &GameCtx) -> Vec<Stack> {
    object_keys(cx, "")
        .into_iter()
        .flat_map(|slot| counts_under(cx, &format!("/{}/Resources", pointer_escape(&slot)), GOW_CAP, |k| !gow2018_skip(k)))
        .collect()
}

// ------------------------------------------------------------------ God of War Ragnarök

/// Crafting materials from public resource lists (embers, timber, metals, remnants, crystals,
/// leather, bones and realm materials). Resources not named here (quest items, Valhalla
/// currencies, food) are left alone.
const GOW_RAGNAROK: [&str; 33] = [
    "Glowing Embers", "Smoldering Embers", "Blazing Embers", "Sovereign Coals", "Stonewood", "Ydalir Timber",
    "Folkvangr Whetstone", "Dwarven Steel", "Forged Iron", "Honed Metal", "Asgardian Ingot", "Luminous Alloy",
    "Tempered Remnants", "Hardened Remnants", "Fortified Remnants", "Crystalline Fragment", "Crystalline Shard",
    "Purified Crystalline", "Sparkling Crystal", "Gleaming Crystal", "Shining Crystal", "Bonded Leather", "Rawhide",
    "Beast Scraps", "Beast Bones", "Slag Deposits", "Petrified Bones", "Celestial Fossil", "Whispering Slab",
    "Nidavellir Core", "Dragon Claw", "Aegir's Gold", "Skap Slag",
];

fn gow_ragnarok(cx: &GameCtx) -> Vec<Stack> {
    counts_under(cx, "/Resources", GOW_CAP, |k| GOW_RAGNAROK.contains(&k))
}

// ------------------------------------------------------------------ Mass Effect: Andromeda

/// The game's settings cap resources at 999 (`max_resources_count`).
fn andromeda(cx: &GameCtx) -> Vec<Stack> {
    ["Minerals", "Biological materials", "Faction materials"]
        .iter()
        .flat_map(|g| counts_under(cx, &format!("/Resources/{}", pointer_escape(g)), 999, |_| true))
        .collect()
}

// ------------------------------------------------------------------ Death Stranding 2

/// Estimate: the game states no limit for a facility's stock.
const DS2_CAP: i64 = 9999;

fn death_stranding2(cx: &GameCtx) -> Vec<Stack> {
    let mut out = vec![];
    for f in object_keys(cx, "/Facilities") {
        let base = format!("/Facilities/{}", pointer_escape(&f));
        for m in crate::fmt::ds2::MATERIALS {
            if let Some(s) = single(cx, format!("{base}/{}", pointer_escape(m)), format!("{f}: {m}"), DS2_CAP) {
                out.push(s);
            }
        }
    }
    out
}

// ------------------------------------------------------------------ Metro Exodus

/// Estimate of the most metal and chemicals Artyom carries on Normal.
const METRO_CAP: i64 = 150;

fn metro(cx: &GameCtx) -> Vec<Stack> {
    counts_under(cx, "/Inventory", METRO_CAP, |k| k.starts_with("Metal (crafting)") || k.starts_with("Chemicals (crafting)"))
}

// ------------------------------------------------------------------ Final Fantasy XV

fn ffxv(cx: &GameCtx) -> Vec<Stack> {
    counts_under(cx, "/Items/Treasures", 99, |_| true)
}

// ------------------------------------------------------------------ KOTOR II, Half-Life: Alyx

/// Estimate: KOTOR II states no limit for components or chemicals.
const KOTOR2_CAP: i64 = 9999;
/// Estimate: Half-Life: Alyx states no limit for resin.
const ALYX_RESIN_CAP: i64 = 999;

// ------------------------------------------------------------------ Starbound

/// Item ids of the base game's crafting materials (ores, bars, cloth and parts). Most items
/// stack to 1000 (the game's default stack size).
const STARBOUND: [&str; 40] = [
    "coalore", "copperore", "ironore", "silverore", "goldore", "platinumore", "titaniumore", "durasteelore", "tungstenore",
    "uraniumore", "plutoniumore", "solariumore", "corefragmentore", "aegisaltore", "ferozoniumore", "violiumore",
    "diamond", "copperbar", "ironbar", "silverbar", "goldbar", "platinumbar", "titaniumbar", "durasteelbar",
    "tungstenbar", "solariumstar", "refinedaegisalt", "refinedferozium", "refinedviolium", "fabric", "leather", "silk",
    "plantfibre", "wire", "glass", "bone", "cottonwool", "siliconboard", "volatilepowder", "phasematter",
];

fn starbound(cx: &GameCtx) -> Vec<Stack> {
    let mut bags: Vec<String> = object_keys(cx, "/inventory/itemBags").into_iter().map(|k| format!("/inventory/itemBags/{}", pointer_escape(&k))).collect();
    if bags.is_empty() {
        bags = ["bag", "tileBag", "actionBar"].iter().map(|k| format!("/inventory/{k}")).collect();
    }
    let mut out = vec![];
    for bag in bags {
        let Some(Value::Array(slots)) = cx.get(&bag) else { continue };
        for i in 0..slots.len() {
            let slot = format!("{bag}/{i}");
            // 1.0 and later wrap the stack in `content`.
            let base = if cx.get(&format!("{slot}/content/name")).is_some() { format!("{slot}/content") } else { slot };
            let name = cx.text(&format!("{base}/name")).unwrap_or_default();
            if STARBOUND.contains(&name.as_str()) {
                if let Some(s) = single(cx, format!("{base}/count"), name, 1000).filter(|s| s.have > 0) {
                    out.push(s);
                }
            }
        }
    }
    out
}

// ------------------------------------------------------------------ Clair Obscur: Expedition 33

/// Estimate: the game states no limit for Chroma Catalysts.
fn expedition33(cx: &GameCtx) -> Vec<Stack> {
    counts_under(cx, "/InventoryItems", 999, |k| k.starts_with("UpgradeMaterial_Level"))
}

// ------------------------------------------------------------------ Valheim

/// Prefab name and stack size (Valheim wiki).
const VALHEIM: [(&str, i64); 74] = [
    ("Wood", 50),
    ("FineWood", 50),
    ("RoundLog", 50),
    ("ElderBark", 50),
    ("YggdrasilWood", 50),
    ("Blackwood", 50),
    ("Stone", 50),
    ("Flint", 30),
    ("Resin", 50),
    ("Coal", 50),
    ("CopperOre", 30),
    ("TinOre", 30),
    ("IronScrap", 30),
    ("IronOre", 30),
    ("SilverOre", 30),
    ("BlackMetalScrap", 30),
    ("CopperScrap", 30),
    ("BronzeScrap", 30),
    ("FlametalOreNew", 30),
    ("Copper", 30),
    ("Tin", 30),
    ("Bronze", 30),
    ("Iron", 30),
    ("Silver", 30),
    ("BlackMetal", 30),
    ("FlametalNew", 30),
    ("LeatherScraps", 50),
    ("DeerHide", 50),
    ("TrollHide", 20),
    ("WolfPelt", 50),
    ("LoxPelt", 50),
    ("ScaleHide", 50),
    ("AskHide", 50),
    ("BjornHide", 50),
    ("MooseHide", 50),
    ("BoneFragments", 50),
    ("Feathers", 50),
    ("Chitin", 50),
    ("Obsidian", 50),
    ("Crystal", 50),
    ("Guck", 50),
    ("Needle", 50),
    ("Bloodbag", 50),
    ("Ooze", 50),
    ("GreydwarfEye", 50),
    ("SurtlingCore", 10),
    ("Thistle", 50),
    ("Dandelion", 50),
    ("FreezeGland", 50),
    ("LinenThread", 50),
    ("Tar", 50),
    ("WolfFang", 50),
    ("Sap", 50),
    ("Eitr", 30),
    ("Carapace", 50),
    ("Mandible", 50),
    ("BlackMarble", 50),
    ("Chain", 50),
    ("IronNails", 100),
    ("BronzeNails", 100),
    ("SerpentScale", 50),
    ("Root", 50),
    ("Grausten", 50),
    ("CharredBone", 50),
    ("MorgenSinew", 50),
    ("CelestialFeather", 50),
    ("SulfurStone", 50),
    ("ProustitePowder", 50),
    ("Wisp", 50),
    ("JuteRed", 50),
    ("JuteBlue", 50),
    ("Bilebag", 50),
    ("WolfHairBundle", 50),
    ("FrostWood", 50),
];

fn valheim_caps() -> &'static HashMap<&'static str, i64> {
    static M: OnceLock<HashMap<&'static str, i64>> = OnceLock::new();
    M.get_or_init(|| VALHEIM.iter().copied().collect())
}

fn valheim(cx: &GameCtx) -> Vec<Stack> {
    let caps = valheim_caps();
    let mut out = vec![];
    for k in object_keys(cx, "/Inventory") {
        // A second stack of the same item is listed as "Wood (2)".
        let prefab = match k.rsplit_once(" (") {
            Some((a, b)) if b.ends_with(')') && b[..b.len() - 1].chars().all(|c| c.is_ascii_digit()) => a,
            _ => k.as_str(),
        };
        let Some(&cap) = caps.get(prefab) else { continue };
        let p = format!("/Inventory/{}/Stack", pointer_escape(&k));
        if let Some(have) = cx.int(&p) {
            out.push(Stack { paths: vec![p], name: prefab.to_string(), have, cap });
        }
    }
    out
}

// ------------------------------------------------------------------ Minecraft

/// Java Edition item ids (without `minecraft:`) of crafting materials that stack to 64.
const MINECRAFT: [&str; 50] = [
    "iron_ingot", "gold_ingot", "copper_ingot", "netherite_ingot", "netherite_scrap", "iron_nugget", "gold_nugget",
    "raw_iron", "raw_gold", "raw_copper", "diamond", "emerald", "lapis_lazuli", "redstone", "coal", "charcoal",
    "quartz", "amethyst_shard", "string", "leather", "rabbit_hide", "feather", "bone", "bone_meal", "gunpowder",
    "slime_ball", "blaze_rod", "blaze_powder", "flint", "paper", "sugar", "stick", "glowstone_dust",
    "prismarine_shard", "prismarine_crystals", "phantom_membrane", "clay_ball", "brick", "nether_brick",
    "magma_cream", "ghast_tear", "ink_sac", "glow_ink_sac", "honeycomb", "nether_wart", "shulker_shell",
    "turtle_scute", "scute", "armadillo_scute", "breeze_rod",
];

fn minecraft(cx: &GameCtx) -> Vec<Stack> {
    let mut out = vec![];
    for list in ["/Data/Player/Inventory", "/Data/Player/EnderItems"] {
        let Some(Value::Array(items)) = cx.get(list) else { continue };
        for i in 0..items.len() {
            let p = format!("{list}/{i}");
            let id = cx.text(&format!("{p}/id")).unwrap_or_default();
            let short = id.strip_prefix("minecraft:").unwrap_or(&id);
            if !MINECRAFT.contains(&short) {
                continue;
            }
            // Before 1.20.5 the count is a byte named Count; since then an int named count.
            let Some(c) = ["Count", "count"].iter().map(|k| format!("{p}/{k}")).find(|q| cx.int(q).is_some()) else { continue };
            let have = cx.int(&c).unwrap_or(0);
            out.push(Stack { paths: vec![c], name: short.to_string(), have, cap: 64 });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(root: &Value) -> GameCtx<'_> {
        GameCtx { root, baseline: root, tree: None, edits: vec![], archive: None, open_entry: None }
    }

    fn edits(cx: &GameCtx) -> HashMap<String, Value> {
        cx.edits.iter().cloned().collect()
    }

    #[test]
    fn terraria_ids_are_known_items() {
        for &i in &TERRARIA_IDS {
            assert!(!crate::fmt::terraria::item_name(i).starts_with("Unknown"), "{i}");
        }
        assert_eq!(terraria_names().len(), TERRARIA_IDS.len());
    }

    #[test]
    fn terraria_needs_1_4_4() {
        let root = json!({"Character": {"File version": 248}, "Inventory": {"Slot 1": {"Item": "Iron Bar", "Stack": 5}}});
        let mut cx = ctx(&root);
        assert!(stacks(Game::Terraria, &cx).is_err());
        assert_eq!(apply(Game::Terraria, &mut cx), 0);
        assert!(cx.edits.is_empty());
    }

    #[test]
    fn stardew_only_touches_listed_materials() {
        let root = json!({"SaveGame": {"player": {"farmName": "x", "items": {"Item": [
            {"@xsi:type": "Object", "name": "Wood", "category": -16, "Stack": 12, "stack": 12, "questItem": "false"},
            {"@xsi:type": "Object", "name": "Iron Bar", "category": -15, "stack": 999},
            {"@xsi:type": "Object", "name": "Quartz", "category": -2, "stack": 3},
            {"@xsi:type": "Object", "name": "Coal", "category": -15, "stack": 4, "questItem": "true"},
            {"@xsi:type": "Object", "name": "Slime", "category": -20, "stack": 4},
            {"@xsi:type": "Tool", "name": "Stone", "stack": 1},
            {"@xsi:nil": "true"}
        ]}}}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Stardew, &mut cx), 1);
        let e = edits(&cx);
        assert_eq!(e.len(), 2);
        assert_eq!(e["/SaveGame/player/items/Item/0/stack"], 999);
        assert_eq!(e["/SaveGame/player/items/Item/0/Stack"], 999);
    }

    #[test]
    fn nier_raises_only_materials() {
        let root = json!({"Items": {"Small Recovery": 3}, "Materials": {"Copper Ore": 12, "Pearl": 99}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Nier, &mut cx), 1);
        let e = edits(&cx);
        assert_eq!(e.len(), 1);
        assert_eq!(e["/Materials/Copper Ore"], 99);
    }

    #[test]
    fn minecraft_both_count_spellings() {
        let root = json!({"Data": {"Player": {
            "Inventory": [
                {"Slot": 0, "id": "minecraft:iron_ingot", "Count": 10},
                {"Slot": 1, "id": "minecraft:ender_pearl", "Count": 3},
                {"Slot": 2, "id": "minecraft:diamond", "Count": 64},
                {"Slot": 3, "id": "minecraft:nether_star", "Count": 1}
            ],
            "EnderItems": [{"Slot": 0, "id": "minecraft:string", "count": 2}, {"Slot": 1, "id": "minecraft:coal"}]
        }}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Minecraft, &mut cx), 2);
        let e = edits(&cx);
        assert_eq!(e.len(), 2);
        assert_eq!(e["/Data/Player/Inventory/0/Count"], 64);
        assert_eq!(e["/Data/Player/EnderItems/0/count"], 64);
    }

    #[test]
    fn valheim_second_stacks_and_caps() {
        let root = json!({"Inventory": {
            "Wood": {"Stack": 10}, "Wood (2)": {"Stack": 3}, "Iron": {"Stack": 30}, "SurtlingCore": {"Stack": 2},
            "Coins": {"Stack": 5}, "DragonTear": {"Stack": 1}, "BellFragment": {"Stack": 1}
        }});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Valheim, &mut cx), 3);
        let e = edits(&cx);
        assert_eq!(e.len(), 3);
        assert_eq!(e["/Inventory/Wood/Stack"], 50);
        assert_eq!(e["/Inventory/Wood (2)/Stack"], 50);
        assert_eq!(e["/Inventory/SurtlingCore/Stack"], 10);
    }

    /// Runs the button on a parsed save and writes the result, like the app does.
    fn round_trip(game: Game, doc: &mut crate::tree::TreeDoc) -> usize {
        let view = doc.view.clone();
        let mut cx = GameCtx { root: &view, baseline: &view, tree: Some(doc), edits: vec![], archive: None, open_entry: None };
        let n = apply(game, &mut cx);
        let e = cx.edits;
        for (p, v) in e {
            doc.set(&p, v).unwrap().expect("editable");
        }
        n
    }

    #[test]
    fn terraria_save_round_trip() {
        use crate::fmt::terraria::{parse, synthetic};
        // Iron Bar, Lens, Dirt Block, Suspicious Looking Eye; Wood and a full Gold Bar stack in the bank.
        let d = synthetic(279, &[(0, 22, 7), (1, 38, 3), (2, 2, 40), (3, 43, 1)], &[(0, 9, 120), (1, 19, 9999)]);
        let mut doc = parse(&d).unwrap();
        assert_eq!(round_trip(Game::Terraria, &mut doc), 3);
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        let v = |p: &str| again.view.pointer(p).cloned().unwrap();
        assert_eq!(v("/Inventory/Slot 1/Stack"), 9999);
        assert_eq!(v("/Inventory/Slot 2/Stack"), 9999);
        assert_eq!(v("/Inventory/Slot 3/Stack"), 40, "blocks aren't materials");
        assert_eq!(v("/Inventory/Slot 4/Stack"), 1, "boss summons are left alone");
        assert_eq!(v("/Piggy bank/Slot 1/Stack"), 9999);
        assert_eq!(v("/Piggy bank/Slot 2/Stack"), 9999);
        assert_eq!(v("/Inventory/Slot 5/Item"), crate::fmt::terraria::EMPTY, "nothing added");
        // Only the stacks changed.
        let changed = doc.to_bytes().unwrap().iter().zip(&d).filter(|(a, b)| a != b).count();
        assert!(changed <= 3 * 4, "{changed} bytes changed");
        // A 1.4.0 character is refused.
        let mut old = parse(&synthetic(260, &[(0, 22, 7)], &[])).unwrap();
        assert_eq!(round_trip(Game::Terraria, &mut old), 0);
    }

    #[test]
    fn valheim_save_round_trip() {
        use crate::fmt::valheim::{parse, tests::synthetic_with};
        let d = synthetic_with(37, &[("Wood", 12, 1), ("Wood", 50, 1), ("Iron", 4, 1), ("Coins", 10, 1), ("DragonTear", 2, 1), ("BellFragment", 1, 1)]);
        let mut doc = parse(&d).unwrap();
        assert_eq!(round_trip(Game::Valheim, &mut doc), 2);
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).unwrap();
        let v = |p: &str| again.view.pointer(p).cloned().unwrap();
        assert_eq!(v("/Inventory/Wood/Stack"), 50);
        assert_eq!(v("/Inventory/Wood (2)/Stack"), 50);
        assert_eq!(v("/Inventory/Iron/Stack"), 30);
        assert_eq!(v("/Inventory/Coins/Stack"), 10);
        assert_eq!(v("/Inventory/DragonTear/Stack"), 2, "boss drops are left alone");
        assert_eq!(v("/Inventory/BellFragment/Stack"), 1, "key fragments are left alone");
    }

    #[test]
    fn stardew_save_round_trip() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><player><name>A</name><farmName>B</farmName><items><Item xsi:type="Object"><category>-16</category><name>Wood</name><Stack>15</Stack><parentSheetIndex>388</parentSheetIndex><stack>15</stack><questItem>false</questItem></Item><Item xsi:type="Object"><category>-2</category><name>Quartz</name><Stack>2</Stack><stack>2</stack></Item><Item xsi:type="Object"><category>-15</category><name>Iron Ore</name><stack>4</stack><questItem>true</questItem></Item><Item xsi:type="Tool"><name>Pickaxe</name><Stack>1</Stack></Item><Item xsi:nil="true" /></items></player></SaveGame>"#;
        let mut doc = crate::fmt::xml::parse(xml).unwrap();
        assert_eq!(round_trip(Game::Stardew, &mut doc), 1);
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, xml.replace("<Stack>15</Stack>", "<Stack>999</Stack>").replace("<stack>15</stack>", "<stack>999</stack>"));
    }

    #[test]
    fn elden_ring_pouch_and_chest() {
        let root = json!({
            "Items": {"Slot 1": {"Item": "Sliver of Meat", "Quantity": 4}, "Slot 2": {"Item": "Smithing Stone [1]", "Quantity": 3}, "Slot 3": {"Item": "Gravel Stone", "Quantity": 999}},
            "Key items": {"Slot 1": {"Item": "Rold Medallion", "Quantity": 1}},
            "Storage": {"Slot 1": {"Item": "Rowa Fruit", "Quantity": 20}, "Slot 2": {"Item": "Erdleaf Flower", "Quantity": 0}}
        });
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::EldenRing, &mut cx), 2);
        let e = edits(&cx);
        assert_eq!(e.len(), 2);
        assert_eq!(e["/Items/Slot 1/Quantity"], 999);
        assert_eq!(e["/Storage/Slot 1/Quantity"], 999);
    }

    #[test]
    fn hogwarts_resource_bag_only() {
        let root = json!({"InventoryDynamic": [
            {"CharacterID": "Player0", "HolderID": "ResourceInventory", "ItemID": "Knuts", "Count": 50},
            {"CharacterID": "Player0", "HolderID": "ResourceInventory", "ItemID": "Moonstone", "Count": 12},
            {"CharacterID": "Player0", "HolderID": "ResourceInventory", "ItemID": "MandrakeSeed", "Count": 2},
            {"CharacterID": "Player0", "HolderID": "GearInventory", "ItemID": "Hat", "Count": 1},
            {"CharacterID": "Player0", "HolderID": "ResourceInventory", "ItemID": "LacewingFlies_Byproduct", "Count": 3}
        ]});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::HogwartsDb, &mut cx), 2);
        let e = edits(&cx);
        assert_eq!(e["/InventoryDynamic/1/Count"], 999);
        assert_eq!(e["/InventoryDynamic/4/Count"], 999);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn god_of_war_skips_story_flames_and_dew() {
        let root = json!({"Slot 1": {"Hacksilver": 10, "Resources": {"Dust of Realms": 5, "Frozen Flame": 1, "Chaos Flame": 1, "Yggdrasil's Dew of Vitality": 1}}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Gow2018, &mut cx), 1);
        assert_eq!(edits(&cx)["/Slot 1/Resources/Dust of Realms"], GOW_CAP);
        let root = json!({"Currency": {"Hacksilver": 3}, "Resources": {"Stonewood": 7, "Meal of Comfort": 1}, "Yggdrasil's Dew": {"Strength": 1}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::GowRagnarok, &mut cx), 1);
        assert_eq!(edits(&cx)["/Resources/Stonewood"], GOW_CAP);
    }

    #[test]
    fn fixed_and_grouped_counts() {
        let root = json!({"Resources": {"Minerals": {"Iron": 4}, "Research data": {"Milky Way": 1}, "Salvage": {"Scrap": 2}, "Faction materials": {"Kett Alloy": 1000}}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Andromeda, &mut cx), 1);
        assert_eq!(edits(&cx)["/Resources/Minerals/Iron"], 999);

        let root = json!({"Facilities": {"Port": {"Connection level": 2, "Metals": 10, "Resins": 9999}}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::DeathStranding2, &mut cx), 1);
        assert_eq!(edits(&cx)["/Facilities/Port/Metals"], DS2_CAP);

        let root = json!({"Inventory": {"Military-grade rounds (money)": 5, "Shotgun shells": 4, "Metal (crafting)": 20, "Chemicals (crafting)": 3}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Metro, &mut cx), 2);
        assert_eq!(edits(&cx).len(), 2);

        let root = json!({"Items": {"Key items": {"10": 1}, "Treasures": {"20": 4}}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Ffxv, &mut cx), 1);
        assert_eq!(edits(&cx)["/Items/Treasures/20"], 99);

        let root = json!({"Party": {"Credits": 1, "Components": 30, "Chemicals": 2}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Kotor, &mut cx), 2);
        assert!(stacks(Game::Kotor, &ctx(&json!({"Party": {"Credits": 1}}))).is_err(), "KOTOR I has none");

        let root = json!({"Player": {"Health": 50, "Resin": 26}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::HalfLife2, &mut cx), 1);
        assert_eq!(edits(&cx)["/Player/Resin"], ALYX_RESIN_CAP);

        let root = json!({"InventoryItems": {"UpgradeMaterial_Level1": 3, "Consumable_LuminaPoint": 2, "SkinVerso": 1}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Expedition33, &mut cx), 1);
        assert_eq!(edits(&cx)["/InventoryItems/UpgradeMaterial_Level1"], 999);
    }

    #[test]
    fn starbound_bags_old_and_new() {
        let root = json!({"inventory": {"itemBags": {"materialBag": [
            {"content": {"name": "ironore", "count": 12}}, null, {"content": {"name": "dirt", "count": 500}}
        ], "mainBag": [{"content": {"name": "fabric", "count": 1000}}, {"content": {"name": "liquidfuel", "count": 2}}]}}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Starbound, &mut cx), 1);
        assert_eq!(edits(&cx)["/inventory/itemBags/materialBag/0/content/count"], 1000);
        let root = json!({"inventory": {"bag": [{"name": "copperbar", "count": 3}]}});
        let mut cx = ctx(&root);
        assert_eq!(apply(Game::Starbound, &mut cx), 1);
        assert_eq!(edits(&cx)["/inventory/bag/0/count"], 1000);
    }

    #[test]
    fn lists_have_no_duplicates() {
        for list in [&ELDEN_RING[..], &GOW_RAGNAROK[..], &STARBOUND[..]] {
            assert_eq!(list.iter().collect::<HashSet<_>>().len(), list.len());
        }
        let er: HashSet<&str> = crate::fmt::er_items::GOODS.iter().map(|(_, n)| *n).collect();
        for n in ELDEN_RING {
            assert!(er.contains(n), "{n}");
        }
        let gowr = include_str!("../fmt/data/gowr_items.tsv");
        for n in GOW_RAGNAROK {
            assert!(gowr.lines().any(|l| l.split('\t').nth(1) == Some(n) && l.ends_with("Resources")), "{n}");
        }
        assert_eq!(valheim_caps().len(), VALHEIM.len());
        let mc: HashSet<_> = MINECRAFT.iter().collect();
        assert_eq!(mc.len(), MINECRAFT.len());
    }
}
