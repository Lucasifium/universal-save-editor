//! Horizon Zero Dawn (PC, Complete Edition) `checkpoint.dat` saves.
//!
//! ```text
//! 0x000  32-byte version text "HRZ-PCR ...", u32 game version, u8 save version, ...
//! 0x130  save type, four GUIDs, f64 play time in seconds
//! 0x1EC  MD5 of the data block, u32 data block length
//! 0x200  data block; its last 8 bytes give the offsets of the type section and the
//!        string/GUID pools, both relative to 0x200
//! ```
//!
//! The type section stores the class layouts (member type and name) of every object saved
//! in the block, so the player's inventory and character progression can be walked without
//! the game's files. Objects are `var-int id, u8 1, var-int layout index, members`.
//! Values are changed in place without changing their width, and the MD5 is made again.
//! Layout worked out with the help of the format notes in Nukem9's HZDCoreEditor (facts only).

use crate::tree::{Backend, Patches, TreeDoc};
use md5::{Digest, Md5};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Horizon Zero Dawn save";
/// What the Forbidden West file kind is called.
pub const KIND_FW: &str = "Horizon Forbidden West save";
const BASE: usize = 0x200;
/// Item resource GUID of metal shards (the money), as stored in the save.
const SHARDS: [u8; 16] = [
    0xf7, 0xd8, 0xda, 0x0e, 0x21, 0x24, 0x36, 0x1a, 0x81, 0xbe, 0x39, 0x25, 0x1b, 0x2b, 0xe0, 0xec,
];
const MAX_SHARDS: i64 = 9_999_999;

struct R<'a> {
    d: &'a [u8],
    p: usize,
    end: usize,
}

impl<'a> R<'a> {
    fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        if n > self.end.checked_sub(self.p)? {
            return None;
        }
        self.p += n;
        let d: &'a [u8] = self.d;
        Some(&d[self.p - n..self.p])
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.bytes(1)?[0])
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.bytes(4)?.try_into().ok()?))
    }
    fn vi(&mut self) -> Option<i32> {
        match self.u8()? as i8 {
            -128 => self.i32(),
            -127 => Some(i16::from_le_bytes(self.bytes(2)?.try_into().ok()?) as i32),
            b => Some(b as i32),
        }
    }
    fn count(&mut self) -> Option<usize> {
        let n = usize::try_from(self.vi()?).ok()?;
        (n <= self.end - self.p).then_some(n)
    }
}

/// A var-int that was read: where it is, how wide, its value.
#[derive(Clone, Copy, Debug)]
struct Int {
    at: usize,
    width: usize,
    value: i32,
}

struct Doc<'a> {
    strings: Vec<(usize, Vec<&'a [u8]>)>,
    guids: Vec<Vec<[u8; 16]>>,
    objects: HashMap<i32, String>,
    layouts: Vec<(String, Vec<(String, String)>)>,
}

impl<'a> Doc<'a> {
    fn string(&self, r: &mut R) -> Option<String> {
        let (i, o) = (usize::try_from(r.vi()?).ok()?, usize::try_from(r.vi()?).ok()?);
        let (size, blocks) = self.strings.get(i)?;
        let b = blocks.get(o / size)?;
        let s = &b[o % size..];
        let n = s.iter().position(|&c| c == 0)?;
        Some(String::from_utf8_lossy(&s[..n]).into_owned())
    }
    fn guid(&self, r: &mut R) -> Option<Option<[u8; 16]>> {
        let v = i16::from_le_bytes(r.bytes(2)?.try_into().ok()?);
        if v == -1 {
            return Some(None);
        }
        Some(Some(*self.guids.get(((v >> 8) & 0xff) as usize)?.get((v & 0xff) as usize)?))
    }
}

fn open_doc(d: &[u8]) -> Option<(Doc<'_>, usize)> {
    let n = u32::from_le_bytes(d.get(0x1FC..0x200)?.try_into().ok()?) as usize;
    let end = BASE.checked_add(n).filter(|&e| e <= d.len() && n >= 8)?;
    let tail = &d[end - 8..end];
    let types = u32::from_le_bytes(tail[..4].try_into().ok()?) as usize;
    let pools = u32::from_le_bytes(tail[4..].try_into().ok()?) as usize;
    let mut doc = Doc { strings: vec![], guids: vec![], objects: HashMap::new(), layouts: vec![] };
    let mut r = R { d, p: BASE.checked_add(pools)?, end };
    for _ in 0..r.count()? {
        let count = usize::try_from(r.i32()?).ok()?;
        let size = usize::try_from(r.i32()?).ok()?;
        if size == 0 || !size.is_power_of_two() {
            return None;
        }
        let mut blocks = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            blocks.push(r.bytes(size)?);
        }
        let extra = usize::try_from(r.i32()?).ok()?;
        r.bytes(extra.checked_mul(8)?)?;
        doc.strings.push((size, blocks));
    }
    for _ in 0..r.count()? {
        let n = r.count()?;
        r.bytes(n.checked_mul(2)?)?;
    }
    for _ in 0..256 {
        let n = r.count()?;
        let mut t = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            t.push(r.bytes(16)?.try_into().ok()?);
        }
        doc.guids.push(t);
    }
    let mut r = R { d, p: BASE.checked_add(types)?, end };
    for _ in 0..r.count()? {
        let t = doc.string(&mut r)?;
        for _ in 0..r.count()? {
            doc.objects.insert(r.vi()?, t.clone());
        }
    }
    let hash_width = if d.starts_with(b"HRZ2") { 2 } else { 16 };
    for _ in 0..r.count()? {
        r.vi()?;
        r.bytes(hash_width)?;
    }
    for _ in 0..r.count()? {
        let class = doc.string(&mut r)?;
        let mut members = vec![];
        for _ in 0..r.count()? {
            let ty = doc.string(&mut r)?;
            doc.string(&mut r)?;
            members.push((ty, doc.string(&mut r)?));
        }
        doc.layouts.push((class, members));
    }
    Some((doc, BASE.checked_add(types)?))
}

/// One inventory entry: what it is and where its amount sits.
#[derive(Clone, Debug)]
struct Item {
    guid: [u8; 16],
    path: String,
    depth: usize,
    amount: Amount,
}

#[derive(Clone, Copy, Debug)]
enum Amount {
    /// Zero Dawn: an f32 at this offset.
    Float(usize, f32),
    /// Forbidden West: a var-int.
    Int(Int),
}

/// What one walked object holds that this screen needs.
#[derive(Default, Debug)]
struct Obj {
    class: String,
    ints: Vec<(String, Int)>,
    /// (item resource GUID, item resource path, where its f32 amount is, amount).
    items: Vec<Item>,
}

struct Walker<'a, 'b> {
    doc: &'b Doc<'a>,
    depth: usize,
    out: Obj,
    resource: Option<([u8; 16], String)>,
    amount: Option<Amount>,
}

impl Walker<'_, '_> {
    fn handle(&mut self, r: &mut R) -> Option<()> {
        let id = r.vi()?;
        if id == 0 || id & 1 == 0 {
            return Some(());
        }
        if r.u8()? != 1 {
            return None;
        }
        self.object(r)
    }
    fn object(&mut self, r: &mut R) -> Option<()> {
        self.depth += 1;
        if self.depth > 32 {
            return None;
        }
        let (class, members) = self.doc.layouts.get(usize::try_from(r.vi()?).ok()?)?;
        if self.depth == 1 {
            self.out.class = class.clone();
        }
        let saved = (self.resource.take(), self.amount.take());
        for (ty, name) in members {
            self.value(r, ty, name)?;
        }
        if let (Some((guid, path)), Some(amount)) = (self.resource.take(), self.amount.take()) {
            self.out.items.push(Item { guid, path, depth: self.depth, amount });
        }
        (self.resource, self.amount) = saved;
        self.depth -= 1;
        Some(())
    }
    fn value(&mut self, r: &mut R, ty: &str, name: &str) -> Option<()> {
        if let Some(inner) = ty.strip_prefix("Array_") {
            let n = r.count()?;
            if inner == "uint8" {
                r.bytes(n)?;
            } else {
                for _ in 0..n {
                    self.value(r, inner, "")?;
                }
            }
            return Some(());
        }
        if ty.starts_with("Ref_") || ty.starts_with("cptr_") {
            return self.handle(r);
        }
        if ty.starts_with("StreamingRef_") {
            let path = self.doc.string(r)?;
            let g = self.doc.guid(r)?;
            r.u8()?;
            if name == "ItemResource" || name == "Resource" {
                self.resource = g.map(|g| (g, path));
            }
            return Some(());
        }
        match ty {
            "GGUUID" => {
                self.doc.guid(r)?;
            }
            "String" => {
                self.doc.string(r)?;
            }
            "WString" => {
                r.vi()?;
            }
            "bool" | "EHorseRestoreState" | "EQuestRunState" => {
                if r.u8()? > 1 && ty == "bool" {
                    return None;
                }
            }
            "int" | "uint" => {
                let at = r.p;
                let value = r.vi()?;
                if name == "Amount" && ty == "int" {
                    self.amount = Some(Amount::Int(Int { at, width: r.p - at, value }));
                }
                if self.depth == 1 {
                    self.out.ints.push((name.to_string(), Int { at, width: r.p - at, value }));
                }
            }
            "float" => {
                let at = r.p;
                let v = f32::from_le_bytes(r.bytes(4)?.try_into().ok()?);
                if name == "Amount" {
                    self.amount = Some(Amount::Float(at, v));
                }
            }
            "uint64" | "int64" | "double" => {
                r.bytes(8)?;
            }
            "WorldTransform" => {
                let f = r.u8()?;
                if f & 1 == 0 {
                    if f & 2 == 0 {
                        r.bytes(60)?;
                    } else {
                        r.bytes(if f & 4 == 0 { 16 } else { 0 } + if f & 8 == 0 { 24 } else { 0 })?;
                    }
                }
            }
            t if t.starts_with('E') => {
                r.bytes(4)?;
            }
            _ => return self.object(r),
        }
        Some(())
    }
}

fn enc(v: i32) -> Vec<u8> {
    if (-126..0x80).contains(&v) {
        vec![v as u8]
    } else if (i16::MIN as i32..=i16::MAX as i32).contains(&v) {
        let mut b = vec![0x81];
        b.extend((v as i16).to_le_bytes());
        b
    } else {
        let mut b = vec![0x80];
        b.extend(v.to_le_bytes());
        b
    }
}

/// Finds the one saved object of `class` and walks it.
fn walk(d: &[u8], doc: &Doc, types_at: usize, class: &str) -> Option<Obj> {
    let index = doc.layouts.iter().position(|(c, _)| c == class)?;
    let (&id, _) = doc.objects.iter().find(|(_, c)| *c == class)?;
    let mut pat = enc(id);
    pat.push(1);
    pat.extend(enc(index as i32));
    let mut at = BASE;
    while let Some(i) = d[at..types_at].windows(pat.len()).position(|w| w == pat.as_slice()) {
        let start = at + i;
        let mut r = R { d, p: start + pat.len() - enc(index as i32).len(), end: types_at };
        let mut w = Walker { doc, depth: 0, out: Obj::default(), resource: None, amount: None };
        if w.object(&mut r).is_some() && w.out.class == class {
            return Some(w.out);
        }
        at = start + 1;
    }
    None
}

/// One resource of the inventory the screen lists.
#[derive(Clone, Debug)]
struct Mat {
    name: String,
    amount: Amount,
}

#[derive(Clone, Debug)]
struct Save {
    /// Bytes before the checkpoint (the Forbidden West container header) and after it.
    pre: Vec<u8>,
    post: Vec<u8>,
    /// The checkpoint: 0x200-byte header and the data block.
    raw: Vec<u8>,
    shards: Amount,
    xp: Int,
    perks: Int,
    level: Int,
    mats: Vec<Mat>,
    hfw: bool,
}

/// Forbidden West wraps the checkpoint: `GGDS`, u32 1, u32 version, u32 checkpoint length,
/// u32 thumbnail length, 0x80 title bytes, 0x100 subtitle bytes, then the checkpoint.
const GGDS_HEAD: usize = 0x194;

/// Splits a file into (container header, checkpoint, rest).
fn split(d: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
    if d.starts_with(b"GGDS") {
        let n = u32::from_le_bytes(d.get(0xC..0x10)?.try_into().ok()?) as usize;
        let end = GGDS_HEAD.checked_add(n)?;
        let cp = d.get(GGDS_HEAD..end)?;
        return Some((&d[..GGDS_HEAD], cp, &d[end..]));
    }
    Some((&d[..0], d, &d[d.len()..]))
}

fn open(file: &[u8]) -> Option<Save> {
    if !looks(file) {
        return None;
    }
    let (pre, d, post) = split(file)?;
    let n = u32::from_le_bytes(d.get(0x1FC..0x200)?.try_into().ok()?) as usize;
    if Md5::digest(d.get(BASE..BASE + n)?).as_slice() != &d[0x1EC..0x1FC] {
        return None;
    }
    let hfw = d.starts_with(b"HRZ2");
    let (doc, types_at) = open_doc(d)?;
    let inv = walk(d, &doc, types_at, "InventoryRestoreState")?;
    let prog = walk(d, &doc, types_at, if hfw { "CharacterProgressionRestoreState" } else { "CharacterProgressionSave" })?;
    let top = inv.items.iter().map(|i| i.depth).min()?;
    let items: Vec<&Item> = inv.items.iter().filter(|i| i.depth == top).collect();
    let shard_guid = if hfw { super::hzd_items::HFW_SHARDS } else { SHARDS };
    let shards = items.iter().find(|i| i.guid == shard_guid)?.amount;
    let int = |n: &str| prog.ints.iter().find(|(m, _)| m == n).map(|x| x.1);
    let mut mats: Vec<Mat> = vec![];
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for i in &items {
        let name = if hfw {
            super::hzd_items::HFW_ITEMS.iter().find(|x| x.0 == i.guid).map(|x| x.1)
        } else {
            super::hzd_items::HZD_ITEMS.iter().find(|x| x.0 == i.guid).map(|x| x.1)
        };
        let Some(name) = name else { continue };
        let k = seen.entry(name).or_insert(0);
        *k += 1;
        let name = if *k == 1 { name.to_string() } else { format!("{name} ({k})") };
        mats.push(Mat { name, amount: i.amount });
    }
    Some(Save {
        pre: pre.to_vec(),
        post: post.to_vec(),
        raw: d.to_vec(),
        shards,
        xp: int("Xp")?,
        perks: int("AvailablePerkPoints")?,
        level: int("CurrentLevel")?,
        mats,
        hfw,
    })
}

pub fn looks(d: &[u8]) -> bool {
    let inner = if d.starts_with(b"GGDS") { d.get(GGDS_HEAD..GGDS_HEAD + 4).unwrap_or(&[]) } else { d };
    (inner.starts_with(b"HRZ-") || inner.starts_with(b"HRZ2")) && d.len() > BASE
}

/// The largest value a var-int can take without changing its width.
fn cap(i: Int) -> i64 {
    match i.width {
        1 => 127,
        3 => i16::MAX as i64,
        _ => i32::MAX as i64,
    }
}

fn put(i: Int, v: i32) -> Vec<u8> {
    match i.width {
        1 => vec![v as u8],
        3 => {
            let mut b = vec![0x81];
            b.extend((v as i16).to_le_bytes());
            b
        }
        _ => {
            let mut b = vec![0x80];
            b.extend(v.to_le_bytes());
            b
        }
    }
}

const SHARDS_LEAF: usize = 0;
const PERKS_LEAF: usize = 1;
const MAT_LEAF: usize = 2;
/// Most of one resource the screen accepts.
const MAX_MATERIAL: i64 = 99_999;

#[derive(Clone, Debug)]
struct B(Save);

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let s = &self.0;
        let mut p = Patches::default();
        // Bytes added by edits whose var-int grows (the data block shifts after them).
        let mut grown: i64 = 0;
        let mut amount = |p: &mut Patches, a: Amount, n: i64| match a {
            Amount::Float(at, _) => p.add(at, at + 4, (n as f32).to_le_bytes().to_vec()),
            Amount::Int(i) => {
                let bytes = if n <= cap(i) { put(i, n as i32) } else { enc(n as i32) };
                grown += bytes.len() as i64 - i.width as i64;
                p.add(i.at, i.at + i.width, bytes);
            }
        };
        for (id, v) in changes {
            let n = v.as_i64().ok_or("Must be a whole number")?;
            match *id {
                SHARDS_LEAF => amount(&mut p, s.shards, n),
                PERKS_LEAF => p.add(s.perks.at, s.perks.at + s.perks.width, put(s.perks, n as i32)),
                m => amount(&mut p, s.mats.get(m - MAT_LEAF).ok_or("Not editable")?.amount, n),
            }
        }
        let mut out = p.apply(&s.raw)?;
        let mut pre = s.pre.clone();
        if grown != 0 {
            let n = u32::from_le_bytes(out[0x1FC..0x200].try_into().unwrap()) as i64 + grown;
            out[0x1FC..0x200].copy_from_slice(&(n as u32).to_le_bytes());
            // The type section and the pools sit after every edit: their offsets move too.
            let end = BASE + n as usize;
            for at in [end - 8, end - 4] {
                let v = u32::from_le_bytes(out[at..at + 4].try_into().unwrap()) as i64 + grown;
                out[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
            }
            if pre.len() >= 0x10 {
                let n = u32::from_le_bytes(pre[0xC..0x10].try_into().unwrap()) as i64 + grown;
                pre[0xC..0x10].copy_from_slice(&(n as u32).to_le_bytes());
            }
        }
        let n = u32::from_le_bytes(out[0x1FC..0x200].try_into().unwrap()) as usize;
        let md5 = Md5::digest(&out[BASE..BASE + n]);
        out[0x1EC..0x1FC].copy_from_slice(&md5);
        pre.extend(out);
        pre.extend(&s.post);
        Ok(pre)
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let max = match leaf {
            SHARDS_LEAF => MAX_SHARDS,
            PERKS_LEAF => cap(self.0.perks),
            _ => MAX_MATERIAL,
        };
        match v.as_i64() {
            Some(n) if (0..=max).contains(&n) => Ok(()),
            _ => Err(format!("Must be a whole number from 0 to {max}")),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn amount_value(d: &[u8], a: Amount) -> i64 {
    match a {
        Amount::Float(at, _) => f32::from_le_bytes(d[at..at + 4].try_into().unwrap()).round() as i64,
        Amount::Int(i) => i.value as i64,
    }
}

pub fn parse(file: &[u8]) -> Option<TreeDoc> {
    let s = open(file)?;
    let d = &s.raw;
    let mut aloy = Map::new();
    aloy.insert("Metal shards".into(), Value::from(amount_value(d, s.shards)));
    aloy.insert("Skill points".into(), Value::from(s.perks.value));
    aloy.insert("Level".into(), Value::from(s.level.value));
    aloy.insert("Experience".into(), Value::from(s.xp.value));
    let mut edit = HashMap::new();
    edit.insert("/Aloy/Metal shards".to_string(), SHARDS_LEAF);
    edit.insert("/Aloy/Skill points".to_string(), PERKS_LEAF);
    let mut mats = Map::new();
    for (i, m) in s.mats.iter().enumerate() {
        mats.insert(m.name.clone(), Value::from(amount_value(d, m.amount)));
        edit.insert(format!("/Materials/{}", m.name), MAT_LEAF + i);
    }
    let mut root = Map::new();
    root.insert("Aloy".into(), Value::Object(aloy));
    root.insert("Materials".into(), Value::Object(mats));
    let kind = if s.hfw { KIND_FW } else { KIND };
    Some(TreeDoc::new(Value::Object(root), edit, kind, Box::new(B(s))))
}

#[cfg(test)]
mod tests {
    use super::*;


    /// Builds a small save: one string table, no wide strings, a GUID pool holding the
    /// shards GUID, and the inventory and progression objects.
    fn sample() -> Vec<u8> {
        // String table: one block of 256 bytes.
        let names = [
            "InventoryRestoreState", "ItemListRestoreState", "ItemRestoreState", "CharacterProgressionSave",
            "Ref_ItemListRestoreState", "Array_Ref_ItemRestoreState", "StreamingRef_EntityResource", "float", "int", "bool",
            "GGUUID", "StateSaving", "InventoryItemList", "Items", "ItemResource", "Amount", "Xp", "CurrentLevel",
            "AvailablePerkPoints", "PerksUnlocked", "ObjectUUID", "entities/lootdrops/robot_loot_items",
        ];
        let mut block = vec![];
        let mut off = HashMap::new();
        for n in names {
            off.insert(n, block.len());
            block.extend(n.as_bytes());
            block.push(0);
        }
        block.resize(1024, 0);
        let s = |n: &str| {
            let mut b = enc(0);
            b.extend(enc(off[n] as i32));
            b
        };
        // Objects (data section).
        let mut data = vec![0u8; 8];
        data.extend(enc(1));
        data.push(1);
        data.extend(enc(0)); // InventoryRestoreState
        data.extend(enc(3));
        data.push(1);
        data.extend(enc(1)); // ItemListRestoreState
        data.extend(enc(1)); // one item
        data.extend(enc(5));
        data.push(1);
        data.extend(enc(2)); // ItemRestoreState
        data.extend(s("entities/lootdrops/robot_loot_items"));
        data.extend(0i16.to_le_bytes());
        data.push(0);
        data.extend(1234.0f32.to_le_bytes());
        data.extend(enc(7));
        data.push(1);
        data.extend(enc(3)); // CharacterProgressionSave
        data.extend((-1i16).to_le_bytes());
        data.extend([0x80]);
        data.extend(2_360_000i32.to_le_bytes());
        data.extend(enc(60));
        data.extend(enc(107));
        data.push(1);
        data.extend(b"pad");
        // Type section.
        let types = data.len();
        data.extend(enc(4));
        for (n, id) in [("InventoryRestoreState", 1), ("ItemListRestoreState", 3), ("ItemRestoreState", 5), ("CharacterProgressionSave", 7)] {
            data.extend(s(n));
            data.extend(enc(1));
            data.extend(enc(id));
        }
        data.extend(enc(0));
        let layouts: [(&str, &[(&str, &str)]); 4] = [
            ("InventoryRestoreState", &[("Ref_ItemListRestoreState", "InventoryItemList")]),
            ("ItemListRestoreState", &[("Array_Ref_ItemRestoreState", "Items")]),
            ("ItemRestoreState", &[("StreamingRef_EntityResource", "ItemResource"), ("float", "Amount")]),
            (
                "CharacterProgressionSave",
                &[("GGUUID", "ObjectUUID"), ("int", "Xp"), ("int", "CurrentLevel"), ("int", "AvailablePerkPoints"), ("bool", "PerksUnlocked")],
            ),
        ];
        data.extend(enc(4));
        for (c, members) in layouts {
            data.extend(s(c));
            data.extend(enc(members.len() as i32));
            for (t, n) in members {
                data.extend(s(t));
                data.extend(s("StateSaving"));
                data.extend(s(n));
            }
        }
        // Pools.
        let pools = data.len();
        data.extend(enc(1));
        data.extend(1i32.to_le_bytes());
        data.extend(1024i32.to_le_bytes());
        data.extend(&block);
        data.extend(0i32.to_le_bytes());
        data.extend(enc(0));
        data.extend(enc(1));
        data.extend(SHARDS);
        for _ in 1..256 {
            data.extend(enc(0));
        }
        data.extend((types as u32).to_le_bytes());
        data.extend((pools as u32).to_le_bytes());
        let mut d = vec![0u8; BASE];
        d[..8].copy_from_slice(b"HRZ-PCR ");
        d[0x1EC..0x1FC].copy_from_slice(&Md5::digest(&data));
        d[0x1FC..0x200].copy_from_slice(&(data.len() as u32).to_le_bytes());
        d.extend(data);
        d.resize(d.len() + 64, 0);
        d
    }

    #[test]
    fn reads_and_edits_shards_and_skill_points() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Aloy/Metal shards").unwrap(), 1234);
        assert_eq!(doc.view.pointer("/Aloy/Level").unwrap(), 60);
        assert_eq!(doc.view.pointer("/Aloy/Experience").unwrap(), 2_360_000);
        doc.set("/Aloy/Metal shards", Value::from(9_999_999)).unwrap();
        doc.set("/Aloy/Skill points", Value::from(120)).unwrap();
        assert!(doc.set("/Aloy/Skill points", Value::from(500)).is_err());
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Aloy/Metal shards").unwrap(), 9_999_999);
        assert_eq!(back.view.pointer("/Aloy/Skill points").unwrap(), 120);
    }

    /// A small Forbidden West save inside its container: shards (a one-byte amount, so raising it
    /// grows the block), one named resource and the character progression.
    fn sample_fw() -> Vec<u8> {
        let names = [
            "InventoryRestoreState", "InventoryItemRestoreState", "CharacterProgressionRestoreState", "Array_Ref_InventoryItemRestoreState",
            "Ref_InventoryItemRestoreState", "StreamingRef_InventoryItemResource", "GGUUID", "int", "bool", "StateSaving", "InventoryItems",
            "ObjectUUID", "Amount", "Resource", "Xp", "CurrentLevel", "AvailablePerkPoints", "PerksUnlocked",
        ];
        let mut block = vec![];
        let mut off = HashMap::new();
        for n in names {
            off.insert(n, block.len());
            block.extend(n.as_bytes());
            block.push(0);
        }
        block.resize(1024, 0);
        let s = |n: &str| {
            let mut b = enc(0);
            b.extend(enc(off[n] as i32));
            b
        };
        let mut data = vec![0u8; 8];
        data.extend(enc(1));
        data.push(1);
        data.extend(enc(0)); // InventoryRestoreState
        data.extend(enc(2)); // two items
        let item = |data: &mut Vec<u8>, id: i32, guid: i16, amount: &[u8]| {
            data.extend(enc(id));
            data.push(1);
            data.extend(enc(1)); // InventoryItemRestoreState
            data.extend((-1i16).to_le_bytes()); // ObjectUUID
            data.extend(amount);
            data.extend(enc(0)); // resource path
            data.extend(enc(0));
            data.extend(guid.to_le_bytes());
            data.push(0);
        };
        item(&mut data, 3, 0, &[6]);
        item(&mut data, 5, 1, &[0x81, 0x2c, 0x01]);
        data.extend(enc(7));
        data.push(1);
        data.extend(enc(2)); // CharacterProgressionRestoreState
        data.extend((-1i16).to_le_bytes());
        data.extend([0x80]);
        data.extend(1_000_000i32.to_le_bytes());
        data.extend(enc(40));
        data.extend(enc(30));
        data.push(1);
        data.extend(b"pad");
        let types = data.len();
        data.extend(enc(3));
        for (n, id) in [("InventoryRestoreState", 1), ("InventoryItemRestoreState", 3), ("CharacterProgressionRestoreState", 7)] {
            data.extend(s(n));
            data.extend(enc(1));
            data.extend(enc(id));
        }
        data.extend(enc(1)); // type hashes are two bytes wide in Forbidden West
        data.extend(enc(2));
        data.extend([0xab, 0xcd]);
        let layouts: [(&str, Vec<(&str, &str)>); 3] = [
            ("InventoryRestoreState", vec![("Array_Ref_InventoryItemRestoreState", "InventoryItems")]),
            ("InventoryItemRestoreState", vec![("GGUUID", "ObjectUUID"), ("int", "Amount"), ("StreamingRef_InventoryItemResource", "Resource")]),
            (
                "CharacterProgressionRestoreState",
                vec![("GGUUID", "ObjectUUID"), ("int", "Xp"), ("int", "CurrentLevel"), ("int", "AvailablePerkPoints"), ("bool", "PerksUnlocked")],
            ),
        ];
        data.extend(enc(3));
        for (c, members) in layouts {
            data.extend(s(c));
            data.extend(enc(members.len() as i32));
            for (t, n) in members {
                data.extend(s(t));
                data.extend(s("StateSaving"));
                data.extend(s(n));
            }
        }
        let pools = data.len();
        data.extend(enc(1));
        data.extend(1i32.to_le_bytes());
        data.extend(1024i32.to_le_bytes());
        data.extend(&block);
        data.extend(0i32.to_le_bytes());
        data.extend(enc(0));
        data.extend(enc(2));
        data.extend(super::super::hzd_items::HFW_SHARDS);
        data.extend(super::super::hzd_items::HFW_ITEMS[0].0);
        for _ in 1..256 {
            data.extend(enc(0));
        }
        data.extend((types as u32).to_le_bytes());
        data.extend((pools as u32).to_le_bytes());
        let mut cp = vec![0u8; BASE];
        cp[..12].copy_from_slice(b"HRZ2-PCR 37/");
        cp[0x1EC..0x1FC].copy_from_slice(&Md5::digest(&data));
        cp[0x1FC..0x200].copy_from_slice(&(data.len() as u32).to_le_bytes());
        cp.extend(data);
        let mut d = vec![0u8; GGDS_HEAD];
        d[..4].copy_from_slice(b"GGDS");
        d[4..8].copy_from_slice(&1u32.to_le_bytes());
        d[0xC..0x10].copy_from_slice(&(cp.len() as u32).to_le_bytes());
        d.extend(cp);
        d.extend(b"\x89PNG thumbnail");
        d
    }

    #[test]
    fn forbidden_west_reads_edits_and_grows_amounts() {
        let d = sample_fw();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, KIND_FW);
        assert_eq!(doc.to_bytes().unwrap(), d, "an unchanged save is written back byte for byte");
        assert_eq!(doc.view.pointer("/Aloy/Metal shards").unwrap(), 6);
        assert_eq!(doc.view.pointer("/Aloy/Level").unwrap(), 40);
        let first = super::super::hzd_items::HFW_ITEMS[0].1;
        assert_eq!(doc.view.pointer(&format!("/Materials/{first}")).unwrap(), 300);
        // 6 needs one byte, 200 needs three: the block grows and every offset follows.
        doc.set("/Aloy/Metal shards", Value::from(200)).unwrap();
        doc.set(&format!("/Materials/{first}"), Value::from(7)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_ne!(out.len(), d.len());
        assert!(out.ends_with(b"\x89PNG thumbnail"));
        let back = parse(&out).expect("the edited save opens again");
        assert_eq!(back.view.pointer("/Aloy/Metal shards").unwrap(), 200);
        assert_eq!(back.view.pointer(&format!("/Materials/{first}")).unwrap(), 7);
        assert_eq!(back.view.pointer("/Aloy/Skill points").unwrap(), 30);
        let n = u32::from_le_bytes(out[0xC..0x10].try_into().unwrap()) as usize;
        assert_eq!(n, out.len() - GGDS_HEAD - b"\x89PNG thumbnail".len(), "the container length follows");
    }

    /// Public saves kept outside the repository: `HZD_SAMPLES=<folder>` checks every file in it.
    #[test]
    #[ignore]
    fn real_saves_open_round_trip_and_survive_edits() {
        let dir = std::env::var("HZD_SAMPLES").unwrap();
        let mut stack = vec![std::path::PathBuf::from(dir)];
        let mut n = 0;
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let raw = std::fs::read(&p).unwrap();
                if !looks(&raw) {
                    continue;
                }
                let name = p.display().to_string();
                let Some(mut doc) = parse(&raw) else { println!("NOT OPENED {name}"); continue };
                assert_eq!(doc.to_bytes().unwrap(), raw, "{name}");
                let mats = doc.view.pointer("/Materials").unwrap().as_object().unwrap().clone();
                // raise every material past a one-byte var-int, then read it back
                let paths: Vec<String> = mats.keys().map(|k| format!("/Materials/{k}")).collect();
                for q in &paths {
                    doc.set(q, Value::from(777)).unwrap();
                }
                doc.set("/Aloy/Metal shards", Value::from(123_456)).unwrap();
                let out = doc.to_bytes().unwrap();
                let back = parse(&out).unwrap_or_else(|| panic!("edited save did not open: {name}"));
                for q in &paths {
                    assert_eq!(back.view.pointer(q).unwrap(), 777, "{name} {q}");
                }
                assert_eq!(back.view.pointer("/Aloy/Metal shards").unwrap(), 123_456);
                assert_eq!(back.view.pointer("/Aloy/Level"), doc.view.pointer("/Aloy/Level"));
                println!("OK {} mats={} level={} shards_was={:?}", name.rsplit('/').next().unwrap(), mats.len(), back.view.pointer("/Aloy/Level").unwrap(), 0);
                n += 1;
            }
        }
        assert!(n > 0);
    }

}
