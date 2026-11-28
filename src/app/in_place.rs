//! Browser version, where the browser allows it (Chrome and Edge): saving back to the file the
//! player picked, instead of downloading it, and the window that hands back the copies of each
//! file as it was before its first such save, which the browser keeps in its own storage.

use super::*;
use std::cell::{Cell, RefCell};

thread_local! {
    /// The "Originals kept in this browser" window is showing.
    static SHOWN: Cell<bool> = const { Cell::new(false) };
    /// How handing over a kept original went, for the window to show.
    static GOT: RefCell<Option<Result<String, String>>> = const { RefCell::new(None) };
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
        if ui.button(tr("Originals kept in this browser\u{2026}")).on_hover_text(tr("Download a file as it was before you first saved it in place")).clicked() {
            ui.close_menu();
            SHOWN.with(|s| s.set(true));
            crate::web::list_originals(&self.ctx);
        }
    }

    /// Builds and checks the file as `save_with` does, then writes it back to the player's file
    /// once the browser answers. Until then it counts as saving, like a save on a worker thread.
    pub(super) fn save_in_place(&mut self, path: &Path, then: AfterSave) {
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
        crate::web::write_in_place(&self.ctx, path, &data, move |outcome| {
            let _ = tx.send(in_place_result(&p, &name, bytes, len, check, outcome));
        });
        self.saving = Some(Saving { path: path.to_path_buf(), then, gen: (self.edit_gen, self.tabs.active), started: f64::NAN, rx });
        self.info(trf("Saving {0}\u{2026}", &[&file_name(path)]));
    }

    pub(super) fn originals_window(&mut self, ctx: &egui::Context) {
        if let Some(r) = GOT.with(|g| g.borrow_mut().take()) {
            match r {
                Ok(name) => self.ok(trf("Your browser downloaded {0} as it was before it was first saved here.", &[&name])),
                Err(e) if e.is_empty() => self.fail(tr("That copy isn't kept in this browser any more.")),
                Err(e) => self.fail(trf("Couldn't hand over the kept copy: {0}", &[&e])),
            }
        }
        if !SHOWN.with(|s| s.get()) {
            return;
        }
        let mut open = true;
        let mut get = None;
        egui::Window::new(tr("Originals kept in this browser")).open(&mut open).collapsible(false).default_width(fit(ctx, 520.0)).max_width(fit(ctx, 700.0)).show(ctx, |ui| {
            ui.label(trf("Before the first save to a file opened with Open to save in place, the file as it was is kept here. These copies live in this browser's own storage, not in a folder on your computer: clearing this site's data in the browser removes them. The newest {0} are kept, up to {1} MB in all.",
                &[&KEEP_COUNT, &KEEP_MB],
            ));
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

/// The line the "Save changes?" list shows about where the file goes.
pub(super) fn save_note(path: &Path) -> &'static str {
    if crate::web::has_handle(path) {
        tr("The browser asks whether this page may change the file, then writes it back to the file you opened. Before the first save, the file as it was is kept in this browser: Backups \u{203A} Originals kept in this browser.")
    } else {
        tr("Your browser downloads the edited file. Keep a copy of the original before you replace it.")
    }
}

/// What a save in place came to, in the words `finish_save` shows. Where the browser didn't
/// let the file be written, it is downloaded instead, as every save is in other browsers.
fn in_place_result(path: &Path, name: &str, bytes: Vec<u8>, len: usize, check: formats::Decoded, outcome: crate::web::InPlace) -> Result<Written, String> {
    use crate::web::InPlace;
    let instead = |why: String| match crate::web::download(name, &bytes) {
        Ok(()) => Ok(format!("{why} {}", tr("Your browser downloaded the edited file instead."))),
        Err(e) => Err(trf("{0} The download didn't start either ({1}). Your changes are still here.", &[&why, &e])),
    };
    let note = match outcome {
        InPlace::Saved { kept: true } => {
            Ok(trf("Written back to {0} on your computer. The file as it was before is kept in this browser: Backups \u{203A} Originals kept in this browser.", &[&name]))
        }
        InPlace::Saved { kept: false } => Ok(trf("Written back to {0} on your computer.", &[&name])),
        InPlace::Refused(e) if e.is_empty() => instead(trf("The browser wasn't allowed to change {0}, so that file wasn't changed.", &[&name])),
        InPlace::Refused(e) => instead(trf("The browser couldn't ask to change {0} ({1}), so that file wasn't changed.", &[&name, &e])),
        InPlace::NoBackup(e) => instead(trf("{0} wasn't changed: a copy of it couldn't be kept in this browser first ({1}).", &[&name, &e])),
        InPlace::Failed { error, maybe_changed: false } => {
            Err(trf("Couldn't save to {0}: {1}. The file on your computer wasn't changed. Your changes are still here: save again, or use Save a copy as to download them.", &[&name, &error]))
        }
        InPlace::Failed { error, maybe_changed: true } => Err(trf("Couldn't save to {0}: {1}. The browser can't tell whether the file on your computer changed. Your changes are still here, and the file as it was before your first save is kept in this browser: Backups \u{203A} Originals kept in this browser.",
            &[&name, &error],
        )),
    }?;
    // The tab's copy of the file is what was written or downloaded.
    let _ = crate::vfs::write(path, &bytes);
    Ok(Written { check, len, note: Some(note), stamp: None })
}
