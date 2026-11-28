//! The Changes panel and the hex side panel.

use super::*;

/// How many changes the Changes panel lists; a bulk edit can make tens of thousands, and
/// drawing a card for each one on every frame would freeze the window.
pub(super) const CHANGES_SHOWN: usize = 300;

/// The line under a cut-short list of changes.
pub(super) fn more_changes(ui: &mut egui::Ui, total: usize) {
    if total > CHANGES_SHOWN {
        ui.add_space(6.0);
        ui.weak(trf("Showing the first {0} of {1} changes. Saving, Revert all and Copy as text cover all of them.", &[&CHANGES_SHOWN, &total]));
    }
}

/// Contiguous ranges where `a` differs from `b`, merging gaps of < 4 bytes.
pub(super) fn byte_ranges(a: &[u8], b: &[u8]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for i in (0..a.len().min(b.len())).filter(|&i| a[i] != b[i]) {
        match out.last_mut() {
            Some((_, end)) if i <= *end + 3 => *end = i + 1,
            _ => out.push((i, i + 1)),
        }
    }
    out
}

pub(super) fn type_combo(ui: &mut egui::Ui, id: &str, ty: &mut NumType) {
    egui::ComboBox::from_id_salt(id).selected_text(ty.name()).width(120.0).show_ui(ui, |ui| {
        for t in NumType::ALL {
            ui.selectable_value(ty, t, t.name());
        }
    });
}

impl App {
    pub(super) fn changes_panel(&mut self, ctx: &egui::Context, now: f64) {
        let mut revert: Vec<(String, Value)> = vec![];
        let mut revert_groups: Vec<Vec<(String, Value)>> = vec![];
        let mut revert_bytes: Vec<(usize, Vec<u8>)> = vec![];
        let mut jump = None;
        let mut revert_all = false;
        let mut close = false;
        let mut copy = false;
        let mut preset = false;
        // Edits made earlier in this frame are in the list straight away.
        self.refresh_changes();
        let changes = std::mem::take(&mut self.changes);
        // On a phone the list takes the whole width (beside the screen it left it no room);
        // Close brings the screen back.
        let phone = ctx.screen_rect().width() < PHONE_W;
        let panel = egui::SidePanel::right("changes").default_width(320.0).min_width(260.0);
        let panel = if phone { panel.resizable(false).exact_width(ctx.screen_rect().width()) } else { panel };
        panel.show(ctx, |ui| {
            let Some(d) = &self.doc else { return };
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(tr("Changes"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::widgets::small_button(ui, tr("Close")).clicked() {
                        close = true;
                    }
                });
            });
            ui.weak(tr("Since you opened or last saved this file."));
            ui.add_space(4.0);
            if self.dirty {
                ui.horizontal_wrapped(|ui| {
                    if crate::widgets::small_button(ui, tr("Copy as text")).on_hover_text(tr("Copy the list of changes, to paste when asking for help or sharing what you did")).clicked() {
                        copy = true;
                    }
                    if crate::widgets::small_button(ui, tr("Save as preset\u{2026}")).on_hover_text(tr("Keep these changes to make them again on another save or slot")).clicked() {
                        preset = true;
                    }
                    if crate::widgets::small_button(ui, tr("Revert all")).on_hover_text(tr("Put every value back as it was when the file was opened or last saved")).clicked() {
                        revert_all = true;
                    }
                });
                ui.separator();
            }
            ScrollArea::vertical().auto_shrink([false, false]).show_locked(ui, |ui| match (&d.content, &d.baseline) {
                (Content::Binary(cur), Content::Binary(base)) => {
                    let ranges = &changes.ranges;
                    if ranges.is_empty() {
                        ui.weak(tr("No changes yet."));
                    }
                    for &(start, end) in ranges.iter().take(CHANGES_SHOWN) {
                        // A change list a frame old after the data got shorter.
                        if end > cur.len() || end > base.len() {
                            continue;
                        }
                        theme::card(ui, |ui| {
                            let name = self.hex.labels.get(&start).map(|l| l.name.clone()).unwrap_or(format!("Offset 0x{start:X}"));
                            if ui.link(RichText::new(name).strong()).clicked() {
                                self.hex.jump(start);
                            }
                            let hex = |b: &[u8]| b.iter().take(16).map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ");
                            ui.label(RichText::new(hex(&base[start..end])).weak().strikethrough().monospace());
                            ui.label(RichText::new(hex(&cur[start..end])).color(theme::fg(ui, theme::MODIFIED)).monospace());
                            if crate::widgets::small_button(ui, tr("Revert")).clicked() {
                                revert_bytes.push((start, base[start..end].to_vec()));
                            }
                        });
                    }
                    more_changes(ui, ranges.len());
                }
                (Content::Text(a), Content::Text(b)) => {
                    ui.weak(if a != b { tr("The text has been edited.") } else { tr("No changes yet.") });
                }
                (c, b) => {
                    let (Some(cv), Some(bv)) = (root(c), root(b)) else { return };
                    let changed = &changes.ptrs;
                    if changed.is_empty() {
                        ui.weak(tr("No changes yet. Edited values are highlighted in orange."));
                    }
                    // Values one button press changed together (Max crafting materials...)
                    // are one entry, reverted together. Fewer than two left: listed alone.
                    let mut members: HashMap<u64, Vec<&String>> = HashMap::new();
                    for p in changed.iter() {
                        if let Some(&g) = self.touched.group.get(p) {
                            members.entry(g).or_default().push(p);
                        }
                    }
                    let mut shown_groups = HashSet::new();
                    let mut shown = 0;
                    for ptr in changed.iter() {
                        if shown >= CHANGES_SHOWN {
                            break;
                        }
                        if let Some(list) = self.touched.group.get(ptr).and_then(|g| members.get(g).map(|l| (*g, l))).filter(|(_, l)| l.len() > 1) {
                            if !shown_groups.insert(list.0) {
                                continue;
                            }
                            shown += 1;
                            let (gid, list) = list;
                            theme::card(ui, |ui| {
                                ui.label(RichText::new(trf("{0} values changed together", &[&list.len()])).strong());
                                ui.weak(tr("Made by one button. Revert puts all of them back."));
                                egui::CollapsingHeader::new(tr("Show values")).id_source(("change_group", gid)).show(ui, |ui| {
                                    for p in list.iter().take(CHANGES_SHOWN) {
                                        let name = self.model.by_ptr.get(p.as_str()).map(|&i| self.model.rows[i].name().to_string()).unwrap_or_else(|| (*p).clone());
                                        let old = bv.pointer(p).map(widgets::display).unwrap_or_else(|| tr("(none)").into());
                                        let new = cv.pointer(p).map(widgets::display).unwrap_or_else(|| tr("(none)").into());
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(name);
                                            ui.label(RichText::new(old).weak().strikethrough());
                                            ui.label("\u{203A}");
                                            ui.label(RichText::new(new).color(theme::fg(ui, theme::MODIFIED)));
                                        });
                                    }
                                });
                                if crate::widgets::small_button(ui, tr("Revert all of these")).clicked() {
                                    let group: Vec<(String, Value)> = list.iter().filter_map(|p| bv.pointer(p).map(|o| ((*p).clone(), o.clone()))).collect();
                                    revert_groups.push(group);
                                }
                            });
                            continue;
                        }
                        shown += 1;
                        theme::card(ui, |ui| {
                            if ptr.is_empty() {
                                ui.label(RichText::new(tr("Whole document (Raw JSON edit)")).strong());
                            } else {
                                let (name, context) = match self.model.by_ptr.get(ptr.as_str()) {
                                    Some(&i) => (self.model.rows[i].name().to_string(), self.model.rows[i].context().to_string()),
                                    None => (ptr.clone(), String::new()),
                                };
                                if ui.link(RichText::new(&name).strong()).on_hover_text(tr("Show in list")).clicked() {
                                    jump = self.model.by_ptr.get(ptr.as_str()).copied();
                                }
                                if !context.is_empty() {
                                    ui.weak(context);
                                }
                                let old = bv.pointer(ptr).map(widgets::display).unwrap_or_else(|| tr("(none)").into());
                                let new = cv.pointer(ptr).map(widgets::display).unwrap_or_else(|| tr("(none)").into());
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(old).weak().strikethrough());
                                    ui.label("\u{203A}");
                                    ui.label(RichText::new(new).color(theme::fg(ui, theme::MODIFIED)));
                                });
                            }
                            if crate::widgets::small_button(ui, tr("Revert")).clicked() {
                                if let Some(orig) = bv.pointer(ptr) {
                                    revert.push((ptr.clone(), orig.clone()));
                                }
                            }
                        });
                    }
                    more_changes(ui, changed.len());
                }
            });
        });
        self.changes = changes;
        if copy {
            self.copy_changes(ctx);
        }
        if preset {
            self.start_preset_save();
        }
        for (p, v) in revert {
            self.apply_edit(p, v, f64::NEG_INFINITY);
        }
        for group in revert_groups {
            self.apply_edits(group, f64::NEG_INFINITY);
        }
        for (o, b) in revert_bytes {
            self.apply_bytes(o, b);
        }
        if let Some(i) = jump {
            self.jump_to_row(i);
        }
        if revert_all {
            self.revert_all();
        }
        if close {
            self.show_changes = false;
        }
        let _ = now;
    }

    // ------------------------------------------------------------ UI: hex

    pub(super) fn hex_side_panel(&mut self, ctx: &egui::Context) {
        let mut refine = false;
        let mut labels_changed = false;
        let mut byte_edit: Option<(usize, Vec<u8>)> = None;
        let mut status: Option<Result<String, String>> = None;
        // Cairn: the saves inside the slot pack, newest first.
        let cairn: Option<(String, Vec<(String, String)>, usize)> = self.doc.as_ref().and_then(|d| {
            let current = d.layers.iter().find_map(|l| if let Layer::Copeck { name, .. } = l { Some(name.clone()) } else { None })?;
            let mut saves: Vec<(String, String)> = d.zip_entries.iter().filter_map(|e| cairn_save_label(e).map(|l| (e.clone(), l))).collect();
            // Dated saves newest first, then QUICK.SAV and any others.
            saves.sort_by(|a, b| {
                let dated = |l: &str| l.starts_with(|c: char| c.is_ascii_digit());
                dated(&b.1).cmp(&dated(&a.1)).then(b.1.cmp(&a.1))
            });
            let others = d.zip_entries.len() - saves.len();
            Some((current, saves, others))
        });
        let mut open_entry: Option<String> = None;
        let mut names_action: Option<NamesAction> = None;
        let mut hide_guide = false;
        let mut show_guide = false;
        let guide = self.settings.hex_guide;
        // Beside the grid on a wide window; under it on a narrow one, so the bytes keep the width.
        let wide = ctx.screen_rect().width() >= 900.0;
        let body = |ui: &mut egui::Ui| {
            let Some(Doc { content: Content::Binary(data), .. }) = &self.doc else { return };
            let h = &mut self.hex;
            ScrollArea::vertical().auto_shrink([false, false]).show_locked(ui, |ui| {
                ui.add_space(6.0);
                if let Some((current, saves, others)) = &cairn {
                    theme::card(ui, |ui| {
                        ui.label(crate::theme::panel_title(tr("Cairn saves in this slot")));
                        ui.weak(trf("{0} saves, the dated ones newest first; pick one to open it.", &[&saves.len()]));
                        ScrollArea::vertical().show_list(ui, "cairn_saves", 220.0, |ui| {
                            for (entry, label) in saves {
                                let shown = entry == current;
                                let text = if shown { RichText::new(trf("{0}  (open)", &[&label])).strong() } else { RichText::new(label) };
                                if ui.selectable_label(shown, text).on_hover_text(entry).clicked() && !shown {
                                    open_entry = Some(entry.clone());
                                }
                            }
                        });
                        if let Some(at) = crate::fmt::cairn::stats_at(data) {
                            ui.add_space(6.0);
                            ui.label(RichText::new(tr("This save's run")).strong());
                            egui::Grid::new("cairn_stats").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                                for (label, off, _) in crate::fmt::cairn::FIELDS {
                                    let o = at + off;
                                    let Some(b) = data.get(o..o + 4) else { continue };
                                    let mut v = u32::from_le_bytes(b.try_into().unwrap());
                                    ui.label(label);
                                    if ui.add(egui::DragValue::new(&mut v).range(0..=99_999_999)).changed() {
                                        byte_edit = Some((o, v.to_le_bytes().to_vec()));
                                        status = Some(Ok(trf("Set {0} to {1}", &[&label, &v])));
                                    }
                                    if off == 0 {
                                        ui.weak(format!("{}:{:02}:{:02}", v / 3600, v / 60 % 60, v % 60));
                                    } else {
                                        ui.label("");
                                    }
                                    ui.end_row();
                                }
                            });
                            ui.weak(tr("As on the game's slot card. The card's own copy of the play time updates the next time the game saves."));
                        }
                        if *others > 0 {
                            ui.weak(trf("{0} other files (routes, tutorials, run info) are in the pack too: Tools \u{203A} Open another file in this archive.",
                                &[others],
                            ));
                        }
                    });
                    ui.add_space(6.0);
                }
                if guide {
                    Self::guide_card(ui, h, &mut hide_guide);
                }
                theme::card(ui, |ui| {
                    ui.label(crate::theme::panel_title(tr("Named values")));
                    if h.labels.is_empty() {
                        ui.weak(tr("Binary saves have no names. Find a value with Value search, click it, then name it here \u{2014} names are remembered for this file."));
                    }
                    let mut remove = None;
                    for (&off, l) in &h.labels {
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(egui::vec2(96.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.set_width(96.0);
                                let link = egui::Link::new(RichText::new(&l.name).strong());
                                if ui.add(link).on_hover_text(trf("Show in grid \u{2014} 0x{0} as {1}", &[&format!("{off:X}"), &l.ty.name()])).clicked() {
                                    h.selected = off;
                                    h.scroll_to = Some(off);
                                }
                            });
                            let current = hex_view::read_value(l.ty, data, off, h.big_endian).unwrap_or_else(|| "\u{2014}".into());
                            let mut text = h.label_edits.get(&off).cloned().unwrap_or(current);
                            let r = ui.add(TextEdit::singleline(&mut text).desired_width(120.0));
                            if r.changed() {
                                h.label_edits.insert(off, text.clone());
                            }
                            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if ui.button(tr("Set")).on_hover_text(tr("Write this value (or press Enter)")).clicked() || enter {
                                match hex_view::encode_value(l.ty, &text, h.big_endian) {
                                    Ok(b) => {
                                        byte_edit = Some((off, b));
                                        status = Some(Ok(trf("Set {0} to {1}", &[&l.name, &text])));
                                    }
                                    Err(e) => status = Some(Err(trf("Invalid value for {0}: {1}", &[&l.name, &e]))),
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let trash = widgets::icon(self.has_icons, "\u{E74D}", "\u{00D7}").size(13.0).weak();
                                let tip = tr("Forget this name (the value is not changed)");
                                if widgets::icon_button(ui, trash, tip, tip).clicked() {
                                    remove = Some(off);
                                }
                            });
                        });
                    }
                    if byte_edit.is_some() {
                        if let Some((o, _)) = &byte_edit {
                            h.label_edits.remove(o);
                        }
                    }
                    if let Some(off) = remove {
                        h.labels.remove(&off);
                        h.label_edits.remove(&off);
                        labels_changed = true;
                    }
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut h.label_name).hint_text(trf("Name for 0x{0}", &[&format!("{:X}", h.selected)])).desired_width(130.0));
                        type_combo(ui, "label_type", &mut h.edit_type);
                        if ui.add_enabled(!h.label_name.trim().is_empty(), egui::Button::new(tr("Add"))).clicked() {
                            h.labels.insert(h.selected, OffsetLabel { name: h.label_name.trim().to_string(), ty: h.edit_type });
                            h.label_name.clear();
                            labels_changed = true;
                        }
                    });
                    names_action = Self::names_buttons(ui, !h.labels.is_empty());
                });

                theme::card(ui, |ui| {
                    ui.label(crate::theme::panel_title(tr("Value search")));
                    if guide {
                        ui.weak(tr("Don't know the number (a health bar, a hidden counter)? Start with \u{201C}Unknown starting value\u{201D}, then refine with Increased, Decreased or Changed after each in-game save."));
                    } else {
                        ui.weak(tr("Know the value? Search for it, change it in-game, save, and refine with the new value. Don't know it (a health bar, a hidden counter)? Start with \u{201C}Unknown starting value\u{201D}, then refine with Increased, Decreased or Changed after each in-game save."));
                        if ui.link(tr("Show the step-by-step guide")).clicked() {
                            show_guide = true;
                        }
                    }
                    let scanning = h.searched && (h.all_locations || !h.results.is_empty());
                    let mut start = false;
                    ui.horizontal(|ui| {
                        type_combo(ui, "search_type", &mut h.search_type);
                        let mode = if scanning { &mut h.next_mode } else { &mut h.first_mode };
                        let modes: &[hex_view::ScanMode] = if scanning { &hex_view::ScanMode::NEXT } else { &hex_view::ScanMode::FIRST };
                        egui::ComboBox::from_id_salt("scan_mode").selected_text(mode.name()).width(140.0).show_ui(ui, |ui| {
                            for &m in modes {
                                ui.selectable_value(mode, m, m.name());
                            }
                        });
                    });
                    let inputs = if scanning { h.next_mode.inputs() } else { h.first_mode.inputs() };
                    if inputs > 0 {
                        ui.horizontal(|ui| {
                            let hint = if inputs == 2 { "from" } else { "value" };
                            let r = ui.add(TextEdit::singleline(&mut h.search_text).hint_text(hint).desired_width(110.0));
                            let mut enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if inputs == 2 {
                                let r2 = ui.add(TextEdit::singleline(&mut h.search_text2).hint_text(tr("to")).desired_width(110.0));
                                enter |= r2.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            }
                            if enter {
                                if scanning { refine = true } else { start = true }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui.button(if scanning { tr("Start over") } else { tr("Search") }).clicked() {
                            if scanning {
                                h.set_results(Vec::new());
                                h.searched = false;
                            } else {
                                start = true;
                            }
                        }
                        if ui
                            .add_enabled(scanning, egui::Button::new(tr("Refine")))
                            .on_hover_text(tr("Re-reads the file from disk (save in-game first) and keeps only the locations that pass the test above"))
                            .clicked()
                        {
                            refine = true;
                        }
                        crate::widgets::checkbox(ui, &mut h.aligned, tr("Aligned")).on_hover_text(tr("Only test offsets that are a multiple of the value size. Faster, and how most games store numbers."));
                    });
                    if scanning && !WEB && !guide {
                        crate::widgets::checkbox(ui, &mut h.auto_refine, tr("Refine by itself each time the game saves"))
                            .on_hover_text(tr("Watches the file and refines with the test above as soon as the game writes it again"));
                    }
                    if start {
                        match h.start_scan(data) {
                            Ok(m) => status = Some(Ok(m)),
                            Err(e) => status = Some(Err(e)),
                        }
                    }
                    if h.all_locations {
                        ui.label(tr("Every location is still in play."));
                    }
                    if h.searched {
                        ui.label(if h.results.len() >= hex_view::MAX_RESULTS { trf("{0} result(s) (capped)", &[&h.results.len()]) } else { trf("{0} result(s)", &[&h.results.len()]) });
                        let mut jump = None;
                        ScrollArea::vertical().show_list(ui, "results", 220.0, |ui| {
                            for &off in h.results.iter().take(1000) {
                                let cur = hex_view::read_value(h.search_type, data, off, h.big_endian).unwrap_or_default();
                                let name = h.labels.get(&off).map(|l| format!("  [{}]", l.name)).unwrap_or_default();
                                if ui.selectable_label(off == h.selected, RichText::new(format!("0x{off:08X}   {cur}{name}")).monospace()).clicked() {
                                    jump = Some(off);
                                }
                            }
                        });
                        if let Some(o) = jump {
                            h.jump(o);
                            h.edit_type = h.search_type;
                            h.edit_text = hex_view::read_value(h.search_type, data, o, h.big_endian).unwrap_or_default();
                        }
                    }
                });

                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(crate::theme::panel_title(format!("Selected: 0x{:X}", h.selected)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            crate::widgets::checkbox(ui, &mut h.big_endian, tr("Big-endian")).on_hover_text(tr("Most PC games use little-endian (unticked)"));
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("Go to"));
                        let r = ui.add(TextEdit::singleline(&mut h.goto_text).hint_text(tr("hex offset")).desired_width(100.0));
                        if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || crate::widgets::small_button(ui, tr("Go")).clicked() {
                            match usize::from_str_radix(h.goto_text.trim().trim_start_matches("0x"), 16) {
                                Ok(o) if o < data.len() => h.jump(o),
                                _ => status = Some(Err(tr("That offset is outside the file").into())),
                            }
                        }
                    });
                    egui::Grid::new("inspector").striped(true).num_columns(2).show(ui, |ui| {
                        for ty in NumType::ALL.into_iter().filter(|t| !matches!(t, NumType::Ascii | NumType::Utf16)) {
                            ui.monospace(ty.name());
                            let v = hex_view::read_value(ty, data, h.selected, h.big_endian).unwrap_or_else(|| "\u{2014}".into());
                            if ui
                                .add(egui::Label::new(RichText::new(&v).monospace()).sense(egui::Sense::click()))
                                .on_hover_text(tr("Click to edit as this type"))
                                .clicked()
                            {
                                h.edit_type = ty;
                                h.edit_text = v;
                            }
                            ui.end_row();
                        }
                    });
                    ui.horizontal(|ui| {
                        type_combo(ui, "edit_type", &mut h.edit_type);
                        ui.add(TextEdit::singleline(&mut h.edit_text).hint_text(tr("new value")).desired_width(110.0));
                        if ui.button(tr("Write")).clicked() {
                            match hex_view::encode_value(h.edit_type, &h.edit_text, h.big_endian) {
                                Ok(b) if h.selected + b.len() <= data.len() => {
                                    status = Some(Ok(trf("Wrote {0} byte(s) at 0x{1}", &[&b.len(), &format!("{:X}", h.selected)])));
                                    byte_edit = Some((h.selected, b));
                                }
                                Ok(_) => status = Some(Err(tr("That value runs past the end of the file").into())),
                                Err(e) => status = Some(Err(trf("Invalid value: {0}", &[&e]))),
                            }
                        }
                    });
                });
            });
        };
        if wide {
            egui::SidePanel::right("hex_tools").default_width(340.0).min_width(300.0).show(ctx, body);
        } else {
            let h = ctx.screen_rect().height() * 0.42;
            egui::TopBottomPanel::bottom("hex_tools_bottom").resizable(true).default_height(h).min_height(120.0).show(ctx, body);
        }
        if let Some((o, b)) = byte_edit {
            self.apply_bytes(o, b);
        }
        match status {
            Some(Ok(m)) => self.ok(m),
            Some(Err(m)) => self.fail(m),
            None => {}
        }
        if labels_changed {
            self.save_labels();
        }
        if hide_guide || show_guide {
            self.settings.hex_guide = show_guide;
            self.settings.save();
        }
        match names_action {
            Some(NamesAction::Share) => self.start_share_names(),
            Some(NamesAction::Export) => {
                let name = self.doc.as_ref().map(|d| format!("{}.names.json", file_name(&d.path)));
                self.ask(Ask::ExportNames, None, name);
            }
            Some(NamesAction::Import) => self.ask(Ask::ImportNames, None, None),
            None => {}
        }
        if refine {
            self.refine_from_disk();
        }
        if let Some(e) = open_entry {
            self.reopen_with(|o| o.zip_entry = Some(e));
        }
    }
}
