//! A simple built-in file browser, for platforms without native file dialogs (Android).

// Desktop uses native dialogs; this is only shown on Android.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use eframe::egui::{self, RichText, ScrollArea};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    OpenFile,
    Folder,
    SaveFile,
}

pub struct FileBrowser {
    mode: Mode,
    title: String,
    dir: PathBuf,
    name: String,
    /// (name, is folder, size)
    entries: Vec<(String, bool, u64)>,
    error: Option<String>,
    places: Vec<(String, PathBuf)>,
    /// Android: whether file access was granted, re-checked now and then (the user grants it
    /// in the system settings and comes back).
    access: Option<(bool, f64)>,
}

/// What the browser was closed with.
pub enum Outcome {
    Picked(PathBuf),
    Cancelled,
}

fn places() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    if cfg!(target_os = "android") {
        let internal = PathBuf::from("/storage/emulated/0");
        out.push(("Phone storage".to_string(), internal.clone()));
        for (label, sub) in [
            ("Downloads", "Download"),
            ("RetroArch saves", "RetroArch/saves"),
            ("Documents", "Documents"),
        ] {
            let p = internal.join(sub);
            if p.is_dir() {
                out.push((label.to_string(), p));
            }
        }
        // SD cards and USB drives show up as /storage/XXXX-XXXX.
        if let Ok(rd) = std::fs::read_dir("/storage") {
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n != "emulated" && n != "self" && e.path().is_dir() {
                    out.push((format!("SD card ({n})"), e.path()));
                }
            }
        }
    } else if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from) {
        out.push(("Home".to_string(), home));
    }
    out
}

impl FileBrowser {
    pub fn new(mode: Mode, title: impl Into<String>, start: Option<PathBuf>, name: Option<String>) -> Self {
        let places = places();
        let dir = start
            .filter(|d| d.is_dir())
            .or_else(|| places.first().map(|p| p.1.clone()))
            .unwrap_or_else(|| PathBuf::from("/"));
        let mut b = Self {
            mode,
            title: title.into(),
            dir,
            name: name.unwrap_or_default(),
            entries: vec![],
            error: None,
            places,
            access: None,
        };
        b.refresh();
        b
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;
        match std::fs::read_dir(&self.dir) {
            Ok(rd) => {
                for e in rd.flatten() {
                    let md = e.metadata().ok();
                    let is_dir = md.as_ref().is_some_and(|m| m.is_dir());
                    if self.mode == Mode::Folder && !is_dir {
                        continue;
                    }
                    self.entries
                        .push((e.file_name().to_string_lossy().into_owned(), is_dir, md.map_or(0, |m| m.len())));
                }
                self.entries
                    .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
            }
            Err(e) => self.error = Some(format!("Can't open this folder: {e}")),
        }
    }

    fn go(&mut self, dir: PathBuf) {
        self.dir = dir;
        self.refresh();
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<Outcome> {
        let mut out = None;
        let screen = ctx.screen_rect().shrink(8.0);
        egui::Window::new(self.title.clone())
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .fixed_rect(screen)
            .show(ctx, |ui| {
                ui.spacing_mut().interact_size.y = 36.0;
                #[cfg(target_os = "android")]
                {
                    let now = ui.input(|i| i.time);
                    let stale = self.access.map_or(true, |(_, t)| now - t > 1.0);
                    if stale {
                        let ok = crate::platform::android::has_file_access();
                        if self.access.is_some_and(|(was, _)| !was && ok) {
                            self.refresh();
                        }
                        self.access = Some((ok, now));
                    }
                    if self.access.is_some_and(|(ok, _)| !ok) {
                        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
                        ui.group(|ui| {
                            ui.label("Allow \u{201C}All files access\u{201D} so the editor can open and save game saves on your phone.");
                            if ui.button("Allow access\u{2026}").clicked() {
                                crate::platform::android::request_file_access();
                            }
                        });
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    let mut target = None;
                    for (label, p) in &self.places {
                        if ui.selectable_label(self.dir.starts_with(p), label).clicked() {
                            target = Some(p.clone());
                        }
                    }
                    if let Some(p) = target {
                        self.go(p);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.dir.parent().is_some(), egui::Button::new("Up")).clicked() {
                        if let Some(p) = self.dir.parent().map(Path::to_path_buf) {
                            self.go(p);
                        }
                    }
                    ui.label(RichText::new(self.dir.display().to_string()).monospace());
                });
                ui.separator();

                // Footer first so the list takes the remaining space.
                egui::TopBottomPanel::bottom("browser_footer").show_inside(ui, |ui| {
                    ui.add_space(4.0);
                    if self.mode == Mode::SaveFile {
                        ui.horizontal(|ui| {
                            ui.label("File name:");
                            ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY));
                        });
                    }
                    ui.horizontal(|ui| {
                        let (label, ok) = match self.mode {
                            Mode::OpenFile => ("", false),
                            Mode::Folder => ("Use this folder", true),
                            Mode::SaveFile => ("Save here", !self.name.trim().is_empty()),
                        };
                        if !label.is_empty() && ui.add_enabled(ok, egui::Button::new(label)).clicked() {
                            out = Some(Outcome::Picked(match self.mode {
                                Mode::SaveFile => self.dir.join(self.name.trim()),
                                _ => self.dir.clone(),
                            }));
                        }
                        if ui.button("Cancel").clicked() {
                            out = Some(Outcome::Cancelled);
                        }
                    });
                });

                if let Some(e) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                let mut open_dir = None;
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                        if self.entries.is_empty() && self.error.is_none() {
                            ui.weak("(empty folder)");
                        }
                        for (name, is_dir, size) in &self.entries {
                            let text = if *is_dir {
                                RichText::new(format!("{name}/")).strong()
                            } else {
                                RichText::new(format!("{name}    {}", crate::app::human_size(*size as usize)))
                            };
                            let r = ui.add(egui::SelectableLabel::new(self.name == *name, text));
                            if r.clicked() {
                                if *is_dir {
                                    open_dir = Some(self.dir.join(name));
                                } else if self.mode == Mode::SaveFile {
                                    self.name = name.clone();
                                } else {
                                    out = Some(Outcome::Picked(self.dir.join(name)));
                                }
                            }
                        }
                    });
                });
                if let Some(d) = open_dir {
                    self.go(d);
                }
            });
        out
    }
}
