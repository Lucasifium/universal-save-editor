//! Keeping saves safe: noticing when something else wrote the open file (the game autosaved,
//! cloud sync ran) so older data is never put over it without asking, and showing failures
//! where they can't be missed.

use crate::i18n::tr;
use super::{file_name, modal, App, StatusKind};
use crate::formats::Content;
use crate::theme;
use eframe::egui::{self, Color32, RichText};
use serde_json::Value;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::time::SystemTime;

/// What a file looked like on disk: its size and when it was last written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
}

/// The file's stamp now (`None`: it isn't there, or this is the browser version).
pub fn stamp(path: &Path) -> Option<Stamp> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let m = std::fs::metadata(path).ok()?;
    m.is_file().then(|| Stamp { len: m.len(), modified: m.modified().ok() })
}

/// Whether the file at `path` is no longer the one stamped `was` (written again, or gone).
/// Files that weren't stamped (the browser version) never count as changed.
pub fn changed(path: &Path, was: Option<&Stamp>) -> bool {
    match was {
        Some(w) => stamp(path).as_ref() != Some(w),
        None => false,
    }
}

/// Whether any text or name in the save needs the Japanese, Chinese or Korean fonts.
pub fn content_has_cjk(c: &Content) -> bool {
    fn walk(v: &Value) -> bool {
        match v {
            Value::String(s) => theme::has_cjk(s),
            Value::Array(a) => a.iter().any(walk),
            Value::Object(m) => m.iter().any(|(k, v)| theme::has_cjk(k) || walk(v)),
            _ => false,
        }
    }
    match c {
        Content::Json { value, .. } => walk(value),
        Content::Tree(t) => walk(&t.view),
        Content::Text(t) => theme::has_cjk(t),
        _ => false,
    }
}

impl App {
    /// Reads the open file again, dropping unsaved changes (they can't apply to newer data).
    pub(super) fn reload_from_disk(&mut self) {
        let Some(d) = &self.doc else { return };
        let (path, opts) = (d.path.clone(), d.opts.clone());
        self.dirty = false;
        self.stale_save = None;
        self.disk_changed = false;
        self.load_with(&path, opts);
        if self.status_kind != StatusKind::Err {
            self.ok(format!("Reloaded {} from disk.", file_name(&path)));
        }
    }

    /// When the window gets the focus back (the player was in the game), see whether the
    /// open file was written meanwhile.
    pub(super) fn check_disk(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.focused);
        let back = focused && !self.focused;
        self.focused = focused;
        if !back || self.disk_changed {
            return;
        }
        if let Some(d) = &self.doc {
            if changed(&d.path, d.stamp.as_ref()) && (self.disk_seen.is_none() || stamp(&d.path) != self.disk_seen) {
                self.disk_changed = true;
            }
        }
    }

    /// Failures, and the "changed on disk" notice, under the toolbar.
    pub(super) fn banners(&mut self, ctx: &egui::Context) {
        if self.banner.is_none() && !self.disk_changed {
            return;
        }
        let mut dismiss = false;
        let mut reload = false;
        let mut keep = false;
        egui::TopBottomPanel::top("banner").show_separator_line(false).show(ctx, |ui| {
            ui.add_space(4.0);
            if self.disk_changed {
                if let Some(d) = &self.doc {
                    let name = file_name(&d.path);
                    let gone = !d.path.exists();
                    strip(ui, theme::WARN, |ui| {
                        let what = if gone {
                            format!("{name} was moved or deleted since it was opened.")
                        } else if self.dirty {
                            format!("{name} changed on disk (the game or a cloud sync may have saved). Saving will ask before writing over it.")
                        } else {
                            format!("{name} changed on disk (the game or a cloud sync may have saved).")
                        };
                        ui.add(egui::Label::new(RichText::new(what).color(theme::fg(ui, theme::WARN))).wrap());
                        if !gone && ui.button(if self.dirty { "Reload (drop my changes)" } else { "Reload" }).clicked() {
                            reload = true;
                        }
                        if ui.button(tr("Keep what's shown")).on_hover_text(tr("Hide this. Saving still asks before writing over the newer file.")).clicked() {
                            keep = true;
                        }
                    });
                }
            }
            if let Some(msg) = &self.banner {
                strip(ui, theme::ERR, |ui| {
                    ui.add(egui::Label::new(RichText::new(msg).color(theme::fg(ui, theme::ERR))).wrap());
                    if ui.button(tr("Dismiss")).clicked() {
                        dismiss = true;
                    }
                });
            }
            ui.add_space(2.0);
        });
        if dismiss {
            self.banner = None;
        }
        if keep {
            // Hidden until the file changes again; the check when saving still runs.
            self.disk_changed = false;
            self.disk_seen = self.doc.as_ref().and_then(|d| stamp(&d.path));
        }
        if reload {
            self.reload_from_disk();
        }
    }

    /// "This file changed on disk since you opened it": Reload / Save anyway / Cancel.
    pub(super) fn stale_dialog(&mut self, ctx: &egui::Context) {
        let Some(path) = self.stale_save.clone() else { return };
        let mut choice = None;
        modal(ctx, tr("The save changed on disk"), |ui| {
            ui.set_max_width(460.0);
            let name = file_name(&path);
            if path.exists() {
                ui.label(format!("{name} was written by something else after you opened it \u{2014} most likely the game saved, or a cloud sync brought in another copy."));
                ui.add_space(4.0);
                ui.label(tr("Saving now would put the version you opened, with your changes, over that newer file."));
            } else {
                ui.label(format!("{name} was moved or deleted after you opened it. Saving puts your version back in its place."));
            }
            ui.weak(tr("Either way, whatever is on disk now is backed up before anything is written."));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if path.exists() && ui.add(egui::Button::new(RichText::new(tr("Reload the newer file")).color(Color32::WHITE)).fill(theme::ACCENT)).on_hover_text(tr("Open what's on disk now. Your unsaved changes are dropped.")).clicked() {
                    choice = Some(0);
                }
                if ui.button(tr("Save anyway")).clicked() {
                    choice = Some(1);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(2);
                }
            });
        });
        match choice {
            Some(0) => self.reload_from_disk(),
            Some(1) => {
                self.stale_save = None;
                self.stale_ok = true;
                self.save_to(&path);
                self.stale_ok = false;
            }
            Some(_) => {
                self.stale_save = None;
                self.info(tr("Not saved. Your changes are still here."));
            }
            None => {}
        }
    }

}

/// A coloured strip across the window with a message and buttons.
fn strip(ui: &mut egui::Ui, color: Color32, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(color.gamma_multiply(0.16))
        .stroke(egui::Stroke::new(1.0_f32, color.gamma_multiply(0.6)))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(add);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("use-stale-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn unchanged_file_is_not_stale() {
        let d = dir("same");
        let p = d.join("save.dat");
        std::fs::write(&p, b"gold=100").unwrap();
        let s = stamp(&p);
        assert!(s.is_some());
        assert!(!changed(&p, s.as_ref()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rewritten_file_is_stale() {
        let d = dir("rewritten");
        let p = d.join("save.dat");
        std::fs::write(&p, b"gold=100").unwrap();
        let s = stamp(&p);
        // The game saves again: other size.
        std::fs::write(&p, b"gold=100000").unwrap();
        assert!(changed(&p, s.as_ref()));
        // Same size, later time.
        std::fs::write(&p, b"gold=100").unwrap();
        let s = stamp(&p);
        let f = std::fs::OpenOptions::new().write(true).open(&p).unwrap();
        f.set_modified(SystemTime::now() + std::time::Duration::from_secs(5)).unwrap();
        drop(f);
        assert!(changed(&p, s.as_ref()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn deleted_file_is_stale_and_unstamped_never_is() {
        let d = dir("deleted");
        let p = d.join("save.dat");
        std::fs::write(&p, b"x").unwrap();
        let s = stamp(&p);
        std::fs::remove_file(&p).unwrap();
        assert!(changed(&p, s.as_ref()));
        assert!(!changed(&p, None));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn finds_cjk_text() {
        let v: Value = serde_json::json!({"party": [{"name": "Hero"}, {"name": "\u{30AF}\u{30E9}\u{30A6}\u{30C9}"}]});
        assert!(content_has_cjk(&Content::Json { value: v, pretty: true }));
        let v: Value = serde_json::json!({"name": "Cloud", "gil": 100});
        assert!(!content_has_cjk(&Content::Json { value: v, pretty: true }));
        assert!(theme::has_cjk("\u{D55C}\u{AE00}"));
        assert!(!theme::has_cjk("Caf\u{E9} \u{2192} na\u{EF}ve"));
    }
}
