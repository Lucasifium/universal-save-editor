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
//!
//! **Adding equipment.** 2018: a piece is a 16-byte owned row (id, 1, its index); new rows go
//! after the last, everything after them in the slot's active section (0x10400..) moves up,
//! and the section's sizes and row count are updated. Pieces are added at their base level.
//! Ragnarök: a new piece copies the record of a piece of the same type you own, with the new
//! id and the next instance number, plus its instance in the owned list; only the active part
//! of the block moves, and the file's size counters are updated. Both are checked by reading
//! the result back before it is saved.

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
/// A 2018 item's internal name.
fn internal_2018(id: &str) -> Option<&'static str> {
    static T: OnceLock<HashMap<String, String>> = OnceLock::new();
    let t = T.get_or_init(|| {
        include_str!("data/gow2018_items.tsv")
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| {
                let c: Vec<&str> = l.split('\t').collect();
                Some((c.first()?.to_string(), c.get(2)?.to_string()))
            })
            .collect()
    });
    t.get(id).map(String::as_str)
}

fn items_ragnarok() -> &'static HashMap<String, Item> {
    static T: OnceLock<HashMap<String, Item>> = OnceLock::new();
    T.get_or_init(|| table(include_str!("data/gowr_items.tsv"), None, 2))
}

/// A Ragnarök equipment family: one piece at each of its levels.
pub struct Family {
    pub name: String,
    pub category: String,
    /// Can a new piece of this family be added (a known, simple record)?
    pub can_add: bool,
    /// (id, level label) from lowest to highest.
    pub variants: Vec<(String, String)>,
}

fn level_label(lv: &str) -> String {
    let ng = lv.ends_with('+');
    let n = lv.trim_end_matches('+');
    let base = if n == "0" { "Base".to_string() } else { format!("Level {n}") };
    if ng {
        base + " (NG+)"
    } else {
        base
    }
}

fn families() -> &'static (Vec<Family>, HashMap<String, usize>) {
    static T: OnceLock<(Vec<Family>, HashMap<String, usize>)> = OnceLock::new();
    T.get_or_init(|| {
        let mut list = vec![];
        let mut by_id = HashMap::new();
        for l in include_str!("data/gowr_families.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let c: Vec<&str> = l.split('\t').collect();
            let (Some(name), Some(cat), Some(add), Some(vs)) = (c.first(), c.get(1), c.get(2), c.get(3)) else { continue };
            let variants: Vec<(String, String)> = vs.split(',').filter_map(|v| v.split_once(':').map(|(id, lv)| (id.to_string(), level_label(lv)))).collect();
            for (id, _) in &variants {
                by_id.insert(id.clone(), list.len());
            }
            list.push(Family { name: name.to_string(), category: cat.to_string(), can_add: *add == "1", variants });
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

/// Pieces of equipment that can be added in 2018: (id, name, category, family), base level only.
fn addable_2018() -> &'static Vec<(String, String, String, String)> {
    static T: OnceLock<Vec<(String, String, String, String)>> = OnceLock::new();
    T.get_or_init(|| {
        let mut out = vec![];
        for l in include_str!("data/gow2018_items.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
            let c: Vec<&str> = l.split('\t').collect();
            let (Some(id), Some(name), Some(internal), Some(cat)) = (c.first(), c.get(1), c.get(2), c.get(3)) else { continue };
            let equipment = ["KratosArmorChest_", "KratosArmorWrist_", "KratosArmorLegs_", "WeaponComponent_Axe_", "WeaponComponent_Blades_", "KratosArmorTrinket_"]
                .iter()
                .any(|p| internal.starts_with(p));
            // Base pieces with a real name; upgrades, New Game+ copies and unnamed test entries are left out.
            if !equipment || internal.contains("_Reinforce") || internal.contains("_NGP") || name == internal || name.is_empty() || name.to_uppercase().contains("PLACEHOLDER") {
                continue;
            }
            let _ = cat;
            let cat = [
                ("KratosArmorChest_", "Chest armor"),
                ("KratosArmorWrist_", "Wrist armor"),
                ("KratosArmorLegs_", "Waist armor"),
                ("WeaponComponent_Axe_", "Axe pommels"),
                ("WeaponComponent_Blades_", "Blades pommels"),
                ("KratosArmorTrinket_", "Talismans"),
            ]
            .iter()
            .find(|(p, _)| internal.starts_with(p))
            .map_or("Other", |(_, c)| c);
            out.push((id.to_string(), name.to_string(), cat.to_string(), internal.to_string()));
        }
        out.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)));
        out
    })
}

/// A piece's family: its internal name without the upgrade step.
fn family_2018(internal: &str) -> &str {
    internal.find("_Reinforce").map_or(internal, |i| &internal[..i])
}

const ACTIVE: usize = 0x10400;
const ACTIVE_END: usize = 0x30400;
const MARGIN: usize = 4096;
const MAX_ADD: usize = 32;

/// Where a 2018 slot's row table ends and its active section ends, after checking the layout.
fn active_2018(d: &[u8], s: &Slot2018) -> Result<(usize, usize, usize), String> {
    let bad = || "this save's layout isn't one the editor can add to".to_string();
    let base = HEADER + s.index * SLOT;
    let table = base + TABLE;
    let stats = table - 24;
    let ok = stats == base + ACTIVE + 5
        && d.get(base + ACTIVE..base + ACTIVE + 2) == Some(&[1, 0][..])
        && d.get(base + ACTIVE + 3..base + ACTIVE + 5) == Some(&[0xFA, 0xFF][..])
        && d.get(stats + 4..stats + 12) == Some(&unhex("5F2243C695480000")[..]);
    if !ok {
        return Err(bad());
    }
    let stats_end = stats + 4 + u32_at(d, stats).ok_or_else(bad)?;
    let table_end = table + s.rows.len() * 16;
    let mut cur = stats_end;
    for _ in 0..3 {
        cur += 2 + u16_at(d, cur).ok_or_else(bad)?;
    }
    if d.get(cur..cur + 2) != Some(&[0x2E, 0xFA][..]) {
        return Err(bad());
    }
    let end = cur + 4 + u16_at(d, cur + 2).ok_or_else(bad)?;
    if stats_end <= table_end || end != base + ACTIVE + u32_at(d, base + 0x124).ok_or_else(bad)? || end > base + ACTIVE_END {
        return Err(bad());
    }
    Ok((stats, table_end, end))
}

fn post_2018(out: &mut Vec<u8>, chosen: &[(std::sync::Arc<str>, String)]) -> Result<(), String> {
    // tag = "slot|id"
    let mut by_slot: HashMap<usize, Vec<String>> = HashMap::new();
    for (tag, choice) in chosen {
        if choice != "Add" {
            continue;
        }
        let (slot, id) = tag.split_once('|').ok_or("bad request")?;
        by_slot.entry(slot.parse().map_err(|_| "bad request")?).or_default().push(id.to_string());
    }
    for (index, ids) in by_slot {
        let s = slot_2018(out, index).ok_or("this save slot can't be read")?;
        let (stats, table_end, end) = active_2018(out, &s)?;
        let have: HashSet<&str> = s.rows.iter().map(|(id, _)| id.as_str()).collect();
        let ids: Vec<String> = ids.into_iter().filter(|id| !have.contains(id.as_str())).collect();
        if ids.is_empty() {
            continue;
        }
        if ids.len() > MAX_ADD || s.rows.len() + ids.len() > 4096 {
            return Err(format!("add at most {MAX_ADD} pieces at a time"));
        }
        let base = HEADER + index * SLOT;
        let grow = ids.len() * 16;
        if end + grow + MARGIN > base + ACTIVE_END {
            return Err("this save has no room left for more equipment".into());
        }
        let tail = out[table_end..end].to_vec();
        out[table_end + grow..end + grow].copy_from_slice(&tail);
        for (k, id) in ids.iter().enumerate() {
            let at = table_end + 16 * k;
            out[at..at + 8].copy_from_slice(&unhex(id));
            out[at + 8..at + 12].copy_from_slice(&1u32.to_le_bytes());
            out[at + 12..at + 16].copy_from_slice(&((s.rows.len() + k) as u32).to_le_bytes());
        }
        let bump = |out: &mut Vec<u8>, at: usize, by: usize| -> Result<(), String> {
            let v = u32_at(out, at).ok_or("bad size")? + by;
            out[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
            Ok(())
        };
        bump(out, stats, grow)?;
        bump(out, stats + 12, ids.len())?;
        bump(out, base + 0x124, grow)?;
        // Read it back: every piece is there once and the section still lines up.
        let again = slot_2018(out, index).ok_or("the slot doesn't read back after adding")?;
        active_2018(out, &again)?;
        for id in &ids {
            if again.rows.iter().filter(|(r, _)| r == id).count() != 1 {
                return Err("an added piece didn't read back".into());
            }
        }
    }
    Ok(())
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
        // Equipment this save doesn't have yet (any level), if its layout allows adding.
        if active_2018(d, s).is_ok() {
            let owned_families: HashSet<String> = s
                .rows
                .iter()
                .filter_map(|(id, _)| internal_2018(id))
                .map(|n| family_2018(n).to_string())
                .collect();
            for (id, name, cat, internal) in addable_2018() {
                if owned_families.contains(family_2018(internal)) {
                    continue;
                }
                b.option(&[k, "Add equipment", cat, name], vec!["Don't add".into(), "Add".into()], format!("{}|{id}", s.index));
            }
        }
    }
    Ok(b.finish_with("God of War save", Checksum::None, Some(post_2018)).with_note(
        "Every saved game in game.sav (newest first): XP, hacksilver and resources, and the equipment each one owns. \
         Each upgrade level of a piece is a separate entry. Pick the save by its title and date; the slot number can \
         differ from the game's list.",
    ))
}

// ------------------------------------------------------------------ God of War Ragnarök

const GEAR_BLOCK: usize = 525_376;
const HEALTH_BLOCK: usize = 787_520;
const EXTRA_GROUPS: [&str; 4] = ["CDE09762F1", "36148DA179", "2A7CA18363", "9534BCAF4C"];
const BLOCK_START: usize = 1088;

/// The type a new piece must share with the piece it's copied from.
fn kind_ragnarok(f: &Family) -> Option<&'static str> {
    if !f.can_add {
        return None;
    }
    let has = |w: &str| f.name.split_whitespace().any(|x| x == w);
    Some(match f.category.as_str() {
        "Chest armor" => "chest",
        "Wrist armor" => "wrist",
        "Waist armor" => "waist",
        "Shields" => "shield",
        "Weapon attachments" if f.name.starts_with("Rond") => "shield attachment",
        "Weapon attachments" if has("Hind") || has("Sauroter") => "spear attachment",
        "Weapon attachments" if has("Handles") || has("Pommels") => "blades attachment",
        "Weapon attachments" if has("Knob") || has("Grip") || f.name == "The Furious Maul" => "axe attachment",
        _ => return None,
    })
}

/// Everything adding a piece needs to know about a campaign save.
struct AddRagnarok {
    inv_header: usize,
    inv_end: usize,
    stats_header: usize,
    owned_end: usize,
    active_end: usize,
    max_uid: usize,
    /// (kind, offset of a piece that can be copied)
    templates: Vec<(&'static str, usize)>,
    capacity: usize,
}

fn add_ragnarok(d: &[u8], l: &Layout) -> Result<AddRagnarok, String> {
    let bad = |what: &str| format!("adding equipment isn't possible in this save ({what})");
    let inv_size = u16_at(d, l.inv_header).ok_or_else(|| bad("inventory"))?;
    let stats_header = l.attr_start - 24;
    let stats_size = u32_at(d, stats_header).ok_or_else(|| bad("attributes"))?;
    if l.inv_header + inv_size + 2 != stats_header || u32_at(d, stats_header + 12) != Some(l.attr_count) {
        return Err(bad("table limits"));
    }
    let (count8, count4) = (u32_at(d, stats_header + 16).ok_or_else(|| bad("refs"))?, u32_at(d, stats_header + 20).ok_or_else(|| bad("refs"))?);
    let owned_start = l.attr_start + l.attr_count * 16 + count8 * 8;
    let owned_end = owned_start + count4 * 4;
    // The sections after the attributes: events, talismans, settings, ids, counters, an
    // empty one and the last; together they end where the header says the block's data ends.
    let mut cur = stats_header + stats_size + 4;
    let mut frames = vec![];
    for width in [2usize, 2, 2, 2, 4, 1, 4] {
        let len = match width {
            1 => *d.get(cur).ok_or_else(|| bad("sections"))? as usize,
            2 => u16_at(d, cur).ok_or_else(|| bad("sections"))?,
            _ => u32_at(d, cur).ok_or_else(|| bad("sections"))?,
        };
        frames.push((cur + width, len));
        cur += width + len;
        if cur > GEAR_BLOCK {
            return Err(bad("sections"));
        }
    }
    let total = u32_at(d, 24).ok_or_else(|| bad("header"))?;
    let parts: Vec<usize> = [408, 412, 416].iter().filter_map(|&o| u32_at(d, o)).collect();
    let (ev, tal, _, ids, counters, opt, fin) = (frames[0], frames[1], frames[2], frames[3], frames[4], frames[5], frames[6]);
    let ok = parts.len() == 3
        && total == parts.iter().sum::<usize>()
        && cur == BLOCK_START + parts[1]
        && ev.1 >= 4
        && Some(ev.1) == u32_at(d, ev.0).map(|n| 4 + n * 32)
        && tal.1 == 2208
        && d.get(tal.0..tal.0 + 20) == Some(&unhex("FFFFFFFFFFFFFFFF0100000005000000C65F8613")[..])
        && ids.1 >= 6
        && (ids.1 - 6) % 8 == 0
        && counters.1 == 72_520
        && opt.1 == 0
        && Some(fin.1) == u32_at(d, fin.0).map(|n| 4 + n * 8);
    if !ok {
        return Err(bad("sections"));
    }
    let uids: HashSet<usize> = l.rows.iter().map(|r| r.instance).collect();
    let owned: Vec<usize> = (0..count4).filter_map(|i| u32_at(d, owned_start + 4 * i)).collect();
    if uids.len() != l.rows.len() || owned.iter().any(|u| !uids.contains(u)) || owned.iter().collect::<HashSet<_>>().len() != owned.len() {
        return Err(bad("owned list"));
    }
    let owned: HashSet<usize> = owned.into_iter().collect();
    let (fams, by_id) = families();
    let mut count: HashMap<&str, usize> = HashMap::new();
    for r in &l.rows {
        *count.entry(r.id.as_str()).or_default() += 1;
    }
    let mut templates = vec![];
    for r in &l.rows {
        let Some(kind) = by_id.get(&r.id).and_then(|&i| kind_ragnarok(&fams[i])) else { continue };
        if r.simple && owned.contains(&r.instance) && d.get(r.at + 12..r.at + 16) == Some(&[0xFF; 4][..]) && count.get(r.id.as_str()) == Some(&1) {
            templates.push((kind, r.at));
        }
    }
    let max_uid = l.rows.iter().map(|r| r.instance).max().unwrap_or(0);
    let capacity = MAX_ADD
        .min(GEAR_BLOCK.saturating_sub(cur + MARGIN) / 28)
        .min(65_535usize.saturating_sub(inv_size) / 24)
        .min(20_000usize.saturating_sub(l.rows.len()));
    Ok(AddRagnarok { inv_header: l.inv_header, inv_end: l.inv_end, stats_header, owned_end, active_end: cur, max_uid, templates, capacity })
}

fn post_ragnarok(out: &mut Vec<u8>, chosen: &[(std::sync::Arc<str>, String)]) -> Result<(), String> {
    let (fams, _) = families();
    let l = layout_ragnarok(out)?;
    let st = add_ragnarok(out, &l)?;
    let owned_ids: HashSet<&str> = l.rows.iter().map(|r| r.id.as_str()).collect();
    let mut plan: Vec<(String, usize)> = vec![]; // (new id, template offset)
    for (tag, choice) in chosen {
        if choice == "Don't add" {
            continue;
        }
        let fi: usize = tag.strip_prefix('R').and_then(|n| n.parse().ok()).ok_or("bad request")?;
        let f = fams.get(fi).ok_or("bad request")?;
        let id = f.variants.iter().find(|(_, lv)| lv == choice).map(|(id, _)| id.clone()).ok_or("pick one of the listed levels")?;
        if f.variants.iter().any(|(v, _)| owned_ids.contains(v.as_str())) {
            continue; // already owned (e.g. added in the same save)
        }
        let kind = kind_ragnarok(f).ok_or("this piece can't be added")?;
        let &(_, template) = st.templates.iter().find(|(k, _)| *k == kind).ok_or_else(|| format!("to add {}, you first need any piece of that type in the game", f.name))?;
        plan.push((id, template));
    }
    if plan.is_empty() {
        return Ok(());
    }
    if plan.len() > st.capacity {
        return Err(format!("add at most {} pieces at a time", st.capacity));
    }
    let n = plan.len();
    let (items, refs) = (24 * n, 4 * n);
    let original = out.clone();
    // Only the active part moves: [inv_end, owned_end) by the new records, and
    // [owned_end, active_end) by the records and references together.
    out[st.owned_end + items + refs..st.active_end + items + refs].copy_from_slice(&original[st.owned_end..st.active_end]);
    out[st.inv_end + items..st.owned_end + items].copy_from_slice(&original[st.inv_end..st.owned_end]);
    for (k, (id, template)) in plan.iter().enumerate() {
        let at = st.inv_end + 24 * k;
        let uid = st.max_uid + 1 + k;
        out[at..at + 24].copy_from_slice(&original[*template..template + 24]);
        out[at..at + 8].copy_from_slice(&unhex(id));
        out[at + 8..at + 12].copy_from_slice(&(uid as u32).to_le_bytes());
        let r = st.owned_end + items + 4 * k;
        out[r..r + 4].copy_from_slice(&(uid as u32).to_le_bytes());
    }
    let set16 = |out: &mut Vec<u8>, at: usize, v: usize| out[at..at + 2].copy_from_slice(&(v as u16).to_le_bytes());
    let set32 = |out: &mut Vec<u8>, at: usize, v: usize| out[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
    let get = |at: usize| u32_at(&original, at).unwrap_or(0);
    set16(out, st.inv_header, u16_at(&original, st.inv_header).unwrap_or(0) + items);
    set32(out, st.inv_header + 2, get(st.inv_header + 2) + n);
    let stats = st.stats_header + items;
    set32(out, stats, get(st.stats_header) + refs);
    set32(out, stats + 20, get(st.stats_header + 20) + n);
    set32(out, 24, get(24) + items + refs);
    set32(out, 412, get(412) + items + refs);
    set32(out, 444, get(444) + 2 * n);
    // Read it back.
    let again = layout_ragnarok(out)?;
    let st2 = add_ragnarok(out, &again)?;
    if again.rows.len() != l.rows.len() + n || st2.active_end != st.active_end + items + refs || out[st2.active_end..] != original[st2.active_end..] {
        return Err("the added equipment didn't read back; nothing was saved".into());
    }
    for (k, (id, _)) in plan.iter().enumerate() {
        if !again.rows.iter().any(|r| r.id == *id && r.instance == st.max_uid + 1 + k) {
            return Err("an added piece didn't read back; nothing was saved".into());
        }
    }
    Ok(())
}

/// Where the Ragnarök blocks are (shared by reading and adding).
struct Layout {
    title: String,
    valhalla: bool,
    rows: Vec<InvRow>,
    inv_header: usize,
    inv_end: usize,
    attr_start: usize,
    attr_count: usize,
}

fn layout_ragnarok(d: &[u8]) -> Result<Layout, String> {
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
        rows.push(InvRow { id: hex(&r[..8]), at: cur, instance: u32_at(r, 8).unwrap_or(0), simple: extra == 0 && r[16..24].iter().all(|&b| b == 0) });
        cur += 24 + extra * 8;
    }
    // The attributes table comes after the inventory block.
    let block_end = inv_start + u16_at(d, inv_header).ok_or_else(bad)? - 4;
    let count_at = if valhalla { find(d, &unhex("FA40764A30A428C3"), block_end, GEAR_BLOCK).ok_or_else(bad)? + 8 } else { block_end + 12 };
    let attr_count = u32_at(d, count_at).ok_or_else(bad)?;
    if attr_count > 20_000 || cur > block_end || count_at + 12 + attr_count * 16 > GEAR_BLOCK {
        return Err(bad());
    }
    Ok(Layout { title, valhalla, rows, inv_header, inv_end: cur, attr_start: count_at + 12, attr_count })
}

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
    let l = layout_ragnarok(d)?;
    let (title, valhalla, rows, inv_count, attr_count) = (l.title.clone(), l.valhalla, &l.rows, l.rows.len(), l.attr_count);
    let attr_start = l.attr_start;
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
            let f = &fams[fi];
            let (name, cat) = (&f.name, &f.category);
            if f.variants.len() < 2 || name.to_lowercase().contains("valhalla") {
                continue;
            }
            let options: Vec<(String, u64)> = f
                .variants
                .iter()
                .filter(|(id, _)| *id == r.id || !in_use.contains(id.as_str()))
                .filter_map(|(id, label)| Some((label.clone(), id_value(id)?)))
                .collect();
            if options.len() < 2 {
                continue;
            }
            let n = used_keys.entry(format!("{cat}/{name}")).or_default();
            *n += 1;
            let key = if *n == 1 { name.clone() } else { format!("{name} ({n})") };
            b.pick(&["Equipment", cat, &key], r.at, Kind::U64, &options);
        }
    }
    // Equipment that can be added: pieces this save has at no level, of a type it already has
    // a piece of (that piece is copied).
    if !valhalla {
        if let Ok(st) = add_ragnarok(d, &l) {
            let (fams, _) = families();
            let owned: HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
            let kinds: HashSet<&str> = st.templates.iter().map(|(k, _)| *k).collect();
            let mut order: Vec<usize> = (0..fams.len()).collect();
            order.sort_by(|&a, &b| fams[a].category.cmp(&fams[b].category).then(fams[a].name.cmp(&fams[b].name)));
            let mut used: HashSet<String> = HashSet::new();
            for fi in order {
                let f = &fams[fi];
                let Some(kind) = kind_ragnarok(f) else { continue };
                // Appearance-only and placeholder entries aren't pieces the game lets you own this way.
                let odd = f.name.contains("(Transmog)") || f.name.starts_with("An Extremely Long") || f.name.to_lowercase().contains("valhalla");
                if !kinds.contains(kind) || odd || f.variants.iter().any(|(id, _)| owned.contains(id.as_str())) {
                    continue;
                }
                if !used.insert(format!("{}/{}", f.category, f.name)) {
                    continue;
                }
                let mut names = vec!["Don't add".to_string()];
                names.extend(f.variants.iter().filter(|(_, lv)| lv.starts_with("Level")).map(|(_, lv)| lv.clone()));
                if names.len() < 2 {
                    continue;
                }
                b.option(&["Add equipment", &f.category, &f.name], names, format!("R{fi}"));
            }
        }
    }
    b.info(&["Save", "Inventory entries"], inv_count as u64);
    Ok(b.finish_with("God of War Ragnarök save", Checksum::None, Some(post_ragnarok)).with_note(
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
mod peek_add {
    #[test]
    #[ignore]
    fn list() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_GOW").unwrap()).unwrap();
        let doc = if super::looks_2018(&d) { super::parse_2018(&d) } else { super::parse_ragnarok(&d) }.unwrap();
        let mut ptrs: Vec<&String> = doc.editable.keys().filter(|p| p.contains("Add equipment")).collect();
        ptrs.sort();
        println!("{} add choices", ptrs.len());
        for p in ptrs.iter().take(6) {
            println!("{p} {:?}", doc.choices(p).map(|c| c.iter().take(4).cloned().collect::<Vec<_>>()));
        }
    }
}
