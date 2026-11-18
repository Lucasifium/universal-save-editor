//! Sample check: every game screen must open the saves of its game on that screen.
//!
//! `tests/screen-samples.tsv` lists every screen with its status: `sample` (a public save of
//! the game was opened on it and checked) or `none` (no public save could be found yet).
//! `every_screen_has_a_sample_status` fails when a screen is missing from the list, or the list
//! names a screen that no longer exists, so a new screen can't be added without saying whether
//! it was checked on a real save.
//!
//! The saves themselves are never kept in the repository. To check them on your machine, put
//! one `screen<TAB>path` line per save into a file and run
//!
//!     SAVE_EDITOR_SAMPLES=samples.tsv cargo test --release screens_open_their_samples -- --nocapture
//!
//! (without the variable the test does nothing). Each save is opened with the full decode chain
//! and has to land on its screen, hold editable values, and write back unchanged.
//! `scripts/check-samples.sh` wraps both steps.
//!
//! To find which screen each file in a list opens on (to build such a list):
//!
//!     SAVE_EDITOR_SCAN=files.txt cargo test --release which_screen -- --ignored --nocapture

use crate::formats::{decode_with, describe, encode, same_data, same_layers, Content, Decoded, Options};
use crate::game_ui::{detect, simple, Game};
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

const STATUS: &str = include_str!("../tests/screen-samples.tsv");

/// The screen a decoded save opens on, by name: a `Game` variant, or a table-driven game's name.
pub(crate) fn screen_of(d: &Decoded) -> Option<String> {
    let game = match &d.content {
        Content::Tree(t) => detect(&t.kind_name, &t.view),
        Content::Json { value, .. } => detect("JSON", value),
        _ => None,
    }?;
    Some(screen_name(game))
}

fn screen_name(g: Game) -> String {
    match g {
        Game::Simple(i) => simple::game_name(i).to_string(),
        other => format!("{other:?}"),
    }
}

/// Every screen: the `Game` variants (read from the enum's source) and the table-driven games.
fn all_screens() -> BTreeSet<String> {
    let src = include_str!("game_ui/mod.rs");
    let src = src.replace("\r\n", "\n");
    let body = src.split("pub enum Game {").nth(1).expect("Game enum").split("\n}").next().unwrap();
    let mut out = BTreeSet::new();
    for l in body.lines().map(str::trim) {
        if l.starts_with("//") || l.is_empty() {
            continue;
        }
        let name = l.split("//").next().unwrap_or("").trim().trim_end_matches(',');
        if name.chars().all(|c| c.is_ascii_alphanumeric()) {
            out.insert(name.to_string());
        }
    }
    // The reading above skips what it doesn't understand: fail loudly if it lost the enum.
    assert!(out.len() > 100, "could not read the Game variants from src/game_ui/mod.rs");
    for s in simple::all() {
        out.insert(s.name.to_string());
    }
    out
}

fn statuses() -> BTreeMap<String, (String, String)> {
    let mut m = BTreeMap::new();
    for (n, l) in STATUS.lines().enumerate() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let mut f = l.split('\t');
        let (screen, status, note) = (f.next().unwrap_or(""), f.next().unwrap_or(""), f.next().unwrap_or(""));
        assert!(matches!(status, "sample" | "none"), "tests/screen-samples.tsv line {}: status must be `sample` or `none`", n + 1);
        assert!(m.insert(screen.to_string(), (status.to_string(), note.to_string())).is_none(), "tests/screen-samples.tsv: `{screen}` is listed twice");
    }
    m
}

#[test]
fn every_screen_has_a_sample_status() {
    let screens = all_screens();
    let listed = statuses();
    let missing: Vec<_> = screens.iter().filter(|s| !listed.contains_key(*s)).collect();
    let stale: Vec<_> = listed.keys().filter(|s| !screens.contains(*s)).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "tests/screen-samples.tsv is out of date.\nScreens with no line (open a public save of the game on the screen, then add `<screen>\\tsample\\t<where the save came from>`, or `<screen>\\tnone\\t<why>` if there is none): {missing:?}\nLines for screens that no longer exist: {stale:?}"
    );
}

/// Opens one save as the app does and checks it. Returns the problems found.
pub(crate) fn check_open(screen: &str, path: &Path) -> Vec<String> {
    let mut bad = vec![];
    let Ok(raw) = std::fs::read(path) else { return vec!["cannot read the file".into()] };
    let opts = Options::for_path(path);
    let r = catch_unwind(AssertUnwindSafe(|| {
        let d = decode_with(&raw, &opts);
        let got = screen_of(&d);
        let rows = match &d.content {
            Content::Tree(t) => t.editable.len(),
            Content::Json { .. } => 1,
            _ => 0,
        };
        let back = encode(&d.layers, &d.content).ok().map(|out| {
            let again = decode_with(&out, &opts);
            (out == raw, same_layers(&again.layers, &d.layers) && same_data(&again.content, &d.content))
        });
        (got, rows, back, describe(&d.layers, &d.content))
    }));
    match r {
        Err(_) => bad.push("panicked while opening".into()),
        Ok((got, rows, back, kind)) => {
            if got.as_deref() != Some(screen) {
                bad.push(format!("opens on {:?} instead ({kind})", got));
            }
            if rows == 0 {
                bad.push("no editable values".into());
            }
            match back {
                None => bad.push("cannot be written back".into()),
                Some((_, false)) => bad.push("writes back different data".into()),
                // Not byte-identical is fine for archives that are rebuilt (zip) when it reads back the same.
                Some((false, true)) => println!("  note: re-encoded bytes differ but read back the same ({kind})"),
                Some((true, true)) => {}
            }
        }
    }
    bad
}

#[test]
fn screens_open_their_samples() {
    let Ok(list) = std::env::var("SAVE_EDITOR_SAMPLES") else { return };
    let listed = statuses();
    let text = std::fs::read_to_string(&list).expect("SAVE_EDITOR_SAMPLES file");
    let mut failures = vec![];
    let (mut ok, mut absent) = (0, 0);
    let mut seen = BTreeSet::new();
    for l in text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let Some((screen, path)) = l.split_once('\t') else { continue };
        let path = Path::new(path.trim());
        if !path.exists() {
            absent += 1;
            continue;
        }
        let bad = check_open(screen, path);
        if bad.is_empty() {
            ok += 1;
            seen.insert(screen.to_string());
        } else {
            failures.push(format!("{screen}: {} ({})", bad.join("; "), path.display()));
        }
    }
    println!("{ok} saves open on their screen, {absent} listed saves not present here");
    for (s, (st, _)) in &listed {
        if st == "sample" && !seen.contains(s) && std::env::var_os("SAVE_EDITOR_STRICT").is_some() {
            failures.push(format!("{s}: marked `sample` but no listed save of it opened"));
        }
    }
    assert!(failures.is_empty(), "{} problems:\n{}", failures.len(), failures.join("\n"));
}

/// Research aid: the screen each file of SAVE_EDITOR_SCAN (one path per line) opens on, with
/// whether it writes back unchanged: `screen<TAB>path<TAB>result`.
#[test]
#[ignore]
fn which_screen() {
    std::panic::set_hook(Box::new(|_| {}));
    let list = std::fs::read_to_string(std::env::var("SAVE_EDITOR_SCAN").expect("SAVE_EDITOR_SCAN")).unwrap();
    for path in list.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let p = Path::new(path);
        let Ok(meta) = std::fs::metadata(p) else { continue };
        if meta.len() > 200 << 20 {
            continue;
        }
        let Ok(raw) = std::fs::read(p) else { continue };
        let opts = Options::for_path(p);
        // Where a run that is killed (out of memory) stopped, for the wrapper script.
        if let Ok(f) = std::env::var("SAVE_EDITOR_CURRENT") {
            let _ = std::fs::write(f, path);
        }
        let r = catch_unwind(AssertUnwindSafe(|| {
            let d = decode_with(&raw, &opts);
            let screen = screen_of(&d);
            let res = match encode(&d.layers, &d.content) {
                Err(_) => "noencode".to_string(),
                Ok(out) => {
                    let again = decode_with(&out, &opts);
                    if !(same_layers(&again.layers, &d.layers) && same_data(&again.content, &d.content)) {
                        "READBACK-DIFFERS".to_string()
                    } else if out == raw {
                        "identical".to_string()
                    } else {
                        "same-data".to_string()
                    }
                }
            };
            (screen, res)
        }));
        match r {
            Ok((Some(s), res)) => println!("SCREEN\t{s}\t{path}\t{res}"),
            Ok((None, _)) => {}
            Err(_) => println!("PANIC\t-\t{path}\t-"),
        }
    }
}

/// Research aid: prints every screen name, one per line.
#[test]
#[ignore]
fn list_screens() {
    for s in all_screens() {
        println!("SCREENNAME\t{s}");
    }
}

/// Research aid: writes what SAVE_EDITOR_OPEN would be saved as, unchanged, to SAVE_EDITOR_OUT.
#[test]
#[ignore]
fn reencode() {
    let path = std::path::PathBuf::from(std::env::var("SAVE_EDITOR_OPEN").expect("SAVE_EDITOR_OPEN"));
    let raw = std::fs::read(&path).unwrap();
    let d = decode_with(&raw, &Options::for_path(&path));
    std::fs::write(std::env::var("SAVE_EDITOR_OUT").expect("SAVE_EDITOR_OUT"), encode(&d.layers, &d.content).unwrap()).unwrap();
}
