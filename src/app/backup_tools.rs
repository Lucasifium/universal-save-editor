//! The Save manager's tools beyond single backups: backing up and restoring the whole save
//! folder, checking that every backup still reads, moving all backups of a save as one zip,
//! and the "Backup space" window (how much room backups take, cleaning up old automatic ones
//! and an optional size limit).

#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use super::*;
use crate::backups::{self, folders, space, Version};
use std::sync::mpsc::Receiver;

/// What a check found for one backup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    Good(String),
    /// Reads, but not like the open save (an older version of the game can do that too).
    Odd(String),
    Bad(String),
}

/// The "Check backups" run: results arrive from a worker thread one by one.
struct Check {
    rx: Receiver<(PathBuf, Verdict)>,
    total: usize,
}

/// Room freed by "Delete automatic backups older than" or "Apply the limit now".
#[derive(Clone, Copy, PartialEq, Eq)]
enum Clean {
    Age(u64),
    Cap,
}

#[derive(Default)]
pub(super) struct Tools {
    /// The save the cached details below are for.
    for_path: Option<PathBuf>,
    /// What a whole-folder backup of the save's folder would take, or why it can't be made.
    folder: Option<Result<folders::Contents, String>>,
    folder_name: String,
    /// A folder backup waiting for its Restore (`true`) or Delete to be confirmed.
    folder_confirm: Option<(PathBuf, bool)>,
    check: Option<Check>,
    verdicts: HashMap<PathBuf, Verdict>,
    pub space_open: bool,
    survey: Option<space::Survey>,
    clean_days: u64,
    clean_confirm: Option<Clean>,
}

/// What the player pressed in the Save manager's tools.
pub(super) enum Act {
    Check,
    Export,
    Import,
    Space,
    FolderBackup(String),
    FolderRestore(PathBuf),
    FolderDelete(PathBuf),
}

const AGES: [u64; 5] = [7, 30, 90, 180, 365];

fn size(n: u64) -> String {
    human_size(usize::try_from(n).unwrap_or(usize::MAX))
}

fn age_text(days: u64) -> String {
    trf("{0} days", &[&days])
}

fn cap_text(gb: u64) -> String {
    if gb == 0 {
        tr("No limit").into()
    } else {
        trf("{0} GB", &[&gb])
    }
}

/// Checks one backup of a file: it reads, and decodes the way the open save does.
fn check_file(path: &Path, opts: &formats::Options, want: Option<&(Vec<String>, String)>) -> Verdict {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return Verdict::Bad(trf("Can't be read: {0}", &[&e])),
    };
    if bytes.is_empty() {
        return Verdict::Bad(tr("The file is empty.").into());
    }
    let Some((layers, kind)) = want else { return Verdict::Good(tr("Reads fine").into()) };
    let decoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| formats::decode_with(&bytes, opts)));
    match decoded {
        Ok(d) if d.layers.iter().map(Layer::name).collect::<Vec<_>>() == *layers && d.content.kind() == *kind => Verdict::Good(tr("Reads fine").into()),
        Ok(_) => Verdict::Odd(tr("Reads differently from the open save. It may be damaged, or from another version of the game.").into()),
        Err(_) => Verdict::Bad(tr("Couldn't be read as a save.").into()),
    }
}

impl Tools {
    /// Clears what was worked out for another save.
    fn for_save(&mut self, path: &Path) {
        if self.for_path.as_deref() != Some(path) {
            let open = self.space_open;
            let survey = self.survey.take();
            *self = Tools { for_path: Some(path.to_path_buf()), space_open: open, survey, clean_days: 90, ..Default::default() };
        }
    }

    /// Shows what the check found for one backup, beside its row.
    pub(super) fn mark(&self, ui: &mut egui::Ui, p: &Path) {
        let Some(v) = self.verdicts.get(p) else { return };
        let (text, c, tip) = match v {
            Verdict::Good(t) => (tr("Reads fine"), theme::OK, t),
            Verdict::Odd(t) => (tr("Check this one"), theme::WARN, t),
            Verdict::Bad(t) => (tr("Damaged"), theme::ERR, t),
        };
        ui.label(RichText::new(text).color(theme::fg(ui, c))).on_hover_text(tip.as_str());
    }

    /// The tools under the list of backups in the Save manager.
    pub(super) fn ui(&mut self, ui: &mut egui::Ui, path: &Path, versions: &[Version], dirty: bool) -> Option<Act> {
        self.for_save(path);
        let mut act = None;
        let bytes: u64 = versions.iter().map(|v| v.size).sum();
        if !versions.is_empty() {
            ui.weak(trf("This save's backups take {0} ({1} backups).", &[&size(bytes), &versions.len()]));
        }
        ui.horizontal_wrapped(|ui| {
            let checking = self.check.is_some();
            let label = match &self.check {
                Some(c) => trf("Checking\u{2026} {0} of {1}", &[&self.verdicts.len().min(c.total), &c.total]),
                None => tr("Check backups").into(),
            };
            let any = !versions.is_empty() || self.folder_list(path).len() > 0;
            if ui.add_enabled(any && !checking, egui::Button::new(label)).on_hover_text(tr("Reads every backup of this save to make sure it can still be restored. Nothing is changed.")).clicked() {
                act = Some(Act::Check);
            }
            if ui.add_enabled(any, egui::Button::new(tr("Export backups\u{2026}"))).on_hover_text(tr("Puts every backup of this save into one zip file, to keep a copy elsewhere or move them to another computer")).clicked() {
                act = Some(Act::Export);
            }
            if ui.button(tr("Import backups\u{2026}")).on_hover_text(tr("Adds the backups from a zip made by Export backups. Nothing is replaced, and the save itself is not changed.")).clicked() {
                act = Some(Act::Import);
            }
            if ui.button(tr("Backup space\u{2026}")).on_hover_text(tr("How much room all backups take, and ways to free some")).clicked() {
                act = Some(Act::Space);
            }
        });
        ui.add_space(6.0);
        let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()).map(Path::to_path_buf) else { return act };
        let list = self.folder_list(path);
        let title = if list.is_empty() { tr("Whole save folder").to_string() } else { trf("Whole save folder ({0} backups)", &[&list.len()]) };
        egui::CollapsingHeader::new(title).id_salt("manager_folder").show(ui, |ui| {
            if self.folder.is_none() {
                self.folder = Some(folders::contents(&dir));
            }
            match self.folder.as_ref() {
                Some(Ok(c)) => {
                    ui.label(trf("For games that keep a save as several files: backs up every file in {0} as one backup ({1} files, {2}). Restoring puts the folder back exactly as it was.", &[&dir.display(), &c.files.len(), &size(c.bytes)]));
                    ui.weak(tr("These backups are kept in the editor's own backups folder, never in the save folder."));
                    ui.horizontal_wrapped(|ui| {
                        ui.add(TextEdit::singleline(&mut self.folder_name).hint_text(tr("backup name, e.g. before the boss")).desired_width(fit(ui.ctx(), 260.0)));
                        let ready = !self.folder_name.trim().is_empty() && !c.files.is_empty();
                        if ui.add_enabled(ready, egui::Button::new(tr("Back up the whole folder"))).on_disabled_hover_text(tr("Type a name for the backup first")).clicked() {
                            act = Some(Act::FolderBackup(self.folder_name.clone()));
                        }
                    });
                }
                Some(Err(e)) => {
                    ui.weak(trf("This folder can't be backed up as a whole: {0}.", &[e]));
                }
                None => {}
            }
            if list.is_empty() {
                return;
            }
            ScrollArea::vertical().show_list_fit(ui, "manager_folder_list", 200.0, |ui| {
            for v in &list {
                ui.horizontal_wrapped(|ui| {
                    let when = backups::describe(v.when);
                    ui.label(match &v.name {
                        Some(n) => format!("\u{201C}{n}\u{201D} \u{00B7} {when} ({})", size(v.size)),
                        None => format!("{when} ({})", size(v.size)),
                    });
                    self.mark(ui, &v.path);
                    match self.folder_confirm.clone() {
                        Some((p, is_restore)) if p == v.path => {
                            let yes = match (is_restore, dirty) {
                                (true, true) => tr("Restore and drop unsaved changes"),
                                (true, false) => tr("Restore now"),
                                _ => tr("Delete permanently"),
                            };
                            if ui.button(yes).clicked() {
                                act = Some(if is_restore { Act::FolderRestore(p) } else { Act::FolderDelete(p) });
                                self.folder_confirm = None;
                            }
                            if safe_choice(ui.button(tr("Cancel"))).clicked() {
                                self.folder_confirm = None;
                            }
                        }
                        _ => {
                            if crate::widgets::small_button(ui, tr("Restore this backup")).on_hover_text(tr("Puts the whole folder back as it was in this backup. The folder as it is now is backed up first, so you can undo it.")).clicked() {
                                self.folder_confirm = Some((v.path.clone(), true));
                            }
                            if crate::widgets::small_button(ui, tr("Delete backup")).on_hover_text(tr("Permanently removes this backup")).clicked() {
                                self.folder_confirm = Some((v.path.clone(), false));
                            }
                        }
                    }
                });
            }
            });
        });
        act
    }

    fn folder_list(&self, path: &Path) -> Vec<Version> {
        path.parent().filter(|d| !d.as_os_str().is_empty()).map(folders::list).unwrap_or_default()
    }

    /// Results of a running check, and its summary when it is done.
    fn poll(&mut self) -> Option<Result<String, String>> {
        let c = self.check.as_ref()?;
        loop {
            match c.rx.try_recv() {
                Ok((p, v)) => {
                    self.verdicts.insert(p, v);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
        let total = c.total;
        self.check = None;
        let bad = self.verdicts.values().filter(|v| matches!(v, Verdict::Bad(_))).count();
        let odd = self.verdicts.values().filter(|v| matches!(v, Verdict::Odd(_))).count();
        Some(match (bad, odd) {
            (0, 0) => Ok(trf("Checked {0} backups: every one reads fine.", &[&total])),
            _ => Err(trf("Checked {0} backups: {1} damaged, {2} that read differently from the open save. They are marked in the Save manager.", &[&total, &bad, &odd])),
        })
    }
}

impl App {
    /// Each frame: results of a running check, and the Backup space window.
    pub(super) fn backup_tools_frame(&mut self, ctx: &egui::Context) {
        if let Some(r) = self.backup_tools.poll() {
            match r {
                Ok(m) => self.ok(m),
                Err(m) => self.fail(m),
            }
        }
        if self.backup_tools.check.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.space_window(ctx);
    }

    /// Carries out what was pressed in the Save manager's tools, for the save at `path`.
    pub(super) fn backup_act(&mut self, act: Act, path: &Path) {
        match act {
            Act::Check => self.start_backup_check(path),
            Act::Export => {
                let stem = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or("save".into());
                self.ask(Ask::ExportBackups, None, Some(trf("{0} backups.zip", &[&stem])));
            }
            Act::Import => self.ask(Ask::ImportBackups, None, None),
            Act::Space => self.open_backup_space(),
            Act::FolderBackup(name) => {
                let Some(dir) = path.parent() else { return };
                match folders::take_named(dir, &name) {
                    Ok(_) => {
                        self.ok(trf("Backed up the whole folder as \u{201C}{0}\u{201D}.", &[&name.trim()]));
                        self.backup_tools.folder_name.clear();
                        self.backup_tools.folder = None;
                    }
                    Err(e) => self.fail(trf("Couldn't back up the folder: {0}. Nothing in it was changed.", &[&e])),
                }
            }
            Act::FolderRestore(archive) => self.restore_folder(path, &archive),
            Act::FolderDelete(archive) => {
                let v = path.parent().map(folders::list).unwrap_or_default().into_iter().find(|v| v.path == archive);
                match v.map(|v| backups::remove(&v)) {
                    Some(Ok(())) => self.ok(tr("Deleted the backup.")),
                    Some(Err(e)) => self.fail(trf("Couldn't delete the backup: {0}. It is still in the list.", &[&e])),
                    None => {}
                }
            }
        }
    }

    fn start_backup_check(&mut self, path: &Path) {
        if WEB {
            return;
        }
        let files: Vec<PathBuf> = backups::list(path).into_iter().map(|v| v.path).collect();
        let dirs: Vec<PathBuf> = path.parent().map(folders::list).unwrap_or_default().into_iter().map(|v| v.path).collect();
        let (opts, want) = match self.doc.as_ref().filter(|d| d.path == path) {
            // A file the editor doesn't understand: only that each backup reads is checked.
            Some(d) if matches!(d.content, Content::Binary(_)) && d.layers.is_empty() => (d.opts.clone(), None),
            Some(d) => (d.opts.clone(), Some((d.layers.iter().map(Layer::name).collect::<Vec<_>>(), d.content.kind()))),
            None => (formats::Options::for_path(path), None),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let total = files.len() + dirs.len();
        std::thread::spawn(move || {
            for p in files {
                let v = check_file(&p, &opts, want.as_ref());
                if tx.send((p, v)).is_err() {
                    return;
                }
            }
            for p in dirs {
                let v = match folders::check(&p) {
                    Ok(n) => Verdict::Good(trf("{0} files, all readable", &[&n])),
                    Err(e) => Verdict::Bad(e),
                };
                if tx.send((p, v)).is_err() {
                    return;
                }
            }
        });
        self.backup_tools.verdicts.clear();
        self.backup_tools.check = Some(Check { rx, total });
        self.info(tr("Checking backups\u{2026}"));
    }

    /// Puts the save's whole folder back from a folder backup, then opens the save again.
    fn restore_folder(&mut self, path: &Path, archive: &Path) {
        let Some(dir) = path.parent().map(Path::to_path_buf) else { return };
        if self.dirty && self.doc.as_ref().is_some_and(|d| d.path != path) {
            return self.fail(trf("Save or revert the changes to {0} first.", &[&self.doc.as_ref().map(|d| file_name(&d.path)).unwrap_or_default()]));
        }
        if let Some(game) = crate::running::game_for(path) {
            return self.fail(trf("{0} looks like it's running. Quit it first, or it may overwrite the restored save. Nothing was restored.", &[&game]));
        }
        let r = folders::restore(&dir, archive);
        self.backup_tools.folder = None;
        let reopen = |app: &mut App| {
            app.dirty = false;
            if path.is_file() {
                app.load(path);
            } else if app.doc.as_ref().is_some_and(|d| d.path == path) {
                // The backup has no such file: close it so its old data can't be saved back.
                app.doc = None;
            }
        };
        match r {
            Ok(done) => {
                reopen(self);
                self.ok(trf("Restored the whole folder ({0} files written, {1} removed). The folder as it was is backed up too, so you can switch back.", &[&done.written, &done.removed]));
            }
            Err(e) if e.unchanged => self.fail(trf("Couldn't restore the folder: {0}. Nothing in it was changed.", &[&e.error])),
            Err(e) => {
                reopen(self);
                let kept = e.before.map(|b| b.display().to_string()).unwrap_or_default();
                self.fail(trf("Couldn't restore the folder: {0}. Some files may have changed and couldn't be put back; the folder as it was before is kept in {1}.", &[&e.error, &kept]));
            }
        }
    }

    pub(super) fn export_backups(&mut self, dest: &Path) {
        let Some(save) = self.backup_tools.for_path.clone() else { return };
        // A file already there (another save, picked by mistake) is backed up before it's replaced.
        if dest.is_file() {
            if let Err(e) = backups::take(dest) {
                return self.fail(trf("{0} is already there and couldn't be backed up first ({1}), so it wasn't replaced.", &[&file_name(dest), &e]));
            }
        }
        match backups::bundle::export(&save, dest) {
            Ok(e) => self.ok(trf("Exported {0} backups ({1}) to {2}.", &[&(e.files + e.folders), &size(e.bytes), &file_name(dest)])),
            Err(e) => self.fail(trf("Couldn't export the backups: {0}.", &[&e])),
        }
    }

    pub(super) fn import_backups(&mut self, src: &Path) {
        let Some(save) = self.backup_tools.for_path.clone() else { return };
        match backups::bundle::import(&save, src) {
            Ok(i) => {
                self.backup_tools.survey = None;
                let other = !i.from.is_empty() && Some(i.from.as_str()) != save.file_name().and_then(|n| n.to_str());
                self.ok(if other {
                    trf("Imported {0} backups ({1} were already here). They were exported from {2}.", &[&i.added, &i.already, &i.from])
                } else {
                    trf("Imported {0} backups ({1} were already here).", &[&i.added, &i.already])
                });
            }
            Err(e) => self.fail(trf("Couldn't import the backups: {0}. The save wasn't changed.", &[&e])),
        }
    }

    pub(super) fn open_backup_space(&mut self) {
        self.backup_tools.space_open = true;
        self.backup_tools.survey = None;
        if self.backup_tools.clean_days == 0 {
            self.backup_tools.clean_days = 90;
        }
    }

    /// The saves whose backups the survey also looks for: the open one and recent files.
    fn survey_saves(&self) -> Vec<PathBuf> {
        self.doc.iter().map(|d| d.path.clone()).chain(self.settings.recent.iter().cloned()).collect()
    }

    fn space_window(&mut self, ctx: &egui::Context) {
        if !self.backup_tools.space_open || WEB {
            return;
        }
        if self.backup_tools.survey.is_none() {
            let saves = self.survey_saves();
            space::remember_existing(&saves);
            self.backup_tools.survey = Some(space::survey(&saves));
        }
        let mut open = true;
        let mut recount = false;
        let mut confirm_now = None;
        let mut cap_changed = None;
        let mut reveal = None;
        let cap_gb = self.settings.backup_cap_gb;
        let t = &mut self.backup_tools;
        let Some(s) = t.survey.as_ref() else { return };
        egui::Window::new(tr("Backup space")).open(&mut open).collapsible(false).default_width(fit(ctx, 600.0)).max_width(fit(ctx, 900.0)).show(ctx, |ui| {
            ui.label(RichText::new(trf("Backups take {0} in all: {1} backups of {2} saves.", &[&size(s.bytes), &s.files, &s.saves.len()])).strong());
            ui.weak(tr("Counted: every backups folder this app has written to, the backups of recent files, and the editor's own backups folder."));
            ui.add_space(4.0);
            if s.saves.is_empty() {
                ui.label(tr("No backups yet. One is made automatically each time you save."));
            } else {
                ScrollArea::vertical().show_list_fit(ui, "backup_space_list", 300.0, |ui| {
                    egui::Grid::new("backup_space_grid").num_columns(2).striped(true).spacing([16.0, 4.0]).show(ui, |ui| {
                        ui.strong(tr("Save"));
                        ui.strong(tr("Backups"));
                        ui.end_row();
                        for u in &s.saves {
                            let name = match u.kind {
                                space::Kind::File => u.name.clone(),
                                space::Kind::Folder => trf("{0} (whole folder)", &[&u.name]),
                            };
                            let tip = u.beside.as_ref().unwrap_or(&u.dir).display().to_string();
                            let r = ui
                                .scope(|ui| {
                                    ui.set_min_width(120.0);
                                    ui.set_max_width(if ui.ctx().screen_rect().width() < 600.0 { 140.0 } else { 300.0 });
                                    ui.add(egui::Label::new(name).wrap().sense(egui::Sense::click()))
                                })
                                .inner
                                .on_hover_text(trf("{0} (click to show the backups folder)", &[&tip]));
                            if r.clicked() {
                                reveal = Some(u.dir.clone());
                            }
                            ui.label(format!("{} \u{00B7} {}", trf("{0} automatic, {1} named", &[&u.automatic(), &u.named()]), size(u.bytes)));
                            ui.end_row();
                        }
                    });
                });
            }
            if ui.button(tr("Count again")).clicked() {
                recount = true;
            }
            ui.separator();
            ui.label(RichText::new(tr("Free up room")).strong());
            ui.weak(tr("Only automatic backups are deleted. Named backups, and the newest automatic backup of every save, are always kept."));
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Delete automatic backups older than"));
                egui::ComboBox::from_id_salt("clean_days").selected_text(age_text(t.clean_days)).show_ui(ui, |ui| {
                    for d in AGES {
                        if ui.selectable_label(t.clean_days == d, age_text(d)).clicked() {
                            t.clean_days = d;
                            t.clean_confirm = None;
                        }
                    }
                });
                if ui.button(tr("Delete\u{2026}")).clicked() {
                    t.clean_confirm = Some(Clean::Age(t.clean_days));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Limit all backups to"));
                egui::ComboBox::from_id_salt("backup_cap").selected_text(cap_text(cap_gb)).show_ui(ui, |ui| {
                    for gb in space::CAP_CHOICES_GB {
                        if ui.selectable_label(cap_gb == gb, cap_text(gb)).clicked() {
                            cap_changed = Some(gb);
                        }
                    }
                });
            });
            ui.weak(tr("When a new backup passes the limit, the oldest automatic backups of any save are deleted first."));
            let over = space::over_cap(s, cap_gb.saturating_mul(space::GB), &[]);
            if !over.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(trf("{0} over the limit now.", &[&size(s.bytes - cap_gb.saturating_mul(space::GB))])).color(theme::fg(ui, theme::WARN)));
                    if ui.button(tr("Apply the limit now\u{2026}")).clicked() {
                        t.clean_confirm = Some(Clean::Cap);
                    }
                });
            }
            if let Some(c) = t.clean_confirm {
                let list = match c {
                    Clean::Age(d) => space::older_than(s, d),
                    Clean::Cap => over,
                };
                ui.add_space(4.0);
                if list.is_empty() {
                    ui.label(tr("No automatic backups are that old."));
                } else {
                    let bytes: u64 = list.iter().map(|v| v.size).sum();
                    ui.label(RichText::new(trf("This deletes {0} automatic backups ({1}). They can't be brought back.", &[&list.len(), &size(bytes)])).color(theme::fg(ui, theme::WARN)));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Delete them")).clicked() {
                            confirm_now = Some(c);
                        }
                        if safe_choice(ui.button(tr("Cancel"))).clicked() {
                            t.clean_confirm = None;
                        }
                    });
                }
            }
        });
        if let Some(c) = confirm_now {
            let s = t.survey.take().unwrap_or_default();
            let list = match c {
                Clean::Age(d) => space::older_than(&s, d),
                Clean::Cap => space::over_cap(&s, cap_gb.saturating_mul(space::GB), &[]),
            };
            let want = list.len();
            let (n, bytes) = space::remove(&list);
            t.clean_confirm = None;
            if n == want {
                self.ok(trf("Deleted {0} automatic backups and freed {1}.", &[&n, &size(bytes)]));
            } else {
                self.fail(trf("Deleted {0} of {1} automatic backups and freed {2}. The others couldn't be deleted and are still there.", &[&n, &want, &size(bytes)]));
            }
        }
        if let Some(gb) = cap_changed {
            self.settings.backup_cap_gb = gb;
            space::set_cap(gb.saturating_mul(space::GB));
            self.settings.save();
        }
        if recount {
            self.backup_tools.survey = None;
        }
        if let Some(d) = reveal {
            platform::reveal(&d);
        }
        if !open {
            self.backup_tools.space_open = false;
            self.backup_tools.clean_confirm = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_check_tells_a_good_backup_from_a_damaged_or_odd_one() {
        let dir = std::env::temp_dir().join(format!("use-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("good.sebak");
        std::fs::write(&good, br#"{"gold": 5, "level": 2}"#).unwrap();
        let opts = formats::Options::for_path(&dir.join("save.json"));
        let d = formats::decode_with(&std::fs::read(&good).unwrap(), &opts);
        let want = (d.layers.iter().map(Layer::name).collect::<Vec<_>>(), d.content.kind());
        assert_eq!(check_file(&good, &opts, Some(&want)), Verdict::Good(tr("Reads fine").into()));
        // Cut off half way: no longer the same kind of data.
        let cut = dir.join("cut.sebak");
        std::fs::write(&cut, br#"{"gold": 5, "le"#).unwrap();
        assert!(matches!(check_file(&cut, &opts, Some(&want)), Verdict::Odd(_)));
        let empty = dir.join("empty.sebak");
        std::fs::write(&empty, b"").unwrap();
        assert!(matches!(check_file(&empty, &opts, Some(&want)), Verdict::Bad(_)));
        assert!(matches!(check_file(&dir.join("gone.sebak"), &opts, Some(&want)), Verdict::Bad(_)));
        // A format the editor doesn't read: only that the file reads is checked.
        assert!(matches!(check_file(&cut, &opts, None), Verdict::Good(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
