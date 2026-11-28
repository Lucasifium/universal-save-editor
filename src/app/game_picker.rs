//! The "Find my game's saves" window and the background search behind it.

use super::*;

/// The "Find my game's saves" dialog: the game list, which game is picked, and the saves found for
/// each game (filled in by a background search).
pub(super) struct GamePicker {
    pub(super) query: String,
    pub(super) picked: Option<usize>,
    /// For entries covering many games (Ren'Py, RPG Maker…): the game whose saves are shown.
    pub(super) group: Option<String>,
    pub(super) found: SaveSearch,
    /// "Only games found here": `None` until the player changes it (then on once the search
    /// finds something).
    pub(super) only_found: Option<bool>,
    /// Only games with this much support.
    pub(super) level: Option<crate::games::Level>,
    /// Which game each save found for the picked entry belongs to, for (entry, saves found).
    /// Working it out looks through folders, so it is done once rather than on every frame.
    pub(super) save_groups: Option<((usize, usize), Vec<String>)>,
    /// A remembered save that was picked but is no longer there (offers a new search).
    pub(super) missing: Option<PathBuf>,
    /// The starred games (copied from the settings while the window is open).
    pub(super) favs: Vec<String>,
    /// Only games of this genre (a key of `genres::GAME_GENRES`).
    pub(super) genre: Option<&'static str>,
    /// The search box takes the focus when the window opens.
    pub(super) focus_search: bool,
}

/// The saves found for each game in the catalog (`None`: not looked at yet).
pub(super) type SaveSearch = std::sync::Arc<std::sync::Mutex<Vec<Option<Vec<crate::games::Found>>>>>;

/// Looks for every game's saves on a worker thread. Folders where saves were found before are
/// checked first and are all that's looked at while the last full search is recent; the whole
/// computer is searched on the first start, once a week, and when asked (`rescan`).
pub(super) fn search_saves() -> SaveSearch {
    let found: SaveSearch = std::sync::Arc::new(std::sync::Mutex::new(vec![None; crate::games::GAMES.len()]));
    spawn_search(&found, false);
    found
}

/// Starts a search that fills `found` in (a new full one when `full`).
pub(super) fn spawn_search(found: &SaveSearch, full: bool) {
    let out = found.clone();
    std::thread::spawn(move || {
        crate::drives::forget_lookups();
        let mut cache = crate::save_cache::SaveCache::load();
        let quick = !full && cache.fresh();
        crate::drives::set_export_prefs(!quick);
        if !quick {
            if let Ok(mut v) = out.lock() {
                v.iter_mut().for_each(|x| *x = None);
            }
        }
        for (i, g) in crate::games::GAMES.iter().enumerate() {
            let known = cache.games.get(g.name).cloned().unwrap_or_default();
            // Quick: the remembered folders first, then the usual places again (a game or a new
            // account installed since the last full search has no remembered folder yet).
            let (f, locs) = crate::games::find_remembering(g, &known, quick);
            if locs.is_empty() {
                cache.games.remove(g.name);
            } else {
                cache.games.insert(g.name.to_string(), locs);
            }
            if let Ok(mut v) = out.lock() {
                v[i] = Some(f);
            }
        }
        if !quick {
            cache.mark_searched();
        }
        cache.save();
    });
}

impl GamePicker {
    pub(super) fn with(found: SaveSearch) -> Self {
        Self { query: String::new(), picked: None, group: None, found, only_found: None, level: None, save_groups: None, missing: None, favs: vec![], genre: None, focus_search: true }
    }
}

/// How many found-save cards the game picker draws before offering "Show all".
pub(super) const CARDS_SHOWN: usize = 60;

/// Draws the "Find my game's saves" window; returns the save picked to open and whether it was closed.
pub(super) fn game_picker_window(ctx: &egui::Context, gp: &mut GamePicker) -> (Option<PathBuf>, bool) {
        let found = gp.found.lock().map(|v| v.clone()).unwrap_or_default();
        if found.iter().any(Option::is_none) {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        let mut open = None;
        let mut close = false;
        let screen = ctx.screen_rect();
        let wide = screen.width() > 820.0;
        // A fixed size that always fits: the lists inside fill whatever they're given.
        let size = egui::vec2((screen.width() - 40.0).min(900.0), (screen.height() - 90.0).min(620.0));
        egui::Window::new(tr("Find my game's saves"))
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let searching = found.iter().any(Option::is_none);
                let here = |i: usize| found[i].as_ref().is_some_and(|f| !f.is_empty());
                let any_here = (0..found.len()).any(here);
                let only = gp.only_found.unwrap_or(!searching && any_here);
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("Search:"));
                    let search = ui.add(TextEdit::singleline(&mut gp.query).hint_text(tr("game name")).desired_width(200.0));
                    // Ready to type a name as soon as the window opens.
                    if std::mem::take(&mut gp.focus_search) {
                        search.request_focus();
                    }
                    search.on_hover_text(tr("Words in any order work (\"witcher 3\" or \"3 witcher\"), and so do short names made from the initials of the name with its numbers, such as bg3, gta5 or ff7."));
                    let mut o = only;
                    if crate::widgets::checkbox(ui, &mut o, tr("Only games found here"))
                        .on_hover_text(tr("Show only the games whose saves were found on this computer. Typing a name searches every game."))
                        .changed()
                    {
                        gp.only_found = Some(o);
                    }
                    // The box doesn't wrap by itself: start a new row when its (translated) text needs one.
                    let level_text = gp.level.map_or(tr("Any support"), |l| l.label());
                    let font = egui::TextStyle::Button.resolve(ui.style());
                    let need = ui.fonts(|f| f.layout_no_wrap(level_text.to_string(), font, Color32::WHITE).size().x) + 40.0;
                    if ui.available_size_before_wrap().x < need {
                        ui.end_row();
                    }
                    egui::ComboBox::from_id_salt("gp_level")
                        .wrap_mode(egui::TextWrapMode::Truncate)
                        .width(need.min(ui.available_size_before_wrap().x))
                        .selected_text(level_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut gp.level, None, tr("Any support"));
                            for l in crate::games::Level::ALL {
                                ui.selectable_value(&mut gp.level, Some(l), l.label()).on_hover_text(l.hint());
                            }
                        });
                    // The genres the website groups games by.
                    let genre_text = gp.genre.map_or(tr("Any genre"), picker_extra::genre_label);
                    let font = egui::TextStyle::Button.resolve(ui.style());
                    let need = ui.fonts(|f| f.layout_no_wrap(genre_text.to_string(), font, Color32::WHITE).size().x) + 40.0;
                    if ui.available_size_before_wrap().x < need {
                        ui.end_row();
                    }
                    egui::ComboBox::from_id_salt("gp_genre")
                        .wrap_mode(egui::TextWrapMode::Truncate)
                        .width(need.min(ui.available_size_before_wrap().x))
                        .selected_text(genre_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut gp.genre, None, tr("Any genre"));
                            for g in picker_extra::GENRES {
                                ui.selectable_value(&mut gp.genre, Some(*g), picker_extra::genre_label(g));
                            }
                        })
                        .response
                        .on_hover_text(tr("Only games of this kind. Games without a genre in the list are shown under Any genre."));
                    if ui
                        .add_enabled(!searching, egui::Button::new(tr("Search again")))
                        .on_hover_text(tr("The saves found before are remembered and checked first. Search again looks through all the usual places on this computer for games added since."))
                        .clicked()
                    {
                        spawn_search(&gp.found, true);
                        gp.save_groups = None;
                    }
                    // Inline (not pushed to the far edge) so it wraps with the rest instead of leaving the window.
                    if ui.button(tr("Close")).clicked() {
                        close = true;
                    }
                });
                if let Some(m) = gp.missing.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme::WARN, trf("That save was not found at {0}. It was moved or deleted, or its drive is not connected.", &[&m.display()]));
                        if ui.add_enabled(!searching, egui::Button::new(tr("Search the drives again"))).clicked() {
                            spawn_search(&gp.found, true);
                            gp.save_groups = None;
                            gp.missing = None;
                        }
                        if ui.button(tr("Dismiss")).clicked() {
                            gp.missing = None;
                        }
                    });
                }
                ui.separator();
                let q = gp.query.trim().to_lowercase();
                let (level, genre) = (gp.level, gp.genre);
                // Browsing the whole list: the letters start closed. A typed name or a genre
                // narrows it, and the letters holding what is left open by themselves.
                let browsing = q.is_empty() && genre.is_none();
                let matches = |i: usize| {
                    let g = &crate::games::GAMES[i];
                    crate::games::name_matches(g.name, &q) && level.is_none_or(|l| crate::games::level(g) == l) && genre.is_none_or(|k| picker_extra::genre_of(g.name) == Some(k))
                };
                // Games with saves on this computer first, then the rest; a typed name looks through all of them.
                // Starred games come first, wherever they are.
                let favs = gp.favs.clone();
                let is_fav = |i: usize| favs.iter().any(|f| f == crate::games::GAMES[i].name);
                let starred: Vec<usize> = (0..crate::games::GAMES.len()).filter(|&i| is_fav(i) && matches(i)).collect();
                let found_here: Vec<usize> = (0..crate::games::GAMES.len()).filter(|&i| here(i) && matches(i) && !is_fav(i)).collect();
                let others: Vec<usize> = if only && q.is_empty() { vec![] } else { (0..crate::games::GAMES.len()).filter(|&i| !here(i) && matches(i) && !is_fav(i)).collect() };
                let hidden = if only && q.is_empty() { (0..crate::games::GAMES.len()).filter(|&i| !here(i) && matches(i) && !is_fav(i)).count() } else { 0 };
                // Well-known games, above the A to Z groups while no name is typed (a typed name
                // opens the letters that hold it instead).
                let popular: Vec<usize> = if q.is_empty() && !(only && any_here) { picker_extra::popular_indices().into_iter().filter(|&i| matches(i) && !is_fav(i) && !here(i)).collect() } else { vec![] };
                let star_clicked: std::cell::Cell<Option<usize>> = std::cell::Cell::new(None);
                let mut show_all = false;
                let list = |ui: &mut egui::Ui, picked: &mut Option<usize>, show_all: &mut bool| {
                    ScrollArea::vertical().id_salt("gp_list").auto_shrink([false, false]).show_locked(ui, |ui| {
                        let row = |ui: &mut egui::Ui, i: usize, picked: &mut Option<usize>| {
                            let g = &crate::games::GAMES[i];
                            let n = match &found[i] {
                                Some(f) if f.is_empty() => tr("none found").to_string(),
                                Some(f) => trf("{0} found", &[&f.len()]),
                                None => tr("looking\u{2026}").to_string(),
                            };
                            ui.horizontal(|ui| {
                                let l = crate::games::level(g);
                                let color = match l {
                                    crate::games::Level::Screen => theme::OK,
                                    crate::games::Level::Values => theme::ACCENT,
                                    crate::games::Level::Hex => theme::WARN,
                                };
                                // The name gives way (with the whole text on hover) so the badge always stays beside it.
                                let chip_w = ui.fonts(|f| f.layout_no_wrap(l.label().to_string(), crate::theme::font(crate::theme::size::CAPTION), color).size().x) + 16.0;
                                // The star: starred games are listed first, and on the start page.
                                let fav = is_fav(i);
                                let star = ui
                                    .add(egui::Button::new(RichText::new("\u{2605}").color(if fav { theme::fg(ui, theme::MODIFIED) } else { ui.visuals().weak_text_color() })).frame(false).min_size(egui::vec2(22.0, ui.spacing().interact_size.y)))
                                    .on_hover_text(if fav { tr("Remove from favourites") } else { tr("Add to favourites") });
                                if star.clicked() {
                                    star_clicked.set(Some(i));
                                }
                                let name_w = (ui.available_width() - chip_w - ui.spacing().item_spacing.x).max(60.0);
                                let text = format!("{}  \u{00B7}  {n}", g.name);
                                let r = ui
                                    .allocate_ui_with_layout(egui::vec2(name_w, ui.spacing().interact_size.y), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                                        ui.selectable_label(*picked == Some(i), text.clone())
                                    })
                                    .inner;
                                if r.clicked() {
                                    *picked = Some(i);
                                }
                                r.on_hover_text(text);
                                App::chip(ui, l.label(), color).on_hover_text(l.hint());
                            });
                        };
                        if !starred.is_empty() {
                            ui.label(RichText::new(tr("Favourites")).strong());
                            for &i in &starred {
                                row(ui, i, picked);
                            }
                            ui.add_space(6.0);
                        }
                        if !found_here.is_empty() {
                            ui.label(RichText::new(tr("On this computer")).strong());
                            for &i in &found_here {
                                row(ui, i, picked);
                            }
                            ui.add_space(6.0);
                        } else if searching && only && q.is_empty() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.weak(tr("Looking for saves on this computer\u{2026}"));
                            });
                        }
                        if !popular.is_empty() {
                            ui.label(RichText::new(tr("Popular")).strong());
                            for &i in &popular {
                                row(ui, i, picked);
                            }
                            ui.add_space(6.0);
                        }
                        if !others.is_empty() && (!found_here.is_empty() || !popular.is_empty()) {
                            ui.label(RichText::new(tr("All games")).strong());
                        }
                        // Grouped by first letter (digits and symbols under #), collapsed unless a name is typed.
                        let letter = |i: usize| -> char {
                            let c = crate::games::GAMES[i].name.chars().next().unwrap_or('#').to_uppercase().next().unwrap_or('#');
                            if c.is_alphabetic() { c } else { '#' }
                        };
                        // PC games go under their first letter; console games under their console below.
                        let mut groups: Vec<(char, Vec<usize>)> = vec![];
                        let mut by_console: Vec<(&'static str, Vec<usize>)> = vec![];
                        for &i in &others {
                            let consoles = crate::games::consoles_of(crate::games::GAMES[i].name);
                            if consoles.is_empty() {
                                let c = letter(i);
                                match groups.iter_mut().find(|(k, _)| *k == c) {
                                    Some((_, v)) => v.push(i),
                                    None => groups.push((c, vec![i])),
                                }
                            }
                            for c in consoles {
                                match by_console.iter_mut().find(|(k, _)| *k == c) {
                                    Some((_, v)) => v.push(i),
                                    None => by_console.push((c, vec![i])),
                                }
                            }
                        }
                        groups.sort_by_key(|(c, _)| (*c != '#', *c));
                        by_console.sort_by_key(|(c, _)| crate::games::CONSOLES.iter().position(|(n, _)| n == c));
                        for (c, members) in &groups {
                            let title = format!("{c}  ({})", members.len());
                            let mut h = egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(("gp_letter", *c, found_here.is_empty(), browsing));
                            h = if browsing { h.default_open(false) } else { h.open(Some(true)) };
                            h.show(ui, |ui| {
                                for &i in members {
                                    row(ui, i, picked);
                                }
                            });
                        }
                        if !by_console.is_empty() {
                            ui.add_space(6.0);
                            ui.label(RichText::new(tr("Console games")).strong());
                            for (c, members) in &by_console {
                                let title = format!("{c}  ({})", members.len());
                                let mut h = egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(("gp_console", *c, found_here.is_empty(), browsing));
                                h = if browsing { h.default_open(false) } else { h.open(Some(true)) };
                                h.show(ui, |ui| {
                                    for &i in members {
                                        row(ui, i, picked);
                                    }
                                });
                            }
                        }
                        if hidden > 0 {
                            ui.add_space(6.0);
                            if ui.link(trf("Show the other {0} games", &[&hidden])).clicked() {
                                *show_all = true;
                            }
                        }
                        if starred.is_empty() && found_here.is_empty() && others.is_empty() && hidden == 0 {
                            ui.weak(tr("No game matches. Use Open\u{2026} to pick the file yourself."));
                        }
                    });
                };
                let details_body = |ui: &mut egui::Ui, picked: Option<usize>, group: &mut Option<String>, open: &mut Option<PathBuf>, save_groups: &mut Option<((usize, usize), Vec<String>)>| {
                    let Some(i) = picked else {
                        // Until a game is picked: how this window works.
                        ui.strong(tr("How this works"));
                        ui.add_space(2.0);
                        ui.label(tr("Pick a game to see where it keeps its saves and the ones on this computer. Open\u{2026} and dragging a file onto the window still work for anything else."));
                        ui.add_space(4.0);
                        ui.label(tr("Click a save to open it. Nothing is written until you save, and the file on disk is backed up first."));
                        ui.add_space(4.0);
                        ui.label(tr("The badge beside each game says how far editing goes: Screen (a screen of its own), Values (every value by name) or Hex (find a number you know, then change it)."));
                        ui.add_space(4.0);
                        ui.weak(tr("Type part of a name, or short names such as bg3 or ff7; the letters holding a match open by themselves."));
                        return;
                    };
                    let g = &crate::games::GAMES[i];
                    ui.heading(g.name);
                    for (k, v) in [(tr("Usually in"), crate::games::folder_text(g)), (tr("Files"), crate::games::files_text(g)), (tr("Editing"), crate::games::support_text(g))] {
                        widgets::label_text(ui, &format!("{k}:"), &v, false);
                    }
                    ui.add_space(6.0);
                    match &found[i] {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(tr("Looking for saves\u{2026}"));
                            });
                        }
                        Some(f) if f.is_empty() => {
                            ui.colored_label(theme::fg(ui, theme::WARN), tr("No saves found on this computer."));
                            ui.weak(tr("If the game keeps them somewhere else (another drive, a cloud folder), use Open\u{2026} to pick the file."));
                        }
                        Some(f) => {
                            let cards = |ui: &mut egui::Ui, saves: &[&crate::games::Found], open: &mut Option<PathBuf>| {
                                {
                                    // Many saves: the first 60, with a link for the rest (the cards vary in height).
                                    let all_id = ui.id().with("gp_cards_all");
                                    let all = ui.data(|d| d.get_temp::<bool>(all_id)).unwrap_or(false);
                                    let shown = if all { saves.len() } else { saves.len().min(CARDS_SHOWN) };
                                    for s in &saves[..shown] {
                                        theme::card(ui, |ui| {
                                            if ui.link(RichText::new(s.label.clone().unwrap_or_else(|| file_name(&s.path))).strong()).clicked() {
                                                *open = Some(s.path.clone());
                                            }
                                            // Who is in the save (characters, slots), read in the background.
                                            match crate::preview::cached_details(&s.path, s.modified) {
                                                Some(Some(who)) => {
                                                    ui.add(egui::Label::new(who.clone()).truncate()).on_hover_text(who);
                                                }
                                                Some(None) => {}
                                                None => ui.ctx().request_repaint_after(std::time::Duration::from_millis(150)),
                                            }
                                            let when = s
                                                .modified
                                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                                .map(|d| crate::backups::describe(d.as_secs()))
                                                .unwrap_or_default();
                                            let mut line = trf("{0}  \u{00B7}  {1}", &[&when, &human_size(s.size as usize)]);
                                            if let Some(n) = large_save_note(s.size) {
                                                line.push_str(&format!("  \u{00B7}  {n}"));
                                            }
                                            ui.add(egui::Label::new(RichText::new(line).weak()).truncate());
                                            ui.add(egui::Label::new(RichText::new(s.path.parent().map(|p| p.display().to_string()).unwrap_or_default()).weak().size(crate::theme::size::CAPTION)).truncate());
                                        });
                                    }
                                    if shown < saves.len() && ui.link(trf("Show all {0}", &[&saves.len()])).clicked() {
                                        ui.data_mut(|d| d.insert_temp(all_id, true));
                                    }
                                }
                            };
                            if !crate::games::many_games(g) {
                                ui.label(if f.len() == 1 { trf("{0} save found \u{2014} click one to open it:", &[&f.len()]) } else { trf("{0} saves found \u{2014} click one to open it:", &[&f.len()]) });
                                let all: Vec<&crate::games::Found> = f.iter().collect();
                                cards(ui, &all, open);
                                return;
                            }
                            // One entry per game, most recently played first.
                            let key = (i, f.len());
                            if save_groups.as_ref().map(|c| c.0) != Some(key) {
                                *save_groups = Some((key, f.iter().map(|s| crate::games::group_of(g, &s.path)).collect()));
                            }
                            let names = &save_groups.as_ref().unwrap().1;
                            let mut groups: Vec<(String, Vec<&crate::games::Found>)> = vec![];
                            for (s, name) in f.iter().zip(names) {
                                let name = name.clone();
                                match groups.iter_mut().find(|(n, _)| *n == name) {
                                    Some((_, v)) => v.push(s),
                                    None => groups.push((name, vec![s])),
                                }
                            }
                            if let Some(name) = group.clone() {
                                if let Some((_, saves)) = groups.iter().find(|(n, _)| *n == name) {
                                    if ui.button(trf("\u{2039} All {0} games", &[&groups.len()])).clicked() {
                                        *group = None;
                                    }
                                    ui.strong(&name);
                                    ui.label(if saves.len() == 1 { trf("{0} save \u{2014} click one to open it:", &[&saves.len()]) } else { trf("{0} saves \u{2014} click one to open it:", &[&saves.len()]) });
                                    cards(ui, saves, open);
                                    return;
                                }
                                *group = None;
                            }
                            ui.label(if groups.len() == 1 { trf("{0} game with saves ({1} saves) \u{2014} pick one:", &[&groups.len(), &f.len()]) } else { trf("{0} games with saves ({1} saves) \u{2014} pick one:", &[&groups.len(), &f.len()]) });
                            {
                                for (name, saves) in &groups {
                                    let when = saves[0]
                                        .modified
                                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                        .map(|d| crate::backups::describe(d.as_secs()))
                                        .unwrap_or_default();
                                    theme::card(ui, |ui| {
                                        if ui.link(RichText::new(name).strong()).clicked() {
                                            *group = Some(name.clone());
                                        }
                                        ui.weak(if saves.len() == 1 { trf("{0} save  \u{00B7}  last saved {1}", &[&saves.len(), &when]) } else { trf("{0} saves  \u{00B7}  last saved {1}", &[&saves.len(), &when]) });
                                    });
                                }
                            }
                        }
                    }
                };
                // The whole right-hand side scrolls as one, so a long description never pushes the window past the screen.
                let details = |ui: &mut egui::Ui, picked: Option<usize>, group: &mut Option<String>, open: &mut Option<PathBuf>, save_groups: &mut Option<((usize, usize), Vec<String>)>| {
                    ScrollArea::vertical().id_salt(("gp_details", picked, group.clone())).auto_shrink([false, false]).show_locked(ui, |ui| details_body(ui, picked, group, open, save_groups));
                };
                let before = gp.picked;
                if wide {
                    ui.columns(2, |c| {
                        list(&mut c[0], &mut gp.picked, &mut show_all);
                        // Not justified (columns are): wrapped lines were spread across the width.
                        c[1].with_layout(egui::Layout::top_down(egui::Align::Min), |ui| details(ui, gp.picked, &mut gp.group, &mut open, &mut gp.save_groups));
                    });
                } else if gp.picked.is_none() {
                    // No room for the "how this works" pane beside the list: its first line goes above it.
                    ui.weak(tr("Pick a game to see where it keeps its saves and the ones on this computer. Open\u{2026} and dragging a file onto the window still work for anything else."));
                    list(ui, &mut gp.picked, &mut show_all);
                } else {
                    if ui.button(tr("\u{2039} All games")).clicked() {
                        gp.picked = None;
                    }
                    details(ui, gp.picked, &mut gp.group, &mut open, &mut gp.save_groups);
                }
                if gp.picked != before {
                    gp.group = None;
                }
                if let Some(i) = star_clicked.get() {
                    let name = crate::games::GAMES[i].name.to_string();
                    match gp.favs.iter().position(|f| *f == name) {
                        Some(k) => {
                            gp.favs.remove(k);
                        }
                        None => gp.favs.push(name),
                    }
                }
                if show_all {
                    gp.only_found = Some(false);
                }
            });

    (open, close)
}

impl App {
    /// Find my game's saves: the results of the search made at start (or by "Search again") are
    /// reused, so opening the list never reads the drives again.
    pub(super) fn picker(&mut self) -> GamePicker {
        let found = match &self.saves_found {
            Some(f) => f.clone(),
            None => {
                let f = search_saves();
                self.saves_found = Some(f.clone());
                f
            }
        };
        GamePicker::with(found)
    }
}
