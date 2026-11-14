//! Divinity: Original Sin 2 (Definitive Edition) and Baldur's Gate 3 saves (`.lsv`): an
//! LSPK package (`lspk.rs`) of LSF resources (`lsf.rs`).
//!
//! Divinity: Original Sin 2 keeps the world in `globals.lsf`. Each player character is a
//! `Characters/CharacterFactory/Characters/Character` node: current `Vitality` and its
//! `Inventory` handle on the node, `Experience` in its `Stats` child (with `IsPlayer` and
//! `InParty`), the name in `PlayerData/PlayerCustomData`, unspent points and the six
//! attributes (Strength, Finesse, Intelligence, Constitution, Memory, Wits, in that order)
//! in `PlayerUpgrade`. Gold is an item (`Items/ItemFactory/Items/Item`, stats such as
//! `Small_Gold`) whose `Parent` is the character's inventory and whose `Amount` is the
//! number of coins. All of these are plain numbers, changed in place.
//!
//! Baldur's Gate 3: `Globals.lsf` is opened. Stack sizes (gold too) live in its packed entity
//! store (`lsmf.rs`); see `bg3_carried`. Ability scores and hit points, also in that store,
//! aren't mapped yet; the save's own summary (`SaveInfo.json`, `meta.lsf`) is shown read-only.

use super::fields::{Builder, Checksum, Kind};
use super::lsf::{self, Lsf};
use super::lspk;
use crate::tree::TreeDoc;
use serde_json::{json, Value};
use std::collections::HashMap;

pub const DOS2: &str = "Divinity: Original Sin 2 save";
pub const BG3: &str = "Baldur's Gate 3 save";

pub const ATTRIBUTES: [&str; 6] = ["Strength", "Finesse", "Intelligence", "Constitution", "Memory", "Wits"];
const POINTS: [(&str, &str); 4] = [
    ("AttributePoints", "Attribute points"),
    ("CombatAbilityPoints", "Combat ability points"),
    ("CivilAbilityPoints", "Civil ability points"),
    ("TalentPoints", "Talent points"),
];

fn is_bg3(pkg: &lspk::Package) -> bool {
    pkg.find("SaveInfo.json").is_some()
}

/// The LSF file the editor opens: `globals.lsf` (DOS2) or `Globals.lsf` (BG3, falling back to
/// `meta.lsf` when the world doesn't read), unpacked.
pub fn open(data: &[u8]) -> Option<(String, Vec<u8>)> {
    let pkg = lspk::read(data)?;
    if is_bg3(&pkg) {
        if let Some(e) = pkg.find("Globals.lsf") {
            if let Some(plain) = pkg.file(data, e).and_then(|r| lsf::unpack(&r)).filter(|p| Lsf::read(p).is_some()) {
                return Some((e.name.clone(), plain));
            }
        }
    }
    let e = if is_bg3(&pkg) { pkg.find("meta.lsf") } else { pkg.find("globals.lsf") }?;
    let plain = lsf::unpack(&pkg.file(data, e)?)?;
    Some((e.name.clone(), plain))
}

/// Puts the edited (unpacked) LSF back into the package. Unchanged, the package is
/// returned as it was.
pub fn save(archive: &[u8], file: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    let pkg = lspk::read(archive).ok_or("the save package no longer reads")?;
    let e = pkg.find(file).ok_or_else(|| format!("{file} is missing from the save"))?;
    let raw = pkg.file(archive, e).ok_or_else(|| format!("{file} doesn't unpack"))?;
    if lsf::unpack(&raw).as_deref() == Some(plain) {
        return Ok(archive.to_vec());
    }
    let packed = lsf::pack(plain, &raw)?;
    let mut replace = HashMap::from([(e.name.clone(), packed)]);
    // Baldur's Gate 3: flag edits are mirrored in the story's own flag list.
    if is_bg3(&pkg) && file.eq_ignore_ascii_case("Globals.lsf") {
        if let Some(se) = pkg.find("StorySave.bin") {
            let story = pkg.file(archive, se).ok_or("StorySave.bin doesn't unpack")?;
            let old = lsf::unpack(&raw).ok_or("Globals.lsf doesn't unpack")?;
            if let Some(new_story) = super::bg3_flags::sync_story(&old, plain, &story)? {
                replace.insert(se.name.clone(), new_story);
            }
        }
    }
    pkg.rebuild(archive, &replace)
}

/// The game view of the opened LSF (`plain`), with the rest of the package for context.
pub fn parse(plain: &[u8], archive: &[u8], file: &str) -> Option<TreeDoc> {
    let pkg = lspk::read(archive)?;
    if is_bg3(&pkg) {
        bg3(plain, archive, &pkg)
    } else if file.eq_ignore_ascii_case("globals.lsf") {
        dos2(plain, archive, &pkg)
    } else {
        None
    }
}

fn cat<'a>(base: &[&'a str], rest: &[&'a str]) -> Vec<&'a str> {
    base.iter().chain(rest).copied().collect()
}

fn unique(seen: &mut HashMap<String, usize>, name: String) -> String {
    let n = seen.entry(name.clone()).or_insert(0);
    *n += 1;
    if *n == 1 {
        name
    } else {
        format!("{name} ({n})")
    }
}

/// An editable number at attribute `name` of node `n`, if it's a plain integer.
fn number(b: &mut Builder, l: &Lsf, path: &[&str], n: usize, name: &str, range: Option<(i64, i64)>) {
    let Some(a) = l.attr(n, name) else { return };
    let Some(kind) = lsf::kind(a.ty) else { return };
    match range {
        Some((lo, hi)) => b.ranged(path, a.at, kind, lo, hi),
        None => b.field(path, a.at, kind),
    };
}

/// Coin stacks: the stats names gold uses in the game (Tiny_Gold … Trader_Huge_Gold,
/// InBetween_Gold_Big, Neo_Gold_Reward_*), not gold-coloured items.
pub fn is_gold(stats: &str) -> bool {
    stats.ends_with("_Gold") || stats.ends_with("_Gold_Big") || stats.starts_with("Neo_Gold_Reward")
}

fn meta_facts(archive: &[u8], pkg: &lspk::Package) -> Vec<(&'static str, Value)> {
    let mut out = vec![];
    let Some(plain) = pkg.find("meta.lsf").and_then(|e| pkg.file(archive, e)).and_then(|r| lsf::unpack(&r)) else { return out };
    let Some(l) = Lsf::read(&plain) else { return out };
    if let Some(&m) = l.at_path(&["MetaData", "MetaData"]).first() {
        if let Some(s) = l.string_of(m, "LeaderName").filter(|s| !s.is_empty()) {
            out.push(("Leader", json!(s)));
        }
        if let Some(s) = l.string_of(m, "Level").filter(|s| !s.is_empty()) {
            out.push(("Region", json!(s)));
        }
        let versions: Vec<String> = l.children_named(m, "GameVersions").flat_map(|g| l.children_named(g, "GameVersion").collect::<Vec<_>>()).filter_map(|g| l.string_of(g, "Object")).collect();
        if let Some(v) = versions.last() {
            out.push(("Game version", json!(v)));
        }
    }
    out
}

/// The player characters of a Divinity save, by the name the screen shows, with their nodes.
pub fn dos2_party(l: &Lsf) -> Vec<(String, usize)> {
    let mut seen = HashMap::new();
    let mut out = vec![];
    for c in l.at_path(&["Characters", "CharacterFactory", "Characters", "Character"]) {
        let Some(stats) = l.child(c, "Stats") else { continue };
        if l.int_of(stats, "IsPlayer") != Some(1) {
            continue;
        }
        let custom = l.child(c, "PlayerData").and_then(|p| l.child(p, "PlayerCustomData"));
        let text = |k: &str| custom.and_then(|p| l.string_of(p, k)).filter(|s| !s.is_empty());
        let name = text("Name").or_else(|| text("OriginName")).unwrap_or_else(|| format!("Character {}", out.len() + 1));
        out.push((unique(&mut seen, name), c));
    }
    out
}

/// "Add items": a drop-down per item kind the save can add, and who gets them.
fn add_items(b: &mut Builder, l: &Lsf, plain: &[u8]) {
    let party = dos2_party(l);
    let offers = super::larian_add::offers(plain);
    if party.is_empty() || offers.is_empty() {
        return;
    }
    b.option(&["Add items", "Add to"], party.iter().map(|p| p.0.clone()).collect(), "W".into());
    for o in &offers {
        let mut names = vec!["Don't add".to_string()];
        let amounts: &[u32] = if o.stackable { &super::larian_add::AMOUNTS } else { &super::larian_add::AMOUNTS[..1] };
        names.extend(amounts.iter().map(|n| format!("Add {n}")));
        b.option(&["Add items", o.stats.as_str()], names, format!("I{}", o.stats));
    }
}

/// Saves the items chosen under "Add items" into the chosen character's inventory.
fn add_post(out: &mut Vec<u8>, chosen: &[(std::sync::Arc<str>, String)]) -> Result<(), String> {
    let adds: Vec<(String, u32)> = chosen.iter().filter_map(|(t, c)| Some((t.strip_prefix('I')?.to_string(), c.strip_prefix("Add ")?.parse().ok()?))).collect();
    if adds.is_empty() {
        return Ok(());
    }
    let who = chosen.iter().find(|(t, _)| &**t == "W").map(|(_, c)| c.clone());
    let new = {
        let l = Lsf::read(out).ok_or("the game data no longer reads")?;
        let party = dos2_party(&l);
        let node = match &who {
            Some(w) => party.iter().find(|p| &p.0 == w),
            None => party.first(),
        }
        .ok_or("that character isn't in the save")?
        .1;
        super::larian_add::add(out, node, &adds)?
    };
    *out = new;
    Ok(())
}

fn dos2(plain: &[u8], archive: &[u8], pkg: &lspk::Package) -> Option<TreeDoc> {
    let l = Lsf::read(plain)?;
    let mut b = Builder::new(plain);
    for (k, v) in meta_facts(archive, pkg) {
        b.info(&["Save", k], v);
    }
    // Coin stacks by the inventory that holds them.
    let mut gold: HashMap<i64, Vec<usize>> = HashMap::new();
    let mut bags: HashMap<i64, Vec<usize>> = HashMap::new();
    for item in l.at_path(&["Items", "ItemFactory", "Items", "Item"]) {
        let (Some(stats), Some(parent)) = (l.string_of(item, "Stats"), l.int_of(item, "Parent")) else { continue };
        if l.attr(item, "Amount").is_none() {
            continue;
        }
        if is_gold(&stats) {
            gold.entry(parent).or_default().push(item);
        } else {
            bags.entry(parent).or_default().push(item);
        }
    }
    let mut members = 0;
    for (key, c) in dos2_party(&l) {
        let Some(stats) = l.child(c, "Stats") else { continue };
        let custom = l.child(c, "PlayerData").and_then(|p| l.child(p, "PlayerCustomData"));
        let text = |k: &str| custom.and_then(|p| l.string_of(p, k)).filter(|s| !s.is_empty());
        members += 1;
        let base = ["Party", key.as_str()];
        if let Some(o) = text("OriginName") {
            b.info(&cat(&base, &["Origin"]), o);
        }
        if let Some(r) = text("Race") {
            b.info(&cat(&base, &["Race"]), r);
        }
        b.info(&cat(&base, &["In party"]), l.int_of(stats, "InParty") == Some(1));
        number(&mut b, &l, &cat(&base, &["Experience"]), stats, "Experience", Some((0, i64::from(i32::MAX))));
        number(&mut b, &l, &cat(&base, &["Vitality"]), c, "Vitality", None);
        if let Some(up) = l.child(c, "PlayerUpgrade") {
            let attrs: Vec<usize> = l.children_named(up, "Attributes").collect();
            if attrs.len() == ATTRIBUTES.len() {
                for (n, name) in attrs.into_iter().zip(ATTRIBUTES) {
                    number(&mut b, &l, &cat(&base, &["Attributes", name]), n, "Object", None);
                }
            }
            for (k, label) in POINTS {
                number(&mut b, &l, &cat(&base, &["Points", label]), up, k, None);
            }
        }
        let inv = l.int_of(c, "Inventory");
        let mut piles = HashMap::new();
        for &item in inv.and_then(|i| gold.get(&i)).into_iter().flatten() {
            let stats = l.string_of(item, "Stats").unwrap_or_default();
            let k = unique(&mut piles, stats);
            let Some(a) = l.attr(item, "Amount") else { continue };
            if let Some(kind) = lsf::kind(a.ty).filter(|k| matches!(k, Kind::I32 | Kind::U32)) {
                b.ranged(&cat(&base, &["Gold", k.as_str()]), a.at, kind, 1, i64::from(i32::MAX));
            }
        }
        // Everything else the character carries directly (not inside a bag): stack sizes by item.
        let mut names = HashMap::new();
        for &item in inv.and_then(|i| bags.get(&i)).into_iter().flatten() {
            let stats = l.string_of(item, "Stats").unwrap_or_default();
            if stats.is_empty() {
                continue;
            }
            let k = unique(&mut names, stats);
            let Some(a) = l.attr(item, "Amount") else { continue };
            if let Some(kind) = lsf::kind(a.ty).filter(|k| matches!(k, Kind::I32 | Kind::U32)) {
                b.ranged(&cat(&base, &["Inventory", k.as_str()]), a.at, kind, 1, i64::from(i32::MAX));
            }
        }
    }
    if members == 0 {
        return None;
    }
    add_items(&mut b, &l, plain);
    Some(b.finish_with(DOS2, Checksum::None, Some(add_post)))
}

fn bg3(plain: &[u8], archive: &[u8], pkg: &lspk::Package) -> Option<TreeDoc> {
    Lsf::read(plain)?;
    let info: Value = pkg.find("SaveInfo.json").and_then(|e| pkg.file(archive, e)).and_then(|b| serde_json::from_slice(&b).ok())?;
    let mut b = Builder::new(plain);
    if let Some(s) = info.get("Save Name").and_then(Value::as_str) {
        b.info(&["Save", "Save name"], s);
    }
    for (k, v) in meta_facts(archive, pkg) {
        b.info(&["Save", k], v);
    }
    if let Some(d) = info.get("Difficulty").and_then(Value::as_array) {
        let d: Vec<&str> = d.iter().filter_map(Value::as_str).collect();
        b.info(&["Save", "Difficulty"], d.join(", "));
    }
    if let Some(p) = info.get("Platform").and_then(Value::as_str) {
        b.info(&["Save", "Platform"], p);
    }
    let mut party = vec![];
    for c in info.pointer("/Active Party/Characters").and_then(Value::as_array).into_iter().flatten() {
        let s = |k: &str| c.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
        let classes: Vec<String> = c
            .get("Classes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|k| {
                let main = k.get("Main").and_then(Value::as_str).unwrap_or_default();
                match k.get("Sub").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                    Some(sub) => format!("{main} ({sub})"),
                    None => main.to_string(),
                }
            })
            .collect();
        party.push(json!({
            "Origin": s("Origin"),
            "Race": s("Race"),
            "Class": classes.join(" / "),
            "Level": c.get("Level").cloned().unwrap_or(Value::Null),
            "Experience": c.get("Experience Points (Total)").cloned().unwrap_or(Value::Null),
        }));
    }
    b.info(&["Party"], Value::Array(party));
    let carried = bg3_carried(&mut b, plain);
    bg3_add_items(&mut b, plain);
    let story = pkg.find("StorySave.bin").and_then(|e| pkg.file(archive, e)).and_then(|d| super::osiris::read(&d));
    let flags = match (&story, Lsf::read(plain)) {
        (Some(st), Some(l)) => super::bg3_flags::add(&mut b, &l, plain, 0, st),
        _ => 0,
    };
    let note = if flags > 0 {
        "Baldur's Gate 3: gold and stack sizes of what each companion carries, and the story's flags by name. \
         A flag changed here is also changed in the story script's own list when the save is written."
    } else if carried {
        "Baldur's Gate 3: gold and stack sizes of what each companion carries, from the save's entity store. \
         Ability scores and hit points aren't mapped yet."
    } else {
        "Baldur's Gate 3: the party summary is shown read-only."
    };
    Some(b.finish_with(BG3, Checksum::None, Some(bg3_add_post)).with_note(note))
}

/// "Add items" for BG3: who gets them, and how many copies of each kind of item the party
/// carries in its bags (see `bg3_add`).
fn bg3_add_items(b: &mut Builder, plain: &[u8]) {
    let Some(l) = Lsf::read(plain) else { return };
    let Some(a) = super::bg3_add::store_attr(&l) else { return };
    let Some(s) = super::bg3_add::Store::read(&plain[a.at..a.at + a.len]) else { return };
    let party = bg3_recipients(&l, plain);
    let offers = super::bg3_add::offers(&l, &s, &party);
    if party.is_empty() || offers.is_empty() {
        return;
    }
    b.option(&["Add items", "Add to"], party.iter().map(|p| p.0.clone()).collect(), "W".into());
    for stats in offers.keys() {
        let mut names = vec!["Don't add".to_string()];
        names.extend(super::bg3_add::COPIES.iter().map(|n| format!("Add {n}")));
        b.option(&["Add items", stats.as_str()], names, format!("I{stats}"));
    }
}

/// Who can be given items: your character first, then whoever is in the active party (a low
/// bit of `Flags` set), then the rest (companions at camp, and those not met yet).
fn bg3_recipients(l: &Lsf, plain: &[u8]) -> Vec<(String, usize)> {
    let mut who: Vec<(u8, String, usize)> = bg3_party(l, plain)
        .into_iter()
        .map(|p| (if p.0 == "Your character" { 0 } else if l.int_of(p.1, "Flags").is_some_and(|f| f & 0x10 != 0) { 1 } else { 2 }, p.0, p.1))
        .collect();
    who.sort_by_key(|w| w.0);
    who.into_iter().map(|w| (w.1, w.2)).collect()
}

/// Saves the items chosen under "Add items" into the chosen character's bag.
fn bg3_add_post(out: &mut Vec<u8>, chosen: &[(std::sync::Arc<str>, String)]) -> Result<(), String> {
    let adds: Vec<(String, u32)> = chosen.iter().filter_map(|(t, c)| Some((t.strip_prefix('I')?.to_string(), c.strip_prefix("Add ")?.parse().ok()?))).collect();
    if adds.is_empty() {
        return Ok(());
    }
    let who = chosen.iter().find(|(t, _)| &**t == "W").map(|(_, c)| c.clone());
    let new = {
        let l = Lsf::read(out).ok_or("the game data no longer reads")?;
        let party = bg3_recipients(&l, out);
        let node = match &who {
            Some(w) => party.iter().find(|p| &p.0 == w),
            None => party.first(),
        }
        .ok_or("that character isn't in the save")?
        .1;
        super::bg3_add::add_items(out, &party, node, &adds)?
    };
    *out = new;
    Ok(())
}

/// Origin characters by root template (the GUIDs the game's own scripts use).
const ORIGINS: [(&str, &str); 10] = [
    ("3ed74f06-3c60-42dc-83f6-f034cb47c679", "Shadowheart"),
    ("c7c13742-bacd-460a-8f65-f864fe41f255", "Astarion"),
    ("ad9af97d-75da-406a-ae13-7071c563f604", "Gale"),
    ("58a69333-40bf-8358-1d17-fff240d7fb12", "Lae'zel"),
    ("c774d764-4a17-48dc-b470-32ace9ce447d", "Wyll"),
    ("2c76687d-93a2-477b-8b18-8a14b549304c", "Karlach"),
    ("25721313-0c15-4935-8176-9f134385451b", "Minthara"),
    ("7628bc0e-52b8-42a7-856a-13a6fd413323", "Halsin"),
    ("91b6b200-7d00-4d62-8dc9-99e8339dfa1a", "Jaheira"),
    ("0de603c5-42e2-4811-9dad-f652de080eba", "Minsc"),
];

/// A GUID attribute as text, in the order Larian's tools print it (the last eight bytes are
/// stored in swapped pairs).
pub fn guid_text(g: &[u8]) -> String {
    if g.len() < 16 {
        return String::new();
    }
    let d1 = u32::from_le_bytes(g[0..4].try_into().unwrap());
    let d2 = u16::from_le_bytes(g[4..6].try_into().unwrap());
    let d3 = u16::from_le_bytes(g[6..8].try_into().unwrap());
    let t: Vec<u8> = g[8..16].chunks(2).flat_map(|p| [p[1], p[0]]).collect();
    format!("{d1:08x}-{d2:04x}-{d3:04x}-{:02x}{:02x}-{}", t[0], t[1], t[2..].iter().map(|b| format!("{b:02x}")).collect::<String>())
}

/// Coins: `OBJ_GoldCoin` and the gold piles found in the world.
pub fn bg3_is_gold(stats: &str) -> bool {
    matches!(stats, "OBJ_GoldCoin" | "OBJ_GoldPile") || stats.starts_with("LOOT_Gold")
}

/// The player characters of a BG3 save that carry something, by the name the screen shows:
/// (name, character node, what they carry as (item node, creator node)). A carried item's
/// position is its holder's; character creation's stand-ins are left out.
pub fn bg3_party(l: &Lsf, plain: &[u8]) -> Vec<(String, usize, Vec<(usize, usize)>)> {
    let items = l.at_path(&["Items", "ItemFactory", "Items", "Item"]);
    let creators = l.at_path(&["Items", "ItemFactory", "Creators", "Creator"]);
    if items.len() != creators.len() {
        return vec![];
    }
    let pos = |n: usize| l.attr(n, "Translate").filter(|a| a.len == 12).map(|a| &plain[a.at..a.at + 12]);
    let mut people: Vec<(usize, Option<&str>, Vec<(usize, usize)>)> = vec![];
    for c in l.at_path(&["Characters", "CharacterFactory", "Characters", "Character"]) {
        if l.child(c, "PlayerData").is_none() || l.string_of(c, "Level").is_some_and(|s| s.starts_with("SYS_CC")) {
            continue;
        }
        let Some(here) = pos(c) else { continue };
        let mine: Vec<(usize, usize)> = items.iter().zip(&creators).filter(|(i, _)| pos(**i) == Some(here)).map(|(i, c)| (*i, *c)).collect();
        if mine.is_empty() {
            continue;
        }
        let tmpl = l.attr(c, "CurrentTemplate").filter(|a| a.len == 16).map(|a| guid_text(&plain[a.at..a.at + 16])).unwrap_or_default();
        let origin = ORIGINS.iter().find(|(g, _)| *g == tmpl).map(|o| o.1);
        people.push((c, origin, mine));
    }
    // The custom character carries the most of the characters that aren't origins; the
    // others are hirelings and the like.
    let you = people.iter().enumerate().filter(|p| p.1 .1.is_none()).max_by_key(|p| p.1 .2.len()).map(|p| p.0);
    let mut seen = HashMap::new();
    let mut others = 0;
    let mut out = vec![];
    for (n, (c, origin, mine)) in people.into_iter().enumerate() {
        let name = match origin {
            Some(o) => o.to_string(),
            None if Some(n) == you => "Your character".to_string(),
            None => {
                others += 1;
                format!("Other character {others}")
            }
        };
        out.push((unique(&mut seen, name), c, mine));
    }
    out
}

/// What each player character carries, from `Globals.lsf`: a carried item's position is its
/// holder's position, the `Creators` table (in step with `Items`) gives each item's entity, and
/// the entity store holds the stack sizes. Each stack size whose item has a single entry is
/// editable; gold is summed per character and its stacks are listed under "Gold".
fn bg3_carried(b: &mut Builder, plain: &[u8]) -> bool {
    let Some(l) = Lsf::read(plain) else { return false };
    let Some(&na) = l.at_path(&["NewAge"]).first() else { return false };
    let Some(blob_at) = l.attr(na, "NewAge").filter(|a| a.ty == 25 && a.at + a.len <= plain.len()) else { return false };
    let blob = &plain[blob_at.at..blob_at.at + blob_at.len];
    let stacks = super::lsmf::stacks(blob);
    if stacks.is_empty() {
        return false;
    }
    let mut any = false;
    for (key, _c, mine) in bg3_party(&l, plain) {
        let mut gold_total = 0u64;
        let mut names = HashMap::new();
        let mut golds = 0;
        let mut had = false;
        for (item, cr) in mine {
            let Some(e) = l.attr(cr, "Entity").filter(|a| a.len == 16) else { continue };
            let g: [u8; 16] = plain[e.at..e.at + 16].try_into().unwrap();
            let Some(st) = stacks.get(&g) else { continue };
            let stats = l.string_of(item, "Stats").unwrap_or_default();
            if stats.is_empty() {
                continue;
            }
            let gold = bg3_is_gold(&stats);
            if gold {
                gold_total += st.total(blob);
            }
            let label = if gold {
                golds += 1;
                format!("Stack {golds}")
            } else {
                unique(&mut names, stats)
            };
            let path = if gold { vec!["Characters", key.as_str(), "Gold", label.as_str()] } else { vec!["Characters", key.as_str(), "Items", label.as_str()] };
            if st.amounts.len() == 1 {
                b.ranged(&path, blob_at.at + st.amounts[0], Kind::U32, 1, i64::from(i32::MAX));
            } else {
                b.info(&path, st.total(blob));
            }
            any = true;
            had = true;
        }
        if had {
            b.info(&["Characters", key.as_str(), "Gold total"], gold_total);
        }
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::lsf::tests::build as lsf_build;
    use crate::fmt::lspk::tests::build as pkg_build;

    fn i32v(v: i32) -> Vec<u8> {
        v.to_le_bytes().to_vec()
    }
    fn u64v(v: u64) -> Vec<u8> {
        v.to_le_bytes().to_vec()
    }
    fn s(v: &str) -> Vec<u8> {
        let mut b = v.as_bytes().to_vec();
        b.push(0);
        b
    }

    /// A small DOS2 world: one player (made-up "Testa") with a coin stack, one NPC.
    fn dos2_save() -> Vec<u8> {
        let mut nodes: Vec<(&str, i32, Vec<(&str, u32, Vec<u8>)>)> = vec![
            ("Characters", -1, vec![]),
            ("CharacterFactory", 0, vec![]),
            ("Characters", 1, vec![]),
            ("Character", 2, vec![("Vitality", 4, i32v(55)), ("Inventory", 24, u64v(77))]),
            ("Stats", 3, vec![("Experience", 4, i32v(3350)), ("IsPlayer", 19, vec![1]), ("InParty", 19, vec![1])]),
            ("PlayerData", 3, vec![]),
            ("PlayerCustomData", 5, vec![("Name", 22, s("Testa")), ("Race", 22, s("Elf"))]),
            ("PlayerUpgrade", 3, vec![("AttributePoints", 4, i32v(2)), ("CombatAbilityPoints", 4, i32v(1)), ("CivilAbilityPoints", 4, i32v(0)), ("TalentPoints", 4, i32v(1))]),
        ];
        for v in [11u32, 12, 10, 13, 10, 14] {
            nodes.push(("Attributes", 7, vec![("Object", 5, v.to_le_bytes().to_vec())]));
        }
        let npc = nodes.len() as i32;
        nodes.push(("Character", 2, vec![("Vitality", 4, i32v(9))]));
        nodes.push(("Stats", npc, vec![("Experience", 4, i32v(0)), ("IsPlayer", 19, vec![0])]));
        let items = nodes.len() as i32;
        nodes.push(("Items", -1, vec![]));
        nodes.push(("ItemFactory", items, vec![]));
        nodes.push(("Items", items + 1, vec![]));
        nodes.push(("Item", items + 2, vec![("Stats", 22, s("Small_Gold")), ("Parent", 24, u64v(77)), ("Amount", 4, i32v(309))]));
        nodes.push(("Item", items + 2, vec![("Stats", 22, s("ITEM_EmptyCupGold")), ("Parent", 24, u64v(77)), ("Amount", 4, i32v(1))]));
        nodes.push(("Item", items + 2, vec![("Stats", 22, s("CON_Potion_Health")), ("Parent", 24, u64v(77)), ("Amount", 4, i32v(6))]));
        let globals = lsf_build(&nodes, true);
        let meta = lsf_build(&[("MetaData", -1, vec![]), ("MetaData", 0, vec![("Level", 22, s("TestRegion"))])], true);
        pkg_build(13, &[("globals.lsf", &globals), ("meta.lsf", &meta), ("test.png", b"png")], 0x21)
    }

    #[test]
    fn dos2_party_edits_round_trip() {
        let raw = dos2_save();
        let d = crate::formats::decode(&raw);
        let crate::formats::Content::Tree(t) = &d.content else { panic!("not read: {}", crate::formats::describe(&d.layers, &d.content)) };
        assert_eq!(t.kind_name, DOS2);
        assert_eq!(crate::game_ui::detect(&t.kind_name, &t.view), Some(crate::game_ui::Game::Dos2));
        let v = &t.view;
        assert_eq!(v.pointer("/Party/Testa/Experience"), Some(&json!(3350)));
        assert_eq!(v.pointer("/Party/Testa/Vitality"), Some(&json!(55)));
        assert_eq!(v.pointer("/Party/Testa/Attributes/Wits"), Some(&json!(14)));
        assert_eq!(v.pointer("/Party/Testa/Points/Attribute points"), Some(&json!(2)));
        assert_eq!(v.pointer("/Party/Testa/Gold/Small_Gold"), Some(&json!(309)));
        assert_eq!(v.pointer("/Party/Testa/Inventory/CON_Potion_Health"), Some(&json!(6)));
        assert!(v.pointer("/Party/Testa/Inventory/ITEM_EmptyCupGold").is_some(), "other items are listed");
        assert_eq!(v.pointer("/Party/Testa/Gold").and_then(Value::as_object).map(|m| m.len()), Some(1), "a gold cup isn't gold");
        assert_eq!(v.pointer("/Save/Region"), Some(&json!("TestRegion")));
        assert_eq!(v.pointer("/Party").and_then(Value::as_object).map(|m| m.len()), Some(1), "NPCs aren't in the party");
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), raw, "no edit keeps the file");
        let mut d = d;
        let crate::formats::Content::Tree(t) = &mut d.content else { unreachable!() };
        t.set("/Party/Testa/Experience", json!(3351)).unwrap();
        t.set("/Party/Testa/Attributes/Strength", json!(12)).unwrap();
        t.set("/Party/Testa/Gold/Small_Gold", json!(310)).unwrap();
        t.set("/Party/Testa/Inventory/CON_Potion_Health", json!(99)).unwrap();
        assert!(t.set("/Party/Testa/Gold/Small_Gold", json!(0)).is_err());
        let out = crate::formats::encode(&d.layers, &d.content).unwrap();
        let p = lspk::read(&out).unwrap();
        assert!(p.checksum_ok(&out));
        let again = crate::formats::decode(&out);
        let crate::formats::Content::Tree(t2) = &again.content else { panic!("re-read failed") };
        assert_eq!(t2.view.pointer("/Party/Testa/Experience"), Some(&json!(3351)));
        assert_eq!(t2.view.pointer("/Party/Testa/Attributes/Strength"), Some(&json!(12)));
        assert_eq!(t2.view.pointer("/Party/Testa/Gold/Small_Gold"), Some(&json!(310)));
        assert_eq!(t2.view.pointer("/Party/Testa/Inventory/CON_Potion_Health"), Some(&json!(99)));
        assert_eq!(t2.view.pointer("/Party/Testa/Attributes/Wits"), Some(&json!(14)));
        // The other files are untouched.
        let q = lspk::read(&raw).unwrap();
        for n in ["meta.lsf", "test.png"] {
            assert_eq!(p.file(&out, p.find(n).unwrap()), q.file(&raw, q.find(n).unwrap()));
        }
    }

    /// A tiny entity store: two entities, one stack record giving the second a stack of `amount`.
    fn lsmf_blob(guids: [[u8; 16]; 2], amount: u32) -> Vec<u8> {
        let names = b"\0core.v0.EntityIdgame.inventory.v0.NewStackComponentgame.inventory.v0.StackEntry".to_vec();
        let comps: [(usize, usize, u32, u64); 3] = [(1, 16, 16, 2), (17, 35, 8, 1), (52, 28, 8, 1)];
        // Layout: header 48 | entity ids (32) | stack component (8) | entries (8) | record (32) | members (8) | names | descriptors
        let (eid, ns, se, rec, mem) = (48usize, 80usize, 88usize, 96usize, 128usize);
        let names_off = 136usize;
        let desc_rel = names.len() as u32;
        let mut b = vec![0u8; names_off];
        b[0..4].copy_from_slice(b"LSMF");
        b[16..24].copy_from_slice(&((names_off - 48) as u64).to_le_bytes());
        b[24..32].copy_from_slice(&(names.len() as u64).to_le_bytes());
        b[32..36].copy_from_slice(&desc_rel.to_le_bytes());
        b[36..38].copy_from_slice(&3u16.to_le_bytes());
        b[eid..eid + 16].copy_from_slice(&guids[0]);
        b[eid + 16..eid + 32].copy_from_slice(&guids[1]);
        b[ns..ns + 8].copy_from_slice(&((rec - 48) as u64).to_le_bytes());
        b[se..se + 2].copy_from_slice(&0u16.to_le_bytes());
        b[se + 4..se + 8].copy_from_slice(&amount.to_le_bytes());
        for (k, v) in [(mem - 48) as u64, (mem + 8 - 48) as u64, (se - 48) as u64, (se + 8 - 48) as u64].into_iter().enumerate() {
            b[rec + 8 * k..rec + 8 * k + 8].copy_from_slice(&v.to_le_bytes());
        }
        b[mem..mem + 8].copy_from_slice(&((eid + 16 - 48) as u64).to_le_bytes());
        b.extend(&names);
        for (i, (no, nl, elem, rows)) in comps.into_iter().enumerate() {
            let off = ([eid, ns, se][i] - 48) as u64;
            let mut d = vec![0u8; 48];
            d[0..8].copy_from_slice(&(no as u64).to_le_bytes());
            d[8..12].copy_from_slice(&(nl as u32).to_le_bytes());
            d[24..28].copy_from_slice(&elem.to_le_bytes());
            d[32..40].copy_from_slice(&rows.to_le_bytes());
            d[40..48].copy_from_slice(&off.to_le_bytes());
            b.extend(d);
        }
        b
    }

    #[test]
    fn bg3_gold_and_stacks_edit_in_the_entity_store() {
        let g = |n: u8| [n; 16];
        // Shadowheart's root template, as the save stores it.
        let sh: Vec<u8> = vec![0x06, 0x4f, 0xd7, 0x3e, 0x60, 0x3c, 0xdc, 0x42, 0xf6, 0x83, 0x34, 0xf0, 0x47, 0xcb, 0x79, 0xc6];
        assert_eq!(guid_text(&sh), "3ed74f06-3c60-42dc-83f6-f034cb47c679");
        let here: Vec<u8> = [1.5f32, 2.0, -3.25].iter().flat_map(|f| f.to_le_bytes()).collect();
        let away: Vec<u8> = [9.0f32, 9.0, 9.0].iter().flat_map(|f| f.to_le_bytes()).collect();
        let blob = lsmf_blob([g(7), g(8)], 766);
        assert_eq!(super::super::lsmf::stacks(&blob).get(&g(8)).map(|s| s.total(&blob)), Some(766));
        let nodes: Vec<(&str, i32, Vec<(&str, u32, Vec<u8>)>)> = vec![
            ("NewAge", -1, vec![("NewAge", 25, blob.clone())]),
            ("Characters", -1, vec![]),
            ("CharacterFactory", 1, vec![]),
            ("Characters", 2, vec![]),
            ("Character", 3, vec![("Translate", 12, here.clone()), ("CurrentTemplate", 31, sh.clone()), ("Level", 22, s("WLD_Main_A"))]),
            ("PlayerData", 4, vec![]),
            ("Items", -1, vec![]),
            ("ItemFactory", 6, vec![]),
            ("Creators", 7, vec![]),
            ("Creator", 8, vec![("Entity", 31, g(8).to_vec())]),
            ("Creator", 8, vec![("Entity", 31, g(7).to_vec())]),
            ("Items", 7, vec![]),
            ("Item", 11, vec![("Translate", 12, here.clone()), ("Stats", 22, s("OBJ_GoldCoin")), ("Amount", 4, i32v(3))]),
            ("Item", 11, vec![("Translate", 12, away), ("Stats", 22, s("OBJ_GoldPile"))]),
        ];
        let globals = lsf_build(&nodes, true);
        let meta = lsf_build(&[("MetaData", -1, vec![]), ("MetaData", 0, vec![("LeaderName", 23, s("Hero"))])], true);
        let info = br#"{"Active Party":{"Characters":[]},"Save Name":"Test"}"#;
        let raw = pkg_build(18, &[("meta.lsf", &meta), ("SaveInfo.json", info), ("Globals.lsf", &globals)], 0x23);
        let mut d = crate::formats::decode(&raw);
        let crate::formats::Content::Tree(t) = &mut d.content else { panic!("not read") };
        assert_eq!(t.kind_name, BG3);
        let p = "/Characters/Shadowheart/Gold/Stack 1";
        assert_eq!(t.view.pointer(p), Some(&json!(766)), "the entity store's count, not the stale Amount");
        assert!(t.view.pointer("/Characters/Other character 1").is_none(), "gold on the floor isn't carried");
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), raw, "no edit keeps the file");
        let crate::formats::Content::Tree(t) = &mut d.content else { unreachable!() };
        t.set(p, json!(5000)).unwrap();
        assert!(t.set(p, json!(0)).is_err());
        let out = crate::formats::encode(&d.layers, &d.content).unwrap();
        assert!(lspk::read(&out).unwrap().checksum_ok(&out));
        let again = crate::formats::decode(&out);
        let crate::formats::Content::Tree(t2) = &again.content else { panic!("re-read failed") };
        assert_eq!(t2.view.pointer(p), Some(&json!(5000)));
        assert_eq!(t2.view.pointer("/Save/Leader"), Some(&json!("Hero")));
    }

    #[test]
    fn bg3_summary_is_read_only() {
        let meta = lsf_build(&[("MetaData", -1, vec![]), ("MetaData", 0, vec![("LeaderName", 23, s("Hero")), ("Level", 22, s("TestLevel"))])], true);
        let info = br#"{"Active Party":{"Characters":[{"Classes":[{"Main":"Wizard","Sub":"Evocation"}],"Experience Points (Total)":756,"Level":2,"Origin":"Generic","Race":"Human"}]},"Difficulty":["DifficultyHard"],"Save Name":"Test"}"#;
        let raw = pkg_build(18, &[("meta.lsf", &meta), ("SaveInfo.json", info), ("Globals.lsf", b"x")], 0x23);
        let d = crate::formats::decode(&raw);
        let crate::formats::Content::Tree(t) = &d.content else { panic!("not read") };
        assert_eq!(t.kind_name, BG3);
        assert_eq!(crate::game_ui::detect(&t.kind_name, &t.view), Some(crate::game_ui::Game::Bg3));
        assert!(t.editable.is_empty());
        assert_eq!(t.view.pointer("/Party/0/Class"), Some(&json!("Wizard (Evocation)")));
        assert_eq!(t.view.pointer("/Party/0/Experience"), Some(&json!(756)));
        assert_eq!(t.view.pointer("/Save/Leader"), Some(&json!("Hero")));
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), raw);
    }
}

#[cfg(test)]
mod real {
    //! Research aid over sample saves: SAVE_EDITOR_LSV = one or more `.lsv` files separated by `|`.
    use super::*;
    use crate::formats::{decode, encode, Content};

    #[test]
    #[ignore]
    fn real_saves() {
        let list = std::env::var("SAVE_EDITOR_LSV").unwrap();
        for path in list.split('|').filter(|s| !s.is_empty()) {
            let raw = std::fs::read(path).unwrap();
            let pkg = lspk::read(&raw).expect("package");
            assert!(pkg.checksum_ok(&raw), "{path}: the original checksum doesn't match");
            let mut d = decode(&raw);
            println!("== {path}\n{}", crate::formats::describe(&d.layers, &d.content));
            assert_eq!(encode(&d.layers, &d.content).unwrap(), raw, "{path}: no edit keeps the file");
            let Content::Tree(t) = &mut d.content else { panic!("{path}: not read") };
            println!("{}", serde_json::to_string_pretty(&t.view).unwrap().chars().take(3000).collect::<String>());
            assert!(crate::game_ui::detect(&t.kind_name, &t.view).is_some(), "no screen");
            // Rebuilding the package with nothing replaced keeps every file's bytes and place.
            let (file, _plain) = open(&raw).unwrap();
            let same = pkg.rebuild(&raw, &HashMap::new()).unwrap();
            let list_at = pkg.entries.iter().map(|e| e.offset + e.size_on_disk).max().unwrap() as usize;
            assert_eq!(same[40..list_at], raw[40..list_at], "{path}: file data laid out as the game does");
            println!("   rebuilt package identical to the original: {}", same == raw);
            // Packing the opened file again (zlib / zstd) reads back the same.
            let orig = pkg.file(&raw, pkg.find(&file).unwrap()).unwrap();
            let repacked = pkg.rebuild(&raw, &HashMap::from([(file.clone(), orig.clone())])).unwrap();
            let rp = lspk::read(&repacked).unwrap();
            assert!(rp.checksum_ok(&repacked));
            assert_eq!(rp.file(&repacked, rp.find(&file).unwrap()).unwrap(), orig);
            if t.kind_name == DOS2 {
                let first = t.view["Party"].as_object().unwrap().keys().next().unwrap().clone();
                let base = format!("/Party/{}", crate::tree::esc(&first));
                let xp = t.view.pointer(&format!("{base}/Experience")).and_then(Value::as_i64).unwrap();
                let st = t.view.pointer(&format!("{base}/Attributes/Strength")).and_then(Value::as_i64).unwrap();
                t.set(&format!("{base}/Experience"), json!(xp + 1)).unwrap();
                t.set(&format!("{base}/Attributes/Strength"), json!(st + 1)).unwrap();
                let gold = t.view.pointer("/Party").and_then(Value::as_object).unwrap().iter().find_map(|(k, m)| {
                    let g = m.get("Gold")?.as_object()?.iter().next()?;
                    Some((format!("/Party/{}/Gold/{}", crate::tree::esc(k), crate::tree::esc(g.0)), g.1.as_i64()?))
                });
                if let Some((p, g)) = &gold {
                    t.set(p, json!(g + 1)).unwrap();
                }
                let before = t.view.clone();
                let out = encode(&d.layers, &d.content).unwrap();
                let p2 = lspk::read(&out).unwrap();
                assert!(p2.checksum_ok(&out), "{path}: checksum after the edit");
                let again = decode(&out);
                let Content::Tree(t2) = &again.content else { panic!("re-read failed") };
                assert_eq!(t2.view, before, "{path}: the edit reads back, the rest unchanged");
                // Every other file unpacks to the same bytes; the LSF differs only in the edited values.
                for e in &pkg.entries {
                    let a = pkg.file(&raw, e).unwrap();
                    let b = p2.file(&out, p2.find(&e.name).unwrap()).unwrap();
                    if e.name == file {
                        let (ua, ub) = (lsf::unpack(&a).unwrap(), lsf::unpack(&b).unwrap());
                        assert_eq!(ua.len(), ub.len());
                        let diff = ua.iter().zip(&ub).filter(|(x, y)| x != y).count();
                        println!("   {} bytes changed in {file}", diff);
                        assert!(diff <= 12);
                    } else {
                        assert_eq!(a, b, "{}", e.name);
                    }
                }
                if let Ok(dir) = std::env::var("LSV_OUT") {
                    let name = std::path::Path::new(path).file_name().unwrap();
                    std::fs::write(std::path::Path::new(&dir).join(name), &out).unwrap();
                }
            } else {
                // BG3: one stack size changed through the entity store, then the package written again.
                let chars = t.view.get("Characters").and_then(Value::as_object).cloned().unwrap_or_default();
                for (k, c) in &chars {
                    let gold: i64 = c.get("Gold").and_then(Value::as_object).map(|m| m.values().filter_map(Value::as_i64).sum()).unwrap_or(0);
                    let items = c.get("Items").and_then(Value::as_object).map_or(0, |m| m.len());
                    println!("   {k}: gold {gold} (total {}), {items} stacked items", c["Gold total"]);
                }
                // Story flags: one set flag cleared and one cleared flag set, on a fresh copy.
                {
                    let mut fd = decode(&raw);
                    let Content::Tree(ft) = &mut fd.content else { unreachable!() };
                    let mut on = None;
                    let mut off = None;
                    for (sec, m) in ft.view.get("Story flags").and_then(Value::as_object).cloned().unwrap_or_default() {
                        for (k, v) in m.as_object().cloned().unwrap_or_default() {
                            let p = format!("/Story flags/{}/{}", crate::tree::esc(&sec), crate::tree::esc(&k));
                            if v == json!(1) && on.is_none() { on = Some((p, k)); } else if v == json!(0) && off.is_none() { off = Some((p, k)); }
                        }
                    }
                    let n = ft.view.get("Story flags").and_then(Value::as_object).map_or(0, |m| m.values().map(|x| x.as_object().map_or(0, |o| o.len())).sum::<usize>());
                    println!("   {n} named flags; clearing {:?}, setting {:?}", on.as_ref().map(|x| &x.1), off.as_ref().map(|x| &x.1));
                    if let (Some((pon, kon)), Some((poff, koff))) = (on, off) {
                        ft.set(&pon, json!(0)).unwrap();
                        ft.set(&poff, json!(1)).unwrap();
                        let out = encode(&fd.layers, &fd.content).unwrap();
                        let p2 = lspk::read(&out).unwrap();
                        assert!(p2.checksum_ok(&out));
                        let again = decode(&out);
                        let Content::Tree(t2) = &again.content else { panic!("re-read failed") };
                        // A cleared flag named only in the story's list has no name left, so it may drop out.
                        assert!(matches!(t2.view.pointer(&pon), None | Some(&Value::Number(_))) && t2.view.pointer(&pon) != Some(&json!(1)));
                        assert_eq!(t2.view.pointer(&poff), Some(&json!(1)));
                        let st = crate::fmt::osiris::read(&p2.file(&out, p2.find("StorySave.bin").unwrap()).unwrap()).unwrap();
                        let st0 = crate::fmt::osiris::read(&pkg.file(&raw, pkg.find("StorySave.bin").unwrap()).unwrap()).unwrap();
                        let list = |s: &crate::fmt::osiris::Story| -> Vec<String> { s.db("DB_GlobalFlag").unwrap().facts.iter().filter_map(|f| match &f.values[0] { crate::fmt::osiris::Val::Str(x) => Some(crate::fmt::osiris::strip_guid(x).to_string()), _ => None }).collect() };
                        let (a, b) = (list(&st0), list(&st));
                        let base = |k: &str| k.split(" (").next().unwrap().to_string();
                        assert!(a.contains(&base(&kon)) && !b.contains(&base(&kon)), "cleared flag left the story's list");
                        assert!(!a.contains(&base(&koff)) && b.contains(&base(&koff)), "set flag joined the story's list");
                        assert_eq!(a.len(), b.len());
                        // Every other database is unchanged.
                        for (x, y) in st0.databases.iter().zip(&st.databases) {
                            if x.name != "DB_GlobalFlag" {
                                assert_eq!(x.facts.len(), y.facts.len(), "{}", x.name);
                            }
                        }
                        for e in &pkg.entries {
                            if e.name != file && e.name != "StorySave.bin" {
                                assert_eq!(pkg.file(&raw, e).unwrap(), p2.file(&out, p2.find(&e.name).unwrap()).unwrap(), "{}", e.name);
                            }
                        }
                    }
                }
                let target = chars.iter().find_map(|(k, c)| {
                    c.get("Gold")?.as_object()?.iter().find_map(|(n, v)| {
                        let p = format!("/Characters/{}/Gold/{}", crate::tree::esc(k), crate::tree::esc(n));
                        t.editable.contains_key(&p).then(|| Some((p, v.as_i64()?))).flatten()
                    })
                });
                let Some((p, g)) = target else { println!("   no gold stack to edit"); continue };
                t.set(&p, json!(g + 1234)).unwrap();
                let before = t.view.clone();
                let out = encode(&d.layers, &d.content).unwrap();
                let p2 = lspk::read(&out).unwrap();
                assert!(p2.checksum_ok(&out), "{path}: checksum after the edit");
                let again = decode(&out);
                let Content::Tree(t2) = &again.content else { panic!("re-read failed") };
                assert_eq!(t2.view.pointer(&p), before.pointer(&p), "{path}: the edit reads back");
                for e in &pkg.entries {
                    let a = pkg.file(&raw, e).unwrap();
                    let b = p2.file(&out, p2.find(&e.name).unwrap()).unwrap();
                    if e.name == file {
                        let (ua, ub) = (lsf::unpack(&a).unwrap(), lsf::unpack(&b).unwrap());
                        assert_eq!(ua.len(), ub.len());
                        let diff = ua.iter().zip(&ub).filter(|(x, y)| x != y).count();
                        assert!((1..=4).contains(&diff), "{diff} bytes changed");
                    } else {
                        assert_eq!(a, b, "{}", e.name);
                    }
                }
            }
        }
    }
}
