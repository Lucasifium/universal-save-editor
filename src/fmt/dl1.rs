//! Dying Light (2015), after the gzip layer. The skill trees are stored by name (u16
//! length + name) followed by their level, twice in the save summaries (32-bit) and once
//! in the player's data (16-bit, after a 16-bit marker). All copies are written together.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const TREES: [(&str, &str); 6] = [
    ("Runner", "Agility"),
    ("Fighter", "Power"),
    ("Status", "Survivor"),
    ("Reputation", "Reputation"),
    ("Legend", "Legend"),
    ("Driver", "Driver"),
];

fn find_all(d: &[u8], pat: &[u8]) -> Vec<usize> {
    d.windows(pat.len())
        .enumerate()
        .filter(|(_, w)| *w == pat)
        .map(|(i, _)| i + pat.len())
        .collect()
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 1024
        && d.starts_with(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0])
        && d[..1024].windows(8).any(|w| w == b"\x06\x00Runner")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Dying Light save".into());
    }
    let mut b = Builder::new(d);
    let mut any = false;
    for (key, label) in TREES {
        let mut pat = (key.len() as u16).to_le_bytes().to_vec();
        pat.extend(key.as_bytes());
        let hits = find_all(d, &pat);
        // Summaries: the value right after the name; player data: a 16-bit marker (1), then it.
        let summary: Vec<usize> = hits
            .iter()
            .copied()
            .filter(|&e| d.get(e + 2..e + 4) == Some(&[0, 0]))
            .collect();
        let player: Vec<usize> = hits
            .iter()
            .copied()
            .filter(|&e| d.get(e..e + 2) == Some(&[1, 0]))
            .map(|e| e + 2)
            .collect();
        // The summaries' 32-bit copies have zero top halves, so one 16-bit value covers all.
        let all: Vec<usize> = summary.into_iter().chain(player).collect();
        let Some(&first) = all.first() else { continue };
        b.ranged(&["Skill trees", label], first, Kind::U16, 0, 99);
        for &at in &all[1..] {
            b.mirror(&["Skill trees", label], at);
        }
        any = true;
    }
    if !any {
        return Err("no skill trees found".into());
    }
    Ok(b.finish("Dying Light save (2015)", Checksum::None).with_note(
        "Skill tree levels (every copy the save keeps is changed together). The game may recalculate a level \
         from the points earned in that tree.",
    ))
}
