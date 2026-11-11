//! Mass Effect: Andromeda (PC) `Career*` saves.
//!
//! Outer layout: "FBCHUNKS", u16 version 1, u32 meta length, u32 data length, then the meta
//! block and the data block. Each block starts with a u32 CRC-32 of the rest of the block
//! (the standard zlib CRC-32 continued from 0x12345678). The meta block is plain bytes: magic,
//! u16 version, u32 count, then (DJB name hash, u16 length, UTF-8 text) pairs — what the load
//! screen shows.
//!
//! The data block is an LSB-first bitstream. Sections carry their length or position in bits,
//! so this reader jumps straight to the party manager's squad data: one raw blob per party
//! member (itself a small "FB\0SAVE\n" bitstream of components, copied in at whatever bit it
//! lands on) and one snapshot per member (level, skill points…). The player's inventory is a
//! list of items, each starting with its partition name (e.g. `Game/Items/Stackables/Credit`),
//! then the quantity. Values are edited in place as bit fields, nothing moves; both CRCs are
//! computed again on save.
//!
//! Layout ported from Gibbed.MassEffectAndromeda by Rick (gibbed), zlib licence
//! (SaveFile, SaveData, PartySquad, PartyMember, InventoryComponent, ItemData), and checked on
//! sample saves: credits, crafting materials, levels and skill points.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Mass Effect: Andromeda save";

const MAGIC: &[u8; 8] = b"FBCHUNKS";
/// "FB\0SAVE\n" read as a little-endian u64: the start of the data block and of each member blob.
const SAVE_MAGIC: u64 = 0x0A45_5641_5300_4246;
const CRC_SEED: u32 = 0x1234_5678;

/// Party members by character id (names from Gibbed's PartyMembers.json).
const MEMBERS: &[(i64, &str)] = &[
    (0, "Ryder"),
    (1, "Cora"),
    (2, "Jaal"),
    (4, "Vetra"),
    (5, "Drack"),
    (6, "Peebee"),
    (7, "Liam"),
];

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 18 && d.starts_with(MAGIC) && u16::from_le_bytes([d[8], d[9]]) == 1
}

fn djb(s: &str) -> u32 {
    s.bytes()
        .fold(5381u32, |h, c| h.wrapping_mul(33) ^ u32::from(c))
}

/// zlib's CRC-32 continued from `seed`.
fn crc32(d: &[u8], seed: u32) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *t = c;
    }
    let mut c = !seed;
    for &b in d {
        c = table[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

/// The two blocks: (start, length) of the meta block and of the data block.
fn blocks(d: &[u8]) -> Option<((usize, usize), (usize, usize))> {
    if !looks(d) {
        return None;
    }
    let meta = u32::from_le_bytes(d[10..14].try_into().ok()?) as usize;
    let data = u32::from_le_bytes(d[14..18].try_into().ok()?) as usize;
    if meta < 4 || data < 4 || 18 + meta + data > d.len() {
        return None;
    }
    Some(((18, meta), (18 + meta, data)))
}

fn fix_crcs(d: &mut [u8]) {
    if let Some(((ms, ml), (ds, dl))) = blocks(d) {
        for (s, l) in [(ms, ml), (ds, dl)] {
            let crc = crc32(&d[s + 4..s + l], CRC_SEED);
            d[s..s + 4].copy_from_slice(&crc.to_le_bytes());
        }
    }
}

// ------------------------------------------------------------------ bit reading

/// An LSB-first bit reader over `d`; positions are bits from `base` (a byte offset).
struct Bits<'a> {
    d: &'a [u8],
    base: usize,
    len: usize,
    p: usize,
}

impl Bits<'_> {
    fn u(&mut self, n: usize) -> Result<u64, String> {
        if self.p + n > self.len {
            return Err("the save ends too soon".into());
        }
        let mut v = 0u64;
        for i in 0..n {
            let at = self.base * 8 + self.p + i;
            v |= u64::from((self.d[at / 8] >> (at % 8)) & 1) << i;
        }
        self.p += n;
        Ok(v)
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(self.u(32)? as u32 as i32)
    }
    fn skip(&mut self, n: usize) -> Result<(), String> {
        if self.p + n > self.len {
            return Err("the save ends too soon".into());
        }
        self.p += n;
        Ok(())
    }
    fn string(&mut self) -> Result<Vec<u8>, String> {
        let n = self.u(16)? as usize;
        (0..n).map(|_| self.u(8).map(|b| b as u8)).collect()
    }
    /// A section with an `n`-bit length in front: returns where it ends.
    fn frame(&mut self, n: usize) -> Result<usize, String> {
        let l = self.u(n)? as usize;
        let end = self.p + l;
        if end > self.len {
            return Err("a section runs past the end of the save".into());
        }
        Ok(end)
    }
    fn end(&mut self, end: usize) -> Result<(), String> {
        if self.p != end {
            return Err("a section isn't the length it says".into());
        }
        Ok(())
    }
    /// Absolute bit position in the file of the reader's position.
    fn abs(&self) -> usize {
        self.base * 8 + self.p
    }
    fn skip_save_magic(&mut self) -> Result<(), String> {
        if self.u(64)? != SAVE_MAGIC {
            return Err("not a Mass Effect: Andromeda save (data magic)".into());
        }
        if self.u(1)? == 1 {
            self.skip(27)?;
        }
        Ok(())
    }
}

/// Reads `width` bits at absolute bit `at` of `d`.
fn get_bits(d: &[u8], at: usize, width: usize) -> u64 {
    (0..width).fold(0u64, |v, i| {
        v | u64::from((d[(at + i) / 8] >> ((at + i) % 8)) & 1) << i
    })
}

fn put_bits(d: &mut [u8], at: usize, width: usize, v: u64) {
    for i in 0..width {
        let (byte, bit) = ((at + i) / 8, (at + i) % 8);
        d[byte] = (d[byte] & !(1 << bit)) | ((((v >> i) & 1) as u8) << bit);
    }
}

// ------------------------------------------------------------------ the walk

/// An item in the player's inventory: partition name and where its i32 quantity is (absolute bit).
struct Item {
    name: String,
    qty: usize,
}

struct Member {
    id: i64,
    /// Snapshot level and skill point count (absolute bits, i32 each).
    snap_level: usize,
    snap_points: usize,
    /// The progression component's level, when the member has one.
    prog_level: Option<usize>,
    items: Vec<Item>,
}

struct Walk {
    meta: Vec<(String, String)>,
    members: Vec<Member>,
}

fn meta_names() -> Vec<(u32, &'static str)> {
    [
        "AreaNameStringId",
        "AreaThumbnailTextureId",
        "GameVersion",
        "RequiredDLC",
        "RequiredInstallGroup",
        "ProfileName",
        "ProfileUniqueName",
        "ProfileId",
        "LevelID",
        "PlayerLevel",
        "GameCompleted",
        "TrialMode",
        "CompletionPercentage",
        "DateTime",
        "LevelTitleID",
        "LevelFloorID",
        "LevelRegionID",
        "TotalPlaytime",
        "NameOverrideStringId",
    ]
    .iter()
    .map(|n| (djb(n), *n))
    .collect()
}

fn read_meta(m: &[u8]) -> Vec<(String, String)> {
    let names = meta_names();
    let mut out = vec![];
    let (Some(count), true) = (m.get(10..14), m.len() >= 14) else {
        return out;
    };
    let count = u32::from_le_bytes(count.try_into().unwrap());
    let mut p = 14;
    for _ in 0..count {
        let Some(h) = m.get(p..p + 4) else { break };
        let h = u32::from_le_bytes(h.try_into().unwrap());
        let Some(l) = m.get(p + 4..p + 6) else { break };
        let l = u16::from_le_bytes(l.try_into().unwrap()) as usize;
        let Some(v) = m.get(p + 6..p + 6 + l) else {
            break;
        };
        p += 6 + l;
        if let Some((_, n)) = names.iter().find(|(x, _)| *x == h) {
            out.push((
                n.to_string(),
                String::from_utf8_lossy(v)
                    .trim_end_matches('\0')
                    .to_string(),
            ));
        }
    }
    out
}

fn walk(d: &[u8]) -> Result<Walk, String> {
    let ((ms, ml), (ds, dl)) = blocks(d).ok_or("not a Mass Effect: Andromeda save")?;
    let meta = read_meta(&d[ms + 4..ms + ml]);
    let mut r = Bits {
        d,
        base: ds + 4,
        len: (dl - 4) * 8,
        p: 0,
    };

    // SaveData header.
    r.skip_save_magic()?;
    r.skip(64)?; // timestamp
    r.string()?; // save file name
    let (game_version, save_version) = (r.u(16)?, r.u(16)?);
    if game_version != 3 || !(20..=22).contains(&save_version) || r.u(16)? != 10 || r.u(16)? != 0 {
        return Err("this version of the save isn't supported".into());
    }
    r.skip(32)?;
    r.string()?; // level name
    r.skip(32)?;
    for _ in 0..r.u(16)? {
        r.string()?; // preloaded bundles
    }
    for _ in 0..r.u(16)? {
        let e = r.frame(24)?; // layer inclusion
        r.p = e;
    }
    for _ in 0..2 {
        for _ in 0..r.u(12)? {
            let e = r.frame(24)?; // bundle heaps, then another list
            r.p = e;
        }
    }
    r.skip(128)?; // level hash
    if r.u(20)? != 0 {
        return Err("this save has a section that isn't supported".into());
    }
    let u3 = r.u(16)?;
    if u3 >= 5 {
        r.skip(16)?;
    }
    r.skip(1)?;
    let frame_position = r.u(26)? as usize;
    let _component_position = r.u(26)?;
    let entity_bulk = r.u(32)? as usize;
    r.skip(entity_bulk)?;
    let here = r.p;
    let e = r.frame(if save_version >= 18 { 24 } else { 6 })?;
    r.p = e;
    if r.u(26)? as usize != here {
        return Err("the save's agent table isn't where it should be".into());
    }
    let agent_bulk = r.u(32)? as usize;
    let (bulk_start, bulk_end) = (r.p, r.p + agent_bulk);
    r.skip(agent_bulk)?;
    let mut offsets: Vec<(u64, [usize; 5])> = vec![];
    for _ in 0..r.u(4)? {
        let idx = r.u(8)?;
        let mut o = [0usize; 5];
        for x in &mut o {
            *x = r.u(26)? as usize;
        }
        offsets.push((idx, o));
    }
    let e = r.frame(24)?; // entity table
    r.p = e;
    if r.p != frame_position {
        return Err("the save's agent list isn't where it should be".into());
    }
    let e = r.frame(24)?;
    let party_hash = u64::from(djb("MEPartyManagerSaveGameAgent"));
    let mut party = None;
    for _ in 0..r.u(4)? {
        let (hash, idx) = (r.u(32)?, r.u(8)?);
        r.skip(16)?; // version
        if hash == party_hash {
            party = Some(idx);
        }
    }
    r.end(e)?;
    let party = party.ok_or("the save has no party data")?;
    let at = offsets
        .iter()
        .find(|(i, _)| *i == party)
        .map(|(_, o)| o[3])
        .unwrap_or(0);
    if at == 0 {
        return Ok(Walk {
            meta,
            members: vec![],
        });
    }
    if at < bulk_start || at > bulk_end {
        return Err("the party data isn't where it should be".into());
    }

    // PartyManagerAgent, part 4: the squad.
    r.p = at;
    let agent_end = r.frame(24)?;
    let squad_end = r.frame(24)?;
    if squad_end > agent_end || squad_end == r.p {
        return Ok(Walk {
            meta,
            members: vec![],
        });
    }
    let version = r.u(32)?;
    if !(11..=12).contains(&version) {
        return Err(format!("squad data version {version} isn't supported"));
    }
    for _ in 0..r.u(16)? {
        let e = r.frame(24)?;
        r.p = e;
    }
    let mut raws = vec![];
    for _ in 0..r.u(16)? {
        let e = r.frame(24)?;
        let n = r.u(32)? as usize;
        raws.push((r.p, n * 8));
        r.p = e;
    }
    let mut snaps = vec![];
    for _ in 0..r.u(16)? {
        let e = r.frame(24)?;
        let start = r.abs();
        r.skip(4 * 32)?;
        let id = r.i32()?;
        snaps.push((start, i64::from(id)));
        r.p = e;
    }
    let mut members = vec![];
    for (&(raw, raw_len), &(snap, id)) in raws.iter().zip(&snaps) {
        if id < 0 || raw_len == 0 {
            continue;
        }
        let mut m = Member {
            id,
            snap_level: snap,
            snap_points: snap + 32,
            prog_level: None,
            items: vec![],
        };
        let mut q = Bits {
            d,
            base: r.base,
            len: raw + raw_len,
            p: raw,
        };
        member(&mut q, &mut m)?;
        members.push(m);
    }
    Ok(Walk { meta, members })
}

/// One member's blob: the components, each in a frame of its own.
fn member(q: &mut Bits, m: &mut Member) -> Result<(), String> {
    q.skip_save_magic()?;
    let base_version = q.u(32)?;
    if !(1..=11).contains(&base_version) {
        return Err("party member data version isn't supported".into());
    }
    let version = if base_version >= 11 {
        q.u(16)?
    } else if base_version >= 8 {
        9
    } else {
        4
    };
    let e = q.frame(24)?; // unknown component
    q.p = e;
    let e = q.frame(24)?; // inventory
    if e > q.p && m.id == 0 && version >= 6 {
        for _ in 0..q.u(16)? {
            let ie = q.frame(24)?;
            q.skip(32)?; // item id
            let len = q.u(24)? as usize;
            let start = q.p;
            q.skip(32)?;
            let name = q.string()?;
            if q.p + 32 <= start + len {
                m.items.push(Item {
                    name: String::from_utf8_lossy(&name).to_string(),
                    qty: q.abs(),
                });
            }
            q.p = ie;
        }
        q.end(e)?;
    }
    q.p = e;
    let e = q.frame(24)?; // equipment
    q.p = e;
    if base_version >= 2 {
        let e = q.frame(24)?; // progression: level first
        if e >= q.p + 32 {
            m.prog_level = Some(q.abs());
        }
        q.p = e;
    }
    Ok(())
}

// ------------------------------------------------------------------ view

/// "Game/Items/Research/Materials/Mineral/RM_ElementZero_01" → ("Minerals", "Element Zero").
fn resource(name: &str) -> Option<(&'static str, String)> {
    let low = name.to_ascii_lowercase();
    let group = if low == "game/items/stackables/credit" {
        return None;
    } else if low.starts_with("game/items/stackables/salvage/") {
        "Salvage"
    } else if low.starts_with("game/items/stackables/") && low.ends_with("rdipoints") {
        "Research data"
    } else if low.starts_with("game/items/research/materials/mineral/") {
        "Minerals"
    } else if low.starts_with("game/items/research/materials/faction/") {
        "Faction materials"
    } else if low.starts_with("game/items/research/materials/hunting/") {
        "Biological materials"
    } else if low.starts_with("game/items/research/materials/augmentation/") {
        "Augmentations"
    } else if low.starts_with("game/items/consumables/") {
        "Consumables"
    } else {
        return None;
    };
    let last = name.rsplit('/').next()?;
    let mut s = last.to_string();
    for p in ["rm_", "augmaterial_", "consumable_", "salvage_"] {
        if s.to_ascii_lowercase().starts_with(p) {
            s = s[p.len()..].to_string();
        }
    }
    if s.ends_with("_01") {
        s.truncate(s.len() - 3);
    }
    if group == "Research data" {
        s = match s.to_ascii_lowercase().trim_end_matches("rdipoints") {
            "milkyway" => "Milky Way".into(),
            "alien" => "Heleus".into(),
            "remnant" => "Remnant".into(),
            _ => s,
        };
    }
    // Split CamelCase and underscores into words.
    let mut out = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c == '_' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if c.is_ascii_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        out.push(c);
    }
    let mut cs = out.chars();
    let out = cs
        .next()
        .map(|f| f.to_ascii_uppercase().to_string() + cs.as_str())
        .unwrap_or_default();
    Some((group, out))
}

/// An i32 at absolute bit `at`, as a bit field of the (at most 8) bytes around it.
#[allow(clippy::too_many_arguments)]
fn int_field(b: &mut Builder, path: &[&str], at: usize, bias: i64, lo: i64, hi: i64) {
    let byte = (at / 8).min(b.d.len().saturating_sub(8));
    let shift = at - byte * 8;
    if shift + 32 > 64 || byte + 8 > b.d.len() {
        return;
    }
    let raw = get_bits(b.d, at, 32) as u32 as i32;
    if i64::from(raw) + bias < lo || i64::from(raw) + bias > hi {
        b.info(path, i64::from(raw) + bias);
        return;
    }
    b.bits(path, byte, Kind::U64, shift as u32, 32, bias, lo, hi);
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let ((ms, ml), (ds, dl)) = blocks(d).ok_or("not a Mass Effect: Andromeda save")?;
    if u32::from_le_bytes(d[ms..ms + 4].try_into().unwrap()) != crc32(&d[ms + 4..ms + ml], CRC_SEED)
        || u32::from_le_bytes(d[ds..ds + 4].try_into().unwrap())
            != crc32(&d[ds + 4..ds + dl], CRC_SEED)
    {
        return Err("the save's checksum is wrong (damaged file?)".into());
    }
    let w = walk(d)?;
    let mut b = Builder::new(d);
    let meta = |k: &str| w.meta.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    if let Some(v) = meta("ProfileName") {
        b.info(&["Save", "Profile name"], v);
    }
    if let Some(v) = meta("PlayerLevel").and_then(|v| v.parse::<i64>().ok()) {
        b.info(&["Save", "Level (load screen)"], v);
    }
    if let Some(v) = meta("LevelID") {
        b.info(
            &["Save", "Location"],
            v.rsplit('/').next().unwrap_or_default().to_string(),
        );
    }
    if let Some(v) = meta("CompletionPercentage").and_then(|v| v.parse::<i64>().ok()) {
        b.info(&["Save", "Completion %"], v);
    }
    if let Some(v) = meta("TotalPlaytime").and_then(|v| v.parse::<f64>().ok()) {
        b.info(&["Save", "Play time (hours)"], (v / 36.0).round() / 100.0);
    }
    if let Some(v) = meta("GameCompleted") {
        b.info(&["Save", "Game completed"], v == "true");
    }
    for m in &w.members {
        let name = MEMBERS
            .iter()
            .find(|(id, _)| *id == m.id)
            .map(|(_, n)| n.to_string())
            .unwrap_or(format!("Member {}", m.id));
        let group = if m.id == 0 { "Ryder" } else { "Squad" };
        let path = |k: &'static str| -> Vec<String> {
            if m.id == 0 {
                vec![group.into(), k.into()]
            } else {
                vec![group.into(), name.clone(), k.into()]
            }
        };
        let p = path("Level");
        int_field(
            &mut b,
            &p.iter().map(String::as_str).collect::<Vec<_>>(),
            m.snap_level,
            1,
            1,
            200,
        );
        let p = path("Skill points");
        int_field(
            &mut b,
            &p.iter().map(String::as_str).collect::<Vec<_>>(),
            m.snap_points,
            0,
            0,
            9999,
        );
        if m.id != 0 {
            continue;
        }
        if let Some(c) = m
            .items
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case("game/items/stackables/credit"))
        {
            int_field(&mut b, &["Ryder", "Credits"], c.qty, 0, 0, 999_999_999);
        }
        let mut seen: Vec<(String, String)> = vec![];
        for it in &m.items {
            let Some((g, mut n)) = resource(&it.name) else {
                continue;
            };
            let k = seen
                .iter()
                .filter(|(sg, sn)| sg == g && (sn == &n || sn.starts_with(&format!("{n} ("))))
                .count();
            if k > 0 {
                n = format!("{n} ({})", k + 1);
            }
            seen.push((g.to_string(), n.clone()));
            int_field(&mut b, &["Resources", g, &n], it.qty, 0, 0, 999_999);
        }
    }
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note(
        "Ryder's credits, crafting materials and research data, and each squad member's level and \
         skill points. A level change is copied to the member's progression data too. Not tried in \
         the game yet.",
    ))
}

/// After an edit: copy each member's snapshot level (the one shown) to their progression level,
/// which holds the same number in every save seen, then fix both CRCs.
fn fix(d: &mut Vec<u8>) {
    if let Ok(w) = walk(d) {
        for m in &w.members {
            if let Some(p) = m.prog_level {
                let v = get_bits(d, m.snap_level, 32);
                put_bits(d, p, 32, v);
            }
        }
    }
    fix_crcs(d);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// LSB-first bit writer with length frames, for building a synthetic save.
    #[derive(Default)]
    struct W {
        bits: Vec<bool>,
        frames: Vec<(usize, usize)>,
    }

    impl W {
        fn u(&mut self, n: usize, v: u64) {
            for i in 0..n {
                self.bits.push(i < 64 && (v >> i) & 1 == 1);
            }
        }
        fn s(&mut self, t: &str) {
            self.u(16, t.len() as u64);
            for c in t.bytes() {
                self.u(8, u64::from(c));
            }
        }
        fn push(&mut self, n: usize) {
            self.frames.push((self.bits.len(), n));
            self.u(n, 0);
        }
        fn pop(&mut self) {
            let (at, n) = self.frames.pop().unwrap();
            let len = self.bits.len() - at - n;
            for i in 0..n {
                self.bits[at + i] = (len >> i) & 1 == 1;
            }
        }
        fn bytes(&self) -> Vec<u8> {
            let mut out = vec![0u8; self.bits.len().div_ceil(8)];
            for (i, b) in self.bits.iter().enumerate() {
                out[i / 8] |= u8::from(*b) << (i % 8);
            }
            out
        }
        fn blob(&mut self, b: &[u8]) {
            for &x in b {
                self.u(8, u64::from(x));
            }
        }
    }

    fn member_blob(items: &[(&str, i32)], level: i32) -> Vec<u8> {
        let mut w = W::default();
        w.u(64, SAVE_MAGIC);
        w.u(1, 0);
        w.u(32, 11);
        w.u(16, 10);
        w.push(24); // unknown component
        w.u(5, 0b10110);
        w.pop();
        w.push(24); // inventory
        w.u(16, items.len() as u64);
        for (i, (name, qty)) in items.iter().enumerate() {
            w.push(24);
            w.u(32, i as u64 + 1);
            let mut it = W::default();
            it.u(32, 7);
            it.s(name);
            it.u(32, *qty as u32 as u64);
            it.u(1, 1);
            it.u(32, 0);
            w.u(24, it.bits.len() as u64);
            w.bits.extend(it.bits);
            w.pop();
        }
        w.pop();
        w.push(24); // equipment
        w.u(3, 0b101);
        w.pop();
        w.push(24); // progression
        w.u(32, level as u32 as u64);
        w.u(32, 0.25f32.to_bits().into());
        w.u(32, 1);
        w.pop();
        w.push(24); // crafting progression
        w.pop();
        w.push(24); // soldier health
        w.pop();
        w.bytes()
    }

    fn synthetic() -> Vec<u8> {
        let mut w = W::default();
        w.u(64, SAVE_MAGIC);
        w.u(1, 1);
        w.u(27, 12345);
        w.u(64, 0);
        w.s("Career00000000-Test");
        for v in [3, 22, 10, 0] {
            w.u(16, v);
        }
        w.u(32, 0);
        w.s("Game/Levels/Test");
        w.u(32, 0);
        w.u(16, 1);
        w.s("bundle");
        w.u(16, 1);
        w.push(24);
        w.s("k");
        w.s("v");
        w.pop();
        w.u(12, 0);
        w.u(12, 0);
        w.u(128, 0);
        w.u(20, 0);
        w.u(16, 5);
        w.u(16, 10);
        w.u(1, 0);
        let frame_pos = w.bits.len();
        w.u(26, 0); // patched below
        w.u(26, 0); // component data position (not used)
        w.u(32, 9);
        w.u(9, 0x1FF);
        let here = w.bits.len();
        w.push(24);
        w.push(24);
        w.s("a");
        w.pop();
        w.pop();
        w.u(26, here as u64);
        // Agent bulk: the party manager's part 4.
        let mut bulk = W::default();
        bulk.push(24);
        bulk.push(24);
        bulk.u(32, 12);
        bulk.u(16, 0);
        let ryder = member_blob(
            &[
                (
                    "Game/Items/Research/Materials/Mineral/RM_ElementZero_01",
                    936,
                ),
                ("Game/Items/Stackables/Credit", 100),
            ],
            4,
        );
        let cora = member_blob(&[("Game/Items/Stackables/Credit", 100)], 2);
        bulk.u(16, 3);
        for b in [&ryder, &cora, &vec![]] {
            bulk.push(24);
            bulk.u(32, b.len() as u64);
            bulk.blob(b);
            bulk.u(2, 0);
            bulk.pop();
        }
        bulk.u(16, 3);
        for (lvl, sp, id) in [(4, 7, 0i32), (2, 3, 1), (-1, -1, -1)] {
            bulk.push(24);
            for v in [lvl, sp, 1, 3, id, 250, 500] {
                bulk.u(32, v as u32 as u64);
            }
            bulk.u(16, 0);
            bulk.pop();
        }
        bulk.u(16 + 128 + 32 + 16 + 12 * 32, 0);
        bulk.pop();
        bulk.pop();
        w.u(32, bulk.bits.len() as u64 + 3);
        let bulk_at = w.bits.len();
        w.u(3, 0);
        w.bits.extend(bulk.bits);
        w.u(4, 1);
        w.u(8, 7);
        for o in [0, 0, 0, bulk_at as u64 + 3, 0] {
            w.u(26, o);
        }
        w.push(24);
        w.u(16, 0);
        w.pop();
        let fp = w.bits.len() as u64;
        for i in 0..26 {
            w.bits[frame_pos + i] = (fp >> i) & 1 == 1;
        }
        w.push(24);
        w.u(4, 2);
        for (name, idx) in [
            ("MEPlotSaveGameAgent", 1u64),
            ("MEPartyManagerSaveGameAgent", 7),
        ] {
            w.u(32, u64::from(djb(name)));
            w.u(8, idx);
            w.u(16, 4);
        }
        w.pop();
        w.u(32, 0);
        let data = w.bytes();

        let mut meta = b"FBMETA\0\0".to_vec();
        meta.extend(1u16.to_le_bytes());
        meta.extend(2u32.to_le_bytes());
        for (k, v) in [("ProfileName", "Tester"), ("PlayerLevel", "5")] {
            meta.extend(djb(k).to_le_bytes());
            meta.extend((v.len() as u16).to_le_bytes());
            meta.extend(v.as_bytes());
        }
        let mut d = MAGIC.to_vec();
        d.extend(1u16.to_le_bytes());
        d.extend((meta.len() as u32 + 4).to_le_bytes());
        d.extend((data.len() as u32 + 4).to_le_bytes());
        d.extend([0; 4]);
        d.extend(&meta);
        d.extend([0; 4]);
        d.extend(&data);
        fix_crcs(&mut d);
        d
    }

    #[test]
    fn synthetic_save_edits_and_crcs() {
        assert_eq!(crc32(b"123456789", 0), 0xCBF4_3926);
        let d = synthetic();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/Save/Profile name"), Some(Value::from("Tester")));
        assert_eq!(v(&doc, "/Ryder/Credits"), Some(Value::from(100)));
        assert_eq!(v(&doc, "/Ryder/Level"), Some(Value::from(5)));
        assert_eq!(v(&doc, "/Ryder/Skill points"), Some(Value::from(7)));
        assert_eq!(v(&doc, "/Squad/Cora/Level"), Some(Value::from(3)));
        assert_eq!(
            v(&doc, "/Resources/Minerals/Element Zero"),
            Some(Value::from(936))
        );
        assert_eq!(doc.to_bytes().unwrap(), d);

        doc.set("/Ryder/Credits", Value::from(999_999)).unwrap();
        doc.set("/Squad/Cora/Level", Value::from(50)).unwrap();
        doc.set("/Resources/Minerals/Element Zero", Value::from(5000))
            .unwrap();
        assert!(doc.set("/Ryder/Level", Value::from(0)).is_err());
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let again = parse(&out).expect("CRCs fixed");
        assert_eq!(v(&again, "/Ryder/Credits"), Some(Value::from(999_999)));
        assert_eq!(v(&again, "/Squad/Cora/Level"), Some(Value::from(50)));
        assert_eq!(v(&again, "/Ryder/Level"), Some(Value::from(5)));
        assert_eq!(
            v(&again, "/Resources/Minerals/Element Zero"),
            Some(Value::from(5000))
        );
        // The level went to Cora's progression data as well.
        let w = walk(&out).unwrap();
        let cora = w.members.iter().find(|m| m.id == 1).unwrap();
        assert_eq!(get_bits(&out, cora.prog_level.unwrap(), 32), 49);
        let ryder = w.members.iter().find(|m| m.id == 0).unwrap();
        assert_eq!(get_bits(&out, ryder.prog_level.unwrap(), 32), 4);
    }

    #[test]
    #[ignore]
    fn research_real_save() {
        let path = std::env::var("SAVE_EDITOR_MEA").expect("set SAVE_EDITOR_MEA");
        let d = std::fs::read(path).unwrap();
        let mut doc = parse(&d).unwrap();
        println!("{}", serde_json::to_string_pretty(&doc.view).unwrap());
        assert_eq!(
            doc.to_bytes().unwrap(),
            d,
            "unedited save comes back byte for byte"
        );
        let credits = doc
            .view
            .pointer("/Ryder/Credits")
            .and_then(Value::as_i64)
            .expect("credits");
        doc.set("/Ryder/Credits", Value::from(credits + 12_345))
            .unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).expect("re-parses with good CRCs");
        assert_eq!(
            again.view.pointer("/Ryder/Credits").and_then(Value::as_i64),
            Some(credits + 12_345)
        );
        let diff = d.iter().zip(&out).filter(|(a, b)| a != b).count();
        println!(
            "credits {credits} -> {}, {diff} bytes differ",
            credits + 12_345
        );
    }
}
