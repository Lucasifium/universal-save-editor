//! XCOM: Enemy Unknown / Enemy Within game data (what's left once the save's LZO chunks
//! are unpacked): a list of saved objects, each written as its full name, its instance
//! name, a location and rotation, its class, the length of its properties and then
//! Unreal Engine 3 property tags (name, type, size, array index, value).
//!
//! Every object's numbers and ticks become editable in place. Text, enums and object
//! references are shown but left alone, so nothing changes size.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

const MARK: &[u8] = b":PersistentLevel.";

struct P<'a, 'b> {
    b: &'b mut Builder<'a>,
}

fn fstring(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?);
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
        Some((s[..n - 1].iter().map(|&c| super::bethesda_cp1252(c)).collect(), at + 4 + n))
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

/// A property name or type: plain ASCII identifier.
fn ident(d: &[u8], at: usize) -> Option<(String, usize)> {
    let (s, end) = fstring(d, at)?;
    let ok = !s.is_empty() && s.len() < 80 && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_');
    ok.then_some((s, end))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

impl P<'_, '_> {
    /// Reads a property list starting at `at`, ending at its "None"; returns where it ends.
    /// `path` is where its values go in the view; `limit` the end of the enclosing data.
    fn list(&mut self, at: usize, limit: usize, path: &[String], depth: usize) -> Option<usize> {
        let d = self.b.d;
        let mut p = at;
        let mut seen: HashMap<String, usize> = HashMap::new();
        // Static arrays repeat a name with rising indexes: count names first.
        let mut items: Vec<(String, String, usize, u32, usize)> = vec![]; // name, type, value at, index, size
        let mut extra: Vec<Option<String>> = vec![];
        loop {
            if p >= limit || items.len() > 20_000 {
                return None;
            }
            let (name, q) = ident(d, p)?;
            if u32_at(d, q)? != 0 {
                return None;
            }
            if name == "None" {
                p = q + 4;
                break;
            }
            let (ty, q) = ident(d, q + 4)?;
            if !ty.ends_with("Property") || u32_at(d, q)? != 0 {
                return None;
            }
            let size = u32_at(d, q + 4)? as usize;
            let index = u32_at(d, q + 8)?;
            let mut v = q + 12;
            let mut ex = None;
            match ty.as_str() {
                "StructProperty" => {
                    let (s, e) = ident(d, v)?;
                    ex = Some(s);
                    v = e + 4;
                }
                "ByteProperty" => {
                    let (s, e) = ident(d, v)?;
                    ex = Some(s);
                    v = e + 4;
                }
                _ => {}
            }
            let end = if ty == "BoolProperty" { v + 1 } else { v + size };
            if end > limit {
                return None;
            }
            items.push((name, ty, v, index, size));
            extra.push(ex);
            p = end;
        }
        for (name, ..) in &items {
            *seen.entry(name.clone()).or_default() += 1;
        }
        let mut used: HashMap<String, usize> = HashMap::new();
        for ((name, ty, v, index, size), ex) in items.into_iter().zip(extra) {
            let mut key = if seen[&name] > 1 { format!("{name}[{index}]") } else { name.clone() };
            let n = used.entry(key.clone()).or_default();
            *n += 1;
            if *n > 1 {
                key = format!("{key} ({n})");
            }
            let mut here: Vec<String> = path.to_vec();
            here.push(key);
            let refs: Vec<&str> = here.iter().map(String::as_str).collect();
            match ty.as_str() {
                "IntProperty" if size == 4 => {
                    self.b.field(&refs, v, Kind::I32);
                }
                "FloatProperty" if size == 4 => {
                    self.b.field(&refs, v, Kind::F32);
                }
                "BoolProperty" => {
                    self.b.field(&refs, v, Kind::B8);
                }
                "StrProperty" | "NameProperty" => {
                    if let Some((s, _)) = fstring(d, v) {
                        self.b.info(&refs, s);
                    }
                }
                "ByteProperty" => {
                    if ex.as_deref() == Some("None") && size == 1 {
                        self.b.field(&refs, v, Kind::U8);
                    } else if let Some((s, _)) = fstring(d, v) {
                        self.b.info(&refs, s);
                    }
                }
                "StructProperty" if depth < 12 => {
                    // Simple structs (vectors…) have no tags inside; those are skipped.
                    let mut sub = self.b.d.len();
                    sub = sub.min(v + size);
                    if ident(d, v).is_some() {
                        let _ = self.list(v, sub, &here, depth + 1);
                    }
                }
                // Storage: how many of each item type HQ holds.
                "ArrayProperty" if name == "m_arrItems" && size >= 4 && u32_at(d, v)? as usize * 4 + 4 == size => {
                    let count = u32_at(d, v)? as usize;
                    for i in 0..count {
                        let mut el = here.clone();
                        el.push(format!("Item {i}"));
                        let r: Vec<&str> = el.iter().map(String::as_str).collect();
                        self.b.ranged(&r, v + 4 + 4 * i, Kind::I32, 0, i32::MAX as i64);
                    }
                }
                "ArrayProperty" if depth < 12 && size >= 4 => {
                    // Arrays of structs: each element is a property list.
                    let count = u32_at(d, v)? as usize;
                    let end = v + size;
                    let mut q = v + 4;
                    if count > 0 && ident(d, q).is_some() {
                        for i in 0..count.min(5000) {
                            let mut el = here.clone();
                            el.push(i.to_string());
                            match self.list(q, end, &el, depth + 1) {
                                Some(e) => q = e,
                                None => break,
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Some(p)
    }
}

/// Saved objects: (instance name, class, where the properties start, their length).
fn objects(d: &[u8]) -> Vec<(String, String, usize, usize)> {
    let mut out = vec![];
    let mut from = 0;
    while let Some(i) = find(&d[from..], MARK).map(|i| i + from) {
        from = i + MARK.len();
        // The full name is an FString: its length is somewhere just before the mark.
        let lo = i.saturating_sub(200);
        let Some(start) = (lo..i.saturating_sub(3)).rev().find(|&s| {
            u32_at(d, s).is_some_and(|n| n > 0 && n < 400 && s + 4 + n as usize > i && d.get(s + 3 + n as usize) == Some(&0))
        }) else {
            continue;
        };
        let Some((_full, p)) = fstring(d, start) else { continue };
        let Some((inst, p)) = fstring(d, p) else { continue };
        let p = p + 24;
        let Some((class, p)) = fstring(d, p) else { continue };
        let Some(len) = u32_at(d, p) else { continue };
        let props = p + 4;
        if inst.is_empty() || class.is_empty() || props + len as usize > d.len() {
            continue;
        }
        out.push((inst, class, props, len as usize));
        from = props + len as usize;
    }
    out
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn looks(d: &[u8]) -> bool {
    // XCOM 2 also has these names, but keeps a copy of every object for every turn
    // ("XComGameState_…"): editing one copy wouldn't be safe, so it isn't read here.
    let head = &d[..d.len().min(1 << 20)];
    d.len() > 4096 && find(head, b"XComGame").is_some() && find(head, b"XComGameState").is_none() && find(d, MARK).is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let objs = objects(d);
    if objs.len() < 3 {
        return Err("no XCOM objects found".into());
    }
    let mut b = Builder::new(d);
    let mut read = 0;
    for (inst, class, props, len) in &objs {
        let base = vec!["Objects".to_string(), inst.clone()];
        let short = class.rsplit('.').next().unwrap_or(class).to_string();
        b.info(&["Objects", inst, "(class)"], short);
        let mut p = P { b: &mut b };
        if p.list(*props, props + len, &base, 0).is_some() {
            read += 1;
        }
    }
    if read == 0 {
        return Err("no XCOM objects could be read".into());
    }
    Ok(b.finish("XCOM: Enemy Unknown game data", Checksum::None).with_note(
        "Numbers and ticks of every saved object can be edited (soldier stats, money…); names and other text are shown as they are.",
    ))
}

#[cfg(test)]
mod tests {
    /// Research aid: SAVE_EDITOR_XCOM = unpacked XCOM EU data.
    #[test]
    #[ignore]
    fn real_save() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_XCOM").unwrap()).unwrap();
        let t = std::time::Instant::now();
        let doc = super::parse(&d).unwrap();
        println!("{:?} {} editable", t.elapsed(), doc.editable.len());
        let objs = doc.view.pointer("/Objects").and_then(|v| v.as_object()).unwrap();
        println!("{} objects", objs.len());
        if let Some(hq) = doc.view.pointer("/Objects/XGHeadQuarters_0/m_iCash") {
            println!("cash {hq}");
        }
        let mut classes = std::collections::BTreeMap::new();
        for (k, v) in objs {
            *classes.entry(v["(class)"].as_str().unwrap_or("").to_string()).or_insert(0) += 1;
            if std::env::var("SHOW").is_ok_and(|s| k.starts_with(&s)) {
                println!("{k}: {}", serde_json::to_string(v).unwrap().chars().take(4000).collect::<String>());
            }
        }
        println!("{classes:?}");
        if let Some(s) = doc.view.pointer("/Objects/XGStrategySoldier_1") {
            println!("{}", serde_json::to_string_pretty(s).unwrap().chars().take(3000).collect::<String>());
        }
        assert_eq!(doc.to_bytes().unwrap(), d);
    }
}
