//! Pokémon Red / Blue / Yellow (Generation I) and Gold / Silver / Crystal
//! (Generation II) battery saves (`.sav`, 32 KiB), English releases.
//!
//! Layouts follow Bulbapedia's save-structure articles and PKHeX. Everything is
//! big-endian; text uses the Game Boy character set (0x50 ends a name). Gen I keeps
//! an 8-bit checksum over the main data plus per-bank and per-box checksums for the
//! PC; Gen II keeps a 16-bit sum and a second copy of the main data. All of that is
//! rewritten on save.

use crate::tree::{esc, int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

const NONE: &str = "(none)";
const TERM: u8 = 0x50;

// ------------------------------------------------------------------ game data

struct Tables {
    /// Gen I: internal species index -> name, Pokédex number.
    g1_names: Vec<String>,
    g1_dex: Vec<usize>,
    /// Gen I base data by Pokédex number: hp, atk, def, spe, spc, type1, type2, catch rate, growth.
    g1_base: Vec<Option<[u16; 9]>>,
    g1_items: Vec<String>,
    /// Gen II base data by Pokédex number: hp, atk, def, spe, spa, spd, type1, type2, growth, gender ratio.
    g2_base: Vec<Option<[u16; 10]>>,
    g2_items: Vec<String>,
    /// Gen II species names by Pokédex number (shared with Gen III's first 251).
    g2_names: Vec<String>,
    moves: Vec<String>,
    types: Vec<String>,
    g1_species_list: Arc<Vec<String>>,
    g2_species_list: Arc<Vec<String>>,
    g1_item_list: Arc<Vec<String>>,
    g2_item_list: Arc<Vec<String>>,
    g1_move_list: Arc<Vec<String>>,
    g2_move_list: Arc<Vec<String>>,
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let src = include_str!("data/gen12.txt");
        let line = |k: &str| src.lines().find_map(|l| l.strip_prefix(k)).unwrap_or("");
        let split = |s: &str| s.split('|').map(str::to_string).collect::<Vec<_>>();
        fn base<const N: usize>(s: &str) -> Vec<Option<[u16; N]>> {
            s.split(';')
                .map(|e| {
                    let v: Vec<u16> = e.split(',').filter_map(|x| x.parse().ok()).collect();
                    (v.len() == N).then(|| v.try_into().unwrap())
                })
                .collect()
        }
        let g3 = super::pokemon3::tables();
        let mut moves: Vec<String> = g3.moves.iter().take(252).cloned().collect();
        moves[0] = String::new();
        let g2_names: Vec<String> = g3.species.iter().take(252).cloned().collect();
        let g1_names = split(line("G1SP="));
        let g1_dex: Vec<usize> = line("G1DEX=")
            .split(',')
            .map(|x| x.parse().unwrap_or(0))
            .collect();
        let list = |v: &[String], none: bool| {
            let mut l = if none { vec![NONE.to_string()] } else { vec![] };
            let mut seen = std::collections::HashSet::new();
            l.extend(
                v.iter()
                    .filter(|s| !s.is_empty() && seen.insert((*s).clone()))
                    .cloned(),
            );
            Arc::new(l)
        };
        let g1_items = split(line("G1IT="));
        let g2_items = split(line("G2IT="));
        // Gen I's species list in Pokédex order (its internal order is scrambled).
        let mut by_dex: Vec<(usize, String)> = g1_names
            .iter()
            .enumerate()
            .filter(|(i, n)| !n.is_empty() && g1_dex.get(*i).is_some_and(|&d| d > 0))
            .map(|(i, n)| (g1_dex[i], n.clone()))
            .collect();
        by_dex.sort();
        let g1_species: Vec<String> = by_dex.into_iter().map(|(_, n)| n).collect();
        Tables {
            g1_species_list: Arc::new(g1_species),
            g2_species_list: list(&g2_names[1..], false),
            g1_item_list: list(
                &g1_items
                    .iter()
                    .map(|n| {
                        if g1_not_an_item(n) {
                            String::new()
                        } else {
                            n.clone()
                        }
                    })
                    .collect::<Vec<_>>(),
                true,
            ),
            g2_item_list: list(&g2_items, true),
            g1_move_list: list(&moves[..166], true),
            g2_move_list: list(&moves, true),
            g1_names,
            g1_dex,
            g1_base: base::<9>(line("G1BASE=")),
            g1_items,
            g2_base: base::<10>(line("G2BASE=")),
            g2_items,
            g2_names,
            moves,
            types: split(line("TYPES=")),
        }
    })
}

/// "Grass / Poison", or one name when both types are the same.
fn type_text(t1: u8, t2: u8) -> String {
    let t = tables();
    let tn = |x: u8| {
        t.types
            .get(x as usize)
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("#{x}"))
    };
    if t1 == t2 {
        tn(t1)
    } else {
        format!("{} / {}", tn(t1), tn(t2))
    }
}

/// Gen I item ids that aren't real items: badges and the elevator's floor labels.
fn g1_not_an_item(name: &str) -> bool {
    let n = name.to_lowercase();
    let floor = n
        .strip_prefix('b')
        .unwrap_or(&n)
        .strip_suffix('f')
        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()));
    n.ends_with("badge") || floor
}

fn name_of(table: &[String], i: usize) -> String {
    match table.get(i) {
        _ if i == 0 => NONE.into(),
        Some(s) if !s.is_empty() => s.clone(),
        _ => format!("#{i}"),
    }
}

fn index_of(table: &[String], name: &str) -> Option<usize> {
    let n = name.trim();
    if n.is_empty() || n.eq_ignore_ascii_case(NONE) {
        return Some(0);
    }
    if let Some(num) = n.strip_prefix('#') {
        return num.parse().ok().filter(|&i: &usize| i < 256);
    }
    table
        .iter()
        .position(|s| !s.is_empty() && s.eq_ignore_ascii_case(n))
}

/// Experience needed for a level (Gen I/II growth formulas).
fn exp_for(growth: u16, n: u32) -> u32 {
    if n <= 1 {
        return 0;
    }
    let n = n as i64;
    let c = n * n * n;
    let e = match growth {
        1 => 3 * c / 4 + 10 * n * n - 30,
        2 => 3 * c / 4 + 20 * n * n - 70,
        3 => 6 * c / 5 - 15 * n * n + 100 * n - 140,
        4 => 4 * c / 5,
        5 => 5 * c / 4,
        _ => c,
    };
    e.max(0) as u32
}

fn level_for(growth: u16, exp: u32) -> u32 {
    let mut lv = 1;
    while lv < 100 && exp_for(growth, lv + 1) <= exp {
        lv += 1;
    }
    lv
}

/// Stat experience's contribution to a stat: ⌈√exp⌉ / 4, capped at 63.
fn ev_term(stat_exp: u32) -> u32 {
    let mut r = (stat_exp as f64).sqrt().ceil() as u32;
    if r * r < stat_exp {
        r += 1;
    }
    r.min(255) / 4
}

fn gb_stat(base: u32, iv: u32, stat_exp: u32, level: u32, hp: bool) -> u32 {
    let core = ((base + iv) * 2 + ev_term(stat_exp)) * level / 100;
    if hp {
        core + level + 10
    } else {
        core + 5
    }
}

// ------------------------------------------------------------------ text

fn charset(g2: bool) -> [Option<char>; 256] {
    let mut t = [None; 256];
    t[0x7F] = Some(' ');
    for i in 0..26u8 {
        t[0x80 + i as usize] = Some((b'A' + i) as char);
        t[0xA0 + i as usize] = Some((b'a' + i) as char);
    }
    for (i, c) in "():;[]".chars().enumerate() {
        t[0x9A + i] = Some(c);
    }
    for i in 0..10u8 {
        t[0xF6 + i as usize] = Some((b'0' + i) as char);
    }
    for (b, c) in [
        (0xE0, '\''),
        (0xE3, '-'),
        (0xE6, '?'),
        (0xE7, '!'),
        (0xE8, '.'),
        (0xEF, '♂'),
        (0xF1, '×'),
        (0xF3, '/'),
        (0xF4, ','),
        (0xF5, '♀'),
    ] {
        t[b] = Some(c);
    }
    if g2 {
        t[0xEA] = Some('é');
        t[0xE9] = Some('&');
    } else {
        t[0xBA] = Some('é');
    }
    t
}

/// Contractions the games store as one character (e.g. a nickname ending in 's).
fn ligatures(g2: bool) -> &'static [(u8, &'static str)] {
    if g2 {
        &[
            (0xD0, "'d"),
            (0xD1, "'l"),
            (0xD2, "'m"),
            (0xD3, "'r"),
            (0xD4, "'s"),
            (0xD5, "'t"),
            (0xD6, "'v"),
        ]
    } else {
        &[
            (0xBB, "'d"),
            (0xBC, "'l"),
            (0xBD, "'s"),
            (0xBE, "'t"),
            (0xBF, "'v"),
            (0xE4, "'r"),
            (0xE5, "'m"),
        ]
    }
}

fn decode_text(b: &[u8], g2: bool) -> Option<String> {
    let cs = charset(g2);
    let mut s = String::new();
    for &c in b {
        if c == TERM {
            return Some(s);
        }
        match ligatures(g2).iter().find(|l| l.0 == c) {
            Some((_, t)) => s.push_str(t),
            None => s.push(cs[c as usize]?),
        }
    }
    Some(s)
}

/// Encode into a `len`-byte field: the text, then 0x50 up to `fill` bytes; bytes after
/// `fill` keep their original value.
fn encode_text(
    s: &str,
    len: usize,
    fill: usize,
    max: usize,
    orig: &[u8],
    g2: bool,
) -> Result<Vec<u8>, String> {
    let cs = charset(g2);
    let mut out = Vec::with_capacity(len);
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let pair: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        if let Some((b, _)) = ligatures(g2).iter().find(|l| l.1 == pair) {
            out.push(*b);
            i += 2;
            continue;
        }
        let ch = chars[i];
        let b = cs
            .iter()
            .position(|&c| c == Some(ch))
            .ok_or_else(|| format!("the game can't show \u{201C}{ch}\u{201D}"))?;
        out.push(b as u8);
        i += 1;
    }
    if out.is_empty() {
        return Err("a name can't be empty".into());
    }
    if out.len() > max {
        return Err(format!("names can be at most {max} characters"));
    }
    out.resize(fill.max(out.len() + 1).min(len), TERM);
    out.extend_from_slice(&orig[out.len().min(orig.len())..len.min(orig.len())]);
    out.resize(len, TERM);
    Ok(out)
}

// ------------------------------------------------------------------ layouts

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Game {
    RedBlue,
    Yellow,
    GoldSilver,
    Crystal,
}

impl Game {
    fn g2(self) -> bool {
        matches!(self, Game::GoldSilver | Game::Crystal)
    }
    fn name(self) -> &'static str {
        match self {
            Game::RedBlue => "Pokémon Red / Blue",
            Game::Yellow => "Pokémon Yellow",
            Game::GoldSilver => "Pokémon Gold / Silver",
            Game::Crystal => "Pokémon Crystal",
        }
    }
    /// Stored / party Pokémon record sizes.
    fn sizes(self) -> (usize, usize) {
        if self.g2() {
            (32, 48)
        } else {
            (33, 44)
        }
    }
}

fn sum8(d: &[u8]) -> u8 {
    !d.iter().fold(0u8, |a, &b| a.wrapping_add(b))
}

fn sum16(d: &[u8]) -> u16 {
    d.iter().fold(0u16, |a, &b| a.wrapping_add(b as u16))
}

fn be(d: &[u8]) -> u32 {
    d.iter().fold(0, |a, &b| (a << 8) | b as u32)
}

fn bcd(d: &[u8]) -> Option<u32> {
    d.iter().try_fold(0u32, |a, &b| {
        ((b >> 4) < 10 && (b & 15) < 10).then(|| a * 100 + (b >> 4) as u32 * 10 + (b & 15) as u32)
    })
}

fn to_bcd(mut n: u32, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    for b in out.iter_mut().rev() {
        *b = ((n / 10 % 10) << 4 | n % 10) as u8;
        n /= 100;
    }
    out
}

const G1_MAIN: (usize, usize) = (0x2598, 0x3523);
const G2_GS_SUM: (usize, usize, usize, usize) = (0x2009, 0x2D68, 0x2D69, 0x7E6D);
const G2_C_SUM: (usize, usize, usize, usize) = (0x2009, 0x2B82, 0x2D0D, 0x1F0D);

fn detect(d: &[u8]) -> Option<Game> {
    if !(0x8000..=0x8000 + 0x40).contains(&d.len()) {
        return None;
    }
    let crystal = u16::from_le_bytes([d[G2_C_SUM.2], d[G2_C_SUM.2 + 1]])
        == sum16(&d[G2_C_SUM.0..=G2_C_SUM.1])
        && d[0x2008] == 0x63
        && d[0x2D0F] == 0x7F;
    if crystal {
        return Some(Game::Crystal);
    }
    let gs = u16::from_le_bytes([d[G2_GS_SUM.2], d[G2_GS_SUM.2 + 1]])
        == sum16(&d[G2_GS_SUM.0..=G2_GS_SUM.1])
        && d[0x2008] == 0x63
        && d[0x2D6B] == 0x7F;
    if gs {
        return Some(Game::GoldSilver);
    }
    // Gen I: valid main checksum and a terminated player name.
    if d[G1_MAIN.1] == sum8(&d[G1_MAIN.0..G1_MAIN.1])
        && d[0x2598..0x25A3].contains(&TERM)
        && d[0x2598] != TERM
    {
        let party = d[0x2F2C] as usize;
        if party <= 6 && d[0x2F2D + party] == 0xFF {
            // Yellow's starter is always Pikachu (internal index 0x54).
            return Some(if d[0x29C3] == 0x54 {
                Game::Yellow
            } else {
                Game::RedBlue
            });
        }
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    detect(d).is_some()
}

#[derive(Clone, Copy, Debug)]
struct Offsets {
    name: usize,
    rival: usize,
    tid: usize,
    money: usize,
    coins: usize,
    badges: usize,
    time: usize,
    items: usize,
    pc_items: usize,
    balls: usize,
    keys: usize,
    tms: usize,
    party: usize,
    cur_box: usize,
    cur_box_index: usize,
    box_names: usize,
    gender: usize,
}

fn offsets(g: Game) -> Offsets {
    let n = usize::MAX;
    match g {
        Game::RedBlue | Game::Yellow => Offsets {
            name: 0x2598,
            rival: 0x25F6,
            tid: 0x2605,
            money: 0x25F3,
            coins: 0x2850,
            badges: 0x2602,
            time: 0x2CED,
            items: 0x25C9,
            pc_items: 0x27E6,
            balls: n,
            keys: n,
            tms: n,
            party: 0x2F2C,
            cur_box: 0x30C0,
            cur_box_index: 0x284C,
            box_names: n,
            gender: n,
        },
        Game::GoldSilver => Offsets {
            name: 0x200B,
            rival: 0x2021,
            tid: 0x2009,
            money: 0x23DB,
            coins: 0x23E2,
            badges: 0x23E4,
            time: 0x2053,
            items: 0x241F,
            pc_items: 0x247E,
            balls: 0x2464,
            keys: 0x2449,
            tms: 0x23E6,
            party: 0x288A,
            cur_box: 0x2D6C,
            cur_box_index: 0x2724,
            box_names: 0x2727,
            gender: n,
        },
        Game::Crystal => Offsets {
            name: 0x200B,
            rival: 0x2021,
            tid: 0x2009,
            money: 0x23DC,
            coins: 0x23E3,
            badges: 0x23E5,
            time: 0x2052,
            items: 0x2420,
            pc_items: 0x247F,
            balls: 0x2465,
            keys: 0x244A,
            tms: 0x23E7,
            party: 0x2865,
            cur_box: 0x2D10,
            cur_box_index: 0x2700,
            box_names: 0x2703,
            gender: 0x3E3D,
        },
    }
}

/// File offset of PC box `i`'s list.
fn box_offset(g: Game, i: usize) -> usize {
    if g.g2() {
        if i < 7 {
            0x4000 + i * 0x450
        } else {
            0x6000 + (i - 7) * 0x450
        }
    } else if i < 6 {
        0x4000 + i * 0x462
    } else {
        0x6000 + (i - 6) * 0x462
    }
}

/// A Pokémon list: count, species (cap + 1), records, OT names, nicknames.
#[derive(Clone, Copy, Debug)]
struct List {
    at: usize,
    cap: usize,
    rec: usize,
}

impl List {
    fn species_at(&self, i: usize) -> usize {
        self.at + 1 + i
    }
    fn rec_at(&self, i: usize) -> usize {
        self.at + 2 + self.cap + i * self.rec
    }
    fn ot_at(&self, i: usize) -> usize {
        self.at + 2 + self.cap + self.cap * self.rec + i * 11
    }
    fn nick_at(&self, i: usize) -> usize {
        self.at + 2 + self.cap + self.cap * self.rec + self.cap * 11 + i * 11
    }
    fn size(&self) -> usize {
        2 + self.cap * (self.rec + 23)
    }
    fn valid(&self, d: &[u8]) -> bool {
        let n = d[self.at] as usize;
        n <= self.cap && d[self.species_at(n)] == 0xFF
    }
}

// ------------------------------------------------------------------ fields

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Species,
    Nickname,
    Level,
    Experience,
    HeldItem,
    Friendship,
    Move(usize),
    Pp(usize),
    PpUps(usize),
    Iv(usize),
    StatExp(usize),
    CurrentHp,
    Stat(usize),
}

#[derive(Clone, Debug)]
enum Field {
    Text {
        at: usize,
        len: usize,
        fill: usize,
        max: usize,
    },
    Int {
        at: usize,
        size: usize,
        max: u32,
    },
    Bcd {
        at: usize,
        size: usize,
        max: u32,
    },
    Bit {
        at: usize,
        bit: u8,
    },
    Gender {
        at: usize,
    },
    /// One slot of an item list: count byte at `list`, `cap` entries of (id, qty).
    ListItem {
        list: usize,
        slot: usize,
    },
    ListQty {
        list: usize,
        slot: usize,
    },
    Mon {
        mon: usize,
        part: Part,
    },
}

#[derive(Clone, Debug)]
struct Mon {
    list: List,
    slot: usize,
    party: bool,
    ptr: String,
}

const STAT_NAMES_G1: [&str; 5] = ["HP", "Attack", "Defense", "Speed", "Special"];
const STAT_NAMES_G2: [&str; 6] = ["HP", "Attack", "Defense", "Speed", "Sp. Atk", "Sp. Def"];
const IV_NAMES: [&str; 4] = ["Attack", "Defense", "Speed", "Special"];

#[derive(Clone, Debug)]
struct GbBackend {
    raw: Vec<u8>,
    game: Game,
    fields: Vec<Field>,
    mons: Vec<Mon>,
    /// Item lists by count offset: their item and quantity pointers, slot by slot.
    lists: HashMap<usize, Vec<(String, Option<String>)>>,
    /// Gen I PC boxes have been used (otherwise their banks are uninitialised).
    boxes_live: bool,
    cur_box: usize,
}

fn as_str(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "expected text".into())
}

fn num_at(view: &Value, ptr: &str) -> u32 {
    view.pointer(ptr).and_then(Value::as_u64).unwrap_or(0) as u32
}

impl GbBackend {
    fn g2(&self) -> bool {
        self.game.g2()
    }
    fn species_names(&self) -> &'static [String] {
        if self.g2() {
            &tables().g2_names
        } else {
            &tables().g1_names
        }
    }
    fn items(&self) -> &'static [String] {
        if self.g2() {
            &tables().g2_items
        } else {
            &tables().g1_items
        }
    }
    fn moves(&self) -> &'static [String] {
        let m = &tables().moves;
        if self.g2() {
            &m[..252]
        } else {
            &m[..166]
        }
    }
    /// Base data as (stats [hp, atk, def, spe, spa, spd], growth, gen I types/catch rate).
    fn base(&self, species: usize) -> Option<([u32; 6], u16, Option<[u8; 3]>, u16)> {
        let t = tables();
        if self.g2() {
            let b = t.g2_base.get(species).copied().flatten()?;
            Some((
                [b[0], b[1], b[2], b[3], b[4], b[5]].map(u32::from),
                b[8],
                None,
                b[9],
            ))
        } else {
            let b = t.g1_base.get(*t.g1_dex.get(species)?).copied().flatten()?;
            Some((
                [b[0], b[1], b[2], b[3], b[4], b[4]].map(u32::from),
                b[8],
                Some([b[5] as u8, b[6] as u8, b[7] as u8]),
                0,
            ))
        }
    }
    fn mon_ptr(&self, mon: usize, key: &str) -> String {
        format!("{}/{}", self.mons[mon].ptr, key)
    }
    fn species_in_view(&self, mon: usize, view: &Value) -> usize {
        view.pointer(&self.mon_ptr(mon, "Species"))
            .and_then(Value::as_str)
            .and_then(|s| index_of(self.species_names(), s))
            .unwrap_or(0)
    }
    fn ivs_in_view(&self, mon: usize, view: &Value) -> [u32; 4] {
        std::array::from_fn(|i| num_at(view, &self.mon_ptr(mon, &format!("IVs/{}", IV_NAMES[i]))))
    }

    /// Stats from the view's species, level, IVs and stat experience.
    fn stats(&self, mon: usize, view: &Value, level: u32) -> Option<Vec<u32>> {
        let (b, ..) = self.base(self.species_in_view(mon, view))?;
        let iv = self.ivs_in_view(mon, view);
        let hp_iv = (iv[0] & 1) << 3 | (iv[1] & 1) << 2 | (iv[2] & 1) << 1 | (iv[3] & 1);
        let se = |k: &str| num_at(view, &self.mon_ptr(mon, &format!("Stat experience/{k}")));
        let names: &[&str] = if self.g2() {
            &STAT_NAMES_G2
        } else {
            &STAT_NAMES_G1
        };
        let spc = se("Special");
        Some(
            (0..names.len())
                .map(|i| {
                    let (iv_i, exp_i) = match i {
                        0 => (hp_iv, se("HP")),
                        1..=3 => (iv[i - 1], se(names[i])),
                        _ => (iv[3], spc),
                    };
                    gb_stat(b[i], iv_i, exp_i, level, i == 0)
                })
                .collect(),
        )
    }

    fn default_name(&self, species: usize) -> String {
        name_of(self.species_names(), species).to_uppercase()
    }
}

/// Gen II: shiny if Defense, Speed and Special IVs are 10 and Attack is 2, 3, 6, 7, 10, 11, 14 or 15.
fn g2_shiny(iv: [u32; 4]) -> bool {
    iv[1] == 10 && iv[2] == 10 && iv[3] == 10 && matches!(iv[0], 2 | 3 | 6 | 7 | 10 | 11 | 14 | 15)
}

/// Gen II gender: female when (Attack IV << 4 | Speed IV) <= the species' ratio
/// (pokecrystal's GetGender).
fn g2_gender(ratio: u16, iv: [u32; 4]) -> &'static str {
    match ratio {
        255 => "Unknown",
        0 => "Male",
        254 => "Female",
        r => {
            if (iv[0] << 4 | iv[2]) <= r as u32 {
                "Female"
            } else {
                "Male"
            }
        }
    }
}

impl Backend for GbBackend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = self.raw.clone();
        if changes.is_empty() {
            return Ok(out);
        }
        let g2 = self.g2();
        let (stored, _) = self.game.sizes();
        let mut touched_lists = std::collections::HashSet::new();
        let mut touched_boxes = std::collections::HashSet::new();
        for (leaf, v) in changes {
            match &self.fields[*leaf] {
                Field::Text { at, len, fill, max } => {
                    let orig = out[*at..*at + *len].to_vec();
                    out[*at..*at + *len].copy_from_slice(&encode_text(
                        as_str(v)?,
                        *len,
                        *fill,
                        *max,
                        &orig,
                        g2,
                    )?);
                }
                Field::Int { at, size, max } => {
                    let n = int_in(v, 0, *max as i128, "This value")? as u32;
                    out[*at..*at + *size].copy_from_slice(&n.to_be_bytes()[4 - size..]);
                }
                Field::Bcd { at, size, max } => {
                    let n = int_in(v, 0, *max as i128, "This value")? as u32;
                    out[*at..*at + *size].copy_from_slice(&to_bcd(n, *size));
                }
                Field::Bit { at, bit } => {
                    let on = v.as_bool().ok_or("expected on/off")?;
                    out[*at] = if on {
                        out[*at] | 1 << bit
                    } else {
                        out[*at] & !(1 << bit)
                    };
                }
                Field::Gender { at } => out[*at] = u8::from(as_str(v)? == "Girl"),
                Field::ListItem { list, .. } | Field::ListQty { list, .. } => {
                    touched_lists.insert(*list);
                }
                Field::Mon { mon, part } => {
                    let m = &self.mons[*mon];
                    let r = m.list.rec_at(m.slot);
                    if !m.party && m.list.at != offsets(self.game).cur_box {
                        touched_boxes.insert(m.list.at);
                    }
                    let int = |max: i128| int_in(v, 0, max, "This value").map(|n| n as u32);
                    let put = |out: &mut [u8], off: usize, size: usize, n: u32| {
                        out[r + off..r + off + size].copy_from_slice(&n.to_be_bytes()[4 - size..])
                    };
                    match *part {
                        Part::Species => {
                            let sp = index_of(self.species_names(), as_str(v)?)
                                .ok_or("unknown Pokémon")?;
                            out[r] = sp as u8;
                            if out[m.list.species_at(m.slot)] != 0xFD {
                                out[m.list.species_at(m.slot)] = sp as u8;
                            }
                            if let Some((_, _, Some([t1, t2, catch]), _)) = self.base(sp) {
                                // Gen I stores the species' types and catch rate in each Pokémon.
                                out[r + 5] = t1;
                                out[r + 6] = t2;
                                out[r + 7] = catch;
                            }
                        }
                        Part::Nickname => {
                            let at = m.list.nick_at(m.slot);
                            let orig = out[at..at + 11].to_vec();
                            out[at..at + 11].copy_from_slice(&encode_text(
                                as_str(v)?,
                                11,
                                11,
                                10,
                                &orig,
                                g2,
                            )?);
                        }
                        Part::Level => {
                            let lv = int(100)?;
                            if g2 {
                                if m.party || stored > 0x1F {
                                    out[r + 0x1F] = lv as u8;
                                }
                            } else {
                                out[r + 3] = lv as u8;
                                if m.party {
                                    out[r + 0x21] = lv as u8;
                                }
                            }
                        }
                        Part::Experience => {
                            put(&mut out, if g2 { 8 } else { 0x0E }, 3, int(0xFF_FFFF)?)
                        }
                        Part::HeldItem => {
                            out[r + 1] =
                                index_of(self.items(), as_str(v)?).ok_or("unknown item")? as u8
                        }
                        Part::Friendship => out[r + 0x1B] = int(255)? as u8,
                        Part::Move(i) => {
                            out[r + if g2 { 2 } else { 8 } + i] =
                                index_of(self.moves(), as_str(v)?).ok_or("unknown move")? as u8
                        }
                        Part::Pp(i) => {
                            let at = r + if g2 { 0x17 } else { 0x1D } + i;
                            out[at] = (out[at] & 0xC0) | int(63)? as u8;
                        }
                        Part::PpUps(i) => {
                            let at = r + if g2 { 0x17 } else { 0x1D } + i;
                            out[at] = (out[at] & 0x3F) | (int(3)? as u8) << 6;
                        }
                        Part::Iv(i) => {
                            let at = r + if g2 { 0x15 } else { 0x1B };
                            let w = be(&out[at..at + 2]);
                            let shift = 12 - 4 * i as u32;
                            let w = (w & !(0xF << shift)) | int(15)? << shift;
                            out[at..at + 2].copy_from_slice(&(w as u16).to_be_bytes());
                        }
                        Part::StatExp(i) => put(
                            &mut out,
                            if g2 { 0x0B } else { 0x11 } + i * 2,
                            2,
                            int(0xFFFF)?,
                        ),
                        Part::CurrentHp => {
                            put(&mut out, if g2 { 0x22 } else { 1 }, 2, int(0xFFFF)?)
                        }
                        Part::Stat(i) => put(
                            &mut out,
                            if g2 { 0x24 } else { 0x22 } + i * 2,
                            2,
                            int(0xFFFF)?,
                        ),
                    }
                }
            }
        }
        // Item lists are rewritten whole from the view (kept compact by `derived`).
        for list in touched_lists {
            let Some(slots) = self.lists.get(&list) else {
                continue;
            };
            let mut entries: Vec<(u8, u8)> = Vec::new();
            for (item_ptr, qty_ptr) in slots {
                let id = view
                    .pointer(item_ptr)
                    .and_then(Value::as_str)
                    .and_then(|s| index_of(self.items(), s))
                    .ok_or("unknown item")?;
                if id == 0 {
                    continue;
                }
                let q = qty_ptr.as_ref().map_or(1, |p| num_at(view, p).clamp(1, 99));
                entries.push((id as u8, q as u8));
            }
            let qty = slots.first().is_some_and(|s| s.1.is_some());
            out[list] = entries.len() as u8;
            let mut at = list + 1;
            for (id, q) in &entries {
                out[at] = *id;
                at += 1;
                if qty {
                    out[at] = *q;
                    at += 1;
                }
            }
            out[at] = 0xFF;
        }
        // Checksums (and Gen II's backup copy).
        let o = offsets(self.game);
        match self.game {
            Game::RedBlue | Game::Yellow => {
                // Gen I loads the current box from the main data (the game leaves its storage
                // slot empty), so only the other boxes' banks need new checksums.
                if self.boxes_live {
                    for bank in [0x4000usize, 0x6000] {
                        if touched_boxes
                            .iter()
                            .any(|&b| (bank..bank + 0x2000).contains(&b))
                        {
                            out[bank + 0x1A4C] = sum8(&out[bank..bank + 0x1A4C]);
                            for i in 0..6 {
                                let b = bank + i * 0x462;
                                out[bank + 0x1A4D + i] = sum8(&out[b..b + 0x462]);
                            }
                        }
                    }
                }
                out[G1_MAIN.1] = sum8(&out[G1_MAIN.0..G1_MAIN.1]);
            }
            Game::GoldSilver | Game::Crystal => {
                // The game loads the current box from its storage bank (pokecrystal's
                // TryLoadSaveFile -> LoadBox), so that's what gets edited; the active-box copy
                // is brought in line, as the game's own SaveBox does.
                let cur_store = box_offset(self.game, self.cur_box.min(13));
                if self.cur_box < 14 && touched_boxes.contains(&cur_store) {
                    let size = List {
                        at: 0,
                        cap: 20,
                        rec: 32,
                    }
                    .size();
                    let block = out[cur_store..cur_store + size].to_vec();
                    out[o.cur_box..o.cur_box + size].copy_from_slice(&block);
                }
                let (start, end, p1, p2) = if self.game == Game::Crystal {
                    G2_C_SUM
                } else {
                    G2_GS_SUM
                };
                let sum = sum16(&out[start..=end]).to_le_bytes();
                out[p1..p1 + 2].copy_from_slice(&sum);
                out[p2..p2 + 2].copy_from_slice(&sum);
                if self.game == Game::Crystal {
                    let primary = out[0x2009..0x2009 + 0xB7A].to_vec();
                    out[0x1209..0x1209 + 0xB7A].copy_from_slice(&primary);
                } else {
                    // Gold/Silver split the backup into five pieces. (The second one starts at
                    // 0x3D96, right after the Hall of Fame in pokegold's SRAM layout.)
                    for (from, to, dst) in [
                        (0x2009, 0x222F, 0x15C7),
                        (0x222F, 0x23D9, 0x3D96),
                        (0x23D9, 0x2856, 0x0C6B),
                        (0x2856, 0x288A, 0x7E39),
                        (0x288A, 0x2D69, 0x10E8),
                    ] {
                        let src = out[from..to].to_vec();
                        out[dst..dst + src.len()].copy_from_slice(&src);
                    }
                }
            }
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let g2 = self.g2();
        let known = |table: &[String], what: &str| -> Result<(), String> {
            let s = as_str(v)?;
            index_of(table, s)
                .map(|_| ())
                .ok_or_else(|| format!("there's no {what} called \u{201C}{s}\u{201D}"))
        };
        match &self.fields[leaf] {
            Field::Text { len, fill, max, .. } => {
                encode_text(as_str(v)?, *len, *fill, *max, &vec![TERM; *len], g2).map(|_| ())
            }
            Field::Int { max, .. } | Field::Bcd { max, .. } => {
                int_in(v, 0, *max as i128, "This value").map(|_| ())
            }
            Field::Bit { .. } => v
                .as_bool()
                .map(|_| ())
                .ok_or_else(|| "expected on/off".into()),
            Field::Gender { .. } => matches!(as_str(v)?, "Boy" | "Girl")
                .then_some(())
                .ok_or_else(|| "choose Boy or Girl".into()),
            Field::ListItem { .. } => known(self.items(), "item"),
            Field::ListQty { .. } => int_in(v, 0, 99, "Quantity").map(|_| ()),
            Field::Mon { part, .. } => match part {
                Part::Species => match index_of(self.species_names(), as_str(v)?) {
                    Some(i) if i > 0 && self.base(i).is_some() => Ok(()),
                    _ => Err(format!(
                        "there's no Pokémon called \u{201C}{}\u{201D}",
                        as_str(v)?
                    )),
                },
                Part::Nickname => encode_text(as_str(v)?, 11, 11, 10, &[TERM; 11], g2).map(|_| ()),
                Part::HeldItem => known(self.items(), "item"),
                Part::Move(_) => known(self.moves(), "move"),
                Part::Level => int_in(v, 2, 100, "Level").map(|_| ()),
                Part::Experience => int_in(v, 0, 1_640_000, "Experience").map(|_| ()),
                Part::Friendship => int_in(v, 0, 255, "Friendship").map(|_| ()),
                Part::Pp(_) => int_in(v, 0, 63, "PP").map(|_| ()),
                Part::PpUps(_) => int_in(v, 0, 3, "PP Ups").map(|_| ()),
                Part::Iv(_) => int_in(v, 0, 15, "IVs").map(|_| ()),
                Part::StatExp(_) => int_in(v, 0, 65535, "Stat experience").map(|_| ()),
                Part::CurrentHp | Part::Stat(_) => int_in(v, 0, 999, "Stats").map(|_| ()),
            },
        }
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        match &self.fields[leaf] {
            Field::ListItem { list, slot, .. } | Field::ListQty { list, slot, .. } => {
                // Keep the list compact: empty slots move to the end, like the game does.
                let Some(slots) = self.lists.get(list) else {
                    return vec![];
                };
                let mut filled: Vec<(String, u32)> = Vec::new();
                for (i, (ip, qp)) in slots.iter().enumerate() {
                    let name = view
                        .pointer(ip)
                        .and_then(Value::as_str)
                        .unwrap_or(NONE)
                        .to_string();
                    let mut q = qp.as_ref().map_or(1, |p| num_at(view, p));
                    if i == *slot && matches!(self.fields[leaf], Field::ListItem { .. }) && q == 0 {
                        q = 1;
                    }
                    if name != NONE && q > 0 {
                        filled.push((name, q.min(99)));
                    }
                }
                let mut out = Vec::new();
                for (i, (ip, qp)) in slots.iter().enumerate() {
                    let (n, q) = filled.get(i).cloned().unwrap_or((NONE.into(), 0));
                    out.push((ip.clone(), Value::from(n)));
                    if let Some(qp) = qp {
                        out.push((qp.clone(), Value::from(q)));
                    }
                }
                out
            }
            Field::Mon { mon, part } => {
                let (mon, part) = (*mon, *part);
                let p = |k: &str| self.mon_ptr(mon, k);
                let species = self.species_in_view(mon, view);
                let Some((_, growth, _, ratio)) = self.base(species) else {
                    return vec![];
                };
                let mut out = Vec::new();
                let mut level = num_at(view, &p("Level"));
                match part {
                    Part::Level => out.push((p("Experience"), Value::from(exp_for(growth, level)))),
                    Part::Experience | Part::Species => {
                        level = level_for(growth, num_at(view, &p("Experience")));
                        out.push((p("Level"), Value::from(level)));
                    }
                    Part::Iv(_) | Part::StatExp(_) => {}
                    _ => return vec![],
                }
                if part == Part::Species {
                    // Gen I stores the types in each Pokémon; they're rewritten with the species.
                    if let Some((_, _, Some([t1, t2, _]), _)) = self.base(species) {
                        out.push((p("Type"), Value::from(type_text(t1, t2))));
                    }
                    // A Pokémon that wasn't nicknamed takes the new species' name.
                    let nick = view
                        .pointer(&p("Nickname"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let old = self.raw[self.mons[mon].list.rec_at(self.mons[mon].slot)] as usize;
                    if nick == self.default_name(old)
                        || self
                            .species_names()
                            .iter()
                            .any(|s| s.to_uppercase() == nick)
                    {
                        out.push((p("Nickname"), Value::from(self.default_name(species))));
                    }
                }
                let iv = self.ivs_in_view(mon, view);
                out.push((
                    p("IVs/HP (from the others)"),
                    Value::from(
                        (iv[0] & 1) << 3 | (iv[1] & 1) << 2 | (iv[2] & 1) << 1 | (iv[3] & 1),
                    ),
                ));
                if self.g2() {
                    out.push((p("Shiny"), Value::Bool(g2_shiny(iv))));
                    out.push((p("Gender"), Value::from(g2_gender(ratio, iv))));
                }
                if self.mons[mon].party {
                    if let Some(stats) = self.stats(mon, view, level) {
                        let names: &[&str] = if self.g2() {
                            &STAT_NAMES_G2
                        } else {
                            &STAT_NAMES_G1
                        };
                        let old_max = num_at(view, &p("Stats/Max HP"));
                        let cur = num_at(view, &p("Stats/Current HP"));
                        out.push((p("Stats/Max HP"), Value::from(stats[0])));
                        out.push((
                            p("Stats/Current HP"),
                            Value::from((cur + stats[0].saturating_sub(old_max)).min(stats[0])),
                        ));
                        for i in 1..names.len() {
                            out.push((p(&format!("Stats/{}", names[i])), Value::from(stats[i])));
                        }
                    }
                }
                out
            }
            _ => vec![],
        }
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        let t = tables();
        let g2 = self.g2();
        let items = || {
            Some(if g2 {
                t.g2_item_list.clone()
            } else {
                t.g1_item_list.clone()
            })
        };
        match &self.fields[leaf] {
            Field::Gender { .. } => Some(Arc::new(vec!["Boy".into(), "Girl".into()])),
            Field::ListItem { .. } => items(),
            Field::Mon { part, .. } => match part {
                Part::Species => Some(if g2 {
                    t.g2_species_list.clone()
                } else {
                    t.g1_species_list.clone()
                }),
                Part::HeldItem => items(),
                Part::Move(_) => Some(if g2 {
                    t.g2_move_list.clone()
                } else {
                    t.g1_move_list.clone()
                }),
                _ => None,
            },
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

// ------------------------------------------------------------------ view

struct Builder<'a> {
    d: &'a [u8],
    game: Game,
    fields: Vec<Field>,
    editable: HashMap<String, usize>,
    mons: Vec<Mon>,
    lists: HashMap<usize, Vec<(String, Option<String>)>>,
}

impl Builder<'_> {
    fn add(
        &mut self,
        map: &mut Map<String, Value>,
        base: &str,
        key: &str,
        v: Value,
        f: Option<Field>,
    ) {
        if let Some(f) = f {
            self.editable
                .insert(format!("{base}/{}", esc(key)), self.fields.len());
            self.fields.push(f);
        }
        map.insert(key.to_string(), v);
    }

    fn text(
        &mut self,
        map: &mut Map<String, Value>,
        base: &str,
        key: &str,
        at: usize,
        fill: usize,
        max: usize,
    ) {
        match decode_text(&self.d[at..at + 11], self.game.g2()) {
            Some(s) if !s.is_empty() => self.add(
                map,
                base,
                key,
                Value::from(s),
                Some(Field::Text {
                    at,
                    len: 11,
                    fill,
                    max,
                }),
            ),
            _ => self.add(map, base, key, Value::from("?"), None),
        }
    }

    /// An item list with `cap` slots; every slot is shown so the layout never changes.
    fn item_list(
        &mut self,
        root: &mut Map<String, Value>,
        title: &str,
        list: usize,
        cap: usize,
        qty: bool,
    ) {
        let items: &[String] = if self.game.g2() {
            &tables().g2_items
        } else {
            &tables().g1_items
        };
        let count = (self.d[list] as usize).min(cap);
        let base = format!("/Bag/{}", esc(title));
        let mut pocket = Map::new();
        let mut slots = Vec::new();
        let step = if qty { 2 } else { 1 };
        for i in 0..cap {
            let key = format!("Slot {}", i + 1);
            let sp = format!("{base}/{}", esc(&key));
            let at = list + 1 + i * step;
            let (id, q) = if i < count {
                (
                    self.d[at] as usize,
                    if qty { self.d[at + 1] as u32 } else { 1 },
                )
            } else {
                (0, 0)
            };
            let mut slot = Map::new();
            self.add(
                &mut slot,
                &sp,
                "Item",
                Value::from(name_of(items, id)),
                Some(Field::ListItem { list, slot: i }),
            );
            let qty_ptr = if qty {
                self.add(
                    &mut slot,
                    &sp,
                    "Quantity",
                    Value::from(q),
                    Some(Field::ListQty { list, slot: i }),
                );
                Some(format!("{sp}/Quantity"))
            } else {
                None
            };
            slots.push((format!("{sp}/Item"), qty_ptr));
            pocket.insert(key, Value::Object(slot));
        }
        self.lists.insert(list, slots);
        root.insert(title.into(), Value::Object(pocket));
    }

    fn mon(&mut self, list: List, slot: usize, party: bool, ptr: &str) -> Option<Value> {
        let t = tables();
        let g2 = self.game.g2();
        let d = self.d;
        let r = list.rec_at(slot);
        let species = d[r] as usize;
        let names: &[String] = if g2 { &t.g2_names } else { &t.g1_names };
        let tmp = GbBackend {
            raw: vec![],
            game: self.game,
            fields: vec![],
            mons: vec![],
            lists: HashMap::new(),
            boxes_live: false,
            cur_box: 0,
        };
        let (_, growth, _, ratio) = tmp.base(species)?;
        let egg = g2 && d[list.species_at(slot)] == 0xFD;
        let mon = self.mons.len();
        self.mons.push(Mon {
            list,
            slot,
            party,
            ptr: ptr.to_string(),
        });
        let mf = |part| Some(Field::Mon { mon, part });
        let mut m = Map::new();
        self.add(
            &mut m,
            ptr,
            "Species",
            Value::from(name_of(names, species)),
            mf(Part::Species),
        );
        if egg {
            m.insert("Egg".into(), Value::Bool(true));
        }
        match decode_text(&d[list.nick_at(slot)..list.nick_at(slot) + 11], g2) {
            Some(n) if !n.is_empty() && !egg => {
                self.add(&mut m, ptr, "Nickname", Value::from(n), mf(Part::Nickname))
            }
            other => {
                m.insert(
                    "Nickname".into(),
                    Value::from(other.unwrap_or_else(|| "?".into())),
                );
            }
        }
        if let Some(ot) = decode_text(&d[list.ot_at(slot)..list.ot_at(slot) + 11], g2) {
            m.insert("Original trainer".into(), Value::from(ot));
        }
        let exp = be(&d[r + if g2 { 8 } else { 0x0E }..][..3]);
        let level = if g2 {
            d[r + 0x1F] as u32
        } else if party {
            d[r + 0x21] as u32
        } else {
            d[r + 3] as u32
        };
        let _ = growth;
        self.add(&mut m, ptr, "Level", Value::from(level), mf(Part::Level));
        self.add(
            &mut m,
            ptr,
            "Experience",
            Value::from(exp),
            mf(Part::Experience),
        );
        let ivw = be(&d[r + if g2 { 0x15 } else { 0x1B }..][..2]);
        let iv: [u32; 4] = std::array::from_fn(|i| (ivw >> (12 - 4 * i)) & 0xF);
        if g2 {
            self.add(
                &mut m,
                ptr,
                "Held item",
                Value::from(name_of(&t.g2_items, d[r + 1] as usize)),
                mf(Part::HeldItem),
            );
            self.add(
                &mut m,
                ptr,
                "Friendship",
                Value::from(d[r + 0x1B]),
                mf(Part::Friendship),
            );
            m.insert("Shiny".into(), Value::Bool(g2_shiny(iv)));
            m.insert("Gender".into(), Value::from(g2_gender(ratio, iv)));
        } else {
            m.insert("Type".into(), Value::from(type_text(d[r + 5], d[r + 6])));
        }
        let mut moves = Map::new();
        let mp = format!("{ptr}/Moves");
        let move_at = r + if g2 { 2 } else { 8 };
        let pp_at = r + if g2 { 0x17 } else { 0x1D };
        let move_names: &[String] = if g2 { &t.moves[..252] } else { &t.moves[..166] };
        for i in 0..4 {
            self.add(
                &mut moves,
                &mp,
                &format!("Move {}", i + 1),
                Value::from(name_of(move_names, d[move_at + i] as usize)),
                mf(Part::Move(i)),
            );
            self.add(
                &mut moves,
                &mp,
                &format!("PP {}", i + 1),
                Value::from(d[pp_at + i] & 0x3F),
                mf(Part::Pp(i)),
            );
            self.add(
                &mut moves,
                &mp,
                &format!("PP Ups {}", i + 1),
                Value::from(d[pp_at + i] >> 6),
                mf(Part::PpUps(i)),
            );
        }
        m.insert("Moves".into(), Value::Object(moves));
        let mut ivs = Map::new();
        let ip = format!("{ptr}/IVs");
        for i in 0..4 {
            self.add(
                &mut ivs,
                &ip,
                IV_NAMES[i],
                Value::from(iv[i]),
                mf(Part::Iv(i)),
            );
        }
        ivs.insert(
            "HP (from the others)".into(),
            Value::from((iv[0] & 1) << 3 | (iv[1] & 1) << 2 | (iv[2] & 1) << 1 | (iv[3] & 1)),
        );
        m.insert("IVs".into(), Value::Object(ivs));
        let mut se = Map::new();
        let sp = format!("{ptr}/Stat experience");
        let se_at = r + if g2 { 0x0B } else { 0x11 };
        for i in 0..5 {
            self.add(
                &mut se,
                &sp,
                STAT_NAMES_G1[i],
                Value::from(be(&d[se_at + i * 2..se_at + i * 2 + 2])),
                mf(Part::StatExp(i)),
            );
        }
        m.insert("Stat experience".into(), Value::Object(se));
        if party {
            let names: &[&str] = if g2 { &STAT_NAMES_G2 } else { &STAT_NAMES_G1 };
            let mut st = Map::new();
            let stp = format!("{ptr}/Stats");
            let cur = be(&d[r + if g2 { 0x22 } else { 1 }..][..2]);
            self.add(
                &mut st,
                &stp,
                "Current HP",
                Value::from(cur),
                mf(Part::CurrentHp),
            );
            let base = r + if g2 { 0x24 } else { 0x22 };
            for (i, n) in names.iter().enumerate() {
                let key = if i == 0 { "Max HP" } else { n };
                self.add(
                    &mut st,
                    &stp,
                    key,
                    Value::from(be(&d[base + i * 2..base + i * 2 + 2])),
                    mf(Part::Stat(i)),
                );
            }
            m.insert("Stats".into(), Value::Object(st));
        }
        Some(Value::Object(m))
    }
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let game = detect(d).ok_or("not a Generation I/II Pokémon save")?;
    let g2 = game.g2();
    let o = offsets(game);
    let (stored, full) = game.sizes();
    let mut b = Builder {
        d,
        game,
        fields: vec![],
        editable: HashMap::new(),
        mons: vec![],
        lists: HashMap::new(),
    };
    let mut root = Map::new();

    // Trainer
    let mut tr = Map::new();
    let tp = "/Trainer";
    // Gen II fills only the first 8 bytes of a player/rival name with 0x50.
    let fill = if g2 { 8 } else { 11 };
    b.text(&mut tr, tp, "Name", o.name, fill, 7);
    b.text(&mut tr, tp, "Rival name", o.rival, fill, 7);
    tr.insert("Trainer ID".into(), Value::from(be(&d[o.tid..o.tid + 2])));
    if o.gender != usize::MAX && d[o.gender] <= 1 {
        b.add(
            &mut tr,
            tp,
            "Gender",
            Value::from(if d[o.gender] == 1 { "Girl" } else { "Boy" }),
            Some(Field::Gender { at: o.gender }),
        );
    }
    if g2 {
        b.add(
            &mut tr,
            tp,
            "Money",
            Value::from(be(&d[o.money..o.money + 3])),
            Some(Field::Int {
                at: o.money,
                size: 3,
                max: 999_999,
            }),
        );
        b.add(
            &mut tr,
            tp,
            "Coins",
            Value::from(be(&d[o.coins..o.coins + 2])),
            Some(Field::Int {
                at: o.coins,
                size: 2,
                max: 9999,
            }),
        );
        b.add(
            &mut tr,
            tp,
            "Play time (hours)",
            Value::from(be(&d[o.time..o.time + 2])),
            Some(Field::Int {
                at: o.time,
                size: 2,
                max: 999,
            }),
        );
    } else {
        match bcd(&d[o.money..o.money + 3]) {
            Some(n) => b.add(
                &mut tr,
                tp,
                "Money",
                Value::from(n),
                Some(Field::Bcd {
                    at: o.money,
                    size: 3,
                    max: 999_999,
                }),
            ),
            None => b.add(&mut tr, tp, "Money", Value::from("?"), None),
        }
        if let Some(n) = bcd(&d[o.coins..o.coins + 2]) {
            b.add(
                &mut tr,
                tp,
                "Coins",
                Value::from(n),
                Some(Field::Bcd {
                    at: o.coins,
                    size: 2,
                    max: 9999,
                }),
            );
        }
        b.add(
            &mut tr,
            tp,
            "Play time (hours)",
            Value::from(d[o.time]),
            Some(Field::Int {
                at: o.time,
                size: 1,
                max: 255,
            }),
        );
    }
    b.add(
        &mut tr,
        tp,
        "Play time (minutes)",
        Value::from(d[o.time + 2]),
        Some(Field::Int {
            at: o.time + 2,
            size: 1,
            max: 59,
        }),
    );
    b.add(
        &mut tr,
        tp,
        "Play time (seconds)",
        Value::from(d[o.time + 3]),
        Some(Field::Int {
            at: o.time + 3,
            size: 1,
            max: 59,
        }),
    );
    root.insert("Trainer".into(), Value::Object(tr));

    // Badges
    let mut badges = Map::new();
    let names: &[(&str, usize)] = if g2 {
        &[
            ("Zephyr", 0),
            ("Hive", 0),
            ("Plain", 0),
            ("Fog", 0),
            ("Storm", 0),
            ("Mineral", 0),
            ("Glacier", 0),
            ("Rising", 0),
            ("Boulder", 1),
            ("Cascade", 1),
            ("Thunder", 1),
            ("Rainbow", 1),
            ("Soul", 1),
            ("Marsh", 1),
            ("Volcano", 1),
            ("Earth", 1),
        ]
    } else {
        &[
            ("Boulder", 0),
            ("Cascade", 0),
            ("Thunder", 0),
            ("Rainbow", 0),
            ("Soul", 0),
            ("Marsh", 0),
            ("Volcano", 0),
            ("Earth", 0),
        ]
    };
    for (i, (n, byte)) in names.iter().enumerate() {
        let at = o.badges + byte;
        let bit = (i % 8) as u8;
        b.add(
            &mut badges,
            "/Badges",
            &format!("{n} Badge"),
            Value::Bool(d[at] >> bit & 1 == 1),
            Some(Field::Bit { at, bit }),
        );
    }
    root.insert("Badges".into(), Value::Object(badges));

    // Bag
    let mut bag = Map::new();
    b.item_list(&mut bag, "Items", o.items, 20, true);
    if g2 {
        b.item_list(&mut bag, "Key items", o.keys, 25, false);
        b.item_list(&mut bag, "Poké Balls", o.balls, 12, true);
        let mut tms = Map::new();
        for i in 0..57 {
            let key = if i < 50 {
                format!("TM{:02}", i + 1)
            } else {
                format!("HM{:02}", i - 49)
            };
            let max = if i < 50 { 99 } else { 1 };
            b.add(
                &mut tms,
                "/Bag/TMs & HMs",
                &key,
                Value::from(d[o.tms + i]),
                Some(Field::Int {
                    at: o.tms + i,
                    size: 1,
                    max,
                }),
            );
        }
        bag.insert("TMs & HMs".into(), Value::Object(tms));
    }
    b.item_list(&mut bag, "PC items", o.pc_items, 50, true);
    root.insert("Bag".into(), Value::Object(bag));

    // Party
    let party = List {
        at: o.party,
        cap: 6,
        rec: full,
    };
    let mut pm = Map::new();
    if party.valid(d) {
        for i in 0..d[party.at] as usize {
            let key = format!("Pokémon {}", i + 1);
            if let Some(v) = b.mon(party, i, true, &format!("/Party/{}", esc(&key))) {
                pm.insert(key, v);
            }
        }
    }
    root.insert("Party".into(), Value::Object(pm));

    // PC boxes. Gen I keeps the current box in the main data; Gen II loads every box,
    // the current one included, from its storage bank.
    let boxes = if g2 { 14 } else { 12 };
    let cur_raw = d[o.cur_box_index];
    let cur = (cur_raw & if g2 { 0x0F } else { 0x7F }) as usize;
    let boxes_live = g2 || cur_raw & 0x80 != 0;
    let mut pc = Map::new();
    for bx in 0..boxes {
        let at = if bx == cur && !g2 {
            o.cur_box
        } else {
            box_offset(game, bx)
        };
        if bx != cur && !boxes_live {
            continue;
        }
        let list = List {
            at,
            cap: 20,
            rec: stored,
        };
        let key = format!("Box {}", bx + 1);
        let bp = format!("/PC boxes/{}", esc(&key));
        let mut bm = Map::new();
        if g2 {
            let na = o.box_names + bx * 9;
            if let Some(n) = decode_text(&d[na..na + 9], true).filter(|n| !n.is_empty()) {
                b.add(
                    &mut bm,
                    &bp,
                    "Box name",
                    Value::from(n),
                    Some(Field::Text {
                        at: na,
                        len: 9,
                        fill: 9,
                        max: 8,
                    }),
                );
            }
        }
        if bx == cur {
            bm.insert("Current box".into(), Value::Bool(true));
        }
        if list.valid(d) {
            for i in 0..d[list.at] as usize {
                let k = format!("Slot {}", i + 1);
                if let Some(v) = b.mon(list, i, false, &format!("{bp}/{}", esc(&k))) {
                    bm.insert(k, v);
                }
            }
        }
        pc.insert(key, Value::Object(bm));
    }
    root.insert("PC boxes".into(), Value::Object(pc));

    let backend = GbBackend {
        raw: d.to_vec(),
        game,
        fields: b.fields,
        mons: b.mons,
        lists: b.lists,
        boxes_live,
        cur_box: cur,
    };
    let doc = TreeDoc::new(
        Value::Object(root),
        b.editable,
        game.name(),
        Box::new(backend),
    );
    Ok(doc.with_note(
        "Checksums are fixed automatically. Changing a Pokémon's level also sets its experience and stats; empty bag slots \
         move to the end like in the game. Close the game in your emulator before saving, and don't load a save state afterwards.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_and_formulas() {
        let t = tables();
        assert_eq!(t.g1_names[0x01], "Rhydon");
        assert_eq!(t.g1_dex[0x54], 25);
        assert_eq!(t.g1_names[0x54], "Pikachu");
        assert_eq!(t.g2_names[251], "Celebi");
        assert_eq!(t.g1_items[0x28], "Rare Candy");
        assert!(
            !t.g1_item_list.iter().any(|n| g1_not_an_item(n)),
            "no badges or floors in the item list"
        );
        assert!(
            t.g1_item_list.contains(&"Potion".to_string())
                && t.g1_item_list.contains(&"HM01".to_string())
        );
        assert!(
            g1_not_an_item("B2f")
                && g1_not_an_item("10f")
                && g1_not_an_item("Boulderbadge")
                && !g1_not_an_item("Bicycle")
        );
        assert_eq!(t.g2_items[0x05], "Poké Ball");
        assert_eq!(exp_for(3, 100), 1_059_860);
        assert_eq!(level_for(0, 125), 5);
        // Mewtwo with max IVs and stat experience: 415 HP at level 100 (the well-known maximum).
        assert_eq!(gb_stat(106, 15, 65535, 100, true), 415);
        assert_eq!(gb_stat(106, 15, 65535, 70, true), 293);
        assert_eq!(ev_term(65535), 63);
        assert_eq!(ev_term(0), 0);
        assert_eq!(bcd(&[0x01, 0x23, 0x45]), Some(12345));
        assert_eq!(to_bcd(999_999, 3), [0x99, 0x99, 0x99]);
        assert!(g2_shiny([10, 10, 10, 10]) && !g2_shiny([9, 10, 10, 10]));
    }

    fn text(s: &str, g2: bool) -> Vec<u8> {
        encode_text(s, 11, 11, 10, &[TERM; 11], g2).unwrap()
    }

    /// A minimal Red/Blue save: trainer, one item, one party Pokémon.
    pub fn red_save() -> Vec<u8> {
        let mut d = vec![0u8; 0x8000];
        d[0x2598..0x25A3].copy_from_slice(&text("ASH", false));
        d[0x25F6..0x2601].copy_from_slice(&text("GARY", false));
        d[0x25F3..0x25F6].copy_from_slice(&[0x00, 0x30, 0x00]);
        d[0x25C9] = 1;
        d[0x25CA..0x25CD].copy_from_slice(&[0x14, 5, 0xFF]); // 5 Potions
        d[0x27E6] = 0;
        d[0x27E7] = 0xFF;
        // Party: one Pikachu (internal 0x54), level 5.
        let p = 0x2F2C;
        d[p] = 1;
        d[p + 1] = 0x54;
        d[p + 2] = 0xFF;
        let r = p + 8;
        d[r] = 0x54;
        d[r + 3] = 5;
        d[r + 5] = 23;
        d[r + 6] = 23;
        d[r + 8] = 84; // Thundershock
        d[r + 0x0E..r + 0x11].copy_from_slice(&[0, 0, 125]);
        d[r + 0x1B..r + 0x1D].copy_from_slice(&[0xFF, 0xFF]);
        d[r + 0x21] = 5;
        d[p + 8 + 6 * 44 + 6 * 11..][..11].copy_from_slice(&text("PIKACHU", false));
        d[p + 8 + 6 * 44..][..11].copy_from_slice(&text("ASH", false));
        // PC boxes in use (bit 7), box 1 current: a Bulbasaur in the current box copy and a
        // Charmander stored in box 2.
        d[0x284C] = 0x80;
        let put_box = |d: &mut Vec<u8>, at: usize, species: u8, nick: &str| {
            let l = List {
                at,
                cap: 20,
                rec: 33,
            };
            d[at] = 1;
            d[at + 1] = species;
            d[at + 2] = 0xFF;
            let r = l.rec_at(0);
            d[r] = species;
            d[r + 3] = 10;
            d[r + 0x0E..r + 0x11].copy_from_slice(&[0, 0x02, 0x60]);
            d[l.ot_at(0)..l.ot_at(0) + 11].copy_from_slice(&text("ASH", false));
            d[l.nick_at(0)..l.nick_at(0) + 11].copy_from_slice(&text(nick, false));
        };
        for bx in 0..12 {
            let at = box_offset(Game::RedBlue, bx);
            d[at + 1] = 0xFF;
        }
        put_box(&mut d, 0x30C0, 0x99, "BULBASAUR");
        put_box(&mut d, box_offset(Game::RedBlue, 1), 0xB0, "CHARMANDER");
        for bank in [0x4000usize, 0x6000] {
            d[bank + 0x1A4C] = sum8(&d[bank..bank + 0x1A4C]);
            for i in 0..6 {
                let b = bank + i * 0x462;
                d[bank + 0x1A4D + i] = sum8(&d[b..b + 0x462]);
            }
        }
        d[0x3523] = sum8(&d[0x2598..0x3523]);
        d
    }

    #[test]
    fn red_pc_boxes() {
        let d = red_save();
        let mut doc = parse(&d).unwrap();
        assert_eq!(
            doc.view.pointer("/PC boxes/Box 1/Slot 1/Species").unwrap(),
            "Bulbasaur"
        );
        assert_eq!(
            doc.view.pointer("/PC boxes/Box 2/Slot 1/Species").unwrap(),
            "Charmander"
        );
        doc.set("/PC boxes/Box 2/Slot 1/Level", Value::from(40))
            .unwrap();
        doc.set("/PC boxes/Box 1/Slot 1/Nickname", Value::from("BULBY"))
            .unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
        for bank in [0x4000usize, 0x6000] {
            assert_eq!(
                out[bank + 0x1A4C],
                sum8(&out[bank..bank + 0x1A4C]),
                "bank checksum"
            );
            for i in 0..6 {
                let b = bank + i * 0x462;
                assert_eq!(
                    out[bank + 0x1A4D + i],
                    sum8(&out[b..b + 0x462]),
                    "box checksum"
                );
            }
        }
        // The current box is edited where the game reads it (the main data); its empty
        // storage slot is left alone.
        assert_eq!(out[0x4000..0x4000 + 0x462], d[0x4000..0x4000 + 0x462]);
        assert_eq!(
            out[0x30C0 + 0x16 + 20 * 33 + 20 * 11..][..5],
            [0x81, 0x94, 0x8B, 0x81, 0x98],
            "BULBY"
        );
        // Box level byte follows the level (Gen I shows it in the PC).
        let r = List {
            at: box_offset(Game::RedBlue, 1),
            cap: 20,
            rec: 33,
        }
        .rec_at(0);
        assert_eq!(out[r + 3], 40);
    }

    #[test]
    fn edit_red_save() {
        let d = red_save();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Pokémon Red / Blue");
        assert_eq!(doc.view.pointer("/Trainer/Name").unwrap(), "ASH");
        assert_eq!(doc.view.pointer("/Trainer/Money").unwrap(), 3000);
        assert_eq!(
            doc.view.pointer("/Bag/Items/Slot 1/Item").unwrap(),
            "Potion"
        );
        let p = "/Party/Pokémon 1";
        assert_eq!(
            doc.view.pointer(&format!("{p}/Species")).unwrap(),
            "Pikachu"
        );
        doc.set("/Trainer/Money", Value::from(999_999)).unwrap();
        doc.set("/Trainer/Name", Value::from("RED")).unwrap();
        doc.set("/Badges/Boulder Badge", Value::Bool(true)).unwrap();
        // Filling slot 3 moves the item up to slot 2 (lists have no gaps).
        doc.set("/Bag/Items/Slot 3/Item", Value::from("Rare Candy"))
            .unwrap();
        assert_eq!(
            doc.view.pointer("/Bag/Items/Slot 2/Item").unwrap(),
            "Rare Candy"
        );
        assert_eq!(doc.view.pointer("/Bag/Items/Slot 2/Quantity").unwrap(), 1);
        doc.set("/Bag/Items/Slot 2/Quantity", Value::from(99))
            .unwrap();
        doc.set(&format!("{p}/Level"), Value::from(50)).unwrap();
        doc.set(&format!("{p}/Species"), Value::from("Raichu"))
            .unwrap();
        assert_eq!(
            doc.view.pointer(&format!("{p}/Nickname")).unwrap(),
            "RAICHU",
            "un-nicknamed Pokémon follow the species"
        );
        doc.set(&format!("{p}/Moves/Move 2"), Value::from("Thunderbolt"))
            .unwrap();
        assert!(
            doc.set(&format!("{p}/Moves/Move 3"), Value::from("Shadow Ball"))
                .is_err(),
            "Gen II move in Gen I"
        );
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0x3523], sum8(&out[0x2598..0x3523]), "checksum");
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
        assert_eq!(out[0x25C9], 2, "two entries in the item list");
        assert_eq!(&out[0x25CA..0x25CF], &[0x14, 5, 0x28, 99, 0xFF]);
        // Types and catch rate follow the new species (Raichu: Electric, catch rate 75).
        let r = 0x2F2C + 8;
        assert_eq!((out[r], out[r + 5], out[r + 7]), (0x55, 23, 75));
        assert_eq!(out[0x2F2D], 0x55, "species list updated too");
        assert_eq!(again.view.pointer(&format!("{p}/Level")).unwrap(), 50);
    }

    /// A minimal Crystal save with one party Pokémon.
    pub fn crystal_save() -> Vec<u8> {
        let mut d = vec![0u8; 0x8000];
        d[0x2008] = 0x63;
        d[0x2D0F] = 0x7F;
        d[0x2009..0x200B].copy_from_slice(&[0x12, 0x34]);
        let mut name = encode_text("KRIS", 11, 8, 7, &[TERM; 11], true).unwrap();
        name[8..].fill(0);
        d[0x200B..0x2016].copy_from_slice(&name);
        d[0x2021..0x202C]
            .copy_from_slice(&encode_text("SILVER", 11, 8, 7, &[TERM; 11], true).unwrap());
        d[0x23DC..0x23DF].copy_from_slice(&[0, 0x0B, 0xB8]); // 3000
        d[0x2420] = 0;
        d[0x2421] = 0xFF;
        d[0x244A] = 0;
        d[0x244B] = 0xFF;
        d[0x2465] = 0;
        d[0x2466] = 0xFF;
        d[0x247F] = 0;
        d[0x2480] = 0xFF;
        let p = 0x2865;
        d[p] = 1;
        d[p + 1] = 155; // Cyndaquil
        d[p + 2] = 0xFF;
        let r = p + 8;
        d[r] = 155;
        d[r + 2] = 33; // Tackle
        d[r + 8..r + 11].copy_from_slice(&[0, 0, 135]);
        d[r + 0x15..r + 0x17].copy_from_slice(&[0xAA, 0xAA]);
        d[r + 0x1B] = 70;
        d[r + 0x1F] = 5;
        d[p + 8 + 6 * 48..][..11]
            .copy_from_slice(&encode_text("KRIS", 11, 11, 10, &[TERM; 11], true).unwrap());
        d[p + 8 + 6 * 48 + 6 * 11..][..11]
            .copy_from_slice(&encode_text("CYNDAQUIL", 11, 11, 10, &[TERM; 11], true).unwrap());
        d[0x2D10 + 1] = 0xFF;
        for bx in 0..14 {
            d[box_offset(Game::Crystal, bx) + 1] = 0xFF;
        }
        // An egg in the party's second slot and a Togepi stored in box 3.
        d[p] = 2;
        d[p + 2] = 0xFD;
        d[p + 3] = 0xFF;
        let r2 = p + 8 + 48;
        d[r2] = 175;
        d[r2 + 0x1F] = 5;
        d[p + 8 + 6 * 48 + 6 * 11 + 11..][..11]
            .copy_from_slice(&encode_text("EGG", 11, 11, 10, &[TERM; 11], true).unwrap());
        let b3 = List {
            at: box_offset(Game::Crystal, 2),
            cap: 20,
            rec: 32,
        };
        d[b3.at] = 1;
        d[b3.at + 1] = 175;
        d[b3.at + 2] = 0xFF;
        let r = b3.rec_at(0);
        d[r] = 175;
        d[r + 0x1F] = 7;
        d[b3.nick_at(0)..b3.nick_at(0) + 11]
            .copy_from_slice(&encode_text("TOGEPI", 11, 11, 10, &[TERM; 11], true).unwrap());
        d[b3.ot_at(0)..b3.ot_at(0) + 11]
            .copy_from_slice(&encode_text("KRIS", 11, 11, 10, &[TERM; 11], true).unwrap());
        d[p + 8 + 6 * 48 + 11..][..11]
            .copy_from_slice(&encode_text("KRIS", 11, 11, 10, &[TERM; 11], true).unwrap());
        let sum = sum16(&d[0x2009..=0x2B82]).to_le_bytes();
        d[0x2D0D..0x2D0F].copy_from_slice(&sum);
        d
    }

    #[test]
    fn edit_crystal_save() {
        let d = crystal_save();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Pokémon Crystal");
        let p = "/Party/Pokémon 1";
        assert_eq!(doc.view.pointer("/Trainer/Money").unwrap(), 3000);
        assert_eq!(
            doc.view.pointer(&format!("{p}/Shiny")).unwrap(),
            true,
            "all-10 IVs are shiny"
        );
        doc.set(&format!("{p}/IVs/Defense"), Value::from(15))
            .unwrap();
        assert_eq!(doc.view.pointer(&format!("{p}/Shiny")).unwrap(), false);
        doc.set(&format!("{p}/Held item"), Value::from("Leftovers"))
            .unwrap();
        doc.set(&format!("{p}/Level"), Value::from(36)).unwrap();
        doc.set("/Trainer/Name", Value::from("LYRA")).unwrap();
        doc.set("/Bag/Key items/Slot 1/Item", Value::from("Bicycle"))
            .unwrap();
        doc.set("/Bag/TMs & HMs/HM03", Value::from(1)).unwrap();
        doc.set("/Badges/Rising Badge", Value::Bool(true)).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
        assert_eq!(
            u16::from_le_bytes([out[0x2D0D], out[0x2D0E]]),
            sum16(&out[0x2009..=0x2B82])
        );
        assert_eq!(
            out[0x1F0D..0x1F0F],
            out[0x2D0D..0x2D0F],
            "both checksums written"
        );
        assert_eq!(
            out[0x1209..0x1209 + 0xB7A],
            out[0x2009..0x2009 + 0xB7A],
            "backup copy matches"
        );
        assert_eq!(
            &out[0x200B + 8..0x200B + 11],
            &[0, 0, 0],
            "bytes after the name are kept"
        );
        assert_eq!(
            &out[0x244A..0x244D],
            &[1, 7, 0xFF],
            "key items have no quantities"
        );
        assert_eq!(again.view.pointer("/Party/Pokémon 2/Egg").unwrap(), true);
        assert!(
            again.editable.get("/Party/Pokémon 2/Nickname").is_none(),
            "eggs keep their name"
        );
        let mut doc = again;
        doc.set("/PC boxes/Box 3/Slot 1/Level", Value::from(20))
            .unwrap();
        let out2 = doc.to_bytes().unwrap();
        assert_eq!(parse(&out2).unwrap().view, doc.view);
        let r = List {
            at: box_offset(Game::Crystal, 2),
            cap: 20,
            rec: 32,
        }
        .rec_at(0);
        assert_eq!(
            out2[r + 0x1F],
            20,
            "Gen II box Pokémon keep their level byte"
        );
        assert_eq!(out2[0x2865 + 2], 0xFD, "egg marker untouched");
    }

    #[test]
    #[ignore]
    fn write_samples() {
        let dir = std::env::var("SAVE_EDITOR_SAMPLES").expect("SAVE_EDITOR_SAMPLES");
        std::fs::write(format!("{dir}/red.sav"), red_save()).unwrap();
        std::fs::write(format!("{dir}/crystal.sav"), crystal_save()).unwrap();
    }

    #[test]
    fn gold_silver_backup_copy() {
        // Turn the Crystal sample into a minimal Gold/Silver one (different offsets).
        let mut d = vec![0u8; 0x8000];
        d[0x2008] = 0x63;
        d[0x2D6B] = 0x7F;
        d[0x200B..0x2016]
            .copy_from_slice(&encode_text("GOLD", 11, 8, 7, &[TERM; 11], true).unwrap());
        d[0x2021..0x202C]
            .copy_from_slice(&encode_text("SILVER", 11, 8, 7, &[TERM; 11], true).unwrap());
        for at in [0x241F, 0x2449, 0x2464, 0x247E] {
            d[at + 1] = 0xFF;
        }
        d[0x288B] = 0xFF;
        d[0x2D6C + 1] = 0xFF;
        for bx in 0..14 {
            d[box_offset(Game::GoldSilver, bx) + 1] = 0xFF;
        }
        let sum = sum16(&d[0x2009..=0x2D68]).to_le_bytes();
        d[0x2D69..0x2D6B].copy_from_slice(&sum);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Pokémon Gold / Silver");
        doc.set("/Trainer/Money", Value::from(123_456)).unwrap();
        doc.set("/Bag/Items/Slot 1/Item", Value::from("Rare Candy"))
            .unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(parse(&out).unwrap().view, doc.view);
        assert_eq!(
            u16::from_le_bytes([out[0x2D69], out[0x2D6A]]),
            sum16(&out[0x2009..=0x2D68])
        );
        // The backup pieces hold the same bytes, so its checksum (over 0x0C6B-0x17EC,
        // 0x3D96-0x3F3F and 0x7E39-0x7E6C) matches too.
        let backup = [
            &out[0x0C6B..=0x17EC],
            &out[0x3D96..=0x3F3F],
            &out[0x7E39..=0x7E6C],
        ];
        let bsum = backup.iter().fold(0u16, |a, s| a.wrapping_add(sum16(s)));
        assert_eq!(bsum, u16::from_le_bytes([out[0x7E6D], out[0x7E6E]]));
        assert_eq!(
            out[0x23DB..0x23DE],
            out[0x0C6B + 2..0x0C6B + 5],
            "money is in the backup too"
        );
    }

    #[test]
    fn crystal_current_box_comes_from_storage() {
        // Box 1 is current; the game loads it from storage (0x4000), not the active copy.
        let mut d = crystal_save();
        let st = List {
            at: box_offset(Game::Crystal, 0),
            cap: 20,
            rec: 32,
        };
        d[st.at] = 1;
        d[st.at + 1] = 1;
        d[st.at + 2] = 0xFF;
        d[st.rec_at(0)] = 1; // Bulbasaur
        d[st.rec_at(0) + 0x1F] = 9;
        d[st.nick_at(0)..st.nick_at(0) + 11].copy_from_slice(&text("BULBASAUR", true));
        let mut doc = parse(&d).unwrap();
        assert_eq!(
            doc.view.pointer("/PC boxes/Box 1/Slot 1/Species").unwrap(),
            "Bulbasaur"
        );
        doc.set("/PC boxes/Box 1/Slot 1/Level", Value::from(30))
            .unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[st.rec_at(0) + 0x1F], 30, "edited in storage");
        let size = st.size();
        assert_eq!(
            out[0x2D10..0x2D10 + size],
            out[st.at..st.at + size],
            "active copy mirrored"
        );
        assert_eq!(parse(&out).unwrap().view, doc.view);
    }

    #[test]
    fn contractions_round_trip() {
        for g2 in [false, true] {
            let enc = encode_text("BOB's", 11, 11, 10, &[TERM; 11], g2).unwrap();
            assert_eq!(
                enc.iter().take_while(|&&b| b != TERM).count(),
                4,
                "'s is one character"
            );
            assert_eq!(decode_text(&enc, g2).unwrap(), "BOB's");
            assert_eq!(
                decode_text(
                    &encode_text("FARFETCH'D", 11, 11, 10, &[TERM; 11], g2).unwrap(),
                    g2
                )
                .unwrap(),
                "FARFETCH'D"
            );
        }
    }

    #[test]
    fn damaged_box_index_is_safe() {
        let mut d = crystal_save();
        d[0x2700] = 0x0F; // box 16 doesn't exist
        let sum = sum16(&d[0x2009..=0x2B82]).to_le_bytes();
        d[0x2D0D..0x2D0F].copy_from_slice(&sum);
        let mut doc = parse(&d).unwrap();
        doc.set("/Trainer/Money", Value::from(1)).unwrap();
        assert!(doc.to_bytes().is_ok());
        let mut d = red_save();
        d[0x284C] = 0xFF;
        d[0x3523] = sum8(&d[0x2598..0x3523]);
        let mut doc = parse(&d).unwrap();
        doc.set("/Trainer/Money", Value::from(1)).unwrap();
        assert!(doc.to_bytes().is_ok());
    }

    /// Many random edits on both samples, saved for the independent Python validator.
    #[test]
    #[ignore]
    fn random_edit_outputs() {
        let dir = std::env::var("SAVE_EDITOR_SAMPLES").expect("SAVE_EDITOR_SAMPLES");
        let mut seed = 0x1234_5678_9ABCu64;
        let mut rnd = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n.max(1) as u64) as usize
        };
        for (name, base) in [("red", red_save()), ("crystal", crystal_save())] {
            for run in 0..40 {
                let mut doc = parse(&base).unwrap();
                let mut keys: Vec<String> = doc.editable.keys().cloned().collect();
                keys.sort();
                for _ in 0..25 {
                    let k = keys[rnd(keys.len())].clone();
                    let cur = doc.view.pointer(&k).cloned().unwrap();
                    let nv = match (&cur, doc.choices(&k)) {
                        (Value::String(_), Some(list)) => {
                            Value::from(list[rnd(list.len())].clone())
                        }
                        (Value::String(_), None) => {
                            Value::from(["RED", "BLUE", "Ash", "A-Z!", "MEW2"][rnd(5)])
                        }
                        (Value::Bool(b), _) => Value::Bool(!b),
                        (Value::Number(_), _) => Value::from(
                            [0, 1, 5, 50, 99, 100, 255, 999, 9999, 65535, 999_999][rnd(11)],
                        ),
                        _ => continue,
                    };
                    let _ = doc.set(&k, nv);
                }
                let out = doc.to_bytes().unwrap();
                let again = parse(&out).expect("edited save still opens");
                assert_eq!(again.view, doc.view, "{name} run {run}");
                std::fs::write(format!("{dir}/{name}_{run:02}.sav"), out).unwrap();
            }
        }
    }
}
