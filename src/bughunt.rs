//! Stress test over sample saves (ignored by default):
//!
//!     SAVE_EDITOR_SCAN=list.txt cargo test --release bug_hunt -- --ignored --nocapture
//!
//! For every file: decode, re-encode unchanged, edit a spread of values of every kind
//! (numbers, text, switches, drop-down choices), save, re-open and compare, then
//! feed truncated / corrupted copies to the decoder to make sure nothing panics.

use crate::formats::{decode_with, describe, encode, same_layers, Content, Decoded, Options};
use serde_json::Value;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;

/// A different but plausible value for an edit test.
fn tweak(v: &Value, choices: Option<&Vec<String>>, attempt: usize) -> Option<Value> {
    if let (Some(list), Value::String(cur)) = (choices, v) {
        return list.iter().filter(|c| *c != cur).nth(attempt * 7 % list.len().max(1)).map(|c| Value::from(c.clone()));
    }
    Some(match v {
        Value::Bool(b) => Value::Bool(!b),
        Value::Number(n) => match (n.as_i64(), n.as_u64(), n.as_f64()) {
            (Some(i), _, _) => Value::from(match attempt {
                0 => i.saturating_add(1),
                1 => i.saturating_sub(1),
                _ => 1,
            }),
            (_, Some(u), _) => Value::from(u - 1),
            (_, _, Some(f)) => crate::tree::num(if attempt == 0 { f + 0.5 } else { f / 2.0 }),
            _ => return None,
        },
        Value::String(s) => Value::from(match attempt {
            0 => format!("{s}Z"),
            1 if !s.is_empty() => {
                let mut c: Vec<char> = s.chars().collect();
                let last = c.len() - 1;
                c[last] = if c[last] == 'A' { 'B' } else { 'A' };
                c.into_iter().collect::<String>()
            }
            _ => "A".to_string(),
        }),
        _ => return None,
    })
}

fn root(c: &Content) -> Option<&Value> {
    match c {
        Content::Json { value, .. } => Some(value),
        Content::Tree(t) => Some(&t.view),
        _ => None,
    }
}

/// Re-open edited bytes and list pointers whose values differ from what we set.
fn verify(bytes: &[u8], opts: &Options, d: &Decoded, ptrs: &[String]) -> Result<(), String> {
    let again = decode_with(bytes, opts);
    if !same_layers(&again.layers, &d.layers) {
        return Err(format!("layers changed: {}", describe(&again.layers, &again.content)));
    }
    let (Some(a), Some(b)) = (root(&d.content), root(&again.content)) else {
        return Err(format!("re-opened as {}", again.content.kind()));
    };
    let bad: Vec<String> = ptrs
        .iter()
        .filter(|p| a.pointer(p) != b.pointer(p))
        .take(3)
        .map(|p| format!("{p}: set {:?}, read back {:?}", a.pointer(p), b.pointer(p)))
        .collect();
    if !bad.is_empty() {
        return Err(bad.join(" | "));
    }
    if a != b {
        // Something we didn't touch changed (or a key was renamed).
        let diff = first_diff(a, b, String::new()).unwrap_or_default();
        return Err(format!("other data differs after save: {diff}"));
    }
    Ok(())
}

fn first_diff(a: &Value, b: &Value, at: String) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                match y.get(k) {
                    Some(w) => {
                        if let Some(d) = first_diff(v, w, format!("{at}/{k}")) {
                            return Some(d);
                        }
                    }
                    None => return Some(format!("{at}/{k} missing after save (keys now: {:?})", y.keys().take(4).collect::<Vec<_>>())),
                }
            }
            (x.len() != y.len()).then(|| format!("{at}: {} keys before, {} after", x.len(), y.len()))
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{at}: {} items before, {} after", x.len(), y.len()));
            }
            x.iter().zip(y).enumerate().find_map(|(i, (v, w))| first_diff(v, w, format!("{at}/{i}")))
        }
        _ => (a != b).then(|| format!("{at}: {a} vs {b}")),
    }
}

fn edit_test(raw: &[u8], opts: &Options, report: &mut Vec<String>) {
    let d = decode_with(raw, opts);
    if let Content::Tree(t) = &d.content {
        let orphans: Vec<&String> = t.editable.keys().filter(|p| t.view.pointer(p).is_none()).take(3).collect();
        if !orphans.is_empty() {
            report.push(format!("EDITABLE FIELDS MISSING FROM VIEW: {orphans:?}"));
        }
        let containers = t.editable.keys().filter(|p| t.view.pointer(p).is_some_and(|v| v.is_object() || v.is_array())).count();
        if containers > 0 {
            report.push(format!("EDITABLE ENTRIES THAT ARE NOT VALUES: {containers}"));
        }
    }
    let leaves: Vec<String> = match &d.content {
        Content::Tree(t) => {
            let mut v: Vec<String> = t.editable.keys().filter(|p| t.view.pointer(p).is_some()).cloned().collect();
            v.sort();
            v
        }
        Content::Json { value, .. } => {
            let mut out = Vec::new();
            fn walk(v: &Value, p: String, out: &mut Vec<String>) {
                match v {
                    Value::Object(m) => m.iter().for_each(|(k, c)| walk(c, format!("{p}/{}", k.replace('~', "~0").replace('/', "~1")), out)),
                    Value::Array(a) => a.iter().enumerate().for_each(|(i, c)| walk(c, format!("{p}/{i}"), out)),
                    Value::Null => {}
                    _ => out.push(p),
                }
            }
            walk(value, String::new(), &mut out);
            out
        }
        _ => return,
    };
    if leaves.is_empty() {
        return;
    }
    // Unchanged save must verify too.
    match encode(&d.layers, &d.content) {
        Ok(b) => {
            if let Err(e) = verify(&b, opts, &d, &[]) {
                report.push(format!("UNCHANGED SAVE FAILS: {e}"));
                return;
            }
            if b != raw && d.layers.is_empty() {
                report.push(format!("note: unchanged save not byte-identical ({} vs {} bytes)", b.len(), raw.len()));
            }
        }
        Err(e) => {
            report.push(format!("UNCHANGED ENCODE ERROR: {e}"));
            return;
        }
    }
    // Pick leaves spread across the document (fewer for big files: each edit re-opens it).
    let n = match raw.len() {
        0..=1_000_000 => 150,
        1_000_001..=5_000_000 => 30,
        _ => 8,
    };
    let step = (leaves.len() / n).max(1);
    let picks: Vec<&String> = leaves.iter().step_by(step).take(n).collect();
    let mut batch = decode_with(raw, opts);
    let mut set_ptrs = Vec::new();
    let mut refused = 0;
    let mut singles_failed = 0;
    for p in &picks {
        // Single edit on a fresh copy.
        let mut single = decode_with(raw, opts);
        let mut done = false;
        for attempt in 0..3 {
            let (cur, choices) = match &single.content {
                Content::Tree(t) => (t.view.pointer(p).cloned(), t.choices(p).map(|c| (*c).clone())),
                Content::Json { value, .. } => (value.pointer(p).cloned(), None),
                _ => (None, None),
            };
            let Some(cur) = cur else { break };
            let Some(nv) = tweak(&cur, choices.as_ref(), attempt) else { break };
            let ok = match &mut single.content {
                Content::Tree(t) => t.set(p, nv.clone()).is_ok(),
                Content::Json { value, .. } => {
                    *value.pointer_mut(p).unwrap() = nv.clone();
                    true
                }
                _ => false,
            };
            if !ok {
                continue;
            }
            done = true;
            match encode(&single.layers, &single.content) {
                Ok(b) => {
                    if let Err(e) = verify(&b, opts, &single, &[p.to_string()]) {
                        singles_failed += 1;
                        if singles_failed <= 4 {
                            report.push(format!("EDIT FAILS VERIFY {p} = {nv}: {e}"));
                        }
                    }
                }
                Err(e) => {
                    singles_failed += 1;
                    if singles_failed <= 4 {
                        report.push(format!("EDIT ENCODE ERROR {p} = {nv}: {e}"));
                    }
                }
            }
            // (A format may have moved the value, e.g. an item to the first free slot.)
            // Same edit into the batch document.
            if let Content::Tree(t) = &mut batch.content {
                // Only what the batch document itself offers (its drop-downs leave out
                // items another slot already holds there).
                let offered = t.choices(p).is_none_or(|c| c.contains(&nv.as_str().unwrap_or_default().to_string()) || !nv.is_string());
                if offered && t.set(p, nv.clone()).is_ok() {
                    t.take_derived();
                    set_ptrs.push(p.to_string());
                }
            } else if let Content::Json { value, .. } = &mut batch.content {
                *value.pointer_mut(p).unwrap() = nv;
                set_ptrs.push(p.to_string());
            }
            break;
        }
        if !done {
            refused += 1;
            if std::env::var("VERBOSE").is_ok() && refused <= 6 {
                report.push(format!("refused: {p}"));
            }
        }
    }
    if singles_failed > 4 {
        report.push(format!("... {singles_failed} single edits failed in total"));
    }
    match encode(&batch.layers, &batch.content) {
        Ok(b) => {
            if let Err(e) = verify(&b, opts, &batch, &set_ptrs) {
                report.push(format!("BATCH OF {} EDITS FAILS: {e}", set_ptrs.len()));
            }
        }
        Err(e) => report.push(format!("BATCH ENCODE ERROR: {e}")),
    }
    report.push(format!("ok-ish: {} leaves, tested {}, refused {refused}", leaves.len(), picks.len()));
}

fn fuzz(raw: &[u8], opts: &Options, report: &mut Vec<String>) {
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for frac in [0.0, 0.01, 0.1, 0.33, 0.5, 0.9, 0.99] {
        cases.push(raw[..(raw.len() as f64 * frac) as usize].to_vec());
    }
    if raw.len() > 1 {
        cases.push(raw[..raw.len() - 1].to_vec());
    }
    let mut seed = raw.len() as u64 ^ 0x9E37_79B9_7F4A_7C15;
    for _ in 0..12 {
        let mut c = raw.to_vec();
        if c.is_empty() {
            break;
        }
        for _ in 0..3 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let i = (seed as usize) % c.len();
            c[i] ^= (seed >> 32) as u8 | 1;
        }
        cases.push(c);
    }
    for (i, c) in cases.iter().enumerate() {
        let r = catch_unwind(AssertUnwindSafe(|| {
            let d = decode_with(c, opts);
            // Saving a damaged-but-opened file must not panic either.
            let _ = encode(&d.layers, &d.content);
        }));
        if let Err(e) = r {
            let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            report.push(format!("PANIC on damaged copy #{i} ({} bytes): {msg}", c.len()));
            return;
        }
    }
}

#[test]
#[ignore]
fn bug_hunt() {
    std::panic::set_hook(Box::new(|_| {}));
    let list = std::fs::read_to_string(std::env::var("SAVE_EDITOR_SCAN").expect("SAVE_EDITOR_SCAN")).unwrap();
    let mut problems = 0;
    for path in list.lines().map(|l| l.trim().trim_start_matches('\u{feff}')).filter(|l| !l.is_empty()) {
        let Ok(raw) = std::fs::read(path) else { continue };
        if raw.len() > 40 << 20 {
            continue;
        }
        let opts = Options::for_path(std::path::Path::new(path));
        let mut report = Vec::new();
        let t0 = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(|| decode_with(&raw, &opts)));
        let Ok(d) = r else {
            println!("PANIC decoding {path}");
            problems += 1;
            continue;
        };
        let kind = describe(&d.layers, &d.content);
        let secs = t0.elapsed().as_secs_f32();
        if secs > 3.0 {
            report.push(format!("SLOW: opening took {secs:.1}s"));
        }
        if let Err(e) = catch_unwind(AssertUnwindSafe(|| edit_test(&raw, &opts, &mut report))) {
            let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            report.push(format!("PANIC during edit test: {msg}"));
        }
        if raw.len() < 6 << 20 {
            fuzz(&raw, &opts, &mut report);
        }
        let bad = report.iter().filter(|l| l.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && !l.starts_with("ok")).count();
        problems += bad;
        use std::io::Write;
        let _ = std::io::stdout().flush();
        eprintln!("[{:>5.1}s] {kind}  {path}", t0.elapsed().as_secs_f32());
        if bad > 0 || std::env::var("VERBOSE").is_ok() {
            println!("== {kind}  [{path}]");
            for l in &report {
                println!("   {l}");
            }
        }
    }
    println!("\nPROBLEMS: {problems}");
}

/// How long the Values view takes to build for the biggest saves.
#[test]
#[ignore]
fn view_build_timing() {
    for path in std::env::var("SAVE_EDITOR_BIG").expect("SAVE_EDITOR_BIG").split(';') {
        let raw = std::fs::read(path).unwrap();
        let t0 = Instant::now();
        let d = decode_with(&raw, &Options::for_path(std::path::Path::new(path)));
        let t1 = t0.elapsed();
        let names = crate::names::Names::default();
        if let Content::Tree(t) = &d.content {
            let t2 = Instant::now();
            let model = crate::values_view::build(&t.view, &names, Some(&t.editable));
            let b = t2.elapsed();
            let t3 = Instant::now();
            let disp = crate::values_view::filter(&model, &t.view, "money", true, &Default::default());
            println!("{path}: decode {t1:?}, build {b:?} ({} rows), search {:?} ({} hits)", model.rows.len(), t3.elapsed(), disp.len());
        }
    }
}

/// Research aid: writes every decrypted FromSoftware slot of SAVE_EDITOR_SL2 (`;`-separated)
/// into SAVE_EDITOR_OUT as `<save name>.<entry>.bin`.
#[test]
#[ignore]
fn dump_fromsoft_slots() {
    let out = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_OUT").expect("SAVE_EDITOR_OUT"));
    std::fs::create_dir_all(&out).unwrap();
    for path in std::env::var("SAVE_EDITOR_SL2").expect("SAVE_EDITOR_SL2").split(';') {
        let raw = std::fs::read(path).unwrap();
        let stem = std::path::Path::new(path).file_stem().unwrap().to_string_lossy().into_owned();
        for e in crate::fmt::containers::bnd4_entries(&raw).expect("BND4") {
            if let Some((plain, key, _)) = crate::fmt::containers::bnd4_open(&raw, &e) {
                std::fs::write(out.join(format!("{stem}.{}.bin", e.name)), &plain).unwrap();
                println!("{stem} {} {} bytes, encrypted: {}", e.name, plain.len(), key.is_some());
            }
        }
    }
}

/// Research aid: prints the decoded view of each SAVE_EDITOR_VIEW file (`;`-separated),
/// abbreviated: long arrays show their first items only.
#[test]
#[ignore]
fn dump_views() {
    fn short(v: &Value, depth: usize) -> Value {
        match v {
            Value::Array(a) if depth > 0 => Value::Array(a.iter().take(3).map(|x| short(x, depth - 1)).chain(
                (a.len() > 3).then(|| Value::from(format!("… {} items", a.len())))).collect()),
            Value::Object(m) if depth > 0 => Value::Object(m.iter().take(std::env::var("KEYS").ok().and_then(|k| k.parse().ok()).unwrap_or(40)).map(|(k, x)| (k.clone(), short(x, depth - 1))).collect()),
            Value::Array(a) => Value::from(format!("[{} items]", a.len())),
            Value::Object(m) => Value::from(format!("{{{} keys}}", m.len())),
            x => x.clone(),
        }
    }
    let depth: usize = std::env::var("DEPTH").ok().and_then(|d| d.parse().ok()).unwrap_or(4);
    for path in std::env::var("SAVE_EDITOR_VIEW").expect("SAVE_EDITOR_VIEW").split(';') {
        let raw = std::fs::read(path).unwrap();
        let d = decode_with(&raw, &Options::for_path(std::path::Path::new(path)));
        let kind = match &d.content {
            Content::Tree(t) => t.kind_name.clone(),
            c => c.kind().to_string(),
        };
        println!("===== {path}
{kind}  layers {}", format!("{:?}", d.layers).chars().take(200).collect::<String>());
        if let (Ok(out), Some(v)) = (std::env::var("JSON_OUT"), root(&d.content)) {
            std::fs::write(out, serde_json::to_string_pretty(v).unwrap()).unwrap();
        }
        let at = std::env::var("PTR").unwrap_or_default();
        if let Some(v) = root(&d.content).and_then(|v| v.pointer(&at)) {
            println!("{}", serde_json::to_string_pretty(&short(v, depth)).unwrap());
        }
    }
}

/// Research aid: edits Sekiro items in SAVE_EDITOR_SEKIRO through the whole save pipeline.
#[test]
#[ignore]
fn sekiro_item_save() {
    let path = std::env::var("SAVE_EDITOR_SEKIRO").expect("SAVE_EDITOR_SEKIRO");
    let raw = std::fs::read(&path).unwrap();
    let mut d = decode_with(&raw, &Options::for_path(std::path::Path::new(&path)));
    let Content::Tree(t) = &mut d.content else { panic!("not a tree") };
    let items = t.view.pointer("/Items").unwrap().as_object().unwrap().clone();
    let (filled, _) = items.iter().find(|(_, v)| v["Item"] != "(empty)" && v["Item"] != "Spirit Emblem").unwrap();
    let (empty, _) = items.iter().find(|(_, v)| v["Item"] == "(empty)").unwrap();
    let _ = empty;
    let mut todo = vec![(format!("/Items/{filled}/Quantity"), Value::from(7))];
    // What the game screen's "Give upgrade materials" button asks for.
    let root = t.view.clone();
    let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: Some(&**t), edits: vec![], archive: None, open_entry: None };
    crate::game_ui::give_materials(crate::game_ui::Game::Sekiro, &mut cx);
    todo.extend(cx.edits);
    for (p, v) in todo {
        println!("set {p} = {v}: {:?}", t.set(&p, v.clone()).map(|_| ()));
    }
    match encode(&d.layers, &d.content) {
        Ok(out) => {
            println!("encoded {} bytes (orig {})", out.len(), raw.len());
            let back = decode_with(&out, &Options::for_path(std::path::Path::new(&path)));
            assert!(crate::formats::same_data(&back.content, &d.content), "the app's save check would fail");
            let Content::Tree(b) = &back.content else { panic!("re-read failed") };
            println!("re-read: {} = {}", filled, b.view.pointer(&format!("/Items/{filled}/Quantity")).unwrap());
            for (_, v) in b.view["Items"].as_object().unwrap() { if v["Quantity"] == 49 { println!("  {}", v["Item"]); } }
        }
        Err(e) => println!("ENCODE ERROR: {e}"),
    }
}

/// Research aid: edits Elden Ring items in SAVE_EDITOR_ER (every character slot) through
/// the whole save pipeline, including the "Max upgrade materials" button.
#[test]
#[ignore]
fn er_item_save() {
    let path = std::env::var("SAVE_EDITOR_ER").expect("SAVE_EDITOR_ER");
    let raw = std::fs::read(&path).unwrap();
    let entries = crate::fmt::containers::bnd4_entries(&raw).unwrap();
    for e in entries {
        let Some((plain, _, _)) = crate::fmt::containers::bnd4_open(&raw, &e) else { continue };
        let Ok(mut doc) = crate::fmt::fromsoft::parse(&plain) else { continue };
        let Some(items) = doc.view.pointer("/Items").and_then(Value::as_object).cloned() else {
            println!("{}: no item lists found", e.name);
            continue;
        };
        assert_eq!(doc.to_bytes().unwrap(), plain, "{} must round-trip", e.name);
        let goods = items.values().filter(|v| v["Item"] != "(empty)").count();
        let unknown = items.values().filter(|v| v["Item"].as_str().is_some_and(|s| s.starts_with("Item #"))).count();
        let (first, _) = items.iter().find(|(_, v)| v["Item"] != "(empty)").unwrap();
        doc.set(&format!("/Items/{first}/Quantity"), Value::from(3)).unwrap();
        let root = doc.view.clone();
        let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: Some(&doc), edits: vec![], archive: None, open_entry: None };
        crate::game_ui::give_materials(crate::game_ui::Game::EldenRing, &mut cx);
        let edits = cx.edits;
        let n = edits.len();
        for (p, v) in edits {
            doc.set(&p, v).unwrap();
        }
        // Weapons and armor: raise a weapon, add one of each.
        let weapons = doc.view["Weapons"].as_object().cloned().unwrap_or_default();
        let mut w_info = String::new();
        if let Some((k, w)) = weapons.iter().find(|(_, w)| w["Item"] != "(empty)" && w["Level"].as_u64().is_some_and(|l| l < 10)) {
            let r = doc.set(&format!("/Weapons/{k}/Level"), Value::from(w["Level"].as_u64().unwrap() + 1));
            w_info = format!("{} {}+1: {:?}; ", w["Item"], w["Level"], r.map(|_| ()));
        }
        let free_w = weapons.iter().find(|(_, w)| w["Item"] == "(empty)").map(|(k, _)| k.clone()).unwrap();
        let add_w = doc.set(&format!("/Weapons/{free_w}/Item"), Value::from("Uchigatana")).map(|_| ());
        let _ = doc.set(&format!("/Weapons/{free_w}/Level"), Value::from(25));
        let armor = doc.view["Armor"].as_object().cloned().unwrap_or_default();
        let free_a = armor.iter().find(|(_, a)| a["Item"] == "(empty)").map(|(k, _)| k.clone()).unwrap();
        let add_a = doc.set(&format!("/Armor/{free_a}/Item"), Value::from("White Mask")).map(|_| ());
        let count = |s: &str| doc.view[s].as_object().map_or(0, |m| m.values().filter(|v| v == &&Value::Bool(true)).count());
        println!("  graces {}, bosses {}, ashes {}", count("Sites of Grace"), count("Bosses defeated"),
            doc.view["Ashes of War"].as_object().map_or(0, |m| m.values().filter(|v| v["Item"] != "(empty)").count()));
        let free_s = doc.view["Ashes of War"].as_object().and_then(|m| m.iter().find(|(_, v)| v["Item"] == "(empty)").map(|(k, _)| k.clone())).unwrap();
        let add_s = doc.set(&format!("/Ashes of War/{free_s}/Item"), Value::from("Lion's Claw")).map(|_| ());
        let add_g = doc.set("/Sites of Grace/Table of Lost Grace ~1 Roundtable Hold", Value::Bool(true)).map(|_| ());
        println!("  {w_info}add Uchigatana +25: {add_w:?}; add White Mask: {add_a:?}; add Lion's Claw: {add_s:?}; grace: {add_g:?}");
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), plain.len());
        let back = crate::fmt::fromsoft::parse(&out).unwrap();
        assert_eq!(back.view, doc.view, "{}: the app's save check would fail", e.name);
        assert_eq!(back.view[&format!("Weapons")][&free_w]["Level"], 25);
        let changed = (0..out.len()).filter(|&i| out[i] != plain[i]).count();
        println!("{}: {} goods carried ({unknown} unnamed), key {}, storage {}; {n} edits -> {changed} bytes changed, reads back OK",
            e.name, goods, doc.view["Key items"].as_object().map_or(0, |m| m.len()), doc.view.get("Storage").and_then(Value::as_object).map_or(0, |m| m.len()));
    }
}

/// Research aid: edits Dark Souls III items in SAVE_EDITOR_DS3 (every character slot)
/// through the whole save pipeline.
#[test]
#[ignore]
fn ds3_item_save() {
    let path = std::env::var("SAVE_EDITOR_DS3").expect("SAVE_EDITOR_DS3");
    let raw = std::fs::read(&path).unwrap();
    for e in crate::fmt::containers::bnd4_entries(&raw).unwrap() {
        let Some((plain, _, _)) = crate::fmt::containers::bnd4_open(&raw, &e) else { continue };
        let Ok(mut doc) = crate::fmt::fromsoft::parse(&plain) else { continue };
        assert_eq!(doc.to_bytes().unwrap(), plain, "{} must round-trip", e.name);
        let show = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().map(|m| m.iter().filter(|(_, v)| v["Item"] != "(empty)").map(|(k, v)| format!("{k}: {} {}{}", v["Item"], v.get("Quantity").map_or(String::new(), |q| format!("x{q}")), v.get("Level").map_or(String::new(), |l| format!(" +{l} {}", v["Infusion"])))).collect::<Vec<_>>().join("; ")).unwrap_or_default();
        for s in ["Items", "Storage", "Weapons", "Armor", "Rings"] {
            println!("{} {s}: {}", e.name, show(&doc, s));
        }
        let free = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().unwrap().iter().find(|(_, v)| v["Item"] == "(empty)").map(|(k, _)| k.clone()).unwrap();
        let (w, a, r, g) = (free(&doc, "Weapons"), free(&doc, "Armor"), free(&doc, "Rings"), free(&doc, "Items"));
        let results = [
            doc.set(&format!("/Weapons/{w}/Item"), Value::from("Long Sword")).map(|_| ()),
            doc.set(&format!("/Weapons/{w}/Level"), Value::from(10)).map(|_| ()),
            doc.set(&format!("/Weapons/{w}/Infusion"), Value::from("Sharp")).map(|_| ()),
            doc.set(&format!("/Armor/{a}/Item"), Value::from("Black Iron Helm")).map(|_| ()),
            doc.set(&format!("/Rings/{r}/Item"), Value::from("Life Ring+3")).map(|_| ()),
            doc.set(&format!("/Items/{g}/Item"), Value::from("Titanite Shard")).map(|_| ()),
        ];
        println!("  edits: {results:?}");
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), plain.len());
        let back = crate::fmt::fromsoft::parse(&out).unwrap();
        assert_eq!(back.view, doc.view, "{}: the app's save check would fail", e.name);
        println!("  saved and read back: {}", show(&back, "Weapons"));
    }
}

/// Research aid: edits Dark Souls II items in SAVE_EDITOR_DS2 (every character slot)
/// through the whole save pipeline.
#[test]
#[ignore]
fn ds2_item_save() {
    let path = std::env::var("SAVE_EDITOR_DS2").expect("SAVE_EDITOR_DS2");
    let raw = std::fs::read(&path).unwrap();
    for e in crate::fmt::containers::bnd4_entries(&raw).unwrap() {
        let Some((plain, _, _)) = crate::fmt::containers::bnd4_open(&raw, &e) else { continue };
        let Ok(mut doc) = crate::fmt::fromsoft::parse(&plain) else { continue };
        assert_eq!(doc.to_bytes().unwrap(), plain, "{} must round-trip", e.name);
        let count = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().map_or(0, |m| m.values().filter(|v| v["Item"] != "(empty)").count());
        let unnamed = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().map_or(0, |m| m.values().filter(|v| v["Item"].as_str().is_some_and(|n| n.contains('#'))).count());
        for s in ["Items", "Item box", "Weapons", "Armor", "Rings", "Key items"] {
            println!("{} {s}: {} ({} unnamed)", e.name, count(&doc, s), unnamed(&doc, s));
        }
        let w = doc.view["Weapons"].as_object().unwrap().iter().filter(|(_, v)| v["Item"] != "(empty)").take(3).map(|(k, v)| format!("{k} {} +{} {}", v["Item"], v["Level"], v["Infusion"])).collect::<Vec<_>>();
        println!("  weapons: {w:?}");
        let free = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().unwrap().iter().find(|(_, v)| v["Item"] == "(empty)").map(|(k, _)| k.clone()).unwrap();
        let (fw, fa, fr, fg) = (free(&doc, "Weapons"), free(&doc, "Armor"), free(&doc, "Rings"), free(&doc, "Items"));
        let shard = doc.view["Items"].as_object().unwrap().iter().find(|(_, v)| v["Item"] == "Titanite Shard").map(|(k, _)| k.clone());
        let results = [
            doc.set(&format!("/Weapons/{fw}/Item"), Value::from("Broadsword")).map(|_| ()),
            doc.set(&format!("/Weapons/{fw}/Level"), Value::from(5)).map(|_| ()),
            doc.set(&format!("/Armor/{fa}/Item"), Value::from("Alva Helm")).map(|_| ()),
            doc.set(&format!("/Rings/{fr}/Item"), Value::from("Ring of Blades+1")).map(|_| ()),
            {
                let pick = doc.choices(&format!("/Items/{fg}/Item")).unwrap()[1].clone();
                doc.set(&format!("/Items/{fg}/Item"), Value::from(pick)).map(|_| ())
            },
            match &shard { Some(k) => doc.set(&format!("/Items/{k}/Quantity"), Value::from(99)).map(|_| ()), None => Ok(()) },
        ];
        println!("  edits: {results:?}");
        let out = doc.to_bytes().unwrap();
        let back = crate::fmt::fromsoft::parse(&out).unwrap();
        assert_eq!(back.view, doc.view, "{}: the app's save check would fail", e.name);
        let changed = (0..out.len()).filter(|&i| out[i] != plain[i]).count();
        println!("  saved: {changed} bytes changed, reads back OK; new weapon {}", back.view["Weapons"][&fw]);
    }
}

/// Research aid: reads and edits every Dark Souls Remastered character in SAVE_EDITOR_DS1
/// (adding and removing items) through the whole save pipeline.
#[test]
#[ignore]
fn ds1_item_save() {
    let path = std::env::var("SAVE_EDITOR_DS1").expect("SAVE_EDITOR_DS1");
    let raw = std::fs::read(&path).unwrap();
    for e in crate::fmt::containers::bnd4_entries(&raw).unwrap() {
        let Some((plain, _, _)) = crate::fmt::containers::bnd4_open(&raw, &e) else { println!("{}: not opened", e.name); continue };
        let mut doc = match crate::fmt::fromsoft::parse(&plain) {
            Ok(d) => d,
            Err(err) => { println!("{}: {err}", e.name); continue }
        };
        assert_eq!(doc.to_bytes().unwrap(), plain, "{} must round-trip", e.name);
        println!("{}: {} level {} souls {} class {}", e.name, doc.view["Character"]["Name"], doc.view["Character"]["Soul level"], doc.view["Character"]["Souls"], doc.view["Character"]["Starting class"]);
        let Some(sections) = doc.view.as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()) else { continue };
        let count = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().map_or(0, |m| m.values().filter(|v| v["Item"] != "(empty)").count());
        let unnamed = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().map_or(0, |m| m.values().filter(|v| v["Item"].as_str().is_some_and(|n| n.contains('#'))).count());
        for s in ["Items", "Key items", "Weapons", "Armor", "Rings"] {
            println!("  {s}: {} ({} unnamed)", count(&doc, s), unnamed(&doc, s));
        }
        if !sections.iter().any(|s| s == "Items") {
            continue;
        }
        let w = doc.view["Weapons"].as_object().unwrap().iter().filter(|(_, v)| v["Item"] != "(empty)").take(4).map(|(k, v)| format!("{k} {} +{} {}", v["Item"], v["Level"], v.get("Infusion").cloned().unwrap_or_default())).collect::<Vec<_>>();
        println!("  weapons: {w:?}");
        let items = doc.view["Items"].as_object().unwrap().iter().filter(|(_, v)| v["Item"] != "(empty)").take(6).map(|(k, v)| format!("{k} {} x{}", v["Item"], v["Quantity"])).collect::<Vec<_>>();
        println!("  items: {items:?}");
        println!("  key items: {:?}", doc.view["Key items"].as_object().unwrap().values().filter(|v| v["Item"] != "(empty)").map(|v| v["Item"].clone()).collect::<Vec<_>>());
        let free = |doc: &crate::tree::TreeDoc, s: &str| doc.view[s].as_object().unwrap().iter().find(|(_, v)| v["Item"] == "(empty)").map(|(k, _)| k.clone()).unwrap();
        let (fw, fa, fr, fg) = (free(&doc, "Weapons"), free(&doc, "Armor"), free(&doc, "Rings"), free(&doc, "Items"));
        let results = [
            doc.set(&format!("/Weapons/{fw}/Item"), Value::from("Longsword")).map(|_| ()),
            doc.set(&format!("/Weapons/{fw}/Level"), Value::from(5)).map(|_| ()),
            doc.set(&format!("/Armor/{fa}/Item"), Value::from("Elite Knight Helm")).map(|_| ()),
            doc.set(&format!("/Rings/{fr}/Item"), Value::from("Ring of Favor and Protection")).map(|_| ()),
            doc.set(&format!("/Items/{fg}/Item"), Value::from("Green Blossom")).map(|_| ()),
        ];
        println!("  add: {results:?}");
        let root = doc.view.clone();
        let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: Some(&doc), edits: vec![], archive: None, open_entry: None };
        crate::game_ui::give_materials(crate::game_ui::Game::DarkSouls1, &mut cx);
        let edits = cx.edits;
        let n = edits.len();
        let mut bad = 0;
        for (p, v) in edits {
            bad += doc.set(&p, v).is_err() as usize;
        }
        println!("  materials: {n} edits, {bad} refused");
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), plain.len());
        let back = crate::fmt::fromsoft::parse(&out).unwrap();
        assert_eq!(back.view, doc.view, "{}: the app's save check would fail", e.name);
        // Remove what was added again.
        let mut doc2 = back;
        let rm = [
            doc2.set(&format!("/Weapons/{fw}/Item"), Value::from("(empty)")).map(|_| ()),
            doc2.set(&format!("/Armor/{fa}/Item"), Value::from("(empty)")).map(|_| ()),
            doc2.set(&format!("/Rings/{fr}/Item"), Value::from("(empty)")).map(|_| ()),
            doc2.set(&format!("/Items/{fg}/Item"), Value::from("(empty)")).map(|_| ()),
        ];
        println!("  remove: {rm:?}");
        let out2 = doc2.to_bytes().unwrap();
        let back2 = crate::fmt::fromsoft::parse(&out2).unwrap();
        assert_eq!(back2.view, doc2.view, "{}: removal read-back", e.name);
        let changed = (0..out.len()).filter(|&i| out[i] != plain[i]).count();
        println!("  saved: {changed} bytes changed, reads back OK");
    }
}

/// Research aid: how long SAVE_EDITOR_TIME takes to decode and to build the Values model.
#[test]
#[ignore]
fn time_load() {
    let path = std::env::var("SAVE_EDITOR_TIME").expect("SAVE_EDITOR_TIME");
    let raw = std::fs::read(&path).unwrap();
    let t = std::time::Instant::now();
    let d = crate::formats::decode_with(&raw, &crate::formats::Options::for_path(std::path::Path::new(&path)));
    println!("decode: {:?} ({})", t.elapsed(), crate::formats::describe(&d.layers, &d.content));
    let t = std::time::Instant::now();
    let names = crate::names::Names::default();
    let m = match &d.content {
        crate::formats::Content::Json { value, .. } => crate::values_view::build(value, &names, None),
        crate::formats::Content::Tree(tr) => crate::values_view::build(&tr.view, &names, Some(&tr.editable)),
        _ => Default::default(),
    };
    println!("model: {:?} ({} rows)", t.elapsed(), m.rows.len());
    let t = std::time::Instant::now();
    let c = d.content.clone();
    println!("clone for baseline: {:?}", t.elapsed());
    drop(c);
}


/// Research aid: opens SAVE_EDITOR_RE (an encrypted RE Engine save) the way the app does,
/// changes the first Money it finds and checks the saved file reads back the same.
#[test]
#[ignore]
fn re_engine_save() {
    let path = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_RE").expect("SAVE_EDITOR_RE"));
    let raw = std::fs::read(&path).unwrap();
    let opts = crate::formats::Options::for_path(&path);
    println!("userdata dirs {:?}, accounts {:?}", crate::platform::steam_userdata_dirs(), crate::fmt::mandarin::accounts_for(&path).len());
    println!("looks {} key {:?}", crate::fmt::mandarin::looks(&raw), crate::fmt::mandarin::find_key(&raw, "", &crate::fmt::mandarin::accounts_for(&path)).map(|(g, u)| (g.title, u)));
    let dec = crate::formats::decode_with(&raw, &opts);
    println!("{}", crate::formats::describe(&dec.layers, &dec.content));
    let crate::formats::Content::Tree(mut doc) = dec.content else { panic!("not a tree: {:?}", dec.hint) };
    let money: Vec<String> = doc.editable.keys().filter(|k| k.ends_with("/Money") || k.ends_with("/SoulAmount") || k.ends_with("/TotalExp") || k.ends_with("/Level")).cloned().collect();
    let mut shown: Vec<(String, String)> = doc
        .editable
        .keys()
        .filter(|k| ["Money", "SoulAmount", "TotalExp", "Exp", "PlayTime", "OniEnergy", "Health", "ClearCount", "GameTime"].iter().any(|n| k.ends_with(&format!("/{n}"))))
        .filter_map(|k| doc.view.pointer(k).filter(|v| v.as_f64().is_some_and(|f| f != 0.0)).map(|v| (k.clone(), v.to_string())))
        .collect();
    shown.sort();
    for (k, v) in shown.iter().take(80) {
        println!("{k} = {v}");
    }
    let top: Vec<String> = doc.view.as_object().unwrap().iter().map(|(k, v)| format!("{k}: {}", v.as_object().map_or(0, |m| m.len()))).collect();
    println!("top {top:?}");
    if let Some(m) = doc.view.pointer("/#dbe3f199").and_then(Value::as_object) {
        println!("under first: {:?}", m.iter().map(|(k, v)| format!("{k}={}", match v { Value::Array(a) => format!("[{}]", a.len()), Value::Object(o) => format!("{{{}}}", o.len()), x => x.to_string() })).collect::<Vec<_>>());
    }
    if let Some(m) = doc.view.pointer("/#dbe3f199/#feea1f9b/0/_Data").and_then(Value::as_object) {
        println!("slot 0 _Data: {:?}", m.iter().map(|(k, v)| format!("{k}={}", match v { Value::Array(a) => format!("[{}]", a.len()), Value::Object(o) => format!("{{{}}}", o.len()), x => x.to_string() })).collect::<Vec<_>>());
    }
    for p in ["/#dbe3f199/#feea1f9b/0/_Data/_PlayerStatus", "/#dbe3f199/#feea1f9b/0/_Data/_ResourceData", "/#dbe3f199/#feea1f9b/0/_Data/_Items/0", "/#dbe3f199/#feea1f9b/0/_Data/_Items/1", "/#dbe3f199/#feea1f9b/0/_Data/_Equipments/0", "/#dbe3f199/#feea1f9b/0/_Data/_EquipItems", "/#dbe3f199/#feea1f9b/0/_Data/_StageStatus", "/#dbe3f199/#feea1f9b/0/_Data/_StoryParam", "/#dbe3f199/_SystemSaveData"] {
        let v = doc.view.pointer(p).cloned().unwrap_or_default();
        let t = v.to_string();
        println!("{p}: {}", &t[..t.len().min(900)]);
    }
    if let Ok(out) = std::env::var("SAVE_EDITOR_RE_JSON") {
        std::fs::write(out, serde_json::to_string(&doc.view).unwrap()).unwrap();
    }
    let items = doc.view.pointer("/#dbe3f199/#feea1f9b/0/_Data/_Items").and_then(Value::as_array).cloned().unwrap_or_default();
    let used: Vec<String> = items.iter().filter(|i| i.to_string().contains("ItemId") && i["ItemId"] != 0).take(40).map(|i| i.to_string()).collect();
    println!("items used {}: {:?}", items.iter().filter(|i| i["ItemId"] != 0 && !i["ItemId"].is_null()).count(), used);
    let k = money.iter().find(|k| k.ends_with("/Money")).expect("Money");
    let old = doc.view.pointer(k).unwrap().as_i64().unwrap();
    doc.set(k, Value::from(old + 12345)).unwrap();
    let content = crate::formats::Content::Tree(doc);
    let out = crate::formats::encode(&dec.layers, &content).unwrap();
    let back = crate::formats::decode_with(&out, &opts);
    assert!(crate::formats::same_layers(&back.layers, &dec.layers));
    assert!(crate::formats::same_data(&back.content, &content), "save check would fail");
    println!("saved {} bytes; Money {old} -> {}", out.len(), old + 12345);
}

/// Research aid: opens SAVE_EDITOR_PIPE the way the app does, sets SAVE_EDITOR_PTR to
/// SAVE_EDITOR_VAL (JSON), saves, and checks the app's read-back check would pass.
#[test]
#[ignore]
fn pipe_edit() {
    let path = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_PIPE").expect("SAVE_EDITOR_PIPE"));
    let ptr = std::env::var("SAVE_EDITOR_PTR").unwrap();
    let val: Value = serde_json::from_str(&std::env::var("SAVE_EDITOR_VAL").unwrap()).unwrap();
    let raw = std::fs::read(&path).unwrap();
    let opts = crate::formats::Options::for_path(&path);
    let dec = crate::formats::decode_with(&raw, &opts);
    println!("{}", crate::formats::describe(&dec.layers, &dec.content));
    let crate::formats::Content::Tree(mut doc) = dec.content else { panic!("not a tree: {:?}", dec.hint) };
    println!("{ptr}: {:?} -> {val}", doc.view.pointer(&ptr));
    assert_eq!(crate::formats::encode(&dec.layers, &crate::formats::Content::Tree(doc.clone())).unwrap().len() > 0, true);
    doc.set(&ptr, val.clone()).unwrap();
    let content = crate::formats::Content::Tree(doc);
    let out = crate::formats::encode(&dec.layers, &content).unwrap();
    let back = crate::formats::decode_with(&out, &opts);
    assert!(crate::formats::same_layers(&back.layers, &dec.layers), "layers differ");
    assert!(crate::formats::verified(&back.content, &content), "save check would fail");
    let crate::formats::Content::Tree(b) = back.content else { panic!() };
    println!("saved {} bytes, reads back {:?}", out.len(), b.view.pointer(&ptr));
    if let Ok(o) = std::env::var("SAVE_EDITOR_OUT") {
        std::fs::write(o, &out).unwrap();
    }
}

/// Research aid: opens SAVE_EDITOR_OPEN the way the app does and prints its layers and note,
/// then checks that saving it unchanged reads back the same.
#[test]
#[ignore]
fn open_only() {
    let path = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_OPEN").expect("SAVE_EDITOR_OPEN"));
    let raw = std::fs::read(&path).unwrap();
    let opts = crate::formats::Options::for_path(&path);
    let t = std::time::Instant::now();
    let dec = crate::formats::decode_with(&raw, &opts);
    println!("{:?}: {} | hint {:?}", t.elapsed(), crate::formats::describe(&dec.layers, &dec.content), dec.hint);
    let out = crate::formats::encode(&dec.layers, &dec.content).unwrap();
    let back = crate::formats::decode_with(&out, &opts);
    assert!(crate::formats::same_layers(&back.layers, &dec.layers) && crate::formats::same_data(&back.content, &dec.content), "read-back differs");
    println!("re-encoded {} bytes (was {}), reads back the same", out.len(), raw.len());
    if let Ok(o) = std::env::var("SAVE_EDITOR_OUT") {
        std::fs::write(o, &out).unwrap();
    }
}

/// Research aid: for every file in SAVE_EDITOR_DIR, how the app opens it.
#[test]
#[ignore]
fn open_dir() {
    let dir = std::env::var("SAVE_EDITOR_DIR").expect("SAVE_EDITOR_DIR");
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let path = e.path();
        let raw = std::fs::read(&path).unwrap();
        let opts = crate::formats::Options::for_path(&path);
        let t = std::time::Instant::now();
        let dec = crate::formats::decode_with(&raw, &opts);
        let rows = match &dec.content {
            crate::formats::Content::Tree(t) => t.editable.len(),
            crate::formats::Content::Json { .. } => 1,
            _ => 0,
        };
        let ok = crate::formats::encode(&dec.layers, &dec.content).ok().map(|out| {
            let back = crate::formats::decode_with(&out, &opts);
            crate::formats::same_layers(&back.layers, &dec.layers) && crate::formats::same_data(&back.content, &dec.content)
        });
        println!("{:<20} {:>6?} {} | {} editable | roundtrip {:?}", e.file_name().to_string_lossy(), t.elapsed(), crate::formats::describe(&dec.layers, &dec.content), rows, ok);
    }
}

/// Research aid: why SAVE_EDITOR_OPEN isn't read by the structured parsers.
#[test]
#[ignore]
fn parse_errors() {
    let path = std::env::var("SAVE_EDITOR_OPEN").expect("SAVE_EDITOR_OPEN");
    let d = std::fs::read(&path).unwrap();
    println!("nrbf looks {} -> {:?}", crate::fmt::nrbf::looks(&d), crate::fmt::nrbf::parse(&d).err());
}

/// Opens saves of every game in the catalog found where that game keeps them (read-only, in
/// memory) and checks each writes back unchanged. SAVE_EDITOR_PER_GAME = saves per game (5).
#[test]
#[ignore]
fn every_game_found() {
    let per: usize = std::env::var("SAVE_EDITOR_PER_GAME").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
    let mut bad = 0;
    for g in crate::games::GAMES {
        let found = crate::games::find(g);
        if found.is_empty() {
            continue;
        }
        let mut kinds = std::collections::BTreeMap::<String, usize>::new();
        let mut fails = vec![];
        let mut slow = 0f32;
        for f in found.iter().filter(|f| f.size < 300 << 20).take(per) {
            let Ok(raw) = std::fs::read(&f.path) else { continue };
            let opts = Options::for_path(&f.path);
            let t = Instant::now();
            let r = catch_unwind(AssertUnwindSafe(|| {
                let d = decode_with(&raw, &opts);
                let kind = describe(&d.layers, &d.content);
                let ok = crate::formats::encode(&d.layers, &d.content).ok().map(|out| {
                    let back = decode_with(&out, &opts);
                    same_layers(&back.layers, &d.layers) && crate::formats::same_data(&back.content, &d.content)
                });
                (kind, ok)
            }));
            slow = slow.max(t.elapsed().as_secs_f32());
            match r {
                Ok((kind, Some(true))) => *kinds.entry(kind).or_default() += 1,
                Ok((kind, other)) => fails.push(format!("{} [{kind}] write-back {other:?}", f.path.file_name().unwrap_or_default().to_string_lossy())),
                Err(_) => fails.push(format!("{}: PANIC", f.path.file_name().unwrap_or_default().to_string_lossy())),
            }
        }
        bad += fails.len();
        println!("{} :: {} found | {:?} | slowest {slow:.1}s", g.name, found.len(), kinds);
        for f in fails {
            println!("   FAIL {f}");
        }
    }
    println!("FAILURES: {bad}");
}

/// Research aid: for each game whose newest save opens as plain binary, writes the data
/// (after any unpacking) to SAVE_EDITOR_DUMP/<game>.bin.
#[test]
#[ignore]
fn dump_binary_games() {
    let dir = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_DUMP").unwrap());
    for g in crate::games::GAMES {
        // Prefer a big file (a real slot rather than settings).
        let mut found = crate::games::find(g);
        found.truncate(8);
        found.sort_by_key(|f| std::cmp::Reverse(f.size));
        let Some(f) = found.into_iter().find(|f| f.size < 200 << 20) else { continue };
        let Ok(raw) = std::fs::read(&f.path) else { continue };
        let d = decode_with(&raw, &Options::for_path(&f.path));
        if let crate::formats::Content::Binary(b) = &d.content {
            let name: String = g.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
            std::fs::write(dir.join(format!("{name}.bin")), b).unwrap();
            println!("{} <- {} ({} bytes, {})", name, f.path.file_name().unwrap().to_string_lossy(), b.len(), describe(&d.layers, &d.content));
        }
    }
}

/// Research aid: SAVE_EDITOR_GLOB_DIR = folder; opens each file (read-only) and prints the
/// top of its view (to compare one game's saves).
#[test]
#[ignore]
fn view_dir() {
    let dir = std::env::var("SAVE_EDITOR_GLOB_DIR").unwrap();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            continue;
        }
        let raw = std::fs::read(&p).unwrap();
        let d = decode_with(&raw, &Options::for_path(&p));
        let v = root(&d.content).map(|v| serde_json::to_string(v).unwrap()).unwrap_or_else(|| describe(&d.layers, &d.content));
        println!("{}: {}", p.file_name().unwrap().to_string_lossy(), v.chars().take(300).collect::<String>());
    }
}

/// Research aid: SAVE_EDITOR_INNER = a save; writes what the last layer unpacks to (next to it, `.inner`).
#[test]
#[ignore]
fn dump_inner() {
    let path = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_INNER").unwrap());
    let raw = std::fs::read(&path).unwrap();
    let opts = crate::formats::Options::for_path(&path);
    let dec = crate::formats::decode_with(&raw, &opts);
    let bytes = dec.content.to_bytes().unwrap();
    println!("{} → {} bytes", crate::formats::describe(&dec.layers, &dec.content), bytes.len());
    std::fs::write(path.with_extension("inner"), bytes).unwrap();
}

/// Research aid: pulls every message file (GMSG) out of an RE Engine game's .pak archives.
/// SAVE_EDITOR_PAKS = archives separated by ';' (later ones override), SAVE_EDITOR_OUT_DIR = where to write.
/// Archive layout facts from ree-pak-rs (eigeen, MIT).
#[test]
#[ignore]
fn pak_messages() {
    use num_bigint::BigUint;
    use std::io::{Read, Seek, SeekFrom};
    const MODULUS: [u8; 129] = [
        0x7D, 0x0B, 0xF8, 0xC1, 0x7C, 0x23, 0xFD, 0x3B, 0xD4, 0x75, 0x16, 0xD2, 0x33, 0x21, 0xD8, 0x10, 0x71, 0xF9, 0x7C, 0xD1, 0x34, 0x93, 0xBA, 0x77,
        0x26, 0xFC, 0xAB, 0x2C, 0xEE, 0xDA, 0xD9, 0x1C, 0x89, 0xE7, 0x29, 0x7B, 0xDD, 0x8A, 0xAE, 0x50, 0x39, 0xB6, 0x01, 0x6D, 0x21, 0x89, 0x5D, 0xA5,
        0xA1, 0x3E, 0xA2, 0xC0, 0x8C, 0x93, 0x13, 0x36, 0x65, 0xEB, 0xE8, 0xDF, 0x06, 0x17, 0x67, 0x96, 0x06, 0x2B, 0xAC, 0x23, 0xED, 0x8C, 0xB7, 0x8B,
        0x90, 0xAD, 0xEA, 0x71, 0xC4, 0x40, 0x44, 0x9D, 0x1C, 0x7B, 0xBA, 0xC4, 0xB6, 0x2D, 0xD6, 0xD2, 0x4B, 0x62, 0xD6, 0x26, 0xFC, 0x74, 0x20, 0x07,
        0xEC, 0xE3, 0x59, 0x9A, 0xE6, 0xAF, 0xB9, 0xA8, 0x35, 0x8B, 0xE0, 0xE8, 0xD3, 0xCD, 0x45, 0x65, 0xB0, 0x91, 0xC4, 0x95, 0x1B, 0xF3, 0x23, 0x1E,
        0xC6, 0x71, 0xCF, 0x3E, 0x35, 0x2D, 0x6B, 0xE3, 0x00,
    ];
    let out_dir = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_OUT_DIR").unwrap());
    std::fs::create_dir_all(&out_dir).unwrap();
    for pak in std::env::var("SAVE_EDITOR_PAKS").unwrap().split(';') {
        let mut f = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(pak).unwrap());
        let mut head = [0u8; 16];
        f.read_exact(&mut head).unwrap();
        assert_eq!(&head[..4], b"KPKA");
        let feature = u16::from_le_bytes([head[6], head[7]]);
        let count = u32::from_le_bytes(head[8..12].try_into().unwrap()) as usize;
        let mut table = vec![0u8; count * 48];
        f.read_exact(&mut table).unwrap();
        if feature & 0x10 != 0 {
            f.read_exact(&mut [0u8; 4]).unwrap();
        }
        if feature & 0x04 != 0 {
            f.read_exact(&mut [0u8; 9]).unwrap();
        }
        if feature & 0x08 != 0 {
            let mut raw = [0u8; 128];
            f.read_exact(&mut raw).unwrap();
            let mut k = raw.to_vec();
            k.resize(129, 0);
            let key = BigUint::from_bytes_le(&k).modpow(&BigUint::from(65537u32), &BigUint::from_bytes_le(&MODULUS)).to_bytes_le();
            let key: Vec<usize> = (0..32).map(|i| *key.get(i).unwrap_or(&0) as usize).collect();
            for (i, b) in table.iter_mut().enumerate() {
                *b ^= (i + key[i % 32] * key[i % 29]) as u8;
            }
        }
        // Chunk table: block size, count, then (start low u32, meta u32) per chunk.
        let mut chunks: Vec<(u64, u32)> = vec![];
        let mut block = 0u32;
        if feature & 0x20 != 0 {
            let mut b4 = [0u8; 4];
            f.read_exact(&mut b4).unwrap();
            block = u32::from_le_bytes(b4);
            f.read_exact(&mut b4).unwrap();
            let n = u32::from_le_bytes(b4) as usize;
            let mut raw = vec![0u8; n * 8];
            f.read_exact(&mut raw).unwrap();
            let (mut high, mut prev) = (0u64, u32::from_le_bytes(raw[0..4].try_into().unwrap()));
            for c in raw.chunks(8) {
                let lo = u32::from_le_bytes(c[0..4].try_into().unwrap());
                if lo < prev {
                    high += 1 << 32;
                }
                prev = lo;
                chunks.push((high | u64::from(lo), u32::from_le_bytes(c[4..8].try_into().unwrap())));
            }
        }
        let mut file = f.into_inner();
        let read_chunk = |file: &mut std::fs::File, i: usize| -> Option<Vec<u8>> {
            let (start, meta) = *chunks.get(i)?;
            let len = (meta >> 10) as usize;
            let mut buf = vec![0u8; len];
            file.seek(SeekFrom::Start(start)).ok()?;
            file.read_exact(&mut buf).ok()?;
            if len as u32 == block { Some(buf) } else { zstd::decode_all(&buf[..]).ok() }
        };
        let (mut found, mut checked) = (0, 0);
        for e in table.chunks(48) {
            let hash = u64::from_le_bytes(e[0..8].try_into().unwrap());
            let offset = u64::from_le_bytes(e[8..16].try_into().unwrap());
            let csize = u64::from_le_bytes(e[16..24].try_into().unwrap()) as usize;
            let usize_ = u64::from_le_bytes(e[24..32].try_into().unwrap()) as usize;
            let attr = u64::from_le_bytes(e[32..40].try_into().unwrap());
            if usize_ > 8 << 20 || usize_ < 16 || (attr >> 16) & 0xFF != 0 {
                continue;
            }
            checked += 1;
            let data: Option<Vec<u8>> = if attr & (1 << 24) != 0 {
                // Chunked: check the magic in the first chunk, then read as many as needed.
                let first = read_chunk(&mut file, offset as usize);
                match first {
                    Some(c) if c.get(4..8) == Some(b"GMSG") => {
                        let mut all = c;
                        let mut i = offset as usize + 1;
                        while all.len() < usize_ {
                            let Some(c) = read_chunk(&mut file, i) else { break };
                            all.extend(c);
                            i += 1;
                        }
                        all.truncate(usize_);
                        Some(all)
                    }
                    _ => None,
                }
            } else {
                let mut buf = vec![0u8; csize];
                if file.seek(SeekFrom::Start(offset)).is_err() || file.read_exact(&mut buf).is_err() {
                    continue;
                }
                let plain = match attr & 0xF {
                    0 => Some(buf),
                    1 => {
                        let mut o = vec![];
                        flate2::read::DeflateDecoder::new(&buf[..]).read_to_end(&mut o).ok().map(|_| o)
                    }
                    2 => zstd::decode_all(&buf[..]).ok(),
                    _ => None,
                };
                plain.filter(|p| p.get(4..8) == Some(b"GMSG"))
            };
            if let Some(d) = data {
                std::fs::write(out_dir.join(format!("{hash:016x}.msg")), d).unwrap();
                found += 1;
            }
        }
        println!("{pak}: {count} entries, {checked} checked, {found} message files");
    }
}

/// UI smoke test: for the saves found for every game in the catalog that open on a game screen
/// (read-only, in memory), draws the screen off-screen at a desktop and a phone width,
/// clicks into it a few times, and reports any panic. SAVE_EDITOR_PER_GAME = saves per game (3).
/// SAVE_EDITOR_DRAW = files to draw instead (separated by `|`), e.g. downloaded saves.
#[test]
#[ignore]
fn every_game_screen_draws() {
    use eframe::egui;
    let per: usize = std::env::var("SAVE_EDITOR_PER_GAME").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
    let (mut drawn, mut bad) = (0, vec![]);
    let mut todo: Vec<(String, std::path::PathBuf)> = vec![];
    if let Ok(list) = std::env::var("SAVE_EDITOR_DRAW") {
        // "@list.txt": one path per line (for more files than an environment variable holds).
        let list = match list.strip_prefix('@') {
            Some(file) => std::fs::read_to_string(file).expect("list file").lines().collect::<Vec<_>>().join("|"),
            None => list,
        };
        todo = list.split('|').filter(|s| !s.is_empty()).map(|s| ("(given)".to_string(), std::path::PathBuf::from(s))).collect();
    } else {
        for g in crate::games::GAMES {
            for f in crate::games::find(g).iter().filter(|f| f.size < 300 << 20).take(per) {
                todo.push((g.name.to_string(), f.path.clone()));
            }
        }
    }
    for (gname, path) in &todo {
        {
            let Ok(raw) = std::fs::read(path) else { continue };
            let d = decode_with(&raw, &Options::for_path(path));
            let (game, tree, view) = match &d.content {
                Content::Tree(t) => (crate::game_ui::detect(&t.kind_name, &t.view), Some(&**t), t.view.clone()),
                Content::Json { value, .. } => (crate::game_ui::detect("JSON", value), None, value.clone()),
                _ => (None, None, Value::Null),
            };
            let Some(game) = game else { continue };
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let r = catch_unwind(AssertUnwindSafe(|| {
                let ctx = egui::Context::default();
                crate::theme::install_fonts(&ctx);
                crate::theme::apply(&ctx, true);
                let mut edits = 0;
                for (w, h) in [(1280.0, 900.0), (380.0, 800.0)] {
                    for frame in 0..4 {
                        let mut input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() };
                        // A click somewhere in the upper half on later frames, to open grids and editors.
                        if frame >= 1 {
                            let p = egui::pos2(w * (0.15 + 0.2 * frame as f32), h * 0.35);
                            input.events.push(egui::Event::PointerMoved(p));
                            input.events.push(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
                            input.events.push(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
                        }
                        let _ = ctx.run(input, |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    let mut cx = crate::game_ui::GameCtx { root: &view, baseline: &view, tree, edits: vec![], archive: None, open_entry: None };
                                    crate::game_ui::show(game, ui, &mut cx);
                                    edits += cx.edits.len();
                                });
                            });
                        });
                    }
                }
                edits
            }));
            match r {
                Ok(_) => {
                    drawn += 1;
                    if std::env::var_os("SAVE_EDITOR_LIST").is_some() {
                        println!("DRAWN {game:?}\t{}", path.display());
                    }
                }
                Err(e) => {
                    let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                    bad.push(format!("{gname} / {name} ({game:?}): {msg}"));
                }
            }
        }
    }
    println!("screens drawn: {drawn}, panics: {}", bad.len());
    for b in &bad {
        println!("   PANIC {b}");
    }
    assert!(bad.is_empty());
}

/// Research aid: for every game in the catalog, whether its saves open on a game screen,
/// and what the reader calls them (read-only, in memory).
#[test]
#[ignore]
fn screens_per_game() {
    for g in crate::games::GAMES {
        let found = crate::games::find(g);
        let mut kinds = std::collections::BTreeMap::<String, (usize, bool)>::new();
        for f in found.iter().filter(|f| f.size < 300 << 20).take(4) {
            let Ok(raw) = std::fs::read(&f.path) else { continue };
            let d = decode_with(&raw, &Options::for_path(&f.path));
            let (kind, screen) = match &d.content {
                Content::Tree(t) => (t.kind_name.clone(), crate::game_ui::detect(&t.kind_name, &t.view).is_some()),
                Content::Json { value, .. } => ("JSON".into(), crate::game_ui::detect("JSON", value).is_some()),
                _ => (describe(&d.layers, &d.content), false),
            };
            let e = kinds.entry(kind).or_insert((0, false));
            e.0 += 1;
            e.1 |= screen;
        }
        let any = kinds.values().any(|k| k.1);
        println!("{}\t{}\t{}\t{:?}", if found.is_empty() { "NOSAVE" } else if any { "SCREEN" } else { "none" }, g.name, found.len(), kinds);
    }
}

/// Research aid: SAVE_EDITOR_UNPACK = save; writes what's left after the outer layers
/// (decryption, unpacking) to UNPACK_OUT, when it's plain binary.
#[test]
#[ignore]
fn unpack_save() {
    let path = std::env::var("SAVE_EDITOR_UNPACK").unwrap();
    let raw = std::fs::read(&path).unwrap();
    let d = decode_with(&raw, &Options::for_path(std::path::Path::new(&path)));
    println!("layers {:?}", d.layers.iter().map(|l| format!("{l:?}").chars().take(60).collect::<String>()).collect::<Vec<_>>());
    if let Content::Binary(b) = &d.content {
        std::fs::write(std::env::var("UNPACK_OUT").unwrap(), b).unwrap();
        println!("wrote {} bytes", b.len());
    }
}

/// Research aid: SAVE_EDITOR_DETECT = a file listing paths (one per line); prints each
/// path with the game screen it opens on (or the reader's kind when there's none).
#[test]
#[ignore]
fn detect_files() {
    let list = std::fs::read_to_string(std::env::var("SAVE_EDITOR_DETECT").unwrap()).unwrap();
    for line in list.lines().filter(|l| !l.is_empty()) {
        let path = std::path::Path::new(line);
        let Ok(raw) = std::fs::read(path) else { continue };
        let r = catch_unwind(AssertUnwindSafe(|| {
            let d = decode_with(&raw, &Options::for_path(path));
            match &d.content {
                Content::Tree(t) => match crate::game_ui::detect(&t.kind_name, &t.view) {
                    Some(g) => format!("{g:?}"),
                    None => format!("-({})", t.kind_name),
                },
                Content::Json { value, .. } => crate::game_ui::detect("JSON", value).map_or("-(JSON)".into(), |g| format!("{g:?}")),
                _ => "-".into(),
            }
        }));
        println!("DET\t{line}\t{}", r.unwrap_or_else(|_| "PANIC".into()));
    }
}

/// Research aid: runs "Max crafting materials" on the saves in SAVE_EDITOR_CRAFT
/// ("a|b|…") through the whole save pipeline: every material stack must read back at its
/// cap, and every other value must read back unchanged.
#[test]
#[ignore]
fn crafting_materials_on_saves() {
    let list = std::env::var("SAVE_EDITOR_CRAFT").expect("SAVE_EDITOR_CRAFT");
    for path in list.split('|').filter(|s| !s.is_empty()).map(std::path::PathBuf::from) {
        let raw = std::fs::read(&path).unwrap();
        let mut d = decode_with(&raw, &Options::for_path(&path));
        let Content::Tree(t) = &mut d.content else { panic!("{}: not a tree", path.display()) };
        let game = crate::game_ui::detect(&t.kind_name, &t.view).expect("a game screen");
        let before = t.view.clone();
        let mut cx = crate::game_ui::GameCtx { root: &before, baseline: &before, tree: Some(&**t), edits: vec![], archive: None, open_entry: None };
        let list = crate::game_ui::crafting::stacks(game, &cx).unwrap_or_default();
        let n = crate::game_ui::max_crafting_materials(game, &mut cx);
        let edits = cx.edits;
        for (p, v) in &edits {
            t.set(p, v.clone()).unwrap().expect("editable");
        }
        let out = encode(&d.layers, &d.content).unwrap();
        let back = decode_with(&out, &Options::for_path(&path));
        let Content::Tree(b) = &back.content else { panic!("reads back") };
        let mut expect = before.clone();
        for (p, v) in &edits {
            *expect.pointer_mut(p).unwrap() = v.clone();
        }
        assert_eq!(b.view, expect, "{}: only the material stacks may change", path.display());
        let names: Vec<String> = list.iter().map(|s| format!("{} {}->{}", s.name, s.have, s.cap)).collect();
        println!("{} ({game:?}): {} material stacks, {n} raised, {} values written: {}", path.display(), list.len(), edits.len(), names.join(", "));
    }
}

/// Every game screen must draw (without panicking) on saves that lack what it looks for, or
/// hold it with unexpected types: a damaged, truncated or newer-version save.
#[test]
fn every_screen_survives_odd_data() {
    use crate::game_ui::Game;
    use eframe::egui;
    use serde_json::json;
    const ALL: &[Game] = &[Game::EldenRing,Game::DarkSouls3,Game::DarkSouls2,Game::DarkSouls1,Game::Sekiro,Game::Terraria,Game::TerrariaWorld,Game::Pokemon,Game::Bethesda,Game::Hades,Game::Stardew,Game::Minecraft,Game::Balatro,Game::HollowKnight,Game::SlayTheSpire,Game::RpgMaker,Game::Onimusha,Game::Paradox,Game::RimWorld,Game::Subnautica,Game::OxygenNotIncluded,Game::GreenHell,Game::ReSlot,Game::Hogwarts,Game::RenPy,Game::MassEffect,Game::Xcom,Game::Xcom2,Game::Civ6,Game::Diablo2,Game::Pathfinder,Game::Ff7,Game::Ff7Remake,Game::HogwartsDb,Game::Ff9,Game::DyingLight2,Game::DyingLight,Game::Witcher3,Game::Cyberpunk,Game::Kenshi,Game::Palworld,Game::P4g,Game::Ffx2,Game::P5r,Game::Ffx,Game::Metro,Game::Yakuza0,Game::YakuzaDe,Game::Kiwami,Game::Ffxv,Game::DeathStranding2,Game::Gow2018,Game::GowRagnarok,Game::Cairn,Game::Borderlands,Game::Borderlands3,Game::Wonderlands,Game::TruckSim,Game::X4,Game::Mgsv,Game::TotalWar,Game::GrimDawn,Game::TitanQuest,Game::Torchlight2,Game::Fallout12,Game::DeusEx,Game::Kotor,Game::Nwn,Game::Infinity,Game::DragonAge,Game::HalfLife,Game::HalfLife2,Game::Silksong,Game::Undertale,Game::Deltarune,Game::VampireSurvivors,Game::DarkestDungeon,Game::DarkestDungeon2,Game::Octopath,Game::Dos2,Game::Bg3,Game::Wasteland3,Game::Valheim,Game::GoldenSun,Game::Torment,Game::Starbound,Game::Expedition33,Game::Fallout3Nv,Game::Andromeda,Game::Nier,Game::Sims1,Game::Sims2,Game::Sims4,Game::Phasmophobia,Game::LethalCompany,Game::Repo,Game::DeadRising,Game::DeadRising2,Game::LegoBatman];
    let odd = json!({
        "Save": {"Level": "x", "Gold": -1}, "Shepard": [], "player": {"Money": null}, "party": {}, "Character": 5,
        "Progress": {"Level": 1e30}, "Attributes": [1, 2], "Squad": {"": {}}, "Resources": "none", "#1": {"_Data": {}},
        "GameState": {"Resources": []}, "SaveGame": {"player": {"farmName": 1}}, "Data": {"Player": {"Inventory": {}}},
        "playerData": {"geo": "a"}, "actors": {"_data": {"@a": [null]}}, "Variables": [], "base": {"m_title": 0}
    });
    let mut bad = vec![];
    for &game in ALL {
        let _ = game.tab_label();
        for root in [json!({}), json!(null), json!([]), odd.clone()] {
            let r = catch_unwind(AssertUnwindSafe(|| {
                let ctx = egui::Context::default();
                crate::theme::install_fonts(&ctx);
                crate::theme::apply(&ctx, true);
                for (w, h) in [(1280.0, 900.0), (360.0, 700.0)] {
                    for frame in 0..3 {
                        let mut input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() };
                        if frame >= 1 {
                            let p = egui::pos2(w * (0.2 + 0.3 * frame as f32), h * 0.3);
                            input.events.push(egui::Event::PointerMoved(p));
                            input.events.push(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() });
                            input.events.push(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() });
                        }
                        let _ = ctx.run(input, |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: None, edits: vec![], archive: None, open_entry: None };
                                    crate::game_ui::show(game, ui, &mut cx);
                                });
                            });
                        });
                    }
                }
            }));
            if let Err(e) = r {
                let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                bad.push(format!("{game:?} on {}: {msg}", if root.is_object() && root.as_object().unwrap().is_empty() { "{}".to_string() } else { root.to_string().chars().take(20).collect() }));
            }
        }
    }
    assert!(bad.is_empty(), "screens that panicked:\n{}", bad.join("\n"));
}
