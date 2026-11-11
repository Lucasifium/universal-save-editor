//! Final Fantasy X-2 HD Remaster (PC) `ffx2_NNN`. Layout facts from FFX2SaveEditor
//! (Meth962): gil at 0x7818 (the load screen's copy at 0x14), the three characters'
//! records at 0x8204 (0x80 bytes each: experience, next level, HP, MP, max HP, max MP,
//! one unused byte, then strength, defense, magic, magic defense, agility, accuracy,
//! evasion, luck and level). A CRC-16 (polynomial 0x1021, start and final XOR 0xFFFF, and
//! the game's own table, whose last entry is 0)
//! of 0x40..0x1626C, worked out with its own place zeroed, is stored at 0x16268 and 0x1A.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const CRC_AT: usize = 0x16268;
const GIL: usize = 0x7818;
const CHARS: usize = 0x8204;

fn crc16(d: &[u8]) -> u16 {
    static TABLE: std::sync::OnceLock<[u16; 256]> = std::sync::OnceLock::new();
    let t = TABLE.get_or_init(|| {
        let mut t = [0u16; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = (i as u16) << 8;
            for _ in 0..8 {
                c = if c & 0x8000 != 0 { (c << 1) ^ 0x1021 } else { c << 1 };
            }
            *e = c;
        }
        // The game's table has 0 as its last entry (0x1EF0 in the standard one).
        t[255] = 0;
        t
    });
    d.iter().fold(0xFFFF, |c, &b| t[((c >> 8) as u8 ^ b) as usize] ^ (c << 8)) ^ 0xFFFF
}

/// The checksum with `zero` bytes of its own place cleared.
fn sum(d: &[u8], zero: usize) -> u16 {
    let mut v = d[0x40..CRC_AT + 4].to_vec();
    v[CRC_AT - 0x40..CRC_AT - 0x40 + zero].fill(0);
    crc16(&v)
}

fn stored(d: &[u8]) -> u16 {
    u16::from_le_bytes([d[CRC_AT], d[CRC_AT + 1]])
}

pub fn looks(d: &[u8]) -> bool {
    d.len() >= CRC_AT + 4 && d.len() < 0x40000 && {
        let s = stored(d);
        (s == sum(d, 2) || s == sum(d, 4)) && d[0x1A..0x1C] == s.to_le_bytes()
    }
}

fn fix(out: &mut Vec<u8>) {
    // As the game's own editor does: clear the whole 32-bit place, then store the 16-bit sum.
    out[CRC_AT..CRC_AT + 4].fill(0);
    let s = sum(out, 4).to_le_bytes();
    out[CRC_AT..CRC_AT + 2].copy_from_slice(&s);
    out[0x1A..0x1C].copy_from_slice(&s);
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Final Fantasy X-2 save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Gullwings", "Gil"], GIL, Kind::I32, 0, 9_999_999);
    b.mirror(&["Gullwings", "Gil"], 0x14);
    for (c, who) in ["Yuna", "Rikku", "Paine"].into_iter().enumerate() {
        let r = CHARS + 0x80 * c;
        let mut f = |k: &str, off: usize, kind: Kind, lo: i64, hi: i64| {
            b.ranged(&["Party", who, k], r + off, kind, lo, hi);
        };
        f("Level", 33, Kind::U8, 1, 99);
        f("Experience", 0, Kind::U32, 0, 99_999_999);
        f("HP", 8, Kind::U32, 0, 99_999);
        f("Max HP", 16, Kind::U32, 1, 99_999);
        f("MP", 12, Kind::U32, 0, 9_999);
        f("Max MP", 20, Kind::U32, 0, 9_999);
        for (i, s) in ["Strength", "Defense", "Magic", "Magic defense", "Agility", "Accuracy", "Evasion", "Luck"].into_iter().enumerate() {
            f(s, 25 + i, Kind::U8, 0, 255);
        }
    }
    Ok(b.finish("Final Fantasy X-2 save", Checksum::Custom(fix)).with_note(
        "Gil and Yuna, Rikku and Paine's level, experience, HP/MP and base stats. The save's checksum is updated \
         when you save.",
    ))
}

// ------------------------------------------------------------------ Final Fantasy X

/// Final Fantasy X (PC) `ffx_NNN`: the same CRC, over 0x40..0x64F8, stored at 0x64F4 and
/// 0x1A. Worked out from a sample save: gil at 0x3D88 (copy at 0x14), and 18 character
/// records of 0x94 bytes from 0x5610 (HP, MP, max HP and max MP at +0x18..+0x27; the
/// eight stats as the Sphere Grid has raised them, from +0x2B).
mod ffx {
    pub const CRC_AT: usize = 0x64F4;
    pub const LEN: usize = 0x6900;
    pub const GIL: usize = 0x3D88;
    pub const CHARS: usize = 0x5610;
    pub const NAMES: [&str; 18] = [
        "Tidus", "Yuna", "Auron", "Kimahri", "Wakka", "Lulu", "Rikku", "Seymour", "Valefor", "Ifrit", "Ixion", "Shiva", "Bahamut", "Anima",
        "Yojimbo", "Cindy", "Sandy", "Mindy",
    ];
    pub const MODES: [&str; 20] = [
        "Warrior", "Comrade", "Stoic", "Healer", "Tactician", "Victim", "Dancer", "Avenger", "Slayer", "Hero", "Rook", "Victor", "Coward",
        "Ally", "Sufferer", "Daredevil", "Loner", "(unused 17)", "(unused 18)", "Aeons only",
    ];
    pub const ITEM_SLOTS: usize = 112;
    pub const ITEM_IDS: usize = 0x3F0C;
    pub const ITEM_COUNTS: usize = 0x410C;
    pub const KEY_ITEMS: usize = 0x44CC;
    pub const MONSTERS: usize = 0x424C;
    /// Rows of the names table that start with `tag` (tab-separated; see data/ffx.txt).
    pub fn table(tag: char) -> impl Iterator<Item = Vec<&'static str>> {
        include_str!("data/ffx.txt").lines().filter(move |l| l.starts_with(tag)).map(|l| l.split('\t').collect())
    }
}

fn ffx_sum(d: &[u8]) -> u16 {
    let mut v = d[0x40..ffx::CRC_AT + 4].to_vec();
    v[ffx::CRC_AT - 0x40..ffx::CRC_AT - 0x40 + 2].fill(0);
    crc16(&v)
}

pub fn looks_ffx(d: &[u8]) -> bool {
    d.len() == ffx::LEN && {
        let s = u16::from_le_bytes([d[ffx::CRC_AT], d[ffx::CRC_AT + 1]]);
        s == ffx_sum(d) && d[0x1A..0x1C] == s.to_le_bytes()
    }
}

fn fix_ffx(out: &mut Vec<u8>) {
    let s = ffx_sum(out).to_le_bytes();
    out[ffx::CRC_AT..ffx::CRC_AT + 2].copy_from_slice(&s);
    out[0x1A..0x1C].copy_from_slice(&s);
}

pub fn parse_ffx(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks_ffx(d) {
        return Err("not a Final Fantasy X save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Party", "Gil"], ffx::GIL, Kind::I32, 0, 999_999_999);
    b.mirror(&["Party", "Gil"], 0x14);
    for (c, who) in ffx::NAMES.into_iter().enumerate() {
        let r = ffx::CHARS + 0x94 * c;
        let max_hp = u32::from_le_bytes(d[r + 0x20..r + 0x24].try_into().unwrap());
        if max_hp == 0 {
            continue;
        }
        let group = if c < 8 { "Characters" } else { "Aeons" };
        let mut f = |k: &str, off: usize, kind: Kind, lo: i64, hi: i64| {
            b.ranged(&[group, who, k], r + off, kind, lo, hi);
        };
        f("HP", 0x18, Kind::U32, 0, 99_999);
        f("MP", 0x1C, Kind::U32, 0, 9_999);
        f("Max HP", 0x20, Kind::U32, 1, 99_999);
        f("Max MP", 0x24, Kind::U32, 0, 9_999);
        for (i, s) in ["Strength", "Defense", "Magic", "Magic defense", "Agility", "Luck", "Evasion", "Accuracy"].into_iter().enumerate() {
            f(s, 0x2B + i, Kind::U8, 0, 255);
        }
        // What the character has without equipment: the game rebuilds the totals above from these.
        f("Base max HP", 0x0, Kind::U32, 1, 99_999);
        f("Base max MP", 0x4, Kind::U32, 0, 9_999);
        for (i, s) in ["Strength", "Defense", "Magic", "Magic defense", "Agility", "Luck", "Evasion", "Accuracy"].into_iter().enumerate() {
            f(&format!("Base {}", s.to_lowercase()), 0x8 + i, Kind::U8, 0, 255);
        }
        f("Overdrive gauge", 0x35, Kind::U8, 0, if c < 8 { 100 } else { 20 });
        if c < 8 {
            f("AP", 0x14, Kind::U32, 0, 99_999_999);
            f("Sphere levels", 0x37, Kind::U8, 0, 99);
            b.named(&[group, who, "Overdrive mode"], r + 0x34, Kind::U8, &ffx::MODES);
            for (i, m) in ffx::MODES.iter().enumerate().take(17) {
                b.bits(&["Overdrive modes", who, m], r + 0x84 + i / 8, Kind::U8, (i % 8) as u32, 1, 0, 0, 1);
            }
        }
        for l in ffx::table('A') {
            let (i, aeon_excluded) = (l[1].parse::<usize>().unwrap_or(0), l[4] == "1");
            if c >= 8 && aeon_excluded {
                continue;
            }
            b.bits(&["Abilities", who, l[2], l[3]], r + 0x3A + i / 8, Kind::U8, (i % 8) as u32, 1, 0, 0, 1);
        }
    }
    for l in ffx::table('O') {
        let (at, bit, c) = (l[1].parse::<usize>().unwrap_or(0), l[2].parse::<u32>().unwrap_or(0), l[3].parse::<usize>().unwrap_or(0));
        b.bits(&["Overdrives", ffx::NAMES[c], l[4]], at, Kind::U8, bit, 1, 0, 0, 1);
    }
    let mut items: Vec<(String, u64)> = vec![("(empty)".into(), 0xFF)];
    items.extend(ffx::table('I').map(|l| (l[3].to_string(), l[1].parse::<u64>().unwrap_or(0))));
    for i in 0..ffx::ITEM_SLOTS {
        let slot = format!("Slot {:03}", i + 1);
        b.pick(&["Items", &slot, "Item"], ffx::ITEM_IDS + 2 * i, Kind::U16, &items);
        b.ranged(&["Items", &slot, "Count"], ffx::ITEM_COUNTS + i, Kind::U8, 0, 99);
    }
    for l in ffx::table('K') {
        let i = l[1].parse::<usize>().unwrap_or(0);
        // The Al Bhed primers are counted in two more places, so they are left alone.
        if (4..30).contains(&i) {
            continue;
        }
        b.bits(&["Key items", l[2]], ffx::KEY_ITEMS + i / 8, Kind::U8, (i % 8) as u32, 1, 0, 0, 1);
    }
    for l in ffx::table('M') {
        let i = l[1].parse::<usize>().unwrap_or(0);
        b.ranged(&["Monster Arena", l[2], l[3]], ffx::MONSTERS + i, Kind::U8, 0, l[4].parse().unwrap_or(10));
    }
    Ok(b.finish("Final Fantasy X save", Checksum::Custom(fix_ffx)).with_note(
        "Gil, items, key items, every character's and aeon's HP, MP, stats, abilities and overdrives, and the Monster Arena captures. \
         The checksum is updated when you save.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_standard_check() {
        // CRC-16/GENIBUS check value (these bytes never reach the table's last entry).
        assert_eq!(crc16(b"123456789"), 0xD64E);
    }

    #[test]
    fn edit_and_checksum() {
        let mut d = vec![0u8; CRC_AT + 0x100];
        d[GIL..GIL + 4].copy_from_slice(&1500i32.to_le_bytes());
        d[CHARS + 33] = 7;
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Gullwings/Gil").unwrap(), 1500);
        assert_eq!(doc.view.pointer("/Party/Yuna/Level").unwrap(), 7);
        doc.set("/Gullwings/Gil", serde_json::Value::from(250000)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out), "the checksum was updated");
        assert_eq!(i32::from_le_bytes(out[0x14..0x18].try_into().unwrap()), 250000);
        assert_eq!(i32::from_le_bytes(out[GIL..GIL + 4].try_into().unwrap()), 250000);
    }
}
