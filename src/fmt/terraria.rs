//! Terraria player files (`.plr`, after the AES layer is removed), Terraria 1.4+.
//!
//! The format is a packed sequence of fields that depends on the file version. We
//! walk it up to the end of the containers (inventory, piggy bank, safe, forge, void
//! vault) and edit those fields in place; everything after stays untouched.

use crate::tree::{esc, int_in, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

pub(crate) const EMPTY: &str = "(empty)";
pub(crate) const NO_PREFIX: &str = "(none)";
const DIFFICULTY: [&str; 4] = ["Classic", "Mediumcore", "Hardcore", "Journey"];
pub(crate) const MIN_VERSION: i32 = 230; // Terraria 1.4.0

pub(crate) struct Tables {
    items: HashMap<i32, String>,
    item_ids: HashMap<String, i32>,
    pub(crate) item_list: Arc<Vec<String>>,
    /// The same names in item id order.
    pub(crate) item_list_by_id: Arc<Vec<String>>,
    prefixes: Vec<String>,
    pub(crate) prefix_list: Arc<Vec<String>>,
}

pub(crate) fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut items = HashMap::new();
        let mut prefixes = vec![String::new(); 1];
        for line in include_str!("data/terraria.txt").lines() {
            let mut parts = line.splitn(2, '\t');
            let (Some(id), Some(name)) = (parts.next(), parts.next()) else {
                continue;
            };
            if let Some(i) = id.strip_prefix('I').and_then(|n| n.parse().ok()) {
                items.insert(i, name.to_string());
            } else if let Some(i) = id.strip_prefix('P').and_then(|n| n.parse::<usize>().ok()) {
                if prefixes.len() <= i {
                    prefixes.resize(i + 1, String::new());
                }
                prefixes[i] = name.to_string();
            }
        }
        let item_ids = items.iter().map(|(k, v)| (v.to_lowercase(), *k)).collect();
        let mut sorted: Vec<&String> = items.values().collect();
        sorted.sort_by_key(|s| s.to_lowercase());
        let mut item_list = vec![EMPTY.to_string()];
        item_list.extend(sorted.into_iter().cloned());
        let mut ids: Vec<(&i32, &String)> = items.iter().collect();
        ids.sort();
        let mut by_id = vec![EMPTY.to_string()];
        by_id.extend(ids.into_iter().map(|(_, n)| n.clone()));
        let mut prefix_list = vec![NO_PREFIX.to_string()];
        prefix_list.extend(prefixes.iter().filter(|p| !p.is_empty()).cloned());
        Tables {
            items,
            item_ids,
            item_list: Arc::new(item_list),
            item_list_by_id: Arc::new(by_id),
            prefixes,
            prefix_list: Arc::new(prefix_list),
        }
    })
}

pub(crate) fn item_name(id: i32) -> String {
    match id {
        0 => EMPTY.into(),
        _ => tables()
            .items
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("#{id}")),
    }
}

pub(crate) fn item_id(name: &str) -> Option<i32> {
    let n = name.trim();
    if n.is_empty() || n.eq_ignore_ascii_case(EMPTY) {
        return Some(0);
    }
    if let Some(num) = n.strip_prefix('#') {
        return num.parse().ok().filter(|&i| i > 0);
    }
    tables().item_ids.get(&n.to_lowercase()).copied()
}

pub(crate) fn prefix_name(id: u8) -> String {
    match tables().prefixes.get(id as usize) {
        _ if id == 0 => NO_PREFIX.into(),
        Some(p) if !p.is_empty() => p.clone(),
        _ => format!("#{id}"),
    }
}

pub(crate) fn prefix_id(name: &str) -> Option<u8> {
    let n = name.trim();
    if n.eq_ignore_ascii_case(NO_PREFIX) || n.is_empty() {
        return Some(0);
    }
    if let Some(num) = n.strip_prefix('#') {
        return num.parse().ok();
    }
    tables()
        .prefixes
        .iter()
        .position(|p| p.eq_ignore_ascii_case(n))
        .map(|i| i as u8)
}

// ------------------------------------------------------------------ reader

struct Reader<'a> {
    d: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<usize, String> {
        let at = self.pos;
        if at + n > self.d.len() {
            return Err("unexpected end of player file".into());
        }
        self.pos += n;
        Ok(at)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.d[self.take(1)?])
    }
    fn i32(&mut self) -> Result<i32, String> {
        let o = self.take(4)?;
        Ok(i32::from_le_bytes(self.d[o..o + 4].try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, String> {
        let o = self.take(8)?;
        Ok(i64::from_le_bytes(self.d[o..o + 8].try_into().unwrap()))
    }
    /// .NET string: 7-bit encoded length then UTF-8. Returns (start, end, text).
    fn string(&mut self) -> Result<(usize, usize, String), String> {
        let start = self.pos;
        let (mut len, mut shift) = (0usize, 0);
        loop {
            let b = self.u8()?;
            len |= ((b & 0x7F) as usize) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 28 {
                return Err("bad string".into());
            }
        }
        let o = self.take(len)?;
        let s = std::str::from_utf8(&self.d[o..o + len])
            .map_err(|_| "bad string")?
            .to_string();
        Ok((start, self.pos, s))
    }
}

fn net_string(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut n = s.len();
    while n >= 0x80 {
        out.push((n as u8 & 0x7F) | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(s.as_bytes());
    out
}

// ------------------------------------------------------------------ fields

#[derive(Clone, Debug)]
enum Field {
    Name { start: usize, end: usize },
    Difficulty { at: usize },
    Int { at: usize, min: i32, max: i32 },
    Bool { at: usize },
    Item { at: usize },
    Stack { at: usize },
    Prefix { at: usize },
}

#[derive(Default)]
struct Builder {
    fields: Vec<Field>,
    editable: HashMap<String, usize>,
}

impl Builder {
    fn add(&mut self, map: &mut Map<String, Value>, base: &str, key: &str, v: Value, f: Field) {
        self.editable
            .insert(format!("{base}/{}", esc(key)), self.fields.len());
        self.fields.push(f);
        map.insert(key.to_string(), v);
    }

    /// An item slot: id (i32), [stack (i32)], prefix (u8), [favorite (bool)].
    /// `extra`: one more (unknown) byte after the prefix, as equipment slots have since 1.4.5.
    fn slot(
        &mut self,
        r: &mut Reader,
        parent: &mut Map<String, Value>,
        base: &str,
        key: &str,
        stack: bool,
        fav: bool,
        extra: bool,
    ) -> Result<(), String> {
        let ptr = format!("{base}/{}", esc(key));
        let mut m = Map::new();
        let at = r.take(4)?;
        let id = i32::from_le_bytes(r.d[at..at + 4].try_into().unwrap());
        // Sanity checks: a misread layout shows up as nonsense ids/stacks right away.
        if !(0..20_000).contains(&id) {
            return Err("unexpected player layout".into());
        }
        self.add(
            &mut m,
            &ptr,
            "Item",
            Value::from(item_name(id)),
            Field::Item { at },
        );
        if stack {
            let s = r.take(4)?;
            let n = i32::from_le_bytes(r.d[s..s + 4].try_into().unwrap());
            if !(0..=1_000_000).contains(&n) {
                return Err("unexpected player layout".into());
            }
            self.add(
                &mut m,
                &ptr,
                "Stack",
                Value::from(n),
                Field::Stack { at: s },
            );
        }
        let p = r.take(1)?;
        self.add(
            &mut m,
            &ptr,
            "Prefix",
            Value::from(prefix_name(r.d[p])),
            Field::Prefix { at: p },
        );
        if extra {
            r.take(1)?;
        }
        if fav {
            let f = r.take(1)?;
            if r.d[f] > 1 {
                return Err("unexpected player layout".into());
            }
            self.add(
                &mut m,
                &ptr,
                "Favorite",
                Value::Bool(r.d[f] != 0),
                Field::Bool { at: f },
            );
        }
        parent.insert(key.to_string(), Value::Object(m));
        Ok(())
    }

    fn container(
        &mut self,
        r: &mut Reader,
        root: &mut Map<String, Value>,
        title: &str,
        n: usize,
        stack: bool,
        fav: bool,
    ) -> Result<(), String> {
        let base = format!("/{}", esc(title));
        let mut m = Map::new();
        for i in 0..n {
            self.slot(
                r,
                &mut m,
                &base,
                &format!("Slot {}", i + 1),
                stack,
                fav,
                false,
            )?;
        }
        root.insert(title.to_string(), Value::Object(m));
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct PlrBackend {
    raw: Vec<u8>,
    fields: Vec<Field>,
    /// For each slot's Item leaf: pointers of its Stack and Prefix (for empty/new items).
    slot_ptrs: HashMap<usize, (Option<String>, String)>,
}

impl Backend for PlrBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (leaf, v) in changes {
            let (at, bytes) = match &self.fields[*leaf] {
                Field::Name { start, end } => {
                    let s = v.as_str().ok_or("expected text")?;
                    p.add(*start, *end, net_string(s));
                    continue;
                }
                Field::Difficulty { at } => {
                    let i = DIFFICULTY
                        .iter()
                        .position(|d| Some(*d) == v.as_str())
                        .ok_or("unknown difficulty")?;
                    (*at, vec![i as u8])
                }
                Field::Int { at, min, max } => (
                    *at,
                    (int_in(v, *min as i128, *max as i128, "This value")? as i32)
                        .to_le_bytes()
                        .to_vec(),
                ),
                Field::Bool { at } => (*at, vec![u8::from(v.as_bool().ok_or("expected on/off")?)]),
                Field::Item { at } => (
                    *at,
                    item_id(v.as_str().ok_or("expected text")?)
                        .ok_or("unknown item")?
                        .to_le_bytes()
                        .to_vec(),
                ),
                Field::Stack { at } => (
                    *at,
                    (int_in(v, 0, 9999, "Stack")? as i32).to_le_bytes().to_vec(),
                ),
                Field::Prefix { at } => (
                    *at,
                    vec![prefix_id(v.as_str().ok_or("expected text")?).ok_or("unknown prefix")?],
                ),
            };
            p.add(at, at + bytes.len(), bytes);
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let s = || v.as_str().ok_or_else(|| "expected text".to_string());
        match &self.fields[leaf] {
            Field::Name { .. } => match s()?.chars().count() {
                0 => Err("the name can't be empty".into()),
                n if n > 20 => Err("Terraria names can be at most 20 characters".into()),
                _ => Ok(()),
            },
            Field::Difficulty { .. } => DIFFICULTY
                .contains(&s()?)
                .then_some(())
                .ok_or_else(|| "unknown difficulty".into()),
            Field::Int { min, max, .. } => {
                int_in(v, *min as i128, *max as i128, "This value").map(|_| ())
            }
            Field::Bool { .. } => Ok(()),
            Field::Item { .. } => item_id(s()?).map(|_| ()).ok_or_else(|| {
                format!(
                    "there's no item called \u{201C}{}\u{201D}",
                    s().unwrap_or_default()
                )
            }),
            Field::Stack { .. } => int_in(v, 0, 9999, "Stack").map(|_| ()),
            Field::Prefix { .. } => prefix_id(s()?)
                .map(|_| ())
                .ok_or_else(|| "unknown prefix".into()),
        }
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let (Field::Item { .. }, Some((stack, prefix))) =
            (&self.fields[leaf], self.slot_ptrs.get(&leaf))
        else {
            return vec![];
        };
        let item_ptr = format!("{}/Item", prefix.rsplit_once('/').map_or("", |p| p.0));
        let empty = view.pointer(&item_ptr).and_then(Value::as_str) == Some(EMPTY);
        let mut out = Vec::new();
        if let Some(stack) = stack {
            let n = view.pointer(stack).and_then(Value::as_i64).unwrap_or(0);
            if empty {
                out.push((stack.clone(), Value::from(0)));
            } else if n <= 0 {
                out.push((stack.clone(), Value::from(1)));
            }
        }
        if empty {
            out.push((prefix.clone(), Value::from(NO_PREFIX)));
        }
        out
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        match &self.fields[leaf] {
            Field::Item { .. } => Some(tables().item_list.clone()),
            Field::Prefix { .. } => Some(tables().prefix_list.clone()),
            Field::Difficulty { .. } => {
                Some(Arc::new(DIFFICULTY.iter().map(|s| s.to_string()).collect()))
            }
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 24 && &d[4..11] == b"relogic" && d[11] == 3
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Terraria player".into());
    }
    let mut r = Reader { d, pos: 0 };
    let v = r.i32()?;
    if v < MIN_VERSION {
        return Err(format!(
            "player file version {v} is older than Terraria 1.4"
        ));
    }
    r.take(7 + 1 + 4 + 8)?; // signature, file type, revision, favourite flags
    let mut b = Builder::default();
    let mut root = Map::new();
    let mut ch = Map::new();
    let cp = "/Character";
    let (start, end, name) = r.string()?;
    b.add(
        &mut ch,
        cp,
        "Name",
        Value::from(name),
        Field::Name { start, end },
    );
    let at = r.take(1)?;
    let diff = d[at] as usize;
    if diff < DIFFICULTY.len() {
        b.add(
            &mut ch,
            cp,
            "Difficulty",
            Value::from(DIFFICULTY[diff]),
            Field::Difficulty { at },
        );
    }
    let ticks = r.i64()?;
    ch.insert("File version".into(), Value::from(v));
    ch.insert(
        "Play time (hours)".into(),
        Value::from((ticks as f64 / 36e9 * 10.0).round() / 10.0),
    );
    r.i32()?; // hair
    r.u8()?; // hair dye
    if v >= 283 {
        r.u8()?; // team
    }
    r.take(2 + 1 + 1)?; // hidden accessories, hidden pets, skin variant
    let int = |b: &mut Builder,
               r: &mut Reader,
               ch: &mut Map<String, Value>,
               key: &str,
               min: i32,
               max: i32|
     -> Result<(), String> {
        let at = r.take(4)?;
        let n = i32::from_le_bytes(d[at..at + 4].try_into().unwrap());
        b.add(ch, cp, key, Value::from(n), Field::Int { at, min, max });
        Ok(())
    };
    // The game caps max life at 500 and max mana at 200 when it loads a player
    // (current life can be higher thanks to buffs).
    int(&mut b, &mut r, &mut ch, "Life", 0, 2000)?;
    int(&mut b, &mut r, &mut ch, "Max life", 100, 500)?;
    int(&mut b, &mut r, &mut ch, "Mana", 0, 2000)?;
    int(&mut b, &mut r, &mut ch, "Max mana", 20, 200)?;
    let mut upgrades = Map::new();
    let up = "/Permanent upgrades";
    let flag = |b: &mut Builder,
                r: &mut Reader,
                m: &mut Map<String, Value>,
                key: Option<&str>|
     -> Result<(), String> {
        let at = r.take(1)?;
        if let Some(k) = key {
            b.add(m, up, k, Value::Bool(d[at] != 0), Field::Bool { at });
        }
        Ok(())
    };
    flag(
        &mut b,
        &mut r,
        &mut upgrades,
        Some("Extra accessory slot (Demon Heart)"),
    )?;
    flag(&mut b, &mut r, &mut upgrades, Some("Torch God's Favor"))?;
    flag(&mut b, &mut r, &mut upgrades, None)?; // using biome torches (unused copy)
    if v >= 256 {
        flag(&mut b, &mut r, &mut upgrades, Some("Artisan Loaf eaten"))?;
    }
    if v >= 260 {
        for k in [
            "Vital Crystal used",
            "Aegis Fruit used",
            "Arcane Crystal used",
            "Galaxy Pearl used",
            "Gummy Worm used",
            "Ambrosia used",
        ] {
            flag(&mut b, &mut r, &mut upgrades, Some(k))?;
        }
    }
    flag(
        &mut b,
        &mut r,
        &mut upgrades,
        Some("Old One's Army defeated"),
    )?;
    // Terraria 1.4.5 (file version 319+) adds a byte around here; which side of the
    // tax field it's on isn't known, so tax is only editable in older files.
    let new = v > 318;
    if new {
        r.take(5)?;
    } else {
        int(
            &mut b,
            &mut r,
            &mut ch,
            "Tax money collected (copper)",
            0,
            i32::MAX,
        )?;
    }
    if v >= 254 {
        int(&mut b, &mut r, &mut ch, "Deaths (enemies)", 0, i32::MAX)?;
        int(&mut b, &mut r, &mut ch, "Deaths (players)", 0, i32::MAX)?;
    }
    r.take(7 * 3)?; // colours
    root.insert("Character".into(), Value::Object(ch));
    root.insert("Permanent upgrades".into(), Value::Object(upgrades));

    // Equipment: 10 armor/accessory, 10 vanity, 10 dye slots (item id + prefix).
    let mut eq = Map::new();
    let names = [
        "Helmet",
        "Chestplate",
        "Leggings",
        "Accessory 1",
        "Accessory 2",
        "Accessory 3",
        "Accessory 4",
        "Accessory 5",
        "Accessory 6",
        "Accessory 7",
    ];
    for group in ["", "Vanity ", "Dye "] {
        for n in names {
            b.slot(
                &mut r,
                &mut eq,
                "/Equipment",
                &format!("{group}{n}"),
                false,
                false,
                new,
            )?;
        }
    }
    b.container(&mut r, &mut root, "Inventory", 50, true, true)?;
    let mut coins = Map::new();
    for i in 0..4 {
        b.slot(
            &mut r,
            &mut coins,
            "/Coins and ammo",
            &format!("Coin slot {}", i + 1),
            true,
            true,
            false,
        )?;
    }
    for i in 0..4 {
        b.slot(
            &mut r,
            &mut coins,
            "/Coins and ammo",
            &format!("Ammo slot {}", i + 1),
            true,
            true,
            false,
        )?;
    }
    root.insert("Coins and ammo".into(), Value::Object(coins));
    for n in [
        "Pet",
        "Pet dye",
        "Light pet",
        "Light pet dye",
        "Minecart",
        "Minecart dye",
        "Mount",
        "Mount dye",
        "Hook",
        "Hook dye",
    ] {
        b.slot(&mut r, &mut eq, "/Equipment", n, false, false, false)?;
    }
    root.insert("Equipment".into(), Value::Object(eq));
    b.container(&mut r, &mut root, "Piggy bank", 40, true, false)?;
    b.container(&mut r, &mut root, "Safe", 40, true, false)?;
    b.container(&mut r, &mut root, "Defender's Forge", 40, true, false)?;
    b.container(&mut r, &mut root, "Void Vault", 40, true, v >= 255)?;

    let mut slot_ptrs = HashMap::new();
    for (ptr, &leaf) in &b.editable {
        if let (Some(base), Field::Item { .. }) = (ptr.strip_suffix("/Item"), &b.fields[leaf]) {
            let stack = format!("{base}/Stack");
            let stack = b.editable.contains_key(&stack).then_some(stack);
            slot_ptrs.insert(leaf, (stack, format!("{base}/Prefix")));
        }
    }
    let backend = PlrBackend {
        raw: d.to_vec(),
        fields: b.fields,
        slot_ptrs,
    };
    let doc = TreeDoc::new(
        Value::Object(root),
        b.editable,
        "Terraria player",
        Box::new(backend),
    );
    Ok(doc.with_note("Close Terraria (or at least go back to the title screen) before saving, or the game will overwrite your edit."))
}

/// A made-up player file of `version` (1.4.4 to 1.4.4.9) with `items` (inventory slot
/// from 0, item id, stack) and the same in the piggy bank (`bank`).
#[cfg(test)]
pub(crate) fn synthetic(
    version: i32,
    items: &[(usize, i32, i32)],
    bank: &[(usize, i32, i32)],
) -> Vec<u8> {
    assert!((260..=318).contains(&version));
    let mut d = version.to_le_bytes().to_vec();
    d.extend(b"relogic");
    d.push(3);
    d.extend([0u8; 12]);
    d.extend(net_string("Tester"));
    d.push(0); // classic
    d.extend(36_000_000_000i64.to_le_bytes());
    d.extend([0u8; 5]); // hair, hair dye
    d.extend([0u8; 4]);
    for n in [100i32, 100, 20, 20] {
        d.extend(n.to_le_bytes());
    }
    d.extend([0u8; 3 + 1 + 6 + 1]); // upgrade flags
    d.extend(0i32.to_le_bytes()); // tax
    d.extend([0u8; 8]); // deaths
    d.extend([0u8; 21]); // colours
    d.extend(vec![0u8; 30 * 5]); // armor, vanity, dyes
    let slot = |d: &mut Vec<u8>, list: &[(usize, i32, i32)], i: usize, fav: bool| {
        let (id, n) = list
            .iter()
            .find(|s| s.0 == i)
            .map_or((0, 0), |s| (s.1, s.2));
        d.extend(id.to_le_bytes());
        d.extend(n.to_le_bytes());
        d.push(0);
        if fav {
            d.push(0);
        }
    };
    for i in 0..50 {
        slot(&mut d, items, i, true);
    }
    for i in 0..8 {
        slot(&mut d, &[], i, true);
    }
    d.extend(vec![0u8; 10 * 5]); // pet, mount, hook…
    for i in 0..40 {
        slot(&mut d, bank, i, false);
    }
    for _ in 0..2 * 40 {
        slot(&mut d, &[], 0, false);
    }
    for _ in 0..40 {
        slot(&mut d, &[], 0, true);
    }
    d.extend([0u8; 64]); // the rest of the file
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_player_reads() {
        let d = synthetic(279, &[(0, 22, 7)], &[(3, 9, 50)]);
        let doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Character/File version").unwrap(), 279);
        assert_eq!(
            doc.view.pointer("/Inventory/Slot 1/Item").unwrap(),
            "Iron Bar"
        );
        assert_eq!(doc.view.pointer("/Piggy bank/Slot 4/Stack").unwrap(), 50);
        assert_eq!(doc.to_bytes().unwrap(), d);
    }

    #[test]
    #[ignore]
    fn real_player() {
        let dir = std::env::var("TERRARIA_PLAYERS").expect("TERRARIA_PLAYERS");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            if e.path().extension().is_none_or(|x| x != "plr") {
                continue;
            }
            let raw = std::fs::read(e.path()).unwrap();
            let plain = crate::fmt::crypto::terraria_decrypt(&raw).expect("decrypts");
            println!(
                "{} version {}",
                e.path().display(),
                i32::from_le_bytes(plain[0..4].try_into().unwrap())
            );
            let mut doc = parse(&plain).unwrap();
            println!("{}", serde_json::to_string(&doc.view["Character"]).unwrap());
            println!(
                "{}",
                serde_json::to_string(&doc.view["Inventory"]["Slot 1"]).unwrap()
            );
            assert_eq!(doc.to_bytes().unwrap(), plain);
            doc.set("/Character/Name", Value::from("Edited Name"))
                .unwrap();
            doc.set("/Character/Max life", Value::from(500)).unwrap();
            doc.set("/Inventory/Slot 50/Item", Value::from("Zenith"))
                .unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out).unwrap();
            assert_eq!(again.view, doc.view);
            assert_eq!(again.view.pointer("/Inventory/Slot 50/Stack").unwrap(), 1);
            assert_eq!(
                crate::fmt::crypto::terraria_decrypt(&crate::fmt::crypto::terraria_encrypt(&out))
                    .unwrap(),
                out
            );
        }
    }
}
