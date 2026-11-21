//! Yu-Gi-Oh! Legacy of the Duelist (and Link Evolution): savegame.dat, a fixed-layout binary file
//! with a CRC-32 signature at offset 12 and a save counter at offset 16.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const KIND: &str = "Yu-Gi-Oh! Legacy of the Duelist save";
const MAGIC1: u32 = 0x54CE_29F9;
const MAGIC_LOTD: u32 = 0x0471_4D02;
const MAGIC_LINK: u32 = 0x04AB_E802;
const STATS: usize = 36;

const STAT_NAMES: [&str; 43] = [
    "Games: campaign",
    "Games: campaign, normal",
    "Games: campaign, reverse",
    "Games: challenge",
    "Games: multiplayer",
    "Games: multiplayer, 1 vs 1",
    "Games: multiplayer, tag",
    "Games: multiplayer, ranked",
    "Games: multiplayer, friendly",
    "Games: multiplayer, any battle pack",
    "Games: multiplayer, battle pack 1",
    "Games: multiplayer, battle pack 2",
    "Games: multiplayer, battle pack 3",
    "Wins: campaign",
    "Wins: campaign, normal",
    "Wins: campaign, reverse",
    "Wins: challenge",
    "Wins: multiplayer",
    "Wins: multiplayer, 1 vs 1",
    "Wins: multiplayer, tag",
    "Wins: multiplayer, ranked",
    "Wins: multiplayer, friendly",
    "Wins: multiplayer, any battle pack",
    "Wins: multiplayer, battle pack 1",
    "Wins: multiplayer, battle pack 2",
    "Wins: multiplayer, battle pack 3",
    "Wins: match",
    "Wins: non-match",
    "Normal Summons",
    "Tribute Summons",
    "Ritual Summons",
    "Fusion Summons",
    "Xyz Summons",
    "Synchro Summons",
    "Pendulum Summons",
    "Damage dealt",
    "Battle damage",
    "Direct damage",
    "Effect damage",
    "Reflected damage",
    "Chains",
    "Decks created",
    "Cards earned",
];

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The version's file length and the offset of the duel points, from the header.
fn layout(d: &[u8]) -> Option<(usize, usize)> {
    if u32_at(d, 0)? != MAGIC1 || u32_at(d, 8)? as usize != d.len() {
        return None;
    }
    match (u32_at(d, 4)?, d.len()) {
        (MAGIC_LOTD, 29005) => Some((3600, 380)),
        (MAGIC_LINK, 44008) => Some((4056, 836)),
        _ => None,
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 16 && layout(d).is_some()
}

/// The signature: CRC-32 (reflected, no final inversion) of the file with the signature zeroed.
fn sign(out: &mut Vec<u8>) {
    if out.len() < 20 {
        return;
    }
    let count = u32::from_le_bytes([out[16], out[17], out[18], out[19]]).wrapping_add(1);
    out[16..20].copy_from_slice(&count.to_le_bytes());
    out[12..16].fill(0);
    let crc = super::crc32_raw(0xFFFF_FFFF, out);
    out[12..16].copy_from_slice(&crc.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (misc, packs) = layout(d).ok_or("not a Legacy of the Duelist save")?;
    let mut b = Builder::new(d);
    b.ranged(&["Duelist", "Duel points"], misc + 16, Kind::U64, 0, 99_999_999);
    let numbers = (packs - STATS) / 8;
    for (i, name) in STAT_NAMES.iter().enumerate().take(numbers) {
        b.ranged(&["Statistics", name], STATS + 8 * i, Kind::U64, 0, 2_000_000_000);
    }
    Ok(b.finish(KIND, Checksum::Custom(sign)).with_note(crate::i18n::tx(
        "Close the game first. The save's signature is written again when you save. Cards, decks and unlocks are not edited.",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save() -> Vec<u8> {
        let mut d = vec![0u8; 29005];
        d[0..4].copy_from_slice(&MAGIC1.to_le_bytes());
        d[4..8].copy_from_slice(&MAGIC_LOTD.to_le_bytes());
        d[8..12].copy_from_slice(&29005u32.to_le_bytes());
        d[3616..3624].copy_from_slice(&337_398u64.to_le_bytes());
        d
    }

    #[test]
    fn reads_edits_and_signs_again() {
        let d = save();
        assert!(looks(&d));
        let mut t = parse(&d).unwrap();
        assert_eq!(t.view["Duelist"]["Duel points"], 337_398);
        assert_eq!(t.to_bytes().unwrap(), d, "unchanged data returns the file");
        t.set("/Duelist/Duel points", 5.into()).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(&out[3616..3624], &5u64.to_le_bytes());
        let mut check = out.clone();
        let stored = u32_at(&check, 12).unwrap();
        check[12..16].fill(0);
        assert_eq!(super::super::crc32_raw(0xFFFF_FFFF, &check), stored);
    }
}
