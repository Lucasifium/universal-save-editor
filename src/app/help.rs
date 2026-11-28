//! Preferences, the Help menu and its windows (keyboard shortcuts, licences, About), and
//! "Report a problem", which opens a pre-filled GitHub issue form.

use crate::i18n::{tr, tr_any, trf, tx};
use super::{fit, friendly_kind, Ask, App, WEB};
use crate::{platform, theme, widgets};
use eframe::egui::{self, RichText};

pub const SITE: &str = "https://universalsaveeditor.com/";
const DISCUSSIONS: &str = "https://github.com/Lucasifium/universal-save-editor/discussions";
pub const REPO: &str = "https://github.com/Lucasifium/universal-save-editor";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Shortcuts,
    Licences,
    About,
}

const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+O", tx("Open a save file")),
    ("Ctrl+S", tx("Save (shows what changed first)")),
    ("Ctrl+W", tx("Close the open save and go back to the start page")),
    ("Ctrl+Z", tx("Undo")),
    ("Ctrl+Y / Ctrl+Shift+Z", tx("Redo")),
    ("Ctrl+F", tx("Find on the open tab")),
    ("Ctrl+T", tx("Open another save in a new tab")),
    ("Ctrl+Tab / Ctrl+Shift+Tab", tx("Next / previous tab")),
    ("Ctrl+Plus / Ctrl+Minus", tx("Zoom in / out")),
    ("Ctrl+0", tx("Reset the zoom")),
    ("Enter", tx("Confirm a typed value")),
    ("Esc", tx("Close the dialog on top")),
];

fn os() -> &'static str {
    if WEB {
        "Browser"
    } else if cfg!(target_os = "android") {
        "Android"
    } else if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    }
}

/// Percent-encodes a query value.
fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The GitHub form for a problem report, with the version, system and (when a save is open)
/// the kind of save filled in. Nothing about the file itself (its name or folder) goes in it.
pub fn report_url(kind: Option<&str>) -> String {
    let mut u = format!("{REPO}/issues/new?template=save-problem.yml&version={}&platform={}", enc(env!("CARGO_PKG_VERSION")), enc(os()));
    if let Some(k) = kind {
        u.push_str(&format!("&detected={}", enc(k)));
    }
    u
}

pub fn request_url() -> String {
    format!("{REPO}/issues/new?template=request-a-game.yml")
}

fn open_url(ctx: &egui::Context, url: &str) {
    ctx.open_url(egui::OpenUrl::new_tab(url));
}

impl App {
    /// Dark / Light / Same as the system, in Preferences and the View menu. Returns whether
    /// the choice changed (the theme is applied at once).
    pub(super) fn theme_choice(&mut self, ui: &mut egui::Ui) -> bool {
        let s = &mut self.settings;
        let now = (s.follow_system, s.dark);
        let mut pick = now;
        if ui.radio(!s.follow_system && s.dark, tr("Dark")).clicked() {
            pick = (false, true);
        }
        if ui.radio(!s.follow_system && !s.dark, tr("Light")).clicked() {
            pick = (false, false);
        }
        if ui.radio(s.follow_system, tr("Same as the system")).on_hover_text(tr("Dark or light as your system is set, changing with it.")).clicked() {
            pick = (true, s.dark);
        }
        if pick == now {
            return false;
        }
        (s.follow_system, s.dark) = pick;
        theme::apply_choice(ui.ctx(), s.dark, s.follow_system);
        true
    }

    /// The Help menu.
    pub(super) fn help_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(tr("Help"), |ui| {
            if ui.button(tr("Keyboard shortcuts")).clicked() {
                ui.close_menu();
                self.help = Some(Window::Shortcuts);
            }
            if WEB || cfg!(target_os = "android") {
                if ui.button(tr("Supported games (website)")).clicked() {
                    ui.close_menu();
                    open_url(ui.ctx(), SITE);
                }
            } else if ui.button(tr("Supported games\u{2026}")).on_hover_text(tr("Every game in the list, with what can be edited and where its saves are")).clicked() {
                ui.close_menu();
                let mut gp = self.picker();
                gp.only_found = Some(false);
                self.game_picker = Some(gp);
            }
            if ui.button(tr("Website")).on_hover_text(SITE).clicked() {
                ui.close_menu();
                open_url(ui.ctx(), SITE);
            }
            ui.separator();
            if ui.button(tr("Report a problem\u{2026}")).on_hover_text(tr("Opens a form on GitHub. The version and system are filled in; your save is not sent.")).clicked() {
                ui.close_menu();
                let kind = self.doc.as_ref().map(|d| friendly_kind(&d.layers, &d.content));
                open_url(ui.ctx(), &report_url(kind.as_deref()));
            }
            if ui.button(tr("Request a game\u{2026}")).on_hover_text(tr("Opens a form on GitHub")).clicked() {
                ui.close_menu();
                open_url(ui.ctx(), &request_url());
            }
            if ui.button(tr("Questions and ideas\u{2026}")).on_hover_text(tr("Opens the discussions on GitHub, to ask a question or suggest an idea")).clicked() {
                ui.close_menu();
                open_url(ui.ctx(), DISCUSSIONS);
            }
            if crate::update::AVAILABLE {
                ui.separator();
                if ui.add_enabled(self.update_rx.is_none(), egui::Button::new(tr("Check for a new version"))).on_hover_text(tr("Asks GitHub once for the newest release. Nothing else is sent.")).clicked() {
                    ui.close_menu();
                    self.start_update_check(true);
                }
            }
            ui.separator();
            if ui.button(tr("Licences and credits")).clicked() {
                ui.close_menu();
                self.help = Some(Window::Licences);
            }
            if ui.button(trf("About {0}", &[&platform::DISPLAY_NAME])).clicked() {
                ui.close_menu();
                self.help = Some(Window::About);
            }
        });
    }

    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        let Some(which) = self.help else { return };
        let mut open = true;
        let title = match which {
            Window::Shortcuts => tr("Keyboard shortcuts").to_string(),
            Window::Licences => tr("Licences and credits").to_string(),
            Window::About => trf("About {0}", &[&platform::DISPLAY_NAME]),
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .max_width(fit(ctx, 460.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| match which {
                Window::Shortcuts => {
                    egui::Grid::new("shortcuts").num_columns(2).spacing([24.0, 6.0]).striped(true).show(ui, |ui| {
                        for (keys, what) in SHORTCUTS {
                            ui.label(RichText::new(*keys).strong());
                            ui.label(tr_any(what));
                            ui.end_row();
                        }
                    });
                    ui.add_space(4.0);
                    ui.weak(tr("On a Mac, use Cmd instead of Ctrl."));
                }
                Window::Licences => {
                    ui.label(trf("{0} is free software under the GNU General Public License, version 3.", &[&platform::DISPLAY_NAME]));
                    ui.add_space(4.0);
                    ui.label(tr("It builds on save-format research, tables and code from many other projects, each listed with its author and licence in the third-party notices."));
                    ui.add_space(6.0);
                    ui.hyperlink_to(tr("GNU General Public License v3"), format!("{REPO}/blob/main/LICENSE"));
                    ui.hyperlink_to(tr("Third-party notices"), format!("{REPO}/blob/main/THIRD_PARTY_NOTICES.md"));
                    ui.add_space(6.0);
                    ui.weak(tr("The interface uses egui and eframe (MIT or Apache-2.0)."));
                }
                Window::About => {
                    ui.label(RichText::new(platform::DISPLAY_NAME).family(theme::semibold_family()).size(20.0));
                    ui.label(trf("Version {0} \u{00B7} {1}", &[&env!("CARGO_PKG_VERSION"), &os()]));
                    ui.add_space(4.0);
                    ui.label(tr("Edit game saves safely. Everything stays on your device: nothing is uploaded, and every save is backed up before it is written."));
                    ui.add_space(6.0);
                    ui.hyperlink_to(tr("Website"), SITE);
                    ui.hyperlink_to(tr("Source code and releases"), REPO);
                    if WEB {
                        ui.add_space(6.0);
                        ui.weak(tr("In the browser, Japanese, Chinese and Korean names may show as boxes. The downloadable app uses your computer's fonts for them."));
                    }
                }
            });
        if !open {
            self.help = None;
        }
    }

    /// Edit › Preferences: theme, zoom, review before saving, backups.
    pub(super) fn prefs_window(&mut self, ctx: &egui::Context) {
        if !self.prefs_open {
            return;
        }
        let mut open = true;
        let mut changed = false;
        let mut pick_dir = false;
        egui::Window::new(tr("Preferences"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .max_width(fit(ctx, 480.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new(tr("Look")).strong());
                ui.horizontal(|ui| {
                    ui.label(tr("Language:"));
                    let auto = tr("Same as the system");
                    let shown = match &self.settings.language {
                        None => auto.to_string(),
                        Some(c) => crate::i18n::LANGS.iter().find(|l| &l.code == c).map_or(c.clone(), |l| l.name.to_string()),
                    };
                    egui::ComboBox::from_id_salt("language").selected_text(shown).show_ui(ui, |ui| {
                        let mut pick: Option<Option<String>> = None;
                        if ui.selectable_label(self.settings.language.is_none(), auto).clicked() {
                            pick = Some(None);
                        }
                        let cjk = theme::cjk_available();
                        for l in crate::i18n::LANGS.iter().filter(|l| cjk || !l.cjk) {
                            if ui.selectable_label(self.settings.language.as_deref() == Some(l.code), l.name).clicked() {
                                pick = Some(Some(l.code.to_string()));
                            }
                        }
                        if let Some(p) = pick {
                            self.settings.language = p;
                            crate::i18n::set(self.settings.language.as_deref());
                            self.need_cjk |= crate::i18n::current().cjk;
                            changed = true;
                        }
                    });
                })
                .response
                .on_hover_text(tr("Menus, buttons and messages. Game screens and value names stay as the game has them."));
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("Theme:"));
                    changed |= self.theme_choice(ui);
                });
                ui.horizontal(|ui| {
                    ui.label(tr("Zoom:"));
                    let mut z = ui.ctx().zoom_factor();
                    if ui.add(egui::Slider::new(&mut z, 0.5..=3.0).step_by(0.05).custom_formatter(|v, _| format!("{:.0}%", v * 100.0))).changed() {
                        ui.ctx().set_zoom_factor(z);
                    }
                    if ui.small_button(tr("Reset")).clicked() {
                        ui.ctx().set_zoom_factor(1.0);
                    }
                });
                changed |= crate::widgets::checkbox(ui, &mut self.settings.show_advanced, tr("Show rarely needed values")).changed();
                changed |= crate::widgets::checkbox(ui, &mut self.settings.show_raw_keys, tr("Show raw key names")).changed();
                changed |= crate::widgets::checkbox(ui, &mut self.settings.hex_guide, tr("Show the find-a-value guide in the hex view")).changed();
                ui.separator();
                ui.label(RichText::new(tr("Saving")).strong());
                changed |= crate::widgets::checkbox(ui, &mut self.settings.confirm_save, tr("Review changes before saving"))
                    .on_hover_text(tr("Show the list of changes, and warn when the game is running, before writing the save"))
                    .changed();
                if crate::update::AVAILABLE {
                    ui.separator();
                    ui.label(RichText::new(tr("Updates")).strong());
                    changed |= crate::widgets::checkbox(ui, &mut self.settings.check_updates, tr("Tell me when a new version is out"))
                        .on_hover_text(tr("When the program starts, asks GitHub for the newest release. Nothing else is sent. Turn it off and the program stays fully offline."))
                        .changed();
                }
                if !WEB && !cfg!(target_os = "android") {
                    ui.separator();
                    ui.label(RichText::new(tr("Finding saves")).strong());
                    if crate::widgets::checkbox(ui, &mut self.settings.search_removable, tr("Also search removable and network drives"))
                        .on_hover_text(tr("Saves are always looked for on every fixed drive. USB sticks and network drives are skipped unless this is on, because they can be slow. Use Search again after changing it."))
                        .changed()
                    {
                        crate::drives::set_include_removable(self.settings.search_removable);
                        changed = true;
                    }
                }
                if !WEB {
                    ui.separator();
                    ui.label(RichText::new(tr("Backups")).strong());
                    ui.horizontal(|ui| {
                        ui.label(tr("Automatic backups to keep per save:"));
                        let mut n = self.settings.backup_keep;
                        if ui.add(egui::DragValue::new(&mut n).range(crate::backups::KEEP_MIN..=crate::backups::KEEP_MAX)).changed() {
                            self.settings.backup_keep = n;
                            crate::backups::set_keep(n);
                            changed = true;
                        }
                    });
                    ui.weak(tr("A backup is taken every time this app writes a save. Named backups are always kept."));
                    ui.add_space(4.0);
                    let folder = match &self.settings.backup_dir {
                        Some(d) => d.display().to_string(),
                        None => trf("Next to each save, in a \u{201C}{0}\u{201D} folder", &[&crate::backups::BESIDE]),
                    };
                    widgets::label_text(ui, tr("Backups go:"), &folder, true);
                    ui.horizontal(|ui| {
                        if ui.button(tr("Choose a folder\u{2026}")).clicked() {
                            pick_dir = true;
                        }
                        if self.settings.backup_dir.is_some() && ui.button(tr("Back to the default")).clicked() {
                            self.settings.set_backup_dir(None);
                        }
                    });
                }
            });
        if changed {
            self.settings.save();
        }
        if !open {
            self.prefs_open = false;
        }
        if pick_dir {
            self.ask(Ask::BackupDir, self.settings.backup_dir.clone(), None);
        }
    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn report_link_has_no_file_details() {
        let u = super::report_url(Some("Diablo II character"));
        assert!(u.starts_with("https://github.com/Lucasifium/universal-save-editor/issues/new?template=save-problem.yml"));
        assert!(u.contains("&detected=Diablo%20II%20character"));
        // GitHub treats a `format` query parameter as the response format and answers 406.
        assert!(!u.contains("format="));
        assert!(u.contains(&format!("version={}", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn issue_templates_exist() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/ISSUE_TEMPLATE");
        for f in ["save-problem.yml", "request-a-game.yml"] {
            let t = std::fs::read_to_string(root.join(f)).unwrap_or_default();
            assert!(t.contains("name:"), "{f}");
        }
        // The fields the app fills in must exist in the form.
        let t = std::fs::read_to_string(root.join("save-problem.yml")).unwrap();
        for id in ["id: version", "id: platform", "id: detected"] {
            assert!(t.contains(id), "{id}");
        }
    }
}
