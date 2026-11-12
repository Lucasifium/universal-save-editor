//! Kenshi `.save`: lists of named values (u32 name length, name, 4-byte value). The
//! player's money is the one value named "player money".

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const MONEY: &[u8] = b"\x0c\x00\x00\x00player money";

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn looks(d: &[u8]) -> bool {
    // A Kenshi save names itself ("…-quick.save-INGAME") near the start; saves whose ids
    // don't carry the name are told by their header (file type 15) and the money record.
    d.len() > 4096 && (find(&d[..256], b".save-").is_some() || (d[..4] == [15, 0, 0, 0] && find(d, MONEY).is_some()))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Kenshi save".into());
    }
    let at = find(d, MONEY).ok_or("no player money in this save")? + MONEY.len();
    let mut b = Builder::new(d);
    b.ranged(&["Player", "Money (cats)"], at, Kind::I32, 0, i32::MAX as i64);
    Ok(b.finish("Kenshi save", Checksum::None).with_note("The player faction's cats. Each squad's characters (stats, health) are in the save's platoon folder: open the .platoon file named after your faction (Nameless_0.platoon unless you renamed it)."))
}

// ------------------------------------------------------------------ squad (.platoon) files

/// The player's own faction in the game's data.
const PLAYER_FACTION: &str = "204-gamedata.base";

const ATTRIBUTES: &[(&str, &str)] = &[("strength", "Strength"), ("toughness2", "Toughness"), ("dexterity", "Dexterity"), ("perception", "Perception")];

/// The skills the game's character screen shows, by their key in the stats record.
pub const SKILLS: &[(&str, &str)] = &[
    ("attack", "Melee Attack"),
    ("defence", "Melee Defence"),
    ("dodge", "Dodge"),
    ("unarmed", "Martial Arts"),
    ("katana", "Katanas"),
    ("sabres", "Sabres"),
    ("hackers", "Hackers"),
    ("heavy weapons", "Heavy Weapons"),
    ("blunt", "Blunt"),
    ("poles", "Polearms"),
    ("bow", "Crossbows"),
    ("turrets", "Turrets"),
    ("ff", "Precision Shooting"),
    ("stealth", "Stealth"),
    ("lockpicking", "Lockpicking"),
    ("thievery", "Thievery"),
    ("assassin", "Assassination"),
    ("athletics", "Athletics"),
    ("swimming", "Swimming"),
    ("medic", "Field Medic"),
    ("robotics", "Robotics"),
    ("engineer", "Engineer"),
    ("science", "Science"),
    ("weapon smith", "Weapon Smith"),
    ("armour smith", "Armour Smith"),
    ("bow smith", "Crossbow Smith"),
    ("labouring", "Labouring"),
    ("farming", "Farming"),
    ("cooking", "Cooking"),
];

/// Body parts in the order the medical record keeps them (the same slots for every race;
/// an animal's forelegs use the arm slots).
pub const PARTS: &[&str] = &["Head", "Chest", "Stomach", "Left arm", "Right arm", "Left leg", "Right leg"];

/// One record of a Kenshi data file: where each named value of it lives.
#[derive(Default)]
struct Rec {
    kind: i32,
    name: String,
    sid: String,
    bools: Vec<(String, usize)>,
    floats: Vec<(String, usize)>,
    ints: Vec<(String, usize)>,
    strings: Vec<(String, String)>,
    /// (instance id, ids of the records that belong to it)
    instances: Vec<(String, Vec<String>)>,
}

impl Rec {
    fn float(&self, k: &str) -> Option<usize> {
        self.floats.iter().find(|(n, _)| n == k).map(|(_, a)| *a)
    }
    fn boolean(&self, k: &str) -> Option<usize> {
        self.bools.iter().find(|(n, _)| n == k).map(|(_, a)| *a)
    }
    fn int(&self, k: &str) -> Option<usize> {
        self.ints.iter().find(|(n, _)| n == k).map(|(_, a)| *a)
    }
    fn string(&self, k: &str) -> Option<&str> {
        self.strings.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }
}

struct Rd<'a> {
    d: &'a [u8],
    p: usize,
}

impl Rd<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.d.get(self.p..self.p.checked_add(n)?)?;
        self.p += n;
        Some(s)
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn count(&mut self) -> Option<usize> {
        let n = self.i32()?;
        // Every entry takes at least four bytes.
        (n >= 0 && (n as usize) <= (self.d.len() - self.p) / 4 + 1).then_some(n as usize)
    }
    fn str(&mut self) -> Option<String> {
        let n = self.i32()?;
        if n <= 0 {
            return (n == 0).then(String::new);
        }
        Some(String::from_utf8_lossy(self.take(n as usize)?).into_owned())
    }
    /// A section of (name, fixed-size value) pairs: the name and where its value starts.
    fn section(&mut self, size: usize) -> Option<Vec<(String, usize)>> {
        let n = self.count()?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let k = self.str()?;
            out.push((k, self.p));
            self.take(size)?;
        }
        Some(out)
    }
}

fn record(r: &mut Rd) -> Option<Rec> {
    let mut rec = Rec::default();
    r.i32()?; // instance count (not always the real one)
    rec.kind = r.i32()?;
    r.i32()?; // record id
    rec.name = r.str()?;
    rec.sid = r.str()?;
    r.i32()?; // flags
    rec.bools = r.section(1)?;
    rec.floats = r.section(4)?;
    rec.ints = r.section(4)?;
    r.section(12)?; // 3D vectors
    r.section(16)?; // 4D vectors
    for _ in 0..2 {
        // text values, then file names
        for _ in 0..r.count()? {
            let k = r.str()?;
            let v = r.str()?;
            rec.strings.push((k, v));
        }
    }
    for _ in 0..r.count()? {
        r.str()?; // category
        for _ in 0..r.count()? {
            r.str()?;
            r.take(12)?;
        }
    }
    for _ in 0..r.count()? {
        let id = r.str()?;
        r.str()?; // template
        r.take(28)?; // position, rotation
        let mut states = vec![];
        for _ in 0..r.count()? {
            states.push(r.str()?);
        }
        rec.instances.push((id, states));
    }
    Some(rec)
}

/// Every record of a squad file, if `d` is one (read to the last byte).
fn platoon(d: &[u8]) -> Option<Vec<Rec>> {
    let mut r = Rd { d, p: 0 };
    if r.i32()? != 15 {
        return None;
    }
    r.i32()?; // next free id
    let n = r.count()?;
    let mut recs = Vec::with_capacity(n);
    for _ in 0..n {
        recs.push(record(&mut r)?);
    }
    (r.p == d.len() && recs.iter().any(|x| x.kind == SQUAD)).then_some(recs)
}

const STATS: i32 = 25;
const SQUAD: i32 = 30;
const CHARACTER: i32 = 36;
const MEDICAL: i32 = 57;

pub fn looks_platoon(d: &[u8]) -> bool {
    // filetype 15, then a small id and record count, then the first record's header.
    d.len() > 40 && d[..4] == [15, 0, 0, 0] && platoon(d).is_some()
}

pub fn parse_platoon(d: &[u8]) -> Result<TreeDoc, String> {
    let recs = platoon(d).ok_or("not a Kenshi squad file")?;
    let by_sid = |s: &str| recs.iter().find(|r| r.sid == s);
    let mut b = Builder::new(d);
    let mut names: Vec<String> = vec![];
    let mut players = 0;
    for squad in recs.iter().filter(|r| r.kind == SQUAD) {
        for (id, states) in &squad.instances {
            let own: Vec<&Rec> = states.iter().filter_map(|s| by_sid(s)).collect();
            let pick = |k: i32| own.iter().copied().find(|r| r.kind == k);
            let (Some(stats), ch) = (pick(STATS), pick(CHARACTER)) else { continue };
            let mut name = ch.and_then(|c| c.string("name")).filter(|n| !n.trim().is_empty()).unwrap_or(&stats.name).trim().to_string();
            if name.is_empty() || name == "0" {
                name = format!("Member {}", names.len() + 1);
            }
            let base = name.clone();
            let mut n = 2;
            while names.contains(&name) {
                name = format!("{base} ({n})");
                n += 1;
            }
            names.push(name.clone());
            let m = name.as_str();
            let player = ch.and_then(|c| c.string("owner faction ID")) == Some(PLAYER_FACTION);
            players += usize::from(player);
            b.info(&["Members", m, "Id"], id.as_str());
            b.info(&["Members", m, "Player's"], player);
            if ch.is_some_and(|c| c.boolean("is leader").is_some_and(|a| d[a] != 0)) {
                b.info(&["Members", m, "Squad leader"], true);
            }
            for (k, label) in ATTRIBUTES {
                if let Some(at) = stats.float(k) {
                    b.ranged(&["Members", m, "Attributes", label], at, Kind::F32, 0, 100);
                }
            }
            // Skills a character never trained sit a little below zero in the game's own saves.
            for (k, label) in SKILLS {
                if let Some(at) = stats.float(k) {
                    b.ranged(&["Members", m, "Skills", label], at, Kind::F32, -100, 100);
                }
            }
            let Some(med) = pick(MEDICAL) else { continue };
            for (i, part) in PARTS.iter().enumerate() {
                if let Some(at) = med.float(&format!("flesh{i}")) {
                    // A lost limb reads far below zero.
                    b.ranged(&["Members", m, "Health", part], at, Kind::F32, -1000, 1000);
                }
            }
            for (k, label, lo, hi) in [("blood", "Blood", 0, 1000), ("bleeding", "Bleeding", 0, 1000), ("hung", "Hunger", 0, 1000), ("KO", "Knocked out (time)", 0, 1000)] {
                if let Some(at) = med.float(k) {
                    b.ranged(&["Members", m, "Condition", label], at, Kind::F32, lo, hi);
                }
            }
            // "unconcious" is the game's own spelling.
            for (k, label) in [("dead", "Dead"), ("unconcious", "Unconscious"), ("coma", "Coma"), ("incapacitated", "Incapacitated")] {
                if let Some(at) = med.boolean(k) {
                    b.field(&["Members", m, "Status", label], at, Kind::B8);
                }
            }
            // Two bits per limb: own, lost or robotic. Only kept once a limb isn't the character's own.
            if let Some(at) = med.int("limbs") {
                let opts: Vec<(String, u64)> = ["Own", "Lost", "Robotic"].iter().enumerate().map(|(i, s)| (s.to_string(), i as u64)).collect();
                for (k, part) in PARTS[3..].iter().enumerate() {
                    b.pick_bits(&["Members", m, "Limbs", part], at, Kind::I32, k as u32 * 2, 2, &opts);
                }
            }
        }
    }
    if names.is_empty() {
        return Err("no characters in this squad file".into());
    }
    let faction = recs.iter().filter(|r| r.kind == CHARACTER).find_map(|r| r.string("owner faction ID")).unwrap_or_default();
    b.info(&["Squad", "Members"], names.len() as i64);
    b.info(&["Squad", "Player's squad"], players > 0 || faction == PLAYER_FACTION);
    Ok(b.finish("Kenshi squad", Checksum::None).with_note("One squad of a Kenshi save (a file in the save's platoon folder). The player's squads are the files named after your faction, \"Nameless\" unless you renamed it."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::TreeDoc;
    use serde_json::json;

    fn s(o: &mut Vec<u8>, t: &str) {
        o.extend((t.len() as i32).to_le_bytes());
        o.extend(t.as_bytes());
    }
    fn n(o: &mut Vec<u8>, v: i32) {
        o.extend(v.to_le_bytes());
    }

    /// One record: bools, floats, ints, strings and instances (id, states).
    fn rec(o: &mut Vec<u8>, kind: i32, sid: &str, bools: &[(&str, u8)], floats: &[(&str, f32)], ints: &[(&str, i32)], strs: &[(&str, &str)], inst: &[(&str, &[&str])]) {
        n(o, 0);
        n(o, kind);
        n(o, 1);
        s(o, "0");
        s(o, sid);
        n(o, 0);
        n(o, bools.len() as i32);
        for (k, v) in bools {
            s(o, k);
            o.push(*v);
        }
        n(o, floats.len() as i32);
        for (k, v) in floats {
            s(o, k);
            o.extend(v.to_le_bytes());
        }
        n(o, ints.len() as i32);
        for (k, v) in ints {
            s(o, k);
            n(o, *v);
        }
        n(o, 0);
        n(o, 0);
        n(o, strs.len() as i32);
        for (k, v) in strs {
            s(o, k);
            s(o, v);
        }
        n(o, 0);
        n(o, 0);
        n(o, inst.len() as i32);
        for (id, states) in inst {
            s(o, id);
            s(o, "");
            o.extend([0u8; 28]);
            n(o, states.len() as i32);
            for st in *states {
                s(o, st);
            }
        }
    }

    fn sample() -> Vec<u8> {
        let mut o = vec![];
        n(&mut o, 15);
        n(&mut o, 9);
        n(&mut o, 4);
        rec(&mut o, CHARACTER, "1-x", &[("is leader", 1)], &[], &[], &[("name", "Beep"), ("owner faction ID", PLAYER_FACTION)], &[]);
        rec(&mut o, MEDICAL, "2-x", &[("dead", 0), ("unconcious", 1)], &[("flesh0", 100.0), ("flesh3", -80.0), ("bleeding", 2.0)], &[("limbs", 1)], &[], &[]);
        rec(&mut o, STATS, "3-x", &[], &[("strength", 5.0), ("ff", 1.0), ("thievery", -3.4)], &[], &[], &[]);
        rec(&mut o, SQUAD, "4-x", &[], &[], &[("char count", 1)], &[], &[("5-x", &["1-x", "2-x", "3-x"])]);
        o
    }

    fn doc(d: &[u8]) -> TreeDoc {
        assert!(looks_platoon(d));
        parse_platoon(d).unwrap()
    }

    #[test]
    fn reads_a_squad() {
        let d = sample();
        let t = doc(&d);
        let m = &t.view["Members"]["Beep"];
        assert_eq!(m["Player's"], json!(true));
        assert_eq!(m["Squad leader"], json!(true));
        assert_eq!(m["Attributes"]["Strength"].as_f64(), Some(5.0));
        assert_eq!(m["Skills"]["Precision Shooting"].as_f64(), Some(1.0));
        assert!((m["Skills"]["Thievery"].as_f64().unwrap() + 3.4).abs() < 1e-4);
        assert_eq!(m["Health"]["Left arm"].as_f64(), Some(-80.0));
        assert_eq!(m["Status"]["Unconscious"], json!(true));
        assert_eq!(m["Limbs"]["Left arm"], json!("Lost"));
        assert_eq!(m["Limbs"]["Right leg"], json!("Own"));
        assert_eq!(t.view["Squad"]["Player's squad"], json!(true));
    }

    #[test]
    fn edits_in_place() {
        let d = sample();
        let mut t = doc(&d);
        t.set("/Members/Beep/Attributes/Strength", json!(100.0)).unwrap();
        t.set("/Members/Beep/Limbs/Left arm", json!("Own")).unwrap();
        t.set("/Members/Beep/Status/Unconscious", json!(false)).unwrap();
        assert!(t.set("/Members/Beep/Skills/Thievery", json!(101.0)).is_err());
        let out = t.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = doc(&out);
        let m = &again.view["Members"]["Beep"];
        assert_eq!(m["Attributes"]["Strength"].as_f64(), Some(100.0));
        assert_eq!(m["Limbs"]["Left arm"], json!("Own"));
        assert_eq!(m["Status"]["Unconscious"], json!(false));
    }

    #[test]
    fn other_files_are_not_squads() {
        let mut d = sample();
        d.push(0);
        assert!(!looks_platoon(&d));
        assert!(!looks_platoon(&[15, 0, 0, 0, 1, 0, 0, 0, 255, 255, 255, 127]));
    }
}
