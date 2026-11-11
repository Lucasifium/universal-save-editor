//! Neverwinter Nights (Enhanced Edition and 1.69). A save is a folder
//! (`saves\000002 - <name>\`) holding `<Module>.sav`, an ERF of the module's state whose
//! `Module.ifo` keeps the player in `Mod_PlayerList`, and `player.bic`, a copy of the player
//! (a creature GFF). Characters in `localvault\` are `.bic` files on their own.
//!
//! Opening the `.sav` edits the player the game loads; when it's saved, the same values are
//! copied into `player.bic` beside it (see `companions`). Every value is a number edited in
//! place (see `gff.rs`). Field labels follow the public BioWare creature format and were
//! checked on sample NWN:EE saves and server-vault characters.

use super::fields::{Builder, Checksum};
use super::gff::{entry_gff, erf_entries, is_erf, At, Gff, T_IFO, T_SAV};
use crate::tree::TreeDoc;
use std::path::{Path, PathBuf};

/// skills.2da rows (checked against ranks on sample characters: a wizard's Concentration,
/// Lore and Spellcraft, a barbarian's Discipline, Listen and Taunt).
const SKILLS: [&str; 28] = [
    "Animal Empathy", "Concentration", "Disable Trap", "Discipline", "Heal", "Hide", "Listen", "Lore", "Move Silently", "Open Lock",
    "Parry", "Perform", "Persuade", "Pick Pocket", "Search", "Set Trap", "Spellcraft", "Spot", "Taunt", "Use Magic Device", "Appraise",
    "Tumble", "Craft Trap", "Bluff", "Intimidate", "Craft Armor", "Craft Weapon", "Ride",
];

/// The base classes (classes.2da rows 0–10); later rows are monster and prestige classes.
const CLASSES: [&str; 11] = ["Barbarian", "Bard", "Cleric", "Druid", "Fighter", "Monk", "Paladin", "Ranger", "Rogue", "Sorcerer", "Wizard"];
const RACES: [&str; 7] = ["Dwarf", "Elf", "Gnome", "Halfling", "Half-elf", "Half-orc", "Human"];

fn pick(table: &[&str], i: i64, what: &str) -> String {
    usize::try_from(i).ok().and_then(|i| table.get(i)).map_or_else(|| format!("{what} {i}"), |s| s.to_string())
}

/// The player in a module save (`Module.ifo` → `Mod_PlayerList[0]`), if it's an NWN one.
fn save_player(d: &[u8]) -> Option<At<'_>> {
    let top = erf_entries(d, 0, d.len())?;
    // KOTOR saves nest one archive per module; NWN's don't.
    if top.iter().any(|e| e.ty == T_SAV) {
        return None;
    }
    let ifo = top.iter().find(|e| e.ty == T_IFO && e.name.eq_ignore_ascii_case("Module")).and_then(|e| entry_gff(d, e))?;
    let pc = *At::root(ifo).list("Mod_PlayerList").first()?;
    (pc.has("Str") && pc.has("Gold") && !pc.has("ForcePoints")).then_some(pc)
}

fn bic(d: &[u8]) -> Option<At<'_>> {
    let g = Gff::open(d, 0, d.len())?;
    let pc = At::root(g);
    (&g.file_type == b"BIC " && pc.has("Str")).then_some(pc)
}

pub fn looks(d: &[u8]) -> bool {
    if is_erf(d, 0) {
        save_player(d).is_some()
    } else {
        d.starts_with(b"BIC V3.2") && bic(d).is_some()
    }
}

/// The values edited (and copied to player.bic): label, where in the view, lowest, highest.
const NUMBERS: [(&str, &[&str], i64, i64); 14] = [
    ("Gold", &["Gold"], 0, i32::MAX as i64),
    ("Experience", &["Experience"], 0, i32::MAX as i64),
    ("SkillPoints", &["Skill points"], 0, 9999),
    ("GoodEvil", &["Alignment", "Good\u{2013}evil"], 0, 100),
    ("LawfulChaotic", &["Alignment", "Law\u{2013}chaos"], 0, 100),
    ("Str", &["Attributes", "Strength"], 1, 100),
    ("Dex", &["Attributes", "Dexterity"], 1, 100),
    ("Con", &["Attributes", "Constitution"], 1, 100),
    ("Int", &["Attributes", "Intelligence"], 1, 100),
    ("Wis", &["Attributes", "Wisdom"], 1, 100),
    ("Cha", &["Attributes", "Charisma"], 1, 100),
    ("CurrentHitPoints", &["Health", "Current HP"], i16::MIN as i64, i16::MAX as i64),
    ("MaxHitPoints", &["Health", "Max HP"], 0, i16::MAX as i64),
    ("HitPoints", &["Health", "Base HP"], 0, i16::MAX as i64),
];

fn character(b: &mut Builder, c: At) {
    let first = c.text("FirstName").unwrap_or_default();
    let last = c.text("LastName").unwrap_or_default();
    b.info(&["Character", "Name"], format!("{} {}", first.trim(), last.trim()).trim());
    if let Some(r) = c.int("Race") {
        b.info(&["Character", "Race"], pick(&RACES, r, "Race"));
    }
    let classes: Vec<(i64, i64)> = c.list("ClassList").iter().filter_map(|x| Some((x.int("Class")?, x.int("ClassLevel")?))).collect();
    if !classes.is_empty() {
        let text: Vec<String> = classes.iter().map(|(k, l)| format!("{} {l}", pick(&CLASSES, *k, "Class"))).collect();
        b.info(&["Character", "Classes"], text.join(" / "));
        b.info(&["Character", "Level"], classes.iter().map(|x| x.1).sum::<i64>());
    }
    for (label, sub, lo, hi) in NUMBERS.iter() {
        let path: Vec<&str> = std::iter::once("Character").chain(sub.iter().copied()).collect();
        c.edit(b, &path, label, *lo, *hi, &[]);
    }
    for (i, s) in c.list("SkillList").iter().enumerate() {
        let name = SKILLS.get(i).map_or_else(|| format!("Skill {i}"), |s| s.to_string());
        s.edit(b, &["Character", "Skills", &name], "Rank", 0, 127, &[]);
    }
    let mut seen: Vec<(String, i64)> = vec![];
    for it in c.list("ItemList") {
        let tag = it.text("Tag").unwrap_or_default();
        let n = it.int("StackSize").unwrap_or(1).max(1);
        match seen.iter_mut().find(|x| x.0 == tag) {
            Some(x) => x.1 += n,
            None => seen.push((tag, n)),
        }
    }
    b.info(&["Inventory"], seen.into_iter().map(|(t, n)| if n > 1 { format!("{t} \u{00D7}{n}") } else { t }).collect::<Vec<_>>());
}

pub fn parse(d: &[u8], path: Option<&Path>) -> Result<TreeDoc, String> {
    let mut b = Builder::new(d);
    b.info(&["Save", "Game"], "Neverwinter Nights");
    if is_erf(d, 0) {
        let pc = save_player(d).ok_or("no player in this save")?;
        if let Some(area) = super::kotor::sibling(path, "savenfo.txt").and_then(|p| std::fs::read(p).ok()) {
            let area: String = area.iter().map(|&c| super::bethesda_cp1252(c)).collect();
            b.info(&["Save", "Area"], area.trim());
        }
        if let Some(m) = pc.g.text(0, "Mod_Name").filter(|m| !m.is_empty()) {
            b.info(&["Save", "Module"], m);
        }
        character(&mut b, pc);
        Ok(b.finish("Neverwinter Nights save", Checksum::None).with_note(
            "The player as the game loads it (Module.ifo in the save). When you save, player.bic in the same folder gets the same values.",
        ))
    } else {
        let pc = bic(d).ok_or("not a character file")?;
        character(&mut b, pc);
        Ok(b.finish("Neverwinter Nights character", Checksum::None).with_note(
            "A character file. In a save folder the game loads the player from the .sav instead: open that (player.bic is updated with it).",
        ))
    }
}

/// After a module save is written: the same values in `player.bic` beside it.
pub fn companions(path: &Path, bytes: &[u8]) -> Vec<(PathBuf, Vec<u8>)> {
    let Some(pc) = is_erf(bytes, 0).then(|| save_player(bytes)).flatten() else { return vec![] };
    let Some(file) = super::kotor::sibling(Some(path), "player.bic") else { return vec![] };
    let Ok(old) = std::fs::read(&file) else { return vec![] };
    let mut new = old.clone();
    let Some(them) = bic(&old) else { return vec![] };
    let mut copy = |from: At, to: At, label: &str| {
        let (Some(f), Some(t)) = (from.g.get(from.s, label), to.g.get(to.s, label)) else { return };
        if f.ty != t.ty {
            return;
        }
        if let (Some((a, k)), Some((b, _))) = (from.g.number(&f), to.g.number(&t)) {
            new[b..b + k.size()].copy_from_slice(&bytes[a..a + k.size()]);
        }
    };
    for (label, ..) in NUMBERS.iter() {
        copy(pc, them, label);
    }
    for (a, b) in pc.list("SkillList").into_iter().zip(them.list("SkillList")) {
        copy(a, b, "Rank");
    }
    if new == old {
        return vec![];
    }
    vec![(file, new)]
}

#[cfg(test)]
mod tests {
    use super::super::gff::build::{erf, gff, V};
    use super::*;
    use serde_json::Value;

    fn pc() -> Vec<(&'static str, V)> {
        vec![
            ("FirstName", V::Loc(-1, "Aribeth")),
            ("LastName", V::Loc(-1, "de Tylmarande")),
            ("Race", V::Byte(4)),
            ("Gold", V::Dword(75)),
            ("Str", V::Byte(10)),
            ("Dex", V::Byte(14)),
            ("Con", V::Byte(14)),
            ("Int", V::Byte(16)),
            ("Wis", V::Byte(12)),
            ("Cha", V::Byte(10)),
            ("HitPoints", V::Short(5)),
            ("CurrentHitPoints", V::Short(-2)),
            ("MaxHitPoints", V::Short(5)),
            ("Experience", V::Dword(0)),
            ("GoodEvil", V::Byte(50)),
            ("LawfulChaotic", V::Byte(15)),
            ("SkillPoints", V::Word(0)),
            ("ClassList", V::List(vec![(2, vec![("Class", V::Int(6)), ("ClassLevel", V::Short(3))])])),
            ("SkillList", V::List((0..29).map(|i| (0, vec![("Rank", V::Byte((i % 5) as u8))])).collect())),
            ("ItemList", V::List(vec![(0, vec![("Tag", V::Str("NW_WAMBO001")), ("StackSize", V::Word(99))])])),
        ]
    }

    fn save() -> Vec<u8> {
        erf(&[
            ("Repute", 2038, gff("FAC ", vec![])),
            ("Module", T_IFO, gff("IFO ", vec![("Mod_Name", V::Loc(-1, "Prelude")), ("Mod_PlayerList", V::List(vec![(0, pc())]))])),
        ])
    }

    #[test]
    fn edits_the_save_and_keeps_player_bic_in_step() {
        let d = save();
        assert!(looks(&d));
        let dir = std::env::temp_dir().join(format!("nwn_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sav = dir.join("Prelude.sav");
        std::fs::write(dir.join("savenfo.txt"), "Barracks").unwrap();
        let bic_bytes = gff("BIC ", pc());
        std::fs::write(dir.join("player.bic"), &bic_bytes).unwrap();
        let mut doc = parse(&d, Some(&sav)).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Save/Area"), Some(&Value::from("Barracks")));
        assert_eq!(v.pointer("/Save/Module"), Some(&Value::from("Prelude")));
        assert_eq!(v.pointer("/Character/Name"), Some(&Value::from("Aribeth de Tylmarande")));
        assert_eq!(v.pointer("/Character/Race"), Some(&Value::from("Half-elf")));
        assert_eq!(v.pointer("/Character/Classes"), Some(&Value::from("Paladin 3")));
        assert_eq!(v.pointer("/Character/Health/Current HP"), Some(&Value::from(-2)));
        assert_eq!(v.pointer("/Character/Skills/Spellcraft"), Some(&Value::from(1)));
        assert_eq!(v.pointer("/Character/Skills/Skill 28"), Some(&Value::from(3)));
        assert_eq!(v.pointer("/Inventory/0"), Some(&Value::from("NW_WAMBO001 \u{00D7}99")));
        // Nothing changed: player.bic is left alone.
        assert!(companions(&sav, &d).is_empty());
        doc.set("/Character/Gold", Value::from(76)).unwrap();
        doc.set("/Character/Attributes/Strength", Value::from(18)).unwrap();
        doc.set("/Character/Skills/Lore", Value::from(9)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out, None).unwrap();
        assert_eq!(again.view.pointer("/Character/Gold"), Some(&Value::from(76)));
        assert_eq!(again.view.pointer("/Character/Attributes/Dexterity"), Some(&Value::from(14)));
        let side = companions(&sav, &out);
        assert_eq!(side.len(), 1);
        assert_eq!(side[0].1.len(), bic_bytes.len());
        let b = parse(&side[0].1, None).unwrap();
        assert_eq!(b.kind_name, "Neverwinter Nights character");
        assert_eq!(b.view.pointer("/Character/Gold"), Some(&Value::from(76)));
        assert_eq!(b.view.pointer("/Character/Attributes/Strength"), Some(&Value::from(18)));
        assert_eq!(b.view.pointer("/Character/Skills/Lore"), Some(&Value::from(9)));
        assert_eq!(b.view.pointer("/Character/Skills/Heal"), Some(&Value::from(4)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Research aid: SAVE_EDITOR_NWN = .sav and .bic files separated by `|`.
    #[test]
    #[ignore]
    fn real_files() {
        for p in std::env::var("SAVE_EDITOR_NWN").unwrap().split('|').filter(|p| !p.is_empty()) {
            let d = std::fs::read(p).unwrap();
            assert!(looks(&d), "{p}");
            let mut doc = parse(&d, Some(Path::new(p))).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d);
            let before = doc.view.clone();
            println!("{p}: {}", serde_json::to_string(&before).unwrap().chars().take(400).collect::<String>());
            let gold = before.pointer("/Character/Gold").and_then(Value::as_i64).unwrap();
            doc.set("/Character/Gold", Value::from(gold + 1)).unwrap();
            doc.set("/Character/Attributes/Strength", Value::from(18)).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out, Some(Path::new(p))).unwrap();
            let mut want = before.clone();
            *want.pointer_mut("/Character/Gold").unwrap() = Value::from(gold + 1);
            *want.pointer_mut("/Character/Attributes/Strength").unwrap() = Value::from(18);
            assert_eq!(again.view, want);
            let side = companions(Path::new(p), &out);
            println!("  edit ok; companion files updated: {}", side.len());
            for (f, bytes) in side {
                let b = parse(&bytes, None).unwrap();
                assert_eq!(b.view.pointer("/Character/Gold"), Some(&Value::from(gold + 1)), "{}", f.display());
                assert_eq!(b.view.pointer("/Character/Attributes/Strength"), Some(&Value::from(18)));
                let old = std::fs::read(&f).unwrap();
                println!("  {} bytes of player.bic would change", old.iter().zip(&bytes).filter(|(a, b)| a != b).count());
            }
        }
    }
}
