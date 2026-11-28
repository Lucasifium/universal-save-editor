//! Tools › Copy a slot to another slot: for saves that keep several self-contained slots in one
//! file (Elden Ring characters, Final Fantasy VII, Super Mario 64 and other console games).

use super::{fit, modal, safe_choice, write_in_place, App, WEB};
use crate::fmt::slot_copy::{self, Slot};
use crate::i18n::{tr, trf};
use crate::theme;
use eframe::egui::{self, Color32, RichText};

/// The "copy a slot" dialog.
pub struct CopySlots {
    pub game: &'static str,
    pub slots: Vec<Slot>,
    pub from: usize,
    pub to: usize,
}

/// Files larger than this are never read to look for slots (the largest, Elden Ring, is ~28 MB).
const MAX_SIZE: usize = 48 << 20;

fn label(i: usize, s: &Slot) -> String {
    if s.used {
        trf("Slot {0}: {1}", &[&(i + 1), &s.label])
    } else {
        trf("Slot {0}: empty", &[&(i + 1)])
    }
}

impl App {
    /// The open file's slots, if it is a multi-slot save; worked out once per file version.
    pub(super) fn file_slots(&self, ctx: &egui::Context) -> Option<(&'static str, Vec<Slot>)> {
        let d = self.doc.as_ref()?;
        let kind = match &d.content {
            crate::formats::Content::Tree(t) => t.kind_name.clone(),
            _ => return None,
        };
        let id = egui::Id::new(("file_slots", d.path.clone(), format!("{:?}", d.stamp)));
        if let Some(hit) = ctx.data(|m| m.get_temp::<Option<(&'static str, Vec<Slot>)>>(id)) {
            return hit;
        }
        let found = crate::vfs::read(&d.path).ok().filter(|raw| raw.len() <= MAX_SIZE).and_then(|raw| slot_copy::slots(&raw)).filter(|(f, _)| f.kind == kind).map(|(f, s)| (f.game, s));
        ctx.data_mut(|m| m.insert_temp(id, found.clone()));
        found
    }

    /// The Tools menu entry (shown only for multi-slot saves).
    pub(super) fn copy_slot_button(&mut self, ui: &mut egui::Ui) {
        let Some((game, slots)) = self.file_slots(ui.ctx()) else { return };
        if ui
            .button(tr("Copy a slot to another slot\u{2026}"))
            .on_hover_text(trf("Duplicate a {0} save slot, e.g. to try something out on the copy", &[&game]))
            .clicked()
        {
            ui.close_menu();
            self.start_copy_slot(game, slots);
        }
    }

    /// Opens the "copy a slot" dialog for the open file's slots (refused with unsaved changes).
    pub(super) fn start_copy_slot(&mut self, game: &'static str, slots: Vec<Slot>) {
        if self.dirty {
            self.fail(tr("Save or revert your changes first. Copying works on the file as it is on disk."));
        } else {
            let from = slots.iter().position(|s| s.used).unwrap_or(0);
            let to = slots.iter().position(|s| !s.used).unwrap_or(if from == 0 { 1 } else { 0 });
            self.copy_slots = Some(CopySlots { game, slots, from, to });
        }
    }

    pub(super) fn copy_slots_dialog(&mut self, ctx: &egui::Context) {
        let Some(c) = &mut self.copy_slots else { return };
        let mut choice = None;
        modal(ctx, tr("Copy a slot"), |ui| {
            ui.set_max_width(fit(ctx, 520.0));
            ui.weak(c.game);
            egui::Grid::new("copy_slots").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(tr("Copy"));
                egui::ComboBox::from_id_salt("copy_from").width(320.0).wrap_mode(egui::TextWrapMode::Truncate).selected_text(label(c.from, &c.slots[c.from])).show_ui(ui, |ui| {
                    for (i, s) in c.slots.iter().enumerate().filter(|(_, s)| s.used) {
                        ui.selectable_value(&mut c.from, i, label(i, s));
                    }
                });
                ui.end_row();
                ui.label(tr("Into"));
                egui::ComboBox::from_id_salt("copy_to").width(320.0).wrap_mode(egui::TextWrapMode::Truncate).selected_text(label(c.to, &c.slots[c.to])).show_ui(ui, |ui| {
                    for (i, s) in c.slots.iter().enumerate() {
                        ui.selectable_value(&mut c.to, i, label(i, s));
                    }
                });
                ui.end_row();
            });
            let dest = &c.slots[c.to];
            if c.to == c.from {
                ui.colored_label(theme::fg(ui, theme::WARN), tr("Pick a different slot to copy into."));
            } else if dest.used {
                ui.colored_label(theme::fg(ui, theme::WARN), trf("This replaces slot {0} ({1}).", &[&(c.to + 1), &dest.label]));
            }
            ui.weak(tr("The file is backed up first (Backups \u{203A} Quick restore)."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = c.to != c.from && c.slots[c.from].used;
                if ui.add_enabled(ok, egui::Button::new(RichText::new(tr("Copy slot")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if safe_choice(ui.button(tr("Cancel"))).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                if let Some(c) = self.copy_slots.take() {
                    self.copy_slot(c.from, c.to);
                }
            }
            Some(false) => self.copy_slots = None,
            None => {}
        }
    }

    fn copy_slot(&mut self, from: usize, to: usize) {
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        if let Some(game) = crate::running::game_for(&path) {
            return self.fail(trf("{0} looks like it's running. Quit it first, or it will overwrite the copied slot. Nothing was copied.", &[&game]));
        }
        let raw = match crate::vfs::read(&path) {
            Ok(r) => r,
            Err(e) => return self.fail(trf("Couldn't read {0}: {1}", &[&path.display(), &e])),
        };
        // Made and read back (every other slot unchanged) before anything on disk is touched.
        let out = match slot_copy::copy(&raw, from, to) {
            Ok(o) => o,
            Err(e) => return self.fail(trf("Couldn't copy the slot: {0}. The save wasn't changed.", &[&e])),
        };
        if !WEB {
            if let Err(e) = crate::backups::take(&path) {
                return self.fail(trf("Couldn't back up the file first: {0}. Nothing was changed.", &[&e]));
            }
        }
        if let Err(e) = write_in_place(&path, &out) {
            // In the browser the download may already have happened.
            return self.fail(if WEB { trf("Couldn't write {0}: {1}", &[&path.display(), &e]) } else { trf("Couldn't write {0}: {1}. The file on disk wasn't changed.", &[&path.display(), &e]) });
        }
        self.load(&path);
        self.ok(trf("Copied slot {0} into slot {1}. It shows up in the game's own load menu.", &[&(from + 1), &(to + 1)]));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn copy_reaches_the_disk_only_through_a_checked_write() {
        // The write path: slot_copy::copy (made and read back), then the same verified
        // write every save uses. A slot copied on disk reads back with both slots equal.
        let dir = std::env::temp_dir().join(format!("use_slots_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.srm");
        let mut d = vec![0u8; 0x2000];
        d[0x263] = 12;
        let sum = crate::fmt::ff6snes::sum(&d, 0);
        d[0x9fe..0xa00].copy_from_slice(&sum.to_le_bytes());
        std::fs::write(&path, &d).unwrap();
        let out = crate::fmt::slot_copy::copy(&d, 0, 1).unwrap();
        super::write_in_place(&path, &out).unwrap();
        let back = std::fs::read(&path).unwrap();
        assert_eq!(&back[0xa00..0x1400], &d[..0xa00]);
        assert!(crate::fmt::ff6snes::valid(&back, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
