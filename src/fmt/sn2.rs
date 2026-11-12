//! Subnautica 2 `savegame_<n>.sav`: a "GSWU" wrapper around an Unreal GVAS file whose two
//! save blobs (current state and the previous checkpoint) keep their data as back-to-back
//! Oodle (Kraken) blocks of 128 KiB. Unpacked, the current state is one stream of Unreal
//! 5.4+ property tags (each tag carries a type-name tree instead of a flat type name); the
//! per-object records inside it are byte arrays that hold the same kind of stream again.
//!
//! Only numbers that are already written are edited, in place, so nothing moves: the stream
//! keeps its length, and packing it again only has to fix the sizes of the three properties
//! around the compressed data.
//!
//! Block and tag layout as described by the MIT-licensed sn2-save-tools (Joey Fisher).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

const TAG: [u8; 8] = [0xc1, 0x83, 0x2a, 0x9e, 0x22, 0x22, 0x22, 0x22];
const CHUNK: usize = 131072;

/// The file around the unpacked stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    head: Vec<u8>,
    tail: Vec<u8>,
    /// Offsets (in `head`) of the sizes of the three properties around the compressed data.
    size_at: [usize; 3],
    /// Offset (in `head`) of the byte count of the compressed data.
    count_at: usize,
    old_len: usize,
}

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn put_i32(d: &mut [u8], at: usize, v: i64) -> Result<(), String> {
    let v = i32::try_from(v).map_err(|_| "the save would grow past what it can hold")?;
    d.get_mut(at..at + 4).ok_or("bad save layout")?.copy_from_slice(&v.to_le_bytes());
    Ok(())
}

fn fstring(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = i32_at(d, at)?;
    if n == 0 {
        return Some((String::new(), at + 4));
    }
    if n > 0 {
        let n = n as usize;
        if n > 4096 {
            return None;
        }
        let s = d.get(at + 4..at + 4 + n)?;
        if s[n - 1] != 0 {
            return None;
        }
        Some((s[..n - 1].iter().map(|&c| c as char).collect(), at + 4 + n))
    } else {
        let n = n.unsigned_abs() as usize;
        if n > 4096 {
            return None;
        }
        let s = d.get(at + 4..at + 4 + 2 * n)?;
        let u: Vec<u16> = s.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&x| x != 0).collect();
        Some((String::from_utf16_lossy(&u), at + 4 + 2 * n))
    }
}

#[derive(Debug)]
struct Tree {
    name: String,
    inner: Vec<Tree>,
}

fn tree(d: &[u8], at: usize, depth: usize) -> Option<(Tree, usize)> {
    if depth > 8 {
        return None;
    }
    let (name, mut p) = fstring(d, at)?;
    let n = i32_at(d, p)?;
    p += 4;
    if !(0..=8).contains(&n) {
        return None;
    }
    let mut inner = vec![];
    for _ in 0..n {
        let (t, q) = tree(d, p, depth + 1)?;
        inner.push(t);
        p = q;
    }
    Some((Tree { name, inner }, p))
}

#[derive(Debug)]
struct Tag {
    name: String,
    tree: Tree,
    size: usize,
    flags: u8,
    payload: usize,
}

enum Next {
    End(usize),
    Tag(Tag),
}

/// The property tag at `at` (or the "None" that ends a list); `end` bounds its payload.
fn next_tag(d: &[u8], at: usize, end: usize) -> Option<Next> {
    if at >= end {
        return None;
    }
    let (name, p) = fstring(d, at)?;
    if name == "None" {
        return Some(Next::End(p));
    }
    if name.is_empty() {
        return None;
    }
    let (tree, p) = tree(d, p, 0)?;
    let size = i32_at(d, p)?;
    let flags = *d.get(p + 4)?;
    // Array index (1) and property guid (2) are skipped; extensions (4) are not understood.
    let mut q = p + 5;
    if flags & 4 != 0 {
        return None;
    }
    if flags & 1 != 0 {
        q += 4;
    }
    if flags & 2 != 0 {
        q += 16;
    }
    let size = usize::try_from(size).ok()?;
    if q + size > end {
        return None;
    }
    Some(Next::Tag(Tag { name, tree, size, flags, payload: q }))
}

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    Plain,
    Item,
    Attr,
}

#[derive(Debug, Clone)]
pub struct Item {
    /// Short name of the container holding it ("Player", "Wall Locker"…).
    pub container: String,
    pub name: String,
    /// Where the count is.
    pub at: usize,
}

#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub base: usize,
    pub cur: usize,
}

struct Walk<'a> {
    d: &'a [u8],
    items: Vec<Item>,
    attrs: Vec<Attr>,
    container: String,
    steps: u64,
}

/// "/Game/Data/ItemType/Resource/DA_PlantMatter_ItemType.DA_PlantMatter_ItemType" → "Plant Matter".
fn pretty(path: &str, strip: &[&str]) -> String {
    let last = path.rsplit('/').next().unwrap_or(path);
    let mut s = last.split('.').next().unwrap_or(last);
    for p in strip {
        if let Some(r) = s.strip_prefix(p) {
            s = r;
        }
    }
    for p in ["_ItemType", "_C"] {
        if let Some(r) = s.strip_suffix(p) {
            s = r;
        }
    }
    let mut out = String::new();
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if c == '_' {
            out.push(' ');
        } else {
            if c.is_ascii_uppercase() && prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit()) {
                out.push(' ');
            }
            out.push(c);
        }
        prev = Some(c);
    }
    // "Item Type" is the data asset's own suffix (also in the middle: "… Item Type 01").
    let out = out.replace(" Item Type", "");
    out.trim().to_string()
}

impl Walk<'_> {
    /// Walks the tags from `p` to the "None" that ends them; None if they don't parse.
    fn props(&mut self, mut p: usize, end: usize, depth: usize, ctx: Ctx) -> Option<usize> {
        if depth > 24 {
            return None;
        }
        let d = self.d;
        let (mut ty, mut count, mut an, mut base, mut cur) = (None::<String>, None, None::<String>, None, None);
        loop {
            self.steps += 1;
            if self.steps > 300_000_000 {
                return None;
            }
            let t = match next_tag(d, p, end)? {
                Next::End(q) => {
                    p = q;
                    break;
                }
                Next::Tag(t) => t,
            };
            let pend = t.payload + t.size;
            let kind = t.tree.name.as_str();
            match (t.name.as_str(), kind) {
                ("ItemType", "ObjectProperty") => ty = fstring(d, t.payload).map(|x| x.0),
                ("Count", "IntProperty") if t.size == 4 => count = Some(t.payload),
                ("Name", "StrProperty") => an = fstring(d, t.payload).map(|x| x.0),
                ("BaseValue", "FloatProperty") if t.size == 4 => base = Some(t.payload),
                ("CurrentValue", "FloatProperty") if t.size == 4 => cur = Some(t.payload),
                ("InventoryClass", "ObjectProperty" | "SoftObjectProperty") => {
                    if let Some((s, _)) = fstring(d, t.payload) {
                        self.container = if s.contains("Character") { "Player".into() } else { pretty(&s, &["BP_"]) };
                    }
                }
                _ => {}
            }
            match kind {
                "StructProperty" if t.flags & 8 == 0 => {
                    let _ = self.props(t.payload, pend, depth + 1, Ctx::Plain);
                }
                "ArrayProperty" => {
                    let inner = t.tree.inner.first();
                    match inner.map(|i| i.name.as_str()) {
                        Some("StructProperty") => {
                            let elem = inner.and_then(|i| i.inner.first()).map(|e| e.name.as_str());
                            let c = match (t.name.as_str(), elem) {
                                ("Items", Some("UWEInventoryItem")) => Ctx::Item,
                                ("AttributeSaveData", Some("UWEGameplayAttributeSaveData")) => Ctx::Attr,
                                _ => Ctx::Plain,
                            };
                            if let Some(n) = i32_at(d, t.payload).filter(|n| *n >= 0) {
                                let mut q = t.payload + 4;
                                for _ in 0..n {
                                    match self.props(q, pend, depth + 1, c) {
                                        Some(r) => q = r,
                                        None => break,
                                    }
                                }
                            }
                        }
                        Some("ByteProperty") => {
                            // Per-object records: a zero byte, then more tags.
                            let n = i32_at(d, t.payload).unwrap_or(0);
                            if n > 8 && d.get(t.payload + 4) == Some(&0) {
                                let _ = self.props(t.payload + 5, pend, depth + 1, Ctx::Plain);
                            }
                        }
                        _ => {}
                    }
                }
                "MapProperty" => {
                    let k = t.tree.inner.first();
                    let v = t.tree.inner.get(1);
                    let guid_key = k.is_some_and(|k| k.name == "StructProperty" && k.inner.first().is_some_and(|g| g.name == "Guid"));
                    if guid_key && v.is_some_and(|v| v.name == "StructProperty") {
                        if let (Some(0), Some(n)) = (i32_at(d, t.payload), i32_at(d, t.payload + 4)) {
                            let mut q = t.payload + 8;
                            for _ in 0..n.max(0) {
                                match self.props(q + 16, pend, depth + 1, Ctx::Plain) {
                                    Some(r) => q = r,
                                    None => break,
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            p = pend;
        }
        match ctx {
            Ctx::Item => {
                if let (Some(ty), Some(at)) = (ty, count) {
                    self.items.push(Item { container: self.container.clone(), name: pretty(&ty, &["DA_"]), at });
                }
            }
            Ctx::Attr => {
                if let (Some(name), Some(base), Some(cur)) = (an, base, cur) {
                    self.attrs.push(Attr { name, base, cur });
                }
            }
            Ctx::Plain => {}
        }
        Some(p)
    }
}

/// What the unpacked stream holds that the screen edits.
pub struct Found {
    pub items: Vec<Item>,
    pub attrs: Vec<Attr>,
}

pub fn scan(d: &[u8]) -> Result<Found, String> {
    if d.len() < 16 || i32_at(d, 0) != Some(d.len() as i32 - 4) || d[4] != 0 {
        return Err("not a Subnautica 2 save stream".into());
    }
    let mut w = Walk { d, items: vec![], attrs: vec![], container: "Storage".into(), steps: 0 };
    w.props(5, d.len(), 0, Ctx::Plain).ok_or("couldn't read the save's properties")?;
    Ok(Found { items: w.items, attrs: w.attrs })
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 4096 && i32_at(d, 0) == Some(d.len() as i32 - 4) && d[4] == 0 && d[..4096].windows(9).any(|w| w == b"SaveData\0")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let f = scan(d)?;
    let mut b = Builder::new(d);
    b.info(&["Save", "Properties read"], f.items.len() + f.attrs.len());
    // Survival attributes: the first set found for each.
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for a in &f.attrs {
        *seen.entry(a.name.as_str()).or_default() += 1;
    }
    for name in ["Health", "Oxygen", "Food", "Water"] {
        if let Some(a) = f.attrs.iter().find(|a| a.name == name) {
            b.ranged(&["Survival", name], a.cur, Kind::F32, 0, 100_000);
            b.ranged(&["Survival", &format!("{name} (maximum)")], a.base, Kind::F32, 0, 100_000);
        }
    }
    let mut used: HashMap<String, u32> = HashMap::new();
    for it in &f.items {
        let (group, key) = if it.container == "Player" { ("Player", it.name.clone()) } else { ("Storage", format!("{} [{}]", it.name, it.container)) };
        let n = used.entry(format!("{group}/{key}")).or_default();
        *n += 1;
        let key = if *n > 1 { format!("{key} ({n})") } else { key };
        b.ranged(&[group, &key], it.at, Kind::I32, 0, i32::MAX as i64);
    }
    Ok(b.finish("Subnautica 2 save", Checksum::None))
}

// ---------------------------------------------------------------------------- the container

/// Decodes every Oodle block in `d[start..end]`.
fn decode(d: &[u8], start: usize, end: usize) -> Result<Vec<u8>, String> {
    // (data, plain size, packed?) of every chunk, in order.
    let mut jobs: Vec<(&[u8], usize, bool)> = vec![];
    let mut p = start;
    let rd = |at: usize| -> Result<i64, String> { Ok(i64::from_le_bytes(d.get(at..at + 8).ok_or("the save is cut off")?.try_into().unwrap())) };
    while p < end {
        if d.get(p..p + 8) != Some(&TAG[..]) {
            return Err("unexpected data between the save's blocks".into());
        }
        let chunk = rd(p + 8)?;
        let comp = *d.get(p + 16).ok_or("the save is cut off")?;
        let usum = rd(p + 25)?;
        p += 33;
        if chunk <= 0 || usum < 0 {
            return Err("bad block header".into());
        }
        let n = ((usum + chunk - 1) / chunk) as usize;
        let mut sizes = vec![];
        for _ in 0..n {
            sizes.push((rd(p)? as usize, rd(p + 8)? as usize));
            p += 16;
        }
        for (c, u) in sizes {
            jobs.push((d.get(p..p + c).ok_or("the save is cut off")?, u, comp == 2));
            p += c;
        }
    }
    let out = crate::par::map(&jobs, |(src, u, packed)| if *packed { crate::oodle::decompress(src, *u) } else { Some(src.to_vec()) });
    let mut all = Vec::with_capacity(jobs.iter().map(|j| j.1).sum());
    for o in out {
        all.extend(o.ok_or(crate::oodle::MISSING)?);
    }
    Ok(all)
}

fn encode(stream: &[u8]) -> Result<Vec<u8>, String> {
    let pieces: Vec<&[u8]> = stream.chunks(CHUNK).collect();
    let packed = crate::par::map(&pieces, |p| {
        let c = crate::oodle::compress(p)?;
        // Check it reads back before it goes into the save.
        (crate::oodle::decompress(&c, p.len())? == *p).then_some(c)
    });
    let mut out = vec![];
    for (p, c) in pieces.iter().zip(packed) {
        let c = c.ok_or("Oodle couldn't pack the save again (no Oodle library found)")?;
        out.extend(TAG);
        out.extend((CHUNK as i64).to_le_bytes());
        out.push(2);
        for _ in 0..2 {
            out.extend((c.len() as i64).to_le_bytes());
            out.extend((p.len() as i64).to_le_bytes());
        }
        out.extend(c);
    }
    Ok(out)
}

/// Splits the file into the layout around, and the unpacked stream of, the current save.
pub fn unpack(data: &[u8]) -> Result<(Layout, Vec<u8>), String> {
    if !data.starts_with(b"GSWU") {
        return Err("not a Subnautica 2 save".into());
    }
    let marker: &[u8] = b"\x0e\x00\x00\x00ContainerInfo\x00";
    let top = data.windows(marker.len()).position(|w| w == marker).ok_or("not a Subnautica 2 save")?;
    let end = data.len();
    // Top level: skip tag by tag to SerializedSaveGames.
    let mut p = top;
    let ss = loop {
        match next_tag(data, p, end).ok_or("couldn't read the save's layout")? {
            Next::End(_) => return Err("no saved games in this file".into()),
            Next::Tag(t) if t.name == "SerializedSaveGames" => break t,
            Next::Tag(t) => p = t.payload + t.size,
        }
    };
    // The first element is the current save.
    let mut q = ss.payload + 4;
    let mut gd = None;
    loop {
        match next_tag(data, q, ss.payload + ss.size).ok_or("couldn't read the save's layout")? {
            Next::End(_) => break,
            Next::Tag(t) => {
                q = t.payload + t.size;
                if t.name == "GameData" {
                    gd = Some(t);
                    break;
                }
            }
        }
    }
    let gd = gd.ok_or("no game data in this save")?;
    let mut r = gd.payload;
    let dt = loop {
        match next_tag(data, r, gd.payload + gd.size).ok_or("couldn't read the save's layout")? {
            Next::End(_) => return Err("no data in this save".into()),
            Next::Tag(t) if t.name == "Data" => break t,
            Next::Tag(t) => r = t.payload + t.size,
        }
    };
    if [ss.flags, gd.flags, dt.flags].iter().any(|f| *f != 0) {
        return Err("unexpected property flags".into());
    }
    let count = i32_at(data, dt.payload).filter(|c| *c >= 0).ok_or("bad data size")? as usize;
    let s = dt.payload + 4;
    let e = s + count;
    if e > data.len() {
        return Err("cut off".into());
    }
    let stream = decode(data, s, e)?;
    let layout = Layout {
        head: data[..s].to_vec(),
        tail: data[e..].to_vec(),
        size_at: [ss.payload - 5, gd.payload - 5, dt.payload - 5],
        count_at: dt.payload,
        old_len: count,
    };
    Ok((layout, stream))
}

pub fn pack(l: &Layout, stream: &[u8]) -> Result<Vec<u8>, String> {
    let blocks = encode(stream)?;
    let delta = blocks.len() as i64 - l.old_len as i64;
    let mut head = l.head.clone();
    for at in l.size_at {
        let old = i32_at(&head, at).ok_or("bad save layout")?;
        put_i32(&mut head, at, i64::from(old) + delta)?;
    }
    put_i32(&mut head, l.count_at, blocks.len() as i64)?;
    head.extend(blocks);
    head.extend(&l.tail);
    Ok(head)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs(s: &str) -> Vec<u8> {
        let mut v = (s.len() as i32 + 1).to_le_bytes().to_vec();
        v.extend(s.as_bytes());
        v.push(0);
        v
    }
    fn ty(n: &str, inner: &[Vec<u8>]) -> Vec<u8> {
        let mut v = fs(n);
        v.extend((inner.len() as i32).to_le_bytes());
        for i in inner {
            v.extend(i);
        }
        v
    }
    fn tg(name: &str, tree: Vec<u8>, payload: &[u8]) -> Vec<u8> {
        let mut v = fs(name);
        v.extend(tree);
        v.extend((payload.len() as i32).to_le_bytes());
        v.push(0);
        v.extend(payload);
        v
    }
    fn none() -> Vec<u8> {
        fs("None")
    }
    fn stream(body: Vec<u8>) -> Vec<u8> {
        let mut s = vec![0u8];
        s.extend(body);
        let mut out = (s.len() as i32).to_le_bytes().to_vec();
        out.extend(s);
        out
    }
    fn item(path: &str, n: i32) -> Vec<u8> {
        let mut e = tg("ItemType", ty("ObjectProperty", &[]), &fs(path));
        e.extend(tg("Count", ty("IntProperty", &[]), &n.to_le_bytes()));
        e.extend(none());
        e
    }
    fn attr(name: &str, v: f32) -> Vec<u8> {
        let mut e = tg("Name", ty("StrProperty", &[]), &fs(name));
        e.extend(tg("BaseValue", ty("FloatProperty", &[]), &v.to_le_bytes()));
        e.extend(tg("CurrentValue", ty("FloatProperty", &[]), &v.to_le_bytes()));
        e.extend(none());
        e
    }

    fn sample() -> Vec<u8> {
        // SaveData: Map<Guid, UWESaveBuffer>, one entry whose Data holds an inventory.
        let mut inner = tg("InventoryClass", ty("ObjectProperty", &[]), &fs("/Game/Blueprints/Character/Player/BP_Character_01.BP_Character_01_C"));
        let mut items = 2i32.to_le_bytes().to_vec();
        items.extend(item("/Game/Data/ItemType/Resource/DA_Titanium_ItemType.DA_Titanium_ItemType", 7));
        items.extend(item("/Game/Data/ItemType/Resource/DA_PlantMatter_ItemType.DA_PlantMatter_ItemType", 3));
        let arr = ty("ArrayProperty", &[ty("StructProperty", &[ty("UWEInventoryItem", &[ty("/Script/UWEInventory", &[])])])]);
        let mut cont = tg("Items", arr, &items);
        cont.extend(none());
        inner.extend(tg("ItemsContainer", ty("StructProperty", &[ty("UWEInventoryContainer", &[ty("/Script/UWEInventory", &[])])]), &cont));
        let mut attrs = 2i32.to_le_bytes().to_vec();
        attrs.extend(attr("Health", 100.0));
        attrs.extend(attr("Food", 80.0));
        let aarr = ty("ArrayProperty", &[ty("StructProperty", &[ty("UWEGameplayAttributeSaveData", &[ty("/Script/UWEAbilitySystem", &[])])])]);
        inner.extend(tg("AttributeSaveData", aarr, &attrs));
        inner.extend(none());
        let mut rec = vec![0u8];
        rec.extend(inner);
        let mut data = (rec.len() as i32).to_le_bytes().to_vec();
        data.extend(&rec);
        let mut buf = tg("Data", ty("ArrayProperty", &[ty("ByteProperty", &[])]), &data);
        buf.extend(none());
        let mut map = 0i32.to_le_bytes().to_vec();
        map.extend(1i32.to_le_bytes());
        map.extend([7u8; 16]);
        map.extend(buf);
        let mt = ty("MapProperty", &[ty("StructProperty", &[ty("Guid", &[ty("/Script/CoreUObject", &[])])]), ty("StructProperty", &[ty("UWESaveBuffer", &[ty("/Script/UWESaveSystem", &[])])])]);
        let mut body = tg("SaveData", mt, &map);
        body.extend(none());
        let mut s = stream(body);
        s.extend(vec![0u8; 4096]);
        let n = s.len() as i32 - 4;
        s[..4].copy_from_slice(&n.to_le_bytes());
        s
    }

    #[test]
    fn finds_items_and_survival_values_in_nested_records() {
        let s = sample();
        let f = scan(&s).unwrap();
        assert_eq!(f.items.len(), 2);
        assert_eq!(f.items[0].name, "Titanium");
        assert_eq!(f.items[0].container, "Player");
        assert_eq!(f.items[1].name, "Plant Matter");
        assert_eq!(f.attrs.len(), 2);
        let mut doc = parse(&s).unwrap();
        assert_eq!(doc.view.pointer("/Player/Titanium"), Some(&serde_json::Value::from(7)));
        doc.set("/Player/Titanium", serde_json::Value::from(500)).unwrap();
        doc.set("/Survival/Food", serde_json::Value::from(99.0)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), s.len());
        let f2 = scan(&out).unwrap();
        assert_eq!(i32::from_le_bytes(out[f2.items[0].at..f2.items[0].at + 4].try_into().unwrap()), 500);
        assert_eq!(f32::from_le_bytes(out[f2.attrs[1].cur..f2.attrs[1].cur + 4].try_into().unwrap()), 99.0);
    }

    #[test]
    fn pretty_names() {
        assert_eq!(pretty("/Game/Data/ItemType/Resource/DA_PlantMatter_ItemType.DA_PlantMatter_ItemType", &["DA_"]), "Plant Matter");
        assert_eq!(pretty("/Game/Blueprints/BaseBuilding/BP_Locker_Wall.BP_Locker_Wall_C", &["BP_"]), "Locker Wall");
        assert_eq!(pretty("/Game/Data/DA_Oxygen_Tank_Medium_Equippable_ItemType.DA_Oxygen_Tank_Medium_Equippable_ItemType", &["DA_"]), "Oxygen Tank Medium Equippable");
    }

    /// Runs against a real save when SN2_SAVE names one (needs an Oodle library); prints what was found.
    #[test]
    fn real_save_roundtrip() {
        let Some(path) = std::env::var_os("SN2_SAVE") else { return };
        let data = std::fs::read(path).unwrap();
        let (l, s) = unpack(&data).unwrap();
        eprintln!("stream {} bytes", s.len());
        let f = scan(&s).unwrap();
        eprintln!("items {} attrs {}", f.items.len(), f.attrs.len());
        for a in &f.attrs {
            if matches!(a.name.as_str(), "Health" | "Oxygen" | "Food" | "Water") {
                let v = |at: usize| f32::from_le_bytes(s[at..at + 4].try_into().unwrap());
                eprintln!("attr {} base {} cur {} at {}", a.name, v(a.base), v(a.cur), a.cur);
            }
        }
        let mut seen: HashMap<String, i32> = HashMap::new();
        for it in &f.items {
            let c = i32::from_le_bytes(s[it.at..it.at + 4].try_into().unwrap());
            *seen.entry(format!("{} | {}", it.container, it.name)).or_default() += c;
        }
        let mut v: Vec<_> = seen.into_iter().collect();
        v.sort();
        for (k, c) in v {
            eprintln!("item {k} = {c}");
        }
        let packed = pack(&l, &s).unwrap();
        let (_, s2) = unpack(&packed).unwrap();
        assert_eq!(s, s2, "stream changed after packing");
        eprintln!("repack ok: {} -> {} bytes", data.len(), packed.len());
        let doc = parse(&s).unwrap();
        for k in ["Survival", "Player", "Storage"] {
            let n = doc.view.get(k).and_then(|v| v.as_object()).map_or(0, |o| o.len());
            eprintln!("view {k}: {n} entries");
        }
        eprintln!("view survival: {}", doc.view.get("Survival").map(|v| v.to_string()).unwrap_or_default());
        eprintln!("view player: {}", doc.view.get("Player").map(|v| v.to_string()).unwrap_or_default());
    }
}
