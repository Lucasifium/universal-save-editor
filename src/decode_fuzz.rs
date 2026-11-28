//! Damaged and made-up files fed to every reader: opening, re-saving and the found-saves
//! details must never panic, whatever the bytes are. The browser version aborts on a panic,
//! and the desktop window reads small files on its own thread, so one panic loses the
//! player's unsaved work.
//!
//! The inputs are built here: the magic bytes each reader looks for (taken from the readers'
//! own source), followed by short, long, zero, 0xFF and pseudo-random tails, and the same
//! under the file extensions the readers look at; plus truncated copies of valid files.

use crate::formats::{decode_with, encode, Options};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// A small, fixed pseudo-random source (xorshift), so a failure repeats.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}

fn sources() -> Vec<String> {
    fn walk(d: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.extend(std::fs::read_to_string(&p));
            }
        }
    }
    let mut out = vec![];
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut out);
    out
}

/// The byte strings a reader compares the start of a file with: `starts_with(b"...")`.
fn magics(src: &[String]) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = vec![];
    for s in src {
        let mut rest = s.as_str();
        while let Some(at) = rest.find("starts_with(b\"") {
            rest = &rest[at + 14..];
            let Some(end) = rest.find('"') else { break };
            let lit = &rest[..end];
            rest = &rest[end..];
            let mut b = vec![];
            let mut it = lit.chars().peekable();
            let mut ok = true;
            while let Some(c) = it.next() {
                if c != '\\' {
                    b.extend(c.to_string().bytes());
                    continue;
                }
                match it.next() {
                    Some('x') => {
                        let h: String = it.by_ref().take(2).collect();
                        match u8::from_str_radix(&h, 16) {
                            Ok(v) => b.push(v),
                            Err(_) => ok = false,
                        }
                    }
                    Some('0') => b.push(0),
                    Some('n') => b.push(b'\n'),
                    Some('r') => b.push(b'\r'),
                    Some('t') => b.push(b'\t'),
                    Some(c) => b.extend(c.to_string().bytes()),
                    None => ok = false,
                }
            }
            if ok && b.len() >= 2 && !out.contains(&b) {
                out.push(b);
            }
        }
    }
    // Packings that are told apart by bytes the source writes as numbers.
    for m in [&[0x1f, 0x8b, 8][..], &[0x78, 0x9c], &[0x78, 0xda], &[0x78, 0x01], b"PK\x03\x04", &[0x28, 0xb5, 0x2f, 0xfd], b"\x04\x22\x4d\x18", b"{", b"[", b"<?xml", &[0xEF, 0xBB, 0xBF, b'{']] {
        if !out.iter().any(|o| o == m) {
            out.push(m.to_vec());
        }
    }
    out.sort();
    out
}

/// File extensions the readers look at (`"sav"` in a line that mentions `ext`).
fn extensions(src: &[String]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for s in src {
        for line in s.lines().filter(|l| l.contains("ext") && !l.trim_start().starts_with("//")) {
            for part in line.split('"').skip(1).step_by(2) {
                let p = part.trim_start_matches('.');
                if (2..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()) && !out.iter().any(|o| o == p) {
                    out.push(p.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

/// Opens, re-saves and describes `bytes` (named `path`), and says what panicked, if anything.
fn try_all(bytes: &[u8], path: Option<&Path>) -> Option<String> {
    let opts = path.map(Options::for_path).unwrap_or_default();
    catch_unwind(AssertUnwindSafe(|| {
        let d = decode_with(bytes, &opts);
        let _ = encode(&d.layers, &d.content);
        let _ = crate::preview::details_of(bytes);
        let _ = crate::fmt::slot_copy::slots(bytes);
        for (from, to) in [(0, 1), (1, 0), (0, 2), (3, 0)] {
            let _ = crate::fmt::slot_copy::copy(bytes, from, to);
        }
    }))
    .err()
    .map(|e| e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default())
}

fn tails(n: &mut Noise) -> Vec<Vec<u8>> {
    let mut v = vec![vec![], vec![0], vec![0xFF; 3], vec![0; 64], vec![0xFF; 64], vec![1; 300]];
    for len in [1, 4, 7, 16, 33, 128, 513, 4096] {
        v.push(n.bytes(len));
    }
    v
}

fn report(failures: &[String]) {
    assert!(failures.is_empty(), "{} inputs panicked:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn readers_never_panic_on_damaged_or_made_up_files() {
    let src = sources();
    let magics = magics(&src);
    assert!(magics.len() > 50, "magic bytes found in the readers: {}", magics.len());
    let mut noise = Noise(0x9E37_79B9_7F4A_7C15);
    let tails = tails(&mut noise);
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut failures = vec![];
    for m in &magics {
        for t in &tails {
            let bytes = [m.as_slice(), t].concat();
            if let Some(e) = try_all(&bytes, None) {
                failures.push(format!("{:?} + {} bytes: {e}", String::from_utf8_lossy(m), t.len()));
            }
        }
    }
    std::panic::set_hook(prev);
    report(&failures);
}

#[test]
fn readers_never_panic_whatever_the_file_is_called() {
    let src = sources();
    let exts = extensions(&src);
    assert!(exts.len() > 20, "extensions found in the readers: {}", exts.len());
    let mut noise = Noise(0x2545_F491_4F6C_DD1D);
    let tails = tails(&mut noise);
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut failures = vec![];
    for x in &exts {
        let path = PathBuf::from(format!("fuzz-{}.{x}", std::process::id()));
        for t in &tails {
            if let Some(e) = try_all(t, Some(&path)) {
                failures.push(format!(".{x}, {} bytes: {e}", t.len()));
            }
        }
    }
    std::panic::set_hook(prev);
    report(&failures);
}

/// Every cut-off length of small valid files, and each with one byte changed.
#[test]
fn truncated_and_flipped_copies_of_valid_files_never_panic() {
    use std::io::Write;
    let mut gz = flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
    gz.write_all(br#"{"gold": 100, "name": "Ann", "party": [1, 2, 3]}"#).unwrap();
    let gz = gz.finish().unwrap();
    let mut zip = vec![];
    {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut zip));
        w.start_file("save.json", zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(br#"{"hp": 5}"#).unwrap();
        w.finish().unwrap();
    }
    let er = crate::fmt::slot_copy::tests::elden_ring(&[(0, "Tarnished", 12)]);
    let files: Vec<(&str, Vec<u8>)> = vec![("save.dat", gz), ("save.zip", zip), ("ER0000.sl2", er), ("save.json", br#"{"a": {"b": [1, 2.5, "x", true, null]}}"#.to_vec())];
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut failures = vec![];
    let mut noise = Noise(7);
    for (name, f) in &files {
        let path = PathBuf::from(name);
        // Every length for the small ones; a spread of lengths for the big one.
        let step = (f.len() / 400).max(1);
        for cut in (0..f.len()).step_by(step) {
            if let Some(e) = try_all(&f[..cut], Some(&path)) {
                failures.push(format!("{name} cut to {cut}: {e}"));
            }
        }
        for _ in 0..40 {
            let mut c = f.clone();
            let i = noise.next() as usize % c.len();
            c[i] ^= (noise.next() as u8) | 1;
            if let Some(e) = try_all(&c, Some(&path)) {
                failures.push(format!("{name} with byte {i} changed: {e}"));
            }
        }
    }
    std::panic::set_hook(prev);
    report(&failures);
}
