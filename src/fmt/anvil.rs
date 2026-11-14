//! Assassin's Creed saves from Brotherhood to Odyssey (Ubisoft's Anvil engine).
//!
//! ```text
//! wrapper   Ubisoft Connect header (u32 0x224 / 0x204 / 0x24, u32 0, …), a crack "CDX" header,
//!           0x208 zero bytes, or nothing (Brotherhood's own .SAV files)
//! body      XORed from the body start to the end of the file: byte i ^= K[(fileLength - i) % len K]
//!           K is MD5 of the account id (Unity and later), a fixed key (Liberation HD, Black Flag,
//!           Rogue), the account name (Revelations) or nothing (Brotherhood). The key is found from
//!           known bytes of the body, so the account never has to be known.
//! body      u32 version, AC DB FE 00, then containers. Each container is preceded by a record
//!           whose last u32 is its unpacked size (and one u32 of the record its size in the file):
//!             33 AA FB 57 99 FA 04 10, u16 version, u8 compression (0 LZO1X, 2 LZO2A), u32 block size
//!             blocks: 01, u32 packed size, u32 size, u32 check, bytes | 00, u32 size, bytes
//!           check = Adler-32 with both sums starting at 0, over the packed bytes.
//! data      a property stream: u32 length, u32 CRC-32 of the name, u32, 00 00, u16 type, 0B, value
//! ```
//!
//! Money, materials and ammunition live in `InventoryContainer` objects (Brotherhood to
//! Syndicate: a `Category` enum, then `CurrentItemCount`), or as item ids with a `Quantity`
//! (Origins and Odyssey). The game keeps several inventories (the player's, its checkpoint
//! copies, other characters and memories); edits go to the player's and its copies only.

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

const MAGIC: [u8; 4] = [0xAC, 0xDB, 0xFE, 0x00];
const SIG: [u8; 8] = [0x33, 0xAA, 0xFB, 0x57, 0x99, 0xFA, 0x04, 0x10];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ac {
    Brotherhood,
    Revelations,
    Ac3r,
    LiberationHd,
    BlackFlag,
    Rogue,
    Unity,
    Syndicate,
    Origins,
    Odyssey,
}

pub const KIND_BROTHERHOOD: &str = "Assassin's Creed Brotherhood save";
pub const KIND_REVELATIONS: &str = "Assassin's Creed Revelations save";
pub const KIND_AC3R: &str = "Assassin's Creed III Remastered save";
pub const KIND_LIBERATION: &str = "Assassin's Creed Liberation HD save";
pub const KIND_BLACK_FLAG: &str = "Assassin's Creed IV Black Flag save";
pub const KIND_ROGUE: &str = "Assassin's Creed Rogue save";
pub const KIND_UNITY: &str = "Assassin's Creed Unity save";
pub const KIND_SYNDICATE: &str = "Assassin's Creed Syndicate save";
pub const KIND_ORIGINS: &str = "Assassin's Creed Origins save";
pub const KIND_ODYSSEY: &str = "Assassin's Creed Odyssey save";

impl Ac {
    pub fn kind(self) -> &'static str {
        match self {
            Ac::Brotherhood => KIND_BROTHERHOOD,
            Ac::Revelations => KIND_REVELATIONS,
            Ac::Ac3r => KIND_AC3R,
            Ac::LiberationHd => KIND_LIBERATION,
            Ac::BlackFlag => KIND_BLACK_FLAG,
            Ac::Rogue => KIND_ROGUE,
            Ac::Unity => KIND_UNITY,
            Ac::Syndicate => KIND_SYNDICATE,
            Ac::Origins => KIND_ORIGINS,
            Ac::Odyssey => KIND_ODYSSEY,
        }
    }

    fn name(self) -> &'static str {
        self.kind().trim_end_matches(" save")
    }
}

/// The fixed key of Liberation HD and Black Flag saves.
const KEY_BLACK_FLAG: [u8; 16] = [0xab, 0x8f, 0x71, 0x3a, 0x71, 0x85, 0xc7, 0x85, 0xf7, 0xb9, 0xf4, 0x37, 0x78, 0x2e, 0xe6, 0x6c];

// ------------------------------------------------------------------ checks and packing

/// Adler-32 with both sums starting at 0.
pub fn adler0(d: &[u8]) -> u32 {
    let (mut a, mut b) = (0u32, 0u32);
    for chunk in d.chunks(5000) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// LZO2A, as the older games pack their saves.
fn lzo2a_decode(src: &[u8], want: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(want);
    let (mut ip, mut bits, mut k) = (0usize, 0u32, 0u32);
    macro_rules! need {
        ($j:expr) => {
            while k < $j {
                bits |= (*src.get(ip)? as u32) << k;
                ip += 1;
                k += 8;
            }
        };
    }
    macro_rules! take {
        ($j:expr) => {{
            let v = bits & ((1 << $j) - 1);
            bits >>= $j;
            k -= $j;
            v
        }};
    }
    loop {
        if out.len() > want {
            return None;
        }
        need!(1);
        if take!(1) == 0 {
            out.push(*src.get(ip)?);
            ip += 1;
            continue;
        }
        need!(1);
        if take!(1) == 0 {
            need!(2);
            let len = 2 + take!(2) as usize;
            let dist = 1 + *src.get(ip)? as usize;
            ip += 1;
            let from = out.len().checked_sub(dist)?;
            for i in 0..len {
                out.push(out[from + i]);
            }
            continue;
        }
        let t = *src.get(ip)? as usize;
        let u = *src.get(ip + 1)? as usize;
        ip += 2;
        let dist = (t & 31) | (u << 5);
        let mut len = t >> 5;
        if len == 0 {
            len = 9;
            while *src.get(ip)? == 0 {
                len += 255;
                ip += 1;
            }
            len += src[ip] as usize;
            ip += 1;
        } else {
            if dist == 0 {
                break;
            }
            len += 2;
        }
        let from = out.len().checked_sub(dist).filter(|_| dist > 0)?;
        for i in 0..len {
            out.push(out[from + i]);
        }
    }
    (out.len() == want).then_some(out)
}

/// Packs data as LZO2A (greedy matches up to 8191 bytes back, longest of a few candidates).
fn lzo2a_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 16);
    let (mut flag, mut used) = (0usize, 8u32);
    let mut bit = |out: &mut Vec<u8>, b: u8| {
        if used == 8 {
            flag = out.len();
            out.push(0);
            used = 0;
        }
        out[flag] |= b << used;
        used += 1;
    };
    const SLOTS: usize = 4;
    let mut table = vec![[usize::MAX; SLOTS]; 1 << 14];
    let hash = |i: usize| ((u32::from_le_bytes([data[i], data[i + 1], data[i + 2], 0]).wrapping_mul(2_654_435_761)) >> 18) as usize;
    let mut i = 0;
    while i < data.len() {
        let (mut best_len, mut best_dist) = (0usize, 0usize);
        if i + 3 <= data.len() {
            let h = hash(i);
            for &c in &table[h] {
                if c == usize::MAX || i - c > 8191 {
                    continue;
                }
                let max = (data.len() - i).min(4096);
                let mut l = 0;
                while l < max && data[c + l] == data[i + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = i - c;
                }
            }
            let row = &mut table[h];
            row.rotate_right(1);
            row[0] = i;
        }
        if best_len >= 3 {
            bit(&mut out, 1);
            bit(&mut out, 1);
            if best_len <= 9 {
                out.push((((best_len - 2) << 5) | (best_dist & 31)) as u8);
                out.push((best_dist >> 5) as u8);
            } else {
                out.push((best_dist & 31) as u8);
                out.push((best_dist >> 5) as u8);
                let mut rem = best_len - 9;
                while rem > 255 {
                    out.push(0);
                    rem -= 255;
                }
                out.push(rem as u8);
            }
            for j in i + 1..(i + best_len).min(data.len().saturating_sub(2)) {
                let row = &mut table[hash(j)];
                row.rotate_right(1);
                row[0] = j;
            }
            i += best_len;
        } else {
            bit(&mut out, 0);
            out.push(data[i]);
            i += 1;
        }
    }
    bit(&mut out, 1);
    bit(&mut out, 1);
    out.extend([0x20, 0x00]);
    out
}

fn unpack(ctype: u8, comp: &[u8], us: usize) -> Option<Vec<u8>> {
    if comp.len() == us {
        return Some(comp.to_vec());
    }
    match ctype {
        0 | 1 => std::panic::catch_unwind(|| lzokay_native::decompress_all(comp, Some(us))).ok()?.ok().filter(|v| v.len() == us),
        2 => lzo2a_decode(comp, us),
        _ => None,
    }
}

fn pack(ctype: u8, data: &[u8]) -> Result<Vec<u8>, String> {
    let out = match ctype {
        0 | 1 => lzokay_native::compress(data).map_err(|e| format!("LZO: {e:?}"))?,
        _ => lzo2a_encode(data),
    };
    // Never write a block that doesn't read back to exactly what was meant.
    if out.len() == data.len() || unpack(ctype, &out, data.len()).as_deref() != Some(data) {
        return Err("A part of the save could not be packed again safely".into());
    }
    Ok(out)
}

// ------------------------------------------------------------------ wrapper and key

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// Where the body starts.
fn body_offset(d: &[u8]) -> Option<usize> {
    if d.starts_with(b"CDX\0") {
        return Some(0x110);
    }
    let first = u32_at(d, 0)?;
    if matches!(first, 0x224 | 0x204 | 0x24) && u32_at(d, 4)? == 0 {
        return Some(first as usize + 4);
    }
    if d.len() > 0x220 && d[..0x208].iter().all(|&b| b == 0) {
        return Some(0x208);
    }
    if first == 0x16 && d.get(4..8)? == MAGIC {
        return Some(0);
    }
    None
}

/// Offset of the first container inside the body, by body version.
fn first_container(version: u32) -> Option<usize> {
    Some(match version {
        0x16 => 16,
        0x21 | 0x23 | 0x24 => 24,
        0x2A | 0x2C => 20,
        0x31 => 28,
        _ => return None,
    })
}

const VERSIONS: [u32; 7] = [0x16, 0x21, 0x23, 0x24, 0x2A, 0x2C, 0x31];

fn xor(d: &mut [u8], from: usize, n: usize, key: &[u8]) {
    if key.is_empty() {
        return;
    }
    for (i, b) in d.iter_mut().enumerate() {
        *b ^= key[(n - (from + i)) % key.len()];
    }
}

/// Does the first block at `s` (body offset of a container) check out with this key?
fn first_block_ok(d: &[u8], o: usize, s: usize, key: &[u8]) -> bool {
    let n = d.len();
    let at = o + s + 15;
    let Some(head) = d.get(at..at + 13) else { return false };
    let mut head = head.to_vec();
    xor(&mut head, at, n, key);
    if head[0] != 1 {
        return false;
    }
    let cs = u32::from_le_bytes(head[1..5].try_into().unwrap()) as usize;
    let ck = u32::from_le_bytes(head[9..13].try_into().unwrap());
    let Some(comp) = d.get(at + 13..at + 13 + cs).filter(|_| cs > 0) else { return false };
    let mut comp = comp.to_vec();
    xor(&mut comp, at + 13, n, key);
    adler0(&comp) == ck
}

/// The XOR key (empty: not encrypted) and the body version.
fn solve(d: &[u8], o: usize) -> Option<(Vec<u8>, u32)> {
    let n = d.len();
    let body = d.get(o..)?;
    if body.len() < 64 {
        return None;
    }
    // Not encrypted.
    let v = u32_at(body, 0)?;
    if body[4..8] == MAGIC {
        if let Some(s) = first_container(v) {
            if body.get(s..s + 8) == Some(&SIG) && first_block_ok(d, o, s, &[]) {
                return Some((vec![], v));
            }
        }
    }
    for version in VERSIONS {
        let s = first_container(version)?;
        for cver in [1u16, 2] {
            for ctype in [0u8, 2] {
                for bsize in [0x8000u32, 0x40000] {
                    let mut known: Vec<(usize, u8)> = vec![];
                    known.extend(version.to_le_bytes().iter().enumerate().map(|(i, &b)| (i, b)));
                    known.extend(MAGIC.iter().enumerate().map(|(i, &b)| (4 + i, b)));
                    let mut c = SIG.to_vec();
                    c.extend(cver.to_le_bytes());
                    c.push(ctype);
                    c.extend(bsize.to_le_bytes());
                    c.push(1);
                    known.extend(c.iter().enumerate().map(|(i, &b)| (s + i, b)));
                    'len: for l in 1..=16usize {
                        let mut key: Vec<Option<u8>> = vec![None; l];
                        for &(i, p) in &known {
                            let at = o + i;
                            let kb = d[at] ^ p;
                            let slot = &mut key[(n - at) % l];
                            match slot {
                                Some(x) if *x != kb => continue 'len,
                                _ => *slot = Some(kb),
                            }
                        }
                        let Some(key) = key.into_iter().collect::<Option<Vec<u8>>>() else { continue };
                        if first_block_ok(d, o, s, &key) {
                            return Some((key, version));
                        }
                    }
                }
            }
        }
    }
    None
}

// ------------------------------------------------------------------ containers

#[derive(Clone, Debug)]
struct Block {
    stored: bool,
    /// Bytes as in the file (packed, or raw for a stored block).
    bytes: Vec<u8>,
    size: usize,
}

#[derive(Clone, Debug)]
struct Container {
    /// Body offset of the signature.
    pos: usize,
    /// Body offset just past the last block.
    end: usize,
    ctype: u8,
    blocks: Vec<Block>,
    /// Unpacked data, and where each block starts in it.
    data: Vec<u8>,
    starts: Vec<usize>,
    /// Body offset of the u32 holding the container's size in the file, if it has one.
    size_field: Option<usize>,
}

fn read_container(b: &[u8], pos: usize, lower: usize) -> Option<Container> {
    if b.get(pos..pos + 8)? != SIG {
        return None;
    }
    let ctype = b[pos + 10];
    let total = u32_at(b, pos.checked_sub(4)?)? as usize;
    let mut q = pos + 15;
    let (mut blocks, mut data, mut starts) = (vec![], vec![], vec![]);
    while data.len() < total {
        let kind = *b.get(q)?;
        let (stored, size, bytes) = match kind {
            1 => {
                let cs = u32_at(b, q + 1)? as usize;
                let us = u32_at(b, q + 5)? as usize;
                let ck = u32_at(b, q + 9)?;
                let comp = b.get(q + 13..q + 13 + cs)?;
                if cs == 0 || adler0(comp) != ck {
                    return None;
                }
                q += 13 + cs;
                (false, us, comp)
            }
            0 => {
                let sz = u32_at(b, q + 1)? as usize;
                if sz == 0 {
                    return None;
                }
                let raw = b.get(q + 5..q + 5 + sz)?;
                q += 5 + sz;
                (true, sz, raw)
            }
            _ => return None,
        };
        let plain = if stored { bytes.to_vec() } else { unpack(ctype, bytes, size)? };
        starts.push(data.len());
        data.extend(plain);
        blocks.push(Block { stored, bytes: bytes.to_vec(), size });
    }
    if data.len() != total {
        return None;
    }
    let clen = (q - pos) as i64;
    let size_field = [8usize, 16].into_iter().filter_map(|back| pos.checked_sub(back)).filter(|&f| f >= lower).find(|&f| {
        let v = u32_at(b, f).map_or(-1, |v| v as i64);
        matches!(v - clen, 4 | 12)
    });
    Some(Container { pos, end: q, ctype, blocks, data, starts, size_field })
}

fn containers(b: &[u8], first: usize) -> Option<Vec<Container>> {
    let mut out: Vec<Container> = vec![];
    let mut pos = first;
    loop {
        let lower = out.last().map_or(0, |c| c.end);
        out.push(read_container(b, pos, lower)?);
        let end = out.last().unwrap().end;
        if b.get(end + 16..end + 24) == Some(&SIG) {
            pos = end + 16;
        } else {
            break;
        }
    }
    Some(out)
}

// ------------------------------------------------------------------ property stream

/// Offsets of the values of leaves with this name hash and type.
fn leaves(d: &[u8], key: u32, ty: u16) -> Vec<usize> {
    let k = key.to_le_bytes();
    let mut out = vec![];
    let mut p = 4;
    while p + 13 <= d.len() {
        let Some(i) = d[p..].windows(4).position(|w| w == k) else { break };
        let at = p + i;
        if at + 13 <= d.len() && d[at + 8] == 0 && d[at + 9] == 0 && u16::from_le_bytes([d[at + 10], d[at + 11]]) == ty && d[at + 12] == 0x0B {
            out.push(at + 13);
        }
        p = at + 1;
    }
    out
}

/// Offsets of the values of object arrays with this name hash (u8 1, u32 entry count, entries).
fn arrays(d: &[u8], key: u32) -> Vec<usize> {
    [0x0A1D, 0x0A9D, 0x0B1D].into_iter().flat_map(|t| leaves(d, key, t)).collect()
}

const T_U32: u16 = 7;
const T_ENUM: u16 = 0x19;
const T_ID: u16 = 0x12;
const K_CATEGORY: u32 = 0xFF3A_7B97;
const K_LIST: u32 = 0x303E_B186;
/// The enum before an inventory list in Brotherhood and Revelations naming whose it is.
const K_OWNER: u32 = 0xB159_0889;
const OWNER_PLAYER: u32 = 0x4424_9429;
const T_LIST: u16 = 0x0A1D;
const K_COUNT: u32 = 0xF77B_9912;
const K_QUANTITY: u32 = 0x6685_7460;
const K_ITEM_ID: u32 = 0xBF29_8A20;
/// An inventory's item list (Odyssey).
const K_ITEMS: u32 = 0x20DF_C649;
const K_XP: u32 = 0x58D0_9457;
const K_POINTS: u32 = 0x6E5A_0AC8;

/// A place a value is kept: (container, offset in its data).
type Spot = (usize, usize);

/// Where a section of the screen puts an inventory category.
#[derive(Clone, Copy, PartialEq)]
enum Group {
    Money,
    Materials,
    Trade,
    Tools,
}

/// `InvItem_*` category names (CRC-32 of the name) the screens show.
const CATEGORIES: &[(u32, &str, Group)] = &[
    (0x12F6_B6F1, "Money", Group::Money),
    (0xD98A_C0FF, "Creed Points", Group::Money),
    (0x1F8D_672A, "Wood", Group::Materials),
    (0x654B_51DC, "Metal", Group::Materials),
    (0x87D8_CEAB, "Cloth", Group::Materials),
    (0xBC09_A03F, "Stone", Group::Materials),
    (0xBD24_D087, "Leather", Group::Materials),
    (0x597C_302F, "Metal", Group::Materials),
    (0x762F_6E61, "Chemicals", Group::Materials),
    (0x1B75_E0B9, "Silk", Group::Materials),
    (0x0066_6D8D, "Rum", Group::Trade),
    (0x0EED_0FEA, "Sugar", Group::Trade),
    (0x54C4_070C, "Tobacco", Group::Trade),
    (0x7121_9CE3, "Smoke bombs", Group::Tools),
    (0x5F5F_4957, "Throwing knives", Group::Tools),
    (0x940C_D407, "Poison", Group::Tools),
    (0x248B_BD27, "Rope darts", Group::Tools),
    (0x27AA_9908, "Berserk darts", Group::Tools),
    (0x27CE_7DE3, "Medicine", Group::Tools),
    (0xB7CF_2800, "Arrows", Group::Tools),
    (0xC717_2DAB, "Musket bullets", Group::Tools),
    (0xB7C1_34C5, "Gun bullets", Group::Tools),
    (0x5815_7823, "Cherry bombs", Group::Tools),
    (0x5DD4_6D4D, "Stun grenades", Group::Tools),
    (0xFB35_7AE1, "Voltaic bombs", Group::Tools),
    (0x6F44_6793, "Snares", Group::Tools),
    (0xEE89_98D9, "Mortar shots", Group::Tools),
    (0xED3E_BACD, "Heavy shot", Group::Tools),
    (0x945D_B140, "Harpoons", Group::Tools),
    (0xABFC_3096, "Burning oil", Group::Tools),
    (0x6D1E_3297, "Lockpicks", Group::Tools),
    (0xAA59_3E95, "Crew", Group::Tools),
];

/// Odyssey and Origins item ids.
const DRACHMAE: u64 = 0x0000_001B_FDB9_F655;
const ITEMS: &[(u64, &str)] = &[
    (0x0000_0150_408F_7320, "Olive Wood"),
    (0x0000_0150_408F_900F, "Iron Metal"),
    (0x0000_0150_6FB5_C135, "Soft Leather"),
    (0x0000_0073_12FA_86C4, "Hard Leather"),
    (0x0000_0150_408F_9013, "Obsidian Glass"),
    (0x0000_0150_6FB5_C133, "Precious Gems"),
    (0x0000_0150_6FB5_C137, "Ancient Tablets"),
];

/// The most money the screens allow.
pub const MONEY_MAX: i64 = 99_999_999;
pub const DRACHMAE_MAX: i64 = 9_999_999;
/// Black Flag: the Jackdaw's hold, fully upgraded, takes 2,000 of each cargo.
pub const BLACK_FLAG_CARGO: i64 = 2_000;
/// Rogue: the Morrigan's hold, fully upgraded, takes 2,500.
pub const ROGUE_CARGO: i64 = 2_500;
/// Syndicate: no public source states a limit for crafting materials (estimate).
pub const SYNDICATE_MATERIALS: i64 = 9_999;
/// Odyssey: no public source states a limit; real saves hold over 60,000 (estimate).
pub const ODYSSEY_MATERIALS: i64 = 99_999;
pub const TOOLS_MAX: i64 = 999;
pub const XP_MAX: i64 = 99_999_999;
pub const POINTS_MAX: i64 = 9_999;

fn cap(game: Ac, name: &str, group: Group) -> i64 {
    match group {
        Group::Money => MONEY_MAX,
        Group::Materials | Group::Trade => match game {
            Ac::BlackFlag => BLACK_FLAG_CARGO,
            Ac::Rogue => ROGUE_CARGO,
            Ac::Syndicate => SYNDICATE_MATERIALS,
            _ if name == "Stone" => ROGUE_CARGO,
            _ => 9_999,
        },
        Group::Tools => TOOLS_MAX,
    }
}

#[derive(Clone, Debug)]
struct Leaf {
    spots: Vec<Spot>,
    max: i64,
}

/// The live copy of an XP / ability point counter: the non-zero one of the first two.
fn live(conts: &[Container], all: &[Spot]) -> Option<Spot> {
    let read = |&(c, o): &Spot| u32_at(&conts[c].data, o).unwrap_or(0);
    all.iter().take(2).find(|s| read(s) != 0).or(all.first()).copied()
}

// ------------------------------------------------------------------ the save

#[derive(Clone, Debug)]
struct Save {
    orig: Vec<u8>,
    off: usize,
    key: Vec<u8>,
    /// Decrypted body.
    body: Vec<u8>,
    conts: Vec<Container>,
    leaves: Vec<Leaf>,
    /// The CDX wrapper keeps the body size at 0x08.
    cdx: bool,
}

fn identify(version: u32, cver: u16, key: &[u8]) -> Option<Ac> {
    Some(match version {
        0x16 if key.is_empty() => Ac::Brotherhood,
        0x16 => Ac::Revelations,
        0x21 if key == KEY_BLACK_FLAG => Ac::LiberationHd,
        0x21 => Ac::Ac3r,
        0x23 => Ac::BlackFlag,
        0x24 => Ac::Rogue,
        0x2A if cver >= 2 => Ac::Syndicate,
        0x2A => Ac::Unity,
        0x2C => Ac::Origins,
        0x31 => Ac::Odyssey,
        _ => return None,
    })
}

pub fn looks(d: &[u8]) -> bool {
    body_offset(d).is_some_and(|o| d.len() > o + 64)
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let off = body_offset(d)?;
    let (key, version) = solve(d, off)?;
    let mut body = d[off..].to_vec();
    xor(&mut body, off, d.len(), &key);
    let first = first_container(version)?;
    let conts = containers(&body, first)?;
    let cver = u16::from_le_bytes([body[first + 8], body[first + 9]]);
    let game = identify(version, cver, &key)?;
    let cdx = d.starts_with(b"CDX\0") && u32_at(d, 8) == Some(body.len() as u32);

    let mut leaves_out: Vec<Leaf> = vec![];
    let mut edit = HashMap::new();
    let mut root = Map::new();
    let mut info = Map::new();
    info.insert("Game".into(), Value::from(game.name()));
    let mut money = Map::new();
    let mut materials = Map::new();
    let mut trade = Map::new();
    let mut tools = Map::new();
    let mut progress = Map::new();
    let read = |conts: &[Container], (c, o): Spot| u32_at(&conts[c].data, o).unwrap_or(0) as i64;
    let mut add = |section: &str, name: &str, spots: Vec<Spot>, max: i64, map: &mut Map<String, Value>, leaves_out: &mut Vec<Leaf>| {
        let v = read(&conts, spots[0]);
        map.insert(name.into(), Value::from(v));
        leaves_out.push(Leaf { spots, max: max.max(v) });
        edit.insert(format!("/{section}/{}", name.replace('~', "~0").replace('/', "~1")), leaves_out.len() - 1);
    };

    if matches!(game, Ac::Origins | Ac::Odyssey) {
        // Item list: an id, then (in the same entry) a Quantity. Chests, merchants and loot have
        // item lists too; the player's is the one with the most entries.
        let mut player: Option<(usize, usize, usize, u32)> = None;
        for (ci, c) in conts.iter().enumerate() {
            for at in arrays(&c.data, K_ITEMS) {
                let (Some(len), Some(n)) = (at.checked_sub(17).and_then(|l| u32_at(&c.data, l)), u32_at(&c.data, at + 1)) else { continue };
                if player.is_none_or(|p| n > p.3) {
                    player = Some((ci, at, (at - 13 + len as usize).min(c.data.len()), n));
                }
            }
        }
        let mut by_id: Vec<(u64, Vec<Spot>)> = vec![];
        for (ci, c) in conts.iter().enumerate() {
            let ids = leaves(&c.data, K_ITEM_ID, T_ID);
            if ids.is_empty() {
                continue;
            }
            for q in leaves(&c.data, K_QUANTITY, T_U32) {
                if let Some((pc, a, e, _)) = player {
                    if pc != ci || !(a..e).contains(&q) {
                        continue;
                    }
                }
                // The nearest id before the quantity, within one entry.
                let Some(&i) = ids.iter().rev().find(|&&i| i < q && q - i < 1400) else { continue };
                let Some(id) = c.data.get(i..i + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap())) else { continue };
                match by_id.iter_mut().find(|(x, _)| *x == id) {
                    Some((_, s)) => s.push((ci, q)),
                    None => by_id.push((id, vec![(ci, q)])),
                }
            }
        }
        // Without a player list, an id listed more than once (a pickup holds one drachma) is the
        // player's where its stack is biggest.
        let pick = |s: &Vec<Spot>| {
            if player.is_some() {
                s.first().map(|&x| vec![x])
            } else {
                s.iter().copied().max_by_key(|&(c, o)| u32_at(&conts[c].data, o).unwrap_or(0)).map(|x| vec![x])
            }
        };
        if let Some(s) = by_id.iter().find(|(id, _)| *id == DRACHMAE).and_then(|(_, s)| pick(s)) {
            add("Money", "Drachmae", s, DRACHMAE_MAX, &mut money, &mut leaves_out);
        }
        for &(id, name) in ITEMS {
            if game == Ac::Origins && name != "Hard Leather" {
                continue;
            }
            if let Some(s) = by_id.iter().find(|(x, _)| *x == id).and_then(|(_, s)| pick(s)) {
                add("Crafting materials", name, s, ODYSSEY_MATERIALS, &mut materials, &mut leaves_out);
            }
        }
        for (key, name, max) in [(K_XP, "XP", XP_MAX), (K_POINTS, "Ability points", POINTS_MAX)] {
            let all: Vec<Spot> = conts.iter().enumerate().flat_map(|(ci, c)| leaves(&c.data, key, T_U32).into_iter().map(move |o| (ci, o))).collect();
            if let Some(s) = live(&conts, &all) {
                add("Progress", name, vec![s], max, &mut progress, &mut leaves_out);
            }
        }
    } else {
        // Inventory container lists in the main stream; each entry is a Category enum, then
        // CurrentItemCount. Lists of other characters and memories are left alone.
        let (mi, main) = conts.iter().enumerate().max_by_key(|(_, c)| c.data.len())?;
        let data = &main.data;
        let counts = leaves(data, K_COUNT, T_U32);
        let owners = leaves(data, K_OWNER, T_ENUM);
        // (start, end, entries, owner hash)
        let lists: Vec<(usize, usize, u32, u32)> = leaves(data, K_LIST, T_LIST)
            .into_iter()
            .filter_map(|at| {
                let len = u32_at(data, at.checked_sub(17)?)? as usize;
                let owner = owners.iter().rev().find(|&&o| o < at && at - o < 240).and_then(|&o| u32_at(data, o + 4)).unwrap_or(0);
                Some((at, (at - 13 + len).min(data.len()), u32_at(data, at + 1)?, owner))
            })
            .collect();
        let chosen: Vec<(usize, usize)> = if lists.iter().any(|l| l.3 == OWNER_PLAYER) {
            // Brotherhood and Revelations name the player's inventory.
            lists.iter().filter(|l| l.3 == OWNER_PLAYER).map(|l| (l.0, l.1)).collect()
        } else {
            // The player's list has the most entries (the biggest of those); its checkpoint
            // copies have the same entries and about the same size.
            let &(a, e, n, _) = lists.iter().max_by_key(|l| (l.2, l.1 - l.0, std::cmp::Reverse(l.0)))?;
            let mut v = vec![(a, e)];
            v.extend(lists.iter().filter(|l| l.0 != a && l.2 == n && (l.1 - l.0) * 4 >= (e - a) * 3).map(|l| (l.0, l.1)));
            v
        };
        let mut by_cat: Vec<(u32, Vec<Spot>)> = vec![];
        for at in leaves(data, K_CATEGORY, T_ENUM) {
            if !chosen.iter().any(|&(a, e)| (a..e).contains(&at)) {
                continue;
            }
            let Some(hash) = u32_at(data, at + 4) else { continue };
            let Some(&cnt) = counts.iter().find(|&&c| c > at && c - at < 80) else { continue };
            // The best list's copy goes first: it is the one shown.
            let first = (chosen[0].0..chosen[0].1).contains(&at);
            match by_cat.iter_mut().find(|(h, _)| *h == hash) {
                Some((_, s)) if first => s.insert(0, (mi, cnt)),
                Some((_, s)) => s.push((mi, cnt)),
                None => by_cat.push((hash, vec![(mi, cnt)])),
            }
        }
        for &(hash, name, group) in CATEGORIES {
            let Some((_, s)) = by_cat.iter().find(|(h, _)| *h == hash) else { continue };
            let (section, map) = match group {
                Group::Money => ("Money", &mut money),
                Group::Materials => ("Crafting materials", &mut materials),
                Group::Trade => ("Trade goods", &mut trade),
                Group::Tools => ("Tools and ammunition", &mut tools),
            };
            if map.contains_key(name) {
                continue;
            }
            add(section, name, s.clone(), cap(game, name, group), map, &mut leaves_out);
        }
    }
    if money.is_empty() {
        return None;
    }
    root.insert("Save".into(), Value::Object(info));
    root.insert("Money".into(), Value::Object(money));
    for (k, m) in [("Progress", progress), ("Crafting materials", materials), ("Trade goods", trade), ("Tools and ammunition", tools)] {
        if !m.is_empty() {
            root.insert(k.into(), Value::Object(m));
        }
    }
    let save = Save { orig: d.to_vec(), off, key, body, conts, leaves: leaves_out, cdx };
    Some(TreeDoc::new(Value::Object(root), edit, game.kind(), Box::new(B(save))))
}

#[derive(Clone, Debug)]
struct B(Save);

impl B {
    fn rebuild(&self, conts: &[Container]) -> Result<Vec<u8>, String> {
        let s = &self.0;
        let mut out = Vec::with_capacity(s.body.len() + 4096);
        let mut from = 0;
        for (old, new) in s.conts.iter().zip(conts) {
            let mut bytes = s.body[old.pos..old.pos + 15].to_vec();
            for nb in &new.blocks {
                if nb.stored {
                    bytes.push(0);
                    bytes.extend((nb.size as u32).to_le_bytes());
                } else {
                    bytes.push(1);
                    bytes.extend((nb.bytes.len() as u32).to_le_bytes());
                    bytes.extend((nb.size as u32).to_le_bytes());
                    bytes.extend(adler0(&nb.bytes).to_le_bytes());
                }
                bytes.extend(&nb.bytes);
            }
            let delta = bytes.len() as i64 - (old.end - old.pos) as i64;
            let gap_start = out.len();
            out.extend(&s.body[from..old.pos]);
            if delta != 0 {
                let f = old.size_field.ok_or("The save's layout isn't fully understood here, so it can't be changed")?;
                let at = gap_start + (f - from);
                let v = u32_at(&out, at).unwrap() as i64 + delta;
                out[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
            }
            out.extend(bytes);
            from = old.end;
        }
        out.extend(&s.body[from..]);
        Ok(out)
    }
}

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let s = &self.0;
        if changes.is_empty() {
            return Ok(s.orig.clone());
        }
        let mut conts = s.conts.clone();
        for (leaf, v) in changes {
            let l = s.leaves.get(*leaf).ok_or("Not editable")?;
            let n = v.as_i64().filter(|n| (0..=l.max).contains(n)).ok_or("Value out of range")?;
            for &(c, o) in &l.spots {
                conts[c].data[o..o + 4].copy_from_slice(&(n as u32).to_le_bytes());
            }
        }
        for (ci, c) in conts.iter_mut().enumerate() {
            let old = &s.conts[ci];
            if c.data == old.data {
                continue;
            }
            for bi in 0..c.blocks.len() {
                let (a, len) = (c.starts[bi], c.blocks[bi].size);
                let plain = &c.data[a..a + len];
                if plain == &old.data[a..a + len] {
                    continue;
                }
                let b = &mut c.blocks[bi];
                b.bytes = if b.stored || b.bytes.len() == len { plain.to_vec() } else { pack(c.ctype, plain)? };
            }
        }
        let mut body = self.rebuild(&conts)?;
        // Read it back before handing it out.
        let first = s.conts[0].pos;
        let back = containers(&body, first).ok_or("The save could not be packed again safely")?;
        if back.len() != conts.len() || back.iter().zip(&conts).any(|(a, b)| a.data != b.data) {
            return Err("The save could not be packed again safely".into());
        }
        let n = s.off + body.len();
        xor(&mut body, s.off, n, &s.key);
        let mut out = s.orig[..s.off].to_vec();
        if s.cdx {
            out[8..12].copy_from_slice(&(body.len() as u32).to_le_bytes());
        }
        out.extend(body);
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let max = self.0.leaves.get(leaf).map_or(0, |l| l.max);
        match v.as_i64() {
            Some(n) if (0..=max).contains(&n) => Ok(()),
            _ => Err(format!("Must be a whole number from 0 to {max}")),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(key: u32, ty: u16, value: &[u8]) -> Vec<u8> {
        let mut v = ((13 + value.len()) as u32).to_le_bytes().to_vec();
        v.extend(key.to_le_bytes());
        v.extend([0u8; 6]);
        v.extend(ty.to_le_bytes());
        v.push(0x0B);
        v.extend(value);
        v
    }

    fn entry(hash: u32, count: u32) -> Vec<u8> {
        let mut e = leaf(K_CATEGORY, T_ENUM, &[1, 0, 0, 0, hash as u8, (hash >> 8) as u8, (hash >> 16) as u8, (hash >> 24) as u8]);
        e.extend(leaf(0x6AF7_D44F, T_U32, &0u32.to_le_bytes()));
        e.extend(leaf(K_COUNT, T_U32, &count.to_le_bytes()));
        e
    }

    /// Game data: two copies of the inventory (live and snapshot) between filler, about 70 KiB.
    fn stream() -> Vec<u8> {
        let mut d = vec![0u8; 14];
        let mut x = 0x1234_5678u32;
        for copy in 0..2u32 {
            for i in 0..30_000u32 {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                d.push(if i % 4 == 0 { (x >> 24) as u8 } else { b"PlayerProgression"[(i % 17) as usize] });
            }
            let mut list = vec![1u8];
            list.extend(4u32.to_le_bytes());
            list.extend(entry(0x12F6_B6F1, 1_000 + copy));
            list.extend(entry(0x1F8D_672A, 150));
            list.extend(entry(0xDEAD_BEEF, 7));
            list.extend(entry(0x7121_9CE3, 0));
            d.extend(leaf(K_LIST, T_LIST, &list));
        }
        // Another character's smaller list is not the player's.
        let mut other = vec![1u8];
        other.extend(1u32.to_le_bytes());
        other.extend(entry(0x12F6_B6F1, 55));
        d.extend(leaf(K_LIST, T_LIST, &other));
        d.extend(vec![0u8; 5000]);
        d
    }

    fn container(ctype: u8, data: &[u8], bsize: usize) -> Vec<u8> {
        let mut c = SIG.to_vec();
        c.extend(1u16.to_le_bytes());
        c.push(ctype);
        c.extend((bsize as u32).to_le_bytes());
        for part in data.chunks(bsize) {
            let z = if ctype == 2 { lzo2a_encode(part) } else { lzokay_native::compress(part).unwrap() };
            c.push(1);
            c.extend((z.len() as u32).to_le_bytes());
            c.extend((part.len() as u32).to_le_bytes());
            c.extend(adler0(&z).to_le_bytes());
            c.extend(z);
        }
        c
    }

    /// A Black Flag save as a crack writes it: 0x208 zero bytes, then the encrypted body.
    fn black_flag() -> Vec<u8> {
        let head = vec![0x42u8; 230];
        let h = container(0, &head, 0x8000);
        let main = stream();
        let m = container(0, &main, 0x8000);
        let mut body = 0x23u32.to_le_bytes().to_vec();
        body.extend(MAGIC);
        body.extend([0u8; 8]);
        body.extend(((h.len() + 4) as u32).to_le_bytes());
        body.extend((head.len() as u32).to_le_bytes());
        body.extend(h);
        body.extend(((m.len() + 12) as u32).to_le_bytes());
        body.extend(3u32.to_le_bytes());
        body.extend([0, 0xFE, 0xCA, 0]);
        body.extend((main.len() as u32).to_le_bytes());
        body.extend(m);
        let mut d = vec![0u8; 0x208];
        let n = 0x208 + body.len();
        xor(&mut body, 0x208, n, &KEY_BLACK_FLAG);
        d.extend(body);
        d
    }

    #[test]
    fn adler0_starts_both_sums_at_zero() {
        assert_eq!(adler0(b""), 0);
        assert_eq!(adler0(b"a"), (97 << 16) | 97);
        let big = vec![0xFFu8; 100_000];
        let (mut a, mut b) = (0u64, 0u64);
        for &x in &big {
            a = (a + x as u64) % 65521;
            b = (b + a) % 65521;
        }
        assert_eq!(adler0(&big), ((b << 16) | a) as u32);
    }

    #[test]
    fn lzo2a_round_trips() {
        let mut x = 7u32;
        for n in [0usize, 1, 2, 3, 7, 8, 9, 16, 1000, 32768] {
            let d: Vec<u8> = (0..n)
                .map(|i| {
                    x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    if i % 5 == 0 { (x >> 24) as u8 } else { b"abcabcabd"[i % 9] }
                })
                .collect();
            let z = lzo2a_encode(&d);
            assert_eq!(lzo2a_decode(&z, n).unwrap(), d, "{n}");
            if n > 1000 {
                assert!(z.len() < n * 3 / 4, "{n}: packs to {}", z.len());
            }
        }
        let zeros = vec![0u8; 100_000];
        assert_eq!(lzo2a_decode(&lzo2a_encode(&zeros), zeros.len()).unwrap(), zeros);
        // A short match: "ab" then copy 3 from 2 back gives "ababa".
        // Flag bits from the low end: 0 0 (two literals), 1 0 (short match), 1 0 (length 3),
        // then 1 1 (long form; length 1 and distance 0 is the end).
        let s = [0b1101_0100u8, b'a', b'b', 1, 0x20, 0];
        assert_eq!(lzo2a_decode(&s, 5).unwrap(), b"ababa");
    }

    #[test]
    fn key_is_found_and_money_edited() {
        let d = black_flag();
        assert!(looks(&d));
        let mut doc = parse(&d).expect("parsed");
        assert_eq!(doc.kind_name, KIND_BLACK_FLAG);
        assert_eq!(doc.view.pointer("/Money/Money").unwrap(), 1000);
        assert_eq!(doc.view.pointer("/Crafting materials/Wood").unwrap(), 150);
        assert_eq!(doc.view.pointer("/Tools and ammunition/Smoke bombs").unwrap(), 0);
        assert_eq!(doc.to_bytes().unwrap(), d, "unchanged save is written back as it was");
        assert!(doc.set("/Crafting materials/Wood", Value::from(2001)).is_err());
        doc.set("/Money/Money", Value::from(5_000_000)).unwrap();
        doc.set("/Crafting materials/Wood", Value::from(2000)).unwrap();
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).expect("edited save opens");
        assert_eq!(back.view.pointer("/Money/Money").unwrap(), 5_000_000);
        assert_eq!(back.view.pointer("/Crafting materials/Wood").unwrap(), 2000);
        // Both copies changed; the unnamed category kept its count.
        let s = &back_save(&out).conts[1].data;
        let moneys: Vec<u32> = leaves(s, K_CATEGORY, T_ENUM)
            .into_iter()
            .filter(|&a| u32_at(s, a + 4) == Some(0x12F6_B6F1))
            .map(|a| u32_at(s, leaves(s, K_COUNT, T_U32).into_iter().find(|&c| c > a).unwrap()).unwrap())
            .collect();
        assert_eq!(moneys, [5_000_000, 5_000_000, 55]);
        assert_eq!(s.windows(4).filter(|w| *w == 0xDEAD_BEEFu32.to_le_bytes()).count(), 2);
    }

    fn back_save(d: &[u8]) -> Save {
        let off = body_offset(d).unwrap();
        let (key, version) = solve(d, off).unwrap();
        let mut body = d[off..].to_vec();
        xor(&mut body, off, d.len(), &key);
        let conts = containers(&body, first_container(version).unwrap()).unwrap();
        Save { orig: d.to_vec(), off, key, body, conts, leaves: vec![], cdx: false }
    }

    #[test]
    fn short_user_name_key_and_lzo2a() {
        // Revelations: version 0x16, LZO2A, XORed with the account name.
        let head = vec![0x11u8; 200];
        let main = stream();
        let h = container(2, &head, 0x8000);
        let m = container(2, &main, 0x8000);
        let mut body = 0x16u32.to_le_bytes().to_vec();
        body.extend(MAGIC);
        body.extend(((h.len() + 4) as u32).to_le_bytes());
        body.extend((head.len() as u32).to_le_bytes());
        body.extend(h);
        body.extend(((m.len() + 12) as u32).to_le_bytes());
        body.extend(1u32.to_le_bytes());
        body.extend([0, 0xFE, 0xCA, 0]);
        body.extend((main.len() as u32).to_le_bytes());
        body.extend(m);
        let mut d = 0x204u32.to_le_bytes().to_vec();
        d.extend([0u8; 0x204]);
        let n = d.len() + body.len();
        let at = d.len();
        xor(&mut body, at, n, b"player7");
        d.extend(body);
        let mut doc = parse(&d).expect("parsed");
        assert_eq!(doc.kind_name, KIND_REVELATIONS);
        doc.set("/Money/Money", Value::from(77_777)).unwrap();
        let back = parse(&doc.to_bytes().unwrap()).unwrap();
        assert_eq!(back.view.pointer("/Money/Money").unwrap(), 77_777);
    }

    #[test]
    fn other_files_are_not_taken() {
        let mut d = black_flag();
        let n = d.len();
        d[n - 100] ^= 0x55;
        assert!(parse(&d).is_none(), "a damaged block fails its check");
        assert!(parse(&[0u8; 4096]).is_none());
    }

    /// Research: AC_SAVES lists real saves (separated by `|`); each opens, writes back unchanged,
    /// and takes a money edit.
    #[test]
    #[ignore]
    fn real_saves() {
        let list = std::env::var("AC_SAVES").expect("AC_SAVES");
        for f in list.split('|').filter(|s| !s.is_empty()) {
            let d = std::fs::read(f).unwrap();
            let Some(mut doc) = parse(&d) else {
                println!("{f}: not opened");
                continue;
            };
            assert_eq!(doc.to_bytes().unwrap(), d, "{f}");
            println!("{f}: {} {}", doc.kind_name, serde_json::to_string(&doc.view).unwrap());
            let money = doc.view["Money"].as_object().unwrap().keys().next().unwrap().clone();
            let ptr = format!("/Money/{money}");
            let was = doc.view.pointer(&ptr).unwrap().as_i64().unwrap();
            doc.set(&ptr, Value::from(if was == 123_456 { 654_321 } else { 123_456 })).unwrap();
            let mut extra = None;
            if let Some(m) = doc.view.get("Crafting materials").and_then(Value::as_object) {
                if let Some(k) = m.keys().next() {
                    extra = Some(format!("/Crafting materials/{k}"));
                }
            }
            if let Some(p) = &extra {
                doc.set(p, Value::from(321)).unwrap();
            }
            let out = doc.to_bytes().unwrap();
            let back = parse(&out).unwrap_or_else(|| panic!("{f}: edited save doesn't open"));
            assert_eq!(back.view.pointer(&ptr).unwrap().as_i64().unwrap(), if was == 123_456 { 654_321 } else { 123_456 });
            if let Some(p) = &extra {
                assert_eq!(back.view.pointer(p).unwrap(), 321);
            }
            println!("  edited: {} -> {} bytes", d.len(), out.len());
            if let Ok(dir) = std::env::var("AC_OUT") {
                let name = std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string();
                std::fs::write(std::path::Path::new(&dir).join(format!("{}-{name}", back.kind_name.replace(' ', "_"))), &out).unwrap();
            }
        }
    }
}
