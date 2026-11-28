//! Browser version, where the browser allows it (Chrome and Edge): saving back to the file the
//! player picked, instead of downloading it; what to do when that file changed meanwhile; and
//! the window that hands back the copies of each file as it was before it was written, which
//! the browser keeps in its own storage.

use super::*;
use std::cell::{Cell, RefCell};

thread_local! {
    /// The "Originals kept in this browser" window is showing.
    static SHOWN: Cell<bool> = const { Cell::new(false) };
    /// How handing over a kept original went, for the window to show.
    static GOT: RefCell<Option<Result<String, String>>> = const { RefCell::new(None) };
    /// A save in place found the file changed on disk: the file and its handle id, for the
    /// "changed on disk" question.
    /// What was to happen once the save went through goes with it.
    static STALE: RefCell<Option<(PathBuf, u64, AfterSave)>> = const { RefCell::new(None) };
}

/// The newest originals the browser keeps, and their size in all (web/fsa.js has the same).
const KEEP_COUNT: usize = 10;
const KEEP_MB: usize = 200;

impl App {
    /// File menu: open a file so that saving writes back to it.
    pub(super) fn open_in_place_item(&mut self, ui: &mut egui::Ui) {
        if !crate::web::can_save_in_place() {
            return;
        }
        if ui
            .button(tr("Open to save in place\u{2026}"))
            .on_hover_text(tr("Save then writes back to the file you pick, once you allow it in the browser, instead of downloading it. The file as it was is kept in this browser first. The browser refuses some system folders this way; use Open\u{2026} for those."))
            .clicked()
        {
            ui.close_menu();
            self.open_in_place();
        }
    }

    pub(super) fn open_in_place(&mut self) {
        if self.saving.is_some() {
            return;
        }
        self.web_asked = Some(Ask::Open);
        crate::web::pick_file_in_place(&self.ctx);
    }

    /// Backups menu: the originals kept in this browser.
    pub(super) fn originals_item(&mut self, ui: &mut egui::Ui) {
        if !crate::web::can_save_in_place() {
            return;
        }
        if ui.button(tr("Originals kept in this browser\u{2026}")).on_hover_text(tr("Download a file as it was before you saved it in place")).clicked() {
            ui.close_menu();
            SHOWN.with(|s| s.set(true));
            crate::web::list_originals(&self.ctx);
        }
    }

    /// Builds and checks the file as `save_with` does, then writes it back through handle `id`
    /// (the open document's own) once the browser answers. Until then it counts as saving, like
    /// a save on a worker thread. `anyway`: write even though the file changed on disk.
    pub(super) fn save_in_place(&mut self, path: &Path, id: u64, then: AfterSave, anyway: bool) {
        if self.saving.is_some() || self.loading.is_some() {
            return;
        }
        let Some(doc) = &self.doc else { return };
        let (bytes, check) = match build_save(&doc.layers, &doc.content, &doc.opts) {
            Ok(b) => b,
            Err(e) => return self.fail(e),
        };
        if !formats::companion_files(path, &bytes).is_empty() {
            // Files the game checks against this save can only be handed over beside it, as
            // downloads, so the save goes the same way.
            let r = write_save(path, &doc.layers, &doc.content, &doc.opts);
            self.finish_save(path, r, false);
            return self.after_save(then);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let p = path.to_path_buf();
        let name = file_name(path);
        let len = bytes.len();
        let data = bytes.clone();
        let after = then.clone();
        crate::web::write_in_place(&self.ctx, id, &data, anyway, move |outcome| {
            if outcome == crate::web::InPlace::Changed {
                STALE.with(|s| *s.borrow_mut() = Some((p.clone(), id, after)));
            }
            let _ = tx.send(in_place_result(&p, &name, bytes, len, check, outcome));
        });
        self.saving = Some(Saving { path: path.to_path_buf(), then, gen: (self.edit_gen, self.tabs.active), started: f64::NAN, rx });
        self.info(trf("Saving {0}\u{2026}", &[&file_name(path)]));
    }

    /// Each frame: the "changed on disk" question, a file read again, and the originals window.
    pub(super) fn in_place_frame(&mut self, ctx: &egui::Context) {
        self.in_place_stale_dialog(ctx);
        // A file read again is shown once no save or open is under way: until then the file
        // on screen can't be replaced, and the read waits.
        if self.saving.is_none() && self.loading.is_none() {
            if let Some((id, r)) = crate::web::take_reread() {
                self.reread_done(id, r);
            }
        }
        self.originals_window(ctx);
    }

    /// The file a save in place was about to write changed on disk: Reload / Save anyway /
    /// Cancel, as the desktop app asks.
    fn in_place_stale_dialog(&mut self, ctx: &egui::Context) {
        let Some((path, id, then)) = STALE.with(|s| s.borrow().clone()) else { return };
        if self.saving.is_some() {
            return;
        }
        // Only while the document that found it is on screen.
        if !self.doc.as_ref().is_some_and(|d| d.path == path && d.handle.as_ref().map(|h| h.0) == Some(id)) {
            STALE.with(|s| *s.borrow_mut() = None);
            return;
        }
        let mut choice = None;
        modal(ctx, tr("The save changed on disk"), |ui| {
            ui.set_max_width(460.0);
            ui.label(trf("{0} changed on disk after you opened it. Most likely the game saved, or a cloud sync brought in another copy.", &[&file_name(&path)]));
            ui.add_space(4.0);
            ui.label(tr("Saving now would put the version you opened, with your changes, over that newer file."));
            ui.weak(tr("If you save anyway, the file as it is now is kept in this browser first: Backups \u{203A} Originals kept in this browser."));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let reload = ui.add(egui::Button::new(RichText::new(tr("Reload the newer file")).color(Color32::WHITE)).fill(theme::ACCENT)).on_hover_text(tr("Open what's on disk now. Your unsaved changes are dropped."));
                if reload.clicked() {
                    choice = Some(0);
                }
                if ui.button(tr("Save anyway")).clicked() {
                    choice = Some(1);
                }
                let cancel = safe_choice(ui.button(tr("Cancel")));
                if cancel.clicked() {
                    choice = Some(2);
                }
                tab_ring(&reload, &cancel);
            });
        });
        if choice.is_some() {
            STALE.with(|s| *s.borrow_mut() = None);
            // The "Not saved" message is answered; what happens next says how it went.
            self.banner = None;
        }
        match choice {
            Some(0) => crate::web::reread(&self.ctx, id),
            // Then whatever the save was for: closing the file, opening another, quitting.
            Some(1) => self.save_in_place(&path, id, then, true),
            Some(_) => self.info(tr("Not saved. Your changes are still here.")),
            None => {}
        }
    }

    /// The file behind handle `id`, read again ("Reload the newer file"): shown in place of the
    /// document holding that handle, which keeps it.
    fn reread_done(&mut self, id: u64, r: Result<Vec<u8>, String>) {
        let Some(path) = self.doc.as_ref().filter(|d| d.handle.as_ref().map(|h| h.0) == Some(id)).map(|d| d.path.clone()) else { return };
        let name = file_name(&path);
        if let Err(e) = r.and_then(|b| crate::vfs::write(&path, &b).map_err(|e| e.to_string())) {
            return self.fail(trf("Couldn't read {0} from your computer again: {1}. What is on screen wasn't changed.", &[&name, &e]));
        }
        let handle = self.doc.as_mut().and_then(|d| d.handle.take());
        let loads = self.loads;
        // The unsaved changes are dropped by the load itself, only once it has gone through.
        self.load(&path);
        let fresh = self.loads != loads;
        match self.doc.as_mut() {
            Some(d) if fresh && d.path == path => {
                d.handle = handle;
                // Only now does the page take this for what the file holds: a save from here
                // on writes over it without asking, as it is what is on screen.
                crate::web::accept_reread(id);
                self.ok(trf("Read {0} again from your computer.", &[&name]));
            }
            // It couldn't be shown: what is on screen keeps its handle.
            Some(d) if d.path == path => d.handle = handle,
            _ => {}
        }
    }

    fn originals_window(&mut self, ctx: &egui::Context) {
        if let Some(r) = GOT.with(|g| g.borrow_mut().take()) {
            match r {
                Ok(name) => self.ok(trf("Your browser downloaded the kept copy of {0}.", &[&name])),
                Err(e) if e.is_empty() => self.fail(tr("That copy isn't kept in this browser any more.")),
                Err(e) => self.fail(trf("Couldn't hand over the kept copy: {0}", &[&e])),
            }
        }
        if !SHOWN.with(|s| s.get()) {
            return;
        }
        let mut open = true;
        let mut get = None;
        egui::Window::new(tr("Originals kept in this browser")).open(&mut open).collapsible(false).default_pos(super::modal::start_pos(ctx, fit(ctx, 520.0))).default_width(fit(ctx, 520.0)).max_width(fit(ctx, 700.0)).show(ctx, |ui| {
            ui.label(trf("Before a file opened with Open to save in place is written, the file as it is then is kept here, unless it is just what this page saved there last. These copies live in this browser's own storage, not in a folder on your computer: clearing this site's data in the browser removes them. The newest {0} are kept, up to {1} MB in all.", &[&KEEP_COUNT, &KEEP_MB]));
            ui.add_space(6.0);
            match crate::web::originals() {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr("Reading the browser's storage\u{2026}"));
                    });
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(trf("The browser's storage couldn't be read: {0}", &[&e])).color(theme::fg(ui, theme::ERR)));
                }
                Some(Ok(list)) if list.is_empty() => {
                    ui.weak(tr("Nothing is kept yet."));
                }
                Some(Ok(list)) => {
                    ScrollArea::vertical().show_list(ui, "originals_list", 260.0, |ui| {
                        for o in &list {
                            ui.horizontal(|ui| {
                                if ui.button(tr("Download")).clicked() {
                                    get = Some((o.id, o.name.clone()));
                                }
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&o.name).strong());
                                    ui.weak(format!("{} \u{00B7} {}", human_size(o.size as usize), o.when));
                                });
                            });
                        }
                    });
                }
            }
        });
        if !open {
            SHOWN.with(|s| s.set(false));
        }
        if let Some((id, name)) = get {
            crate::web::download_original(ctx, id, move |r| GOT.with(|g| *g.borrow_mut() = Some(r.map(|()| name))));
        }
    }
}

/// The line the "Save changes?" list shows about where the file goes (`in_place`: back to
/// the file the document was opened from).
pub(super) fn save_note(in_place: bool) -> &'static str {
    if in_place {
        tr("The browser asks whether this page may change the file, then writes it back to the file you opened. Unless the file is just what this page saved there last, it is kept in this browser first: Backups \u{203A} Originals kept in this browser.")
    } else {
        tr("Your browser downloads the edited file. Keep a copy of the original before you replace it.")
    }
}

/// What a save in place came to, in the words `finish_save` shows. Where the browser didn't
/// let the file be written, it is downloaded instead, as every save is in other browsers, and
/// the message says only that.
fn in_place_result(path: &Path, name: &str, bytes: Vec<u8>, len: usize, check: formats::Decoded, outcome: crate::web::InPlace) -> Result<Written, String> {
    use crate::web::InPlace;
    let instead = |why: String| match crate::web::download(name, &bytes) {
        Ok(()) => Ok((format!("{why} {}", tr("Your browser downloaded the edited file instead.")), true)),
        Err(e) => Err(trf("{0} The download didn't start either ({1}). Your changes are still here.", &[&why, &e])),
    };
    let (note, instead) = match outcome {
        InPlace::Saved { kept: true } => Ok((trf("Written back to {0} on your computer. The file as it was before is kept in this browser: Backups \u{203A} Originals kept in this browser.", &[&name]), false)),
        InPlace::Saved { kept: false } => Ok((trf("Written back to {0} on your computer.", &[&name]), false)),
        InPlace::Refused(e) if e.is_empty() => instead(trf("The browser wasn't allowed to change {0}, so that file wasn't changed.", &[&name])),
        InPlace::Refused(e) => instead(trf("The browser couldn't ask to change {0} ({1}), so that file wasn't changed.", &[&name, &e])),
        InPlace::NoBackup(e) => instead(trf("{0} wasn't changed: a copy of it couldn't be kept in this browser first ({1}).", &[&name, &e])),
        InPlace::Failed { error, maybe_changed: false } => Err(trf("Couldn't save to {0}: {1}. The file on your computer wasn't changed. Your changes are still here: save again, or use Save a copy as to download them.", &[&name, &error])),
        InPlace::Failed { error, maybe_changed: true } => Err(trf("Couldn't save to {0}: {1}. The browser can't tell whether the file on your computer changed. Your changes are still here, and copies of the file kept in this browser are under Backups \u{203A} Originals kept in this browser.", &[&name, &error])),
        InPlace::Changed => Err(trf("Not saved: {0} on your computer changed since this page last read or wrote it, so it wasn't touched. Your changes are still here.", &[&name])),
        InPlace::NoHandle => Err(trf("Not saved: this page no longer has access to {0}, so it wasn't touched. Your changes are still here; Save a copy as downloads them.", &[&name])),
    }?;
    // The tab's copy of the file is what was written or downloaded.
    let _ = crate::vfs::write(path, &bytes);
    Ok(Written { check, len, note: Some(note), stamp: None, instead })
}
