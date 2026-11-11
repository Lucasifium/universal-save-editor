//! Xbox app / PC Game Pass saves (the Microsoft Store "wgs" container).
//!
//! The Xbox app keeps each game's saves under
//! `%LOCALAPPDATA%\Packages\<package>\SystemAppData\wgs\<user id>_<title id>\`:
//!
//! - `containers.index` lists the containers (a save slot, or one file the game asked for) with
//!   the folder each one lives in;
//! - `<folder GUID>\container.<n>` lists the files in that container;
//! - `<folder GUID>\<file GUID>` is the file itself, byte for byte what the game wrote.
//!
//! There's no compression, encryption or checksum at this level, so a blob opens in the
//! editor like the same game's Steam save, and saving writes the blob back in place (same name,
//! same folder), which is how other Game Pass tools replace a save too.
//!
//! Layout from Z1ni's XGP-save-extractor (MIT), which credits the format work in
//! goatfungus/NMSSaveEditor issue 306.

use std::path::{Path, PathBuf};

/// One file inside a container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blob {
    /// The name the game gave the file.
    pub name: String,
    pub path: PathBuf,
}

/// A container: usually one save slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub name: String,
    pub files: Vec<Blob>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Index {
    /// The Store package name (`Publisher.Game_hash`).
    pub package: String,
    pub containers: Vec<Container>,
    /// Problems worth showing (a file missing while the Xbox app is still syncing).
    pub warnings: Vec<String>,
}

struct Rd<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self
            .d
            .get(self.p..self.p + n)
            .ok_or("containers.index ends early")?;
        self.p += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn utf16(&mut self, chars: Option<usize>) -> Result<String, String> {
        let n = match chars {
            Some(n) => n,
            None => {
                let n = self.i32()?;
                if !(0..=4096).contains(&n) {
                    return Err("containers.index: a name is too long".into());
                }
                n as usize
            }
        };
        let raw = self.take(n * 2)?;
        let units: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string())
    }
    fn guid(&mut self) -> Result<String, String> {
        Ok(guid_hex(self.take(16)?))
    }
}

/// A GUID stored little-endian, as the folder and file names write it (32 upper-case hex digits).
fn guid_hex(b: &[u8]) -> String {
    let order = [3, 2, 1, 0, 5, 4, 7, 6, 8, 9, 10, 11, 12, 13, 14, 15];
    order.iter().map(|&i| format!("{:02X}", b[i])).collect()
}

/// Does this look like a blob or container folder name (32 hex digits)?
pub fn is_guid_name(name: &str) -> bool {
    name.len() == 32 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The files listed in one `container.<n>`.
pub fn read_container_file(d: &[u8]) -> Result<Vec<(String, String, String)>, String> {
    let mut r = Rd { d, p: 0 };
    r.take(4)?;
    let count = r.i32()?;
    if !(0..=10_000).contains(&count) {
        return Err("container file: bad file count".into());
    }
    let mut out = vec![];
    for _ in 0..count {
        let name = r.utf16(Some(64))?;
        let g1 = r.guid()?;
        let g2 = r.guid()?;
        out.push((name, g1, g2));
    }
    Ok(out)
}

/// Reads a `wgs\<user>` folder.
pub fn read_index(dir: &Path) -> Result<Index, String> {
    let d = std::fs::read(dir.join("containers.index"))
        .map_err(|e| format!("containers.index: {e}"))?;
    parse_index(&d, |folder, num| {
        let cdir = dir.join(folder);
        let raw = std::fs::read(cdir.join(format!("container.{num}"))).ok()?;
        Some((cdir, raw))
    })
}

/// Parses `containers.index`; `open` returns a container's folder and its `container.<n>`.
pub fn parse_index(
    d: &[u8],
    open: impl Fn(&str, u8) -> Option<(PathBuf, Vec<u8>)>,
) -> Result<Index, String> {
    let mut r = Rd { d, p: 0 };
    r.take(4)?;
    let count = r.i32()?;
    if !(0..=100_000).contains(&count) {
        return Err("containers.index: bad container count".into());
    }
    let _display_name = r.utf16(None)?;
    let package = r
        .utf16(None)?
        .split('!')
        .next()
        .unwrap_or_default()
        .to_string();
    r.take(8)?; // created (FILETIME)
    r.take(4)?;
    r.utf16(None)?;
    r.take(8)?;
    let mut idx = Index {
        package,
        ..Default::default()
    };
    for _ in 0..count {
        let name = r.utf16(None)?;
        r.utf16(None)?;
        r.utf16(None)?;
        let num = r.u8()?;
        r.take(4)?;
        let folder = r.guid()?;
        r.take(8)?; // modified (FILETIME)
        r.take(16)?;
        let Some((cdir, raw)) = open(&folder, num) else {
            idx.warnings.push(format!(
                "\u{201C}{name}\u{201D} is missing (the Xbox app may still be syncing)"
            ));
            continue;
        };
        let mut files = vec![];
        for (fname, g1, g2) in read_container_file(&raw)? {
            let (p1, p2) = (cdir.join(&g1), cdir.join(&g2));
            let path = match (p1.is_file(), g1 == g2 || !p2.is_file()) {
                (true, true) => p1,
                (false, _) if p2.is_file() => p2,
                (true, false) => {
                    idx.warnings.push(format!("\u{201C}{name}/{fname}\u{201D} has two copies (sync in progress?); skipped"));
                    continue;
                }
                _ => {
                    idx.warnings.push(format!("\u{201C}{name}/{fname}\u{201D} is missing (the Xbox app may still be syncing)"));
                    continue;
                }
            };
            files.push(Blob { name: fname, path });
        }
        idx.containers.push(Container { name, files });
    }
    Ok(idx)
}

/// `container/file` for a blob, as the game named it, or `None` if `path` isn't a Game Pass blob.
pub fn logical_name(path: &Path) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static CACHE: Mutex<Option<HashMap<PathBuf, Option<String>>>> = Mutex::new(None);
    let name = path.file_name()?.to_string_lossy().to_string();
    if !is_guid_name(&name) {
        return None;
    }
    if let Some(hit) = CACHE
        .lock()
        .ok()?
        .get_or_insert_with(HashMap::new)
        .get(path)
    {
        return hit.clone();
    }
    let found = (|| {
        let user = path.parent()?.parent()?;
        let idx = read_index(user).ok()?;
        idx.containers.iter().find_map(|c| {
            c.files.iter().find(|f| f.path == path).map(|f| {
                if c.files.len() == 1 && (f.name == c.name || f.name.is_empty() || f.name == "data")
                {
                    c.name.clone()
                } else {
                    format!("{}/{}", c.name, f.name)
                }
            })
        })
    })();
    if let Ok(mut c) = CACHE.lock() {
        c.get_or_insert_with(HashMap::new)
            .insert(path.to_path_buf(), found.clone());
    }
    found
}

/// Every Game Pass user folder (`wgs\<user>_<title>`) for `package` (`*` for every package).
pub fn user_dirs(packages_root: &Path, package: &str) -> Vec<PathBuf> {
    let pkgs: Vec<PathBuf> = if package == "*" {
        std::fs::read_dir(packages_root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect()
    } else {
        vec![packages_root.join(package)]
    };
    let mut out = vec![];
    for p in pkgs {
        for e in std::fs::read_dir(p.join("SystemAppData").join("wgs"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let n = e.file_name().to_string_lossy().to_string();
            if n != "t" && !n.contains("backup") && e.path().join("containers.index").is_file() {
                out.push(e.path());
            }
        }
    }
    out
}

/// The package a blob or user folder belongs to (`…\Packages\<package>\SystemAppData\…`).
pub fn package_of(path: &Path) -> Option<String> {
    let parts: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let i = parts
        .iter()
        .position(|p| p.eq_ignore_ascii_case("SystemAppData"))?;
    let pkg = parts.get(i.checked_sub(1)?)?;
    // "Publisher.Game_hash": the readable part.
    let short = pkg.split('_').next().unwrap_or(pkg);
    Some(
        short
            .split_once('.')
            .map(|(_, g)| g)
            .unwrap_or(short)
            .to_string(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn utf16(s: &str, prefixed: bool) -> Vec<u8> {
        let mut out = vec![];
        if prefixed {
            out.extend((s.encode_utf16().count() as i32).to_le_bytes());
        }
        out.extend(s.encode_utf16().flat_map(|u| u.to_le_bytes()));
        out
    }

    fn guid_bytes(hex: &str) -> Vec<u8> {
        let b: Vec<u8> = (0..16)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
            .collect();
        let order = [3, 2, 1, 0, 5, 4, 7, 6, 8, 9, 10, 11, 12, 13, 14, 15];
        order.iter().map(|&i| b[i]).collect()
    }

    /// A synthetic `wgs` user folder with one container holding `files`.
    pub fn make(dir: &Path, container: &str, files: &[(&str, &[u8])]) {
        let folder = "0123456789ABCDEF0123456789ABCDEF";
        let mut idx = vec![14, 0, 0, 0];
        idx.extend(1i32.to_le_bytes());
        idx.extend(utf16("", true));
        idx.extend(utf16("Studio.Game_abc123!App", true));
        idx.extend([0; 12]);
        idx.extend(utf16("", true));
        idx.extend([0; 8]);
        idx.extend(utf16(container, true));
        idx.extend(utf16(container, true));
        idx.extend(utf16("\"0x8DB\"", true));
        idx.push(3);
        idx.extend([0; 4]);
        idx.extend(guid_bytes(folder));
        idx.extend([0; 24]);
        std::fs::create_dir_all(dir.join(folder)).unwrap();
        std::fs::write(dir.join("containers.index"), idx).unwrap();
        let mut c = vec![4, 0, 0, 0];
        c.extend((files.len() as i32).to_le_bytes());
        for (i, (name, data)) in files.iter().enumerate() {
            let g = format!("{:032X}", 0xAB00 + i);
            let mut n = utf16(name, false);
            n.resize(128, 0);
            c.extend(n);
            c.extend(guid_bytes(&g));
            c.extend(guid_bytes(&g));
            std::fs::write(dir.join(folder).join(&g), data).unwrap();
        }
        std::fs::write(dir.join(folder).join("container.3"), c).unwrap();
    }

    #[test]
    fn reads_a_wgs_folder() {
        let tmp = std::env::temp_dir().join(format!("use_xgp_{}", std::process::id()));
        let user = tmp.join("Packages/Studio.Game_abc123/SystemAppData/wgs/000901F0A1B2C3D4_0000000000000000000000006A7B8C9D");
        make(&user, "Slot1", &[("save.sav", b"hello"), ("meta", b"m")]);
        let idx = read_index(&user).unwrap();
        assert_eq!(idx.package, "Studio.Game_abc123");
        assert_eq!(idx.containers.len(), 1);
        assert_eq!(idx.containers[0].name, "Slot1");
        assert_eq!(idx.containers[0].files.len(), 2);
        assert_eq!(
            std::fs::read(&idx.containers[0].files[0].path).unwrap(),
            b"hello"
        );
        assert_eq!(
            logical_name(&idx.containers[0].files[0].path).as_deref(),
            Some("Slot1/save.sav")
        );
        assert_eq!(
            package_of(&idx.containers[0].files[0].path).as_deref(),
            Some("Game")
        );
        assert_eq!(user_dirs(&tmp.join("Packages"), "*"), vec![user.clone()]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
