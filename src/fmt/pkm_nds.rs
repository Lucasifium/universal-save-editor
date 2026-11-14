//! Pokémon Diamond / Pearl / Platinum / HeartGold / SoulSilver and Black / White / Black 2 /
//! White 2 (Nintendo DS) saves: 512 KiB `.sav` files, also the DeSmuME `.dsv` flavour with its
//! footer. Record and block layouts follow PKHeX (GPL-3.0).

use super::pkm::{crc16_ccitt, name_of, names, Builder, Finish, Gen, Slot, Txt};
use crate::tree::{esc, TreeDoc};
use serde_json::{Map, Value};

const SIZE: usize = 0x80000;
const PART: usize = 0x40000;

fn rd32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Game {
    DiamondPearl,
    Platinum,
    HeartGoldSoulSilver,
    BlackWhite,
    Black2White2,
}

impl Game {
    #[allow(dead_code)]
    fn gen(self) -> Gen {
        match self {
            Game::BlackWhite | Game::Black2White2 => Gen::G5,
            _ => Gen::G4,
        }
    }
}

/// (general size, storage size, storage start, footer size, trainer, party, bag, rival-free)
struct G4 {
    general: usize,
    storage: usize,
    s_start: usize,
    footer: usize,
    trainer: usize,
    party: usize,
    bag: usize,
}

fn g4(game: Game) -> G4 {
    match game {
        Game::DiamondPearl => G4 { general: 0xC100, storage: 0x121E0, s_start: 0xC100, footer: 0x14, trainer: 0x64, party: 0x98, bag: 0x624 },
        Game::Platinum => G4 { general: 0xCF2C, storage: 0x121E4, s_start: 0xCF2C, footer: 0x14, trainer: 0x68, party: 0xA0, bag: 0x630 },
        _ => G4 { general: 0xF628, storage: 0x12310, s_start: 0xF700, footer: 0x10, trainer: 0x64, party: 0x98, bag: 0x644 },
    }
}

/// The partition (0 or 1) holding the newest copy of a block, by its footer counters.
fn active(d: &[u8], begin: usize, len: usize) -> usize {
    let o = begin + len - 0x14;
    let cmp = |a: u32, b: u32| -> Option<usize> {
        if a == u32::MAX && b != u32::MAX - 1 {
            return Some(1);
        }
        if b == u32::MAX && a != u32::MAX - 1 {
            return Some(0);
        }
        match a.cmp(&b) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            _ => None,
        }
    };
    cmp(rd32(d, o), rd32(d, o + PART)).unwrap_or_else(|| usize::from(cmp(rd32(d, o + 4), rd32(d, o + 4 + PART)) == Some(1)))
}

fn valid_general(d: &[u8], len: usize) -> bool {
    let g = &d[PART..PART + len];
    rd32(g, len - 0xC) as usize == len && matches!(rd32(g, len - 8), 0x2006_0623 | 0x2007_0903)
}

fn detect4(d: &[u8]) -> Option<Game> {
    [Game::DiamondPearl, Game::Platinum, Game::HeartGoldSoulSilver].into_iter().find(|&g| valid_general(d, g4(g).general))
}

#[allow(dead_code)]
fn crc_ok(d: &[u8], off: usize, len: usize, footer: usize) -> bool {
    crc16_ccitt(&d[off..off + len - footer]) == u16::from_le_bytes([d[off + len - 2], d[off + len - 1]])
}

fn blocks5(game: Game) -> Vec<[usize; 4]> {
    let tag = if game == Game::BlackWhite { "BW=" } else { "B2W2=" };
    let line = include_str!("data/pkm_nds_blocks.txt").lines().find(|l| l.starts_with(tag)).unwrap_or("");
    line[tag.len()..]
        .split(';')
        .filter_map(|e| {
            let v: Vec<usize> = e.split(',').filter_map(|x| x.parse().ok()).collect();
            (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
        })
        .collect()
}

fn detect5(d: &[u8]) -> Option<Game> {
    for (g, main, info) in [(Game::BlackWhite, 0x24000, 0x8C), (Game::Black2White2, 0x26000, 0x94)] {
        let f = &d[main - 0x100..main - 0x100 + info + 0x10];
        let stored = u16::from_le_bytes([f[f.len() - 2], f[f.len() - 1]]);
        if crc16_ccitt(&f[..info]) == stored {
            return Some(g);
        }
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    let n = d.len();
    (n == SIZE || (n > SIZE && n < SIZE + 0x200 && d[SIZE..].starts_with(b"|-DESMUME SAVE-|"))) && (detect4(d).is_some() || detect5(d).is_some())
}

fn finish4(out: &mut Vec<u8>, p: &[usize]) -> Result<Vec<u8>, String> {
    // p: general offset, general len, storage offset, storage len, footer, hgss
    let (go, gl, so, sl, foot, hgss) = (p[0], p[1], p[2], p[3], p[4], p[5]);
    if hgss == 1 {
        // The game skips boxes it thinks haven't changed, so mark all of them.
        out[so + 0x12000 + 4..so + 0x12000 + 8].copy_from_slice(&0x3_FFFFu32.to_le_bytes());
    }
    for (o, l) in [(go, gl), (so, sl)] {
        let sum = crc16_ccitt(&out[o..o + l - foot]);
        out[o + l - 2..o + l].copy_from_slice(&sum.to_le_bytes());
    }
    Ok(out.clone())
}

fn finish5(out: &mut Vec<u8>, p: &[usize]) -> Result<Vec<u8>, String> {
    for b in p.chunks_exact(4) {
        let sum = crc16_ccitt(&out[b[0]..b[0] + b[1]]).to_le_bytes();
        out[b[2]..b[2] + 2].copy_from_slice(&sum);
        out[b[3]..b[3] + 2].copy_from_slice(&sum);
    }
    Ok(out.clone())
}

const SINNOH: [&str; 8] = ["Coal", "Forest", "Cobble", "Fen", "Relic", "Mine", "Icicle", "Beacon"];
const JOHTO: [&str; 8] = ["Zephyr", "Hive", "Plain", "Fog", "Storm", "Mineral", "Glacier", "Rising"];
const KANTO: [&str; 8] = ["Boulder", "Cascade", "Thunder", "Rainbow", "Soul", "Marsh", "Volcano", "Earth"];
const UNOVA1: [&str; 8] = ["Trio", "Basic", "Insect", "Bolt", "Quake", "Jet", "Freeze", "Legend"];
const UNOVA2: [&str; 8] = ["Basic", "Toxic", "Insect", "Bolt", "Quake", "Jet", "Legend", "Wave"];

fn badges(b: &mut Builder, root: &mut Map<String, Value>, d: &[u8], sets: &[(usize, &[&str; 8])]) {
    let mut m = Map::new();
    for (off, names) in sets {
        for (i, n) in names.iter().enumerate() {
            let on = d[*off] >> i & 1 == 1;
            b.add(&mut m, "/Badges", &format!("{n} Badge"), Value::Bool(on), Some(super::pkm::Field::Bit { off: *off, bit: i as u8 }));
        }
    }
    root.insert("Badges".into(), Value::Object(m));
}

pub fn parse(file: &[u8]) -> Result<TreeDoc, String> {
    if !looks(file) {
        return Err("not a Generation IV/V Pokémon save".into());
    }
    let d = &file[..SIZE];
    if let Some(game) = detect4(d) {
        parse4(file, game)
    } else {
        parse5(file, detect5(d).ok_or("unrecognised Pokémon save")?)
    }
}

fn parse4(file: &[u8], game: Game) -> Result<TreeDoc, String> {
    let d = &file[..SIZE];
    let c = g4(game);
    let g = Gen::G4;
    let gp = active(d, 0, c.general) * PART;
    let sp = active(d, c.s_start, c.storage) * PART;
    let (go, so) = (gp, sp + c.s_start);
    let raw = file.to_vec();
    let t = go + c.trainer;
    let hgss = game == Game::HeartGoldSoulSilver;
    let kind = match game {
        Game::DiamondPearl => "Pokémon Diamond / Pearl",
        Game::Platinum => "Pokémon Platinum",
        _ => {
            if d[t + 0x1C] == 13 { "Pokémon SoulSilver" } else { "Pokémon HeartGold" }
        }
    };
    let mut b = Builder::default();
    let mut root = Map::new();
    let mut tr = Map::new();
    b.text(&mut tr, "/Trainer", "Name", d, t, 16, 7, Txt::G4);
    b.add(&mut tr, "/Trainer", "Gender", Value::from(if d[t + 0x18] == 1 { "Girl" } else { "Boy" }), Some(super::pkm::Field::Gender { off: t + 0x18 }));
    tr.insert("Trainer ID".into(), Value::from(u16::from_le_bytes([d[t + 0x10], d[t + 0x11]])));
    tr.insert("Secret ID".into(), Value::from(u16::from_le_bytes([d[t + 0x12], d[t + 0x13]])));
    b.int(&mut tr, "/Trainer", "Money", d, t + 0x14, 4, 999_999);
    b.int(&mut tr, "/Trainer", "Coins", d, t + 0x20, 2, 50_000);
    b.int(&mut tr, "/Trainer", "Play time (hours)", d, t + 0x22, 2, 999);
    b.int(&mut tr, "/Trainer", "Play time (minutes)", d, t + 0x24, 1, 59);
    b.int(&mut tr, "/Trainer", "Play time (seconds)", d, t + 0x25, 1, 59);
    root.insert("Trainer".into(), Value::Object(tr));
    if hgss {
        badges(&mut b, &mut root, d, &[(t + 0x1A, &JOHTO), (t + 0x1F, &KANTO)]);
    } else {
        badges(&mut b, &mut root, d, &[(t + 0x1A, &SINNOH)]);
    }

    // Party
    let party_at = go + c.party;
    let n = (d[party_at - 4] as usize).min(6);
    let mut party = Map::new();
    for i in 0..n {
        let key = format!("Pokémon {}", i + 1);
        let ptr = format!("/Party/{}", esc(&key));
        if let Some(v) = b.mon(g, d, party_at + i * g.party(), true, &ptr) {
            party.insert(key, v);
        }
    }
    root.insert("Party".into(), Value::Object(party));

    // Bag: pouch start offsets inside the bag area, (title, offset, slots, max quantity)
    let base = go + c.bag;
    type Pouch = (&'static str, usize, usize, u32);
    let pouches: &[Pouch] = if hgss {
        &[("Items", 0x000, 165, 999), ("Key items", 0x294, 50, 1), ("TMs & HMs", 0x35C, 100, 99), ("Mail", 0x4F0, 12, 999), ("Medicine", 0x520, 40, 999), ("Berries", 0x5C0, 64, 999), ("Poké Balls", 0x6C0, 24, 999), ("Battle items", 0x720, 13, 999)]
    } else {
        &[("Items", 0x000, 165, 999), ("Key items", 0x294, 50, 1), ("TMs & HMs", 0x35C, 100, 99), ("Mail", 0x4EC, 12, 999), ("Medicine", 0x51C, 40, 999), ("Berries", 0x5BC, 64, 999), ("Poké Balls", 0x6BC, 15, 999), ("Battle items", 0x6F8, 13, 999)]
    };
    let mut bag = Map::new();
    for &(title, off, slots, max) in pouches {
        b.pocket(&mut bag, title, d, base + off, slots, Slot::Pair, 4, max, g);
    }
    root.insert("Bag".into(), Value::Object(bag));

    // PC boxes
    let (box_at, box_stride, name_at): (usize, usize, usize) = if hgss { (so, 0x1000, so + 0x12008) } else { (so + 4, 30 * g.stored(), so + 4 + 18 * 30 * g.stored()) };
    let mut boxes = Map::new();
    for bx in 0..18 {
        let key = format!("Box {}", bx + 1);
        let bp = format!("/PC boxes/{}", esc(&key));
        let mut bm = Map::new();
        let nm = name_at + bx * 40;
        if super::pkm::decode_text(Txt::G4, &d[nm..nm + 40]).is_some_and(|s| !s.is_empty()) {
            b.text(&mut bm, &bp, "Box name", d, nm, 40, 8, Txt::G4);
        }
        for slot in 0..30 {
            let addr = box_at + bx * box_stride + slot * g.stored();
            let k = format!("Slot {}", slot + 1);
            let ptr = format!("{bp}/{}", esc(&k));
            if let Some(v) = b.mon(g, d, addr, false, &ptr) {
                bm.insert(k, v);
            }
        }
        boxes.insert(key, Value::Object(bm));
    }
    root.insert("PC boxes".into(), Value::Object(boxes));

    let _ = name_of;
    let _ = names;
    let fin = Finish::Custom(finish4, vec![go, c.general, so, c.storage, c.footer, usize::from(hgss)]);
    Ok(b.finish(g, raw, root, kind, fin, NOTE))
}

const NOTE: &str = "Edits go into the newest of the two copies the game keeps, and checksums are fixed automatically. Changing a Pokémon's level also sets its experience and stats. Close the game in your emulator before saving, and don't load a save state afterwards (that would overwrite the edit).";

fn parse5(file: &[u8], game: Game) -> Result<TreeDoc, String> {
    let d = &file[..SIZE];
    let g = Gen::G5;
    let blocks = blocks5(game);
    let b2 = game == Game::Black2White2;
    let trainer = blocks[27][0];
    let misc = blocks[52][0];
    let bag_at = blocks[25][0];
    let kind = match (b2, d[trainer + 0x1F]) {
        (false, 21) => "Pokémon Black",
        (false, 20) => "Pokémon White",
        (true, 23) => "Pokémon Black 2",
        (true, 22) => "Pokémon White 2",
        (false, _) => "Pokémon Black / White",
        (true, _) => "Pokémon Black 2 / White 2",
    };
    let mut b = Builder::default();
    let mut root = Map::new();
    let mut tr = Map::new();
    b.text(&mut tr, "/Trainer", "Name", d, trainer + 4, 16, 7, Txt::U16F);
    b.add(&mut tr, "/Trainer", "Gender", Value::from(if d[trainer + 0x21] == 1 { "Girl" } else { "Boy" }), Some(super::pkm::Field::Gender { off: trainer + 0x21 }));
    tr.insert("Trainer ID".into(), Value::from(u16::from_le_bytes([d[trainer + 0x14], d[trainer + 0x15]])));
    tr.insert("Secret ID".into(), Value::from(u16::from_le_bytes([d[trainer + 0x16], d[trainer + 0x17]])));
    b.int(&mut tr, "/Trainer", "Money", d, misc, 4, 9_999_999);
    b.int(&mut tr, "/Trainer", "Play time (hours)", d, trainer + 0x24, 2, 999);
    b.int(&mut tr, "/Trainer", "Play time (minutes)", d, trainer + 0x26, 1, 59);
    b.int(&mut tr, "/Trainer", "Play time (seconds)", d, trainer + 0x27, 1, 59);
    root.insert("Trainer".into(), Value::Object(tr));
    badges(&mut b, &mut root, d, &[(misc + 4, if b2 { &UNOVA2 } else { &UNOVA1 })]);

    let party_at = blocks[26][0];
    let n = (d[party_at + 4] as usize).min(6);
    let mut party = Map::new();
    for i in 0..n {
        let key = format!("Pokémon {}", i + 1);
        let ptr = format!("/Party/{}", esc(&key));
        if let Some(v) = b.mon(g, d, party_at + 8 + i * g.party(), true, &ptr) {
            party.insert(key, v);
        }
    }
    root.insert("Party".into(), Value::Object(party));

    let mut bag = Map::new();
    for (title, off, slots, max) in [("Items", 0x000, 310, 999), ("Key items", 0x4D8, 83, 1), ("TMs & HMs", 0x624, 109, 1), ("Medicine", 0x7D8, 48, 999), ("Berries", 0x898, 64, 999)] {
        b.pocket(&mut bag, title, d, bag_at + off, slots, Slot::Pair, 4, max, g);
    }
    root.insert("Bag".into(), Value::Object(bag));

    let mut boxes = Map::new();
    let names_at = blocks[0][0];
    for bx in 0..24 {
        let key = format!("Box {}", bx + 1);
        let bp = format!("/PC boxes/{}", esc(&key));
        let mut bm = Map::new();
        let nm = names_at + 4 + bx * 0x28;
        if super::pkm::decode_text(Txt::U16F, &d[nm..nm + 0x14]).is_some_and(|s| !s.is_empty()) {
            b.text(&mut bm, &bp, "Box name", d, nm, 0x14, 13, Txt::U16F);
        }
        for slot in 0..30 {
            let addr = blocks[1 + bx][0] + slot * g.stored();
            let k = format!("Slot {}", slot + 1);
            let ptr = format!("{bp}/{}", esc(&k));
            if let Some(v) = b.mon(g, d, addr, false, &ptr) {
                bm.insert(k, v);
            }
        }
        boxes.insert(key, Value::Object(bm));
    }
    root.insert("PC boxes".into(), Value::Object(boxes));

    let flat: Vec<usize> = blocks.iter().flat_map(|x| x.iter().copied()).collect();
    Ok(b.finish(g, file.to_vec(), root, kind, Finish::Custom(finish5, flat), NOTE))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real saves: every one must parse, write back identically unchanged, and survive an edit.
    /// `PKM_SAMPLES=<dir> cargo test --release --lib pkm_nds::tests::real_saves -- --ignored --nocapture`
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
            println!("{} -> {}", e.path().display(), doc.kind_name);
            println!("{}", doc.view["Trainer"]);
            if let Some(Value::Object(p)) = doc.view.get("Party") {
                for (k, v) in p {
                    println!("  {k}: {} lv{} {} nature {} shiny {} stats {}", v["Species"], v["Level"], v["Nickname"], v["Nature"], v["Shiny"], v["Stats"]);
                }
            }
            super::super::pkm::verify_party_stats(&doc.view, if doc.kind_name.contains("Black") && doc.kind_name.contains("2") || doc.kind_name.contains("White") || doc.kind_name == "Pokémon Black" { Gen::G5 } else { Gen::G4 });
            assert_eq!(doc.to_bytes().unwrap(), d, "unchanged save must be written back identically");
            doc.set("/Trainer/Money", Value::from(1234)).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out).unwrap();
            assert_eq!(again.view.pointer("/Trainer/Money").unwrap(), 1234);
            println!("  edited money ok");
        }
    }
}
