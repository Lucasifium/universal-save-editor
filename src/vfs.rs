//! File access for the app window. The desktop and Android builds use the real file system.
//! The browser version can't see the user's disks: the files they pick are kept in memory
//! under their file name, and saving hands the result back as a download (see `web.rs`).

#[cfg(target_arch = "wasm32")]
use std::io;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub use std::fs::{copy, read, read_to_string, remove_file, write};

#[cfg(not(target_arch = "wasm32"))]
pub fn exists(p: &Path) -> bool {
    p.exists()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn len(p: &Path) -> Option<u64> {
    std::fs::metadata(p).ok().map(|m| m.len())
}

#[cfg(target_arch = "wasm32")]
mod mem {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    thread_local! {
        pub static FILES: RefCell<HashMap<PathBuf, Vec<u8>>> = RefCell::new(HashMap::new());
    }
}

#[cfg(target_arch = "wasm32")]
fn missing(p: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{} isn't open in this tab", p.display()),
    )
}

#[cfg(target_arch = "wasm32")]
pub fn read(p: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    let p = p.as_ref();
    mem::FILES
        .with(|f| f.borrow().get(p).cloned())
        .ok_or_else(|| missing(p))
}

#[cfg(target_arch = "wasm32")]
pub fn read_to_string(p: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(p)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(target_arch = "wasm32")]
pub fn write(p: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
    mem::FILES.with(|f| {
        f.borrow_mut()
            .insert(p.as_ref().to_path_buf(), data.as_ref().to_vec())
    });
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn remove_file(p: impl AsRef<Path>) -> io::Result<()> {
    let p = p.as_ref();
    mem::FILES
        .with(|f| f.borrow_mut().remove(p))
        .map(|_| ())
        .ok_or_else(|| missing(p))
}

#[cfg(target_arch = "wasm32")]
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let d = read(from)?;
    let n = d.len() as u64;
    write(to, d).map(|_| n)
}

#[cfg(target_arch = "wasm32")]
pub fn exists(p: &Path) -> bool {
    mem::FILES.with(|f| f.borrow().contains_key(p))
}

#[cfg(target_arch = "wasm32")]
pub fn len(p: &Path) -> Option<u64> {
    mem::FILES.with(|f| f.borrow().get(p).map(|d| d.len() as u64))
}

/// The first bytes of a file (for sniffing its type).
pub fn head(p: &Path, n: usize) -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Read;
        let mut v = Vec::with_capacity(n);
        std::fs::File::open(p)
            .ok()?
            .take(n as u64)
            .read_to_end(&mut v)
            .ok()?;
        Some(v)
    }
    #[cfg(target_arch = "wasm32")]
    {
        mem::FILES.with(|f| f.borrow().get(p).map(|d| d[..n.min(d.len())].to_vec()))
    }
}
