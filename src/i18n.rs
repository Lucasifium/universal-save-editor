//! Interface translations.
//!
//! Text in the program is written in English and passed through `tr`, which looks it up in
//! the chosen language's catalog (`src/i18n/<code>.txt`) and falls back to the English text
//! when there is no translation. Game names, value names and game screens stay as they are.
//!
//! Catalog format: an English line, then a line starting with "= " holding its translation,
//! then a blank line. "\n" stands for a line break. Lines starting with "#" are comments.
//! `{0}`, `{1}`… in a text are filled in by `trf`, in whatever order the translation needs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

pub struct Lang {
    /// BCP 47 code ("de", "pt-BR").
    pub code: &'static str,
    /// The language's own name for itself.
    pub name: &'static str,
    /// Needs the Japanese/Chinese/Korean fonts.
    pub cjk: bool,
    catalog: &'static str,
}

pub const LANGS: &[Lang] = &[
    Lang { code: "en", name: "English", cjk: false, catalog: "" },
    Lang { code: "de", name: "Deutsch", cjk: false, catalog: include_str!("i18n/de.txt") },
    Lang { code: "es", name: "Español", cjk: false, catalog: include_str!("i18n/es.txt") },
    Lang { code: "fr", name: "Français", cjk: false, catalog: include_str!("i18n/fr.txt") },
    Lang { code: "pt-BR", name: "Português (Brasil)", cjk: false, catalog: include_str!("i18n/pt-BR.txt") },
    Lang { code: "ja", name: "日本語", cjk: true, catalog: include_str!("i18n/ja.txt") },
    Lang { code: "zh-CN", name: "简体中文", cjk: true, catalog: include_str!("i18n/zh-CN.txt") },
];

static CURRENT: AtomicUsize = AtomicUsize::new(0);

fn tables() -> &'static Vec<HashMap<String, &'static str>> {
    static T: OnceLock<Vec<HashMap<String, &'static str>>> = OnceLock::new();
    T.get_or_init(|| LANGS.iter().map(|l| parse(l.catalog).into_iter().collect()).collect())
}

/// (English, translation) pairs of a catalog, in file order.
pub fn parse(text: &'static str) -> Vec<(String, &'static str)> {
    let mut out = vec![];
    let mut key: Option<String> = None;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with('#') || line.trim().is_empty() {
            key = None;
            continue;
        }
        if let Some(t) = line.strip_prefix("= ") {
            if let Some(k) = key.take() {
                if !t.trim().is_empty() {
                    out.push((k, t));
                }
            }
        } else {
            key = Some(line.replace("\\n", "\n"));
        }
    }
    out
}

/// The language in use.
pub fn current() -> &'static Lang {
    &LANGS[CURRENT.load(Ordering::Relaxed).min(LANGS.len() - 1)]
}

/// Picks the language by code (`None`: the system's language, when there is a catalog for it).
pub fn set(code: Option<&str>) {
    let want = code.map(str::to_string).or_else(system_language);
    let i = want.as_deref().and_then(find).unwrap_or(0);
    CURRENT.store(i, Ordering::Relaxed);
}

/// The catalog matching a locale such as "de-AT", "pt_BR.UTF-8" or "zh-Hans-CN".
pub fn find(locale: &str) -> Option<usize> {
    let l = locale.split('.').next().unwrap_or("").replace('_', "-").to_ascii_lowercase();
    if let Some(i) = LANGS.iter().position(|x| x.code.to_ascii_lowercase() == l) {
        return Some(i);
    }
    let base = l.split('-').next().unwrap_or("");
    // Traditional Chinese reads Simplified poorly; leave it in English rather than guess.
    if base == "zh" && (l.contains("hant") || l.ends_with("-tw") || l.ends_with("-hk") || l.ends_with("-mo")) {
        return None;
    }
    LANGS.iter().position(|x| x.code.split('-').next() == Some(base))
}

fn system_language() -> Option<String> {
    if let Ok(v) = std::env::var("SAVE_EDITOR_LANG") {
        return Some(v);
    }
    sys_locale::get_locale()
}

/// `s` in the language in use.
/// `tr` for text that is not a literal (the texts of the data-driven game screens): translated
/// when the catalog has it, as written otherwise.
pub fn tr_any(s: &str) -> &str {
    let i = CURRENT.load(Ordering::Relaxed);
    if i == 0 {
        return s;
    }
    tables().get(i).and_then(|t| t.get(s).copied()).unwrap_or(s)
}

/// Marks a text of a data-driven game screen as translatable (the screen shows it through `tr_any`).
pub const fn tx(s: &'static str) -> &'static str {
    s
}

pub fn tr(s: &'static str) -> &'static str {
    let i = CURRENT.load(Ordering::Relaxed);
    if i == 0 {
        return s;
    }
    tables().get(i).and_then(|t| t.get(s).copied()).unwrap_or(s)
}

/// `tr`, then `{0}`, `{1}`… replaced by `args`.
pub fn trf(s: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = tr(s).to_string();
    for (i, a) in args.iter().enumerate() {
        out = out.replace(&format!("{{{i}}}"), &a.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every text a catalog translates must still be in the program, with the same
    /// placeholders, so a reworded English text doesn't keep a stale translation.
    #[test]
    fn catalogs_match_the_program() {
        let mut src = String::new();
        let mut stack = vec![std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    src.push_str(&std::fs::read_to_string(&p).unwrap());
                }
            }
        }
        let keys = keys_in(&src);
        let placeholders = |s: &str| {
            let mut v: Vec<String> = (0..9).map(|i| format!("{{{i}}}")).filter(|p| s.contains(p.as_str())).collect();
            v.sort();
            v
        };
        for l in &LANGS[1..] {
            let pairs = parse(l.catalog);
            assert!(!pairs.is_empty(), "{} catalog is empty", l.code);
            for (k, t) in &pairs {
                assert!(keys.contains(k), "{}: \"{k}\" is not in the program any more", l.code);
                assert_eq!(placeholders(k), placeholders(t), "{}: placeholders differ for \"{k}\"", l.code);
            }
        }
    }

    /// Every text the program passes to `tr`/`trf` must have a translation in every
    /// catalog, so a new screen can't ship with a language silently falling back to English.
    /// Add the missing entries to all of src/i18n/*.txt (English line, "= translation", blank line).
    #[test]
    fn every_text_is_translated_in_every_language() {
        let mut src = String::new();
        let mut stack = vec![std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("i18n.rs") {
                    src.push_str(&std::fs::read_to_string(&p).unwrap());
                }
            }
        }
        let keys = keys_in(&src);
        for l in &LANGS[1..] {
            let have: std::collections::HashSet<String> = parse(l.catalog).into_iter().map(|(k, _)| k).collect();
            let mut missing: Vec<&String> = keys.iter().filter(|k| !have.contains(*k)).collect();
            missing.sort();
            assert!(missing.is_empty(), "{}: {} text(s) have no translation, e.g. {:?}", l.code, missing.len(), &missing[..missing.len().min(8)]);
        }
    }

    /// The texts passed to `tr(...)`/`trf(...)` as literals, with Rust escapes resolved.
    pub fn keys_in(src: &str) -> std::collections::HashSet<String> {
        let mut out = std::collections::HashSet::new();
        for start in ["tr(\"", "trf(\"", "tx(\""] {
            let mut rest = src;
            while let Some(i) = rest.find(start) {
                let before = rest[..i].chars().last();
                rest = &rest[i + start.len()..];
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let mut s = String::new();
                let mut chars = rest.char_indices();
                while let Some((_, c)) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next().map(|x| x.1) {
                            Some('n') => s.push('\n'),
                            Some('u') => {
                                let mut hex = String::new();
                                for (_, h) in chars.by_ref() {
                                    if h == '}' {
                                        break;
                                    }
                                    if h != '{' {
                                        hex.push(h);
                                    }
                                }
                                s.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
                            }
                            Some(o) => s.push(o),
                            None => {}
                        },
                        c => s.push(c),
                    }
                }
                out.insert(s);
            }
        }
        out
    }

    #[test]
    fn finds_languages() {
        assert_eq!(find("de_AT.UTF-8").map(|i| LANGS[i].code), Some("de"));
        assert_eq!(find("pt-PT").map(|i| LANGS[i].code), Some("pt-BR"));
        assert_eq!(find("zh-Hans-CN").map(|i| LANGS[i].code), Some("zh-CN"));
        assert_eq!(find("zh-TW"), None);
        assert_eq!(find("nl-NL"), None);
    }
}
