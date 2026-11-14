//! The Bouncer (PlayStation 2) saves: the game's save file holds 16 slots of 0x400 bytes. A used
//! slot starts with the text "DRF/SQUARE" and ends its first 0x1e0 bytes with a CRC-32 kept at
//! offset 0x2c (the four bytes of the CRC itself are left out of the sum). Offsets and names
//! follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection (see
//! THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::ps1::opts;
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "The Bouncer (PlayStation 2) save";

const SLOT: usize = 0x400;
const SLOTS: usize = 16;

fn table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for (i, e) in t.iter_mut().enumerate() {
        let mut c = (i as u32) << 24;
        for _ in 0..8 {
            c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04c1_1db7 } else { c << 1 };
        }
        *e = c;
    }
    t
}

fn crc(slot: &[u8]) -> u32 {
    let t = table();
    let mut c = 0xffff_ffffu32;
    for (i, b) in slot[..0x1e0].iter().enumerate() {
        if (0x2c..0x30).contains(&i) {
            continue;
        }
        c = (c << 8) ^ t[((*b as u32 ^ (c >> 24)) & 0xff) as usize];
    }
    !c
}

fn active(s: &[u8]) -> bool {
    s[0x20] != 0 && s.starts_with(b"DRF/SQUARE")
}

fn valid(s: &[u8]) -> bool {
    active(s) && u32::from_le_bytes([s[0x2c], s[0x2d], s[0x2e], s[0x2f]]) == crc(s)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SLOT * SLOTS && d.chunks(SLOT).any(valid)
}

const CHARACTERS: [&str; 3] = ["Sion Barzahd", "Volt Krueger", "Kou Leifoh"];
const SKILLS: [[&str; 8]; 3] = [
    ["Buster Throw", "Torpedo Kick", "Ground Sweep", "Floating Mine", "Tornado Uppercut", "Double Knuckle", "Hurricane Blitz", ""],
    ["Soulder Uppercut", "Hammer Typhoon", "Lift Up Slam", "Power Bridge", "Cannonball Strike", "Earthshaker", "Giant Swing", ""],
    ["Heel Smash", "Circular Uppercut", "Double Spin Kick", "Mountain Storm", "Lightning Smash", "Tiger Spin Kick", "Tiger Frenzy", "Raging Tiger"],
];
const STAGES: [(&str, usize, u32); 11] = [
    ("Bar - \"Fate\"", 0x100, 4),
    ("Central Square", 0x100, 5),
    ("Central Station", 0x100, 6),
    ("MSD Cargo Train", 0x100, 7),
    ("Air-Carrier", 0x101, 0),
    ("Mikado Bldg. - Hanging Garden", 0x101, 1),
    ("Crystal Dome", 0x101, 2),
    ("Mikado Bldg. - Robot Factory", 0x101, 3),
    ("Rocket Tower - Deck", 0x101, 4),
    ("Air-Carrier (over the Galeos)", 0x101, 5),
    ("Galeos - Control Room", 0x101, 6),
];
const UNLOCK_CHARS: [(&str, usize, u32); 12] = [
    ("Echidna", 0x101, 7),
    ("Dauragon (One-handed)", 0x102, 0),
    ("Dauragon (Two-handed)", 0x102, 1),
    ("Dauragon (Shirtless)", 0x102, 2),
    ("Dauragon (Tattoo)", 0x102, 3),
    ("Mugetsu", 0x102, 4),
    ("Mugetsu (Unmasked)", 0x102, 5),
    ("PD-4", 0x102, 6),
    ("Dominique", 0x102, 7),
    ("Kaldea", 0x103, 0),
    ("Wong", 0x103, 1),
    ("Leann", 0x103, 2),
];
const COSTUMES: [(&str, usize, u32); 2] = [("Black-Hooded Sion", 0x103, 3), ("MSF Kou", 0x103, 4)];
const FIGHTERS: [&str; 15] = [
    "Sion", "Volt", "Kou", "Echidna", "Dauragon (One-handed)", "Dauragon (Two-handed)", "Dauragon (Shirtless)", "Dauragon (Tattoo)", "Mugetsu", "Mugetsu (Unmasked)", "PD-4", "Dominique", "Kaldea", "Wong", "Leann",
];
const RANKS: [&str; 8] = ["G", "F", "E", "D", "C", "B", "A", "S"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a The Bouncer save".into());
    }
    let mut b = Builder::new(d);
    let fighters = opts(&FIGHTERS.iter().enumerate().map(|(i, n)| (*n, i as u64)).collect::<Vec<_>>());
    let ranks = opts(&RANKS.iter().enumerate().map(|(i, n)| (*n, i as u64)).collect::<Vec<_>>());
    for s in 0..SLOTS {
        let slot = &d[s * SLOT..(s + 1) * SLOT];
        if !valid(slot) {
            continue;
        }
        let o = s * SLOT;
        let name = format!("Slot {}", s + 1);
        b.ranged(&["Progress", &name, "Clear count"], o + 0x84, Kind::F32Int, 0, 99);
        for (c, n) in CHARACTERS.iter().enumerate() {
            b.ranged(&["Bouncer Points", &name, n], o + 0x148 + c * 8, Kind::U32, 0, 99_999);
            b.bits(&[n, &name, "Life Up"], o + 0x120 + c * 2, Kind::U8, 0, 4, 0, 0, 7);
            b.bits(&[n, &name, "Power Up"], o + 0x120 + c * 2, Kind::U8, 4, 4, 0, 0, 7);
            b.bits(&[n, &name, "Defence Up"], o + 0x121 + c * 2, Kind::U8, 0, 4, 0, 0, 7);
            b.pick_bits(&[n, &name, "Rank"], o + 0x121 + c * 2, Kind::U8, 4, 4, &ranks);
            for (bit, sk) in SKILLS[c].iter().enumerate() {
                if !sk.is_empty() {
                    flag(&mut b, &[n, &name, sk], o + 0x126 + c * 2, bit as u32);
                }
            }
        }
        for (n, at, bit) in STAGES {
            flag(&mut b, &["Unlocked stages", &name, n], o + at, bit);
        }
        for (n, at, bit) in UNLOCK_CHARS {
            flag(&mut b, &["Unlocked characters", &name, n], o + at, bit);
        }
        for (n, at, bit) in COSTUMES {
            flag(&mut b, &["Unlocked costumes", &name, n], o + at, bit);
        }
        for p in 0..10 {
            let at = o + 0x160 + p * 0xc;
            let time = u32::from_le_bytes([d[at + 8], d[at + 9], d[at + 10], d[at + 11]]);
            if time == 0 {
                continue;
            }
            let label = format!("{name}, place {}", p + 1);
            b.pick(&["Survival mode", &label, "Character"], at, Kind::U8, &fighters);
            b.pick(&["Survival mode", &label, "Rank"], at + 1, Kind::U8, &ranks);
            b.ranged(&["Survival mode", &label, "KOs"], at + 2, Kind::U8, 0, 50);
        }
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note(crate::i18n::tx("Close the emulator first. The save file holds 16 slots; only the used ones are listed. The game ranks each fighter from the Bouncer Points spent on stats and skills, so the rank and the points spent are written again when you change a stat or a skill; each changed slot's checksum is written again too.")))
}

const STAT_COST: [[u32; 7]; 3] = [[200, 300, 400, 500, 700, 1000, 1500], [160, 240, 320, 400, 560, 800, 1200], [220, 330, 440, 550, 770, 1100, 1650]];
const SKILL_COST: [[u32; 8]; 3] = [[400, 600, 1000, 1200, 1600, 2000, 2800, 0], [960, 1000, 1200, 1500, 1800, 2400, 3500, 0], [320, 400, 550, 750, 1000, 1200, 1500, 2500]];
const RANK_FROM: [u32; 8] = [0, 1000, 2500, 4700, 7400, 11100, 16100, 23400];

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    let mut slots: Vec<usize> = touched.iter().map(|t| t / SLOT).filter(|s| *s < SLOTS).collect();
    slots.sort_unstable();
    slots.dedup();
    for s in slots {
        let o = s * SLOT;
        if !active(&d[o..o + SLOT]) {
            continue;
        }
        for c in 0..3 {
            let hit = touched.iter().any(|t| *t >= o && *t < o + SLOT && matches!(*t - o, x if x == 0x120 + c * 2 || x == 0x121 + c * 2 || x == 0x126 + c * 2));
            if !hit {
                continue;
            }
            let (a, e) = (d[o + 0x120 + c * 2], d[o + 0x121 + c * 2]);
            let sk = d[o + 0x126 + c * 2];
            let mut points = 0u32;
            for v in [(a & 15) as usize, (a >> 4) as usize, (e & 15) as usize] {
                points += STAT_COST[c][..v.min(7)].iter().sum::<u32>();
            }
            for bit in 0..8 {
                if sk >> bit & 1 == 1 {
                    points += SKILL_COST[c][bit];
                }
            }
            let rank = RANK_FROM.iter().rposition(|m| points >= *m).unwrap_or(0) as u8;
            d[o + 0x121 + c * 2] = (e & 15) | rank << 4;
            d[o + 0x14c + c * 8..o + 0x150 + c * 8].copy_from_slice(&points.to_le_bytes());
        }
        let c = crc(&d[o..o + SLOT]);
        d[o + 0x2c..o + 0x30].copy_from_slice(&c.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; SLOT * SLOTS];
        d[..10].copy_from_slice(b"DRF/SQUARE");
        d[0x20] = 65;
        let c = crc(&d[..SLOT]);
        d[0x2c..0x30].copy_from_slice(&c.to_le_bytes());
        d
    }

    #[test]
    fn detects_and_rewrites_rank_and_checksum() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert!(doc.set("/Sion Barzahd/Slot 1/Life Up", serde_json::json!(3)).is_ok());
        assert!(doc.set("/Sion Barzahd/Slot 1/Buster Throw", serde_json::json!("Yes")).is_ok());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0x120] & 15, 3);
        assert_eq!(out[0x126] & 1, 1);
        // 200 + 300 + 400 stats, 400 for the skill: 1300 points, rank F.
        assert_eq!(u32::from_le_bytes([out[0x14c], out[0x14d], out[0x14e], out[0x14f]]), 1300);
        assert_eq!(out[0x121] >> 4, 1);
        assert!(valid(&out[..SLOT]));
    }
}
