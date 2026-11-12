//! Final Fantasy XV (Windows Edition) `gameplay0.save`.
//!
//! Outer layer (every `.save` of the game): the last 53 bytes are a plain footer (IV, a
//! 16-byte tweak, a Mersenne Twister seed, 16 zero bytes, 0x02). The body is AES-192-CBC with
//! a fixed key, then its 16-byte blocks are shuffled by the seeded generator, then the tweak is
//! added byte by byte. Packing again with the same footer gives the same bytes back.
//!
//! Inside: a 24-byte header (the fourth u32 points at a schema at the end of the file), then
//! the game's structures one after another, each starting with its 8-byte id. The schema lists
//! every structure's name, id and members (name, type, size); members are written packed,
//! with runs of `bool`s sharing bytes as bits. Values are found by structure id and edited in
//! place, so nothing moves.
//!
//! Facts from Surihix/FFXVSaveCrypt (encryption, facts only) and the gameplay0.save template
//! in Kizari/FFXVTemplates (MIT); checked on sample saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};

pub const KIND: &str = "Final Fantasy XV save";

const KEY: [u8; 24] = [
    0x54, 0x5f, 0xd4, 0x9b, 0xce, 0x5d, 0xc2, 0x76, 0xa5, 0x1a, 0xcb, 0x86, 0x44, 0x79, 0x1d, 0x6d, 0xf4, 0xb7, 0xe7, 0xb1, 0x33, 0x0c, 0x3f, 0x1d,
];
const FOOTER: usize = 53;

// ------------------------------------------------------------------ encryption

struct Mt {
    s: [u32; 624],
    i: usize,
}

impl Mt {
    fn new(seed: u32) -> Self {
        let mut s = [0u32; 624];
        s[0] = seed;
        for i in 1..624 {
            s[i] = 1_812_433_253u32.wrapping_mul(s[i - 1] ^ (s[i - 1] >> 30)).wrapping_add(i as u32);
        }
        Mt { s, i: 624 }
    }
    fn next(&mut self) -> u32 {
        if self.i >= 624 {
            for k in 0..624 {
                let y = (self.s[k] & 0x8000_0000) | (self.s[(k + 1) % 624] & 0x7fff_ffff);
                self.s[k] = self.s[(k + 397) % 624] ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 };
            }
            self.i = 0;
        }
        let mut y = self.s[self.i];
        self.i += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }
}

/// The block each block is swapped with.
fn swaps(seed: u32, n: usize) -> Vec<usize> {
    let mut ks = vec![0usize; n];
    let mut mt = Mt::new(seed);
    for i in (2..n).rev() {
        let lim = u32::MAX - (u32::MAX % i as u32);
        loop {
            let v = mt.next();
            if v < lim {
                ks[i] = (v % i as u32) as usize;
                break;
            }
        }
    }
    ks
}

fn swap_blocks(b: &mut [u8], i: usize, j: usize) {
    if i != j {
        let (lo, hi) = (i.min(j), i.max(j));
        let (a, c) = b.split_at_mut(hi * 16);
        a[lo * 16..lo * 16 + 16].swap_with_slice(&mut c[..16]);
    }
}

fn footer_ok(d: &[u8]) -> bool {
    d.len() >= FOOTER + 64 && (d.len() - FOOTER) % 16 == 0 && d[d.len() - 1] == 2 && d[d.len() - 17..d.len() - 1].iter().all(|&b| b == 0)
}

/// Decrypts a Final Fantasy XV `.save`: (plain data, footer to pack it again with).
pub fn decrypt(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let (plain, foot) = decrypt_with(&KEY, d)?;
    // The header: format version 4 and a zero, then the schema's offset.
    let le = |at: usize| u32::from_le_bytes(plain[at..at + 4].try_into().unwrap()) as usize;
    (le(4) == 4 && le(8) == 0 && le(12) < plain.len()).then_some((plain, foot))
}

/// The outer layer alone, with the game's own key (Forspoken uses the same layer).
pub fn decrypt_with(key: &[u8; 24], d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if !footer_ok(d) {
        return None;
    }
    let (body, foot) = d.split_at(d.len() - FOOTER);
    let iv: [u8; 16] = foot[..16].try_into().unwrap();
    let tweak = &foot[16..32];
    let seed = u32::from_le_bytes(foot[32..36].try_into().unwrap());
    let mut b: Vec<u8> = body.iter().enumerate().map(|(i, &x)| x.wrapping_sub(tweak[i & 15])).collect();
    let n = b.len() / 16;
    let ks = swaps(seed, n);
    for i in 1..n {
        swap_blocks(&mut b, i, ks[i]);
    }
    let plain = cbc::Decryptor::<aes::Aes192>::new(&(*key).into(), &iv.into())
        .decrypt_padded_mut::<aes::cipher::block_padding::NoPadding>(&mut b)
        .ok()?
        .to_vec();
    Some((plain, foot.to_vec()))
}

pub fn encrypt(plain: &[u8], foot: &[u8]) -> Result<Vec<u8>, String> {
    encrypt_with(&KEY, plain, foot)
}

pub fn encrypt_with(key: &[u8; 24], plain: &[u8], foot: &[u8]) -> Result<Vec<u8>, String> {
    if plain.len() % 16 != 0 || foot.len() != FOOTER {
        return Err("Final Fantasy XV saves must stay a whole number of 16-byte blocks".into());
    }
    let iv: [u8; 16] = foot[..16].try_into().unwrap();
    let tweak = &foot[16..32];
    let seed = u32::from_le_bytes(foot[32..36].try_into().unwrap());
    let mut b = cbc::Encryptor::<aes::Aes192>::new(&(*key).into(), &iv.into()).encrypt_padded_vec_mut::<aes::cipher::block_padding::NoPadding>(plain);
    let n = b.len() / 16;
    let ks = swaps(seed, n);
    for i in (1..n).rev() {
        swap_blocks(&mut b, i, ks[i]);
    }
    for (i, x) in b.iter_mut().enumerate() {
        *x = x.wrapping_add(tweak[i & 15]);
    }
    b.extend_from_slice(foot);
    Ok(b)
}

// ------------------------------------------------------------------ schema

struct Member {
    name: String,
    ty: String,
    size: usize,
    count: usize,
}

struct Schema {
    /// Name, 8-byte id and members of every structure.
    structs: Vec<(String, [u8; 8], Vec<Member>)>,
    /// Where the data ends (the schema starts).
    end: usize,
}

fn schema(d: &[u8]) -> Option<Schema> {
    let u32_at = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let end = u32_at(12)?;
    let mut o = end;
    let n = u32_at(o)?;
    if n == 0 || n > 10_000 {
        return None;
    }
    o += 4;
    let s = |o: &mut usize| -> Option<String> {
        let l = u32_at(*o)?;
        let t = d.get(*o + 4..*o + 4 + l)?;
        *o += 4 + l;
        Some(String::from_utf8_lossy(t).trim_end_matches('\0').to_string())
    };
    let mut structs = vec![];
    for _ in 0..n {
        let name = s(&mut o)?;
        let id: [u8; 8] = d.get(o..o + 8)?.try_into().ok()?;
        o += 16;
        let mc = u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?) as usize;
        o += 2;
        let mut members = vec![];
        for _ in 0..mc {
            let mname = s(&mut o)?;
            let ty = s(&mut o)?;
            let size = u32_at(o + 4)?;
            let count = u16::from_le_bytes(d.get(o + 8..o + 10)?.try_into().ok()?) as usize;
            o += 13;
            members.push(Member { name: mname, ty, size, count });
        }
        structs.push((name, id, members));
    }
    Some(Schema { structs, end })
}

impl Schema {
    fn get(&self, name: &str) -> Option<&(String, [u8; 8], Vec<Member>)> {
        self.structs.iter().find(|s| s.0 == name)
    }
}

/// Where each leading plain member of a structure sits in the file, from the start of its data
/// (after the 8-byte id): runs of bools share bytes (8 to a byte), numbers are packed. Stops at
/// the first member that isn't a single number or bool.
fn layout(members: &[Member]) -> Vec<(&str, usize, Kind)> {
    let mut out = vec![];
    let (mut at, mut bits) = (0usize, 0u32);
    for m in members {
        if m.ty == "bool" && m.count == 1 {
            if bits == 0 || bits == 8 {
                at += 1;
                bits = 0;
            }
            bits += 1;
            continue;
        }
        bits = 0;
        let kind = match (m.ty.as_str(), m.size) {
            ("int32_t" | "int" | "SQEX.Ebony.Std.Fixid", 4) => Kind::I32,
            ("uint32_t", 4) => Kind::U32,
            ("float", 4) => Kind::F32,
            (t, 4) if t.starts_with("Black.") && m.count == 1 => Kind::I32,
            ("int64_t" | "uint64_t", 8) => Kind::U64,
            _ => break,
        };
        if m.count != 1 {
            break;
        }
        out.push((m.name.as_str(), at, kind));
        at += kind.size();
    }
    out
}

fn find_all(d: &[u8], id: &[u8; 8], end: usize) -> Vec<usize> {
    let mut v = vec![];
    let mut i = 24;
    while i + 8 <= end {
        match d[i..end].windows(8).position(|w| w == id) {
            Some(p) => {
                v.push(i + p);
                i += p + 8;
            }
            None => break,
        }
    }
    v
}

fn find_first(d: &[u8], id: &[u8; 8], from: usize, end: usize) -> Option<usize> {
    d.get(from..end)?.windows(8).position(|w| w == id).map(|p| from + p)
}

fn i32_at(d: &[u8], at: usize) -> i64 {
    i32::from_le_bytes(d[at..at + 4].try_into().unwrap()) as i64
}

// ------------------------------------------------------------------ reader

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && u32::from_le_bytes(d[4..8].try_into().unwrap()) == 4 && schema(d).is_some_and(|s| s.get("Black.Save.Party.SavePartyDataStruct").is_some())
}

/// The members of a structure the screen reads, as (name in the tree, member).
const PARTY: [(&str, &str, i64); 3] = [("Gil", "gil", 99_999_999), ("Ability points", "ability_point", 9_999), ("Medals", "medal", 99_999_999)];
const STATS: [(&str, &str); 11] = [
    ("Level", "level"),
    ("HP", "hp"),
    ("Max HP", "hp_max"),
    ("MP", "mp"),
    ("Max MP", "mp_max"),
    ("Strength", "strength"),
    ("Vitality", "vitality"),
    ("Magic", "magic"),
    ("Spirit", "spirit"),
    ("Critical", "critical"),
    ("Party slot", "party_index"),
];

/// Item lists in `SaveItemStruct`, in order (each: u32 count, then {id, item id, amount}).
const ITEM_LISTS: [&str; 8] = ["Battle items", "Key items", "Food and ingredients", "Treasures", "Car items", "Leisure items", "Upgrade items", "Trunk"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let s = schema(d).ok_or("not a Final Fantasy XV gameplay save")?;
    let mut b = Builder::new(d);
    let le32 = |at: usize| u32::from_le_bytes(d[at..at + 4].try_into().unwrap());

    // Load-screen summary.
    if let Some((_, id, m)) = s.get("Black.Save.SaveInformation") {
        if let Some(at) = find_first(d, id, 24, s.end) {
            // Starts with SaveType, then a string (u32 length + text); the rest are numbers.
            let base = at + 8 + 4;
            let len = le32(base) as usize;
            let rest = base + 4 + len;
            let after: Vec<&Member> = m.iter().skip_while(|x| x.name != "AreaFilePath").skip(1).collect();
            let mut at2 = rest;
            for x in after {
                match x.name.as_str() {
                    "PlayerLevel" => b.info(&["Save", "Level shown"], le32(at2)),
                    "ChapterNo" => b.info(&["Save", "Chapter"], le32(at2) as i32),
                    "PlayTime" => {
                        let t = u64::from_le_bytes(d[at2..at2 + 8].try_into().unwrap());
                        b.info(&["Save", "Play time"], format!("{}:{:02}", t / 3600, t / 60 % 60));
                        break;
                    }
                    _ => {}
                }
                // Positions are four doubles, the rotation four floats; every other member before
                // the play time is four bytes.
                at2 += match x.ty.as_str() {
                    "Luminous.Math.DoubleVector4" => 32,
                    "Luminous.Math.VectorA" => 16,
                    _ => 4,
                };
            }
        }
    }

    // Party money and points.
    let (_, pid, pm) = s.get("Black.Save.Party.SavePartyDataStruct").ok_or("no party data")?;
    let pat = find_first(d, pid, 24, s.end).ok_or("no party data")? + 8;
    let pl = layout(pm);
    for (label, member, hi) in PARTY {
        if let Some((_, off, kind)) = pl.iter().find(|x| x.0 == member) {
            b.ranged(&["Party", label], pat + off, *kind, 0, hi);
        }
    }

    // Characters: SavePlayerStruct holds the name, then SaveStatusStruct, then SavePlayerStatusStruct.
    let (_, plid, _) = s.get("Black.Save.Player.SavePlayerStruct").ok_or("no characters")?;
    let (_, stid, stm) = s.get("Black.Save.Player.SaveStatusStruct").ok_or("no characters")?;
    let (_, psid, psm) = s.get("Black.Save.Player.SavePlayerStatusStruct").ok_or("no characters")?;
    let (sl, psl) = (layout(stm), layout(psm));
    for at in find_all(d, plid, s.end) {
        // object, character and default character entry ids, then the name (u32 length + bytes).
        let nat = at + 8 + 12;
        let len = le32(nat) as usize;
        let Some(raw) = d.get(nat + 4..nat + 4 + len) else { continue };
        let name = String::from_utf8_lossy(&raw[..raw.iter().position(|&c| c == 0).unwrap_or(raw.len())]).to_string();
        if name.is_empty() {
            continue;
        }
        let (Some(st), Some(ps)) = (find_first(d, stid, nat, s.end), find_first(d, psid, nat, s.end)) else { continue };
        // Both follow the name directly; anything farther away belongs to someone else.
        if st > nat + 4 + len + 64 || ps > st + 4096 {
            continue;
        }
        let (st, ps) = (st + 8, ps + 8);
        let joined = sl.iter().any(|x| x.0 == "party_index") && i32_at(d, st + sl.iter().find(|x| x.0 == "party_index").unwrap().1) >= 0;
        let group = if joined { "Party" } else { "Others" };
        let base = ["Characters", group, name.as_str()];
        fn path<'a>(base: &[&'a str; 3], k: &'a str) -> Vec<&'a str> {
            base.iter().copied().chain([k]).collect()
        }
        for (label, member) in STATS {
            if let Some((_, off, _)) = sl.iter().find(|x| x.0 == member) {
                b.info(&path(&base, label), i32_at(d, st + off));
            }
        }
        for (label, member, hi) in [("Experience", "exp", 99_999_999i64), ("Total experience", "exp_total", -1), ("Kills", "kill_count", -1)] {
            if let Some((_, off, kind)) = psl.iter().find(|x| x.0 == member) {
                if hi > 0 {
                    b.ranged(&path(&base, label), ps + off, *kind, 0, hi);
                } else {
                    b.info(&path(&base, label), i32_at(d, ps + off));
                }
            }
        }
    }

    // Items.
    if let Some((_, iid, _)) = s.get("Black.Save.Item.SaveItemStruct") {
        if let Some(at) = find_first(d, iid, 24, s.end) {
            let mut o = at + 8;
            for list in ITEM_LISTS {
                let Some(n) = d.get(o..o + 4).map(|_| le32(o) as usize) else { break };
                if n > 4096 || o + 4 + 16 * n > s.end {
                    break;
                }
                o += 4;
                for k in 0..n {
                    let e = o + 16 * k;
                    let id = le32(e + 8);
                    if id != 0 && i32_at(d, e + 12) > 0 {
                        let key = format!("Item {id}");
                        b.ranged(&["Items", list, &key], e + 12, Kind::I32, 0, 999);
                    }
                }
                o += 16 * n;
            }
        }
    }

    Ok(b.finish(KIND, Checksum::None).with_note(
        "Gil, ability points, medals, each character's experience waiting to be banked (spend it by resting) and item amounts. Levels, HP and stats are shown; the game works them out from experience.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &mut Vec<u8>, t: &str) {
        v.extend((t.len() as u32).to_le_bytes());
        v.extend(t.as_bytes());
    }
    fn member(v: &mut Vec<u8>, name: &str, ty: &str, size: u32, count: u16) {
        s(v, name);
        s(v, ty);
        v.extend(0u32.to_le_bytes());
        v.extend(size.to_le_bytes());
        v.extend(count.to_le_bytes());
        v.extend([4, 0, 0]);
    }

    /// A tiny gameplay save: header, party data, one character, items and the schema.
    fn synthetic() -> Vec<u8> {
        const PARTY: [u8; 8] = *b"PARTYDAT";
        const PLAYER: [u8; 8] = *b"PLAYERST";
        const STATUS: [u8; 8] = *b"STATUSST";
        const PSTAT: [u8; 8] = *b"PSTATUSS";
        const ITEMS: [u8; 8] = *b"ITEMSSTR";
        let mut d = vec![0u8; 24];
        d[4] = 4;
        d.extend(PARTY);
        for v in [0i32, 7, 123_456, 3, 0, 42] {
            d.extend(v.to_le_bytes());
        }
        d.extend(PLAYER);
        d.extend([0u8; 12]);
        d.extend(16u32.to_le_bytes());
        d.extend(b"Noctis\0\0\0\0\0\0\0\0\0\0");
        d.extend(STATUS);
        d.extend(0u32.to_le_bytes()); // id
        d.extend(0i32.to_le_bytes()); // party_index
        d.push(3); // join, joined
        d.extend(5000i32.to_le_bytes()); // hp
        d.extend(5200i32.to_le_bytes()); // hp_max
        d.extend(55u32.to_le_bytes()); // level
        d.extend(PSTAT);
        d.extend(1234u32.to_le_bytes());
        d.extend(99_000u32.to_le_bytes());
        d.extend(ITEMS);
        d.extend(1u32.to_le_bytes());
        d.extend([0u8; 8]);
        d.extend(16_789_763u32.to_le_bytes());
        d.extend(36i32.to_le_bytes());
        while d.len() % 16 != 0 {
            d.push(0);
        }
        let at = d.len() as u32;
        d[12..16].copy_from_slice(&at.to_le_bytes());
        let mut sch = 5u32.to_le_bytes().to_vec();
        let defs: [(&str, [u8; 8], &[(&str, &str, u32)]); 5] = [
            ("Black.Save.Party.SavePartyDataStruct", PARTY, &[("usercontrol_player_save_number", "Black.Save.SAVE_PLAYER", 4), ("chapter", "int32_t", 4), ("gil", "int32_t", 4), ("medal", "int32_t", 4), ("coin", "int32_t", 4), ("ability_point", "int32_t", 4)]),
            ("Black.Save.Player.SavePlayerStruct", PLAYER, &[("object_entry_id", "SQEX.Ebony.Std.Fixid", 4)]),
            ("Black.Save.Player.SaveStatusStruct", STATUS, &[("id", "SQEX.Ebony.Std.Fixid", 4), ("party_index", "int32_t", 4), ("join", "bool", 1), ("joined", "bool", 1), ("hp", "int32_t", 4), ("hp_max", "int32_t", 4), ("level", "uint32_t", 4)]),
            ("Black.Save.Player.SavePlayerStatusStruct", PSTAT, &[("exp", "uint32_t", 4), ("exp_total", "uint32_t", 4)]),
            ("Black.Save.Item.SaveItemStruct", ITEMS, &[("battle_item", "Black.Save.Item.SaveItemDataStruct", 512)]),
        ];
        for (name, id, members) in defs {
            s(&mut sch, name);
            sch.extend(id);
            sch.extend([0u8; 8]);
            sch.extend((members.len() as u16).to_le_bytes());
            for (n, t, z) in members {
                member(&mut sch, n, t, *z, if *z == 512 { 64 } else { 1 });
            }
        }
        d.extend(sch);
        while d.len() % 16 != 0 {
            d.push(0);
        }
        d
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = synthetic();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Party/Gil").unwrap(), 123_456);
        assert_eq!(doc.view.pointer("/Party/Ability points").unwrap(), 42);
        assert_eq!(doc.view.pointer("/Characters/Party/Noctis/Max HP").unwrap(), 5200);
        assert_eq!(doc.view.pointer("/Characters/Party/Noctis/Level").unwrap(), 55);
        assert_eq!(doc.view.pointer("/Characters/Party/Noctis/Experience").unwrap(), 1234);
        assert_eq!(doc.view.pointer("/Items/Battle items/Item 16789763").unwrap(), 36);
        doc.set("/Party/Gil", serde_json::Value::from(9_000_000)).unwrap();
        doc.set("/Items/Battle items/Item 16789763", serde_json::Value::from(99)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        assert_eq!(parse(&out).unwrap().view.pointer("/Party/Gil").unwrap(), 9_000_000);
        assert_eq!(parse(&out).unwrap().view.pointer("/Items/Battle items/Item 16789763").unwrap(), 99);
    }

    #[test]
    fn encryption_round_trips() {
        let plain = synthetic();
        let mut foot = vec![0u8; FOOTER];
        for (i, b) in foot[..36].iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        foot[52] = 2;
        let enc = encrypt(&plain, &foot).unwrap();
        assert_ne!(&enc[..plain.len()], &plain[..]);
        let (back, f2) = decrypt(&enc).unwrap();
        assert_eq!(back, plain);
        assert_eq!(f2, foot);
        assert_eq!(encrypt(&back, &f2).unwrap(), enc);
    }

    /// `SAVE_EDITOR_FFXV=<gameplay0.save>`: decrypts, re-encrypts byte for byte, and prints what's read.
    #[test]
    #[ignore]
    fn research_real_save() {
        let path = std::env::var("SAVE_EDITOR_FFXV").expect("set SAVE_EDITOR_FFXV");
        let d = std::fs::read(path).unwrap();
        let (plain, foot) = decrypt(&d).expect("decrypts");
        assert_eq!(encrypt(&plain, &foot).unwrap(), d, "re-encrypts byte for byte");
        let doc = parse(&plain).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view).unwrap());
        assert_eq!(doc.to_bytes().unwrap(), plain);
    }
}
