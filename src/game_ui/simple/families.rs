//! Game series whose screens share one colour, so the games of one series look like one
//! family on their header cards and tiles (see `game_ui::accent_of`). The names here are
//! matched against the screens' names, never shown.

use crate::game_ui::Game;
use eframe::egui::Color32;

/// Series whose screens share one colour: the games of one series look like one family.
/// Table-driven screens join by name; the colour is that of the series' first screen in the
/// table, unless a hand-made screen of the series sets it below.
const FAMILIES: &[(&[&str], Option<(u8, u8, u8)>)] = &[
    (&["Final Fantasy"], Some((60, 110, 180))),
    (&["Yakuza", "Like a Dragon"], Some((200, 40, 50))),
    (&["Assassin's Creed"], None),
    (&["Kingdom Hearts"], None),
    (&["The Legend of Zelda", "Oracle of Ages", "Oracle of Seasons"], None),
    (&["Grand Theft Auto"], None),
    (&["Kingdom Rush", "Legends of Kingdom Rush"], None),
    (&["Warriors Orochi", "Dynasty Warriors", "Samurai Warriors", "Warriors All-Stars"], None),
    (&["Resident Evil"], None),
    (&["Red Dead Redemption"], None),
    (&["Tekken"], None),
    (&["Farming Simulator"], None),
    (&["Car Mechanic Simulator"], None),
    (&["Grandia"], None),
    (&["Atelier"], None),
    (&["Mortal Shell"], None),
    (&["Sons of the Forest"], None),
    (&["Schedule I"], None),
    (&["Tales of"], None),
    (&["The Legend of Heroes", "Trails in the Sky"], None),
    (&["Dishonored"], None),
    (&["Horizon Zero Dawn", "Horizon Forbidden West"], None),
    (&["Attack on Titan"], None),
    (&["Crash Bandicoot", "Crash Team Racing"], None),
    (&["Banjo-"], None),
    (&["Saints Row"], None),
    (&["Deep Rock Galactic"], None),
    (&["Super Mario", "Mario ", "Paper Mario"], None),
    (&["Kirby", "Super Kirby", "Team Kirby"], None),
    (&["My Hero One's Justice"], None),
    (&["The Elder Scrolls"], None),
    (&["Shape of Dreams"], None),
];

/// The series' colour for `game`, if it belongs to a series with more than one screen.
pub fn family_accent(game: Game) -> Option<Color32> {
    let rgb = |(r, g, b): (u8, u8, u8)| Color32::from_rgb(r, g, b);
    let fixed = match game {
        Game::EldenRing | Game::DarkSouls3 | Game::DarkSouls2 | Game::DarkSouls1 | Game::Sekiro => return Some(rgb((201, 168, 92))),
        Game::Ffx | Game::Ffx2 | Game::Ff7 | Game::Ff7Remake | Game::Ff9 | Game::Ffxv | Game::Ff16 | Game::FfResonance => Some("Final Fantasy"),
        Game::Yakuza0 | Game::YakuzaDe | Game::Kiwami => Some("Yakuza"),
        Game::P4g | Game::P5r => return Some(rgb((230, 190, 30))),
        Game::DyingLight | Game::DyingLight2 => return Some(rgb((190, 100, 40))),
        Game::Gow2018 | Game::GowRagnarok => return Some(rgb((170, 40, 30))),
        Game::Xcom | Game::Xcom2 => return Some(rgb((60, 120, 200))),
        Game::MassEffect | Game::Andromeda => return Some(rgb((200, 60, 50))),
        _ => None,
    };
    let name = match (fixed, game) {
        (Some(n), _) => n,
        (None, Game::Simple(i)) => super::game_name(i),
        _ => return None,
    };
    let (prefixes, colour) = FAMILIES.iter().find(|(ps, _)| ps.iter().any(|p| name.starts_with(p)))?;
    if let Some(c) = colour {
        return Some(rgb(*c));
    }
    // The first table-driven screen of the series, when there are at least two.
    let members: Vec<(u8, u8, u8)> = super::all().iter().filter(|s| prefixes.iter().any(|p| s.name.starts_with(p))).map(|s| s.accent).collect();
    (members.len() > 1).then(|| rgb(members[0]))
}


#[cfg(test)]
mod tests {
    use super::*;

    fn spec(name: &str) -> Game {
        Game::Simple(super::super::all().iter().position(|s| s.name == name).unwrap_or_else(|| panic!("{name}")) as u16)
    }

    #[test]
    fn games_of_one_series_share_a_colour() {
        assert_eq!(family_accent(Game::DarkSouls1), family_accent(Game::EldenRing));
        assert_eq!(family_accent(Game::Ffx), family_accent(spec("Final Fantasy VIII")));
        assert_eq!(family_accent(Game::Yakuza0), family_accent(spec("Yakuza 3 Remastered")));
        assert_eq!(family_accent(Game::Yakuza0), family_accent(spec("Like a Dragon: Ishin!")));
        assert_eq!(family_accent(spec("Assassin's Creed Unity")), family_accent(spec("Assassin's Creed Rogue")));
        assert!(family_accent(spec("Assassin's Creed Unity")).is_some());
        assert_eq!(family_accent(spec("Celeste")), None);
        // "Horizon Chase Turbo" is not Horizon Zero Dawn's series.
        assert_eq!(family_accent(spec("Horizon Chase Turbo")), None);
        assert_eq!(family_accent(Game::Terraria), None);
    }

    /// A series listed here has at least two screens, or it does nothing.
    #[test]
    fn every_series_has_two_screens() {
        let names: Vec<&str> = super::super::all().iter().map(|s| s.name).collect();
        for (prefixes, colour) in FAMILIES {
            let n = names.iter().filter(|n| prefixes.iter().any(|p| n.starts_with(p))).count();
            assert!(n >= 2 || colour.is_some(), "{prefixes:?} matches {n} screen(s)");
        }
    }
}
