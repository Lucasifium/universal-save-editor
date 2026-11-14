//! Dragon Ball Z: Budokai Tenkaichi 2 / Sparking! NEO (PlayStation 2) saves: the 16,384-byte save
//! file of the save folder. A 64-bit checksum at the start covers the 16-bit words from byte 8 to
//! 0x4000. Offsets and names follow the MIT-licensed save editor in RyudoSynbios/game-tools-collection
//! (see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum, Kind};
use super::retro::flag;
use crate::tree::TreeDoc;

pub const KIND: &str = "Dragon Ball Z: Budokai Tenkaichi 2 (PlayStation 2) save";

const LEN: usize = 0x4000;
const ITEMS: usize = 0x1f30;

fn sum(d: &[u8]) -> u64 {
    let s: u32 = d[8..LEN].chunks(2).map(|w| u16::from_le_bytes([w[0], w[1]]) as u32).sum();
    ((s & 0xff) as u64) << 32 | (s >> 8 & 0xff) as u64
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == LEN && u64::from_le_bytes(d[..8].try_into().unwrap()) == sum(d) && d[0x283c + 3] == 0 && d[8..].iter().any(|b| *b != 0)
}

struct Item {
    index: usize,
    kind: usize,
    name: String,
    unique: bool,
}

fn items() -> Vec<Item> {
    include_str!("data/dbz_zitems.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Some(Item { index: f[0].parse().ok()?, kind: f[1].parse().ok()?, name: f[2].to_string(), unique: f[3] == "1" })
        })
        .collect()
}

/// Characters and forms the player can unlock are the "secret" items 0x1ae..=0x22e.
fn is_character(i: &Item) -> bool {
    i.kind == 3 && (0x1ae..=0x22e).contains(&i.index)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Budokai Tenkaichi 2 save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["General", "Zeni"], 0x283c, Kind::U32, 0, 9_999_999);
    let list = items();
    let mut seen = std::collections::HashSet::new();
    for it in &list {
        if is_character(it) {
            let name = if seen.insert(format!("c{}", it.name)) { it.name.clone() } else { format!("{} ({})", it.name, it.index) };
            flag(&mut b, &["Unlocked characters", &name], ITEMS + it.index * 4, 0);
        }
    }
    for (kind, title) in [(0, "Ability type"), (1, "Support type"), (2, "Fusion type"), (3, "Secret type"), (4, "Dragon Balls")] {
        let mut seen = std::collections::HashSet::new();
        for it in list.iter().filter(|i| i.kind == kind && !is_character(i)) {
            let name = if seen.insert(it.name.clone()) { it.name.clone() } else { format!("{} ({})", it.name, it.index) };
            b.ranged(&["Z-items", title, &name], ITEMS + 2 + it.index * 4, Kind::U16, 0, if it.unique { 1 } else { 999 });
        }
    }
    Ok(b.finish(KIND, Checksum::Touched(fix)).with_note(crate::i18n::tx("Close the emulator first. Each Z-item and each unlocked character also keeps an obtained flag, written again on save, and the save's checksum is written again.")))
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    let n = (d.len() - ITEMS) / 4;
    for t in touched {
        if *t < ITEMS || *t >= ITEMS + n * 4 {
            continue;
        }
        let i = (*t - ITEMS) / 4;
        let at = ITEMS + i * 4;
        if (0x1ae..=0x22e).contains(&i) && *t == at {
            // an unlocked character: the count is 1 exactly when the flag is set
            let on = d[at] & 1;
            d[at + 2] = on;
            d[at + 3] = 0;
        } else if *t >= at + 2 {
            let q = u16::from_le_bytes([d[at + 2], d[at + 3]]);
            if q > 0 {
                d[at] |= 1;
            }
        }
    }
    let s = sum(d);
    d[..8].copy_from_slice(&s.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; LEN];
        d[0x2000] = 1;
        let s = sum(&d);
        d[..8].copy_from_slice(&s.to_le_bytes());
        d
    }

    #[test]
    fn writes_items_flags_and_checksum() {
        let d = sample();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/General/Zeni", serde_json::json!(5000)).unwrap();
        doc.set("/Z-items/Ability type/Health +1", serde_json::json!(7)).unwrap();
        doc.set("/Unlocked characters/Goku", serde_json::json!("Yes")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32::from_le_bytes(out[0x283c..0x2840].try_into().unwrap()), 5000);
        assert_eq!(out[ITEMS + 2], 7);
        assert_eq!(out[ITEMS] & 1, 1);
        let g = ITEMS + 0x1ae * 4;
        assert_eq!((out[g] & 1, out[g + 2]), (1, 1));
        assert!(looks(&out));
    }
}
