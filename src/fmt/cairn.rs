//! Cairn (The Game Bakers) saves, after the pack layer (and gzip, for most saves). A save
//! starts with the 💿 marker. Its run statistics follow the name of the current route
//! fragment (`FRAG_<ticks>_<GUID>`) and a few fields that newer formats added: play time
//! in seconds, three more 64-bit totals (the last can be negative), then 32-bit counters:
//! deaths, then falls. Worked out from sample saves against the game's
//! own slot cards (play time to the second, falls, deaths; deaths stay 0 in Free solo).

pub const MARKER: [u8; 4] = [0xF0, 0x9F, 0x92, 0xBF];

/// Where the statistics start (the play time), if this is a Cairn save.
pub fn stats_at(d: &[u8]) -> Option<usize> {
    if !d.starts_with(&MARKER) {
        return None;
    }
    let i64_at = |o: usize| d.get(o..o + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()));
    let is_guid = |s: &[u8]| {
        s.len() == 36
            && s.iter().enumerate().all(|(i, &c)| if [8, 13, 18, 23].contains(&i) { c == b'-' } else { c.is_ascii_hexdigit() })
    };
    let ok = |at: usize| {
        let (t, a, b, c) = (i64_at(at), i64_at(at + 8), i64_at(at + 16), i64_at(at + 24));
        matches!((t, a, b, c), (Some(t), Some(a), Some(b), Some(c)) if t > 0 && t < 100_000_000 && a <= t && b >> 32 == 0 && (c as i64).unsigned_abs() >> 32 == 0)
    };
    // How many of the four totals have a non-zero low byte: read one byte off, the next
    // total's zero high byte lands in the low byte instead.
    let score = |at: usize| (0..4).filter(|k| d.get(at + 8 * k).is_some_and(|&b| b != 0)).count();
    // A few fields come between the name and the totals, more in newer versions (none in
    // format 24, one byte in 29, 14 bytes in 41): try each gap and keep the best fit.
    (36..d.len().saturating_sub(48)).filter(|&p| is_guid(&d[p - 36..p])).find_map(|p| {
        (0..=32).map(|g| p + g).filter(|&at| ok(at)).max_by_key(|&at| (score(at), std::cmp::Reverse(at)))
    })
}

/// (label, offset from the start of the statistics, byte width)
pub const FIELDS: [(&str, usize, usize); 3] = [("Play time (seconds)", 0, 4), ("Deaths", 32, 4), ("Falls", 36, 4)];

// ------------------------------------------------------------------ the backpack

use crate::tree::{esc, int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

const NONE: &str = "(none)";
/// Empty rows offered under each list, to add items.
const SPARE: [usize; 3] = [8, 1, 4];
pub const LISTS: [&str; 3] = ["Backpack", "Chalk pouch", "Piton belt"];
/// Piton and crystal piton: they carry uses.
const PITONS: [u32; 2] = [3, 112];

struct Items {
    by_id: HashMap<u32, String>,
    by_name: HashMap<String, u32>,
    list: Arc<Vec<String>>,
}

fn items() -> &'static Items {
    static T: OnceLock<Items> = OnceLock::new();
    T.get_or_init(|| {
        let mut by_id = HashMap::new();
        for l in include_str!("data/cairn_items.tsv").lines().filter(|l| !l.starts_with('#')) {
            let mut f = l.split('\t');
            let (Some(id), Some(code)) = (f.next(), f.next()) else { continue };
            let Ok(id) = id.parse::<u32>() else { continue };
            // The English name the game shows, else the code name made readable.
            let name = f.next().map(str::trim).filter(|n| !n.is_empty()).map_or_else(|| pretty(code.trim().trim_start_matches("ITEM_")), str::to_string);
            by_id.insert(id, name);
        }
        // Names must be unique to be picked from a list: add the id to any repeat.
        let mut ids: Vec<u32> = by_id.keys().copied().collect();
        ids.sort();
        let mut seen = std::collections::HashSet::new();
        for id in &ids {
            let n = by_id[id].clone();
            if !seen.insert(n.clone()) {
                by_id.insert(*id, format!("{n} (#{id})"));
            }
        }
        let by_name = by_id.iter().map(|(k, v)| (v.clone(), *k)).collect();
        let mut list = vec![NONE.to_string()];
        list.extend(ids.iter().map(|i| by_id[i].clone()));
        Items { by_id, by_name, list: Arc::new(list) }
    })
}

/// "CLIMBING_TAPE" -> "Climbing Tape"
fn pretty(code: &str) -> String {
    code.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let w = w.to_lowercase();
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + c.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn item_name(id: u32) -> String {
    match id {
        0 => NONE.into(),
        _ => items().by_id.get(&id).cloned().unwrap_or_else(|| format!("#{id}")),
    }
}

fn item_id(name: &str) -> Option<u32> {
    let n = name.trim();
    if n.is_empty() || n == NONE {
        return Some(0);
    }
    if let Some(num) = n.strip_prefix('#') {
        return num.parse().ok();
    }
    items().by_name.get(n).copied()
}

/// One carried item. Backpack items also have a place in the bag (x, y and a turn in degrees).
#[derive(Clone, Debug, PartialEq)]
struct Entry {
    pos: Option<[f32; 3]>,
    item: u32,
    /// What a bottle or flask holds (0: nothing).
    content: u32,
    /// How many (uses for chalk, sips for drinks).
    count: u32,
    /// Pitons: 5 more bytes (their uses).
    extra: Option<[u8; 5]>,
    b: u8,
    uid: u16,
    t: f32,
    e: u8,
}

fn read_entry(d: &[u8], p: &mut usize, placed: bool) -> Option<Entry> {
    let f32_at = |o: usize| d.get(o..o + 4).map(|b| f32::from_le_bytes(b.try_into().unwrap()));
    let u32_at = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let pos = if placed {
        let v = [f32_at(*p)?, f32_at(*p + 4)?, f32_at(*p + 8)?];
        if !v.iter().all(|x| x.is_finite()) || !(-5.0..40.0).contains(&v[0]) || !(-5.0..40.0).contains(&v[1]) || !(-720.0..=720.0).contains(&v[2]) {
            return None;
        }
        *p += 12;
        Some(v)
    } else {
        None
    };
    let (item, content, count) = (u32_at(*p)?, u32_at(*p + 4)?, u32_at(*p + 8)?);
    let known = |i: u32| items().by_id.contains_key(&i);
    if !known(item) || (content != 0 && !known(content)) || count >= 100_000 {
        return None;
    }
    *p += 12;
    let a = *d.get(*p)?;
    *p += 1;
    let extra = match a {
        0 => None,
        1 => {
            let e: [u8; 5] = d.get(*p..*p + 5)?.try_into().ok()?;
            *p += 5;
            Some(e)
        }
        _ => return None,
    };
    let b = *d.get(*p)?;
    let uid = u16::from_le_bytes(d.get(*p + 1..*p + 3)?.try_into().ok()?);
    let t = f32_at(*p + 3)?;
    let e = *d.get(*p + 7)?;
    if b > 1 || e > 1 || !t.is_finite() {
        return None;
    }
    *p += 8;
    Some(Entry { pos, item, content, count, extra, b, uid, t, e })
}

fn write_entry(x: &Entry, out: &mut Vec<u8>) {
    if let Some(v) = x.pos {
        for f in v {
            out.extend_from_slice(&f.to_le_bytes());
        }
    }
    for n in [x.item, x.content, x.count] {
        out.extend_from_slice(&n.to_le_bytes());
    }
    match x.extra {
        Some(e) => {
            out.push(1);
            out.extend_from_slice(&e);
        }
        None => out.push(0),
    }
    out.push(x.b);
    out.extend_from_slice(&x.uid.to_le_bytes());
    out.extend_from_slice(&x.t.to_le_bytes());
    out.push(x.e);
}

fn read_list(d: &[u8], p: &mut usize, placed: bool) -> Option<Vec<Entry>> {
    let n = u16::from_le_bytes(d.get(*p..*p + 2)?.try_into().ok()?) as usize;
    if n > 300 {
        return None;
    }
    *p += 2;
    (0..n).map(|_| read_entry(d, p, placed)).collect()
}

/// The backpack (items placed in the bag), a flag byte, the chalk pouch and the piton belt.
#[derive(Clone, Debug)]
struct Lists {
    start: usize,
    end: usize,
    sep: u8,
    lists: [Vec<Entry>; 3],
}

fn find_lists(d: &[u8]) -> Option<Lists> {
    if !d.starts_with(&MARKER) {
        return None;
    }
    (8..d.len().min(4096).saturating_sub(4)).find_map(|start| {
        let n = u16::from_le_bytes(d.get(start..start + 2)?.try_into().ok()?);
        if n == 0 {
            return None;
        }
        let mut p = start;
        let a = read_list(d, &mut p, true)?;
        // 0 to 2 in the saves seen (2 with two chalk pouches in an On the Trail run).
        let sep = *d.get(p)?;
        if sep > 16 {
            return None;
        }
        p += 1;
        let b = read_list(d, &mut p, false)?;
        let c = read_list(d, &mut p, false)?;
        Some(Lists { start, end: p, sep, lists: [a, b, c] })
    })
}

fn write_lists(l: &Lists) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, list) in l.lists.iter().enumerate() {
        out.extend_from_slice(&(list.len() as u16).to_le_bytes());
        for x in list {
            write_entry(x, &mut out);
        }
        if i == 0 {
            out.push(l.sep);
        }
    }
    out
}

/// The climbers, by the game's character number (1 and 2).
const CLIMBERS: [&str; 2] = ["Aava", "Marco"];

/// Where the save says who climbs: a u32 (1 Aava, 2 Marco) after the run's 16-byte id, two
/// flag bytes and a 4-byte hash. The first such spot from the start is the one (checked
/// against the run id in every save of every slot, Story and On the Trail).
fn climber_at(d: &[u8], before: usize) -> Option<usize> {
    (24..before.min(200).saturating_sub(4)).find(|&k| {
        let v = u32::from_le_bytes(d[k..k + 4].try_into().unwrap());
        let distinct = d[k - 22..k - 6].iter().collect::<std::collections::HashSet<_>>().len();
        (1..=2).contains(&v) && d[k - 6] <= 1 && d[k - 5] <= 1 && distinct >= 12
    })
}

#[derive(Clone, Copy, Debug)]
enum Field {
    Stat { off: usize },
    Climber { at: usize },
    Item,
    Content,
    Count,
}

#[derive(Clone, Debug)]
struct CairnBackend {
    raw: Arc<Vec<u8>>,
    lists: Lists,
    fields: Vec<Field>,
    /// Count and contents pointers for each Item leaf (for follow-up values).
    rows: HashMap<usize, (String, String)>,
}

impl CairnBackend {
    /// A place in the bag for an added item: spread over the bottom; "Shake up bag" settles them.
    fn spot(k: usize) -> [f32; 3] {
        [1.5 + (k % 6) as f32 * 2.0, 0.6 + (k / 6 % 4) as f32 * 1.1, 0.0]
    }

    /// Uses block for a piton: as on the player's other pitons, else 10.
    fn extra_for(&self, item: u32) -> Option<[u8; 5]> {
        if !PITONS.contains(&item) {
            return None;
        }
        self.lists.lists.iter().flatten().find(|x| x.item == item).and_then(|x| x.extra).or(Some([0, 0, 10, 0, 0]))
    }
}

impl Backend for CairnBackend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = self.raw.to_vec();
        // The climber comes before the lists, so its place doesn't move.
        for (leaf, v) in changes {
            if let Field::Climber { at } = self.fields[*leaf] {
                let i = CLIMBERS.iter().position(|c| Some(*c) == v.as_str()).ok_or("unknown climber")?;
                out[at..at + 4].copy_from_slice(&(i as u32 + 1).to_le_bytes());
            }
        }
        if changes.iter().any(|(l, _)| matches!(self.fields[*l], Field::Item | Field::Content | Field::Count)) {
            // Rebuild the three lists from the rows as they are now.
            let mut lists = self.lists.clone();
            let mut next_uid = lists.lists.iter().flatten().map(|x| x.uid).max().unwrap_or(0);
            let mut added = 0;
            for (li, name) in LISTS.iter().enumerate() {
                let rows = view.pointer(&format!("/{}", esc(name))).and_then(Value::as_object).ok_or("missing list")?;
                let mut new = Vec::new();
                for (ri, row) in rows.values().enumerate() {
                    let item = item_id(row.get("Item").and_then(Value::as_str).unwrap_or(NONE)).ok_or("unknown item")?;
                    if item == 0 {
                        continue;
                    }
                    let content = item_id(row.get("Contents").and_then(Value::as_str).unwrap_or(NONE)).ok_or("unknown contents")?;
                    let count = row.get("Count").and_then(Value::as_u64).unwrap_or(1).min(99_999) as u32;
                    let mut x = match self.lists.lists[li].get(ri) {
                        Some(old) => old.clone(),
                        None => {
                            next_uid += 1;
                            added += 1;
                            Entry { pos: (li == 0).then(|| Self::spot(added - 1)), item, content: 0, count: 1, extra: self.extra_for(item), b: 0, uid: next_uid, t: 0.0, e: 0 }
                        }
                    };
                    if x.item != item {
                        x.item = item;
                        x.extra = self.extra_for(item);
                    }
                    x.content = content;
                    x.count = count;
                    new.push(x);
                }
                lists.lists[li] = new;
            }
            out.splice(self.lists.start..self.lists.end, write_lists(&lists));
        }
        // The statistics come after the lists: find them again in the new bytes.
        if let Some(at) = stats_at(&out) {
            for (leaf, v) in changes {
                if let Field::Stat { off } = self.fields[*leaf] {
                    let n = int_in(v, 0, 99_999_999, "This value")? as u32;
                    out[at + off..at + off + 4].copy_from_slice(&n.to_le_bytes());
                }
            }
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match self.fields[leaf] {
            Field::Stat { .. } => int_in(v, 0, 99_999_999, "This value").map(|_| ()),
            Field::Climber { .. } => CLIMBERS.contains(&v.as_str().unwrap_or("")).then_some(()).ok_or_else(|| "pick Aava or Marco".into()),
            Field::Item | Field::Content => {
                let s = v.as_str().ok_or("expected text")?;
                item_id(s).map(|_| ()).ok_or_else(|| format!("there's no item called \u{201C}{s}\u{201D}"))
            }
            Field::Count => int_in(v, 0, 99_999, "Count").map(|_| ()),
        }
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let (Field::Item, Some((count, content))) = (self.fields[leaf], self.rows.get(&leaf)) else { return vec![] };
        let base = count.rsplit_once('/').map_or("", |p| p.0);
        let item = view.pointer(&format!("{base}/Item")).and_then(Value::as_str).unwrap_or(NONE);
        let n = view.pointer(count).and_then(Value::as_i64).unwrap_or(0);
        if item == NONE {
            vec![(count.clone(), Value::from(0)), (content.clone(), Value::from(NONE))]
        } else if n <= 0 {
            vec![(count.clone(), Value::from(1))]
        } else {
            vec![]
        }
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        match self.fields[leaf] {
            Field::Item | Field::Content => Some(items().list.clone()),
            Field::Climber { .. } => Some(Arc::new(CLIMBERS.iter().map(|c| c.to_string()).collect())),
            _ => None,
        }
    }

    fn reshapes(&self, leaf: usize) -> bool {
        matches!(self.fields[leaf], Field::Item | Field::Content | Field::Count)
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// A Cairn save as values: the run statistics, backpack, chalk pouch and piton belt.
pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let lists = find_lists(d).ok_or("no backpack found in this save")?;
    let mut fields = Vec::new();
    let mut editable = HashMap::new();
    let mut rows = HashMap::new();
    let mut root = Map::new();
    let mut add = |fields: &mut Vec<Field>, map: &mut Map<String, Value>, ptr: String, key: &str, v: Value, f: Field| {
        editable.insert(ptr, fields.len());
        fields.push(f);
        map.insert(key.to_string(), v);
    };
    let mut run = Map::new();
    if let Some(at) = climber_at(d, lists.start) {
        let n = u32::from_le_bytes(d[at..at + 4].try_into().unwrap()) as usize;
        add(&mut fields, &mut run, "/Run/Climber".into(), "Climber", Value::from(CLIMBERS[n - 1]), Field::Climber { at });
    }
    if let Some(at) = stats_at(d) {
        for (label, off, _) in FIELDS {
            let n = u32::from_le_bytes(d[at + off..at + off + 4].try_into().unwrap());
            add(&mut fields, &mut run, format!("/Run/{}", esc(label)), label, Value::from(n), Field::Stat { off });
        }
    }
    if !run.is_empty() {
        root.insert("Run".into(), Value::Object(run));
    }
    for (li, name) in LISTS.iter().enumerate() {
        let mut m = Map::new();
        let list = &lists.lists[li];
        for row in 0..list.len() + SPARE[li] {
            let key = format!("{} {}", if li == 2 { "Piton" } else { "Item" }, row + 1);
            let base = format!("/{}/{}", esc(name), esc(&key));
            let x = list.get(row);
            let mut r = Map::new();
            let item_leaf = fields.len();
            add(&mut fields, &mut r, format!("{base}/Item"), "Item", Value::from(x.map_or(NONE.to_string(), |x| item_name(x.item))), Field::Item);
            add(&mut fields, &mut r, format!("{base}/Contents"), "Contents", Value::from(x.map_or(NONE.to_string(), |x| item_name(x.content))), Field::Content);
            add(&mut fields, &mut r, format!("{base}/Count"), "Count", Value::from(x.map_or(0, |x| x.count)), Field::Count);
            rows.insert(item_leaf, (format!("{base}/Count"), format!("{base}/Contents")));
            m.insert(key, Value::Object(r));
        }
        root.insert(name.to_string(), Value::Object(m));
    }
    let backend = CairnBackend { raw: Arc::new(d.to_vec()), lists, fields, rows };
    Ok(TreeDoc::new(Value::Object(root), editable, "Cairn save", Box::new(backend))
        .with_note("Close Cairn (or go back to its title screen) before saving: the game keeps the slot file locked while a climb is loaded. Added items go at the bottom of the bag; \u{201C}Shake up bag\u{201D} in the game settles them."))
}

#[cfg(test)]
mod backpack {
    /// SAVE_EDITOR_CAIRN_DIR = a folder of SLOT_x.CPK packs (copies): every save reads, writes
    /// back unchanged, and takes an edit and an added item.
    #[test]
    #[ignore]
    fn every_backpack() {
        use crate::formats::{decode_with, Content, Options};
        for e in std::fs::read_dir(std::env::var("SAVE_EDITOR_CAIRN_DIR").unwrap()).unwrap() {
            let p = e.unwrap().path();
            let raw = std::fs::read(&p).unwrap();
            let first = decode_with(&raw, &Options::default());
            let mut n = 0;
            for name in first.zip_entries.iter().filter(|n| n.ends_with(".SAV")) {
                let dec = decode_with(&raw, &Options { zip_entry: Some(name.clone()), ..Options::default() });
                let Content::Tree(t) = &dec.content else { panic!("{name}: not read as a Cairn save") };
                let mut t = (**t).clone();
                let bytes = t.to_bytes().unwrap();
                let back = super::parse(&bytes).unwrap();
                assert_eq!(back.view, t.view, "{name}");
                t.set("/Backpack/Item 1/Count", serde_json::Value::from(7)).unwrap();
                let rows = t.view["Backpack"].as_object().unwrap().len();
                t.set(&format!("/Backpack/Item {rows}/Item"), serde_json::Value::from("Protein bar")).unwrap();
                let out = t.to_bytes().unwrap();
                let again = super::parse(&out).unwrap();
                let items: Vec<&str> = again.view["Backpack"].as_object().unwrap().values().filter_map(|r| r["Item"].as_str()).collect();
                assert!(items.contains(&"Protein bar"), "{name}: added item missing");
                assert_eq!(again.view["Backpack"]["Item 1"]["Count"], 7);
                assert_eq!(again.view.get("Run"), t.view.get("Run"), "{name}");
                let who = t.view["Run"]["Climber"].as_str().unwrap().to_string();
                let other = if who == "Aava" { "Marco" } else { "Aava" };
                t.set("/Run/Climber", serde_json::Value::from(other)).unwrap();
                let swapped = super::parse(&t.to_bytes().unwrap()).unwrap();
                assert_eq!(swapped.view["Run"]["Climber"], other, "{name}");
                n += 1;
            }
            println!("{}: {n} saves", p.display());
        }
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(&MARKER)
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_the_statistics() {
        let mut d = super::MARKER.to_vec();
        d.push(29);
        d.extend([0u8; 19]);
        d.extend(b"3d2a4a96-8d2b-4b11-ace3-0a69b2f1c2d3\0");
        let at = d.len();
        for v in [2916u64, 2607, 35337, 71917] {
            d.extend(v.to_le_bytes());
        }
        d.extend([0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0]);
        d.extend([0u8; 64]);
        assert_eq!(super::stats_at(&d), Some(at));
    }
}

#[cfg(test)]
mod real {
    /// Research aid: SAVE_EDITOR_CAIRN = an unpacked Cairn save.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_CAIRN").unwrap()).unwrap();
        let at = super::stats_at(&d).expect("statistics");
        for (l, o, _) in super::FIELDS {
            println!("{l}: {}", u32::from_le_bytes(d[at + o..at + o + 4].try_into().unwrap()));
        }
    }
}

#[cfg(test)]
mod real_pack {
    /// Research aid: SAVE_EDITOR_CAIRN_PACK = a SLOT_x.CPK; sets Falls to 3 and reads it back.
    #[test]
    #[ignore]
    fn edit_falls() {
        use crate::formats::{decode, encode, Content};
        let raw = std::fs::read(std::env::var("SAVE_EDITOR_CAIRN_PACK").unwrap()).unwrap();
        let mut dec = decode(&raw);
        let Content::Binary(d) = &mut dec.content else { panic!("not binary") };
        let at = super::stats_at(d).unwrap() + 36;
        d[at..at + 4].copy_from_slice(&3u32.to_le_bytes());
        let out = encode(&dec.layers, &dec.content).unwrap();
        assert_eq!(out.len(), raw.len());
        let back = decode(&out);
        let Content::Binary(b) = &back.content else { panic!() };
        let at = super::stats_at(b).unwrap() + 36;
        assert_eq!(u32::from_le_bytes(b[at..at + 4].try_into().unwrap()), 3);
        println!("falls now 3; pack still {} bytes", out.len());
    }
}

#[cfg(test)]
mod every_save {
    /// Research aid: SAVE_EDITOR_CAIRN_DIR = a folder of SLOT_x.CPK packs; reads every save in them.
    #[test]
    #[ignore]
    fn all() {
        use crate::formats::{decode_with, Content, Options};
        for e in std::fs::read_dir(std::env::var("SAVE_EDITOR_CAIRN_DIR").unwrap()).unwrap() {
            let p = e.unwrap().path();
            let raw = std::fs::read(&p).unwrap();
            let first = decode_with(&raw, &Options::default());
            let (mut ok, mut bad) = (0, vec![]);
            for name in first.zip_entries.iter().filter(|n| n.ends_with(".SAV")) {
                let opts = Options { zip_entry: Some(name.clone()), ..Options::default() };
                let dec = decode_with(&raw, &opts);
                match &dec.content {
                    Content::Binary(d) => match super::stats_at(d) {
                        Some(at) if name.ends_with("QUICK.SAV") || true => {
                            ok += 1;
                            if name.ends_with("QUICK.SAV") || name == first.zip_entries.iter().filter(|n| n.ends_with(".SAV")).last().unwrap() {
                                println!("  {name}: {} s, deaths {}, falls {}", u32::from_le_bytes(d[at..at + 4].try_into().unwrap()), d[at + 32], d[at + 36]);
                            }
                        }
                        _ => bad.push(format!("{name} (format {})", d.get(4).copied().unwrap_or(0))),
                    },
                    _ => bad.push(format!("{name} (not binary)")),
                }
            }
            println!("{}: {ok} read, missing {:?}", p.display(), bad);
        }
    }
}

#[cfg(test)]
mod one_save {
    /// Research aid: SAVE_EDITOR_CAIRN = an unpacked Cairn save; prints its lists.
    #[test]
    #[ignore]
    fn lists() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_CAIRN").unwrap()).unwrap();
        match super::find_lists(&d) {
            Some(l) => println!("at {} to {}: {:?}", l.start, l.end, l.lists.iter().map(|x| x.len()).collect::<Vec<_>>()),
            None => println!("no lists"),
        }
        println!("{:?}", super::parse(&d).map(|t| t.view["Backpack"]["Item 1"].clone()));
    }
}
