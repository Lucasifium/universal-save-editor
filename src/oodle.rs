//! Oodle (RAD Game Tools / Epic) compression, used by Palworld and Hogwarts Legacy saves.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]
//!
//! Oodle has no open-source implementation we may ship, so nothing of it is built into this
//! program. Instead the editor uses the Oodle library that games already install on this
//! computer (`oo2core_*_win64.dll`, or `liboo2corelinux64.so*` on Linux), loaded while the
//! program runs, exactly as the game itself does. Set `SAVE_EDITOR_OODLE` to the library's
//! path if it isn't found automatically.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

type Decompress = unsafe extern "C" fn(
    *const u8,
    i64,
    *mut u8,
    i64,
    i32,
    i32,
    i32,
    *mut u8,
    i64,
    *const u8,
    *const u8,
    *mut u8,
    i64,
    i32,
) -> i64;
type Compress = unsafe extern "C" fn(
    i32,
    *const u8,
    i64,
    *mut u8,
    i32,
    *const u8,
    *const u8,
    *const u8,
    *mut u8,
    i64,
) -> i64;

struct Lib {
    #[cfg(not(target_arch = "wasm32"))]
    _lib: libloading::Library,
    decompress: Decompress,
    compress: Option<Compress>,
    path: PathBuf,
}

/// Kraken, the compressor Unreal games use.
const KRAKEN: i32 = 8;
/// "Normal" level (fast, good ratio).
const NORMAL: i32 = 4;

fn names(n: &str) -> bool {
    let n = n.to_lowercase();
    (n.starts_with("oo2core_") && n.ends_with("_win64.dll"))
        || n.starts_with("liboo2corelinux64.so")
}

/// The library file with the highest version number among `dir`'s games (two levels deep).
fn search() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SAVE_EDITOR_OODLE")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return Some(p);
    }
    // Where it was found last time (searching every game folder takes a few seconds).
    let memo = crate::platform::config_dir().map(|d| d.join("oodle_library.txt"));
    if let Some(p) = memo
        .as_ref()
        .and_then(|m| std::fs::read_to_string(m).ok())
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| p.is_file())
    {
        return Some(p);
    }
    let found = search_games();
    if let (Some(m), Some(f)) = (&memo, &found) {
        let _ = std::fs::create_dir_all(m.parent().unwrap_or(m));
        let _ = std::fs::write(m, f.to_string_lossy().as_bytes());
    }
    found
}

fn search_games() -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = vec![];
    let mut look = |dir: &Path, depth: usize| {
        fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    if depth > 0 {
                        walk(&p, depth - 1, out);
                    }
                } else if names(&e.file_name().to_string_lossy()) {
                    out.push(p);
                }
            }
        }
        walk(dir, depth, &mut found);
    };
    for lib in crate::games::steam_libraries() {
        look(&lib.join("steamapps").join("common"), 3);
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(pf) = std::env::var_os(var) {
            look(&PathBuf::from(pf).join("Epic Games"), 3);
        }
    }
    // Newest version first: oo2core_9 decodes everything the older ones wrote.
    // "oo2core_9_win64.dll" → 9; "liboo2corelinux64.so.9" → 9.
    let version = |p: &PathBuf| {
        let n = p
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let tail = n
            .strip_prefix("oo2core_")
            .or_else(|| n.rsplit_once(".so.").map(|(_, v)| v))
            .unwrap_or("");
        tail.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0)
    };
    found.sort_by_key(|p| std::cmp::Reverse(version(p)));
    found.into_iter().next()
}

fn lib() -> Option<&'static Lib> {
    static L: OnceLock<Option<Lib>> = OnceLock::new();
    L.get_or_init(load).as_ref()
}

/// The browser version can't load a library from the user's games.
#[cfg(target_arch = "wasm32")]
fn load() -> Option<Lib> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn load() -> Option<Lib> {
    let path = search()?;
    // SAFETY: loading the game's own Oodle library and looking up its two documented
    // entry points; their signatures are the published Oodle 2.x ones.
    unsafe {
        let lib = libloading::Library::new(&path).ok()?;
        let decompress = *lib.get::<Decompress>(b"OodleLZ_Decompress\0").ok()?;
        let compress = lib.get::<Compress>(b"OodleLZ_Compress\0").ok().map(|s| *s);
        Some(Lib {
            _lib: lib,
            decompress,
            compress,
            path,
        })
    }
}

/// Where the Oodle library was found, if it was.
pub fn location() -> Option<&'static Path> {
    lib().map(|l| l.path.as_path())
}

/// Why Oodle data can't be opened here (shown to the player).
pub const MISSING: &str = "This save uses Oodle compression. No Oodle library (oo2core_*_win64.dll) was found on this \
computer; it comes with many games (Sekiro, Telltale games…). Set SAVE_EDITOR_OODLE to its path, or open the \
save on a computer that has one.";

pub fn decompress(src: &[u8], raw_len: usize) -> Option<Vec<u8>> {
    let l = lib()?;
    let mut out = vec![0u8; raw_len];
    // SAFETY: buffers are valid for the lengths given; Oodle writes at most `raw_len` bytes.
    let n = unsafe {
        (l.decompress)(
            src.as_ptr(),
            src.len() as i64,
            out.as_mut_ptr(),
            raw_len as i64,
            1,
            0,
            0,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
            3,
        )
    };
    (n == raw_len as i64).then_some(out)
}

pub fn compress(src: &[u8]) -> Option<Vec<u8>> {
    let l = lib()?;
    let f = l.compress?;
    // Oodle's documented worst case: input + 274 bytes per 256 KiB block (rounded up generously).
    let cap = src.len() + 274 * (src.len() / 0x40000 + 2) + 64;
    let mut out = vec![0u8; cap];
    // SAFETY: as above; `cap` covers Oodle's maximum compressed size.
    let n = unsafe {
        f(
            KRAKEN,
            src.as_ptr(),
            src.len() as i64,
            out.as_mut_ptr(),
            NORMAL,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
        )
    };
    if n <= 0 || n as usize > cap {
        return None;
    }
    out.truncate(n as usize);
    // Check it reads back before trusting it with a save.
    (decompress(&out, src.len()).as_deref() == Some(src)).then_some(out)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn round_trip_with_local_library() {
        let Some(p) = super::location() else {
            println!("no Oodle library found");
            return;
        };
        println!("using {}", p.display());
        let data: Vec<u8> = (0..200_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let c = super::compress(&data).expect("compress");
        assert!(c.len() < data.len());
        assert_eq!(super::decompress(&c, data.len()).unwrap(), data);
    }
}
