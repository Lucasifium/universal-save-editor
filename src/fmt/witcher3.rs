//! The Witcher 3 game data (after the CD Projekt LZ4 layer). Layout from the MIT-licensed
//! W3SavegameEditor (Atvaark): "SAV3", …, a name table (`NM` + `MANU` strings), a variable
//! table, and a footer ending in the variable table's offset and "SE". Offsets count from
//! the start of the original file, so the length of what came before this data is worked
//! out from where the name table must be.
//!
//! Script objects store properties as (name index, type index, size, value). The player's
//! `levelManager` (type `handle:W3LevelManager`) appears once: it holds `level` and the
//! `points` array (skill points, then experience; each {free, used}). Geralt's money is the
//! first "Crowns" item after it: name index, id, flags, two dye names, then the quantity.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

fn u32_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"SAV3") && d.ends_with(b"SE")
}

struct Names {
    list: Vec<String>,
    /// Where the name table starts (end of the searchable game data).
    at: usize,
}

impl Names {
    fn index(&self, n: &str) -> Option<u16> {
        self.list.iter().position(|x| x == n).map(|i| (i + 1) as u16)
    }
}

fn names(d: &[u8]) -> Option<Names> {
    let vt = u32_at(d, d.len() - 6)?;
    // The offsets count from the original file's start: find the shift that makes the
    // footer point at the "NM" marker.
    for base in 0..(1 << 21).min(vt) {
        let Some(sf) = (vt - base).checked_sub(10) else { break };
        let Some(nm) = u32_at(d, sf).and_then(|n| n.checked_sub(base)) else { continue };
        if d.get(nm..nm + 6) == Some(b"NMMANU") {
            let count = u32_at(d, nm + 6)?;
            let mut p = nm + 14;
            let mut list = Vec::with_capacity(count);
            for _ in 0..count {
                let n = *d.get(p)? as usize;
                list.push(d.get(p + 1..p + 1 + n)?.iter().map(|&c| c as char).collect());
                p += 1 + n;
            }
            return Some(Names { list, at: nm });
        }
    }
    None
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// One story fact: its name and where its value is (a single entry's i16).
pub struct Fact {
    pub name: String,
    pub value_at: usize,
}

/// The story's fact table: "SBDF", a u32 count, then per fact a name (a length byte: bit 7
/// set for one byte per character, else UTF-16; bit 6 when more length bytes follow, 7
/// bits each), a u16, a u16 entry count and the entries ({i16 value, f32 time, f32 expiry
/// time}), then "EBDF". A fact's value is the sum of its entries. Only facts with one entry
/// are returned. `None` when the table doesn't read through to its end marker.
pub fn facts(d: &[u8]) -> Option<Vec<Fact>> {
    let at = find(d, b"SBDF")?;
    let n = u32_at(d, at + 4)?;
    let mut p = at + 8;
    let mut out = vec![];
    for _ in 0..n {
        let b = *d.get(p)?;
        p += 1;
        let mut len = (b & 0x3F) as usize;
        if b & 0x40 != 0 {
            let mut shift = 6;
            loop {
                let c = *d.get(p)?;
                p += 1;
                len |= ((c & 0x7F) as usize) << shift;
                shift += 7;
                if c & 0x80 == 0 || shift > 27 {
                    break;
                }
            }
        }
        let bytes = if b & 0x80 != 0 { len } else { len * 2 };
        let raw = d.get(p..p + bytes)?;
        p += bytes;
        let entries = u16::from_le_bytes(d.get(p + 2..p + 4)?.try_into().ok()?) as usize;
        p += 4;
        if b & 0x80 != 0 && entries == 1 && raw.is_ascii() {
            out.push(Fact { name: raw.iter().map(|&c| c as char).collect(), value_at: p });
        }
        p += entries * 10;
    }
    (d.get(p..p + 4)? == b"EBDF").then_some(out)
}

/// Where a quest fact is listed: its kind and area, from the quest code at the start of the
/// name (q = main, sq = side, mq = minor, mh = contracts, th = treasure hunts, cg = Gwent;
/// the first digit is the area). `None` for facts that aren't about a quest's story, such as
/// hit and kill counters, books read and music cues.
pub fn story_section(name: &str) -> Option<String> {
    const NOISE: [&str; 16] = [
        "weapon_hit", "was_killed", "_hit_by", "igni_hit", "bomb_hit", "BookReadState", "merchant_card", "_dd_start", "_eqp_", "_mus", "tut_",
        "spawn", "failsafe", "music", "cutscene", "achievement",
    ];
    if NOISE.iter().any(|x| name.contains(x)) || name.ends_with("_hit") || name.ends_with("_cs") || name.contains("_cs_") {
        return None;
    }
    let letters = name.bytes().take_while(u8::is_ascii_lowercase).count();
    let digit = name.as_bytes().get(letters).filter(|c| c.is_ascii_digit())?;
    let kind = match &name[..letters] {
        "q" => "Main quests",
        "sq" => "Side quests",
        "mq" => "Minor quests",
        "mh" => "Contracts",
        "th" => "Treasure hunts",
        "cg" => "Gwent",
        "ep" => return Some(if *digit == b'1' { "Hearts of Stone".into() } else { "Blood and Wine".into() }),
        _ => return None,
    };
    let area = match digit {
        b'0' => "White Orchard",
        b'1' => "Velen",
        b'2' => "Skellige",
        b'3' => "Novigrad",
        b'4' => "Kaer Morhen",
        b'5' => "the finale",
        b'6' => "Hearts of Stone",
        b'7' => "Blood and Wine",
        _ => return None,
    };
    Some(format!("{kind}: {area}"))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not Witcher 3 game data".into());
    }
    let nt = names(d).ok_or("the name table wasn't found")?;
    let ix = |n: &str| nt.index(n).ok_or_else(|| format!("no \u{201C}{n}\u{201D} in this save"));
    let int32 = ix("Int32")?;
    let pair = |a: u16, b: u16| [a.to_le_bytes(), b.to_le_bytes()].concat();
    let lm = find(&d[..nt.at], &pair(ix("levelManager")?, ix("handle:W3LevelManager")?)).ok_or("the player's level data wasn't found")?;
    let lm_size = u32_at(d, lm + 4).ok_or("bad level data")?;
    let lm_end = (lm + 4 + lm_size).min(nt.at);
    let block = &d[lm..lm_end];
    let mut b = Builder::new(d);
    // level (Int32): name, type, size 8, value.
    let level = pair(ix("level")?, int32);
    if let Some(o) = find(block, &[&level[..], &8u32.to_le_bytes()].concat()) {
        b.ranged(&["Geralt", "Level"], lm + o + 8, Kind::I32, 1, 100);
    }
    // points: an array of {free, used} — skill points, then experience. Fields at their
    // default (0) aren't stored, so each element is read on its own.
    let (free, used) = (ix("free")?, ix("used")?);
    let arr = pair(ix("points")?, ix("array:2,0,SSpendablePoints")?);
    if let Some(o) = find(block, &arr) {
        let start = lm + o;
        let end = (start + 4 + u32_at(d, start + 4).unwrap_or(0)).min(lm_end);
        let count = u32_at(d, start + 8).unwrap_or(0);
        let mut q = start + 12;
        for (n, (label, a, c)) in [("Skill points", "Unspent", "Spent"), ("Experience", "Toward next level", "Total")].into_iter().enumerate() {
            if n >= count || q >= end {
                break;
            }
            q += 1; // each element starts with a 0 byte
            // Properties until a 0 name.
            while q + 2 <= end {
                let name = u32_at(d, q).map(|x| x as u16).unwrap_or(0);
                if name == 0 {
                    q += 2;
                    break;
                }
                let size = u32_at(d, q + 4).unwrap_or(0);
                if size < 4 {
                    q = end;
                    break;
                }
                let ty = (u32_at(d, q).unwrap_or(0) >> 16) as u16;
                if ty == int32 && size == 8 {
                    let which = if name == free { Some(a) } else if name == used { Some(c) } else { None };
                    if let Some(w) = which {
                        b.ranged(&["Geralt", label, w], q + 8, Kind::I32, 0, i32::MAX as i64);
                    }
                }
                q += 4 + size;
            }
        }
    }
    // Crowns: the first stack after the player's data.
    if let (Ok(crowns), Ok(dye1), Ok(dye2)) = (ix("Crowns"), ix("Dye Default"), ix("dye_default")) {
        let dyes = pair(dye1, dye2);
        let mut o = lm_end;
        while let Some(i) = find(&d[o..nt.at], &crowns.to_le_bytes()).map(|i| i + o) {
            if d.get(i + 13..i + 17) == Some(&dyes[..]) {
                b.ranged(&["Geralt", "Crowns"], i + 17, Kind::U32, 0, i32::MAX as u32 as i64);
                break;
            }
            o = i + 2;
            if o > lm_end + (4 << 20) {
                break;
            }
        }
    }
    // Story facts: 0/1 ones as flags, the rest as numbers.
    let mut story = 0;
    for f in facts(&d[..nt.at]).unwrap_or_default() {
        let Some(sec) = story_section(&f.name) else { continue };
        let v = i16::from_le_bytes([d[f.value_at], d[f.value_at + 1]]);
        let added = if v == 0 || v == 1 {
            b.ranged(&["Story flags", &sec, &f.name], f.value_at, Kind::I16, 0, 1)
        } else {
            b.ranged(&["Story numbers", &sec, &f.name], f.value_at, Kind::I16, i16::MIN as i64, i16::MAX as i64)
        };
        story += usize::from(added.is_some());
    }
    let note = if story > 0 {
        "Geralt's crowns, level, and unspent skill points and experience, and the story's facts by quest (the choices made, who lived, \
         who was helped). The game works out the level from the experience, so raise both together. Turning a fact off sets it to 0: \
         scenes that only check whether a fact was ever recorded still see it."
    } else {
        "Geralt's crowns, level, and unspent skill points and experience. The game works out the level from the \
         experience, so raise both together."
    };
    Ok(b.finish("The Witcher 3 game data", Checksum::None).with_note(note))
}

#[cfg(test)]
mod tests {
    #[test]
    fn story_fact_table_reads() {
        let mut d = b"xxSBDF".to_vec();
        d.extend(3u32.to_le_bytes());
        // An ASCII name with one entry (value 1).
        d.push(0x80 | 9);
        d.extend(b"q206_test");
        d.extend([0, 0, 1, 0, 1, 0]);
        d.extend(1.0f32.to_le_bytes());
        d.extend((-1.0f32).to_le_bytes());
        // A UTF-16 name with two entries: skipped.
        d.push(2);
        d.extend([b'a', 0, b'b', 0]);
        d.extend([0, 0, 2, 0]);
        d.extend([0u8; 20]);
        // A 70-character name (two length bytes) with no entries: skipped.
        d.extend([0x80 | 0x40 | (70 & 0x3F), 70 >> 6]);
        d.extend([b'z'; 70]);
        d.extend([0, 0, 0, 0]);
        d.extend(b"EBDF");
        let f = super::facts(&d).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "q206_test");
        assert_eq!(d[f[0].value_at], 1);
        assert_eq!(super::story_section("q206_test").as_deref(), Some("Main quests: Skellige"));
        assert_eq!(super::story_section("mq3035_radovid_dead").as_deref(), Some("Minor quests: Novigrad"));
        assert_eq!(super::story_section("q002_bandit_weapon_hit_by_GERALT"), None);
        assert_eq!(super::story_section("statistic_known_potions"), None);
        // A table that doesn't end where it should isn't used.
        d.truncate(d.len() - 1);
        assert!(super::facts(&d).is_none());
    }

    /// Research aid: SAVE_EDITOR_W3 = unpacked Witcher 3 game data.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_W3").unwrap()).unwrap();
        let t = std::time::Instant::now();
        let doc = super::parse(&d).unwrap();
        println!("{:?} {}", t.elapsed(), serde_json::to_string_pretty(&doc.view).unwrap());
        assert_eq!(doc.to_bytes().unwrap(), d);
    }

    /// Research aid: SAVE_EDITOR_W3_SAV = a `.sav` file. Flips a story fact through the
    /// whole file (LZ4 layer included) and reads it back.
    #[test]
    #[ignore]
    fn real_story_facts() {
        use crate::formats::{decode, encode, Content};
        use serde_json::Value;
        let raw = std::fs::read(std::env::var("SAVE_EDITOR_W3_SAV").unwrap()).unwrap();
        let mut d = decode(&raw);
        let Content::Tree(t) = &mut d.content else { panic!("no tree") };
        let flags = t.view.get("Story flags").and_then(Value::as_object).cloned().unwrap_or_default();
        let n: usize = flags.values().map(|m| m.as_object().map_or(0, |o| o.len())).sum();
        println!("{n} story flags in {} sections", flags.len());
        for (sec, m) in &flags {
            println!("  {sec}: {}", m.as_object().map_or(0, |o| o.len()));
        }
        for k in ["mq3035_radovid_dead", "mq3035_fdb_dijkstra_dead", "q206_cerys_queen", "q208_cerys_won", "mq7023_romance_triss"] {
            let hit = flags.iter().find_map(|(s, m)| m.get(k).map(|v| (s.clone(), v.clone())));
            println!("  {k}: {hit:?}");
        }
        let p = format!("/Story flags/{}/mq3035_radovid_dead", crate::tree::esc("Minor quests: Novigrad"));
        assert_eq!(t.view.pointer(&p), Some(&Value::from(1)));
        t.set(&p, Value::from(0)).unwrap();
        let out = encode(&d.layers, &d.content).unwrap();
        let again = decode(&out);
        let Content::Tree(t2) = &again.content else { panic!("re-read failed") };
        assert_eq!(t2.view.pointer(&p), Some(&Value::from(0)));
        assert_eq!(t2.view.get("Story flags").and_then(Value::as_object).map(|m| m.len()), Some(flags.len()));
    }
}
