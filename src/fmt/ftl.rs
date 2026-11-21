//! FTL: Faster Than Light saved game (`continue.sav`, file format 11, the current Steam version):
//! plain little-endian 32-bit values and length-prefixed strings, no checksum.
//!
//! The layout follows the public descriptions of the file by Vhati's FTL Profile Editor and
//! enoy19's ftl-editor (both GPL-3.0; the code here is new, only the layout is shared). Only the
//! part up to the crew list is read: that holds the run's counters and the ship's hull, fuel,
//! missiles, drone parts and scrap, and each crew member's health and records.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "FTL saved game";
const FORMAT: i32 = 11;

struct R<'a> {
    d: &'a [u8],
    at: usize,
}

impl R<'_> {
    fn i32(&mut self) -> Option<i32> {
        let v = i32::from_le_bytes(self.d.get(self.at..self.at + 4)?.try_into().ok()?);
        self.at += 4;
        Some(v)
    }
    /// A 32-bit true/false (0 or 1 only).
    fn flag(&mut self) -> Option<bool> {
        match self.i32()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    /// A count that must be small enough to be believable.
    fn count(&mut self, max: i32) -> Option<usize> {
        let n = self.i32()?;
        (0..=max).contains(&n).then_some(n as usize)
    }
    fn text(&mut self) -> Option<String> {
        let n = self.i32()?;
        if !(0..=256).contains(&n) {
            return None;
        }
        let s = self.d.get(self.at..self.at + n as usize)?;
        self.at += n as usize;
        String::from_utf8(s.to_vec()).ok()
    }
    fn skip(&mut self, ints: usize) -> Option<()> {
        self.at = self.at.checked_add(ints * 4).filter(|e| *e <= self.d.len())?;
        Some(())
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 200 && d[..4] == FORMAT.to_le_bytes() && parse(d).is_ok()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    parse_inner(d).ok_or_else(|| "not an FTL saved game (format 11)".to_string())
}

fn parse_inner(d: &[u8]) -> Option<TreeDoc> {
    let mut r = R { d, at: 0 };
    if r.i32()? != FORMAT {
        return None;
    }
    r.flag()?; // random native
    r.flag()?; // advanced edition content
    let diff_at = r.at;
    let difficulty = r.i32()?;
    if !(0..=2).contains(&difficulty) {
        return None;
    }
    let (defeated, explored, collected, hired) = (r.at, r.at + 4, r.at + 8, r.at + 12);
    r.skip(4)?;
    let _ship_name = r.text()?;
    let blueprint = r.text()?;
    if blueprint.is_empty() {
        return None;
    }
    let sector = r.i32()?;
    if !(1..=9).contains(&sector) {
        return None;
    }
    r.i32()?; // victory
    for _ in 0..r.count(4096)? {
        r.text()?;
        r.i32()?;
    }
    // The ship: blueprint, name, graphics base, starting crew, jump state, then the stores.
    let ship_blueprint = r.text()?;
    let ship_name = r.text()?;
    r.text()?;
    for _ in 0..r.count(16)? {
        r.text()?;
        r.text()?;
    }
    r.skip(4)?;
    let (hull, fuel, drones, missiles, scrap) = (r.at, r.at + 4, r.at + 8, r.at + 12, r.at + 16);
    r.skip(5)?;
    let crew_count = r.count(16)?;
    let mut crew = Vec::new();
    for n in 0..crew_count {
        let name = r.text()?;
        let race = r.text()?;
        r.flag()?; // enemy boarding drone
        let health = r.at;
        r.skip(5)?; // health, position, room, square
        r.flag()?; // player controlled
        r.skip(2)?; // clone ready, death order
        let tints = r.count(64)?;
        r.skip(tints + 3)?; // tints, mind controlled, saved square, saved room
        r.skip(6)?; // six skill values
        r.flag()?; // male
        let records = r.at;
        r.skip(5)?; // repairs, kills, evasions, jumps, masteries
        r.skip(6 + 12)?; // status counters, masteries
        r.flag()?; // teleporting
        r.skip(7)?; // teleport animation
        r.flag()?; // teleport animation reversed
        if race == "crystal" {
            r.skip(3)?;
        }
        crew.push((n, name, race, health, records));
    }

    let mut b = Builder::new(d);
    b.named(&["Run", "Difficulty"], diff_at, Kind::I32, &["Easy", "Normal", "Hard"])?;
    b.info(&["Run", "Sector"], sector);
    b.ranged(&["Run", "Ships defeated"], defeated, Kind::I32, 0, 99_999)?;
    b.ranged(&["Run", "Beacons explored"], explored, Kind::I32, 0, 99_999)?;
    b.ranged(&["Run", "Scrap collected"], collected, Kind::I32, 0, 9_999_999)?;
    b.ranged(&["Run", "Crew hired"], hired, Kind::I32, 0, 99_999)?;
    b.info(&["Ship", "Ship"], format!("{ship_name} ({ship_blueprint})"));
    b.ranged(&["Ship", "Scrap"], scrap, Kind::I32, 0, 99_999)?;
    b.ranged(&["Ship", "Fuel"], fuel, Kind::I32, 0, 999)?;
    b.ranged(&["Ship", "Missiles"], missiles, Kind::I32, 0, 999)?;
    b.ranged(&["Ship", "Drone parts"], drones, Kind::I32, 0, 999)?;
    b.ranged(&["Ship", "Hull"], hull, Kind::I32, 1, 30)?;
    for (n, name, race, health, records) in crew {
        let who = format!("{} - {name} ({race})", n + 1);
        b.ranged(&["Crew", &who, "Health"], health, Kind::I32, 1, 1000)?;
        b.ranged(&["Crew", &who, "Repairs"], records, Kind::I32, 0, 99_999)?;
        b.ranged(&["Crew", &who, "Combat kills"], records + 4, Kind::I32, 0, 99_999)?;
        b.ranged(&["Crew", &who, "Piloted evasions"], records + 8, Kind::I32, 0, 99_999)?;
        b.ranged(&["Crew", &who, "Jumps survived"], records + 12, Kind::I32, 0, 99_999)?;
    }
    Some(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn int(o: &mut Vec<u8>, v: i32) {
        o.extend(v.to_le_bytes());
    }
    fn text(o: &mut Vec<u8>, s: &str) {
        int(o, s.len() as i32);
        o.extend(s.as_bytes());
    }

    /// A made-up save with one crew member.
    pub(crate) fn synthetic() -> Vec<u8> {
        let mut o = vec![];
        for v in [FORMAT, 0, 1, 1, 12, 3, 140, 4] {
            int(&mut o, v);
        }
        text(&mut o, "The Test Ship");
        text(&mut o, "PLAYER_SHIP_TEST");
        int(&mut o, 2); // sector
        int(&mut o, 0); // victory
        int(&mut o, 1);
        text(&mut o, "fired_shot");
        int(&mut o, 8);
        text(&mut o, "PLAYER_SHIP_TEST");
        text(&mut o, "The Test Ship");
        text(&mut o, "test");
        int(&mut o, 1);
        text(&mut o, "human");
        text(&mut o, "Ann");
        for v in [1, 1130, 0, 0] {
            int(&mut o, v);
        }
        for v in [25, 11, 4, 7, 60] {
            int(&mut o, v);
        }
        int(&mut o, 1); // one crew member
        text(&mut o, "Ann");
        text(&mut o, "human");
        int(&mut o, 0); // enemy boarding drone
        for v in [100, 5, 6, 0, 0] {
            int(&mut o, v); // health, position, room, square
        }
        int(&mut o, 1); // player controlled
        for v in [0, 0, 0, 0, 0, 0] {
            int(&mut o, v); // clone ready, death order, no tints, mind controlled, saved square, saved room
        }
        for v in [3, 4, 5, 6, 7, 8] {
            int(&mut o, v); // six skills
        }
        int(&mut o, 0); // male
        for v in [9, 10, 11, 12, 13] {
            int(&mut o, v); // repairs, kills, evasions, jumps, masteries
        }
        for _ in 0..18 {
            int(&mut o, 0); // status counters and masteries
        }
        int(&mut o, 0); // teleporting
        for _ in 0..7 {
            int(&mut o, 0); // animation
        }
        int(&mut o, 0); // reversed
        o.extend([0u8; 64]); // the rest of the file
        o
    }

    #[test]
    fn reads_and_edits_a_save() {
        let d = synthetic();
        assert!(looks(&d));
        let mut t = parse(&d).unwrap();
        assert_eq!(t.view.pointer("/Ship/Scrap").unwrap(), 60);
        assert_eq!(t.view.pointer("/Ship/Hull").unwrap(), 25);
        assert_eq!(t.view.pointer("/Run/Difficulty").unwrap(), "Normal");
        assert_eq!(t.view.pointer("/Crew/1 - Ann (human)/Combat kills").unwrap(), 10);
        assert_eq!(t.to_bytes().unwrap(), d);
        t.set("/Ship/Scrap", serde_json::json!(9999)).unwrap();
        t.set("/Crew/1 - Ann (human)/Health", serde_json::json!(150)).unwrap();
        assert!(t.set("/Ship/Hull", serde_json::json!(31)).is_err());
        let out = t.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Ship/Scrap").unwrap(), 9999);
        assert_eq!(back.view.pointer("/Crew/1 - Ann (human)/Health").unwrap(), 150);
    }

    /// A real save: `FTL_SAVE=path cargo test --release --lib ftl_real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn ftl_real_save() {
        let d = std::fs::read(std::env::var("FTL_SAVE").unwrap()).unwrap();
        let t = parse(&d).unwrap();
        println!("{}", serde_json::to_string_pretty(&t.view).unwrap());
        assert_eq!(t.to_bytes().unwrap(), d);
    }

    #[test]
    fn rejects_other_files() {
        assert!(!looks(&[0u8; 400]));
        assert!(parse(&[]).is_err());
    }
}
