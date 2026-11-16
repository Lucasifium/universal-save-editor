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
mod safety;

use safety::Stamp;
mod tools;
use tools::{Bulk, NamesAction, PresetApply, PresetSave, ShareNames};
mod tabs;

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

/// Result of comparing the open save with another file.
/// Files at least this big are opened on a worker thread, so the window keeps responding.
/// (Working out what a file is takes longer than its size suggests: a save well under a
/// megabyte can hold half a million values.)
const BACKGROUND_LOAD: u64 = 256 << 10;

/// How long a file may take to open before the "Opening" box is shown, so that it doesn't
/// flash up for the many files that open at once.
const OPENING_BOX_AFTER: f64 = 0.25;

/// A file being read and decoded on a worker thread.
struct Loading {
    path: PathBuf,
    opts: formats::Options,
    started: f64,
    rx: std::sync::mpsc::Receiver<Result<(usize, Prepared), String>>,
}

/// Saves at least this big are written on a worker thread, so the window keeps responding:
/// a file on disk this size (like `BACKGROUND_LOAD`), or this many values in the Values list
/// (a small packed file can hold a great many).
const BACKGROUND_SAVE: u64 = BACKGROUND_LOAD;
const BACKGROUND_SAVE_ROWS: usize = 20_000;

/// How long a save may take before the "Saving" box is shown.
const SAVING_BOX_AFTER: f64 = OPENING_BOX_AFTER;

/// What to do once a save has gone through (nothing, when it failed: the changes stay).
#[derive(Debug, PartialEq)]
enum AfterSave {
    Stay,
    CloseFile,
    Open(PathBuf),
    Quit,
}

/// What `write_save` wrote: the file read back, its size, a note for the player and the
/// written file's stamp.
struct Written {
    check: formats::Decoded,
    len: usize,
    note: Option<String>,
    stamp: Option<Stamp>,
}

/// A save being written on a worker thread. While it runs nothing on screen takes input, and
/// no other save, open, close or tab switch can start.
struct Saving {
    path: PathBuf,
    then: AfterSave,
    /// `edit_gen` and the tab showing when it started. Anything else when it ends means the
    /// file on screen changed meanwhile (which input being held should rule out).
    gen: (u64, usize),
    started: f64,
    rx: std::sync::mpsc::Receiver<Result<Written, String>>,
}

/// What to tell the player when a file the game checks against the save couldn't be written.
fn companion_failed(e: &crate::backups::CompanionError) -> String {
    if e.stuck.is_empty() {
        return trf("Couldn't write {0}, a file the game checks against the save ({1}). The save and its files were put back as they were, so nothing changed.", &[&e.file.display(), &e.error]);
    }
    let stuck: Vec<String> = e.stuck.iter().map(|p| file_name(p)).collect();
    trf("Couldn't write {0}, a file the game checks against the save ({1}). These files couldn't be put back as they were and may not match each other now: {2}. Their earlier versions are in the backups folder.", &[&e.file.display(), &e.error, &stuck.join(", ")])
}

/// Builds the file for `content`, checks that it reads back as exactly that, keeps a backup of
/// the file on disk and writes the new one. Touches nothing but files, so it runs on the UI
/// thread or a worker thread alike. Every failure says what happened, in the player's language.
fn write_save(path: &Path, layers: &[Layer], content: &Content, opts: &formats::Options) -> Result<Written, String> {
    let bytes = formats::encode(layers, content).map_err(|e| trf("Couldn't save: the new file couldn't be built ({0}). The save on disk wasn't changed.", &[&e]))?;
    // Verify before touching the disk: the new file must decode to exactly what's on screen.
    let check = formats::decode_with(&bytes, opts);
    if !formats::same_layers(&check.layers, layers) || !formats::verified(&check.content, content) {
        return Err(tr("Couldn't save: the new file didn't read back exactly as edited, so nothing was written. The save on disk wasn't changed. Please report this with Help \u{203A} Report a problem.").into());
    }

    let mut note: Option<String> = None;
    if WEB {
        write_in_place(path, &bytes).map_err(|e| trf("Couldn't hand over {0}: {1}", &[&path.display(), &e]))?;
        for (other, data) in formats::companion_files(path, &bytes) {
            let _ = write_in_place(&other, &data);
        }
        note = Some(tr("Your browser downloaded the edited file. Keep a copy of the old save, then put this one in its place.").into());
    } else {
        if path.exists() {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".bak");
            let bak = PathBuf::from(bak);
            if !bak.exists() {
                crate::vfs::copy(path, &bak).map_err(|e| trf("Couldn't save: the backup {0} couldn't be made ({1}). The save on disk wasn't changed.", &[&bak.display(), &e]))?;
                note = Some(trf("Original backed up as {0}.", &[&file_name(&bak)]));
            }
            // Every earlier version stays restorable from Backups \u{203A} Quick restore.
            crate::backups::take(path).map_err(|e| trf("Couldn't save: the current save couldn't be backed up first ({0}). The save on disk wasn't changed.", &[&e]))?;
        }
        // Files the game checks against this save (Persona 4 Golden's .binslot).
        let companions = formats::companion_files(path, &bytes);
        // What to put back if one of them can't be written.
        let before = if companions.is_empty() { None } else { Some(crate::backups::Kept::read(path).map_err(|e| trf("Couldn't save: {0} couldn't be read first ({1}). The save on disk wasn't changed.", &[&path.display(), &e]))?) };
        write_in_place(path, &bytes).map_err(|e| trf("Couldn't write {0}: {1}. The file on disk wasn't changed.", &[&path.display(), &e]))?;
        if let Some(before) = before {
            crate::backups::write_companions(&before, companions, true).map_err(|e| companion_failed(&e))?;
        }
        #[cfg(not(target_arch = "wasm32"))]
        match crate::playerprefs::after_save(path) {
            Ok(Some(m)) => {
                note = Some(match note {
                    Some(n) => format!("{n} {m}"),
                    None => m,
                })
            }
            Err(e) => return Err(trf("Saved the file, but {0}", &[&e])),
            Ok(None) => {}
        }
    }
    Ok(Written { stamp: safety::stamp(path), check, len: bytes.len(), note })
}

/// Whether the Values list built from `new` is the one built from `old`, for the same names.
/// The list (see `values_view::build`) depends on the values only through their shape (keys
/// and their order, array lengths, which values are objects, arrays, null or other), on the
/// short name each object takes from a field (`names::object_summary`) and, for a tree, on
/// which values are editable; so only scalar values may differ.
fn same_values_list(old: &Content, new: &Content) -> bool {
    fn same(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Object(x), Value::Object(y)) => {
                x.len() == y.len()
                    && names::object_summary(a) == names::object_summary(b)
                    && x.iter().zip(y).all(|((ka, va), (kb, vb))| ka == kb && same(va, vb))
            }
            (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(va, vb)| same(va, vb)),
            (Value::Null, Value::Null) => true,
            (Value::Null, _) | (_, Value::Null) | (Value::Object(_) | Value::Array(_), _) | (_, Value::Object(_) | Value::Array(_)) => false,
            _ => true,
        }
    }
    match (old, new) {
        (Content::Json { value: a, .. }, Content::Json { value: b, .. }) => same(a, b),
        (Content::Tree(a), Content::Tree(b)) => a.editable == b.editable && same(&a.view, &b.view),
        _ => false,
    }
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
    /// Taken just before the file was read.
    stamp: Option<Stamp>,
    /// It has Japanese, Chinese or Korean text (the fonts for it are loaded when needed).
    cjk: bool,
}

fn prepare(path: &Path, decoded: formats::Decoded) -> Prepared {
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

/// The "Find my game's saves" dialog: the game list, which game is picked, and the saves found for
/// each game (filled in by a background search).
struct GamePicker {
    query: String,
    picked: Option<usize>,
    /// For entries covering many games (Ren'Py, RPG Maker…): the game whose saves are shown.
    group: Option<String>,
    found: SaveSearch,
    /// "Only games found here": `None` until the player changes it (then on once the search
    /// finds something).
    only_found: Option<bool>,
    /// Only games with this much support.
    level: Option<crate::games::Level>,
    /// Which game each save found for the picked entry belongs to, for (entry, saves found).
    /// Working it out looks through folders, so it is done once rather than on every frame.
    save_groups: Option<((usize, usize), Vec<String>)>,
    /// A remembered save that was picked but is no longer there (offers a new search).
    missing: Option<PathBuf>,
}

/// The saves found for each game in the catalog (`None`: not looked at yet).
type SaveSearch = std::sync::Arc<std::sync::Mutex<Vec<Option<Vec<crate::games::Found>>>>>;

/// Looks for every game's saves on a worker thread. Folders where saves were found before are
/// checked first and are all that's looked at while the last full search is recent; the whole
/// computer is searched on the first start, once a week, and when asked (`rescan`).
fn search_saves() -> SaveSearch {
    let found: SaveSearch = std::sync::Arc::new(std::sync::Mutex::new(vec![None; crate::games::GAMES.len()]));
    spawn_search(&found, false);
    found
}

/// Starts a search that fills `found` in (a new full one when `full`).
fn spawn_search(found: &SaveSearch, full: bool) {
    let out = found.clone();
    std::thread::spawn(move || {
        let mut cache = crate::save_cache::SaveCache::load();
        let quick = !full && cache.fresh();
        if !quick {
            if let Ok(mut v) = out.lock() {
                v.iter_mut().for_each(|x| *x = None);
            }
        }
        for (i, g) in crate::games::GAMES.iter().enumerate() {
            let known = cache.games.get(g.name).cloned().unwrap_or_default();
            // Quick: only the remembered folders. A game with none remembered has no saves here.
            let (f, locs) = if quick { crate::games::find_in(g, &known) } else { crate::games::find_locating(g) };
            if locs.is_empty() {
                cache.games.remove(g.name);
            } else {
                cache.games.insert(g.name.to_string(), locs);
            }
            if let Ok(mut v) = out.lock() {
                v[i] = Some(f);
            }
        }
        if !quick {
            cache.mark_searched();
        }
        cache.save();
    });
}

impl GamePicker {
    fn with(found: SaveSearch) -> Self {
        Self { query: String::new(), picked: None, group: None, found, only_found: None, level: None, save_groups: None, missing: None }
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

/// Tools › Cheat patches: the patch file picked and the codes ticked in it.
struct Patches {
    query: String,
    /// The game's patch files found from the save's name, then search results.
    found: Vec<&'static crate::apollo::db::Entry>,
    /// The patch file shown: a label, its codes and the save's byte order.
    label: String,
    codes: Vec<crate::apollo::Code>,
    on: Vec<bool>,
    picks: Vec<Vec<usize>>,
    big_endian: bool,
    /// The save file's own name (codes say which file they're for).
    file: String,
    /// Show codes for other files too.
    all_files: bool,
    /// The save opened through its patch file's own unpacking codes (and the unpacked stream
    /// shown, if any): those required codes have already run.
    layer: Option<Option<String>>,
}

impl Patches {
    /// Whether code `i` is listed: it's for this file, or every file is being shown. A heading
    /// is listed when something under it is.
    fn listed(&self, i: usize) -> bool {
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

    fn ticked(&self) -> usize {
        (0..self.codes.len()).filter(|&i| self.on[i] && self.listed(i)).count()
    }

    fn show(&mut self, label: String, codes: Vec<crate::apollo::Code>, big_endian: bool) {
        self.on = codes.iter().map(|c| c.default_on && !c.required && c.kind != crate::apollo::Kind::Python).collect();
        self.picks = codes.iter().map(|c| vec![0; c.choices.len()]).collect();
        self.codes = codes;
        self.label = label;
        self.big_endian = big_endian;
    }
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
    /// Tools › Change Steam account: the IDs found in the save, the one picked, and the new one typed.
    rebind: Option<(Vec<u64>, usize, String)>,
    patches: Option<Patches>,
    compare: Option<Compare>,
    save_review: Option<SaveReview>,
    copy_slots: Option<CopySlots>,
    game_picker: Option<GamePicker>,
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
    /// Saving over a file that changed on disk: waiting for Reload / Save anyway / Cancel.
    stale_save: Option<PathBuf>,
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
    help: Option<help::Window>,
    /// Load the Japanese, Chinese and Korean fonts on the next frame.
    need_cjk: bool,
    /// Other open files (see `tabs`) and copied values.
    tabs: tabs::Tabs,
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
        let has_icons = theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, settings.dark);
        cc.egui_ctx.set_zoom_factor(settings.zoom);
        crate::drives::set_include_removable(settings.search_removable);
        let mut app = Self::with(&cc.egui_ctx, settings, has_icons);
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
            rebind: None,
            patches: None,
            compare: None,
            save_review: None,
            copy_slots: None,
            game_picker: None,
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
            help: None,
            // Japanese and Chinese menus need those fonts from the start.
            need_cjk: crate::i18n::current().cjk,
            tabs: tabs::Tabs::new(),
            #[cfg(target_os = "android")]
            keyboard_shown: false,
            #[cfg(target_arch = "wasm32")]
            web_asked: None,
            #[cfg(target_arch = "wasm32")]
            ctx: ctx.clone(),
        }
    }

    /// Find my game's saves: the results of the search made at start (or by "Search again") are
    /// reused, so opening the list never reads the drives again.
    fn picker(&mut self) -> GamePicker {
        let found = match &self.saves_found {
            Some(f) => f.clone(),
            None => {
                let f = search_saves();
                self.saves_found = Some(f.clone());
                f
            }
        };
        GamePicker::with(found)
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
            Ask::SaveAs => self.save_to(&p),
            Ask::Export => self.export_to(&p),
            Ask::Compare => self.compare_with(p),
            Ask::Import => self.import_from(&p),
            Ask::Patch => self.load_patch_file(&p),
            Ask::PresetFile => self.open_preset_file(&p),
            Ask::ExportNames => self.export_names(&p),
            Ask::ImportNames => self.import_names(&p),
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
        if self.quick_restore.is_some() {
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

    /// Opens a file the user picked: in a new tab when another file is open.
    fn request_open(&mut self, path: PathBuf) {
        self.open_in_tab(path);
    }

    /// Opens `path`; a big file is read and decoded on a worker thread while the window
    /// shows that it's opening.
    fn load_async(&mut self, path: &Path) {
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
        std::thread::spawn(move || {
            let stamp = safety::stamp(&p);
            let r = crate::vfs::read(&p)
                .map_err(|e| trf("Couldn't open {0}: {1}", &[&p.display(), &e]))
                .map(|raw| (raw.len(), Prepared { stamp, ..prepare(&p, formats::decode_with(&raw, &o)) }));
            let _ = tx.send(r);
        });
        self.loading = Some(Loading { path: path.to_path_buf(), opts, started: f64::NAN, rx });
        self.info(trf("Opening {0}\u{2026}", &[&file_name(path)]));
    }

    /// The worker thread's result, once it's there.
    fn poll_loading(&mut self, ctx: &egui::Context, now: f64) {
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
                modal(ctx, tr("Opening"), |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(trf("Reading {0}\u{2026}", &[&name]));
                    });
                    if secs > 2.0 {
                        ui.weak(trf("{0} s. Big saves take a moment.", &[&format!("{secs:.0}")]));
                    }
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(_) => {
                self.loading = None;
                self.fail(tr("Opening the file stopped unexpectedly."));
            }
        }
    }

    // ------------------------------------------------------------ loading

    fn load(&mut self, path: &Path) {
        self.load_with(path, formats::Options::for_path(path));
    }

    /// Load with explicit options (password, zip entry).
    fn load_with(&mut self, path: &Path, opts: formats::Options) {
        // The file on screen is being saved: it stays until that is done.
        if self.saving.is_some() {
            return;
        }
        let stamp = safety::stamp(path);
        let raw = match crate::vfs::read(path) {
            Ok(r) => r,
            Err(e) => return self.fail(trf("Couldn't open {0}: {1}", &[&path.display(), &e])),
        };
        let prepared = Prepared { stamp, ..prepare(path, formats::decode_with(&raw, &opts)) };
        self.finish_load(path, opts, raw.len(), prepared);
    }

    /// Shows a decoded file (from `load_with` or the worker thread).
    fn finish_load(&mut self, path: &Path, opts: formats::Options, raw_len: usize, p: Prepared) {
        let Prepared { decoded, baseline, names, note, game, model, running, stamp, cjk } = p;
        self.need_cjk |= cjk || theme::has_cjk(&path.to_string_lossy());
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
            stamp,
        });
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
        self.info(crate::i18n::trf("Opened {0} ({1}) \u{2014} {2}.", &[&file_name(path), &human_size(raw_len), &kind_label(&desc)]));
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
        self.model_from_baseline = false;
    }

    fn locate_data_dir(&mut self) {
        self.ask(Ask::DataDir, None, None);
    }

    fn use_data_dir(&mut self, dir: PathBuf) {
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
        if self.saving.is_some() {
            return Err(tr("the file is being saved").into());
        }
        if let Some(Doc { content: Content::Tree(t), .. }) = &mut self.doc {
            return t.set(ptr, v);
        }
        let Some(target) = self.root_mut().and_then(|r| r.pointer_mut(ptr)) else { return Ok(None) };
        Ok(Some((std::mem::replace(target, v), 0)))
    }

    /// Apply a user edit, recording it for undo. Consecutive edits to the same field are merged.
    fn apply_edit(&mut self, ptr: String, v: Value, now: f64) {
        match self.try_edit(ptr.clone(), v, now) {
            Err(e) => {
                self.fail(trf("Can't use that value: {0}.", &[&e]));
                self.error_ptr = Some(ptr);
            }
            Ok(_) => self.clear_edit_error(&ptr),
        }
    }

    /// A value was accepted: drop the "can't use that value" message that field left behind.
    fn clear_edit_error(&mut self, ptr: &str) {
        if take_if_same(&mut self.error_ptr, ptr) {
            if self.banner.as_deref() == Some(self.status.as_str()) {
                self.banner = None;
            }
            if self.status_kind == StatusKind::Err {
                self.status.clear();
                self.status_kind = StatusKind::Info;
            }
        }
    }

    /// `apply_edit` that hands back why the format refused the value instead of showing it.
    /// `Ok(false)`: there is no such value, or it already held that.
    fn try_edit(&mut self, ptr: String, v: Value, now: f64) -> Result<bool, String> {
        let (before, aliases) = match self.set_value(&ptr, v.clone())? {
            Some(r) => r,
            None => return Ok(false),
        };
        let derived = match &mut self.doc {
            Some(Doc { content: Content::Tree(t), .. }) => t.take_derived(),
            _ => vec![],
        };
        // The format may have adjusted the value (e.g. rounded to a 32-bit float).
        let v = self.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(&ptr)).cloned().unwrap_or(v);
        if before == v && derived.is_empty() {
            return Ok(false);
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
            self.info(trf("Also updated: {0}.", &[&names.join(", ")]));
        } else if aliases > 0 {
            self.info(if aliases == 1 {
                tr("This value is shared with 1 other place in the save, which changed too.").to_string()
            } else {
                trf("This value is shared with {0} other places in the save, which changed too.", &[&aliases])
            });
        }
        Ok(true)
    }

    fn apply_bytes(&mut self, off: usize, bytes: Vec<u8>) {
        if self.saving.is_some() {
            return;
        }
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
        self.touched.insert(ptr);
        match &mut self.trace {
            Some(t) => t.push(ptr.to_string()),
            // An edit made alone (compare, tools...) is no longer part of a button's group.
            None => self.touched.leave_group(ptr),
        }
        if ptr.is_empty() && root(&self.doc.as_ref().unwrap().content).is_some() {
            self.rebuild_model();
        }
        self.hex.label_edits.clear();
        self.sync_raw();
        self.edit_gen += 1;
        // This value differing from the file is enough to know there are unsaved changes; only
        // when it was put back does every edited value have to be looked at again. (Looking at
        // them all after each of the thousands of edits of a bulk edit took seconds.)
        let differs = self.doc.as_ref().is_some_and(|d| match (root(&d.content), root(&d.baseline)) {
            (Some(cv), Some(bv)) => cv.pointer(ptr) != bv.pointer(ptr),
            _ => false,
        });
        if differs {
            self.dirty = true;
        } else if self.trace.is_none() {
            self.recompute_dirty();
        }
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
        if self.saving.is_some() {
            return;
        }
        let Some(change) = (if undo { self.undo.pop() } else { self.redo.pop() }) else { return };
        let ptrs = self.replay(&change, undo);
        if undo { self.redo.push(change) } else { self.undo.push(change) }
        self.last_edit = None;
        for p in &ptrs {
            self.after_change(p);
        }
        self.info(if undo { tr("Undone.") } else { tr("Redone.") });
    }

    /// Puts a change back (`undo`) or makes it again. Returns the values it touched (`""` for
    /// bytes), for `after_change`.
    fn replay(&mut self, change: &Change, undo: bool) -> Vec<String> {
        match change {
            Change::Value { ptr, before, after, extra } => {
                // Undo/redo only restores values that were valid before, so this can't fail.
                let _ = self.set_value(ptr, if undo { before.clone() } else { after.clone() });
                if let Some(Doc { content: Content::Tree(t), .. }) = &mut self.doc {
                    t.take_derived();
                    for (p, b, a) in extra.iter() {
                        t.set_plain(p, if undo { b.clone() } else { a.clone() });
                    }
                }
                let mut out: Vec<String> = extra.iter().map(|(p, _, _)| p.clone()).collect();
                out.push(ptr.clone());
                out
            }
            Change::Bytes { off, before, after } => {
                if let Some(Doc { content: Content::Binary(data), .. }) = &mut self.doc {
                    // The two sides can differ in length (a patch that inserts or deletes bytes).
                    let (cur, src) = if undo { (after, before) } else { (before, after) };
                    let end = (*off + cur.len()).min(data.len());
                    data.splice(*off..end, src.iter().copied());
                }
                vec![String::new()]
            }
            Change::Batch(list) => {
                let mut out = Vec::new();
                if undo {
                    for c in list.iter().rev() {
                        out.extend(self.replay(c, true));
                    }
                } else {
                    for c in list {
                        out.extend(self.replay(c, false));
                    }
                }
                out.dedup();
                out
            }
        }
    }

    /// Applies what one screen action asked for. Several edits from one press (a mass-edit
    /// button) become one undo step and one change group, so Undo and the Changes panel's
    /// Revert put them all back together; a single edit is as before.
    fn apply_edits(&mut self, edits: Vec<(String, Value)>, now: f64) {
        if edits.is_empty() {
            return;
        }
        let many = edits.len() > 1;
        if many {
            self.last_edit = None;
        }
        let start = self.undo.len();
        self.trace = Some(vec![]);
        for (ptr, v) in edits {
            self.apply_edit(ptr, v, now);
        }
        let mut seen = HashSet::new();
        let mut touched: Vec<String> = self.trace.take().unwrap_or_default();
        touched.retain(|p| !p.is_empty() && seen.insert(p.clone()));
        // Looked at once, not after each of possibly thousands of edits.
        self.recompute_dirty();
        if many {
            self.group_undo_from(start);
        }
        if many && touched.len() > 1 {
            self.touched.set_group(&touched);
        } else {
            for p in &touched {
                self.touched.leave_group(p);
            }
        }
    }

    /// Folds the undo steps made since the stack held `start` into one, so a bulk edit or a
    /// preset is undone in one go.
    fn group_undo_from(&mut self, start: usize) {
        if self.undo.len() > start + 1 {
            let list: Vec<Change> = self.undo.drain(start..).collect();
            self.undo.push(Change::Batch(list));
        }
        self.last_edit = None;
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
        if self.saving.is_some() {
            return;
        }
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
        self.ok(tr("All changes reverted."));
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
            self.fail(trf("Couldn't save names to {0}: {1}", &[&path.display(), &e]));
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
        self.save_to_then(path, AfterSave::Stay);
    }

    /// Saves the open file as `path`, then does `then` if that went through.
    fn save_to_then(&mut self, path: &Path, then: AfterSave) {
        let background = self.save_in_background();
        self.save_with(path, then, background);
    }

    /// Big files are saved on a worker thread (see `BACKGROUND_SAVE`).
    fn save_in_background(&self) -> bool {
        let Some(d) = &self.doc else { return false };
        let size = match &d.content {
            Content::Binary(b) => b.len() as u64,
            Content::Text(t) => t.len() as u64,
            _ => 0,
        };
        // The browser version has no worker threads.
        !WEB && (size.max(crate::vfs::len(&d.path).unwrap_or(0)) >= BACKGROUND_SAVE || self.model.rows.len() >= BACKGROUND_SAVE_ROWS)
    }

    /// Checks that the file can be saved, then writes it here or on a worker thread. Either way
    /// the same `write_save` does the work and `finish_save` takes in its outcome.
    fn save_with(&mut self, path: &Path, then: AfterSave, background: bool) {
        // A file being opened (or read again) is not the one on screen yet: saving now would
        // write the old one and leave the new one with the wrong idea of what is on disk.
        if self.saving.is_some() || self.loading.is_some() {
            return;
        }
        if !self.commit_raw() {
            return self.fail(tr("Not saved: the Raw JSON text has an error. Fix it on the Raw JSON tab, then save."));
        }
        let Some(doc) = &self.doc else { return };
        // Something else wrote the file since it was read: don't silently put older data over it.
        if !std::mem::take(&mut self.stale_ok) && doc.path == path && safety::changed(&doc.path, doc.stamp.as_ref()) {
            self.stale_save = Some(path.to_path_buf());
            return;
        }
        if background && !WEB {
            // The worker gets its own copy: what is on screen can't change under it.
            let (p, layers, content, opts) = (path.to_path_buf(), doc.layers.clone(), doc.content.clone(), doc.opts.clone());
            return self.spawn_save(path, then, move || write_save(&p, &layers, &content, &opts));
        }
        let r = write_save(path, &doc.layers, &doc.content, &doc.opts);
        self.finish_save(path, r, false);
        self.after_save(then);
    }

    /// Runs `work` on a worker thread; `poll_saving` takes in what it hands back.
    fn spawn_save(&mut self, path: &Path, then: AfterSave, work: impl FnOnce() -> Result<Written, String> + Send + 'static) {
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new().name("save".into()).spawn(move || {
            let _ = tx.send(work());
        });
        if spawned.is_err() {
            // No thread to be had: save here instead.
            return self.save_with(path, then, false);
        }
        self.saving = Some(Saving { path: path.to_path_buf(), then, gen: (self.edit_gen, self.tabs.active), started: f64::NAN, rx });
        self.info(trf("Saving {0}\u{2026}", &[&file_name(path)]));
    }

    /// The worker thread's outcome, once it's there. Until then the window shows that it is
    /// saving and takes no input.
    fn poll_saving(&mut self, ctx: &egui::Context, now: f64) {
        let Some(s) = &mut self.saving else { return };
        if s.started.is_nan() {
            s.started = now;
            // Nothing typed from here on may land in the file being saved.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        match s.rx.try_recv() {
            Ok(r) => {
                let s = self.saving.take().unwrap();
                let meanwhile = s.gen != (self.edit_gen, self.tabs.active);
                self.finish_save(&s.path, r, meanwhile);
                self.after_save(s.then);
                self.closing_after_save();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                let name = file_name(&s.path);
                let secs = now - s.started;
                // Clicks must not reach the file being saved, before the box and under it.
                let screen = ctx.screen_rect();
                egui::Area::new(egui::Id::new("saving_hold")).fixed_pos(screen.min).sense(widgets::BLOCK_CLICKS).order(egui::Order::Foreground).show(ctx, |ui| {
                    ui.allocate_response(screen.size(), widgets::BLOCK_CLICKS);
                });
                if secs < SAVING_BOX_AFTER {
                    ctx.request_repaint_after(std::time::Duration::from_millis(30));
                    return;
                }
                modal(ctx, tr("Saving the file"), |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(trf("Writing {0}\u{2026}", &[&name]));
                    });
                    if secs > 2.0 {
                        ui.weak(trf("{0} s. Big saves take a moment.", &[&format!("{secs:.0}")]));
                    }
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(_) => {
                // The worker stopped without an answer (it panicked). Whether or not it got as far
                // as writing, the changes stay unsaved, and the next save asks first if the file
                // on disk is no longer the one opened.
                self.saving = None;
                self.fail(tr("Saving stopped unexpectedly. Your changes are still here but not saved. Save again."));
                self.closing_after_save();
            }
        }
    }

    /// The window was asked to close during a save: close now, or ask about unsaved changes.
    fn closing_after_save(&mut self) {
        if !std::mem::take(&mut self.close_after_save) {
            return;
        }
        if self.any_dirty() && !self.allow_close {
            self.show_first_dirty();
            self.confirm_close = true;
        } else {
            self.allow_close = true;
            self.close_now = true;
        }
    }

    /// Takes in a save's outcome, on the UI thread. `meanwhile`: the file on screen changed
    /// after the save started, so what was written is not what is shown.
    fn finish_save(&mut self, path: &Path, r: Result<Written, String>, meanwhile: bool) {
        let w = match r {
            Ok(w) => w,
            Err(e) => return self.fail(e),
        };
        let Written { check, len, note, stamp } = w;
        let Some(d) = self.doc.as_mut() else { return };
        d.path = path.to_path_buf();
        d.stamp = stamp;
        self.disk_changed = false;
        self.disk_seen = None;
        self.settings.push_recent(path);
        if meanwhile {
            // The file holds what was shown when the save started; what is shown now stays as
            // it is and counts as unsaved.
            self.dirty = true;
            return self.fail(tr("Saved, but changes made while saving aren't in the file yet. Save again to keep them."));
        }
        let signed = check.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. }));
        let persistent = check.layers.iter().any(|l| matches!(l, Layer::RenpyPersistent { .. }));
        // The Values list is built again only when it could come out differently.
        let keep_model = self.model_from_baseline && same_values_list(&d.baseline, &check.content);
        d.layers = check.layers;
        d.content = check.content;
        d.baseline = d.content.clone();
        self.touched.clear();
        if !keep_model {
            self.rebuild_model();
        }
        self.model_from_baseline = true;
        self.edit_gen += 1;
        self.recompute_dirty();
        let extra = if signed && crate::renpy_sign::can_sign() {
            Some(tr("Re-signed with your Ren'Py key, so the game will load it normally."))
        } else if signed {
            Some(tr("Your Ren'Py key wasn't found, so the old signature was removed. The game asks once whether to load the save: choose Yes."))
        } else if persistent {
            Some(tr("Re-signed with your Ren'Py key. If the game is running, quit it without saving first, because it rewrites this file when it quits."))
        } else {
            None
        };
        let cloud = in_steam_cloud(path).then(|| tr("If Steam asks which save to keep, choose the local one."));
        let mut msg = vec![trf("Saved and verified ({0}).", &[&human_size(len)])];
        msg.extend(note);
        msg.extend(extra.map(String::from));
        msg.extend(cloud.map(String::from));
        self.ok(msg.join(" "));
    }

    /// After a save: close the file, open the next one or quit, if it went through.
    fn after_save(&mut self, then: AfterSave) {
        if self.dirty {
            return;
        }
        match then {
            AfterSave::Stay => {}
            AfterSave::CloseFile => self.close_file(),
            AfterSave::Open(p) => self.load_async(&p),
            AfterSave::Quit => {
                if self.any_dirty() {
                    // Another tab still has unsaved changes: show it and ask again.
                    self.show_first_dirty();
                    self.confirm_close = true;
                } else {
                    self.allow_close = true;
                    self.close_now = true;
                }
            }
        }
    }

    fn save_then(&mut self, then: AfterSave) {
        if let Some(p) = self.doc.as_ref().map(|d| d.path.clone()) {
            self.save_to_then(&p, then);
        }
    }

    /// Save from the button / Ctrl+S: first show what changed and warn if the game is open.
    fn save_clicked(&mut self) {
        // Not while a save is being written, nor while the file is being read again.
        if self.saving.is_some() || self.loading.is_some() {
            return;
        }
        let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        if !self.commit_raw() {
            return self.fail(tr("Not saved: the Raw JSON text has an error. Fix it on the Raw JSON tab, then save."));
        }
        let running = crate::running::game_for(&path);
        if running.is_none() && !self.settings.confirm_save {
            return self.save_to(&path);
        }
        self.running = running.clone();
        self.save_review = Some(SaveReview { path, running, lines: self.change_lines() });
    }

    /// The unsaved changes in words: "Inventory \u{203A} Scrap Iron: 12 -> 99".
    fn change_lines(&self) -> Vec<String> {
        let Some(d) = &self.doc else { return vec![] };
        match (&d.content, &d.baseline) {
            (Content::Binary(a), Content::Binary(b)) => byte_ranges(a, b)
                .into_iter()
                .map(|(s, e)| {
                    let name = self.hex.labels.get(&s).map(|l| l.name.clone()).unwrap_or(format!("Offset 0x{s:X}"));
                    if e - s == 1 { trf("{0}: 1 byte changed", &[&name]) } else { trf("{0}: {1} bytes changed", &[&name, &(e - s)]) }
                })
                .collect(),
            (Content::Text(a), Content::Text(b)) => {
                if a != b {
                    vec![tr("The text was edited").into()]
                } else {
                    vec![]
                }
            }
            (c, b) => {
                let (Some(cv), Some(bv)) = (root(c), root(b)) else { return vec![] };
                let noted: std::cell::RefCell<HashSet<&str>> = Default::default();
                self.touched
                    .iter()
                    .filter(|p| cv.pointer(p) != bv.pointer(p))
                    .map(|p| {
                        if p.is_empty() {
                            return tr("Whole document (Raw JSON edit)").to_string();
                        }
                        let (name, context) = match self.model.by_ptr.get(p.as_str()) {
                            Some(&i) => (self.model.rows[i].name().to_string(), self.model.rows[i].context().to_string()),
                            None => (p.clone(), String::new()),
                        };
                        let old = bv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                        let new = cv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                        let at = if context.is_empty() { name } else { format!("{context} \u{203A} {name}") };
                        let kind = match &d.content {
                            Content::Tree(t) => t.kind_name.as_str(),
                            _ => "JSON",
                        };
                        // Each note once, on the first value it applies to.
                        match crate::risks::note(kind, p).filter(|n| noted.borrow_mut().insert(*n)) {
                            Some(note) => format!("{at}: {old} -> {new}  (note: {note})"),
                            None => format!("{at}: {old} -> {new}"),
                        }
                    })
                    .collect()
            }
        }
    }

    fn game_picker_dialog(&mut self, ctx: &egui::Context) {
        let Some(gp) = &mut self.game_picker else { return };
        let (open, close) = game_picker_window(ctx, gp);
        if close {
            self.game_picker = None;
        }
        if let Some(p) = open {
            if crate::vfs::exists(&p) {
                self.game_picker = None;
                self.request_open(p);
            } else if let Some(gp) = &mut self.game_picker {
                // The remembered folder is out of date: say so and offer a new search.
                gp.missing = Some(p);
            }
        }
    }

    fn copy_slots_dialog(&mut self, ctx: &egui::Context) {
        let Some(c) = &mut self.copy_slots else { return };
        let label = |i: usize, s: &crate::fmt::slot_copy::Slot| {
            if s.active {
                trf("Slot {0}: {1} (level {2})", &[&(i + 1), &s.name, &s.level])
            } else {
                trf("Slot {0}: empty", &[&(i + 1)])
            }
        };
        let mut choice = None;
        modal(ctx, tr("Copy character"), |ui| {
            egui::Grid::new("copy_slots").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(tr("Copy"));
                egui::ComboBox::from_id_salt("copy_from").width(260.0).selected_text(label(c.from, &c.slots[c.from])).show_ui(ui, |ui| {
                    for (i, s) in c.slots.iter().enumerate().filter(|(_, s)| s.active) {
                        ui.selectable_value(&mut c.from, i, label(i, s));
                    }
                });
                ui.end_row();
                ui.label(tr("Into"));
                egui::ComboBox::from_id_salt("copy_to").width(260.0).selected_text(label(c.to, &c.slots[c.to])).show_ui(ui, |ui| {
                    for (i, s) in c.slots.iter().enumerate() {
                        ui.selectable_value(&mut c.to, i, label(i, s));
                    }
                });
                ui.end_row();
            });
            let dest = &c.slots[c.to];
            if c.to == c.from {
                ui.colored_label(theme::fg(ui, theme::WARN), tr("Pick a different slot to copy into."));
            } else if dest.active {
                ui.colored_label(theme::fg(ui, theme::WARN), trf("This replaces {0} (level {1}) in slot {2}.", &[&dest.name, &dest.level, &(c.to + 1)]));
            }
            ui.weak(tr("The file is backed up first (Backups \u{203A} Quick restore)."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = c.to != c.from && c.slots[c.from].active;
                if ui.add_enabled(ok, egui::Button::new(RichText::new(tr("Copy character")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
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
            return self.fail(trf("{0} looks like it's running. Quit it first, or it will overwrite the copied character. Nothing was copied.", &[&game]));
        }
        let raw = match crate::vfs::read(&path) {
            Ok(r) => r,
            Err(e) => return self.fail(trf("Couldn't read {0}: {1}", &[&path.display(), &e])),
        };
        let out = match crate::fmt::slot_copy::er_copy(&raw, from, to) {
            Ok(o) => o,
            Err(e) => return self.fail(trf("Couldn't copy the character: {0}. The save wasn't changed.", &[&e])),
        };
        if !WEB {
            if let Err(e) = crate::backups::take(&path) {
                return self.fail(trf("Couldn't back up the file first: {0}. Nothing was changed.", &[&e]));
            }
        }
        if let Err(e) = write_in_place(&path, &out) {
            // In the browser the download may already have happened.
            return self.fail(if WEB { trf("Couldn't write {0}: {1}", &[&path.display(), &e]) } else { trf("Couldn't write {0}: {1}. The file on disk wasn't changed.", &[&path.display(), &e]) });
        }
        self.load(&path);
        self.ok(trf("Copied {0} (level {1}) into slot {2}. It shows up on the game's Load Game screen.", &[&who.name, &who.level, &(to + 1)]));
    }

    fn save_review_dialog(&mut self, ctx: &egui::Context) {
        let Some(r) = &self.save_review else { return };
        let mut choice = None;
        let mut dont_ask = !self.settings.confirm_save;
        modal(ctx, tr("Save changes?"), |ui| {
            ui.set_max_width(520.0);
            if let Some(game) = &r.running {
                theme::card(ui, |ui| {
                    ui.label(RichText::new(trf("{0} looks like it's running.", &[game])).color(theme::fg(ui, theme::WARN)).strong());
                    ui.label(tr("Games keep the save in memory and write it back when they autosave or quit, which would undo these changes. Quit the game completely (not just to the title screen), then save.",
                    ));
                });
                ui.add_space(6.0);
            }
            ui.label(if r.lines.len() == 1 { trf("{0} change to {1}:", &[&r.lines.len(), &file_name(&r.path)]) } else { trf("{0} changes to {1}:", &[&r.lines.len(), &file_name(&r.path)]) });
            ScrollArea::vertical().show_list_fit(ui, "app_list1", 280.0, |ui| {
                for l in &r.lines {
                    ui.label(format!("\u{2022} {l}"));
                }
                if r.lines.is_empty() {
                    ui.weak(tr("(nothing that the list can describe)"));
                }
            });
            ui.add_space(4.0);
            if in_steam_cloud(&r.path) {
                ui.weak(tr("This save is in a Steam Cloud folder. If Steam asks which save to keep, choose the local one."));
            }
            ui.weak(if WEB {
                tr("Your browser downloads the edited file. Keep a copy of the original before you replace it.")
            } else {
                tr("The save on disk now is backed up first. Backups \u{203A} Quick restore brings it back.")
            });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let label = if r.running.is_some() { tr("Save anyway") } else { tr("Save") };
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
        if self.saving.is_some() {
            return;
        }
        let Some(doc) = &self.doc else { return };
        let name = file_name(&doc.path);
        self.ask(Ask::SaveAs, None, Some(name));
    }

    /// Backups › Quick restore picked `from`: asks "Restore this backup?" first. With unsaved
    /// changes it is refused straight away, as the restore itself would be.
    fn ask_restore(&mut self, from: PathBuf, what: String) {
        let Some(save) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
        if self.dirty {
            return self.fail(tr("You have unsaved changes. Save or revert them first, then restore."));
        }
        self.quick_restore = Some((save, from, what));
    }

    /// Put `from` (a backup) back in place of `save`, the open file. The file as it is now is
    /// backed up first, so a restore can itself be undone. Does nothing when another file is
    /// open by now.
    fn restore_from(&mut self, save: &Path, from: &Path, what: &str) {
        if self.doc.as_ref().map(|d| d.path.as_path()) != Some(save) {
            return;
        }
        if self.dirty {
            return self.fail(tr("You have unsaved changes. Save or revert them first, then restore."));
        }
        self.restore_to(save, from, what);
    }

    /// Put `from` back as `path` (which may be missing, after Back up and start a new game)
    /// and open it.
    fn restore_to(&mut self, path: &Path, from: &Path, what: &str) {
        let path = path.to_path_buf();
        if self.dirty && self.doc.as_ref().is_some_and(|d| d.path != path) {
            return self.fail(trf("Save or revert the changes to {0} first.", &[&self.doc.as_ref().map(|d| file_name(&d.path)).unwrap_or_default()]));
        }
        if let Some(game) = crate::running::game_for(&path) {
            return self.fail(trf("{0} looks like it's running. Quit it first, or it may overwrite the restored save. Nothing was restored.", &[&game]));
        }
        // What to put back if a file the game checks against the save can't be updated.
        let before = match crate::backups::Kept::read(&path) {
            Ok(k) => k,
            Err(e) => return self.fail(trf("Couldn't restore the backup: {0}. The save on disk wasn't changed.", &[&trf("couldn't read {0}: {1}", &[&path.display(), &e])])),
        };
        match crate::backups::restore(&path, from) {
            Ok(()) => {
                let companions = crate::vfs::read(&path).map(|bytes| formats::companion_files(&path, &bytes)).unwrap_or_default();
                if let Err(e) = crate::backups::write_companions(&before, companions, true) {
                    if !e.stuck.is_empty() {
                        // Show what is on disk now.
                        self.dirty = false;
                        self.load(&path);
                    }
                    return self.fail(trf("Couldn't restore the backup. {0}", &[&companion_failed(&e)]));
                }
                self.dirty = false;
                self.load(&path);
                self.ok(trf("Restored {0}. The save it replaced was backed up too, so you can switch back.", &[&what]));
            }
            Err(e) => self.fail(trf("Couldn't restore the backup: {0}. The save on disk wasn't changed.", &[&e])),
        }
    }

    fn export_decoded(&mut self) {
        if !self.commit_raw() {
            return self.fail(tr("The Raw JSON has a syntax error."));
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
            return self.fail(tr("Save or revert your changes before importing."));
        }
        let Some(doc) = &self.doc else { return };
        let payload = match crate::vfs::read(src) {
            Ok(b) => b,
            Err(e) => return self.fail(trf("Couldn't read {0}: {1}", &[&src.display(), &e])),
        };
        let bytes = match formats::encode(&doc.layers, &Content::Binary(payload)) {
            Ok(b) => b,
            Err(e) => return self.fail(trf("Couldn't import: the data couldn't be packed into this save ({0}). Nothing was changed.", &[&e])),
        };
        let check = formats::decode_with(&bytes, &doc.opts);
        if !formats::same_layers(&check.layers, &doc.layers) || std::mem::discriminant(&check.content) != std::mem::discriminant(&doc.content) {
            return self.fail(tr("Couldn't import: the data doesn't fit this kind of save (wrong file, or saved in another format). Nothing was changed."));
        }
        let d = self.doc.as_mut().unwrap();
        d.content = check.content;
        // The whole document counts as edited, so the import shows as an unsaved change.
        self.undo.clear();
        self.redo.clear();
        self.last_edit = None;
        self.after_change("");
        self.recompute_dirty();
        self.ok(trf("Imported {0}. Review the changes, then save.", &[&file_name(src)]));
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
        let result = result.and_then(|b| {
            // Whatever is there (perhaps a save) is kept first, like any file the app replaces.
            crate::backups::take(dest).map_err(|e| trf("{0} is already there and couldn't be backed up first ({1}), so it wasn't replaced.", &[&file_name(dest), &e]))?;
            write_in_place(dest, &b)
        });
        match result {
            Ok(()) => self.ok(trf("Exported to {0}", &[&dest.display()])),
            Err(e) => self.fail(trf("Couldn't export: {0}", &[&e])),
        }
    }

    fn view_as_hex(&mut self) {
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

    fn refine_from_disk(&mut self) {
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

    // ------------------------------------------------------------ UI: chrome

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| self.menus(ui));
    }

    /// The menus, in the menu bar or (on a phone) under the toolbar's menu button.
    fn menus(&mut self, ui: &mut egui::Ui) {
        {
            let has = self.doc.is_some();
            ui.menu_button(tr("File"), |ui| {
                if ui.add(egui::Button::new(tr("Open\u{2026}")).shortcut_text("Ctrl+O")).clicked() {
                    ui.close_menu();
                    self.ask(Ask::Open, None, None);
                }
                if !cfg!(target_os = "android") && !WEB && ui.button(tr("Find my game's saves\u{2026}")).on_hover_text(tr("Pick a game to see the saves it has on this computer")).clicked() {
                    ui.close_menu();
                    self.game_picker = Some(self.picker());
                }
                ui.menu_button(tr("Open recent"), |ui| {
                    let recent = self.settings.recent.clone();
                    if recent.is_empty() {
                        ui.weak(tr("Nothing yet"));
                    }
                    for p in recent {
                        if ui.add_enabled(crate::vfs::exists(&p), egui::Button::new(recent_label(&p))).on_hover_text(p.display().to_string()).clicked() {
                            ui.close_menu();
                            self.request_open(p);
                        }
                    }
                });
                ui.separator();
                if ui.add_enabled(has, egui::Button::new(tr("Save")).shortcut_text("Ctrl+S")).clicked() {
                    ui.close_menu();
                    self.save_clicked();
                }
                if ui.add_enabled(has, egui::Button::new(tr("Save a copy as\u{2026}"))).clicked() {
                    ui.close_menu();
                    self.save_as();
                }
                let on_disk_file = self.doc.as_ref().is_some_and(|d| crate::vfs::exists(&d.path));
                if ui
                    .add_enabled(on_disk_file, egui::Button::new(tr("Reload from disk")))
                    .on_hover_text(tr("Reads the file again, for when the game or another program changed it. Unsaved changes are dropped (you are asked first)."))
                    .clicked()
                {
                    ui.close_menu();
                    if let Some(p) = self.doc.as_ref().map(|d| d.path.clone()) {
                        self.request_open(p);
                    }
                }
                if ui.add_enabled(has, egui::Button::new(tr("Close save")).shortcut_text("Ctrl+W")).clicked() {
                    ui.close_menu();
                    self.close_clicked();
                }
                ui.separator();
                if ui.add_enabled(self.dirty, egui::Button::new(tr("Revert all changes"))).clicked() {
                    ui.close_menu();
                    self.revert_all();
                }
                ui.separator();
                if ui.add_enabled(has, egui::Button::new(tr("Export decoded data\u{2026}"))).clicked() {
                    ui.close_menu();
                    self.export_decoded();
                }
                let importable = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_) | Content::Text(_) | Content::Json { .. }));
                if ui
                    .add_enabled(importable, egui::Button::new(tr("Import edited data\u{2026}")))
                    .on_hover_text(tr("Bring back data you exported and edited in another program; it is packed the way this save is packed when you save."))
                    .clicked()
                {
                    ui.close_menu();
                    self.ask(Ask::Import, None, None);
                }
            });
            ui.menu_button(tr("Backups"), |ui| {
                let on_disk = !WEB && self.doc.as_ref().is_some_and(|d| d.path.is_file());
                if !WEB && ui.add_enabled(on_disk, egui::Button::new(tr("Back up now\u{2026}"))).on_hover_text(tr("Keeps a named backup of the save file as it is on disk now, until you delete it. The save itself is not changed.")).clicked() {
                    ui.close_menu();
                    self.backup_name = Some(String::new());
                }
                if !WEB && ui.add_enabled(on_disk, egui::Button::new(tr("Back up and start a new game\u{2026}"))).on_hover_text(tr("Keeps a named backup, then removes the save file so the game starts a new game. Restore the backup any time to go back.")).clicked() {
                    ui.close_menu();
                    self.new_game_name = Some(String::new());
                }
                if !WEB && ui.add_enabled(has, egui::Button::new(tr("Save manager\u{2026}"))).on_hover_text(tr("All backups of this save in one window: restore, compare, delete, name. Restoring backs up the current save first.")).clicked() {
                    ui.close_menu();
                    self.open_save_manager();
                }
                ui.separator();
                let bak = self.doc.as_ref().map(|d| {
                    let mut b = d.path.as_os_str().to_owned();
                    b.push(".bak");
                    PathBuf::from(b)
                });
                let versions = self.doc.as_ref().map(|d| crate::backups::list(&d.path)).unwrap_or_default();
                let has_bak = bak.as_ref().is_some_and(|b| b.exists());
                let on_disk = !WEB && self.doc.as_ref().is_some_and(|d| d.path.is_file());
                ui.add_enabled_ui(has_bak || !versions.is_empty() || on_disk, |ui| {
                    ui.menu_button(tr("Quick restore"), |ui| {
                        ui.weak(tr("Automatic backups are taken each time this app saves over the file."));
                        ui.weak(trf("Named backups are kept until you delete them. Only the {0} newest automatic ones are kept.", &[&crate::backups::keep()]));
                        ui.separator();
                        let mut pick = None;
                        let mut compare = None;
                        let mut unlock = None;
                        // Inside a pop-up menu: shrink to the rows, the menu sets the width.
                        ScrollArea::vertical().auto_shrink([true, true]).max_height(crate::widgets::list_height(ui, 360.0)).show_locked(ui, |ui| {
                            for (i, v) in versions.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    let when = crate::backups::describe(v.when);
                                    let label = match &v.name {
                                        Some(n) => format!("\u{201C}{n}\u{201D} \u{00B7} {when} ({})", human_size(v.size as usize)),
                                        None => format!("{when} ({})", human_size(v.size as usize)),
                                    };
                                    if ui.button(label).on_hover_text(tr("Replaces the current save with this backup. The current save is backed up first, so you can undo it.")).clicked() {
                                        pick = Some((v.path.clone(), trf("the backup from {0}", &[&when])));
                                    }
                                    if ui.small_button(tr("Compare")).on_hover_text(tr("Shows what differs between this backup and the open save")).clicked() {
                                        compare = Some(v.path.clone());
                                    }
                                    if v.name.is_some() && ui.small_button(tr("Make automatic")).on_hover_text(tr("Stops keeping this backup for good; it is cleared out in time like the automatic ones")).clicked() {
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
                                self.fail(trf("Couldn't change that backup: {0}", &[&e]));
                            }
                        }
                        if has_bak {
                            ui.separator();
                            if ui.button(tr("Original save (.bak)")).on_hover_text(tr("The original save, from before this app first saved over it")).clicked() {
                                pick = bak.clone().map(|b| (b, tr("the original save").to_string()));
                            }
                        }
                        if let Some(dir) = self.doc.as_ref().and_then(|d| crate::backups::folder(&d.path)) {
                            if dir.is_dir() {
                                if let Some(label) = reveal_text(true) {
                                    ui.separator();
                                    if ui.button(label).clicked() {
                                        ui.close_menu();
                                        platform::reveal(&dir);
                                    }
                                }
                            }
                        }
                        if let Some((p, what)) = pick {
                            ui.close_menu();
                            self.ask_restore(p, what);
                        }
                    });
                });
            });
            ui.menu_button(tr("Edit"), |ui| {
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new(tr("Undo")).shortcut_text("Ctrl+Z")).clicked() {
                    ui.close_menu();
                    self.undo_redo(true);
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new(tr("Redo")).shortcut_text("Ctrl+Y")).clicked() {
                    ui.close_menu();
                    self.undo_redo(false);
                }
                ui.separator();
                if ui.add_enabled(has, egui::Button::new(tr("Find\u{2026}")).shortcut_text("Ctrl+F")).clicked() {
                    ui.close_menu();
                    self.focus_search = true;
                    self.focus_game_find = self.tab == Tab::Game;
                }
                ui.separator();
                if ui.add_enabled(self.dirty, egui::Button::new(tr("Copy changes as text"))).on_hover_text(tr("The list of changes, to paste when asking for help or sharing what you did")).clicked() {
                    ui.close_menu();
                    let ctx = ui.ctx().clone();
                    self.copy_changes(&ctx);
                }
                ui.separator();
                if ui.button(tr("Preferences\u{2026}")).clicked() {
                    ui.close_menu();
                    self.prefs_open = true;
                }
            });
            ui.menu_button(tr("View"), |ui| {
                let mut changed = false;
                changed |= crate::widgets::checkbox(ui, &mut self.settings.show_advanced, tr("Show advanced values")).changed();
                changed |= crate::widgets::checkbox(ui, &mut self.settings.show_raw_keys, tr("Show raw key names")).changed();
                crate::widgets::checkbox(ui, &mut self.show_changes, tr("Changes panel"));
                if ui.add_enabled(self.doc.is_some() && !self.doc_note.is_empty() && self.note_hidden(), egui::Button::new(tr("Show the note about this file"))).clicked() {
                    let key = Settings::note_key(&self.doc_note);
                    self.settings.hidden_notes.retain(|k| *k != key);
                    changed = true;
                }
                changed |= crate::widgets::checkbox(ui, &mut self.settings.confirm_save, tr("Review changes before saving"))
                    .on_hover_text(tr("Lists what changed and checks whether the game is running each time you save."))
                    .changed();
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
                ui.weak(tr("Zoom: Ctrl + / Ctrl \u{2212} / Ctrl 0"));
                ui.separator();
                let structured = self.doc.as_ref().is_some_and(|d| !matches!(d.content, Content::Binary(_)));
                if ui.add_enabled(structured, egui::Button::new(tr("Show decoded data as hex"))).clicked() {
                    ui.close_menu();
                    self.view_as_hex();
                }
                if changed {
                    self.settings.save();
                }
            });
            ui.menu_button(tr("Tools"), |ui| {
                let is_rpg = matches!(&self.doc, Some(Doc { content: Content::Json { value, .. }, .. }) if names::is_rpgmaker(value));
                if !WEB && ui.add_enabled(is_rpg, egui::Button::new(tr("Locate game data folder (RPG Maker names)\u{2026}"))).on_disabled_hover_text(tr("Only for RPG Maker saves.")).clicked() {
                    ui.close_menu();
                    self.locate_data_dir();
                }
                let has = self.doc.is_some();
                if ui
                    .add_enabled(has, egui::Button::new(tr("Decrypt with password\u{2026}")))
                    .on_disabled_hover_text(tr("Open a save first."))
                    .on_hover_text(tr("For encrypted saves (Easy Save 3, Godot). Easy Save's default password is tried automatically."))
                    .clicked()
                {
                    ui.close_menu();
                    self.password_prompt = Some(self.doc.as_ref().and_then(|d| d.opts.password.clone()).unwrap_or_default());
                }
                if ui
                    .add_enabled(has, egui::Button::new(tr("Change Steam account in this save\u{2026}")))
                    .on_disabled_hover_text(tr("Open a save first."))
                    .on_hover_text(tr("Finds the Steam IDs stored in the save and changes them to another account's, so a save copied from a friend or another account loads."))
                    .clicked()
                {
                    ui.close_menu();
                    self.start_rebind();
                }
                let binary = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_)));
                if ui
                    .add_enabled(binary, egui::Button::new(tr("Cheat patches\u{2026}")))
                    .on_disabled_hover_text(tr("Only for saves opened in the hex view, which means a format the editor doesn't understand yet."))
                    .on_hover_text(tr("Ready-made cheats for console saves (PlayStation 1\u{2013}4, PSP, Vita, Dreamcast) from the Apollo patch collection."))
                    .clicked()
                {
                    ui.close_menu();
                    self.start_patches();
                }
                if ui
                    .add_enabled(binary, egui::Button::new(tr("Find checksum\u{2026}")))
                    .on_disabled_hover_text(tr("Only for saves opened in the hex view, which means a format the editor doesn't understand yet."))
                    .on_hover_text(tr("Looks for a checksum in a save the editor doesn't know (CRC, Adler, sums, MD5, SHA) and keeps it right whenever you save."))
                    .clicked()
                {
                    ui.close_menu();
                    self.find_sums();
                }
                #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
                if ui
                    .button(tr("Templates folder\u{2026}"))
                    .on_hover_text(tr("Your own templates for games the editor doesn't know: a JSON file per game that says where each value is. The folder has a guide with an example."))
                    .clicked()
                {
                    ui.close_menu();
                    match crate::fmt::template::ensure_dir() {
                        Ok(d) => {
                            crate::platform::reveal(&d.join("README.txt"));
                            self.ok(trf("Templates folder: {0}", &[&d.display()]));
                        }
                        Err(e) => self.fail(trf("Couldn't make the templates folder: {0}", &[&e])),
                    }
                }
                ui.menu_button(tr("Presets"), |ui| {
                    if ui
                        .add_enabled(self.dirty, egui::Button::new(tr("Save changes as a preset\u{2026}")))
                        .on_hover_text(tr("Keep the changes you made, to make them again on another save or slot"))
                        .clicked()
                    {
                        ui.close_menu();
                        self.start_preset_save();
                    }
                    if ui.add_enabled(has, egui::Button::new(tr("Apply a preset\u{2026}"))).on_hover_text(tr("Make a saved set of changes on this save")).clicked() {
                        ui.close_menu();
                        self.start_preset_apply();
                    }
                });
                if ui
                    .add_enabled(has, egui::Button::new(tr("Compare with another save\u{2026}")))
                    .on_disabled_hover_text(tr("Open a save first."))
                    .on_hover_text(tr("List every value that differs from another save. Handy for finding which value is your gold or HP."))
                    .clicked()
                {
                    ui.close_menu();
                    self.start_compare();
                }
                let entries = self.doc.as_ref().map(|d| d.zip_entries.clone()).unwrap_or_default();
                if entries.len() > 1 {
                    ui.menu_button(tr("Open another file in this archive"), |ui| {
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
                        .button(tr("Copy character to another slot\u{2026}"))
                        .on_hover_text(tr("Duplicate an Elden Ring character, e.g. to try something out on the copy"))
                        .clicked()
                    {
                        ui.close_menu();
                        if self.dirty {
                            self.fail(tr("Save or revert your changes first. Copying works on the file as it is on disk."));
                        } else {
                            let to = slots.iter().position(|s| !s.active).unwrap_or(1);
                            self.copy_slots = Some(CopySlots { slots, from: 0, to });
                        }
                    }
                }
                ui.separator();
                if let (Some(d), Some(label)) = (&self.doc, reveal_text(false)) {
                    if ui.button(label).clicked() {
                        ui.close_menu();
                        platform::reveal(&d.path);
                    }
                }
            });
            self.help_menu(ui);
        }
    }

    /// Re-read the open file with changed decode options (password, zip entry).
    fn reopen_with(&mut self, change: impl FnOnce(&mut formats::Options)) {
        if self.dirty {
            return self.fail(tr("Save or revert your changes first."));
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
            Err(e) => return self.fail(trf("Couldn't open {0}: {1}", &[&other.display(), &e])),
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
                _ => return self.fail(tr("Those two files aren't the same kind of save, so they can't be compared.")),
            },
        };
        let n = diffs.len();
        self.compare = Some(Compare { other, diffs });
        self.info(if n == 1 { tr("1 difference found.").to_string() } else { trf("{0} differences found.", &[&n]) });
    }

    fn compare_window(&mut self, ctx: &egui::Context, now: f64) {
        let Some(cmp) = &self.compare else { return };
        let mut open = true;
        let mut jump: Option<String> = None;
        let mut take: Option<(String, Value)> = None;
        let other_name = file_name(&cmp.other);
        let binary = matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_)));
        egui::Window::new(trf("Compare with {0}", &[&other_name]))
            .open(&mut open)
            .default_size([fit(ctx, 620.0), 460.0])
            .max_width(fit(ctx, 1400.0))
            .show(ctx, |ui| {
                ui.weak(tr("Values that differ between this save and the other one. \u{201C}Use theirs\u{201D} copies the other save's value into this one."));
                ui.separator();
                if cmp.diffs.is_empty() {
                    ui.label(tr("No differences. The saves hold the same values."));
                }
                ScrollArea::vertical().auto_shrink([false, false]).show_rows_locked(ui, 26.0, cmp.diffs.len(), |ui, range| {
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
                                    if !r.context().is_empty() {
                                        parts.push(r.context());
                                    }
                                    parts.push(r.name());
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
                            let show = |v: &Option<Value>| v.as_ref().map(widgets::display).unwrap_or_else(|| tr("(missing)").into());
                            if mine == *theirs {
                                ui.label(RichText::new(trf("{0}  (now the same)", &[&show(&mine)])).weak());
                                return;
                            }
                            ui.label(RichText::new(show(&mine)).color(theme::fg(ui, theme::MODIFIED)));
                            ui.label("\u{203A}");
                            ui.label(show(theirs));
                            let editable = self.model.by_ptr.get(p.as_str()).is_some_and(|&i| self.model.rows[i].editable);
                            let scalar = theirs.as_ref().is_some_and(|t| t.is_number() || t.is_string() || t.is_boolean());
                            if let (true, false, true, Some(t), Some(_)) = (editable, binary, scalar, theirs, &mine) {
                                if ui.small_button(tr("Use theirs")).clicked() {
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

    /// Backs the save up under `name`, then removes it and closes it so the game starts a new game. True on success.
    fn start_new_game(&mut self, path: &Path, name: &str) -> bool {
        if let Some(game) = crate::running::game_for(path) {
            self.fail(trf("{0} looks like it's running. Quit it first, then try again. The save wasn't changed.", &[&game]));
            return false;
        }
        match crate::backups::set_aside(path, name) {
            Ok(_) => {
                // Close the file so its old data can't be saved back over the new game.
                self.doc = None;
                self.dirty = false;
                self.ok(trf("Backed up as \u{201C}{0}\u{201D} and removed the save, so the game starts a new game. Restore it from Backups \u{203A} Save manager\u{2026} any time.", &[&name.trim()]));
                true
            }
            Err(e) => {
                self.fail(trf("Couldn't start a new game: {0}. The save is still in place.", &[&e]));
                false
            }
        }
    }

    fn new_game_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.new_game_name.take() else { return };
        let mut choice = None;
        modal(ctx, tr("Back up and start a new game"), |ui| {
            ui.label(tr("The save is backed up under this name and then removed, so the game starts a new game. Restore the backup later to come back to it."));
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text(tr("for example: first run")).desired_width(280.0));
            r.request_focus();
            let ready = !text.trim().is_empty();
            if r.lost_focus() && ready && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(ready, egui::Button::new(RichText::new(tr("Back up and start a new game")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
                if self.start_new_game(&path, &text) {
                    self.manager = Some((path, String::new(), None));
                } else {
                    self.new_game_name = Some(text);
                }
            }
            Some(false) => {}
            None => self.new_game_name = Some(text),
        }
    }

    fn backup_name_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.backup_name.take() else { return };
        let mut choice = None;
        modal(ctx, tr("Back up this save"), |ui| {
            ui.label(tr("Backs up the save as it is on disk now, without unsaved changes. The backup is kept until you delete it."));
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text(tr("for example: before the final boss")).desired_width(280.0));
            r.request_focus();
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(tr("Back up")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let Some(path) = self.doc.as_ref().map(|d| d.path.clone()) else { return };
                match crate::backups::take_named(&path, &text) {
                    Ok(_) => self.ok(trf("Backed up this save as \u{201C}{0}\u{201D}.", &[&text.trim()])),
                    Err(e) => {
                        self.fail(trf("Couldn't make the backup: {0}. The save wasn't changed.", &[&e]));
                        self.backup_name = Some(text);
                    }
                }
            }
            Some(false) => {}
            None => self.backup_name = Some(text),
        }
    }

    /// Backups › Open save manager: named backups (named copies, kept for good) and automatic backups of the
    /// open file, stored next to it. Restoring backs up the current file first.
    fn manager_window(&mut self, ctx: &egui::Context) {
        let Some((path, mut name, mut confirm)) = self.manager.take() else { return };
        // Another file was opened: this window was for the one before.
        if self.doc.as_ref().is_some_and(|d| d.path != path) {
            return;
        }
        let versions = crate::backups::list(&path);
        let mut open = true;
        let mut restore = None;
        let mut compare = None;
        let mut delete = None;
        let mut unname = None;
        let mut keep = false;
        let mut aside = false;
        let mut change_dir = false;
        let mut reset_dir = false;
        let has_doc = self.doc.is_some();
        let dirty = self.dirty;
        let newest_dir = versions.first().and_then(|v| v.path.parent()).map(Path::to_path_buf);
        egui::Window::new(tr("Save manager")).open(&mut open).collapsible(false).default_width(fit(ctx, 560.0)).max_width(fit(ctx, 900.0)).show(ctx, |ui| {
            ui.label(RichText::new(file_name(&path)).strong());
            if !path.exists() {
                ui.label(RichText::new(tr("There is no save file here right now. The game makes a new one when you save in it, or restore a backup below to put one back.")).color(theme::fg(ui, theme::WARN)));
            }
            if let Some(dir) = crate::backups::folder(&path) {
                let note = if self.settings.backup_dir.is_some() { tr("the folder you chose for backups") } else { tr("a folder next to the save, with files no game reads as saves") };
                ui.weak(trf("Backups are stored in {0} ({1}).", &[&dir.display(), &note]));
                if let Some(other) = newest_dir.as_ref().filter(|d| **d != dir) {
                    ui.weak(trf("The newest backup is in {0} (the folder above couldn't be written to, or it was made earlier).", &[&other.display()]));
                }
                if !WEB && !cfg!(target_os = "android") {
                    ui.horizontal(|ui| {
                        if ui.small_button(tr("Keep backups in another folder\u{2026}")).clicked() {
                            change_dir = true;
                        }
                        if self.settings.backup_dir.is_some() && ui.small_button(tr("Back to the default folder")).clicked() {
                            reset_dir = true;
                        }
                    });
                }
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(TextEdit::singleline(&mut name).hint_text(tr("backup name, e.g. first run")).desired_width(260.0));
                let ready = !name.trim().is_empty() && path.is_file();
                if ui.add_enabled(ready, egui::Button::new(tr("Back up this save"))).on_disabled_hover_text(tr("Type a name for the backup first")).on_hover_text(tr("Keeps a named backup of the save file as it is on disk now, until you delete it. The save itself is not changed.")).clicked() {
                    keep = true;
                }
                if ui.add_enabled(ready, egui::Button::new(tr("Back up and start a new game"))).on_hover_text(tr("Saves a named backup, then removes the current save file so the game starts a new game. Restore the backup later to go back.")).clicked() {
                    aside = true;
                }
            });
            ui.weak(tr("For games with a single save: use \u{201C}Back up and start a new game\u{201D}, play a new run, and restore the old backup later."));
            ui.separator();
            if versions.is_empty() {
                ui.label(tr("No backups yet. One is made automatically each time you save, or press \u{201C}Back up this save\u{201D}."));
            }
            ScrollArea::vertical().show_list_fit(ui, "app_list3", 380.0, |ui| {
                for (title, named) in [(tr("Named backups (kept until you delete them)"), true), (trf("Automatic backups (made each time you save; newest {0} kept)", &[&crate::backups::keep()]).as_str(), false)] {
                    let rows: Vec<&crate::backups::Version> = versions.iter().filter(|v| v.name.is_some() == named).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    ui.strong(title);
                    for v in rows {
                        ui.horizontal(|ui| {
                            let when = crate::backups::describe(v.when);
                            let label = match &v.name {
                                Some(n) => format!("\u{201C}{n}\u{201D} \u{00B7} {when} ({})", human_size(v.size as usize)),
                                None => format!("{when} ({})", human_size(v.size as usize)),
                            };
                            ui.label(label);
                            if confirm.as_ref().is_some_and(|(p, _)| *p == v.path) {
                                let is_restore = confirm.as_ref().is_some_and(|(_, r)| *r);
                                let yes = match (is_restore, has_doc && dirty) {
                                    (true, true) => tr("Restore and drop unsaved changes"),
                                    (true, false) => tr("Restore now"),
                                    _ => tr("Delete permanently"),
                                };
                                if ui.button(yes).clicked() {
                                    if is_restore { restore = Some(v.path.clone()) } else { delete = Some(v.path.clone()) }
                                    confirm = None;
                                }
                                if ui.button(tr("Cancel")).clicked() {
                                    confirm = None;
                                }
                            } else {
                                if ui.small_button(tr("Restore this backup")).on_hover_text(tr("Replaces the current save with this backup. The current save is backed up first, so you can undo it.")).clicked() {
                                    confirm = Some((v.path.clone(), true));
                                }
                                if has_doc && path.is_file() && ui.small_button(tr("Compare")).on_hover_text(tr("Shows what differs between this backup and the open save")).clicked() {
                                    compare = Some(v.path.clone());
                                }
                                if v.name.is_some() && ui.small_button(tr("Make automatic")).on_hover_text(tr("Stops keeping this backup for good; it is cleared out in time like the automatic ones")).clicked() {
                                    unname = Some(v.path.clone());
                                }
                                if ui.small_button(tr("Delete backup")).on_hover_text(tr("Permanently removes this backup")).clicked() {
                                    confirm = Some((v.path.clone(), false));
                                }
                            }
                        });
                    }
                    ui.add_space(6.0);
                }
            });
            if let Some(dir) = newest_dir.clone().or_else(|| crate::backups::folder(&path)).filter(|d| d.is_dir()) {
                if let Some(label) = reveal_text(true) {
                    if ui.button(label).clicked() {
                        platform::reveal(&dir);
                    }
                }
            }
        });
        let find = |p: &PathBuf| versions.iter().find(|v| v.path == *p);
        if change_dir {
            self.ask(Ask::BackupDir, self.settings.backup_dir.clone(), None);
        }
        if reset_dir {
            self.settings.set_backup_dir(None);
        }
        if keep {
            match crate::backups::take_named(&path, &name) {
                Ok(_) => {
                    self.ok(trf("Backed up this save as \u{201C}{0}\u{201D}.", &[&name.trim()]));
                    name.clear();
                }
                Err(e) => self.fail(trf("Couldn't make the backup: {0}. The save wasn't changed.", &[&e])),
            }
        }
        if aside && self.start_new_game(&path, &name) {
            self.manager = Some((path, String::new(), None));
            return;
        }
        if let Some(p) = restore {
            let what = find(&p).map(|v| v.name.as_ref().map(|n| format!("\u{201C}{n}\u{201D}")).unwrap_or_else(|| trf("the backup from {0}", &[&crate::backups::describe(v_when(&versions, &p))]))).unwrap_or_default();
            self.restore_to(&path, &p, &what);
        }
        if let Some(p) = compare {
            self.compare_with(p);
        }
        if let Some(v) = unname.as_ref().and_then(find) {
            match crate::backups::unlock(v) {
                Ok(()) => self.ok(tr("That backup is automatic now: it is cleared out in time like the others.")),
                Err(e) => self.fail(trf("Couldn't change that backup: {0}", &[&e])),
            }
        }
        if let Some(v) = delete.as_ref().and_then(find) {
            match crate::backups::remove(v) {
                Ok(()) => self.ok(tr("Deleted the backup.")),
                Err(e) => self.fail(trf("Couldn't delete the backup: {0}. It is still in the list.", &[&e])),
            }
        }
        // A restore opens the file, which closes this window unless it is still wanted.
        if open && self.manager.is_none() {
            self.manager = Some((path, name, confirm));
        }
    }

    fn ids_in_save(&self) -> Vec<u64> {
        match self.doc.as_ref().map(|d| &d.content) {
            Some(Content::Binary(b)) => crate::steamid::in_bytes(b).into_iter().map(|(id, _)| id).collect(),
            Some(c) => root(c).map(|v| crate::steamid::in_values(v).into_iter().map(|(id, _)| id).collect()).unwrap_or_default(),
            None => vec![],
        }
    }

    fn start_patches(&mut self) {
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

    fn load_patch_file(&mut self, path: &Path) {
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

    fn patches_dialog(&mut self, ctx: &egui::Context) {
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
    fn apply_patches(&mut self, p: &Patches) {
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

    /// Replaces the whole binary content (its length may change) as one undo step.
    fn replace_bytes(&mut self, after: Vec<u8>) {
        let Some(Doc { content: Content::Binary(data), .. }) = &mut self.doc else { return };
        let before = std::mem::replace(data, after.clone());
        self.undo.push(Change::Bytes { off: 0, before, after });
        self.redo.clear();
        self.last_edit = None;
        self.after_change("");
    }

    fn start_rebind(&mut self) {
        let ids = self.ids_in_save();
        if ids.is_empty() {
            return self.fail(tr("No Steam ID was found in this save's data (some games key the save to the account in other ways)."));
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
        modal(ctx, tr("Change Steam account in this save"), |ui| {
            ui.label(tr("Steam IDs in this save:"));
            for (i, id) in ids.iter().enumerate() {
                ui.radio_value(&mut pick, i, format!("{id} (account {})", id - crate::steamid::BASE));
            }
            ui.add_space(6.0);
            ui.label(tr("Change it to:"));
            ui.add(TextEdit::singleline(&mut text).hint_text(tr("17-digit Steam ID or account number")).desired_width(280.0));
            if !locals.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.weak(tr("Accounts on this computer:"));
                    for id in &locals {
                        if ui.small_button((id - crate::steamid::BASE).to_string()).clicked() {
                            text = id.to_string();
                        }
                    }
                });
            }
            ui.weak(tr("Every copy of the ID changes; save to write the file. Some games also check the folder the save sits in."));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let ok = crate::steamid::parse(&text).is_some();
                if ui.add_enabled(ok, egui::Button::new(RichText::new(tr("Change")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let (Some(new), Some(&old)) = (crate::steamid::parse(&text), ids.get(pick)) else { return };
                let n = self.rebind_to(old, new);
                self.ok(if n == 1 { tr("Changed the Steam ID in 1 place. Save to write the file.").to_string() } else { trf("Changed the Steam ID in {0} places. Save to write the file.", &[&n]) });
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
        modal(ctx, tr("Find checksum"), |ui| {
            if hits.is_empty() {
                ui.label(tr("No checksum the editor knows was found at the start or end of this file."));
                ui.weak(tr("It may be encrypted, compressed, or checked with something the game made up. The value search still works on it."));
            } else {
                ui.label(if hits.len() == 1 { tr("This checksum holds for the file:") } else { tr("These checksums hold for the file:") });
                ui.weak(tr("Pick one to have it written again every time you save. Saving a few times and searching again narrows a long list."));
                ui.add_space(6.0);
                for h in &hits {
                    ui.horizontal(|ui| {
                        ui.label(h.describe());
                        if current == Some(*h) {
                            ui.weak(tr("(kept fixed)"));
                        } else if ui.button(tr("Keep fixed")).clicked() {
                            pick = Some(Some(*h));
                        }
                    });
                }
            }
            if current.is_some() && ui.button(tr("Stop fixing the checksum")).clicked() {
                pick = Some(None);
            }
            ui.add_space(6.0);
            if ui.button(tr("Close")).clicked() {
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
                self.ok(tr("The checksum will be written again every time you save this file."));
            }
        } else if close {
            self.sum_hits = None;
        }
    }

    fn password_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut text) = self.password_prompt.take() else { return };
        let mut choice = None;
        modal(ctx, tr("Decrypt with password"), |ui| {
            ui.label(tr("Some games encrypt their saves with a password built into the game."));
            ui.weak(tr("Supported: Easy Save 3 (Unity) and Godot encrypted files. Easy Save's default password is tried automatically."));
            ui.add_space(6.0);
            let r = ui.add(TextEdit::singleline(&mut text).hint_text(tr("password")).desired_width(280.0));
            r.request_focus();
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(true);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new(tr("Decrypt")).color(Color32::WHITE)).fill(theme::ACCENT)).clicked() {
                    choice = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    choice = Some(false);
                }
            });
        });
        match choice {
            Some(true) => {
                let pw = text.clone();
                self.reopen_with(|o| o.password = (!pw.is_empty()).then_some(pw.clone()));
                if matches!(self.doc.as_ref().map(|d| &d.content), Some(Content::Binary(_))) {
                    self.fail(tr("That password didn't decrypt the file (or the encryption isn't one this editor knows)."));
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

    fn note_hidden(&self) -> bool {
        self.settings.hidden_notes.contains(&Settings::note_key(&self.doc_note))
    }

    /// The note about the opened file, on one line with More to read it all. Dismissing it
    /// hides it for every file with the same note, until View \u{203A} Show the note about this file.
    fn note_bar(&mut self, ui: &mut egui::Ui) {
        let font = egui::TextStyle::Body.resolve(ui.style());
        let width = |ui: &egui::Ui, t: &str| ui.fonts(|f| f.layout_no_wrap(t.to_string(), font.clone(), Color32::WHITE).size().x);
        let pad = 2.0 * ui.spacing().button_padding.x + ui.spacing().item_spacing.x;
        let dismiss_w = width(ui, tr("Dismiss")) + pad;
        let more_w = width(ui, tr("More")).max(width(ui, tr("Less"))) + pad;
        let note = crate::i18n::tr_any(&self.doc_note).to_string();
        let fits = width(ui, &note) + dismiss_w + 8.0 <= ui.available_width();
        let (mut dismiss, mut toggle) = (false, false);
        ui.horizontal_top(|ui| {
            let text_w = (ui.available_width() - dismiss_w - if fits { 0.0 } else { more_w } - 4.0).max(80.0);
            ui.allocate_ui(egui::vec2(text_w, 0.0), |ui| {
                ui.set_width(text_w);
                let text = RichText::new(&note).color(ui.visuals().weak_text_color());
                if fits || self.note_open {
                    ui.add(egui::Label::new(text).wrap());
                } else {
                    ui.add(egui::Label::new(text).truncate());
                }
            });
            if !fits && ui.small_button(if self.note_open { tr("Less") } else { tr("More") }).clicked() {
                toggle = true;
            }
            if ui.small_button(tr("Dismiss")).on_hover_text(tr("Hide this note for every file like this one. View \u{203A} Show the note about this file brings it back.")).clicked() {
                dismiss = true;
            }
        });
        if toggle {
            self.note_open = !self.note_open;
        }
        if dismiss {
            self.settings.hidden_notes.push(Settings::note_key(&self.doc_note));
            self.settings.save();
        }
    }

    /// "Save", or "Save (3)" with the number of unsaved changes.
    fn save_label(&self) -> String {
        let n = self.change_count();
        if self.dirty && n > 0 {
            trf("Save ({0})", &[&n])
        } else {
            tr("Save").to_string()
        }
    }

    fn save_button(&mut self, ui: &mut egui::Ui) {
        let hi = self.has_icons;
        let label = self.save_label();
        let save_btn = if self.dirty {
            egui::Button::new(widgets::icon_text_colored(ui, hi, icon::SAVE, &label, Color32::WHITE)).fill(theme::ACCENT)
        } else {
            egui::Button::new(widgets::icon_text(ui, hi, icon::SAVE, &label))
        };
        let b = ui.add_enabled(self.dirty, save_btn);
        if widgets::named(b, egui::WidgetType::Button, &label).on_hover_text(tr("Save changes to the file (Ctrl+S)")).clicked() {
            self.save_clicked();
        }
    }

    /// Opens the Save manager window for the open save.
    fn open_save_manager(&mut self) {
        self.manager = self.doc.as_ref().map(|d| (d.path.clone(), String::new(), None));
    }

    fn changes_toggle(&mut self, ui: &mut egui::Ui) {
        let hi = self.has_icons;
        // The selected colours: white on the light theme's pale selection could not be read.
        let color = if self.show_changes { ui.visuals().selection.stroke.color } else { ui.visuals().text_color() };
        let label = tr("Changes");
        let r = ui.toggle_value(&mut self.show_changes, widgets::icon_text_colored(ui, hi, icon::HISTORY, label, color));
        // Its text starts with an icon glyph: the screen reader gets the words and the on/off state.
        let (on, enabled) = (self.show_changes, r.enabled());
        r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, enabled, on, label));
        r.on_hover_text(tr("List everything you've changed since the last save"));
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        if narrow(ui) {
            return self.phone_toolbar(ui);
        }
        let hi = self.has_icons;
        ui.add_space(4.0);
        // The toolbar wraps well before the rest of the layout turns phone-sized: the file
        // name and its chips need the room.
        // Also when the one-line toolbar didn't fit last time (long translations, a long file
        // name or game name).
        let need_id = egui::Id::new(("toolbar_need", self.doc.as_ref().map(|d| d.path.clone()), crate::i18n::current().code));
        let need = ui.ctx().data(|d| d.get_temp::<f32>(need_id)).unwrap_or(0.0);
        let wrap = ui.available_width() < 900.0 || ui.available_width() < need;
        let full_w = ui.available_width();
        let row = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
            if wrap {
                ui.horizontal_wrapped(|ui| add(ui));
            } else {
                ui.horizontal(|ui| add(ui));
            }
        };
        row(ui, &mut |ui| {
            // These buttons start with an icon glyph: `named` keeps it out of what a screen reader says.
            let b = ui.button(widgets::icon_text(ui, hi, icon::OPEN, tr("Open")));
            if widgets::named(b, egui::WidgetType::Button, tr("Open")).on_hover_text(tr("Open a save file (Ctrl+O)")).clicked() {
                self.ask(Ask::Open, None, None);
            }
            if !cfg!(target_os = "android")
                && !WEB
                && ui.button(tr("Find saves")).on_hover_text(tr("Pick a game from the list to see the saves it has on this computer")).clicked()
            {
                self.game_picker = Some(self.picker());
            }
            self.save_button(ui);
            // The list of what Save will write sits next to it.
            if self.doc.is_some() {
                self.changes_toggle(ui);
            }
            if !wrap {
                ui.separator();
            }
            let b = ui.add_enabled(!self.undo.is_empty(), egui::Button::new(widgets::icon_text(ui, hi, icon::UNDO, tr("Undo"))));
            if widgets::named(b, egui::WidgetType::Button, tr("Undo")).on_hover_text(tr("Ctrl+Z")).clicked() {
                self.undo_redo(true);
            }
            let b = ui.add_enabled(!self.redo.is_empty(), egui::Button::new(widgets::icon_text(ui, hi, icon::REDO, tr("Redo"))));
            if widgets::named(b, egui::WidgetType::Button, tr("Redo")).on_hover_text(tr("Ctrl+Y")).clicked() {
                self.undo_redo(false);
            }
            if self.doc.is_some() {
                if !wrap {
                    ui.separator();
                }
                let b = ui.button(tr("Close save"));
                if b.on_hover_text(tr("Close this save and go back to the start page (Ctrl+W)")).clicked() {
                    self.close_clicked();
                }
                if !WEB && ui.button(tr("Save manager")).on_hover_text(tr("All backups of this save in one window: restore, compare, delete, name. Restoring backs up the current save first.")).clicked() {
                    self.open_save_manager();
                }
            }
            let Some(doc) = &self.doc else { return };
            if !wrap {
                ui.separator();
            }
            ui.label(RichText::new(file_name(&doc.path)).family(theme::semibold_family()).size(16.0))
                .on_hover_text(doc.path.display().to_string());
            self.file_chips(ui, true);
            if !wrap {
                let used = ui.min_rect().width() + ui.spacing().item_spacing.x;
                if used > full_w {
                    ui.ctx().data_mut(|d| d.insert_temp(need_id, used));
                    ui.ctx().request_repaint();
                }
            }
        });
        ui.add_space(2.0);
    }

    /// The phone's toolbar: one line with the menus, the file name, Undo and Save.
    fn phone_toolbar(&mut self, ui: &mut egui::Ui) {
        let hi = self.has_icons;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            // Finger-sized buttons.
            ui.spacing_mut().button_padding = egui::vec2(12.0, 7.0);
            let m = ui.menu_button(RichText::new(PHONE_MENU).size(20.0), |ui| {
                ui.spacing_mut().button_padding = egui::vec2(10.0, 8.0);
                self.menus(ui);
            })
            .response;
            widgets::named(m, egui::WidgetType::Button, tr("Menu")).on_hover_text(tr("Menu"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.save_button(ui);
                if self.doc.is_some() && widgets::symbol_button(ui, "\u{00D7}", tr("Close save"), tr("Close this save and go back to the start page (Ctrl+W)")).clicked() {
                    self.close_clicked();
                }
                let b = ui.add_enabled(!self.undo.is_empty(), egui::Button::new(widgets::icon(hi, icon::UNDO, "\u{21A9}").size(17.0)));
                if widgets::named(b, egui::WidgetType::Button, tr("Undo")).on_hover_text(tr("Undo")).clicked() {
                    self.undo_redo(true);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let name = self.doc.as_ref().map(|d| (file_name(&d.path), d.path.display().to_string()));
                    match name {
                        Some((n, full)) => {
                            ui.add(egui::Label::new(RichText::new(n).family(theme::semibold_family()).size(15.0)).truncate()).on_hover_text(full);
                        }
                        None => {
                            ui.add(egui::Label::new(RichText::new(platform::DISPLAY_NAME).family(theme::semibold_family()).size(15.0)).truncate());
                        }
                    }
                });
            });
        });
        // Warnings about the file (game running, signed save, names not found) on their own line.
        if self.doc.as_ref().is_some_and(|d| self.has_warning_chips(d)) {
            ui.horizontal_wrapped(|ui| self.file_chips(ui, false));
        }
        ui.add_space(2.0);
    }

    fn has_warning_chips(&self, doc: &Doc) -> bool {
        self.running.is_some()
            || doc.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. } | Layer::RenpyPersistent { signed: true }))
            || matches!(&doc.content, Content::Json { value, .. } if names::is_rpgmaker(value))
    }

    /// The chips after the file name: its format (when `with_format`) and any warnings.
    fn file_chips(&mut self, ui: &mut egui::Ui, with_format: bool) {
        let Some(doc) = &self.doc else { return };
        if with_format {
            let kind = friendly_kind(&doc.layers, &doc.content);
            let described = formats::describe(&doc.layers, &doc.content);
            // A game screen was found for a generic format (JSON, XML...): name the game on the chip.
            let named = doc.game.and_then(|g| g.name()).filter(|n| !kind.contains(n));
            let chip_text = match named {
                Some(n) => trf("{0} save", &[&n]),
                None => kind_label(&kind),
            };
            let hover = if named.is_some() { trf("Detected format: {0} ({1})", &[&kind, &described]) } else { trf("Detected format: {0}", &[&described]) };
            Self::chip(ui, &chip_text, theme::ACCENT).on_hover_text(hover);
        }
        if doc.layers.iter().any(|l| matches!(l, Layer::RenpyZip { signed: true, .. } | Layer::RenpyPersistent { signed: true })) {
            if crate::renpy_sign::can_sign() {
                Self::chip(ui, tr("Signed, will be re-signed"), theme::OK).on_hover_text(
                    tr("Ren'Py 8.1+ signs its saves. Your Ren'Py key was found on this PC, so edited saves are re-signed and load normally."),
                );
            } else {
                Self::chip(ui, tr("Signed save"), theme::WARN).on_hover_text(tr("Ren'Py 8.1+ signs its saves, but your Ren'Py key (%APPDATA%\\RenPy\\tokens) wasn't found. The old signature will be removed, and the game will ask once whether to load the save: choose Yes."));
            }
        }
        if let Some(game) = &self.running {
            Self::chip(ui, &trf("{0} is running", &[game]), theme::WARN).on_hover_text(
                tr("The game was running when this file was opened. Quit it before saving, or it may overwrite your changes."),
            );
        }
        let rpg = matches!(&doc.content, Content::Json { value, .. } if names::is_rpgmaker(value));
        let mut locate = false;
        if rpg {
            match &doc.names.source {
                Some(dir) => {
                    Self::chip(ui, tr("Names from game data"), theme::OK).on_hover_text(dir.display().to_string());
                }
                None => {
                    let text = tr("Game data not found. Click to locate it.");
                    let chip = Self::chip(ui, text, theme::WARN).interact(egui::Sense::click());
                    if widgets::named(chip, egui::WidgetType::Button, text)
                        .on_hover_text(tr("Point to the game's data folder to see variable, switch and item names"))
                        .clicked()
                    {
                        locate = true;
                    }
                }
            }
        }
        if locate {
            self.locate_data_dir();
        }
    }

    fn change_count(&self) -> usize {
        let Some(d) = &self.doc else { return 0 };
        if self.changes.gen == Some(self.edit_gen) {
            self.changes.count()
        } else {
            Changes::of(d, &self.touched).count()
        }
    }

    /// Brings `changes` up to date with the last edit.
    fn refresh_changes(&mut self) {
        if self.changes.gen != Some(self.edit_gen) {
            self.changes = match &self.doc {
                Some(d) => Changes::of(d, &self.touched),
                None => Changes::default(),
            };
            self.changes.gen = Some(self.edit_gen);
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
                    ui.label(RichText::new(if n == 1 { tr("1 unsaved change").to_string() } else { crate::i18n::trf("{0} unsaved changes", &[&n]) }).color(theme::fg(ui, theme::MODIFIED)));
                } else {
                    ui.weak(tr("No unsaved changes"));
                }
                ui.add_space(12.0);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // Done and failed differ by a mark as well as by colour (✔ and ⚠ without the icon font).
                let (color, glyph, fallback) = match self.status_kind {
                    StatusKind::Info => (ui.visuals().weak_text_color(), "", ""),
                    StatusKind::Ok => (theme::fg(ui, theme::OK), icon::CHECK, "\u{2714}"),
                    StatusKind::Err => (theme::fg(ui, theme::ERR), icon::WARNING, "\u{26A0}"),
                };
                if !glyph.is_empty() {
                    ui.label(widgets::icon(self.has_icons, glyph, fallback).color(color).size(12.0));
                }
                if self.status_kind == StatusKind::Err && self.banner.as_deref() == Some(self.status.as_str()) {
                    // The whole message is in the banner at the top.
                    ui.label(RichText::new(tr("That didn't work. The message at the top says why.")).color(color));
                } else {
                    ui.add(egui::Label::new(RichText::new(&self.status).color(color)).truncate()).on_hover_text(&self.status);
                }
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
        bar(ui, &mut |ui| {
            let old = self.tab;
            if let Some(g) = game {
                let r = ui.selectable_value(&mut self.tab, Tab::Game, crate::i18n::tr_any(g.tab_label())).on_hover_text(tr("This game's own screen: its stats, inventory and more, laid out like the game"));
                theme::mark_selected(ui, &r, old == Tab::Game);
            }
            let r = ui.selectable_value(&mut self.tab, Tab::Values, tr("Values")).on_hover_text(tr("Every editable value in a simple list"));
            theme::mark_selected(ui, &r, old == Tab::Values);
            let r = ui.selectable_value(&mut self.tab, Tab::Tree, tr("Tree")).on_hover_text(tr("The full structure of the save, as nested groups"));
            theme::mark_selected(ui, &r, old == Tab::Tree);
            if is_json {
                let r = ui.selectable_value(&mut self.tab, Tab::Raw, tr("Raw JSON")).on_hover_text(tr("Edit the JSON text directly"));
                theme::mark_selected(ui, &r, old == Tab::Raw);
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
            if self.tab == Tab::Game {
                self.game_find_box(ui);
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
                if !self.search.is_empty() && ui.small_button(tr("Clear")).clicked() {
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
        if narrow(ui) && self.tab == Tab::Game {
            section_chips(ui);
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

    /// The find box of a game screen: highlights and scrolls to labels on the visible tab.
    fn game_find_box(&mut self, ui: &mut egui::Ui) {
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
                if ui.small_button(tr("Next")).clicked() || (enter && !back) {
                    self.game_find_idx = (self.game_find_idx + 1) % total;
                    if enter {
                        r.request_focus();
                    }
                }
                if ui.small_button(tr("Prev")).clicked() || (enter && back) {
                    self.game_find_idx = (self.game_find_idx + total - 1) % total;
                    if enter {
                        r.request_focus();
                    }
                }
            }
            if ui.small_button(tr("Clear")).clicked() || ((r.has_focus() || r.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Escape))) {
                self.game_find.clear();
            }
        }
    }

    /// The Raw keys / Advanced toggles.
    fn view_options(&mut self, ui: &mut egui::Ui) {
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
        let label = if hidden > 0 && !self.settings.show_advanced { format!("{} ({hidden})", tr("Advanced")) } else { tr("Advanced").into() };
        changed |= widgets::labeled_toggle(ui, &mut self.settings.show_advanced, &label, &tip);
        if changed {
            self.settings.save();
        }
    }

    fn structured_body(&mut self, ui: &mut egui::Ui, now: f64) {
        let hi = self.has_icons;
        let mut edits: Vec<(String, Value)> = Vec::new();
        let (mut copy, mut paste_at) = (None, None);
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
                ScrollArea::both().auto_shrink([false, false]).max_width(w + sb).show_locked(ui, |ui| {
                    ui.set_max_width(w);
                    crate::game_ui::show(game, ui, &mut cx);
                    ui.add_space(12.0);
                    ui.weak(tr("Everything else in this save is in the Values and Tree tabs."));
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
                if self.disp_key.as_ref() != Some(&key) && !typing {
                    self.disp = values_view::filter(&self.model, r, &self.search, self.settings.show_advanced, &self.collapsed);
                    self.disp_key = Some(key);
                }
                if self.disp.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        if self.search.trim().is_empty() {
                            ui.weak(tr("No editable values here. Try turning on Advanced, or use the Tree view."));
                        } else {
                            ui.label(trf("Nothing matches \u{201C}{0}\u{201D}.", &[&self.search.trim()]));
                            ui.weak(tr("Try part of a name (\u{201C}gold\u{201D}) or a current value (\u{201C}1500\u{201D})."));
                            if !self.settings.show_advanced {
                                ui.weak(tr("Advanced values are hidden \u{2014} tick Advanced to search them too."));
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
                };
                values_view::show(ui, &self.model, &self.disp, &self.collapsed, &mut ctx);
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
        if let Some((what, values)) = copy {
            self.copy_values(what, values);
        }
        if let Some(p) = paste_at {
            self.paste_one_at(&p);
        }
    }

    fn changes_panel(&mut self, ctx: &egui::Context, now: f64) {
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
                    if ui.small_button(tr("Close")).clicked() {
                        close = true;
                    }
                });
            });
            ui.weak(tr("Since you opened or last saved this file."));
            ui.add_space(4.0);
            if self.dirty {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button(tr("Copy as text")).on_hover_text(tr("Copy the list of changes, to paste when asking for help or sharing what you did")).clicked() {
                        copy = true;
                    }
                    if ui.small_button(tr("Save as preset\u{2026}")).on_hover_text(tr("Keep these changes to make them again on another save or slot")).clicked() {
                        preset = true;
                    }
                    if ui.small_button(tr("Revert all")).on_hover_text(tr("Put every value back as it was when the file was opened or last saved")).clicked() {
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
                            if ui.small_button(tr("Revert")).clicked() {
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
                                        let old = bv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                                        let new = cv.pointer(p).map(widgets::display).unwrap_or_else(|| "(none)".into());
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(name);
                                            ui.label(RichText::new(old).weak().strikethrough());
                                            ui.label("\u{203A}");
                                            ui.label(RichText::new(new).color(theme::fg(ui, theme::MODIFIED)));
                                        });
                                    }
                                });
                                if ui.small_button(tr("Revert all of these")).clicked() {
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
                                let old = bv.pointer(ptr).map(widgets::display).unwrap_or_else(|| "(none)".into());
                                let new = cv.pointer(ptr).map(widgets::display).unwrap_or_else(|| "(none)".into());
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new(old).weak().strikethrough());
                                    ui.label("\u{203A}");
                                    ui.label(RichText::new(new).color(theme::fg(ui, theme::MODIFIED)));
                                });
                            }
                            if ui.small_button(tr("Revert")).clicked() {
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
                        ui.label(RichText::new(tr("Cairn saves in this slot")).family(theme::semibold_family()).size(16.0));
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
                    ui.label(RichText::new(tr("Named values")).family(theme::semibold_family()).size(16.0));
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
                    ui.label(RichText::new(tr("Value search")).family(theme::semibold_family()).size(16.0));
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
                        ui.label(RichText::new(format!("Selected: 0x{:X}", h.selected)).family(theme::semibold_family()).size(16.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            crate::widgets::checkbox(ui, &mut h.big_endian, tr("Big-endian")).on_hover_text(tr("Most PC games use little-endian (unticked)"));
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("Go to"));
                        let r = ui.add(TextEdit::singleline(&mut h.goto_text).hint_text(tr("hex offset")).desired_width(100.0));
                        if (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.small_button(tr("Go")).clicked() {
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

    // ------------------------------------------------------------ UI: welcome

    fn welcome(&mut self, ui: &mut egui::Ui) {
        let mut open: Option<PathBuf> = None;
        let mut restore_missing: Option<PathBuf> = None;
        let mut browse: Option<Option<PathBuf>> = None;
        ScrollArea::vertical().auto_shrink([false, false]).show_locked(ui, |ui| {
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
                    tr("Edit game saves safely. It runs in your browser and nothing is uploaded.")
                } else if cfg!(target_os = "android") {
                    tr("Edit game saves safely. Everything stays on this device.")
                } else {
                    tr("Edit game saves safely. Everything stays on this computer.")
                });
                ui.add_space(14.0);
                if self.saves_found.is_some() {
                    // Finding the saves for you comes first; picking the file yourself is the other way in.
                    let find = egui::Button::new(RichText::new(tr("Find my game's saves")).color(Color32::WHITE)).fill(theme::ACCENT).min_size(egui::vec2(220.0, 40.0));
                    let file = egui::Button::new(widgets::icon_text(ui, self.has_icons, icon::OPEN, tr("Open a file\u{2026}"))).min_size(egui::vec2(160.0, 40.0));
                    let w = 220.0 + 160.0 + ui.spacing().item_spacing.x;
                    let side_by_side = ui.available_width() >= w + 16.0;
                    let mut clicks = (false, false);
                    if side_by_side {
                        ui.allocate_ui_with_layout(egui::vec2(w, 40.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            clicks.0 = ui.add(find).on_hover_text(tr("Pick your game from the list; its saves on this computer are found for you")).clicked();
                            clicks.1 = ui.add(file).clicked();
                        });
                    } else {
                        clicks.0 = ui.add(find).clicked();
                        clicks.1 = ui.add(file).clicked();
                    }
                    if clicks.0 {
                        let search = self.saves_found.clone().unwrap_or_else(search_saves);
                        self.game_picker = Some(GamePicker::with(search));
                    }
                    if clicks.1 {
                        browse = Some(None);
                    }
                } else {
                    let btn = egui::Button::new(widgets::icon_text_colored(ui, self.has_icons, icon::OPEN, tr("Open a save file"), Color32::WHITE))
                        .fill(theme::ACCENT)
                        .min_size(egui::vec2(220.0, 40.0));
                    if ui.add(btn).clicked() {
                        browse = Some(None);
                    }
                }
                ui.add_space(4.0);
                if !cfg!(target_os = "android") {
                    ui.weak(tr("or drag a file anywhere onto this window"));
                }
                ui.add_space(18.0);

                let w = ui.available_width().min(680.0);
                ui.allocate_ui(egui::vec2(w, 0.0), |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        if let Some(p) = self.your_games(ui) {
                            open = Some(p);
                        }
                        if self.saves_found.is_some() {
                            ui.add_space(8.0);
                        }
                        theme::card(ui, |ui| {
                            ui.label(RichText::new(tr("Recent files")).family(theme::semibold_family()).size(16.0));
                            if self.settings.recent.is_empty() {
                                ui.weak(tr("Files you open will appear here."));
                            }
                            for p in self.settings.recent.iter().take(8) {
                                let exists = crate::vfs::exists(p);
                                ui.horizontal(|ui| {
                                    let r = ui.add_enabled(exists, egui::Link::new(file_name(p)));
                                    if r.clicked() {
                                        open = Some(p.clone());
                                    }
                                    let folder = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
                                    if !exists && !WEB && !crate::backups::list(p).is_empty() {
                                        // Set aside for a new game (or deleted): its backups can bring it back.
                                        if ui.small_button(tr("Restore a backup\u{2026}")).on_hover_text(tr("The file isn't there any more, but it has backups")).clicked() {
                                            restore_missing = Some(p.clone());
                                        }
                                    }
                                    ui.add(egui::Label::new(RichText::new(if exists { folder } else { tr("(file no longer exists)").into() }).weak().size(12.5)).truncate());
                                });
                            }
                        });
                        ui.add_space(8.0);
                        theme::card(ui, |ui| {
                            ui.label(RichText::new(tr("Find your saves")).family(theme::semibold_family()).size(16.0));
                            let locations = settings::save_locations();
                            if WEB {
                                ui.weak(tr("The browser can't look through your folders, so pick the file yourself. On Windows most games keep saves in Documents\\My Games, %APPDATA%, %LOCALAPPDATA% or Steam\\userdata. Type one of these into the file picker's address bar to jump there."));
                            } else if !locations.is_empty() {
                                // Only over a list: nothing found left "Common places…:" over nothing.
                                ui.weak(if cfg!(target_os = "android") {
                                    tr("Common places emulators and games keep save files:")
                                } else {
                                    tr("Common places games keep save files on this PC:")
                                });
                            }
                            for (label, path) in locations {
                                ui.horizontal(|ui| {
                                    if ui.link(&label).clicked() {
                                        browse = Some(Some(path.clone()));
                                    }
                                    ui.add(egui::Label::new(RichText::new(path.display().to_string()).weak().size(12.5)).truncate());
                                });
                            }
                            ui.weak(tr("RPG Maker games usually keep saves in a \u{201C}save\u{201D} folder next to the game."));
                        });
                        ui.add_space(8.0);
                        theme::card(ui, |ui| {
                            ui.label(RichText::new(tr("What it can open")).family(theme::semibold_family()).size(16.0));
                            let games = trf("{0} games, {1} of them with a screen of their own: Elden Ring, Baldur's Gate, Mass Effect, Final Fantasy, Stardew Valley, Terraria, Cyberpunk 2077 and many more \u{00B7} Pok\u{00E9}mon Gen I\u{2013}III (emulator saves) \u{00B7} {2}",
                                &[
                                    &crate::games::GAMES.len(),
                                    &crate::games::GAMES.iter().filter(|g| g.support.starts_with("Game screen")).count(),
                                    &if WEB { tr("the full list is on the website") } else { tr("Find my game's saves lists them all") },
                                ],
                            );
                            let data_formats = if WEB {
                                tr("JSON \u{00B7} XML \u{00B7} INI/CFG \u{00B7} TOML \u{00B7} YAML \u{00B7} plist \u{00B7} MessagePack \u{00B7} BSON \u{00B7} Python pickle (SQLite needs the downloaded app)")
                            } else {
                                tr("JSON \u{00B7} XML \u{00B7} INI/CFG \u{00B7} TOML \u{00B7} YAML \u{00B7} plist \u{00B7} MessagePack \u{00B7} BSON \u{00B7} SQLite \u{00B7} Python pickle")
                            };
                            for (title, list) in [
                                (tr("Games:"), games.as_str()),
                                (tr("Game engines:"), tr("Ren'Py \u{00B7} RPG Maker MV/MZ/XP/VX/VX Ace \u{00B7} Unreal Engine (GVAS) \u{00B7} Unity (.NET BinaryFormatter, Easy Save 3, JSON, XML) \u{00B7} Godot (store_var, encrypted) \u{00B7} Minecraft NBT \u{00B7} TyranoScript \u{00B7} Twine/SugarCube \u{00B7} Barotrauma \u{00B7} Palworld (PlZ)")),
                                (tr("Data formats:"), data_formats),
                                (tr("Wrappers:"), tr("gzip \u{00B7} zlib \u{00B7} zstd \u{00B7} LZ4 \u{00B7} xz \u{00B7} bzip2 \u{00B7} zip \u{00B7} Base64 \u{00B7} UTF-16 \u{00B7} XOR \u{00B7} AES (with password)")),
                                (tr("Anything else:"), tr("Any file opens in the hex editor with value search")),
                            ] {
                                widgets::label_text(ui, title, list, true);
                            }
                        });
                    });
                });
            });
        });
        if let Some(p) = restore_missing {
            self.manager = Some((p, String::new(), None));
        }
        if let Some(p) = open {
            if crate::vfs::exists(&p) {
                self.request_open(p);
            } else {
                // A remembered save that is gone: open the list with the notice and its new search.
                let mut gp = self.picker();
                gp.missing = Some(p);
                self.game_picker = Some(gp);
            }
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

/// The phone's row of a game screen's sections: finger-sized chips that jump to each one,
/// marked when the section holds a change. Swipe it sideways when there are more.
fn section_chips(ui: &mut egui::Ui) {
    let list = crate::game_ui::sections(ui.ctx());
    if list.len() < 2 {
        return;
    }
    ui.add_space(2.0);
    ScrollArea::horizontal().id_salt("section_chips").auto_shrink([false, true]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.spacing_mut().button_padding = egui::vec2(16.0, 10.0);
            for s in &list {
                let title: String = if s.title.chars().count() > 28 { s.title.chars().take(27).chain(['\u{2026}']).collect() } else { s.title.clone() };
                let mut btn = egui::Button::new(RichText::new(title).size(16.0)).rounding(22.0).min_size(egui::vec2(48.0, 44.0));
                if s.edited {
                    btn = btn.stroke(egui::Stroke::new(1.5_f32, theme::MODIFIED));
                }
                let mut r = ui.add(btn);
                if s.edited {
                    ui.painter().circle_filled(r.rect.right_top() + egui::vec2(-9.0, 9.0), 4.0, theme::MODIFIED);
                    r = r.on_hover_text(tr("Has changes"));
                }
                if r.clicked() {
                    crate::game_ui::jump_to_section(ui.ctx(), s.index);
                }
            }
        });
    });
    ui.add_space(2.0);
}

/// The phone toolbar's menu button (U+2630, "trigram for heaven").
const PHONE_MENU: &str = "\u{2630}";

/// Below this window width the layout is the phone one.
const PHONE_W: f32 = 640.0;

fn narrow(ui: &egui::Ui) -> bool {
    ui.ctx().screen_rect().width() < PHONE_W
}

/// Whether the file sits in Steam's per-account cloud folder (`userdata/<id>/<app>/remote`).
fn in_steam_cloud(p: &Path) -> bool {
    let parts: Vec<String> = p.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
    parts.iter().any(|c| c == "userdata") && parts.iter().any(|c| c == "remote")
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

/// How many changes the Changes panel lists; a bulk edit can make tens of thousands, and
/// drawing a card for each one on every frame would freeze the window.
const CHANGES_SHOWN: usize = 300;

/// The line under a cut-short list of changes.
fn more_changes(ui: &mut egui::Ui, total: usize) {
    if total > CHANGES_SHOWN {
        ui.add_space(6.0);
        ui.weak(trf("Showing the first {0} of {1} changes. Saving, Revert all and Copy as text cover all of them.", &[&CHANGES_SHOWN, &total]));
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
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        #[cfg(target_os = "android")]
        {
            let _ = platform::android::CTX.set(ctx.clone());
            platform::android::take_typed(raw);
        }
        wrap_dialog_tab(ctx, raw);
        if self.saving.is_some() {
            hold_input(raw);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame(ctx);
    }
}

/// While a save is being written: takes away every click, key, typed text and dropped file
/// before anything on screen sees it, so nothing can change the file being saved. (Pointer
/// moves and focus changes stay, so the window still looks alive.)
fn hold_input(raw: &mut egui::RawInput) {
    raw.events.retain(|e| matches!(e, egui::Event::PointerMoved(_) | egui::Event::PointerGone | egui::Event::WindowFocused(_) | egui::Event::Screenshot { .. }));
    raw.dropped_files.clear();
    raw.hovered_files.clear();
}

impl App {
    /// One frame of the window.
    fn frame(&mut self, ctx: &egui::Context) {
        begin_dialogs(ctx);
        let now = ctx.input(|i| i.time);
        let saving = self.saving.is_some();
        #[cfg(not(target_os = "android"))]
        self_shot(ctx);
        theme::fit_window_margin(ctx);
        self.refresh_changes();

        // Files dropped on the window.
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone())).filter(|_| !saving && !modal_shown(ctx)) {
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
        if !saving && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.ask(Ask::Open, None, None);
        }
        if !saving && self.doc.is_some() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_clicked();
        }
        let structured = self.doc.as_ref().is_some_and(|d| root(&d.content).is_some());
        if structured && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.focus_search = true;
            self.focus_game_find = self.tab == Tab::Game;
            self.tab = if matches!(self.tab, Tab::Raw | Tab::Tree | Tab::Game) { self.tab } else { Tab::Values };
        }
        if no_text_focus && !saving {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z))
                || ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y))
            {
                self.undo_redo(false);
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
                self.undo_redo(true);
            }
        }
        self.tab_keys(ctx);
        // Remember zoom.
        let zoom = ctx.zoom_factor();
        if (zoom - self.settings.zoom).abs() > 0.001 {
            self.settings.zoom = zoom;
            self.settings.save();
        }
        // Don't lose work on close. A save being written is always finished first.
        if ctx.input(|i| i.viewport().close_requested()) {
            if saving {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.close_after_save = true;
            } else if self.any_dirty() && !self.allow_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_first_dirty();
                self.confirm_close = true;
            }
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
        // On a phone the menus fold into the toolbar's menu button.
        if ctx.screen_rect().width() >= PHONE_W {
            egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu_bar(ui));
        }
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        self.tab_strip(ctx);
        if self.doc.is_some() && !self.doc_note.is_empty() && !self.note_hidden() {
            egui::TopBottomPanel::top("doc_note").show(ctx, |ui| self.note_bar(ui));
        }
        // The file is being written by this app: that is not a change on disk to warn about.
        if !saving {
            self.check_disk(ctx);
        }
        self.poll_update(ctx);
        self.banners(ctx);
        if std::mem::take(&mut self.need_cjk) {
            theme::enable_cjk(ctx);
        }
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
                let live = self.saving.is_none();
                let Some(Doc { content: Content::Text(text), .. }) = &mut self.doc else { return };
                let changed = ScrollArea::both()
                    .auto_shrink([false, false])
                    .show_locked(ui, |ui| ui.add(TextEdit::multiline(text).code_editor().interactive(live).desired_width(f32::INFINITY)).changed())
                    .inner;
                if changed {
                    self.edit_gen += 1;
                    self.recompute_dirty();
                }
            }
            _ => {
                let Some(Doc { content: Content::Binary(data), baseline: Content::Binary(base), .. }) = &self.doc else { return };
                if data.is_empty() {
                    ui.weak(tr("(empty file)"));
                } else {
                    hex_view::hex_grid(ui, data, base, &mut self.hex);
                }
            }
        });

        // Escape closes the topmost dialog.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_top_dialog();
        }

        // Dialogs.
        if self.pending_close {
            let mut choice = None;
            modal(ctx, tr("Unsaved changes"), |ui| {
                ui.label(tr("You have unsaved changes. Save them before closing this file?"));
                ui.add_space(8.0);
                choice = ui.horizontal(|ui| save_discard_cancel(ui, tr("Save and close"), tr("Close without saving"))).inner;
            });
            match choice {
                Some(0) => {
                    self.pending_close = false;
                    self.save_then(AfterSave::CloseFile);
                }
                Some(1) => {
                    self.pending_close = false;
                    self.close_file();
                }
                Some(_) => self.pending_close = false,
                None => {}
            }
        }
        if let Some(path) = self.pending_open.clone() {
            let mut choice = None;
            modal(ctx, tr("Unsaved changes"), |ui| {
                ui.label(crate::i18n::trf("You have unsaved changes. Save them before opening {0}?", &[&file_name(&path)]));
                ui.add_space(8.0);
                choice = ui.horizontal(|ui| save_discard_cancel(ui, tr("Save and open"), tr("Open without saving"))).inner;
            });
            match choice {
                Some(0) => {
                    self.pending_open = None;
                    self.save_then(AfterSave::Open(path));
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
        self.new_game_dialog(ctx);
        self.manager_window(ctx);
        self.rebind_dialog(ctx);
        self.patches_dialog(ctx);
        self.poll_loading(ctx, now);
        self.poll_saving(ctx, now);
        self.save_review_dialog(ctx);
        self.copy_slots_dialog(ctx);
        self.game_picker_dialog(ctx);
        self.preset_save_dialog(ctx);
        self.preset_apply_dialog(ctx, now);
        self.share_names_dialog(ctx);
        if self.saving.is_none() {
            self.poll_auto_refine(ctx, now);
        }
        self.compare_window(ctx, now);
        self.stale_dialog(ctx);
        self.quick_restore_dialog(ctx);
        self.prefs_window(ctx);
        self.help_window(ctx);
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
            modal(ctx, tr("Unsaved changes"), |ui| {
                ui.label(tr("You have unsaved changes. Save them before closing?"));
                ui.add_space(8.0);
                choice = ui.horizontal(|ui| save_discard_cancel(ui, tr("Save and close"), tr("Close without saving"))).inner;
            });
            match choice {
                Some(0) => {
                    self.confirm_close = false;
                    self.save_then(AfterSave::Quit);
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
        if std::mem::take(&mut self.close_now) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        theme::focus_ring(ctx);
        // A save started late in this frame (from a dialog): its box and outcome need frames.
        if self.saving.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
}

/// The "show in the file manager" command for the open file or (`backups`) its backups folder.
fn reveal_text(backups: bool) -> Option<&'static str> {
    platform::reveal_label()?;
    Some(if cfg!(windows) {
        if backups { tr("Show backups in Explorer") } else { tr("Show file in Explorer") }
    } else if cfg!(target_os = "macos") {
        if backups { tr("Show backups in Finder") } else { tr("Show file in Finder") }
    } else if backups {
        tr("Open backups folder")
    } else {
        tr("Open containing folder")
    })
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
        // Written beside it, flushed, read back and checked, and only then renamed over it.
        crate::backups::write_verified(path, bytes, true)
    }
}

fn v_when(v: &[crate::backups::Version], p: &PathBuf) -> u64 {
    v.iter().find(|x| x.path == *p).map(|x| x.when).unwrap_or(0)
}

fn modal(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(modal_key(), pass));
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("modal_dim")).fixed_pos(screen.min).sense(widgets::BLOCK_CLICKS).order(egui::Order::Middle).show(ctx, |ui| {
        ui.allocate_response(screen.size(), widgets::BLOCK_CLICKS);
        ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(140));
    });
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .max_width(fit(ctx, 640.0))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, add);
}

fn modal_key() -> egui::Id {
    egui::Id::new("modal_shown")
}

/// A dialog that blocks the window was drawn this frame or the one before: keys and dropped
/// files that would close or switch the tab under it are ignored, as clicks there are.
pub(crate) fn modal_shown(ctx: &egui::Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<u64>(modal_key())).is_some_and(|n| n + 1 >= pass)
}

/// The first and last control of the dialog showing now, for [`wrap_dialog_tab`].
#[derive(Clone, Copy)]
struct TabRing {
    first: egui::Id,
    last: egui::Id,
}

fn tab_ring_key() -> egui::Id {
    egui::Id::new("dialog_tab_ring")
}

/// Start of a frame: no dialog has said where its Tab order begins and ends yet.
fn begin_dialogs(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<TabRing>(tab_ring_key()));
}

/// Keeps Tab inside the dialog showing now: Tab on its `last` control goes round to its `first`,
/// and Shift+Tab on `first` to `last`. (Left to egui, Tab would pass through the controls behind
/// the dialog first.)
pub(crate) fn tab_ring(first: &egui::Response, last: &egui::Response) {
    let ring = TabRing { first: first.id, last: last.id };
    first.ctx.data_mut(|d| d.insert_temp(tab_ring_key(), ring));
}

/// Before egui sees a frame's input: takes a Tab that would leave the dialog of the last frame
/// and moves focus round to the dialog's other end instead, so no control behind the dialog
/// ever has focus, even for a frame (a screen reader would announce it, and an Enter in the
/// same input would press it).
fn wrap_dialog_tab(ctx: &egui::Context, raw: &mut egui::RawInput) {
    let Some(ring) = ctx.data(|d| d.get_temp::<TabRing>(tab_ring_key())) else { return };
    let Some(focused) = ctx.memory(|m| m.focused()) else { return };
    let Some(i) = raw.events.iter().position(|e| matches!(e, egui::Event::Key { key: egui::Key::Tab, pressed: true, .. })) else { return };
    let egui::Event::Key { modifiers, .. } = &raw.events[i] else { return };
    let to = match (modifiers.shift, focused) {
        (false, f) if f == ring.last => ring.first,
        (true, f) if f == ring.first => ring.last,
        _ => return,
    };
    raw.events.remove(i);
    ctx.memory_mut(|m| m.request_focus(to));
}

/// The buttons of an "Unsaved changes" dialog: save (0), drop the changes (1) or Cancel (2).
/// Focus starts on Cancel, and Tab goes round the three.
fn save_discard_cancel(ui: &mut egui::Ui, save: &str, discard: &str) -> Option<u8> {
    let mut choice = None;
    let first = ui.add(egui::Button::new(RichText::new(save).color(Color32::WHITE)).fill(theme::ACCENT));
    if first.clicked() {
        choice = Some(0);
    }
    if ui.button(discard).clicked() {
        choice = Some(1);
    }
    let cancel = safe_choice(ui.button(tr("Cancel")));
    if cancel.clicked() {
        choice = Some(2);
    }
    tab_ring(&first, &cancel);
    choice
}

/// Gives keyboard focus to a dialog's safe choice (`r`, its Cancel) whenever focus is outside the
/// dialog (as when it opens): Enter or Space can't press the destructive button, or the control
/// behind the dialog, by accident. Tab then goes round the dialog's controls (see [`tab_ring`]).
pub(crate) fn safe_choice(r: egui::Response) -> egui::Response {
    let focused = r.ctx.memory(|m| m.focused());
    let inside = focused.filter(|&id| id != r.layer_id.id.with("move")).and_then(|id| r.ctx.read_response(id)).is_some_and(|f| f.layer_id == r.layer_id);
    if !inside {
        r.request_focus();
    }
    r
}

/// How many found-save cards the game picker draws before offering "Show all".
const CARDS_SHOWN: usize = 60;

/// Draws the "Find my game's saves" window; returns the save picked to open and whether it was closed.
fn game_picker_window(ctx: &egui::Context, gp: &mut GamePicker) -> (Option<PathBuf>, bool) {
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
        egui::Window::new(tr("Find my game's saves"))
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let searching = found.iter().any(Option::is_none);
                let here = |i: usize| found[i].as_ref().is_some_and(|f| !f.is_empty());
                let any_here = (0..found.len()).any(here);
                let only = gp.only_found.unwrap_or(!searching && any_here);
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("Search:"));
                    ui.add(TextEdit::singleline(&mut gp.query).hint_text(tr("game name")).desired_width(200.0))
                        .on_hover_text(tr("Words in any order work (\"witcher 3\" or \"3 witcher\"), and so do short names made from the initials of the name with its numbers, such as bg3, gta5 or ff7."));
                    let mut o = only;
                    if crate::widgets::checkbox(ui, &mut o, tr("Only games found here"))
                        .on_hover_text(tr("Show only the games whose saves were found on this computer. Typing a name searches every game."))
                        .changed()
                    {
                        gp.only_found = Some(o);
                    }
                    // The box doesn't wrap by itself: start a new row when its (translated) text needs one.
                    let level_text = gp.level.map_or(tr("Any support"), |l| l.label());
                    let font = egui::TextStyle::Button.resolve(ui.style());
                    let need = ui.fonts(|f| f.layout_no_wrap(level_text.to_string(), font, Color32::WHITE).size().x) + 40.0;
                    if ui.available_size_before_wrap().x < need {
                        ui.end_row();
                    }
                    egui::ComboBox::from_id_salt("gp_level")
                        .wrap_mode(egui::TextWrapMode::Truncate)
                        .width(need.min(ui.available_size_before_wrap().x))
                        .selected_text(level_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut gp.level, None, tr("Any support"));
                            for l in crate::games::Level::ALL {
                                ui.selectable_value(&mut gp.level, Some(l), l.label()).on_hover_text(l.hint());
                            }
                        });
                    if ui
                        .add_enabled(!searching, egui::Button::new(tr("Search again")))
                        .on_hover_text(tr("The saves found before are remembered and checked first. Search again looks through all the usual places on this computer for games added since."))
                        .clicked()
                    {
                        spawn_search(&gp.found, true);
                        gp.save_groups = None;
                    }
                    // Inline (not pushed to the far edge) so it wraps with the rest instead of leaving the window.
                    if ui.button(tr("Close")).clicked() {
                        close = true;
                    }
                });
                if let Some(m) = gp.missing.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme::WARN, trf("That save was not found at {0}. It was moved or deleted, or its drive is not connected.", &[&m.display()]));
                        if ui.add_enabled(!searching, egui::Button::new(tr("Search the drives again"))).clicked() {
                            spawn_search(&gp.found, true);
                            gp.save_groups = None;
                            gp.missing = None;
                        }
                        if ui.button(tr("Dismiss")).clicked() {
                            gp.missing = None;
                        }
                    });
                }
                ui.weak(tr("Pick a game to see where it keeps its saves and the ones on this computer. Open\u{2026} and dragging a file onto the window still work for anything else."));
                ui.separator();
                let q = gp.query.trim().to_lowercase();
                let level = gp.level;
                let matches = |i: usize| {
                    let g = &crate::games::GAMES[i];
                    crate::games::name_matches(g.name, &q) && level.is_none_or(|l| crate::games::level(g) == l)
                };
                // Games with saves on this computer first, then the rest; a typed name looks through all of them.
                let found_here: Vec<usize> = (0..crate::games::GAMES.len()).filter(|&i| here(i) && matches(i)).collect();
                let others: Vec<usize> = if only && q.is_empty() { vec![] } else { (0..crate::games::GAMES.len()).filter(|&i| !here(i) && matches(i)).collect() };
                let hidden = if only && q.is_empty() { (0..crate::games::GAMES.len()).filter(|&i| !here(i) && matches(i)).count() } else { 0 };
                let mut show_all = false;
                let list = |ui: &mut egui::Ui, picked: &mut Option<usize>, show_all: &mut bool| {
                    ScrollArea::vertical().id_salt("gp_list").auto_shrink([false, false]).show_locked(ui, |ui| {
                        let row = |ui: &mut egui::Ui, i: usize, picked: &mut Option<usize>| {
                            let g = &crate::games::GAMES[i];
                            let n = match &found[i] {
                                Some(f) if f.is_empty() => tr("none found").to_string(),
                                Some(f) => trf("{0} found", &[&f.len()]),
                                None => tr("looking\u{2026}").to_string(),
                            };
                            ui.horizontal(|ui| {
                                let l = crate::games::level(g);
                                let color = match l {
                                    crate::games::Level::Screen => theme::OK,
                                    crate::games::Level::Values => theme::ACCENT,
                                    crate::games::Level::Hex => theme::WARN,
                                };
                                // The name gives way (with the whole text on hover) so the badge always stays beside it.
                                let chip_w = ui.fonts(|f| f.layout_no_wrap(l.label().to_string(), egui::FontId::proportional(13.0), color).size().x) + 16.0;
                                let name_w = (ui.available_width() - chip_w - ui.spacing().item_spacing.x).max(60.0);
                                let text = format!("{}  \u{00B7}  {n}", g.name);
                                let r = ui
                                    .allocate_ui_with_layout(egui::vec2(name_w, ui.spacing().interact_size.y), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                                        ui.selectable_label(*picked == Some(i), text.clone())
                                    })
                                    .inner;
                                if r.clicked() {
                                    *picked = Some(i);
                                }
                                r.on_hover_text(text);
                                App::chip(ui, l.label(), color).on_hover_text(l.hint());
                            });
                        };
                        if !found_here.is_empty() {
                            ui.label(RichText::new(tr("On this computer")).strong());
                            for &i in &found_here {
                                row(ui, i, picked);
                            }
                            if !others.is_empty() {
                                ui.add_space(6.0);
                                ui.label(RichText::new(tr("Other games")).strong());
                            }
                        } else if searching && only && q.is_empty() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.weak(tr("Looking for saves on this computer\u{2026}"));
                            });
                        }
                        // Grouped by first letter (digits and symbols under #), collapsed unless a name is typed.
                        let letter = |i: usize| -> char {
                            let c = crate::games::GAMES[i].name.chars().next().unwrap_or('#').to_uppercase().next().unwrap_or('#');
                            if c.is_alphabetic() { c } else { '#' }
                        };
                        // PC games go under their first letter; console games under their console below.
                        let mut groups: Vec<(char, Vec<usize>)> = vec![];
                        let mut by_console: Vec<(&'static str, Vec<usize>)> = vec![];
                        for &i in &others {
                            let consoles = crate::games::consoles_of(crate::games::GAMES[i].name);
                            if consoles.is_empty() {
                                let c = letter(i);
                                match groups.iter_mut().find(|(k, _)| *k == c) {
                                    Some((_, v)) => v.push(i),
                                    None => groups.push((c, vec![i])),
                                }
                            }
                            for c in consoles {
                                match by_console.iter_mut().find(|(k, _)| *k == c) {
                                    Some((_, v)) => v.push(i),
                                    None => by_console.push((c, vec![i])),
                                }
                            }
                        }
                        groups.sort_by_key(|(c, _)| (*c != '#', *c));
                        by_console.sort_by_key(|(c, _)| crate::games::CONSOLES.iter().position(|(n, _)| n == c));
                        for (c, members) in &groups {
                            let title = format!("{c}  ({})", members.len());
                            let mut h = egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(("gp_letter", *c, found_here.is_empty()));
                            h = if q.is_empty() { h.default_open(false) } else { h.open(Some(true)) };
                            h.show(ui, |ui| {
                                for &i in members {
                                    row(ui, i, picked);
                                }
                            });
                        }
                        if !by_console.is_empty() {
                            ui.add_space(6.0);
                            ui.label(RichText::new(tr("Console games")).strong());
                            for (c, members) in &by_console {
                                let title = format!("{c}  ({})", members.len());
                                let mut h = egui::CollapsingHeader::new(RichText::new(title).strong()).id_salt(("gp_console", *c, found_here.is_empty()));
                                h = if q.is_empty() { h.default_open(false) } else { h.open(Some(true)) };
                                h.show(ui, |ui| {
                                    for &i in members {
                                        row(ui, i, picked);
                                    }
                                });
                            }
                        }
                        if hidden > 0 {
                            ui.add_space(6.0);
                            if ui.link(trf("Show the other {0} games", &[&hidden])).clicked() {
                                *show_all = true;
                            }
                        }
                        if found_here.is_empty() && others.is_empty() && hidden == 0 {
                            ui.weak(tr("No game matches. Use Open\u{2026} to pick the file yourself."));
                        }
                    });
                };
                let details_body = |ui: &mut egui::Ui, picked: Option<usize>, group: &mut Option<String>, open: &mut Option<PathBuf>, save_groups: &mut Option<((usize, usize), Vec<String>)>| {
                    let Some(i) = picked else {
                        ui.weak(tr("Choose a game from the list."));
                        return;
                    };
                    let g = &crate::games::GAMES[i];
                    ui.heading(g.name);
                    for (k, v) in [(tr("Usually in"), crate::games::folder_text(g)), (tr("Files"), crate::games::files_text(g)), (tr("Editing"), crate::games::support_text(g))] {
                        widgets::label_text(ui, &format!("{k}:"), &v, false);
                    }
                    ui.add_space(6.0);
                    match &found[i] {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(tr("Looking for saves\u{2026}"));
                            });
                        }
                        Some(f) if f.is_empty() => {
                            ui.colored_label(theme::fg(ui, theme::WARN), tr("No saves found on this computer."));
                            ui.weak(tr("If the game keeps them somewhere else (another drive, a cloud folder), use Open\u{2026} to pick the file."));
                        }
                        Some(f) => {
                            let cards = |ui: &mut egui::Ui, saves: &[&crate::games::Found], open: &mut Option<PathBuf>| {
                                {
                                    // Many saves: the first 60, with a link for the rest (the cards vary in height).
                                    let all_id = ui.id().with("gp_cards_all");
                                    let all = ui.data(|d| d.get_temp::<bool>(all_id)).unwrap_or(false);
                                    let shown = if all { saves.len() } else { saves.len().min(CARDS_SHOWN) };
                                    for s in &saves[..shown] {
                                        theme::card(ui, |ui| {
                                            if ui.link(RichText::new(s.label.clone().unwrap_or_else(|| file_name(&s.path))).strong()).clicked() {
                                                *open = Some(s.path.clone());
                                            }
                                            let when = s
                                                .modified
                                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                                .map(|d| crate::backups::describe(d.as_secs()))
                                                .unwrap_or_default();
                                            let mut line = trf("{0}  \u{00B7}  {1}", &[&when, &human_size(s.size as usize)]);
                                            if let Some(n) = large_save_note(s.size) {
                                                line.push_str(&format!("  \u{00B7}  {n}"));
                                            }
                                            ui.add(egui::Label::new(RichText::new(line).weak()).truncate());
                                            ui.add(egui::Label::new(RichText::new(s.path.parent().map(|p| p.display().to_string()).unwrap_or_default()).weak().size(12.0)).truncate());
                                        });
                                    }
                                    if shown < saves.len() && ui.link(trf("Show all {0}", &[&saves.len()])).clicked() {
                                        ui.data_mut(|d| d.insert_temp(all_id, true));
                                    }
                                }
                            };
                            if !crate::games::many_games(g) {
                                ui.label(if f.len() == 1 { trf("{0} save found \u{2014} click one to open it:", &[&f.len()]) } else { trf("{0} saves found \u{2014} click one to open it:", &[&f.len()]) });
                                let all: Vec<&crate::games::Found> = f.iter().collect();
                                cards(ui, &all, open);
                                return;
                            }
                            // One entry per game, most recently played first.
                            let key = (i, f.len());
                            if save_groups.as_ref().map(|c| c.0) != Some(key) {
                                *save_groups = Some((key, f.iter().map(|s| crate::games::group_of(g, &s.path)).collect()));
                            }
                            let names = &save_groups.as_ref().unwrap().1;
                            let mut groups: Vec<(String, Vec<&crate::games::Found>)> = vec![];
                            for (s, name) in f.iter().zip(names) {
                                let name = name.clone();
                                match groups.iter_mut().find(|(n, _)| *n == name) {
                                    Some((_, v)) => v.push(s),
                                    None => groups.push((name, vec![s])),
                                }
                            }
                            if let Some(name) = group.clone() {
                                if let Some((_, saves)) = groups.iter().find(|(n, _)| *n == name) {
                                    if ui.button(trf("\u{2039} All {0} games", &[&groups.len()])).clicked() {
                                        *group = None;
                                    }
                                    ui.strong(&name);
                                    ui.label(if saves.len() == 1 { trf("{0} save \u{2014} click one to open it:", &[&saves.len()]) } else { trf("{0} saves \u{2014} click one to open it:", &[&saves.len()]) });
                                    cards(ui, saves, open);
                                    return;
                                }
                                *group = None;
                            }
                            ui.label(if groups.len() == 1 { trf("{0} game with saves ({1} saves) \u{2014} pick one:", &[&groups.len(), &f.len()]) } else { trf("{0} games with saves ({1} saves) \u{2014} pick one:", &[&groups.len(), &f.len()]) });
                            {
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
                                        ui.weak(if saves.len() == 1 { trf("{0} save  \u{00B7}  last saved {1}", &[&saves.len(), &when]) } else { trf("{0} saves  \u{00B7}  last saved {1}", &[&saves.len(), &when]) });
                                    });
                                }
                            }
                        }
                    }
                };
                // The whole right-hand side scrolls as one, so a long description never pushes the window past the screen.
                let details = |ui: &mut egui::Ui, picked: Option<usize>, group: &mut Option<String>, open: &mut Option<PathBuf>, save_groups: &mut Option<((usize, usize), Vec<String>)>| {
                    ScrollArea::vertical().id_salt(("gp_details", picked, group.clone())).auto_shrink([false, false]).show_locked(ui, |ui| details_body(ui, picked, group, open, save_groups));
                };
                let before = gp.picked;
                if wide {
                    ui.columns(2, |c| {
                        list(&mut c[0], &mut gp.picked, &mut show_all);
                        details(&mut c[1], gp.picked, &mut gp.group, &mut open, &mut gp.save_groups);
                    });
                } else if gp.picked.is_none() {
                    list(ui, &mut gp.picked, &mut show_all);
                } else {
                    if ui.button(tr("\u{2039} All games")).clicked() {
                        gp.picked = None;
                    }
                    details(ui, gp.picked, &mut gp.group, &mut open, &mut gp.save_groups);
                }
                if gp.picked != before {
                    gp.group = None;
                }
                if show_all {
                    gp.only_found = Some(false);
                }
            });

    (open, close)
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

/// True (and forgets it) when `ptr` is the field that put up the current error.
fn take_if_same(slot: &mut Option<String>, ptr: &str) -> bool {
    if slot.as_deref() == Some(ptr) {
        *slot = None;
        true
    } else {
        false
    }
}

/// Keyboard focus in an "Unsaved changes" dialog: it starts on Cancel, Tab goes round the
/// dialog's buttons, and never lands on (or passes through) the controls behind it.
#[cfg(test)]
mod dialog_focus_tests {
    use super::*;

    fn tab(shift: bool) -> egui::Event {
        let modifiers = if shift { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
        egui::Event::Key { key: egui::Key::Tab, physical_key: Some(egui::Key::Tab), pressed: true, repeat: false, modifiers }
    }

    /// One frame of a window with a button behind the dialog. Returns where the focused control
    /// is (its left edge and layer), if anything has focus.
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>) -> Option<(f32, egui::Order)> {
        let mut raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0))), ..Default::default() };
        wrap_dialog_tab(ctx, &mut raw);
        let _ = ctx.run(raw, |ctx| {
            begin_dialogs(ctx);
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.button("Behind 1");
                let _ = ui.button("Behind 2");
            });
            modal(ctx, "Unsaved changes", |ui| {
                ui.horizontal(|ui| save_discard_cancel(ui, "Save", "Don't save"));
            });
        });
        let id = ctx.memory(|m| m.focused())?;
        ctx.read_response(id).map(|r| (r.rect.left(), r.layer_id.order))
    }

    #[test]
    fn tab_goes_round_the_dialog_starting_on_cancel() {
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&ctx, vec![]);
        }
        let start = frame(&ctx, vec![]).expect("the dialog gives focus to Cancel");
        assert_eq!(start.1, egui::Order::Foreground, "focus starts in the dialog");
        // A full round and a bit: Cancel, Save, Don't save, Cancel, Save.
        let mut xs = vec![start.0];
        for _ in 0..4 {
            let (x, order) = frame(&ctx, vec![tab(false)]).expect("something keeps focus");
            assert_eq!(order, egui::Order::Foreground, "focus left the dialog");
            xs.push(x);
            // The next frame without input must not move it either (egui hands focus on a frame late).
            let (x2, order2) = frame(&ctx, vec![]).unwrap();
            assert_eq!((x2, order2), (x, order), "focus moved without a key");
        }
        let (cancel, save, discard) = (xs[0], xs[1], xs[2]);
        assert!(save < discard && discard < cancel, "order is Save, Don't save, Cancel: {xs:?}");
        assert_eq!(xs[3], cancel, "Tab from Don't save reaches Cancel");
        assert_eq!(xs[4], save, "Tab from Cancel goes round to Save, the first button");
        // Shift+Tab from the first button goes round to the last.
        let (x, order) = frame(&ctx, vec![tab(true)]).unwrap();
        assert_eq!((x, order), (cancel, egui::Order::Foreground));
    }
}

#[cfg(test)]
mod edit_error_tests {
    #[test]
    fn error_clears_only_for_its_own_field() {
        let mut slot = Some("/level".to_string());
        assert!(!super::take_if_same(&mut slot, "/gold"), "another field's fix keeps the error");
        assert!(super::take_if_same(&mut slot, "/level"), "fixing the failing field clears it");
        assert!(slot.is_none());
        assert!(!super::take_if_same(&mut slot, "/level"));
    }
}

/// Saving on a worker thread: the same file and backups as saving on the window's thread,
/// nothing can change the file while it is written, and no failure loses the changes.
#[cfg(test)]
mod save_tests {
    use super::*;
    use eframe::App as _;

    /// A fresh folder for one test, removed when it ends.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("use-bgsave-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Scratch(d)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Backups go beside the save (no chosen folder), and messages are in English, for the
    /// whole test.
    fn locks() -> (std::sync::MutexGuard<'static, ()>, std::sync::MutexGuard<'static, ()>) {
        let b = crate::backups::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::backups::set_root(None);
        let l = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("en"));
        (b, l)
    }

    /// A JSON save over the background thresholds (file size and number of values).
    fn big_json() -> Vec<u8> {
        let items: Vec<Value> = (0..12_000).map(|i| json!({"name": format!("Item {i}"), "count": i})).collect();
        serde_json::to_vec_pretty(&json!({"gold": 100, "hero": {"name": "Ann", "level": 3}, "items": items})).unwrap()
    }

    /// A window with the app's fonts and look.
    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        theme::apply(&ctx, true);
        ctx
    }

    fn opened(ctx: &egui::Context, path: &Path) -> App {
        let mut app = App::with(ctx, Settings::default(), false);
        app.load(path);
        assert!(matches!(app.doc.as_ref().map(|d| &d.content), Some(Content::Json { .. })), "{}", app.status);
        app
    }

    /// One frame, with the input going through `raw_input_hook` as eframe does.
    fn frame(app: &mut App, ctx: &egui::Context, mut raw: egui::RawInput) -> Vec<egui::ViewportCommand> {
        app.raw_input_hook(ctx, &mut raw);
        let out = ctx.run(raw, |ctx| app.frame(ctx));
        out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default()
    }

    /// Runs frames until the save is in; returns every window command sent meanwhile.
    fn wait(app: &mut App, ctx: &egui::Context) -> Vec<egui::ViewportCommand> {
        let mut cmds = vec![];
        for _ in 0..4000 {
            cmds.extend(frame(app, ctx, egui::RawInput::default()));
            if app.saving.is_none() {
                return cmds;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("the save never finished");
    }

    fn value(app: &App, ptr: &str) -> Option<Value> {
        app.doc.as_ref().and_then(|d| root(&d.content)).and_then(|r| r.pointer(ptr)).cloned()
    }

    /// What the Values list shows: each value's section, name, place and whether it can be edited.
    fn rows(m: &Model) -> Vec<(String, String, String, String, bool)> {
        m.rows.iter().map(|r| (r.ptr.to_string(), m.sections[r.section].title.clone(), r.name().to_string(), r.context().to_string(), r.editable)).collect()
    }

    #[test]
    fn small_saves_stay_on_the_window_thread_and_big_ones_do_not() {
        let _l = locks();
        let s = Scratch::new("threshold");
        let ctx = window();
        let small = s.0.join("small.json");
        std::fs::write(&small, br#"{"gold": 1}"#).unwrap();
        let mut app = opened(&ctx, &small);
        assert!(!app.save_in_background());
        app.apply_edit("/gold".into(), json!(2), 0.0);
        app.save_to(&small);
        assert!(app.saving.is_none(), "written before save_to returned");
        assert!(!app.dirty);
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&small).unwrap()).unwrap()["gold"], json!(2));
        let big = s.0.join("big.json");
        std::fs::write(&big, big_json()).unwrap();
        assert!(opened(&ctx, &big).save_in_background());
    }

    #[test]
    fn a_background_save_writes_the_same_file_and_backups() {
        let _l = locks();
        let ctx = window();
        let original = big_json();
        let mut done = vec![];
        for background in [false, true] {
            let s = Scratch::new(if background { "same-bg" } else { "same-fg" });
            let path = s.0.join("save.json");
            std::fs::write(&path, &original).unwrap();
            let mut app = opened(&ctx, &path);
            app.apply_edit("/gold".into(), json!(999_999), 0.0);
            app.apply_edit("/items/7/count".into(), json!(-5), 0.0);
            app.save_with(&path, AfterSave::Stay, background);
            assert_eq!(app.saving.is_some(), background);
            wait(&mut app, &ctx);
            assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
            assert!(!app.dirty);
            let d = app.doc.as_ref().unwrap();
            assert!(formats::same_data(&d.content, &d.baseline));
            assert_eq!(d.stamp, safety::stamp(&path));
            let backups: Vec<Vec<u8>> = crate::backups::list(&path).iter().map(|v| std::fs::read(&v.path).unwrap()).collect();
            let bak = std::fs::read(s.0.join("save.json.bak")).unwrap();
            done.push((std::fs::read(&path).unwrap(), backups, bak, app.status.clone(), app.undo.len(), rows(&app.model)));
        }
        let (fg, bg) = (&done[0], &done[1]);
        assert_eq!(fg.0, bg.0, "byte-identical files");
        assert_ne!(fg.0, original);
        assert_eq!(fg.1, vec![original.clone()], "the file as it was is the one backup");
        assert_eq!(fg.1, bg.1);
        assert_eq!((&fg.2, &bg.2), (&original, &original), "the .bak copy holds the original");
        assert_eq!((&fg.3, fg.4, &fg.5), (&bg.3, bg.4, &bg.5), "same message, undo steps and Values list");
    }

    #[test]
    fn a_failed_background_save_leaves_the_file_and_the_changes() {
        let _l = locks();
        let s = Scratch::new("fail");
        let ctx = window();
        let path = s.0.join("save.json");
        let original = big_json();
        std::fs::write(&path, &original).unwrap();
        let mut app = opened(&ctx, &path);
        let stamp = app.doc.as_ref().unwrap().stamp.clone();
        app.apply_edit("/gold".into(), json!(5), 0.0);
        // The temporary file the new save is written to can't be made.
        std::fs::create_dir_all(s.0.join("save.json.partial").join("inside")).unwrap();
        app.save_with(&path, AfterSave::Stay, true);
        assert!(app.saving.is_some());
        wait(&mut app, &ctx);
        assert_eq!(std::fs::read(&path).unwrap(), original, "the file on disk is untouched");
        assert_eq!(app.status_kind, StatusKind::Err);
        assert!(app.status.starts_with("Couldn't write"), "{}", app.status);
        assert!(app.banner.is_some(), "shown where it can't be missed");
        assert!(app.dirty, "the changes still count as unsaved");
        assert_eq!(value(&app, "/gold"), Some(json!(5)));
        assert_eq!(app.doc.as_ref().unwrap().stamp, stamp);
        // Once the way is clear, saving again works.
        std::fs::remove_dir_all(s.0.join("save.json.partial")).unwrap();
        app.save_with(&path, AfterSave::Stay, true);
        wait(&mut app, &ctx);
        assert!(!app.dirty, "{}", app.status);
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(5));
    }

    #[test]
    fn nothing_changes_the_file_while_it_is_being_saved() {
        let _l = locks();
        let s = Scratch::new("held");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, big_json()).unwrap();
        let other = s.0.join("other.json");
        std::fs::write(&other, br#"{"gold": 1}"#).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(777), 0.0);
        app.save_with(&path, AfterSave::Stay, true);
        assert!(app.saving.is_some());
        let gen = app.edit_gen;
        // Every way the program changes the file, opens another or starts another save.
        app.apply_edit("/gold".into(), json!(1), 1.0);
        assert_eq!(value(&app, "/gold"), Some(json!(777)));
        assert_eq!(app.paste_values(&[("/gold".into(), json!(2))]), (0, 1));
        app.undo_redo(true);
        app.revert_all();
        app.open_in_tab(other.clone());
        app.request_open(other.clone());
        app.load_async(&other);
        app.load(&other);
        app.close_tab(0);
        app.close_file();
        app.save_clicked();
        app.save_with(&path, AfterSave::Stay, false);
        assert_eq!(app.tab_count(), 1);
        assert_eq!(app.edit_gen, gen);
        assert_eq!(value(&app, "/gold"), Some(json!(777)));
        assert_eq!(app.doc.as_ref().unwrap().path, path);
        // Keys, typed text, clicks and dropped files never reach the window.
        let mut raw = egui::RawInput::default();
        raw.events = vec![
            egui::Event::Key { key: egui::Key::Z, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND },
            egui::Event::Text("9".into()),
            egui::Event::PointerButton { pos: egui::pos2(10.0, 10.0), button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE },
            egui::Event::PointerMoved(egui::pos2(10.0, 10.0)),
        ];
        raw.dropped_files.push(egui::DroppedFile { path: Some(other.clone()), ..Default::default() });
        app.raw_input_hook(&ctx, &mut raw);
        assert_eq!(raw.events, vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))]);
        assert!(raw.dropped_files.is_empty());
        wait(&mut app, &ctx);
        assert!(!app.dirty, "{}", app.status);
        assert_eq!(app.undo.len(), 1, "the edit made before saving is still the one undo step");
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(777));
        // And afterwards everything works again.
        app.apply_edit("/gold".into(), json!(1), 2.0);
        assert_eq!(value(&app, "/gold"), Some(json!(1)));
    }

    #[test]
    fn a_change_that_slips_in_during_a_save_stays_unsaved() {
        let _l = locks();
        let s = Scratch::new("slip");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, big_json()).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(10), 0.0);
        app.save_with(&path, AfterSave::CloseFile, true);
        // Past every guard, as a bug might: the value on screen changes mid-save.
        *app.root_mut().unwrap().pointer_mut("/gold").unwrap() = json!(20);
        app.touched.insert("/gold");
        app.edit_gen += 1;
        wait(&mut app, &ctx);
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(10), "the file holds what was saved");
        assert_eq!(value(&app, "/gold"), Some(json!(20)), "what is shown is kept");
        assert!(app.dirty, "and still counts as unsaved");
        assert!(app.doc.is_some(), "the file wasn't closed");
        assert_eq!(app.status_kind, StatusKind::Err);
        assert_eq!(app.doc.as_ref().unwrap().stamp, safety::stamp(&path), "not mistaken for a change on disk");
        app.save_with(&path, AfterSave::Stay, true);
        wait(&mut app, &ctx);
        assert!(!app.dirty);
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(20));
    }

    /// What the Changes panel's group Revert applies: the original of every value still differing.
    fn originals(app: &App, ptrs: &[&str]) -> Vec<(String, Value)> {
        let b = root(&app.doc.as_ref().unwrap().baseline).unwrap();
        ptrs.iter().map(|p| (p.to_string(), b.pointer(p).unwrap().clone())).collect()
    }

    #[test]
    fn a_mass_edit_is_one_undo_step_and_one_change_group() {
        let _l = locks();
        let s = Scratch::new("group");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, big_json()).unwrap();
        let mut app = opened(&ctx, &path);
        let original = app.doc.as_ref().unwrap().content.to_bytes().unwrap();
        let ptrs = ["/gold", "/hero/level", "/items/3/count", "/items/9/count"];
        app.apply_edits(ptrs.iter().map(|p| (p.to_string(), json!(9999))).collect(), 0.0);
        assert_eq!(app.undo.len(), 1, "one button press, one undo step");
        let ids: HashSet<_> = ptrs.iter().map(|p| app.touched.group.get(*p).copied()).collect();
        assert_eq!(ids.len(), 1, "all in one group");
        assert!(ids.iter().next().unwrap().is_some());
        // Undo puts every value back, byte for byte, and Redo makes them all again.
        app.undo_redo(true);
        assert!(!app.dirty);
        assert_eq!(app.doc.as_ref().unwrap().content.to_bytes().unwrap(), original);
        app.undo_redo(false);
        for p in ptrs {
            assert_eq!(value(&app, p), Some(json!(9999)));
        }
        // The group's Revert in the Changes panel: one step, exact original.
        app.refresh_changes();
        assert_eq!(app.changes.ptrs.len(), 4);
        let back = originals(&app, &ptrs);
        app.apply_edits(back, f64::NEG_INFINITY);
        assert!(!app.dirty);
        assert_eq!(app.doc.as_ref().unwrap().content.to_bytes().unwrap(), original);
        assert_eq!(app.undo.len(), 2, "the revert is a single step too");
        app.undo_redo(true);
        for p in ptrs {
            assert_eq!(value(&app, p), Some(json!(9999)), "undoing the revert brings them all back");
        }
    }

    #[test]
    fn a_single_edit_stays_on_its_own_and_leaves_its_group_when_edited_again() {
        let _l = locks();
        let s = Scratch::new("single");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, big_json()).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edits(vec![("/gold".into(), json!(5))], 0.0);
        assert!(app.touched.group.is_empty(), "one edit is no group");
        assert_eq!(app.undo.len(), 1);
        app.apply_edits(vec![("/gold".into(), json!(6)), ("/hero/level".into(), json!(7))], 10.0);
        assert_eq!(app.touched.group.len(), 2);
        // Edited alone afterwards, one value is no longer part of the group.
        app.apply_edits(vec![("/gold".into(), json!(8))], 20.0);
        assert!(!app.touched.group.contains_key("/gold"));
        assert!(app.touched.group.contains_key("/hero/level"));
        // Saving starts afresh.
        app.revert_all();
        assert!(app.touched.group.is_empty());
    }

        #[test]
    fn closing_the_window_during_a_save_waits_for_it() {
        let _l = locks();
        let s = Scratch::new("close");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, big_json()).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(31), 0.0);
        app.save_with(&path, AfterSave::Stay, true);
        let mut raw = egui::RawInput::default();
        raw.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo { events: vec![egui::ViewportEvent::Close], ..Default::default() });
        let first = frame(&mut app, &ctx, raw);
        if app.saving.is_some() {
            assert!(first.contains(&egui::ViewportCommand::CancelClose), "{first:?}");
            assert!(!first.contains(&egui::ViewportCommand::Close));
        }
        let mut cmds = first;
        cmds.extend(wait(&mut app, &ctx));
        assert!(cmds.contains(&egui::ViewportCommand::Close), "closes once saved: {cmds:?}");
        assert!(!app.any_dirty());
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(31));
    }

    #[test]
    fn save_and_quit_in_the_background_closes_only_once_saved() {
        let _l = locks();
        let s = Scratch::new("quit");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, big_json()).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(32), 0.0);
        app.save_with(&path, AfterSave::Quit, true);
        assert!(!app.close_now && !app.allow_close);
        let cmds = wait(&mut app, &ctx);
        assert_eq!(cmds.iter().filter(|c| **c == egui::ViewportCommand::Close).count(), 1, "{cmds:?}");
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(32));
        // A save that fails doesn't quit: the question comes back instead.
        app.apply_edit("/gold".into(), json!(33), 1.0);
        std::fs::create_dir_all(s.0.join("save.json.partial").join("x")).unwrap();
        app.allow_close = false;
        app.save_with(&path, AfterSave::Quit, true);
        let cmds = wait(&mut app, &ctx);
        assert!(!cmds.contains(&egui::ViewportCommand::Close), "{cmds:?}");
        assert!(app.dirty);
    }

    #[test]
    fn save_and_quit_with_two_unsaved_tabs_asks_about_the_second() {
        let _l = locks();
        let s = Scratch::new("quit2");
        let ctx = window();
        let (a, b) = (s.0.join("a.json"), s.0.join("b.json"));
        std::fs::write(&a, br#"{"gold": 1}"#).unwrap();
        std::fs::write(&b, br#"{"gold": 1}"#).unwrap();
        let mut app = opened(&ctx, &a);
        app.apply_edit("/gold".into(), json!(2), 0.0);
        app.open_in_tab(b.clone());
        for _ in 0..400 {
            if app.loading.is_none() {
                break;
            }
            frame(&mut app, &ctx, egui::RawInput::default());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(b.clone()), "{}", app.status);
        app.apply_edit("/gold".into(), json!(3), 1.0);
        app.save_then(AfterSave::Quit);
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&b).unwrap()).unwrap()["gold"], json!(3));
        // The other tab is still unsaved: no closing yet, the question comes back for it.
        assert!(!app.close_now && !app.allow_close && app.confirm_close);
        assert!(app.dirty, "the unsaved tab is shown");
        app.confirm_close = false;
        app.save_then(AfterSave::Quit);
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&a).unwrap()).unwrap()["gold"], json!(2));
        assert!(app.close_now && app.allow_close);
    }

    #[test]
    fn nothing_is_saved_while_a_file_is_being_opened() {
        let _l = locks();
        let s = Scratch::new("loading");
        let ctx = window();
        let path = s.0.join("save.json");
        let original = big_json();
        std::fs::write(&path, &original).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(7), 0.0);
        app.load_async(&path);
        assert!(app.loading.is_some(), "big files are read on a worker thread");
        app.save_clicked();
        app.save_with(&path, AfterSave::Stay, true);
        app.save_with(&path, AfterSave::Stay, false);
        assert!(app.saving.is_none() && app.save_review.is_none());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }

    #[test]
    fn a_save_thread_that_panics_is_reported_and_the_changes_stay() {
        let _l = locks();
        let s = Scratch::new("panic");
        let ctx = window();
        let path = s.0.join("save.json");
        let original = big_json();
        std::fs::write(&path, &original).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(6), 0.0);
        app.close_after_save = true;
        app.spawn_save(&path, AfterSave::CloseFile, || panic!("a bug in the save code"));
        let cmds = wait(&mut app, &ctx);
        assert_eq!(app.status_kind, StatusKind::Err);
        assert!(app.banner.is_some());
        assert!(app.dirty);
        assert!(app.doc.is_some(), "not closed");
        assert!(!cmds.contains(&egui::ViewportCommand::Close), "asked about the unsaved changes instead of closing");
        assert!(app.confirm_close);
        assert_eq!(value(&app, "/gold"), Some(json!(6)));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        // Saving works again afterwards.
        app.confirm_close = false;
        app.save_with(&path, AfterSave::Stay, true);
        wait(&mut app, &ctx);
        assert!(!app.dirty, "{}", app.status);
    }

    #[test]
    fn the_values_list_is_kept_only_when_it_would_come_out_the_same() {
        let _l = locks();
        let s = Scratch::new("model");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, big_json()).unwrap();
        let mut app = opened(&ctx, &path);
        let fresh = |app: &App| {
            let d = app.doc.as_ref().unwrap();
            rows(&values_view::build(root(&d.content).unwrap(), &d.names, None))
        };
        // A number: the list is the same, so it is kept.
        let gen = app.model_gen;
        app.apply_edit("/gold".into(), json!(1), 0.0);
        app.save_with(&path, AfterSave::Stay, true);
        wait(&mut app, &ctx);
        assert_eq!(app.model_gen, gen, "not built again");
        assert_eq!(rows(&app.model), fresh(&app));
        // A name an object is listed by: built again.
        app.apply_edit("/hero/name".into(), json!("Bea"), 1.0);
        app.save_with(&path, AfterSave::Stay, false);
        assert_ne!(app.model_gen, gen);
        assert_eq!(rows(&app.model), fresh(&app));
        assert!(app.model.sections.iter().any(|s| s.title.contains("Bea")));
        // A list built again since the file was saved (as after picking the data folder) is
        // never taken to be the saved file's.
        let gen = app.model_gen;
        app.rebuild_model();
        app.apply_edit("/gold".into(), json!(2), 2.0);
        app.save_with(&path, AfterSave::Stay, false);
        assert_eq!(app.model_gen, gen + 2);
        assert_eq!(rows(&app.model), fresh(&app));
    }

    fn gold_on_disk(path: &Path) -> Value {
        serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()["gold"].clone()
    }

    fn backed_up(path: &Path) -> Vec<Vec<u8>> {
        crate::backups::list(path).iter().map(|v| std::fs::read(&v.path).unwrap()).collect()
    }

    #[test]
    fn a_save_never_silently_writes_over_a_file_the_game_changed() {
        let _l = locks();
        let s = Scratch::new("stale");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(2), 0.0);
        // The game autosaves while the file is open.
        let newer = br#"{"gold": 50, "new": true}"#;
        std::fs::write(&path, newer).unwrap();
        app.save_to(&path);
        assert_eq!(std::fs::read(&path).unwrap(), newer, "the game's newer save is left alone");
        assert_eq!(app.stale_save.as_deref(), Some(path.as_path()), "the player is asked first");
        assert!(app.dirty);
        // "Save anyway": the newer file is backed up before the player's version goes over it.
        app.stale_save = None;
        app.stale_ok = true;
        app.save_to(&path);
        assert!(!app.stale_ok);
        assert_eq!(gold_on_disk(&path), json!(2), "{}", app.status);
        assert!(backed_up(&path).contains(&newer.to_vec()), "the game's save can be brought back");
        assert!(!app.dirty);
        // Moved or deleted meanwhile: asked about too, and "Save anyway" puts it back.
        app.apply_edit("/gold".into(), json!(3), 1.0);
        std::fs::remove_file(&path).unwrap();
        app.save_to(&path);
        assert!(!path.exists());
        assert!(app.stale_save.is_some());
        app.stale_save = None;
        app.stale_ok = true;
        app.save_to(&path);
        assert_eq!(gold_on_disk(&path), json!(3));
    }

    #[test]
    fn an_outside_change_that_hides_from_the_check_is_still_backed_up() {
        let _l = locks();
        let s = Scratch::new("hidden-change");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        let mut app = opened(&ctx, &path);
        let when = std::fs::metadata(&path).unwrap().modified().unwrap();
        app.apply_edit("/gold".into(), json!(2), 0.0);
        // Same size and, on a file system with coarse times, the same time: not noticed.
        let newer = br#"{"gold": 9}"#;
        std::fs::write(&path, newer).unwrap();
        std::fs::OpenOptions::new().write(true).open(&path).unwrap().set_modified(when).unwrap();
        assert!(!safety::changed(&path, app.doc.as_ref().unwrap().stamp.as_ref()));
        app.save_to(&path);
        assert_eq!(gold_on_disk(&path), json!(2));
        assert!(backed_up(&path).contains(&newer.to_vec()), "what was on disk can still be brought back");
    }

    #[cfg(unix)]
    #[test]
    fn a_save_whose_first_backup_cant_be_made_writes_nothing() {
        let _l = locks();
        let s = Scratch::new("no-bak");
        let ctx = window();
        let path = s.0.join("save.json");
        let original = br#"{"gold": 1}"#;
        std::fs::write(&path, original).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(2), 0.0);
        // The ".bak" copy leads nowhere, so it can't be written.
        std::os::unix::fs::symlink(s.0.join("no such folder").join("x"), s.0.join("save.json.bak")).unwrap();
        app.save_to(&path);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(app.status_kind, StatusKind::Err);
        assert!(app.status.starts_with("Couldn't save: the backup"), "{}", app.status);
        assert!(app.dirty);
        assert_eq!(value(&app, "/gold"), Some(json!(2)));
    }

    #[test]
    fn saving_again_and_undoing_after_a_save_keep_every_version() {
        let _l = locks();
        let s = Scratch::new("twice");
        let ctx = window();
        let path = s.0.join("save.json");
        let original = br#"{"gold": 1}"#.to_vec();
        std::fs::write(&path, &original).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(2), 0.0);
        app.save_to(&path);
        let first = std::fs::read(&path).unwrap();
        app.apply_edit("/gold".into(), json!(3), 10.0);
        app.save_to(&path);
        assert_eq!(gold_on_disk(&path), json!(3));
        assert_eq!(std::fs::read(s.0.join("save.json.bak")).unwrap(), original, "the .bak copy is the original, never replaced");
        assert_eq!(backed_up(&path), vec![first.clone(), original.clone()]);
        // Saving again with nothing new keeps the file on disk once, never twice.
        app.save_to(&path);
        app.save_to(&path);
        assert_eq!(backed_up(&path).len(), 3);
        assert_eq!(gold_on_disk(&path), json!(3));
        // Undo after saving: the change comes back as unsaved, and saving it keeps the newer file too.
        app.undo_redo(true);
        assert_eq!(value(&app, "/gold"), Some(json!(2)));
        assert!(app.dirty);
        let third = std::fs::read(&path).unwrap();
        app.save_to(&path);
        assert_eq!(gold_on_disk(&path), json!(2));
        assert!(!app.dirty);
        assert_eq!(backed_up(&path)[0], third);
    }

    #[test]
    fn save_as_backs_up_the_file_it_replaces_and_a_failed_one_changes_nothing() {
        let _l = locks();
        let s = Scratch::new("save-as");
        let ctx = window();
        let path = s.0.join("slot1.json");
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        let other = s.0.join("slot2.json");
        let other_bytes = br#"{"gold": 77, "slot": 2}"#;
        std::fs::write(&other, other_bytes).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(5), 0.0);
        // Into a folder that doesn't exist: an error, and the open file is still the same one.
        let nowhere = s.0.join("missing").join("copy.json");
        app.save_to(&nowhere);
        assert_eq!(app.status_kind, StatusKind::Err);
        assert!(!nowhere.exists() && !s.0.join("missing").exists());
        assert_eq!(app.doc.as_ref().unwrap().path, path);
        assert!(app.dirty);
        // Over another save: that save is backed up before it is replaced.
        app.save_to(&other);
        assert_eq!(gold_on_disk(&other), json!(5), "{}", app.status);
        assert_eq!(backed_up(&other), vec![other_bytes.to_vec()]);
        assert_eq!(std::fs::read(s.0.join("slot2.json.bak")).unwrap(), other_bytes);
        assert_eq!(gold_on_disk(&path), json!(1), "the file it was opened from is untouched");
        assert_eq!(app.doc.as_ref().unwrap().path, other);
    }

    #[test]
    fn a_companion_file_that_cant_be_written_puts_the_save_back() {
        let _l = locks();
        let s = Scratch::new("companion");
        let path = s.0.join("data0001.bin");
        let original = br#"{"gold": 1}"#.to_vec();
        std::fs::write(&path, &original).unwrap();
        // Persona 4 Golden keeps the save's MD5 in the .binslot beside it.
        let mut slot = b"SAVE0001".to_vec();
        slot.resize(0x40, 0);
        let slot_path = s.0.join("data0001.binslot");
        std::fs::write(&slot_path, &slot).unwrap();
        let opts = formats::Options::for_path(&path);
        let d = formats::decode_with(&original, &opts);
        let edited = Content::Json { value: json!({"gold": 2}), pretty: false };
        // The .binslot can't be replaced.
        std::fs::create_dir_all(s.0.join("data0001.binslot.partial").join("x")).unwrap();
        let r = write_save(&path, &d.layers, &edited, &opts);
        assert!(r.is_err(), "reported as not saved");
        assert_eq!(std::fs::read(&path).unwrap(), original, "the save still matches its .binslot");
        assert_eq!(std::fs::read(&slot_path).unwrap(), slot);
        // Once it can be, both are written.
        std::fs::remove_dir_all(s.0.join("data0001.binslot.partial")).unwrap();
        let w = write_save(&path, &d.layers, &edited, &opts).unwrap();
        assert_eq!(w.stamp, safety::stamp(&path));
        use md5::Digest;
        assert_eq!(std::fs::read(&slot_path).unwrap()[0x18..0x28], md5::Md5::digest(std::fs::read(&path).unwrap())[..]);
    }

    /// A Persona 4 Golden-style save (JSON here) with the .binslot that holds its MD5.
    fn save_with_binslot(s: &Scratch) -> (PathBuf, PathBuf, Vec<u8>) {
        let path = s.0.join("data0001.bin");
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        let mut slot = b"SAVE0001".to_vec();
        slot.resize(0x40, 0);
        let slot_path = s.0.join("data0001.binslot");
        std::fs::write(&slot_path, &slot).unwrap();
        (path, slot_path, slot)
    }

    #[test]
    fn saving_again_after_a_rolled_back_save_does_not_ask_about_changes_on_disk() {
        let _l = locks();
        let s = Scratch::new("rollback-stamp");
        let ctx = window();
        let (path, slot_path, _) = save_with_binslot(&s);
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(2), 0.0);
        std::fs::create_dir_all(s.0.join("data0001.binslot.partial").join("x")).unwrap();
        app.save_to(&path);
        assert_eq!(app.status_kind, StatusKind::Err);
        assert!(app.status.starts_with("Couldn't write"), "{}", app.status);
        assert_eq!(gold_on_disk(&path), json!(1));
        assert!(!safety::changed(&path, app.doc.as_ref().unwrap().stamp.as_ref()), "the file is as it was opened, time included");
        std::fs::remove_dir_all(s.0.join("data0001.binslot.partial")).unwrap();
        app.save_to(&path);
        assert!(app.stale_save.is_none(), "no question about a change nobody else made");
        assert_eq!(gold_on_disk(&path), json!(2), "{}", app.status);
        assert!(!app.dirty);
        use md5::Digest;
        assert_eq!(std::fs::read(&slot_path).unwrap()[0x18..0x28], md5::Md5::digest(std::fs::read(&path).unwrap())[..]);
    }

    #[test]
    fn a_restore_whose_companion_cant_be_written_says_so_and_changes_nothing() {
        let _l = locks();
        let s = Scratch::new("restore-companion");
        let ctx = window();
        let (path, slot_path, slot) = save_with_binslot(&s);
        let from = s.0.join("older.sebak");
        std::fs::write(&from, br#"{"gold": 0}"#).unwrap();
        let mut app = opened(&ctx, &path);
        std::fs::create_dir_all(s.0.join("data0001.binslot.partial").join("x")).unwrap();
        app.restore_from(&path, &from, "older");
        assert_eq!(app.status_kind, StatusKind::Err, "{}", app.status);
        assert!(app.status.starts_with("Couldn't restore the backup"), "{}", app.status);
        assert_eq!(gold_on_disk(&path), json!(1), "the save still matches its .binslot");
        assert_eq!(std::fs::read(&slot_path).unwrap(), slot);
        assert!(!safety::changed(&path, app.doc.as_ref().unwrap().stamp.as_ref()));
        // Once it can be written, the restore goes through and both match.
        std::fs::remove_dir_all(s.0.join("data0001.binslot.partial")).unwrap();
        app.restore_from(&path, &from, "older");
        assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
        assert_eq!(gold_on_disk(&path), json!(0));
        use md5::Digest;
        assert_eq!(std::fs::read(&slot_path).unwrap()[0x18..0x28], md5::Md5::digest(std::fs::read(&path).unwrap())[..]);
    }

    #[test]
    fn exporting_over_an_existing_file_backs_it_up_first() {
        let _l = locks();
        let s = Scratch::new("export");
        let ctx = window();
        // A gzip-packed save: its decoded data is not something the game can load.
        use std::io::Write;
        let mut e = flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
        e.write_all(br#"{"gold": 1}"#).unwrap();
        let packed = e.finish().unwrap();
        let path = s.0.join("save.dat");
        std::fs::write(&path, &packed).unwrap();
        let app = &mut opened(&ctx, &path);
        // Picked the save itself in the export dialog and confirmed the replace.
        app.export_to(&path);
        assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
        assert_eq!(gold_on_disk(&path), json!(1), "the decoded data was written");
        assert!(backed_up(&path).contains(&packed), "the packed save can be brought back");
        // Exporting to a new file makes no backups.
        let out = s.0.join("out.json");
        app.export_to(&out);
        assert!(out.exists() && crate::backups::list(&out).is_empty());
    }

    #[test]
    fn a_restore_with_unsaved_changes_is_refused() {
        let _l = locks();
        let s = Scratch::new("restore-dirty");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        let from = s.0.join("old.json");
        std::fs::write(&from, br#"{"gold": 0}"#).unwrap();
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(2), 0.0);
        app.restore_from(&path, &from, "old");
        assert_eq!(app.status_kind, StatusKind::Err);
        assert_eq!(gold_on_disk(&path), json!(1));
        assert_eq!(value(&app, "/gold"), Some(json!(2)), "the changes are still there");
        assert!(crate::backups::list(&path).is_empty());
        // Without changes it goes through, and what it replaced is kept.
        app.revert_all();
        app.restore_from(&path, &from, "old");
        assert_eq!(gold_on_disk(&path), json!(0), "{}", app.status);
        assert_eq!(value(&app, "/gold"), Some(json!(0)));
        assert_eq!(backed_up(&path), vec![br#"{"gold": 1}"#.to_vec()]);
    }

    /// One frame of a 1000×700 window with `events`, the input going through `raw_input_hook`.
    fn run(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0))), ..Default::default() };
        app.raw_input_hook(ctx, &mut raw);
        ctx.run(raw, |ctx| app.frame(ctx))
    }

    /// Every piece of text drawn in a frame, with the middle of where it was drawn.
    fn texts(out: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
        let mut found = vec![];
        for cs in &out.shapes {
            let mut stack = vec![&cs.shape];
            while let Some(sh) = stack.pop() {
                match sh {
                    egui::epaint::Shape::Vec(v) => stack.extend(v.iter()),
                    egui::epaint::Shape::Text(t) => found.push((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2()).center())),
                    _ => {}
                }
            }
        }
        found
    }

    fn shown(app: &mut App, ctx: &egui::Context, text: &str) -> bool {
        (0..2).any(|_| texts(&run(app, ctx, vec![])).iter().any(|(t, _)| t.contains(text)))
    }

    /// Clicks the text starting with `text`, as a player would with the mouse.
    fn click(app: &mut App, ctx: &egui::Context, text: &str) {
        // A window that just opened is sized on its first frame and drawn on the next.
        let mut all = vec![];
        for _ in 0..3 {
            all = texts(&run(app, ctx, vec![]));
            if all.iter().any(|(t, _)| t.starts_with(text)) {
                break;
            }
        }
        let at = all.iter().find(|(t, _)| t.starts_with(text)).unwrap_or_else(|| panic!("{text:?} is not shown; shown: {:?}", all.iter().map(|t| &t.0).collect::<Vec<_>>())).1;
        run(app, ctx, vec![egui::Event::PointerMoved(at)]);
        let press = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        run(app, ctx, vec![press(true), press(false)]);
        run(app, ctx, vec![]);
    }

    fn key(app: &mut App, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
        run(app, ctx, vec![egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }]);
        run(app, ctx, vec![egui::Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers }]);
        run(app, ctx, vec![]);
    }

    /// Opens Backups › Quick restore and clicks the entry starting with `entry`.
    fn quick_restore(app: &mut App, ctx: &egui::Context, entry: &str) {
        click(app, ctx, "Backups");
        click(app, ctx, "Quick restore");
        click(app, ctx, entry);
    }

    /// A save with one automatic backup of an older version and a .bak of the first one.
    fn save_with_backups(dir: &Path) -> PathBuf {
        let path = dir.join("save.json");
        std::fs::write(&path, br#"{"gold": 0}"#).unwrap();
        crate::backups::take(&path).unwrap();
        std::fs::write(dir.join("save.json.bak"), br#"{"gold": -1}"#).unwrap();
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        path
    }

    #[test]
    fn quick_restore_asks_first_and_cancelling_changes_nothing() {
        let _l = locks();
        let s = Scratch::new("quick-ask");
        let ctx = window();
        let path = save_with_backups(&s.0);
        let before = backed_up(&path);
        let mut app = opened(&ctx, &path);
        quick_restore(&mut app, &ctx, "just now");
        assert_eq!(gold_on_disk(&path), json!(1), "clicking the backup alone restores nothing");
        assert!(shown(&mut app, &ctx, "save.json"), "the dialog names the save");
        assert!(shown(&mut app, &ctx, "Restore now"));
        // Focus starts on Cancel: Enter cancels.
        key(&mut app, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
        assert!(!shown(&mut app, &ctx, "Restore now"), "the dialog closed");
        assert_eq!(gold_on_disk(&path), json!(1));
        assert_eq!(backed_up(&path), before, "no backup was made or removed");
        // Escape cancels too.
        quick_restore(&mut app, &ctx, "just now");
        key(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
        assert!(!shown(&mut app, &ctx, "Restore now"));
        assert_eq!(gold_on_disk(&path), json!(1));
        assert_eq!(backed_up(&path), before);
        // Cancel with the mouse.
        quick_restore(&mut app, &ctx, "just now");
        click(&mut app, &ctx, "Cancel");
        assert!(!shown(&mut app, &ctx, "Restore now"));
        assert_eq!(gold_on_disk(&path), json!(1));
        assert_eq!(backed_up(&path), before);
        // Restore now puts the backup back, after backing up the save it replaces.
        quick_restore(&mut app, &ctx, "just now");
        click(&mut app, &ctx, "Restore now");
        assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
        assert_eq!(gold_on_disk(&path), json!(0));
        assert!(backed_up(&path).contains(&br#"{"gold": 1}"#.to_vec()));
    }

    #[test]
    fn restoring_the_original_save_asks_first_and_keyboard_confirms() {
        let _l = locks();
        let s = Scratch::new("quick-bak");
        let ctx = window();
        let path = save_with_backups(&s.0);
        let mut app = opened(&ctx, &path);
        quick_restore(&mut app, &ctx, "Original save (.bak)");
        assert_eq!(gold_on_disk(&path), json!(1));
        assert!(shown(&mut app, &ctx, "the original save"), "the dialog says which backup");
        // Shift+Tab from Cancel reaches Restore now; Enter presses it.
        key(&mut app, &ctx, egui::Key::Tab, egui::Modifiers::SHIFT);
        key(&mut app, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
        assert_eq!(gold_on_disk(&path), json!(-1), "{}", app.status);
        assert!(backed_up(&path).contains(&br#"{"gold": 1}"#.to_vec()));
    }

    #[test]
    fn quick_restore_with_unsaved_changes_is_refused_without_asking() {
        let _l = locks();
        let s = Scratch::new("quick-dirty");
        let ctx = window();
        let path = save_with_backups(&s.0);
        let before = backed_up(&path);
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(5), 0.0);
        quick_restore(&mut app, &ctx, "just now");
        assert_eq!(app.status_kind, StatusKind::Err);
        assert!(!shown(&mut app, &ctx, "Restore now"), "nothing to confirm");
        assert_eq!(gold_on_disk(&path), json!(1));
        assert_eq!(backed_up(&path), before);
        assert_eq!(value(&app, "/gold"), Some(json!(5)), "the changes are still there");
    }

    #[test]
    fn deleting_a_preset_asks_first() {
        let _l = locks();
        let s = Scratch::new("preset-delete");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        let file = s.0.join("rich.json");
        let preset = crate::presets::Preset { name: "Rich".into(), game: String::new(), values: vec![], bytes: vec![] };
        std::fs::write(&file, b"{}").unwrap();
        let mut app = opened(&ctx, &path);
        let open = |app: &mut App| app.preset_apply = Some(PresetApply { list: vec![(file.clone(), preset.clone())], picked: Some(0), opened: None, confirm_delete: None });
        open(&mut app);
        click(&mut app, &ctx, "Delete preset");
        assert!(file.exists(), "clicking Delete preset alone deletes nothing");
        assert!(shown(&mut app, &ctx, "Rich"), "the dialog names the preset");
        assert!(shown(&mut app, &ctx, "Delete permanently"));
        // Focus starts on Cancel: Enter cancels, back to the list.
        key(&mut app, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
        assert!(file.exists());
        assert!(!shown(&mut app, &ctx, "Delete permanently"));
        assert!(shown(&mut app, &ctx, "Delete preset"), "the preset list is still open");
        // Escape cancels the question only.
        click(&mut app, &ctx, "Delete preset");
        key(&mut app, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
        assert!(file.exists());
        assert!(app.preset_apply.is_some(), "Escape closed only the question");
        assert!(shown(&mut app, &ctx, "Delete preset"));
        // Delete permanently deletes it.
        click(&mut app, &ctx, "Delete preset");
        click(&mut app, &ctx, "Delete permanently");
        assert!(!file.exists(), "{}", app.status);
        assert_eq!(app.status_kind, StatusKind::Ok, "{}", app.status);
        assert!(app.preset_apply.as_ref().is_some_and(|pa| pa.list.is_empty()));
    }

    /// Two tabs, `a` showing, each a save with a backup and a .bak (see [`save_with_backups`]).
    fn two_tabs(ctx: &egui::Context, s: &Scratch) -> (App, PathBuf, PathBuf) {
        let (da, db) = (s.0.join("a"), s.0.join("b"));
        std::fs::create_dir_all(&da).unwrap();
        std::fs::create_dir_all(&db).unwrap();
        let (a, b) = (save_with_backups(&da), save_with_backups(&db));
        let mut app = opened(ctx, &a);
        app.open_in_tab(b.clone());
        for _ in 0..400 {
            if app.loading.is_none() {
                break;
            }
            run(&mut app, ctx, vec![]);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        app.switch_tab(0);
        assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(a.clone()));
        (app, a, b)
    }

    fn on_disk(path: &Path) -> (Vec<u8>, Vec<Vec<u8>>) {
        (std::fs::read(path).unwrap(), backed_up(path))
    }

    #[test]
    fn close_save_asks_first_and_never_drops_unsaved_changes_silently() {
        let _l = locks();
        let s = Scratch::new("close-save");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, br#"{"gold": 5}"#).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut app = opened(&ctx, &path);
        assert!(shown(&mut app, &ctx, "Close save"), "the toolbar has the button");
        app.apply_edit("/gold".into(), json!(9), 0.0);
        assert!(app.dirty);
        // Unsaved changes: the button asks; Cancel keeps everything.
        click(&mut app, &ctx, "Close save");
        assert!(app.pending_close && app.doc.is_some());
        click(&mut app, &ctx, "Cancel");
        assert!(app.doc.is_some() && app.dirty && !app.undo.is_empty(), "cancel changed nothing");
        // Ctrl+W asks too.
        key(&mut app, &ctx, egui::Key::W, egui::Modifiers::COMMAND);
        assert!(app.pending_close && app.doc.is_some());
        // Discarding closes without writing, and the next save opens clean.
        click(&mut app, &ctx, "Close without saving");
        assert!(app.doc.is_none() && !app.dirty && app.undo.is_empty() && app.redo.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), before, "the file on disk is untouched");
        assert!(!shown(&mut app, &ctx, "Close save"), "no button on the start page");
        // Save and close writes first.
        let mut app = opened(&ctx, &path);
        app.apply_edit("/gold".into(), json!(7), 0.0);
        click(&mut app, &ctx, "Close save");
        click(&mut app, &ctx, "Save and close");
        wait(&mut app, &ctx);
        assert!(app.doc.is_none());
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap()["gold"], json!(7));
        // Nothing unsaved: closes straight away.
        let mut app = opened(&ctx, &path);
        key(&mut app, &ctx, egui::Key::W, egui::Modifiers::COMMAND);
        assert!(app.doc.is_none() && !app.pending_close);
    }

    #[test]
    fn close_save_is_reachable_at_phone_width() {
        let _l = locks();
        let s = Scratch::new("close-save-phone");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, br#"{"gold": 5}"#).unwrap();
        let mut app = opened(&ctx, &path);
        let mut raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(380.0, 700.0))), ..Default::default() };
        app.raw_input_hook(&ctx, &mut raw);
        let out = ctx.run(raw, |ctx| app.frame(ctx));
        let at = texts(&out).into_iter().find(|(t, _)| t == "\u{00D7}").expect("a close button on the phone toolbar").1;
        assert!(at.x > 0.0 && at.x < 380.0, "inside the screen: {at:?}");
    }

    #[test]
    fn tab_keys_under_a_quick_restore_never_restore_over_another_save() {
        let _l = locks();
        for (name, k, shift) in [("ctrl-w", egui::Key::W, false), ("ctrl-tab", egui::Key::Tab, false), ("ctrl-shift-tab", egui::Key::Tab, true)] {
            let s = Scratch::new(&format!("quick-tabs-{name}"));
            let ctx = window();
            let (mut app, a, b) = two_tabs(&ctx, &s);
            let (a_before, b_before) = (on_disk(&a), on_disk(&b));
            quick_restore(&mut app, &ctx, "just now");
            let mods = if shift { egui::Modifiers::COMMAND | egui::Modifiers::SHIFT } else { egui::Modifiers::COMMAND };
            key(&mut app, &ctx, k, mods);
            assert_eq!(app.tab_count(), 2, "{name}: the tab under the dialog stays open");
            assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(a.clone()), "{name}: the tab under the dialog stays in front");
            assert!(shown(&mut app, &ctx, "Restore now"), "{name}: the dialog is still there, about a");
            assert_eq!((on_disk(&a), on_disk(&b)), (a_before.clone(), b_before.clone()), "{name}: nothing changed on disk");
            // Confirming restores a, and only a.
            click(&mut app, &ctx, "Restore now");
            assert_eq!(gold_on_disk(&a), json!(0), "{name}: {}", app.status);
            assert_eq!(on_disk(&b), b_before, "{name}: b untouched");
        }
    }

    #[test]
    fn a_quick_restore_is_dropped_when_its_save_is_no_longer_in_front() {
        let _l = locks();
        let s = Scratch::new("quick-switch");
        let ctx = window();
        let (mut app, a, b) = two_tabs(&ctx, &s);
        let (a_before, b_before) = (on_disk(&a), on_disk(&b));
        // Another tab comes to the front by any way at all.
        quick_restore(&mut app, &ctx, "just now");
        app.switch_tab(1);
        assert!(app.quick_restore.is_none(), "switching tabs drops the question");
        app.switch_tab(0);
        assert!(!shown(&mut app, &ctx, "Restore now"), "and it doesn't come back with the tab");
        // The tab is closed.
        quick_restore(&mut app, &ctx, "just now");
        app.drop_tab(0);
        assert!(app.quick_restore.is_none(), "closing the tab drops the question");
        assert!(!shown(&mut app, &ctx, "Restore now"));
        assert_eq!((on_disk(&a), on_disk(&b)), (a_before.clone(), b_before.clone()));
        // Another file replaces it in the same tab: the dialog closes itself and restores nothing.
        app.quick_restore = Some((a.clone(), list_first(&a), "the backup".into()));
        assert_eq!(app.doc.as_ref().map(|d| d.path.clone()), Some(b.clone()));
        run(&mut app, &ctx, vec![]);
        assert!(app.quick_restore.is_none());
        assert_eq!((on_disk(&a), on_disk(&b)), (a_before, b_before));
    }

    fn list_first(save: &Path) -> PathBuf {
        crate::backups::list(save)[0].path.clone()
    }

    #[test]
    fn a_preset_that_cant_be_deleted_stays_in_the_list() {
        let _l = locks();
        let s = Scratch::new("preset-delete-fails");
        let ctx = window();
        let path = s.0.join("save.json");
        std::fs::write(&path, br#"{"gold": 1}"#).unwrap();
        // A folder with something in it where the preset file should be: removing it fails.
        let file = s.0.join("rich.json");
        std::fs::create_dir_all(file.join("inside")).unwrap();
        let preset = crate::presets::Preset { name: "Rich".into(), game: String::new(), values: vec![], bytes: vec![] };
        let mut app = opened(&ctx, &path);
        app.preset_apply = Some(PresetApply { list: vec![(file.clone(), preset)], picked: Some(0), opened: None, confirm_delete: None });
        click(&mut app, &ctx, "Delete preset");
        click(&mut app, &ctx, "Delete permanently");
        assert_eq!(app.status_kind, StatusKind::Err, "{}", app.status);
        assert!(file.exists());
        assert_eq!(app.preset_apply.as_ref().map(|pa| pa.list.len()), Some(1), "still listed");
        assert!(shown(&mut app, &ctx, "Rich"));
    }

    #[test]
    fn values_list_shape() {
        let j = |v: Value| Content::Json { value: v, pretty: true };
        let same = |a: Value, b: Value| same_values_list(&j(a), &j(b));
        assert!(same(json!({"a": 1, "b": [true, "x"]}), json!({"a": 2.5, "b": [false, "y"]})));
        assert!(same(json!({"o": {"name": "A", "hp": 1}}), json!({"o": {"name": "A", "hp": 9}})));
        assert!(!same(json!({"o": {"name": "A"}}), json!({"o": {"name": "B"}})), "an object's short name");
        assert!(!same(json!({"o": {"title": "A", "n": 1}}), json!({"o": {"title": " ", "n": 1}})));
        assert!(!same(json!({"a": null}), json!({"a": 1})), "a null value has no row");
        assert!(!same(json!({"a": 1}), json!({"a": [1]})));
        assert!(!same(json!({"a": 1, "b": 2}), json!({"b": 2, "a": 1})), "order");
        assert!(!same(json!({"a": [1, 2]}), json!({"a": [1, 2, 3]})));
        assert!(!same(json!({"a": 1}), json!({"b": 1})));
        assert!(!same_values_list(&Content::Text("a".into()), &Content::Text("a".into())));
        assert!(!same_values_list(&j(json!(1)), &Content::Binary(vec![1])));
    }
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

/// Layout guards: draws windows off-screen at several sizes and fails when text is cut off at the
/// side of its box or a window doesn't fit the screen.
#[cfg(test)]
pub(crate) mod layout_tests {
    use super::*;

    /// Text that sticks out sideways past the area it was clipped to (so it is cut off or spills
    /// into a neighbour). Returns one line per offender.
    pub(crate) fn cut_off_text(out: &egui::FullOutput) -> Vec<String> {
        let mut bad = vec![];
        for cs in &out.shapes {
            let mut stack = vec![&cs.shape];
            while let Some(sh) = stack.pop() {
                match sh {
                    egui::epaint::Shape::Vec(v) => stack.extend(v.iter()),
                    egui::epaint::Shape::Text(t) => {
                        let r = t.galley.rect.translate(t.pos.to_vec2());
                        // 1 px for rounding; text under the top/bottom edge is just scrolled.
                        if r.right() > cs.clip_rect.right() + 1.0 || r.left() < cs.clip_rect.left() - 1.0 {
                            let s: String = t.galley.text().chars().take(50).collect();
                            bad.push(format!("{s:?} spans {:.0}..{:.0} but is clipped to {:.0}..{:.0}", r.left(), r.right(), cs.clip_rect.left(), cs.clip_rect.right()));
                        }
                    }
                    _ => {}
                }
            }
        }
        bad
    }

    fn input(w: f32, h: f32) -> egui::RawInput {
        egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() }
    }

    #[test]
    fn find_my_games_saves_fits_every_size() {
        let long = "C:\\Users\\SomeOne\\AppData\\Local\\Packages\\Some.Very.Long.PublisherName.GameTitle_8wekyb3d8bbwe\\SystemAppData\\wgs\\0009000000AB1234_0000000000000000000000007B2E4C81\\Slot1";
        let found: Vec<Option<Vec<crate::games::Found>>> = crate::games::GAMES
            .iter()
            .map(|_| Some((0..6).map(|i| crate::games::Found { path: PathBuf::from(format!("{long}\\quicksave_{i}.sav")), modified: Some(std::time::SystemTime::now()), size: 1 << 20, label: None }).collect()))
            .collect();
        let found: SaveSearch = std::sync::Arc::new(std::sync::Mutex::new(found));
        // The longest names, the ones covering many games, and a couple of ordinary ones.
        let mut picks: Vec<usize> = (0..crate::games::GAMES.len()).collect();
        picks.sort_by_key(|&i| std::cmp::Reverse(crate::games::GAMES[i].name.len()));
        picks.truncate(6);
        picks.extend((0..crate::games::GAMES.len()).filter(|&i| crate::games::many_games(&crate::games::GAMES[i])).take(2));
        picks.push(0);
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut problems = vec![];
        for lang in ["en", "de", "es", "fr", "pt-BR", "ja", "zh-CN"] {
        crate::i18n::set(Some(lang));
        for dark in [true, false] {
            for (w, h) in [(1280.0, 800.0), (1000.0, 700.0), (820.0, 600.0), (640.0, 480.0), (380.0, 760.0)] {
                for &pick in &picks {
                    let ctx = egui::Context::default();
                    crate::theme::install_fonts(&ctx);
                    crate::theme::apply(&ctx, dark);
                    let mut gp = GamePicker::with(found.clone());
                    gp.only_found = Some(true);
                    gp.picked = Some(pick);
                    for frame in 0..4 {
                        let out = ctx.run(input(w, h), |ctx| {
                            game_picker_window(ctx, &mut gp);
                        });
                        if frame == 3 {
                            let id = egui::Id::new(tr("Find my game's saves"));
                            let area = ctx.memory(|m| m.area_rect(id)).unwrap();
                            let who = format!("{} at {w}x{h} dark={dark} {lang}", crate::games::GAMES[pick].name);
                            if area.left() < -0.5 || area.top() < -0.5 || area.right() > w + 0.5 || area.bottom() > h + 0.5 {
                                problems.push(format!("{who}: window {area:?} leaves the screen"));
                            }
                            for t in cut_off_text(&out) {
                                problems.push(format!("{who}: {t}"));
                            }
                        }
                    }
                }
            }
        }
        }
        crate::i18n::set(Some("en"));
        problems.sort();
        problems.dedup();
        assert!(problems.is_empty(), "{} layout problems:\n{}", problems.len(), problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }

    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        theme::apply(&ctx, true);
        ctx
    }

    /// A Hollow Knight save as plain JSON: a header card with tiles and several panels.
    fn knight_save(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("use-phone-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user1.json");
        let pd = json!({"playerData": {"geo": 1234, "maxHealthBase": 5, "charmSlots": 3, "ore": 2, "simpleKeys": 1, "nailDamage": 5, "maxMP": 99, "MPReserveMax": 33, "health": 5, "maxHealth": 5, "hasDash": true, "playTime": 7200.0}});
        std::fs::write(&path, serde_json::to_vec_pretty(&pd).unwrap()).unwrap();
        path
    }

    fn panel_rect(ctx: &egui::Context, id: &str) -> Option<egui::Rect> {
        egui::containers::panel::PanelState::load(ctx, egui::Id::new(id)).map(|p| p.rect)
    }

    fn run_frames(app: &mut App, ctx: &egui::Context, w: f32, h: f32, n: usize) -> egui::FullOutput {
        let mut last = None;
        for _ in 0..n {
            let mut raw = input(w, h);
            // Time moves on, so scrolling animations finish.
            raw.time = Some(ctx.input(|i| i.time) + 0.5);
            eframe::App::raw_input_hook(app, ctx, &mut raw);
            last = Some(ctx.run(raw, |ctx| app.frame(ctx)));
        }
        last.unwrap()
    }

    /// The start page's game rows: the "Other saves" button sits at the right edge of an
    /// equal-height row and nothing is cut off or runs under it, in every language and size.
    #[test]
    fn start_page_game_rows_stay_even_with_other_saves_button() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let long = "C:\\Users\\SomeOne\\AppData\\Local\\Packages\\Some.Very.Long.PublisherName.GameTitle_8wekyb3d8bbwe\\Slot1";
        let singles: Vec<usize> = (0..crate::games::GAMES.len()).filter(|&i| !crate::games::many_games(&crate::games::GAMES[i])).take(4).collect();
        let mut v: Vec<Option<Vec<crate::games::Found>>> = vec![Some(vec![]); crate::games::GAMES.len()];
        for (k, &i) in singles.iter().enumerate() {
            let n = if k % 2 == 0 { 5 } else { 1 };
            v[i] = Some((0..n).map(|j| crate::games::Found { path: PathBuf::from(format!("{long}\\quicksave_{j}.sav")), modified: Some(std::time::SystemTime::now()), size: 1 << 20, label: None }).collect());
        }
        let found: SaveSearch = std::sync::Arc::new(std::sync::Mutex::new(v));
        let mut problems = vec![];
        for lang in ["en", "de", "es", "fr", "pt-BR", "ja", "zh-CN"] {
            crate::i18n::set(Some(lang));
            for (w, h) in [(1280.0, 800.0), (820.0, 600.0), (380.0, 760.0)] {
                let ctx = window();
                let mut app = App::with(&ctx, Settings::default(), false);
                app.saves_found = Some(found.clone());
                let mut out = None;
                for _ in 0..4 {
                    out = Some(ctx.run(input(w, h), |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            app.your_games(ui);
                        });
                    }));
                }
                let who = format!("{lang} at {w}x{h}");
                let out = out.unwrap();
                for t in cut_off_text(&out) {
                    problems.push(format!("{who}: {t}"));
                }
                // No two pieces of text may overlap each other.
                let mut texts: Vec<(egui::Rect, String)> = vec![];
                for cs in &out.shapes {
                    let mut stack = vec![&cs.shape];
                    while let Some(sh) = stack.pop() {
                        match sh {
                            egui::epaint::Shape::Vec(v) => stack.extend(v.iter()),
                            egui::epaint::Shape::Text(t) if !t.galley.text().trim().is_empty() => {
                                texts.push((t.galley.rect.translate(t.pos.to_vec2()).intersect(cs.clip_rect), t.galley.text().chars().take(30).collect()));
                            }
                            _ => {}
                        }
                    }
                }
                for (i, a) in texts.iter().enumerate() {
                    for b in &texts[i + 1..] {
                        let o = a.0.intersect(b.0);
                        if o.width() > 2.0 && o.height() > 2.0 {
                            problems.push(format!("{who}: {:?} overlaps {:?}", a.1, b.1));
                        }
                    }
                }
            }
        }
        crate::i18n::set(Some("en"));
        problems.sort();
        problems.dedup();
        assert!(problems.is_empty(), "{} problems:\n{}", problems.len(), problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }

    #[test]
    fn phone_layout_has_one_toolbar_line_and_section_chips_in_every_language() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = knight_save("chips");
        let mut problems = vec![];
        for lang in ["en", "de", "es", "fr", "pt-BR", "ja", "zh-CN"] {
            crate::i18n::set(Some(lang));
            for (w, h) in [(360.0, 740.0), (412.0, 900.0), (600.0, 900.0)] {
                let ctx = window();
                let mut app = App::with(&ctx, Settings::default(), false);
                app.load(&path);
                assert_eq!(app.doc.as_ref().and_then(|d| d.game), Some(crate::game_ui::Game::HollowKnight), "{}", app.status);
                // An edit, so Save shows its count and a section gets the edited mark.
                app.apply_edit("/playerData/geo".into(), json!(5000), 0.0);
                let out = run_frames(&mut app, &ctx, w, h, 5);
                let who = format!("{lang} at {w}x{h}");
                if panel_rect(&ctx, "menu").is_some() {
                    problems.push(format!("{who}: the menu bar is shown on a phone"));
                }
                match panel_rect(&ctx, "toolbar") {
                    Some(r) if r.height() > 56.0 => problems.push(format!("{who}: toolbar is {} px tall (more than one line)", r.height())),
                    None => problems.push(format!("{who}: no toolbar")),
                    _ => {}
                }
                let secs = crate::game_ui::sections(&ctx);
                let titles: Vec<&str> = secs.iter().map(|s| s.title.as_str()).collect();
                if secs.len() < 3 {
                    problems.push(format!("{who}: only {} sections: {titles:?}", secs.len()));
                }
                if !secs.first().is_some_and(|s| s.edited) || secs.iter().skip(1).any(|s| s.edited) {
                    problems.push(format!("{who}: only the header card holds the edit: {secs:?}"));
                }
                // The chips row scrolls sideways: chips past its edge are meant to be cut off.
                for t in cut_off_text(&out).into_iter().filter(|t| !secs.iter().any(|s| t.starts_with(&format!("{:?} spans", s.title)) || t.starts_with(&format!("\"{}\u{2026}\" spans", s.title.chars().take(27).collect::<String>())))) {
                    problems.push(format!("{who}: {t}"));
                }
            }
        }
        crate::i18n::set(Some("en"));
        problems.sort();
        problems.dedup();
        assert!(problems.is_empty(), "{} problems:\n{}", problems.len(), problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }

    #[test]
    fn a_section_chip_scrolls_its_section_to_the_top() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("en"));
        let path = knight_save("jump");
        let ctx = window();
        let mut app = App::with(&ctx, Settings::default(), false);
        app.load(&path);
        // Short, so the last section starts below the bottom of the screen.
        let (w, h) = (380.0, 420.0);
        run_frames(&mut app, &ctx, w, h, 4);
        let secs = crate::game_ui::sections(&ctx);
        let last = secs.last().cloned().expect("sections");
        assert!(last.top > h, "the last section is already on screen: {secs:?}");
        crate::game_ui::jump_to_section(&ctx, last.index);
        run_frames(&mut app, &ctx, w, h, 6);
        let after = crate::game_ui::sections(&ctx).into_iter().find(|s| s.index == last.index).unwrap();
        // The last section can only come up as far as the end of the screen allows.
        assert!(after.top < h - 60.0, "{} still at {} after the jump", after.title, after.top);
    }

    #[test]
    fn desktop_keeps_its_menu_bar_and_save_shows_the_count() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("en"));
        let path = knight_save("desk");
        let ctx = window();
        let mut app = App::with(&ctx, Settings::default(), false);
        app.load(&path);
        app.apply_edit("/playerData/geo".into(), json!(5000), 0.0);
        let out = run_frames(&mut app, &ctx, 1280.0, 800.0, 4);
        assert!(panel_rect(&ctx, "menu").is_some(), "the menu bar is missing on a wide window");
        assert!(panel_rect(&ctx, "toolbar").is_some_and(|r| r.height() < 56.0), "the toolbar wrapped on a wide window");
        assert_eq!(app.save_label(), "Save (1)");
        assert!(cut_off_text(&out).is_empty(), "{:?}", cut_off_text(&out));
    }

    #[test]
    fn a_dismissed_file_note_stays_hidden_and_comes_back_from_the_view_menu() {
        let ctx = window();
        let mut app = App::with(&ctx, Settings::default(), false);
        app.doc_note = "Level, experience and gold.".into();
        assert!(!app.note_hidden());
        app.settings.hidden_notes.push(Settings::note_key(&app.doc_note));
        assert!(app.note_hidden());
        // Another file with the same note: still hidden.
        app.doc_note = "Level, experience and gold.".into();
        assert!(app.note_hidden());
        app.doc_note = "Something else.".into();
        assert!(!app.note_hidden());
    }

    /// Every piece of text a frame painted, with where it sits, in reading order.
    fn text_layout(out: &egui::FullOutput) -> Vec<(String, [i32; 4])> {
        let mut v = vec![];
        for cs in &out.shapes {
            let mut stack = vec![&cs.shape];
            while let Some(sh) = stack.pop() {
                match sh {
                    egui::epaint::Shape::Vec(x) => stack.extend(x.iter()),
                    egui::epaint::Shape::Text(t) if !t.galley.text().trim().is_empty() => {
                        let r = t.galley.rect.translate(t.pos.to_vec2());
                        v.push((t.galley.text().to_string(), [r.left().round() as i32, r.top().round() as i32, r.right().round() as i32, r.bottom().round() as i32]));
                    }
                    _ => {}
                }
            }
        }
        v.sort();
        v
    }

    /// Shrinking the window (to a phone size and back) must leave the same layout as a window
    /// that was never shrunk: nothing remembered from the small size may linger.
    #[test]
    fn growing_the_window_back_restores_the_layout() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("en"));
        let path = knight_save("regrow");
        let mut problems = vec![];
        for (big, small) in [((1280.0, 800.0), (380.0, 760.0)), ((1280.0, 800.0), (700.0, 600.0)), ((900.0, 700.0), (500.0, 700.0))] {
            let fresh_ctx = window();
            let mut fresh = App::with(&fresh_ctx, Settings::default(), false);
            fresh.load(&path);
            let want = text_layout(&run_frames(&mut fresh, &fresh_ctx, big.0, big.1, 8));
            let ctx = window();
            let mut app = App::with(&ctx, Settings::default(), false);
            app.load(&path);
            run_frames(&mut app, &ctx, big.0, big.1, 8);
            run_frames(&mut app, &ctx, small.0, small.1, 8);
            let got = text_layout(&run_frames(&mut app, &ctx, big.0, big.1, 8));
            if got != want {
                let missing: Vec<_> = want.iter().filter(|x| !got.contains(x)).take(6).collect();
                let extra: Vec<_> = got.iter().filter(|x| !want.contains(x)).take(6).collect();
                problems.push(format!("{big:?} -> {small:?} -> {big:?}: differs from a fresh window\n  expected {missing:?}\n  got {extra:?}"));
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
