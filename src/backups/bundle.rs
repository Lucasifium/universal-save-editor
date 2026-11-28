//! All backups of one save in a single zip file, to move them to another computer or keep a
//! copy somewhere else, and back again. The zip holds the single-file backups under `files/`,
//! the whole-folder backups of the save's folder under `folders/`, and a small note naming the
//! save. Importing only ever adds backups: it never replaces one, and never touches the save.

use super::folders::{self, write_zip, Source};
use crate::i18n::{tr, trf};
use std::io::Read;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

/// The note inside every exported zip.
pub const NOTE: &str = "universal-save-editor-backups.json";
/// The largest single backup an import reads.
const MAX_ENTRY: u64 = 4 << 30;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Exported {
    pub files: usize,
    pub folders: usize,
    pub bytes: u64,
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// `name` with its same-second counter raised until `taken` says it is free:
/// `1700000000~boss.sav` becomes `1700000000-1~boss.sav`, then `-2`, and so on.
fn free_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(name) {
        return name.to_string();
    }
    let cut = name.find(['~', '.']).unwrap_or(name.len());
    let (stamp, rest) = name.split_at(cut);
    let (secs, n) = stamp.split_once('-').unwrap_or((stamp, "0"));
    let mut n: u32 = n.parse().unwrap_or(0);
    loop {
        n += 1;
        let next = format!("{secs}-{n}{rest}");
        if !taken(&next) {
            return next;
        }
    }
}

/// Writes every backup of `save` (and every whole-folder backup of its folder) into one zip at
/// `dest`. The zip is checked against the backups before it takes the name `dest`.
pub fn export(save: &Path, dest: &Path) -> Result<Exported, String> {
    let files = super::list(save);
    let dirs = save.parent().map(folders::list).unwrap_or_default();
    if files.is_empty() && dirs.is_empty() {
        return Err(tr("there are no backups of this save to export").into());
    }
    if same_file(dest, save) || files.iter().chain(&dirs).any(|v| same_file(dest, &v.path)) {
        return Err(tr("choose another file name: that one is the save or one of its backups").into());
    }
    let note = serde_json::json!({
        "save": save.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        "version": 1,
    })
    .to_string();
    let mut entries: Vec<(String, Source)> = vec![(NOTE.to_string(), Source::Bytes(note.as_bytes()))];
    let mut out = Exported::default();
    for (prefix, list) in [("files", &files), ("folders", &dirs)] {
        for v in list.iter() {
            let base = v.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            // The same name can come from two backups folders: keep both.
            let name = free_name(&base, |n| entries.iter().any(|(e, _)| *e == format!("{prefix}/{n}")));
            entries.push((format!("{prefix}/{name}"), Source::File(&v.path)));
            out.bytes += v.size;
            if prefix == "files" { out.files += 1 } else { out.folders += 1 }
        }
    }
    write_zip(dest, &entries, true)?;
    Ok(out)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Imported {
    /// Backups added.
    pub added: usize,
    /// Backups that were already here, byte for byte.
    pub already: usize,
    /// The save the zip was exported from, by file name.
    pub from: String,
}

/// Adds the backups in `zip_path` (a zip made by [`export`]) to the backups of `save`. A
/// backup already here is skipped; one whose name is taken by a different copy is added under
/// a free name. On an error, the backups added before it stay and are counted in the message.
pub fn import(save: &Path, zip_path: &Path) -> Result<Imported, String> {
    let f = std::fs::File::open(zip_path).map_err(|e| trf("couldn't read {0}: {1}", &[&zip_path.display(), &e]))?;
    let mut zip = zip::ZipArchive::new(f).map_err(|_| tr("that file isn't a backups file made by Export backups").to_string())?;
    let mut note = String::new();
    zip.by_name(NOTE)
        .map_err(|_| tr("that file isn't a backups file made by Export backups").to_string())?
        .take(64 << 10)
        .read_to_string(&mut note)
        .map_err(|e| e.to_string())?;
    let from = serde_json::from_str::<serde_json::Value>(&note).ok().and_then(|v| v["save"].as_str().map(str::to_string)).unwrap_or_default();
    let mut done = Imported { from, ..Default::default() };
    let file_dir = super::folder(save);
    let folder_dir = save.parent().and_then(folders::home);
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).map_err(|e| trf("the backups file is damaged: {0}", &[&e]))?;
        let name = e.name().to_string();
        let Some((prefix, base)) = name.split_once('/') else { continue };
        // Only names this app makes: one level down, a stamp it can read, no path in it.
        let fits = !base.is_empty() && !base.contains(['/', '\\', ':', '\0']) && super::parse_name(base).is_some();
        let dir = match prefix {
            "files" if fits && !folders::is_folder_copy(Path::new(base)) => file_dir.as_ref(),
            "folders" if fits && folders::is_folder_copy(Path::new(base)) => folder_dir.as_ref(),
            _ => continue,
        };
        let Some(dir) = dir else { return Err(tr("no folder on this device can hold backups").into()) };
        let fail = |what: String| {
            if done.added > 0 {
                trf("{0} (the {1} backups added before this stay)", &[&what, &done.added])
            } else {
                what
            }
        };
        if e.size() > MAX_ENTRY {
            return Err(fail(trf("{0} is too large to import", &[&base])));
        }
        let mut bytes = Vec::with_capacity(e.size().min(64 << 20) as usize);
        e.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut bytes).map_err(|err| fail(trf("the backups file is damaged: {0}", &[&format!("{name}: {err}")])))?;
        std::fs::create_dir_all(dir).map_err(|err| fail(format!("{}: {err}", dir.display())))?;
        let here = |n: &str| dir.join(n);
        if std::fs::read(here(base)).is_ok_and(|b| b == bytes) {
            done.already += 1;
            continue;
        }
        let target = here(&free_name(base, |n| here(n).exists()));
        super::write_verified(&target, &bytes, false).map_err(fail)?;
        super::space::remember(dir);
        done.added += 1;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::super::tests::LOCK;
    use super::super::{list, set_root, take, take_named};
    use super::*;

    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("use-bundle-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            set_root(Some(d.join("own")));
            Scratch(d)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            set_root(None);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn sorted(save: &Path) -> Vec<(Option<String>, Vec<u8>)> {
        let mut v: Vec<_> = list(save).iter().map(|v| (v.name.clone(), std::fs::read(&v.path).unwrap())).collect();
        v.sort();
        v
    }

    #[test]
    fn backups_move_to_another_computer_and_back() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let s = Scratch::new("move");
        let save = s.0.join("old pc").join("slot.sav");
        std::fs::create_dir_all(save.parent().unwrap()).unwrap();
        for v in ["one", "two", "three"] {
            std::fs::write(&save, v).unwrap();
            take(&save).unwrap();
        }
        take_named(&save, "before boss").unwrap();
        std::fs::write(save.parent().unwrap().join("other.dat"), b"x").unwrap();
        folders::take_named(save.parent().unwrap(), "whole").unwrap();
        let zip = s.0.join("slot backups.zip");
        let e = export(&save, &zip).unwrap();
        assert_eq!((e.files, e.folders), (4, 1));
        // On the new computer the save is somewhere else, with no backups yet.
        set_root(Some(s.0.join("new own")));
        let moved = s.0.join("new pc").join("slot.sav");
        std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
        std::fs::write(&moved, b"live").unwrap();
        let i = import(&moved, &zip).unwrap();
        assert_eq!((i.added, i.already, i.from.as_str()), (5, 0, "slot.sav"));
        set_root(Some(s.0.join("own")));
        let want = sorted(&save);
        set_root(Some(s.0.join("new own")));
        assert_eq!(sorted(&moved), want);
        assert_eq!(folders::list(moved.parent().unwrap()).len(), 1);
        assert_eq!(std::fs::read(&moved).unwrap(), b"live", "the save itself is never touched");
        // Importing again adds nothing.
        let again = import(&moved, &zip).unwrap();
        assert_eq!((again.added, again.already), (0, 5));
        assert_eq!(list(&moved).len(), 4);
    }

    #[test]
    fn an_import_never_replaces_a_different_backup_with_the_same_name() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let s = Scratch::new("clash");
        let save = s.0.join("slot.sav");
        std::fs::write(&save, b"theirs").unwrap();
        let theirs = take(&save).unwrap().unwrap();
        let zip = s.0.join("b.zip");
        export(&save, &zip).unwrap();
        // Here, a different copy already has that name.
        std::fs::write(&theirs, b"mine").unwrap();
        let i = import(&save, &zip).unwrap();
        assert_eq!(i.added, 1);
        assert_eq!(std::fs::read(&theirs).unwrap(), b"mine");
        let mut got: Vec<Vec<u8>> = list(&save).iter().map(|v| std::fs::read(&v.path).unwrap()).collect();
        got.sort();
        assert_eq!(got, vec![b"mine".to_vec(), b"theirs".to_vec()]);
    }

    #[test]
    fn exporting_over_the_save_or_a_backup_is_refused() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let s = Scratch::new("refuse");
        let save = s.0.join("slot.sav");
        assert!(export(&save, &s.0.join("x.zip")).is_err(), "nothing to export");
        std::fs::write(&save, b"live").unwrap();
        let b = take(&save).unwrap().unwrap();
        assert!(export(&save, &save).is_err());
        assert!(export(&save, &b).is_err());
        assert_eq!(std::fs::read(&save).unwrap(), b"live");
        assert_eq!(std::fs::read(&b).unwrap(), b"live");
    }

    #[test]
    fn a_foreign_or_hostile_zip_adds_nothing_outside_the_backups_folder() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let s = Scratch::new("hostile");
        let save = s.0.join("slot.sav");
        std::fs::write(&save, b"live").unwrap();
        // A zip that wasn't made by export.
        let plain = s.0.join("plain.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&plain).unwrap());
        z.start_file("files/1700000000.sebak", zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut z, b"x").unwrap();
        z.finish().unwrap();
        assert!(import(&save, &plain).is_err());
        // One that has the note but names meant to escape, or that aren't backups.
        let evil = s.0.join("evil.zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
        for name in [NOTE, "files/../../slot.sav", "files/x/1700000000.sebak", "files/readme.txt", "slot.sav", "folders/1700000000.sebak", "files/1700000001.sebak.partial"] {
            z.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            std::io::Write::write_all(&mut z, b"{}").unwrap();
        }
        z.finish().unwrap();
        let i = import(&save, &evil).unwrap();
        assert_eq!(i.added, 0);
        assert_eq!(std::fs::read(&save).unwrap(), b"live");
        assert!(list(&save).is_empty());
        assert!(!s.0.join("x").exists());
    }

    #[test]
    fn free_names_count_up_within_the_second() {
        let taken = ["1700000000.sav", "1700000000-1.sav", "5~boss.sav"];
        let t = |n: &str| taken.contains(&n);
        assert_eq!(free_name("1700000000.sav", t), "1700000000-2.sav");
        assert_eq!(free_name("5~boss.sav", t), "5-1~boss.sav");
        assert_eq!(free_name("9.sav", t), "9.sav");
    }
}
