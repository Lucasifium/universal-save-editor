//! Half-Life saves: GoldSrc (Half-Life, Opposing Force, Blue Shift; `JSAV` version 0x71),
//! Source (Half-Life 2, Episode One and Two, Lost Coast, Half-Life: Source; version 0x73;
//! Black Mesa: 0x765) and Source 2 (Half-Life: Alyx; `S2SV` version 0x74, the same records
//! with a longer map-state header).
//!
//! A `.sav` is a container: `JSAV`, version, header size, token count, token bytes, the
//! token table, the game header (records: map name, comment…), then one entry per visited
//! map (`<map>.hl1` entity state, `.hl2`, `.hl3`; 260-byte name, length, data). Newer
//! Source builds put the number of entries before them. The live player is in the `.hl1`
//! of the map named in the header; the other maps keep stale copies.
//!
//! Everything inside is "records": u16 size, u16 token (an index into the file's token
//! table, which holds the field names), then the value. Values equal to zero are left out.
//! An entity table (`ETABLE` records: id, location, size, classname…) says where each
//! entity's records are; ammo is an int[32] on the player indexed by ammo type, and each
//! weapon entity says which ammo types it uses, so ammo names come from the weapons.
//!
//! Layout from VolvoWrench's SaveParser (WTFPL), facts from the Half-Life and Source SDKs
//! (field names, weapon ammo; facts only). No compression or checksum: values are edited
//! in place.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::BTreeMap;

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn cstr(b: &[u8]) -> String {
    b.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Engine {
    GoldSrc,
    Source,
    /// Half-Life: Alyx (`S2SV` version 0x74).
    Source2,
}

/// One record: its field name (token index), where its value starts and how long it is.
#[derive(Clone, Copy, Debug)]
struct Rec {
    tok: u16,
    at: usize,
    len: usize,
}

/// The records from `start` to `end` (stops at the first one that doesn't fit).
fn records(d: &[u8], start: usize, end: usize) -> Vec<Rec> {
    let mut out = vec![];
    let end = end.min(d.len());
    let mut o = start;
    while o + 4 <= end {
        let len = u16::from_le_bytes([d[o], d[o + 1]]) as usize;
        let tok = u16::from_le_bytes([d[o + 2], d[o + 3]]);
        if o + 4 + len > end {
            break;
        }
        out.push(Rec { tok, at: o + 4, len });
        o += 4 + len;
    }
    out
}

/// A token table: `count` NUL-terminated names (empty slots are a lone NUL).
fn tokens(d: &[u8], at: usize, bytes: usize, count: usize) -> Option<Vec<String>> {
    let t = d.get(at..at.checked_add(bytes)?)?;
    Some(t.split(|&c| c == 0).take(count).map(|s| s.iter().map(|&c| c as char).collect()).collect())
}

struct Names(Vec<String>);

impl Names {
    fn of(&self, r: &Rec) -> &str {
        self.0.get(r.tok as usize).map_or("", String::as_str)
    }
}

pub struct Container {
    pub version: i32,
    header: (usize, usize),
    names: Names,
    /// (name, data start, length)
    pub files: Vec<(String, usize, usize)>,
}

fn directory(d: &[u8], mut o: usize) -> Option<Vec<(String, usize, usize)>> {
    let mut out = vec![];
    while o < d.len() {
        let name = cstr(d.get(o..o + 260)?);
        let n = usize::try_from(i32_at(d, o + 260)?).ok()?;
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_graphic()) {
            return None;
        }
        let start = o + 264;
        if start + n > d.len() {
            return None;
        }
        out.push((name, start, n));
        o = start + n;
    }
    Some(out)
}

pub fn container(d: &[u8]) -> Option<Container> {
    if !d.starts_with(b"JSAV") && !d.starts_with(b"S2SV") {
        return None;
    }
    let version = i32_at(d, 4)?;
    let size = usize::try_from(i32_at(d, 8)?).ok()?;
    let count = usize::try_from(i32_at(d, 12)?).ok()?;
    let tbytes = usize::try_from(i32_at(d, 16)?).ok()?;
    if count > 1 << 16 {
        return None;
    }
    let names = Names(tokens(d, 20, tbytes, count)?);
    let hstart = 20 + tbytes;
    let hend = hstart.checked_add(size)?;
    if hend > d.len() {
        return None;
    }
    // Newer Source builds write the number of entries first; older ones don't.
    let files = match i32_at(d, hend) {
        Some(n) if (0..100_000).contains(&n) => directory(d, hend + 4).filter(|f| f.len() == n as usize).or_else(|| directory(d, hend)),
        _ => directory(d, hend),
    }?;
    Some(Container { version, header: (hstart, hend), names, files })
}

impl Container {
    pub fn engine(&self) -> Option<Engine> {
        match self.version {
            0x71 => Some(Engine::GoldSrc),
            // 0x765: Black Mesa.
            0x73 | 0x765 => Some(Engine::Source),
            0x74 => Some(Engine::Source2),
            _ => None,
        }
    }

    fn header_text(&self, d: &[u8], field: &str) -> Option<String> {
        records(d, self.header.0, self.header.1).iter().find(|r| self.names.of(r) == field).map(|r| cstr(&d[r.at..r.at + r.len]))
    }

    pub fn map(&self, d: &[u8]) -> Option<String> {
        // Source 2 names the map's spawn groups instead; the first is the map itself.
        self.header_text(d, "mapName").or_else(|| self.header_text(d, "m_sGroupName")).filter(|m| !m.is_empty())
    }
}

/// An entity: classname and where its records are (absolute offsets).
#[derive(Clone, Debug)]
struct Ent {
    class: String,
    start: usize,
    end: usize,
}

/// A map's entity state (`<map>.hl1`).
struct Level {
    names: Names,
    ents: Vec<Ent>,
    /// The data section (absolute), for values outside entities (skill level).
    data: (usize, usize),
}

const TABLE_FIELDS: &[&str] = &["id", "edictindex", "saveentityindex", "restoreentityindex", "location", "size", "flags", "classname", "globalname", "landmarkModelSpace", "modelname", "entityname", "m_pPrecacheEntityKeys"];

/// Reads `ETABLE` records from `recs`: one entity per `ETABLE`, until a record that isn't
/// part of the table. Returns the entities (location relative) and how many records were used.
fn entity_table(d: &[u8], names: &Names, recs: &[Rec]) -> (Vec<(String, i64, i64)>, usize) {
    let mut out: Vec<(String, i64, i64)> = vec![];
    let mut used = 0;
    for r in recs {
        let n = names.of(r);
        if n == "ETABLE" {
            out.push((String::new(), 0, 0));
        } else if TABLE_FIELDS.contains(&n) && !out.is_empty() {
            let e = out.last_mut().unwrap();
            match n {
                "classname" => e.0 = cstr(&d[r.at..r.at + r.len]),
                "location" if r.len == 4 => e.1 = i64::from(i32_at(d, r.at).unwrap_or(0)),
                "size" if r.len == 4 => e.2 = i64::from(i32_at(d, r.at).unwrap_or(0)),
                _ => {}
            }
        } else {
            break;
        }
        used += 1;
    }
    (out, used)
}

/// The entity table sits among the block headers: find its first `ETABLE` record.
fn find_table(d: &[u8], names: &Names, hs: usize, he: usize) -> Option<Vec<(String, i64, i64)>> {
    let et = names.0.iter().position(|s| s == "ETABLE")? as u16;
    let pat = [4, 0, et as u8, (et >> 8) as u8];
    let mut from = hs;
    loop {
        let p = from + d.get(from..he)?.windows(4).position(|w| w == pat)?;
        let (t, _) = entity_table(d, names, &records(d, p, he));
        if t.len() > 1 {
            return Some(t);
        }
        from = p + 1;
    }
}

fn level(d: &[u8], engine: Engine, start: usize, len: usize) -> Option<Level> {
    if d.get(start..start + 4)? != b"VALV" {
        return None;
    }
    let end = start + len;
    let h = |i: usize| i32_at(d, start + 4 + 4 * i).and_then(|v| usize::try_from(v).ok());
    match engine {
        Engine::GoldSrc => {
            let (size, tcount, tbytes) = (h(1)?, h(3)?, h(4)?);
            let names = Names(tokens(d, start + 24, tbytes, tcount)?);
            let ds = start + 24 + tbytes;
            let de = ds.checked_add(size)?;
            if de > end {
                return None;
            }
            let recs = records(d, ds, de);
            let (table, used) = entity_table(d, &names, &recs);
            // Locations count from the first record after the table.
            let base = recs.get(used)?.at - 4;
            let ents = table
                .into_iter()
                .map(|(class, loc, size)| {
                    let s = (base as i64 + loc).clamp(ds as i64, de as i64) as usize;
                    Ent { class, start: s, end: (s as i64 + size.max(0)).min(de as i64) as usize }
                })
                .collect();
            Some(Level { names, ents, data: (base, de) })
        }
        Engine::Source2 => {
            // VALV, version, token count, ?, token bytes, then the sizes of three sections:
            // block headers (with the entity table at their end), strings, entity data.
            let (tcount, tbytes, a, b, c) = (h(1)?, h(3)?, h(4)?, h(5)?, h(6)?);
            let names = Names(tokens(d, start + 32, tbytes, tcount)?);
            let hs = start + 32 + tbytes;
            let he = hs.checked_add(a)?;
            let ds = he.checked_add(b)?;
            let de = ds.checked_add(c)?;
            if de > end {
                return None;
            }
            let ents = find_table(d, &names, hs, he)?
                .into_iter()
                .map(|(class, loc, size)| {
                    let s = (ds as i64 + loc).clamp(ds as i64, de as i64) as usize;
                    Ent { class, start: s, end: (s as i64 + size.max(0)).min(de as i64) as usize }
                })
                .collect();
            Some(Level { names, ents, data: (ds, de) })
        }
        Engine::Source => {
            let (sbytes, scount, hbytes, dbytes) = (h(1)?, h(2)?, h(3)?, h(4)?);
            let names = Names(tokens(d, start + 24, sbytes, scount)?);
            let hs = start + 24 + sbytes;
            let he = hs.checked_add(hbytes)?;
            let ds = he;
            let de = ds.checked_add(dbytes)?;
            if de > end {
                return None;
            }
            let ents = find_table(d, &names, hs, he)?
                .into_iter()
                .map(|(class, loc, size)| {
                    let s = (ds as i64 + loc).clamp(ds as i64, de as i64) as usize;
                    Ent { class, start: s, end: (s as i64 + size.max(0)).min(de as i64) as usize }
                })
                .collect();
            Some(Level { names, ents, data: (ds, de) })
        }
    }
}

impl Level {
    fn fields(&self, d: &[u8], e: &Ent) -> Vec<(String, Rec)> {
        records(d, e.start, e.end).into_iter().map(|r| (self.names.of(&r).to_string(), r)).collect()
    }
}

fn get<'a>(f: &'a [(String, Rec)], name: &str) -> Option<&'a Rec> {
    f.iter().find(|(n, _)| n == name).map(|(_, r)| r)
}

fn int(d: &[u8], f: &[(String, Rec)], name: &str) -> Option<i32> {
    get(f, name).filter(|r| r.len == 4).and_then(|r| i32_at(d, r.at))
}

/// HUD name and ammo names (primary, secondary) of a weapon, by classname (from the SDKs).
pub fn weapon_info(class: &str) -> Option<(&'static str, Option<&'static str>, Option<&'static str>)> {
    Some(match class.strip_prefix("weapon_")?.to_ascii_lowercase().as_str() {
        "crowbar" => ("Crowbar", None, None),
        "9mmhandgun" | "glock" => ("9mm pistol", Some("9mm"), None),
        "357" | "python" => (".357 Magnum", Some(".357"), None),
        "9mmar" | "mp5" => ("MP5", Some("9mm"), Some("AR grenades")),
        "shotgun" => ("Shotgun", Some("Buckshot"), None),
        "crossbow" => ("Crossbow", Some("Crossbow bolts"), None),
        "rpg" => ("RPG", Some("Rockets"), None),
        "gauss" => ("Tau cannon", Some("Uranium"), None),
        "egon" => ("Gluon gun", Some("Uranium"), None),
        "hornetgun" => ("Hivehand", Some("Hornets"), None),
        "handgrenade" => ("Hand grenades", Some("Hand grenades"), None),
        "satchel" => ("Satchel charges", Some("Satchel charges"), None),
        "tripmine" => ("Tripmines", Some("Tripmines"), None),
        "snark" => ("Snarks", Some("Snarks"), None),
        // Opposing Force
        "pipewrench" => ("Pipe wrench", None, None),
        "knife" => ("Combat knife", None, None),
        "grapple" => ("Barnacle grapple", None, None),
        "eagle" => ("Desert Eagle", Some(".357"), None),
        "m249" => ("M249 SAW", Some("5.56mm"), None),
        "displacer" => ("Displacer cannon", Some("Uranium"), None),
        "sniperrifle" => ("Sniper rifle", Some("7.62mm"), None),
        "shockrifle" => ("Shock roach", Some("Shock charge"), None),
        "sporelauncher" => ("Spore launcher", Some("Spores"), None),
        "penguin" => ("Penguins", Some("Penguins"), None),
        // Black Mesa
        "tau" => ("Tau cannon", Some("Uranium"), None),
        "gluon" => ("Gluon gun", Some("Uranium"), None),
        "hivehand" => ("Hivehand", Some("Hornets"), None),
        // Half-Life 2 and the episodes
        "physcannon" => ("Gravity Gun", None, None),
        "pistol" => ("Pistol", Some("Pistol rounds"), None),
        "smg1" => ("SMG", Some("SMG rounds"), Some("SMG grenades")),
        "ar2" => ("Pulse rifle (AR2)", Some("Pulse rounds"), Some("Energy balls")),
        "frag" => ("Grenades", Some("Grenades"), None),
        "slam" => ("S.L.A.M.", Some("S.L.A.M."), None),
        "bugbait" => ("Bugbait", None, None),
        "stunstick" => ("Stunstick", None, None),
        _ => return None,
    })
}

/// "weapon_sniperrifle" → "Sniperrifle" for weapons the table doesn't know.
fn plain_name(class: &str) -> String {
    let s = class.strip_prefix("weapon_").unwrap_or(class);
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

fn game_name(engine: Engine, map: &str) -> &'static str {
    let m = map.to_ascii_lowercase();
    match engine {
        Engine::GoldSrc if m.starts_with("of") => "Half-Life: Opposing Force",
        Engine::GoldSrc if m.starts_with("ba_") => "Half-Life: Blue Shift",
        Engine::GoldSrc => "Half-Life",
        Engine::Source if m.starts_with("ep1_") => "Half-Life 2: Episode One",
        Engine::Source if m.starts_with("ep2_") => "Half-Life 2: Episode Two",
        Engine::Source if m == "d2_lostcoast" => "Half-Life 2: Lost Coast",
        Engine::Source if m.starts_with("bm_") => "Black Mesa",
        Engine::Source if m.starts_with("d1_") || m.starts_with("d2_") || m.starts_with("d3_") || m == "background01" => "Half-Life 2",
        // Half-Life: Source plays the original maps (c1a0, t0a0…).
        Engine::Source if (m.starts_with('c') || m.starts_with('t')) && m.as_bytes().get(1).is_some_and(u8::is_ascii_digit) => "Half-Life: Source",
        Engine::Source => "Source engine game",
        Engine::Source2 => "Half-Life: Alyx",
    }
}

/// "#HL2_Chapter11_Title" or "#HLVR_Chapter_Number_06 …" → "Chapter 11", "Chapter 6".
fn chapter(comment: &str) -> Option<String> {
    let c = comment.trim().trim_start_matches('#');
    let i = c.find("Chapter")?;
    let n: String = c[i + 7..].trim_start_matches(|ch: char| ch == '_' || ch.is_ascii_alphabetic()).chars().take_while(char::is_ascii_digit).collect();
    (!n.is_empty()).then(|| format!("Chapter {}", n.trim_start_matches('0')))
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && ((d.starts_with(b"JSAV") && matches!(i32_at(d, 4), Some(0x71 | 0x73 | 0x765))) || (d.starts_with(b"S2SV") && i32_at(d, 4) == Some(0x74)))
}

pub const KIND_GOLDSRC: &str = "Half-Life save (GoldSrc)";
pub const KIND_SOURCE: &str = "Half-Life 2 save (Source)";
pub const KIND_ALYX: &str = "Half-Life: Alyx save (Source 2)";

/// Half-Life: Alyx: the player's `m_iAmmo` slots, checked against the ammo counts the game
/// writes into each save's description ("stats <health> <resin> <pistol> <shotgun> <rapidfire> …").
const ALYX_AMMO: &[(usize, &str)] = &[(3, "Pistol rounds"), (7, "Shotgun shells"), (4, "Rapidfire rounds")];

fn alyx_weapon(class: &str) -> Option<&'static str> {
    Some(match class {
        "hlvr_weapon_energygun" => "Pistol",
        "hlvr_weapon_shotgun" => "Shotgun",
        "hlvr_weapon_rapidfire" => "Rapidfire",
        "hlvr_multitool" => "Multi-tool",
        _ => return None,
    })
}

fn parse_alyx(d: &[u8], c: &Container, map: &str, lv: &Level, pf: &[(String, Rec)]) -> Result<TreeDoc, String> {
    let mut b = Builder::new(d);
    let mut missing: Vec<String> = vec![];
    b.info(&["Save", "Game"], "Half-Life: Alyx");
    b.info(&["Save", "Map"], map.to_string());
    if let Some(ch) = c.header_text(d, "m_sComment").as_deref().and_then(chapter) {
        b.info(&["Save", "Chapter"], ch);
    }
    for (label, field, lo, hi) in [("Health", "m_iHealth", 1, 999), ("Max health", "m_iMaxHealth", 1, 999), ("Resin", "m_nCraftingCurrency", 0, 9999)] {
        match get(pf, field).filter(|r| r.len == 4) {
            Some(r) => {
                b.ranged(&["Player", label], r.at, Kind::I32, lo, hi);
            }
            None => missing.push(format!("{label}: 0, so the game left it out of the save")),
        }
    }
    match get(pf, "m_iAmmo").filter(|r| r.len % 4 == 0 && r.len <= 1024) {
        Some(r) => {
            for &(t, label) in ALYX_AMMO {
                if 4 * t + 4 <= r.len {
                    b.ranged(&["Ammo", label], r.at + 4 * t, Kind::I32, 0, 9999);
                }
            }
            for t in 0..r.len / 4 {
                if !ALYX_AMMO.iter().any(|a| a.0 == t) && i32_at(d, r.at + 4 * t).unwrap_or(0) != 0 {
                    b.ranged(&["Ammo", &format!("Ammo type {t}")], r.at + 4 * t, Kind::I32, 0, 9999);
                }
            }
        }
        None => missing.push("Ammo: none carried, so the game left the ammo list out of the save".into()),
    }
    // The weapons Alyx has are kept attached to her hands.
    let mut held: Vec<(&str, &str)> = vec![];
    for e in &lv.ents {
        let Some(name) = alyx_weapon(&e.class) else { continue };
        let f = lv.fields(d, e);
        if get(&f, "m_bIsAttached").is_some_and(|r| r.len == 1 && d[r.at] != 0) && !held.iter().any(|h| h.0 == name) {
            held.push((name, &e.class));
        }
    }
    held.sort_by_key(|h| ["Pistol", "Shotgun", "Rapidfire", "Multi-tool"].iter().position(|n| *n == h.0));
    for (name, class) in &held {
        b.info(&["Weapons", name, "Entity"], class.to_string());
    }
    if held.is_empty() {
        missing.push("Weapons: none carried".into());
    }
    if !missing.is_empty() {
        b.info(&["Not in this save"], missing);
    }
    Ok(b.finish(KIND_ALYX, Checksum::None).with_note(
        "Alyx as saved in the current map. The save's description in the game's Load menu (its health, resin and ammo) is written when the game saves, so it shows the old numbers until you save again.",
    ))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let c = container(d).ok_or("not a Half-Life save container")?;
    let engine = c.engine().ok_or("unknown save version")?;
    let map = c.map(d).ok_or("the save names no map")?;
    let want = format!("{map}.hl1");
    let &(_, start, len) = c.files.iter().find(|f| f.0.eq_ignore_ascii_case(&want)).ok_or("the current map isn't in the save")?;
    let lv = level(d, engine, start, len).ok_or("couldn't read the map's entity state")?;
    let pi = lv.ents.iter().position(|e| e.class == "player" && e.end > e.start).ok_or("no player in the current map")?;
    let pf = lv.fields(d, &lv.ents[pi]);
    if engine == Engine::Source2 {
        return parse_alyx(d, &c, &map, &lv, &pf);
    }
    let gs = engine == Engine::GoldSrc;

    let mut b = Builder::new(d);
    let mut missing: Vec<String> = vec![];
    b.info(&["Save", "Game"], game_name(engine, &map));
    b.info(&["Save", "Map"], map.clone());
    if let Some(ch) = c.header_text(d, "comment").as_deref().and_then(chapter) {
        b.info(&["Save", "Chapter"], ch);
    }
    let maps = c.files.iter().filter(|f| f.0.to_ascii_lowercase().ends_with(".hl1")).count();
    b.info(&["Save", "Maps in this save"], maps as i64);
    if let Some(s) = records(d, lv.data.0, lv.data.1).iter().take(64).find(|r| lv.names.of(r) == "skillLevel" && r.len == 4).and_then(|r| i32_at(d, r.at)) {
        let s = match s {
            1 => "Easy".to_string(),
            2 => "Medium".to_string(),
            3 => "Hard".to_string(),
            n => n.to_string(),
        };
        b.info(&["Save", "Difficulty"], s);
    }

    // Health and suit.
    let (health, max_health, armor) = if gs { ("health", "max_health", "armorvalue") } else { ("m_iHealth", "m_iMaxHealth", "m_ArmorValue") };
    let kind = if gs { Kind::F32 } else { Kind::I32 };
    for (label, field, lo, hi) in [("Health", health, 1, 999), ("Max health", max_health, 1, 999), ("Armor", armor, 0, 999)] {
        match get(&pf, field).filter(|r| r.len == 4) {
            Some(r) => {
                b.ranged(&["Player", label], r.at, kind, lo, hi);
            }
            None => missing.push(format!("{label}: 0, so the game left it out of the save")),
        }
    }
    if gs {
        match get(&pf, "m_iFlashBattery").filter(|r| r.len == 4) {
            Some(r) => {
                b.ranged(&["Player", "Flashlight"], r.at, Kind::I32, 0, 100);
            }
            None => missing.push("Flashlight: empty, so the game left it out of the save".into()),
        }
        if let Some(w) = int(d, &pf, "weapons") {
            b.info(&["Player", "HEV suit"], if (w as u32) & 0x8000_0000 != 0 { "Yes" } else { "No" });
        }
    } else {
        let sub = |name: &str| -> Vec<(String, Rec)> {
            get(&pf, name).map(|r| records(d, r.at, r.at + r.len).into_iter().map(|x| (lv.names.of(&x).to_string(), x)).collect()).unwrap_or_default()
        };
        let local = sub("m_Local");
        if let Some(r) = get(&local, "m_bWearingSuit").filter(|r| r.len == 1) {
            b.info(&["Player", "HEV suit"], if d[r.at] != 0 { "Yes" } else { "No" });
        }
        let hl2 = sub("m_HL2Local");
        if !hl2.is_empty() {
            for (label, field) in [("Suit power", "m_flSuitPower"), ("Flashlight", "m_flFlashBattery")] {
                // Half-Life 2 and Episode One keep -1 here for the flashlight (it runs off suit power).
                if let Some(r) = get(&hl2, field).filter(|r| r.len == 4 && i32_at(d, r.at).is_some_and(|v| f32::from_bits(v as u32) >= 0.0)) {
                    b.ranged(&["Player", label], r.at, Kind::F32, 0, 100);
                } else if field == "m_flSuitPower" {
                    missing.push("Suit power: 0, so the game left it out of the save".into());
                }
            }
        }
    }

    // Weapons the player carries, as entity indexes.
    let mut held: Vec<usize> = vec![];
    let idx = |v: i32| usize::try_from(v).ok().filter(|&i| i > 0 && i < lv.ents.len());
    if gs {
        if let Some(r) = get(&pf, "m_rgpPlayerItems") {
            for k in 0..r.len / 4 {
                let mut cur = i32_at(d, r.at + 4 * k).and_then(idx);
                let mut steps = 0;
                while let Some(i) = cur {
                    if held.contains(&i) || steps > 64 {
                        break;
                    }
                    held.push(i);
                    steps += 1;
                    cur = int(d, &lv.fields(d, &lv.ents[i]), "m_pNext").and_then(idx);
                }
            }
        }
        if held.is_empty() {
            held = (0..lv.ents.len()).filter(|&i| lv.ents[i].class.starts_with("weapon_") && int(d, &lv.fields(d, &lv.ents[i]), "m_pPlayer") == Some(pi as i32)).collect();
        }
    } else if let Some(r) = get(&pf, "m_hMyWeapons") {
        held = (0..r.len / 4).filter_map(|k| i32_at(d, r.at + 4 * k).and_then(idx)).collect();
    }
    held.retain(|&i| lv.ents[i].class.starts_with("weapon_") && lv.ents[i].end > lv.ents[i].start);
    let active = int(d, &pf, if gs { "m_pActiveItem" } else { "m_hActiveWeapon" }).and_then(idx);

    // Ammo names: each weapon entity in the level says which ammo types it uses.
    let (pri, sec, clip1, clip2) = if gs { ("m_iPrimaryAmmoType", "m_iSecondaryAmmoType", "m_iClip", "") } else { ("m_iPrimaryAmmoType", "m_iSecondaryAmmoType", "m_iClip1", "m_iClip2") };
    let mut ammo_names: BTreeMap<usize, Vec<&'static str>> = BTreeMap::new();
    let mut order: Vec<usize> = held.clone();
    order.extend((0..lv.ents.len()).filter(|i| !held.contains(i) && lv.ents[*i].class.starts_with("weapon_") && lv.ents[*i].end > lv.ents[*i].start));
    for &i in &order {
        let Some((_, a1, a2)) = weapon_info(&lv.ents[i].class) else { continue };
        let f = lv.fields(d, &lv.ents[i]);
        for (field, name) in [(pri, a1), (sec, a2)] {
            if let (Some(t), Some(n)) = (int(d, &f, field).and_then(|t| usize::try_from(t).ok()), name) {
                let e = ammo_names.entry(t).or_default();
                if !e.contains(&n) {
                    e.push(n);
                }
            }
        }
    }
    let ammo_label = |t: usize| ammo_names.get(&t).map_or_else(|| format!("Ammo type {t}"), |n| n.join(" / "));

    match get(&pf, if gs { "m_rgAmmo" } else { "m_iAmmo" }).filter(|r| r.len % 4 == 0 && r.len <= 1024) {
        Some(r) => {
            let mut used: Vec<String> = vec![];
            for t in 0..r.len / 4 {
                let v = i32_at(d, r.at + 4 * t).unwrap_or(0);
                if v == 0 && !ammo_names.contains_key(&t) {
                    continue;
                }
                let mut label = ammo_label(t);
                if used.contains(&label) {
                    label = format!("{label} (type {t})");
                }
                b.ranged(&["Ammo", &label], r.at + 4 * t, Kind::I32, 0, 9999);
                used.push(label);
            }
        }
        None => missing.push("Ammo: none carried, so the game left the ammo list out of the save".into()),
    }

    let mut names_used: Vec<String> = vec![];
    for &i in &held {
        let e = &lv.ents[i];
        let f = lv.fields(d, e);
        let mut name = weapon_info(&e.class).map_or_else(|| plain_name(&e.class), |w| w.0.to_string());
        if names_used.contains(&name) {
            name = format!("{name} ({})", names_used.len() + 1);
        }
        names_used.push(name.clone());
        b.info(&["Weapons", &name, "Entity"], e.class.clone());
        if Some(i) == active {
            b.info(&["Weapons", &name, "In hand"], "Yes");
        }
        let types: Vec<String> = [pri, sec].iter().filter_map(|k| int(d, &f, k)).filter_map(|t| usize::try_from(t).ok()).map(ammo_label).collect();
        if !types.is_empty() {
            b.info(&["Weapons", &name, "Uses"], types.join(", "));
        }
        // Weapons that take no ammo (crowbar, Gravity Gun) keep a meaningless clip value.
        let no_ammo = weapon_info(&e.class).is_some_and(|w| w.1.is_none());
        for (label, field) in [("Magazine", clip1), ("Second magazine", clip2)] {
            if field.is_empty() || no_ammo {
                continue;
            }
            match get(&f, field).filter(|r| r.len == 4) {
                Some(r) if i32_at(d, r.at).unwrap_or(-1) >= 0 => {
                    b.ranged(&["Weapons", &name, label], r.at, Kind::I32, 0, 999);
                }
                Some(_) => {}
                // Only weapons that take magazines matter here: those with an ammo type.
                None if label == "Magazine" && int(d, &f, pri).is_some_and(|t| t > 0) => {
                    b.info(&["Weapons", &name, label], "empty (not stored)");
                }
                None => {}
            }
        }
    }
    if held.is_empty() {
        missing.push("Weapons: none carried".into());
    }
    if let Some(a) = active.filter(|a| held.contains(a)) {
        let n = weapon_info(&lv.ents[a].class).map_or_else(|| plain_name(&lv.ents[a].class), |w| w.0.to_string());
        b.info(&["Player", "Weapon in hand"], n);
    }
    if !missing.is_empty() {
        b.info(&["Not in this save"], missing);
    }
    let kind = if gs { KIND_GOLDSRC } else { KIND_SOURCE };
    Ok(b.finish(kind, Checksum::None).with_note(
        "The player as saved in the current map. Values that are zero (no armour, an empty magazine) aren't stored by the game, so they can't be set here.",
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Token table builder: names get indexes in order.
    struct Toks(Vec<String>);
    impl Toks {
        fn id(&mut self, s: &str) -> u16 {
            if let Some(i) = self.0.iter().position(|x| x == s) {
                return i as u16;
            }
            self.0.push(s.into());
            (self.0.len() - 1) as u16
        }
        fn bytes(&self) -> Vec<u8> {
            let mut o = vec![];
            for s in &self.0 {
                o.extend(s.as_bytes());
                o.push(0);
            }
            o
        }
    }

    fn rec(o: &mut Vec<u8>, t: &mut Toks, name: &str, v: &[u8]) {
        o.extend((v.len() as u16).to_le_bytes());
        o.extend(t.id(name).to_le_bytes());
        o.extend(v);
    }

    fn ints(v: &[i32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn sz(s: &str) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        v
    }

    /// Wraps one map's state into a `JSAV` container whose header names that map.
    fn jsav(version: i32, map: &str, hl1: Vec<u8>, count_first: bool) -> Vec<u8> {
        let mut t = Toks(vec![String::new()]);
        let mut hdr = vec![];
        rec(&mut hdr, &mut t, "GameHeader", &ints(&[2]));
        let mut name = sz(map);
        name.resize(32, 0);
        rec(&mut hdr, &mut t, "mapName", &name);
        rec(&mut hdr, &mut t, "comment", &sz("#HL2_Chapter3_Title"));
        let tb = t.bytes();
        let mut d = b"JSAV".to_vec();
        d.extend(ints(&[version, hdr.len() as i32, t.0.len() as i32, tb.len() as i32]));
        d.extend(tb);
        d.extend(hdr);
        let files = [(format!("{map}.hl1"), hl1), (format!("{map}.hl2"), b"VALV".to_vec())];
        if count_first {
            d.extend(ints(&[files.len() as i32]));
        }
        for (n, data) in files {
            let mut nm = n.into_bytes();
            nm.resize(260, 0);
            d.extend(nm);
            d.extend(ints(&[data.len() as i32]));
            d.extend(data);
        }
        d
    }

    /// Entities as (classname, records): the entity table, a save header, then their data.
    fn ents(t: &mut Toks, list: &[(&str, Vec<u8>)]) -> (Vec<u8>, Vec<u8>) {
        let mut table = vec![];
        let mut data = vec![];
        for (i, (class, recs)) in list.iter().enumerate() {
            rec(&mut table, t, "ETABLE", &ints(&[4]));
            rec(&mut table, t, "id", &ints(&[i as i32]));
            rec(&mut table, t, "location", &ints(&[data.len() as i32]));
            rec(&mut table, t, "size", &ints(&[recs.len() as i32]));
            rec(&mut table, t, "classname", &sz(class));
            data.extend(recs);
        }
        (table, data)
    }

    pub fn goldsrc() -> Vec<u8> {
        let mut t = Toks(vec![String::new()]);
        let mut pl = vec![];
        rec(&mut pl, &mut t, "ENTVARS", &ints(&[4]));
        rec(&mut pl, &mut t, "classname", &sz("player"));
        rec(&mut pl, &mut t, "health", &75f32.to_le_bytes());
        rec(&mut pl, &mut t, "max_health", &100f32.to_le_bytes());
        rec(&mut pl, &mut t, "weapons", &ints(&[i32::MIN | 0b10100]));
        rec(&mut pl, &mut t, "PLAYER", &ints(&[3]));
        rec(&mut pl, &mut t, "m_iFlashBattery", &ints(&[80]));
        rec(&mut pl, &mut t, "m_rgpPlayerItems", &ints(&[-1, -1, 2, -1, -1, -1]));
        rec(&mut pl, &mut t, "m_pActiveItem", &ints(&[2]));
        let mut ammo = [0i32; 32];
        ammo[1] = 12;
        ammo[2] = 150;
        ammo[3] = 2;
        rec(&mut pl, &mut t, "m_rgAmmo", &ints(&ammo));
        let mut mp5 = vec![];
        rec(&mut mp5, &mut t, "m_pPlayer", &ints(&[1]));
        rec(&mut mp5, &mut t, "m_pNext", &ints(&[3]));
        rec(&mut mp5, &mut t, "m_iId", &ints(&[4]));
        rec(&mut mp5, &mut t, "m_iPrimaryAmmoType", &ints(&[2]));
        rec(&mut mp5, &mut t, "m_iSecondaryAmmoType", &ints(&[3]));
        rec(&mut mp5, &mut t, "m_iClip", &ints(&[25]));
        let mut sg = vec![];
        rec(&mut sg, &mut t, "m_pPlayer", &ints(&[1]));
        rec(&mut sg, &mut t, "m_iPrimaryAmmoType", &ints(&[1]));
        let (table, data) = ents(&mut t, &[("worldspawn", vec![]), ("player", pl), ("weapon_9mmAR", mp5), ("weapon_shotgun", sg)]);
        let mut body = table;
        let mut sh = vec![];
        rec(&mut sh, &mut t, "Save Header", &ints(&[1]));
        rec(&mut sh, &mut t, "skillLevel", &ints(&[2]));
        // Locations count from the save header: put it first, then the entity data.
        let shift = sh.len() as i32;
        let mut fixed = vec![];
        for r in records(&body, 0, body.len()) {
            let mut v = body[r.at..r.at + r.len].to_vec();
            if t.0[r.tok as usize] == "location" {
                v = ints(&[i32_at(&v, 0).unwrap() + shift]);
            }
            fixed.extend((r.len as u16).to_le_bytes());
            fixed.extend(r.tok.to_le_bytes());
            fixed.extend(v);
        }
        body = fixed;
        body.extend(sh);
        body.extend(data);
        let tb = t.bytes();
        let mut hl1 = b"VALV".to_vec();
        hl1.extend(ints(&[0x71, body.len() as i32, 4, t.0.len() as i32, tb.len() as i32]));
        hl1.extend(tb);
        hl1.extend(body);
        jsav(0x71, "of1a1", hl1, false)
    }

    pub fn source() -> Vec<u8> {
        let mut t = Toks(vec![String::new()]);
        let mut pl = vec![];
        rec(&mut pl, &mut t, "CBaseEntity", &ints(&[3]));
        rec(&mut pl, &mut t, "m_iClassname", &sz("player"));
        rec(&mut pl, &mut t, "m_iHealth", &ints(&[58]));
        rec(&mut pl, &mut t, "m_iMaxHealth", &ints(&[100]));
        let mut ammo = [0i32; 32];
        ammo[4] = 225;
        ammo[9] = 3;
        ammo[21] = 1;
        rec(&mut pl, &mut t, "m_iAmmo", &ints(&ammo));
        let mut w = [-1i32; 48];
        w[0] = 2;
        rec(&mut pl, &mut t, "m_hMyWeapons", &ints(&w));
        rec(&mut pl, &mut t, "m_hActiveWeapon", &ints(&[2]));
        rec(&mut pl, &mut t, "m_ArmorValue", &ints(&[28]));
        let mut local = vec![];
        rec(&mut local, &mut t, "CHL2PlayerLocalData", &ints(&[1]));
        rec(&mut local, &mut t, "m_flSuitPower", &55f32.to_le_bytes());
        rec(&mut pl, &mut t, "m_HL2Local", &local);
        let mut smg = vec![];
        rec(&mut smg, &mut t, "CBaseEntity", &ints(&[1]));
        rec(&mut smg, &mut t, "m_iClassname", &sz("weapon_smg1"));
        rec(&mut smg, &mut t, "m_hOwner", &ints(&[1]));
        rec(&mut smg, &mut t, "m_iPrimaryAmmoType", &ints(&[4]));
        rec(&mut smg, &mut t, "m_iSecondaryAmmoType", &ints(&[9]));
        rec(&mut smg, &mut t, "m_iClip1", &ints(&[45]));
        rec(&mut smg, &mut t, "m_iClip2", &ints(&[-1]));
        let (table, data) = ents(&mut t, &[("worldspawn", vec![]), ("player", pl), ("weapon_smg1", smg)]);
        let mut hdr = vec![0u8; 12]; // block headers come first
        hdr.extend(table);
        let mut body = vec![];
        rec(&mut body, &mut t, "skillLevel", &ints(&[3]));
        let shift = body.len() as i32;
        // Locations are relative to the data section: move them past the skill record.
        let mut fixed = vec![0u8; 12];
        for r in records(&hdr, 12, hdr.len()) {
            let mut v = hdr[r.at..r.at + r.len].to_vec();
            if t.0[r.tok as usize] == "location" {
                v = ints(&[i32_at(&v, 0).unwrap() + shift]);
            }
            fixed.extend((r.len as u16).to_le_bytes());
            fixed.extend(r.tok.to_le_bytes());
            fixed.extend(v);
        }
        body.extend(data);
        let tb = t.bytes();
        let mut hl1 = b"VALV".to_vec();
        hl1.extend(ints(&[0x73, tb.len() as i32, t.0.len() as i32, fixed.len() as i32, body.len() as i32]));
        hl1.extend(tb);
        hl1.extend(fixed);
        hl1.extend(body);
        jsav(0x73, "d1_canals_05", hl1, true)
    }

    /// Half-Life: Alyx: an `S2SV` container whose header names the map's spawn group, and a
    /// map state with a 32-byte header and three sections (block headers, strings, entity data).
    pub fn alyx() -> Vec<u8> {
        let mut t = Toks(vec![String::new()]);
        let mut pl = vec![];
        rec(&mut pl, &mut t, "CBaseEntity", &ints(&[3]));
        rec(&mut pl, &mut t, "m_iMaxHealth", &ints(&[100]));
        rec(&mut pl, &mut t, "m_iHealth", &ints(&[64]));
        let mut ammo = [0i32; 32];
        ammo[3] = 90;
        ammo[4] = 150;
        rec(&mut pl, &mut t, "m_iAmmo", &ints(&ammo));
        rec(&mut pl, &mut t, "CHL_VR_Player", &ints(&[2]));
        rec(&mut pl, &mut t, "m_nCraftingCurrency", &ints(&[26]));
        let mut gun = vec![];
        rec(&mut gun, &mut t, "m_bIsAttached", &[1]);
        let mut loose = vec![];
        rec(&mut loose, &mut t, "m_iHealth", &ints(&[1]));
        let (table, data) = ents(&mut t, &[("worldspawn", vec![]), ("player", pl), ("hlvr_weapon_energygun", gun), ("hlvr_weapon_shotgun", loose)]);
        let mut blocks = vec![0u8; 16];
        blocks.extend(table);
        let strings = b"strings\0".to_vec();
        let tb = t.bytes();
        let mut hl1 = b"VALV".to_vec();
        hl1.extend(ints(&[0x74, t.0.len() as i32, 0, tb.len() as i32, blocks.len() as i32, strings.len() as i32, data.len() as i32]));
        hl1.extend(tb);
        hl1.extend(blocks);
        hl1.extend(strings);
        hl1.extend(data);

        let mut t = Toks(vec![String::new()]);
        let mut hdr = vec![];
        rec(&mut hdr, &mut t, "GameHeader", &ints(&[2]));
        rec(&mut hdr, &mut t, "m_sComment", &sz("#HLVR_Chapter_Number_06 #HLVR_Chapter_Title_06 stats 64 26 90 0 150 -1 0 1:00:00"));
        rec(&mut hdr, &mut t, "m_nSpawnGroupCount", &ints(&[1]));
        rec(&mut hdr, &mut t, "SpawnGroups", &ints(&[2]));
        rec(&mut hdr, &mut t, "m_sGroupName", &sz("a3_distillery"));
        let tb = t.bytes();
        let mut d = b"S2SV".to_vec();
        d.extend(ints(&[0x74, hdr.len() as i32, t.0.len() as i32, tb.len() as i32]));
        d.extend(tb);
        d.extend(hdr);
        let files = [("a3_distillery.hl1".to_string(), hl1), ("a3_distillery.hl3".to_string(), ints(&[0]))];
        d.extend(ints(&[files.len() as i32]));
        for (n, data) in files {
            let mut nm = n.into_bytes();
            nm.resize(260, 0);
            d.extend(nm);
            d.extend(ints(&[data.len() as i32]));
            d.extend(data);
        }
        d
    }

    #[test]
    fn alyx_reads_and_edits() {
        let d = alyx();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(doc.kind_name, KIND_ALYX);
        assert_eq!(v.pointer("/Save/Game"), Some(&"Half-Life: Alyx".into()));
        assert_eq!(v.pointer("/Save/Map"), Some(&"a3_distillery".into()));
        assert_eq!(v.pointer("/Save/Chapter"), Some(&"Chapter 6".into()));
        assert_eq!(v.pointer("/Player/Health"), Some(&64.into()));
        assert_eq!(v.pointer("/Player/Resin"), Some(&26.into()));
        assert_eq!(v.pointer("/Ammo/Pistol rounds"), Some(&90.into()));
        assert_eq!(v.pointer("/Ammo/Rapidfire rounds"), Some(&150.into()));
        assert_eq!(v.pointer("/Ammo/Shotgun shells"), Some(&0.into()));
        assert!(v.pointer("/Weapons/Pistol").is_some());
        // A weapon lying in the world (not attached to Alyx) isn't hers.
        assert!(v.pointer("/Weapons/Shotgun").is_none());
        doc.set("/Player/Resin", 500.into()).unwrap();
        doc.set("/Ammo/Shotgun shells", 30.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap().view;
        assert_eq!(back.pointer("/Player/Resin"), Some(&500.into()));
        assert_eq!(back.pointer("/Ammo/Shotgun shells"), Some(&30.into()));
        assert_eq!(back.pointer("/Player/Health"), Some(&64.into()));
    }

    #[test]
    fn goldsrc_reads_and_edits() {
        let d = goldsrc();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(doc.kind_name, KIND_GOLDSRC);
        assert_eq!(v.pointer("/Save/Game"), Some(&"Half-Life: Opposing Force".into()));
        assert_eq!(v.pointer("/Save/Difficulty"), Some(&"Medium".into()));
        assert_eq!(v.pointer("/Player/Health").and_then(|x| x.as_f64()), Some(75.0));
        assert_eq!(v.pointer("/Player/HEV suit"), Some(&"Yes".into()));
        assert_eq!(v.pointer("/Player/Flashlight"), Some(&80.into()));
        assert_eq!(v.pointer("/Player/Weapon in hand"), Some(&"MP5".into()));
        assert_eq!(v.pointer("/Ammo/9mm"), Some(&150.into()));
        assert_eq!(v.pointer("/Ammo/AR grenades"), Some(&2.into()));
        assert_eq!(v.pointer("/Ammo/Buckshot"), Some(&12.into()));
        assert_eq!(v.pointer("/Weapons/MP5/Magazine"), Some(&25.into()));
        assert_eq!(v.pointer("/Weapons/Shotgun/Magazine"), Some(&"empty (not stored)".into()));
        assert!(v.pointer("/Player/Armor").is_none());
        assert!(v.pointer("/Not in this save/0").and_then(|x| x.as_str()).is_some_and(|s| s.starts_with("Armor")));
        doc.set("/Player/Health", 100.0.into()).unwrap();
        doc.set("/Ammo/9mm", 250.into()).unwrap();
        doc.set("/Weapons/MP5/Magazine", 50.into()).unwrap();
        assert!(doc.set("/Ammo/9mm", (-1).into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap().view;
        assert_eq!(back.pointer("/Player/Health").and_then(|x| x.as_f64()), Some(100.0));
        assert_eq!(back.pointer("/Ammo/9mm"), Some(&250.into()));
        assert_eq!(back.pointer("/Weapons/MP5/Magazine"), Some(&50.into()));
        assert_eq!(back.pointer("/Ammo/Buckshot"), Some(&12.into()));
    }

    #[test]
    fn source_reads_and_edits() {
        let d = source();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(doc.kind_name, KIND_SOURCE);
        assert_eq!(v.pointer("/Save/Game"), Some(&"Half-Life 2".into()));
        assert_eq!(v.pointer("/Save/Chapter"), Some(&"Chapter 3".into()));
        assert_eq!(v.pointer("/Save/Difficulty"), Some(&"Hard".into()));
        assert_eq!(v.pointer("/Player/Health"), Some(&58.into()));
        assert_eq!(v.pointer("/Player/Armor"), Some(&28.into()));
        assert_eq!(v.pointer("/Player/Suit power").and_then(|x| x.as_f64()), Some(55.0));
        assert_eq!(v.pointer("/Ammo/SMG rounds"), Some(&225.into()));
        assert_eq!(v.pointer("/Ammo/SMG grenades"), Some(&3.into()));
        assert_eq!(v.pointer("/Ammo/Ammo type 21"), Some(&1.into()));
        assert_eq!(v.pointer("/Weapons/SMG/Magazine"), Some(&45.into()));
        assert!(v.pointer("/Weapons/SMG/Second magazine").is_none());
        doc.set("/Player/Armor", 100.into()).unwrap();
        doc.set("/Ammo/SMG grenades", 5.into()).unwrap();
        doc.set("/Player/Suit power", 100.0.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap().view;
        assert_eq!(back.pointer("/Player/Armor"), Some(&100.into()));
        assert_eq!(back.pointer("/Ammo/SMG grenades"), Some(&5.into()));
        assert_eq!(back.pointer("/Player/Suit power").and_then(|x| x.as_f64()), Some(100.0));
        assert_eq!(back.pointer("/Player/Health"), Some(&58.into()));
    }

    /// Research aid: HL_SAVES = folder(s) of sample saves, separated by `|`. Every `.sav` must
    /// parse, write back unchanged, and take an edit to health and an ammo count that reads
    /// back with every other value the same.
    #[test]
    #[ignore]
    fn real_saves() {
        let mut files = vec![];
        fn walk(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if p.is_dir() {
                for e in std::fs::read_dir(p).unwrap().flatten() {
                    walk(&e.path(), out);
                }
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sav")) {
                out.push(p.to_path_buf());
            }
        }
        for dir in std::env::var("HL_SAVES").unwrap().split('|') {
            walk(std::path::Path::new(dir), &mut files);
        }
        files.sort();
        let (mut ok, mut bad) = (0, vec![]);
        for f in &files {
            let d = std::fs::read(f).unwrap();
            let r = std::panic::catch_unwind(|| -> Result<String, String> {
                let mut doc = parse(&d)?;
                if doc.to_bytes()? != d {
                    return Err("no-edit write differs".into());
                }
                let before = doc.view.clone();
                let h = before.pointer("/Player/Health").and_then(|x| x.as_f64()).ok_or("no health")?;
                let nh = if h >= 999.0 { h - 1.0 } else { h + 1.0 };
                doc.set("/Player/Health", if nh.fract() == 0.0 { serde_json::json!(nh as i64) } else { serde_json::json!(nh) })?;
                let ammo = before.pointer("/Ammo").and_then(|a| a.as_object()).and_then(|a| a.iter().next().map(|(k, v)| (k.clone(), v.as_i64().unwrap_or(0))));
                if let Some((k, v)) = &ammo {
                    doc.set(&format!("/Ammo/{}", crate::json_view::pointer_escape(k)), (v + 7).into())?;
                }
                let out = doc.to_bytes()?;
                if out.len() != d.len() {
                    return Err("size changed".into());
                }
                let mut after = parse(&out)?.view;
                if after.pointer("/Player/Health").and_then(|x| x.as_f64()) != Some(nh) {
                    return Err("health didn't read back".into());
                }
                *after.pointer_mut("/Player/Health").unwrap() = before.pointer("/Player/Health").unwrap().clone();
                if let Some((k, v)) = &ammo {
                    let p = format!("/Ammo/{}", crate::json_view::pointer_escape(k));
                    if after.pointer(&p).and_then(|x| x.as_i64()) != Some(v + 7) {
                        return Err("ammo didn't read back".into());
                    }
                    *after.pointer_mut(&p).unwrap() = (*v).into();
                }
                if after != before {
                    return Err("other values changed".into());
                }
                let differ = d.iter().zip(&out).filter(|(a, b)| a != b).count();
                Ok(format!("{} bytes differ; {}", differ, serde_json::to_string(&before).unwrap()))
            });
            match r {
                Ok(Ok(s)) => {
                    ok += 1;
                    println!("OK {}: {}", f.display(), s);
                }
                Ok(Err(e)) => bad.push(format!("{}: {e}", f.display())),
                Err(_) => bad.push(format!("{}: panic", f.display())),
            }
        }
        println!("{ok} of {} saves pass", files.len());
        assert!(bad.is_empty(), "{bad:#?}");
    }
}
