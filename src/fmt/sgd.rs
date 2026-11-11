//! Techland "SGD" saves (Dying Light 2 / The Beast), after the gzip layer: sections of
//! class descriptions (`SGDd`: class name, then each field's name and type code) followed
//! by objects (`SGDs`: object id, class number, then the fields in order). Every block
//! states its size, so a field type we don't know only stops that one object.
//!
//! Type codes were worked out from sample saves (sizes checked against every block):
//! 2 = u16, 3/9 = f32, 5/6/8 = 32-bit integers, 7 = i64, 0x0b = bool,
//! 0x0c = string (u16 length), 0x0e = vector (3×f32), 0x14 = enum (u32 + its name),
//! 0x55 = object reference, 0x56 = list of references, 0x1e = list of strings.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

fn u16_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?) as usize)
}
fn u32_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

fn string(d: &[u8], o: usize) -> Option<(String, usize)> {
    let n = u16_at(d, o)?;
    let s = d.get(o + 2..o + 2 + n)?;
    Some((String::from_utf8_lossy(s).into_owned(), o + 2 + n))
}

pub fn looks(d: &[u8]) -> bool {
    let head = &d[..d.len().min(4096)];
    head.windows(4).any(|w| w == b"SGDd") && head.windows(10).any(|w| w == b"Savegame::")
}

enum Val {
    Num(Kind),
    Text(String),
    Skip,
}

/// One field's value at `o`: what it is and where the next field starts.
fn field(d: &[u8], ty: u16, o: usize) -> Option<(Val, usize)> {
    Some(match ty {
        2 => (Val::Num(Kind::U16), o + 2),
        3 | 9 => (Val::Num(Kind::F32), o + 4),
        5 | 8 => (Val::Num(Kind::I32), o + 4),
        6 => (Val::Num(Kind::U32), o + 4),
        7 => (Val::Skip, o + 8),
        0x0b => (Val::Num(Kind::B8), o + 1),
        0x0c => {
            let (s, e) = string(d, o)?;
            (Val::Text(s), e)
        }
        0x0e => (Val::Skip, o + 12),
        0x14 => {
            let (s, e) = string(d, o + 4)?;
            (Val::Text(s), e)
        }
        0x55 => (Val::Skip, o + 8),
        0x56 => (Val::Skip, o + 4 + 8 * u32_at(d, o)?),
        0x1e => {
            let n = u32_at(d, o)?;
            let mut e = o + 4;
            for _ in 0..n {
                e = string(d, e)?.1;
            }
            (Val::Skip, e)
        }
        _ => return None,
    })
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Techland SGD save".into());
    }
    let mut b = Builder::new(d);
    let mut classes: HashMap<usize, (String, Vec<(String, u16)>)> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut used: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    let mut objects = 0;
    let mut p = 0;
    while let Some(i) = d[p..].windows(3).position(|w| w == b"SGD").map(|i| i + p) {
        p = i + 4;
        let tag = d.get(i + 3).copied();
        let (Some(size), Some(body)) = (u32_at(d, i + 8), Some(i + 12)) else { continue };
        let end = body + size;
        if end > d.len() {
            continue;
        }
        match tag {
            Some(b'd') => {
                let Some(idx) = u32_at(d, i + 12) else { continue };
                let Some((name, mut q)) = string(d, i + 24) else { continue };
                let Some(n) = u32_at(d, q) else { continue };
                q += 4;
                let mut fields = vec![];
                for _ in 0..n {
                    let Some((f, e)) = string(d, q) else { break };
                    let Some(t) = u16_at(d, e) else { break };
                    fields.push((f, t as u16));
                    q = e + 2;
                }
                // Numbering starts again with each section's descriptions.
                if idx == 0 {
                    classes.clear();
                }
                classes.insert(idx, (name, fields));
                p = end;
            }
            Some(b's') => {
                let Some(c) = u32_at(d, body + 8) else { continue };
                let Some((cname, fields)) = classes.get(&c) else { continue };
                // Objects with a name (skills, buffs, items) are listed under it.
                let mut q = body + 16;
                let mut vals: Vec<(String, Val, usize)> = vec![];
                for (f, t) in fields {
                    if q >= end {
                        break;
                    }
                    let Some((v, e)) = field(d, *t, q) else { break };
                    if e > end {
                        break;
                    }
                    vals.push((f.clone(), v, q));
                    q = e;
                }
                let label = vals.iter().find_map(|(f, v, _)| match v {
                    Val::Text(s) if (f == "m_NameId" || f == "m_Type" || f == "m_Name") && !s.is_empty() => Some(s.clone()),
                    _ => None,
                });
                let short = cname.rsplit("::").next().unwrap_or(cname).to_string();
                let n = counts.entry(short.clone()).or_default();
                *n += 1;
                let mut key = match label {
                    Some(l) => l,
                    None => format!("#{n}"),
                };
                // Two objects with the same name keep apart.
                let mut k = 2;
                while !used.insert((short.clone(), key.clone())) {
                    key = format!("{} ({k})", key.trim_end_matches(&format!(" ({})", k - 1)));
                    k += 1;
                }
                let mut any = false;
                for (f, v, at) in vals {
                    let name = f.trim_start_matches("m_").to_string();
                    match v {
                        // Ids (slot ids, seeds) are kept as they are: changing them breaks links.
                        Val::Num(_) if name.ends_with("Id") || name == "Seed" => {}
                        Val::Num(k) => {
                            b.field(&[&short, &key, &name], at, k);
                            any = true;
                        }
                        Val::Text(s) => b.info(&[&short, &key, &name], s),
                        Val::Skip => {}
                    }
                }
                if any {
                    objects += 1;
                }
                p = end;
            }
            _ => {}
        }
    }
    if objects == 0 {
        return Err("no objects could be read".into());
    }
    Ok(b.finish("Dying Light save", Checksum::None).with_note(
        "Every saved object's numbers, by class (money is in Group \u{203A} Main \u{203A} Cash). Text and links between \
         objects are shown but not changed.",
    ))
}

#[cfg(test)]
mod tests {
    /// Research aid: SAVE_EDITOR_SGD = an unpacked Dying Light 2 save.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_SGD").unwrap()).unwrap();
        let doc = super::parse(&d).unwrap();
        println!("{} editable; classes {:?}", doc.editable.len(), doc.view.as_object().unwrap().keys().take(60).collect::<Vec<_>>());
        println!("cash {:?}", doc.view.pointer("/Group/Main/Cash"));
        println!("{:?}", doc.view.pointer("/Bucket"));
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}

#[cfg(test)]
mod peek {
    #[test]
    #[ignore]
    fn items() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_SGD").unwrap()).unwrap();
        let doc = super::parse(&d).unwrap();
        for k in ["Item", "ItemContainer", "HealthState", "State", "QuickSlots"] {
            let v = doc.view.get(k).map(|v| serde_json::to_string(v).unwrap()).unwrap_or_default();
            println!("{k}: {}", v.chars().take(700).collect::<String>());
        }
    }
}
