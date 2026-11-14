//! Screens for PC games with packed or checksummed formats (see `crate::fmt`).

use super::{Section, Spec, Table};
use crate::i18n::tx;

pub const GTASA: Spec = Spec {
    name: "Grand Theft Auto: San Andreas",
    tab: "Player",
    accent: (220, 140, 50),
    kind: Some(crate::fmt::gtasa::KIND),
    markers: &["/Player/Money", "/Statistics/Days passed"],
    subtitle: tx("GTASAsf*.b (the 2005 PC release). Money (both copies), health and armor, body stats, skills, weapons and ammunition, story line and options; the save's checksum is rewritten."),
    tiles: &[(tx("Money"), "/Player/Money"), (tx("Respect"), "/Player/Respect"), (tx("Days passed"), "/Statistics/Days passed")],
    sections: &[
    Section { title: tx("Player"), rows: &[
        (tx("Money"), "/Player/Money"), (tx("Money shown on screen"), "/Player/Money shown on screen"), (tx("Health"), "/Player/Health"), (tx("Armor"), "/Player/Armor"),
        (tx("Maximum health"), "/Player/Maximum health"), (tx("Respect"), "/Player/Respect"), (tx("Fat"), "/Player/Fat"), (tx("Stamina"), "/Player/Stamina"),
        (tx("Muscle"), "/Player/Muscle"), (tx("Sex appeal"), "/Player/Sex appeal"), (tx("Story line"), "/Player/Story line"),
    ] },
    Section { title: tx("Skills"), rows: &[
        (tx("Driving skill"), "/Skills/Driving skill"), (tx("Flying skill"), "/Skills/Flying skill"), (tx("Lung capacity"), "/Skills/Lung capacity"),
        (tx("Bike skill"), "/Skills/Bike skill"), (tx("Cycling skill"), "/Skills/Cycling skill"), (tx("Gambling"), "/Skills/Gambling"),
        (tx("Pistol"), "/Skills/Pistol"), (tx("Silenced pistol"), "/Skills/Silenced pistol"), (tx("Desert Eagle"), "/Skills/Desert Eagle"),
        (tx("Shotgun"), "/Skills/Shotgun"), (tx("Sawn-off shotgun"), "/Skills/Sawn-off shotgun"), (tx("Combat shotgun"), "/Skills/Combat shotgun"),
        (tx("Machine pistol"), "/Skills/Machine pistol"), (tx("SMG"), "/Skills/SMG"), (tx("AK-47"), "/Skills/AK-47"), (tx("M4"), "/Skills/M4"),
    ] },
    Section { title: tx("Time and statistics"), rows: &[
        (tx("Days passed"), "/Statistics/Days passed"), (tx("Day of the week"), "/Statistics/Day of the week"), (tx("Hour"), "/Statistics/Hour"),
        (tx("Minute"), "/Statistics/Minute"), (tx("Minute length (milliseconds)"), "/Statistics/Minute length (milliseconds)"),
        (tx("Times busted"), "/Statistics/Times busted"), (tx("Times wasted"), "/Statistics/Times wasted"), (tx("Times cheated"), "/Statistics/Times cheated"),
    ] },
    Section { title: tx("Options"), rows: &[
        (tx("Infinite sprint"), "/Options/Infinite sprint"), (tx("Fast reload"), "/Options/Fast reload"), (tx("Fireproof"), "/Options/Fireproof"),
        (tx("Drive-by shooting"), "/Options/Drive-by shooting"), (tx("Free release once busted"), "/Options/Free release once busted"),
        (tx("Free release once wasted"), "/Options/Free release once wasted"), (tx("Riots in Los Santos"), "/Options/Riots in Los Santos"),
        (tx("All taxis have nitro"), "/Options/All taxis have nitro"), (tx("Prostitutes pay you"), "/Options/Prostitutes pay you"),
    ] },
    ],
    lists: &[],
    tables: &[
    Table { title: tx("Weapons"), path: "/Weapons", name: &[], cols: &[(tx("Weapon"), "/Weapon"), (tx("Ammo"), "/Ammo")], only: &[], names: &[] },
    ],
};

// ------------------------------------------------------------------ Single Player Tarkov (SPT)

pub const SPT: Spec = Spec {
    name: "Single Player Tarkov (SPT)",
    tab: "Character",
    accent: (200, 170, 70),
    kind: None,
    markers: &["/characters/pmc/Info/Level", "/characters/pmc/Inventory/items"],
    subtitle: tx("An SPT profile (user/profiles/<id>.json), a plain JSON file. Your PMC's level and experience, skills, trader standing, health and the stacks of roubles, dollars and euros in your stash."),
    tiles: &[(tx("Level"), "/characters/pmc/Info/Level"), (tx("Experience"), "/characters/pmc/Info/Experience")],
    sections: &[
    Section { title: tx("Character"), rows: &[
        (tx("Nickname"), "/characters/pmc/Info/Nickname"), (tx("Level"), "/characters/pmc/Info/Level"), (tx("Experience"), "/characters/pmc/Info/Experience"),
    ] },
    ],
    lists: &[],
    tables: &[
    Table { title: tx("Currency stacks"), path: "/characters/pmc/Inventory/items", name: &["/_tpl"], cols: &[(tx("Amount"), "/upd/StackObjectsCount")], only: &["Roubles", "Dollars", "Euros"],
        names: &[("5449016a4bdc2d6f028b456f", "Roubles"), ("5696686a4bdc2da3298b456a", "Dollars"), ("569668774bdc2da2298b4568", "Euros")] },
    Table { title: tx("Skills"), path: "/characters/pmc/Skills/Common", name: &["/Id"], cols: &[(tx("Progress"), "/Progress")], only: &[], names: &[] },
    Table { title: tx("Trader standing"), path: "/characters/pmc/TradersInfo", name: &[], cols: &[(tx("Standing"), "/standing"), (tx("Loyalty level"), "/loyaltyLevel"), (tx("Sales sum"), "/salesSum"), (tx("Unlocked"), "/unlocked")], only: &[],
        names: &[("54cb50c76803fa8b248b4571", "Prapor"), ("54cb57776803fa99248b456e", "Therapist"), ("579dc571d53a0658a154fbec", "Fence"), ("58330581ace78e27b8b10cee", "Skier"),
            ("5935c25fb3acc3127c3d8cd9", "Peacekeeper"), ("5a7c2eca46aef81a7ca2145d", "Mechanic"), ("5ac3b934156ae10c4430e83c", "Ragman"), ("5c0647fdd443bc2504c2d371", "Jaeger")] },
    Table { title: tx("Health"), path: "/characters/pmc/Health/BodyParts", name: &[], cols: &[(tx("Current"), "/Health/Current"), (tx("Maximum"), "/Health/Maximum")], only: &[], names: &[] },
    ],
};

#[cfg(test)]
mod tests {
    #[test]
    fn spt_profile_is_recognised() {
        let v = serde_json::json!({"characters": {"pmc": {"Info": {"Level": 3, "Experience": 5000}, "Inventory": {"items": []}}}});
        let g = crate::game_ui::detect("JSON", &v).expect("recognised");
        assert!(matches!(g, crate::game_ui::Game::Simple(_)));
        if let crate::game_ui::Game::Simple(i) = g {
            assert_eq!(super::super::game_name(i), "Single Player Tarkov (SPT)");
        }
    }
}

// ------------------------------------------------------------------ Batman: Arkham City

pub const ARKHAM_CITY: Spec = Spec {
    name: "Batman: Arkham City",
    tab: "Progress",
    accent: (200, 190, 60),
    kind: Some(crate::fmt::arkhamcity::KIND),
    markers: &["/Progress/Experience", "/Options/Difficulty"],
    subtitle: tx("A .sgd save of the PC game (Game of the Year or original). Difficulty, experience, player level and the upgrades bought for Batman and Catwoman; the save is unpacked and packed again. Saves of a brand-new game have no experience yet and are not opened."),
    tiles: &[(tx("Player level"), "/Progress/Player level"), (tx("Experience"), "/Progress/Experience")],
    sections: &[
    Section { title: tx("Progress"), rows: &[
        (tx("Difficulty"), "/Options/Difficulty"), (tx("Experience"), "/Progress/Experience"), (tx("Player level"), "/Progress/Player level"),
        (tx("Experience at the last upgrade point"), "/Progress/Experience at the last upgrade point"),
        (tx("Experience for the next upgrade point"), "/Progress/Experience for the next upgrade point"),
        (tx("Upgrades bought (Batman)"), "/Progress/Upgrades bought (Batman)"), (tx("Upgrades bought (Catwoman)"), "/Progress/Upgrades bought (Catwoman)"),
    ] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Tales of Vesperia: Definitive Edition

pub const TOV: Spec = Spec {
    name: "Tales of Vesperia: Definitive Edition",
    tab: "Party",
    accent: (90, 150, 220),
    kind: Some(crate::fmt::tov::KIND),
    markers: &["/Party/Gald", "/Characters/Yuri/Level"],
    subtitle: tx("TLSaveData0_NN in the Steam remote folder. Gald (also in the save list), and level, experience, HP, TP and the base stats of all nine characters."),
    tiles: &[(tx("Gald"), "/Party/Gald")],
    sections: &[
    Section { title: tx("Party"), rows: &[(tx("Gald"), "/Party/Gald"), (tx("Play time (hours)"), "/Party/Play time (hours)")] },
    ],
    lists: &[],
    tables: &[
    Table { title: tx("Characters"), path: "/Characters", name: &[], cols: &[
        (tx("Level"), "/Level"), (tx("Experience"), "/Experience"), (tx("HP"), "/HP"), (tx("Max HP"), "/Max HP"), (tx("TP"), "/TP"), (tx("Max TP"), "/Max TP"),
        (tx("Attack"), "/Attack"), (tx("Magic attack"), "/Magic attack"), (tx("Defense"), "/Defense"), (tx("Magic defense"), "/Magic defense"),
        (tx("Agility"), "/Agility"), (tx("Luck"), "/Luck"),
    ], only: &[], names: &[] },
    ],
};

// ------------------------------------------------------------------ Sonic Adventure 2 (Chao garden)

pub const SA2_CHAO: Spec = Spec {
    name: "Sonic Adventure 2",
    tab: "Chao",
    accent: (70, 130, 230),
    kind: Some(crate::fmt::sa2chao::KIND),
    markers: &[],
    subtitle: tx("The Chao garden file SONIC2B__ALF (the PC release). The level and the points of every stat for each Chao in the garden; the garden's checksum is rewritten."),
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[
    Table { title: tx("Stat levels"), path: "", name: &[], cols: &[(tx("Swim"), "/Swim level"), (tx("Fly"), "/Fly level"), (tx("Run"), "/Run level"), (tx("Power"), "/Power level"), (tx("Stamina"), "/Stamina level"), (tx("Luck"), "/Luck level"), (tx("Intelligence"), "/Intelligence level")], only: &[], names: &[] },
    Table { title: tx("Stat points"), path: "", name: &[], cols: &[(tx("Swim"), "/Swim points"), (tx("Fly"), "/Fly points"), (tx("Run"), "/Run points"), (tx("Power"), "/Power points"), (tx("Stamina"), "/Stamina points"), (tx("Luck"), "/Luck points"), (tx("Intelligence"), "/Intelligence points")], only: &[], names: &[] },
    ],
};

// ------------------------------------------------------------------ Bug Fables: The Everlasting Sapling

pub const BUG_FABLES: Spec = Spec {
    name: "Bug Fables: The Everlasting Sapling",
    tab: "Team Snakemouth",
    accent: (230, 180, 60),
    kind: None,
    markers: &["/Bug Fables/Berries", "/Party"],
    subtitle: tx("save0.dat, save1.dat or save2.dat in the game's folder (the PC release). Berries, rank, experience, TP, MP, the inventory and storage sizes and play time, and the HP, attack and defense of every party member. Only these values are rewritten; the rest of the save is kept as it was."),
    tiles: &[(tx("Berries"), "/Bug Fables/Berries"), (tx("Rank"), "/Bug Fables/Rank"), (tx("Experience"), "/Bug Fables/Experience")],
    sections: &[
    Section { title: tx("Team"), rows: &[
        (tx("Berries"), "/Bug Fables/Berries"), (tx("Rank"), "/Bug Fables/Rank"), (tx("Experience"), "/Bug Fables/Experience"),
        (tx("Experience needed"), "/Bug Fables/Experience needed"), (tx("TP"), "/Bug Fables/TP"), (tx("Max TP"), "/Bug Fables/Max TP"),
        (tx("MP"), "/Bug Fables/MP"), (tx("Max MP"), "/Bug Fables/Max MP"), (tx("Inventory slots"), "/Bug Fables/Inventory slots"),
        (tx("Storage slots"), "/Bug Fables/Storage slots"),
    ] },
    Section { title: tx("Play time"), rows: &[
        (tx("Hours"), "/Bug Fables/Play time (hours)"), (tx("Minutes"), "/Bug Fables/Play time (minutes)"), (tx("Seconds"), "/Bug Fables/Play time (seconds)"),
    ] },
    ],
    lists: &[],
    tables: &[
    Table { title: tx("Party"), path: "/Party", name: &["/Member"], cols: &[
        (tx("HP"), "/HP"), (tx("Max HP"), "/Max HP"), (tx("Base HP"), "/Base HP"), (tx("Attack"), "/Attack"),
        (tx("Base attack"), "/Base attack"), (tx("Defense"), "/Defense"), (tx("Base defense"), "/Base defense"),
    ], only: &[], names: &[] },
    ],
};

// ------------------------------------------------------------------ Planetbase

pub const PLANETBASE: Spec = Spec {
    name: "Planetbase",
    tab: "Colony",
    accent: (90, 170, 200),
    kind: Some("XML"),
    markers: &["/save-game/colony/extra-prestige", "/save-game/characters"],
    subtitle: tx("A .sav file in Documents\\Planetbase\\Saves (plain XML). Coins (once the colony has traded), extra prestige, the landing permissions, and the health, food, water, oxygen, sleep and morale of every colonist and the condition of every bot, from 0 to 1. Goods in storage are single items on the ground and are not listed."),
    tiles: &[(tx("Coins"), "/save-game/resources/inmaterial-resources/amount/amount/@value"), (tx("Extra prestige"), "/save-game/colony/extra-prestige/@value")],
    sections: &[
    Section { title: tx("Colony"), rows: &[
        (tx("Colony name"), "/save-game/colony/name/@value"), (tx("Coins"), "/save-game/resources/inmaterial-resources/amount/amount/@value"),
        (tx("Extra prestige"), "/save-game/colony/extra-prestige/@value"),
    ] },
    Section { title: tx("Landing permissions"), rows: &[
        (tx("Colonists allowed"), "/save-game/ship-manager/landing-permissions/colonists-allowed/@value"),
        (tx("Merchants allowed"), "/save-game/ship-manager/landing-permissions/merchants-allowed/@value"),
        (tx("Visitors allowed"), "/save-game/ship-manager/landing-permissions/visitors-allowed/@value"),
        (tx("Workers (%)"), "/save-game/ship-manager/landing-permissions/Worker-percentage/@value"),
        (tx("Biologists (%)"), "/save-game/ship-manager/landing-permissions/Biologist-percentage/@value"),
        (tx("Engineers (%)"), "/save-game/ship-manager/landing-permissions/Engineer-percentage/@value"),
        (tx("Medics (%)"), "/save-game/ship-manager/landing-permissions/Medic-percentage/@value"),
        (tx("Guards (%)"), "/save-game/ship-manager/landing-permissions/Guard-percentage/@value"),
    ] },
    ],
    lists: &[],
    tables: &[
    Table { title: tx("Colonists and bots"), path: "/save-game/characters/character", name: &["/name/@value"], cols: &[
        (tx("Health"), "/Health/@value"), (tx("Nutrition"), "/Nutrition/@value"), (tx("Hydration"), "/Hydration/@value"), (tx("Oxygen"), "/Oxygen/@value"),
        (tx("Sleep"), "/Sleep/@value"), (tx("Morale"), "/Morale/@value"), (tx("Condition"), "/Condition/@value"), (tx("Integrity"), "/Integrity/@value"),
    ], only: &[], names: &[] },
    ],
};
