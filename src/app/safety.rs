//! Keeping saves safe: noticing when something else wrote the open file (the game autosaved,
//! cloud sync ran) so older data is never put over it without asking, and showing failures
//! where they can't be missed.

use crate::i18n::{tr, trf};
use super::{file_name, modal, safe_choice, tab_ring, App, StatusKind};
use crate::formats::Content;
use crate::theme;
use eframe::egui::{self, Color32, RichText};
use serde_json::Value;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::time::SystemTime;

/// What a file looked like on disk: its size, when it was last written and, when the bytes
/// were at hand, a hash of them.
///
/// Size and time alone miss a rewrite of the same size within the file system's time step
/// (some keep only whole or even two seconds). The hash closes that gap without slowing big
/// files down: it is taken from the bytes already in memory (when the file is opened, or the
/// bytes just written), never by reading the file again. Only when a save is about to be
/// written and size and time still match is the file read once more and hashed
/// (`changed`); the check when the window gets the focus back stays on size and time
/// (`changed_quick`), so switching windows never reads a large save.
#[derive(Clone, Debug)]
pub struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    hash: Option<u64>,
}

/// Two stamps are the same file when size and time match, and the hashes too when both have one.
impl PartialEq for Stamp {
    fn eq(&self, o: &Self) -> bool {
        self.len == o.len && self.modified == o.modified && (self.hash.is_none() || o.hash.is_none() || self.hash == o.hash)
    }
}
impl Eq for Stamp {}

/// A fast hash of a file's bytes (FNV-1a over 8-byte words; not for security, only to tell
/// two versions of a save apart).
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01B3;
    let mut h: u64 = 0xCBF2_9CE4_8422_2325 ^ bytes.len() as u64;
    let mut words = bytes.chunks_exact(8);
    for w in &mut words {
        let mut a = [0u8; 8];
        a.copy_from_slice(w);
        h = (h ^ u64::from_le_bytes(a)).wrapping_mul(PRIME);
        h ^= h >> 29;
    }
    for &b in words.remainder() {
        h = (h ^ b as u64).wrapping_mul(PRIME);
    }
    h
}

/// The file's stamp now (`None`: it isn't there, or this is the browser version). Size and
/// time only.
pub fn stamp(path: &Path) -> Option<Stamp> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    let m = std::fs::metadata(path).ok()?;
    m.is_file().then(|| Stamp { len: m.len(), modified: m.modified().ok(), hash: None })
}

/// The stamp of a file whose bytes are `bytes` (just read from it, or just written to it).
/// Size and time are taken from `meta`, the file as it was before it was read; when the
/// bytes don't have that size the file changed while it was read, and the stamp says so
/// by not matching what is on disk.
pub fn stamp_with(meta: Option<Stamp>, bytes: &[u8]) -> Option<Stamp> {
    meta.map(|m| Stamp { hash: (m.len == bytes.len() as u64).then(|| hash_bytes(bytes)), ..m })
}

/// Whether the file at `path` is no longer the one stamped `was` (written again, or gone).
/// Files that weren't stamped (the browser version) never count as changed. When size and
/// time match and `was` has a hash, the file is read again and its hash compared.
pub fn changed(path: &Path, was: Option<&Stamp>) -> bool {
    let Some(w) = was else { return false };
    if changed_quick(path, Some(w)) {
        return true;
    }
    match w.hash {
        Some(h) => match std::fs::read(path) {
            Ok(b) => b.len() as u64 != w.len || hash_bytes(&b) != h,
            Err(_) => true,
        },
        None => false,
    }
}

/// `changed` on size and time only, without reading the file.
pub fn changed_quick(path: &Path, was: Option<&Stamp>) -> bool {
    match was {
        Some(w) => match stamp(path) {
            Some(now) => now.len != w.len || now.modified != w.modified,
            None => true,
        },
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
            self.ok(trf("Reloaded {0} from disk.", &[&file_name(&path)]));
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
            if changed_quick(&d.path, d.stamp.as_ref()) && (self.disk_seen.is_none() || stamp(&d.path) != self.disk_seen) {
                self.disk_changed = true;
            }
        }
    }

    /// Starts the new-version check: at startup when the player turned it on, or when asked for.
    pub(super) fn start_update_check(&mut self, manual: bool) {
        if !crate::update::AVAILABLE || self.update_rx.is_some() {
            return;
        }
        self.update_manual = manual;
        self.update_note = None;
        self.update_rx = Some(crate::update::start());
    }

    /// The question asked once, on the first launch: turn on the check for a new version?
    /// Yes is the focused default. The answer is saved, so it is never asked again.
    pub(super) fn update_question_dialog(&mut self, ctx: &egui::Context) {
        if !self.update_question {
            return;
        }
        let mut answer: Option<bool> = None;
        modal(ctx, tr("Check for new versions?"), |ui| {
            ui.set_max_width(440.0);
            ui.label(tr("Tell you when a new version is out? When the program starts it asks GitHub for the newest release. Nothing else is sent."));
            ui.add_space(4.0);
            ui.weak(tr("You can change this any time in Preferences."));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let yes = safe_choice(ui.add(egui::Button::new(RichText::new(tr("Yes, tell me")).color(Color32::WHITE)).fill(theme::ACCENT)));
                if yes.clicked() {
                    answer = Some(true);
                }
                let no = ui.button(tr("No thanks"));
                if no.clicked() {
                    answer = Some(false);
                }
                tab_ring(&yes, &no);
            });
        });
        if let Some(on) = answer {
            self.update_question = false;
            self.settings.check_updates = on;
            self.settings.update_question_answered = true;
            self.settings.save();
            if on {
                self.start_update_check(false);
            }
        }
    }

    /// Collects the answer of a new-version check. A check at startup says nothing unless a
    /// newer version that wasn't dismissed before turns up.
    pub(super) fn poll_update(&mut self, ctx: &egui::Context) {
        use crate::update::Outcome;
        if !self.update_started {
            self.update_started = true;
            if self.settings.check_updates {
                self.start_update_check(false);
            }
        }
        let Some(rx) = &self.update_rx else { return };
        match rx.try_recv() {
            Ok(outcome) => {
                self.update_rx = None;
                match outcome {
                    Outcome::Newer(v) => {
                        if self.update_manual || self.settings.dismissed_update.as_deref() != Some(v.as_str()) {
                            self.update_found = Some(v);
                        }
                    }
                    Outcome::UpToDate if self.update_manual => self.update_note = Some(tr("This is the newest version.")),
                    Outcome::Failed if self.update_manual => self.update_note = Some(tr("Couldn't check for a new version. Check your internet connection and try again.")),
                    _ => {}
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(300)),
            Err(_) => self.update_rx = None,
        }
    }

    /// Failures, and the "changed on disk" notice, under the toolbar.
    pub(super) fn banners(&mut self, ctx: &egui::Context) {
        if self.banner.is_none() && !self.disk_changed && self.update_found.is_none() && self.update_note.is_none() {
            return;
        }
        let mut dismiss_update = false;
        let mut dismiss = false;
        let mut reload = false;
        let mut keep = false;
        egui::TopBottomPanel::top("banner").show_separator_line(false).show(ctx, |ui| {
            ui.add_space(4.0);
            if let Some(v) = self.update_found.clone() {
                strip(ui, theme::WARN, |ui| {
                    let text = crate::i18n::trf("A new version is available: {0} (you have {1}).", &[&v, &env!("CARGO_PKG_VERSION")]);
                    ui.add(egui::Label::new(RichText::new(text).color(theme::fg(ui, theme::WARN))).wrap());
                    ui.hyperlink_to(tr("Download"), crate::update::release_url(&v));
                    if ui.button(tr("Dismiss")).on_hover_text(tr("Hide this until a newer version comes out.")).clicked() {
                        dismiss_update = true;
                    }
                });
            }
            if let Some(n) = self.update_note {
                strip(ui, theme::WARN, |ui| {
                    ui.add(egui::Label::new(RichText::new(n).color(theme::fg(ui, theme::WARN))).wrap());
                    if ui.button(tr("Dismiss")).clicked() {
                        self.update_note = None;
                    }
                });
            }
            if self.disk_changed {
                if let Some(d) = &self.doc {
                    let name = file_name(&d.path);
                    let gone = !d.path.exists();
                    strip(ui, theme::WARN, |ui| {
                        let what = if gone {
                            trf("{0} was moved or deleted since it was opened.", &[&name])
                        } else if self.dirty {
                            trf("{0} changed on disk (the game or a cloud sync may have saved). Saving will ask before writing over it.", &[&name])
                        } else {
                            trf("{0} changed on disk (the game or a cloud sync may have saved).", &[&name])
                        };
                        ui.add(egui::Label::new(RichText::new(what).color(theme::fg(ui, theme::WARN))).wrap());
                        if !gone && ui.button(if self.dirty { tr("Reload and drop my changes") } else { tr("Reload") }).clicked() {
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
        if dismiss_update {
            self.settings.dismissed_update = self.update_found.take();
            self.settings.save();
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

    /// "Restore this backup?" for Backups › Quick restore: Restore now / Cancel. Focus starts
    /// on Cancel; Cancel and Escape leave everything on disk as it is.
    pub(super) fn quick_restore_dialog(&mut self, ctx: &egui::Context) {
        let Some((path, from, what)) = self.quick_restore.clone() else { return };
        // Another file is in front now: the question was about one that isn't.
        if self.doc.as_ref().map(|d| &d.path) != Some(&path) {
            self.quick_restore = None;
            return;
        }
        let mut choice = None;
        modal(ctx, tr("Restore this backup?"), |ui| {
            ui.set_max_width(460.0);
            ui.label(trf("This replaces {0} on disk with {1}.", &[&file_name(&path), &what]));
            ui.add_space(4.0);
            ui.weak(tr("The save on disk now is backed up first. Backups \u{203A} Quick restore brings it back."));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let restore = ui.button(tr("Restore now"));
                if restore.clicked() {
                    choice = Some(true);
                }
                let cancel = safe_choice(ui.button(tr("Cancel")));
                if cancel.clicked() {
                    choice = Some(false);
                }
                tab_ring(&restore, &cancel);
            });
        });
        match choice {
            Some(true) => {
                self.quick_restore = None;
                self.restore_from(&path, &from, &what);
            }
            Some(false) => self.quick_restore = None,
            None => {}
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
                ui.label(trf("{0} changed on disk after you opened it. Most likely the game saved, or a cloud sync brought in another copy.", &[&name]));
                ui.add_space(4.0);
                ui.label(tr("Saving now would put the version you opened, with your changes, over that newer file."));
            } else {
                ui.label(trf("{0} was moved or deleted after you opened it. Saving puts your version back in its place.", &[&name]));
            }
            ui.weak(tr("Either way, whatever is on disk now is backed up before anything is written."));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let reload = path.exists().then(|| ui.add(egui::Button::new(RichText::new(tr("Reload the newer file")).color(Color32::WHITE)).fill(theme::ACCENT)).on_hover_text(tr("Open what's on disk now. Your unsaved changes are dropped.")));
                if reload.as_ref().is_some_and(|r| r.clicked()) {
                    choice = Some(0);
                }
                let save = ui.button(tr("Save anyway"));
                if save.clicked() {
                    choice = Some(1);
                }
                let cancel = safe_choice(ui.button(tr("Cancel")));
                if cancel.clicked() {
                    choice = Some(2);
                }
                tab_ring(reload.as_ref().unwrap_or(&save), &cancel);
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
    fn same_size_same_time_rewrite_is_stale() {
        let d = dir("same-size");
        let p = d.join("save.dat");
        std::fs::write(&p, b"gold=100").unwrap();
        let s = stamp_with(stamp(&p), b"gold=100");
        assert!(s.as_ref().is_some_and(|s| s.hash.is_some()));
        assert!(!changed(&p, s.as_ref()));
        let when = std::fs::metadata(&p).unwrap().modified().unwrap();
        // Another program writes as many bytes and the time stays the same.
        std::fs::write(&p, b"gold=999").unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(&p).unwrap();
        f.set_modified(when).unwrap();
        drop(f);
        assert!(!changed_quick(&p, s.as_ref()), "size and time alone can't tell");
        assert!(changed(&p, s.as_ref()), "the hash does");
        // Put back byte for byte: not a change.
        std::fs::write(&p, b"gold=100").unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(&p).unwrap();
        f.set_modified(when).unwrap();
        drop(f);
        assert!(!changed(&p, s.as_ref()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn stamp_of_bytes_that_dont_match_the_size_has_no_hash() {
        let d = dir("short-read");
        let p = d.join("save.dat");
        std::fs::write(&p, b"gold=100").unwrap();
        // The file grew between the stamp and the read.
        let s = stamp_with(stamp(&p), b"gold=100000");
        assert!(s.as_ref().is_some_and(|s| s.hash.is_none()));
        assert_eq!(s, stamp(&p), "a stamp without a hash still compares by size and time");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn hash_tells_close_bytes_apart() {
        assert_ne!(hash_bytes(b"gold=100"), hash_bytes(b"gold=101"));
        assert_ne!(hash_bytes(b""), hash_bytes(b"\0"));
        assert_ne!(hash_bytes(&[0u8; 16]), hash_bytes(&[0u8; 17]));
        assert_eq!(hash_bytes(b"same bytes here!"), hash_bytes(b"same bytes here!"));
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
