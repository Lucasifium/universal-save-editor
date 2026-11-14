//! Remembers where each game's saves were found, so the next start checks those folders
//! first instead of searching the whole computer again. Stored next to the settings
//! (`save_locations.json` in the app's config folder); a missing or damaged file is simply
//! an empty memory.

use crate::games::Located;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// After this long the remembered folders are searched for again anyway, to notice games
/// installed since.
pub const MAX_AGE_SECS: u64 = 7 * 24 * 3600;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct SaveCache {
    /// Seconds since 1970 of the last complete search.
    pub searched: u64,
    pub games: BTreeMap<String, Vec<Located>>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn path() -> Option<PathBuf> {
    crate::platform::config_dir().map(|d| d.join("save_locations.json"))
}

impl SaveCache {
    pub fn load() -> Self {
        path().map(|p| Self::load_from(&p)).unwrap_or_default()
    }

    pub fn load_from(p: &Path) -> Self {
        let Some(v) = std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else {
            return Self::default();
        };
        let mut c = Self { searched: v["searched"].as_u64().unwrap_or(0), games: BTreeMap::new() };
        if let Some(m) = v["games"].as_object() {
            for (name, dirs) in m {
                let list: Vec<Located> = dirs
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|d| {
                        let dir = d["dir"].as_str().filter(|s| !s.is_empty())?;
                        Some(Located { dir: PathBuf::from(dir), pass: d["pass"].as_bool().unwrap_or(false) })
                    })
                    .collect();
                if !list.is_empty() {
                    c.games.insert(name.clone(), list);
                }
            }
        }
        c
    }

    pub fn save(&self) {
        if let Some(p) = path() {
            self.save_to(&p);
        }
    }

    pub fn save_to(&self, p: &Path) {
        let games: serde_json::Map<String, Value> = self
            .games
            .iter()
            .map(|(k, v)| (k.clone(), Value::Array(v.iter().map(|l| json!({"dir": l.dir.to_string_lossy(), "pass": l.pass})).collect())))
            .collect();
        let v = json!({ "searched": self.searched, "games": games });
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Written beside and renamed, so a crash never leaves half a file.
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap_or_default()).is_ok() {
            let _ = std::fs::rename(&tmp, p);
        }
    }

    /// Whether there is a recent search to rely on.
    pub fn fresh(&self) -> bool {
        self.searched != 0 && now().saturating_sub(self.searched) < MAX_AGE_SECS
    }

    pub fn mark_searched(&mut self) {
        self.searched = now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("use_cache_{}_{name}", std::process::id()))
    }

    #[test]
    fn round_trips_and_ignores_damage() {
        let dir = tmp("rt");
        let p = dir.join("save_locations.json");
        let mut c = SaveCache::default();
        c.mark_searched();
        c.games.insert("Some Game".into(), vec![Located { dir: dir.join("a"), pass: false }, Located { dir: dir.join("b"), pass: true }]);
        c.save_to(&p);
        assert_eq!(SaveCache::load_from(&p), c);
        assert!(c.fresh());
        std::fs::write(&p, "{ not json").unwrap();
        assert_eq!(SaveCache::load_from(&p), SaveCache::default());
        assert!(!SaveCache::default().fresh());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_searches_are_not_fresh() {
        let c = SaveCache { searched: now() - MAX_AGE_SECS - 10, games: BTreeMap::new() };
        assert!(!c.fresh());
    }

    #[test]
    fn remembered_folder_is_verified_not_trusted() {
        let g = crate::games::GAMES.iter().find(|g| g.patterns.iter().any(|p| p.starts_with("*."))).unwrap();
        let ext = g.patterns.iter().find(|p| p.starts_with("*.")).unwrap().trim_start_matches('*');
        let dir = tmp("verify");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("slot1{ext}"));
        std::fs::write(&file, b"x").unwrap();
        let known = vec![Located { dir: dir.clone(), pass: false }, Located { dir: dir.join("gone"), pass: false }];
        let (found, still) = crate::games::find_in(g, &known);
        assert_eq!(found.len(), 1);
        assert_eq!(still, vec![known[0].clone()]);
        // The save is deleted: the folder is dropped from the memory.
        std::fs::remove_file(&file).unwrap();
        let (found, still) = crate::games::find_in(g, &known);
        assert!(found.is_empty() && still.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
