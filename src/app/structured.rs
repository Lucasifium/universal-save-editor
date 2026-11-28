//! The main panel: the game screen with its section tabs, and the Values, Tree and Raw tabs.

use super::*;

impl App {
    pub(super) fn structured_panel(&mut self, ui: &mut egui::Ui, now: f64) {
        let is_json = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Json { .. }));
        let game = self.doc.as_ref().and_then(|d| d.game);
        if (self.tab == Tab::Raw && !is_json) || (self.tab == Tab::Game && game.is_none()) {
            self.tab = Tab::Values;
        }
        let hi = self.has_icons;

        // View bar: tabs, search, options. On a phone the search box goes under the tabs when
        // they leave it too little room.
        let phone = narrow(ui);
        let bar = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
            if phone {
                ui.horizontal_wrapped(|ui| add(ui));
            } else {
                ui.horizontal(|ui| add(ui));
            }
        };
        let old = self.tab;
        if let Some(g) = game {
            // Many tabs wrap onto more lines rather than running past the window's edge.
            ui.horizontal_wrapped(|ui| {
                self.section_tabs(ui, g);
                if self.tab == Tab::Game {
                    self.game_find_box(ui);
                }
            });
        }
        let on_game_tab = self.tab == Tab::Game;
        let views_bar = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
            if on_game_tab {
                return;
            }
            bar(ui, add)
        };
        views_bar(ui, &mut |ui| {
            let r = ui.selectable_value(&mut self.tab, Tab::Values, tr("Values")).on_hover_text(tr("Every editable value in a simple list"));
            theme::mark_selected(ui, &r, old == Tab::Values);
            let r = ui.selectable_value(&mut self.tab, Tab::Tree, tr("Tree")).on_hover_text(tr("The full structure of the save, as nested groups"));
            theme::mark_selected(ui, &r, old == Tab::Tree);
            if is_json {
                let r = ui.selectable_value(&mut self.tab, Tab::Raw, tr("Raw JSON")).on_hover_text(tr("Edit the JSON text directly"));
                theme::mark_selected(ui, &r, old == Tab::Raw);
            }
            if matches!(self.tab, Tab::Values | Tab::Tree) {
                self.advanced_tab = self.tab;
            }
            if old != self.tab {
                if old == Tab::Raw {
                    let target = self.tab;
                    self.tab = Tab::Raw;
                    if self.commit_raw() {
                        self.tab = target;
                    }
                }
                if self.tab == Tab::Raw {
                    if let Some(Doc { content: Content::Json { value, .. }, .. }) = &self.doc {
                        self.raw_text = serde_json::to_string_pretty(value).unwrap_or_default();
                    }
                    self.raw_error = None;
                }
            }
            ui.separator();
            if self.tab == Tab::Raw {
                if ui.button(tr("Apply")).clicked() && self.commit_raw() {
                    self.ok(tr("Raw JSON applied."));
                }
                if let Some(e) = &self.raw_error {
                    ui.colored_label(theme::fg(ui, theme::ERR), e);
                }
                return;
            }
            if matches!(self.tab, Tab::Values | Tab::Tree) {
                if hi {
                    ui.label(RichText::new(icon::SEARCH).family(theme::icons_family()).weak());
                }
                let id = egui::Id::new("search_box");
                if narrow(ui) && ui.available_size_before_wrap().x < 250.0 {
                    ui.end_row();
                }
                let w = if narrow(ui) { (ui.available_size_before_wrap().x - 150.0).max(80.0) } else { (ui.available_width() - 400.0).clamp(120.0, 320.0) };
                let hint = if self.tab == Tab::Tree { tr("Filter the tree by name or value  (Ctrl+F)") } else { tr("Search names or values  (Ctrl+F)") };
                let r = ui.add(TextEdit::singleline(&mut self.search).id(id).hint_text(hint).desired_width(w));
                let r = widgets::named(r, egui::WidgetType::TextEdit, hint);
                if r.changed() {
                    self.search_typed_at = ui.input(|i| i.time);
                }
                if self.focus_search {
                    r.request_focus();
                    self.focus_search = false;
                }
                // The text box gives up focus on Escape before we see it, so check both.
                if (r.has_focus() || r.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.search.clear();
                }
                if !self.search.is_empty() && crate::widgets::small_button(ui, tr("Clear")).clicked() {
                    self.search.clear();
                }
                let on = self.bulk.is_some();
                if ui.selectable_label(on, tr("Bulk edit")).on_hover_text(tr("Tick several values and set them all at once, or add to or multiply them")).clicked() {
                    self.bulk = if on { None } else { Some(Bulk { ticked: HashSet::new(), op: values_view::BulkOp::Set, arg: String::new(), model_gen: self.model_gen }) };
                }
            }
            if !narrow(ui) && self.tab != Tab::Game {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| self.view_options(ui));
            }
        });
        if narrow(ui) && matches!(self.tab, Tab::Values | Tab::Tree) {
            ui.horizontal(|ui| self.view_options(ui));
        }
        if self.tab == Tab::Values {
            self.clip_bar(ui);
        }
        if self.tab == Tab::Values && self.bulk.is_some() {
            self.bulk_bar(ui, now);
        }
        ui.add_space(4.0);
        self.structured_body(ui, now);
    }

    /// The tabs of a game's screen: one per section of the screen, then Advanced (the Values and
    /// Tree views) at the end.
    pub(super) fn section_tabs(&mut self, ui: &mut egui::Ui, game: crate::game_ui::Game) {
        let list = crate::game_ui::sections(ui.ctx());
        let on_game = self.tab == Tab::Game;
        let open = if list.iter().any(|s| s.title == self.game_section) { self.game_section.clone() } else { list.first().map(|s| s.title.clone()).unwrap_or_default() };
        // Some(Some(title)): that section; Some(None): Advanced.
        let mut pick: Option<Option<String>> = None;
        let mut tab = |ui: &mut egui::Ui, title: &str, selected: bool, edited: bool, tip: &str| -> bool {
            let mut r = ui.selectable_label(selected, title);
            if edited {
                ui.painter().circle_filled(r.rect.right_top() + egui::vec2(-3.0, 4.0), 3.0, theme::MODIFIED);
                r = r.on_hover_text(tr("Has changes"));
            } else if !tip.is_empty() {
                r = r.on_hover_text(tip);
            }
            theme::mark_selected(ui, &r, selected);
            r.clicked()
        };
        if let Some((id, names)) = crate::game_ui::page_tabs(ui.ctx()) {
            let cur = ui.ctx().data(|d| d.get_temp::<usize>(egui::Id::new(("game_sel", id.as_str())))).unwrap_or(0).min(names.len().saturating_sub(1));
            for (i, n) in names.iter().enumerate() {
                if tab(ui, crate::i18n::tr_any(n), on_game && i == cur, false, "") {
                    crate::game_ui::open_page(ui.ctx(), &id, i);
                    pick = Some(Some(open.clone()));
                }
            }
        } else if list.is_empty() {
            let label = crate::i18n::tr_any(game.tab_label());
            if tab(ui, label, on_game, false, tr("This game's own screen: its stats, inventory and more, laid out like the game")) {
                pick = Some(Some(String::new()));
            }
        }
        let paged = crate::game_ui::page_tabs(ui.ctx()).is_some();
        for s in list.iter().filter(|_| !paged) {
            if tab(ui, &s.title, on_game && s.title == open, s.edited, "") {
                pick = Some(Some(s.title.clone()));
            }
        }
        if tab(ui, tr("Advanced"), !on_game, false, tr("Every value and the full structure of the save, for anything this screen doesn't show")) {
            pick = Some(None);
        }
        // Testing aid, with SAVE_EDITOR_SHOT: SAVE_EDITOR_SHOT_TAB=n opens the nth tab (counting from 0; one past the last is Advanced).
        #[cfg(not(target_os = "android"))]
        if pick.is_none() && !list.is_empty() {
            static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if let Some(n) = std::env::var("SAVE_EDITOR_SHOT_TAB").ok().and_then(|s| s.parse::<usize>().ok()) {
                if !DONE.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    pick = Some(list.get(n).map(|s| s.title.clone()));
                }
            }
        }
        match pick {
            Some(Some(title)) => {
                if self.tab == Tab::Raw && !self.commit_raw() {
                    return;
                }
                self.game_section = title;
                self.tab = Tab::Game;
            }
            Some(None) if on_game => {
                self.tab = self.advanced_tab;
            }
            _ => {}
        }
    }

    /// The find box of a game screen: highlights and scrolls to labels on the visible tab.
    pub(super) fn game_find_box(&mut self, ui: &mut egui::Ui) {
        let id = egui::Id::new("game_find_box");
        let before = self.game_find.clone();
        // Too little room left on the phone's tab line: the box goes on its own line.
        // (In the phone's wrapping line, what is left of this row; the rest of a row on desktop.)
        if narrow(ui) && ui.available_size_before_wrap().x < 110.0 {
            ui.end_row();
        }
        let room = ui.available_size_before_wrap().x;
        // A phone has no Ctrl+F, and no room for the long hint.
        let hint = if narrow(ui) { tr("Find\u{2026}") } else { tr("Find on this screen  (Ctrl+F)") };
        // As wide as its hint (French and German cut it off at 200 px), within reason.
        let hint_w = ui.fonts(|f| f.layout_no_wrap(hint.to_string(), egui::TextStyle::Body.resolve(ui.style()), Color32::WHITE).size().x);
        let w = (hint_w + 12.0).clamp(200.0, 320.0).min(room - 2.0 * ui.spacing().item_spacing.x - 12.0);
        let r = ui.add(TextEdit::singleline(&mut self.game_find).id(id).hint_text(hint).desired_width(w));
        let r = widgets::named(r, egui::WidgetType::TextEdit, hint);
        if self.focus_game_find {
            r.request_focus();
            self.focus_game_find = false;
        }
        if before != self.game_find {
            self.game_find_idx = 0;
        }
        let total = ui.ctx().data(|d| d.get_temp::<usize>(egui::Id::new(crate::game_ui::FIND_COUNT))).unwrap_or(0);
        if !self.game_find.trim().is_empty() {
            if total == 0 {
                ui.weak(tr("no match on this tab"));
            } else {
                self.game_find_idx %= total;
                ui.weak(trf("{0} of {1}", &[&(self.game_find_idx + 1), &total]));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let back = ui.input(|i| i.modifiers.shift);
                if crate::widgets::small_button(ui, tr("Next")).clicked() || (enter && !back) {
                    self.game_find_idx = (self.game_find_idx + 1) % total;
                    if enter {
                        r.request_focus();
                    }
                }
                if crate::widgets::small_button(ui, tr("Prev")).clicked() || (enter && back) {
                    self.game_find_idx = (self.game_find_idx + total - 1) % total;
                    if enter {
                        r.request_focus();
                    }
                }
            }
            if crate::widgets::small_button(ui, tr("Clear")).clicked() || ((r.has_focus() || r.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Escape))) {
                self.game_find.clear();
            }
        }
    }

    /// The Raw keys / Advanced toggles.
    pub(super) fn view_options(&mut self, ui: &mut egui::Ui) {
        let mut changed = widgets::labeled_toggle(
            ui,
            &mut self.settings.show_raw_keys,
            tr("Raw keys"),
            tr("Show the game's own key name next to each friendly name"),
        );
        if self.tab != Tab::Values {
            if changed {
                self.settings.save();
            }
            return;
        }
        ui.add_space(8.0);
        let hidden = self.model.sections.iter().filter(|s| s.advanced).count();
        let tip = if hidden == 1 && !self.settings.show_advanced {
            tr("Also show engine internals and read-only values (1 section hidden now)").to_string()
        } else if hidden > 0 && !self.settings.show_advanced {
            trf("Also show engine internals and read-only values ({0} sections hidden now)", &[&hidden])
        } else {
            tr("Also show engine internals and read-only values").to_string()
        };
        let label = if hidden > 0 && !self.settings.show_advanced { format!("{} ({hidden})", tr("Rarely needed")) } else { tr("Rarely needed").into() };
        changed |= widgets::labeled_toggle(ui, &mut self.settings.show_advanced, &label, &tip);
        if changed {
            self.settings.save();
        }
    }

    pub(super) fn structured_body(&mut self, ui: &mut egui::Ui, now: f64) {
        let hi = self.has_icons;
        let mut edits: Vec<(String, Value)> = Vec::new();
        let (mut copy, mut paste_at) = (None, None);
        let mut pin_click: Option<(String, usize)> = None;
        match self.tab {
            Tab::Game => {
                let d = self.doc.as_ref().unwrap();
                let (Some(r), Some(b), Some(game)) = (root(&d.content), root(&d.baseline), d.game) else { return };
                let newest = formats::copeck_newest(&d.layers);
                let mut cx = crate::game_ui::GameCtx {
                    root: r,
                    baseline: b,
                    tree: match &d.content {
                        Content::Tree(t) => Some(&**t),
                        _ => None,
                    },
                    edits: vec![],
                    archive: d.layers.iter().find_map(|l| if let Layer::Copeck { name, .. } = l { Some(name.as_str()) } else { None }).map(|n| (n, d.zip_entries.as_slice(), newest.as_deref())),
                    open_entry: None,
                };
                // Laid out to the window's width; a table or banner that is wider still (a phone)
                // scrolls sideways instead of being cut off.
                // Minus the vertical scrollbar, or the content is a little wider than the window.
                let sb = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin + ui.spacing().scroll.bar_outer_margin;
                let w = (ui.available_width() - sb).max(100.0);
                crate::game_ui::begin_find(ui.ctx(), &self.game_find, self.game_find_idx);
                crate::game_ui::set_active_section(ui.ctx(), Some(&self.game_section));
                // Changes with every edit, undo, revert, load and save (and with the open tab of files).
                // The language too: hidden tabs' quick actions hold translated text.
                let lang = crate::i18n::current().code.bytes().fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
                crate::game_ui::set_data_version(ui.ctx(), self.edit_gen.wrapping_mul(1 << 16).wrapping_add(self.tabs.active as u64) ^ lang.wrapping_mul(0x9E37_79B9_7F4A_7C15));
                crate::game_ui::set_header_facts(ui.ctx(), header::facts(d.stamp.as_ref().and_then(|s| s.modified()), r));
                ScrollArea::both().auto_shrink([false, false]).max_width(w + sb).show_locked(ui, |ui| {
                    ui.set_max_width(w);
                    crate::game_ui::show(game, ui, &mut cx);
                    ui.add_space(12.0);
                    ui.weak(tr("Everything else in this save is in the Advanced tab."));
                });
                edits = cx.edits;
                if let Some(p) = ui.ctx().data_mut(|d| d.remove_temp::<String>(egui::Id::new(crate::game_ui::SHOW_IN_TREE))) {
                    self.search.clear();
                    self.tree_reveal = Some((p, ui.input(|i| i.time)));
                    self.tab = Tab::Tree;
                }
                // A story panel asked for a named backup before its first edit.
                let want = egui::Id::new(crate::game_ui::STORY_BACKUP_REQUEST);
                if ui.ctx().data_mut(|d| d.remove_temp::<bool>(want)).unwrap_or(false) && !WEB && d.path.is_file() {
                    self.backup_name = Some(String::new());
                }
                if let Some(e) = cx.open_entry {
                    self.reopen_with(|o| o.zip_entry = Some(e));
                    return;
                }
            }
            Tab::Raw => {
                ScrollArea::both().auto_shrink([false, false]).show_locked(ui, |ui| {
                    ui.add(TextEdit::multiline(&mut self.raw_text).code_editor().desired_width(f32::INFINITY));
                });
            }
            Tab::Tree => {
                let d = self.doc.as_ref().unwrap();
                let (Some(r), Some(b)) = (root(&d.content), root(&d.baseline)) else { return };
                let editable = match &d.content {
                    Content::Tree(t) => Some(&t.editable),
                    _ => None,
                };
                let value_x = ui.min_rect().left() + (ui.available_width() * 0.42).clamp(140.0, 480.0);
                let needle = self.search.trim().to_string();
                if needle.is_empty() {
                    self.tree_visible = None;
                } else {
                    let key = (needle.clone(), self.model_gen, self.edit_gen);
                    if self.tree_visible.as_ref().map(|t| &t.1) != Some(&key) {
                        self.tree_visible = Some((json_view::filter_ptrs(r, &d.names, &needle), key));
                    }
                    if self.tree_visible.as_ref().is_some_and(|t| t.0.is_empty()) {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(trf("Nothing in the tree matches \u{201C}{0}\u{201D}.", &[&needle]));
                            ui.weak(tr("The filter looks at names, keys and current values."));
                        });
                        return;
                    }
                }
                let now_t = ui.input(|i| i.time);
                if self.tree_reveal.as_ref().is_some_and(|(_, t0)| now_t - t0 > 4.0) {
                    self.tree_reveal = None;
                }
                let reveal_scroll = self.tree_reveal.as_ref().is_some_and(|(_, t0)| now_t - t0 < 0.7);
                if self.tree_reveal.is_some() {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                }
                let mut ctx = TreeCtx {
                    names: &d.names,
                    editable,
                    baseline: b,
                    show_raw_keys: self.settings.show_raw_keys,
                    value_x,
                    edits: vec![],
                    tree: match &d.content {
                        Content::Tree(t) => Some(&**t),
                        _ => None,
                    },
                    visible: self.tree_visible.as_ref().map(|t| &t.0),
                    reveal: self.tree_reveal.as_ref().map(|(p, _)| p.as_str()),
                    reveal_scroll,
                };
                ScrollArea::vertical().auto_shrink([false, false]).show_locked(ui, |ui| json_view::tree_ui(ui, r, &mut ctx));
                edits = ctx.edits;
            }
            Tab::Values => {
                let ticked = self.bulk_ticked().cloned();
                let d = self.doc.as_ref().unwrap();
                let (Some(r), Some(b)) = (root(&d.content), root(&d.baseline)) else { return };
                let value_gen = if self.search.trim().is_empty() { 0 } else { self.edit_gen };
                let key = (self.search.clone(), self.settings.show_advanced, self.collapsed_gen, self.model_gen, value_gen);
                // On very large saves, searching every value takes a moment: wait until
                // typing pauses instead of re-searching on every key press.
                let t = ui.input(|i| i.time);
                let now_t_values = t;
                let typing = self.model.rows.len() > 50_000 && t - self.search_typed_at < 0.3 && self.disp_key.is_some();
                if typing {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
                }
                let pin_key = crate::pins::game_key(&d.path, &d.content.kind());
                let pinned: Vec<usize> = self.pins.of(&pin_key).iter().filter_map(|p| self.model.by_ptr.get(p.as_str()).copied()).collect();
                if self.disp_key.as_ref() != Some(&key) && !typing {
                    self.disp = values_view::filter_with(&self.model, r, &self.search, self.settings.show_advanced, &self.collapsed, &pinned, d.game.is_none());
                    self.disp_key = Some(key);
                }
                let pinned: HashSet<usize> = pinned.into_iter().collect();
                if self.disp.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        if self.search.trim().is_empty() {
                            ui.weak(tr("No editable values here. Try turning on Rarely needed, or use the Tree view."));
                        } else {
                            ui.label(trf("Nothing matches \u{201C}{0}\u{201D}.", &[&self.search.trim()]));
                            ui.weak(tr("Try part of a name (\u{201C}gold\u{201D}) or a current value (\u{201C}1500\u{201D})."));
                            if !self.settings.show_advanced {
                                ui.weak(tr("Rarely needed values are hidden \u{2014} turn on Rarely needed to search them too."));
                            }
                        }
                    });
                    return;
                }
                let mut ctx = ViewCtx {
                    root: r,
                    tree: match &d.content {
                        Content::Tree(t) => Some(&**t),
                        _ => None,
                    },
                    baseline: b,
                    show_raw_keys: self.settings.show_raw_keys,
                    has_icons: hi,
                    edits: vec![],
                    toggle_section: None,
                    scroll_to: self.scroll_to_row.take(),
                    ticked: ticked.as_ref(),
                    toggle_tick: vec![],
                    show_in_tree: None,
                    can_paste_one: self.tabs.clip.as_ref().is_some_and(|c| c.values.len() == 1),
                    copy: None,
                    paste_at: None,
                    pins: Some(&pinned),
                    toggle_pin: None,
                };
                values_view::show(ui, &self.model, &self.disp, &self.collapsed, &mut ctx);
                pin_click = ctx.toggle_pin.take().map(|i| (pin_key, i));
                copy = ctx.copy.take();
                paste_at = ctx.paste_at.take();
                if let Some(p) = ctx.show_in_tree.take() {
                    self.search.clear();
                    self.tree_reveal = Some((p, now_t_values));
                    self.tab = Tab::Tree;
                }
                if let Some(sec) = ctx.toggle_section {
                    if !self.collapsed.remove(&sec) {
                        self.collapsed.insert(sec);
                    }
                    self.collapsed_gen += 1;
                }
                edits = ctx.edits;
                let ticks = ctx.toggle_tick;
                self.bulk_toggle(ticks);
            }
        }
        self.apply_edits(edits, now);
        if let Some((key, i)) = pin_click {
            self.toggle_pin(&key, i);
        }
        if let Some((what, values)) = copy {
            self.copy_values(what, values);
        }
        if let Some(p) = paste_at {
            self.paste_one_at(&p);
        }
    }
}
