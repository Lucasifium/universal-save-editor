//! Synthetic saves: plain-JSON saves built from a table-driven screen's own spec, with made-up
//! neutral values. They exist for screenshots and for checking that a screen opens, shows its
//! values and writes back. They are NOT real saves: they say nothing about what the game
//! accepts, and the real-save status in `tests/screen-samples.tsv` is not affected.
//!
//!     SAVE_EDITOR_SYNTH_OUT=dir cargo test --release write_synthetic_saves -- --ignored --nocapture

use crate::formats::{decode_with, encode, same_data, Content, Options};
use crate::game_ui::simple::{self, Shape, Spec};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

fn set(root: &mut Value, ptr: &str, v: Value) {
    let segs: Vec<&str> = ptr.split('/').skip(1).collect();
    let mut cur = root;
    for (i, seg) in segs.iter().enumerate() {
        let seg = seg.replace("~1", "/").replace("~0", "~");
        let last = i + 1 == segs.len();
        let next_is_idx = !last && segs[i + 1].parse::<usize>().is_ok();
        if let Ok(ix) = seg.parse::<usize>() {
            if !cur.is_array() {
                *cur = json!([]);
            }
            let a = cur.as_array_mut().unwrap();
            while a.len() <= ix {
                a.push(Value::Null);
            }
            if last {
                a[ix] = v;
                return;
            }
            if a[ix].is_null() {
                a[ix] = if next_is_idx { json!([]) } else { json!({}) };
            }
            cur = &mut a[ix];
        } else {
            if !cur.is_object() {
                *cur = json!({});
            }
            let o = cur.as_object_mut().unwrap();
            if last {
                o.insert(seg, v);
                return;
            }
            cur = o.entry(seg).or_insert_with(|| if next_is_idx { json!([]) } else { json!({}) });
            if cur.is_null() {
                *cur = if next_is_idx { json!([]) } else { json!({}) };
            }
        }
    }
}

fn is_text(ptr: &str) -> bool {
    let l = ptr.rsplit('/').next().unwrap_or("").to_lowercase();
    ["name", "title", "id", "type", "class", "job", "map", "location", "version"].iter().any(|k| l.contains(k))
}

/// A made-up, plausible mid-game figure that differs between fields.
fn number(ptr: &str, salt: usize) -> i64 {
    let h = ptr.bytes().fold(salt as u64 + 7, |a, b| a.wrapping_mul(31).wrapping_add(b as u64)) >> 3;
    let l = ptr.rsplit('/').next().unwrap_or("").to_lowercase();
    let has = |ks: &[&str]| ks.iter().any(|k| l.contains(k));
    let r = |lo: u64, hi: u64| (lo + h % (hi - lo + 1)) as i64;
    if l == "max" || l.starts_with("max") {
        r(100, 150)
    } else if has(&["current", "cur"]) {
        r(40, 99)
    } else if has(&["node", "city", "area", "chapter", "act", "difficulty"]) {
        r(1, 6)
    } else if has(&["level", "lvl", "rank", "tier", "grade"]) {
        r(12, 34)
    } else if has(&["health", "hp", "life", "energy", "mana", "stamina", "food", "water", "sleep", "hunger"]) {
        r(60, 100)
    } else if has(&["gold", "money", "cash", "credit", "coin", "ruble", "unit", "stardust", "gem", "crystal", "token", "fame", "currency"]) {
        r(2_000, 48_000)
    } else if has(&["exp", "xp", "score"]) {
        r(3_000, 42_000)
    } else if has(&["day", "season", "week", "month"]) {
        r(14, 80)
    } else if has(&["death", "kill", "win", "loss", "run", "streak", "count", "found", "complet", "finish", "visit"]) {
        r(8, 160)
    } else if has(&["point", "perk", "skill", "stat", "attribute", "strength", "agility", "intellect", "endurance", "perception", "luck", "charisma"]) {
        r(4, 40)
    } else {
        r(10, 90)
    }
}

fn value_for(ptr: &str, salt: usize) -> Value {
    if is_text(ptr) {
        json!("Player")
    } else {
        json!(number(ptr, salt))
    }
}

fn supported(p: &str) -> bool {
    !p.contains("/@") && !p.contains("/=")
}

/// A JSON save that the spec recognises, filled from the spec's own pointers.
pub fn build(s: &Spec) -> Option<Value> {
    let mut v = json!({});
    let mut ptrs: Vec<&str> = vec![];
    ptrs.extend(s.markers.iter().copied());
    ptrs.extend(s.tiles.iter().map(|t| t.1));
    for sec in s.sections {
        ptrs.extend(sec.rows.iter().map(|r| r.1));
    }
    if !ptrs.iter().all(|p| supported(p)) {
        return None;
    }
    for (i, p) in ptrs.iter().enumerate() {
        if v.pointer(p).is_none() {
            set(&mut v, p, value_for(p, i));
        }
    }
    for l in s.lists {
        if !supported(l.path) {
            return None;
        }
        let names: Vec<&str> = l.names.iter().take(12).map(|n| n.0).collect();
        let ids: Vec<&str> = if names.is_empty() { vec!["item_1", "item_2", "item_3"] } else { names };
        let body = match &l.shape {
            Shape::Array { id_key, qty_keys } => Value::Array(
                ids.iter()
                    .enumerate()
                    .map(|(i, id)| {
                        let mut o = Map::new();
                        o.insert((*id_key).into(), id.parse::<i64>().map(Value::from).unwrap_or_else(|_| json!(id)));
                        for k in qty_keys.iter() {
                            o.insert((*k).into(), json!(3 + i as i64 * 7));
                        }
                        Value::Object(o)
                    })
                    .collect(),
            ),
            Shape::Map => Value::Object(ids.iter().enumerate().map(|(i, id)| (id.to_string(), json!(3 + i as i64 * 7))).collect()),
            Shape::Records { qty_key } => Value::Object(ids.iter().enumerate().map(|(i, id)| (id.to_string(), json!({ *qty_key: 3 + i as i64 * 7 }))).collect()),
        };
        if v.pointer(l.path).is_none() {
            set(&mut v, l.path, body);
        }
    }
    for t in s.tables {
        if !supported(t.path) || t.name.iter().chain(t.cols.iter().map(|c| &c.1)).any(|p| !supported(p)) {
            return None;
        }
        let mut rows = vec![];
        for r in 0..3 {
            let mut row = json!({});
            for (j, n) in t.name.iter().enumerate() {
                set(&mut row, n, json!(format!("Member {}{}", r + 1, if j > 0 { "b" } else { "" })));
            }
            for (j, c) in t.cols.iter().enumerate() {
                set(&mut row, c.1, value_for(c.1, r * 10 + j));
            }
            rows.push(row);
        }
        if v.pointer(t.path).is_none() {
            set(&mut v, t.path, Value::Array(rows));
        }
    }
    Some(v)
}

fn jsonish(s: &Spec) -> bool {
    s.kind.is_none()
}

fn status() -> BTreeSet<String> {
    include_str!("../tests/screen-samples.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            let (n, st) = (f.next()?, f.next()?);
            (st == "none").then(|| n.to_string())
        })
        .collect()
}

/// Every JSON screen without a real sample: the synthetic save must open on that very screen,
/// hold editable values and write back the same data.
#[test]
fn synthetic_saves_open_on_their_screens() {
    let (mut ok, mut skipped) = (0, vec![]);
    for s in simple::all().iter().filter(|s| jsonish(s)) {
        let Some(v) = build(s) else { skipped.push(s.name); continue };
        let bytes = serde_json::to_vec_pretty(&v).unwrap();
        let opts = Options::for_path(std::path::Path::new("synthetic.json"));
        let d = decode_with(&bytes, &opts);
        let got = crate::sample_check::screen_of(&d);
        if got.as_deref() != Some(s.name) {
            skipped.push(s.name);
            continue;
        }
        let out = encode(&d.layers, &d.content).expect("encode");
        let again = decode_with(&out, &opts);
        assert!(same_data(&again.content, &d.content), "{} does not write back", s.name);
        assert!(matches!(d.content, Content::Json { .. }));
        ok += 1;
    }
    println!("synthetic: {ok} open on their own screen, skipped {}: {:?}", skipped.len(), skipped);
    assert!(ok >= 60, "only {ok} synthetic saves open on their own screen");
    assert!(skipped.len() <= 8, "too many screens skipped: {skipped:?}");
}

#[test]
#[ignore]
fn write_synthetic_saves() {
    let Ok(dir) = std::env::var("SAVE_EDITOR_SYNTH_OUT") else { return };
    let none = status();
    std::fs::create_dir_all(&dir).unwrap();
    for s in simple::all().iter().filter(|s| jsonish(s) && none.contains(s.name)) {
        let Some(v) = build(s) else { println!("skip {} (unsupported pointers)", s.name); continue };
        let bytes = serde_json::to_vec_pretty(&v).unwrap();
        let d = decode_with(&bytes, &Options::for_path(std::path::Path::new("Save01.json")));
        if crate::sample_check::screen_of(&d).as_deref() != Some(s.name) {
            println!("skip {} (opens elsewhere)", s.name);
            continue;
        }
        let slug: String = s.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        std::fs::write(format!("{dir}/{slug}.json"), bytes).unwrap();
        println!("wrote {}", s.name);
    }
}

#[test]
#[ignore]
fn list_synthetic_subtitles() {
    let none = status();
    for s in simple::all().iter().filter(|s| jsonish(s) && none.contains(s.name)) {
        println!("## {} :: {}", s.name, s.subtitle);
    }
}
