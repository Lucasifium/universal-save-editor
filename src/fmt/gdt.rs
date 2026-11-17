//! Game Dev Tycoon: the game is a web app, so its saves live in the browser-style Local Storage
//! database (`file__0.localstorage`, an SQLite file with one `ItemTable` of key/value rows).
//! Every `slot_<id>` row holds one company's whole game as JSON text in UTF-16; a row named
//! after the same id (and the auto-save's `auto`) holds the short summary the load menu shows.
//! Cash, fans and research points are edited in the slot's JSON text and mirrored in the summary.

use super::sqlite::Temp;
use crate::tree::{int_in, Backend, TreeDoc};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Game Dev Tycoon save";

/// (label in the view, key inside `company`, lowest accepted value, highest accepted value).
const FIELDS: [(&str, &str, i128, i128, bool); 5] = [
    // Cash keeps fractions once the company earns money; the others are whole numbers.
    ("Cash", "cash", -9_000_000_000_000_000, 9_000_000_000_000_000, false),
    ("Fans", "fans", 0, 9_000_000_000_000_000, true),
    ("Research points", "researchPoints", 0, 999_999_999, true),
    ("Staff limit", "maxStaff", 1, 99, true),
    ("Company level", "currentLevel", 1, 99, true),
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Wide {
    /// Plain UTF-16LE (what the game's own runtime writes).
    Plain,
    /// A leading 0x00 byte, then UTF-16LE.
    Prefixed,
}

fn decode(b: &[u8]) -> Option<(Wide, String)> {
    let (wide, body) = match b.first() {
        Some(0) => (Wide::Prefixed, &b[1..]),
        _ => (Wide::Plain, b),
    };
    if body.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = body.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16(&units).ok().map(|s| (wide, s))
}

fn encode(wide: Wide, s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    if wide == Wide::Prefixed {
        out.push(0);
    }
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

#[derive(Clone, Debug)]
struct Leaf {
    /// The `slot_…` key of the row.
    key: String,
    field: &'static str,
    lo: i128,
    hi: i128,
    whole: bool,
}

#[derive(Clone, Debug)]
struct Gdt {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

fn set_number(o: &mut Map<String, Value>, field: &str, v: &Value) {
    if o.contains_key(field) {
        o.insert(field.to_string(), v.clone());
    }
}

impl Backend for Gdt {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let tmp = Temp::new(&self.raw)?;
        {
            let conn = Connection::open(&tmp.0).map_err(|e| e.to_string())?;
            let err = |e: rusqlite::Error| e.to_string();
            conn.execute_batch("BEGIN").map_err(err)?;
            let mut keys: Vec<&str> = changes.iter().map(|(l, _)| self.leaves[*l].key.as_str()).collect();
            keys.dedup();
            for key in keys {
                let summary_key = key.strip_prefix("slot_").unwrap_or(key);
                for (row_key, in_company) in [(key, true), (summary_key, false)] {
                    let blob: Option<Vec<u8>> =
                        conn.query_row("SELECT value FROM ItemTable WHERE key = ?1", [row_key], |r| r.get(0)).ok();
                    let Some(blob) = blob else { continue };
                    let Some((wide, text)) = decode(&blob) else { continue };
                    let mut json: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                    {
                        let obj = if in_company { json.get_mut("company").and_then(Value::as_object_mut) } else { json.as_object_mut() };
                        let Some(obj) = obj else { continue };
                        for (l, v) in changes {
                            let leaf = &self.leaves[*l];
                            if leaf.key == key {
                                set_number(obj, leaf.field, v);
                            }
                        }
                    }
                    let out = serde_json::to_string(&json).map_err(|e| e.to_string())?;
                    conn.execute("UPDATE ItemTable SET value = ?1 WHERE key = ?2", rusqlite::params![encode(wide, &out), row_key]).map_err(err)?;
                }
            }
            conn.execute_batch("COMMIT").map_err(err)?;
        }
        std::fs::read(&tmp.0).map_err(|e| e.to_string())
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let l = &self.leaves[leaf];
        if l.whole {
            return int_in(v, l.lo, l.hi, "This number").map(|_| ());
        }
        match v.as_f64() {
            Some(f) if f.is_finite() && f >= l.lo as f64 && f <= l.hi as f64 => Ok(()),
            Some(_) => Err(format!("This number must be between {} and {}", l.lo, l.hi)),
            None => Err("This number must be a number".into()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Reads the slot rows: (key, parsed company object).
fn slots(d: &[u8]) -> Option<Vec<(String, Value)>> {
    let tmp = Temp::new(d).ok()?;
    let conn = Connection::open_with_flags(&tmp.0, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    let mut st = conn.prepare("SELECT key, value FROM ItemTable WHERE key LIKE 'slot\\_%' ESCAPE '\\' ORDER BY key").ok()?;
    let rows: Vec<(String, Vec<u8>)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).ok()?.flatten().collect();
    let mut out = Vec::new();
    for (key, blob) in rows {
        let Some((_, text)) = decode(&blob) else { continue };
        let Ok(json) = serde_json::from_str::<Value>(&text) else { continue };
        // Only a whole game counts: knowledge and error-log rows share the prefix.
        let ok = json.get("company").and_then(|c| c.get("cash")).is_some_and(Value::is_number);
        if ok {
            out.push((key, json["company"].clone()));
        }
    }
    drop(st);
    drop(conn);
    Some(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"SQLite format 3\0") && slots(d).is_some_and(|s| !s.is_empty())
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let found = slots(d).filter(|s| !s.is_empty()).ok_or("not a Game Dev Tycoon save")?;
    let mut rows = Vec::new();
    let mut leaves = Vec::new();
    let mut editable = HashMap::new();
    for (i, (key, company)) in found.iter().enumerate() {
        let mut m = Map::new();
        m.insert("Company".into(), company.get("name").cloned().unwrap_or(Value::Null));
        let slot = key.strip_prefix("slot_").unwrap_or(key);
        m.insert("Save".into(), Value::String(slot.to_string()));
        for (label, field, lo, hi, whole) in FIELDS {
            let Some(v) = company.get(field).filter(|v| v.is_number()) else { continue };
            m.insert(label.into(), v.clone());
            // A whole-number field holding a fraction is left read-only rather than rounded.
            if !whole || v.as_i64().is_some() {
                editable.insert(format!("/Saves/{i}/{label}"), leaves.len());
                leaves.push(Leaf { key: key.clone(), field, lo, hi, whole });
            }
        }
        rows.push(Value::Object(m));
    }
    let mut root = Map::new();
    root.insert("Saves".into(), Value::Array(rows));
    let note = crate::i18n::tx("Close the game first. A company's cash, fans and research points are changed in its saved game and in the load menu's summary. Level and staff limit change what the game reads, not what it unlocks.");
    Ok(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(Gdt { raw: d.to_vec(), leaves })).with_note(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(rows: &[(&str, &str)]) -> Vec<u8> {
        let p = std::env::temp_dir().join(format!("gdt_test_{}_{}.sqlite", std::process::id(), rows.len()));
        let _ = std::fs::remove_file(&p);
        {
            let c = Connection::open(&p).unwrap();
            c.execute_batch("CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB NOT NULL ON CONFLICT FAIL)").unwrap();
            for (k, v) in rows {
                c.execute("INSERT INTO ItemTable VALUES (?1, ?2)", rusqlite::params![k, encode(Wide::Plain, v)]).unwrap();
            }
        }
        let d = std::fs::read(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        d
    }

    const SLOT: &str = r#"{"version":8,"slot":2,"company":{"uid":"u1","name":"Acme","cash":70000,"fans":12,"researchPoints":40,"maxStaff":4,"currentLevel":2,"currentWeek":3.5,"cashLog":[1,2]},"flags":{}}"#;
    const SUMMARY: &str = r#"{"name":"Acme","cash":70000,"fans":12,"currentWeek":3.5}"#;

    #[test]
    fn reads_edits_both_rows_and_leaves_the_rest() {
        let d = db(&[("slot_u1", SLOT), ("u1", SUMMARY), ("slot_knowledge", r#"{"combos":[]}"#), ("muted", "false")]);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Saves/0/Cash"), Some(&Value::from(70000)));
        assert_eq!(doc.view.pointer("/Saves/0/Company"), Some(&Value::from("Acme")));
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Saves/0/Cash", Value::from(999_999_999)).unwrap();
        doc.set("/Saves/0/Fans", Value::from(5_000_000)).unwrap();
        assert!(doc.set("/Saves/0/Research points", Value::from(-1)).is_err());
        let out = doc.to_bytes().unwrap();
        let re = parse(&out).unwrap();
        assert_eq!(re.view.pointer("/Saves/0/Cash"), Some(&Value::from(999_999_999)));
        let tmp = Temp::new(&out).unwrap();
        let c = Connection::open(&tmp.0).unwrap();
        let get = |k: &str| -> String { decode(&c.query_row("SELECT value FROM ItemTable WHERE key = ?1", [k], |r| r.get::<_, Vec<u8>>(0)).unwrap()).unwrap().1 };
        assert_eq!(get("slot_u1"), SLOT.replace("70000", "999999999").replace("\"fans\":12", "\"fans\":5000000"));
        assert_eq!(get("u1"), SUMMARY.replace("70000", "999999999").replace("\"fans\":12", "\"fans\":5000000"));
        assert_eq!(get("slot_knowledge"), r#"{"combos":[]}"#);
    }

    #[test]
    fn other_databases_are_not_this_game() {
        assert!(!looks(&db(&[("muted", "false")])));
        assert!(!looks(b"not sqlite"));
    }

    /// `GDT_SAVE=<file__0.localstorage> cargo test --release --lib gdt -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_save_is_recognised() {
        let Ok(p) = std::env::var("GDT_SAVE") else { return };
        let d = std::fs::read(p).unwrap();
        assert!(looks(&d));
        let doc = parse(&d).unwrap();
        println!("{}", doc.view);
        assert_eq!(doc.to_bytes().unwrap(), d, "unchanged open writes the file back byte for byte");
    }
}
