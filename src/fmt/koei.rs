//! Koei Tecmo PC saves: Dynasty Warriors 8 Xtreme Legends (`save.dat`), One Piece: Pirate
//! Warriors 3 (`OP3WIN0000.dat`) and 4 (`OP4WINSLOT0000.dat`), and Persona 5 Strikers
//! (`SAVEDATA.BIN`).
//!
//! The first three share one envelope: a 16-bit word checksum and a 16-bit seed, then the data
//! XORed word by word with a linear-congruential stream (three steps per word for DW8XL and
//! PW3, one for PW4). DW8XL has a second layer, a per-byte stream, and a last byte that holds the
//! plain byte sum XORed with the seed. Persona 5 Strikers XORs bytes with a 24-bit generator
//! whose start is found from the file's known first four bytes, and keeps a byte sum at the end.
//!
//! Offsets and checks follow Sanrenan's Universal Koei Tecmo Save Editor (MIT).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Dw8xl,
    Pw3,
    Pw4,
    P5s,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wrap {
    pub game: Game,
    pub seed: u32,
}

const DW8_SIZE: usize = 0xb7f49;
const PW3_SIZE: usize = 0x135d04;
const PW4_SIZE: usize = 0x27161C;
const P5S_SIZE: usize = 0x55DEA0;
const DW8_INNER: u32 = 0x1310_0200;

pub const KIND_DW8: &str = "Dynasty Warriors 8: Xtreme Legends save";
pub const KIND_PW3: &str = "One Piece: Pirate Warriors 3 save";
pub const KIND_PW4: &str = "One Piece: Pirate Warriors 4 save";
pub const KIND_P5S: &str = "Persona 5 Strikers save";

fn word_sum(d: &[u8]) -> u16 {
    let mut s = 0u32;
    for c in d.chunks(2) {
        s = s.wrapping_add(c[0] as u32);
        if c.len() > 1 {
            s = s.wrapping_add((c[1] as u32) << 8);
        }
    }
    s as u16
}

fn step(s: u32) -> u32 {
    s.wrapping_mul(0x5b1a_7851).wrapping_add(0xce4e)
}

/// XOR whole words with the stream (`steps` generator steps per word). Symmetric.
fn word_cipher(d: &[u8], mut seed: u32, steps: usize) -> Vec<u8> {
    let mut out = d.to_vec();
    for c in out.chunks_exact_mut(4) {
        for _ in 0..steps {
            seed = step(seed);
        }
        let v = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) ^ seed;
        c.copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// DW8XL's per-byte layer. Symmetric.
fn byte_cipher(d: &[u8], mut seed: u32) -> Vec<u8> {
    d.iter()
        .map(|b| {
            seed = seed.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
            b ^ ((seed >> 16) & 0xff) as u8
        })
        .collect()
}

fn byte_sum(d: &[u8]) -> u8 {
    d.iter().fold(0u8, |a, b| a.wrapping_add(*b))
}

fn mix(s: u32) -> u32 {
    step(step(step(s)))
}

const P5S_MASK: u32 = (1 << 24) - 1;
const P5S_MUL: u32 = 0x41C6_4E6D;

fn p5s_xor(d: &[u8], mut s: u32) -> Vec<u8> {
    d.iter()
        .map(|b| {
            s = P5S_MUL.wrapping_mul(s).wrapping_add(12345) & P5S_MASK;
            b ^ ((s >> 16) & 0xff) as u8
        })
        .collect()
}

/// The generator start that turns the file's first four bytes into the known version bytes.
fn p5s_state(raw: &[u8]) -> Option<u32> {
    let known = [0x00u8, 0x20, 0x01, 0x20];
    let masks: Vec<u32> = (0..4).map(|i| (raw[i] ^ known[i]) as u32).collect();
    // inverse of the multiplier modulo 2^24
    let mut inv = 1u32;
    for _ in 0..5 {
        inv = inv.wrapping_mul(2u32.wrapping_sub(P5S_MUL.wrapping_mul(inv)));
    }
    inv &= P5S_MASK;
    let mut found = None;
    for low in 0..(1u32 << 16) {
        let first = (masks[0] << 16) | low;
        let mut t = first;
        if masks[1..].iter().all(|m| {
            t = P5S_MUL.wrapping_mul(t).wrapping_add(12345) & P5S_MASK;
            (t >> 16) & 0xff == *m
        }) {
            let start = first.wrapping_sub(12345).wrapping_mul(inv) & P5S_MASK;
            if found.replace(start).is_some() {
                return None;
            }
        }
    }
    found
}

fn u32_at(d: &[u8], at: usize) -> u32 {
    d.get(at..at + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

// ---- Pirate Warriors 4 object blocks
const PW4_PROFILE: (usize, usize) = (0x6B3C, 0x106DC);
const PW4_COINS: (usize, usize) = (0xB8908, 0x19480);
const PW4_COIN_STRIDE: usize = 0x30;
const PW4_COIN_COUNT: usize = 400;

fn pw4_block_ok(d: &[u8], (head, size): (usize, usize)) -> bool {
    d.len() >= head + size && u32_at(d, head + 4) as usize == size && u32_at(d, head) == pw4_sum(d, (head, size))
}

fn pw4_sum(d: &[u8], (head, size): (usize, usize)) -> u32 {
    d[head + 0x80..head + size].iter().fold(0u32, |a, b| a.wrapping_add(*b as u32)) & 0x7fff_ffff
}

fn pw4_fix(d: &mut Vec<u8>) {
    for blk in [PW4_PROFILE, PW4_COINS] {
        if d.len() >= blk.0 + blk.1 {
            let c = pw4_sum(d, blk);
            d[blk.0..blk.0 + 4].copy_from_slice(&c.to_le_bytes());
        }
    }
}

// ---- Persona 5 Strikers slots
const P5S_HEADER: usize = 0x1C;
const P5S_SLOT: usize = 0x88DF4 + 2 * 33;
const P5S_NAME: usize = 0x87842;

fn p5s_fix(d: &mut Vec<u8>) {
    let n = d.len();
    if n > 4 {
        d[n - 4] = byte_sum(&d[..n - 4]);
    }
}

fn p5s_occupied(d: &[u8], slot: usize) -> bool {
    let base = P5S_HEADER + slot * P5S_SLOT;
    if d[base..base + 2] == [0xff, 0xff] {
        return false;
    }
    [P5S_NAME, P5S_NAME + 33].iter().all(|off| {
        let name = &d[base + off..base + off + 33];
        match name.iter().position(|b| *b == 0) {
            Some(n) if name[0] != 0 => std::str::from_utf8(&name[..n]).is_ok_and(|s| !s.chars().any(|c| c.is_control())),
            _ => false,
        }
    })
}

/// Opens a file of one of the four games: (how to write it back, plain data).
pub fn unpack(raw: &[u8]) -> Option<(Wrap, Vec<u8>)> {
    let head = |len: usize| -> Option<(u16, u32)> {
        (raw.len() == len).then(|| (u16::from_le_bytes([raw[0], raw[1]]), u16::from_le_bytes([raw[2], raw[3]]) as u32))
    };
    match raw.len() {
        PW3_SIZE => {
            let (ck, seed) = head(PW3_SIZE)?;
            let p = word_cipher(&raw[4..], seed, 3);
            (word_sum(&p) == ck && p.starts_with(b"ONE PIECE: PIRATE WARRIORS 3\0")).then_some((Wrap { game: Game::Pw3, seed }, p))
        }
        DW8_SIZE => {
            let (ck, seed) = head(DW8_SIZE)?;
            let inner = word_cipher(&raw[4..DW8_SIZE - 1], seed, 3);
            if word_sum(&inner) != ck {
                return None;
            }
            let p = byte_cipher(&inner, DW8_INNER);
            let tail = raw[DW8_SIZE - 1] ^ (mix(seed) & 0xff) as u8;
            (byte_sum(&p) == tail && (p.starts_with(&[0xf0, 0x02, 0x10, 0x13, 0x09]) || p.starts_with(&[0xf0, 0x27, 0x02, 0x19, 0x09])))
                .then_some((Wrap { game: Game::Dw8xl, seed }, p))
        }
        PW4_SIZE => {
            let (ck, seed) = head(PW4_SIZE)?;
            let p = word_cipher(&raw[4..], seed, 1);
            (word_sum(&p) == ck && pw4_block_ok(&p, PW4_PROFILE) && pw4_block_ok(&p, PW4_COINS)).then_some((Wrap { game: Game::Pw4, seed }, p))
        }
        P5S_SIZE => {
            let state = p5s_state(raw)?;
            let n = raw.len();
            let mut p = p5s_xor(&raw[..n - 4], state);
            p.extend_from_slice(&raw[n - 4..]);
            (p[n - 4] == byte_sum(&p[..n - 4]) && p.starts_with(&[0x00, 0x20, 0x01, 0x20]) && p[n - 3..] == [0, 0, 0]).then_some((Wrap { game: Game::P5s, seed: state }, p))
        }
        _ => None,
    }
}

/// Closes the plain data again (checksums right).
pub fn pack(w: &Wrap, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut p = plain.to_vec();
    Ok(match w.game {
        Game::Pw3 => {
            let mut out = word_sum(&p).to_le_bytes().to_vec();
            out.extend_from_slice(&(w.seed as u16).to_le_bytes());
            out.extend(word_cipher(&p, w.seed, 3));
            out
        }
        Game::Dw8xl => {
            let inner = byte_cipher(&p, DW8_INNER);
            let mut out = word_sum(&inner).to_le_bytes().to_vec();
            out.extend_from_slice(&(w.seed as u16).to_le_bytes());
            out.extend(word_cipher(&inner, w.seed, 3));
            out.push(byte_sum(&p) ^ (mix(w.seed) & 0xff) as u8);
            out
        }
        Game::Pw4 => {
            pw4_fix(&mut p);
            let mut out = word_sum(&p).to_le_bytes().to_vec();
            out.extend_from_slice(&(w.seed as u16).to_le_bytes());
            out.extend(word_cipher(&p, w.seed, 1));
            out
        }
        Game::P5s => {
            p5s_fix(&mut p);
            let n = p.len();
            let mut out = p5s_xor(&p[..n - 4], w.seed);
            out.extend_from_slice(&p[n - 4..]);
            out
        }
    })
}

pub fn looks(d: &[u8]) -> bool {
    (d.len() == PW3_SIZE - 4 && d.starts_with(b"ONE PIECE: PIRATE WARRIORS 3\0"))
        || (d.len() == DW8_SIZE - 5 && (d.starts_with(&[0xf0, 0x02, 0x10, 0x13, 0x09]) || d.starts_with(&[0xf0, 0x27, 0x02, 0x19, 0x09])))
        || (d.len() == PW4_SIZE - 4 && pw4_block_ok(d, PW4_PROFILE) && pw4_block_ok(d, PW4_COINS))
        || (d.len() == P5S_SIZE && d.starts_with(&[0x00, 0x20, 0x01, 0x20]) && d[d.len() - 4] == byte_sum(&d[..d.len() - 4]))
}

const PW3_NAMES: [(usize, &str); 16] = [
    (0, "Luffy"),
    (1, "Zoro"),
    (2, "Nami"),
    (3, "Usopp"),
    (4, "Sanji"),
    (5, "Chopper"),
    (6, "Robin"),
    (9, "Ace"),
    (10, "Hancock"),
    (22, "Perona"),
    (26, "Law"),
    (28, "Tashigi"),
    (36, "Shanks"),
    (38, "Zoro (post-timeskip)"),
    (39, "Nami (post-timeskip)"),
    (43, "Robin (post-timeskip)"),
];

const DW8_NAMES: [&str; 42] = [
    "Zhao Yun", "Guan Yu", "Zhang Fei", "Xiahou Dun", "Dian Wei", "Xu Zhu", "Zhou Yu", "Lu Xun", "Taishi Ci", "Diaochan", "Zhuge Liang", "Cao Cao", "Lu Bu",
    "Sun Shangxiang", "Liu Bei", "Sun Jian", "Sun Quan", "Dong Zhuo", "Yuan Shao", "Ma Chao", "Huang Zhong", "Xiahou Yuan", "Zhang Liao", "Sima Yi", "Lu Meng",
    "Gan Ning", "Jiang Wei", "Zhang Jiao", "Xu Huang", "Zhang He", "Zhenji", "Huang Gai", "Sun Ce", "Wei Yan", "Pang Tong", "Meng Huo", "Zhurong", "Daqiao",
    "Xiaoqiao", "Fuxi", "Nuwa", "Ziluan",
];

const CONSUMABLES: [(usize, &str); 34] = [
    (0x869B2, "Adhesive Bandage"), (0x869B8, "Medicine"), (0x869DA, "Devil Fruit"), (0x869DC, "Life Stone"), (0x869DE, "Bead"), (0x869E0, "Bead Chain"),
    (0x869EE, "Soul Drop"), (0x869F0, "Snuff Soul"), (0x869F2, "Chewing Soul"), (0x869F4, "Soul Food"), (0x869F6, "Soma"), (0x86A02, "Revival Bead"),
    (0x86A04, "Balm of Life"), (0x86A06, "Rescue Pill"), (0x86A08, "Resuscitation Pill"), (0x86AA2, "Leblanc Coffee"), (0x86AA4, "Gyutan Stew"),
    (0x86AA6, "Salmon Bowl"), (0x86AA8, "Leblanc Curry"), (0x86AAA, "Miso Ramen"), (0x86AAC, "Jingisukan"), (0x86AAE, "Seafood Bowl"),
    (0x86AB0, "Okinawa Soba"), (0x86AB2, "Kyoto Curry"), (0x86AB4, "Goya Chanpuru"), (0x86AB6, "Goat Soup"), (0x86AB8, "Churrasco"),
    (0x86ABA, "Obanzai"), (0x86ABC, "Okonomiyaki"), (0x86ABE, "Crab Hot Pot"), (0x86AC0, "Osaka Sushi"), (0x86AC2, "Kushikatsu"),
    (0x86AC4, "Master Coffee"), (0x86AC8, "Master Curry"),
];

const INGREDIENTS: [(usize, &str); 20] = [
    (0x86B06, "Select Coffee Beans"), (0x86B08, "Salmon"), (0x86B0A, "Gyutan"), (0x86B0C, "Carrot"), (0x86B0E, "Onion"), (0x86B10, "Pork"), (0x86B12, "Rice"),
    (0x86B14, "Flour"), (0x86B16, "Miso"), (0x86B18, "Lamb Meat"), (0x86B1A, "Cabbage"), (0x86B1C, "Tuna"), (0x86B1E, "Egg"), (0x86B20, "Beef"),
    (0x86B22, "Cooking Awamori"), (0x86B24, "Goya"), (0x86B26, "Tofu"), (0x86B28, "Goat Meat"), (0x86B2A, "Kamo-Nasu Eggplant"), (0x86B2C, "Crab"),
];

const INCENSES: [(usize, &str); 7] = [
    (0x869C6, "Power Incense"), (0x869C8, "Magic Incense"), (0x869CA, "Guard Incense"), (0x869CC, "Speed Incense"), (0x869CE, "Luck Incense"),
    (0x869D0, "HP Incense"), (0x869D2, "SP Incense"),
];

const REMEDIES: [(usize, &str); 9] = [
    (0x86A16, "Hot and Sour Tea"), (0x86A18, "Super Jolt"), (0x86A1A, "Mental Floss"), (0x86A1C, "Donut-Worry"), (0x86A1E, "Soothing Towel"),
    (0x86A20, "Wide Eye Drops"), (0x86A22, "Repentance Ashes"), (0x86A24, "Hiranya"), (0x86A26, "Amrita Soda"),
];

const SKILL_CARDS: [(usize, &str); 24] = [
    (0x871CA, "Agi"), (0x871CC, "Agilao"), (0x871CE, "Agidyne"), (0x871D2, "Maragi"), (0x871DA, "Bufu"), (0x871DC, "Bufula"), (0x871DE, "Bufudyne"),
    (0x871E2, "Mabufu"), (0x871EA, "Zio"), (0x871EC, "Zionga"), (0x871EE, "Ziodyne"), (0x871F2, "Mazio"), (0x871FA, "Garu"), (0x871FC, "Garula"),
    (0x871FE, "Garudyne"), (0x87202, "Magaru"), (0x8720A, "Psi"), (0x87212, "Mapsi"), (0x8721A, "Frei"), (0x87222, "Mafrei"), (0x8722A, "Kouha"),
    (0x8723E, "Eiha"), (0x87272, "Dia"), (0x87278, "Media"),
];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut b = Builder::new(d);
    if d.len() == DW8_SIZE - 5 {
        b.ranged(&["Resources", "Gold"], 0x105, Kind::U32, 0, 9_999_999);
        b.ranged(&["Resources", "Gems"], 0x1d43, Kind::U16, 0, 9_999);
        b.ranged(&["Resources", "Facility materials"], 0x1f6d1, Kind::U16, 0, 9_999);
        b.ranged(&["Resources", "Weapon materials"], 0x1f6d3, Kind::U16, 0, 9_999);
        for i in 0..82usize {
            let name = DW8_NAMES.get(i).map_or(format!("Officer {:02}", i + 1), |n| n.to_string());
            let at = i * 0x48;
            b.ranged(&["Officers", &name, "Attack"], 0x7fd5 + at, Kind::U16, 0, 1500);
            b.ranged(&["Officers", &name, "Defence"], 0x7fd7 + at, Kind::U16, 0, 1500);
            b.ranged(&["Officers", &name, "Health"], 0x7fd9 + at, Kind::U16, 0, 1000);
            let stars: Vec<(String, u64)> = [25u64, 50, 75, 100].iter().enumerate().map(|(k, v)| (format!("{} star{}", k + 1, if k == 0 { "" } else { "s" }), *v)).collect();
            for (k, m) in ["Dash", "Dive", "Shadow Sprint", "Whirlwind"].iter().enumerate() {
                b.pick(&["Officers", &name, &format!("{m} compatibility")], 0x7fdb + at + k, Kind::U8, &stars);
            }
        }
        return Ok(b.finish(KIND_DW8, Checksum::None).with_note(crate::i18n::tr("Gold, gems, facility and weapon materials, each officer's attack, defence and health, and the weapon-compatibility stars. The checksums are updated when you save.")));
    }
    if d.len() == PW3_SIZE - 4 {
        b.info(&["Beli (read only)"], Value::from(u32_at(d, 0xc5d4)));
        for i in 0..47usize {
            let name = PW3_NAMES.iter().find(|(k, _)| *k == i).map_or(format!("Character {:02}", i + 1), |(_, n)| n.to_string());
            let at = i * 0x1f0;
            b.ranged(&["Characters", &name, "Health"], 0x654 + at, Kind::U16, 0, 10_000);
            b.ranged(&["Characters", &name, "Attack"], 0x656 + at, Kind::U16, 0, 1_000);
            b.ranged(&["Characters", &name, "Defence"], 0x658 + at, Kind::U16, 0, 1_000);
            b.ranged(&["Characters", &name, "Special bars"], 0x65a + at, Kind::U8, 1, 4);
            b.ranged(&["Characters", &name, "Skill slots"], 0x65c + at, Kind::U8, 1, 6);
        }
        return Ok(b.finish(KIND_PW3, Checksum::None).with_note(crate::i18n::tr("Each character's health, attack, defence, special bars and skill slots (a level-up may reset the stats). The checksum is updated when you save.")));
    }
    if d.len() == PW4_SIZE - 4 {
        b.ranged(&["Beli"], PW4_PROFILE.0 + 0x80 + 4, Kind::U32, 0, 999_999_999);
        let body = PW4_COINS.0 + 0x80;
        for id in 0..PW4_COIN_COUNT {
            let at = body + id * PW4_COIN_STRIDE;
            if u32_at(d, at) != 0 && d[at + 12] & 1 != 0 {
                b.ranged(&["Coins (quantity by coin id)", &format!("Coin {id:03}")], at + 8, Kind::U32, 0, 999);
            }
        }
        return Ok(b.finish(KIND_PW4, Checksum::Custom(pw4_fix)).with_note(crate::i18n::tr("Beli and the quantity of every coin you already own (by coin id). The object checksums are updated when you save.")));
    }
    if d.len() == P5S_SIZE {
        let groups: [(&str, &[(usize, &str)]); 5] = [
            ("Consumables", &CONSUMABLES),
            ("Cooking ingredients", &INGREDIENTS),
            ("Incenses", &INCENSES),
            ("Ailment remedies", &REMEDIES),
            ("Skill cards", &SKILL_CARDS),
        ];
        for slot in 1..10 {
            if !p5s_occupied(d, slot) {
                continue;
            }
            let base = P5S_HEADER + slot * P5S_SLOT;
            let s = format!("Save slot {slot}");
            b.ranged(&[&s, "Money"], base + 0x8788E, Kind::U32, 0, 9_999_999);
            b.ranged(&[&s, "Persona points"], base + 0x87892, Kind::U32, 0, 9_999_999);
            b.ranged(&[&s, "Unspent BOND points"], base + 0x876E2, Kind::U32, 0, 999);
            for (group, items) in groups {
                for (rel, name) in items {
                    let at = base + rel;
                    if d[at + 1] == 0 && (1..=99).contains(&d[at]) {
                        b.ranged(&[&s, group, name], at, Kind::U8, 1, 99);
                    }
                }
            }
        }
        if b.is_empty() {
            return Err("no occupied save slot".into());
        }
        return Ok(b.finish(KIND_P5S, Checksum::Custom(p5s_fix)).with_note(crate::i18n::tr("For each save slot: money, Persona points, BOND points and the stacks you already hold of consumables, cooking ingredients, incenses, remedies and skill cards. The checksum is updated when you save.")));
    }
    Err("not a Koei Tecmo save".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pw3_round_trip() {
        let mut p = vec![0u8; PW3_SIZE - 4];
        p[..29].copy_from_slice(b"ONE PIECE: PIRATE WARRIORS 3\0");
        p[0x654..0x656].copy_from_slice(&6000u16.to_le_bytes());
        let w = Wrap { game: Game::Pw3, seed: 0x1234 };
        let raw = pack(&w, &p).unwrap();
        let (w2, back) = unpack(&raw).unwrap();
        assert_eq!(w2, w);
        assert_eq!(back, p);
        let mut t = parse(&back).unwrap();
        t.set("/Characters/Luffy/Health", serde_json::json!(9000)).unwrap();
        let out = t.to_bytes().unwrap();
        assert_eq!(u16::from_le_bytes([out[0x654], out[0x655]]), 9000);
    }

    #[test]
    fn dw8_round_trip() {
        let mut p = vec![0u8; DW8_SIZE - 5];
        p[..5].copy_from_slice(&[0xf0, 0x02, 0x10, 0x13, 0x09]);
        let w = Wrap { game: Game::Dw8xl, seed: 0x00ab };
        let raw = pack(&w, &p).unwrap();
        assert_eq!(raw.len(), DW8_SIZE);
        let (_, back) = unpack(&raw).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn p5s_state_recovers() {
        let mut p = vec![0u8; P5S_SIZE];
        p[..4].copy_from_slice(&[0x00, 0x20, 0x01, 0x20]);
        p5s_fix(&mut p);
        let w = Wrap { game: Game::P5s, seed: 0x55_1234 };
        let raw = pack(&w, &p).unwrap();
        assert!(p5s_state(&raw).is_some());
    }
}
