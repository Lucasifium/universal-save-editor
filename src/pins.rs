//! Values the player pinned to the top of the Values list, kept for every save of the same
//! game, and the automatic "Common edits" group (money, levels, experience, health, points
//! and item counts) for saves that have no game screen of their own.
//!
//! Pins are kept in `pins.json` in the app's config folder:
//!
//! ```json
//! { "games": { "Unreal Engine save (PBSaveGameData) | Bloodstained": ["/Gold", "/Level"] } }
//! ```
//!
//! A game is told apart by the save's format together with the nearest folder above the save
//! that names something (folders like `Saves`, `remote` or a Steam account number are skipped).

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Pins kept per game.
pub const MAX_PER_GAME: usize = 200;

/// Most rows the "Common edits" group lists, and of those, most item counts.
pub const MAX_COMMON: usize = 40;
pub const MAX_COMMON_COUNTS: usize = 10;

/// `common_rank` of an item count.
pub const COUNT_RANK: u8 = 5;

#[derive(Default)]
pub struct Pins {
    /// Pinned value paths by game key, in the order they were pinned.
    games: BTreeMap<String, Vec<String>>,
    /// Where they are kept (`None`: in memory only, as in tests and the browser build).
    path: Option<PathBuf>,
}

fn store_path() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    crate::platform::config_dir().map(|d| d.join("pins.json"))
}

impl Pins {
    /// The player's pins (none when the file is missing or unreadable).
    pub fn load() -> Self {
        Self::load_from(store_path())
    }

    pub fn load_from(path: Option<PathBuf>) -> Self {
        let mut out = Pins { path, ..Default::default() };
        let Some(text) = out.path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) else { return out };
        let Ok(v) = serde_json::from_str::<Value>(&text) else { return out };
        if let Some(games) = v["games"].as_object() {
            for (k, list) in games {
                let ptrs: Vec<String> = list.as_array().into_iter().flatten().filter_map(|p| p.as_str()).filter(|p| p.starts_with('/')).take(MAX_PER_GAME).map(str::to_string).collect();
                if !ptrs.is_empty() {
                    out.games.insert(k.clone(), ptrs);
                }
            }
        }
        out
    }

    /// The pinned paths for this game, oldest pin first.
    pub fn of(&self, game: &str) -> &[String] {
        self.games.get(game).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn is_pinned(&self, game: &str, ptr: &str) -> bool {
        self.of(game).iter().any(|p| p == ptr)
    }

    /// Pins or unpins `ptr` for `game` and writes the file. Returns whether it is pinned now,
    /// or why the pins couldn't be kept (the change still holds until the app closes).
    pub fn toggle(&mut self, game: &str, ptr: &str) -> Result<bool, String> {
        let list = self.games.entry(game.to_string()).or_default();
        let now = match list.iter().position(|p| p == ptr) {
            Some(i) => {
                list.remove(i);
                false
            }
            None => {
                if list.len() >= MAX_PER_GAME {
                    return Err(crate::i18n::trf("You can pin up to {0} values per game. Unpin one first.", &[&MAX_PER_GAME]));
                }
                list.push(ptr.to_string());
                true
            }
        };
        if list.is_empty() {
            self.games.remove(game);
        }
        self.write()?;
        Ok(now)
    }

    /// Writes the pins next to the file first and then puts them in its place, so a crash
    /// half-way never leaves an empty file.
    fn write(&self) -> Result<(), String> {
        let Some(path) = &self.path else { return Ok(()) };
        let text = serde_json::to_string_pretty(&json!({ "games": self.games })).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            e.to_string()
        })
    }
}

/// Folder names that say nothing about which game a save is from.
const PLAIN_FOLDERS: &[&str] = &[
    "save", "saves", "savegame", "savegames", "saved", "savedata", "save data", "saved games", "savefiles", "savefile", "saving", "remote", "local", "data", "profiles", "profile", "users", "user",
    "userdata", "player", "players", "slots", "slot", "backup", "backups", "windows", "windowsnoeditor", "wingdk", "steam", "epic", "gog", "game", "games", "config", "persistent", "storage",
    "documents", "my games", "appdata", "roaming", "locallow", "www",
];

/// The key pins are kept under for a save at `path` whose format is `kind`.
pub fn game_key(path: &Path, kind: &str) -> String {
    let folder = path
        .ancestors()
        .skip(1)
        .filter_map(|a| a.file_name())
        .map(|n| n.to_string_lossy().trim().to_string())
        .map(|n| match n.rsplit_once('-') {
            // Ren'Py names its folders "<game>-<digits>".
            Some((g, d)) if !g.is_empty() && !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()) => g.to_string(),
            _ => n,
        })
        .find(|n| {
            let lc = n.to_lowercase();
            // Account numbers and ids; a short number (a Steam app id) does name the game.
            let account = lc.len() >= 8 && lc.bytes().all(|b| b.is_ascii_digit());
            let id_like = lc.len() >= 16 && lc.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-' || b == b'_' || b == b'{' || b == b'}');
            !n.is_empty() && !account && !id_like && !PLAIN_FOLDERS.contains(&lc.as_str())
        })
        .unwrap_or_default();
    format!("{kind} | {folder}")
}

/// What kind of common edit a value's key names, best first (`None`: not one).
pub fn common_rank(key: &str) -> Option<u8> {
    const MONEY: &[&str] = &["gold", "money", "cash", "credits", "credit", "coins", "coin", "currency", "souls", "runes", "gil", "zenny", "bells", "dollars", "silver", "gems", "funds", "geo", "caps", "scrap", "essence"];
    const LEVEL: &[&str] = &["level", "lvl", "lv"];
    const EXP: &[&str] = &["exp", "xp", "experience"];
    const HEALTH: &[&str] = &["hp", "health", "mp", "mana", "stamina"];
    const POINTS: &[&str] = &["points", "point", "sp", "ap"];
    const COUNT: &[&str] = &["count", "amount", "quantity", "qty", "stack", "num"];
    // Words that make it something else: a limit, a setting, an id, a time or a requirement.
    const NOT: &[&str] = &[
        "id", "ids", "type", "name", "icon", "index", "rate", "multiplier", "mult", "scale", "volume", "sound", "music", "audio", "required", "req", "next", "min", "cap", "limit", "threshold", "time",
        "timer", "date", "seed", "version", "difficulty", "graphics", "quality", "detail", "zoom", "speed", "regen", "chance", "percent", "ratio", "cost", "price", "save", "load", "map", "loading", "ui",
        "setting", "settings", "option", "options", "camera", "unlock", "unlocked", "blocklevel", "water", "light", "sensitivity", "tutorial", "fov", "brightness", "gamma", "slot", "spent", "used",
        "lost", "total", "earned", "collected", "gained", "spawn", "respawn", "flag", "hash", "random",
    ];
    let words: Vec<String> = crate::names::humanize(key).split(' ').map(str::to_lowercase).collect();
    if words.is_empty() || words.iter().any(|w| NOT.contains(&w.as_str())) {
        return None;
    }
    let has = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()));
    [MONEY, LEVEL, EXP, HEALTH, POINTS, COUNT].iter().position(|l| has(l)).map(|i| i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_are_told_apart_by_format_and_folder() {
        let k = |p: &str| game_key(Path::new(p), "JSON");
        assert_eq!(k("/home/p/.local/share/Hollow Game/Saves/slot1.json"), "JSON | Hollow Game");
        assert_eq!(k("C:/Users/p/AppData/LocalLow/Studio/Some Game/Profiles/76561198000000000/save.json"), "JSON | Some Game");
        assert_eq!(k("/a/Steam/userdata/12345678/987650/remote/save.dat"), "JSON | 987650");
        assert_eq!(k("/r/RenPy/MyStory-1617263540/1-1-LT1.save"), "JSON | MyStory");
        assert_eq!(k("save.json"), "JSON | ");
        assert_ne!(game_key(Path::new("/g/Game/a.sav"), "Unreal Engine save (A)"), game_key(Path::new("/g/Game/a.sav"), "Unreal Engine save (B)"));
    }

    #[test]
    fn common_edits_are_money_levels_experience_health_points_and_counts() {
        for (key, rank) in [("gold", 0), ("_partyGold", 0), ("PlayerMoney", 0), ("level", 1), ("charLv", 1), ("currentExp", 2), ("XP", 2), ("maxHp", 3), ("Health", 3), ("skillPoints", 4), ("itemCount", 5), ("quantity", 5)] {
            assert_eq!(common_rank(key), Some(rank), "{key}");
        }
        for key in ["goldId", "levelName", "musicVolume", "nextLevelExp", "requiredXp", "goldMultiplier", "MiniMapZoom", "ValidSaveGameData", "difficultyLevel", "totalGoldEarned", "playTime", "itemId"] {
            assert_eq!(common_rank(key), None, "{key}");
        }
    }

    #[test]
    fn pins_are_kept_per_game_and_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("use-pins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("pins.json");
        let mut p = Pins::load_from(Some(file.clone()));
        assert_eq!(p.toggle("A", "/gold"), Ok(true));
        assert_eq!(p.toggle("A", "/level"), Ok(true));
        assert_eq!(p.toggle("B", "/gold"), Ok(true));
        assert_eq!(p.toggle("B", "/gold"), Ok(false));
        let again = Pins::load_from(Some(file.clone()));
        assert_eq!(again.of("A"), ["/gold", "/level"]);
        assert!(again.of("B").is_empty());
        assert!(again.is_pinned("A", "/level") && !again.is_pinned("B", "/gold"));
        assert!(!dir.join("pins.json.tmp").exists());
        // A damaged file is ignored rather than stopping the app.
        std::fs::write(&file, "{ not json").unwrap();
        assert!(Pins::load_from(Some(file)).of("A").is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pins_per_game_are_capped() {
        let mut p = Pins::default();
        for i in 0..MAX_PER_GAME {
            assert_eq!(p.toggle("G", &format!("/v{i}")), Ok(true));
        }
        assert!(p.toggle("G", "/one_more").is_err());
        assert_eq!(p.toggle("G", "/v0"), Ok(false));
        assert_eq!(p.toggle("G", "/one_more"), Ok(true));
    }
}
