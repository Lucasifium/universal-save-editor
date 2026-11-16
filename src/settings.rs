//! Small persisted preferences, stored in the app's config folder (on Windows,
//! %APPDATA%\UniversalSaveEditor\settings.json): theme, zoom, recent files and view toggles.

use serde_json::{json, Value};
use std::path::PathBuf;

pub const MAX_RECENT: usize = 12;

#[derive(Clone)]
pub struct Settings {
    pub recent: Vec<PathBuf>,
    pub dark: bool,
    pub zoom: f32,
    pub show_raw_keys: bool,
    pub show_advanced: bool,
    /// Show the list of changes and ask before writing a save.
    pub confirm_save: bool,
    /// Where backups go, when the player chose a folder (otherwise the app's own folder).
    pub backup_dir: Option<PathBuf>,
    /// Folders chosen for backups before, still searched for the copies made there.
    pub earlier_backup_dirs: Vec<PathBuf>,
    /// Show the step-by-step "find a value" guide in the hex view.
    pub hex_guide: bool,
    /// Automatic backups kept per save.
    pub backup_keep: usize,
    /// Interface language code ("de"); `None` follows the system.
    pub language: Option<String>,
    /// Check GitHub for a newer release when the program starts (off until the player asks).
    pub check_updates: bool,
    /// Also search removable and network drives for saves.
    pub search_removable: bool,
    /// The newest version the player dismissed, so it isn't offered again.
    pub dismissed_update: Option<String>,
    /// File notes (the line under the toolbar) the player dismissed, so they stay hidden.
    pub hidden_notes: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { recent: vec![], dark: true, zoom: 1.0, show_raw_keys: false, show_advanced: false, confirm_save: true, backup_dir: None, earlier_backup_dirs: vec![], hex_guide: true, backup_keep: crate::backups::KEEP, language: None, check_updates: false, search_removable: false, dismissed_update: None, hidden_notes: vec![] }
    }
}

fn path() -> Option<PathBuf> {
    crate::platform::config_dir().map(|d| d.join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        let mut s = Self::default();
        let Some(v) = path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<Value>(&t).ok()) else {
            return s;
        };
        if let Some(r) = v["recent"].as_array() {
            s.recent = r.iter().filter_map(|x| x.as_str()).map(PathBuf::from).collect();
        }
        s.dark = v["dark"].as_bool().unwrap_or(s.dark);
        s.zoom = v["zoom"].as_f64().map(|z| z as f32).filter(|z| (0.5..=3.0).contains(z)).unwrap_or(1.0);
        s.show_raw_keys = v["show_raw_keys"].as_bool().unwrap_or(false);
        s.show_advanced = v["show_advanced"].as_bool().unwrap_or(false);
        s.confirm_save = v["confirm_save"].as_bool().unwrap_or(true);
        s.hex_guide = v["hex_guide"].as_bool().unwrap_or(true);
        s.backup_dir = v["backup_dir"].as_str().filter(|d| !d.is_empty()).map(PathBuf::from);
        if let Some(r) = v["earlier_backup_dirs"].as_array() {
            s.earlier_backup_dirs = r.iter().filter_map(|x| x.as_str()).filter(|d| !d.is_empty()).map(PathBuf::from).collect();
        }
        s.backup_keep = v["backup_keep"].as_u64().map(|n| (n as usize).clamp(crate::backups::KEEP_MIN, crate::backups::KEEP_MAX)).unwrap_or(crate::backups::KEEP);
        s.language = v["language"].as_str().filter(|l| !l.is_empty()).map(str::to_string);
        s.check_updates = v["check_updates"].as_bool().unwrap_or(false);
        s.search_removable = v["search_removable"].as_bool().unwrap_or(false);
        s.dismissed_update = v["dismissed_update"].as_str().filter(|l| !l.is_empty()).map(str::to_string);
        if let Some(r) = v["hidden_notes"].as_array() {
            s.hidden_notes = r.iter().filter_map(|x| x.as_str()).map(str::to_string).collect();
        }
        s
    }

    pub fn save(&self) {
        // Tests never write the player's own settings.
        if cfg!(test) {
            return;
        }
        let Some(p) = path() else { return };
        let v = json!({
            "recent": self.recent.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
            "dark": self.dark,
            "zoom": self.zoom,
            "show_raw_keys": self.show_raw_keys,
            "show_advanced": self.show_advanced,
            "confirm_save": self.confirm_save,
            "hex_guide": self.hex_guide,
            "backup_dir": self.backup_dir.as_ref().map(|d| d.to_string_lossy().to_string()),
            "earlier_backup_dirs": self.earlier_backup_dirs.iter().map(|p| p.to_string_lossy()).collect::<Vec<_>>(),
            "backup_keep": self.backup_keep,
            "language": self.language,
            "check_updates": self.check_updates,
            "search_removable": self.search_removable,
            "dismissed_update": self.dismissed_update,
            "hidden_notes": self.hidden_notes,
        });
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, serde_json::to_string_pretty(&v).unwrap_or_default());
    }

    /// The key a file note is remembered by when dismissed.
    pub fn note_key(note: &str) -> String {
        note.chars().take(160).collect()
    }

    /// Switches backups to `dir` (`None`: the default), remembering the folder used until now
    /// so the copies already in it stay listed.
    pub fn set_backup_dir(&mut self, dir: Option<PathBuf>) {
        if let Some(old) = self.backup_dir.take() {
            if !self.earlier_backup_dirs.contains(&old) {
                self.earlier_backup_dirs.push(old);
            }
        }
        self.backup_dir = dir;
        crate::backups::set_root(self.backup_dir.clone());
        crate::backups::set_earlier_roots(self.earlier_backup_dirs.clone());
        self.save();
    }

    pub fn push_recent(&mut self, p: &std::path::Path) {
        // Kept absolute: a file opened by a relative path (from a terminal) was listed a second
        // time once it was opened by its full path, and stopped opening from another folder.
        let p = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
        self.recent.retain(|r| *r != p);
        self.recent.insert(0, p);
        self.recent.truncate(MAX_RECENT);
        self.save();
    }
}

/// Folders where games commonly keep saves, that exist on this machine.
pub fn save_locations() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    if cfg!(target_os = "android") {
        let phone = PathBuf::from("/storage/emulated/0");
        for (label, sub) in [("RetroArch saves", "RetroArch/saves"), ("RetroArch states", "RetroArch/states"), (crate::i18n::tr("Downloads"), "Download"), (crate::i18n::tr("Documents"), "Documents")] {
            let p = phone.join(sub);
            if p.is_dir() {
                out.push((label.to_string(), p));
            }
        }
        out.push((crate::i18n::tr("Phone storage").to_string(), phone));
        return out;
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    if let Some(appdata) = env("APPDATA") {
        // Ren'Py keeps one folder per game; list the most recently used ones.
        let renpy = appdata.join("RenPy");
        if let Ok(rd) = std::fs::read_dir(&renpy) {
            let mut games: Vec<(std::time::SystemTime, String, PathBuf)> = rd
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| {
                    let t = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                    let name = e.file_name().to_string_lossy().split('-').next().unwrap_or("").to_string();
                    (t, name, e.path())
                })
                .filter(|(_, n, _)| !n.is_empty() && n != "tokens" && n != "persistent")
                .collect();
            games.sort_by_key(|g| std::cmp::Reverse(g.0));
            for (_, name, path) in games.into_iter().take(6) {
                out.push((format!("Ren'Py: {name}"), path));
            }
        }
    }
    if let Some(home) = env("USERPROFILE") {
        for (label, p) in [
            ("Unity games (LocalLow)", home.join("AppData").join("LocalLow")),
            ("Saved Games", home.join("Saved Games")),
            ("Documents \u{203A} My Games", home.join("Documents").join("My Games")),
        ] {
            if p.is_dir() {
                out.push((label.to_string(), p));
            }
        }
    }
    if let Some(local) = env("LOCALAPPDATA") {
        out.push(("Local AppData (Unreal & others)".into(), local));
    }
    out
}
