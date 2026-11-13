//! Pokémon Essentials v19+ saves (Pokémon Infinite Fusion and other fan games): one Ruby
//! Marshal hash with `:player`, `:bag`, `:storage_system` and so on. The Marshal reader
//! does the reading and writing; this module adds what the game itself keeps in step:
//!
//! * the caps the game clamps to (money 999,999, 999 of an item, IVs 0–31, EVs 0–252…);
//! * a Pokémon's Exp. Points follow its level (and the level its Exp.), using the game's
//!   six growth rates, because the game works the level out from Exp. after the next battle;
//! * its stats are worked out again after a change to level, IVs, EVs or nature, with the
//!   game's own formula, when the save carries the species' base stats (Infinite Fusion
//!   keeps them with each Pokémon) and the stored stats match that formula;
//! * natures are picked from the game's 25, and shininess can be switched on for a
//!   Pokémon whose save still says "work it out" (nil).

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const KIND: &str = "Essentials fan-game save (Ruby Marshal)";

const PARTY: &str = "/Part 1/player/@party/";
const BOXES: &str = "/Part 1/storage_system/@boxes/";
/// Infinite Fusion's difficulty switches (Easy, Hard; neither on is Normal).
pub const SWITCH_EASY: &str = "/Part 1/switches/@data/665";
pub const SWITCH_HARD: &str = "/Part 1/switches/@data/666";
/// Infinite Fusion's "no levels" mode: stats then follow the badge count, not the level.
const SWITCH_NO_LEVELS: &str = "/Part 1/switches/@data/774";

pub const MAX_MONEY: i64 = 999_999;
pub const MAX_COINS: i64 = 99_999;
pub const MAX_PER_SLOT: i64 = 999;

/// The 25 natures in the game's order; nature n raises stat n / 5 and lowers stat n % 5
/// of Attack, Defense, Speed, Sp. Atk, Sp. Def (the same stat both ways: no change).
pub const NATURES: [&str; 25] = [
    "HARDY", "LONELY", "BRAVE", "ADAMANT", "NAUGHTY", "BOLD", "DOCILE", "RELAXED", "IMPISH", "LAX", "TIMID", "HASTY", "SERIOUS", "JOLLY", "NAIVE", "MODEST", "MILD", "QUIET", "BASHFUL", "RASH", "CALM", "GENTLE", "SASSY", "CAREFUL", "QUIRKY",
];
const NATURE_STATS: [&str; 5] = ["ATTACK", "DEFENSE", "SPEED", "SPECIAL_ATTACK", "SPECIAL_DEFENSE"];
/// Stat key in `@iv`/`@ev`/base stats → the Pokémon's stored stat.
const STATS: [(&str, &str); 6] = [("HP", "@totalhp"), ("ATTACK", "@attack"), ("DEFENSE", "@defense"), ("SPECIAL_ATTACK", "@spatk"), ("SPECIAL_DEFENSE", "@spdef"), ("SPEED", "@speed")];

/// Growth rates, by the names the game stores (`@growth_rate`).
const RATES: [&str; 6] = ["Medium", "Erratic", "Fluctuating", "Parabolic", "Fast", "Slow"];

/// Exp. Points needed to reach `level` at a growth rate (level 1 is 0).
pub fn min_exp(rate: &str, level: i64) -> i64 {
    let n = level.clamp(1, 100);
    if n == 1 {
        return 0;
    }
    let c = n * n * n;
    match rate {
        "Erratic" => match n {
            ..=50 => c * (100 - n) / 50,
            51..=68 => c * (150 - n) / 100,
            69..=98 => c * ((1911 - 10 * n) / 3) / 500,
            _ => c * (160 - n) / 100,
        },
        "Fluctuating" => match n {
            ..=15 => c * ((n + 1) / 3 + 24) / 50,
            16..=36 => c * (n + 14) / 50,
            _ => c * (n / 2 + 32) / 50,
        },
        "Parabolic" => c * 6 / 5 - 15 * n * n + 100 * n - 140,
        "Fast" => c * 4 / 5,
        "Slow" => c * 5 / 4,
        _ => c,
    }
}

/// The level a Pokémon with `exp` Exp. Points is at.
pub fn level_for_exp(rate: &str, exp: i64) -> i64 {
    (1..=100).rev().find(|&l| exp >= min_exp(rate, l)).unwrap_or(1)
}

/// The fewest Exp. Points that put a Pokémon at `level` whichever of `rates` it has, if
/// there is such a number (with one rate, simply where that level starts).
fn exp_for_level(rates: &[&str], level: i64) -> Option<i64> {
    if !(1..=100).contains(&level) {
        return None;
    }
    let lo = rates.iter().map(|r| min_exp(r, level)).max()?;
    let hi = if level == 100 { i64::MAX } else { rates.iter().map(|r| min_exp(r, level + 1)).min()? };
    (lo < hi).then_some(lo)
}

/// The level `exp` gives, if all of `rates` agree on it.
fn level_of(rates: &[&str], exp: i64) -> Option<i64> {
    let l = level_for_exp(rates.first()?, exp);
    rates.iter().all(|r| level_for_exp(r, exp) == l).then_some(l)
}

/// Splits a pointer into (the Pokémon's pointer, the field below it) for party and PC Pokémon.
pub fn mon_field(p: &str) -> Option<(&str, &str)> {
    let digits = |s: &str| s.find('/').filter(|&i| i > 0 && s[..i].bytes().all(|b| b.is_ascii_digit()));
    let cut = if let Some(rest) = p.strip_prefix(PARTY) {
        PARTY.len() + digits(rest)?
    } else {
        let rest = p.strip_prefix(BOXES)?;
        let a = digits(rest)?;
        let rest2 = rest[a..].strip_prefix("/@pokemon/")?;
        BOXES.len() + a + "/@pokemon/".len() + digits(rest2)?
    };
    Some((&p[..cut], &p[cut + 1..]))
}

/// A nil the reader should offer as on/off (a Pokémon's shininess, the difficulty switches).
pub fn nil_editable(p: &str) -> bool {
    p == SWITCH_EASY || p == SWITCH_HARD || mon_field(p).is_some_and(|(_, f)| f == "@shiny")
}

/// A symbol the reader should offer as a choice (a Pokémon's nature).
pub fn sym_editable(p: &str) -> bool {
    mon_field(p).is_some_and(|(_, f)| f == "@nature")
}

/// Is this Marshal tree an Essentials v19+ save?
pub fn looks(root: &Value) -> bool {
    let p = |s: &str| root.pointer(s).is_some();
    p("/Part 1/player/@party") && p("/Part 1/bag/@pockets") && p("/Part 1/storage_system/@boxes") && root.get("Part 2").is_none()
}

#[derive(Debug, Default)]
pub struct Info {
    /// Leaf → (lowest, highest) the game accepts.
    limits: HashMap<usize, (i64, i64)>,
    /// Leaf → (Pokémon pointer, field) for the fields that move other values.
    mon_leaves: HashMap<usize, (String, String)>,
    /// Pokémon pointer → its growth rate, or the rates its stored level and Exp. fit when
    /// the save doesn't name it (older versions).
    rates: HashMap<String, Vec<&'static str>>,
    /// Pokémon whose stored stats follow the formula, so they can be worked out again.
    stats_ok: HashSet<String>,
    natures: HashSet<usize>,
    nature_list: Arc<Vec<String>>,
    no_levels: bool,
}

fn int(v: &Value, p: &str) -> Option<i64> {
    v.pointer(p)?.as_i64()
}

fn sym(v: &Value, p: &str) -> Option<String> {
    v.pointer(p)?.as_str()?.strip_prefix(':').map(str::to_string)
}

/// The stats the game's `calc_stats` gives a Pokémon as it stands in `view`.
fn calc_stats(view: &Value, mon: &str) -> Option<Vec<(&'static str, i64)>> {
    let m = view.pointer(mon)?;
    let level = int(m, "/@level")?;
    let nature = sym(m, "/@nature_for_stats").or_else(|| sym(m, "/@nature"))?;
    let ni = NATURES.iter().position(|n| *n == nature)?;
    let ivmax = |s: &str| m.pointer(&format!("/@ivMaxed/{s}")).and_then(Value::as_bool).unwrap_or(false);
    let mut out = vec![];
    for (key, field) in STATS {
        let base = int(m, &format!("/@species_data/@base_stats/{key}"))?;
        let iv = if ivmax(key) { 31 } else { int(m, &format!("/@iv/{key}"))? };
        let ev = int(m, &format!("/@ev/{key}"))?;
        let core = (base * 2 + iv + ev / 4) * level / 100;
        let v = if key == "HP" {
            if base == 1 {
                1
            } else if sym(m, "/@ability").as_deref() == Some("WONDERGUARD") {
                1
            } else {
                core + level + 10
            }
        } else {
            let si = NATURE_STATS.iter().position(|s| *s == key)?;
            let (up, down) = (ni / 5, ni % 5);
            let modi = 100 + if up != down && si == up { 10 } else { 0 } - if up != down && si == down { 10 } else { 0 };
            (core + 5) * modi / 100
        };
        out.push((field, v));
    }
    Some(out)
}

fn stored_stats(view: &Value, mon: &str) -> Option<Vec<(&'static str, i64)>> {
    STATS.iter().map(|(_, f)| Some((*f, int(view, &format!("{mon}/{f}"))?))).collect()
}

impl Info {
    pub fn build(view: &Value, editable: &HashMap<String, usize>) -> Info {
        let mut info = Info {
            nature_list: Arc::new(NATURES.iter().map(|n| format!(":{n}")).collect()),
            no_levels: view.pointer(SWITCH_NO_LEVELS).and_then(Value::as_bool).unwrap_or(false),
            ..Default::default()
        };
        for (p, &leaf) in editable {
            let lim = match p.as_str() {
                "/Part 1/player/@money" => Some((0, MAX_MONEY)),
                "/Part 1/player/@coins" => Some((0, MAX_COINS)),
                "/Part 1/player/@battle_points" | "/Part 1/player/@soot" => Some((0, 9_999)),
                _ if p.starts_with("/Part 1/bag/@pockets/") && p.ends_with("/1") && p.matches('/').count() == 6 => Some((1, MAX_PER_SLOT)),
                _ => None,
            };
            if let Some(l) = lim {
                info.limits.insert(leaf, l);
            }
            let Some((mon, field)) = mon_field(p) else { continue };
            let lim = match field {
                "@level" => Some((1, 100)),
                "@happiness" => Some((0, 255)),
                f if f.starts_with("@iv/") && f.matches('/').count() == 1 => Some((0, 31)),
                f if f.starts_with("@ev/") && f.matches('/').count() == 1 => Some((0, 252)),
                _ => None,
            };
            if let Some(l) = lim {
                info.limits.insert(leaf, l);
            }
            if field == "@nature" {
                info.natures.insert(leaf);
            }
            let moves = matches!(field, "@level" | "@exp" | "@nature") || (field.matches('/').count() == 1 && (field.starts_with("@iv/") || field.starts_with("@ev/")));
            if moves {
                info.mon_leaves.insert(leaf, (mon.to_string(), field.to_string()));
            }
        }
        let mons: HashSet<String> = info.mon_leaves.values().map(|(m, _)| m.clone()).collect();
        for mon in mons {
            let Some(m) = view.pointer(&mon) else { continue };
            let (Some(level), Some(exp)) = (int(m, "/@level"), int(m, "/@exp")) else { continue };
            let rates: Vec<&'static str> = match sym(m, "/@species_data/@growth_rate") {
                Some(r) => RATES.iter().copied().filter(|x| *x == r).collect(),
                None => RATES.iter().copied().filter(|r| level_for_exp(r, exp) == level).collect(),
            };
            if !rates.is_empty() {
                info.rates.insert(mon.clone(), rates);
            }
            if calc_stats(view, &mon).is_some_and(|c| Some(c) == stored_stats(view, &mon)) {
                info.stats_ok.insert(mon);
            }
        }
        info
    }

    pub fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        if let Some(&(lo, hi)) = self.limits.get(&leaf) {
            crate::tree::int_in(v, lo as i128, hi as i128, "This number")?;
        }
        if let Some((mon, field)) = self.mon_leaves.get(&leaf) {
            let n = v.as_i64().unwrap_or(0);
            let ok = match (field.as_str(), self.rates.get(mon)) {
                ("@level", Some(r)) => exp_for_level(r, n).is_some(),
                ("@exp", Some(r)) => level_of(r, n).is_some(),
                ("@level" | "@exp", None) => false,
                _ => true,
            };
            if !ok {
                return Err("The save doesn't say how fast this Pokémon levels up, and the growth rates it could have don't agree on this; change it in the game".into());
            }
        }
        if self.natures.contains(&leaf) && !v.as_str().is_some_and(|s| self.nature_list.iter().any(|n| n == s)) {
            return Err("Pick one of the 25 natures".into());
        }
        Ok(())
    }

    pub fn choices(&self, leaf: usize) -> Option<Arc<Vec<String>>> {
        self.natures.contains(&leaf).then(|| self.nature_list.clone())
    }

    pub fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Some((mon, field)) = self.mon_leaves.get(&leaf) else { return vec![] };
        let mut out = vec![];
        if let Some(rates) = self.rates.get(mon) {
            match field.as_str() {
                "@level" => {
                    if let Some(e) = int(view, &format!("{mon}/@level")).and_then(|l| exp_for_level(rates, l)) {
                        out.push((format!("{mon}/@exp"), Value::from(e)));
                    }
                }
                "@exp" => {
                    if let Some(e) = int(view, &format!("{mon}/@exp")) {
                        let e = e.clamp(0, rates.iter().map(|r| min_exp(r, 100)).min().unwrap_or(0));
                        out.push((format!("{mon}/@exp"), Value::from(e)));
                        if let Some(l) = level_of(rates, e) {
                            out.push((format!("{mon}/@level"), Value::from(l)));
                        }
                    }
                }
                _ => {}
            }
        }
        if self.stats_ok.contains(mon) && !self.no_levels {
            // Work the stats out with the level as it will be after this change.
            let mut v2;
            let mut src = view;
            if let Some((p, nv)) = out.iter().find(|(p, _)| p.ends_with("/@level")) {
                v2 = view.clone();
                if let Some(t) = v2.pointer_mut(p) {
                    *t = nv.clone();
                }
                src = &v2;
            }
            if let (Some(new), Some(old)) = (calc_stats(src, mon), stored_stats(view, mon)) {
                let hp_lost = old.iter().find(|(f, _)| *f == "@totalhp").map(|x| x.1).unwrap_or(0) - int(view, &format!("{mon}/@hp")).unwrap_or(0);
                for (f, v) in &new {
                    out.push((format!("{mon}/{f}"), Value::from(*v)));
                    if *f == "@totalhp" {
                        out.push((format!("{mon}/@hp"), Value::from((v - hp_lost).max(0))));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_rates_match_known_values() {
        // Spot values of the six curves (level 2, 50 and 100).
        let want = [("Medium", [8, 125_000, 1_000_000]), ("Erratic", [15, 125_000, 600_000]), ("Fluctuating", [4, 142_500, 1_640_000]), ("Parabolic", [9, 117_360, 1_059_860]), ("Fast", [6, 100_000, 800_000]), ("Slow", [10, 156_250, 1_250_000])];
        for (r, v) in want {
            assert_eq!([min_exp(r, 2), min_exp(r, 50), min_exp(r, 100)], v, "{r}");
            assert_eq!(min_exp(r, 1), 0);
            for l in 1..=100 {
                assert_eq!(level_for_exp(r, min_exp(r, l)), l, "{r} {l}");
            }
        }
    }

    #[test]
    fn pointers_split_into_pokemon_and_field() {
        assert_eq!(mon_field("/Part 1/player/@party/3/@iv/HP"), Some(("/Part 1/player/@party/3", "@iv/HP")));
        assert_eq!(mon_field("/Part 1/storage_system/@boxes/12/@pokemon/29/@level"), Some(("/Part 1/storage_system/@boxes/12/@pokemon/29", "@level")));
        assert_eq!(mon_field("/Part 1/storage_system/@boxes/12/@name"), None);
        assert_eq!(mon_field("/Part 1/player/@party"), None);
        assert!(nil_editable("/Part 1/player/@party/0/@shiny"));
        assert!(!nil_editable("/Part 1/player/@party/0/@item"));
        assert!(sym_editable("/Part 1/storage_system/@boxes/0/@pokemon/1/@nature"));
        assert!(!sym_editable("/Part 1/player/@party/0/@nature_for_stats"));
    }
}
