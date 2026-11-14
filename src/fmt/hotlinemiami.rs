//! Hotline Miami `SaveData.sav`: a byte 1, then named files (u32 name length, name, u32 size,
//! bytes) up to a 0xDEADBEEF end mark, padded with zeros to 256 KiB. The progress lives in the
//! text file `saves.dat`: lines of 0/1 flags (masks, levels, weapons, puzzle letters), the overall
//! score, 22 level ranks, 22 high scores and the puzzle-solved flag.
//!
//! The bundle and line layout follow thisisignitedoreo/hm-savedit (WTFPL; see THIRD_PARTY_NOTICES.md).
//! Only the values that changed are rewritten; every other byte is kept.

use serde_json::{json, Map, Value};

const MASKS: [&str; 26] = [
    "Richard", "Rasmus", "Tony", "Aubrey", "Don Juan", "Graham", "Dennis", "George", "Ted", "Rufus", "Rami", "Willem", "Peter",
    "Zack", "Oscar", "Rick", "Brandon", "Charlie", "Louie", "Phil", "Nigel", "Earl", "Jones", "Carl", "Jake", "Richter",
];
const LEVELS: [&str; 20] = [
    "The Metro", "No Talk", "Overdose", "Decadence", "Tension", "Full House", "Clean Hit", "Neighbors", "Push It", "Crackdown",
    "Hot & Heavy", "Deadline", "Trauma", "Assault", "Vengeance", "Showdown", "Safehouse", "Fun & Games", "Prank Call", "Resolution",
];
const WEAPONS: [&str; 16] = [
    "Katana", "Glass Bottle", "Magnum", "Machete", "Beer Can", "Skorpion", "Hammer", "Brick", "Uzi", "Sledgehammer", "Ninja Star",
    "Dart", "MP5", "Fireaxe", "Scissors", "Silenced Pistol",
];
const LETTERS: &str = "IWASBORNINTHEUSA";
const SAVES: &[u8] = b"saves.dat";
const END: u32 = 0xDEAD_BEEF;

/// (offset of the `saves.dat` size field, offset of its bytes, its size) when `data` is such a bundle.
fn locate(data: &[u8]) -> Option<(usize, usize, usize)> {
    if data.first() != Some(&1) {
        return None;
    }
    let mut p = 1;
    let mut found = None;
    for _ in 0..64 {
        let n = u32::from_le_bytes(data.get(p..p + 4)?.try_into().ok()?);
        if n == END {
            return found;
        }
        let n = n as usize;
        if n == 0 || n > 64 {
            return None;
        }
        let name = data.get(p + 4..p + 4 + n)?;
        if !name.iter().all(|c| c.is_ascii_graphic()) {
            return None;
        }
        let size_at = p + 4 + n;
        let m = u32::from_le_bytes(data.get(size_at..size_at + 4)?.try_into().ok()?) as usize;
        data.get(size_at + 4..size_at + 4 + m)?;
        if name == SAVES {
            found = Some((size_at, size_at + 4, m));
        }
        p = size_at + 4 + m;
    }
    None
}

fn flags(line: &str, names: &[&str]) -> Value {
    let b = line.as_bytes();
    let mut o = Map::new();
    for (i, n) in names.iter().enumerate() {
        o.insert(n.to_string(), json!({ "Unlocked": b.get(i) == Some(&b'1') }));
    }
    Value::Object(o)
}

fn level_name(i: usize) -> String {
    LEVELS.get(i).map_or_else(|| format!("Level {}", i + 1), |n| n.to_string())
}

fn letters() -> Vec<String> {
    LETTERS.chars().enumerate().map(|(i, c)| format!("{} ({c})", i + 1)).collect()
}

/// The `saves.dat` text and the values shown, when `data` is a Hotline Miami save.
pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    let (_, at, n) = locate(data)?;
    let text = std::str::from_utf8(&data[at..at + n]).ok()?;
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() < 50 || lines[0].len() < 20 || !lines[0].bytes().all(|c| c == b'0' || c == b'1') {
        return None;
    }
    let score = lines[4].trim().parse::<f64>().ok()?;
    let mut ranks = Map::new();
    for i in 0..22 {
        let r: i64 = lines[5 + i].trim().parse().ok()?;
        let s: i64 = lines[27 + i].trim().parse().ok()?;
        ranks.insert(level_name(i), json!({ "Rank": r, "High score": s }));
    }
    let names = letters();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let v = json!({
        "Hotline Miami": { "Overall score": score.round() as i64, "Puzzle solved": lines[49].trim() == "1" },
        "Masks": flags(lines[0], &MASKS),
        "Levels": flags(lines[1], &LEVELS),
        "Weapons": flags(lines[2], &WEAPONS),
        "Puzzle letters": flags(lines[3], &names),
        "Level ranks": Value::Object(ranks),
    });
    serde_json::to_vec_pretty(&v).ok()
}

fn set_flags(line: &mut String, names: &[&str], o: Option<&Value>) {
    let Some(o) = o else { return };
    let mut b = line.clone().into_bytes();
    for (i, n) in names.iter().enumerate() {
        if let (Some(c), Some(u)) = (b.get_mut(i), o.get(*n).and_then(|e| e.get("Unlocked")).and_then(Value::as_bool)) {
            *c = if u { b'1' } else { b'0' };
        }
    }
    *line = String::from_utf8(b).unwrap_or_else(|_| line.clone());
}

fn set_int(tok: &mut String, v: Option<&Value>) {
    if let Some(n) = v.and_then(Value::as_i64) {
        if tok.trim().parse::<i64>().ok() != Some(n) {
            *tok = n.max(0).to_string();
        }
    }
}

pub fn pack(original: &[u8], json: &[u8]) -> Result<Vec<u8>, String> {
    let (size_at, at, n) = locate(original).ok_or("not a Hotline Miami save")?;
    let v: Value = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let text = std::str::from_utf8(&original[at..at + n]).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.len() < 50 {
        return Err("saves.dat is too short".into());
    }
    set_flags(&mut lines[0], &MASKS, v.get("Masks"));
    set_flags(&mut lines[1], &LEVELS, v.get("Levels"));
    set_flags(&mut lines[2], &WEAPONS, v.get("Weapons"));
    let names = letters();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    set_flags(&mut lines[3], &names, v.get("Puzzle letters"));
    if let Some(h) = v.get("Hotline Miami") {
        if let Some(s) = h.get("Overall score").and_then(Value::as_i64) {
            let old = lines[4].trim().parse::<f64>().unwrap_or(-1.0).round() as i64;
            if old != s {
                lines[4] = s.max(0).to_string();
            }
        }
        if let Some(p) = h.get("Puzzle solved").and_then(Value::as_bool) {
            if (lines[49].trim() == "1") != p {
                lines[49] = if p { "1".into() } else { "0".into() };
            }
        }
    }
    if let Some(r) = v.get("Level ranks") {
        for i in 0..22 {
            let Some(e) = r.get(level_name(i)) else { continue };
            set_int(&mut lines[5 + i], e.get("Rank"));
            set_int(&mut lines[27 + i], e.get("High score"));
        }
    }
    let new = lines.join("\n").into_bytes();
    let mut out = original[..size_at].to_vec();
    out.extend_from_slice(&(new.len() as u32).to_le_bytes());
    out.extend_from_slice(&new);
    // Later files, the end mark and the padding after it (which may hold leftover bytes) are kept;
    // the file keeps its size unless the new text no longer fits before it.
    out.extend_from_slice(&original[at + n..]);
    let keep = locate_end(&out).ok_or("no end mark")?.max(original.len());
    out.truncate(keep);
    out.resize(keep, 0);
    Ok(out)
}

/// The offset just after the 0xDEADBEEF end mark.
fn locate_end(data: &[u8]) -> Option<usize> {
    let mut p = 1;
    loop {
        let n = u32::from_le_bytes(data.get(p..p + 4)?.try_into().ok()?);
        if n == END {
            return Some(p + 4);
        }
        let size_at = p + 4 + n as usize;
        let m = u32::from_le_bytes(data.get(size_at..size_at + 4)?.try_into().ok()?) as usize;
        p = size_at + 4 + m;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(saves: &str) -> Vec<u8> {
        let mut d = vec![1u8];
        for (name, body) in [("mask.nfo", "13".as_bytes()), ("saves.dat", saves.as_bytes()), ("saveversion.dat", b"1.01")] {
            d.extend_from_slice(&(name.len() as u32).to_le_bytes());
            d.extend_from_slice(name.as_bytes());
            d.extend_from_slice(&(body.len() as u32).to_le_bytes());
            d.extend_from_slice(body);
        }
        d.extend_from_slice(&END.to_le_bytes());
        d.resize(256 * 1024, 0);
        d
    }

    fn text() -> String {
        let mut l = vec!["1".repeat(26), "1".repeat(20), "0".repeat(16), "0".repeat(16), "3.92761e+006".to_string()];
        l.extend((0..22).map(|_| "15".to_string()));
        l.extend((0..22).map(|i| (1000 + i).to_string()));
        l.push("1".into());
        l.join("\n")
    }

    #[test]
    fn round_trip_and_edit() {
        let d = bundle(&text());
        let js = open(&d).unwrap();
        assert_eq!(pack(&d, &js).unwrap(), d);
        let mut v: Value = serde_json::from_slice(&js).unwrap();
        assert_eq!(v["Hotline Miami"]["Overall score"], 3_927_610);
        v["Weapons"]["Katana"]["Unlocked"] = json!(true);
        v["Level ranks"]["No Talk"]["High score"] = json!(99_999);
        v["Hotline Miami"]["Overall score"] = json!(5_000_000);
        let out = pack(&d, &serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(out.len(), d.len());
        let w: Value = serde_json::from_slice(&open(&out).unwrap()).unwrap();
        assert_eq!(w["Weapons"]["Katana"]["Unlocked"], true);
        assert_eq!(w["Weapons"]["Magnum"]["Unlocked"], false);
        assert_eq!(w["Level ranks"]["No Talk"]["High score"], 99_999);
        assert_eq!(w["Hotline Miami"]["Overall score"], 5_000_000);
        assert!(out.windows(15).any(|x| x == b"saveversion.dat"));
        assert!(open(b"\x01abc").is_none());
    }

    #[test]
    fn public_sample() {
        let Ok(p) = std::env::var("HOTLINE_SAMPLE") else { return };
        let d = std::fs::read(p).unwrap();
        let js = open(&d).unwrap();
        assert_eq!(pack(&d, &js).unwrap(), d);
    }
}
