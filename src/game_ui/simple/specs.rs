//! One `Spec` per game. Names and ids come from public editors (credited in
//! THIRD_PARTY_NOTICES.md); values are only listed once they read correctly on a real save.

use super::ps_specs::*;
use super::retro_specs::*;
use super::{List, Max, Section, Shape, Spec, Table};
use crate::i18n::tx;

pub static SPECS: &[Spec] = &[OPENXCOM, XCOM_DOS, RCT2, COTL, FALLOUT_SHELTER, DRG, S1_MONEY, S1_RANK, S1_TIME, S1_GAME, SOTF_STATE, SOTF_GAME, SOTF_WEATHER, SOTF_ITEMS, DMD, CUPHEAD, ANTIMATTER, PVZ, CELESTE, DRGS, KH3, RF4S, SKY1ST, KSP, FFPR, MCD, STONESHARD, ROGUE_TRADER, BIOSHOCK_INFINITE, SPACE_HAVEN, SNOWRUNNER, GRAVEYARD_KEEPER, NFSMW, TOS, GTA3, GTAVC, BITBURNER, TERRA_INVICTA, PRISON, FF13, SONIC_MANIA, CAVE_STORY_PLUS, CAVE_STORY, LIES_OF_P, TXR, DAVE, MSC, ULTRAKILL, RPGXP, RE5, ANNO1800, FTK, LEGO_TSS, LEGO_CLASSIC, MYZ, CMS21, MEDIEVAL_DYNASTY, MECHANICUS, ORI, OMORI, ENDER_LILIES, SW2, ES2, MOONLIGHTER, WASTELAND2, ATOM, MORTAL_SHELL, MORTAL_SHELL_2, COTW, BEAMNG, FOREST, YAKUZA3, YAKUZA4, YAKUZA5, ISHIN, SALT, DRAKENSANG, ALPHA, TYRANNY, JEDI_FO, SR3, SR4, WD2, FABLE, WANDERING_SWORD, STELLAR_BLADE, PRAGMATA, SLIME_RANCHER, SUBNAUTICA2, HZD, SLEEPING_DOGS, DXMD, ATOMIC_HEART, DISHONORED2, DEATHLOOP, AC_BROTHERHOOD, AC_REVELATIONS, AC3R, AC_LIBERATION, AC_BLACK_FLAG, AC_ROGUE, AC_UNITY, AC_SYNDICATE, AC_ORIGINS, AC_ODYSSEY, DISHONORED1, RDR1, RDR2, WORLDBOX, COREKEEPER, HOLOCURE, POTION_CRAFT, WWZ, ROBOQUEST, CMS18, LONGDRIVE, LONGDRIVE_PROP, XC3, MLDT, SMASH, MK7, SKC, UTY, DAGGERFALL_UNITY, DMC1, MEWGENICS, MORROWIND, OBLIVION, WOL, SOD_PROFILE, SOD_STATS, TRAVELLERS_REST, YOUTUBERS_LIFE, LUCK_LANDLORD, LUNA_NIGHTS, EVERSPACE1, STREETS_OF_ROGUE, DELTATRAVELER, PIZZA_TOWER, TEARDOWN, WOLCEN, BLASPHEMOUS, NOITA, LAST_EPOCH, GUNGEON, DISCO, CLICKER_HEROES, ISAAC, ROR2, STS2, HEROES3, MOO2, YSF, BD2, SKY3, CS1, WO4, SW4II, NIER_REPLICANT, GENZERO,
    // SNES and N64 emulator saves (retro_specs.rs)
    SM64, SUPER_METROID, LTTP, FF6_SNES, CT_SNES,
    // PlayStation 1 and 2 (ps_specs.rs)
    SOTN, FF8, FFT, RE1, KH2, MK64, GOLDENEYE, BANJO_KAZOOIE, OOT, SMASH64, TORCHLIGHT3, WRECKFEST, EVIL_WITHIN, BORDERLANDS1,
    // PlayStation 1 and 2 (ps_specs.rs), continued
    CRASH, KH1, GT1, BL4, WUKONG, BOTW, RAYMAN, CTR, THPS, SOPHIE, FIRIS];
// ------------------------------------------------------------------ Cult of the Lamb

const COTL_ITEMS: &[(&str, &str)] = &[
    ("1", "Lumber"), ("2", "Stone"), ("6", "Meat"), ("8", "Berry Bush Seed"), ("9", "Bone"), ("20", "Coin"),
    ("21", "Berry"), ("22", "Heart of a Heretic"), ("28", "Salmon"), ("29", "Menticide Mushroom"), ("33", "Minnow"),
    ("34", "Tuna"), ("35", "Grass"), ("39", "Fertilizer"), ("43", "Small Gift"), ("44", "Big Gift"),
    ("45", "Flower Necklace"), ("46", "Feather Necklace"), ("47", "Skull Necklace"), ("48", "Nature's Necklace"),
    ("49", "Moon Necklace"), ("50", "Pumpkin"), ("51", "Pumpkin Seed"), ("55", "Camellia"), ("62", "Follower Meat"),
    ("70", "Menticide Mushroom Spore"), ("72", "Camellia Seed"), ("81", "Wooden Planks"), ("82", "Stone Blocks"),
    ("83", "Gold Nugget"), ("86", "Gold Bars"), ("89", "Crystal Shards"), ("90", "Spider Silk"), ("91", "Crab"),
    ("92", "Lobster"), ("93", "Octopus"), ("94", "Squid"), ("95", "Swordfish"), ("96", "Blowfish"), ("97", "Beetroot"),
    ("98", "Beetroot Seed"), ("101", "Eye of the Witness"), ("102", "Cauliflower"), ("103", "Cauliflower Seed"),
    ("105", "Morsel"), ("114", "Holy Talisman"), ("117", "Shell"),
];

const COTL: Spec = Spec {
    name: "Cult of the Lamb",
    tab: "Cult",
    accent: (178, 34, 52),
    kind: None,
    markers: &["/SaveUniqueID", "/items", "/Followers"],
    subtitle: "",
    tiles: &[("Level", "/Level"), ("Faith", "/CultFaith"), ("Doctrine stones", "/DoctrineStoneTotalCount"), ("Deaths", "/playerDeaths")],
    sections: &[
        Section {
            title: "The Lamb",
            rows: &[
                ("Level", "/Level"), ("XP", "/XP"), ("Hearts", "/PLAYER_HEALTH"), ("Total hearts", "/PLAYER_TOTAL_HEALTH"),
                ("Blue hearts", "/PLAYER_BLUE_HEARTS"), ("Black hearts", "/PLAYER_BLACK_HEARTS"), ("Spirit hearts", "/PLAYER_SPIRIT_HEARTS"),
                ("Fervour (special charge)", "/PLAYER_SPECIAL_CHARGE"), ("Damage level", "/PLAYER_DAMAGE_LEVEL"),
                ("Kills", "/KillsInGame"), ("Deaths", "/playerDeaths"), ("Play time (seconds)", "/TimeInGame"),
            ],
        },
        Section {
            title: "The cult",
            rows: &[
                ("Faith", "/CultFaith"), ("Static faith", "/StaticFaith"), ("Cult level", "/CurrentCultLevel"),
                ("Shrine level", "/ShrineLevel"), ("Doctrine stones", "/DoctrineStoneTotalCount"),
                ("Doctrine stones now", "/DoctrineCurrentCount"), ("Hunger", "/HungerBarCount"),
                ("Illness", "/IllnessBarCount"), ("Murders", "/STATS_Murders"), ("Sacrifices", "/STATS_Sacrifices"),
            ],
        },
        Section {
            title: "Doctrines",
            rows: &[
                ("Work & Worship level", "/Doctrine_WorkWorship_Level"), ("Possessions level", "/Doctrine_Possessions_Level"),
                ("Food level", "/Doctrine_Food_Level"), ("Afterlife level", "/Doctrine_Afterlife_Level"),
                ("Law & Order level", "/Doctrine_LawAndOrder_Level"),
            ],
        },
    ],
    lists: &[List {
        title: "Inventory and storage",
        path: "/items",
        shape: Shape::Array { id_key: "type", qty_keys: &["quantity", "UnreservedQuantity"] },
        names: COTL_ITEMS,
        max: Some(Max {
            ids: &["1", "2", "9", "35", "81", "82", "83", "86", "89", "90", "117"],
            cap: 999,
            caps: &[],
            estimate: true,
            about: "Raises lumber, stone, bones, grass, wooden planks, stone blocks, gold nuggets, gold bars, crystal shards, spider silk and shells you already have. Coins, food, seeds, gifts, necklaces, boss items and key items are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Fallout Shelter

const FALLOUT_SHELTER: Spec = Spec {
    name: "Fallout Shelter",
    tab: "Vault",
    accent: (74, 196, 224),
    kind: None,
    markers: &["/vault/storage/resources/Nuka", "/dwellers/dwellers"],
    subtitle: "",
    tiles: &[("Caps", "/vault/storage/resources/Nuka"), ("Nuka-Cola Quantum", "/vault/storage/resources/NukaColaQuantum"), ("Lunchboxes", "/vault/storage/resources/Lunchbox")],
    sections: &[Section {
        title: "Vault resources",
        rows: &[
            ("Caps", "/vault/storage/resources/Nuka"), ("Food", "/vault/storage/resources/Food"), ("Power", "/vault/storage/resources/Energy"),
            ("Water", "/vault/storage/resources/Water"), ("Stimpaks", "/vault/storage/resources/StimPack"), ("RadAway", "/vault/storage/resources/RadAway"),
            ("Lunchboxes", "/vault/storage/resources/Lunchbox"), ("Mr. Handy", "/vault/storage/resources/MrHandy"),
            ("Pet carriers", "/vault/storage/resources/PetCarrier"), ("Nuka-Cola Quantum", "/vault/storage/resources/NukaColaQuantum"),
            ("Crafted outfits", "/vault/storage/resources/CraftedOutfit"), ("Crafted weapons", "/vault/storage/resources/CraftedWeapon"),
            ("Crafted themes", "/vault/storage/resources/CraftedTheme"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: "Dwellers",
        path: "/dwellers/dwellers",
        name: &["/name", "/lastName"],
        cols: &[
            ("Level", "/experience/currentLevel"), ("Happiness", "/happiness/happinessValue"), ("Health", "/health/healthValue"),
            ("Max health", "/health/maxHealth"), ("Radiation", "/health/radiationValue"),
            ("S", "/stats/stats/1/value"), ("P", "/stats/stats/2/value"), ("E", "/stats/stats/3/value"), ("C", "/stats/stats/4/value"),
            ("I", "/stats/stats/5/value"), ("A", "/stats/stats/6/value"), ("L", "/stats/stats/7/value"),
        ],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Deep Rock Galactic

const DRG_RESOURCES: &[(&str, &str)] = &[
    ("af0dc4fe8361bb48b32c92cc97e21de7", "Bismor"), ("aaded8766c227d408032afd18d63561e", "Magnite"),
    ("8aa7fb43293a0b49b8be42ffe068a44c", "Croppa"), ("5f2bcf8347760a42a23b6edc07c0941d", "Umanite"),
    ("22bc4f7d07d13e43bfca81bd9c14b1af", "Jadiz"), ("488d05146f5f754ba3d4610d08c0603e", "Enor Pearl"),
    ("67668aae828fdb48a9111e1b912dbfa4", "Phazyonite"), ("72312204e287bc41815540a0cf881280", "Starch Nut"),
    ("078548b93232c04085f892e084a74100", "Yeast Cone"), ("41ea550c1d46c54bbe2e9ca5a7accb06", "Malt Star"),
    ("22daa757ad7a8049891b17edcc2fe098", "Barley Bulb"), ("99fa526ad87748459498905a278693f6", "Data Cells"),
    ("5828652c9a5de845a9e2e1b8b463c516", "Error Cubes"), ("a10cb2853871fb499ac854a1cde2202c", "Blank Cores"),
];

const DRG: Spec = Spec {
    name: "Deep Rock Galactic",
    tab: "Dwarves",
    accent: (232, 150, 24),
    kind: Some("Unreal Engine save (FSDSaveGame)"),
    markers: &["/CharacterSaves", "/Credits"],
    subtitle: "A dwarf reaches level 25 at 315,000 XP; each level before that takes a bit more than the last.",
    tiles: &[("Credits", "/Credits"), ("Perk points", "/PerkPoints")],
    sections: &[Section {
        title: "Account",
        rows: &[
            ("Credits", "/Credits"), ("Perk points", "/PerkPoints"), ("Vanity mastery XP", "/VanityMasterySave/XP"),
            ("Forge XP", "/Forging/XP"), ("Games played", "/NumberOfGamesPlayed"), ("Play time (seconds)", "/TotalPlayTimeSeconds"),
        ],
    }],
    lists: &[List {
        title: "Minerals and brewing ingredients",
        path: "/Resources/OwnedResources",
        shape: Shape::Map,
        names: DRG_RESOURCES,
        max: Some(Max {
            ids: &[
                "af0dc4fe8361bb48b32c92cc97e21de7", "aaded8766c227d408032afd18d63561e", "8aa7fb43293a0b49b8be42ffe068a44c",
                "5f2bcf8347760a42a23b6edc07c0941d", "22bc4f7d07d13e43bfca81bd9c14b1af", "488d05146f5f754ba3d4610d08c0603e",
                "67668aae828fdb48a9111e1b912dbfa4", "72312204e287bc41815540a0cf881280", "078548b93232c04085f892e084a74100",
                "41ea550c1d46c54bbe2e9ca5a7accb06", "22daa757ad7a8049891b17edcc2fe098",
            ],
            cap: 9999,
            caps: &[],
            estimate: true,
            about: "Raises Bismor, Magnite, Croppa, Umanite, Jadiz, Enor Pearl, Phazyonite and the four brewing ingredients you already have. Credits, Data Cells, Error Cubes and Blank Cores are left alone.",
        }),
    }],
    tables: &[Table {
        title: "Dwarves",
        path: "/CharacterSaves",
        name: &["/SavegameID"],
        only: &[],
        cols: &[("XP", "/XP"), ("Times retired", "/TimesRetired"), ("Retired levels", "/RetiredCharacterLevels")],
        names: &[
            ("9edd56f1eebcc5488d5b5e5b80b62db4", "Driller"), ("ae56e180fec0c44d96fa29c28366b97b", "Gunner"),
            ("30d8ea17d8fbba4c95306de9655c2f8c", "Scout"), ("85ef626c65f1024a8dfeb5d0f3909d2e", "Engineer"),
        ],
    }],
};

// ------------------------------------------------------------------ Schedule I (one screen per save file)

const S1_ACCENT: (u8, u8, u8) = (96, 170, 84);

const S1_MONEY: Spec = Spec {
    name: "Schedule I: money",
    tab: "Money",
    accent: S1_ACCENT,
    kind: None,
    markers: &["/OnlineBalance", "/Networth", "/LifetimeEarnings"],
    subtitle: "Money.json. Cash you carry is in the player's own files.",
    tiles: &[("Bank balance", "/OnlineBalance"), ("Net worth", "/Networth")],
    sections: &[Section {
        title: "Money",
        rows: &[("Bank balance", "/OnlineBalance"), ("Net worth", "/Networth"), ("Lifetime earnings", "/LifetimeEarnings"), ("Weekly deposits", "/WeeklyDepositSum")],
    }],
    lists: &[],
    tables: &[],
};

const S1_RANK: Spec = Spec {
    name: "Schedule I: rank",
    tab: "Rank",
    accent: S1_ACCENT,
    kind: None,
    markers: &["/Rank", "/Tier", "/TotalXP"],
    subtitle: "Rank.json",
    tiles: &[("Rank", "/Rank"), ("Tier", "/Tier"), ("Total XP", "/TotalXP")],
    sections: &[Section { title: "Rank", rows: &[("Rank", "/Rank"), ("Tier", "/Tier"), ("XP in this tier", "/XP"), ("Total XP", "/TotalXP")] }],
    lists: &[],
    tables: &[],
};

const S1_TIME: Spec = Spec {
    name: "Schedule I: time",
    tab: "Time",
    accent: S1_ACCENT,
    kind: None,
    markers: &["/ElapsedDays", "/TimeOfDay", "/Playtime"],
    subtitle: "Time.json",
    tiles: &[("Day", "/ElapsedDays"), ("Time of day", "/TimeOfDay")],
    sections: &[Section { title: "Time", rows: &[("Days elapsed", "/ElapsedDays"), ("Time of day (HHMM)", "/TimeOfDay"), ("Play time (seconds)", "/Playtime")] }],
    lists: &[],
    tables: &[],
};

const S1_GAME: Spec = Spec {
    name: "Schedule I: game",
    tab: "Game",
    accent: S1_ACCENT,
    kind: None,
    markers: &["/OrganisationName", "/Seed", "/Settings/ConsoleEnabled"],
    subtitle: "Game.json",
    tiles: &[("Organisation", "/OrganisationName")],
    sections: &[Section { title: "Game", rows: &[("Organisation name", "/OrganisationName"), ("Seed", "/Seed"), ("Console enabled", "/Settings/ConsoleEnabled")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Sons of the Forest

const SOTF_NAMES: &[(&str, &str)] = &[
    ("78", "Log"),
    ("340", "Guitar"),
    ("341", "Binoculars"),
    ("346", "Shotgun Rail"),
    ("353", "Stun Gun"),
    ("354", "Night Vision Goggles"),
    ("355", "Compact Pistol"),
    ("356", "Modern Axe"),
    ("358", "Shotgun"),
    ("359", "Machete"),
    ("360", "Compound/Tactical Bow"),
    ("361", "Rifle"),
    ("362", "Pistol Ammo"),
    ("363", "Shotgun Slug"),
    ("364", "Shotgun Buckshot"),
    ("365", "Crossbow"),
    ("367", "Katana"),
    ("368", "Crossbow Bolt"),
    ("369", "Stun Gun Ammo"),
    ("373", "Carbon Fiber Arrow"),
    ("374", "Pistol Suppressor/Silencer"),
    ("375", "Laser Sight"),
    ("376", "Pistol Rail"),
    ("378", "Flashlight Attachment"),
    ("379", "Tactical Axe"),
    ("380", "Utility Knife"),
    ("381", "Grenade"),
    ("382", "GrenadeAmmo"),
    ("383", "Rifle Rail Mod"),
    ("386", "Revolver"),
    ("387", "Rifle Ammo"),
    ("388", "Molotov"),
    ("390", "Printer Resin"),
    ("391", "Red Mask"),
    ("392", "Stick"),
    ("393", "Rock"),
    ("394", "Chainsaw"),
    ("396", "Stun Baton"),
    ("397", "Shiitake"),
    ("398", "King Oyster"),
    ("399", "Hydnum Repandum"),
    ("400", "Fly Amanita"),
    ("401", "Turtle Egg"),
    ("402", "Backpack"),
    ("403", "Rope"),
    ("405", "Bone"),
    ("410", "Watch"),
    ("412", "GPS Tracker"),
    ("413", "Plasma Lighter"),
    ("414", "Vodka Bottle"),
    ("415", "Cloth"),
    ("416", "Circuit Board"),
    ("417", "Time Bomb"),
    ("418", "Wire"),
    ("419", "Duct Tape"),
    ("420", "C4 Brick"),
    ("421", "Ramen Noodles"),
    ("422", "Repair Tool"),
    ("423", "Novel [Pennant Line]"),
    ("424", "Novel [Deep Sleep]"),
    ("425", "Crunchie Wunchies"),
    ("426", "Printed Flask"),
    ("428", "Printed Sled"),
    ("430", "Skull"),
    ("431", "Firefighter Axe"),
    ("432", "Can Opener"),
    ("433", "Meat"),
    ("434", "Canned Food"),
    ("435", "Golden Mask"),
    ("436", "Fish"),
    ("437", "Meds"),
    ("438", "MRE Pack"),
    ("439", "Energy Drink"),
    ("440", "Flare"),
    ("441", "Energy Bar"),
    ("443", "Crafted Bow"),
    ("444", "Rebreather"),
    ("445", "Blueberries"),
    ("446", "Twinberries"),
    ("447", "Salmonberries"),
    ("448", "Snowberries"),
    ("449", "Devil's Club"),
    ("450", "Horsetail"),
    ("451", "Aloe Vera"),
    ("452", "Yarrow"),
    ("453", "Fireweed"),
    ("454", "Arrowleaf"),
    ("455", "Health Mix"),
    ("456", "Health Mix+"),
    ("457", "Stun Gun Ammo"),
    ("459", "Slingshot"),
    ("461", "Energy Mix"),
    ("462", "Energy Mix+"),
    ("464", "Cat Food"),
    ("465", "Chicory"),
    ("466", "Oyster"),
    ("467", "Album Cover"),
    ("468", "Cross"),
    ("469", "Air Canister"),
    ("471", "Flashlight"),
    ("472", "Deer Hide"),
    ("473", "Leaf Armour"),
    ("474", "Crafted Spear"),
    ("476", "Small Rock"),
    ("477", "Crafted Club"),
    ("479", "Feather"),
    ("480", "Severed Arm"),
    ("481", "Severed Leg"),
    ("482", "Severed Head"),
    ("483", "Emergency Pack"),
    ("484", "Leaf"),
    ("485", "Shovel"),
    ("486", "Walkie Talkie"),
    ("487", "Silk Pajamas"),
    ("489", "Tactical Pants"),
    ("490", "Hoodie"),
    ("491", "Blazer"),
    ("492", "Tuxedo"),
    ("493", "Leather Jacket"),
    ("494", "Bone Armour"),
    ("495", "Tactical Jacket"),
    ("496", "Money"),
    ("499", "Wetsuit"),
    ("500", "Winter Jacket"),
    ("501", "Tactical Boots"),
    ("502", "Coins"),
    ("503", "Torch"),
    ("504", "Tarp"),
    ("506", "Turtle Shell"),
    ("507", "Stone Arrow"),
    ("508", "Skin Pouch"),
    ("509", "Holosprings Pamphlet"),
    ("517", "Cooking Pot"),
    ("518", "Paper Target"),
    ("519", "Deer Hide Armour"),
    ("520", "Structure Element"),
    ("521", "Email [More Lighting]"),
    ("522", "Rope Gun"),
    ("523", "Zipline Rope"),
    ("524", "Golf Ball"),
    ("525", "Putter"),
    ("526", "Mr Puffy Will"),
    ("527", "Batteries"),
    ("528", "MrPuffy Financials"),
    ("529", "GPS Locator"),
    ("530", "Drying Rack Blueprint"),
    ("531", "Stick Chair Blueprint"),
    ("532", "Stick Bed Blueprint"),
    ("533", "Weapon Rack Blueprint"),
    ("534", "Golf Email Pt 1"),
    ("535", "Golf Email Pt 2"),
    ("536", "Golf Email Pt 3"),
    ("537", "Golf Email Pt 4"),
    ("538", "Painting Email"),
    ("539", "Document [Out Demonia]"),
    ("540", "Document [Crosses burn Demons]"),
    ("541", "Document [Cube Activation Cycles]"),
    ("542", "Email [Thought you might find this interesting]"),
    ("543", "Email [Another one]"),
    ("544", "Email [One more]"),
    ("545", "Email [Photo Reference]"),
    ("546", "Email [Note to all employees]"),
    ("547", "Email [Safe in Cube]"),
    ("548", "Document [Stay in the Cube]"),
    ("549", "Bone Maker Blueprint"),
    ("550", "Fly Swatter Blueprint"),
    ("551", "Small Animal Catcher Blueprint"),
    ("552", "S.C.R.E.W."),
    ("553", "Tech Mesh"),
    ("554", "Tech Armour"),
    ("555", "Track Suit"),
    ("556", "Dress"),
    ("557", "Leather Suit"),
    ("558", "Camouflage Suit"),
    ("560", "Printed Grappling Hook"),
    ("562", "HolovilleNoticeA"),
    ("563", "HolovilleNoticeB"),
    ("564", "HolovilleNoticeC"),
    ("565", "HolovilleNoticeD"),
    ("566", "Maintenance Key Card"),
    ("567", "Guest Key Card"),
    ("568", "VIP Key Card"),
    ("569", "Brain Bite"),
    ("570", "Steak Bite"),
    ("571", "Steak and Bacon Bite"),
    ("572", "Ancient Armor"),
    ("574", "Novel [Rescued]"),
    ("575", "Novel [Parallel Universes]"),
    ("579", "Bone Chair Blueprint"),
    ("580", "Stick Ladder Blueprint"),
    ("581", "Bird House Blueprint"),
    ("582", "Fish Trap Blueprint"),
    ("584", "Mannequin Blueprint"),
    ("585", "Scarecrow Blueprint"),
    ("586", "Planter Wall Blueprint"),
    ("587", "Planter Floor Blueprint"),
    ("588", "Bone Chandelier Blueprint"),
    ("589", "S.C.R.E.W. / Tutorials"),
    ("590", "Radio"),
    ("591", "Small Log Cabin Blueprint"),
    ("593", "Creepy Armour"),
    ("594", "Guarana Berries"),
    ("595", "Blackberries"),
    ("596", "AloeVera Seed"),
    ("597", "Arrowleaf Seed"),
    ("598", "Blackberry Seed"),
    ("599", "Blueberry Seed"),
    ("600", "DevilsClub Seed"),
    ("601", "Fireweed Seed"),
    ("602", "Guarana Seed"),
    ("603", "Horsetail Seed"),
    ("604", "Salmonberry Seed"),
    ("605", "Chicory Seed"),
    ("606", "Yarro Seed"),
    ("607", "Rock Storage Blueprint"),
    ("608", "Stone Storage Blueprint"),
    ("609", "Log Storage Blueprint"),
    ("610", "Sticks Storage Blueprint"),
    ("611", "Numbers Printout"),
    ("612", "Newspaper Cutout [Missing CEO]"),
    ("613", "Newspaper Cutout [Puffcorp]"),
    ("614", "E-Mail [Artifact - Preliminary Findings]"),
    ("615", "E-Mail [Scientific Data - Cube]"),
    ("616", "Classified Document"),
    ("617", "Stock Price"),
    ("618", "Printed Arrow"),
    ("619", "Swim Suit"),
    ("620", "Table Blueprint"),
    ("621", "Shelf Blueprint"),
    ("622", "Rock Path Blueprint"),
    ("623", "Wall Shelf Blueprint"),
    ("624", "Wall Torch Blueprint"),
    ("625", "Hanging Skull Light Blueprint"),
    ("626", "Foldable Glider"),
    ("627", "Mr. Puffton's Reservation Card"),
    ("628", "Mrs. Puffton's Reservation Card"),
    ("629", "Rain Catcher Blueprint"),
    ("630", "KnightV"),
    ("631", "The Realm Beyond"),
    ("632", "Spring Trap Blueprint"),
    ("633", "Cult Pamphlet"),
    ("634", "Solar Panel"),
    ("635", "Light Bulb"),
    ("636", "TemplateD"),
    ("637", "Action Camera"),
    ("638", "Basic Log Sled Blueprint"),
    ("639", "Space Suit"),
    ("640", "Stone"),
    ("641", "Stone Fireplace Blueprint"),
    ("642", "Duck Head"),
    ("643", "Eagle Head"),
    ("644", "Small Turtle Head"),
    ("645", "Head Trophy Mount Blueprint"),
    ("646", "Rabbit Head"),
    ("647", "Squirrel Head"),
    ("648", "Seagull Head"),
    ("649", "Stones Storage Blueprint"),
    ("650", "Turtle Head"),
    ("651", "Deer Head"),
    ("652", "Moose Head"),
    ("653", "Personal Note A"),
    ("654", "Personal Note B"),
    ("655", "Personal Note C"),
    ("656", "Blueprint Hokey Pokey Trap"),
    ("657", "Printed GPS Case"),
    ("658", "Radio Trap Blueprint"),
    ("659", "Radio Blueprint"),
    ("661", "Golf Cart Battery"),
    ("662", "Artifact Piece A"),
    ("663", "Pickaxe"),
    ("664", "Solafite Ore"),
    ("665", "Item Plating Blueprint"),
    ("666", "Powered Cross Blueprint"),
    ("667", "Artifact Piece B"),
    ("668", "Artifact Piece C"),
    ("669", "Artifact Piece D"),
    ("670", "Gore Chair Blueprint"),
    ("671", "Gore Couch Blueprint"),
    ("672", "Uber Trap Blueprint"),
    ("673", "Round Table Blueprint"),
    ("674", "Old Note A"),
    ("675", "Old Note B"),
    ("676", "Old Note C"),
    ("677", "Fishing Email A"),
    ("678", "Fishing Email B"),
    ("679", "Grind Trap Blueprint"),
    ("680", "Spotlight Blueprint"),
    ("681", "Clock Blueprint"),
    ("682", "Spin Trap Blueprint"),
    ("683", "Double Bed Blueprint"),
    ("684", "Old Note E"),
    ("685", "Old Note D"),
    ("686", "Cult Sign F"),
    ("687", "Puffton Email A"),
    ("688", "Spear Thrower Trap Blueprint"),
    ("689", "Artifact Piece E"),
    ("690", "Story Pages"),
    ("691", "Old Note F"),
    ("692", "Fishing Email C"),
    ("693", "Bunker Map A"),
    ("694", "Jianyu Email Printout A"),
    ("695", "Cult Sign G"),
    ("696", "Cult Sign H"),
    ("697", "Cult Sign I"),
    ("698", "Cult Sign J"),
    ("699", "Cult Sign K"),
    ("700", "Spear Storage Blueprint"),
    ("701", "Leg Lamp Blueprint"),
    ("702", "Firewood Storage Blueprint"),
    ("703", "Priest Outfit"),
    ("704", "Wall Weapon Rack Blueprint"),
    ("705", "Circus Pamphlet"),
    ("706", "Newspaper Cutout [Husbands]"),
    ("707", "Artifact"),
    ("708", "Artifact Piece F"),
    ("709", "Teleporter Blueprint"),
    ("710", "Repel Shrine Blueprint"),
    ("711", "Attract Shrine Blueprint"),
    ("712", "Artifact Piece G"),
    ("713", "Plater Counter Blueprint"),
    ("714", "Gold Armor Plater Blueprint"),
    ("715", "Raccoon Head"),
    ("716", "Bible Paper D"),
    ("717", "Old Note G"),
    ("718", "Personal Note E"),
    ("719", "Personal Note F"),
    ("720", "Personal Note G"),
    ("721", "Bible Paper Scrap A"),
    ("722", "Puffton Email B"),
    ("723", "Puffton Email C"),
    ("724", "Twinberry Seed"),
    ("725", "Snowberry Seed"),
    ("726", "Hang Glider Launcher Blueprint"),
    ("727", "Solafite Armor"),
    ("729", "Skunk Head"),
    ("730", "Small Rabbit Hutch Blueprint"),
    ("749", "Stewardess Outfit"),
];

const SOTF_STATE: Spec = Spec {
    name: "Sons of the Forest: player",
    tab: "Player",
    accent: (90, 150, 70),
    kind: None,
    markers: &["/Data/PlayerState/_entries"],
    subtitle: "PlayerStateSaveData.json. Health, stamina, hydration and rest are 0 to their maximum; strength level is shown with its progress.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: "Stats",
        path: "/Data/PlayerState/_entries",
        name: &["/Name"],
        only: &[
            "MaxHealth", "CurrentHealth", "TargetHealth", "Stamina", "Hydration", "Rest", "Fullness", "Strength", "StrengthLevel",
            "CurrentStrength", "RequiredCurrentStrengthPerLevel", "Stealth", "StealthLevel", "Cold", "Sickness", "Armor",
        ],
        cols: &[("Decimal value", "/FloatValue"), ("Whole value", "/IntValue")],
        names: &[],
    }],
};

const SOTF_GAME: Spec = Spec {
    name: "Sons of the Forest: game clock",
    tab: "Game",
    accent: (90, 150, 70),
    kind: None,
    markers: &["/Data/GameState/GameDays"],
    subtitle: "GameStateSaveData.json. The in-game clock the save shows in its slot.",
    tiles: &[],
    sections: &[Section {
        title: "Game clock",
        rows: &[("Day", "/Data/GameState/GameDays"), ("Hour", "/Data/GameState/GameHours"), ("Minute", "/Data/GameState/GameMinutes"), ("Second", "/Data/GameState/GameSeconds")],
    }],
    lists: &[],
    tables: &[],
};

const SOTF_WEATHER: Spec = Spec {
    name: "Sons of the Forest: weather",
    tab: "Weather",
    accent: (90, 150, 70),
    kind: None,
    markers: &["/Data/WeatherSystem/_currentSeason"],
    subtitle: "WeatherSystemSaveData.json. Season is 0 spring, 1 summer, 2 autumn, 3 winter.",
    tiles: &[],
    sections: &[Section { title: "Season", rows: &[("Current season", "/Data/WeatherSystem/_currentSeason"), ("Starting day offset", "/Data/WeatherSystem/_startingDayOffset")] }],
    lists: &[],
    tables: &[],
};

const SOTF_ITEMS: Spec = Spec {
    name: "Sons of the Forest: inventory",
    tab: "Inventory",
    accent: (90, 150, 70),
    kind: None,
    markers: &["/Data/PlayerInventory/ItemInstanceManagerData/ItemBlocks"],
    subtitle: "PlayerInventorySaveData.json",
    tiles: &[],
    sections: &[],
    lists: &[List {
        title: "Items carried",
        path: "/Data/PlayerInventory/ItemInstanceManagerData/ItemBlocks",
        shape: Shape::Array { id_key: "ItemId", qty_keys: &["TotalCount"] },
        names: SOTF_NAMES,
        max: Some(Max {
            ids: &["78", "392", "393", "405", "415", "416", "418", "419", "403", "479", "484", "476", "553", "664", "390"],
            cap: 1,
            caps: &[("78", 2), ("392", 20), ("393", 10), ("405", 16), ("415", 1000), ("416", 6), ("418", 100), ("419", 100), ("403", 10), ("479", 1000), ("484", 1000), ("476", 30), ("553", 10), ("664", 20), ("390", 1000)],
            estimate: false,
            about: "Raises logs, sticks, rocks, bones, cloth, circuit boards, wire, duct tape, rope, feathers, leaves, small rocks, tech mesh, Solafite ore and printer resin you already carry to the game's own carrying limit for each. Weapons, ammo, food, blueprints and story items are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Death Must Die

const DMD: Spec = Spec {
    name: "Death Must Die",
    tab: "Profile",
    accent: (150, 40, 60),
    kind: None,
    markers: &["/serializedSaveData/values/0/Gold", "/serializedSaveData/values/0/ProgressionJson/_runCount"],
    subtitle: "",
    tiles: &[("Gold", "/serializedSaveData/values/0/Gold"), ("Runs", "/serializedSaveData/values/0/ProgressionJson/_runCount"), ("Wins", "/serializedSaveData/values/0/ProgressionJson/_successfulRuns")],
    sections: &[Section {
        title: "Progress",
        rows: &[
            ("Gold", "/serializedSaveData/values/0/Gold"), ("Runs", "/serializedSaveData/values/0/ProgressionJson/_runCount"),
            ("Successful runs", "/serializedSaveData/values/0/ProgressionJson/_successfulRuns"),
            ("Current streak", "/serializedSaveData/values/0/ProgressionJson/_currentStreak"),
            ("Best streak", "/serializedSaveData/values/0/ProgressionJson/_bestStreak"),
            ("Time in runs (seconds)", "/serializedSaveData/values/0/ProgressionJson/_timeSpentInRunSec"),
            ("Time in the game (seconds)", "/serializedSaveData/values/0/ProgressionJson/_timeSpentInAppSec"),
            ("Selected character", "/serializedSaveData/values/0/ProgressionJson/_selectedCharacterId"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Cuphead

const CUPHEAD: Spec = Spec {
    name: "Cuphead",
    tab: "Inkwell Isle",
    accent: (214, 168, 48),
    kind: None,
    markers: &["/inventories/playerOne/money", "/coinManager", "/statictics/playerOne"],
    subtitle: "",
    tiles: &[("Coins (Cuphead)", "/inventories/playerOne/money"), ("Coins (Mugman)", "/inventories/playerTwo/money"), ("Wishes left", "/djimmiWishes")],
    sections: &[
        Section {
            title: "Coins and deaths",
            rows: &[
                ("Coins, player 1", "/inventories/playerOne/money"), ("Coins, player 2", "/inventories/playerTwo/money"),
                ("Deaths, player 1", "/statictics/playerOne/numDeaths"), ("Deaths, player 2", "/statictics/playerTwo/numDeaths"),
                ("Djimmi's wishes", "/djimmiWishes"), ("Player 1 is Mugman", "/isPlayer1Mugman"),
            ],
        },
        Section {
            title: "Unlocks",
            rows: &[
                ("Expert mode available", "/_isHardModeAvailable"), ("Expert mode available (DLC)", "/_isHardModeAvailableDLC"),
                ("Black and white filter", "/unlockedBlackAndWhite"), ("2-strip colour filter", "/unlocked2Strip"),
                ("Chalice recolour", "/unlockedChaliceRecolor"), ("Boatman unlocked", "/hasUnlockedBoatman"),
                ("First super unlocked", "/hasUnlockedFirstSuper"), ("Curse charm puzzle done", "/curseCharmPuzzleComplete"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Antimatter Dimensions

const ANTIMATTER: Spec = Spec {
    name: "Antimatter Dimensions",
    tab: "Dimensions",
    accent: (40, 150, 200),
    kind: None,
    markers: &["/antimatter", "/dimensions/antimatter", "/infinityPoints", "/eternityPoints"],
    subtitle: "",
    tiles: &[("Antimatter", "/antimatter"), ("Infinity points", "/infinityPoints"), ("Eternity points", "/eternityPoints"), ("Reality machines", "/reality/realityMachines")],
    sections: &[
        Section {
            title: "Antimatter and Infinity",
            rows: &[
                ("Antimatter", "/antimatter"), ("Dimension boosts", "/dimensionBoosts"), ("Antimatter galaxies", "/galaxies"),
                ("Infinity points", "/infinityPoints"), ("Infinities", "/infinities"), ("Banked infinities", "/infinitiesBanked"),
                ("Infinity power", "/infinityPower"),
            ],
        },
        Section {
            title: "Eternity",
            rows: &[
                ("Eternity points", "/eternityPoints"), ("Eternities", "/eternities"), ("Time shards", "/timeShards"),
                ("Replicanti", "/replicanti/amount"), ("Replicanti galaxies", "/replicanti/galaxies"),
                ("Tachyon particles", "/dilation/tachyonParticles"), ("Dilated time", "/dilation/dilatedTime"),
            ],
        },
        Section {
            title: "Reality",
            rows: &[
                ("Realities", "/realities"), ("Reality machines", "/reality/realityMachines"),
                ("Imaginary machines", "/reality/imaginaryMachines"), ("Imaginary machine cap", "/reality/iMCap"),
                ("Perk points", "/reality/perkPoints"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Plants vs. Zombies

const PVZ: Spec = Spec {
    name: "Plants vs. Zombies",
    tab: "Profile",
    accent: (110, 180, 60),
    kind: Some("Plants vs. Zombies profile"),
    markers: &["/Adventure"],
    subtitle: "userN.dat. Money is stored divided by 10; the game's cap is 99,990.",
    tiles: &[("Money ÷ 10", "/Adventure/Money (the game shows this × 10)"), ("Adventure level", "/Adventure/Current level (1–50)")],
    sections: &[
        Section { title: "Adventure", rows: &[("Current level (1–50)", "/Adventure/Current level (1–50)"), ("Money (the game shows this × 10)", "/Adventure/Money (the game shows this × 10)"), ("Times completed", "/Adventure/Times completed"), ("Tree of Wisdom height (feet)", "/Adventure/Tree of Wisdom height (feet)"), ("Seed slot upgrades", "/Adventure/Seed slot upgrades"), ("Mini-games unlocked (1 = yes)", "/Adventure/Mini-games unlocked (1 = yes)"), ("Puzzle mode unlocked (1 = yes)", "/Adventure/Puzzle mode unlocked (1 = yes)")] },
        Section { title: "Crazy Dave's shop (1 = bought)", rows: &[("Gatling Pea", "/Shop/Gatling Pea/Owned (1 = yes)"), ("Twin Sunflower", "/Shop/Twin Sunflower/Owned (1 = yes)"), ("Gloom-shroom", "/Shop/Gloom-shroom/Owned (1 = yes)"), ("Cattail", "/Shop/Cattail/Owned (1 = yes)"), ("Winter Melon", "/Shop/Winter Melon/Owned (1 = yes)"), ("Gold Magnet", "/Shop/Gold Magnet/Owned (1 = yes)"), ("Spikerock", "/Shop/Spikerock/Owned (1 = yes)"), ("Cob Cannon", "/Shop/Cob Cannon/Owned (1 = yes)"), ("Imitater", "/Shop/Imitater/Owned (1 = yes)"), ("Golden Watering Can", "/Shop/Golden Watering Can/Owned (1 = yes)"), ("Phonograph", "/Shop/Phonograph/Owned (1 = yes)"), ("Gardening Glove", "/Shop/Gardening Glove/Owned (1 = yes)"), ("Mushroom Garden", "/Shop/Mushroom Garden/Owned (1 = yes)"), ("Wheel Barrow", "/Shop/Wheel Barrow/Owned (1 = yes)"), ("Pool Cleaner", "/Shop/Pool Cleaner/Owned (1 = yes)"), ("Roof Cleaner", "/Shop/Roof Cleaner/Owned (1 = yes)"), ("Aquarium Garden", "/Shop/Aquarium Garden/Owned (1 = yes)")] },
        Section { title: "Supplies (count + 1000)", rows: &[("Fertilizer", "/Shop/Fertilizer/Amount + 1000 (0 hides it)"), ("Bug spray", "/Shop/Bug spray/Amount + 1000 (0 hides it)"), ("Chocolate", "/Shop/Chocolate/Amount + 1000 (0 hides it)"), ("Tree food", "/Shop/Tree food/Amount + 1000 (0 hides it)"), ("Garden rake uses", "/Shop/Garden rake/Uses left")] },
        Section { title: "Survival: rounds completed", rows: &[("Day", "/Survival/Day: rounds completed"), ("Night", "/Survival/Night: rounds completed"), ("Pool", "/Survival/Pool: rounds completed"), ("Fog", "/Survival/Fog: rounds completed"), ("Roof", "/Survival/Roof: rounds completed"), ("Day (Hard)", "/Survival/Day (Hard): rounds completed"), ("Night (Hard)", "/Survival/Night (Hard): rounds completed"), ("Pool (Hard)", "/Survival/Pool (Hard): rounds completed"), ("Fog (Hard)", "/Survival/Fog (Hard): rounds completed"), ("Roof (Hard)", "/Survival/Roof (Hard): rounds completed")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Celeste

const CELESTE: Spec = Spec {
    name: "Celeste",
    tab: "Save",
    accent: (232, 90, 120),
    kind: Some("XML"),
    markers: &["/SaveData/UnlockedAreas", "/SaveData/Assists/GameSpeed"],
    subtitle: "Saves\\0.celeste … 2.celeste. Times are in ticks of 100 nanoseconds (10,000,000 per second).",
    tiles: &[("Strawberries", "/SaveData/TotalStrawberries"), ("Deaths", "/SaveData/TotalDeaths"), ("Chapters unlocked", "/SaveData/UnlockedAreas")],
    sections: &[
        Section { title: "File", rows: &[("Name", "/SaveData/Name"), ("Chapters unlocked (0–10)", "/SaveData/UnlockedAreas"), ("Total strawberries", "/SaveData/TotalStrawberries"), ("Golden strawberries", "/SaveData/TotalGoldenStrawberries"), ("Deaths", "/SaveData/TotalDeaths"), ("Jumps", "/SaveData/TotalJumps"), ("Wall jumps", "/SaveData/TotalWallJumps"), ("Dashes", "/SaveData/TotalDashes"), ("Play time (ticks)", "/SaveData/Time"), ("Cheat mode", "/SaveData/CheatMode"), ("Assist mode", "/SaveData/AssistMode"), ("Variant mode", "/SaveData/VariantMode")] },
        Section { title: "Assists", rows: &[("GameSpeed", "/SaveData/Assists/GameSpeed"), ("Invincible", "/SaveData/Assists/Invincible"), ("DashMode", "/SaveData/Assists/DashMode"), ("DashAssist", "/SaveData/Assists/DashAssist"), ("InfiniteStamina", "/SaveData/Assists/InfiniteStamina"), ("MirrorMode", "/SaveData/Assists/MirrorMode"), ("ThreeSixtyDashing", "/SaveData/Assists/ThreeSixtyDashing"), ("InvisibleMotion", "/SaveData/Assists/InvisibleMotion"), ("NoGrabbing", "/SaveData/Assists/NoGrabbing"), ("LowFriction", "/SaveData/Assists/LowFriction"), ("SuperDashing", "/SaveData/Assists/SuperDashing"), ("Hiccups", "/SaveData/Assists/Hiccups"), ("PlayAsBadeline", "/SaveData/Assists/PlayAsBadeline")] },
    ],
    lists: &[],
    tables: &[Table {
        title: "Chapters",
        path: "/SaveData/Areas/AreaStats",
        name: &["/@ID"],
        only: &[],
        cols: &[("Cassette", "/@Cassette"), ("A done", "/Modes/AreaModeStats/0/@Completed"), ("A full clear", "/Modes/AreaModeStats/0/@FullClear"), ("A heart", "/Modes/AreaModeStats/0/@HeartGem"), ("B done", "/Modes/AreaModeStats/1/@Completed"), ("B full clear", "/Modes/AreaModeStats/1/@FullClear"), ("B heart", "/Modes/AreaModeStats/1/@HeartGem"), ("C done", "/Modes/AreaModeStats/2/@Completed"), ("C full clear", "/Modes/AreaModeStats/2/@FullClear"), ("C heart", "/Modes/AreaModeStats/2/@HeartGem")],
        names: &[("0", "Prologue"), ("1", "Forsaken City"), ("2", "Old Site"), ("3", "Celestial Resort"), ("4", "Golden Ridge"), ("5", "Mirror Temple"), ("6", "Reflection"), ("7", "The Summit"), ("8", "Epilogue"), ("9", "Core"), ("10", "Farewell")],
    }],
};

// ------------------------------------------------------------------ Deep Rock Galactic: Survivor

const DRGS: Spec = Spec {
    name: "Deep Rock Galactic: Survivor",
    tab: "Profile",
    accent: (232, 150, 24),
    kind: None,
    markers: &["/Credits", "/ClassRanks", "/MetaStatUpgrades"],
    subtitle: "drgs.dat. Plain JSON; close the game before editing.",
    tiles: &[("Credits", "/Credits"), ("Player rank", "/PlayerRank")],
    sections: &[
        Section {
            title: "Resources",
            rows: &[("Credits", "/Credits"), ("Bismor", "/Bismor"), ("Croppa", "/Croppa"), ("Enor Pearl", "/EnorPearl"), ("Jadiz", "/Jadiz"), ("Magnite", "/Magnite"), ("Umanite", "/Umanite"), ("Power Core", "/PowerCore")],
        },
        Section {
            title: "Progress",
            rows: &[("Player rank", "/PlayerRank"), ("Total class levels", "/TotalClassLevels"), ("Dives", "/NumDivesTotal"), ("Biome goal points", "/BiomeGoalPoints")],
        },
    ],
    lists: &[],
    tables: &[
        Table { title: "Classes", path: "/ClassRanks", name: &["/ClassType"], only: &[], cols: &[("XP", "/Xp"), ("Rank", "/Rank")], names: &[] },
        Table { title: "Stat upgrades", path: "/MetaStatUpgrades", name: &["/Id"], only: &[], cols: &[("Level", "/Level")], names: &[] },
    ],
};

// ------------------------------------------------------------------ Kingdom Hearts III

const KH3: Spec = Spec {
    name: "Kingdom Hearts III",
    tab: "Sora",
    accent: (70, 120, 220),
    kind: Some(crate::fmt::kh3::KIND),
    markers: &["/Sora/Munny"],
    subtitle: "",
    tiles: &[("Munny", "/Sora/Munny"), ("Level", "/Sora/Level"), ("Total EXP", "/Sora/Total EXP")],
    sections: &[
        Section {
            title: "Sora",
            rows: &[
                ("Play time (seconds)", "/Sora/Play time (seconds)"), ("Enemies defeated", "/Sora/Enemies defeated"),
                ("Crabs collected", "/Sora/Crabs collected"), ("Bonus HP", "/Sora/Bonus HP"), ("Bonus MP", "/Sora/Bonus MP"),
                ("Bonus strength", "/Sora/Bonus strength"), ("Bonus magic", "/Sora/Bonus magic"), ("Bonus defense", "/Sora/Bonus defense"),
                ("Difficulty", "/Save/Difficulty"), ("Munny earned", "/Save/Munny earned"), ("Munny spent", "/Save/Munny spent"),
            ],
        },
        Section {
            title: "Sora, Donald and Goofy",
            rows: &[
                ("Sora HP", "/Characters/Sora/HP"), ("Sora MP", "/Characters/Sora/MP"), ("Sora focus", "/Characters/Sora/Focus"),
                ("Sora AP boosts", "/Characters/Sora/AP boosts"), ("Donald HP", "/Characters/Donald/HP"), ("Donald MP", "/Characters/Donald/MP"),
                ("Goofy HP", "/Characters/Goofy/HP"), ("Goofy MP", "/Characters/Goofy/MP"),
            ],
        },
    ],
    lists: &[
        List {
            title: "Synthesis materials",
            path: "/Synthesis materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Blazing Shard", "Blazing Stone", "Blazing Gem", "Blazing Crystal", "Frost Shard", "Frost Stone", "Frost Gem", "Frost Crystal", "Lightning Shard", "Lightning Stone", "Lightning Gem", "Lightning Crystal", "Lucid Shard", "Lucid Stone", "Lucid Gem", "Lucid Crystal", "Pulsing Shard", "Pulsing Stone", "Pulsing Gem", "Pulsing Crystal", "Writhing Shard", "Writhing Stone", "Writhing Gem", "Writhing Crystal", "Betwixt Shard", "Betwixt Stone", "Betwixt Gem", "Betwixt Crystal", "Twilight Shard", "Twilight Stone", "Twilight Gem", "Twilight Crystal", "Mythril Shard", "Mythril Stone", "Mythril Gem", "Mythril Crystal", "Sinister Shard", "Sinister Stone", "Sinister Gem", "Sinister Crystal", "Soothing Shard", "Soothing Stone", "Soothing Gem", "Soothing Crystal", "Wellspring Shard", "Wellspring Stone", "Wellspring Gem", "Wellspring Crystal", "Hungry Shard", "Hungry Stone", "Hungry Gem", "Hungry Crystal", "Fluorite", "Damascus", "Adamantite", "Orichalcum", "Orichalcum+", "Electrum", "Evanescent Crystal", "Illusory Crystal"],
                cap: crate::fmt::kh3::MATERIAL_CAP,
                caps: &[],
                estimate: true,
                about: "Sets every synthesis material to 99, the stack the game's menus show as full.",
            }),
        },
        List { title: "Items", path: "/Items", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Rune Factory 4 Special

const RF4S: Spec = Spec {
    name: "Rune Factory 4 Special",
    tab: "Farmer",
    accent: (110, 170, 70),
    kind: Some(crate::fmt::rf4s::KIND),
    markers: &["/Player/Money"],
    subtitle: "",
    tiles: &[("Money", "/Player/Money"), ("Order points", "/Player/Order points"), ("Tourists", "/Player/Tourists")],
    sections: &[Section {
        title: "Player",
        rows: &[
            ("HP", "/Player/HP"), ("Max HP", "/Player/Max HP"), ("RP", "/Player/RP"), ("Max RP", "/Player/Max RP"),
            ("Strength", "/Player/Strength"), ("Intelligence", "/Player/Intelligence"), ("Vitality", "/Player/Vitality"),
            ("Platform", "/Save/Platform"),
        ],
    }],
    lists: &[List {
        title: "Materials",
        path: "/Materials",
        shape: Shape::Map,
        names: &[],
        max: Some(Max {
            ids: &["Lumber", "Material stone"],
            cap: crate::fmt::rf4s::MATERIAL_CAP,
            caps: &[],
            estimate: false,
            about: "Sets lumber and material stone to 50,000, the most the game keeps. Fertilizer is left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Trails in the Sky 1st Chapter

const SKY1ST: Spec = Spec {
    name: "Trails in the Sky 1st Chapter",
    tab: "Bracers",
    accent: (200, 150, 60),
    kind: Some(crate::fmt::sky1st::KIND),
    markers: &["/Party/Mira"],
    subtitle: "",
    tiles: &[("Mira", "/Party/Mira")],
    sections: &[
        Section {
            title: "Characters 1 to 4",
            rows: &[
                ("1: experience", "/Characters/Character 1/Experience"), ("1: max HP", "/Characters/Character 1/Max HP"), ("1: max EP", "/Characters/Character 1/Max EP"),
                ("2: experience", "/Characters/Character 2/Experience"), ("2: max HP", "/Characters/Character 2/Max HP"), ("2: max EP", "/Characters/Character 2/Max EP"),
                ("3: experience", "/Characters/Character 3/Experience"), ("3: max HP", "/Characters/Character 3/Max HP"), ("3: max EP", "/Characters/Character 3/Max EP"),
                ("4: experience", "/Characters/Character 4/Experience"), ("4: max HP", "/Characters/Character 4/Max HP"), ("4: max EP", "/Characters/Character 4/Max EP"),
            ],
        },
    ],
    lists: &[List {
        title: "Sepith",
        path: "/Sepith",
        shape: Shape::Map,
        names: &[],
        max: Some(Max {
            ids: &["Earth", "Water", "Fire", "Wind", "Time", "Space", "Mirage", "Mass"],
            cap: crate::fmt::sky1st::SEPITH_CAP,
            caps: &[],
            estimate: false,
            about: "Sets all eight sepith to 9,999, the most the game holds.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Kerbal Space Program

const KSP: Spec = Spec {
    name: "Kerbal Space Program",
    tab: "Space program",
    accent: (240, 140, 30),
    kind: Some(crate::fmt::ksp::KIND),
    markers: &["/GAME/SCENARIO: Funding/funds"],
    subtitle: "Career funds, science and reputation (sandbox saves keep these at their defaults)",
    tiles: &[("Funds", "/GAME/SCENARIO: Funding/funds"), ("Science", "/GAME/SCENARIO: ResearchAndDevelopment/sci"), ("Reputation", "/GAME/SCENARIO: Reputation/rep")],
    sections: &[],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Final Fantasy Pixel Remaster

const FFPR: Spec = Spec {
    name: "Final Fantasy Pixel Remaster (I-VI)",
    tab: "Party",
    accent: (60, 110, 200),
    kind: None,
    markers: &["/userData/owendGil", "/userData/ownedCharacterList"],
    subtitle: "One save slot of FINAL FANTASY I to VI Pixel Remaster (a character's number is its id in the game)",
    tiles: &[("Gil", "/userData/owendGil"), ("Total gil", "/userData/totalGil"), ("Steps", "/userData/steps")],
    sections: &[Section {
        title: "Progress",
        rows: &[("Battles", "/userData/battleCount"), ("Escapes", "/userData/escapeCount"), ("Chests opened", "/userData/openChestCount"), ("Monsters defeated", "/userData/monstersKilledCount"), ("Times saved", "/userData/saveCompleteCount")],
    }],
    lists: &[
        List { title: "Items", path: "/userData/normalOwnedItemList/target", shape: Shape::Array { id_key: "contentId", qty_keys: &["count"] }, names: &[], max: None },
        List { title: "Key items", path: "/userData/importantOwendItemList/target", shape: Shape::Array { id_key: "contentId", qty_keys: &["count"] }, names: &[], max: None },
    ],
    tables: &[Table {
        title: "Characters",
        path: "/userData/ownedCharacterList/target",
        name: &["/id"],
        cols: &[
            ("Experience", "/currentExp"), ("HP", "/parameter/currentHP"), ("MP", "/parameter/currentMP"), ("Bonus max HP", "/parameter/addtionalMaxHp"), ("Bonus max MP", "/parameter/addtionalMaxMp"),
            ("Bonus power", "/parameter/addtionalPower"), ("Bonus vitality", "/parameter/addtionalVitality"), ("Bonus agility", "/parameter/addtionalAgility"), ("Bonus intelligence", "/parameter/addtionalIntelligence"), ("Bonus spirit", "/parameter/addtionalSpirit"), ("Bonus magic", "/parameter/addtionalMagic"), ("Bonus luck", "/parameter/addtionalLuck"),
        ],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Minecraft Dungeons

const MCD: Spec = Spec {
    name: "Minecraft Dungeons",
    tab: "Hero",
    accent: (90, 170, 70),
    kind: None,
    markers: &["/currency", "/playerId", "/xp"],
    subtitle: "A hero's save (Players\\Active\\*.dat), decrypted; items and enchantments are in the Tree",
    tiles: &[("Experience", "/xp"), ("Total gear power", "/totalGearPower")],
    sections: &[],
    lists: &[List { title: "Currency", path: "/currency", shape: Shape::Array { id_key: "type", qty_keys: &["count"] }, names: &[("Emerald", "Emeralds"), ("Gold", "Gold"), ("EyeOfEnder", "Eyes of Ender")], max: None }],
    tables: &[],
};

// ------------------------------------------------------------------ Stoneshard

const STONESHARD: Spec = Spec {
    name: "Stoneshard",
    tab: "Character",
    accent: (170, 60, 50),
    kind: None,
    markers: &["/characterDataMap/LVL", "/characterDataMap/XP"],
    subtitle: "One character's data.sav; unspent points let you respec in the game",
    tiles: &[("Level", "/characterDataMap/LVL"), ("Experience", "/characterDataMap/XP"), ("Ability points", "/characterDataMap/SP")],
    sections: &[Section {
        title: "Attributes",
        rows: &[
            ("Unspent attribute points", "/characterDataMap/AP"), ("Strength", "/characterDataMap/STR"), ("Agility", "/characterDataMap/AGL"),
            ("Perception", "/characterDataMap/PRC"), ("Vitality", "/characterDataMap/Vitality"), ("Willpower", "/characterDataMap/WIL"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Warhammer 40,000: Rogue Trader
// Paths from Arcemi's save editor (MPL-2.0, facts only).

const ROGUE_TRADER: Spec = Spec {
    name: "Warhammer 40,000: Rogue Trader",
    tab: "Dynasty",
    accent: (196, 152, 64),
    kind: None,
    markers: &["/player/Scrap/m_Value", "/player/ProfitFactor", "/party"],
    subtitle: "",
    tiles: &[("Scrap", "/player/Scrap/m_Value"), ("Profit Factor", "/player/ProfitFactor/InitialValue"), ("Navigator's insight", "/player/WarpTravelState/NavigatorResource")],
    sections: &[Section {
        title: "Difficulty",
        rows: &[
            ("Preset", "/player/MinDifficultyController/MinDifficulty/GameDifficulty"),
            ("Only one save", "/player/MinDifficultyController/MinDifficulty/OnlyOneSave"),
            ("Respec allowed", "/player/MinDifficultyController/MinDifficulty/RespecAllowed"),
            ("Skill test modifier (%)", "/player/MinDifficultyController/MinDifficulty/SkillCheckModifier"),
            ("Enemy wounds modifier (%)", "/player/MinDifficultyController/MinDifficulty/EnemyHitPointsPercentModifier"),
        ],
    }],
    lists: &[],
    tables: &[Table { title: "Reputation", path: "/player/FractionsReputation", name: &["/Key"], cols: &[("Reputation", "/Value")], only: &[], names: &[] }],
};

// ------------------------------------------------------------------ BioShock Infinite

const BIOSHOCK_INFINITE: Spec = Spec {
    name: "BioShock Infinite",
    tab: "Booker",
    accent: (90, 150, 200),
    kind: Some(crate::fmt::bsinfinite::KIND),
    markers: &[],
    subtitle: "Infusion levels go from 0 to 10; a kind you've never picked has no entry yet.",
    tiles: &[("Silver Eagles", "/Booker/Silver Eagles"), ("Lockpicks", "/Booker/Lockpicks"), ("Salts", "/Booker/Salts")],
    sections: &[Section {
        title: "Infusions",
        rows: &[("Health", "/Infusions/Health"), ("Shield", "/Infusions/Shield"), ("Salts", "/Infusions/Salts")],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Space Haven

const SH_CREW_COLS: &[(&str, &str)] = &[
    ("Health", "/props/Health/@v"), ("Food", "/props/Food/@v"), ("Rest", "/props/Rest/@v"), ("Comfort", "/props/Comfort/@v"),
    ("Mood", "/props/Mood/@v"),
    ("Bravery", "/pers/attr/a/0/@points"), ("Zest", "/pers/attr/a/1/@points"), ("Intelligence", "/pers/attr/a/2/@points"),
    ("Perception", "/pers/attr/a/3/@points"),
    ("Mining", "/pers/skills/s/0/@level"), ("Botany", "/pers/skills/s/1/@level"), ("Construct", "/pers/skills/s/2/@level"),
    ("Industry", "/pers/skills/s/3/@level"), ("Medical", "/pers/skills/s/4/@level"), ("Navigation", "/pers/skills/s/5/@level"),
    ("Gunner", "/pers/skills/s/6/@level"), ("Shielding", "/pers/skills/s/7/@level"), ("Operations", "/pers/skills/s/8/@level"),
    ("Research", "/pers/skills/s/10/@level"), ("Weapons", "/pers/skills/s/11/@level"), ("Piloting", "/pers/skills/s/12/@level"),
];

const SPACE_HAVEN: Spec = Spec {
    name: "Space Haven",
    tab: "Fleet",
    accent: (64, 160, 200),
    kind: Some("XML"),
    markers: &["/game/playerBank/@ca", "/game/ships", "/game/clock"],
    subtitle: "Skills run 0 to 8 and attributes 0 to 10. A skill can only be trained up to its own limit, so raising a level here past what a crew member could reach is kept as typed",
    tiles: &[("Credits", "/game/playerBank/@ca"), ("Day", "/game/clock/@days"), ("Hour", "/game/clock/@hours")],
    sections: &[
        Section {
            title: "Credits and clock",
            rows: &[
                ("Credits available", "/game/playerBank/@ca"), ("Credits reserved", "/game/playerBank/@cr"),
                ("Day", "/game/clock/@days"), ("Hour", "/game/clock/@hours"), ("Minute", "/game/clock/@minutes"),
            ],
        },
        Section {
            title: "Difficulty",
            rows: &[
                ("Mood", "/game/settings/diff/@moodDifficulty"), ("Enemy targeting", "/game/settings/diff/@npcTargeting"),
                ("Quests", "/game/settings/diff/@questDifficulty"), ("Sandbox", "/game/settings/diff/@sandbox"),
                ("Enemies", "/game/settings/diff/@enemiesEnabled"), ("Friends", "/game/settings/diff/@friendsEnabled"),
                ("Lovers", "/game/settings/diff/@loversEnabled"),
            ],
        },
    ],
    lists: &[],
    tables: &[Table {
        title: "Crew of the first ship",
        path: "/game/ships/ship/0/characters/c",
        name: &["/@name", "/@lname"],
        only: &[],
        cols: SH_CREW_COLS,
        names: &[],
    }, Table {
        title: "Crew of the second ship",
        path: "/game/ships/ship/1/characters/c",
        name: &["/@name", "/@lname"],
        only: &[],
        cols: SH_CREW_COLS,
        names: &[],
    }, Table {
        title: "Crew of the third ship",
        path: "/game/ships/ship/2/characters/c",
        name: &["/@name", "/@lname"],
        only: &[],
        cols: SH_CREW_COLS,
        names: &[],
    }],
};

// ------------------------------------------------------------------ SnowRunner

const SNOWRUNNER: Spec = Spec {
    name: "SnowRunner",
    tab: "Garage",
    accent: (214, 138, 30),
    kind: None,
    markers: &["/CompleteSave/SslValue/persistentProfileData/money", "/CompleteSave/SslValue/persistentProfileData/rank", "/CompleteSave/SslValue/gameTime"],
    subtitle: "The rank follows the experience the game expects for it: after changing one, check the other in the game",
    tiles: &[("Money", "/CompleteSave/SslValue/persistentProfileData/money"), ("Rank", "/CompleteSave/SslValue/persistentProfileData/rank"), ("Experience", "/CompleteSave/SslValue/persistentProfileData/experience")],
    sections: &[
        Section {
            title: "Profile",
            rows: &[
                ("Money", "/CompleteSave/SslValue/persistentProfileData/money"), ("Rank", "/CompleteSave/SslValue/persistentProfileData/rank"),
                ("Experience", "/CompleteSave/SslValue/persistentProfileData/experience"), ("Hard mode", "/CompleteSave/SslValue/isHardMode"),
                ("Last level", "/CompleteSave/SslValue/lastLoadedLevel"),
            ],
        },
        Section {
            title: "Statistics",
            rows: &[("Money earned", "/CompleteSave/SslValue/gameStat/MONEY_EARNED"), ("Money spent", "/CompleteSave/SslValue/gameStat/MONEY_SPENT"), ("Trucks bought", "/CompleteSave/SslValue/gameStat/TRUCK_BOUGHT"), ("Trucks sold", "/CompleteSave/SslValue/gameStat/TRUCK_SOLD"), ("Trailers bought", "/CompleteSave/SslValue/gameStat/TRAILER_BOUGHT"), ("Trailers sold", "/CompleteSave/SslValue/gameStat/TRAILER_SOLD"), ("Add-ons bought", "/CompleteSave/SslValue/gameStat/ADDON_BOUGHT"), ("Add-ons sold", "/CompleteSave/SslValue/gameStat/ADDON_SOLD"), ("Sessions played", "/CompleteSave/SslValue/gameStat/SESSION_NUMBER")],
        },
        Section {
            title: "Difficulty",
            rows: &[
                ("Truck prices", "/CompleteSave/SslValue/gameDifficultySettings/truckPricingFactor"), ("Trailer prices", "/CompleteSave/SslValue/gameDifficultySettings/trailerPricingFactor"),
                ("Add-on prices", "/CompleteSave/SslValue/gameDifficultySettings/addonPricingFactor"), ("Selling price", "/CompleteSave/SslValue/gameDifficultySettings/sellMultiplier"),
                ("Fuel price", "/CompleteSave/SslValue/gameDifficultySettings/fuelPriceFactor"), ("Contract payouts", "/CompleteSave/SslValue/gameDifficultySettings/contractsPayoutsFactor"),
                ("Task and contest payouts", "/CompleteSave/SslValue/gameDifficultySettings/tasksAndContestsPayoutsFactor"),
                ("Vehicle damage", "/CompleteSave/SslValue/gameDifficultySettings/vehicleDamageFactor"), ("Can skip time", "/CompleteSave/SslValue/gameDifficultySettings/isAbleToSkipTime"),
                ("Garage refuel", "/CompleteSave/SslValue/gameDifficultySettings/isGarageRefuelAvailable"),
            ],
        },
    ],
    lists: &[],
    tables: &[Table {
        title: "Trucks in the warehouse",
        path: "/CompleteSave/SslValue/persistentProfileData/trucksInWarehouse",
        name: &["/type"],
        only: &[],
        cols: &[("Repairs", "/repairs"), ("Gearbox damage", "/gearboxDamage"), ("Suspension damage", "/suspensionDamage")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Graveyard Keeper

const GRAVEYARD_KEEPER: Spec = Spec {
    name: "Graveyard Keeper",
    tab: "Keeper",
    accent: (120, 150, 70),
    kind: Some(crate::fmt::gk::KIND),
    markers: &["/_inventory/_params/_money"],
    subtitle: "Money is counted in silver (100 = 1 gold, with copper as the decimals); technology points (red, green and blue) are in the Tree under _res_v",
    tiles: &[("Money (silver)", "/_inventory/_params/_money"), ("Health", "/_inventory/_params/_hp")],
    sections: &[],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Need for Speed: Most Wanted

const NFSMW: Spec = Spec {
    name: "Need for Speed: Most Wanted (2005)",
    tab: "Driver",
    accent: (210, 160, 30),
    kind: Some(crate::fmt::nfsmw::KIND),
    markers: &["/Profile/Money"],
    subtitle: "A 2005 profile (PC or PlayStation 2); the file's MD5 is made again on save",
    tiles: &[("Money", "/Profile/Money"), ("Pursuit bounty", "/Profile/Pursuit bounty")],
    sections: &[],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Tales of Symphonia

const TOS: Spec = Spec {
    name: "Tales of Symphonia",
    tab: "Party",
    accent: (230, 170, 60),
    kind: Some(crate::fmt::tos::KIND),
    markers: &["/Party/Gald", "/Characters/Lloyd/Level"],
    subtitle: "TOSSaveData0.dat. Gald is kept in two places and both are written. Item counts go up to 20.",
    tiles: &[("Gald", "/Party/Gald"), ("Grade (x100)", "/Party/Grade points (stored x100)"), ("Battles", "/Party/Battles")],
    sections: &[Section {
        title: "Party and records",
        rows: &[
            ("Gald", "/Party/Gald"), ("Most gald held", "/Party/Most gald held"), ("Total gald spent", "/Party/Total gald spent"),
            ("Grade points (x100)", "/Party/Grade points (stored x100)"), ("Battles", "/Party/Battles"), ("Escapes", "/Party/Escapes"),
            ("Times saved", "/Party/Times saved"), ("Times cleared", "/Party/Times the game was cleared"),
            ("Play time (frames, 60 a second)", "/Party/Play time (frames, 60 a second)"),
        ],
    }],
    lists: &[List {
        title: "Items",
        path: "/Items",
        shape: Shape::Map,
        names: &[],
        max: None,
    }],
    tables: &[Table {
        title: "Characters",
        path: "/Characters",
        name: &[],
        only: &[],
        cols: &[
            ("Level", "/Level"), ("Experience", "/Experience"), ("HP", "/HP"), ("Max HP", "/Max HP"), ("TP", "/TP"), ("Max TP", "/Max TP"),
            ("Strength", "/Strength"), ("Attack", "/Attack"), ("Attack 2", "/Attack 2"), ("Defense", "/Defense"), ("Luck", "/Luck"),
            ("Accuracy", "/Accuracy"), ("Evasion", "/Evasion"), ("Intelligence", "/Intelligence"), ("Overlimit %", "/Overlimit (%)"), ("Affection", "/Affection"),
        ],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Grand Theft Auto III

const GTA3: Spec = Spec {
    name: "Grand Theft Auto III",
    tab: "Player",
    accent: (210, 120, 60),
    kind: Some(crate::fmt::gta3::KIND),
    markers: &["/Player/Money", "/Statistics/Progress made"],
    subtitle: "GTA3sf*.b. Money is kept in two places; the save's checksum is rewritten.",
    tiles: &[("Money", "/Player/Money"), ("Progress", "/Statistics/Progress made"), ("Packages", "/Player/Hidden packages collected")],
    sections: &[
        Section {
            title: "Player",
            rows: &[
                ("Money", "/Player/Money"), ("Money shown on screen", "/Player/Money shown on screen"),
                ("Hidden packages collected", "/Player/Hidden packages collected"), ("Hidden packages in the game", "/Player/Hidden packages in the game"),
            ],
        },
        Section {
            title: "Statistics",
            rows: &[
                ("Progress made", "/Statistics/Progress made"), ("Total progress in the game", "/Statistics/Total progress in the game"),
                ("Missions passed", "/Statistics/Missions passed"), ("Missions given", "/Statistics/Missions given"),
                ("Unique stunt jumps found", "/Statistics/Unique stunt jumps found"), ("Total unique stunt jumps", "/Statistics/Total unique stunt jumps"),
                ("Days passed", "/Statistics/Days passed"), ("Times arrested", "/Statistics/Times arrested"), ("Times died", "/Statistics/Times died"),
                ("People killed by the player", "/Statistics/People killed by the player"), ("Cars exploded", "/Statistics/Cars exploded"),
                ("Rounds fired", "/Statistics/Rounds fired"), ("Helicopters destroyed", "/Statistics/Helicopters destroyed"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Grand Theft Auto: Vice City

const GTAVC: Spec = Spec {
    name: "Grand Theft Auto: Vice City",
    tab: "Player",
    accent: (240, 110, 170),
    kind: Some(crate::fmt::gta3::KIND_VC),
    markers: &["/Player/Money", "/Statistics/Progress made"],
    subtitle: "GTAVCsf*.b. Money is kept in two places; the save's checksum is rewritten.",
    tiles: &[("Money", "/Player/Money"), ("Progress", "/Statistics/Progress made"), ("Packages", "/Player/Hidden packages collected")],
    sections: &[
        Section {
            title: "Player",
            rows: &[
                ("Money", "/Player/Money"), ("Money shown on screen", "/Player/Money shown on screen"),
                ("Hidden packages collected", "/Player/Hidden packages collected"), ("Hidden packages in the game", "/Player/Hidden packages in the game"),
            ],
        },
        Section {
            title: "Statistics",
            rows: &[
                ("Progress made", "/Statistics/Progress made"), ("Total progress in the game", "/Statistics/Total progress in the game"),
                ("Missions given", "/Statistics/Missions given"),
                ("Unique stunt jumps found", "/Statistics/Unique stunt jumps found"), ("Total unique stunt jumps", "/Statistics/Total unique stunt jumps"),
                ("Days passed", "/Statistics/Days passed"), ("Times arrested", "/Statistics/Times arrested"), ("Times died", "/Statistics/Times died"),
                ("People killed by the player", "/Statistics/People killed by the player"), ("Cars exploded", "/Statistics/Cars exploded"),
                ("Boats exploded", "/Statistics/Boats exploded"), ("Rounds fired", "/Statistics/Rounds fired"),
                ("Helicopters destroyed", "/Statistics/Helicopters destroyed"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Bitburner

const BITBURNER: Spec = Spec {
    name: "Bitburner",
    tab: "Player",
    accent: (60, 200, 90),
    kind: None,
    markers: &["/data/PlayerSave/data/money", "/data/PlayerSave/data/skills/hacking"],
    subtitle: "An exported save (bitburnerSave_*.json). Skill levels are worked out from experience when the game loads, so raise the experience too if a skill doesn't stick.",
    tiles: &[("Money", "/data/PlayerSave/data/money"), ("Hacking", "/data/PlayerSave/data/skills/hacking"), ("BitNode", "/data/PlayerSave/data/bitNodeN")],
    sections: &[
        Section {
            title: "Player",
            rows: &[
                ("Money", "/data/PlayerSave/data/money"), ("Current BitNode", "/data/PlayerSave/data/bitNodeN"),
                ("City", "/data/PlayerSave/data/city"), ("Karma", "/data/PlayerSave/data/karma"),
                ("People killed", "/data/PlayerSave/data/numPeopleKilled"), ("Entropy", "/data/PlayerSave/data/entropy"),
                ("HP", "/data/PlayerSave/data/hp/current"), ("Max HP", "/data/PlayerSave/data/hp/max"),
            ],
        },
        Section {
            title: "Skill levels",
            rows: &[
                ("Hacking", "/data/PlayerSave/data/skills/hacking"), ("Strength", "/data/PlayerSave/data/skills/strength"),
                ("Defense", "/data/PlayerSave/data/skills/defense"), ("Dexterity", "/data/PlayerSave/data/skills/dexterity"),
                ("Agility", "/data/PlayerSave/data/skills/agility"), ("Charisma", "/data/PlayerSave/data/skills/charisma"),
                ("Intelligence", "/data/PlayerSave/data/skills/intelligence"),
            ],
        },
        Section {
            title: "Experience",
            rows: &[
                ("Hacking", "/data/PlayerSave/data/exp/hacking"), ("Strength", "/data/PlayerSave/data/exp/strength"),
                ("Defense", "/data/PlayerSave/data/exp/defense"), ("Dexterity", "/data/PlayerSave/data/exp/dexterity"),
                ("Agility", "/data/PlayerSave/data/exp/agility"), ("Charisma", "/data/PlayerSave/data/exp/charisma"),
                ("Intelligence", "/data/PlayerSave/data/exp/intelligence"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Terra Invicta

const TERRA_INVICTA: Spec = Spec {
    name: "Terra Invicta",
    tab: "Factions",
    accent: (80, 150, 230),
    kind: None,
    markers: &["/gamestates/PavonisInteractive.TerraInvicta.TIFactionState"],
    subtitle: "A save (.gz or the JSON inside it). Every faction's stockpile is listed, yours and the AI's. Saves from an ironman campaign aren't covered.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: "Faction resources",
        path: "/gamestates/PavonisInteractive.TerraInvicta.TIFactionState",
        name: &["/Value/displayName"],
        only: &[],
        cols: &[("Money", "/Value/resources/Money"), ("Influence", "/Value/resources/Influence"), ("Operations", "/Value/resources/Operations"), ("Research", "/Value/resources/Research"), ("Projects", "/Value/resources/Projects"), ("Boost", "/Value/resources/Boost"), ("MissionControl", "/Value/resources/MissionControl"), ("Water", "/Value/resources/Water"), ("Volatiles", "/Value/resources/Volatiles"), ("Metals", "/Value/resources/Metals"), ("NobleMetals", "/Value/resources/NobleMetals"), ("Fissiles", "/Value/resources/Fissiles"), ("Antimatter", "/Value/resources/Antimatter"), ("Exotics", "/Value/resources/Exotics")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Prison Architect

const PRISON: Spec = Spec {
    name: "Prison Architect",
    tab: "Prison",
    accent: (230, 140, 40),
    kind: Some(crate::fmt::prison::KIND),
    markers: &["/Finance/Balance"],
    subtitle: "A .prison save. Only the bank balance and the top-level settings are shown; the rest of the file is left byte for byte as it was.",
    tiles: &[("Balance", "/Finance/Balance")],
    sections: &[Section {
        title: "Settings",
        rows: &[
            ("Seconds played", "/Settings/SecondsPlayed"), ("Time index", "/Settings/TimeIndex"),
            ("Time warp factor", "/Settings/TimeWarpFactor"), ("Food quantity", "/Settings/FoodQuantity"),
            ("Food variation", "/Settings/FoodVariation"), ("Staff have guard keys", "/Settings/StaffHaveGuardKeys"),
            ("Electricity", "/Settings/EnabledElectricity"), ("Water", "/Settings/EnabledWater"),
            ("Food", "/Settings/EnabledFood"), ("Misconduct", "/Settings/EnabledMisconduct"),
            ("Gangs", "/Settings/EnabledGangs"), ("Decay", "/Settings/EnabledDecay"),
            ("Failure conditions", "/Settings/FailureConditions"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Final Fantasy XIII

const FF13: Spec = Spec {
    name: "Final Fantasy XIII",
    tab: "Party",
    accent: (110, 170, 230),
    kind: Some(crate::fmt::ff13::KIND),
    markers: &["/Party/Gil"],
    subtitle: "ff13-NN.dat. The save is decrypted to edit and encrypted again on save. Gil goes up to 9,999,999.",
    tiles: &[("Gil", "/Party/Gil")],
    sections: &[Section { title: "Party", rows: &[("Gil", "/Party/Gil")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Sonic Mania

const SONIC_MANIA: Spec = Spec {
    name: "Sonic Mania",
    tab: "Slots",
    accent: (50, 120, 230),
    kind: Some(crate::fmt::sonic_mania::KIND),
    markers: &["/Mania slot 1/Lives", "/Encore slot 1/Lives"],
    subtitle: "SaveData.bin: eight Mania Mode and three Encore Mode slots. Emeralds are a bit field (127 = all seven).",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[
        Table {
            title: "Mania Mode slots",
            path: "",
            name: &[],
            only: &["Mania slot 1", "Mania slot 2", "Mania slot 3", "Mania slot 4", "Mania slot 5", "Mania slot 6", "Mania slot 7", "Mania slot 8"],
            cols: &[
                ("State", "/State"), ("Character", "/Character"), ("Zone", "/Zone"), ("Lives", "/Lives"), ("Score", "/Score"),
                ("Emeralds", "/Chaos Emeralds (127 = all seven)"), ("Continues", "/Continues"), ("Next special stage", "/Next special stage"),
            ],
            names: &[],
        },
        Table {
            title: "Encore Mode slots",
            path: "",
            name: &[],
            only: &["Encore slot 1", "Encore slot 2", "Encore slot 3"],
            cols: &[
                ("State", "/State"), ("Zone", "/Zone"), ("Score", "/Score"),
                ("Emeralds", "/Chaos Emeralds (127 = all seven)"), ("Continues", "/Continues"), ("Next special stage", "/Next special stage"),
            ],
            names: &[],
        },
    ],
};

// ------------------------------------------------------------------ Cave Story

const CAVE_STORY_PLUS: Spec = Spec {
    name: "Cave Story+",
    tab: "Quote",
    accent: (200, 60, 50),
    kind: Some(crate::fmt::cave_story::KIND),
    markers: &["/Slot 1/Health"],
    subtitle: "Profile.dat next to CaveStory+.exe: up to three save slots. Equipment, items and the stage are shown for reference.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[
        Table {
            title: "Save slots",
            path: "",
            name: &[],
            only: &["Slot 1", "Slot 2", "Slot 3"],
            cols: &[("Difficulty", "/Difficulty"), ("Health", "/Health"), ("Max health", "/Max health"), ("Whimsical stars", "/Whimsical stars")],
            names: &[],
        },
        Table { title: "Slot 1: weapons", path: "/Slot 1/Weapons", name: &[], only: &[], cols: &[("Type", "/Type"), ("Level", "/Level"), ("Energy", "/Energy"), ("Ammo", "/Ammo"), ("Max ammo (0 = unlimited)", "/Max ammo (0 = unlimited)")], names: &[] },
        Table { title: "Slot 2: weapons", path: "/Slot 2/Weapons", name: &[], only: &[], cols: &[("Type", "/Type"), ("Level", "/Level"), ("Energy", "/Energy"), ("Ammo", "/Ammo"), ("Max ammo (0 = unlimited)", "/Max ammo (0 = unlimited)")], names: &[] },
        Table { title: "Slot 3: weapons", path: "/Slot 3/Weapons", name: &[], only: &[], cols: &[("Type", "/Type"), ("Level", "/Level"), ("Energy", "/Energy"), ("Ammo", "/Ammo"), ("Max ammo (0 = unlimited)", "/Max ammo (0 = unlimited)")], names: &[] },
    ],
};

const CAVE_STORY: Spec = Spec {
    name: "Cave Story",
    tab: "Quote",
    accent: (200, 60, 50),
    kind: Some(crate::fmt::cave_story::KIND),
    markers: &["/Profile/Health"],
    subtitle: "Profile.dat of the freeware release: one save.",
    tiles: &[("Health", "/Profile/Health"), ("Max health", "/Profile/Max health"), ("Whimsical stars", "/Profile/Whimsical stars")],
    sections: &[Section { title: "Quote", rows: &[("Health", "/Profile/Health"), ("Max health", "/Profile/Max health"), ("Whimsical stars", "/Profile/Whimsical stars"), ("Stage number", "/Profile/Stage number"), ("Equipment", "/Profile/Equipment"), ("Items", "/Profile/Items")] }],
    lists: &[],
    tables: &[
        Table { title: "Weapons", path: "/Profile/Weapons", name: &[], only: &[], cols: &[("Type", "/Type"), ("Level", "/Level"), ("Energy", "/Energy"), ("Ammo", "/Ammo"), ("Max ammo (0 = unlimited)", "/Max ammo (0 = unlimited)")], names: &[] },
    ],
};

// ------------------------------------------------------------------ Lies of P

const LIES_OF_P: Spec = Spec {
    name: "Lies of P",
    tab: "Pinocchio",
    accent: (196, 160, 92),
    kind: Some("Unreal Engine save (LCharacterSaveGame)"),
    markers: &["/CharacterSaveData/PlayerLevel", "/CharacterSaveData/FirstStatSimpleList"],
    subtitle: "SaveData-<n>_Character_<slot>.sav. Ergo held is what you can spend; the level is kept with the stats, so raise both together.",
    tiles: &[("Ergo", "/CharacterSaveData/AcquisitionSoul"), ("Level", "/CharacterSaveData/PlayerLevel"), ("Humanity", "/CharacterSaveData/HumanityLevel")],
    sections: &[Section {
        title: "Character",
        rows: &[
            ("Ergo", "/CharacterSaveData/AcquisitionSoul"), ("Level", "/CharacterSaveData/PlayerLevel"),
            ("Ergo for next level", "/CharacterSaveData/NextLevelUpRequireSoul"), ("Humanity level", "/CharacterSaveData/HumanityLevel"),
            ("Humanity gained", "/CharacterSaveData/AcquisitionHumanity"), ("Golden Lie harvests ready", "/CharacterSaveData/GoldenTreeHarvestableCount"),
            ("Deaths", "/CharacterSaveData/YouDieCount"), ("Play time (seconds)", "/CharacterSaveData/CharacterPlayTime"),
        ],
    }],
    lists: &[List {
        title: "Items",
        path: "/CharacterSaveData/CharacterItem/PlayerItems",
        shape: Shape::Array { id_key: "FirstCodeName", qty_keys: &["Count"] },
        names: &[
            ("Reinforce_Blade_Common_G1", "Hidden Moonstone"), ("Reinforce_Blade_Common_G2", "Crescent Moonstone"),
            ("Reinforce_Blade_Common_G3", "Half Moonstone"), ("Reinforce_Blade_Common_G4", "Full Moonstone"),
            ("Reinforce_Hero_G1", "Dark Moon Moonstone of the Covenant"), ("Reinforce_Hero_G2", "Full Moonstone of the Covenant"),
            ("Reinforce_SlaveArm_G1", "Legion Caliber"), ("Exchange_GoldenFruit", "Gold Coin Fruit"),
            ("Consume_Rechargeable_1", "Pulse Cell"), ("Helpmate_Material", "Star Fragment"),
        ],
        max: Some(Max {
            ids: &[
                "Reinforce_Blade_Common_G1", "Reinforce_Blade_Common_G2", "Reinforce_Blade_Common_G3", "Reinforce_Blade_Common_G4",
                "Reinforce_Hero_G1", "Reinforce_Hero_G2", "Reinforce_SlaveArm_G1",
            ],
            cap: 99,
            caps: &[],
            estimate: true,
            about: "Raises the Moonstones for normal and special weapons and Legion Calibers you already have. Quest items, Quartz and consumables are left alone.",
        }),
    }],
    tables: &[Table {
        title: "Stats",
        path: "/CharacterSaveData/FirstStatSimpleList",
        name: &["/StatType"],
        only: &[],
        cols: &[("Points", "/StatData")],
        names: &[
            ("ELFirstStat::E_VITALITY", "Vitality"), ("ELFirstStat::E_VIGOR", "Vigor"), ("ELFirstStat::E_TENACITY", "Tenacity (unused)"),
            ("ELFirstStat::E_CAPACITY", "Capacity"), ("ELFirstStat::E_MOTIVITY", "Motivity"), ("ELFirstStat::E_TECHNIQUE", "Technique"),
            ("ELFirstStat::E_ADVANCE", "Advance"),
        ],
    }],
};

// ------------------------------------------------------------------ Tokyo Xtreme Racer (2025)

const TXR: Spec = Spec {
    name: "Tokyo Xtreme Racer (2025)",
    tab: "Driver",
    accent: (230, 70, 60),
    kind: Some("Unreal Engine save (BP_SaveData_C)"),
    markers: &["/user_info/Cp", "/user_info/PP"],
    subtitle: "UserData_00.sav. CP is money and cannot go above the CP limit next to it; PP buys perks.",
    tiles: &[("CP", "/user_info/Cp"), ("PP", "/user_info/PP"), ("Garage size", "/user_info/MaxGarageMyCarNum")],
    sections: &[
        Section {
            title: "Money and points",
            rows: &[
                ("CP", "/user_info/Cp"), ("CP limit", "/user_info/UpperLimitCP"), ("CP earned", "/user_info/CumulativeAcquisitionCP"),
                ("PP", "/user_info/PP"), ("PP limit", "/user_info/UpperLimitPP"), ("PP earned", "/user_info/CumulativeAcquisitionPP"),
                ("Garage size", "/user_info/MaxGarageMyCarNum"), ("Day", "/user_info/Day"),
            ],
        },
        Section {
            title: "Records",
            rows: &[
                ("Battles", "/user_info/BattleNum"), ("Wins", "/user_info/WinNum"), ("Draws", "/user_info/DrawNum"),
                ("Longest winning streak", "/user_info/MaxWiningStreak"), ("Rivals beaten", "/user_info/NumOfRivalDefeated"),
                ("Teams beaten", "/user_info/NumOfTeamDefeated"), ("Wanderers beaten", "/user_info/NumOfWandererDefeated"),
                ("Top speed", "/user_info/MaxSpeed"), ("Distance driven", "/user_info/TotalMileages"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Dave the Diver

const DAVE: Spec = Spec {
    name: "Dave the Diver",
    tab: "Dave",
    accent: (40, 150, 210),
    kind: None,
    markers: &["/PlayerInfo/m_Gold", "/Ingredients"],
    subtitle: "GameSave_NN_GD.sav, decrypted for you and encrypted again on save. Gold is the sushi bar's money; Bei buys gear from Duff and Cobra.",
    tiles: &[("Gold", "/PlayerInfo/m_Gold"), ("Bei", "/PlayerInfo/m_Bei"), ("Chef flames", "/PlayerInfo/m_ChefFlame")],
    sections: &[Section {
        title: "Dave",
        rows: &[
            ("Gold", "/PlayerInfo/m_Gold"), ("Bei", "/PlayerInfo/m_Bei"), ("Chef flames", "/PlayerInfo/m_ChefFlame"),
            ("Research points", "/PlayerInfo/m_researchPoint"), ("Trust points", "/PlayerInfo/m_trustPoint"),
            ("Deepest dive (m)", "/PlayerInfo/m_BestReachDepth"), ("Dives", "/GameInfo/totalDivingCount"),
        ],
    }],
    lists: &[List {
        title: "Ingredients",
        path: "/Ingredients",
        shape: Shape::Records { qty_key: "count" },
        names: &[],
        max: Some(Max {
            ids: DAVE_INGREDIENTS,
            cap: 999,
            caps: &[],
            estimate: true,
            about: "Raises every fish and farm ingredient you already have, the stock the sushi bar cooks from. Gold, Bei and gear are left alone.",
        }),
    }],
    tables: &[],
};

const DAVE_INGREDIENTS: &[&str] = &[
    "1021002", "1021003", "1021004", "1021005", "1021006", "1021007", "1021008", "1021009", "1021010", "1021011", "1021012", "1021013",
    "1021014", "1021015", "1021016", "1021017", "1021018", "1021019", "1021020", "1021022", "1021023", "1021027", "1021028", "1021029",
    "1021031", "1021032", "1021033", "1021034", "1021036", "1021040", "1021060", "1021101", "1021102", "1021103", "1021105", "1021106",
    "1021107", "1021108", "1021109", "1021110", "1021111", "1021112", "1021113", "1021114", "1021115", "1021116", "1021118", "1021121",
    "1021123", "1021129", "1021130", "1021131", "1021134", "1021136", "1021201", "1021202", "1021205", "1021208", "1021211", "1021212",
    "1021214", "1021215", "1021217", "1021218", "1021219", "1021220", "1021222", "1021301", "1021302", "1021303", "1021304", "1021305",
    "1021306", "1021401", "1021402", "1021403", "1021405", "1021407", "1021408", "1021410", "1021412", "1021413", "1021415", "1021501",
    "1021502", "1021503", "1021504", "1021505", "1021508", "1021550", "1022025", "1023001", "1023003", "1023005", "1023007", "1023009",
    "1023011", "1023013", "1023015", "1023017", "1023019", "1023021", "1023023", "1023025", "1023027", "1023028", "1023029", "1023030",
    "1023031", "1023032", "1023035", "1023037", "1023039", "1023043", "1023045", "1023047", "1023049", "1023051", "1023053", "1023055",
    "1023059", "1023061", "1023063", "1023065", "1025901", "1025902", "1025903", "1025904", "1025905", "1025906", "1025907", "1025908",
    "1025909", "1025910", "1026001", "1026002", "1026003", "1026004", "1026005", "1026006", "1026007", "1026008", "1026009", "1026010",
    "1026951", "1026952", "1027001", "1027002", "1027004", "1027008", "1027014", "1027016", "1027017", "1027018", "1027101", "1027102",
    "1027103", "1027104", "1027106", "1027107", "1027108", "1027109",
];

// ------------------------------------------------------------------ My Summer Car

const MSC: Spec = Spec {
    name: "My Summer Car",
    tab: "Finland",
    accent: (214, 168, 46),
    kind: Some(crate::fmt::es2::KIND),
    markers: &["/PlayerMoney", "/PlayerHunger"],
    subtitle: "defaultES2File.txt (items.txt holds the shop items). Money is in marks. The needs run from 0 (fine) to 100 (passing out); hunger, thirst and the rest are best set low.",
    tiles: &[("Money (mk)", "/PlayerMoney"), ("Hunger", "/PlayerHunger"), ("Thirst", "/PlayerThirst"), ("Fatigue", "/PlayerFatigue")],
    sections: &[Section {
        title: "Player",
        rows: &[
            ("Money (mk)", "/PlayerMoney"), ("Hunger", "/PlayerHunger"), ("Thirst", "/PlayerThirst"), ("Fatigue", "/PlayerFatigue"),
            ("Stress", "/PlayerStress"), ("Urine", "/PlayerUrine"), ("Dirtiness", "/PlayerDirtiness"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ ULTRAKILL

const ULTRAKILL: Spec = Spec {
    name: "ULTRAKILL",
    tab: "V1",
    accent: (200, 30, 40),
    kind: Some(".NET BinaryFormatter (GameProgressMoneyAndGear)"),
    markers: &["/money", "/rev0"],
    subtitle: "generalprogress.bepis in the save slot folder. Each weapon variant is 1 when owned and 0 when not; the level files (lvlNprogress.bepis) hold ranks and secrets.",
    tiles: &[("Points (P)", "/money")],
    sections: &[
        Section {
            title: "Revolver and shotgun",
            rows: &[
                ("Piercer", "/rev0"), ("Marksman", "/rev1"), ("Sharpshooter", "/rev2"), ("Slab revolvers", "/revalt"),
                ("Core Eject", "/sho0"), ("Pump Charge", "/sho1"), ("Sawed-On", "/sho2"),
            ],
        },
        Section {
            title: "Nailgun, railcannon, rockets, arms",
            rows: &[
                ("Attractor", "/nai0"), ("Overheat", "/nai1"), ("JumpStart", "/nai2"), ("Sawblade launchers", "/naialt"),
                ("Electric railcannon", "/rai0"), ("Screwdriver", "/rai1"), ("Malicious", "/rai2"),
                ("Freezeframe", "/rock0"), ("S.R.S. Cannon", "/rock1"), ("Firestarter", "/rock2"),
                ("Knuckleblaster", "/arm1"), ("Whiplash", "/arm2"),
            ],
        },
        Section {
            title: "Progress",
            rows: &[("Tutorial beaten", "/tutorialBeat"), ("Intro seen", "/introSeen"), ("Clash mode unlocked", "/clashModeUnlocked")],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ RPG Maker XP games

const RPGXP_ACTOR_COLS: &[(&str, &str)] = &[
    ("Level", "/@level"), ("EXP", "/@exp"), ("HP", "/@hp"), ("SP", "/@sp"), ("Max HP +", "/@maxhp_plus"), ("Max SP +", "/@maxsp_plus"),
    ("STR +", "/@str_plus"), ("DEX +", "/@dex_plus"), ("AGI +", "/@agi_plus"), ("INT +", "/@int_plus"),
];

const RPGXP: Spec = Spec {
    name: "RPG Maker XP game",
    tab: "Party",
    accent: (90, 130, 200),
    kind: Some("RPG Maker XP/VX/Ace (Ruby Marshal)"),
    markers: &["/Part 9/@gold", "/Part 8/@data", "/Part 9/@items"],
    subtitle: "SaveN.rxdata. Items, weapons and armour are listed by their number in the game's database (Data/Items.rxdata), with how many you carry. HP and SP above the maximum are cut back by the game. The save keeps the party and every actor as separate copies, so change a party member in both tables.",
    tiles: &[("Gold", "/Part 9/@gold"), ("Steps", "/Part 9/@steps")],
    sections: &[],
    lists: &[
        List { title: "Items (by database number)", path: "/Part 9/@items", shape: Shape::Map, names: &[], max: None },
        List { title: "Weapons (by database number)", path: "/Part 9/@weapons", shape: Shape::Map, names: &[], max: None },
        List { title: "Armour (by database number)", path: "/Part 9/@armors", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[
        Table { title: "Party (what the menu and battles use)", path: "/Part 9/@actors", name: &["/@name"], only: &[], cols: RPGXP_ACTOR_COLS, names: &[] },
        Table { title: "Every actor (copied in when someone joins the party)", path: "/Part 8/@data", name: &["/@name"], only: &[], cols: RPGXP_ACTOR_COLS, names: &[] },
    ],
};

// ------------------------------------------------------------------ Resident Evil 5

const RE5: Spec = Spec {
    name: "Resident Evil 5",
    tab: "BSAA",
    accent: (190, 120, 40),
    kind: Some(crate::fmt::re5::KIND),
    markers: &["/Save/Money"],
    subtitle: "savedata.bin of the Steam edition. Money buys and upgrades weapons; exchange points buy the Bonus Features (costumes, filters, infinite ammo, figures).",
    tiles: &[("Money", "/Save/Money"), ("Exchange points", "/Save/Exchange points")],
    sections: &[Section { title: "Save", rows: &[("Money", "/Save/Money"), ("Exchange points", "/Save/Exchange points"), ("Saved", "/Save/Saved")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Anno 1800

const ANNO1800: Spec = Spec {
    name: "Anno 1800",
    tab: "Company",
    accent: (176, 140, 70),
    kind: Some(crate::fmt::anno::KIND),
    markers: &["/Player/Coins"],
    subtitle: "A .a7s save (Documents\\Anno 1800\\accounts\\<id>\\<profile>\\savegame). Coins are your company's money; the load screen's figure is changed with them. Other amounts of your company's storage are in the Tree, by GUID.",
    tiles: &[("Coins", "/Player/Coins"), ("Influence", "/Player/Influence")],
    sections: &[],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ For The King

const FTK: Spec = Spec {
    name: "For The King",
    tab: "Party",
    accent: (196, 150, 60),
    kind: None,
    markers: &["/m_GameInfo/m_GameDefinition", "/m_PlayerSerialize", "/m_GameStates/m_GameFlowStates/m_LifePool/$content"],
    subtitle: "An adventure save (.run) in the game's save folder. Each hero keeps their own gold. Health above the hero's maximum is cut back by the game; the life pool is how many times the party can still be revived.",
    tiles: &[("Life pool", "/m_GameStates/m_GameFlowStates/m_LifePool/$content")],
    sections: &[Section {
        title: "Adventure",
        rows: &[
            ("Life pool", "/m_GameStates/m_GameFlowStates/m_LifePool/$content"), ("Extra life pool", "/m_GameStates/m_GameFlowStates/m_LifePoolMaxMod/$content"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: "Heroes",
        path: "/m_PlayerSerialize",
        name: &["/m_InstData/3/$content"],
        only: &[],
        cols: &[
            ("Gold", "/m_StateCSData/m_Gold/$content"), ("Level", "/m_StateCSData/m_PlayerLevel/$content"), ("XP", "/m_StateCSData/m_PlayerXP/$content"),
            ("Health", "/m_StateCSData/m_HealthCurrent/$content"), ("Focus", "/m_StateCSData/m_FocusPoints/$content"),
            ("Max health +", "/m_StateCSData/m_AugmentedMaxHealth/$content"), ("Max focus +", "/m_StateCSData/m_AugmentedMaxFocus/$content"),
        ],
        names: &[],
    }],
};
// ------------------------------------------------------------------ LEGO Star Wars: The Skywalker Saga

const LEGO_TSS: Spec = Spec {
    name: "LEGO Star Wars: The Skywalker Saga",
    tab: "Studs",
    accent: (240, 200, 40),
    kind: Some(crate::fmt::lego_tss::KIND),
    markers: &["/Studs"],
    subtitle: "COLLECTABLES.BLOB in a SLOTn folder. The stud wallet is saved twice; both copies are written.",
    tiles: &[("Studs", "/Studs")],
    sections: &[Section { title: "Studs", rows: &[("Studs", "/Studs")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Older LEGO games (stud counter)

const LEGO_CLASSIC: Spec = Spec {
    name: "Older LEGO games",
    tab: "Studs",
    accent: (240, 200, 40),
    kind: Some(crate::fmt::lego_classic::KIND),
    markers: &["/Studs"],
    subtitle: "Studs in LEGO Star Wars (2005), The Complete Saga, Batman and Indiana Jones; the save's checksum is rewritten.",
    tiles: &[("Studs", "/Studs")],
    sections: &[Section { title: "Studs", rows: &[("Studs", "/Studs"), ("Game", "/Game")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Mutant Year Zero: Road to Eden

const MYZ: Spec = Spec {
    name: "Mutant Year Zero: Road to Eden",
    tab: "Squad",
    accent: (150, 170, 70),
    kind: Some("Unreal Engine save (ZoneSavegame)"),
    markers: &["/Inventory/Scrap", "/PlayerUnits"],
    subtitle: "A Slot or Autosave file in the SaveGames folder. Scrap and weapon parts are the Ark's shared stock; each squad member has their own level, health and mutation points.",
    tiles: &[("Scrap", "/Inventory/Scrap"), ("Weapon parts", "/Inventory/WeaponParts"), ("Medpacks", "/Inventory/Medpacks")],
    sections: &[
        Section {
            title: "The Ark",
            rows: &[
                ("Artifact points available", "/Ark/AvailableArtifactPoints"), ("Artifact points spent", "/Ark/SpentArtifactPoints"),
                ("Population", "/Ark/Population"), ("Food quality", "/Ark/FoodQuality"), ("Recruits this week", "/Ark/AvailableRecruitsThisWeek"),
                ("Electricity level", "/Ark/ElectricityLevel"), ("Science level", "/Ark/ScienceLevel"), ("Living level", "/Ark/LivingLevel"),
            ],
        },
        Section { title: "Squad", rows: &[("Squad level", "/SquadXPData/SquadLevel"), ("Elias level", "/EliasLevel"), ("Week", "/CurrentWeek")] },
    ],
    lists: &[List {
        title: "Shared stock",
        path: "/Inventory",
        shape: Shape::Map,
        names: &[],
        max: Some(Max {
            ids: &["Scrap", "WeaponParts"],
            cap: 999,
            caps: &[],
            estimate: true,
            about: "Raises scrap and weapon parts to 999 (an estimate: no public source states a cap). Medpacks, artifacts, notes and gear are left alone.",
        }),
    }],
    tables: &[Table {
        title: "Squad members",
        path: "/PlayerUnits",
        name: &["/UniqueCharacterName"],
        only: &[],
        cols: &[
            ("Level", "/Level"), ("XP", "/Experience"), ("Max health", "/MaxHitpoints"), ("Health", "/Hitpoints"),
            ("Max sanity", "/MaxSanity"), ("Sanity", "/Sanity"), ("Mutation points", "/MutationPoints"),
        ],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Car Mechanic Simulator 2021

const CMS21: Spec = Spec {
    name: "Car Mechanic Simulator 2021",
    tab: "Mechanic",
    accent: (230, 120, 40),
    kind: None,
    markers: &["/globalDataWrapper/PlayerMoney", "/carsInGarage"],
    subtitle: "A profile file (profile0.cms21 …) in the game's Save folder; the Backup folder keeps an older copy, so edit the one the game loads.",
    tiles: &[("Money", "/globalDataWrapper/PlayerMoney"), ("Level", "/globalDataWrapper/PlayerLevel"), ("XP", "/globalDataWrapper/PlayerExp"), ("Scrap", "/globalDataWrapper/PlayerScraps")],
    sections: &[
        Section {
            title: "Profile",
            rows: &[("Name", "/Name"), ("Play time", "/PlayTime"), ("Difficulty", "/Difficulty"), ("Best race time", "/BestRaceTime"), ("Top speed", "/TopSpeed"), ("Finished tutorial", "/FinishedTutorial")],
        },
        Section {
            title: "Garage and missions",
            rows: &[
                ("Parking levels unlocked", "/globalDataWrapper/UnlockedParkingLevels"), ("Barns found", "/globalDataWrapper/BarnsAmount"),
                ("Missions finished", "/globalDataWrapper/MissionsFinished"), ("Warehouses unlocked", "/warehouseData/amountOfUnlockedWarehouses"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Medieval Dynasty

const MEDIEVAL_DYNASTY: Spec = Spec {
    name: "Medieval Dynasty",
    tab: "Settler",
    accent: (170, 130, 70),
    kind: Some("Unreal Engine save (SAVE_Game_C)"),
    markers: &["/DynastyReputation", "/PlayerData"],
    subtitle: "A Quicksave or Autosave file. Money is the Coin item in the bag; skill levels and the four technology point pools are below.",
    tiles: &[("Reputation", "/DynastyReputation"), ("Tax", "/CurrentTax")],
    sections: &[
        Section {
            title: "Needs",
            rows: &[
                ("Health", "/PlayerData/Stats_57_CF87A144455D92DB6C482FB22BA8C487/Health_5_87035C8C4554FD207A64F392A82F8C80"),
                ("Stamina", "/PlayerData/Stats_57_CF87A144455D92DB6C482FB22BA8C487/Stamina_9_BDAAE3B24DB2260CE2A8E291179CE762"),
                ("Food", "/PlayerData/Stats_57_CF87A144455D92DB6C482FB22BA8C487/Food_3_AB4C50FD4267F3D70946EABDE7841042"),
                ("Water", "/PlayerData/Stats_57_CF87A144455D92DB6C482FB22BA8C487/Water_7_D4D5072F4C90181D22D20584CF754102"),
                ("Age", "/PlayerData/Age_17_95D4C25B404796B039F50EAB60304D82"),
            ],
        },
        Section {
            title: "Technology points",
            rows: &[
                ("Building", "/TechnologyPoints/Building_2_1C1D5F674D6813156020C48175C195CD"),
                ("Survival", "/TechnologyPoints/Survival_6_6503F9984203857C64CE4499504F6D5B"),
                ("Farming", "/TechnologyPoints/Farming_7_B3769E3E4F33E547F9CE24837510332E"),
                ("Crafting", "/TechnologyPoints/Crafting_8_538BF3D54E1E6799C4919CB13D94A5C4"),
            ],
        },
        Section {
            title: "Skill levels",
            rows: &[
                ("Extraction", "/PlayerData/Skills_27_BC25071441646825BA0361A6C186A2B6/E_L_62_87035C8C4554FD207A64F392A82F8C80"),
                ("Hunting", "/PlayerData/Skills_27_BC25071441646825BA0361A6C186A2B6/H_L_65_BDAAE3B24DB2260CE2A8E291179CE762"),
                ("Farming", "/PlayerData/Skills_27_BC25071441646825BA0361A6C186A2B6/F_L_83_AB4C50FD4267F3D70946EABDE7841042"),
                ("Diplomacy", "/PlayerData/Skills_27_BC25071441646825BA0361A6C186A2B6/D_L_71_D4D5072F4C90181D22D20584CF754102"),
                ("Survival", "/PlayerData/Skills_27_BC25071441646825BA0361A6C186A2B6/S_L_74_380CB82E4C3CAFE24FB611B17954B35F"),
                ("Crafting", "/PlayerData/Skills_27_BC25071441646825BA0361A6C186A2B6/C_L_80_2067C5474E664F95D07DB69DD2B8968B"),
            ],
        },
    ],
    lists: &[List {
        title: "Bag",
        path: "/PlayerData/EQ_58_E8A95A55473CB02502A1959F9FCF1D1C",
        shape: Shape::Array { id_key: "ID_8_4F27866A4A6D5E95B59001A7FC0A0EB3", qty_keys: &["Count_5_C53A69004BB01EEBBA02CD86D4CBA6C6"] },
        names: &[],
        max: Some(Max {
            ids: &["Log", "Stick", "Clay", "Plank", "Rock", "Leather", "Straw", "Salt", "Fur", "Wool", "Daub", "Feather", "Firewood", "LinenThread", "WoolThread"],
            cap: 999,
            caps: &[],
            estimate: true,
            about: "Raises the plain building and crafting materials in the bag (wood, stone, clay, leather, straw, salt, fur, wool, daub, feathers, thread) to 999, an estimate: the game limits what fits by weight and no public source gives a stack cap. Money, tools, food and clothing are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Warhammer 40,000: Mechanicus

const MECHANICUS: Spec = Spec {
    name: "Warhammer 40,000: Mechanicus",
    tab: "Cadmus",
    accent: (200, 60, 50),
    kind: Some(crate::fmt::mechanicus::KIND),
    markers: &["/Ark/Blackstone"],
    subtitle: "Meta.sav of a save slot. Blackstone is the Ark's research currency; SaveState.sav (the game's checksum of this file) is updated when you save.",
    tiles: &[("Blackstone", "/Ark/Blackstone"), ("Cognition bonus points", "/Ark/Cognition bonus points")],
    sections: &[Section {
        title: "Ark",
        rows: &[
            ("Extra deployment points", "/Ark/Extra deployment points"), ("Troop deployment cost reduction", "/Ark/Troop deployment cost reduction"),
            ("Techpriest slots", "/Ark/Techpriest slots"), ("Troop slots", "/Ark/Troop slots"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: "Techpriests",
        path: "/Techpriests",
        name: &["/Name"],
        only: &[],
        cols: &[("Level", "/Level"), ("XP", "/XP"), ("Skill points", "/Skill points"), ("Slot points", "/Slot points"), ("Augment capacity", "/Augment capacity"), ("Health", "/Health")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Ori and the Blind Forest

const ORI: Spec = Spec {
    name: "Ori and the Blind Forest",
    tab: "Ori",
    accent: (90, 170, 230),
    kind: Some(crate::fmt::ori::KIND),
    markers: &["/Ori"],
    subtitle: "saveFile<n>.sav: level, ability points, life and energy cells, keystones, mapstones and every skill and ability-tree upgrade.",
    tiles: &[("Level", "/Ori/Level"), ("Ability points", "/Ori/Ability points"), ("Keystones", "/Ori/Keystones"), ("Mapstones", "/Ori/Mapstones")],
    sections: &[
        Section {
            title: "Ori",
            rows: &[
                ("Level", "/Ori/Level"), ("Experience", "/Ori/Experience"), ("Ability points", "/Ori/Ability points"),
                ("Life (4 per cell)", "/Ori/Life (4 per cell)"), ("Max life (4 per cell)", "/Ori/Max life (4 per cell)"),
                ("Energy", "/Ori/Energy"), ("Max energy", "/Ori/Max energy"), ("Keystones", "/Ori/Keystones"),
                ("Mapstones", "/Ori/Mapstones"), ("Deaths", "/Ori/Deaths"),
            ],
        },
        Section { title: "Skills", rows: &[("Spirit Flame", "/Skills/Spirit Flame"), ("Wall Jump", "/Skills/Wall Jump"), ("Charge Flame", "/Skills/Charge Flame"), ("Double Jump", "/Skills/Double Jump"), ("Bash", "/Skills/Bash"), ("Stomp", "/Skills/Stomp"), ("Glide", "/Skills/Glide"), ("Climb", "/Skills/Climb"), ("Charge Jump", "/Skills/Charge Jump"), ("Dash", "/Skills/Dash"), ("Light Burst", "/Skills/Light Burst")] },
        Section { title: "Ability tree", rows: &[("Spirit Efficiency", "/Ability tree/Spirit Efficiency"), ("Water Breath", "/Ability tree/Water Breath"), ("Charge Flame Blast", "/Ability tree/Charge Flame Blast"), ("Charge Flame Burn", "/Ability tree/Charge Flame Burn"), ("Triple Jump", "/Ability tree/Triple Jump"), ("Ultra Defense", "/Ability tree/Ultra Defense"), ("Life Efficiency", "/Ability tree/Life Efficiency"), ("Sense", "/Ability tree/Sense"), ("Ultra Stomp", "/Ability tree/Ultra Stomp"), ("Quick Flame", "/Ability tree/Quick Flame"), ("Map Markers", "/Ability tree/Map Markers"), ("Energy Efficiency", "/Ability tree/Energy Efficiency"), ("Life Markers", "/Ability tree/Life Markers"), ("Energy Markers", "/Ability tree/Energy Markers"), ("Ability Markers", "/Ability tree/Ability Markers"), ("Rekindle", "/Ability tree/Rekindle"), ("Regroup", "/Ability tree/Regroup"), ("Charge Flame Efficiency", "/Ability tree/Charge Flame Efficiency"), ("Ultra Soul Link", "/Ability tree/Ultra Soul Link"), ("Soul Link Efficiency", "/Ability tree/Soul Link Efficiency"), ("Split Flame", "/Ability tree/Split Flame"), ("Spark Flame", "/Ability tree/Spark Flame"), ("Cinder Flame", "/Ability tree/Cinder Flame"), ("Ultra Split Flame", "/Ability tree/Ultra Split Flame"), ("Ultra Light Burst", "/Ability tree/Ultra Light Burst"), ("Charge Dash", "/Ability tree/Charge Dash"), ("Air Dash", "/Ability tree/Air Dash"), ("Magnet", "/Ability tree/Magnet"), ("Ultra Magnet", "/Ability tree/Ultra Magnet"), ("Rapid Flame", "/Ability tree/Rapid Flame")] },
        Section { title: "This save", rows: &[("Area", "/Save/Area"), ("Completion (%)", "/Save/Completion (%)"), ("Play time", "/Save/Play time")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ OMORI

const OMORI_ITEMS: &[(&str, &str)] = &[("2", "Cold Steak"), ("3", "Microwaved Steak"), ("4", "Snacktime"), ("7", "Rubberband"), ("13", "Tofu"), ("14", "Candy"), ("15", "Smores"), ("16", "Granola Bar"), ("17", "Bread"), ("18", "Nachos"), ("19", "Chicken Wing"), ("20", "Hot Dog"), ("21", "Waffle"), ("22", "Pancake"), ("23", "Pizza Slice"), ("24", "Fish Taco"), ("25", "Cheeseburger"), ("26", "Chocolate"), ("27", "Donut"), ("28", "Ramen"), ("29", "Spaghetti"), ("30", "Popcorn"), ("31", "Fries"), ("32", "Cheese Wheel"), ("33", "Whole Chicken"), ("34", "Whole Pizza"), ("35", "Sno-Cone"), ("36", "Tomato"), ("37", "Combo Meal"), ("38", "Dino Clumps"), ("39", "Dino Pasta"), ("41", "Mari'S Cookie"), ("42", "Mari'S Cookie"), ("46", "Plum Juice"), ("47", "Apple Juice"), ("48", "Cherry Soda"), ("49", "Star Fruit Soda"), ("50", "Breadfruit Juice"), ("51", "Tasty Soda"), ("52", "Lemonade"), ("53", "Peach Soda"), ("54", "Butt Peach Soda"), ("55", "Orange Juice"), ("56", "Pineapple Juice"), ("57", "Grape Soda"), ("58", "Banana Smoothie"), ("59", "Mango Smoothie"), ("60", "Berry Smoothie"), ("61", "Melon Smoothie"), ("62", "S.Berry Smoothie"), ("63", "Watermelon Juice"), ("64", "Dino Melon Soda"), ("65", "Dino Smoothie"), ("66", "Bottled Water"), ("67", "Fruit Juice?"), ("68", "Prune Juice"), ("69", "Rotten Milk"), ("70", "Milk"), ("73", "Can"), ("74", "Glass Bottle"), ("75", "Cardboard"), ("76", "Dead Batteries"), ("77", "Computer Part"), ("78", "Jacks"), ("79", "Rubber Band"), ("80", "Dynamite"), ("81", "Sparkler"), ("82", "Confetti"), ("83", "Poetry Book"), ("84", "Rain Cloud"), ("85", "Present"), ("86", "Air Horn"), ("87", "Dandelion"), ("88", "Mush"), ("89", "Life Jam"), ("91", "Coffee"), ("92", "Dino Jam"), ("93", "Mystery Potion"), ("94", "Big Rubber Band"), ("95", "Pearl"), ("96", "Jam Packets"), ("101", "Donut"), ("102", "Candy"), ("103", "Salad"), ("104", "Bread"), ("105", "Chocolate"), ("106", "Pizza Slice"), ("107", "Hero Sandwich"), ("108", "Caramel Apple"), ("109", "Whole Pizza"), ("110", "Pie"), ("111", "Big Bag Of Candy"), ("112", "Slice Of Cake"), ("113", "Bandage"), ("114", "First Aid Kit"), ("115", "Hamburger"), ("116", "Meat"), ("117", "Fish"), ("118", "Pepper Spray"), ("120", "Tasty Soda"), ("121", "Orange Joe"), ("122", "Apple Juice"), ("123", "Orange Juice"), ("124", "Coffee"), ("125", "Small Sketch"), ("128", "Foe Facts!"), ("129", "Junkyard Key"), ("130", "Special Mixtape"), ("131", "Show Tickets"), ("132", "Cool Key Card"), ("133", "V.I.P. Key Card"), ("134", "Wooden Track"), ("135", "Slimy Key Card"), ("136", "Emotion Chart"), ("138", "Cold Steak"), ("139", "Lukewarm Steak"), ("140", "Hero'S Gift"), ("141", "\"Pizza\" Order"), ("142", "Bakery Order"), ("143", "Homecooked Meal"), ("144", "Microwaved Meal"), ("146", "Toy Box Key"), ("151", "Light Bulb"), ("153", "Toy Box Key"), ("157", "Sheet Music"), ("158", "Flower Crown"), ("161", "Joke Book"), ("162", "Big Airhorn"), ("163", "B.E.D."), ("164", "Blackmail"), ("165", "Strange List"), ("166", "Interesting Book"), ("167", "Thingamabob"), ("168", "Whatchamacallit"), ("169", "Doohickey"), ("170", "Care Package"), ("171", "Giant Check"), ("172", "Spooky Map"), ("173", "Batteries"), ("174", "Orange Crest"), ("176", "Teddy Bear"), ("177", "Rat Meat"), ("178", "Snot"), ("179", "Scrap Metal"), ("180", "Thank-You Fossil"), ("181", "Pessi'S Thing"), ("185", "Potato Spoon"), ("186", "Plastic Spoon"), ("187", "Wooden Spoon"), ("188", "Silver Spoon"), ("190", "Butt Certificate"), ("191", "Flower Puzzle"), ("192", "Ancient Code"), ("195", "Seashell"), ("196", "Rare Card"), ("197", "Small Sketch"), ("198", "Nostalgic Cd"), ("199", "Celestial Cd"), ("200", "Merry Cd"), ("201", "Devilish Cd"), ("202", "Otherworldly Cd"), ("203", "Lively Cd"), ("204", "Brutal Cd"), ("205", "Ethereal Cd"), ("206", "Ghoulish Cd"), ("207", "Tranquil Cd"), ("208", "Chill Cd"), ("209", "Dynamic Cd"), ("210", "Secret Cd"), ("211", "Trash"), ("212", "Bubble Wrap"), ("213", "Arcade Piece"), ("214", "Flower Clip"), ("215", "Prescription"), ("216", "Id Card"), ("217", "Lunchbox"), ("218", "Letter For Dad"), ("219", "Garden Shears"), ("220", "Green Flyer"), ("221", "Blue Flyer"), ("222", "Tv Remote"), ("223", "Granny'S Medicine"), ("224", "Floor Lamp"), ("225", "Flowers"), ("226", "Missing Teeth"), ("227", "Coin"), ("241", "Sprout Mole Masks"), ("242", "Matchbox"), ("243", "Bad Drawing"), ("244", "Clems"), ("246", "Recipe 4 Disaster"), ("247", "Sheet Music"), ("248", "Pyramid Key"), ("249", "Treasure Map"), ("250", "Train Pass"), ("251", "Sno-Cone Ticket"), ("252", "Actual Mole"), ("254", "Self-Help Guide"), ("286", "Tomato (Backup)"), ("287", "Combo Meal (Backup)"), ("401", "Mixtape"), ("410", "Sugar"), ("411", "All-Purpose Flour"), ("412", "Garden Soil"), ("413", "Strawberries"), ("414", "Pineberries"), ("415", "Bottles Of Fruit Juice"), ("416", "Stick Of Butter"), ("417", "Jar Of Pickles"), ("418", "Blue Fish"), ("419", "Egg"), ("420", "Leather Shoe"), ("421", "Roast Chicken"), ("700", "Twenty Dollars"), ("703", "'Key' Ingredient"), ("800", "Yellow Keychain"), ("801", "Green Keychain"), ("802", "Pink Keychain"), ("803", "Amazing Keychain"), ("804", "Purple Ball"), ("805", "Pink Ball"), ("806", "Green Ball"), ("807", "Red Hand"), ("808", "Green Hand"), ("809", "Purple Hand"), ("810", "Green Slinky"), ("811", "Blue Slinky"), ("812", "Yellow Slinky"), ("876", "Dreamworld Album (Hidden)"), ("877", "DW Polaroid 1"), ("878", "DW Polaroid 2"), ("879", "DW Polaroid 3"), ("880", "DW Polaroid 4"), ("881", "DW Polaroid 5"), ("882", "DW Polaroid 6"), ("883", "DW Polaroid 7"), ("884", "DW Polaroid 8"), ("885", "DW Polaroid 9"), ("886", "DW Polaroid 10"), ("887", "DW Polaroid 11"), ("888", "DW Polaroid 12"), ("889", "BlackSpace Album (Hidden)"), ("890", "BS Polaroid 1"), ("891", "BS Polaroid 2"), ("892", "BS Polaroid 3"), ("893", "BS Polaroid 4"), ("894", "BS Polaroid 5"), ("895", "BS Polaroid 6"), ("896", "BS Polaroid 7"), ("897", "BS Polaroid 8"), ("898", "BS Polaroid 9"), ("899", "BS Polaroid 10"), ("900", "BS Polaroid 11"), ("901", "BS Polaroid 12"), ("902", "BS Polaroid 13"), ("903", "BS Polaroid 14"), ("904", "BS Polaroid 15"), ("905", "BS Polaroid 16"), ("906", "BS Polaroid 17"), ("907", "BS Polaroid 18"), ("908", "BS Polaroid 19"), ("909", "BS Polaroid 20"), ("910", "BS Polaroid 21"), ("911", "BS Polaroid 22"), ("912", "BS Polaroid 23"), ("913", "BS Polaroid 24"), ("914", "Faraway Album (Hidden)"), ("915", "FA Polaroid 1"), ("916", "FA Polaroid 2"), ("917", "FA Polaroid 3"), ("918", "FA Polaroid 4"), ("919", "FA Polaroid 5"), ("920", "FA Polaroid 6"), ("921", "FA Polaroid 7"), ("922", "FA Polaroid 8"), ("923", "FA Polaroid 9"), ("924", "FA Polaroid 10"), ("925", "FA Polaroid 11"), ("926", "FA Polaroid 12"), ("927", "FA Polaroid 13"), ("928", "FA Polaroid 14"), ("929", "FA Polaroid 15"), ("930", "FA Polaroid 16"), ("931", "FA Polaroid 17"), ("932", "FA Polaroid 18"), ("933", "FA Polaroid 19"), ("934", "FA Polaroid 20"), ("935", "FA Polaroid 21"), ("936", "FA Polaroid 22"), ("937", "FA Polaroid 23"), ("938", "FA Polaroid 24"), ("939", "FA Polaroid 25"), ("940", "FA Polaroid 26"), ("941", "FA Polaroid 27"), ("942", "FA Polaroid 28"), ("943", "FA Polaroid 29"), ("944", "FA Polaroid 30"), ("945", "FA Polaroid 31"), ("946", "FA Polaroid 32"), ("947", "FA Polaroid 33"), ("948", "FA Polaroid 34"), ("949", "FA Polaroid 35"), ("950", "FA Polaroid 36"), ("951", "FA Polaroid 37"), ("952", "FA Polaroid 38"), ("953", "FA Polaroid 39"), ("954", "FA Polaroid 40"), ("955", "FA Polaroid 41"), ("956", "FA Polaroid 42"), ("957", "FA Polaroid 43"), ("958", "FA Polaroid 44"), ("959", "FA Polaroid 45"), ("960", "FA Polaroid 46"), ("961", "FA Polaroid 47"), ("962", "FA Polaroid 48"), ("963", "DW Polaroid 13"), ("964", "Blurry Photo"), ("966", "Photo Album"), ("967", "Photo Album"), ("968", "Photo Album"), ("970", "FA Polaroid 1"), ("971", "FA Polaroid 4"), ("972", "FA Polaroid 5"), ("973", "FA Polaroid 8"), ("974", "FA Polaroid 9"), ("975", "FA Polaroid 10"), ("976", "FA Polaroid 11"), ("977", "FA Polaroid 12"), ("978", "FA Polaroid 16"), ("979", "FA Polaroid 18"), ("980", "FA Polaroid 19"), ("981", "FA Polaroid 20"), ("982", "FA Polaroid 21"), ("983", "FA Polaroid 22"), ("984", "FA Polaroid 25"), ("985", "FA Polaroid 26"), ("986", "FA Polaroid 27"), ("987", "FA Polaroid 31"), ("988", "FA Polaroid 32"), ("989", "FA Polaroid 33"), ("990", "FA Polaroid 34"), ("991", "FA Polaroid 35"), ("992", "FA Polaroid 38"), ("993", "FA Polaroid 43"), ("994", "FA Polaroid 44")];
const OMORI_WEAPONS: &[(&str, &str)] = &[("2", "Shiny Knife"), ("3", "Knife"), ("4", "Dull Knife"), ("5", "Rusty Knife"), ("6", "Red Knife"), ("8", "Fly Swatter"), ("9", "Hands"), ("10", "Violin"), ("11", "Violin"), ("12", "Violin"), ("14", "Stuffed Toy"), ("15", "Comet Hammer"), ("16", "Body Pillow"), ("17", "Pool Noodle"), ("18", "Cool Noodle"), ("19", "Hero'S Trophy"), ("20", "Mailbox"), ("21", "Baguette"), ("22", "Baseball Bat"), ("23", "Lol Sword"), ("25", "Nail Bat"), ("28", "Rubber Ball"), ("29", "Meteor Ball"), ("30", "Blood Orange"), ("31", "Jack"), ("32", "Beach Ball"), ("33", "Coconut"), ("34", "Globe"), ("35", "Chicken Ball"), ("36", "Snowball"), ("37", "Basketball"), ("42", "Basketball"), ("45", "Spatula"), ("46", "Rolling Pin"), ("47", "Teapot"), ("48", "Frying Pan"), ("49", "Blender"), ("50", "Baking Pan"), ("51", "Tenderizer"), ("52", "Ol' Reliable"), ("53", "Shucker"), ("55", "Fist"), ("61", "Steak Knife"), ("62", "Sweetheart Bust"), ("110", "Steak Knife"), ("112", "Shiny Knife"), ("113", "Knife"), ("114", "Dull Knife")];
const OMORI_CHARMS: &[(&str, &str)] = &[("2", "\"Gold\" Watch"), ("4", "3D Glasses"), ("5", "3-Leaf Clover"), ("6", "4-Leaf Clover"), ("7", "5-Leaf Clover"), ("8", "Appendix"), ("9", "Baby Chicken"), ("10", "Backpack"), ("11", "Baseball Cap"), ("12", "Binoculars"), ("13", "Blanket"), ("14", "Book"), ("15", "Bow-Tie"), ("16", "Bracelet"), ("17", "Breadphones"), ("18", "Bubble Wrap"), ("19", "Bunny Ears"), ("20", "Cactus"), ("21", "Cat Ears"), ("22", "Cellphone"), ("23", "Clam Coin"), ("24", "Cool Glasses"), ("25", "Bottle Cap"), ("26", "Cough Mask"), ("27", "Daisy"), ("28", "Eye Patch"), ("29", "Faux Tail"), ("30", "Fedora"), ("31", "Finger"), ("32", "Flower Clip"), ("33", "Fox Tail"), ("34", "Friendship Bracelet"), ("35", "Nerdy Glasses"), ("36", "Gold Watch"), ("37", "Hard Hat"), ("38", "Headband"), ("39", "Heart String"), ("40", "High Heels"), ("41", "Homework"), ("42", "Inner Tube"), ("43", "Lucky Token"), ("44", "Magical Bean"), ("45", "Monocle"), ("46", "Mustache"), ("47", "Onion Ring"), ("48", "Oven Mitts"), ("49", "Paper Bag"), ("50", "Hector"), ("51", "Pretty Bow"), ("52", "Punching Bag"), ("53", "Rabbit Foot"), ("54", "Red Ribbon"), ("55", "Deep Poetry Book"), ("56", "Rubber Duck"), ("57", "Sales Tag"), ("58", "Seashell Necklace"), ("59", "Seer Goggles"), ("60", "Top Hat"), ("61", "Pet Rock"), ("62", "Hector Jr"), ("63", "Wedding Ring"), ("64", "Wishbone"), ("65", "Veggie Kid"), ("66", "Watering Pail"), ("67", "Rake"), ("68", "Scarf"), ("69", "Teddy Bear"), ("70", "Cotton Ball"), ("71", "Flashlight"), ("72", "Universal Remote"), ("73", "Chef'S Hat"), ("74", "Tv Remote"), ("75", "Sunscreen"), ("76", "Contract"), ("77", "Flower Crown"), ("78", "Abbi'S Eye"), ("81", "Red Hand"), ("82", "Green Hand"), ("83", "Purple Hand"), ("84", "Purple Ball"), ("85", "Pink Ball"), ("86", "Green Ball"), ("87", "Green Slinky"), ("88", "Blue Slinky"), ("89", "Yellow Slinky"), ("90", "Yellow Keychain"), ("91", "Green Keychain"), ("92", "Pink Keychain"), ("93", "Chimera Keychain"), ("94", "Pet Rock"), ("95", "Flower Clip"), ("96", "Red Headband"), ("97", "Orange Headband"), ("98", "Cool Glasses"), ("99", "Seashell Necklace"), ("100", "Gold Watch"), ("101", "\"Gold\" Watch"), ("102", "Fedora"), ("103", "Rabbit Foot"), ("104", "Teddy Bear"), ("105", "\"You Rock\" Cap"), ("106", "Pinwheel"), ("107", "Paint Brush"), ("108", "Cool Bottlecap"), ("109", "Kel'S Pet Rock"), ("120", "Tulip Hairstick"), ("121", "Gladiolus Hairband"), ("122", "Cactus Hairclip"), ("123", "Rose Hairclip")];

const OMORI: Spec = Spec {
    name: "OMORI",
    tab: "OMORI",
    accent: (150, 110, 200),
    kind: None,
    markers: &["/party/_stressEnergyCount", "/party/_blackLetters", "/actors/_data/@a"],
    subtitle: "file<n>.rpgsave in www\\save: Clams, each friend's level, heart and juice in Headspace and Faraway Town, and every snack, toy, weapon and charm you carry.",
    tiles: &[("Clams", "/party/_gold"), ("Steps", "/party/_steps")],
    sections: &[],
    lists: &[
        List { title: "Snacks, toys and key items", path: "/party/_items", shape: Shape::Map, names: OMORI_ITEMS, max: None },
        List { title: "Weapons", path: "/party/_weapons", shape: Shape::Map, names: OMORI_WEAPONS, max: None },
        List { title: "Charms", path: "/party/_armors", shape: Shape::Map, names: OMORI_CHARMS, max: None },
    ],
    tables: &[Table {
        title: "Friends",
        path: "/actors/_data/@a",
        name: &["/_actorId"],
        cols: &[("Level", "/_level"), ("Heart", "/_hp"), ("Juice", "/_mp")],
        only: &["1", "2", "3", "4", "8", "9", "10", "11"],
        names: &[("1", "OMORI"), ("2", "AUBREY"), ("3", "KEL"), ("4", "HERO"), ("8", "SUNNY (Faraway Town)"), ("9", "AUBREY (Faraway Town)"), ("10", "KEL (Faraway Town)"), ("11", "HERO (Faraway Town)")],
    }],
};

// ------------------------------------------------------------------ ENDER LILIES

const ENDER_LILIES: Spec = Spec {
    name: "ENDER LILIES: Quietus of the Knights",
    tab: "Lily",
    accent: (200, 205, 230),
    kind: Some("Unreal Engine save (SaveData)"),
    markers: &["/Player/Inventory", "/Player/SpiritEquip", "/GameMode/GameStats"],
    subtitle: "GameSlot_<n>.sav: Lily's level and blight, prayer heals, the spirit-upgrade materials and every spirit's level.",
    tiles: &[("Level", "/Player/Parameters/Level"), ("Blight (experience)", "/Player/Parameters/ExpPoint"), ("Prayer level", "/Player/Heals/HealPowerLevel")],
    sections: &[Section {
        title: "Lily",
        rows: &[
            ("Level", "/Player/Parameters/Level"), ("Blight (experience)", "/Player/Parameters/ExpPoint"),
            ("Prayer (heal) level", "/Player/Heals/HealPowerLevel"), ("Prayers left", "/Player/Heals/CurrHealCount"),
            ("Relic slots", "/Player/PassiveEquip/SlotCount"), ("Play time (seconds)", "/GameMode/GameStats/PlayTime"),
        ],
    }],
    lists: &[
        List {
            title: "Spirit upgrade materials",
            path: "/Player/Inventory/Currencies",
            shape: Shape::Map,
            names: &[
                ("ECurrencyType::Spirit_Lv1", "Stagnant Blight"),
                ("ECurrencyType::Spirit_Lv2", "Furious Blight"),
                ("ECurrencyType::Spirit_Lv3", "Ancient Souls"),
            ],
            max: Some(Max {
                ids: &["ECurrencyType::Spirit_Lv1", "ECurrencyType::Spirit_Lv2"],
                cap: 999,
                caps: &[],
                estimate: true,
                about: "Fills Stagnant and Furious Blight to 999, the most players report the game holds. Ancient Souls (found once each) are left alone.",
            }),
        },
        List { title: "Spirit levels", path: "/Player/Inventory/ItemSpiritLevels", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Shadow Warrior 2

const SW2: Spec = Spec {
    name: "Shadow Warrior 2",
    tab: "Lo Wang",
    accent: (214, 60, 48),
    kind: Some(crate::fmt::roadhog::KIND),
    markers: &["/PlayerTemplate/Stats/Money", "/PlayerTemplate/Stats/Level"],
    subtitle: "save_auto.bin and the other .bin saves in the game's save folder (each is a zip; the text inside is opened for you). Weapons and upgrades are in the Tree tab under PlayerTemplate.",
    tiles: &[
        ("Level", "/PlayerTemplate/Stats/Level"), ("Money", "/PlayerTemplate/Stats/Money"),
        ("Crystal points", "/PlayerTemplate/Stats/CrystalPoints"), ("Perk points", "/PlayerTemplate/Stats/PerksPoints"),
    ],
    sections: &[
        Section {
            title: "Lo Wang",
            rows: &[
                ("Level", "/PlayerTemplate/Stats/Level"), ("Experience", "/PlayerTemplate/Stats/XP"), ("Money", "/PlayerTemplate/Stats/Money"),
                ("Crystal points", "/PlayerTemplate/Stats/CrystalPoints"), ("Perk points", "/PlayerTemplate/Stats/PerksPoints"),
                ("Health", "/PlayerTemplate/Stats/HP"), ("Mana (chi)", "/PlayerTemplate/Stats/Mana"), ("Rage", "/PlayerTemplate/Stats/Rage"),
            ],
        },
        Section {
            title: "Ammunition",
            rows: &[
                ("Light bullets", "/PlayerTemplate/Stats/Ammo_Bullet_Light"), ("Medium bullets", "/PlayerTemplate/Stats/Ammo_Bullet_Medium"),
                ("Heavy bullets", "/PlayerTemplate/Stats/Ammo_Bullet_Heavy"), ("Shells", "/PlayerTemplate/Stats/Ammo_Shell"),
                ("Energy", "/PlayerTemplate/Stats/Ammo_Energy"), ("Plasma", "/PlayerTemplate/Stats/Ammo_Plasma"),
                ("Rockets", "/PlayerTemplate/Stats/Ammo_Rocket"), ("Grenades", "/PlayerTemplate/Stats/Ammo_Grenade"),
                ("Bolts", "/PlayerTemplate/Stats/Ammo_Bolt"), ("Arrows", "/PlayerTemplate/Stats/Ammo_Arrow"),
                ("Chainsaw fuel", "/PlayerTemplate/Stats/Ammo_Chainsaw"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Everspace 2

const ES2_NAMES: &[(&str, &str)] = &[("crafting_resource_basic", "Crafting Resource (basic)"), ("crafting_resource_common", "Crafting Resource (common)"), ("crystal_aetheum_common", "Crystal Aetheum (common)"), ("crystal_aetheum_uncommon", "Crystal Aetheum (uncommon)"), ("scrap_metal", "Scrap Metal"), ("ore_iron_common", "Ore Iron (common)"), ("cooling_unit", "Cooling Unit"), ("power_casing", "Power Casing"), ("ore_copper_common", "Ore Copper (common)"), ("device_reset_item", "Device Reset Item"), ("plasma", "Plasma"), ("wiring_kit", "Wiring Kit"), ("biomass", "Biomass"), ("ore_iron_uncommon", "Ore Iron (uncommon)"), ("crafting_resource_uncommon", "Crafting Resource (uncommon)"), ("ore_titanium_uncommon", "Ore Titanium (uncommon)"), ("heat_detector", "Heat Detector"), ("hadron_buffers", "Hadron Buffers"), ("charge_compensator", "Charge Compensator"), ("dark_matter", "Dark Matter"), ("crystal_culver_uncommon", "Crystal Culver (uncommon)"), ("neutron_membrane", "Neutron Membrane"), ("power_cell", "Power Cell"), ("supra_gel", "Supra Gel"), ("crystal_culver_rare", "Crystal Culver (rare)"), ("crystal_mallemite_uncommon", "Crystal Mallemite (uncommon)"), ("crystal_mallemite_rare", "Crystal Mallemite (rare)"), ("crystal_diode", "Crystal Diode"), ("ore_insidium_uncommon", "Ore Insidium (uncommon)"), ("ore_insidium_rare", "Ore Insidium (rare)"), ("ore_baromite_rare", "Ore Baromite (rare)"), ("crafting_resource_rare", "Crafting Resource (rare)"), ("particle_modulator", "Particle Modulator"), ("ore_clygon_rare", "Ore Clygon (rare)"), ("solar_particles", "Solar Particles"), ("power_unit_hx1", "Power Unit Hx1"), ("ore_titanium_rare", "Ore Titanium (rare)"), ("crafting_resource_superior", "Crafting Resource (superior)"), ("proton_capacitor", "Proton Capacitor"), ("quantum_conductor", "Quantum Conductor"), ("pulse_mutator", "Pulse Mutator"), ("signal_circulator", "Signal Circulator"), ("fusion_core", "Fusion Core"), ("ore_coruscate_superior", "Ore Coruscate (superior)"), ("crystal_phazium_superior", "Crystal Phazium (superior)"), ("crystal_phazium_rare", "Crystal Phazium (rare)"), ("ore_baromite_superior", "Ore Baromite (superior)"), ("dark_energy", "Dark Energy"), ("ore_glacit_superior", "Ore Glacit (superior)"), ("rift_resource", "Rift Resource"), ("ore_copper_uncommon", "Ore Copper (uncommon)"), ("bio_processor", "Bio Processor"), ("multiphase_propellant", "Multiphase Propellant"), ("echo_canceller", "Echo Canceller"), ("tachyon_emitter", "Tachyon Emitter"), ("resistor_array", "Resistor Array"), ("fission_controller", "Fission Controller"), ("gauss_receptor", "Gauss Receptor"), ("crafting_resource_ascendant", "Crafting Resource (ascendant)"), ("eshahar_fragments", "Eshahar Fragments"), ("ore_clygon_superior", "Ore Clygon (superior)"), ("resources_okkar", "Resources Okkar"), ("ore_leviathan_uncommon", "Ore Leviathan (uncommon)"), ("ore_leviathan_superior", "Ore Leviathan (superior)"), ("leviathan_attractant", "Leviathan Attractant"), ("crafting_resource_legendary", "Crafting Resource (legendary)"), ("organic_conduit", "Organic Conduit"), ("crystal_okkar1_rare", "Crystal Okkar1 (rare)"), ("okkar_component_1", "Okkar Component 1"), ("okkar_component_2", "Okkar Component 2"), ("ore_okkar1_rare", "Ore Okkar1 (rare)"), ("ore_okkar1_uncommon", "Ore Okkar1 (uncommon)"), ("crystal_okkar1_uncommon", "Crystal Okkar1 (uncommon)"), ("resources_okkar_special", "Resources Okkar Special"), ("crystal_okkar_refined", "Crystal Okkar Refined"), ("ancient_shielding_component", "Ancient Shielding Component")];

const ES2: Spec = Spec {
    name: "Everspace 2",
    tab: "Pilot",
    accent: (64, 160, 230),
    kind: Some("Unreal Engine save (PlayerData)"),
    markers: &["/credits", "/PlayerLevel", "/CraftingResourcesSaveState/Cargo"],
    subtitle: "ES2_<date>.sav in the game's SaveGames folder (the packed Unreal save is unpacked for you; big saves take a few seconds to open). Ships, items and missions are in the Tree tab.",
    tiles: &[("Credits", "/credits"), ("Level", "/PlayerLevel"), ("Experience", "/XP"), ("Device upgrade tokens", "/DeviceUpgradeTokens")],
    sections: &[Section {
        title: "Pilot",
        rows: &[
            ("Credits", "/credits"), ("Level", "/PlayerLevel"), ("Experience", "/XP"), ("Device upgrade tokens", "/DeviceUpgradeTokens"),
        ],
    }],
    lists: &[List {
        title: "Crafting resources",
        path: "/CraftingResourcesSaveState/Cargo",
        shape: Shape::Array { id_key: "ItemTemplateID", qty_keys: &["Amount"] },
        names: ES2_NAMES,
        max: Some(Max {
            ids: &["crafting_resource_basic", "crafting_resource_common", "crystal_aetheum_common", "crystal_aetheum_uncommon", "scrap_metal", "ore_iron_common", "ore_copper_common", "ore_iron_uncommon", "crafting_resource_uncommon", "ore_titanium_uncommon", "crystal_culver_uncommon", "crystal_culver_rare", "crystal_mallemite_uncommon", "crystal_mallemite_rare", "crystal_diode", "ore_insidium_uncommon", "ore_insidium_rare", "ore_baromite_rare", "crafting_resource_rare", "ore_clygon_rare", "ore_titanium_rare", "crafting_resource_superior", "ore_coruscate_superior", "crystal_phazium_superior", "crystal_phazium_rare", "ore_baromite_superior", "ore_glacit_superior", "ore_copper_uncommon", "crafting_resource_ascendant", "ore_clygon_superior", "crafting_resource_legendary"],
            cap: 99_999,
            caps: &[],
            estimate: true,
            about: "Raises the ores, crystals, scrap metal and basic crafting resources you already have to 99,999 (the game states no cap, so this is an estimate). Catalysts, components, rift and faction resources and story items are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Moonlighter

const MOONLIGHTER: Spec = Spec {
    name: "Moonlighter",
    tab: "Will",
    accent: (226, 160, 56),
    kind: None,
    markers: &["/gameSlot/willGold", "/gameSlot/willInventory"],
    subtitle: "The gameslot file in the game's save folder. Enchantment levels run 0 to 3. Gold is Will's gold; the banker's invested money is separate.",
    tiles: &[("Gold", "/gameSlot/willGold"), ("Days passed", "/gameSlot/daysPassed")],
    sections: &[Section {
        title: "Will and the shop",
        rows: &[
            ("Gold", "/gameSlot/willGold"), ("Days passed", "/gameSlot/daysPassed"), ("Money invested with the banker", "/gameSlot/bankerSaveInfo/moneyInvested"),
            ("Cultures defeated", "/gameSlot/culturesDefeated"), ("Equipped weapon set", "/gameSlot/equippedWeaponSet"),
        ],
    }],
    lists: &[
        List { title: "Backpack", path: "/gameSlot/willInventory", shape: Shape::Array { id_key: "name", qty_keys: &["quantity"] }, names: &[], max: None },
        List { title: "Equipped", path: "/gameSlot/willEquippedItems", shape: Shape::Array { id_key: "name", qty_keys: &["quantity"] }, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Wasteland 2

const WASTELAND2: Spec = Spec {
    name: "Wasteland 2",
    tab: "Rangers",
    accent: (196, 120, 48),
    kind: Some(crate::fmt::wasteland2::KIND),
    markers: &["/save/pcs/pc/0/xp"],
    subtitle: "<save name>.xml in a folder under Save Games (the .bin beside it is not needed). Character names are shown but not edited. Skills are in the Tree under each ranger's skillXps.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: "Rangers",
        path: "/save/pcs/pc",
        name: &["/portraitName"],
        only: &[],
        cols: &[
            ("Level", "/level"), ("XP", "/xp"), ("Health", "/curHp"), ("Attribute points", "/availableAttributePoints"),
            ("Skill points", "/availableSkillPoints"), ("Trait points", "/availableTraitPoints"), ("Kills", "/noOfKills"),
            ("Coordination", "/attributes/pair/0/value"), ("Luck", "/attributes/pair/1/value"), ("Awareness", "/attributes/pair/2/value"),
            ("Strength", "/attributes/pair/3/value"), ("Speed", "/attributes/pair/4/value"), ("Intelligence", "/attributes/pair/5/value"),
            ("Charisma", "/attributes/pair/6/value"),
        ],
        names: &[],
    }],
};

// ------------------------------------------------------------------ ATOM RPG

const ATOM: Spec = Spec {
    name: "ATOM RPG",
    tab: "Party",
    accent: (150, 130, 80),
    kind: None,
    markers: &["/0/class", "/0/stats/xp", "/0/stats/level"],
    subtitle: "Save_N_v10.as in the game's save folder (the game's own packing is opened and closed for you; the character file player.dat is shown). The party and the money are here; other entries are the levels. Rubles are the Money item in the player's inventory.",
    tiles: &[],
    sections: &[],
    lists: &[
        List { title: "Inventory (character 1)", path: "/0/inventory", shape: Shape::Array { id_key: "proto", qty_keys: &["count"] }, names: &[("Money", "Rubles")], max: None },
        List { title: "Inventory (character 2)", path: "/1/inventory", shape: Shape::Array { id_key: "proto", qty_keys: &["count"] }, names: &[("Money", "Rubles")], max: None },
        List { title: "Inventory (character 3)", path: "/2/inventory", shape: Shape::Array { id_key: "proto", qty_keys: &["count"] }, names: &[("Money", "Rubles")], max: None },
        List { title: "Inventory (character 4)", path: "/3/inventory", shape: Shape::Array { id_key: "proto", qty_keys: &["count"] }, names: &[("Money", "Rubles")], max: None },
        List { title: "Inventory (character 5)", path: "/4/inventory", shape: Shape::Array { id_key: "proto", qty_keys: &["count"] }, names: &[("Money", "Rubles")], max: None },
        List { title: "Inventory (character 6)", path: "/5/inventory", shape: Shape::Array { id_key: "proto", qty_keys: &["count"] }, names: &[("Money", "Rubles")], max: None },
    ],
    tables: &[Table {
        title: "Party",
        path: "",
        name: &["/character"],
        only: &[],
        cols: &[
            ("Level", "/stats/level"), ("XP", "/stats/xp"), ("Strength", "/stats/strength"), ("Perception", "/stats/perception"),
            ("Endurance", "/stats/endurance"), ("Charisma", "/stats/charisma"), ("Intelligence", "/stats/intelligence"),
            ("Agility", "/stats/agility"), ("Luck", "/stats/luck"), ("Skill points", "/stats/freeSkillPoints"),
            ("Stat points", "/stats/freeStatPoints"), ("Perk points", "/stats/freeSpecPoints"),
            ("Small Guns", "/stats/skills/SmallGuns"), ("Big Guns", "/stats/skills/BigGuns"), ("Automatic Firearms", "/stats/skills/AutomaticFirearms"), ("Unarmed", "/stats/skills/Unarmed"), ("Melee Weapons", "/stats/skills/MeleeWeapons"), ("Throwing", "/stats/skills/Throwing"), ("First Aid", "/stats/skills/FirstAid"), ("Crafting", "/stats/skills/Crafting"), ("Sneak", "/stats/skills/Sneak"), ("Lockpick", "/stats/skills/Lockpick"), ("Steal", "/stats/skills/Steal"), ("Alertness", "/stats/skills/Alertness"), ("Science", "/stats/skills/Science"), ("Speech", "/stats/skills/Speech"), ("Barter", "/stats/skills/Barter"), ("Gambling", "/stats/skills/Gambling"), ("Outdoorsman", "/stats/skills/Outdoorsman"),
        ],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Mortal Shell

const MORTAL_SHELL: Spec = Spec {
    name: "Mortal Shell",
    tab: "Fallgrim",
    accent: (150, 140, 120),
    kind: Some("Unreal Engine save (ServerSaveGame_C)"),
    markers: &["/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/BoneShards"],
    subtitle: "GPData_<slot>.sav in Documents\\My Games\\MortalShell\\Dungeonhaven\\Saved\\SaveGames. Tar is stored as BoneShards; the Storm Mode run keeps its own copy. Inventory item names are stored as game text and are not shown.",
    tiles: &[("Tar", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/BoneShards"), ("Max health", "/SaveGameFile/PlayerMaxHealth_7_329129454F2E3E21AAC8A2A0554AEBE1"), ("Deaths", "/SaveGameFile/TotalPlayerDeaths_19_3F1ADF634C3F62BBD1BEC4B7DF4B0301")],
    sections: &[
        Section {
            title: "Tar and shell points",
            rows: &[
            ("Tar", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/BoneShards"),
            ("Tar gained in total", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/BoneShardsEverGained"),
            ("Tar lost on death", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/BoneShardsLost"),
            ("Balanced shell points", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/BalancedShellPoints"),
            ("Heavy shell points", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/HeavyShellPoints"),
            ("Ninja shell points", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/NinjaShellPoints"),
            ("Hadern shell points", "/SaveGameFile/NamedInts_82_CB032F6F45A54ED7B3582AB3AF6AC3F2/HadernShellPoints"),
            ],
        },
        Section {
            title: "Character",
            rows: &[
            ("Max health", "/SaveGameFile/PlayerMaxHealth_7_329129454F2E3E21AAC8A2A0554AEBE1"),
            ("Deaths", "/SaveGameFile/TotalPlayerDeaths_19_3F1ADF634C3F62BBD1BEC4B7DF4B0301"),
            ("New Game+ level", "/SaveGameFile/NewGamePlusLevel_98_0DCE90C549CCEE82E941888FFD7D0984"),
            ("Death statue uses", "/SaveGameFile/PlayerDeathStatueUsedCounter_67_AB6D290345536982E2EB93B982821F1F"),
            ],
        },
        Section {
            title: "Storm Mode run",
            rows: &[
            ("Tar", "/StormModeSaveFile/NamedInts_StormMode_31_CF554F674F568B81370C968E9EF1459F/BoneShards"),
            ("Tar gained in total", "/StormModeSaveFile/NamedInts_StormMode_31_CF554F674F568B81370C968E9EF1459F/BoneShardsEverGained"),
            ("Glimpses", "/StormModeSaveFile/NamedInts_StormMode_31_CF554F674F568B81370C968E9EF1459F/StormModeGlimpses"),
            ("Glimpses gained in total", "/StormModeSaveFile/NamedInts_StormMode_31_CF554F674F568B81370C968E9EF1459F/StormModeGlimpsesEverGained"),
            ("Max health", "/StormModeSaveFile/MaxHealth_89_747BF7B7416D17384FCE89AC1280800B"),
            ("Deaths", "/StormModeSaveFile/NamedInts_StormMode_31_CF554F674F568B81370C968E9EF1459F/StormMode_TotalDeaths"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Mortal Shell II

const MORTAL_SHELL_2: Spec = Spec {
    name: "Mortal Shell II",
    tab: "Harbinger",
    accent: (176, 120, 96),
    kind: Some("Unreal Engine save (SpartaSaveGame)"),
    markers: &["/SaveObjects/BP_SpartaPlayerSaveObject_C/CumulativeGold", "/SaveObjects/BP_SpartaPlayerSaveObject_C/DropItems"],
    subtitle: "WorldState_<n>.sav in %LOCALAPPDATA%\\MortalShell2\\Saved\\SaveGames. Gold, Gloom and Glimpses held are stored with the items; the lifetime totals are kept separately, so change both together.",
    tiles: &[("Gold", "/SaveObjects/BP_SpartaPlayerSaveObject_C/DropItems/~1Game~1Sparta~1Items~1Pickups~1ID_Gold.ID_Gold_C"), ("Gloom", "/SaveObjects/BP_SpartaPlayerSaveObject_C/DropItems/~1Game~1Sparta~1Items~1Pickups~1ID_Gloom.ID_Gloom_C"), ("Glimpses", "/SaveObjects/BP_SpartaPlayerSaveObject_C/DropItems/~1Game~1Sparta~1Items~1Pickups~1ID_Glimpses.ID_Glimpses_C")],
    sections: &[Section {
        title: "Totals",
        rows: &[
            ("Gold earned in total", "/SaveObjects/BP_SpartaPlayerSaveObject_C/CumulativeGold"),
            ("Gloom earned in total", "/SaveObjects/BP_SpartaPlayerSaveObject_C/CumulativeGloom"),
            ("Glimpses earned in total", "/SaveObjects/BP_SpartaPlayerSaveObject_C/CumulativeGlimpses"),
            ("Play time (seconds)", "/SaveObjects/BP_SpartaPlayerSaveObject_C/TotalPlayTimeSeconds"),
            ("Prologue finished", "/SaveObjects/BP_SpartaPlayerSaveObject_C/FinishedPrologue"),
        ],
    },
    Section {
        title: "Unlocked shells, weapons and seals",
        rows: &[
            ("Shell: Eredrim", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Eredrim"),
            ("Shell: Genessa", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Genessa"),
            ("Shell: Gragu", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Gragu"),
            ("Shell: Harros", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Harros"),
            ("Shell: Knight Lady", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.KnightLady"),
            ("Shell: Necrophage", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Necrophage.Default"),
            ("Shell: Smert", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Smert"),
            ("Shell: Solomon", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Solomon"),
            ("Shell: Thorn", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Thorn"),
            ("Shell: Tiel", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/CharacterId.Player.Shell.Tiel"),
            ("Weapon: Axatana", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Axatana"),
            ("Weapon: Ballistazooka", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Ballistazooka"),
            ("Weapon: Battle Axe", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.BattleAxe"),
            ("Weapon: Black Needle", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.BlackNeedle"),
            ("Weapon: Clockwork Scythe", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.ClockworkScythe"),
            ("Weapon: Crossbow", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Crossbow"),
            ("Weapon: Cursed Child", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.CursedChild"),
            ("Weapon: Dagger Axe", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.DaggerAxe"),
            ("Weapon: Haderns Sword", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.HadernsSword"),
            ("Weapon: Heavy Hammer", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.HeavyHammer"),
            ("Lute: Simple", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Lute.Simple"),
            ("Weapon: Machine Gun", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.MachineGun"),
            ("Weapon: Martyrs Blade", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.MartyrsBlade"),
            ("Weapon: Nail Shotgun", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.NailShotgun"),
            ("Weapon: Parasite Gun", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.ParasiteGun"),
            ("Seal: Default", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Seal.Default"),
            ("Seal: Infinite", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Seal.Infinite"),
            ("Seal: Slayer", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Seal.Slayer"),
            ("Seal: Stone", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Seal.Stone"),
            ("Weapon: Trebuchaxe", "/SaveObjects/BP_SpartaPlayerSaveObject_C/EquipmentUnlockState/Weapon.Trebuchaxe"),
        ],
    }],
    lists: &[List {
        title: "Items",
        path: "/SaveObjects/BP_SpartaPlayerSaveObject_C/DropItems",
        shape: Shape::Map,
        names: &[
            ("/Game/Sparta/Items/Pickups/ID_Gloom.ID_Gloom_C", "Gloom"),
            ("/Game/Sparta/Items/Pickups/ID_Gold.ID_Gold_C", "Gold"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Moonshine_Inferior.ID_PP_Moonshine_Inferior_C", "Inferior Moonshine"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_Tarforge_ForgeMode_Weapon.ID_Tarforge_ForgeMode_Weapon_C", "Tarforge: enhance weapons"),
            ("/Game/Sparta/Items/Pickups/ID_Glimpses.ID_Glimpses_C", "Glimpses"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Effigy_Burned.ID_PP_Effigy_Burned_C", "Upgrade: Effigy Burned"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_01_Ventrium.ID_TFM_01_Ventrium_C", "Ventrium"),
            ("/Game/Sparta/Items/Pickups/Core/ID_Ovum.ID_Ovum_C", "Ovum"),
            ("/Game/Sparta/Items/Map/ID_MapUnlock.ID_MapUnlock_C", "Map"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_03_Dorsalite.ID_TFM_03_Dorsalite_C", "Dorsalite"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_MartyrAsh.ID_PP_MartyrAsh_C", "Upgrade: Martyr Ash"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Bloodseed.ID_PP_Bloodseed_C", "Upgrade: Bloodseed"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Gear_Strange.ID_PP_Gear_Strange_C", "Upgrade: Gear Strange"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_HealingAmount.ID_PP_HealingAmount_C", "Upgrade: Healing Amount"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_HealingCharge.ID_PP_HealingCharge_C", "Upgrade: Healing Charge"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Mango.ID_PP_Mango_C", "Upgrade: Mango"),
            ("/Game/Sparta/Items/Pickups/ID_BagheadMoonshine.ID_BagheadMoonshine_C", "Baghead Moonshine"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Tarspore.ID_PP_Tarspore_C", "Upgrade: Tarspore"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_ParadoxicalScripture.ID_PP_ParadoxicalScripture_C", "Upgrade: Paradoxical Scripture"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_UniqueScripture.ID_PP_UniqueScripture_C", "Upgrade: Unique Scripture"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Mushroom_Weltcap.ID_PP_Mushroom_Weltcap_C", "Upgrade: Mushroom Weltcap"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Trollweed.ID_PP_Trollweed_C", "Upgrade: Trollweed"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Sheep.ID_PP_Sheep_C", "Upgrade: Sheep"),
            ("/Game/Sparta/Items/Pickups/PermProgression/ID/ID_PP_Grisha.ID_PP_Grisha_C", "Upgrade: Grisha"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_02_Laterite.ID_TFM_02_Laterite_C", "Laterite"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_04_Thoracium.ID_TFM_04_Thoracium_C", "Thoracium"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_05_Thoracium_Prime.ID_TFM_05_Thoracium_Prime_C", "Thoracium Prime"),
            ("/Game/Sparta/Items/Pickups/Tardust/ID_Tarcore.ID_Tarcore_C", "Tarcore"),
            ("/Game/Sparta/Items/Pickups/ID_Common_Old_Artifact.ID_Common_Old_Artifact_C", "Common Old Artifact"),
            ("/Game/Sparta/Items/Pickups/ID_Rare_Old_Artifact.ID_Rare_Old_Artifact_C", "Rare Old Artifact"),
            ("/Game/Sparta/Items/Pickups/ID_Ancient_Artifact.ID_Ancient_Artifact_C", "Ancient Artifact"),
            ("/Game/Sparta/Items/Pickups/ID_Epic_Old_Artifact.ID_Epic_Old_Artifact_C", "Epic Old Artifact"),
            ("/Game/Sparta/Items/Pickups/ID_Artifact_MarshPearl.ID_Artifact_MarshPearl_C", "Artifact Marsh Pearl"),
            ("/Game/Sparta/Items/Pickups/ID_Legendary_Old_Artifact.ID_Legendary_Old_Artifact_C", "Legendary Old Artifact"),
            ("/Game/Sparta/Items/Pickups/ID_Specimen.ID_Specimen_C", "Specimen"),
            ("/Game/Sparta/Items/Pickups/ID_MushroomVillageKey.ID_MushroomVillageKey_C", "Mushroom Village Key"),
            ("/Game/Sparta/Items/Pickups/ID_MissingLever.ID_MissingLever_C", "Missing Lever"),
            ("/Game/Sparta/Items/Pickups/ID_MissingCannonBall.ID_MissingCannonBall_C", "Missing Cannon Ball"),
            ("/Game/Sparta/Items/Pickups/ID_GraveyardCryptKey.ID_GraveyardCryptKey_C", "Graveyard Crypt Key"),
            ("/Game/Sparta/Items/Pickups/ID_GrishaCageKey.ID_GrishaCageKey_C", "Grisha Cage Key"),
            ("/Game/Sparta/Items/Pickups/ID_MemoryTiel_TheContract_Key.ID_MemoryTiel_TheContract_Key_C", "Memory Tiel The Contract Key"),
            ("/Game/Sparta/Items/Pickups/ID_KS_GenessaIncenseBurner.ID_KS_GenessaIncenseBurner_C", "KS Genessa Incense Burner"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_Tarforge_ForgeMode_Sidearm.ID_Tarforge_ForgeMode_Sidearm_C", "Tarforge: enhance sidearms"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_Tarforge_SmeltMode.ID_Tarforge_SmeltMode_C", "Tarforge: smelting"),
            ("/Game/Sparta/Items/Pickups/Tarforge/ID_Tarforge_TarstoneMode.ID_Tarforge_TarstoneMode_C", "Tarforge: Tarstones"),
            ("/Game/Sparta/Items/Pickups/ID_Map_Weapon_ObsidianHammer.ID_Map_Weapon_ObsidianHammer_C", "Map Weapon Obsidian Hammer"),
            ("/Game/Sparta/Items/Pickups/ID_Map_Weapon_GreatMartyrSword.ID_Map_Weapon_GreatMartyrSword_C", "Map Weapon Great Martyr Sword"),
            ("/Game/Sparta/Items/Pickups/ID_Map_Weapon_AxeDagger.ID_Map_Weapon_AxeDagger_C", "Map Weapon Axe Dagger"),
            ("/Game/Sparta/Items/Pickups/ID_Map_Weapon_BlackNeedle.ID_Map_Weapon_BlackNeedle_C", "Map Weapon Black Needle"),
            ("/Game/Sparta/Items/Pickups/ID_GrimoriumObscurum.ID_GrimoriumObscurum_C", "Grimorium Obscurum"),
            ("/Game/Sparta/Items/Pickups/ID_FastTravel.ID_FastTravel_C", "Fast travel"),
            ("/Game/Sparta/Items/Pickups/ID_BlackMarrowKey.ID_BlackMarrowKey_C", "Black Marrow Key"),
            ("/Game/Sparta/Items/Pickups/ID_EndlessCore.ID_EndlessCore_C", "Endless Core"),
            ("/Game/Sparta/Items/Pickups/ID_VlasCageKey.ID_VlasCageKey_C", "Vlas Cage Key"),
            ("/Game/Sparta/Items/Pickups/ID_Artifact_FinalBoss.ID_Artifact_FinalBoss_C", "Artifact Final Boss"),
            ("/Game/Sparta/Items/Pickups/ID_Artifact_TwinSesters.ID_Artifact_TwinSesters_C", "Artifact Twin Sesters"),
            ("/Game/Sparta/Items/Pickups/ID_Artifact_Monolith.ID_Artifact_Monolith_C", "Artifact Monolith"),
            ("/Game/Sparta/Items/Pickups/ID_Artifact_Offspring.ID_Artifact_Offspring_C", "Artifact Offspring"),
            ("/Game/Sparta/Items/Pickups/ID_Artifact_ParasiteGolem.ID_Artifact_ParasiteGolem_C", "Artifact Parasite Golem"),
            ("/Game/Sparta/Items/Pickups/ID_MethersFlesh.ID_MethersFlesh_C", "Methers Flesh"),
            ("/Game/Sparta/Items/Pickups/ID_SacredGland_Fire.ID_SacredGland_Fire_C", "Sacred Gland Fire"),
            ("/Game/Sparta/Items/Pickups/ID_SacredGland_Cathedral.ID_SacredGland_Cathedral_C", "Sacred Gland Cathedral"),
            ("/Game/Sparta/Items/Pickups/ID_SacredGland_Snow.ID_SacredGland_Snow_C", "Sacred Gland Snow"),
            ("/Game/Sparta/Items/Pickups/ID_ThestusFlame.ID_ThestusFlame_C", "Thestus Flame"),
            ("/Game/Sparta/Items/Pickups/ID_IceCrown.ID_IceCrown_C", "Ice Crown"),
            ("/Game/Sparta/Items/Pickups/ID_ElectricConduit.ID_ElectricConduit_C", "Electric Conduit"),
            ("/Game/Sparta/Items/Pickups/ID_VratkoFeetPic_01.ID_VratkoFeetPic_01_C", "Vratko Feet Pic 01"),
            ("/Game/Sparta/Items/Pickups/ID_VratkoFeetPic_02.ID_VratkoFeetPic_02_C", "Vratko Feet Pic 02"),
            ("/Game/Sparta/Items/Pickups/ID_VratkoFeetPic_03.ID_VratkoFeetPic_03_C", "Vratko Feet Pic 03"),
            ("/Game/Sparta/Items/Pickups/ID_VratkoFeetPic_04.ID_VratkoFeetPic_04_C", "Vratko Feet Pic 04"),
            ("/Game/Sparta/Items/Pickups/ID_VratkoFeetPic_05.ID_VratkoFeetPic_05_C", "Vratko Feet Pic 05"),
            ("/Game/Sparta/Items/Pickups/ID_VratkoFeetPic_06.ID_VratkoFeetPic_06_C", "Vratko Feet Pic 06"),
        ],
        max: Some(Max {
            ids: &[
                "/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_01_Ventrium.ID_TFM_01_Ventrium_C",
                "/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_03_Dorsalite.ID_TFM_03_Dorsalite_C",
                "/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_02_Laterite.ID_TFM_02_Laterite_C",
                "/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_04_Thoracium.ID_TFM_04_Thoracium_C",
                "/Game/Sparta/Items/Pickups/Tarforge/ID_TFM_05_Thoracium_Prime.ID_TFM_05_Thoracium_Prime_C",
            ],
            cap: 99,
            caps: &[],
            estimate: true,
            about: "Raises the Tarforge smithing materials (Ventrium, Laterite, Dorsalite, Thoracium and Thoracium Prime) you already have. Keys, artifacts, tools and upgrade items are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ theHunter: Call of the Wild

const COTW: Spec = Spec {
    name: "theHunter: Call of the Wild",
    tab: "Hunter",
    accent: (200, 150, 60),
    kind: Some(crate::fmt::adf::KIND),
    markers: &["/Hunter/Cash"],
    subtitle: "thp_player_profile_adf in your Saves folder (each slot folder keeps its own copy). Close the game first; skill points already spent are stored per skill tree and are not shown.",
    tiles: &[("Cash", "/Hunter/Cash"), ("Level", "/Hunter/Level"), ("XP", "/Hunter/XP")],
    sections: &[Section { title: "Points", rows: &[("Skill points", "/Hunter/Skill points"), ("Perk points", "/Hunter/Perk points")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ BeamNG.drive

const BEAMNG: Spec = Spec {
    name: "BeamNG.drive",
    tab: "Career",
    accent: (240, 120, 30),
    kind: None,
    markers: &["/money/value", "/beamXP/value"],
    subtitle: "career/playerAttributes.json inside a career save slot's newest autosave folder (the game loads the newest of its rotating autosaves, so edit that one). Each branch (the career's skill lines) keeps its own value, shown in the Tree tab.",
    tiles: &[("Money", "/money/value"), ("BeamXP", "/beamXP/value")],
    sections: &[],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ The Forest

const FOREST: Spec = Spec {
    name: "The Forest",
    tab: "Survivor",
    accent: (90, 140, 70),
    kind: Some(crate::fmt::forest::KIND),
    markers: &["/Inventory"],
    subtitle: "The __RESUME__ file of a single-player or multiplayer slot. The inventory holds what the player carries; the game's own limit for each item is stored with it (Max amount, -1 for no limit).",
    tiles: &[],
    sections: &[],
    lists: &[List {
        title: "Inventory",
        path: "/Inventory",
        shape: Shape::Records { qty_key: "Amount" },
        names: &[],
        max: Some(Max {
            ids: &["Rock", "Stick", "Skull", "Rope", "Bone", "Lizard Skin", "Deer Skin", "Boar Skin", "Raccoon Skin", "Rabbit Fur", "Circuit Board"],
            cap: 10,
            caps: &[("Rock", 5), ("Skull", 4), ("Rope", 4), ("Bone", 15), ("Circuit Board", 5)],
            estimate: false,
            about: "Raises rocks, sticks, skulls, rope, bones, skins, rabbit fur and circuit boards to the limit the game stores for each in the save. Items with no limit (cloth, leaves, feathers, teeth, sap, tape), tools, weapons, food and story items are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Yakuza 3, 4 and 5 Remastered

const YAKUZA3: Spec = Spec {
    name: "Yakuza 3 Remastered",
    tab: "Kiryu",
    accent: (200, 150, 40),
    kind: Some(crate::fmt::yakuza_old::KIND_Y3),
    markers: &["/Kiryu/Money"],
    subtitle: "savegameN.sav in the Steam remote folder. The save has no checksum; close the game first.",
    tiles: &[("Money", "/Kiryu/Money"), ("Club money", "/Kiryu/Club money"), ("Experience in hand", "/Kiryu/Experience in hand")],
    sections: &[Section {
        title: "Kiryu",
        rows: &[("Money", "/Kiryu/Money"), ("Club money", "/Kiryu/Club money"), ("Total experience", "/Kiryu/Total experience"), ("Experience in hand", "/Kiryu/Experience in hand")],
    }],
    lists: &[],
    tables: &[],
};

const YAKUZA4: Spec = Spec {
    name: "Yakuza 4 Remastered",
    tab: "Characters",
    accent: (200, 150, 40),
    kind: Some(crate::fmt::yakuza_old::KIND_Y4),
    markers: &["/Kiryu/Money"],
    subtitle: "savegameN.sav in the Steam remote folder. Each of the four playable characters keeps their own money, experience and skill orbs. The save has no checksum; close the game first.",
    tiles: &[("Akiyama's money", "/Akiyama/Money"), ("Saejima's money", "/Saejima/Money"), ("Tanimura's money", "/Tanimura/Money"), ("Kiryu's money", "/Kiryu/Money")],
    sections: &[
        Section { title: "Akiyama", rows: &[("Money", "/Akiyama/Money"), ("Experience", "/Akiyama/Experience"), ("Experience to next level", "/Akiyama/Experience to next level"), ("Skill orbs", "/Akiyama/Skill orbs")] },
        Section { title: "Saejima", rows: &[("Money", "/Saejima/Money"), ("Experience", "/Saejima/Experience"), ("Experience to next level", "/Saejima/Experience to next level"), ("Skill orbs", "/Saejima/Skill orbs")] },
        Section { title: "Tanimura", rows: &[("Money", "/Tanimura/Money"), ("Experience", "/Tanimura/Experience"), ("Experience to next level", "/Tanimura/Experience to next level"), ("Skill orbs", "/Tanimura/Skill orbs")] },
        Section { title: "Kiryu", rows: &[("Money", "/Kiryu/Money"), ("Experience", "/Kiryu/Experience"), ("Experience to next level", "/Kiryu/Experience to next level"), ("Skill orbs", "/Kiryu/Skill orbs")] },
        Section { title: "Shared", rows: &[("Eco points", "/Shared/Eco points")] },
    ],
    lists: &[],
    tables: &[],
};

const YAKUZA5: Spec = Spec {
    name: "Yakuza 5 Remastered",
    tab: "Characters",
    accent: (200, 150, 40),
    kind: Some(crate::fmt::yakuza_old::KIND_Y5),
    markers: &["/Haruka/Money"],
    subtitle: "SAVEGAMEN.sav in the Steam remote folder. Each of the five playable characters keeps their own money, experience and skill orbs. The save has no checksum; close the game first.",
    tiles: &[("Kiryu's money", "/Kiryu/Money"), ("Saejima's money", "/Saejima/Money"), ("Akiyama's money", "/Akiyama/Money"), ("Shinada's money", "/Shinada/Money"), ("Haruka's money", "/Haruka/Money")],
    sections: &[
        Section { title: "Kiryu", rows: &[("Money", "/Kiryu/Money"), ("Experience", "/Kiryu/Experience"), ("Experience to next level", "/Kiryu/Experience to next level"), ("Skill orbs", "/Kiryu/Skill orbs")] },
        Section { title: "Saejima", rows: &[("Money", "/Saejima/Money"), ("Experience", "/Saejima/Experience"), ("Experience to next level", "/Saejima/Experience to next level"), ("Skill orbs", "/Saejima/Skill orbs")] },
        Section { title: "Akiyama", rows: &[("Money", "/Akiyama/Money"), ("Experience", "/Akiyama/Experience"), ("Experience to next level", "/Akiyama/Experience to next level"), ("Skill orbs", "/Akiyama/Skill orbs")] },
        Section { title: "Shinada", rows: &[("Money", "/Shinada/Money"), ("Experience", "/Shinada/Experience"), ("Experience to next level", "/Shinada/Experience to next level"), ("Skill orbs", "/Shinada/Skill orbs")] },
        Section { title: "Haruka", rows: &[("Money", "/Haruka/Money"), ("Experience", "/Haruka/Experience"), ("Experience to next level", "/Haruka/Experience to next level"), ("Skill orbs", "/Haruka/Skill orbs")] }
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Like a Dragon: Ishin!

const ISHIN: Spec = Spec {
    name: "Like a Dragon: Ishin!",
    tab: "Items",
    accent: (190, 40, 50),
    kind: Some(crate::fmt::ishin::KIND),
    markers: &["/Materials", "/Valuables"],
    subtitle: "data.sav in %APPDATA%\\Sega\\LikeADragonIshin. Food, seafood and crafting materials stack to 999; Virtue buys inventory upgrades at shrines. The wrapper's checksum is rewritten on save. Ryo isn't mapped yet (it stays in the Tree).",
    tiles: &[("Virtue", "/Valuables/Virtue"), ("Wooden tags", "/Valuables/Wooden Tags"), ("Prize tickets", "/Valuables/Prize Tickets")],
    sections: &[
        Section { title: "Valuables", rows: &[("Virtue", "/Valuables/Virtue"), ("Wooden tags", "/Valuables/Wooden Tags"), ("Mahjong sticks", "/Valuables/Mahjong Sticks"), ("Shogi points", "/Valuables/Shogi Points"), ("Prize tickets", "/Valuables/Prize Tickets")] },
        Section { title: "Style experience", rows: &[("Swordsman", "/Style experience/Swordsman"), ("Gunman", "/Style experience/Gunman"), ("Wild Dancer", "/Style experience/Wild Dancer"), ("Brawler", "/Style experience/Brawler")] },
        Section { title: "Orbs", rows: &[("Gunman", "/Orbs/Gunman"), ("Wild Dancer", "/Orbs/Wild Dancer"), ("Brawler", "/Orbs/Brawler"), ("Training", "/Orbs/Training")] },
    ],
    lists: &[
        List {
            title: "Crafting materials",
            path: "/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: crate::fmt::ishin::MATERIAL_NAMES,
                cap: 999,
                caps: &[],
                estimate: false,
                about: "Raises every crafting and upgrade material you already have to 999, the game's stack limit. Food, seafood, ammunition and the valuables are left alone.",
            }),
        },
        List { title: "Food", path: "/Food", shape: Shape::Map, names: &[], max: None },
        List { title: "Seafood", path: "/Seafood", shape: Shape::Map, names: &[], max: None },
        List { title: "Ammunition", path: "/Ammo", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Salt and Sanctuary

const SALT: Spec = Spec {
    name: "Salt and Sanctuary",
    tab: "Character",
    accent: (196, 180, 150),
    kind: Some(crate::fmt::saltsanc::KIND),
    markers: &["/Character/Salt", "/Character/Gold"],
    subtitle: "datN.slv in the game's savedata folder (one file per character). Salt and gold are what the character carries. Items and the skill tree are not edited yet.",
    tiles: &[("Salt", "/Character/Salt"), ("Gold", "/Character/Gold"), ("Level", "/Character/Level")],
    sections: &[Section {
        title: "Character",
        rows: &[
            ("Level", "/Character/Level"), ("Salt", "/Character/Salt"), ("Gold", "/Character/Gold"), ("Kills", "/Character/Kills"),
            ("Deaths", "/Character/Deaths"), ("Sanctuary rests", "/Character/Sanctuary rests"), ("Corruption", "/Character/Corruption"),
            ("Playthrough (0 = first)", "/Character/Playthrough"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Drakensang

const DRAKENSANG: Spec = Spec {
    name: "Drakensang",
    tab: "Heroes",
    accent: (176, 120, 70),
    kind: Some("SQLite database"),
    markers: &["/_Instance_PC/0/Stufe", "/_Instance_PC/0/UpgradeXP"],
    subtitle: "savegame.dsa in a savegame_N folder (a SQLite database; the screenshot and .nfo beside it are not needed). Heroes with a party slot of 0 or more are in your party; the other rows are the characters in cutscenes and other places. Adventure points to spend are in \"Spendable AP\". Items and money are in the Tree.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: "Heroes",
        path: "/_Instance_PC",
        name: &["/Name"],
        only: &[],
        cols: &[
            ("Party slot", "/PartySlotId"), ("Level", "/Stufe"), ("AP earned", "/XP"), ("Spendable AP", "/UpgradeXP"),
            ("Life", "/LE"), ("Endurance", "/AU"), ("Astral energy", "/AE"),
            ("Courage", "/MU"), ("Cleverness", "/KL"), ("Intuition", "/IN"), ("Charisma", "/CH"),
            ("Dexterity", "/FF"), ("Agility", "/GE"), ("Constitution", "/KO"), ("Strength", "/KK"),
            ("Armbrust", "/TaArmbrust"), ("Dolche", "/TaDolche"), ("Fechtwaffen", "/TaFechtwaffen"), ("Hiebwaffen", "/TaHiebwaffen"), ("Saebel", "/TaSaebel"), ("Schwerter", "/TaSchwerter"), ("Speere", "/TaSpeere"), ("Staebe", "/TaStaebe"), ("Zw Hiebwaffen", "/TaZwHiebwaffen"), ("Zw Schwerter", "/TaZwSchwerter"), ("Raufen", "/TaRaufen"), ("Bogen", "/TaBogen"), ("Wurfwaffen", "/TaWurfwaffen"), ("Schleichen", "/TaSchleichen"), ("Selbstbeherrschung", "/TaSelbstbeherrschung"), ("Sinnenschaerfe", "/TaSinnenschaerfe"), ("Taschendiebstahl", "/TaTaschendiebstahl"), ("Zwergennase", "/TaZwergennase"), ("Fallenstellen", "/TaFallenstellen"), ("Pflanzenkunde", "/TaPflanzenkunde"), ("Tierkunde", "/TaTierkunde"), ("Wildnisleben", "/TaWildnisleben"), ("Magiekunde", "/TaMagiekunde"), ("Heilkunde Wunden", "/TaHeilkundeWunden"), ("Heilkunde Gift", "/TaHeilkundeGift"), ("Gassenwissen", "/TaGassenwissen"), ("Alchimie", "/TaAlchimie"), ("Bogenbau", "/TaBogenbau"), ("Schmieden", "/TaSchmieden"), ("Fallen Entschaerfen", "/TaFallenEntschaerfen"), ("Schloesser", "/TaSchloesser"), ("Feilschen", "/TaFeilschen"), ("Etikette", "/TaEtikette"), ("Einschuechtern", "/TaEinschuechtern"), ("Betoeren", "/TaBetoeren"), ("Ueberreden", "/TaUeberreden"), ("Menschenkenntnis", "/TaMenschenkenntnis"), ("rget Size", "/TargetSize"), ("rget Entity Id", "/TargetEntityId"),
        ],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Alpha Protocol

const ALPHA: Spec = Spec {
    name: "Alpha Protocol",
    tab: "Agent",
    accent: (170, 150, 110),
    kind: Some(crate::fmt::alphaprotocol::KIND),
    markers: &["/Agent/Money", "/Agent/PlayerLevel"],
    subtitle: "A checkpoint .sav from Documents\\Alpha Protocol\\Checkpoints. Money, experience, skill points, skill ranks, ammunition and gadgets.",
    tiles: &[("Money", "/Agent/Money"), ("Level", "/Agent/PlayerLevel"), ("Skill points", "/Agent/AdvancementPoints")],
    sections: &[Section {
        title: "Agent",
        rows: &[
            ("Level", "/Agent/PlayerLevel"), ("Experience", "/Agent/ExperiencePoints"), ("Experience (total)", "/Agent/ExperiencePointsTotal"),
            ("Skill points to spend", "/Agent/AdvancementPoints"), ("Skill points (total)", "/Agent/AdvancementPointsTotal"), ("Money", "/Agent/Money"),
        ],
    }],
    lists: &[
        List { title: "Skills", path: "/Skills", shape: Shape::Map, names: &[], max: None },
        List { title: "Ammunition", path: "/Ammunition", shape: Shape::Map, names: &[], max: None },
        List { title: "Gadgets (count)", path: "/Gadgets", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Tyranny

const TYRANNY: Spec = Spec {
    name: "Tyranny",
    tab: "Party",
    accent: (190, 90, 60),
    kind: Some(crate::fmt::obsidian::KIND),
    markers: &["/Party/Money", "/Characters"],
    subtitle: "A .savegame from Saved Games\\Tyranny (a zip; its MobileObjects.save is edited, the rest is kept). Money, camping supplies, and each character's experience, attributes, skill points and skills.",
    tiles: &[("Money", "/Party/Money"), ("Camping supplies", "/Party/Camping supplies")],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: "Characters",
        path: "/Characters",
        name: &["/Name"],
        only: &[],
        cols: &[
            ("Experience", "/Experience"), ("Attribute pts", "/AttributePoints"), ("Talent pts", "/TalentPoints"),
            ("Resolve", "/Resolve"), ("Might", "/Might"), ("Finesse", "/Finesse"), ("Quickness", "/Quickness"), ("Wits", "/Wits"), ("Vitality", "/Vitality"),
            ("Lore", "/LoreSkill"), ("Athletics", "/AthleticsSkill"), ("Stealth", "/StealthSkill"), ("Mechanics", "/MechanicsSkill"),
            ("One-handed", "/OneHandedSkill"), ("Dual wield", "/DualWieldSkill"), ("Two-handed", "/TwoHandedSkill"), ("Staff", "/StaffSkill"),
            ("Unarmed", "/UnarmedSkill"), ("Bows", "/BowsSkill"), ("Magical weapons", "/MagicalWeaponSkill"),
            ("Light armor", "/LightArmorSkill"), ("Heavy armor", "/HeavyArmorSkill"), ("Parry", "/ParrySkill"), ("Dodge", "/DodgeSkill"),
            ("Engagement", "/EngagementSkill"), ("Protection", "/ProtectionSkill"),
        ],
        names: &[("Player_NewCharacter(Clone)_0", "Player character")],
    }],
};

// ------------------------------------------------------------------ Star Wars Jedi: Fallen Order

const JEDI_FO: Spec = Spec {
    name: "Star Wars Jedi: Fallen Order / Survivor",
    tab: "Cal Kestis",
    accent: (60, 150, 220),
    kind: Some(crate::fmt::jedi::KIND),
    markers: &["/Skill points/Unspent skill points"],
    subtitle: "SaveGame00.sav. The zlib-packed actor records are opened and packed again on save.",
    tiles: &[("Unspent skill points", "/Skill points/Unspent skill points"), ("Stims", "/Health and Force/Stims carried"), ("Deaths", "/Progress/Deaths")],
    sections: &[
        Section {
            title: "Skill points",
            rows: &[
                ("Unspent skill points", "/Skill points/Unspent skill points"), ("Total skill points earned", "/Skill points/Total skill points earned"),
                ("XP collected toward the next point", "/Skill points/XP collected toward the next point"),
            ],
        },
        Section {
            title: "Health and Force",
            rows: &[
                ("Stims carried", "/Health and Force/Stims carried"), ("Stim capacity", "/Health and Force/Stim capacity"),
                ("Health upgrades found", "/Health and Force/Health upgrades found"), ("Force upgrades found", "/Health and Force/Force upgrades found"),
                ("Health upgrade fragments", "/Health and Force/Health upgrade fragments"), ("Force upgrade fragments", "/Health and Force/Force upgrade fragments"),
            ],
        },
        Section {
            title: "Progress",
            rows: &[
                ("Secrets collected", "/Progress/Secrets collected"), ("Secrets in the game", "/Progress/Secrets in the game"),
                ("Map sections explored", "/Progress/Map sections explored"), ("Map sections in the game", "/Progress/Map sections in the game"),
                ("Deaths", "/Progress/Deaths"), ("Revives used", "/Progress/Revives used"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Saints Row: The Third

const SR3: Spec = Spec {
    name: "Saints Row: The Third (and Remastered)",
    tab: "Boss",
    accent: (150, 60, 190),
    kind: Some(crate::fmt::sr3::KIND),
    markers: &["/Player/Cash (in cents)"],
    subtitle: "sr3save_NN.sr3s_pc. Cash is kept in cents; the save's checksum is rewritten.",
    tiles: &[("Cash (in cents)", "/Player/Cash (in cents)"), ("Respect level", "/Player/Respect level"), ("Crib stash (in cents)", "/Player/Crib money stash (in cents)")],
    sections: &[
        Section {
            title: "Player",
            rows: &[
                ("Cash (in cents)", "/Player/Cash (in cents)"), ("Crib money stash (in cents)", "/Player/Crib money stash (in cents)"),
                ("Respect level", "/Player/Respect level"), ("Respect in this level", "/Player/Respect in this level"), ("Total respect", "/Player/Total respect"),
            ],
        },
        Section { title: "Statistics", rows: &[("Barnstorms found", "/Statistics/Barnstorms found"), ("Stunt jumps found", "/Statistics/Stunt jumps found")] },
        Section { title: "Game", rows: &[("Difficulty index (0 to 2)", "/Game/Difficulty index")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Saints Row IV

const SR4: Spec = Spec {
    name: "Saints Row IV",
    tab: "President",
    accent: (120, 70, 200),
    kind: Some(crate::fmt::sr4::KIND),
    markers: &["/Player/Cash (in cents)"],
    subtitle: "sr4save_N.sr4s_pc. Cash is kept in cents; the save's checksum is rewritten.",
    tiles: &[("Cash (in cents)", "/Player/Cash (in cents)"), ("Data clusters", "/Player/Data clusters"), ("Respect level", "/Player/Respect level")],
    sections: &[Section {
        title: "Player",
        rows: &[
            ("Cash (in cents)", "/Player/Cash (in cents)"), ("Data clusters", "/Player/Data clusters"),
            ("Respect level (from 0)", "/Player/Respect level"), ("Respect in this level", "/Player/Respect in this level"), ("Total respect", "/Player/Total respect"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Watch Dogs 2

const WD2: Spec = Spec {
    name: "Watch Dogs 2",
    tab: "Marcus",
    accent: (230, 60, 110),
    kind: Some(crate::fmt::wd2::KIND),
    markers: &[],
    subtitle: "Numbered .save files in the Ubisoft Connect save folder (2688 or 3619).",
    tiles: &[("Money", "/Player/Money"), ("Followers", "/Player/Followers"), ("Research points", "/Player/Research points")],
    sections: &[
        Section { title: "Player", rows: &[("Money", "/Player/Money"), ("Followers", "/Player/Followers"), ("Research points", "/Player/Research points")] },
        Section { title: "Totals", rows: &[("Money earned in total", "/Totals/Money earned in total"), ("Followers gained in total", "/Totals/Followers gained in total")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Fable: The Lost Chapters

const FABLE: Spec = Spec {
    name: "Fable: The Lost Chapters",
    tab: "Albion",
    accent: (170, 130, 60),
    kind: Some(crate::fmt::fable::KIND),
    markers: &["/Hero/Gold"],
    subtitle: "AutoSave or a Manual - Save file in Documents\\My Games\\Fable\\Saves\\<hero name>. Morality runs from evil (negative) to good (positive); the game keeps it within its own limits. Age changes how old the hero looks.",
    tiles: &[("Gold", "/Hero/Gold"), ("Morality", "/Hero/Morality"), ("Renown level", "/Hero/Renown level")],
    sections: &[Section { title: "Hero", rows: &[("Gold", "/Hero/Gold"), ("Morality", "/Hero/Morality"), ("Renown level", "/Hero/Renown level"), ("Age", "/Hero/Age")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Wandering Sword

const WANDERING_SWORD: Spec = Spec {
    name: "Wandering Sword",
    tab: "Jianghu",
    accent: (176, 64, 48),
    kind: Some("Unreal Engine save (JHSaveGame)"),
    markers: &["/Money", "/MapName"],
    subtitle: "<n>.sav: your money; the map, mount and play time are shown for reference. Characters and items are kept in the game's own records after the save's properties and are left as they are.",
    tiles: &[("Money", "/Money")],
    sections: &[Section {
        title: "This save",
        rows: &[
            ("Money", "/Money"), ("Map", "/MapName"), ("Mount", "/SelectedMountId"),
            ("Play time (seconds)", "/SaveGameTime"), ("Difficulty", "/DifficultyMode"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Stellar Blade

const STELLAR_BLADE_ITEMS: &[(&str, &str)] = &[
    ("BetaCrystal", "Beta crystal (probably the Gold balance)"), ("LowDensityWafer", "Low-density wafer (nano element)"), ("HighDensityWafer", "High-density wafer (nano element)"),
    ("2DQuantumWafer", "2D quantum wafer (nano element)"), ("PolymerOrganicFilm", "Polymer organic film"), ("ElastomerFiber", "Elastomer fiber"),
    ("ECelluloseRayon", "Cellulose rayon"), ("BodyCore", "Body core"), ("betacore", "Beta core"), ("BurstCore", "Burst core"),
    ("DroneCore", "Drone core"), ("GearCore", "Gear core"), ("WeaponCore_Damaged", "Damaged weapon core"), ("SPInitializer", "Skill reset item"),
    ("HP_Potion", "Health potion"), ("HP_Potion_Small", "Small health potion"), ("Recovery_HP_Potion", "Recovery potion"),
    ("ResurrectionItem", "Resurrection item"), ("PulseGrenade", "Pulse grenade"), ("HealGrenade", "Heal grenade"),
    ("ConcussionGrenade", "Concussion grenade"), ("FlashGrenade", "Flash grenade"), ("TrapGrenade", "Trap grenade"),
    ("Bullet_Normal", "Standard rounds"), ("Bullet_Scatter", "Scatter rounds"), ("Bullet_Railgun", "Railgun rounds"),
    ("Bullet_Explosive", "Explosive rounds"), ("Bullet_Missile", "Missiles"), ("ETC_Item_VendingMachineCoin", "Vending machine coin"),
    ("Money_JunkIron_A", "Junk iron A"), ("Money_JunkIron_B", "Junk iron B"), ("Money_JunkIron_C", "Junk iron C"),
];

const STELLAR_BLADE: Spec = Spec {
    name: "Stellar Blade",
    tab: "Eve",
    accent: (200, 205, 220),
    kind: Some("Unreal Engine save (SBSaveGame)"),
    markers: &["/DataMap_SBActor/PlayerActor/HP", "/DataMap_SBItem/Item/Inventory"],
    subtitle: "StellarBladeSave00.sav in %LOCALAPPDATA%\\SB\\Saved\\SaveGames\\<Steam id>. Item names are the game's internal ids; the save's CRC-32 is written again on save. Checked on a real early-game save, not yet tried in the game.",
    tiles: &[("Skill points", "/DataMap_SBActor/PlayerActor/SP"), ("Level", "/DataMap_SBActor/PlayerActor/SPLevel"), ("Beta gauge", "/DataMap_SBActor/PlayerActor/BetaGauge")],
    sections: &[
        Section {
            title: "Eve",
            rows: &[
                ("Health", "/DataMap_SBActor/PlayerActor/HP"), ("Shield", "/DataMap_SBActor/PlayerActor/Shield"),
                ("Stamina", "/DataMap_SBActor/PlayerActor/Stamina"), ("Beta gauge", "/DataMap_SBActor/PlayerActor/BetaGauge"),
                ("Burst gauge", "/DataMap_SBActor/PlayerActor/BurstGauge"), ("Tachy gauge", "/DataMap_SBActor/PlayerActor/TachyGauge"),
                ("Skill points", "/DataMap_SBActor/PlayerActor/SP"), ("Level", "/DataMap_SBActor/PlayerActor/SPLevel"),
                ("Experience", "/DataMap_SBActor/PlayerActor/SPExp"),
            ],
        },
        Section {
            title: "Game",
            rows: &[
                ("Play time (seconds)", "/DataMap_int32/PlayTime"), ("Difficulty setting", "/DataMap_int32/GameDifficulty"),
                ("New Game+ playthroughs", "/DataMap_int32/NewGamePlusPlayCount"), ("Current map", "/DataMap_FString/MapName"),
            ],
        },
    ],
    lists: &[List {
        title: "Gold, materials and consumables",
        path: "/DataMap_SBItem/Item/Inventory",
        shape: Shape::Array { id_key: "/ItemInstance/ItemAlias", qty_keys: &["/ItemInstance/ItemCount", "/ItemInstance/ItemChargeCount"] },
        names: STELLAR_BLADE_ITEMS,
        max: Some(Max {
            ids: &["LowDensityWafer", "HighDensityWafer", "2DQuantumWafer", "PolymerOrganicFilm", "ElastomerFiber", "ECelluloseRayon"],
            cap: 9999,
            caps: &[],
            estimate: true,
            about: "Raises the six nano-element and polymer crafting materials you already have to 9,999. The game's stack figure for them is 1,000 per slot but real saves hold more, so the cap is an estimate. Gold, cores, bait, grenades, potions, gear and key items are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Pragmata

const PRAGMATA_ITEMS: &[(&str, &str)] = &[
    ("37185847", "It00043"), ("60698712", "It10016"), ("179699175", "It00020"), ("187746965", "It00034"), ("410847773",
    "It00052"), ("456045304", "It00060"), ("477967137", "It32010"), ("546576816", "It10020"), ("558265447", "It30002"),
    ("594303752", "It50000"), ("619773764", "It32001"), ("659273485", "It31015"), ("662500995", "It32003"), ("663376744",
    "It00026"), ("701076331", "It00035"), ("725482672", "It50020"), ("772735750", "It00054"), ("827358611", "It32102"),
    ("890096294", "It31003"), ("902949829", "It32005"), ("972633138", "It31005"), ("1010019990", "It10025"), ("1049082274",
    "It31001"), ("1123299822", "It10017"), ("1179591319", "It00021"), ("1247845661", "It00032"), ("1273641536", "It32004"),
    ("1363364141", "It50010"), ("1449550544", "It10010"), ("1566293329", "It32002"), ("1591402201", "It10013"), ("1629885634",
    "It31018"), ("1650242729", "It10023"), ("1669182627", "It30006"), ("1764290202", "It00042"), ("1786547206", "It00053"),
    ("1802091743", "It00041"), ("1852445624", "It00022"), ("1855745451", "It31012"), ("1883713603", "It00027"), ("1887847758",
    "It30004"), ("1931569026", "It00062"), ("1942031930", "It30000"), ("1951767408", "It10019"), ("1966150077", "It10018"),
    ("1984889503", "It00033"), ("2009356002", "It31017"), ("2037365272", "It32007"), ("2041150774", "It31002"), ("2074518467",
    "It00023"), ("2107318115", "It00036"), ("2142691531", "It10000"), ("2202646009", "It00028"), ("2214733431", "It31016"),
    ("2227368435", "It00025"), ("2429803748", "It00051"), ("2577669408", "It32000"), ("2658089936", "It32200"), ("2716541145",
    "It10012"), ("2817286685", "It10011"), ("2819440956", "It31007"), ("2838867836", "It32006"), ("2863592220", "It31013"),
    ("2870192522", "It00010"), ("2947257529", "It31008"), ("2995522671", "It31004"), ("3018310814", "It00044"), ("3036423139",
    "It30003"), ("3084090558", "It10015"), ("3090659029", "It30001"), ("3118172021", "It31000"), ("3136906403", "It10024"),
    ("3279827537", "It30007"), ("3287219028", "It31011"), ("3319989394", "It32100"), ("3440652198", "It10021"), ("3530230425",
    "It60000"), ("3549133694", "It00039"), ("3558176768", "It32009"), ("3604778020", "It31009"), ("3720674438", "It32008"),
    ("3746788365", "It30005"), ("3800993639", "It99999"), ("3820139266", "It31010"), ("3929263919", "It10014"), ("3940343410",
    "It00061"), ("3970737334", "It10022"), ("4023811948", "It32101"), ("4128867654", "It31014"), ("4235794659", "It31006")
];

const PRAGMATA_WEAPONS: &[(&str, &str)] = &[
    ("163051433", "It03100"), ("494541585", "It01600"), ("713657173", "It01200"), ("775612873", "It01500"), ("860467045",
    "It01400"), ("1028629837", "It01000"), ("1107688515", "It02700"), ("1504483110", "It02100"), ("2427932221", "It01300"),
    ("2600790876", "It01900"), ("2620883957", "It01800"), ("2629333698", "It02300"), ("3183340999", "It02200"), ("3320209826",
    "It02600"), ("3516715133", "It04000"), ("3613536690", "It01700"), ("4281100518", "It02500")
];

const PRAGMATA_PERKS: &[(&str, &str)] = &[
    ("110906079", "Pe21015"), ("155565408", "Pe00006"), ("271799045", "Pe21014"), ("450124851", "Pe13630"), ("560564079",
    "Pe21002"), ("598897470", "Pe20007"), ("690202999", "Pe30000"), ("841271601", "Pe20004"), ("911942583", "Pe00000"),
    ("1040890151", "Pe21009"), ("1042964762", "Pe20002"), ("1147259676", "Pe13650"), ("1189307844", "Pe13640"), ("1218555137",
    "Pe30022"), ("1317660761", "Pe00001"), ("1379549660", "Pe30001"), ("1420355279", "Pe21018"), ("1585667794", "Pe00003"),
    ("1596387545", "Pe20011"), ("1611914211", "Pe21013"), ("1698913819", "Pe21011"), ("1740597971", "Pe20003"), ("1908354787",
    "Pe20001"), ("1908363473", "Pe30002"), ("2169297064", "Pe20010"), ("2208997515", "Pe20005"), ("2237314716", "Pe30013"),
    ("2253119052", "Pe30017"), ("2313963936", "Pe20006"), ("2365119386", "Pe20008"), ("2420827925", "Pe30018"), ("2481915581",
    "Pe30019"), ("2544407042", "Pe30021"), ("2605892101", "Pe21007"), ("2620558456", "Pe20000"), ("2722744742", "Pe21010"),
    ("2760485071", "Pe30023"), ("2797002247", "Pe21016"), ("2820153309", "Pe60001"), ("2897315983", "Pe21012"), ("2960685315",
    "Pe30020"), ("3000094269", "Pe13660"), ("3032724676", "Pe21003"), ("3035662856", "Pe21004"), ("3074270727", "Pe00007"),
    ("3219377548", "Pe21017"), ("3245834254", "Pe21001"), ("3307507513", "Pe30024"), ("3345336964", "Pe13600"), ("3372994762",
    "Pe30003"), ("3523414087", "Pe21008"), ("3553535700", "Pe30005"), ("3575509088", "Pe13620"), ("3713912905", "Pe20009"),
    ("3735227779", "Pe13610"), ("3810885086", "Pe60000"), ("3840123116", "Pe30012"), ("3889127890", "Pe00004"), ("4063943238",
    "Pe00005"), ("4085891812", "Pe00002"), ("4133191251", "Pe30015"), ("4191270525", "Pe20012"), ("4251594225", "Pe30007"),
    ("4252418028", "Pe21006")
];

const PRAGMATA: Spec = Spec {
    name: "Pragmata",
    tab: "Hugh & Diana",
    accent: (120, 190, 230),
    kind: Some("RE Engine save"),
    markers: &["/#7c416083/_GameSaveDataList/@4249498766/_InventoryItems", "/#7c416083/_GameSaveDataList/@3874718496/_PurchaseWeapons"],
    subtitle: "data000.bin and the slot files in Steam\\userdata\\<account>\\3357650\\remote\\win64_save, opened with your own Steam account. Items, weapons and perks are listed by the game's own codes (its text files, which hold the names, aren't read). Checked on real saves, not yet tried in the game.",
    tiles: &[("Difficulty rank points", "/#7c416083/_GameSaveDataList/@3086865873/RankPoint"), ("New Game+ playthroughs", "/#7c416083/_GameSaveDataList/@60430928/_CurrentCampaignPlaythroughCount")],
    sections: &[Section {
        title: "Progress",
        rows: &[
            ("Difficulty rank points", "/#7c416083/_GameSaveDataList/@3086865873/RankPoint"), ("Stored rank points", "/#7c416083/_GameSaveDataList/@3086865873/StoredRankPoint"),
            ("New Game+ playthroughs", "/#7c416083/_GameSaveDataList/@60430928/_CurrentCampaignPlaythroughCount"), ("Game time (microseconds)", "/#7c416083/_GameSaveDataList/@3540344843/_GameElapsedTime"),
        ],
    }],
    lists: &[List {
        title: "Items and currencies",
        path: "/#7c416083/_GameSaveDataList/@4249498766/_InventoryItems",
        shape: Shape::Array { id_key: "_ID", qty_keys: &["_Quantity"] },
        names: PRAGMATA_ITEMS,
        max: None,
    }],
    tables: &[
        Table {
            title: "Weapons",
            path: "/#7c416083/_GameSaveDataList/@3874718496/_PurchaseWeapons",
            name: &["/_ID"],
            only: &[],
            cols: &[("Level", "/_Level"), ("Unlocked", "/_Unlock")],
            names: PRAGMATA_WEAPONS,
        },
        Table {
            title: "Perks",
            path: "/#7c416083/_GameSaveDataList/@3656688439/EquipmentPerkSaveData/PossessionPerkUnits",
            name: &["/_ID"],
            only: &[],
            cols: &[("Level", "/_Level"), ("Unlocked", "/_Unlock")],
            names: PRAGMATA_PERKS,
        },
    ],
};

// ------------------------------------------------------------------ Slime Rancher

const SLIME_RANCHER: Spec = Spec {
    name: "Slime Rancher",
    tab: "Ranch",
    accent: (230, 120, 170),
    kind: Some(crate::fmt::slime::KIND),
    markers: &["/Player/Newbucks"],
    subtitle: "A .sav in %USERPROFILE%\\AppData\\LocalLow\\Monomi Park\\Slime Rancher (the first game; Slime Rancher 2 isn't read yet). The load screen's newbucks are changed with yours. Energy above your maximum is cut back by the game.",
    tiles: &[("Newbucks", "/Player/Newbucks"), ("Slime keys", "/Player/Slime keys")],
    sections: &[Section {
        title: "Beatrix",
        rows: &[("Newbucks", "/Player/Newbucks"), ("Health", "/Player/Health"), ("Energy", "/Player/Energy"), ("Radiation", "/Player/Radiation"), ("Slime keys", "/Player/Slime keys")],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Subnautica 2

const SUBNAUTICA2: Spec = Spec {
    name: "Subnautica 2",
    tab: "Survival",
    accent: (46, 196, 214),
    kind: Some("Subnautica 2 save"),
    markers: &["/Save"],
    subtitle: "savegame_<n>.sav in %LOCALAPPDATA%\\Subnautica2\\Saved\\SaveGames. Edits the current save; the previous-checkpoint copy is left alone. Needs the Oodle library of any game on this PC. Resources don't stack in this game, so there is no max-materials button.",
    tiles: &[("Health", "/Survival/Health"), ("Oxygen", "/Survival/Oxygen"), ("Food", "/Survival/Food"), ("Water", "/Survival/Water")],
    sections: &[Section {
        title: "Survival",
        rows: &[
            ("Health", "/Survival/Health"), ("Health (maximum)", "/Survival/Health (maximum)"),
            ("Oxygen", "/Survival/Oxygen"), ("Oxygen (maximum)", "/Survival/Oxygen (maximum)"),
            ("Food", "/Survival/Food"), ("Food (maximum)", "/Survival/Food (maximum)"),
            ("Water", "/Survival/Water"), ("Water (maximum)", "/Survival/Water (maximum)"),
        ],
    }],
    lists: &[
        List { title: "Player inventory", path: "/Player", shape: Shape::Map, names: &[], max: None },
        List { title: "Lockers, vehicles and other storage", path: "/Storage", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Horizon Zero Dawn

const HZD: Spec = Spec {
    name: "Horizon Zero Dawn",
    tab: "Aloy",
    accent: (222, 120, 60),
    kind: Some(crate::fmt::hzd::KIND),
    markers: &["/Aloy/Metal shards"],
    subtitle: "checkpoint.dat in Documents\\Horizon Zero Dawn\\Saved Game\\<slot> (the PC Complete Edition). The save's checksum is made again on save. Level and experience are shown but not changed.",
    tiles: &[("Metal shards", "/Aloy/Metal shards"), ("Skill points", "/Aloy/Skill points"), ("Level", "/Aloy/Level")],
    sections: &[Section {
        title: "Aloy",
        rows: &[("Metal shards", "/Aloy/Metal shards"), ("Skill points", "/Aloy/Skill points"), ("Level", "/Aloy/Level"), ("Experience", "/Aloy/Experience")],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Sleeping Dogs

const SLEEPING_DOGS: Spec = Spec {
    name: "Sleeping Dogs",
    tab: "Wei",
    accent: (200, 40, 50),
    kind: Some(crate::fmt::sleeping_dogs::KIND),
    markers: &["/Wei/Money"],
    subtitle: "HK GameSlot 1 ... or HK Autosave Slot in the game's data folder (Steam: steamapps\\common\\SleepingDogsDefinitiveEdition\\data). The save is unscrambled and scrambled again, with its checksum, on save. Each XP track tops out at 50,000.",
    tiles: &[("Money", "/Wei/Money"), ("Face XP", "/Wei/Face XP")],
    sections: &[Section {
        title: "Wei Shen",
        rows: &[("Money", "/Wei/Money"), ("Face XP", "/Wei/Face XP"), ("Cop XP", "/Wei/Cop XP"), ("Triad XP", "/Wei/Triad XP")],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Deus Ex: Mankind Divided

const DXMD: Spec = Spec {
    name: "Deus Ex: Mankind Divided",
    tab: "Adam",
    accent: (214, 160, 50),
    kind: Some(crate::fmt::dxmd::KIND),
    markers: &["/Adam/Credits"],
    subtitle: "DXNGsavegame0000.dat ... in Steam\\userdata\\<account>\\337000\\remote. The game state is unpacked and packed again on save. Close the game and turn Steam Cloud off for it first, or Steam may bring the old save back. Experience is shown but not changed. Ammo stacks are listed in the order the save keeps them.",
    tiles: &[("Credits", "/Adam/Credits"), ("Praxis kits", "/Adam/Praxis kits"), ("Crafting parts", "/Crafting/Crafting parts")],
    sections: &[Section {
        title: "Adam Jensen",
        rows: &[
            ("Credits", "/Adam/Credits"), ("Praxis kits", "/Adam/Praxis kits"),
            ("XP toward next Praxis kit", "/Adam/XP toward next Praxis kit"), ("Total XP", "/Adam/Total XP"),
        ],
    }],
    lists: &[
        List {
            title: "Crafting",
            path: "/Crafting",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Crafting parts"],
                cap: crate::fmt::dxmd::PARTS_CAP,
                caps: &[],
                estimate: true,
                about: "Sets crafting parts to 999, the stack size players report (no official figure, so this is an estimate).",
            }),
        },
        List { title: "Ammo", path: "/Ammo", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Atomic Heart

const ATOMIC_HEART: Spec = Spec {
    name: "Atomic Heart",
    tab: "P-3",
    accent: (200, 60, 50),
    kind: Some(crate::fmt::atomic_heart::KIND),
    markers: &["/Save/Stacks found"],
    subtitle: "<n>.sav in %LOCALAPPDATA%\\AtomicHeart\\Saved\\SaveGames. The save's Oodle data is unpacked and packed again with the Oodle library of any game on this PC, and its SHA-1 is made again. Only stacks the save already holds are listed: pick up one of an item in the game first to add it.",
    tiles: &[("Neuropolymer", "/Special/Neuropolymer")],
    sections: &[Section { title: "Special", rows: &[("Neuropolymer", "/Special/Neuropolymer")] }],
    lists: &[List {
        title: "Resources",
        path: "/Resources",
        shape: Shape::Map,
        names: &[],
        max: Some(Max {
            ids: &["Metal parts", "Synthetic material", "Biomaterial", "Superconductors", "Chemicals", "Energy cells", "Microelectronics"],
            cap: 999,
            caps: &[],
            estimate: true,
            about: "Raises metal parts, synthetic material, biomaterial, superconductors, chemicals, energy cells and microelectronics you already hold to 999 (an estimate: no public source states the stack limit, and saves with 437 are known). Neural modules are left alone.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ Dishonored 2
const DISHONORED2: Spec = Spec {
    name: "Dishonored 2",
    tab: "Bonecharms",
    accent: (150, 110, 70),
    kind: Some(crate::fmt::dishonored2::KIND),
    markers: &["/Crafting/Raw Whalebone"],
    subtitle: "savegame\\SLOT<n>\\checkpoint.dat: Raw Whalebone, runes and ammo. Each ammo count goes up to what the save says the player can carry. The save is unpacked and packed again, with its check value, on save.",
    tiles: &[("Raw Whalebone", "/Crafting/Raw Whalebone"), ("Runes", "/Inventory/Runes")],
    sections: &[Section {
        title: "Save",
        rows: &[("Map", "/Save/Map"), ("Bonecharm traits learned", "/Save/Bonecharm traits learned")],
    }],
    lists: &[List {
        title: "Crafting materials",
        path: "/Crafting",
        shape: Shape::Map,
        names: &[],
        max: Some(Max {
            ids: &["Raw Whalebone"],
            cap: crate::fmt::dishonored2::WHALEBONE_MAX,
            caps: &[],
            estimate: true,
            about: "Sets Raw Whalebone to 999. The game shows no limit and no public source states one, so 999 is a safe three-digit amount.",
        }),
    },
    List { title: "Runes and ammo", path: "/Inventory", shape: Shape::Map, names: &[], max: None }],
    tables: &[],
};

// ------------------------------------------------------------------ Deathloop

const DEATHLOOP: Spec = Spec {
    name: "Deathloop",
    tab: "Colt",
    accent: (232, 120, 40),
    kind: None,
    markers: &["/save/commonState/TimeEssence/m_currency", "/save/commonState/Progression/m_day"],
    subtitle: "save.dloop in Saved Games\\Arkane Studios\\Deathloop\\base\\savegame\\<Steam id>\\SLOT01. The save's Oodle data is unpacked and packed again with the Oodle library of any game on this PC (the game's own works), and the file's checksum is made again. Close the game and turn Steam Cloud off for it first, or Steam may bring the old save back. Weapons, trinkets and slabs are stored as numbers and are not listed.",
    tiles: &[
        ("Residuum", "/save/commonState/TimeEssence/m_currency"),
        ("Day of the loop", "/save/commonState/Progression/m_day"),
        ("Stress level", "/save/commonState/Progression/stressLoop/currentStressLevel"),
    ],
    sections: &[
        Section {
            title: "Colt",
            rows: &[
                ("Residuum", "/save/commonState/TimeEssence/m_currency"),
                ("Stress level", "/save/commonState/Progression/stressLoop/currentStressLevel"),
                ("Stress XP", "/save/commonState/Progression/stressLoop/currentStressXp"),
            ],
        },
        Section {
            title: "The loop",
            rows: &[
                ("Day", "/save/commonState/Progression/m_day"),
                ("Current location", "/save/commonState/Progression/m_currentLocation"),
                ("Targets killed this loop", "/save/commonState/Progression/m_killedTargetsCountInLoop"),
                ("Invasions so far", "/save/commonState/Progression/m_curInvasionCount"),
                ("First loop done", "/save/commonState/Progression/m_firstLoopDone"),
                ("Golden Loop done", "/save/commonState/Progression/m_goldenLoopDone"),
                ("Hunter (Julianna) unlocked", "/save/commonState/Progression/m_hunterUnlocked"),
                ("Void essence ability unlocked", "/save/commonState/Progression/m_voidEssenceAbilityUnlocked"),
                ("Speedrun challenge available", "/save/commonState/Progression/m_speedrunChallengeAvailable"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Dishonored

const DISHONORED1: Spec = Spec {
    name: "Dishonored",
    tab: "Corvo",
    accent: (150, 130, 90),
    kind: Some(crate::fmt::dishonored1::KIND),
    markers: &[],
    subtitle: "Coins only for now: runes, rewire tools, elixirs and remedies aren't located yet. Back up the save first.",
    tiles: &[("Coins", "/Corvo/Coins")],
    sections: &[Section { title: "Purse", rows: &[("Coins", "/Corvo/Coins")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Red Dead Redemption

const RDR1: Spec = Spec {
    name: "Red Dead Redemption",
    tab: "John",
    accent: (178, 34, 34),
    kind: Some(crate::fmt::rdr::KIND_1),
    markers: &["/John"],
    subtitle: "RDR2SAVE<n>.SAV (and REDDEAD2HCOR<n>.SAV, RDR2ZOMBIESAVE<n>.SAV) in Documents\\Rockstar Games\\Red Dead Redemption\\Profiles\\<id>. The save is decrypted and encrypted again with its CRC-32 when you save. Money, honor, fame, the Dead Eye meter and consumables can be changed; weapons, horses and kit are listed as owned. PS4 and Switch saves of the same release use the same format.",
    tiles: &[("Money ($)", "/John/Money ($)"), ("Honor", "/John/Honor"), ("Fame", "/John/Fame")],
    sections: &[
        Section { title: "John", rows: &[("Money ($)", "/John/Money ($)"), ("Honor", "/John/Honor"), ("Fame", "/John/Fame"), ("Dead Eye meter", "/John/Dead Eye meter")] },
        Section { title: "Save", rows: &[("Title", "/Save/Title")] },
    ],
    lists: &[
        List { title: "Consumables", path: "/Supplies", shape: Shape::Map, names: &[], max: None },
        List { title: "Weapons, horses and kit", path: "/Inventory", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Red Dead Redemption 2

const RDR2_MAX_ABOUT: &str = "Sets each herb, feather and animal fat stack the satchel holds to 99, the most an item stacks to once the Legend of the East satchel is crafted. Rockstar publishes no smaller per-item limits; with smaller satchels the game may cut stacks down when the save loads.";

const RDR2: Spec = Spec {
    name: "Red Dead Redemption 2",
    tab: "Arthur",
    accent: (160, 30, 30),
    kind: Some(crate::fmt::rdr::KIND_2),
    markers: &["/Arthur"],
    subtitle: "SRDR3<nnnn> in Documents\\Rockstar Games\\Red Dead Redemption 2\\Profiles\\<id> (story mode, PC). The save is decrypted, and its checksums are written again and it is encrypted when you save. Satchel stacks can be changed, and supplies the satchel lacks can be added (Add to satchel: an experimental step that grows the save; keep a backup). Weapons keep their own loaded ammo, which is left alone. Challenge ranks and compendium ticks change the record only: they do not hand out rewards. Compendium entries whose name is not known yet are shown by their number. Honor and camp funds also update the matching saved statistic. Turn Steam/Rockstar cloud saves off for the game first, or the old save may come back.",
    tiles: &[("Cash ($)", "/Arthur/Cash ($)"), ("Honor", "/Arthur/Honor"), ("Camp funds ($)", "/Camp/Camp funds ($)")],
    sections: &[
        Section {
            title: "Arthur",
            rows: &[
                ("Cash ($)", "/Arthur/Cash ($)"),
                ("Honor", "/Arthur/Honor"),
                ("Health core", "/Arthur/Health core"),
                ("Stamina core", "/Arthur/Stamina core"),
                ("Dead Eye core", "/Arthur/Dead Eye core"),
                ("Health XP", "/Arthur/Health XP"),
                ("Stamina XP", "/Arthur/Stamina XP"),
                ("Dead Eye XP", "/Arthur/Dead Eye XP"),
                ("Weight", "/Arthur/Weight"),
                ("Saved health", "/Arthur/Saved health"),
            ],
        },
        Section { title: "Camp", rows: &[("Camp funds ($)", "/Camp/Camp funds ($)")] },
        Section {
            title: "Bounties",
            rows: &[
                ("Ambarino ($)", "/Bounties/Ambarino ($)"),
                ("New Hanover ($)", "/Bounties/New Hanover ($)"),
                ("Lemoyne ($)", "/Bounties/Lemoyne ($)"),
                ("West Elizabeth ($)", "/Bounties/West Elizabeth ($)"),
                ("New Austin ($)", "/Bounties/New Austin ($)"),
                ("Guarma ($)", "/Bounties/Guarma ($)"),
            ],
        },
        Section { title: "World", rows: &[("Hour", "/World/Hour"), ("Minute", "/World/Minute"), ("Second", "/World/Second"), ("Save", "/Save/Title")] },
    ],
    lists: &[
        List { title: "Ammunition", path: "/Ammunition", shape: Shape::Map, names: &[], max: None },
        List {
            title: "Consumables and herbs",
            path: "/Consumables",
            shape: Shape::Map,
            names: &[],
            max: Some(Max { ids: &["Herb Alaskan Ginseng", "Herb American Ginseng", "Herb Bay Bolete", "Herb Black Berry", "Herb Black Currant", "Herb Burdock Root", "Herb Chanterelles", "Herb Common Bulrush", "Herb Creeping Thyme", "Herb Currant", "Herb Desert Sage", "Herb English Mace", "Herb Evergreen Huckleberry", "Herb Ginseng", "Herb Golden Currant", "Herb Harrietum", "Herb Hummingbird Sage", "Herb Indian Tobacco", "Herb Milkweed", "Herb Oleander Sage", "Herb Oregano", "Herb Parasol Mushroom", "Herb Prairie Poppy", "Herb Rams Head", "Herb Red Raspberry", "Herb Red Sage", "Herb Sage", "Herb Vanilla Flower", "Herb Violet Snowdrop", "Herb Wild Carrots", "Herb Wild Feverfew", "Herb Wild Mint", "Herb Wintergreen Berry", "Herb Yarrow"], cap: crate::fmt::rdr::RDR2_STACK_MAX, caps: &[], estimate: false, about: RDR2_MAX_ABOUT }),
        },
        List {
            title: "Provisions",
            path: "/Provisions",
            shape: Shape::Map,
            names: &[],
            max: Some(Max { ids: &["Bird Feather Flight", "Bluejay Feather", "Booby Feather", "Cardinal Feather", "Cedarwaxwing Feather", "Chicken Feather", "Condor Feather", "Cormorant Feather", "Crow Feather", "Duck Feather", "Eagle Feather", "Egret Feather", "Goose Feather", "Hawk Feather", "Heron Feather", "Loon Feather", "Oriole Feather", "Owl Feather", "Parakeet Feather", "Parrot Feather", "Pelican Feather", "Pheasant Feather", "Pigeon Feather", "Quail Feather", "Raven Feather", "Robin Feather", "Rooster Feather", "Seagull Feather", "Songbird Feather", "Sparrow Feather", "Spoonbill Feather", "Turkey Feather", "Vulture Feather", "Whooping Crane Feather", "Woodpecker Feather", "Animal Fat"], cap: crate::fmt::rdr::RDR2_STACK_MAX, caps: &[], estimate: false, about: RDR2_MAX_ABOUT }),
        },
        List { title: "Statistics", path: "/Statistics", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[
        Table { title: "Challenges (rank 0 to 10)", path: "/Challenges", name: &[], cols: &[("Rank", "/Rank")], only: &[], names: &[] },
        Table { title: "Add to satchel", path: "/Add to satchel", name: &[], cols: &[("Add", "/Add"), ("Group", "/Group")], only: &[], names: &[] },
        Table { title: "Compendium: animals", path: "/Compendium/Animals", name: &[], cols: &[("Studied", "/Studied"), ("Discovered", "/Discovered")], only: &[], names: &[] },
        Table { title: "Compendium: plants", path: "/Compendium/Plants", name: &[], cols: &[("Studied", "/Studied")], only: &[], names: &[] },
        Table { title: "Compendium: fish", path: "/Compendium/Fish", name: &[], cols: &[("Studied", "/Studied")], only: &[], names: &[] },
        Table { title: "Compendium: horses", path: "/Compendium/Horses", name: &[], cols: &[("Studied", "/Studied")], only: &[], names: &[] },
        Table { title: "Compendium: weapons", path: "/Compendium/Weapons", name: &[], cols: &[("Owned", "/Owned")], only: &[], names: &[] },
        Table { title: "Compendium: equipment", path: "/Compendium/Equipment", name: &[], cols: &[("Owned", "/Owned")], only: &[], names: &[] },
    ],
};

// ------------------------------------------------------------------ Assassin's Creed (Brotherhood to Odyssey)
const AC_BROTHERHOOD: Spec = Spec {
    name: "Assassin's Creed Brotherhood",
    tab: "Ezio",
    accent: (180, 40, 40),
    kind: Some(crate::fmt::anvil::KIND_BROTHERHOOD),
    markers: &["/Money/Money"],
    subtitle: "ACBROTHERHOODSAVEGAME0.SAV in Saved Games\\\\Assassin's Creed Brotherhood\\\\SAVES (or the Ubisoft Connect savegames folder). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back.",
    tiles: &[("Florins", "/Money/Money")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: None },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_REVELATIONS: Spec = Spec {
    name: "Assassin's Creed Revelations",
    tab: "Ezio",
    accent: (170, 60, 40),
    kind: Some(crate::fmt::anvil::KIND_REVELATIONS),
    markers: &["/Money/Money"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 40). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back.",
    tiles: &[("Akçe", "/Money/Money")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: None },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC3R: Spec = Spec {
    name: "Assassin's Creed III Remastered",
    tab: "Connor",
    accent: (60, 110, 160),
    kind: Some(crate::fmt::anvil::KIND_AC3R),
    markers: &["/Money/Money"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 5183). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back. The save ends with a 4-byte value that isn't understood yet and is kept as it is; edited saves of this game have not been tried in the game.",
    tiles: &[("Money", "/Money/Money")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: None },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_LIBERATION: Spec = Spec {
    name: "Assassin's Creed Liberation HD",
    tab: "Aveline",
    accent: (150, 80, 140),
    kind: Some(crate::fmt::anvil::KIND_LIBERATION),
    markers: &["/Money/Money"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (games 625 and 632). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back.",
    tiles: &[("Money", "/Money/Money")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: None },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_BLACK_FLAG: Spec = Spec {
    name: "Assassin's Creed IV Black Flag",
    tab: "Edward",
    accent: (40, 90, 150),
    kind: Some(crate::fmt::anvil::KIND_BLACK_FLAG),
    markers: &["/Money/Money"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 273 or 437). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back.",
    tiles: &[("Reales", "/Money/Money")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: Some(Max { ids: &["Wood", "Metal", "Cloth"], cap: crate::fmt::anvil::BLACK_FLAG_CARGO, caps: &[], estimate: false, about: "Sets wood, metal and cloth to 2,000, what the Jackdaw's fully upgraded hold takes. Rum and sugar are trade goods and are left alone." }) },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_ROGUE: Spec = Spec {
    name: "Assassin's Creed Rogue",
    tab: "Shay",
    accent: (60, 120, 90),
    kind: Some(crate::fmt::anvil::KIND_ROGUE),
    markers: &["/Money/Money"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 895 or 934). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back.",
    tiles: &[("Reales", "/Money/Money")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: Some(Max { ids: &["Wood", "Metal", "Cloth", "Stone"], cap: crate::fmt::anvil::ROGUE_CARGO, caps: &[], estimate: false, about: "Sets wood, metal, cloth and stone to 2,500, what the Morrigan's fully upgraded hold takes. Tobacco is a trade good and is left alone." }) },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_UNITY: Spec = Spec {
    name: "Assassin's Creed Unity",
    tab: "Arno",
    accent: (70, 80, 160),
    kind: Some(crate::fmt::anvil::KIND_UNITY),
    markers: &["/Money/Money"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 720 or 857). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back.",
    tiles: &[("Francs", "/Money/Money"), ("Creed Points", "/Money/Creed Points")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: None },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_SYNDICATE: Spec = Spec {
    name: "Assassin's Creed Syndicate",
    tab: "Frye twins",
    accent: (140, 40, 60),
    kind: Some(crate::fmt::anvil::KIND_SYNDICATE),
    markers: &["/Money/Money"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 1875 or 1957). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back. The save ends with a 4-byte value that isn't understood yet and is kept as it is; edited saves of this game have not been tried in the game.",
    tiles: &[("Pounds", "/Money/Money")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: Some(Max { ids: &["Leather", "Metal", "Chemicals", "Silk"], cap: crate::fmt::anvil::SYNDICATE_MATERIALS, caps: &[], estimate: true, about: "Sets leather, metal, chemicals and silk to 9,999. No public source states a limit, so this is an estimate. Unique materials such as black pearls are left alone." }) },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_ORIGINS: Spec = Spec {
    name: "Assassin's Creed Origins",
    tab: "Bayek",
    accent: (200, 150, 60),
    kind: Some(crate::fmt::anvil::KIND_ORIGINS),
    markers: &["/Money/Drachmae"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 3539 or 4923). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back. The save ends with a 4-byte value that isn't understood yet and is kept as it is; edited saves of this game have not been tried in the game. Other crafting materials are stored by item number and aren't named yet.",
    tiles: &[("Drachmae", "/Money/Drachmae"), ("XP", "/Progress/XP"), ("Ability points", "/Progress/Ability points")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }, Section { title: "Progress", rows: &[("XP", "/Progress/XP"), ("Ability points", "/Progress/Ability points")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: None },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};
const AC_ODYSSEY: Spec = Spec {
    name: "Assassin's Creed Odyssey",
    tab: "Misthios",
    accent: (60, 140, 170),
    kind: Some(crate::fmt::anvil::KIND_ODYSSEY),
    markers: &["/Money/Drachmae"],
    subtitle: "<number>.save in the Ubisoft Connect savegames folder (game 5059 or 5092). The save is unscrambled, unpacked and packed again with its block checks; the account it belongs to is kept. Close the game and turn cloud saves off in Ubisoft Connect first, or the old save may come back.",
    tiles: &[("Drachmae", "/Money/Drachmae"), ("XP", "/Progress/XP"), ("Ability points", "/Progress/Ability points")],
    sections: &[Section { title: "Save", rows: &[("Game", "/Save/Game")] }, Section { title: "Progress", rows: &[("XP", "/Progress/XP"), ("Ability points", "/Progress/Ability points")] }],
    lists: &[
        List { title: "Crafting materials", path: "/Crafting materials", shape: Shape::Map, names: &[], max: Some(Max { ids: &["Olive Wood", "Iron Metal", "Soft Leather", "Obsidian Glass", "Precious Gems", "Ancient Tablets"], cap: crate::fmt::anvil::ODYSSEY_MATERIALS, caps: &[], estimate: true, about: "Sets the six crafting materials to 99,999. No public source states a limit (real saves hold over 60,000), so this is an estimate." }) },
        List { title: "Trade goods", path: "/Trade goods", shape: Shape::Map, names: &[], max: None },
        List { title: "Tools and ammunition", path: "/Tools and ammunition", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};




// ------------------------------------------------------------------ WorldBox

const WORLDBOX: Spec = Spec {
    name: "WorldBox - God Simulator",
    tab: "World",
    accent: (92, 168, 96),
    kind: None,
    markers: &["/mapStats/id_unit", "/actors_data"],
    subtitle: "A map.wbox from AppData\\LocalLow\\mkarpenko\\WorldBox\\saves\\saveN (zlib-compressed JSON, unpacked and packed again). Every unit's level, experience, health, hunger and ruler stats, and the first village's stored resources.",
    tiles: &[("Population", "/mapStats/population"), ("Deaths", "/mapStats/deaths")],
    sections: &[],
    lists: &[List {
        title: "First village: stored resources",
        path: "/cities/0/storage/savedResources",
        shape: Shape::Array { id_key: "id", qty_keys: &["amount"] },
        names: &[],
        max: None,
    }],
    tables: &[Table {
        title: "Units",
        path: "/actors_data",
        name: &["/name"],
        only: &[],
        cols: &[
            ("Level", "/level"), ("Experience", "/experience"), ("Health", "/health"), ("Hunger", "/hunger"), ("Diplomacy", "/diplomacy"),
            ("Intelligence", "/intelligence"), ("Stewardship", "/stewardship"), ("Warfare", "/warfare"), ("Age", "/age_overgrowth"), ("Children", "/children"),
        ],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Core Keeper

const CK_SKILLS: &[(&str, &str)] = &[("0", "Mining"), ("1", "Running"), ("2", "Melee"), ("3", "Vitality"), ("4", "Crafting"), ("5", "Range"), ("6", "Gardening"), ("7", "Fishing"), ("8", "Cooking"), ("9", "Magic"), ("10", "Summoning")];

const CK_ITEMS: &[(&str, &str)] = &[("37", "Galaxite Mining Pick"), ("38", "Galaxite Shovel"), ("52", "Iron Fishing Rod"), ("55", "Galaxite Fishing Rod"), ("65", "Galaxite Sledge"), ("110", "Torch"), ("210", "Locked Copper Chest"), ("212", "Copper Key"), ("300", "Wall Dirt Block"), ("327", "Wall Limestone Block"), ("1246", "Pet Prince Slime Blob"), ("1500", "Copper Ore"), ("1522", "Octarine Ore"), ("1610", "Wood"), ("1640", "Fiber"), ("1648", "Scrap Part"), ("3510", "Jelly Fish"), ("5041", "Rusty Spoon"), ("5042", "Green Kelp"), ("5044", "Amber Fish Egg"), ("6581", "Speeder Boat"), ("7900", "Milk"), ("8301", "Orb Lantern"), ("8322", "Octarine Shield"), ("8340", "Swift Feather"), ("8341", "Azeos Swift Feather"), ("8384", "Bait Octopus Boss"), ("8404", "Octarine Backpack"), ("8504", "Iron Helm"), ("8514", "Octarine Helm"), ("8523", "Galaxite Helm"), ("8614", "Octarine Breast Armor"), ("8623", "Galaxite Breast Armor"), ("8652", "Alien Tech Breast Armor"), ("8704", "Iron Pants Armor"), ("8708", "Octarine Pants Armor"), ("8717", "Galaxite Pants Armor"), ("8727", "Tamer Pants Armor"), ("8808", "Remedaisy Necklace"), ("8839", "Fang Necklace"), ("8850", "Clot Ring"), ("8855", "Ring Of Rock"), ("8859", "Sky Ring"), ("8861", "Petal Ring"), ("8862", "Mold Ring"), ("8907", "Slippery Slime Sword"), ("8908", "Galaxite Sword"), ("9002", "Small Bomb"), ("9112", "Blowpipe"), ("9114", "Octarine Bow"), ("9119", "Bubble Gun"), ("9400", "Healing Potion"), ("9404", "Greater Healing Potion"), ("9500", "Cooked Soup"), ("9502", "Cooked Salad"), ("9503", "Cooked Wrap"), ("9504", "Cooked Steak"), ("9509", "Cooked Fillet"), ("9513", "Cooked Pan Curry")];

const COREKEEPER: Spec = Spec {
    name: "Core Keeper",
    tab: "Character",
    accent: (200, 140, 70),
    kind: None,
    markers: &["/characterGuid", "/skills", "/inventory"],
    subtitle: "A character .json from AppData\\LocalLow\\Pugstorm\\Core Keeper\\Steam\\<id>\\saves. Skill experience, coins, health and inventory counts. Close the game before editing.",
    tiles: &[("Ancient coins", "/coinAmount"), ("Max health", "/maxHealth")],
    sections: &[],
    lists: &[
        List { title: "Skill experience", path: "/skills", shape: Shape::Array { id_key: "skillID", qty_keys: &["value"] }, names: CK_SKILLS, max: None },
        List { title: "Inventory", path: "/inventory", shape: Shape::Array { id_key: "objectID", qty_keys: &["amount"] }, names: CK_ITEMS, max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ HoloCure

const HOLOCURE: Spec = Spec {
    name: "HoloCure - Save the Fans!",
    tab: "Fans",
    accent: (110, 190, 235),
    kind: None,
    markers: &["/holoCoins", "/unlockedWeapons", "/ATK"],
    subtitle: "A save_n.dat from %LOCALAPPDATA%\\HoloCure (base64 JSON, unpacked and packed again). HoloCoins and the level of each upgrade.",
    tiles: &[("HoloCoins", "/holoCoins")],
    sections: &[Section {
        title: "Upgrades (levels)",
        rows: &[
            ("ATK", "/ATK"), ("HP", "/HP"), ("SPD", "/SPD"), ("Crit", "/crit"), ("Pickup range", "/pickupRange"), ("Haste", "/haste"),
            ("Damage resistance", "/DR"), ("Regen", "/regen"), ("Special cooldown", "/specCDR"), ("Enhance", "/enhanceUp"), ("EXP", "/EXP"),
            ("Money gain", "/moneyGain"), ("Rerolls", "/reroll"), ("Refund", "/refund"), ("Eliminate", "/eliminate"), ("Skill damage", "/skillDamage"),
            ("Food", "/food"), ("Growth", "/growth"), ("Item limit", "/itemLimit"), ("Weapon limit", "/weaponLimit"), ("Fandom", "/fandom"), ("Stamps", "/stamps"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Potion Craft

const POTION_CRAFT: Spec = Spec {
    name: "Potion Craft: Alchemist Simulator",
    tab: "Alchemist",
    accent: (150, 110, 200),
    kind: None,
    markers: &["/1/gold", "/1/playerInventory"],
    subtitle: "A .pcsave from AppData\\LocalLow\\niceplay games\\Potion Craft\\SavesSteam (shown as two parts: the save's name and day, then the game state). Close the game before editing.",
    tiles: &[("Gold", "/1/gold"), ("Popularity", "/1/popularity"), ("Karma", "/1/karma"), ("Level", "/1/experience/currentLvl")],
    sections: &[Section {
        title: "Alchemist",
        rows: &[("Gold", "/1/gold"), ("Popularity", "/1/popularity"), ("Karma", "/1/karma"), ("Experience", "/1/experience/currentExp"), ("Level", "/1/experience/currentLvl"), ("Day", "/1/day")],
    }],
    lists: &[List {
        title: "Inventory",
        path: "/1/playerInventory",
        shape: Shape::Array { id_key: "name", qty_keys: &["count"] },
        names: &[],
        max: Some(Max {
            ids: &["Waterbloom", "Leaf", "GreenMushroom", "Windbloom", "GoblinMushroom", "IceDragonfruit", "GreyChanterelle", "Wierdshroom", "RedMushroom", "Firebell", "Marshrooms", "FrostSapphire", "DryadMushroom", "WitchMushroom", "LavaRoot", "Thornstick", "Goldthorn", "BloodyRoot", "CliffFungus", "GraveTruffle", "Thistle", "Refruit", "SulphurShelf", "EarthCrystal", "BloodRuby", "FireCitrine", "Crystal", "LumpyBeet", "Tangleweeds"],
            cap: 999,
            caps: &[],
            estimate: true,
            about: "Raises the plant, mushroom and crystal ingredients you already have to 999. Potions and the alchemy machine's products are left alone. The game states no stack limit, so 999 is an estimate.",
        }),
    }],
    tables: &[],
};

// ------------------------------------------------------------------ World War Z

const WWZ: Spec = Spec {
    name: "World War Z (and Aftermath)",
    tab: "Survivor",
    accent: (200, 80, 60),
    kind: None,
    markers: &["/UserProgression/Shared/sharedProgressionJson/currency"],
    subtitle: "user_progression.dat from %LOCALAPPDATA%\\Saber\\WWZ\\client\\storage\\slots (unpacked with the key derived from the file name, and packed again).",
    tiles: &[
        ("Currency", "/UserProgression/Shared/sharedProgressionJson/currency"),
        ("Coins", "/UserProgression/Shared/sharedProgressionJson/coins"),
        ("Level", "/UserProgression/Shared/sharedProgressionJson/level"),
    ],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: "Class perks",
        path: "/UserProgression/PVE/perksProgressionJson/data",
        name: &[],
        only: &[],
        cols: &[("Level", "/level"), ("Experience", "/experience"), ("Prestige", "/prestige")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Roboquest

const ROBOQUEST: Spec = Spec {
    name: "Roboquest",
    tab: "Robot",
    accent: (240, 170, 40),
    kind: Some("Unreal Engine save (BP_SaveGame_Profile_C)"),
    markers: &["/WrenchAmount", "/PlayerClass"],
    subtitle: "RQProfile.sav from %LOCALAPPDATA%\\RoboQuest\\Saved\\SaveGames. Wrenches, difficulty, class and statistics.",
    tiles: &[("Wrenches", "/WrenchAmount"), ("Wrenches spent", "/ConsumedWrenches"), ("Player level", "/Saved_PlayerData/PlayerLevel")],
    sections: &[
        Section { title: "Profile", rows: &[("Wrenches", "/WrenchAmount"), ("Wrenches spent", "/ConsumedWrenches"), ("Class", "/PlayerClass"), ("Difficulty", "/Difficulty"), ("Secret ruins found", "/SecretRuins")] },
        Section {
            title: "Statistics",
            rows: &[
                ("Enemies destroyed", "/Stats/NB_ENEMY_DESTROY"), ("Bosses taken down", "/Stats/NB_BOSS_TAKEDOWN"), ("Damage dealt", "/Stats/NB_DAMAGE_DEALT"),
                ("Power cells gathered", "/Stats/NB_POWERCELL_GATHER"), ("Wrenches gathered", "/Stats/NB_WRENCH_GATHER"), ("Gadgets unlocked", "/Stats/NB_GADGET_UNLOCK"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};



// ------------------------------------------------------------------ Car Mechanic Simulator 2018

const CMS18: Spec = Spec {
    name: "Car Mechanic Simulator 2018",
    tab: "Mechanic",
    accent: (230, 120, 40),
    kind: Some(".NET BinaryFormatter (GlobalDataWrapper)"),
    markers: &["/playerMoney", "/PlayerLevel"],
    subtitle: "GlobalData.dat from a profile folder (profile0 …) of the game's Saves folder. Money, level and experience.",
    tiles: &[("Money", "/playerMoney"), ("Level", "/PlayerLevel"), ("Experience", "/playerExp")],
    sections: &[Section { title: "Profile", rows: &[("Money", "/playerMoney"), ("Level", "/PlayerLevel"), ("Experience", "/playerExp"), ("Missions finished", "/MissionsFinished"), ("Sheds available", "/ShedsAvailable")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ The Long Drive

const LONGDRIVE: Spec = Spec {
    name: "The Long Drive",
    tab: "Driver",
    accent: (230, 170, 60),
    kind: Some(".NET BinaryFormatter (savedata)"),
    markers: &["/player/hp", "/player/food", "/time"],
    subtitle: "TLDsaveData.tlds from a save folder of the game. The driver's vitals and the world time; the distance driven is in TLDsaveProp.tldsp (the same folder).",
    tiles: &[("Health", "/player/hp"), ("Food", "/player/food"), ("Water", "/player/water"), ("Sleep", "/player/sleep")],
    sections: &[Section { title: "Driver", rows: &[("Health", "/player/hp"), ("Food", "/player/food"), ("Water", "/player/water"), ("Sleep", "/player/sleep"), ("World time", "/time")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ The Long Drive (properties)

const LONGDRIVE_PROP: Spec = Spec {
    name: "The Long Drive",
    tab: "Trip",
    accent: (230, 170, 60),
    kind: Some(".NET BinaryFormatter (saveproperties)"),
    markers: &["/driven", "/seed"],
    subtitle: "TLDsaveProp.tldsp from a save folder of the game: the distance driven and the world seed.",
    tiles: &[("Driven", "/driven"), ("Seed", "/seed")],
    sections: &[Section { title: "Trip", rows: &[("Distance driven", "/driven"), ("World seed", "/seed")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Xenoblade Chronicles 3

const XC3: Spec = Spec {
    name: "Xenoblade Chronicles 3",
    tab: "Xenoblade",
    accent: (64, 160, 220),
    kind: Some(crate::fmt::xc3::KIND),
    markers: &["/Party/Gold"],
    subtitle: "A Switch save from an emulator or a dump (bf3game*.sav). Gold, play time, each character's level and experience, and the cylinder, gem and collectible amounts.",
    tiles: &[("Gold", "/Party/Gold"), ("Play time (seconds)", "/Party/Play time (seconds)")],
    sections: &[],
    lists: &[],
    tables: &[
        Table {
            title: "Characters",
            path: "/Characters",
            name: &[],
            cols: &[("Level", "/Level"), ("Experience", "/Experience"), ("Bonus experience", "/Bonus experience")],
            only: &[],
            names: &[],
        },
        Table {
            title: "Ether cylinders",
            path: "/Ether cylinders",
            name: &[],
            cols: &[("Amount", "/Amount")],
            only: &[],
            names: &[],
        },
        Table {
            title: "Gem crystals",
            path: "/Gem crystals",
            name: &[],
            cols: &[("Amount", "/Amount")],
            only: &[],
            names: &[],
        },
        Table {
            title: "Collectibles",
            path: "/Collectibles",
            name: &[],
            cols: &[("Amount", "/Amount")],
            only: &[],
            names: &[],
        },
    ],
};

const MLDT: Spec = Spec {
    name: "Mario & Luigi: Dream Team",
    tab: "Brothers",
    accent: (220, 50, 50),
    kind: Some(crate::fmt::mldt::KIND),
    markers: &["/Coins"],
    subtitle: "A decrypted 3DS save (ML4_001.sav). Coins, both brothers' level, experience and stats, and the counts of recovery items, battle items and beans.",
    tiles: &[("Coins", "/Coins")],
    sections: &[
        Section {
            title: "Mario",
            rows: &[("Level", "/Mario/Level"), ("Experience", "/Mario/Experience"), ("HP", "/Mario/HP"), ("Max HP", "/Mario/Max HP"), ("BP", "/Mario/BP"), ("Max BP", "/Mario/Max BP"), ("Power", "/Mario/Power"), ("Defense", "/Mario/Defense"), ("Speed", "/Mario/Speed"), ("Stache", "/Mario/Stache")],
        },
        Section {
            title: "Luigi",
            rows: &[("Level", "/Luigi/Level"), ("Experience", "/Luigi/Experience"), ("HP", "/Luigi/HP"), ("Max HP", "/Luigi/Max HP"), ("BP", "/Luigi/BP"), ("Max BP", "/Luigi/Max BP"), ("Power", "/Luigi/Power"), ("Defense", "/Luigi/Defense"), ("Speed", "/Luigi/Speed"), ("Stache", "/Luigi/Stache")],
        },
        Section {
            title: "Recovery items",
            rows: &[("Mushroom", "/Recovery items/Mushroom"), ("Super Mushroom", "/Recovery items/Super Mushroom"), ("Ultra Mushroom", "/Recovery items/Ultra Mushroom"), ("Max Mushroom", "/Recovery items/Max Mushroom"), ("Nut", "/Recovery items/Nut"), ("Super Nut", "/Recovery items/Super Nut"), ("Ultra Nut", "/Recovery items/Ultra Nut"), ("Max Nut", "/Recovery items/Max Nut"), ("Syrup Jar", "/Recovery items/Syrup Jar"), ("Super Syrup Jar", "/Recovery items/Super Syrup Jar"), ("Ultra Syrup Jar", "/Recovery items/Ultra Syrup Jar"), ("Max Syrup Jar", "/Recovery items/Max Syrup Jar"), ("Candy", "/Recovery items/Candy"), ("Super Candy", "/Recovery items/Super Candy"), ("Ultra Candy", "/Recovery items/Ultra Candy"), ("Max Candy", "/Recovery items/Max Candy"), ("1-Up Mushroom", "/Recovery items/1-Up Mushroom"), ("1-Up Deluxe", "/Recovery items/1-Up Deluxe"), ("Refreshing Herb", "/Recovery items/Refreshing Herb")],
        },
        Section {
            title: "Battle items",
            rows: &[("Taunt Ball", "/Battle items/Taunt Ball"), ("Shock Bomb", "/Battle items/Shock Bomb"), ("Boo Biscuit", "/Battle items/Boo Biscuit"), ("Secret Box", "/Battle items/Secret Box")],
        },
        Section {
            title: "Beans",
            rows: &[("HP Bean", "/Beans/HP Bean"), ("Bros. Bean", "/Beans/Bros. Bean"), ("Power Bean", "/Beans/Power Bean"), ("Defense Bean", "/Beans/Defense Bean"), ("Speed Bean", "/Beans/Speed Bean"), ("Stache Bean", "/Beans/Stache Bean"), ("Super HP Bean", "/Beans/Super HP Bean"), ("Super Bros. Bean", "/Beans/Super Bros. Bean"), ("Super Power Bean", "/Beans/Super Power Bean"), ("Super Defense Bean", "/Beans/Super Defense Bean"), ("Super Speed Bean", "/Beans/Super Speed Bean"), ("Super Stache Bean", "/Beans/Super Stache Bean")],
        },
    ],
    lists: &[],
    tables: &[],
};

const SMASH: Spec = Spec {
    name: "Super Smash Bros. Ultimate",
    tab: "Smash",
    accent: (230, 60, 60),
    kind: Some(crate::fmt::nintendo::SMASH_KIND),
    markers: &["/Main/Gold"],
    subtitle: "A Switch system_data.bin (decrypted). Gold, tickets, hammers, snacks, SP and the spirit board item counts.",
    tiles: &[("Gold", "/Main/Gold"), ("SP", "/Spirits/SP")],
    sections: &[
        Section {
            title: "Main",
            rows: &[("Tickets", "/Main/Tickets"), ("Gold", "/Main/Gold"), ("Hammers", "/Main/Hammers")],
        },
        Section {
            title: "Spirits",
            rows: &[("Snacks (S)", "/Spirits/Snacks (S)"), ("Snacks (M)", "/Spirits/Snacks (M)"), ("Snacks (L)", "/Spirits/Snacks (L)"), ("SP", "/Spirits/SP")],
        },
        Section {
            title: "Spirit board items",
            rows: &[("Shuffle all", "/Spirit board items/Shuffle all"), ("All primaries", "/Spirit board items/All primaries"), ("All supports", "/Spirit board items/All supports"), ("Filler", "/Spirit board items/Filler"), ("Rematch", "/Spirit board items/Rematch"), ("50% damage", "/Spirit board items/50% damage"), ("Slow FS charging", "/Spirit board items/Slow FS charging"), ("Weaken minions", "/Spirit board items/Weaken minions"), ("Health drain", "/Spirit board items/Health drain"), ("Disable items", "/Spirit board items/Disable items"), ("Shield spacer", "/Spirit board items/Shield spacer"), ("Sluggish shield", "/Spirit board items/Sluggish shield")],
        },
    ],
    lists: &[],
    tables: &[],
};

const MK7: Spec = Spec {
    name: "Mario Kart 7",
    tab: "Kart",
    accent: (230, 120, 30),
    kind: Some(crate::fmt::nintendo::MK7_KIND),
    markers: &["/Player/Coins"],
    subtitle: "A decrypted 3DS system0.dat. Coins, StreetPass count, online wins and losses, and VR points; the save's checksum is rewritten.",
    tiles: &[("Coins", "/Player/Coins"), ("VR points", "/Player/VR points")],
    sections: &[
        Section {
            title: "Player",
            rows: &[("Name", "/Player/Name"), ("Coins", "/Player/Coins"), ("StreetPass", "/Player/StreetPass"), ("Online wins", "/Player/Online wins"), ("Online losses", "/Player/Online losses"), ("VR points", "/Player/VR points")],
        },
    ],
    lists: &[],
    tables: &[],
};

const SKC: Spec = Spec {
    name: "Super Kirby Clash",
    tab: "Clash",
    accent: (240, 110, 170),
    kind: Some(crate::fmt::nintendo::SKC_KIND),
    markers: &["/Resources/Apple gems"],
    subtitle: "A decrypted 3DS savedata.dat. Apple gems and the four kinds of shards.",
    tiles: &[("Apple gems", "/Resources/Apple gems")],
    sections: &[
        Section {
            title: "Resources",
            rows: &[("Apple gems", "/Resources/Apple gems"), ("Apple gems bought", "/Resources/Apple gems bought"), ("Red shards", "/Resources/Red shards"), ("Blue shards", "/Resources/Blue shards"), ("Yellow shards", "/Resources/Yellow shards"), ("Rare shards", "/Resources/Rare shards")],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Undertale Yellow

const UTY: Spec = Spec {
    name: "Undertale Yellow",
    tab: "Clover",
    accent: (255, 222, 0),
    kind: Some("INI / config"),
    markers: &["/Save1/Gold", "/Save1/MAXHP", "/Items"],
    subtitle: "The save file from %LOCALAPPDATA%\\UNDERTALE_YELLOW (the one with Save1 and Items sections). Stats, gold, equipment names, the eight item slots and the play time. Layout from KockaAdmiralac's uty-save-editor.",
    tiles: &[("Gold", "/Save1/Gold"), ("LV", "/Save1/LV"), ("HP", "/Save1/HP")],
    sections: &[
        Section { title: "Stats", rows: &[
            ("LV", "/Save1/LV"), ("EXP", "/Save1/EXP"), ("Gold", "/Save1/Gold"), ("HP", "/Save1/HP"), ("Max HP", "/Save1/MAXHP"),
            ("AT (primary)", "/Save1/AT - Primary"), ("AT (ammo)", "/Save1/AT - Secondary"), ("DF (primary)", "/Save1/DFP"), ("DF (accessory)", "/Save1/DFS"),
            ("PP", "/Save1/PP"), ("Max PP", "/Save1/MAXPP"), ("SP", "/Save1/SP"), ("Max SP", "/Save1/MAXSP"), ("RP", "/Save1/RP"), ("Max RP", "/Save1/MAXRP"),
        ] },
        Section { title: "Equipment (names as the game writes them)", rows: &[
            ("Weapon", "/Save1/Weapon"), ("Ammo", "/Save1/Ammo"), ("Armor", "/Save1/Armor"), ("Accessory", "/Save1/Accessory"),
        ] },
        Section { title: "Items", rows: &[
            ("Slot 1", "/Items/00"), ("Slot 2", "/Items/01"), ("Slot 3", "/Items/02"), ("Slot 4", "/Items/03"),
            ("Slot 5", "/Items/04"), ("Slot 6", "/Items/05"), ("Slot 7", "/Items/06"), ("Slot 8", "/Items/07"),
        ] },
        Section { title: "Progress", rows: &[("Play time (seconds)", "/Playtime/Seconds"), ("Fast travel unlocked", "/Save1/FTravel"), ("Running allowed", "/Save1/playerCanRun")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Daggerfall Unity

const DAGGERFALL_UNITY: Spec = Spec {
    name: "The Elder Scrolls II: Daggerfall (Daggerfall Unity)",
    tab: "Character",
    accent: (170, 60, 50),
    kind: None,
    markers: &["/playerData/playerEntity/goldPieces", "/playerData/playerEntity/stats/Luck", "/playerData/playerEntity/skills/Archery"],
    subtitle: "SaveData.txt from a Daggerfall Unity save folder (LocalLow\\Daggerfall Workshop\\Daggerfall Unity\\Saves\\<name>-<n>). Gold carried, level, health, the eight attributes, all 35 skills, resistances, reputations and the gold in each region's bank. Attributes and skills run 0 to 100.",
    tiles: &[("Gold carried", "/playerData/playerEntity/goldPieces"), ("Level", "/playerData/playerEntity/level")],
    sections: &[
        Section { title: "Character", rows: &[("Gold carried", "/playerData/playerEntity/goldPieces"), ("Level", "/playerData/playerEntity/level"), ("Current health", "/playerData/playerEntity/currentHealth"), ("Maximum health", "/playerData/playerEntity/maxHealth"), ("Current magicka", "/playerData/playerEntity/currentMagicka"), ("Current fatigue", "/playerData/playerEntity/currentFatigue")] },
        Section { title: "Attributes", rows: &[("Strength", "/playerData/playerEntity/stats/Strength"), ("Intelligence", "/playerData/playerEntity/stats/Intelligence"), ("Willpower", "/playerData/playerEntity/stats/Willpower"), ("Agility", "/playerData/playerEntity/stats/Agility"), ("Endurance", "/playerData/playerEntity/stats/Endurance"), ("Personality", "/playerData/playerEntity/stats/Personality"), ("Speed", "/playerData/playerEntity/stats/Speed"), ("Luck", "/playerData/playerEntity/stats/Luck")] },
        Section { title: "Skills", rows: &[("Medical", "/playerData/playerEntity/skills/Medical"), ("Etiquette", "/playerData/playerEntity/skills/Etiquette"), ("Streetwise", "/playerData/playerEntity/skills/Streetwise"), ("Jumping", "/playerData/playerEntity/skills/Jumping"), ("Orcish", "/playerData/playerEntity/skills/Orcish"), ("Harpy", "/playerData/playerEntity/skills/Harpy"), ("Giantish", "/playerData/playerEntity/skills/Giantish"), ("Dragonish", "/playerData/playerEntity/skills/Dragonish"), ("Nymph", "/playerData/playerEntity/skills/Nymph"), ("Daedric", "/playerData/playerEntity/skills/Daedric"), ("Spriggan", "/playerData/playerEntity/skills/Spriggan"), ("Centaurian", "/playerData/playerEntity/skills/Centaurian"), ("Impish", "/playerData/playerEntity/skills/Impish"), ("Lockpicking", "/playerData/playerEntity/skills/Lockpicking"), ("Mercantile", "/playerData/playerEntity/skills/Mercantile"), ("Pickpocket", "/playerData/playerEntity/skills/Pickpocket"), ("Stealth", "/playerData/playerEntity/skills/Stealth"), ("Swimming", "/playerData/playerEntity/skills/Swimming"), ("Climbing", "/playerData/playerEntity/skills/Climbing"), ("Backstabbing", "/playerData/playerEntity/skills/Backstabbing"), ("Dodging", "/playerData/playerEntity/skills/Dodging"), ("Running", "/playerData/playerEntity/skills/Running"), ("Destruction", "/playerData/playerEntity/skills/Destruction"), ("Restoration", "/playerData/playerEntity/skills/Restoration"), ("Illusion", "/playerData/playerEntity/skills/Illusion"), ("Alteration", "/playerData/playerEntity/skills/Alteration"), ("Thaumaturgy", "/playerData/playerEntity/skills/Thaumaturgy"), ("Mysticism", "/playerData/playerEntity/skills/Mysticism"), ("Short Blade", "/playerData/playerEntity/skills/ShortBlade"), ("Long Blade", "/playerData/playerEntity/skills/LongBlade"), ("Hand-to-Hand", "/playerData/playerEntity/skills/HandToHand"), ("Axe", "/playerData/playerEntity/skills/Axe"), ("Blunt Weapon", "/playerData/playerEntity/skills/BluntWeapon"), ("Archery", "/playerData/playerEntity/skills/Archery"), ("Critical Strike", "/playerData/playerEntity/skills/CriticalStrike")] },
        Section { title: "Resistances", rows: &[("Fire", "/playerData/playerEntity/resistances/Fire"), ("Frost", "/playerData/playerEntity/resistances/Frost"), ("Disease or poison", "/playerData/playerEntity/resistances/DiseaseOrPoison"), ("Shock", "/playerData/playerEntity/resistances/Shock"), ("Magic", "/playerData/playerEntity/resistances/Magic")] },
        Section { title: "Reputation", rows: &[("Commoners", "/playerData/playerEntity/reputationCommoners"), ("Merchants", "/playerData/playerEntity/reputationMerchants"), ("Nobility", "/playerData/playerEntity/reputationNobility"), ("Scholars", "/playerData/playerEntity/reputationScholars"), ("Underworld", "/playerData/playerEntity/reputationUnderworld")] },
    ],
    lists: &[],
    tables: &[Table {
        title: "Bank accounts (by region number)",
        path: "/bankAccounts",
        name: &["/regionIndex"],
        only: &[],
        cols: &[("Gold in bank", "/accountGold"), ("Loan", "/loanTotal")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Devil May Cry 1

const DMC1: Spec = Spec {
    name: "Devil May Cry HD Collection (Devil May Cry 1)",
    tab: "Slots",
    accent: (200, 40, 50),
    kind: Some(crate::fmt::dmc1::KIND),
    markers: &[],
    subtitle: "dmc1.sav from Steam\\userdata\\<account>\\631510\\remote: orbs and play time of each used slot.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[Table {
        title: "Slots",
        path: "",
        name: &[],
        only: &["Slot 1", "Slot 2", "Slot 3", "Slot 4", "Slot 5", "Slot 6", "Slot 7", "Slot 8", "Slot 9", "Slot 10"],
        cols: &[("Red orbs", "/Red orbs"), ("Yellow orbs", "/Yellow orbs"), ("Blue orbs", "/Blue orbs"), ("Play time (frames)", "/Play time (frames, 60 per second)")],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Mewgenics

const MEWGENICS: Spec = Spec {
    name: "Mewgenics",
    tab: "House",
    accent: (225, 150, 90),
    kind: Some("SQLite database"),
    markers: &["/properties/=house_gold/data", "/cats"],
    subtitle: "steamcampaign01.sav from %APPDATA%\\Glaiel Games\\Mewgenics\\<Steam id>\\saves (a SQLite database). The cats' own data is kept as it is.",
    tiles: &[("House gold", "/properties/=house_gold/data"), ("House food", "/properties/=house_food/data"), ("Day", "/properties/=current_day/data")],
    sections: &[Section {
        title: "House and adventure",
        rows: &[
            ("House gold", "/properties/=house_gold/data"), ("House food", "/properties/=house_food/data"), ("Current day", "/properties/=current_day/data"),
            ("Adventure coins", "/properties/=adventure_coins/data"), ("Adventure food", "/properties/=adventure_food/data"),
            ("Storage upgrades", "/properties/=house_storage_upgrades/data"), ("Blank collars", "/properties/=blank_collars/data"),
            ("Completion (%)", "/properties/=save_file_percent/data"),
        ],
    }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Deltatraveler

const DELTATRAVELER: Spec = Spec {
    name: "Deltatraveler",
    tab: "Party",
    accent: (90, 160, 230),
    kind: Some(".NET BinaryFormatter (SAVEFile)"),
    markers: &["/gold", "/exp", "/susieActive"],
    subtitle: "A SAVEn.sav (a .NET BinaryFormatter file; it is unpacked and written back for you). Name, gold, experience, play time, party members, and the item, weapon and armor ids. Layout from BenjaminUrquhart's DeltatravelerSaveEditor.",
    tiles: &[("Gold", "/gold"), ("EXP", "/exp")],
    sections: &[
        Section { title: "Save", rows: &[("Name", "/name"), ("Gold", "/gold"), ("EXP", "/exp"), ("Play time", "/playTime"), ("Zone", "/zone"), ("Deaths", "/deaths"), ("Susie in party", "/susieActive"), ("Noelle in party", "/noelleActive")] },
        Section { title: "Items (ids)", rows: &[("Item 1", "/items/0"), ("Item 2", "/items/1"), ("Item 3", "/items/2"), ("Item 4", "/items/3"), ("Item 5", "/items/4"), ("Item 6", "/items/5"), ("Item 7", "/items/6"), ("Item 8", "/items/7")] },
        Section { title: "Weapons and armor (ids per party member)", rows: &[("Weapon", "/weapon/0"), ("Weapon", "/weapon/1"), ("Weapon", "/weapon/2"), ("Armor", "/armor/0"), ("Armor", "/armor/1"), ("Armor", "/armor/2")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ The Elder Scrolls III / IV

const MORROWIND: Spec = Spec {
    name: "The Elder Scrolls III: Morrowind",
    tab: "Character",
    accent: (170, 140, 90),
    kind: Some(crate::fmt::morrowind::KIND),
    markers: &["/Attributes/Strength"],
    subtitle: "A Morrowind .ess save: level, gold, the eight attributes, all 27 skills, health, fatigue and magicka, and the inventory counts by item id. Checked on public saves, not yet tried in the game.",
    tiles: &[("Level", "/Character/Level"), ("Gold", "/Character/Gold")],
    sections: &[
        Section { title: "Character", rows: &[("Level", "/Character/Level"), ("Gold", "/Character/Gold")] },
        Section { title: "Vitals (now and full)", rows: &[("Health", "/Vitals/Health"), ("Full health", "/Vitals/Full health"), ("Fatigue", "/Vitals/Fatigue"), ("Full fatigue", "/Vitals/Full fatigue"), ("Magicka", "/Vitals/Magicka"), ("Full magicka", "/Vitals/Full magicka")] },
        Section { title: "Attributes", rows: &[("Strength", "/Attributes/Strength"), ("Intelligence", "/Attributes/Intelligence"), ("Willpower", "/Attributes/Willpower"), ("Agility", "/Attributes/Agility"), ("Speed", "/Attributes/Speed"), ("Endurance", "/Attributes/Endurance"), ("Personality", "/Attributes/Personality"), ("Luck", "/Attributes/Luck")] },
        Section { title: "Skills", rows: &[("Block", "/Skills/Block"), ("Armorer", "/Skills/Armorer"), ("Medium armor", "/Skills/Medium armor"), ("Heavy armor", "/Skills/Heavy armor"), ("Blunt weapon", "/Skills/Blunt weapon"), ("Long blade", "/Skills/Long blade"), ("Axe", "/Skills/Axe"), ("Spear", "/Skills/Spear"), ("Athletics", "/Skills/Athletics"), ("Enchant", "/Skills/Enchant"), ("Destruction", "/Skills/Destruction"), ("Alteration", "/Skills/Alteration"), ("Illusion", "/Skills/Illusion"), ("Conjuration", "/Skills/Conjuration"), ("Mysticism", "/Skills/Mysticism"), ("Restoration", "/Skills/Restoration"), ("Alchemy", "/Skills/Alchemy"), ("Unarmored", "/Skills/Unarmored"), ("Security", "/Skills/Security"), ("Sneak", "/Skills/Sneak"), ("Acrobatics", "/Skills/Acrobatics"), ("Light armor", "/Skills/Light armor"), ("Short blade", "/Skills/Short blade"), ("Marksman", "/Skills/Marksman"), ("Mercantile", "/Skills/Mercantile"), ("Speechcraft", "/Skills/Speechcraft"), ("Hand-to-hand", "/Skills/Hand-to-hand")] },
    ],
    lists: &[List { title: "Inventory (item id: count)", path: "/Inventory", shape: Shape::Map, names: &[], max: None }],
    tables: &[],
};

const OBLIVION: Spec = Spec {
    name: "The Elder Scrolls IV: Oblivion",
    tab: "Character",
    accent: (120, 150, 80),
    kind: Some(crate::fmt::oblivion::KIND),
    markers: &["/Attributes/Strength"],
    subtitle: "An Oblivion .ess save: level, gold, the eight attributes, all 21 skills and base health. Checked on public saves, not yet tried in the game.",
    tiles: &[("Level", "/Character/Level"), ("Gold", "/Character/Gold")],
    sections: &[
        Section { title: "Character", rows: &[("Level", "/Character/Level"), ("Gold", "/Character/Gold"), ("Base health", "/Character/Base health")] },
        Section { title: "Attributes", rows: &[("Strength", "/Attributes/Strength"), ("Intelligence", "/Attributes/Intelligence"), ("Willpower", "/Attributes/Willpower"), ("Agility", "/Attributes/Agility"), ("Speed", "/Attributes/Speed"), ("Endurance", "/Attributes/Endurance"), ("Personality", "/Attributes/Personality"), ("Luck", "/Attributes/Luck")] },
        Section { title: "Skills", rows: &[("Armorer", "/Skills/Armorer"), ("Athletics", "/Skills/Athletics"), ("Blade", "/Skills/Blade"), ("Block", "/Skills/Block"), ("Blunt", "/Skills/Blunt"), ("Hand to Hand", "/Skills/Hand to Hand"), ("Heavy Armor", "/Skills/Heavy Armor"), ("Alchemy", "/Skills/Alchemy"), ("Alteration", "/Skills/Alteration"), ("Conjuration", "/Skills/Conjuration"), ("Destruction", "/Skills/Destruction"), ("Illusion", "/Skills/Illusion"), ("Mysticism", "/Skills/Mysticism"), ("Restoration", "/Skills/Restoration"), ("Acrobatics", "/Skills/Acrobatics"), ("Light Armor", "/Skills/Light Armor"), ("Marksman", "/Skills/Marksman"), ("Mercantile", "/Skills/Mercantile"), ("Security", "/Skills/Security"), ("Sneak", "/Skills/Sneak"), ("Speechcraft", "/Skills/Speechcraft")] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ West of Loathing

const WOL: Spec = Spec {
    name: "West of Loathing",
    tab: "Cowpoke",
    accent: (196, 150, 70),
    kind: None,
    markers: &["/PLAYER/flags/meat", "/PLAYER/skills", "/WAA_ID"],
    subtitle: tx("save-<name>-<number>.json. Meat is the money; muscle, mysticality, moxie and every skill and perk are in the skills list below (skills are levels, perks are 1 or 0). The game stores most numbers as text, so type digits only."),
    tiles: &[(tx("Meat"), "/PLAYER/flags/meat"), (tx("Level"), "/PLAYER/flags/level"), (tx("XP"), "/PLAYER/flags/xp")],
    sections: &[
        Section {
            title: tx("The cowpoke"),
            rows: &[
                (tx("First name"), "/PLAYER/flags/firstname"), (tx("Last name"), "/PLAYER/flags/lastname"), (tx("Meat"), "/PLAYER/flags/meat"),
                (tx("Level"), "/PLAYER/flags/level"), (tx("XP"), "/PLAYER/flags/xp"), (tx("Total XP"), "/PLAYER/flags/totalxp"),
                ("Muscle XP", "/PLAYER/flags/musxp"), ("Mysticality XP", "/PLAYER/flags/mysxp"), ("Moxie XP", "/PLAYER/flags/moxxp"),
                ("Class (1 cow puncher, 2 beanslinger, 3 snake oiler)", "/PLAYER/flags/class"), (tx("Day"), "/PLAYER/flags/day"),
                (tx("Seconds played"), "/PLAYER/flags/secondsplayed"),
            ],
        },
        Section {
            title: tx("Resistances"),
            rows: &[
                ("Hot", "/PLAYER/flags/hotresist"), ("Cold", "/PLAYER/flags/coldresist"), ("Spooky", "/PLAYER/flags/spookyresist"),
                ("Stench", "/PLAYER/flags/stenchresist"), ("Sleaze", "/PLAYER/flags/sleazeresist"),
            ],
        },
    ],
    lists: &[List { title: tx("Stats, skills and perks"), path: "/PLAYER/skills", shape: Shape::Map, names: &[], max: None }],
    tables: &[Table { title: tx("Inventory"), path: "/PLAYER/items", name: &["/name"], cols: &[(tx("Quantity"), "/qty")], only: &[], names: &[] }],
};

// ------------------------------------------------------------------ Shape of Dreams

const SOD_PROFILE: Spec = Spec {
    name: "Shape of Dreams",
    tab: "Traveler",
    accent: (150, 120, 220),
    kind: None,
    markers: &["/root/stardust", "/root/spentStardust", "/root/preferredHero"],
    subtitle: tx("r_profile<id>.json in the QuickSave folder. Stardust is the meta currency; the unlocked items live in r_items.json, which the game encrypts (not opened). Mastery and records are in the r_stats file."),
    tiles: &[(tx("Stardust"), "/root/stardust"), (tx("Spent stardust"), "/root/spentStardust"), (tx("Reveries done"), "/root/completedReveries")],
    sections: &[
        Section {
            title: tx("Profile"),
            rows: &[
                (tx("Name"), "/root/name"), (tx("Stardust"), "/root/stardust"), (tx("Spent stardust"), "/root/spentStardust"),
                (tx("Reveries completed"), "/root/completedReveries"), (tx("Play time (minutes)"), "/root/totalPlayTimeMinutes"),
            ],
        },
        Section {
            title: tx("Preferences"),
            rows: &[
                ("Difficulty (diffNormal, …)", "/root/preferredDifficulty"), (tx("Traveler"), "/root/preferredHero"),
                (tx("Lobby name"), "/root/preferredLobbyName"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

const SOD_STATS: Spec = Spec {
    name: "Shape of Dreams (statistics)",
    tab: "Mastery",
    accent: (150, 120, 220),
    kind: None,
    markers: &["/root/total/masteryLevel", "/root/heroes"],
    subtitle: tx("r_stats<id>.json in the QuickSave folder: mastery and the record of every traveler."),
    tiles: &[(tx("Mastery level"), "/root/total/masteryLevel"), (tx("Mastery points"), "/root/total/currentMasteryPoints"), (tx("Wins"), "/root/total/wins")],
    sections: &[Section {
        title: tx("All travelers"),
        rows: &[
            (tx("Mastery level"), "/root/total/masteryLevel"), (tx("Mastery points to spend"), "/root/total/currentMasteryPoints"),
            (tx("Total mastery points"), "/root/total/totalMasteryPoints"), (tx("Wins"), "/root/total/wins"), (tx("Losses"), "/root/total/loses"),
            (tx("Runs played"), "/root/total/playCount"), (tx("Level ups"), "/root/total/levelUps"), (tx("Kills"), "/root/total/kills"),
            (tx("Deaths"), "/root/total/deaths"), (tx("Gold earned"), "/root/total/earnedGold"), ("Dream dust earned", "/root/total/earnedDreamDust"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: tx("Travelers"),
        path: "/root/heroes",
        name: &[],
        cols: &[
            (tx("Mastery level"), "/masteryLevel"), (tx("Mastery points"), "/currentMasteryPoints"), (tx("Total mastery points"), "/totalMasteryPoints"),
            (tx("Wins"), "/wins"), (tx("Losses"), "/loses"), (tx("Runs"), "/playCount"), (tx("Kills"), "/kills"),
        ],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Travellers Rest

const TRAVELLERS_REST: Spec = Spec {
    name: "Travellers Rest",
    tab: "Tavern",
    accent: (214, 150, 60),
    kind: Some(".NET BinaryFormatter (SaveData)"),
    markers: &["/money", "/ingredients", "/itemsInInventory"],
    subtitle: tx("SaveFile-<day>-<month>-<year>-<time>.save. Money is counted in copper (100 copper = 1 silver, 100 silver = 1 gold). Items are shown by their game id."),
    tiles: &[(tx("Money (copper)"), "/money")],
    sections: &[Section {
        title: tx("Tavern"),
        rows: &[
            (tx("Money (copper)"), "/money"), ("Crafting room upgrades", "/craftingRoomUpgradeNumber"), (tx("Tavern floor upgrades"), "/tavernFloorUpgradeNumber"),
            (tx("Cellar upgrades"), "/cellarUpgradeNumber"), ("Halloween candles", "/halloweenCandlesNum"),
        ],
    }],
    lists: &[
        List { title: tx("Inventory"), path: "/itemsInInventory", shape: Shape::Array { id_key: "/itemInstanceSave/itemID", qty_keys: &["stack"] }, names: &[], max: None },
        List { title: tx("Action bar"), path: "/itemsInActionBar", shape: Shape::Array { id_key: "/itemInstanceSave/itemID", qty_keys: &["stack"] }, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Youtubers Life

const YOUTUBERS_LIFE: Spec = Spec {
    name: "Youtubers Life",
    tab: "Youtuber",
    accent: (230, 60, 60),
    kind: Some("SQLite database"),
    markers: &["/savegame/0/money", "/youtuber/0/gaming_exp", "/channel/0/subscribers"],
    subtitle: tx("<name>_<date>.yls in the Saves folder (a SQLite database, written back as one). The first row of each table is yours; the other youtuber and channel rows are the game's rivals and friends."),
    tiles: &[(tx("Money"), "/savegame/0/money"), (tx("Subscribers"), "/channel/0/subscribers"), (tx("Views"), "/channel/0/views")],
    sections: &[
        Section {
            title: tx("Savegame"),
            rows: &[
                (tx("Money"), "/savegame/0/money"), (tx("House"), "/savegame/0/house"), (tx("Work hours"), "/savegame/0/work_hours"),
                (tx("Study hours"), "/savegame/0/study_hours"), (tx("Current date (days)"), "/savegame/0/current_date"),
            ],
        },
        Section {
            title: tx("Your channel"),
            rows: &[(tx("Name"), "/channel/0/name"), (tx("Subscribers"), "/channel/0/subscribers"), (tx("Views"), "/channel/0/views"), (tx("Status"), "/channel/0/status"), (tx("Videos"), "/channel/0/num_videos")],
        },
    ],
    lists: &[],
    tables: &[Table {
        title: tx("Youtubers"),
        path: "/youtuber",
        name: &["/name"],
        cols: &[
            (tx("Energy"), "/energy"), (tx("Hunger"), "/hunger"), (tx("Social life"), "/social_life"), (tx("Motivation"), "/motivation"),
            (tx("Scripting"), "/scripting"), ("Acting", "/acting"), (tx("Sound"), "/sound"), (tx("Editing"), "/editing"), (tx("Effects"), "/effects"),
            (tx("Gaming level"), "/gaming"), (tx("Cooking level"), "/cooking"), (tx("Life level"), "/life"), (tx("Music level"), "/music"), (tx("Fashion level"), "/fashion"),
            (tx("Gaming EXP"), "/gaming_exp"), (tx("Main level"), "/main_level"), (tx("Main EXP"), "/main_exp"), (tx("Perk points"), "/perk"),
        ],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Luck be a Landlord

const LUCK_LANDLORD: Spec = Spec {
    name: "Luck be a Landlord",
    tab: "Run",
    accent: (240, 190, 60),
    kind: None,
    markers: &["/~1root~1Main~1Coins/coins", "/~1root~1Main~1Pop-up Sprite~1Pop-up/removal_tokens"],
    subtitle: tx("LBAL.save (the run in progress). The save is one JSON object per line; each node of the run is shown under its path. Symbols and items are lists of names in the Tree."),
    tiles: &[
        (tx("Coins"), "/~1root~1Main~1Coins/coins"),
        (tx("Removal tokens"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/removal_tokens"),
        (tx("Reroll tokens"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/reroll_tokens"),
    ],
    sections: &[
        Section {
            title: tx("The run"),
            rows: &[
                (tx("Coins"), "/~1root~1Main~1Coins/coins"), (tx("Removal tokens"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/removal_tokens"),
                (tx("Reroll tokens"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/reroll_tokens"), (tx("Spins"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/spins"),
                (tx("Symbols to choose from"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/symbols_to_choose_from"),
                (tx("Current floor"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/current_floor"),
            ],
        },
        Section {
            title: tx("Rent and progress"),
            rows: &[
                (tx("Rent due (coins)"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/rent_values/0"), (tx("Spins until rent"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/rent_values/1"),
                (tx("Rent payments made"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/times_rent_paid"), (tx("Rent payments to make"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/times_to_pay_rent"),
                (tx("Highest unlocked floor"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/highest_unlocked_floor"), (tx("Total runs"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/total_runs"),
                (tx("Endless mode"), "/~1root~1Main~1Pop-up Sprite~1Pop-up/endless_mode"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Touhou Luna Nights

const LUNA_NIGHTS: Spec = Spec {
    name: "Touhou Luna Nights",
    tab: "Sakuya",
    accent: (110, 130, 230),
    kind: None,
    markers: &["/118", "/117", "/102", "/134"],
    subtitle: tx("gameN.sav is a list of 1,000 values, one per line (stored as base64 text); the screen names the lines worked out by the Luna Nights save editor. Abilities and keys are owned when the value equals their line number (80 to 84, 90 to 94) and 0 when not."),
    tiles: &[(tx("Gold"), "/118"), (tx("Level"), "/117"), (tx("HP upgrades"), "/104")],
    sections: &[
        Section {
            title: "Sakuya",
            rows: &[
                (tx("Gold"), "/118"), (tx("Level (upgrade number)"), "/117"), (tx("Experience"), "/106"), (tx("HP upgrades"), "/104"), (tx("MP upgrades"), "/105"),
                (tx("Knife upgrades"), "/108"), (tx("Clock upgrades"), "/109"), (tx("Stage"), "/102"), (tx("Save time"), "/107"),
            ],
        },
        Section {
            title: tx("Gems"),
            rows: &[
                ("Amethyst", "/110"), ("Turquoise", "/111"), ("Topaz", "/112"), ("Ruby", "/113"), ("Sapphire", "/114"), ("Emerald", "/115"), ("Diamond", "/116"),
            ],
        },
        Section {
            title: tx("Abilities and keys"),
            rows: &[
                ("Slide (80)", "/80"), ("Double jump (81)", "/81"), ("Wall grip (82)", "/82"), ("Screw (83)", "/83"), ("Dash (84)", "/84"),
                ("Red key (90)", "/90"), ("Yellow key (91)", "/91"), ("Green key (92)", "/92"), ("Blue key (93)", "/93"), ("Purple key (94)", "/94"), ("Ice magatama (95)", "/95"),
            ],
        },
        Section {
            title: tx("Shop and trash cans"),
            rows: &[(tx("Clock upgrades bought"), "/134"), (tx("Knife upgrades bought"), "/135"), (tx("Trash cans opened"), "/120")],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ EVERSPACE

const EVERSPACE1: Spec = Spec {
    name: "EVERSPACE",
    tab: "Pilot",
    accent: (70, 170, 230),
    kind: Some("Unreal Engine save (CurrentPlayerData)"),
    markers: &["/DNAFragments", "/NumRuns", "/Perks/0/Level"],
    subtitle: tx("CurrentPlayerSave.sav: the pilot's permanent progress. DNA fragments buy perks in the Lab; every perk shows its level and whether it is still locked. Credits, resources and the ship of a run are in the run's own save files."),
    tiles: &[(tx("DNA fragments"), "/DNAFragments"), (tx("Runs"), "/NumRuns"), (tx("Wins"), "/NumWins")],
    sections: &[
        Section {
            title: tx("Pilot"),
            rows: &[
                (tx("Pilot name"), "/PlayerName"), (tx("DNA fragments"), "/DNAFragments"), (tx("Flashback progression"), "/FlashbackProgression"),
                ("Difficulty (EDifficulty::Easy, …)", "/LastDifficulty"), (tx("Highest sector"), "/MaxSector"),
            ],
        },
        Section {
            title: tx("Records"),
            rows: &[
                (tx("Runs"), "/NumRuns"), (tx("Wins"), "/NumWins"), (tx("Lifetime kills"), "/LifeTimeKills"), (tx("Most kills in a run"), "/MaxKills"),
                (tx("Most credits held"), "/MaxCredits"), (tx("Credits picked up"), "/TotalCreditsPickedUp"), (tx("Play time (seconds)"), "/TotalPlayTime"),
            ],
        },
    ],
    lists: &[],
    tables: &[Table {
        title: tx("Perks"),
        path: "/Perks",
        name: &["/Name"],
        cols: &[(tx("Locked"), "/IsLocked"), (tx("Level"), "/Level"), (tx("Sub-level"), "/SubLevel")],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Streets of Rogue

const STREETS_OF_ROGUE: Spec = Spec {
    name: "Streets of Rogue",
    tab: "Unlocks",
    accent: (230, 150, 40),
    kind: Some(".NET BinaryFormatter (UnlockSaveData)"),
    markers: &["/nuggets", "/totalWins", "/unlocks/0/unlockName"],
    subtitle: tx("GameUnlocks.dat in Documents\\Streets of Rogue\\CloudData. Nuggets buy unlocks; every agent, item, trait and floor is listed with whether it is unlocked and what it costs. Close the game first."),
    tiles: &[(tx("Nuggets"), "/nuggets"), (tx("Wins"), "/totalWins"), (tx("Games played"), "/totalGamesPlayed")],
    sections: &[Section {
        title: tx("Progress"),
        rows: &[
            (tx("Nuggets"), "/nuggets"), (tx("Wins"), "/totalWins"), (tx("Deaths"), "/totalDeaths"), (tx("Games played"), "/totalGamesPlayed"),
            (tx("Tutorial finished"), "/finishedTutorial"), (tx("Game version"), "/currentVersion"),
        ],
    }],
    lists: &[],
    tables: &[Table {
        title: tx("Unlocks"),
        path: "/unlocks",
        name: &["/unlockName"],
        cols: &[
            (tx("Type"), "/unlockType"), (tx("Unlocked"), "/unlocked"), (tx("Available now"), "/nowAvailable"), (tx("Cost"), "/cost"), (tx("Cost 2"), "/cost2"), (tx("Cost 3"), "/cost3"), (tx("Progress"), "/progressCount"),
        ],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ Pizza Tower
const PIZZA_TOWER: Spec = Spec {
    name: "Pizza Tower",
    tab: "Tower",
    accent: (214, 92, 54),
    kind: Some("INI / config"),
    markers: &["/Game/percent", "/Highscore", "/Ranks"],
    subtitle: "saveData.ini (the file is a GameMaker ini; values are written as text like 1.000000). Scores are points, ranks are the letters d, c, b, a, s and p, treasure is 1 when found and secrets found are 0 to 3.",
    tiles: &[("Completion %", "/Game/percent"), ("Enemies killed", "/Game/enemies"), ("Minutes played", "/Game/minutes")],
    sections: &[
        Section {
            title: "Game",
            rows: &[
                ("Completion %", "/Game/percent"),
                ("Enemies killed", "/Game/enemies"),
                ("Damage taken", "/Game/damage"),
                ("Minutes played", "/Game/minutes"),
                ("Seconds played", "/Game/seconds"),
                ("Final rank", "/Game/finalrank"),
            ],
        },
        Section {
            title: "Level scores",
            rows: &[
                ("Entrance", "/Highscore/entrance"),
                ("Medieval", "/Highscore/medieval"),
                ("Ruin", "/Highscore/ruin"),
                ("Dungeon", "/Highscore/dungeon"),
                ("Graveyard", "/Highscore/graveyard"),
                ("Farm", "/Highscore/farm"),
                ("Saloon", "/Highscore/saloon"),
                ("Pineapple Beach", "/Highscore/plage"),
                ("Forest", "/Highscore/forest"),
                ("Space", "/Highscore/space"),
                ("Golf", "/Highscore/minigolf"),
                ("Street", "/Highscore/street"),
                ("Sewer", "/Highscore/sewer"),
                ("Industrial", "/Highscore/industrial"),
                ("Freezer", "/Highscore/freezer"),
                ("Chateau", "/Highscore/chateau"),
                ("Kid's Party", "/Highscore/kidsparty"),
                ("War", "/Highscore/war"),
                ("Badland", "/Highscore/badland"),
                ("Exit", "/Highscore/exit"),
                ("Secret World", "/Highscore/secretworld"),
                ("Pepperman", "/Highscore/b_pepperman"),
                ("The Vigilante", "/Highscore/b_vigilante"),
                ("The Noise", "/Highscore/b_noise"),
                ("Fake Peppino", "/Highscore/b_fakepep"),
            ],
        },
        Section {
            title: "Level ranks",
            rows: &[
                ("Entrance", "/Ranks/entrance"),
                ("Medieval", "/Ranks/medieval"),
                ("Ruin", "/Ranks/ruin"),
                ("Dungeon", "/Ranks/dungeon"),
                ("Graveyard", "/Ranks/graveyard"),
                ("Farm", "/Ranks/farm"),
                ("Saloon", "/Ranks/saloon"),
                ("Pineapple Beach", "/Ranks/plage"),
                ("Forest", "/Ranks/forest"),
                ("Space", "/Ranks/space"),
                ("Golf", "/Ranks/minigolf"),
                ("Street", "/Ranks/street"),
                ("Sewer", "/Ranks/sewer"),
                ("Industrial", "/Ranks/industrial"),
                ("Freezer", "/Ranks/freezer"),
                ("Chateau", "/Ranks/chateau"),
                ("Kid's Party", "/Ranks/kidsparty"),
                ("War", "/Ranks/war"),
                ("Badland", "/Ranks/badland"),
                ("Exit", "/Ranks/exit"),
                ("Secret World", "/Ranks/secretworld"),
                ("Pepperman", "/Ranks/b_pepperman"),
                ("The Vigilante", "/Ranks/b_vigilante"),
                ("The Noise", "/Ranks/b_noise"),
                ("Fake Peppino", "/Ranks/b_fakepep"),
            ],
        },
        Section {
            title: "Treasure found",
            rows: &[
                ("Entrance", "/Treasure/entrance"),
                ("Medieval", "/Treasure/medieval"),
                ("Ruin", "/Treasure/ruin"),
                ("Dungeon", "/Treasure/dungeon"),
                ("Graveyard", "/Treasure/graveyard"),
                ("Farm", "/Treasure/farm"),
                ("Saloon", "/Treasure/saloon"),
                ("Pineapple Beach", "/Treasure/plage"),
                ("Forest", "/Treasure/forest"),
                ("Space", "/Treasure/space"),
                ("Golf", "/Treasure/minigolf"),
                ("Street", "/Treasure/street"),
                ("Sewer", "/Treasure/sewer"),
                ("Industrial", "/Treasure/industrial"),
                ("Freezer", "/Treasure/freezer"),
                ("Chateau", "/Treasure/chateau"),
                ("Kid's Party", "/Treasure/kidsparty"),
                ("War", "/Treasure/war"),
                ("Badland", "/Treasure/badland"),
                ("Exit", "/Treasure/exit"),
                ("Secret World", "/Treasure/secretworld"),
            ],
        },
        Section {
            title: "Secrets found",
            rows: &[
                ("Entrance", "/Secret/entrance"),
                ("Medieval", "/Secret/medieval"),
                ("Ruin", "/Secret/ruin"),
                ("Dungeon", "/Secret/dungeon"),
                ("Graveyard", "/Secret/graveyard"),
                ("Farm", "/Secret/farm"),
                ("Saloon", "/Secret/saloon"),
                ("Pineapple Beach", "/Secret/plage"),
                ("Forest", "/Secret/forest"),
                ("Space", "/Secret/space"),
                ("Golf", "/Secret/minigolf"),
                ("Street", "/Secret/street"),
                ("Sewer", "/Secret/sewer"),
                ("Industrial", "/Secret/industrial"),
                ("Freezer", "/Secret/freezer"),
                ("Chateau", "/Secret/chateau"),
                ("Kid's Party", "/Secret/kidsparty"),
                ("War", "/Secret/war"),
                ("Badland", "/Secret/badland"),
                ("Exit", "/Secret/exit"),
                ("Secret World", "/Secret/secretworld"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Teardown
const TEARDOWN: Spec = Spec {
    name: "Teardown",
    tab: "Crew",
    accent: (235, 160, 40),
    kind: Some("XML"),
    markers: &["/registry/savegame/cash/@value", "/registry/savegame/tool"],
    subtitle: "savegame.xml from Documents\\Teardown (inside the game's data folder). Cash, tool ammunition and tool upgrades, and each mission's score. Upgrades beyond what the shop sells can look odd in the game.",
    tiles: &[("Cash", "/registry/savegame/cash/@value"), ("Hub score", "/registry/savegame/hub/score/@value")],
    sections: &[
        Section {
            title: "Cash and progress",
            rows: &[
                ("Cash", "/registry/savegame/cash/@value"),
                ("Hub score", "/registry/savegame/hub/score/@value"),
            ],
        },
        Section {
            title: "Ammunition",
            rows: &[
                ("Blowtorch", "/registry/savegame/tool/blowtorch/ammo/@value"),
                ("Shotgun", "/registry/savegame/tool/shotgun/ammo/@value"),
                ("Plank", "/registry/savegame/tool/plank/ammo/@value"),
                ("Pipe bomb", "/registry/savegame/tool/pipebomb/ammo/@value"),
                ("Gun", "/registry/savegame/tool/gun/ammo/@value"),
                ("Bomb", "/registry/savegame/tool/bomb/ammo/@value"),
                ("Rocket", "/registry/savegame/tool/rocket/ammo/@value"),
            ],
        },
        Section {
            title: "Range, damage and size upgrades",
            rows: &[
                ("Shotgun range", "/registry/savegame/tool/shotgun/range/@value"),
                ("Shotgun damage", "/registry/savegame/tool/shotgun/damage/@value"),
                ("Gun range", "/registry/savegame/tool/gun/range/@value"),
                ("Gun damage", "/registry/savegame/tool/gun/damage/@value"),
                ("Pipe bomb damage", "/registry/savegame/tool/pipebomb/damage/@value"),
                ("Bomb damage", "/registry/savegame/tool/bomb/damage/@value"),
                ("Rocket damage", "/registry/savegame/tool/rocket/damage/@value"),
                ("Plank width", "/registry/savegame/tool/plank/width/@value"),
                ("Plank length", "/registry/savegame/tool/plank/length/@value"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Wolcen: Lords of Mayhem
const WOLCEN: Spec = Spec {
    name: "Wolcen: Lords of Mayhem",
    tab: "Character",
    accent: (200, 40, 50),
    kind: None,
    markers: &["/Stats/Gold", "/Stats/Level", "/CharacterId"],
    subtitle: "A character file from Saved Games\\wolcen\\savegames\\characters (plain JSON). Gold, level, experience, the four attributes and the points to spend. Items, the skill bar and passive skills are in the Tree; the shared chest is its own file.",
    tiles: &[("Gold", "/Stats/Gold"), ("Level", "/Stats/Level"), ("Passive skill points", "/Stats/PassiveSkillPoints")],
    sections: &[
        Section {
            title: "Stats",
            rows: &[
                ("Level", "/Stats/Level"),
                ("Experience", "/Stats/CurrentXP"),
                ("Gold", "/Stats/Gold"),
                ("Primordial affinity", "/Stats/PrimordialAffinity"),
                ("Passive skill points", "/Stats/PassiveSkillPoints"),
                ("Attribute points to spend", "/Stats/RemainingStatsPoints"),
            ],
        },
        Section {
            title: "Attributes",
            rows: &[
                ("Strength", "/Stats/Strength"),
                ("Agility", "/Stats/Agility"),
                ("Constitution", "/Stats/Constitution"),
                ("Power", "/Stats/Power"),
            ],
        },
        Section {
            title: "Character",
            rows: &[
                ("Difficulty mode", "/DifficultyMode"),
                ("League", "/League"),
                ("Archetype", "/CharacterCustomization/Archetype"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Blasphemous
const BLASPHEMOUS: Spec = Spec {
    name: "Blasphemous",
    tab: "Penitent One",
    accent: (150, 20, 30),
    kind: None,
    markers: &["/commonElements/ID_STATS/currentValues/Purge", "/commonElements/ID_PERSISTENT_MANAGER"],
    subtitle: "savegame_N.save (base64-wrapped JSON, opened and closed for you). Tears of Atonement, current health and fervour, the upgrade levels of each stat, flask count and guilt. Owned rosary beads, prayers, relics and sword hearts are lists of ids in the Tree.",
    tiles: &[("Tears of Atonement", "/commonElements/ID_STATS/currentValues/Purge"), ("Health", "/commonElements/ID_STATS/currentValues/Life"), ("Fervour", "/commonElements/ID_STATS/currentValues/Fervour")],
    sections: &[
        Section {
            title: "Current values",
            rows: &[
                ("Tears of Atonement", "/commonElements/ID_STATS/currentValues/Purge"),
                ("Health", "/commonElements/ID_STATS/currentValues/Life"),
                ("Fervour", "/commonElements/ID_STATS/currentValues/Fervour"),
                ("Bile flasks", "/commonElements/ID_STATS/currentValues/Flask"),
                ("Attack speed", "/commonElements/ID_STATS/currentValues/AttackSpeed"),
            ],
        },
        Section {
            title: "Permanent upgrades",
            rows: &[
                ("Health", "/commonElements/ID_STATS/permanetBonus/Life"),
                ("Fervour", "/commonElements/ID_STATS/permanetBonus/Fervour"),
                ("Strength", "/commonElements/ID_STATS/permanetBonus/Strength"),
                ("Defence", "/commonElements/ID_STATS/permanetBonus/Defense"),
                ("Resistance", "/commonElements/ID_STATS/permanetBonus/Resistance"),
                ("Speed", "/commonElements/ID_STATS/permanetBonus/Speed"),
                ("Agility", "/commonElements/ID_STATS/permanetBonus/Agility"),
                ("Flask", "/commonElements/ID_STATS/permanetBonus/Flask"),
                ("Fervour strength", "/commonElements/ID_STATS/permanetBonus/FervourStrength"),
                ("Dash cooldown", "/commonElements/ID_STATS/permanetBonus/DashCooldown"),
            ],
        },
        Section {
            title: "Progress",
            rows: &[
                ("Tears collected (total)", "/commonElements/ID_PERSISTENT_MANAGER/Purge"),
                ("Play time (seconds)", "/commonElements/ID_PERSISTENT_MANAGER/Time"),
                ("Completion %", "/commonElements/ID_PERSISTENT_MANAGER/Percent"),
                ("New Game+ upgrades", "/commonElements/ID_PERSISTENT_MANAGER/NewGamePlusUpgrades"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Noita
const NOITA: Spec = Spec {
    name: "Noita",
    tab: "Player",
    accent: (180, 60, 200),
    kind: Some("XML"),
    markers: &["/Entity/WalletComponent/@money", "/Entity/DamageModelComponent/@hp"],
    subtitle: "player.xml from the save00 folder (close the game first). Gold and health; the game shows health 25 times the stored value. Wands, spells and perks are entities inside the file, in the Tree.",
    tiles: &[("Gold", "/Entity/WalletComponent/@money"), ("Health (stored)", "/Entity/DamageModelComponent/@hp"), ("Max health (stored)", "/Entity/DamageModelComponent/@max_hp")],
    sections: &[
        Section {
            title: "Wallet",
            rows: &[
                ("Gold", "/Entity/WalletComponent/@money"),
                ("Gold spent", "/Entity/WalletComponent/@money_spent"),
                ("Gold shown by the HUD", "/Entity/InventoryGuiComponent/@wallet_money_target"),
            ],
        },
        Section {
            title: "Health (shown value = stored x 25)",
            rows: &[
                ("Health", "/Entity/DamageModelComponent/@hp"),
                ("Max health", "/Entity/DamageModelComponent/@max_hp"),
                ("Max health cap", "/Entity/DamageModelComponent/@max_hp_cap"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Last Epoch
const LAST_EPOCH: Spec = Spec {
    name: "Last Epoch",
    tab: "Character",
    accent: (120, 90, 200),
    kind: None,
    markers: &["/characterName", "/level", "/currentExp"],
    subtitle: "A 1CHARACTERSLOT file from Last Epoch's Saves folder (offline characters; the EPOCH header is kept). Level, experience, deaths and respecs. Items, passives and the monolith are in the Tree.",
    tiles: &[("Level", "/level"), ("Experience", "/currentExp"), ("Deaths", "/deaths")],
    sections: &[
        Section {
            title: "Character",
            rows: &[
                ("Name", "/characterName"),
                ("Level", "/level"),
                ("Experience", "/currentExp"),
                ("Class id", "/characterClass"),
                ("Deaths", "/deaths"),
                ("Respecs", "/respecs"),
                ("Hardcore", "/hardcore"),
                ("Masochist", "/masochist"),
            ],
        },
        Section {
            title: "Progress",
            rows: &[
                ("Portal unlocked", "/portalUnlocked"),
                ("Reached town", "/reachedTown"),
                ("Seqence number", "/seqNo"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Enter the Gungeon
const GUNGEON: Spec = Spec {
    name: "Enter the Gungeon",
    tab: "Gungeoneers",
    accent: (230, 120, 40),
    kind: None,
    markers: &["/m_characterStats", "/m_flags"],
    subtitle: "SlotA.save (a version line, then JSON XORed with the game's own key; both are handled for you). Each Gungeoneer's statistics, including Hegemony credits. Unlocked guns, items and flags are in the Tree.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[
        Table { title: "Gungeoneers", path: "/m_characterStats", name: &[], only: &[], cols: &[("Hegemony credits", "/stats/META_CURRENCY"), ("Runs", "/stats/NUMBER_ATTEMPTS"), ("Deaths", "/stats/NUMBER_DEATHS"), ("Enemies killed", "/stats/ENEMIES_KILLED"), ("Money collected", "/stats/TOTAL_MONEY_COLLECTED"), ("Secret rooms", "/stats/SECRET_ROOMS_FOUND"), ("Time played (s)", "/stats/TIME_PLAYED"), ("Credits spent", "/stats/META_CURRENCY_SPENT_AT_META_SHOP"), ("Most keys held", "/maxima/MOST_KEYS_HELD"), ("Most blanks held", "/maxima/MOST_BLANKS_HELD"), ("Highest curse", "/maxima/HIGHEST_CURSE_LEVEL"), ("Cleared Keep", "/stats/TIMES_CLEARED_CASTLE"), ("Cleared Gungeon Proper", "/stats/TIMES_CLEARED_GUNGEON"), ("Cleared Mines", "/stats/TIMES_CLEARED_MINES"), ("Cleared Hollow", "/stats/TIMES_CLEARED_CATACOMBS"), ("Cleared Forge", "/stats/TIMES_CLEARED_FORGE")], names: &[("Soldier", "Marine"), ("Guide", "Hunter"), ("Pilot", "Pilot"), ("Convict", "Convict"), ("Robot", "Robot"), ("Bullet", "Bullet"), ("Cultist", "Cultist"), ("Paradox", "Paradox"), ("Gunslinger", "Gunslinger")] },
    ],
};

// ------------------------------------------------------------------ Disco Elysium
const DISCO: Spec = Spec {
    name: "Disco Elysium",
    tab: "Detective",
    accent: (214, 170, 60),
    kind: None,
    markers: &["/playerCharacter/Money", "/characterSheet/psyche/value"],
    subtitle: "A save from Documents\\ZAUM Studio\\Disco Elysium\\SaveGames (a .ntwtf.zip; its big .2nd.ntwtf.json is edited and the rest of the zip is kept). Money, level, experience, skill points, the four attributes and the skills as the game keeps them. Skills the save lists under different names stay in the Tree.",
    tiles: &[("Money", "/playerCharacter/Money"), ("Level", "/playerCharacter/Level"), ("Skill points", "/playerCharacter/SkillPoints")],
    sections: &[
        Section {
            title: "Detective",
            rows: &[
                ("Money (centims)", "/playerCharacter/Money"),
                ("Level", "/playerCharacter/Level"),
                ("Experience", "/playerCharacter/XpAmount"),
                ("Skill points to spend", "/playerCharacter/SkillPoints"),
                ("Health (Endurance pool)", "/playerCharacter/healingPools/ENDURANCE"),
                ("Morale (Volition pool)", "/playerCharacter/healingPools/VOLITION"),
            ],
        },
        Section {
            title: "Attributes",
            rows: &[
                ("Intellect", "/characterSheet/intellect/value"),
                ("Psyche", "/characterSheet/psyche/value"),
                ("Physique", "/characterSheet/fysique/value"),
                ("Motorics", "/characterSheet/motorics/value"),
            ],
        },
        Section {
            title: "Skills",
            rows: &[
                ("Logic", "/characterSheet/logic/value"),
                ("Encyclopedia", "/characterSheet/encyclopedia/value"),
                ("Rhetoric", "/characterSheet/rhetoric/value"),
                ("Drama", "/characterSheet/drama/value"),
                ("Conceptualization", "/characterSheet/conceptualization/value"),
                ("Visual Calculus", "/characterSheet/visualCalculus/value"),
                ("Volition", "/characterSheet/volition/value"),
                ("Inland Empire", "/characterSheet/inlandEmpire/value"),
                ("Empathy", "/characterSheet/empathy/value"),
                ("Authority", "/characterSheet/authority/value"),
                ("Esprit de Corps", "/characterSheet/espritDeCorps/value"),
                ("Suggestion", "/characterSheet/suggestion/value"),
                ("Endurance", "/characterSheet/endurance/value"),
                ("Pain Threshold", "/characterSheet/painThreshold/value"),
                ("Physical Instrument", "/characterSheet/physicalInstrument/value"),
                ("Electrochemistry", "/characterSheet/electrochemistry/value"),
                ("Shivers", "/characterSheet/shivers/value"),
                ("Half Light", "/characterSheet/halfLight/value"),
                ("Hand/Eye Coordination", "/characterSheet/handEyeCoordination/value"),
                ("Perception", "/characterSheet/perception/value"),
                ("Reaction Speed", "/characterSheet/reaction/value"),
                ("Savoir Faire", "/characterSheet/savoirFaire/value"),
                ("Interfacing", "/characterSheet/interfacing/value"),
                ("Composure", "/characterSheet/composure/value"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Clicker Heroes
const CLICKER_HEROES: Spec = Spec {
    name: "Clicker Heroes",
    tab: "Clicker",
    accent: (240, 180, 40),
    kind: None,
    markers: &["/heroSouls", "/heroCollection/heroes", "/rubies"],
    subtitle: "The Steam version's save (a text key, then base64 of raw-deflated JSON; both handled for you). Gold, rubies, hero souls, ancient souls, zones and each hero's and outsider's level. Numbers the game keeps as text (gold, primal souls) are edited as text.",
    tiles: &[("Hero souls", "/heroSouls"), ("Rubies", "/rubies"), ("Ancient souls", "/ancientSouls")],
    sections: &[
        Section {
            title: "Currencies",
            rows: &[
                ("Gold", "/gold"),
                ("Rubies", "/rubies"),
                ("Hero souls", "/heroSouls"),
                ("Ancient souls", "/ancientSouls"),
                ("Ancient souls (total)", "/ancientSoulsTotal"),
                ("Primal souls", "/primalSouls"),
                ("Forge coals", "/forgeCoals"),
                ("Candy canes", "/candyCanes"),
            ],
        },
        Section {
            title: "Progress",
            rows: &[
                ("Highest zone finished", "/highestFinishedZone"),
                ("Highest zone before transcending", "/pretranscendentHighestFinishedZone"),
                ("Highest zone since transcending", "/transcendentHighestFinishedZone"),
                ("Ascensions", "/numWorldResets"),
                ("Transcensions", "/numberOfTranscensions"),
                ("Transcended", "/transcendent"),
            ],
        },
        Section {
            title: "Multipliers",
            rows: &[
                ("Click damage", "/clickMultiplier"),
                ("All damage", "/allDpsMultiplier"),
                ("Gold", "/goldMultiplier"),
                ("Critical click", "/criticalMultiplier"),
            ],
        },
    ],
    lists: &[],
    tables: &[
        Table { title: "Heroes", path: "/heroCollection/heroes", name: &[], only: &[], cols: &[("Level", "/level"), ("Epic level", "/epicLevel"), ("Locked", "/locked")], names: &[] },
    
        Table { title: "Outsiders", path: "/outsiders/outsiders", name: &[], only: &[], cols: &[("Level", "/level"), ("Ancient souls spent", "/spentAncientSouls")], names: &[] },
    ],
};

// ------------------------------------------------------------------ The Binding of Isaac: Rebirth
const ISAAC: Spec = Spec {
    name: "The Binding of Isaac: Rebirth",
    tab: "Basement",
    accent: (200, 60, 60),
    kind: Some(crate::fmt::isaac::KIND),
    markers: &["/Counters/Eden tokens", "/Counters/Deaths"],
    subtitle: "persistentgamedata1.dat (Rebirth), ab_ / abp_ (Afterbirth, Afterbirth+) or rep_ / rep+ (Repentance), in the Steam remote folder. Counters, achievements, collectibles seen and challenges; each unlock is 0 (locked) or 1 (unlocked), numbered as in the game's own lists. The checksum is written again on save.",
    tiles: &[("Eden tokens", "/Counters/Eden tokens"), ("Deaths", "/Counters/Deaths"), ("Win streak", "/Counters/Win streak")],
    sections: &[
        Section {
            title: "Counters",
            rows: &[
                ("Mom kills", "/Counters/Mom kills"),
                ("Rocks broken", "/Counters/Rocks broken"),
                ("Tinted rocks broken", "/Counters/Tinted rocks broken"),
                ("Poop destroyed", "/Counters/Poop destroyed"),
                ("Deaths", "/Counters/Deaths"),
                ("Shopkeepers destroyed", "/Counters/Shopkeepers destroyed"),
                ("Shell game plays", "/Counters/Shell game plays"),
                ("Slots destroyed", "/Counters/Slots destroyed"),
                ("Coins in the donation machine", "/Counters/Coins in the donation machine"),
                ("Eden tokens", "/Counters/Eden tokens"),
                ("Win streak", "/Counters/Win streak"),
                ("Best win streak", "/Counters/Best win streak"),
                ("Loss streak", "/Counters/Loss streak"),
                ("Coins in the greed donation machine", "/Counters/Coins in the greed donation machine"),
                ("Caves cleared", "/Counters/Caves cleared"),
                ("Basements cleared", "/Counters/Basements cleared"),
                ("Depths cleared", "/Counters/Depths cleared"),
            ],
        },
    ],
    lists: &[
        List { title: "Achievements", path: "/Achievements", shape: Shape::Map, names: &[], max: None },
    
        List { title: "Collectibles seen", path: "/Items seen", shape: Shape::Map, names: &[], max: None },
    
        List { title: "Challenges", path: "/Challenges", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[],
};

// ------------------------------------------------------------------ Risk of Rain 2
const ROR2: Spec = Spec {
    name: "Risk of Rain 2",
    tab: "Survivor",
    accent: (110, 160, 230),
    kind: Some("XML"),
    markers: &["/UserProfile/coins", "/UserProfile/totalRunCount"],
    subtitle: "A profile XML from Steam\\userdata\\<account>\\632360\\remote\\UserProfiles. Lunar coins and the profile's totals. Unlocked achievements, discovered items and the logbook are space-separated lists in the Tree; the per-stat table is under UserProfile > stats.",
    tiles: &[("Lunar coins", "/UserProfile/coins"), ("Runs", "/UserProfile/totalRunCount"), ("Coins collected", "/UserProfile/totalCollectedCoins")],
    sections: &[
        Section {
            title: "Profile",
            rows: &[
                ("Name", "/UserProfile/name"),
                ("Lunar coins", "/UserProfile/coins"),
                ("Lunar coins collected (total)", "/UserProfile/totalCollectedCoins"),
                ("Runs played", "/UserProfile/totalRunCount"),
                ("Run time (seconds)", "/UserProfile/totalRunSeconds"),
                ("Time alive (seconds)", "/UserProfile/totalAliveSeconds"),
                ("Time logged in (seconds)", "/UserProfile/totalLoginSeconds"),
                ("Preferred survivor", "/UserProfile/survivorPreference"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Slay the Spire 2
const STS2: Spec = Spec {
    name: "Slay the Spire 2",
    tab: "Progress",
    accent: (200, 150, 70),
    kind: None,
    markers: &["/architect_damage", "/current_score", "/character_stats"],
    subtitle: "progress.save from the game's profile folder (plain JSON). Each character's highest Ascension, wins, losses, win streaks and play time, plus the profile's score. Discovered cards, relics and the rest are lists in the Tree.",
    tiles: &[("Score", "/current_score"), ("Architect damage", "/architect_damage")],
    sections: &[
        Section {
            title: "Profile",
            rows: &[
                ("Score", "/current_score"),
                ("Architect damage", "/architect_damage"),
            ],
        },
    ],
    lists: &[],
    tables: &[
        Table { title: "Characters", path: "/character_stats", name: &["/id"], only: &[], cols: &[("Highest Ascension", "/max_ascension"), ("Preferred Ascension", "/preferred_ascension"), ("Wins", "/total_wins"), ("Losses", "/total_losses"), ("Win streak", "/current_streak"), ("Best win streak", "/best_win_streak"), ("Play time (s)", "/playtime")], names: &[("CHARACTER.IRONCLAD", "Ironclad"), ("CHARACTER.SILENT", "Silent"), ("CHARACTER.DEFECT", "Defect"), ("CHARACTER.NECROBINDER", "Necrobinder"), ("CHARACTER.REGENT", "Regent")] },
    ],
};

// ------------------------------------------------------------------ Heroes of Might and Magic III

const HEROES3: Spec = Spec {
    name: "Heroes of Might and Magic III",
    tab: "Heroes",
    accent: (196, 156, 62),
    kind: Some(crate::fmt::heroes3::KIND),
    markers: &["/Heroes"],
    subtitle: "A .GM1, .CGM or .GM2 save from the game's Games folder (Shadow of Death, Armageddon's Blade and Horn of the Abyss 1.8+). Each player's resources, and each hero's level, experience, attributes, spell points, movement, army and the skills they already have.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[
        Table {
            title: "Players (resources)",
            path: "/Players",
            name: &[],
            only: &[],
            cols: &[("Gold", "/Gold"), ("Wood", "/Wood"), ("Ore", "/Ore"), ("Mercury", "/Mercury"), ("Sulfur", "/Sulfur"), ("Crystal", "/Crystal"), ("Gems", "/Gems")],
            names: &[],
        },
        Table {
            title: "Heroes",
            path: "/Heroes",
            name: &[],
            only: &[],
            cols: &[
                ("Level", "/Level"), ("Experience", "/Experience"), ("Attack", "/Attack"), ("Defense", "/Defense"), ("Spell power", "/Spell power"),
                ("Knowledge", "/Knowledge"), ("Spell points", "/Spell points"), ("Movement left", "/Movement left"), ("Movement (full)", "/Movement (full)"),
            ],
            names: &[],
        },
        Table { title: "Armies", path: "/Army", name: &[], only: &[], cols: &[("Creature", "/Creature"), ("Count", "/Count")], names: &[] },
        Table { title: "Skills", path: "/Skills", name: &[], only: &[], cols: &[("Level", "/Level")], names: &[] },
    ],
};

// ------------------------------------------------------------------ Master of Orion II

const MOO2: Spec = Spec {
    name: "Master of Orion II",
    tab: "Empires",
    accent: (90, 140, 220),
    kind: Some(crate::fmt::moo2::KIND),
    markers: &["/Empires"],
    subtitle: "SAVE1.GAM to SAVE10.GAM from the game's folder (the original game; 208000-byte saves). Each empire's treasury, research points and ship range, and each colony's stored production and ground troops.",
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[
        Table {
            title: "Empires",
            path: "/Empires",
            name: &[],
            only: &[],
            cols: &[("BC reserve", "/BC reserve"), ("Research points", "/Research points"), ("Fuel range (parsecs)", "/Fuel range (parsecs)")],
            names: &[],
        },
        Table {
            title: "Colonies",
            path: "/Colonies",
            name: &[],
            only: &[],
            cols: &[("Production stored", "/Production stored"), ("Marines", "/Marines"), ("Tanks", "/Tanks")],
            names: &[],
        },
    ],
};

// ------------------------------------------------------------------ OpenXcom (X-COM: UFO Defense)

const OPENXCOM: Spec = Spec {
    name: "OpenXcom (X-COM: UFO Defense)",
    tab: "Base",
    accent: (96, 150, 110),
    kind: Some(crate::fmt::openxcom::KIND),
    markers: &["/funds/Current"],
    subtitle: "An OpenXcom or OpenXcom Extended save (.sav in Documents\\OpenXcom\\<mod>), which also plays UFO Defense from the Steam and GOG releases. Funds, the date, the difficulty, every soldier's stats and the base stores.",
    tiles: &[("Funds", "/funds/Current"), ("Months passed", "/monthsPassed")],
    sections: &[Section {
        title: "Game",
        rows: &[
            ("Funds", "/funds/Current"), ("Months passed", "/monthsPassed"), ("Days passed", "/daysPassed"),
            ("Difficulty (0 beginner, 1 experienced, 2 veteran, 3 genius, 4 superhuman)", "/difficulty"),
            ("Scientists in the first base", "/bases/0/scientists"), ("Engineers in the first base", "/bases/0/engineers"),
            ("Scientists in the second base", "/bases/1/scientists"), ("Engineers in the second base", "/bases/1/engineers"),
        ],
    }],
    lists: &[
        List { title: "Stores of the first base", path: "/bases/0/items", shape: Shape::Map, names: &[], max: None },
        List { title: "Stores of the second base", path: "/bases/1/items", shape: Shape::Map, names: &[], max: None },
        List { title: "Stores of the third base", path: "/bases/2/items", shape: Shape::Map, names: &[], max: None },
    ],
    tables: &[Table {
        title: "Soldiers (every base)",
        path: "/Soldiers",
        name: &["/name", "/base"],
        cols: &[
            ("Rank", "/rank"), ("Missions", "/missions"), ("Kills", "/kills"), ("Stuns", "/stuns"),
            ("Time units", "/currentStats/tu"), ("Stamina", "/currentStats/stamina"), ("Health", "/currentStats/health"),
            ("Bravery", "/currentStats/bravery"), ("Reactions", "/currentStats/reactions"), ("Firing", "/currentStats/firing"),
            ("Throwing", "/currentStats/throwing"), ("Strength", "/currentStats/strength"), ("Psi strength", "/currentStats/psiStrength"),
            ("Psi skill", "/currentStats/psiSkill"), ("Melee", "/currentStats/melee"),
        ],
        only: &[],
        names: &[],
    }],
};

// ------------------------------------------------------------------ X-COM (original DOS saves)

const XCOM_DOS: Spec = Spec {
    name: "X-COM: UFO Defense and Terror from the Deep (original saves)",
    tab: "Funds",
    accent: (96, 150, 110),
    kind: Some(crate::fmt::xcomdat::KIND),
    markers: &["/Funds/Money"],
    subtitle: "LIGLOB.DAT from a save slot folder (GAME_1 to GAME_10) of the original game. The funds; the other .DAT files are not edited yet.",
    tiles: &[("Funds", "/Funds/Money")],
    sections: &[Section { title: "Funds", rows: &[("Funds", "/Funds/Money")] }],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ RollerCoaster Tycoon 2

const RCT2: Spec = Spec {
    name: "RollerCoaster Tycoon 2",
    tab: "Park",
    accent: (214, 120, 50),
    kind: Some(crate::fmt::sv6::KIND),
    markers: &["/Park/Cash"],
    subtitle: "A saved park (.SV6) from the game's Saved Games folder. Cash, loan, maximum loan and the entrance fee; the park's chunk is unpacked and packed again, with the file sum written right.",
    tiles: &[("Cash", "/Park/Cash"), ("Bank loan", "/Park/Bank loan")],
    sections: &[Section {
        title: "Finances",
        rows: &[
            ("Cash", "/Park/Cash"), ("Bank loan", "/Park/Bank loan"), ("Maximum loan", "/Park/Maximum loan"),
            ("Entrance fee (tenths of a unit)", "/Park/Entrance fee (tenths of a unit)"),
        ],
    }],
    lists: &[],
    tables: &[],
};

const YSF: Spec = Spec {
    name: "Ys: The Oath in Felghana",
    tab: "Adol",
    accent: (200, 60, 50),
    kind: Some(crate::fmt::ysf::KIND),
    markers: &["/Adol/Gold", "/Adol/Level"],
    subtitle: "A ysf_NN.nacci save (the cipher and both checksums are handled). Level, HP, experience, gold and the difficulty.",
    tiles: &[("Gold", "/Adol/Gold"), ("Level", "/Adol/Level"), ("Experience", "/Adol/Experience")],
    sections: &[
        Section { title: "Adol", rows: &[("Level", "/Adol/Level"), ("Max HP", "/Adol/Max HP"), ("Experience", "/Adol/Experience"), ("Gold", "/Adol/Gold")] },
        Section { title: "Difficulty", rows: &[("Now", "/Difficulty/Now"), ("Change to", "/Difficulty/Change to")] },
        Section { title: "Save", rows: &[("Place", "/Save/Place"), ("Date", "/Save/Date"), ("Time", "/Save/Time")] },
    ],
    lists: &[],
    tables: &[],
};

const BD2: Spec = Spec {
    name: "Bravely Default II",
    tab: "Party",
    accent: (120, 170, 210),
    kind: Some("Unreal Engine save (SunriseSaveData)"),
    markers: &["/Inventory/money", "/Characters"],
    subtitle: "SaveGameData0.sav (the SAVE header and zlib packing are handled). pg, difficulty, each party member's level, experience, HP, MP and special ability points, and the item counts.",
    tiles: &[("pg", "/Inventory/money")],
    sections: &[Section { title: "Game", rows: &[("pg (money)", "/Inventory/money"), ("Difficulty", "/SystemData/ConfigData/DifficultyLevel"), ("Current map", "/MapName")] }],
    lists: &[List {
        title: "Items",
        path: "/Inventory/Items",
        shape: Shape::Array { id_key: "ID", qty_keys: &["Stock"] },
        names: &[],
        max: None,
    }],
    tables: &[Table {
        title: "Party",
        path: "/Characters",
        name: &["/Name"],
        only: &[],
        cols: &[
            ("Level", "/Level"), ("Experience", "/Exp"), ("HP", "/Hp"), ("Max HP", "/MaxHp"), ("MP", "/MP"), ("Max MP", "/MaxMp"),
            ("Special ability pts", "/SpecialAbilityPoints"),
            ("Raise: HP", "/ParameterRise/MaximumHealthPoints"), ("Raise: MP", "/ParameterRise/MaximumMagicPoints"), ("Raise: Speed", "/ParameterRise/Speed"),
            ("Raise: P.Atk", "/ParameterRise/PhysicalAttack"), ("Raise: P.Def", "/ParameterRise/PhysicalDefence"),
            ("Raise: M.Atk", "/ParameterRise/MagicAttack"), ("Raise: M.Def", "/ParameterRise/MagicDefence"), ("Raise: Heal", "/ParameterRise/HealPower"),
        ],
        names: &[],
    }],
};

const SKY3: Spec = Spec {
    name: "The Legend of Heroes: Trails in the Sky the 3rd",
    tab: "Party",
    accent: (200, 150, 60),
    kind: Some(crate::fmt::falcom::KIND_SKY3),
    markers: &["/Party/Mira"],
    subtitle: "SVDATnnn.SAV or Auto*.sav from Saved Games\\FALCOM\\ED_SORA3. Mira, sepith, character levels and the item counts.",
    tiles: &[("Mira", "/Party/Mira")],
    sections: &[Section {
        title: "Sepith",
        rows: &[("Earth", "/Sepith/Earth"), ("Water", "/Sepith/Water"), ("Fire", "/Sepith/Fire"), ("Wind", "/Sepith/Wind"), ("Time", "/Sepith/Time"), ("Space", "/Sepith/Space"), ("Mirage", "/Sepith/Mirage")],
    }],
    lists: &[List { title: "Items, quartz, equipment and meals", path: "/Items", shape: Shape::Map, names: &[], max: None }],
    tables: &[Table { title: "Characters", path: "/Characters", name: &[], only: &[], cols: &[("Level", "/Level")], names: &[] }],
};

const CS1: Spec = Spec {
    name: "The Legend of Heroes: Trails of Cold Steel",
    tab: "Party",
    accent: (150, 40, 50),
    kind: Some(crate::fmt::falcom::KIND_CS1),
    markers: &["/Party/Mira"],
    subtitle: "saveNNN.dat from Saved Games\\Falcom\\ed8. Mira, sepith, each member's level, experience, HP, EP, CP and stats, and the bag.",
    tiles: &[("Mira", "/Party/Mira")],
    sections: &[Section {
        title: "Sepith",
        rows: &[("Earth", "/Sepith/Earth"), ("Water", "/Sepith/Water"), ("Fire", "/Sepith/Fire"), ("Wind", "/Sepith/Wind"), ("Time", "/Sepith/Time"), ("Space", "/Sepith/Space"), ("Mirage", "/Sepith/Mirage"), ("Mass", "/Sepith/Mass")],
    }],
    lists: &[List { title: "Items, quartz, equipment and meals", path: "/Items", shape: Shape::Map, names: &[], max: None }],
    tables: &[Table {
        title: "Characters",
        path: "/Characters",
        name: &[],
        only: &[],
        cols: &[
            ("Level", "/Level"), ("Experience", "/Experience"), ("HP", "/HP"), ("Max HP", "/Max HP"), ("EP", "/EP"), ("Max EP", "/Max EP"), ("CP", "/CP"), ("Max CP", "/Max CP"),
            ("STR", "/STR"), ("DEF", "/DEF"), ("ATS", "/ATS"), ("ADF", "/ADF"), ("SPD", "/SPD"), ("DEX", "/DEX"), ("AGL", "/AGL"), ("MOV", "/MOV"), ("RNG", "/RNG"),
        ],
        names: &[],
    }],
};

const WO4: Spec = Spec {
    name: "Warriors Orochi 4",
    tab: "Resources",
    accent: (150, 60, 90),
    kind: Some(crate::fmt::wo4::KIND),
    markers: &["/Resources/Gems"],
    subtitle: "SAVEDATA.BIN (original) or SAVEDATAU.BIN (Ultimate) from Documents\\KoeiTecmo\\WARRIORS OROCHI 4\\Savedata. Gems, Crystals, Growth Points, Skill Orbs (lower only), weapon elements and officers' skill points.",
    tiles: &[("Gems", "/Resources/Gems"), ("Crystals", "/Resources/Crystals"), ("Growth Points", "/Resources/Growth Points")],
    sections: &[Section { title: "Resources", rows: &[("Gems", "/Resources/Gems"), ("Crystals", "/Resources/Crystals"), ("Growth Points", "/Resources/Growth Points"), ("Skill Orbs (lower only)", "/Resources/Skill Orbs (can only be lowered)")] }],
    lists: &[List {
        title: "Weapon elements (fusion materials you own)",
        path: "/Weapon elements",
        shape: Shape::Map,
        names: &[],
        max: Some(Max {
            ids: &["Element 1", "Element 2", "Element 3", "Element 4", "Element 5", "Element 6", "Element 7", "Element 8", "Element 9", "Element 10", "Element 11", "Element 12", "Element 13", "Element 14", "Element 15", "Element 16", "Element 17", "Element 18", "Element 19", "Element 20", "Element 21", "Element 22", "Element 23", "Element 24", "Element 25", "Element 26", "Element 27", "Element 28", "Element 29", "Element 30", "Element 31", "Element 32", "Element 33", "Element 34", "Element 35", "Element 36", "Element 37", "Element 38", "Element 39", "Element 40", "Element 41", "Element 42", "Element 43", "Element 44", "Element 45", "Element 46", "Element 47", "Element 48", "Element 49", "Element 50", "Element 51", "Element 52", "Element 53", "Element 54", "Element 55", "Element 56", "Element 57", "Element 58", "Element 59", "Element 60", "Element 61", "Element 62", "Element 63"],
            cap: 999,
            caps: &[],
            estimate: false,
            about: "Raises every weapon element you already own to 999, the stack limit in the official manual (Ultimate allows 9,999; 999 is safe for both). Elements you don't own are left alone.",
        }),
    }],
    tables: &[Table { title: "Officers", path: "/Officers", name: &[], only: &[], cols: &[("Experience", "/Experience"), ("Skill points", "/Skill points")], names: &[] }],
};

const SW4II: Spec = Spec {
    name: "Samurai Warriors 4-II",
    tab: "Army",
    accent: (170, 50, 50),
    kind: Some(crate::fmt::sw4ii::KIND),
    markers: &["/Army/Gold"],
    subtitle: "SAVEDATA0000.dat from Documents\\KoeiTecmo\\SAMURAI WARRIORS 4-II\\Savedata. Gold, strategy tomes and each officer's five stat bonuses.",
    tiles: &[("Gold", "/Army/Gold")],
    sections: &[
        Section { title: "Army", rows: &[("Gold", "/Army/Gold")] },
        Section { title: "Strategy tomes", rows: &[("Red", "/Strategy tomes/Red"), ("Green", "/Strategy tomes/Green"), ("Blue", "/Strategy tomes/Blue"), ("Yellow", "/Strategy tomes/Yellow"), ("Purple", "/Strategy tomes/Purple")] },
    ],
    lists: &[],
    tables: &[Table { title: "Officers", path: "/Officers", name: &[], only: &[], cols: &[("Level", "/Level"), ("Health", "/Health"), ("Attack", "/Attack"), ("Defense", "/Defense"), ("Riding", "/Riding"), ("Speed", "/Speed")], names: &[] }],
};

const NIER_REPLICANT: Spec = Spec {
    name: "NieR Replicant ver.1.22474487139...",
    tab: "Slots",
    accent: (190, 170, 130),
    kind: Some(crate::fmt::nier_replicant::KIND),
    markers: &[],
    subtitle: "GAMEDATA from Documents\\My Games\\NieR Replicant ver.1.22474487139...\\Steam\\<account>: health, level, experience, money, recovery items and materials of each used slot. Slot checksums are written again on save.",
    tiles: &[],
    sections: &[],
    lists: &[
        List { title: "NieR 1: recovery items", path: "/NieR 1/Recovery items", shape: Shape::Map, names: &[], max: None },
        List {
            title: "NieR 1: materials",
            path: "/NieR 1/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Aquatic Plant", "Deadwood", "Rusty Bucket", "Empty Can", "Gold Ore", "Silver Ore", "Copper Ore", "Iron Ore", "Crystal", "Pyrite", "Moldavite", "Meteorite", "Amber", "Fluorite", "Clay", "Berries", "Royal Fern", "Tree Branch", "Log", "Natural Rubber", "Ivy", "Lichen", "Mushroom", "Sap", "Mutton", "Boar Meat", "Wool", "Boar Hide", "Wolf Hide", "Wolf Fang", "Giant Spider Silk", "Bat Fang", "Bat Wing", "Goat Meat", "Goat Hide", "Venison", "Rainbow Spider Silk", "Boar Liver", "Scorpion Claw", "Scorpion Tail", "Dented Metal Board", "Stripped Bolt", "Broken Lens", "Severed Cable", "Broken Arm", "Broken Antenna", "Broken Motor", "Broken Battery", "Mysterious Switch", "Large Gear", "Titanium Alloy", "Memory Alloy", "Rusted Clump", "Machine Oil", "Forlorn Necklace", "Twisted Ring", "Broken Earring", "Pretty Choker", "Metal Piercing", "Subdued Bracelet", "Technical Guide", "Grubby Book", "Thick Dictionary", "Closed Book", "Used Coloring Book", "Old Schoolbook", "Dirty Bag", "Flashy Hat", "Leather Gloves", "Silk Handkerchief", "Leather Boots", "Complex Machine", "Elaborate Machine", "Simple Machine", "Stopped Clock", "Broken Wristwatch", "Rusty Kitchen Knife", "Broken Saw", "Dented Metal Bat", "Shell", "Gastropod", "Bivalve", "Seaweed", "Empty Bottle", "Driftwood", "Pearl", "Black Pearl", "Crab", "Starfish", "Sea Turtle Egg", "Broken Pottery", "Desert Rose", "Giant Egg", "Damascus Steel", "Eagle Egg", "Chicken Egg", "Mouse Tail", "Lizard Tail", "Deer Antler"],
                cap: 99,
                caps: &[],
                estimate: false,
                about: "Raises every material this slot already holds (plants, ores, animal parts, machine parts, salvage) to 99, the most the game carries.",
            }),
        },
        List { title: "NieR 2: recovery items", path: "/NieR 2/Recovery items", shape: Shape::Map, names: &[], max: None },
        List {
            title: "NieR 2: materials",
            path: "/NieR 2/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Aquatic Plant", "Deadwood", "Rusty Bucket", "Empty Can", "Gold Ore", "Silver Ore", "Copper Ore", "Iron Ore", "Crystal", "Pyrite", "Moldavite", "Meteorite", "Amber", "Fluorite", "Clay", "Berries", "Royal Fern", "Tree Branch", "Log", "Natural Rubber", "Ivy", "Lichen", "Mushroom", "Sap", "Mutton", "Boar Meat", "Wool", "Boar Hide", "Wolf Hide", "Wolf Fang", "Giant Spider Silk", "Bat Fang", "Bat Wing", "Goat Meat", "Goat Hide", "Venison", "Rainbow Spider Silk", "Boar Liver", "Scorpion Claw", "Scorpion Tail", "Dented Metal Board", "Stripped Bolt", "Broken Lens", "Severed Cable", "Broken Arm", "Broken Antenna", "Broken Motor", "Broken Battery", "Mysterious Switch", "Large Gear", "Titanium Alloy", "Memory Alloy", "Rusted Clump", "Machine Oil", "Forlorn Necklace", "Twisted Ring", "Broken Earring", "Pretty Choker", "Metal Piercing", "Subdued Bracelet", "Technical Guide", "Grubby Book", "Thick Dictionary", "Closed Book", "Used Coloring Book", "Old Schoolbook", "Dirty Bag", "Flashy Hat", "Leather Gloves", "Silk Handkerchief", "Leather Boots", "Complex Machine", "Elaborate Machine", "Simple Machine", "Stopped Clock", "Broken Wristwatch", "Rusty Kitchen Knife", "Broken Saw", "Dented Metal Bat", "Shell", "Gastropod", "Bivalve", "Seaweed", "Empty Bottle", "Driftwood", "Pearl", "Black Pearl", "Crab", "Starfish", "Sea Turtle Egg", "Broken Pottery", "Desert Rose", "Giant Egg", "Damascus Steel", "Eagle Egg", "Chicken Egg", "Mouse Tail", "Lizard Tail", "Deer Antler"],
                cap: 99,
                caps: &[],
                estimate: false,
                about: "Raises every material this slot already holds (plants, ores, animal parts, machine parts, salvage) to 99, the most the game carries.",
            }),
        },
        List { title: "NieR 3: recovery items", path: "/NieR 3/Recovery items", shape: Shape::Map, names: &[], max: None },
        List {
            title: "NieR 3: materials",
            path: "/NieR 3/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Aquatic Plant", "Deadwood", "Rusty Bucket", "Empty Can", "Gold Ore", "Silver Ore", "Copper Ore", "Iron Ore", "Crystal", "Pyrite", "Moldavite", "Meteorite", "Amber", "Fluorite", "Clay", "Berries", "Royal Fern", "Tree Branch", "Log", "Natural Rubber", "Ivy", "Lichen", "Mushroom", "Sap", "Mutton", "Boar Meat", "Wool", "Boar Hide", "Wolf Hide", "Wolf Fang", "Giant Spider Silk", "Bat Fang", "Bat Wing", "Goat Meat", "Goat Hide", "Venison", "Rainbow Spider Silk", "Boar Liver", "Scorpion Claw", "Scorpion Tail", "Dented Metal Board", "Stripped Bolt", "Broken Lens", "Severed Cable", "Broken Arm", "Broken Antenna", "Broken Motor", "Broken Battery", "Mysterious Switch", "Large Gear", "Titanium Alloy", "Memory Alloy", "Rusted Clump", "Machine Oil", "Forlorn Necklace", "Twisted Ring", "Broken Earring", "Pretty Choker", "Metal Piercing", "Subdued Bracelet", "Technical Guide", "Grubby Book", "Thick Dictionary", "Closed Book", "Used Coloring Book", "Old Schoolbook", "Dirty Bag", "Flashy Hat", "Leather Gloves", "Silk Handkerchief", "Leather Boots", "Complex Machine", "Elaborate Machine", "Simple Machine", "Stopped Clock", "Broken Wristwatch", "Rusty Kitchen Knife", "Broken Saw", "Dented Metal Bat", "Shell", "Gastropod", "Bivalve", "Seaweed", "Empty Bottle", "Driftwood", "Pearl", "Black Pearl", "Crab", "Starfish", "Sea Turtle Egg", "Broken Pottery", "Desert Rose", "Giant Egg", "Damascus Steel", "Eagle Egg", "Chicken Egg", "Mouse Tail", "Lizard Tail", "Deer Antler"],
                cap: 99,
                caps: &[],
                estimate: false,
                about: "Raises every material this slot already holds (plants, ores, animal parts, machine parts, salvage) to 99, the most the game carries.",
            }),
        },
        List { title: "Kaine 1: recovery items", path: "/Kaine 1/Recovery items", shape: Shape::Map, names: &[], max: None },
        List {
            title: "Kaine 1: materials",
            path: "/Kaine 1/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Aquatic Plant", "Deadwood", "Rusty Bucket", "Empty Can", "Gold Ore", "Silver Ore", "Copper Ore", "Iron Ore", "Crystal", "Pyrite", "Moldavite", "Meteorite", "Amber", "Fluorite", "Clay", "Berries", "Royal Fern", "Tree Branch", "Log", "Natural Rubber", "Ivy", "Lichen", "Mushroom", "Sap", "Mutton", "Boar Meat", "Wool", "Boar Hide", "Wolf Hide", "Wolf Fang", "Giant Spider Silk", "Bat Fang", "Bat Wing", "Goat Meat", "Goat Hide", "Venison", "Rainbow Spider Silk", "Boar Liver", "Scorpion Claw", "Scorpion Tail", "Dented Metal Board", "Stripped Bolt", "Broken Lens", "Severed Cable", "Broken Arm", "Broken Antenna", "Broken Motor", "Broken Battery", "Mysterious Switch", "Large Gear", "Titanium Alloy", "Memory Alloy", "Rusted Clump", "Machine Oil", "Forlorn Necklace", "Twisted Ring", "Broken Earring", "Pretty Choker", "Metal Piercing", "Subdued Bracelet", "Technical Guide", "Grubby Book", "Thick Dictionary", "Closed Book", "Used Coloring Book", "Old Schoolbook", "Dirty Bag", "Flashy Hat", "Leather Gloves", "Silk Handkerchief", "Leather Boots", "Complex Machine", "Elaborate Machine", "Simple Machine", "Stopped Clock", "Broken Wristwatch", "Rusty Kitchen Knife", "Broken Saw", "Dented Metal Bat", "Shell", "Gastropod", "Bivalve", "Seaweed", "Empty Bottle", "Driftwood", "Pearl", "Black Pearl", "Crab", "Starfish", "Sea Turtle Egg", "Broken Pottery", "Desert Rose", "Giant Egg", "Damascus Steel", "Eagle Egg", "Chicken Egg", "Mouse Tail", "Lizard Tail", "Deer Antler"],
                cap: 99,
                caps: &[],
                estimate: false,
                about: "Raises every material this slot already holds (plants, ores, animal parts, machine parts, salvage) to 99, the most the game carries.",
            }),
        },
        List { title: "Kaine 2: recovery items", path: "/Kaine 2/Recovery items", shape: Shape::Map, names: &[], max: None },
        List {
            title: "Kaine 2: materials",
            path: "/Kaine 2/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Aquatic Plant", "Deadwood", "Rusty Bucket", "Empty Can", "Gold Ore", "Silver Ore", "Copper Ore", "Iron Ore", "Crystal", "Pyrite", "Moldavite", "Meteorite", "Amber", "Fluorite", "Clay", "Berries", "Royal Fern", "Tree Branch", "Log", "Natural Rubber", "Ivy", "Lichen", "Mushroom", "Sap", "Mutton", "Boar Meat", "Wool", "Boar Hide", "Wolf Hide", "Wolf Fang", "Giant Spider Silk", "Bat Fang", "Bat Wing", "Goat Meat", "Goat Hide", "Venison", "Rainbow Spider Silk", "Boar Liver", "Scorpion Claw", "Scorpion Tail", "Dented Metal Board", "Stripped Bolt", "Broken Lens", "Severed Cable", "Broken Arm", "Broken Antenna", "Broken Motor", "Broken Battery", "Mysterious Switch", "Large Gear", "Titanium Alloy", "Memory Alloy", "Rusted Clump", "Machine Oil", "Forlorn Necklace", "Twisted Ring", "Broken Earring", "Pretty Choker", "Metal Piercing", "Subdued Bracelet", "Technical Guide", "Grubby Book", "Thick Dictionary", "Closed Book", "Used Coloring Book", "Old Schoolbook", "Dirty Bag", "Flashy Hat", "Leather Gloves", "Silk Handkerchief", "Leather Boots", "Complex Machine", "Elaborate Machine", "Simple Machine", "Stopped Clock", "Broken Wristwatch", "Rusty Kitchen Knife", "Broken Saw", "Dented Metal Bat", "Shell", "Gastropod", "Bivalve", "Seaweed", "Empty Bottle", "Driftwood", "Pearl", "Black Pearl", "Crab", "Starfish", "Sea Turtle Egg", "Broken Pottery", "Desert Rose", "Giant Egg", "Damascus Steel", "Eagle Egg", "Chicken Egg", "Mouse Tail", "Lizard Tail", "Deer Antler"],
                cap: 99,
                caps: &[],
                estimate: false,
                about: "Raises every material this slot already holds (plants, ores, animal parts, machine parts, salvage) to 99, the most the game carries.",
            }),
        },
        List { title: "Kaine 3: recovery items", path: "/Kaine 3/Recovery items", shape: Shape::Map, names: &[], max: None },
        List {
            title: "Kaine 3: materials",
            path: "/Kaine 3/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Aquatic Plant", "Deadwood", "Rusty Bucket", "Empty Can", "Gold Ore", "Silver Ore", "Copper Ore", "Iron Ore", "Crystal", "Pyrite", "Moldavite", "Meteorite", "Amber", "Fluorite", "Clay", "Berries", "Royal Fern", "Tree Branch", "Log", "Natural Rubber", "Ivy", "Lichen", "Mushroom", "Sap", "Mutton", "Boar Meat", "Wool", "Boar Hide", "Wolf Hide", "Wolf Fang", "Giant Spider Silk", "Bat Fang", "Bat Wing", "Goat Meat", "Goat Hide", "Venison", "Rainbow Spider Silk", "Boar Liver", "Scorpion Claw", "Scorpion Tail", "Dented Metal Board", "Stripped Bolt", "Broken Lens", "Severed Cable", "Broken Arm", "Broken Antenna", "Broken Motor", "Broken Battery", "Mysterious Switch", "Large Gear", "Titanium Alloy", "Memory Alloy", "Rusted Clump", "Machine Oil", "Forlorn Necklace", "Twisted Ring", "Broken Earring", "Pretty Choker", "Metal Piercing", "Subdued Bracelet", "Technical Guide", "Grubby Book", "Thick Dictionary", "Closed Book", "Used Coloring Book", "Old Schoolbook", "Dirty Bag", "Flashy Hat", "Leather Gloves", "Silk Handkerchief", "Leather Boots", "Complex Machine", "Elaborate Machine", "Simple Machine", "Stopped Clock", "Broken Wristwatch", "Rusty Kitchen Knife", "Broken Saw", "Dented Metal Bat", "Shell", "Gastropod", "Bivalve", "Seaweed", "Empty Bottle", "Driftwood", "Pearl", "Black Pearl", "Crab", "Starfish", "Sea Turtle Egg", "Broken Pottery", "Desert Rose", "Giant Egg", "Damascus Steel", "Eagle Egg", "Chicken Egg", "Mouse Tail", "Lizard Tail", "Deer Antler"],
                cap: 99,
                caps: &[],
                estimate: false,
                about: "Raises every material this slot already holds (plants, ores, animal parts, machine parts, salvage) to 99, the most the game carries.",
            }),
        },
        List { title: "NieR (after ending D): recovery items", path: "/NieR (after ending D)/Recovery items", shape: Shape::Map, names: &[], max: None },
        List {
            title: "NieR (after ending D): materials",
            path: "/NieR (after ending D)/Materials",
            shape: Shape::Map,
            names: &[],
            max: Some(Max {
                ids: &["Aquatic Plant", "Deadwood", "Rusty Bucket", "Empty Can", "Gold Ore", "Silver Ore", "Copper Ore", "Iron Ore", "Crystal", "Pyrite", "Moldavite", "Meteorite", "Amber", "Fluorite", "Clay", "Berries", "Royal Fern", "Tree Branch", "Log", "Natural Rubber", "Ivy", "Lichen", "Mushroom", "Sap", "Mutton", "Boar Meat", "Wool", "Boar Hide", "Wolf Hide", "Wolf Fang", "Giant Spider Silk", "Bat Fang", "Bat Wing", "Goat Meat", "Goat Hide", "Venison", "Rainbow Spider Silk", "Boar Liver", "Scorpion Claw", "Scorpion Tail", "Dented Metal Board", "Stripped Bolt", "Broken Lens", "Severed Cable", "Broken Arm", "Broken Antenna", "Broken Motor", "Broken Battery", "Mysterious Switch", "Large Gear", "Titanium Alloy", "Memory Alloy", "Rusted Clump", "Machine Oil", "Forlorn Necklace", "Twisted Ring", "Broken Earring", "Pretty Choker", "Metal Piercing", "Subdued Bracelet", "Technical Guide", "Grubby Book", "Thick Dictionary", "Closed Book", "Used Coloring Book", "Old Schoolbook", "Dirty Bag", "Flashy Hat", "Leather Gloves", "Silk Handkerchief", "Leather Boots", "Complex Machine", "Elaborate Machine", "Simple Machine", "Stopped Clock", "Broken Wristwatch", "Rusty Kitchen Knife", "Broken Saw", "Dented Metal Bat", "Shell", "Gastropod", "Bivalve", "Seaweed", "Empty Bottle", "Driftwood", "Pearl", "Black Pearl", "Crab", "Starfish", "Sea Turtle Egg", "Broken Pottery", "Desert Rose", "Giant Egg", "Damascus Steel", "Eagle Egg", "Chicken Egg", "Mouse Tail", "Lizard Tail", "Deer Antler"],
                cap: 99,
                caps: &[],
                estimate: false,
                about: "Raises every material this slot already holds (plants, ores, animal parts, machine parts, salvage) to 99, the most the game carries.",
            }),
        },
    ],
    tables: &[Table {
        title: "Slots",
        path: "",
        name: &[],
        only: &["NieR 1", "NieR 2", "NieR 3", "Kaine 1", "Kaine 2", "Kaine 3", "NieR (after ending D)"],
        cols: &[("Health", "/Health"), ("Level (shown minus one)", "/Level (shown level minus one)"), ("Experience", "/Experience"), ("Money", "/Money")],
        names: &[],
    }],
};

const GENZERO: Spec = Spec {
    name: "Generation Zero",
    tab: "Characters",
    accent: (210, 120, 50),
    kind: Some(crate::fmt::genzero::KIND),
    markers: &[],
    subtitle: "savegame from Documents\\Avalanche Studios\\GenerationZero\\Saves\\<account>: skill points, experience, level, inventory amounts, the resource store and the safehouse storage.",
    tiles: &[],
    sections: &[],
    lists: &[
        List { title: "Character 1: inventory", path: "/Character 1/Inventory", shape: Shape::Records { qty_key: "Amount" }, names: &[], max: Some(Max {
                ids: &["Aluminium", "Uranium", "Textile", "Steel", "Tungsten", "Rubber", "Adhesive", "Titanium", "Plastic", "Thread"],
                cap: 99,
                caps: &[],
                estimate: true,
                about: "Raises the crafting materials the character already holds (aluminium, uranium, textile, steel, tungsten, rubber, adhesive, titanium, plastic, thread) to 99, the stack size every item in the public sample has. Weapons, ammunition and other items are left alone. The game states no limit, so 99 is an estimate.",
            }) },
        List { title: "Character 2: inventory", path: "/Character 2/Inventory", shape: Shape::Records { qty_key: "Amount" }, names: &[], max: Some(Max {
                ids: &["Aluminium", "Uranium", "Textile", "Steel", "Tungsten", "Rubber", "Adhesive", "Titanium", "Plastic", "Thread"],
                cap: 99,
                caps: &[],
                estimate: true,
                about: "Raises the crafting materials the character already holds (aluminium, uranium, textile, steel, tungsten, rubber, adhesive, titanium, plastic, thread) to 99, the stack size every item in the public sample has. Weapons, ammunition and other items are left alone. The game states no limit, so 99 is an estimate.",
            }) },
        List { title: "Character 3: inventory", path: "/Character 3/Inventory", shape: Shape::Records { qty_key: "Amount" }, names: &[], max: Some(Max {
                ids: &["Aluminium", "Uranium", "Textile", "Steel", "Tungsten", "Rubber", "Adhesive", "Titanium", "Plastic", "Thread"],
                cap: 99,
                caps: &[],
                estimate: true,
                about: "Raises the crafting materials the character already holds (aluminium, uranium, textile, steel, tungsten, rubber, adhesive, titanium, plastic, thread) to 99, the stack size every item in the public sample has. Weapons, ammunition and other items are left alone. The game states no limit, so 99 is an estimate.",
            }) },
        List { title: "Character 4: inventory", path: "/Character 4/Inventory", shape: Shape::Records { qty_key: "Amount" }, names: &[], max: Some(Max {
                ids: &["Aluminium", "Uranium", "Textile", "Steel", "Tungsten", "Rubber", "Adhesive", "Titanium", "Plastic", "Thread"],
                cap: 99,
                caps: &[],
                estimate: true,
                about: "Raises the crafting materials the character already holds (aluminium, uranium, textile, steel, tungsten, rubber, adhesive, titanium, plastic, thread) to 99, the stack size every item in the public sample has. Weapons, ammunition and other items are left alone. The game states no limit, so 99 is an estimate.",
            }) },
        List { title: "Resource storage", path: "/Resource storage", shape: Shape::Map, names: &[], max: Some(Max {
                ids: &["Aluminium", "Uranium", "Textile", "Steel", "Tungsten", "Rubber", "Adhesive", "Titanium", "Plastic", "Thread"],
                cap: 99,
                caps: &[],
                estimate: true,
                about: "Raises the crafting materials the store already holds (aluminium, uranium, textile, steel, tungsten, rubber, adhesive, titanium, plastic, thread) to 99, the stack size every item in the public sample has. Weapons, ammunition and other items are left alone. The game states no limit, so 99 is an estimate.",
            }) },
        List { title: "Safehouse storage", path: "/Safehouse storage", shape: Shape::Records { qty_key: "Amount" }, names: &[], max: None },
    ],
    tables: &[Table {
        title: "Characters",
        path: "",
        name: &[],
        only: &["Character 1", "Character 2", "Character 3", "Character 4"],
        cols: &[("Skill points", "/Skill points"), ("Experience", "/Experience"), ("Level", "/Level")],
        names: &[],
    }],
};
// ------------------------------------------------------------------ Mario Kart 64 (gen)

const MK64: Spec = Spec {
    name: "Mario Kart 64",
    tab: "Racer",
    accent: (220, 60, 40),
    kind: Some(crate::fmt::mk64::KIND),
    markers: &["/Options/Sound mode"],
    subtitle: "An emulator's .eep save. Grand Prix cups for each class, the Time Trial records (five places and a best lap, with the character) of all sixteen courses, and the sound mode.",
    tiles: &[],
    sections: &[
    Section { title: "Options", rows: &[
        ("Sound mode", "/Options/Sound mode"),
    ] },
    Section { title: "Grand Prix cups", rows: &[
        ("50cc Mushroom Cup", "/Grand Prix/50cc/Mushroom Cup"),
        ("50cc Flower Cup", "/Grand Prix/50cc/Flower Cup"),
        ("50cc Star Cup", "/Grand Prix/50cc/Star Cup"),
        ("50cc Special Cup", "/Grand Prix/50cc/Special Cup"),
        ("100cc Mushroom Cup", "/Grand Prix/100cc/Mushroom Cup"),
        ("100cc Flower Cup", "/Grand Prix/100cc/Flower Cup"),
        ("100cc Star Cup", "/Grand Prix/100cc/Star Cup"),
        ("100cc Special Cup", "/Grand Prix/100cc/Special Cup"),
        ("150cc Mushroom Cup", "/Grand Prix/150cc/Mushroom Cup"),
        ("150cc Flower Cup", "/Grand Prix/150cc/Flower Cup"),
        ("150cc Star Cup", "/Grand Prix/150cc/Star Cup"),
        ("150cc Special Cup", "/Grand Prix/150cc/Special Cup"),
        ("Extra Mushroom Cup", "/Grand Prix/Extra/Mushroom Cup"),
        ("Extra Flower Cup", "/Grand Prix/Extra/Flower Cup"),
        ("Extra Star Cup", "/Grand Prix/Extra/Star Cup"),
        ("Extra Special Cup", "/Grand Prix/Extra/Special Cup"),
    ] },
    ],
    lists: &[],
    tables: &[
        Table { title: "Time Trials: times (hundredths of a second, 600000 = no record)", path: "/Time Trials", name: &[], cols: &[("1st place", "/1st place time"), ("2nd place", "/2nd place time"), ("3rd place", "/3rd place time"), ("4th place", "/4th place time"), ("5th place", "/5th place time"), ("Best lap", "/Best lap time")], only: &[], names: &[] },
        Table { title: "Time Trials: characters", path: "/Time Trials", name: &[], cols: &[("1st place", "/1st place character"), ("2nd place", "/2nd place character"), ("3rd place", "/3rd place character"), ("4th place", "/4th place character"), ("5th place", "/5th place character"), ("Best lap", "/Best lap character")], only: &[], names: &[] },
    ],
};

// end MK64

// ------------------------------------------------------------------ GoldenEye 007 (gen)

const GOLDENEYE: Spec = Spec {
    name: "GoldenEye 007",
    tab: "Agent",
    accent: (190, 150, 40),
    kind: Some(crate::fmt::goldeneye::KIND),
    markers: &["/Info/Slots in use"],
    subtitle: "An emulator's .eep save. Per player slot: the best time of all 20 missions on each difficulty, the cheats and the control options.",
    tiles: &[("Slots in use", "/Info/Slots in use")],
    sections: &[
    Section { title: "Slot 1: cheats", rows: &[
        ("All levels and 007 difficulty unlocked", "/Slot 1/Cheats/All levels and 007 difficulty unlocked"),
        ("Invincible", "/Slot 1/Cheats/Invincible"),
        ("All Guns", "/Slot 1/Cheats/All Guns"),
        ("Bond Invisible", "/Slot 1/Cheats/Bond Invisible"),
        ("Infinite Ammo", "/Slot 1/Cheats/Infinite Ammo"),
        ("DK Mode", "/Slot 1/Cheats/DK Mode"),
        ("Tiny Bond", "/Slot 1/Cheats/Tiny Bond"),
        ("Paintball Mode", "/Slot 1/Cheats/Paintball Mode"),
        ("Silver PP7", "/Slot 1/Cheats/Silver PP7"),
        ("Gold PP7", "/Slot 1/Cheats/Gold PP7"),
        ("No Radar (multiplayer)", "/Slot 1/Cheats/No Radar (multiplayer)"),
        ("Turbo Mode", "/Slot 1/Cheats/Turbo Mode"),
        ("Fast Animation", "/Slot 1/Cheats/Fast Animation"),
        ("Slow Animation", "/Slot 1/Cheats/Slow Animation"),
        ("Enemy Rockets", "/Slot 1/Cheats/Enemy Rockets"),
        ("2x Rocket Launcher", "/Slot 1/Cheats/2x Rocket Launcher"),
        ("2x Grenade Launcher", "/Slot 1/Cheats/2x Grenade Launcher"),
        ("2x RC-P90", "/Slot 1/Cheats/2x RC-P90"),
        ("2x Throwing Knife", "/Slot 1/Cheats/2x Throwing Knife"),
        ("2x Hunting Knife", "/Slot 1/Cheats/2x Hunting Knife"),
        ("2x Laser", "/Slot 1/Cheats/2x Laser"),
    ] },
    Section { title: "Slot 1: options", rows: &[
        ("Control style", "/Slot 1/Options/Control style"),
        ("Music", "/Slot 1/Options/Music"),
        ("Effects", "/Slot 1/Options/Effects"),
        ("Look up/down", "/Slot 1/Options/Look up~1down"),
        ("Auto-aim", "/Slot 1/Options/Auto-aim"),
        ("Sight on-screen", "/Slot 1/Options/Sight on-screen"),
        ("Look ahead", "/Slot 1/Options/Look ahead"),
        ("Ammo on-screen", "/Slot 1/Options/Ammo on-screen"),
        ("Aim control", "/Slot 1/Options/Aim control"),
        ("Screen", "/Slot 1/Options/Screen"),
    ] },
    Section { title: "Slot 2: cheats", rows: &[
        ("All levels and 007 difficulty unlocked", "/Slot 2/Cheats/All levels and 007 difficulty unlocked"),
        ("Invincible", "/Slot 2/Cheats/Invincible"),
        ("All Guns", "/Slot 2/Cheats/All Guns"),
        ("Bond Invisible", "/Slot 2/Cheats/Bond Invisible"),
        ("Infinite Ammo", "/Slot 2/Cheats/Infinite Ammo"),
        ("DK Mode", "/Slot 2/Cheats/DK Mode"),
        ("Tiny Bond", "/Slot 2/Cheats/Tiny Bond"),
        ("Paintball Mode", "/Slot 2/Cheats/Paintball Mode"),
        ("Silver PP7", "/Slot 2/Cheats/Silver PP7"),
        ("Gold PP7", "/Slot 2/Cheats/Gold PP7"),
        ("No Radar (multiplayer)", "/Slot 2/Cheats/No Radar (multiplayer)"),
        ("Turbo Mode", "/Slot 2/Cheats/Turbo Mode"),
        ("Fast Animation", "/Slot 2/Cheats/Fast Animation"),
        ("Slow Animation", "/Slot 2/Cheats/Slow Animation"),
        ("Enemy Rockets", "/Slot 2/Cheats/Enemy Rockets"),
        ("2x Rocket Launcher", "/Slot 2/Cheats/2x Rocket Launcher"),
        ("2x Grenade Launcher", "/Slot 2/Cheats/2x Grenade Launcher"),
        ("2x RC-P90", "/Slot 2/Cheats/2x RC-P90"),
        ("2x Throwing Knife", "/Slot 2/Cheats/2x Throwing Knife"),
        ("2x Hunting Knife", "/Slot 2/Cheats/2x Hunting Knife"),
        ("2x Laser", "/Slot 2/Cheats/2x Laser"),
    ] },
    Section { title: "Slot 2: options", rows: &[
        ("Control style", "/Slot 2/Options/Control style"),
        ("Music", "/Slot 2/Options/Music"),
        ("Effects", "/Slot 2/Options/Effects"),
        ("Look up/down", "/Slot 2/Options/Look up~1down"),
        ("Auto-aim", "/Slot 2/Options/Auto-aim"),
        ("Sight on-screen", "/Slot 2/Options/Sight on-screen"),
        ("Look ahead", "/Slot 2/Options/Look ahead"),
        ("Ammo on-screen", "/Slot 2/Options/Ammo on-screen"),
        ("Aim control", "/Slot 2/Options/Aim control"),
        ("Screen", "/Slot 2/Options/Screen"),
    ] },
    Section { title: "Slot 3: cheats", rows: &[
        ("All levels and 007 difficulty unlocked", "/Slot 3/Cheats/All levels and 007 difficulty unlocked"),
        ("Invincible", "/Slot 3/Cheats/Invincible"),
        ("All Guns", "/Slot 3/Cheats/All Guns"),
        ("Bond Invisible", "/Slot 3/Cheats/Bond Invisible"),
        ("Infinite Ammo", "/Slot 3/Cheats/Infinite Ammo"),
        ("DK Mode", "/Slot 3/Cheats/DK Mode"),
        ("Tiny Bond", "/Slot 3/Cheats/Tiny Bond"),
        ("Paintball Mode", "/Slot 3/Cheats/Paintball Mode"),
        ("Silver PP7", "/Slot 3/Cheats/Silver PP7"),
        ("Gold PP7", "/Slot 3/Cheats/Gold PP7"),
        ("No Radar (multiplayer)", "/Slot 3/Cheats/No Radar (multiplayer)"),
        ("Turbo Mode", "/Slot 3/Cheats/Turbo Mode"),
        ("Fast Animation", "/Slot 3/Cheats/Fast Animation"),
        ("Slow Animation", "/Slot 3/Cheats/Slow Animation"),
        ("Enemy Rockets", "/Slot 3/Cheats/Enemy Rockets"),
        ("2x Rocket Launcher", "/Slot 3/Cheats/2x Rocket Launcher"),
        ("2x Grenade Launcher", "/Slot 3/Cheats/2x Grenade Launcher"),
        ("2x RC-P90", "/Slot 3/Cheats/2x RC-P90"),
        ("2x Throwing Knife", "/Slot 3/Cheats/2x Throwing Knife"),
        ("2x Hunting Knife", "/Slot 3/Cheats/2x Hunting Knife"),
        ("2x Laser", "/Slot 3/Cheats/2x Laser"),
    ] },
    Section { title: "Slot 3: options", rows: &[
        ("Control style", "/Slot 3/Options/Control style"),
        ("Music", "/Slot 3/Options/Music"),
        ("Effects", "/Slot 3/Options/Effects"),
        ("Look up/down", "/Slot 3/Options/Look up~1down"),
        ("Auto-aim", "/Slot 3/Options/Auto-aim"),
        ("Sight on-screen", "/Slot 3/Options/Sight on-screen"),
        ("Look ahead", "/Slot 3/Options/Look ahead"),
        ("Ammo on-screen", "/Slot 3/Options/Ammo on-screen"),
        ("Aim control", "/Slot 3/Options/Aim control"),
        ("Screen", "/Slot 3/Options/Screen"),
    ] },
    Section { title: "Slot 4: cheats", rows: &[
        ("All levels and 007 difficulty unlocked", "/Slot 4/Cheats/All levels and 007 difficulty unlocked"),
        ("Invincible", "/Slot 4/Cheats/Invincible"),
        ("All Guns", "/Slot 4/Cheats/All Guns"),
        ("Bond Invisible", "/Slot 4/Cheats/Bond Invisible"),
        ("Infinite Ammo", "/Slot 4/Cheats/Infinite Ammo"),
        ("DK Mode", "/Slot 4/Cheats/DK Mode"),
        ("Tiny Bond", "/Slot 4/Cheats/Tiny Bond"),
        ("Paintball Mode", "/Slot 4/Cheats/Paintball Mode"),
        ("Silver PP7", "/Slot 4/Cheats/Silver PP7"),
        ("Gold PP7", "/Slot 4/Cheats/Gold PP7"),
        ("No Radar (multiplayer)", "/Slot 4/Cheats/No Radar (multiplayer)"),
        ("Turbo Mode", "/Slot 4/Cheats/Turbo Mode"),
        ("Fast Animation", "/Slot 4/Cheats/Fast Animation"),
        ("Slow Animation", "/Slot 4/Cheats/Slow Animation"),
        ("Enemy Rockets", "/Slot 4/Cheats/Enemy Rockets"),
        ("2x Rocket Launcher", "/Slot 4/Cheats/2x Rocket Launcher"),
        ("2x Grenade Launcher", "/Slot 4/Cheats/2x Grenade Launcher"),
        ("2x RC-P90", "/Slot 4/Cheats/2x RC-P90"),
        ("2x Throwing Knife", "/Slot 4/Cheats/2x Throwing Knife"),
        ("2x Hunting Knife", "/Slot 4/Cheats/2x Hunting Knife"),
        ("2x Laser", "/Slot 4/Cheats/2x Laser"),
    ] },
    Section { title: "Slot 4: options", rows: &[
        ("Control style", "/Slot 4/Options/Control style"),
        ("Music", "/Slot 4/Options/Music"),
        ("Effects", "/Slot 4/Options/Effects"),
        ("Look up/down", "/Slot 4/Options/Look up~1down"),
        ("Auto-aim", "/Slot 4/Options/Auto-aim"),
        ("Sight on-screen", "/Slot 4/Options/Sight on-screen"),
        ("Look ahead", "/Slot 4/Options/Look ahead"),
        ("Ammo on-screen", "/Slot 4/Options/Ammo on-screen"),
        ("Aim control", "/Slot 4/Options/Aim control"),
        ("Screen", "/Slot 4/Options/Screen"),
    ] },
    ],
    lists: &[],
    tables: &[
        Table { title: "Slot 1: mission times (seconds; 0 = not done)", path: "/Slot 1/Missions", name: &[], cols: &[("Agent", "/Agent"), ("Secret Agent", "/Secret Agent"), ("00 Agent", "/00 Agent")], only: &[], names: &[] },
        Table { title: "Slot 2: mission times (seconds; 0 = not done)", path: "/Slot 2/Missions", name: &[], cols: &[("Agent", "/Agent"), ("Secret Agent", "/Secret Agent"), ("00 Agent", "/00 Agent")], only: &[], names: &[] },
        Table { title: "Slot 3: mission times (seconds; 0 = not done)", path: "/Slot 3/Missions", name: &[], cols: &[("Agent", "/Agent"), ("Secret Agent", "/Secret Agent"), ("00 Agent", "/00 Agent")], only: &[], names: &[] },
        Table { title: "Slot 4: mission times (seconds; 0 = not done)", path: "/Slot 4/Missions", name: &[], cols: &[("Agent", "/Agent"), ("Secret Agent", "/Secret Agent"), ("00 Agent", "/00 Agent")], only: &[], names: &[] },
    ],
};

// end GOLDENEYE

// ------------------------------------------------------------------ Banjo-Kazooie (gen)

const BANJO_KAZOOIE: Spec = Spec {
    name: "Banjo-Kazooie",
    tab: "Banjo",
    accent: (230, 170, 50),
    kind: Some(crate::fmt::banjokazooie::KIND),
    markers: &["/Info/Slots in use"],
    subtitle: "An emulator's .eep save. Per save slot: jiggies, Mumbo tokens, health, eggs and feathers with their maximums, every move, and each world's jiggies. Plus the Stop 'n' Swop items.",
    tiles: &[("Slots in use", "/Info/Slots in use")],
    sections: &[
    Section { title: "Slot 1: collectibles", rows: &[
        ("Progression", "/Slot 1/General/Progression"),
        ("Jiggies", "/Slot 1/General/Jiggies"),
        ("Mumbo Tokens", "/Slot 1/General/Mumbo Tokens"),
        ("Health", "/Slot 1/General/Health"),
        ("Eggs", "/Slot 1/General/Eggs"),
        ("Eggs max", "/Slot 1/General/Eggs max"),
        ("Red Feathers", "/Slot 1/General/Red Feathers"),
        ("Red Feathers max", "/Slot 1/General/Red Feathers max"),
        ("Gold Feathers", "/Slot 1/General/Gold Feathers"),
        ("Gold Feathers max", "/Slot 1/General/Gold Feathers max"),
    ] },
    Section { title: "Slot 1: basic moves", rows: &[
        ("High Jump", "/Slot 1/Special Moves/Basic Moves/High Jump"),
        ("Feathery Flap", "/Slot 1/Special Moves/Basic Moves/Feathery Flap"),
        ("Flap Flip", "/Slot 1/Special Moves/Basic Moves/Flap Flip"),
        ("Climbing", "/Slot 1/Special Moves/Basic Moves/Climbing"),
        ("Swimming", "/Slot 1/Special Moves/Basic Moves/Swimming"),
        ("Claw Swipe", "/Slot 1/Special Moves/Basic Moves/Claw Swipe"),
        ("Forward Roll Attack", "/Slot 1/Special Moves/Basic Moves/Forward Roll Attack"),
        ("Rat-a-tat Rap", "/Slot 1/Special Moves/Basic Moves/Rat-a-tat Rap"),
        ("Beak Barge", "/Slot 1/Special Moves/Basic Moves/Beak Barge"),
    ] },
    Section { title: "Slot 1: advanced moves", rows: &[
        ("Talon Trot", "/Slot 1/Special Moves/Advanced Moves/Talon Trot"),
        ("Beak Buster", "/Slot 1/Special Moves/Advanced Moves/Beak Buster"),
        ("Egg Firing", "/Slot 1/Special Moves/Advanced Moves/Egg Firing"),
        ("Shock Jump Spring", "/Slot 1/Special Moves/Advanced Moves/Shock Jump Spring"),
        ("Flight", "/Slot 1/Special Moves/Advanced Moves/Flight"),
        ("Wonderwing", "/Slot 1/Special Moves/Advanced Moves/Wonderwing"),
        ("Turbo Talon Trot", "/Slot 1/Special Moves/Advanced Moves/Turbo Talon Trot"),
        ("Stilt Stride", "/Slot 1/Special Moves/Advanced Moves/Stilt Stride"),
        ("Beak Bomb", "/Slot 1/Special Moves/Advanced Moves/Beak Bomb"),
    ] },
    Section { title: "Slot 1: Gruntilda's Lair jiggies", rows: &[
        ("Entrance", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Entrance"),
        ("Witch Switch of Mumbo's Mountain", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Mumbo's Mountain"),
        ("Witch Switch of Treasure Trove Cove", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Treasure Trove Cove"),
        ("Witch Switch of Clanker's Cavern", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Clanker's Cavern"),
        ("Witch Switch of Bubblegloop Swamp", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Bubblegloop Swamp"),
        ("Witch Switch of Freezeezy Peak", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Freezeezy Peak"),
        ("Witch Switch of Gobi's Valley", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Gobi's Valley"),
        ("Witch Switch of Mad Monster Mansion", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Mad Monster Mansion"),
        ("Witch Switch of Rusty Bucket Bay", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Rusty Bucket Bay"),
        ("Witch Switch of Click Clock Wood", "/Slot 1/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Click Clock Wood"),
    ] },
    Section { title: "Slot 1: Mumbo's Mountain jiggies", rows: &[
        ("Conga's orange blocks", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/Conga's orange blocks"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/5 Jinjos rescued"),
        ("At the top of the termites nest", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/At the top of the termites nest"),
        ("In the eye of the Mumbo's Skull", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/In the eye of the Mumbo's Skull"),
        ("Feeding Juju", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/Feeding Juju"),
        ("Inside a hut of the village", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/Inside a hut of the village"),
        ("At the center of the stone structure", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/At the center of the stone structure"),
        ("On the hill near entrance", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/On the hill near entrance"),
        ("Give an orange to Chimpy's", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/Give an orange to Chimpy's"),
        ("Conga's Battle", "/Slot 1/Levels/Mumbo's Mountain/General/Jiggies/Conga's Battle"),
    ] },
    Section { title: "Slot 1: Treasure Trove Cove jiggies", rows: &[
        ("Sandcastle password", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/Sandcastle password"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/5 Jinjos rescued"),
        ("At the top of the lighthouse", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/At the top of the lighthouse"),
        ("Shock Spring Pads path", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/Shock Spring Pads path"),
        ("On the backside of the spiral tower", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/On the backside of the spiral tower"),
        ("At the bottom of a pool of water", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/At the bottom of a pool of water"),
        ("Golden treasure marks", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/Golden treasure marks"),
        ("Inside Nipper's Shell", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/Inside Nipper's Shell"),
        ("Inside a Lockup", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/Inside a Lockup"),
        ("Captain Blubber's treasures", "/Slot 1/Levels/Treasure Trove Cove/General/Jiggies/Captain Blubber's treasures"),
    ] },
    Section { title: "Slot 1: Clanker's Cavern jiggies", rows: &[
        ("Above Clanker using his tail", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/Above Clanker using his tail"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/5 Jinjos rescued"),
        ("Battle against Mutie-Snippets", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/Battle against Mutie-Snippets"),
        ("The large key", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/The large key"),
        ("Above Clanker using his bolt", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/Above Clanker using his bolt"),
        ("In a pipe near Clanker", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/In a pipe near Clanker"),
        ("On the Clanker's right golden tooth", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/On the Clanker's right golden tooth"),
        ("The ring challenge", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/The ring challenge"),
        ("Near the Witch Switch", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/Near the Witch Switch"),
        ("Near Bottles using Wonderwing", "/Slot 1/Levels/Clanker's Cavern/General/Jiggies/Near Bottles using Wonderwing"),
    ] },
    Section { title: "Slot 1: Bubblegloop Swamp jiggies", rows: &[
        ("45 seconds race", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/45 seconds race"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/5 Jinjos rescued"),
        ("Mr. Vile game", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/Mr. Vile game"),
        ("Inside the big egg", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/Inside the big egg"),
        ("Feed the Croctus", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/Feed the Croctus"),
        ("In the highest hut of the treetop area", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/In the highest hut of the treetop area"),
        ("Yellow Flibbits battle", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/Yellow Flibbits battle"),
        ("10 seconds race", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/10 seconds race"),
        ("Tanktup's feet", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/Tanktup's feet"),
        ("Musical lessons", "/Slot 1/Levels/Bubblegloop Swamp/General/Jiggies/Musical lessons"),
    ] },
    Section { title: "Slot 1: Freezeezy Peak jiggies", rows: &[
        ("First sled race against Boggy", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/First sled race against Boggy"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/5 Jinjos rescued"),
        ("Inside Boggy's stomach", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/Inside Boggy's stomach"),
        ("Inside the giant snowman's pipe", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/Inside the giant snowman's pipe"),
        ("Second sled race against Boggy", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/Second sled race against Boggy"),
        ("Buttons of giant snowman", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/Buttons of giant snowman"),
        ("Presents to Boggy's kids", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/Presents to Boggy's kids"),
        ("At the top of the christmas tree", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/At the top of the christmas tree"),
        ("Destroy all Sire Slushes", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/Destroy all Sire Slushes"),
        ("Given by Wozza", "/Slot 1/Levels/Freezeezy Peak/General/Jiggies/Given by Wozza"),
    ] },
    Section { title: "Slot 1: Gobi's Valley jiggies", rows: &[
        ("In the tile puzzle pyramid", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/In the tile puzzle pyramid"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/5 Jinjos rescued"),
        ("Stolen from Grabba", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Stolen from Grabba"),
        ("Inside Jinyx", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Inside Jinyx"),
        ("Inside King Sandybutt's sarcophagus", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Inside King Sandybutt's sarcophagus"),
        ("Inside the water pyramid", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Inside the water pyramid"),
        ("Inside Rubee's temple", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Inside Rubee's temple"),
        ("Free Gobi", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Free Gobi"),
        ("Above Trunker", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Above Trunker"),
        ("Flying ring challenge", "/Slot 1/Levels/Gobi's Valley/General/Jiggies/Flying ring challenge"),
    ] },
    Section { title: "Slot 1: Mad Monster Mansion jiggies", rows: &[
        ("After the organ lesson in the church", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/After the organ lesson in the church"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/5 Jinjos rescued"),
        ("Inside the well", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/Inside the well"),
        ("On the table of the dining room", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/On the table of the dining room"),
        ("In a barrel in the cellar", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/In a barrel in the cellar"),
        ("At the top of the church", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/At the top of the church"),
        ("In a basin by using the gutter", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/In a basin by using the gutter"),
        ("Tumblar's game", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/Tumblar's game"),
        ("Flowers in the graveyard", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/Flowers in the graveyard"),
        ("In the sewer after being swallowed by Loggo", "/Slot 1/Levels/Mad Monster Mansion/General/Jiggies/In the sewer after being swallowed by Loggo"),
    ] },
    Section { title: "Slot 1: Rusty Bucket Bay jiggies", rows: &[
        ("In the captain's room", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/In the captain's room"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/5 Jinjos rescued"),
        ("In the warehouse", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/In the warehouse"),
        ("Snorkel the dolphin rescued", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/Snorkel the dolphin rescued"),
        ("3 whistle switches", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/3 whistle switches"),
        ("At the top of a chimney", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/At the top of a chimney"),
        ("Beat Boss Boom Box", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/Beat Boss Boom Box"),
        ("Behind the propellers", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/Behind the propellers"),
        ("In a cage on the ship", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/In a cage on the ship"),
        ("In the engine room", "/Slot 1/Levels/Rusty Bucket Bay/General/Jiggies/In the engine room"),
    ] },
    Section { title: "Slot 1: Click Clock Wood jiggies", rows: &[
        ("At the top of the tree", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/At the top of the tree"),
        ("5 Jinjos rescued", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/5 Jinjos rescued"),
        ("Spring / Winter: At the highest platform of the tree", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Spring ~1 Winter: At the highest platform of the tree"),
        ("Summer: Defeat the Zubbas inside the beehive", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Summer: Defeat the Zubbas inside the beehive"),
        ("Summer / Autumn: In the treehouse", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Summer ~1 Autumn: In the treehouse"),
        ("Summer / Autumn: Following the leaves path", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Summer ~1 Autumn: Following the leaves path"),
        ("Autumn: The acorns of Nabnut", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Autumn: The acorns of Nabnut"),
        ("Autumn: In the house of Gnawty", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Autumn: In the house of Gnawty"),
        ("Autumn: At the top of the eggplant", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Autumn: At the top of the eggplant"),
        ("Winter: Eyrie the eagle", "/Slot 1/Levels/Click Clock Wood/General/Jiggies/Winter: Eyrie the eagle"),
    ] },
    Section { title: "Slot 2: collectibles", rows: &[
        ("Progression", "/Slot 2/General/Progression"),
        ("Jiggies", "/Slot 2/General/Jiggies"),
        ("Mumbo Tokens", "/Slot 2/General/Mumbo Tokens"),
        ("Health", "/Slot 2/General/Health"),
        ("Eggs", "/Slot 2/General/Eggs"),
        ("Eggs max", "/Slot 2/General/Eggs max"),
        ("Red Feathers", "/Slot 2/General/Red Feathers"),
        ("Red Feathers max", "/Slot 2/General/Red Feathers max"),
        ("Gold Feathers", "/Slot 2/General/Gold Feathers"),
        ("Gold Feathers max", "/Slot 2/General/Gold Feathers max"),
    ] },
    Section { title: "Slot 2: basic moves", rows: &[
        ("High Jump", "/Slot 2/Special Moves/Basic Moves/High Jump"),
        ("Feathery Flap", "/Slot 2/Special Moves/Basic Moves/Feathery Flap"),
        ("Flap Flip", "/Slot 2/Special Moves/Basic Moves/Flap Flip"),
        ("Climbing", "/Slot 2/Special Moves/Basic Moves/Climbing"),
        ("Swimming", "/Slot 2/Special Moves/Basic Moves/Swimming"),
        ("Claw Swipe", "/Slot 2/Special Moves/Basic Moves/Claw Swipe"),
        ("Forward Roll Attack", "/Slot 2/Special Moves/Basic Moves/Forward Roll Attack"),
        ("Rat-a-tat Rap", "/Slot 2/Special Moves/Basic Moves/Rat-a-tat Rap"),
        ("Beak Barge", "/Slot 2/Special Moves/Basic Moves/Beak Barge"),
    ] },
    Section { title: "Slot 2: advanced moves", rows: &[
        ("Talon Trot", "/Slot 2/Special Moves/Advanced Moves/Talon Trot"),
        ("Beak Buster", "/Slot 2/Special Moves/Advanced Moves/Beak Buster"),
        ("Egg Firing", "/Slot 2/Special Moves/Advanced Moves/Egg Firing"),
        ("Shock Jump Spring", "/Slot 2/Special Moves/Advanced Moves/Shock Jump Spring"),
        ("Flight", "/Slot 2/Special Moves/Advanced Moves/Flight"),
        ("Wonderwing", "/Slot 2/Special Moves/Advanced Moves/Wonderwing"),
        ("Turbo Talon Trot", "/Slot 2/Special Moves/Advanced Moves/Turbo Talon Trot"),
        ("Stilt Stride", "/Slot 2/Special Moves/Advanced Moves/Stilt Stride"),
        ("Beak Bomb", "/Slot 2/Special Moves/Advanced Moves/Beak Bomb"),
    ] },
    Section { title: "Slot 2: Gruntilda's Lair jiggies", rows: &[
        ("Entrance", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Entrance"),
        ("Witch Switch of Mumbo's Mountain", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Mumbo's Mountain"),
        ("Witch Switch of Treasure Trove Cove", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Treasure Trove Cove"),
        ("Witch Switch of Clanker's Cavern", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Clanker's Cavern"),
        ("Witch Switch of Bubblegloop Swamp", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Bubblegloop Swamp"),
        ("Witch Switch of Freezeezy Peak", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Freezeezy Peak"),
        ("Witch Switch of Gobi's Valley", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Gobi's Valley"),
        ("Witch Switch of Mad Monster Mansion", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Mad Monster Mansion"),
        ("Witch Switch of Rusty Bucket Bay", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Rusty Bucket Bay"),
        ("Witch Switch of Click Clock Wood", "/Slot 2/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Click Clock Wood"),
    ] },
    Section { title: "Slot 2: Mumbo's Mountain jiggies", rows: &[
        ("Conga's orange blocks", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/Conga's orange blocks"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/5 Jinjos rescued"),
        ("At the top of the termites nest", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/At the top of the termites nest"),
        ("In the eye of the Mumbo's Skull", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/In the eye of the Mumbo's Skull"),
        ("Feeding Juju", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/Feeding Juju"),
        ("Inside a hut of the village", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/Inside a hut of the village"),
        ("At the center of the stone structure", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/At the center of the stone structure"),
        ("On the hill near entrance", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/On the hill near entrance"),
        ("Give an orange to Chimpy's", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/Give an orange to Chimpy's"),
        ("Conga's Battle", "/Slot 2/Levels/Mumbo's Mountain/General/Jiggies/Conga's Battle"),
    ] },
    Section { title: "Slot 2: Treasure Trove Cove jiggies", rows: &[
        ("Sandcastle password", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/Sandcastle password"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/5 Jinjos rescued"),
        ("At the top of the lighthouse", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/At the top of the lighthouse"),
        ("Shock Spring Pads path", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/Shock Spring Pads path"),
        ("On the backside of the spiral tower", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/On the backside of the spiral tower"),
        ("At the bottom of a pool of water", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/At the bottom of a pool of water"),
        ("Golden treasure marks", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/Golden treasure marks"),
        ("Inside Nipper's Shell", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/Inside Nipper's Shell"),
        ("Inside a Lockup", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/Inside a Lockup"),
        ("Captain Blubber's treasures", "/Slot 2/Levels/Treasure Trove Cove/General/Jiggies/Captain Blubber's treasures"),
    ] },
    Section { title: "Slot 2: Clanker's Cavern jiggies", rows: &[
        ("Above Clanker using his tail", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/Above Clanker using his tail"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/5 Jinjos rescued"),
        ("Battle against Mutie-Snippets", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/Battle against Mutie-Snippets"),
        ("The large key", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/The large key"),
        ("Above Clanker using his bolt", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/Above Clanker using his bolt"),
        ("In a pipe near Clanker", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/In a pipe near Clanker"),
        ("On the Clanker's right golden tooth", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/On the Clanker's right golden tooth"),
        ("The ring challenge", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/The ring challenge"),
        ("Near the Witch Switch", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/Near the Witch Switch"),
        ("Near Bottles using Wonderwing", "/Slot 2/Levels/Clanker's Cavern/General/Jiggies/Near Bottles using Wonderwing"),
    ] },
    Section { title: "Slot 2: Bubblegloop Swamp jiggies", rows: &[
        ("45 seconds race", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/45 seconds race"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/5 Jinjos rescued"),
        ("Mr. Vile game", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/Mr. Vile game"),
        ("Inside the big egg", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/Inside the big egg"),
        ("Feed the Croctus", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/Feed the Croctus"),
        ("In the highest hut of the treetop area", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/In the highest hut of the treetop area"),
        ("Yellow Flibbits battle", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/Yellow Flibbits battle"),
        ("10 seconds race", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/10 seconds race"),
        ("Tanktup's feet", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/Tanktup's feet"),
        ("Musical lessons", "/Slot 2/Levels/Bubblegloop Swamp/General/Jiggies/Musical lessons"),
    ] },
    Section { title: "Slot 2: Freezeezy Peak jiggies", rows: &[
        ("First sled race against Boggy", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/First sled race against Boggy"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/5 Jinjos rescued"),
        ("Inside Boggy's stomach", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/Inside Boggy's stomach"),
        ("Inside the giant snowman's pipe", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/Inside the giant snowman's pipe"),
        ("Second sled race against Boggy", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/Second sled race against Boggy"),
        ("Buttons of giant snowman", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/Buttons of giant snowman"),
        ("Presents to Boggy's kids", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/Presents to Boggy's kids"),
        ("At the top of the christmas tree", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/At the top of the christmas tree"),
        ("Destroy all Sire Slushes", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/Destroy all Sire Slushes"),
        ("Given by Wozza", "/Slot 2/Levels/Freezeezy Peak/General/Jiggies/Given by Wozza"),
    ] },
    Section { title: "Slot 2: Gobi's Valley jiggies", rows: &[
        ("In the tile puzzle pyramid", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/In the tile puzzle pyramid"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/5 Jinjos rescued"),
        ("Stolen from Grabba", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Stolen from Grabba"),
        ("Inside Jinyx", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Inside Jinyx"),
        ("Inside King Sandybutt's sarcophagus", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Inside King Sandybutt's sarcophagus"),
        ("Inside the water pyramid", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Inside the water pyramid"),
        ("Inside Rubee's temple", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Inside Rubee's temple"),
        ("Free Gobi", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Free Gobi"),
        ("Above Trunker", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Above Trunker"),
        ("Flying ring challenge", "/Slot 2/Levels/Gobi's Valley/General/Jiggies/Flying ring challenge"),
    ] },
    Section { title: "Slot 2: Mad Monster Mansion jiggies", rows: &[
        ("After the organ lesson in the church", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/After the organ lesson in the church"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/5 Jinjos rescued"),
        ("Inside the well", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/Inside the well"),
        ("On the table of the dining room", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/On the table of the dining room"),
        ("In a barrel in the cellar", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/In a barrel in the cellar"),
        ("At the top of the church", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/At the top of the church"),
        ("In a basin by using the gutter", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/In a basin by using the gutter"),
        ("Tumblar's game", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/Tumblar's game"),
        ("Flowers in the graveyard", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/Flowers in the graveyard"),
        ("In the sewer after being swallowed by Loggo", "/Slot 2/Levels/Mad Monster Mansion/General/Jiggies/In the sewer after being swallowed by Loggo"),
    ] },
    Section { title: "Slot 2: Rusty Bucket Bay jiggies", rows: &[
        ("In the captain's room", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/In the captain's room"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/5 Jinjos rescued"),
        ("In the warehouse", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/In the warehouse"),
        ("Snorkel the dolphin rescued", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/Snorkel the dolphin rescued"),
        ("3 whistle switches", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/3 whistle switches"),
        ("At the top of a chimney", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/At the top of a chimney"),
        ("Beat Boss Boom Box", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/Beat Boss Boom Box"),
        ("Behind the propellers", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/Behind the propellers"),
        ("In a cage on the ship", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/In a cage on the ship"),
        ("In the engine room", "/Slot 2/Levels/Rusty Bucket Bay/General/Jiggies/In the engine room"),
    ] },
    Section { title: "Slot 2: Click Clock Wood jiggies", rows: &[
        ("At the top of the tree", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/At the top of the tree"),
        ("5 Jinjos rescued", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/5 Jinjos rescued"),
        ("Spring / Winter: At the highest platform of the tree", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Spring ~1 Winter: At the highest platform of the tree"),
        ("Summer: Defeat the Zubbas inside the beehive", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Summer: Defeat the Zubbas inside the beehive"),
        ("Summer / Autumn: In the treehouse", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Summer ~1 Autumn: In the treehouse"),
        ("Summer / Autumn: Following the leaves path", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Summer ~1 Autumn: Following the leaves path"),
        ("Autumn: The acorns of Nabnut", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Autumn: The acorns of Nabnut"),
        ("Autumn: In the house of Gnawty", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Autumn: In the house of Gnawty"),
        ("Autumn: At the top of the eggplant", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Autumn: At the top of the eggplant"),
        ("Winter: Eyrie the eagle", "/Slot 2/Levels/Click Clock Wood/General/Jiggies/Winter: Eyrie the eagle"),
    ] },
    Section { title: "Slot 3: collectibles", rows: &[
        ("Progression", "/Slot 3/General/Progression"),
        ("Jiggies", "/Slot 3/General/Jiggies"),
        ("Mumbo Tokens", "/Slot 3/General/Mumbo Tokens"),
        ("Health", "/Slot 3/General/Health"),
        ("Eggs", "/Slot 3/General/Eggs"),
        ("Eggs max", "/Slot 3/General/Eggs max"),
        ("Red Feathers", "/Slot 3/General/Red Feathers"),
        ("Red Feathers max", "/Slot 3/General/Red Feathers max"),
        ("Gold Feathers", "/Slot 3/General/Gold Feathers"),
        ("Gold Feathers max", "/Slot 3/General/Gold Feathers max"),
    ] },
    Section { title: "Slot 3: basic moves", rows: &[
        ("High Jump", "/Slot 3/Special Moves/Basic Moves/High Jump"),
        ("Feathery Flap", "/Slot 3/Special Moves/Basic Moves/Feathery Flap"),
        ("Flap Flip", "/Slot 3/Special Moves/Basic Moves/Flap Flip"),
        ("Climbing", "/Slot 3/Special Moves/Basic Moves/Climbing"),
        ("Swimming", "/Slot 3/Special Moves/Basic Moves/Swimming"),
        ("Claw Swipe", "/Slot 3/Special Moves/Basic Moves/Claw Swipe"),
        ("Forward Roll Attack", "/Slot 3/Special Moves/Basic Moves/Forward Roll Attack"),
        ("Rat-a-tat Rap", "/Slot 3/Special Moves/Basic Moves/Rat-a-tat Rap"),
        ("Beak Barge", "/Slot 3/Special Moves/Basic Moves/Beak Barge"),
    ] },
    Section { title: "Slot 3: advanced moves", rows: &[
        ("Talon Trot", "/Slot 3/Special Moves/Advanced Moves/Talon Trot"),
        ("Beak Buster", "/Slot 3/Special Moves/Advanced Moves/Beak Buster"),
        ("Egg Firing", "/Slot 3/Special Moves/Advanced Moves/Egg Firing"),
        ("Shock Jump Spring", "/Slot 3/Special Moves/Advanced Moves/Shock Jump Spring"),
        ("Flight", "/Slot 3/Special Moves/Advanced Moves/Flight"),
        ("Wonderwing", "/Slot 3/Special Moves/Advanced Moves/Wonderwing"),
        ("Turbo Talon Trot", "/Slot 3/Special Moves/Advanced Moves/Turbo Talon Trot"),
        ("Stilt Stride", "/Slot 3/Special Moves/Advanced Moves/Stilt Stride"),
        ("Beak Bomb", "/Slot 3/Special Moves/Advanced Moves/Beak Bomb"),
    ] },
    Section { title: "Slot 3: Gruntilda's Lair jiggies", rows: &[
        ("Entrance", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Entrance"),
        ("Witch Switch of Mumbo's Mountain", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Mumbo's Mountain"),
        ("Witch Switch of Treasure Trove Cove", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Treasure Trove Cove"),
        ("Witch Switch of Clanker's Cavern", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Clanker's Cavern"),
        ("Witch Switch of Bubblegloop Swamp", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Bubblegloop Swamp"),
        ("Witch Switch of Freezeezy Peak", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Freezeezy Peak"),
        ("Witch Switch of Gobi's Valley", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Gobi's Valley"),
        ("Witch Switch of Mad Monster Mansion", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Mad Monster Mansion"),
        ("Witch Switch of Rusty Bucket Bay", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Rusty Bucket Bay"),
        ("Witch Switch of Click Clock Wood", "/Slot 3/Levels/Gruntilda's Lair/General/Jiggies/Witch Switch of Click Clock Wood"),
    ] },
    Section { title: "Slot 3: Mumbo's Mountain jiggies", rows: &[
        ("Conga's orange blocks", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/Conga's orange blocks"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/5 Jinjos rescued"),
        ("At the top of the termites nest", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/At the top of the termites nest"),
        ("In the eye of the Mumbo's Skull", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/In the eye of the Mumbo's Skull"),
        ("Feeding Juju", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/Feeding Juju"),
        ("Inside a hut of the village", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/Inside a hut of the village"),
        ("At the center of the stone structure", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/At the center of the stone structure"),
        ("On the hill near entrance", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/On the hill near entrance"),
        ("Give an orange to Chimpy's", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/Give an orange to Chimpy's"),
        ("Conga's Battle", "/Slot 3/Levels/Mumbo's Mountain/General/Jiggies/Conga's Battle"),
    ] },
    Section { title: "Slot 3: Treasure Trove Cove jiggies", rows: &[
        ("Sandcastle password", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/Sandcastle password"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/5 Jinjos rescued"),
        ("At the top of the lighthouse", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/At the top of the lighthouse"),
        ("Shock Spring Pads path", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/Shock Spring Pads path"),
        ("On the backside of the spiral tower", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/On the backside of the spiral tower"),
        ("At the bottom of a pool of water", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/At the bottom of a pool of water"),
        ("Golden treasure marks", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/Golden treasure marks"),
        ("Inside Nipper's Shell", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/Inside Nipper's Shell"),
        ("Inside a Lockup", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/Inside a Lockup"),
        ("Captain Blubber's treasures", "/Slot 3/Levels/Treasure Trove Cove/General/Jiggies/Captain Blubber's treasures"),
    ] },
    Section { title: "Slot 3: Clanker's Cavern jiggies", rows: &[
        ("Above Clanker using his tail", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/Above Clanker using his tail"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/5 Jinjos rescued"),
        ("Battle against Mutie-Snippets", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/Battle against Mutie-Snippets"),
        ("The large key", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/The large key"),
        ("Above Clanker using his bolt", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/Above Clanker using his bolt"),
        ("In a pipe near Clanker", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/In a pipe near Clanker"),
        ("On the Clanker's right golden tooth", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/On the Clanker's right golden tooth"),
        ("The ring challenge", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/The ring challenge"),
        ("Near the Witch Switch", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/Near the Witch Switch"),
        ("Near Bottles using Wonderwing", "/Slot 3/Levels/Clanker's Cavern/General/Jiggies/Near Bottles using Wonderwing"),
    ] },
    Section { title: "Slot 3: Bubblegloop Swamp jiggies", rows: &[
        ("45 seconds race", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/45 seconds race"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/5 Jinjos rescued"),
        ("Mr. Vile game", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/Mr. Vile game"),
        ("Inside the big egg", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/Inside the big egg"),
        ("Feed the Croctus", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/Feed the Croctus"),
        ("In the highest hut of the treetop area", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/In the highest hut of the treetop area"),
        ("Yellow Flibbits battle", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/Yellow Flibbits battle"),
        ("10 seconds race", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/10 seconds race"),
        ("Tanktup's feet", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/Tanktup's feet"),
        ("Musical lessons", "/Slot 3/Levels/Bubblegloop Swamp/General/Jiggies/Musical lessons"),
    ] },
    Section { title: "Slot 3: Freezeezy Peak jiggies", rows: &[
        ("First sled race against Boggy", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/First sled race against Boggy"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/5 Jinjos rescued"),
        ("Inside Boggy's stomach", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/Inside Boggy's stomach"),
        ("Inside the giant snowman's pipe", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/Inside the giant snowman's pipe"),
        ("Second sled race against Boggy", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/Second sled race against Boggy"),
        ("Buttons of giant snowman", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/Buttons of giant snowman"),
        ("Presents to Boggy's kids", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/Presents to Boggy's kids"),
        ("At the top of the christmas tree", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/At the top of the christmas tree"),
        ("Destroy all Sire Slushes", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/Destroy all Sire Slushes"),
        ("Given by Wozza", "/Slot 3/Levels/Freezeezy Peak/General/Jiggies/Given by Wozza"),
    ] },
    Section { title: "Slot 3: Gobi's Valley jiggies", rows: &[
        ("In the tile puzzle pyramid", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/In the tile puzzle pyramid"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/5 Jinjos rescued"),
        ("Stolen from Grabba", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Stolen from Grabba"),
        ("Inside Jinyx", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Inside Jinyx"),
        ("Inside King Sandybutt's sarcophagus", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Inside King Sandybutt's sarcophagus"),
        ("Inside the water pyramid", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Inside the water pyramid"),
        ("Inside Rubee's temple", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Inside Rubee's temple"),
        ("Free Gobi", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Free Gobi"),
        ("Above Trunker", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Above Trunker"),
        ("Flying ring challenge", "/Slot 3/Levels/Gobi's Valley/General/Jiggies/Flying ring challenge"),
    ] },
    Section { title: "Slot 3: Mad Monster Mansion jiggies", rows: &[
        ("After the organ lesson in the church", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/After the organ lesson in the church"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/5 Jinjos rescued"),
        ("Inside the well", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/Inside the well"),
        ("On the table of the dining room", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/On the table of the dining room"),
        ("In a barrel in the cellar", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/In a barrel in the cellar"),
        ("At the top of the church", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/At the top of the church"),
        ("In a basin by using the gutter", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/In a basin by using the gutter"),
        ("Tumblar's game", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/Tumblar's game"),
        ("Flowers in the graveyard", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/Flowers in the graveyard"),
        ("In the sewer after being swallowed by Loggo", "/Slot 3/Levels/Mad Monster Mansion/General/Jiggies/In the sewer after being swallowed by Loggo"),
    ] },
    Section { title: "Slot 3: Rusty Bucket Bay jiggies", rows: &[
        ("In the captain's room", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/In the captain's room"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/5 Jinjos rescued"),
        ("In the warehouse", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/In the warehouse"),
        ("Snorkel the dolphin rescued", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/Snorkel the dolphin rescued"),
        ("3 whistle switches", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/3 whistle switches"),
        ("At the top of a chimney", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/At the top of a chimney"),
        ("Beat Boss Boom Box", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/Beat Boss Boom Box"),
        ("Behind the propellers", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/Behind the propellers"),
        ("In a cage on the ship", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/In a cage on the ship"),
        ("In the engine room", "/Slot 3/Levels/Rusty Bucket Bay/General/Jiggies/In the engine room"),
    ] },
    Section { title: "Slot 3: Click Clock Wood jiggies", rows: &[
        ("At the top of the tree", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/At the top of the tree"),
        ("5 Jinjos rescued", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/5 Jinjos rescued"),
        ("Spring / Winter: At the highest platform of the tree", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Spring ~1 Winter: At the highest platform of the tree"),
        ("Summer: Defeat the Zubbas inside the beehive", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Summer: Defeat the Zubbas inside the beehive"),
        ("Summer / Autumn: In the treehouse", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Summer ~1 Autumn: In the treehouse"),
        ("Summer / Autumn: Following the leaves path", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Summer ~1 Autumn: Following the leaves path"),
        ("Autumn: The acorns of Nabnut", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Autumn: The acorns of Nabnut"),
        ("Autumn: In the house of Gnawty", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Autumn: In the house of Gnawty"),
        ("Autumn: At the top of the eggplant", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Autumn: At the top of the eggplant"),
        ("Winter: Eyrie the eagle", "/Slot 3/Levels/Click Clock Wood/General/Jiggies/Winter: Eyrie the eagle"),
    ] },
    Section { title: "Stop 'n' Swop", rows: &[
        ("Unlocked: Ice Key", "/Stop 'n' Swop/Unlocked/Ice Key"),
        ("Unlocked: Cyan Mystery Egg", "/Stop 'n' Swop/Unlocked/Cyan Mystery Egg"),
        ("Unlocked: Pink Mystery Egg", "/Stop 'n' Swop/Unlocked/Pink Mystery Egg"),
        ("Unlocked: Blue Mystery Egg", "/Stop 'n' Swop/Unlocked/Blue Mystery Egg"),
        ("Unlocked: Green Mystery Egg", "/Stop 'n' Swop/Unlocked/Green Mystery Egg"),
        ("Unlocked: Red Mystery Egg", "/Stop 'n' Swop/Unlocked/Red Mystery Egg"),
        ("Unlocked: Yellow Mystery Egg", "/Stop 'n' Swop/Unlocked/Yellow Mystery Egg"),
        ("Obtained: Ice Key", "/Stop 'n' Swop/Obtained/Ice Key"),
        ("Obtained: Cyan Mystery Egg", "/Stop 'n' Swop/Obtained/Cyan Mystery Egg"),
        ("Obtained: Pink Mystery Egg", "/Stop 'n' Swop/Obtained/Pink Mystery Egg"),
        ("Obtained: Blue Mystery Egg", "/Stop 'n' Swop/Obtained/Blue Mystery Egg"),
        ("Obtained: Green Mystery Egg", "/Stop 'n' Swop/Obtained/Green Mystery Egg"),
        ("Obtained: Red Mystery Egg", "/Stop 'n' Swop/Obtained/Red Mystery Egg"),
        ("Obtained: Yellow Mystery Egg", "/Stop 'n' Swop/Obtained/Yellow Mystery Egg"),
    ] },
    ],
    lists: &[],
    tables: &[
    ],
};

// end BANJO_KAZOOIE

// ------------------------------------------------------------------ The Legend of Zelda: Ocarina of Time (gen)

const OOT: Spec = Spec {
    name: "The Legend of Zelda: Ocarina of Time",
    tab: "Link",
    accent: (60, 160, 90),
    kind: Some(crate::fmt::zelda64::KIND),
    markers: &["/Info/Files in use"],
    subtitle: "An emulator's .sra, .srm or GameCube .gci save. Per file: health, magic, rupees, equipment, every item and its ammo, the songs, medallions and stones, and the dungeon maps, compasses and keys.",
    tiles: &[("Files in use", "/Info/Files in use")],
    sections: &[
    Section { title: "File 1: general", rows: &[
        ("Death Count", "/File 1/General/Death Count"),
        ("Age", "/File 1/General/Age"),
        ("Location", "/File 1/General/Location"),
        ("Health (sixteenths of a heart)", "/File 1/General/Health (sixteenths of a heart)"),
        ("Max health (sixteenths of a heart)", "/File 1/General/Max health (sixteenths of a heart)"),
        ("Heart Pieces", "/File 1/General/Heart Pieces"),
        ("Magic", "/File 1/General/Magic"),
        ("Rupees", "/File 1/General/Rupees"),
        ("Rupees max", "/File 1/General/Rupees max"),
        ("Magic Level", "/File 1/General/Magic Level"),
        ("Double Defense", "/File 1/General/Double Defense"),
    ] },
    Section { title: "File 1: equipment", rows: &[
        ("Sword", "/File 1/Inventory/Equipment/Sword"),
        ("Kokiri Sword", "/File 1/Inventory/Equipment/Kokiri Sword"),
        ("Master Sword", "/File 1/Inventory/Equipment/Master Sword"),
        ("Shield", "/File 1/Inventory/Equipment/Shield"),
        ("Deku Shield", "/File 1/Inventory/Equipment/Deku Shield"),
        ("Hylian Shield", "/File 1/Inventory/Equipment/Hylian Shield"),
        ("Mirror Shield", "/File 1/Inventory/Equipment/Mirror Shield"),
        ("Tunic", "/File 1/Inventory/Equipment/Tunic"),
        ("Kokiri Tunic", "/File 1/Inventory/Equipment/Kokiri Tunic"),
        ("Goron Tunic", "/File 1/Inventory/Equipment/Goron Tunic"),
        ("Zora Tunic", "/File 1/Inventory/Equipment/Zora Tunic"),
        ("Boots", "/File 1/Inventory/Equipment/Boots"),
        ("Kokiri Boots", "/File 1/Inventory/Equipment/Kokiri Boots"),
        ("Iron Boots", "/File 1/Inventory/Equipment/Iron Boots"),
        ("Hover Boots", "/File 1/Inventory/Equipment/Hover Boots"),
        ("Goron Sword", "/File 1/Inventory/Equipment/Goron Sword"),
        ("Giant's Knife Health", "/File 1/Inventory/Equipment/Giant's Knife Health"),
        ("Wallet", "/File 1/Inventory/Equipment/Wallet"),
        ("Deku Seeds Bullet Bag", "/File 1/Inventory/Equipment/Deku Seeds Bullet Bag"),
        ("Quiver", "/File 1/Inventory/Equipment/Quiver"),
        ("Bomb Bag", "/File 1/Inventory/Equipment/Bomb Bag"),
        ("Bracelet / Gauntlets", "/File 1/Inventory/Equipment/Bracelet ~1 Gauntlets"),
        ("Scale", "/File 1/Inventory/Equipment/Scale"),
        ("C-Left", "/File 1/Inventory/Equipment/C-Left"),
        ("C-Down", "/File 1/Inventory/Equipment/C-Down"),
        ("C-Right", "/File 1/Inventory/Equipment/C-Right"),
    ] },
    Section { title: "File 1: items", rows: &[
        ("Deku Stick", "/File 1/Inventory/Items/Deku Stick"),
        ("Deku Nut", "/File 1/Inventory/Items/Deku Nut"),
        ("Bomb", "/File 1/Inventory/Items/Bomb"),
        ("Fairy Bow", "/File 1/Inventory/Items/Fairy Bow"),
        ("Fire Arrow", "/File 1/Inventory/Items/Fire Arrow"),
        ("Ice Arrow", "/File 1/Inventory/Items/Ice Arrow"),
        ("Light Arrow", "/File 1/Inventory/Items/Light Arrow"),
        ("Fairy Slingshot", "/File 1/Inventory/Items/Fairy Slingshot"),
        ("Bombchu", "/File 1/Inventory/Items/Bombchu"),
        ("Boomerang", "/File 1/Inventory/Items/Boomerang"),
        ("Lens of Truth", "/File 1/Inventory/Items/Lens of Truth"),
        ("Magic Beans", "/File 1/Inventory/Items/Magic Beans"),
        ("Megaton Hammer", "/File 1/Inventory/Items/Megaton Hammer"),
        ("Din's Fire", "/File 1/Inventory/Items/Din's Fire"),
        ("Farore's Wind", "/File 1/Inventory/Items/Farore's Wind"),
        ("Nayru's Love", "/File 1/Inventory/Items/Nayru's Love"),
        ("Ocarina", "/File 1/Inventory/Items/Ocarina"),
        ("Hookshot", "/File 1/Inventory/Items/Hookshot"),
        ("Bottle 1", "/File 1/Inventory/Items/Bottle 1"),
        ("Bottle 2", "/File 1/Inventory/Items/Bottle 2"),
        ("Bottle 3", "/File 1/Inventory/Items/Bottle 3"),
        ("Bottle 4", "/File 1/Inventory/Items/Bottle 4"),
        ("Adult Trading Item", "/File 1/Inventory/Items/Adult Trading Item"),
        ("Child Trading Item", "/File 1/Inventory/Items/Child Trading Item"),
    ] },
    Section { title: "File 1: ammunition and upgrades", rows: &[
        ("Deku Sticks", "/File 1/Inventory/Quantities/Deku Sticks"),
        ("Deku Sticks max", "/File 1/Inventory/Quantities/Deku Sticks max"),
        ("Deku Nuts", "/File 1/Inventory/Quantities/Deku Nuts"),
        ("Deku Nuts max", "/File 1/Inventory/Quantities/Deku Nuts max"),
        ("Bombs", "/File 1/Inventory/Quantities/Bombs"),
        ("Bombs max", "/File 1/Inventory/Quantities/Bombs max"),
        ("Arrows", "/File 1/Inventory/Quantities/Arrows"),
        ("Arrows max", "/File 1/Inventory/Quantities/Arrows max"),
        ("Deku Seeds", "/File 1/Inventory/Quantities/Deku Seeds"),
        ("Deku Seeds max", "/File 1/Inventory/Quantities/Deku Seeds max"),
        ("Bombchu", "/File 1/Inventory/Quantities/Bombchu"),
        ("Magic Beans", "/File 1/Inventory/Quantities/Magic Beans"),
    ] },
    Section { title: "File 1: songs", rows: &[
        ("Zelda's Lullaby", "/File 1/Inventory/Quest Items/Songs/Zelda's Lullaby"),
        ("Epona's Song", "/File 1/Inventory/Quest Items/Songs/Epona's Song"),
        ("Saria's Song", "/File 1/Inventory/Quest Items/Songs/Saria's Song"),
        ("Sun's Song", "/File 1/Inventory/Quest Items/Songs/Sun's Song"),
        ("Song of Time", "/File 1/Inventory/Quest Items/Songs/Song of Time"),
        ("Song of Storms", "/File 1/Inventory/Quest Items/Songs/Song of Storms"),
        ("Minuet of Forest", "/File 1/Inventory/Quest Items/Songs/Minuet of Forest"),
        ("Bolero of Fire", "/File 1/Inventory/Quest Items/Songs/Bolero of Fire"),
        ("Serenade of Water", "/File 1/Inventory/Quest Items/Songs/Serenade of Water"),
        ("Requiem of Spirit", "/File 1/Inventory/Quest Items/Songs/Requiem of Spirit"),
        ("Nocturne of Shadow", "/File 1/Inventory/Quest Items/Songs/Nocturne of Shadow"),
        ("Predule of Light", "/File 1/Inventory/Quest Items/Songs/Predule of Light"),
    ] },
    Section { title: "File 1: medallions", rows: &[
        ("Forest Medallion", "/File 1/Inventory/Quest Items/Medaillions/Forest Medallion"),
        ("Fire Medallion", "/File 1/Inventory/Quest Items/Medaillions/Fire Medallion"),
        ("Water Medallion", "/File 1/Inventory/Quest Items/Medaillions/Water Medallion"),
        ("Spirit Medallion", "/File 1/Inventory/Quest Items/Medaillions/Spirit Medallion"),
        ("Shadow Medallion", "/File 1/Inventory/Quest Items/Medaillions/Shadow Medallion"),
        ("Light Medallion", "/File 1/Inventory/Quest Items/Medaillions/Light Medallion"),
    ] },
    Section { title: "File 1: spiritual stones", rows: &[
        ("Kokiri's Emerald", "/File 1/Inventory/Quest Items/Spiritual Stones/Kokiri's Emerald"),
        ("Goron's Ruby", "/File 1/Inventory/Quest Items/Spiritual Stones/Goron's Ruby"),
        ("Zora's Sapphire", "/File 1/Inventory/Quest Items/Spiritual Stones/Zora's Sapphire"),
    ] },
    Section { title: "File 1: miscellaneous", rows: &[
        ("Stone of Agony", "/File 1/Inventory/Quest Items/Miscellaneous/Stone of Agony"),
        ("Gerudo's Card", "/File 1/Inventory/Quest Items/Miscellaneous/Gerudo's Card"),
    ] },
    Section { title: "File 2: general", rows: &[
        ("Death Count", "/File 2/General/Death Count"),
        ("Age", "/File 2/General/Age"),
        ("Location", "/File 2/General/Location"),
        ("Health (sixteenths of a heart)", "/File 2/General/Health (sixteenths of a heart)"),
        ("Max health (sixteenths of a heart)", "/File 2/General/Max health (sixteenths of a heart)"),
        ("Heart Pieces", "/File 2/General/Heart Pieces"),
        ("Magic", "/File 2/General/Magic"),
        ("Rupees", "/File 2/General/Rupees"),
        ("Rupees max", "/File 2/General/Rupees max"),
        ("Magic Level", "/File 2/General/Magic Level"),
        ("Double Defense", "/File 2/General/Double Defense"),
    ] },
    Section { title: "File 2: equipment", rows: &[
        ("Sword", "/File 2/Inventory/Equipment/Sword"),
        ("Kokiri Sword", "/File 2/Inventory/Equipment/Kokiri Sword"),
        ("Master Sword", "/File 2/Inventory/Equipment/Master Sword"),
        ("Shield", "/File 2/Inventory/Equipment/Shield"),
        ("Deku Shield", "/File 2/Inventory/Equipment/Deku Shield"),
        ("Hylian Shield", "/File 2/Inventory/Equipment/Hylian Shield"),
        ("Mirror Shield", "/File 2/Inventory/Equipment/Mirror Shield"),
        ("Tunic", "/File 2/Inventory/Equipment/Tunic"),
        ("Kokiri Tunic", "/File 2/Inventory/Equipment/Kokiri Tunic"),
        ("Goron Tunic", "/File 2/Inventory/Equipment/Goron Tunic"),
        ("Zora Tunic", "/File 2/Inventory/Equipment/Zora Tunic"),
        ("Boots", "/File 2/Inventory/Equipment/Boots"),
        ("Kokiri Boots", "/File 2/Inventory/Equipment/Kokiri Boots"),
        ("Iron Boots", "/File 2/Inventory/Equipment/Iron Boots"),
        ("Hover Boots", "/File 2/Inventory/Equipment/Hover Boots"),
        ("Goron Sword", "/File 2/Inventory/Equipment/Goron Sword"),
        ("Giant's Knife Health", "/File 2/Inventory/Equipment/Giant's Knife Health"),
        ("Wallet", "/File 2/Inventory/Equipment/Wallet"),
        ("Deku Seeds Bullet Bag", "/File 2/Inventory/Equipment/Deku Seeds Bullet Bag"),
        ("Quiver", "/File 2/Inventory/Equipment/Quiver"),
        ("Bomb Bag", "/File 2/Inventory/Equipment/Bomb Bag"),
        ("Bracelet / Gauntlets", "/File 2/Inventory/Equipment/Bracelet ~1 Gauntlets"),
        ("Scale", "/File 2/Inventory/Equipment/Scale"),
        ("C-Left", "/File 2/Inventory/Equipment/C-Left"),
        ("C-Down", "/File 2/Inventory/Equipment/C-Down"),
        ("C-Right", "/File 2/Inventory/Equipment/C-Right"),
    ] },
    Section { title: "File 2: items", rows: &[
        ("Deku Stick", "/File 2/Inventory/Items/Deku Stick"),
        ("Deku Nut", "/File 2/Inventory/Items/Deku Nut"),
        ("Bomb", "/File 2/Inventory/Items/Bomb"),
        ("Fairy Bow", "/File 2/Inventory/Items/Fairy Bow"),
        ("Fire Arrow", "/File 2/Inventory/Items/Fire Arrow"),
        ("Ice Arrow", "/File 2/Inventory/Items/Ice Arrow"),
        ("Light Arrow", "/File 2/Inventory/Items/Light Arrow"),
        ("Fairy Slingshot", "/File 2/Inventory/Items/Fairy Slingshot"),
        ("Bombchu", "/File 2/Inventory/Items/Bombchu"),
        ("Boomerang", "/File 2/Inventory/Items/Boomerang"),
        ("Lens of Truth", "/File 2/Inventory/Items/Lens of Truth"),
        ("Magic Beans", "/File 2/Inventory/Items/Magic Beans"),
        ("Megaton Hammer", "/File 2/Inventory/Items/Megaton Hammer"),
        ("Din's Fire", "/File 2/Inventory/Items/Din's Fire"),
        ("Farore's Wind", "/File 2/Inventory/Items/Farore's Wind"),
        ("Nayru's Love", "/File 2/Inventory/Items/Nayru's Love"),
        ("Ocarina", "/File 2/Inventory/Items/Ocarina"),
        ("Hookshot", "/File 2/Inventory/Items/Hookshot"),
        ("Bottle 1", "/File 2/Inventory/Items/Bottle 1"),
        ("Bottle 2", "/File 2/Inventory/Items/Bottle 2"),
        ("Bottle 3", "/File 2/Inventory/Items/Bottle 3"),
        ("Bottle 4", "/File 2/Inventory/Items/Bottle 4"),
        ("Adult Trading Item", "/File 2/Inventory/Items/Adult Trading Item"),
        ("Child Trading Item", "/File 2/Inventory/Items/Child Trading Item"),
    ] },
    Section { title: "File 2: ammunition and upgrades", rows: &[
        ("Deku Sticks", "/File 2/Inventory/Quantities/Deku Sticks"),
        ("Deku Sticks max", "/File 2/Inventory/Quantities/Deku Sticks max"),
        ("Deku Nuts", "/File 2/Inventory/Quantities/Deku Nuts"),
        ("Deku Nuts max", "/File 2/Inventory/Quantities/Deku Nuts max"),
        ("Bombs", "/File 2/Inventory/Quantities/Bombs"),
        ("Bombs max", "/File 2/Inventory/Quantities/Bombs max"),
        ("Arrows", "/File 2/Inventory/Quantities/Arrows"),
        ("Arrows max", "/File 2/Inventory/Quantities/Arrows max"),
        ("Deku Seeds", "/File 2/Inventory/Quantities/Deku Seeds"),
        ("Deku Seeds max", "/File 2/Inventory/Quantities/Deku Seeds max"),
        ("Bombchu", "/File 2/Inventory/Quantities/Bombchu"),
        ("Magic Beans", "/File 2/Inventory/Quantities/Magic Beans"),
    ] },
    Section { title: "File 2: songs", rows: &[
        ("Zelda's Lullaby", "/File 2/Inventory/Quest Items/Songs/Zelda's Lullaby"),
        ("Epona's Song", "/File 2/Inventory/Quest Items/Songs/Epona's Song"),
        ("Saria's Song", "/File 2/Inventory/Quest Items/Songs/Saria's Song"),
        ("Sun's Song", "/File 2/Inventory/Quest Items/Songs/Sun's Song"),
        ("Song of Time", "/File 2/Inventory/Quest Items/Songs/Song of Time"),
        ("Song of Storms", "/File 2/Inventory/Quest Items/Songs/Song of Storms"),
        ("Minuet of Forest", "/File 2/Inventory/Quest Items/Songs/Minuet of Forest"),
        ("Bolero of Fire", "/File 2/Inventory/Quest Items/Songs/Bolero of Fire"),
        ("Serenade of Water", "/File 2/Inventory/Quest Items/Songs/Serenade of Water"),
        ("Requiem of Spirit", "/File 2/Inventory/Quest Items/Songs/Requiem of Spirit"),
        ("Nocturne of Shadow", "/File 2/Inventory/Quest Items/Songs/Nocturne of Shadow"),
        ("Predule of Light", "/File 2/Inventory/Quest Items/Songs/Predule of Light"),
    ] },
    Section { title: "File 2: medallions", rows: &[
        ("Forest Medallion", "/File 2/Inventory/Quest Items/Medaillions/Forest Medallion"),
        ("Fire Medallion", "/File 2/Inventory/Quest Items/Medaillions/Fire Medallion"),
        ("Water Medallion", "/File 2/Inventory/Quest Items/Medaillions/Water Medallion"),
        ("Spirit Medallion", "/File 2/Inventory/Quest Items/Medaillions/Spirit Medallion"),
        ("Shadow Medallion", "/File 2/Inventory/Quest Items/Medaillions/Shadow Medallion"),
        ("Light Medallion", "/File 2/Inventory/Quest Items/Medaillions/Light Medallion"),
    ] },
    Section { title: "File 2: spiritual stones", rows: &[
        ("Kokiri's Emerald", "/File 2/Inventory/Quest Items/Spiritual Stones/Kokiri's Emerald"),
        ("Goron's Ruby", "/File 2/Inventory/Quest Items/Spiritual Stones/Goron's Ruby"),
        ("Zora's Sapphire", "/File 2/Inventory/Quest Items/Spiritual Stones/Zora's Sapphire"),
    ] },
    Section { title: "File 2: miscellaneous", rows: &[
        ("Stone of Agony", "/File 2/Inventory/Quest Items/Miscellaneous/Stone of Agony"),
        ("Gerudo's Card", "/File 2/Inventory/Quest Items/Miscellaneous/Gerudo's Card"),
    ] },
    Section { title: "File 3: general", rows: &[
        ("Death Count", "/File 3/General/Death Count"),
        ("Age", "/File 3/General/Age"),
        ("Location", "/File 3/General/Location"),
        ("Health (sixteenths of a heart)", "/File 3/General/Health (sixteenths of a heart)"),
        ("Max health (sixteenths of a heart)", "/File 3/General/Max health (sixteenths of a heart)"),
        ("Heart Pieces", "/File 3/General/Heart Pieces"),
        ("Magic", "/File 3/General/Magic"),
        ("Rupees", "/File 3/General/Rupees"),
        ("Rupees max", "/File 3/General/Rupees max"),
        ("Magic Level", "/File 3/General/Magic Level"),
        ("Double Defense", "/File 3/General/Double Defense"),
    ] },
    Section { title: "File 3: equipment", rows: &[
        ("Sword", "/File 3/Inventory/Equipment/Sword"),
        ("Kokiri Sword", "/File 3/Inventory/Equipment/Kokiri Sword"),
        ("Master Sword", "/File 3/Inventory/Equipment/Master Sword"),
        ("Shield", "/File 3/Inventory/Equipment/Shield"),
        ("Deku Shield", "/File 3/Inventory/Equipment/Deku Shield"),
        ("Hylian Shield", "/File 3/Inventory/Equipment/Hylian Shield"),
        ("Mirror Shield", "/File 3/Inventory/Equipment/Mirror Shield"),
        ("Tunic", "/File 3/Inventory/Equipment/Tunic"),
        ("Kokiri Tunic", "/File 3/Inventory/Equipment/Kokiri Tunic"),
        ("Goron Tunic", "/File 3/Inventory/Equipment/Goron Tunic"),
        ("Zora Tunic", "/File 3/Inventory/Equipment/Zora Tunic"),
        ("Boots", "/File 3/Inventory/Equipment/Boots"),
        ("Kokiri Boots", "/File 3/Inventory/Equipment/Kokiri Boots"),
        ("Iron Boots", "/File 3/Inventory/Equipment/Iron Boots"),
        ("Hover Boots", "/File 3/Inventory/Equipment/Hover Boots"),
        ("Goron Sword", "/File 3/Inventory/Equipment/Goron Sword"),
        ("Giant's Knife Health", "/File 3/Inventory/Equipment/Giant's Knife Health"),
        ("Wallet", "/File 3/Inventory/Equipment/Wallet"),
        ("Deku Seeds Bullet Bag", "/File 3/Inventory/Equipment/Deku Seeds Bullet Bag"),
        ("Quiver", "/File 3/Inventory/Equipment/Quiver"),
        ("Bomb Bag", "/File 3/Inventory/Equipment/Bomb Bag"),
        ("Bracelet / Gauntlets", "/File 3/Inventory/Equipment/Bracelet ~1 Gauntlets"),
        ("Scale", "/File 3/Inventory/Equipment/Scale"),
        ("C-Left", "/File 3/Inventory/Equipment/C-Left"),
        ("C-Down", "/File 3/Inventory/Equipment/C-Down"),
        ("C-Right", "/File 3/Inventory/Equipment/C-Right"),
    ] },
    Section { title: "File 3: items", rows: &[
        ("Deku Stick", "/File 3/Inventory/Items/Deku Stick"),
        ("Deku Nut", "/File 3/Inventory/Items/Deku Nut"),
        ("Bomb", "/File 3/Inventory/Items/Bomb"),
        ("Fairy Bow", "/File 3/Inventory/Items/Fairy Bow"),
        ("Fire Arrow", "/File 3/Inventory/Items/Fire Arrow"),
        ("Ice Arrow", "/File 3/Inventory/Items/Ice Arrow"),
        ("Light Arrow", "/File 3/Inventory/Items/Light Arrow"),
        ("Fairy Slingshot", "/File 3/Inventory/Items/Fairy Slingshot"),
        ("Bombchu", "/File 3/Inventory/Items/Bombchu"),
        ("Boomerang", "/File 3/Inventory/Items/Boomerang"),
        ("Lens of Truth", "/File 3/Inventory/Items/Lens of Truth"),
        ("Magic Beans", "/File 3/Inventory/Items/Magic Beans"),
        ("Megaton Hammer", "/File 3/Inventory/Items/Megaton Hammer"),
        ("Din's Fire", "/File 3/Inventory/Items/Din's Fire"),
        ("Farore's Wind", "/File 3/Inventory/Items/Farore's Wind"),
        ("Nayru's Love", "/File 3/Inventory/Items/Nayru's Love"),
        ("Ocarina", "/File 3/Inventory/Items/Ocarina"),
        ("Hookshot", "/File 3/Inventory/Items/Hookshot"),
        ("Bottle 1", "/File 3/Inventory/Items/Bottle 1"),
        ("Bottle 2", "/File 3/Inventory/Items/Bottle 2"),
        ("Bottle 3", "/File 3/Inventory/Items/Bottle 3"),
        ("Bottle 4", "/File 3/Inventory/Items/Bottle 4"),
        ("Adult Trading Item", "/File 3/Inventory/Items/Adult Trading Item"),
        ("Child Trading Item", "/File 3/Inventory/Items/Child Trading Item"),
    ] },
    Section { title: "File 3: ammunition and upgrades", rows: &[
        ("Deku Sticks", "/File 3/Inventory/Quantities/Deku Sticks"),
        ("Deku Sticks max", "/File 3/Inventory/Quantities/Deku Sticks max"),
        ("Deku Nuts", "/File 3/Inventory/Quantities/Deku Nuts"),
        ("Deku Nuts max", "/File 3/Inventory/Quantities/Deku Nuts max"),
        ("Bombs", "/File 3/Inventory/Quantities/Bombs"),
        ("Bombs max", "/File 3/Inventory/Quantities/Bombs max"),
        ("Arrows", "/File 3/Inventory/Quantities/Arrows"),
        ("Arrows max", "/File 3/Inventory/Quantities/Arrows max"),
        ("Deku Seeds", "/File 3/Inventory/Quantities/Deku Seeds"),
        ("Deku Seeds max", "/File 3/Inventory/Quantities/Deku Seeds max"),
        ("Bombchu", "/File 3/Inventory/Quantities/Bombchu"),
        ("Magic Beans", "/File 3/Inventory/Quantities/Magic Beans"),
    ] },
    Section { title: "File 3: songs", rows: &[
        ("Zelda's Lullaby", "/File 3/Inventory/Quest Items/Songs/Zelda's Lullaby"),
        ("Epona's Song", "/File 3/Inventory/Quest Items/Songs/Epona's Song"),
        ("Saria's Song", "/File 3/Inventory/Quest Items/Songs/Saria's Song"),
        ("Sun's Song", "/File 3/Inventory/Quest Items/Songs/Sun's Song"),
        ("Song of Time", "/File 3/Inventory/Quest Items/Songs/Song of Time"),
        ("Song of Storms", "/File 3/Inventory/Quest Items/Songs/Song of Storms"),
        ("Minuet of Forest", "/File 3/Inventory/Quest Items/Songs/Minuet of Forest"),
        ("Bolero of Fire", "/File 3/Inventory/Quest Items/Songs/Bolero of Fire"),
        ("Serenade of Water", "/File 3/Inventory/Quest Items/Songs/Serenade of Water"),
        ("Requiem of Spirit", "/File 3/Inventory/Quest Items/Songs/Requiem of Spirit"),
        ("Nocturne of Shadow", "/File 3/Inventory/Quest Items/Songs/Nocturne of Shadow"),
        ("Predule of Light", "/File 3/Inventory/Quest Items/Songs/Predule of Light"),
    ] },
    Section { title: "File 3: medallions", rows: &[
        ("Forest Medallion", "/File 3/Inventory/Quest Items/Medaillions/Forest Medallion"),
        ("Fire Medallion", "/File 3/Inventory/Quest Items/Medaillions/Fire Medallion"),
        ("Water Medallion", "/File 3/Inventory/Quest Items/Medaillions/Water Medallion"),
        ("Spirit Medallion", "/File 3/Inventory/Quest Items/Medaillions/Spirit Medallion"),
        ("Shadow Medallion", "/File 3/Inventory/Quest Items/Medaillions/Shadow Medallion"),
        ("Light Medallion", "/File 3/Inventory/Quest Items/Medaillions/Light Medallion"),
    ] },
    Section { title: "File 3: spiritual stones", rows: &[
        ("Kokiri's Emerald", "/File 3/Inventory/Quest Items/Spiritual Stones/Kokiri's Emerald"),
        ("Goron's Ruby", "/File 3/Inventory/Quest Items/Spiritual Stones/Goron's Ruby"),
        ("Zora's Sapphire", "/File 3/Inventory/Quest Items/Spiritual Stones/Zora's Sapphire"),
    ] },
    Section { title: "File 3: miscellaneous", rows: &[
        ("Stone of Agony", "/File 3/Inventory/Quest Items/Miscellaneous/Stone of Agony"),
        ("Gerudo's Card", "/File 3/Inventory/Quest Items/Miscellaneous/Gerudo's Card"),
    ] },
    ],
    lists: &[],
    tables: &[
        Table { title: "File 1: dungeon items", path: "/File 1/Dungeons", name: &[], cols: &[("Map", "/Items/Dungeon Map"), ("Compass", "/Items/Compass"), ("Boss key", "/Items/Boss Key")], only: &[], names: &[] },
        Table { title: "File 2: dungeon items", path: "/File 2/Dungeons", name: &[], cols: &[("Map", "/Items/Dungeon Map"), ("Compass", "/Items/Compass"), ("Boss key", "/Items/Boss Key")], only: &[], names: &[] },
        Table { title: "File 3: dungeon items", path: "/File 3/Dungeons", name: &[], cols: &[("Map", "/Items/Dungeon Map"), ("Compass", "/Items/Compass"), ("Boss key", "/Items/Boss Key")], only: &[], names: &[] },
    ],
};

// end OOT

// ------------------------------------------------------------------ Super Smash Bros. (N64) (gen)

const SMASH64: Spec = Spec {
    name: "Super Smash Bros. (N64)",
    tab: "Fighters",
    accent: (230, 80, 60),
    kind: Some(crate::fmt::smash64::KIND),
    markers: &["/Options/Sound"],
    subtitle: "An emulator's .sra or .srm save. The unlocked fighters and stages, the 1P records of every fighter, the 2P records and the options.",
    tiles: &[],
    sections: &[
    Section { title: "Unlocked fighters", rows: &[
        ("Luigi", "/General/Unlocked Challengers/Luigi"),
        ("Captain Falcon", "/General/Unlocked Challengers/Captain Falcon"),
        ("Jigglypuff", "/General/Unlocked Challengers/Jigglypuff"),
        ("Ness", "/General/Unlocked Challengers/Ness"),
    ] },
    Section { title: "Unlocked fighters", rows: &[
        ("Luigi", "/General/Unlocked Challengers/Luigi"),
        ("Captain Falcon", "/General/Unlocked Challengers/Captain Falcon"),
        ("Jigglypuff", "/General/Unlocked Challengers/Jigglypuff"),
        ("Ness", "/General/Unlocked Challengers/Ness"),
    ] },
    Section { title: "Unlocked stage", rows: &[
        ("Mushroom Kingdom", "/General/Unlocked Options/Mushroom Kingdom"),
        ("Sound Test", "/General/Unlocked Options/Sound Test"),
        ("Item Switch", "/General/Unlocked Options/Item Switch"),
    ] },
    Section { title: "Options", rows: &[
        ("Sound", "/Options/Sound"),
        ("Language", "/Options/Language"),
    ] },
    ],
    lists: &[],
    tables: &[
        Table { title: "1P Game (Arcade)", path: "/1P Records/Arcade", name: &[], cols: &[("Cleared Difficulty", "/Cleared Difficulty"), ("High Score", "/High Score"), ("Bonus", "/Bonus")], only: &[], names: &[] },
        Table { title: "Break the Targets", path: "/1P Records/Break the Targets", name: &[], cols: &[("Time (1/60 s)", "/Time (1/60 s)"), ("Count", "/Count")], only: &[], names: &[] },
        Table { title: "Board the Platforms", path: "/1P Records/Board the Platforms", name: &[], cols: &[("Time (1/60 s)", "/Time (1/60 s)"), ("Count", "/Count")], only: &[], names: &[] },
        Table { title: "2P records", path: "/2P Records", name: &[], cols: &[("TKOs", "/TKOs"), ("Time (seconds)", "/Time (seconds)"), ("Attack Total", "/Attack Total"), ("Damage Total", "/Damage Total")], only: &[], names: &[] },
    ],
};

// end SMASH64

// ------------------------------------------------------------------ Torchlight III

const TORCHLIGHT3: Spec = Spec {
    name: "Torchlight III",
    tab: "Character",
    accent: (214, 120, 40),
    kind: None,
    markers: &["/playerClassRowName", "/traits", "/itemInventory/entries"],
    subtitle: tx("<id>-<n>-CharacterData.sav in %LOCALAPPDATA%\\Frontiers\\Saved\\SaveGames. The save is an Unreal file holding the whole character as JSON text; it is opened and written back as that string. Close the game first."),
    tiles: &[],
    sections: &[],
    lists: &[],
    tables: &[
        Table {
            title: tx("Character"),
            path: "/traits",
            name: &["/traitName", "/param"],
            cols: &[(tx("Value"), "/value")],
            only: &["Level 0", "Hitpoints 0", "ExperiencePercent 0", "TotalLevelSkillPoints 0", "SkillRegretPoints 0", "Difficulty 0", "LowestDifficulty 0"],
            names: &[],
        },
        Table {
            title: tx("Equipped and carried gear"),
            path: "/itemInventory/entries",
            name: &["/itemDataRow"],
            cols: &[(tx("Level"), "/level"), (tx("Quantity"), "/quantity")],
            only: &[],
            names: &[],
        },
        Table {
            title: tx("Consumables"),
            path: "/consumableInventory/entries",
            name: &["/itemDataRow"],
            cols: &[(tx("Level"), "/level"), (tx("Quantity"), "/quantity")],
            only: &[],
            names: &[],
        },
        Table {
            title: tx("Pet gear"),
            path: "/petInventory/entries",
            name: &["/itemDataRow"],
            cols: &[(tx("Level"), "/level"), (tx("Quantity"), "/quantity")],
            only: &[],
            names: &[],
        },
    ],
};

const WRECKFEST: Spec = Spec {
    name: "Wreckfest",
    tab: "Driver",
    accent: (230, 120, 40),
    kind: Some(crate::fmt::wreckfest::KIND),
    markers: &[],
    subtitle: "career.cres from Steam\\userdata\\<account>\\228380\\local\\wreckfest, Documents\\My Games\\wreckfest or the GOG profile folder: credits, experience and tournament fame. The cars file opens as hex.",
    tiles: &[("Credits", "/Driver/Credits"), ("Experience", "/Driver/Experience"), ("Fame", "/Driver/Fame")],
    sections: &[Section { title: "Driver", rows: &[("Credits", "/Driver/Credits"), ("Experience", "/Driver/Experience"), ("Fame", "/Driver/Fame")] }],
    lists: &[],
    tables: &[],
};

const EVIL_WITHIN: Spec = Spec {
    name: "The Evil Within",
    tab: "Sebastian",
    accent: (150, 40, 40),
    kind: Some(crate::fmt::evil_within::KIND),
    markers: &[],
    subtitle: "inventory.zwei from Documents\\My Games\\TangoGameworks\\The Evil Within\\base\\savegame\\<slot>: green gel, trap parts, weapon upgrade levels and stat upgrades.",
    tiles: &[("Green gel", "/Resources/Green gel"), ("Trap parts", "/Resources/Trap parts")],
    sections: &[
        Section { title: "Resources", rows: &[("Green gel", "/Resources/Green gel"), ("Trap parts", "/Resources/Trap parts")] },
        Section {
            title: "Upgrades",
            rows: &[("Life gauge", "/Upgrades/Life gauge"), ("Maximum sprint", "/Upgrades/Maximum sprint"), ("Melee damage", "/Upgrades/Melee damage"), ("Syringe recovery", "/Upgrades/Syringe recovery")],
        },
    ],
    lists: &[],
    tables: &[Table {
        title: "Weapon upgrade levels",
        path: "/Weapon upgrades",
        name: &[],
        only: &[],
        cols: &[("Damage", "/Damage"), ("Fire rate", "/Fire rate"), ("Reload time", "/Reload time"), ("Clip capacity", "/Clip capacity"), ("Accuracy", "/Accuracy"), ("Critical", "/Critical"), ("Stock", "/Stock")],
        names: &[],
    }],
};

const BORDERLANDS1: Spec = Spec {
    name: "Borderlands (2009) and Borderlands Game of the Year Enhanced",
    tab: "Vault Hunter",
    accent: (230, 180, 40),
    kind: Some(crate::fmt::borderlands1::KIND),
    markers: &[],
    subtitle: "Save0001.sav and the others from Documents\\My Games\\Borderlands\\SaveData (the Enhanced edition keeps them under Borderlands GOTY Enhanced\\WillowGame\\SaveData): level, experience, skill points, money, skill levels and ammunition. Items and weapons are left as they are.",
    tiles: &[("Level", "/Character/Level"), ("Money", "/Character/Money"), ("Skill points", "/Character/Skill points")],
    sections: &[Section {
        title: "Character",
        rows: &[("Level", "/Character/Level"), ("Experience", "/Character/Experience"), ("Skill points", "/Character/Skill points"), ("Money", "/Character/Money")],
    }],
    lists: &[],
    tables: &[
        Table { title: "Skills", path: "/Skills", name: &[], only: &[], cols: &[("Level", "/Level")], names: &[] },
        Table { title: "Ammunition", path: "/Ammunition", name: &[], only: &[], cols: &[("Quantity", "/Quantity"), ("Upgrade level", "/Upgrade level")], names: &[] },
    ],
};

// ------------------------------------------------------------------ Borderlands 4
const BL4: Spec = Spec {
    name: "Borderlands 4",
    tab: "Character",
    accent: (240, 190, 60),
    kind: Some("Borderlands 4 save"),
    markers: &["/state/char_guid", "/state/currencies"],
    subtitle: "A character save (1.sav, 2.sav …) from Documents\\My Games\\Borderlands 4\\Saved\\SaveGames\\<Steam id>. Open it from its own folder so the Steam account that unlocks it can be found. Cash, Eridium, level, ammunition and skill points; items and the rest are in the Tree.",
    tiles: &[("Cash", "/state/currencies/cash"), ("Eridium", "/state/currencies/eridium"), ("Level", "/state/experience/0/level")],
    sections: &[
        Section {
            title: "Character",
            rows: &[
                ("Name", "/state/char_name"),
                ("Difficulty", "/state/player_difficulty"),
                ("Level", "/state/experience/0/level"),
                ("Experience points", "/state/experience/0/points"),
                ("Specialization level", "/state/experience/1/level"),
                ("Specialization points", "/state/experience/1/points"),
                ("Skill points to spend", "/progression/point_pools/characterprogresspoints"),
                ("Play time (seconds)", "/state/total_playtime"),
            ],
        },
        Section {
            title: "Currencies",
            rows: &[("Cash", "/state/currencies/cash"), ("Eridium", "/state/currencies/eridium")],
        },
        Section {
            title: "Ammunition",
            rows: &[
                ("Assault rifle ammo", "/state/ammo/assaultrifle"),
                ("Pistol ammo", "/state/ammo/pistol"),
                ("Shotgun ammo", "/state/ammo/shotgun"),
                ("SMG ammo", "/state/ammo/smg"),
                ("Sniper ammo", "/state/ammo/sniper"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};


// ------------------------------------------------------------------ Black Myth: Wukong
const WUKONG: Spec = Spec {
    name: "Black Myth: Wukong",
    tab: "Destined One",
    accent: (210, 160, 80),
    kind: Some("Black Myth: Wukong save"),
    markers: &["/Will", "/Sparks", "/Experience (total)"],
    subtitle: "ArchiveSaveFile.<n>.sav from b1\\Saved\\SaveGames\\<Steam id>. Will, Sparks, level, experience and play time; the file's checksum is written again when you save. Close the game first and keep the backup. Raising the level does not add Sparks or experience for you.",
    tiles: &[("Will", "/Will"), ("Sparks", "/Sparks"), ("Level", "/Level")],
    sections: &[
        Section {
            title: "Destined One",
            rows: &[
                ("Level", "/Level"),
                ("Will", "/Will"),
                ("Sparks", "/Sparks"),
                ("Experience (this level)", "/Experience (this level)"),
                ("Experience (total)", "/Experience (total)"),
                ("Play time (seconds)", "/Play time (seconds)"),
            ],
        },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ The Legend of Zelda: Breath of the Wild

const BOTW_MATERIAL_IDS: &[&str] = &["Item_Fruit_A", "Item_Fruit_B", "Item_Fruit_C", "Item_Fruit_D", "Item_Fruit_E", "Item_Fruit_E_00", "Item_Fruit_F", "Item_Fruit_G", "Item_Fruit_H", "Item_Fruit_I", "Item_Fruit_J", "Item_Fruit_K", "Item_Fruit_L", "Item_Mushroom_A", "Item_MushroomGet_A", "Item_Mushroom_B", "Item_MushroomGet_B", "Item_Mushroom_C", "Item_MushroomGet_C", "Item_Mushroom_D", "Item_MushroomGet_D", "Item_Mushroom_E", "Item_MushroomGet_E", "Item_Mushroom_F", "Item_Mushroom_F_00", "Item_MushroomGet_F", "Item_Mushroom_H", "Item_MushroomGet_H", "Item_Mushroom_J", "Item_MushroomGet_J", "Item_Mushroom_L", "Item_MushroomGet_L", "Item_Mushroom_M", "Item_MushroomGet_M", "Item_Mushroom_N", "Item_MushroomGet_N", "Item_Mushroom_N_00", "Item_Mushroom_O", "Item_MushroomGet_O", "Item_Plant_A", "Item_PlantGet_A", "Item_Plant_B", "Item_PlantGet_B", "Item_Plant_C", "Item_PlantGet_C", "Item_Plant_E", "Item_PlantGet_E", "Item_Plant_F", "Item_PlantGet_F", "Item_Plant_G", "Item_PlantGet_G", "Item_Plant_H", "Item_PlantGet_H", "Item_Plant_I", "Item_PlantGet_I", "Item_Plant_J", "Item_PlantGet_J", "Item_Plant_L", "Item_PlantGet_L", "Item_Plant_M", "Item_PlantGet_M", "Item_Plant_O", "Item_PlantGet_O", "Item_Plant_Q", "Item_PlantGet_Q", "Item_Meat_01", "Item_Meat_02", "Item_Meat_06", "Item_Meat_07", "Item_Meat_11", "Item_Meat_12", "Item_FishGet_A", "Item_FishGet_B", "Item_FishGet_C", "Item_FishGet_D", "Item_FishGet_E", "Item_FishGet_F", "Item_FishGet_G", "Item_FishGet_H", "Item_FishGet_I", "Item_FishGet_J", "Item_FishGet_K", "Item_FishGet_L", "Item_FishGet_M", "Item_FishGet_X", "Item_FishGet_Z", "Animal_Insect_A", "Item_InsectGet_A", "Animal_Insect_B", "Item_InsectGet_B", "Animal_Insect_C", "Item_InsectGet_C", "Animal_Insect_E", "Item_InsectGet_E", "Animal_Insect_F", "Item_InsectGet_F", "Animal_Insect_G", "Item_InsectGet_G", "Animal_Insect_H", "Item_InsectGet_H", "Animal_Insect_I", "Item_InsectGet_I", "Animal_Insect_K", "Item_InsectGet_K", "Animal_Insect_M", "Item_InsectGet_M", "Animal_Insect_N", "Item_InsectGet_N", "Animal_Insect_O", "Item_InsectGet_O", "Animal_Insect_P", "Item_InsectGet_P", "Animal_Insect_Q", "Item_InsectGet_Q", "Animal_Insect_R", "Item_InsectGet_R", "Animal_Insect_S", "Item_InsectGet_S", "Animal_Insect_T", "Item_InsectGet_T", "Animal_Insect_X", "Item_InsectGet_X", "Animal_Insect_Z", "Item_InsectGet_Z", "Animal_Insect_AA", "Item_InsectGet_AA", "Animal_Insect_AB", "Item_InsectGet_AB", "BeeHome", "Obj_FireWoodBundle", "Item_Enemy_00", "Item_Enemy_01", "Item_Enemy_02", "Item_Enemy_03", "Item_Enemy_04", "Item_Enemy_05", "Item_Enemy_06", "Item_Enemy_07", "Item_Enemy_08", "Item_Enemy_12", "Item_Enemy_13", "Item_Enemy_14", "Item_Enemy_15", "Item_Enemy_16", "Item_Enemy_17", "Item_Enemy_18", "Item_Enemy_19", "Item_Enemy_20", "Item_Enemy_21", "Item_Enemy_24", "Item_Enemy_25", "Item_Enemy_26", "Item_Enemy_27", "Item_Enemy_28", "Item_Enemy_29", "Item_Enemy_30", "Item_Enemy_31", "Item_Enemy_32", "Item_Enemy_33", "Item_Enemy_34", "Item_Enemy_38", "Item_Enemy_39", "Item_Enemy_40", "Item_Enemy_41", "Item_Enemy_42", "Item_Enemy_43", "Item_Enemy_44", "Item_Enemy_45", "Item_Enemy_46", "Item_Enemy_47", "Item_Enemy_48", "Item_Enemy_49", "Item_Enemy_50", "Item_Enemy_51", "Item_Enemy_52", "Item_Enemy_53", "Item_Enemy_54", "Item_Enemy_55", "Item_Enemy_56", "Item_Enemy_57", "Item_Enemy_Put_57", "Item_Material_01", "Item_Material_02", "Item_Material_03", "Item_Material_04", "Item_Material_05", "Item_Material_06", "Item_Material_07", "Item_Material_08", "Item_Ore_A", "Item_Ore_A_00", "Item_Ore_B", "Item_Ore_C", "Item_Ore_D", "Item_Ore_E", "Item_Ore_F", "Item_Ore_G", "Item_Ore_H", "Item_Ore_I", "Item_Ore_J"];

const BOTW: Spec = Spec {
    name: "The Legend of Zelda: Breath of the Wild",
    tab: "Zelda",
    accent: (214, 178, 70),
    kind: Some(crate::fmt::botw::KIND),
    markers: &["/Player/Rupees"],
    subtitle: tx("game_data.sav from the Wii U or Switch game (any version). Rupees, hearts, stamina, Korok seeds, boss counters and the amounts of materials, food and arrows."),
    tiles: &[(tx("Rupees"), "/Player/Rupees"), (tx("Korok seeds found"), "/Player/Korok seeds found")],
    sections: &[Section {
        title: tx("Player"),
        rows: &[(tx("Rupees"), "/Player/Rupees"), (tx("Monster parts (Mons)"), "/Player/Monster parts (Mons)"), (tx("Max hearts (4 per heart)"), "/Player/Max hearts (4 per heart)"), (tx("Max stamina (1000 per wheel)"), "/Player/Max stamina (1000 per wheel)"), (tx("Korok seeds found"), "/Player/Korok seeds found"), (tx("Hinox defeated"), "/Player/Hinox defeated"), (tx("Talus defeated"), "/Player/Talus defeated"), (tx("Molduga defeated"), "/Player/Molduga defeated"), (tx("Play time (seconds)"), "/Player/Play time (seconds)")],
    }],
    lists: &[
        List {
            title: tx("Materials"),
            path: "/Materials",
            shape: Shape::Records { qty_key: "Quantity" },
            names: &[("Item_Fruit_A", "Apple"), ("Item_Fruit_B", "Wildberry"), ("Item_Fruit_C", "Voltfruit"), ("Item_Fruit_D", "Hearty Durian"), ("Item_Fruit_E", "Fleet-Lotus Seeds"), ("Item_Fruit_E_00", "Fleet-Lotus Seeds x0"), ("Item_Fruit_F", "Hydromelon"), ("Item_Fruit_G", "Palm Fruit"), ("Item_Fruit_H", "Mighty Bananas"), ("Item_Fruit_I", "Spicy Pepper"), ("Item_Fruit_J", "Fortified Pumpkin"), ("Item_Fruit_K", "Acorn"), ("Item_Fruit_L", "Chickaloo Tree Nut"), ("Item_Mushroom_A", "Stamella Mushroom"), ("Item_MushroomGet_A", "Stamella Mushroom"), ("Item_Mushroom_B", "Chillshroom"), ("Item_MushroomGet_B", "Chillshroom"), ("Item_Mushroom_C", "Sunshroom"), ("Item_MushroomGet_C", "Sunshroom"), ("Item_Mushroom_D", "Rushroom"), ("Item_MushroomGet_D", "Rushroom"), ("Item_Mushroom_E", "Hylian Mushroom"), ("Item_MushroomGet_E", "Hylian Mushroom"), ("Item_Mushroom_F", "Hearty Truffle"), ("Item_Mushroom_F_00", "Hearty Truffle x0"), ("Item_MushroomGet_F", "Hearty Truffle"), ("Item_Mushroom_H", "Zapshroom"), ("Item_MushroomGet_H", "Zapshroom"), ("Item_Mushroom_J", "Silent Shroom"), ("Item_MushroomGet_J", "Silent Shroom"), ("Item_Mushroom_L", "Razorshroom"), ("Item_MushroomGet_L", "Razorshroom"), ("Item_Mushroom_M", "Ironshroom"), ("Item_MushroomGet_M", "Ironshroom"), ("Item_Mushroom_N", "Big Hearty Truffle"), ("Item_MushroomGet_N", "Big Hearty Truffle"), ("Item_Mushroom_N_00", "Big Hearty Truffle x0"), ("Item_Mushroom_O", "Endura Shroom"), ("Item_MushroomGet_O", "Endura Shroom"), ("Item_Plant_A", "Hyrule Herb"), ("Item_PlantGet_A", "Hyrule Herb"), ("Item_Plant_B", "Hearty Radish"), ("Item_PlantGet_B", "Hearty Radish"), ("Item_Plant_C", "Big Hearty Radish"), ("Item_PlantGet_C", "Big Hearty Radish"), ("Item_Plant_E", "Cool Safflina"), ("Item_PlantGet_E", "Cool Safflina"), ("Item_Plant_F", "Warm Safflina"), ("Item_PlantGet_F", "Warm Safflina"), ("Item_Plant_G", "Mighty Thistle"), ("Item_PlantGet_G", "Mighty Thistle"), ("Item_Plant_H", "Armoranth"), ("Item_PlantGet_H", "Armoranth"), ("Item_Plant_I", "Blue Nightshade"), ("Item_PlantGet_I", "Blue Nightshade"), ("Item_Plant_J", "Silent Princess"), ("Item_PlantGet_J", "Silent Princess"), ("Item_Plant_L", "Electric Safflina"), ("Item_PlantGet_L", "Electric Safflina"), ("Item_Plant_M", "Swift Carrot"), ("Item_PlantGet_M", "Swift Carrot"), ("Item_Plant_O", "Swift Violet"), ("Item_PlantGet_O", "Swift Violet"), ("Item_Plant_Q", "Endura Carrot"), ("Item_PlantGet_Q", "Endura Carrot"), ("Item_Meat_01", "Raw Meat"), ("Item_Meat_02", "Raw Prime Meat"), ("Item_Meat_06", "Raw Bird Drumstick"), ("Item_Meat_07", "Raw Bird Thigh"), ("Item_Meat_11", "Raw Gourmet Meat"), ("Item_Meat_12", "Raw Whole Meat"), ("Item_FishGet_A", "Hylian Bass"), ("Item_FishGet_B", "Hearty Bass"), ("Item_FishGet_C", "Chillfin Trout"), ("Item_FishGet_D", "Voltfin Trout"), ("Item_FishGet_E", "Mighty Carp"), ("Item_FishGet_F", "Mighty Porgy"), ("Item_FishGet_G", "Armored Porgy"), ("Item_FishGet_H", "Armored Carp"), ("Item_FishGet_I", "Hearty Salmon"), ("Item_FishGet_J", "Sizzlefin Trout"), ("Item_FishGet_K", "Hearty Blueshell Snail"), ("Item_FishGet_L", "Staminoka Bass"), ("Item_FishGet_M", "Sneaky River Snail"), ("Item_FishGet_X", "Stealthfin Trout"), ("Item_FishGet_Z", "Sanke Carp"), ("Animal_Insect_A", "Hot-Footed Frog"), ("Item_InsectGet_A", "Hot-Footed Frog"), ("Animal_Insect_B", "Tireless Frog"), ("Item_InsectGet_B", "Tireless Frog"), ("Animal_Insect_C", "Cold Darner"), ("Item_InsectGet_C", "Cold Darner"), ("Animal_Insect_E", "Sunset Firefly"), ("Item_InsectGet_E", "Sunset Firefly"), ("Animal_Insect_F", "Fairy"), ("Item_InsectGet_F", "Fairy"), ("Animal_Insect_G", "Bladed Rhino Beetle"), ("Item_InsectGet_G", "Bladed Rhino Beetle"), ("Animal_Insect_H", "Restless Cricket"), ("Item_InsectGet_H", "Restless Cricket"), ("Animal_Insect_I", "Electric Darner"), ("Item_InsectGet_I", "Electric Darner"), ("Animal_Insect_K", "Razorclaw Crab"), ("Item_InsectGet_K", "Razorclaw Crab"), ("Animal_Insect_M", "Hearty Lizard"), ("Item_InsectGet_M", "Hearty Lizard"), ("Animal_Insect_N", "Winterwing Butterfly"), ("Item_InsectGet_N", "Winterwing Butterfly"), ("Animal_Insect_O", "Ironshell Crab"), ("Item_InsectGet_O", "Ironshell Crab"), ("Animal_Insect_P", "Rugged Rhino Beetle"), ("Item_InsectGet_P", "Rugged Rhino Beetle"), ("Animal_Insect_Q", "Summerwing Butterfly"), ("Item_InsectGet_Q", "Summerwing Butterfly"), ("Animal_Insect_R", "Thunderwing Butterfly"), ("Item_InsectGet_R", "Thunderwing Butterfly"), ("Animal_Insect_S", "Hightail Lizard"), ("Item_InsectGet_S", "Hightail Lizard"), ("Animal_Insect_T", "Warm Darner"), ("Item_InsectGet_T", "Warm Darner"), ("Animal_Insect_X", "Fireproof Lizard"), ("Item_InsectGet_X", "Fireproof Lizard"), ("Animal_Insect_Z", "Bright-Eyed Crab"), ("Item_InsectGet_Z", "Bright-Eyed Crab"), ("Animal_Insect_AA", "Energetic Rhino Beetle"), ("Item_InsectGet_AA", "Energetic Rhino Beetle"), ("Animal_Insect_AB", "Smotherwing Butterfly"), ("Item_InsectGet_AB", "Smotherwing Butterfly"), ("BeeHome", "Courser Bee Honey"), ("Obj_FireWoodBundle", "Wood"), ("Item_Enemy_00", "Bokoblin Horn"), ("Item_Enemy_01", "Bokoblin Fang"), ("Item_Enemy_02", "Bokoblin Guts"), ("Item_Enemy_03", "Lizalfos Horn"), ("Item_Enemy_04", "Lizalfos Talon"), ("Item_Enemy_05", "Lizalfos Tail"), ("Item_Enemy_06", "Moblin Horn"), ("Item_Enemy_07", "Moblin Fang"), ("Item_Enemy_08", "Moblin Guts"), ("Item_Enemy_12", "Lynel Horn"), ("Item_Enemy_13", "Lynel Hoof"), ("Item_Enemy_14", "Lynel Guts"), ("Item_Enemy_15", "Red Chuchu Jelly"), ("Item_Enemy_16", "Yellow Chuchu Jelly"), ("Item_Enemy_17", "White Chuchu Jelly"), ("Item_Enemy_18", "Keese Wing"), ("Item_Enemy_19", "Keese Eyeball"), ("Item_Enemy_20", "Octorok Tentacle"), ("Item_Enemy_21", "Tentacle Eyeball"), ("Item_Enemy_24", "Molduga Fin"), ("Item_Enemy_25", "Molduga Guts"), ("Item_Enemy_26", "Ancient Gear"), ("Item_Enemy_27", "Ancient Screw"), ("Item_Enemy_28", "Ancient Spring"), ("Item_Enemy_29", "Ancient Shaft"), ("Item_Enemy_30", "Ancient Core"), ("Item_Enemy_31", "Giant Ancient Core"), ("Item_Enemy_32", "Hinox Toenail"), ("Item_Enemy_33", "Hinox Tooth"), ("Item_Enemy_34", "Hinox Guts"), ("Item_Enemy_38", "Dinraal's Scale"), ("Item_Enemy_39", "Dinraal's Claw"), ("Item_Enemy_40", "Chuchu Jelly"), ("Item_Enemy_41", "Red Lizalfos Tail"), ("Item_Enemy_42", "Icy Lizalfos Tail"), ("Item_Enemy_43", "Yellow Lizalfos Tail"), ("Item_Enemy_44", "Fire Keese Wing"), ("Item_Enemy_45", "Electric Keese Wing"), ("Item_Enemy_46", "Ice Keese Wing"), ("Item_Enemy_47", "Shard of Dinraal's Fang"), ("Item_Enemy_48", "Shard of Dinraal's Horn"), ("Item_Enemy_49", "Naydra's Scale"), ("Item_Enemy_50", "Naydra's Claw"), ("Item_Enemy_51", "Shard of Naydra's Fang"), ("Item_Enemy_52", "Shard of Naydra's Horn"), ("Item_Enemy_53", "Farosh's Scale"), ("Item_Enemy_54", "Farosh's Claw"), ("Item_Enemy_55", "Shard of Farosh's Fang"), ("Item_Enemy_56", "Shard of Farosh's Horn"), ("Item_Enemy_57", "Octo Balloon"), ("Item_Enemy_Put_57", "Octo Balloon"), ("Item_Material_01", "Cane Sugar"), ("Item_Material_02", "Goron Spice"), ("Item_Material_03", "Hylian Rice"), ("Item_Material_04", "Bird Egg"), ("Item_Material_05", "Fresh Milk"), ("Item_Material_06", "Goat Butter"), ("Item_Material_07", "Tabantha Wheat"), ("Item_Material_08", "Monster Extract"), ("Item_Ore_A", "Diamond"), ("Item_Ore_A_00", "Diamant x0"), ("Item_Ore_B", "Ruby"), ("Item_Ore_C", "Sapphire"), ("Item_Ore_D", "Topaz"), ("Item_Ore_E", "Opal"), ("Item_Ore_F", "Amber"), ("Item_Ore_G", "Luminous Stone"), ("Item_Ore_H", "Rock Salt"), ("Item_Ore_I", "Flint"), ("Item_Ore_J", "Star Fragment")],
            max: Some(Max {
                ids: BOTW_MATERIAL_IDS,
                cap: 999,
                caps: &[],
                estimate: false,
                about: tx("Raises every material you already have (fruit, mushrooms, plants, fish, insects, monster parts, ore and gems) to the game's limit of 999. Food, arrows, weapons, armour and key items are left alone."),
            }),
        },
        List {
            title: tx("Food"),
            path: "/Food",
            shape: Shape::Records { qty_key: "Quantity" },
            names: &[("Item_Boiled_01", "Hard-Boiled Egg"), ("Item_ChilledFish_01", "Frozen Bass"), ("Item_ChilledFish_02", "Frozen Hearty Salmon"), ("Item_ChilledFish_03", "Frozen Trout"), ("Item_ChilledFish_04", "Frozen Carp"), ("Item_ChilledFish_05", "Frozen Porgy"), ("Item_ChilledFish_06", "Frozen Hearty Bass"), ("Item_ChilledFish_07", "Frozen Crab"), ("Item_ChilledFish_08", "Frozen River Snail"), ("Item_ChilledFish_09", "Icy Hearty Blueshell Snail"), ("Item_RoastFish_01", "Roasted Bass"), ("Item_RoastFish_02", "Roasted Hearty Bass"), ("Item_RoastFish_03", "Roasted Trout"), ("Item_RoastFish_04", "Roasted Hearty Salmon"), ("Item_RoastFish_07", "Roasted Carp"), ("Item_RoastFish_09", "Roasted Porgy"), ("Item_RoastFish_11", "Blueshell Escargot"), ("Item_RoastFish_13", "Sneaky River Escargot"), ("Item_RoastFish_15", "Blackened Crab"), ("Item_Roast_01", "Seared Steak"), ("Item_Roast_02", "Roasted Bird Drumstick"), ("Item_Roast_03", "Baked Apple"), ("Item_Roast_04", "Toasty Stamella Shroom"), ("Item_Roast_05", "Toasted Hearty Truffle"), ("Item_Roast_06", "Toasty Hylian Shroom"), ("Item_Roast_07", "Roasted Wildberry"), ("Item_Roast_08", "Roasted Voltfruit"), ("Item_Roast_09", "Roasted Hearty Durian"), ("Item_Roast_10", "Baked Palm Fruit"), ("Item_Roast_11", "Roasted Mighty Bananas"), ("Item_Roast_12", "Roasted Hydromelon"), ("Item_Roast_13", "Charred Pepper"), ("Item_Roast_15", "Baked Fortified Pumpkin"), ("Item_Roast_16", "Roasted Lotus Seed"), ("Item_Roast_18", "Roasted Radish"), ("Item_Roast_19", "Roasted Big Radish"), ("Item_Roast_24", "Roasted Swift Carrot"), ("Item_Roast_27", "Roasted Mighty Thistle"), ("Item_Roast_28", "Roasted Armoranth"), ("Item_Roast_31", "Toasty Chillshroom"), ("Item_Roast_32", "Toasty Sunshroom"), ("Item_Roast_33", "Toasty Zapshroom"), ("Item_Roast_36", "Toasty Rushroom"), ("Item_Roast_37", "Toasty Razorshroom"), ("Item_Roast_38", "Toasty Ironshroom"), ("Item_Roast_39", "Toasty Silent Shroom"), ("Item_Roast_40", "Seared Prime Steak"), ("Item_Roast_41", "Roasted Bird Thigh"), ("Item_Roast_45", "Seared Gourmet Steak"), ("Item_Roast_46", "Roasted Whole Bird"), ("Item_Roast_48", "Roasted Acorn"), ("Item_Roast_49", "Toasted Big Heart Truffle"), ("Item_Roast_50", "Roasted Endura Carrot"), ("Item_Roast_51", "Campfire Egg"), ("Item_Roast_52", "Roasted Tree Nut"), ("Item_Roast_53", "Toasty Endura Shroom"), ("Item_Chilled_01", "Icy Meat"), ("Item_Chilled_02", "Icy Prime Meat"), ("Item_Chilled_03", "Icy Gourmet Meat"), ("Item_Chilled_04", "Frozen Bird Drumstick"), ("Item_Chilled_05", "Frozen Bird Thigh"), ("Item_Chilled_06", "Frozen Whole Bird"), ("Item_Cook_A_01", "Mushroom Skewer"), ("Item_Cook_A_02", "Steamed Mushrooms"), ("Item_Cook_A_03", "Steamed Fruit"), ("Item_Cook_A_04", "Steamed Fish"), ("Item_Cook_A_05", "Steamed Meat"), ("Item_Cook_A_07", "Fruit and Mushroom Mix"), ("Item_Cook_A_08", "Fish and Mushroom Skewer"), ("Item_Cook_A_09", "Meat and Mushroom Skewer"), ("Item_Cook_A_10", "Omelet"), ("Item_Cook_A_11", "Glazed Mushroom"), ("Item_Cook_A_12", "Glazed Meat"), ("Item_Cook_A_13", "Glazed Seafood"), ("Item_Cook_A_14", "Glazed Veggies"), ("Item_Cook_B_01", "Fried Wild Greens"), ("Item_Cook_B_02", "Simmered Fruits"), ("Item_Cook_B_05", "Fish Skewer"), ("Item_Cook_B_06", "Meat Skewer"), ("Item_Cook_B_11", "Copious Fried Wild Greens"), ("Item_Cook_B_12", "Copious Simmered Fruits"), ("Item_Cook_B_13", "Copious Mushroom Skewers"), ("Item_Cook_B_15", "Copious Seafood Skewers"), ("Item_Cook_B_16", "Copious Meat Skewers"), ("Item_Cook_B_17", "Meat and Seafood Fry"), ("Item_Cook_B_18", "Prime Meat and Seafood Fry"), ("Item_Cook_B_19", "Gourmet Meat and Seafood Fry"), ("Item_Cook_B_20", "Meat-Stuffed Pumpkin"), ("Item_Cook_B_21", "Sautéed Peppers"), ("Item_Cook_B_22", "Sautéed Nuts"), ("Item_Cook_B_23", "Seafood Skewers"), ("Item_Cook_C_16", "Fairy Tonic"), ("Item_Cook_C_17", "Elixir"), ("Item_Cook_D_01", "Salt-Grilled Mushrooms"), ("Item_Cook_D_02", "Salt-Grilled Greens"), ("Item_Cook_D_03", "Salt-Grilled Fish"), ("Item_Cook_D_04", "Salt-Grilled Meat"), ("Item_Cook_D_05", "Salt-Grilled Prime Meat"), ("Item_Cook_D_06", "Salt-Grilled Gourmet Meat"), ("Item_Cook_D_07", "Pepper Steak"), ("Item_Cook_D_08", "Pepper Seafood"), ("Item_Cook_D_09", "Salt-Grilled Crab"), ("Item_Cook_D_10", "Crab Stir-Fry"), ("Item_Cook_E_01", "Poultry Pilaf"), ("Item_Cook_E_02", "Prime Poultry Pilaf"), ("Item_Cook_E_03", "Gourmet Poultry Pilaf"), ("Item_Cook_E_04", "Fried Egg and Rice"), ("Item_Cook_F_01", "Creamy Meat Soup"), ("Item_Cook_F_02", "Creamy Seafood Soup"), ("Item_Cook_F_03", "Veggie Cream Soup"), ("Item_Cook_F_04", "Creamy Heart Soup"), ("Item_Cook_G_02", "Seafood Rice Balls"), ("Item_Cook_G_03", "Veggie Rice Balls"), ("Item_Cook_G_04", "Mushroom Rice Balls"), ("Item_Cook_G_05", "Meat and Rice Bowl"), ("Item_Cook_G_06", "Prime Meat and Rice Bowl"), ("Item_Cook_G_09", "Gourmet Meat and Rice Bowl"), ("Item_Cook_G_10", "Seafood Fried Rice"), ("Item_Cook_G_11", "Curry Pilaf"), ("Item_Cook_G_12", "Mushroom Risotto"), ("Item_Cook_G_13", "Vegetable Risotto"), ("Item_Cook_G_14", "Salmon Risotto"), ("Item_Cook_G_15", "Meaty Rice Balls"), ("Item_Cook_G_16", "Crab Omelet with Rice"), ("Item_Cook_G_17", "Crab Risotto"), ("Item_Cook_H_01", "Seafood Meunière"), ("Item_Cook_H_02", "Porgy Meunière"), ("Item_Cook_H_03", "Salmon Meunière"), ("Item_Cook_I_01", "Fruit Pie"), ("Item_Cook_I_02", "Apple Pie"), ("Item_Cook_I_03", "Egg Tart"), ("Item_Cook_I_04", "Meat Pie"), ("Item_Cook_I_05", "Carrot Cake"), ("Item_Cook_I_06", "Pumpkin Pie"), ("Item_Cook_I_07", "Hot Buttered Apple"), ("Item_Cook_I_08", "Honeyed Apple"), ("Item_Cook_I_09", "Honeyed Fruits"), ("Item_Cook_I_10", "Plain Crepe"), ("Item_Cook_I_11", "Wildberry Crepe"), ("Item_Cook_I_12", "Nutcake"), ("Item_Cook_I_13", "Fried Bananas"), ("Item_Cook_I_14", "Egg Pudding"), ("Item_Cook_I_15", "Fish Pie"), ("Item_Cook_I_16", "Honey Candy"), ("Item_Cook_I_17", "Honey Crepe"), ("Item_Cook_J_01", "Curry Rice"), ("Item_Cook_J_02", "Vegetable Curry"), ("Item_Cook_J_03", "Seafood Curry"), ("Item_Cook_J_04", "Poultry Curry"), ("Item_Cook_J_05", "Prime Poultry Curry"), ("Item_Cook_J_06", "Meat Curry"), ("Item_Cook_J_07", "Prime Meat Curry"), ("Item_Cook_J_08", "Gourmet Poultry Curry"), ("Item_Cook_J_09", "Gourmet Meat Curry"), ("Item_Cook_K_01", "Meat Stew"), ("Item_Cook_K_02", "Prime Meat Stew"), ("Item_Cook_K_03", "Pumpkin Stew"), ("Item_Cook_K_04", "Clam Chowder"), ("Item_Cook_K_05", "Gourmet Meat Stew"), ("Item_Cook_K_06", "Cream of Mushroom Soup"), ("Item_Cook_K_07", "Cream of Vegetable Soup"), ("Item_Cook_K_08", "Carrot Stew"), ("Item_Cook_K_09", "Milk"), ("Item_Material_05_00", "Milk x0"), ("Item_Cook_L_01", "Monster Stew"), ("Item_Cook_L_02", "Monster Soup"), ("Item_Cook_L_03", "Monster Cake"), ("Item_Cook_L_04", "Monster Rice Balls"), ("Item_Cook_L_05", "Monster Curry"), ("Item_Cook_M_01", "Wheat Bread"), ("Item_Cook_N_01", "Seafood Paella"), ("Item_Cook_N_02", "Fruitcake"), ("Item_Cook_N_03", "Vegetable Omelet"), ("Item_Cook_N_04", "Mushroom Omelet"), ("Item_Cook_O_01", "Dubious Food"), ("Item_Cook_O_02", "Rock-Hard Food"), ("Item_Cook_P_01", "Fragrant Mushroom Sauté"), ("Item_Cook_P_02", "Herb Sauté"), ("Item_Cook_P_03", "Spiced Meat Skewer"), ("Item_Cook_P_04", "Prime Spiced Meat Skewer"), ("Item_Cook_P_05", "Gourmet Spiced Meat Skewer")],
            max: None,
        },
        List {
            title: tx("Arrows"),
            path: "/Arrows",
            shape: Shape::Records { qty_key: "Quantity" },
            names: &[("NormalArrow", "Arrow"), ("FireArrow", "Fire Arrow"), ("IceArrow", "Ice Arrow"), ("ElectricArrow", "Shock Arrow"), ("BombArrow_A", "Bomb Arrow"), ("AncientArrow", "Ancient Arrow")],
            max: None,
        },
        List {
            title: tx("Other"),
            path: "/Other",
            shape: Shape::Records { qty_key: "Quantity" },
            names: &[("Obj_DungeonClearSeal", "Spirit Orb"), ("Obj_KorokNuts", "Korok Seed")],
            max: None,
        },
    ],
    tables: &[],
};

const SOPHIE: Spec = Spec {
    name: "Atelier Sophie: The Alchemist of the Mysterious Book",
    tab: "Resources",
    accent: (200, 120, 150),
    kind: Some(crate::fmt::gust::SOPHIE),
    markers: &["/Resources/Cole"],
    subtitle: tx("GAMEDATA00 to GAMEDATA30 from Documents\\KoeiTecmo\\A17\\SAVEDATA (the original 2017 release). Cole and Tess exchange tickets; item qualities and recipes are left alone."),
    tiles: &[(tx("Cole"), "/Resources/Cole"), (tx("Tess exchange tickets"), "/Resources/Tess exchange tickets")],
    sections: &[Section { title: tx("Resources"), rows: &[(tx("Cole"), "/Resources/Cole"), (tx("Tess exchange tickets"), "/Resources/Tess exchange tickets")] }],
    lists: &[],
    tables: &[],
};

const FIRIS: Spec = Spec {
    name: "Atelier Firis: The Alchemist and the Mysterious Journey",
    tab: "Resources",
    accent: (90, 150, 200),
    kind: Some(crate::fmt::gust::FIRIS),
    markers: &["/Resources/Cole"],
    subtitle: tx("GAMEDATA slot files from Documents\\KoeiTecmo\\A18\\SAVEDATA (the original 2017 release). Cole; item qualities and recipes are left alone."),
    tiles: &[(tx("Cole"), "/Resources/Cole")],
    sections: &[Section { title: tx("Resources"), rows: &[(tx("Cole"), "/Resources/Cole")] }],
    lists: &[],
    tables: &[],
};
