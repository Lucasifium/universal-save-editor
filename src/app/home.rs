//! "Your games" on the start page: the games whose saves were found on this computer, each
//! with its newest save one click away.

use crate::i18n::{tr, trf};
use super::{file_name, human_size, App, GamePicker};
use crate::games::{self, Found, GAMES};
use crate::theme;
use eframe::egui::{self, RichText};
use std::path::PathBuf;

/// How many games the start page lists before "See all".
const SHOWN: usize = 8;

/// One row: a game (or, for entries covering many games, one of them) and its saves.
struct Row {
    game: usize,
    name: String,
    newest: Found,
    count: usize,
}

fn rows(found: &[Option<Vec<Found>>]) -> Vec<Row> {
    let mut out = vec![];
    for (i, f) in found.iter().enumerate() {
        let Some(f) = f.as_ref().filter(|f| !f.is_empty()) else { continue };
        let g = &GAMES[i];
        let mut groups: Vec<(String, Vec<&Found>)> = vec![];
        if games::many_games(g) {
            for s in f {
                let name = games::group_of(g, &s.path);
                match groups.iter_mut().find(|(n, _)| *n == name) {
                    Some((_, v)) => v.push(s),
                    None => groups.push((name, vec![s])),
                }
            }
        } else {
            groups.push((g.name.to_string(), f.iter().collect()));
        }
        for (name, saves) in groups {
            let newest = saves.iter().max_by_key(|s| s.modified).map(|s| (*s).clone());
            if let Some(newest) = newest {
                out.push(Row { game: i, name, newest, count: saves.len() });
            }
        }
    }
    out.sort_by(|a, b| b.newest.modified.cmp(&a.newest.modified));
    out
}

fn when(f: &Found) -> String {
    f.modified
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| crate::backups::describe(d.as_secs()))
        .unwrap_or_default()
}

impl App {
    /// "Favourites and recent games": the starred games, then the ones opened last, each one
    /// click from its newest save. Returns a save to open.
    pub(super) fn favourites_card(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let names: Vec<(String, bool)> = self
            .settings
            .favourites
            .iter()
            .map(|n| (n.clone(), true))
            .chain(self.settings.recent_games.iter().filter(|n| !self.settings.is_favourite(n)).map(|n| (n.clone(), false)))
            .filter(|(n, _)| GAMES.iter().any(|g| g.name == n))
            .collect();
        if names.is_empty() {
            return None;
        }
        // The newest save of each game, from the search made at start (when it has finished).
        let found = self.saves_found.as_ref().and_then(|s| s.lock().ok().map(|v| v.clone())).unwrap_or_default();
        let mut open = None;
        let mut pick: Option<usize> = None;
        let mut toggle: Option<String> = None;
        theme::card(ui, |ui| {
            ui.label(RichText::new(tr("Favourites and recent games")).family(theme::semibold_family()).size(16.0));
            for (name, fav) in &names {
                let Some(i) = GAMES.iter().position(|g| g.name == name) else { continue };
                let newest = found.get(i).and_then(|f| f.as_ref()).and_then(|f| f.iter().max_by_key(|s| s.modified)).cloned();
                ui.horizontal(|ui| {
                    ui.set_min_height(ui.spacing().interact_size.y);
                    let color = if *fav { theme::fg(ui, theme::MODIFIED) } else { ui.visuals().weak_text_color() };
                    let star = ui.add(egui::Button::new(RichText::new("\u{2605}").color(color)).frame(false)).on_hover_text(if *fav { tr("Remove from favourites") } else { tr("Add to favourites") });
                    if star.clicked() {
                        toggle = Some(name.clone());
                    }
                    let link = ui.link(RichText::new(name).strong());
                    if link.clicked() {
                        match &newest {
                            Some(n) if !games::many_games(&GAMES[i]) => open = Some(n.path.clone()),
                            _ => pick = Some(i),
                        }
                    }
                    match &newest {
                        Some(n) => {
                            link.on_hover_text(n.path.display().to_string());
                            if ui.available_width() > 48.0 {
                                ui.add(egui::Label::new(RichText::new(format!("{} \u{00B7} {}", n.label.clone().unwrap_or_else(|| file_name(&n.path)), when(n))).weak().size(12.5)).truncate());
                            }
                        }
                        None => {
                            link.on_hover_text(tr("Show this game's saves"));
                        }
                    }
                });
            }
        });
        if let Some(n) = toggle {
            self.settings.toggle_favourite(&n);
        }
        if let Some(i) = pick {
            let mut gp = self.picker();
            gp.picked = Some(i);
            self.game_picker = Some(gp);
        }
        if open.is_some() {
            if let Some((name, _)) = names.iter().find(|(n, _)| GAMES.iter().position(|g| g.name == n).is_some_and(|i| found.get(i).and_then(|f| f.as_ref()).is_some_and(|f| f.iter().any(|s| Some(&s.path) == open.as_ref())))) {
                self.settings.push_recent_game(name);
            }
        }
        open
    }

    /// The "Your games" card. Returns a save to open, and whether "Open a file\u{2026}" was clicked.
    pub(super) fn your_games(&mut self, ui: &mut egui::Ui) -> (Option<PathBuf>, bool) {
        let Some(search) = self.saves_found.clone() else { return (None, false) };
        let mut browse = false;
        let found = search.lock().map(|v| v.clone()).unwrap_or_default();
        let done = found.iter().all(Option::is_some);
        if !done {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
        }
        let rows = rows(&found);
        let mut open = None;
        let mut opened_game = None;
        let mut show_game: Option<Option<usize>> = None;
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Your games")).family(theme::semibold_family()).size(16.0));
                if !done {
                    ui.spinner();
                    ui.weak(tr("looking for saves on this computer\u{2026}"));
                }
            });
            if rows.is_empty() {
                ui.weak(if done {
                    tr("No saves of the games in the list were found in their usual places. Use Open a file\u{2026} to pick a save yourself.")
                } else {
                    tr("Games whose saves are found will appear here.")
                });
                if done {
                    // The two ways on from here, as buttons rather than only words.
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr("Search again")).on_hover_text(tr("Look through all the usual places on this computer again, for games added since.")).clicked() {
                            super::spawn_search(&search, true);
                        }
                        if ui.button(tr("Open a file\u{2026}")).clicked() {
                            browse = true;
                        }
                    });
                }
                return;
            }
            ui.weak(tr("Click a game to open its newest save."));
            for r in rows.iter().take(SHOWN) {
                for s in [&r.name, &r.newest.path.to_string_lossy().to_string()] {
                    theme::cjk_for(ui.ctx(), s);
                }
                // Every row is the same height, and the button is placed first
                // (at the right edge) so the text takes only the room left over
                // and truncates instead of running under it.
                ui.horizontal(|ui| {
                    ui.set_min_height(ui.spacing().interact_size.y);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if r.count > 1 && ui.small_button(tr("Other saves\u{2026}")).clicked() {
                            show_game = Some(Some(r.game));
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let link = ui.link(RichText::new(&r.name).strong()).on_hover_text(r.newest.path.display().to_string());
                            if link.clicked() {
                                open = Some(r.newest.path.clone());
                                opened_game = Some(r.game);
                            }
                            let label = r.newest.label.clone().unwrap_or_else(|| file_name(&r.newest.path));
                            let mut line = format!("{label} \u{00B7} {} \u{00B7} {}", when(&r.newest), human_size(r.newest.size as usize));
                            if let Some(n) = crate::app::large_save_note(r.newest.size) {
                                line.push_str(&format!(" \u{00B7} {n}"));
                            }
                            if r.count > 1 {
                                line.push_str(&format!(" \u{00B7} {}", trf("{0} saves", &[&r.count])));
                            }
                            // Too little room left (a phone, a long name): leave the details out rather than run under the button.
                            if ui.available_width() > 48.0 {
                                ui.add(egui::Label::new(RichText::new(line).weak().size(12.5)).truncate());
                            }
                        });
                    });
                });
            }
            if rows.len() > SHOWN || done {
                ui.add_space(2.0);
                let more = if rows.len() > SHOWN { trf("See all {0} in Find my game's saves", &[&rows.len()]) } else { tr("See every game in Find my game's saves").into() };
                if ui.link(more).clicked() {
                    show_game = Some(None);
                }
            }
        });
        if let Some(g) = opened_game {
            self.settings.push_recent_game(GAMES[g].name);
        }
        if let Some(pick) = show_game {
            // Reuse what was just found rather than searching again.
            let mut gp = GamePicker::with(search);
            gp.picked = pick;
            if pick.is_none() {
                gp.only_found = Some(true);
            }
            self.game_picker = Some(gp);
        }
        (open, browse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn found(path: &str, secs: u64) -> Found {
        Found { path: PathBuf::from(path), modified: Some(UNIX_EPOCH + Duration::from_secs(secs)), size: 10, label: None }
    }

    #[test]
    fn newest_game_first_with_its_newest_save() {
        let mut v: Vec<Option<Vec<Found>>> = vec![Some(vec![]); GAMES.len()];
        let (a, b) = (GAMES.iter().position(|g| !games::many_games(g)).unwrap(), GAMES.iter().rposition(|g| !games::many_games(g)).unwrap());
        v[a] = Some(vec![found("/a/old.sav", 100), found("/a/new.sav", 300)]);
        v[b] = Some(vec![found("/b/one.sav", 200)]);
        let r = rows(&v);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].game, a);
        assert_eq!(r[0].newest.path, PathBuf::from("/a/new.sav"));
        assert_eq!(r[0].count, 2);
        assert_eq!(r[1].game, b);
    }
}
