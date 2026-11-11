//! Dragon Age: Origins (and Awakening) `.das` saves: one GFF V4.0 file (type `SAVE`,
//! version `V1.1`; Dragon Age II writes `V2.0` and numbers its stats differently, so it's
//! left alone). See `gff4.rs` for the container.
//!
//! Where things are (field ids): root 16000 campaign (16014 campaign resource, 17101 area),
//! 16002 player → 16208 the player creature, 16003 party: 16212 money in copper, 16204
//! followers, 16210 backpack (items: 3 resource name, 1002 stack size), 16275 approval
//! (16276 follower object id, 16277 approval). A creature: 23 object id, 3 resource name,
//! 16222 tag, 16255 name, 16219 equipped items, 16209 stats → 16350 list of stat
//! properties (16353 stat id, 16300 base value as a float). Stat ids as in the MIT
//! dragon-age-save-editor by Aleksei Tovkachev: 1–6 the attributes, 15 level, 19
//! experience, 34 / 35 / 36 / 38 attribute / skill / talent / specialization points.
//! Values are edited in place; the file has no checksum.

use super::fields::{Builder, Checksum, Kind};
use super::gff4::{Gff4, St};
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "Dragon Age: Origins save";

pub fn looks(d: &[u8]) -> bool {
    d.len() > 0x1C && d.starts_with(b"GFF V4.0PC  SAVEV1.1")
}

/// "gen_im_qck_health_101" → "Qck health 101"; "prm000im_wolf_charm" → "Wolf charm".
pub fn pretty_resref(res: &str) -> String {
    let s = res.trim_end_matches('\0');
    let s = match s.find("im_") {
        Some(i) if i <= 10 && s.len() > i + 3 => &s[i + 3..],
        _ => s,
    };
    let mut out: String = s.replace('_', " ").trim().to_string();
    if let Some(f) = out.get(0..1) {
        out = f.to_uppercase() + &out[1..];
    }
    out
}

/// A follower's name from its tag or resource name ("gen00fl_wynne" → "Wynne").
fn follower_name(tag: &str, res: &str) -> String {
    let pick = [tag, res].into_iter().find(|s| s.to_lowercase().starts_with("gen00fl_")).unwrap_or(if res.is_empty() { tag } else { res });
    let lower = pick.to_lowercase();
    let base = lower.strip_prefix("gen00fl_").unwrap_or(&lower);
    pretty_resref(base)
}

const STATS: [(u32, &str, i64); 12] = [
    (1, "Strength", 100),
    (2, "Dexterity", 100),
    (3, "Willpower", 100),
    (4, "Magic", 100),
    (5, "Cunning", 100),
    (6, "Constitution", 100),
    (15, "Level", 50),
    (19, "Experience", 10_000_000),
    (34, "Attribute points", 999),
    (35, "Skill points", 999),
    (36, "Talent points", 999),
    (38, "Specialization points", 99),
];

/// A creature's level, experience, attributes and point pools.
fn creature(g: &Gff4, b: &mut Builder, base: &[&str], c: St) {
    let path = |k: &str| -> Vec<String> { base.iter().map(|s| s.to_string()).chain(std::iter::once(k.to_string())).collect() };
    let put_info = |b: &mut Builder, k: &str, v: serde_json::Value| {
        let p = path(k);
        let p: Vec<&str> = p.iter().map(String::as_str).collect();
        b.info(&p, v);
    };
    let Some(stats) = g.child(c, 16209) else { return };
    let mut by_id: HashMap<u32, St> = HashMap::new();
    for p in g.list(stats, 16350) {
        if let Some(id) = g.value(p, 16353) {
            by_id.entry(id as u32).or_insert(p);
        }
    }
    for (id, label, hi) in STATS {
        let Some((at, kind)) = by_id.get(&id).and_then(|&p| g.num(p, 16300)) else { continue };
        let p = path(label);
        let p: Vec<&str> = p.iter().map(String::as_str).collect();
        let lo = if id == 15 { 1 } else { 0 };
        // Stored as floats; whole numbers are edited as whole numbers.
        let whole = kind == Kind::F32 && g.value(by_id[&id], 16300).is_some_and(|v| v.fract() == 0.0 && v.abs() < 16_777_216.0);
        b.ranged(&p, at, if whole { Kind::F32Int } else { kind }, lo, hi);
    }
    for (id, label) in [(7, "Health"), (8, "Mana or stamina")] {
        if let Some(p) = by_id.get(&id) {
            if let Some(v) = g.value(*p, 16302).or_else(|| g.value(*p, 16300)) {
                put_info(b, label, (v.round() as i64).into());
            }
        }
    }
    put_info(b, "Talents and spells known", (g.list_len(stats, 16306) + g.list_len(stats, 16305)).into());
    put_info(b, "Skills known", g.list_len(stats, 16307).into());
    let worn: Vec<String> = g.list(c, 16219).into_iter().filter_map(|i| g.string(i, 3)).map(|r| pretty_resref(&r)).collect();
    if !worn.is_empty() {
        put_info(b, "Equipped", worn.join(", ").into());
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Dragon Age: Origins save".into());
    }
    let g = Gff4::open(d, b"SAVE")?;
    let root = g.root();
    let party = g.child(root, 16003).filter(|&p| g.label(p) == b"PRTY").ok_or("no party in this save")?;
    let player = g.child(root, 16002).and_then(|p| g.child(p, 16208)).ok_or("no player in this save")?;
    let mut b = Builder::new(d);

    if let Some(c) = g.child(root, 16000) {
        if let Some(area) = g.string(c, 17101) {
            b.info(&["Party", "Area"], area);
        }
        if let Some(camp) = g.string(c, 16014) {
            // Awakening's campaign ids, as the MIT editor reads them.
            let awakening = matches!(camp.to_uppercase().as_str(), "DAO_PRC_EP_1" | "DAO_PRC_STR" | "DAO_PRC_GIB");
            b.info(&["Party", "Campaign"], if awakening { format!("Awakening ({camp})") } else { camp });
        }
    }
    let (at, kind) = g.num(party, 16212).ok_or("no party money")?;
    b.ranged(&["Party", "Money (copper)"], at, kind, 0, i32::MAX as i64);
    if let Some(n) = g.value(party, 16226) {
        b.info(&["Party", "Backpack size"], n as i64);
    }

    let name = g.tlk(player, 16255).and_then(|t| t.1).filter(|s| !s.trim().is_empty()).unwrap_or_else(|| "Warden".into());
    b.info(&["Player", "Name"], name);
    creature(&g, &mut b, &["Player"], player);

    // Approval by follower object id.
    let mut approval: Vec<(u32, St)> = g.list(party, 16275).into_iter().filter_map(|a| Some((g.value(a, 16276)? as u32, a))).collect();
    let mut used: Vec<String> = vec![];
    for f in g.list(party, 16204) {
        let tag = g.string(f, 16222).unwrap_or_default();
        let res = g.string(f, 3).unwrap_or_default();
        let base = follower_name(&tag, &res);
        let mut key = base.clone();
        let mut k = 2;
        while used.contains(&key) {
            key = format!("{base} ({k})");
            k += 1;
        }
        used.push(key.clone());
        if let Some(n) = g.tlk(f, 16255).and_then(|t| t.1).filter(|s| !s.trim().is_empty()) {
            b.info(&["Followers", &key, "Name"], n);
        }
        if let Some(id) = g.value(f, 23).map(|v| v as u32) {
            if let Some(i) = approval.iter().position(|a| a.0 == id) {
                let (_, a) = approval.remove(i);
                if let Some((at, kind)) = g.num(a, 16277) {
                    b.ranged(&["Followers", &key, "Approval"], at, kind, -100, 100);
                }
            }
        }
        creature(&g, &mut b, &["Followers", &key], f);
    }
    // Approval of followers who aren't in the party pool (left, or not met yet).
    for (id, a) in approval {
        if let Some((at, kind)) = g.num(a, 16277) {
            b.ranged(&["Other approval", &format!("Follower #{id}")], at, kind, -100, 100);
        }
    }

    let items = g.list(party, 16210);
    let width = items.len().to_string().len().max(2);
    for (i, it) in items.iter().enumerate() {
        let res = g.string(*it, 3).unwrap_or_default();
        let key = format!("{:0width$} {}", i + 1, pretty_resref(&res));
        if let Some((at, kind)) = g.num(*it, 1002) {
            if super::fields::read(d, at, kind, false).and_then(|v| v.as_i64()).is_some_and(|n| n >= 1) {
                b.ranged(&["Backpack", &key], at, kind, 1, 999);
            } else {
                b.info(&["Backpack", &key], super::fields::read(d, at, kind, false).unwrap_or_default());
            }
        }
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::super::gff4::tests::build;
    use super::*;

    fn w32(d: &mut Vec<u8>, v: u32) {
        d.extend(v.to_le_bytes());
    }

    fn put(d: &mut [u8], at: usize, v: &[u8]) {
        d[at..at + v.len()].copy_from_slice(v);
    }

    fn string(d: &mut Vec<u8>, s: &str) -> u32 {
        let at = d.len() as u32;
        w32(d, s.encode_utf16().count() as u32 + 1);
        for c in s.encode_utf16().chain(std::iter::once(0)) {
            d.extend(c.to_le_bytes());
        }
        at
    }

    /// A synthetic DAO save: a player and one follower (stats: Strength, level, experience,
    /// attribute points), money, one approval entry and a backpack of one potion stack.
    pub fn sample() -> Vec<u8> {
        // Templates: 0 root, 1 PLYR, 2 PRTY, 3 creature, 4 stats, 5 ATTR, 6 SMPL, 7 item, 8 APRV.
        let templates: Vec<(&[u8; 4], u32, Vec<(u32, u16, u16, u32)>)> = vec![
            (b"ROOT", 20, vec![(16002, 1, 0x4000, 0), (16003, 2, 0x4000, 4)]),
            (b"PLYR", 4, vec![(16208, 3, 0x6000, 0)]),
            (b"PRTY", 16, vec![(16212, 5, 0, 0), (16204, 3, 0xC000, 4), (16210, 7, 0xC000, 8), (16275, 8, 0xC000, 12)]),
            (b"CRE ", 28, vec![(3, 14, 0, 0), (23, 5, 0, 4), (16222, 14, 0, 8), (16209, 4, 0x6000, 12), (16255, 17, 0, 16), (16219, 7, 0xC000, 24)]),
            (b"CPS1", 4, vec![(16350, 0xFFFF, 0xA000, 0)]),
            (b"ATTR", 12, vec![(16300, 8, 0, 0), (16301, 8, 0, 4), (16353, 2, 0, 8)]),
            (b"SMPL", 8, vec![(16300, 8, 0, 0), (16353, 2, 0, 4)]),
            (b"ITL1", 8, vec![(3, 14, 0, 0), (1002, 5, 0, 4)]),
            (b"APRV", 8, vec![(16276, 4, 0, 0), (16277, 5, 0, 4)]),
        ];
        // The root: PLYR inline at 0 (a reference to the player), PRTY inline at 4..20.
        let mut d = vec![0u8; 20];
        put(&mut d, 4, &1234567i32.to_le_bytes());
        // Creatures: player (referenced) and the follower (inline list).
        let mk_creature = |d: &mut Vec<u8>, res: &str, tag: &str, id: i32, name: Option<&str>, stats: &[(u16, f32, bool)]| -> Vec<u8> {
            let res_at = string(d, res);
            let tag_at = string(d, tag);
            let name_at = name.map_or(0, |n| string(d, n));
            // Stat structs.
            let mut refs = vec![];
            for &(id, v, attr) in stats {
                let at = d.len() as u32;
                d.extend(v.to_le_bytes());
                if attr {
                    d.extend(2.0f32.to_le_bytes());
                }
                d.extend(id.to_le_bytes());
                d.extend([0u8; 2]);
                refs.push((if attr { 5u16 } else { 6u16 }, at));
            }
            let list_at = d.len() as u32;
            w32(d, refs.len() as u32);
            for (t, at) in refs {
                d.extend(t.to_le_bytes());
                d.extend(0x4000u16.to_le_bytes());
                w32(d, at);
            }
            let cps = d.len() as u32;
            w32(d, list_at);
            let mut c = vec![];
            w32(&mut c, res_at);
            c.extend(id.to_le_bytes());
            w32(&mut c, tag_at);
            w32(&mut c, cps);
            w32(&mut c, 1000);
            w32(&mut c, name_at);
            w32(&mut c, 0xFFFF_FFFF);
            c
        };
        let pc = mk_creature(&mut d, "default_player", "player", 1, Some("Aeducan"), &[(1, 15.0, true), (15, 7.0, false), (19, 9000.0, false), (34, 3.0, false)]);
        let player_at = d.len() as u32;
        d.extend(pc);
        let fc = mk_creature(&mut d, "gen00fl_alistair", "", 42, None, &[(1, 18.0, true), (15, 6.0, false), (19, 8000.0, false)]);
        let followers_at = d.len() as u32;
        w32(&mut d, 1);
        d.extend(fc);
        let res_at = string(&mut d, "gen_im_qck_health_101");
        let items_at = d.len() as u32;
        w32(&mut d, 1);
        w32(&mut d, res_at);
        d.extend(8i32.to_le_bytes());
        let apr_at = d.len() as u32;
        w32(&mut d, 2);
        for (id, v) in [(42u32, 30i32), (7, -5)] {
            w32(&mut d, id);
            d.extend(v.to_le_bytes());
        }
        // PLYR inline at root+0 → reference to the player creature.
        put(&mut d, 0, &player_at.to_le_bytes());
        put(&mut d, 8, &followers_at.to_le_bytes());
        put(&mut d, 12, &items_at.to_le_bytes());
        put(&mut d, 16, &apr_at.to_le_bytes());
        build(b"SAVE", b"V1.1", &templates, &d)
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Party/Money (copper)"), Some(&1234567.into()));
        assert_eq!(v.pointer("/Player/Name"), Some(&"Aeducan".into()));
        assert_eq!(v.pointer("/Player/Strength"), Some(&15.into()));
        assert_eq!(v.pointer("/Player/Level").and_then(|x| x.as_f64()), Some(7.0));
        assert_eq!(v.pointer("/Player/Attribute points").and_then(|x| x.as_f64()), Some(3.0));
        assert_eq!(v.pointer("/Followers/Alistair/Experience").and_then(|x| x.as_f64()), Some(8000.0));
        assert_eq!(v.pointer("/Followers/Alistair/Approval"), Some(&30.into()));
        assert_eq!(v.pointer("/Other approval/Follower #7"), Some(&(-5).into()));
        assert_eq!(v.pointer("/Backpack/01 Qck health 101"), Some(&8.into()));
        doc.set("/Party/Money (copper)", 1234568.into()).unwrap();
        doc.set("/Player/Strength", 20.into()).unwrap();
        assert!(doc.set("/Player/Strength", 101.into()).is_err());
        doc.set("/Followers/Alistair/Approval", 100.into()).unwrap();
        doc.set("/Backpack/01 Qck health 101", 99.into()).unwrap();
        assert!(doc.set("/Followers/Alistair/Approval", 101.into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Party/Money (copper)"), Some(&1234568.into()));
        assert_eq!(again.view.pointer("/Player/Strength"), Some(&20.into()));
        assert_eq!(again.view.pointer("/Followers/Alistair/Approval"), Some(&100.into()));
        assert_eq!(again.view.pointer("/Backpack/01 Qck health 101"), Some(&99.into()));
        assert_eq!(again.view.pointer("/Player/Level"), doc.view.pointer("/Player/Level"));
        // Dragon Age II saves (type version V2.0) are left alone.
        let mut da2 = d.clone();
        da2[16..20].copy_from_slice(b"V2.0");
        assert!(!looks(&da2));
        assert_eq!(pretty_resref("prm000im_wolf_charm"), "Wolf charm");
        assert_eq!(follower_name("gen00fl_shale", "shale_new"), "Shale");
    }

    /// Research aid: SAVE_EDITOR_DAO = .das files separated by `|`. Each must parse, write
    /// back byte for byte, and take an edit (money +1, the player's Strength) that reads
    /// back with every other value unchanged.
    #[test]
    #[ignore]
    fn real_saves() {
        for path in std::env::var("SAVE_EDITOR_DAO").unwrap().split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(path).unwrap();
            let mut doc = parse(&d).unwrap();
            println!("{path}\n{}", serde_json::to_string_pretty(&doc.view).unwrap());
            assert_eq!(doc.to_bytes().unwrap(), d, "round trip");
            let money = doc.view["Party"]["Money (copper)"].as_i64().unwrap();
            doc.set("/Party/Money (copper)", (money + 1).into()).unwrap();
            let s = doc.view["Player"]["Strength"].as_i64().unwrap();
            doc.set("/Player/Strength", (s + 1).into()).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            let changed = (0..d.len()).filter(|&i| d[i] != out[i]).count();
            assert!((2..=8).contains(&changed), "{changed} bytes changed");
            let again = parse(&out).unwrap();
            assert_eq!(again.view, doc.view);
        }
    }
}
