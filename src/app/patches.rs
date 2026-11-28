//! Tools › Cheat patches.

use super::*;

/// Tools › Cheat patches: the patch file picked and the codes ticked in it.
pub(super) struct Patches {
    pub(super) query: String,
    /// The game's patch files found from the save's name, then search results.
    pub(super) found: Vec<&'static crate::apollo::db::Entry>,
    /// The patch file shown: a label, its codes and the save's byte order.
    pub(super) label: String,
    pub(super) codes: Vec<crate::apollo::Code>,
    pub(super) on: Vec<bool>,
    pub(super) picks: Vec<Vec<usize>>,
    pub(super) big_endian: bool,
    /// The save file's own name (codes say which file they're for).
    pub(super) file: String,
    /// Show codes for other files too.
    pub(super) all_files: bool,
    /// The save opened through its patch file's own unpacking codes (and the unpacked stream
    /// shown, if any): those required codes have already run.
    pub(super) layer: Option<Option<String>>,
}

impl Patches {
    /// Whether code `i` is listed: it's for this file, or every file is being shown. A heading
    /// is listed when something under it is.
    pub(super) fn listed(&self, i: usize) -> bool {
        let c = &self.codes[i];
        if c.required {
            return false;
        }
        if !c.empty() {
            return self.all_files || crate::apollo::targets(c, &self.file);
        }
        let next = self.codes[i + 1..].iter().position(|n| n.empty()).map_or(self.codes.len(), |k| i + 1 + k);
        (i + 1..next).any(|j| self.listed(j)) || (next == i + 1 && !c.parent)
    }

    pub(super) fn ticked(&self) -> usize {
        (0..self.codes.len()).filter(|&i| self.on[i] && self.listed(i)).count()
    }

    pub(super) fn show(&mut self, label: String, codes: Vec<crate::apollo::Code>, big_endian: bool) {
        self.on = codes.iter().map(|c| c.default_on && !c.required && c.kind != crate::apollo::Kind::Python).collect();
        self.picks = codes.iter().map(|c| vec![0; c.choices.len()]).collect();
        self.codes = codes;
        self.label = label;
        self.big_endian = big_endian;
    }
}

impl App {
    pub(super) fn ids_in_save(&self) -> Vec<u64> {
        match self.doc.as_ref().map(|d| &d.content) {
            Some(Content::Binary(b)) => crate::steamid::in_bytes(b).into_iter().map(|(id, _)| id).collect(),
            Some(c) => root(c).map(|v| crate::steamid::in_values(v).into_iter().map(|(id, _)| id).collect()).unwrap_or_default(),
            None => vec![],
        }
    }

    pub(super) fn start_patches(&mut self) {
        use crate::apollo::db;
        let Some(d) = &self.doc else { return };
        let inner = d.layers.iter().rev().find_map(|l| match l {
            Layer::ZipEntry { name, .. } | Layer::Copeck { name, .. } => Some(name.clone()),
            Layer::Ps2Card { file, .. } => Some(file.name.clone()),
            Layer::PsSave { name, .. } => Some(name.clone()),
            _ => None,
        });
        let layer = d.layers.iter().find_map(|l| if let Layer::Apollo { var, .. } = l { Some(var.clone()) } else { None });
        let path = d.path.to_string_lossy().replace('\\', "/");
        let full = match &inner {
            Some(n) => format!("{path}/{n}"),
            None => path.clone(),
        };
        let mut file = full.rsplit('/').next().unwrap_or_default().to_string();
        if let Some(Some(v)) = &layer {
            file = v.clone();
        }
        let found: Vec<_> = db::ids_in(&full).iter().flat_map(|id| db::find(id)).collect();
        let mut p = Patches {
            query: String::new(),
            found,
            label: String::new(),
            codes: vec![],
            on: vec![],
            picks: vec![],
            big_endian: false,
            file,
            all_files: false,
            layer,
        };
        if let [e] = p.found[..] {
            p.show(e.title.clone() + " (" + &e.platform + ")", e.codes(), e.big_endian());
        }
        self.patches = Some(p);
    }

    pub(super) fn load_patch_file(&mut self, path: &Path) {
        let Some(p) = &mut self.patches else { return };
        let raw = match crate::vfs::read(path) {
            Ok(b) => b,
            Err(e) => return self.fail(trf("Couldn't read {0}: {1}", &[&path.display(), &e])),
        };
        let (text, latin1) = match String::from_utf8(raw) {
            Ok(t) => (t, false),
            Err(e) => (e.into_bytes().iter().map(|&b| b as char).collect(), true),
        };
        let codes = crate::apollo::parse(&text, latin1);
        if codes.is_empty() {
            return self.fail(tr("That file has no cheat codes in it."));
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let (id, title) = crate::apollo::header(&text);
        let be = crate::apollo::db::ps3_id(if id.is_empty() { &name } else { &id });
        p.show(if title.is_empty() { name } else { title }, codes, be);
    }

    pub(super) fn patches_dialog(&mut self, ctx: &egui::Context) {
        use crate::apollo::Kind;
        let Some(mut p) = self.patches.take() else { return };
        let mut choice = None;
        let mut open_file = false;
        egui::Window::new(tr("Cheat patches"))
            .collapsible(false)
            .resizable(true)
            .default_size([fit(ctx, 560.0), 520.0])
            .max_width(fit(ctx, 1400.0))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Game:"));
                    let r = ui.add(TextEdit::singleline(&mut p.query).hint_text(tr("name or title ID (SLUS-20312, BLES00371\u{2026})")).desired_width(300.0));
                    if r.changed() && p.query.trim().len() >= 2 {
                        p.found = crate::apollo::db::search(&p.query, 40);
                    }
                    if !WEB && ui.button(tr("Open patch file\u{2026}")).clicked() {
                        open_file = true;
                    }
                });
                if !p.found.is_empty() {
                    ScrollArea::vertical().show_list_fit(ui, "patch_games", 110.0, |ui| {
                        for e in p.found.clone() {
                            let label = format!("{}  \u{2014}  {} ({})", e.title, e.id, e.platform);
                            if ui.selectable_label(p.label.starts_with(&e.title) && p.label.ends_with(&format!("({})", e.platform)), label).clicked() {
                                p.show(e.title.clone() + " (" + &e.platform + ")", e.codes(), e.big_endian());
                            }
                        }
                    });
                } else if p.codes.is_empty() {
                    ui.weak(tr("No patches were found from this save's file or folder name. Type the game's name or title ID."));
                }
                if p.codes.is_empty() {
                    ui.add_space(6.0);
                    if ui.button(tr("Close")).clicked() {
                        choice = Some(false);
                    }
                    return;
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(&p.label);
                    ui.weak(if p.big_endian { "big-endian" } else { "little-endian" });
                });
                let any_here = p.codes.iter().any(|c| !c.empty() && crate::apollo::targets(c, &p.file));
                if !any_here && !p.all_files {
                    ui.weak(trf("None of these codes name \u{201C}{0}\u{201D}; they may be for another file in the save folder.", &[&p.file]));
                }
                if p.layer.is_some() {
                    ui.weak(tr("This save was unpacked with the patch file's own codes when it opened; they run again when you save."));
                }
                crate::widgets::checkbox(ui, &mut p.all_files, tr("Show codes for other files too"));
                ScrollArea::vertical().show_list_fit(ui, "patch_codes", 300.0, |ui| {
                    for i in 0..p.codes.len() {
                        let c = &p.codes[i];
                        if !p.listed(i) {
                            continue;
                        }
                        if c.empty() {
                            ui.add_space(4.0);
                            ui.strong(&c.name);
                            continue;
                        }
                        let here = crate::apollo::targets(c, &p.file);
                        let usable = c.kind != Kind::Python;
                        ui.horizontal(|ui| {
                            if c.child {
                                ui.add_space(16.0);
                            }
                            // A code the patch flags as risky: a warning sign as well as the colour.
                            let mut name = RichText::new(if c.alert { format!("\u{26A0} {}", c.name) } else { c.name.clone() });
                            if c.alert {
                                name = name.color(theme::fg(ui, theme::WARN));
                            }
                            let r = widgets::square(ui, |ui| ui.add_enabled(usable, egui::Checkbox::new(&mut p.on[i], name)));
                            if !usable {
                                r.on_disabled_hover_text(tr("This one is a Python script, which only Apollo itself can run."));
                            } else if !here {
                                r.on_hover_text(trf("Written for {0}", &[&c.file]));
                            }
                            for (k, ch) in p.codes[i].choices.clone().iter().enumerate() {
                                let cur = p.picks[i][k];
                                let shown = ch.values.get(cur).map(|v| v.1.clone()).unwrap_or_default();
                                egui::ComboBox::from_id_salt(("patch_pick", i, k)).selected_text(shown).show_ui(ui, |ui| {
                                    for (j, (_, n)) in ch.values.iter().enumerate() {
                                        ui.selectable_value(&mut p.picks[i][k], j, n);
                                    }
                                });
                            }
                        });
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    crate::widgets::checkbox(ui, &mut p.big_endian, tr("Big-endian (PS3)"))
                        .on_hover_text(tr("Byte order for numbers in the codes. Set from the patch file's console; change it only if you know the save uses the other one."));
                });
                ui.weak(tr("Back up first: a code meant for another version of the game can damage the save. Codes change the data now; save to write the file."));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let n = p.ticked();
                    if ui.add_enabled(n > 0, egui::Button::new(RichText::new(if n == 1 { tr("Apply 1 code").to_string() } else { trf("Apply {0} codes", &[&n]) }).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                        choice = Some(true);
                    }
                    if ui.button(tr("Close")).clicked() {
                        choice = Some(false);
                    }
                });
            });
        match choice {
            Some(true) => self.apply_patches(&p),
            Some(false) => {}
            None => self.patches = Some(p),
        }
        if open_file {
            self.ask(Ask::Patch, None, None);
        }
    }

    /// Runs the ticked codes (and the patch file's required ones) in order, as one undo step.
    pub(super) fn apply_patches(&mut self, p: &Patches) {
        let Some(Doc { content: Content::Binary(data), .. }) = &self.doc else { return };
        let mut out = data.clone();
        let mut session = crate::apollo::Session::new(p.big_endian);
        if let Some(Some(v)) = &p.layer {
            session.seed(v, out.clone());
        }
        let (mut done, mut failed) = (0, vec![]);
        for (i, c) in p.codes.iter().enumerate() {
            let ticked = p.on[i] && p.listed(i);
            let wanted = ticked || (c.required && p.layer.is_none() && (p.all_files || crate::apollo::targets(c, &p.file)));
            if !wanted || c.empty() {
                continue;
            }
            match session.apply(&mut out, c, &p.picks[i]) {
                Ok(()) => done += usize::from(ticked),
                Err(e) => failed.push(format!("{}: {e}", c.name)),
            }
        }
        if let Some(Some(v)) = &p.layer {
            out = session.unpacked(v).unwrap_or(&out).to_vec();
        }
        if out == *data {
            if failed.is_empty() {
                self.info(tr("The codes didn't change anything (the values may already be set)."));
            } else {
                self.fail(trf("Nothing was changed. {0}", &[&failed.join("; ")]));
            }
            return;
        }
        self.replace_bytes(out);
        if failed.is_empty() {
            self.ok(if done == 1 { tr("Applied 1 code. Save to write the file.").to_string() } else { trf("Applied {0} codes. Save to write the file.", &[&done]) });
        } else {
            self.fail(trf("Applied {0}, but some failed: {1}", &[&done, &failed.join("; ")]));
        }
    }
}
