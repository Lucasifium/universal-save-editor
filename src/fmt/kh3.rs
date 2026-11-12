//! Kingdom Hearts III (PC): the save container.
//!
//! The save (it starts with `S@vE`, its size at 4, a CRC-32 of `0x10..0x10+size` at 0x0C) is
//! encrypted with AES-256-ECB under a key made from the owner's account id (the SteamID64, or
//! `1638` for every Epic Games Store copy), followed by `0x08` and the MD5 of `0..size+16` of
//! the plain save. The account is found from the folder the save is in. On save the CRC and the
//! MD5 are worked out again.
//!
//! Ported from kh3-save-editor (thirteenth-order), GPL-3.0.

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use md5::{Digest, Md5};

pub const MAGIC: &[u8; 4] = b"S@vE";
const TRAILER: usize = 17;
const KEY_MASK: &[u8; 32] = b"hN96q4X9f%BCURBV&pMT4kcvqTMhHYD&";
const KEY_INDEX: &[u8] = b"ABCDE!#$%&FGHIJ012345KLMNOPqrstuvwxyzQRSTUVWXYZ6789abcdefgh},.<>ijklmnop()=~|-^+*;:[]{/?_@";
/// Every Epic Games Store copy uses this account id.
pub const EPIC: &str = "1638";

/// Munny: the balance, and the earned and spent totals it must stay equal to the difference of.
pub const MUNNY: usize = 0x28;
pub const MUNNY_EARNED: usize = 0x840;
pub const MUNNY_SPENT: usize = 0x844;

pub fn key(account: &str) -> [u8; 32] {
    let a = account.as_bytes();
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = KEY_INDEX[((KEY_MASK[i] ^ a[(i + 1) % a.len()]) % 0x5A) as usize];
    }
    k
}

fn size(plain: &[u8]) -> usize {
    u32::from_le_bytes(plain[4..8].try_into().unwrap()) as usize
}

/// Sets the CRC at 0x0C.
pub fn reseal(plain: &mut [u8]) -> Result<(), String> {
    if plain.len() < 0x20 || &plain[..4] != MAGIC {
        return Err("the save no longer starts with S@vE".into());
    }
    let fs = size(plain);
    if fs == 0 || fs + 0x10 > plain.len() {
        return Err("the save's size field no longer fits it".into());
    }
    let crc = crc32fast_ieee(&plain[0x10..0x10 + fs]);
    plain[0x0C..0x10].copy_from_slice(&crc.to_le_bytes());
    Ok(())
}

fn crc32fast_ieee(d: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in d {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

fn crc_ok(plain: &[u8]) -> bool {
    let fs = size(plain);
    fs != 0 && fs + 0x10 <= plain.len() && crc32fast_ieee(&plain[0x10..0x10 + fs]).to_le_bytes() == plain[0x0C..0x10]
}

/// Does `account` open this file? (One AES block.)
pub fn key_matches(blob: &[u8], account: &str) -> bool {
    if account.is_empty() || blob.len() < TRAILER + 16 {
        return false;
    }
    let c = aes::Aes256::new(&key(account).into());
    let mut b = aes::Block::clone_from_slice(&blob[..16]);
    c.decrypt_block(&mut b);
    &b[..4] == MAGIC
}

/// Account ids to try for a save at `path`: the folder `SaveGames` sits in, then every
/// all-digit folder, innermost first, then extra ones, then the Epic id.
pub fn candidates(path: &str, extra: &[String]) -> Vec<String> {
    let parts: Vec<&str> = path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    let mut out: Vec<String> = vec![];
    let mut add = |s: &str| {
        if !s.is_empty() && !out.iter().any(|o| o == s) {
            out.push(s.to_string());
        }
    };
    for w in parts.windows(2) {
        if w[1].eq_ignore_ascii_case("SaveGames") {
            add(w[0]);
        }
    }
    for p in parts.iter().rev() {
        if p.parse::<u64>().is_ok() {
            add(p);
        }
    }
    for e in extra {
        add(e);
    }
    add(EPIC);
    out
}

/// Decrypts and checks a PC save. Returns the account id used and the plain save.
pub fn open(blob: &[u8], accounts: &[String]) -> Option<(String, Vec<u8>)> {
    if blob.len() <= TRAILER || (blob.len() - TRAILER) % 16 != 0 || blob[blob.len() - TRAILER] != 0x08 {
        return None;
    }
    let account = accounts.iter().find(|a| key_matches(blob, a))?.clone();
    let c = aes::Aes256::new(&key(&account).into());
    let mut plain = blob[..blob.len() - TRAILER].to_vec();
    for chunk in plain.chunks_exact_mut(16) {
        c.decrypt_block(aes::Block::from_mut_slice(chunk));
    }
    let fs = size(&plain);
    if fs == 0 || fs + 24 > plain.len() {
        return None;
    }
    let sum = Md5::digest(&plain[..fs + 16]);
    (sum[..] == blob[blob.len() - 16..] && crc_ok(&plain)).then_some((account, plain))
}

/// Encrypts a plain save again, with a fresh CRC and MD5.
pub fn close(account: &str, plain: &[u8]) -> Result<Vec<u8>, String> {
    if plain.len() % 16 != 0 {
        return Err("the save's length must stay a multiple of 16".into());
    }
    let mut out = plain.to_vec();
    reseal(&mut out)?;
    let fs = size(&out);
    let sum = Md5::digest(&out[..fs + 16]);
    let c = aes::Aes256::new(&key(account).into());
    for chunk in out.chunks_exact_mut(16) {
        c.encrypt_block(aes::Block::from_mut_slice(chunk));
    }
    out.push(0x08);
    out.extend_from_slice(&sum);
    Ok(out)
}


pub const KIND: &str = "Kingdom Hearts III save";
const CHARS: [&str; 15] = ["Sora", "Donald", "Goofy", "Hercules", "Woody", "Buzz", "Rapunzel", "Flynn", "Sulley", "Mike", "Marshmallow", "Baymax", "Jack", "Riku", "Mickey"];
const CHAR_BASE: usize = 0x1880;
const CHAR_SIZE: usize = 0x9C0;
const INVENTORY: usize = 0x8F4;
const MATERIAL: usize = 0x165E;
/// The game's own cap is unstated; 99 is what its menus show for a full stack.
pub const MATERIAL_CAP: i64 = 99;

/// A plain (decrypted) save, which the game screen reads.
pub fn looks(d: &[u8]) -> bool {
    d.len() >= CHAR_BASE + 16 * CHAR_SIZE && &d[..4] == MAGIC && crc_ok(d)
}

/// The game screen's values. Munny is kept equal to earned minus spent, as the game checks.
pub fn parse(d: &[u8]) -> Result<crate::tree::TreeDoc, String> {
    use super::fields::{Builder, Checksum, Kind};
    if !looks(d) {
        return Err("not a Kingdom Hearts III save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Sora", "Munny"], MUNNY, Kind::U32, 0, 9_999_999);
    b.ranged(&["Sora", "Level"], 0x2C, Kind::U8, 1, 99);
    b.field(&["Sora", "Total EXP"], 0x24, Kind::U32);
    b.field(&["Sora", "Play time (seconds)"], 0x20, Kind::U32);
    b.field(&["Sora", "Enemies defeated"], 0x70, Kind::U32);
    b.field(&["Sora", "Crabs collected"], 0x17EC, Kind::I32);
    for (k, at) in [("Bonus HP", 0xB49C), ("Bonus MP", 0xB4A0), ("Bonus strength", 0xB4A4), ("Bonus magic", 0xB4A8), ("Bonus defense", 0xB4AC)] {
        b.field(&["Sora", k], at, Kind::I32);
    }
    let diff = ["Beginner", "Standard", "Proud", "Critical"];
    b.info(&["Save", "Difficulty"], diff.get(d[0x14] as usize).copied().unwrap_or("?"));
    b.info(&["Save", "Munny earned"], u32::from_le_bytes(d[MUNNY_EARNED..MUNNY_EARNED + 4].try_into().unwrap()));
    b.info(&["Save", "Munny spent"], u32::from_le_bytes(d[MUNNY_SPENT..MUNNY_SPENT + 4].try_into().unwrap()));
    for (i, name) in CHARS.iter().enumerate() {
        let base = CHAR_BASE + i * CHAR_SIZE;
        b.field(&["Characters", name, "HP"], base + 0x984, Kind::I32);
        b.field(&["Characters", name, "MP"], base + 0x988, Kind::I32);
        b.field(&["Characters", name, "Focus"], base + 0x98C, Kind::I32);
        b.field(&["Characters", name, "Strength boosts"], base + 0x980, Kind::U8);
        b.field(&["Characters", name, "Magic boosts"], base + 0x981, Kind::U8);
        b.field(&["Characters", name, "Defense boosts"], base + 0x982, Kind::U8);
        b.field(&["Characters", name, "AP boosts"], base + 0x983, Kind::U8);
    }
    for (i, name) in super::kh3_tables::MATERIALS.iter().enumerate() {
        b.ranged(&["Synthesis materials", name], MATERIAL + i * 2, Kind::U16, 0, 999);
    }
    // Items the save already holds (their "obtained" flag is set), by name.
    let mut seen = std::collections::HashSet::new();
    for &(id, name) in super::kh3_tables::ITEMS {
        let at = INVENTORY + id as usize * 2;
        if d[at + 1] & 1 == 0 {
            continue;
        }
        let key = if seen.insert(name) { name.to_string() } else { format!("{name} ({id})") };
        b.ranged(&["Items", &key], at, Kind::U8, 0, 99);
    }
    Ok(b.finish(KIND, Checksum::Touched(keep_ledger)).with_note(
        "Munny, level and experience, each character's HP, MP and boosts, synthesis materials and the items already in the inventory. Munny earned is moved with munny so the two totals still agree.",
    ))
}

/// After munny is changed, earned = munny + spent (spent is lowered when that won't fit);
/// then the CRC (for a save that was already decrypted; an encrypted one gets it again anyway).
fn keep_ledger(d: &mut Vec<u8>, touched: &[usize]) {
    if touched.contains(&MUNNY) {
        ledger(d);
    }
    let _ = reseal(d);
}

fn ledger(d: &mut [u8]) {
    let rd = |d: &[u8], at: usize| u32::from_le_bytes(d[at..at + 4].try_into().unwrap());
    let munny = rd(d, MUNNY);
    let mut spent = rd(d, MUNNY_SPENT);
    let earned = match munny.checked_add(spent) {
        Some(e) => e,
        None => {
            spent = u32::MAX - munny;
            u32::MAX
        }
    };
    d[MUNNY_EARNED..MUNNY_EARNED + 4].copy_from_slice(&earned.to_le_bytes());
    d[MUNNY_SPENT..MUNNY_SPENT + 4].copy_from_slice(&spent.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    /// kh3-save-editor's frozen synthetic fixture (internal/fixture: no real save is used).
    fn fixture(difficulty: u8, level: u8) -> Vec<u8> {
        let mut p = vec![0u8; 0x20000];
        let fs: u32 = 0x20000 - 24;
        p[..4].copy_from_slice(MAGIC);
        p[4..8].copy_from_slice(&fs.to_le_bytes());
        p[8..10].copy_from_slice(&5u16.to_le_bytes());
        p[10..12].copy_from_slice(&2u16.to_le_bytes());
        p[0x14] = difficulty;
        p[0x18] = 4;
        p[0x20..0x24].copy_from_slice(&5896u32.to_le_bytes());
        p[0x24..0x28].copy_from_slice(&1896u32.to_le_bytes());
        p[0x28..0x2C].copy_from_slice(&583u32.to_le_bytes());
        p[0x2C] = level;
        p[0x54] = 67;
        p[0x70..0x74].copy_from_slice(&276u32.to_le_bytes());
        p[0x5B8..0x5BA].copy_from_slice(&7u16.to_le_bytes());
        let set = |p: &mut Vec<u8>, ci: usize, off: usize, v: u32| {
            let o = 0x1880 + ci * 0x9C0 + off;
            p[o..o + 4].copy_from_slice(&v.to_le_bytes());
        };
        for ci in 0..16 {
            for aid in 0..512 {
                set(&mut p, ci, 0x160 + aid * 4, 0x444);
            }
            set(&mut p, ci, 0x160 + 0x003 * 4, 0x44B);
            set(&mut p, ci, 0x160 + 0x01A * 4, 0x449);
            set(&mut p, ci, 0x160 + 0x01C * 4, 0x8443);
            set(&mut p, ci, 0x984, 145);
            set(&mut p, ci, 0x988, 115);
            set(&mut p, ci, 0x98C, 100);
        }
        if difficulty == 3 {
            for aid in [0x68, 0x69, 0x6A] {
                set(&mut p, 0, 0x160 + aid * 4, 0x44B);
            }
        }
        for (id, n, f) in [(1usize, 5u8, 3u8), (21, 2, 3), (227, 1, 0x0D)] {
            p[0x8F4 + id * 2] = n;
            p[0x8F4 + id * 2 + 1] = f;
        }
        let s1 = b"/Game/Levels/he/he_02/he_02";
        p[0xBBA0..0xBBA0 + s1.len()].copy_from_slice(s1);
        let s2 = b"he_02_Lv_Save_02";
        p[0xBCA0..0xBCA0 + s2.len()].copy_from_slice(s2);
        reseal(&mut p).unwrap();
        p
    }

    fn sha(b: &[u8]) -> String {
        Sha256::digest(b).iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The golden vectors kh3-save-editor froze from an independent implementation.
    #[test]
    fn golden_vectors() {
        let acc = "76561190000000000";
        let p0 = fixture(1, 6);
        assert_eq!(sha(&p0), "9f55ebdec9c15a967f6c332ecbc421736b68f7caee5a789adbc66acb06cf5d63");
        assert_eq!(sha(&fixture(3, 7)), "8a9f5946a47a6ce389214b98d629f56df019610f18bb53e9745c39ad3b9058be");
        let blob = close(acc, &p0).unwrap();
        assert_eq!(sha(&blob), "e36102abb1681d4c7f140aa439d84be77433957b12c0a5eaccb75da67a6fd9a7");
        let rekeyed = close("76561190000000009", &p0).unwrap();
        assert_eq!(sha(&rekeyed), "79760f204c62a17f6aaeebdf56b6459a41e9618f4a2ce44deee70f9f02299154");
        // Opened from its Steam folder, and back the same.
        let path = format!("C:/Users/x/Documents/My Games/KINGDOM HEARTS III/Steam/{acc}/SaveGames/slot0.sav");
        let (a, plain) = open(&blob, &candidates(&path, &[])).unwrap();
        assert_eq!((a.as_str(), &plain), (acc, &p0));
        assert_eq!(close(&a, &plain).unwrap(), blob);
        assert!(open(&blob, &candidates("slot0.sav", &[])).is_none());
    }

    #[test]
    fn screen_values_and_ledger() {
        let mut p = vec![0u8; CHAR_BASE + 16 * CHAR_SIZE];
        p[..4].copy_from_slice(MAGIC);
        let fs = (p.len() - 16) as u32;
        p[4..8].copy_from_slice(&fs.to_le_bytes());
        p[MUNNY..MUNNY + 4].copy_from_slice(&583u32.to_le_bytes());
        p[MUNNY_EARNED..MUNNY_EARNED + 4].copy_from_slice(&1183u32.to_le_bytes());
        p[MUNNY_SPENT..MUNNY_SPENT + 4].copy_from_slice(&600u32.to_le_bytes());
        p[INVENTORY + 2] = 5;
        p[INVENTORY + 3] = 3;
        reseal(&mut p).unwrap();
        let mut doc = parse(&p).unwrap();
        assert_eq!(doc.view.pointer("/Items/Potion"), Some(&serde_json::Value::from(5)));
        assert!(doc.view.pointer("/Items/Ether").is_none());
        doc.set("/Sora/Munny", serde_json::Value::from(9000)).unwrap();
        doc.set("/Synthesis materials/Blazing Shard", serde_json::Value::from(99)).unwrap();
        let out = doc.to_bytes().unwrap();
        let rd = |at: usize| u32::from_le_bytes(out[at..at + 4].try_into().unwrap());
        assert_eq!((rd(MUNNY), rd(MUNNY_EARNED), rd(MUNNY_SPENT)), (9000, 9600, 600));
        assert_eq!(u16::from_le_bytes([out[MATERIAL], out[MATERIAL + 1]]), 99);
    }

    #[test]
    fn opens_on_the_game_screen_from_its_folder() {
        let acc = "76561190000000000";
        let blob = close(acc, &fixture(1, 6)).unwrap();
        let path = format!("/home/x/Documents/KINGDOM HEARTS III/Steam/{acc}/SaveGames/kh3sv2/data/KHIII_slot0.bin");
        let d = crate::formats::decode_with(&blob, &crate::formats::Options::for_path(std::path::Path::new(&path)));
        assert_eq!(d.layers, vec![crate::formats::Layer::Kh3 { account: acc.into() }]);
        let crate::formats::Content::Tree(t) = &d.content else { panic!("{:?}", d.hint) };
        assert_eq!(t.kind_name, KIND);
        assert_eq!(t.view.pointer("/Sora/Munny"), Some(&serde_json::Value::from(583)));
        assert_eq!(t.view.pointer("/Items/Potion"), Some(&serde_json::Value::from(5)));
        assert_eq!(crate::formats::encode(&d.layers, &d.content).unwrap(), blob);
    }
}
