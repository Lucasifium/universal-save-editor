//! Copying a save from the Xbox app (PC Game Pass) to the same game's Steam version.
//!
//! A Game Pass save file is byte for byte what the game wrote (see `xgp`), so moving it is a
//! copy of that file into the folder the Steam version uses, under a name the Steam version
//! looks for. Only that direction is offered: nothing is ever written into the Xbox app's
//! container. An existing Steam save of the same name is backed up before it is replaced, and
//! the copy is written to a temporary file, read back and only then renamed into place.

use crate::games::{self, GameInfo};
use crate::i18n::{tr, trf};
use std::path::{Path, PathBuf};

/// What the copy needs: the game and where its other version keeps saves.
pub struct Source {
    pub game: &'static GameInfo,
    pub blob: PathBuf,
    /// The name the game gave the file inside the Xbox app ("Slot1/save.sav").
    pub logical: String,
}

/// The Game Pass save at `path`, if it belongs to a game whose Steam saves the editor knows.
pub fn source(path: &Path) -> Option<Source> {
    let logical = crate::xgp::logical_name(path)?;
    let game = games::game_pass_game(path)?;
    game.places.iter().any(|(b, _)| !matches!(b, games::Base::GamePass(_))).then(|| Source { game, blob: path.to_path_buf(), logical })
}

/// Is `name` a plain file name that the game's other versions look for?
pub fn check_name(g: &GameInfo, name: &str) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() || n != name || n.contains(['/', '\\', ':']) || n == "." || n == ".." {
        return Err(tr("type a plain file name, without a folder").into());
    }
    if !g.patterns.iter().any(|p| games::wildcard(p, n)) {
        return Err(trf("{0} isn't a name {1} uses for its saves ({2})", &[&n, &g.name, &g.patterns.join(", ")]));
    }
    Ok(())
}

/// The name to suggest: whichever part of the Xbox app's name for the file fits the game's
/// file names (the file's own name, then its container's).
pub fn suggested_name(src: &Source) -> String {
    let (container, file) = src.logical.rsplit_once('/').unwrap_or((src.logical.as_str(), src.logical.as_str()));
    [file, container].into_iter().find(|n| check_name(src.game, n).is_ok()).unwrap_or(file).to_string()
}

/// Is this inside an Xbox app container (which this copy never writes to)?
fn in_xbox_container(p: &Path) -> bool {
    p.components().any(|c| c.as_os_str().eq_ignore_ascii_case("SystemAppData") || c.as_os_str().eq_ignore_ascii_case("wgs"))
}

/// Copies the blob to `dir/name`. When a file is there already it is replaced only with
/// `replace`, after a backup. Returns the new file and the backup taken, if any.
pub fn copy(src: &Source, dir: &Path, name: &str, replace: bool) -> Result<(PathBuf, Option<PathBuf>), String> {
    check_name(src.game, name)?;
    if !dir.is_dir() {
        return Err(trf("{0} isn't a folder on this computer", &[&dir.display()]));
    }
    let dest = dir.join(name);
    // Where the write really lands: a folder or file that is a link into the Xbox app's
    // folders counts as inside them.
    let real_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let real_dest = std::fs::canonicalize(&dest).ok();
    if in_xbox_container(dir) || in_xbox_container(&real_dir) || real_dest.as_deref().is_some_and(in_xbox_container) {
        return Err(tr("that folder belongs to the Xbox app; pick the Steam version's folder").into());
    }
    let bytes = std::fs::read(&src.blob).map_err(|e| trf("couldn't read the Xbox app's save: {0}", &[&e]))?;
    let mut backup = None;
    if dest.exists() {
        if !replace {
            return Err(trf("{0} already exists", &[&dest.display()]));
        }
        backup = crate::backups::take(&dest).map_err(|e| trf("couldn't back up {0} first: {1}", &[&dest.display(), &e]))?;
    }
    crate::backups::write_verified(&dest, &bytes, replace)?;
    Ok((dest, backup))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hades() -> &'static GameInfo {
        games::GAMES.iter().find(|g| g.name == "Hades").unwrap()
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("use_xs_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn names_must_be_ones_the_game_uses() {
        let g = hades();
        assert!(check_name(g, "Profile1.sav").is_ok());
        assert!(check_name(g, "notes.txt").is_err());
        assert!(check_name(g, "..\\Profile1.sav").is_err());
        assert!(check_name(g, "sub/Profile1.sav").is_err());
        assert!(check_name(g, " Profile1.sav").is_err());
        assert!(check_name(g, "").is_err());
    }

    #[test]
    fn finds_the_game_and_copies_without_touching_the_container() {
        let tmp = temp("copy");
        let user = tmp.join("Packages/SupergiantGamesLLC.Hades_q53c1yqmx7pha/SystemAppData/wgs/000901F0A1B2C3D4_0000000000000000000000006A7B8C9D");
        crate::xgp::tests::make(&user, "Profile1", &[("Profile1.sav", b"SGB1 save data")]);
        let blob = crate::xgp::read_index(&user).unwrap().containers[0].files[0].path.clone();
        let before = std::fs::read(&blob).unwrap();
        let src = source(&blob).expect("a Hades Game Pass save");
        assert_eq!(src.game.name, "Hades");
        assert_eq!(suggested_name(&src), "Profile1.sav");

        let steam = tmp.join("Saved Games/Hades");
        std::fs::create_dir_all(&steam).unwrap();
        let (dest, backup) = copy(&src, &steam, "Profile1.sav", false).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), before);
        assert!(backup.is_none());
        // A second copy over it needs `replace`, and backs the old file up first.
        std::fs::write(&dest, b"the Steam save that was there").unwrap();
        assert!(copy(&src, &steam, "Profile1.sav", false).is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), b"the Steam save that was there", "untouched when refused");
        let (_, backup) = copy(&src, &steam, "Profile1.sav", true).unwrap();
        let backup = backup.expect("a backup of the replaced save");
        assert_eq!(std::fs::read(&backup).unwrap(), b"the Steam save that was there");
        assert_eq!(std::fs::read(&dest).unwrap(), before);
        // Never into the Xbox app's own folders, never under a name the game doesn't use.
        assert!(copy(&src, &user, "Profile1.sav", true).is_err());
        assert!(copy(&src, &steam, "Profile1.txt", true).is_err());
        assert_eq!(std::fs::read(&blob).unwrap(), before, "the Xbox app's file is never changed");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// A Steam save that is a link to a file in the Xbox app's folders, or a Steam folder that
    /// is a link to one of them: the copy would land in the container, so it is refused.
    #[cfg(unix)]
    #[test]
    fn a_link_into_the_container_is_never_written_through() {
        let tmp = temp("link");
        let user = tmp.join("Packages/SupergiantGamesLLC.Hades_q53c1yqmx7pha/SystemAppData/wgs/000901F0A1B2C3D4_0000000000000000000000006A7B8C9D");
        crate::xgp::tests::make(&user, "Profile1", &[("Profile1.sav", b"SGB1 save one")]);
        let blob = crate::xgp::read_index(&user).unwrap().containers[0].files[0].path.clone();
        // Another file in the container, which the Steam save links to.
        let other = blob.with_file_name("0123456789ABCDEF0123456789ABCDEF");
        std::fs::write(&other, b"another container file").unwrap();
        let src = source(&blob).unwrap();
        let steam = tmp.join("Saved Games/Hades");
        std::fs::create_dir_all(&steam).unwrap();
        std::os::unix::fs::symlink(&other, steam.join("Profile1.sav")).unwrap();
        assert!(copy(&src, &steam, "Profile1.sav", true).is_err());
        assert_eq!(std::fs::read(&other).unwrap(), b"another container file");
        // The Steam folder itself leads into the container.
        let linked = tmp.join("Saved Games/Linked");
        std::os::unix::fs::symlink(blob.parent().unwrap(), &linked).unwrap();
        assert!(copy(&src, &linked, "Profile2.sav", false).is_err());
        assert!(!blob.parent().unwrap().join("Profile2.sav").exists(), "nothing added to the container");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn other_packages_are_not_offered() {
        let tmp = temp("other");
        let user = tmp.join("Packages/Some.Other_abc/SystemAppData/wgs/000901F0A1B2C3D4_0000000000000000000000006A7B8C9D");
        crate::xgp::tests::make(&user, "Slot1", &[("save.sav", b"x")]);
        let blob = crate::xgp::read_index(&user).unwrap().containers[0].files[0].path.clone();
        assert!(source(&blob).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
