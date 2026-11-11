//! XCOM 2 / War of the Chosen game data (what's left once the save's LZO chunks are
//! unpacked): an `XComGameStateHistory`. Each state object is written as its class
//! index, -1, sometimes a few native bytes, then Unreal Engine 3 property tags (name,
//! type, size, array index, value) ending at "None". Names are an FString plus a number.
//!
//! Objects are found with one forward pass: wherever a tag list parses all the way to
//! "None" and has an `ObjectID`, that's an object, and the pass carries on after it.
//! Only numbers that are already written can be changed (UE3 leaves default values
//! out), and they're changed in place, so nothing moves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

/// Resources HQ holds as items, in the order the screen shows them.
pub const RESOURCES: [(&str, &str); 6] = [
    ("Supplies", "Supplies"),
    ("Intel", "Intel"),
    ("AlienAlloy", "Alien Alloys"),
    ("EleriumDust", "Elerium Crystals"),
    ("EleriumCore", "Elerium Cores"),
    ("AbilityPoint", "Ability Points"),
];

fn i32_at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
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
        Some((
            s[..n - 1]
                .iter()
                .map(|&c| super::bethesda_cp1252(c))
                .collect(),
            at + 4 + n,
        ))
    } else {
        let n = n.unsigned_abs() as usize;
        if n > 4096 {
            return None;
        }
        let s = d.get(at + 4..at + 4 + 2 * n)?;
        let u: Vec<u16> = s
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&x| x != 0)
            .collect();
        Some((String::from_utf16_lossy(&u), at + 4 + 2 * n))
    }
}

/// An FName: FString plus its number ("Soldier", 0). Names with a number show as `name_N-1`
/// the way Unreal prints them.
fn fname(d: &[u8], at: usize) -> Option<(String, usize)> {
    let (s, q) = fstring(d, at)?;
    let n = i32_at(d, q)?;
    Some((if n > 0 { format!("{s}_{}", n - 1) } else { s }, q + 4))
}

/// A property name or type: a plain identifier with number 0.
fn ident(d: &[u8], at: usize) -> Option<(String, usize)> {
    // Quick check before building a string: most positions fail here.
    let n = i32_at(d, at)?;
    if !(2..80).contains(&n) {
        return None;
    }
    let (s, q) = fstring(d, at)?;
    let ok = s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_');
    (ok && i32_at(d, q)? == 0).then_some((s, q + 4))
}

#[derive(Debug, Clone)]
struct Prop {
    name: String,
    ty: String,
    #[allow(dead_code)]
    index: i32,
    /// Where the value starts and its size (BoolProperty: 1).
    at: usize,
    size: usize,
    /// StructProperty: its own tags; ArrayProperty of structs: one list per element.
    sub: Vec<Vec<Prop>>,
}

impl Prop {
    fn int(&self, d: &[u8]) -> Option<i32> {
        (self.ty == "IntProperty" && self.size == 4)
            .then(|| i32_at(d, self.at))
            .flatten()
    }
    fn name_value(&self, d: &[u8]) -> Option<String> {
        match self.ty.as_str() {
            "NameProperty" | "ByteProperty" => fname(d, self.at).map(|x| x.0),
            "StrProperty" => fstring(d, self.at).map(|x| x.0),
            _ => None,
        }
    }
}

fn get<'p>(props: &'p [Prop], name: &str) -> Option<&'p Prop> {
    props.iter().find(|p| p.name == name)
}

/// Reads tags from `at` up to and including "None"; returns them and where they end.
fn tags(d: &[u8], at: usize, limit: usize, depth: usize) -> Option<(Vec<Prop>, usize)> {
    let mut out = vec![];
    let mut p = at;
    loop {
        if p >= limit || out.len() > 5000 {
            return None;
        }
        let (name, q) = ident(d, p)?;
        if name == "None" {
            return Some((out, q));
        }
        let (ty, q) = ident(d, q)?;
        if !ty.ends_with("Property") {
            return None;
        }
        let size = i32_at(d, q)?;
        let index = i32_at(d, q + 4)?;
        if size < 0 || index < 0 {
            return None;
        }
        let size = size as usize;
        let mut v = q + 8;
        if ty == "StructProperty" || ty == "ByteProperty" {
            v = ident(d, v)?.1;
        }
        let end = if ty == "BoolProperty" {
            v + 1
        } else {
            v + size
        };
        if end > limit {
            return None;
        }
        let mut sub = vec![];
        if depth < 8 {
            if ty == "StructProperty" && ident(d, v).is_some() {
                if let Some((s, e)) = tags(d, v, end, depth + 1) {
                    if e == end {
                        sub.push(s);
                    }
                }
            } else if ty == "ArrayProperty" && size > 4 {
                let count = i32_at(d, v)?.max(0) as usize;
                let mut e = v + 4;
                if ident(d, e).is_some() {
                    for _ in 0..count.min(5000) {
                        match tags(d, e, end, depth + 1) {
                            Some((s, n)) => {
                                sub.push(s);
                                e = n;
                            }
                            None => {
                                sub.clear();
                                break;
                            }
                        }
                    }
                }
            }
        }
        let size = if ty == "BoolProperty" { 1 } else { size };
        out.push(Prop {
            name,
            ty,
            index: index as i32,
            at: v,
            size,
            sub,
        });
        p = end;
    }
}

/// State objects: (ObjectID, tags), the newest copy of each.
fn objects(d: &[u8]) -> Vec<(i32, Vec<Prop>)> {
    let mut found: Vec<(i32, Vec<Prop>)> = vec![];
    let mut at = 0;
    while at + 16 < d.len() {
        if ident(d, at).is_some() {
            if let Some((props, end)) = tags(d, at, d.len(), 0) {
                if props.len() >= 3 {
                    if let Some(id) = get(&props, "ObjectID").and_then(|p| p.int(d)) {
                        found.push((id, props));
                        at = end;
                        continue;
                    }
                }
            }
        }
        at += 1;
    }
    // A history can keep older copies of an object: the last one is the current state.
    let mut last: HashMap<i32, usize> = HashMap::new();
    for (i, (id, _)) in found.iter().enumerate() {
        last.insert(*id, i);
    }
    found
        .into_iter()
        .enumerate()
        .filter(|(i, (id, _))| last[id] == *i)
        .map(|(_, x)| x)
        .collect()
}

pub fn looks(d: &[u8]) -> bool {
    let head = &d[..d.len().min(1 << 20)];
    d.len() > 4096
        && find(head, b"XComGameState").is_some()
        && find(d, b"m_TemplateName\0").is_some()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Stat names as the game's eStat_ values, shortened.
fn stat_label(s: &str) -> String {
    let s = s.strip_prefix("eStat_").unwrap_or(s);
    match s {
        "HP" => "HP".into(),
        "Offense" => "Aim".into(),
        "Will" => "Will".into(),
        "Mobility" => "Mobility".into(),
        "Defense" => "Defense".into(),
        "Dodge" => "Dodge".into(),
        "Hacking" => "Hacking".into(),
        "PsiOffense" => "Psi".into(),
        "ArmorMitigation" => "Armor".into(),
        "ShieldHP" => "Shield".into(),
        "SightRadius" => "Sight".into(),
        other => other.to_string(),
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let objs = objects(d);
    if objs.is_empty() {
        return Err("no XCOM 2 state objects found".into());
    }
    let by_id: HashMap<i32, &Vec<Prop>> = objs.iter().map(|(id, p)| (*id, p)).collect();
    let template = |p: &[Prop]| get(p, "m_TemplateName").and_then(|t| t.name_value(d));
    let mut b = Builder::new(d);
    b.info(&["Objects read"], objs.len());

    // HQ: the object with an Inventory and HighestSoldierRank.
    let hq = objs
        .iter()
        .find(|(_, p)| get(p, "Inventory").is_some() && get(p, "HighestSoldierRank").is_some());
    if let Some((_, hq)) = hq {
        let inv = get(hq, "Inventory")
            .map(|p| p.sub.clone())
            .unwrap_or_default();
        let mut have: HashMap<String, (i32, usize)> = HashMap::new();
        for el in &inv {
            let Some(id) = get(el, "ObjectID").and_then(|p| p.int(d)) else {
                continue;
            };
            let Some(item) = by_id.get(&id) else { continue };
            let Some(t) = template(item) else { continue };
            if let Some(q) = get(item, "Quantity").filter(|q| q.int(d).is_some()) {
                have.entry(t).or_insert((id, q.at));
            }
        }
        for (t, label) in RESOURCES {
            if let Some(&(_, at)) = have.get(t) {
                b.ranged(&["Resources", label], at, Kind::I32, 0, i32::MAX as i64);
            }
        }
        // The rest of what HQ holds, by template name.
        let mut rest: Vec<(&String, &(i32, usize))> = have
            .iter()
            .filter(|(t, _)| !RESOURCES.iter().any(|r| r.0 == t.as_str()))
            .collect();
        rest.sort();
        for (t, &(_, at)) in rest {
            b.ranged(&["Storage", t], at, Kind::I32, 0, i32::MAX as i64);
        }
    }

    // Soldiers.
    let mut n = 0;
    for (id, p) in &objs {
        if template(p).as_deref() != Some("Soldier") {
            continue;
        }
        let text = |k: &str| get(p, k).and_then(|x| x.name_value(d)).unwrap_or_default();
        let key = format!(
            "{:04} {}",
            id,
            [text("strFirstName"), text("strLastName")].join(" ").trim()
        );
        let path = |f: &str| vec!["Soldiers".to_string(), key.clone(), f.to_string()];
        let put = |b: &mut Builder, f: &str, v: String| {
            let p = path(f);
            let r: Vec<&str> = p.iter().map(String::as_str).collect();
            b.info(&r, v);
        };
        put(&mut b, "First name", text("strFirstName"));
        put(&mut b, "Last name", text("strLastName"));
        put(&mut b, "Nickname", text("strNickName"));
        put(&mut b, "Class", text("m_SoldierClassTemplateName"));
        for (tag, label, kind) in [
            ("m_SoldierRank", "Rank", Kind::I32),
            ("iNumMissions", "Missions", Kind::I32),
            ("KillCount", "Kills", Kind::F32),
            ("SpentAbilityPoints", "Spent ability points", Kind::I32),
            ("AbilityPoints", "Ability points", Kind::I32),
        ] {
            if let Some(x) = get(p, tag) {
                let want = if kind == Kind::I32 {
                    "IntProperty"
                } else {
                    "FloatProperty"
                };
                if x.ty == want && x.size == 4 {
                    let pp = path(label);
                    let r: Vec<&str> = pp.iter().map(String::as_str).collect();
                    if kind == Kind::I32 {
                        b.ranged(&r, x.at, kind, 0, i32::MAX as i64);
                    } else {
                        b.field(&r, x.at, kind);
                    }
                }
            }
        }
        // CharacterStats[i]: { Type, CurrentValue, MaxValue, BaseMaxValue }. The editor shows
        // MaxValue; Current and BaseMax follow it.
        for st in p
            .iter()
            .filter(|x| x.name == "CharacterStats" && x.ty == "StructProperty")
        {
            let Some(s) = st.sub.first() else { continue };
            let Some(ty) = get(s, "Type").and_then(|t| t.name_value(d)) else {
                continue;
            };
            let floats = |k: &str| {
                get(s, k)
                    .filter(|v| v.ty == "FloatProperty" && v.size == 4)
                    .map(|v| v.at)
            };
            let Some(max) = floats("MaxValue") else {
                continue;
            };
            let pp = vec![
                "Soldiers".to_string(),
                key.clone(),
                "Stats".to_string(),
                stat_label(&ty),
            ];
            let r: Vec<&str> = pp.iter().map(String::as_str).collect();
            b.field(&r, max, Kind::F32);
            for k in ["CurrentValue", "BaseMaxValue"] {
                if let Some(at) = floats(k) {
                    b.mirror(&r, at);
                }
            }
        }
        n += 1;
    }
    if n == 0 && hq.is_none() {
        return Err("no XCOM 2 HQ or soldiers found".into());
    }
    Ok(b.finish("XCOM 2 game data", Checksum::None).with_note(
        "HQ resources and soldiers (rank, kills, missions, stats) can be changed. A stat sets its current, maximum and base \
         value together. Values the game left at their default aren't written in the save, so they can't be changed here.",
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn fstr(o: &mut Vec<u8>, s: &str) {
        o.extend(((s.len() + 1) as i32).to_le_bytes());
        o.extend(s.as_bytes());
        o.push(0);
    }
    fn name(o: &mut Vec<u8>, s: &str) {
        fstr(o, s);
        o.extend(0i32.to_le_bytes());
    }
    fn head(o: &mut Vec<u8>, n: &str, ty: &str, size: usize, extra: Option<&str>) {
        name(o, n);
        name(o, ty);
        o.extend((size as i32).to_le_bytes());
        o.extend(0i32.to_le_bytes());
        if let Some(e) = extra {
            name(o, e);
        }
    }
    pub fn int(o: &mut Vec<u8>, n: &str, v: i32) {
        head(o, n, "IntProperty", 4, None);
        o.extend(v.to_le_bytes());
    }
    fn float(o: &mut Vec<u8>, n: &str, v: f32) {
        head(o, n, "FloatProperty", 4, None);
        o.extend(v.to_le_bytes());
    }
    fn name_prop(o: &mut Vec<u8>, n: &str, v: &str) {
        let mut x = vec![];
        name(&mut x, v);
        head(o, n, "NameProperty", x.len(), None);
        o.extend(x);
    }
    fn str_prop(o: &mut Vec<u8>, n: &str, v: &str) {
        let mut x = vec![];
        fstr(&mut x, v);
        head(o, n, "StrProperty", x.len(), None);
        o.extend(x);
    }
    fn none(o: &mut Vec<u8>) {
        name(o, "None");
    }
    fn object(o: &mut Vec<u8>, body: Vec<u8>) {
        o.extend(7i32.to_le_bytes());
        o.extend((-1i32).to_le_bytes());
        o.extend(body);
        none(o);
    }

    /// A small synthetic history: HQ, two resource items, a soldier with two stats.
    pub fn sample() -> Vec<u8> {
        let mut d = b"XComGameStateHistory XComGameState_HeadquartersXCom ".to_vec();
        d.extend([0u8; 64]);
        let item = |id: i32, t: &str, q: i32| {
            let mut x = vec![];
            int(&mut x, "Quantity", q);
            name_prop(&mut x, "m_TemplateName", t);
            int(&mut x, "ObjectID", id);
            x
        };
        let supplies = item(11, "Supplies", 250);
        let intel = item(12, "Intel", 40);
        let mut hq = vec![];
        let mut arr = 2i32.to_le_bytes().to_vec();
        for id in [11, 12] {
            int(&mut arr, "ObjectID", id);
            none(&mut arr);
        }
        head(&mut hq, "Inventory", "ArrayProperty", arr.len(), None);
        hq.extend(arr);
        int(&mut hq, "HighestSoldierRank", 3);
        int(&mut hq, "ObjectID", 2);
        let mut sol = vec![];
        str_prop(&mut sol, "strFirstName", "Ana");
        str_prop(&mut sol, "strLastName", "Test");
        name_prop(&mut sol, "m_TemplateName", "Soldier");
        name_prop(&mut sol, "m_SoldierClassTemplateName", "Ranger");
        int(&mut sol, "m_SoldierRank", 2);
        float(&mut sol, "KillCount", 5.0);
        int(&mut sol, "iNumMissions", 4);
        for (i, (t, v)) in [("eStat_HP", 6.0f32), ("eStat_Offense", 70.0)]
            .iter()
            .enumerate()
        {
            let mut s = vec![];
            let mut e = vec![];
            name(&mut e, t);
            head(
                &mut s,
                "Type",
                "ByteProperty",
                e.len(),
                Some("ECharStatType"),
            );
            s.extend(e);
            float(&mut s, "CurrentValue", *v);
            float(&mut s, "MaxValue", *v);
            float(&mut s, "BaseMaxValue", *v);
            none(&mut s);
            name(&mut sol, "CharacterStats");
            name(&mut sol, "StructProperty");
            sol.extend((s.len() as i32).to_le_bytes());
            sol.extend((i as i32).to_le_bytes());
            name(&mut sol, "CharacterStat");
            sol.extend(s);
        }
        int(&mut sol, "ObjectID", 20);
        for body in [hq, supplies, intel, sol] {
            object(&mut d, body);
        }
        d.extend([0u8; 4096]);
        d
    }

    #[test]
    fn reads_hq_and_soldiers() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Resources/Supplies"), Some(&250.into()));
        assert_eq!(v.pointer("/Resources/Intel"), Some(&40.into()));
        let s = v.pointer("/Soldiers/0020 Ana Test").unwrap();
        assert_eq!(s["Class"], "Ranger");
        assert_eq!(s["Rank"], 2);
        assert_eq!(s["Missions"], 4);
        assert_eq!(s["Stats"]["Aim"], 70.0);
        doc.set("/Resources/Supplies", 999.into()).unwrap();
        doc.set("/Soldiers/0020 Ana Test/Stats/HP", 10.0.into())
            .unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let doc2 = parse(&out).unwrap();
        assert_eq!(doc2.view.pointer("/Resources/Supplies"), Some(&999.into()));
        assert_eq!(
            doc2.view.pointer("/Soldiers/0020 Ana Test/Stats/HP"),
            Some(&10.0.into())
        );
        // Current and base max follow: all three 10.0 now.
        let ten = 10.0f32.to_le_bytes();
        assert_eq!(out.windows(4).filter(|w| *w == ten).count(), 3);
    }

    /// Research aid: SAVE_EDITOR_XCOM2 = unpacked XCOM 2 data.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_XCOM2").unwrap()).unwrap();
        let t = std::time::Instant::now();
        let doc = parse(&d).unwrap();
        println!(
            "{:?}\n{}",
            t.elapsed(),
            serde_json::to_string_pretty(&doc.view)
                .unwrap()
                .chars()
                .take(6000)
                .collect::<String>()
        );
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}
