//! The toolbar, the file note bar, the file chips and the status bar.

use super::*;

impl App {
    pub(super) fn chip(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
        // A frame doesn't wrap on its own: start a new line when the chip won't fit.
        let w = ui.fonts(|f| f.layout_no_wrap(text.to_string(), crate::theme::font(crate::theme::size::CAPTION), color).size().x) + 16.0;
        if ui.layout().main_wrap && w > ui.available_size_before_wrap().x {
            ui.end_row();
        }
        egui::Frame::none()
            .fill(color.gamma_multiply(0.18))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(8.0, 2.0))
            .show(ui, |ui| ui.add(egui::Label::new(RichText::new(text).color(theme::fg(ui, color)).size(crate::theme::size::CAPTION)).extend()))
            .response
    }

    pub(super) fn note_hidden(&self) -> bool {
        self.settings.hidden_notes.contains(&Settings::note_key(&self.doc_note))
    }

    /// The note about the opened file, on one line with More to read it all. Dismissing it
    /// hides it for every file with the same note, until View \u{203A} Show the note about this file.
    pub(super) fn note_bar(&mut self, ui: &mut egui::Ui) {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let width = |ui: &egui::Ui, t: &str| ui.fonts(|f| f.layout_no_wrap(t.to_string(), font.clone(), Color32::WHITE).size().x);
        let pad = 2.0 * ui.spacing().button_padding.x + ui.spacing().item_spacing.x;
        let dismiss_w = width(ui, tr("Dismiss")) + pad;
        let more_w = width(ui, tr("More")).max(width(ui, tr("Less"))) + pad;
        let note = crate::i18n::tr_any(&self.doc_note).to_string();
        // The strip stays one line: a note that doesn't fit on it is cut short, with More.
        let fits = width(ui, &note) <= ui.available_width() - dismiss_w - 8.0;
        let (mut dismiss, mut toggle) = (false, false);
        ui.horizontal_top(|ui| {
            let text_w = (ui.available_width() - dismiss_w - if fits { 0.0 } else { more_w } - 4.0).max(80.0);
            ui.allocate_ui(egui::vec2(text_w, 0.0), |ui| {
                ui.set_width(text_w);
                let text = RichText::new(&note).color(ui.visuals().weak_text_color());
                if fits || self.note_open {
                    ui.add(egui::Label::new(text).wrap());
                } else {
                    ui.add(egui::Label::new(text).truncate());
                }
            });
            if !fits && crate::widgets::small_button(ui, if self.note_open { tr("Less") } else { tr("More") }).clicked() {
                toggle = true;
            }
            if crate::widgets::small_button(ui, tr("Dismiss")).on_hover_text(tr("Hide this note for every file like this one. View \u{203A} Show the note about this file brings it back.")).clicked() {
                dismiss = true;
            }
        });
        if toggle {
            self.note_open = !self.note_open;
        }
        if dismiss {
            self.settings.hidden_notes.push(Settings::note_key(&self.doc_note));
            self.settings.save();
        }
    }

    /// "Save", or "Save (3)" with the number of unsaved changes.
    pub(super) fn save_label(&self) -> String {
        let n = self.change_count();
        if self.dirty && n > 0 {
            trf("Save ({0})", &[&n])
        } else {
            tr("Save").to_string()
        }
    }

    pub(super) fn save_button(&mut self, ui: &mut egui::Ui) {
        let hi = self.has_icons;
        let label = self.save_label();
        let save_btn = if self.dirty {
            egui::Button::new(widgets::icon_text_colored(ui, hi, icon::SAVE, &label, Color32::WHITE)).fill(theme::ACCENT)
        } else {
            egui::Button::new(widgets::icon_text(ui, hi, icon::SAVE, &label))
        };
        let b = ui.add_enabled(self.dirty, save_btn);
        if widgets::named(b, egui::WidgetType::Button, &label).on_hover_text(tr("Save changes to the file (Ctrl+S)")).clicked() {
            self.save_clicked();
        }
    }

    /// Opens the Save manager window for the open save.
    pub(super) fn open_save_manager(&mut self) {
        self.manager = self.doc.as_ref().map(|d| (d.path.clone(), String::new(), None));
    }

    pub(super) fn changes_toggle(&mut self, ui: &mut egui::Ui) {
        let hi = self.has_icons;
        // The selected colours: white on the light theme's pale selection could not be read.
        let color = if self.show_changes { ui.visuals().selection.stroke.color } else { ui.visuals().text_color() };
        let n = self.change_count();
        let label = if self.dirty && n > 0 { trf("Changes ({0})", &[&n]) } else { tr("Changes").to_string() };
        // A framed button like its neighbours, so it reads as something to press.
        let text = widgets::icon_text_colored(ui, hi, icon::HISTORY, &label, color);
        let r = ui.add(egui::Button::new(text).selected(self.show_changes));
        if r.clicked() {
            self.show_changes = !self.show_changes;
        }
        // Its text starts with an icon glyph: the screen reader gets the words and the on/off state.
        let (on, enabled) = (self.show_changes, r.enabled());
        r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, enabled, on, &label));
        r.on_hover_text(tr("List everything you've changed since the last save"));
    }

    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        if narrow(ui) {
            return self.phone_toolbar(ui);
        }
        let hi = self.has_icons;
        ui.add_space(4.0);
        // The toolbar wraps well before the rest of the layout turns phone-sized: the file
        // name and its chips need the room.
        // Also when the one-line toolbar didn't fit last time (long translations, a long file
        // name or game name).
        let need_id = egui::Id::new(("toolbar_need", self.doc.as_ref().map(|d| d.path.clone()), crate::i18n::current().code));
        let need = ui.ctx().data(|d| d.get_temp::<f32>(need_id)).unwrap_or(0.0);
        let wrap = ui.available_width() < 900.0 || ui.available_width() < need;
        let full_w = ui.available_width();
        let row = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
            if wrap {
                ui.horizontal_wrapped(|ui| add(ui));
            } else {
                ui.horizontal(|ui| add(ui));
            }
        };
        row(ui, &mut |ui| {
            // These buttons start with an icon glyph: `named` keeps it out of what a screen reader says.
            let b = ui.button(widgets::icon_text(ui, hi, icon::OPEN, tr("Open")));
            if widgets::named(b, egui::WidgetType::Button, tr("Open")).on_hover_text(tr("Open a save file (Ctrl+O)")).clicked() {
                self.ask(Ask::Open, None, None);
            }
            if !cfg!(target_os = "android")
                && !WEB
                && ui.button(tr("Find saves")).on_hover_text(tr("Pick a game from the list to see the saves it has on this computer")).clicked()
            {
                self.game_picker = Some(self.picker());
            }
            self.save_button(ui);
            // The list of what Save will write sits next to it.
            if self.doc.is_some() {
                self.changes_toggle(ui);
            }
            if !wrap {
                ui.separator();
            }
            let b = ui.add_enabled(!self.undo.is_empty(), egui::Button::new(widgets::icon_text(ui, hi, icon::UNDO, tr("Undo"))));
            if widgets::named(b, egui::WidgetType::Button, tr("Undo")).on_hover_text(tr("Ctrl+Z")).clicked() {
                self.undo_redo(true);
            }
            let b = ui.add_enabled(!self.redo.is_empty(), egui::Button::new(widgets::icon_text(ui, hi, icon::REDO, tr("Redo"))));
            if widgets::named(b, egui::WidgetType::Button, tr("Redo")).on_hover_text(tr("Ctrl+Y")).clicked() {
                self.undo_redo(false);
            }
            if self.doc.is_some() {
                if !wrap {
                    ui.separator();
                }
                let b = ui.button(tr("Close save"));
                if b.on_hover_text(tr("Close this save and go back to the start page (Ctrl+W)")).clicked() {
                    self.close_clicked();
                }
                if !WEB && ui.button(tr("Save manager")).on_hover_text(tr("All backups of this save in one window: restore, compare, delete, name. Restoring backs up the current save first.")).clicked() {
                    self.open_save_manager();
                }
            }
            let Some(doc) = &self.doc else { return };
            if !wrap {
                ui.separator();
            }
            // A long file name is shortened (the whole path is in its tooltip) rather than
            // pushing the toolbar onto a second line.
            let name_w = if wrap { ui.available_width() } else { (ui.available_width() - 220.0).clamp(160.0, 520.0) };
            ui.allocate_ui(egui::vec2(name_w, ui.available_height()), |ui| {
                ui.add(egui::Label::new(crate::theme::panel_title(file_name(&doc.path))).truncate())
                    .on_hover_text(doc.path.display().to_string());
            });
            self.file_chips(ui, true);
            if !wrap {
                let used = ui.min_rect().width() + ui.spacing().item_spacing.x;
                if used > full_w {
                    ui.ctx().data_mut(|d| d.insert_temp(need_id, used));
                    ui.ctx().request_repaint();
                }
            }
        });
        ui.add_space(2.0);
    }

    /// The phone's toolbar: one line with the menus, the file name, Undo and Save.
    pub(super) fn phone_toolbar(&mut self, ui: &mut egui::Ui) {
        let hi = self.has_icons;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            // Finger-sized buttons.
            ui.spacing_mut().button_padding = egui::vec2(12.0, 7.0);
            let m = ui.menu_button(RichText::new(PHONE_MENU).size(20.0), |ui| {
                ui.spacing_mut().button_padding = egui::vec2(10.0, 8.0);
                self.menus(ui);
            })
            .response;
            self.point_at(ui, m.rect, 3);
            widgets::named(m, egui::WidgetType::Button, tr("Menu")).on_hover_text(tr("Menu"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.save_button(ui);
                if self.doc.is_some() && widgets::symbol_button(ui, "\u{00D7}", tr("Close save"), tr("Close this save and go back to the start page (Ctrl+W)")).clicked() {
                    self.close_clicked();
                }
                let b = ui.add_enabled(!self.undo.is_empty(), egui::Button::new(widgets::icon(hi, icon::UNDO, "\u{21A9}").size(17.0)));
                if widgets::named(b, egui::WidgetType::Button, tr("Undo")).on_hover_text(tr("Undo")).clicked() {
                    self.undo_redo(true);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let name = self.doc.as_ref().map(|d| (file_name(&d.path), d.path.display().to_string()));
                    match name {
                        Some((n, full)) => {
                            ui.add(egui::Label::new(RichText::new(n).family(theme::semibold_family()).size(crate::theme::size::BODY)).truncate()).on_hover_text(full);
                        }
                        None => {
                            ui.add(egui::Label::new(RichText::new(platform::DISPLAY_NAME).family(theme::semibold_family()).size(crate::theme::size::BODY)).truncate());
                        }
                    }
                });
            });
        });
        // Warnings about the file (game running, signed save, names not found) on their own line.
        if self.doc.as_ref().is_some_and(|d| self.has_warning_chips(d)) {
            ui.horizontal_wrapped(|ui| self.file_chips(ui, false));
        }
        ui.add_space(2.0);
    }

    pub(super) fn has_warning_chips(&self, doc: &Doc) -> bool {
        self.running.is_some()
            || crate::cloud::service_for(&doc.path).is_some()
            || doc.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. } | Layer::RenpyPersistent { signed: true }))
            || matches!(&doc.content, Content::Json { value, .. } if names::is_rpgmaker(value))
    }

    /// The chips after the file name: its format (when `with_format`) and any warnings.
    pub(super) fn file_chips(&mut self, ui: &mut egui::Ui, with_format: bool) {
        let mut open_as_pick = None;
        self.file_chips_inner(ui, with_format, &mut open_as_pick);
        if let Some(w) = open_as_pick {
            self.open_as(w);
        }
    }

    pub(super) fn file_chips_inner(&mut self, ui: &mut egui::Ui, with_format: bool, open_as_pick: &mut Option<formats::ReadAs>) {
        let Some(doc) = &self.doc else { return };
        if with_format {
            let kind = friendly_kind(&doc.layers, &doc.content);
            let described = formats::describe(&doc.layers, &doc.content);
            // A game screen was found for a generic format (JSON, XML...): name the game on the chip.
            // Formats shared by several games name the one in this save (Half-Life: Source,
            // Black Mesa, Metro, the older LEGO games).
            let in_save = match &doc.content {
                Content::Tree(t) => ["/Save/Game", "/Game"].iter().find_map(|p| t.view.pointer(p).and_then(|v| v.as_str()).filter(|g| !g.is_empty()).map(str::to_string)),
                _ => None,
            };
            // No game screen: the catalog game whose save folder and file name this is.
            let by_path = || if doc.game.is_none() { crate::game_match::name_for(ui.ctx(), &doc.path).map(str::to_string) } else { None };
            let named = in_save.or_else(|| doc.game.and_then(|g| g.name()).map(str::to_string)).or_else(by_path).filter(|n| !kind.contains(n.as_str()) && n != "Source engine game");
            let chip_text = match &named {
                Some(n) => trf("{0} save", &[&n]),
                None => kind_label(&kind),
            };
            let hover = if named.is_some() { trf("Detected format: {0} ({1})", &[&kind, &described]) } else { trf("Detected format: {0}", &[&described]) };
            let (chip_text, color) = match open_as::chosen_label(doc.opts.read_as) {
                Some(l) => (l.to_string(), theme::WARN),
                None => (chip_text, theme::ACCENT),
            };
            let chip = Self::chip(ui, &chip_text, color).interact(egui::Sense::click());
            let chip = widgets::named(chip, egui::WidgetType::Button, &chip_text).on_hover_text(format!("{hover}\n{}", tr("Click to open it another way (Open as).")));
            *open_as_pick = self.open_as_popup(ui, &chip);
        }
        if doc.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. } | Layer::RenpyPersistent { signed: true })) {
            if crate::renpy_sign::can_sign() {
                Self::chip(ui, tr("Signed, will be re-signed"), theme::OK).on_hover_text(
                    tr("Ren'Py 8.1+ signs its saves. Your Ren'Py key was found on this PC, so edited saves are re-signed and load normally."),
                );
            } else {
                Self::chip(ui, tr("Signed save"), theme::WARN).on_hover_text(tr("Ren'Py 8.1+ signs its saves, but your Ren'Py key (%APPDATA%\\RenPy\\tokens) wasn't found. The old signature will be removed, and the game will ask once whether to load the save: choose Yes."));
            }
        }
        if let Some(game) = &self.running {
            Self::chip(ui, &trf("{0} is running", &[game]), theme::WARN).on_hover_text(
                tr("The game was running when this file was opened. Quit it before saving, or it may overwrite your changes."),
            );
        }
        if let Some(c) = crate::cloud::service_for(&doc.path) {
            let hover = trf("This save is in a folder that {0} syncs. Quit the game before saving, and if {0} asks which copy to keep afterwards, choose the local one.", &[&c]);
            Self::chip(ui, &trf("Cloud sync: {0}", &[&c]), theme::ACCENT).on_hover_text(hover);
        }
        let rpg = matches!(&doc.content, Content::Json { value, .. } if names::is_rpgmaker(value));
        let mut locate = false;
        if rpg {
            match &doc.names.source {
                Some(dir) => {
                    Self::chip(ui, tr("Names from game data"), theme::OK).on_hover_text(dir.display().to_string());
                }
                None => {
                    let text = tr("Game data not found. Click to locate it.");
                    let chip = Self::chip(ui, text, theme::WARN).interact(egui::Sense::click());
                    if widgets::named(chip, egui::WidgetType::Button, text)
                        .on_hover_text(tr("Point to the game's data folder to see variable, switch and item names"))
                        .clicked()
                    {
                        locate = true;
                    }
                }
            }
        }
        if locate {
            self.locate_data_dir();
        }
    }

    pub(super) fn change_count(&self) -> usize {
        let Some(d) = &self.doc else { return 0 };
        if self.changes.gen == Some(self.edit_gen) {
            self.changes.count()
        } else {
            Changes::of(d, &self.touched).count()
        }
    }

    /// Brings `changes` up to date with the last edit.
    pub(super) fn refresh_changes(&mut self) {
        if self.changes.gen != Some(self.edit_gen) {
            self.changes = match &self.doc {
                Some(d) => Changes::of(d, &self.touched),
                None => Changes::default(),
            };
            self.changes.gen = Some(self.edit_gen);
        }
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        // The change count goes on the right first, so the message is cut short before it
        // rather than running underneath it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // On a phone the highlighted Save button says the same thing.
            if self.doc.is_some() && !narrow(ui) {
                let n = self.change_count();
                if self.dirty {
                    ui.label(RichText::new(if n == 1 { tr("1 unsaved change").to_string() } else { crate::i18n::trf("{0} unsaved changes", &[&n]) }).color(theme::fg(ui, theme::MODIFIED)));
                } else {
                    ui.weak(tr("No unsaved changes"));
                }
                ui.add_space(12.0);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // Done and failed differ by a mark as well as by colour (✔ and ⚠ without the icon font).
                let (color, glyph, fallback) = match self.status_kind {
                    StatusKind::Info => (ui.visuals().weak_text_color(), "", ""),
                    StatusKind::Ok => (theme::fg(ui, theme::OK), icon::CHECK, "\u{2714}"),
                    StatusKind::Err => (theme::fg(ui, theme::ERR), icon::WARNING, "\u{26A0}"),
                };
                if !glyph.is_empty() {
                    ui.label(widgets::icon(self.has_icons, glyph, fallback).color(color).size(12.0));
                }
                if self.status_kind == StatusKind::Err && self.banner.as_deref() == Some(self.status.as_str()) {
                    // The whole message is in the banner at the top.
                    ui.label(RichText::new(tr("That didn't work. The message at the top says why.")).color(color));
                } else {
                    ui.add(egui::Label::new(RichText::new(&self.status).color(color)).truncate()).on_hover_text(&self.status);
                }
            });
        });
    }
}
