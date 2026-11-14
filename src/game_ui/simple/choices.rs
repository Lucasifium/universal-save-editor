//! Difficulty and mode values that are chosen from a few named levels: a screen shows a button
//! per level instead of a bare number.

pub struct Choice {
    /// The game screen (`Spec::name`) it belongs to.
    pub spec: &'static str,
    pub caption: &'static str,
    pub ptr: &'static str,
    /// `(name, stored value)`.
    pub levels: &'static [(&'static str, i64)],
}

pub const CHOICES: &[Choice] = &[
    Choice { spec: "Farming Simulator 19, 22 and 25: savegame", caption: "Economic difficulty", ptr: "/careerSavegame/settings/economicDifficulty", levels: &[("Easy", 1), ("Normal", 2), ("Hard", 3)] },
    Choice { spec: "Farming Simulator 19, 22 and 25: savegame", caption: "Difficulty", ptr: "/careerSavegame/settings/difficulty", levels: &[("Easy", 1), ("Normal", 2), ("Hard", 3)] },
    Choice { spec: "Farming Simulator 17", caption: "Difficulty", ptr: "/careerSavegame/settings/difficulty", levels: &[("Easy", 1), ("Normal", 2), ("Hard", 3)] },
    Choice { spec: "Farming Simulator 15", caption: "Difficulty", ptr: "/careerSavegame/@difficulty", levels: &[("Easy", 1), ("Normal", 2), ("Hard", 3)] },
    Choice { spec: "Kingdom Rush and Kingdom Rush Frontiers", caption: "Difficulty", ptr: "/difficulty", levels: &[("Casual", 1), ("Normal", 2), ("Veteran", 3), ("Impossible", 4)] },
    Choice { spec: "Kingdom Rush Origins", caption: "Difficulty", ptr: "/difficulty", levels: &[("Casual", 1), ("Normal", 2), ("Veteran", 3), ("Impossible", 4)] },
    Choice { spec: "Kingdom Rush 5: Alliance TD", caption: "Difficulty", ptr: "/difficulty", levels: &[("Casual", 1), ("Normal", 2), ("Veteran", 3), ("Impossible", 4)] },
];

/// Values the game keeps twice: `(screen, edited value, copy)`. When the first is changed the copy
/// is set to the same number, so the two never disagree.
pub const MIRRORS: &[(&str, &str, &str)] = &[
    ("House Flipper", "/moneyProgress/Money", "/moneyProgress/_money"),
];
