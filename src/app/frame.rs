//! One frame of the window: input handling and laying out the panels and dialogs.

use super::*;

impl eframe::App for App {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        #[cfg(target_os = "android")]
        {
            let _ = platform::android::CTX.set(ctx.clone());
            platform::android::take_typed(raw);
        }
        wrap_dialog_tab(ctx, raw);
        if self.saving.is_some() {
            hold_input(raw);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame(ctx);
    }
}

/// While a save is being written: takes away every click, key, typed text and dropped file
/// before anything on screen sees it, so nothing can change the file being saved. (Pointer
/// moves and focus changes stay, so the window still looks alive.)
pub(super) fn hold_input(raw: &mut egui::RawInput) {
    raw.events.retain(|e| matches!(e, egui::Event::PointerMoved(_) | egui::Event::PointerGone | egui::Event::WindowFocused(_) | egui::Event::Screenshot { .. }));
    raw.dropped_files.clear();
    raw.hovered_files.clear();
}

impl App {
    /// One frame of the window.
    pub(super) fn frame(&mut self, ctx: &egui::Context) {
        begin_dialogs(ctx);
        let now = ctx.input(|i| i.time);
        let saving = self.saving.is_some();
        crate::widgets::set_busy(saving || self.loading.is_some() || self.save_review.is_some());
        #[cfg(not(target_os = "android"))]
        self_shot(ctx);
        theme::fit_window_margin(ctx);
        window_size::remember(ctx, &mut self.settings);
        self.refresh_changes();

        // Files dropped on the window.
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone())).filter(|_| !saving && !modal_shown(ctx)) {
            self.request_open(p);
        }
        #[cfg(target_arch = "wasm32")]
        {
            // The browser hands over the dropped file's contents, not a path.
            if let Some((name, bytes)) = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| Some((f.name.clone(), f.bytes.clone()?)))) {
                let p = crate::web::keep(&name, &bytes);
                self.request_open(p);
            }
            if let Some(p) = crate::web::take_picked() {
                let what = self.web_asked.take();
                // A file picked to save in place keeps its handle only if it is the one that opens.
                let id = crate::web::settle_handle(&p, matches!(what, None | Some(Ask::Open)) && self.handle_may_follow(&p));
                let loads = self.loads;
                match what {
                    Some(what) => self.answered(what, p.clone()),
                    None => self.request_open(p.clone()),
                }
                if let Some(id) = id {
                    if !self.attach_handle(&p, id, loads) {
                        crate::web::release_handle(id);
                    }
                }
            }
            if let Some(e) = crate::web::take_pick_error() {
                self.fail(trf("Couldn't open the file: {0}", &[&e]));
            }
        }
        // Shortcuts.
        let no_text_focus = ctx.memory(|m| m.focused().is_none());
        if !saving && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.ask(Ask::Open, None, None);
        }
        if !saving && self.doc.is_some() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_clicked();
        }
        let structured = self.doc.as_ref().is_some_and(|d| root(&d.content).is_some());
        if structured && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.focus_search = true;
            self.focus_game_find = self.tab == Tab::Game;
            self.tab = if matches!(self.tab, Tab::Raw | Tab::Tree | Tab::Game) { self.tab } else { Tab::Values };
        }
        if no_text_focus && !saving {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z))
                || ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y))
            {
                self.undo_redo(false);
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.undo_redo(true);
            }
        }
        self.tab_keys(ctx);
        self.palette_frame(ctx);
        // Remember zoom.
        let zoom = ctx.zoom_factor();
        if (zoom - self.settings.zoom).abs() > 0.001 {
            self.settings.zoom = zoom;
            self.settings.save();
        }
        // Don't lose work on close. A save being written is always finished first.
        if ctx.input(|i| i.viewport().close_requested()) {
            if saving {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.close_after_save = true;
            } else if self.any_dirty() && !self.allow_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_first_dirty();
                self.confirm_close = true;
            }
        }

        let title = match &self.doc {
            Some(d) => format!("{}{} \u{2014} {}", if self.dirty { "\u{2022} " } else { "" }, file_name(&d.path), platform::DISPLAY_NAME),
            None => platform::DISPLAY_NAME.into(),
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }

        // Android: keep clear of the taskbar / navigation buttons drawn over the window.
        #[cfg(target_os = "android")]
        {
            let px = platform::android::bottom_inset_px();
            if px > 0.0 {
                egui::TopBottomPanel::bottom("system_bars")
                    .exact_height(px / ctx.pixels_per_point())
                    .frame(egui::Frame::none().fill(ctx.style().visuals.panel_fill))
                    .show_separator_line(false)
                    .show(ctx, |_| {});
            }
        }
        // On a phone the menus fold into the toolbar's menu button.
        if ctx.screen_rect().width() >= PHONE_W {
            egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu_bar(ui));
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        self.tab_strip(ctx);
        if self.doc.is_some() && !self.doc_note.is_empty() && !self.note_hidden() {
            egui::TopBottomPanel::top("doc_note").show(ctx, |ui| self.note_bar(ui));
        }
        // The file is being written by this app: that is not a change on disk to warn about.
        if !saving {
            self.check_disk(ctx);
        }
        self.update_question_dialog(ctx);
        self.poll_update(ctx);
        self.banners(ctx);
        if std::mem::take(&mut self.need_cjk) {
            theme::enable_cjk(ctx);
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.add_space(2.0);
            self.status_bar(ui);
        });

        let kind = self.doc.as_ref().map(|d| match d.content {
            Content::Binary(_) => 'b',
            Content::Text(_) => 't',
            _ => 's',
        });
        if self.show_changes && self.doc.is_some() && kind != Some('t') {
            self.changes_panel(ctx, now);
        }
        if kind == Some('b') {
            self.hex_side_panel(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| match kind {
            None => self.welcome(ui),
            Some('s') => self.structured_panel(ui, now),
            Some('t') => {
                let live = self.saving.is_none();
                let Some(Doc { content: Content::Text(text), .. }) = &mut self.doc else { return };
                let changed = ScrollArea::both()
                    .auto_shrink([false, false])
                    .show_locked(ui, |ui| ui.add(TextEdit::multiline(text).code_editor().interactive(live).desired_width(f32::INFINITY)).changed())
                    .inner;
                if changed {
                    self.edit_gen += 1;
                    self.recompute_dirty();
                }
            }
            _ => {
                let Some(Doc { content: Content::Binary(data), baseline: Content::Binary(base), .. }) = &self.doc else { return };
                if data.is_empty() {
                    ui.weak(tr("(empty file)"));
                } else {
                    hex_view::hex_grid(ui, data, base, &mut self.hex);
                }
            }
        });

        // Escape closes the topmost dialog.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_top_dialog();
        }

        // Dialogs.
        if self.pending_close {
            let mut choice = None;
            modal(ctx, tr("Unsaved changes"), |ui| {
                ui.label(tr("You have unsaved changes. Save them before closing this file?"));
                ui.add_space(8.0);
                choice = ui.horizontal(|ui| save_discard_cancel(ui, tr("Save and close"), tr("Close without saving"))).inner;
            });
            match choice {
                Some(0) => {
                    self.pending_close = false;
                    self.save_then(AfterSave::CloseFile);
                }
                Some(1) => {
                    self.pending_close = false;
                    self.close_file();
                }
                Some(_) => self.pending_close = false,
                None => {}
            }
        }
        if let Some(path) = self.pending_open.clone() {
            let mut choice = None;
            modal(ctx, tr("Unsaved changes"), |ui| {
                ui.label(crate::i18n::trf("You have unsaved changes. Save them before opening {0}?", &[&file_name(&path)]));
                ui.add_space(8.0);
                choice = ui.horizontal(|ui| save_discard_cancel(ui, tr("Save and open"), tr("Open without saving"))).inner;
            });
            match choice {
                Some(0) => {
                    self.pending_open = None;
                    self.save_then(AfterSave::Open(path));
                }
                Some(1) => {
                    self.pending_open = None;
                    self.dirty = false;
                    self.load_async(&path);
                }
                Some(_) => self.pending_open = None,
                None => {}
            }
        }
        self.password_dialog(ctx);
        self.sums_dialog(ctx);
        self.backup_name_dialog(ctx);
        self.new_game_dialog(ctx);
        self.manager_window(ctx);
        self.backup_tools_frame(ctx);
        self.rebind_dialog(ctx);
        self.patches_dialog(ctx);
        self.poll_loading(ctx, now);
        self.poll_saving(ctx, now);
        self.save_review_dialog(ctx);
        self.copy_slots_dialog(ctx);
        self.xbox_steam_dialog(ctx);
        self.game_picker_dialog(ctx);
        self.preset_save_dialog(ctx);
        self.preset_apply_dialog(ctx, now);
        self.share_names_dialog(ctx);
        if self.saving.is_none() {
            self.poll_auto_refine(ctx, now);
        }
        self.compare_window(ctx, now);
        self.stale_dialog(ctx);
        self.quick_restore_dialog(ctx);
        self.prefs_window(ctx);
        self.help_window(ctx);
        self.browser_window(ctx);
        #[cfg(target_arch = "wasm32")]
        self.in_place_frame(ctx);
        // Android: bring up the on-screen keyboard while a text field has focus.
        #[cfg(target_os = "android")]
        {
            let wants = ctx.wants_keyboard_input();
            if wants != self.keyboard_shown {
                self.keyboard_shown = wants;
                platform::android::set_keyboard(wants);
            }
        }
        if self.confirm_close {
            let mut choice = None;
            modal(ctx, tr("Unsaved changes"), |ui| {
                ui.label(tr("You have unsaved changes. Save them before closing?"));
                ui.add_space(8.0);
                choice = ui.horizontal(|ui| save_discard_cancel(ui, tr("Save and close"), tr("Close without saving"))).inner;
            });
            match choice {
                Some(0) => {
                    self.confirm_close = false;
                    self.save_then(AfterSave::Quit);
                }
                Some(1) => {
                    self.confirm_close = false;
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(_) => self.confirm_close = false,
                None => {}
            }
        }
        if std::mem::take(&mut self.close_now) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        theme::focus_ring(ctx);
        // A save started late in this frame (from a dialog): its box and outcome need frames.
        if self.saving.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
}
