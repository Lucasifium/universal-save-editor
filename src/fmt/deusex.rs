//! Deus Ex (2000) saves: each `Save\SaveNNNN` folder holds `SaveInfo.dxs` (description,
//! date and the map the player was on) and one `<map>.dxs` per map visited in the current
//! mission. Every `.dxs` is a plain Unreal Engine 1 package (little-endian, no packing, no
//! checksum): a name table, imports, and exports whose data is a list of tagged properties
//! (compact name index, an info byte with the type, size code and "array index follows"
//! bit, then the value), ending at the name `None`. Only values that differ from the
//! class defaults are written.
//!
//! JC Denton is a `JCDentonMale` export. Every map file keeps a copy of him from when he
//! last left that map; only the map named in SaveInfo.dxs holds the one the game loads, and
//! on every save checked that copy is also the only one linked to a skill and augmentation
//! system. So a map file whose JC has those links is edited; other copies are shown
//! read-only with a pointer to the right file.
//!
//! The package layout follows UELib (EliotVU; facts only); property and class names
//! come from the save itself. Values are changed in place: properties the game left out
//! (at their defaults) can't be set.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use serde_json::Value;

const MAGIC: u32 = 0x9E2A_83C1;
/// Object flag: a state frame comes before the properties.
const HAS_STACK: u32 = 0x0200_0000;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// UE1 compact index: sign bit, "more" bit and 6 bits, then 7 bits per byte.
fn cidx(d: &[u8], at: usize) -> Option<(i32, usize)> {
    let b0 = *d.get(at)?;
    let mut v = i64::from(b0 & 0x3F);
    let mut p = at + 1;
    if b0 & 0x40 != 0 {
        let mut shift = 6;
        loop {
            let b = *d.get(p)?;
            p += 1;
            v |= i64::from(b & 0x7F) << shift;
            shift += 7;
            if b & 0x80 == 0 || shift > 34 {
                break;
            }
        }
    }
    let v = if b0 & 0x80 != 0 { -v } else { v };
    Some((i32::try_from(v).ok()?, p))
}

struct Export {
    class: i32,
    name: String,
    flags: u32,
    off: usize,
    size: usize,
}

struct Pkg<'a> {
    d: &'a [u8],
    names: Vec<String>,
    /// Object name of each import.
    imports: Vec<String>,
    exports: Vec<Export>,
}

#[derive(Clone, Debug)]
struct Prop {
    name: String,
    index: u32,
    ty: u8,
    /// Where the value starts (for a bool: the info byte, which holds it).
    at: usize,
    size: usize,
    flag: bool,
}

impl<'a> Pkg<'a> {
    fn open(d: &'a [u8]) -> Option<Self> {
        if u32_at(d, 0)? != MAGIC {
            return None;
        }
        let ver = u16::from_le_bytes(d.get(4..6)?.try_into().ok()?);
        if !(61..=69).contains(&ver) {
            return None;
        }
        let f = |i: usize| u32_at(d, 12 + 4 * i).map(|v| v as usize);
        let (nn, no, ne, eo, ni, io) = (f(0)?, f(1)?, f(2)?, f(3)?, f(4)?, f(5)?);
        if nn > 200_000 || ne > 200_000 || ni > 200_000 {
            return None;
        }
        let mut names = Vec::with_capacity(nn);
        let mut p = no;
        for _ in 0..nn {
            let (len, q) = cidx(d, p)?;
            let len = usize::try_from(len).ok()?;
            let s = d.get(q..q + len)?;
            names.push(s.iter().take_while(|&&c| c != 0).map(|&c| super::bethesda_cp1252(c)).collect());
            p = q + len + 4;
        }
        let name = |i: i32| names.get(usize::try_from(i).ok()?).cloned();
        let mut imports = Vec::with_capacity(ni);
        let mut p = io;
        for _ in 0..ni {
            let (_, q) = cidx(d, p)?;
            let (_, q) = cidx(d, q)?;
            let (on, q) = cidx(d, q + 4)?;
            imports.push(name(on)?);
            p = q;
        }
        let mut exports = Vec::with_capacity(ne);
        let mut p = eo;
        for _ in 0..ne {
            let (class, q) = cidx(d, p)?;
            let (_, q) = cidx(d, q)?;
            let (on, q) = cidx(d, q + 4)?;
            let flags = u32_at(d, q)?;
            let (size, mut q) = cidx(d, q + 4)?;
            let mut off = 0;
            if size > 0 {
                let (o, r) = cidx(d, q)?;
                off = usize::try_from(o).ok()?;
                q = r;
            }
            let size = usize::try_from(size).ok()?;
            if off + size > d.len() {
                return None;
            }
            exports.push(Export { class, name: name(on)?, flags, off, size });
            p = q;
        }
        Some(Pkg { d, names, imports, exports })
    }

    fn class(&self, e: &Export) -> &str {
        match e.class {
            c if c < 0 => self.imports.get((-c - 1) as usize).map_or("", String::as_str),
            c if c > 0 => self.exports.get((c - 1) as usize).map_or("", |x| x.name.as_str()),
            _ => "Class",
        }
    }

    fn has_name(&self, n: &str) -> bool {
        self.names.iter().any(|x| x == n)
    }

    /// The tagged properties of export `i` (0-based).
    fn props(&self, i: usize) -> Option<Vec<Prop>> {
        let e = self.exports.get(i)?;
        let d = self.d;
        let end = e.off + e.size;
        let mut p = e.off;
        if e.flags & HAS_STACK != 0 {
            let (node, q) = cidx(d, p)?;
            let (_, q) = cidx(d, q)?;
            p = q + 12;
            if node != 0 {
                p = cidx(d, p)?.1;
            }
        }
        let mut out = vec![];
        while p < end {
            let (ni, q) = cidx(d, p)?;
            let name = self.names.get(usize::try_from(ni).ok()?)?.clone();
            if name == "None" {
                return Some(out);
            }
            let info_at = q;
            let info = *d.get(q)?;
            let mut q = q + 1;
            let ty = info & 0x0F;
            if ty == 10 {
                q = cidx(d, q)?.1; // struct name
            }
            let size = match (info >> 4) & 7 {
                0 => 1,
                1 => 2,
                2 => 4,
                3 => 12,
                4 => 16,
                5 => {
                    q += 1;
                    usize::from(*d.get(q - 1)?)
                }
                6 => {
                    q += 2;
                    usize::from(u16::from_le_bytes(d.get(q - 2..q)?.try_into().ok()?))
                }
                _ => {
                    q += 4;
                    u32_at(d, q - 4)? as usize
                }
            };
            let mut index = 0;
            if info & 0x80 != 0 && ty != 3 {
                let b = *d.get(q)?;
                if b < 0x80 {
                    index = u32::from(b);
                    q += 1;
                } else if b < 0xC0 {
                    index = u32::from(u16::from_be_bytes(d.get(q..q + 2)?.try_into().ok()?)) & 0x3FFF;
                    q += 2;
                } else {
                    index = u32::from_be_bytes(d.get(q..q + 4)?.try_into().ok()?) & 0x3FFF_FFFF;
                    q += 4;
                }
            }
            if ty == 3 {
                out.push(Prop { name, index: 0, ty, at: info_at, size: 0, flag: info & 0x80 != 0 });
                p = q;
                continue;
            }
            if q + size > end {
                return None;
            }
            out.push(Prop { name, index, ty, at: q, size, flag: false });
            p = q + size;
        }
        None
    }

    /// Export index (0-based) an object property points at.
    fn target(&self, p: &Prop) -> Option<usize> {
        if p.ty != 5 {
            return None;
        }
        let (r, _) = cidx(self.d, p.at)?;
        (r > 0).then(|| r as usize - 1).filter(|&i| i < self.exports.len())
    }

    fn string(&self, p: &Prop) -> Option<String> {
        match p.ty {
            13 | 7 => {
                let (len, q) = cidx(self.d, p.at)?;
                let s = self.d.get(q..q + usize::try_from(len).ok()?)?;
                Some(s.iter().take_while(|&&c| c != 0).map(|&c| super::bethesda_cp1252(c)).collect())
            }
            6 => self.names.get(usize::try_from(cidx(self.d, p.at)?.0).ok()?).cloned(),
            _ => None,
        }
    }
}

fn get<'p>(props: &'p [Prop], name: &str) -> Option<&'p Prop> {
    props.iter().find(|p| p.index == 0 && p.name.eq_ignore_ascii_case(name))
}

fn int(p: Option<&Prop>) -> Option<&Prop> {
    p.filter(|p| p.ty == 2 && p.size == 4)
}

fn float(p: Option<&Prop>) -> Option<&Prop> {
    p.filter(|p| p.ty == 4 && p.size == 4)
}

pub fn looks(d: &[u8]) -> bool {
    u32_at(d, 0) == Some(MAGIC) && Pkg::open(d).is_some_and(|p| p.has_name("JCDentonMale") || p.has_name("DeusExSaveInfo"))
}

/// "HazMatSuit" → "Haz Mat Suit", "EMPGrenade" → "EMP Grenade".
fn words(key: &str) -> String {
    let c: Vec<char> = key.chars().collect();
    let mut out = String::new();
    for (i, &ch) in c.iter().enumerate() {
        let prev = i.checked_sub(1).map(|j| c[j]);
        let next = c.get(i + 1).copied();
        let boundary = ch.is_uppercase()
            && prev.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit() || (p.is_uppercase() && next.is_some_and(char::is_lowercase)));
        if boundary {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn skill_name(class: &str) -> String {
    match class {
        "SkillWeaponHeavy" => "Weapons: Heavy".into(),
        "SkillWeaponPistol" => "Weapons: Pistol".into(),
        "SkillWeaponRifle" => "Weapons: Rifle".into(),
        "SkillWeaponLowTech" => "Weapons: Low-Tech".into(),
        "SkillDemolition" => "Weapons: Demolition".into(),
        "SkillEnviro" => "Environmental Training".into(),
        "SkillTech" => "Electronics".into(),
        "SkillLockpicking" => "Lockpicking".into(),
        "SkillMedicine" => "Medicine".into(),
        "SkillComputer" => "Computer".into(),
        "SkillSwimming" => "Swimming".into(),
        c => words(c.trim_start_matches("Skill")),
    }
}

fn aug_name(class: &str) -> String {
    match class {
        "AugAqualung" => "Aqualung".into(),
        "AugBallistic" => "Ballistic Protection".into(),
        "AugCloak" => "Cloak".into(),
        "AugCombat" => "Combat Strength".into(),
        "AugDefense" => "Aggressive Defense System".into(),
        "AugDrone" => "Spy Drone".into(),
        "AugEMP" => "EMP Shield".into(),
        "AugEnviro" => "Environmental Resistance".into(),
        "AugHealing" => "Regeneration".into(),
        "AugHeartLung" => "Synthetic Heart".into(),
        "AugMuscle" => "Microfibral Muscle".into(),
        "AugPower" => "Power Recirculator".into(),
        "AugRadarTrans" => "Radar Transparency".into(),
        "AugShield" => "Energy Shield".into(),
        "AugSpeed" => "Speed Enhancement".into(),
        "AugStealth" => "Run Silent".into(),
        "AugTarget" => "Targeting".into(),
        "AugVision" => "Vision Enhancement".into(),
        "AugIFF" => "IFF".into(),
        "AugDatalink" => "Infolink".into(),
        "AugLight" => "Light".into(),
        c => words(c.trim_start_matches("Aug")),
    }
}

fn item_name(class: &str) -> String {
    match class {
        "Ammo10mm" => "10mm Ammo".into(),
        "Ammo20mm" => "20mm HE Ammo".into(),
        "Ammo3006" => "30.06 Ammo".into(),
        "Ammo762mm" => "7.62x51mm Ammo".into(),
        "AmmoBattery" => "Prod Charger".into(),
        "AmmoDart" => "Darts".into(),
        "AmmoDartFlare" => "Flare Darts".into(),
        "AmmoDartPoison" => "Tranquilizer Darts".into(),
        "AmmoNapalm" => "Napalm Canister".into(),
        "AmmoPepper" => "Pepper Cartridge".into(),
        "AmmoPlasma" => "Plasma Clip".into(),
        "AmmoRocket" => "Rockets".into(),
        "AmmoRocketWP" => "WP Rockets".into(),
        "AmmoSabot" => "12 Gauge Sabot Shells".into(),
        "AmmoShell" => "12 Gauge Buckshot Shells".into(),
        "MedKit" => "Medkits".into(),
        "BioelectricCell" => "Bioelectric Cells".into(),
        "Lockpick" => "Lockpicks".into(),
        "Multitool" => "Multitools".into(),
        c if c.starts_with("Ammo") => format!("{} (ammo)", words(&c[4..])),
        c if c.starts_with("Weapon") => words(&c[6..]),
        c => words(c),
    }
}

/// `name`, or `name (2)`… when it's taken under `group`.
fn unique(used: &mut std::collections::HashSet<String>, group: &str, name: String) -> String {
    let mut k = name.clone();
    let mut n = 2;
    while !used.insert(format!("{group}/{k}")) {
        k = format!("{name} ({n})");
        n += 1;
    }
    k
}

fn save_info(pkg: &Pkg, b: &mut Builder) -> Option<String> {
    let i = pkg.exports.iter().position(|e| pkg.class(e) == "DeusExSaveInfo" && e.size > 0)?;
    let props = pkg.props(i)?;
    let s = |n: &str| get(&props, n).and_then(|p| pkg.string(p));
    let n = |k: &str| get(&props, k).filter(|p| p.ty == 2).and_then(|p| u32_at(pkg.d, p.at)).map(|v| v as i32);
    if let Some(v) = s("Description") {
        b.info(&["Save", "Description"], v);
    }
    if let Some(v) = s("MissionLocation") {
        b.info(&["Save", "Location"], v);
    }
    let map = s("MapName");
    if let Some(v) = &map {
        b.info(&["Save", "Map"], v.clone());
    }
    if let (Some(y), Some(mo), Some(d)) = (n("Year"), n("Month"), n("Day")) {
        b.info(&["Save", "Saved on"], format!("{y:04}-{mo:02}-{d:02} {:02}:{:02}", n("Hour").unwrap_or(0), n("Minute").unwrap_or(0)));
    }
    if let Some(v) = n("saveCount") {
        b.info(&["Save", "Save count"], i64::from(v));
    }
    if let Some(v) = n("saveTime") {
        b.info(&["Save", "Play time (seconds)"], i64::from(v));
    }
    map.or_else(|| Some(String::new()))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let pkg = Pkg::open(d).ok_or("not an Unreal package")?;
    let mut b = Builder::new(d);
    b.info(&["Save", "Game"], "Deus Ex");
    if let Some(map) = save_info(&pkg, &mut b) {
        let file = if map.is_empty() { "the map's .dxs file".to_string() } else { format!("{map}.dxs") };
        b.info(&["Save", "Open to edit JC"], file.clone());
        return Ok(b.finish("Deus Ex save info", Checksum::None).with_note(format!(
            "This is the save's SaveInfo.dxs (description, date and current map). JC Denton is in {file} in the same folder: open that file to edit him."
        )));
    }

    let objs: Vec<Option<Vec<Prop>>> = (0..pkg.exports.len()).map(|i| pkg.props(i)).collect();
    let players: Vec<usize> = (0..pkg.exports.len()).filter(|&i| pkg.class(&pkg.exports[i]) == "JCDentonMale" && objs[i].is_some()).collect();
    let live = |i: &usize| objs[*i].as_ref().is_some_and(|p| get(p, "SkillSystem").is_some() || get(p, "AugmentationSystem").is_some());
    let Some(pi) = players.iter().copied().find(live).or_else(|| players.first().copied()) else {
        return Err("no JC Denton in this file".into());
    };
    let is_live = live(&pi);
    let player = objs[pi].clone().unwrap_or_default();
    b.info(&["Save", "Live player"], is_live);

    // Editable only in the copy the game loads.
    let ranged = |b: &mut Builder, path: &[&str], p: Option<&Prop>, lo: i64, hi: i64| {
        let p = int(p)?;
        if is_live {
            b.ranged(path, p.at, Kind::I32, lo, hi)
        } else {
            let v = Value::from(u32_at(d, p.at)? as i32);
            b.info(path, v.clone());
            Some(v)
        }
    };
    let ranged_f = |b: &mut Builder, path: &[&str], p: Option<&Prop>, lo: i64, hi: i64| {
        let p = float(p)?;
        if is_live {
            b.ranged(path, p.at, Kind::F32, lo, hi)
        } else {
            let v = crate::tree::f32_value(f32::from_bits(u32_at(d, p.at)?));
            b.info(path, v.clone());
            Some(v)
        }
    };

    ranged(&mut b, &["JC Denton", "Credits"], get(&player, "Credits"), 0, 99_999_999);
    ranged(&mut b, &["JC Denton", "Skill points"], get(&player, "SkillPointsAvail"), 0, 9_999_999);
    ranged(&mut b, &["JC Denton", "Skill points earned"], get(&player, "SkillPointsTotal"), 0, 9_999_999);
    if ranged_f(&mut b, &["JC Denton", "Bioelectric energy"], get(&player, "Energy"), 0, 1000).is_none() {
        b.info(&["JC Denton", "Bioelectric energy"], "not stored (at the game's default)");
    }
    ranged_f(&mut b, &["JC Denton", "Bioelectric energy max"], get(&player, "EnergyMax"), 0, 1000);
    for (label, prop) in [
        ("Overall", "Health"),
        ("Head", "HealthHead"),
        ("Torso", "HealthTorso"),
        ("Left arm", "HealthArmLeft"),
        ("Right arm", "HealthArmRight"),
        ("Left leg", "HealthLegLeft"),
        ("Right leg", "HealthLegRight"),
    ] {
        if ranged(&mut b, &["Health", label], get(&player, prop), 0, 1000).is_none() {
            // Left out of the save: at full health (100).
            b.info(&["Health", label], 100);
        }
    }

    // The owned items: a chain through each item's Inventory property.
    let mut used = std::collections::HashSet::new();
    let mut weapons = vec![];
    let mut seen = std::collections::HashSet::new();
    let mut cur = get(&player, "Inventory").and_then(|p| pkg.target(p));
    while let Some(i) = cur {
        if !seen.insert(i) || seen.len() > 1000 {
            break;
        }
        let class = pkg.class(&pkg.exports[i]).to_string();
        let props = objs[i].clone().unwrap_or_default();
        cur = get(&props, "Inventory").and_then(|p| pkg.target(p));
        if class.starts_with("Image") || class.starts_with("NanoKey") || class == "AmmoNone" {
            continue;
        }
        if class.starts_with("Weapon") {
            weapons.push(item_name(&class));
            continue;
        }
        let (group, prop, default) = if class.starts_with("Ammo") { ("Ammo", "AmmoAmount", "default amount (not stored)") } else { ("Items", "NumCopies", "1 (not stored)") };
        let label = unique(&mut used, group, item_name(&class));
        if ranged(&mut b, &[group, &label], get(&props, prop), 0, 9999).is_none() {
            b.info(&[group, &label], default);
        }
    }
    if !weapons.is_empty() {
        b.info(&["Weapons"], weapons.join(", "));
    }

    // Skills and augmentations: linked lists from the player's managers.
    let chain = |manager: &str, first: &str| -> Vec<usize> {
        let mut out = vec![];
        let mut cur = get(&player, manager).and_then(|p| pkg.target(p)).and_then(|m| objs[m].as_ref()).and_then(|m| get(m, first)).and_then(|p| pkg.target(p));
        while let Some(i) = cur {
            if out.contains(&i) || out.len() > 100 {
                break;
            }
            out.push(i);
            cur = objs[i].as_ref().and_then(|p| get(p, "Next")).and_then(|p| pkg.target(p));
        }
        out
    };
    const SKILL_LEVELS: [&str; 4] = ["Untrained", "Trained", "Advanced", "Master"];
    for i in chain("SkillSystem", "FirstSkill") {
        let label = skill_name(pkg.class(&pkg.exports[i]));
        let props = objs[i].clone().unwrap_or_default();
        match int(get(&props, "CurrentLevel")) {
            Some(p) => {
                b.named(&["Skills", &label], p.at, Kind::I32, &SKILL_LEVELS);
            }
            None => b.info(&["Skills", &label], SKILL_LEVELS[0]),
        }
    }
    const AUG_LEVELS: [&str; 4] = ["Level 1", "Level 2", "Level 3", "Level 4"];
    for i in chain("AugmentationSystem", "FirstAug") {
        let props = objs[i].clone().unwrap_or_default();
        if !get(&props, "bHasIt").is_some_and(|p| p.ty == 3 && p.flag) {
            continue;
        }
        let label = aug_name(pkg.class(&pkg.exports[i]));
        match int(get(&props, "CurrentLevel")) {
            Some(p) => {
                b.named(&["Augmentations", &label], p.at, Kind::I32, &AUG_LEVELS);
            }
            None => b.info(&["Augmentations", &label], AUG_LEVELS[0]),
        }
    }

    let note = if is_live {
        "JC Denton from the map he was on when the game was saved. Values the game left out (full health, untrained skills, level-1 augmentations) aren't in the file, so they can't be changed here."
            .to_string()
    } else {
        "This map file only holds the copy of JC Denton from when he last left this map; the game loads him from the map named in the folder's SaveInfo.dxs. Open that map's .dxs file to edit him."
            .to_string()
    };
    Ok(b.finish("Deus Ex map save", Checksum::None).with_note(note))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn ci(v: i32) -> Vec<u8> {
        let mut out = vec![];
        let neg = v < 0;
        let mut a = v.unsigned_abs();
        let mut b0 = (a & 0x3F) as u8;
        if neg {
            b0 |= 0x80;
        }
        a >>= 6;
        if a > 0 {
            b0 |= 0x40;
        }
        out.push(b0);
        while a > 0 {
            let mut b = (a & 0x7F) as u8;
            a >>= 7;
            if a > 0 {
                b |= 0x80;
            }
            out.push(b);
        }
        out
    }

    /// A synthetic package: JC with credits, health and an inventory; skills and augs.
    pub(crate) fn sample(live: bool) -> Vec<u8> {
        let names = [
            "None", "JCDentonMale", "Credits", "SkillPointsAvail", "Energy", "HealthHead", "Inventory", "SkillSystem", "AugmentationSystem", "AmmoAmount", "NumCopies",
            "Ammo10mm", "MedKit", "SkillManager", "FirstSkill", "Next", "SkillComputer", "CurrentLevel", "AugmentationManager", "FirstAug", "AugSpeed", "bHasIt",
            "JCDentonMale1", "Ammo10mm0", "MedKit0", "SkillManager0", "SkillComputer0", "AugmentationManager0", "AugSpeed0", "SkillTech", "SkillTech0", "Core", "Class",
        ];
        let n = |s: &str| names.iter().position(|x| *x == s).unwrap() as i32;
        // Imports: the classes (export class = -(import index + 1)).
        let classes = ["JCDentonMale", "Ammo10mm", "MedKit", "SkillManager", "SkillComputer", "AugmentationManager", "AugSpeed", "SkillTech"];
        let cls = |s: &str| -(classes.iter().position(|x| *x == s).unwrap() as i32 + 1);
        let tag = |o: &mut Vec<u8>, name: &str, info: u8| {
            o.extend(ci(n(name)));
            o.push(info);
        };
        let int_p = |o: &mut Vec<u8>, name: &str, v: i32| {
            tag(o, name, 0x22);
            o.extend(v.to_le_bytes());
        };
        let obj_p = |o: &mut Vec<u8>, name: &str, export: i32| {
            let r = ci(export);
            tag(o, name, 0x05 | if r.len() == 1 { 0 } else { 0x10 });
            o.extend(r);
        };
        let end = |o: &mut Vec<u8>| o.extend(ci(n("None")));
        // Exports (1-based refs): 1 JC, 2 ammo, 3 medkit, 4 skill manager, 5 computer, 6 tech, 7 aug manager, 8 speed.
        let mut objs: Vec<(i32, &str, Vec<u8>)> = vec![];
        let mut jc = vec![];
        int_p(&mut jc, "Credits", 450);
        int_p(&mut jc, "SkillPointsAvail", 1033);
        tag(&mut jc, "Energy", 0x24);
        jc.extend(73.5f32.to_le_bytes());
        int_p(&mut jc, "HealthHead", 54);
        obj_p(&mut jc, "Inventory", 2);
        if live {
            obj_p(&mut jc, "SkillSystem", 4);
            obj_p(&mut jc, "AugmentationSystem", 7);
        }
        end(&mut jc);
        objs.push((cls("JCDentonMale"), "JCDentonMale1", jc));
        let mut ammo = vec![];
        int_p(&mut ammo, "AmmoAmount", 148);
        obj_p(&mut ammo, "Inventory", 3);
        end(&mut ammo);
        objs.push((cls("Ammo10mm"), "Ammo10mm0", ammo));
        let mut med = vec![];
        int_p(&mut med, "NumCopies", 10);
        end(&mut med);
        objs.push((cls("MedKit"), "MedKit0", med));
        let mut sm = vec![];
        obj_p(&mut sm, "FirstSkill", 5);
        end(&mut sm);
        objs.push((cls("SkillManager"), "SkillManager0", sm));
        let mut comp = vec![];
        int_p(&mut comp, "CurrentLevel", 3);
        obj_p(&mut comp, "Next", 6);
        end(&mut comp);
        objs.push((cls("SkillComputer"), "SkillComputer0", comp));
        let mut tech = vec![];
        end(&mut tech);
        objs.push((cls("SkillTech"), "SkillTech0", tech));
        let mut am = vec![];
        obj_p(&mut am, "FirstAug", 8);
        end(&mut am);
        objs.push((cls("AugmentationManager"), "AugmentationManager0", am));
        let mut speed = vec![];
        int_p(&mut speed, "CurrentLevel", 1);
        tag(&mut speed, "bHasIt", 0x83);
        end(&mut speed);
        objs.push((cls("AugSpeed"), "AugSpeed0", speed));

        let mut d = vec![];
        d.extend(MAGIC.to_le_bytes());
        d.extend(68u16.to_le_bytes());
        d.extend(0u16.to_le_bytes());
        d.extend(1u32.to_le_bytes());
        let counts_at = d.len();
        d.extend([0u8; 24]);
        let name_off = d.len();
        for s in names {
            d.extend(ci(s.len() as i32 + 1));
            d.extend(s.as_bytes());
            d.push(0);
            d.extend(0u32.to_le_bytes());
        }
        let import_off = d.len();
        for c in classes {
            d.extend(ci(n("Core")));
            d.extend(ci(n("Class")));
            d.extend(0i32.to_le_bytes());
            d.extend(ci(n(c)));
        }
        let data_off = d.len();
        let mut at = data_off;
        let mut table = vec![];
        for (c, name, data) in &objs {
            d.extend(data);
            table.extend(ci(*c));
            table.extend(ci(0));
            table.extend(0i32.to_le_bytes());
            table.extend(ci(n(name)));
            table.extend(0x0007_0001u32.to_le_bytes());
            table.extend(ci(data.len() as i32));
            table.extend(ci(at as i32));
            at += data.len();
        }
        let export_off = d.len();
        d.extend(table);
        for (i, v) in [names.len(), name_off, objs.len(), export_off, classes.len(), import_off].iter().enumerate() {
            d[counts_at + 4 * i..counts_at + 4 * i + 4].copy_from_slice(&(*v as u32).to_le_bytes());
        }
        d
    }

    #[test]
    fn compact_index() {
        for v in [0, 1, 63, 64, 100, 8191, 8192, 1_000_000, -5, -100_000] {
            assert_eq!(cidx(&ci(v), 0), Some((v, ci(v).len())), "{v}");
        }
    }

    #[test]
    fn reads_and_edits_jc() {
        let d = sample(true);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(doc.kind_name, "Deus Ex map save");
        assert_eq!(v(&doc, "/Save/Live player"), Some(true.into()));
        assert_eq!(v(&doc, "/JC Denton/Credits"), Some(450.into()));
        assert_eq!(v(&doc, "/JC Denton/Skill points"), Some(1033.into()));
        assert_eq!(v(&doc, "/JC Denton/Bioelectric energy"), Some(73.5.into()));
        assert_eq!(v(&doc, "/Health/Head"), Some(54.into()));
        assert_eq!(v(&doc, "/Health/Torso"), Some(100.into()));
        assert!(!doc.editable.contains_key("/Health/Torso"));
        assert_eq!(v(&doc, "/Ammo/10mm Ammo"), Some(148.into()));
        assert_eq!(v(&doc, "/Items/Medkits"), Some(10.into()));
        assert_eq!(words("EMPGrenade"), "EMP Grenade");
        assert_eq!(words("HazMatSuit"), "Haz Mat Suit");
        assert_eq!(v(&doc, "/Skills/Computer"), Some("Master".into()));
        assert_eq!(v(&doc, "/Skills/Electronics"), Some("Untrained".into()));
        assert!(!doc.editable.contains_key("/Skills/Electronics"));
        assert_eq!(v(&doc, "/Augmentations/Speed Enhancement"), Some("Level 2".into()));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/JC Denton/Credits", 451.into()).unwrap();
        doc.set("/Skills/Computer", "Trained".into()).unwrap();
        doc.set("/Ammo/10mm Ammo", 200.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let back = parse(&out).unwrap();
        assert_eq!(back.view, doc.view);

        // A copy left behind on another map: shown, not editable.
        let stale = parse(&sample(false)).unwrap();
        assert_eq!(stale.view.pointer("/Save/Live player"), Some(&false.into()));
        assert_eq!(stale.view.pointer("/JC Denton/Credits"), Some(&450.into()));
        assert!(stale.editable.is_empty());
    }

    /// Research aid: DEUSEX_SAVES = save folders (Save####) separated by `|` (or a .txt file
    /// listing them). The map named
    /// in SaveInfo.dxs must hold the live JC (every other map a stale copy); each file reads,
    /// writes back unchanged, and the live one takes an edit (credits +1) that reads back.
    #[test]
    #[ignore]
    fn real_saves() {
        let arg = std::env::var("DEUSEX_SAVES").unwrap();
        let list = if arg.ends_with(".txt") { std::fs::read_to_string(&arg).unwrap().replace('\n', "|") } else { arg };
        for dir in list.split('|').map(str::trim).filter(|s| !s.is_empty()) {
            let info = parse(&std::fs::read(format!("{dir}/SaveInfo.dxs")).unwrap()).unwrap();
            assert_eq!(info.kind_name, "Deus Ex save info");
            let map = info.view.pointer("/Save/Map").and_then(Value::as_str).unwrap().to_lowercase();
            let mut lives = 0;
            for e in std::fs::read_dir(dir).unwrap() {
                let path = e.unwrap().path();
                let file = path.file_name().unwrap().to_string_lossy().to_string();
                if file == "SaveInfo.dxs" || !file.ends_with(".dxs") {
                    continue;
                }
                let d = std::fs::read(&path).unwrap();
                assert!(looks(&d), "{file}");
                let mut doc = parse(&d).unwrap_or_else(|e| panic!("{dir}/{file}: {e}"));
                let live = doc.view.pointer("/Save/Live player") == Some(&true.into());
                assert_eq!(live, file[..file.len() - 4].to_lowercase() == map, "{dir}/{file}");
                assert_eq!(doc.to_bytes().unwrap(), d);
                if !live {
                    continue;
                }
                lives += 1;
                let g = |p: &str| doc.view.pointer(p).cloned().unwrap_or(Value::Null);
                println!("{dir} {file}: credits {} sp {} energy {} health {} ammo {} items {} skills {} augs {}", g("/JC Denton/Credits"), g("/JC Denton/Skill points"), g("/JC Denton/Bioelectric energy"), g("/Health"), g("/Ammo"), g("/Items"), g("/Skills"), g("/Augmentations"));
                let credits = g("/JC Denton/Credits").as_i64().unwrap();
                doc.set("/JC Denton/Credits", (credits + 1).into()).unwrap();
                let out = doc.to_bytes().unwrap();
                assert_eq!(out.iter().zip(&d).filter(|(a, b)| a != b).count() <= 4, true);
                let back = parse(&out).unwrap();
                assert_eq!(back.view, doc.view);
            }
            assert_eq!(lives, 1, "{dir}");
        }
    }
}
