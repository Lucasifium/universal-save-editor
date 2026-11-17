//! Games added from the open-source sweep: one `Spec` per game, as in `specs.rs`.
//! Values are only listed once they read correctly on a real save.

use super::{Section, Spec};
use crate::i18n::tx;

pub static SWEEP: &[Spec] = &[DESTROY_ALL_HUMANS, HUNTING_SIM_2, CYBERDIMENSION, HIGH_ON_LIFE, MONSTER_TRAIN_2, ASTLIBRA, SUPERMARKET_SIM];

// ------------------------------------------------------------------ Destroy All Humans! (2020)

const DESTROY_ALL_HUMANS: Spec = Spec {
    name: "Destroy All Humans! (2020)",
    tab: "Crypto",
    accent: (120, 200, 90),
    kind: Some("Unreal Engine save (BFGSaveGame)"),
    markers: &["/m_playerData/m_stateData/m_iDNA"],
    subtitle: tx("A save from AppData\\Local\\DH\\Saved\\SaveGames (the DevAutoSave and numbered saves; SaveOptions holds only settings). Crypto's DNA, the currency spent on weapon, saucer and ability upgrades. Close the game first."),
    tiles: &[("DNA", "/m_playerData/m_stateData/m_iDNA")],
    sections: &[Section { title: "Crypto", rows: &[("DNA", "/m_playerData/m_stateData/m_iDNA")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Hunting Simulator 2

const HUNTING_SIM_2: Spec = Spec {
    name: "Hunting Simulator 2",
    tab: "Hunter",
    accent: (150, 170, 90),
    kind: Some("Unreal Engine save (HunterSaveGame)"),
    markers: &["/Credits", "/LicenseSaveData"],
    subtitle: tx("HSSave.sav from AppData\\Local\\Hunter2\\Saved\\SaveGames. Your credits, the shots fired, animals spotted and tracks inspected, and the time played. Licences, weapons and gear stay in the Tree."),
    tiles: &[("Credits", "/Credits"), ("Animals spotted", "/AnimalsSpotted"), ("Shots fired", "/ShotsFired")],
    sections: &[
        Section { title: "Hunter", rows: &[("Credits", "/Credits"), ("Tracks inspected", "/TotalTracksInspected")] },
        Section {
            title: "Record",
            rows: &[("Animals spotted", "/AnimalsSpotted"), ("Shots fired", "/ShotsFired"), ("Distance travelled", "/DistanceTravelled"), ("Time in game (seconds)", "/TimeInGame")],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Cyberdimension Neptunia: 4 Goddesses Online

const CYBERDIMENSION: Spec = Spec {
    name: "Cyberdimension Neptunia: 4 Goddesses Online",
    tab: "Guild",
    accent: (200, 130, 220),
    kind: Some("Unreal Engine save (SaveEnforce_C)"),
    markers: &["/SaveData/Money_40_AE7C6D824CD4904414688DACC3180872"],
    subtitle: tx("SaveData<n>.sav from AppData\\Local\\CyberdimensionNeptunia4GoddessesOnline\\Saved\\SaveGames. Your money, guild rank, the totals the game keeps (money earned, enemies defeated, dungeons cleared) and the time played. Close the game first."),
    tiles: &[("Money", "/SaveData/Money_40_AE7C6D824CD4904414688DACC3180872"), ("Guild rank", "/SaveData/GuildRank_43_605F31164FBA852BBBF88E904413E951")],
    sections: &[
        Section {
            title: "Guild",
            rows: &[
                ("Money", "/SaveData/Money_40_AE7C6D824CD4904414688DACC3180872"),
                ("Guild rank", "/SaveData/GuildRank_43_605F31164FBA852BBBF88E904413E951"),
            ],
        },
        Section {
            title: "Record",
            rows: &[
                ("Total money earned", "/SaveData/TotalMoney_45_B19CD1E041E7CD98E7256D931B16BE7B"),
                ("Enemies defeated", "/SaveData/EnemyKillCount_44_B5DD99A84A90233FAB21F5B0D019FD4D"),
                ("Dungeons cleared", "/SaveData/DungeonCount_54_7F7EA0BE46604B07C1C02BB020F9DAAA"),
                ("Longest combo", "/SaveData/MaxComboCount_46_083F62E542E1A1CBE8EC9EA7F39EDC27"),
                ("Time played (seconds)", "/SaveData/PlayTime_57_DD4EA3E4437F09E1E29692A3BAB8A3DB"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ High on Life

const HIGH_ON_LIFE: Spec = Spec {
    name: "High on Life",
    tab: "Bounty hunter",
    accent: (230, 120, 170),
    kind: Some("Unreal Engine save (ORSaveGame)"),
    markers: &["/SaveData/DictionaryRaw/Inventory.Descriptor.Item.Pesos"],
    subtitle: tx("Slot<n>.sav from AppData\\Local\\Oregon\\Saved\\SaveGames (Profile.sav holds settings only). Your pesos. Every other inventory count and story flag is in the Tree under SaveData/DictionaryRaw. Close the game first."),
    tiles: &[("Pesos", "/SaveData/DictionaryRaw/Inventory.Descriptor.Item.Pesos")],
    sections: &[Section { title: "Bounty hunter", rows: &[("Pesos", "/SaveData/DictionaryRaw/Inventory.Descriptor.Item.Pesos")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Monster Train 2

const MONSTER_TRAIN_2: Spec = Spec {
    name: "Monster Train 2",
    tab: "Covenant",
    accent: (210, 90, 90),
    kind: None,
    markers: &["/currentAscensionLevel", "/maxAscensionLevel", "/classXP"],
    subtitle: tx("metagameSave.json from AppData\\LocalLow\\Shiny Shoe\\MonsterTrain2 (plain JSON). The Covenant level you play at and the highest you have unlocked, and the story progress. Clan levels and unlocked cards are in the Tree."),
    tiles: &[("Covenant level", "/currentAscensionLevel"), ("Highest Covenant unlocked", "/maxAscensionLevel")],
    sections: &[Section {
        title: "Progress",
        rows: &[
            ("Covenant level", "/currentAscensionLevel"),
            ("Highest Covenant unlocked", "/maxAscensionLevel"),
            ("Story chapters completed", "/storyProgressionNumCompleted"),
            ("Railforged story chapters completed", "/storyProgressionRailforgedNumCompleted"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ ASTLIBRA Revision

const ASTLIBRA: Spec = Spec {
    name: "ASTLIBRA Revision",
    tab: "Hero",
    accent: (90, 150, 220),
    kind: Some(crate::fmt::astlibra::KIND),
    markers: &["/Hero/Gold", "/Force/Fire"],
    subtitle: tx("SAVE_<n>.DAT from AppData\\Local\\ASTLIBRA\\SAVE (or Steam\\userdata\\<account>\\1718570\\remote). Gold, level, experience towards the next level and the six force pools (fire, earth, water, darkness, wind, holy). Both of the save's checksums are written again."),
    tiles: &[("Gold", "/Hero/Gold"), ("Level", "/Hero/Level")],
    sections: &[
        Section { title: "Hero", rows: &[("Gold", "/Hero/Gold"), ("Level", "/Hero/Level"), ("Experience towards next level", "/Hero/Experience towards next level")] },
        Section {
            title: "Force",
            rows: &[("Fire", "/Force/Fire"), ("Earth", "/Force/Earth"), ("Water", "/Force/Water"), ("Darkness", "/Force/Darkness"), ("Wind", "/Force/Wind"), ("Holy", "/Force/Holy")],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Supermarket Simulator

const SUPERMARKET_SIM: Spec = Spec {
    name: "Supermarket Simulator",
    tab: "Store",
    accent: (90, 190, 130),
    kind: Some("Easy Save 3 data"),
    markers: &["/Progression/value/Money", "/Progression/value/CurrentStoreLevel"],
    subtitle: tx("SaveFile.es3 from AppData\\LocalLow\\Nokta Games\\Supermarket Simulator (the other .es3 files there are backups). Your money, the store level and points, the upgrade and storage levels and the day. Prices, licences and boxes stay in the Tree. Close the game first."),
    tiles: &[("Money", "/Progression/value/Money"), ("Store level", "/Progression/value/CurrentStoreLevel"), ("Day", "/Progression/value/CurrentDay")],
    sections: &[
        Section {
            title: "Store",
            rows: &[
                ("Money", "/Progression/value/Money"),
                ("Store level", "/Progression/value/CurrentStoreLevel"),
                ("Store points", "/Progression/value/CurrentStorePoint"),
                ("Store upgrade level", "/Progression/value/StoreUpgradeLevel"),
                ("Storage level", "/Storage/value/StorageLevel"),
                ("Day", "/Progression/value/CurrentDay"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};
