//! The Planet Crafter (`Survival-<n>.json`, `Backup.json`): not one JSON document but
//! sections of JSON records. Sections are separated by a line holding only `@`; records
//! inside a section end with `|` and a line break (the last one has no `|`).
//!
//! Sections in order: the progress record (Terra tokens, `unlockedGroups`), one record per
//! planet (the six terraformation units), the players, every world object (`id` and `gId`,
//! plus position or container for placed ones), the item groups (`id` 1 is the backpack,
//! `id` 2 the worn equipment; `woIds` lists the item ids in it and `size` is its slot count),
//! the stats, messages, story events, the game settings and more.
//!
//! The record layout is described by the community (savegame notes by Diego Martín and the
//! game wiki); the fields used here were checked on a real late-game save. Every item takes
//! one slot, so a material "count" is how many objects with that `gId` sit in the backpack:
//! raising it writes new objects and lists their ids in the backpack, lowering it removes
//! the last ones. Everything else is rewritten in place and the rest of the file stays byte
//! for byte.

use crate::tree::{child, text_to_value, value_to_text, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

pub const KIND: &str = "The Planet Crafter save";

/// Raw materials the Backpack list shows (the `gId` each is stored under) and what they are
/// called in the game. Seeds, food, fuses, tokens, key cards and chips are left out.
pub const MATERIALS: [(&str, &str); 19] = [
    ("Iron", "Iron"),
    ("Cobalt", "Cobalt"),
    ("Silicon", "Silicon"),
    ("Titanium", "Titanium"),
    ("Magnesium", "Magnesium"),
    ("Aluminium", "Aluminium"),
    ("Iridium", "Iridium"),
    ("Alloy", "Super alloy"),
    ("Zeolite", "Zeolite"),
    ("Sulfur", "Sulfur"),
    ("Osmium", "Osmium"),
    ("Uranim", "Uranim"),
    ("Obsidian", "Obsidian"),
    ("Selenium", "Selenium"),
    ("Phosphorus", "Phosphorus"),
    ("Dolomite", "Dolomite"),
    ("Bauxite", "Bauxite"),
    ("Uraninite", "Uraninite"),
    ("ice", "Ice"),
];

#[derive(Clone, Debug)]
enum Leaf {
    Scalar { start: usize, end: usize, orig: String },
    Material { gid: String },
}

#[derive(Clone, Debug)]
struct Line {
    start: usize,
    /// End including the `|` and line break.
    end: usize,
    /// Has the `|` after it (the section's last record doesn't, and can't be cut out alone).
    piped: bool,
}

#[derive(Clone, Debug)]
struct PcBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
    /// Where new object records go (the start of the objects section).
    objects_at: usize,
    /// The backpack's `woIds` value (between the quotes).
    woids: (usize, usize),
    pack: Vec<u64>,
    /// Backpack item id → its `gId`, and where its record is.
    gid_of: HashMap<u64, String>,
    lines: HashMap<u64, Line>,
    used_ids: HashSet<u64>,
}

impl PcBackend {
    fn fresh_ids(&self, n: usize, taken: &mut HashSet<u64>) -> Vec<u64> {
        let mut seed: u64 = self.used_ids.len() as u64 ^ 0x9E37_79B9;
        let mut out = vec![];
        while out.len() < n {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let id = 100_000_000 + (seed >> 33) % 110_000_000;
            if !self.used_ids.contains(&id) && taken.insert(id) {
                out.push(id);
            }
        }
        out
    }
}

impl Backend for PcBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        let mut mats: Vec<(&str, usize)> = vec![];
        for (id, v) in changes {
            match &self.leaves[*id] {
                Leaf::Scalar { start, end, orig } => p.add(*start, *end, value_to_text(orig, v).into_bytes()),
                Leaf::Material { gid } => mats.push((gid, v.as_u64().ok_or("not a whole number")? as usize)),
            }
        }
        if !mats.is_empty() {
            let mut pack = self.pack.clone();
            let mut taken = HashSet::new();
            let mut inserts = vec![];
            let mut cuts = vec![];
            for (gid, want) in mats {
                let have: Vec<u64> = pack.iter().copied().filter(|i| self.gid_of.get(i).is_some_and(|g| g == gid)).collect();
                if want > have.len() {
                    for id in self.fresh_ids(want - have.len(), &mut taken) {
                        pack.push(id);
                        inserts.push(format!("{{\"id\":{id},\"gId\":\"{gid}\"}}|\n"));
                    }
                } else {
                    // Remove from the end, leaving alone any record that can't be cut out.
                    let mut drop = have.len() - want;
                    for id in have.iter().rev() {
                        if drop == 0 {
                            break;
                        }
                        let Some(l) = self.lines.get(id) else { continue };
                        if !l.piped {
                            continue;
                        }
                        cuts.push((l.start, l.end));
                        pack.retain(|x| x != id);
                        drop -= 1;
                    }
                    if drop > 0 {
                        return Err(format!("couldn't remove {drop} of the {gid} items"));
                    }
                }
            }
            if !inserts.is_empty() {
                p.add(self.objects_at, self.objects_at, inserts.concat().into_bytes());
            }
            for (s, e) in cuts {
                // A cut can start exactly where the inserted text goes; insert first.
                p.add(s, e, vec![]);
            }
            let list: Vec<String> = pack.iter().map(u64::to_string).collect();
            p.add(self.woids.0, self.woids.1, list.join(",").into_bytes());
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match &self.leaves[leaf] {
            Leaf::Material { .. } => match v.as_u64() {
                Some(n) if n <= 100_000 => Ok(()),
                _ => Err("A material count is a whole number from 0 to 100000".into()),
            },
            Leaf::Scalar { orig, .. } => {
                if orig == "true" || orig == "false" {
                    return v.as_bool().map(|_| ()).ok_or_else(|| "true or false".into());
                }
                match v.as_f64() {
                    Some(n) if n.is_finite() && n >= 0.0 => Ok(()),
                    _ => Err("Enter a number that isn't negative".into()),
                }
            }
        }
    }

    fn reshapes(&self, leaf: usize) -> bool {
        matches!(self.leaves[leaf], Leaf::Material { .. })
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// The text starts with the progress record and has sections.
pub fn looks(text: &str) -> bool {
    separator(text).is_some() && text.trim_start().starts_with("{\"terraTokens\":")
}

/// The line holding only `@` between sections: the game writes it with carriage returns
/// (`\r@\r`); line breaks work too.
fn separator(text: &str) -> Option<&'static str> {
    ["\r@\r{\"planetId\"", "\n@\n{\"planetId\""].into_iter().find(|s| text.contains(s)).map(|s| &s[..3])
}

/// Byte ranges of the sections (the text between the `@` lines).
fn sections(text: &str) -> Vec<(usize, usize)> {
    let sep = separator(text).unwrap_or("\n@\n");
    let mut out = vec![];
    let mut at = 0;
    while let Some(i) = text[at..].find(sep) {
        out.push((at, at + i));
        at += i + 3;
    }
    out.push((at, text.len()));
    out
}

/// Records of a section: (start, end incl. `|`+line break, content without the `|`).
fn records(text: &str, (s, e): (usize, usize)) -> Vec<(usize, usize, &str)> {
    let mut out = vec![];
    let mut off = s;
    for line in text[s..e].split_inclusive('\n') {
        let start = off;
        off += line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let body = body.strip_suffix('|').unwrap_or(body);
        if body.trim().starts_with('{') {
            out.push((start, off, body));
        }
    }
    out
}

/// Where `"key":<value>` has its value, as absolute byte positions.
fn value_span(base: usize, rec: &str, key: &str) -> Option<(usize, usize)> {
    let pat = format!("\"{key}\":");
    let i = rec.find(&pat)? + pat.len();
    let rest = &rec[i..];
    if rest.starts_with('"') {
        let j = rest[1..].find('"')?;
        return Some((base + i, base + i + j + 2));
    }
    let j = rest.find([',', '}']).unwrap_or(rest.len());
    Some((base + i, base + i + j))
}

fn num_key(
    root: &mut Map<String, Value>,
    path: &[&str],
    leaves: &mut Vec<Leaf>,
    editable: &mut HashMap<String, usize>,
    base: usize,
    rec: &str,
    key: &str,
) {
    let Some((s, e)) = value_span(base, rec, key) else { return };
    let orig = rec_slice(base, rec, s, e);
    if orig.starts_with('"') {
        return;
    }
    let v = serde_json::from_str::<Value>(&orig).unwrap_or_else(|_| text_to_value(&orig));
    let mut m = root;
    for (i, k) in path.iter().enumerate() {
        if i + 1 == path.len() {
            m.insert((*k).into(), v);
            break;
        }
        let e = m.entry(*k).or_insert_with(|| Value::Object(Map::new()));
        m = e.as_object_mut().unwrap();
    }
    let ptr = path.iter().fold(String::new(), |p, k| child(&p, k));
    editable.insert(ptr, leaves.len());
    leaves.push(Leaf::Scalar { start: s, end: e, orig });
}

fn rec_slice(base: usize, rec: &str, s: usize, e: usize) -> String {
    rec[s - base..e - base].to_string()
}

fn int_after(rec: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = rec.find(&pat)? + pat.len();
    let t = &rec[i..];
    t[..t.find([',', '}']).unwrap_or(t.len())].trim().parse().ok()
}

fn str_after<'a>(rec: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":\"");
    let i = rec.find(&pat)? + pat.len();
    let t = &rec[i..];
    Some(&t[..t.find('"')?])
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not a Planet Crafter save".into());
    }
    let secs = sections(text);
    if secs.len() < 9 {
        return Err("a Planet Crafter save has at least nine sections".into());
    }
    let mut root = Map::new();
    let mut leaves: Vec<Leaf> = vec![];
    let mut editable: HashMap<String, usize> = HashMap::new();

    // Progress.
    let head = records(text, secs[0]);
    let (hs, _, hb) = *head.first().ok_or("no progress record")?;
    for (key, name) in [("terraTokens", "Terra tokens"), ("allTimeTerraTokens", "Terra tokens earned in all")] {
        num_key(&mut root, &["Progress", name], &mut leaves, &mut editable, hs, hb, key);
    }
    let unlocked = str_after(hb, "unlockedGroups").map_or(0, |s| s.split(',').filter(|x| !x.is_empty()).count());
    root.entry("Progress").or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap().insert("Blueprints unlocked".into(), Value::from(unlocked));

    // Planets.
    for (start, _, rec) in records(text, secs[1]) {
        let Some(id) = str_after(rec, "planetId") else { continue };
        let planet = format!("Planet {id}");
        for (key, name) in [
            ("unitOxygenLevel", "Oxygen"),
            ("unitHeatLevel", "Heat"),
            ("unitPressureLevel", "Pressure"),
            ("unitPlantsLevel", "Plants"),
            ("unitInsectsLevel", "Insects"),
            ("unitAnimalsLevel", "Animals"),
        ] {
            num_key(&mut root, &[&planet, name], &mut leaves, &mut editable, start, rec, key);
        }
    }

    // The player.
    if let Some((start, _, rec)) = records(text, secs[2]).first().copied() {
        for (key, name) in [("playerGaugeOxygen", "Oxygen"), ("playerGaugeThirst", "Thirst"), ("playerGaugeHealth", "Health")] {
            num_key(&mut root, &["Player", name], &mut leaves, &mut editable, start, rec, key);
        }
        if let Some(n) = str_after(rec, "name") {
            root.entry("Player").or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap().insert("Name".into(), Value::from(n));
        }
        if let Some(n) = str_after(rec, "planetId") {
            root.entry("Player").or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap().insert("On planet".into(), Value::from(n));
        }
    }

    // Objects: ids, and where the records are.
    let objs = records(text, secs[3]);
    let objects_at = secs[3].0;
    let mut gid_all: HashMap<u64, String> = HashMap::new();
    let mut lines: HashMap<u64, Line> = HashMap::new();
    let mut used_ids: HashSet<u64> = HashSet::new();
    let n_objs = objs.len();
    for (k, (s, e, rec)) in objs.iter().enumerate() {
        let (Some(id), Some(g)) = (int_after(rec, "id"), str_after(rec, "gId")) else { continue };
        used_ids.insert(id);
        let plain = rec.matches(',').count() == 1;
        gid_all.insert(id, g.to_string());
        if plain {
            lines.insert(id, Line { start: *s, end: *e, piped: k + 1 < n_objs });
        }
    }

    // Item groups: id 1 is the backpack.
    let mut woids = None;
    let mut pack: Vec<u64> = vec![];
    let mut size = 0usize;
    for (start, _, rec) in records(text, secs[4]) {
        if int_after(rec, "id") == Some(1) {
            woids = value_span(start, rec, "woIds").map(|(s, e)| (s + 1, e - 1));
            pack = str_after(rec, "woIds").unwrap_or("").split(',').filter_map(|x| x.parse().ok()).collect();
            size = int_after(rec, "size").unwrap_or(0) as usize;
        }
    }
    let woids = woids.ok_or("no backpack in this save")?;
    let gid_of: HashMap<u64, String> = pack.iter().filter_map(|i| gid_all.get(i).map(|g| (*i, g.clone()))).collect();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for g in gid_of.values() {
        *counts.entry(g.as_str()).or_default() += 1;
    }
    let mut bp = Map::new();
    for (gid, name) in MATERIALS {
        bp.insert(name.into(), Value::from(counts.get(gid).copied().unwrap_or(0)));
        editable.insert(child("/Backpack", name), leaves.len());
        leaves.push(Leaf::Material { gid: gid.into() });
    }
    bp.insert("Slots".into(), Value::from(size));
    bp.insert("Items carried".into(), Value::from(pack.len()));
    root.insert("Backpack".into(), Value::Object(bp));

    // Stats.
    if let Some((start, _, rec)) = records(text, secs[5]).first().copied() {
        num_key(&mut root, &["Stats", "Objects crafted"], &mut leaves, &mut editable, start, rec, "craftedObjects");
    }

    // Game settings.
    if let Some((start, _, rec)) = records(text, secs[8]).first().copied() {
        for (key, name) in [
            ("freeCraft", "Free crafting"),
            ("unlockedEverything", "Everything unlocked"),
            ("unlockedSpaceTrading", "Space trading unlocked"),
            ("unlockedOreExtrators", "Ore extractors unlocked"),
            ("unlockedTeleporters", "Teleporters unlocked"),
            ("unlockedDrones", "Drones unlocked"),
            ("unlockedAutocrafter", "Autocrafter unlocked"),
        ] {
            if let Some((s, e)) = value_span(start, rec, key) {
                let orig = rec_slice(start, rec, s, e);
                if orig == "true" || orig == "false" {
                    root.entry("Settings").or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap().insert(name.into(), Value::Bool(orig == "true"));
                    editable.insert(child("/Settings", name), leaves.len());
                    leaves.push(Leaf::Scalar { start: s, end: e, orig });
                }
            }
        }
        for (key, name) in [("mode", "Mode"), ("dyingConsequencesLabel", "When you die"), ("startLocationLabel", "Start location")] {
            if let Some(v) = str_after(rec, key) {
                root.entry("Settings").or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap().insert(name.into(), Value::from(v));
            }
        }
    }

    let backend = PcBackend { raw: text.as_bytes().to_vec(), leaves, objects_at, woids, pack, gid_of, lines, used_ids };
    let doc = TreeDoc::new(Value::Object(root), editable, KIND, Box::new(backend));
    Ok(doc.with_note(
        "Terra tokens, terraformation levels, your gauges, the game settings and the raw materials in your backpack. \
         Every item takes one backpack slot, so a material count can't go past the free slots. Close the game before saving.",
    ))
}

/// Free backpack slots in an opened save.
pub fn free_slots(view: &Value) -> i64 {
    let n = |k: &str| view.pointer(&format!("/Backpack/{k}")).and_then(Value::as_i64).unwrap_or(0);
    n("Slots") - n("Items carried")
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &str = "\n{\"terraTokens\":500,\"allTimeTerraTokens\":900,\"unlockedGroups\":\"A,B,C\"}\n@\n\
{\"planetId\":\"Prime\",\"unitOxygenLevel\":100.0,\"unitHeatLevel\":2.5,\"unitPressureLevel\":3.0,\"unitPlantsLevel\":4.0,\"unitInsectsLevel\":5.0,\"unitAnimalsLevel\":6.0}\n@\n\
{\"id\":7,\"name\":\"Me\",\"inventoryId\":1,\"equipmentId\":2,\"playerGaugeOxygen\":4300.0,\"playerGaugeThirst\":75.5,\"playerGaugeHealth\":88.6,\"planetId\":\"Prime\"}\n@\n\
{\"id\":101,\"gId\":\"Iron\"}|\n{\"id\":102,\"gId\":\"Iron\"}|\n{\"id\":103,\"gId\":\"Silicon\"}|\n{\"id\":104,\"gId\":\"Seed\"}\n@\n\
{\"id\":1,\"woIds\":\"101,102,103,104\",\"size\":20}|\n{\"id\":2,\"woIds\":\"\",\"size\":10}\n@\n\
{\"craftedObjects\":12,\"totalSaveFileLoad\":0}\n@\n{\"stringId\":\"M\",\"isRead\":true}\n@\n{\"stringId\":\"E\"}\n@\n\
{\"saveDisplayName\":\"\",\"planetId\":\"Prime\",\"unlockedSpaceTrading\":false,\"freeCraft\":false,\"mode\":\"Standard\",\"worldSeed\":5}\n@\n{\"layerId\":\"x\"}\n";

    #[test]
    fn game_style_separators() {
        let g = S.replace("\n@\n", "\r@\r");
        let mut doc = parse(&g).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), g.as_bytes());
        doc.set("/Backpack/Iron", Value::from(4)).unwrap();
        doc.set("/Backpack/Silicon", Value::from(0)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(parse(&out).unwrap().view.pointer("/Backpack/Iron"), Some(&Value::from(4)));
        assert!(!out.contains("\"id\":103,"));
    }

    #[test]
    fn unchanged_is_byte_identical_and_values_edit_in_place() {
        let mut doc = parse(S).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), S.as_bytes());
        assert_eq!(doc.view.pointer("/Backpack/Iron"), Some(&Value::from(2)));
        assert_eq!(free_slots(&doc.view), 16);
        doc.set("/Progress/Terra tokens", Value::from(999_999)).unwrap();
        doc.set("/Planet Prime/Oxygen", Value::from(250.0)).unwrap();
        doc.set("/Settings/Free crafting", Value::Bool(true)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("\"terraTokens\":999999,"));
        assert!(out.contains("\"unitOxygenLevel\":250.0,"));
        assert!(out.contains("\"freeCraft\":true"));
    }

    #[test]
    fn materials_add_and_remove_items() {
        let mut doc = parse(S).unwrap();
        doc.set("/Backpack/Iron", Value::from(5)).unwrap();
        doc.set("/Backpack/Silicon", Value::from(0)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Backpack/Iron"), Some(&Value::from(5)));
        assert_eq!(again.view.pointer("/Backpack/Silicon"), Some(&Value::from(0)));
        assert_eq!(again.view.pointer("/Backpack/Items carried"), Some(&Value::from(6)));
        assert!(!out.contains("\"id\":103,"));
        assert!(out.contains("\"id\":104,\"gId\":\"Seed\"}\n@"));
    }

    /// `SAVE_EDITOR_PC=<file> cargo test --release --lib planet_crafter_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn planet_crafter_real() {
        let Ok(path) = std::env::var("SAVE_EDITOR_PC") else { return };
        let text = std::fs::read_to_string(path).unwrap();
        let mut doc = parse(&text).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), text.as_bytes());
        println!("{}", serde_json::to_string_pretty(&doc.view).unwrap());
        doc.set("/Progress/Terra tokens", Value::from(5)).unwrap();
        doc.set("/Backpack/Iron", Value::from(480)).unwrap();
        doc.set("/Backpack/Ice", Value::from(13)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Backpack/Iron"), Some(&Value::from(480)));
        assert_eq!(again.view.pointer("/Backpack/Ice"), Some(&Value::from(13)));
        assert_eq!(again.view.pointer("/Progress/Terra tokens"), Some(&Value::from(5)));
        let old: Vec<&str> = text.split("\r@\r").collect();
        let new: Vec<&str> = out.split("\r@\r").collect();
        assert_eq!(old.len(), new.len());
        for i in [1, 2, 5, 6, 7, 8, 9] {
            assert_eq!(old[i], new[i], "section {i} changed");
        }
        println!("objects {} -> {}", old[3].lines().count(), new[3].lines().count());
    }
}
