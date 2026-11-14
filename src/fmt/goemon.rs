//! Mystical Ninja Starring Goemon and Goemon's Great Adventure (Mystical Ninja 2) for the
//! Nintendo 64. Both keep their saves on the Controller Pak (`.mpk`, or the pak part of an
//! emulator's `.srm`); the Japanese Goemon's Great Adventure uses cartridge SRAM (`.sra`/`.srm`).
//! Every block carries a CRC-32 (MSB first, polynomial 0x04C11DB7, inverted) of its bytes.
//!
//! Layouts after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::{mpk, n64data};
use crate::tree::TreeDoc;

pub const KIND1: &str = "Mystical Ninja Starring Goemon save";
pub const KIND2: &str = "Goemon's Great Adventure save";

fn crc(bytes: impl Iterator<Item = u8>) -> u32 {
    let mut c = 0xffff_ffffu32;
    for b in bytes {
        c ^= u32::from(b) << 24;
        for _ in 0..8 {
            c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04c1_1db7 } else { c << 1 };
        }
    }
    !c
}

/// The CRC of `d[from..to]` with the bytes `zero` (absolute range) read as zero.
fn crc_of(d: &[u8], from: usize, to: usize, zero: std::ops::Range<usize>) -> u32 {
    crc((from..to).map(|i| if zero.contains(&i) { 0 } else { d[i] }))
}

fn opts(names: &[&str]) -> Vec<(String, u64)> {
    names.iter().enumerate().map(|(i, n)| (n.to_string(), i as u64)).collect()
}

fn yes_no() -> Vec<(String, u64)> {
    opts(&["No", "Yes"])
}

// ------------------------------------------------------------------ Mystical Ninja Starring Goemon

const SLOT1: usize = 0x500;

fn is_goemon1(c: &[u8; 4]) -> bool {
    &c[..3] == b"NG5" && matches!(c[3], b'E' | b'P' | b'J')
}

/// A slot is in use when the word at 0x172 is set; unused slots may hold stale bytes.
fn used1(d: &[u8], s: usize) -> bool {
    d[s + 0x172] != 0 || d[s + 0x173] != 0
}

fn slot1_ok(d: &[u8], s: usize) -> bool {
    !used1(d, s) || d[s + 0x100..s + 0x104] == crc_of(d, s + 0x104, s + 0x408, s + 0x104..s + 0x10c).to_be_bytes()
}

fn put1(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    mpk::put(outer, inner, is_goemon1, false)
}

pub fn looks1(d: &[u8]) -> bool {
    mpk::take(d, is_goemon1, false).is_some_and(|(_, n)| n.len() >= 3 * SLOT1 && (0..3).all(|i| slot1_ok(&n, i * SLOT1)))
}

const CHARACTERS: [(&str, usize, [&str; 4]); 4] = [
    ("Goemon", 0x19b, ["Chain Pipe", "Medal", "Flaming Medal", "Sudden Impact"]),
    ("Ebisumaru", 0x19f, ["Meat Saw-Hammer", "Windup Camera", "Windup Camera Flash", "Mini-Ebisu"]),
    ("Sasuke", 0x1a3, ["Fire Cracker Bomb", "Kunai of Severe Cold", "Kunai Intense Hail Beam", "Flying"]),
    ("Yae", 0x1a7, ["Koryuta's Flute", "Yae Bazooka", "Lock on Bazooka", "Mermaid"]),
];
const KEY_ITEMS: [(&str, usize); 10] = [
    ("Map of Japan", 0x36f), ("Triton Shell", 0x1fb), ("Key to Training", 0x20f), ("Super Pass", 0x1ff), ("Achilles' Heel", 0x20b),
    ("Quality Cucumber", 0x213), ("Miracle Star", 0x357), ("Miracle Moon", 0x35b), ("Miracle Flower", 0x35f), ("Miracle Snow", 0x363),
];
const CASTLES: [(&str, usize, usize); 5] = [
    ("Transformed Oedo Castle", 0x373, 0x387), ("Ghost Toys Castle", 0x377, 0x38b), ("Festival Temple Castle", 0x37b, 0x38f),
    ("Gourmet Submarine Castle", 0x37f, 0x393), ("Gorgeous Musical Castle", 0x383, 0x397),
];

pub fn parse1(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks1(d) {
        return Err("not a Mystical Ninja Starring Goemon save".into());
    }
    let (_, n) = mpk::take(d, is_goemon1, false).ok_or("no Goemon note")?;
    let mut b = Builder::new(&n);
    b.inside(d, put1);
    b.big_endian = true;
    let mut used = 0;
    for i in 0..3 {
        let s = i * SLOT1;
        if !used1(&n, s) {
            continue;
        }
        used += 1;
        let slot = format!("Slot {}", i + 1);
        let sl = slot.as_str();
        for (c, at, hi) in [
            ("Ryo", 0x178, 9999),
            ("Play time (seconds)", 0x368, 99 * 3600 + 59 * 60 + 59),
            ("Fortune doll pieces", 0x200, 3),
            ("Red fish", 0x214, 8),
            ("Yellow fish", 0x218, 5),
            ("Blue fish", 0x21c, 3),
            ("Headwear quantity", 0x188, 8),
            ("Bodywear quantity", 0x190, 8),
        ] {
            b.ranged(&[sl, "General", c], s + at, Kind::U32, 0, hi);
        }
        b.pick(&[sl, "General", "Character"], s + 0x16c, Kind::U32, &opts(&["Goemon", "Ebisumaru", "Sasuke", "Yae"]));
        b.pick(&[sl, "General", "Food"], s + 0x180, Kind::U32, &opts(&["-", "Plain Rice Balls", "Plum Rice Balls", "Fish Rice Balls"]));
        b.pick(&[sl, "General", "Headwear"], s + 0x184, Kind::U32, &opts(&["-", "Sombrero", "Metal Helmet", "Gold Helmet"]));
        b.pick(&[sl, "General", "Bodywear"], s + 0x18c, Kind::U32, &opts(&["-", "Straw Raincoat", "Metal Armour", "Gold Armour"]));
        let keys = opts(&["-", "Silver Key", "Gold Key", "Diamond Key"]);
        for k in 0..5 {
            let name = format!("Key {}", k + 1);
            b.pick(&[sl, "General", name.as_str()], s + 0x3f4 + k * 4, Kind::U32, &keys);
        }
        for (who, at, items) in CHARACTERS {
            b.pick_bits(&[sl, "Characters", who, "Unlocked"], s + at, Kind::U8, 0, 1, &yes_no());
            let levels = opts(&["Level 1", "Level 2", "Level 3"]);
            b.pick(&[sl, "Characters", who, "Weapon level"], s + at + 0x10, Kind::U8, &levels);
            for (j, item) in items.iter().enumerate() {
                let name = format!("{who}: {item}");
                b.pick_bits(&[sl, "Character items", name.as_str(), "Owned"], s + at + 0x20 + j * 0x10, Kind::U8, 0, 1, &yes_no());
            }
        }
        for (item, at) in KEY_ITEMS {
            b.pick_bits(&[sl, "Key items", item, "Owned"], s + at, Kind::U8, 0, 1, &yes_no());
        }
        for (castle, map, boss) in CASTLES {
            b.pick_bits(&[sl, "Castles", castle, "Map"], s + map, Kind::U8, 0, 1, &yes_no());
            b.pick_bits(&[sl, "Castles", castle, "Boss revealed"], s + boss, Kind::U8, 0, 1, &yes_no());
        }
    }
    b.info(&["Info", "Slots in use"], used as i64);
    Ok(b.finish(KIND1, Checksum::Custom(fix1)).with_note(crate::i18n::tx("Close the emulator first. Only the first Goemon note on the Controller Pak is edited; each slot's CRC is made again on save.")))
}

fn fix1(d: &mut Vec<u8>) {
    for i in 0..3 {
        let s = i * SLOT1;
        if s + 0x408 <= d.len() && used1(d, s) {
            let c = crc_of(d, s + 0x104, s + 0x408, s + 0x104..s + 0x10c).to_be_bytes();
            d[s + 0x100..s + 0x104].copy_from_slice(&c);
        }
    }
}

// ------------------------------------------------------------------ Goemon's Great Adventure

const BLOCK2: usize = 0x130;

fn is_goemon2(c: &[u8; 4]) -> bool {
    &c[..3] == b"NGM" && matches!(c[3], b'E' | b'P')
}

/// Where the three slot blocks start: 0x100, 0x300, 0x500 in the pak note; 0x10, 0x140,
/// 0x270 in the Japanese SRAM.
fn blocks2(len: usize) -> [usize; 3] {
    if len == 0x8000 {
        [0x10, 0x10 + BLOCK2, 0x10 + 2 * BLOCK2]
    } else {
        [0x100, 0x300, 0x500]
    }
}

fn block_ok(d: &[u8], at: usize, len: usize) -> bool {
    at + len <= d.len() && d[at + 4..at + 8] == crc_of(d, at, at + len, at + 4..at + 8).to_be_bytes()
}

/// A slot is in use when its location byte is set; unused slots may hold stale bytes.
fn used2(d: &[u8], at: usize) -> bool {
    d[at + 0x08] != 0
}

fn valid2(d: &[u8]) -> bool {
    d.len() >= 0x700 && block_ok(d, 0, 0x10) && blocks2(d.len()).iter().all(|&a| !used2(d, a) || block_ok(d, a, BLOCK2))
}

/// The game data: the pak note (Europe and USA) or the SRAM (Japan), and the note's code.
fn data2(d: &[u8]) -> Option<(Option<[u8; 4]>, Vec<u8>)> {
    if let Some((codes, n)) = mpk::take(d, is_goemon2, false) {
        if valid2(&n) {
            return Some((Some(codes[0]), n));
        }
    }
    n64data::sram(d, valid2).map(|(_, s)| (None, s))
}

fn put2(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    if inner.len() == 0x8000 {
        n64data::sram_put(outer, inner, valid2)
    } else {
        mpk::put(outer, inner, is_goemon2, false)
    }
}

pub fn looks2(d: &[u8]) -> bool {
    data2(d).is_some()
}

pub fn parse2(d: &[u8]) -> Result<TreeDoc, String> {
    let (code, n) = data2(d).ok_or("not a Goemon's Great Adventure save")?;
    let mut b = Builder::new(&n);
    b.inside(d, put2);
    b.big_endian = true;
    if code.is_some_and(|c| c[3] == b'P') {
        b.pick(&["Options", "Language"], 0x8, Kind::U8, &opts(&["English", "French", "German"]));
    }
    b.pick(&["Options", "Sound"], 0x9, Kind::U8, &opts(&["Stereo", "Monaural"]));
    let places: Vec<(String, u64)> = [
        (1, "Lost'n Town"), (2, "Otohime Town"), (3, "Spook Village"), (4, "Tree Stump Village"), (5, "Floating World Town"),
        (6, "Creep Village"), (7, "Home of Wiseman"), (8, "Kappa Road"), (9, "Kokeshi Road"), (10, "Kaiware Road"), (11, "Cloud Fortress"),
    ]
    .iter()
    .map(|(v, n)| (n.to_string(), *v))
    .collect();
    let mut used = 0;
    for (i, s) in blocks2(n.len()).into_iter().enumerate() {
        if !used2(&n, s) {
            continue;
        }
        used += 1;
        let slot = format!("Slot {}", i + 1);
        let p = |k: &'static str| ["Slots", slot.as_str(), k];
        b.pick(&p("Location"), s + 0x08, Kind::U8, &places);
        b.ranged(&p("Ryo"), s + 0x7e, Kind::U16, 0, 999);
        b.ranged(&p("Armor"), s + 0x7d, Kind::U8, 0, 3);
        b.pick(&p("Weapon level"), s + 0x81, Kind::U8, &opts(&["Level 1", "Level 2", "Level 3"]));
        b.pick(&p("Item"), s + 0x82, Kind::U8, &opts(&["-", "Rice balls", "Plum rice balls", "Fish rice balls"]));
        b.ranged(&p("Money given to Kenzo"), s + 0x78, Kind::U16, 0, 100);
        b.ranged(&p("Money given to Kensuke"), s + 0x76, Kind::U16, 0, 3500);
        b.pick_bits(&p("Chain Pipe"), s + 0x22, Kind::U8, 2, 1, &yes_no());
        b.pick_bits(&p("Underwater Device"), s + 0x23, Kind::U8, 3, 1, &yes_no());
    }
    b.info(&["Info", "Slots in use"], used as i64);
    Ok(b.finish(KIND2, Checksum::Custom(fix2)).with_note(crate::i18n::tx("Close the emulator first. The CRC of the options and of each slot is made again on save.")))
}

fn fix2(d: &mut Vec<u8>) {
    let mut blocks = vec![(0, 0x10)];
    blocks.extend(blocks2(d.len()).iter().filter(|&&a| used2(d, a)).map(|&a| (a, BLOCK2)));
    for (at, len) in blocks {
        let c = crc_of(d, at, at + len, at + 4..at + 8).to_be_bytes();
        d[at + 4..at + 8].copy_from_slice(&c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::n64data::swap32;

    #[test]
    fn goemon1_slot_edit_keeps_crc() {
        let mut n = vec![0u8; 0x1000];
        n[0x500 + 0x173] = 0x0a;
        fix1(&mut n);
        let pak = mpk::build(&[(*b"NG5E", n)]);
        assert!(looks1(&pak));
        let mut doc = parse1(&pak).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), pak);
        doc.set("/Slot 2/General/Ryo", 9999.into()).unwrap();
        doc.set("/Slot 2/Characters/Yae/Unlocked", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks1(&out));
        let (_, n) = mpk::take(&out, is_goemon1, false).unwrap();
        assert_eq!(u32::from_be_bytes(n[0x500 + 0x178..0x500 + 0x17c].try_into().unwrap()), 9999);
        assert_eq!(n[0x500 + 0x1a7], 1);
        assert!(doc.view.pointer("/Slot 1/General/Ryo").is_none());
    }

    #[test]
    fn goemon2_pak_and_japanese_sram() {
        let mut n = vec![0u8; 0x700];
        n[0x300 + 0x08] = 1;
        fix2(&mut n);
        let pak = mpk::build(&[(*b"NGMP", n)]);
        assert!(looks2(&pak));
        let mut doc = parse2(&pak).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), pak);
        doc.set("/Slots/Slot 2/Ryo", 999.into()).unwrap();
        doc.set("/Options/Language", "German".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        let (_, n) = data2(&out).unwrap();
        assert_eq!(u16::from_be_bytes([n[0x37e], n[0x37f]]), 999);
        assert_eq!(n[8], 2);
        assert!(valid2(&n));

        let mut s = vec![0u8; 0x8000];
        s[0x10 + BLOCK2 + 0x08] = 3;
        fix2(&mut s);
        let file = swap32(&s);
        let mut doc = parse2(&file).unwrap();
        assert!(doc.view.pointer("/Options/Language").is_none());
        doc.set("/Slots/Slot 2/Armor", 3.into()).unwrap();
        let out = swap32(&doc.to_bytes().unwrap());
        assert_eq!(out[0x10 + BLOCK2 + 0x7d], 3);
        assert!(valid2(&out));
    }

    /// `SAVE_SAMPLES=<dir>`: real saves open and write back unchanged, with their CRCs.
    #[test]
    #[ignore]
    fn real_saves_round_trip() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut seen = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            for (ok, parse) in [(looks1 as fn(&[u8]) -> bool, parse1 as fn(&[u8]) -> Result<TreeDoc, String>), (looks2, parse2)] {
                if ok(&d) {
                    let doc = parse(&d).unwrap();
                    assert_eq!(doc.to_bytes().unwrap(), d, "{:?}", e.path());
                    println!("{:?}: {}", e.path(), doc.view.pointer("/Info/Slots in use").map(|v| v.to_string()).unwrap_or_default());
                    seen += 1;
                }
            }
        }
        assert!(seen > 0);
    }
}
