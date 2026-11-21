//! Middle-earth: Shadow of War saves (`<user id>_ShadowOfWar.sav`): a 16-byte header (`SOM3`, four
//! key bytes, the padded and the real length) around AES-256-CBC data. The key is made from a fixed
//! string with four bytes of the header and four bytes of the player's 32-bit user id mixed in,
//! and the four header bytes are the CRC-32 of the plain data, so the file only opens with the
//! right id and a matching checksum. The plain data is a LithTech chunk file (`SAVE` ...).
//!
//! The encryption follows the public descriptions by SystematicSkid's shadow-of-war-save-converter
//! and Marcello Cuoghi's shadowofwar-save-tools (both MIT). The statistics are found by name:
//! the game's lifetime counters are stored as `04 <CRC-32 of the CamelCase name> 04 <value>`; the
//! names listed here were checked against the Steam statistics of two public saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::block_padding::NoPadding;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use std::collections::HashMap;

pub const KIND: &str = "Shadow of War save";

const RAW_KEY: &[u8; 32] = b"ad@210766@vac94Cd_?dVt5$alivjz$e";
const IV: &[u8; 16] = b"yuwgb@oftv@gx$t3";

/// The counters whose names and values were confirmed on two public saves (CRC-32 of each name).
const STATS: &[&str] = &[
    "Ambushed", "Balrog", "BladeCompleted", "BlindOrcs", "Branded", "BrandedArchers", "BrandedBerserkers", "BrandedCaptains",
    "BrandedDefenders", "BrandedOlog", "BrandedOverlords", "BrandedSpearmen", "BrandedWarchiefs", "BrandedWarriors",
    "CarnanMissionsCompleted", "CombatDeathFromAboveHighest", "CombatDeathFromAboveLast", "CombatFinishers", "CombatMountAttacks",
    "CombatMountEats", "CombatShadowBrands", "CombatShadowKills", "CombatShadowMounts", "CombatShadowStrikes",
    "CombatWeaknessCaptains", "CombatWeaknessOverlords", "CombatWeaknessWarchiefs", "DrakeBaited", "EquipmentUpgradesApplied",
    "FearsTriggered", "FollowerKill", "ForgedGems", "ForgedGemsLevel5", "FortAttacks", "FortDefenses", "GearCollected",
    "GemSlotsUnlocked", "GondorArtifacts", "GondorCompleted", "HaedirPurified", "HatesTriggered", "HelmHammerhand",
    "IthildinDoorsOpened", "KilledArchers", "KilledBerserkers", "KilledCaptains", "KilledCaragors", "KilledCritter",
    "KilledDefenders", "KilledGhuls", "KilledGraugs", "KilledOlog", "KilledOrcsByCurse", "KilledOrcsByPoison",
    "KilledOrcsWhileFrozen", "KilledOverlords", "KilledSpearmen", "KilledSpiders", "KilledUruks", "KilledUruksByFire",
    "KilledWarchiefs", "KilledWarriors", "Kills", "KillsByBow", "KillsByDagger", "KillsBySword", "KillsTypeDeathFromAbove",
    "KillsTypeDecapitations", "KillsTypeStealth", "KillsTypeStealthOnCaragors", "KillsTypeWraith", "MaxLevelReached",
    "MonumentsDestroyed", "MountsDominated", "OrcDeathByOrc", "OutpostsDisabled", "PlayerSkillsUnlocked", "Quests",
    "ShelobPuzzles", "Suladan", "TotalArtifactsDiscovered", "TotalBaitDropped", "TotalBowHeadshots", "TotalCagesOpened",
    "TotalCaragorsReleased", "TotalDrakesReleased", "TotalFiresExploded", "TotalFliesDropped", "TotalGraugsReleased",
    "TotalIntelLearned", "TotalMirian", "TotalSkillPoints", "TotalWallsDestroyed", "TribeDarkKilled", "TribeFeralKilled",
    "TribeMachineKilled", "TribeMachineRecruited", "TribeMarauderRecruited", "TribeOutlawKilled", "TribeOutlawRecruited", "TribeSlaughterKilled",
    "TribeSlaughterRecruited", "TribeSorcerousKilled", "TribeSorcerousRecruited", "TribeTerrorKilled", "TribeTerrorRecruited",
    "TribeWarmongerKilled", "TribeWarmongerRecruited", "UruksDominated", "VendettasCompleted", "WarUpgradesEquipped",
    "WarchiefsDefeated",
];

/// Where in the plain data the counters are looked for (they sit in the first few kilobytes).
const WINDOW: usize = 0x10000;

fn crc32(d: &[u8]) -> u32 {
    let mut c = flate2::Crc::new();
    c.update(d);
    c.sum()
}

/// The AES key for a header's four key bytes and a user id.
fn key(key_data: [u8; 4], user: u32) -> [u8; 32] {
    let mut k = *RAW_KEY;
    for (pos, idx) in [(11, 1), (26, 0), (17, 3), (18, 2)] {
        k[pos] = key_data[idx];
    }
    let u = user.to_be_bytes();
    for (pos, idx) in [(30, 2), (10, 3), (2, 0), (22, 1)] {
        k[pos] = u[idx];
    }
    k
}

/// True for a file that starts like an encrypted Shadow of War save.
pub fn maybe(d: &[u8]) -> bool {
    d.len() >= 48 && d.starts_with(b"SOM3") && (d.len() - 16) % 16 == 0
}

/// The plain data of a save the user id opens (the checksum in the header has to match).
pub fn decrypt(d: &[u8], user: u32) -> Option<Vec<u8>> {
    if !maybe(d) {
        return None;
    }
    let key_data: [u8; 4] = d[4..8].try_into().ok()?;
    let real = u32::from_le_bytes(d[12..16].try_into().ok()?) as usize;
    let body = &d[16..];
    if u32::from_le_bytes(d[8..12].try_into().ok()?) as usize != body.len() || real > body.len() || body.len() - real > 16 {
        return None;
    }
    // The first block alone tells a wrong id (the plain data starts with "SAVE").
    let mut first = [0u8; 16];
    first.copy_from_slice(&body[..16]);
    let k = key(key_data, user);
    cbc::Decryptor::<aes::Aes256>::new(&k.into(), IV.into()).decrypt_padded_mut::<NoPadding>(&mut first).ok()?;
    if &first[..4] != b"SAVE" {
        return None;
    }
    let mut plain = body.to_vec();
    cbc::Decryptor::<aes::Aes256>::new(&k.into(), IV.into()).decrypt_padded_mut::<NoPadding>(&mut plain).ok()?;
    plain.truncate(real);
    (crc32(&plain).to_le_bytes() == key_data).then_some(plain)
}

/// The file for plain data: the checksum becomes the key bytes, PKCS#7 padding as the game writes it.
pub fn encrypt(plain: &[u8], user: u32) -> Vec<u8> {
    let key_data = crc32(plain).to_le_bytes();
    let pad = 16 - plain.len() % 16;
    let mut body = plain.to_vec();
    body.extend(std::iter::repeat(pad as u8).take(pad));
    let k = key(key_data, user);
    let len = body.len();
    cbc::Encryptor::<aes::Aes256>::new(&k.into(), IV.into()).encrypt_padded_mut::<NoPadding>(&mut body, len).expect("whole blocks");
    let mut out = b"SOM3".to_vec();
    out.extend(key_data);
    out.extend((body.len() as u32).to_le_bytes());
    out.extend((plain.len() as u32).to_le_bytes());
    out.extend(body);
    out
}

/// The user ids to try: the hex name the game gives the file (`6c3dc14a_ShadowOfWar.sav`, also
/// `6c3dc14a_2_ShadowOfWar.sav`), an id typed in (hex or decimal), and 0, which GOG saves use.
pub fn users(path: Option<&std::path::Path>, typed: Option<&str>) -> Vec<u32> {
    let mut out: Vec<u32> = vec![];
    if let Some(name) = path.and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
        if let Some(id) = name.split('_').next().filter(|h| (1..=8).contains(&h.len())).and_then(|h| u32::from_str_radix(h, 16).ok()) {
            out.push(id);
        }
    }
    if let Some(t) = typed.map(str::trim).filter(|t| !t.is_empty()) {
        let t = t.trim_start_matches("0x");
        out.extend(u32::from_str_radix(t, 16).ok());
        out.extend(t.parse::<u32>().ok());
    }
    out.push(0);
    out.dedup();
    out
}

pub fn find(d: &[u8], users: &[u32]) -> Option<(u32, Vec<u8>)> {
    users.iter().find_map(|u| decrypt(d, *u).map(|p| (*u, p)))
}

/// The statistics of a decrypted save: each name's value position, when the save has enough of them.
fn located(d: &[u8]) -> Option<Vec<(&'static str, usize)>> {
    let names: HashMap<u32, &'static str> = STATS.iter().map(|n| (crc32(n.as_bytes()), *n)).collect();
    let end = d.len().min(WINDOW);
    let mut found: HashMap<&'static str, Vec<usize>> = HashMap::new();
    for p in 0..end.saturating_sub(10) {
        if d[p] == 4 && d[p + 5] == 4 {
            let id = u32::from_le_bytes(d[p + 1..p + 5].try_into().ok()?);
            if let Some(n) = names.get(&id) {
                found.entry(n).or_default().push(p + 6);
            }
        }
    }
    // Names that turn up twice are left out rather than guessed.
    let mut out: Vec<(&'static str, usize)> = STATS.iter().filter_map(|n| found.get(n).filter(|v| v.len() == 1).map(|v| (*n, v[0]))).collect();
    out.sort_by_key(|(_, at)| *at);
    (out.len() >= 60).then_some(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 4096 && d.starts_with(b"SAVE") && d[4..8] == 2u32.to_le_bytes() && located(d).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let found = located(d).ok_or("not a Shadow of War save")?;
    let mut b = Builder::new(d);
    for (name, at) in found {
        b.field(&["Statistics", name], at, Kind::U32);
    }
    Ok(b.finish(KIND, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save-shaped file: a `SAVE` header, then the counters as the game writes them.
    fn synthetic() -> Vec<u8> {
        let mut d = b"SAVE".to_vec();
        d.extend(2u32.to_le_bytes());
        d.resize(40, 0);
        for (i, n) in STATS.iter().enumerate() {
            d.push(4);
            d.extend(crc32(n.as_bytes()).to_le_bytes());
            d.push(4);
            d.extend((i as u32 * 7).to_le_bytes());
        }
        d.resize(6000, 0);
        d
    }

    #[test]
    fn round_trips_and_wrong_ids_fail() {
        let plain = synthetic();
        let file = encrypt(&plain, 0x6c3dc14a);
        assert!(maybe(&file));
        assert_eq!(decrypt(&file, 0x6c3dc14a).as_deref(), Some(&plain[..]));
        assert!(decrypt(&file, 0x6c3dc14b).is_none());
        assert_eq!(find(&file, &[1, 2, 0x6c3dc14a]).map(|(u, _)| u), Some(0x6c3dc14a));
        // Same plain data, same file: the game's own padding and key bytes are reproduced.
        assert_eq!(encrypt(&decrypt(&file, 0x6c3dc14a).unwrap(), 0x6c3dc14a), file);
    }

    #[test]
    fn user_ids_come_from_the_file_name() {
        let p = std::path::Path::new("x/6c3dc14a_2_ShadowOfWar.sav");
        assert_eq!(users(Some(p), None), vec![0x6c3dc14a, 0]);
        assert_eq!(users(None, Some("0x10")), vec![16, 10, 0]);
    }

    #[test]
    fn reads_and_edits_statistics() {
        let plain = synthetic();
        assert!(looks(&plain));
        let mut doc = parse(&plain).unwrap();
        assert_eq!(doc.view.pointer("/Statistics/Balrog").and_then(|v| v.as_u64()), Some(7));
        assert_eq!(doc.to_bytes().unwrap(), plain);
        doc.set("/Statistics/Kills", serde_json::Value::from(123456)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), plain.len());
        assert_eq!(&out[..40], &plain[..40]);
        assert_eq!(parse(&out).unwrap().view.pointer("/Statistics/Kills").and_then(|v| v.as_u64()), Some(123456));
    }
}
