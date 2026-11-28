//! Backups of a whole save folder, for games that keep one save as several files or a folder
//! (a save folder with its screenshot and summary, a farm folder, slots side by side). The
//! folder is copied into one zip file, kept in the editor's own backups folder (or the one the
//! player chose) and never next to the save: games list the folders in their save folder, and
//! one more could show up there as a broken save.
//!
//! Copies are named like single-file backups (`<unix seconds>[-n][~name].sefolder`), so they
//! sort, prune and unlock the same way. Restoring puts the folder back exactly as it was in the
//! copy: files are written back, and files added since are removed. The folder as it is just
//! before is copied first, and a restore that fails half way is undone from that copy.

use super::{now, under, Version};
use crate::i18n::{tr, trf};
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::path::{Path, PathBuf};

/// The extension of a whole-folder copy.
pub const EXT: &str = "sefolder";
/// The most files a folder backup takes.
pub const MAX_FILES: usize = 5_000;
/// The most bytes a folder backup takes, all files together.
pub const MAX_BYTES: u64 = 1 << 30;
/// How deep below the save folder files are still taken.
const MAX_DEPTH: usize = 12;

/// One file of a folder: its path inside the folder (with `/`) and where it is.
#[derive(Clone, Debug)]
pub struct Entry {
    pub rel: String,
    pub path: PathBuf,
}

/// What a folder holds: the files a backup of it would take.
#[derive(Clone, Debug, Default)]
pub struct Contents {
    pub files: Vec<Entry>,
    pub bytes: u64,
}

/// The sub-folder of a backups root that holds folder backups.
const FOLDERS: &str = "folders";

/// Where new copies of `dir` go: `<backups root>/folders/<folder name>-<hash>`.
pub fn home(dir: &Path) -> Option<PathBuf> {
    super::root().map(|r| under(&r.join(FOLDERS), dir))
}

/// Every folder that may hold copies of `dir`, where new ones go first.
pub fn dirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    let legacy = crate::platform::config_dir().map(|d| d.join("backups"));
    let earlier: Vec<PathBuf> = super::EARLIER.read().map(|r| r.clone()).unwrap_or_default();
    for base in [super::root(), legacy].into_iter().flatten().chain(earlier) {
        let d = under(&base.join(FOLDERS), dir);
        if !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

/// Every kept copy of `dir`, newest first.
pub fn list(dir: &Path) -> Vec<Version> {
    let now = now();
    let mut out: Vec<_> = dirs(dir).iter().flat_map(|d| super::scan(d, now)).filter(|(_, v)| is_folder_copy(&v.path)).collect();
    super::sort(&mut out);
    out.into_iter().map(|(_, v)| v).collect()
}

/// A folder backup (and not a single-file one).
pub fn is_folder_copy(p: &Path) -> bool {
    p.extension().is_some_and(|e| e == EXT)
}

/// Why `dir` can't be backed up as a whole, or `None` when it can.
fn refused(dir: &Path) -> Option<String> {
    if !dir.is_dir() {
        return Some(trf("{0} isn't a folder", &[&dir.display()]));
    }
    let full = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if full.parent().is_none() {
        return Some(tr("a whole drive can't be backed up as one save folder").into());
    }
    let home = ["HOME", "USERPROFILE"].iter().filter_map(|k| std::env::var_os(k)).map(PathBuf::from).filter_map(|h| std::fs::canonicalize(h).ok());
    if home.into_iter().any(|h| h == full || h.starts_with(&full)) {
        return Some(tr("this is your home folder, not a save folder").into());
    }
    let inside = |p: Option<PathBuf>| p.is_some_and(|p| p.starts_with(dir) || std::fs::canonicalize(&p).is_ok_and(|p| p.starts_with(&full)));
    if inside(super::root()) || inside(crate::platform::config_dir()) {
        return Some(tr("this folder holds the editor's own backups").into());
    }
    None
}

/// The files a backup of `dir` takes: everything inside, except the editor's own backups
/// folders, half-written files and links. Fails past [`MAX_FILES`] files or [`MAX_BYTES`].
pub fn contents(dir: &Path) -> Result<Contents, String> {
    if let Some(why) = refused(dir) {
        return Err(why);
    }
    let mut out = Contents::default();
    walk(dir, "", 0, &mut out)?;
    out.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

fn walk(dir: &Path, prefix: &str, depth: usize, out: &mut Contents) -> Result<(), String> {
    let rd = std::fs::read_dir(dir).map_err(|e| trf("couldn't read {0}: {1}", &[&dir.display(), &e]))?;
    for e in rd {
        let e = e.map_err(|e| trf("couldn't read {0}: {1}", &[&dir.display(), &e]))?;
        let name = e.file_name().to_string_lossy().to_string();
        if name == super::BESIDE || name.ends_with(".partial") {
            continue;
        }
        let meta = std::fs::symlink_metadata(e.path()).map_err(|err| trf("couldn't read {0}: {1}", &[&e.path().display(), &err]))?;
        let rel = format!("{prefix}{name}");
        if meta.file_type().is_symlink() {
            continue;
        } else if meta.is_dir() {
            if depth + 1 > MAX_DEPTH {
                return Err(tr("the folder is nested too deep to be a save folder").into());
            }
            walk(&e.path(), &format!("{rel}/"), depth + 1, out)?;
        } else if meta.is_file() {
            out.bytes += meta.len();
            out.files.push(Entry { rel, path: e.path() });
            if out.files.len() > MAX_FILES {
                return Err(trf("the folder holds more than {0} files, too many for a save folder", &[&MAX_FILES]));
            }
            if out.bytes > MAX_BYTES {
                return Err(trf("the folder holds more than {0}, too much for a save folder", &[&crate::app::human_size(MAX_BYTES as usize)]));
            }
        }
    }
    Ok(())
}

/// What a zip entry is filled from.
pub(super) enum Source<'a> {
    File(&'a Path),
    Bytes(&'a [u8]),
}

impl Source<'_> {
    fn open(&self) -> std::io::Result<Box<dyn Read + '_>> {
        Ok(match self {
            Source::File(p) => Box::new(std::fs::File::open(p)?),
            Source::Bytes(b) => Box::new(*b),
        })
    }
}

/// Are the two readers' bytes the same, all the way to the end?
fn same_stream(mut a: impl Read, mut b: impl Read) -> std::io::Result<bool> {
    let (mut x, mut y) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    loop {
        let n = read_full(&mut a, &mut x)?;
        let m = read_full(&mut b, &mut y)?;
        if n != m || x[..n] != y[..m] {
            return Ok(false);
        }
        if n == 0 {
            return Ok(true);
        }
    }
}

fn read_full(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..])? {
            0 => break,
            k => n += k,
        }
    }
    Ok(n)
}

/// Writes a zip of `entries` to `dest` so that `dest` either holds all of them or doesn't
/// exist: a temporary file is written and flushed, every entry is read back and compared with
/// where it came from, and only then is it renamed. Fails rather than replace an existing
/// `dest` unless `replace` is set. Big files are streamed, never held in memory.
pub(super) fn write_zip(dest: &Path, entries: &[(String, Source)], replace: bool) -> Result<(), String> {
    let mut tmp = dest.as_os_str().to_owned();
    tmp.push(".partial");
    let tmp = PathBuf::from(tmp);
    let result = (|| -> Result<(), String> {
        let io = |e: std::io::Error| e.to_string();
        let file = std::fs::File::create(&tmp).map_err(io)?;
        let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
        for (name, src) in entries {
            let size = match src {
                Source::File(p) => std::fs::metadata(p).map(|m| m.len()).map_err(io)?,
                Source::Bytes(b) => b.len() as u64,
            };
            // Already-packed data (saves, other backups) gains nothing from deflate.
            let method = if size > 64 << 20 { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };
            let opts = zip::write::SimpleFileOptions::default().compression_method(method).large_file(size >= u32::MAX as u64);
            zip.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
            std::io::copy(&mut src.open().map_err(io)?, &mut zip).map_err(io)?;
        }
        let w = zip.finish().map_err(|e| e.to_string())?;
        let file = w.into_inner().map_err(|e| e.to_string())?;
        file.sync_all().map_err(io)?;
        drop(file);
        // Read every entry back (which also checks its CRC) and compare it with its source.
        let mut back = zip::ZipArchive::new(std::fs::File::open(&tmp).map_err(io)?).map_err(|e| e.to_string())?;
        if back.len() != entries.len() {
            return Err(tr("the written file read back differently from what was written").into());
        }
        for (i, (name, src)) in entries.iter().enumerate() {
            let entry = back.by_index(i).map_err(|e| e.to_string())?;
            if entry.name() != name || !same_stream(entry, src.open().map_err(io)?).map_err(io)? {
                return Err(trf("{0} changed while it was being copied (the game may be saving). Try again in a moment.", &[name]));
            }
        }
        if !replace && dest.exists() {
            return Err(trf("{0} already exists", &[&dest.display()]));
        }
        std::fs::rename(&tmp, dest).map_err(io)?;
        #[cfg(unix)]
        if let Some(dir) = dest.parent() {
            let _ = std::fs::File::open(dir).and_then(|d| d.sync_all());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Copies `dir` as it is on disk now, under `name` (kept for good) or as an automatic copy
/// (`None`, pruned like the automatic backups of a file).
fn copy(dir: &Path, name: Option<&str>, keep: Option<&Path>) -> Result<PathBuf, String> {
    let c = contents(dir)?;
    let Some(home) = home(dir) else { return Err(tr("no folder on this device can hold backups").into()) };
    std::fs::create_dir_all(&home).map_err(|e| format!("{}: {e}", home.display()))?;
    let tag = name.map(|n| format!("~{n}")).unwrap_or_default();
    let t = now();
    let n = super::next_in_second(&home, t);
    let mut dest = home.join(if n == 0 { format!("{t}{tag}.{EXT}") } else { format!("{t}-{n}{tag}.{EXT}") });
    let mut n = n.max(1);
    while dest.exists() {
        dest = home.join(format!("{t}-{n}{tag}.{EXT}"));
        n += 1;
    }
    let entries: Vec<(String, Source)> = c.files.iter().map(|f| (f.rel.clone(), Source::File(&f.path))).collect();
    if let Err(e) = write_zip(&dest, &entries, false) {
        let _ = std::fs::remove_dir(&home);
        return Err(e);
    }
    if name.is_none() {
        for old in list(dir).iter().filter(|v| v.name.is_none() && v.path != dest && Some(v.path.as_path()) != keep).skip(super::keep() - 1) {
            let _ = std::fs::remove_file(&old.path);
        }
    }
    super::space::remember(&home);
    let protect: Vec<&Path> = [Some(dest.as_path()), keep].into_iter().flatten().collect();
    super::space::enforce_cap(&protect);
    Ok(dest)
}

/// Keeps a copy of the whole folder as it is now under `name`, for good.
pub fn take_named(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let clean = super::clean_name(name);
    if clean.is_empty() {
        return Err(tr("type a name for the backup").into());
    }
    copy(dir, Some(&clean), None)
}

/// Checks that a folder backup can be put back and returns its files, without writing anything.
/// A name that would land outside the folder, or in the editor's own backups folder, refuses
/// the whole backup.
pub fn read_names(archive: &Path) -> Result<Vec<(String, u64)>, String> {
    let f = std::fs::File::open(archive).map_err(|e| trf("couldn't read the backup: {0}", &[&e]))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| trf("the backup is damaged: {0}", &[&e]))?;
    let mut out = vec![];
    let mut total = 0u64;
    for i in 0..zip.len() {
        let e = zip.by_index_raw(i).map_err(|e| trf("the backup is damaged: {0}", &[&e]))?;
        if e.is_dir() {
            continue;
        }
        let name = e.name().to_string();
        if !safe_rel(&name) {
            return Err(trf("the backup holds a file that would land outside the folder ({0}), so it wasn't used", &[&name]));
        }
        total = total.saturating_add(e.size());
        out.push((name, e.size()));
    }
    if out.len() > MAX_FILES || total > MAX_BYTES {
        return Err(tr("the backup is larger than any save folder backup this app makes, so it wasn't used").into());
    }
    Ok(out)
}

/// A path inside the folder: relative, `/`-separated, no `..`, no drive or root, and nothing
/// the editor keeps for itself.
fn safe_rel(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains(':')
        && !name.contains('\0')
        && name.split('/').all(|p| !p.is_empty() && p != "." && p != ".." && p != super::BESIDE && !p.ends_with(".partial") && !p.ends_with(['.', ' ']))
}

/// Joins a checked relative name onto `dir`.
fn join(dir: &Path, rel: &str) -> PathBuf {
    rel.split('/').fold(dir.to_path_buf(), |p, part| p.join(part))
}

/// How a restore went.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Restored {
    /// Files written back.
    pub written: usize,
    /// Files removed because the backup didn't have them.
    pub removed: usize,
}

/// Why a restore didn't go through, and what the folder holds now.
#[derive(Debug)]
pub struct RestoreError {
    pub error: String,
    /// `true`: the folder is exactly as it was before the restore started.
    pub unchanged: bool,
    /// The copy of the folder taken just before, if one was made.
    pub before: Option<PathBuf>,
}

/// Puts the folder back as it was in `archive`. The folder as it is now is copied first (unless
/// it is empty or missing); if anything then fails, the folder is put back from that copy.
pub fn restore(dir: &Path, archive: &Path) -> Result<Restored, RestoreError> {
    let early = |error: String| RestoreError { error, unchanged: true, before: None };
    read_names(archive).map_err(early)?;
    let has_files = dir.is_dir() && contents(dir).map_err(|e| early(trf("couldn't back up the folder first: {0}", &[&e])))?.files.len() > 0;
    let before = if has_files { Some(copy(dir, None, Some(archive)).map_err(|e| early(trf("couldn't back up the folder first: {0}", &[&e])))?) } else { None };
    match apply(dir, archive) {
        Ok(r) => Ok(r),
        Err(error) => {
            // Undo: from the copy just taken, or (for a folder that was empty) by removing
            // what was written.
            let undone = match &before {
                Some(b) => apply(dir, b).is_ok(),
                None => contents(dir).map(|c| c.files.iter().all(|f| std::fs::remove_file(&f.path).is_ok())).unwrap_or(false),
            };
            Err(RestoreError { error, unchanged: undone, before })
        }
    }
}

/// Makes `dir` hold exactly the files in `archive`: each one is written with a verified write
/// (skipped when already the same), then files the archive doesn't have are removed.
fn apply(dir: &Path, archive: &Path) -> Result<Restored, String> {
    let names = read_names(archive)?;
    let f = std::fs::File::open(archive).map_err(|e| trf("couldn't read the backup: {0}", &[&e]))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| trf("the backup is damaged: {0}", &[&e]))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut done = Restored::default();
    for (name, size) in &names {
        let mut e = zip.by_name(name).map_err(|e| trf("the backup is damaged: {0}", &[&e]))?;
        let mut bytes = Vec::with_capacity((*size).min(MAX_BYTES) as usize);
        e.by_ref().take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|e| trf("the backup is damaged: {0}", &[&e]))?;
        if bytes.len() as u64 != *size {
            return Err(trf("the backup is damaged: {0}", &[name]));
        }
        let dest = join(dir, name);
        if std::fs::read(&dest).is_ok_and(|b| b == bytes) {
            continue;
        }
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
        }
        super::write_verified(&dest, &bytes, true).map_err(|e| format!("{name}: {e}"))?;
        done.written += 1;
    }
    let keep: std::collections::HashSet<&str> = names.iter().map(|(n, _)| n.as_str()).collect();
    let now = contents(dir)?;
    for f in now.files.iter().filter(|f| !keep.contains(f.rel.as_str())) {
        std::fs::remove_file(&f.path).map_err(|e| format!("{}: {e}", f.rel))?;
        done.removed += 1;
        // Drop folders left empty, up to the save folder itself.
        let mut p = f.path.parent();
        while let Some(d) = p.filter(|d| *d != dir && d.starts_with(dir)) {
            if std::fs::remove_dir(d).is_err() {
                break;
            }
            p = d.parent();
        }
    }
    Ok(done)
}

/// Reads every file of a folder backup to the end, which checks each one against the checksum
/// stored with it. `Ok(n)`: the number of files, all readable.
pub fn check(archive: &Path) -> Result<usize, String> {
    let names = read_names(archive)?;
    let f = std::fs::File::open(archive).map_err(|e| trf("couldn't read the backup: {0}", &[&e]))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| trf("the backup is damaged: {0}", &[&e]))?;
    for (name, _) in &names {
        let mut e = zip.by_name(name).map_err(|e| trf("the backup is damaged: {0}", &[&e]))?;
        std::io::copy(&mut e, &mut std::io::sink()).map_err(|e| trf("the backup is damaged: {0}", &[&format!("{name}: {e}")]))?;
    }
    Ok(names.len())
}

#[cfg(test)]
mod tests {
    use super::super::tests::LOCK;
    use super::super::{set_root, unlock};
    use super::*;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("use-fbk-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            set_root(Some(d.join("backups")));
            Scratch(d)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            set_root(None);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A save folder of a few files, one in a sub-folder.
    fn save_folder(s: &Scratch) -> PathBuf {
        let dir = s.0.join("Saves").join("Farm_123");
        std::fs::create_dir_all(dir.join("screens")).unwrap();
        std::fs::write(dir.join("Farm_123"), b"farm one").unwrap();
        std::fs::write(dir.join("SaveGameInfo"), b"info one").unwrap();
        std::fs::write(dir.join("screens").join("shot.png"), b"png one").unwrap();
        dir
    }

    fn read_tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
        contents(dir).unwrap().files.iter().map(|f| (f.rel.clone(), std::fs::read(&f.path).unwrap())).collect()
    }

    #[test]
    fn a_folder_backup_restores_the_folder_exactly() {
        let _g = guard();
        let s = Scratch::new("round");
        let dir = save_folder(&s);
        let was = read_tree(&dir);
        let kept = take_named(&dir, "before boss").unwrap();
        assert!(kept.starts_with(s.0.join("backups")), "kept in the backups folder, not next to the save");
        assert!(!dir.parent().unwrap().join(super::super::BESIDE).exists());
        // The game saves again: one file changes, one is added, one is removed.
        std::fs::write(dir.join("Farm_123"), b"farm two").unwrap();
        std::fs::write(dir.join("new.tmp"), b"added").unwrap();
        std::fs::remove_file(dir.join("screens").join("shot.png")).unwrap();
        let after = read_tree(&dir);
        let r = restore(&dir, &kept).unwrap();
        assert_eq!(r, Restored { written: 2, removed: 1 });
        assert_eq!(read_tree(&dir), was);
        // The folder as it was before the restore was copied, so the restore can be undone.
        let v = list(&dir);
        assert_eq!(v.len(), 2);
        let auto = v.iter().find(|v| v.name.is_none()).unwrap();
        restore(&dir, &auto.path).unwrap();
        assert_eq!(read_tree(&dir), after);
        assert!(v.iter().any(|v| v.name.as_deref() == Some("before boss")));
    }

    #[test]
    fn the_editors_own_folders_and_leftovers_are_left_out_and_left_alone() {
        let _g = guard();
        let s = Scratch::new("skip");
        let dir = save_folder(&s);
        std::fs::create_dir_all(dir.join(super::super::BESIDE).join("x")).unwrap();
        std::fs::write(dir.join(super::super::BESIDE).join("x").join("1.sebak"), b"bak").unwrap();
        std::fs::write(dir.join("Farm_123.partial"), b"half").unwrap();
        let c = contents(&dir).unwrap();
        assert_eq!(c.files.iter().map(|f| f.rel.as_str()).collect::<Vec<_>>(), ["Farm_123", "SaveGameInfo", "screens/shot.png"]);
        let kept = take_named(&dir, "x").unwrap();
        std::fs::write(dir.join("Farm_123"), b"changed").unwrap();
        restore(&dir, &kept).unwrap();
        assert_eq!(std::fs::read(dir.join(super::super::BESIDE).join("x").join("1.sebak")).unwrap(), b"bak", "single-file backups untouched");
        assert_eq!(std::fs::read(dir.join("Farm_123")).unwrap(), b"farm one");
    }

    #[test]
    fn a_backup_with_paths_outside_the_folder_is_refused_and_writes_nothing() {
        let _g = guard();
        let s = Scratch::new("evil");
        let dir = save_folder(&s);
        let was = read_tree(&dir);
        for bad in ["../escape.txt", "/abs.txt", "a\\..\\b", "C:/x", "sub/../../x", "Save Editor Backups/x", "ok/.", "x.partial"] {
            let archive = s.0.join("evil.sefolder");
            let _ = std::fs::remove_file(&archive);
            let mut z = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            z.start_file("Farm_123", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"evil").unwrap();
            z.start_file(bad, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"evil").unwrap();
            z.finish().unwrap();
            let e = restore(&dir, &archive).unwrap_err();
            assert!(e.unchanged && e.before.is_none(), "{bad}");
            assert_eq!(read_tree(&dir), was, "{bad}");
        }
        assert!(!s.0.join("escape.txt").exists());
        assert!(list(&dir).is_empty(), "no copy taken for a restore that never started");
    }

    #[test]
    fn a_damaged_backup_is_found_by_the_check_and_never_half_restored() {
        let _g = guard();
        let s = Scratch::new("damaged");
        let dir = save_folder(&s);
        // Data that doesn't compress, so the middle of the backup is this file's.
        let mut x = 1u32;
        let noise: Vec<u8> = (0..400_000).map(|_| { x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345); (x >> 16) as u8 }).collect();
        std::fs::write(dir.join("big.dat"), noise).unwrap();
        let kept = take_named(&dir, "x").unwrap();
        assert_eq!(check(&kept).unwrap(), 4);
        let mut bytes = std::fs::read(&kept).unwrap();
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xFF;
        std::fs::write(&kept, &bytes).unwrap();
        assert!(check(&kept).is_err());
        std::fs::write(dir.join("Farm_123"), b"farm two").unwrap();
        let was = read_tree(&dir);
        let e = restore(&dir, &kept).unwrap_err();
        assert!(e.unchanged, "{}", e.error);
        assert_eq!(read_tree(&dir), was, "put back as it was after the failed restore");
        // Not a zip at all.
        std::fs::write(&kept, b"not a zip").unwrap();
        assert!(check(&kept).is_err());
        assert!(restore(&dir, &kept).unwrap_err().unchanged);
    }

    #[test]
    fn a_restore_into_a_missing_folder_brings_it_back() {
        let _g = guard();
        let s = Scratch::new("missing");
        let dir = save_folder(&s);
        let was = read_tree(&dir);
        let kept = take_named(&dir, "x").unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        restore(&dir, &kept).unwrap();
        assert_eq!(read_tree(&dir), was);
        assert_eq!(list(&dir).len(), 1, "nothing to copy before: the folder was gone");
    }

    #[test]
    fn folders_that_are_not_save_folders_are_refused() {
        let _g = guard();
        let s = Scratch::new("refuse");
        // The folder that holds the backups themselves.
        assert!(take_named(&s.0, "x").is_err());
        let root = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
        assert!(contents(&root).is_err());
        if let Some(h) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            assert!(contents(Path::new(&h)).is_err());
        }
        // Too many files.
        let many = s.0.join("many");
        std::fs::create_dir_all(&many).unwrap();
        for i in 0..=MAX_FILES {
            std::fs::write(many.join(format!("{i}")), b"").unwrap();
        }
        assert!(take_named(&many, "x").is_err());
        assert!(list(&many).is_empty());
        assert!(take_named(&save_folder(&s), " ").is_err());
    }

    #[test]
    fn a_size_limit_never_removes_the_folder_backup_being_restored() {
        let _g = guard();
        let s = Scratch::new("cap-restore");
        let dir = save_folder(&s);
        let was = read_tree(&dir);
        copy(&dir, None, None).unwrap();
        let first = list(&dir)[0].path.clone();
        std::fs::write(dir.join("Farm_123"), b"farm two").unwrap();
        copy(&dir, None, None).unwrap();
        std::fs::write(dir.join("Farm_123"), b"farm three").unwrap();
        super::super::space::set_cap(1);
        let r = restore(&dir, &first);
        super::super::space::set_cap(0);
        r.unwrap();
        assert_eq!(read_tree(&dir), was);
        assert!(first.exists(), "the restored backup is still there");
    }

    #[test]
    fn automatic_folder_copies_are_pruned_and_named_ones_kept() {
        let _g = guard();
        let s = Scratch::new("prune");
        let dir = save_folder(&s);
        let named = take_named(&dir, "keep me").unwrap();
        for i in 0..super::super::keep() + 3 {
            std::fs::write(dir.join("Farm_123"), format!("v{i}")).unwrap();
            copy(&dir, None, None).unwrap();
        }
        let v = list(&dir);
        assert_eq!(v.iter().filter(|v| v.name.is_none()).count(), super::super::keep());
        assert!(named.exists());
        unlock(v.iter().find(|v| v.name.is_some()).unwrap()).unwrap();
        assert!(list(&dir).iter().all(|v| v.name.is_none()));
    }
}
