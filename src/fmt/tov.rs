//! Tales of Vesperia: Definitive Edition (PC): `TLSaveData0_NN`, 838,872 bytes. A 0x228-byte menu header (what the
//! save list shows), then a `TO8SAVE` file of named sections (`PARTY_DATA`, `PC_STATUS1` … `PC_STATUS9`, …).
//!
//! The section table and the field positions follow AdmiralCurtiss/HyoutaTools (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Tales of Vesperia save";
const SIZE: usize = 838_872;
const BASE: usize = 0x228;
const NAMES: [&str; 9] = ["Yuri", "Estelle", "Karol", "Rita", "Raven", "Judith", "Repede", "Flynn", "Patty"];

fn le(d: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?) as usize)
}

/// (name, offset in the file, size) of every section.
fn sections(d: &[u8]) -> Option<Vec<(String, usize, usize)>> {
    if d.len() != SIZE || &d[BASE..BASE + 8] != b"TO8SAVE\0" {
        return None;
    }
    let (meta, count, data, strs) = (le(d, BASE + 0x20)?, le(d, BASE + 0x24)?, le(d, BASE + 0x28)?, le(d, BASE + 0x2c)?);
    let mut v = vec![];
    for i in 0..count.min(64) {
        let h = BASE + meta + i * 0x20;
        let (name_at, off, size) = (BASE + strs + le(d, h)?, BASE + data + le(d, h + 4)?, le(d, h + 8)?);
        let end = d.get(name_at..)?.iter().position(|&c| c == 0)? + name_at;
        v.push((String::from_utf8_lossy(&d[name_at..end]).into_owned(), off, size));
    }
    v.iter().all(|s| s.1 + s.2 <= d.len()).then_some(v)
}

pub fn looks(d: &[u8]) -> bool {
    sections(d).is_some_and(|s| s.iter().any(|x| x.0 == "PARTY_DATA" && x.2 >= 0x34))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let secs = sections(d).ok_or("not a Tales of Vesperia save")?;
    let party = secs.iter().find(|s| s.0 == "PARTY_DATA").ok_or("no party data")?.1;
    let mut b = Builder::new(d);
    let g = "Party";
    b.ranged(&[g, "Gald"], party + 0x30, Kind::U32, 0, 999_999_999);
    // The save list shows the same number.
    b.mirror(&[g, "Gald"], 0x20);
    b.info(&[g, "Play time (hours)"], format!("{:.1}", f64::from(u32::from_le_bytes(d[party + 0x2c..party + 0x30].try_into().unwrap())) / 216_000.0));
    for (name, at, size) in &secs {
        let Some(n) = name.strip_prefix("PC_STATUS").and_then(|n| n.parse::<usize>().ok()) else { continue };
        if !(1..=9).contains(&n) || *size < 0x90 {
            continue;
        }
        let who = NAMES[n - 1];
        for (label, off, hi) in [
            ("Level", 0x48, 200),
            ("HP", 0x4c, 99_999),
            ("TP", 0x50, 9_999),
            ("Max HP", 0x54, 99_999),
            ("Max TP", 0x58, 9_999),
            ("Experience", 0x60, 999_999_999),
            ("Attack", 0x6c, 99_999),
            ("Magic attack", 0x70, 99_999),
            ("Defense", 0x74, 99_999),
            ("Magic defense", 0x78, 99_999),
            ("Agility", 0x80, 99_999),
            ("Luck", 0x84, 99_999),
        ] {
            b.ranged(&["Characters", who, label], at + off, Kind::U32, 0, hi);
        }
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_round_trips_and_edits() {
        let Ok(dir) = std::env::var("TOV_SAMPLES") else { return };
        for f in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(f.path()).unwrap();
            assert!(looks(&d));
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d);
            doc.set("/Party/Gald", 5_000_000.into()).unwrap();
            doc.set("/Characters/Yuri/Level", 99.into()).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out).unwrap();
            assert_eq!(again.view.pointer("/Party/Gald"), Some(&serde_json::Value::from(5_000_000)));
            assert_eq!(u32::from_le_bytes(out[0x20..0x24].try_into().unwrap()), 5_000_000);
            assert_eq!(again.view.pointer("/Characters/Yuri/Level"), Some(&serde_json::Value::from(99)));
        }
    }
}
