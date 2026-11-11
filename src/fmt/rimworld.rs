//! RimWorld saves (`.rws`): XML, but often well over 100 MB, so instead of loading the whole
//! document this finds the parts worth editing and changes them in place:
//!
//! - the player's faction (the `PlayerColony` / `PlayerTribe` entry of `factionManager`,
//!   whose `loadID` N makes pawns' `<faction>Faction_N</faction>`);
//! - its colonists: every `<li Class="Pawn">` of that faction with skills — name, and
//!   for each skill its `<level>` and `<passion>` (both left out of the file when they're 0
//!   or none, so editing one may add the element after the skill's `<def>`);
//! - silver: every thing with `<def>Silver</def>` and its `<stackCount>`.

use crate::tree::{esc, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub const PASSIONS: [&str; 3] = ["None", "Minor", "Major"];
/// Vanilla stops at 20; some mods go far past it.
const MAX_LEVEL: i128 = 1_000_000;

fn find(d: &[u8], pat: &[u8], from: usize) -> Option<usize> {
    let first = pat[0];
    let mut i = from;
    while i + pat.len() <= d.len() {
        let j = i + d[i..d.len() - pat.len() + 1].iter().position(|&b| b == first)?;
        if &d[j..j + pat.len()] == pat {
            return Some(j);
        }
        i = j + 1;
    }
    None
}

/// End (just past `</li>`) of the `<li …>` element starting at `start`.
fn li_end(d: &[u8], start: usize) -> Option<usize> {
    elem_end(d, start, "li")
}

/// End (just past its closing tag) of the `<tag …>` element starting at `start`.
fn elem_end(d: &[u8], start: usize, tag: &str) -> Option<usize> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut depth = 0usize;
    let mut i = start;
    loop {
        let j = i + d[i..].iter().position(|&b| b == b'<')?;
        if d[j..].starts_with(close.as_bytes()) {
            depth = depth.checked_sub(1)?;
            i = j + close.len();
            if depth == 0 {
                return Some(i);
            }
        } else if d[j..].starts_with(open.as_bytes()) && matches!(d.get(j + open.len()), Some(b'>' | b' ' | b'/')) {
            let close = j + d[j..].iter().position(|&b| b == b'>')?;
            if d[close - 1] != b'/' {
                depth += 1;
            } else if depth == 0 {
                return Some(close + 1);
            }
            i = close + 1;
        } else {
            i = j + 1;
        }
    }
}

/// The text of the first `<tag>…</tag>` in `d[from..to]`, with its content's span.
fn child(d: &[u8], tag: &str, from: usize, to: usize) -> Option<(String, usize, usize)> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let s = find(&d[..to], open.as_bytes(), from)? + open.len();
    let e = find(&d[..to], close.as_bytes(), s)?;
    Some((String::from_utf8_lossy(&d[s..e]).into_owned(), s, e))
}

#[derive(Clone, Debug)]
enum Leaf {
    /// Text between `start` and `end` replaced by the new number.
    Number { start: usize, end: usize },
    /// A whole `<level>` element at `start..end` (empty span: not in the file yet).
    Level { start: usize, end: usize },
    /// A whole `<passion>` element at `start..end` (empty: not in the file).
    Passion { start: usize, end: usize },
}

impl Leaf {
    fn span(&self) -> (usize, usize) {
        match *self {
            Leaf::Number { start, end } | Leaf::Level { start, end } | Leaf::Passion { start, end } => (start, end),
        }
    }
}

#[derive(Clone, Debug)]
struct RwBackend {
    data: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
    /// None, Minor, Major and any modded passions this save uses.
    passions: Arc<Vec<String>>,
}

impl RwBackend {
    fn render(&self, leaf: &Leaf, v: &Value) -> Result<String, String> {
        Ok(match leaf {
            Leaf::Number { .. } => crate::tree::int_in(v, 0, i32::MAX as i128, "This value")?.to_string(),
            Leaf::Level { .. } => {
                let n = crate::tree::int_in(v, 0, MAX_LEVEL, "Skill level")?;
                if n == 0 {
                    String::new()
                } else {
                    format!("<level>{n}</level>")
                }
            }
            Leaf::Passion { .. } => match v.as_str() {
                Some("None") => String::new(),
                Some(p) if self.passions.iter().any(|x| x == p) => format!("<passion>{p}</passion>"),
                _ => return Err("pick a passion from the list".into()),
            },
        })
    }
}

impl Backend for RwBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut edits: Vec<((usize, usize), String)> = changes.iter().map(|(l, v)| Ok((self.leaves[*l].span(), self.render(&self.leaves[*l], v)?))).collect::<Result<_, String>>()?;
        edits.sort_by_key(|e| e.0);
        let mut out = Vec::with_capacity(self.data.len() + 256);
        let mut at = 0;
        for ((s, e), text) in edits {
            out.extend_from_slice(&self.data[at..s]);
            out.extend(text.into_bytes());
            at = e;
        }
        out.extend_from_slice(&self.data[at..]);
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        self.render(&self.leaves[leaf], v).map(|_| ())
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        if !matches!(self.leaves[leaf], Leaf::Number { .. }) {
            return vec![];
        }
        let total: i64 = view["Silver"].as_object().map_or(0, |m| m.values().filter_map(Value::as_i64).sum());
        vec![("/Colony/Silver (all stacks)".into(), Value::from(total))]
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        matches!(self.leaves[leaf], Leaf::Passion { .. }).then(|| self.passions.clone())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Every passion named in the colonists' skills.
fn colonists_view(root: &Map<String, Value>) -> Vec<String> {
    let mut out = vec![];
    for c in root.get("Colonists").and_then(Value::as_object).into_iter().flat_map(|m| m.values()) {
        for sk in c["Skills"].as_object().into_iter().flat_map(|m| m.values()) {
            if let Some(p) = sk["Passion"].as_str() {
                if !out.iter().any(|x| x == p) {
                    out.push(p.to_string());
                }
            }
        }
    }
    out
}

pub fn looks(d: &[u8]) -> bool {
    let head = &d[..d.len().min(512)];
    find(head, b"<savegame>", 0).is_some() && find(head, b"<gameVersion>", 0).is_some()
}

/// `loadID` and name of the player's faction.
fn player_faction(d: &[u8]) -> Option<(String, String)> {
    let fm = find(d, b"<factionManager>", 0)?;
    for def in [&b"<def>PlayerColony</def>"[..], b"<def>PlayerTribe</def>"] {
        if let Some(at) = find(d, def, fm) {
            let end = (at + 4000).min(d.len());
            let id = child(d, "loadID", at, end)?.0;
            let name = child(d, "name", at, end).map(|n| n.0).unwrap_or_default();
            return Some((id, name));
        }
    }
    None
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut leaves = vec![];
    let mut editable = HashMap::new();
    let mut root = Map::new();
    let mut leaf = |ptr: String, l: Leaf, leaves: &mut Vec<Leaf>| {
        editable.insert(ptr, leaves.len());
        leaves.push(l);
    };

    let (fid, fname) = player_faction(d).ok_or("no player faction found")?;
    let mut colony = Map::new();
    colony.insert("Faction".into(), Value::from(fname.clone()));
    if let Some((v, _, _)) = child(d, "gameVersion", 0, d.len().min(4096)) {
        colony.insert("Game version".into(), Value::from(v));
    }
    let tag = format!("<faction>Faction_{fid}</faction>");

    // Colonists.
    let mut colonists = Map::new();
    // Pawns are `<thing Class="Pawn">` on a map and plain `<li>` among world pawns and in
    // caravans; each starts `<def>…</def> … <id>…</id> <faction>…</faction>`. Work back
    // from every skills list to that header (other `<faction>` tags, like an injury's
    // attacker, don't follow an `<id>`).
    let mut done = 0;
    let mut at = 0;
    while let Some(sk) = find(d, b"<skills>", at) {
        at = sk + 1;
        let next = d[sk + 8..].iter().position(|b| !b.is_ascii_whitespace()).map(|k| sk + 8 + k);
        if next.is_none_or(|n| !d[n..].starts_with(b"<skills>")) || sk < done {
            continue;
        }
        let mut f = sk;
        let header = loop {
            let Some(p) = d[..f].windows(9).rposition(|w| w == b"<faction>") else { break None };
            if d[p.saturating_sub(160)..p].windows(5).any(|w| w == b"</id>") {
                break Some(p);
            }
            f = p;
            if sk - p > 400_000 {
                break None;
            }
        };
        let Some(f) = header else { continue };
        if !d[f..].starts_with(tag.as_bytes()) {
            continue;
        }
        let li = d[..f].windows(3).rposition(|w| w == b"<li");
        let th = d[..f].windows(6).rposition(|w| w == b"<thing");
        let (s, elem) = match (li, th) {
            (Some(a), Some(b)) if b > a => (b, "thing"),
            (Some(a), _) => (a, "li"),
            (None, Some(b)) => (b, "thing"),
            _ => continue,
        };
        let Some(e) = elem_end(d, s, elem).filter(|&e| e > sk) else { continue };
        let Some(list) = find(&d[..e], b"<skills>", sk + 8) else { continue };
        let Some(list_end) = find(&d[..e], b"</skills>", list) else { continue };
        let name = match (child(d, "nick", s, e), child(d, "first", s, e), child(d, "last", s, e)) {
            (Some(n), _, _) if !n.0.is_empty() => n.0,
            (_, Some(f), Some(l)) => format!("{} {}", f.0, l.0),
            _ => child(d, "id", s, e).map_or_else(|| "Colonist".into(), |i| i.0),
        };
        let key = unique_key(&colonists, &name);
        let base = format!("/Colonists/{}", esc(&key));
        let mut pawn = Map::new();
        if let Some((full, _, _)) = child(d, "first", s, e) {
            let last = child(d, "last", s, e).map(|l| l.0).unwrap_or_default();
            pawn.insert("Name".into(), Value::from(format!("{full} {last}").trim().to_string()));
        }
        let mut skills = Map::new();
        let mut i = list + 8;
        while let Some(li) = find(&d[..list_end], b"<li>", i) {
            let Some(le) = li_end(d, li) else { break };
            i = le;
            let Some((def, _, def_end)) = child(d, "def", li, le) else { continue };
            let after_def = def_end + "</def>".len();
            let sp = format!("{base}/Skills/{}", esc(&def));
            let mut sk = Map::new();
            let (lv, lspan) = match find(&d[..le], b"<level>", li) {
                Some(ls) => {
                    let lend = find(&d[..le], b"</level>", ls).unwrap_or(ls) + "</level>".len();
                    let n: i64 = String::from_utf8_lossy(&d[ls + 7..lend - 8]).trim().parse().unwrap_or(0);
                    (n, (ls, lend))
                }
                None => (0, (after_def, after_def)),
            };
            sk.insert("Level".into(), Value::from(lv));
            leaf(format!("{sp}/Level"), Leaf::Level { start: lspan.0, end: lspan.1 }, &mut leaves);
            let (pv, pspan) = match find(&d[..le], b"<passion>", li) {
                Some(ps) => {
                    let pend = find(&d[..le], b"</passion>", ps).unwrap_or(ps) + "</passion>".len();
                    (String::from_utf8_lossy(&d[ps + 9..pend - 10]).into_owned(), (ps, pend))
                }
                // A new passion goes after the level (or the def).
                None => (String::from("None"), (lspan.1.max(after_def), lspan.1.max(after_def))),
            };
            sk.insert("Passion".into(), Value::from(pv));
            leaf(format!("{sp}/Passion"), Leaf::Passion { start: pspan.0, end: pspan.1 }, &mut leaves);
            skills.insert(def, Value::Object(sk));
        }
        pawn.insert("Skills".into(), Value::Object(skills));
        colonists.insert(key, Value::Object(pawn));
        done = e;
    }

    // Silver stacks.
    let mut silver = Map::new();
    let mut total: i64 = 0;
    let mut at = 0;
    while let Some(s) = find(d, b"<def>Silver</def>", at) {
        at = s + 1;
        let end = (s + 1500).min(d.len());
        let stop = find(&d[..end], b"</li>", s).or_else(|| find(&d[..end], b"</thing>", s)).unwrap_or(end);
        let Some((n, cs, ce)) = child(d, "stackCount", s, stop) else { continue };
        let Ok(n) = n.trim().parse::<i64>() else { continue };
        let pos = child(d, "pos", s, stop).map(|p| p.0).unwrap_or_default();
        let key = unique_key(&silver, &if pos.is_empty() { format!("Stack {}", silver.len() + 1) } else { format!("Stack at {pos}") });
        leaf(format!("/Silver/{}", esc(&key)), Leaf::Number { start: cs, end: ce }, &mut leaves);
        silver.insert(key, Value::from(n));
        total += n;
    }
    // The other stockpiled resources: every stack of each, by map position.
    let mut resources = Map::new();
    for def in ["Steel", "Plasteel", "ComponentIndustrial", "ComponentSpacer", "Gold", "Uranium", "Jade", "Chemfuel", "WoodLog", "Cloth", "Hyperweave", "Synthread", "Devilstrand", "MedicineHerbal", "MedicineIndustrial", "MedicineUltratech", "Neutroamine", "Penoxycyline", "Luciferium", "Beer", "Smokeleaf", "Psychite"] {
        let pat = format!("<def>{def}</def>");
        let mut stacks = Map::new();
        let mut at = 0;
        while let Some(s) = find(d, pat.as_bytes(), at) {
            at = s + 1;
            let end = (s + 1500).min(d.len());
            let stop = find(&d[..end], b"</li>", s).or_else(|| find(&d[..end], b"</thing>", s)).unwrap_or(end);
            let Some((n, cs, ce)) = child(d, "stackCount", s, stop) else { continue };
            let Ok(n) = n.trim().parse::<i64>() else { continue };
            let pos = child(d, "pos", s, stop).map(|p| p.0).unwrap_or_default();
            let key = unique_key(&stacks, &if pos.is_empty() { format!("Stack {}", stacks.len() + 1) } else { format!("Stack at {pos}") });
            leaf(format!("/Resources/{}/{}", esc(def), esc(&key)), Leaf::Number { start: cs, end: ce }, &mut leaves);
            stacks.insert(key, Value::from(n));
        }
        if !stacks.is_empty() {
            resources.insert(def.to_string(), Value::Object(stacks));
        }
    }
    if !resources.is_empty() {
        root.insert("Resources".into(), Value::Object(resources));
    }
    colony.insert("Silver (all stacks)".into(), Value::from(total));
    colony.insert("Colonists".into(), Value::from(colonists.len()));
    root.insert("Colony".into(), Value::Object(colony));
    root.insert("Colonists".into(), Value::Object(colonists));
    root.insert("Silver".into(), Value::Object(silver));
    let mut passions: Vec<String> = PASSIONS.iter().map(|s| s.to_string()).collect();
    for c in colonists_view(&root) {
        if !passions.contains(&c) {
            passions.push(c);
        }
    }
    let backend = RwBackend { data: Arc::new(d.to_vec()), leaves, passions: Arc::new(passions) };
    Ok(TreeDoc::new(Value::Object(root), editable, "RimWorld save", Box::new(backend)).with_note(
        "Colonists' skills and passions and every silver stack can be changed. The rest of this (very large) save is kept as it is.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<savegame>\n<meta><gameVersion>1.6</gameVersion></meta>\n<game><world>\
        <factionManager><allFactions><li><def>Pirate</def><loadID>3</loadID></li><li><def>PlayerColony</def><name>New Hope</name><loadID>7</loadID></li></allFactions></factionManager></world>\
        <maps><li><things>\
        <li Class=\"Pawn\"><def>Human</def><id>Human1</id><faction>Faction_7</faction><name Class=\"NameTriple\"><first>Ann</first><nick>Annie</nick><last>Lee</last></name>\
        <skills><skills><li><def>Shooting</def><level>8</level><passion>Major</passion></li><li><def>Cooking</def></li></skills></skills><inventory><li><def>Gun</def></li></inventory></li>\
        <li Class=\"Pawn\"><def>Muffalo</def><id>Muffalo2</id><faction>Faction_7</faction></li>\
        <li Class=\"Pawn\"><def>Human</def><id>Human3</id><faction>Faction_3</faction><skills><skills><li><def>Shooting</def><level>3</level></li></skills></skills></li>\
        <li Class=\"Thing\"><def>Silver</def><pos>(10, 0, 4)</pos><stackCount>120</stackCount></li><li Class=\"Thing\"><def>Steel</def><pos>(3, 0, 3)</pos><stackCount>40</stackCount></li>\
        </things></li></maps></game></savegame>";

    #[test]
    fn reads_and_edits() {
        let d = SAMPLE.as_bytes();
        assert!(looks(d));
        let mut doc = parse(d).unwrap();
        assert_eq!(doc.view["Colony"]["Faction"], "New Hope");
        assert_eq!(doc.view["Colonists"]["Annie"]["Skills"]["Shooting"]["Level"], 8);
        assert_eq!(doc.view["Colonists"]["Annie"]["Skills"]["Cooking"]["Level"], 0);
        assert_eq!(doc.view["Colonists"]["Annie"]["Skills"]["Cooking"]["Passion"], "None");
        assert_eq!(doc.view["Colonists"].as_object().unwrap().len(), 1);
        assert_eq!(doc.view["Silver"]["Stack at (10, 0, 4)"], 120);
        assert_eq!(doc.view["Resources"]["Steel"]["Stack at (3, 0, 3)"], 40);
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Colonists/Annie/Skills/Cooking/Level", Value::from(15)).unwrap();
        doc.set("/Colonists/Annie/Skills/Cooking/Passion", Value::from("Minor")).unwrap();
        doc.set("/Colonists/Annie/Skills/Shooting/Passion", Value::from("None")).unwrap();
        doc.set("/Silver/Stack at (10, 0, 4)", Value::from(500)).unwrap();
        assert!(doc.set("/Colonists/Annie/Skills/Shooting/Level", Value::from(-1)).is_err());
        assert!(doc.set("/Colonists/Annie/Skills/Shooting/Passion", Value::from("Burning")).is_err());
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert!(out.contains("<li><def>Cooking</def><level>15</level><passion>Minor</passion></li>"));
        assert!(out.contains("<li><def>Shooting</def><level>8</level></li>"));
        assert!(out.contains("<stackCount>500</stackCount>"));
        assert_eq!(parse(out.as_bytes()).unwrap().view, doc.view);
    }

    /// Research aid: parses RW_SAVE and lists the colonists.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("RW_SAVE").expect("RW_SAVE")).unwrap();
        let t = std::time::Instant::now();
        let doc = parse(&d).unwrap();
        println!("{:?}: {}", t.elapsed(), doc.view["Colony"]);
        for (k, v) in doc.view["Colonists"].as_object().unwrap() {
            println!("{k}: {}", v["Skills"].as_object().map(|m| m.iter().map(|(s, x)| format!("{s} {} {}", x["Level"], x["Passion"])).collect::<Vec<_>>().join(", ")).unwrap_or_default());
        }
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}
