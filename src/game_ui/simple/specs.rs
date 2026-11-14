//! One `Spec` per game. Names and ids come from public editors (credited in
//! THIRD_PARTY_NOTICES.md); values are only listed once they read correctly on a real save.

use super::{List, Max, Section, Shape, Spec, Table};

pub static SPECS: &[Spec] = &[COTL, FALLOUT_SHELTER, DRG, S1_MONEY, S1_RANK, S1_TIME, S1_GAME, SOTF_STATE, SOTF_GAME, SOTF_WEATHER, SOTF_ITEMS, DMD, CUPHEAD, ANTIMATTER, PVZ, CELESTE, DRGS, KH3, RF4S, SKY1ST, KSP, FFPR, MCD, STONESHARD, ROGUE_TRADER, BIOSHOCK_INFINITE, SPACE_HAVEN, SNOWRUNNER, GRAVEYARD_KEEPER, NFSMW, TOS, GTA3, GTAVC, BITBURNER, TERRA_INVICTA, PRISON, FF13, SONIC_MANIA, CAVE_STORY_PLUS, CAVE_STORY, LIES_OF_P, TXR, DAVE, MSC, ULTRAKILL, RPGXP, RE5, ANNO1800, FTK, LEGO_TSS, LEGO_CLASSIC, MYZ, CMS21, MEDIEVAL_DYNASTY, MECHANICUS, ORI, OMORI, ENDER_LILIES, SW2, ES2, MOONLIGHTER, WASTELAND2, ATOM, MORTAL_SHELL, MORTAL_SHELL_2, COTW, BEAMNG, FOREST, YAKUZA3, YAKUZA4, YAKUZA5, ISHIN, SALT, DRAKENSANG, ALPHA, TYRANNY, JEDI_FO, SR3, SR4, WD2, FABLE, WANDERING_SWORD, STELLAR_BLADE, PRAGMATA, SLIME_RANCHER, SUBNAUTICA2, HZD, SLEEPING_DOGS, DXMD, ATOMIC_HEART, DISHONORED2, DEATHLOOP, DISHONORED1, RDR1, RDR2, SM64, SUPER_METROID, LTTP, YSF, BD2, SKY3, CS1];

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

// ------------------------------------------------------------------ Super Mario 64

const SM64: Spec = Spec {
    name: "Super Mario 64",
    tab: "Mario",
    accent: (210, 50, 40),
    kind: Some(crate::fmt::sm64::KIND),
    markers: &["/Info/Files in use"],
    subtitle: "An emulator's .eep save (512 or 2048 bytes). Per file: caps, keys, switches, secret stars, each course's six stars, 100-coin star, cannon and coin record.",
    tiles: &[("Files in use", "/Info/Files in use")],
    sections: &[
        Section {
        title: "File A",
        rows: &[
            ("Stars collected", "/File A/Stars collected"),
            ("Wing Cap switch", "/File A/Wing Cap switch"),
            ("Metal Cap switch", "/File A/Metal Cap switch"),
            ("Vanish Cap switch", "/File A/Vanish Cap switch"),
            ("Basement key", "/File A/Basement key"),
            ("Gallery key", "/File A/Gallery key"),
            ("Basement key door open", "/File A/Basement key door open"),
            ("Gallery key door open", "/File A/Gallery key door open"),
            ("Moat drained", "/File A/Moat drained"),
            ("Toad (Hazy Maze Cave)", "/File A/Toad (Hazy Maze Cave)"),
            ("Toad (Gallery)", "/File A/Toad (Gallery)"),
            ("Toad (Clock Room)", "/File A/Toad (Clock Room)"),
            ("Mips (first star)", "/File A/Mips (first star)"),
            ("Mips (second star)", "/File A/Mips (second star)"),
            ("Secret slide star", "/File A/Secret slide star"),
            ("Secret slide star (under 21 s)", "/File A/Secret slide star (under 21 s)"),
            ("Bowser in the Dark World (red coins)", "/File A/Bowser in the Dark World (red coins)"),
            ("Bowser in the Fire Sea (red coins)", "/File A/Bowser in the Fire Sea (red coins)"),
            ("Bowser in the Sky (red coins)", "/File A/Bowser in the Sky (red coins)"),
            ("Cavern of the Metal Cap", "/File A/Cavern of the Metal Cap"),
            ("Tower of the Wing Cap", "/File A/Tower of the Wing Cap"),
            ("Vanish Cap Under the Moat", "/File A/Vanish Cap Under the Moat"),
            ("Wing Mario Over the Rainbow", "/File A/Wing Mario Over the Rainbow"),
            ("The Secret Aquarium", "/File A/The Secret Aquarium"),
        ],
    },
        Section {
        title: "File B",
        rows: &[
            ("Stars collected", "/File B/Stars collected"),
            ("Wing Cap switch", "/File B/Wing Cap switch"),
            ("Metal Cap switch", "/File B/Metal Cap switch"),
            ("Vanish Cap switch", "/File B/Vanish Cap switch"),
            ("Basement key", "/File B/Basement key"),
            ("Gallery key", "/File B/Gallery key"),
            ("Basement key door open", "/File B/Basement key door open"),
            ("Gallery key door open", "/File B/Gallery key door open"),
            ("Moat drained", "/File B/Moat drained"),
            ("Toad (Hazy Maze Cave)", "/File B/Toad (Hazy Maze Cave)"),
            ("Toad (Gallery)", "/File B/Toad (Gallery)"),
            ("Toad (Clock Room)", "/File B/Toad (Clock Room)"),
            ("Mips (first star)", "/File B/Mips (first star)"),
            ("Mips (second star)", "/File B/Mips (second star)"),
            ("Secret slide star", "/File B/Secret slide star"),
            ("Secret slide star (under 21 s)", "/File B/Secret slide star (under 21 s)"),
            ("Bowser in the Dark World (red coins)", "/File B/Bowser in the Dark World (red coins)"),
            ("Bowser in the Fire Sea (red coins)", "/File B/Bowser in the Fire Sea (red coins)"),
            ("Bowser in the Sky (red coins)", "/File B/Bowser in the Sky (red coins)"),
            ("Cavern of the Metal Cap", "/File B/Cavern of the Metal Cap"),
            ("Tower of the Wing Cap", "/File B/Tower of the Wing Cap"),
            ("Vanish Cap Under the Moat", "/File B/Vanish Cap Under the Moat"),
            ("Wing Mario Over the Rainbow", "/File B/Wing Mario Over the Rainbow"),
            ("The Secret Aquarium", "/File B/The Secret Aquarium"),
        ],
    },
        Section {
        title: "File C",
        rows: &[
            ("Stars collected", "/File C/Stars collected"),
            ("Wing Cap switch", "/File C/Wing Cap switch"),
            ("Metal Cap switch", "/File C/Metal Cap switch"),
            ("Vanish Cap switch", "/File C/Vanish Cap switch"),
            ("Basement key", "/File C/Basement key"),
            ("Gallery key", "/File C/Gallery key"),
            ("Basement key door open", "/File C/Basement key door open"),
            ("Gallery key door open", "/File C/Gallery key door open"),
            ("Moat drained", "/File C/Moat drained"),
            ("Toad (Hazy Maze Cave)", "/File C/Toad (Hazy Maze Cave)"),
            ("Toad (Gallery)", "/File C/Toad (Gallery)"),
            ("Toad (Clock Room)", "/File C/Toad (Clock Room)"),
            ("Mips (first star)", "/File C/Mips (first star)"),
            ("Mips (second star)", "/File C/Mips (second star)"),
            ("Secret slide star", "/File C/Secret slide star"),
            ("Secret slide star (under 21 s)", "/File C/Secret slide star (under 21 s)"),
            ("Bowser in the Dark World (red coins)", "/File C/Bowser in the Dark World (red coins)"),
            ("Bowser in the Fire Sea (red coins)", "/File C/Bowser in the Fire Sea (red coins)"),
            ("Bowser in the Sky (red coins)", "/File C/Bowser in the Sky (red coins)"),
            ("Cavern of the Metal Cap", "/File C/Cavern of the Metal Cap"),
            ("Tower of the Wing Cap", "/File C/Tower of the Wing Cap"),
            ("Vanish Cap Under the Moat", "/File C/Vanish Cap Under the Moat"),
            ("Wing Mario Over the Rainbow", "/File C/Wing Mario Over the Rainbow"),
            ("The Secret Aquarium", "/File C/The Secret Aquarium"),
        ],
    },
        Section {
        title: "File D",
        rows: &[
            ("Stars collected", "/File D/Stars collected"),
            ("Wing Cap switch", "/File D/Wing Cap switch"),
            ("Metal Cap switch", "/File D/Metal Cap switch"),
            ("Vanish Cap switch", "/File D/Vanish Cap switch"),
            ("Basement key", "/File D/Basement key"),
            ("Gallery key", "/File D/Gallery key"),
            ("Basement key door open", "/File D/Basement key door open"),
            ("Gallery key door open", "/File D/Gallery key door open"),
            ("Moat drained", "/File D/Moat drained"),
            ("Toad (Hazy Maze Cave)", "/File D/Toad (Hazy Maze Cave)"),
            ("Toad (Gallery)", "/File D/Toad (Gallery)"),
            ("Toad (Clock Room)", "/File D/Toad (Clock Room)"),
            ("Mips (first star)", "/File D/Mips (first star)"),
            ("Mips (second star)", "/File D/Mips (second star)"),
            ("Secret slide star", "/File D/Secret slide star"),
            ("Secret slide star (under 21 s)", "/File D/Secret slide star (under 21 s)"),
            ("Bowser in the Dark World (red coins)", "/File D/Bowser in the Dark World (red coins)"),
            ("Bowser in the Fire Sea (red coins)", "/File D/Bowser in the Fire Sea (red coins)"),
            ("Bowser in the Sky (red coins)", "/File D/Bowser in the Sky (red coins)"),
            ("Cavern of the Metal Cap", "/File D/Cavern of the Metal Cap"),
            ("Tower of the Wing Cap", "/File D/Tower of the Wing Cap"),
            ("Vanish Cap Under the Moat", "/File D/Vanish Cap Under the Moat"),
            ("Wing Mario Over the Rainbow", "/File D/Wing Mario Over the Rainbow"),
            ("The Secret Aquarium", "/File D/The Secret Aquarium"),
        ],
    },
    ],
    lists: &[],
    tables: &[
        Table {
        title: "File A courses",
        path: "/File A/Courses",
        name: &[],
        cols: &[("Star 1", "/Star 1"), ("Star 2", "/Star 2"), ("Star 3", "/Star 3"), ("Star 4", "/Star 4"), ("Star 5", "/Star 5"), ("Star 6", "/Star 6"), ("100 coins", "/100 coins"), ("Cannon open", "/Cannon open"), ("Coin record", "/Coin record")],
        only: &[],
        names: &[],
    },
        Table {
        title: "File B courses",
        path: "/File B/Courses",
        name: &[],
        cols: &[("Star 1", "/Star 1"), ("Star 2", "/Star 2"), ("Star 3", "/Star 3"), ("Star 4", "/Star 4"), ("Star 5", "/Star 5"), ("Star 6", "/Star 6"), ("100 coins", "/100 coins"), ("Cannon open", "/Cannon open"), ("Coin record", "/Coin record")],
        only: &[],
        names: &[],
    },
        Table {
        title: "File C courses",
        path: "/File C/Courses",
        name: &[],
        cols: &[("Star 1", "/Star 1"), ("Star 2", "/Star 2"), ("Star 3", "/Star 3"), ("Star 4", "/Star 4"), ("Star 5", "/Star 5"), ("Star 6", "/Star 6"), ("100 coins", "/100 coins"), ("Cannon open", "/Cannon open"), ("Coin record", "/Coin record")],
        only: &[],
        names: &[],
    },
        Table {
        title: "File D courses",
        path: "/File D/Courses",
        name: &[],
        cols: &[("Star 1", "/Star 1"), ("Star 2", "/Star 2"), ("Star 3", "/Star 3"), ("Star 4", "/Star 4"), ("Star 5", "/Star 5"), ("Star 6", "/Star 6"), ("100 coins", "/100 coins"), ("Cannon open", "/Cannon open"), ("Coin record", "/Coin record")],
        only: &[],
        names: &[],
    },
    ],
};

// ------------------------------------------------------------------ Super Metroid

const SUPER_METROID: Spec = Spec {
    name: "Super Metroid",
    tab: "Samus",
    accent: (200, 120, 30),
    kind: Some(crate::fmt::supermetroid::KIND),
    markers: &["/Info/Slots in use"],
    subtitle: "An emulator's 8 KiB .srm / .sav save. Per Samus slot: energy and reserve tanks, missiles, super missiles, power bombs, play time and which beams, suits and boots she has and has equipped.",
    tiles: &[("Slots in use", "/Info/Slots in use")],
    sections: &[
    Section { title: "Samus A", rows: &[
        ("Energy", "/Samus A/Energy"),
        ("Energy tanks (max energy)", "/Samus A/Energy tanks (max energy)"),
        ("Reserve energy", "/Samus A/Reserve energy"),
        ("Reserve tanks (max reserve)", "/Samus A/Reserve tanks (max reserve)"),
        ("Missiles", "/Samus A/Missiles"),
        ("Missiles (max)", "/Samus A/Missiles (max)"),
        ("Super missiles", "/Samus A/Super missiles"),
        ("Super missiles (max)", "/Samus A/Super missiles (max)"),
        ("Power bombs", "/Samus A/Power bombs"),
        ("Power bombs (max)", "/Samus A/Power bombs (max)"),
        ("Play time hours", "/Samus A/Play time hours"),
        ("Play time minutes", "/Samus A/Play time minutes"),
        ("Play time seconds", "/Samus A/Play time seconds"),
    ] },
    Section { title: "Samus A: obtained", rows: &[
        ("Charge beam", "/Samus A/Obtained/Charge beam"),
        ("Ice beam", "/Samus A/Obtained/Ice beam"),
        ("Wave beam", "/Samus A/Obtained/Wave beam"),
        ("Spazer", "/Samus A/Obtained/Spazer"),
        ("Plasma beam", "/Samus A/Obtained/Plasma beam"),
        ("Varia suit", "/Samus A/Obtained/Varia suit"),
        ("Gravity suit", "/Samus A/Obtained/Gravity suit"),
        ("Morphing ball", "/Samus A/Obtained/Morphing ball"),
        ("Bombs", "/Samus A/Obtained/Bombs"),
        ("Spring ball", "/Samus A/Obtained/Spring ball"),
        ("Screw attack", "/Samus A/Obtained/Screw attack"),
        ("Grappling beam", "/Samus A/Obtained/Grappling beam"),
        ("X-ray scope", "/Samus A/Obtained/X-ray scope"),
        ("Hi-jump boots", "/Samus A/Obtained/Hi-jump boots"),
        ("Space jump", "/Samus A/Obtained/Space jump"),
        ("Speed booster", "/Samus A/Obtained/Speed booster"),
    ] },
    Section { title: "Samus A: equipped", rows: &[
        ("Charge beam", "/Samus A/Equipped/Charge beam"),
        ("Ice beam", "/Samus A/Equipped/Ice beam"),
        ("Wave beam", "/Samus A/Equipped/Wave beam"),
        ("Spazer", "/Samus A/Equipped/Spazer"),
        ("Plasma beam", "/Samus A/Equipped/Plasma beam"),
        ("Varia suit", "/Samus A/Equipped/Varia suit"),
        ("Gravity suit", "/Samus A/Equipped/Gravity suit"),
        ("Morphing ball", "/Samus A/Equipped/Morphing ball"),
        ("Bombs", "/Samus A/Equipped/Bombs"),
        ("Spring ball", "/Samus A/Equipped/Spring ball"),
        ("Screw attack", "/Samus A/Equipped/Screw attack"),
        ("Grappling beam", "/Samus A/Equipped/Grappling beam"),
        ("X-ray scope", "/Samus A/Equipped/X-ray scope"),
        ("Hi-jump boots", "/Samus A/Equipped/Hi-jump boots"),
        ("Space jump", "/Samus A/Equipped/Space jump"),
        ("Speed booster", "/Samus A/Equipped/Speed booster"),
    ] },
    Section { title: "Samus B", rows: &[
        ("Energy", "/Samus B/Energy"),
        ("Energy tanks (max energy)", "/Samus B/Energy tanks (max energy)"),
        ("Reserve energy", "/Samus B/Reserve energy"),
        ("Reserve tanks (max reserve)", "/Samus B/Reserve tanks (max reserve)"),
        ("Missiles", "/Samus B/Missiles"),
        ("Missiles (max)", "/Samus B/Missiles (max)"),
        ("Super missiles", "/Samus B/Super missiles"),
        ("Super missiles (max)", "/Samus B/Super missiles (max)"),
        ("Power bombs", "/Samus B/Power bombs"),
        ("Power bombs (max)", "/Samus B/Power bombs (max)"),
        ("Play time hours", "/Samus B/Play time hours"),
        ("Play time minutes", "/Samus B/Play time minutes"),
        ("Play time seconds", "/Samus B/Play time seconds"),
    ] },
    Section { title: "Samus B: obtained", rows: &[
        ("Charge beam", "/Samus B/Obtained/Charge beam"),
        ("Ice beam", "/Samus B/Obtained/Ice beam"),
        ("Wave beam", "/Samus B/Obtained/Wave beam"),
        ("Spazer", "/Samus B/Obtained/Spazer"),
        ("Plasma beam", "/Samus B/Obtained/Plasma beam"),
        ("Varia suit", "/Samus B/Obtained/Varia suit"),
        ("Gravity suit", "/Samus B/Obtained/Gravity suit"),
        ("Morphing ball", "/Samus B/Obtained/Morphing ball"),
        ("Bombs", "/Samus B/Obtained/Bombs"),
        ("Spring ball", "/Samus B/Obtained/Spring ball"),
        ("Screw attack", "/Samus B/Obtained/Screw attack"),
        ("Grappling beam", "/Samus B/Obtained/Grappling beam"),
        ("X-ray scope", "/Samus B/Obtained/X-ray scope"),
        ("Hi-jump boots", "/Samus B/Obtained/Hi-jump boots"),
        ("Space jump", "/Samus B/Obtained/Space jump"),
        ("Speed booster", "/Samus B/Obtained/Speed booster"),
    ] },
    Section { title: "Samus B: equipped", rows: &[
        ("Charge beam", "/Samus B/Equipped/Charge beam"),
        ("Ice beam", "/Samus B/Equipped/Ice beam"),
        ("Wave beam", "/Samus B/Equipped/Wave beam"),
        ("Spazer", "/Samus B/Equipped/Spazer"),
        ("Plasma beam", "/Samus B/Equipped/Plasma beam"),
        ("Varia suit", "/Samus B/Equipped/Varia suit"),
        ("Gravity suit", "/Samus B/Equipped/Gravity suit"),
        ("Morphing ball", "/Samus B/Equipped/Morphing ball"),
        ("Bombs", "/Samus B/Equipped/Bombs"),
        ("Spring ball", "/Samus B/Equipped/Spring ball"),
        ("Screw attack", "/Samus B/Equipped/Screw attack"),
        ("Grappling beam", "/Samus B/Equipped/Grappling beam"),
        ("X-ray scope", "/Samus B/Equipped/X-ray scope"),
        ("Hi-jump boots", "/Samus B/Equipped/Hi-jump boots"),
        ("Space jump", "/Samus B/Equipped/Space jump"),
        ("Speed booster", "/Samus B/Equipped/Speed booster"),
    ] },
    Section { title: "Samus C", rows: &[
        ("Energy", "/Samus C/Energy"),
        ("Energy tanks (max energy)", "/Samus C/Energy tanks (max energy)"),
        ("Reserve energy", "/Samus C/Reserve energy"),
        ("Reserve tanks (max reserve)", "/Samus C/Reserve tanks (max reserve)"),
        ("Missiles", "/Samus C/Missiles"),
        ("Missiles (max)", "/Samus C/Missiles (max)"),
        ("Super missiles", "/Samus C/Super missiles"),
        ("Super missiles (max)", "/Samus C/Super missiles (max)"),
        ("Power bombs", "/Samus C/Power bombs"),
        ("Power bombs (max)", "/Samus C/Power bombs (max)"),
        ("Play time hours", "/Samus C/Play time hours"),
        ("Play time minutes", "/Samus C/Play time minutes"),
        ("Play time seconds", "/Samus C/Play time seconds"),
    ] },
    Section { title: "Samus C: obtained", rows: &[
        ("Charge beam", "/Samus C/Obtained/Charge beam"),
        ("Ice beam", "/Samus C/Obtained/Ice beam"),
        ("Wave beam", "/Samus C/Obtained/Wave beam"),
        ("Spazer", "/Samus C/Obtained/Spazer"),
        ("Plasma beam", "/Samus C/Obtained/Plasma beam"),
        ("Varia suit", "/Samus C/Obtained/Varia suit"),
        ("Gravity suit", "/Samus C/Obtained/Gravity suit"),
        ("Morphing ball", "/Samus C/Obtained/Morphing ball"),
        ("Bombs", "/Samus C/Obtained/Bombs"),
        ("Spring ball", "/Samus C/Obtained/Spring ball"),
        ("Screw attack", "/Samus C/Obtained/Screw attack"),
        ("Grappling beam", "/Samus C/Obtained/Grappling beam"),
        ("X-ray scope", "/Samus C/Obtained/X-ray scope"),
        ("Hi-jump boots", "/Samus C/Obtained/Hi-jump boots"),
        ("Space jump", "/Samus C/Obtained/Space jump"),
        ("Speed booster", "/Samus C/Obtained/Speed booster"),
    ] },
    Section { title: "Samus C: equipped", rows: &[
        ("Charge beam", "/Samus C/Equipped/Charge beam"),
        ("Ice beam", "/Samus C/Equipped/Ice beam"),
        ("Wave beam", "/Samus C/Equipped/Wave beam"),
        ("Spazer", "/Samus C/Equipped/Spazer"),
        ("Plasma beam", "/Samus C/Equipped/Plasma beam"),
        ("Varia suit", "/Samus C/Equipped/Varia suit"),
        ("Gravity suit", "/Samus C/Equipped/Gravity suit"),
        ("Morphing ball", "/Samus C/Equipped/Morphing ball"),
        ("Bombs", "/Samus C/Equipped/Bombs"),
        ("Spring ball", "/Samus C/Equipped/Spring ball"),
        ("Screw attack", "/Samus C/Equipped/Screw attack"),
        ("Grappling beam", "/Samus C/Equipped/Grappling beam"),
        ("X-ray scope", "/Samus C/Equipped/X-ray scope"),
        ("Hi-jump boots", "/Samus C/Equipped/Hi-jump boots"),
        ("Space jump", "/Samus C/Equipped/Space jump"),
        ("Speed booster", "/Samus C/Equipped/Speed booster"),
    ] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ A Link to the Past

const LTTP: Spec = Spec {
    name: "The Legend of Zelda: A Link to the Past",
    tab: "Link",
    accent: (60, 150, 70),
    kind: Some(crate::fmt::zelda_lttp::KIND),
    markers: &["/Info/Slots in use"],
    subtitle: "An emulator's 8 KiB .srm / .sav save. Per slot: hearts, magic, rupees, bombs and arrows with their upgrades, the sword, shield, tunic and every item, and the four bottles.",
    tiles: &[("Slots in use", "/Info/Slots in use")],
    sections: &[
    Section { title: "Slot 1", rows: &[
        ("Max health (8 per heart)", "/Slot 1/Max health (8 per heart)"),
        ("Health (8 per heart)", "/Slot 1/Health (8 per heart)"),
        ("Heart pieces", "/Slot 1/Heart pieces"),
        ("Magic", "/Slot 1/Magic"),
        ("Rupees", "/Slot 1/Rupees"),
        ("Bombs", "/Slot 1/Bombs"),
        ("Bomb capacity upgrades (0-7)", "/Slot 1/Bomb capacity upgrades (0-7)"),
        ("Arrows", "/Slot 1/Arrows"),
        ("Arrow capacity upgrades (0-7)", "/Slot 1/Arrow capacity upgrades (0-7)"),
        ("Rupees in the fairy pond", "/Slot 1/Rupees in the fairy pond"),
    ] },
    Section { title: "Slot 1: items", rows: &[
        ("Sword", "/Slot 1/Items/Sword"),
        ("Shield", "/Slot 1/Items/Shield"),
        ("Tunic", "/Slot 1/Items/Tunic"),
        ("Bow", "/Slot 1/Items/Bow"),
        ("Boomerang", "/Slot 1/Items/Boomerang"),
        ("Mushroom / magic powder", "/Slot 1/Items/Mushroom / magic powder"),
        ("Shovel / flute", "/Slot 1/Items/Shovel / flute"),
        ("Gloves", "/Slot 1/Items/Gloves"),
        ("Magic mirror", "/Slot 1/Items/Magic mirror"),
        ("Hookshot", "/Slot 1/Items/Hookshot"),
        ("Fire rod", "/Slot 1/Items/Fire rod"),
        ("Ice rod", "/Slot 1/Items/Ice rod"),
        ("Bombos medallion", "/Slot 1/Items/Bombos medallion"),
        ("Ether medallion", "/Slot 1/Items/Ether medallion"),
        ("Quake medallion", "/Slot 1/Items/Quake medallion"),
        ("Lamp", "/Slot 1/Items/Lamp"),
        ("Magic hammer", "/Slot 1/Items/Magic hammer"),
        ("Bug-catching net", "/Slot 1/Items/Bug-catching net"),
        ("Book of Mudora", "/Slot 1/Items/Book of Mudora"),
        ("Cane of Somaria", "/Slot 1/Items/Cane of Somaria"),
        ("Cane of Byrna", "/Slot 1/Items/Cane of Byrna"),
        ("Magic cape", "/Slot 1/Items/Magic cape"),
        ("Pegasus boots", "/Slot 1/Items/Pegasus boots"),
        ("Zora's flippers", "/Slot 1/Items/Zora's flippers"),
        ("Moon pearl", "/Slot 1/Items/Moon pearl"),
        ("Bottle 1", "/Slot 1/Items/Bottle 1"),
        ("Bottle 2", "/Slot 1/Items/Bottle 2"),
        ("Bottle 3", "/Slot 1/Items/Bottle 3"),
        ("Bottle 4", "/Slot 1/Items/Bottle 4"),
        ("Half magic use", "/Slot 1/Items/Half magic use"),
    ] },
    Section { title: "Slot 2", rows: &[
        ("Max health (8 per heart)", "/Slot 2/Max health (8 per heart)"),
        ("Health (8 per heart)", "/Slot 2/Health (8 per heart)"),
        ("Heart pieces", "/Slot 2/Heart pieces"),
        ("Magic", "/Slot 2/Magic"),
        ("Rupees", "/Slot 2/Rupees"),
        ("Bombs", "/Slot 2/Bombs"),
        ("Bomb capacity upgrades (0-7)", "/Slot 2/Bomb capacity upgrades (0-7)"),
        ("Arrows", "/Slot 2/Arrows"),
        ("Arrow capacity upgrades (0-7)", "/Slot 2/Arrow capacity upgrades (0-7)"),
        ("Rupees in the fairy pond", "/Slot 2/Rupees in the fairy pond"),
    ] },
    Section { title: "Slot 2: items", rows: &[
        ("Sword", "/Slot 2/Items/Sword"),
        ("Shield", "/Slot 2/Items/Shield"),
        ("Tunic", "/Slot 2/Items/Tunic"),
        ("Bow", "/Slot 2/Items/Bow"),
        ("Boomerang", "/Slot 2/Items/Boomerang"),
        ("Mushroom / magic powder", "/Slot 2/Items/Mushroom / magic powder"),
        ("Shovel / flute", "/Slot 2/Items/Shovel / flute"),
        ("Gloves", "/Slot 2/Items/Gloves"),
        ("Magic mirror", "/Slot 2/Items/Magic mirror"),
        ("Hookshot", "/Slot 2/Items/Hookshot"),
        ("Fire rod", "/Slot 2/Items/Fire rod"),
        ("Ice rod", "/Slot 2/Items/Ice rod"),
        ("Bombos medallion", "/Slot 2/Items/Bombos medallion"),
        ("Ether medallion", "/Slot 2/Items/Ether medallion"),
        ("Quake medallion", "/Slot 2/Items/Quake medallion"),
        ("Lamp", "/Slot 2/Items/Lamp"),
        ("Magic hammer", "/Slot 2/Items/Magic hammer"),
        ("Bug-catching net", "/Slot 2/Items/Bug-catching net"),
        ("Book of Mudora", "/Slot 2/Items/Book of Mudora"),
        ("Cane of Somaria", "/Slot 2/Items/Cane of Somaria"),
        ("Cane of Byrna", "/Slot 2/Items/Cane of Byrna"),
        ("Magic cape", "/Slot 2/Items/Magic cape"),
        ("Pegasus boots", "/Slot 2/Items/Pegasus boots"),
        ("Zora's flippers", "/Slot 2/Items/Zora's flippers"),
        ("Moon pearl", "/Slot 2/Items/Moon pearl"),
        ("Bottle 1", "/Slot 2/Items/Bottle 1"),
        ("Bottle 2", "/Slot 2/Items/Bottle 2"),
        ("Bottle 3", "/Slot 2/Items/Bottle 3"),
        ("Bottle 4", "/Slot 2/Items/Bottle 4"),
        ("Half magic use", "/Slot 2/Items/Half magic use"),
    ] },
    Section { title: "Slot 3", rows: &[
        ("Max health (8 per heart)", "/Slot 3/Max health (8 per heart)"),
        ("Health (8 per heart)", "/Slot 3/Health (8 per heart)"),
        ("Heart pieces", "/Slot 3/Heart pieces"),
        ("Magic", "/Slot 3/Magic"),
        ("Rupees", "/Slot 3/Rupees"),
        ("Bombs", "/Slot 3/Bombs"),
        ("Bomb capacity upgrades (0-7)", "/Slot 3/Bomb capacity upgrades (0-7)"),
        ("Arrows", "/Slot 3/Arrows"),
        ("Arrow capacity upgrades (0-7)", "/Slot 3/Arrow capacity upgrades (0-7)"),
        ("Rupees in the fairy pond", "/Slot 3/Rupees in the fairy pond"),
    ] },
    Section { title: "Slot 3: items", rows: &[
        ("Sword", "/Slot 3/Items/Sword"),
        ("Shield", "/Slot 3/Items/Shield"),
        ("Tunic", "/Slot 3/Items/Tunic"),
        ("Bow", "/Slot 3/Items/Bow"),
        ("Boomerang", "/Slot 3/Items/Boomerang"),
        ("Mushroom / magic powder", "/Slot 3/Items/Mushroom / magic powder"),
        ("Shovel / flute", "/Slot 3/Items/Shovel / flute"),
        ("Gloves", "/Slot 3/Items/Gloves"),
        ("Magic mirror", "/Slot 3/Items/Magic mirror"),
        ("Hookshot", "/Slot 3/Items/Hookshot"),
        ("Fire rod", "/Slot 3/Items/Fire rod"),
        ("Ice rod", "/Slot 3/Items/Ice rod"),
        ("Bombos medallion", "/Slot 3/Items/Bombos medallion"),
        ("Ether medallion", "/Slot 3/Items/Ether medallion"),
        ("Quake medallion", "/Slot 3/Items/Quake medallion"),
        ("Lamp", "/Slot 3/Items/Lamp"),
        ("Magic hammer", "/Slot 3/Items/Magic hammer"),
        ("Bug-catching net", "/Slot 3/Items/Bug-catching net"),
        ("Book of Mudora", "/Slot 3/Items/Book of Mudora"),
        ("Cane of Somaria", "/Slot 3/Items/Cane of Somaria"),
        ("Cane of Byrna", "/Slot 3/Items/Cane of Byrna"),
        ("Magic cape", "/Slot 3/Items/Magic cape"),
        ("Pegasus boots", "/Slot 3/Items/Pegasus boots"),
        ("Zora's flippers", "/Slot 3/Items/Zora's flippers"),
        ("Moon pearl", "/Slot 3/Items/Moon pearl"),
        ("Bottle 1", "/Slot 3/Items/Bottle 1"),
        ("Bottle 2", "/Slot 3/Items/Bottle 2"),
        ("Bottle 3", "/Slot 3/Items/Bottle 3"),
        ("Bottle 4", "/Slot 3/Items/Bottle 4"),
        ("Half magic use", "/Slot 3/Items/Half magic use"),
    ] },
    ],
    lists: &[],
    tables: &[],
};

// ------------------------------------------------------------------ Ys: The Oath in Felghana

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

// ------------------------------------------------------------------ Bravely Default II

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

// ------------------------------------------------------------------ Trails in the Sky the 3rd

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

// ------------------------------------------------------------------ Trails of Cold Steel

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
