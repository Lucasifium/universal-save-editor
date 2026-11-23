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

/// Tables whose rows are named by an id that another list of the same save explains:
/// `(screen, table path, [(list path, pointer of the id inside an entry, pointer of the name)])`.
pub const NAME_LOOKUPS: &[(&str, &str, &[(&str, &str, &str)])] = &[(
    "Space Engineers",
    "/MyObjectBuilder_Checkpoint/SessionComponents/MyObjectBuilder_SessionComponent/=MyObjectBuilder_BankingSystem/Accounts/MyObjectBuilder_AccountEntry",
    &[
        ("/MyObjectBuilder_Checkpoint/Identities/MyObjectBuilder_Identity", "/IdentityId", "/DisplayName"),
        ("/MyObjectBuilder_Checkpoint/Factions/Factions/MyObjectBuilder_Faction", "/FactionId", "/Name"),
    ],
)];

/// The name another list gives to `id` for this table, if any.
pub fn lookup_name(root: &serde_json::Value, spec: &str, table: &str, id: &str) -> Option<String> {
    let (_, _, lists) = NAME_LOOKUPS.iter().find(|(s, t, _)| *s == spec && *t == table)?;
    for (list, id_ptr, name_ptr) in *lists {
        let list = super::resolve(root, list);
        let entries: Vec<&serde_json::Value> = match root.pointer(&list)? {
            serde_json::Value::Array(a) => a.iter().collect(),
            o @ serde_json::Value::Object(_) => vec![o],
            _ => continue,
        };
        for e in entries {
            let same = match e.pointer(id_ptr) {
                Some(serde_json::Value::String(s)) => s == id,
                Some(v) => v.to_string() == id,
                None => false,
            };
            if same {
                if let Some(n) = e.pointer(name_ptr).and_then(|v| v.as_str()).filter(|n| !n.is_empty()) {
                    return Some(n.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    #[test]
    fn space_engineers_credits_are_named_from_identities_and_factions() {
        let root = serde_json::json!({"MyObjectBuilder_Checkpoint": {
            "Identities": {"MyObjectBuilder_Identity": [{"IdentityId": 7, "DisplayName": "Pilot"}]},
            "Factions": {"Factions": {"MyObjectBuilder_Faction": [{"FactionId": 9, "Name": "Space Pirates"}]}}}});
        let t = NAME_LOOKUPS[0].1;
        assert_eq!(lookup_name(&root, "Space Engineers", t, "7").as_deref(), Some("Pilot"));
        assert_eq!(lookup_name(&root, "Space Engineers", t, "9").as_deref(), Some("Space Pirates"));
        assert_eq!(lookup_name(&root, "Space Engineers", t, "11"), None);
    }
}
