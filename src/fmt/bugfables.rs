//! Bug Fables: The Everlasting Sapling (PC) `save0.dat` … `save2.dat`: UTF-8 text in which every
//! character is the save's character XOR 543. The plain text is 18 lines (header, party, global
//! values, medal shops, quests, items, medals, …) of `,` and `@` separated values.
//!
//! The line order and field order follow aldelaro5/Bug-Fables-Save-Editor (MIT; see
//! THIRD_PARTY_NOTICES.md). Only the values shown are rewritten; every other character is kept.

use serde_json::{json, Map, Value};

const KEY: u32 = 543;
const GAME: &str = "Bug Fables";

/// Line 2 (global values): (JSON key, index). Index 6 and 7 are the map and area, 15 the icon.
const GLOBAL: [(&str, usize); 13] = [
    ("Rank", 0),
    ("Experience", 1),
    ("Experience needed", 2),
    ("Max TP", 3),
    ("TP", 4),
    ("Berries", 5),
    ("MP", 8),
    ("Max MP", 9),
    ("Inventory slots", 10),
    ("Storage slots", 11),
    ("Play time (hours)", 12),
    ("Play time (minutes)", 13),
    ("Play time (seconds)", 14),
];

/// Line 1, one `@` entry per member: (JSON key, index). Index 0 is the member's id.
const MEMBER: [(&str, usize); 7] =
    [("HP", 1), ("Max HP", 2), ("Base HP", 3), ("Attack", 4), ("Base attack", 5), ("Defense", 6), ("Base defense", 7)];

const NAMES: [&str; 3] = ["Vi", "Kabbu", "Leif"];

fn xor(text: &str) -> Option<String> {
    text.chars().map(|c| char::from_u32(c as u32 ^ KEY)).collect()
}

fn int(t: &str) -> Option<i64> {
    t.trim().parse().ok()
}

/// The plain save text and the values shown, when `data` is a PC save.
pub fn open(data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    // The text starts with the player's X position: a digit or '-' (0x30..0x39, 0x2D XOR 543).
    let first = std::str::from_utf8(data.get(..2)?).ok()?.chars().next()? as u32 ^ KEY;
    if !(first == 0x2D || (0x30..=0x39).contains(&first)) {
        return None;
    }
    let plain = xor(std::str::from_utf8(data).ok()?)?;
    let lines: Vec<&str> = plain.split('\n').collect();
    if lines.len() < 18 || lines[0].split(',').count() != 10 {
        return None;
    }
    let global: Vec<&str> = lines[2].split(',').collect();
    if global.len() != 16 || global.iter().any(|t| int(t).is_none()) {
        return None;
    }
    let mut g = Map::new();
    for (k, i) in GLOBAL {
        g.insert(k.into(), json!(int(global[i])?));
    }
    let mut party = vec![];
    for m in lines[1].split('@') {
        let f: Vec<&str> = m.split(',').collect();
        if f.len() != 8 || f.iter().any(|t| int(t).is_none()) {
            return None;
        }
        let id = int(f[0])?;
        let name = usize::try_from(id).ok().and_then(|i| NAMES.get(i)).map(|n| n.to_string()).unwrap_or_else(|| format!("Member {id}"));
        let mut o = Map::new();
        o.insert("Member".into(), json!(name));
        for (k, i) in MEMBER {
            o.insert(k.into(), json!(int(f[i])?));
        }
        party.push(Value::Object(o));
    }
    let js = serde_json::to_vec_pretty(&json!({ GAME: Value::Object(g), "Party": party })).ok()?;
    Some((plain.into_bytes(), js))
}

/// Writes `tok` again only when the value really changed.
fn patch(tok: &mut String, new: Option<&Value>) {
    if let Some(n) = new.and_then(Value::as_i64) {
        if int(tok) != Some(n) {
            *tok = n.clamp(i64::from(i32::MIN), i64::from(i32::MAX)).to_string();
        }
    }
}

pub fn pack(plain: &[u8], json: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(plain).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.len() < 3 {
        return Err("not a Bug Fables save".into());
    }
    if let Some(g) = v.get(GAME) {
        let mut f: Vec<String> = lines[2].split(',').map(str::to_string).collect();
        for (k, i) in GLOBAL {
            if let Some(t) = f.get_mut(i) {
                patch(t, g.get(k));
            }
        }
        lines[2] = f.join(",");
    }
    if let Some(p) = v.get("Party").and_then(Value::as_array) {
        let mut ms: Vec<String> = lines[1].split('@').map(str::to_string).collect();
        for (m, o) in ms.iter_mut().zip(p) {
            let mut f: Vec<String> = m.split(',').map(str::to_string).collect();
            for (k, i) in MEMBER {
                if let Some(t) = f.get_mut(i) {
                    patch(t, o.get(k));
                }
            }
            *m = f.join(",");
        }
        lines[1] = ms.join("@");
    }
    let out = xor(&lines.join("\n")).ok_or("a character can't be written back")?;
    Ok(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let plain = "-7,0,3.17,False,True,False,False,False,False,Test\n0,17,17,17,9,4,0,2@1,22,22,22,2,2,0,0@2,22,22,22,2,2,0,0\n27,140,140,39,39,367,0,1,2,32,15,35,99,2,46,6\n@\n@\n0@61\n1,2@@\n@\n@\n@\n@\n0\n\n0\n0\n0\n0\n0,0@0,0";
        xor(plain).unwrap().into_bytes()
    }

    #[test]
    fn round_trip_and_edit() {
        let d = sample();
        let (plain, js) = open(&d).unwrap();
        assert_eq!(pack(&plain, &js).unwrap(), d);
        let mut v: Value = serde_json::from_slice(&js).unwrap();
        assert_eq!(v["Bug Fables"]["Berries"], 367);
        assert_eq!(v["Party"][1]["Member"], "Kabbu");
        v["Bug Fables"]["Berries"] = json!(9999);
        v["Party"][0]["Max HP"] = json!(60);
        let out = pack(&plain, &serde_json::to_vec(&v).unwrap()).unwrap();
        let (p2, j2) = open(&out).unwrap();
        let v2: Value = serde_json::from_slice(&j2).unwrap();
        assert_eq!(v2["Bug Fables"]["Berries"], 9999);
        assert_eq!(v2["Party"][0]["Max HP"], 60);
        assert_eq!(v2["Party"][0]["HP"], 17);
        assert_eq!(p2.len(), plain.len() + 1);
        assert!(open(b"{\"a\": 1}").is_none());
    }

    #[test]
    fn public_sample() {
        let Ok(p) = std::env::var("BUGFABLES_SAMPLE") else { return };
        let d = std::fs::read(p).unwrap();
        let (plain, js) = open(&d).unwrap();
        assert_eq!(pack(&plain, &js).unwrap(), d);
    }
}
