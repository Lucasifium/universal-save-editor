//! Shin Megami Tensei: Lucifer's Call / Nocturne (PlayStation 2, USA and Europe): the save file
//! of the save folder (107,792 bytes, starting with "VER"). Offsets and names follow the
//! MIT-licensed save editor in RyudoSynbios/game-tools-collection (see THIRD_PARTY_NOTICES.md).
//! The save has no checksum. The Japanese and Korean layouts are not read.

use super::fields::{Builder, Checksum, Kind};
use super::retro::flag;
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "Shin Megami Tensei: Lucifer's Call (PlayStation 2) save";

const LEN: usize = 107_792;

pub fn looks(d: &[u8]) -> bool {
    d.len() == LEN && d.starts_with(b"VER\x05")
}

struct Data {
    demons: HashMap<u16, String>,
    skills: Vec<(String, u64)>,
    items: Vec<(usize, String, &'static str)>,
    special: Vec<(usize, String)>,
}

fn data() -> Data {
    let mut out = Data { demons: HashMap::new(), skills: vec![("-".to_string(), 0)], items: vec![], special: vec![] };
    for l in include_str!("data/smt3.tsv").lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let f: Vec<&str> = l.split('\t').collect();
        let idx: usize = f[1].parse().unwrap_or(0);
        match f[0] {
            "demon" => {
                out.demons.insert(idx as u16, f[3].to_string());
            }
            "skill" => out.skills.push((f[3].to_string(), idx as u64)),
            "item" => {
                let (ty, sub) = f[2].split_once('.').unwrap_or(("0", "0"));
                let group = match (ty, sub) {
                    ("0", "0") => "Recovery items",
                    ("0", "1") => "Incenses",
                    ("0", "2") => "Battle items",
                    ("0", _) => "Miscellaneous items",
                    _ => "Gems",
                };
                out.items.push((idx, f[3].to_string(), group));
            }
            "special" if f[2] == "0" => out.special.push((idx, f[3].to_string())),
            _ => {}
        }
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Lucifer's Call save".into());
    }
    let data = data();
    let mut b = Builder::new(d);
    b.ranged(&["General", "Macca"], 0xa48, Kind::U32, 0, 9_999_999);
    b.bits(&["General", "Cycle"], 0xc, Kind::U16, 0, 16, 1, 1, 99);
    for i in 0..13usize {
        let base = if i == 0 { 0xa54 } else { 0xa54 + (i + 3) * 0xec };
        let demon = u16::from_le_bytes([d[base + 4], d[base + 5]]);
        let max_hp = u16::from_le_bytes([d[base + 8], d[base + 9]]);
        if i > 0 && demon == 0 && max_hp == 0 {
            continue;
        }
        let name = if i == 0 { "Protagonist".to_string() } else { format!("Demon {i}") };
        let shown = if i == 0 { "Protagonist".to_string() } else { data.demons.get(&demon).cloned().unwrap_or_else(|| format!("Demon {demon}")) };
        b.info(&["Party", &name, "Demon"], shown);
        b.ranged(&["Party", &name, "Level"], base + 0x14, Kind::U16, 1, 99);
        b.ranged(&["Party", &name, "Experience"], base + 0x10, Kind::U32, 0, 99_999_999);
        b.ranged(&["Party", &name, "HP"], base + 6, Kind::U16, 0, 999);
        b.ranged(&["Party", &name, "Max HP"], base + 8, Kind::U16, 1, 999);
        b.ranged(&["Party", &name, "MP"], base + 0xa, Kind::U16, 0, 999);
        b.ranged(&["Party", &name, "Max MP"], base + 0xc, Kind::U16, 1, 999);
        for (n, at) in [("Strength", 0x16), ("Magic", 0x18), ("Vitality", 0x19), ("Agility", 0x1a), ("Luck", 0x1b)] {
            b.ranged(&["Party", &name, n], base + at, Kind::U8, 0, 99);
        }
        b.ranged(&["Skills", &name, "Skill slots"], base + 0x38, Kind::U16, 0, 8);
        for k in 0..8 {
            b.pick(&["Skills", &name, &format!("Skill {}", k + 1)], base + 0x3a + k * 2, Kind::U16, &data.skills);
        }
    }
    let mut taken: HashMap<&str, std::collections::HashSet<String>> = HashMap::new();
    for (idx, n, group) in &data.items {
        let seen = taken.entry(group).or_default();
        let label = if seen.insert(n.clone()) { n.clone() } else { format!("{n} ({idx})") };
        b.ranged(&["Items", group, &label], 0x1928 + idx, Kind::U8, 0, 99);
    }
    for (idx, n) in &data.special {
        flag(&mut b, &["Special items", n], 0x8ba + idx / 8, (idx % 8) as u32);
    }
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. This reads the USA and European saves; the game keeps no checksum. A demon's own identity, its resistances and the magatama are not edited here; item stocks are capped at 99.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_party_and_items() {
        let mut d = vec![0u8; LEN];
        d[..4].copy_from_slice(b"VER\x05");
        d[0xa54 + 8] = 30;
        d[0xa54 + 0x14] = 1;
        let mut doc = parse(&d).unwrap();
        doc.set("/General/Macca", serde_json::json!(1234)).unwrap();
        doc.set("/Party/Protagonist/Level", serde_json::json!(40)).unwrap();
        doc.set("/Items/Recovery items/Medicine", serde_json::json!(9)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes(out[0xa48..0xa4c].try_into().unwrap()), 1234);
        assert_eq!(out[0xa54 + 0x14], 40);
        assert_eq!(out[0x1928 + 2], 9);
    }
}
