//! Paradox binary saves (ironman, and the default saves of Hearts of Iron IV and Europa
//! Universalis V): Europa Universalis IV, Crusader Kings II, Hearts of Iron IV, and the
//! `SAV…` files of Crusader Kings III, Victoria 3, Imperator: Rome and Europa Universalis V.
//!
//! The data is a stream of little-endian 16-bit ids. A few ids are structure (`=`, `{`,
//! `}`) or typed values followed by their bytes; every other id stands for a field name.
//! Blocks end with a `}` id and store no length, so a value is changed by overwriting its
//! own bytes and everything else stays byte for byte: the file remains a binary save.
//!
//! The games don't publish which id is which name. A short list for the fields the game
//! screens use is built in; fields outside it show as their id (`0x2d82`) unless the
//! player supplies a token file (see [`Names`]). Europa Universalis V stores nearly all
//! names in the save itself (`string_lookup`), so its saves read in full.
//!
//! The lexeme ids and number encodings follow the jomini parser (MIT, see
//! THIRD_PARTY_NOTICES.md); the built-in names come from CEParser (MIT).

use crate::tree::{esc, unique_key, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

const SECTION_BUDGET: usize = 6_000;
const FOCUS_BUDGET: usize = 400_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Game {
    Eu4,
    Ck2,
    Hoi4,
    Ck3,
    Vic3,
    Imperator,
    Eu5,
    Unknown,
}

impl Game {
    fn kind_name(self) -> &'static str {
        match self {
            Game::Eu4 => "Europa Universalis IV save (binary)",
            Game::Ck2 => "Crusader Kings II save (binary)",
            Game::Hoi4 => "Hearts of Iron IV save (binary)",
            Game::Ck3 => "Crusader Kings III save (binary)",
            Game::Vic3 => "Victoria 3 save (binary)",
            Game::Imperator => "Imperator: Rome save (binary)",
            Game::Eu5 => "Europa Universalis V save (binary)",
            Game::Unknown => "Paradox save (binary)",
        }
    }

    /// The name of this game's token file (`tokens/eu4.txt`).
    pub fn slug(self) -> &'static str {
        match self {
            Game::Eu4 => "eu4",
            Game::Ck2 => "ck2",
            Game::Hoi4 => "hoi4",
            Game::Ck3 => "ck3",
            Game::Vic3 => "vic3",
            Game::Imperator => "imperator",
            Game::Eu5 => "eu5",
            Game::Unknown => "paradox",
        }
    }

    /// The game a `SAV…` game state belongs to, by its first field.
    pub fn guess(body: &[u8]) -> Game {
        match body.get(..2) {
            Some([0x55, 0x31]) => Game::Ck3,
            Some([0xad, 0x55]) => Game::Vic3,
            Some([0xde, 0x09]) => Game::Eu5,
            Some([0x8f, 0x05]) => Game::Imperator,
            _ => Game::Unknown,
        }
    }

    pub fn from_ext(ext: &str) -> Game {
        match ext {
            "eu4" => Game::Eu4,
            "ck2" => Game::Ck2,
            "hoi4" => Game::Hoi4,
            "ck3" => Game::Ck3,
            "v3" => Game::Vic3,
            "rome" => Game::Imperator,
            "eu5" => Game::Eu5,
            _ => Game::Unknown,
        }
    }
}

/// Is this one of the kind names this module gives its documents?
pub fn is_kind(kind: &str) -> bool {
    [Game::Eu4, Game::Ck2, Game::Hoi4, Game::Ck3, Game::Vic3, Game::Imperator, Game::Eu5, Game::Unknown].iter().any(|g| g.kind_name() == kind)
}

// ------------------------------------------------------------------ field names

/// The fields the game screens show, by id. Everything else needs a token file.
const EU4_NAMES: &[(u16, &str)] = &[
    (0x001b, "name"), (0x0179, "checksum"), (0x2719, "war_exhaustion"), (0x2839, "base_tax"), (0x283b, "primary_culture"),
    (0x283e, "religion"), (0x283f, "manpower"), (0x284d, "date"), (0x284e, "capital"), (0x2876, "treasury"),
    (0x2886, "stability"), (0x2888, "inflation"), (0x290f, "war_exhaustion"), (0x2951, "army_tradition"),
    (0x2952, "navy_tradition"), (0x2973, "max_manpower"), (0x29a1, "prestige"), (0x2a38, "player"),
    (0x2a3a, "estimated_monthly_income"), (0x2a49, "prestige"), (0x2ab2, "powers"), (0x2b27, "score"),
    (0x2b51, "legitimacy"), (0x2beb, "patriarch_authority"), (0x2bfc, "piety"), (0x2bfd, "mercantilism"),
    (0x2c05, "republican_tradition"), (0x2c69, "save_game"), (0x2e4e, "countries"), (0x2f94, "fervor"),
    (0x3105, "doom"), (0x312c, "authority"), (0x319a, "government_rank"), (0x31bb, "development"),
    (0x31c0, "devotion"), (0x31d4, "karma"), (0x31ef, "raw_development"), (0x32b8, "displayed_country_name"),
    (0x32e8, "horde_unity"), (0x332d, "piety"), (0x33ab, "sailors"), (0x33ac, "max_sailors"), (0x33f1, "corruption"),
    (0x343c, "great_power_score"), (0x356d, "splendor"), (0x357c, "absolutism"), (0x35d4, "meritocracy"),
    (0x36d6, "army_professionalism"), (0x3707, "tribal_allegiance"), (0x37e2, "innovativeness"),
];

const HOI4_NAMES: &[(u16, &str)] = &[
    (0x001b, "name"), (0x00ee, "version"), (0x283c, "manpower"), (0x284a, "date"), (0x284b, "capital"),
    (0x2883, "stability"), (0x299f, "difficulty"), (0x2a35, "player"), (0x2d13, "ironman"), (0x2d49, "countries"),
    (0x2e3e, "ideology"), (0x2f71, "research_slot"), (0x2ffa, "politics"), (0x3008, "ruling_party"),
    (0x301e, "political_power"), (0x3392, "army_experience"), (0x3393, "navy_experience"), (0x3394, "air_experience"),
    (0x349d, "save_version"), (0x37fd, "war_support"), (0x3883, "command_power"),
];

const CK2_NAMES: &[(u16, &str)] = &[
    (0x000b, "id"), (0x001b, "name"), (0x00da, "type"), (0x010a, "score"), (0x0116, "version"), (0x01a2, "checksum"),
    (0x2712, "fertility"), (0x2717, "character"), (0x2718, "traits"), (0x2736, "dynasty"), (0x2739, "wealth"),
    (0x2740, "attributes"), (0x274e, "piety"), (0x2785, "culture"), (0x278b, "religion"), (0x279b, "date"),
    (0x28a5, "prestige"), (0x2910, "female"), (0x292d, "player"), (0x299e, "health"), (0x2afb, "player_realm"),
    (0x2c70, "birth_name"), (0x31dd, "bn"), (0x31de, "b_d"), (0x31df, "d_d"), (0x31e4, "att"), (0x31e5, "rel"),
    (0x31e6, "cul"), (0x3439, "dnt"), (0x343b, "fer"), (0x343c, "prs"), (0x344a, "fem"),
];

/// Field names by id: the built-in short list, then whatever token file the player has.
#[derive(Clone, Debug, Default)]
pub struct Names {
    map: HashMap<u16, Arc<str>>,
    /// How many names came from the player's own token file.
    pub supplied: usize,
}

impl Names {
    pub fn builtin(game: Game) -> Names {
        let list = match game {
            Game::Eu4 => EU4_NAMES,
            Game::Hoi4 => HOI4_NAMES,
            Game::Ck2 => CK2_NAMES,
            _ => &[],
        };
        Names { map: list.iter().map(|&(id, n)| (id, Arc::from(n))).collect(), supplied: 0 }
    }

    /// Adds a token file. Two layouts are read, line by line: `0x284d date` (id, then the
    /// name), and `0x4D28;date;…` (semicolons, the id's two bytes in file order).
    pub fn add_file(&mut self, text: &str) {
        for line in text.lines() {
            let line = line.trim();
            let (id, name) = if let Some((code, rest)) = line.split_once(';') {
                let name = rest.split(';').next().unwrap_or("").trim();
                let Some(le) = parse_hex(code) else { continue };
                (le.swap_bytes(), name)
            } else if let Some((code, name)) = line.split_once([' ', '\t', ',']) {
                let Some(id) = parse_hex(code) else { continue };
                (id, name.trim())
            } else {
                continue;
            };
            let plain = !name.is_empty() && name.len() <= 96 && name.bytes().all(|c| c.is_ascii_graphic() && !b"{}=\"".contains(&c));
            if plain && id > 0x0004 {
                self.map.insert(id, Arc::from(name));
                self.supplied += 1;
            }
        }
    }

    /// The built-in names plus the player's token file for this game, if there is one:
    /// the file named by `<GAME>_IRONMAN_TOKENS`, or `tokens/<game>.txt` (or `.csv`) next
    /// to the program or in its settings folder.
    pub fn for_game(game: Game) -> Names {
        let mut names = Names::builtin(game);
        #[cfg(not(target_arch = "wasm32"))]
        for path in token_paths(game) {
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.is_file() && meta.len() < 16 << 20 {
                    if let Ok(bytes) = std::fs::read(&path) {
                        names.add_file(&String::from_utf8_lossy(&bytes));
                        if names.supplied > 0 {
                            break;
                        }
                    }
                }
            }
        }
        names
    }

    fn get(&self, id: u16) -> Option<&Arc<str>> {
        self.map.get(&id)
    }

    fn id_of(&self, name: &str) -> Vec<u16> {
        self.map.iter().filter(|(_, n)| &***n == name).map(|(id, _)| *id).collect()
    }
}

fn parse_hex(s: &str) -> Option<u16> {
    let s = s.trim();
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if s.is_empty() || s.len() > 4 {
        return None;
    }
    u16::from_str_radix(s, 16).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn token_paths(game: Game) -> Vec<std::path::PathBuf> {
    let slug = game.slug();
    let mut out = vec![];
    if let Some(p) = std::env::var_os(format!("{}_IRONMAN_TOKENS", slug.to_uppercase())).filter(|v| !v.is_empty()) {
        out.push(std::path::PathBuf::from(p));
    }
    let mut dirs = vec![];
    if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("tokens"))) {
        dirs.push(d);
    }
    if let Some(d) = crate::platform::config_dir() {
        dirs.push(d.join("tokens"));
    }
    for d in dirs {
        out.push(d.join(format!("{slug}.txt")));
        out.push(d.join(format!("{slug}.csv")));
        out.push(d.join(format!("{slug}bin.csv")));
    }
    out
}

// ------------------------------------------------------------------ lexer

#[derive(Clone, Copy, Debug, PartialEq)]
enum T {
    Open,
    Close,
    Eq,
    I32,
    U32,
    I64,
    U64,
    /// A 4-byte "float": thousandths (older games) or IEEE.
    F32,
    /// An 8-byte fixed-point number.
    F64,
    Bool,
    Str,
    Empty,
    /// An index into the save's own string table, in this many bytes.
    Lookup(u8),
    /// Europa Universalis V's compact fixed-point number: this many bytes, negated or not.
    Fixed5(u8, bool),
    Rgb,
    Id(u16),
}

#[derive(Clone, Copy, Debug)]
struct Tok {
    t: T,
    /// Where the id starts and where the token's bytes end.
    at: usize,
    end: usize,
}

/// How this game writes numbers and text.
#[derive(Clone, Copy, Debug)]
struct Flavor {
    game: Game,
    /// The 4-byte float holds thousandths (EU4, CK2, older HOI4); otherwise IEEE.
    f32_thousandths: bool,
    /// Hearts of Iron IV from save version 30: the "float" id is followed by 8 bytes of
    /// hundred-thousandths.
    f32_wide: bool,
    /// What the 8-byte number counts in: 1/32768, 1/1000 or 1/100000.
    f64_div: f64,
    utf8: bool,
}

#[inline]
fn lex(d: &[u8], at: usize, fl: &Flavor) -> Option<Tok> {
    let id = u16::from_le_bytes([*d.get(at)?, *d.get(at + 1)?]);
    let p = at + 2;
    let (t, len) = match id {
        0x0001 => (T::Eq, 0),
        0x0003 => (T::Open, 0),
        0x0004 => (T::Close, 0),
        0x000c => (T::I32, 4),
        0x0014 => (T::U32, 4),
        0x029c => (T::U64, 8),
        0x0317 => (T::I64, 8),
        0x000d if fl.f32_wide => (T::F64, 8),
        0x000d => (T::F32, 4),
        0x0167 => (T::F64, 8),
        0x0190 | 0x01e7 if fl.game == Game::Ck2 => (T::F64, 8),
        0x000e => (T::Bool, 1),
        0x000f | 0x0017 | 0x0d5e => {
            let n = u16::from_le_bytes([*d.get(p)?, *d.get(p + 1)?]) as usize;
            (T::Str, 2 + n)
        }
        0x0243 => (T::Rgb, 0),
        0x0d42 => (T::Empty, 0),
        0x0d40 | 0x0d43 | 0x0d5a => (T::Lookup(1), 1),
        0x0d3e | 0x0d44 | 0x0d5b => (T::Lookup(2), 2),
        0x0d41 | 0x0d45 | 0x0d5c => (T::Lookup(3), 3),
        0x0d3f | 0x0d46 | 0x0d5d => (T::Lookup(4), 4),
        0x0d47..=0x0d55 => {
            let off = (id - 0x0d47) as u8;
            let neg = off > 7;
            let w = if neg { off - 7 } else { off };
            (T::Fixed5(w, neg), w as usize)
        }
        _ => (T::Id(id), 0),
    };
    let end = p + len;
    (end <= d.len()).then_some(Tok { t, at, end })
}

// ------------------------------------------------------------------ values

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    I32,
    U32,
    I64,
    U64,
    /// i32 thousandths.
    Milli,
    Ieee,
    /// i64 in units of 1/div.
    Fixed { div: f64 },
    /// The compact number: the whole token (id and bytes) is rewritten.
    Fixed5,
    Bool,
    Str,
    /// An i32 shown as a date (`hours`: the game also counts the hour).
    Date { hours: bool },
}

#[derive(Clone, Copy, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    kind: Kind,
}

fn num(f: f64) -> Value {
    serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number)
}

fn round_to(f: f64, places: i32) -> f64 {
    let m = 10f64.powi(places);
    (f * m).round() / m
}

fn fixed_show(raw: i64, div: f64) -> f64 {
    // 1/32768 steps print as five decimals in the games' own text saves.
    round_to(raw as f64 / div, if div == 1000.0 { 3 } else { 5 })
}

fn fixed5_raw(d: &[u8], w: u8, neg: bool) -> i64 {
    let mut b = [0u8; 8];
    b[..w as usize].copy_from_slice(d);
    let m = u64::from_le_bytes(b) as i64;
    if neg {
        -m
    } else {
        m
    }
}

fn fixed5_encode(raw: i64) -> Vec<u8> {
    let m = raw.unsigned_abs();
    let w = (8 - m.leading_zeros() as usize / 8).min(8);
    if w > 7 {
        // Too wide for the compact form: the plain 8-byte number.
        let mut out = 0x0167u16.to_le_bytes().to_vec();
        out.extend(raw.to_le_bytes());
        return out;
    }
    let id = 0x0d47 + w as u16 + if raw < 0 { 7 } else { 0 };
    let mut out = id.to_le_bytes().to_vec();
    out.extend(&m.to_le_bytes()[..w]);
    out
}

/// A date stored as hours since 1 January 5000 BC, in 365-day years.
fn date_text(v: i32, hours: bool) -> Option<String> {
    const DAYS: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if v < 0 {
        return None;
    }
    let (days, hour) = (v / 24, v % 24);
    let year = days / 365 - 5000;
    if !(-5000..=9999).contains(&year) {
        return None;
    }
    let mut doy = days % 365;
    let mut month = 1;
    for len in DAYS {
        if doy < len {
            break;
        }
        doy -= len;
        month += 1;
    }
    Some(if hours { format!("{year}.{month}.{}.{}", doy + 1, hour + 1) } else { format!("{year}.{month}.{}", doy + 1) })
}

/// The reverse of [`date_text`]: `1444.11.11` (or `1936.1.1.12` with the hour) as stored.
fn date_value(text: &str, hours: bool) -> Option<i32> {
    const DAYS: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let parts: Vec<i32> = text.trim().split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let (year, month, day, hour) = match (parts.as_slice(), hours) {
        (&[y, m, d], _) => (y, m, d, 1),
        (&[y, m, d, h], true) => (y, m, d, h),
        _ => return None,
    };
    let ok = (-5000..=9999).contains(&year) && (1..=12).contains(&month) && (1..=24).contains(&hour);
    if !ok || day < 1 || day > DAYS[month as usize - 1] {
        return None;
    }
    let doy: i32 = DAYS[..month as usize - 1].iter().sum::<i32>() + day - 1;
    Some(((year + 5000) * 365 + doy) * 24 + hour - 1)
}

fn text_of(b: &[u8], utf8: bool) -> String {
    if utf8 {
        String::from_utf8_lossy(b).into_owned()
    } else {
        b.iter().map(|&c| super::bethesda_cp1252(c)).collect()
    }
}

struct Builder<'a> {
    d: &'a [u8],
    pos: usize,
    fl: Flavor,
    names: &'a Names,
    lookup: &'a [String],
    leaves: Vec<Leaf>,
    leaf_ptrs: Vec<String>,
    editable: HashMap<String, usize>,
    focus: Vec<String>,
    budget: usize,
    /// Field ids seen without a name.
    unnamed: usize,
    named: usize,
    bad: bool,
}

impl Builder<'_> {
    fn peek(&mut self) -> Option<Tok> {
        let t = lex(self.d, self.pos, &self.fl);
        if t.is_none() && self.pos < self.d.len() {
            self.bad = true;
        }
        t
    }

    fn peek_after(&self, t: Tok) -> Option<Tok> {
        lex(self.d, t.end, &self.fl)
    }

    fn lookup_text(&self, t: Tok) -> String {
        let mut b = [0u8; 4];
        b[..t.end - t.at - 2].copy_from_slice(&self.d[t.at + 2..t.end]);
        let i = u32::from_le_bytes(b) as usize;
        self.lookup.get(i).cloned().unwrap_or_else(|| format!("#{i}"))
    }

    fn id_text(&mut self, id: u16) -> String {
        match self.names.get(id) {
            Some(n) => {
                self.named += 1;
                n.to_string()
            }
            None => {
                self.unnamed += 1;
                format!("0x{id:04x}")
            }
        }
    }

    /// A token used as a key.
    fn key_text(&mut self, t: Tok) -> String {
        let p = &self.d[t.at + 2..t.end];
        match t.t {
            T::Id(id) => self.id_text(id),
            T::Str => text_of(&p[2..], self.fl.utf8),
            T::Lookup(_) => self.lookup_text(t),
            T::I32 => i32::from_le_bytes(p.try_into().unwrap()).to_string(),
            T::U32 => u32::from_le_bytes(p.try_into().unwrap()).to_string(),
            T::I64 => i64::from_le_bytes(p.try_into().unwrap()).to_string(),
            T::U64 => u64::from_le_bytes(p.try_into().unwrap()).to_string(),
            T::Bool => (if p[0] != 0 { "yes" } else { "no" }).to_string(),
            T::Fixed5(w, neg) => (fixed5_raw(p, w, neg) as f64 / 100_000.0).to_string(),
            T::F32 | T::F64 => format!("0x{:04x}", u16::from_le_bytes([self.d[t.at], self.d[t.at + 1]])),
            T::Empty => String::new(),
            T::Open | T::Close | T::Eq | T::Rgb => "?".into(),
        }
    }

    fn leaf(&mut self, ptr: &str, start: usize, end: usize, kind: Kind) {
        self.editable.insert(ptr.to_string(), self.leaves.len());
        self.leaf_ptrs.push(ptr.to_string());
        self.leaves.push(Leaf { start, end, kind });
    }

    /// A scalar value. `key` is the name it sits under (dates are shown as dates).
    fn scalar(&mut self, t: Tok, ptr: &str, key: &str) -> Value {
        let (a, e) = (t.at + 2, t.end);
        let p = &self.d[a..e];
        match t.t {
            T::I32 => {
                let v = i32::from_le_bytes(p.try_into().unwrap());
                let dated = key == "date" || key.ends_with("_date") || key == "b_d" || key == "d_d";
                let hours = self.fl.game == Game::Hoi4;
                if let Some(s) = dated.then(|| date_text(v, hours)).flatten() {
                    // Only a date that reads back to the same number is offered for editing.
                    if date_value(&s, hours) == Some(v) {
                        self.leaf(ptr, a, e, Kind::Date { hours });
                    }
                    return Value::from(s);
                }
                self.leaf(ptr, a, e, Kind::I32);
                Value::from(v)
            }
            T::U32 => {
                self.leaf(ptr, a, e, Kind::U32);
                Value::from(u32::from_le_bytes(p.try_into().unwrap()))
            }
            T::I64 => {
                self.leaf(ptr, a, e, Kind::I64);
                Value::from(i64::from_le_bytes(p.try_into().unwrap()))
            }
            T::U64 => {
                self.leaf(ptr, a, e, Kind::U64);
                Value::from(u64::from_le_bytes(p.try_into().unwrap()))
            }
            T::F32 if self.fl.f32_thousandths => {
                self.leaf(ptr, a, e, Kind::Milli);
                num(f64::from(i32::from_le_bytes(p.try_into().unwrap())) / 1000.0)
            }
            T::F32 => {
                self.leaf(ptr, a, e, Kind::Ieee);
                let f = f32::from_le_bytes(p.try_into().unwrap());
                // The shortest decimal that reads back as the same f32.
                num(f.to_string().parse::<f64>().unwrap_or(f64::from(f)))
            }
            T::F64 => {
                // Hearts of Iron IV's wide "float" counts hundred-thousandths.
                let wide = self.fl.f32_wide && self.d[t.at] == 0x0d && self.d[t.at + 1] == 0;
                let div = if wide { 100_000.0 } else { self.fl.f64_div };
                self.leaf(ptr, a, e, Kind::Fixed { div });
                num(fixed_show(i64::from_le_bytes(p.try_into().unwrap()), div))
            }
            T::Fixed5(w, neg) => {
                self.leaf(ptr, t.at, e, Kind::Fixed5);
                num(fixed5_raw(p, w, neg) as f64 / 100_000.0)
            }
            T::Bool => {
                self.leaf(ptr, a, e, Kind::Bool);
                Value::Bool(p[0] != 0)
            }
            T::Str => {
                self.leaf(ptr, a, e, Kind::Str);
                Value::from(text_of(&p[2..], self.fl.utf8))
            }
            T::Empty => Value::from(""),
            T::Lookup(_) => Value::from(self.lookup_text(t)),
            T::Id(id) => Value::from(self.id_text(id)),
            T::Open | T::Close | T::Eq | T::Rgb => Value::Null,
        }
    }

    /// Skips a block whose `{` was just taken; returns how many values it held.
    fn skip_block(&mut self) -> usize {
        let (mut depth, mut n) = (1usize, 0usize);
        while let Some(t) = self.peek() {
            self.pos = t.end;
            match t.t {
                T::Open => depth += 1,
                T::Close => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                T::Eq | T::Rgb => {}
                _ => n += 1,
            }
        }
        n
    }

    /// Skips one value (a scalar or a whole block); returns how many values it held.
    fn skip_value(&mut self) -> usize {
        let Some(t) = self.peek() else { return 0 };
        self.pos = t.end;
        match t.t {
            T::Open => self.skip_block(),
            T::Rgb if self.peek().is_some_and(|n| n.t == T::Open) => {
                self.pos += 2;
                self.skip_block()
            }
            _ => 1,
        }
    }

    fn value(&mut self, ptr: &str, key: &str) -> Value {
        let Some(t) = self.peek() else { return Value::Null };
        self.pos = t.end;
        match t.t {
            T::Open => self.block(ptr, false),
            T::Rgb if self.peek().is_some_and(|n| n.t == T::Open) => {
                self.pos += 2;
                self.block(ptr, false)
            }
            T::Close => {
                // `key=` with nothing after it: leave the brace for the block.
                self.pos = t.at;
                Value::Null
            }
            T::Eq => Value::Null,
            _ => self.scalar(t, ptr, key),
        }
    }

    fn focused(&self, ptr: &str) -> bool {
        self.focus.iter().any(|f| ptr == f || ptr.starts_with(&format!("{f}/")) || f.starts_with(&format!("{ptr}/")))
    }

    /// The items of a block (its `{` already taken), up to its `}` (or the end of the data
    /// for the top level).
    fn block(&mut self, ptr: &str, top: bool) -> Value {
        let mut kv = Map::new();
        let mut list: Vec<Value> = vec![];
        let mut hidden = 0usize;
        let mut item_leaves: Vec<usize> = vec![];
        loop {
            let Some(t) = self.peek() else { break };
            if t.t == T::Close {
                self.pos = t.end;
                if top {
                    continue;
                }
                break;
            }
            if t.t == T::Eq {
                self.pos = t.end;
                continue;
            }
            let keyed = !matches!(t.t, T::Open | T::Rgb) && self.peek_after(t).is_some_and(|n| n.t == T::Eq);
            if top {
                self.budget = SECTION_BUDGET;
            }
            if keyed {
                let name = self.key_text(t);
                let key = unique_key(&kv, &name);
                let p = format!("{ptr}/{}", esc(&key));
                self.pos = t.end + 2;
                let focus = self.focused(&p);
                if !focus && self.budget == 0 {
                    hidden += 1 + self.skip_value().saturating_sub(1);
                    continue;
                }
                let saved = self.budget;
                if focus {
                    self.budget = FOCUS_BUDGET;
                }
                self.budget = self.budget.saturating_sub(1);
                let v = self.value(&p, &name);
                if focus {
                    self.budget = saved;
                }
                kv.insert(key, v);
            } else {
                let p = format!("{ptr}/{}", list.len());
                item_leaves.push(self.leaves.len());
                if self.budget == 0 && !self.focused(&p) {
                    hidden += self.skip_value().max(1);
                    list.push(Value::Null);
                    continue;
                }
                self.budget = self.budget.saturating_sub(1);
                let v = self.value(&p, "");
                list.push(v);
            }
        }
        if hidden > 0 {
            let key = unique_key(&kv, "(not shown)");
            kv.insert(key, Value::from(format!("{hidden} more values \u{2014} not listed here")));
        }
        if kv.is_empty() {
            return Value::Array(list);
        }
        if !list.is_empty() {
            // Keys and plain values together: keep the plain ones under numbered keys.
            let end = self.leaves.len();
            for (i, v) in list.into_iter().enumerate() {
                let key = unique_key(&kv, &format!("#{}", i + 1));
                let (from, to) = (format!("{ptr}/{i}"), format!("{ptr}/{}", esc(&key)));
                let last = item_leaves.get(i + 1).copied().unwrap_or(end);
                for leaf in item_leaves[i]..last {
                    let old = &self.leaf_ptrs[leaf];
                    if old == &from || old.starts_with(&format!("{from}/")) {
                        let new = format!("{to}{}", &old[from.len()..]);
                        self.editable.remove(old);
                        self.editable.insert(new.clone(), leaf);
                        self.leaf_ptrs[leaf] = new;
                    }
                }
                kv.insert(key, v);
            }
        }
        Value::Object(kv)
    }
}

// ------------------------------------------------------------------ backend

#[derive(Clone, Debug)]
struct BinBackend {
    data: Arc<Vec<u8>>,
    leaves: Vec<Leaf>,
    utf8: bool,
    /// An uncompressed `SAV…` file: where the header's metadata length is written (8 hex
    /// digits) and where the metadata ends.
    meta: Option<(usize, usize)>,
}

impl BinBackend {
    fn encode(&self, leaf: &Leaf, v: &Value) -> Result<Vec<u8>, String> {
        use crate::tree::int_in;
        let float = || v.as_f64().filter(|f| f.is_finite()).ok_or_else(|| "expected a number".to_string());
        let scaled = |div: f64, min: f64, max: f64| -> Result<f64, String> {
            let raw = (float()? * div).round();
            if raw < min || raw > max {
                return Err("that number is too large for this value".into());
            }
            Ok(raw)
        };
        Ok(match leaf.kind {
            Kind::I32 => (int_in(v, i32::MIN.into(), i32::MAX.into(), "This value")? as i32).to_le_bytes().to_vec(),
            Kind::U32 => (int_in(v, 0, u32::MAX.into(), "This value")? as u32).to_le_bytes().to_vec(),
            Kind::I64 => (int_in(v, i64::MIN.into(), i64::MAX.into(), "This value")? as i64).to_le_bytes().to_vec(),
            Kind::U64 => (int_in(v, 0, u64::MAX.into(), "This value")? as u64).to_le_bytes().to_vec(),
            Kind::Milli => (scaled(1000.0, i32::MIN.into(), i32::MAX.into())? as i32).to_le_bytes().to_vec(),
            Kind::Ieee => {
                let f = float()? as f32;
                if !f.is_finite() {
                    return Err("that number is too large for this value".into());
                }
                f.to_le_bytes().to_vec()
            }
            Kind::Fixed { div } => (scaled(div, -9.0e18, 9.0e18)? as i64).to_le_bytes().to_vec(),
            Kind::Fixed5 => fixed5_encode(scaled(100_000.0, -9.0e18, 9.0e18)? as i64),
            Kind::Bool => vec![u8::from(v.as_bool().ok_or("tick or untick it")?)],
            Kind::Date { hours } => {
                let wanted = if hours { "a date like 1936.1.1.12 (year.month.day.hour)" } else { "a date like 1444.11.11 (year.month.day)" };
                v.as_str().and_then(|s| date_value(s, hours)).ok_or_else(|| format!("expected {wanted}"))?.to_le_bytes().to_vec()
            }
            Kind::Str => {
                let s = v.as_str().ok_or("expected text")?;
                let bytes: Vec<u8> = if self.utf8 {
                    s.as_bytes().to_vec()
                } else {
                    s.chars().map(|c| super::bethesda_cp1252_encode(c).ok_or_else(|| format!("this save can't hold the character {c:?}"))).collect::<Result<_, _>>()?
                };
                if bytes.len() > u16::MAX as usize {
                    return Err("that text is too long".into());
                }
                let mut out = (bytes.len() as u16).to_le_bytes().to_vec();
                out.extend(bytes);
                out
            }
        })
    }
}

impl Backend for BinBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.data.as_ref().clone());
        }
        let mut edits: Vec<(Leaf, Vec<u8>)> = changes.iter().map(|(l, v)| Ok((self.leaves[*l], self.encode(&self.leaves[*l], v)?))).collect::<Result<_, String>>()?;
        edits.sort_by_key(|e| e.0.start);
        let mut out = Vec::with_capacity(self.data.len() + 64);
        let (mut at, mut meta_delta) = (0usize, 0i64);
        for (leaf, bytes) in edits {
            if self.meta.is_some_and(|(_, end)| leaf.start < end) {
                meta_delta += bytes.len() as i64 - (leaf.end - leaf.start) as i64;
            }
            out.extend_from_slice(&self.data[at..leaf.start]);
            out.extend(bytes);
            at = leaf.end;
        }
        out.extend_from_slice(&self.data[at..]);
        if let (Some((field, _)), true) = (self.meta, meta_delta != 0) {
            let old = std::str::from_utf8(&out[field..field + 8]).ok().and_then(|h| i64::from_str_radix(h, 16).ok()).ok_or("the save's header can't be read")?;
            out[field..field + 8].copy_from_slice(format!("{:08x}", old + meta_delta).as_bytes());
        }
        Ok(out)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if let Kind::Date { hours } = self.leaves[leaf].kind {
            return match v.as_str().and_then(|s| date_value(s, hours)).and_then(|n| date_text(n, hours)) {
                Some(s) => Value::from(s),
                None => v,
            };
        }
        let Some(f) = v.as_f64() else { return v };
        match self.leaves[leaf].kind {
            Kind::Milli => num(round_to(f, 3)),
            Kind::Ieee => num((f as f32).to_string().parse::<f64>().unwrap_or(f)),
            Kind::Fixed { div } => num(fixed_show((f * div).round() as i64, div)),
            Kind::Fixed5 => num(round_to(f, 5)),
            _ => v,
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        self.encode(&self.leaves[leaf], v).map(|_| ())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

// ------------------------------------------------------------------ containers

/// The first line of a `SAV…` file: `SAV`, version, kind, 8 random digits, the metadata
/// length (all hex), sometimes 8 more digits, and a newline.
#[derive(Clone, Copy, Debug)]
pub struct SavHeader {
    pub kind: u8,
    /// Bytes in the header line, newline included.
    pub len: usize,
    pub meta_len: usize,
}

impl SavHeader {
    pub fn read(d: &[u8]) -> Option<SavHeader> {
        if d.len() < 24 || !d.starts_with(b"SAV") || !d[3..23].iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        let hex = |r: std::ops::Range<usize>| usize::from_str_radix(std::str::from_utf8(&d[r]).ok()?, 16).ok();
        let nl = d[23..d.len().min(64)].iter().position(|&c| c == b'\n')? + 23;
        Some(SavHeader { kind: hex(5..7)? as u8, len: nl + 1, meta_len: hex(15..23)? })
    }

    /// Kinds 1, 3 and 5 hold binary data; 0, 2 and 4 text.
    pub fn binary(&self) -> bool {
        self.kind % 2 == 1
    }

    /// Kinds 2 to 5 end in a zip archive that holds the game state.
    pub fn zipped(&self) -> bool {
        self.kind >= 2
    }
}

/// Binary saves this module opens directly: Europa Universalis IV, Crusader Kings II and
/// Hearts of Iron IV data (also as found inside their zip archives), and uncompressed
/// binary `SAV…` files.
pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"EU4bin") || d.starts_with(b"CK2bin") || d.starts_with(b"HOI4bin") || SavHeader::read(d).is_some_and(|h| h.kind == 1)
}

/// The string table Europa Universalis V keeps beside the game state: five bytes, then
/// strings after their 16-bit lengths.
pub fn string_lookup(d: &[u8]) -> Vec<String> {
    let mut out = vec![];
    let mut at = 5;
    while at + 2 <= d.len() {
        let n = u16::from_le_bytes([d[at], d[at + 1]]) as usize;
        let Some(s) = d.get(at + 2..at + 2 + n) else { break };
        out.push(String::from_utf8_lossy(s).into_owned());
        at += 2 + n;
    }
    out
}

/// `old` is a run of whole tokens; `new` starts with the same tokens after an edit (values
/// may have changed size). How many bytes of `new` they take.
pub fn token_prefix_len(old: &[u8], new: &[u8]) -> Option<usize> {
    let fl = flavor(Game::Unknown, &[]);
    let (mut a, mut b) = (0, 0);
    while a < old.len() {
        let (x, y) = (lex(old, a, &fl)?, lex(new, b, &fl)?);
        // The same field: only a compact number may change its id.
        if x.t != y.t && !matches!((x.t, y.t), (T::Fixed5(..) | T::F64, T::Fixed5(..) | T::F64)) {
            return None;
        }
        a = x.end;
        b = y.end;
    }
    (a == old.len()).then_some(b)
}

fn flavor(game: Game, body: &[u8]) -> Flavor {
    let old = matches!(game, Game::Eu4 | Game::Ck2 | Game::Hoi4);
    let mut fl = Flavor { game, f32_thousandths: old, f32_wide: false, f64_div: if old { 32768.0 } else { 100_000.0 }, utf8: !matches!(game, Game::Eu4 | Game::Ck2) };
    if game == Game::Hoi4 {
        // `save_version=<i32>` near the top says which float encoding follows.
        let head = &body[..body.len().min(1 << 16)];
        if let Some(i) = head.windows(6).position(|w| w == [0x9d, 0x34, 0x01, 0x00, 0x0c, 0x00]) {
            if let Some(v) = head.get(i + 6..i + 10) {
                fl.f32_wide = i32::from_le_bytes(v.try_into().unwrap()) >= 30;
            }
        }
    }
    if game == Game::Ck3 {
        // `meta_data={ version=<i32>`: saves before 1.5 (version 5 and lower) count thousandths.
        if body.get(10..12) == Some(&[0x0c, 0x00]) {
            if let Some(v) = body.get(12..16) {
                if i32::from_le_bytes(v.try_into().unwrap()) <= 5 {
                    fl.f64_div = 1000.0;
                }
            }
        }
    }
    fl
}

/// The player's own part of the save, shown in full: `player="TAG"` → /countries/TAG;
/// Crusader Kings II `player={ id=N … }` → /character/N.
fn focus_paths(d: &[u8], start: usize, fl: &Flavor, names: &Names) -> Vec<String> {
    let player = names.id_of("player");
    let ids = names.id_of("id");
    let mut at = start;
    for _ in 0..4096 {
        let Some(t) = lex(d, at, fl) else { break };
        at = t.end;
        let T::Id(id) = t.t else { continue };
        if !player.contains(&id) {
            continue;
        }
        let Some(eq) = lex(d, at, fl).filter(|n| n.t == T::Eq) else { continue };
        let Some(v) = lex(d, eq.end, fl) else { break };
        match v.t {
            T::Str => return vec![format!("/countries/{}", esc(&text_of(&d[v.at + 4..v.end], fl.utf8)))],
            T::Open => {
                let mut p = v.end;
                for _ in 0..16 {
                    let Some(k) = lex(d, p, fl) else { break };
                    p = k.end;
                    if matches!(k.t, T::Id(i) if ids.contains(&i)) {
                        if let (Some(e), Some(n)) = (lex(d, p, fl), lex(d, p + 2, fl)) {
                            if e.t == T::Eq && matches!(n.t, T::I32 | T::U32) {
                                let n = u32::from_le_bytes(d[n.at + 2..n.end].try_into().unwrap());
                                return vec![format!("/character/{n}")];
                            }
                        }
                    }
                    if k.t == T::Close {
                        break;
                    }
                }
            }
            _ => {}
        }
        break;
    }
    vec![]
}

/// Opens binary save data. `game`: what the file is, when the caller knows (a `SAV…` file
/// doesn't say). `lookup`: Europa Universalis V's string table.
pub fn parse_as(d: &[u8], game: Game, lookup: &[String]) -> Result<TreeDoc, String> {
    parse_full(d, game, lookup, &[])
}

/// Opens the `gamestate` of a zipped save whose `meta` entry (binary too) holds the date
/// and the player: both are shown, read-only, and the player's country is shown in full.
pub fn parse_with_meta(d: &[u8], meta: &[u8]) -> Result<TreeDoc, String> {
    parse_full(d, Game::Unknown, &[], meta)
}

fn parse_full(d: &[u8], game: Game, lookup: &[String], side: &[u8]) -> Result<TreeDoc, String> {
    let (game, start, meta) = if d.starts_with(b"EU4bin") {
        (Game::Eu4, 6, None)
    } else if d.starts_with(b"CK2bin") {
        (Game::Ck2, 6, None)
    } else if d.starts_with(b"HOI4bin") {
        (Game::Hoi4, 7, None)
    } else if let Some(h) = SavHeader::read(d).filter(|h| h.kind == 1) {
        (game, h.len, Some((15, h.len + h.meta_len)))
    } else {
        (game, 0, None)
    };
    let game = if game == Game::Unknown { Game::guess(&d[start..]) } else { game };
    let fl = flavor(game, &d[start..]);
    let names = Names::for_game(game);
    let mut focus = focus_paths(d, start, &fl, &names);
    let mut beside = Map::new();
    if side.len() > start && side[..start] == d[..start] {
        let mut m = Builder { d: side, pos: start, fl, names: &names, lookup, leaves: vec![], leaf_ptrs: vec![], editable: HashMap::new(), focus: vec![], budget: SECTION_BUDGET, unnamed: 0, named: 0, bad: false };
        if let Value::Object(top) = m.block("", true) {
            beside = top;
        }
        if focus.is_empty() {
            focus = focus_paths(side, start, &fl, &names);
        }
    }
    let mut b = Builder { d, pos: start, fl, names: &names, lookup, leaves: vec![], leaf_ptrs: vec![], editable: HashMap::new(), focus, budget: SECTION_BUDGET, unnamed: 0, named: 0, bad: false };
    let mut view = b.block("", true);
    if b.bad || view.as_object().is_none_or(|m| m.len() < 2) {
        return Err("not a Paradox binary save".into());
    }
    if let Value::Object(top) = &mut view {
        for key in ["date", "player", "displayed_country_name"] {
            if let (Some(v), false) = (beside.get(key), top.contains_key(key)) {
                top.insert(key.into(), v.clone());
            }
        }
    }
    let mut note = String::from(
        "Binary (ironman) save: values are changed in place and the file stays a binary save. The game is likely to \
         notice the edit and stop awarding achievements for this campaign. ",
    );
    if b.unnamed > 0 && names.supplied == 0 {
        note.push_str(&format!(
            "The game doesn't publish its field names, so most fields show as numbers (0x\u{2026}); the Values tab lists them all, with your own country in full. \
             If you have a token file for this game, put it in a \"tokens\" folder next to the program as {}.txt and every name it lists is shown. ",
            game.slug()
        ));
    } else if names.supplied > 0 {
        note.push_str(&format!("Field names come from your token file ({} names). ", names.supplied));
    }
    note.push_str("Big sections show their first few thousand values; your own country is shown in full.");
    let backend = BinBackend { data: Arc::new(d.to_vec()), leaves: b.leaves, utf8: fl.utf8, meta };
    Ok(TreeDoc::new(view, b.editable, game.kind_name(), Box::new(backend)).with_note(note))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    parse_as(d, Game::Unknown, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u16) -> Vec<u8> {
        n.to_le_bytes().to_vec()
    }

    fn s(text: &str) -> Vec<u8> {
        [id(0x000f), (text.len() as u16).to_le_bytes().to_vec(), text.as_bytes().to_vec()].concat()
    }

    fn i32v(v: i32) -> Vec<u8> {
        [id(0x000c), v.to_le_bytes().to_vec()].concat()
    }

    const EQ: u16 = 1;
    const OPEN: u16 = 3;
    const CLOSE: u16 = 4;

    /// date=1444.11.11 player="FRA" countries={ FRA={ treasury=12.345 0x3f00=7 powers={ 1 2 3 } } }
    fn eu4() -> Vec<u8> {
        let date = (((1444 + 5000) * 365 + 314) * 24) as i32;
        [
            b"EU4bin".to_vec(),
            id(0x284d), id(EQ), i32v(date),
            id(0x2a38), id(EQ), s("FRA"),
            id(0x2e4e), id(EQ), id(OPEN),
            [id(0x0017), 3u16.to_le_bytes().to_vec(), b"FRA".to_vec()].concat(), id(EQ), id(OPEN),
            id(0x2876), id(EQ), id(0x000d), 12345i32.to_le_bytes().to_vec(),
            id(0x3f00), id(EQ), i32v(7),
            id(0x2ab2), id(EQ), id(OPEN), i32v(1), i32v(2), i32v(3), id(CLOSE),
            id(0x2bfd), id(EQ), id(0x0167), (32768i64 * 5 / 2).to_le_bytes().to_vec(),
            id(CLOSE), id(CLOSE),
        ]
        .concat()
    }

    #[test]
    fn reads_and_edits_in_place() {
        let d = eu4();
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Europa Universalis IV save (binary)");
        assert_eq!(doc.view["date"], "1444.11.11");
        assert_eq!(doc.view["player"], "FRA");
        let c = &doc.view["countries"]["FRA"];
        assert_eq!(c["treasury"], 12.345);
        assert_eq!(c["0x3f00"], 7);
        assert_eq!(c["powers"][2], 3);
        assert_eq!(c["mercantilism"], 2.5);
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/countries/FRA/treasury", Value::from(99999.5)).unwrap();
        doc.set("/countries/FRA/powers/1", Value::from(999)).unwrap();
        doc.set("/countries/FRA/mercantilism", Value::from(40)).unwrap();
        doc.set("/player", Value::from("ENGL")).unwrap();
        let out = doc.to_bytes().unwrap();
        // One string grew by a byte; nothing else moved.
        assert_eq!(out.len(), d.len() + 1);
        let back = parse(&out).unwrap();
        assert_eq!(back.view, doc.view);
        assert_eq!(back.view["countries"]["FRA"]["treasury"], 99999.5);
        assert_eq!(back.view["countries"]["FRA"]["mercantilism"], 40.0);
        assert!(doc.set("/countries/FRA/powers/0", Value::from(1i64 << 40)).is_err());
    }

    #[test]
    fn token_files() {
        let mut n = Names::builtin(Game::Eu4);
        n.add_file("0x3f00 my_field\n0x4D28;renamed_date;Date;no;\nnot a line\n0x0001 =\n");
        assert_eq!(n.supplied, 2);
        assert_eq!(&**n.get(0x3f00).unwrap(), "my_field");
        assert_eq!(&**n.get(0x284d).unwrap(), "renamed_date");
        assert!(n.get(0x0001).is_none());
    }

    #[test]
    fn compact_numbers() {
        for raw in [0i64, 1, 255, 256, -1, -70000, 123_456_789_012, -(1 << 55), 1 << 60] {
            let enc = fixed5_encode(raw);
            let fl = flavor(Game::Eu5, &[]);
            let t = lex(&enc, 0, &fl).unwrap();
            assert_eq!(t.end, enc.len());
            let got = match t.t {
                T::Fixed5(w, neg) => fixed5_raw(&enc[2..], w, neg),
                T::F64 => i64::from_le_bytes(enc[2..].try_into().unwrap()),
                other => panic!("{other:?}"),
            };
            assert_eq!(got, raw);
        }
    }

    #[test]
    fn sav_header_and_metadata_length() {
        // An uncompressed binary SAV file whose metadata holds a string that is edited.
        let meta = [id(0x3155), id(EQ), id(OPEN), id(0x058f), id(EQ), i32v(9), id(0x001c), id(EQ), s("Ada"), id(CLOSE)].concat();
        let body = [id(0x7000), id(EQ), i32v(5)].concat();
        let d = [format!("SAV0101aabbccdd{:08x}\n", meta.len()).into_bytes(), meta.clone(), body].concat();
        let h = SavHeader::read(&d).unwrap();
        assert!(h.binary() && !h.zipped());
        assert_eq!((h.len, h.meta_len), (24, meta.len()));
        let mut doc = parse_as(&d, Game::Ck3, &[]).unwrap();
        assert_eq!(doc.view["0x3155"]["0x001c"], "Ada");
        doc.set("/0x3155/0x001c", Value::from("Adelaide")).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(SavHeader::read(&out).unwrap().meta_len, meta.len() + 5);
        assert_eq!(parse_as(&out, Game::Ck3, &[]).unwrap().view["0x7000"], 5);
    }

    #[test]
    fn dates() {
        assert_eq!(date_text(56379360, false).as_deref(), Some("1436.1.1"));
        assert_eq!(date_text(60759371, true).as_deref(), Some("1936.1.1.12"));
        assert_eq!(date_value("1436.1.1", false), Some(56379360));
        assert_eq!(date_value("1936.1.1.12", true), Some(60759371));
        assert_eq!(date_value(" 1444.11.11 ", false), Some(((1444 + 5000) * 365 + 314) * 24));
        for bad in ["1444.13.1", "1444.2.29", "1444.11", "soon", "1444.11.11.5"] {
            assert_eq!(date_value(bad, false), None, "{bad}");
        }
    }

    /// Research aid: opens PDXBIN_SAVE (binary data, not a zip) and prints what it finds.
    #[test]
    #[ignore]
    fn real_save() {
        let path = std::env::var("PDXBIN_SAVE").expect("PDXBIN_SAVE");
        let d = std::fs::read(&path).unwrap();
        let game = Game::from_ext(&std::env::var("PDXBIN_GAME").unwrap_or_default());
        let lookup = std::env::var("PDXBIN_LOOKUP").ok().map(|p| string_lookup(&std::fs::read(p).unwrap())).unwrap_or_default();
        let t = std::time::Instant::now();
        let doc = parse_as(&d, game, &lookup).unwrap();
        println!("{} in {:?}: {} editable, top {:?}", doc.kind_name, t.elapsed(), doc.editable.len(), doc.view.as_object().unwrap().keys().take(30).collect::<Vec<_>>());
        assert_eq!(doc.to_bytes().unwrap(), d);
        for p in std::env::var("PDXBIN_SHOW").unwrap_or_default().split(',').filter(|p| !p.is_empty()) {
            let s = doc.view.pointer(p).cloned().unwrap_or_default().to_string();
            println!("{p}: {}", &s[..s.len().min(1500)]);
        }
    }
}
