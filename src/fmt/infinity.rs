//! Infinity Engine Enhanced Edition saves: `BALDUR.gam` (Baldur's Gate: EE, Siege of
//! Dragonspear, Baldur's Gate II: EE) and `ICEWIND.gam` (Icewind Dale: EE), format
//! `GAMEV2.0`. A fixed header (party gold at 0x18, reputation ×10 at 0x54), a table of
//! in-party NPC structs (0x160 bytes, each pointing at an embedded `CRE V1.0` creature) and
//! a table of global variables (0x54 bytes: name, …, value at +0x28).
//!
//! The creature holds experience (0x18), hit points (0x24 / 0x26), levels (0x234–0x236),
//! abilities (0x238–0x23E), race / class (0x272 / 0x273), alignment (0x27B) and its item
//! table (offset / count at 0x2BC / 0x2C0, 0x14 bytes per item: resref, …, three u16
//! charges). Layouts per IESDP (facts only), checked on sample BG:EE and BG2:EE saves.
//! Everything here is a fixed-size value, edited in place; no checksum.
//!
//! Planescape: Torment: Enhanced Edition writes the same `GAMEV2.0` / `CRE V1.0` files
//! (also named `BALDUR.gam`), checked on real PST:EE saves. It is told apart by the
//! protagonist's death variable `Nameless`, and its creatures use PST's own RACE.IDS.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Infinity Engine save (GAM V2.0)";
pub const KIND_PST: &str = "Planescape: Torment EE save (GAM V2.0)";

const NPC_SIZE: usize = 0x160;
const VAR_SIZE: usize = 0x54;
const ITEM_SIZE: usize = 0x14;

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// A NUL-padded text field.
fn text(d: &[u8], at: usize, n: usize) -> String {
    d.get(at..at + n)
        .unwrap_or_default()
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| super::bethesda_cp1252(c))
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 0xB4
        && d.starts_with(b"GAMEV2.0")
        && table(d, 0x20, NPC_SIZE).is_some_and(|(_, n)| (1..=6).contains(&n))
}

/// (offset, count) of a table whose header fields are at `at`, if it fits in the file.
fn table(d: &[u8], at: usize, size: usize) -> Option<(usize, usize)> {
    let (off, n) = (u32_at(d, at)? as usize, u32_at(d, at + 4)? as usize);
    (n <= 100_000 && off.checked_add(n.checked_mul(size)?)? <= d.len()).then_some((off, n))
}

/// ALIGNMEN.IDS.
const ALIGNMENTS: [(&str, u64); 9] = [
    ("Lawful good", 0x11),
    ("Lawful neutral", 0x12),
    ("Lawful evil", 0x13),
    ("Neutral good", 0x21),
    ("True neutral", 0x22),
    ("Neutral evil", 0x23),
    ("Chaotic good", 0x31),
    ("Chaotic neutral", 0x32),
    ("Chaotic evil", 0x33),
];

/// CLASS.IDS (player classes); a multi-class lists its classes in the order of its levels.
fn class_name(id: u8) -> Option<&'static str> {
    Some(match id {
        1 => "Mage",
        2 => "Fighter",
        3 => "Cleric",
        4 => "Thief",
        5 => "Bard",
        6 => "Paladin",
        7 => "Fighter/Mage",
        8 => "Fighter/Cleric",
        9 => "Fighter/Thief",
        10 => "Fighter/Mage/Thief",
        11 => "Druid",
        12 => "Ranger",
        13 => "Mage/Thief",
        14 => "Cleric/Mage",
        15 => "Cleric/Thief",
        16 => "Fighter/Druid",
        17 => "Fighter/Mage/Cleric",
        18 => "Cleric/Ranger",
        19 => "Sorcerer",
        20 => "Monk",
        21 => "Shaman",
        _ => return None,
    })
}

/// RACE.IDS (player races).
fn race_name(id: u8) -> Option<&'static str> {
    Some(match id {
        1 => "Human",
        2 => "Elf",
        3 => "Half-elf",
        4 => "Dwarf",
        5 => "Halfling",
        6 => "Gnome",
        7 => "Half-orc",
        _ => return None,
    })
}

/// RACE.IDS of Planescape: Torment (the entries party members use).
fn pst_race_name(id: u8) -> Option<&'static str> {
    Some(match id {
        1 => "Human",
        6 => "Annah",
        12 => "Dakkon",
        19 => "Fall-From-Grace",
        30 => "Githzerai",
        35 => "Ignus",
        45 => "Morte",
        48 => "Nordom",
        64 => "Tiefling",
        68 => "Vhailor",
        _ => return None,
    })
}

/// A Planescape: Torment: EE save: a party member has the death variable `Nameless`.
pub fn is_pst(d: &[u8]) -> bool {
    let Some((off, n)) = table(d, 0x20, NPC_SIZE) else {
        return false;
    };
    looks(d)
        && (0..n).map(|i| off + i * NPC_SIZE).any(|n| {
            let (Some(cre), Some(size)) = (
                u32_at(d, n + 4).map(|v| v as usize),
                u32_at(d, n + 8).map(|v| v as usize),
            ) else {
                return false;
            };
            size >= 0x2D4
                && cre.checked_add(size).is_some_and(|e| e <= d.len())
                && d[cre..].starts_with(b"CRE V1.0")
                && text(d, cre + 0x280, 32).eq_ignore_ascii_case("nameless")
        })
}

/// "VICONIA" / "jaheira" → "Viconia" / "Jaheira".
fn pretty(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f
            .to_uppercase()
            .chain(c.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

/// The value of a global variable (e.g. CHAPTER).
fn global(d: &[u8], name: &str) -> Option<(usize, i32)> {
    let (off, n) = table(d, 0x38, VAR_SIZE)?;
    (0..n)
        .map(|i| off + i * VAR_SIZE)
        .find(|&v| text(d, v, 32).eq_ignore_ascii_case(name))
        .and_then(|v| Some((v + 0x28, u32_at(d, v + 0x28)? as i32)))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a GAM V2.0 save".into());
    }
    let pst = is_pst(d);
    let mut b = Builder::new(d);
    b.field(&["Party", "Gold"], 0x18, Kind::U32);
    let reps: Vec<(String, u64)> = (1..=20).map(|r| (r.to_string(), r * 10)).collect();
    if b.pick(&["Party", "Reputation"], 0x54, Kind::U32, &reps)
        .is_none()
    {
        b.ranged(&["Party", "Reputation (x10)"], 0x54, Kind::U32, 10, 200);
    }
    if let Some((_, ch)) = global(d, "CHAPTER") {
        b.info(&["Party", "Chapter"], ch);
    }
    if let Some(t) = u32_at(d, 0x08) {
        b.info(&["Party", "Game hours"], t / 300);
    }
    b.info(&["Party", "Area"], text(d, 0x58, 8));

    let (npc_off, npc_n) = table(d, 0x20, NPC_SIZE).ok_or("party table out of range")?;
    let mut npcs: Vec<(u16, usize)> = (0..npc_n)
        .map(|i| npc_off + i * NPC_SIZE)
        .map(|n| (u16_at(d, n + 2).unwrap_or(0xFFFF), n))
        .collect();
    npcs.sort_by_key(|x| x.0);
    let mut used: Vec<String> = vec![];
    let mut members = 0;
    for (order, n) in npcs {
        let (Some(cre), Some(size)) = (
            u32_at(d, n + 4).map(|v| v as usize),
            u32_at(d, n + 8).map(|v| v as usize),
        ) else {
            continue;
        };
        if size < 0x2D4
            || cre.checked_add(size).is_none_or(|e| e > d.len())
            || !d[cre..].starts_with(b"CRE V1.0")
        {
            continue;
        }
        // The protagonist's name is in the NPC struct; companions are named by their script name.
        let mut name = text(d, n + 0xC0, 32);
        if name.is_empty() {
            let death = text(d, cre + 0x280, 32);
            name = if death.is_empty() || death.eq_ignore_ascii_case("none") {
                format!("Character (name #{})", u32_at(d, cre + 8).unwrap_or(0))
            } else if pst && death.eq_ignore_ascii_case("nameless") {
                "The Nameless One".into()
            } else {
                pretty(&death)
            };
        }
        let mut key = name.clone();
        let mut k = 2;
        while used.contains(&key) {
            key = format!("{name} ({k})");
            k += 1;
        }
        used.push(key.clone());
        let m = ["Party members", key.as_str()];
        let p = |k: &'static str| [m[0], m[1], k];
        b.info(&p("Name"), name);
        if order != 0xFFFF {
            b.info(&p("Portrait slot"), order as u64 + 1);
        }
        b.info(&p("Creature"), text(d, n + 0x0C, 8));
        let class = d[cre + 0x273];
        let cname = class_name(class);
        b.info(
            &p("Class"),
            cname.map_or_else(|| format!("class #{class}"), str::to_string),
        );
        let race = d[cre + 0x272];
        // PST:EE leaves some party members' race byte at values that aren't theirs (0 for
        // the Nameless One), so only a known entry is shown there.
        match (
            pst,
            if pst {
                pst_race_name(race)
            } else {
                race_name(race)
            },
        ) {
            (_, Some(r)) => b.info(&p("Race"), r),
            (false, None) => b.info(&p("Race"), format!("race #{race}")),
            (true, None) => {}
        }
        b.ranged(&p("Experience"), cre + 0x18, Kind::U32, 0, 99_999_999);
        b.ranged(&p("Hit points"), cre + 0x24, Kind::U16, 0, 32767);
        b.ranged(&p("Max hit points"), cre + 0x26, Kind::U16, 1, 32767);
        // One level per class of a multi-class (first class first); a single class has one.
        let parts: Vec<&str> = cname
            .map(|c| c.split('/').collect())
            .unwrap_or_else(|| vec!["Class"]);
        for (i, part) in parts.iter().enumerate().take(3) {
            let label = if parts.len() == 1 {
                "Level".to_string()
            } else {
                format!("{part} level")
            };
            b.ranged(
                &[m[0], m[1], &label],
                cre + 0x234 + i,
                Kind::U8,
                if i == 0 { 1 } else { 0 },
                100,
            );
        }
        for (i, label) in [
            "Strength",
            "Strength bonus (18/xx)",
            "Intelligence",
            "Wisdom",
            "Dexterity",
            "Constitution",
            "Charisma",
        ]
        .iter()
        .enumerate()
        {
            let (lo, hi) = if i == 1 { (0, 100) } else { (1, 25) };
            b.ranged(&p(label), cre + 0x238 + i, Kind::U8, lo, hi);
        }
        let opts: Vec<(String, u64)> = ALIGNMENTS
            .iter()
            .map(|(n, v)| (n.to_string(), *v))
            .collect();
        if b.pick(&p("Alignment"), cre + 0x27B, Kind::U8, &opts)
            .is_none()
        {
            b.info(&p("Alignment"), format!("0x{:02X}", d[cre + 0x27B]));
        }
        // Items: stack size / charges (the first counter is the stack size of ammunition
        // and potions, a wand's charges otherwise).
        let (io, ic) = (
            u32_at(d, cre + 0x2BC).unwrap_or(0) as usize,
            u32_at(d, cre + 0x2C0).unwrap_or(0) as usize,
        );
        if ic <= 1000 && io.checked_add(ic * ITEM_SIZE).is_some_and(|e| e <= size) {
            let width = ic.to_string().len().max(2);
            for i in 0..ic {
                let at = cre + io + i * ITEM_SIZE;
                let res = text(d, at, 8);
                let ik = format!(
                    "{:0width$} {}",
                    i + 1,
                    if res.is_empty() { "(no name)" } else { &res }
                );
                let it = [m[0], m[1], "Items", ik.as_str()];
                let mut any = false;
                for (j, label) in ["Quantity / charges", "Charges 2", "Charges 3"]
                    .iter()
                    .enumerate()
                {
                    if u16_at(d, at + 0x0A + 2 * j).is_some_and(|v| v > 0) {
                        b.ranged(
                            &[it[0], it[1], it[2], it[3], label],
                            at + 0x0A + 2 * j,
                            Kind::U16,
                            0,
                            32767,
                        );
                        any = true;
                    }
                }
                if !any {
                    b.info(&[it[0], it[1], it[2], it[3], "Quantity / charges"], 0);
                }
            }
        }
        members += 1;
    }
    if members == 0 {
        return Err("no party member found".into());
    }
    Ok(b.finish(if pst { KIND_PST } else { KIND }, Checksum::None))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A synthetic GAM V2.0 with two party members (one named, one by script name) and a
    /// CHAPTER global.
    pub fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 0xB4];
        d[..8].copy_from_slice(b"GAMEV2.0");
        d[0x08..0x0C].copy_from_slice(&3000u32.to_le_bytes());
        d[0x18..0x1C].copy_from_slice(&1234u32.to_le_bytes());
        d[0x54..0x58].copy_from_slice(&120u32.to_le_bytes());
        d[0x58..0x5E].copy_from_slice(b"AR0100");
        let npc_off = d.len();
        d.resize(npc_off + 2 * NPC_SIZE, 0);
        let var_off = d.len();
        d.resize(var_off + VAR_SIZE, 0);
        d[var_off..var_off + 7].copy_from_slice(b"CHAPTER");
        d[var_off + 0x28..var_off + 0x2C].copy_from_slice(&3i32.to_le_bytes());
        let put =
            |d: &mut Vec<u8>, at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut d, 0x20, npc_off as u32);
        put(&mut d, 0x24, 2);
        put(&mut d, 0x38, var_off as u32);
        put(&mut d, 0x3C, 1);
        for (i, (order, name, death, class, xp)) in [
            (1u16, "", "SKIE", 4u8, 900u32),
            (0, "Hero", "None", 8, 5000),
        ]
        .into_iter()
        .enumerate()
        {
            let cre = d.len();
            let mut c = vec![0u8; 0x2D4 + 2 * ITEM_SIZE];
            c[..8].copy_from_slice(b"CRE V1.0");
            c[0x18..0x1C].copy_from_slice(&xp.to_le_bytes());
            c[0x24..0x26].copy_from_slice(&20u16.to_le_bytes());
            c[0x26..0x28].copy_from_slice(&24u16.to_le_bytes());
            c[0x234..0x237].copy_from_slice(&[3, 2, 1]);
            c[0x238..0x23F].copy_from_slice(&[18, 50, 12, 13, 16, 15, 10]);
            c[0x272] = 1;
            c[0x273] = class;
            c[0x27B] = 0x22;
            c[0x280..0x280 + death.len()].copy_from_slice(death.as_bytes());
            c[0x2BC..0x2C0].copy_from_slice(&0x2D4u32.to_le_bytes());
            c[0x2C0..0x2C4].copy_from_slice(&2u32.to_le_bytes());
            c[0x2D4..0x2DA].copy_from_slice(b"POTN08");
            c[0x2D4 + 0x0A..0x2D4 + 0x0C].copy_from_slice(&5u16.to_le_bytes());
            c[0x2D4 + ITEM_SIZE..0x2D4 + ITEM_SIZE + 6].copy_from_slice(b"SW1H01");
            let n = npc_off + i * NPC_SIZE;
            d[n + 2..n + 4].copy_from_slice(&order.to_le_bytes());
            put(&mut d, n + 4, cre as u32);
            put(&mut d, n + 8, c.len() as u32);
            d[n + 0xC0..n + 0xC0 + name.len()].copy_from_slice(name.as_bytes());
            d.extend(c);
        }
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Party/Gold"), Some(&1234.into()));
        assert_eq!(v.pointer("/Party/Reputation"), Some(&"12".into()));
        assert_eq!(v.pointer("/Party/Chapter"), Some(&3.into()));
        assert_eq!(v.pointer("/Party/Game hours"), Some(&10.into()));
        // Sorted by portrait slot: the protagonist first.
        let names: Vec<&String> = v["Party members"].as_object().unwrap().keys().collect();
        assert_eq!(names, ["Hero", "Skie"]);
        assert_eq!(
            v.pointer("/Party members/Hero/Class"),
            Some(&"Fighter/Cleric".into())
        );
        assert_eq!(
            v.pointer("/Party members/Hero/Fighter level"),
            Some(&3.into())
        );
        assert_eq!(
            v.pointer("/Party members/Hero/Cleric level"),
            Some(&2.into())
        );
        assert!(v.pointer("/Party members/Skie/Thief level").is_none());
        assert_eq!(v.pointer("/Party members/Skie/Level"), Some(&3.into()));
        assert_eq!(
            v.pointer("/Party members/Skie/Strength bonus (18~1xx)"),
            Some(&50.into())
        );
        assert_eq!(
            v.pointer("/Party members/Skie/Alignment"),
            Some(&"True neutral".into())
        );
        assert_eq!(
            v.pointer("/Party members/Skie/Items/01 POTN08/Quantity ~1 charges"),
            Some(&5.into())
        );
        assert!(!doc
            .editable
            .contains_key("/Party members/Skie/Items/02 SW1H01/Quantity ~1 charges"));
        doc.set("/Party/Gold", 1235.into()).unwrap();
        doc.set("/Party/Reputation", "20".into()).unwrap();
        doc.set("/Party members/Hero/Dexterity", 18.into()).unwrap();
        doc.set("/Party members/Hero/Alignment", "Chaotic evil".into())
            .unwrap();
        doc.set(
            "/Party members/Skie/Items/01 POTN08/Quantity ~1 charges",
            10.into(),
        )
        .unwrap();
        assert!(doc.set("/Party members/Hero/Strength", 26.into()).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let diff: Vec<usize> = (0..d.len()).filter(|&i| d[i] != out[i]).collect();
        assert_eq!(diff.len(), 5);
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Party/Gold"), Some(&1235.into()));
        assert_eq!(again.view.pointer("/Party/Reputation"), Some(&"20".into()));
        assert_eq!(
            again.view.pointer("/Party members/Hero/Dexterity"),
            Some(&18.into())
        );
        assert_eq!(
            again.view.pointer("/Party members/Hero/Alignment"),
            Some(&"Chaotic evil".into())
        );
        assert_eq!(
            again
                .view
                .pointer("/Party members/Skie/Items/01 POTN08/Quantity ~1 charges"),
            Some(&10.into())
        );
        // Not a GAM.
        assert!(!looks(b"GAMEV1.1 and more bytes"));
    }

    #[test]
    fn planescape_torment_is_told_apart() {
        let mut d = sample();
        assert!(!is_pst(&d));
        // The protagonist (second NPC struct) gets PST's death variable, the other PST's Morte race.
        let npc_off = u32_at(&d, 0x20).unwrap() as usize;
        let hero = u32_at(&d, npc_off + NPC_SIZE + 4).unwrap() as usize;
        d[hero + 0x280..hero + 0x2A0].fill(0);
        d[hero + 0x280..hero + 0x288].copy_from_slice(b"Nameless");
        let skie = u32_at(&d, npc_off + 4).unwrap() as usize;
        d[skie + 0x272] = 45;
        d[hero + 0x272] = 0;
        d[npc_off + NPC_SIZE + 0xC0..npc_off + NPC_SIZE + 0xC4].fill(0);
        assert!(is_pst(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, KIND_PST);
        let v = &doc.view;
        assert!(v.pointer("/Party members/The Nameless One").is_some());
        assert!(
            v.pointer("/Party members/The Nameless One/Race").is_none(),
            "race 0 isn't a PST race"
        );
        assert_eq!(v.pointer("/Party members/Skie/Race"), Some(&"Morte".into()));
        doc.set("/Party members/The Nameless One/Experience", 7000.into())
            .unwrap();
        let again = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(
            again
                .view
                .pointer("/Party members/The Nameless One/Experience"),
            Some(&7000.into())
        );
    }

    /// Research aid: SAVE_EDITOR_IE = BALDUR.gam files separated by `|`. Each must parse,
    /// write back byte for byte, and take an edit (gold +1, a member's Dexterity) that reads
    /// back with every other value unchanged.
    #[test]
    #[ignore]
    fn real_saves() {
        for path in std::env::var("SAVE_EDITOR_IE")
            .unwrap()
            .split('|')
            .filter(|s| !s.is_empty())
        {
            let d = std::fs::read(path).unwrap();
            let mut doc = parse(&d).unwrap();
            println!(
                "{path}\n{}",
                serde_json::to_string_pretty(&doc.view).unwrap()
            );
            assert_eq!(doc.to_bytes().unwrap(), d, "round trip");
            let gold = doc.view["Party"]["Gold"].as_u64().unwrap();
            doc.set("/Party/Gold", (gold + 1).into()).unwrap();
            let first = doc.view["Party members"]
                .as_object()
                .unwrap()
                .keys()
                .next()
                .unwrap()
                .replace('~', "~0")
                .replace('/', "~1");
            let dp = format!("/Party members/{first}/Dexterity");
            let dex = doc.view.pointer(&dp).unwrap().as_u64().unwrap();
            let nd = if dex < 25 { dex + 1 } else { dex - 1 };
            doc.set(&dp, nd.into()).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            assert_eq!((0..d.len()).filter(|&i| d[i] != out[i]).count(), 2);
            let again = parse(&out).unwrap();
            assert_eq!(again.view, doc.view);
        }
    }
}
