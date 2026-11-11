//! Final Fantasy VII Remake (Intergrade) and Rebirth slot saves (`ff7remake000.sav`,
//! `ff7remakeplus000.sav`, `ff7rebirth000.sav` …). Neither is encrypted or checksummed.
//! After a 16-byte header and the "RESD" marker comes the party: a count, then one record
//! per character (0x40 bytes in Remake, 0x100 in Rebirth) starting with the level, with
//! current/max HP, current/max MP and experience at +0x10.
//!
//! Worked out from the player's own saves (current HP below max where a slot was saved
//! hurt, levels matching the game's caps); only these values are offered.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const MARK: &[u8] = b"RESDRESDRESDRESD";
const FIRST: usize = 0x30;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The record size that makes every used record look like a character, if one does.
fn layout(d: &[u8]) -> Option<(usize, usize)> {
    if d.len() < 0x200 || d[..4] != [0, 0, 1, 0] || &d[0x10..0x20] != MARK {
        return None;
    }
    let n = u32_at(d, 0x20)? as usize;
    if !(1..=64).contains(&n) {
        return None;
    }
    [0x40usize, 0x100]
        .into_iter()
        .find(|&stride| {
            let mut used = 0;
            for k in 0..n {
                let r = FIRST + k * stride;
                let (Some(hp), Some(max), Some(mp), Some(mmp)) = (
                    u32_at(d, r + 0x10),
                    u32_at(d, r + 0x14),
                    u32_at(d, r + 0x18),
                    u32_at(d, r + 0x1C),
                ) else {
                    return false;
                };
                if max == 0 {
                    continue; // not joined yet
                }
                // Current values above the maximum are allowed (an edit, or a temporary boost).
                if !(1..=99).contains(&d[r])
                    || max > 99_999
                    || hp > 99_999
                    || mp > 99_999
                    || mmp > 99_999
                {
                    return false;
                }
                used += 1;
            }
            used > 0
        })
        .map(|s| (s, n))
}

pub fn looks(d: &[u8]) -> bool {
    layout(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (stride, n) = layout(d).ok_or("not a Final Fantasy VII Remake/Rebirth slot")?;
    let rebirth = stride == 0x100;
    let mut b = Builder::new(d);
    let game = if rebirth {
        "Final Fantasy VII Rebirth"
    } else {
        "Final Fantasy VII Remake"
    };
    b.info(&["Game"], game);
    let cap = if rebirth { 70 } else { 50 };
    for k in 0..n {
        let r = FIRST + k * stride;
        if u32_at(d, r + 0x14).unwrap_or(0) == 0 {
            continue;
        }
        let who = format!("Character {}", k + 1);
        let w = who.as_str();
        // Levels above the game's cap are kept as they are (they can come from other editors).
        b.ranged(
            &["Party", w, "Level"],
            r,
            Kind::U8,
            1,
            i64::from(d[r].max(cap)),
        );
        b.ranged(&["Party", w, "HP"], r + 0x10, Kind::U32, 0, 99_999);
        b.ranged(&["Party", w, "Max HP"], r + 0x14, Kind::U32, 1, 99_999);
        b.ranged(&["Party", w, "MP"], r + 0x18, Kind::U32, 0, 99_999);
        b.ranged(&["Party", w, "Max MP"], r + 0x1C, Kind::U32, 0, 99_999);
        b.ranged(
            &["Party", w, "Experience"],
            r + 0x20,
            Kind::U32,
            0,
            99_999_999,
        );
    }
    Ok(b.finish(&format!("{game} save"), Checksum::None).with_note(
        "Party levels, HP, MP and experience. The game may recalculate max HP and MP from level and equipment when it loads; \
         for gil and items use Value search with an amount you know.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_slot() {
        let mut d = vec![0u8; 0x400];
        d[..4].copy_from_slice(&[0, 0, 1, 0]);
        d[0x10..0x20].copy_from_slice(MARK);
        d[0x20] = 2;
        for (k, (lv, hp, max)) in [(40u8, 900u32, 1000u32), (12, 300, 300)]
            .into_iter()
            .enumerate()
        {
            let r = FIRST + k * 0x40;
            d[r] = lv;
            d[r + 0x10..r + 0x14].copy_from_slice(&hp.to_le_bytes());
            d[r + 0x14..r + 0x18].copy_from_slice(&max.to_le_bytes());
        }
        let mut doc = parse(&d).unwrap();
        assert_eq!(
            doc.view.pointer("/Game").unwrap(),
            "Final Fantasy VII Remake"
        );
        assert_eq!(doc.view.pointer("/Party/Character 1/HP").unwrap(), 900);
        assert!(doc
            .set("/Party/Character 1/Level", serde_json::Value::from(51))
            .is_err());
        doc.set(
            "/Party/Character 2/Experience",
            serde_json::Value::from(12345),
        )
        .unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, FIRST + 0x40 + 0x20), Some(12345));
    }

    /// Research aid: SAVE_EDITOR_FF7R_DIR = a folder of Remake/Rebirth saves (read only).
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("SAVE_EDITOR_FF7R_DIR").unwrap();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            match parse(&d) {
                Ok(doc) => {
                    assert_eq!(doc.to_bytes().unwrap(), d);
                    let lv: Vec<String> = doc.view["Party"]
                        .as_object()
                        .unwrap()
                        .values()
                        .map(|c| c["Level"].to_string())
                        .collect();
                    println!("{:?}: {} levels {:?}", e.file_name(), doc.kind_name, lv);
                }
                Err(err) => println!("{:?}: {err}", e.file_name()),
            }
        }
    }
}
