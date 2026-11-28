//! Smaller dialogs: Find my game's saves, starting a new game, naming a backup, the
//! backups window, the Steam ID, checksums and password boxes.

use super::*;

pub(super) fn v_when(v: &[crate::backups::Version], p: &PathBuf) -> u64 {
    v.iter().find(|x| x.path == *p).map(|x| x.when).unwrap_or(0)
}

impl App {
    pub(super) fn game_picker_dialog(&mut self, ctx: &egui::Context) {
        let Some(gp) = &mut self.game_picker else { return };
        gp.favs = self.settings.favourites.clone();
        let (open, close) = game_picker_window(ctx, gp);
        if gp.favs != self.settings.favourites {
            self.settings.favourites = gp.favs.clone();
            self.settings.save();
        }
        let picked_game = gp.picked.map(|i| crate::games::GAMES[i].name);
        if close {
            self.game_picker = None;
        }
        if let Some(p) = open {
            if crate::vfs::exists(&p) {
                self.game_picker = None;
                if let Some(name) = picked_game {
                    self.settings.push_recent_game(name);
                }
                self.request_open(p);
            } else if let Some(gp) = &mut self.game_picker {
                // The remembered folder is out of date: say so and offer a new search.
                gp.missing = Some(p);
            }
        }
    }

    /// Backs the save up under `name`, then removes it and closes it so the game starts a new game. True on success.
    pub(super) fn start_new_game(&mut self, path: &Path, name: &str) -> bool {
        if let Some(game) = crate::running::game_for(path) {
            self.fail(trf("{0} looks like it's running. Quit it first, then try again. The save wasn't changed.", &[&game]));
            return false;
        }
        match crate::backups::set_aside(path, name) {
            Ok(_) => {
                // Close the file so its old data can't be saved back over the new game.
                self.doc = None;
                self.dirty = false;
                self.ok(trf("Backed up as \u{201C}{0}\u{201D} and removed the save, so the game starts a new game. Restore it from Backups \u{203A} Save manager\u{2026} any time.", &[&name.trim()]));
                true
            }
            Err(e) => {
                self.fail(trf("Couldn't start a new game: {0}. The save is still in place.", &[&e]));
                false
            }
        }
    }

    pub(super) fn new_game_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.new_game_name.take() else { return };
        let mut choice = None;
        modal(ctx, tr("Back up and start a new game"), |ui| {
            ui.label(tr("The save is backed up under this name and then removed, so the game starts a new game. Restore the backup later to come back to it."));
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text(tr("for example: first run")).desired_width(280.0));
            r.request_focus();
            let ready = !text.trim().is_empty();
            if r.lost_focus() && ready && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(ready, egui::Button::new(RichText::new(tr("Back up and start a new game")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
                if self.start_new_game(&path, &text) {
                    self.manager = Some((path, String::new(), None));
                } else {
                    self.new_game_name = Some(text);
                }
            }
            Some(false) => {}
            None => self.new_game_name = Some(text),
        }
    }

    pub(super) fn backup_name_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.backup_name.take() else { return };
        let mut choice = None;
        modal(ctx, tr("Back up this save"), |ui| {
            ui.label(tr("Backs up the save as it is on disk now, without unsaved changes. The backup is kept until you delete it."));
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text(tr("for example: before the final boss")).desired_width(280.0));
            r.request_focus();
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(tr("Back up")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
                match crate::backups::take_named(&path, &text) {
                    Ok(_) => self.ok(trf("Backed up this save as \u{201C}{0}\u{201D}.", &[&text.trim()])),
                    Err(e) => {
                        self.fail(trf("Couldn't make the backup: {0}. The save wasn't changed.", &[&e]));
                        self.backup_name = Some(text);
                    }
                }
            }
            Some(false) => {}
            None => self.backup_name = Some(text),
        }
    }

    /// Backups › Open save manager: named backups (named copies, kept for good) and automatic backups of the
    /// open file, stored next to it. Restoring backs up the current file first.
    pub(super) fn manager_window(&mut self, ctx: &egui::Context) {
        let Some((path, mut name, mut confirm)) = self.manager.take() else { return };
        // Another file was opened: this window was for the one before.
        if self.doc.as_ref().is_some_and(|d| d.path != path) {
            return;
        }
        let versions = crate::backups::list(&path);
        let mut open = true;
        let mut restore = None;
        let mut compare = None;
        let mut delete = None;
        let mut unname = None;
        let mut keep = false;
        let mut aside = false;
        let mut change_dir = false;
        let mut reset_dir = false;
        let mut tool_act = None;
        let has_doc = self.doc.is_some();
        let dirty = self.dirty;
        let newest_dir = versions.first().and_then(|v| v.path.parent()).map(Path::to_path_buf);
        egui::Window::new(tr("Save manager")).open(&mut open).collapsible(false).default_width(fit(ctx, 760.0)).max_width(fit(ctx, 900.0)).show(ctx, |ui| {
            ui.label(RichText::new(file_name(&path)).strong());
            if !path.exists() {
                ui.label(RichText::new(tr("There is no save file here right now. The game makes a new one when you save in it, or restore a backup below to put one back.")).color(theme::fg(ui, theme::WARN)));
            }
            if let Some(dir) = crate::backups::folder(&path) {
                let note = if self.settings.backup_dir.is_some() { tr("the folder you chose for backups") } else { tr("a folder next to the save, with files no game reads as saves") };
                ui.weak(trf("Backups are stored in {0} ({1}).", &[&dir.display(), &note]));
                if let Some(other) = newest_dir.as_ref().filter(|d| **d != dir) {
                    ui.weak(trf("The newest backup is in {0} (the folder above couldn't be written to, or it was made earlier).", &[&other.display()]));
                }
                if !WEB && !cfg!(target_os = "android") {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr("Keep backups in another folder\u{2026}")).clicked() {
                            change_dir = true;
                        }
                        if self.settings.backup_dir.is_some() && ui.button(tr("Back to the default folder")).clicked() {
                            reset_dir = true;
                        }
                    });
                }
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(TextEdit::singleline(&mut name).hint_text(tr("backup name, e.g. first run")).desired_width(260.0));
                let ready = !name.trim().is_empty() && path.is_file();
                if ui.add_enabled(ready, egui::Button::new(tr("Back up this save"))).on_disabled_hover_text(tr("Type a name for the backup first")).on_hover_text(tr("Keeps a named backup of the save file as it is on disk now, until you delete it. The save itself is not changed.")).clicked() {
                    keep = true;
                }
                if ui.add_enabled(ready, egui::Button::new(tr("Back up and start a new game"))).on_hover_text(tr("Saves a named backup, then removes the current save file so the game starts a new game. Restore the backup later to go back.")).clicked() {
                    aside = true;
                }
            });
            ui.weak(tr("For games with a single save: use \u{201C}Back up and start a new game\u{201D}, play a new run, and restore the old backup later."));
            ui.separator();
            if versions.is_empty() {
                ui.label(tr("No backups yet. One is made automatically each time you save, or press \u{201C}Back up this save\u{201D}."));
            }
            ScrollArea::vertical().show_list_fit(ui, "app_list3", 380.0, |ui| {
                for (title, named) in [(tr("Named backups (kept until you delete them)"), true), (trf("Automatic backups (made each time you save; newest {0} kept)", &[&crate::backups::keep()]).as_str(), false)] {
                    let rows: Vec<&crate::backups::Version> = versions.iter().filter(|v| v.name.is_some() == named).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    ui.strong(title);
                    for v in rows {
                        ui.horizontal_wrapped(|ui| {
                            let when = crate::backups::describe(v.when);
                            let label = match &v.name {
                                Some(n) => format!("\u{201C}{n}\u{201D} \u{00B7} {when} ({})", human_size(v.size as usize)),
                                None => format!("{when} ({})", human_size(v.size as usize)),
                            };
                            ui.label(label);
                            self.backup_tools.mark(ui, &v.path);
                            if confirm.as_ref().is_some_and(|(p, _)| *p == v.path) {
                                let is_restore = confirm.as_ref().is_some_and(|(_, r)| *r);
                                let yes = match (is_restore, has_doc && dirty) {
                                    (true, true) => tr("Restore and drop unsaved changes"),
                                    (true, false) => tr("Restore now"),
                                    _ => tr("Delete permanently"),
                                };
                                if ui.button(yes).clicked() {
                                    if is_restore { restore = Some(v.path.clone()) } else { delete = Some(v.path.clone()) }
                                    confirm = None;
                                }
                                if ui.button(tr("Cancel")).clicked() {
                                    confirm = None;
                                }
                            } else {
                                if crate::widgets::small_button(ui, tr("Restore this backup")).on_hover_text(tr("Replaces the current save with this backup. The current save is backed up first, so you can undo it.")).clicked() {
                                    confirm = Some((v.path.clone(), true));
                                }
                                if has_doc && path.is_file() && crate::widgets::small_button(ui, tr("Compare")).on_hover_text(tr("Shows what differs between this backup and the open save")).clicked() {
                                    compare = Some(v.path.clone());
                                }
                                if v.name.is_some() && crate::widgets::small_button(ui, tr("Make automatic")).on_hover_text(tr("Stops keeping this backup for good; it is cleared out in time like the automatic ones")).clicked() {
                                    unname = Some(v.path.clone());
                                }
                                if crate::widgets::small_button(ui, tr("Delete backup")).on_hover_text(tr("Permanently removes this backup")).clicked() {
                                    confirm = Some((v.path.clone(), false));
                                }
                            }
                        });
                    }
                    ui.add_space(6.0);
                }
            });
            if let Some(dir) = newest_dir.clone().or_else(|| crate::backups::folder(&path)).filter(|d| d.is_dir()) {
                if let Some(label) = reveal_text(true) {
                    if ui.button(label).clicked() {
                        platform::reveal(&dir);
                    }
                }
            }
            ui.separator();
            tool_act = self.backup_tools.ui(ui, &path, &versions, has_doc && dirty);
        });
        if let Some(a) = tool_act.take() {
            self.backup_act(a, &path);
        }
        let find = |p: &PathBuf| versions.iter().find(|v| v.path == *p);
        if change_dir {
            self.ask(Ask::BackupDir, self.settings.backup_dir.clone(), None);
        }
        if reset_dir {
            self.settings.set_backup_dir(None);
        }
        if keep {
            match crate::backups::take_named(&path, &name) {
                Ok(_) => {
                    self.ok(trf("Backed up this save as \u{201C}{0}\u{201D}.", &[&name.trim()]));
                    name.clear();
                }
                Err(e) => self.fail(trf("Couldn't make the backup: {0}. The save wasn't changed.", &[&e])),
            }
        }
        if aside && self.start_new_game(&path, &name) {
            self.manager = Some((path, String::new(), None));
            return;
        }
        if let Some(p) = restore {
            let what = find(&p).map(|v| v.name.as_ref().map(|n| format!("\u{201C}{n}\u{201D}")).unwrap_or_else(|| trf("the backup from {0}", &[&crate::backups::describe(v_when(&versions, &p))]))).unwrap_or_default();
            self.restore_to(&path, &p, &what);
        }
        if let Some(p) = compare {
            self.compare_with(p);
        }
        if let Some(v) = unname.as_ref().and_then(find) {
            match crate::backups::unlock(v) {
                Ok(()) => self.ok(tr("That backup is automatic now: it is cleared out in time like the others.")),
                Err(e) => self.fail(trf("Couldn't change that backup: {0}", &[&e])),
            }
        }
        if let Some(v) = delete.as_ref().and_then(find) {
            match crate::backups::remove(v) {
                Ok(()) => self.ok(tr("Deleted the backup.")),
                Err(e) => self.fail(trf("Couldn't delete the backup: {0}. It is still in the list.", &[&e])),
            }
        }
        // A restore opens the file, which closes this window unless it is still wanted.
        if open && self.manager.is_none() {
            self.manager = Some((path, name, confirm));
        }
    }

    pub(super) fn start_rebind(&mut self) {
        let ids = self.ids_in_save();
        if ids.is_empty() {
            return self.fail(tr("No Steam ID was found in this save's data (some games key the save to the account in other ways)."));
        }
        self.rebind = Some((ids, 0, String::new()));
    }

    pub(super) fn rebind_dialog(&mut self, ctx: &egui::Context) {
        let Some((ids, mut pick, mut text)) = self.rebind.take() else { return };
        #[cfg(not(target_arch = "wasm32"))]
        let locals = crate::steamid::local_accounts();
        #[cfg(target_arch = "wasm32")]
        let locals: Vec<u64> = vec![];
        let mut choice = None;
        modal(ctx, tr("Change Steam account in this save"), |ui| {
            ui.label(tr("Steam IDs in this save:"));
            for (i, id) in ids.iter().enumerate() {
                ui.radio_value(&mut pick, i, format!("{id} (account {})", id - crate::steamid::BASE));
            }
            ui.add_space(6.0);
            ui.label(tr("Change it to:"));
            ui.add(TextEdit::singleline(&mut text).hint_text(tr("17-digit Steam ID or account number")).desired_width(280.0));
            if !locals.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.weak(tr("Accounts on this computer:"));
                    for id in &locals {
                        if crate::widgets::small_button(ui, (id - crate::steamid::BASE).to_string()).clicked() {
                            text = id.to_string();
                        }
                    }
                });
            }
            ui.weak(tr("Every copy of the ID changes; save to write the file. Some games also check the folder the save sits in."));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ok = crate::steamid::parse(&text).is_some();
                if ui.add_enabled(ok, egui::Button::new(RichText::new(tr("Change")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let (Some(new), Some(&old)) = (crate::steamid::parse(&text), ids.get(pick)) else { return };
                let n = self.rebind_to(old, new);
                self.ok(if n == 1 { tr("Changed the Steam ID in 1 place. Save to write the file.").to_string() } else { trf("Changed the Steam ID in {0} places. Save to write the file.", &[&n]) });
            }
            Some(false) => {}
            None => self.rebind = Some((ids, pick, text)),
        }
    }

    /// Changes every copy of `old` to `new`; returns how many places changed.
    pub(super) fn rebind_to(&mut self, old: u64, new: u64) -> usize {
        let Some(d) = &self.doc else { return 0 };
        match &d.content {
            Content::Binary(b) => {
                let at = crate::steamid::in_bytes(b).into_iter().find(|(id, _)| *id == old).map(|(_, a)| a).unwrap_or_default();
                for &o in &at {
                    self.apply_bytes(o, new.to_le_bytes().to_vec());
                }
                at.len()
            }
            c => {
                let Some(v) = root(c) else { return 0 };
                let ptrs = crate::steamid::in_values(v).into_iter().find(|(id, _)| *id == old).map(|(_, p)| p).unwrap_or_default();
                let edits: Vec<(String, Value)> = ptrs.iter().filter_map(|p| v.pointer(p).map(|x| (p.clone(), crate::steamid::replaced(x, new)))).collect();
                for (p, val) in &edits {
                    self.apply_edit(p.clone(), val.clone(), 0.0);
                    self.last_edit = None;
                }
                edits.len()
            }
        }
    }

    pub(super) fn find_sums(&mut self) {
        let Some(d) = &self.doc else { return };
        let Content::Binary(b) = &d.content else { return };
        // Earlier copies of the file weed out values that only match by chance.
        #[cfg(not(target_arch = "wasm32"))]
        let others: Vec<Vec<u8>> = crate::backups::list(&d.path)
            .into_iter()
            .filter(|v| v.size as usize == b.len())
            .take(8)
            .filter_map(|v| std::fs::read(&v.path).ok())
            .collect();
        #[cfg(target_arch = "wasm32")]
        let others: Vec<Vec<u8>> = vec![];
        self.sum_hits = Some(crate::fmt::sumfind::find(b, &others));
    }

    pub(super) fn sums_dialog(&mut self, ctx: &egui::Context) {
        let Some(hits) = self.sum_hits.clone() else { return };
        let current = self.doc.as_ref().and_then(|d| d.opts.sum);
        let mut close = false;
        let mut pick: Option<Option<crate::fmt::sumfind::Sum>> = None;
        modal(ctx, tr("Find checksum"), |ui| {
            if hits.is_empty() {
                ui.label(tr("No checksum the editor knows was found at the start or end of this file."));
                ui.weak(tr("It may be encrypted, compressed, or checked with something the game made up. The value search still works on it."));
            } else {
                ui.label(if hits.len() == 1 { tr("This checksum holds for the file:") } else { tr("These checksums hold for the file:") });
                ui.weak(tr("Pick one to have it written again every time you save. Saving a few times and searching again narrows a long list."));
                ui.add_space(6.0);
                for h in &hits {
                    ui.horizontal(|ui| {
                        ui.label(h.describe());
                        if current == Some(*h) {
                            ui.weak(tr("(kept fixed)"));
                        } else if ui.button(tr("Keep fixed")).clicked() {
                            pick = Some(Some(*h));
                        }
                    });
                }
            }
            if current.is_some() && ui.button(tr("Stop fixing the checksum")).clicked() {
                pick = Some(None);
            }
            ui.add_space(6.0);
            if ui.button(tr("Close")).clicked() {
                close = true;
            }
        });
        if let Some(p) = pick {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(d) = &self.doc {
                let _ = crate::fmt::sumfind::remember(&d.path, p);
            }
            self.sum_hits = None;
            self.reopen_with(|o| o.sum = p);
            if p.is_some() {
                self.ok(tr("The checksum will be written again every time you save this file."));
            }
        } else if close {
            self.sum_hits = None;
        }
    }

    pub(super) fn password_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.password_prompt.take() else { return };
        let mut choice = None;
        modal(ctx, tr("Decrypt with password"), |ui| {
            ui.label(tr("Some games encrypt their saves with a password built into the game."));
            ui.weak(tr("Supported: Easy Save 3 (Unity) and Godot encrypted files. Easy Save's default password is tried automatically."));
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text(tr("password")).desired_width(280.0));
            r.request_focus();
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(tr("Decrypt")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let pw = text.clone();
                self.reopen_with(|o| o.password = (!pw.is_empty()).then_some(pw.clone()));
                if matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_))) {
                    self.fail(tr("That password didn't decrypt the file (or the encryption isn't one this editor knows)."));
                }
            }
            Some(false) => {}
            None => self.password_prompt = Some(text),
        }
    }
}
