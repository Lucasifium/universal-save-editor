//! Layout guards: draws windows off-screen at several sizes and fails when text is cut off at the
//! side of its box or a window doesn't fit the screen.

use super::*;

/// Text that sticks out sideways past the area it was clipped to (so it is cut off or spills
/// into a neighbour). Returns one line per offender.
pub(crate) fn cut_off_text(out: &egui::FullOutput) -> Vec<String> {
    let mut bad = vec![];
    for cs in &out.shapes {
        let mut stack = vec![&cs.shape];
        while let Some(sh) = stack.pop() {
            match sh {
                egui::epaint::Shape::Vec(v) => stack.extend(v.iter()),
                egui::epaint::Shape::Text(t) => {
                    let r = t.galley.rect.translate(t.pos.to_vec2());
                    // 1 px for rounding; text under the top/bottom edge is just scrolled.
                    if r.right() > cs.clip_rect.right() + 1.0 || r.left() < cs.clip_rect.left() - 1.0 {
                        let s: String = t.galley.text().chars().take(50).collect();
                        bad.push(format!("{s:?} spans {:.0}..{:.0} but is clipped to {:.0}..{:.0}", r.left(), r.right(), cs.clip_rect.left(), cs.clip_rect.right()));
                    }
                }
                _ => {}
            }
        }
    }
    bad
}

fn input(w: f32, h: f32) -> egui::RawInput {
    egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() }
}

#[test]
fn find_my_games_saves_fits_every_size() {
    let long = "C:\\Users\\SomeOne\\AppData\\Local\\Packages\\Some.Very.Long.PublisherName.GameTitle_8wekyb3d8bbwe\\SystemAppData\\wgs\\0009000000AB1234_0000000000000000000000007B2E4C81\\Slot1";
    let found: Vec<Option<Vec<crate::games::Found>>> = crate::games::GAMES
        .iter()
        .map(|_| Some((0..6).map(|i| crate::games::Found { path: PathBuf::from(format!("{long}\\quicksave_{i}.sav")), modified: Some(std::time::SystemTime::now()), size: 1 << 20, label: None }).collect()))
        .collect();
    let found: SaveSearch = std::sync::Arc::new(std::sync::Mutex::new(found));
    // The longest names, the ones covering many games, and a couple of ordinary ones.
    let mut picks: Vec<usize> = (0..crate::games::GAMES.len()).collect();
    picks.sort_by_key(|&i| std::cmp::Reverse(crate::games::GAMES[i].name.len()));
    picks.truncate(6);
    picks.extend((0..crate::games::GAMES.len()).filter(|&i| crate::games::many_games(&crate::games::GAMES[i])).take(2));
    picks.push(0);
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut problems = vec![];
    for lang in crate::i18n::LANGS.iter().map(|l| l.code) {
    crate::i18n::set(Some(lang));
    for dark in [true, false] {
        for (w, h) in [(1280.0, 800.0), (1000.0, 700.0), (820.0, 600.0), (640.0, 480.0), (380.0, 760.0)] {
            for &pick in &picks {
                let ctx = egui::Context::default();
                crate::theme::install_fonts(&ctx);
                crate::theme::apply(&ctx, dark);
                let mut gp = GamePicker::with(found.clone());
                gp.only_found = Some(true);
                gp.picked = Some(pick);
                for frame in 0..4 {
                    let out = ctx.run(input(w, h), |ctx| {
                        game_picker_window(ctx, &mut gp);
                    });
                    if frame == 3 {
                        let id = egui::Id::new(tr("Find my game's saves"));
                        let area = ctx.memory(|m| m.area_rect(id)).unwrap();
                        let who = format!("{} at {w}x{h} dark={dark} {lang}", crate::games::GAMES[pick].name);
                        if area.left() < -0.5 || area.top() < -0.5 || area.right() > w + 0.5 || area.bottom() > h + 0.5 {
                            problems.push(format!("{who}: window {area:?} leaves the screen"));
                        }
                        for t in cut_off_text(&out) {
                            problems.push(format!("{who}: {t}"));
                        }
                    }
                }
            }
        }
    }
    }
    crate::i18n::set(Some("en"));
    problems.sort();
    problems.dedup();
    assert!(problems.is_empty(), "{} layout problems:\n{}", problems.len(), problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
}

fn window() -> egui::Context {
    let ctx = egui::Context::default();
    theme::install_fonts(&ctx);
    theme::apply(&ctx, true);
    ctx
}

/// A Hollow Knight save as plain JSON: a header card with tiles and several panels.
fn knight_save(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("use-phone-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user1.json");
    let pd = json!({"playerData": {"geo": 1234, "maxHealthBase": 5, "charmSlots": 3, "ore": 2, "simpleKeys": 1, "nailDamage": 5, "maxMP": 99, "MPReserveMax": 33, "health": 5, "maxHealth": 5, "hasDash": true, "playTime": 7200.0}});
    std::fs::write(&path, serde_json::to_vec_pretty(&pd).unwrap()).unwrap();
    path
}

fn panel_rect(ctx: &egui::Context, id: &str) -> Option<egui::Rect> {
    egui::containers::panel::PanelState::load(ctx, egui::Id::new(id)).map(|p| p.rect)
}

fn run_frames(app: &mut App, ctx: &egui::Context, w: f32, h: f32, n: usize) -> egui::FullOutput {
    let mut last = None;
    for _ in 0..n {
        let mut raw = input(w, h);
        // Time moves on, so scrolling animations finish.
        raw.time = Some(ctx.input(|i| i.time) + 0.5);
        eframe::App::raw_input_hook(app, ctx, &mut raw);
        last = Some(ctx.run(raw, |ctx| app.frame(ctx)));
    }
    last.unwrap()
}

/// The start page's game rows: the "Other saves" button sits at the right edge of an
/// equal-height row and nothing is cut off or runs under it, in every language and size.
#[test]
fn start_page_game_rows_stay_even_with_other_saves_button() {
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let long = "C:\\Users\\SomeOne\\AppData\\Local\\Packages\\Some.Very.Long.PublisherName.GameTitle_8wekyb3d8bbwe\\Slot1";
    let singles: Vec<usize> = (0..crate::games::GAMES.len()).filter(|&i| !crate::games::many_games(&crate::games::GAMES[i])).take(4).collect();
    let mut v: Vec<Option<Vec<crate::games::Found>>> = vec![Some(vec![]); crate::games::GAMES.len()];
    for (k, &i) in singles.iter().enumerate() {
        let n = if k % 2 == 0 { 5 } else { 1 };
        v[i] = Some((0..n).map(|j| crate::games::Found { path: PathBuf::from(format!("{long}\\quicksave_{j}.sav")), modified: Some(std::time::SystemTime::now()), size: 1 << 20, label: None }).collect());
    }
    let found: SaveSearch = std::sync::Arc::new(std::sync::Mutex::new(v));
    let mut problems = vec![];
    for lang in crate::i18n::LANGS.iter().map(|l| l.code) {
        crate::i18n::set(Some(lang));
        for (w, h) in [(1280.0, 800.0), (820.0, 600.0), (380.0, 760.0)] {
            let ctx = window();
            let mut app = App::with(&ctx, Settings::default(), false);
            app.saves_found = Some(found.clone());
            let mut out = None;
            for _ in 0..4 {
                out = Some(ctx.run(input(w, h), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.your_games(ui);
                    });
                }));
            }
            let who = format!("{lang} at {w}x{h}");
            let out = out.unwrap();
            for t in cut_off_text(&out) {
                problems.push(format!("{who}: {t}"));
            }
            // No two pieces of text may overlap each other.
            let mut texts: Vec<(egui::Rect, String)> = vec![];
            for cs in &out.shapes {
                let mut stack = vec![&cs.shape];
                while let Some(sh) = stack.pop() {
                    match sh {
                        egui::epaint::Shape::Vec(v) => stack.extend(v.iter()),
                        egui::epaint::Shape::Text(t) if !t.galley.text().trim().is_empty() => {
                            texts.push((t.galley.rect.translate(t.pos.to_vec2()).intersect(cs.clip_rect), t.galley.text().chars().take(30).collect()));
                        }
                        _ => {}
                    }
                }
            }
            for (i, a) in texts.iter().enumerate() {
                for b in &texts[i + 1..] {
                    let o = a.0.intersect(b.0);
                    if o.width() > 2.0 && o.height() > 2.0 {
                        problems.push(format!("{who}: {:?} overlaps {:?}", a.1, b.1));
                    }
                }
            }
        }
    }
    crate::i18n::set(Some("en"));
    problems.sort();
    problems.dedup();
    assert!(problems.is_empty(), "{} problems:\n{}", problems.len(), problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn phone_layout_has_one_toolbar_line_and_section_tabs_in_every_language() {
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = knight_save("chips");
    let mut problems = vec![];
    for lang in crate::i18n::LANGS.iter().map(|l| l.code) {
        crate::i18n::set(Some(lang));
        for (w, h) in [(360.0, 740.0), (412.0, 900.0), (600.0, 900.0)] {
            let ctx = window();
            let mut app = App::with(&ctx, Settings::default(), false);
            app.load(&path);
            assert_eq!(app.doc.as_ref().and_then(|d| d.game), Some(crate::game_ui::Game::HollowKnight), "{}", app.status);
            // An edit, so Save shows its count and a section gets the edited mark.
            app.apply_edit("/playerData/geo".into(), json!(5000), 0.0);
            let out = run_frames(&mut app, &ctx, w, h, 5);
            let who = format!("{lang} at {w}x{h}");
            if panel_rect(&ctx, "menu").is_some() {
                problems.push(format!("{who}: the menu bar is shown on a phone"));
            }
            match panel_rect(&ctx, "toolbar") {
                Some(r) if r.height() > 56.0 => problems.push(format!("{who}: toolbar is {} px tall (more than one line)", r.height())),
                None => problems.push(format!("{who}: no toolbar")),
                _ => {}
            }
            let secs = crate::game_ui::sections(&ctx);
            let titles: Vec<&str> = secs.iter().map(|s| s.title.as_str()).collect();
            if secs.len() < 3 {
                problems.push(format!("{who}: only {} sections: {titles:?}", secs.len()));
            }
            // The tabs wrap onto more lines, so none runs past the edge.
            for t in cut_off_text(&out) {
                problems.push(format!("{who}: {t}"));
            }
        }
    }
    crate::i18n::set(Some("en"));
    problems.sort();
    problems.dedup();
    assert!(problems.is_empty(), "{} problems:\n{}", problems.len(), problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
}

/// Each panel of a game screen is a tab: only the open one is drawn, the others sit
/// out of sight, and Advanced (Values and Tree) comes last.
#[test]
fn a_game_screen_shows_one_section_tab_at_a_time() {
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::i18n::set(Some("en"));
    let path = knight_save("tabs");
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    app.load(&path);
    let (w, h) = (1100.0, 900.0);
    let shown = |out: &egui::FullOutput| -> String {
        let mut all = String::new();
        let mut stack: Vec<&egui::epaint::Shape> = out.shapes.iter().map(|c| &c.shape).collect();
        while let Some(sh) = stack.pop() {
            match sh {
                egui::epaint::Shape::Vec(v) => stack.extend(v.iter()),
                egui::epaint::Shape::Text(t) => {
                    all.push_str(t.galley.text());
                    all.push('\n');
                }
                _ => {}
            }
        }
        all
    };
    run_frames(&mut app, &ctx, w, h, 4);
    let secs = crate::game_ui::sections(&ctx);
    assert!(secs.len() >= 2, "{secs:?}");
    let (first, second) = (secs[0].title.clone(), secs[1].title.clone());
    let on_first = shown(&run_frames(&mut app, &ctx, w, h, 3));
    assert!(on_first.contains(&format!("{first}\n")), "the first tab is open at first");
    assert!(!on_first.contains(&format!("{second}\n")) || on_first.matches(&format!("{second}\n")).count() == 1, "the second tab's panel is out of sight (only its tab label shows)");
    app.game_section = second.clone();
    let on_second = shown(&run_frames(&mut app, &ctx, w, h, 3));
    assert_eq!(on_second.matches(&format!("{second}\n")).count(), 2, "its tab label and its panel title");
    assert_eq!(on_second.matches(&format!("{first}\n")).count(), 1, "the first panel is gone, its tab label stays");
    assert_eq!(crate::game_ui::sections(&ctx), secs, "every section is still drawn, so the tabs stay");
}

/// Hidden tabs are not laid out again while the data stays the same, but their edit marks
/// and quick actions still follow every change, whichever tab is open.
#[test]
fn hidden_tabs_keep_their_marks_without_being_laid_out_again() {
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::i18n::set(Some("en"));
    let path = knight_save("tabcache");
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    app.load(&path);
    let (w, h) = (1100.0, 900.0);
    run_frames(&mut app, &ctx, w, h, 4);
    let titles: Vec<String> = crate::game_ui::sections(&ctx).iter().map(|s| s.title.clone()).collect();
    assert!(titles.len() >= 2);
    let marks = |app: &mut App| -> Vec<Vec<bool>> {
        titles
            .iter()
            .map(|t| {
                app.game_section = t.clone();
                run_frames(app, &ctx, w, h, 3);
                crate::game_ui::sections(&ctx).iter().map(|s| s.edited).collect()
            })
            .collect()
    };
    let before = marks(&mut app);
    assert!(before.iter().all(|m| m.iter().all(|e| !e)), "nothing edited yet: {before:?}");
    // Nail damage is on the Nail & soul tab, which isn't the first one.
    app.apply_edit("/playerData/nailDamage".into(), json!(9), 0.0);
    let after = marks(&mut app);
    let nail = titles.iter().position(|t| t == "Nail & soul").expect("a Nail & soul tab");
    assert!(after[0][nail] && after[0].iter().filter(|&&e| e).count() == 1, "the edit is marked on its tab only: {after:?}");
    assert!(after.iter().all(|m| *m == after[0]), "the same marks whichever tab is open: {after:?}");
    app.undo_redo(true);
    let undone = marks(&mut app);
    assert_eq!(undone, before, "undo takes the mark away again");
}

#[test]
fn desktop_keeps_its_menu_bar_and_save_shows_the_count() {
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::i18n::set(Some("en"));
    let path = knight_save("desk");
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    app.load(&path);
    app.apply_edit("/playerData/geo".into(), json!(5000), 0.0);
    let out = run_frames(&mut app, &ctx, 1280.0, 800.0, 4);
    assert!(panel_rect(&ctx, "menu").is_some(), "the menu bar is missing on a wide window");
    assert!(panel_rect(&ctx, "toolbar").is_some_and(|r| r.height() < 56.0), "the toolbar wrapped on a wide window");
    assert_eq!(app.save_label(), "Save (1)");
    assert!(cut_off_text(&out).is_empty(), "{:?}", cut_off_text(&out));
}

#[test]
fn a_dismissed_file_note_stays_hidden_and_comes_back_from_the_view_menu() {
    let ctx = window();
    let mut app = App::with(&ctx, Settings::default(), false);
    app.doc_note = "Level, experience and gold.".into();
    assert!(!app.note_hidden());
    app.settings.hidden_notes.push(Settings::note_key(&app.doc_note));
    assert!(app.note_hidden());
    // Another file with the same note: still hidden.
    app.doc_note = "Level, experience and gold.".into();
    assert!(app.note_hidden());
    app.doc_note = "Something else.".into();
    assert!(!app.note_hidden());
}

/// Every piece of text a frame painted, with where it sits, in reading order.
fn text_layout(out: &egui::FullOutput) -> Vec<(String, [i32; 4])> {
    let mut v = vec![];
    for cs in &out.shapes {
        let mut stack = vec![&cs.shape];
        while let Some(sh) = stack.pop() {
            match sh {
                egui::epaint::Shape::Vec(x) => stack.extend(x.iter()),
                egui::epaint::Shape::Text(t) if !t.galley.text().trim().is_empty() => {
                    let r = t.galley.rect.translate(t.pos.to_vec2());
                    v.push((t.galley.text().to_string(), [r.left().round() as i32, r.top().round() as i32, r.right().round() as i32, r.bottom().round() as i32]));
                }
                _ => {}
            }
        }
    }
    v.sort();
    v
}

/// Shrinking the window (to a phone size and back) must leave the same layout as a window
/// that was never shrunk: nothing remembered from the small size may linger.
#[test]
fn growing_the_window_back_restores_the_layout() {
    let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::i18n::set(Some("en"));
    let path = knight_save("regrow");
    let mut problems = vec![];
    for (big, small) in [((1280.0, 800.0), (380.0, 760.0)), ((1280.0, 800.0), (700.0, 600.0)), ((900.0, 700.0), (500.0, 700.0))] {
        let fresh_ctx = window();
        let mut fresh = App::with(&fresh_ctx, Settings::default(), false);
        fresh.load(&path);
        let want = text_layout(&run_frames(&mut fresh, &fresh_ctx, big.0, big.1, 8));
        let ctx = window();
        let mut app = App::with(&ctx, Settings::default(), false);
        app.load(&path);
        run_frames(&mut app, &ctx, big.0, big.1, 8);
        run_frames(&mut app, &ctx, small.0, small.1, 8);
        let got = text_layout(&run_frames(&mut app, &ctx, big.0, big.1, 8));
        if got != want {
            let missing: Vec<_> = want.iter().filter(|x| !got.contains(x)).take(6).collect();
            let extra: Vec<_> = got.iter().filter(|x| !want.contains(x)).take(6).collect();
            problems.push(format!("{big:?} -> {small:?} -> {big:?}: differs from a fresh window\n  expected {missing:?}\n  got {extra:?}"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
