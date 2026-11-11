//! Star Wars: Knights of the Old Republic 1 and 2 (The Sith Lords). A save is a folder
//! (`saves\000012 - Game11\`) of BioWare GFF files (see `gff.rs`):
//!
//! - `SAVEGAME.sav`: an ERF with the party members (`AVAILNPC<n>`, creature GFFs), the party
//!   inventory, an old copy of the player (`PC`) and one nested ERF per visited module, each
//!   with a `Module.ifo` whose `Mod_PlayerList` holds a copy of the player. The copy the game
//!   loads is the one in the module named by `savenfo.res` (`LASTMODULE`); auto-saves may
//!   keep the player in `pifo.ifo` beside it instead.
//! - `PARTYTABLE.res`: credits, the party XP pool and (KOTOR 2) components, chemicals and
//!   each companion's influence.
//!
//! Each file opens on its own; the screen says which file holds what. Every value is a
//! number edited in place, so nothing in the archives moves. Player edits are written to
//! every copy of the player in `SAVEGAME.sav`, so the game finds them whichever it reads.
//!
//! Field labels and the party layout were checked on sample KOTOR 1 and 2 saves; the save
//! layout matches the public BioWare GFF/ERF descriptions, xoreos, KotOR Savegame Editor and
//! sotor (all GPL; facts only).

use super::fields::{Builder, Checksum};
use super::gff::{entry_gff, erf_entries, is_erf, At, Gff, T_IFO, T_SAV, T_UTC};
use crate::tree::TreeDoc;
use std::path::Path;

/// skills.2da order (the same in both games).
pub const SKILLS: [&str; 8] = [
    "Computer Use",
    "Demolitions",
    "Stealth",
    "Awareness",
    "Persuade",
    "Repair",
    "Security",
    "Treat Injury",
];

/// classes.2da rows (KOTOR 2 adds the prestige classes from 9 on). Checked on sample saves:
/// Bastila 5, Carth 0, HK-47 6, Jolee 4, Juhani 3, Mission 2, T3-M4 7, Zaalbar 1;
/// Atton 2 + 5, Bao-Dur 9, Kreia 4, Visas 5, the player 13 (Jedi Watchman).
const CLASSES: [&str; 17] = [
    "Soldier",
    "Scout",
    "Scoundrel",
    "Jedi Guardian",
    "Jedi Consular",
    "Jedi Sentinel",
    "Combat Droid",
    "Expert Droid",
    "Minion",
    "Tech Specialist",
    "Bounty Hunter",
    "Jedi Weapon Master",
    "Jedi Master",
    "Jedi Watchman",
    "Sith Marauder",
    "Sith Lord",
    "Sith Assassin",
];

/// The party NPC numbers (`AVAILNPC<n>`, the party table's lists). Checked against the
/// creatures' tags in sample saves; KOTOR 2's 10 and 11 (not in the saves we have) follow the
/// game's NPC_HANHARR and NPC_DISCIPLE script constants.
const NPCS1: [&str; 9] = [
    "Bastila",
    "Canderous",
    "Carth",
    "HK-47",
    "Jolee",
    "Juhani",
    "Mission",
    "T3-M4",
    "Zaalbar",
];
const NPCS2: [&str; 12] = [
    "Atton",
    "Bao-Dur",
    "Mandalore",
    "G0-T0",
    "Handmaiden",
    "HK-47",
    "Kreia",
    "Mira",
    "T3-M4",
    "Visas",
    "Hanharr",
    "Disciple",
];

fn class_name(c: i64) -> String {
    usize::try_from(c)
        .ok()
        .and_then(|i| CLASSES.get(i))
        .map_or_else(|| format!("Class {c}"), |s| s.to_string())
}

/// A file next to `path`, found whatever its case.
pub(crate) fn sibling(path: Option<&Path>, name: &str) -> Option<std::path::PathBuf> {
    let dir = path?.parent()?;
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name))
        })
}

fn is_k2(c: &At) -> bool {
    c.has("ItemComponent") || c.has("BonusForcePoints")
}

/// The player struct of a `Module.ifo`/`pifo.ifo`, if it holds a KOTOR creature.
fn player_of(g: Gff) -> Option<At> {
    let pc = *At::root(g).list("Mod_PlayerList").first()?;
    pc.has("ForcePoints").then_some(pc)
}

/// What kind of KOTOR file this is, if any.
enum File {
    Save,
    Party,
    Player,
}

fn which(d: &[u8]) -> Option<File> {
    if is_erf(d, 0) {
        let e = erf_entries(d, 0, d.len())?;
        let creatures = e.iter().any(|x| {
            x.ty == T_UTC
                && (x.name.eq_ignore_ascii_case("PC")
                    || x.name.to_ascii_uppercase().starts_with("AVAILNPC"))
        });
        return (creatures && e.iter().any(|x| x.ty == T_SAV)).then_some(File::Save);
    }
    let g = Gff::open(d, 0, d.len())?;
    match &g.file_type {
        b"PT  " if g.get(0, "PT_GOLD").is_some() => Some(File::Party),
        b"IFO " if player_of(g).is_some() => Some(File::Player),
        _ => None,
    }
}

pub fn looks(d: &[u8]) -> bool {
    which(d).is_some()
}

/// Builds one creature's values under `base`; `copies` are other copies written along.
fn creature(b: &mut Builder, base: &[&str], c: At, copies: &[At]) {
    let at = |sub: &[&'static str]| -> Vec<&str> {
        base.iter().copied().chain(sub.iter().copied()).collect()
    };
    let mut one = |sub: &[&'static str], label: &str, lo: i64, hi: i64| {
        let path = at(sub);
        c.edit(b, &path, label, lo, hi, copies);
    };
    one(&["Experience"], "Experience", 0, i32::MAX as i64);
    one(&["Alignment"], "GoodEvil", 0, 100);
    one(&["Skill points"], "SkillPoints", 0, 9999);
    for (name, label) in [
        ("Strength", "Str"),
        ("Dexterity", "Dex"),
        ("Constitution", "Con"),
        ("Intelligence", "Int"),
        ("Wisdom", "Wis"),
        ("Charisma", "Cha"),
    ] {
        one(&["Attributes", name], label, 1, 100);
    }
    for (name, label, lo) in [
        ("Current HP", "CurrentHitPoints", i16::MIN as i64),
        ("Max HP", "MaxHitPoints", 0),
        ("Base HP", "HitPoints", 0),
        ("Current FP", "CurrentForce", i16::MIN as i64),
        ("Max FP", "MaxForcePoints", 0),
        ("Base FP", "ForcePoints", 0),
    ] {
        one(&["Health", name], label, lo, i16::MAX as i64);
    }
    let skills = c.list("SkillList");
    let theirs: Vec<Vec<At>> = copies.iter().map(|x| x.list("SkillList")).collect();
    for (i, s) in skills.iter().enumerate().take(SKILLS.len()) {
        let mirrors: Vec<At> = theirs.iter().filter_map(|l| l.get(i).copied()).collect();
        s.edit(b, &at(&["Skills", SKILLS[i]]), "Rank", 0, 99, &mirrors);
    }
    let classes: Vec<(i64, i64)> = c
        .list("ClassList")
        .iter()
        .filter_map(|x| Some((x.int("Class")?, x.int("ClassLevel")?)))
        .collect();
    if !classes.is_empty() {
        let text: Vec<String> = classes
            .iter()
            .map(|(k, l)| format!("{} {l}", class_name(*k)))
            .collect();
        b.info(&at(&["Classes"]), text.join(" / "));
        b.info(&at(&["Level"]), classes.iter().map(|x| x.1).sum::<i64>());
    }
}

/// A party member past the known list: its own first name, else its tag.
fn name_of(c: &At) -> String {
    if let Some(n) = c.text("FirstName").filter(|n| !n.trim().is_empty()) {
        return n.trim().to_string();
    }
    let tag = c.text("Tag").unwrap_or_default();
    let mut ch = tag.chars();
    ch.next()
        .map(|f| f.to_uppercase().chain(ch).collect())
        .unwrap_or_else(|| "Companion".into())
}

pub fn parse(d: &[u8], path: Option<&Path>) -> Result<TreeDoc, String> {
    match which(d).ok_or("not a KOTOR save file")? {
        File::Save => parse_save(d, path),
        File::Party => parse_party(d),
        File::Player => parse_player(d),
    }
}

fn parse_save(d: &[u8], path: Option<&Path>) -> Result<TreeDoc, String> {
    let top = erf_entries(d, 0, d.len()).ok_or("broken archive")?;
    // Every copy of the player: one per visited module, and the old top-level PC.
    let mut modules: Vec<(String, At)> = vec![];
    for e in top.iter().filter(|e| e.ty == T_SAV) {
        let Some(inner) = erf_entries(d, e.at, e.at + e.size) else {
            continue;
        };
        let pc = inner
            .iter()
            .find(|x| x.ty == T_IFO && x.name.eq_ignore_ascii_case("Module"))
            .and_then(|x| entry_gff(d, x))
            .and_then(player_of);
        if let Some(pc) = pc {
            modules.push((e.name.clone(), pc));
        }
    }
    let old_pc = top
        .iter()
        .find(|e| e.ty == T_UTC && e.name.eq_ignore_ascii_case("PC"))
        .and_then(|e| entry_gff(d, e))
        .map(At::root);
    let mut npcs: Vec<(usize, At)> = top
        .iter()
        .filter(|e| e.ty == T_UTC)
        .filter_map(|e| {
            Some((
                e.name
                    .to_ascii_uppercase()
                    .strip_prefix("AVAILNPC")?
                    .parse::<usize>()
                    .ok()?,
                entry_gff(d, e)?,
            ))
        })
        .map(|(n, g)| (n, At::root(g)))
        .collect();
    npcs.sort_by_key(|x| x.0);

    let k2 = modules
        .iter()
        .map(|m| m.1)
        .chain(old_pc)
        .chain(npcs.iter().map(|n| n.1))
        .any(|c| is_k2(&c));
    let mut b = Builder::new(d);
    b.info(
        &["Save", "Game"],
        if k2 {
            "Star Wars: Knights of the Old Republic II"
        } else {
            "Star Wars: Knights of the Old Republic"
        },
    );

    // The module the game was saved in (from savenfo.res beside the save).
    let nfo = sibling(path, "savenfo.res").and_then(|p| std::fs::read(p).ok());
    let nfo_g = nfo.as_deref().and_then(|n| Gff::open(n, 0, n.len()));
    let last = nfo_g.and_then(|g| g.text(0, "LASTMODULE"));
    if let Some(area) = nfo_g.and_then(|g| g.text(0, "AREANAME")) {
        b.info(&["Save", "Area"], area);
    }
    let live = match last
        .as_deref()
        .and_then(|l| modules.iter().position(|m| m.0.eq_ignore_ascii_case(l)))
    {
        Some(i) => Some(i),
        // Without savenfo.res: the copy with the most experience (the newest).
        None => (0..modules.len()).max_by_key(|&i| {
            (
                modules[i].1.int("Experience").unwrap_or(0),
                std::cmp::Reverse(i),
            )
        }),
    };
    let pifo = sibling(path, "pifo.ifo").is_some();
    if let Some(i) = live {
        b.info(&["Save", "Module"], modules[i].0.clone());
    }
    if pifo {
        b.info(&["Save", "Player file"], "pifo.ifo");
    } else if let Some(main) = live.map(|i| modules[i].1).or(old_pc) {
        let copies: Vec<At> = modules
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != live)
            .map(|x| x.1 .1)
            .chain(old_pc.filter(|_| live.is_some()))
            .collect();
        b.info(&["Save", "Player copies"], copies.len() as u64 + 1);
        let name = main.text("FirstName").unwrap_or_default();
        b.info(&["Player", "Name"], name.trim());
        creature(&mut b, &["Player"], main, &copies);
    }

    let names: &[&str] = if k2 { &NPCS2 } else { &NPCS1 };
    let mut used: Vec<String> = vec![];
    for (n, c) in &npcs {
        let mut name = names
            .get(*n)
            .map(|s| s.to_string())
            .unwrap_or_else(|| name_of(c));
        if used.contains(&name) {
            name = format!("{name} ({n})");
        }
        used.push(name.clone());
        creature(&mut b, &["Party", &name], *c, &[]);
    }
    let mut doc_items = vec![];
    if let Some(inv) = top
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case("INVENTORY"))
        .and_then(|e| entry_gff(d, e))
    {
        let mut seen: Vec<(String, i64)> = vec![];
        for it in At::root(inv).list("ItemList") {
            let tag = it.text("Tag").unwrap_or_default();
            let n = it.int("StackSize").unwrap_or(1).max(1);
            match seen.iter_mut().find(|x| x.0 == tag) {
                Some(x) => x.1 += n,
                None => seen.push((tag, n)),
            }
        }
        doc_items = seen
            .into_iter()
            .map(|(t, n)| if n > 1 { format!("{t} \u{00D7}{n}") } else { t })
            .collect();
    }
    b.info(&["Inventory"], doc_items);
    let doc = b.finish(
        if k2 {
            "Star Wars: KOTOR II save"
        } else {
            "Star Wars: KOTOR save"
        },
        Checksum::None,
    );
    if live.is_none() && old_pc.is_none() && !pifo {
        return Err("no player found in this save".into());
    }
    Ok(doc.with_note(
        "SAVEGAME.sav: the player and the party members. Credits, party XP, influence, components and chemicals are in \
         PARTYTABLE.res in the same folder.",
    ))
}

fn parse_party(d: &[u8]) -> Result<TreeDoc, String> {
    let g = Gff::open(d, 0, d.len()).ok_or("broken party table")?;
    let r = At::root(g);
    let k2 = r.has("PT_ITEM_COMPONEN") || r.has("PT_INFLUENCE");
    let names: &[&str] = if k2 { &NPCS2 } else { &NPCS1 };
    let mut b = Builder::new(d);
    b.info(
        &["Save", "Game"],
        if k2 {
            "Star Wars: Knights of the Old Republic II"
        } else {
            "Star Wars: Knights of the Old Republic"
        },
    );
    if let Some(n) = r.text("PT_PCNAME").filter(|n| !n.is_empty()) {
        b.info(&["Save", "Player"], n);
    }
    if let Some(s) = r.int("PT_PLAYEDSECONDS") {
        b.info(&["Save", "Hours played"], (s as f64 / 360.0).round() / 10.0);
    }
    let max = i32::MAX as i64;
    r.edit(&mut b, &["Party", "Credits"], "PT_GOLD", 0, max, &[]);
    r.edit(&mut b, &["Party", "Party XP"], "PT_XP_POOL", 0, max, &[]);
    r.edit(
        &mut b,
        &["Party", "Components"],
        "PT_ITEM_COMPONEN",
        0,
        max,
        &[],
    );
    r.edit(
        &mut b,
        &["Party", "Chemicals"],
        "PT_ITEM_CHEMICAL",
        0,
        max,
        &[],
    );
    let npc = |i: i64| {
        usize::try_from(i)
            .ok()
            .and_then(|i| names.get(i))
            .map_or_else(|| format!("NPC {i}"), |s| s.to_string())
    };
    let members: Vec<String> = r
        .list("PT_MEMBERS")
        .iter()
        .filter_map(|m| m.int("PT_MEMBER_ID"))
        .map(npc)
        .collect();
    b.info(&["Party", "In the party"], members);
    let avail: Vec<String> = r
        .list("PT_AVAIL_NPCS")
        .iter()
        .enumerate()
        .filter(|(_, m)| m.int("PT_NPC_AVAIL").unwrap_or(0) != 0)
        .map(|(i, _)| npc(i as i64))
        .collect();
    b.info(&["Party", "Available"], avail);
    for (i, m) in r.list("PT_INFLUENCE").iter().enumerate() {
        let name = npc(i as i64);
        match m.int("PT_NPC_INFLUENCE") {
            Some(v) if v >= 0 => {
                m.edit(
                    &mut b,
                    &["Influence", &name],
                    "PT_NPC_INFLUENCE",
                    0,
                    100,
                    &[],
                );
            }
            Some(_) => b.info(&["Influence", &name], "not met"),
            None => {}
        }
    }
    if r.int("PT_GOLD").is_none() {
        return Err("no credits in this party table".into());
    }
    Ok(b.finish(if k2 { "Star Wars: KOTOR II party table" } else { "Star Wars: KOTOR party table" }, Checksum::None).with_note(
        "PARTYTABLE.res: credits, party XP and (KOTOR II) influence, components and chemicals. The player and the party \
         members' stats are in SAVEGAME.sav in the same folder.",
    ))
}

fn parse_player(d: &[u8]) -> Result<TreeDoc, String> {
    let g = Gff::open(d, 0, d.len()).ok_or("broken player file")?;
    let pc = player_of(g).ok_or("no player in this file")?;
    let mut b = Builder::new(d);
    let k2 = is_k2(&pc);
    b.info(
        &["Save", "Game"],
        if k2 {
            "Star Wars: Knights of the Old Republic II"
        } else {
            "Star Wars: Knights of the Old Republic"
        },
    );
    b.info(&["Save", "Player file"], "pifo.ifo");
    b.info(
        &["Player", "Name"],
        pc.text("FirstName").unwrap_or_default().trim(),
    );
    creature(&mut b, &["Player"], pc, &[]);
    Ok(b.finish(
        if k2 {
            "Star Wars: KOTOR II save"
        } else {
            "Star Wars: KOTOR save"
        },
        Checksum::None,
    )
    .with_note(
        "pifo.ifo: the player as this auto-save keeps it (the game loads the player from here).",
    ))
}

#[cfg(test)]
mod tests {
    use super::super::gff::build::{erf, gff, V};
    use super::*;
    use serde_json::Value;

    fn utc(tag: &'static str, first: &'static str, xp: u32, str_: u8, k2: bool) -> Vec<u8> {
        let mut f = vec![
            ("FirstName", V::Loc(-1, first)),
            ("Tag", V::Str(tag)),
            ("Str", V::Byte(str_)),
            ("Dex", V::Byte(14)),
            ("Con", V::Byte(12)),
            ("Int", V::Byte(10)),
            ("Wis", V::Byte(16)),
            ("Cha", V::Byte(8)),
            ("HitPoints", V::Short(50)),
            ("CurrentHitPoints", V::Short(40)),
            ("MaxHitPoints", V::Short(60)),
            ("ForcePoints", V::Short(20)),
            ("CurrentForce", V::Short(18)),
            ("MaxForcePoints", V::Short(30)),
            ("Experience", V::Dword(xp)),
            ("GoodEvil", V::Byte(70)),
            ("SkillPoints", V::Word(2)),
            (
                "ClassList",
                V::List(vec![
                    (2, vec![("Class", V::Int(1)), ("ClassLevel", V::Short(8))]),
                    (2, vec![("Class", V::Int(4)), ("ClassLevel", V::Short(3))]),
                ]),
            ),
            (
                "SkillList",
                V::List(
                    (0..8)
                        .map(|i| (0, vec![("Rank", V::Byte(i as u8))]))
                        .collect(),
                ),
            ),
        ];
        if k2 {
            f.push(("ItemComponent", V::Dword(0)));
        }
        gff("UTC ", f)
    }

    fn ifo(pc: Vec<(&'static str, V)>) -> Vec<u8> {
        gff(
            "IFO ",
            vec![
                ("Mod_Name", V::Str("m")),
                ("Mod_PlayerList", V::List(vec![(0, pc)])),
            ],
        )
    }

    /// The fields of `utc` as a list (to put a player into Module.ifo).
    fn pc_fields(xp: u32, str_: u8) -> Vec<(&'static str, V)> {
        vec![
            ("FirstName", V::Loc(-1, "Revan")),
            ("Str", V::Byte(str_)),
            ("ForcePoints", V::Short(20)),
            ("Experience", V::Dword(xp)),
            (
                "SkillList",
                V::List(
                    (0..8)
                        .map(|i| (0, vec![("Rank", V::Byte(i as u8))]))
                        .collect(),
                ),
            ),
        ]
    }

    fn save(k2: bool) -> Vec<u8> {
        erf(&[
            ("AVAILNPC0", T_UTC, utc("bastila", "", 1000, 12, k2)),
            ("AVAILNPC2", T_UTC, utc("carth", "Carth", 900, 16, k2)),
            (
                "INVENTORY",
                0,
                gff(
                    "INV ",
                    vec![(
                        "ItemList",
                        V::List(vec![(
                            0,
                            vec![
                                ("Tag", V::Str("g_i_medeqpmnt01")),
                                ("StackSize", V::Word(5)),
                            ],
                        )]),
                    )],
                ),
            ),
            ("PC", T_UTC, utc("", "Revan", 100, 10, k2)),
            (
                "old_mod",
                T_SAV,
                erf(&[("Module", T_IFO, ifo(pc_fields(500, 11)))]),
            ),
            (
                "new_mod",
                T_SAV,
                erf(&[("Module", T_IFO, ifo(pc_fields(800, 13)))]),
            ),
        ])
    }

    #[test]
    fn reads_and_edits_the_player_everywhere() {
        let d = save(false);
        assert!(looks(&d));
        let mut doc = parse(&d, None).unwrap();
        let v = &doc.view;
        assert_eq!(
            v.pointer("/Save/Game"),
            Some(&Value::from("Star Wars: Knights of the Old Republic"))
        );
        // Without savenfo.res, the newest copy (most experience) is shown.
        assert_eq!(v.pointer("/Save/Module"), Some(&Value::from("new_mod")));
        assert_eq!(v.pointer("/Player/Experience"), Some(&Value::from(800)));
        assert_eq!(
            v.pointer("/Player/Attributes/Strength"),
            Some(&Value::from(13))
        );
        assert_eq!(
            v.pointer("/Player/Skills/Treat Injury"),
            Some(&Value::from(7))
        );
        assert_eq!(
            v.pointer("/Party/Bastila/Attributes/Strength"),
            Some(&Value::from(12))
        );
        assert_eq!(
            v.pointer("/Party/Carth/Classes"),
            Some(&Value::from("Scout 8 / Jedi Consular 3"))
        );
        assert_eq!(v.pointer("/Party/Carth/Level"), Some(&Value::from(11)));
        assert_eq!(
            v.pointer("/Party/Carth/Health/Max FP"),
            Some(&Value::from(30))
        );
        assert_eq!(
            v.pointer("/Inventory/0"),
            Some(&Value::from("g_i_medeqpmnt01 \u{00D7}5"))
        );
        doc.set("/Player/Experience", Value::from(5000)).unwrap();
        doc.set("/Player/Attributes/Strength", Value::from(18))
            .unwrap();
        doc.set("/Player/Skills/Repair", Value::from(9)).unwrap();
        doc.set("/Party/Carth/Attributes/Dexterity", Value::from(20))
            .unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out, None).unwrap();
        assert_eq!(
            again.view.pointer("/Player/Experience"),
            Some(&Value::from(5000))
        );
        assert_eq!(
            again.view.pointer("/Party/Carth/Attributes/Dexterity"),
            Some(&Value::from(20))
        );
        assert_eq!(
            again.view.pointer("/Party/Bastila/Attributes/Dexterity"),
            Some(&Value::from(14))
        );
        // Every copy of the player got the edit.
        let top = erf_entries(&out, 0, out.len()).unwrap();
        for e in top.iter().filter(|e| e.ty == T_SAV) {
            let inner = erf_entries(&out, e.at, e.at + e.size).unwrap();
            let pc = player_of(entry_gff(&out, &inner[0]).unwrap()).unwrap();
            assert_eq!(
                (pc.int("Experience"), pc.int("Str")),
                (Some(5000), Some(18))
            );
            assert_eq!(pc.list("SkillList")[5].int("Rank"), Some(9));
        }
        let old = At::root(entry_gff(&out, top.iter().find(|e| e.name == "PC").unwrap()).unwrap());
        assert_eq!(old.int("Experience"), Some(5000));
        // Untouched values are unchanged.
        assert_eq!(
            again.view.pointer("/Player/Skills/Stealth"),
            Some(&Value::from(2))
        );
    }

    #[test]
    fn kotor2_and_the_party_table() {
        let d = save(true);
        let doc = parse(&d, None).unwrap();
        assert_eq!(doc.kind_name, "Star Wars: KOTOR II save");
        // KOTOR II numbers its NPCs differently: 0 is Atton, 2 Mandalore.
        assert!(doc.view.pointer("/Party/Mandalore").is_some());
        let pt = gff(
            "PT  ",
            vec![
                ("PT_PCNAME", V::Str("Exile")),
                ("PT_GOLD", V::Dword(28079)),
                ("PT_ITEM_COMPONEN", V::Dword(346)),
                ("PT_ITEM_CHEMICAL", V::Dword(514)),
                ("PT_XP_POOL", V::Int(416319)),
                ("PT_PLAYEDSECONDS", V::Dword(7200)),
                (
                    "PT_MEMBERS",
                    V::List(vec![(
                        0,
                        vec![("PT_MEMBER_ID", V::Int(9)), ("PT_IS_LEADER", V::Byte(0))],
                    )]),
                ),
                (
                    "PT_INFLUENCE",
                    V::List(vec![
                        (0, vec![("PT_NPC_INFLUENCE", V::Int(92))]),
                        (0, vec![("PT_NPC_INFLUENCE", V::Int(-1))]),
                    ]),
                ),
            ],
        );
        assert!(looks(&pt));
        let mut p = parse(&pt, None).unwrap();
        assert_eq!(p.kind_name, "Star Wars: KOTOR II party table");
        assert_eq!(p.view.pointer("/Party/Credits"), Some(&Value::from(28079)));
        assert_eq!(p.view.pointer("/Party/Components"), Some(&Value::from(346)));
        assert_eq!(
            p.view.pointer("/Party/In the party/0"),
            Some(&Value::from("Visas"))
        );
        assert_eq!(p.view.pointer("/Influence/Atton"), Some(&Value::from(92)));
        assert_eq!(
            p.view.pointer("/Influence/Bao-Dur"),
            Some(&Value::from("not met"))
        );
        assert!(p.set("/Influence/Atton", Value::from(101)).is_err());
        p.set("/Influence/Atton", Value::from(100)).unwrap();
        p.set("/Party/Credits", Value::from(28080)).unwrap();
        let out = p.to_bytes().unwrap();
        let q = parse(&out, None).unwrap();
        assert_eq!(q.view.pointer("/Party/Credits"), Some(&Value::from(28080)));
        assert_eq!(q.view.pointer("/Influence/Atton"), Some(&Value::from(100)));
        assert_eq!(q.view.pointer("/Party/Chemicals"), Some(&Value::from(514)));
        // pifo.ifo: the player alone.
        let pifo = ifo(pc_fields(42, 9));
        let pl = parse(&pifo, None).unwrap();
        assert_eq!(
            pl.view.pointer("/Player/Experience"),
            Some(&Value::from(42))
        );
        assert!(pl.view.pointer("/Party").is_none());
    }

    /// Research aid: SAVE_EDITOR_KOTOR = save folders separated by `|`. Each file parses,
    /// writes back unchanged, and an edit re-reads with the rest unchanged.
    #[test]
    #[ignore]
    fn real_saves() {
        for dir in std::env::var("SAVE_EDITOR_KOTOR").unwrap().split('|') {
            for name in ["SAVEGAME.sav", "PARTYTABLE.res", "pifo.ifo"] {
                let Some(p) = sibling(Some(&Path::new(dir).join("x")), name) else {
                    continue;
                };
                let d = std::fs::read(&p).unwrap();
                let mut doc =
                    parse(&d, Some(&p)).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                assert_eq!(doc.to_bytes().unwrap(), d, "{}", p.display());
                let before = doc.view.clone();
                let (ptr, key) = if name == "PARTYTABLE.res" {
                    ("/Party/Credits", "")
                } else {
                    ("/Player/Experience", "/Player/Attributes/Strength")
                };
                println!(
                    "{} {}: {} = {:?}",
                    p.display(),
                    doc.kind_name,
                    ptr,
                    before.pointer(ptr)
                );
                if before.pointer(ptr).is_none() {
                    println!("{}", serde_json::to_string_pretty(&before["Save"]).unwrap());
                    continue;
                }
                let n = before.pointer(ptr).and_then(Value::as_i64).unwrap();
                doc.set(ptr, Value::from(n + 1)).unwrap();
                if !key.is_empty() {
                    doc.set(key, Value::from(18)).unwrap();
                }
                let out = doc.to_bytes().unwrap();
                assert_eq!(out.len(), d.len());
                let again = parse(&out, Some(&p)).unwrap();
                let mut want = before.clone();
                *want.pointer_mut(ptr).unwrap() = Value::from(n + 1);
                if !key.is_empty() {
                    *want.pointer_mut(key).unwrap() = Value::from(18);
                }
                assert_eq!(again.view, want, "{}", p.display());
                let changed = d.iter().zip(&out).filter(|(a, b)| a != b).count();
                println!(
                    "  edit ok, {changed} bytes changed; {}",
                    serde_json::to_string(&again.view["Party"])
                        .unwrap()
                        .chars()
                        .take(300)
                        .collect::<String>()
                );
            }
        }
    }
}
