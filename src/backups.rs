//! Timestamped copies of a save, taken every time the app writes over it, so any earlier
//! version can be brought back. They live in the app's own folder (not next to the save,
//! where a game or its cloud sync might trip over them):
//! `<config>/backups/<file name>-<path hash>/<unix seconds>.<ext>`.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Versions kept per save; the oldest go first.
pub const KEEP: usize = 30;

pub struct Version {
    pub path: PathBuf,
    /// Seconds since 1970 (UTC) when the copy was taken.
    pub when: u64,
    pub size: u64,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// FNV-1a, so two saves with the same name in different folders get different folders.
fn hash(s: &str) -> u32 {
    s.bytes().fold(0x811C_9DC5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

/// Where the versions of `save` are kept.
pub fn folder(save: &Path) -> Option<PathBuf> {
    let full = std::fs::canonicalize(save).unwrap_or_else(|_| save.to_path_buf());
    let name: String = save
        .file_name()
        .map(|n| n.to_string_lossy().chars().map(|c| if c.is_alphanumeric() || "._-".contains(c) { c } else { '_' }).collect())
        .unwrap_or_else(|| "save".into());
    let key = full.to_string_lossy().to_lowercase();
    crate::platform::config_dir().map(|d| d.join("backups").join(format!("{name}-{:08x}", hash(&key))))
}

/// Every kept version of `save`, newest first.
pub fn list(save: &Path) -> Vec<Version> {
    let Some(dir) = folder(save) else { return vec![] };
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    let mut out: Vec<Version> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let stem = p.file_stem()?.to_str()?;
            // "<secs>" or "<secs>-<n>" when two copies land in the same second.
            let when = stem.split('-').next()?.parse().ok()?;
            Some(Version { size: e.metadata().ok()?.len(), path: p, when })
        })
        .collect();
    out.sort_by(|a, b| b.when.cmp(&a.when).then_with(|| b.path.cmp(&a.path)));
    out
}

/// Copies the file as it is on disk now. Returns the copy, or `None` when the newest
/// version already holds exactly these bytes (or the file doesn't exist yet).
pub fn take(save: &Path) -> Result<Option<PathBuf>, String> {
    let Ok(bytes) = std::fs::read(save) else { return Ok(None) };
    let dir = folder(save).ok_or("no folder for backups on this device")?;
    let versions = list(save);
    if let Some(v) = versions.first() {
        if v.size == bytes.len() as u64 && std::fs::read(&v.path).is_ok_and(|b| b == bytes) {
            return Ok(None);
        }
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ext = save.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let t = now();
    let mut dest = dir.join(format!("{t}{ext}"));
    let mut n = 1;
    while dest.exists() {
        dest = dir.join(format!("{t}-{n}{ext}"));
        n += 1;
    }
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    for old in versions.iter().skip(KEEP - 1) {
        let _ = std::fs::remove_file(&old.path);
    }
    Ok(Some(dest))
}

/// "3 minutes ago", "yesterday 14:02 UTC"-style label for a version.
pub fn describe(when: u64) -> String {
    let ago = now().saturating_sub(when);
    let rel = match ago {
        0..=59 => "just now".to_string(),
        60..=3599 => plural(ago / 60, "minute"),
        3600..=86_399 => plural(ago / 3600, "hour"),
        _ => plural(ago / 86_400, "day"),
    };
    let (y, m, d) = civil(when / 86_400);
    let (hh, mm) = ((when % 86_400) / 3600, (when % 3600) / 60);
    format!("{rel} \u{2014} {y}-{m:02}-{d:02} {hh:02}:{mm:02} UTC")
}

fn plural(n: u64, unit: &str) -> String {
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

/// Days since 1970-01-01 to (year, month, day) — Howard Hinnant's algorithm.
fn civil(days: u64) -> (i64, u32, u32) {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(19_723), (2024, 1, 1));
        assert_eq!(civil(20_761), (2026, 11, 4));
    }
}
