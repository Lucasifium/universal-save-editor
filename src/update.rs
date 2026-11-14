//! The optional "a new version is available" check. It is off until the player turns it on
//! (Preferences), or asks for one check from the Help menu. The only thing sent is one HTTPS
//! request for the public "latest release" page of the project on GitHub; nothing about the
//! player, their saves or their games goes in it. Failures (no network) are silent.

use std::sync::mpsc::{channel, Receiver};

pub const LATEST_API: &str = "https://api.github.com/repos/Lucasifium/universal-save-editor/releases/latest";
pub const RELEASES_PAGE: &str = "https://github.com/Lucasifium/universal-save-editor/releases/tag/";

/// What a check found: the newest published version, or why nothing is known.
pub enum Outcome {
    /// A published version (without the leading "v") newer than the one running.
    Newer(String),
    UpToDate,
    Failed,
}

/// "v0.6.1" / "0.6.1-beta" → (0, 6, 1). Missing parts count as 0; `None` when not a version.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    let patch = parts.next().map_or(Some(0), |p| p.parse().ok())?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Whether `latest` is a newer version than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// The version in the "latest release" reply, when it is a plain version number.
pub fn tag_from_reply(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = v["tag_name"].as_str()?.trim().trim_start_matches(['v', 'V']);
    parse_version(tag)?;
    // Only digits and dots go into the link that is shown.
    tag.chars().all(|c| c.is_ascii_digit() || c == '.').then(|| tag.to_string())
}

/// The page of a release.
pub fn release_url(version: &str) -> String {
    format!("{RELEASES_PAGE}v{version}")
}

pub fn decide(reply: Option<&str>, current: &str) -> Outcome {
    match reply.and_then(tag_from_reply) {
        Some(t) if is_newer(&t, current) => Outcome::Newer(t),
        Some(_) => Outcome::UpToDate,
        None => Outcome::Failed,
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn fetch() -> Option<String> {
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(8)).build();
    agent
        .get(LATEST_API)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", concat!("UniversalSaveEditor/", env!("CARGO_PKG_VERSION")))
        .call()
        .ok()?
        .into_string()
        .ok()
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
fn fetch() -> Option<String> {
    None
}

/// Whether this build can check at all (not the browser version or the phone app).
pub const AVAILABLE: bool = cfg!(not(any(target_arch = "wasm32", target_os = "android")));

/// Starts a check on its own thread; the answer arrives on the returned channel.
pub fn start() -> Receiver<Outcome> {
    let (tx, rx) = channel();
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let reply = fetch();
            let _ = tx.send(decide(reply.as_deref(), env!("CARGO_PKG_VERSION")));
        });
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = tx.send(Outcome::Failed);
    }
    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("v0.6.1"), Some((0, 6, 1)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("0.7.0-beta.1"), Some((0, 7, 0)));
        assert_eq!(parse_version("nightly"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn compares_numerically() {
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("v1.0.0", "0.99.99"));
        assert!(is_newer("0.6.2", "0.6.1"));
        assert!(!is_newer("0.6.1", "0.6.1"));
        assert!(!is_newer("0.6.0", "0.6.1"));
        assert!(!is_newer("garbage", "0.6.1"));
        assert!(!is_newer("0.7.0", "garbage"));
    }

    #[test]
    fn reads_the_reply() {
        assert_eq!(tag_from_reply(r#"{"tag_name":"v0.8.0","html_url":"x"}"#).as_deref(), Some("0.8.0"));
        assert_eq!(tag_from_reply(r#"{"tag_name":"v0.8.0/../evil"}"#), None);
        assert_eq!(tag_from_reply("not json"), None);
        assert_eq!(tag_from_reply(r#"{"message":"rate limited"}"#), None);
    }

    #[test]
    fn decides() {
        assert!(matches!(decide(Some(r#"{"tag_name":"v0.9.0"}"#), "0.7.0"), Outcome::Newer(v) if v == "0.9.0"));
        assert!(matches!(decide(Some(r#"{"tag_name":"v0.7.0"}"#), "0.7.0"), Outcome::UpToDate));
        assert!(matches!(decide(None, "0.7.0"), Outcome::Failed));
        assert_eq!(release_url("0.9.0"), "https://github.com/Lucasifium/universal-save-editor/releases/tag/v0.9.0");
    }
}
