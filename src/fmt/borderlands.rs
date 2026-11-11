//! Borderlands 2 and Borderlands: The Pre-Sequel characters (`SaveNNNN.sav`).
//!
//! The file is a SHA-1 of the rest, the unpacked size (big-endian) and one LZO1X block;
//! that outer part is a `Layer` in `formats.rs` (`unpack` / `pack` here). Inside is the
//! "WSG" blob read here: its own size, "WSG", version 2, the CRC-32 and size of the
//! character data, then that data Huffman-coded (the tree first, pre-order: 1 + byte for
//! a leaf, 0 for a branch; bits most significant first). The character data is a
//! protobuf message (`WillowTwoPlayerSaveGame`).
//!
//! Numbers in a protobuf change size, so an edit rewrites the fields it touches and
//! codes the message again (new Huffman tree, CRC, sizes); everything else is copied as
//! it was. Field numbers and the container from Gibbed's Borderlands 2 tools (zlib); the
//! black market (SDU) order, currency slots, ammo capacities and item header bit widths
//! from apocalyptech's borderlands2 (facts only), checked against sample saves.

use crate::tree::{Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

// ------------------------------------------------------------------ protobuf wire format

/// A tiny protobuf reader/writer: fields are kept as byte spans so a message can be
/// written back with only the edited fields re-encoded.
pub mod pb {
    #[derive(Clone, Copy, Debug)]
    pub enum Val<'a> {
        Int(u64),
        F32([u8; 4]),
        /// A 64-bit value (none are edited here).
        F64,
        Bytes(&'a [u8]),
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Field<'a> {
        pub num: u32,
        pub val: Val<'a>,
        /// The whole field (key included) in the message.
        pub start: usize,
        pub end: usize,
    }

    impl<'a> Field<'a> {
        pub fn int(&self) -> Option<u64> {
            match self.val {
                Val::Int(n) => Some(n),
                _ => None,
            }
        }
        pub fn bytes(&self) -> Option<&'a [u8]> {
            match self.val {
                Val::Bytes(b) => Some(b),
                _ => None,
            }
        }
        pub fn f32(&self) -> Option<f32> {
            match self.val {
                Val::F32(b) => Some(f32::from_le_bytes(b)),
                _ => None,
            }
        }
        pub fn text(&self) -> Option<String> {
            self.bytes().map(|b| String::from_utf8_lossy(b).into_owned())
        }
    }

    pub fn varint(b: &[u8], i: &mut usize) -> Option<u64> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let c = *b.get(*i)?;
            *i += 1;
            v |= u64::from(c & 0x7F) << shift;
            if c < 0x80 {
                return Some(v);
            }
        }
        None
    }

    pub fn put_varint(out: &mut Vec<u8>, mut v: u64) {
        while v >= 0x80 {
            out.push((v as u8) | 0x80);
            v >>= 7;
        }
        out.push(v as u8);
    }

    /// Every field of a message, in order; `None` if it isn't a well-formed message.
    pub fn fields(b: &[u8]) -> Option<Vec<Field<'_>>> {
        let mut out = vec![];
        let mut i = 0;
        while i < b.len() {
            let start = i;
            let key = varint(b, &mut i)?;
            let num = u32::try_from(key >> 3).ok().filter(|&n| n > 0)?;
            let val = match key & 7 {
                0 => Val::Int(varint(b, &mut i)?),
                1 => {
                    b.get(i..i + 8)?;
                    i += 8;
                    Val::F64
                }
                2 => {
                    let n = usize::try_from(varint(b, &mut i)?).ok()?;
                    let v = b.get(i..i.checked_add(n)?)?;
                    i += n;
                    Val::Bytes(v)
                }
                5 => {
                    let v = b.get(i..i + 4)?.try_into().ok()?;
                    i += 4;
                    Val::F32(v)
                }
                _ => return None,
            };
            out.push(Field { num, val, start, end: i });
        }
        Some(out)
    }

    pub fn first<'a>(fs: &[Field<'a>], num: u32) -> Option<Field<'a>> {
        fs.iter().find(|f| f.num == num).copied()
    }

    pub fn all<'a>(fs: &'a [Field<'a>], num: u32) -> impl Iterator<Item = Field<'a>> + 'a {
        fs.iter().filter(move |f| f.num == num).copied()
    }

    /// A packed list of varints.
    pub fn packed(b: &[u8]) -> Option<Vec<u64>> {
        let mut i = 0;
        let mut out = vec![];
        while i < b.len() {
            out.push(varint(b, &mut i)?);
        }
        Some(out)
    }

    pub fn int_field(num: u32, v: u64) -> Vec<u8> {
        let mut o = vec![];
        put_varint(&mut o, u64::from(num) << 3);
        put_varint(&mut o, v);
        o
    }

    pub fn f32_field(num: u32, v: f32) -> Vec<u8> {
        let mut o = vec![];
        put_varint(&mut o, (u64::from(num) << 3) | 5);
        o.extend(v.to_le_bytes());
        o
    }

    pub fn bytes_field(num: u32, v: &[u8]) -> Vec<u8> {
        let mut o = vec![];
        put_varint(&mut o, (u64::from(num) << 3) | 2);
        put_varint(&mut o, v.len() as u64);
        o.extend_from_slice(v);
        o
    }

    pub fn packed_field(num: u32, vals: &[u64]) -> Vec<u8> {
        let mut body = vec![];
        for &v in vals {
            put_varint(&mut body, v);
        }
        bytes_field(num, &body)
    }

    /// `msg` with field `num` replaced by `encoded` (a whole field), or `encoded` added
    /// at the end when the message doesn't have it.
    pub fn replace(msg: &[u8], num: u32, encoded: &[u8]) -> Option<Vec<u8>> {
        let fs = fields(msg)?;
        let mut out = Vec::with_capacity(msg.len() + encoded.len());
        match first(&fs, num) {
            Some(f) => {
                out.extend_from_slice(&msg[..f.start]);
                out.extend_from_slice(encoded);
                out.extend_from_slice(&msg[f.end..]);
            }
            None => {
                out.extend_from_slice(msg);
                out.extend_from_slice(encoded);
            }
        }
        Some(out)
    }
}

// ------------------------------------------------------------------ outer container

/// One LZO1X block holds at most this much (bigger characters are split; not handled).
const BLOCK: usize = 0x40000;

fn sha1(d: &[u8]) -> [u8; 20] {
    use sha1::{Digest, Sha1};
    Sha1::digest(d).into()
}

/// The WSG blob inside a `SaveNNNN.sav`, if the SHA-1 matches and the LZO block unpacks
/// to exactly the size the file states.
pub fn unpack(d: &[u8]) -> Option<Vec<u8>> {
    if d.len() < 32 {
        return None;
    }
    let size = u32::from_be_bytes(d[20..24].try_into().ok()?) as usize;
    if size < 20 || size > BLOCK || sha1(&d[20..])[..] != d[..20] {
        return None;
    }
    let body = &d[24..];
    let inner = std::panic::catch_unwind(|| lzokay_native::decompress_all(body, Some(size))).ok()?.ok()?;
    (inner.len() == size && looks(&inner)).then_some(inner)
}

/// The file for a WSG blob: LZO1X, size, SHA-1.
pub fn pack(inner: &[u8]) -> Result<Vec<u8>, String> {
    if inner.len() > BLOCK {
        return Err("this character's data is too big for one LZO block (not supported)".into());
    }
    let packed = lzokay_native::compress(inner).map_err(|e| format!("LZO: {e:?}"))?;
    let mut out = vec![0u8; 20];
    out.extend((inner.len() as u32).to_be_bytes());
    out.extend(packed);
    let h = sha1(&out[20..]);
    out[..20].copy_from_slice(&h);
    Ok(out)
}

// ------------------------------------------------------------------ WSG blob

pub fn looks(d: &[u8]) -> bool {
    d.len() > 19 && &d[4..7] == b"WSG" && u32::from_be_bytes(d[0..4].try_into().unwrap()) as usize + 4 == d.len()
}

fn crc32(d: &[u8]) -> u32 {
    let mut h = flate2::Crc::new();
    h.update(d);
    h.sum()
}

/// Version, then CRC and size in the version's byte order (little-endian on PC).
struct Header {
    big_endian: bool,
    crc: u32,
    len: usize,
}

fn header(d: &[u8]) -> Option<Header> {
    let at = |o: usize, be: bool| {
        let b: [u8; 4] = d.get(o..o + 4)?.try_into().ok()?;
        Some(if be { u32::from_be_bytes(b) } else { u32::from_le_bytes(b) })
    };
    let big_endian = match (at(7, false)?, at(7, true)?) {
        (2, _) => false,
        (_, 2) => true,
        _ => return None,
    };
    Some(Header { big_endian, crc: at(11, big_endian)?, len: at(15, big_endian)? as usize })
}

/// Huffman decoding: tree (pre-order) then codes, `n` bytes out.
fn huff_decode(d: &[u8], n: usize) -> Option<Vec<u8>> {
    let mut bit = 0usize;
    let mut read = |k: u32| -> Option<u32> {
        let mut v = 0;
        for _ in 0..k {
            let byte = *d.get(bit / 8)?;
            v = (v << 1) | u32::from((byte >> (7 - bit % 8)) & 1);
            bit += 1;
        }
        Some(v)
    };
    // (left, right) for branches; leaves have left == usize::MAX and the byte in right.
    let mut nodes: Vec<(usize, usize)> = vec![];
    fn node(nodes: &mut Vec<(usize, usize)>, read: &mut dyn FnMut(u32) -> Option<u32>, depth: usize) -> Option<usize> {
        if nodes.len() > 1024 || depth > 300 {
            return None;
        }
        let i = nodes.len();
        nodes.push((0, 0));
        if read(1)? == 1 {
            nodes[i] = (usize::MAX, read(8)? as usize);
        } else {
            let l = node(nodes, read, depth + 1)?;
            let r = node(nodes, read, depth + 1)?;
            nodes[i] = (l, r);
        }
        Some(i)
    }
    node(&mut nodes, &mut read, 0)?;
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let mut i = 0;
        while nodes[i].0 != usize::MAX {
            i = if read(1)? == 1 { nodes[i].1 } else { nodes[i].0 };
        }
        out.push(nodes[i].1 as u8);
    }
    Some(out)
}

/// Huffman coding with a tree built from the data's byte counts.
fn huff_encode(d: &[u8]) -> Vec<u8> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let mut counts = [0u64; 256];
    for &b in d {
        counts[b as usize] += 1;
    }
    // Nodes: leaves 0..256 (byte), branches after; (left, right).
    let mut kids: Vec<(usize, usize)> = (0..256).map(|_| (usize::MAX, usize::MAX)).collect();
    let mut heap: BinaryHeap<Reverse<(u64, usize)>> = (0..256).filter(|&b| counts[b] > 0).map(|b| Reverse((counts[b], b))).collect();
    if heap.is_empty() {
        heap.push(Reverse((0, 0)));
    }
    while heap.len() > 1 {
        let Reverse((fa, a)) = heap.pop().unwrap();
        let Reverse((fb, b)) = heap.pop().unwrap();
        kids.push((a, b));
        heap.push(Reverse((fa + fb, kids.len() - 1)));
    }
    let root = heap.pop().unwrap().0 .1;
    let mut bits: Vec<bool> = Vec::with_capacity(d.len() * 6);
    let mut codes: Vec<Vec<bool>> = vec![vec![]; 256];
    // Pre-order walk: write the tree and note each byte's code.
    let mut stack = vec![(root, Vec::<bool>::new())];
    while let Some((n, code)) = stack.pop() {
        if n < 256 {
            bits.push(true);
            bits.extend((0..8).rev().map(|k| (n >> k) & 1 == 1));
            codes[n] = code;
        } else {
            bits.push(false);
            let (l, r) = kids[n];
            let mut rc = code.clone();
            rc.push(true);
            let mut lc = code;
            lc.push(false);
            stack.push((r, rc));
            stack.push((l, lc));
        }
    }
    for &b in d {
        bits.extend_from_slice(&codes[b as usize]);
    }
    let mut out = vec![0u8; bits.len().div_ceil(8)];
    for (i, &b) in bits.iter().enumerate() {
        if b {
            out[i / 8] |= 0x80 >> (i % 8);
        }
    }
    out
}

/// The character data (protobuf) in a WSG blob, if its CRC matches.
pub fn decode(d: &[u8]) -> Result<Vec<u8>, String> {
    if !looks(d) {
        return Err("not a Borderlands character".into());
    }
    let h = header(d).ok_or("unknown save version")?;
    if h.len > 64 << 20 {
        return Err("the character data's size is wrong".into());
    }
    let proto = huff_decode(&d[19..], h.len).ok_or("the character data couldn't be unpacked")?;
    if crc32(&proto) != h.crc {
        return Err("the character data's checksum doesn't match".into());
    }
    Ok(proto)
}

/// A WSG blob for this character data, in the byte order of `like`.
fn encode(like: &[u8], proto: &[u8]) -> Result<Vec<u8>, String> {
    let h = header(like).ok_or("unknown save version")?;
    let w = |v: u32| if h.big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
    let body = huff_encode(proto);
    let mut out = vec![0u8; 4];
    out.extend_from_slice(b"WSG");
    out.extend(w(2));
    out.extend(w(crc32(proto)));
    out.extend(w(proto.len() as u32));
    out.extend(body);
    let n = (out.len() - 4) as u32;
    out[..4].copy_from_slice(&n.to_be_bytes());
    Ok(out)
}

// ------------------------------------------------------------------ names and facts

/// The six playable classes of each game: class object name → (class, vault hunter).
const CLASSES: [(&str, &str, &str, bool); 12] = [
    ("CharClass_Soldier", "Commando", "Axton", false),
    ("CharClass_Siren", "Siren", "Maya", false),
    ("CharClass_Mercenary", "Gunzerker", "Salvador", false),
    ("CharClass_Assassin", "Assassin", "Zer0", false),
    ("CharClass_Mechromancer", "Mechromancer", "Gaige", false),
    ("CharClass_LilacPlayerClass", "Psycho", "Krieg", false),
    ("CharClass_Gladiator", "Gladiator", "Athena", true),
    ("CharClass_Enforcer", "Enforcer", "Wilhelm", true),
    ("CharClass_Lawbringer", "Lawbringer", "Nisha", true),
    ("CharClass_Prototype", "Fragtrap", "Claptrap", true),
    ("charclass_doppelganger", "Doppelganger", "Jack", true),
    ("CharClass_Baroness", "Baroness", "Aurelia", true),
];

/// Black market (SDU) slots in save order; The Pre-Sequel adds lasers at the end.
const BLACK_MARKET: [&str; 10] = ["Assault rifle", "Pistol", "Rocket launcher", "Shotgun", "SMG", "Sniper rifle", "Grenades", "Backpack", "Bank", "Laser"];

/// Ammo resource object → the name shared with its SDU.
const AMMO: [(&str, &str); 8] = [
    ("Ammo_Combat_Rifle", "Assault rifle"),
    ("Ammo_Repeater_Pistol", "Pistol"),
    ("Ammo_Rocket_Launcher", "Rocket launcher"),
    ("Ammo_Combat_Shotgun", "Shotgun"),
    ("Ammo_Patrol_SMG", "SMG"),
    ("Ammo_Sniper_Rifle", "Sniper rifle"),
    ("Ammo_Grenade_Protean", "Grenades"),
    ("Ammo_Combat_Laser", "Laser"),
];

/// How much ammo a pool holds at each black market level (0–7), before relic or class bonuses.
const CAPACITY: [(&str, [u32; 8]); 8] = [
    ("Assault rifle", [280, 420, 560, 700, 840, 980, 1120, 1260]),
    ("Pistol", [200, 300, 400, 500, 600, 700, 800, 900]),
    ("Rocket launcher", [12, 15, 18, 21, 24, 27, 30, 33]),
    ("Shotgun", [80, 100, 120, 140, 160, 180, 200, 220]),
    ("SMG", [360, 540, 720, 900, 1080, 1260, 1440, 1620]),
    ("Sniper rifle", [48, 60, 72, 84, 96, 108, 120, 132]),
    ("Grenades", [3, 4, 5, 6, 7, 8, 9, 10]),
    ("Laser", [500, 620, 740, 860, 980, 1100, 1220, 1340]),
];

/// Ammo an SDU level holds (without bonuses), for the pools whose upgrade is ammo.
pub fn ammo_capacity(name: &str, level: i64) -> Option<u32> {
    let t = CAPACITY.iter().find(|c| c.0 == name)?;
    Some(t.1[level.clamp(0, 7) as usize])
}

/// Highest black-market level of an ammo SDU.
pub const MAX_AMMO_SDU: i64 = 7;

/// An ammo amount (a 32-bit float) shown as a whole number when it is one.
pub fn amount_value(f: f32) -> Value {
    if f.fract() == 0.0 && f.abs() < 1e9 {
        Value::from(f as i64)
    } else {
        crate::tree::f32_value(f)
    }
}

fn ammo_name(resource: &str) -> String {
    let last = resource.rsplit('.').next().unwrap_or(resource);
    AMMO.iter().find(|a| a.0 == last).map_or_else(|| last.trim_start_matches("Ammo_").replace('_', " "), |a| a.1.to_string())
}

/// What an inventory serial says once de-obfuscated: is its check right, is it a weapon,
/// and its level (0: the item has none).
pub struct Serial {
    pub ok: bool,
    pub weapon: bool,
    pub level: u32,
}

pub fn serial(s: &[u8]) -> Option<Serial> {
    if s.len() < 8 {
        return None;
    }
    let seed = i32::from_be_bytes(s[1..5].try_into().ok()?);
    let mut t = s[5..].to_vec();
    let mut x = ((seed >> 5) as u32) as u64;
    for b in t.iter_mut() {
        x = (x * 0x10A8_60C1) % 0xFFFF_FFFB;
        *b ^= x as u8;
    }
    let r = (seed as u32 & 31) as usize % t.len();
    t.rotate_right(r);
    let check = u16::from_be_bytes([t[0], t[1]]);
    let mut b = s[..5].to_vec();
    b.extend([0xFF, 0xFF]);
    b.extend_from_slice(&t[2..]);
    if b.len() < 40 {
        b.resize(40, 0xFF);
    }
    let c = crc32(&b);
    let weapon = s[0] & 0x80 != 0;
    // Bit fields after the check, least significant bit first: set (8), type (17, or 13
    // for weapons), balance (20), manufacturer (11), then the level (7).
    let at = if weapon { 8 + 13 + 20 + 11 } else { 8 + 17 + 20 + 11 };
    let bits = &t[2..];
    let level = (0..7).map(|k| at + k).fold(0u32, |v, i| v | (u32::from(bits.get(i / 8).map_or(0, |b| (b >> (i % 8)) & 1)) << (i - at)));
    Some(Serial { ok: ((c >> 16) ^ (c & 0xFFFF)) as u16 == check, weapon, level })
}

// ------------------------------------------------------------------ editable document

#[derive(Clone, Debug)]
enum Target {
    /// A top-level number (level, experience, skill points).
    Int(u32),
    /// One slot of the currency list (field 6).
    Currency(usize),
    /// One black market level (field 36).
    BlackMarket(usize),
    /// An ammo amount: where its 4-byte float is in the character data.
    Ammo(usize),
    /// How many weapons can be readied (field 2 of the inventory slot data, field 13).
    GunSlots,
}

/// Backpack and bank black market slots, and the room each level gives: the backpack
/// starts at 12 and grows by 3, the bank starts at 6 and grows by 2 (figures from
/// apocalyptech's Borderlands 2 save editor notes).
const BACKPACK_SDU: usize = 7;
const BANK_SDU: usize = 8;
pub const MAX_STORAGE_SDU: i64 = 9;

pub fn backpack_slots(level: i64) -> i64 {
    12 + 3 * level
}

pub fn bank_slots(level: i64) -> i64 {
    6 + 2 * level
}

#[derive(Clone, Debug)]
struct Bl2Backend {
    blob: Arc<Vec<u8>>,
    proto: Arc<Vec<u8>>,
    targets: Vec<Target>,
    max_level: i64,
}

fn whole(v: &Value) -> Result<i64, String> {
    v.as_i64().or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64)).ok_or_else(|| "expected a whole number".to_string())
}

impl Bl2Backend {
    fn range(&self, t: &Target) -> (i64, i64) {
        match t {
            Target::Int(2) => (1, self.max_level),
            Target::Int(7) => (0, 2),
            Target::Int(_) | Target::Currency(_) => (0, i64::from(i32::MAX)),
            Target::BlackMarket(BACKPACK_SDU | BANK_SDU) => (0, MAX_STORAGE_SDU),
            Target::BlackMarket(_) => (0, MAX_AMMO_SDU),
            Target::Ammo(_) => (0, 99_999),
            Target::GunSlots => (2, 4),
        }
    }
}

impl Backend for Bl2Backend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.blob.to_vec());
        }
        let proto = self.proto.as_slice();
        let top = pb::fields(proto).ok_or("the character data couldn't be read")?;
        let mut patches = Patches::default();
        // A packed list: its span in the message and its values.
        let list = |num: u32| -> Result<((usize, usize), Vec<u64>), String> {
            let f = pb::first(&top, num).ok_or("missing list")?;
            Ok(((f.start, f.end), f.bytes().and_then(pb::packed).ok_or("the list couldn't be read")?))
        };
        let mut currency = None;
        let mut market = None;
        // New values for fields 1 (backpack room) and 2 (weapon slots) of the slot data.
        let mut slots: [Option<u64>; 2] = [None, None];
        let mut bank = None;
        for (leaf, v) in changes {
            let t = &self.targets[*leaf];
            let n = whole(v)?;
            let (lo, hi) = self.range(t);
            if n < lo || n > hi {
                return Err(format!("must be between {lo} and {hi}"));
            }
            match t {
                Target::Int(num) => {
                    let f = pb::first(&top, *num).ok_or("missing value")?;
                    patches.add(f.start, f.end, pb::int_field(*num, n as u64));
                }
                Target::Currency(i) | Target::BlackMarket(i) => {
                    let slot = if matches!(t, Target::Currency(_)) { &mut currency } else { &mut market };
                    if slot.is_none() {
                        *slot = Some(list(if matches!(t, Target::Currency(_)) { 6 } else { 36 })?);
                    }
                    let (_, vals) = slot.as_mut().unwrap();
                    if vals.len() <= *i {
                        vals.resize(*i + 1, 0);
                    }
                    vals[*i] = n as u64;
                    // The room an upgrade gives is saved beside its level; keep the two in step.
                    if matches!(t, Target::BlackMarket(BACKPACK_SDU)) {
                        slots[0] = Some(backpack_slots(n) as u64);
                    } else if matches!(t, Target::BlackMarket(BANK_SDU)) {
                        bank = Some(bank_slots(n) as u64);
                    }
                }
                Target::Ammo(at) => patches.add(*at, *at + 4, (n as f32).to_le_bytes().to_vec()),
                Target::GunSlots => slots[1] = Some(n as u64),
            }
        }
        if slots.iter().any(Option::is_some) {
            let f = pb::first(&top, 13).ok_or("missing inventory slot data")?;
            let inner = pb::fields(f.bytes().ok_or("missing inventory slot data")?).ok_or("the inventory slot data couldn't be read")?;
            let body = f.bytes().unwrap();
            let mut out = vec![];
            let mut done = [false; 2];
            for g in &inner {
                match (g.num, slots.get(g.num as usize - 1).copied().flatten()) {
                    (1 | 2, Some(v)) => {
                        out.extend(pb::int_field(g.num, v));
                        done[g.num as usize - 1] = true;
                    }
                    _ => out.extend_from_slice(&body[g.start..g.end]),
                }
            }
            for (i, v) in slots.iter().enumerate() {
                if let (Some(v), false) = (v, done[i]) {
                    out.extend(pb::int_field(i as u32 + 1, *v));
                }
            }
            let mut field = vec![];
            pb::put_varint(&mut field, (13 << 3) | 2);
            pb::put_varint(&mut field, out.len() as u64);
            field.extend(out);
            patches.add(f.start, f.end, field);
        }
        if let Some(v) = bank {
            match pb::first(&top, 56) {
                Some(f) => patches.add(f.start, f.end, pb::int_field(56, v)),
                None => patches.add(proto.len(), proto.len(), pb::int_field(56, v)),
            }
        }
        for (num, slot) in [(6, currency), (36, market)] {
            if let Some(((start, end), vals)) = slot {
                patches.add(start, end, pb::packed_field(num, &vals));
            }
        }
        let proto = patches.apply(proto)?;
        encode(&self.blob, &proto)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let n = whole(v)?;
        let (lo, hi) = self.range(&self.targets[leaf]);
        if n < lo || n > hi {
            return Err(format!("must be between {lo} and {hi}"));
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub const KIND_BL2: &str = "Borderlands 2 character";
pub const KIND_TPS: &str = "Borderlands: The Pre-Sequel character";

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let proto = decode(d)?;
    let top = pb::fields(&proto).ok_or("the character data couldn't be read")?;
    let int = |n: u32| pb::first(&top, n).and_then(|f| f.int());
    let class_path = pb::first(&top, 1).and_then(|f| f.text()).ok_or("no character class")?;
    let class_obj = class_path.rsplit('.').next().unwrap_or("").to_string();
    let class = CLASSES.iter().find(|c| c.0.eq_ignore_ascii_case(&class_obj));
    let resources: Vec<(String, Option<f32>, usize)> = pb::all(&top, 11)
        .filter_map(|f| {
            let b = f.bytes()?;
            let inner = pb::fields(b)?;
            let name = ammo_name(&pb::first(&inner, 1)?.text()?);
            let amount = pb::first(&inner, 3);
            // Where the float is in the whole message: field start + key/length bytes.
            let body_at = f.end - b.len();
            Some((name, amount.and_then(|a| a.f32()), amount.map_or(0, |a| body_at + a.end - 4)))
        })
        .collect();
    let tps = class.map_or_else(|| resources.iter().any(|r| r.0 == "Laser"), |c| c.3);
    let mut view = Map::new();
    let mut editable = HashMap::new();
    let mut targets = vec![];
    let mut add = |view: &mut Map<String, Value>, group: &str, key: &str, v: Value, t: Option<Target>| {
        if let Some(t) = t {
            editable.insert(format!("/{}/{}", crate::tree::esc(group), crate::tree::esc(key)), targets.len());
            targets.push(t);
        }
        view.entry(group.to_string()).or_insert_with(|| Value::Object(Map::new())).as_object_mut().unwrap().insert(key.to_string(), v);
    };

    let name = pb::first(&top, 19).and_then(|f| f.bytes()).and_then(|b| pb::fields(b)?.iter().find(|f| f.num == 1)?.text()).unwrap_or_default();
    add(&mut view, "Character", "Name", name.into(), None);
    add(&mut view, "Character", "Class", class.map_or_else(|| class_obj.clone(), |c| c.1.to_string()).into(), None);
    add(&mut view, "Character", "Vault hunter", class.map_or("", |c| c.2).into(), None);
    add(&mut view, "Character", "Game", if tps { "Borderlands: The Pre-Sequel" } else { "Borderlands 2" }.into(), None);
    for (num, label) in [(2, "Level"), (3, "Experience"), (4, "Skill points")] {
        if let Some(n) = int(num) {
            add(&mut view, "Character", label, n.into(), Some(Target::Int(num)));
        }
    }
    if let Some(n) = int(7) {
        add(&mut view, "Character", "Playthroughs completed", n.into(), Some(Target::Int(7)));
    }

    let currency = pb::first(&top, 6).and_then(|f| f.bytes()).and_then(pb::packed);
    if let Some(cur) = &currency {
        let slots: &[(usize, &str)] = if tps { &[(0, "Money"), (1, "Moonstone")] } else { &[(0, "Money"), (1, "Eridium"), (2, "Seraph crystals"), (4, "Torgue tokens")] };
        for &(i, label) in slots {
            add(&mut view, "Currency", label, cur.get(i).copied().unwrap_or(0).into(), Some(Target::Currency(i)));
        }
    }
    let mut seen = Map::new();
    for (name, amount, at) in &resources {
        let key = crate::tree::insert_unique(&mut seen, name, Value::Null);
        match amount {
            Some(a) => add(&mut view, "Ammo", &key, amount_value(*a), Some(Target::Ammo(*at))),
            None => add(&mut view, "Ammo", &key, 0.into(), None),
        }
    }
    if let Some(levels) = pb::first(&top, 36).and_then(|f| f.bytes()).and_then(pb::packed) {
        for (i, &lv) in levels.iter().enumerate() {
            let label = BLACK_MARKET.get(i).map_or_else(|| format!("Upgrade {}", i + 1), |s| s.to_string());
            // Backpack and bank room is kept elsewhere too; the writer keeps it in step.
            let known = CAPACITY.iter().any(|c| c.0 == label) || i == BACKPACK_SDU || i == BANK_SDU;
            add(&mut view, "SDU upgrades", &label, lv.into(), known.then_some(Target::BlackMarket(i)));
        }
    }
    if let Some(inner) = pb::first(&top, 13).and_then(|f| f.bytes()).and_then(pb::fields) {
        if let Some(n) = pb::first(&inner, 1).and_then(|f| f.int()) {
            add(&mut view, "Character", "Backpack room", n.into(), None);
        }
        if let Some(n) = pb::first(&inner, 2).and_then(|f| f.int()) {
            add(&mut view, "Character", "Weapon slots", n.into(), Some(Target::GunSlots));
        }
    }
    if let Some(n) = int(56) {
        add(&mut view, "Character", "Bank room", n.into(), None);
    }

    let item_view = |f: pb::Field, weapon_list: bool| -> Option<Value> {
        let inner = pb::fields(f.bytes()?)?;
        let s = serial(pb::first(&inner, 1)?.bytes()?)?;
        let mut m = Map::new();
        m.insert("Level".into(), s.level.into());
        if weapon_list {
            m.insert("Quick slot".into(), pb::first(&inner, 2).and_then(|f| f.int()).unwrap_or(0).into());
        } else {
            m.insert("Quantity".into(), pb::first(&inner, 2).and_then(|f| f.int()).unwrap_or(0).into());
            m.insert("Equipped".into(), (pb::first(&inner, 3).and_then(|f| f.int()).unwrap_or(0) != 0).into());
        }
        m.insert("Weapon".into(), s.weapon.into());
        m.insert("Serial check".into(), if s.ok { "OK" } else { "doesn't match" }.into());
        Some(Value::Object(m))
    };
    let weapons: Vec<Value> = pb::all(&top, 54).filter_map(|f| item_view(f, true)).collect();
    let items: Vec<Value> = pb::all(&top, 53).filter_map(|f| item_view(f, false)).collect();
    view.insert("Weapons".into(), Value::Array(weapons));
    view.insert("Items".into(), Value::Array(items));
    view.insert("Bank".into(), Value::from(pb::all(&top, 41).count()));

    let backend = Bl2Backend { blob: Arc::new(d.to_vec()), proto: Arc::new(proto), targets, max_level: if tps { 70 } else { 80 } };
    let kind = if tps { KIND_TPS } else { KIND_BL2 };
    let currency_name = if tps { "moonstone" } else { "Eridium" };
    Ok(TreeDoc::new(Value::Object(view), editable, kind, Box::new(backend)).with_note(format!(
        "Level, experience, skill points, money, {currency_name}, ammo and ammo SDU levels can be changed; the character data is coded again and its checksums \
         (CRC-32 and SHA-1) are written when you save. Weapons and items are shown, not edited. Turn off Steam Cloud for the game first, or it may put the old save back."
    )))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A synthetic character protobuf (BL2 Psycho, made-up values).
    pub fn sample_proto(tps: bool) -> Vec<u8> {
        let mut p = vec![];
        let class = if tps { "GD_Enforcer.Character.CharClass_Enforcer" } else { "GD_Lilac_PlayerClass.Character.CharClass_LilacPlayerClass" };
        p.extend(pb::bytes_field(1, class.as_bytes()));
        p.extend(pb::int_field(2, 30));
        p.extend(pb::int_field(3, 400_000));
        p.extend(pb::int_field(4, 3));
        p.extend(pb::packed_field(6, &[1500, 12, 0, 0, 5]));
        p.extend(pb::int_field(7, 0));
        for (res, amt) in [("Ammo_Combat_Rifle", 560.0f32), ("Ammo_Grenade_Protean", 3.0)] {
            let mut r = pb::bytes_field(1, format!("D_Resources.AmmoResources.{res}").as_bytes());
            r.extend(pb::bytes_field(2, format!("D_Resourcepools.AmmoPools.{res}_Pool").as_bytes()));
            r.extend(pb::f32_field(3, amt));
            r.extend(pb::int_field(4, 1));
            p.extend(pb::bytes_field(11, &r));
        }
        let mut slots = pb::int_field(1, 15);
        slots.extend(pb::int_field(2, 2));
        slots.extend(pb::int_field(3, 0));
        p.extend(pb::bytes_field(13, &slots));
        p.extend(pb::bytes_field(19, &pb::bytes_field(1, b"Tester")));
        p.extend(pb::int_field(56, 6));
        p.extend(pb::packed_field(36, &[2, 1, 0, 0, 0, 0, 0, 1, 0]));
        // One weapon: header byte (weapon, version 7), seed 0, check, then bit fields.
        let mut body = vec![0u8; 30];
        let at = 8 + 13 + 20 + 11;
        for k in 0..7 {
            if (30 >> k) & 1 == 1 {
                body[(at + k) / 8] |= 1 << ((at + k) % 8);
            }
        }
        let mut s = vec![0x87, 0, 0, 0, 0, 0xFF, 0xFF];
        s.extend(&body);
        let mut padded = s.clone();
        padded.resize(40.max(s.len()), 0xFF);
        let c = crc32(&padded);
        let check = ((c >> 16) ^ (c & 0xFFFF)) as u16;
        s[5..7].copy_from_slice(&check.to_be_bytes());
        let mut w = pb::bytes_field(1, &s);
        w.extend(pb::int_field(2, 1));
        p.extend(pb::bytes_field(54, &w));
        p
    }

    pub fn sample_blob(tps: bool) -> Vec<u8> {
        let mut like = vec![0u8; 19];
        like[7..11].copy_from_slice(&2u32.to_le_bytes());
        encode(&like, &sample_proto(tps)).unwrap()
    }

    #[test]
    fn huffman_round_trip() {
        for data in [&b""[..], b"a", b"aaaa", b"hello huffman world", &(0..=255u8).cycle().take(5000).collect::<Vec<_>>()] {
            let e = huff_encode(data);
            assert_eq!(huff_decode(&e, data.len()).unwrap(), data);
        }
    }

    #[test]
    fn container_round_trip() {
        let blob = sample_blob(false);
        assert!(looks(&blob));
        let file = pack(&blob).unwrap();
        assert_eq!(unpack(&file).unwrap(), blob);
        let mut bad = file.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(unpack(&bad).is_none(), "SHA-1 is checked");
        assert!(unpack(&vec![0u8; 45_000]).is_none(), "an all-zero file isn't a save");
    }

    #[test]
    fn reads_and_edits() {
        let blob = sample_blob(false);
        let mut doc = parse(&blob).unwrap();
        assert_eq!(doc.kind_name, KIND_BL2);
        let v = &doc.view;
        assert_eq!(v.pointer("/Character/Class"), Some(&"Psycho".into()));
        assert_eq!(v.pointer("/Character/Name"), Some(&"Tester".into()));
        assert_eq!(v.pointer("/Currency/Money"), Some(&1500.into()));
        assert_eq!(v.pointer("/Currency/Torgue tokens"), Some(&5.into()));
        assert_eq!(v.pointer("/Ammo/Assault rifle"), Some(&560.into()));
        assert_eq!(v.pointer("/SDU upgrades/Assault rifle"), Some(&2.into()));
        assert_eq!(v.pointer("/Weapons/0/Level"), Some(&30.into()));
        assert_eq!(v.pointer("/Weapons/0/Serial check"), Some(&"OK".into()));
        assert_eq!(v.pointer("/Character/Backpack room"), Some(&15.into()));
        assert_eq!(v.pointer("/Character/Weapon slots"), Some(&2.into()));
        assert_eq!(doc.to_bytes().unwrap(), blob);
        // Money grows a byte; ammo, SDU and level change.
        doc.set("/Currency/Money", 300_000.into()).unwrap();
        doc.set("/Ammo/Grenades", 9.into()).unwrap();
        doc.set("/SDU upgrades/Pistol", 7.into()).unwrap();
        doc.set("/Character/Level", 31.into()).unwrap();
        assert!(doc.set("/SDU upgrades/Shotgun", 9.into()).is_err());
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
        assert_eq!(again.view.pointer("/Currency/Money"), Some(&300_000.into()));
        assert_eq!(again.view.pointer("/Currency/Eridium"), Some(&12.into()));
        // Backpack and bank upgrades carry their room with them; weapon slots and playthroughs change alone.
        let mut doc = again;
        doc.set("/SDU upgrades/Backpack", 9.into()).unwrap();
        doc.set("/SDU upgrades/Bank", 3.into()).unwrap();
        doc.set("/Character/Weapon slots", 4.into()).unwrap();
        doc.set("/Character/Playthroughs completed", 2.into()).unwrap();
        assert!(doc.set("/Character/Weapon slots", 5.into()).is_err());
        assert!(doc.set("/SDU upgrades/Backpack", 10.into()).is_err());
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/SDU upgrades/Backpack"), Some(&9.into()));
        assert_eq!(again.view.pointer("/Character/Backpack room"), Some(&39.into()));
        assert_eq!(again.view.pointer("/Character/Bank room"), Some(&12.into()));
        assert_eq!(again.view.pointer("/Character/Weapon slots"), Some(&4.into()));
        assert_eq!(again.view.pointer("/Character/Playthroughs completed"), Some(&2.into()));
        assert_eq!(again.view.pointer("/Currency/Money"), Some(&300_000.into()));
        let doc = again;
        // Through the whole file as the game reads it.
        let file = pack(&out).unwrap();
        assert_eq!(parse(&unpack(&file).unwrap()).unwrap().view, doc.view);
    }

    #[test]
    fn tps_names() {
        let doc = parse(&sample_blob(true)).unwrap();
        assert_eq!(doc.kind_name, KIND_TPS);
        assert_eq!(doc.view.pointer("/Currency/Moonstone"), Some(&12.into()));
        assert!(doc.view.pointer("/Currency/Eridium").is_none());
        assert!(parse(&[0u8; 100]).is_err());
    }

    /// Research aid: SAVE_EDITOR_BL2 = folders of Borderlands 2 / Pre-Sequel saves.
    /// Every file: opens (or fails cleanly), writes back byte for byte, and an edit
    /// (money + 1, an ammo amount) reads back with everything else unchanged.
    #[test]
    #[ignore]
    fn real_saves() {
        use crate::formats::{decode_with, encode, Content, Options};
        let Ok(dirs) = std::env::var("SAVE_EDITOR_BL2") else { return };
        let mut files = vec![];
        for dir in dirs.split('|') {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let n = e.file_name().to_string_lossy().to_lowercase();
                if n.ends_with(".sav") || n.ends_with(".sav.bak") {
                    files.push(e.path());
                }
            }
        }
        files.sort();
        for f in files {
            let raw = std::fs::read(&f).unwrap();
            let d = decode_with(&raw, &Options::for_path(&f));
            let Content::Tree(doc) = &d.content else {
                println!("{}: not opened ({} bytes, all zero: {}) hint: {:?}", f.display(), raw.len(), raw.iter().all(|&b| b == 0), d.hint);
                continue;
            };
            let v = &doc.view;
            println!(
                "{}: {} L{} {} xp {} money {:?} ammo {} weapons {} items {}",
                f.display(),
                v["Character"]["Class"],
                v["Character"]["Level"],
                v["Character"]["Game"],
                v["Character"]["Experience"],
                v["Currency"]["Money"],
                v["Ammo"].as_object().map_or(0, |m| m.len()),
                v["Weapons"].as_array().unwrap().len(),
                v["Items"].as_array().unwrap().len()
            );
            assert!(v["Weapons"].as_array().unwrap().iter().chain(v["Items"].as_array().unwrap()).all(|i| i["Serial check"] == "OK"));
            assert_eq!(encode(&d.layers, &d.content).unwrap(), raw, "unchanged save writes back byte for byte");
            let mut doc = doc.as_ref().clone();
            let money = doc.view.pointer("/Currency/Money").and_then(Value::as_i64).unwrap();
            doc.set("/Currency/Money", (money + 1).into()).unwrap();
            let ammo_key = doc.view["Ammo"].as_object().unwrap().keys().next().unwrap().clone();
            let ammo_ptr = format!("/Ammo/{}", crate::tree::esc(&ammo_key));
            doc.set(&ammo_ptr, 7.into()).unwrap();
            let out = encode(&d.layers, &Content::Tree(Box::new(doc.clone()))).unwrap();
            let back = decode_with(&out, &Options::for_path(&f));
            let Content::Tree(again) = &back.content else { panic!("edited save doesn't open") };
            assert_eq!(again.view, doc.view, "{}", f.display());
            assert_eq!(again.view.pointer("/Currency/Money").and_then(Value::as_i64), Some(money + 1));
            // Everything else in the character data is unchanged.
            let (p0, p1) = (decode(&unpack(&raw).unwrap()).unwrap(), decode(&unpack(&out).unwrap()).unwrap());
            let (t0, t1) = (pb::fields(&p0).unwrap(), pb::fields(&p1).unwrap());
            assert_eq!(t0.len(), t1.len());
            for (a, b) in t0.iter().zip(&t1) {
                assert_eq!(a.num, b.num);
                if a.num != 6 && a.num != 11 {
                    assert_eq!(&p0[a.start..a.end], &p1[b.start..b.end], "field {}", a.num);
                }
            }
            if let Ok(o) = std::env::var("SAVE_EDITOR_BL2_OUT") {
                std::fs::write(std::path::Path::new(&o).join(f.file_name().unwrap()), &out).unwrap();
            }
        }
    }
}
