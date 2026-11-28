//! First-run pointers: once the update question is answered, the start page points out three
//! things a new player would not find before their first save (finding saves, the backup taken
//! before every save, the Save manager), with a mark on each, until "Got it". No tour window.

use super::{App, WEB};
use crate::i18n::tr;
use crate::theme;
use eframe::egui;

impl App {
    /// Whether the start page shows the pointers now.
    pub(super) fn pointers_shown(&self) -> bool {
        !WEB && !cfg!(target_os = "android") && !self.settings.pointers_seen && !self.update_question && self.doc.is_none() && self.saves_found.is_some()
    }

    /// The card that says what the marks point to.
    pub(super) fn pointers_card(&mut self, ui: &mut egui::Ui) {
        if !self.pointers_shown() {
            return;
        }
        let mut done = false;
        theme::card(ui, |ui| {
            ui.label(theme::panel_title(tr("Good to know")));
            ui.add_space(2.0);
            for (n, text) in [
                (1, tr("Find my game's saves looks in the places each game keeps them, so you don't have to.")),
                (2, tr("Every save is backed up first: the file as it was is kept each time you save over it.")),
                (3, tr("Backups \u{203A} Save manager\u{2026} lists the backups of the open save, to restore or compare.")),
            ] {
                ui.horizontal_top(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                    badge(ui, r.center(), n);
                    ui.add(egui::Label::new(text).wrap());
                });
            }
            ui.add_space(2.0);
            if crate::widgets::small_button(ui, tr("Got it")).on_hover_text(tr("Hide these pointers for good")).clicked() {
                done = true;
            }
        });
        ui.add_space(8.0);
        if done {
            self.settings.pointers_seen = true;
            self.settings.save();
        }
    }

    /// Marks `r` (a button the card points to) with a ring and its number while the card shows.
    pub(super) fn point_at(&self, ui: &egui::Ui, r: egui::Rect, n: u32) {
        if !self.pointers_shown() {
            return;
        }
        let c = theme::focus_color(ui.visuals().dark_mode);
        let p = ui.painter().with_clip_rect(ui.ctx().screen_rect());
        p.rect_stroke(r.expand(3.0), egui::Rounding::same(7.0), egui::Stroke::new(2.0_f32, c));
        badge_with(&p, ui, r.right_top() + egui::vec2(0.0, 0.0), n);
    }
}

fn badge(ui: &egui::Ui, at: egui::Pos2, n: u32) {
    badge_with(ui.painter(), ui, at, n);
}

/// A filled circle with a number, the same on the card and on the marked button.
fn badge_with(p: &egui::Painter, ui: &egui::Ui, at: egui::Pos2, n: u32) {
    p.circle_filled(at, 9.0, theme::focus_color(ui.visuals().dark_mode));
    let text = if ui.visuals().dark_mode { egui::Color32::from_rgb(16, 17, 21) } else { egui::Color32::WHITE };
    p.text(at, egui::Align2::CENTER_CENTER, n.to_string(), theme::font(theme::size::CAPTION), text);
}

#[cfg(test)]
mod tests {
    use super::super::*;

    fn app() -> (egui::Context, App) {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        theme::apply(&ctx, true);
        let mut app = App::with(&ctx, Settings::default(), false);
        app.saves_found = Some(std::sync::Arc::new(std::sync::Mutex::new(vec![None; crate::games::GAMES.len()])));
        (ctx, app)
    }

    #[test]
    fn pointers_show_after_the_update_question_until_got_it() {
        let (_ctx, mut app) = app();
        if WEB {
            return;
        }
        app.update_question = true;
        assert!(!app.pointers_shown(), "not while the update question is open");
        app.update_question = false;
        assert!(app.pointers_shown());
        app.settings.pointers_seen = true;
        assert!(!app.pointers_shown(), "gone for good once seen");
        let s = Settings { pointers_seen: true, ..Settings::default() };
        assert!(s.to_json()["pointers_seen"].as_bool().unwrap());
    }

    #[test]
    fn pointers_card_fits_in_every_language() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut problems = vec![];
        for lang in ["en", "de", "es", "fr", "pt-BR", "ja", "zh-CN"] {
            crate::i18n::set(Some(lang));
            for w in [1150.0, 390.0] {
                let (ctx, mut app) = app();
                let mut out = None;
                for _ in 0..3 {
                    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, 800.0))), ..Default::default() };
                    out = Some(ctx.run(input, |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| app.pointers_card(ui));
                    }));
                }
                for t in layout_tests::cut_off_text(&out.unwrap()) {
                    problems.push(format!("{lang} at {w}: {t}"));
                }
            }
        }
        crate::i18n::set(Some("en"));
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
