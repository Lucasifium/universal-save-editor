//! Civilization VI saves (`.Civ6Save`): "CIV6", then a stream of entries, each a 4-byte
//! marker and a 4-byte type (1 bool, 2 int, 5 text, 6 UTF-16 text, 0x0A / 0x0B arrays…),
//! up to the zlib-packed game state. The settings and player slots live in that
//! uncompressed part: game turn, speed, map, mods, and per civilisation its leader,
//! player name, whether a human or the AI plays it and whose turn it is.
//!
//! The walk follows civ6-save-parser (Mike Rosack / playyourdamnturn.com, MIT), ported.
//! Gold, faith and the like are in the packed game state, which isn't mapped.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const GAME_TURN: u32 = u32::from_le_bytes([0x9D, 0x2C, 0xE6, 0xBD]);
const GAME_SPEED: u32 = u32::from_le_bytes([0x99, 0xB0, 0xD9, 0x05]);
const MAP_FILE: u32 = u32::from_le_bytes([0x5A, 0x87, 0xD8, 0x63]);
const MAP_SIZE: u32 = u32::from_le_bytes([0x40, 0x5C, 0x83, 0x0B]);
const MOD_TITLE: u32 = u32::from_le_bytes([0x72, 0xE1, 0x34, 0x30]);
const START_ACTOR: u32 = u32::from_le_bytes([0x58, 0xBA, 0x7F, 0x4C]);
const END_UNCOMPRESSED: [u8; 4] = [0, 0, 1, 0];
const COMPRESSED_DATA_END: [u8; 4] = [0, 0, 0xFF, 0xFF];

const SLOT_HEADERS: [[u8; 4]; 12] = [
    [0xC8, 0x9B, 0x5F, 0x65],
    [0x5E, 0xAB, 0x58, 0x12],
    [0xE4, 0xFA, 0x51, 0x8B],
    [0x72, 0xCA, 0x56, 0xFC],
    [0xD1, 0x5F, 0x32, 0x62],
    [0x47, 0x6F, 0x35, 0x15],
    [0xFD, 0x3E, 0x3C, 0x8C],
    [0x6B, 0x0E, 0x3B, 0xFB],
    [0xFA, 0x13, 0x84, 0x6B],
    [0x6C, 0x23, 0x83, 0x1C],
    [0xF4, 0x14, 0x18, 0xAA],
    [0x62, 0x24, 0x1F, 0xDD],
];

const ACTOR_NAME: u32 = u32::from_le_bytes([0x2F, 0x5C, 0x5E, 0x9D]);
const LEADER_NAME: u32 = u32::from_le_bytes([0x5F, 0x5E, 0xCD, 0xE8]);
const ACTOR_TYPE: u32 = u32::from_le_bytes([0xBE, 0xAB, 0x55, 0xCA]);
const PLAYER_NAME: u32 = u32::from_le_bytes([0xFD, 0x6B, 0xB9, 0xDA]);
const PLAYER_ALIVE: u32 = u32::from_le_bytes([0xA6, 0xDF, 0xA7, 0x62]);
const IS_CURRENT_TURN: u32 = u32::from_le_bytes([0xCB, 0x21, 0xB0, 0x7A]);
const ACTOR_AI_HUMAN: u32 = u32::from_le_bytes([0x95, 0xB9, 0x42, 0xCE]);
const ACTOR_DESCRIPTION: u32 = u32::from_le_bytes([0x65, 0x19, 0x9B, 0xFF]);
const ACTOR_KEYS: [u32; 7] = [
    ACTOR_NAME,
    LEADER_NAME,
    ACTOR_TYPE,
    PLAYER_NAME,
    PLAYER_ALIVE,
    IS_CURRENT_TURN,
    ACTOR_AI_HUMAN,
];

#[derive(Clone, Debug, PartialEq)]
enum Data {
    Skip,
    Bool(bool),
    Int(u32),
    Text(String),
    Other,
}

#[derive(Clone, Debug)]
struct Entry {
    marker: u32,
    /// Where an int or bool value sits (marker + 16).
    value_at: usize,
    data: Data,
}

struct W<'a> {
    d: &'a [u8],
    pos: usize,
}

impl W<'_> {
    fn u32(&self, at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.d.get(at..at + 4)?.try_into().ok()?))
    }

    fn find(&self, from: usize, what: &[u8]) -> Option<usize> {
        self.d
            .get(from..)?
            .windows(what.len())
            .position(|w| w == what)
            .map(|i| i + from)
    }

    /// One entry at `pos`. Unknown types are retried one byte further on, as the
    /// reference parser does.
    fn entry(&mut self, dont_skip: bool, depth: usize) -> Option<Entry> {
        loop {
            let start = self.pos;
            let marker = self.u32(start)?;
            let ty = self.u32(start + 4)?;
            self.pos += 8;
            let value_at = start + 16;
            if !dont_skip && (marker < 256 || ty == 0) {
                return Some(Entry {
                    marker,
                    value_at,
                    data: Data::Skip,
                });
            }
            if ty == 0x18 || self.d.get(start + 4..start + 6) == Some(&[0x78, 0x9C][..]) {
                self.pos = self.find(self.pos, &COMPRESSED_DATA_END)? + 4;
                return Some(Entry {
                    marker,
                    value_at,
                    data: Data::Other,
                });
            }
            let data = match ty {
                1 => {
                    self.pos += 8;
                    let v = *self.d.get(self.pos)? != 0;
                    self.pos += 4;
                    Data::Bool(v)
                }
                2 => {
                    self.pos += 8;
                    let v = self.u32(self.pos)?;
                    self.pos += 4;
                    Data::Int(v)
                }
                0x0A => {
                    self.array_0a(depth)?;
                    Data::Other
                }
                3 => {
                    self.pos += 12;
                    Data::Other
                }
                0x15 => {
                    self.pos += if self.d.get(self.pos..self.pos + 4) == Some(&[0, 0, 0, 0x80][..])
                    {
                        20
                    } else {
                        12
                    };
                    Data::Other
                }
                4 | 5 => self.string()?,
                6 => self.utf_string()?,
                0x14 | 0x0D => {
                    self.pos += 16;
                    Data::Other
                }
                0x0B => {
                    self.array_0b(depth)?;
                    Data::Other
                }
                _ => {
                    self.pos = start + 1;
                    continue;
                }
            };
            return Some(Entry {
                marker,
                value_at,
                data,
            });
        }
    }

    fn string(&mut self) -> Option<Data> {
        let d = self.d;
        let len = u32::from_le_bytes([
            *d.get(self.pos)?,
            *d.get(self.pos + 1)?,
            *d.get(self.pos + 2)?,
            0,
        ]) as usize;
        self.pos += 2;
        let kind = *d.get(self.pos + 1)?;
        if kind == 0 || kind == 0x20 {
            self.pos += 10;
            return Some(Data::Other);
        }
        if kind == 0x21 {
            self.pos += 6;
            let end = d.get(self.pos..)?.iter().position(|&c| c == 0)?;
            let s = String::from_utf8_lossy(&d[self.pos..self.pos + end]).into_owned();
            self.pos += len;
            return Some(Data::Text(s));
        }
        Some(Data::Other)
    }

    fn utf_string(&mut self) -> Option<Data> {
        let len =
            u16::from_le_bytes(self.d.get(self.pos..self.pos + 2)?.try_into().ok()?) as usize * 2;
        self.pos += 2;
        if self.d.get(self.pos..self.pos + 6) == Some(&[0, 0x21, 2, 0, 0, 0][..]) {
            self.pos += 6;
            let s = self.d.get(self.pos..self.pos + len.saturating_sub(2))?;
            let u: Vec<u16> = s
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            self.pos += len;
            return Some(Data::Text(String::from_utf16_lossy(&u)));
        }
        Some(Data::Other)
    }

    fn array_0a(&mut self, depth: usize) -> Option<()> {
        self.pos += 8;
        let n = self.u32(self.pos)?;
        self.pos += 4;
        if depth > 8 {
            return None;
        }
        for _ in 0..n {
            if self.u32(self.pos)? > n {
                return Some(());
            }
            self.entry(true, depth + 1)?;
        }
        Some(())
    }

    /// Mod lists and the like: elements start with 0x0A and end at an entry whose text is "1".
    fn array_0b(&mut self, depth: usize) -> Option<Vec<Vec<Entry>>> {
        let mut out = vec![];
        let start = self.pos;
        self.pos += 8;
        let n = self.u32(self.pos)?;
        self.pos += 4;
        if depth > 8 || n > 10_000 {
            return None;
        }
        for _ in 0..n {
            if *self.d.get(self.pos)? != 0x0A {
                // The reference parser gives up on the array here and carries on after its header.
                let _ = start;
                return Some(out);
            }
            self.pos += 16;
            let mut el = vec![];
            loop {
                let e = self.entry(false, depth + 1)?;
                let end = e.data == Data::Text("1".into());
                el.push(e);
                if end {
                    break;
                }
            }
            out.push(el);
        }
        Some(out)
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && &d[..4] == b"CIV6"
}

#[derive(Default)]
struct Actor {
    slot: Option<usize>,
    fields: Vec<Entry>,
}

impl Actor {
    fn get(&self, m: u32) -> Option<&Entry> {
        self.fields.iter().find(|e| e.marker == m)
    }
    fn text(&self, m: u32) -> Option<String> {
        match &self.get(m)?.data {
            Data::Text(s) => Some(s.clone()),
            _ => None,
        }
    }
}

fn pretty(id: &str, prefix: &str) -> String {
    let s = id.strip_prefix(prefix).unwrap_or(id);
    s.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            if w.len() <= 4 && w.chars().all(|c| matches!(c, 'I' | 'V' | 'X')) {
                return w.to_string(); // Roman numerals: Philip II
            }
            let w = w.to_lowercase();
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A mod title is either plain text or `{"LOC_KEY":[{"locale":"en_US","text":"…"}…]}`:
/// the English text if there is one, else the key made readable.
fn mod_title(t: &str) -> String {
    let Ok(serde_json::Value::Object(m)) = serde_json::from_str::<serde_json::Value>(t) else {
        return t.to_string();
    };
    let Some((key, v)) = m.into_iter().next() else {
        return t.to_string();
    };
    let en = v
        .as_array()
        .and_then(|a| a.iter().find(|x| x["locale"] == "en_US"))
        .and_then(|x| x["text"].as_str())
        .map(str::to_string);
    en.unwrap_or_else(|| {
        let k = key.strip_prefix("LOC_").unwrap_or(&key);
        let k = k
            .strip_suffix("_MOD_TITLE")
            .or_else(|| k.strip_suffix("_TITLE"))
            .unwrap_or(k);
        if k.contains(' ') {
            k.to_string()
        } else {
            pretty(k, "")
        }
    })
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Civilization VI save".into());
    }
    let mut w = W { d, pos: 0 };
    // The entries we can follow start at the game speed.
    let speed = GAME_SPEED.to_le_bytes();
    w.pos = w.find(0, &speed).ok_or("no game settings found")?;
    let mut game: Vec<Entry> = vec![];
    let mut actors: Vec<Actor> = vec![];
    let mut cur: Option<usize> = None;
    let mut mods: Vec<String> = vec![];
    while w.pos + 8 < d.len() {
        if d.get(w.pos..w.pos + 4) == Some(&END_UNCOMPRESSED[..]) {
            break;
        }
        let before = w.pos;
        let marker = w.u32(w.pos).unwrap_or(0);
        let ty = w.u32(w.pos + 4).unwrap_or(0);
        // Mod lists: keep the titles.
        if ty == 0x0B && marker >= 256 {
            w.pos += 8;
            let Some(els) = w.array_0b(0) else { break };
            for el in els {
                if let Some(Data::Text(t)) = el
                    .iter()
                    .find(|e| e.marker == MOD_TITLE)
                    .map(|e| e.data.clone())
                {
                    let t = mod_title(&t);
                    if !mods.contains(&t) {
                        mods.push(t);
                    }
                }
            }
            continue;
        }
        let Some(e) = w.entry(false, 0) else { break };
        if w.pos <= before {
            break;
        }
        let slot = SLOT_HEADERS
            .iter()
            .position(|h| u32::from_le_bytes(*h) == e.marker);
        if let Some(s) = slot {
            actors.push(Actor {
                slot: Some(s),
                fields: vec![],
            });
            cur = Some(actors.len() - 1);
        } else if cur.is_none() && e.marker == START_ACTOR {
            actors.push(Actor::default());
            cur = Some(actors.len() - 1);
        } else if e.marker == ACTOR_DESCRIPTION {
            cur = None;
        } else {
            if [GAME_TURN, GAME_SPEED, MAP_FILE, MAP_SIZE].contains(&e.marker)
                && !game.iter().any(|g| g.marker == e.marker)
            {
                game.push(e.clone());
            }
            if let Some(c) = cur {
                if ACTOR_KEYS.contains(&e.marker) {
                    actors[c].fields.retain(|f| f.marker != e.marker);
                    actors[c].fields.push(e);
                }
            }
        }
    }

    let mut b = Builder::new(d);
    let gtext = |m: u32| {
        game.iter().find(|g| g.marker == m).and_then(|g| {
            if let Data::Text(s) = &g.data {
                Some(s.clone())
            } else {
                None
            }
        })
    };
    if let Some(t) = game.iter().find(|g| g.marker == GAME_TURN) {
        if let Data::Int(v) = t.data {
            b.info(&["Game", "Turn"], v);
        }
    }
    if let Some(s) = gtext(GAME_SPEED) {
        b.info(&["Game", "Speed"], pretty(&s, "GAMESPEED_"));
    }
    if let Some(s) = gtext(MAP_SIZE) {
        b.info(&["Game", "Map size"], pretty(&s, "MAPSIZE_"));
    }
    if let Some(s) = gtext(MAP_FILE) {
        let f = s
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(&s)
            .trim_end_matches(".lua")
            .to_string();
        b.info(&["Game", "Map"], f);
    }
    for (i, m) in mods.iter().enumerate() {
        b.info(&["Mods", &format!("{:03}", i + 1)], m.clone());
    }

    // Civilisations: per slot, the first full civ that has a player type.
    let mut civs = 0;
    for slot in 0..SLOT_HEADERS.len() {
        let Some(a) = actors.iter().find(|a| {
            a.slot == Some(slot)
                && a.get(ACTOR_AI_HUMAN)
                    .is_some_and(|e| e.data != Data::Int(2))
                && a.text(ACTOR_TYPE).as_deref() == Some("CIVILIZATION_LEVEL_FULL_CIV")
                && a.text(ACTOR_NAME).is_some()
        }) else {
            continue;
        };
        civs += 1;
        let civ = pretty(&a.text(ACTOR_NAME).unwrap_or_default(), "CIVILIZATION_");
        let key = format!("{:02} {civ}", slot + 1);
        let p = |f: &str| ["Civilizations".to_string(), key.clone(), f.to_string()];
        let put = |b: &mut Builder, f: &str, v: String| {
            let pp = p(f);
            b.info(&[&pp[0], &pp[1], &pp[2]], v);
        };
        put(&mut b, "Civilization", civ.clone());
        if let Some(l) = a.text(LEADER_NAME) {
            put(&mut b, "Leader", pretty(&l, "LEADER_"));
        }
        if let Some(n) = a.text(PLAYER_NAME) {
            put(&mut b, "Player", n);
        }
        if let Some(e) = a.get(ACTOR_AI_HUMAN) {
            let pp = p("Played by");
            b.pick(
                &[&pp[0], &pp[1], &pp[2]],
                e.value_at,
                Kind::U32,
                &[("AI".to_string(), 1), ("Human".to_string(), 3)],
            );
        }
        if let Some(e) = a
            .get(IS_CURRENT_TURN)
            .filter(|e| matches!(e.data, Data::Bool(_)))
        {
            let pp = p("Current turn");
            b.field(&[&pp[0], &pp[1], &pp[2]], e.value_at, Kind::B8);
        }
        if let Some(Data::Bool(alive)) = a.get(PLAYER_ALIVE).map(|e| e.data.clone()) {
            let pp = p("Alive");
            b.info(&[&pp[0], &pp[1], &pp[2]], alive);
        }
    }
    if civs == 0 {
        return Err("no civilisations found in this Civilization VI save".into());
    }
    Ok(b.finish("Civilization VI save", Checksum::None).with_note(
        "The game's settings and its civilisations. Each civilisation can be handed to a human player or the AI, and whose turn it is \
         can be changed (as hotseat and Play Your Damn Turn games do). Gold, faith and the map are in the packed part of the save, which isn't mapped.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(o: &mut Vec<u8>, m: u32, v: u32) {
        o.extend(m.to_le_bytes());
        o.extend([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        o.extend(v.to_le_bytes());
    }
    fn boolean(o: &mut Vec<u8>, m: u32, v: bool) {
        o.extend(m.to_le_bytes());
        o.extend([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        o.extend((v as u32).to_le_bytes());
    }
    fn text(o: &mut Vec<u8>, m: u32, s: &str) {
        o.extend(m.to_le_bytes());
        o.extend([5, 0, 0, 0]);
        o.extend(((s.len() + 1) as u16).to_le_bytes());
        o.extend([0, 0x21, 1, 0, 0, 0]);
        o.extend(s.as_bytes());
        o.push(0);
    }

    fn sample() -> Vec<u8> {
        let mut d = b"CIV6".to_vec();
        d.extend([0u8; 12]);
        text(&mut d, GAME_SPEED, "GAMESPEED_STANDARD");
        int(&mut d, GAME_TURN, 42);
        text(&mut d, MAP_SIZE, "MAPSIZE_SMALL");
        for (slot, civ, human) in [
            (0, "CIVILIZATION_ROME", true),
            (1, "CIVILIZATION_EGYPT", false),
        ] {
            int(&mut d, u32::from_le_bytes(SLOT_HEADERS[slot]), 0);
            text(&mut d, ACTOR_NAME, civ);
            text(&mut d, ACTOR_TYPE, "CIVILIZATION_LEVEL_FULL_CIV");
            int(&mut d, ACTOR_AI_HUMAN, if human { 3 } else { 1 });
            boolean(&mut d, IS_CURRENT_TURN, human);
            int(&mut d, ACTOR_DESCRIPTION, 0);
        }
        d.extend(END_UNCOMPRESSED);
        d.extend([0u8; 64]);
        d
    }

    #[test]
    fn reads_and_switches_players() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        let v = &doc.view;
        assert_eq!(v.pointer("/Game/Turn"), Some(&42.into()));
        assert_eq!(v.pointer("/Game/Speed"), Some(&"Standard".into()));
        assert_eq!(
            v.pointer("/Civilizations/01 Rome/Played by"),
            Some(&"Human".into())
        );
        assert_eq!(
            v.pointer("/Civilizations/02 Egypt/Played by"),
            Some(&"AI".into())
        );
        doc.set("/Civilizations/02 Egypt/Played by", "Human".into())
            .unwrap();
        doc.set("/Civilizations/01 Rome/Current turn", false.into())
            .unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out.len(), d.len());
        let v2 = parse(&out).unwrap().view;
        assert_eq!(
            v2.pointer("/Civilizations/02 Egypt/Played by"),
            Some(&"Human".into())
        );
        assert_eq!(
            v2.pointer("/Civilizations/01 Rome/Current turn"),
            Some(&false.into())
        );
    }

    /// Research aid: SAVE_EDITOR_CIV6 = a folder of .Civ6Save files.
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("SAVE_EDITOR_CIV6").unwrap();
        for f in std::fs::read_dir(dir).unwrap() {
            let p = f.unwrap().path();
            let d = std::fs::read(&p).unwrap();
            match parse(&d) {
                Ok(doc) => {
                    println!(
                        "{}: {}",
                        p.display(),
                        serde_json::to_string(&doc.view)
                            .unwrap()
                            .chars()
                            .take(900)
                            .collect::<String>()
                    );
                    assert_eq!(doc.to_bytes().unwrap(), d);
                }
                Err(e) => println!("{}: ERR {e}", p.display()),
            }
        }
    }
}
