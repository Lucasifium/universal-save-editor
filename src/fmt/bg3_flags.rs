//! Baldur's Gate 3 story flags.
//!
//! The game keeps every global flag it has touched in `Globals.lsf`, under
//! `OsirisVariableHelper/VariableManager/FlagMap/FlagContainerMap`: one `Object` per flag with
//! the flag's GUID (`MapKey`) and a `FlagContainerMap` child whose `Flags` byte is 1 (set) or 0
//! (cleared). The story scripts keep their own list of the set flags, the Osiris database
//! `DB_GlobalFlag` in `StorySave.bin`, one fact per flag as "Name_GUID". That list is also
//! where the flags get their names, so a flag named only there drops out of the list once it
//! is cleared and the save is opened again.
//!
//! An edit flips the byte in `Globals.lsf`; when the package is written, `DB_GlobalFlag` is
//! brought in step: a cleared flag's fact is taken out and a set flag gets a fact (copied from
//! an existing one, with the new name). Checked on public sample saves: every flag in
//! `DB_GlobalFlag` is set in the flag map, and no cleared flag is in it.

use super::fields::{Builder, Kind};
use super::lsf::Lsf;
use super::osiris::{self, Val};
use std::collections::{HashMap, HashSet};

pub const FLAG_MAP: [&str; 5] = ["OsirisVariableHelper", "VariableManager", "FlagMap", "FlagContainerMap", "Object"];

/// Each flag in the flag map: GUID text and where its on/off byte is.
pub fn flag_map(l: &Lsf, plain: &[u8]) -> Vec<(String, usize)> {
    let mut out = vec![];
    for o in l.at_path(&FLAG_MAP) {
        let Some(k) = l.attr(o, "MapKey").filter(|a| a.len == 16 && a.at + 16 <= plain.len()) else { continue };
        let Some(c) = l.child(o, "FlagContainerMap") else { continue };
        let Some(f) = l.attr(c, "Flags").filter(|a| a.ty == 19 && a.len == 1) else { continue };
        out.push((super::larian::guid_text(&plain[k.at..k.at + 16]), f.at));
    }
    out
}

/// "Name_GUID" by GUID, from every string in the story's databases.
pub fn names(story: &osiris::Story) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for d in &story.databases {
        for f in &d.facts {
            for v in &f.values {
                if let Val::Str(s) = v {
                    if let Some(g) = osiris::guid_of(s) {
                        out.entry(g.to_ascii_lowercase()).or_insert_with(|| s.clone());
                    }
                }
            }
        }
    }
    out
}

/// The companions, as the flag names spell them (lower case) and as the screen names them.
const COMPANIONS: [(&str, &str); 11] = [
    ("shadowheart", "Shadowheart"),
    ("astarion", "Astarion"),
    ("gale", "Gale"),
    ("laezel", "Lae'zel"),
    ("wyll", "Wyll"),
    ("karlach", "Karlach"),
    ("minthara", "Minthara"),
    ("halsin", "Halsin"),
    ("jaheira", "Jaheira"),
    ("minsc", "Minsc"),
    ("darkurge", "The Dark Urge"),
];

/// The companion a name part is about ("ShadowHeart", "LaezelRecruitment", "GALECAMP").
fn companion(part: &str) -> Option<&'static str> {
    let low = part.to_ascii_lowercase();
    COMPANIONS.iter().find(|(k, _)| low.starts_with(k)).map(|c| c.1)
}

/// Where a flag is listed: its area or companion, from the name's prefix. Companion flags
/// read "Companion: <name>", unknown prefixes "Other (<prefix>)"; the rest are fixed English
/// keys the screen translates.
pub fn section(name: &str) -> String {
    let mut parts = name.split('_');
    let first = parts.next().unwrap_or_default();
    if first == "ORI" {
        let who = parts.next().unwrap_or_default();
        return match companion(who) {
            Some(c) => format!("Companion: {c}"),
            None => "Companions".to_string(),
        };
    }
    let area = match first {
        "GLO" | "GEN" => "General",
        "CAMP" => "Camp",
        "NIGHT" => "Camp nights",
        "TUT" => "Nautiloid",
        "CRA" => "Nautiloid crash site",
        "CHA" => "Chapel ruins",
        "FOR" => "Forest and Blighted Village",
        "PLA" => "Risen Road",
        "HAG" => "Riverside Teahouse",
        "WLD" => "Wilderness",
        "DEN" => "Emerald Grove",
        "GOB" => "Goblin camp",
        "UND" => "Underdark",
        "CRE" => "Githyanki creche",
        "SCL" => "Shadow-Cursed Lands",
        "HAV" => "Last Light Inn",
        "MOO" => "Moonrise Towers",
        "LOW" => "Lower City",
        "CTY" => "Baldur's Gate",
        "WYR" => "Wyrm's Crossing",
        "BANTERREGION" | "CURRENTREGION" | "VISITEDREGION" => "Regions",
        "Hirelings" => "Hirelings",
        f if f.starts_with("Debug") || f == "DBG" => "Debug switches",
        f => match companion(f) {
            // "ShadowHeart_InParty_…", "Wyll_Recruitment_…" and the bare tags ("GALECAMP").
            Some(c) => return format!("Companion: {c}"),
            None => return format!("Other ({f})"),
        },
    };
    area.to_string()
}

/// Why changing a flag is riskier than most (see `risk`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    /// A developer's testing switch.
    Debug,
    /// Set and cleared by a script while a scene runs.
    Scene,
    /// The game mode (difficulty, early access), which is kept elsewhere too.
    Mode,
    /// Someone lives or dies, or can be brought back.
    Life,
    /// Who is in the party or at camp, which is kept elsewhere too.
    Party,
    /// Romance and relationship state.
    Romance,
    /// Story progress: an area left, the tutorial over, an act begun.
    Progress,
    /// The tadpole and mind flayer story.
    Tadpole,
}

/// Flags that are riskier to change than most, by their name. Names the scripts use for
/// knowledge ("…_Knows_…") and the like aren't marked; any flag can still have effects the
/// name doesn't show.
pub fn risk(name: &str) -> Option<Risk> {
    let low = name.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| low.contains(w));
    let first = name.split('_').next().unwrap_or_default();
    if first.starts_with("Debug") || first == "DBG" {
        return Some(Risk::Debug);
    }
    if has(&["startspot", "stopspot", "_spotting", "inconversation"]) {
        return Some(Risk::Scene);
    }
    if has(&["difficultymode", "not_for_ea"]) {
        return Some(Risk::Mode);
    }
    if has(&["dead", "death", "died", "killed", "_kill", "alive", "permadefeated", "resurrect"]) {
        return Some(Risk::Life);
    }
    let tag = !name.contains('_') && name.chars().all(|c| c.is_ascii_uppercase()) && companion(name).is_some();
    if tag || has(&["partoftheteam", "inparty", "wasrecruited", "canbekicked", "recruited", "hired", "incamp", "partysize"]) {
        return Some(Risk::Party);
    }
    if has(&["dating", "romance", "partner", "flirt", "kiss"]) {
        return Some(Risk::Romance);
    }
    if has(&["partyprogress", "tutorialended", "_act1", "_act2", "_act3", "_ending", "endgame", "gameover"]) {
        return Some(Risk::Progress);
    }
    if has(&["tadpole", "ceremorphosis", "illithid", "mindflayerpower"]) {
        return Some(Risk::Tadpole);
    }
    None
}

/// Adds the named flags to the view as on/off values under "Story flags/<section>/<name>".
pub fn add(b: &mut Builder, l: &Lsf, plain: &[u8], base: usize, story: &osiris::Story) -> usize {
    let names = names(story);
    let mut n = 0;
    let mut seen = HashSet::new();
    let mut unnamed = 0u64;
    for (g, at) in flag_map(l, plain) {
        let Some(full) = names.get(&g) else {
            unnamed += 1;
            continue;
        };
        let name = osiris::strip_guid(full).to_string();
        let key = if seen.insert(name.clone()) { name.clone() } else { format!("{name} ({})", &g[..8]) };
        if b.bits(&["Story flags", &section(&name), &key], base + at, Kind::U8, 0, 1, 0, 0, 1).is_some() {
            n += 1;
        }
    }
    if n > 0 {
        b.info(&["Story flags unnamed"], unnamed);
    }
    n
}

/// The story with `DB_GlobalFlag` brought in step with the flag map edits between `old` and
/// `new` (the unpacked `Globals.lsf` before and after). `None` when nothing needs changing.
pub fn sync_story(old: &[u8], new: &[u8], story_bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let (Some(lo), Some(ln)) = (Lsf::read(old), Lsf::read(new)) else { return Ok(None) };
    let before: HashMap<String, usize> = flag_map(&lo, old).into_iter().collect();
    let mut set = vec![];
    let mut cleared = HashSet::new();
    for (g, at) in flag_map(&ln, new) {
        let Some(&was_at) = before.get(&g) else { continue };
        let (was, now) = (old.get(was_at).copied().unwrap_or(0) != 0, new.get(at).copied().unwrap_or(0) != 0);
        if was != now {
            if now {
                set.push(g);
            } else {
                cleared.insert(g);
            }
        }
    }
    if set.is_empty() && cleared.is_empty() {
        return Ok(None);
    }
    let story = osiris::read(story_bytes).ok_or("the story file doesn't read")?;
    let db = story.db("DB_GlobalFlag").ok_or("the story has no global flag list")?;
    let names = names(&story);
    let have: HashSet<String> = db.facts.iter().filter_map(|f| match f.values.first() {
        Some(Val::Str(s)) => osiris::guid_of(s).map(str::to_ascii_lowercase),
        _ => None,
    }).collect();
    // A fact to copy: column count 1, the value's header ending in has-value 1, then the
    // scrambled name ending in 0xAD. Only the name is replaced.
    let template = db.facts.iter().find_map(|f| {
        let Some(Val::Str(s)) = f.values.first() else { return None };
        let b = story_bytes.get(f.start..f.end)?;
        let head = b.len().checked_sub(s.len() + 1)?;
        let ok = f.values.len() == 1 && s.is_ascii() && b[0] == 1 && head >= 2 && b[head - 1] == 1 && b[..head].contains(&b'0') && b[b.len() - 1] == 0xAD;
        ok.then(|| b[..head].to_vec())
    }).ok_or("no flag in the story's list to copy the layout from")?;
    let mut drop: Vec<(usize, usize)> = db.facts.iter().filter(|f| match f.values.first() {
        Some(Val::Str(s)) => osiris::guid_of(s).is_some_and(|g| cleared.contains(&g.to_ascii_lowercase())),
        _ => false,
    }).map(|f| (f.start, f.end)).collect();
    let mut add = vec![];
    for g in &set {
        if have.contains(g) {
            continue;
        }
        let full = names.get(g).ok_or_else(|| format!("flag {g} has no name in the story"))?;
        add.extend(&template);
        add.extend(full.bytes().map(|c| c ^ 0xAD));
        add.push(0xAD);
    }
    let added = set.iter().filter(|g| !have.contains(*g)).count();
    let count = db.facts.len() - drop.len() + added;
    let insert_at = db.facts.last().map_or(db.count_at + 4, |f| f.end);
    drop.sort();
    let mut out = Vec::with_capacity(story_bytes.len() + add.len());
    let mut p = 0;
    for (s, e) in &drop {
        out.extend(&story_bytes[p..*s]);
        p = *e;
        if p > insert_at {
            return Err("unexpected story layout".into());
        }
    }
    out.extend(&story_bytes[p..insert_at]);
    out.extend(&add);
    out.extend(&story_bytes[insert_at..]);
    // The count sits before every fact, so nothing before it moved.
    out[db.count_at..db.count_at + 4].copy_from_slice(&(count as u32).to_le_bytes());
    // Read back: the list must hold exactly the expected flags.
    let check = osiris::read(&out).ok_or("the edited story doesn't read back")?;
    let got = check.db("DB_GlobalFlag").map_or(0, |d| d.facts.len());
    if got != count || check.databases.len() != story.databases.len() {
        return Err("the edited story doesn't read back as expected".into());
    }
    Ok(Some(out))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_by_area_and_companion() {
        assert_eq!(section("DEN_GroveConflict_Quest_AcceptedKagha"), "Emerald Grove");
        assert_eq!(section("ORI_LaezelRecruitment_Event_InvitedToCamp"), "Companion: Lae'zel");
        assert_eq!(section("ORI_ShadowHeart_HasMet"), "Companion: Shadowheart");
        assert_eq!(section("ShadowHeart_InParty_Knows_SharWorshipper"), "Companion: Shadowheart");
        assert_eq!(section("GALECAMP"), "Companion: Gale");
        assert_eq!(section("ORI_Banter_State_AllowLaezelCreche"), "Companions");
        assert_eq!(section("Debug_CAMP_IgnoreCampTypeRestrictions"), "Debug switches");
        assert_eq!(section("BANTERREGION_WLD_Forest"), "Regions");
        assert_eq!(section("JERGAL"), "Other (JERGAL)");
    }

    #[test]
    fn risky_flags_are_marked() {
        assert_eq!(risk("Debug_CAMP_IgnoreCampTypeRestrictions"), Some(Risk::Debug));
        assert_eq!(risk("GOB_GoblinToast_Event_StartSpotting"), Some(Risk::Scene));
        assert_eq!(risk("GLO_DifficultyMode_Medium"), Some(Risk::Mode));
        assert_eq!(risk("HAG_Hag_Brother_001_State_IsDead"), Some(Risk::Life));
        assert_eq!(risk("ORI_Wyll_State_Alive"), Some(Risk::Life));
        assert_eq!(risk("GLO_Origin_PartOfTheTeam_Halsin"), Some(Risk::Party));
        assert_eq!(risk("ORI_Laezel_State_IsInParty"), Some(Risk::Party));
        assert_eq!(risk("WYLLPARTY"), Some(Risk::Party));
        assert_eq!(risk("ORI_Gale_State_NotDating"), Some(Risk::Romance));
        assert_eq!(risk("CHA_PartyProgress_LeavingCrashSite"), Some(Risk::Progress));
        assert_eq!(risk("NIGHT_Ceremorphosis1"), Some(Risk::Tadpole));
        // Knowledge and the like aren't marked.
        assert_eq!(risk("ORI_Astarion_Knows_Name"), None);
        assert_eq!(risk("DEN_DruidLair_Knows_HiddenLibraryEntrance"), None);
        assert_eq!(risk("GOB_VoloBallad_State_OnStage"), None);
        assert_eq!(risk("JERGAL"), None);
    }
}
