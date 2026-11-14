//! Screens for games whose saves are plain Unreal, JSON or XML trees (see `specs.rs` for the shape).

use super::{List, Max, Section, Shape, Spec, Table};

// ------------------------------------------------------------------ Darksiders Genesis

pub const DARKSIDERS_GENESIS: Spec = Spec {
    name: "Darksiders Genesis",
    tab: "Horsemen",
    accent: (214, 92, 28),
    kind: Some("Unreal Engine save (MayhemSaveGame)"),
    markers: &["/PlayerStateData/PlayerData/Souls", "/PlayerStateData/CharacterData"],
    subtitle: "SaveSlot_N.sav from %LOCALAPPDATA%\\THQ Nordic\\Darksiders Genesis\\Saved\\SaveGames\\user. Souls, coins, keys, potions and the shards and wrath of Strife and War. Close the game first.",
    tiles: &[("Souls", "/PlayerStateData/PlayerData/Souls"), ("Coins", "/PlayerStateData/PlayerData/Coins"), ("Keys", "/PlayerStateData/PlayerData/Keys")],
    sections: &[Section {
        title: "Account",
        rows: &[
            ("Souls", "/PlayerStateData/PlayerData/Souls"), ("Coins", "/PlayerStateData/PlayerData/Coins"),
            ("Keys", "/PlayerStateData/PlayerData/Keys"), ("Health potions", "/PlayerStateData/PlayerData/HealthPotions"),
            ("Strife power", "/StrifePower"), ("War power", "/WarPower"),
            ("Play time (hours)", "/PlayTimeHours"), ("Play time (minutes)", "/PlayTimeMinutes"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: "Horsemen",
        path: "/PlayerStateData/CharacterData",
        name: &["/CharacterStats/ID"],
        cols: &[
            ("Wrath", "/PlayerCharacterStats/CurrentWrath"),
            ("Health shards", "/PlayerCharacterStats/HealthShards"),
            ("Wrath shards", "/PlayerCharacterStats/WrathShards"),
        ],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Journey to the Savage Planet

const JSP_NAMES: &[(&str, &str)] = &[
    ("Base_Carbon", "Carbon"), ("Base_Silicon", "Silicon"), ("Base_Aluminium", "Aluminium"), ("Rare_AlienAlloy", "Alien alloy"),
];

pub const JOURNEY_SAVAGE_PLANET: Spec = Spec {
    name: "Journey to the Savage Planet",
    tab: "Stash",
    accent: (240, 150, 40),
    kind: Some("Unreal Engine save (TowersPlayerSaveGame)"),
    markers: &["/SaveGameData/LootStashInventoryItems"],
    subtitle: "PlayerSave_N.sav from %LOCALAPPDATA%\\Towers\\Saved\\SaveGames\\<id>. The resources in the stash and the items in the bag. Close the game first.",
    tiles: &[],
    sections: &[Section {
        title: "Progress",
        rows: &[("Completion (%)", "/SaveGameData/PlayerStats/TotalCompletion")],
    }],
    lists: &[
        List {
            title: "Stash",
            path: "/SaveGameData/LootStashInventoryItems",
            shape: Shape::Array { id_key: "LootId", qty_keys: &["AmountInInventory"] },
            names: JSP_NAMES,
            max: Some(Max {
                ids: &["Base_Carbon", "Base_Silicon", "Base_Aluminium", "Rare_AlienAlloy"],
                cap: 9999,
                caps: &[],
                estimate: true,
                about: "Raises carbon, silicon, aluminium and alien alloy to 9,999. Fruit and every other item are left alone.",
            }),
        },
        List {
            title: "Bag",
            path: "/SaveGameData/LootBagInventoryItems",
            shape: Shape::Array { id_key: "LootId", qty_keys: &["AmountInInventory"] },
            names: &[],
            max: None,
        },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ My Hero One's Justice

pub const MHOJ1: Spec = Spec {
    name: "My Hero One's Justice",
    tab: "Heroes",
    accent: (46, 150, 90),
    kind: Some("Unreal Engine save (SystemSaveGameX170)"),
    markers: &["/Cash", "/ItemSaveData"],
    subtitle: "systemdata.sav from %LOCALAPPDATA%\\BANDAI NAMCO Entertainment\\My Hero Ones Justice\\Saved\\SaveGames\\<id>. Cash. Close the game first.",
    tiles: &[("Cash", "/Cash")],
    sections: &[Section { title: "Account", rows: &[("Cash", "/Cash")] }],
    lists: &[],
    tables: &[],
};

pub const MHOJ2: Spec = Spec {
    name: "My Hero One's Justice 2",
    tab: "Heroes",
    accent: (46, 150, 90),
    kind: Some("Unreal Engine save (SaveData)"),
    markers: &["/HeroCoin", "/ItemSaveData"],
    subtitle: "systemdata*.sav from %LOCALAPPDATA%\\BANDAI NAMCO Entertainment\\My Hero Ones Justice 2\\Saved\\SaveGames\\<id>. Hero coins. Close the game first.",
    tiles: &[("Hero coins", "/HeroCoin")],
    sections: &[Section { title: "Account", rows: &[("Hero coins", "/HeroCoin")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Vikings: Wolves of Midgard

pub const VIKINGS_WOLVES: Spec = Spec {
    name: "Vikings: Wolves of Midgard",
    tab: "Clan",
    accent: (150, 100, 50),
    kind: None,
    markers: &["/playerStorage/data/0/goldStor", "/playerStorage/data/0/ironStor"],
    subtitle: "game_*.sav from %USERPROFILE%\\AppData\\LocalLow\\Games Farm s_r_o_\\Vikings_ Wolves of Midgard\\saves. Gold, iron, steel, blood and jotunjarn, the ores, secrets found and finished runs. Close the game first.",
    tiles: &[("Gold", "/playerStorage/data/0/goldStor"), ("Iron", "/playerStorage/data/0/ironStor"), ("Steel", "/playerStorage/data/0/steelStor")],
    sections: &[
        Section {
            title: "Resources",
            rows: &[
                ("Gold", "/playerStorage/data/0/goldStor"), ("Iron", "/playerStorage/data/0/ironStor"),
                ("Steel", "/playerStorage/data/0/steelStor"), ("Blood", "/playerStorage/data/0/bloodStor"),
                ("Jotunjarn", "/playerStorage/data/0/jotunjarnStor"), ("Gold ore", "/playerStorage/data/0/goldOreStor"),
                ("Iron ore", "/playerStorage/data/0/ironOreStor"), ("Bloodstone", "/playerStorage/data/0/bloodStoneStor"),
            ],
        },
        Section {
            title: "Progress",
            rows: &[("Secrets found", "/playerStorage/data/0/numSecretsFoundStor"), ("Finished games", "/playerStorage/data/0/finishedGameCount")],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Sea of Stars

pub const SEA_OF_STARS: Spec = Spec {
    name: "Sea of Stars",
    tab: "Party",
    accent: (96, 170, 230),
    kind: None,
    markers: &["/partyProgressData/totalXP", "/currencySaveData/currencyQuantities"],
    subtitle: "SOS_SaveSlotN.sos from %USERPROFILE%\\AppData\\LocalLow\\Sabotage Studio\\Sea of Stars (JSON behind a length prefix, unpacked and packed again). The party's level and experience, gold and the number of deaths. Close the game first.",
    tiles: &[("Level", "/partyProgressData/currentLevel"), ("Gold", "/currencySaveData/currencyQuantities/values/0")],
    sections: &[Section {
        title: "Party",
        rows: &[
            ("Level", "/partyProgressData/currentLevel"), ("Total experience", "/partyProgressData/totalXP"),
            ("Gold", "/currencySaveData/currencyQuantities/values/0"), ("Deaths", "/deathCount"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ The Messenger

pub const THE_MESSENGER: Spec = Spec {
    name: "The Messenger",
    tab: "Ninja",
    accent: (230, 90, 60),
    kind: None,
    markers: &["/saveSlots/0/secondsPlayed", "/saveSlots/0/totalDeathCount"],
    subtitle: "SaveGame.txt from %USERPROFILE%\\AppData\\LocalLow\\Sabotage Studio\\The Messenger (every character is scrambled, unpacked and scrambled again). Each save slot's play time, deaths and New Game Plus. Close the game first.",
    tiles: &[("Deaths", "/saveSlots/0/totalDeathCount")],
    sections: &[
        Section {
            title: "Save slot 1",
            rows: &[("Play time (seconds)", "/saveSlots/0/secondsPlayed"), ("Deaths", "/saveSlots/0/totalDeathCount"), ("New Game Plus count", "/saveSlots/0/newGamePlusCount")],
        },
        Section {
            title: "Save slot 2",
            rows: &[("Play time (seconds)", "/saveSlots/1/secondsPlayed"), ("Deaths", "/saveSlots/1/totalDeathCount"), ("New Game Plus count", "/saveSlots/1/newGamePlusCount")],
        },
        Section {
            title: "Save slot 3",
            rows: &[("Play time (seconds)", "/saveSlots/2/secondsPlayed"), ("Deaths", "/saveSlots/2/totalDeathCount"), ("New Game Plus count", "/saveSlots/2/newGamePlusCount")],
        },
    ],
    lists: &[],
    tables: &[],
};

const KR1_HEROES: &[(&str, &str)] = &[("hero_10yr", "10Yr"), ("hero_alleria", "Alleria"), ("hero_bolin", "Bolin"), ("hero_denas", "Denas"), ("hero_elora", "Elora"), ("hero_gerald", "Gerald"), ("hero_hacksaw", "Hacksaw"), ("hero_ignus", "Ignus"), ("hero_ingvar", "Ingvar"), ("hero_magnus", "Magnus"), ("hero_malik", "Malik"), ("hero_oni", "Oni"), ("hero_thor", "Thor"), ("hero_alien", "Alien"), ("hero_alric", "Alric"), ("hero_beastmaster", "Beastmaster"), ("hero_crab", "Crab"), ("hero_dracolich", "Dracolich"), ("hero_dragon", "Dragon"), ("hero_giant", "Giant"), ("hero_minotaur", "Minotaur"), ("hero_mirage", "Mirage"), ("hero_monk", "Monk"), ("hero_monkey_god", "Monkey God"), ("hero_pirate", "Pirate"), ("hero_priest", "Priest"), ("hero_van_helsing", "Van Helsing"), ("hero_voodoo_witch", "Voodoo Witch"), ("hero_wizard", "Wizard")];

pub const KR1: Spec = Spec {
    name: "Kingdom Rush and Kingdom Rush Frontiers",
    tab: "Kingdom",
    accent: (200, 150, 50),
    kind: Some("Lua table"),
    markers: &["/gems", "/upgrades/rain", "/upgrades/engineers"],
    subtitle: "slot_N.lua from %APPDATA%\\kingdom_rush (or kingdom_rush_frontiers). Gems, the difficulty, the upgrade ranks and each hero's experience. Close the game first.",
    tiles: &[("Gems", "/gems")],
    sections: &[
        Section { title: "Game", rows: &[("Gems", "/gems")] },
        Section {
            title: "Upgrade ranks",
            rows: &[("Archers", "/upgrades/archers"), ("Barracks", "/upgrades/barracks"), ("Mages", "/upgrades/mages"), ("Engineers", "/upgrades/engineers"), ("Rain of Fire", "/upgrades/rain"), ("Reinforcements", "/upgrades/reinforcements")],
        },
    ],
    lists: &[List {
        title: "Heroes (experience)",
        path: "/heroes/status",
        shape: Shape::Records { qty_key: "xp" },
        names: KR1_HEROES,
        max: None,
    }],
    tables: &[],
};


const KRO_HEROES: &[(&str, &str)] = &[("hero_arivan", "Arivan"), ("hero_bravebark", "Bravebark"), ("hero_bruce", "Bruce"), ("hero_catha", "Catha"), ("hero_durax", "Durax"), ("hero_elves_archer", "Elves Archer"), ("hero_elves_denas", "Elves Denas"), ("hero_faustus", "Faustus"), ("hero_lilith", "Lilith"), ("hero_lynn", "Lynn"), ("hero_phoenix", "Phoenix"), ("hero_rag", "Rag"), ("hero_regson", "Regson"), ("hero_veznan", "Veznan"), ("hero_wilbur", "Wilbur"), ("hero_xin", "Xin")];

pub const KRO: Spec = Spec {
    name: "Kingdom Rush Origins",
    tab: "Forest",
    accent: (80, 160, 80),
    kind: Some("Lua table"),
    markers: &["/gems", "/upgrades/thunder", "/upgrades/rocks"],
    subtitle: "slot_N.lua from %APPDATA%\\kingdom_rush_origins. Gems, the difficulty, the upgrade ranks and each hero's experience. Close the game first.",
    tiles: &[("Gems", "/gems")],
    sections: &[
        Section { title: "Game", rows: &[("Gems", "/gems")] },
        Section {
            title: "Upgrade ranks",
            rows: &[("Archers", "/upgrades/archers"), ("Barracks", "/upgrades/barracks"), ("Mages", "/upgrades/mages"), ("Rocks", "/upgrades/rocks"), ("Thunder", "/upgrades/thunder"), ("Reinforcements", "/upgrades/reinforcements")],
        },
    ],
    lists: &[List {
        title: "Heroes (experience)",
        path: "/heroes/status",
        shape: Shape::Records { qty_key: "xp" },
        names: KRO_HEROES,
        max: None,
    }],
    tables: &[],
};

const KRA_HEROES: &[(&str, &str)] = &[("hero_bird", "Bird"), ("hero_builder", "Builder"), ("hero_dragon_arb", "Dragon Arb"), ("hero_dragon_bone", "Dragon Bone"), ("hero_dragon_gem", "Dragon Gem"), ("hero_hunter", "Hunter"), ("hero_lava", "Lava"), ("hero_lumenir", "Lumenir"), ("hero_mecha", "Mecha"), ("hero_muyrn", "Muyrn"), ("hero_raelyn", "Raelyn"), ("hero_robot", "Robot"), ("hero_space_elf", "Space Elf"), ("hero_spider", "Spider"), ("hero_venom", "Venom"), ("hero_vesper", "Vesper"), ("hero_witch", "Witch"), ("hero_wukong", "Wukong")];

pub const KRA: Spec = Spec {
    name: "Kingdom Rush 5: Alliance TD",
    tab: "Alliance",
    accent: (120, 100, 210),
    kind: Some("Lua table"),
    markers: &["/gems", "/upgrades/alliance", "/upgrades/towers"],
    subtitle: "slot_N.lua from %APPDATA%\\kingdom_rush_alliance. Gems, the difficulty, the upgrade ranks and each hero's experience. Close the game first.",
    tiles: &[("Gems", "/gems")],
    sections: &[
        Section { title: "Game", rows: &[("Gems", "/gems")] },
        Section {
            title: "Upgrade ranks",
            rows: &[("Towers", "/upgrades/towers"), ("Reinforcements", "/upgrades/reinforcements"), ("Heroes", "/upgrades/heroes"), ("Alliance", "/upgrades/alliance")],
        },
    ],
    lists: &[List {
        title: "Heroes (experience)",
        path: "/heroes/status",
        shape: Shape::Records { qty_key: "xp" },
        names: KRA_HEROES,
        max: None,
    },
        List {
            title: "Consumables",
            path: "/items/status",
            shape: Shape::Map,
            names: &[("cluster_bomb", "Cluster bomb"), ("deaths_touch", "Death's touch"), ("loot_box", "Loot box"), ("medical_kit", "Medical kit"), ("portable_coil", "Portable coil"), ("scroll_of_spaceshift", "Scroll of space shift"), ("second_breath", "Second breath"), ("summon_blackburn", "Summon Blackburn"), ("veznan_wrath", "Veznan's wrath"), ("winter_age", "Winter age")],
            max: None,
        }],
    tables: &[],
};

// ------------------------------------------------------------------ Kingdom Rush Vengeance

pub const KR_VENGEANCE: Spec = Spec {
    name: "Kingdom Rush Vengeance",
    tab: "Dark army",
    accent: (150, 60, 170),
    kind: Some("Property list (XML)"),
    markers: &["/veznan/gems", "/campaign/levels"],
    subtitle: "slot_N.plist from %LOCALAPPDATA%\\Kingdom Rush Vengeance. Gems and the stars earned on each campaign level. Close the game first.",
    tiles: &[("Gems", "/veznan/gems")],
    sections: &[Section { title: "Game", rows: &[("Gems", "/veznan/gems")] }],
    lists: &[],
    tables: &[Table {
        title: "Campaign levels",
        path: "/campaign/levels",
        name: &["/level_index"],
        cols: &[("Stars", "/stars")],
        only: &[],
        names: &[],
    }],
};

/// Difficulty names of the Kingdom Rush games (see `choices.rs`).
#[allow(dead_code)]
const KR_DIFFICULTY_TEXTS: &[&str] = &["Casual", "Veteran", "Impossible"];
