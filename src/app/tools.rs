//! Tools for games the editor has no screen for: bulk edit in the Values list, presets (a set
//! of changes saved and replayed on another save), names from the hex view shared across
//! saves of a game, the guided value search and refining by itself when the game saves.

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use super::*;
use crate::presets::{ByteChange, Preset, ValueChange};
use crate::values_view::BulkOp;

/// Bulk edit in the Values tab: the ticked rows and what to do to them.
pub(super) struct Bulk {
    pub ticked: HashSet<usize>,
    pub op: BulkOp,
    pub arg: String,
    /// The Values list the ticks belong to (`App::model_gen`); a new list clears them.
    pub model_gen: u64,
}

/// The "Save these changes as a preset" dialog.
pub(super) struct PresetSave {
    pub preset: Preset,
    pub replace: bool,
}

/// The "Apply a preset" dialog.
pub(super) struct PresetApply {
    pub list: Vec<(PathBuf, Preset)>,
    pub picked: Option<usize>,
    /// A preset opened from a file outside the presets folder.
    pub opened: Option<Preset>,
}

/// The "Use these names for every save of this game" dialog.
pub(super) struct ShareNames {
    pub game: String,
    pub by_ext: bool,
    pub by_magic: bool,
    pub by_size: bool,
    pub replace: bool,
}

/// What the buttons under the hex view's named values asked for.
pub(super) enum NamesAction {
    Share,
    Export,
    Import,
}

/// Where the game's issue tracker takes requests for new games.
const REQUEST_GAME: &str = "https://github.com/Lucasifium/universal-save-editor/issues/new?title=Support%20for%20";

impl App {
    // ------------------------------------------------------------ bulk edit

    pub(super) fn bulk_ticked(&mut self) -> Option<&HashSet<usize>> {
        if self.bulk.as_ref().is_some_and(|b| b.model_gen != self.model_gen) {
            let b = self.bulk.as_mut().unwrap();
            b.ticked.clear();
            b.model_gen = self.model_gen;
        }
        self.bulk.as_ref().map(|b| &b.ticked)
    }

    pub(super) fn bulk_toggle(&mut self, rows: Vec<usize>) {
        let Some(b) = &mut self.bulk else { return };
        for i in rows {
            if !b.ticked.remove(&i) {
                b.ticked.insert(i);
            }
        }
    }

    /// The bar under the search box while bulk edit is on.
    pub(super) fn bulk_bar(&mut self, ui: &mut egui::Ui, now: f64) {
        let Some(b) = &mut self.bulk else { return };
        let shown: Vec<usize> =
            self.disp.iter().filter_map(|d| if let Disp::Item(i) = d { Some(*i) } else { None }).filter(|&i| self.model.rows[i].editable).collect();
        let mut apply = false;
        let mut close = false;
        theme::card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Bulk edit").strong());
                ui.label(format!("{} ticked", b.ticked.len()));
                if ui.small_button(format!("Tick all shown ({})", shown.len())).on_hover_text("Ticks every editable value in the list below. Search first to pick a group, e.g. \u{201C}count\u{201D} or a section's name.").clicked() {
                    b.ticked.extend(shown.iter().copied());
                }
                if !b.ticked.is_empty() && ui.small_button("Untick all").clicked() {
                    b.ticked.clear();
                }
                ui.separator();
                egui::ComboBox::from_id_salt("bulk_op").selected_text(b.op.name()).width(100.0).show_ui(ui, |ui| {
                    for op in BulkOp::ALL {
                        ui.selectable_value(&mut b.op, op, op.name());
                    }
                });
                let hint = match b.op {
                    BulkOp::Set => "99",
                    BulkOp::Add => "100 or -100",
                    BulkOp::Multiply => "2",
                };
                let r = ui.add(TextEdit::singleline(&mut b.arg).hint_text(hint).desired_width(90.0));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let ready = !b.ticked.is_empty() && !b.arg.trim().is_empty();
                if ui.add_enabled(ready, egui::Button::new("Apply")).on_hover_text("Changes every ticked value; one Undo puts them all back").clicked() || (enter && ready) {
                    apply = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Done").clicked() {
                        close = true;
                    }
                });
            });
        });
        if apply {
            self.apply_bulk(now);
        }
        if close {
            self.bulk = None;
        }
    }

    fn apply_bulk(&mut self, now: f64) {
        let Some(b) = &self.bulk else { return };
        let (op, arg) = (b.op, b.arg.clone());
        let mut rows: Vec<usize> = b.ticked.iter().copied().collect();
        rows.sort_unstable();
        let start = self.undo.len();
        let (mut changed, mut skipped, mut refused) = (0, 0, Vec::new());
        for i in rows {
            let Some(row) = self.model.rows.get(i) else { continue };
            let (ptr, name) = (row.ptr.clone(), row.name.clone());
            let Some(cur) = self.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(&ptr)).cloned() else { continue };
            match values_view::bulk_value(&cur, op, &arg) {
                Ok(Some(v)) => match self.try_edit(ptr, v, now) {
                    Ok(true) => changed += 1,
                    Ok(false) => {}
                    Err(e) => refused.push(format!("{name} ({e})")),
                },
                Ok(None) => skipped += 1,
                Err(e) => return self.fail(format!("Bulk edit: {e}.")),
            }
            // Each value is its own edit, not merged with the one before.
            self.last_edit = None;
        }
        self.group_undo_from(start);
        let mut msg = format!("Bulk edit changed {changed} value{}.", if changed == 1 { "" } else { "s" });
        if skipped > 0 {
            msg.push_str(&format!(" {skipped} of another kind left alone (a number changes only numbers; true or false only switches; other text only text)."));
        }
        if refused.is_empty() {
            self.ok(msg);
        } else {
            let n = refused.len();
            refused.truncate(3);
            self.fail(format!("{msg} {n} not changed, the save can't hold that: {}{}", refused.join("; "), if n > 3 { "\u{2026}" } else { "" }));
        }
    }

    // ------------------------------------------------------------ change list as text, presets

    /// What kind of save is open, in words ("Diablo II character"), for presets and the
    /// shared change list.
    fn save_kind(&self) -> String {
        self.doc.as_ref().map(|d| friendly_kind(&d.layers, &d.content)).unwrap_or_default()
    }

    /// The unsaved changes as plain text, for pasting into a message or a forum post.
    pub(super) fn copy_changes(&mut self, ctx: &egui::Context) {
        let lines = self.change_lines();
        if lines.is_empty() {
            return self.info("No changes to copy yet.");
        }
        let mut text = format!("Changes to a {} save:\n", self.save_kind());
        for l in &lines {
            text.push_str(&format!("- {}\n", l.replace('\u{2192}', "->").replace('\u{203A}', ">")));
        }
        ctx.copy_text(text);
        self.ok(format!("Copied {} change{} as text.", lines.len(), if lines.len() == 1 { "" } else { "s" }));
    }

    /// The unsaved changes as a preset.
    fn preset_from_changes(&self) -> Preset {
        let mut p = Preset { name: String::new(), game: self.save_kind(), ..Default::default() };
        let Some(d) = &self.doc else { return p };
        match (&d.content, &d.baseline) {
            (Content::Binary(cur), Content::Binary(base)) => {
                for (s, e) in byte_ranges(cur, base) {
                    let name = self.hex.labels.get(&s).map(|l| l.name.clone()).unwrap_or(format!("Offset 0x{s:X}"));
                    p.bytes.push(ByteChange { offset: s, name, bytes: cur[s..e].to_vec() });
                }
            }
            (c, b) => {
                let (Some(cv), Some(bv)) = (root(c), root(b)) else { return p };
                for ptr in &self.touched {
                    // The whole document (a Raw JSON edit) is no use on another save.
                    if ptr.is_empty() || cv.pointer(ptr) == bv.pointer(ptr) {
                        continue;
                    }
                    let Some(v) = cv.pointer(ptr).filter(|v| !v.is_object() && !v.is_array()) else { continue };
                    let name = match self.model.by_ptr.get(ptr.as_str()) {
                        Some(&i) if !self.model.rows[i].context.is_empty() => format!("{} \u{203A} {}", self.model.rows[i].context, self.model.rows[i].name),
                        Some(&i) => self.model.rows[i].name.clone(),
                        None => ptr.clone(),
                    };
                    p.values.push(ValueChange { path: ptr.clone(), name, value: v.clone() });
                }
            }
        }
        p
    }

    pub(super) fn start_preset_save(&mut self) {
        let preset = self.preset_from_changes();
        if preset.is_empty() {
            return self.info("Make some changes first: a preset keeps the changes you made, to make them again on another save.");
        }
        self.preset_save = Some(PresetSave { preset, replace: false });
    }

    pub(super) fn preset_save_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut ps) = self.preset_save.take() else { return };
        let mut choice = None;
        modal(ctx, "Save changes as a preset", |ui| {
            ui.label(format!("Keeps these {} change{} to make again on another save or slot of this game:", ps.preset.len(), if ps.preset.len() == 1 { "" } else { "s" }));
            ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                for l in ps.preset.lines() {
                    ui.weak(l);
                }
            });
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut ps.preset.name).hint_text("for example: max gold and potions").desired_width(300.0));
            r.request_focus();
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            if !WEB {
                ui.checkbox(&mut ps.replace, "Replace a preset with the same name");
            } else {
                ui.weak("Your browser downloads the preset; open it again with Apply preset.");
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ok = !ps.preset.name.trim().is_empty();
                if ui.add_enabled(ok, egui::Button::new(RichText::new("Save preset").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) if !ps.preset.name.trim().is_empty() => {
                ps.preset.name = ps.preset.name.trim().to_string();
                #[cfg(not(target_arch = "wasm32"))]
                match crate::presets::keep(&ps.preset, ps.replace) {
                    Ok(_) => self.ok(format!("Saved the preset \u{201C}{}\u{201D}. Use Tools \u{203A} Presets \u{203A} Apply a preset on another save.", ps.preset.name)),
                    Err(e) => {
                        self.fail(format!("Couldn't save the preset: {e}."));
                        self.preset_save = Some(ps);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                match crate::web::download(&crate::presets::file_name(&ps.preset.name), ps.preset.to_json().as_bytes()) {
                    Ok(()) => self.ok("Your browser downloaded the preset."),
                    Err(e) => self.fail(format!("Couldn't hand over the preset: {e}")),
                }
            }
            Some(_) => {}
            None => self.preset_save = Some(ps),
        }
    }

    pub(super) fn start_preset_apply(&mut self) {
        let list = crate::presets::list();
        let picked = (!list.is_empty()).then_some(0);
        self.preset_apply = Some(PresetApply { list, picked, opened: None });
        if WEB {
            self.ask(Ask::PresetFile, None, None);
        }
    }

    /// A preset file picked with "Open a preset file".
    pub(super) fn open_preset_file(&mut self, p: &Path) {
        let text = match crate::vfs::read_to_string(p) {
            Ok(t) => t,
            Err(e) => return self.fail(format!("Couldn't read {}: {e}", file_name(p))),
        };
        match Preset::from_json(&text) {
            Ok(preset) => {
                let pa = self.preset_apply.get_or_insert_with(|| PresetApply { list: vec![], picked: None, opened: None });
                pa.opened = Some(preset);
                pa.picked = None;
            }
            Err(e) => self.fail(format!("{} isn't a preset: {e}.", file_name(p))),
        }
    }

    pub(super) fn preset_apply_dialog(&mut self, ctx: &egui::Context, now: f64) {
        let Some(mut pa) = self.preset_apply.take() else { return };
        let kind = self.save_kind();
        let mut apply: Option<Preset> = None;
        let mut close = false;
        let mut open_file = false;
        let mut delete: Option<usize> = None;
        modal(ctx, "Apply a preset", |ui| {
            ui.set_max_width(460.0);
            if pa.list.is_empty() && pa.opened.is_none() {
                ui.label("No presets yet.");
                ui.weak("Make some changes, then use Save changes as a preset (in the Changes panel or Tools \u{203A} Presets). Presets shared by others open with Open a preset file.");
            }
            if !pa.list.is_empty() {
                ScrollArea::vertical().id_salt("preset_list").max_height(200.0).show(ui, |ui| {
                    for (i, (_, p)) in pa.list.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let text = format!("{}  ({} change{})", p.name, p.len(), if p.len() == 1 { "" } else { "s" });
                            if ui.selectable_label(pa.picked == Some(i) && pa.opened.is_none(), text).on_hover_text(format!("Made on a {} save", p.game)).clicked() {
                                pa.picked = Some(i);
                                pa.opened = None;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("Delete").on_hover_text("Delete this preset (saves are not changed)").clicked() {
                                    delete = Some(i);
                                }
                            });
                        });
                    }
                });
            }
            let chosen = pa.opened.as_ref().or_else(|| pa.picked.and_then(|i| pa.list.get(i)).map(|(_, p)| p));
            if let Some(p) = chosen {
                ui.separator();
                ui.label(RichText::new(&p.name).strong());
                if !p.game.is_empty() && p.game != kind {
                    ui.colored_label(theme::ERR, format!("Made on a {} save; this one is a {kind} save. Only the values found here are changed.", p.game));
                }
                ScrollArea::vertical().id_salt("preset_lines").max_height(160.0).show(ui, |ui| {
                    for l in p.lines() {
                        ui.weak(l);
                    }
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ok = chosen.is_some() && self.doc.is_some();
                if ui.add_enabled(ok, egui::Button::new(RichText::new("Apply").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    apply = chosen.cloned();
                }
                if ui.button("Open a preset file\u{2026}").clicked() {
                    open_file = true;
                }
                #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
                if let Some(d) = crate::presets::dir().filter(|d| d.is_dir()) {
                    if ui.button("Presets folder").clicked() {
                        crate::platform::reveal(&d);
                    }
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
        if let Some(i) = delete {
            let (path, p) = pa.list.remove(i);
            match crate::vfs::remove_file(&path) {
                Ok(()) => self.ok(format!("Deleted the preset \u{201C}{}\u{201D}.", p.name)),
                Err(e) => self.fail(format!("Couldn't delete the preset: {e}")),
            }
            pa.picked = None;
        }
        if let Some(p) = apply {
            self.apply_preset(&p, now);
            return;
        }
        if close {
            return;
        }
        self.preset_apply = Some(pa);
        if open_file {
            #[cfg(not(target_arch = "wasm32"))]
            let dir = crate::presets::dir().filter(|d| d.is_dir());
            #[cfg(target_arch = "wasm32")]
            let dir = None;
            self.ask(Ask::PresetFile, dir, None);
        }
    }

    fn apply_preset(&mut self, p: &Preset, now: f64) {
        let start = self.undo.len();
        let (mut changed, mut missing, mut refused) = (0usize, 0usize, Vec::new());
        let binary = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_)));
        if binary {
            let len = match self.doc.as_ref().map(|d| &d.content) {
                Some(Content::Binary(b)) => b.len(),
                _ => 0,
            };
            for c in &p.bytes {
                if c.offset + c.bytes.len() > len {
                    missing += 1;
                    continue;
                }
                self.apply_bytes(c.offset, c.bytes.clone());
                changed += 1;
            }
            missing += p.values.len();
        } else {
            for c in &p.values {
                let exists = self.doc.as_ref().and_then(|d| root(&d.content)).is_some_and(|r| r.pointer(&c.path).is_some());
                if !exists {
                    missing += 1;
                    continue;
                }
                match self.try_edit(c.path.clone(), c.value.clone(), now) {
                    Ok(true) => changed += 1,
                    Ok(false) => {}
                    Err(e) => refused.push(format!("{} ({e})", c.name)),
                }
                self.last_edit = None;
            }
            missing += p.bytes.len();
        }
        self.group_undo_from(start);
        let mut msg = format!("Applied \u{201C}{}\u{201D}: {changed} value{} changed.", p.name, if changed == 1 { "" } else { "s" });
        if missing > 0 {
            msg.push_str(&format!(" {missing} not in this save, left out."));
        }
        if changed > 0 {
            msg.push_str(" Review them in the Changes panel, then save.");
            self.show_changes = true;
        }
        if refused.is_empty() {
            self.ok(msg);
        } else {
            self.fail(format!("{msg} Not changed: {}.", refused.join("; ")));
        }
    }

    // ------------------------------------------------------------ names shared across saves

    pub(super) fn export_names(&mut self, dest: &Path) {
        let list: Vec<Value> = self.hex.labels.iter().map(|(o, l)| json!({"offset": o, "name": l.name, "type": l.ty.name()})).collect();
        let text = serde_json::to_string_pretty(&json!({ "game": self.save_kind(), "labels": list })).unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let r = crate::web::download(&file_name(dest), text.as_bytes());
        #[cfg(not(target_arch = "wasm32"))]
        let r = crate::vfs::write(dest, text).map_err(|e| e.to_string());
        match r {
            Ok(()) => self.ok(format!("Exported {} name{} to {}.", list.len(), if list.len() == 1 { "" } else { "s" }, file_name(dest))),
            Err(e) => self.fail(format!("Couldn't export the names: {e}")),
        }
    }

    pub(super) fn import_names(&mut self, src: &Path) {
        let text = match crate::vfs::read_to_string(src) {
            Ok(t) => t,
            Err(e) => return self.fail(format!("Couldn't read {}: {e}", file_name(src))),
        };
        let names = match crate::fmt::template::names_from_json(&text) {
            Ok(n) => n,
            Err(e) => return self.fail(format!("{} has no names to import: {e}.", file_name(src))),
        };
        let len = match self.doc.as_ref().map(|d| &d.content) {
            Some(Content::Binary(b)) => b.len(),
            _ => return,
        };
        let (mut added, mut outside) = (0, 0);
        for (at, name, ty) in names {
            let Some(ty) = NumType::from_name(&ty) else { continue };
            if at + ty.size().max(1) > len {
                outside += 1;
                continue;
            }
            self.hex.labels.insert(at, OffsetLabel { name, ty });
            added += 1;
        }
        self.save_labels();
        let mut msg = format!("Imported {added} name{}.", if added == 1 { "" } else { "s" });
        if outside > 0 {
            msg.push_str(&format!(" {outside} point past the end of this file and were left out."));
        }
        self.ok(msg);
    }

    pub(super) fn start_share_names(&mut self) {
        let game = self.doc.as_ref().and_then(|d| d.path.parent()).and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.share_names = Some(ShareNames { game, by_ext: true, by_magic: true, by_size: false, replace: false });
    }

    /// The first bytes of the file, when they look like a fixed header worth matching on.
    fn header_bytes(data: &[u8]) -> Option<Vec<u8>> {
        let h = data.get(..4)?;
        (h.iter().any(|&b| b != 0) && h.iter().any(|&b| b != 0xFF)).then(|| h.to_vec())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn share_names_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut sn) = self.share_names.take() else { return };
        let Some(d) = &self.doc else { return };
        let Content::Binary(data) = &d.content else { return };
        let ext = d.path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let magic = Self::header_bytes(data);
        let size = data.len();
        let sum = d.opts.sum;
        let mut choice = None;
        modal(ctx, "Use these names for every save of this game", |ui| {
            ui.set_max_width(440.0);
            ui.label("Makes a template from your names, so every save of the game opens with these values listed by name, ready to change. Use View \u{203A} Show decoded data as hex to find more values later.");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Game");
                ui.add(TextEdit::singleline(&mut sn.game).hint_text("the game's name").desired_width(260.0));
            });
            ui.add_space(4.0);
            ui.label("Recognise its saves by:");
            ui.add_enabled_ui(!ext.is_empty(), |ui| ui.checkbox(&mut sn.by_ext, format!("the file extension (.{ext})")));
            if let Some(m) = &magic {
                let hex: Vec<String> = m.iter().map(|b| format!("{b:02X}")).collect();
                ui.checkbox(&mut sn.by_magic, format!("the first bytes ({})", hex.join(" ")));
            }
            ui.checkbox(&mut sn.by_size, format!("the exact size ({})", human_size(size))).on_hover_text("Only if every save of the game is the same size");
            if let Some(s) = sum {
                ui.weak(format!("The checksum you set ({}) is kept fixed too.", s.describe()));
            }
            ui.checkbox(&mut sn.replace, "Replace a template with the same name");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Make template").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let by = crate::fmt::template::MatchBy {
                    extension: (sn.by_ext && !ext.is_empty()).then(|| ext.clone()),
                    magic: if sn.by_magic { magic.map(|m| (0, m)) } else { None },
                    size: sn.by_size.then_some(size),
                };
                let types: Vec<(usize, String, &'static str)> = self.hex.labels.iter().map(|(o, l)| (*o, l.name.clone(), l.ty.name())).collect();
                let named: Vec<crate::fmt::template::Named> = types.iter().map(|(at, name, ty)| crate::fmt::template::Named { at: *at, name, ty }).collect();
                let made = crate::fmt::template::from_names(&sn.game, &named, self.hex.big_endian, &by, sum)
                    .and_then(|(text, skipped)| crate::fmt::template::keep(&sn.game, &text, sn.replace).map(|p| (p, skipped)));
                match made {
                    Ok((p, skipped)) => {
                        let mut msg = format!("Made the template {}: saves of {} now open with your names.", file_name(&p), sn.game.trim());
                        if !skipped.is_empty() {
                            msg.push_str(&format!(" Left out (text, i8, i64 and f64 can't go in a template): {}.", skipped.join(", ")));
                        }
                        if !self.dirty {
                            self.reopen_with(|_| {});
                        }
                        self.ok(msg);
                    }
                    Err(e) => {
                        self.fail(format!("Couldn't make the template: {e}."));
                        self.share_names = Some(sn);
                    }
                }
            }
            Some(false) => {}
            None => self.share_names = Some(sn),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn share_names_dialog(&mut self, _ctx: &egui::Context) {
        self.share_names = None;
    }

    /// The buttons under the named values in the hex view.
    pub(super) fn names_buttons(ui: &mut egui::Ui, has_names: bool) -> Option<NamesAction> {
        let mut out = None;
        ui.horizontal_wrapped(|ui| {
            if !WEB
                && ui
                    .add_enabled(has_names, egui::Button::new("Use for every save\u{2026}"))
                    .on_hover_text("Make a template from these names, so every save of this game opens with them listed")
                    .clicked()
            {
                out = Some(NamesAction::Share);
            }
            if ui.add_enabled(has_names, egui::Button::new("Export\u{2026}")).on_hover_text("Save these names to a file to share or keep").clicked() {
                out = Some(NamesAction::Export);
            }
            if ui.button("Import\u{2026}").on_hover_text("Add names from a file someone shared (exported names or a template)").clicked() {
                out = Some(NamesAction::Import);
            }
        });
        out
    }

    // ------------------------------------------------------------ guided value search

    /// The step-by-step card at the top of the hex view's tools.
    pub(super) fn guide_card(ui: &mut egui::Ui, h: &mut HexState, hide: &mut bool) {
        let step = h.guide_step();
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Find a value, step by step").family(theme::semibold_family()).size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Hide").on_hover_text("Bring it back from Value search").clicked() {
                        *hide = true;
                    }
                });
            });
            let left = h.results.len();
            let steps: [(&str, String); 4] = [
                ("Search", "In the game, look at a number you want to change (gold, ammo, a stat) and save. Type it in Value search below and press Search.".into()),
                (
                    "Narrow it down",
                    if h.all_locations {
                        "Every location is in play. Make the value go up or down in the game, save, then pick Increased or Decreased and press Refine.".into()
                    } else {
                        format!("{left} places hold that number. Change it in the game (spend or earn some), save, type the new number and press Refine. Repeat until a few are left.")
                    },
                ),
                (
                    "Name it",
                    format!("{left} left. Click one to select it, then type a name under Named values and press Add; the name is remembered for this file. To narrow them further, change the number in the game again and Refine."),
                ),
                ("Change it", "Type the new number next to its name and press Set, then save with Ctrl+S. Load the save in the game to check.".into()),
            ];
            for (i, (title, text)) in steps.iter().enumerate() {
                let n = i + 1;
                let current = n == step;
                ui.horizontal_top(|ui| {
                    let badge = RichText::new(format!(" {n} ")).monospace();
                    let badge = if current { badge.color(Color32::WHITE).background_color(theme::ACCENT) } else { badge.weak() };
                    ui.label(badge);
                    ui.vertical(|ui| {
                        if current {
                            ui.label(RichText::new(*title).strong());
                            ui.label(text.as_str());
                        } else {
                            ui.weak(*title);
                        }
                    });
                });
            }
            if (step == 2 || step == 3) && !WEB {
                ui.checkbox(&mut h.auto_refine, "Refine by itself each time the game saves")
                    .on_hover_text("Watches the file and refines with the test chosen in Value search as soon as the game writes it again");
            }
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.weak("No luck? Try another number type, or");
                ui.hyperlink_to("ask for this game", REQUEST_GAME);
            });
        });
    }

    // ------------------------------------------------------------ refine when the game saves

    /// The file's modified time (ms) and size, for noticing when the game writes it again.
    #[cfg(not(target_arch = "wasm32"))]
    fn file_stamp(p: &Path) -> Option<(u128, u64)> {
        let m = std::fs::metadata(p).ok()?;
        let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis();
        Some((t, m.len()))
    }

    /// With "Refine by itself" on, looks at the file about once a second and refines once the
    /// game has finished writing a new copy.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn poll_auto_refine(&mut self, ctx: &egui::Context, now: f64) {
        let scanning = self.hex.searched && (self.hex.all_locations || !self.hex.results.is_empty());
        let Some(path) = self.doc.as_ref().filter(|d| matches!(d.content, Content::Binary(_))).map(|d| d.path.clone()) else { return };
        if !self.hex.auto_refine || !scanning {
            self.hex.watch = None;
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        if now - self.hex.watch_checked < 1.0 {
            return;
        }
        self.hex.watch_checked = now;
        let Some(stamp) = Self::file_stamp(&path) else { return };
        let Some(seen) = self.hex.watch else {
            self.hex.watch = Some(stamp);
            return;
        };
        if stamp == seen {
            return;
        }
        // Wait until the game has finished writing: the time must be at least a second old.
        let age = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0).saturating_sub(stamp.0);
        if age < 1000 {
            return;
        }
        self.hex.watch = Some(stamp);
        if self.dirty {
            self.hex.auto_refine = false;
            return self.fail("The game saved, but you have unsaved changes here, so refining stopped. Revert or save them, then refine.");
        }
        self.refine_from_disk();
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn poll_auto_refine(&mut self, _ctx: &egui::Context, _now: f64) {}
}
