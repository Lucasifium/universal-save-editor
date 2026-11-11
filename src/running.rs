//! Is the game that owns this save running right now? Games keep their save in memory and
//! write it back on autosave or when they quit, which silently undoes an edit made while
//! they're open. Used to warn before saving.

use std::path::Path;

/// Process names that never mean "the game": they show up in many save paths
/// (Steam\userdata, OneDrive\Documents…) without being the game.
const IGNORE: &[&str] = &[
    "steam",
    "steamwebhelper",
    "onedrive",
    "dropbox",
    "googledrivefs",
    "explorer",
    "epicgameslauncher",
    "galaxyclient",
    "code",
    "claude",
    "universalsaveeditor",
    "node",
    "python",
    "java",
    "javaw",
    "cmd",
    "powershell",
    "bash",
    "svchost",
    "documents",
    "users",
    "appdata",
    "local",
    "roaming",
    "locallow",
    "game",
    "games",
    "save",
    "saves",
];

/// Executables whose name doesn't match the folder their saves are in.
const ALIASES: &[(&str, &str, &str)] = &[
    ("skyrimse", "skyrimspecialedition", "Skyrim"),
    ("skyrim", "skyrim", "Skyrim"),
    ("tesv", "skyrim", "Skyrim"),
    ("fallout4", "fallout4", "Fallout 4"),
    ("stardewmoddingapi", "stardewvalley", "Stardew Valley"),
    ("hades2", "hadesii", "Hades II"),
    ("bg3", "baldursgate3", "Baldur's Gate 3"),
    ("bg3_dx11", "baldursgate3", "Baldur's Gate 3"),
    ("palworldwin64shipping", "palworld", "Palworld"),
    (
        "darksoulsremastered",
        "darksoulsremastered",
        "Dark Souls Remastered",
    ),
    ("darksouls", "darksouls", "Dark Souls"),
    ("darksoulsii", "darksoulsii", "Dark Souls II"),
    ("darksoulsiii", "darksoulsiii", "Dark Souls III"),
    ("eldenring", "eldenring", "Elden Ring"),
    ("sekiro", "sekiro", "Sekiro"),
    ("hollowknight", "hollowknight", "Hollow Knight"),
    ("stardewvalley", "stardewvalley", "Stardew Valley"),
];

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Names (lower-case, letters and digits only, no ".exe") of the running processes.
#[cfg(windows)]
fn processes() -> Vec<String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Ok(out) = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return vec![];
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split("\",\"").next())
        .map(|n| {
            let n = n.trim_start_matches('"');
            norm(
                n.strip_suffix(".exe")
                    .or_else(|| n.strip_suffix(".EXE"))
                    .unwrap_or(n),
            )
        })
        .collect()
}

#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
fn processes() -> Vec<String> {
    let Ok(rd) = std::fs::read_dir("/proc") else {
        return vec![];
    };
    rd.flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .filter_map(|e| std::fs::read(e.path().join("cmdline")).ok())
        .filter_map(|c| {
            // Proton/Wine games show up as "Z:\...\eldenring.exe".
            let first = String::from_utf8_lossy(c.split(|&b| b == 0).next()?).to_string();
            let base = first.rsplit(['/', '\\']).next()?.to_string();
            let base = base
                .strip_suffix(".exe")
                .map(str::to_string)
                .unwrap_or(base);
            Some(norm(&base))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn processes() -> Vec<String> {
    let Ok(out) = std::process::Command::new("ps")
        .args(["-axco", "command"])
        .output()
    else {
        return vec![];
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .skip(1)
        .map(norm)
        .collect()
}

#[cfg(any(target_os = "android", target_arch = "wasm32"))]
fn processes() -> Vec<String> {
    vec![]
}

/// The game that seems to own `save` and is running, if any. A process counts when its
/// name matches one of the folders the save is in ("eldenring.exe" and
/// …\EldenRing\7656…\ER0000.sl2), or a known alias of it.
pub fn game_for(save: &Path) -> Option<String> {
    let folders: Vec<(String, String)> = save
        .parent()?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .map(|c| (norm(&c), c))
        .filter(|c| c.0.len() >= 4)
        .collect();
    if folders.is_empty() {
        return None;
    }
    let procs = processes();
    procs
        .iter()
        .filter(|p| p.len() >= 4 && !IGNORE.contains(&p.as_str()))
        .find_map(|p| {
            if let Some(&(_, folder, name)) = ALIASES.iter().find(|a| a.0 == p.as_str()) {
                return folders
                    .iter()
                    .any(|f| f.0.starts_with(folder))
                    .then(|| name.to_string());
            }
            folders.iter().find(|f| &f.0 == p).map(|f| f.1.clone())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_normalize() {
        assert_eq!(norm("Hollow Knight"), "hollowknight");
        assert_eq!(norm("hollow_knight"), "hollowknight");
        assert_eq!(norm("Palworld-Win64-Shipping"), "palworldwin64shipping");
    }

    #[test]
    fn lists_processes() {
        // Whatever runs the tests is a process too; an empty list means the lookup broke.
        if cfg!(any(windows, target_os = "linux")) {
            assert!(!processes().is_empty());
        }
    }
}
