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
    let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: Some(&**t), edits: vec![] };
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
        let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: Some(&doc), edits: vec![] };
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
        let mut cx = crate::game_ui::GameCtx { root: &root, baseline: &root, tree: Some(&doc), edits: vec![] };
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
    assert!(crate::formats::same_data(&back.content, &content), "save check would fail");
    let crate::formats::Content::Tree(b) = back.content else { panic!() };
    println!("saved {} bytes, reads back {}", out.len(), b.view.pointer(&ptr).unwrap());
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
