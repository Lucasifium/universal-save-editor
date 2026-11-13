//! Several saves open at once, one per tab, and copying values between them.
//!
//! Only the open tab's state lives in `App`'s own fields; every other tab keeps the same
//! fields in a `Parked`, and switching tabs swaps the two. The rest of the app works on
//! "the open file" exactly as it did with one file.

use super::*;

macro_rules! parked {
    ($($f:ident: $t:ty),* $(,)?) => {
        /// The per-file state of a tab that isn't showing.
        #[derive(Default)]
        pub(super) struct Parked {
            $($f: $t,)*
        }

        impl App {
            fn swap_parked(&mut self, p: &mut Parked) {
                $(std::mem::swap(&mut self.$f, &mut p.$f);)*
            }
        }
    };
}

parked! {
    doc: Option<Doc>,
    dirty: bool,
    undo: Vec<Change>,
    redo: Vec<Change>,
    last_edit: Option<(String, f64)>,
    touched: Vec<String>,
    tab: Tab,
    search: String,
    search_typed_at: f64,
    model: Model,
    collapsed: HashSet<usize>,
    disp: Vec<Disp>,
    scroll_to_row: Option<usize>,
    raw_text: String,
    raw_error: Option<String>,
    hex: HexState,
    running: Option<String>,
    compare: Option<Compare>,
    patches: Option<Patches>,
    rebind: Option<(Vec<u64>, usize, String)>,
    save_review: Option<SaveReview>,
    copy_slots: Option<CopySlots>,
    sum_hits: Option<Vec<crate::fmt::sumfind::Sum>>,
    password_prompt: Option<String>,
    doc_note: String,
    pending_close: bool,
    bulk: Option<Bulk>,
    preset_save: Option<PresetSave>,
    preset_apply: Option<PresetApply>,
    disk_changed: bool,
    disk_seen: Option<Stamp>,
    stale_save: Option<PathBuf>,
    stale_ok: bool,
}

impl Default for Tab {
    fn default() -> Self {
        Tab::Values
    }
}

/// Values copied from a save, waiting to be pasted into another (or the same) one.
pub(super) struct Clip {
    /// What was copied ("Gold", "Inventory (42 values)").
    pub what: String,
    /// The save it came from.
    pub from: String,
    pub values: Vec<(String, Value)>,
}

/// Tabs and the clipboard, kept in `App`.
#[derive(Default)]
pub(super) struct Tabs {
    /// One entry per tab; the entry at `active` is an empty stand-in (its state is in `App`).
    list: Vec<Parked>,
    pub active: usize,
    pub clip: Option<Clip>,
}

impl Tabs {
    pub fn new() -> Self {
        Self { list: vec![Parked::default()], ..Default::default() }
    }
}

impl App {
    pub(super) fn tab_count(&self) -> usize {
        self.tabs.list.len()
    }

    /// Any tab, shown or not, with unsaved changes.
    pub(super) fn any_dirty(&self) -> bool {
        self.dirty || self.tabs.list.iter().enumerate().any(|(i, p)| i != self.tabs.active && p.dirty)
    }

    fn tab_path(&self, i: usize) -> Option<&Path> {
        if i == self.tabs.active {
            self.doc.as_ref().map(|d| d.path.as_path())
        } else {
            self.tabs.list[i].doc.as_ref().map(|d| d.path.as_path())
        }
    }

    fn tab_dirty(&self, i: usize) -> bool {
        if i == self.tabs.active {
            self.dirty
        } else {
            self.tabs.list[i].dirty
        }
    }

    pub(super) fn switch_tab(&mut self, i: usize) {
        if i == self.tabs.active || i >= self.tabs.list.len() {
            return;
        }
        let mut list = std::mem::take(&mut self.tabs.list);
        self.swap_parked(&mut list[self.tabs.active]);
        self.swap_parked(&mut list[i]);
        self.tabs.list = list;
        self.tabs.active = i;
        // Cached lists of rows were worked out for the other file.
        self.disp_key = None;
        self.model_gen += 1;
        self.collapsed_gen += 1;
        self.edit_gen += 1;
        self.last_edit = None;
        if self.tab == Tab::Raw {
            self.sync_raw();
        }
        if let Some(d) = &self.doc {
            let name = file_name(&d.path);
            self.info(format!("Showing {name}."));
        }
    }

    /// Adds an empty tab after the open one and shows it.
    pub(super) fn new_tab(&mut self) {
        let at = self.tabs.active + 1;
        self.tabs.list.insert(at, Parked::default());
        self.switch_tab(at);
    }

    /// Opens `path` in a tab: the tab that already has it, the open tab if it's empty,
    /// otherwise a new one.
    pub(super) fn open_in_tab(&mut self, path: PathBuf) {
        if let Some(i) = (0..self.tab_count()).find(|&i| i != self.tabs.active && self.tab_path(i) == Some(path.as_path())) {
            self.switch_tab(i);
            return;
        }
        if self.doc.is_some() && self.doc.as_ref().is_some_and(|d| d.path != path) {
            self.new_tab();
        }
        if self.dirty {
            self.pending_open = Some(path);
        } else {
            self.load_async(&path);
        }
    }

    /// Closes tab `i`, asking first if it has unsaved changes.
    pub(super) fn close_tab(&mut self, i: usize) {
        self.switch_tab(i);
        if self.dirty {
            // Asks "save before closing?" and then closes it (`close_file`).
            self.pending_close = true;
        } else {
            self.close_file();
        }
    }

    pub(super) fn drop_tab(&mut self, i: usize) {
        if self.tab_count() == 1 {
            // The last tab: back to the welcome page.
            let mut blank = Parked::default();
            self.swap_parked(&mut blank);
            self.disp_key = None;
            self.model_gen += 1;
            return;
        }
        if i == self.tabs.active {
            self.switch_tab(if i + 1 < self.tab_count() { i + 1 } else { i - 1 });
        }
        self.tabs.list.remove(i);
        if self.tabs.active > i {
            self.tabs.active -= 1;
        }
    }

    /// The row of tabs above the file, shown while more than one file is open.
    pub(super) fn tab_strip(&mut self, ctx: &egui::Context) {
        if self.tab_count() < 2 {
            return;
        }
        let mut pick = None;
        let mut close = None;
        egui::TopBottomPanel::top("file_tabs").show(ctx, |ui| {
            ScrollArea::horizontal().id_salt("file_tabs_scroll").show(ui, |ui| {
                ui.horizontal(|ui| {
                    for i in 0..self.tab_count() {
                        let name = self.tab_path(i).map(file_name).unwrap_or_else(|| "New tab".into());
                        let text = if self.tab_dirty(i) { format!("\u{2022} {name}") } else { name };
                        let full = self.tab_path(i).map(|p| p.display().to_string()).unwrap_or_default();
                        ui.group(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let r = ui.selectable_label(i == self.tabs.active, text);
                            let r = if full.is_empty() { r } else { r.on_hover_text(full) };
                            if r.clicked() {
                                pick = Some(i);
                            }
                            if r.middle_clicked() {
                                close = Some(i);
                            }
                            if ui.small_button("\u{00D7}").on_hover_text("Close this tab (Ctrl+W)").clicked() {
                                close = Some(i);
                            }
                        });
                    }
                    if ui.button("+").on_hover_text("Open another save in a new tab (Ctrl+T)").clicked() {
                        pick = Some(usize::MAX);
                    }
                });
            });
        });
        match (pick, close) {
            (_, Some(i)) => self.close_tab(i),
            (Some(usize::MAX), _) => self.ask(Ask::Open, None, None),
            (Some(i), _) => self.switch_tab(i),
            _ => {}
        }
    }

    /// Ctrl+T, Ctrl+W, Ctrl+Tab and Ctrl+Shift+Tab.
    pub(super) fn tab_keys(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::T)) {
            self.ask(Ask::Open, None, None);
        }
        if self.doc.is_some() && ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::W)) {
            self.close_tab(self.tabs.active);
        }
        let n = self.tab_count();
        if n > 1 {
            if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Tab)) {
                self.switch_tab((self.tabs.active + n - 1) % n);
            } else if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Tab)) {
                self.switch_tab((self.tabs.active + 1) % n);
            }
        }
    }

    /// Before quitting with unsaved changes in a tab that isn't showing: show it, so the
    /// "save before closing?" question is about the file on screen.
    pub(super) fn show_first_dirty(&mut self) {
        if !self.dirty {
            if let Some(i) = (0..self.tab_count()).find(|&i| self.tab_dirty(i)) {
                self.switch_tab(i);
            }
        }
    }

    // ------------------------------------------------------------ copy and paste

    pub(super) fn copy_values(&mut self, what: String, values: Vec<(String, Value)>) {
        let Some(d) = &self.doc else { return };
        if values.is_empty() {
            return;
        }
        let from = file_name(&d.path);
        let n = values.len();
        self.ok(if n == 1 {
            format!("Copied {what}. Open another save (or tab) and paste it there.")
        } else {
            format!("Copied {what} ({n} values). Open another save (or tab) and use Paste.")
        });
        self.tabs.clip = Some(Clip { what, from, values });
    }

    /// Writes `values` into the open save as one undo step. Values this save doesn't have,
    /// or can't store, are skipped. Returns (pasted, skipped).
    pub(super) fn paste_values(&mut self, values: &[(String, Value)]) -> (usize, usize) {
        let start = self.undo.len();
        let (mut pasted, mut skipped) = (0, 0);
        let mut ptrs = vec![];
        for (ptr, v) in values {
            let editable = match &self.doc {
                Some(Doc { content: Content::Tree(t), .. }) => t.editable.contains_key(ptr),
                Some(d) => root(&d.content).is_some_and(|r| r.pointer(ptr).is_some()),
                None => false,
            };
            if !editable {
                skipped += 1;
                continue;
            }
            match self.set_value(ptr, v.clone()) {
                Ok(Some((before, _))) => {
                    let derived = match &mut self.doc {
                        Some(Doc { content: Content::Tree(t), .. }) => t.take_derived(),
                        _ => vec![],
                    };
                    let after = self.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(ptr)).cloned().unwrap_or(v.clone());
                    if before != after || !derived.is_empty() {
                        ptrs.extend(derived.iter().map(|(p, _, _)| p.clone()));
                        ptrs.push(ptr.clone());
                        self.undo.push(Change::Value { ptr: ptr.clone(), before, after, extra: derived });
                        pasted += 1;
                    }
                }
                _ => skipped += 1,
            }
        }
        if pasted > 0 {
            self.group_undo_from(start);
            self.redo.clear();
            self.last_edit = None;
            for p in &ptrs {
                self.after_change(p);
            }
        }
        (pasted, skipped)
    }

    /// Pastes the clipboard into the open save at the same places it was copied from.
    pub(super) fn paste_clip(&mut self) {
        let Some(clip) = self.tabs.clip.take() else { return };
        let (pasted, skipped) = self.paste_values(&clip.values);
        let what = &clip.what;
        match (pasted, skipped) {
            (0, 0) => self.info(format!("{what}: this save already has the same values.")),
            (0, _) => self.fail(format!("{what} isn't in this save in the same place, so nothing was pasted.")),
            (_, 0) => self.ok(format!("Pasted {what} from {}. Save to keep it, or Undo.", clip.from)),
            _ => self.ok(format!("Pasted {pasted} values of {what} from {}; {skipped} aren't in this save and were left out.", clip.from)),
        }
        self.tabs.clip = Some(clip);
    }

    /// Pastes a single copied value into `ptr` (any field, not only the one it came from).
    pub(super) fn paste_one_at(&mut self, ptr: &str) {
        let Some(v) = self.tabs.clip.as_ref().and_then(|c| (c.values.len() == 1).then(|| c.values[0].1.clone())) else { return };
        match self.paste_values(&[(ptr.to_string(), v)]) {
            (1.., _) => self.ok("Pasted. Save to keep it, or Undo."),
            (0, 0) => self.info("That field already has this value."),
            _ => self.fail("This field can't take the copied value."),
        }
    }

    /// The bar above the values that offers to paste what was copied.
    pub(super) fn clip_bar(&mut self, ui: &mut egui::Ui) {
        let Some(c) = &self.tabs.clip else { return };
        let n = c.values.len();
        let text = if n == 1 {
            format!("Copied: {} from {}", c.what, c.from)
        } else {
            format!("Copied: {} ({n} values) from {}", c.what, c.from)
        };
        let mut paste = false;
        let mut clear = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(text).color(theme::ACCENT));
            if ui
                .button("Paste into this save")
                .on_hover_text("Writes the copied values to the same places in this save. One Undo takes it back.")
                .clicked()
            {
                paste = true;
            }
            if n == 1 {
                ui.weak("or right-click a value \u{203A} Paste here");
            }
            if ui.small_button("\u{00D7}").on_hover_text("Forget the copied values").clicked() {
                clear = true;
            }
        });
        if paste {
            self.paste_clip();
        }
        if clear {
            self.tabs.clip = None;
        }
    }
}
