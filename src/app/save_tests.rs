//! Saving on a worker thread: the same file and backups as saving on the window's thread,
//! nothing can change the file while it is written, and no failure loses the changes.

use super::*;
use eframe::App as _;

/// A fresh folder for one test, removed when it ends.
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("use-bgsave-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Scratch(d)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Backups go beside the save (no chosen folder), and messages are in English, for the
/// whole test.
fn locks() -> (std::sync::MutexGuard<'static, ()>, std::sync::MutexGuard<'static, ()>) {
    let b = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::backups::set_root(None);
    let l = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::i18n::set(Some("en"));
    (b, l)
}

/// A JSON save over the background thresholds (file size and number of values).
fn big_json() -> Vec<u8> {
    let items: Vec<Value> = (0..12_000).map(|i| json!({"name": format!("Item {i}"), "count": i})).collect();
    serde_json::to_vec_pretty(&json!({"gold": 100, "hero": {"name": "Ann", "level": 3}, "items": items})).unwrap()
}

/// A window with the app's fonts and look.
fn window() -> egui::Context {
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, true);
    ctx
}

fn opened(ctx: &egui::Context, path: &Path) -> App {
    let mut app = App::with(ctx, Settings::default(), false);
    app.load(path);
    assert!(matches!(app.doc.as_ref().map(|d| &d.content), Some(Content::Json { .. })), "{}", app.status);
    app
}

/// One frame, with the input going through `raw_input_hook` as eframe does.
fn frame(app: &mut App, ctx: &egui::Context, mut raw: egui::RawInput) -> Vec<egui::ViewportCommand> {
    app.raw_input_hook(ctx, &mut raw);
    let out = ctx.run(raw, |ctx| app.frame(ctx));
    out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default()
}

#[test]
fn first_launch_update_question_defaults_to_yes_on_enter() {
    let _locks = locks();
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    assert!(!app.settings.update_question_answered && !app.settings.check_updates);
    app.update_question = true;
    for _ in 0..3 {
        frame(&mut app, &ctx, egui::RawInput::default());
    }
    assert!(app.update_question, "stays until answered");
    let key = |pressed| egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed, repeat: false, modifiers: egui::Modifiers::NONE };
    let mut raw = egui::RawInput::default();
    raw.events = vec![key(true), key(false)];
    frame(&mut app, &ctx, raw);
    for _ in 0..2 {
        frame(&mut app, &ctx, egui::RawInput::default());
    }
    assert!(!app.update_question);
    assert!(app.settings.check_updates && app.settings.update_question_answered);
}

/// Runs frames until the save is in; returns every window command sent meanwhile.
fn wait(app: &mut App, ctx: &egui::Context) -> Vec<egui::ViewportCommand> {
    let mut cmds = vec![];
    for _ in 0..4000 {
        cmds.extend(frame(app, ctx, egui::RawInput::default()));
        if app.saving.is_none() {
            return cmds;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the save never finished");
}

fn value(app: &App, ptr: &str) -> Option<Value> {
    app.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(ptr)).cloned()
}

/// What the Values list shows: each value's section, name, place and whether it can be edited.
fn rows(m: &Model) -> Vec<(String, String, String, String, bool)> {
    m.rows.iter().map(|r| (r.ptr.to_string(), m.sections[r.section].title.clone(), r.name().to_string(), r.context().to_string(), r.editable)).collect()
}

#[test]
fn small_saves_stay_on_the_window_thread_and_big_ones_do_not() {
    let _l = locks();
    let s = Scratch::new("threshold");
    let ctx = window();
    let small = s.0.join("small.json");
    std::fs::write(&small, br#"{"gold": 1}"#).unwrap();
    let mut app = opened(&ctx, &small);
    assert!(!app.save_in_background());
    app.apply_edit("/gold".into(), json!(2), 0.0);
    app.save_to(&small);
    assert!(app.saving.is_none(), "written before save_to returned");
    assert!(!app.dirty);
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&small).unwrap()).unwrap()["gold"], json!(2));
    let big = s.0.join("big.json");
    std::fs::write(&big, big_json()).unwrap();
    assert!(opened(&ctx, &big).save_in_background());
}

#[test]
fn a_background_save_writes_the_same_file_and_backups() {
    let _l = locks();
    let ctx = window();
    let original = big_json();
    let mut done = vec![];
    for background in [false, true] {
        let s = Scratch::new(if background { "same-bg" } else { "same-fg" });
        let path = s.0.join("save.json");
        std::fs::write(&path, &original).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(999_999), 0.0);
        app.apply_edit("/items/7/count".into(), json!(-5), 0.0);
        app.save_with(&path, AfterSave::Stay, background);
        assert_eq!(app.saving.is_some(), background);
        wait(&mut app, &ctx);
        assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
        assert!(!app.dirty);
        let d = app.doc.as_ref().unwrap();
        assert!(formats::same_data(&d.content, &d.baseline));
        assert_eq!(d.stamp, safety::stamp(&path));
        let backups: Vec<Vec<u8>> = crate::backups::list(&path).iter().map(|v| std::fs::read(&v.path).unwrap()).collect();
        let bak = std::fs::read(s.0.join("save.json.bak")).unwrap();
        done.push((std::fs::read(&path).unwrap(), backups, bak, app.status.clone(), app.undo.len(), rows(&app.model)));
    }
    let (fg, bg) = (&done[0], &done[1]);
    assert_eq!(fg.0, bg.0, "byte-identical files");
    assert_ne!(fg.0, original);
    assert_eq!(fg.1, vec![original.clone()], "the file as it was is the one backup");
    assert_eq!(fg.1, bg.1);
    assert_eq!((&fg.2, &bg.2), (&original, &original), "the .bak copy holds the original");
    assert_eq!((&fg.3, fg.4, &fg.5), (&bg.3, bg.4, &bg.5), "same message, undo steps and Values list");
}

#[test]
fn a_failed_background_save_leaves_the_file_and_the_changes() {
    let _l = locks();
    let s = Scratch::new("fail");
    let ctx = window();
    let path = s.0.join("save.json");
    let original = big_json();
    std::fs::write(&path, &original).unwrap();
    let mut app = opened(&ctx, &path);
    let stamp = app.doc.as_ref().unwrap().stamp.clone();
    app.apply_edit("/gold".into(), json!(5), 0.0);
    // The temporary file the new save is written to can't be made.
    std::fs::create_dir_all(s.0.join("save.json.partial").join("inside")).unwrap();
    app.save_with(&path, AfterSave::Stay, true);
    assert!(app.saving.is_some());
    wait(&mut app, &ctx);
    assert_eq!(std::fs::read(&path).unwrap(), original, "the file on disk is untouched");
    assert_eq!(app.status_kind, StatusKind::Err);
    assert!(app.status.starts_with("Couldn't write"), "{}", app.status);
    assert!(app.banner.is_some(), "shown where it can't be missed");
    assert!(app.dirty, "the changes still count as unsaved");
    assert_eq!(value(&app, "/gold"), Some(json!(5)));
    assert_eq!(app.doc.as_ref().unwrap().stamp, stamp);
    // Once the way is clear, saving again works.
    std::fs::remove_dir_all(s.0.join("save.json.partial")).unwrap();
    app.save_with(&path, AfterSave::Stay, true);
    wait(&mut app, &ctx);
    assert!(!app.dirty, "{}", app.status);
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(5));
}

#[test]
fn nothing_changes_the_file_while_it_is_being_saved() {
    let _l = locks();
    let s = Scratch::new("held");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, big_json()).unwrap();
    let other = s.0.join("other.json");
    std::fs::write(&other, br#"{"gold": 1}"#).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(777), 0.0);
    app.save_with(&path, AfterSave::Stay, true);
    assert!(app.saving.is_some());
    let gen = app.edit_gen;
    // Every way the program changes the file, opens another or starts another save.
    app.apply_edit("/gold".into(), json!(1), 1.0);
    assert_eq!(value(&app, "/gold"), Some(json!(777)));
    assert_eq!(app.paste_values(&[("/gold".into(), json!(2))]), (0, 1));
    app.undo_redo(true);
    app.revert_all();
    app.open_in_tab(other.clone());
    app.request_open(other.clone());
    app.load_async(&other);
    app.load(&other);
    app.close_tab(0);
    app.close_file();
    app.save_clicked();
    app.save_with(&path, AfterSave::Stay, false);
    assert_eq!(app.tab_count(), 1);
    assert_eq!(app.edit_gen, gen);
    assert_eq!(value(&app, "/gold"), Some(json!(777)));
    assert_eq!(app.doc.as_ref().unwrap().path, path);
    // Keys, typed text, clicks and dropped files never reach the window.
    let mut raw = egui::RawInput::default();
    raw.events = vec![
        egui::Event::Key { key: egui::Key::Z, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND },
        egui::Event::Text("9".into()),
        egui::Event::PointerButton { pos: egui::pos2(10.0, 10.0), button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE },
        egui::Event::PointerMoved(egui::pos2(10.0, 10.0)),
    ];
    raw.dropped_files.push(egui::DroppedFile { path: Some(other.clone()), ..Default::default() });
    app.raw_input_hook(&ctx, &mut raw);
    assert_eq!(raw.events, vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))]);
    assert!(raw.dropped_files.is_empty());
    wait(&mut app, &ctx);
    assert!(!app.dirty, "{}", app.status);
    assert_eq!(app.undo.len(), 1, "the edit made before saving is still the one undo step");
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(777));
    // And afterwards everything works again.
    app.apply_edit("/gold".into(), json!(1), 2.0);
    assert_eq!(value(&app, "/gold"), Some(json!(1)));
}

#[test]
fn a_change_that_slips_in_during_a_save_stays_unsaved() {
    let _l = locks();
    let s = Scratch::new("slip");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, big_json()).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(10), 0.0);
    app.save_with(&path, AfterSave::CloseFile, true);
    // Past every guard, as a bug might: the value on screen changes mid-save.
    *app.root_mut().unwrap().pointer_mut("/gold").unwrap() = json!(20);
    app.touched.insert("/gold");
    app.edit_gen += 1;
    wait(&mut app, &ctx);
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(10), "the file holds what was saved");
    assert_eq!(value(&app, "/gold"), Some(json!(20)), "what is shown is kept");
    assert!(app.dirty, "and still counts as unsaved");
    assert!(app.doc.is_some(), "the file wasn't closed");
    assert_eq!(app.status_kind, StatusKind::Err);
    assert_eq!(app.doc.as_ref().unwrap().stamp, safety::stamp(&path), "not mistaken for a change on disk");
    app.save_with(&path, AfterSave::Stay, true);
    wait(&mut app, &ctx);
    assert!(!app.dirty);
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(20));
}

/// What the Changes panel's group Revert applies: the original of every value still differing.
fn originals(app: &App, ptrs: &[&str]) -> Vec<(String, Value)> {
    let b = root(&app.doc.as_ref().unwrap().baseline).unwrap();
    ptrs.iter().map(|p| (p.to_string(), b.pointer(p).unwrap().clone())).collect()
}

#[test]
fn a_mass_edit_is_one_undo_step_and_one_change_group() {
    let _l = locks();
    let s = Scratch::new("group");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, big_json()).unwrap();
    let mut app = opened(&ctx, &path);
    let original = app.doc.as_ref().unwrap().content.to_bytes().unwrap();
    let ptrs = ["/gold", "/hero/level", "/items/3/count", "/items/9/count"];
    app.apply_edits(ptrs.iter().map(|p| (p.to_string(), json!(9999))).collect(), 0.0);
    assert_eq!(app.undo.len(), 1, "one button press, one undo step");
    let ids: HashSet<_> = ptrs.iter().map(|p| app.touched.group.get(*p).copied()).collect();
    assert_eq!(ids.len(), 1, "all in one group");
    assert!(ids.iter().next().unwrap().is_some());
    // Undo puts every value back, byte for byte, and Redo makes them all again.
    app.undo_redo(true);
    assert!(!app.dirty);
    assert_eq!(app.doc.as_ref().unwrap().content.to_bytes().unwrap(), original);
    app.undo_redo(false);
    for p in ptrs {
        assert_eq!(value(&app, p), Some(json!(9999)));
    }
    // The group's Revert in the Changes panel: one step, exact original.
    app.refresh_changes();
    assert_eq!(app.changes.ptrs.len(), 4);
    let back = originals(&app, &ptrs);
    app.apply_edits(back, f64::NEG_INFINITY);
    assert!(!app.dirty);
    assert_eq!(app.doc.as_ref().unwrap().content.to_bytes().unwrap(), original);
    assert_eq!(app.undo.len(), 2, "the revert is a single step too");
    app.undo_redo(true);
    for p in ptrs {
        assert_eq!(value(&app, p), Some(json!(9999)), "undoing the revert brings them all back");
    }
}

#[test]
fn a_single_edit_stays_on_its_own_and_leaves_its_group_when_edited_again() {
    let _l = locks();
    let s = Scratch::new("single");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, big_json()).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edits(vec![("/gold".into(), json!(5))], 0.0);
    assert!(app.touched.group.is_empty(), "one edit is no group");
    assert_eq!(app.undo.len(), 1);
    app.apply_edits(vec![("/gold".into(), json!(6)), ("/hero/level".into(), json!(7))], 10.0);
    assert_eq!(app.touched.group.len(), 2);
    // Edited alone afterwards, one value is no longer part of the group.
    app.apply_edits(vec![("/gold".into(), json!(8))], 20.0);
    assert!(!app.touched.group.contains_key("/gold"));
    assert!(app.touched.group.contains_key("/hero/level"));
    // Saving starts afresh.
    app.revert_all();
    assert!(app.touched.group.is_empty());
}

    #[test]
fn closing_the_window_during_a_save_waits_for_it() {
    let _l = locks();
    let s = Scratch::new("close");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, big_json()).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(31), 0.0);
    app.save_with(&path, AfterSave::Stay, true);
    let mut raw = egui::RawInput::default();
    raw.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo { events: vec![egui::ViewportEvent::Close], ..Default::default() });
    let first = frame(&mut app, &ctx, raw);
    if app.saving.is_some() {
        assert!(first.contains(&egui::ViewportCommand::CancelClose), "{first:?}");
        assert!(!first.contains(&egui::ViewportCommand::Close));
    }
    let mut cmds = first;
    cmds.extend(wait(&mut app, &ctx));
    assert!(cmds.contains(&egui::ViewportCommand::Close), "closes once saved: {cmds:?}");
    assert!(!app.any_dirty());
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(31));
}

#[test]
fn save_and_quit_in_the_background_closes_only_once_saved() {
    let _l = locks();
    let s = Scratch::new("quit");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, big_json()).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(32), 0.0);
    app.save_with(&path, AfterSave::Quit, true);
    assert!(!app.close_now && !app.allow_close);
    let cmds = wait(&mut app, &ctx);
    assert_eq!(cmds.iter().filter(|c| **c == egui::ViewportCommand::Close).count(), 1, "{cmds:?}");
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(32));
    // A save that fails doesn't quit: the question comes back instead.
    app.apply_edit("/gold".into(), json!(33), 1.0);
    std::fs::create_dir_all(s.0.join("save.json.partial").join("x")).unwrap();
    app.allow_close = false;
    app.save_with(&path, AfterSave::Quit, true);
    let cmds = wait(&mut app, &ctx);
    assert!(!cmds.contains(&egui::ViewportCommand::Close), "{cmds:?}");
    assert!(app.dirty);
}

#[test]
fn save_and_quit_with_two_unsaved_tabs_asks_about_the_second() {
    let _l = locks();
    let s = Scratch::new("quit2");
    let ctx = window();
    let (a, b) = (s.0.join("a.json"), s.0.join("b.json"));
    std::fs::write(&a, br#"{"gold": 1}"#).unwrap();
    std::fs::write(&b, br#"{"gold": 1}"#).unwrap();
    let mut app = opened(&ctx, &a);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    app.open_in_tab(b.clone());
    for _ in 0..400 {
        if app.loading.is_none() {
            break;
        }
        frame(&mut app, &ctx, egui::RawInput::default());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(b.clone()), "{}", app.status);
    app.apply_edit("/gold".into(), json!(3), 1.0);
    app.save_then(AfterSave::Quit);
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&b).unwrap()).unwrap()["gold"], json!(3));
    // The other tab is still unsaved: no closing yet, the question comes back for it.
    assert!(!app.close_now && !app.allow_close && app.confirm_close);
    assert!(app.dirty, "the unsaved tab is shown");
    app.confirm_close = false;
    app.save_then(AfterSave::Quit);
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&a).unwrap()).unwrap()["gold"], json!(2));
    assert!(app.close_now && app.allow_close);
}

#[test]
fn nothing_is_saved_while_a_file_is_being_opened() {
    let _l = locks();
    let s = Scratch::new("loading");
    let ctx = window();
    let path = s.0.join("save.json");
    let original = big_json();
    std::fs::write(&path, &original).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(7), 0.0);
    app.load_async(&path);
    assert!(app.loading.is_some(), "big files are read on a worker thread");
    app.save_clicked();
    app.save_with(&path, AfterSave::Stay, true);
    app.save_with(&path, AfterSave::Stay, false);
    assert!(app.saving.is_none() && app.save_review.is_none());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn a_save_thread_that_panics_is_reported_and_the_changes_stay() {
    let _l = locks();
    let s = Scratch::new("panic");
    let ctx = window();
    let path = s.0.join("save.json");
    let original = big_json();
    std::fs::write(&path, &original).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(6), 0.0);
    app.close_after_save = true;
    app.spawn_save(&path, AfterSave::CloseFile, || panic!("a bug in the save code"));
    let cmds = wait(&mut app, &ctx);
    assert_eq!(app.status_kind, StatusKind::Err);
    assert!(app.banner.is_some());
    assert!(app.dirty);
    assert!(app.doc.is_some(), "not closed");
    assert!(!cmds.contains(&egui::ViewportCommand::Close), "asked about the unsaved changes instead of closing");
    assert!(app.confirm_close);
    assert_eq!(value(&app, "/gold"), Some(json!(6)));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    // Saving works again afterwards.
    app.confirm_close = false;
    app.save_with(&path, AfterSave::Stay, true);
    wait(&mut app, &ctx);
    assert!(!app.dirty, "{}", app.status);
}

#[test]
fn the_values_list_is_kept_only_when_it_would_come_out_the_same() {
    let _l = locks();
    let s = Scratch::new("model");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, big_json()).unwrap();
    let mut app = opened(&ctx, &path);
    let fresh = |app: &App| {
        let d = app.doc.as_ref().unwrap();
        rows(&values_view::build(root(&d.content).unwrap(), &d.names, None))
    };
    // A number: the list is the same, so it is kept.
    let gen = app.model_gen;
    app.apply_edit("/gold".into(), json!(1), 0.0);
    app.save_with(&path, AfterSave::Stay, true);
    wait(&mut app, &ctx);
    assert_eq!(app.model_gen, gen, "not built again");
    assert_eq!(rows(&app.model), fresh(&app));
    // A name an object is listed by: built again.
    app.apply_edit("/hero/name".into(), json!("Bea"), 1.0);
    app.save_with(&path, AfterSave::Stay, false);
    assert_ne!(app.model_gen, gen);
    assert_eq!(rows(&app.model), fresh(&app));
    assert!(app.model.sections.iter().any(|s| s.title.contains("Bea")));
    // A list built again since the file was saved (as after picking the data folder) is
    // never taken to be the saved file's.
    let gen = app.model_gen;
    app.rebuild_model();
    app.apply_edit("/gold".into(), json!(2), 2.0);
    app.save_with(&path, AfterSave::Stay, false);
    assert_eq!(app.model_gen, gen + 2);
    assert_eq!(rows(&app.model), fresh(&app));
}

fn gold_on_disk(path: &Path) -> Value {
    serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()["gold"].clone()
}

fn backed_up(path: &Path) -> Vec<Vec<u8>> {
    crate::backups::list(path).iter().map(|v| std::fs::read(&v.path).unwrap()).collect()
}

#[test]
fn a_save_never_silently_writes_over_a_file_the_game_changed() {
    let _l = locks();
    let s = Scratch::new("stale");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    // The game autosaves while the file is open.
    let newer = br#"{"gold": 50, "new": true}"#;
    std::fs::write(&path, newer).unwrap();
    app.save_to(&path);
    assert_eq!(std::fs::read(&path).unwrap(), newer, "the game's newer save is left alone");
    assert_eq!(app.stale_save.as_ref().map(|s| s.0.as_path()), Some(path.as_path()), "the player is asked first");
    assert!(app.dirty);
    // "Save anyway": the newer file is backed up before the player's version goes over it.
    app.stale_save = None;
    app.stale_ok = true;
    app.save_to(&path);
    assert!(!app.stale_ok);
    assert_eq!(gold_on_disk(&path), json!(2), "{}", app.status);
    assert!(backed_up(&path).contains(&newer.to_vec()), "the game's save can be brought back");
    assert!(!app.dirty);
    // Moved or deleted meanwhile: asked about too, and "Save anyway" puts it back.
    app.apply_edit("/gold".into(), json!(3), 1.0);
    std::fs::remove_file(&path).unwrap();
    app.save_to(&path);
    assert!(!path.exists());
    assert!(app.stale_save.is_some());
    app.stale_save = None;
    app.stale_ok = true;
    app.save_to(&path);
    assert_eq!(gold_on_disk(&path), json!(3));
}

/// "Save and quit" (or close, or open another file) that runs into "The save changed on
/// disk": Save anyway saves and then still quits, instead of leaving the window open.
#[test]
fn save_anyway_still_does_what_the_save_was_for() {
    let _l = locks();
    let s = Scratch::new("stale-then");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    std::fs::write(&path, br#"{"gold": 50, "new": true}"#).unwrap();
    app.save_then(AfterSave::Quit);
    let (p, then) = app.stale_save.clone().expect("asked first");
    assert_eq!(then, AfterSave::Quit);
    app.stale_answer(&p, then, 1);
    assert_eq!(gold_on_disk(&path), json!(2), "{}", app.status);
    assert!(app.close_now && app.allow_close, "the window closes as asked");
    // Cancel leaves everything as it is, and does nothing else either.
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(3), 0.0);
    std::fs::write(&path, br#"{"gold": 60, "newer": true}"#).unwrap();
    app.save_then(AfterSave::CloseFile);
    let (p, then) = app.stale_save.clone().expect("asked first");
    assert_eq!(then, AfterSave::CloseFile);
    app.stale_answer(&p, then, 2);
    assert!(app.doc.is_some() && app.dirty && !app.close_now);
    assert_eq!(gold_on_disk(&path), json!(60));
}

#[test]
fn an_outside_change_of_the_same_size_and_time_is_noticed_and_backed_up() {
    let _l = locks();
    let s = Scratch::new("hidden-change");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let mut app = opened(&ctx, &path);
    let when = std::fs::metadata(&path).unwrap().modified().unwrap();
    app.apply_edit("/gold".into(), json!(2), 0.0);
    // Same size and, on a file system with coarse times, the same time: the hash of what
    // was read tells them apart.
    let newer = br#"{"gold": 9}"#;
    std::fs::write(&path, newer).unwrap();
    std::fs::OpenOptions::new().write(true).open(&path).unwrap().set_modified(when).unwrap();
    assert!(!safety::changed_quick(&path, app.doc.as_ref().unwrap().stamp.as_ref()));
    assert!(safety::changed(&path, app.doc.as_ref().unwrap().stamp.as_ref()));
    app.save_to(&path);
    assert!(app.stale_save.is_some(), "asked before writing over it");
    assert_eq!(gold_on_disk(&path), json!(9), "nothing written yet");
    app.stale_save = None;
    app.stale_ok = true;
    app.save_to(&path);
    assert_eq!(gold_on_disk(&path), json!(2));
    assert!(backed_up(&path).contains(&newer.to_vec()), "what was on disk can still be brought back");
    // The stamp after saving is the written file's: saving again isn't mistaken for a change.
    assert!(!safety::changed(&path, app.doc.as_ref().unwrap().stamp.as_ref()));
}

#[test]
fn a_reload_that_fails_keeps_the_unsaved_changes_and_the_warning() {
    let _l = locks();
    let s = Scratch::new("reload-fails");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    app.stale_save = Some((path.clone(), AfterSave::Stay));
    app.disk_changed = true;
    // The file is gone (moved by cloud sync): nothing can be read back.
    std::fs::remove_file(&path).unwrap();
    app.reload_from_disk();
    assert!(app.dirty, "the changes are still there to save");
    assert_eq!(app.stale_save.as_ref().map(|s| s.0.as_path()), Some(path.as_path()));
    assert!(app.disk_changed);
    assert_eq!(app.doc.as_ref().and_then(|d| root(&d.content)).and_then(|v| v.pointer("/gold").cloned()), Some(json!(2)));
    // Back on disk: the reload goes through and drops them.
    std::fs::write(&path, br#"{"gold": 7}"#).unwrap();
    app.reload_from_disk();
    assert!(!app.dirty);
    assert!(app.stale_save.is_none() && !app.disk_changed);
    assert_eq!(app.doc.as_ref().and_then(|d| root(&d.content)).and_then(|v| v.pointer("/gold").cloned()), Some(json!(7)));
}

#[cfg(unix)]
#[test]
fn a_save_whose_first_backup_cant_be_made_writes_nothing() {
    let _l = locks();
    let s = Scratch::new("no-bak");
    let ctx = window();
    let path = s.0.join("save.json");
    let original = br#"{"gold": 1}"#;
    std::fs::write(&path, original).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    // The ".bak" copy leads nowhere, so it can't be written.
    std::os::unix::fs::symlink(s.0.join("no such folder").join("x"), s.0.join("save.json.bak")).unwrap();
    app.save_to(&path);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(app.status_kind, StatusKind::Err);
    assert!(app.status.starts_with("Couldn't save: the backup"), "{}", app.status);
    assert!(app.dirty);
    assert_eq!(value(&app, "/gold"), Some(json!(2)));
}

/// Runs only inside `a_full_disk_never_leaves_a_cut_off_original_backup`, under a file size
/// limit: opens the save in that test's folder, edits it and tries to save.
#[cfg(unix)]
#[test]
#[ignore]
fn full_disk_child() {
    let Some(dir) = std::env::var_os("USE_FULL_DISK_DIR").map(PathBuf::from) else { return };
    let _l = locks();
    let ctx = window();
    let path = dir.join("save.json");
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    app.save_to(&path);
    assert!(app.dirty, "the disk is full, so nothing can be saved: {}", app.status);
}

/// The first save keeps the file as it was in "<save>.bak", and never makes that copy again.
/// When the disk fills up while it is written, the cut-off copy must not stay behind as the
/// "original" (the player's only copy of it once the dated backups have been pruned).
#[cfg(unix)]
#[test]
fn a_full_disk_never_leaves_a_cut_off_original_backup() {
    let s = Scratch::new("full-disk");
    let path = s.0.join("save.json");
    let items: Vec<Value> = (0..3000).map(|i| json!({"name": format!("Item {i}"), "count": i})).collect();
    let original = serde_json::to_vec_pretty(&json!({"gold": 1, "items": items})).unwrap();
    assert!(original.len() > 64 << 10);
    std::fs::write(&path, &original).unwrap();
    let Ok(exe) = std::env::current_exe() else { return };
    // A full disk, as far as this one program can see: writes past 16 blocks fail
    // ("file too large") instead of stopping the program.
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg("trap '' XFSZ; ulimit -f 16 && exec \"$0\" \"$@\"")
        .arg(exe)
        .args(["--ignored", "--exact", "app::save_tests::full_disk_child", "--test-threads=1", "-q"])
        .env("USE_FULL_DISK_DIR", &s.0)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let Ok(status) = status else { return };
    assert!(status.success(), "the child test failed: {status}");
    assert_eq!(std::fs::read(&path).unwrap(), original, "the save is as it was");
    let bak = s.0.join("save.json.bak");
    if let Ok(b) = std::fs::read(&bak) {
        assert_eq!(b.len(), original.len(), "a cut-off backup was left as the original");
    }
    // With room again, the save goes through and the original is kept whole.
    let _l = locks();
    let ctx = window();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    app.save_to(&path);
    assert!(!app.dirty, "{}", app.status);
    assert_eq!(std::fs::read(&bak).unwrap(), original, "the original is kept whole");
}

#[test]
fn saving_again_and_undoing_after_a_save_keep_every_version() {
    let _l = locks();
    let s = Scratch::new("twice");
    let ctx = window();
    let path = s.0.join("save.json");
    let original = br#"{"gold": 1}"#.to_vec();
    std::fs::write(&path, &original).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    app.save_to(&path);
    let first = std::fs::read(&path).unwrap();
    app.apply_edit("/gold".into(), json!(3), 10.0);
    app.save_to(&path);
    assert_eq!(gold_on_disk(&path), json!(3));
    assert_eq!(std::fs::read(s.0.join("save.json.bak")).unwrap(), original, "the .bak copy is the original, never replaced");
    assert_eq!(backed_up(&path), vec![first.clone(), original.clone()]);
    // Saving again with nothing new keeps the file on disk once, never twice.
    app.save_to(&path);
    app.save_to(&path);
    assert_eq!(backed_up(&path).len(), 3);
    assert_eq!(gold_on_disk(&path), json!(3));
    // Undo after saving: the change comes back as unsaved, and saving it keeps the newer file too.
    app.undo_redo(true);
    assert_eq!(value(&app, "/gold"), Some(json!(2)));
    assert!(app.dirty);
    let third = std::fs::read(&path).unwrap();
    app.save_to(&path);
    assert_eq!(gold_on_disk(&path), json!(2));
    assert!(!app.dirty);
    assert_eq!(backed_up(&path)[0], third);
}

#[test]
fn save_as_backs_up_the_file_it_replaces_and_a_failed_one_changes_nothing() {
    let _l = locks();
    let s = Scratch::new("save-as");
    let ctx = window();
    let path = s.0.join("slot1.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let other = s.0.join("slot2.json");
    let other_bytes = br#"{"gold": 77, "slot": 2}"#;
    std::fs::write(&other, other_bytes).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(5), 0.0);
    // Into a folder that doesn't exist: an error, and the open file is still the same one.
    let nowhere = s.0.join("missing").join("copy.json");
    app.save_to(&nowhere);
    assert_eq!(app.status_kind, StatusKind::Err);
    assert!(!nowhere.exists() && !s.0.join("missing").exists());
    assert_eq!(app.doc.as_ref().unwrap().path, path);
    assert!(app.dirty);
    // Over another save: that save is backed up before it is replaced.
    app.save_to(&other);
    assert_eq!(gold_on_disk(&other), json!(5), "{}", app.status);
    assert_eq!(backed_up(&other), vec![other_bytes.to_vec()]);
    assert_eq!(std::fs::read(s.0.join("slot2.json.bak")).unwrap(), other_bytes);
    assert_eq!(gold_on_disk(&path), json!(1), "the file it was opened from is untouched");
    assert_eq!(app.doc.as_ref().unwrap().path, other);
}

#[test]
fn a_companion_file_that_cant_be_written_puts_the_save_back() {
    let _l = locks();
    let s = Scratch::new("companion");
    let path = s.0.join("data0001.bin");
    let original = br#"{"gold": 1}"#.to_vec();
    std::fs::write(&path, &original).unwrap();
    // Persona 4 Golden keeps the save's MD5 in the .binslot beside it.
    let mut slot = b"SAVE0001".to_vec();
    slot.resize(0x40, 0);
    let slot_path = s.0.join("data0001.binslot");
    std::fs::write(&slot_path, &slot).unwrap();
    let opts = formats::Options::for_path(&path);
    let d = formats::decode_with(&original, &opts);
    let edited = Content::Json { value: json!({"gold": 2}), pretty: false };
    // The .binslot can't be replaced.
    std::fs::create_dir_all(s.0.join("data0001.binslot.partial").join("x")).unwrap();
    let r = write_save(&path, &d.layers, &edited, &opts);
    assert!(r.is_err(), "reported as not saved");
    assert_eq!(std::fs::read(&path).unwrap(), original, "the save still matches its .binslot");
    assert_eq!(std::fs::read(&slot_path).unwrap(), slot);
    // Once it can be, both are written.
    std::fs::remove_dir_all(s.0.join("data0001.binslot.partial")).unwrap();
    let w = write_save(&path, &d.layers, &edited, &opts).unwrap();
    assert_eq!(w.stamp, safety::stamp(&path));
    use md5::Digest;
    assert_eq!(std::fs::read(&slot_path).unwrap()[0x18..0x28], md5::Md5::digest(std::fs::read(&path).unwrap())[..]);
}

/// A Persona 4 Golden-style save (JSON here) with the .binslot that holds its MD5.
fn save_with_binslot(s: &Scratch) -> (PathBuf, PathBuf, Vec<u8>) {
    let path = s.0.join("data0001.bin");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let mut slot = b"SAVE0001".to_vec();
    slot.resize(0x40, 0);
    let slot_path = s.0.join("data0001.binslot");
    std::fs::write(&slot_path, &slot).unwrap();
    (path, slot_path, slot)
}

#[test]
fn saving_again_after_a_rolled_back_save_does_not_ask_about_changes_on_disk() {
    let _l = locks();
    let s = Scratch::new("rollback-stamp");
    let ctx = window();
    let (path, slot_path, _) = save_with_binslot(&s);
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    std::fs::create_dir_all(s.0.join("data0001.binslot.partial").join("x")).unwrap();
    app.save_to(&path);
    assert_eq!(app.status_kind, StatusKind::Err);
    assert!(app.status.starts_with("Couldn't write"), "{}", app.status);
    assert_eq!(gold_on_disk(&path), json!(1));
    assert!(!safety::changed(&path, app.doc.as_ref().unwrap().stamp.as_ref()), "the file is as it was opened, time included");
    std::fs::remove_dir_all(s.0.join("data0001.binslot.partial")).unwrap();
    app.save_to(&path);
    assert!(app.stale_save.is_none(), "no question about a change nobody else made");
    assert_eq!(gold_on_disk(&path), json!(2), "{}", app.status);
    assert!(!app.dirty);
    use md5::Digest;
    assert_eq!(std::fs::read(&slot_path).unwrap()[0x18..0x28], md5::Md5::digest(std::fs::read(&path).unwrap())[..]);
}

#[test]
fn a_restore_whose_companion_cant_be_written_says_so_and_changes_nothing() {
    let _l = locks();
    let s = Scratch::new("restore-companion");
    let ctx = window();
    let (path, slot_path, slot) = save_with_binslot(&s);
    let from = s.0.join("older.sebak");
    std::fs::write(&from, br#"{"gold": 0}"#).unwrap();
    let mut app = opened(&ctx, &path);
    std::fs::create_dir_all(s.0.join("data0001.binslot.partial").join("x")).unwrap();
    app.restore_from(&path, &from, "older");
    assert_eq!(app.status_kind, StatusKind::Err, "{}", app.status);
    assert!(app.status.starts_with("Couldn't restore the backup"), "{}", app.status);
    assert_eq!(gold_on_disk(&path), json!(1), "the save still matches its .binslot");
    assert_eq!(std::fs::read(&slot_path).unwrap(), slot);
    assert!(!safety::changed(&path, app.doc.as_ref().unwrap().stamp.as_ref()));
    // Once it can be written, the restore goes through and both match.
    std::fs::remove_dir_all(s.0.join("data0001.binslot.partial")).unwrap();
    app.restore_from(&path, &from, "older");
    assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
    assert_eq!(gold_on_disk(&path), json!(0));
    use md5::Digest;
    assert_eq!(std::fs::read(&slot_path).unwrap()[0x18..0x28], md5::Md5::digest(std::fs::read(&path).unwrap())[..]);
}

#[test]
fn exporting_over_an_existing_file_backs_it_up_first() {
    let _l = locks();
    let s = Scratch::new("export");
    let ctx = window();
    // A gzip-packed save: its decoded data is not something the game can load.
    use std::io::Write;
    let mut e = flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
    e.write_all(br#"{"gold": 1}"#).unwrap();
    let packed = e.finish().unwrap();
    let path = s.0.join("save.dat");
    std::fs::write(&path, &packed).unwrap();
    let app = &mut opened(&ctx, &path);
    // Picked the save itself in the export dialog and confirmed the replace.
    app.export_to(&path);
    assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
    assert_eq!(gold_on_disk(&path), json!(1), "the decoded data was written");
    assert!(backed_up(&path).contains(&packed), "the packed save can be brought back");
    // Exporting to a new file makes no backups.
    let out = s.0.join("out.json");
    app.export_to(&out);
    assert!(out.exists() && crate::backups::list(&out).is_empty());
}

#[test]
fn a_restore_with_unsaved_changes_is_refused() {
    let _l = locks();
    let s = Scratch::new("restore-dirty");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let from = s.0.join("old.json");
    std::fs::write(&from, br#"{"gold": 0}"#).unwrap();
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(2), 0.0);
    app.restore_from(&path, &from, "old");
    assert_eq!(app.status_kind, StatusKind::Err);
    assert_eq!(gold_on_disk(&path), json!(1));
    assert_eq!(value(&app, "/gold"), Some(json!(2)), "the changes are still there");
    assert!(crate::backups::list(&path).is_empty());
    // Without changes it goes through, and what it replaced is kept.
    app.revert_all();
    app.restore_from(&path, &from, "old");
    assert_eq!(gold_on_disk(&path), json!(0), "{}", app.status);
    assert_eq!(value(&app, "/gold"), Some(json!(0)));
    assert_eq!(backed_up(&path), vec![br#"{"gold": 1}"#.to_vec()]);
}

/// One frame of a 1000×700 window with `events`, the input going through `raw_input_hook`.
fn run(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0))), ..Default::default() };
    app.raw_input_hook(ctx, &mut raw);
    ctx.run(raw, |ctx| app.frame(ctx))
}

/// Every piece of text drawn in a frame, with the middle of where it was drawn.
fn texts(out: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
    let mut found = vec![];
    for cs in &out.shapes {
        let mut stack = vec![&cs.shape];
        while let Some(sh) = stack.pop() {
            match sh {
                egui::epaint::Shape::Vec(v) => stack.extend(v.iter()),
                egui::epaint::Shape::Text(t) => found.push((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2()).center())),
                _ => {}
            }
        }
    }
    found
}

fn shown(app: &mut App, ctx: &egui::Context, text: &str) -> bool {
    (0..2).any(|_| texts(&run(app, ctx, vec![])).iter().any(|(t, _)| t.contains(text)))
}

/// Clicks the text starting with `text`, as a player would with the mouse.
fn click(app: &mut App, ctx: &egui::Context, text: &str) {
    // A window that just opened is sized on its first frame and drawn on the next.
    let mut all = vec![];
    for _ in 0..3 {
        all = texts(&run(app, ctx, vec![]));
        if all.iter().any(|(t, _)| t.starts_with(text)) {
            break;
        }
    }
    let at = all.iter().find(|(t, _)| t.starts_with(text)).unwrap_or_else(|| panic!("{text:?} is not shown; shown: {:?}", all.iter().map(|t| &t.0).collect::<Vec<_>>())).1;
    run(app, ctx, vec![egui::Event::PointerMoved(at)]);
    let press = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
    run(app, ctx, vec![press(true), press(false)]);
    run(app, ctx, vec![]);
}

fn key(app: &mut App, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    run(app, ctx, vec![egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }]);
    run(app, ctx, vec![egui::Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers }]);
    run(app, ctx, vec![]);
}

/// Opens Backups › Quick restore and clicks the entry starting with `entry`.
fn quick_restore(app: &mut App, ctx: &egui::Context, entry: &str) {
    click(app, ctx, "Backups");
    click(app, ctx, "Quick restore");
    click(app, ctx, entry);
}

/// A save with one automatic backup of an older version and a .bak of the first one.
fn save_with_backups(dir: &Path) -> PathBuf {
    let path = dir.join("save.json");
    std::fs::write(&path, br#"{"gold": 0}"#).unwrap();
    crate::backups::take(&path).unwrap();
    std::fs::write(dir.join("save.json.bak"), br#"{"gold": -1}"#).unwrap();
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    path
}

#[test]
fn quick_restore_asks_first_and_cancelling_changes_nothing() {
    let _l = locks();
    let s = Scratch::new("quick-ask");
    let ctx = window();
    let path = save_with_backups(&s.0);
    let before = backed_up(&path);
    let mut app = opened(&ctx, &path);
    quick_restore(&mut app, &ctx, "just now");
    assert_eq!(gold_on_disk(&path), json!(1), "clicking the backup alone restores nothing");
    assert!(shown(&mut app, &ctx, "save.json"), "the dialog names the save");
    assert!(shown(&mut app, &ctx, "Restore now"));
    // Focus starts on Cancel: Enter cancels.
    key(&mut app, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(!shown(&mut app, &ctx, "Restore now"), "the dialog closed");
    assert_eq!(gold_on_disk(&path), json!(1));
    assert_eq!(backed_up(&path), before, "no backup was made or removed");
    // Escape cancels too.
    quick_restore(&mut app, &ctx, "just now");
    key(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!shown(&mut app, &ctx, "Restore now"));
    assert_eq!(gold_on_disk(&path), json!(1));
    assert_eq!(backed_up(&path), before);
    // Cancel with the mouse.
    quick_restore(&mut app, &ctx, "just now");
    click(&mut app, &ctx, "Cancel");
    assert!(!shown(&mut app, &ctx, "Restore now"));
    assert_eq!(gold_on_disk(&path), json!(1));
    assert_eq!(backed_up(&path), before);
    // Restore now puts the backup back, after backing up the save it replaces.
    quick_restore(&mut app, &ctx, "just now");
    click(&mut app, &ctx, "Restore now");
    assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
    assert_eq!(gold_on_disk(&path), json!(0));
    assert!(backed_up(&path).contains(&br#"{"gold": 1}"#.to_vec()));
}

#[test]
fn restoring_the_original_save_asks_first_and_keyboard_confirms() {
    let _l = locks();
    let s = Scratch::new("quick-bak");
    let ctx = window();
    let path = save_with_backups(&s.0);
    let mut app = opened(&ctx, &path);
    quick_restore(&mut app, &ctx, "Original save (.bak)");
    assert_eq!(gold_on_disk(&path), json!(1));
    assert!(shown(&mut app, &ctx, "the original save"), "the dialog says which backup");
    // Shift+Tab from Cancel reaches Restore now; Enter presses it.
    key(&mut app, &ctx, egui::Key::Tab, egui::Modifiers::SHIFT);
    key(&mut app, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(gold_on_disk(&path), json!(-1), "{}", app.status);
    assert!(backed_up(&path).contains(&br#"{"gold": 1}"#.to_vec()));
}

#[test]
fn quick_restore_with_unsaved_changes_is_refused_without_asking() {
    let _l = locks();
    let s = Scratch::new("quick-dirty");
    let ctx = window();
    let path = save_with_backups(&s.0);
    let before = backed_up(&path);
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(5), 0.0);
    quick_restore(&mut app, &ctx, "just now");
    assert_eq!(app.status_kind, StatusKind::Err);
    assert!(!shown(&mut app, &ctx, "Restore now"), "nothing to confirm");
    assert_eq!(gold_on_disk(&path), json!(1));
    assert_eq!(backed_up(&path), before);
    assert_eq!(value(&app, "/gold"), Some(json!(5)), "the changes are still there");
}

#[test]
fn deleting_a_preset_asks_first() {
    let _l = locks();
    let s = Scratch::new("preset-delete");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    let file = s.0.join("rich.json");
    let preset = crate::presets::Preset { name: "Rich".into(), game: String::new(), values: vec![], bytes: vec![], built_in: false };
    std::fs::write(&file, b"{}").unwrap();
    let mut app = opened(&ctx, &path);
    let open = |app: &mut App| app.preset_apply = Some(PresetApply { list: vec![(file.clone(), preset.clone())], picked: Some(0), opened: None, confirm_delete: None });
    open(&mut app);
    click(&mut app, &ctx, "Delete preset");
    assert!(file.exists(), "clicking Delete preset alone deletes nothing");
    assert!(shown(&mut app, &ctx, "Rich"), "the dialog names the preset");
    assert!(shown(&mut app, &ctx, "Delete permanently"));
    // Focus starts on Cancel: Enter cancels, back to the list.
    key(&mut app, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(file.exists());
    assert!(!shown(&mut app, &ctx, "Delete permanently"));
    assert!(shown(&mut app, &ctx, "Delete preset"), "the preset list is still open");
    // Escape cancels the question only.
    click(&mut app, &ctx, "Delete preset");
    key(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(file.exists());
    assert!(app.preset_apply.is_some(), "Escape closed only the question");
    assert!(shown(&mut app, &ctx, "Delete preset"));
    // Delete permanently deletes it.
    click(&mut app, &ctx, "Delete preset");
    click(&mut app, &ctx, "Delete permanently");
    assert!(!file.exists(), "{}", app.status);
    assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
    assert!(app.preset_apply.as_ref().is_some_and(|pa| pa.list.is_empty()));
}

/// Two tabs, `a` showing, each a save with a backup and a .bak (see [`save_with_backups`]).
fn two_tabs(ctx: &egui::Context, s: &Scratch) -> (App, PathBuf, PathBuf) {
    let (da, db) = (s.0.join("a"), s.0.join("b"));
    std::fs::create_dir_all(&da).unwrap();
    std::fs::create_dir_all(&db).unwrap();
    let (a, b) = (save_with_backups(&da), save_with_backups(&db));
    let mut app = opened(ctx, &a);
    app.open_in_tab(b.clone());
    for _ in 0..400 {
        if app.loading.is_none() {
            break;
        }
        run(&mut app, ctx, vec![]);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.switch_tab(0);
    assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(a.clone()));
    (app, a, b)
}

fn on_disk(path: &Path) -> (Vec<u8>, Vec<Vec<u8>>) {
    (std::fs::read(path).unwrap(), backed_up(path))
}

#[test]
fn close_save_asks_first_and_never_drops_unsaved_changes_silently() {
    let _l = locks();
    let s = Scratch::new("close-save");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 5}"#).unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut app = opened(&ctx, &path);
    assert!(shown(&mut app, &ctx, "Close save"), "the toolbar has the button");
    app.apply_edit("/gold".into(), json!(9), 0.0);
    assert!(app.dirty);
    // Unsaved changes: the button asks; Cancel keeps everything.
    click(&mut app, &ctx, "Close save");
    assert!(app.pending_close && app.doc.is_some());
    click(&mut app, &ctx, "Cancel");
    assert!(app.doc.is_some() && app.dirty && !app.undo.is_empty(), "cancel changed nothing");
    // Ctrl+W asks too.
    key(&mut app, &ctx, egui::Key::W, egui::Modifiers::COMMAND);
    assert!(app.pending_close && app.doc.is_some());
    // Discarding closes without writing, and the next save opens clean.
    click(&mut app, &ctx, "Close without saving");
    assert!(app.doc.is_none() && !app.dirty && app.undo.is_empty() && app.redo.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), before, "the file on disk is untouched");
    assert!(!shown(&mut app, &ctx, "Close save"), "no button on the start page");
    // Save and close writes first.
    let mut app = opened(&ctx, &path);
    app.apply_edit("/gold".into(), json!(7), 0.0);
    click(&mut app, &ctx, "Close save");
    click(&mut app, &ctx, "Save and close");
    wait(&mut app, &ctx);
    assert!(app.doc.is_none());
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(7));
    // Nothing unsaved: closes straight away.
    let mut app = opened(&ctx, &path);
    key(&mut app, &ctx, egui::Key::W, egui::Modifiers::COMMAND);
    assert!(app.doc.is_none() && !app.pending_close);
}

#[test]
fn close_save_is_reachable_at_phone_width() {
    let _l = locks();
    let s = Scratch::new("close-save-phone");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 5}"#).unwrap();
    let mut app = opened(&ctx, &path);
    let mut raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(380.0, 700.0))), ..Default::default() };
    app.raw_input_hook(&ctx, &mut raw);
    let out = ctx.run(raw, |ctx| app.frame(ctx));
    let at = texts(&out).into_iter().find(|(t, _)| t == "\u{00D7}").expect("a close button on the phone toolbar").1;
    assert!(at.x > 0.0 && at.x < 380.0, "inside the screen: {at:?}");
}

#[test]
fn tab_keys_under_a_quick_restore_never_restore_over_another_save() {
    let _l = locks();
    for (name, k, shift) in [("ctrl-w", egui::Key::W, false), ("ctrl-tab", egui::Key::Tab, false), ("ctrl-shift-tab", egui::Key::Tab, true)] {
        let s = Scratch::new(&format!("quick-tabs-{name}"));
        let ctx = window();
        let (mut app, a, b) = two_tabs(&ctx, &s);
        let (a_before, b_before) = (on_disk(&a), on_disk(&b));
        quick_restore(&mut app, &ctx, "just now");
        let mods = if shift { egui::Modifiers::COMMAND | egui::Modifiers::SHIFT } else { egui::Modifiers::COMMAND };
        key(&mut app, &ctx, k, mods);
        assert_eq!(app.tab_count(), 2, "{name}: the tab under the dialog stays open");
        assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(a.clone()), "{name}: the tab under the dialog stays in front");
        assert!(shown(&mut app, &ctx, "Restore now"), "{name}: the dialog is still there, about a");
        assert_eq!((on_disk(&a), on_disk(&b)), (a_before.clone(), b_before.clone()), "{name}: nothing changed on disk");
        // Confirming restores a, and only a.
        click(&mut app, &ctx, "Restore now");
        assert_eq!(gold_on_disk(&a), json!(0), "{name}: {}", app.status);
        assert_eq!(on_disk(&b), b_before, "{name}: b untouched");
    }
}

#[test]
fn a_quick_restore_is_dropped_when_its_save_is_no_longer_in_front() {
    let _l = locks();
    let s = Scratch::new("quick-switch");
    let ctx = window();
    let (mut app, a, b) = two_tabs(&ctx, &s);
    let (a_before, b_before) = (on_disk(&a), on_disk(&b));
    // Another tab comes to the front by any way at all.
    quick_restore(&mut app, &ctx, "just now");
    app.switch_tab(1);
    assert!(app.quick_restore.is_none(), "switching tabs drops the question");
    app.switch_tab(0);
    assert!(!shown(&mut app, &ctx, "Restore now"), "and it doesn't come back with the tab");
    // The tab is closed.
    quick_restore(&mut app, &ctx, "just now");
    app.drop_tab(0);
    assert!(app.quick_restore.is_none(), "closing the tab drops the question");
    assert!(!shown(&mut app, &ctx, "Restore now"));
    assert_eq!((on_disk(&a), on_disk(&b)), (a_before.clone(), b_before.clone()));
    // Another file replaces it in the same tab: the dialog closes itself and restores nothing.
    app.quick_restore = Some((a.clone(), list_first(&a), "the backup".into()));
    assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(b.clone()));
    run(&mut app, &ctx, vec![]);
    assert!(app.quick_restore.is_none());
    assert_eq!((on_disk(&a), on_disk(&b)), (a_before, b_before));
}

fn list_first(save: &Path) -> PathBuf {
    crate::backups::list(save)[0].path.clone()
}

#[test]
fn a_preset_that_cant_be_deleted_stays_in_the_list() {
    let _l = locks();
    let s = Scratch::new("preset-delete-fails");
    let ctx = window();
    let path = s.0.join("save.json");
    std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
    // A folder with something in it where the preset file should be: removing it fails.
    let file = s.0.join("rich.json");
    std::fs::create_dir_all(file.join("inside")).unwrap();
    let preset = crate::presets::Preset { name: "Rich".into(), game: String::new(), values: vec![], bytes: vec![], built_in: false };
    let mut app = opened(&ctx, &path);
    app.preset_apply = Some(PresetApply { list: vec![(file.clone(), preset)], picked: Some(0), opened: None, confirm_delete: None });
    click(&mut app, &ctx, "Delete preset");
    click(&mut app, &ctx, "Delete permanently");
    assert_eq!(app.status_kind, StatusKind::Err, "{}", app.status);
    assert!(file.exists());
    assert_eq!(app.preset_apply.as_ref().map(|pa| pa.list.len()), Some(1), "still listed");
    assert!(shown(&mut app, &ctx, "Rich"));
}

#[test]
fn values_list_shape() {
    let j = |v: Value| Content::Json { value: v, pretty: true };
    let same = |a: Value, b: Value| same_values_list(&j(a), &j(b));
    assert!(same(json!({"a": 1, "b": [true, "x"]}), json!({"a": 2.5, "b": [false, "y"]})));
    assert!(same(json!({"o": {"name": "A", "hp": 1}}), json!({"o": {"name": "A", "hp": 9}})));
    assert!(!same(json!({"o": {"name": "A"}}), json!({"o": {"name": "B"}})), "an object's short name");
    assert!(!same(json!({"o": {"title": "A", "n": 1}}), json!({"o": {"title": " ", "n": 1}})));
    assert!(!same(json!({"a": null}), json!({"a": 1})), "a null value has no row");
    assert!(!same(json!({"a": 1}), json!({"a": [1]})));
    assert!(!same(json!({"a": 1, "b": 2}), json!({"b": 2, "a": 1})), "order");
    assert!(!same(json!({"a": [1, 2]}), json!({"a": [1, 2, 3]})));
    assert!(!same(json!({"a": 1}), json!({"b": 1})));
    assert!(!same_values_list(&Content::Text("a".into()), &Content::Text("a".into())));
    assert!(!same_values_list(&j(json!(1)), &Content::Binary(vec![1])));
}

/// Browser version: a newly picked file's handle is used only when opening it shows that
/// file, so unsaved changes to another file of the same name can never be written into it.
#[test]
fn picked_handle_follows_only_the_file_that_opens() {
    let _locks = locks();
    let dir = Scratch::new("handle");
    let a = dir.0.join("a.json");
    let b = dir.0.join("b.json");
    std::fs::write(&a, br#"{"gold": 1}"#).unwrap();
    std::fs::write(&b, br#"{"gold": 2}"#).unwrap();
    let ctx = window();
    let mut app = opened(&ctx, &a);
    assert!(app.handle_may_follow(&b), "a file not open anywhere");
    assert!(app.handle_may_follow(&a), "the file on screen, with nothing unsaved, is read again");
    app.dirty = true;
    assert!(!app.handle_may_follow(&a), "unsaved changes to the file on screen");
    app.dirty = false;
    app.open_in_tab(b.clone());
    assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(b.clone()));
    assert!(!app.handle_may_follow(&a), "open in another tab, which is shown instead");
    assert!(app.handle_may_follow(&b));
}

/// The id of the handle the document on screen saves back through.
fn handle_of(app: &App) -> Option<u64> {
    app.doc.as_ref().and_then(|d| d.handle.as_ref().map(|h| h.0))
}

/// Opens `p` in a tab as the browser version does for "Open to save in place", handing the
/// file handle `id` to the document if it is the one that opened.
fn open_with_handle(app: &mut App, p: &Path, id: u64) -> bool {
    let loads = app.loads;
    app.open_in_tab(p.to_path_buf());
    app.attach_handle(p, id, loads)
}

/// A handle belongs to the document that opened the file: closing it lets the handle go,
/// and another file opened later under the same name (in the browser: the same path) has
/// none, so its saves download instead of writing into the first file.
#[test]
fn handle_goes_with_its_document_when_closed() {
    let _locks = locks();
    let dir = Scratch::new("handle-close");
    let p = dir.0.join("save.json");
    std::fs::write(&p, br#"{"gold": 1}"#).unwrap();
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    assert!(open_with_handle(&mut app, &p, 7));
    assert_eq!(handle_of(&app), Some(7));
    app.close_file();
    assert!(app.doc.is_none());
    // Another file of the same name, opened the ordinary way.
    std::fs::write(&p, br#"{"gold": 2}"#).unwrap();
    app.open_in_tab(p.clone());
    assert!(app.doc.is_some(), "{}", app.status);
    assert_eq!(handle_of(&app), None);
    // A handle can't be given to a document that didn't just open (nothing was read).
    assert!(!app.attach_handle(&p, 8, app.loads));
}

/// "Save a copy as" under another name leaves the document without its handle, and a
/// document saved under the name of a file opened in place never gets that file's handle.
#[test]
fn save_as_never_borrows_or_keeps_a_handle() {
    let _locks = locks();
    let dir = Scratch::new("handle-saveas");
    let save = dir.0.join("save.json");
    let slot = dir.0.join("slot1.json");
    std::fs::write(&save, br#"{"gold": 1}"#).unwrap();
    std::fs::write(&slot, br#"{"gold": 5}"#).unwrap();
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    assert!(open_with_handle(&mut app, &save, 3));
    app.open_in_tab(slot.clone());
    assert_eq!(app.tab_count(), 2);
    assert_eq!(handle_of(&app), None);
    // slot1 saved under save.json's name: still no handle; the next save goes to its own path.
    app.save_to(&save);
    assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(save.clone()), "{}", app.status);
    assert_eq!(handle_of(&app), None);
    // The tab that opened save.json still has its handle.
    app.switch_tab(0);
    assert_eq!(handle_of(&app), Some(3));
    // Saved as another file: the handle stays behind with the old name.
    let other = dir.0.join("copy.json");
    app.save_to(&other);
    assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(other));
    assert_eq!(handle_of(&app), None);
}

/// Two tabs with the same file: only the document that opened it holds the handle.
#[test]
fn two_tabs_with_the_same_file_name_share_no_handle() {
    let _locks = locks();
    let dir = Scratch::new("handle-tabs");
    let save = dir.0.join("save.json");
    let other = dir.0.join("other.json");
    std::fs::write(&save, br#"{"gold": 1}"#).unwrap();
    std::fs::write(&other, br#"{"gold": 2}"#).unwrap();
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    assert!(open_with_handle(&mut app, &other, 1));
    assert!(open_with_handle(&mut app, &save, 2));
    assert_eq!(app.tab_count(), 2);
    // Picking save.json again while its tab is open shows that tab; the new handle isn't used.
    app.switch_tab(0);
    assert!(!app.handle_may_follow(&save));
    assert!(!open_with_handle(&mut app, &save, 9));
    assert_eq!(app.tabs.active, 1);
    assert_eq!(handle_of(&app), Some(2));
    app.switch_tab(0);
    assert_eq!(handle_of(&app), Some(1));
}
