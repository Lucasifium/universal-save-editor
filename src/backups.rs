//! Timestamped copies of a save, taken every time the app writes over it, so any earlier
//! version can be brought back. By default they sit next to the save, in
//! `<save folder>/Save Editor Backups/<file name>/<unix seconds>.sebak`: one level down in a
//! folder of their own and with an extension no game looks for, so a game never loads or
//! overwrites them as saves. A player can instead keep them under a folder of their choice
//! (`<folder>/<file name>-<path hash>/<unix seconds>.<ext>`); older copies in the app's own
//! folder (`<config>/backups`) are still listed. A copy the player names
//! (`<unix seconds>~<name>`) is a slot and is kept for good: only unnamed copies are pruned.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Versions kept per save; the oldest go first.
pub const KEEP: usize = 30;

pub struct Version {
    pub path: PathBuf,
    /// The name the player gave this copy; named copies are never pruned.
    pub name: Option<String>,
    /// Seconds since 1970 (UTC) when the copy was taken.
    pub when: u64,
    pub size: u64,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// FNV-1a, so two saves with the same name in different folders get different folders.
fn hash(s: &str) -> u32 {
    s.bytes().fold(0x811C_9DC5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

static ROOT: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

/// Keeps backups under `dir` instead of the app's own folder (`None`: the default again).
pub fn set_root(dir: Option<PathBuf>) {
    if let Ok(mut r) = ROOT.write() {
        *r = dir;
    }
}

/// The folder all backups go under.
pub fn root() -> Option<PathBuf> {
    ROOT.read().ok().and_then(|r| r.clone()).or_else(|| crate::platform::config_dir().map(|d| d.join("backups")))
}

/// Name of the folder, next to a save, that holds its backups and slots.
pub const BESIDE: &str = "Save Editor Backups";

/// Where new versions of `save` go: next to it, unless the player chose another folder.
pub fn folder(save: &Path) -> Option<PathBuf> {
    let custom = ROOT.read().ok().and_then(|r| r.clone());
    if custom.is_none() {
        return beside(save);
    }
    elsewhere(save)
}

/// `<save folder>/Save Editor Backups/<file name>`.
fn beside(save: &Path) -> Option<PathBuf> {
    let parent = save.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    Some(parent.join(BESIDE).join(save.file_name()?))
}

/// The per-save folder under the app's own folder or the chosen one.
fn elsewhere(save: &Path) -> Option<PathBuf> {
    root().map(|d| under(&d, save))
}

/// `<base>/<file name>-<hash of the full path>`.
fn under(base: &Path, save: &Path) -> PathBuf {
    let full = std::fs::canonicalize(save).unwrap_or_else(|_| save.to_path_buf());
    let name: String = save
        .file_name()
        .map(|n| n.to_string_lossy().chars().map(|c| if c.is_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect())
        .unwrap_or_else(|| "save".into());
    base.join(format!("{name}-{:08x}", hash(&full.to_string_lossy().to_lowercase())))
}

/// The default app-folder location older releases used.
fn legacy(save: &Path) -> Option<PathBuf> {
    crate::platform::config_dir().map(|d| under(&d.join("backups"), save))
}

static EARLIER: std::sync::RwLock<Vec<PathBuf>> = std::sync::RwLock::new(Vec::new());

/// Folders the player chose for backups before, so the copies made there stay listed after
/// switching to another folder or back to the default.
pub fn set_earlier_roots(dirs: Vec<PathBuf>) {
    if let Ok(mut r) = EARLIER.write() {
        *r = dirs;
    }
}

/// Every kept version of `save`, newest first.
pub fn list(save: &Path) -> Vec<Version> {
    let mut out: Vec<_> = dirs(save)
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|rd| rd.flatten())
        .filter_map(|e| {
            let p = e.path();
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            let file = p.file_name()?.to_str()?;
            if file.ends_with(".partial") {
                return None; // a copy cut off half way, never a version
            }
            // "<secs>" or "<secs>-<n>" when two copies land in the same second, then "~<name>",
            // then the extension. A name may hold dots of its own, so only the last one goes.
            let (stamp, name) = match file.split_once('~') {
                Some((t, rest)) => (t, Some(rest.rsplit_once('.').map_or(rest, |(n, _)| n).to_string())),
                None => (file.split_once('.').map_or(file, |(t, _)| t), None),
            };
            let (secs, n) = stamp.split_once('-').unwrap_or((stamp, "0"));
            let when = secs.parse().ok()?;
            let order = (when, n.parse::<u32>().ok()?, meta.modified().ok());
            Some((order, Version { size: meta.len(), path: p, when, name }))
        })
        .collect();
    // Newest first: by second, then by the counter added within a second, then by the
    // time the file was written.
    out.sort_by(|(a, va), (b, vb)| b.cmp(a).then_with(|| vb.path.cmp(&va.path)));
    out.into_iter().map(|(_, v)| v).collect()
}

/// Every folder that may hold versions of `save`: where new ones go first.
fn dirs(save: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = vec![];
    let earlier: Vec<PathBuf> = EARLIER.read().map(|r| r.iter().map(|d| under(d, save)).collect()).unwrap_or_default();
    for d in [folder(save), elsewhere(save), beside(save), legacy(save)].into_iter().flatten().chain(earlier) {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    dirs
}

/// Writes `bytes` to `dest` so that `dest` either holds all of them or doesn't exist: a
/// temporary file is written, flushed to disk, read back and compared, then renamed.
/// Fails rather than replace an existing `dest` unless `replace` is set.
pub fn write_verified(dest: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    use std::io::Write;
    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    let result = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        if std::fs::read(&tmp)? != bytes {
            return Err(std::io::Error::other("the copy read back differently from what was written"));
        }
        if !replace && dest.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, format!("{} already exists", dest.display())));
        }
        std::fs::rename(&tmp, dest)?;
        // The rename itself is only durable once the folder is flushed (not possible on Windows).
        #[cfg(unix)]
        if let Some(dir) = dest.parent() {
            let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    Ok(())
}

/// The save's bytes, `None` when there is no such file. Any other failure (no permission,
/// locked by the game) is an error, so nothing goes on to overwrite a file it couldn't copy.
fn read_save(save: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(save) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("couldn't read {}: {e}", save.display())),
    }
}

/// Copies the file as it is on disk now. Returns the copy, or `None` when the newest
/// version already holds exactly these bytes (or the file doesn't exist yet).
pub fn take(save: &Path) -> Result<Option<PathBuf>, String> {
    copy(save, None, None)
}

/// Like [`take`], but never prunes `keep` (the backup about to be restored).
pub fn take_keeping(save: &Path, keep: &Path) -> Result<Option<PathBuf>, String> {
    copy(save, None, Some(keep))
}

/// Cleans a name the player typed into one that is safe in a file name on every system.
fn clean_name(name: &str) -> String {
    let clean: String = name.trim().chars().map(|c| if c.is_alphanumeric() || " ._-()".contains(c) { c } else { '_' }).take(60).collect();
    // Windows drops dots and spaces at the end of a file name.
    clean.trim().trim_end_matches(['.', ' ']).to_string()
}

/// Keeps a copy of the file as it is on disk now under `name`, for good.
pub fn take_named(save: &Path, name: &str) -> Result<PathBuf, String> {
    let clean = clean_name(name);
    if clean.is_empty() {
        return Err("give the copy a name".into());
    }
    copy(save, Some(&clean), None).and_then(|p| p.ok_or_else(|| format!("{} doesn't exist", save.display())))
}

/// Takes a name away from a copy, so it is pruned like the others in time.
pub fn unlock(v: &Version) -> Result<(), String> {
    let file = v.path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    let Some((stamp, rest)) = file.split_once('~') else { return Ok(()) };
    let ext = rest.rsplit_once('.').map(|(_, e)| format!(".{e}")).unwrap_or_default();
    let secs = stamp.split('-').next().unwrap_or(stamp);
    // Never rename over another copy taken in the same second.
    let mut dest = v.path.with_file_name(format!("{stamp}{ext}"));
    let mut n = match (dest.exists(), v.path.parent(), secs.parse()) {
        (true, Some(dir), Ok(t)) => next_in_second(dir, t).max(1),
        _ => 1,
    };
    while dest.exists() {
        dest = v.path.with_file_name(format!("{secs}-{n}{ext}"));
        n += 1;
    }
    std::fs::rename(&v.path, dest).map_err(|e| e.to_string())
}

fn copy(save: &Path, name: Option<&str>, keep: Option<&Path>) -> Result<Option<PathBuf>, String> {
    let Some(bytes) = read_save(save)? else { return Ok(None) };
    let versions = list(save);
    if name.is_none() {
        if let Some(v) = versions.first() {
            if v.size == bytes.len() as u64 && std::fs::read(&v.path).is_ok_and(|b| b == bytes) {
                return Ok(None);
            }
        }
    }
    let tag = name.map(|n| format!("~{n}")).unwrap_or_default();
    let t = now();
    let mut errors = vec![];
    let mut written = None;
    // Next to the save first; if that folder can't be written to (read-only, full, a device
    // that forbids it), the app's own folder.
    let candidates: Vec<PathBuf> = [folder(save), elsewhere(save), beside(save), legacy(save)].into_iter().flatten().collect();
    for dir in candidates.iter().enumerate().filter(|(i, d)| !candidates[..*i].contains(d)).map(|(_, d)| d) {
        if let Err(e) = std::fs::create_dir_all(dir) {
            errors.push(format!("{}: {e}", dir.display()));
            continue;
        }
        let beside = dir.parent().and_then(|p| p.file_name()).is_some_and(|n| n == BESIDE);
        // A save without an extension still gets one, so a name with dots in it reads back whole.
        let ext = match save.extension() {
            Some(e) if !beside => format!(".{}", e.to_string_lossy()),
            _ => ".sebak".to_string(),
        };
        // Copies taken in the same second count up, always past the highest one there, so
        // the order they were taken in survives pruning.
        let n = next_in_second(dir, t);
        let mut dest = dir.join(if n == 0 { format!("{t}{tag}{ext}") } else { format!("{t}-{n}{tag}{ext}") });
        let mut n = n.max(1);
        while dest.exists() {
            dest = dir.join(format!("{t}-{n}{tag}{ext}"));
            n += 1;
        }
        match write_verified(&dest, &bytes, false) {
            Ok(()) => {
                written = Some(dest);
                break;
            }
            Err(e) => {
                errors.push(format!("{}: {e}", dir.display()));
                // Don't leave an empty folder behind next to the save.
                let _ = std::fs::remove_dir(dir);
                if beside {
                    let _ = std::fs::remove_dir(dir.parent().unwrap_or(dir));
                }
            }
        }
    }
    let Some(dest) = written else {
        return Err(if errors.is_empty() { "no folder for backups on this device".into() } else { errors.join("; ") });
    };
    // Only once the new copy is safely on disk do the oldest unnamed ones go.
    for old in versions.iter().filter(|v| v.name.is_none() && Some(v.path.as_path()) != keep).skip(KEEP - 1) {
        let _ = std::fs::remove_file(&old.path);
    }
    Ok(Some(dest))
}

/// 0 when no copy in `dir` was taken in second `t`, otherwise one past the highest counter.
fn next_in_second(dir: &Path, t: u64) -> u32 {
    let secs = t.to_string();
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let f = e.file_name().to_string_lossy().to_string();
                    let stamp = f.split(['~', '.']).next()?.to_string();
                    let (s, n) = stamp.split_once('-').unwrap_or((&stamp, "0"));
                    (s == secs).then(|| n.parse::<u32>().ok().map(|n| n + 1)).flatten()
                })
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

/// Reads a backup and puts it in place of `save`. The save as it is now is backed up first
/// (unless it is missing), the backup is read before anything is pruned, and the new file is
/// written to a temporary name, checked and only then renamed over the save.
pub fn restore(save: &Path, from: &Path) -> Result<(), String> {
    let bytes = std::fs::read(from).map_err(|e| format!("couldn't read the backup: {e}"))?;
    take_keeping(save, from).map_err(|e| format!("couldn't back up the current file first, so nothing was changed: {e}"))?;
    write_verified(save, &bytes, true)
}

/// Deletes one kept copy (a slot or a backup); drops the folder if it was the last.
pub fn remove(v: &Version) -> Result<(), String> {
    std::fs::remove_file(&v.path).map_err(|e| e.to_string())?;
    if let Some(dir) = v.path.parent() {
        let _ = std::fs::remove_dir(dir); // only succeeds when empty
        if dir.parent().and_then(|p| p.file_name()).is_some_and(|n| n == BESIDE) {
            if let Some(b) = dir.parent() {
                let _ = std::fs::remove_dir(b);
            }
        }
    }
    Ok(())
}

/// Puts a copy aside under `name` and then removes the live save, so the game starts a new
/// game with no save. The copy is read back and compared before the file is removed.
pub fn set_aside(save: &Path, name: &str) -> Result<PathBuf, String> {
    let kept = take_named(save, name)?;
    let (a, b) = (std::fs::read(save).map_err(|e| e.to_string())?, std::fs::read(&kept).map_err(|e| e.to_string())?);
    if a != b {
        return Err("the backup doesn't match the save (did the game write to it just now?); the save was left in place".into());
    }
    std::fs::remove_file(save).map_err(|e| format!("the backup was made, but the save couldn't be removed: {e}"))?;
    Ok(kept)
}

/// "3 minutes ago", "yesterday 14:02 UTC"-style label for a version.
pub fn describe(when: u64) -> String {
    let ago = now().saturating_sub(when);
    let rel = match ago {
        0..=59 => "just now".to_string(),
        60..=3599 => plural(ago / 60, "minute"),
        3600..=86_399 => plural(ago / 3600, "hour"),
        _ => plural(ago / 86_400, "day"),
    };
    let (y, m, d) = civil(when / 86_400);
    let (hh, mm) = ((when % 86_400) / 3600, (when % 3600) / 60);
    format!("{rel} \u{2014} {y}-{m:02}-{d:02} {hh:02}:{mm:02} UTC")
}

fn plural(n: u64, unit: &str) -> String {
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

/// Days since 1970-01-01 to (year, month, day) — Howard Hinnant's algorithm.
fn civil(days: u64) -> (i64, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn named_copies_are_kept() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("use-backup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        set_root(Some(dir.join("b")));
        let save = dir.join("slot.sav");
        std::fs::write(&save, b"one").unwrap();
        take_named(&save, "before boss").unwrap();
        for i in 0..KEEP + 3 {
            std::fs::write(&save, format!("v{i}")).unwrap();
            take(&save).unwrap();
        }
        let v = list(&save);
        assert_eq!(v.iter().filter(|v| v.name.is_none()).count(), KEEP - 1 + 1);
        let named = v.iter().find(|v| v.name.as_deref() == Some("before boss")).expect("named copy kept");
        assert_eq!(std::fs::read(&named.path).unwrap(), b"one");
        unlock(named).unwrap();
        assert!(list(&save).iter().all(|v| v.name.is_none()));
        set_root(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sits_beside_the_save_and_is_not_a_save() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("use-beside-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        set_root(None);
        let save = dir.join("game.sav");
        std::fs::write(&save, b"first").unwrap();
        let kept = set_aside(&save, "run one").unwrap();
        assert!(!save.exists(), "live save cleared for a new game");
        assert!(kept.starts_with(dir.join(BESIDE).join("game.sav")));
        assert_eq!(kept.extension().unwrap(), "sebak");
        // Nothing but the backup folder was added next to the save.
        let names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from(BESIDE)]);
        std::fs::write(&save, b"second").unwrap();
        let v = list(&save);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].name.as_deref(), Some("run one"));
        // Restoring puts the first back, after taking the second.
        take(&save).unwrap();
        std::fs::copy(&v[0].path, &save).unwrap();
        assert_eq!(std::fs::read(&save).unwrap(), b"first");
        assert_eq!(list(&save).len(), 2);
        for v in list(&save) {
            remove(&v).unwrap();
        }
        assert!(!dir.join(BESIDE).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A fresh folder for one test, removed when it ends.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("use-bk-{tag}-{}", std::process::id()));
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

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_root(None);
        g
    }

    fn contents(save: &Path) -> Vec<Vec<u8>> {
        list(save).iter().map(|v| std::fs::read(&v.path).unwrap()).collect()
    }

    #[test]
    fn pruning_keeps_the_newest_thirty_and_drops_the_oldest() {
        let _g = guard();
        let s = Scratch::new("prune");
        let save = s.0.join("a.sav");
        for i in 0..KEEP + 5 {
            std::fs::write(&save, format!("v{i}")).unwrap();
            assert!(take(&save).unwrap().is_some());
        }
        let got = contents(&save);
        assert_eq!(got.len(), KEEP);
        // Newest first, and exactly the last KEEP versions.
        let want: Vec<Vec<u8>> = (5..KEEP + 5).rev().map(|i| format!("v{i}").into_bytes()).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn the_same_bytes_are_not_copied_twice() {
        let _g = guard();
        let s = Scratch::new("dedup");
        let save = s.0.join("a.sav");
        std::fs::write(&save, b"same").unwrap();
        assert!(take(&save).unwrap().is_some());
        assert!(take(&save).unwrap().is_none());
        assert_eq!(list(&save).len(), 1);
        // A named copy is always made, even of identical bytes.
        take_named(&save, "keep").unwrap();
        assert_eq!(list(&save).len(), 2);
    }

    #[test]
    fn a_missing_save_is_not_an_error_but_cant_be_named() {
        let _g = guard();
        let s = Scratch::new("missing");
        let save = s.0.join("nothing.sav");
        assert_eq!(take(&save).unwrap(), None);
        assert!(take_named(&save, "x").is_err());
        assert!(set_aside(&save, "x").is_err());
        assert!(!s.0.join(BESIDE).exists(), "no empty backup folder left behind");
    }

    #[test]
    fn a_save_that_cant_be_read_is_never_overwritten() {
        let _g = guard();
        let s = Scratch::new("unreadable");
        // A folder where the save should be: reading it fails like a file the game has locked.
        let save = s.0.join("locked.sav");
        std::fs::create_dir_all(&save).unwrap();
        let backup = s.0.join("other.bin");
        std::fs::write(&backup, b"old").unwrap();
        assert!(take(&save).is_err());
        assert!(restore(&save, &backup).is_err());
        assert!(save.is_dir(), "left alone");
    }

    #[test]
    fn names_are_cleaned_and_kept_whole() {
        let _g = guard();
        let s = Scratch::new("names");
        let save = s.0.join("game.sav");
        std::fs::write(&save, b"x").unwrap();
        for (typed, want) in [
            ("../../escape", ".._.._escape"),
            ("v1.2 before boss", "v1.2 before boss"),
            ("trailing dots...", "trailing dots"),
            ("a/b\\c:d*e?f\"g<h>i|j~k", "a_b_c_d_e_f_g_h_i_j_k"),
            ("ボス前 ✓ done", "ボス前 _ done"),
        ] {
            let p = take_named(&save, typed).unwrap();
            assert!(p.starts_with(s.0.join(BESIDE).join("game.sav")), "{typed} stayed inside the backup folder");
            assert!(list(&save).iter().any(|v| v.name.as_deref() == Some(want)), "{typed} lists as {want}");
        }
        let long = "x".repeat(500);
        take_named(&save, &long).unwrap();
        assert!(list(&save).iter().any(|v| v.name.as_deref() == Some(&*"x".repeat(60))));
        assert!(take_named(&save, "  ").is_err());
        assert!(take_named(&save, "...").is_err());
    }

    #[test]
    fn copies_in_the_same_second_never_replace_each_other() {
        let _g = guard();
        let s = Scratch::new("collide");
        let save = s.0.join("game.sav");
        std::fs::write(&save, b"one").unwrap();
        take(&save).unwrap();
        take_named(&save, "same").unwrap();
        std::fs::write(&save, b"two").unwrap();
        take_named(&save, "same").unwrap();
        take(&save).unwrap(); // identical to the newest: skipped
        std::fs::write(&save, b"three").unwrap();
        take(&save).unwrap();
        let mut got = contents(&save);
        got.sort();
        assert_eq!(got, vec![b"one".to_vec(), b"one".to_vec(), b"three".to_vec(), b"two".to_vec()]);
        // Making the named ones automatic, in the same second as the unnamed ones, loses nothing.
        for v in list(&save).iter().filter(|v| v.name.is_some()) {
            unlock(v).unwrap();
        }
        let mut after = contents(&save);
        after.sort();
        assert_eq!(after, got);
        assert!(list(&save).iter().all(|v| v.name.is_none()));
    }

    #[test]
    fn restoring_the_oldest_backup_when_full_keeps_it() {
        let _g = guard();
        let s = Scratch::new("restore-oldest");
        let save = s.0.join("a.sav");
        for i in 0..KEEP {
            std::fs::write(&save, format!("v{i}")).unwrap();
            take(&save).unwrap();
        }
        std::fs::write(&save, b"current").unwrap();
        let oldest = list(&save).pop().unwrap();
        assert_eq!(std::fs::read(&oldest.path).unwrap(), b"v0");
        restore(&save, &oldest.path).unwrap();
        assert_eq!(std::fs::read(&save).unwrap(), b"v0");
        let got = contents(&save);
        assert!(got.contains(&b"current".to_vec()), "the replaced version was backed up");
        assert!(oldest.path.exists(), "the restored backup is still there");
        assert_eq!(got.len(), KEEP + 1, "one over until the next save prunes it");
    }

    #[test]
    fn start_a_new_game_then_restore() {
        let _g = guard();
        let s = Scratch::new("fresh");
        let save = s.0.join("only.sav");
        std::fs::write(&save, b"first run").unwrap();
        take(&save).unwrap();
        let kept = set_aside(&save, "first run").unwrap();
        assert!(!save.exists());
        // Changing your mind before the game made a new save: the backup comes straight back.
        restore(&save, &kept).unwrap();
        assert_eq!(std::fs::read(&save).unwrap(), b"first run");
        // Set aside again, the game makes a new save, then the old run comes back.
        let kept = set_aside(&save, "first run again").unwrap();
        std::fs::write(&save, b"second run").unwrap();
        restore(&save, &kept).unwrap();
        assert_eq!(std::fs::read(&save).unwrap(), b"first run");
        assert!(contents(&save).contains(&b"second run".to_vec()), "the new game was backed up before it was replaced");
        assert!(kept.exists());
    }

    #[test]
    fn falls_back_when_the_save_folder_cant_take_backups() {
        let _g = guard();
        let s = Scratch::new("readonly");
        set_root(Some(s.0.join("fallback")));
        let save = s.0.join("game.sav");
        std::fs::write(&save, b"data").unwrap();
        // With a chosen folder that can't be created, they go next to the save.
        std::fs::write(s.0.join("fallback"), b"a file, not a folder").unwrap();
        let p = take(&save).unwrap().unwrap();
        assert!(p.starts_with(s.0.join(BESIDE)), "{}", p.display());
        assert_eq!(list(&save).len(), 1, "still listed while a chosen folder is set");
        set_root(None);
        // Next to the save is blocked too: they go to the app's own folder, and still list.
        std::fs::remove_dir_all(s.0.join(BESIDE)).unwrap();
        std::fs::write(s.0.join(BESIDE), b"blocked").unwrap();
        std::fs::write(&save, b"data two").unwrap();
        let p = take(&save).unwrap().unwrap();
        assert!(!p.starts_with(&s.0), "{}", p.display());
        let v = list(&save);
        assert_eq!(std::fs::read(&v[0].path).unwrap(), b"data two");
        for v in v {
            remove(&v).unwrap();
        }
        assert!(list(&save).is_empty());
    }

    #[test]
    fn half_written_copies_are_ignored_and_cleaned_up() {
        let _g = guard();
        let s = Scratch::new("partial");
        let save = s.0.join("a.sav");
        std::fs::write(&save, b"good").unwrap();
        let dir = folder(&save).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        // What a crash mid-copy leaves behind, plus files a player might drop in the folder.
        std::fs::write(dir.join("1700000000.sebak.partial"), b"go").unwrap();
        std::fs::write(dir.join("notes.txt"), b"mine").unwrap();
        std::fs::write(dir.join("1700000000-x.sebak"), b"?").unwrap();
        std::fs::create_dir_all(dir.join("1700000001")).unwrap();
        assert!(list(&save).is_empty());
        take(&save).unwrap();
        assert_eq!(contents(&save), vec![b"good".to_vec()]);
        // A write that fails leaves neither the target nor its temporary file.
        let target = s.0.join("blocked");
        std::fs::create_dir_all(target.join("inside")).unwrap();
        assert!(write_verified(&target, b"x", true).is_err());
        assert!(!s.0.join("blocked.partial").exists());
        // And a new copy never replaces one already there.
        let f = s.0.join("exists.bin");
        std::fs::write(&f, b"keep").unwrap();
        assert!(write_verified(&f, b"new", false).is_err());
        assert_eq!(std::fs::read(&f).unwrap(), b"keep");
        write_verified(&f, b"new", true).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"new");
    }

    #[test]
    fn saves_side_by_side_keep_their_own_backups() {
        let _g = guard();
        let s = Scratch::new("multi");
        let (a, b, c) = (s.0.join("slot1.sav"), s.0.join("slot2.sav"), s.0.join("slot1.sav.bak"));
        for (p, d) in [(&a, "A"), (&b, "B"), (&c, "C")] {
            std::fs::write(p, d).unwrap();
            take(p).unwrap();
        }
        assert_eq!(contents(&a), vec![b"A".to_vec()]);
        assert_eq!(contents(&b), vec![b"B".to_vec()]);
        assert_eq!(contents(&c), vec![b"C".to_vec()]);
        // A save with no extension, in a chosen folder, keeps a dotted name whole.
        set_root(Some(s.0.join("chosen")));
        let bare = s.0.join("SAVEDATA");
        std::fs::write(&bare, b"x").unwrap();
        take_named(&bare, "v1.2").unwrap();
        assert_eq!(list(&bare)[0].name.as_deref(), Some("v1.2"));
        // Switching back to the default still lists copies made in the chosen folder.
        set_root(None);
        set_earlier_roots(vec![s.0.join("chosen")]);
        assert_eq!(list(&bare).len(), 1);
        unlock(&list(&bare)[0]).unwrap();
        assert_eq!(list(&bare)[0].name, None);
        set_earlier_roots(vec![]);
    }

    #[test]
    fn unicode_and_long_paths() {
        let _g = guard();
        let s = Scratch::new("unicode");
        let mut dir = s.0.join("セーブ データ 😀");
        for i in 0..6 {
            dir = dir.join(format!("{i}-{}", "長いフォルダ名".repeat(4)));
        }
        std::fs::create_dir_all(&dir).unwrap();
        let save = dir.join("Ünïcödé slot ✓.sav");
        assert!(save.as_os_str().len() > 260);
        std::fs::write(&save, b"data").unwrap();
        let kept = set_aside(&save, "ボスの前").unwrap();
        assert!(!save.exists());
        restore(&save, &kept).unwrap();
        assert_eq!(std::fs::read(&save).unwrap(), b"data");
        let v = list(&save);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].name.as_deref(), Some("ボスの前"));
        remove(&v[0]).unwrap();
        assert!(!dir.join(BESIDE).exists());
    }

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_723), (2024, 1, 1));
        assert_eq!(civil(20_761), (2026, 11, 4));
    }
}
