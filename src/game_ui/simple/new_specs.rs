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
