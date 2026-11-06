//! Oxygen Not Included: the map of every asteroid and rocket interior in a save, and the
//! brush strokes that change it.
//!
//! All worlds of a save share one grid of cells. The simulation block ("SIMSAVE") keeps,
//! for a grid one cell larger on every side, three arrays one after another: per cell
//! the element (a hash of its name), temperature (K), mass (kg) and radiation; the germ
//! type (a hash, 0 for none) and count; and a third element/temperature/mass record this
//! editor leaves alone. Each world is a rectangle of the grid (its `WorldContainer` gives
//! the offset and size); y runs up from the bottom.

use std::sync::OnceLock;

pub const EDITS: &str = "/Map edits";

/// Klei's hash of a name (lower-cased), used for element and germ ids.
pub fn hash(name: &str) -> u32 {
    let mut h: u32 = 0;
    for c in name.to_lowercase().bytes() {
        h = (c as u32).wrapping_add(h << 6).wrapping_add(h << 16).wrapping_sub(h);
    }
    h
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Vacuum,
    Gas,
    Liquid,
    Solid,
}

impl State {
    pub fn word(self) -> &'static str {
        match self {
            State::Vacuum => "vacuum",
            State::Gas => "gas",
            State::Liquid => "liquid",
            State::Solid => "solid",
        }
    }
}

#[derive(Debug)]
pub struct Element {
    pub id: u32,
    pub name: &'static str,
    pub state: State,
    pub color: [u8; 3],
    /// In every version of the game, so it can always be painted. Others can be painted
    /// only when the save already has some (they may belong to a DLC the colony lacks).
    pub core: bool,
}

use State::{Gas as G, Liquid as L, Solid as S};

/// (id, name shown, state, colour, in every version).
const TABLE: &[(&str, &str, State, [u8; 3], bool)] = &[
    ("Vacuum", "Vacuum", State::Vacuum, [0, 0, 0], true),
    ("Unobtanium", "Neutronium", S, [60, 52, 70], true),
    ("Oxygen", "Oxygen", G, [130, 190, 235], true),
    ("ContaminatedOxygen", "Polluted Oxygen", G, [150, 175, 110], true),
    ("CarbonDioxide", "Carbon Dioxide", G, [95, 95, 95], true),
    ("Hydrogen", "Hydrogen", G, [230, 150, 200], true),
    ("ChlorineGas", "Chlorine Gas", G, [200, 230, 120], true),
    ("Methane", "Natural Gas", G, [210, 180, 150], true),
    ("Steam", "Steam", G, [200, 210, 225], true),
    ("SourGas", "Sour Gas", G, [160, 150, 90], false),
    ("Propane", "Propane", G, [200, 170, 210], false),
    ("Helium", "Helium", G, [240, 220, 160], false),
    ("CarbonGas", "Carbon Gas", G, [90, 80, 80], false),
    ("RockGas", "Rock Gas", G, [200, 120, 80], false),
    ("SaltGas", "Salt Gas", G, [230, 220, 220], false),
    ("CopperGas", "Copper Gas", G, [220, 140, 90], false),
    ("GoldGas", "Gold Gas", G, [240, 210, 110], false),
    ("IronGas", "Iron Gas", G, [200, 150, 140], false),
    ("SteelGas", "Steel Gas", G, [190, 190, 200], false),
    ("TungstenGas", "Tungsten Gas", G, [180, 170, 200], false),
    ("NiobiumGas", "Niobium Gas", G, [170, 180, 210], false),
    ("AluminumGas", "Aluminum Gas", G, [200, 210, 220], false),
    ("CobaltGas", "Cobalt Gas", G, [120, 140, 220], false),
    ("LeadGas", "Lead Gas", G, [150, 150, 170], false),
    ("MercuryGas", "Mercury Gas", G, [200, 200, 210], false),
    ("PhosphorusGas", "Phosphorus Gas", G, [230, 210, 200], false),
    ("SulfurGas", "Sulfur Gas", G, [230, 220, 120], false),
    ("EthanolGas", "Ethanol Gas", G, [210, 190, 160], false),
    ("SuperCoolantGas", "Super Coolant Gas", G, [150, 220, 230], false),
    ("Fallout", "Nuclear Fallout", G, [150, 200, 60], false),
    ("Syngas", "Syngas", G, [180, 160, 200], false),
    ("Water", "Water", L, [40, 110, 210], true),
    ("DirtyWater", "Polluted Water", L, [110, 120, 60], true),
    ("SaltWater", "Salt Water", L, [70, 140, 200], true),
    ("Brine", "Brine", L, [120, 160, 190], true),
    ("CrudeOil", "Crude Oil", L, [60, 45, 35], true),
    ("Petroleum", "Petroleum", L, [200, 160, 60], true),
    ("Magma", "Magma", L, [240, 90, 30], true),
    ("Chlorine", "Liquid Chlorine", L, [180, 220, 90], false),
    ("LiquidMethane", "Liquid Methane", L, [200, 170, 140], false),
    ("Naphtha", "Naphtha", L, [150, 110, 90], false),
    ("Ethanol", "Ethanol", L, [200, 170, 120], false),
    ("SuperCoolant", "Super Coolant", L, [90, 200, 220], false),
    ("ViscoGel", "Visco-Gel", L, [200, 110, 200], false),
    ("Mercury", "Mercury", L, [190, 190, 200], false),
    ("LiquidSulfur", "Liquid Sulfur", L, [230, 200, 60], false),
    ("LiquidOxygen", "Liquid Oxygen", L, [110, 170, 230], false),
    ("LiquidHydrogen", "Liquid Hydrogen", L, [230, 140, 190], false),
    ("LiquidCarbonDioxide", "Liquid Carbon Dioxide", L, [90, 90, 100], false),
    ("LiquidPropane", "Liquid Propane", L, [190, 160, 200], false),
    ("LiquidHelium", "Liquid Helium", L, [240, 220, 170], false),
    ("LiquidPhosphorus", "Liquid Phosphorus", L, [230, 200, 190], false),
    ("MoltenCopper", "Molten Copper", L, [240, 130, 60], false),
    ("MoltenGold", "Molten Gold", L, [250, 200, 70], false),
    ("MoltenIron", "Molten Iron", L, [240, 110, 70], false),
    ("MoltenSteel", "Molten Steel", L, [230, 150, 120], false),
    ("MoltenTungsten", "Molten Tungsten", L, [220, 140, 170], false),
    ("MoltenNiobium", "Molten Niobium", L, [200, 150, 190], false),
    ("MoltenAluminum", "Molten Aluminum", L, [230, 170, 150], false),
    ("MoltenCobalt", "Molten Cobalt", L, [170, 120, 220], false),
    ("MoltenLead", "Molten Lead", L, [200, 140, 140], false),
    ("MoltenUranium", "Molten Uranium", L, [160, 230, 60], false),
    ("MoltenGlass", "Molten Glass", L, [250, 160, 80], false),
    ("MoltenSalt", "Molten Salt", L, [250, 200, 170], false),
    ("MoltenCarbon", "Liquid Carbon", L, [120, 60, 50], false),
    ("MoltenSucrose", "Molten Sucrose", L, [230, 190, 120], false),
    ("MoltenSyngas", "Liquid Syngas", L, [170, 150, 200], false),
    ("Resin", "Liquid Resin", L, [200, 130, 50], false),
    ("NuclearWaste", "Nuclear Waste", L, [130, 200, 40], false),
    ("Milk", "Brackene", L, [230, 225, 210], false),
    ("PhytoOil", "Phyto Oil", L, [170, 190, 80], false),
    ("MurkyBrine", "Murky Brine", L, [90, 110, 100], false),
    ("SugarWater", "Nectar", L, [220, 170, 190], false),
    ("Granite", "Granite", S, [150, 120, 120], true),
    ("SandStone", "Sandstone", S, [200, 160, 110], true),
    ("IgneousRock", "Igneous Rock", S, [120, 115, 120], true),
    ("SedimentaryRock", "Sedimentary Rock", S, [160, 130, 100], true),
    ("Obsidian", "Obsidian", S, [55, 45, 65], true),
    ("Katairite", "Abyssalite", S, [95, 75, 130], true),
    ("Dirt", "Dirt", S, [130, 90, 60], true),
    ("Clay", "Clay", S, [180, 120, 90], true),
    ("Sand", "Sand", S, [225, 200, 140], true),
    ("Algae", "Algae", S, [70, 140, 70], true),
    ("Fertilizer", "Fertilizer", S, [140, 110, 70], true),
    ("Ice", "Ice", S, [170, 210, 240], true),
    ("DirtyIce", "Polluted Ice", S, [160, 180, 150], true),
    ("BrineIce", "Brine Ice", S, [180, 200, 220], true),
    ("CrushedIce", "Crushed Ice", S, [200, 225, 245], false),
    ("Snow", "Snow", S, [235, 240, 250], true),
    ("Carbon", "Coal", S, [45, 40, 40], true),
    ("RefinedCarbon", "Refined Carbon", S, [30, 30, 35], true),
    ("Cuprite", "Copper Ore", S, [190, 110, 70], true),
    ("Copper", "Copper", S, [210, 120, 70], true),
    ("GoldAmalgam", "Gold Amalgam", S, [210, 180, 90], true),
    ("Gold", "Gold", S, [240, 200, 80], true),
    ("IronOre", "Iron Ore", S, [160, 90, 80], true),
    ("Iron", "Iron", S, [170, 150, 150], true),
    ("Wolframite", "Wolframite", S, [110, 90, 110], true),
    ("Tungsten", "Tungsten", S, [140, 130, 150], true),
    ("Steel", "Steel", S, [180, 180, 190], true),
    ("Glass", "Glass", S, [190, 225, 235], true),
    ("Diamond", "Diamond", S, [200, 240, 250], true),
    ("Fossil", "Fossil", S, [215, 205, 180], true),
    ("Regolith", "Regolith", S, [140, 130, 125], true),
    ("Lime", "Lime", S, [225, 225, 200], true),
    ("SlimeMold", "Slime", S, [120, 150, 60], true),
    ("BleachStone", "Bleach Stone", S, [200, 230, 150], true),
    ("OxyRock", "Oxylite", S, [100, 170, 210], true),
    ("Rust", "Rust", S, [170, 80, 50], true),
    ("Salt", "Salt", S, [235, 230, 230], true),
    ("Phosphorite", "Phosphorite", S, [200, 170, 160], true),
    ("Phosphorus", "Refined Phosphorus", S, [220, 190, 180], false),
    ("ToxicSand", "Polluted Dirt", S, [120, 110, 70], true),
    ("Polypropylene", "Plastic", S, [230, 230, 200], false),
    ("Isoresin", "Isoresin", S, [210, 150, 60], false),
    ("Niobium", "Niobium", S, [150, 160, 190], false),
    ("Fullerene", "Fullerene", S, [90, 90, 100], false),
    ("Electrum", "Electrum", S, [210, 200, 140], false),
    ("Lead", "Lead", S, [120, 120, 140], false),
    ("Aluminum", "Aluminum", S, [200, 205, 215], false),
    ("AluminumOre", "Aluminum Ore", S, [170, 150, 140], false),
    ("Cobalt", "Cobalt", S, [100, 120, 200], false),
    ("Cobaltite", "Cobalt Ore", S, [80, 100, 170], false),
    ("UraniumOre", "Uranium Ore", S, [110, 160, 50], false),
    ("EnrichedUranium", "Enriched Uranium", S, [120, 220, 60], false),
    ("DepletedUranium", "Depleted Uranium", S, [100, 130, 70], false),
    ("SolidNuclearWaste", "Solid Nuclear Waste", S, [110, 170, 40], false),
    ("Graphite", "Graphite", S, [70, 70, 80], false),
    ("Sucrose", "Sucrose", S, [240, 235, 225], false),
    ("SolidResin", "Solid Resin", S, [190, 120, 40], false),
    ("Mud", "Mud", S, [110, 80, 55], false),
    ("ToxicMud", "Polluted Mud", S, [100, 95, 60], false),
    ("MaficRock", "Mafic Rock", S, [90, 85, 95], true),
    ("Sulfur", "Sulfur", S, [230, 210, 80], false),
    ("Bitumen", "Bitumen", S, [40, 35, 30], false),
    ("SuperInsulator", "Insulite", S, [230, 150, 170], false),
    ("TempConductorSolid", "Thermium", S, [170, 200, 220], false),
    ("Ceramic", "Ceramic", S, [220, 200, 180], false),
    ("Brick", "Brick", S, [180, 90, 70], false),
    ("Cement", "Cement", S, [190, 190, 180], false),
    ("CrushedRock", "Crushed Rock", S, [150, 140, 135], false),
    ("FoolsGold", "Pyrite", S, [200, 170, 80], false),
    ("Cinnabar", "Cinnabar", S, [190, 70, 70], false),
    ("SolidMercury", "Solid Mercury", S, [180, 180, 190], false),
    ("SolidCrudeOil", "Solid Crude Oil", S, [50, 40, 35], false),
    ("SolidPetroleum", "Solid Petroleum", S, [190, 150, 60], false),
    ("SolidNaphtha", "Solid Naphtha", S, [140, 100, 80], false),
    ("SolidChlorine", "Solid Chlorine", S, [170, 210, 90], false),
    ("SolidMethane", "Solid Methane", S, [200, 180, 150], false),
    ("SolidCarbonDioxide", "Solid Carbon Dioxide", S, [110, 110, 115], false),
    ("SolidOxygen", "Solid Oxygen", S, [120, 170, 220], false),
    ("SolidHydrogen", "Solid Hydrogen", S, [220, 140, 190], false),
    ("SolidPropane", "Solid Propane", S, [180, 150, 190], false),
    ("SolidEthanol", "Solid Ethanol", S, [190, 160, 120], false),
    ("SolidSuperCoolant", "Solid Super Coolant", S, [90, 190, 210], false),
    ("SolidViscoGel", "Solid Visco-Gel", S, [190, 100, 190], false),
    ("SolidSyngas", "Solid Syngas", S, [160, 140, 190], false),
    ("MilkFat", "Brackwax", S, [235, 225, 200], false),
    ("MilkIce", "Frozen Brackene", S, [230, 230, 220], false),
    ("FrozenPhytoOil", "Frozen Phyto Oil", S, [160, 180, 80], false),
    ("Tallow", "Tallow", S, [230, 215, 170], false),
    ("Peat", "Peat", S, [90, 70, 50], false),
    ("Amber", "Amber", S, [230, 150, 40], false),
    ("WoodLog", "Wood", S, [150, 100, 60], false),
    ("Radium", "Radium", S, [150, 230, 120], false),
    ("Yellowcake", "Yellowcake", S, [220, 210, 60], false),
    ("Corium", "Corium", L, [180, 230, 60], false),
    ("Isosap", "Isosap", S, [200, 160, 80], false),
    ("Zinc", "Zinc", S, [170, 180, 190], false),
    ("ZincOre", "Zinc Ore", S, [140, 150, 160], false),
    ("MoltenZinc", "Molten Zinc", L, [220, 160, 140], false),
    ("ZincGas", "Zinc Gas", G, [200, 200, 210], false),
    ("Iridium", "Iridium", S, [200, 200, 220], false),
    ("MoltenIridium", "Molten Iridium", L, [230, 170, 200], false),
    ("Nickel", "Nickel", S, [180, 180, 170], false),
    ("NickelOre", "Nickel Ore", S, [150, 150, 130], false),
    ("Shale", "Shale", S, [100, 100, 110], false),
    ("SiltStone", "Siltstone", S, [170, 150, 120], false),
    ("Basalt", "Basalt", S, [70, 70, 75], false),
    ("UnobtaniumAlloy", "Neutronium Alloy", S, [90, 80, 100], false),
    ("MurkyBrineIce", "Murky Brine Ice", S, [150, 170, 165], false),
];

pub fn elements() -> &'static [Element] {
    static E: OnceLock<Vec<Element>> = OnceLock::new();
    E.get_or_init(|| TABLE.iter().map(|&(id, name, state, color, core)| Element { id: hash(id), name, state, color, core }).collect())
}

pub fn element(id: u32) -> Option<&'static Element> {
    elements().iter().find(|e| e.id == id)
}

pub fn element_name(id: u32) -> String {
    element(id).map_or_else(|| format!("Unknown element #{id:08x}"), |e| e.name.to_string())
}

pub const VACUUM: &str = "Vacuum";
pub fn vacuum() -> u32 {
    hash(VACUUM)
}
pub fn neutronium() -> u32 {
    hash("Unobtanium")
}

/// Germ types, by the hash the save keeps.
pub fn germ_name(id: u32) -> String {
    for (k, name) in [
        ("FoodPoisoning", "Food Poisoning"),
        ("SlimeLung", "Slimelung"),
        ("PollenGerms", "Floral Scents"),
        ("ZombieSpores", "Zombie Spores"),
        ("RadiationSickness", "Radioactive Contaminants"),
    ] {
        if hash(k) == id {
            return name.to_string();
        }
    }
    format!("Germs #{id:08x}")
}

/// One cell of the grid, as far as the editor changes it.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Cell {
    pub element: u32,
    /// Kelvin.
    pub temp: f32,
    /// Kilograms.
    pub mass: f32,
    pub germ: u32,
    pub germs: i32,
}

#[derive(Clone, Debug)]
pub struct World {
    pub id: i32,
    pub name: String,
    pub rocket: bool,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub discovered: bool,
    pub start: bool,
    /// Its key under `/Worlds` in the value tree.
    pub key: String,
}

pub struct PoiKind {
    pub name: &'static str,
    pub color: [u8; 3],
    pub shown: bool,
}

pub const POI_KINDS: [PoiKind; 7] = [
    PoiKind { name: "Geysers & vents", color: [90, 210, 255], shown: true },
    PoiKind { name: "Teleporters", color: [200, 120, 255], shown: true },
    PoiKind { name: "Gravitas finds", color: [255, 200, 70], shown: true },
    PoiKind { name: "Story traits", color: [255, 130, 90], shown: true },
    PoiKind { name: "Artifacts", color: [240, 240, 140], shown: true },
    PoiKind { name: "Printing Pod", color: [120, 255, 140], shown: true },
    PoiKind { name: "Duplicants", color: [255, 255, 255], shown: false },
];

#[derive(Clone, Debug)]
pub struct Poi {
    pub kind: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
}

fn words(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c == '_' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if (c.is_uppercase() && prev_lower) || (c.is_ascii_digit() && prev_lower && !out.ends_with(|p: char| p.is_ascii_digit() || p.is_uppercase())) {
            out.push(' ');
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        out.push(c);
    }
    let mut cs = out.chars();
    cs.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + cs.as_str())
}

/// The point of interest a prefab is, if it's one: (kind, name).
pub fn poi_of(prefab: &str) -> Option<(usize, String)> {
    if let Some(g) = prefab.strip_prefix("GeyserGeneric_") {
        let name = match g {
            "steam" => "Cool Steam Vent",
            "hot_steam" => "Steam Vent",
            "hot_water" => "Water Geyser",
            "slush_water" => "Cool Slush Geyser",
            "filthy_water" => "Polluted Water Vent",
            "slush_salt_water" => "Cool Salt Slush Geyser",
            "salt_water" => "Salt Water Geyser",
            "small_volcano" => "Minor Volcano",
            "big_volcano" => "Volcano",
            "liquid_co2" => "Carbon Dioxide Geyser",
            "hot_co2" => "Carbon Dioxide Vent",
            "hot_hydrogen" => "Hydrogen Vent",
            "hot_po2" => "Hot Polluted Oxygen Vent",
            "slimy_po2" => "Infectious Polluted Oxygen Vent",
            "chlorine_gas" => "Chlorine Gas Vent",
            "chlorine_gas_cool" => "Cool Chlorine Gas Vent",
            "methane" => "Natural Gas Geyser",
            "molten_copper" => "Copper Volcano",
            "molten_iron" => "Iron Volcano",
            "molten_gold" => "Gold Volcano",
            "molten_aluminum" => "Aluminum Volcano",
            "molten_tungsten" => "Tungsten Volcano",
            "molten_niobium" => "Niobium Volcano",
            "molten_cobalt" => "Cobalt Volcano",
            "oil_drip" => "Leaky Oil Fissure",
            "liquid_sulfur" => "Liquid Sulfur Geyser",
            "molten_zinc" => "Zinc Volcano",
            "molten_iridium" => "Iridium Volcano",
            "ethanol" => "Ethanol Geyser",
            "murky_brine" => "Murky Brine Geyser",
            _ => "",
        };
        let name = if name.is_empty() { format!("{} geyser", words(g)) } else { name.to_string() };
        return Some((0, name));
    }
    let (kind, name) = match prefab {
        "OilWell" => (0, "Oil Reservoir"),
        "UnderwaterVent" => (0, "Underwater Vent"),
        "SmallReefGeyser" => (0, "Reef Geyser"),
        "WarpPortal" => (1, "Teleporter Transmitter"),
        "WarpReceiver" => (1, "Teleporter Receiver"),
        "WarpConduitSender" => (1, "Supply Teleporter Input"),
        "WarpConduitReceiver" => (1, "Supply Teleporter Output"),
        "TemporalTearOpener" => (1, "Temporal Tear Opener"),
        "GeneShuffler" => (2, "Neural Vacillator"),
        "CryoTank" => (2, "Cryotank 3000"),
        "SetLocker" => (2, "Locker"),
        "VendingMachine" => (2, "Vending Machine"),
        "GravitasPedestal" => (2, "Gravitas Pedestal"),
        "GravitasContainer" => (2, "Gravitas Container"),
        "PropReceptionDesk" => (2, "Reception Desk"),
        "MegaBrainTank" => (3, "Somnium Synthesizer"),
        "FossilDig" => (3, "Ancient Specimen"),
        "FossilIce" | "FossilResin" | "FossilRock" | "FossilBitsLarge" | "FossilBitsSmall" | "FossilSculpture" => (3, "Fossil"),
        "HijackedHeadquarters" => (3, "Hijacked Headquarters"),
        "MorbRoverMaker" => (3, "Biobot Builder"),
        "CritterManipulator" | "GravitasCreatureManipulator" => (3, "Critter Flux-O-Matic"),
        "LonelyMinionHouse" => (3, "Lonely Duplicant's house"),
        "Headquarters" => (5, "Printing Pod"),
        "Minion" => (6, "Duplicant"),
        _ => {
            if let Some(a) = prefab.strip_prefix("keepsake_") {
                return Some((4, format!("Keepsake: {}", words(a))));
            }
            if let Some(a) = prefab.strip_prefix("artifact_") {
                let name = match a {
                    "ameliaswatch" => "Amelia's Watch".to_string(),
                    "eggrock" => "Egg Rock".to_string(),
                    "honeyjar" => "Honey Jar".to_string(),
                    "pacupercolator" => "Pacu Percolator".to_string(),
                    "rocktornado" => "Rock Tornado".to_string(),
                    "solarsystem" => "Solar System".to_string(),
                    "sink" => "Kitchen Sink".to_string(),
                    "teapot" => "Teapot".to_string(),
                    "brickphone" => "Brick Phone".to_string(),
                    "modernart" => "Modern Art".to_string(),
                    "hatchfossil" => "Hatch Fossil".to_string(),
                    "rubikscube" => "Puzzle Cube".to_string(),
                    "officemug" => "Office Mug".to_string(),
                    "bioluminescentrock" => "Bioluminescent Rock".to_string(),
                    "grubstatue" => "Grub Statue".to_string(),
                    "visonaryart" | "visionaryart" => "Visionary Art".to_string(),
                    "plasmalamp" => "Plasma Lamp".to_string(),
                    "robotarm" => "Robot Arm".to_string(),
                    "tallstalk" => "Tall Stalk".to_string(),
                    "dnamodel" => "DNA Model".to_string(),
                    "shieldgenerator" => "Shield Generator".to_string(),
                    "russiandolls" => "Nesting Dolls".to_string(),
                    _ => words(a),
                };
                return Some((4, format!("Artifact: {name}")));
            }
            return None;
        }
    };
    Some((kind, name.to_string()))
}

/// Space points of interest live on the star map, not in the grid.
pub fn starmap_name(prefab: &str) -> Option<String> {
    if let Some(p) = prefab.strip_prefix("HarvestableSpacePOI_") {
        return Some(words(p));
    }
    if let Some(p) = prefab.strip_prefix("ArtifactSpacePOI_") {
        return Some(words(p));
    }
    (prefab == "TemporalTear").then(|| "Temporal Tear".to_string())
}

/// Single-cell buildings that make their cell solid (tiles): brushes leave them alone.
pub fn is_tile_building(prefab: &str) -> bool {
    prefab.ends_with("Tile") && prefab != "TilePOI" || prefab == "GasPermeableMembrane"
}

#[derive(Debug)]
pub struct OniMap {
    /// The world grid (cells of every world).
    pub width: usize,
    pub height: usize,
    /// y * width + x, y up from the bottom.
    pub cells: Vec<Cell>,
    pub radiation: Vec<f32>,
    /// Cells taken by tile buildings.
    pub protected: Vec<bool>,
    /// Discovered cells (fog of war), when the save has them.
    pub visible: Option<Vec<u8>>,
    pub worlds: Vec<World>,
    pub pois: Vec<Poi>,
    /// Star map points of interest: name and how many.
    pub starmap: Vec<(String, usize)>,
    /// Elements found in the save.
    pub present: Vec<u32>,
    pub(crate) sim_at: usize,
    pub(crate) sim_w: usize,
    pub(crate) sim_h: usize,
    pub(crate) visible_at: Option<usize>,
}

const SIM_HEADER: usize = 29;

impl OniMap {
    /// Reads the grid from the simulation block at `at` in the body.
    pub fn read(body: &[u8], at: usize, len: usize) -> Option<(Vec<Cell>, Vec<f32>, usize, usize)> {
        let sim = body.get(at..at + len)?;
        if sim.get(..8)? != b"SIMSAVE\0" {
            return None;
        }
        let u = |o: usize| u32::from_le_bytes(sim[o..o + 4].try_into().unwrap()) as usize;
        let (sw, sh) = (u(12), u(16));
        let n = sw.checked_mul(sh)?;
        if sw < 3 || sh < 3 || len != SIM_HEADER + 36 * n {
            return None;
        }
        let (w, h) = (sw - 2, sh - 2);
        let mut cells = Vec::with_capacity(w * h);
        let mut rad = Vec::with_capacity(w * h);
        let f = |o: usize| f32::from_le_bytes(sim[o..o + 4].try_into().unwrap());
        for y in 0..h {
            for x in 0..w {
                let si = (y + 1) * sw + x + 1;
                let a = SIM_HEADER + 16 * si;
                let g = SIM_HEADER + 16 * n + 8 * si;
                cells.push(Cell { element: u(a) as u32, temp: f(a + 4), mass: f(a + 8), germ: u(g) as u32, germs: u(g + 4) as u32 as i32 });
                rad.push(f(a + 12));
            }
        }
        Some((cells, rad, sw, sh))
    }

    pub fn world(&self, id: i32) -> Option<&World> {
        self.worlds.iter().find(|w| w.id == id)
    }

    /// Elements a brush can paint in this save.
    pub fn paintable(&self) -> Vec<&'static Element> {
        let mut v: Vec<&'static Element> = elements().iter().filter(|e| e.core || self.present.contains(&e.id)).collect();
        v.sort_by_key(|e| e.name);
        v
    }

    /// Writes cells back into the body (the simulation block keeps its size).
    fn put(&self, body: &mut [u8], x: usize, y: usize, c: &Cell) {
        let n = self.sim_w * self.sim_h;
        let si = (y + 1) * self.sim_w + x + 1;
        let a = self.sim_at + SIM_HEADER + 16 * si;
        body[a..a + 4].copy_from_slice(&c.element.to_le_bytes());
        body[a + 4..a + 8].copy_from_slice(&c.temp.to_le_bytes());
        body[a + 8..a + 12].copy_from_slice(&c.mass.to_le_bytes());
        let g = self.sim_at + SIM_HEADER + 16 * n + 8 * si;
        body[g..g + 4].copy_from_slice(&c.germ.to_le_bytes());
        body[g + 4..g + 8].copy_from_slice(&c.germs.to_le_bytes());
    }

    /// Applies map edits to the body.
    pub fn apply_to_body(&self, body: &mut [u8], ops: &[Op]) -> Result<(), String> {
        let mut cells = self.cells.clone();
        let mut visible = self.visible.clone();
        let mut touched = vec![false; cells.len()];
        for op in ops {
            op_ok(op, self)?;
            if op.base() == REVEAL {
                if let (Some(v), Some(w)) = (visible.as_mut(), self.world(op.world as i32)) {
                    for y in w.y..w.y + w.h {
                        v[y * self.width + w.x..y * self.width + w.x + w.w].fill(255);
                    }
                }
                continue;
            }
            for &pt in &op.points {
                for (x, y) in op.stamp(pt, self) {
                    let i = y * self.width + x;
                    let new = apply_cell(op, cells[i], self.protected[i]);
                    if new != cells[i] {
                        cells[i] = new;
                        touched[i] = true;
                    }
                }
            }
        }
        for (i, c) in cells.iter().enumerate() {
            if touched[i] {
                self.put(body, i % self.width, i / self.width, c);
            }
        }
        if let (Some(v), Some(at)) = (visible, self.visible_at) {
            body[at..at + v.len()].copy_from_slice(&v);
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ map edits

pub const PAINT: u8 = 1;
pub const VACUUM_OUT: u8 = 2;
pub const HEAT: u8 = 3;
pub const MASS: u8 = 4;
pub const CLEAN: u8 = 5;
pub const REVEAL: u8 = 6;
/// Added to a mode: the brush changes Neutronium too.
pub const ALSO_NEUTRONIUM: u8 = 0x80;

/// One brush stroke in one world: a square brush of `size` cells stamped at each point
/// (world-grid coordinates, y up).
#[derive(Clone, Debug, PartialEq)]
pub struct Op {
    pub mode: u8,
    pub world: u16,
    pub element: u32,
    pub mass: f32,
    pub temp: f32,
    pub size: u16,
    pub points: Vec<(u32, u32)>,
}

impl Op {
    pub fn base(&self) -> u8 {
        self.mode & !ALSO_NEUTRONIUM
    }

    pub fn to_text(&self) -> String {
        let mut s = format!("{} {} {} {} {} {}", self.mode, self.world, self.element, self.mass, self.temp, self.size);
        for (x, y) in &self.points {
            s.push_str(&format!(" {x} {y}"));
        }
        s
    }

    /// The cells one stamp covers, clipped to the stroke's world.
    pub fn stamp(&self, (x, y): (u32, u32), map: &OniMap) -> impl Iterator<Item = (usize, usize)> {
        let (wx, wy, ww, wh) = map.world(self.world as i32).map_or((0, 0, 0, 0), |w| (w.x as i64, w.y as i64, w.w as i64, w.h as i64));
        let half = (self.size as i64 - 1) / 2;
        let (x0, y0) = ((x as i64 - half).max(wx), (y as i64 - half).max(wy));
        let (x1, y1) = ((x as i64 - half + self.size as i64).min(wx + ww), (y as i64 - half + self.size as i64).min(wy + wh));
        (y0..y1.max(y0)).flat_map(move |yy| (x0..x1.max(x0)).map(move |xx| (xx as usize, yy as usize)))
    }
}

pub fn parse_ops(s: &str) -> Result<Vec<Op>, String> {
    let bad = || "map edits aren't in the expected form".to_string();
    let mut ops = Vec::new();
    for part in s.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let t: Vec<&str> = part.split_whitespace().collect();
        if t.len() < 6 || t.len() % 2 != 0 {
            return Err(bad());
        }
        let mode: u8 = t[0].parse().map_err(|_| bad())?;
        let world: u16 = t[1].parse().map_err(|_| bad())?;
        let element: u32 = t[2].parse().map_err(|_| bad())?;
        let mass: f32 = t[3].parse().map_err(|_| bad())?;
        let temp: f32 = t[4].parse().map_err(|_| bad())?;
        let size: u16 = t[5].parse().map_err(|_| bad())?;
        let n: Vec<u32> = t[6..].iter().map(|v| v.parse::<u32>().map_err(|_| bad())).collect::<Result<_, _>>()?;
        ops.push(Op { mode, world, element, mass, temp, size, points: n.chunks(2).map(|p| (p[0], p[1])).collect() });
    }
    Ok(ops)
}

pub const MAX_MASS: f32 = 100_000.0;
pub const MAX_TEMP: f32 = 9_999.0;

/// Is this a stroke the save can take?
pub fn op_ok(op: &Op, map: &OniMap) -> Result<(), String> {
    let w = map.world(op.world as i32).ok_or("map edit for a world that isn't in this save")?;
    match op.base() {
        PAINT => {
            let e = element(op.element).ok_or("unknown element")?;
            if !e.core && !map.present.contains(&e.id) {
                return Err(format!("{} isn't in this save, so it can't be painted", e.name));
            }
            if e.state != State::Vacuum {
                if !(op.mass > 0.0 && op.mass <= MAX_MASS) {
                    return Err(format!("mass must be above 0 and at most {MAX_MASS} kg"));
                }
                if !(op.temp > 0.0 && op.temp <= MAX_TEMP) {
                    return Err("temperature must be between absolute zero and 9,999 K".into());
                }
            }
        }
        HEAT => {
            if !(op.temp > 0.0 && op.temp <= MAX_TEMP) {
                return Err("temperature must be between absolute zero and 9,999 K".into());
            }
        }
        MASS => {
            if !(op.mass > 0.0 && op.mass <= MAX_MASS) {
                return Err(format!("mass must be above 0 and at most {MAX_MASS} kg"));
            }
        }
        VACUUM_OUT | CLEAN => {}
        REVEAL => return Ok(()),
        _ => return Err("unknown map edit".into()),
    }
    if op.size == 0 || op.size > 64 || op.points.is_empty() {
        return Err("map edit without a brush".into());
    }
    if op.points.iter().any(|&(x, y)| y as usize >= map.height || (x as usize) < w.x || (x as usize) >= w.x + w.w || (y as usize) < w.y || (y as usize) >= w.y + w.h) {
        return Err("map edit outside its world".into());
    }
    Ok(())
}

/// What one brush does to one cell.
pub fn apply_cell(op: &Op, c: Cell, protected: bool) -> Cell {
    if protected || (c.element == neutronium() && op.mode & ALSO_NEUTRONIUM == 0) {
        return c;
    }
    let vac = vacuum();
    let empty = Cell { element: vac, temp: 0.0, mass: 0.0, germ: 0, germs: 0 };
    match op.base() {
        PAINT if op.element == vac => empty,
        PAINT => Cell { element: op.element, temp: op.temp, mass: op.mass, germ: 0, germs: 0 },
        VACUUM_OUT => empty,
        HEAT if c.element != vac => Cell { temp: op.temp, ..c },
        MASS if c.element != vac => Cell { mass: op.mass, ..c },
        CLEAN => Cell { germ: 0, germs: 0, ..c },
        _ => c,
    }
}

/// The colour a cell is drawn in.
pub fn color_of(c: &Cell) -> [u8; 3] {
    match element(c.element) {
        Some(e) => match e.state {
            // Gases are drawn faintly, thicker gas a little stronger.
            State::Gas => {
                let k = (0.25 + (c.mass / 4.0).min(1.0) * 0.35).min(0.6);
                [(e.color[0] as f32 * k) as u8, (e.color[1] as f32 * k) as u8, (e.color[2] as f32 * k) as u8]
            }
            _ => e.color,
        },
        None => {
            let h = c.element;
            [(100 + (h & 0x3F)) as u8, (90 + ((h >> 8) & 0x3F)) as u8, (110 + ((h >> 16) & 0x3F)) as u8]
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_match_the_game() {
        assert_eq!(hash("Vacuum"), 0x2d39bf75);
        assert_eq!(hash("Oxygen"), 0xa4e0b340);
        assert_eq!(hash("Unobtanium"), 0x6d95058c);
        assert_eq!(hash("Water"), 0x6d796197);
        assert_eq!(hash("Katairite"), 0x3fe0146e);
    }

    #[test]
    fn ops_round_trip() {
        let op = Op { mode: PAINT | ALSO_NEUTRONIUM, world: 2, element: hash("Water"), mass: 1000.5, temp: 293.15, size: 3, points: vec![(5, 6), (7, 8)] };
        assert_eq!(parse_ops(&format!("{};{}", op.to_text(), op.to_text())).unwrap(), vec![op.clone(), op]);
        assert!(parse_ops("1 0 0 1").is_err());
    }

    #[test]
    fn brushes() {
        let water = Cell { element: hash("Water"), temp: 300.0, mass: 900.0, germ: 7, germs: 50 };
        let op = |mode| Op { mode, world: 0, element: hash("Granite"), mass: 1200.0, temp: 280.0, size: 1, points: vec![] };
        assert_eq!(apply_cell(&op(PAINT), water, false), Cell { element: hash("Granite"), temp: 280.0, mass: 1200.0, germ: 0, germs: 0 });
        assert_eq!(apply_cell(&op(PAINT), water, true), water);
        assert_eq!(apply_cell(&op(HEAT), water, false).temp, 280.0);
        assert_eq!(apply_cell(&op(MASS), water, false).mass, 1200.0);
        assert_eq!(apply_cell(&op(CLEAN), water, false).germs, 0);
        assert_eq!(apply_cell(&op(VACUUM_OUT), water, false).element, vacuum());
        let n = Cell { element: neutronium(), ..water };
        assert_eq!(apply_cell(&op(VACUUM_OUT), n, false), n);
        assert_eq!(apply_cell(&op(VACUUM_OUT | ALSO_NEUTRONIUM), n, false).element, vacuum());
        let v = Cell { element: vacuum(), temp: 0.0, mass: 0.0, germ: 0, germs: 0 };
        assert_eq!(apply_cell(&op(HEAT), v, false), v);
    }
}
