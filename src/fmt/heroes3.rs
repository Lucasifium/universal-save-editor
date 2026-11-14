//! Heroes of Might and Magic III saves (`.GM1` single scenario, `.CGM` campaign, `.GM2` hotseat;
//! Shadow of Death / Armageddon's Blade and Horn of the Abyss). The game writes a gzip stream
//! whose checksum is wrong, which the container layer reads anyway and writes back correctly.
//! Inside, the save is a long dump with no index, so the player records (resources), the hero
//! records and their armies are found by their shape, as the MIT-licensed h3sed does, and
//! values are edited in place: nothing moves and the length never changes.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Heroes of Might and Magic III save";

include!("heroes3_data.rs");

const COLOURS: [&str; 8] = ["Red", "Blue", "Tan", "Green", "Orange", "Purple", "Teal", "Pink"];
const SKILLS: [&str; 30] = [
    "Pathfinding", "Archery", "Logistics", "Scouting", "Diplomacy", "Navigation", "Leadership", "Wisdom", "Mysticism", "Luck",
    "Ballistics", "Eagle Eye", "Necromancy", "Estates", "Fire Magic", "Air Magic", "Water Magic", "Earth Magic", "Scholar",
    "Tactics", "Artillery", "Learning", "Offense", "Armorer", "Intelligence", "Sorcery", "Resistance", "First Aid",
    "Interference", "Runes",
];

/// The game's gzip streams carry a wrong checksum; read them anyway when the length field is
/// right and the content is a Heroes III save (it is written back with a correct checksum).
pub fn gunzip_lax(d: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    if d.len() < 30 || d[..3] != [0x1F, 0x8B, 8] || d[3] != 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut dec = flate2::bufread::DeflateDecoder::new(&d[10..]);
    dec.read_to_end(&mut out).ok()?;
    let rest = dec.into_inner();
    (rest.len() == 8 && rest[4..] == (out.len() as u32).to_le_bytes() && out.starts_with(b"H3SV")).then_some(out)
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 20_000 && d.starts_with(b"H3SV") && matches!(d[4], b'G' | b'C')
}

/// Horn of the Abyss writes major version 44 and up; 1.8.0 (minor 10) added the Runes skill.
fn is_hota(d: &[u8]) -> bool {
    d[8] >= 44
}

struct Layout {
    /// Bytes from the start of a hero record to its end.
    len: usize,
    skills: usize,
    faction_len: usize,
}

fn layout(d: &[u8]) -> Option<Layout> {
    if is_hota(d) {
        (d[12] >= 10).then_some(Layout { len: 1122, skills: 30, faction_len: 149 })
    } else {
        (42..=43).contains(&d[8]).then_some(Layout { len: 1056, skills: 28, faction_len: 145 })
    }
}

/// Does a hero record start at `s`? The checks mirror the shape of the record, so that only
/// the real ones match.
fn hero_at(d: &[u8], s: usize, lay: &Layout, hota: bool) -> bool {
    let Some(r) = d.get(s..s + lay.len) else { return false };
    if r[0] > 7 && r[0] != 255 {
        return false;
    }
    if r[43] > 0x1C || r[44] != 0 || r[45] != 0 || r[46] != 0 {
        return false;
    }
    if r[169] <= 0x20 || r[181] != 0 {
        return false;
    }
    if r[182..182 + lay.skills].iter().any(|&b| b > 3) {
        return false;
    }
    if !hota && r[210..238].iter().any(|&b| b > 0x1C) {
        return false;
    }
    if r[242..382].iter().any(|&b| b > 1) {
        return false;
    }
    let eq = &r[382..534];
    if eq.iter().all(|&b| b == 0) || eq.chunks(8).all(|c| c[..4] == [0, 0, 0, 0] && c[4..] == [255; 4]) {
        return false;
    }
    for c in eq.chunks(8) {
        let blank = c[..4] == [255; 4];
        let item = c[1..4] == [0, 0, 0];
        if !(blank || item && (c[4..] == [0; 4] || c[4..] == [255; 4] || c[6..] == [0, 0])) {
            return false;
        }
    }
    for c in r[534..1046].chunks(4) {
        if !(c == [255; 4] || c[1..] == [0, 0, 0]) {
            return false;
        }
    }
    if hota {
        r[1092..1122].iter().all(|&b| b <= 0x1C)
    } else {
        r[1047..1053].iter().all(|&b| b <= 1) && r[1053] <= 2 && r[1054] <= 1 && r[1055] <= 5
    }
}

fn is_town(d: &[u8], p: usize, lay: &Layout) -> bool {
    let Some(t) = d.get(p..p + 90) else { return false };
    if !(t[0] <= 7 || t[0] == 255) || t[3] > if is_hota(d) { 11 } else { 8 } || t[4] > 252 || t[5] > 252 || t[6] > 1 {
        return false;
    }
    let _ = lay;
    let names = &t[9..37];
    if names.chunks(4).any(|c| c[1..].iter().any(|&x| x != 0 && x != 255 && x != b',')) {
        return false;
    }
    let blank_ids = names.iter().all(|&b| b == 255);
    let blank_counts = t[37..65].iter().all(|&b| b == 0 || b == 255);
    if blank_ids != blank_counts {
        return false;
    }
    let nl = t[69] as usize;
    if t[69] == 0 || t[70] != 0 || nl > 14 {
        return false;
    }
    let Some(name) = d.get(p + 71..p + 71 + nl) else { return false };
    name[0] > 0x20 && name[0] != 255 && name.iter().all(|&b| b >= 0x20 && b != 255)
}

fn faction_ok(d: &[u8], b: usize, len: usize, heroes: usize) -> bool {
    let Some(r) = d.get(b..b + len) else { return false };
    let at = |i: usize| i < r.len();
    if !at(140) || (0..6).any(|i| r[99 + 4 * i] != 0) || r[140] > 1 {
        return false;
    }
    let mut seen = false;
    for &x in &r[1..9] {
        if x == 255 {
            seen = true;
        } else if seen || x as usize >= heroes.max(1) {
            return false;
        }
    }
    r[9..11].iter().all(|&x| x == 255 || (x as usize) < heroes.max(1))
}

fn creature_options(d: &[u8]) -> Vec<(String, u64)> {
    let extra: &[(u32, &str)] = if is_hota(d) { HOTA_CREATURES } else { &[] };
    CREATURES.iter().chain(extra).map(|(i, n)| (n.to_string(), u64::from(*i))).collect()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Heroes III save".into());
    }
    let lay = layout(d).ok_or("this Heroes III version isn't supported (Shadow of Death, Armageddon's Blade and Horn of the Abyss 1.8 or newer are)")?;
    let hota = is_hota(d);
    // The hero records, one after another (a biography may sit in front of each).
    let mut heroes: Vec<usize> = Vec::new();
    let mut pos = 30_000usize;
    while pos + lay.len <= d.len() {
        if hero_at(d, pos, &lay, hota) {
            heroes.push(pos);
            pos += lay.len;
        } else {
            pos += 1;
            // Once the heroes section is reached the next record starts soon after the last.
            if !heroes.is_empty() && pos > heroes.last().unwrap() + lay.len + 5000 {
                break;
            }
        }
    }
    if heroes.is_empty() {
        return Err("no heroes found in this save".into());
    }
    let first_hero = heroes[0];
    // The eight player records sit right in front of the first town.
    let b = lay.faction_len;
    let mut factions_at = None;
    let from = first_hero.saturating_sub(48 * 600 + 5000).max(30_000);
    for p in from..first_hero {
        if is_town(d, p, &lay) {
            let q = p.wrapping_sub(8 * b);
            if q < p && (0..8).all(|k| faction_ok(d, q + k * b, b, 250)) {
                factions_at = Some(q);
                break;
            }
        }
    }
    let mut bl = Builder::new(d);
    // The record length differs between versions: the resources are at the same place.
    if let Some(q) = factions_at {
        let resources: [(&str, usize, i64); 7] = [
            ("Gold", 120, 4_000_000_000),
            ("Wood", 96, 16_777_215),
            ("Ore", 104, 16_777_215),
            ("Mercury", 100, 16_777_215),
            ("Sulfur", 108, 16_777_215),
            ("Crystal", 112, 16_777_215),
            ("Gems", 116, 16_777_215),
        ];
        for (k, colour) in COLOURS.iter().enumerate() {
            let at = q + k * b;
            let any = resources.iter().any(|(_, o, _)| d[at + o..at + o + 4] != [0, 0, 0, 0]) || d[at + 1] != 255;
            if !any {
                continue;
            }
            for (label, o, hi) in resources {
                bl.ranged(&["Players", colour, label], at + o, Kind::U32, 0, hi);
            }
        }
    }
    let options = creature_options(d);
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for &s in &heroes {
        let raw = &d[s + 169..s + 181];
        let end = raw.iter().position(|&c| c == 0).unwrap_or(12);
        let name: String = raw[..end].iter().map(|&c| if c < 128 { c as char } else { '?' }).collect();
        let owner = match d[s] {
            0..=7 => COLOURS[d[s] as usize],
            _ => "not hired",
        };
        let count = seen.entry(format!("{name} {owner}")).or_insert(0);
        *count += 1;
        let key = if *count > 1 { format!("{name} [{owner}] #{count}") } else { format!("{name} [{owner}]") };
        let key = key.as_str();
        bl.ranged(&["Heroes", key, "Level"], s + 49, Kind::U8, 1, 75);
        bl.ranged(&["Heroes", key, "Experience"], s + 39, Kind::U32, 0, 4_000_000_000);
        bl.ranged(&["Heroes", key, "Attack"], s + 238, Kind::U8, 0, 99);
        bl.ranged(&["Heroes", key, "Defense"], s + 239, Kind::U8, 0, 99);
        bl.ranged(&["Heroes", key, "Spell power"], s + 240, Kind::U8, 1, 99);
        bl.ranged(&["Heroes", key, "Knowledge"], s + 241, Kind::U8, 1, 99);
        bl.ranged(&["Heroes", key, "Spell points"], s + 47, Kind::U16, 0, 65_535);
        bl.ranged(&["Heroes", key, "Movement left"], s + 35, Kind::U32, 0, 1_000_000);
        bl.ranged(&["Heroes", key, "Movement (full)"], s + 31, Kind::U32, 0, 1_000_000);
        // Only the heroes in play have an army worth listing: the unhired ones are the pool.
        for slot in 0..if d[s] <= 7 { 7 } else { 0 } {
            let id_at = s + 113 + 4 * slot;
            if d[id_at..id_at + 4] == [255; 4] {
                continue;
            }
            let row = format!("{key} / slot {}", slot + 1);
            let row = row.as_str();
            bl.pick(&["Army", row, "Creature"], id_at, Kind::U32, &options);
            bl.ranged(&["Army", row, "Count"], s + 141 + 4 * slot, Kind::U32, 1, 1_000_000);
        }
        for (i, skill) in SKILLS.iter().enumerate().take(lay.skills) {
            if d[s + 182 + i] > 0 && d[s] <= 7 {
                let row = format!("{key} / {skill}");
                bl.pick(
                    &["Skills", row.as_str(), "Level"],
                    s + 182 + i,
                    Kind::U8,
                    &[("Basic".to_string(), 1), ("Advanced".to_string(), 2), ("Expert".to_string(), 3)],
                );
            }
        }
    }
    Ok(bl.finish(KIND, Checksum::None).with_note("Close the game first. Level and experience are separate numbers in the save: change them together. Skills that a hero already has can change level; heroes 'not hired' are the unused pool."))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save shaped like the game's: header, eight player records in front of a town, then hero
    /// records (the shapes are all the reader goes by).
    fn fake(hota: bool) -> Vec<u8> {
        let lay = if hota { Layout { len: 1122, skills: 30, faction_len: 149 } } else { Layout { len: 1056, skills: 28, faction_len: 145 } };
        let mut d = vec![0u8; 40_000];
        d[..5].copy_from_slice(b"H3SVG");
        d[8] = if hota { 44 } else { 42 };
        d[12] = if hota { 11 } else { 2 };
        // Eight player records; players 0 and 1 are in play.
        let q = 40_000;
        for k in 0..8 {
            let mut r = vec![0u8; lay.faction_len];
            r[1..9].fill(255);
            r[9..11].fill(255);
            if k < 2 {
                r[1] = k as u8;
                r[96..100].copy_from_slice(&(10 + k as u32).to_le_bytes());
                r[120..124].copy_from_slice(&(10_000u32 * (k as u32 + 1)).to_le_bytes());
            }
            d.extend_from_slice(&r);
        }
        // One town.
        let mut t = vec![0u8; 400];
        t[0] = 0;
        t[3] = 1;
        t[4] = 20;
        t[5] = 21;
        t[9..37].fill(255);
        t[69] = 4;
        t[71..75].copy_from_slice(b"Town");
        d.extend_from_slice(&t);
        d.extend_from_slice(&[0u8; 30]);
        // Two heroes.
        for (i, name) in ["Orrin", "Valeska"].iter().enumerate() {
            let mut h = vec![0u8; lay.len];
            h[0] = i as u8;
            h[31..35].copy_from_slice(&1500u32.to_le_bytes());
            h[35..39].copy_from_slice(&1000u32.to_le_bytes());
            h[39..43].copy_from_slice(&5000u32.to_le_bytes());
            h[43] = 1;
            h[47..49].copy_from_slice(&30u16.to_le_bytes());
            h[49] = 5;
            h[113..117].copy_from_slice(&0u32.to_le_bytes());
            h[117..145].fill(255);
            h[141..145].copy_from_slice(&20u32.to_le_bytes());
            h[145..169].fill(0);
            h[169..169 + name.len()].copy_from_slice(name.as_bytes());
            h[182 + 7] = 2; // Wisdom, advanced
            if !hota {
                h[210] = 8;
            }
            h[238..242].copy_from_slice(&[4, 3, 2, 1]);
            for e in 0..19 {
                let at = 382 + e * 8;
                h[at..at + 4].fill(255);
            }
            for e in 0..64 {
                let at = 534 + e * 8;
                h[at..at + 8].fill(255);
            }
            if hota {
                h[1092..1122].fill(0);
            }
            d.extend_from_slice(&h);
            d.extend_from_slice(&[0u8; 8]);
        }
        let _ = q;
        d
    }

    #[test]
    fn reads_and_edits_heroes_and_players() {
        for hota in [false, true] {
            let d = fake(hota);
            let mut t = parse(&d).unwrap_or_else(|e| panic!("hota {hota}: {e}"));
            assert_eq!(t.view.pointer("/Players/Red/Gold").unwrap(), 10_000, "hota {hota}");
            assert_eq!(t.view.pointer("/Players/Blue/Gold").unwrap(), 20_000);
            assert!(t.view.pointer("/Players/Tan").is_none());
            assert_eq!(t.view.pointer("/Heroes/Orrin [Red]/Attack").unwrap(), 4);
            assert_eq!(t.view.pointer("/Heroes/Valeska [Blue]/Knowledge").unwrap(), 1);
            assert_eq!(t.view.pointer("/Heroes/Orrin [Red]/Experience").unwrap(), 5000);
            assert_eq!(t.view.pointer("/Army/Orrin [Red] ~1 slot 1/Count").unwrap(), 20);
            assert_eq!(t.view.pointer("/Army/Orrin [Red] ~1 slot 1/Creature").unwrap(), "Pikeman");
            assert_eq!(t.view.pointer("/Skills/Orrin [Red] ~1 Wisdom/Level").unwrap(), "Advanced");
            t.set("/Players/Red/Gold", 777_777.into()).unwrap();
            t.set("/Heroes/Orrin [Red]/Attack", 50.into()).unwrap();
            t.set("/Army/Orrin [Red] ~1 slot 1/Count", 500.into()).unwrap();
            let out = t.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            let t2 = parse(&out).unwrap();
            assert_eq!(t2.view.pointer("/Players/Red/Gold").unwrap(), 777_777);
            assert_eq!(t2.view.pointer("/Heroes/Orrin [Red]/Attack").unwrap(), 50);
            assert_eq!(t2.view.pointer("/Army/Orrin [Red] ~1 slot 1/Count").unwrap(), 500);
            assert_eq!(t2.view.pointer("/Players/Blue/Gold").unwrap(), 20_000);
        }
    }

    #[test]
    fn gzip_with_a_wrong_checksum_is_read_and_written_back_right() {
        use flate2::{write::DeflateEncoder, Compression};
        use std::io::Write;
        let d = fake(false);
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&d).unwrap();
        let mut file = vec![0x1F, 0x8B, 8, 0, 0, 0, 0, 0, 0, 0x0B];
        file.extend(e.finish().unwrap());
        file.extend_from_slice(&[1, 2, 3, 4]);
        file.extend_from_slice(&(d.len() as u32).to_le_bytes());
        let dec = crate::formats::decode(&file);
        assert_eq!(dec.layers, vec![crate::formats::Layer::Gzip]);
        let again = crate::formats::encode(&dec.layers, &dec.content).unwrap();
        assert_eq!(crate::formats::decode(&again).layers, vec![crate::formats::Layer::Gzip]);
        assert!(matches!(dec.content, crate::formats::Content::Tree(ref t) if t.kind_name == KIND));
    }

    /// `HEROES3_SAMPLES` files: open through the whole pipeline, change a value, save, reopen.
    #[test]
    #[ignore]
    fn real_samples() {
        let Ok(list) = std::env::var("HEROES3_SAMPLES") else { return };
        for f in list.split('|') {
            let raw = std::fs::read(f).unwrap();
            let mut d = crate::formats::decode(&raw);
            let crate::formats::Content::Tree(t) = &mut d.content else {
                println!("{f}: not a tree");
                continue;
            };
            let heroes = t.view.pointer("/Heroes").and_then(|h| h.as_object()).map_or(0, |o| o.len());
            let players = t.view.pointer("/Players").cloned();
            let n = |k: &str| t.view.pointer(k).and_then(|h| h.as_object()).map_or(0, |o| o.len());
            println!("{f}: {heroes} heroes, {} army rows, {} skill rows, players {players:?}", n("/Army"), n("/Skills"));
            let hired = t.view.pointer("/Heroes").and_then(|h| h.as_object()).and_then(|o| o.iter().find(|(k, _)| !k.contains("not hired")));
            println!("  hired: {hired:?}");
            if let Some(first) = t.view.pointer("/Heroes").and_then(|h| h.as_object()).and_then(|o| o.keys().next().cloned()) {
                println!("  first hero {first}: {}", t.view.pointer(&format!("/Heroes/{first}")).unwrap());
                let p = format!("/Heroes/{first}/Attack");
                t.set(&p, 77.into()).unwrap();
                let out = crate::formats::encode(&d.layers, &d.content).unwrap();
                let d2 = crate::formats::decode(&out);
                let crate::formats::Content::Tree(t2) = &d2.content else { panic!("{f}: reopened as hex") };
                assert_eq!(t2.view.pointer(&p).unwrap(), 77);
            }
        }
    }
}
