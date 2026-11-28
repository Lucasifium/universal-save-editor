//! Tools › Copy to the Steam version: a Game Pass save copied into the folder the same game's
//! Steam version uses (see `crate::xbox_steam`).

use super::{fit, modal, safe_choice, App};
use crate::i18n::{tr, trf};
use crate::theme;
use crate::xbox_steam::{self, Source};
use eframe::egui::{self, Color32, RichText, TextEdit};
use std::path::PathBuf;

pub struct XboxSteam {
    src: Source,
    folders: Vec<PathBuf>,
    dir: usize,
    name: String,
    replace: bool,
}

impl App {
    /// The Tools menu entry, shown for a Game Pass save of a game that also has a Steam version.
    pub(super) fn xbox_steam_button(&mut self, ui: &mut egui::Ui) {
        if cfg!(target_arch = "wasm32") {
            return;
        }
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        let Some(src) = xbox_steam::source(&path) else { return };
        if ui
            .button(tr("Copy to the Steam version\u{2026}"))
            .on_hover_text(tr("Put a copy of this Xbox app save where the game's Steam version looks for its saves. The Xbox app's save is left as it is."))
            .clicked()
        {
            ui.close_menu();
            self.start_xbox_steam(src);
        }
    }

    /// The Game Pass save open now, when it can be copied to the game's Steam version.
    pub(super) fn xbox_steam_source(&self) -> Option<Source> {
        if cfg!(target_arch = "wasm32") {
            return None;
        }
        xbox_steam::source(&self.doc.as_ref()?.path)
    }

    /// Opens the "copy to the Steam version" dialog for `src` (refused with unsaved changes).
    pub(super) fn start_xbox_steam(&mut self, src: Source) {
        if self.dirty {
            return self.fail(tr("Save or revert your changes first. Copying works on the file as it is on disk."));
        }
        let folders = crate::games::other_save_folders(src.game);
        let name = xbox_steam::suggested_name(&src);
        self.xbox_steam = Some(XboxSteam { src, folders, dir: 0, name, replace: false });
    }

    pub(super) fn xbox_steam_dialog(&mut self, ctx: &egui::Context) {
        let Some(x) = &mut self.xbox_steam else { return };
        let mut choice = None;
        modal(ctx, tr("Copy to the Steam version"), |ui| {
            ui.set_max_width(fit(ctx, 560.0));
            ui.label(trf("{0}: \u{201C}{1}\u{201D} from the Xbox app.", &[&x.src.game.name, &x.src.logical]));
            ui.add_space(4.0);
            if x.folders.is_empty() {
                ui.colored_label(theme::fg(ui, theme::WARN), tr("No folder of the Steam version was found on this computer. Start the game on Steam once and save, then try again."));
            } else {
                egui::Grid::new("xbox_steam").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label(tr("Folder"));
                    let shown = |p: &PathBuf| p.display().to_string();
                    egui::ComboBox::from_id_salt("xs_dir").width(380.0).wrap_mode(egui::TextWrapMode::Truncate).selected_text(shown(&x.folders[x.dir])).show_ui(ui, |ui| {
                        for (i, f) in x.folders.iter().enumerate() {
                            ui.selectable_value(&mut x.dir, i, shown(f)).on_hover_text(shown(f));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("File name"));
                    ui.add(TextEdit::singleline(&mut x.name).desired_width(380.0));
                    ui.end_row();
                });
            }
            let name_ok = xbox_steam::check_name(x.src.game, &x.name);
            let dest = x.folders.get(x.dir).map(|d| d.join(x.name.trim()));
            let exists = name_ok.is_ok() && dest.as_ref().is_some_and(|d| d.exists());
            match &name_ok {
                Err(e) if !x.folders.is_empty() => {
                    ui.colored_label(theme::fg(ui, theme::WARN), e);
                }
                _ if exists => {
                    crate::widgets::checkbox(ui, &mut x.replace, tr("Replace the save that is there (it is backed up first)"));
                }
                _ => {}
            }
            ui.weak(tr("The Xbox app's save is left as it is. If Steam asks which copy to keep, choose the one on this computer."));
            ui.weak(tr("If the Steam version says the save belongs to someone else, open the copy here and use Tools \u{203A} Change Steam account in this save."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = !x.folders.is_empty() && name_ok.is_ok() && (!exists || x.replace);
                if ui.add_enabled(ok, egui::Button::new(RichText::new(tr("Copy")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if safe_choice(ui.button(tr("Cancel"))).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                if let Some(x) = self.xbox_steam.take() {
                    let Some(dir) = x.folders.get(x.dir) else { return };
                    if let Some(game) = crate::running::game_for(&dir.join(x.name.trim())).or_else(|| crate::running::game_for(&x.src.blob)) {
                        return self.fail(trf("{0} looks like it's running. Quit it first. Nothing was copied.", &[&game]));
                    }
                    match xbox_steam::copy(&x.src, dir, &x.name, x.replace) {
                        Ok((dest, _)) => self.ok(trf("Copied to {0}. Start the Steam version to load it.", &[&dest.display()])),
                        Err(e) => self.fail(trf("Couldn't copy the save: {0}. The Steam version's save wasn't changed.", &[&e])),
                    }
                }
            }
            Some(false) => self.xbox_steam = None,
            None => {}
        }
    }
}
