//! How much room backups take, across every save the editor has backed up, and ways to give
//! some of it back: removing automatic backups older than some age, and an optional limit on
//! the total size. Named backups are never removed here, and neither is the newest automatic
//! backup of any save, so every save keeps at least one way back.
//!
//! Backups sit next to each save by default, so there is no single folder to measure. The
//! editor remembers each backups folder it writes to (`backup-folders.txt` in its own folder)
//! and also looks through the folders it keeps backups in itself.

use super::{in_folder, is_future, now, Version};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// The remembered backups folders, at most this many (the oldest are forgotten first).
const REMEMBER_MAX: usize = 5_000;

/// Total size limit in bytes for all backups (0: no limit).
static CAP: AtomicU64 = AtomicU64::new(0);

/// Sets the total size limit for backups, from Preferences (0: no limit).
pub fn set_cap(bytes: u64) {
    CAP.store(bytes, Relaxed);
}

pub fn cap() -> u64 {
    CAP.load(Relaxed)
}

/// The size limits offered, in GB (0: no limit).
pub const CAP_CHOICES_GB: [u64; 8] = [0, 1, 2, 5, 10, 20, 50, 100];
pub const GB: u64 = 1 << 30;

/// The file listing remembered backups folders. In tests, only a file a test chose.
static LIST_FILE: std::sync::RwLock<Option<PathBuf>> = std::sync::RwLock::new(None);

fn list_file() -> Option<PathBuf> {
    let set = LIST_FILE.read().ok().and_then(|f| f.clone());
    if cfg!(test) {
        return set;
    }
    set.or_else(|| crate::platform::config_dir().map(|d| d.join("backup-folders.txt")))
}

#[cfg(test)]
pub(crate) fn set_list_file(p: Option<PathBuf>) {
    if let Ok(mut f) = LIST_FILE.write() {
        *f = p;
    }
}

fn remembered() -> Vec<PathBuf> {
    let Some(f) = list_file() else { return vec![] };
    std::fs::read_to_string(f).map(|t| t.lines().filter(|l| !l.trim().is_empty()).map(PathBuf::from).collect()).unwrap_or_default()
}

/// Notes a folder backups were just written to, so it can be measured later. Never fails a
/// backup: a list that can't be written only means the folder isn't counted.
pub fn remember(dir: &Path) {
    let Some(f) = list_file() else { return };
    let mut all = remembered();
    if all.iter().any(|d| d == dir) {
        return;
    }
    all.push(dir.to_path_buf());
    // Forget folders that are gone, then the oldest, to keep the list short.
    all.retain(|d| d.is_dir());
    let skip = all.len().saturating_sub(REMEMBER_MAX);
    let text: String = all.iter().skip(skip).map(|d| format!("{}\n", d.display())).collect();
    if let Some(p) = f.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = super::write_verified(&f, text.as_bytes(), true);
}

/// Whose backups a folder holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Backups of one file.
    File,
    /// Backups of a whole save folder.
    Folder,
}

/// The backups of one save (a file or a whole folder).
#[derive(Debug)]
pub struct Use {
    /// The folder the backups are in.
    pub dir: PathBuf,
    /// The save's file or folder name.
    pub name: String,
    /// The folder the save is in, when the backups sit next to it.
    pub beside: Option<PathBuf>,
    pub kind: Kind,
    pub versions: Vec<Version>,
    pub bytes: u64,
}

impl Use {
    pub fn named(&self) -> usize {
        self.versions.iter().filter(|v| v.name.is_some()).count()
    }
    pub fn automatic(&self) -> usize {
        self.versions.len() - self.named()
    }
}

/// Every backup the editor knows of, by save.
#[derive(Debug, Default)]
pub struct Survey {
    pub saves: Vec<Use>,
    pub bytes: u64,
    pub files: usize,
}

/// The folders under a backups root: one per save, plus the whole-folder ones.
fn under_root(base: &Path) -> Vec<PathBuf> {
    let sub = |d: &Path| -> Vec<PathBuf> { std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect() };
    let mut out = sub(base);
    out.retain(|p| p.file_name().is_none_or(|n| n != "folders"));
    out.extend(sub(&base.join("folders")));
    out
}

/// Every folder that may hold backups: remembered ones, those under the editor's own and the
/// chosen backups folders, and those of `saves` (files the player opened lately).
pub fn known_folders(saves: &[PathBuf]) -> Vec<PathBuf> {
    // (Tests never look through the editor's real folder.)
    let own = if cfg!(test) { None } else { crate::platform::config_dir().map(|d| d.join("backups")) };
    let custom = super::ROOT.read().ok().and_then(|r| r.clone());
    let mut roots: Vec<PathBuf> = [custom, own].into_iter().flatten().collect();
    roots.extend(super::EARLIER.read().map(|r| r.clone()).unwrap_or_default());
    let mut all: Vec<PathBuf> = remembered();
    for r in &roots {
        all.extend(under_root(r));
    }
    for s in saves {
        all.extend(super::dirs(s));
        if let Some(parent) = s.parent() {
            all.extend(super::folders::dirs(parent));
        }
    }
    let mut seen = std::collections::HashSet::new();
    all.into_iter().filter(|d| d.is_dir()).filter(|d| seen.insert(std::fs::canonicalize(d).unwrap_or_else(|_| d.clone()))).collect()
}

/// The save name a backups folder stands for: the folder's own name, less the path hash added
/// in the editor's own folder.
fn save_name(dir: &Path) -> String {
    let n = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    match n.rsplit_once('-') {
        Some((name, hash)) if hash.len() == 8 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && !is_beside(dir) => name.to_string(),
        _ => n,
    }
}

fn is_beside(dir: &Path) -> bool {
    dir.parent().and_then(|p| p.file_name()).is_some_and(|n| n == super::BESIDE)
}

/// Measures every folder in `dirs`. Folders with no backups in them are left out.
pub fn survey_of(dirs: &[PathBuf]) -> Survey {
    let mut s = Survey::default();
    for d in dirs {
        let versions = in_folder(d);
        if versions.is_empty() {
            continue;
        }
        let bytes = versions.iter().map(|v| v.size).sum();
        let kind = if versions.iter().all(|v| super::folders::is_folder_copy(&v.path)) { Kind::Folder } else { Kind::File };
        let beside = if is_beside(d) { d.parent().and_then(Path::parent).map(Path::to_path_buf) } else { None };
        s.bytes += bytes;
        s.files += versions.len();
        s.saves.push(Use { dir: d.clone(), name: save_name(d), beside, kind, versions, bytes });
    }
    s.saves.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    s
}

/// [`survey_of`] every known folder.
pub fn survey(saves: &[PathBuf]) -> Survey {
    survey_of(&known_folders(saves))
}

/// Automatic backups that may go to free room, oldest first: never a named one, never the
/// newest automatic one of a save, never one in `protect`. Future-dated ones (taken while the
/// clock was set wrong) come first, as in pruning.
fn removable<'a>(s: &'a Survey, protect: &[&Path]) -> Vec<&'a Version> {
    let now = now();
    let mut out: Vec<&Version> = vec![];
    for u in &s.saves {
        // `versions` is newest first, so the first automatic one is the one to keep.
        out.extend(u.versions.iter().filter(|v| v.name.is_none()).skip(1).filter(|v| !protect.contains(&v.path.as_path())));
    }
    out.sort_by_key(|v| (!is_future(v.when, now), v.when));
    out
}

/// What [`clean_older_than`] would remove: automatic backups taken more than `days` days ago.
pub fn older_than<'a>(s: &'a Survey, days: u64) -> Vec<&'a Version> {
    let cut = now().saturating_sub(days.saturating_mul(86_400));
    removable(s, &[]).into_iter().filter(|v| v.when < cut).collect()
}

/// What keeping the total under `cap` bytes would remove.
pub fn over_cap<'a>(s: &'a Survey, cap: u64, protect: &[&Path]) -> Vec<&'a Version> {
    if cap == 0 || s.bytes <= cap {
        return vec![];
    }
    let mut excess = s.bytes - cap;
    let mut out = vec![];
    for v in removable(s, protect) {
        if excess == 0 {
            break;
        }
        excess = excess.saturating_sub(v.size);
        out.push(v);
    }
    out
}

/// Removes `versions`. Returns how many went and the bytes freed; one that can't be removed is
/// skipped (it is still listed next time).
pub fn remove(versions: &[&Version]) -> (usize, u64) {
    versions.iter().filter(|v| std::fs::remove_file(&v.path).is_ok()).fold((0, 0), |(n, b), v| (n + 1, b + v.size))
}

/// After a backup was made: when a size limit is set and passed, removes the oldest automatic
/// backups until the total is under it again (never those in `protect`).
pub fn enforce_cap(protect: &[&Path]) -> (usize, u64) {
    let cap = cap();
    if cap == 0 {
        return (0, 0);
    }
    let s = survey(&[]);
    remove(&over_cap(&s, cap, protect))
}

#[cfg(test)]
mod tests {
    use super::super::tests::LOCK;
    use super::super::{list, set_root, take, take_named};
    use super::*;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("use-space-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            set_root(None);
            set_list_file(Some(d.join("remembered.txt")));
            Scratch(d)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            set_cap(0);
            set_list_file(None);
            set_root(None);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Writes an automatic backup of `save` by hand, taken `days` days ago.
    fn aged(save: &Path, days: u64, bytes: &[u8]) -> PathBuf {
        let dir = super::super::folder(save).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{}.sebak", now() - days * 86_400));
        std::fs::write(&p, bytes).unwrap();
        remember(&dir);
        p
    }

    fn mine(s: &Survey, root: &Path) -> Vec<String> {
        let mut v: Vec<String> = s.saves.iter().filter(|u| u.dir.starts_with(root)).map(|u| u.name.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn every_save_backed_up_is_counted_once() {
        let _g = guard();
        let s = Scratch::new("count");
        let (a, b) = (s.0.join("one").join("a.sav"), s.0.join("two").join("b.sav"));
        for p in [&a, &b] {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"12345").unwrap();
            take(p).unwrap();
            take_named(p, "kept").unwrap();
        }
        // Found through the remembered list, and once only when also given as an open save.
        let got = survey(&[a.clone()]);
        assert_eq!(mine(&got, &s.0), ["a.sav", "b.sav"]);
        let u = got.saves.iter().find(|u| u.name == "a.sav").unwrap();
        assert_eq!((u.automatic(), u.named(), u.bytes), (1, 1, 10));
        assert_eq!(u.beside.as_deref(), Some(s.0.join("one").as_path()));
        // A save backed up before folders were remembered is still found from the recent files.
        set_list_file(Some(s.0.join("empty.txt")));
        assert_eq!(mine(&survey(&[b.clone()]), &s.0), ["b.sav"]);
    }

    #[test]
    fn cleaning_old_backups_keeps_named_and_the_newest_of_each_save() {
        let _g = guard();
        let s = Scratch::new("old");
        let (a, b) = (s.0.join("a.sav"), s.0.join("b.sav"));
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let a_old = aged(&a, 100, b"a 100");
        let a_mid = aged(&a, 60, b"a 60");
        let a_new = aged(&a, 1, b"a 1");
        // b's only automatic backups are all old: the newest of them stays anyway.
        let b_old = aged(&b, 200, b"b 200");
        let b_newest = aged(&b, 90, b"b 90");
        take_named(&a, "named").unwrap();
        let named = list(&a).into_iter().find(|v| v.name.is_some()).unwrap().path;
        let sv = survey(&[]);
        let mut gone: Vec<PathBuf> = older_than(&sv, 30).iter().map(|v| v.path.clone()).collect();
        gone.sort();
        let mut want = vec![a_old.clone(), a_mid.clone(), b_old.clone()];
        want.sort();
        assert_eq!(gone, want);
        let (n, bytes) = remove(&older_than(&sv, 30));
        assert_eq!((n, bytes), (3, 5 + 4 + 5));
        for p in [&a_new, &b_newest, &named] {
            assert!(p.exists(), "{}", p.display());
        }
        assert_eq!(std::fs::read(&a).unwrap(), b"a", "saves themselves never touched");
    }

    #[test]
    fn a_size_limit_removes_the_oldest_automatic_backups_first() {
        let _g = guard();
        let s = Scratch::new("cap");
        let (a, b) = (s.0.join("a.sav"), s.0.join("b.sav"));
        std::fs::write(&a, [1u8; 100]).unwrap();
        std::fs::write(&b, [2u8; 100]).unwrap();
        let a1 = aged(&a, 50, &[0u8; 100]);
        let b1 = aged(&b, 40, &[0u8; 100]);
        let a2 = aged(&a, 30, &[0u8; 100]);
        let b2 = aged(&b, 20, &[0u8; 100]);
        take_named(&a, "named").unwrap();
        // 500 bytes kept; a limit of 350 takes the two oldest automatic ones.
        set_cap(350);
        std::fs::write(&b, [3u8; 100]).unwrap();
        let newest = take(&b).unwrap().unwrap();
        // 600 bytes with the new one, 250 over: the three oldest automatic ones that aren't the
        // newest of their save go (a1, b1, b2), oldest first.
        assert!(!a1.exists() && !b1.exists() && !b2.exists());
        assert!(a2.exists(), "a's newest automatic backup stays");
        assert!(newest.exists(), "the backup just made is never removed");
        assert!(list(&a).iter().any(|v| v.name.is_some()), "named backups are never removed");
    }

    #[test]
    fn the_newest_automatic_backup_survives_any_limit() {
        let _g = guard();
        let s = Scratch::new("cap-tiny");
        let saves: Vec<PathBuf> = (0..3).map(|i| s.0.join(format!("{i}.sav"))).collect();
        for (i, p) in saves.iter().enumerate() {
            std::fs::write(p, [i as u8; 50]).unwrap();
            aged(p, 10, &[9u8; 50]);
            aged(p, 5, &[8u8; 50]);
        }
        set_cap(1);
        std::fs::write(&saves[0], [7u8; 50]).unwrap();
        take(&saves[0]).unwrap();
        for p in &saves {
            assert_eq!(list(p).len(), 1, "{} keeps exactly its newest", p.display());
        }
        assert_eq!(std::fs::read(&list(&saves[0])[0].path).unwrap(), [7u8; 50]);
    }

    #[test]
    fn no_limit_removes_nothing() {
        let _g = guard();
        let s = Scratch::new("nocap");
        let a = s.0.join("a.sav");
        std::fs::write(&a, b"x").unwrap();
        let old = aged(&a, 900, b"very old");
        take(&a).unwrap();
        assert_eq!(enforce_cap(&[]), (0, 0));
        assert!(old.exists());
    }
}
