//! Search everything (Ctrl+K): one box that finds a menu command, a game, a recent file or a
//! value on the open save, and runs, opens or shows what is picked.
//!
//! The commands are the menu items that do one thing (a submenu that lists choices, such as
//! Open as or Quick restore, stays in the menu). Each one does what its menu item does and is
//! offered only where the menu offers it; one the menu greys out is greyed out here and does
//! nothing when picked.

use super::{fit, friendly_kind, help, modal_shown, recent_label, reveal_text, root, Ask, App, Content, Doc, Settings, Tab, WEB};
use crate::i18n::{tr, tr_any, trf, tx};
use crate::widgets::ListBox;
use crate::{names, platform, theme, values_view, widgets};
use eframe::egui::{self, text::LayoutJob, Align2, Key, Modifiers, RichText, Sense, TextEdit, TextFormat};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// Rows shown per group at most.
const PER_GROUP: usize = 8;
/// Height of a result row.
const ROW_H: f32 = 30.0;
/// Values are looked for once this many letters are typed (a single letter matches nearly
/// every value of a big save).
const VALUE_MIN_CHARS: usize = 2;

/// The open search box.
#[derive(Default)]
pub(super) struct Palette {
    pub(super) query: String,
    /// The row picked with the arrow keys (an index into the rows shown).
    sel: Option<usize>,
    /// The text the rows (and `sel`) were worked out for.
    seen: Option<String>,
    /// Scroll the picked row into view on the next frame.
    scroll: bool,
    /// Rows of the Values list matching (text, model, edits), best first.
    values: Option<((String, u64, u64), Vec<usize>)>,
}

/// A menu command, run the way its menu item runs it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Cmd {
    Open,
    OpenInPlace,
    FindSaves,
    Save,
    SaveAs,
    Reload,
    CloseSave,
    RevertAll,
    Export,
    Import,
    BackUpNow,
    BackUpNewGame,
    SaveManager,
    BackupSpace,
    Undo,
    Redo,
    Find,
    CopyChanges,
    Preferences,
    ShowAdvanced,
    ShowRawKeys,
    ChangesPanel,
    ShowNote,
    ReviewBeforeSave,
    ThemeDark,
    ThemeLight,
    ThemeSystem,
    DecodedAsHex,
    LocateData,
    Decrypt,
    ChangeSteam,
    CheatPatches,
    FindChecksum,
    TemplatesFolder,
    PresetSave,
    PresetApply,
    PresetOpen,
    Compare,
    CopySlot,
    XboxSteam,
    Reveal,
    Shortcuts,
    SupportedGames,
    Website,
    Report,
    RequestGame,
    Questions,
    CheckUpdate,
    Licences,
    About,
}

/// Every command, in menu order (File, Backups, Edit, View, Tools, Help).
const COMMANDS: &[Cmd] = &[
    Cmd::Open,
    Cmd::OpenInPlace,
    Cmd::FindSaves,
    Cmd::Save,
    Cmd::SaveAs,
    Cmd::Reload,
    Cmd::CloseSave,
    Cmd::RevertAll,
    Cmd::Export,
    Cmd::Import,
    Cmd::BackUpNow,
    Cmd::BackUpNewGame,
    Cmd::SaveManager,
    Cmd::BackupSpace,
    Cmd::Undo,
    Cmd::Redo,
    Cmd::Find,
    Cmd::CopyChanges,
    Cmd::Preferences,
    Cmd::ShowAdvanced,
    Cmd::ShowRawKeys,
    Cmd::ChangesPanel,
    Cmd::ShowNote,
    Cmd::ReviewBeforeSave,
    Cmd::ThemeDark,
    Cmd::ThemeLight,
    Cmd::ThemeSystem,
    Cmd::DecodedAsHex,
    Cmd::LocateData,
    Cmd::Decrypt,
    Cmd::ChangeSteam,
    Cmd::CheatPatches,
    Cmd::FindChecksum,
    Cmd::TemplatesFolder,
    Cmd::PresetSave,
    Cmd::PresetApply,
    Cmd::PresetOpen,
    Cmd::Compare,
    Cmd::CopySlot,
    Cmd::XboxSteam,
    Cmd::Reveal,
    Cmd::Shortcuts,
    Cmd::SupportedGames,
    Cmd::Website,
    Cmd::Report,
    Cmd::RequestGame,
    Cmd::Questions,
    Cmd::CheckUpdate,
    Cmd::Licences,
    Cmd::About,
];

/// Games and their saves are looked for on a computer only (the menu has no Find my game's
/// saves in the browser or on a phone).
const HAS_GAMES: bool = !WEB && !cfg!(target_os = "android");

impl Cmd {
    /// The English text of a command named by one menu item.
    fn english(self) -> &'static str {
        match self {
            Cmd::Open => tx("Open\u{2026}"),
            Cmd::OpenInPlace => tx("Open to save in place\u{2026}"),
            Cmd::FindSaves => tx("Find my game's saves\u{2026}"),
            Cmd::Save => tx("Save"),
            Cmd::SaveAs => tx("Save a copy as\u{2026}"),
            Cmd::Reload => tx("Reload from disk"),
            Cmd::CloseSave => tx("Close save"),
            Cmd::RevertAll => tx("Revert all changes"),
            Cmd::Export => tx("Export decoded data\u{2026}"),
            Cmd::Import => tx("Import edited data\u{2026}"),
            Cmd::BackUpNow => tx("Back up now\u{2026}"),
            Cmd::BackUpNewGame => tx("Back up and start a new game\u{2026}"),
            Cmd::SaveManager => tx("Save manager\u{2026}"),
            Cmd::BackupSpace => tx("Backup space\u{2026}"),
            Cmd::Undo => tx("Undo"),
            Cmd::Redo => tx("Redo"),
            Cmd::Find => tx("Find\u{2026}"),
            Cmd::CopyChanges => tx("Copy changes as text"),
            Cmd::Preferences => tx("Preferences\u{2026}"),
            Cmd::ShowAdvanced => tx("Show rarely needed values"),
            Cmd::ShowRawKeys => tx("Show raw key names"),
            Cmd::ChangesPanel => tx("Changes panel"),
            Cmd::ShowNote => tx("Show the note about this file"),
            Cmd::ReviewBeforeSave => tx("Review changes before saving"),
            Cmd::ThemeDark => tx("Dark"),
            Cmd::ThemeLight => tx("Light"),
            Cmd::ThemeSystem => tx("Same as the system"),
            Cmd::DecodedAsHex => tx("Show decoded data as hex"),
            Cmd::LocateData => tx("Locate game data folder (RPG Maker names)\u{2026}"),
            Cmd::Decrypt => tx("Decrypt with password\u{2026}"),
            Cmd::ChangeSteam => tx("Change Steam account in this save\u{2026}"),
            Cmd::CheatPatches => tx("Cheat patches\u{2026}"),
            Cmd::FindChecksum => tx("Find checksum\u{2026}"),
            Cmd::TemplatesFolder => tx("Templates folder\u{2026}"),
            Cmd::PresetSave => tx("Save changes as a preset\u{2026}"),
            Cmd::PresetApply => tx("Apply a preset\u{2026}"),
            Cmd::PresetOpen => tx("Open a preset file\u{2026}"),
            Cmd::Compare => tx("Compare with another save\u{2026}"),
            Cmd::CopySlot => tx("Copy a slot to another slot\u{2026}"),
            Cmd::XboxSteam => tx("Copy to the Steam version\u{2026}"),
            Cmd::Reveal => {
                if cfg!(windows) {
                    tx("Show file in Explorer")
                } else if cfg!(target_os = "macos") {
                    tx("Show file in Finder")
                } else {
                    tx("Open containing folder")
                }
            }
            Cmd::Shortcuts => tx("Keyboard shortcuts"),
            Cmd::SupportedGames => {
                if WEB || cfg!(target_os = "android") {
                    tx("Supported games (website)")
                } else {
                    tx("Supported games\u{2026}")
                }
            }
            Cmd::Website => tx("Website"),
            Cmd::Report => tx("Report a problem\u{2026}"),
            Cmd::RequestGame => tx("Request a game\u{2026}"),
            Cmd::Questions => tx("Questions and ideas\u{2026}"),
            Cmd::CheckUpdate => tx("Check for a new version"),
            Cmd::Licences => tx("Licences and credits"),
            Cmd::About => tx("About {0}"),
        }
    }

    /// The text shown, as the menu shows it (the theme choices say which menu setting they are).
    fn label(self) -> String {
        match self {
            Cmd::ThemeDark | Cmd::ThemeLight | Cmd::ThemeSystem => format!("{} {}", tr("Theme:"), tr_any(self.english())),
            Cmd::Reveal => reveal_text(false).unwrap_or(tr_any(self.english())).to_string(),
            Cmd::About => trf("About {0}", &[&platform::DISPLAY_NAME]),
            _ => tr_any(self.english()).to_string(),
        }
    }

    /// The text matched against what is typed: the label, and the English one too, so a
    /// command can be found by either.
    fn search_text(self) -> String {
        let en = match self {
            Cmd::ThemeDark | Cmd::ThemeLight | Cmd::ThemeSystem => format!("Theme: {}", self.english()),
            Cmd::About => format!("About {}", platform::DISPLAY_NAME),
            _ => self.english().to_string(),
        };
        format!("{}\n{en}", self.label())
    }

    fn shortcut(self) -> Option<&'static str> {
        Some(match self {
            Cmd::Open => "Ctrl+O",
            Cmd::Save => "Ctrl+S",
            Cmd::CloseSave => "Ctrl+W",
            Cmd::Undo => "Ctrl+Z",
            Cmd::Redo => "Ctrl+Y",
            Cmd::Find => "Ctrl+F",
            _ => return None,
        })
    }

    /// `None`: the menu doesn't offer it here; `Some(false)`: offered but greyed out.
    fn state(self, app: &App, ctx: &egui::Context) -> Option<bool> {
        let doc = app.doc.as_ref();
        let has = doc.is_some();
        let content = doc.map(|d| &d.content);
        let binary = matches!(content, Some(Content::Binary(_)));
        let on_disk = !WEB && doc.is_some_and(|d| d.path.is_file());
        let desktop = !WEB;
        Some(match self {
            Cmd::Open | Cmd::Preferences | Cmd::PresetOpen | Cmd::Shortcuts | Cmd::SupportedGames | Cmd::Website | Cmd::Report | Cmd::RequestGame | Cmd::Questions | Cmd::Licences | Cmd::About => true,
            Cmd::ShowAdvanced | Cmd::ShowRawKeys | Cmd::ChangesPanel | Cmd::ReviewBeforeSave | Cmd::ThemeDark | Cmd::ThemeLight | Cmd::ThemeSystem => true,
            Cmd::OpenInPlace => {
                #[cfg(target_arch = "wasm32")]
                {
                    return crate::web::can_save_in_place().then_some(true);
                }
                #[cfg(not(target_arch = "wasm32"))]
                return None;
            }
            Cmd::FindSaves => return HAS_GAMES.then_some(true),
            Cmd::Save | Cmd::SaveAs | Cmd::CloseSave | Cmd::Export | Cmd::Find | Cmd::Decrypt | Cmd::ChangeSteam | Cmd::PresetApply | Cmd::Compare => has,
            Cmd::Reload => doc.is_some_and(|d| crate::vfs::exists(&d.path)),
            Cmd::RevertAll | Cmd::CopyChanges | Cmd::PresetSave => app.dirty,
            Cmd::Import => matches!(content, Some(Content::Binary(_) | Content::Text(_) | Content::Json { .. })),
            Cmd::BackUpNow | Cmd::BackUpNewGame => return desktop.then_some(on_disk),
            Cmd::SaveManager => return desktop.then_some(has),
            Cmd::BackupSpace => return desktop.then_some(true),
            Cmd::Undo => !app.undo.is_empty(),
            Cmd::Redo => !app.redo.is_empty(),
            Cmd::ShowNote => has && !app.doc_note.is_empty() && app.note_hidden(),
            Cmd::DecodedAsHex => doc.is_some_and(|d| !matches!(d.content, Content::Binary(_))),
            Cmd::LocateData => return desktop.then(|| matches!(&app.doc, Some(Doc { content: Content::Json { value, .. }, .. }) if names::is_rpgmaker(value))),
            Cmd::CheatPatches | Cmd::FindChecksum => binary,
            Cmd::TemplatesFolder => return cfg!(not(any(target_arch = "wasm32", target_os = "android"))).then_some(true),
            Cmd::CopySlot => return app.file_slots(ctx).is_some().then_some(true),
            Cmd::XboxSteam => return app.xbox_steam_source().is_some().then_some(true),
            Cmd::Reveal => return (has && reveal_text(false).is_some()).then_some(true),
            Cmd::CheckUpdate => return crate::update::AVAILABLE.then_some(app.update_rx.is_none()),
        })
    }

    /// For a setting the View menu ticks: whether it is on.
    fn ticked(self, app: &App) -> Option<bool> {
        let s = &app.settings;
        Some(match self {
            Cmd::ShowAdvanced => s.show_advanced,
            Cmd::ShowRawKeys => s.show_raw_keys,
            Cmd::ChangesPanel => app.show_changes,
            Cmd::ReviewBeforeSave => s.confirm_save,
            Cmd::ThemeDark => !s.follow_system && s.dark,
            Cmd::ThemeLight => !s.follow_system && !s.dark,
            Cmd::ThemeSystem => s.follow_system,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Group {
    Commands,
    Games,
    Recent,
    Values,
}

impl Group {
    fn title(self) -> &'static str {
        match self {
            Group::Commands => tr("Commands"),
            Group::Games => tr("Games"),
            Group::Recent => tr("Recent files"),
            Group::Values => tr("Values on this save"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Act {
    Cmd(Cmd),
    /// An entry of `crate::games::GAMES`.
    Game(usize),
    Recent(PathBuf),
    /// A row of the Values list.
    Value(usize),
}

/// One result row.
#[derive(Clone, Debug)]
pub(super) struct Hit {
    group: Group,
    pub(super) act: Act,
    label: String,
    /// Weaker text after the label (where a value is).
    sub: String,
    /// Text at the right edge (the shortcut, the value, a setting's state).
    right: String,
    pub(super) enabled: bool,
    tip: Option<String>,
}

/// The typed text in words, lower-cased.
fn words(q: &str) -> Vec<String> {
    q.split_whitespace().map(str::to_lowercase).collect()
}

/// How well `text` matches the typed `words` (lower is better), or `None` when some word is
/// not in it. Every word must appear somewhere (any case, any order). The text starting with
/// what was typed ranks first, then texts where every word starts a word.
fn score(words: &[String], text: &str) -> Option<u32> {
    if words.is_empty() {
        return Some(0);
    }
    let lc = text.to_lowercase();
    if !words.iter().all(|w| lc.contains(w.as_str())) {
        return None;
    }
    if lc.starts_with(&words.join(" ")) || lc.lines().any(|l| l.starts_with(&words.join(" "))) {
        return Some(0);
    }
    let starts_word = |w: &str| {
        lc.match_indices(w).any(|(i, _)| i == 0 || !lc[..i].chars().next_back().is_some_and(char::is_alphanumeric))
    };
    if words.iter().all(|w| starts_word(w)) {
        Some(1)
    } else {
        Some(2)
    }
}

/// The best `PER_GROUP` of `items` (with their scores), best first and otherwise in order.
fn best<T>(mut items: Vec<(u32, usize, T)>) -> Vec<T> {
    items.sort_by_key(|(s, i, _)| (*s, *i));
    items.into_iter().take(PER_GROUP).map(|(_, _, t)| t).collect()
}

/// Lays `text` out on one line no wider than `width`, ending in "…" when it is cut short.
fn one_line(ui: &egui::Ui, parts: &[(&str, egui::Color32)], font: &egui::FontId, width: f32) -> Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    for (t, c) in parts {
        job.append(t, 0.0, TextFormat { font_id: font.clone(), color: *c, ..Default::default() });
    }
    job.wrap = egui::text::TextWrapping { max_width: width.max(1.0), max_rows: 1, break_anywhere: true, overflow_character: Some('\u{2026}') };
    ui.fonts(|f| f.layout_job(job))
}

/// A result row: fixed height, the label on the left, `right` at the right edge.
fn row(ui: &mut egui::Ui, h: &Hit, picked: bool) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, ROW_H), if h.enabled { Sense::click() } else { Sense::hover() });
    let v = ui.visuals();
    if picked {
        ui.painter().rect_filled(rect, 4.0, v.selection.bg_fill);
    } else if h.enabled && resp.hovered() {
        ui.painter().rect_filled(rect, 4.0, v.widgets.hovered.weak_bg_fill);
    }
    let (strong, weak) = if !h.enabled {
        (v.weak_text_color(), v.weak_text_color())
    } else if picked {
        (v.text_color(), v.text_color())
    } else {
        (v.text_color(), v.weak_text_color())
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let inner = rect.shrink2(egui::vec2(8.0, 0.0));
    let right = if h.right.is_empty() { None } else { Some(one_line(ui, &[(&h.right, weak)], &font, inner.width() * 0.4)) };
    let gap = right.as_ref().map_or(0.0, |g| g.size().x + 12.0);
    let sub = if h.sub.is_empty() { String::new() } else { format!("  {}", h.sub) };
    let left = one_line(ui, &[(&h.label, strong), (&sub, weak)], &font, inner.width() - gap);
    let painter = ui.painter();
    painter.galley(egui::pos2(inner.left(), rect.center().y - left.size().y / 2.0), left, strong);
    if let Some(g) = right {
        painter.galley(egui::pos2(inner.right() - g.size().x, rect.center().y - g.size().y / 2.0), g, weak);
    }
    let name = h.label.clone();
    let enabled = h.enabled;
    resp.widget_info(move || egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &name));
    match &h.tip {
        Some(t) => resp.on_hover_text(t),
        None => resp,
    }
}

impl App {
    /// Opens Search everything (Edit menu, Ctrl+K).
    pub(super) fn open_palette(&mut self) {
        if self.palette.is_none() {
            self.palette = Some(Palette::default());
        }
    }

    /// Ctrl+K, and the box while it is open. Called once a frame, before the panels, so the
    /// keys it takes (arrows, Enter, Escape) never reach what is under it.
    pub(super) fn palette_frame(&mut self, ctx: &egui::Context) {
        // Like Ctrl+F, it works wherever the keyboard focus is.
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) {
            if self.palette.is_some() {
                self.palette = None;
                return;
            }
            // Not over a dialog that is asking something, nor while a file opens.
            // (The mark this box leaves for a frame after it closes is not a dialog.)
            let ours = ctx.data(|d| d.get_temp::<u64>(super::modal_key()) == d.get_temp::<u64>(egui::Id::new("palette_pass")));
            if !(modal_shown(ctx) && !ours) && self.loading.is_none() {
                self.open_palette();
            }
        }
        if self.palette.is_none() {
            return;
        }
        let (down, up, enter, escape) = ctx.input_mut(|i| {
            let mut n = |k| {
                let mut c = 0;
                while i.consume_key(Modifiers::NONE, k) {
                    c += 1;
                }
                c
            };
            (n(Key::ArrowDown), n(Key::ArrowUp), n(Key::Enter) > 0, n(Key::Escape) > 0)
        });
        if escape {
            self.palette = None;
            return;
        }
        let hits = self.palette_hits(ctx);
        let Some(p) = &mut self.palette else { return };
        if p.seen.as_deref() != Some(p.query.as_str()) {
            p.seen = Some(p.query.clone());
            p.sel = hits.iter().position(|h| h.enabled);
            p.scroll = true;
        }
        if p.sel.is_some_and(|s| s >= hits.len() || !hits[s].enabled) {
            p.sel = hits.iter().position(|h| h.enabled);
        }
        for _ in 0..down {
            if let Some(n) = hits.iter().enumerate().skip(p.sel.map_or(0, |s| s + 1)).find(|(_, h)| h.enabled).map(|(i, _)| i) {
                p.sel = Some(n);
                p.scroll = true;
            }
        }
        for _ in 0..up {
            if let Some(n) = hits.iter().enumerate().take(p.sel.unwrap_or(0)).rev().find(|(_, h)| h.enabled).map(|(i, _)| i) {
                p.sel = Some(n);
                p.scroll = true;
            }
        }
        let mut run = if enter { p.sel } else { None };
        let mut close = false;

        // Dims the window behind and blocks it like a dialog; a click outside the box closes it.
        let pass = ctx.cumulative_pass_nr();
        ctx.data_mut(|d| {
            d.insert_temp(super::modal_key(), pass);
            d.insert_temp(egui::Id::new("palette_pass"), pass);
        });
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("palette_dim")).fixed_pos(screen.min).sense(widgets::BLOCK_CLICKS).order(egui::Order::Middle).show(ctx, |ui| {
            if ui.allocate_response(screen.size(), widgets::BLOCK_CLICKS).clicked() {
                close = true;
            }
            ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha(140));
        });
        let width = fit(ctx, 560.0);
        let top = (screen.height() * 0.08).clamp(8.0, 72.0);
        egui::Window::new(tr("Search everything"))
            .id(egui::Id::new("palette"))
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_TOP, [0.0, top])
            .show(ctx, |ui| {
                ui.set_width(width);
                let hint = tr("Search");
                let r = ui.add(TextEdit::singleline(&mut p.query).id(egui::Id::new("palette_box")).hint_text(hint).desired_width(f32::INFINITY));
                let r = widgets::named(r, egui::WidgetType::TextEdit, tr("Search everything"));
                // Typing always goes to the box; the rows are picked with the arrow keys or a click.
                if !r.has_focus() {
                    r.request_focus();
                }
                if p.query.trim().is_empty() {
                    ui.weak(tr("Find a menu command, a game, a recent file or a value on the open save. Up and Down pick a row, Enter runs it, Esc closes."));
                } else if hits.is_empty() {
                    ui.weak(tr("Nothing found."));
                }
                if hits.is_empty() {
                    return;
                }
                ui.add_space(4.0);
                let scroll = std::mem::take(&mut p.scroll);
                egui::ScrollArea::vertical().show_list(ui, "palette_list", 360.0, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mut last = None;
                    for (i, h) in hits.iter().enumerate() {
                        if last != Some(h.group) {
                            if last.is_some() {
                                ui.add_space(6.0);
                            }
                            last = Some(h.group);
                            ui.add_space(2.0);
                            ui.label(RichText::new(h.group.title()).small().strong());
                            ui.add_space(2.0);
                        }
                        let picked = p.sel == Some(i);
                        let resp = row(ui, h, picked);
                        if picked && scroll {
                            resp.scroll_to_me(None);
                        }
                        if resp.clicked() {
                            run = Some(i);
                        }
                    }
                });
            });
        if close {
            self.palette = None;
            return;
        }
        if let Some(i) = run {
            if let Some(h) = hits.get(i) {
                self.run_hit(&h.act, ctx);
            }
        }
    }

    /// What matches the text typed in the open box: commands, games, recent files, then values
    /// of the open save, each group best first.
    pub(super) fn palette_hits(&mut self, ctx: &egui::Context) -> Vec<Hit> {
        let Some(q) = self.palette.as_ref().map(|p| p.query.clone()) else { return vec![] };
        let w = words(&q);
        let mut out = vec![];
        if w.is_empty() {
            // Nothing typed: the files opened last, ready to open again.
            out.extend(self.recent_hits(&w));
            return out;
        }
        let cmds: Vec<(u32, usize, Hit)> = COMMANDS
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| {
                let enabled = c.state(self, ctx)?;
                let s = score(&w, &c.search_text())?;
                let right = match c.ticked(self) {
                    Some(on) => (if on { tr("On") } else { tr("Off") }).to_string(),
                    None => c.shortcut().unwrap_or_default().to_string(),
                };
                Some((s, i, Hit { group: Group::Commands, act: Act::Cmd(c), label: c.label(), sub: String::new(), right, enabled, tip: None }))
            })
            .collect();
        out.extend(best(cmds));
        if HAS_GAMES {
            let games: Vec<(u32, usize, Hit)> = crate::games::GAMES
                .iter()
                .enumerate()
                .filter(|(_, g)| crate::games::name_matches(g.name, &q))
                .map(|(i, g)| {
                    let s = score(&w, g.name).unwrap_or(3);
                    (s, i, Hit { group: Group::Games, act: Act::Game(i), label: g.name.to_string(), sub: String::new(), right: String::new(), enabled: true, tip: None })
                })
                .collect();
            out.extend(best(games));
        }
        out.extend(self.recent_hits(&w));
        out.extend(self.value_hits(&q, &w));
        out
    }

    fn recent_hits(&self, w: &[String]) -> Vec<Hit> {
        let recent: Vec<(u32, usize, Hit)> = self
            .settings
            .recent
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let label = recent_label(p);
                let s = score(w, &format!("{label}\n{}", p.display()))?;
                let enabled = crate::vfs::exists(p);
                Some((s, i, Hit { group: Group::Recent, act: Act::Recent(p.clone()), label, sub: String::new(), right: String::new(), enabled, tip: Some(p.display().to_string()) }))
            })
            .collect();
        best(recent)
    }

    /// Values of the open save whose name, place, key or value has every typed word.
    fn value_hits(&mut self, q: &str, w: &[String]) -> Vec<Hit> {
        let Some(top) = self.doc.as_ref().and_then(|d| root(&d.content)) else { return vec![] };
        if q.trim().chars().count() < VALUE_MIN_CHARS || self.model.rows.is_empty() {
            return vec![];
        }
        let key = (q.trim().to_lowercase(), self.model_gen, self.edit_gen);
        let cached = self.palette.as_ref().and_then(|p| p.values.as_ref()).filter(|(k, _)| *k == key).map(|(_, v)| v.clone());
        let rows = match cached {
            Some(v) => v,
            None => {
                // The Values list's own search finds the rows with the longest word; the rest
                // must be in the same row too.
                let first = w.iter().max_by_key(|s| s.chars().count()).cloned().unwrap_or_default();
                let disp = values_view::filter(&self.model, top, &first, true, &HashSet::new());
                let advanced = self.settings.show_advanced;
                let mut found: Vec<(u32, usize, usize)> = vec![];
                for d in disp {
                    let values_view::Disp::Item(i) = d else { continue };
                    let r = &self.model.rows[i];
                    // Rows the Values list can show (a rarely needed section opens when jumped to).
                    if !(r.editable || advanced) {
                        continue;
                    }
                    let shown = top.pointer(&r.ptr).map(widgets::display).unwrap_or_default();
                    let text = format!("{}\n{}\n{}\n{}\n{shown}", r.name(), r.context(), r.raw_key(), self.model.sections[r.section].title);
                    if let Some(s) = score(w, &text) {
                        // A name that matches beats a value or a place that does.
                        let s = if score(w, r.name()).is_some() { s } else { s + 3 };
                        found.push((s, i, i));
                    }
                }
                found.sort_by_key(|(s, i, _)| (*s, *i));
                let v: Vec<usize> = found.into_iter().take(PER_GROUP).map(|(_, _, i)| i).collect();
                if let Some(p) = &mut self.palette {
                    p.values = Some((key, v.clone()));
                }
                v
            }
        };
        let Some(top) = self.doc.as_ref().and_then(|d| root(&d.content)) else { return vec![] };
        rows.into_iter()
            .filter(|&i| i < self.model.rows.len())
            .map(|i| {
                let r = &self.model.rows[i];
                let shown = top.pointer(&r.ptr).map(widgets::display).unwrap_or_default();
                let shown: String = shown.lines().next().unwrap_or("").chars().take(60).collect();
                Hit { group: Group::Values, act: Act::Value(i), label: r.name().to_string(), sub: r.context().to_string(), right: shown, enabled: true, tip: None }
            })
            .collect()
    }

    /// Runs a picked row and closes the box. A row that can't run now (a greyed-out command,
    /// a recent file that is gone) does nothing and leaves the box open. Returns whether it ran.
    pub(super) fn run_hit(&mut self, act: &Act, ctx: &egui::Context) -> bool {
        match act {
            Act::Cmd(c) => {
                if c.state(self, ctx) != Some(true) {
                    return false;
                }
                self.palette = None;
                self.run_cmd(*c, ctx);
            }
            Act::Game(i) => {
                if !HAS_GAMES || *i >= crate::games::GAMES.len() {
                    return false;
                }
                self.palette = None;
                let mut gp = self.picker();
                gp.picked = Some(*i);
                self.game_picker = Some(gp);
            }
            Act::Recent(p) => {
                if !crate::vfs::exists(p) {
                    return false;
                }
                self.palette = None;
                self.request_open(p.clone());
            }
            Act::Value(i) => {
                if *i >= self.model.rows.len() {
                    return false;
                }
                self.palette = None;
                self.jump_to_row(*i);
            }
        }
        true
    }

    /// What the command's menu item does when clicked.
    fn run_cmd(&mut self, c: Cmd, ctx: &egui::Context) {
        match c {
            Cmd::Open => self.ask(Ask::Open, None, None),
            Cmd::OpenInPlace => {
                #[cfg(target_arch = "wasm32")]
                self.open_in_place();
            }
            Cmd::FindSaves => self.game_picker = Some(self.picker()),
            Cmd::Save => self.save_clicked(),
            Cmd::SaveAs => self.save_as(),
            Cmd::Reload => {
                if let Some(p) = self.doc.as_ref().map(|d| d.path.clone()) {
                    self.request_open(p);
                }
            }
            Cmd::CloseSave => self.close_clicked(),
            Cmd::RevertAll => self.revert_all(),
            Cmd::Export => self.export_decoded(),
            Cmd::Import => self.ask(Ask::Import, None, None),
            Cmd::BackUpNow => self.backup_name = Some(String::new()),
            Cmd::BackUpNewGame => self.new_game_name = Some(String::new()),
            Cmd::SaveManager => self.open_save_manager(),
            Cmd::BackupSpace => self.open_backup_space(),
            Cmd::Undo => self.undo_redo(true),
            Cmd::Redo => self.undo_redo(false),
            Cmd::Find => {
                self.focus_search = true;
                self.focus_game_find = self.tab == Tab::Game;
            }
            Cmd::CopyChanges => self.copy_changes(ctx),
            Cmd::Preferences => self.prefs_open = true,
            Cmd::ShowAdvanced => {
                self.settings.show_advanced = !self.settings.show_advanced;
                self.settings.save();
            }
            Cmd::ShowRawKeys => {
                self.settings.show_raw_keys = !self.settings.show_raw_keys;
                self.settings.save();
            }
            Cmd::ChangesPanel => self.show_changes = !self.show_changes,
            Cmd::ShowNote => {
                let key = Settings::note_key(&self.doc_note);
                self.settings.hidden_notes.retain(|k| *k != key);
                self.settings.save();
            }
            Cmd::ReviewBeforeSave => {
                self.settings.confirm_save = !self.settings.confirm_save;
                self.settings.save();
            }
            Cmd::ThemeDark | Cmd::ThemeLight | Cmd::ThemeSystem => {
                let s = &mut self.settings;
                let pick = match c {
                    Cmd::ThemeDark => (false, true),
                    Cmd::ThemeLight => (false, false),
                    _ => (true, s.dark),
                };
                if pick != (s.follow_system, s.dark) {
                    (s.follow_system, s.dark) = pick;
                    theme::apply_choice(ctx, s.dark, s.follow_system);
                    s.save();
                }
            }
            Cmd::DecodedAsHex => self.view_as_hex(),
            Cmd::LocateData => self.locate_data_dir(),
            Cmd::Decrypt => self.password_prompt = Some(self.doc.as_ref().and_then(|d| d.opts.password.clone()).unwrap_or_default()),
            Cmd::ChangeSteam => self.start_rebind(),
            Cmd::CheatPatches => self.start_patches(),
            Cmd::FindChecksum => self.find_sums(),
            Cmd::TemplatesFolder => self.open_templates_folder(),
            Cmd::PresetSave => self.start_preset_save(),
            Cmd::PresetApply => self.start_preset_apply(),
            Cmd::PresetOpen => self.start_preset_open(),
            Cmd::Compare => self.start_compare(),
            Cmd::CopySlot => {
                if let Some((game, slots)) = self.file_slots(ctx) {
                    self.start_copy_slot(game, slots);
                }
            }
            Cmd::XboxSteam => {
                if let Some(src) = self.xbox_steam_source() {
                    self.start_xbox_steam(src);
                }
            }
            Cmd::Reveal => {
                if let Some(d) = &self.doc {
                    platform::reveal(&d.path);
                }
            }
            Cmd::Shortcuts => self.help = Some(help::Window::Shortcuts),
            Cmd::SupportedGames => {
                if WEB || cfg!(target_os = "android") {
                    help::open_url(ctx, help::SITE);
                } else {
                    let mut gp = self.picker();
                    gp.only_found = Some(false);
                    self.game_picker = Some(gp);
                }
            }
            Cmd::Website => help::open_url(ctx, help::SITE),
            Cmd::Report => {
                let kind = self.doc.as_ref().map(|d| friendly_kind(&d.layers, &d.content));
                help::open_url(ctx, &help::report_url(kind.as_deref()));
            }
            Cmd::RequestGame => help::open_url(ctx, &help::request_url()),
            Cmd::Questions => help::open_url(ctx, help::DISCUSSIONS),
            Cmd::CheckUpdate => self.start_update_check(true),
            Cmd::Licences => self.help = Some(help::Window::Licences),
            Cmd::About => self.help = Some(help::Window::About),
        }
    }

    /// Tools › Templates folder: makes the folder (with its guide) and shows it.
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    pub(super) fn open_templates_folder(&mut self) {
        match crate::fmt::template::ensure_dir() {
            Ok(d) => {
                crate::platform::reveal(&d.join("README.txt"));
                self.ok(trf("Templates folder: {0}", &[&d.display()]));
            }
            Err(e) => self.fail(trf("Couldn't make the templates folder: {0}", &[&e])),
        }
    }

    #[cfg(any(target_arch = "wasm32", target_os = "android"))]
    pub(super) fn open_templates_folder(&mut self) {}

    /// Shows row `i` of the Values list: switches to it, opens its section (and the rarely
    /// needed values, when it is one) and scrolls to it.
    pub(super) fn jump_to_row(&mut self, i: usize) {
        let Some(sec) = self.model.rows.get(i).map(|r| r.section) else { return };
        // Text typed on the Raw JSON tab is applied first, as leaving that tab does; when it
        // isn't valid the tab stays, with the error shown.
        if self.tab == Tab::Raw && !self.commit_raw() {
            return;
        }
        self.tab = Tab::Values;
        self.search.clear();
        if self.collapsed.remove(&sec) {
            self.collapsed_gen += 1;
        }
        if self.model.sections[sec].advanced && !self.settings.show_advanced {
            self.settings.show_advanced = true;
        }
        self.scroll_to_row = Some(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::layout_tests::cut_off_text;
    use serde_json::json;

    fn input(w: f32, h: f32) -> egui::RawInput {
        egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h))), ..Default::default() }
    }

    fn window() -> egui::Context {
        let ctx = egui::Context::default();
        theme::install_fonts(&ctx);
        theme::apply(&ctx, true);
        ctx
    }

    fn app(ctx: &egui::Context) -> App {
        let mut app = App::with(ctx, Settings::default(), false);
        // No search of this computer's drives from a test.
        app.saves_found = Some(std::sync::Arc::new(std::sync::Mutex::new(vec![Some(vec![]); crate::games::GAMES.len()])));
        app
    }

    fn key(k: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers }
    }

    fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut raw = input(1280.0, 800.0);
        raw.events = events;
        raw.time = Some(ctx.input(|i| i.time) + 0.5);
        ctx.run(raw, |ctx| app.frame(ctx))
    }

    /// Opens the box with Ctrl+K and types `text` into it.
    fn search(app: &mut App, ctx: &egui::Context, text: &str) {
        frame(app, ctx, vec![]);
        frame(app, ctx, vec![key(Key::K, Modifiers::COMMAND)]);
        assert!(app.palette.is_some(), "Ctrl+K opens the box");
        frame(app, ctx, vec![]);
        frame(app, ctx, vec![egui::Event::Text(text.into())]);
        assert_eq!(app.palette.as_ref().map(|p| p.query.as_str()), Some(text), "typing goes into the box");
        frame(app, ctx, vec![]);
    }

    /// A plain JSON save with many values, the last one called "gold", far down the list.
    fn json_save(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("use-palette-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("slot1.json");
        let mut items = serde_json::Map::new();
        for i in 0..300 {
            items.insert(format!("flag_{i:03}"), json!(i));
        }
        let v = json!({"world": items, "player": {"name": "Ada", "level": 7, "gold": 4321}});
        std::fs::write(&path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
        path
    }

    fn texts(out: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
        let mut v = vec![];
        for cs in &out.shapes {
            let mut stack = vec![&cs.shape];
            while let Some(sh) = stack.pop() {
                match sh {
                    egui::epaint::Shape::Vec(x) => stack.extend(x.iter()),
                    egui::epaint::Shape::Text(t) => v.push((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2()).intersect(cs.clip_rect))),
                    _ => {}
                }
            }
        }
        v
    }

    #[test]
    fn words_match_in_any_order_and_case_and_the_best_come_first() {
        let w = words("  SAVE copy ");
        assert_eq!(w, vec!["save", "copy"]);
        assert!(score(&w, "Save a copy as\u{2026}").is_some());
        assert!(score(&w, "Copy the save").is_some());
        assert_eq!(score(&w, "Save manager\u{2026}"), None, "every word must be there");
        let s = words("save");
        // Starts with what was typed, then a word that starts with it, then inside a word.
        assert_eq!(score(&s, "Save"), Some(0));
        assert_eq!(score(&s, "Back up and save"), Some(1));
        assert_eq!(score(&s, "Unsaved"), Some(2));
        // The English text (on its own line) counts as a start too.
        assert_eq!(score(&s, "Speichern\nSave"), Some(0));
        let ranked = best(vec![(2, 0, "Unsaved"), (1, 1, "Back up and save"), (0, 2, "Save"), (0, 3, "Save a copy as")]);
        assert_eq!(ranked, vec!["Save", "Save a copy as", "Back up and save", "Unsaved"]);
        assert_eq!(best((0..20).map(|i| (0, i, i)).collect()).len(), PER_GROUP, "each group is capped");
    }

    #[test]
    fn commands_are_found_by_their_translated_or_english_name() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("de"));
        let ctx = window();
        let mut a = app(&ctx);
        let found = |a: &mut App, q: &str| {
            a.palette = Some(Palette { query: q.into(), ..Default::default() });
            a.palette_hits(&ctx).into_iter().map(|h| h.act).collect::<Vec<_>>()
        };
        let english = found(&mut a, "keyboard shortcuts");
        let german = found(&mut a, &tr("Keyboard shortcuts").to_lowercase());
        crate::i18n::set(Some("en"));
        assert!(english.contains(&Act::Cmd(Cmd::Shortcuts)), "{english:?}");
        assert!(german.contains(&Act::Cmd(Cmd::Shortcuts)), "{german:?}");
        // Every command has its own text (no two menu items look alike in the list).
        let labels: HashSet<String> = COMMANDS.iter().map(|c| c.label()).collect();
        assert_eq!(labels.len(), COMMANDS.len());
    }

    #[test]
    fn enter_runs_the_command_the_menu_item_runs() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = window();
        let mut a = app(&ctx);
        search(&mut a, &ctx, "keyboard short");
        let hits = a.palette_hits(&ctx);
        assert_eq!(hits.first().map(|h| &h.act), Some(&Act::Cmd(Cmd::Shortcuts)));
        frame(&mut a, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        assert!(a.palette.is_none(), "running a command closes the box");
        assert!(a.help == Some(help::Window::Shortcuts), "Help \u{203A} Keyboard shortcuts opened");

        // Down picks the next row; Enter runs that one.
        a.help = None;
        search(&mut a, &ctx, "theme");
        let hits = a.palette_hits(&ctx);
        let acts: Vec<&Act> = hits.iter().map(|h| &h.act).collect();
        assert_eq!(acts[..3], [&Act::Cmd(Cmd::ThemeDark), &Act::Cmd(Cmd::ThemeLight), &Act::Cmd(Cmd::ThemeSystem)]);
        a.settings.follow_system = false;
        a.settings.dark = true;
        frame(&mut a, &ctx, vec![key(Key::ArrowDown, Modifiers::NONE)]);
        frame(&mut a, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        assert!(!a.settings.dark && !a.settings.follow_system, "View \u{203A} Theme: Light was picked");

        // A setting from the View menu is switched as its tick box would be.
        let before = a.settings.show_raw_keys;
        search(&mut a, &ctx, "raw key names");
        frame(&mut a, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        assert_eq!(a.settings.show_raw_keys, !before);

        // Ctrl+K closes it again.
        frame(&mut a, &ctx, vec![key(Key::K, Modifiers::COMMAND)]);
        assert!(a.palette.is_some());
        frame(&mut a, &ctx, vec![key(Key::K, Modifiers::COMMAND)]);
        assert!(a.palette.is_none());
    }

    #[test]
    fn escape_closes_only_the_box() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = window();
        let mut a = app(&ctx);
        a.help = Some(help::Window::About);
        search(&mut a, &ctx, "pref");
        frame(&mut a, &ctx, vec![key(Key::Escape, Modifiers::NONE)]);
        assert!(a.palette.is_none(), "Escape closes the box");
        assert!(a.help == Some(help::Window::About), "the window under it stays open");
        assert!(!a.prefs_open, "nothing ran");
    }

    #[test]
    fn a_greyed_out_command_does_not_run() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = window();
        let mut a = app(&ctx);
        // No save open: Close save is listed, greyed out, and Enter does nothing.
        search(&mut a, &ctx, "close save");
        let hits = a.palette_hits(&ctx);
        let close = hits.iter().find(|h| h.act == Act::Cmd(Cmd::CloseSave)).expect("listed");
        assert!(!close.enabled);
        assert!(hits.iter().all(|h| !h.enabled || h.act != Act::Cmd(Cmd::CloseSave)));
        frame(&mut a, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        assert!(!a.run_hit(&Act::Cmd(Cmd::CloseSave), &ctx));
        assert!(!a.run_hit(&Act::Cmd(Cmd::Save), &ctx));
        assert!(!a.run_hit(&Act::Cmd(Cmd::RevertAll), &ctx));
        assert!(a.palette.is_some(), "the box stays open");
        assert!(a.saving.is_none() && !a.pending_close);
        // A recent file that is gone can't be opened either.
        let gone = std::env::temp_dir().join("use-palette-no-such-file.sav");
        assert!(!a.run_hit(&Act::Recent(gone), &ctx));
        assert!(a.loading.is_none() && a.doc.is_none());
    }

    #[test]
    fn a_value_found_is_shown_in_the_values_list() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = json_save("jump");
        let ctx = window();
        let mut a = app(&ctx);
        a.load(&path);
        assert!(a.doc.is_some(), "{}", a.status);
        // A search in the list would hide the value; the jump clears it.
        a.search = "flag_001".into();
        let out = frame(&mut a, &ctx, vec![]);
        assert!(!texts(&out).iter().any(|(t, _)| t.eq_ignore_ascii_case("gold")), "gold starts off screen");
        // Its name and its value: games named Gold... don't match that.
        search(&mut a, &ctx, "gold 4321");
        let hits = a.palette_hits(&ctx);
        let gold = hits.iter().position(|h| h.group == Group::Values && h.label.eq_ignore_ascii_case("gold")).expect("gold is found");
        // Only the value matches: it is the first row and Enter shows it.
        assert_eq!(gold, 0, "{hits:?}");
        assert_eq!(hits[gold].right, "4321");
        frame(&mut a, &ctx, vec![key(Key::Enter, Modifiers::NONE)]);
        assert!(a.palette.is_none());
        assert!(a.tab == Tab::Values && a.search.is_empty());
        let mut out = frame(&mut a, &ctx, vec![]);
        for _ in 0..3 {
            out = frame(&mut a, &ctx, vec![]);
        }
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
        assert!(texts(&out).iter().any(|(t, r)| t.eq_ignore_ascii_case("gold") && r.height() > 4.0 && screen.contains_rect(*r)), "gold is on screen after the jump");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_game_opens_find_my_games_saves_on_it() {
        if !HAS_GAMES {
            return;
        }
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let ctx = window();
        let mut a = app(&ctx);
        let name = crate::games::GAMES[0].name;
        a.palette = Some(Palette { query: name.into(), ..Default::default() });
        let hits = a.palette_hits(&ctx);
        let i = hits.iter().find_map(|h| if let Act::Game(i) = h.act { Some(i) } else { None }).expect("the game is found");
        assert_eq!(crate::games::GAMES[i].name, name);
        assert!(a.run_hit(&Act::Game(i), &ctx));
        assert_eq!(a.game_picker.as_ref().and_then(|g| g.picked), Some(i));
        assert!(a.palette.is_none());
    }

    #[test]
    fn a_recent_file_opens() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = json_save("recent");
        let ctx = window();
        let mut a = app(&ctx);
        a.settings.recent = vec![path.clone()];
        a.palette = Some(Palette::default());
        // Nothing typed: the recent files are offered.
        let hits = a.palette_hits(&ctx);
        assert_eq!(hits.len(), 1);
        assert!(a.run_hit(&hits[0].act, &ctx));
        for _ in 0..20 {
            if a.doc.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
            frame(&mut a, &ctx, vec![]);
        }
        assert_eq!(a.doc.as_ref().map(|d| d.path.clone()), Some(path.clone()));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn the_box_fits_and_nothing_is_cut_off_in_every_language() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = json_save("layout");
        let long = "C:\\Users\\SomeOne\\AppData\\Local\\Packages\\Some.Very.Long.PublisherName.GameTitle_8wekyb3d8bbwe\\SystemAppData\\wgs\\Slot1";
        let mut problems = vec![];
        for lang in ["en", "de", "es", "fr", "pt-BR", "ja", "zh-CN"] {
            crate::i18n::set(Some(lang));
            for (w, h) in [(1280.0, 800.0), (380.0, 760.0)] {
                for q in ["", "s", "a", "save", "gold", "zzzzqx"] {
                    let ctx = window();
                    if matches!(lang, "ja" | "zh-CN") {
                        theme::enable_cjk(&ctx);
                    }
                    let mut a = app(&ctx);
                    a.load(&path);
                    a.settings.recent = (0..10).map(|i| PathBuf::from(format!("{long}\\a_rather_long_save_file_name_{i}.sav"))).collect();
                    a.palette = Some(Palette { query: q.into(), ..Default::default() });
                    let mut out = None;
                    for _ in 0..4 {
                        let mut raw = input(w, h);
                        raw.time = Some(ctx.input(|i| i.time) + 0.5);
                        out = Some(ctx.run(raw, |ctx| a.palette_frame(ctx)));
                    }
                    let out = out.unwrap();
                    let who = format!("{lang} at {w}x{h} with {q:?}");
                    match ctx.memory(|m| m.area_rect(egui::Id::new("palette"))) {
                        Some(r) => {
                            if r.left() < -0.5 || r.top() < -0.5 || r.right() > w + 0.5 || r.bottom() > h + 0.5 {
                                problems.push(format!("{who}: the box {r:?} leaves the window"));
                            }
                        }
                        None => problems.push(format!("{who}: no box")),
                    }
                    for t in cut_off_text(&out) {
                        problems.push(format!("{who}: {t}"));
                    }
                    if a.palette.is_none() {
                        problems.push(format!("{who}: the box closed by itself"));
                    }
                }
            }
        }
        crate::i18n::set(Some("en"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        problems.sort();
        problems.dedup();
        assert!(problems.is_empty(), "{} layout problems:\n{}", problems.len(), problems.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
    }
}
