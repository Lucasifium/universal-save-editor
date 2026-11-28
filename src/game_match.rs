//! Which catalog game a file belongs to, from where it is and what it is called, for saves
//! that open without a game screen (an Unreal or JSON save shown as Values): the toolbar and
//! status bar can then name the game instead of only the format.

use crate::games::{wildcard, Base, GameInfo, GAMES};
use std::path::Path;

/// The folder names a game's saves sit under, for one of its places: the place's own folders
/// (`BloodstainedRotN\Saved\SaveGames`), with the Steam app id or install folder in front where
/// the place is under Steam. `None` for places that can't be told from a path (registry
/// values, Game Pass containers) or that name no folder of their own.
fn folders(base: Base, sub: &'static str) -> Option<Vec<&'static str>> {
    let mut v: Vec<&'static str> = match base {
        Base::SteamUser(id) | Base::SteamGame(id) => vec![id],
        Base::UnityPrefs | Base::GamePass(_) => return None,
        _ => vec![],
    };
    v.extend(sub.split(['\\', '/']).filter(|s| !s.is_empty()));
    // At least one real folder name: "*" alone or a bare "remote" says nothing.
    let named = v.iter().filter(|s| !s.contains('*') && s.len() > 2).count();
    let generic = |s: &str| matches!(s.to_ascii_lowercase().as_str(), "remote" | "saves" | "save" | "savegames" | "saved" | "profiles" | "my games");
    (named > 0 && v.iter().any(|s| !s.contains('*') && !generic(s))).then_some(v)
}

/// Whether `dirs` (the file's folders, outermost first) hold `want` one after another, with the
/// file at most `depth` folders below its end. Returns how many folder names matched.
fn match_at(dirs: &[String], want: &[&str], depth: usize) -> Option<usize> {
    if want.is_empty() || want.len() > dirs.len() {
        return None;
    }
    (0..=dirs.len() - want.len()).rev().find_map(|start| {
        let below = dirs.len() - start - want.len();
        let hit = want.iter().zip(&dirs[start..]).all(|(w, d)| wildcard(w, d));
        (hit && below < depth.max(1)).then_some(want.len())
    })
}

/// The catalog game whose save folder and file names fit `path`, when exactly one fits best.
/// Entries covering many games (Ren'Py, RPG Maker...) never count.
pub fn by_path(path: &Path) -> Option<&'static GameInfo> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let dirs: Vec<String> = path.parent()?.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
    let mut best: Option<(&'static GameInfo, usize)> = None;
    let mut tie = false;
    for g in GAMES {
        if crate::games::many_games(g) || !g.patterns.iter().any(|p| wildcard(p, &name)) {
            continue;
        }
        let score = g.places.iter().filter_map(|(b, s)| folders(*b, s)).filter_map(|want| match_at(&dirs, &want, g.depth)).max();
        let Some(score) = score else { continue };
        match best {
            Some((_, s)) if s > score => {}
            Some((b, s)) if s == score => tie |= b.name != g.name,
            _ => {
                best = Some((g, score));
                tie = false;
            }
        }
    }
    best.filter(|_| !tie).map(|(g, _)| g)
}

/// `by_path`'s game name for the file on screen, worked out once per file (looking through
/// the whole catalog every frame would be wasted work).
#[cfg(not(target_arch = "wasm32"))]
pub fn name_for(ctx: &eframe::egui::Context, path: &Path) -> Option<&'static str> {
    let id = eframe::egui::Id::new(("game_by_path", path));
    if let Some(hit) = ctx.data(|d| d.get_temp::<Option<&'static str>>(id)) {
        return hit;
    }
    let hit = by_path(path).map(|g| g.name);
    ctx.data_mut(|d| d.insert_temp(id, hit));
    hit
}

/// The browser version has no folders to go by.
#[cfg(target_arch = "wasm32")]
pub fn name_for(_ctx: &eframe::egui::Context, _path: &Path) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> std::path::PathBuf {
        s.split('/').collect()
    }

    #[test]
    fn finds_the_game_from_its_save_folder() {
        let g = by_path(&p("/home/u/.wine/drive_c/users/u/AppData/Local/BloodstainedRotN/Saved/SaveGames/Story_Slot0.sav"));
        assert_eq!(g.map(|g| g.name), Some("Bloodstained: Ritual of the Night"));
        // Any case.
        let g = by_path(&p("/c/Users/u/AppData/Local/bloodstainedrotn/saved/savegames/story_slot1.sav"));
        assert_eq!(g.map(|g| g.name), Some("Bloodstained: Ritual of the Night"));
    }

    #[test]
    fn wrong_name_or_folder_finds_nothing() {
        assert!(by_path(&p("/home/u/Downloads/Story_Slot0.sav")).is_none(), "the folder says nothing");
        assert!(by_path(&p("/c/Users/u/AppData/Local/BloodstainedRotN/Saved/SaveGames/notes.txt")).is_none(), "not a save file name");
        assert!(by_path(&p("/home/u/.steam/steam/userdata/1/999999999/remote/save.dat")).is_none());
        assert!(by_path(&p("save.sav")).is_none());
    }

    #[test]
    fn generic_folders_alone_never_match() {
        assert!(folders(Base::Documents, "*").is_none());
        assert!(folders(Base::LocalAppData, r"Saved\SaveGames").is_none());
        assert!(folders(Base::SteamUser("1718570"), "remote").is_some(), "the app id names the game");
        assert!(folders(Base::UnityPrefs, "Foo").is_none());
    }
}
