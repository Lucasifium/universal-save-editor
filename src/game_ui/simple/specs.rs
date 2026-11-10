//! One `Spec` per game. Names and ids come from public editors (credited in
//! THIRD_PARTY_NOTICES.md); values are only listed once they read correctly on a real save.

use super::{List, Max, Section, Shape, Spec, Table};

pub static SPECS: &[Spec] = &[COTL, FALLOUT_SHELTER, DRG];

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
            estimate: true,
            about: "Raises Bismor, Magnite, Croppa, Umanite, Jadiz, Enor Pearl, Phazyonite and the four brewing ingredients you already have. Credits, Data Cells, Error Cubes and Blank Cores are left alone.",
        }),
    }],
    tables: &[Table {
        title: "Dwarves",
        path: "/CharacterSaves",
        name: &["/SavegameID"],
        cols: &[("XP", "/XP"), ("Times retired", "/TimesRetired"), ("Retired levels", "/RetiredCharacterLevels")],
        names: &[
            ("9edd56f1eebcc5488d5b5e5b80b62db4", "Driller"), ("ae56e180fec0c44d96fa29c28366b97b", "Gunner"),
            ("30d8ea17d8fbba4c95306de9655c2f8c", "Scout"), ("85ef626c65f1024a8dfeb5d0f3909d2e", "Engineer"),
        ],
    }],
};
