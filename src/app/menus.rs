//! The menu bar.

use super::*;

/// The "show in the file manager" command for the open file or (`backups`) its backups folder.
pub(super) fn reveal_text(backups: bool) -> Option<&'static str> {
    platform::reveal_label()?;
    Some(if cfg!(windows) {
        if backups { tr("Show backups in Explorer") } else { tr("Show file in Explorer") }
    } else if cfg!(target_os = "macos") {
        if backups { tr("Show backups in Finder") } else { tr("Show file in Finder") }
    } else if backups {
        tr("Open backups folder")
    } else {
        tr("Open containing folder")
    })
}

impl App {
    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| self.menus(ui));
    }

    /// The menus, in the menu bar or (on a phone) under the toolbar's menu button.
    pub(super) fn menus(&mut self, ui: &mut egui::Ui) {
        {
            let has = self.doc.is_some();
            ui.menu_button(tr("File"), |ui| {
                if ui.add(egui::Button::new(tr("Open\u{2026}")).shortcut_text("Ctrl+O")).clicked() {
                    ui.close_menu();
                    self.ask(Ask::Open, None, None);
                }
                #[cfg(target_arch = "wasm32")]
                self.open_in_place_item(ui);
                if !cfg!(target_os = "android") && !WEB && ui.button(tr("Find my game's saves\u{2026}")).on_hover_text(tr("Pick a game to see the saves it has on this computer")).clicked() {
                    ui.close_menu();
                    self.game_picker = Some(self.picker());
                }
                ui.menu_button(tr("Open recent"), |ui| {
                    let recent = self.settings.recent.clone();
                    if recent.is_empty() {
                        ui.weak(tr("Nothing yet"));
                    }
                    for p in recent {
                        if ui.add_enabled(crate::vfs::exists(&p), egui::Button::new(recent_label(&p))).on_hover_text(p.display().to_string()).clicked() {
                            ui.close_menu();
                            self.request_open(p);
                        }
                    }
                });
                let mut way = None;
                ui.add_enabled_ui(has, |ui| {
                    ui.menu_button(tr("Open as"), |ui| way = self.open_as_items(ui)).response.on_hover_text(tr("Read this file another way, for when the editor guessed wrong"));
                });
                if let Some(w) = way {
                    ui.close_menu();
                    self.open_as(w);
                }
                ui.separator();
                if ui.add_enabled(has, egui::Button::new(tr("Save")).shortcut_text("Ctrl+S")).clicked() {
                    ui.close_menu();
                    self.save_clicked();
                }
                if ui.add_enabled(has, egui::Button::new(tr("Save a copy as\u{2026}"))).clicked() {
                    ui.close_menu();
                    self.save_as();
                }
                let on_disk_file = self.doc.as_ref().is_some_and(|d| crate::vfs::exists(&d.path));
                if ui
                    .add_enabled(on_disk_file, egui::Button::new(tr("Reload from disk")))
                    .on_hover_text(tr("Reads the file again, for when the game or another program changed it. Unsaved changes are dropped (you are asked first)."))
                    .clicked()
                {
                    ui.close_menu();
                    if let Some(p) = self.doc.as_ref().map(|d| d.path.clone()) {
                        self.request_open(p);
                    }
                }
                if ui.add_enabled(has, egui::Button::new(tr("Close save")).shortcut_text("Ctrl+W")).clicked() {
                    ui.close_menu();
                    self.close_clicked();
                }
                ui.separator();
                if ui.add_enabled(self.dirty, egui::Button::new(tr("Revert all changes"))).clicked() {
                    ui.close_menu();
                    self.revert_all();
                }
                ui.separator();
                if ui.add_enabled(has, egui::Button::new(tr("Export decoded data\u{2026}"))).clicked() {
                    ui.close_menu();
                    self.export_decoded();
                }
                let importable = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_) | Content::Text(_) | Content::Json { .. }));
                if ui
                    .add_enabled(importable, egui::Button::new(tr("Import edited data\u{2026}")))
                    .on_hover_text(tr("Bring back data you exported and edited in another program; it is packed the way this save is packed when you save."))
                    .clicked()
                {
                    ui.close_menu();
                    self.ask(Ask::Import, None, None);
                }
            });
            let backups_menu = ui.menu_button(tr("Backups"), |ui| {
                let on_disk = !WEB && self.doc.as_ref().is_some_and(|d| d.path.is_file());
                if !WEB && ui.add_enabled(on_disk, egui::Button::new(tr("Back up now\u{2026}"))).on_hover_text(tr("Keeps a named backup of the save file as it is on disk now, until you delete it. The save itself is not changed.")).clicked() {
                    ui.close_menu();
                    self.backup_name = Some(String::new());
                }
                if !WEB && ui.add_enabled(on_disk, egui::Button::new(tr("Back up and start a new game\u{2026}"))).on_hover_text(tr("Keeps a named backup, then removes the save file so the game starts a new game. Restore the backup any time to go back.")).clicked() {
                    ui.close_menu();
                    self.new_game_name = Some(String::new());
                }
                if !WEB && ui.add_enabled(has, egui::Button::new(tr("Save manager\u{2026}"))).on_hover_text(tr("All backups of this save in one window: restore, compare, delete, name. Restoring backs up the current save first.")).clicked() {
                    ui.close_menu();
                    self.open_save_manager();
                }
                if !WEB && ui.button(tr("Backup space\u{2026}")).on_hover_text(tr("How much room all backups take, and ways to free some")).clicked() {
                    ui.close_menu();
                    self.open_backup_space();
                }
                #[cfg(target_arch = "wasm32")]
                self.originals_item(ui);
                ui.separator();
                let bak = self.doc.as_ref().map(|d| {
                    let mut b = d.path.as_os_str().to_owned();
                    b.push(".bak");
                    PathBuf::from(b)
                });
                let versions = self.doc.as_ref().map(|d| crate::backups::list(&d.path)).unwrap_or_default();
                let has_bak = bak.as_ref().is_some_and(|b| b.exists());
                let on_disk = !WEB && self.doc.as_ref().is_some_and(|d| d.path.is_file());
                ui.add_enabled_ui(has_bak || !versions.is_empty() || on_disk, |ui| {
                    ui.menu_button(tr("Quick restore"), |ui| {
                        ui.weak(tr("Automatic backups are taken each time this app saves over the file."));
                        ui.weak(trf("Named backups are kept until you delete them. Only the {0} newest automatic ones are kept.", &[&crate::backups::keep()]));
                        ui.separator();
                        let mut pick = None;
                        let mut compare = None;
                        let mut unlock = None;
                        // Inside a pop-up menu: shrink to the rows, the menu sets the width.
                        ScrollArea::vertical().auto_shrink([true, true]).max_height(crate::widgets::list_height(ui, 360.0)).show_locked(ui, |ui| {
                            for (i, v) in versions.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    let when = crate::backups::describe(v.when);
                                    let label = match &v.name {
                                        Some(n) => format!("\u{201C}{n}\u{201D} \u{00B7} {when} ({})", human_size(v.size as usize)),
                                        None => format!("{when} ({})", human_size(v.size as usize)),
                                    };
                                    if ui.button(label).on_hover_text(tr("Replaces the current save with this backup. The current save is backed up first, so you can undo it.")).clicked() {
                                        pick = Some((v.path.clone(), trf("the backup from {0}", &[&when])));
                                    }
                                    if crate::widgets::small_button(ui, tr("Compare")).on_hover_text(tr("Shows what differs between this backup and the open save")).clicked() {
                                        compare = Some(v.path.clone());
                                    }
                                    if v.name.is_some() && crate::widgets::small_button(ui, tr("Make automatic")).on_hover_text(tr("Stops keeping this backup for good; it is cleared out in time like the automatic ones")).clicked() {
                                        unlock = Some(i);
                                    }
                                });
                            }
                        });
                        if let Some(p) = compare {
                            ui.close_menu();
                            self.compare_with(p);
                        }
                        if let Some(i) = unlock {
                            if let Err(e) = crate::backups::unlock(&versions[i]) {
                                self.fail(trf("Couldn't change that backup: {0}", &[&e]));
                            }
                        }
                        if has_bak {
                            ui.separator();
                            if ui.button(tr("Original save (.bak)")).on_hover_text(tr("The original save, from before this app first saved over it")).clicked() {
                                pick = bak.clone().map(|b| (b, tr("the original save").to_string()));
                            }
                        }
                        if let Some(dir) = self.doc.as_ref().and_then(|d| crate::backups::folder(&d.path)) {
                            if dir.is_dir() {
                                if let Some(label) = reveal_text(true) {
                                    ui.separator();
                                    if ui.button(label).clicked() {
                                        ui.close_menu();
                                        platform::reveal(&dir);
                                    }
                                }
                            }
                        }
                        if let Some((p, what)) = pick {
                            ui.close_menu();
                            self.ask_restore(p, what);
                        }
                    });
                });
            });
            self.point_at(ui, backups_menu.response.rect, 3);
            ui.menu_button(tr("Edit"), |ui| {
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new(tr("Undo")).shortcut_text("Ctrl+Z")).clicked() {
                    ui.close_menu();
                    self.undo_redo(true);
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new(tr("Redo")).shortcut_text("Ctrl+Y")).clicked() {
                    ui.close_menu();
                    self.undo_redo(false);
                }
                ui.separator();
                if ui.add_enabled(has, egui::Button::new(tr("Find\u{2026}")).shortcut_text("Ctrl+F")).clicked() {
                    ui.close_menu();
                    self.focus_search = true;
                    self.focus_game_find = self.tab == Tab::Game;
                }
                if ui.add(egui::Button::new(tr("Search everything\u{2026}")).shortcut_text("Ctrl+K")).on_hover_text(tr("Search everything: commands, games, recent files and values")).clicked() {
                    ui.close_menu();
                    self.open_palette();
                }
                ui.separator();
                if ui.add_enabled(self.dirty, egui::Button::new(tr("Copy changes as text"))).on_hover_text(tr("The list of changes, to paste when asking for help or sharing what you did")).clicked() {
                    ui.close_menu();
                    let ctx = ui.ctx().clone();
                    self.copy_changes(&ctx);
                }
                ui.separator();
                if ui.button(tr("Preferences\u{2026}")).clicked() {
                    ui.close_menu();
                    self.prefs_open = true;
                }
            });
            ui.menu_button(tr("View"), |ui| {
                let mut changed = false;
                changed |= crate::widgets::checkbox(ui, &mut self.settings.show_advanced, tr("Show rarely needed values")).changed();
                changed |= crate::widgets::checkbox(ui, &mut self.settings.show_raw_keys, tr("Show raw key names")).changed();
                crate::widgets::checkbox(ui, &mut self.show_changes, tr("Changes panel"));
                if ui.add_enabled(self.doc.is_some() && !self.doc_note.is_empty() && self.note_hidden(), egui::Button::new(tr("Show the note about this file"))).clicked() {
                    let key = Settings::note_key(&self.doc_note);
                    self.settings.hidden_notes.retain(|k| *k != key);
                    changed = true;
                }
                changed |= crate::widgets::checkbox(ui, &mut self.settings.confirm_save, tr("Review changes before saving"))
                    .on_hover_text(tr("Lists what changed and checks whether the game is running each time you save."))
                    .changed();
                ui.separator();
                ui.label(tr("Theme:"));
                changed |= self.theme_choice(ui);
                ui.separator();
                ui.weak(tr("Zoom: Ctrl + / Ctrl \u{2212} / Ctrl 0"));
                ui.separator();
                let structured = self.doc.as_ref().is_some_and(|d| !matches!(d.content, Content::Binary(_)));
                if ui.add_enabled(structured, egui::Button::new(tr("Show decoded data as hex"))).clicked() {
                    ui.close_menu();
                    self.view_as_hex();
                }
                if changed {
                    self.settings.save();
                }
            });
            ui.menu_button(tr("Tools"), |ui| {
                let is_rpg = matches!(&self.doc, Some(Doc { content: Content::Json { value, .. }, .. }) if names::is_rpgmaker(value));
                if !WEB && ui.add_enabled(is_rpg, egui::Button::new(tr("Locate game data folder (RPG Maker names)\u{2026}"))).on_disabled_hover_text(tr("Only for RPG Maker saves.")).clicked() {
                    ui.close_menu();
                    self.locate_data_dir();
                }
                let has = self.doc.is_some();
                if ui
                    .add_enabled(has, egui::Button::new(tr("Decrypt with password\u{2026}")))
                    .on_disabled_hover_text(tr("Open a save first."))
                    .on_hover_text(tr("For encrypted saves (Easy Save 3, Godot). Easy Save's default password is tried automatically."))
                    .clicked()
                {
                    ui.close_menu();
                    self.password_prompt = Some(self.doc.as_ref().and_then(|d| d.opts.password.clone()).unwrap_or_default());
                }
                if ui
                    .add_enabled(has, egui::Button::new(tr("Change Steam account in this save\u{2026}")))
                    .on_disabled_hover_text(tr("Open a save first."))
                    .on_hover_text(tr("Finds the Steam IDs stored in the save and changes them to another account's, so a save copied from a friend or another account loads."))
                    .clicked()
                {
                    ui.close_menu();
                    self.start_rebind();
                }
                let binary = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_)));
                if ui
                    .add_enabled(binary, egui::Button::new(tr("Cheat patches\u{2026}")))
                    .on_disabled_hover_text(tr("Only for saves opened in the hex view, which means a format the editor doesn't understand yet."))
                    .on_hover_text(tr("Ready-made cheats for console saves (PlayStation 1\u{2013}4, PSP, Vita, Dreamcast) from the Apollo patch collection."))
                    .clicked()
                {
                    ui.close_menu();
                    self.start_patches();
                }
                if ui
                    .add_enabled(binary, egui::Button::new(tr("Find checksum\u{2026}")))
                    .on_disabled_hover_text(tr("Only for saves opened in the hex view, which means a format the editor doesn't understand yet."))
                    .on_hover_text(tr("Looks for a checksum in a save the editor doesn't know (CRC, Adler, sums, MD5, SHA) and keeps it right whenever you save."))
                    .clicked()
                {
                    ui.close_menu();
                    self.find_sums();
                }
                #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
                if ui
                    .button(tr("Templates folder\u{2026}"))
                    .on_hover_text(tr("Your own templates for games the editor doesn't know: a JSON file per game that says where each value is. The folder has a guide with an example."))
                    .clicked()
                {
                    ui.close_menu();
                    self.open_templates_folder();
                }
                ui.menu_button(tr("Presets"), |ui| {
                    if ui
                        .add_enabled(self.dirty, egui::Button::new(tr("Save changes as a preset\u{2026}")))
                        .on_hover_text(tr("Keep the changes you made, to make them again on another save or slot"))
                        .clicked()
                    {
                        ui.close_menu();
                        self.start_preset_save();
                    }
                    if ui.add_enabled(has, egui::Button::new(tr("Apply a preset\u{2026}"))).on_hover_text(tr("Make a saved set of changes on this save")).clicked() {
                        ui.close_menu();
                        self.start_preset_apply();
                    }
                    if ui.button(tr("Open a preset file\u{2026}")).on_hover_text(tr("Choose a preset file someone shared with you, to look at it and apply it")).clicked() {
                        ui.close_menu();
                        self.start_preset_open();
                    }
                });
                if ui
                    .add_enabled(has, egui::Button::new(tr("Compare with another save\u{2026}")))
                    .on_disabled_hover_text(tr("Open a save first."))
                    .on_hover_text(tr("List every value that differs from another save. Handy for finding which value is your gold or HP."))
                    .clicked()
                {
                    ui.close_menu();
                    self.start_compare();
                }
                let entries = self.doc.as_ref().map(|d| d.zip_entries.clone()).unwrap_or_default();
                if entries.len() > 1 {
                    ui.menu_button(tr("Open another file in this archive"), |ui| {
                        let current = self.doc.as_ref().and_then(|d| {
                            d.layers.iter().find_map(|l| match l {
                                Layer::ZipEntry { name, .. } | Layer::Copeck { name, .. } => Some(name.clone()),
                                Layer::Ps2Card { file, .. } => Some(file.name.clone()),
                                _ => None,
                            })
                        });
                        for e in entries {
                            if ui.selectable_label(current.as_deref() == Some(&e), &e).clicked() {
                                ui.close_menu();
                                self.reopen_with(|o| o.zip_entry = Some(e.clone()));
                            }
                        }
                    });
                }
                self.copy_slot_button(ui);
                self.xbox_steam_button(ui);
                ui.separator();
                if let (Some(d), Some(label)) = (&self.doc, reveal_text(false)) {
                    if ui.button(label).clicked() {
                        ui.close_menu();
                        platform::reveal(&d.path);
                    }
                }
            });
            self.help_menu(ui);
        }
    }
}
