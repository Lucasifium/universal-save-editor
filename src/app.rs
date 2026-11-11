use crate::file_browser::{self, FileBrowser};
use crate::formats::{self, Content, Layer};
use crate::platform;
use crate::hex_view::{self, HexState, NumType, OffsetLabel};
use crate::json_view::{self, TreeCtx};
use crate::names::{self, Names};
use crate::settings::{self, Settings};
use crate::theme::{self, icon};
use crate::values_view::{self, Disp, Model, ViewCtx};
use crate::widgets;
use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The browser version: no folders to look through, files arrive from the page's picker and
/// leave as downloads.
const WEB: bool = cfg!(target_arch = "wasm32");

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    /// The game's own screen (character sheet, inventory…), for games that have one.
    Game,
    Values,
    Tree,
    Raw,
}

/// A file or folder the app is waiting for the user to choose.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ask {
    Open,
    DataDir,
    SaveAs,
    Export,
    Compare,
    BackupDir,
    Import,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum StatusKind {
    Info,
    Ok,
    Err,
}

enum Change {
    /// `extra`: values that changed along with it (pointer, before, after).
    Value { ptr: String, before: Value, after: Value, extra: Vec<(String, Value, Value)> },
    Bytes { off: usize, before: Vec<u8>, after: Vec<u8> },
}

struct Doc {
    path: PathBuf,
    layers: Vec<Layer>,
    content: Content,
    /// Content as last loaded/saved — the reference for "modified" and "revert".
    baseline: Content,
    names: Names,
    /// How the file was decoded (password, zip entry) — reused for re-reading/verifying.
    opts: formats::Options,
    /// Other files inside a zip archive, if this came from one.
    zip_entries: Vec<String>,
    /// The game screen for this save, if there is one.
    game: Option<crate::game_ui::Game>,
}

fn root(c: &Content) -> Option<&Value> {
    match c {
        Content::Json { value, .. } => Some(value),
        Content::Tree(t) => Some(&t.view),
        _ => None,
    }
}

/// Scalar values of a structured document by pointer (for comparing two saves).
fn flatten(v: &Value, ptr: &str, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(m) => m.iter().for_each(|(k, c)| flatten(c, &crate::tree::child(ptr, k), out)),
        Value::Array(a) => a.iter().enumerate().for_each(|(i, c)| flatten(c, &format!("{ptr}/{i}"), out)),
        other => out.push((ptr.to_string(), other.clone())),
    }
}

/// Result of comparing the open save with another file.
/// Files at least this big are opened on a worker thread, so the window keeps responding.
const BACKGROUND_LOAD: u64 = 4 << 20;

/// A file being read and decoded on a worker thread.
struct Loading {
    path: PathBuf,
    opts: formats::Options,
    started: f64,
    rx: std::sync::mpsc::Receiver<Result<(usize, Prepared), String>>,
}

/// Everything about an opened file that can be worked out away from the window: the
/// decoded data and its copy for "modified", names, the game screen, the Values list.
struct Prepared {
    decoded: formats::Decoded,
    baseline: Content,
    names: Names,
    note: String,
    game: Option<crate::game_ui::Game>,
    model: Model,
    running: Option<String>,
}

fn prepare(path: &Path, decoded: formats::Decoded) -> Prepared {
    let mut note = String::new();
    let mut names = Names::default();
    match &decoded.content {
        Content::Json { value, .. } if names::is_rpgmaker(value) => match names::find_rpgmaker_data(path) {
            Some(dir) => names = names::rpgmaker_names(value, &dir),
            None => {
                names = names.rpgmaker_layout();
                note = " Game data folder not found \u{2014} use Tools \u{203A} Locate game data folder for variable and item names.".into();
            }
        },
        Content::Tree(t) => {
            names.advanced = t.advanced.clone();
            if let Some(n) = &t.note {
                note = format!(" {n}");
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
    Prepared { baseline: decoded.content.clone(), decoded, names, note, game, model, running: crate::running::game_for(path) }
}

/// The "Select game" dialog: the game list, which game is picked, and the saves found for
/// each game (filled in by a background search).
struct GamePicker {
    query: String,
    picked: Option<usize>,
    /// For entries covering many games (Ren'Py, RPG Maker…): the game whose saves are shown.
    group: Option<String>,
    found: std::sync::Arc<std::sync::Mutex<Vec<Option<Vec<crate::games::Found>>>>>,
}

impl GamePicker {
    fn new() -> Self {
        let found = std::sync::Arc::new(std::sync::Mutex::new(vec![None; crate::games::GAMES.len()]));
        let out = found.clone();
        std::thread::spawn(move || {
            for (i, g) in crate::games::GAMES.iter().enumerate() {
                let f = crate::games::find(g);
                if let Ok(mut v) = out.lock() {
                    v[i] = Some(f);
                }
            }
        });
        Self { query: String::new(), picked: None, group: None, found }
    }
}

/// The "copy a character to another slot" dialog (Elden Ring).
struct CopySlots {
    slots: Vec<crate::fmt::slot_copy::Slot>,
    from: usize,
    to: usize,
}

/// The "about to save" dialog: what changed, and whether the game is open.
struct SaveReview {
    path: PathBuf,
    running: Option<String>,
    lines: Vec<String>,
}

struct Compare {
    other: PathBuf,
    /// (pointer, this file's value, other file's value)
    diffs: Vec<(String, Option<Value>, Option<Value>)>,
}

pub struct App {
    doc: Option<Doc>,
    settings: Settings,
    has_icons: bool,
    status: String,
    status_kind: StatusKind,
    pending_open: Option<PathBuf>,
    confirm_close: bool,
    allow_close: bool,
    window_title: String,
    logo: Option<egui::TextureHandle>,
    dirty: bool,

    // Editing history.
    undo: Vec<Change>,
    redo: Vec<Change>,
    last_edit: Option<(String, f64)>,
    touched: Vec<String>,

    // Structured view state.
    tab: Tab,
    search: String,
    /// When the search text last changed (big saves wait for typing to pause).
    search_typed_at: f64,
    focus_search: bool,
    model: Model,
    model_gen: u64,
    edit_gen: u64,
    collapsed: HashSet<usize>,
    collapsed_gen: u64,
    disp: Vec<Disp>,
    disp_key: Option<(String, bool, u64, u64, u64)>,
    scroll_to_row: Option<usize>,
    raw_text: String,
    raw_error: Option<String>,
    show_changes: bool,

    hex: HexState,
    /// Text typed into the "Decrypt with password" dialog while it's open.
    password_prompt: Option<String>,
    /// Checksums found by Tools › Find checksum, while their window is open.
    sum_hits: Option<Vec<crate::fmt::sumfind::Sum>>,
    /// The name being typed for a kept copy (File › Restore previous version › Keep a copy).
    backup_name: Option<String>,
    /// Tools › Change Steam account: the IDs found in the save, the one picked, and the new one typed.
    rebind: Option<(Vec<u64>, usize, String)>,
    compare: Option<Compare>,
    save_review: Option<SaveReview>,
    copy_slots: Option<CopySlots>,
    game_picker: Option<GamePicker>,
    loading: Option<Loading>,
    /// The game that owns the open save, if it was running when the file was opened.
    running: Option<String>,
    /// The built-in file browser, where there are no native dialogs (Android).
    browser: Option<(Ask, FileBrowser)>,
    #[cfg(target_os = "android")]
    keyboard_shown: bool,
    /// Browser version: what the file being picked is for, and the context to wake when it's in.
    #[cfg(target_arch = "wasm32")]
    web_asked: Option<Ask>,
    #[cfg(target_arch = "wasm32")]
    ctx: egui::Context,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<PathBuf>) -> Self {
        let settings = Settings::load();
        crate::backups::set_root(settings.backup_dir.clone());
        let has_icons = theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, settings.dark);
        cc.egui_ctx.set_zoom_factor(settings.zoom);
        let mut app = Self {
            doc: None,
            settings,
            has_icons,
            status: "Ready. Open a save file to get started.".into(),
            status_kind: StatusKind::Info,
            pending_open: None,
            confirm_close: false,
            allow_close: false,
            window_title: String::new(),
            logo: None,
            dirty: false,
            undo: vec![],
            redo: vec![],
            last_edit: None,
            touched: vec![],
            tab: Tab::Values,
            search: String::new(),
            search_typed_at: 0.0,
            focus_search: false,
            model: Model::default(),
            model_gen: 0,
            edit_gen: 0,
            collapsed: HashSet::new(),
            collapsed_gen: 0,
            disp: vec![],
            disp_key: None,
            scroll_to_row: None,
            raw_text: String::new(),
            raw_error: None,
            show_changes: false,
            hex: HexState::default(),
            password_prompt: None,
            sum_hits: None,
            backup_name: None,
            rebind: None,
            compare: None,
            save_review: None,
            copy_slots: None,
            game_picker: None,
            loading: None,
            running: None,
            browser: None,
            #[cfg(target_os = "android")]
            keyboard_shown: false,
            #[cfg(target_arch = "wasm32")]
            web_asked: None,
            #[cfg(target_arch = "wasm32")]
            ctx: cc.egui_ctx.clone(),
        };
        // "--select-game" opens the game list instead of a file; "--select-game=elden" also
        // picks the first game whose name contains that.
        let initial = match initial {
            Some(p) if p.to_string_lossy().starts_with("--select-game") => {
                let mut gp = GamePicker::new();
                if let Some((_, q)) = p.to_string_lossy().split_once('=') {
                    let q = q.to_lowercase();
                    gp.picked = crate::games::GAMES.iter().position(|g| g.name.to_lowercase().contains(&q));
                }
                app.game_picker = Some(gp);
                None
            }
            other => other,
        };
        if let Some(p) = initial {
            app.load_async(&p);
        }
        app
    }

    fn info(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_kind = StatusKind::Info;
    }
    fn ok(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_kind = StatusKind::Ok;
    }
    fn fail(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_kind = StatusKind::Err;
    }

    /// Asks for a file or folder: a native dialog on desktop, the built-in browser on Android.
    fn ask(&mut self, what: Ask, dir: Option<PathBuf>, name: Option<String>) {
        let dir = dir.or_else(|| self.doc.as_ref().and_then(|d| d.path.parent().map(Path::to_path_buf)));
        let (title, mode) = match what {
            Ask::Open => ("Open a save file", file_browser::Mode::OpenFile),
            Ask::DataDir => ("Select the game's data folder (it contains System.json)", file_browser::Mode::Folder),
            Ask::SaveAs => ("Save a copy as", file_browser::Mode::SaveFile),
            Ask::Export => ("Export decoded data as", file_browser::Mode::SaveFile),
            Ask::Compare => ("Compare with which save?", file_browser::Mode::OpenFile),
            Ask::BackupDir => ("Keep backups in which folder?", file_browser::Mode::Folder),
            Ask::Import => ("Import which edited data?", file_browser::Mode::OpenFile),
        };
        #[cfg(target_os = "android")]
        {
            self.browser = Some((what, FileBrowser::new(mode, title, dir, name)));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (title, dir);
            match mode {
                file_browser::Mode::OpenFile => {
                    self.web_asked = Some(what);
                    crate::web::pick_file(&self.ctx);
                }
                file_browser::Mode::SaveFile => self.answered(what, PathBuf::from(name.unwrap_or_else(|| "save".into()))),
                file_browser::Mode::Folder => self.fail("The browser version can't open folders \u{2014} use the desktop app for this."),
            }
        }
        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        {
            let mut dlg = rfd::FileDialog::new().set_title(title);
            if let Some(d) = dir {
                dlg = dlg.set_directory(d);
            }
            if let Some(n) = name {
                dlg = dlg.set_file_name(n);
            }
            let picked = match mode {
                file_browser::Mode::OpenFile => dlg.pick_file(),
                file_browser::Mode::Folder => dlg.pick_folder(),
                file_browser::Mode::SaveFile => dlg.save_file(),
            };
            if let Some(p) = picked {
                self.answered(what, p);
            }
        }
    }

    fn answered(&mut self, what: Ask, p: PathBuf) {
        match what {
            Ask::Open => self.request_open(p),
            Ask::DataDir => self.use_data_dir(p),
            Ask::SaveAs => self.save_to(&p),
            Ask::Export => self.export_to(&p),
            Ask::Compare => self.compare_with(p),
            Ask::Import => self.import_from(&p),
            Ask::BackupDir => {
                crate::backups::set_root(Some(p.clone()));
                self.ok(format!("Backups now go to {}. Earlier ones stay where they were.", p.display()));
                self.settings.backup_dir = Some(p);
                self.settings.save();
            }
        }
    }

    fn browser_window(&mut self, ctx: &egui::Context) {
        let Some((what, b)) = &mut self.browser else { return };
        let what = *what;
        match b.show(ctx) {
            Some(file_browser::Outcome::Picked(p)) => {
                self.browser = None;
                self.answered(what, p);
            }
            Some(file_browser::Outcome::Cancelled) => self.browser = None,
            None => {}
        }
    }

    fn request_open(&mut self, path: PathBuf) {
        if self.dirty {
            self.pending_open = Some(path);
        } else {
            self.load_async(&path);
        }
    }

    /// Opens `path`; a big file is read and decoded on a worker thread while the window
    /// shows that it's opening.
    fn load_async(&mut self, path: &Path) {
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
        std::thread::spawn(move || {
            let r = crate::vfs::read(&p)
                .map_err(|e| format!("Couldn't open {}: {e}", p.display()))
                .map(|raw| (raw.len(), prepare(&p, formats::decode_with(&raw, &o))));
            let _ = tx.send(r);
        });
        self.loading = Some(Loading { path: path.to_path_buf(), opts, started: f64::NAN, rx });
        self.info(format!("Opening {}\u{2026}", file_name(path)));
    }

    /// The worker thread's result, once it's there.
    fn poll_loading(&mut self, ctx: &egui::Context, now: f64) {
        let Some(l) = &mut self.loading else { return };
        if l.started.is_nan() {
            l.started = now;
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
                modal(ctx, "Opening", |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("Reading {name}\u{2026}"));
                    });
                    if secs > 2.0 {
                        ui.weak(format!("{secs:.0} s \u{2014} big saves take a moment."));
                    }
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(_) => {
                self.loading = None;
                self.fail("Opening the file stopped unexpectedly.");
            }
        }
    }

    // ------------------------------------------------------------ loading

    fn load(&mut self, path: &Path) {
        self.load_with(path, formats::Options::for_path(path));
    }

    /// Load with explicit options (password, zip entry).
    fn load_with(&mut self, path: &Path, opts: formats::Options) {
        let raw = match crate::vfs::read(path) {
            Ok(r) => r,
            Err(e) => return self.fail(format!("Couldn't open {}: {e}", path.display())),
        };
        let prepared = prepare(path, formats::decode_with(&raw, &opts));
        self.finish_load(path, opts, raw.len(), prepared);
    }

    /// Shows a decoded file (from `load_with` or the worker thread).
    fn finish_load(&mut self, path: &Path, opts: formats::Options, raw_len: usize, p: Prepared) {
        let Prepared { decoded, baseline, names, note, game, model, running } = p;
        let desc = friendly_kind(&decoded.layers, &decoded.content);
        self.doc = Some(Doc {
            path: path.to_path_buf(),
            layers: decoded.layers,
            baseline,
            content: decoded.content,
            names,
            opts,
            zip_entries: decoded.zip_entries,
            game,
        });
        self.undo.clear();
        self.redo.clear();
        self.touched.clear();
        self.last_edit = None;
        self.dirty = false;
        self.tab = if game.is_some() { Tab::Game } else { Tab::Values };
        self.search.clear();
        self.collapsed.clear();
        self.collapsed_gen += 1;
        self.raw_error = None;
        self.hex = HexState::default();
        self.load_labels();
        self.model = model;
        self.model_gen += 1;
        self.settings.push_recent(path);
        self.running = running;
        self.info(format!("Opened {} ({}) \u{2014} {desc}.{note}", file_name(path), human_size(raw_len)));
    }

    fn rebuild_model(&mut self) {
        self.model = match &self.doc {
            Some(d) => match &d.content {
                Content::Json { value, .. } => values_view::build(value, &d.names, None),
                Content::Tree(t) => values_view::build(&t.view, &d.names, Some(&t.editable)),
                _ => Model::default(),
            },
            None => Model::default(),
        };
        self.model_gen += 1;
    }

    fn locate_data_dir(&mut self) {
        self.ask(Ask::DataDir, None, None);
    }

    fn use_data_dir(&mut self, dir: PathBuf) {
        let Some(doc) = &self.doc else { return };
        let Content::Json { value, .. } = &doc.content else { return };
        if !dir.join("System.json").is_file() {
            return self.fail(format!("{} has no System.json \u{2014} pick the game's data folder.", dir.display()));
        }
        let names = names::rpgmaker_names(value, &dir);
        let n = names.by_ptr.len();
        self.doc.as_mut().unwrap().names = names;
        self.rebuild_model();
        self.ok(format!("Loaded {n} names from {}", dir.display()));
    }

    // ------------------------------------------------------------ editing

    fn root_mut(&mut self) -> Option<&mut Value> {
        match &mut self.doc.as_mut()?.content {
            Content::Json { value, .. } => Some(value),
            Content::Tree(t) => Some(t.view_and_editable().0),
            _ => None,
        }
    }

    /// Replace the value at `ptr`. Returns the old value and how many other places
    /// changed with it (values shared between variables), or why the format can't
    /// store it (e.g. out of range).
    fn set_value(&mut self, ptr: &str, v: Value) -> Result<Option<(Value, usize)>, String> {
        if let Some(Doc { content: Content::Tree(t), .. }) = &mut self.doc {
            return t.set(ptr, v);
        }
        let Some(target) = self.root_mut().and_then(|r| r.pointer_mut(ptr)) else { return Ok(None) };
        Ok(Some((std::mem::replace(target, v), 0)))
    }

    /// Apply a user edit, recording it for undo. Consecutive edits to the same field are merged.
    fn apply_edit(&mut self, ptr: String, v: Value, now: f64) {
        let (before, aliases) = match self.set_value(&ptr, v.clone()) {
            Ok(Some(r)) => r,
            Ok(None) => return,
            Err(e) => return self.fail(format!("Can't use that value: {e}.")),
        };
        let derived = match &mut self.doc {
            Some(Doc { content: Content::Tree(t), .. }) => t.take_derived(),
            _ => vec![],
        };
        // The format may have adjusted the value (e.g. rounded to a 32-bit float).
        let v = self.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(&ptr)).cloned().unwrap_or(v);
        if before == v && derived.is_empty() {
            return;
        }
        let merge = matches!(&self.last_edit, Some((p, t)) if *p == ptr && now - t < 1.5);
        match (merge, self.undo.last_mut()) {
            (true, Some(Change::Value { ptr: p, before: b, after, extra })) if *p == ptr => {
                *after = v;
                for (dp, db, da) in &derived {
                    match extra.iter_mut().find(|(ep, _, _)| ep == dp) {
                        Some(e) => e.2 = da.clone(),
                        None => extra.push((dp.clone(), db.clone(), da.clone())),
                    }
                }
                extra.retain(|(_, b, a)| b != a);
                // Typed back to where it started: drop the no-op undo step.
                if b == after && extra.is_empty() {
                    self.undo.pop();
                }
            }
            _ => self.undo.push(Change::Value { ptr: ptr.clone(), before, after: v, extra: derived.clone() }),
        }
        self.redo.clear();
        for (dp, _, _) in &derived {
            self.after_change(dp);
        }
        self.after_change(&ptr);
        self.last_edit = Some((ptr, now));
        if !derived.is_empty() {
            let names: Vec<String> = derived.iter().map(|(p, _, _)| p.rsplit('/').next().unwrap_or("").replace("~1", "/").replace("~0", "~")).collect();
            self.info(format!("Also updated: {}.", names.join(", ")));
        } else if aliases > 0 {
            self.info(format!(
                "This value is shared with {aliases} other place{} in the save, which changed too.",
                if aliases == 1 { "" } else { "s" }
            ));
        }
    }

    fn apply_bytes(&mut self, off: usize, bytes: Vec<u8>) {
        let Some(Doc { content: Content::Binary(data), .. }) = &mut self.doc else { return };
        let end = off + bytes.len();
        if end > data.len() {
            return;
        }
        let before = data[off..end].to_vec();
        data[off..end].copy_from_slice(&bytes);
        self.undo.push(Change::Bytes { off, before, after: bytes });
        self.redo.clear();
        self.last_edit = None;
        self.after_change("");
    }

    fn after_change(&mut self, ptr: &str) {
        if !self.touched.iter().any(|t| t == ptr) {
            self.touched.push(ptr.to_string());
        }
        if ptr.is_empty() && root(&self.doc.as_ref().unwrap().content).is_some() {
            self.rebuild_model();
        }
        self.hex.label_edits.clear();
        self.sync_raw();
        self.edit_gen += 1;
        self.recompute_dirty();
    }

    /// Keep the Raw JSON text in step with the data, so undo/revert aren't
    /// silently re-applied from stale text on the next save or tab switch.
    fn sync_raw(&mut self) {
        if self.tab == Tab::Raw {
            if let Some(Doc { content: Content::Json { value, .. }, .. }) = &self.doc {
                self.raw_text = serde_json::to_string_pretty(value).unwrap_or_default();
                self.raw_error = None;
            }
        }
    }

    fn undo_redo(&mut self, undo: bool) {
        let Some(change) = (if undo { self.undo.pop() } else { self.redo.pop() }) else { return };
        let ptr = match &change {
            Change::Value { ptr, before, after, extra } => {
                // Undo/redo only restores values that were valid before, so this can't fail.
                let _ = self.set_value(ptr, if undo { before.clone() } else { after.clone() });
                if let Some(Doc { content: Content::Tree(t), .. }) = &mut self.doc {
                    t.take_derived();
                    for (p, b, a) in extra.iter() {
                        t.set_plain(p, if undo { b.clone() } else { a.clone() });
                    }
                }
                let extra_ptrs: Vec<String> = extra.iter().map(|(p, _, _)| p.clone()).collect();
                for p in &extra_ptrs {
                    self.after_change(p);
                }
                ptr.clone()
            }
            Change::Bytes { off, before, after } => {
                if let Some(Doc { content: Content::Binary(data), .. }) = &mut self.doc {
                    let src = if undo { before } else { after };
                    data[*off..*off + src.len()].copy_from_slice(src);
                }
                String::new()
            }
        };
        if undo { self.redo.push(change) } else { self.undo.push(change) }
        self.last_edit = None;
        self.after_change(&ptr);
        self.info(if undo { "Undone." } else { "Redone." });
    }

    fn recompute_dirty(&mut self) {
        let Some(d) = &self.doc else {
            self.dirty = false;
            return;
        };
        self.dirty = match (&d.content, &d.baseline) {
            (Content::Binary(a), Content::Binary(b)) => a != b,
            (Content::Text(a), Content::Text(b)) => a != b,
            (c, b) => match (root(c), root(b)) {
                (Some(cv), Some(bv)) => self.touched.iter().any(|p| cv.pointer(p) != bv.pointer(p)),
                _ => true,
            },
        };
    }

    fn revert_all(&mut self) {
        let Some(d) = &mut self.doc else { return };
        d.content = d.baseline.clone();
        self.undo.clear();
        self.redo.clear();
        self.touched.clear();
        self.hex.label_edits.clear();
        self.sync_raw();
        self.rebuild_model();
        self.edit_gen += 1;
        self.recompute_dirty();
        self.ok("All changes reverted.");
    }

    // ------------------------------------------------------------ hex labels (sidecar)

    fn labels_path(&self) -> Option<PathBuf> {
        let mut p = self.doc.as_ref()?.path.as_os_str().to_owned();
        p.push(".labels.json");
        Some(PathBuf::from(p))
    }

    fn load_labels(&mut self) {
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

    fn save_labels(&mut self) {
        let Some(path) = self.labels_path() else { return };
        let list: Vec<Value> =
            self.hex.labels.iter().map(|(o, l)| json!({"offset": o, "name": l.name, "type": l.ty.name()})).collect();
        let result = if list.is_empty() {
            if crate::vfs::exists(&path) { crate::vfs::remove_file(&path) } else { Ok(()) }
        } else {
            crate::vfs::write(&path, serde_json::to_string_pretty(&json!({ "labels": list })).unwrap())
        };
        if let Err(e) = result {
            self.fail(format!("Couldn't save names to {}: {e}", path.display()));
        }
    }

    // ------------------------------------------------------------ saving

    fn commit_raw(&mut self) -> bool {
        if self.tab != Tab::Raw {
            return true;
        }
        let Some(Doc { content: Content::Json { value, .. }, .. }) = &self.doc else { return true };
        match serde_json::from_str::<Value>(&self.raw_text) {
            Ok(v) => {
                if *value != v {
                    self.apply_edit(String::new(), v, f64::NEG_INFINITY);
                }
                self.raw_error = None;
                true
            }
            Err(e) => {
                self.raw_error = Some(format!("Line {}, column {}: {e}", e.line(), e.column()));
                false
            }
        }
    }

    fn save_to(&mut self, path: &Path) {
        if !self.commit_raw() {
            return self.fail("The Raw JSON has a syntax error \u{2014} fix it before saving.");
        }
        let Some(doc) = &self.doc else { return };
        let bytes = match formats::encode(&doc.layers, &doc.content) {
            Ok(b) => b,
            Err(e) => return self.fail(format!("Couldn't build the save file: {e}")),
        };
        // Verify before touching the disk: the new file must decode to exactly what's on screen.
        let check = formats::decode_with(&bytes, &doc.opts);
        if !formats::same_layers(&check.layers, &doc.layers) || !formats::verified(&check.content, &doc.content) {
            return self.fail("Save aborted: the re-encoded file didn't read back identically. Your file was not changed.");
        }

        let mut note = String::new();
        if WEB {
            if let Err(e) = write_in_place(path, &bytes) {
                return self.fail(format!("Couldn't hand over {}: {e}", path.display()));
            }
            for (other, data) in formats::companion_files(path, &bytes) {
                let _ = write_in_place(&other, &data);
            }
            note = " Your browser downloaded it \u{2014} put it in place of the game's save (keep a copy of the old one).".into();
        } else {
        if path.exists() {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".bak");
            let bak = PathBuf::from(bak);
            if !bak.exists() {
                if let Err(e) = crate::vfs::copy(path, &bak) {
                    return self.fail(format!("Couldn't create backup {}: {e}", bak.display()));
                }
                note = format!(" Original backed up as {}.", file_name(&bak));
            }
            // Every earlier version stays restorable from File \u{203A} Restore previous version.
            if let Err(e) = crate::backups::take(path) {
                return self.fail(format!("Couldn't back up the current file: {e}. Your file was not changed."));
            }
        }
        if let Err(e) = write_in_place(path, &bytes) {
            return self.fail(format!("Couldn't write {}: {e}", path.display()));
        }
        // Files the game checks against this save (Persona 4 Golden's .binslot).
        for (other, data) in formats::companion_files(path, &bytes) {
            let _ = crate::backups::take(&other);
            if let Err(e) = crate::vfs::write(&other, data) {
                return self.fail(format!("Saved, but couldn't update {}: {e}", other.display()));
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        match crate::playerprefs::after_save(path) {
            Ok(Some(m)) => note.push_str(&format!(" {m}")),
            Err(e) => return self.fail(format!("Saved the file, but {e}")),
            Ok(None) => {}
        }
        }

        let signed = check.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. }));
        let persistent = check.layers.iter().any(|l| matches!(l, Layer::RenpyPersistent { .. }));
        let d = self.doc.as_mut().unwrap();
        d.path = path.to_path_buf();
        d.layers = check.layers;
        d.content = check.content;
        d.baseline = d.content.clone();
        self.touched.clear();
        self.rebuild_model();
        self.edit_gen += 1;
        self.recompute_dirty();
        self.settings.push_recent(path);
        let extra = if signed && crate::renpy_sign::can_sign() {
            " Re-signed with your Ren'Py key, so the game will load it normally."
        } else if signed {
            " Your Ren'Py key wasn't found, so the old signature was removed \u{2014} the game will ask once whether to load it; choose Yes."
        } else if persistent {
            " Re-signed with your Ren'Py key. If the game is running, close it without saving first \u{2014} it rewrites this file on exit."
        } else {
            ""
        };
        self.ok(format!("Saved and verified ({}).{note}{extra}", human_size(bytes.len())));
    }

    fn save(&mut self) {
        if let Some(p) = self.doc.as_ref().map(|d| d.path.clone()) {
            self.save_to(&p);
        }
    }

    /// Save from the button / Ctrl+S: first show what changed and warn if the game is open.
    fn save_clicked(&mut self) {
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        if !self.commit_raw() {
            return self.fail("The Raw JSON has a syntax error \u{2014} fix it before saving.");
        }
        let running = crate::running::game_for(&path);
        if running.is_none() && !self.settings.confirm_save {
            return self.save_to(&path);
        }
        self.running = running.clone();
        self.save_review = Some(SaveReview { path, running, lines: self.change_lines() });
    }

    /// The unsaved changes in words: "Inventory \u{203A} Scrap Iron: 12 \u{2192} 99".
    fn change_lines(&self) -> Vec<String> {
        let Some(d) = &self.doc else { return vec![] };
        match (&d.content, &d.baseline) {
            (Content::Binary(a), Content::Binary(b)) => byte_ranges(a, b)
                .into_iter()
                .map(|(s, e)| {
                    let name = self.hex.labels.get(&s).map(|l| l.name.clone()).unwrap_or(format!("Offset 0x{s:X}"));
                    format!("{name}: {} byte{} changed", e - s, if e - s == 1 { "" } else { "s" })
                })
                .collect(),
            (Content::Text(a), Content::Text(b)) => {
                if a != b {
                    vec!["The text was edited".into()]
                } else {
                    vec![]
                }
            }
            (c, b) => {
                let (Some(cv), Some(bv)) = (root(c), root(b)) else { return vec![] };
                self.touched
                    .iter()
                    .filter(|p| cv.pointer(p) != bv.pointer(p))
                    .map(|p| {
                        if p.is_empty() {
                            return "Whole document (Raw JSON edit)".to_string();
                        }
                        let (name, context) = match self.model.by_ptr.get(p.as_str()) {
                            Some(&i) => (self.model.rows[i].name.clone(), self.model.rows[i].context.clone()),
                            None => (p.clone(), String::new()),
                        };
                        let old = bv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                        let new = cv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                        let at = if context.is_empty() { name } else { format!("{context} \u{203A} {name}") };
                        format!("{at}: {old} \u{2192} {new}")
                    })
                    .collect()
            }
        }
    }

    fn game_picker_dialog(&mut self, ctx: &egui::Context) {
        let Some(gp) = &mut self.game_picker else { return };
        let found = gp.found.lock().map(|v| v.clone()).unwrap_or_default();
        if found.iter().any(Option::is_none) {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        let mut open = None;
        let mut close = false;
        let screen = ctx.screen_rect();
        let wide = screen.width() > 820.0;
        // A fixed size that always fits: the lists inside fill whatever they're given.
        let size = egui::vec2((screen.width() - 40.0).min(900.0), (screen.height() - 90.0).min(620.0));
        egui::Window::new("Select game")
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add(TextEdit::singleline(&mut gp.query).hint_text("game name").desired_width(220.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
                ui.weak("Pick a game to see where it keeps its saves and the ones on this computer. Open\u{2026} and dragging a file onto the window still work for anything else.");
                ui.separator();
                let q = gp.query.trim().to_lowercase();
                let shown: Vec<usize> = (0..crate::games::GAMES.len()).filter(|&i| q.is_empty() || crate::games::GAMES[i].name.to_lowercase().contains(&q)).collect();
                let list = |ui: &mut egui::Ui, picked: &mut Option<usize>| {
                    ScrollArea::vertical().id_salt("gp_list").auto_shrink([false, false]).show(ui, |ui| {
                        for &i in &shown {
                            let g = &crate::games::GAMES[i];
                            let n = match &found[i] {
                                Some(f) if f.is_empty() => "none found".to_string(),
                                Some(f) => format!("{} found", f.len()),
                                None => "looking\u{2026}".to_string(),
                            };
                            let r = ui.selectable_label(*picked == Some(i), format!("{}  \u{00B7}  {n}", g.name));
                            if r.clicked() {
                                *picked = Some(i);
                            }
                        }
                        if shown.is_empty() {
                            ui.weak("No game matches. Use Open\u{2026} to pick the file yourself.");
                        }
                    });
                };
                let details = |ui: &mut egui::Ui, picked: Option<usize>, group: &mut Option<String>, open: &mut Option<PathBuf>| {
                    let Some(i) = picked else {
                        ui.weak("Choose a game from the list.");
                        return;
                    };
                    let g = &crate::games::GAMES[i];
                    ui.heading(g.name);
                    for (k, v) in [("Usually in", g.folder), ("Files", g.files), ("Editing", g.support)] {
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(format!("{k}:"));
                            ui.add(egui::Label::new(v).wrap());
                        });
                    }
                    ui.add_space(6.0);
                    match &found[i] {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Looking for saves\u{2026}");
                            });
                        }
                        Some(f) if f.is_empty() => {
                            ui.colored_label(theme::fg(ui, theme::WARN), "No saves found on this computer.");
                            ui.weak("If the game keeps them somewhere else (another drive, a cloud folder), use Open\u{2026} to pick the file.");
                        }
                        Some(f) => {
                            let cards = |ui: &mut egui::Ui, saves: &[&crate::games::Found], open: &mut Option<PathBuf>| {
                                ScrollArea::vertical().id_salt("gp_saves").auto_shrink([false, false]).show(ui, |ui| {
                                    for s in saves {
                                        theme::card(ui, |ui| {
                                            if ui.link(RichText::new(s.label.clone().unwrap_or_else(|| file_name(&s.path))).strong()).clicked() {
                                                *open = Some(s.path.clone());
                                            }
                                            let when = s
                                                .modified
                                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                                .map(|d| crate::backups::describe(d.as_secs()))
                                                .unwrap_or_default();
                                            ui.weak(format!("{when}  \u{00B7}  {}", human_size(s.size as usize)));
                                            ui.add(egui::Label::new(RichText::new(s.path.parent().map(|p| p.display().to_string()).unwrap_or_default()).weak().size(12.0)).truncate());
                                        });
                                    }
                                });
                            };
                            if !crate::games::many_games(g) {
                                ui.label(format!("{} save{} found \u{2014} click one to open it:", f.len(), if f.len() == 1 { "" } else { "s" }));
                                let all: Vec<&crate::games::Found> = f.iter().collect();
                                cards(ui, &all, open);
                                return;
                            }
                            // One entry per game, most recently played first.
                            let mut groups: Vec<(String, Vec<&crate::games::Found>)> = vec![];
                            for s in f {
                                let name = crate::games::group_of(g, &s.path);
                                match groups.iter_mut().find(|(n, _)| *n == name) {
                                    Some((_, v)) => v.push(s),
                                    None => groups.push((name, vec![s])),
                                }
                            }
                            if let Some(name) = group.clone() {
                                if let Some((_, saves)) = groups.iter().find(|(n, _)| *n == name) {
                                    if ui.button(format!("\u{2190} All {} games", groups.len())).clicked() {
                                        *group = None;
                                    }
                                    ui.strong(&name);
                                    ui.label(format!("{} save{} \u{2014} click one to open it:", saves.len(), if saves.len() == 1 { "" } else { "s" }));
                                    cards(ui, saves, open);
                                    return;
                                }
                                *group = None;
                            }
                            ui.label(format!("{} game{} with saves ({} saves) \u{2014} pick one:", groups.len(), if groups.len() == 1 { "" } else { "s" }, f.len()));
                            ScrollArea::vertical().id_salt("gp_groups").auto_shrink([false, false]).show(ui, |ui| {
                                for (name, saves) in &groups {
                                    let when = saves[0]
                                        .modified
                                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                        .map(|d| crate::backups::describe(d.as_secs()))
                                        .unwrap_or_default();
                                    theme::card(ui, |ui| {
                                        if ui.link(RichText::new(name).strong()).clicked() {
                                            *group = Some(name.clone());
                                        }
                                        ui.weak(format!("{} save{}  \u{00B7}  last saved {when}", saves.len(), if saves.len() == 1 { "" } else { "s" }));
                                    });
                                }
                            });
                        }
                    }
                };
                let before = gp.picked;
                if wide {
                    ui.columns(2, |c| {
                        list(&mut c[0], &mut gp.picked);
                        details(&mut c[1], gp.picked, &mut gp.group, &mut open);
                    });
                } else if gp.picked.is_none() {
                    list(ui, &mut gp.picked);
                } else {
                    if ui.button("\u{2190} All games").clicked() {
                        gp.picked = None;
                    }
                    details(ui, gp.picked, &mut gp.group, &mut open);
                }
                if gp.picked != before {
                    gp.group = None;
                }
            });
        if close {
            self.game_picker = None;
        }
        if let Some(p) = open {
            self.game_picker = None;
            self.request_open(p);
        }
    }

    fn copy_slots_dialog(&mut self, ctx: &egui::Context) {
        let Some(c) = &mut self.copy_slots else { return };
        let label = |i: usize, s: &crate::fmt::slot_copy::Slot| {
            if s.active {
                format!("Slot {}: {} (level {})", i + 1, s.name, s.level)
            } else {
                format!("Slot {}: empty", i + 1)
            }
        };
        let mut choice = None;
        modal(ctx, "Copy character", |ui| {
            egui::Grid::new("copy_slots").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label("Copy");
                egui::ComboBox::from_id_salt("copy_from").width(260.0).selected_text(label(c.from, &c.slots[c.from])).show_ui(ui, |ui| {
                    for (i, s) in c.slots.iter().enumerate().filter(|(_, s)| s.active) {
                        ui.selectable_value(&mut c.from, i, label(i, s));
                    }
                });
                ui.end_row();
                ui.label("Into");
                egui::ComboBox::from_id_salt("copy_to").width(260.0).selected_text(label(c.to, &c.slots[c.to])).show_ui(ui, |ui| {
                    for (i, s) in c.slots.iter().enumerate() {
                        ui.selectable_value(&mut c.to, i, label(i, s));
                    }
                });
                ui.end_row();
            });
            let dest = &c.slots[c.to];
            if c.to == c.from {
                ui.colored_label(theme::fg(ui, theme::WARN), "Pick a different slot to copy into.");
            } else if dest.active {
                ui.colored_label(theme::fg(ui, theme::WARN), format!("This replaces {} (level {}) in slot {}.", dest.name, dest.level, c.to + 1));
            }
            ui.weak("The file is backed up first (File \u{203A} Restore previous version).");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = c.to != c.from && c.slots[c.from].active;
                if ui.add_enabled(ok, egui::Button::new(RichText::new("Copy").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let c = self.copy_slots.take().unwrap();
                self.copy_character(c.from, c.to, &c.slots[c.from]);
            }
            Some(false) => self.copy_slots = None,
            None => {}
        }
    }

    fn copy_character(&mut self, from: usize, to: usize, who: &crate::fmt::slot_copy::Slot) {
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        if let Some(game) = crate::running::game_for(&path) {
            return self.fail(format!("{game} looks like it's running \u{2014} quit it first, or it will overwrite the copy."));
        }
        let raw = match crate::vfs::read(&path) {
            Ok(r) => r,
            Err(e) => return self.fail(format!("Couldn't read {}: {e}", path.display())),
        };
        let out = match crate::fmt::slot_copy::er_copy(&raw, from, to) {
            Ok(o) => o,
            Err(e) => return self.fail(format!("Couldn't copy the character: {e}")),
        };
        if !WEB {
            if let Err(e) = crate::backups::take(&path) {
                return self.fail(format!("Couldn't back up the file first: {e}. Nothing was changed."));
            }
        }
        if let Err(e) = write_in_place(&path, &out) {
            return self.fail(format!("Couldn't write {}: {e}", path.display()));
        }
        self.load(&path);
        self.ok(format!("Copied {} (level {}) into slot {}. It shows up on the game's Load Game screen.", who.name, who.level, to + 1));
    }

    fn save_review_dialog(&mut self, ctx: &egui::Context) {
        let Some(r) = &self.save_review else { return };
        let mut choice = None;
        let mut dont_ask = !self.settings.confirm_save;
        modal(ctx, "Save changes?", |ui| {
            ui.set_max_width(520.0);
            if let Some(game) = &r.running {
                theme::card(ui, |ui| {
                    ui.label(RichText::new(format!("{game} looks like it's running.")).color(theme::fg(ui, theme::WARN)).strong());
                    ui.label(
                        "Games keep the save in memory and write it back when they autosave or quit, which would undo these \
                         changes. Quit the game completely (not just to the title screen), then save.",
                    );
                });
                ui.add_space(6.0);
            }
            ui.label(format!("{} change{} to {}:", r.lines.len(), if r.lines.len() == 1 { "" } else { "s" }, file_name(&r.path)));
            ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                for l in &r.lines {
                    ui.label(format!("\u{2022} {l}"));
                }
                if r.lines.is_empty() {
                    ui.weak("(nothing that the list can describe)");
                }
            });
            ui.add_space(4.0);
            ui.weak(if WEB {
                "Your browser downloads the edited file. Keep a copy of the original before you replace it."
            } else {
                "The current file is kept as a backup \u{2014} File \u{203A} Restore previous version brings it back."
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let label = if r.running.is_some() { "Save anyway" } else { "Save" };
                if ui.add(egui::Button::new(RichText::new(label).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(0);
                }
                if r.running.is_some() && ui.button("Check again").clicked() {
                    choice = Some(1);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(2);
                }
                ui.checkbox(&mut dont_ask, "Don't show this list again");
            });
        });
        if dont_ask == self.settings.confirm_save {
            self.settings.confirm_save = !dont_ask;
            self.settings.save();
        }
        match choice {
            Some(0) => {
                let p = self.save_review.take().unwrap().path;
                self.save_to(&p);
            }
            Some(1) => {
                let r = self.save_review.as_mut().unwrap();
                r.running = crate::running::game_for(&r.path);
                self.running = r.running.clone();
                if r.running.is_none() && !self.settings.confirm_save {
                    let p = self.save_review.take().unwrap().path;
                    self.save_to(&p);
                }
            }
            Some(_) => self.save_review = None,
            None => {}
        }
    }

    fn save_as(&mut self) {
        let Some(doc) = &self.doc else { return };
        let name = file_name(&doc.path);
        self.ask(Ask::SaveAs, None, Some(name));
    }

    /// Put `from` (a backup) back in place of the open file. The file as it is now is
    /// backed up first, so a restore can itself be undone.
    fn restore_from(&mut self, from: &Path, what: &str) {
        let Some(doc) = &self.doc else { return };
        let path = doc.path.clone();
        if let Some(game) = crate::running::game_for(&path) {
            return self.fail(format!("{game} looks like it's running \u{2014} quit it first, or it may overwrite the restored file."));
        }
        if let Err(e) = crate::backups::take(&path) {
            return self.fail(format!("Couldn't back up the current file first: {e}"));
        }
        match crate::vfs::copy(from, &path) {
            Ok(_) => {
                if let Ok(bytes) = crate::vfs::read(&path) {
                    for (other, data) in formats::companion_files(&path, &bytes) {
                        let _ = crate::vfs::write(&other, data);
                    }
                }
                self.dirty = false;
                self.load(&path);
                self.ok(format!("Restored {what}. The version it replaced is in the backup list too."));
            }
            Err(e) => self.fail(format!("Couldn't restore backup: {e}")),
        }
    }

    fn export_decoded(&mut self) {
        if !self.commit_raw() {
            return self.fail("The Raw JSON has a syntax error.");
        }
        let Some(doc) = &self.doc else { return };
        let ext = match &doc.content {
            Content::Json { .. } | Content::Tree(_) => "json",
            Content::Text(_) => "txt",
            Content::Binary(_) => "bin",
        };
        let stem = doc.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("save".into());
        self.ask(Ask::Export, None, Some(format!("{stem}.decoded.{ext}")));
    }

    fn import_from(&mut self, src: &Path) {
        if self.dirty {
            return self.fail("Save or revert your changes before importing.");
        }
        let Some(doc) = &self.doc else { return };
        let payload = match crate::vfs::read(src) {
            Ok(b) => b,
            Err(e) => return self.fail(format!("Couldn't read {}: {e}", src.display())),
        };
        let bytes = match formats::encode(&doc.layers, &Content::Binary(payload)) {
            Ok(b) => b,
            Err(e) => return self.fail(format!("Couldn't pack the imported data: {e}")),
        };
        let check = formats::decode_with(&bytes, &doc.opts);
        if !formats::same_layers(&check.layers, &doc.layers) || std::mem::discriminant(&check.content) != std::mem::discriminant(&doc.content) {
            return self.fail("The imported data doesn't pack back into this kind of save (wrong file, or edited into another format).");
        }
        let d = self.doc.as_mut().unwrap();
        d.content = check.content;
        self.rebuild_model();
        self.edit_gen += 1;
        self.recompute_dirty();
        self.ok(format!("Imported {}. Review the changes, then save.", file_name(src)));
    }

    fn export_to(&mut self, dest: &Path) {
        let Some(doc) = &self.doc else { return };
        let result = match &doc.content {
            Content::Json { value, .. } => serde_json::to_vec_pretty(value).map_err(|e| e.to_string()),
            Content::Tree(t) => serde_json::to_vec_pretty(&t.view).map_err(|e| e.to_string()),
            other => other.to_bytes(),
        };
        #[cfg(target_arch = "wasm32")]
        let result = result.and_then(|b| crate::web::download(&file_name(dest), &b));
        #[cfg(not(target_arch = "wasm32"))]
        let result = result.and_then(|b| crate::vfs::write(dest, b).map_err(|e| e.to_string()));
        match result {
            Ok(()) => self.ok(format!("Exported to {}", dest.display())),
            Err(e) => self.fail(format!("Export failed: {e}")),
        }
    }

    fn view_as_hex(&mut self) {
        if self.dirty {
            return self.fail("Save or revert your changes before switching to the hex view.");
        }
        let Some(doc) = &mut self.doc else { return };
        match doc.content.to_bytes() {
            Ok(b) => {
                doc.content = Content::Binary(b);
                doc.baseline = doc.content.clone();
                self.rebuild_model();
                self.ok("Showing the decoded data as hex. Saving re-applies the original encoding.");
            }
            Err(e) => self.fail(e),
        }
    }

    fn refine_from_disk(&mut self) {
        if self.dirty {
            return self.fail("Refine re-reads the file from disk \u{2014} save or revert your changes first.");
        }
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        let raw = match crate::vfs::read(&path) {
            Ok(r) => r,
            Err(e) => return self.fail(format!("Reload failed: {e}")),
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
        self.recompute_dirty();
        self.info(format!("Re-read the file: {before} \u{203A} {after} possible locations."));
    }

    // ------------------------------------------------------------ UI: chrome

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            let has = self.doc.is_some();
            ui.menu_button("File", |ui| {
                if ui.button("Open\u{2026}            Ctrl+O").clicked() {
                    ui.close_menu();
                    self.ask(Ask::Open, None, None);
                }
                if !cfg!(target_os = "android") && !WEB && ui.button("Select game\u{2026}").clicked() {
                    ui.close_menu();
                    self.game_picker = Some(GamePicker::new());
                }
                ui.menu_button("Open recent", |ui| {
                    let recent = self.settings.recent.clone();
                    if recent.is_empty() {
                        ui.weak("Nothing yet");
                    }
                    for p in recent {
                        if ui.add_enabled(crate::vfs::exists(&p), egui::Button::new(file_name(&p))).on_hover_text(p.display().to_string()).clicked() {
                            ui.close_menu();
                            self.request_open(p);
                        }
                    }
                });
                ui.separator();
                if ui.add_enabled(has, egui::Button::new("Save                Ctrl+S")).clicked() {
                    ui.close_menu();
                    self.save_clicked();
                }
                if ui.add_enabled(has, egui::Button::new("Save a copy as\u{2026}")).clicked() {
                    ui.close_menu();
                    self.save_as();
                }
                ui.separator();
                if ui.add_enabled(self.dirty, egui::Button::new("Revert all changes")).clicked() {
                    ui.close_menu();
                    self.revert_all();
                }
                let bak = self.doc.as_ref().map(|d| {
                    let mut b = d.path.as_os_str().to_owned();
                    b.push(".bak");
                    PathBuf::from(b)
                });
                let versions = self.doc.as_ref().map(|d| crate::backups::list(&d.path)).unwrap_or_default();
                let has_bak = bak.as_ref().is_some_and(|b| b.exists());
                let on_disk = !WEB && self.doc.as_ref().is_some_and(|d| d.path.is_file());
                ui.add_enabled_ui(has_bak || !versions.is_empty() || on_disk, |ui| {
                    ui.menu_button("Restore previous version", |ui| {
                        ui.weak("Taken each time this app saved over the file.");
                        ui.weak("Named copies are kept for good; the 30 newest others are kept.");
                        if !WEB && ui.button("Keep a named copy of the file now\u{2026}").clicked() {
                            ui.close_menu();
                            self.backup_name = Some(String::new());
                        }
                        ui.separator();
                        let mut pick = None;
                        let mut compare = None;
                        let mut unlock = None;
                        ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                            for (i, v) in versions.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    let when = crate::backups::describe(v.when);
                                    let label = match &v.name {
                                        Some(n) => format!("\u{201C}{n}\u{201D} \u{2014} {when} ({})", human_size(v.size as usize)),
                                        None => format!("{when} ({})", human_size(v.size as usize)),
                                    };
                                    if ui.button(label).on_hover_text("Put this version back").clicked() {
                                        pick = Some((v.path.clone(), format!("the version from {when}")));
                                    }
                                    if ui.small_button("Compare").on_hover_text("List what differs between this version and the open file").clicked() {
                                        compare = Some(v.path.clone());
                                    }
                                    if v.name.is_some() && ui.small_button("Unname").on_hover_text("Let this copy be cleared out in time like the others").clicked() {
                                        unlock = Some(i);
                                    }
                                });
                            }
                        });
                        if let Some(p) = compare {
                            ui.close_menu();
                            self.compare_with(p);
                        }
                        if let Some(i) = unlock {
                            if let Err(e) = crate::backups::unlock(&versions[i]) {
                                self.fail(format!("Couldn't change that copy: {e}"));
                            }
                        }
                        if has_bak {
                            ui.separator();
                            if ui.button("The original, from before the first save (.bak)").clicked() {
                                pick = bak.clone().map(|b| (b, "the original".to_string()));
                            }
                        }
                        if !WEB && !cfg!(target_os = "android") {
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Keep backups in another folder\u{2026}").clicked() {
                                    ui.close_menu();
                                    self.ask(Ask::BackupDir, self.settings.backup_dir.clone(), None);
                                }
                                if self.settings.backup_dir.is_some() && ui.small_button("Back to the default folder").clicked() {
                                    crate::backups::set_root(None);
                                    self.settings.backup_dir = None;
                                    self.settings.save();
                                }
                            });
                        }
                        if let Some(dir) = self.doc.as_ref().and_then(|d| crate::backups::folder(&d.path)) {
                            if dir.is_dir() {
                                if let Some(label) = platform::reveal_label() {
                                    ui.separator();
                                    if ui.button(label.replace("file", "backups")).clicked() {
                                        ui.close_menu();
                                        platform::reveal(&dir);
                                    }
                                }
                            }
                        }
                        if let Some((p, what)) = pick {
                            ui.close_menu();
                            self.restore_from(&p, &what);
                        }
                    });
                });
                ui.separator();
                if ui.add_enabled(has, egui::Button::new("Export decoded data\u{2026}")).clicked() {
                    ui.close_menu();
                    self.export_decoded();
                }
                let importable = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_) | Content::Text(_) | Content::Json { .. }));
                if ui
                    .add_enabled(importable, egui::Button::new("Import edited data\u{2026}"))
                    .on_hover_text("Bring back data you exported and edited in another program; it is packed the way this save is packed when you save.")
                    .clicked()
                {
                    ui.close_menu();
                    self.ask(Ask::Import, None, None);
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Undo        Ctrl+Z")).clicked() {
                    ui.close_menu();
                    self.undo_redo(true);
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Redo        Ctrl+Y")).clicked() {
                    ui.close_menu();
                    self.undo_redo(false);
                }
                ui.separator();
                if ui.add_enabled(has, egui::Button::new("Find\u{2026}         Ctrl+F")).clicked() {
                    ui.close_menu();
                    self.focus_search = true;
                }
            });
            ui.menu_button("View", |ui| {
                let mut changed = false;
                changed |= ui.checkbox(&mut self.settings.show_advanced, "Show advanced values").changed();
                changed |= ui.checkbox(&mut self.settings.show_raw_keys, "Show raw key names").changed();
                ui.checkbox(&mut self.show_changes, "Changes panel");
                ui.separator();
                if ui.radio(self.settings.dark, "Dark theme").clicked() && !self.settings.dark {
                    self.settings.dark = true;
                    theme::apply(ui.ctx(), true);
                    changed = true;
                }
                if ui.radio(!self.settings.dark, "Light theme").clicked() && self.settings.dark {
                    self.settings.dark = false;
                    theme::apply(ui.ctx(), false);
                    changed = true;
                }
                ui.separator();
                ui.weak("Zoom: Ctrl + / Ctrl \u{2212} / Ctrl 0");
                ui.separator();
                let structured = self.doc.as_ref().is_some_and(|d| !matches!(d.content, Content::Binary(_)));
                if ui.add_enabled(structured, egui::Button::new("Show decoded data as hex")).clicked() {
                    ui.close_menu();
                    self.view_as_hex();
                }
                if changed {
                    self.settings.save();
                }
            });
            ui.menu_button("Tools", |ui| {
                let is_rpg = matches!(&self.doc, Some(Doc { content: Content::Json { value, .. }, .. }) if names::is_rpgmaker(value));
                if !WEB && ui.add_enabled(is_rpg, egui::Button::new("Locate game data folder (RPG Maker names)\u{2026}")).clicked() {
                    ui.close_menu();
                    self.locate_data_dir();
                }
                let has = self.doc.is_some();
                if ui
                    .add_enabled(has, egui::Button::new("Decrypt with password\u{2026}"))
                    .on_hover_text("For encrypted saves (Easy Save 3, Godot). Easy Save's default password is tried automatically.")
                    .clicked()
                {
                    ui.close_menu();
                    self.password_prompt = Some(self.doc.as_ref().and_then(|d| d.opts.password.clone()).unwrap_or_default());
                }
                if ui
                    .add_enabled(has, egui::Button::new("Change Steam account in this save\u{2026}"))
                    .on_hover_text("Finds the Steam IDs stored in the save and changes them to another account's, so a save copied from a friend or another account loads.")
                    .clicked()
                {
                    ui.close_menu();
                    self.start_rebind();
                }
                let binary = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_)));
                if ui
                    .add_enabled(binary, egui::Button::new("Find checksum\u{2026}"))
                    .on_hover_text("Looks for a checksum in a save the editor doesn't know (CRC, Adler, sums, MD5, SHA) and keeps it right whenever you save.")
                    .clicked()
                {
                    ui.close_menu();
                    self.find_sums();
                }
                if ui
                    .add_enabled(has, egui::Button::new("Compare with another save\u{2026}"))
                    .on_hover_text("List every value that differs from another save \u{2014} handy for finding which value is your gold, HP, etc.")
                    .clicked()
                {
                    ui.close_menu();
                    self.start_compare();
                }
                let entries = self.doc.as_ref().map(|d| d.zip_entries.clone()).unwrap_or_default();
                if entries.len() > 1 {
                    ui.menu_button("Open another file in this archive", |ui| {
                        let current = self.doc.as_ref().and_then(|d| {
                            d.layers.iter().find_map(|l| match l {
                                Layer::ZipEntry { name, .. } | Layer::Copeck { name, .. } => Some(name.clone()),
                                Layer::Ps2Card { file, .. } => Some(file.name.clone()),
                                _ => None,
                            })
                        });
                        for e in entries {
                            if ui.selectable_label(current.as_deref() == Some(&e), &e).clicked() {
                                ui.close_menu();
                                self.reopen_with(|o| o.zip_entry = Some(e.clone()));
                            }
                        }
                    });
                }
                let archive = self.doc.as_ref().and_then(|d| {
                    d.layers.iter().find_map(|l| if let Layer::Bnd4 { archive, .. } = l { Some(archive.clone()) } else { None })
                });
                if let Some(slots) = archive.and_then(|a| crate::fmt::slot_copy::er_slots(&a)) {
                    if ui
                        .button("Copy character to another slot\u{2026}")
                        .on_hover_text("Duplicate an Elden Ring character, e.g. to try something out on the copy")
                        .clicked()
                    {
                        ui.close_menu();
                        if self.dirty {
                            self.fail("Save or revert your changes first \u{2014} copying works on the file as it is on disk.");
                        } else {
                            let to = slots.iter().position(|s| !s.active).unwrap_or(1);
                            self.copy_slots = Some(CopySlots { slots, from: 0, to });
                        }
                    }
                }
                ui.separator();
                if let (Some(d), Some(label)) = (&self.doc, platform::reveal_label()) {
                    if ui.button(label).clicked() {
                        ui.close_menu();
                        platform::reveal(&d.path);
                    }
                }
            });
        });
    }

    /// Re-read the open file with changed decode options (password, zip entry).
    fn reopen_with(&mut self, change: impl FnOnce(&mut formats::Options)) {
        if self.dirty {
            return self.fail("Save or revert your changes first.");
        }
        let Some(d) = &self.doc else { return };
        let path = d.path.clone();
        let mut opts = d.opts.clone();
        change(&mut opts);
        self.load_with(&path, opts);
    }

    fn start_compare(&mut self) {
        self.ask(Ask::Compare, None, None);
    }

    fn compare_with(&mut self, other: PathBuf) {
        let Some(d) = &self.doc else { return };
        let raw = match crate::vfs::read(&other) {
            Ok(r) => r,
            Err(e) => return self.fail(format!("Couldn't open {}: {e}", other.display())),
        };
        let mut opts = formats::Options::for_path(&other);
        opts.password = d.opts.password.clone();
        let theirs = formats::decode_with(&raw, &opts);
        let diffs = match (&d.content, &theirs.content) {
            (Content::Binary(a), Content::Binary(b)) => {
                let mut out: Vec<(String, Option<Value>, Option<Value>)> = byte_ranges(a, b)
                    .into_iter()
                    .take(5000)
                    .map(|(s, e)| {
                        let hex = |x: &[u8]| Value::String(x.iter().take(16).map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "));
                        (format!("0x{s:X}"), Some(hex(&a[s..e])), b.get(s..e.min(b.len())).map(hex))
                    })
                    .collect();
                if a.len() != b.len() {
                    out.push(("(file size)".into(), Some(Value::from(a.len())), Some(Value::from(b.len()))));
                }
                out
            }
            (c, t) => match (root(c), root(t)) {
                (Some(mine), Some(other_root)) => {
                    let (mut a, mut b) = (Vec::new(), Vec::new());
                    flatten(mine, "", &mut a);
                    flatten(other_root, "", &mut b);
                    let theirs: std::collections::HashMap<String, Value> = b.into_iter().collect();
                    let mut seen = HashSet::new();
                    let mut out = Vec::new();
                    for (p, v) in a {
                        seen.insert(p.clone());
                        match theirs.get(&p) {
                            Some(o) if *o == v => {}
                            o => out.push((p, Some(v), o.cloned())),
                        }
                    }
                    for (p, v) in theirs {
                        if !seen.contains(&p) {
                            out.push((p, None, Some(v)));
                        }
                    }
                    out
                }
                _ => return self.fail("Those two files aren't the same kind of save, so they can't be compared."),
            },
        };
        let n = diffs.len();
        self.compare = Some(Compare { other, diffs });
        self.info(format!("{n} difference{} found.", if n == 1 { "" } else { "s" }));
    }

    fn compare_window(&mut self, ctx: &egui::Context, now: f64) {
        let Some(cmp) = &self.compare else { return };
        let mut open = true;
        let mut jump: Option<String> = None;
        let mut take: Option<(String, Value)> = None;
        let other_name = file_name(&cmp.other);
        let binary = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_)));
        egui::Window::new(format!("Compare with {other_name}"))
            .open(&mut open)
            .default_size([620.0, 460.0])
            .show(ctx, |ui| {
                ui.weak("Values that differ between this save and the other one. \u{201C}Use theirs\u{201D} copies the other save's value into this one.");
                ui.separator();
                if cmp.diffs.is_empty() {
                    ui.label("No differences \u{2014} the saves hold the same values.");
                }
                ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 26.0, cmp.diffs.len(), |ui, range| {
                    for (p, original, theirs) in &cmp.diffs[range] {
                        ui.horizontal(|ui| {
                            let name = match self.model.by_ptr.get(p.as_str()) {
                                Some(&i) => {
                                    let r = &self.model.rows[i];
                                    let section = &self.model.sections[r.section].title;
                                    let mut parts: Vec<&str> = Vec::new();
                                    if section != "General" {
                                        parts.push(section);
                                    }
                                    if !r.context.is_empty() {
                                        parts.push(&r.context);
                                    }
                                    parts.push(&r.name);
                                    parts.join(" \u{203A} ")
                                }
                                None => p.clone(),
                            };
                            // Show this save's *current* value, so rows update after edits.
                            let current = self.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(p)).cloned();
                            let mine = if binary { original.clone() } else { current };
                            ui.allocate_ui_with_layout(egui::vec2(260.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.set_width(260.0);
                                let color = ui.visuals().hyperlink_color;
                                let link = ui.add(egui::Label::new(RichText::new(&name).color(color)).truncate().sense(egui::Sense::click()));
                                if link.on_hover_text(p).clicked() {
                                    jump = Some(p.clone());
                                }
                            });
                            let show = |v: &Option<Value>| v.as_ref().map(widgets::display).unwrap_or_else(|| "(missing)".into());
                            if mine == *theirs {
                                ui.label(RichText::new(format!("{}  \u{2014} now the same", show(&mine))).weak());
                                return;
                            }
                            ui.label(RichText::new(show(&mine)).color(theme::fg(ui, theme::MODIFIED)));
                            ui.label("\u{203A}");
                            ui.label(show(theirs));
                            let editable = self.model.by_ptr.get(p.as_str()).is_some_and(|&i| self.model.rows[i].editable);
                            let scalar = theirs.as_ref().is_some_and(|t| t.is_number() || t.is_string() || t.is_boolean());
                            if let (true, false, true, Some(t), Some(_)) = (editable, binary, scalar, theirs, &mine) {
                                if ui.small_button("Use theirs").clicked() {
                                    take = Some((p.clone(), t.clone()));
                                }
                            }
                        });
                    }
                });
            });
        if !open {
            self.compare = None;
        }
        if let Some((p, v)) = take {
            self.apply_edit(p, v, now);
        }
        if let Some(p) = jump {
            if let Some(&i) = self.model.by_ptr.get(p.as_str()) {
                self.tab = Tab::Values;
                self.search.clear();
                let sec = self.model.rows[i].section;
                if self.collapsed.remove(&sec) {
                    self.collapsed_gen += 1;
                }
                if self.model.sections[sec].advanced && !self.settings.show_advanced {
                    self.settings.show_advanced = true;
                }
                self.scroll_to_row = Some(i);
            } else if let Some(off) = p.strip_prefix("0x").and_then(|h| usize::from_str_radix(h, 16).ok()) {
                self.hex.jump(off);
            }
        }
    }

    fn backup_name_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.backup_name.take() else { return };
        let mut choice = None;
        modal(ctx, "Keep a named copy", |ui| {
            ui.label("A copy of the file as it is on disk now, kept until you remove it.");
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text("for example: before the final boss").desired_width(280.0));
            r.request_focus();
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Keep").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
                match crate::backups::take_named(&path, &text) {
                    Ok(_) => self.ok(format!("Kept a copy named \u{201C}{}\u{201D} (File \u{203A} Restore previous version).", text.trim())),
                    Err(e) => {
                        self.fail(format!("Couldn't keep a copy: {e}"));
                        self.backup_name = Some(text);
                    }
                }
            }
            Some(false) => {}
            None => self.backup_name = Some(text),
        }
    }

    fn ids_in_save(&self) -> Vec<u64> {
        match self.doc.as_ref().map(|d| &d.content) {
            Some(Content::Binary(b)) => crate::steamid::in_bytes(b).into_iter().map(|(id, _)| id).collect(),
            Some(c) => root(c).map(|v| crate::steamid::in_values(v).into_iter().map(|(id, _)| id).collect()).unwrap_or_default(),
            None => vec![],
        }
    }

    fn start_rebind(&mut self) {
        let ids = self.ids_in_save();
        if ids.is_empty() {
            return self.fail("No Steam ID was found in this save's data (some games key the save to the account in other ways).");
        }
        self.rebind = Some((ids, 0, String::new()));
    }

    fn rebind_dialog(&mut self, ctx: &egui::Context) {
        let Some((ids, mut pick, mut text)) = self.rebind.take() else { return };
        #[cfg(not(target_arch = "wasm32"))]
        let locals = crate::steamid::local_accounts();
        #[cfg(target_arch = "wasm32")]
        let locals: Vec<u64> = vec![];
        let mut choice = None;
        modal(ctx, "Change Steam account in this save", |ui| {
            ui.label("Steam IDs in this save:");
            for (i, id) in ids.iter().enumerate() {
                ui.radio_value(&mut pick, i, format!("{id} (account {})", id - crate::steamid::BASE));
            }
            ui.add_space(6.0);
            ui.label("Change it to:");
            ui.add(TextEdit::singleline(&mut text).hint_text("17-digit Steam ID or account number").desired_width(280.0));
            if !locals.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.weak("Accounts on this computer:");
                    for id in &locals {
                        if ui.small_button((id - crate::steamid::BASE).to_string()).clicked() {
                            text = id.to_string();
                        }
                    }
                });
            }
            ui.weak("Every copy of the ID changes; save to write the file. Some games also check the folder the save sits in.");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ok = crate::steamid::parse(&text).is_some();
                if ui.add_enabled(ok, egui::Button::new(RichText::new("Change").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let (Some(new), Some(&old)) = (crate::steamid::parse(&text), ids.get(pick)) else { return };
                let n = self.rebind_to(old, new);
                self.ok(format!("Changed the Steam ID in {n} place{}. Save to write the file.", if n == 1 { "" } else { "s" }));
            }
            Some(false) => {}
            None => self.rebind = Some((ids, pick, text)),
        }
    }

    /// Changes every copy of `old` to `new`; returns how many places changed.
    fn rebind_to(&mut self, old: u64, new: u64) -> usize {
        let Some(d) = &self.doc else { return 0 };
        match &d.content {
            Content::Binary(b) => {
                let at = crate::steamid::in_bytes(b).into_iter().find(|(id, _)| *id == old).map(|(_, a)| a).unwrap_or_default();
                for &o in &at {
                    self.apply_bytes(o, new.to_le_bytes().to_vec());
                }
                at.len()
            }
            c => {
                let Some(v) = root(c) else { return 0 };
                let ptrs = crate::steamid::in_values(v).into_iter().find(|(id, _)| *id == old).map(|(_, p)| p).unwrap_or_default();
                let edits: Vec<(String, Value)> = ptrs.iter().filter_map(|p| v.pointer(p).map(|x| (p.clone(), crate::steamid::replaced(x, new)))).collect();
                for (p, val) in &edits {
                    self.apply_edit(p.clone(), val.clone(), 0.0);
                    self.last_edit = None;
                }
                edits.len()
            }
        }
    }

    fn find_sums(&mut self) {
        let Some(d) = &self.doc else { return };
        let Content::Binary(b) = &d.content else { return };
        // Earlier copies of the file weed out values that only match by chance.
        #[cfg(not(target_arch = "wasm32"))]
        let others: Vec<Vec<u8>> = crate::backups::list(&d.path)
            .into_iter()
            .filter(|v| v.size as usize == b.len())
            .take(8)
            .filter_map(|v| std::fs::read(&v.path).ok())
            .collect();
        #[cfg(target_arch = "wasm32")]
        let others: Vec<Vec<u8>> = vec![];
        self.sum_hits = Some(crate::fmt::sumfind::find(b, &others));
    }

    fn sums_dialog(&mut self, ctx: &egui::Context) {
        let Some(hits) = self.sum_hits.clone() else { return };
        let current = self.doc.as_ref().and_then(|d| d.opts.sum);
        let mut close = false;
        let mut pick: Option<Option<crate::fmt::sumfind::Sum>> = None;
        modal(ctx, "Find checksum", |ui| {
            if hits.is_empty() {
                ui.label("No checksum the editor knows was found at the start or end of this file.");
                ui.weak("It may be encrypted, compressed, or checked with something the game made up. The value search still works on it.");
            } else {
                ui.label(if hits.len() == 1 { "This checksum holds for the file:" } else { "These checksums hold for the file:" });
                ui.weak("Pick one to have it written again every time you save. Saving a few times and searching again narrows a long list.");
                ui.add_space(6.0);
                for h in &hits {
                    ui.horizontal(|ui| {
                        ui.label(h.describe());
                        if current == Some(*h) {
                            ui.weak("(kept fixed)");
                        } else if ui.button("Keep fixed").clicked() {
                            pick = Some(Some(*h));
                        }
                    });
                }
            }
            if current.is_some() && ui.button("Stop fixing the checksum").clicked() {
                pick = Some(None);
            }
            ui.add_space(6.0);
            if ui.button("Close").clicked() {
                close = true;
            }
        });
        if let Some(p) = pick {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(d) = &self.doc {
                let _ = crate::fmt::sumfind::remember(&d.path, p);
            }
            self.sum_hits = None;
            self.reopen_with(|o| o.sum = p);
            if p.is_some() {
                self.ok("The checksum will be written again every time you save this file.");
            }
        } else if close {
            self.sum_hits = None;
        }
    }

    fn password_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.password_prompt.take() else { return };
        let mut choice = None;
        modal(ctx, "Decrypt with password", |ui| {
            ui.label("Some games encrypt their saves with a password built into the game.");
            ui.weak("Supported: Easy Save 3 (Unity) and Godot encrypted files. Easy Save's default password is tried automatically.");
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text("password").desired_width(280.0));
            r.request_focus();
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("Decrypt").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let pw = text.clone();
                self.reopen_with(|o| o.password = (!pw.is_empty()).then_some(pw.clone()));
                if matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_))) {
                    self.fail("That password didn't decrypt the file (or the encryption isn't one this editor knows).");
                }
            }
            Some(false) => {}
            None => self.password_prompt = Some(text),
        }
    }

    fn chip(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
        // A frame doesn't wrap on its own: start a new line when the chip won't fit.
        let w = ui.fonts(|f| f.layout_no_wrap(text.to_string(), egui::FontId::proportional(13.0), color).size().x) + 16.0;
        if ui.layout().main_wrap && w > ui.available_size_before_wrap().x {
            ui.end_row();
        }
        egui::Frame::none()
            .fill(color.gamma_multiply(0.18))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(8.0, 2.0))
            .show(ui, |ui| ui.add(egui::Label::new(RichText::new(text).color(theme::fg(ui, color)).size(13.0)).extend()))
            .response
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let hi = self.has_icons;
        ui.add_space(4.0);
        // The toolbar wraps well before the rest of the layout turns phone-sized: the file
        // name and its chips need the room.
        let narrow = narrow(ui) || ui.available_width() < 900.0;
        let row = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
            if narrow {
                ui.horizontal_wrapped(|ui| add(ui));
            } else {
                ui.horizontal(|ui| add(ui));
            }
        };
        row(ui, &mut |ui| {
            if ui.button(widgets::icon_text(ui, hi, icon::OPEN, "Open")).on_hover_text("Open a save file (Ctrl+O)").clicked() {
                self.ask(Ask::Open, None, None);
            }
            if !cfg!(target_os = "android")
                && !WEB
                && ui.button("Select game").on_hover_text("Pick a game from the list to see the saves it has on this computer").clicked()
            {
                self.game_picker = Some(GamePicker::new());
            }
            let save_btn = if self.dirty {
                egui::Button::new(widgets::icon_text_colored(ui, hi, icon::SAVE, "Save", Color32::WHITE)).fill(theme::ACCENT)
            } else {
                egui::Button::new(widgets::icon_text(ui, hi, icon::SAVE, "Save"))
            };
            if ui.add_enabled(self.dirty, save_btn).on_hover_text("Save changes to the file (Ctrl+S)").clicked() {
                self.save_clicked();
            }
            if !narrow {
                ui.separator();
            }
            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new(widgets::icon_text(ui, hi, icon::UNDO, "Undo"))).on_hover_text("Ctrl+Z").clicked() {
                self.undo_redo(true);
            }
            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new(widgets::icon_text(ui, hi, icon::REDO, "Redo"))).on_hover_text("Ctrl+Y").clicked() {
                self.undo_redo(false);
            }

            let Some(doc) = &self.doc else { return };
            if !narrow {
                ui.separator();
            }
            ui.label(RichText::new(file_name(&doc.path)).family(theme::semibold_family()).size(16.0))
                .on_hover_text(doc.path.display().to_string());
            let kind = friendly_kind(&doc.layers, &doc.content);
            Self::chip(ui, &kind, theme::ACCENT).on_hover_text(format!("Detected format: {}", formats::describe(&doc.layers, &doc.content)));
            if doc.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. } | Layer::RenpyPersistent { signed: true })) {
                if crate::renpy_sign::can_sign() {
                    Self::chip(ui, "Signed \u{2014} will re-sign", theme::OK).on_hover_text(
                        "Ren'Py 8.1+ signs its saves. Your Ren'Py key was found on this PC, so edited saves are re-signed and load normally.",
                    );
                } else {
                    Self::chip(ui, "Signed save", theme::WARN).on_hover_text(
                        "Ren'Py 8.1+ signs its saves, but your Ren'Py key (%APPDATA%\\RenPy\\tokens) wasn't found. \
                         The old signature will be removed; the game will ask once whether to load the save \u{2014} choose Yes.",
                    );
                }
            }
            if let Some(game) = &self.running {
                Self::chip(ui, &format!("{game} is running"), theme::WARN).on_hover_text(
                    "The game was running when this file was opened. Quit it before saving, or it may overwrite your changes.",
                );
            }
            let rpg = matches!(&doc.content, Content::Json { value, .. } if names::is_rpgmaker(value));
            let mut locate = false;
            if rpg {
                match &doc.names.source {
                    Some(dir) => {
                        Self::chip(ui, "Names from game data", theme::OK).on_hover_text(dir.display().to_string());
                    }
                    None => {
                        if Self::chip(ui, "Game data not found \u{2014} click to locate", theme::WARN)
                            .interact(egui::Sense::click())
                            .on_hover_text("Point to the game's data folder to see variable, switch and item names")
                            .clicked()
                        {
                            locate = true;
                        }
                    }
                }
            }

            let mut changes = |ui: &mut egui::Ui| {
                let n = self.change_count();
                let label = if n > 0 { format!("Changes ({n})") } else { "Changes".into() };
                let color = if self.show_changes { Color32::WHITE } else { ui.visuals().text_color() };
                ui.toggle_value(&mut self.show_changes, widgets::icon_text_colored(ui, hi, icon::HISTORY, &label, color))
                    .on_hover_text("List everything you've changed since the last save");
            };
            if narrow {
                changes(ui);
            } else {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), changes);
            }
            if locate {
                self.locate_data_dir();
            }
        });
        ui.add_space(2.0);
    }

    fn change_count(&self) -> usize {
        let Some(d) = &self.doc else { return 0 };
        match (&d.content, &d.baseline) {
            (Content::Binary(a), Content::Binary(b)) => byte_ranges(a, b).len(),
            (Content::Text(a), Content::Text(b)) => usize::from(a != b),
            (c, b) => match (root(c), root(b)) {
                (Some(cv), Some(bv)) => self.touched.iter().filter(|p| cv.pointer(p) != bv.pointer(p)).count(),
                _ => 0,
            },
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        // The change count goes on the right first, so the message is cut short before it
        // rather than running underneath it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // On a phone the highlighted Save button says the same thing.
            if self.doc.is_some() && !narrow(ui) {
                let n = self.change_count();
                if self.dirty {
                    ui.label(RichText::new(format!("{n} unsaved change{}", if n == 1 { "" } else { "s" })).color(theme::MODIFIED));
                } else {
                    ui.weak("No unsaved changes");
                }
                ui.add_space(12.0);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let (color, glyph) = match self.status_kind {
                    StatusKind::Info => (ui.visuals().weak_text_color(), ""),
                    StatusKind::Ok => (theme::fg(ui, theme::OK), icon::CHECK),
                    StatusKind::Err => (theme::fg(ui, theme::ERR), icon::WARNING),
                };
                if self.has_icons && !glyph.is_empty() {
                    ui.label(RichText::new(glyph).family(theme::icons_family()).color(color).size(12.0));
                }
                ui.add(egui::Label::new(RichText::new(&self.status).color(color)).truncate()).on_hover_text(&self.status);
            });
        });
    }

    // ------------------------------------------------------------ UI: structured data

    fn structured_panel(&mut self, ui: &mut egui::Ui, now: f64) {
        let is_json = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Json { .. }));
        let game = self.doc.as_ref().and_then(|d| d.game);
        if (self.tab == Tab::Raw && !is_json) || (self.tab == Tab::Game && game.is_none()) {
            self.tab = Tab::Values;
        }
        let hi = self.has_icons;

        // View bar: tabs, search, options.
        ui.horizontal(|ui| {
            let old = self.tab;
            if let Some(g) = game {
                ui.selectable_value(&mut self.tab, Tab::Game, g.tab_label()).on_hover_text("This game's own screen: its stats, inventory and more, laid out like the game");
            }
            ui.selectable_value(&mut self.tab, Tab::Values, "Values").on_hover_text("Every editable value in a simple list");
            ui.selectable_value(&mut self.tab, Tab::Tree, "Tree").on_hover_text("The full structure of the save, as nested groups");
            if is_json {
                ui.selectable_value(&mut self.tab, Tab::Raw, "Raw JSON").on_hover_text("Edit the JSON text directly");
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
                if ui.button("Apply").clicked() && self.commit_raw() {
                    self.ok("Raw JSON applied.");
                }
                if let Some(e) = &self.raw_error {
                    ui.colored_label(theme::ERR, e);
                }
                return;
            }
            if self.tab == Tab::Values {
                if hi {
                    ui.label(RichText::new(icon::SEARCH).family(theme::icons_family()).weak());
                }
                let id = egui::Id::new("search_box");
                let w = if narrow(ui) { ui.available_width() - 60.0 } else { (ui.available_width() - 300.0).clamp(140.0, 320.0) };
                let r = ui.add(
                    TextEdit::singleline(&mut self.search)
                        .id(id)
                        .hint_text("Search names or values  (Ctrl+F)")
                        .desired_width(w),
                );
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
                if !self.search.is_empty() && ui.small_button("Clear").clicked() {
                    self.search.clear();
                }
            }
            if !narrow(ui) && self.tab != Tab::Game {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| self.view_options(ui));
            }
        });
        if narrow(ui) && matches!(self.tab, Tab::Values | Tab::Tree) {
            ui.horizontal(|ui| self.view_options(ui));
        }
        ui.add_space(4.0);
        self.structured_body(ui, now);
    }

    /// The Raw keys / Advanced toggles.
    fn view_options(&mut self, ui: &mut egui::Ui) {
        let mut changed = widgets::labeled_toggle(
            ui,
            &mut self.settings.show_raw_keys,
            "Raw keys",
            "Show the game's own key name next to each friendly name",
        );
        if self.tab != Tab::Values {
            if changed {
                self.settings.save();
            }
            return;
        }
        ui.add_space(8.0);
        let hidden = self.model.sections.iter().filter(|s| s.advanced).count();
        let tip = if hidden > 0 && !self.settings.show_advanced {
            format!("Also show engine internals and read-only values ({hidden} section{} hidden now)", if hidden == 1 { "" } else { "s" })
        } else {
            "Also show engine internals and read-only values".to_string()
        };
        let label = if hidden > 0 && !self.settings.show_advanced { format!("Advanced ({hidden})") } else { "Advanced".into() };
        changed |= widgets::labeled_toggle(ui, &mut self.settings.show_advanced, &label, &tip);
        if changed {
            self.settings.save();
        }
    }

    fn structured_body(&mut self, ui: &mut egui::Ui, now: f64) {
        let hi = self.has_icons;
        let mut edits: Vec<(String, Value)> = Vec::new();
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
                // Laid out to the window's width (at most 1200); a table or banner that is wider
                // still (a phone) scrolls sideways instead of being cut off.
                let w = ui.available_width();
                ScrollArea::both().auto_shrink([false, false]).max_width(w).show(ui, |ui| {
                    ui.set_max_width(w.min(1200.0));
                    crate::game_ui::show(game, ui, &mut cx);
                    ui.add_space(12.0);
                    ui.weak("Everything else in this save is in the Values and Tree tabs.");
                });
                edits = cx.edits;
                if let Some(e) = cx.open_entry {
                    self.reopen_with(|o| o.zip_entry = Some(e));
                    return;
                }
            }
            Tab::Raw => {
                ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
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
                let mut ctx = TreeCtx { names: &d.names, editable, baseline: b, show_raw_keys: self.settings.show_raw_keys, value_x, edits: vec![] };
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| json_view::tree_ui(ui, r, &mut ctx));
                edits = ctx.edits;
            }
            Tab::Values => {
                let d = self.doc.as_ref().unwrap();
                let (Some(r), Some(b)) = (root(&d.content), root(&d.baseline)) else { return };
                let value_gen = if self.search.trim().is_empty() { 0 } else { self.edit_gen };
                let key = (self.search.clone(), self.settings.show_advanced, self.collapsed_gen, self.model_gen, value_gen);
                // On very large saves, searching every value takes a moment: wait until
                // typing pauses instead of re-searching on every key press.
                let t = ui.input(|i| i.time);
                let typing = self.model.rows.len() > 50_000 && t - self.search_typed_at < 0.3 && self.disp_key.is_some();
                if typing {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(120));
                }
                if self.disp_key.as_ref() != Some(&key) && !typing {
                    self.disp = values_view::filter(&self.model, r, &self.search, self.settings.show_advanced, &self.collapsed);
                    self.disp_key = Some(key);
                }
                if self.disp.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        if self.search.trim().is_empty() {
                            ui.weak("No editable values here. Try turning on Advanced, or use the Tree view.");
                        } else {
                            ui.label(format!("Nothing matches \u{201C}{}\u{201D}.", self.search.trim()));
                            ui.weak("Try part of a name (\u{201C}gold\u{201D}) or a current value (\u{201C}1500\u{201D}).");
                            if !self.settings.show_advanced {
                                ui.weak("Advanced values are hidden \u{2014} tick Advanced to search them too.");
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
                };
                values_view::show(ui, &self.model, &self.disp, &self.collapsed, &mut ctx);
                if let Some(sec) = ctx.toggle_section {
                    if !self.collapsed.remove(&sec) {
                        self.collapsed.insert(sec);
                    }
                    self.collapsed_gen += 1;
                }
                edits = ctx.edits;
            }
        }
        for (ptr, v) in edits {
            self.apply_edit(ptr, v, now);
        }
    }

    fn changes_panel(&mut self, ctx: &egui::Context, now: f64) {
        let mut revert: Vec<(String, Value)> = vec![];
        let mut revert_bytes: Vec<(usize, Vec<u8>)> = vec![];
        let mut jump = None;
        let mut revert_all = false;
        let mut close = false;
        egui::SidePanel::right("changes").default_width(320.0).min_width(260.0).show(ctx, |ui| {
            let Some(d) = &self.doc else { return };
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("Changes");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Close").clicked() {
                        close = true;
                    }
                });
            });
            ui.weak("Since you opened or last saved this file.");
            ui.add_space(4.0);
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match (&d.content, &d.baseline) {
                (Content::Binary(cur), Content::Binary(base)) => {
                    let ranges = byte_ranges(cur, base);
                    if ranges.is_empty() {
                        ui.weak("No changes yet.");
                    }
                    for (start, end) in ranges {
                        theme::card(ui, |ui| {
                            let name = self.hex.labels.get(&start).map(|l| l.name.clone()).unwrap_or(format!("Offset 0x{start:X}"));
                            if ui.link(RichText::new(name).strong()).clicked() {
                                self.hex.jump(start);
                            }
                            let hex = |b: &[u8]| b.iter().take(16).map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ");
                            ui.label(RichText::new(hex(&base[start..end])).weak().strikethrough().monospace());
                            ui.label(RichText::new(hex(&cur[start..end])).color(theme::MODIFIED).monospace());
                            if ui.small_button("Revert").clicked() {
                                revert_bytes.push((start, base[start..end].to_vec()));
                            }
                        });
                    }
                }
                (Content::Text(a), Content::Text(b)) => {
                    ui.weak(if a != b { "The text has been edited." } else { "No changes yet." });
                }
                (c, b) => {
                    let (Some(cv), Some(bv)) = (root(c), root(b)) else { return };
                    let changed: Vec<&String> = self.touched.iter().filter(|p| cv.pointer(p) != bv.pointer(p)).collect();
                    if changed.is_empty() {
                        ui.weak("No changes yet. Edited values are highlighted in orange.");
                    }
                    for ptr in changed {
                        theme::card(ui, |ui| {
                            if ptr.is_empty() {
                                ui.label(RichText::new("Whole document (Raw JSON edit)").strong());
                            } else {
                                let (name, context) = match self.model.by_ptr.get(ptr.as_str()) {
                                    Some(&i) => (self.model.rows[i].name.clone(), self.model.rows[i].context.clone()),
                                    None => (ptr.clone(), String::new()),
                                };
                                if ui.link(RichText::new(&name).strong()).on_hover_text("Show in list").clicked() {
                                    jump = self.model.by_ptr.get(ptr.as_str()).copied();
                                }
                                if !context.is_empty() {
                                    ui.weak(context);
                                }
                                let old = bv.pointer(ptr).map(widgets::display).unwrap_or_else(|| "(none)".into());
                                let new = cv.pointer(ptr).map(widgets::display).unwrap_or_else(|| "(none)".into());
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(old).weak().strikethrough());
                                    ui.label("\u{203A}");
                                    ui.label(RichText::new(new).color(theme::MODIFIED));
                                });
                            }
                            if ui.small_button("Revert").clicked() {
                                if let Some(orig) = bv.pointer(ptr) {
                                    revert.push((ptr.clone(), orig.clone()));
                                }
                            }
                        });
                    }
                }
            });
            if self.dirty {
                ui.separator();
                if ui.button("Revert all changes").clicked() {
                    revert_all = true;
                }
            }
        });
        for (p, v) in revert {
            self.apply_edit(p, v, f64::NEG_INFINITY);
        }
        for (o, b) in revert_bytes {
            self.apply_bytes(o, b);
        }
        if let Some(i) = jump {
            self.tab = Tab::Values;
            self.search.clear();
            let sec = self.model.rows[i].section;
            if self.collapsed.remove(&sec) {
                self.collapsed_gen += 1;
            }
            if self.model.sections[sec].advanced && !self.settings.show_advanced {
                self.settings.show_advanced = true;
            }
            self.scroll_to_row = Some(i);
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

    fn hex_side_panel(&mut self, ctx: &egui::Context) {
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
        // Beside the grid on a wide window; under it on a narrow one, so the bytes keep the width.
        let wide = ctx.screen_rect().width() >= 900.0;
        let body = |ui: &mut egui::Ui| {
            let Some(Doc { content: Content::Binary(data), .. }) = &self.doc else { return };
            let h = &mut self.hex;
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(6.0);
                if let Some((current, saves, others)) = &cairn {
                    theme::card(ui, |ui| {
                        ui.label(RichText::new("Cairn saves in this slot").family(theme::semibold_family()).size(16.0));
                        ui.weak(format!("{} saves, the dated ones newest first; pick one to open it.", saves.len()));
                        ScrollArea::vertical().id_salt("cairn_saves").max_height(220.0).show(ui, |ui| {
                            for (entry, label) in saves {
                                let shown = entry == current;
                                let text = if shown { RichText::new(format!("{label}  (open)")).strong() } else { RichText::new(label) };
                                if ui.selectable_label(shown, text).on_hover_text(entry).clicked() && !shown {
                                    open_entry = Some(entry.clone());
                                }
                            }
                        });
                        if let Some(at) = crate::fmt::cairn::stats_at(data) {
                            ui.add_space(6.0);
                            ui.label(RichText::new("This save's run").strong());
                            egui::Grid::new("cairn_stats").num_columns(3).spacing([10.0, 4.0]).show(ui, |ui| {
                                for (label, off, _) in crate::fmt::cairn::FIELDS {
                                    let o = at + off;
                                    let Some(b) = data.get(o..o + 4) else { continue };
                                    let mut v = u32::from_le_bytes(b.try_into().unwrap());
                                    ui.label(label);
                                    if ui.add(egui::DragValue::new(&mut v).range(0..=99_999_999)).changed() {
                                        byte_edit = Some((o, v.to_le_bytes().to_vec()));
                                        status = Some(Ok(format!("Set {label} to {v}")));
                                    }
                                    if off == 0 {
                                        ui.weak(format!("{}:{:02}:{:02}", v / 3600, v / 60 % 60, v % 60));
                                    } else {
                                        ui.label("");
                                    }
                                    ui.end_row();
                                }
                            });
                            ui.weak("As on the game's slot card. The card's own copy of the play time updates the next time the game saves.");
                        }
                        if *others > 0 {
                            ui.weak(format!(
                                "{others} other files (routes, tutorials, run info) are in the pack too: Tools \u{203A} Open another file in this archive."
                            ));
                        }
                    });
                    ui.add_space(6.0);
                }
                theme::card(ui, |ui| {
                    ui.label(RichText::new("Named values").family(theme::semibold_family()).size(16.0));
                    if h.labels.is_empty() {
                        ui.weak("Binary saves have no names. Find a value with Value search, click it, then name it here \u{2014} names are remembered for this file.");
                    }
                    let mut remove = None;
                    for (&off, l) in &h.labels {
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(egui::vec2(96.0, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.set_width(96.0);
                                let link = egui::Link::new(RichText::new(&l.name).strong());
                                if ui.add(link).on_hover_text(format!("Show in grid \u{2014} 0x{off:X} as {}", l.ty.name())).clicked() {
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
                            if ui.button("Set").on_hover_text("Write this value (or press Enter)").clicked() || enter {
                                match hex_view::encode_value(l.ty, &text, h.big_endian) {
                                    Ok(b) => {
                                        byte_edit = Some((off, b));
                                        status = Some(Ok(format!("Set {} to {text}", l.name)));
                                    }
                                    Err(e) => status = Some(Err(format!("Invalid value for {}: {e}", l.name))),
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let trash = widgets::icon(self.has_icons, "\u{E74D}", "\u{2715}").size(13.0).weak();
                                if ui.add(egui::Button::new(trash).frame(false)).on_hover_text("Forget this name (the value is not changed)").clicked() {
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
                        ui.add(TextEdit::singleline(&mut h.label_name).hint_text(format!("Name for 0x{:X}", h.selected)).desired_width(130.0));
                        type_combo(ui, "label_type", &mut h.edit_type);
                        if ui.add_enabled(!h.label_name.trim().is_empty(), egui::Button::new("Add")).clicked() {
                            h.labels.insert(h.selected, OffsetLabel { name: h.label_name.trim().to_string(), ty: h.edit_type });
                            h.label_name.clear();
                            labels_changed = true;
                        }
                    });
                });

                theme::card(ui, |ui| {
                    ui.label(RichText::new("Value search").family(theme::semibold_family()).size(16.0));
                    ui.weak("Know the value? Search for it, change it in-game, save, and refine with the new value. Don't know it (a health bar, a hidden counter)? Start with \u{201C}Unknown starting value\u{201D}, then refine with Increased, Decreased or Changed after each in-game save.");
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
                                let r2 = ui.add(TextEdit::singleline(&mut h.search_text2).hint_text("to").desired_width(110.0));
                                enter |= r2.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            }
                            if enter {
                                if scanning { refine = true } else { start = true }
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        if ui.button(if scanning { "Start over" } else { "Search" }).clicked() {
                            if scanning {
                                h.set_results(Vec::new());
                                h.searched = false;
                            } else {
                                start = true;
                            }
                        }
                        if ui
                            .add_enabled(scanning, egui::Button::new("Refine"))
                            .on_hover_text("Re-reads the file from disk (save in-game first) and keeps only the locations that pass the test above")
                            .clicked()
                        {
                            refine = true;
                        }
                        ui.checkbox(&mut h.aligned, "Aligned").on_hover_text("Only test offsets that are a multiple of the value size. Faster, and how most games store numbers.");
                    });
                    if start {
                        match h.start_scan(data) {
                            Ok(m) => status = Some(Ok(m)),
                            Err(e) => status = Some(Err(e)),
                        }
                    }
                    if h.all_locations {
                        ui.label("Every location is still in play.");
                    }
                    if h.searched {
                        ui.label(format!("{} result(s){}", h.results.len(), if h.results.len() >= hex_view::MAX_RESULTS { " (capped)" } else { "" }));
                        let mut jump = None;
                        ScrollArea::vertical().id_salt("results").max_height(220.0).show(ui, |ui| {
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
                        ui.label(RichText::new(format!("Selected: 0x{:X}", h.selected)).family(theme::semibold_family()).size(16.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut h.big_endian, "Big-endian").on_hover_text("Most PC games use little-endian (unticked)");
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Go to");
                        let r = ui.add(TextEdit::singleline(&mut h.goto_text).hint_text("hex offset").desired_width(100.0));
                        if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.small_button("Go").clicked() {
                            match usize::from_str_radix(h.goto_text.trim().trim_start_matches("0x"), 16) {
                                Ok(o) if o < data.len() => h.jump(o),
                                _ => status = Some(Err("That offset is outside the file".into())),
                            }
                        }
                    });
                    egui::Grid::new("inspector").striped(true).num_columns(2).show(ui, |ui| {
                        for ty in NumType::ALL.into_iter().filter(|t| !matches!(t, NumType::Ascii | NumType::Utf16)) {
                            ui.monospace(ty.name());
                            let v = hex_view::read_value(ty, data, h.selected, h.big_endian).unwrap_or_else(|| "\u{2014}".into());
                            if ui
                                .add(egui::Label::new(RichText::new(&v).monospace()).sense(egui::Sense::click()))
                                .on_hover_text("Click to edit as this type")
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
                        ui.add(TextEdit::singleline(&mut h.edit_text).hint_text("new value").desired_width(110.0));
                        if ui.button("Write").clicked() {
                            match hex_view::encode_value(h.edit_type, &h.edit_text, h.big_endian) {
                                Ok(b) if h.selected + b.len() <= data.len() => {
                                    status = Some(Ok(format!("Wrote {} byte(s) at 0x{:X}", b.len(), h.selected)));
                                    byte_edit = Some((h.selected, b));
                                }
                                Ok(_) => status = Some(Err("That value runs past the end of the file".into())),
                                Err(e) => status = Some(Err(format!("Invalid value: {e}"))),
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
        if refine {
            self.refine_from_disk();
        }
        if let Some(e) = open_entry {
            self.reopen_with(|o| o.zip_entry = Some(e));
        }
    }

    // ------------------------------------------------------------ UI: welcome

    fn welcome(&mut self, ui: &mut egui::Ui) {
        let mut open: Option<PathBuf> = None;
        let mut browse: Option<Option<PathBuf>> = None;
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.08).max(24.0));
                let logo = self.logo.get_or_insert_with(|| {
                    let img = egui::ColorImage::from_rgba_unmultiplied([128, 128], &crate::icon::rgba(128));
                    ui.ctx().load_texture("logo", img, Default::default())
                });
                ui.image((logo.id(), egui::vec2(88.0, 88.0)));
                ui.add_space(6.0);
                ui.label(RichText::new(platform::DISPLAY_NAME).family(theme::semibold_family()).size(26.0).color(ui.visuals().strong_text_color()));
                ui.label(if WEB {
                    "Edit game saves safely \u{2014} it runs in your browser and nothing is uploaded."
                } else if cfg!(target_os = "android") {
                    "Edit game saves safely \u{2014} everything stays on this device."
                } else {
                    "Edit game saves safely \u{2014} everything stays on this computer."
                });
                ui.add_space(14.0);
                let btn = egui::Button::new(widgets::icon_text_colored(ui, self.has_icons, icon::OPEN, "Open a save file", Color32::WHITE))
                    .fill(theme::ACCENT)
                    .min_size(egui::vec2(220.0, 40.0));
                if ui.add(btn).clicked() {
                    browse = Some(None);
                }
                ui.add_space(4.0);
                if !cfg!(target_os = "android") {
                    ui.weak("or drag a file anywhere onto this window");
                }
                ui.add_space(18.0);

                let w = ui.available_width().min(680.0);
                ui.allocate_ui(egui::vec2(w, 0.0), |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        theme::card(ui, |ui| {
                            ui.label(RichText::new("Recent files").family(theme::semibold_family()).size(16.0));
                            if self.settings.recent.is_empty() {
                                ui.weak("Files you open will appear here.");
                            }
                            for p in self.settings.recent.iter().take(8) {
                                let exists = crate::vfs::exists(p);
                                ui.horizontal(|ui| {
                                    let r = ui.add_enabled(exists, egui::Link::new(file_name(p)));
                                    if r.clicked() {
                                        open = Some(p.clone());
                                    }
                                    let folder = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
                                    ui.add(egui::Label::new(RichText::new(if exists { folder } else { "(file no longer exists)".into() }).weak().size(12.5)).truncate());
                                });
                            }
                        });
                        ui.add_space(8.0);
                        theme::card(ui, |ui| {
                            ui.label(RichText::new("Find your saves").family(theme::semibold_family()).size(16.0));
                            ui.weak(if WEB {
                                "The browser can't look through your folders, so pick the file yourself. On Windows most games \
                                 keep saves in Documents\\My Games, %APPDATA%, %LOCALAPPDATA% or Steam\\userdata \u{2014} \
                                 type one of these into the file picker's address bar to jump there."
                            } else if cfg!(target_os = "android") {
                                "Common places emulators and games keep save files:"
                            } else {
                                "Common places games keep save files on this PC:"
                            });
                            for (label, path) in settings::save_locations() {
                                ui.horizontal(|ui| {
                                    if ui.link(&label).clicked() {
                                        browse = Some(Some(path.clone()));
                                    }
                                    ui.add(egui::Label::new(RichText::new(path.display().to_string()).weak().size(12.5)).truncate());
                                });
                            }
                            ui.weak("RPG Maker games usually keep saves in a \u{201C}save\u{201D} folder next to the game.");
                        });
                        ui.add_space(8.0);
                        theme::card(ui, |ui| {
                            ui.label(RichText::new("What it can open").family(theme::semibold_family()).size(16.0));
                            let games = format!(
                                "{} games, {} of them with a screen of their own: Elden Ring, Baldur's Gate, Mass Effect, Final Fantasy, Stardew Valley, Terraria, Cyberpunk 2077 and many more \u{00B7} Pok\u{00E9}mon Gen I\u{2013}III (emulator saves) \u{00B7} {}",
                                crate::games::GAMES.len(),
                                crate::games::GAMES.iter().filter(|g| g.support.starts_with("Game screen")).count(),
                                if WEB { "the full list is on the website" } else { "Select game lists them all" },
                            );
                            let data_formats = if WEB {
                                "JSON \u{00B7} XML \u{00B7} INI/CFG \u{00B7} TOML \u{00B7} YAML \u{00B7} plist \u{00B7} MessagePack \u{00B7} BSON \u{00B7} Python pickle (SQLite needs the downloaded app)"
                            } else {
                                "JSON \u{00B7} XML \u{00B7} INI/CFG \u{00B7} TOML \u{00B7} YAML \u{00B7} plist \u{00B7} MessagePack \u{00B7} BSON \u{00B7} SQLite \u{00B7} Python pickle"
                            };
                            for (title, list) in [
                                ("Games", games.as_str()),
                                ("Game engines", "Ren'Py \u{00B7} RPG Maker MV/MZ/XP/VX/VX Ace \u{00B7} Unreal Engine (GVAS) \u{00B7} Unity (.NET BinaryFormatter, Easy Save 3, JSON, XML) \u{00B7} Godot (store_var, encrypted) \u{00B7} Minecraft NBT \u{00B7} TyranoScript \u{00B7} Twine/SugarCube \u{00B7} Barotrauma \u{00B7} Palworld (PlZ)"),
                                ("Data formats", data_formats),
                                ("Wrappers", "gzip \u{00B7} zlib \u{00B7} zstd \u{00B7} LZ4 \u{00B7} xz \u{00B7} bzip2 \u{00B7} zip \u{00B7} Base64 \u{00B7} UTF-16 \u{00B7} XOR \u{00B7} AES (with password)"),
                                ("Anything else", "Any file opens in the hex editor with value search"),
                            ] {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(format!("{title}:")).strong());
                                    ui.weak(list);
                                });
                            }
                        });
                    });
                });
            });
        });
        if let Some(p) = open {
            self.request_open(p);
        }
        if let Some(dir) = browse {
            self.ask(Ask::Open, dir, None);
        }
    }
}

/// "SAVEGAMES/AUTO_INPACK_2026.02.08_16.58.55.SAV" → "2026-02-08 16:58:55 (auto)"; QUICK.SAV keeps its name
/// (the game writes it itself; it is not a manual quick save).
/// The labels sort by date.
pub(crate) fn cairn_save_label(entry: &str) -> Option<String> {
    let name = entry.strip_prefix("SAVEGAMES/")?.strip_suffix(".SAV")?;
    if let Some(stamp) = name.rsplit('_').nth(1).zip(name.rsplit('_').next()).filter(|_| name.contains('_')) {
        let (date, time) = stamp;
        if date.len() == 10 && time.len() == 8 {
            let kind = name.split('_').next().unwrap_or("").to_lowercase();
            let kind = if kind == "auto" { "auto".to_string() } else { kind };
            return Some(format!("{} {} ({kind})", date.replace('.', "-"), time.replace('.', ":")));
        }
    }
    Some(match name {
        "QUICK" => "QUICK.SAV (written by the game)".into(),
        n => n.replace('_', " ").to_lowercase(),
    })
}

/// Short, player-facing name for the detected format.
fn friendly_kind(layers: &[Layer], content: &Content) -> String {
    match content {
        Content::Tree(t) => t.kind_name.clone(),
        Content::Json { value, .. } if layers.iter().any(|l| matches!(l, Layer::Rgg(_))) => {
            value.pointer("/base/m_title").and_then(|t| t.as_str()).map(|t| format!("{t} save")).unwrap_or_else(|| "Yakuza save".into())
        }
        Content::Json { .. } if layers.contains(&Layer::LzString) => "RPG Maker MV".into(),
        Content::Json { .. } if layers.contains(&Layer::RpgMzZlib) => "RPG Maker MZ".into(),
        Content::Json { value, .. } if names::is_rpgmaker(value) => "RPG Maker".into(),
        Content::Binary(_) if layers.iter().any(|l| matches!(l, Layer::Copeck { .. })) => "Cairn save".into(),
        Content::Binary(_) => match layers.last() {
            Some(Layer::Mandarin { game, .. }) => format!("{} save", crate::fmt::mandarin::GAMES[*game].title),
            Some(Layer::Lime { .. }) => "Resident Evil 4 save".into(),
            Some(Layer::Citrus { .. }) => "Monster Hunter Rise save".into(),
            Some(Layer::Civ5 { .. }) => "Civilization V save".into(),
            Some(Layer::Cdpr { prefix, .. }) if prefix.starts_with(b"VASC") => "Cyberpunk 2077 save".into(),
            Some(Layer::Cdpr { .. }) => "The Witcher 3 save".into(),
            Some(Layer::Bnd4 { .. }) => "FromSoftware save".into(),
            _ => "Binary (hex)".into(),
        },
        c => c.kind().to_string(),
    }
}

/// Phone-sized window: rows that are side by side on desktop wrap or stack instead.
fn narrow(ui: &egui::Ui) -> bool {
    ui.ctx().screen_rect().width() < 640.0
}

fn file_name(p: &Path) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(n) = crate::xgp::logical_name(p) {
        return format!("{n} (Xbox app)");
    }
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

pub(crate) fn human_size(n: usize) -> String {
    match n {
        n if n < 1024 => format!("{n} bytes"),
        n if n < 1024 * 1024 => format!("{:.1} KB", n as f64 / 1024.0),
        n => format!("{:.1} MB", n as f64 / 1024.0 / 1024.0),
    }
}

/// Contiguous ranges where `a` differs from `b`, merging gaps of < 4 bytes.
fn byte_ranges(a: &[u8], b: &[u8]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for i in (0..a.len().min(b.len())).filter(|&i| a[i] != b[i]) {
        match out.last_mut() {
            Some((_, end)) if i <= *end + 3 => *end = i + 1,
            _ => out.push((i, i + 1)),
        }
    }
    out
}

fn type_combo(ui: &mut egui::Ui, id: &str, ty: &mut NumType) {
    egui::ComboBox::from_id_salt(id).selected_text(ty.name()).width(120.0).show_ui(ui, |ui| {
        for t in NumType::ALL {
            ui.selectable_value(ty, t, t.name());
        }
    });
}

impl eframe::App for App {
    #[cfg(target_os = "android")]
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let _ = platform::android::CTX.set(ctx.clone());
        platform::android::take_typed(raw);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);
        #[cfg(not(target_os = "android"))]
        self_shot(ctx);

        // Files dropped on the window.
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone())) {
            self.request_open(p);
        }
        #[cfg(target_arch = "wasm32")]
        {
            // The browser hands over the dropped file's contents, not a path.
            if let Some((name, bytes)) = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| Some((f.name.clone(), f.bytes.clone()?)))) {
                let p = crate::web::keep(&name, &bytes);
                self.request_open(p);
            }
            if let Some(p) = crate::web::take_picked() {
                match self.web_asked.take() {
                    Some(what) => self.answered(what, p),
                    None => self.request_open(p),
                }
            }
        }
        // Shortcuts.
        let no_text_focus = ctx.memory(|m| m.focused().is_none());
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.ask(Ask::Open, None, None);
        }
        if self.doc.is_some() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_clicked();
        }
        let structured = self.doc.as_ref().is_some_and(|d| root(&d.content).is_some());
        if structured && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.focus_search = true;
            self.tab = if self.tab == Tab::Raw { Tab::Raw } else { Tab::Values };
        }
        if no_text_focus {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z))
                || ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y))
            {
                self.undo_redo(false);
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.undo_redo(true);
            }
        }
        // Remember zoom.
        let zoom = ctx.zoom_factor();
        if (zoom - self.settings.zoom).abs() > 0.001 {
            self.settings.zoom = zoom;
            self.settings.save();
        }
        // Don't lose work on close.
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }

        let title = match &self.doc {
            Some(d) => format!("{}{} \u{2014} {}", if self.dirty { "\u{2022} " } else { "" }, file_name(&d.path), platform::DISPLAY_NAME),
            None => platform::DISPLAY_NAME.into(),
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }

        // Android: keep clear of the taskbar / navigation buttons drawn over the window.
        #[cfg(target_os = "android")]
        {
            let px = platform::android::bottom_inset_px();
            if px > 0.0 {
                egui::TopBottomPanel::bottom("system_bars")
                    .exact_height(px / ctx.pixels_per_point())
                    .frame(egui::Frame::none().fill(ctx.style().visuals.panel_fill))
                    .show_separator_line(false)
                    .show(ctx, |_| {});
            }
        }
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu_bar(ui));
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.add_space(2.0);
            self.status_bar(ui);
        });

        let kind = self.doc.as_ref().map(|d| match d.content {
            Content::Binary(_) => 'b',
            Content::Text(_) => 't',
            _ => 's',
        });
        if self.show_changes && self.doc.is_some() && kind != Some('t') {
            self.changes_panel(ctx, now);
        }
        if kind == Some('b') {
            self.hex_side_panel(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| match kind {
            None => self.welcome(ui),
            Some('s') => self.structured_panel(ui, now),
            Some('t') => {
                let Some(Doc { content: Content::Text(text), .. }) = &mut self.doc else { return };
                let changed = ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| ui.add(TextEdit::multiline(text).code_editor().desired_width(f32::INFINITY)).changed())
                    .inner;
                if changed {
                    self.recompute_dirty();
                }
            }
            _ => {
                let Some(Doc { content: Content::Binary(data), baseline: Content::Binary(base), .. }) = &self.doc else { return };
                if data.is_empty() {
                    ui.weak("(empty file)");
                } else {
                    hex_view::hex_grid(ui, data, base, &mut self.hex);
                }
            }
        });

        // Dialogs.
        if let Some(path) = self.pending_open.clone() {
            let mut choice = None;
            modal(ctx, "Unsaved changes", |ui| {
                ui.label(format!("You have unsaved changes. Save them before opening {}?", file_name(&path)));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Save").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                        choice = Some(0);
                    }
                    if ui.button("Don't save").clicked() {
                        choice = Some(1);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(2);
                    }
                });
            });
            match choice {
                Some(0) => {
                    self.save();
                    self.pending_open = None;
                    if !self.dirty {
                        self.load_async(&path);
                    }
                }
                Some(1) => {
                    self.pending_open = None;
                    self.dirty = false;
                    self.load_async(&path);
                }
                Some(_) => self.pending_open = None,
                None => {}
            }
        }
        self.password_dialog(ctx);
        self.sums_dialog(ctx);
        self.backup_name_dialog(ctx);
        self.rebind_dialog(ctx);
        self.poll_loading(ctx, now);
        self.save_review_dialog(ctx);
        self.copy_slots_dialog(ctx);
        self.game_picker_dialog(ctx);
        self.compare_window(ctx, now);
        self.browser_window(ctx);
        // Android: bring up the on-screen keyboard while a text field has focus.
        #[cfg(target_os = "android")]
        {
            let wants = ctx.wants_keyboard_input();
            if wants != self.keyboard_shown {
                self.keyboard_shown = wants;
                platform::android::set_keyboard(wants);
            }
        }
        if self.confirm_close {
            let mut choice = None;
            modal(ctx, "Unsaved changes", |ui| {
                ui.label("You have unsaved changes. Save them before closing?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("Save and close").color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                        choice = Some(0);
                    }
                    if ui.button("Close without saving").clicked() {
                        choice = Some(1);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(2);
                    }
                });
            });
            match choice {
                Some(0) => {
                    self.save();
                    self.confirm_close = false;
                    if !self.dirty {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Some(1) => {
                    self.confirm_close = false;
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(_) => self.confirm_close = false,
                None => {}
            }
        }
    }
}

/// Replaces the file at `path` with `bytes`. On a computer: through a temporary file swapped
/// in, so a crash can never leave a half-written save. In the browser: as a download.
fn write_in_place(path: &Path, bytes: &[u8]) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::web::download(&file_name(path), bytes)?;
        crate::vfs::write(path, bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".saving");
        let tmp = PathBuf::from(tmp);
        if let Err(e) = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path)) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.to_string());
        }
        Ok(())
    }
}

fn modal(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("modal_dim")).fixed_pos(screen.min).order(egui::Order::Middle).show(ctx, |ui| {
        ui.allocate_response(screen.size(), egui::Sense::click());
        ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(140));
    });
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, add);
}

/// Testing aid: with SAVE_EDITOR_SHOT=<file.png> set, the window saves a picture of
/// itself a few seconds after starting and then closes (SAVE_EDITOR_SHOT_AFTER = seconds,
/// default 4). Works where screen capture doesn't (e.g. while the display is off).
#[cfg(not(target_os = "android"))]
fn self_shot(ctx: &egui::Context) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static ASKED: AtomicBool = AtomicBool::new(false);
    let Some(out) = std::env::var_os("SAVE_EDITOR_SHOT") else { return };
    for e in ctx.input(|i| i.raw.events.clone()) {
        if let egui::Event::Screenshot { image, .. } = e {
            let _ = std::fs::write(&out, png(&image));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    let after: f64 = std::env::var("SAVE_EDITOR_SHOT_AFTER").ok().and_then(|s| s.parse().ok()).unwrap_or(4.0);
    if ctx.input(|i| i.time) > after && !ASKED.swap(true, Ordering::SeqCst) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(200));
}

/// A plain RGBA PNG.
#[cfg(not(target_os = "android"))]
fn png(img: &egui::ColorImage) -> Vec<u8> {
    use std::io::Write;
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut crc = flate2::Crc::new();
        crc.update(kind);
        crc.update(data);
        out.extend(kind);
        out.extend(data);
        out.extend(crc.sum().to_be_bytes());
    }
    let [w, h] = img.size;
    let mut raw = Vec::with_capacity((w * 4 + 1) * h);
    for row in img.pixels.chunks(w) {
        raw.push(0);
        for p in row {
            raw.extend(p.to_srgba_unmultiplied());
        }
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    let _ = z.write_all(&raw);
    let idat = z.finish().unwrap_or_default();
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend((w as u32).to_be_bytes());
    ihdr.extend((h as u32).to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &idat);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod cairn_label_tests {
    #[test]
    fn labels() {
        assert_eq!(super::cairn_save_label("SAVEGAMES/AUTO_INPACK_2026.02.08_16.58.55.SAV").unwrap(), "2026-02-08 16:58:55 (auto)");
        assert_eq!(super::cairn_save_label("SAVEGAMES/QUICK.SAV").unwrap(), "QUICK.SAV (written by the game)");
        assert!(super::cairn_save_label("PATHS/FRAG_1").is_none());
    }
}
