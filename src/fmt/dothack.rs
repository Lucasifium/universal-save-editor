//! .hack//Infection, Mutation, Outbreak and Quarantine (PlayStation 2) saves: the save folder holds
//! one "dhdataNN" file per slot (34,096 bytes, or 36,228 in the extended games) and a small system
//! file ("<game id>DOTHACK") that keeps a preview and a 16-bit checksum (the sum of all the bytes of
//! the slot file) for each slot. Editing a slot file therefore also rewrites its checksum and
//! preview in the system file, which `sync_system` does for the archive and memory card layers.
//! Offsets and item names follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection
//! (see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = ".hack save";

const SIZES: [usize; 2] = [0x8530, 0x8d84];
const PARTY: usize = 0x7496;
const STRIDE: usize = 0xdc;

pub fn looks(d: &[u8]) -> bool {
    if !SIZES.contains(&d.len()) || d[0] < 0x20 || d[0x17] != 0 {
        return false;
    }
    let u16s = |at: usize| u16::from_le_bytes([d[at], d[at + 1]]);
    (0..3).all(|c| {
        let o = c * STRIDE;
        (1..=99).contains(&u16s(PARTY + o)) && u32::from_le_bytes(d[PARTY + o + 6..PARTY + o + 10].try_into().unwrap()) <= 9_999_999
    })
}

/// Number of party members the file holds (the extended games have 21).
fn party_len(d: &[u8]) -> usize {
    if d.len() == 0x8d84 {
        21
    } else {
        18
    }
}

/// Where member `i` starts (the extended members sit apart from the first 18).
fn member(i: usize) -> usize {
    if i >= 18 {
        0x155c + (i - 18) * STRIDE
    } else {
        i * STRIDE
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a .hack slot".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["General", "Playtime (seconds)"], 0x8400, Kind::U32, 0, 359_999_999);
    b.ranged(&["General", "Progression"], 0x842a, Kind::U8, 0, 255);
    for (i, n) in ["Δ Server", "Θ Server", "Λ Server", "Σ Server", "Ω Server"].iter().enumerate() {
        flag(&mut b, &["Unlocked servers", n], 0x2234, i as u32);
    }
    let cols: [(&str, usize, Kind, i64, i64); 15] = [
        ("Class", 0x7560, Kind::U16, 0, 5),
        ("Level", 0x7496, Kind::U16, 1, 99),
        ("Experience", 0x7498, Kind::U16, 0, 999),
        ("Money", 0x749c, Kind::U32, 0, 9_999_999),
        ("Affection", 0x7562, Kind::U16, 0, 1000),
        ("Max HP", 0x74ac, Kind::U16, 0, 9999),
        ("Max SP", 0x74ae, Kind::U16, 0, 999),
        ("Physical attack", 0x74b0, Kind::U16, 0, 999),
        ("Physical defense", 0x74b2, Kind::U16, 0, 999),
        ("Physical accuracy", 0x74b4, Kind::U16, 0, 999),
        ("Physical evade", 0x74b6, Kind::U16, 0, 999),
        ("Magical attack", 0x74b8, Kind::U16, 0, 999),
        ("Magical defense", 0x74ba, Kind::U16, 0, 999),
        ("Magical accuracy", 0x74bc, Kind::U16, 0, 999),
        ("Magical evade", 0x74be, Kind::U16, 0, 999),
    ];
    for i in 0..party_len(d) {
        let who = format!("{:02} {}", i + 1, CHARS.get(i).unwrap_or(&"Member"));
        for (n, at, k, lo, hi) in cols {
            b.ranged(&["Party", &who, n], at + member(i), k, lo, hi);
        }
    }
    let items: Vec<(String, u64)> = std::iter::once(("-".to_string(), 0xff_ffff)).chain(ITEMS.iter().map(|(i, n)| (n.to_string(), *i as u64))).collect();
    for i in 0..99 {
        let slot = format!("Slot {:02}", i + 1);
        b.pick(&["Elf's Haven", &slot, "Item"], 0xb70 + i * 4, Kind::U24, &items);
        b.ranged(&["Elf's Haven", &slot, "Quantity"], 0xb73 + i * 4, Kind::U8, 0, 99);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. The checksum and preview kept for this slot in the save's system file are written again when you save.")))
}

fn slot_number(name: &str) -> Option<usize> {
    let n = name.rsplit('/').next()?;
    let digits = n.strip_prefix("dhdata")?;
    let k: usize = digits.parse().ok()?;
    (1..=12).contains(&k).then_some(k)
}

fn is_system(name: &str) -> bool {
    name.rsplit('/').next().is_some_and(|n| n.ends_with("DOTHACK"))
}

/// True when `name` is the slot file of a .hack save folder.
pub fn is_slot_file(name: &str) -> bool {
    slot_number(name).is_some()
}

pub fn is_system_file(name: &str) -> bool {
    is_system(name)
}

/// The system file with the entry of slot file `name` (its new bytes are `slot`) brought up to date:
/// the 16-bit byte sum, and, for a slot in use, the level, name and play time shown in the preview.
pub fn system_after(name: &str, sys: &[u8], slot: &[u8]) -> Option<Vec<u8>> {
    let k = slot_number(name)?;
    if sys.len() < 12 * 0x1c || !SIZES.contains(&slot.len()) {
        return None;
    }
    let mut out = sys.to_vec();
    let e = (k - 1) * 0x1c;
    let sum = slot.iter().fold(0u32, |a, b| a.wrapping_add(*b as u32)) as u16;
    if out[e] != 0 {
        out[e + 0x16..e + 0x18].copy_from_slice(&sum.to_le_bytes());
        out[e + 1] = slot[PARTY];
        // The game only copies the name up to its end, leaving older bytes behind it.
        let n = (slot[..0xe].iter().position(|b| *b == 0).unwrap_or(0xd) + 1).min(0xe);
        out[e + 4..e + 4 + n].copy_from_slice(&slot[..n]);
        let play = u32::from_le_bytes(slot[0x8400..0x8404].try_into().unwrap()).min(215_999_940);
        out[e + 0x18..e + 0x1c].copy_from_slice(&play.to_le_bytes());
    }
    Some(out)
}

const CHARS: &[&str] = &["Kite", "Mia", "Orca", "Marlo", "Sanjuro", "Nuke Usagimaru", "Balmung", "Moonstone", "Piros", "Wiseman", "Elk", "Natsume", "Rachel", "Gardenia", "Terajima Ryoko", "BlackRose", "Mistral", "Helba", "Tsukasa", "Subaru", "Sora"];
const ITEMS: &[(u32, &str)] = &[(0x0, "Amateur Blades"), (0x1, "Steel Blades"), (0x2, "Phantom Twin"), (0x3, "Assassin"), (0x4, "Spark Blades"), (0x5, "Lath Blades"), (0x6, "Fuse Blades"), (0x7, "Shadow Blades"), (0x8, "Cats Blades"), (0x9, "Ronin Blades"), (0xa, "Spell Blades"), (0xb, "Bloody Twin"), (0xc, "Sotetsu"), (0xd, "Enou"), (0xe, "Ryokugun"), (0xf, "Raitei"), (0x10, "Anshou"), (0x11, "Masterblades"), (0x12, "Hell's Gate"), (0x13, "Dante's Blades"), (0x14, "Shirogane"), (0x15, "Kurogane"), (0x16, "Yosetu & Fuyou"), (0x17, "Slayers"), (0x18, "Homura"), (0x19, "Tsumuji"), (0x1a, "Mikazuchi"), (0x1b, "Mukuro"), (0x1c, "Magatsu"), (0x1d, "Kyoura"), (0x1e, "Akatsuki"), (0x1f, "Bloody Blades"), (0x20, "Rashou"), (0x21, "Enja"), (0x22, "Hachiyou"), (0x23, "Raikoumaru"), (0x24, "Ankokushiki"), (0x25, "Professional"), (0x26, "Specter Blades"), (0x27, "Kikoku"), (0x28, "Shin"), (0x29, "Rei"), (0x2a, "San"), (0x2b, "Stealth Blades"), (0x2c, "Kurenai"), (0x2d, "Saburo"), (0x2e, "Twin Mizuchi"), (0x2f, "Black Chapter"), (0x30, "Dragon & Tiger"), (0x31, "Kiki & Lala"), (0x32, "Sorcery Swords"), (0x33, "Diablo Blades"), (0x34, "Suigetsu"), (0x35, "Kaien"), (0x36, "Mokuren"), (0x37, "Raian"), (0x38, "Baian"), (0x39, "Miracle Twin"), (0x3a, "Tiger King"), (0x3b, "Sublimer"), (0x10000, "Basic Sword"), (0x10001, "Brave Sword"), (0x10002, "Rondo"), (0x10003, "Mizuchi"), (0x10004, "Gakaku"), (0x10005, "Strange Blade"), (0x10006, "Executioner"), (0x10007, "Unicorn Blade"), (0x10008, "Corpseblade"), (0x10009, "Oval Sword"), (0x1000a, "Grunty's Sword"), (0x1000b, "Fugaku"), (0x1000c, "Ensui"), (0x1000d, "Komura"), (0x1000e, "Souleater"), (0x1000f, "Singing Blade"), (0x10010, "Dogman's Sword"), (0x10011, "Steelblade"), (0x10012, "Glitter"), (0x10013, "Seal Sword"), (0x10014, "Patriot"), (0x10015, "Siegfried"), (0x10016, "Heaven & Earth"), (0x10017, "Lake & Sea"), (0x10018, "Fire & Sky"), (0x10019, "Comet Blade"), (0x1001a, "Demon Killer"), (0x1001b, "6 Side Blade"), (0x1001c, "7 Star Sword"), (0x1001d, "Lion's Blade"), (0x1001e, "Menhir Sword"), (0x1001f, "Matoi"), (0x10020, "Nataku"), (0x10021, "Soujin"), (0x10022, "Bloody Pain"), (0x10023, "Tenkafuubu"), (0x10024, "Phantom Blade"), (0x10025, "Rising Sun"), (0x10026, "Oberon"), (0x10027, "Banished Blade"), (0x10028, "Pegasus Comet"), (0x10029, "Seventh Seal"), (0x1002a, "Fair Soul"), (0x1002b, "Stream Sword"), (0x1002c, "Honor's Breath"), (0x1002d, "Scarlet Cross"), (0x1002e, "Godslayer"), (0x1002f, "Needleblade"), (0x10030, "Shichiyou"), (0x10031, "Gold Sword"), (0x10032, "Trickster"), (0x10033, "Saturn Sword"), (0x10034, "Mercury Sword"), (0x10035, "Mars Sword"), (0x10036, "Phantom Pain"), (0x10037, "Skysweeper"), (0x10038, "Rugrats Nights"), (0x10039, "Evil Eyes"), (0x1003a, "Jormungand"), (0x1003b, "Eternity"), (0x20000, "Adventurer"), (0x20001, "Kikuichimonji"), (0x20002, "Earth Sword"), (0x20003, "Curing Sword"), (0x20004, "Flamberge"), (0x20005, "Green Sword"), (0x20006, "Stun Sword"), (0x20007, "Slayer"), (0x20008, "Nodachi"), (0x20009, "Defense Sword"), (0x2000a, "Magnifier"), (0x2000b, "Shanato"), (0x2000c, "Absorber"), (0x2000d, "Byakuen"), (0x2000e, "Shidan"), (0x2000f, "Raijin"), (0x20010, "Sharp Blade"), (0x20011, "Spiderblade"), (0x20012, "Sun Fang One"), (0x20013, "Devil Blade"), (0x20014, "Claymore"), (0x20015, "Kikujyumonji"), (0x20016, "Earthian Sword"), (0x20017, "Life Sword"), (0x20018, "Flame Sword"), (0x20019, "Dryad's Sword"), (0x2001a, "Light Giver"), (0x2001b, "Pain Giver"), (0x2001c, "Horse-Killer"), (0x2001d, "Blessed Blade"), (0x2001e, "Soul Linker"), (0x2001f, "Stonecrusher"), (0x20020, "Steeler"), (0x20021, "Hien"), (0x20022, "Karin"), (0x20023, "Raimei"), (0x20024, "Sonic Blade"), (0x20025, "Sharktooth"), (0x20026, "Sun Fang Five"), (0x20027, "Ghostdancer"), (0x20028, "High Forger"), (0x20029, "Kikuhyakumonji"), (0x2002a, "Rock'n Roll"), (0x2002b, "Peace Blade"), (0x2002c, "Corona Blade"), (0x2002d, "Forest Sword"), (0x2002e, "Plasma Blade"), (0x2002f, "Executer"), (0x20030, "Jindachi"), (0x20031, "Dispeller"), (0x20032, "Smiling Blade"), (0x20033, "Destroyer"), (0x20034, "Avenger"), (0x20035, "Byakuen Custom"), (0x20036, "Honeyflower"), (0x20037, "Shichishito"), (0x20038, "Divine Speed"), (0x20039, "Narukikyou"), (0x2003a, "Sun Fang Zero"), (0x2003b, "Vajra"), (0x2003c, "Grim Sword"), (0x2003d, "Kikusenmonji"), (0x2003e, "Earthbreaker"), (0x2003f, "Nightingale"), (0x20040, "Honokagutuchi"), (0x20041, "Azure Blade"), (0x20042, "Heavy Tempest"), (0x20043, "Mercy Killer"), (0x20044, "Higekirimaru"), (0x20045, "Guardian Blade"), (0x20046, "Claimh Solais"), (0x20047, "Stonegod Sword"), (0x20048, "Sword of Gain"), (0x20049, "Asian Phoenix"), (0x2004a, "Asura"), (0x2004b, "Thunderlord"), (0x2004c, "Excalibur"), (0x2004d, "Oni's Fork"), (0x2004e, "The Sun Fang"), (0x2004f, "Tsumugari"), (0x30000, "Hatchet"), (0x30001, "Short Swing"), (0x30002, "Battle Axe"), (0x30003, "Water Axe"), (0x30004, "Flame Axe"), (0x30005, "Wind Axe"), (0x30006, "Thunder Axe"), (0x30007, "Midnight Axe"), (0x30008, "Razor Axe"), (0x30009, "Bronze Axe"), (0x3000a, "Cursed Axe"), (0x3000b, "Earth Axe"), (0x3000c, "Water God Axe"), (0x3000d, "Bloody Axe"), (0x3000e, "Bandit's Axe"), (0x3000f, "Charged Axe"), (0x30010, "Darkness Axe"), (0x30011, "Master's Axe"), (0x30012, "Papillon Axe"), (0x30013, "Devil's Axe"), (0x30014, "Handyman's Axe"), (0x30015, "Full Swing"), (0x30016, "Drought Axe"), (0x30017, "Brook Axe"), (0x30018, "Vitality Axe"), (0x30019, "Sinner's Axe"), (0x3001a, "Overloaded"), (0x3001b, "Darkness"), (0x3001c, "Golden Mean"), (0x3001d, "New Scythe"), (0x3001e, "Sorcery's Axe"), (0x3001f, "Quake Axe"), (0x30020, "Dragon Axe"), (0x30021, "Splatter Axe"), (0x30022, "Axe of Mobs"), (0x30023, "Vortex Axe"), (0x30024, "Alien Axe"), (0x30025, "Artisan Axe"), (0x30026, "Fairy Axe"), (0x30027, "Dark God's Axe"), (0x30028, "Lumberjack"), (0x30029, "Over Swing"), (0x3002a, "Desert Axe"), (0x3002b, "Liquid Axe"), (0x3002c, "Vibrant Blade"), (0x3002d, "Brute's Axe"), (0x3002e, "Shock Axe"), (0x3002f, "Black Axe"), (0x30030, "Plasma Axe"), (0x30031, "Miracle Axe"), (0x30032, "Malice's Axe"), (0x30033, "Gaia's Axe"), (0x30034, "Neptune"), (0x30035, "Vampire Axe"), (0x30036, "Crazy Axe"), (0x30037, "Lightning Axe"), (0x30038, "Dimension"), (0x30039, "Maestro Axe"), (0x3003a, "Angel Axe"), (0x3003b, "Missing Axe"), (0x3003c, "Golden Axe"), (0x3003d, "Silver Axe"), (0x40000, "Bronze Spear"), (0x40001, "Iron Spear"), (0x40002, "Relief Lance"), (0x40003, "Water Spear"), (0x40004, "Fire Spear"), (0x40005, "Wooden Spear"), (0x40006, "Electric Spear"), (0x40007, "Glaive"), (0x40008, "Gold Spear"), (0x40009, "Nihonmaru"), (0x4000a, "Spear of Spell"), (0x4000b, "Bloody Lance"), (0x4000c, "Merman Spear"), (0x4000d, "Lavaman Spear"), (0x4000e, "Treeman Spear"), (0x4000f, "Strormer Spear"), (0x40010, "Berserk Spear"), (0x40011, "Sleipnir"), (0x40012, "Fairy Spear"), (0x40013, "Fiend Spear"), (0x40014, "Copper Lance"), (0x40015, "Steel Spear"), (0x40016, "Tranqui Lance"), (0x40017, "River Spear"), (0x40018, "Flame Spear"), (0x40019, "Globe Spear"), (0x4001a, "Blitz Spear"), (0x4001b, "Knight Glaive"), (0x4001c, "Adamant Lance"), (0x4001d, "Super Spear"), (0x4001e, "Mage Spear"), (0x4001f, "Dhampir Lance"), (0x40020, "Fox Spear"), (0x40021, "Blazing Spear"), (0x40022, "Treebeam Spear"), (0x40023, "Flash Spear"), (0x40024, "Insane Spear"), (0x40025, "Dragnir"), (0x40026, "Erling's Spear"), (0x40027, "Demon Spear"), (0x40028, "Sacred Spear"), (0x40029, "Damascus Spear"), (0x4002a, "Happiness"), (0x4002b, "Aqua Spear"), (0x4002c, "Volcanic Spear"), (0x4002d, "Forest Spear"), (0x4002e, "Thunder Spear"), (0x4002f, "Kaiser Glaive"), (0x40030, "Golden Dragon"), (0x40031, "Awful Spear"), (0x40032, "Spear of Curse"), (0x40033, "Demon Lance"), (0x40034, "Rivergod Spear"), (0x40035, "Hellfire Spear"), (0x40036, "Primal Spear"), (0x40037, "Plasma Lance"), (0x40038, "Madlance"), (0x40039, "Gungnir"), (0x4003a, "Spirit Lance"), (0x4003b, "Ultimate Spear"), (0x50000, "Cypress Wand"), (0x50001, "Iron Rod"), (0x50002, "Earth Wand"), (0x50003, "Water Wand"), (0x50004, "Fire Wand"), (0x50005, "Air Wand"), (0x50006, "Electric Wand"), (0x50007, "Ebony Wand"), (0x50008, "Wand of Wisdom"), (0x50009, "Basho Wand"), (0x5000a, "Diabolic Wand"), (0x5000b, "Earth Rod"), (0x5000c, "Rod of the Sea"), (0x5000d, "Inferno Wand"), (0x5000e, "Cedar Wand"), (0x5000f, "Wand of Storms"), (0x50010, "Adian's Rod"), (0x50011, "Almighty Wand"), (0x50012, "Groovy Stick"), (0x50013, "Starstorm Wand"), (0x50014, "Spiral Wand"), (0x50015, "Silver Rod"), (0x50016, "Muddy Rod"), (0x50017, "Bubble Rod"), (0x50018, "Flaming Rod"), (0x50019, "Dark Green Rod"), (0x5001a, "Bolt Rod"), (0x5001b, "Nerd Staff"), (0x5001c, "Revelation"), (0x5001d, "Treasure Wand"), (0x5001e, "Witch's Wand"), (0x5001f, "Gaia's Staff"), (0x50020, "Ocean's Rod"), (0x50021, "Firedrake Bone"), (0x50022, "Rod of Gales"), (0x50023, "Stormlord Rod"), (0x50024, "Apocalypse Rod"), (0x50025, "Tenami"), (0x50026, "Moonstruck"), (0x50027, "Cosmic Staff"), (0x50028, "Oak Staff"), (0x50029, "Crystal Rod"), (0x5002a, "Earth Staff"), (0x5002b, "Cygnus Rod"), (0x5002c, "Burning Brand"), (0x5002d, "Hag's Wand"), (0x5002e, "Stun Rod"), (0x5002f, "Negation Wand"), (0x50030, "Wand of Truth"), (0x50031, "Dharma Wand"), (0x50032, "Witch's Stick"), (0x50033, "Ishtar Wand"), (0x50034, "Commandments"), (0x50035, "Megiddo Wand"), (0x50036, "Wand of Dreams"), (0x50037, "Awful Stun Rod"), (0x50038, "Master Asia"), (0x50039, "Shinto Dream"), (0x5003a, "Caduceus Rod"), (0x5003b, "Staff of Truth"), (0x60000, "Bandana"), (0x60001, "Steel Cap"), (0x60002, "Cougar Bandana"), (0x60003, "Raccoon Earcap"), (0x60004, "Newt Necklace"), (0x60005, "Scarab Earring"), (0x60006, "Thunder Torque"), (0x60007, "Bat Earrings"), (0x60008, "Gold Necklace"), (0x60009, "War Headband"), (0x6000a, "Imp Earrings"), (0x6000b, "Dragon Crest"), (0x6000c, "Fox Crest"), (0x6000d, "Wolf Crest"), (0x6000e, "Owl Crest"), (0x6000f, "Rat Crest"), (0x60010, "Raven Crest"), (0x60011, "Blade's Chain"), (0x60012, "Angel Chain"), (0x60013, "Golden Crown"), (0x60014, "Nomad's Hood"), (0x60015, "Guard Cap"), (0x60016, "Hunter's Hood"), (0x60017, "Ice Hunter Cap"), (0x60018, "Fire Dance Hat"), (0x60019, "Peasant's Cap"), (0x6001a, "Lightning Cap"), (0x6001b, "Darkness Hood"), (0x6001c, "Golden Turban"), (0x6001d, "Infantry Cap"), (0x6001e, "Demon's Cap"), (0x6001f, "Eagle Hood"), (0x60020, "Ice Tiger Hood"), (0x60021, "Fire Lion Hood"), (0x60022, "Cave Bear Hood"), (0x60023, "Giraffe Hood"), (0x60024, "Night Ape Hood"), (0x60025, "Matador Hood"), (0x60026, "Angel's Cap"), (0x60027, "Fallen Pope"), (0x60028, "Head Gear"), (0x60029, "Face Guard"), (0x6002a, "Mountain Helm"), (0x6002b, "Ice Helm"), (0x6002c, "Fire Helm"), (0x6002d, "Forester Helm"), (0x6002e, "Stormlord Helm"), (0x6002f, "Midnight Helm"), (0x60030, "Golden Helm"), (0x60031, "Samurai Helm"), (0x60032, "Devil Helm"), (0x60033, "Rock Dragon"), (0x60034, "Ice Dragon"), (0x60035, "Fire Dragon"), (0x60036, "Spirit Dragon"), (0x60037, "Thunder Dragon"), (0x60038, "Dark Dragon"), (0x60039, "Mercenary Mask"), (0x6003a, "Angel Helm"), (0x6003b, "Usurper Helm"), (0x70000, "Leather Coat"), (0x70001, "Noble Cloak"), (0x70002, "Hiking Gear"), (0x70003, "Winter Coat"), (0x70004, "Fireman's Coat"), (0x70005, "Lincoln Green"), (0x70006, "Thunder Cloak"), (0x70007, "Demon Cloak"), (0x70008, "Saint Cross"), (0x70009, "Blood Suit"), (0x7000a, "Priest's Stole"), (0x7000b, "Tribal Robes"), (0x7000c, "Flowing Robes"), (0x7000d, "Blazing Robes"), (0x7000e, "Airy Robes"), (0x7000f, "Thunder Robes"), (0x70010, "Magus Robes"), (0x70011, "Sublime Stole"), (0x70012, "Celestial Robe"), (0x70013, "Shadow Robes"), (0x70014, "Leather Armour"), (0x70015, "Ring Mail"), (0x70016, "Wyrm Hide"), (0x70017, "Wyrm Scale"), (0x70018, "Firedrake Mail"), (0x70019, "Holy Tree Mail"), (0x7001a, "Quakebeast Fur"), (0x7001b, "Bone Armour"), (0x7001c, "Linen Cuirass"), (0x7001d, "Bandit Mail"), (0x7001e, "Segmentart"), (0x7001f, "Jasper Hauberk"), (0x70020, "Frost Hauberk"), (0x70021, "Blaze Hauberk"), (0x70022, "Spirit Hauberk"), (0x70023, "Storm Hauberk"), (0x70024, "Ebony Hauberk"), (0x70025, "Killer Hauberk"), (0x70026, "Able Lamellar"), (0x70027, "Demon Mail"), (0x70028, "Brigandine"), (0x70029, "Plate Armour"), (0x7002a, "Grand Armour"), (0x7002b, "Frost Armour"), (0x7002c, "Blaze Armour"), (0x7002d, "Spirit Armour"), (0x7002e, "Thunder Armour"), (0x7002f, "Ebony Armour"), (0x70030, "Kris Armour"), (0x70031, "Masamune Armour"), (0x70032, "Armour of Hell"), (0x70033, "Earth Beast"), (0x70034, "Water Beast"), (0x70035, "Fire Beast"), (0x70036, "Tree Beast"), (0x70037, "Thunder Beast"), (0x70038, "Dark Beast"), (0x70039, "Paladin Mail"), (0x7003a, "Ragaraja Mail"), (0x7003b, "Omega Guard"), (0x80000, "Wrist Band"), (0x80001, "Silver Bracer"), (0x80002, "Fossil Bracer"), (0x80003, "Frost Bracer"), (0x80004, "Fire Bracer"), (0x80005, "Air Bracer"), (0x80006, "Storm Bracer"), (0x80007, "Jet Bracer"), (0x80008, "Protect Ring"), (0x80009, "Able Ring"), (0x8000a, "Geist Ring"), (0x8000b, "Rock Guard"), (0x8000c, "Briny Guard"), (0x8000d, "Charred Guard"), (0x8000e, "Hunting Guard"), (0x8000f, "Ion Guard"), (0x80010, "Shadow Guard"), (0x80011, "War God Guard"), (0x80012, "Sacred Guard"), (0x80013, "Uber Guard"), (0x80014, "Leather Gloves"), (0x80015, "Silver Gloves"), (0x80016, "Miner's Gloves"), (0x80017, "Fishing Gloves"), (0x80018, "Smith's Gloves"), (0x80019, "Forest Gloves"), (0x8001a, "Thunder Gloves"), (0x8001b, "Gloves of Dark"), (0x8001c, "Bouncer Gloves"), (0x8001d, "Able Gloves"), (0x8001e, "Drain Gloves"), (0x8001f, "Hammer Gloves"), (0x80020, "Ocean Gloves"), (0x80021, "Inferno Gloves"), (0x80022, "Sonic Gloves"), (0x80023, "Shield Gloves"), (0x80024, "Jet Gloves"), (0x80025, "Bladed Gloves"), (0x80026, "Divine Gloves"), (0x80027, "Hades Gloves"), (0x80028, "Rusted Hands"), (0x80029, "Silver Hands"), (0x8002a, "Hands of Earth"), (0x8002b, "Hands of Water"), (0x8002c, "Hands of Fire"), (0x8002d, "Hands of Wood"), (0x8002e, "Hands of Storm"), (0x8002f, "Hands of Ebony"), (0x80030, "Hands of Ronin"), (0x80031, "Able Hands"), (0x80032, "Cursed Hands"), (0x80033, "Gaia Hands"), (0x80034, "Aqua Hands"), (0x80035, "Fire Hands"), (0x80036, "Sprite Hands"), (0x80037, "Electric Hands"), (0x80038, "Midnight Hands"), (0x80039, "Master's Hands"), (0x8003a, "Divine Hands"), (0x8003b, "Demon's Hands"), (0x90000, "Sandals"), (0x90001, "Leg Mail"), (0x90002, "Ceramic Anklet"), (0x90003, "Frost Anklet"), (0x90004, "Iron Anklet"), (0x90005, "Oak Anklet"), (0x90006, "Thunder Anklet"), (0x90007, "Ninja Anklet"), (0x90008, "Greave Sticker"), (0x90009, "Ivory Greaves"), (0x9000a, "Devil Greaves"), (0x9000b, "Stone Greaves"), (0x9000c, "Sea Greaves"), (0x9000d, "Lava Greaves"), (0x9000e, "Oaken Greaves"), (0x9000f, "Storm Greaves"), (0x90010, "Ebony Greaves"), (0x90011, "Alert Greaves"), (0x90012, "Greaves of Awe"), (0x90013, "Denial Greaves"), (0x90014, "Safety Shoes"), (0x90015, "Jungle Boots"), (0x90016, "Mountain Boots"), (0x90017, "Snow Panther"), (0x90018, "Fire Lizard"), (0x90019, "Ranger's Boots"), (0x9001a, "Thunder Boots"), (0x9001b, "Ninja Socks"), (0x9001c, "Deluxe Boots"), (0x9001d, "Powered Boots"), (0x9001e, "Osorezan Socks"), (0x9001f, "Stone Solleret"), (0x90020, "Frost Solleret"), (0x90021, "Blaze Solleret"), (0x90022, "Oaken Solleret"), (0x90023, "Storm Solleret"), (0x90024, "Night Solleret"), (0x90025, "Guard Solleret"), (0x90026, "Magus Solleret"), (0x90027, "Ultra Solleret"), (0x90028, "Used Greaves"), (0x90029, "Leather Legs"), (0x9002a, "Mountain Guard"), (0x9002b, "Aqua Guard"), (0x9002c, "Flare Guard"), (0x9002d, "Green Guard"), (0x9002e, "Electric Guard"), (0x9002f, "Benkei's Guard"), (0x90030, "Spiked Guard"), (0x90031, "Rare Greaves"), (0x90032, "Magic Leg Mail"), (0x90033, "Imp's Leg Mail"), (0x90034, "Ice Leg Mail"), (0x90035, "Flame Leg Mail"), (0x90036, "Oaken Leg Mail"), (0x90037, "Storm Leg Mail"), (0x90038, "Ebony Leg Mail"), (0x90039, "Lone Leg Mail"), (0x9003a, "Alien Leg Mail"), (0x9003b, "Ultra Leg Mail"), (0xa0000, "Health Drink"), (0xa0001, "Healing Potion"), (0xa0002, "Healing Elixir"), (0xa0003, "Antidote"), (0xa0004, "Restorative"), (0xa0005, "Resurrect"), (0xa0006, "Warrior Blood"), (0xa0007, "Knight Blood"), (0xa0008, "Hunter Blood"), (0xa0009, "Hermit Blood"), (0xa000a, "Beast Blood"), (0xa000b, "Wizard Blood"), (0xa000c, "Well Water"), (0xa000d, "Pure Water"), (0xa000e, "Burning Oil"), (0xa000f, "Holy Sap"), (0xa0010, "Sports Drink"), (0xa0011, "Cooked Bile"), (0xa0012, "Mage's Soul"), (0xa0013, "Artisan's Soul"), (0xa0014, "Emperor's Soul"), (0xa0015, "Noble Wine"), (0xa0017, "Recovery Drink"), (0xb0000, "Raining Rocks"), (0xb0001, "Raging Earth"), (0xb0002, "Stone Storm"), (0xb0003, "Gaia's Spell"), (0xb0004, "Meteor Strike"), (0xb0005, "Cosmic Truth"), (0xb0006, "Ice Storm"), (0xb0007, "Ice Floe"), (0xb0008, "Ice Strike"), (0xb0009, "Cygnus"), (0xb000a, "Absolute Zero"), (0xb000b, "Permafrost"), (0xb000c, "Fire Tempest"), (0xb000d, "Meteor Swarm"), (0xb000e, "Flame Blast"), (0xb000f, "Fireball Storm"), (0xb0010, "Hellstorm"), (0xb0011, "Inferno Strike"), (0xb0012, "Green Gale"), (0xb0013, "Gale Breath"), (0xb0014, "Leafblight"), (0xb0015, "Wood Sprite"), (0xb0016, "Jungle Rage"), (0xb0017, "Forest of Fear"), (0xb0018, "Lightning Bolt"), (0xb0019, "Plasma Storm"), (0xb001a, "Ion Strike"), (0xb001b, "Raging Plasma"), (0xb001c, "Thunderbolt"), (0xb001d, "Plasma Gale"), (0xb001e, "Nightblight"), (0xb001f, "Dark Night"), (0xb0020, "Dark Traitor"), (0xb0021, "Chaos Spell"), (0xb0022, "Nightfear"), (0xb0023, "Nightshade"), (0xb0024, "The Death"), (0xb0025, "The Hanged Man"), (0xb0026, "The Lovers"), (0xb0027, "The Moon"), (0xb0028, "The Fool"), (0xb0029, "The Devil"), (0xb002a, "Warrior's Bane"), (0xb002b, "Knight's Bane"), (0xb002c, "Hunter's Bane"), (0xb002d, "Hermit's Bane"), (0xb002e, "Beast's Bane"), (0xb002f, "Wizard's Bane"), (0xb0030, "Stonebane"), (0xb0031, "Waterbane"), (0xb0032, "Firebane"), (0xb0033, "Treebane"), (0xb0034, "Lightbane"), (0xb0035, "Nightbane"), (0xb0036, "Health Charm"), (0xb0037, "Soul Charm"), (0xb0038, "Speed Charm"), (0xb0039, "Light Cross"), (0xb003a, "Hale Cross"), (0xb003b, "Divine Cross"), (0xb003c, "Summon Earth"), (0xb003d, "Summon Water"), (0xb003e, "Summon Fire"), (0xb003f, "Summon Wood"), (0xb0040, "Summon Thunder"), (0xb0041, "Summon Night"), (0xb0042, "Stonecall"), (0xb0043, "Aquacall"), (0xb0044, "Infernocall"), (0xb0045, "Greencall"), (0xb0046, "Thundercall"), (0xb0047, "Nightcall"), (0xd0000, "Fortune Wire"), (0xd0001, "Sprite Ocarina"), (0xd0002, "Fairy's Orb"), (0xe0000, "Grunt Doll"), (0xe0001, "Rainbow Card"), (0xe0002, "Yellow Candy"), (0xe0003, "Silver Scarab"), (0xe0004, "Ivory Barette"), (0xe0005, "Scarlet Plate"), (0xe0006, "Ruby Brooch"), (0xe0007, "Crystal Stone"), (0xe0008, "Golem Parts"), (0xe0009, "Vessel of Sky")];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_entry_is_updated() {
        let mut slot = vec![0u8; 0x8530];
        slot[..4].copy_from_slice(b"Kite");
        slot[PARTY] = 7;
        slot[0x8400] = 9;
        let mut sys = vec![0u8; 0x150];
        sys[0x1c] = 1;
        assert_eq!(system_after("X/dhdata01", &sys, &slot).unwrap(), sys);
        let out = system_after("X/dhdata02", &sys, &slot).unwrap();
        let sum = (b"Kite".iter().map(|b| *b as u32).sum::<u32>() + 7 + 9) as u16;
        assert_eq!(&out[0x1c + 0x16..0x1c + 0x18], &sum.to_le_bytes());
        assert_eq!(out[0x1c + 1], 7);
        assert_eq!(&out[0x1c + 4..0x1c + 8], b"Kite");
        assert_eq!(out[0x1c + 0x18], 9);
        assert_eq!(&out[..0x1c], &sys[..0x1c]);
    }
}

#[cfg(test)]
mod samples {
    /// `HACK_SAMPLES=<dir of .psu/.psv> cargo test --release --lib dothack::samples -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("HACK_SAMPLES") else { return };
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            let Ok(d) = std::fs::read(&p) else { continue };
            let Some((kind, files)) = crate::fmt::pssave::open(&d, &ext) else { continue };
            let Some(sys) = files.iter().find(|f| super::is_system_file(&f.name)) else { continue };
            for f in files.iter().filter(|f| super::is_slot_file(&f.name)) {
                // The stored entry already agrees with the slot file.
                let mut got = super::system_after(&f.name, &sys.data, &f.data).unwrap();
                // Bytes behind the copied name may be left-overs; compare the rest.
                let k = (super::slot_number(&f.name).unwrap() - 1) * 0x1c;
                got[k + 4..k + 0x12].copy_from_slice(&sys.data[k + 4..k + 0x12]);
                assert_eq!(got, sys.data, "{p:?} {}", f.name);
                if super::looks(&f.data) {
                    let mut doc = super::parse(&f.data).unwrap();
                    assert_eq!(doc.to_bytes().unwrap(), f.data);
                    // An edit keeps the system file in step.
                    let mut edited = f.data.clone();
                    edited[0x749c] = edited[0x749c].wrapping_add(1);
                    let out = crate::fmt::pssave::save(kind, &d, &f.name, &edited).unwrap();
                    let (_, again) = crate::fmt::pssave::open(&out, &ext).unwrap();
                    let sys2 = again.iter().find(|x| super::is_system_file(&x.name)).unwrap();
                    let slot2 = again.iter().find(|x| x.name == f.name).unwrap();
                    assert_eq!(slot2.data, edited);
                    assert_eq!(super::system_after(&f.name, &sys2.data, &edited).unwrap(), sys2.data);
                    assert_eq!(sys2.data.len(), sys.data.len());
                    assert_ne!(sys2.data, sys.data);
                    n += 1;
                }
            }
        }
        println!("{n} slots ok");
    }
}
