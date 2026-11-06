//! Borderlands 3 characters (`<n>.sav`, PC).
//!
//! An Unreal "GVAS" header (versions, build, custom format GUIDs, the save type
//! `OakSaveGame`), the payload's length, then the payload: a protobuf message
//! (`OakSave.Character`) under a rolling XOR. Each byte is XORed with a fixed 32-byte key
//! and with the (encrypted) byte 32 places before it, or a fixed prefix for the first 32.
//! There's no checksum. An edit rewrites the fields it touches, then the payload is
//! encrypted again and its length updated; the header is copied as it was.
//!
//! Keys, header layout, currency hashes, the experience table, SDU limits and item flags
//! from apocalyptech's bl3-cli-saveedit (zlib); field numbers from the published
//! OakSave protobuf schema (gibbed's Borderlands3Protos, facts only).

use super::borderlands::pb;
use crate::tree::{Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

const PREFIX: [u8; 32] = [
    0x71, 0x34, 0x36, 0xB3, 0x56, 0x63, 0x25, 0x5F, 0xEA, 0xE2, 0x83, 0x73, 0xF4, 0x98, 0xB8, 0x18, 0x2E, 0xE5, 0x42, 0x2E, 0x50, 0xA2, 0x0F, 0x49, 0x87, 0x24, 0xE6, 0x65, 0x9A, 0xF0, 0x7C, 0xD7,
];
const XOR: [u8; 32] = [
    0x7C, 0x07, 0x69, 0x83, 0x31, 0x7E, 0x0C, 0x82, 0x5F, 0x2E, 0x36, 0x7F, 0x76, 0xB4, 0xA2, 0x71, 0x38, 0x2B, 0x6E, 0x87, 0x39, 0x05, 0x02, 0xC6, 0xCD, 0xD8, 0xB1, 0xCC, 0xA1, 0x33, 0xF9, 0xB6,
];

const MONEY: u64 = 618_814_354;
const ERIDIUM: u64 = 3_679_636_065;
pub const MAX_LEVEL: i64 = 72;

/// Experience needed for each level (1 to 72).
const XP: [u64; 72] = [
    0, 358, 1241, 2850, 5376, 8997, 13886, 20208, 28126, 37798, 49377, 63016, 78861, 97061, 117757, 141092, 167206, 196238, 228322, 263595, 302190, 344238, 389873, 439222,
    492414, 549578, 610840, 676325, 746158, 820463, 899363, 982980, 1071435, 1164850, 1263343, 1367034, 1476041, 1590483, 1710476, 1836137, 1967582, 2104926, 2248285,
    2397772, 2553501, 2715586, 2884139, 3059273, 3241098, 3429728, 3625271, 3827840, 4037543, 4254491, 4478792, 4710556, 4949890, 5196902, 5451701, 5714393, 5985086,
    6263885, 6550897, 6846227, 7149982, 7462266, 7783184, 8112840, 8451340, 8798786, 9155282, 9520932,
];

pub fn level_for(xp: u64) -> i64 {
    XP.iter().rposition(|&need| xp >= need).unwrap_or(0) as i64 + 1
}

/// SDU object → (name, highest level).
const SDUS: [(&str, &str, i64); 8] = [
    ("SDU_Backpack", "Backpack", 13),
    ("SDU_AssaultRifle", "Assault rifle", 10),
    ("SDU_Pistol", "Pistol", 10),
    ("SDU_SniperRifle", "Sniper rifle", 13),
    ("SDU_Shotgun", "Shotgun", 10),
    ("SDU_Grenade", "Grenades", 10),
    ("SDU_SMG", "SMG", 10),
    ("SDU_Heavy", "Heavy weapons", 13),
];

const AMMO: [(&str, &str); 7] = [
    ("Resource_Ammo_AssaultRifle", "Assault rifle"),
    ("Resource_Ammo_Pistol", "Pistol"),
    ("Resource_Ammo_SniperRifle", "Sniper rifle"),
    ("Resource_Ammo_Sniper", "Sniper rifle"),
    ("Resource_Ammo_Shotgun", "Shotgun"),
    ("Resource_Ammo_Grenade", "Grenades"),
    ("Resource_Ammo_SMG", "SMG"),
];

const CLASSES: [(&str, &str, &str); 4] = [
    ("PlayerClassId_Siren", "Siren", "Amara"),
    ("PlayerClassId_Beastmaster", "Beastmaster", "FL4K"),
    ("PlayerClassId_Gunner", "Gunner", "Moze"),
    ("PlayerClassId_Operative", "Operative", "Zane"),
];

fn object(path: &str) -> &str {
    path.rsplit(['.', '/']).next().unwrap_or(path)
}

fn ammo_name(path: &str) -> String {
    let o = object(path);
    match AMMO.iter().find(|a| a.0 == o) {
        Some(a) => a.1.to_string(),
        None if o == "Resource_Ammo_Heavy" => "Heavy weapons".into(),
        None => o.trim_start_matches("Resource_Ammo_").replace('_', " "),
    }
}

/// Equipped slot object → name ("BPInvSlot_Weapon1" → "Weapon 1").
fn slot_name(path: &str) -> String {
    let o = object(path).trim_start_matches("BPInvSlot_");
    match o {
        "GrenadeMod" => "Grenade mod".into(),
        "ClassMod" => "Class mod".into(),
        _ => match o.strip_prefix("Weapon") {
            Some(n) if !n.is_empty() => format!("Weapon {n}"),
            _ => o.to_string(),
        },
    }
}

// ------------------------------------------------------------------ file layout

struct Layout {
    /// Where the payload length is; the payload follows it to the end of the file.
    len_at: usize,
}

fn layout(d: &[u8]) -> Option<Layout> {
    if !d.starts_with(b"GVAS") {
        return None;
    }
    let u32_at = |o: usize| d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let fstring = |o: usize| -> Option<(String, usize)> {
        let n = i32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?);
        if !(0..=1024).contains(&n) {
            return None;
        }
        let n = n as usize;
        let s = d.get(o + 4..o + 4 + n)?;
        Some((String::from_utf8_lossy(s.strip_suffix(&[0]).unwrap_or(s)).into_owned(), o + 4 + n))
    };
    if u32_at(4)? != 2 {
        return None;
    }
    // Package version, engine version (3 × u16 + build), build id.
    let (_, at) = fstring(4 + 4 + 4 + 6 + 4)?;
    let count = u32_at(at + 4)? as usize;
    if count > 4096 {
        return None;
    }
    let (kind, at) = fstring(at + 8 + count * 20)?;
    if kind != "OakSaveGame" {
        return None;
    }
    let len = u32_at(at)? as usize;
    (at + 4 + len == d.len()).then_some(Layout { len_at: at })
}

pub fn looks(d: &[u8]) -> bool {
    layout(d).is_some()
}

fn decrypt(p: &mut [u8]) {
    for i in (0..p.len()).rev() {
        let k = if i < 32 { PREFIX[i] } else { p[i - 32] };
        p[i] ^= k ^ XOR[i % 32];
    }
}

fn encrypt(p: &mut [u8]) {
    for i in 0..p.len() {
        let k = if i < 32 { PREFIX[i] } else { p[i - 32] };
        p[i] ^= k ^ XOR[i % 32];
    }
}

/// An item serial: is its check right, and the serial format's version.
pub fn serial_ok(s: &[u8]) -> Option<(bool, u8)> {
    if s.len() < 8 {
        return None;
    }
    let seed = i32::from_be_bytes(s[1..5].try_into().ok()?);
    let mut t = s[5..].to_vec();
    if seed != 0 {
        let mut x = (seed >> 5) as u32 as u64;
        for b in t.iter_mut() {
            x = (x * 0x10A8_60C1) % 0xFFFF_FFFB;
            *b ^= x as u8;
        }
        let r = (seed as u32 & 31) as usize % t.len();
        t.rotate_right(r);
    }
    let mut h = flate2::Crc::new();
    h.update(&s[..5]);
    h.update(&[0xFF, 0xFF]);
    h.update(&t[2..]);
    let c = h.sum();
    Some((((c >> 16) ^ c) as u16 == u16::from_be_bytes([t[0], t[1]]), s[0]))
}

// ------------------------------------------------------------------ editable document

#[derive(Clone, Debug)]
enum Target {
    Experience,
    /// Shown from the experience; editing it sets the experience.
    Level,
    SkillPoints,
    /// A currency by its hash.
    Currency(u64),
    /// Ammo pool: index of the field-5 entry.
    Ammo(usize),
    /// SDU: index of the field-29 entry, highest level.
    Sdu(usize, i64),
}

#[derive(Clone, Debug)]
struct Bl3Backend {
    file: Arc<Vec<u8>>,
    proto: Arc<Vec<u8>>,
    len_at: usize,
    targets: Vec<Target>,
}

fn whole(v: &Value) -> Result<i64, String> {
    v.as_i64().or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64)).ok_or_else(|| "expected a whole number".to_string())
}

impl Bl3Backend {
    fn range(&self, t: &Target) -> (i64, i64) {
        match t {
            Target::Level => (1, MAX_LEVEL),
            Target::Experience | Target::SkillPoints | Target::Currency(_) => (0, i64::from(i32::MAX)),
            Target::Ammo(_) => (0, 99_999),
            Target::Sdu(_, max) => (0, *max),
        }
    }
}

impl Backend for Bl3Backend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.file.to_vec());
        }
        let proto = self.proto.as_slice();
        let top = pb::fields(proto).ok_or("the character data couldn't be read")?;
        let bad = || "the character data couldn't be read".to_string();
        let mut patches = Patches::default();
        // A field-`num` entry with a field inside it replaced (or added).
        let rewrite = |f: &pb::Field, inner_num: u32, encoded: &[u8]| -> Result<Vec<u8>, String> {
            let msg = pb::replace(f.bytes().ok_or_else(bad)?, inner_num, encoded).ok_or_else(bad)?;
            Ok(pb::bytes_field(f.num, &msg))
        };
        for (leaf, v) in changes {
            let t = &self.targets[*leaf];
            let n = whole(v)?;
            let (lo, hi) = self.range(t);
            if n < lo || n > hi {
                return Err(format!("must be between {lo} and {hi}"));
            }
            let n = n as u64;
            match t {
                Target::Level => {}
                Target::Experience => match pb::first(&top, 7) {
                    Some(f) => patches.add(f.start, f.end, pb::int_field(7, n)),
                    None => patches.add(proto.len(), proto.len(), pb::int_field(7, n)),
                },
                Target::SkillPoints => match pb::first(&top, 13) {
                    Some(f) => patches.add(f.start, f.end, rewrite(&f, 1, &pb::int_field(1, n))?),
                    None => patches.add(proto.len(), proto.len(), pb::bytes_field(13, &pb::int_field(1, n))),
                },
                Target::Currency(hash) => {
                    let found = pb::all(&top, 9).find(|f| f.bytes().and_then(pb::fields).is_some_and(|i| pb::first(&i, 1).and_then(|x| x.int()) == Some(*hash)));
                    match found {
                        Some(f) => patches.add(f.start, f.end, rewrite(&f, 2, &pb::int_field(2, n))?),
                        None => {
                            let mut e = pb::int_field(1, *hash);
                            e.extend(pb::int_field(2, n));
                            patches.add(proto.len(), proto.len(), pb::bytes_field(9, &e));
                        }
                    }
                }
                Target::Ammo(i) => {
                    let f = pb::all(&top, 5).nth(*i).ok_or_else(bad)?;
                    patches.add(f.start, f.end, rewrite(&f, 1, &pb::f32_field(1, n as f32))?);
                }
                Target::Sdu(i, _) => {
                    let f = pb::all(&top, 29).nth(*i).ok_or_else(bad)?;
                    patches.add(f.start, f.end, rewrite(&f, 1, &pb::int_field(1, n))?);
                }
            }
        }
        let mut payload = patches.apply(proto)?;
        encrypt(&mut payload);
        let mut out = self.file[..self.len_at].to_vec();
        out.extend((payload.len() as u32).to_le_bytes());
        out.extend(payload);
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let n = whole(v)?;
        let (lo, hi) = self.range(&self.targets[leaf]);
        if n < lo || n > hi {
            return Err(format!("must be between {lo} and {hi}"));
        }
        Ok(())
    }

    /// Level and experience follow each other.
    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        match self.targets[leaf] {
            Target::Level => {
                let lv = view.pointer("/Character/Level").and_then(Value::as_i64).unwrap_or(1).clamp(1, MAX_LEVEL);
                let xp = view.pointer("/Character/Experience").and_then(Value::as_u64).unwrap_or(0);
                if level_for(xp) == lv {
                    return vec![];
                }
                vec![("/Character/Experience".into(), XP[lv as usize - 1].into())]
            }
            Target::Experience => {
                let xp = view.pointer("/Character/Experience").and_then(Value::as_u64).unwrap_or(0);
                vec![("/Character/Level".into(), level_for(xp).into())]
            }
            _ => vec![],
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub const KIND: &str = "Borderlands 3 character";

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let lay = layout(d).ok_or("not a Borderlands 3 character")?;
    let mut proto = d[lay.len_at + 4..].to_vec();
    decrypt(&mut proto);
    let top = pb::fields(&proto).ok_or("the character data couldn't be read (is this a console save?)")?;
    let class_path = pb::first(&top, 4).and_then(|f| f.bytes()).and_then(|b| pb::first(&pb::fields(b)?, 1)?.text()).ok_or("no character class")?;

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
    let class = CLASSES.iter().find(|c| c.0 == object(&class_path));
    let name = pb::first(&top, 43).and_then(|f| f.text()).unwrap_or_default();
    add(&mut view, "Character", "Name", name.into(), None);
    add(&mut view, "Character", "Class", class.map_or_else(|| object(&class_path).to_string(), |c| c.1.to_string()).into(), None);
    add(&mut view, "Character", "Vault hunter", class.map_or("", |c| c.2).into(), None);
    let xp = pb::first(&top, 7).and_then(|f| f.int()).unwrap_or(0);
    add(&mut view, "Character", "Level", level_for(xp).into(), Some(Target::Level));
    add(&mut view, "Character", "Experience", xp.into(), Some(Target::Experience));
    let ability = pb::first(&top, 13).and_then(|f| f.bytes()).and_then(pb::fields);
    let points = ability.as_ref().and_then(|a| pb::first(a, 1)).and_then(|f| f.int()).unwrap_or(0);
    add(&mut view, "Character", "Skill points", points.into(), Some(Target::SkillPoints));
    add(&mut view, "Character", "Playthroughs completed", pb::first(&top, 15).and_then(|f| f.int()).unwrap_or(0).into(), None);

    let currencies: Vec<(u64, u64)> = pb::all(&top, 9)
        .filter_map(|f| {
            let i = pb::fields(f.bytes()?)?;
            Some((pb::first(&i, 1)?.int()?, pb::first(&i, 2).and_then(|x| x.int()).unwrap_or(0)))
        })
        .collect();
    for (hash, label) in [(MONEY, "Money"), (ERIDIUM, "Eridium")] {
        let n = currencies.iter().find(|c| c.0 == hash).map_or(0, |c| c.1);
        add(&mut view, "Currency", label, n.into(), Some(Target::Currency(hash)));
    }

    let mut seen = Map::new();
    for (i, f) in pb::all(&top, 5).enumerate() {
        let Some(inner) = f.bytes().and_then(pb::fields) else { continue };
        let Some(path) = pb::first(&inner, 2).and_then(|x| x.text()) else { continue };
        let amount = pb::first(&inner, 1).and_then(|x| x.f32()).unwrap_or(0.0);
        let key = crate::tree::insert_unique(&mut seen, &ammo_name(&path), Value::Null);
        add(&mut view, "Ammo", &key, super::borderlands::amount_value(amount), Some(Target::Ammo(i)));
    }
    let mut seen = Map::new();
    for (i, f) in pb::all(&top, 29).enumerate() {
        let Some(inner) = f.bytes().and_then(pb::fields) else { continue };
        let Some(path) = pb::first(&inner, 2).and_then(|x| x.text()) else { continue };
        let level = pb::first(&inner, 1).and_then(|x| x.int()).unwrap_or(0);
        let known = SDUS.iter().find(|s| s.0 == object(&path));
        let key = crate::tree::insert_unique(&mut seen, known.map_or(object(&path), |s| s.1), Value::Null);
        add(&mut view, "SDU upgrades", &key, level.into(), known.map(|s| Target::Sdu(i, s.2)));
    }

    // Equipped slots point at inventory entries by index.
    let mut slots: HashMap<u64, String> = HashMap::new();
    for f in pb::all(&top, 11) {
        let Some(inner) = f.bytes().and_then(pb::fields) else { continue };
        let (Some(idx), Some(path)) = (pb::first(&inner, 1).and_then(|x| x.int()), pb::first(&inner, 3).and_then(|x| x.text())) else { continue };
        slots.insert(idx, slot_name(&path));
    }
    let items: Vec<Value> = pb::all(&top, 10)
        .enumerate()
        .filter_map(|(i, f)| {
            let inner = pb::fields(f.bytes()?)?;
            let (ok, version) = serial_ok(pb::first(&inner, 1)?.bytes()?)?;
            let flags = pb::first(&inner, 3).and_then(|x| x.int()).unwrap_or(0);
            let mut m = Map::new();
            m.insert("Slot".into(), slots.get(&(i as u64)).cloned().unwrap_or_else(|| "Backpack".into()).into());
            m.insert("Favorite".into(), (flags & 2 != 0).into());
            m.insert("Junk".into(), (flags & 4 != 0).into());
            m.insert("Has skin".into(), pb::first(&inner, 4).is_some().into());
            m.insert("Serial version".into(), version.into());
            m.insert("Serial check".into(), if ok { "OK" } else { "doesn't match" }.into());
            Some(Value::Object(m))
        })
        .collect();
    view.insert("Inventory".into(), Value::Array(items));

    let backend = Bl3Backend { file: Arc::new(d.to_vec()), proto: Arc::new(proto), len_at: lay.len_at, targets };
    Ok(TreeDoc::new(Value::Object(view), editable, KIND, Box::new(backend)).with_note(
        "Level (set through experience), skill points, money, Eridium, ammo and SDU levels can be changed; the save is encrypted again when you save. \
         Items are shown, not edited. Turn off cloud saves for the game first, or the old save may come back.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic BL3 save: header with two format GUIDs, then an encrypted character.
    fn sample() -> Vec<u8> {
        let mut p = vec![];
        p.extend(pb::int_field(1, 3));
        p.extend(pb::bytes_field(4, &pb::bytes_field(1, b"/Game/PlayerCharacters/Gunner/PlayerClassId_Gunner.PlayerClassId_Gunner")));
        for (amount, res) in [(Some(120.0f32), "Resource_Ammo_Pistol"), (None, "Resource_Ammo_Heavy")] {
            let mut r = amount.map(|a| pb::f32_field(1, a)).unwrap_or_default();
            r.extend(pb::bytes_field(2, format!("/Game/GameData/Weapons/Ammo/{res}.{res}").as_bytes()));
            p.extend(pb::bytes_field(5, &r));
        }
        p.extend(pb::int_field(7, 30_000));
        let mut c = pb::int_field(1, MONEY);
        c.extend(pb::int_field(2, 500));
        p.extend(pb::bytes_field(9, &c));
        let mut s = vec![3u8, 0, 0, 0, 0, 0, 0];
        s.extend([7u8; 20]);
        let mut h = flate2::Crc::new();
        h.update(&s[..5]);
        h.update(&[0xFF, 0xFF]);
        h.update(&s[7..]);
        let c = h.sum();
        s[5..7].copy_from_slice(&(((c >> 16) ^ c) as u16).to_be_bytes());
        let mut it = pb::bytes_field(1, &s);
        it.extend(pb::int_field(3, 3));
        p.extend(pb::bytes_field(10, &it));
        p.extend(pb::bytes_field(10, &it));
        let mut eq = pb::int_field(1, 1);
        eq.extend(pb::bytes_field(3, b"/Game/Gear/Weapons/_Shared/_Design/InventorySlots/BPInvSlot_Weapon2.BPInvSlot_Weapon2"));
        p.extend(pb::bytes_field(11, &eq));
        p.extend(pb::bytes_field(13, &pb::bytes_field(2, b"tree")));
        let mut sdu = pb::int_field(1, 2);
        sdu.extend(pb::bytes_field(2, b"/Game/Pickups/SDU/SDU_Backpack.SDU_Backpack"));
        p.extend(pb::bytes_field(29, &sdu));
        p.extend(pb::bytes_field(43, b"Tester"));
        let mut d = b"GVAS".to_vec();
        for v in [2u32, 516] {
            d.extend(v.to_le_bytes());
        }
        d.extend([4, 0, 20, 0, 3, 0, 1, 2, 3, 4]);
        let s = b"OAK-TEST\0";
        d.extend((s.len() as i32).to_le_bytes());
        d.extend(s);
        d.extend(3u32.to_le_bytes());
        d.extend(2u32.to_le_bytes());
        d.extend([0xAB; 40]);
        let s = b"OakSaveGame\0";
        d.extend((s.len() as i32).to_le_bytes());
        d.extend(s);
        d.extend((p.len() as u32).to_le_bytes());
        encrypt(&mut p);
        d.extend(p);
        d
    }

    #[test]
    fn xor_round_trip() {
        let plain: Vec<u8> = (0..300u32).map(|i| (i * 7) as u8).collect();
        let mut e = plain.clone();
        encrypt(&mut e);
        assert_ne!(e, plain);
        decrypt(&mut e);
        assert_eq!(e, plain);
    }

    #[test]
    fn reads_and_edits() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Character/Class"), Some(&"Gunner".into()));
        assert_eq!(v.pointer("/Character/Level"), Some(&9.into()));
        assert_eq!(v.pointer("/Character/Skill points"), Some(&0.into()));
        assert_eq!(v.pointer("/Currency/Money"), Some(&500.into()));
        assert_eq!(v.pointer("/Currency/Eridium"), Some(&0.into()));
        assert_eq!(v.pointer("/Ammo/Pistol"), Some(&120.into()));
        assert_eq!(v.pointer("/Ammo/Heavy weapons"), Some(&0.into()));
        assert_eq!(v.pointer("/SDU upgrades/Backpack"), Some(&2.into()));
        assert_eq!(v.pointer("/Inventory/1/Slot"), Some(&"Weapon 2".into()));
        assert_eq!(v.pointer("/Inventory/0/Favorite"), Some(&true.into()));
        assert_eq!(v.pointer("/Inventory/0/Serial check"), Some(&"OK".into()));
        assert_eq!(doc.to_bytes().unwrap(), d);
        // Values that weren't in the file (eridium, skill points, heavy ammo) are added.
        doc.set("/Currency/Eridium", 250.into()).unwrap();
        doc.set("/Currency/Money", 1_000_000.into()).unwrap();
        doc.set("/Character/Skill points", 4.into()).unwrap();
        doc.set("/Ammo/Heavy weapons", 12.into()).unwrap();
        doc.set("/SDU upgrades/Backpack", 13.into()).unwrap();
        doc.set("/Character/Level", 50.into()).unwrap();
        assert_eq!(doc.view.pointer("/Character/Experience"), Some(&3_429_728.into()));
        assert!(doc.set("/SDU upgrades/Backpack", 14.into()).is_err());
        let out = doc.to_bytes().unwrap();
        let at = layout(&d).unwrap().len_at;
        assert_eq!(out[..at], d[..at], "header kept");
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
    }

    #[test]
    fn levels() {
        assert_eq!(level_for(0), 1);
        assert_eq!(level_for(357), 1);
        assert_eq!(level_for(358), 2);
        assert_eq!(level_for(3_429_728), 50);
        assert_eq!(level_for(u64::MAX), 72);
        assert!(!looks(b"GVAS"));
    }

    /// Research aid: SAVE_EDITOR_BL3 = Borderlands 3 saves separated by `|`.
    #[test]
    #[ignore]
    fn real_saves() {
        use crate::formats::{decode_with, encode, Content, Options};
        let Ok(list) = std::env::var("SAVE_EDITOR_BL3") else { return };
        for f in list.split('|').map(std::path::PathBuf::from) {
            let raw = std::fs::read(&f).unwrap();
            let d = decode_with(&raw, &Options::for_path(&f));
            let Content::Tree(doc) = &d.content else { panic!("{} didn't open", f.display()) };
            assert_eq!(doc.kind_name, KIND);
            let v = &doc.view;
            println!("{}: {} {} L{} xp {} money {} eridium {} ammo {} sdu {} items {}", f.display(), v["Character"]["Name"], v["Character"]["Class"], v["Character"]["Level"], v["Character"]["Experience"], v["Currency"]["Money"], v["Currency"]["Eridium"], v["Ammo"], v["SDU upgrades"], v["Inventory"].as_array().unwrap().len());
            assert!(v["Inventory"].as_array().unwrap().iter().all(|i| i["Serial check"] == "OK"));
            assert_eq!(encode(&d.layers, &d.content).unwrap(), raw, "unchanged save writes back byte for byte");
            let mut doc = doc.as_ref().clone();
            let money = doc.view.pointer("/Currency/Money").and_then(Value::as_i64).unwrap();
            doc.set("/Currency/Money", (money + 1).into()).unwrap();
            doc.set("/Ammo/Pistol", 7.into()).unwrap();
            let out = encode(&d.layers, &Content::Tree(Box::new(doc.clone()))).unwrap();
            let back = decode_with(&out, &Options::for_path(&f));
            let Content::Tree(again) = &back.content else { panic!("edited save doesn't open") };
            assert_eq!(again.view, doc.view);
            assert_eq!(again.view.pointer("/Currency/Money").and_then(Value::as_i64), Some(money + 1));
            assert_eq!(again.view.pointer("/Ammo/Pistol"), Some(&7.into()));
            // Only the money entry (field 9) and an ammo entry (field 5) changed.
            let plain = |b: &[u8]| {
                let l = layout(b).unwrap();
                let mut p = b[l.len_at + 4..].to_vec();
                decrypt(&mut p);
                p
            };
            let (p0, p1) = (plain(&raw), plain(&out));
            assert_eq!(out[..layout(&out).unwrap().len_at], raw[..layout(&raw).unwrap().len_at]);
            let (t0, t1) = (pb::fields(&p0).unwrap(), pb::fields(&p1).unwrap());
            assert_eq!(t0.len(), t1.len());
            let changed: Vec<u32> = t0.iter().zip(&t1).filter(|(a, b)| p0[a.start..a.end] != p1[b.start..b.end]).map(|(a, _)| a.num).collect();
            assert!(changed.iter().all(|&n| n == 9 || n == 5) && changed.len() == 2, "{changed:?}");
        }
    }
}
