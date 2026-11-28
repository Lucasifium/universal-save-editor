//! Opening files: reading and decoding them (on a worker thread for big ones), the names
//! labels kept beside a save, and reading a file again with other options.

use super::*;

/// Files at least this big are opened on a worker thread, so the window keeps responding.
/// (Working out what a file is takes longer than its size suggests: a save well under a
/// megabyte can hold half a million values.)
pub(super) const BACKGROUND_LOAD: u64 = 256 << 10;

/// How long a file may take to open before the "Opening" box is shown, so that it doesn't
/// flash up for the many files that open at once.
pub(super) const OPENING_BOX_AFTER: f64 = 0.25;

/// A file being read and decoded on a worker thread.
pub(super) struct Loading {
    pub(super) path: PathBuf,
    pub(super) opts: formats::Options,
    pub(super) started: f64,
    /// What the worker is up to, and the Cancel button's flag.
    pub(super) job: std::sync::Arc<crate::progress::Job>,
    pub(super) rx: std::sync::mpsc::Receiver<Result<(usize, Prepared), String>>,
}

/// Everything about an opened file that can be worked out away from the window: the
/// decoded data and its copy for "modified", names, the game screen, the Values list.
pub(super) struct Prepared {
    pub(super) decoded: formats::Decoded,
    pub(super) baseline: Content,
    pub(super) names: Names,
    pub(super) note: String,
    pub(super) game: Option<crate::game_ui::Game>,
    pub(super) model: Model,
    pub(super) running: Option<String>,
    /// Taken just before the file was read.
    pub(super) stamp: Option<Stamp>,
    /// It has Japanese, Chinese or Korean text (the fonts for it are loaded when needed).
    pub(super) cjk: bool,
}

pub(super) fn prepare(path: &Path, decoded: formats::Decoded) -> Prepared {
    let mut note = String::new();
    let mut names = Names::default();
    match &decoded.content {
        Content::Json { value, .. } if names::is_rpgmaker(value) => match names::find_rpgmaker_data(path) {
            Some(dir) => names = names::rpgmaker_names(value, &dir),
            None => {
                names = names.rpgmaker_layout();
                note = format!(" {}", tr("Game data folder not found. Use Tools \u{203A} Locate game data folder to see variable and item names."));
            }
        },
        Content::Tree(t) => {
            names.advanced = t.advanced.clone();
            if let Some(n) = &t.note {
                note = format!(" {}", crate::i18n::tr_any(n));
            }
        }
        _ => {}
    }
    if let Some(h) = &decoded.hint {
        note = format!(" {h}");
    }
    let game = match &decoded.content {
        Content::Tree(t) => crate::game_ui::detect(&t.kind_name, &t.view),
        Content::Json { value, .. } => crate::game_ui::detect("JSON", value),
        _ => None,
    };
    let model = match &decoded.content {
        Content::Json { value, .. } => values_view::build(value, &names, None),
        Content::Tree(t) => values_view::build(&t.view, &names, Some(&t.editable)),
        _ => Model::default(),
    };
    let cjk = safety::content_has_cjk(&decoded.content);
    Prepared { baseline: decoded.content.clone(), decoded, names, note, game, model, running: crate::running::game_for(path), stamp: None, cjk }
}

impl App {
    /// Opens a file the user picked: in a new tab when another file is open.
    pub(super) fn request_open(&mut self, path: PathBuf) {
        self.open_in_tab(path);
    }

    /// Opens `path`; a big file is read and decoded on a worker thread while the window
    /// shows that it's opening.
    pub(super) fn load_async(&mut self, path: &Path) {
        if self.saving.is_some() {
            return;
        }
        // Zips go to the worker too: what's inside can be far bigger than the file.
        let zipped = crate::vfs::head(path, 4).is_some_and(|m| m == b"PK\x03\x04");
        let big = zipped || crate::vfs::len(path).is_some_and(|n| n >= BACKGROUND_LOAD);
        // The browser version has no worker threads.
        if !big || WEB {
            return self.load(path);
        }
        let opts = formats::Options::for_path(path);
        let (tx, rx) = std::sync::mpsc::channel();
        let (p, o) = (path.to_path_buf(), opts.clone());
        let job = crate::progress::Job::new();
        let worker_job = job.clone();
        std::thread::spawn(move || {
            let _guard = crate::progress::install(worker_job);
            let stamp = safety::stamp(&p);
            let r = crate::progress::read_file(&p)
                .map_err(|e| trf("Couldn't open {0}: {1}", &[&p.display(), &e]))
                .map(|raw| {
                    let stamp = safety::stamp_with(stamp, &raw);
                    let decoded = formats::decode_with(&raw, &o);
                    crate::progress::stage(crate::progress::Stage::Showing);
                    (raw.len(), Prepared { stamp, ..prepare(&p, decoded) })
                });
            let _ = tx.send(r);
        });
        self.loading = Some(Loading { path: path.to_path_buf(), opts, started: f64::NAN, job, rx });
        crate::widgets::set_busy(true);
        self.info(trf("Opening {0}\u{2026}", &[&file_name(path)]));
    }

    /// The worker thread's result, once it's there.
    pub(super) fn poll_loading(&mut self, ctx: &egui::Context, now: f64) {
        let Some(l) = &mut self.loading else { return };
        if l.started.is_nan() {
            l.started = now;
            // Nothing typed from here on may land in the file that is about to be replaced.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        match l.rx.try_recv() {
            Ok(r) => {
                let l = self.loading.take().unwrap();
                match r {
                    Ok((len, decoded)) => self.finish_load(&l.path, l.opts, len, decoded),
                    Err(e) => self.fail(e),
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                let name = file_name(&l.path);
                let secs = now - l.started;
                if secs < OPENING_BOX_AFTER {
                    // Not worth a box yet, but clicks must not reach the file being replaced.
                    let screen = ctx.screen_rect();
                    egui::Area::new(egui::Id::new("opening_hold")).fixed_pos(screen.min).sense(widgets::BLOCK_CLICKS).order(egui::Order::Foreground).show(ctx, |ui| {
                        ui.allocate_response(screen.size(), widgets::BLOCK_CLICKS);
                    });
                    ctx.request_repaint_after(std::time::Duration::from_millis(30));
                    return;
                }
                let (stage, fraction) = (l.job.current_stage(), l.job.fraction());
                let mut cancel = false;
                modal(ctx, tr("Opening"), |ui| {
                    ui.label(trf("Opening {0}\u{2026}", &[&name]));
                    let what = match stage {
                        crate::progress::Stage::Reading => tr("Reading the file"),
                        crate::progress::Stage::Unpacking => tr("Unpacking"),
                        crate::progress::Stage::Values => tr("Reading the values"),
                        crate::progress::Stage::Showing => tr("Getting the list ready"),
                    };
                    match fraction {
                        Some(f) => {
                            ui.add(egui::ProgressBar::new(f).desired_width(360.0).show_percentage());
                            ui.weak(what);
                        }
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.weak(what);
                            });
                        }
                    }
                    if secs > 2.0 {
                        ui.weak(trf("{0} s. Big saves take a moment.", &[&format!("{secs:.0}")]));
                    }
                    cancel = ui.button(tr("Cancel")).clicked();
                });
                if cancel {
                    self.cancel_loading();
                    return;
                }
                ctx.request_repaint_after(std::time::Duration::from_millis(60));
            }
            Err(_) => {
                self.loading = None;
                self.fail(tr("Opening the file stopped unexpectedly."));
            }
        }
    }

    /// Cancel (or Escape) in the Opening box: the worker is told to stop, and whatever it
    /// still produces is thrown away. The file that was open stays as it was.
    pub(super) fn cancel_loading(&mut self) {
        if let Some(l) = self.loading.take() {
            l.job.cancel();
            self.info(trf("Stopped opening {0}.", &[&file_name(&l.path)]));
        }
    }

    // ------------------------------------------------------------ loading

    pub(super) fn load(&mut self, path: &Path) {
        self.load_with(path, formats::Options::for_path(path));
    }

    /// Load with explicit options (password, zip entry).
    pub(super) fn load_with(&mut self, path: &Path, opts: formats::Options) {
        // The file on screen is being saved: it stays until that is done.
        if self.saving.is_some() {
            return;
        }
        let stamp = safety::stamp(path);
        let raw = match crate::vfs::read(path) {
            Ok(r) => r,
            Err(e) => return self.fail(trf("Couldn't open {0}: {1}", &[&path.display(), &e])),
        };
        let stamp = safety::stamp_with(stamp, &raw);
        let prepared = Prepared { stamp, ..prepare(path, formats::decode_with(&raw, &opts)) };
        self.finish_load(path, opts, raw.len(), prepared);
    }

    /// Shows a decoded file (from `load_with` or the worker thread).
    pub(super) fn finish_load(&mut self, path: &Path, opts: formats::Options, raw_len: usize, p: Prepared) {
        let Prepared { decoded, baseline, names, note, game, model, running, stamp, cjk } = p;
        // Opened without the game screen (File \u{203A} Open as).
        let game = game.filter(|_| opts.read_as != formats::ReadAs::NoScreen);
        self.need_cjk |= cjk || theme::has_cjk(&path.to_string_lossy());
        let desc = friendly_kind(&decoded.layers, &decoded.content);
        if let Some(n) = game.and_then(|g| g.name()).filter(|n| crate::games::GAMES.iter().any(|g| g.name == *n)) {
            self.settings.push_recent_game(n);
        }
        self.doc = Some(Doc {
            path: path.to_path_buf(),
            layers: decoded.layers,
            baseline,
            content: decoded.content,
            names,
            opts,
            zip_entries: decoded.zip_entries,
            game,
            stamp,
            handle: None,
        });
        self.loads += 1;
        self.disk_changed = false;
        self.disk_seen = None;
        self.stale_save = None;
        self.stale_ok = false;
        self.undo.clear();
        self.redo.clear();
        self.touched.clear();
        self.edit_gen += 1;
        self.last_edit = None;
        self.dirty = false;
        self.tab = if game.is_some() { Tab::Game } else { Tab::Values };
        self.game_section.clear();
        self.advanced_tab = Tab::Values;
        self.search.clear();
        self.collapsed.clear();
        self.collapsed_gen += 1;
        self.raw_error = None;
        self.hex = HexState::default();
        self.load_labels();
        self.model = model;
        self.model_gen += 1;
        self.model_from_baseline = true;
        self.settings.push_recent(path);
        self.running = running;
        self.doc_note = note.trim().to_string();
        // A save without a game screen names its game when its folder and name give it away.
        let what = match game.is_none().then(|| crate::game_match::by_path(path)).flatten().filter(|g| !desc.contains(g.name)) {
            Some(g) => trf("{0} save ({1})", &[&g.name, &kind_label(&desc)]),
            None => kind_label(&desc),
        };
        self.info(crate::i18n::trf("Opened {0} ({1}) \u{2014} {2}.", &[&file_name(path), &human_size(raw_len), &what]));
    }

    pub(super) fn rebuild_model(&mut self) {
        self.model = match &self.doc {
            Some(d) => match &d.content {
                Content::Json { value, .. } => values_view::build(value, &d.names, None),
                Content::Tree(t) => values_view::build(&t.view, &d.names, Some(&t.editable)),
                _ => Model::default(),
            },
            None => Model::default(),
        };
        self.model_gen += 1;
        self.model_from_baseline = false;
    }

    pub(super) fn locate_data_dir(&mut self) {
        self.ask(Ask::DataDir, None, None);
    }

    pub(super) fn use_data_dir(&mut self, dir: PathBuf) {
        let Some(doc) = &self.doc else { return };
        let Content::Json { value, .. } = &doc.content else { return };
        if !dir.join("System.json").is_file() {
            return self.fail(trf("{0} has no System.json \u{2014} pick the game's data folder.", &[&dir.display()]));
        }
        let names = names::rpgmaker_names(value, &dir);
        let n = names.by_ptr.len();
        self.doc.as_mut().unwrap().names = names;
        self.rebuild_model();
        self.ok(trf("Loaded {0} names from {1}", &[&n, &dir.display()]));
    }

    pub(super) fn labels_path(&self) -> Option<PathBuf> {
        let mut p = self.doc.as_ref()?.path.as_os_str().to_owned();
        p.push(".labels.json");
        Some(PathBuf::from(p))
    }

    pub(super) fn load_labels(&mut self) {
        let Some(path) = self.labels_path() else { return };
        let Ok(text) = crate::vfs::read_to_string(path) else { return };
        let Ok(v) = serde_json::from_str::<Value>(&text) else { return };
        for l in v["labels"].as_array().into_iter().flatten() {
            let (Some(off), Some(name), Some(ty)) =
                (l["offset"].as_u64(), l["name"].as_str(), l["type"].as_str().and_then(NumType::from_name))
            else {
                continue;
            };
            self.hex.labels.insert(off as usize, OffsetLabel { name: name.to_string(), ty });
        }
    }

    pub(super) fn save_labels(&mut self) {
        let Some(path) = self.labels_path() else { return };
        let list: Vec<Value> =
            self.hex.labels.iter().map(|(o, l)| json!({"offset": o, "name": l.name, "type": l.ty.name()})).collect();
        let result = if list.is_empty() {
            if crate::vfs::exists(&path) { crate::vfs::remove_file(&path) } else { Ok(()) }
        } else {
            crate::vfs::write(&path, serde_json::to_string_pretty(&json!({ "labels": list })).unwrap())
        };
        if let Err(e) = result {
            self.fail(trf("Couldn't save names to {0}: {1}", &[&path.display(), &e]));
        }
    }

    pub(super) fn view_as_hex(&mut self) {
        if self.dirty {
            return self.fail(tr("Save or revert your changes before switching to the hex view."));
        }
        let Some(doc) = &mut self.doc else { return };
        match doc.content.to_bytes() {
            Ok(b) => {
                doc.content = Content::Binary(b);
                doc.baseline = doc.content.clone();
                self.touched.clear();
                self.edit_gen += 1;
                self.rebuild_model();
                self.ok(tr("Showing the decoded data as hex. Saving re-applies the original encoding."));
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn refine_from_disk(&mut self) {
        if self.dirty {
            return self.fail(tr("Refine re-reads the file from disk. Save or revert your changes first."));
        }
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        let raw = match crate::vfs::read(&path) {
            Ok(r) => r,
            Err(e) => return self.fail(trf("Reload failed: {0}", &[&e])),
        };
        let opts = self.doc.as_ref().map(|d| d.opts.clone()).unwrap_or_default();
        let decoded = formats::decode_with(&raw, &opts);
        let bytes = match decoded.content.to_bytes() {
            Ok(b) => b,
            Err(e) => return self.fail(e),
        };
        let before_bytes = match self.doc.as_ref().map(|d| d.content.to_bytes()) {
            Some(Ok(b)) => b,
            _ => Vec::new(),
        };
        let (before, after) = match self.hex.refine(&before_bytes, &bytes) {
            Ok(n) => n,
            Err(e) => return self.fail(e),
        };
        let d = self.doc.as_mut().unwrap();
        d.layers = decoded.layers;
        d.content = Content::Binary(bytes);
        d.baseline = d.content.clone();
        self.undo.clear();
        self.redo.clear();
        self.edit_gen += 1;
        self.recompute_dirty();
        self.info(trf("Re-read the file: {0} \u{203A} {1} possible locations.", &[&before, &after]));
    }

    /// Re-read the open file with changed decode options (password, zip entry).
    pub(super) fn reopen_with(&mut self, change: impl FnOnce(&mut formats::Options)) {
        if self.dirty {
            return self.fail(tr("Save or revert your changes first."));
        }
        let Some(d) = &self.doc else { return };
        let path = d.path.clone();
        let mut opts = d.opts.clone();
        change(&mut opts);
        // The same file read another way (Open as, a password): it still saves back through
        // the handle it was opened with (browser version).
        let handle = self.doc.as_mut().and_then(|d| d.handle.take());
        self.load_with(&path, opts);
        if let Some(d) = self.doc.as_mut().filter(|d| d.path == path && d.handle.is_none()) {
            d.handle = handle;
        }
    }
}
