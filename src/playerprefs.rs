//! Unity PlayerPrefs, which many Unity games use for currency, unlocks and settings.
//!
//! - Windows keeps them in the registry (`HKCU\Software\<company>\<product>`). Each Unity game
//!   that has a `LocalLow\<company>\<product>` folder gets its key exported (`reg export`) to
//!   a `.reg` file in the app's folder; that file opens like any save, and saving it imports it
//!   back (`reg import`). The game must be closed: it writes its prefs back when it quits.
//! - macOS: `~/Library/Preferences/unity.<company>.<product>.plist`.
//! - Linux: `~/.config/unity3d/<company>/<product>/prefs` (XML).

use std::path::{Path, PathBuf};

/// Where the exported `.reg` files go.
pub fn export_dir() -> Option<PathBuf> {
    crate::platform::config_dir().map(|d| d.join("playerprefs"))
}

/// Exports every Unity game's PlayerPrefs found on this computer (Windows; elsewhere nothing
/// to do) and returns the folder holding the `.reg` files.
#[cfg(windows)]
pub fn export_all() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = export_dir()?;
    std::fs::create_dir_all(&out).ok()?;
    let low = std::env::var_os("USERPROFILE")
        .map(|h| PathBuf::from(h).join("AppData").join("LocalLow"))?;
    for company in std::fs::read_dir(&low).into_iter().flatten().flatten() {
        if !company.path().is_dir() {
            continue;
        }
        for product in std::fs::read_dir(company.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            if !product.path().is_dir() {
                continue;
            }
            let (c, p) = (
                company.file_name().to_string_lossy().to_string(),
                product.file_name().to_string_lossy().to_string(),
            );
            let file = out.join(format!("{c} - {p}.reg"));
            let key = format!(r"HKCU\Software\{c}\{p}");
            let ok = std::process::Command::new("reg")
                .args(["export", &key])
                .arg(&file)
                .arg("/y")
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !ok {
                let _ = std::fs::remove_file(&file);
            }
        }
    }
    Some(out)
}

#[cfg(not(windows))]
pub fn export_all() -> Option<PathBuf> {
    None
}

/// Is this one of the exported PlayerPrefs files?
pub fn is_export(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("reg"))
        && export_dir().is_some_and(|d| path.parent() == Some(d.as_path()))
}

/// After an exported `.reg` file is saved: put the values back in the registry.
pub fn after_save(path: &Path) -> Result<Option<String>, String> {
    if !is_export(path) {
        return Ok(None);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let o = std::process::Command::new("reg")
            .arg("import")
            .arg(path)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err(format!(
                "reg import failed: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ));
        }
        Ok(Some("PlayerPrefs written back to the registry.".into()))
    }
    #[cfg(not(windows))]
    Ok(Some("Saved the .reg file; import it on Windows (double-click it) to put the values in the registry.".into()))
}
