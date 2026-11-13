//! Short risk notes for values that are easy to get wrong: shown on hover next to the value
//! and repeated in the "Save changes?" list. Matching is by the value's own key (the last
//! part of its path) and, for a few, the kind of save. The notes only say what is generally
//! true; they are advice, not a block.

struct Rule {
    /// The save kind must contain this (case-insensitive); empty: any kind.
    kind: &'static str,
    /// The value's key must contain one of these (lower case).
    keys: &'static [&'static str],
    /// The key must not contain any of these.
    not: &'static [&'static str],
    note: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        kind: "",
        keys: &["steamid", "steam_id", "accountid", "account_id", "userid", "user_id", "profileid", "xuid", "playerid", "player_id"],
        not: &[],
        note: "An account id. If it doesn't match the account that opens the save, the game may refuse to load it or ignore it (the Steam account switch tool moves a save between accounts).",
    },
    Rule {
        kind: "",
        keys: &["gold", "money", "cash", "credits", "coins", "currency", "souls", "runes", "bells", "gil", "zenny", "meseta", "dollars", "silver", "bits"],
        not: &["max", "limit", "cap", "name", "icon", "id", "type", "count_", "recipe", "rate", "multiplier"],
        note: "Very large amounts can overflow the game's own limit and wrap around to a negative number. Stay under the game's in-game cap (often 999,999,999) to be safe.",
    },
    Rule {
        kind: "",
        keys: &["seed"],
        not: &[],
        note: "The world seed only matters for terrain the game hasn't made yet. Changing it won't redo explored areas, so new areas may not line up with them.",
    },
    Rule {
        kind: "",
        keys: &["version", "build", "revision", "schema"],
        not: &["name", "text", "string"],
        note: "The game reads this to decide how to load the save. A wrong value can make it refuse the save or convert it.",
    },
    Rule {
        kind: "",
        keys: &["quest", "chapter", "mission", "objective", "storyprogress", "story_progress", "questflag", "cutscene"],
        not: &["name", "title", "text", "desc", "icon"],
        note: "Story progress. Setting it out of order can skip, repeat or break quests; keep a backup of the save you started from (the editor makes one automatically).",
    },
    Rule {
        kind: "",
        keys: &["experience", "exp", "xp", "totalxp", "total_xp"],
        not: &["expire", "expand", "explos", "expos", "export", "expr", "name", "text", "next", "required", "need"],
        note: "Experience and level are linked in most games. If you change one, change the other to match, or the game may recalculate it or show a mismatch.",
    },
    Rule {
        kind: "",
        keys: &["level", "lvl"],
        not: &["name", "text", "id", "type", "icon", "unlock", "required", "req", "scal", "map", "difficulty", "volume", "sound", "audio", "music", "light", "detail", "graphics", "quality", "blocklevel", "water"],
        note: "A level above what the game expects can crash it or lock content. The allowed range shown is what the save file can hold, not what the game accepts.",
    },
    Rule {
        kind: "elden ring",
        keys: &["runes", "level", "quantity", "count", "vigor", "mind", "endurance", "strength", "dexterity", "intelligence", "faith", "arcane"],
        not: &[],
        note: "Edited stats or items can get an account banned if you play online. Edit offline saves only, and play offline (without Easy Anti-Cheat) afterwards.",
    },
    Rule {
        kind: "dark souls",
        keys: &["souls", "level", "quantity", "count", "humanity", "vitality", "attunement", "endurance", "strength", "dexterity", "resistance", "intelligence", "faith", "vigor"],
        not: &[],
        note: "Edited stats or items can get an account banned if you play online. Edit offline saves only.",
    },
    Rule {
        kind: "sekiro",
        keys: &["level", "attack", "vitality", "xp", "skill", "quantity", "count", "money"],
        not: &[],
        note: "Edited stats or items can get an account banned if you play online. Edit offline saves only.",
    },
    Rule {
        kind: "borderlands",
        keys: &["level", "xp", "experience", "badass"],
        not: &["name", "text"],
        note: "Characters above the game's level cap, or with impossible XP, can fail to load or be rejected in online co-op.",
    },
    Rule {
        kind: "",
        keys: &["playtime", "play_time", "timeplayed", "time_played", "playedtime", "totaltime", "total_time"],
        not: &[],
        note: "A play-time counter the game uses for display and some achievements. Odd values can confuse its save list or achievement checks.",
    },
    Rule {
        kind: "",
        keys: &["unlock", "achievement", "trophy"],
        not: &["name", "text", "title", "desc", "icon", "count", "level", "required", "req"],
        note: "Unlocking everything at once can skip content you later find missing, and some games tie platform achievements to these flags.",
    },
];

/// The risk note for the value at `ptr` in a save of `kind`, if one applies.
pub fn note(kind: &str, ptr: &str) -> Option<&'static str> {
    let key = ptr.rsplit('/').next().unwrap_or("").to_lowercase();
    if key.is_empty() || key.chars().all(|c| c.is_ascii_digit()) {
        // An array element: judge it by the list it is in.
        let parent = ptr.rsplit('/').nth(1)?.to_lowercase();
        return match_rules(kind, &parent);
    }
    match_rules(kind, &key)
}

fn match_rules(kind: &str, key: &str) -> Option<&'static str> {
    let kind = kind.to_lowercase();
    let key: String = key.chars().filter(|c| !matches!(c, ' ' | '-')).collect();
    // Rules for one game first, so they win over the general ones.
    let specific = RULES.iter().filter(|r| !r.kind.is_empty());
    let general = RULES.iter().filter(|r| r.kind.is_empty());
    specific.chain(general).find_map(|r| {
        let kind_ok = r.kind.is_empty() || kind.contains(r.kind);
        (kind_ok && r.keys.iter().any(|k| key == *k || key.contains(k)) && !r.not.iter().any(|n| key.contains(n))).then_some(r.note)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_match_by_key_and_kind() {
        assert!(note("JSON", "/player/gold").unwrap().contains("overflow"));
        assert!(note("JSON", "/player/Level").unwrap().contains("level"));
        assert!(note("JSON", "/player/name").is_none());
        assert!(note("JSON", "/player/goldMax").is_none(), "limits aren't amounts");
        assert!(note("Elden Ring character", "/stats/vigor").unwrap().contains("banned"));
        assert!(note("JSON", "/stats/vigor").is_none(), "the ban note is for the From Software games");
        assert!(note("JSON", "/SteamID").unwrap().contains("account"));
        assert!(note("JSON", "/items/3").is_none());
        assert!(note("JSON", "/gold/0").is_some(), "list elements use their list's name");
    }
}
