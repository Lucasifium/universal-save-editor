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
//! Baldur's Gate 3 keeps character state in a packed entity store inside `Globals.lsf`
//! that isn't mapped here; the save's own summary (`SaveInfo.json`, `meta.lsf`) is shown
//! read-only.

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

/// The LSF file the editor opens: `globals.lsf` (DOS2) or `meta.lsf` (BG3), unpacked.
pub fn open(data: &[u8]) -> Option<(String, Vec<u8>)> {
    let pkg = lspk::read(data)?;
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
    pkg.rebuild(archive, &HashMap::from([(e.name.clone(), packed)]))
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

fn dos2(plain: &[u8], archive: &[u8], pkg: &lspk::Package) -> Option<TreeDoc> {
    let l = Lsf::read(plain)?;
    let mut b = Builder::new(plain);
    for (k, v) in meta_facts(archive, pkg) {
        b.info(&["Save", k], v);
    }
    // Coin stacks by the inventory that holds them.
    let mut gold: HashMap<i64, Vec<usize>> = HashMap::new();
    for item in l.at_path(&["Items", "ItemFactory", "Items", "Item"]) {
        let (Some(stats), Some(parent)) = (l.string_of(item, "Stats"), l.int_of(item, "Parent")) else { continue };
        if is_gold(&stats) && l.attr(item, "Amount").is_some() {
            gold.entry(parent).or_default().push(item);
        }
    }
    let mut seen = HashMap::new();
    let mut members = 0;
    for c in l.at_path(&["Characters", "CharacterFactory", "Characters", "Character"]) {
        let Some(stats) = l.child(c, "Stats") else { continue };
        if l.int_of(stats, "IsPlayer") != Some(1) {
            continue;
        }
        let custom = l.child(c, "PlayerData").and_then(|p| l.child(p, "PlayerCustomData"));
        let text = |k: &str| custom.and_then(|p| l.string_of(p, k)).filter(|s| !s.is_empty());
        let name = text("Name").or_else(|| text("OriginName")).unwrap_or_else(|| format!("Character {}", members + 1));
        let key = unique(&mut seen, name);
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
    }
    if members == 0 {
        return None;
    }
    Some(b.finish(DOS2, Checksum::None))
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
    Some(b.finish(BG3, Checksum::None).with_note(
        "Baldur's Gate 3: the party summary is shown read-only. Character stats, hit points and gold \
         live in a packed part of the save that isn't mapped yet.",
    ))
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
        assert_eq!(v.pointer("/Party/Testa/Gold").and_then(Value::as_object).map(|m| m.len()), Some(1), "a gold cup isn't gold");
        assert_eq!(v.pointer("/Save/Region"), Some(&json!("TestRegion")));
        assert_eq!(v.pointer("/Party").and_then(Value::as_object).map(|m| m.len()), Some(1), "NPCs aren't in the party");
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), raw, "no edit keeps the file");
        let mut d = d;
        let crate::formats::Content::Tree(t) = &mut d.content else { unreachable!() };
        t.set("/Party/Testa/Experience", json!(3351)).unwrap();
        t.set("/Party/Testa/Attributes/Strength", json!(12)).unwrap();
        t.set("/Party/Testa/Gold/Small_Gold", json!(310)).unwrap();
        assert!(t.set("/Party/Testa/Gold/Small_Gold", json!(0)).is_err());
        let out = crate::formats::encode(&d.layers, &d.content).unwrap();
        let p = lspk::read(&out).unwrap();
        assert!(p.checksum_ok(&out));
        let again = crate::formats::decode(&out);
        let crate::formats::Content::Tree(t2) = &again.content else { panic!("re-read failed") };
        assert_eq!(t2.view.pointer("/Party/Testa/Experience"), Some(&json!(3351)));
        assert_eq!(t2.view.pointer("/Party/Testa/Attributes/Strength"), Some(&json!(12)));
        assert_eq!(t2.view.pointer("/Party/Testa/Gold/Small_Gold"), Some(&json!(310)));
        assert_eq!(t2.view.pointer("/Party/Testa/Attributes/Wits"), Some(&json!(14)));
        // The other files are untouched.
        let q = lspk::read(&raw).unwrap();
        for n in ["meta.lsf", "test.png"] {
            assert_eq!(p.file(&out, p.find(n).unwrap()), q.file(&raw, q.find(n).unwrap()));
        }
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
            let (file, plain) = open(&raw).unwrap();
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
                // BG3: a raw change to meta.lsf (its seed) goes through the zstd package writer.
                let l = Lsf::read(&plain).unwrap();
                let m = l.at_path(&["MetaData", "MetaData"])[0];
                let a = l.attr(m, "Seed").unwrap();
                let mut edited = plain.clone();
                edited[a.at] ^= 1;
                let out = save(&raw, &file, &edited).unwrap();
                let p2 = lspk::read(&out).unwrap();
                assert!(p2.checksum_ok(&out), "{path}: checksum after the edit");
                let (f2, plain2) = open(&out).unwrap();
                assert_eq!(f2, file);
                assert_eq!(plain2, edited);
                for e in &pkg.entries {
                    if e.name != file {
                        assert_eq!(pkg.file(&raw, e).unwrap(), p2.file(&out, p2.find(&e.name).unwrap()).unwrap(), "{}", e.name);
                    }
                }
            }
        }
    }
}
