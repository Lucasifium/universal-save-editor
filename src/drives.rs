//! Which drives the save search looks at, and Windows' known folders (Documents and Saved Games
//! wherever the user has moved them).
//!
//! Only fixed drives are searched, and only a short list of usual places on each (Steam
//! libraries); nothing crawls a whole disk. Removable and network drives are skipped unless the
//! user turns that on in the settings.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static REMOVABLE: AtomicBool = AtomicBool::new(false);

/// Also search removable and network drives (off by default).
pub fn set_include_removable(on: bool) {
    REMOVABLE.store(on, Ordering::Relaxed);
}

pub fn include_removable() -> bool {
    REMOVABLE.load(Ordering::Relaxed)
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fixed,
    Removable,
    Network,
}

/// Where a Steam library or install is usually put on a drive of its own.
const STEAM_PLACES: &[&str] = &["SteamLibrary", "Steam", "Games/Steam", "Games/SteamLibrary", "Program Files (x86)/Steam", "Program Files/Steam"];

/// The roots to search among `drives`: every fixed one, and the others when `removable`.
pub fn select(drives: &[(PathBuf, Kind)], removable: bool) -> Vec<PathBuf> {
    drives.iter().filter(|(_, k)| *k == Kind::Fixed || removable).map(|(p, _)| p.clone()).collect()
}

/// The drives to search now.
pub fn roots() -> Vec<PathBuf> {
    select(&all(), include_removable())
}

/// Steam folders (with `steamapps` or `userdata`) at the usual places of each drive in `roots`.
pub fn steam_roots_on(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = vec![];
    for r in roots {
        for place in STEAM_PLACES {
            let p = r.join(place);
            if p.join("steamapps").is_dir() || p.join("userdata").is_dir() {
                out.push(p);
            }
        }
    }
    out
}

#[cfg(windows)]
mod sys {
    #[repr(C)]
    pub struct Guid(pub u32, pub u16, pub u16, pub [u8; 8]);
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetLogicalDrives() -> u32;
        pub fn GetDriveTypeW(root: *const u16) -> u32;
    }
    #[link(name = "shell32")]
    extern "system" {
        pub fn SHGetKnownFolderPath(id: *const Guid, flags: u32, token: isize, out: *mut *mut u16) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        pub fn CoTaskMemFree(p: *mut core::ffi::c_void);
    }
}

/// Every lettered drive that is a disk (not a CD drive), with what kind it is.
#[cfg(windows)]
fn all() -> Vec<(PathBuf, Kind)> {
    let mask = unsafe { sys::GetLogicalDrives() };
    let mut out = vec![];
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        let root: Vec<u16> = format!("{letter}:\\").encode_utf16().chain([0]).collect();
        let kind = match unsafe { sys::GetDriveTypeW(root.as_ptr()) } {
            3 => Kind::Fixed,
            2 => Kind::Removable,
            4 => Kind::Network,
            _ => continue,
        };
        out.push((PathBuf::from(format!("{letter}:\\")), kind));
    }
    out
}

#[cfg(not(windows))]
fn all() -> Vec<(PathBuf, Kind)> {
    vec![]
}

#[derive(Clone, Copy)]
pub enum Known {
    Documents,
    SavedGames,
}

/// A user folder from Windows itself, so a redirected Documents (to another drive or to
/// OneDrive) is found; `None` elsewhere or when Windows has none.
#[cfg(windows)]
pub fn known_folder(which: Known) -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    let id = match which {
        Known::Documents => sys::Guid(0xFDD39AD0, 0x238F, 0x46AF, [0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7]),
        Known::SavedGames => sys::Guid(0x4C5C32FF, 0xBB9D, 0x43B0, [0xB5, 0xB4, 0x2D, 0x72, 0xE5, 0x4E, 0xAA, 0xA4]),
    };
    let mut raw: *mut u16 = std::ptr::null_mut();
    let hr = unsafe { sys::SHGetKnownFolderPath(&id, 0, 0, &mut raw) };
    if hr < 0 || raw.is_null() {
        unsafe { sys::CoTaskMemFree(raw.cast()) };
        return None;
    }
    let len = (0..).take_while(|&i| unsafe { *raw.add(i) } != 0).count();
    let s = std::ffi::OsString::from_wide(unsafe { std::slice::from_raw_parts(raw, len) });
    unsafe { sys::CoTaskMemFree(raw.cast()) };
    Some(PathBuf::from(s))
}

#[cfg(not(windows))]
pub fn known_folder(_: Known) -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fixed_drives_unless_asked() {
        let d = vec![(PathBuf::from("C:\\"), Kind::Fixed), (PathBuf::from("E:\\"), Kind::Fixed), (PathBuf::from("F:\\"), Kind::Removable), (PathBuf::from("Z:\\"), Kind::Network)];
        assert_eq!(select(&d, false), vec![PathBuf::from("C:\\"), PathBuf::from("E:\\")]);
        assert_eq!(select(&d, true).len(), 4);
    }

    #[test]
    fn finds_steam_folders_on_other_drives_without_crawling() {
        let tmp = std::env::temp_dir().join(format!("use_drives_{}", std::process::id()));
        let (d1, d2) = (tmp.join("d1"), tmp.join("d2"));
        std::fs::create_dir_all(d1.join("SteamLibrary/steamapps")).unwrap();
        std::fs::create_dir_all(d1.join("Other/SteamLibrary/steamapps")).unwrap();
        std::fs::create_dir_all(d2.join("Games/Steam/userdata")).unwrap();
        std::fs::create_dir_all(d2.join("Games/Empty")).unwrap();
        let found = steam_roots_on(&[d1.clone(), d2.clone()]);
        assert_eq!(found, vec![d1.join("SteamLibrary"), d2.join("Games/Steam")]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
