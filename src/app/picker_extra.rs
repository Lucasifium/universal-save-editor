//! The extra ways into the "Find my game's saves" list: a short list of well-known games shown
//! above the A to Z groups, and a genre filter (the genres the website groups games by).

use crate::i18n::tr;

/// Well-known games shown under "Popular" (catalog names, as in `games::GAMES`).
pub const POPULAR: &[&str] = &[
    "Elden Ring",
    "Baldur's Gate 3",
    "Cyberpunk 2077",
    "The Witcher 3",
    "Skyrim (Special / Anniversary Edition)",
    "Fallout 4",
    "Stardew Valley",
    "Terraria (characters)",
    "Minecraft (Java Edition)",
    "Hollow Knight",
    "Dark Souls III",
    "Sekiro: Shadows Die Twice",
    "Red Dead Redemption 2",
    "Palworld",
    "Hades",
    "Monster Hunter Wilds",
    "Persona 5 Royal",
    "Black Myth: Wukong",
    "Clair Obscur: Expedition 33",
    "The Sims 4",
];

/// The genres, by key (as in `genres::GAME_GENRES`), in the order the list offers them.
pub const GENRES: &[&str] = &["rpg", "soulslike", "action", "shooter", "roguelike", "survival", "simulation", "strategy"];

/// A genre's name in the interface language.
pub fn genre_label(key: &str) -> &'static str {
    match key {
        "rpg" => tr("RPG"),
        "soulslike" => tr("Soulslike and Metroidvania"),
        "action" => tr("Action and adventure"),
        "shooter" => tr("Shooter"),
        "roguelike" => tr("Roguelike, roguelite and idle"),
        "survival" => tr("Survival and crafting"),
        "simulation" => tr("Simulation, farming and management"),
        "strategy" => tr("Strategy"),
        _ => tr("Any genre"),
    }
}

/// The genre key of a game (catalog name), when it has one.
/// The table is built once, the first time it is asked: the list asks for every game on
/// every frame while a genre is picked.
pub fn genre_of(name: &str) -> Option<&'static str> {
    static BY_NAME: std::sync::OnceLock<std::collections::HashMap<&'static str, &'static str>> = std::sync::OnceLock::new();
    BY_NAME.get_or_init(|| crate::genres::GAME_GENRES.iter().copied().collect()).get(name).copied()
}

/// The catalog index of each popular game.
pub fn popular_indices() -> Vec<usize> {
    POPULAR.iter().filter_map(|n| crate::games::GAMES.iter().position(|g| g.name == *n)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_popular_game_is_in_the_catalog() {
        for n in POPULAR {
            assert!(crate::games::GAMES.iter().any(|g| g.name == *n), "{n} is not in games::GAMES");
        }
        assert_eq!(popular_indices().len(), POPULAR.len());
        assert!((15..=25).contains(&POPULAR.len()));
    }

    #[test]
    fn every_genre_has_a_name_and_games() {
        for g in GENRES {
            assert_ne!(genre_label(g), tr("Any genre"), "{g}");
            assert!(crate::genres::GAME_GENRES.iter().any(|(_, k)| k == g), "{g} has no games");
        }
        for (_, k) in crate::genres::GAME_GENRES {
            assert!(GENRES.contains(k), "genre {k} is not offered");
        }
    }

    #[test]
    fn the_genre_table_gives_each_game_its_one_genre() {
        let mut seen = std::collections::HashSet::new();
        for (n, k) in crate::genres::GAME_GENRES {
            assert!(seen.insert(*n), "{n} is listed twice");
            assert_eq!(genre_of(n), Some(*k));
        }
        assert_eq!(genre_of("Not a game at all"), None);
    }
}
