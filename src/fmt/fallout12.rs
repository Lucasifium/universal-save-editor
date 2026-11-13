//! Fallout and Fallout 2 `SAVE.DAT` (one per `SAVEGAME\SLOTnn` folder): big-endian, no
//! packing and no checksum. A 30,051-byte header (name, description, map, thumbnail), then
//! each engine subsystem's data in a fixed order: global variables, the visited maps, the
//! player object (hit points and the inventory, items nested in containers), the player's
//! critter data (S.P.E.C.I.A.L., skills), kill counts, tagged skills, perks, combat state,
//! (Fallout 2: party AI packets), level / experience / skill points, (Fallout 1: event
//! queue), traits.
//!
//! Some lengths aren't stored: the number of global variables, the extra bytes after each
//! item (0, 4 or 8, depending on the item's prototype in the game's data files) and, in
//! Fallout 2, how many party members have perks and AI packets. Each is worked out by
//! trying the possibilities and keeping the one where everything after it reads right; when
//! more than one fits, the inventory is shown but not editable.
//!
//! Layout and name tables after fallout-se (Ali Raheem, MIT OR Apache-2.0), with two
//! corrections checked against the game's character sheets: Fallout 2's critter data is one
//! int longer (376 bytes), and its trait 13 is Sex Appeal. Every value is changed in place.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use serde_json::Value;

const HEADER_LEN: usize = 30_051;
const SKILLS: usize = 18;

const SPECIAL: [&str; 7] = ["Strength", "Perception", "Endurance", "Charisma", "Intelligence", "Agility", "Luck"];

const SKILL_NAMES: [&str; SKILLS] = [
    "Small Guns", "Big Guns", "Energy Weapons", "Unarmed", "Melee Weapons", "Throwing", "First Aid", "Doctor", "Sneak", "Lockpick", "Steal", "Traps", "Science",
    "Repair", "Speech", "Barter", "Gambling", "Outdoorsman",
];

const TRAITS_F1: [&str; 16] = [
    "Fast Metabolism", "Bruiser", "Small Frame", "One Hander", "Finesse", "Kamikaze", "Heavy Handed", "Fast Shot", "Bloody Mess", "Jinxed", "Good Natured",
    "Chem Reliant", "Chem Resistant", "Night Person", "Skilled", "Gifted",
];

/// Fallout 2 replaced Night Person with Sex Appeal (checked on a character sheet).
const TRAITS_F2: [&str; 16] = [
    "Fast Metabolism", "Bruiser", "Small Frame", "One Hander", "Finesse", "Kamikaze", "Heavy Handed", "Fast Shot", "Bloody Mess", "Jinxed", "Good Natured",
    "Chem Reliant", "Chem Resistant", "Sex Appeal", "Skilled", "Gifted",
];

const KILLS_F1: [&str; 15] = [
    "Men", "Women", "Children", "Super Mutants", "Ghouls", "Brahmin", "Radscorpions", "Rats", "Floaters", "Centaurs", "Robots", "Dogs", "Mantises", "Deathclaws",
    "Plants",
];

const KILLS_F2: [&str; 19] = [
    "Men", "Women", "Children", "Super Mutants", "Ghouls", "Brahmin", "Radscorpions", "Rats", "Floaters", "Centaurs", "Robots", "Dogs", "Mantises", "Deathclaws",
    "Plants", "Geckos", "Aliens", "Giant Ants", "Big Bad Bosses",
];

const PERKS_F1: [&str; 64] = [
    "Awareness", "Bonus HtH Attacks", "Bonus HtH Damage", "Bonus Move", "Bonus Ranged Damage", "Bonus Rate of Fire", "Earlier Sequence", "Faster Healing",
    "More Criticals", "Night Vision", "Presence", "Rad Resistance", "Toughness", "Strong Back", "Sharpshooter", "Silent Running", "Survivalist", "Master Trader",
    "Educated", "Healer", "Fortune Finder", "Better Criticals", "Empathy", "Slayer", "Sniper", "Silent Death", "Action Boy", "Mental Block", "Lifegiver", "Dodger",
    "Snakeater", "Mr. Fixit", "Medic", "Master Thief", "Speaker", "Heave Ho!", "Friendly Foe", "Pickpocket", "Ghost", "Cult of Personality", "Scrounger",
    "Explorer", "Flower Child", "Pathfinder", "Animal Friend", "Scout", "Mysterious Stranger", "Ranger", "Quick Pockets", "Smooth Talker", "Swift Learner", "Tag!",
    "Mutate!", "Nuka-Cola Addiction", "Buffout Addiction", "Mentats Addiction", "Psycho Addiction", "Radaway Addiction", "Weapon Long Range", "Weapon Accurate",
    "Weapon Penetrate", "Weapon Knockback", "Powered Armor", "Combat Armor",
];

const PERKS_F2: [&str; 119] = [
    "Awareness", "Bonus HtH Attacks", "Bonus HtH Damage", "Bonus Move", "Bonus Ranged Damage", "Bonus Rate of Fire", "Earlier Sequence", "Faster Healing",
    "More Criticals", "Night Vision", "Presence", "Rad Resistance", "Toughness", "Strong Back", "Sharpshooter", "Silent Running", "Survivalist", "Master Trader",
    "Educated", "Healer", "Fortune Finder", "Better Criticals", "Empathy", "Slayer", "Sniper", "Silent Death", "Action Boy", "Mental Block", "Lifegiver", "Dodger",
    "Snakeater", "Mr. Fixit", "Medic", "Master Thief", "Speaker", "Heave Ho!", "Friendly Foe", "Pickpocket", "Ghost", "Cult of Personality", "Scrounger",
    "Explorer", "Flower Child", "Pathfinder", "Animal Friend", "Scout", "Mysterious Stranger", "Ranger", "Quick Pockets", "Smooth Talker", "Swift Learner", "Tag!",
    "Mutate!", "Nuka-Cola Addiction", "Buffout Addiction", "Mentats Addiction", "Psycho Addiction", "Radaway Addiction", "Weapon Long Range", "Weapon Accurate",
    "Weapon Penetrate", "Weapon Knockback", "Powered Armor", "Combat Armor", "Weapon Scope Range", "Weapon Fast Reload", "Weapon Night Sight", "Weapon Flameboy",
    "Armor Advanced I", "Armor Advanced II", "Jet Addiction", "Tragic Addiction", "Armor Charisma", "Gecko Skinning", "Dermal Impact Armor",
    "Dermal Impact Assault Enhancement", "Phoenix Armor Implants", "Phoenix Assault Enhancement", "Vault City Inoculations", "Adrenaline Rush", "Cautious Nature",
    "Comprehension", "Demolition Expert", "Gambler", "Gain Strength", "Gain Perception", "Gain Endurance", "Gain Charisma", "Gain Intelligence", "Gain Agility",
    "Gain Luck", "Harmless", "Here and Now", "HtH Evade", "Kama Sutra Master", "Karma Beacon", "Light Step", "Living Anatomy", "Magnetic Personality",
    "Negotiator", "Pack Rat", "Pyromaniac", "Quick Recovery", "Salesman", "Stonewall", "Thief", "Weapon Handling", "Vault City Training",
    "Alcohol Raised Hit Points", "Alcohol Raised Hit Points II", "Alcohol Lowered Hit Points", "Alcohol Lowered Hit Points II", "Autodoc Raised Hit Points",
    "Autodoc Raised Hit Points II", "Autodoc Lowered Hit Points", "Autodoc Lowered Hit Points II", "Expert Excrement Expeditor", "Weapon Enhanced Knockout",
    "Jinxed",
];

/// Item prototype numbers the engine itself refers to, with their names (fallout-se's
/// table, from the Fallout Community Edition sources). Other items show their number.
const ITEMS: &[(i32, &str)] = &[
    (3, "Power Armor"),
    (38, "Small Energy Cell"),
    (39, "Micro Fusion Cell"),
    (40, "Stimpak"),
    (41, "Bottle Caps"),
    (47, "First Aid Kit"),
    (48, "RadAway"),
    (51, "Dynamite"),
    (52, "Geiger Counter"),
    (53, "Mentats"),
    (54, "Stealth Boy"),
    (59, "Motion Sensor"),
    (73, "Big Book of Science"),
    (76, "Dean's Electronics"),
    (79, "Flare"),
    (80, "First Aid Book"),
    (85, "Plastic Explosives"),
    (86, "Scout Handbook"),
    (87, "Buffout"),
    (91, "Doctor's Bag"),
    (102, "Guns and Bullets"),
    (106, "Nuka-Cola"),
    (110, "Psycho"),
    (124, "Beer"),
    (125, "Booze"),
    (144, "Super Stimpak"),
    (159, "Molotov Cocktail"),
    (205, "Flare (lit)"),
    (206, "Dynamite (armed)"),
    (207, "Geiger Counter (on)"),
    (209, "Plastic Explosives (armed)"),
    (210, "Stealth Boy (on)"),
    (232, "Hardened Power Armor"),
];

const ITEMS_F2: &[(i32, &str)] = &[
    (259, "Jet"),
    (260, "Jet Antidote"),
    (273, "Healing Powder"),
    (304, "Deck of Tragic Cards"),
    (331, "Cat's Paw Issue #5"),
    (348, "Advanced Power Armor"),
    (349, "Advanced Power Armor Mk II"),
    (383, "Shiv"),
    (390, "Solar Scorcher"),
    (399, "Super Cattle Prod"),
    (407, "Mega Power Fist"),
    (408, "Field Medic First Aid Kit"),
    (409, "Paramedic's Bag"),
    (433, "Mirrored Shades"),
    (499, "PIPBoy Lingual Enhancer"),
    (516, "PIPBoy Medical Enhancer"),
];

pub const CAPS_PID: i32 = 41;

pub fn item_name(f2: bool, pid: i32) -> Option<&'static str> {
    let idx = pid & 0x00FF_FFFF;
    ITEMS.iter().chain(if f2 { ITEMS_F2 } else { &[] }).find(|(p, _)| *p == idx).map(|x| x.1)
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn i16_at(d: &[u8], at: usize) -> Option<i16> {
    Some(i16::from_be_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn text(d: &[u8], at: usize, len: usize) -> String {
    d.get(at..at + len).unwrap_or_default().iter().take_while(|&&c| c != 0).map(|&c| super::bethesda_cp1252(c)).collect()
}

/// 1 for Fallout, 2 for Fallout 2.
fn version(d: &[u8]) -> Option<u8> {
    if d.len() < HEADER_LEN + 64 || !d.starts_with(b"FALLOUT SAVE FILE") || i16_at(d, 24)? != 1 || d[28] != b'R' {
        return None;
    }
    match i16_at(d, 26)? {
        1 => Some(1),
        2 => Some(2),
        _ => None,
    }
}

pub fn looks(d: &[u8]) -> bool {
    version(d).is_some()
}

/// One inventory entry: where its count is, its prototype number, and the item's extra
/// bytes (8 for weapons: rounds loaded and the ammo's prototype).
#[derive(Clone, Debug)]
struct Item {
    qty_at: usize,
    pid: i32,
    extra_at: usize,
    extra: usize,
    depth: usize,
}

/// Visited-map list (handler 3) at `at`: (count, end) when it reads as one.
fn map_list(d: &[u8], at: usize) -> Option<(Vec<String>, usize)> {
    let n = i32_at(d, at)?;
    if !(1..=512).contains(&n) {
        return None;
    }
    let mut q = at + 4;
    let mut names = vec![];
    for _ in 0..n {
        let s = d.get(q..(q + 17).min(d.len()))?;
        let nul = s.iter().position(|&c| c == 0)?;
        let name = &s[..nul];
        if name.len() < 5 || !name.is_ascii() || !name.to_ascii_uppercase().ends_with(b".SAV") {
            return None;
        }
        names.push(String::from_utf8_lossy(name).into_owned());
        q += nul + 1;
    }
    let automap = i32_at(d, q)?;
    if !(0..=200_000_000).contains(&automap) {
        return None;
    }
    Some((names, q + 4))
}

struct Walk<'a> {
    d: &'a [u8],
    steps: usize,
    stack: Vec<Item>,
    found: Vec<(Vec<Item>, usize)>,
    proto_len: usize,
}

const MAX_STEPS: usize = 400_000;

impl Walk<'_> {
    /// Does an item object start at `at` (its 18-int header and inventory header)?
    fn item_header(&self, at: usize) -> Option<(i32, i32)> {
        let d = self.d;
        let (tile, pid, cid, inv) = (i32_at(d, at + 4)?, i32_at(d, at + 44)?, i32_at(d, at + 48)?, i32_at(d, at + 72)?);
        let ok = (tile == -1 || cid == -1) && (pid >> 24) == 0 && (1..0x10000).contains(&(pid & 0xFF_FFFF)) && (-1..1000).contains(&inv);
        ok.then_some((pid, inv.max(0)))
    }

    /// `n` (count, item) entries from `at`; `k` gets where they end. Returns true to stop.
    fn items(&mut self, at: usize, n: usize, depth: usize, k: &mut dyn FnMut(&mut Self, usize) -> bool) -> bool {
        if n == 0 {
            return k(self, at);
        }
        self.steps += 1;
        if self.steps > MAX_STEPS || depth > 4 {
            return self.steps > MAX_STEPS;
        }
        let Some(qty) = i32_at(self.d, at) else { return false };
        if qty < 1 {
            return false;
        }
        let Some((pid, inner)) = self.item_header(at + 4) else { return false };
        let data = at + 4 + 84 + 4; // header, inventory header, item flags
        for extra in [0usize, 4, 8] {
            self.stack.push(Item { qty_at: at, pid, extra_at: data, extra, depth });
            let stop = self.items(data + extra, inner as usize, depth + 1, &mut |w: &mut Self, end: usize| w.items(end, n - 1, depth, &mut *k));
            self.stack.pop();
            if stop {
                return true;
            }
        }
        false
    }

    /// The critter data (handler 6), kill counts and tagged skills read right at `at`.
    fn proto_ok(&self, at: usize, kills: usize) -> bool {
        let d = self.d;
        let Some(sneak) = i32_at(d, at) else { return false };
        let stat = |i: usize| i32_at(d, at + 8 + 4 * i);
        let special = (0..7).all(|i| stat(i).is_some_and(|v| (1..=10).contains(&v)));
        let age = stat(33).is_some_and(|v| (1..=150).contains(&v));
        let gender = stat(34).is_some_and(|v| v == 0 || v == 1);
        let skills = (0..SKILLS).all(|i| i32_at(d, at + 288 + 4 * i).is_some_and(|v| (0..=1000).contains(&v)));
        let k = at + self.proto_len;
        let kills_ok = (0..kills).all(|i| i32_at(d, k + 4 * i).is_some_and(|v| v >= 0));
        let tags_ok = (0..4).all(|i| i32_at(d, k + 4 * kills + 4 * i).is_some_and(|v| (-1..SKILLS as i32).contains(&v)));
        (sneak == 0 || sneak == 1) && special && age && gender && skills && kills_ok && tags_ok
    }
}

struct Layout {
    f2: bool,
    globals: usize,
    maps: Vec<String>,
    player: usize,
    items: Vec<Item>,
    /// More than one way to read the inventory fits.
    ambiguous: bool,
    proto: usize,
    kills: usize,
    tags: usize,
    perks: usize,
    pc: usize,
    traits: usize,
}

/// Fallout 1's event queue (handler 15): count, then (time, type, object) and a payload
/// whose size depends on the type. Returns its end.
fn event_queue(d: &[u8], at: usize) -> Option<usize> {
    let n = i32_at(d, at)?;
    if !(0..=10_000).contains(&n) {
        return None;
    }
    let mut q = at + 4;
    for _ in 0..n {
        let ty = i32_at(d, q + 4)?;
        q += 12
            + match ty {
                0 => 24,
                2 => 12,
                3 | 6 => 8,
                1 | 4 | 5 | 7..=12 => 0,
                _ => return None,
            };
    }
    (q <= d.len()).then_some(q)
}

/// The combat block (handler 11) at `at`: flags, and when in combat the turn data, the
/// combatants and (`ai`) four ints of AI state per combatant. Returns its end.
fn combat(d: &[u8], at: usize, ai: bool) -> Option<usize> {
    let flags = i32_at(d, at)?;
    if flags & 1 == 0 {
        return Some(at + 4);
    }
    let (com, noncom, total) = (i32_at(d, at + 16)?, i32_at(d, at + 20)?, i32_at(d, at + 24)?);
    if com < 0 || noncom < 0 || com + noncom != total || total > 500 {
        return None;
    }
    let end = at + 32 + 4 * total as usize * if ai { 5 } else { 1 };
    (end <= d.len()).then_some(end)
}

fn traits_ok(d: &[u8], at: usize) -> bool {
    let (Some(a), Some(b)) = (i32_at(d, at), i32_at(d, at + 4)) else { return false };
    let ok = |t: i32| (-1..16).contains(&t);
    ok(a) && ok(b) && (a == -1 || a != b)
}

/// Level, experience and unspent skill points (handler 13) look right at `at`.
fn pc_ok(d: &[u8], at: usize) -> bool {
    let (Some(points), Some(level), Some(exp)) = (i32_at(d, at), i32_at(d, at + 4), i32_at(d, at + 8)) else { return false };
    (0..=1_000_000).contains(&points) && (1..=99).contains(&level) && exp >= 0
}

fn layout(d: &[u8]) -> Result<Layout, String> {
    let f2 = version(d).ok_or("not a Fallout SAVE.DAT")? == 2;
    let start = HEADER_LEN + 4;
    let extra = usize::from(!f2); // Fallout 1 adds a byte after the globals
    let (globals, maps, after_maps) = (1..5000)
        .find_map(|n| {
            let (maps, end) = map_list(d, start + 4 * n + extra)?;
            // The globals are written twice; the copy must match.
            let len = 4 * n + extra;
            (d.get(end..end + len)? == d.get(start..start + len)?).then_some((n, maps, end + len))
        })
        .ok_or("couldn't find the global variables")?;
    let player = after_maps;
    let d_ = d;
    let (pid, inv_len) = (i32_at(d, player + 44).ok_or("short file")?, i32_at(d, player + 72).ok_or("short file")?);
    if pid != 0x0100_0000 || !(0..1000).contains(&inv_len) {
        return Err("the player object isn't where it should be".into());
    }
    let proto_len = if f2 { 376 } else { 372 };
    let kill_n = if f2 { 19 } else { 15 };
    let mut w = Walk { d: d_, steps: 0, stack: vec![], found: vec![], proto_len };
    let first = player + 84 + 44; // object header, inventory header, critter data
    w.items(first, inv_len as usize, 0, &mut |w: &mut Walk, end: usize| {
        if w.proto_ok(end + 4, kill_n) {
            w.found.push((w.stack.clone(), end + 4));
        }
        w.found.len() > 1
    });
    if w.found.is_empty() {
        return Err(if w.steps > MAX_STEPS { "the inventory has too many possible readings" } else { "couldn't read the inventory" }.into());
    }
    let ambiguous = w.found.len() > 1;
    let (items, proto) = w.found.swap_remove(0);
    let kills = proto + proto_len;
    let tags = kills + 4 * kill_n;
    let perks = tags + 16;
    let (pc, traits) = if f2 {
        // Perks of every party member (119 each), combat state, AI packets (45 ints each)
        // for party members; then the player's level etc., traits, automap flags and the
        // preferences, whose first values have small ranges.
        let prefs_ok = |t: usize| {
            let r = |i: usize, hi: i32| i32_at(d, t + 12 + 4 * i).is_some_and(|v| (0..=hi).contains(&v));
            r(0, 2) && r(1, 2) && r(2, 3) && r(3, 2) && r(4, 1)
        };
        let mut hits = vec![];
        for party in 1..=64usize {
            let combat_at = perks + 476 * party;
            if !(perks..combat_at).step_by(4).all(|q| i32_at(d, q).is_some_and(|v| (0..=20).contains(&v))) {
                break;
            }
            let Some(after) = combat(d, combat_at, true) else { continue };
            for ai in 0..=party {
                let pc = after + 180 * ai;
                if pc_ok(d, pc) && traits_ok(d, pc + 20) && prefs_ok(pc + 20) {
                    hits.push(pc);
                }
            }
        }
        hits.dedup();
        match hits.as_slice() {
            [pc] => (*pc, *pc + 20),
            [] => return Err("couldn't find the level and experience".into()),
            _ => return Err("the level and experience could be in more than one place".into()),
        }
    } else {
        let combat_at = perks + 4 * 64;
        let mut found = None;
        for ai in [false, true] {
            let Some(pc) = combat(d, combat_at, ai) else { continue };
            if !pc_ok(d, pc) {
                continue;
            }
            let Some(t) = event_queue(d, pc + 20) else { continue };
            if traits_ok(d, t) {
                found = Some((pc, t));
                break;
            }
        }
        found.ok_or("couldn't find the level and experience")?
    };
    Ok(Layout { f2, globals, maps, player, items, ambiguous, proto, kills, tags, perks, pc, traits })
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let l = layout(d)?;
    let f2 = l.f2;
    let mut b = Builder::new(d);
    b.big_endian = true;
    let game = if f2 { "Fallout 2" } else { "Fallout" };
    b.info(&["Save", "Game"], game);
    b.info(&["Save", "Character name"], text(d, 29, 32));
    b.info(&["Save", "Description"], text(d, 61, 30));
    b.info(&["Save", "Map"], text(d, 115, 16));
    let (day, month, year) = (i16_at(d, 91).unwrap_or(0), i16_at(d, 93).unwrap_or(0), i16_at(d, 95).unwrap_or(0));
    b.info(&["Save", "Saved on"], format!("{year:04}-{month:02}-{day:02}"));
    let (gm, gd, gy) = (i16_at(d, 101).unwrap_or(0), i16_at(d, 103).unwrap_or(0), i16_at(d, 105).unwrap_or(0));
    b.info(&["Save", "Game date"], format!("{gy:04}-{gm:02}-{gd:02}"));
    b.info(&["Save", "Maps visited"], l.maps.len() as u64);
    b.info(&["Save", "Global variables"], l.globals as u64);

    // Player object: hit points in its critter data.
    let crit = l.player + 84;
    b.ranged(&["Character", "Level"], l.pc + 4, Kind::I32, 1, 99);
    b.ranged(&["Character", "Experience"], l.pc + 8, Kind::I32, 0, i32::MAX as i64);
    b.ranged(&["Character", "Skill points"], l.pc, Kind::I32, 0, 9999);
    b.ranged(&["Character", "Hit points"], crit + 32, Kind::I32, 1, 999);
    let stat = |i: usize| l.proto + 8 + 4 * i;
    let bonus = |i: usize| i32_at(d, l.proto + 148 + 4 * i).unwrap_or(0);
    b.ranged(&["Character", "Max hit points (base)"], stat(7), Kind::I32, 1, 999);
    b.info(&["Character", "Max hit points"], i64::from(i32_at(d, stat(7)).unwrap_or(0) + bonus(7)));
    b.ranged(&["Character", "Radiation"], crit + 36, Kind::I32, 0, 2000);
    b.ranged(&["Character", "Poison"], crit + 40, Kind::I32, 0, 2000);
    b.info(&["Character", "Age"], i64::from(i32_at(d, stat(33)).unwrap_or(0)));
    b.info(&["Character", "Gender"], if i32_at(d, stat(34)) == Some(1) { "Female" } else { "Male" });

    for (i, name) in SPECIAL.iter().enumerate() {
        b.ranged(&["S.P.E.C.I.A.L.", name], stat(i), Kind::I32, 1, 10);
        b.info(&["S.P.E.C.I.A.L. bonus", name], i64::from(bonus(i)));
    }
    for (i, name) in SKILL_NAMES.iter().enumerate() {
        b.ranged(&["Skills", name], l.proto + 288 + 4 * i, Kind::I32, 0, 300);
    }
    // Tags, traits and perks are looked up by the game whenever it needs them, so they can be changed in place.
    let none = ("(none)".to_string(), u64::from(u32::MAX));
    let skill_opts: Vec<(String, u64)> = std::iter::once(none.clone()).chain(SKILL_NAMES.iter().enumerate().map(|(i, n)| (n.to_string(), i as u64))).collect();
    for i in 0..4 {
        b.pick(&["Tagged skill slots", &format!("Tag {}", i + 1)], l.tags + 4 * i, Kind::U32, &skill_opts);
    }
    let trait_names: &[&str] = if f2 { &TRAITS_F2 } else { &TRAITS_F1 };
    let trait_opts: Vec<(String, u64)> = std::iter::once(none).chain(trait_names.iter().enumerate().map(|(i, n)| (n.to_string(), i as u64))).collect();
    for i in 0..2 {
        b.pick(&["Trait slots", &format!("Trait {}", i + 1)], l.traits + 4 * i, Kind::U32, &trait_opts);
    }
    let perk_names: &[&str] = if f2 { &PERKS_F2 } else { &PERKS_F1 };
    for (i, name) in perk_names.iter().enumerate() {
        if i32_at(d, l.perks + 4 * i).is_some_and(|r| (0..=3).contains(&r)) {
            b.ranged(&["Perk ranks", name], l.perks + 4 * i, Kind::I32, 0, 3);
        }
    }
    let kill_names: &[&str] = if f2 { &KILLS_F2 } else { &KILLS_F1 };
    for (i, name) in kill_names.iter().enumerate() {
        b.ranged(&["Kills", name], l.kills + 4 * i, Kind::I32, 0, 999_999);
    }

    // Inventory: one entry per stack, items inside containers after their container.
    let mut caps: Option<String> = None;
    let width = l.items.len().to_string().len().max(2);
    for (n, it) in l.items.iter().enumerate() {
        let idx = it.pid & 0xFF_FFFF;
        let name = item_name(f2, it.pid).map(str::to_string).unwrap_or_else(|| format!("Item #{idx}"));
        let label = format!("{:0width$} {}{name}", n + 1, if it.depth > 0 { "\u{203A} " } else { "" });
        let qty = i32_at(d, it.qty_at).unwrap_or(0);
        if l.ambiguous {
            b.info(&["Inventory", &label, "Count"], i64::from(qty));
        } else {
            b.ranged(&["Inventory", &label, "Count"], it.qty_at, Kind::I32, 1, 99_999_999);
            if idx == CAPS_PID && it.depth == 0 && caps.is_none() {
                caps = Some(format!("/Inventory/{}/Count", crate::tree::esc(&label)));
            }
        }
        b.info(&["Inventory", &label, "Item number"], i64::from(idx));
        if it.extra == 8 {
            let ammo = i32_at(d, it.extra_at + 4).unwrap_or(-1);
            if l.ambiguous {
                b.info(&["Inventory", &label, "Loaded rounds"], i64::from(i32_at(d, it.extra_at).unwrap_or(0)));
            } else {
                b.ranged(&["Inventory", &label, "Loaded rounds"], it.extra_at, Kind::I32, 0, 999);
            }
            if ammo > 0 {
                let an = item_name(f2, ammo).map(str::to_string).unwrap_or_else(|| format!("Item #{}", ammo & 0xFF_FFFF));
                b.info(&["Inventory", &label, "Loaded with"], an);
            }
        }
    }
    if l.items.is_empty() {
        b.info(&["Inventory"], Value::Object(Default::default()));
    }
    let kind = if f2 { "Fallout 2 save" } else { "Fallout 1 save" };
    let mut doc = b.finish(kind, Checksum::None);
    // Bottle caps: the caps stack, under a second name for the money tile (the same leaf,
    // so an edit to either shows in both).
    if let Some(p) = caps {
        if let (Some(&leaf), Some(v)) = (doc.editable.get(&p), doc.view.pointer(&p).cloned()) {
            doc.editable.insert("/Money/Bottle caps".into(), leaf);
            if let Some(o) = doc.view.as_object_mut() {
                o.insert("Money".into(), serde_json::json!({ "Bottle caps": v }));
            }
        }
    }
    let mut note = format!(
        "{game} character from SAVE.DAT. Hit points, level, experience, skill points, S.P.E.C.I.A.L., skills, kills and inventory counts are changed in place."
    );
    if l.ambiguous {
        note.push_str(" The inventory could be read in more than one way, so its counts are shown but can't be changed.");
    }
    Ok(doc.with_note(note))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn put(d: &mut Vec<u8>, v: i32) {
        d.extend(v.to_be_bytes());
    }

    /// An inventory item object (18-int header, inventory header, flags, extra ints).
    fn item(d: &mut Vec<u8>, qty: i32, pid: i32, extra: &[i32]) {
        put(d, qty);
        for (i, v) in [7, -1, 0, 0, 0, 0, 0, 0, 0x0700_0000, 0, 0, pid, -1, 0, 0, 0, -1, -1].iter().enumerate() {
            let _ = i;
            put(d, *v);
        }
        for v in [0, 0, 0, 0] {
            put(d, v);
        }
        for &v in extra {
            put(d, v);
        }
    }

    /// A synthetic SAVE.DAT for Fallout (`f2` false) or Fallout 2.
    pub(crate) fn sample(f2: bool) -> Vec<u8> {
        let mut d = b"FALLOUT SAVE FILE".to_vec();
        d.resize(24, 0);
        d.extend(1i16.to_be_bytes());
        d.extend((if f2 { 2i16 } else { 1 }).to_be_bytes());
        d.push(b'R');
        let mut name = b"Tester".to_vec();
        name.resize(32, 0);
        d.extend(name);
        let mut desc = b"Synthetic".to_vec();
        desc.resize(30, 0);
        d.extend(desc);
        for v in [6i16, 11, 2024] {
            d.extend(v.to_be_bytes());
        }
        put(&mut d, 0);
        for v in [7i16, 25, 2241] {
            d.extend(v.to_be_bytes());
        }
        put(&mut d, 0);
        d.extend([0, 0, 0, 1]);
        let mut map = b"ARTEMPLE.SAV".to_vec();
        map.resize(16, 0);
        d.extend(map);
        d.resize(HEADER_LEN, 0);
        put(&mut d, 4); // player combat id
        let globals: Vec<i32> = (0..50).map(|i| i % 3).collect();
        let write_globals = |d: &mut Vec<u8>| {
            for &g in &globals {
                put(d, g);
            }
            if !f2 {
                d.push(0);
            }
        };
        write_globals(&mut d);
        put(&mut d, 2);
        d.extend(b"ARTEMPLE.SAV\0ARVILLAG.SAV\0");
        put(&mut d, 1234); // automap size
        write_globals(&mut d);
        // Player object.
        for v in [1, 20000, 0, 0, 0, 0, 0, 3, 0x0100_003E, 0x6000_0000u32 as i32, 0, 0x0100_0000, 1, 4, 0x10000, 0, -1, -1] {
            put(&mut d, v);
        }
        for v in [3, 3, 0] {
            put(&mut d, v);
        }
        for v in [0, 0, 0, 8, 0, 0, 1, -1, 33, 5, 0] {
            put(&mut d, v); // critter data: … hit points 33, radiation 5, poison 0
        }
        item(&mut d, 250, 41, &[0]); // caps (an ammo-like 4 extra bytes)
        item(&mut d, 1, 8, &[6, 38]); // a weapon with 6 rounds of small energy cells
        item(&mut d, 3, 40, &[]); // stimpaks
        put(&mut d, 20000); // centre tile
        // Critter data.
        put(&mut d, 0);
        put(&mut d, 0);
        let mut base = [0i32; 35];
        base[..7].copy_from_slice(&[6, 7, 5, 4, 8, 6, 5]);
        base[7] = 30;
        base[33] = 25;
        base[34] = 1;
        for v in base {
            put(&mut d, v);
        }
        let mut bonus = [0i32; 35];
        bonus[0] = 1;
        for v in bonus {
            put(&mut d, v);
        }
        for i in 0..SKILLS as i32 {
            put(&mut d, i % 4);
        }
        for v in [0, 0, 0] {
            put(&mut d, v);
        }
        if f2 {
            put(&mut d, 0);
        }
        for i in 0..if f2 { 19 } else { 15 } {
            put(&mut d, i);
        }
        for v in [0, 14, 6, -1] {
            put(&mut d, v);
        }
        let party = if f2 { 2 } else { 1 };
        let perks = if f2 { 119 } else { 64 };
        for p in 0..party {
            for i in 0..perks {
                put(&mut d, i32::from(p == 0 && (i == 3 || i == 26)));
            }
        }
        put(&mut d, 2); // not in combat
        if f2 {
            for i in 0..45 {
                put(&mut d, 1000 + i);
            }
        }
        for v in [7, 3, 3500, 0, 0] {
            put(&mut d, v); // skill points, level, experience
        }
        if !f2 {
            put(&mut d, 1); // one queued event
            for v in [100, 0, 1, 1, 2, 3, 4, 5, 6] {
                put(&mut d, v);
            }
        }
        for v in [15, 2] {
            put(&mut d, v); // traits
        }
        put(&mut d, 0);
        for v in [1, 1, 3, 2, 0, 1, 1, 0, 1] {
            put(&mut d, v);
        }
        d.extend([0u8; 64]);
        d
    }

    #[test]
    fn reads_and_edits_both_games() {
        for f2 in [false, true] {
            let d = sample(f2);
            assert!(looks(&d));
            let mut doc = parse(&d).unwrap();
            let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
            assert_eq!(doc.kind_name, if f2 { "Fallout 2 save" } else { "Fallout 1 save" });
            assert_eq!(v(&doc, "/Character/Level"), Some(3.into()));
            assert_eq!(v(&doc, "/Character/Experience"), Some(3500.into()));
            assert_eq!(v(&doc, "/Character/Skill points"), Some(7.into()));
            assert_eq!(v(&doc, "/Character/Hit points"), Some(33.into()));
            assert_eq!(v(&doc, "/Character/Gender"), Some("Female".into()));
            assert_eq!(v(&doc, "/S.P.E.C.I.A.L./Intelligence"), Some(8.into()));
            assert_eq!(v(&doc, "/S.P.E.C.I.A.L. bonus/Strength"), Some(1.into()));
            assert_eq!(v(&doc, "/Skills/Steal"), Some(2.into()));
            assert_eq!(v(&doc, "/Tagged skill slots/Tag 1"), Some("Small Guns".into()));
            assert_eq!(v(&doc, "/Tagged skill slots/Tag 3"), Some("First Aid".into()));
            assert_eq!(v(&doc, "/Tagged skill slots/Tag 4"), Some("(none)".into()));
            assert_eq!(v(&doc, "/Trait slots/Trait 2"), Some("Small Frame".into()));
            assert_eq!(v(&doc, "/Perk ranks/Action Boy"), Some(1.into()));
            assert_eq!(v(&doc, "/Kills/Rats"), Some(7.into()));
            assert_eq!(v(&doc, "/Money/Bottle caps"), Some(250.into()));
            assert_eq!(v(&doc, "/Inventory/01 Bottle Caps/Count"), Some(250.into()));
            assert_eq!(v(&doc, "/Inventory/02 Item #8/Loaded rounds"), Some(6.into()));
            assert_eq!(v(&doc, "/Inventory/02 Item #8/Loaded with"), Some("Small Energy Cell".into()));
            assert_eq!(v(&doc, "/Inventory/03 Stimpak/Count"), Some(3.into()));
            let mut e = parse(&d).unwrap();
            e.set("/Trait slots/Trait 2", "Finesse".into()).unwrap();
            e.set("/Perk ranks/Awareness", 1.into()).unwrap();
            let again = parse(&e.to_bytes().unwrap()).unwrap();
            assert_eq!(v(&again, "/Trait slots/Trait 2"), Some("Finesse".into()));
            assert_eq!(v(&again, "/Perk ranks/Awareness"), Some(1.into()));
            assert_eq!(doc.to_bytes().unwrap(), d, "no edit, same bytes");
            doc.set("/Money/Bottle caps", 251.into()).unwrap();
            assert_eq!(v(&doc, "/Inventory/01 Bottle Caps/Count"), Some(251.into()));
            doc.set("/S.P.E.C.I.A.L./Luck", 10.into()).unwrap();
            assert!(doc.set("/S.P.E.C.I.A.L./Luck", 11.into()).is_err());
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            let back = parse(&out).unwrap();
            assert_eq!(v(&back, "/Money/Bottle caps"), Some(251.into()));
            assert_eq!(v(&back, "/S.P.E.C.I.A.L./Luck"), Some(10.into()));
            assert_eq!(v(&back, "/Character/Experience"), Some(3500.into()));
            assert_eq!(out.iter().zip(&d).filter(|(a, b)| a != b).count(), 2);
        }
    }

    /// Research aid: FALLOUT_SAVES = SAVE.DAT files separated by `|` (or a .txt file listing
    /// them). Each must read, write back unchanged, and take an edit (caps +1, Luck) that
    /// reads back with nothing else moved.
    #[test]
    #[ignore]
    fn real_saves() {
        let arg = std::env::var("FALLOUT_SAVES").unwrap();
        let list = if arg.ends_with(".txt") { std::fs::read_to_string(&arg).unwrap().replace('\n', "|") } else { arg };
        for path in list.split('|').map(str::trim).filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            let mut doc = match parse(&d) {
                Ok(doc) => doc,
                Err(e) => panic!("{path}: {e}"),
            };
            let l = layout(&d).unwrap();
            let get = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
            println!(
                "{path}\n  {} | lvl {} exp {} sp {} hp {} | SPECIAL {:?} | caps {} | items {} amb {} | traits {} | tags {}",
                get(&doc, "/Save/Character name"),
                get(&doc, "/Character/Level"),
                get(&doc, "/Character/Experience"),
                get(&doc, "/Character/Skill points"),
                get(&doc, "/Character/Hit points"),
                SPECIAL.iter().map(|s| get(&doc, &format!("/S.P.E.C.I.A.L./{s}"))).collect::<Vec<_>>(),
                get(&doc, "/Money/Bottle caps"),
                l.items.len(),
                l.ambiguous,
                get(&doc, "/Trait slots"),
                get(&doc, "/Tagged skill slots"),
            );
            assert_eq!(doc.to_bytes().unwrap(), d);
            let before = doc.view.clone();
            if let Some(c) = get(&doc, "/Money/Bottle caps").as_i64() {
                doc.set("/Money/Bottle caps", (c + 1).into()).unwrap();
            }
            let luck = get(&doc, "/S.P.E.C.I.A.L./Luck").as_i64().unwrap();
            doc.set("/S.P.E.C.I.A.L./Luck", (if luck == 10 { 9 } else { luck + 1 }).into()).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            let back = parse(&out).unwrap();
            assert_eq!(back.view, doc.view, "{path}: edits read back, nothing else moved");
            assert_ne!(back.view, before);
        }
    }
}
