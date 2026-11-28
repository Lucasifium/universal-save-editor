//! What the header card of a game screen says about the open file, beside its title: when the
//! file was last saved, and the play time where the save names it with its unit.

use crate::i18n::{tr, trf};
use serde_json::Value;
use std::time::SystemTime;

/// (caption, value, hover text) for the header card.
pub(super) fn facts(saved: Option<SystemTime>, root: &Value) -> Vec<(String, String, String)> {
    let mut out = vec![];
    if let Some(p) = play_time(root) {
        out.push((tr("Play time").to_string(), p, String::new()));
    }
    if let Some(secs) = saved.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()) {
        let full = crate::backups::describe(secs);
        // "2 hours ago, 2026-11-28 09:40 UTC": the first part on the card, all of it on hover.
        let short = full.rsplit_once(", ").map_or(full.clone(), |(rel, _)| rel.to_string());
        out.push((tr("Last saved").to_string(), short, full));
    }
    out
}

/// Keys the readers give a play time, with how many seconds one unit is.
const PLAY_TIME: &[(&str, f64)] = &[("Play time (seconds)", 1.0), ("Play time (minutes)", 60.0), ("Play time (hours)", 3600.0)];

/// The save's play time, when one of the top two levels holds it under a key that says its unit.
fn play_time(root: &Value) -> Option<String> {
    let at = |o: &serde_json::Map<String, Value>| PLAY_TIME.iter().find_map(|(k, unit)| o.get(*k).and_then(Value::as_f64).map(|n| n * unit));
    let obj = root.as_object()?;
    let secs = at(obj).or_else(|| obj.values().filter_map(Value::as_object).find_map(at))?;
    if !(0.0..1e10).contains(&secs) || secs < 1.0 {
        return None;
    }
    let mins = (secs / 60.0).round() as u64;
    Some(if mins < 60 { trf("{0} min", &[&mins]) } else { trf("{0} h {1} min", &[&(mins / 60), &format!("{:02}", mins % 60)]) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn play_time_is_read_only_where_the_key_names_its_unit() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("en"));
        assert_eq!(play_time(&json!({"Run": {"Play time (seconds)": 7380}})), Some("2 h 03 min".into()));
        assert_eq!(play_time(&json!({"Character": {"Play time (hours)": 0.5}})), Some("30 min".into()));
        assert_eq!(play_time(&json!({"Play time (minutes)": 90})), Some("1 h 30 min".into()));
        // A bare "playTime" could be seconds, frames or ticks: not shown.
        assert_eq!(play_time(&json!({"playTime": 7200})), None);
        assert_eq!(play_time(&json!({"a": {"b": {"Play time (seconds)": 50}}})), None, "only the top two levels");
        assert_eq!(play_time(&json!({"Play time (seconds)": -5})), None);
        assert_eq!(play_time(&json!([])), None);
    }

    #[test]
    fn last_saved_is_short_on_the_card_and_full_on_hover() {
        let _lock = crate::i18n::LANG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        crate::i18n::set(Some("en"));
        let f = facts(Some(SystemTime::now() - std::time::Duration::from_secs(7300)), &json!({}));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].0, "Last saved");
        assert_eq!(f[0].1, "2 hours ago");
        assert!(f[0].2.starts_with("2 hours ago, ") && f[0].2.ends_with(" UTC"), "{}", f[0].2);
        assert!(facts(None, &json!({})).is_empty());
    }
}
