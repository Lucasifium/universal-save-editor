use crate::file_browser::{self, FileBrowser};
use crate::widgets::{ListBox, ScrollLock};
use crate::i18n::{tr, trf};
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
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

mod help;
mod home;
mod header;
mod pointers;
mod safety;

use safety::Stamp;
mod tools;
use tools::{Bulk, NamesAction, PresetApply, PresetSave, ShareNames};
mod tabs;
mod compare;
#[cfg(target_arch = "wasm32")]
mod in_place;
use compare::Compare;
pub(crate) mod window_size;
mod picker_extra;
mod pinning;
mod open_as;
mod review;
mod slots;
use slots::CopySlots;
mod xbox_steam;
use xbox_steam::XboxSteam;
mod backup_tools;
mod palette;
mod saving;
mod loading;
mod editing;
mod menus;
mod dialogs;
mod patches;
mod toolbar;
mod structured;
mod side_panels;
mod welcome;
mod frame;
mod modal;
mod game_picker;
#[cfg(not(target_os = "android"))]
mod self_shot;
use saving::*;
use loading::*;
use menus::*;
use patches::*;
use side_panels::*;
use modal::*;
use game_picker::*;
#[cfg(not(target_os = "android"))]
use self_shot::*;
pub(crate) use modal::{modal_shown, safe_choice, tab_ring};

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
    Patch,
    PresetFile,
    ExportNames,
    ImportNames,
    ExportBackups,
    ImportBackups,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum StatusKind {
    Info,
    Ok,
    Err,
}

enum Change {
    /// `extra`: values that changed along with it (pointer, before, after).
    Value { ptr: String, before: Value, after: Value, extra: Vec<(String, Value, Value)> },
    Bytes { off: usize, before: Vec<u8>, after: Vec<u8> },
    /// Several changes made in one go (bulk edit, a preset), undone together.
    Batch(Vec<Change>),
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
    /// The file's size and time when it was read or last saved, to notice when something
    /// else (the game, cloud sync) writes it in the meantime.
    stamp: Option<Stamp>,
    /// Browser version: the id of the file handle this document saves back through (opened
    /// with "Open to save in place"). It belongs to this document alone: a document read again
    /// or saved under another name doesn't have it.
    handle: Option<HandleId>,
}

/// Browser version: a document's file handle, by id (see `web.rs`). The browser lets go of the
/// handle when the document does (closed, read again, or saved under another name).
#[derive(Debug, PartialEq, Eq)]
struct HandleId(u64);

impl Drop for HandleId {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        crate::web::release_handle(self.0);
    }
}

/// The values edited since the file was opened or last saved, in the order they were first
/// edited. (A value put back as it was stays listed; what counts is whether it still differs.)
#[derive(Default)]
struct Touched {
    list: Vec<String>,
    seen: HashSet<String>,
    /// Values changed together by one button press (Max crafting materials, unlock all...),
    /// by group number. A value edited alone afterwards leaves its group.
    group: HashMap<String, u64>,
    next_group: u64,
}

impl Touched {
    fn insert(&mut self, ptr: &str) {
        if !self.seen.contains(ptr) {
            self.seen.insert(ptr.to_string());
            self.list.push(ptr.to_string());
        }
    }

    fn clear(&mut self) {
        self.list.clear();
        self.seen.clear();
        self.group.clear();
    }

    /// Puts these values in one new group, so reverting one of them offers to revert all.
    fn set_group(&mut self, ptrs: &[String]) {
        self.next_group += 1;
        for p in ptrs {
            self.group.insert(p.clone(), self.next_group);
        }
    }

    fn leave_group(&mut self, ptr: &str) {
        self.group.remove(ptr);
    }

    fn iter(&self) -> std::slice::Iter<'_, String> {
        self.list.iter()
    }
}

/// What differs from the file as opened or last saved. Worked out once per edit, not on every
/// frame: comparing a big file byte by byte several times a frame makes the window lag.
#[derive(Default)]
struct Changes {
    /// The `App::edit_gen` this was worked out for.
    gen: Option<u64>,
    /// Hex view: the byte ranges that differ.
    ranges: Vec<(usize, usize)>,
    /// Structured saves: the edited values that differ.
    ptrs: Vec<String>,
    /// A text file that was edited.
    text: bool,
}

impl Changes {
    fn of(d: &Doc, touched: &Touched) -> Changes {
        let mut out = Changes::default();
        match (&d.content, &d.baseline) {
            (Content::Binary(a), Content::Binary(b)) => out.ranges = byte_ranges(a, b),
            (Content::Text(a), Content::Text(b)) => out.text = a != b,
            (c, b) => {
                if let (Some(cv), Some(bv)) = (root(c), root(b)) {
                    out.ptrs = touched.iter().filter(|p| cv.pointer(p) != bv.pointer(p)).cloned().collect();
                }
            }
        }
        out
    }

    fn count(&self) -> usize {
        self.ranges.len() + self.ptrs.len() + usize::from(self.text)
    }
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

/// The "about to save" dialog: what changed, and whether the game is open.
struct SaveReview {
    path: PathBuf,
    running: Option<String>,
    lines: Vec<review::Line>,
}

pub struct App {
    doc: Option<Doc>,
    settings: Settings,
    has_icons: bool,
    status: String,
    status_kind: StatusKind,
    pending_open: Option<PathBuf>,
    confirm_close: bool,
    /// File › Close save is waiting on the unsaved-changes question.
    pending_close: bool,
    /// What the editor wants to say about the file just opened (what the screen edits, a missing data folder...).
    doc_note: String,
    allow_close: bool,
    window_title: String,
    logo: Option<egui::TextureHandle>,
    dirty: bool,

    // Editing history.
    undo: Vec<Change>,
    redo: Vec<Change>,
    last_edit: Option<(String, f64)>,
    touched: Touched,
    /// While a button's edits are being applied: every value they changed.
    trace: Option<Vec<String>>,
    /// What differs from the file as opened or last saved, for `edit_gen`.
    changes: Changes,

    // Structured view state.
    tab: Tab,
    /// The open section tab of a game screen (a panel's title; empty: the first), and which of
    /// Values or Tree the Advanced tab shows.
    game_section: String,
    advanced_tab: Tab,
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
    /// Values pinned to the top of the Values list, per game.
    pins: crate::pins::Pins,
    /// Text typed in the game screen's find box, and which match is current.
    game_find: String,
    game_find_idx: usize,
    focus_game_find: bool,
    /// Tree search: pointers kept for (search, model_gen, edit_gen).
    tree_visible: Option<(HashSet<String>, (String, u64, u64))>,
    /// "Show in Tree": the value to reveal and when that began.
    tree_reveal: Option<(String, f64)>,
    raw_text: String,
    raw_error: Option<String>,
    show_changes: bool,
    /// The file note is shown whole instead of on one line.
    note_open: bool,

    hex: HexState,
    /// Text typed into the "Decrypt with password" dialog while it's open.
    password_prompt: Option<String>,
    /// Checksums found by Tools › Find checksum, while their window is open.
    sum_hits: Option<Vec<crate::fmt::sumfind::Sum>>,
    /// The name being typed for a kept copy (Backups › Quick restore › Name a backup).
    backup_name: Option<String>,
    /// The name being typed for Backups › Back up and start a new game.
    new_game_name: Option<String>,
    /// The Save manager window: `Some((save it is for, backup name being typed, version waiting for a delete/replace confirmation))` while open.
    /// It stays open for a save that was set aside (or is missing), so a backup can be put back.
    manager: Option<(PathBuf, String, Option<(PathBuf, bool)>)>,
    /// Folder backups, checks, export and import in the Save manager, and the Backup space window.
    backup_tools: backup_tools::Tools,
    /// Tools › Change Steam account: the IDs found in the save, the one picked, and the new one typed.
    rebind: Option<(Vec<u64>, usize, String)>,
    patches: Option<Patches>,
    compare: Option<Compare>,
    save_review: Option<SaveReview>,
    copy_slots: Option<CopySlots>,
    xbox_steam: Option<XboxSteam>,
    game_picker: Option<GamePicker>,
    /// Search everything (Ctrl+K), while it is open.
    palette: Option<palette::Palette>,
    /// Bulk edit in the Values tab, while it is on.
    bulk: Option<Bulk>,
    preset_save: Option<PresetSave>,
    preset_apply: Option<PresetApply>,
    /// Backups › Quick restore: the save to restore over, the backup picked and how messages
    /// name it, while the "Restore this backup?" dialog waits for an answer.
    quick_restore: Option<(PathBuf, PathBuf, String)>,
    share_names: Option<ShareNames>,
    loading: Option<Loading>,
    saving: Option<Saving>,
    /// The window was asked to close while a save was being written: close once it's done.
    close_after_save: bool,
    /// Close the window at the end of this frame (a "Save and close" went through).
    close_now: bool,
    /// `model` is the Values list of `doc.baseline` (it was built when the file was opened or
    /// saved, and edits since haven't rebuilt it).
    model_from_baseline: bool,
    /// The game that owns the open save, if it was running when the file was opened.
    running: Option<String>,
    /// The built-in file browser, where there are no native dialogs (Android).
    browser: Option<(Ask, FileBrowser)>,
    /// A failure, shown under the toolbar until dismissed.
    banner: Option<String>,
    /// The field whose refused value put up the current error, so fixing it can clear the error.
    error_ptr: Option<String>,
    /// The open file was changed on disk by something else since it was read or saved.
    disk_changed: bool,
    /// The changed file the player chose to keep working past ("Keep what's shown").
    disk_seen: Option<Stamp>,
    /// Saving over a file that changed on disk: waiting for Reload / Save anyway / Cancel,
    /// with what was to happen once the save went through.
    stale_save: Option<(PathBuf, AfterSave)>,
    /// "Save anyway" was chosen for the next save.
    stale_ok: bool,
    /// Whether the window had the keyboard focus last frame (a file check runs on return).
    focused: bool,
    /// The saves found on this computer, for "Your games" on the start page and Find my game's saves.
    saves_found: Option<SaveSearch>,
    prefs_open: bool,
    /// The optional new-version check (see `update.rs`): the pending answer, a newer version
    /// found, and the note after a check the player asked for.
    update_rx: Option<std::sync::mpsc::Receiver<crate::update::Outcome>>,
    update_found: Option<String>,
    update_note: Option<&'static str>,
    update_manual: bool,
    update_started: bool,
    /// The one-time "check for updates?" dialog is showing (real launches only).
    update_question: bool,
    help: Option<help::Window>,
    /// Load the Japanese, Chinese and Korean fonts on the next frame.
    need_cjk: bool,
    /// Other open files (see `tabs`) and copied values.
    tabs: tabs::Tabs,
    /// Files read and shown so far (counts each `finish_load`), to tell whether one just opened.
    loads: u64,
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
        crate::i18n::set(settings.language.as_deref());
        if crate::i18n::current().cjk && !theme::cjk_available() {
            // Without a font for it, the menus would be boxes.
            crate::i18n::set(Some("en"));
        }
        crate::backups::set_root(settings.backup_dir.clone());
        crate::backups::set_earlier_roots(settings.earlier_backup_dirs.clone());
        crate::backups::set_keep(settings.backup_keep);
        crate::backups::space::set_cap(settings.backup_cap_gb.saturating_mul(crate::backups::space::GB));
        if !WEB {
            // Backups made before the editor kept a list of its backups folders count toward a size limit too.
            let recent = settings.recent.clone();
            std::thread::spawn(move || crate::backups::space::remember_existing(&recent));
        }
        let has_icons = theme::install_fonts(&cc.egui_ctx);
        theme::apply_choice(&cc.egui_ctx, settings.dark, settings.follow_system);
        cc.egui_ctx.set_zoom_factor(settings.zoom);
        crate::drives::set_include_removable(settings.search_removable);
        let mut app = Self::with(&cc.egui_ctx, settings, has_icons);
        app.update_question = crate::update::AVAILABLE && !app.settings.update_question_answered;
        // Phones and the browser can't look through folders.
        if !cfg!(target_os = "android") && !WEB && std::env::var_os("SAVE_EDITOR_NO_SEARCH").is_none() {
            app.saves_found = Some(search_saves());
        }
        // "--select-game" opens the game list instead of a file; "--select-game=elden" also
        // picks the first game whose name contains that.
        let initial = match initial {
            Some(p) if p.to_string_lossy().starts_with("--select-game") => {
                let mut gp = app.picker();
                if let Some((_, q)) = p.to_string_lossy().split_once('=') {
                    let q = q.to_lowercase();
                    gp.picked = crate::games::GAMES.iter().position(|g| crate::games::name_matches(g.name, &q));
                }
                app.game_picker = Some(gp);
                None
            }
            other => other,
        };
        if let Some(p) = initial {
            app.load_async(&p);
        }
        for p in app.settings.recent.clone() {
            theme::cjk_for(&cc.egui_ctx, &p.to_string_lossy());
        }
        app
    }

    /// The app with nothing open, before `new` starts anything (and as tests build it).
    fn with(ctx: &egui::Context, settings: Settings, has_icons: bool) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = ctx;
        Self {
            doc: None,
            settings,
            has_icons,
            status: tr("Ready. Open a save file to get started.").into(),
            status_kind: StatusKind::Info,
            pending_open: None,
            confirm_close: false,
            pending_close: false,
            doc_note: String::new(),
            allow_close: false,
            window_title: String::new(),
            logo: None,
            dirty: false,
            undo: vec![],
            redo: vec![],
            last_edit: None,
            touched: Touched::default(),
            trace: None,
            changes: Changes::default(),
            tab: Tab::Values,
            game_section: String::new(),
            advanced_tab: Tab::Values,
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
            pins: crate::pins::Pins::load(),
            game_find: String::new(),
            game_find_idx: 0,
            focus_game_find: false,
            tree_visible: None,
            tree_reveal: None,
            raw_text: String::new(),
            raw_error: None,
            show_changes: false,
            note_open: false,
            hex: HexState::default(),
            password_prompt: None,
            sum_hits: None,
            backup_name: None,
            new_game_name: None,
            manager: None,
            backup_tools: Default::default(),
            rebind: None,
            patches: None,
            compare: None,
            save_review: None,
            copy_slots: None,
            xbox_steam: None,
            game_picker: None,
            palette: None,
            bulk: None,
            preset_save: None,
            preset_apply: None,
            quick_restore: None,
            share_names: None,
            loading: None,
            saving: None,
            close_after_save: false,
            close_now: false,
            model_from_baseline: false,
            running: None,
            browser: None,
            banner: None,
            error_ptr: None,
            disk_changed: false,
            disk_seen: None,
            stale_save: None,
            stale_ok: false,
            focused: true,
            saves_found: None,
            prefs_open: false,
            update_rx: None,
            update_found: None,
            update_note: None,
            update_manual: false,
            update_started: false,
            update_question: false,
            help: None,
            // Japanese and Chinese menus need those fonts from the start.
            need_cjk: crate::i18n::current().cjk,
            tabs: tabs::Tabs::new(),
            loads: 0,
            #[cfg(target_os = "android")]
            keyboard_shown: false,
            #[cfg(target_arch = "wasm32")]
            web_asked: None,
            #[cfg(target_arch = "wasm32")]
            ctx: ctx.clone(),
        }
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
        self.banner = Some(self.status.clone());
    }

    /// Asks for a file or folder: a native dialog on desktop, the built-in browser on Android.
    fn ask(&mut self, what: Ask, dir: Option<PathBuf>, name: Option<String>) {
        let dir = dir.or_else(|| self.doc.as_ref().and_then(|d| d.path.parent().map(Path::to_path_buf)));
        let (title, mode) = match what {
            Ask::Open => (tr("Open a save file"), file_browser::Mode::OpenFile),
            Ask::DataDir => (tr("Select the game's data folder (it contains System.json)"), file_browser::Mode::Folder),
            Ask::SaveAs => (tr("Save a copy as"), file_browser::Mode::SaveFile),
            Ask::Export => (tr("Export decoded data as"), file_browser::Mode::SaveFile),
            Ask::Compare => (tr("Compare with which save?"), file_browser::Mode::OpenFile),
            Ask::BackupDir => (tr("Keep backups in which folder?"), file_browser::Mode::Folder),
            Ask::Import => (tr("Import which edited data?"), file_browser::Mode::OpenFile),
            Ask::Patch => (tr("Open which patch file (.savepatch)?"), file_browser::Mode::OpenFile),
            Ask::PresetFile => (tr("Open which preset?"), file_browser::Mode::OpenFile),
            Ask::ExportNames => (tr("Export the names to"), file_browser::Mode::SaveFile),
            Ask::ImportNames => (tr("Import names from which file?"), file_browser::Mode::OpenFile),
            Ask::ExportBackups => (tr("Export the backups to"), file_browser::Mode::SaveFile),
            Ask::ImportBackups => (tr("Import backups from which file?"), file_browser::Mode::OpenFile),
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
                file_browser::Mode::Folder => self.fail(tr("The browser version can't open folders. Use the desktop app for this.")),
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
            Ask::SaveAs => {
                // Browser: a copy is always a download, even under the file's own name. Set
                // here, where the save follows at once, so nothing else can pick it up.
                #[cfg(target_arch = "wasm32")]
                crate::web::download_next();
                self.save_to(&p)
            }
            Ask::Export => self.export_to(&p),
            Ask::Compare => self.compare_with(p),
            Ask::Import => self.import_from(&p),
            Ask::Patch => self.load_patch_file(&p),
            Ask::PresetFile => self.open_preset_file(&p),
            Ask::ExportNames => self.export_names(&p),
            Ask::ImportNames => self.import_names(&p),
            Ask::ExportBackups => self.export_backups(&p),
            Ask::ImportBackups => self.import_backups(&p),
            Ask::BackupDir => {
                self.ok(trf("Backups now go to {0}. Earlier ones stay where they were and are still listed.", &[&p.display()]));
                self.settings.set_backup_dir(Some(p));
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

    /// Closes the open file: shows the next tab, or the start page if it was the only one.
    fn close_file(&mut self) {
        if self.saving.is_some() {
            return;
        }
        self.dirty = false;
        self.drop_tab(self.tabs.active);
        self.info(tr("Closed the file."));
    }

    /// Close save (button, File menu, Ctrl+W): asks first when there are unsaved changes.
    fn close_clicked(&mut self) {
        if self.doc.is_none() || self.saving.is_some() {
            return;
        }
        if self.dirty {
            self.pending_close = true;
        } else {
            self.close_file();
        }
    }

    /// Escape: closes the dialog on top (the one drawn last), or does nothing when none is open.
    fn close_top_dialog(&mut self) {
        if self.loading.is_some() {
            self.cancel_loading();
        } else if self.update_question {
            // Not answered: asked again on the next launch.
            self.update_question = false;
        } else if self.quick_restore.is_some() {
            self.quick_restore = None;
        } else if self.preset_apply.as_ref().is_some_and(|pa| pa.confirm_delete.is_some()) {
            if let Some(pa) = &mut self.preset_apply {
                pa.confirm_delete = None;
            }
        } else if self.browser.is_some() {
            self.browser = None;
        } else if self.share_names.is_some() {
            self.share_names = None;
        } else if self.preset_save.is_some() {
            self.preset_save = None;
        } else if self.preset_apply.is_some() {
            self.preset_apply = None;
        } else if self.stale_save.is_some() {
            self.stale_save = None;
        } else if self.help.is_some() {
            self.help = None;
        } else if self.prefs_open {
            self.prefs_open = false;
        } else if self.save_review.is_some() {
            self.save_review = None;
        } else if self.confirm_close {
            self.confirm_close = false;
        } else if self.pending_close {
            self.pending_close = false;
        } else if self.pending_open.is_some() {
            self.pending_open = None;
        } else if self.copy_slots.is_some() {
            self.copy_slots = None;
        } else if self.xbox_steam.is_some() {
            self.xbox_steam = None;
        } else if self.game_picker.is_some() {
            self.game_picker = None;
        } else if self.rebind.is_some() {
            self.rebind = None;
        } else if self.password_prompt.is_some() {
            self.password_prompt = None;
        } else if self.sum_hits.is_some() {
            self.sum_hits = None;
        } else if self.backup_name.is_some() {
            self.backup_name = None;
        } else if self.new_game_name.is_some() {
            self.new_game_name = None;
        } else if self.patches.is_some() {
            self.patches = None;
        } else if self.compare.is_some() {
            self.compare = None;
        } else if self.manager.is_some() {
            self.manager = None;
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

/// A format name in the language in use: "<game> save" names go through the "{0} save" text,
/// the rest through the catalog when it has them.
fn kind_label(kind: &str) -> String {
    let whole = crate::i18n::tr_any(kind);
    if whole != kind {
        return whole.to_string();
    }
    match kind.strip_suffix(" save") {
        Some(game) if !game.is_empty() => trf("{0} save", &[&game]),
        _ => crate::i18n::tr_any(kind).to_string(),
    }
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
/// A window width that fits on the screen (phones, small browser windows), at most `want`.
fn fit(ctx: &egui::Context, want: f32) -> f32 {
    // The screen less the window's own margins and a few pixels either side.
    let m = ctx.style().spacing.window_margin.sum().x;
    want.min(ctx.screen_rect().width() - 12.0 - m).max(200.0)
}

/// The phone toolbar's menu button (U+2630, "trigram for heaven").
const PHONE_MENU: &str = "\u{2630}";

/// Below this window width the layout is the phone one.
const PHONE_W: f32 = 640.0;

fn narrow(ui: &egui::Ui) -> bool {
    ui.ctx().screen_rect().width() < PHONE_W
}

/// A recent file for a list: "name \u{2014} parent folder", since many games use the same file names.
fn recent_label(p: &Path) -> String {
    let parent = p.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().into_owned()).filter(|n| !n.is_empty());
    match parent {
        Some(d) => format!("{} \u{2014} {d}", file_name(p)),
        None => file_name(p),
    }
}

fn file_name(p: &Path) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(n) = crate::xgp::logical_name(p) {
        return format!("{n} (Xbox app)");
    }
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// Saves at least this big can take a noticeable time to open.
const LARGE_SAVE: u64 = 20 << 20;

/// A short note for save lists on a big file, so a wait is expected; `None` for the rest.
pub(crate) fn large_save_note(size: u64) -> Option<String> {
    (size >= LARGE_SAVE).then(|| tr("large \u{2014} may take a while to open").to_string())
}

pub(crate) fn human_size(n: usize) -> String {
    match n {
        n if n < 1024 => format!("{n} bytes"),
        n if n < 1024 * 1024 => format!("{:.1} KB", n as f64 / 1024.0),
        n => format!("{:.1} MB", n as f64 / 1024.0 / 1024.0),
    }
}

#[cfg(test)]
mod dialog_focus_tests;

#[cfg(test)]
mod save_tests;

#[cfg(test)]
mod cairn_label_tests {
    #[test]
    fn labels() {
        assert_eq!(super::cairn_save_label("SAVEGAMES/AUTO_INPACK_2026.02.08_16.58.55.SAV").unwrap(), "2026-02-08 16:58:55 (auto)");
        assert_eq!(super::cairn_save_label("SAVEGAMES/QUICK.SAV").unwrap(), "QUICK.SAV (written by the game)");
        assert!(super::cairn_save_label("PATHS/FRAG_1").is_none());
    }
}

#[cfg(test)]
pub(crate) mod layout_tests;
