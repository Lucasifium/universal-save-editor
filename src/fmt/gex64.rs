//! Gex 64: Enter the Gecko (Nintendo 64). Each save is a Controller Pak note (`.mpk`, or the
//! pak part of an emulator's `.srm`) holding the game's 18-letter password: 90 bits that, once
//! the first byte's XOR key is taken off, give the lives and every remote collected.
//!
//! Layout and password coding after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::mpk;
use crate::tree::TreeDoc;

pub const KIND: &str = "Gex 64 save";

/// Each save as decoded: 12 bytes, padded to 16.
const BLOCK: usize = 0x10;
const PAGE: usize = 0x100;

fn is_gex(c: &[u8; 4]) -> bool {
    &c[..3] == b"NX2" && matches!(c[3], b'E' | b'P')
}

/// The 12 bytes behind the password at the start of a note page.
fn decode(page: &[u8]) -> Option<[u8; 12]> {
    if page[..18].iter().any(|&l| l >= 0x20) {
        return None;
    }
    let mut buf = [0u8; 12];
    for bit in 0..90 {
        if (page[bit / 5] >> (4 - bit % 5)) & 1 != 0 {
            buf[bit / 8] |= 0x80 >> (bit % 8);
        }
    }
    for i in 1..12 {
        buf[i] ^= buf[0];
    }
    buf[0] = 0;
    buf[11] &= 0x80;
    Some(buf)
}

/// The 18 password letters for `data` (as `decode` gives it).
fn encode(data: &[u8]) -> [u8; 18] {
    let mut buf = [0u8; 12];
    buf.copy_from_slice(&data[..12]);
    buf[0] = buf[1..].iter().fold(0u8, |a, b| a.wrapping_add(b.count_ones() as u8));
    for i in 1..12 {
        buf[i] ^= buf[0];
    }
    let mut out = [0u8; 18];
    for bit in 0..90 {
        if (buf[bit / 8] >> (7 - bit % 8)) & 1 != 0 {
            out[bit / 5] |= 1 << (4 - bit % 5);
        }
    }
    out
}

fn decoded(d: &[u8]) -> Option<Vec<u8>> {
    let (_, raw) = mpk::take(d, is_gex, true)?;
    let mut out = vec![];
    for page in raw.chunks(PAGE) {
        out.extend_from_slice(&decode(page)?);
        out.extend_from_slice(&[0; BLOCK - 12]);
    }
    Some(out)
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    let Some((_, mut raw)) = mpk::take(outer, is_gex, true) else { return outer.to_vec() };
    for (k, page) in raw.chunks_mut(PAGE).enumerate() {
        let new = &inner[k * BLOCK..k * BLOCK + 12];
        // Saves left alone keep their own letters.
        if decode(page).is_some_and(|old| old[..] != *new) {
            page[..18].copy_from_slice(&encode(new));
        }
    }
    mpk::put(outer, &raw, is_gex, true)
}

pub fn looks(d: &[u8]) -> bool {
    decoded(d).is_some()
}

/// Levels with their red remotes (byte, bit; up to three) and the silver remotes for finding
/// the hidden one and for the reward.
const LEVELS: [(&str, &[(usize, u32)], (usize, u32), (usize, u32)); 14] = [
    ("Out of Toon", &[(1, 7), (1, 6), (1, 5)], (5, 4), (5, 3)),
    ("Smellraiser", &[(1, 4), (1, 3), (1, 2)], (5, 2), (5, 1)),
    ("Gecques Cousteau", &[(1, 1), (1, 0), (2, 7)], (5, 0), (6, 7)),
    ("Frankensteinfeld", &[(2, 6), (2, 5), (2, 4)], (6, 6), (6, 5)),
    ("www.dotcom.com", &[(2, 3), (2, 2)], (6, 4), (6, 3)),
    ("Mao Tse Tongue", &[(2, 1), (2, 0)], (6, 2), (6, 1)),
    ("The Umpire Strikes Out", &[(3, 7), (3, 6)], (6, 0), (7, 7)),
    ("Pangaea 90210", &[(3, 5), (3, 4)], (7, 6), (7, 5)),
    ("Fine Tooning", &[(3, 3), (3, 2)], (7, 4), (7, 3)),
    ("This Old Cave", &[(3, 1), (3, 0), (4, 7)], (7, 2), (7, 1)),
    ("Honey I Shrunk the Gecko", &[(4, 6), (4, 5), (4, 4)], (7, 0), (8, 7)),
    ("Pain in the Asteroids", &[(4, 3), (4, 2), (4, 1)], (8, 6), (8, 5)),
    ("Samurai Night Fever", &[(4, 0), (5, 7), (5, 6)], (8, 4), (8, 3)),
    ("No Weddings and a Funeral", &[(5, 5)], (8, 2), (8, 1)),
];
/// Gold remotes: bonus levels, then bosses.
const GOLD: [(&str, usize, u32); 11] = [
    ("Aztec 2 Step", 8, 0), ("Thursday the 12th", 9, 7), ("In Drag Net", 9, 6), ("The Spy Who Loved Himself", 9, 5),
    ("Lizard in a China Shop", 9, 4), ("Bugged Out", 9, 3), ("Chips and Dips", 9, 2), ("Gilligex Isle", 9, 1),
    ("Mooshoo Pork", 9, 0), ("Gexzilla vs. Mecharez", 10, 7), ("Channel Z", 10, 6),
];

fn count(s: &[u8], bits: impl Iterator<Item = (usize, u32)>) -> i64 {
    bits.filter(|&(at, bit)| s[at] >> bit & 1 != 0).count() as i64
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let inner = decoded(d).ok_or("not a Gex 64 save")?;
    let mut b = Builder::new(&inner);
    b.inside(d, put);
    b.big_endian = true;
    let yes_no = vec![("No".to_string(), 0), ("Yes".to_string(), 1)];
    let saves = inner.len() / BLOCK;
    for k in 0..saves {
        let s = k * BLOCK;
        let blk = &inner[s..s + BLOCK];
        let slot = format!("Save {}", k + 1);
        let sl = slot.as_str();
        b.bits(&["Saves", sl, "Lives"], s + 0xa, Kind::U16, 7, 7, 0, 1, 99);
        b.info(&["Saves", sl, "Red remotes"], count(blk, LEVELS.iter().flat_map(|l| l.1.iter().copied())));
        b.info(&["Saves", sl, "Silver remotes"], count(blk, LEVELS.iter().flat_map(|l| [l.2, l.3])));
        b.info(&["Saves", sl, "Gold remotes"], count(blk, GOLD.iter().map(|g| (g.1, g.2))));
        for (level, reds, hidden, reward) in LEVELS {
            let row = format!("{sl}: {level}");
            let r = row.as_str();
            for (i, &(at, bit)) in reds.iter().enumerate() {
                let name = format!("Red remote {}", i + 1);
                b.pick_bits(&["Levels", r, name.as_str()], s + at, Kind::U8, bit, 1, &yes_no);
            }
            b.pick_bits(&["Levels", r, "Hidden remote"], s + hidden.0, Kind::U8, hidden.1, 1, &yes_no);
            b.pick_bits(&["Levels", r, "Reward remote"], s + reward.0, Kind::U8, reward.1, 1, &yes_no);
        }
        for (level, at, bit) in GOLD {
            let row = format!("{sl}: {level}");
            b.pick_bits(&["Gold remotes", row.as_str(), "Collected"], s + at, Kind::U8, bit, 1, &yes_no);
        }
    }
    b.info(&["Info", "Saves on the pak"], saves as i64);
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tx("Close the emulator first. Each save is the game's password; a changed save gets a new password on save.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip_and_edit() {
        let mut data = [0u8; 12];
        data[0xa] = ((5u16 << 7) >> 8) as u8;
        data[0xb] = (5u16 << 7) as u8;
        let mut page = vec![0u8; PAGE];
        page[..18].copy_from_slice(&encode(&data));
        assert_eq!(decode(&page).unwrap(), data);
        let pak = mpk::build(&[(*b"NX2E", page.clone()), (*b"NETF", vec![7; PAGE]), (*b"NX2E", page)]);
        assert!(looks(&pak));
        let mut doc = parse(&pak).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), pak);
        assert_eq!(doc.view.pointer("/Saves/Save 2/Lives").unwrap(), 5);
        doc.set("/Saves/Save 2/Lives", 99.into()).unwrap();
        doc.set("/Levels/Save 2: Out of Toon/Red remote 1", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let (_, raw) = mpk::take(&out, is_gex, true).unwrap();
        assert_eq!(&raw[..PAGE], &pak[5 * PAGE..6 * PAGE]);
        let got = decode(&raw[PAGE..]).unwrap();
        assert_eq!((u16::from_be_bytes([got[0xa], got[0xb]]) >> 7) & 0x7f, 99);
        assert_eq!(got[1] & 0x80, 0x80);
        assert_eq!(&out[6 * PAGE..7 * PAGE], &[7u8; PAGE][..]);
    }

    /// `SAVE_SAMPLES=<dir>`: real paks open, every password decodes and codes back the same.
    #[test]
    #[ignore]
    fn real_saves_round_trip() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut seen = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if let Some((_, raw)) = mpk::take(&d, is_gex, true) {
                for page in raw.chunks(PAGE) {
                    let x = decode(page).unwrap();
                    assert_eq!(encode(&x)[..], page[..18], "{:?}", e.path());
                }
                let doc = parse(&d).unwrap();
                assert_eq!(doc.to_bytes().unwrap(), d);
                println!("{:?}: {}", e.path(), doc.view.pointer("/Saves/Save 1/Lives").unwrap());
                seen += 1;
            }
        }
        assert!(seen > 0);
    }
}

#[cfg(test)]
mod dispatch {
    /// `SAVE_SAMPLES=<dir>`: the general reader picks the right game for each N64 sample and
    /// shows its values.
    #[test]
    #[ignore]
    fn n64_samples_open_as_their_game() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut paths: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            let d = std::fs::read(&p).unwrap();
            let doc = crate::fmt::parse_binary(&d);
            let kind = doc.as_ref().map(|t| t.kind_name.clone()).unwrap_or_default();
            println!("{}: {kind}", p.file_name().unwrap().to_string_lossy());
            if std::env::var("SHOW").is_ok_and(|s| p.to_string_lossy().contains(&s)) {
                if let Some(t) = doc {
                    println!("{}", serde_json::to_string_pretty(&t.view).unwrap());
                }
            }
        }
    }
}
