//! Games added from the new-games list (third batch): one `Spec` per game, as in `specs.rs`.
//! Values are only listed once they read correctly on a real save.

use super::{Section, Spec, Table};
use crate::i18n::tx;

pub static NP3: &[Spec] = &[ONINAKI, CASTLE_FLIPPER, GESTALT];

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
