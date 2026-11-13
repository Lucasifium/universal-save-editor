//! Red Dead Redemption (the 2023 PS4/Switch and 2024 PC release) and Red Dead Redemption 2 (PC).
//!
//! **Red Dead Redemption** (`RDR2SAVE<n>.SAV`, `RDR2ZOMBIESAVE<n>.SAV`, `REDDEAD2HCOR<n>.SAV`):
//!
//! ```text
//! 0x000  u32 1, u32 CRC-32 of everything from 0x20, u32 size from 0x20 (twice), 16 bytes
//! 0x020  "METAXTR\0" block: date and mission name (UTF-16) for the load menu, up to 0x538
//! 0x538  u32 version, u32 number of records
//! 0x540  the records, AES-256-ECB with every 16-byte block run through the cipher 16 times
//!        (a last partial block is stored as it is); each record is
//!        u32 name length, name, u8 type, u32 size, then the script global's bytes
//! ```
//!
//! The script globals keep their values in 8-byte cells. `gRDR2_Stats` is an array of floats
//! (cell 0 is its length) whose cells 1, 2 and 4 are money in cents, honor and fame.
//! `gInventory` holds (hash, count) pairs from +0x64, as many as the u32 at +0x5C, followed by
//! the number in use; item names are hashed with Jenkins one-at-a-time.
//!
//! **Red Dead Redemption 2** (`SRDR3<nnnn>`): a 0x110-byte header (u32 version 4, the save's
//! title in UTF-16, a header checksum at 0x10C) and then `RSAV` data encrypted with
//! AES-256-ECB. The data is a directory and several PSO frames (`PSIN`, `PMAP`, `PSCH`,
//! `PSIG`), each followed by a `CHKS` record with Jenkins one-at-a-time (signed bytes, seed
//! 0x3FAC7125) of the frame and the record with its size and checksum words zeroed. Every
//! value is found through the frame's own schema (structure and member name hashes, types,
//! offsets), never by searching for numbers. Format facts and the structure, member and
//! script-global locations follow Universal Save Editor by DanCarrillo11 (GPL-3.0),
//! cross-checked against Dxian998's RDR Save Crypto (MIT) and real saves.

use super::fields::{Builder, Checksum, Kind};
use crate::apollo::hash::jenkins_oaat;
use crate::tree::TreeDoc;
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use std::collections::HashMap;
use std::sync::OnceLock;

pub const KIND_1: &str = "Red Dead Redemption save";
pub const KIND_2: &str = "Red Dead Redemption 2 save";

// ------------------------------------------------------------------ Red Dead Redemption

const KEY_1: [u8; 32] = [
    0xB7, 0x62, 0xDF, 0xB6, 0xE2, 0xB2, 0xC6, 0xDE, 0xAF, 0x72, 0x2A, 0x32, 0xD2, 0xFB, 0x6F, 0x0C, 0x98, 0xA3, 0x21, 0x74, 0x62, 0xC9, 0xC4, 0xED, 0xAD, 0xAA,
    0x2E, 0xD0, 0xDD, 0xF9, 0x2F, 0x10,
];
const HEAD_1: usize = 0x538;
const BODY_1: usize = HEAD_1 + 8;

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn crc32(d: &[u8]) -> u32 {
    let mut c = flate2::Crc::new();
    c.update(d);
    c.sum()
}

/// The PC/PS4/Switch container (encrypted or not).
pub fn looks_1(d: &[u8]) -> bool {
    d.len() > BODY_1 + 16 && le32(d, 0) == Some(1) && &d[0x20..0x28] == b"METAXTR\0" && le32(d, 8) == Some((d.len() - 0x20) as u32)
}

fn rounds_1(body: &mut [u8], encrypt: bool) {
    let c = aes::Aes256::new(GenericArray::from_slice(&KEY_1));
    let n = body.len() / 16 * 16;
    for b in body[..n].chunks_exact_mut(16) {
        let b = GenericArray::from_mut_slice(b);
        for _ in 0..16 {
            if encrypt {
                c.encrypt_block(b);
            } else {
                c.decrypt_block(b);
            }
        }
    }
}

struct Rec {
    name: String,
    at: usize,
    size: usize,
}

/// The records of a decrypted save (None when the data doesn't read as records).
fn records_1(d: &[u8]) -> Option<Vec<Rec>> {
    if !looks_1(d) {
        return None;
    }
    let want = le32(d, HEAD_1 + 4)? as usize;
    let mut o = BODY_1;
    let mut out = Vec::new();
    while o < d.len() {
        let n = le32(d, o)? as usize;
        if !(1..=128).contains(&n) || out.len() >= want {
            return None;
        }
        let name = d.get(o + 4..o + 4 + n)?;
        if !name.iter().all(|c| c.is_ascii_graphic()) || *d.get(o + 4 + n)? > 16 {
            return None;
        }
        let size = le32(d, o + 5 + n)? as usize;
        let at = o + 9 + n;
        if at + size > d.len() {
            return None;
        }
        out.push(Rec { name: String::from_utf8_lossy(name).into_owned(), at, size });
        o = at + size;
    }
    (o == d.len() && out.len() == want).then_some(out)
}

/// Decrypts a Red Dead Redemption save (None if it isn't one or is already plain).
pub fn unpack_1(d: &[u8]) -> Option<Vec<u8>> {
    if !looks_1(d) || records_1(d).is_some() || le32(d, 4) != Some(crc32(&d[0x20..])) {
        return None;
    }
    let mut p = d.to_vec();
    rounds_1(&mut p[BODY_1..], false);
    records_1(&p)?;
    Some(p)
}

/// Encrypts the records again and writes the CRC-32.
pub fn pack_1(plain: &[u8]) -> Result<Vec<u8>, String> {
    records_1(plain).ok_or("the Red Dead Redemption save no longer reads as records")?;
    let mut out = plain.to_vec();
    rounds_1(&mut out[BODY_1..], true);
    let c = crc32(&out[0x20..]);
    out[4..8].copy_from_slice(&c.to_le_bytes());
    Ok(out)
}

/// Jenkins one-at-a-time of the lower-case name (how both games name items).
pub fn name_hash(s: &str) -> u32 {
    jenkins_oaat(s.to_ascii_lowercase().as_bytes(), 0)
}

/// Consumables the screen edits: (name in the save, shown name).
const RDR1_SUPPLIES: &[(&str, &str)] = &[
    ("snake_oil", "Snake Oil"),
    ("moonshine", "Moonshine"),
    ("chewing_tobacco", "Chewing Tobacco"),
    ("meds_lvl1", "Medicine"),
    ("tonic", "Tonic"),
    ("elixir", "Elixir"),
    ("absinthe", "Absinthe"),
    ("laudanum", "Laudanum"),
    ("horse_pills", "Horse Pills"),
    ("horse_meds", "Horse Medicine"),
    ("apple", "Apple"),
    ("bait", "Bait"),
    ("edible_meat", "Edible Meat"),
];
/// Most of one consumable the screen lets you set.
pub const RDR1_SUPPLY_MAX: i64 = 99;

/// Other things the inventory can hold, listed (read-only) under their names.
const RDR1_OWNED: &[(&str, &str)] = &[
    ("base_melee_melee_knife", "Knife"),
    ("base_pistol_pistol_highpower", "High Power Pistol"),
    ("base_pistol_pistol_mauser", "Mauser Pistol"),
    ("base_pistol_pistol_semiauto", "Semi-automatic Pistol"),
    ("base_pistol_pistol_volcanic", "Volcanic Pistol"),
    ("base_pistol_revolver_cattleman", "Cattleman Revolver"),
    ("base_pistol_revolver_doubleaction", "Double-action Revolver"),
    ("base_pistol_revolver_lemat", "LeMat Revolver"),
    ("base_pistol_revolver_schofield", "Schofield Revolver"),
    ("base_repeater_repeater_carbine", "Carbine Repeater"),
    ("base_repeater_repeater_evans", "Evans Repeater"),
    ("base_repeater_repeater_henry", "Henry Repeater"),
    ("base_repeater_repeater_winchester", "Winchester Repeater"),
    ("base_rifle_rifle_antitank", "Explosive Rifle"),
    ("base_rifle_rifle_boltaction", "Bolt-action Rifle"),
    ("base_rifle_rifle_buffalo", "Buffalo Rifle"),
    ("base_rifle_rifle_springfield", "Springfield Rifle"),
    ("base_shotgun_shotgun_blunderbuss", "Blunderbuss"),
    ("base_shotgun_shotgun_doublebarrel", "Double-barreled Shotgun"),
    ("base_shotgun_shotgun_pumpaction", "Pump-action Shotgun"),
    ("base_shotgun_shotgun_sawedoff", "Sawed-off Shotgun"),
    ("base_shotgun_shotgun_semiauto", "Semi-auto Shotgun"),
    ("base_sniperrifle_sniperrifle_carcano", "Carcano Rifle"),
    ("base_sniperrifle_sniperrifle_rollingblock", "Rolling Block Rifle"),
    ("base_thrown_exploding_thrown_dynamite", "Dynamite"),
    ("base_thrown_exploding_thrown_firebottle", "Fire Bottle"),
    ("base_thrown_exploding_thrown_holywater", "Holy Water"),
    ("base_thrown_exploding_thrown_zombiebait", "Bait (Undead Nightmare)"),
    ("base_thrown_exploding_thrown_zombieboombait", "Boom Bait"),
    ("base_thrown_thrown_throwingknife", "Throwing Knives"),
    ("base_thrown_thrown_tomahawk", "Tomahawk"),
    ("base_thrown_thrown_torch", "Torch"),
    ("binoculars", "Binoculars"),
    ("bandana", "Bandana"),
    ("bandolier", "Bandolier"),
    ("camp_lvl1", "Campsite"),
    ("rabbits_foot", "Rabbit's Foot"),
    ("survivalist_map", "Survivalist Map"),
    ("treasure_map", "Treasure Map"),
    ("newspaper", "Newspaper"),
    ("lasso", "Lasso"),
    ("lamp", "Lamp"),
    ("holy_relic", "Holy Relic"),
    ("phosphorous_kit", "Phosphorous Kit"),
    ("zombie_bait_kit", "Bait Kit"),
    ("zombie_boom_kit", "Boom Bait Kit"),
    ("blunderbuss_ammo_kit", "Blunderbuss Ammo Kit"),
    ("am_revol", "Revolver ammo"),
    ("am_pist", "Pistol ammo"),
    ("am_repeat", "Repeater ammo"),
    ("am_rifle", "Rifle ammo"),
    ("am_snipr", "Sniper rifle ammo"),
    ("am_shot", "Shotgun ammo"),
    ("am_at", "Explosive rifle ammo"),
    ("am_tomahawk", "Tomahawks"),
    ("am_blbs", "Blunderbuss ammo"),
    ("am_holy", "Holy Water"),
    ("am_zbait", "Bait (Undead Nightmare)"),
    ("am_zbait_ex", "Boom Bait"),
    ("firebottle", "Fire Bottle"),
    ("dynamite", "Dynamite"),
    ("horse_war", "War Horse"),
    ("horse_american", "American Standardbred"),
    ("horse_ardennais", "Ardennais"),
    ("horse_cleveland", "Cleveland Bay"),
    ("horse_dutch", "Dutch Warmblood"),
    ("horse_hedor", "Hungarian Half-bred (Hedor)"),
    ("horse_highland", "Highland Chestnut"),
    ("horse_hungarian", "Hungarian Half-bred"),
    ("horse_infested", "Infested horse"),
    ("horse_jaded", "Jaded horse"),
    ("horse_kentucky", "Kentucky Saddler"),
    ("horse_lusitano", "Lusitano"),
    ("horse_nag", "Nag"),
    ("horse_painted", "Painted horse"),
    ("horse_paintstd", "Standard paint"),
    ("horse_picor", "Picor"),
    ("horse_pinto", "Pinto"),
    ("horse_quarter", "Quarter Horse"),
    ("horse_senor", "Señor"),
    ("horse_tersk", "Tersk"),
    ("horse_tobiano", "Tobiano"),
    ("horse_turkmen", "Turkmen"),
    ("horse_welsh", "Welsh Pony"),
];

fn find<'a>(recs: &'a [Rec], name: &str) -> Option<&'a Rec> {
    recs.iter().find(|r| r.name == name)
}

pub fn parse_1(d: &[u8]) -> Option<TreeDoc> {
    let recs = records_1(d)?;
    let mut b = Builder::new(d);
    let mut any = false;
    if let Some(s) = find(&recs, "gRDR2_Stats").filter(|s| s.size >= 40 && le32(d, s.at) >= Some(5)) {
        let f = |c: usize| f32::from_le_bytes(d[s.at + c * 8..s.at + c * 8 + 4].try_into().unwrap());
        // Undead Nightmare leaves these at -1: they aren't used there.
        if f(1) >= 0.0 {
            b.ranged(&["John", "Money ($)"], s.at + 8, Kind::CentsF32, 0, 99_999);
            b.ranged(&["John", "Honor"], s.at + 16, Kind::F32Int, -10_000, 10_000);
            b.ranged(&["John", "Fame"], s.at + 32, Kind::F32Int, 0, 10_000);
            any = true;
        }
    }
    if let Some(r) = find(&recs, "gfSavedDeadEyePoints").filter(|r| r.size == 4) {
        if (0.0..=100.0).contains(&f32::from_le_bytes(d[r.at..r.at + 4].try_into().unwrap())) {
            b.ranged(&["John", "Dead Eye meter"], r.at, Kind::F32Int, 0, 100);
        }
    }
    if let Some(r) = find(&recs, "gInventory") {
        let cap = le32(d, r.at + 0x5C).unwrap_or(0) as usize;
        let start = r.at + 0x64;
        if (1..=1000).contains(&cap) && start + cap * 8 + 4 <= r.at + r.size {
            let used = le32(d, start + cap * 8).unwrap_or(u32::MAX) as usize;
            let filled = (0..cap).take_while(|i| le32(d, start + i * 8) != Some(0)).count();
            if used == filled {
                let supplies: HashMap<u32, &str> = RDR1_SUPPLIES.iter().map(|(n, l)| (name_hash(n), *l)).collect();
                let owned: HashMap<u32, &str> = RDR1_OWNED.iter().map(|(n, l)| (name_hash(n), *l)).collect();
                for i in 0..used {
                    let at = start + i * 8;
                    let h = le32(d, at)?;
                    let n = le32(d, at + 4)?;
                    if let Some(l) = supplies.get(&h) {
                        if i64::from(n) <= RDR1_SUPPLY_MAX {
                            b.ranged(&["Supplies", l], at + 4, Kind::U32, 0, RDR1_SUPPLY_MAX);
                            any = true;
                        }
                    } else if let Some(l) = owned.get(&h) {
                        let _ = n;
                        b.info(&["Inventory", l], "Owned");
                    }
                }
            }
        }
    }
    if let Some(r) = find(&recs, "geLastMission").filter(|r| r.size == 4) {
        b.info(&["Save", "Last mission"], format!("{:08X}", le32(d, r.at)?));
    }
    let title: String = {
        let u: Vec<u16> = d[0x38..0x138].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
        String::from_utf16_lossy(&u)
    };
    if !title.is_empty() {
        b.info(&["Save", "Title"], title);
    }
    b.info(&["Save", "Script globals"], recs.len());
    any.then(|| b.finish(KIND_1, Checksum::None))
}

// ------------------------------------------------------------------ Red Dead Redemption 2

const KEY_2: [u8; 32] = [
    0x46, 0xED, 0x8D, 0x3F, 0x94, 0x35, 0xE4, 0xEC, 0x12, 0x2C, 0xB2, 0xE2, 0xAF, 0x97, 0xC5, 0x7E, 0x4C, 0x5A, 0x8C, 0x30, 0x92, 0xC7, 0x84, 0x4E, 0x11, 0xC6,
    0x86, 0xFF, 0x41, 0xDF, 0x41, 0x0F,
];
const HEAD_2: usize = 0x110;
const CHKS_SEED: u32 = 0x3FAC_7125;

pub fn looks_2(d: &[u8]) -> bool {
    d.len() > HEAD_2 + 64 && (d.len() - HEAD_2) % 16 == 0 && be32(d, 0) == Some(4)
}

fn ecb_2(body: &mut [u8], encrypt: bool) {
    let c = aes::Aes256::new(GenericArray::from_slice(&KEY_2));
    for b in body.chunks_exact_mut(16) {
        let b = GenericArray::from_mut_slice(b);
        if encrypt {
            c.encrypt_block(b);
        } else {
            c.decrypt_block(b);
        }
    }
}

/// (start of the checked region, offset of its CHKS record) for each frame.
fn regions(p: &[u8]) -> Option<Vec<(usize, usize)>> {
    let mut out = Vec::new();
    let mut at = HEAD_2;
    let mut last = HEAD_2;
    while let Some(i) = p[at..].windows(8).position(|w| w == b"CHKS\0\0\0\x14") {
        let c = at + i;
        let len = be32(p, c + 8)? as usize;
        if len < 20 || c + 20 > p.len() || c + 20 < len || c + 20 - len < last || out.len() >= 64 {
            return None;
        }
        out.push((c + 20 - len, c));
        at = c + 20;
        last = at;
    }
    (!out.is_empty()).then_some(out)
}

fn chks(p: &[u8], begin: usize, c: usize) -> u32 {
    let mut s = p[begin..c + 20].to_vec();
    let n = s.len();
    s[n - 12..n - 4].fill(0);
    jenkins_oaat(&s, CHKS_SEED)
}

fn checks_ok(p: &[u8]) -> bool {
    regions(p).is_some_and(|r| r.iter().all(|&(b, c)| be32(p, c + 12) == Some(chks(p, b, c))))
}

/// Decrypts an RDR2 PC save whose checksums all hold (None otherwise).
pub fn unpack_2(d: &[u8]) -> Option<Vec<u8>> {
    if !looks_2(d) || &d[HEAD_2..HEAD_2 + 4] == b"RSAV" {
        return None;
    }
    let mut p = d.to_vec();
    ecb_2(&mut p[HEAD_2..], false);
    (&p[HEAD_2..HEAD_2 + 4] == b"RSAV" && checks_ok(&p)).then_some(p)
}

/// Writes every frame's checksum again and encrypts.
pub fn pack_2(plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut p = plain.to_vec();
    let r = regions(&p).ok_or("the Red Dead Redemption 2 save's checksum records are missing")?;
    for (b, c) in r {
        let v = chks(&p, b, c);
        p[c + 12..c + 16].copy_from_slice(&v.to_be_bytes());
    }
    ecb_2(&mut p[HEAD_2..], true);
    Ok(p)
}

/// Case-sensitive hash of a parser (structure or member) name.
fn ph(s: &str) -> u32 {
    jenkins_oaat(s.as_bytes(), 0)
}

#[derive(Clone, Copy)]
struct Member {
    hash: u32,
    kind: u8,
    sub: u8,
    off: u16,
    reference: u32,
}

impl Member {
    fn offset(&self) -> usize {
        self.off as usize + (usize::from(self.sub & 0x40) << 10)
    }
}

struct Frame {
    /// (structure, absolute start, length).
    blocks: Vec<(u32, usize, usize)>,
    structs: HashMap<u32, (usize, Vec<Member>)>,
}

fn frame(p: &[u8], start: usize, end: usize) -> Option<Frame> {
    let mut parts = HashMap::new();
    let mut at = start;
    for tag in [b"PSIN", b"PMAP", b"PSCH", b"PSIG"] {
        if at + 8 > end || &p[at..at + 4] != tag {
            return None;
        }
        let size = be32(p, at + 4)? as usize;
        if size < 8 || size > end - at {
            return None;
        }
        parts.insert(*tag, (at, size));
        at += size;
    }
    if at != end {
        return None;
    }
    let psin = parts[b"PSIN"].1;
    let (m, len) = parts[b"PMAP"];
    let mut count = u16::from_be_bytes(p.get(m + 12..m + 14)?.try_into().ok()?) as usize;
    let mut head = 16;
    if count == 0 {
        count = u16::from_be_bytes(p.get(m + 16..m + 18)?.try_into().ok()?) as usize;
        head = 24;
    }
    if !(1..=8192).contains(&count) || head + count * 16 != len {
        return None;
    }
    let mut blocks = Vec::with_capacity(count);
    for i in 0..count {
        let e = m + head + i * 16;
        let (id, off, size) = (be32(p, e)?, be32(p, e + 4)? as usize, be32(p, e + 12)? as usize);
        if off < 8 || off > psin || size > psin - off {
            return None;
        }
        blocks.push((id, start + off, size));
    }
    let (s, len) = parts[b"PSCH"];
    let n = be32(p, s + 8)? as usize;
    if n > 4096 || 12 + n * 8 > len {
        return None;
    }
    let mut structs = HashMap::new();
    for i in 0..n {
        let (id, off) = (be32(p, s + 12 + i * 8)?, be32(p, s + 16 + i * 8)? as usize);
        if off < 12 + n * 8 || off + 12 > len {
            return None;
        }
        let at = s + off;
        let desc = be32(p, at)?;
        if desc >> 24 != 0 {
            continue;
        }
        let size = be32(p, at + 4)? as usize;
        let fields = (desc & 0xFFFF) as usize;
        if fields > 2048 || 12 + fields * 12 > len - off {
            return None;
        }
        let members = (0..fields)
            .map(|j| {
                let f = at + 12 + j * 12;
                Member { hash: be32(p, f).unwrap(), kind: p[f + 4], sub: p[f + 5], off: u16::from_be_bytes([p[f + 6], p[f + 7]]), reference: be32(p, f + 8).unwrap() }
            })
            .collect();
        structs.insert(id, (size, members));
    }
    Some(Frame { blocks, structs })
}

fn width(kind: u8) -> Option<usize> {
    Some(match kind {
        0 | 2 => 1,
        4 => 2,
        5 | 6 | 7 | 11 => 4,
        12 | 31 => 8,
        13 => 16,
        _ => return None,
    })
}

/// Offset and referenced structure of the one member called `name` of the given type.
fn member(f: &Frame, st: u32, name: &str, kind: u8, sub: u8) -> Option<(usize, u32)> {
    let (size, ms) = f.structs.get(&st)?;
    let h = ph(name);
    let mut it = ms.iter().filter(|m| m.hash == h && m.kind == kind && m.sub & 0x3F == sub && m.sub & 0x80 == 0);
    let m = it.next()?;
    if it.next().is_some() {
        return None;
    }
    let w = width(kind)?;
    (m.offset() + w <= *size).then(|| (m.offset(), m.reference))
}

struct Save2<'a> {
    p: &'a [u8],
    frames: Vec<Frame>,
}

/// A row of a structure: its frame, its start, and where each asked-for member lives.
type Row = (usize, usize, Vec<usize>);

impl Save2<'_> {
    fn rows(&self, st: &str, want: &[(&str, u8, u8)]) -> Vec<Row> {
        let id = ph(st);
        let mut out = Vec::new();
        for (fi, f) in self.frames.iter().enumerate() {
            let Some(&(size, _)) = f.structs.get(&id) else { continue };
            if !(1..=4096).contains(&size) {
                continue;
            }
            let Some(ms): Option<Vec<usize>> = want.iter().map(|(n, k, s)| member(f, id, n, *k, *s).map(|m| m.0)).collect() else { continue };
            for &(h, at, len) in &f.blocks {
                if h == id && len % size == 0 && len / size <= 8192 {
                    for r in (at..at + len).step_by(size) {
                        out.push((fi, r, ms.iter().map(|o| r + o).collect()));
                    }
                }
            }
        }
        out
    }

    /// Follows an extended PSO pointer: (structure, absolute target, bytes left in the block).
    fn pointer(&self, f: &Frame, at: usize, need: usize) -> Option<(u32, usize, usize)> {
        let w = be32(self.p, at)?;
        let block = (w & 0xFFF) as usize;
        let off = (w >> 20) as usize | ((le32(self.p, at + 4)? as usize) << 12);
        if w & 0xFF000 != 0 || block == 0 || block > f.blocks.len() {
            return None;
        }
        let (h, start, len) = f.blocks[block - 1];
        (off <= len && need <= len - off).then_some((h, start + off, len - off))
    }

    /// The items of the inventory in use: (frame, item structure, record, [Guid, ItemId, Quantity]).
    fn items(&self) -> Vec<(usize, u32, usize, [usize; 3])> {
        let rows = self.rows("CPedInventorySaveStructure", &[("playerInventory", 12, 0), ("backupInventory", 12, 0), ("usingBackupInventory", 0, 0)]);
        let [(fi, _, m)] = &rows[..] else { return vec![] };
        let f = &self.frames[*fi];
        let flag = self.p[m[2]];
        if flag > 1 {
            return vec![];
        }
        let field = if flag == 1 { "backupInventory" } else { "playerInventory" };
        let base = m[flag as usize];
        let res = (|| {
            let (_, link) = member(f, ph("CPedInventorySaveStructure"), field, 12, 0)?;
            let (arr, _) = member(f, link, "items", 13, 0)?;
            let at = base + arr;
            let count = u16::from_be_bytes(self.p.get(at + 8..at + 10)?.try_into().ok()?) as usize;
            let capacity = u16::from_be_bytes(self.p.get(at + 10..at + 12)?.try_into().ok()?) as usize;
            if !(count > 0 && count <= capacity && capacity <= 4096) {
                return None;
            }
            let (id, start, _) = self.pointer(f, at, count * 8)?;
            if id != 12 {
                return None;
            }
            let mut out = Vec::with_capacity(count);
            for n in 0..count {
                let (h, rec, left) = self.pointer(f, start + n * 8, 48)?;
                let &(size, _) = f.structs.get(&h)?;
                let w = be32(self.p, start + n * 8)?;
                let target = f.blocks[(w & 0xFFF) as usize - 1];
                if left < size || size == 0 || target.2 % size != 0 || (rec - target.1) % size != 0 {
                    return None;
                }
                let g = member(f, h, "Guid", 31, 0)?.0;
                let i = member(f, h, "ItemId", 11, 7)?.0;
                let q = member(f, h, "Quantity", 31, 0)?.0;
                member(f, h, "ParentGuid", 31, 0)?;
                out.push((*fi, h, rec, [rec + g, rec + i, rec + q]));
            }
            let mut guids: Vec<&[u8]> = out.iter().map(|o| &self.p[o.3[0]..o.3[0] + 8]).collect();
            guids.sort();
            guids.dedup();
            let mut recs: Vec<usize> = out.iter().map(|o| o.2).collect();
            recs.sort();
            recs.dedup();
            (guids.len() == out.len() && recs.len() == out.len()).then_some(out)
        })();
        res.unwrap_or_default()
    }

    /// Where the one namespace-0 integer statistic called `name` keeps its value.
    fn stat(&self, stats: &[Row], name: &str) -> Option<usize> {
        let h = name_hash(name);
        let mut it = stats.iter().filter(|r| be32(self.p, r.2[0]) == Some(h) && be32(self.p, r.2[0] + 4) == Some(0));
        let r = it.next()?;
        it.next().is_none().then_some(r.2[1])
    }
}

/// Build-1491 script save root and its attribute block.
const ROOT: u32 = 0x41D6_F894;

/// Inline members of `st` (searched down nested structures) whose value sits at `delta`.
fn scalar_paths(f: &Frame, st: u32, delta: usize, kind: u8, base: usize, depth: usize) -> usize {
    let Some((size, ms)) = f.structs.get(&st) else { return 0 };
    if depth > 16 || delta < base || delta >= base + size {
        return 0;
    }
    let mut n = 0;
    for m in ms {
        if m.sub & 0x80 != 0 {
            continue;
        }
        let off = m.offset();
        if off >= *size {
            continue;
        }
        if m.kind == kind && m.sub & 0x3F == 0 && base + off == delta && off + 4 <= *size {
            n += 1;
        }
        if m.kind == 12 && m.sub & 0x3F == 0 {
            if let Some(&(cs, _)) = f.structs.get(&m.reference) {
                if cs <= size - off && base + off <= delta && delta < base + off + cs {
                    n += scalar_paths(f, m.reference, delta, kind, base + off, depth + 1);
                }
            }
        }
    }
    n
}

fn names_2() -> &'static HashMap<u32, &'static str> {
    static M: OnceLock<HashMap<u32, &'static str>> = OnceLock::new();
    M.get_or_init(|| super::rdr2_names::NAMES.iter().map(|n| (name_hash(n), *n)).collect())
}

/// "CONSUMABLE_HERB_YARROW" -> "Herb Yarrow".
fn pretty(n: &str) -> String {
    let n = ["A_C_HORSE_", "CONSUMABLE_", "PROVISION_", "AMMO_"].iter().find_map(|p| n.strip_prefix(p)).unwrap_or(n);
    let (n, used) = match n.strip_suffix("_USED") {
        Some(n) => (n, " (part used)"),
        None => (n, ""),
    };
    let s = n.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            let first = c.next().unwrap();
            first.to_uppercase().chain(c.flat_map(char::to_lowercase)).collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ");
    s + used
}

/// Most of one item the screen lets you set: the stack size items reach with the Legend of
/// the East satchel.
pub const RDR2_STACK_MAX: i64 = 99;
/// Most ammunition of one type the screen lets you set.
pub const RDR2_AMMO_MAX: i64 = 999;

pub fn parse_2(p: &[u8]) -> Option<TreeDoc> {
    if !looks_2(p) || &p[HEAD_2..HEAD_2 + 4] != b"RSAV" || !checks_ok(p) {
        return None;
    }
    let frames: Vec<Frame> = regions(p)?.into_iter().map(|(b, c)| frame(p, b, c)).collect::<Option<_>>()?;
    let s = Save2 { p, frames };
    let mut b = Builder::new(p);
    b.big_endian = true;
    let mut any = false;

    let items = s.items();
    // Cash: the one CURRENCY_CASH item in the currency slot, stored in cents (i64).
    {
        let inv = ph("SavegameInventoryItem");
        let cash = name_hash("CURRENCY_CASH");
        let slot = name_hash("SLOTID_CURRENCY");
        let hits: Vec<usize> = items
            .iter()
            .filter(|(fi, h, rec, m)| {
                *h == inv
                    && be32(p, m[1]) == Some(cash)
                    && member(&s.frames[*fi], inv, "SlotId", 6, 0).is_some_and(|(o, _)| be32(p, rec + o) == Some(slot))
                    && be32(p, m[2]) == Some(0)
                    && be32(p, m[2] + 4).is_some_and(|v| v <= i32::MAX as u32)
            })
            .map(|(_, _, _, m)| m[2] + 4)
            .collect();
        if let [at] = hits[..] {
            b.ranged(&["Arthur", "Cash ($)"], at, Kind::CentsI32, 0, 9_999_999);
            any = true;
        }
    }

    let stats = s.rows("CBaseStatsSaveStructure__CIntStatStruct", &[("statId", 12, 0), ("Data", 5, 0)]);

    // The script save root: honor, cores, XP and the campaign globals.
    let roots: Vec<(usize, usize, usize)> =
        s.frames.iter().enumerate().flat_map(|(fi, f)| f.blocks.iter().filter(|x| x.0 == ROOT).map(move |x| (fi, x.1, x.2))).collect();
    if let [(fi, root, rlen)] = roots[..] {
        let f = &s.frames[fi];
        let links: Vec<usize> = f
            .structs
            .get(&ROOT)
            .map(|(_, ms)| ms.iter().filter(|m| m.hash == 0xB5D7_5947 && m.kind == 12 && m.sub == 64 && m.reference == 0x41D6_F933).map(Member::offset).collect())
            .unwrap_or_default();
        if let ([delta], Some((568, schema))) = (&links[..], f.structs.get(&0x41D6_F933)) {
            if delta + 568 <= rlen {
                let base = root + delta;
                let typed = |h: u32, k: u8, off: u16, r: u32| schema.iter().any(|m| m.hash == h && m.kind == k && m.sub == 0 && m.off == off && m.reference == r);
                if typed(0xA53C_DE40, 5, 280, 0) {
                    if let Some(st) = s.stat(&stats, "HONOR_CURRENT") {
                        if be32(p, st) == be32(p, base + 280) {
                            b.ranged(&["Arthur", "Honor"], base + 280, Kind::I32, -320, 320);
                            b.mirror(&["Arthur", "Honor"], st);
                            any = true;
                        }
                    }
                }
                let arrays: [(u32, u32, usize, usize, &[(usize, &str, i64, i64)]); 2] = [
                    (0xC336_4EDC, 0x41D6_F934, 0, 3, &[(0, "Health core", 0, 100), (1, "Stamina core", 0, 100), (2, "Dead Eye core", 0, 100)]),
                    (0x18D8_E817, 0x41D6_F938, 88, 23, &[(0, "Health XP", 0, 1100), (1, "Stamina XP", 0, 1100), (2, "Dead Eye XP", 0, 1100), (13, "Weight", -100, 100)]),
                ];
                for (link, child, off, count, entries) in arrays {
                    let Some((cs, cm)) = f.structs.get(&child) else { continue };
                    if !typed(link, 12, off as u16, child) || *cs != 8 + count * 8 || p[base + off] as usize != count {
                        continue;
                    }
                    for &(i, title, lo, hi) in entries {
                        let o = 8 + i * 8;
                        let direct = cm.iter().any(|m| m.kind == 7 && m.sub == 0 && m.offset() == o);
                        let wrapped: Vec<u32> = cm.iter().filter(|m| m.kind == 12 && m.sub == 0 && m.offset() == o).map(|m| m.reference).collect();
                        let ok = direct
                            || matches!(&wrapped[..], [w] if f.structs.get(w).is_some_and(|(sz, ms)| *sz == 8 && ms.iter().any(|m| m.kind == 7 && m.sub == 0 && m.off == 0)));
                        let at = base + off + o;
                        let v = f32::from_be_bytes(p[at..at + 4].try_into().ok()?);
                        if ok && v.is_finite() && (lo as f32..=hi as f32).contains(&v) {
                            b.ranged(&["Arthur", title], at, Kind::F32, lo, hi);
                            any = true;
                        }
                    }
                }
            }
        }
        // Campaign globals: Global_40 script cells (8 bytes each), checked against the schema.
        let scalar = |index: usize, kind: u8| -> Option<usize> {
            let d = index * 8;
            (d + 4 <= rlen && scalar_paths(f, ROOT, d, kind, 0, 0) == 1).then_some(root + d)
        };
        if let (Some(at), Some(st)) = (scalar(4283 + 325, 5), s.stat(&stats, "GANG_SAVINGS_CASH")) {
            let v = be32(p, at)? as i32;
            if v >= 0 && be32(p, st) == be32(p, at) {
                b.ranged(&["Camp", "Camp funds ($)"], at, Kind::CentsI32, 0, 9_999_999);
                b.mirror(&["Camp", "Camp funds ($)"], st);
            }
        }
        // Bounties: six region records of 12 cells from Global_40.f_359 (shown, not edited:
        // the game also keeps the current region's amount and a wanted flag per region).
        const REGIONS: [&str; 6] = ["Ambarino", "New Hanover", "Lemoyne", "West Elizabeth", "New Austin", "Guarma"];
        if rlen >= 359 * 8 && p[root + 358 * 8] == 6 {
            let amounts: Option<Vec<usize>> = (0..6).map(|n| scalar(359 + n * 12, 5)).collect();
            if let Some(amounts) = amounts {
                let vals: Vec<u32> = amounts.iter().map(|&a| be32(p, a).unwrap_or(u32::MAX)).collect();
                if vals.iter().all(|&v| v <= 150_000) {
                    for (n, v) in vals.iter().enumerate() {
                        b.info(&["Bounties", &format!("{} ($)", REGIONS[n])], crate::tree::num(f64::from(*v) / 100.0));
                    }
                }
            }
        }
    }

    let players = s.rows("CPlayerPedSaveStructure", &[("fHealth", 7, 0)]);
    if let [(_, _, m)] = &players[..] {
        if f32::from_be_bytes(p[m[0]..m[0] + 4].try_into().ok()?).is_finite() {
            b.ranged(&["Arthur", "Saved health"], m[0], Kind::F32, 0, 1000);
        }
    }

    // Inventory: existing stacks of ammunition, consumables and provisions.
    let inv = ph("SavegameInventoryItem");
    let horse = ph("CPedInventorySaveStructure__HorseItem");
    let names = names_2();
    let satchel = name_hash("SLOTID_SATCHEL");
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (fi, h, rec, m) in &items {
        let id = be32(p, m[1])?;
        let Some(name) = names.get(&id) else { continue };
        if *h == inv {
            // Only the satchel's own stacks: weapons keep separate copies of what is loaded.
            if !member(&s.frames[*fi], inv, "SlotId", 6, 0).is_some_and(|(o, _)| be32(p, rec + o) == Some(satchel)) {
                continue;
            }
            let (group, max) = if name.starts_with("AMMO_") {
                ("Ammunition", RDR2_AMMO_MAX)
            } else if name.starts_with("CONSUMABLE_") {
                ("Consumables", RDR2_STACK_MAX)
            } else if name.starts_with("PROVISION_") && !name.starts_with("PROVISION_TRINKET_") && !name.starts_with("PROVISION_TALISMAN_") {
                ("Provisions", RDR2_STACK_MAX)
            } else {
                continue;
            };
            let q = (be32(p, m[2])?, be32(p, m[2] + 4)?);
            if q.0 != 0 || i64::from(q.1) < 1 || i64::from(q.1) > max {
                continue;
            }
            let mut label = pretty(name);
            let k = seen.entry(format!("{group}/{label}")).or_insert(0);
            *k += 1;
            if *k > 1 {
                label = format!("{label} ({k})");
            }
            b.ranged(&[group, &label], m[2], Kind::U64, 1, max);
            any = true;
        } else if *h == horse {
            let f = &s.frames[*fi];
            let label = pretty(name);
            for (field, title) in [("Dirty", "Dirt"), ("Hungry", "Hunger")] {
                if let Some((o, _)) = member(f, horse, field, 5, 0) {
                    if be32(p, rec + o).is_some_and(|v| v <= 100) {
                        b.ranged(&["Horses", &label, title], rec + o, Kind::I32, 0, 100);
                    }
                }
            }
        }
    }

    const STATS: &[(&str, &str)] = &[
        ("KILLS", "Kills"),
        ("HEADSHOTS", "Headshots"),
        ("DEATHS", "Deaths"),
        ("HORSES_BROKEN", "Horses broken"),
        ("FISH_ESCAPED", "Fish escaped"),
        ("STOLEN_HORSES_SOLD", "Stolen horses sold"),
        ("STOLEN_WAGONS_SOLD", "Stolen wagons sold"),
        ("HORSE_STABLE_VISITS", "Stable visits"),
        ("BATHS_TAKEN", "Baths taken"),
        ("BAR_DRINKS_CONSUMED", "Bar drinks"),
        ("BAR_MEALS_CONSUMED", "Bar meals"),
        ("THEATRE_VISITS", "Theatre visits"),
        ("SHOWS_SEEN", "Shows seen"),
        ("MEDICINE_ITEMS_USED", "Medicine items used"),
        ("DEADEYE_ITEMS_USED", "Dead Eye items used"),
        ("PROVISION_ITEMS_USED", "Provisions used"),
        ("NUM_PEDS_ANTAGONIZED", "People antagonized"),
    ];
    for (stat, title) in STATS {
        if let Some(at) = s.stat(&stats, stat) {
            if (be32(p, at)? as i32) >= 0 {
                b.ranged(&["Statistics", title], at, Kind::I32, 0, i32::MAX.into());
            }
        }
    }

    let clock = s.rows("CSimpleVariablesSaveStructure", &[("GameClockHour", 6, 0), ("GameClockMinute", 6, 0), ("GameClockSecond", 6, 0)]);
    if let [(_, _, m)] = &clock[..] {
        for (i, (t, hi)) in [("Hour", 23), ("Minute", 59), ("Second", 59)].into_iter().enumerate() {
            if be32(p, m[i]).is_some_and(|v| v <= hi as u32) {
                b.ranged(&["World", t], m[i], Kind::U32, 0, hi);
            }
        }
    }

    let title: String = {
        let u: Vec<u16> = p[4..0x104].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
        String::from_utf16_lossy(&u)
    };
    if !title.is_empty() {
        b.info(&["Save", "Title"], title);
    }
    any.then(|| b.finish(KIND_2, Checksum::None))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rdr1_plain() -> Vec<u8> {
        let mut d = vec![0u8; HEAD_1];
        d[0] = 1;
        d[0x20..0x28].copy_from_slice(b"METAXTR\0");
        d.extend(7u32.to_le_bytes());
        d.extend(3u32.to_le_bytes());
        let mut rec = |name: &str, data: Vec<u8>| {
            d.extend((name.len() as u32).to_le_bytes());
            d.extend(name.as_bytes());
            d.push(1);
            d.extend((data.len() as u32).to_le_bytes());
            d.extend(data);
        };
        let mut stats = vec![0u8; 8 * 6];
        stats[0] = 5;
        stats[8..12].copy_from_slice(&319_330f32.to_le_bytes());
        stats[16..20].copy_from_slice(&400f32.to_le_bytes());
        stats[32..36].copy_from_slice(&1100f32.to_le_bytes());
        rec("gRDR2_Stats", stats);
        let mut inv = vec![0u8; 0x64 + 4 * 8 + 4 + 3];
        inv[0x5C] = 4;
        inv[0x64..0x68].copy_from_slice(&name_hash("snake_oil").to_le_bytes());
        inv[0x68] = 5;
        inv[0x6C..0x70].copy_from_slice(&name_hash("binoculars").to_le_bytes());
        inv[0x70] = 1;
        inv[0x64 + 32] = 2;
        rec("gInventory", inv);
        rec("geLastMission", vec![1, 2, 3, 4]);
        let n = d.len() - 0x20;
        d[8..12].copy_from_slice(&(n as u32).to_le_bytes());
        d[12..16].copy_from_slice(&(n as u32).to_le_bytes());
        d
    }

    #[test]
    fn rdr1_roundtrip_and_fields() {
        let plain = rdr1_plain();
        assert!(records_1(&plain).is_some());
        let enc = pack_1(&plain).unwrap();
        assert_ne!(enc[BODY_1..], plain[BODY_1..]);
        assert_eq!(unpack_1(&enc).unwrap()[8..], plain[8..]);
        let mut doc = parse_1(&plain).unwrap();
        assert_eq!(doc.view.pointer("/John/Money ($)"), Some(&serde_json::json!(3193.3)));
        assert_eq!(doc.view.pointer("/Supplies/Snake Oil"), Some(&serde_json::json!(5)));
        assert_eq!(doc.view.pointer("/Inventory/Binoculars"), Some(&serde_json::json!("Owned")));
        doc.set("/John/Money ($)", serde_json::json!(9999.99)).unwrap();
        doc.set("/Supplies/Snake Oil", serde_json::json!(99)).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse_1(&out).unwrap();
        assert_eq!(again.view.pointer("/John/Money ($)"), Some(&serde_json::json!(9999.99)));
        assert_eq!(again.view.pointer("/Supplies/Snake Oil"), Some(&serde_json::json!(99)));
        assert!(doc.set("/Supplies/Snake Oil", serde_json::json!(100)).is_err());
    }

    #[test]
    fn rdr2_checksum_matches_known_value() {
        // The frame check is plain Jenkins one-at-a-time over signed bytes with Rockstar's seed.
        let mut p = vec![0u8; HEAD_2];
        p.extend(b"RSAV");
        p.extend([0u8; 12]);
        p.extend(b"CHKS\0\0\0\x14");
        p.extend(36u32.to_be_bytes());
        p.extend([0u8; 8]);
        let c = HEAD_2 + 16;
        let v = chks(&p, HEAD_2, c);
        assert_eq!(v, jenkins_oaat(&[b"RSAV".as_slice(), &[0; 12], b"CHKS\0\0\0\x14", &[0; 4], &[0; 8]].concat(), CHKS_SEED));
        assert_eq!(name_hash("CURRENCY_CASH"), name_hash("currency_cash"));
    }

    /// Runs on real saves: RDR_SAVES names a folder of RDR2SAVE*.SAV and SRDR3* files.
    #[test]
    fn real_saves() {
        let Some(dir) = std::env::var_os("RDR_SAVES") else { return };
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let path = e.unwrap().path();
            let d = std::fs::read(&path).unwrap();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let (plain, doc, packed) = if looks_1(&d) {
                let p = unpack_1(&d).expect("rdr1 decrypts");
                let doc = parse_1(&p);
                (p.clone(), doc, pack_1(&p).unwrap())
            } else if looks_2(&d) {
                let p = unpack_2(&d).expect("rdr2 decrypts");
                let doc = parse_2(&p);
                (p.clone(), doc, pack_2(&p).unwrap())
            } else {
                continue;
            };
            assert_eq!(packed, d, "{name}: repacks to the same bytes");
            let dec = crate::formats::decode(&d);
            assert_eq!(crate::formats::encode(&dec.layers, &dec.content).unwrap(), d, "{name}: opens and saves unchanged");
            if doc.is_some() {
                assert!(matches!(dec.content, crate::formats::Content::Tree(ref t) if t.kind_name.starts_with("Red Dead")), "{name}: opens on the game's values");
            }
            n += 1;
            let Some(mut doc) = doc else {
                eprintln!("{name}: no screen values");
                continue;
            };
            let g = crate::game_ui::detect(&doc.kind_name, &doc.view).expect("screen");
            assert!(g.name().is_some_and(|n| n.starts_with("Red Dead")), "{name}: game screen");
            eprintln!("{name}: {}", serde_json::to_string(&doc.view).unwrap().chars().take(1500).collect::<String>());
            // Change every editable number to itself plus nothing: then one real edit.
            let ptrs: Vec<String> = ["/Arthur/Cash ($)", "/John/Money ($)", "/Arthur/Honor", "/Camp/Camp funds ($)"].iter().map(|s| s.to_string()).collect();
            for ptr in ptrs {
                if doc.view.pointer(&ptr).is_some() {
                    doc.set(&ptr, serde_json::json!(123)).unwrap();
                }
            }
            let out = doc.to_bytes().unwrap();
            let again = if looks_1(&plain) { parse_1(&unpack_1(&pack_1(&out).unwrap()).unwrap()) } else { parse_2(&unpack_2(&pack_2(&out).unwrap()).unwrap()) }.unwrap();
            for ptr in ["/Arthur/Cash ($)", "/John/Money ($)", "/Arthur/Honor", "/Camp/Camp funds ($)"] {
                if let Some(v) = again.view.pointer(ptr) {
                    assert_eq!(v.as_f64(), Some(123.0), "{name} {ptr}");
                }
            }
            let enc = if looks_1(&plain) { pack_1(&out).unwrap() } else { pack_2(&out).unwrap() };
            let back = if looks_1(&plain) { unpack_1(&enc) } else { unpack_2(&enc) };
            assert!(back.is_some_and(|b| b[8..] == out[8..] || looks_2(&b)), "{name}: edited save decrypts with valid checksums");
            if let Some(o) = std::env::var_os("RDR_OUT") {
                std::fs::write(std::path::Path::new(&o).join(&name), &enc).unwrap();
            }
        }
        assert!(n > 0);
    }
}
