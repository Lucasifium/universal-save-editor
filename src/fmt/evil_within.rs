//! The Evil Within `inventory.zwei`: the first four bytes are a checksum (big-endian) of the rest
//! of the file, an MD5 whose sixteen bytes are folded to four. Weapon upgrades, green gel and
//! trap parts are found by their names inside the file; the values after them are big-endian.
//!
//! Layout and checksum follow The-Evil-Within-Checksum-Scripts (xymox12, MIT).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use md5::{Digest, Md5};

pub const KIND: &str = "The Evil Within inventory";

fn fold(d: &[u8]) -> u32 {
    let h = Md5::digest(d);
    let be = |a: usize| u32::from_be_bytes([h[a], h[(a + 1) % 16], h[(a + 2) % 16], h[(a + 3) % 16]]);
    let f = (be(10) ^ be(14)) ^ (be(2) ^ be(6));
    (((f >> 16) & 0xFF) << 24) | (((f >> 24) & 0xFF) << 16) | ((f & 0xFF) << 8) | ((f >> 8) & 0xFF)
}

fn fix(d: &mut Vec<u8>) {
    let sum = fold(&d[4..]);
    d[..4].copy_from_slice(&sum.to_be_bytes());
}

fn find(d: &[u8], needle: &[u8]) -> Option<usize> {
    d.windows(needle.len()).position(|w| w == needle)
}

/// End of the length-prefixed name that starts with `name`.
fn name_end(d: &[u8], name: &[u8]) -> Option<usize> {
    let at = find(d, name)?;
    let n = u32::from_le_bytes(d.get(at.checked_sub(4)?..at)?.try_into().ok()?) as usize;
    (name.len() <= n && n < 200).then_some(at + n)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 4096 && u32::from_be_bytes([d[0], d[1], d[2], d[3]]) == fold(&d[4..]) && find(d, b"zwei/player/").is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a The Evil Within inventory".into());
    }
    let mut b = Builder::new(d);
    b.big_endian = true;
    const LEVELS: [&str; 8] = ["Damage", "Fire rate", "Reload time", "", "Clip capacity", "Accuracy", "Critical", "Stock"];
    for (key, name) in [("weapon/upgrade/handgun", "Handgun"), ("weapon/upgrade/shotgun", "Shotgun"), ("weapon/upgrade/rifle", "Rifle"), ("weapon/upgrade/magnum", "Magnum")] {
        if let Some(e) = name_end(d, key.as_bytes()) {
            let base = e + 31;
            for (i, what) in LEVELS.iter().enumerate().filter(|(_, w)| !w.is_empty()) {
                b.ranged(&["Weapon upgrades", name, what], base + 4 * i, Kind::U32, 1, 9);
            }
        }
    }
    if let Some(e) = name_end(d, b"weapon/upgrade/trapgun") {
        b.ranged(&["Weapon upgrades", "Agony Crossbow", "Clip capacity"], e + 43, Kind::U32, 1, 9);
        b.ranged(&["Weapon upgrades", "Agony Crossbow", "Critical"], e + 43 + 4 + 16, Kind::U32, 1, 9);
        b.ranged(&["Weapon upgrades", "Agony Crossbow", "Stock"], e + 43 + 4 + 16 + 4, Kind::U32, 1, 9);
    }
    if let Some(e) = name_end(d, b"zwei/player/upgrade_item") {
        b.ranged(&["Resources", "Green gel"], e, Kind::U32, 0, 999_999);
    }
    if let Some(e) = name_end(d, b"zwei/player/trap_parts") {
        b.ranged(&["Resources", "Trap parts"], e, Kind::U32, 0, 999);
    }
    if let Some(at) = find(d, b"gameflag/dlc3_boxman_story") {
        for (i, what) in ["Life gauge", "Maximum sprint", "Melee damage", "Syringe recovery"].iter().enumerate() {
            b.ranged(&["Upgrades", what], at + 501 + 4 * i, Kind::U32, 1, 9);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the game first. The checksum at the start of the file is written again on save. Weapons and upgrades you do not own yet are not listed."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; 5000];
        let name = b"zwei/player/upgrade_item";
        d[100..104].copy_from_slice(&(name.len() as u32).to_le_bytes());
        d[104..104 + name.len()].copy_from_slice(name);
        d[104 + name.len()..108 + name.len()].copy_from_slice(&1200u32.to_be_bytes());
        fix(&mut d);
        d
    }

    #[test]
    fn green_gel_edits_and_the_checksum_follows() {
        let d = synthetic();
        assert!(looks(&d));
        let mut t = parse(&d).unwrap();
        assert_eq!(t.view.pointer("/Resources/Green gel"), Some(&serde_json::Value::from(1200)));
        t.set("/Resources/Green gel", serde_json::Value::from(99_999)).unwrap();
        let out = t.to_bytes().unwrap();
        assert!(looks(&out));
    }
}
