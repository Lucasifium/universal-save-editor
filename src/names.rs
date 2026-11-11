//! Human-friendly labels for save data.
//!
//! * `humanize` turns raw keys like `store.max_hp` or `_partyGold` into "Max HP" / "Party Gold".
//! * For RPG Maker MV/MZ saves we look for the game's `data/` folder and pull real
//!   names for variables, switches, items, weapons, armors and actors.

use crate::json_view::pointer_escape;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Default, Clone)]
pub struct Names {
    /// Exact labels for specific JSON pointers.
    pub by_ptr: HashMap<String, String>,
    /// Hide RPG Maker JsonEx bookkeeping keys (`@c`, `@`) and unwrap `@a` arrays.
    pub hide_meta: bool,
    /// Where names were loaded from, for the status bar.
    pub source: Option<PathBuf>,
    /// Top-level sections hidden unless "Show advanced" is on.
    pub advanced: Vec<String>,
    /// Top-level sections shown first, in this order.
    pub priority: Vec<String>,
}

impl Names {
    /// Section layout for RPG Maker saves: player-facing data first, engine state as advanced.
    pub fn rpgmaker_layout(mut self) -> Self {
        self.hide_meta = true;
        self.priority = ["party", "actors", "variables", "switches"].map(String::from).to_vec();
        self.advanced = ["system", "screen", "timer", "map", "player", "selfSwitches"].map(String::from).to_vec();
        self
    }
}

/// Humanize identifier-like keys; leave already-friendly text ("Ren'Py internals") alone.
pub fn friendly_key(key: &str) -> String {
    if key.contains(' ') || key.contains('\'') {
        key.to_string()
    } else {
        humanize(key)
    }
}

/// Is the raw key different enough from its label to be worth showing?
pub fn raw_differs(key: &str, label: &str) -> bool {
    let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
    let k = norm(key.strip_prefix("store.").unwrap_or(key));
    !norm(label).starts_with(&k) || k.is_empty()
}

/// Display label for the node at `ptr` whose key is `key`.
pub fn node_label(names: &Names, key: &str, ptr: &str, v: &Value) -> String {
    if let Some(n) = names.by_ptr.get(ptr) {
        return n.clone();
    }
    let numeric = !key.is_empty() && key.chars().all(|c| c.is_ascii_digit());
    let base = if numeric { format!("#{key}") } else { friendly_key(key) };
    match object_summary(v) {
        Some(s) if numeric => format!("{base} {s}"),
        Some(s) => format!("{base} \u{2014} {s}"),
        None => base,
    }
}

const ACRONYMS: &[&str] = &["hp", "mp", "sp", "tp", "xp", "ap", "id", "ui", "npc", "atk", "def", "agi", "luk", "mat", "mdf", "exp", "lv", "dlc", "pc"];

/// "store.max_hp" → "Max HP", "_partyGold" → "Party Gold", "selfSwitches" → "Self Switches".
pub fn humanize(key: &str) -> String {
    let key = key.strip_prefix("store.").unwrap_or(key);
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    for c in key.chars() {
        let boundary = c == '_' || c == '-' || c == ' ' || c == '.';
        let camel = c.is_uppercase() && prev.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit());
        let digit_edge = prev.is_some_and(|p| p.is_ascii_digit() != c.is_ascii_digit() && p.is_alphanumeric());
        if boundary || camel || digit_edge {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            if boundary {
                prev = None;
                continue;
            }
        }
        cur.push(c);
        prev = Some(c);
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    if words.is_empty() {
        return key.to_string();
    }
    words
        .iter()
        .map(|w| {
            let lower = w.to_lowercase();
            if ACRONYMS.contains(&lower.as_str()) {
                lower.to_uppercase()
            } else {
                let mut cs = lower.chars();
                cs.next().map(|f| f.to_uppercase().chain(cs).collect()).unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A short name for an object node: its `name`/`title`/... field, if any.
pub fn object_summary(v: &Value) -> Option<String> {
    let map = v.as_object()?;
    for k in ["name", "_name", "Name", "displayName", "display_name", "title", "_title", "label", "nickname", "_nickname", "Nickname", "Species", "Item"] {
        if let Some(Value::String(s)) = map.get(k) {
            let s = s.trim();
            if !s.is_empty() {
                return Some(s.chars().take(40).collect());
            }
        }
    }
    None
}

// ----------------------------------------------------------- RPG Maker MV/MZ

pub fn is_rpgmaker(v: &Value) -> bool {
    v.get("party").is_some() && v.get("actors").is_some() && v.get("system").is_some()
}

/// Candidate `data/` folders for a save at `save_path`.
fn data_dir_candidates(save_path: &Path) -> Vec<PathBuf> {
    let Some(dir) = save_path.parent() else { return vec![] };
    let mut out = vec![dir.join("data"), dir.join("www").join("data")];
    if let Some(up) = dir.parent() {
        out.push(up.join("data"));
        out.push(up.join("www").join("data"));
        if let Some(up2) = up.parent() {
            out.push(up2.join("www").join("data"));
        }
    }
    out
}

pub fn find_rpgmaker_data(save_path: &Path) -> Option<PathBuf> {
    data_dir_candidates(save_path).into_iter().find(|d| d.join("System.json").is_file())
}

fn read_json(path: PathBuf) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

/// Names from a database file like Items.json: `[null, {"id":1,"name":"Potion"}, ...]`.
fn db_names(dir: &Path, file: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(Value::Array(arr)) = read_json(dir.join(file)) {
        for (i, entry) in arr.iter().enumerate() {
            if let Some(name) = entry.get("name").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
                out.insert(i.to_string(), name.to_string());
            }
        }
    }
    out
}

/// Pointer to an RPG Maker array that may be wrapped as `{"@a": [...]}` by JsonEx.
fn array_ptr(root: &Value, base: &str) -> Option<(String, usize)> {
    match root.pointer(base)? {
        Value::Array(a) => Some((base.to_string(), a.len())),
        Value::Object(m) => match m.get("@a") {
            Some(Value::Array(a)) => Some((format!("{base}/@a"), a.len())),
            _ => None,
        },
        _ => None,
    }
}

pub fn rpgmaker_names(root: &Value, data_dir: &Path) -> Names {
    let mut names = Names { source: Some(data_dir.to_path_buf()), ..Default::default() }.rpgmaker_layout();
    let system = read_json(data_dir.join("System.json")).unwrap_or(Value::Null);

    for (section, sys_key, kind) in [("variables", "variables", "Variable"), ("switches", "switches", "Switch")] {
        let labels = system.get(sys_key).and_then(Value::as_array);
        if let (Some(labels), Some((ptr, len))) = (labels, array_ptr(root, &format!("/{section}/_data"))) {
            for i in 1..len.min(labels.len()) {
                let label = labels[i].as_str().unwrap_or("").trim();
                let text = if label.is_empty() { format!("{kind} {i}") } else { format!("{kind} {i}: {label}") };
                names.by_ptr.insert(format!("{ptr}/{i}"), text);
            }
        }
    }

    for (key, file) in [("_items", "Items.json"), ("_weapons", "Weapons.json"), ("_armors", "Armors.json")] {
        let db = db_names(data_dir, file);
        if let Some(Value::Object(owned)) = root.pointer(&format!("/party/{key}")) {
            for id in owned.keys() {
                if let Some(name) = db.get(id) {
                    names.by_ptr.insert(format!("/party/{key}/{}", pointer_escape(id)), format!("{name}  (#{id})"));
                }
            }
        }
    }

    let actors = db_names(data_dir, "Actors.json");
    if let Some((ptr, len)) = array_ptr(root, "/actors/_data") {
        for i in 1..len {
            if let Some(name) = actors.get(&i.to_string()) {
                names.by_ptr.insert(format!("{ptr}/{i}"), format!("Actor {i}: {name}"));
            }
        }
    }
    if let Some((ptr, len)) = array_ptr(root, "/party/_actors") {
        for i in 0..len {
            let id = root.pointer(&format!("{ptr}/{i}")).and_then(Value::as_u64);
            if let Some(name) = id.and_then(|id| actors.get(&id.to_string())) {
                names.by_ptr.insert(format!("{ptr}/{i}"), format!("Party slot {}: {name}", i + 1));
            }
        }
    }

    for (ptr, label) in [
        ("/party/_gold", "Gold"),
        ("/party/_steps", "Steps taken"),
        ("/system/_saveCount", "Times saved"),
        ("/system/_battleCount", "Battles fought"),
        ("/system/_winCount", "Battles won"),
        ("/system/_escapeCount", "Battles escaped"),
    ] {
        if root.pointer(ptr).is_some() {
            names.by_ptr.insert(ptr.to_string(), label.to_string());
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanizes() {
        assert_eq!(humanize("store.max_hp"), "Max HP");
        assert_eq!(humanize("_partyGold"), "Party Gold");
        assert_eq!(humanize("selfSwitches"), "Self Switches");
        assert_eq!(humanize("chapter2_done"), "Chapter 2 Done");
        assert_eq!(humanize("gold"), "Gold");
    }
}
