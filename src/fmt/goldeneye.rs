//! GoldenEye 007 (Nintendo 64) EEPROM saves (`.eep`, 2048 bytes, or an emulator's combined
//! `.srm`): five 0x60-byte records, four of which are the player slots (the low three bits of
//! the byte at 0x28 name the slot, bit 7 marks a spare) and each ends in a Rare 64-bit sum.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::rare;
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "GoldenEye 007 save";
const REC: usize = 0x60;
const MISSIONS: [(&str, usize, u32); 20] = [
    ("Dam", 0x32, 6), ("Facility", 0x33, 4), ("Runaway", 0x34, 2), ("Surface 1", 0x35, 0), ("Bunker 1", 0x37, 6), ("Silo", 0x38, 4),
    ("Frigate", 0x39, 2), ("Surface 2", 0x3a, 0), ("Bunker 2", 0x3c, 6), ("Statue", 0x3d, 4), ("Archives", 0x3e, 2), ("Streets", 0x3f, 0),
    ("Depot", 0x41, 6), ("Train", 0x42, 4), ("Jungle", 0x43, 2), ("Control", 0x44, 0), ("Caverns", 0x46, 6), ("Cradle", 0x47, 4),
    ("Aztec", 0x48, 2), ("Egyptian", 0x49, 0),
];
const DIFFS: [&str; 3] = ["Agent", "Secret Agent", "00 Agent"];
const STYLES: [&str; 8] = ["1.1 Honey", "1.2 Solitaire", "1.3 Kissy", "1.4 Goodnight", "2.1 Plenty", "2.2 Galore", "2.3 Domino", "2.4 Goodhead"];
const CHEATS: [(&str, usize, u32); 20] = [
    ("Invincible", 0x2e, 1), ("All Guns", 0x30, 3), ("Bond Invisible", 0x2f, 2), ("Infinite Ammo", 0x2f, 7), ("DK Mode", 0x2e, 2),
    ("Tiny Bond", 0x2e, 7), ("Paintball Mode", 0x2e, 0), ("Silver PP7", 0x2f, 5), ("Gold PP7", 0x30, 1), ("No Radar (multiplayer)", 0x2e, 6),
    ("Turbo Mode", 0x2e, 5), ("Fast Animation", 0x2f, 1), ("Slow Animation", 0x2f, 4), ("Enemy Rockets", 0x2f, 3), ("2x Rocket Launcher", 0x2e, 4),
    ("2x Grenade Launcher", 0x2e, 3), ("2x RC-P90", 0x30, 0), ("2x Throwing Knife", 0x2f, 0), ("2x Hunting Knife", 0x2f, 6), ("2x Laser", 0x30, 2),
];

fn sum_ok(d: &[u8], s: usize) -> bool {
    s + 0x80 <= d.len() && d[s + 0x20..s + 0x28] == rare::stored(&d[s + 0x28..s + 0x80])
}

pub fn looks(d: &[u8]) -> bool {
    matches!(d.len(), 2048 | 296_960) && (0..5).any(|i| sum_ok(d, i * REC))
}

/// The record that holds player slot `slot`.
fn record(d: &[u8], slot: usize) -> Option<usize> {
    (0..5).map(|i| i * REC).find(|&s| (d[s + 0x28] & 7) as usize == slot && d[s + 0x28] & 0x80 == 0)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a GoldenEye 007 save".into());
    }
    let mut b = Builder::new(d);
    b.big_endian = true;
    let mut used = 0;
    for slot in 0..4 {
        let Some(s) = record(d, slot) else { continue };
        let at = |o: usize| d[s + o];
        if [0x32, 0x4b, 0x64].iter().all(|&o| u16::from_be_bytes([at(o), at(o + 1)]) == 0) {
            continue;
        }
        used += 1;
        let name = format!("Slot {}", slot + 1);
        let n = name.as_str();
        for (m, off, bit) in MISSIONS {
            for (df, diff) in DIFFS.iter().enumerate() {
                b.bits(&[n, "Missions", m, diff], s + off + df * 0x19, Kind::U16, bit, 10, 0, 0, 1023);
            }
        }
        flag(&mut b, &[n, "Cheats", "All levels and 007 difficulty unlocked"], s + 0x29, 0);
        for (c, off, bit) in CHEATS {
            flag(&mut b, &[n, "Cheats", c], s + off, bit);
        }
        let styles: Vec<(String, u64)> = STYLES.iter().enumerate().map(|(i, x)| (x.to_string(), i as u64)).collect();
        b.pick_bits(&[n, "Options", "Control style"], s + 0x2c, Kind::U8, 0, 3, &styles);
        b.ranged(&[n, "Options", "Music"], s + 0x2a, Kind::U8, 0, 255);
        b.ranged(&[n, "Options", "Effects"], s + 0x2b, Kind::U8, 0, 255);
        let rev = [("Reverse".to_string(), 0), ("Upright".to_string(), 1)];
        b.pick_bits(&[n, "Options", "Look up/down"], s + 0x2d, Kind::U8, 0, 1, &rev);
        for (label, bit) in [("Auto-aim", 1), ("Sight on-screen", 3), ("Look ahead", 4), ("Ammo on-screen", 5)] {
            b.pick_bits(&[n, "Options", label], s + 0x2d, Kind::U8, bit, 1, &[("Off".to_string(), 0), ("On".to_string(), 1)]);
        }
        b.pick_bits(&[n, "Options", "Aim control"], s + 0x2d, Kind::U8, 2, 1, &[("Hold".to_string(), 0), ("Toggle".to_string(), 1)]);
        b.pick_bits(&[n, "Options", "Screen"], s + 0x2d, Kind::U8, 6, 1, &[("Full".to_string(), 0), ("Wide".to_string(), 1)]);
    }
    b.info(&["Info", "Slots in use"], used);
    if used == 0 {
        return Err("no GoldenEye 007 slot has a mission time yet".into());
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note("Close the emulator first. Times are in seconds; 0 means the mission is not done and 1023 means it was skipped."))
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    let mut recs: Vec<usize> = touched.iter().map(|t| t / REC * REC).collect();
    recs.sort_unstable();
    recs.dedup();
    for s in recs {
        let v = rare::stored(&d[s + 0x28..s + 0x80]);
        d[s + 0x20..s + 0x28].copy_from_slice(&v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 2048];
        for i in 0..5 {
            let s = i * REC;
            d[s + 0x28] = if i < 4 { i as u8 | 0x20 } else { 0x80 };
            d[s + 0x29..s + 0x2b].copy_from_slice(&[0, 0xff]);
            fix(&mut d, &[s]);
        }
        d
    }

    #[test]
    fn edits_a_slot_and_its_sum() {
        let mut d = sample();
        assert!(looks(&d));
        assert!(parse(&d).is_err());
        // A Dam time of 85 s on slot 2.
        let s = REC;
        d[s + 0x32..s + 0x34].copy_from_slice(&(85u16 << 6).to_be_bytes());
        fix(&mut d, &[s]);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Slot 2/Missions/Dam/Agent").unwrap(), 85);
        doc.set("/Slot 2/Missions/Facility/Agent", 100.into()).unwrap();
        doc.set("/Slot 2/Cheats/Invincible", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u16::from_be_bytes([out[s + 0x33], out[s + 0x34]]) >> 4 & 0x3ff, 100);
        assert_eq!(out[s + 0x2e] >> 1 & 1, 1);
        assert!(sum_ok(&out, s));
        assert_eq!(u16::from_be_bytes([out[s + 0x32], out[s + 0x33]]) >> 6, 85);
    }
}
