//! God of War (2018) `game.sav` and God of War Ragnarök `GOWRSAVE*` (PC). Format facts from
//! the Forja editors by wilypizza-svg (José Murilo), which credit kg971 and xcier; item names
//! and equipment families in `data/` come from their catalogs. Checked against sample saves.
//!
//! **2018:** a 0x1040-byte header, then 20 slots of 0x1998C8 bytes. A slot starts with its
//! save time (u64, Unix), 10, the build and the title; at slot + 0x1041D a table of 16-byte
//! rows (8-byte item id, u32 quantity, u32 index) holds everything owned: XP and hacksilver
//! first, then resources and every piece of equipment (each upgrade level is its own row).
//! Quantities are edited in place; there is no checksum.
//!
//! **Ragnarök:** one save per file, in fixed blocks. The inventory (24-byte rows: id, an
//! instance number, 12 more bytes; some rows carry a list after them) holds equipment, one
//! row per piece, and a piece's level is part of its id, so a level is changed by writing the
//! id of the same piece at another level. The "attributes" table (16-byte rows like 2018's)
//! holds XP, hacksilver, resources and Yggdrasil's dew. Health and rage are floats.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

fn u16_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?) as usize)
}
fn u32_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}
fn id_value(id: &str) -> Option<u64> {
    let b: Vec<u8> = (0..8).map(|i| u8::from_str_radix(id.get(2 * i..2 * i + 2)?, 16).ok()).collect::<Option<_>>()?;
    Some(u64::from_le_bytes(b.try_into().ok()?))
}
fn find(d: &[u8], pat: &[u8], from: usize, to: usize) -> Option<usize> {
    let to = to.min(d.len());
    (from < to).then(|| d[from..to].windows(pat.len()).position(|w| w == pat).map(|i| i + from)).flatten()
}
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).filter_map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()).collect()
}

/// A catalog entry: name, category, and (2018) whether it's an editable resource.
struct Item {
    name: String,
    category: String,
    resource: bool,
}

fn table(src: &'static str, resource_col: Option<usize>, cat_col: usize) -> HashMap<String, Item> {
    src.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            Some((
                c.first()?.to_string(),
                Item { name: c.get(1)?.to_string(), category: c.get(cat_col)?.to_string(), resource: resource_col.and_then(|i| c.get(i)).is_some_and(|v| *v == "1") },
            ))
        })
        .collect()
}

fn items_2018() -> &'static HashMap<String, Item> {
    static T: OnceLock<HashMap<String, Item>> = OnceLock::new();
    T.get_or_init(|| table(include_str!("data/gow2018_items.tsv"), Some(4), 3))
}
fn items_ragnarok() -> &'static HashMap<String, Item> {
    static T: OnceLock<HashMap<String, Item>> = OnceLock::new();
    T.get_or_init(|| table(include_str!("data/gowr_items.tsv"), None, 2))
}

/// Ragnarök equipment families: (name, category, [(id, level, NG+)]).
type Family = (String, String, Vec<(String, u32, bool)>);
fn families() -> &'static (Vec<Family>, HashMap<String, usize>) {
    static T: OnceLock<(Vec<Family>, HashMap<String, usize>)> = OnceLock::new();
    T.get_or_init(|| {
        let mut list = vec![];
        let mut by_id = HashMap::new();
        for l in include_str!("data/gowr_families.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let c: Vec<&str> = l.split('\t').collect();
            let (Some(name), Some(cat), Some(vs)) = (c.first(), c.get(1), c.get(2)) else { continue };
            let variants: Vec<(String, u32, bool)> = vs
                .split(',')
                .filter_map(|v| {
                    let (id, lv) = v.split_once(':')?;
                    let ng = lv.ends_with('+');
                    Some((id.to_string(), lv.trim_end_matches('+').parse().ok()?, ng))
                })
                .collect();
            for (id, _, _) in &variants {
                by_id.insert(id.clone(), list.len());
            }
            list.push((name.to_string(), cat.to_string(), variants));
        }
        (list, by_id)
    })
}

fn date(ts: u64) -> String {
    // Days since 1970 → civil date (Howard Hinnant's algorithm).
    let (days, secs) = ((ts / 86400) as i64, ts % 86400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", secs / 3600, secs / 60 % 60)
}

// ------------------------------------------------------------------ God of War (2018)

const HEADER: usize = 0x1040;
const SLOT: usize = 0x1998C8;
const SLOTS: usize = 20;
const TABLE: usize = 0x1041D;
const XP: &str = "8A9A9B831CF67C9C";
const HACKSILVER: &str = "9E5240D27406E85A";

pub fn looks_2018(d: &[u8]) -> bool {
    d.len() == HEADER + SLOT * SLOTS
        && u32_at(d, 8) == Some(4)
        && u32_at(d, 16) == Some(208)
        && u32_at(d, 20) == Some(SLOT)
        && u32_at(d, 24) == Some(d.len())
}

struct Slot2018 {
    index: usize,
    time: u64,
    title: String,
    rows: Vec<(String, usize)>, // id, offset of the quantity
}

fn slot_2018(d: &[u8], index: usize) -> Option<Slot2018> {
    let base = HEADER + index * SLOT;
    if d.get(base..base + 256)?.iter().all(|&b| b == 0) {
        return None;
    }
    let time = u64::from_le_bytes(d.get(base..base + 8)?.try_into().ok()?);
    if !(946_684_800..7_258_118_400).contains(&time) || u32_at(d, base + 8)? != 10 {
        return None;
    }
    let raw = d.get(base + 16..base + 256)?;
    let title = String::from_utf8_lossy(&raw[..raw.iter().position(|&b| b == 0).unwrap_or(raw.len())]).trim().to_string();
    let t = base + TABLE;
    if d.get(t - 20..t - 16)? != [0x5F, 0x22, 0x43, 0xC6] || hex(d.get(t..t + 8)?) != XP {
        return None;
    }
    let count = u32_at(d, t - 12)?;
    if count == 0 || count > 4096 || t + count * 16 > base + SLOT {
        return None;
    }
    let rows = (0..count).map(|i| (hex(&d[t + 16 * i..t + 16 * i + 8]), t + 16 * i + 8)).collect();
    Some(Slot2018 { index, time, title, rows })
}

pub fn parse_2018(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_2018(d) {
        return Err("not a God of War (2018) save".into());
    }
    let mut slots: Vec<Slot2018> = (0..SLOTS).filter_map(|i| slot_2018(d, i)).collect();
    if slots.is_empty() {
        return Err("no saved games in this file".into());
    }
    // Newest first, as the game lists them.
    slots.sort_by(|a, b| b.time.cmp(&a.time));
    let names = items_2018();
    let mut b = Builder::new(d);
    for s in &slots {
        let key = format!("Slot {}", s.index + 1);
        let k = key.as_str();
        b.info(&[k, "Saved"], date(s.time));
        b.info(&[k, "Title"], s.title.clone());
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for (id, _) in &s.rows {
            *seen.entry(id).or_default() += 1;
        }
        let mut owned: HashMap<String, Vec<String>> = HashMap::new();
        for (id, at) in &s.rows {
            let Some(item) = names.get(id) else { continue };
            // Rows that appear twice are left alone.
            let single = seen.get(id.as_str()) == Some(&1);
            match id.as_str() {
                XP if single => {
                    b.ranged(&[k, "Experience"], *at, Kind::U32, 0, i32::MAX as i64);
                }
                HACKSILVER if single => {
                    b.ranged(&[k, "Hacksilver"], *at, Kind::U32, 0, i32::MAX as i64);
                }
                _ if item.resource && single => {
                    b.ranged(&[k, "Resources", &item.name], *at, Kind::U32, 0, i32::MAX as i64);
                }
                _ if ["Chest armor", "Wrist armor", "Waist armor", "Weapon attachments", "Talismans"].contains(&item.category.as_str()) => {
                    owned.entry(item.category.clone()).or_default().push(item.name.clone());
                }
                _ => {}
            }
        }
        for (cat, mut list) in owned {
            list.sort();
            b.info(&[k, "Equipment owned", &cat], serde_json::Value::from(list));
        }
    }
    Ok(b.finish("God of War save", Checksum::None).with_note(
        "Every saved game in game.sav (newest first): XP, hacksilver and resources, and the equipment each one owns. \
         Each upgrade level of a piece is a separate entry. Pick the save by its title and date; the slot number can \
         differ from the game's list.",
    ))
}

// ------------------------------------------------------------------ God of War Ragnarök

const GEAR_BLOCK: usize = 525_376;
const HEALTH_BLOCK: usize = 787_520;
const EXTRA_GROUPS: [&str; 4] = ["CDE09762F1", "36148DA179", "2A7CA18363", "9534BCAF4C"];

pub fn looks_ragnarok(d: &[u8]) -> bool {
    (787_800..64 << 20).contains(&d.len())
        && u32_at(d, 8) == Some(8)
        && u32_at(d, 72) == Some(35)
        && u32_at(d, 16) == Some(696)
        && u16_at(d, GEAR_BLOCK) == Some(2)
        && u32_at(d, HEALTH_BLOCK) == Some(0xD31A_D457)
}

struct InvRow {
    id: String,
    at: usize,
    instance: usize,
    simple: bool,
}

pub fn parse_ragnarok(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_ragnarok(d) {
        return Err("not a God of War Ragnarök save".into());
    }
    let bad = || "this save's layout isn't one the editor knows".to_string();
    let raw = &d[80..336];
    let title = String::from_utf8_lossy(&raw[..raw.iter().position(|&b| b == 0).unwrap_or(raw.len())]).to_string();
    let valhalla = title.to_lowercase().contains("valhalla");
    let marker = find(d, &unhex("F6FB3126EEBBA180"), 0, 1_677_512).ok_or_else(bad)?;
    let pointers = *d.get(marker + 24).ok_or_else(bad)? as usize;
    if pointers > 2 {
        return Err(bad());
    }
    let inv_header = marker + 49 + pointers * 8;
    let inv_start = inv_header + 6;
    let inv_count = u32_at(d, inv_header + 2).ok_or_else(bad)?;
    if inv_count > 20_000 {
        return Err(bad());
    }
    // Rows that carry a list after them (the list's count is 16 bytes in).
    let mut groups: HashMap<usize, usize> = HashMap::new();
    for pat in EXTRA_GROUPS {
        if let Some(hit) = find(d, &unhex(pat), inv_start, GEAR_BLOCK) {
            let start = hit - 3;
            groups.insert(start, u32_at(d, start + 16).ok_or_else(bad)?);
        }
    }
    let mut rows = vec![];
    let mut cur = inv_start;
    for _ in 0..inv_count {
        let extra = groups.get(&cur).copied().unwrap_or(0);
        let r = d.get(cur..cur + 24).ok_or_else(bad)?;
        rows.push(InvRow {
            id: hex(&r[..8]),
            at: cur,
            instance: u32_at(r, 8).unwrap_or(0),
            simple: extra == 0 && r[16..24].iter().all(|&b| b == 0),
        });
        cur += 24 + extra * 8;
    }
    // The attributes table comes after the inventory block.
    let block_end = inv_start + u16_at(d, inv_header).ok_or_else(bad)? - 4;
    let count_at = if valhalla { find(d, &unhex("FA40764A30A428C3"), block_end, GEAR_BLOCK).ok_or_else(bad)? + 8 } else { block_end + 12 };
    let attr_count = u32_at(d, count_at).ok_or_else(bad)?;
    if attr_count > 20_000 || cur > block_end {
        return Err(bad());
    }
    let attr_start = count_at + 12;
    let names = items_ragnarok();
    let mut b = Builder::new(d);
    b.info(&["Save", "Title"], title.clone());
    b.info(&["Save", "Mode"], if valhalla { "Valhalla" } else { "Campaign" });
    let mut seen: HashMap<String, usize> = HashMap::new();
    let attrs: Vec<(String, usize)> = (0..attr_count).map(|i| (hex(&d[attr_start + 16 * i..attr_start + 16 * i + 8]), attr_start + 16 * i + 8)).collect();
    for (id, _) in &attrs {
        *seen.entry(id.clone()).or_default() += 1;
    }
    for (id, at) in &attrs {
        let (Some(item), Some(&1)) = (names.get(id), seen.get(id)) else { continue };
        let group = match item.category.as_str() {
            "Experience" => "Experience",
            "Currency" => "Currency",
            "Resources" | "Special resources" => "Resources",
            "Yggdrasil's Dew" => "Yggdrasil's Dew",
            _ => continue,
        };
        b.ranged(&[group, &item.name], *at, Kind::U32, 0, i32::MAX as i64);
    }
    // Health and rage.
    if let Some(m) = find(d, &unhex("2BC947DCFC98D20F"), HEALTH_BLOCK, d.len().saturating_sub(256)) {
        let at = if m >= 151 && u16_at(d, m - 151) == Some(1530) { m - 138 } else { m - 42 };
        b.ranged(&["Kratos", "Health"], at, Kind::F32, 0, 2_000_000_000);
        b.ranged(&["Kratos", "Rage"], at + 8, Kind::F32, 0, 2_000_000_000);
    }
    // Equipment levels (campaign saves): each piece can become the same piece at another level
    // that no other piece in the inventory already is.
    if !valhalla {
        let (fams, by_id) = families();
        let in_use: HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        let mut used_keys: HashMap<String, usize> = HashMap::new();
        for r in rows.iter().filter(|r| r.simple) {
            let Some(&fi) = by_id.get(&r.id) else { continue };
            let (name, cat, variants) = &fams[fi];
            if variants.len() < 2 || name.to_lowercase().contains("valhalla") {
                continue;
            }
            let options: Vec<(String, u64)> = variants
                .iter()
                .filter(|(id, _, _)| *id == r.id || !in_use.contains(id.as_str()))
                .filter_map(|(id, lv, ng)| {
                    let label = if *lv == 0 { "Base".to_string() } else { format!("Level {lv}") } + if *ng { " (NG+)" } else { "" };
                    Some((label, id_value(id)?))
                })
                .collect();
            if options.len() < 2 {
                continue;
            }
            let n = used_keys.entry(format!("{cat}/{name}")).or_default();
            *n += 1;
            let key = if *n == 1 { name.clone() } else { format!("{name} ({n})") };
            b.pick(&["Equipment", cat, &key], r.at, Kind::U64, &options);
            let _ = r.instance;
        }
    }
    b.info(&["Save", "Inventory entries"], inv_count as u64);
    Ok(b.finish("God of War Ragnarök save", Checksum::None).with_note(
        "XP for Kratos, Atreus and Freya, hacksilver and the other currencies, resources, Yggdrasil's dew, health and rage, \
         and the level of each piece of equipment (campaign saves): a piece keeps its place and links, only its level changes.",
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn tables_load() {
        assert_eq!(super::items_2018().get(super::HACKSILVER).map(|i| i.name.as_str()), Some("Hacksilver"));
        assert!(super::items_ragnarok().len() > 3000);
        assert!(super::families().0.len() > 500);
        assert_eq!(super::date(1_664_244_240), "2022-09-27 02:04 UTC");
    }

    /// Research aid: SAVE_EDITOR_GOW = a game.sav or GOWRSAVE file.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_GOW").unwrap()).unwrap();
        let doc = if super::looks_2018(&d) { super::parse_2018(&d) } else { super::parse_ragnarok(&d) }.unwrap();
        let v = serde_json::to_string_pretty(&doc.view).unwrap();
        println!("{} editable\n{}", doc.editable.len(), v.chars().take(3000).collect::<String>());
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}

#[cfg(test)]
mod peek {
    #[test]
    #[ignore]
    fn equipment() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_GOW").unwrap()).unwrap();
        let doc = super::parse_ragnarok(&d).unwrap();
        let eq = doc.view.pointer("/Equipment").unwrap();
        for (cat, m) in eq.as_object().unwrap() {
            println!("{cat}: {}", m.as_object().unwrap().iter().take(4).map(|(k, v)| format!("{k} = {v}")).collect::<Vec<_>>().join("; "));
        }
        let p = "/Equipment";
        let first = eq.as_object().unwrap().iter().next().unwrap();
        let (k, v) = first.1.as_object().unwrap().iter().next().unwrap();
        println!("first: {p}/{}/{k} = {v}, choices {:?}", first.0, doc.choices(&format!("{p}/{}/{k}", first.0)));
    }
}
