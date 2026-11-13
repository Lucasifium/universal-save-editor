//! "Your games" on the start page: the games whose saves were found on this computer, each
//! with its newest save one click away.

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
    /// The "Your games" card. Returns a save to open.
    pub(super) fn your_games(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let search = self.saves_found.clone()?;
        let found = search.lock().map(|v| v.clone()).unwrap_or_default();
        let done = found.iter().all(Option::is_some);
        if !done {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
        }
        let rows = rows(&found);
        let mut open = None;
        let mut show_game: Option<Option<usize>> = None;
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Your games").family(theme::semibold_family()).size(16.0));
                if !done {
                    ui.spinner();
                    ui.weak("looking for saves on this computer\u{2026}");
                }
            });
            if rows.is_empty() {
                ui.weak(if done {
                    "No saves of the games in the list were found in their usual places. Use Open a file\u{2026} to pick a save yourself."
                } else {
                    "Games whose saves are found will appear here."
                });
                return;
            }
            ui.weak("Click a game to open its newest save.");
            for r in rows.iter().take(SHOWN) {
                for s in [&r.name, &r.newest.path.to_string_lossy().to_string()] {
                    theme::cjk_for(ui.ctx(), s);
                }
                ui.horizontal(|ui| {
                    let link = ui.link(RichText::new(&r.name).strong()).on_hover_text(r.newest.path.display().to_string());
                    if link.clicked() {
                        open = Some(r.newest.path.clone());
                    }
                    let label = r.newest.label.clone().unwrap_or_else(|| file_name(&r.newest.path));
                    let mut line = format!("{label} \u{00B7} {} \u{00B7} {}", when(&r.newest), human_size(r.newest.size as usize));
                    if r.count > 1 {
                        line.push_str(&format!(" \u{00B7} {} saves", r.count));
                    }
                    ui.add(egui::Label::new(RichText::new(line).weak().size(12.5)).truncate());
                    if r.count > 1 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Other saves\u{2026}").clicked() {
                                show_game = Some(Some(r.game));
                            }
                        });
                    }
                });
            }
            if rows.len() > SHOWN || done {
                ui.add_space(2.0);
                let more = if rows.len() > SHOWN { format!("See all {} in Find my game's saves", rows.len()) } else { "See every game in Find my game's saves".into() };
                if ui.link(more).clicked() {
                    show_game = Some(None);
                }
            }
        });
        if let Some(pick) = show_game {
            // Reuse what was just found rather than searching again.
            let mut gp = GamePicker::with(search);
            gp.picked = pick;
            if pick.is_none() {
                gp.only_found = Some(true);
            }
            self.game_picker = Some(gp);
        }
        open
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
