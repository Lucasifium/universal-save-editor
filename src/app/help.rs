//! Preferences, the Help menu and its windows (keyboard shortcuts, licences, About), and
//! "Report a problem", which opens a pre-filled GitHub issue form.

use super::{fit, friendly_kind, Ask, App, WEB};
use crate::{platform, theme};
use eframe::egui::{self, RichText};

pub const SITE: &str = "https://lucasifium.github.io/universal-save-editor/";
pub const REPO: &str = "https://github.com/Lucasifium/universal-save-editor";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Shortcuts,
    Licences,
    About,
}

const SHORTCUTS: &[(&str, &str)] = &[
    ("Ctrl+O", "Open a save file"),
    ("Ctrl+S", "Save (shows what changed first)"),
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Y or Ctrl+Shift+Z", "Redo"),
    ("Ctrl+F", "Find a value by name (Values tab)"),
    ("Ctrl+Plus / Ctrl+Minus", "Zoom in / out"),
    ("Ctrl+0", "Reset the zoom"),
    ("Enter", "Confirm a typed value"),
    ("Esc", "Close the dialog on top"),
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
        u.push_str(&format!("&format={}", enc(k)));
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
    /// The Help menu.
    pub(super) fn help_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Help", |ui| {
            if ui.button("Keyboard shortcuts").clicked() {
                ui.close_menu();
                self.help = Some(Window::Shortcuts);
            }
            if WEB || cfg!(target_os = "android") {
                if ui.button("Supported games (website)").clicked() {
                    ui.close_menu();
                    open_url(ui.ctx(), SITE);
                }
            } else if ui.button("Supported games\u{2026}").on_hover_text("Every game in the list, with what can be edited and where its saves are").clicked() {
                ui.close_menu();
                let mut gp = self.picker();
                gp.only_found = Some(false);
                self.game_picker = Some(gp);
            }
            if ui.button("Website").on_hover_text(SITE).clicked() {
                ui.close_menu();
                open_url(ui.ctx(), SITE);
            }
            ui.separator();
            if ui.button("Report a problem\u{2026}").on_hover_text("Opens a form on GitHub. The version and system are filled in; your save is not sent.").clicked() {
                ui.close_menu();
                let kind = self.doc.as_ref().map(|d| friendly_kind(&d.layers, &d.content));
                open_url(ui.ctx(), &report_url(kind.as_deref()));
            }
            if ui.button("Request a game\u{2026}").on_hover_text("Opens a form on GitHub").clicked() {
                ui.close_menu();
                open_url(ui.ctx(), &request_url());
            }
            ui.separator();
            if ui.button("Licences and credits").clicked() {
                ui.close_menu();
                self.help = Some(Window::Licences);
            }
            if ui.button(format!("About {}", platform::DISPLAY_NAME)).clicked() {
                ui.close_menu();
                self.help = Some(Window::About);
            }
        });
    }

    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        let Some(which) = self.help else { return };
        let mut open = true;
        let title = match which {
            Window::Shortcuts => "Keyboard shortcuts".to_string(),
            Window::Licences => "Licences and credits".to_string(),
            Window::About => format!("About {}", platform::DISPLAY_NAME),
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
                            ui.label(*what);
                            ui.end_row();
                        }
                    });
                    ui.add_space(4.0);
                    ui.weak("On a Mac, use Cmd instead of Ctrl.");
                }
                Window::Licences => {
                    ui.label(format!("{} is free software under the GNU General Public License, version 3.", platform::DISPLAY_NAME));
                    ui.add_space(4.0);
                    ui.label("It builds on save-format research, tables and code from many other projects, each listed with its author and licence in the third-party notices.");
                    ui.add_space(6.0);
                    ui.hyperlink_to("GNU General Public License v3", format!("{REPO}/blob/main/LICENSE"));
                    ui.hyperlink_to("Third-party notices", format!("{REPO}/blob/main/THIRD_PARTY_NOTICES.md"));
                    ui.add_space(6.0);
                    ui.weak("The interface uses egui and eframe (MIT or Apache-2.0).");
                }
                Window::About => {
                    ui.label(RichText::new(platform::DISPLAY_NAME).family(theme::semibold_family()).size(20.0));
                    ui.label(format!("Version {} \u{00B7} {}", env!("CARGO_PKG_VERSION"), os()));
                    ui.add_space(4.0);
                    ui.label("Edit game saves safely. Everything stays on your device: nothing is uploaded, and every save is backed up before it is written.");
                    ui.add_space(6.0);
                    ui.hyperlink_to("Website", SITE);
                    ui.hyperlink_to("Source code and releases", REPO);
                    if WEB {
                        ui.add_space(6.0);
                        ui.weak("In the browser, Japanese, Chinese and Korean names may show as boxes. The downloadable app uses your computer's fonts for them.");
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
        egui::Window::new("Preferences")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .max_width(fit(ctx, 480.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("Look").strong());
                ui.horizontal(|ui| {
                    ui.label("Theme:");
                    if ui.radio(self.settings.dark, "Dark").clicked() && !self.settings.dark {
                        self.settings.dark = true;
                        theme::apply(ui.ctx(), true);
                        changed = true;
                    }
                    if ui.radio(!self.settings.dark, "Light").clicked() && self.settings.dark {
                        self.settings.dark = false;
                        theme::apply(ui.ctx(), false);
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Zoom:");
                    let mut z = ui.ctx().zoom_factor();
                    if ui.add(egui::Slider::new(&mut z, 0.5..=3.0).step_by(0.05).custom_formatter(|v, _| format!("{:.0}%", v * 100.0))).changed() {
                        ui.ctx().set_zoom_factor(z);
                    }
                    if ui.small_button("Reset").clicked() {
                        ui.ctx().set_zoom_factor(1.0);
                    }
                });
                changed |= ui.checkbox(&mut self.settings.show_advanced, "Show advanced values").changed();
                changed |= ui.checkbox(&mut self.settings.show_raw_keys, "Show raw key names").changed();
                changed |= ui.checkbox(&mut self.settings.hex_guide, "Show the find-a-value guide in the hex view").changed();
                ui.separator();
                ui.label(RichText::new("Saving").strong());
                changed |= ui
                    .checkbox(&mut self.settings.confirm_save, "Review changes before saving")
                    .on_hover_text("Show the list of changes, and warn when the game is running, before writing the save")
                    .changed();
                if !WEB {
                    ui.separator();
                    ui.label(RichText::new("Backups").strong());
                    ui.horizontal(|ui| {
                        ui.label("Automatic backups to keep per save:");
                        let mut n = self.settings.backup_keep;
                        if ui.add(egui::DragValue::new(&mut n).range(crate::backups::KEEP_MIN..=crate::backups::KEEP_MAX)).changed() {
                            self.settings.backup_keep = n;
                            crate::backups::set_keep(n);
                            changed = true;
                        }
                    });
                    ui.weak("A backup is taken every time this app writes a save. Named backups are always kept.");
                    ui.add_space(4.0);
                    let folder = match &self.settings.backup_dir {
                        Some(d) => d.display().to_string(),
                        None => format!("Next to each save, in a \u{201C}{}\u{201D} folder", crate::backups::BESIDE),
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Backups go:");
                        ui.add(egui::Label::new(RichText::new(folder).weak()).wrap());
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Choose a folder\u{2026}").clicked() {
                            pick_dir = true;
                        }
                        if self.settings.backup_dir.is_some() && ui.button("Back to the default").clicked() {
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
        assert!(u.contains("&format=Diablo%20II%20character"));
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
        for id in ["id: version", "id: platform", "id: format"] {
            assert!(t.contains(id), "{id}");
        }
    }
}
