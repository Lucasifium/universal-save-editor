//! The "Save changes?" dialog: the last look before a save is written. Each change has a tick
//! box; a change unticked there is put back before saving (Undo brings it back afterwards).

use super::*;

/// What putting a line back means.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Undo {
    /// A value, put back to the file's.
    Value(String),
    /// A run of bytes `start..end`.
    Bytes(usize, usize),
    /// Can't be dropped on its own (a text or Raw JSON edit): saved or cancelled as a whole.
    Whole,
}

/// One line of the dialog.
pub(super) struct Line {
    pub text: String,
    /// A risk note, shown under the first line it applies to.
    pub note: Option<&'static str>,
    /// The section the value is in, for grouping long lists.
    pub group: String,
    /// Values one button changed together are kept or dropped together.
    pub together: Option<u64>,
    pub undo: Undo,
    pub keep: bool,
}

/// Lists longer than this are grouped by section.
const GROUP_FROM: usize = 8;

impl App {
    /// The unsaved changes, one line each, for the dialog.
    pub(super) fn review_lines(&self) -> Vec<Line> {
        let Some(d) = &self.doc else { return vec![] };
        let line = |text: String, undo: Undo| Line { text, note: None, group: String::new(), together: None, undo, keep: true };
        match (&d.content, &d.baseline) {
            (Content::Binary(a), Content::Binary(b)) => byte_ranges(a, b)
                .into_iter()
                .map(|(s, e)| {
                    let name = self.hex.labels.get(&s).map(|l| l.name.clone()).unwrap_or(format!("Offset 0x{s:X}"));
                    let text = if e - s == 1 { trf("{0}: 1 byte changed", &[&name]) } else { trf("{0}: {1} bytes changed", &[&name, &(e - s)]) };
                    // Bytes added or removed (a patch) can't be put back on their own.
                    let undo = if e <= a.len() && e <= b.len() && a.len() == b.len() { Undo::Bytes(s, e) } else { Undo::Whole };
                    line(text, undo)
                })
                .collect(),
            (Content::Text(a), Content::Text(b)) => {
                if a != b {
                    vec![line(tr("The text was edited").into(), Undo::Whole)]
                } else {
                    vec![]
                }
            }
            (c, b) => {
                let (Some(cv), Some(bv)) = (root(c), root(b)) else { return vec![] };
                let kind = match &d.content {
                    Content::Tree(t) => t.kind_name.as_str(),
                    _ => "JSON",
                };
                let mut noted = HashSet::new();
                self.touched
                    .iter()
                    .filter(|p| cv.pointer(p) != bv.pointer(p))
                    .map(|p| {
                        if p.is_empty() {
                            return line(tr("Whole document (Raw JSON edit)").into(), Undo::Whole);
                        }
                        let (name, context, group) = match self.model.by_ptr.get(p.as_str()) {
                            Some(&i) => {
                                let r = &self.model.rows[i];
                                (r.name().to_string(), r.context().to_string(), self.model.sections[r.section].title.clone())
                            }
                            None => (p.clone(), String::new(), String::new()),
                        };
                        let old = bv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                        let new = cv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                        let at = if context.is_empty() { name } else { format!("{context} \u{203A} {name}") };
                        Line {
                            text: format!("{at}: {old} -> {new}"),
                            // Each note once, on the first value it applies to.
                            note: crate::risks::note(kind, p).filter(|n| noted.insert(*n)),
                            group,
                            together: self.touched.group.get(p).copied(),
                            // A value the file didn't have can't be put back on its own.
                            undo: if bv.pointer(p).is_some() { Undo::Value(p.clone()) } else { Undo::Whole },
                            keep: true,
                        }
                    })
                    .collect()
            }
        }
    }

    /// Ticks or unticks line `i`, and every line changed together with it.
    fn tick_review_line(lines: &mut [Line], i: usize) {
        let keep = !lines[i].keep;
        let together = lines[i].together;
        for (j, l) in lines.iter_mut().enumerate() {
            if j == i || (together.is_some() && l.together == together) {
                l.keep = keep;
            }
        }
    }

    /// Puts back the changes unticked in the dialog. Returns how many lines were dropped.
    pub(super) fn drop_unticked(&mut self, lines: &[Line]) -> usize {
        let base = self.doc.as_ref().map(|d| &d.baseline);
        let mut values = vec![];
        let mut bytes = vec![];
        for l in lines.iter().filter(|l| !l.keep) {
            match (&l.undo, base) {
                (Undo::Value(p), Some(b)) => {
                    if let Some(v) = root(b).and_then(|r| r.pointer(p)) {
                        values.push((p.clone(), v.clone()));
                    }
                }
                (Undo::Bytes(s, e), Some(Content::Binary(b))) if *e <= b.len() => bytes.push((*s, b[*s..*e].to_vec())),
                _ => {}
            }
        }
        let n = values.len() + bytes.len();
        let start = self.undo.len();
        // Its own Undo step, not folded into the edit it puts back.
        self.last_edit = None;
        self.apply_edits(values, f64::NEG_INFINITY);
        for (o, b) in bytes {
            self.apply_bytes(o, b);
        }
        // One Undo brings back everything that was dropped.
        self.group_undo_from(start);
        n
    }

    /// "Save" in the dialog: puts back what was unticked, then saves the rest.
    fn save_reviewed(&mut self) {
        let Some(r) = self.save_review.take() else { return };
        let dropped = self.drop_unticked(&r.lines);
        if !self.dirty {
            return self.info(tr("Nothing left to save: the unticked changes were put back, and the file wasn't changed. Undo brings them back."));
        }
        if dropped > 0 {
            // Said first: the save's own message follows once it is written.
            self.info(if dropped == 1 { tr("1 unticked change was put back and not saved; Undo brings it back.").to_string() } else { trf("{0} unticked changes were put back and not saved; Undo brings them back.", &[&dropped]) });
        }
        self.save_to(&r.path);
    }

    pub(super) fn save_review_dialog(&mut self, ctx: &egui::Context) {
        let Some(r) = &mut self.save_review else { return };
        let mut choice = None;
        let mut dont_ask = !self.settings.confirm_save;
        let mut tick = None;
        let (mut all, mut none) = (false, false);
        // Lines without a tick box (a Raw JSON edit, bytes added or removed) are always saved.
        let kept = r.lines.iter().filter(|l| l.keep).count();
        modal(ctx, tr("Save changes?"), |ui| {
            ui.set_max_width(520.0);
            if let Some(game) = &r.running {
                theme::card(ui, |ui| {
                    ui.label(RichText::new(trf("{0} looks like it's running.", &[game])).color(theme::fg(ui, theme::WARN)).strong());
                    ui.label(tr("Games keep the save in memory and write it back when they autosave or quit, which would undo these changes. Quit the game completely (not just to the title screen), then save."));
                });
                ui.add_space(6.0);
            }
            ui.label(if r.lines.len() == 1 { trf("{0} change to {1}:", &[&r.lines.len(), &file_name(&r.path)]) } else { trf("{0} changes to {1}:", &[&r.lines.len(), &file_name(&r.path)]) });
            let droppable = r.lines.iter().any(|l| l.undo != Undo::Whole);
            if droppable {
                ui.horizontal_wrapped(|ui| {
                    ui.weak(tr("Untick a change to leave it out of this save."));
                    if kept < r.lines.len() && ui.small_button(tr("Tick all")).clicked() {
                        all = true;
                    }
                    if kept > 0 && r.lines.len() > 1 && ui.small_button(tr("Untick all")).clicked() {
                        none = true;
                    }
                });
            }
            let grouped = r.lines.len() > GROUP_FROM && r.lines.iter().any(|l| l.group != r.lines[0].group);
            ScrollArea::vertical().show_list_fit(ui, "app_list1", 280.0, |ui| {
                // Lines of one section together, in the order the sections first come up.
                let mut order: Vec<usize> = (0..r.lines.len()).collect();
                if grouped {
                    let mut first: Vec<&str> = vec![];
                    for l in &r.lines {
                        if !first.contains(&l.group.as_str()) {
                            first.push(&l.group);
                        }
                    }
                    order.sort_by_key(|&i| first.iter().position(|g| *g == r.lines[i].group));
                }
                let mut last_group: Option<&str> = None;
                for &i in &order {
                    let l = &r.lines[i];
                    if grouped && last_group != Some(l.group.as_str()) {
                        ui.add_space(4.0);
                        ui.label(RichText::new(if l.group.is_empty() { tr("Other") } else { &l.group }).strong());
                        last_group = Some(&l.group);
                    }
                    if l.undo == Undo::Whole || !droppable {
                        ui.label(format!("\u{2022} {}", l.text));
                    } else {
                        let mut on = l.keep;
                        let resp = widgets::checkbox(ui, &mut on, &l.text);
                        let resp = if l.together.is_some() { resp.on_hover_text(tr("Made by one button together with other values: kept or left out together.")) } else { resp };
                        if resp.clicked() {
                            tick = Some(i);
                        }
                    }
                    if let Some(note) = l.note {
                        ui.indent(("review_note", i), |ui| {
                            ui.label(RichText::new(trf("Note: {0}", &[&crate::i18n::tr_any(note)])).small().color(theme::fg(ui, theme::WARN)));
                        });
                    }
                }
                if r.lines.is_empty() {
                    ui.weak(tr("(nothing that the list can describe)"));
                }
            });
            ui.add_space(4.0);
            if let Some(c) = crate::cloud::service_for(&r.path) {
                ui.weak(trf("This save is in a folder that {0} syncs. If it asks which copy to keep, choose the local one.", &[&c]));
            }
            if kept < r.lines.len() {
                ui.weak(if kept == 0 { tr("Nothing is ticked, so saving puts every change back and writes nothing.") } else { tr("Unticked changes are put back before saving. Undo brings them back.") });
            }
            #[cfg(target_arch = "wasm32")]
            let web_note = super::in_place::save_note(self.doc.as_ref().is_some_and(|d| d.handle.is_some() && d.path == r.path));
            #[cfg(not(target_arch = "wasm32"))]
            let web_note = tr("Your browser downloads the edited file. Keep a copy of the original before you replace it.");
            ui.weak(if WEB {
                web_note
            } else {
                tr("The save on disk now is backed up first. Backups \u{203A} Quick restore brings it back.")
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let label = if kept == 0 && !r.lines.is_empty() {
                    tr("Put them back")
                } else if r.running.is_some() {
                    tr("Save anyway")
                } else if kept < r.lines.len() {
                    tr("Save ticked")
                } else {
                    tr("Save")
                };
                let first = ui.add(egui::Button::new(RichText::new(label).color(Color32::WHITE)).fill(theme::ACCENT));
                if first.clicked() {
                    choice = Some(0);
                }
                if r.running.is_some() && ui.button(tr("Check again")).clicked() {
                    choice = Some(1);
                }
                if safe_choice(ui.button(tr("Cancel"))).clicked() {
                    choice = Some(2);
                }
                let last = crate::widgets::checkbox(ui, &mut dont_ask, tr("Don't show this list again"));
                tab_ring(&first, &last);
            });
        });
        if let Some(i) = tick {
            Self::tick_review_line(&mut r.lines, i);
        }
        if all || none {
            // Lines without a tick box are always saved.
            for l in r.lines.iter_mut().filter(|l| l.undo != Undo::Whole) {
                l.keep = all;
            }
        }
        if dont_ask == self.settings.confirm_save {
            self.settings.confirm_save = !dont_ask;
            self.settings.save();
        }
        match choice {
            Some(0) => self.save_reviewed(),
            Some(1) => {
                let Some(r) = self.save_review.as_mut() else { return };
                r.running = crate::running::game_for(&r.path);
                self.running = r.running.clone();
                if r.running.is_none() && !self.settings.confirm_save {
                    let Some(r) = self.save_review.take() else { return };
                    self.drop_unticked(&r.lines);
                    if self.dirty {
                        self.save_to(&r.path);
                    }
                }
            }
            Some(_) => self.save_review = None,
            None => {}
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    fn l(together: Option<u64>) -> Line {
        Line { text: String::new(), note: None, group: String::new(), together, undo: Undo::Value(String::new()), keep: true }
    }

    #[test]
    fn values_changed_together_are_ticked_together() {
        let mut lines = vec![l(None), l(Some(1)), l(Some(1)), l(Some(2))];
        App::tick_review_line(&mut lines, 1);
        assert_eq!(lines.iter().map(|l| l.keep).collect::<Vec<_>>(), [true, false, false, true]);
        App::tick_review_line(&mut lines, 0);
        assert_eq!(lines.iter().map(|l| l.keep).collect::<Vec<_>>(), [false, false, false, true]);
        App::tick_review_line(&mut lines, 2);
        assert_eq!(lines.iter().map(|l| l.keep).collect::<Vec<_>>(), [false, true, true, true]);
    }

    /// A save in a fresh folder, with backups beside it and English messages, for one test.
    pub(in crate::app) struct Fixture {
        pub dir: PathBuf,
        pub path: PathBuf,
        pub ctx: egui::Context,
        _locks: (std::sync::MutexGuard<'static, ()>, std::sync::MutexGuard<'static, ()>),
    }

    impl Fixture {
        pub fn new(tag: &str, bytes: &[u8]) -> Self {
            let b = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
            crate::backups::set_root(None);
            let lang = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            crate::i18n::set(Some("en"));
            let dir = std::env::temp_dir().join(format!("use-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("save.json");
            std::fs::write(&path, bytes).unwrap();
            let ctx = egui::Context::default();
            theme::install_fonts(&ctx);
            Fixture { dir, path, ctx, _locks: (b, lang) }
        }

        pub fn app(&self) -> App {
            let mut app = App::with(&self.ctx, Settings::default(), false);
            app.load(&self.path);
            assert!(app.doc.is_some(), "{}", app.status);
            app
        }

        pub fn on_disk(&self) -> Value {
            serde_json::from_slice(&std::fs::read(&self.path).unwrap()).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn val(app: &App, ptr: &str) -> Value {
        app.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(ptr)).cloned().unwrap_or(Value::Null)
    }

    #[test]
    fn an_unticked_change_is_put_back_and_not_saved_and_undo_brings_it_back() {
        let f = Fixture::new("review-untick", br#"{"gold": 100, "level": 3, "name": "Ann"}"#);
        let mut app = f.app();
        app.apply_edit("/gold".into(), json!(999), 0.0);
        app.apply_edit("/level".into(), json!(9), 10.0);
        app.save_clicked();
        let lines = &mut app.save_review.as_mut().expect("the dialog opens").lines;
        assert_eq!(lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["Gold: 100 -> 999", "Level: 3 -> 9"]);
        lines[1].keep = false;
        app.save_reviewed();
        assert!(app.saving.is_none() && app.save_review.is_none());
        assert_eq!(f.on_disk(), json!({"gold": 999, "level": 3, "name": "Ann"}), "{}", app.status);
        assert_eq!(val(&app, "/level"), json!(3));
        assert!(!app.dirty);
        // The original is kept as a backup.
        assert!(!crate::backups::list(&f.path).is_empty());
        app.undo_redo(true);
        assert_eq!(val(&app, "/level"), json!(9));
        assert!(app.dirty);
    }

    #[test]
    fn with_nothing_ticked_nothing_is_written() {
        let f = Fixture::new("review-none", br#"{"gold": 100, "level": 3}"#);
        let before = std::fs::read(&f.path).unwrap();
        let mut app = f.app();
        app.apply_edits(vec![("/gold".into(), json!(5)), ("/level".into(), json!(50))], 0.0);
        app.save_clicked();
        let lines = &mut app.save_review.as_mut().unwrap().lines;
        // One button changed both: unticking one unticks the other.
        App::tick_review_line(lines, 0);
        assert!(lines.iter().all(|l| !l.keep));
        app.save_reviewed();
        assert_eq!(std::fs::read(&f.path).unwrap(), before);
        assert!(crate::backups::list(&f.path).is_empty());
        assert!(!app.dirty && app.status.starts_with("Nothing left to save"), "{}", app.status);
        assert_eq!((val(&app, "/gold"), val(&app, "/level")), (json!(100), json!(3)));
    }

    #[test]
    fn unticked_bytes_are_put_back_too() {
        let f = Fixture::new("review-bytes", &[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
        let mut app = App::with(&f.ctx, Settings::default(), false);
        let bin = f.dir.join("save.bin");
        std::fs::rename(&f.path, &bin).unwrap();
        app.load(&bin);
        assert!(matches!(app.doc.as_ref().unwrap().content, Content::Binary(_)));
        app.apply_bytes(2, vec![0xAA]);
        app.apply_bytes(10, vec![0xBB, 0xCC]);
        app.save_clicked();
        let lines = &mut app.save_review.as_mut().unwrap().lines;
        assert_eq!(lines.len(), 2);
        lines[0].keep = false;
        app.save_reviewed();
        let mut want: Vec<u8> = (0..20).collect();
        want[10] = 0xBB;
        want[11] = 0xCC;
        assert_eq!(std::fs::read(&bin).unwrap(), want, "{}", app.status);
    }
}
