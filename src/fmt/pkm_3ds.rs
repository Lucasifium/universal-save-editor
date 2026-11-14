//! Pokémon X / Y / Omega Ruby / Alpha Sapphire and Sun / Moon / Ultra Sun / Ultra Moon
//! (Nintendo 3DS, also Citra-family emulators) `main` saves, as exported by Checkpoint, JKSM or
//! the emulator. Block tables, checksums and record layouts follow PKHeX (GPL-3.0).

use super::pkm::{crc16_ccitt, Builder, Field, Finish, Gen, Slot, Txt};
use crate::tree::{esc, TreeDoc};
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Game {
    XY,
    AO,
    SM,
    UU,
}

impl Game {
    fn from_len(n: usize) -> Option<Game> {
        match n {
            0x65600 => Some(Game::XY),
            0x76000 => Some(Game::AO),
            0x6BE00 => Some(Game::SM),
            0x6CC00 => Some(Game::UU),
            _ => None,
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Game::XY => "XY=",
            Game::AO => "AO=",
            Game::SM => "SM=",
            Game::UU => "UU=",
        }
    }
    fn gen(self) -> Gen {
        if matches!(self, Game::XY | Game::AO) { Gen::G6 } else { Gen::G7 }
    }
}

fn blocks(game: Game) -> Vec<(usize, usize)> {
    let line = include_str!("data/pkm_3ds_blocks.txt").lines().find(|l| l.starts_with(game.tag())).unwrap_or("");
    line[game.tag().len()..]
        .split(';')
        .filter_map(|e| {
            let (a, b) = e.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        })
        .collect()
}

pub fn looks(d: &[u8]) -> bool {
    Game::from_len(d.len()).is_some() && d[d.len() - 0x1F0..d.len() - 0x1EC] == *b"BEEF"
}

/// CRC-16 as the 3DS saves of Generation 7 use it (reflected 0xA001, start 0xFFFF, inverted).
fn crc16_usb(d: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in d {
        crc ^= b as u16;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xA001 } else { crc >> 1 };
        }
    }
    !crc
}

fn finish(out: &mut Vec<u8>, p: &[usize]) -> Result<Vec<u8>, String> {
    // p[0] = generation (6 or 7), then (offset, length) pairs by block id
    let meta = out.len() - 0x200;
    for (id, b) in p[1..].chunks_exact(2).enumerate() {
        let sum = if p[0] == 6 { crc16_ccitt(&out[b[0]..b[0] + b[1]]) } else { crc16_usb(&out[b[0]..b[0] + b[1]]) };
        let at = meta + 0x14 + id * 8 + 6;
        out[at..at + 2].copy_from_slice(&sum.to_le_bytes());
    }
    Ok(out.clone())
}

const KALOS: [&str; 8] = ["Bug", "Cliff", "Rumble", "Plant", "Voltage", "Fairy", "Psychic", "Iceberg"];
const HOENN: [&str; 8] = ["Stone", "Knuckle", "Dynamo", "Heat", "Balance", "Feather", "Mind", "Rain"];

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Generation VI/VII Pokémon save".into());
    }
    let game = Game::from_len(d.len()).unwrap();
    let g = game.gen();
    let bl = blocks(game);
    if bl.is_empty() {
        return Err("unrecognised Pokémon save".into());
    }
    let at = |i: usize| bl[i].0;
    let (status, played, misc, party, boxlayout, items, boxes, box_count): (usize, usize, usize, usize, usize, usize, usize, usize) = match game {
        Game::XY => (at(17), at(6), at(11), at(18), at(12), at(1), 0x22600, 31),
        Game::AO => (at(17), at(6), at(11), at(18), at(12), at(1), 0x33000, 31),
        _ => (at(3), at(16), at(9), at(4), at(13), at(0), at(14), 32),
    };
    let ver = d[status + 4];
    let kind = match (game, ver) {
        (Game::XY, 24) => "Pokémon X",
        (Game::XY, 25) => "Pokémon Y",
        (Game::XY, _) => "Pokémon X / Y",
        (Game::AO, 26) => "Pokémon Alpha Sapphire",
        (Game::AO, 27) => "Pokémon Omega Ruby",
        (Game::AO, _) => "Pokémon Omega Ruby / Alpha Sapphire",
        (Game::SM, 30) => "Pokémon Sun",
        (Game::SM, 31) => "Pokémon Moon",
        (Game::SM, _) => "Pokémon Sun / Moon",
        (Game::UU, 32) => "Pokémon Ultra Sun",
        (Game::UU, 33) => "Pokémon Ultra Moon",
        (Game::UU, _) => "Pokémon Ultra Sun / Ultra Moon",
    };
    let mut b = Builder::default();
    let mut root = Map::new();
    let mut tr = Map::new();
    let name_off = if g == Gen::G6 { status + 0x48 } else { status + 0x38 };
    b.text(&mut tr, "/Trainer", "Name", d, name_off, 0x1A, 12, Txt::U16Z);
    b.add(&mut tr, "/Trainer", "Gender", Value::from(if d[status + 5] == 1 { "Girl" } else { "Boy" }), Some(Field::Gender { off: status + 5 }));
    tr.insert("Trainer ID".into(), Value::from(u16::from_le_bytes([d[status], d[status + 1]])));
    tr.insert("Secret ID".into(), Value::from(u16::from_le_bytes([d[status + 2], d[status + 3]])));
    let (money_off, bp_off, bp_size) = match game {
        Game::XY => (misc + 8, misc + 0x3C, 2),
        Game::AO => (misc + 8, misc + 0x30, 2),
        _ => (misc + 4, misc + 0x11C, 4),
    };
    b.int(&mut tr, "/Trainer", "Money", d, money_off, 4, 9_999_999);
    b.int(&mut tr, "/Trainer", "Battle Points", d, bp_off, bp_size, 9_999);
    b.int(&mut tr, "/Trainer", "Play time (hours)", d, played, 2, 999);
    b.int(&mut tr, "/Trainer", "Play time (minutes)", d, played + 2, 1, 59);
    b.int(&mut tr, "/Trainer", "Play time (seconds)", d, played + 3, 1, 59);
    root.insert("Trainer".into(), Value::Object(tr));
    if g == Gen::G6 {
        let mut m = Map::new();
        for (i, n) in (if game == Game::XY { KALOS } else { HOENN }).iter().enumerate() {
            b.add(&mut m, "/Badges", &format!("{n} Badge"), Value::Bool(d[misc + 0xC] >> i & 1 == 1), Some(Field::Bit { off: misc + 0xC, bit: i as u8 }));
        }
        root.insert("Badges".into(), Value::Object(m));
    }

    let n = (d[party + 6 * g.party()] as usize).min(6);
    let mut pm = Map::new();
    for i in 0..n {
        let key = format!("Pokémon {}", i + 1);
        let ptr = format!("/Party/{}", esc(&key));
        if let Some(v) = b.mon(g, d, party + i * g.party(), true, &ptr) {
            pm.insert(key, v);
        }
    }
    root.insert("Party".into(), Value::Object(pm));

    // Bag: (title, offset, slots, max quantity)
    let pouches: &[(&str, usize, usize, u32)] = match game {
        Game::XY => &[("Items", 0x000, 400, 999), ("Key items", 0x640, 96, 1), ("TMs & HMs", 0x7C0, 106, 1), ("Medicine", 0x968, 64, 999), ("Berries", 0xA68, 72, 999)],
        Game::AO => &[("Items", 0x000, 400, 999), ("Key items", 0x640, 96, 1), ("TMs & HMs", 0x7C0, 108, 1), ("Medicine", 0x970, 64, 999), ("Berries", 0xA70, 72, 999)],
        Game::SM => &[("Items", 0x000, 430, 999), ("Medicine", 0xB48, 64, 999), ("TMs & HMs", 0x998, 108, 1), ("Berries", 0xC48, 72, 999), ("Key items", 0x6B8, 184, 1), ("Z-Crystals", 0xD68, 30, 1)],
        Game::UU => &[("Items", 0x000, 427, 999), ("Medicine", 0xB74, 60, 999), ("TMs & HMs", 0x9C4, 108, 1), ("Berries", 0xC64, 67, 999), ("Key items", 0x6AC, 198, 1), ("Z-Crystals", 0xD70, 35, 1), ("Battle items", 0xDFC, 11, 999)],
    };
    let slot = if g == Gen::G6 { Slot::Pair } else { Slot::Packed7 };
    let mut bag = Map::new();
    for &(title, off, slots, max) in pouches {
        b.pocket(&mut bag, title, d, items + off, slots, slot, 4, max, g);
    }
    root.insert("Bag".into(), Value::Object(bag));

    let mut bm = Map::new();
    for bx in 0..box_count {
        let key = format!("Box {}", bx + 1);
        let bp = format!("/PC boxes/{}", esc(&key));
        let mut m = Map::new();
        let nm = boxlayout + bx * 0x22;
        if super::pkm::decode_text(Txt::U16Z, &d[nm..nm + 0x22]).is_some_and(|s| !s.is_empty()) {
            b.text(&mut m, &bp, "Box name", d, nm, 0x22, 16, Txt::U16Z);
        }
        for s in 0..30 {
            let addr = boxes + (bx * 30 + s) * g.stored();
            let k = format!("Slot {}", s + 1);
            let ptr = format!("{bp}/{}", esc(&k));
            if let Some(v) = b.mon(g, d, addr, false, &ptr) {
                m.insert(k, v);
            }
        }
        bm.insert(key, Value::Object(m));
    }
    root.insert("PC boxes".into(), Value::Object(bm));

    let mut params = vec![if g == Gen::G6 { 6 } else { 7 }];
    for (o, l) in &bl {
        params.push(*o);
        params.push(*l);
    }
    let note = "Block checksums are fixed automatically. Changing a Pokémon's level also sets its experience and stats. \
                This works on saves you extract from your console (Checkpoint, JKSM) or from Citra-family emulators; \
                Sun/Moon and Ultra Sun/Moon saves written back to a real 3DS may also need the game's signature block redone, \
                which is only needed on hardware.";
    Ok(b.finish(g, d.to_vec(), root, kind, Finish::Custom(finish, params), note))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_variants() {
        assert_eq!(crc16_usb(b"123456789"), 0xB4C8);
    }

    #[test]
    fn block_tables_cover_the_file() {
        for (g, len) in [(Game::XY, 0x65600usize), (Game::AO, 0x76000), (Game::SM, 0x6BE00), (Game::UU, 0x6CC00)] {
            let b = blocks(g);
            let (o, l) = *b.last().unwrap();
            // The last block ends at or before the metadata chunk, which sits in the last 0x200 bytes.
            assert!(o + l <= len - 0x200 || o + l <= len, "{g:?}");
            for w in b.windows(2) {
                assert!(w[0].0 + w[0].1 <= w[1].0, "{g:?} blocks overlap");
            }
        }
    }

    /// Real saves: `PKM_SAMPLES=<dir> cargo test --release --lib pkm_3ds::tests::real_saves -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("PKM_SAMPLES").expect("PKM_SAMPLES");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let mut doc = parse(&d).unwrap();
            println!("{} -> {} {}", e.path().display(), doc.kind_name, doc.view["Trainer"]);
            if let Some(Value::Object(p)) = doc.view.get("Party") {
                for (k, v) in p {
                    println!("  {k}: {} lv{} {} nature {} shiny {} stats {}", v["Species"], v["Level"], v["Nickname"], v["Nature"], v["Shiny"], v["Stats"]);
                }
            }
            super::super::pkm::verify_party_stats(&doc.view, if doc.kind_name.contains("Sun") || doc.kind_name.contains("Moon") { Gen::G7 } else { Gen::G6 });
            assert_eq!(doc.to_bytes().unwrap(), d, "unchanged save must be written back identically");
            doc.set("/Trainer/Money", Value::from(1234)).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out).unwrap();
            assert_eq!(again.view.pointer("/Trainer/Money").unwrap(), 1234);
            println!("  edited money ok");
        }
    }
}
