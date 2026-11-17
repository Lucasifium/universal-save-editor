//! Games added from the new-games list (third batch): one `Spec` per game, as in `specs.rs`.
//! Values are only listed once they read correctly on a real save.

use super::{List, Max, Section, Shape, Spec, Table};
use crate::i18n::tx;

pub static NP3: &[Spec] = &[ONINAKI, CASTLE_FLIPPER, GESTALT, NMS, NMS_CONTEXTS];

// ------------------------------------------------------------------ ONINAKI

const ONINAKI: Spec = Spec {
    name: "ONINAKI",
    tab: "Watcher",
    accent: (110, 150, 210),
    kind: Some("MessagePack"),
    markers: &["/playerData/level", "/kamishiroDatas"],
    subtitle: tx("A SlotNNNN.sav from Documents\\ONINAKI\\Steam\\<account> (the Header.sav lists the slots). The Watcher's level, health and experience, the souls you hold, the healing charges, and every daemon's rank and soul count. Close the game first."),
    tiles: &[("Level", "/playerData/level"), ("Souls held", "/globalOnitamaNum"), ("Health", "/playerData/hp")],
    sections: &[
        Section {
            title: "Watcher",
            rows: &[("Level", "/playerData/level"), ("Experience", "/playerData/exp"), ("Health", "/playerData/hp"), ("Souls held", "/globalOnitamaNum"), ("Healing charges", "/healData/num"), ("Maximum healing charges", "/healData/max")],
        },
        Section { title: "Record", rows: &[("Time played (seconds)", "/playTime"), ("Enemies defeated for souls", "/spiritDropKillEnemyNum"), ("Enemies transmuted", "/transmutationData/enemyKillNum")] },
    ],
    lists: &[],
    tables: &[Table { title: "Daemons", path: "/kamishiroDatas", name: &[], only: &[], cols: &[("Rank", "/rank"), ("Souls", "/onitamaNum"), ("Soul experience", "/onitamaExp"), ("Active", "/active")], names: &[] }],
};

// ------------------------------------------------------------------ Castle Flipper

const CASTLE_FLIPPER: Spec = Spec {
    name: "Castle Flipper",
    tab: "Builder",
    accent: (200, 150, 80),
    kind: Some(".NET BinaryFormatter (SaveData)"),
    markers: &["/MyDataSaver/0/MyMoneyAmount/0", "/MyDataSaver/0/MyRockAmount/0"],
    subtitle: tx("Data.dat from a save's folder under AppData\\LocalLow (one folder per quest). Money, stone and wood, the skill points and the current mission. Close the game first."),
    tiles: &[("Money", "/MyDataSaver/0/MyMoneyAmount/0"), ("Stone", "/MyDataSaver/0/MyRockAmount/0"), ("Wood", "/MyDataSaver/0/MyWoodAmount/0")],
    sections: &[Section {
        title: "Builder",
        rows: &[("Money", "/MyDataSaver/0/MyMoneyAmount/0"), ("Stone", "/MyDataSaver/0/MyRockAmount/0"), ("Wood", "/MyDataSaver/0/MyWoodAmount/0"), ("Skill points", "/MyDataSaver/0/MySkillPoints/0"), ("Current mission", "/MyDataSaver/0/MyCurrentMission/0")],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Gestalt: Steam & Cinder

const GESTALT: Spec = Spec {
    name: "Gestalt: Steam & Cinder",
    tab: "Aletheia",
    accent: (200, 120, 70),
    kind: None,
    markers: &["/m_list/=game/data/statsAletheia/experience/currentLevel", "/m_list/=game/data/totalPlaytime"],
    subtitle: tx("save_N.dat from the game's save folder under AppData\\LocalLow (the save's own copy of each room is left alone). Level, experience, level points, health, energy and the attributes of Aletheia and Thirty-Four. Close the game first."),
    tiles: &[("Level", "/m_list/=game/data/statsAletheia/experience/currentLevel"), ("Experience", "/m_list/=game/data/statsAletheia/experience/currentExperience"), ("Level points", "/m_list/=game/data/statsAletheia/experience/levelPoints")],
    sections: &[
        Section {
            title: "Aletheia",
            rows: &[
                ("Level", "/m_list/=game/data/statsAletheia/experience/currentLevel"), ("Experience", "/m_list/=game/data/statsAletheia/experience/currentExperience"), ("Level points", "/m_list/=game/data/statsAletheia/experience/levelPoints"),
                ("Maximum health", "/m_list/=game/data/statsAletheia/health/maxHealth"), ("Maximum energy", "/m_list/=game/data/statsAletheia/health/maxEnergy"), ("Endurance", "/m_list/=game/data/statsAletheia/core/endurance"),
                ("Strength", "/m_list/=game/data/statsAletheia/core/strength_Power"), ("Dexterity", "/m_list/=game/data/statsAletheia/core/dexterity_Torque"), ("Luck", "/m_list/=game/data/statsAletheia/core/luck"),
                ("Synergy", "/m_list/=game/data/statsAletheia/core/synergy"),
            ],
        },
        Section {
            title: "Thirty-Four",
            rows: &[
                ("Level", "/m_list/=game/data/statsThirtyFour/experience/currentLevel"), ("Experience", "/m_list/=game/data/statsThirtyFour/experience/currentExperience"), ("Level points", "/m_list/=game/data/statsThirtyFour/experience/levelPoints"),
                ("Maximum health", "/m_list/=game/data/statsThirtyFour/health/maxHealth"), ("Maximum energy", "/m_list/=game/data/statsThirtyFour/health/maxEnergy"), ("Endurance", "/m_list/=game/data/statsThirtyFour/core/endurance"),
                ("Strength", "/m_list/=game/data/statsThirtyFour/core/strength_Power"), ("Dexterity", "/m_list/=game/data/statsThirtyFour/core/dexterity_Torque"), ("Luck", "/m_list/=game/data/statsThirtyFour/core/luck"),
                ("Synergy", "/m_list/=game/data/statsThirtyFour/core/synergy"),
            ],
        },
        Section { title: "Record", rows: &[("Time played (seconds)", "/m_list/=game/data/totalPlaytime"), ("Deaths", "/m_list/=game/data/persistentData/numDeaths"), ("Enemies defeated", "/m_list/=game/data/persistentData/numKills")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ No Man's Sky

/// Raw materials' item ids and names. Names as the game shows them; ids as stored in a slot.
const NMS_NAMES: &[(&str, &str)] = &[("^FUEL1", "Carbon"),("^FUEL2", "Condensed Carbon"),("^OXYGEN", "Oxygen"),("^LAUNCHSUB", "Di-hydrogen"),("^LAUNCHSUB2", "Deuterium"),("^ROCKETSUB", "Tritium"),("^LAND1", "Ferrite Dust"),("^LAND2", "Pure Ferrite"),("^LAND3", "Magnetised Ferrite"),("^SAND1", "Silicate Powder"),("^CATALYST1", "Sodium"),("^CATALYST2", "Sodium Nitrate"),("^CAVE1", "Cobalt"),("^CAVE2", "Ionised Cobalt"),("^WATER1", "Salt"),("^WATER2", "Chlorine"),("^WATERPLANT", "Cyto-phosphate"),("^YELLOW2", "Copper"),("^RED2", "Cadmium"),("^GREEN2", "Emeril"),("^BLUE2", "Indium"),("^PURPLE2", "Quartzite"),("^STELLAR2", "Chromatic Metal"),("^LUSH1", "Paraffinium"),("^DUSTY1", "Pyrite"),("^TOXIC1", "Ammonia"),("^RADIO1", "Uranium"),("^COLD1", "Dioxite"),("^HOT1", "Phosphorus"),("^LAVA1", "Basalt"),("^WATERWORLD1", "Lithium"),("^GASGIANT1", "Crystallised Helium"),("^CREATURE1", "Mordite"),("^ASTEROID1", "Silver"),("^ASTEROID2", "Gold"),("^ASTEROID3", "Platinum"),("^EX_YELLOW", "Activated Copper"),("^EX_RED", "Activated Cadmium"),("^EX_GREEN", "Activated Emeril"),("^EX_BLUE", "Activated Indium"),("^EX_PURPLE", "Activated Quartzite"),("^PLANT_TOXIC", "Fungal Mould"),("^PLANT_SNOW", "Frost Crystal"),("^PLANT_RADIO", "Gamma Root"),("^PLANT_DUST", "Cactus Flesh"),("^PLANT_HOT", "Solanium"),("^PLANT_LUSH", "Star Bulb"),("^PLANT_CAVE", "Marrow Bulb"),("^PLANT_WATER", "Kelp Sac"),("^PLANT_POOP", "Faecium"),("^RUINSUB", "Polished Stone"),("^ROBOT2", "Atlantideum"),("^SPACEGUNK1", "Residual Goop"),("^SPACEGUNK2", "Runaway Mould"),("^SPACEGUNK3", "Rusted Metal"),("^SPACEGUNK4", "Living Slime"),("^SPACEGUNK5", "Viscous Fluids"),("^AF_METAL", "Tainted Metal"),("^BUI_SCRAP", "Void Mote"),("^SPECIAL_POOP", "Hexite"),("^SUNGOLD", "Liquid Sun"),("^SOULFRAG", "Fragmented Qualia"),("^SQUIDFRAG", "Shattered Qualia"),("^WORMDUST", "Cursed Dust"),("^TIMEDUST", "Somnal Dust"),("^TIMEMILK", "Ancestral Memories"),("^SWARMDUST", "Corrupted Ichor"),("^ASTBELT1", "Comet Dust"),("^HULK1", "Isotopic Dust"),("^SLIMEPOST1", "Gelatinous Fibres")];

/// The plain elements, refined metals and harvested plants (not story, salvage or junk items).
const NMS_RAW: &[&str] = &["^FUEL1", "^FUEL2", "^OXYGEN", "^LAUNCHSUB", "^LAUNCHSUB2", "^ROCKETSUB", "^LAND1", "^LAND2", "^LAND3", "^SAND1", "^CATALYST1", "^CATALYST2", "^CAVE1", "^CAVE2", "^WATER1", "^WATER2", "^WATERPLANT", "^YELLOW2", "^RED2", "^GREEN2", "^BLUE2", "^PURPLE2", "^STELLAR2", "^LUSH1", "^DUSTY1", "^TOXIC1", "^RADIO1", "^COLD1", "^HOT1", "^LAVA1", "^WATERWORLD1", "^GASGIANT1", "^CREATURE1", "^ASTEROID1", "^ASTEROID2", "^ASTEROID3", "^EX_YELLOW", "^EX_RED", "^EX_GREEN", "^EX_BLUE", "^EX_PURPLE", "^PLANT_TOXIC", "^PLANT_SNOW", "^PLANT_RADIO", "^PLANT_DUST", "^PLANT_HOT", "^PLANT_LUSH", "^PLANT_CAVE", "^PLANT_WATER", "^PLANT_POOP"];

const NMS_MAX: &str = "Raises every raw material in this inventory that you already hold (elements, refined metals, plants) to the stack limit stored with that stack. Story items, salvage and junk are left alone.";

macro_rules! nms_spec {
    ($const:ident, $name:expr, $p:literal, $sub:expr) => {
        const $const: Spec = Spec {
            name: $name,
            tab: "Currencies",
            accent: (70, 160, 200),
            kind: None,
            markers: &[concat!($p, "/PlayerStateData/Units")],
            subtitle: $sub,
            tiles: &[("Units", concat!($p, "/PlayerStateData/Units")), ("Nanites", concat!($p, "/PlayerStateData/Nanites")), ("Quicksilver", concat!($p, "/PlayerStateData/Specials"))],
            sections: &[Section {
                title: "Currencies",
                rows: &[("Units", concat!($p, "/PlayerStateData/Units")), ("Nanites", concat!($p, "/PlayerStateData/Nanites")), ("Quicksilver", concat!($p, "/PlayerStateData/Specials"))],
            }],
            lists: &[
                List {
                    title: "Exosuit inventory",
                    path: concat!($p, "/PlayerStateData/Inventory/Slots"),
                    shape: Shape::Array { id_key: "Id", qty_keys: &["Amount"] },
                    names: NMS_NAMES,
                    max: Some(Max { ids: NMS_RAW, cap: 0, caps: &[], estimate: false, about: NMS_MAX }),
                },
                List {
                    title: "Exosuit cargo",
                    path: concat!($p, "/PlayerStateData/Inventory_Cargo/Slots"),
                    shape: Shape::Array { id_key: "Id", qty_keys: &["Amount"] },
                    names: NMS_NAMES,
                    max: Some(Max { ids: NMS_RAW, cap: 0, caps: &[], estimate: false, about: NMS_MAX }),
                },
                List {
                    title: "Freighter inventory",
                    path: concat!($p, "/PlayerStateData/FreighterInventory/Slots"),
                    shape: Shape::Array { id_key: "Id", qty_keys: &["Amount"] },
                    names: NMS_NAMES,
                    max: Some(Max { ids: NMS_RAW, cap: 0, caps: &[], estimate: false, about: NMS_MAX }),
                },
                List {
                    title: "Storage container 1",
                    path: concat!($p, "/PlayerStateData/Chest1Inventory/Slots"),
                    shape: Shape::Array { id_key: "Id", qty_keys: &["Amount"] },
                    names: NMS_NAMES,
                    max: Some(Max { ids: NMS_RAW, cap: 0, caps: &[], estimate: false, about: NMS_MAX }),
                },
                List {
                    title: "Storage container 2",
                    path: concat!($p, "/PlayerStateData/Chest2Inventory/Slots"),
                    shape: Shape::Array { id_key: "Id", qty_keys: &["Amount"] },
                    names: NMS_NAMES,
                    max: Some(Max { ids: NMS_RAW, cap: 0, caps: &[], estimate: false, about: NMS_MAX }),
                },
            ],
            tables: &[],
        };
    };
}

nms_spec!(NMS, "No Man's Sky", "", tx("saveN.hg from %APPDATA%\\HelloGames\\NMS\\st_<id> (Steam) with the mf_saveN.hg beside it, which is made to agree with it. Units, nanites, quicksilver and the amount in the Exosuit, cargo, freighter and first two storage containers, with a Max crafting materials button for every container. Close the game first."));
nms_spec!(NMS_CONTEXTS, "No Man's Sky", "/BaseContext", tx("saveN.hg from %APPDATA%\\HelloGames\\NMS\\st_<id> (Steam) with the mf_saveN.hg beside it, which is made to agree with it. Units, nanites, quicksilver and the amount in the Exosuit, cargo, freighter and first two storage containers, with a Max crafting materials button for every container. Close the game first."));
