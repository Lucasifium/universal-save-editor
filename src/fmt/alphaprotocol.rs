//! Alpha Protocol checkpoint saves (`Checkpoints\*.sav`, including `default.sav` and
//! `safehouse.sav`): an Unreal-engine style dump in which every named variable is written as
//! a big-endian length that counts the trailing zero, the name, the zero, then a big-endian
//! 32-bit value. The names and their order are plain text in the file (`Money`, `PlayerLevel`,
//! `AdvancementPoints`, `ExperiencePoints`, `SKILL_<name>_Level`, `AMMO_<name>_Reserve`,
//! `GADGET_nn_Num`). Values are edited in place; the length of the file never changes and no
//! checksum has been found.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

pub const KIND: &str = "Alpha Protocol checkpoint";

pub fn looks(d: &[u8]) -> bool {
    d.len() > 4000
        && d.starts_with(&[0, 0, 5, 0x93])
        && d.windows(17).any(|w| w == b"AdvancementPoints")
        && d.windows(11).any(|w| w == b"PlayerLevel")
}

/// Every `[len][name\0][value]` variable: name → offset of its value (the first one wins).
fn variables(d: &[u8]) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    let mut i = 0;
    while i + 8 < d.len() {
        if d[i] == 0 && d[i + 1] == 0 && d[i + 2] == 0 {
            let l = d[i + 3] as usize;
            if (2..=90).contains(&l) && i + 4 + l + 4 <= d.len() {
                let name = &d[i + 4..i + 4 + l - 1];
                if d[i + 4 + l - 1] == 0
                    && name.first().is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
                    && name.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    out.entry(String::from_utf8_lossy(name).into_owned()).or_insert(i + 4 + l);
                    i += 4 + l + 4;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not an Alpha Protocol save".into());
    }
    let vars = variables(d);
    let mut b = Builder::new(d);
    b.big_endian = true;
    let mut found = 0;
    for name in ["PlayerLevel", "ExperiencePoints", "ExperiencePointsTotal", "AdvancementPoints", "AdvancementPointsTotal", "Money"] {
        if let Some(&at) = vars.get(name) {
            let hi = if name == "PlayerLevel" { 99 } else { 99_999_999 };
            b.ranged(&["Agent", name], at, Kind::I32, 0, hi);
            found += 1;
        }
    }
    let mut keys: Vec<(&String, &usize)> = vars.iter().collect();
    keys.sort_by_key(|(_, at)| **at);
    for (name, &at) in keys {
        let (group, short, hi): (&str, &str, i64) = if let Some(s) = name.strip_prefix("SKILL_").and_then(|s| s.strip_suffix("_Level")) {
            ("Skills", s, 99)
        } else if let Some(s) = name.strip_prefix("AMMO_").and_then(|s| s.strip_suffix("_Reserve")) {
            ("Ammunition", s, 9999)
        } else if let Some(s) = name.strip_prefix("GADGET_").and_then(|s| s.strip_suffix("_Num")).filter(|s| s.len() == 2 && s.bytes().all(|c| c.is_ascii_digit())) {
            ("Gadgets", s, 99)
        } else {
            continue;
        };
        b.ranged(&[group, short], at, Kind::I32, 0, hi);
    }
    if found < 3 {
        return Err("the agent's values aren't in this file".into());
    }
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Edit one checkpoint at a time; the game reads Checkpoints\\*.sav and safehouse.sav."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, v: i32) -> Vec<u8> {
        let mut o = (name.len() as u32 + 1).to_be_bytes().to_vec();
        o.extend(name.as_bytes());
        o.push(0);
        o.extend(v.to_be_bytes());
        o
    }

    #[test]
    fn reads_and_edits_money() {
        let mut d = vec![0, 0, 5, 0x93, 0, 0, 0, 4];
        for (n, v) in [("AdvancementPoints", 2), ("PlayerLevel", 18), ("ExperiencePoints", 40946), ("Money", 177_534), ("SKILL_PRIMARY_STEALTH_Level", 3), ("AMMO_SMG_Reserve", 360)] {
            d.extend(var(n, v));
        }
        d.resize(5000, 0);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Agent/Money").unwrap(), 177_534);
        assert_eq!(doc.view.pointer("/Skills/PRIMARY_STEALTH").unwrap(), 3);
        assert_eq!(doc.view.pointer("/Ammunition/SMG").unwrap(), 360);
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Agent/Money", 5_000_000.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(parse(&out).unwrap().view.pointer("/Agent/Money").unwrap(), 5_000_000);
    }
}
