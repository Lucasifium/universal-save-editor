//! The start page shown when no file is open.

use super::*;

impl App {
    pub(super) fn welcome(&mut self, ui: &mut egui::Ui) {
        let mut open: Option<PathBuf> = None;
        let mut restore_missing: Option<PathBuf> = None;
        let mut browse: Option<Option<PathBuf>> = None;
        #[cfg(target_arch = "wasm32")]
        let mut in_place = false;
        ScrollArea::vertical().auto_shrink([false, false]).show_locked(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.08).max(24.0));
                let logo = self.logo.get_or_insert_with(|| {
                    let img = egui::ColorImage::from_rgba_unmultiplied([128, 128], &crate::icon::rgba(128));
                    ui.ctx().load_texture("logo", img, Default::default())
                });
                ui.image((logo.id(), egui::vec2(88.0, 88.0)));
                ui.add_space(6.0);
                ui.label(crate::theme::title(platform::DISPLAY_NAME).color(ui.visuals().strong_text_color()));
                ui.label(if WEB {
                    tr("Edit game saves safely. It runs in your browser and nothing is uploaded.")
                } else if cfg!(target_os = "android") {
                    tr("Edit game saves safely. Everything stays on this device.")
                } else {
                    tr("Edit game saves safely. Everything stays on this computer.")
                });
                ui.add_space(14.0);
                if self.saves_found.is_some() {
                    // Finding the saves for you comes first; picking the file yourself is the other way in.
                    let find = egui::Button::new(RichText::new(tr("Find my game's saves")).color(Color32::WHITE)).fill(theme::ACCENT).min_size(egui::vec2(220.0, 40.0));
                    let file = egui::Button::new(widgets::icon_text(ui, self.has_icons, icon::OPEN, tr("Open a file\u{2026}"))).min_size(egui::vec2(160.0, 40.0));
                    let w = 220.0 + 160.0 + ui.spacing().item_spacing.x;
                    let side_by_side = ui.available_width() >= w + 16.0;
                    let mut clicks = (false, false);
                    if side_by_side {
                        ui.allocate_ui_with_layout(egui::vec2(w, 40.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let r = ui.add(find).on_hover_text(tr("Pick your game from the list; its saves on this computer are found for you"));
                            self.point_at(ui, r.rect, 1);
                            clicks.0 = r.clicked();
                            clicks.1 = ui.add(file).clicked();
                        });
                    } else {
                        let r = ui.add(find);
                        self.point_at(ui, r.rect, 1);
                        clicks.0 = r.clicked();
                        clicks.1 = ui.add(file).clicked();
                    }
                    if clicks.0 {
                        let search = self.saves_found.clone().unwrap_or_else(search_saves);
                        self.game_picker = Some(GamePicker::with(search));
                    }
                    if clicks.1 {
                        browse = Some(None);
                    }
                } else {
                    let btn = egui::Button::new(widgets::icon_text_colored(ui, self.has_icons, icon::OPEN, tr("Open a save file"), Color32::WHITE))
                        .fill(theme::ACCENT)
                        .min_size(egui::vec2(220.0, 40.0));
                    if ui.add(btn).clicked() {
                        browse = Some(None);
                    }
                }
                ui.add_space(4.0);
                if !cfg!(target_os = "android") {
                    ui.weak(tr("or drag a file anywhere onto this window"));
                }
                #[cfg(target_arch = "wasm32")]
                if crate::web::can_save_in_place() {
                    ui.add_space(2.0);
                    in_place = ui
                        .button(tr("Open to save in place\u{2026}"))
                        .on_hover_text(tr("Save then writes back to the file you pick, once you allow it in the browser, instead of downloading it. The file as it was is kept in this browser first. The browser refuses some system folders this way; use Open\u{2026} for those."))
                        .clicked();
                }
                ui.add_space(18.0);

                let w = ui.available_width().min(680.0);
                ui.allocate_ui(egui::vec2(w, 0.0), |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        self.pointers_card(ui);
                        if let Some(p) = self.favourites_card(ui) {
                            open = Some(p);
                        }
                        if self.settings.favourites.iter().chain(self.settings.recent_games.iter()).any(|n| crate::games::GAMES.iter().any(|g| g.name == n)) {
                            ui.add_space(8.0);
                        }
                        let (found, pick_file) = self.your_games(ui);
                        if let Some(p) = found {
                            open = Some(p);
                        }
                        if pick_file {
                            browse = Some(None);
                        }
                        if self.saves_found.is_some() {
                            ui.add_space(8.0);
                        }
                        theme::card(ui, |ui| {
                            ui.label(crate::theme::panel_title(tr("Recent files")));
                            if self.settings.recent.is_empty() {
                                ui.weak(tr("Files you open will appear here."));
                            }
                            for p in self.settings.recent.iter().take(8) {
                                let exists = crate::vfs::exists(p);
                                ui.horizontal(|ui| {
                                    let r = ui.add_enabled(exists, egui::Link::new(file_name(p)));
                                    if r.clicked() {
                                        open = Some(p.clone());
                                    }
                                    let folder = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
                                    if !exists && !WEB && !crate::backups::list(p).is_empty() {
                                        // Set aside for a new game (or deleted): its backups can bring it back.
                                        if crate::widgets::small_button(ui, tr("Restore a backup\u{2026}")).on_hover_text(tr("The file isn't there any more, but it has backups")).clicked() {
                                            restore_missing = Some(p.clone());
                                        }
                                    }
                                    ui.add(egui::Label::new(RichText::new(if exists { folder } else { tr("(file no longer exists)").into() }).weak().size(crate::theme::size::CAPTION)).truncate());
                                });
                            }
                        });
                        ui.add_space(8.0);
                        theme::card(ui, |ui| {
                            ui.label(crate::theme::panel_title(tr("Find your saves")));
                            let locations = settings::save_locations();
                            if WEB {
                                ui.weak(tr("The browser can't look through your folders, so pick the file yourself. On Windows most games keep saves in Documents\\My Games, %APPDATA%, %LOCALAPPDATA% or Steam\\userdata. Type one of these into the file picker's address bar to jump there."));
                            } else if !locations.is_empty() {
                                // Only over a list: nothing found left "Common places…:" over nothing.
                                ui.weak(if cfg!(target_os = "android") {
                                    tr("Common places emulators and games keep save files:")
                                } else {
                                    tr("Common places games keep save files on this PC:")
                                });
                            }
                            for (label, path) in locations {
                                ui.horizontal(|ui| {
                                    if ui.link(&label).clicked() {
                                        browse = Some(Some(path.clone()));
                                    }
                                    ui.add(egui::Label::new(RichText::new(path.display().to_string()).weak().size(crate::theme::size::CAPTION)).truncate());
                                });
                            }
                            ui.weak(tr("RPG Maker games usually keep saves in a \u{201C}save\u{201D} folder next to the game."));
                        });
                        ui.add_space(8.0);
                        // Once the player has opened something, the long list of formats waits behind a link.
                        let what_id = egui::Id::new("start_what_it_opens");
                        let expanded = self.settings.recent.is_empty() || ui.data(|d| d.get_temp::<bool>(what_id).unwrap_or(false));
                        if !expanded {
                            if ui.link(tr("What can it open?")).clicked() {
                                ui.data_mut(|d| d.insert_temp(what_id, true));
                            }
                        } else { theme::card(ui, |ui| {
                            ui.label(crate::theme::panel_title(tr("What it can open")));
                            let games = trf("{0} games, {1} of them with a screen of their own: Elden Ring, Baldur's Gate, Mass Effect, Final Fantasy, Stardew Valley, Terraria, Cyberpunk 2077 and many more \u{00B7} Pok\u{00E9}mon Gen I\u{2013}III (emulator saves) \u{00B7} {2}",
                                &[
                                    &crate::games::GAMES.len(),
                                    &crate::games::GAMES.iter().filter(|g| g.support.starts_with("Game screen")).count(),
                                    &if WEB { tr("the full list is on the website") } else { tr("Find my game's saves lists them all") },
                                ],
                            );
                            let data_formats = if WEB {
                                tr("JSON \u{00B7} XML \u{00B7} INI/CFG \u{00B7} TOML \u{00B7} YAML \u{00B7} plist \u{00B7} MessagePack \u{00B7} BSON \u{00B7} Python pickle (SQLite needs the downloaded app)")
                            } else {
                                tr("JSON \u{00B7} XML \u{00B7} INI/CFG \u{00B7} TOML \u{00B7} YAML \u{00B7} plist \u{00B7} MessagePack \u{00B7} BSON \u{00B7} SQLite \u{00B7} Python pickle")
                            };
                            for (title, list) in [
                                (tr("Games:"), games.as_str()),
                                (tr("Game engines:"), tr("Ren'Py \u{00B7} RPG Maker MV/MZ/XP/VX/VX Ace \u{00B7} Unreal Engine (GVAS) \u{00B7} Unity (.NET BinaryFormatter, Easy Save 3, JSON, XML) \u{00B7} Godot (store_var, encrypted) \u{00B7} Minecraft NBT \u{00B7} TyranoScript \u{00B7} Twine/SugarCube \u{00B7} Barotrauma \u{00B7} Palworld (PlZ)")),
                                (tr("Data formats:"), data_formats),
                                (tr("Wrappers:"), tr("gzip \u{00B7} zlib \u{00B7} zstd \u{00B7} LZ4 \u{00B7} xz \u{00B7} bzip2 \u{00B7} zip \u{00B7} Base64 \u{00B7} UTF-16 \u{00B7} XOR \u{00B7} AES (with password)")),
                                (tr("Anything else:"), tr("Any file opens in the hex editor with value search")),
                            ] {
                                widgets::label_text(ui, title, list, true);
                            }
                        }); }
                    });
                });
            });
        });
        if let Some(p) = restore_missing {
            self.manager = Some((p, String::new(), None));
        }
        if let Some(p) = open {
            if crate::vfs::exists(&p) {
                self.request_open(p);
            } else {
                // A remembered save that is gone: open the list with the notice and its new search.
                let mut gp = self.picker();
                gp.missing = Some(p);
                self.game_picker = Some(gp);
            }
        }
        if let Some(dir) = browse {
            self.ask(Ask::Open, dir, None);
        }
        #[cfg(target_arch = "wasm32")]
        if in_place {
            self.open_in_place();
        }
    }
}
