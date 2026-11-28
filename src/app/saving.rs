//! Saving: building the file, checking it reads back, writing it (on a worker thread for
//! big saves), and restoring, exporting and importing.

use super::*;

/// Saves at least this big are written on a worker thread, so the window keeps responding:
/// a file on disk this size (like `BACKGROUND_LOAD`), or this many values in the Values list
/// (a small packed file can hold a great many).
pub(super) const BACKGROUND_SAVE: u64 = BACKGROUND_LOAD;
pub(super) const BACKGROUND_SAVE_ROWS: usize = 20_000;

/// How long a save may take before the "Saving" box is shown.
pub(super) const SAVING_BOX_AFTER: f64 = OPENING_BOX_AFTER;

/// What to do once a save has gone through (nothing, when it failed: the changes stay).
#[derive(Clone, Debug, PartialEq)]
pub(super) enum AfterSave {
    Stay,
    CloseFile,
    Open(PathBuf),
    Quit,
}

/// What `write_save` wrote: the file read back, its size, a note for the player and the
/// written file's stamp.
pub(super) struct Written {
    pub(super) check: formats::Decoded,
    pub(super) len: usize,
    pub(super) note: Option<String>,
    pub(super) stamp: Option<Stamp>,
    /// Browser version: the file wasn't written; the edited file was downloaded instead, and
    /// `note` says why. The message is the note alone, not "Saved".
    pub(super) instead: bool,
}

/// A save being written on a worker thread. While it runs nothing on screen takes input, and
/// no other save, open, close or tab switch can start.
pub(super) struct Saving {
    pub(super) path: PathBuf,
    pub(super) then: AfterSave,
    /// `edit_gen` and the tab showing when it started. Anything else when it ends means the
    /// file on screen changed meanwhile (which input being held should rule out).
    pub(super) gen: (u64, usize),
    pub(super) started: f64,
    pub(super) rx: std::sync::mpsc::Receiver<Result<Written, String>>,
}

/// What to tell the player when a file the game checks against the save couldn't be written.
pub(super) fn companion_failed(e: &crate::backups::CompanionError) -> String {
    if e.stuck.is_empty() {
        return trf("Couldn't write {0}, a file the game checks against the save ({1}). The save and its files were put back as they were, so nothing changed.", &[&e.file.display(), &e.error]);
    }
    let stuck: Vec<String> = e.stuck.iter().map(|p| file_name(p)).collect();
    trf("Couldn't write {0}, a file the game checks against the save ({1}). These files couldn't be put back as they were and may not match each other now: {2}. Their earlier versions are in the backups folder.", &[&e.file.display(), &e.error, &stuck.join(", ")])
}

/// Builds the file for `content` and checks that it reads back as exactly that; nothing is
/// written yet. Returns the file and what it reads back as.
pub(super) fn build_save(layers: &[Layer], content: &Content, opts: &formats::Options) -> Result<(Vec<u8>, formats::Decoded), String> {
    let bytes = formats::encode(layers, content).map_err(|e| trf("Couldn't save: the new file couldn't be built ({0}). The save on disk wasn't changed.", &[&e]))?;
    // Verify before touching the disk: the new file must decode to exactly what's on screen.
    let check = formats::decode_with(&bytes, opts);
    if !formats::same_layers(&check.layers, layers) || !formats::verified(&check.content, content) {
        return Err(tr("Couldn't save: the new file didn't read back exactly as edited, so nothing was written. The save on disk wasn't changed. Please report this with Help \u{203A} Report a problem.").into());
    }
    Ok((bytes, check))
}

/// Builds the file for `content`, checks that it reads back as exactly that, keeps a backup of
/// the file on disk and writes the new one. Touches nothing but files, so it runs on the UI
/// thread or a worker thread alike. Every failure says what happened, in the player's language.
pub(super) fn write_save(path: &Path, layers: &[Layer], content: &Content, opts: &formats::Options) -> Result<Written, String> {
    let (bytes, check) = build_save(layers, content, opts)?;

    let mut note: Option<String> = None;
    if WEB {
        write_in_place(path, &bytes).map_err(|e| trf("Couldn't hand over {0}: {1}", &[&path.display(), &e]))?;
        for (other, data) in formats::companion_files(path, &bytes) {
            let _ = write_in_place(&other, &data);
        }
        note = Some(tr("Your browser downloaded the edited file. Keep a copy of the old save, then put this one in its place.").into());
    } else {
        if path.exists() {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".bak");
            let bak = PathBuf::from(bak);
            if !bak.exists() {
                // Written whole or not at all: a copy cut off half way (a full disk) would
                // otherwise stay as the "original" for good, as it is never made again.
                crate::vfs::read(path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| crate::backups::write_verified(&bak, &b, false))
                    .map_err(|e| trf("Couldn't save: the backup {0} couldn't be made ({1}). The save on disk wasn't changed.", &[&bak.display(), &e]))?;
                note = Some(trf("Original backed up as {0}.", &[&file_name(&bak)]));
            }
            // Every earlier version stays restorable from Backups \u{203A} Quick restore.
            crate::backups::take(path).map_err(|e| trf("Couldn't save: the current save couldn't be backed up first ({0}). The save on disk wasn't changed.", &[&e]))?;
        }
        // Files the game checks against this save (Persona 4 Golden's .binslot).
        let companions = formats::companion_files(path, &bytes);
        // What to put back if one of them can't be written.
        let before = if companions.is_empty() { None } else { Some(crate::backups::Kept::read(path).map_err(|e| trf("Couldn't save: {0} couldn't be read first ({1}). The save on disk wasn't changed.", &[&path.display(), &e]))?) };
        write_in_place(path, &bytes).map_err(|e| trf("Couldn't write {0}: {1}. The file on disk wasn't changed.", &[&path.display(), &e]))?;
        if let Some(before) = before {
            crate::backups::write_companions(&before, companions, true).map_err(|e| companion_failed(&e))?;
        }
        #[cfg(not(target_arch = "wasm32"))]
        match crate::playerprefs::after_save(path) {
            Ok(Some(m)) => {
                note = Some(match note {
                    Some(n) => format!("{n} {m}"),
                    None => m,
                })
            }
            Err(e) => return Err(trf("Saved the file, but {0}", &[&e])),
            Ok(None) => {}
        }
    }
    Ok(Written { stamp: safety::stamp_with(safety::stamp(path), &bytes), check, len: bytes.len(), note, instead: false })
}

/// Whether the Values list built from `new` is the one built from `old`, for the same names.
/// The list (see `values_view::build`) depends on the values only through their shape (keys
/// and their order, array lengths, which values are objects, arrays, null or other), on the
/// short name each object takes from a field (`names::object_summary`) and, for a tree, on
/// which values are editable; so only scalar values may differ.
pub(super) fn same_values_list(old: &Content, new: &Content) -> bool {
    fn same(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Object(x), Value::Object(y)) => {
                x.len() == y.len()
                    && names::object_summary(a) == names::object_summary(b)
                    && x.iter().zip(y).all(|((ka, va), (kb, vb))| ka == kb && same(va, vb))
            }
            (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(va, vb)| same(va, vb)),
            (Value::Null, Value::Null) => true,
            (Value::Null, _) | (_, Value::Null) | (Value::Object(_) | Value::Array(_), _) | (_, Value::Object(_) | Value::Array(_)) => false,
            _ => true,
        }
    }
    match (old, new) {
        (Content::Json { value: a, .. }, Content::Json { value: b, .. }) => same(a, b),
        (Content::Tree(a), Content::Tree(b)) => a.editable == b.editable && same(&a.view, &b.view),
        _ => false,
    }
}

/// Replaces the file at `path` with `bytes`. On a computer: through a temporary file swapped
/// in, so a crash can never leave a half-written save. In the browser: as a download.
pub(super) fn write_in_place(path: &Path, bytes: &[u8]) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::web::download(&file_name(path), bytes)?;
        crate::vfs::write(path, bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Written beside it, flushed, read back and checked, and only then renamed over it.
        crate::backups::write_verified(path, bytes, true)
    }
}

impl App {
    pub(super) fn save_to(&mut self, path: &Path) {
        self.save_to_then(path, AfterSave::Stay);
    }

    /// Saves the open file as `path`, then does `then` if that went through.
    pub(super) fn save_to_then(&mut self, path: &Path, then: AfterSave) {
        let background = self.save_in_background();
        self.save_with(path, then, background);
    }

    /// Big files are saved on a worker thread (see `BACKGROUND_SAVE`).
    pub(super) fn save_in_background(&self) -> bool {
        let Some(d) = &self.doc else { return false };
        let size = match &d.content {
            Content::Binary(b) => b.len() as u64,
            Content::Text(t) => t.len() as u64,
            _ => 0,
        };
        // The browser version has no worker threads.
        !WEB && (size.max(crate::vfs::len(&d.path).unwrap_or(0)) >= BACKGROUND_SAVE || self.model.rows.len() >= BACKGROUND_SAVE_ROWS)
    }

    /// Checks that the file can be saved, then writes it here or on a worker thread. Either way
    /// the same `write_save` does the work and `finish_save` takes in its outcome.
    pub(super) fn save_with(&mut self, path: &Path, then: AfterSave, background: bool) {
        // Browser: "Save a copy as" downloads even a file that can be saved in place.
        #[cfg(target_arch = "wasm32")]
        let download = crate::web::take_download_next();
        // A file being opened (or read again) is not the one on screen yet: saving now would
        // write the old one and leave the new one with the wrong idea of what is on disk.
        if self.saving.is_some() || self.loading.is_some() {
            return;
        }
        if !self.commit_raw() {
            return self.fail(tr("Not saved: the Raw JSON text has an error. Fix it on the Raw JSON tab, then save."));
        }
        let Some(doc) = &self.doc else { return };
        // Something else wrote the file since it was read: don't silently put older data over it.
        if !std::mem::take(&mut self.stale_ok) && doc.path == path && safety::changed(&doc.path, doc.stamp.as_ref()) {
            self.stale_save = Some((path.to_path_buf(), then));
            return;
        }
        // Only this document's own handle, and only for its own file.
        #[cfg(target_arch = "wasm32")]
        if let Some(id) = doc.handle.as_ref().map(|h| h.0).filter(|_| !download && doc.path == path) {
            return self.save_in_place(path, id, then, false);
        }
        if background && !WEB {
            // The worker gets its own copy: what is on screen can't change under it.
            let (p, layers, content, opts) = (path.to_path_buf(), doc.layers.clone(), doc.content.clone(), doc.opts.clone());
            return self.spawn_save(path, then, move || write_save(&p, &layers, &content, &opts));
        }
        let r = write_save(path, &doc.layers, &doc.content, &doc.opts);
        self.finish_save(path, r, false);
        self.after_save(then);
    }

    /// Runs `work` on a worker thread; `poll_saving` takes in what it hands back.
    pub(super) fn spawn_save(&mut self, path: &Path, then: AfterSave, work: impl FnOnce() -> Result<Written, String> + Send + 'static) {
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new().name("save".into()).spawn(move || {
            let _ = tx.send(work());
        });
        if spawned.is_err() {
            // No thread to be had: save here instead.
            return self.save_with(path, then, false);
        }
        self.saving = Some(Saving { path: path.to_path_buf(), then, gen: (self.edit_gen, self.tabs.active), started: f64::NAN, rx });
        crate::widgets::set_busy(true);
        self.info(trf("Saving {0}\u{2026}", &[&file_name(path)]));
    }

    /// The worker thread's outcome, once it's there. Until then the window shows that it is
    /// saving and takes no input.
    pub(super) fn poll_saving(&mut self, ctx: &egui::Context, now: f64) {
        let Some(s) = &mut self.saving else { return };
        if s.started.is_nan() {
            s.started = now;
            // Nothing typed from here on may land in the file being saved.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        match s.rx.try_recv() {
            Ok(r) => {
                let s = self.saving.take().unwrap();
                let meanwhile = s.gen != (self.edit_gen, self.tabs.active);
                self.finish_save(&s.path, r, meanwhile);
                self.after_save(s.then);
                self.closing_after_save();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                let name = file_name(&s.path);
                let secs = now - s.started;
                // Clicks must not reach the file being saved, before the box and under it.
                let screen = ctx.screen_rect();
                egui::Area::new(egui::Id::new("saving_hold")).fixed_pos(screen.min).sense(widgets::BLOCK_CLICKS).order(egui::Order::Foreground).show(ctx, |ui| {
                    ui.allocate_response(screen.size(), widgets::BLOCK_CLICKS);
                });
                if secs < SAVING_BOX_AFTER {
                    ctx.request_repaint_after(std::time::Duration::from_millis(30));
                    return;
                }
                modal(ctx, tr("Saving the file"), |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(trf("Writing {0}\u{2026}", &[&name]));
                    });
                    if secs > 2.0 {
                        ui.weak(trf("{0} s. Big saves take a moment.", &[&format!("{secs:.0}")]));
                    }
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(_) => {
                // The worker stopped without an answer (it panicked). Whether or not it got as far
                // as writing, the changes stay unsaved, and the next save asks first if the file
                // on disk is no longer the one opened.
                self.saving = None;
                self.fail(tr("Saving stopped unexpectedly. Your changes are still here but not saved. Save again."));
                self.closing_after_save();
            }
        }
    }

    /// The window was asked to close during a save: close now, or ask about unsaved changes.
    pub(super) fn closing_after_save(&mut self) {
        if !std::mem::take(&mut self.close_after_save) {
            return;
        }
        if self.any_dirty() && !self.allow_close {
            self.show_first_dirty();
            self.confirm_close = true;
        } else {
            self.allow_close = true;
            self.close_now = true;
        }
    }

    /// Takes in a save's outcome, on the UI thread. `meanwhile`: the file on screen changed
    /// after the save started, so what was written is not what is shown.
    pub(super) fn finish_save(&mut self, path: &Path, r: Result<Written, String>, meanwhile: bool) {
        let w = match r {
            Ok(w) => w,
            Err(e) => return self.fail(e),
        };
        let Written { check, len, note, stamp, instead } = w;
        let Some(d) = self.doc.as_mut() else { return };
        if d.path != path {
            // Saved under another name: later saves go there, not back to the file it came from.
            d.handle = None;
        }
        d.path = path.to_path_buf();
        d.stamp = stamp;
        self.disk_changed = false;
        self.disk_seen = None;
        self.settings.push_recent(path);
        if meanwhile {
            // The file holds what was shown when the save started; what is shown now stays as
            // it is and counts as unsaved. Changes are listed, and reverted, against the file
            // as it now is.
            d.baseline = check.content;
            self.model_from_baseline = false;
            self.dirty = true;
            return self.fail(tr("Saved, but changes made while saving aren't in the file yet. Save again to keep them."));
        }
        let signed = check.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. }));
        let persistent = check.layers.iter().any(|l| matches!(l, Layer::RenpyPersistent { .. }));
        // The Values list is built again only when it could come out differently.
        let keep_model = self.model_from_baseline && same_values_list(&d.baseline, &check.content);
        d.layers = check.layers;
        d.content = check.content;
        d.baseline = d.content.clone();
        self.touched.clear();
        if !keep_model {
            self.rebuild_model();
        }
        self.model_from_baseline = true;
        self.edit_gen += 1;
        self.recompute_dirty();
        let extra = if signed && crate::renpy_sign::can_sign() {
            Some(tr("Re-signed with your Ren'Py key, so the game will load it normally."))
        } else if signed {
            Some(tr("Your Ren'Py key wasn't found, so the old signature was removed. The game asks once whether to load the save: choose Yes."))
        } else if persistent {
            Some(tr("Re-signed with your Ren'Py key. If the game is running, quit it without saving first, because it rewrites this file when it quits."))
        } else {
            None
        };
        let cloud = crate::cloud::service_for(path).map(|c| trf("If {0} asks which save to keep, choose the local one.", &[&c]));
        let mut msg = if instead { vec![] } else { vec![trf("Saved and verified ({0}).", &[&human_size(len)])] };
        msg.extend(note);
        msg.extend(extra.map(String::from));
        msg.extend(cloud.map(String::from));
        if instead {
            self.info(msg.join(" "));
        } else {
            self.ok(msg.join(" "));
        }
    }

    /// After a save: close the file, open the next one or quit, if it went through.
    pub(super) fn after_save(&mut self, then: AfterSave) {
        if self.dirty {
            return;
        }
        match then {
            AfterSave::Stay => {}
            AfterSave::CloseFile => self.close_file(),
            AfterSave::Open(p) => self.load_async(&p),
            AfterSave::Quit => {
                if self.any_dirty() {
                    // Another tab still has unsaved changes: show it and ask again.
                    self.show_first_dirty();
                    self.confirm_close = true;
                } else {
                    self.allow_close = true;
                    self.close_now = true;
                }
            }
        }
    }

    pub(super) fn save_then(&mut self, then: AfterSave) {
        if let Some(p) = self.doc.as_ref().map(|d| d.path.clone()) {
            self.save_to_then(&p, then);
        }
    }

    /// Save from the button / Ctrl+S: first show what changed and warn if the game is open.
    pub(super) fn save_clicked(&mut self) {
        // Not while a save is being written, nor while the file is being read again.
        if self.saving.is_some() || self.loading.is_some() {
            return;
        }
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        if !self.commit_raw() {
            return self.fail(tr("Not saved: the Raw JSON text has an error. Fix it on the Raw JSON tab, then save."));
        }
        let running = crate::running::game_for(&path);
        if running.is_none() && !self.settings.confirm_save {
            return self.save_to(&path);
        }
        self.running = running.clone();
        self.save_review = Some(SaveReview { path, running, lines: self.review_lines() });
        crate::widgets::set_busy(true);
    }

    pub(super) fn save_as(&mut self) {
        if self.saving.is_some() {
            return;
        }
        let Some(doc) = &self.doc else { return };
        let name = file_name(&doc.path);
        self.ask(Ask::SaveAs, None, Some(name));
    }

    /// Backups › Quick restore picked `from`: asks "Restore this backup?" first. With unsaved
    /// changes it is refused straight away, as the restore itself would be.
    pub(super) fn ask_restore(&mut self, from: PathBuf, what: String) {
        let Some(save) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        if self.dirty {
            return self.fail(tr("You have unsaved changes. Save or revert them first, then restore."));
        }
        self.quick_restore = Some((save, from, what));
    }

    /// Put `from` (a backup) back in place of `save`, the open file. The file as it is now is
    /// backed up first, so a restore can itself be undone. Does nothing when another file is
    /// open by now.
    pub(super) fn restore_from(&mut self, save: &Path, from: &Path, what: &str) {
        if self.doc.as_ref().map(|d| d.path.as_path()) != Some(save) {
            return;
        }
        if self.dirty {
            return self.fail(tr("You have unsaved changes. Save or revert them first, then restore."));
        }
        self.restore_to(save, from, what);
    }

    /// Put `from` back as `path` (which may be missing, after Back up and start a new game)
    /// and open it.
    pub(super) fn restore_to(&mut self, path: &Path, from: &Path, what: &str) {
        let path = path.to_path_buf();
        if self.dirty && self.doc.as_ref().is_some_and(|d| d.path != path) {
            return self.fail(trf("Save or revert the changes to {0} first.", &[&self.doc.as_ref().map(|d| file_name(&d.path)).unwrap_or_default()]));
        }
        if let Some(game) = crate::running::game_for(&path) {
            return self.fail(trf("{0} looks like it's running. Quit it first, or it may overwrite the restored save. Nothing was restored.", &[&game]));
        }
        // What to put back if a file the game checks against the save can't be updated.
        let before = match crate::backups::Kept::read(&path) {
            Ok(k) => k,
            Err(e) => return self.fail(trf("Couldn't restore the backup: {0}. The save on disk wasn't changed.", &[&trf("couldn't read {0}: {1}", &[&path.display(), &e])])),
        };
        match crate::backups::restore(&path, from) {
            Ok(()) => {
                let companions = crate::vfs::read(&path).map(|bytes| formats::companion_files(&path, &bytes)).unwrap_or_default();
                if let Err(e) = crate::backups::write_companions(&before, companions, true) {
                    if !e.stuck.is_empty() {
                        // Show what is on disk now.
                        self.dirty = false;
                        self.load(&path);
                    }
                    return self.fail(trf("Couldn't restore the backup. {0}", &[&companion_failed(&e)]));
                }
                self.dirty = false;
                self.load(&path);
                self.ok(trf("Restored {0}. The save it replaced was backed up too, so you can switch back.", &[&what]));
            }
            Err(e) => self.fail(trf("Couldn't restore the backup: {0}. The save on disk wasn't changed.", &[&e])),
        }
    }

    pub(super) fn export_decoded(&mut self) {
        if !self.commit_raw() {
            return self.fail(tr("The Raw JSON has a syntax error."));
        }
        let Some(doc) = &self.doc else { return };
        let ext = match &doc.content {
            Content::Json { .. } | Content::Tree(_) => "json",
            Content::Text(_) => "txt",
            Content::Binary(_) => "bin",
        };
        let stem = doc.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("save".into());
        self.ask(Ask::Export, None, Some(format!("{stem}.decoded.{ext}")));
    }

    pub(super) fn import_from(&mut self, src: &Path) {
        if self.dirty {
            return self.fail(tr("Save or revert your changes before importing."));
        }
        let Some(doc) = &self.doc else { return };
        let payload = match crate::vfs::read(src) {
            Ok(b) => b,
            Err(e) => return self.fail(trf("Couldn't read {0}: {1}", &[&src.display(), &e])),
        };
        let bytes = match formats::encode(&doc.layers, &Content::Binary(payload)) {
            Ok(b) => b,
            Err(e) => return self.fail(trf("Couldn't import: the data couldn't be packed into this save ({0}). Nothing was changed.", &[&e])),
        };
        let check = formats::decode_with(&bytes, &doc.opts);
        if !formats::same_layers(&check.layers, &doc.layers) || std::mem::discriminant(&check.content) != std::mem::discriminant(&doc.content) {
            return self.fail(tr("Couldn't import: the data doesn't fit this kind of save (wrong file, or saved in another format). Nothing was changed."));
        }
        let d = self.doc.as_mut().unwrap();
        d.content = check.content;
        // The whole document counts as edited, so the import shows as an unsaved change.
        self.undo.clear();
        self.redo.clear();
        self.last_edit = None;
        self.after_change("");
        self.recompute_dirty();
        self.ok(trf("Imported {0}. Review the changes, then save.", &[&file_name(src)]));
    }

    pub(super) fn export_to(&mut self, dest: &Path) {
        let Some(doc) = &self.doc else { return };
        // Decoded data in place of the save itself is a file the game can't load.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
            if real(dest) == real(&doc.path) {
                return self.fail(tr("That is the save that is open. Export under another name: the decoded data is not a file the game can load."));
            }
        }
        let result = match &doc.content {
            Content::Json { value, .. } => serde_json::to_vec_pretty(value).map_err(|e| e.to_string()),
            Content::Tree(t) => serde_json::to_vec_pretty(&t.view).map_err(|e| e.to_string()),
            other => other.to_bytes(),
        };
        #[cfg(target_arch = "wasm32")]
        let result = result.and_then(|b| crate::web::download(&file_name(dest), &b));
        #[cfg(not(target_arch = "wasm32"))]
        let result = result.and_then(|b| {
            // Whatever is there (perhaps a save) is kept first, like any file the app replaces.
            crate::backups::take(dest).map_err(|e| trf("{0} is already there and couldn't be backed up first ({1}), so it wasn't replaced.", &[&file_name(dest), &e]))?;
            write_in_place(dest, &b)
        });
        match result {
            Ok(()) => self.ok(trf("Exported to {0}", &[&dest.display()])),
            Err(e) => self.fail(trf("Couldn't export: {0}", &[&e])),
        }
    }
}
