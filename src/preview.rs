//! A one-line look at a save for the "Find my saves" list.
//!
//! `peek` reads only a header the format keeps up front (Skyrim, Fallout 4), so it runs while
//! the computer is searched. `details` reads the whole file (up to 48 MB) for the formats where
//! who is in each slot can be read without decoding the rest: FromSoftware `.sl2` characters,
//! Diablo II characters, Pokémon trainers and the multi-slot saves that slots can be copied in.
//! It runs on a background thread, for the saves on screen only (`cached_details`).

use crate::i18n::trf;
use std::io::Read;
use std::path::Path;

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// A string stored as a 16-bit length and that many bytes; returns it and the next offset.
fn wstring(d: &[u8], at: usize) -> Option<(String, usize)> {
    let n = u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize;
    let bytes = d.get(at + 2..at + 2 + n)?;
    Some((bytes.iter().map(|&b| b as char).collect(), at + 2 + n))
}

/// Skyrim (LE and Special Edition) and Fallout 4 saves: name, level, location.
fn bethesda(head: &[u8]) -> Option<String> {
    let magic_len = if head.starts_with(b"TESV_SAVEGAME") {
        13
    } else if head.starts_with(b"FO4_SAVEGAME") {
        12
    } else {
        return None;
    };
    // magic, header size, then: version, save number, name, level, location.
    let at = magic_len + 4 + 4 + 4;
    let (name, at) = wstring(head, at)?;
    let level = u32_at(head, at)?;
    let (place, _) = wstring(head, at + 4)?;
    if name.trim().is_empty() || level == 0 || level > 10_000 {
        return None;
    }
    let place = place.trim();
    Some(if place.is_empty() { trf("{0}, level {1}", &[&name, &level]) } else { trf("{0}, level {1}, {2}", &[&name, &level, &place]) })
}

/// "Name, level 12, Place" for a save file, when its header allows it.
pub fn peek(path: &Path) -> Option<String> {
    let mut head = [0u8; 512];
    let n = std::fs::File::open(path).ok()?.read(&mut head).ok()?;
    bethesda(&head[..n])
}

/// Files larger than this get no details (Elden Ring's, the largest covered, is about 28 MB).
const MAX_DETAILS: u64 = 48 << 20;

/// At most this many slots are named; the rest are counted.
const NAMED: usize = 3;

fn join(parts: Vec<String>) -> Option<String> {
    if parts.is_empty() {
        return None;
    }
    let mut s = parts.iter().take(NAMED).cloned().collect::<Vec<_>>().join("  \u{00B7}  ");
    if parts.len() > NAMED {
        s.push_str("  \u{00B7}  ");
        s.push_str(&trf("{0} more", &[&(parts.len() - NAMED)]));
    }
    Some(s)
}

/// Dark Souls (Remastered), Dark Souls II and III, Sekiro: each character slot's name and level.
fn souls(d: &[u8]) -> Option<String> {
    let entries = crate::fmt::containers::bnd4_entries(d)?;
    let mut parts = vec![];
    let mut nameless = 0;
    for i in 0..10 {
        let Some(e) = entries.iter().find(|e| e.name == format!("USER_DATA{i:03}")) else { continue };
        let Some((slot, _, _)) = crate::fmt::containers::bnd4_open(d, e) else { continue };
        match crate::fmt::fromsoft::summary(&slot) {
            Some((Some(name), Some(level))) => parts.push(trf("{0}, level {1}", &[&name, &level])),
            Some(_) => nameless += 1,
            None => {}
        }
    }
    if parts.is_empty() && nameless > 0 {
        return Some(if nameless == 1 { trf("{0} save slot in use", &[&nameless]) } else { trf("{0} save slots in use", &[&nameless]) });
    }
    join(parts)
}

/// Pokémon (Generation I to III): the trainer's name and play time.
fn pokemon(d: &[u8]) -> Option<String> {
    let doc = if crate::fmt::pokemon3::looks(d) {
        crate::fmt::pokemon3::parse(d).ok()?
    } else if crate::fmt::pokemon12::looks(d) {
        crate::fmt::pokemon12::parse(d).ok()?
    } else {
        return None;
    };
    let v = &doc.view;
    let name = v.pointer("/Trainer/Name")?.as_str()?.trim().to_string();
    let h = v.pointer("/Trainer/Play time (hours)")?.as_u64()?;
    let m = v.pointer("/Trainer/Play time (minutes)")?.as_u64()?;
    (!name.is_empty() && name != "?").then(|| trf("{0}, {1}:{2}", &[&name, &h, &format!("{m:02}")]))
}

/// Who is in the save, for the formats listed in the module notes.
pub fn details_of(d: &[u8]) -> Option<String> {
    if let Some((_, slots)) = crate::fmt::slot_copy::slots(d) {
        let parts = slots.iter().enumerate().filter(|(_, s)| s.used).map(|(i, s)| trf("Slot {0}: {1}", &[&(i + 1), &s.label])).collect();
        return join(parts);
    }
    if d.starts_with(b"BND4") {
        return souls(d);
    }
    if let Some((name, class, level)) = crate::fmt::d2s::summary(d) {
        return Some(trf("{0}, {1}, level {2}", &[&name, &class, &level]));
    }
    pokemon(d)
}

/// `details_of` for a file on disk.
pub fn details(path: &Path) -> Option<String> {
    let size = std::fs::metadata(path).ok()?.len();
    if size > MAX_DETAILS {
        return None;
    }
    let d = std::fs::read(path).ok()?;
    // A damaged file must never take the list down with it.
    std::panic::catch_unwind(|| details_of(&d)).ok().flatten()
}

#[cfg(not(target_arch = "wasm32"))]
mod worker {
    use std::collections::{HashMap, VecDeque};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use std::time::SystemTime;

    type Key = (PathBuf, Option<SystemTime>);
    static DONE: Mutex<Option<HashMap<Key, Option<String>>>> = Mutex::new(None);
    static QUEUE: Mutex<VecDeque<Key>> = Mutex::new(VecDeque::new());
    static RUNNING: AtomicBool = AtomicBool::new(false);
    /// The file the worker is reading now (not queued again meanwhile).
    static IN_HAND: Mutex<Option<Key>> = Mutex::new(None);

    /// `Some(details)` once worked out; `None` while waiting (the file is queued).
    pub fn cached(path: &std::path::Path, modified: Option<SystemTime>) -> Option<Option<String>> {
        let key = (path.to_path_buf(), modified);
        if let Some(hit) = DONE.lock().ok()?.get_or_insert_with(HashMap::new).get(&key) {
            return Some(hit.clone());
        }
        let mut q = QUEUE.lock().ok()?;
        if !q.contains(&key) && IN_HAND.lock().ok().is_none_or(|h| h.as_ref() != Some(&key)) {
            q.push_back(key);
        }
        drop(q);
        if !RUNNING.swap(true, Ordering::SeqCst) {
            let spawned = std::thread::Builder::new().name("save details".into()).spawn(|| loop {
                let next = QUEUE.lock().ok().and_then(|mut q| q.pop_front());
                let Some(key) = next else {
                    RUNNING.store(false, Ordering::SeqCst);
                    // Something queued between the last pop and the store gets its own worker.
                    if QUEUE.lock().map(|q| q.is_empty()).unwrap_or(true) || RUNNING.swap(true, Ordering::SeqCst) {
                        return;
                    }
                    continue;
                };
                // Asked again while it was being read: already done.
                if DONE.lock().ok().is_some_and(|mut d| d.get_or_insert_with(HashMap::new).contains_key(&key)) {
                    continue;
                }
                if let Ok(mut h) = IN_HAND.lock() {
                    *h = Some(key.clone());
                }
                let found = super::details(&key.0);
                if let Ok(mut d) = DONE.lock() {
                    d.get_or_insert_with(HashMap::new).insert(key, found);
                }
                if let Ok(mut h) = IN_HAND.lock() {
                    *h = None;
                }
            });
            if spawned.is_err() {
                RUNNING.store(false, Ordering::SeqCst);
                return Some(None);
            }
        }
        None
    }
}

/// The save's details from a background thread: `Some` once known, `None` while still reading.
pub fn cached_details(path: &Path, modified: Option<std::time::SystemTime>) -> Option<Option<String>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        worker::cached(path, modified)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, modified);
        Some(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skyrim(name: &str, level: u32, place: &str) -> Vec<u8> {
        let mut d = b"TESV_SAVEGAME".to_vec();
        d.extend(100u32.to_le_bytes()); // header size
        d.extend(12u32.to_le_bytes()); // version
        d.extend(7u32.to_le_bytes()); // save number
        d.extend((name.len() as u16).to_le_bytes());
        d.extend(name.bytes());
        d.extend(level.to_le_bytes());
        d.extend((place.len() as u16).to_le_bytes());
        d.extend(place.bytes());
        d
    }

    #[test]
    fn reads_a_skyrim_header() {
        assert_eq!(bethesda(&skyrim("Lydia", 24, "Whiterun")).as_deref(), Some("Lydia, level 24, Whiterun"));
        assert_eq!(bethesda(&skyrim("Lydia", 24, "")).as_deref(), Some("Lydia, level 24"));
    }

    #[test]
    fn details_of_multi_slot_saves() {
        let d = crate::fmt::slot_copy::tests::elden_ring(&[(0, "Tarnished", 125), (2, "Other", 40)]);
        assert_eq!(details_of(&d).as_deref(), Some("Slot 1: Tarnished, level 125  \u{00B7}  Slot 3: Other, level 40"));
        let many: Vec<(usize, &str, u32)> = (0..5).map(|i| (i, "A", 10 + i as u32)).collect();
        let d = crate::fmt::slot_copy::tests::elden_ring(&many);
        assert!(details_of(&d).unwrap().ends_with("2 more"));
        assert_eq!(details_of(b"plain text"), None);
    }

    #[test]
    fn details_of_a_diablo_ii_character() {
        let d = crate::fmt::d2s::tests::sample();
        let (name, class, level) = crate::fmt::d2s::summary(&d).expect("a character");
        assert_eq!(details_of(&d), Some(format!("{name}, {class}, level {level}")));
    }

    #[test]
    fn details_wait_for_the_worker_then_stay() {
        let dir = std::env::temp_dir().join(format!("use_details_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.sl2");
        std::fs::write(&p, crate::fmt::slot_copy::tests::elden_ring(&[(0, "Tarnished", 7)])).unwrap();
        let mut got = None;
        for _ in 0..200 {
            if let Some(v) = cached_details(&p, None) {
                got = Some(v);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(got, Some(Some("Slot 1: Tarnished, level 7".to_string())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn other_files_have_no_preview() {
        assert_eq!(bethesda(b"not a save at all"), None);
        assert_eq!(bethesda(&skyrim("", 5, "x")), None);
        assert_eq!(bethesda(&skyrim("A", 0, "x")), None);
        assert_eq!(bethesda(b"TESV_SAVEGAME\x00\x00"), None, "a cut-off header is not read past its end");
    }
}
