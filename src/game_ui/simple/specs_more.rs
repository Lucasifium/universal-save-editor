//! More games for the table-driven screens: one `Spec` per game, as in `specs.rs`. Values are only
//! listed once they read correctly on a real save.

use super::{List, Max, Section, Shape, Spec, Table};
use crate::i18n::tx;

pub static MORE: &[Spec] = &[STRANDED_DEEP, LOKR, BANNER_SAGA3, PROJECT_WARLOCK, HORIZON_CHASE, DOORKICKERS, DOORKICKERS_DOCTRINE, DOORKICKERS_UNLOCKED, SATELLITE_REIGN, TOOTH_AND_TAIL, CHEF, XREBIRTH, ETHERLORDS, NECROMUNDA, FOOD_TRUCK, WORKSHOP_SIM, PATHWAY, GLADIATOR_GUILD, CAT_GOES_FISHING, THYMESIA, KING_ARTHUR];

// ------------------------------------------------------------------ House Flipper


// ------------------------------------------------------------------ Stranded Deep

const STRANDED_DEEP: Spec = Spec {
    name: "Stranded Deep",
    tab: "Survivor",
    accent: (60, 170, 200),
    kind: None,
    markers: &["/Persistent/PlayerRegistry/0/Statistics/player_health", "/Persistent/PlayerRegistry/0/Skills"],
    subtitle: tx("The <SteamID>.json in a save's folder under AppData\\LocalLow\\Beam Team Games\\Stranded Deep\\Data. Health, food, water and sleep, the five skills and the day count. Close the game before editing."),
    tiles: &[(tx("Health"), "/Persistent/PlayerRegistry/0/Statistics/player_health")],
    sections: &[
        Section {
            title: tx("Vitals (10,000 is full)"),
            rows: &[
                (tx("Health"), "/Persistent/PlayerRegistry/0/Statistics/player_health"), (tx("Food (calories)"), "/Persistent/PlayerRegistry/0/Statistics/playerCalorie"),
                (tx("Water"), "/Persistent/PlayerRegistry/0/Statistics/playerBodyfluid"), (tx("Sleep"), "/Persistent/PlayerRegistry/0/Statistics/Sleep"),
                (tx("Breath (100 is full)"), "/Persistent/PlayerRegistry/0/Statistics/playerBreath"), (tx("Body temperature (°F)"), "/Persistent/PlayerRegistry/0/Statistics/playerBodytemp"),
            ],
        },
        Section {
            title: tx("Skill experience"),
            rows: &[
                (tx("Hunting"), "/Persistent/PlayerRegistry/0/Skills/hunting"), (tx("Cooking"), "/Persistent/PlayerRegistry/0/Skills/cooking"),
                (tx("Harvesting"), "/Persistent/PlayerRegistry/0/Skills/harvesting"), (tx("Physical"), "/Persistent/PlayerRegistry/0/Skills/physical"),
                (tx("Craftsmanship"), "/Persistent/PlayerRegistry/0/Skills/craftsmanship"),
            ],
        },
        Section {
            title: tx("World"),
            rows: &[
                (tx("Days elapsed"), "/Persistent/GameCalendar/DaysElapsed"), (tx("Day of the month"), "/Persistent/GameCalendar/Day"),
                (tx("Month"), "/Persistent/GameCalendar/Month"), (tx("Year"), "/Persistent/GameCalendar/Year"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Tavern Master


// ------------------------------------------------------------------ Legends of Kingdom Rush

const LOKR: Spec = Spec {
    name: "Legends of Kingdom Rush",
    tab: "Heroes",
    accent: (200, 80, 60),
    kind: None,
    markers: &["/heroRosterData/data", "/saveVersion"],
    subtitle: tx("A slot_N_steam.json save. Each hero's experience and whether the hero is unlocked, and the gems, stars, food and fame of a run in progress."),
    tiles: &[(tx("Gems"), "/run/guildData/gems"), (tx("Stars"), "/run/guildData/stars"), (tx("Fame"), "/run/guildData/fame")],
    sections: &[Section {
        title: tx("Run in progress"),
        rows: &[(tx("Gems"), "/run/guildData/gems"), (tx("Stars"), "/run/guildData/stars"), (tx("Fame"), "/run/guildData/fame"), (tx("Food"), "/run/guildData/food"), (tx("Loot"), "/run/guildData/loot"), (tx("Guild experience"), "/run/guildData/xp")],
    }],
    lists: &[],
    tables: &[Table { title: tx("Heroes"), path: "/heroRosterData/data", name: &["/id"], only: &[], cols: &[(tx("Experience"), "/xp"), (tx("Unlocked"), "/unlocked")], names: &[] }],
};

// ------------------------------------------------------------------ The Banner Saga 3

const BANNER_SAGA3: Spec = Spec {
    name: "The Banner Saga 3",
    tab: "Caravan",
    accent: (200, 160, 80),
    kind: None,
    markers: &["/caravans/0/legend/renown", "/cast_info"],
    subtitle: tx("resume.save.json or a sav_*.save.json from %APPDATA%\\TheBannerSaga3\\Local Store\\save\\saga3. The caravan's renown and supplies and each hero's rank, kills and stat bonuses. Several figures are stored as text; keep them whole numbers."),
    tiles: &[(tx("Renown"), "/caravans/0/legend/renown"), (tx("Supplies"), "/caravans/0/vars/supplies")],
    sections: &[Section {
        title: tx("Caravan"),
        rows: &[
            (tx("Renown"), "/caravans/0/legend/renown"), (tx("Renown (second copy, keep equal)"), "/caravans/0/vars/renown"), (tx("Supplies"), "/caravans/0/vars/supplies"),
            (tx("Morale"), "/caravans/0/vars/morale"), (tx("Fighters"), "/caravans/0/vars/num_fighters"), (tx("Peasants"), "/caravans/0/vars/num_peasants"), (tx("Varl"), "/caravans/0/vars/num_varl"),
            (tx("Battles won"), "/caravans/0/vars/tot_battles_won"), (tx("Kills"), "/caravans/0/vars/tot_kills"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: tx("Heroes"),
        path: "/cast_info",
        name: &[],
        only: &[],
        cols: &[(tx("Rank"), "/stat_ups/RANK"), (tx("Kills"), "/stat_ups/KILLS"), (tx("Strength"), "/stat_ups/STRENGTH"), (tx("Armor"), "/stat_ups/ARMOR"), (tx("Willpower"), "/stat_ups/WILLPOWER"), (tx("Exertion"), "/stat_ups/EXERTION")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Project Warlock

const PROJECT_WARLOCK: Spec = Spec {
    name: "Project Warlock",
    tab: "Warlock",
    accent: (200, 50, 50),
    kind: None,
    markers: &["/IntData/=Score/Value", "/IntData/=CharLevel/Value"],
    subtitle: tx("The gamesave file in AppData\\LocalLow\\Buckshot Software\\Project Warlock\\Steam\\<id>. Score, lives, health, mana, the four stats, points to spend and every weapon's ammunition."),
    tiles: &[(tx("Lives"), "/IntData/=Lives/Value"), (tx("Level"), "/IntData/=CharLevel/Value"), (tx("Score"), "/IntData/=Score/Value")],
    sections: &[
        Section {
            title: tx("Warlock"),
            rows: &[
                (tx("Level"), "/IntData/=CharLevel/Value"), (tx("Experience"), "/FloatData/=CurrentEXP/Value"), (tx("Experience needed"), "/IntData/=MaxEXP/Value"), (tx("Score"), "/IntData/=Score/Value"),
                (tx("Lives"), "/IntData/=Lives/Value"), (tx("Health"), "/IntData/=Health/Value"), (tx("Maximum health"), "/IntData/=MaxHealth/Value"), (tx("Mana"), "/IntData/=Mana/Value"),
                (tx("Maximum mana"), "/IntData/=MaxMana/Value"), (tx("Strength"), "/IntData/=Strength/Value"), (tx("Vitality"), "/IntData/=Vitality/Value"), (tx("Wisdom"), "/IntData/=Wisdom/Value"),
                (tx("Capacity"), "/IntData/=Capacity/Value"), (tx("Stat points to spend"), "/IntData/=StatPoints/Value"), (tx("Perk points to spend"), "/IntData/=PerkPoint/Value"),
                (tx("Upgrade points to spend"), "/IntData/=UnlockPoints/Value"), (tx("Upgrade points earned"), "/IntData/=UnlockPointsTotal/Value"), (tx("Difficulty"), "/IntData/=Difficulty/Value"),
            ],
        },
        Section {
            title: tx("Ammunition"),
            rows: &[
                (tx("Pistol"), "/IntData/=PistolAmmo/Value"), (tx("Submachine gun"), "/IntData/=SMGAmmo/Value"), (tx("Shotgun"), "/IntData/=ShotgunAmmo/Value"), (tx("Dynamite"), "/IntData/=DynamiteAmmo/Value"),
                (tx("Crossbow"), "/IntData/=crossbowAmmo/Value"), (tx("Rocket launcher"), "/IntData/=RocketLauncherAmmo/Value"), (tx("Railgun"), "/IntData/=railgunAmmo/Value"),
                (tx("Chain reactor"), "/IntData/=ChainReactorAmmo/Value"), (tx("Flamethrower fuel"), "/FloatData/=FuelAmmo/Value"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Horizon Chase Turbo

const HORIZON_CHASE: Spec = Spec {
    name: "Horizon Chase Turbo",
    tab: "Driver",
    accent: (240, 90, 160),
    kind: None,
    markers: &["/CurrentCampaignData/NumberOfTokens", "/CampaignSlots/0/NumberOfTokens"],
    subtitle: tx("The 1000_userprofile!<number>!.sav file of the profile folder. Campaign tokens and race counts; the token count is stored in three places, so change all three to the same value. Close the game before editing."),
    tiles: &[(tx("Tokens"), "/CurrentCampaignData/NumberOfTokens"), (tx("Races"), "/CurrentCampaignData/TotalRaceCount")],
    sections: &[Section {
        title: tx("Campaign"),
        rows: &[
            (tx("Tokens"), "/CurrentCampaignData/NumberOfTokens"), (tx("Tokens (profile copy, keep equal)"), "/NumberOfTokens"), (tx("Tokens (campaign slot copy, keep equal)"), "/CampaignSlots/0/NumberOfTokens"),
            (tx("Races"), "/CurrentCampaignData/TotalRaceCount"), (tx("Races (profile copy, keep equal)"), "/TotalRaceCount"), (tx("Total race time (seconds)"), "/CurrentCampaignData/TotalRaceTime"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Thief Simulator


// ------------------------------------------------------------------ Door Kickers

const DOORKICKERS: Spec = Spec {
    name: "Door Kickers",
    tab: "Squad",
    accent: (70, 110, 200),
    kind: Some("XML"),
    markers: &["/Roster/@xp", "/Roster/Trooper"],
    subtitle: tx("roster.xml from Documents\\KillHouseGames\\DoorKickers. The squad's experience and each trooper's experience, kills and mission results. doctrine.xml and unlocked.xml have their own small screens."),
    tiles: &[(tx("Squad experience"), "/Roster/@xp")],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: tx("Troopers"),
        path: "/Roster/Trooper",
        name: &["/Id/@name"],
        only: &[],
        cols: &[(tx("Experience"), "/Stats/@XP"), (tx("Kills"), "/Stats/@Kills"), (tx("Injuries"), "/Stats/@Injuries"), (tx("1-star wins"), "/Stats/@MissionsWon1Star"), (tx("2-star wins"), "/Stats/@MissionsWon2Star"), (tx("3-star wins"), "/Stats/@MissionsWon3Star")],
        names: &[],
    }],
};

const DOORKICKERS_DOCTRINE: Spec = Spec {
    name: "Door Kickers",
    tab: "Doctrine",
    accent: (70, 110, 200),
    kind: Some("XML"),
    markers: &["/Doctrine/@availablePoints"],
    subtitle: tx("doctrine.xml from Documents\\KillHouseGames\\DoorKickers: the doctrine points still to spend. The chosen abilities are in the Tree."),
    tiles: &[(tx("Doctrine points to spend"), "/Doctrine/@availablePoints")],
    sections: &[],
    lists: &[],
    tables: &[],
};

const DOORKICKERS_UNLOCKED: Spec = Spec {
    name: "Door Kickers",
    tab: "Stars",
    accent: (70, 110, 200),
    kind: Some("XML"),
    markers: &["/Unlocked/@numStarsUsed"],
    subtitle: tx("unlocked.xml from Documents\\KillHouseGames\\DoorKickers: the stars spent on equipment. A lower (even negative) number leaves more stars to spend. The unlocked equipment is listed in the Tree."),
    tiles: &[(tx("Stars used"), "/Unlocked/@numStarsUsed")],
    sections: &[],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Satellite Reign

const SATELLITE_REIGN: Spec = Spec {
    name: "Satellite Reign",
    tab: "Syndicate",
    accent: (60, 200, 220),
    kind: Some("XML"),
    markers: &["/SaveGame/m_MoneyManagerSave/m_PlayersCurrentFunds"],
    subtitle: tx("srbSave<N>.xml or srbAutoSave.xml from AppData\\Local\\SatelliteReign\\<profile>. Your funds and the progress counters. Agents, research and the world are in the Tree."),
    tiles: &[(tx("Funds"), "/SaveGame/m_MoneyManagerSave/m_PlayersCurrentFunds")],
    sections: &[Section {
        title: tx("Syndicate"),
        rows: &[
            (tx("Funds"), "/SaveGame/m_MoneyManagerSave/m_PlayersCurrentFunds"), (tx("Researchers"), "/SaveGame/m_ProgressionManagerSave/m_ResearcherCount"),
            (tx("Scientists"), "/SaveGame/m_ProgressionManagerSave/m_ScientistCount"), (tx("Doctors"), "/SaveGame/m_ProgressionManagerSave/m_DoctorCount"),
            (tx("Enemies killed"), "/SaveGame/m_ProgressionManagerSave/m_EnemiesKilled"), (tx("Enemies killed unseen"), "/SaveGame/m_ProgressionManagerSave/m_EnemiesKilledWithoutBeingSeen"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Tooth and Tail

const TOOTH_AND_TAIL: Spec = Spec {
    name: "Tooth and Tail",
    tab: "Campaign",
    accent: (210, 150, 70),
    kind: Some("XML"),
    markers: &["/SaveData/CampaignProgress/item"],
    subtitle: tx("SaveData.xml from %APPDATA%\\ToothAndTail. Which campaign missions are marked complete (true or false)."),
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[Table { title: tx("Campaign missions"), path: "/SaveData/CampaignProgress/item", name: &["/@key"], only: &[], cols: &[(tx("Complete"), "/@value")], names: &[] }],
};

// ------------------------------------------------------------------ Chef: A Restaurant Tycoon Game

const CHEF: Spec = Spec {
    name: "Chef: A Restaurant Tycoon Game",
    tab: "Restaurant",
    accent: (230, 120, 60),
    kind: Some("XML"),
    markers: &["/LogicState/PlayerProfile/Budget/CurrentBudgetData/BudgetData/TotalBudget", "/LogicState/RestaurantsList/Restaurant/Level"],
    subtitle: tx("A save_<n>.xml from AppData\\LocalLow\\Inner Void Interactive\\Chef_ A Restaurant Tycoon Game. The budget, popularity, ingredient points and the restaurant's level and rating."),
    tiles: &[(tx("Budget"), "/LogicState/PlayerProfile/Budget/CurrentBudgetData/BudgetData/TotalBudget"), (tx("Popularity"), "/LogicState/PlayerProfile/Popularity")],
    sections: &[Section {
        title: tx("Restaurant"),
        rows: &[
            (tx("Restaurant name"), "/LogicState/RestaurantsList/Restaurant/Name"), (tx("Budget"), "/LogicState/PlayerProfile/Budget/CurrentBudgetData/BudgetData/TotalBudget"),
            (tx("Budget at the end of the last day"), "/LogicState/PlayerProfile/Budget/PreviousBudgetData/BudgetData/TotalBudget"), (tx("Popularity"), "/LogicState/PlayerProfile/Popularity"),
            (tx("Ingredient points"), "/LogicState/PlayerProfile/IngredientsManager/IngredientPoints"), (tx("Restaurant level"), "/LogicState/RestaurantsList/Restaurant/Level"),
            (tx("Rating"), "/LogicState/RestaurantsList/Restaurant/Rating"), (tx("Seats"), "/LogicState/RestaurantsList/Restaurant/AvailableSeats"), (tx("City map points"), "/LogicState/CityMapManager/CityMapPoints"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ X Rebirth

const XREBIRTH: Spec = Spec {
    name: "X Rebirth",
    tab: "Pilot",
    accent: (90, 150, 230),
    kind: Some("XML"),
    markers: &["/savegame/info/player/@money"],
    subtitle: tx("A save_NNN.xml (or its .xml.gz) from Documents\\Egosoft\\X Rebirth\\<id>\\save. Your credits and name. The file is large, so it takes a few seconds to open; ships, stations and wares are in the Tree."),
    tiles: &[(tx("Credits"), "/savegame/info/player/@money")],
    sections: &[Section {
        title: tx("Pilot"),
        rows: &[(tx("Credits"), "/savegame/info/player/@money"), (tx("Name"), "/savegame/info/player/@name"), (tx("Game time (seconds)"), "/savegame/info/game/@time")],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Etherlords

const ETHERLORDS: Spec = Spec {
    name: "Etherlords",
    tab: "Lord",
    accent: (130, 90, 210),
    kind: Some("XML"),
    markers: &["/game/players/player/0/resources/resource", "/game/campaign"],
    subtitle: tx("A .savc file from the game's Save folder. Each player's elemental resources and remaining mana, and the campaign's points and day. Heroes and the map are in the Tree."),
    tiles: &[(tx("Campaign points"), "/game/campaign/@points"), (tx("Day"), "/game/@day")],
    sections: &[
        Section { title: tx("Campaign"), rows: &[(tx("Campaign points"), "/game/campaign/@points"), (tx("Day"), "/game/@day")] },
        Section { title: tx("Mana left"), rows: &[(tx("Player 1"), "/game/players/player/0/mana/@rest"), (tx("Player 2"), "/game/players/player/1/mana/@rest")] },
    ],
    lists: &[],
    tables: &[
        Table { title: tx("Resources of player 1"), path: "/game/players/player/0/resources/resource", name: &["/@type"], only: &[], cols: &[(tx("Amount"), "/@amount")], names: &[] },
        Table { title: tx("Resources of player 2"), path: "/game/players/player/1/resources/resource", name: &["/@type"], only: &[], cols: &[(tx("Amount"), "/@amount")], names: &[] },
    ],
};


// ------------------------------------------------------------------ Farming Simulator 15, 17 and 19




// ------------------------------------------------------------------ Necromunda: Hired Gun

const NECROMUNDA: Spec = Spec {
    name: "Necromunda: Hired Gun",
    tab: "Hired Gun",
    accent: (200, 110, 40),
    kind: Some("Unreal Engine save (NMPlayerSaveSlot)"),
    markers: &["/Credits", "/CurrentLevel", "/ActualFactionsXP"],
    subtitle: tx("Campaign1.sav from %LOCALAPPDATA%\\Streumon\\Necromunda\\Saved\\SaveGames. Credits, level, experience and each faction's reputation experience (the game also keeps a last-seen copy that follows it). Close the game before editing."),
    tiles: &[(tx("Credits"), "/Credits"), (tx("Level"), "/CurrentLevel")],
    sections: &[Section {
        title: tx("Hired Gun"),
        rows: &[(tx("Credits"), "/Credits"), (tx("Level"), "/CurrentLevel"), (tx("Experience"), "/CurrentXP"), (tx("New Game Plus"), "/NewGamePlus")],
    }],
    lists: &[],
    tables: &[Table { title: tx("Faction experience"), path: "/ActualFactionsXP", name: &["/FactionType"], only: &[], cols: &[(tx("Experience"), "/CurrentXP")], names: &[] }],
};

// ------------------------------------------------------------------ Gas Station Simulator


// ------------------------------------------------------------------ Food Truck Simulator

const FOOD_TRUCK: Spec = Spec {
    name: "Food Truck Simulator",
    tab: "Truck",
    accent: (230, 110, 60),
    kind: Some("Unreal Engine save (FTRSaveGame)"),
    markers: &["/SavedMoney", "/SavedPrestige"],
    subtitle: tx("A .sav from %LOCALAPPDATA%\\FTR3\\Saved\\SaveGames. Money, prestige and the fuel in the truck. Close the game before editing."),
    tiles: &[(tx("Money"), "/SavedMoney"), (tx("Prestige"), "/SavedPrestige")],
    sections: &[Section { title: tx("Truck"), rows: &[(tx("Money"), "/SavedMoney"), (tx("Prestige"), "/SavedPrestige"), (tx("Fuel"), "/SavedFuel"), (tx("Gas in the bottle"), "/SavedGasBottle/GasAmountCurrent")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Workshop Simulator

const WORKSHOP_SIM: Spec = Spec {
    name: "Workshop Simulator",
    tab: "Workshop",
    accent: (90, 150, 210),
    kind: Some("Unreal Engine save (WorkshopSaveGame)"),
    markers: &["/AccountBalance", "/TotalOrdersFinished"],
    subtitle: tx("WorkshopSave_<n>.sav or AutoSave.sav from %LOCALAPPDATA%\\WS2020\\Saved\\SaveGames. The account balance and the totals earned and orders finished. The file is large, so it takes a moment to open."),
    tiles: &[(tx("Account balance"), "/AccountBalance")],
    sections: &[Section { title: tx("Workshop"), rows: &[(tx("Account balance"), "/AccountBalance"), (tx("Total money earned"), "/TotalMoneyEarned"), (tx("Orders finished"), "/TotalOrdersFinished")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Pathway

const PATHWAY: Spec = Spec {
    name: "Pathway",
    tab: "Expedition",
    accent: (210, 160, 70),
    kind: None,
    markers: &["/exploration/player/resources/cash", "/exploration/time/day"],
    subtitle: tx("save0.bin (a gzip file with JSON inside, opened and packed again) from AppData\\LocalLow\\Robotality\\Pathway or its Local folder. The expedition's cash, fuel, ammunition, supplies and the day. Close the game before editing."),
    tiles: &[(tx("Cash"), "/exploration/player/resources/cash")],
    sections: &[
        Section {
            title: tx("Resources"),
            rows: &[
                (tx("Cash"), "/exploration/player/resources/cash"), (tx("Fuel"), "/exploration/player/resources/fuel"), (tx("Largest fuel tank"), "/exploration/player/resources/maxFuel"),
                (tx("Ammunition"), "/exploration/player/resources/ammo"), (tx("Largest ammunition store"), "/exploration/player/resources/maxAmmo"),
                (tx("Supplies"), "/exploration/player/resources/supplies"), (tx("Largest supplies store"), "/exploration/player/resources/maxSupplies"),
            ],
        },
        Section {
            title: tx("Time and statistics"),
            rows: &[
                (tx("Day"), "/exploration/time/day"), (tx("Fuel used"), "/exploration/stats/fuelUsed"), (tx("Ammunition used"), "/exploration/stats/ammoUsed"),
                (tx("Items collected"), "/exploration/stats/itemsCollected"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Gladiator Guild Manager

const GLADIATOR_GUILD: Spec = Spec {
    name: "Gladiator Guild Manager",
    tab: "Guild",
    accent: (190, 90, 60),
    kind: Some(".NET BinaryFormatter (PLayerData)"),
    markers: &["/gold", "/manaCristals"],
    subtitle: tx("Save_v<version>.ggm from AppData\\LocalLow\\Entertainment Forge\\Gladiator Manager (not the _Quests or Options files). The guild's gold, building materials, mana crystals, loan and the day. Gladiators and buildings are in the Tree, packed in text. Close the game before editing."),
    tiles: &[(tx("Gold"), "/gold")],
    sections: &[Section {
        title: tx("Guild"),
        rows: &[
            (tx("Gold"), "/gold"), (tx("Wood"), "/wood"), (tx("Iron"), "/iron"), (tx("Stone"), "/stone"), (tx("Mana crystals"), "/manaCristals"),
            (tx("Loan debt"), "/loanDebt"), (tx("Loans available"), "/loansAvaialble"), (tx("Current day"), "/currentDay"), (tx("Difficulty"), "/difficulty"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Cat Goes Fishing

const CAT_GOES_FISHING: Spec = Spec {
    name: "Cat Goes Fishing",
    tab: "Angler",
    accent: (90, 170, 220),
    kind: Some("INI / config"),
    markers: &["/Main/dollars", "/Main/experience"],
    subtitle: tx("sdata<N>.ini from AppData\\Local\\Cat_Goes_Fishing. Your money, level and experience, the chosen rod and gear and the game's counters. The unlocks are in the Tree. Close the game before editing."),
    tiles: &[(tx("Money"), "/Main/dollars")],
    sections: &[Section {
        title: tx("Angler"),
        rows: &[
            (tx("Money"), "/Main/dollars"), (tx("Level"), "/Main/level"), (tx("Experience"), "/Main/experience"),
            (tx("Active rod"), "/Main/activerod"), (tx("Gear"), "/Main/gear"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Thymesia

const THYMESIA: Spec = Spec {
    name: "Thymesia",
    tab: "Corvus",
    accent: (120, 60, 140),
    kind: Some("Unreal Engine save (PlayerGameSave_C)"),
    markers: &["/CurrentExp", "/NumPotions"],
    subtitle: tx("PlayerSave<N>.sav from %LOCALAPPDATA%\\Thymesia\\Saved\\SaveGames. The experience, the number of healing potions, the play time and a table of the carried items with their quantities. Close the game before editing."),
    tiles: &[(tx("Experience"), "/CurrentExp")],
    sections: &[Section {
        title: tx("Character"),
        rows: &[(tx("Experience"), "/CurrentExp"), (tx("Healing potions"), "/NumPotions"), (tx("Play time (seconds)"), "/TotalPlayedTimeInSeconds")],
    }],
    lists: &[],
    tables: &[Table {
        title: tx("Items"),
        path: "/Items",
        name: &["/ItemClass_5_E011FB07474AE88DE12C199DE8B52EAE"],
        only: &[],
        cols: &[(tx("Quantity"), "/Quantity_2_E0AE79EF4A4E386C95FE7C9479CAD4DC")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ King Arthur: Knight's Tale

const KING_ARTHUR: Spec = Spec {
    name: "King Arthur: Knight's Tale",
    tab: "Camelot",
    accent: (190, 150, 60),
    kind: Some("King Arthur save (C2AR)"),
    markers: &["/Adventure.cfg/Gold", "/Adventure.cfg/Heroes"],
    subtitle: tx("A .sav from Documents\\NeocoreGames\\KA_KT\\Users\\<Steam id>\\Save (the container is opened and packed again). Gold, relic dust, soul, spire shards and every hero's level, experience and skill points. Close the game before editing."),
    tiles: &[(tx("Gold"), "/Adventure.cfg/Gold")],
    sections: &[Section {
        title: tx("Camelot"),
        rows: &[
            (tx("Gold"), "/Adventure.cfg/Gold"), (tx("Relic dust"), "/Adventure.cfg/RelicDust"), (tx("Soul"), "/Adventure.cfg/Soul"),
            (tx("Spire shards"), "/Adventure.cfg/SpireShard"), (tx("Resurrection tokens"), "/Adventure.cfg/ResurrectionTokens"), (tx("Turn"), "/Adventure.cfg/Turn"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: tx("Heroes"),
        path: "/Adventure.cfg/Heroes",
        name: &["/HeroType"],
        only: &[],
        cols: &[(tx("Level"), "/Level"), (tx("Experience"), "/xp"), (tx("Skill points"), "/extraskillpoints"), (tx("Enemies killed"), "/EnemiesKilled")],
        names: &[],
    }],
};
