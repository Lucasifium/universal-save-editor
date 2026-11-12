//! One `Spec` per game. Names and ids come from public editors (credited in
//! THIRD_PARTY_NOTICES.md); values are only listed once they read correctly on a real save.

use super::{List, Max, Section, Shape, Spec, Table};

pub static SPECS: &[Spec] = &[COTL, FALLOUT_SHELTER, DRG, S1_MONEY, S1_RANK, S1_TIME, S1_GAME, SOTF_STATE, SOTF_GAME, SOTF_WEATHER, SOTF_ITEMS, DMD, CUPHEAD, ANTIMATTER, PVZ, CELESTE, DRGS, KH3, RF4S, SKY1ST, KSP, FFPR, MCD, STONESHARD, ROGUE_TRADER, BIOSHOCK_INFINITE, SPACE_HAVEN, SNOWRUNNER, GRAVEYARD_KEEPER, NFSMW, TOS, GTA3, GTAVC, BITBURNER, TERRA_INVICTA, PRISON, FF13, SONIC_MANIA];

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
