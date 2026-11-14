//! Pokémon generations 4 to 9: shared data, the Pokémon record formats (PK4 to PK9), their
//! encryption, and the save-independent part of the editing backend.
//!
//! The record layouts, shuffling tables, checksums and string tables follow PKHeX by Kurt
//! (kwsch) and contributors (GPL-3.0). The per-save wrappers (`pkm_nds`, `pkm_3ds`, `pkm_switch`)
//! find where things sit in each kind of save file and hand the rest to this module.

use crate::tree::{esc, int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

pub use super::pokemon3::{exp_for, level_for, NATURES};

pub const NONE: &str = "(none)";
pub const STATS: [&str; 6] = ["HP", "Attack", "Defense", "Speed", "Sp. Atk", "Sp. Def"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gen {
    G4,
    G5,
    G6,
    G7,
    G8,
    G9,
}

impl Gen {
    pub fn stored(self) -> usize {
        match self {
            Gen::G4 | Gen::G5 => 136,
            Gen::G6 | Gen::G7 => 232,
            Gen::G8 | Gen::G9 => 328,
        }
    }
    pub fn party(self) -> usize {
        match self {
            Gen::G4 => 236,
            Gen::G5 => 220,
            Gen::G6 | Gen::G7 => 260,
            Gen::G8 | Gen::G9 => 344,
        }
    }
    fn block(self) -> usize {
        match self {
            Gen::G4 | Gen::G5 => 32,
            Gen::G6 | Gen::G7 => 56,
            Gen::G8 | Gen::G9 => 80,
        }
    }
    fn index(self) -> usize {
        self as usize
    }
    pub fn max_species(self) -> usize {
        [493, 649, 721, 807, 898, 1025][self.index()]
    }
    pub fn max_move(self) -> usize {
        [467, 559, 621, 728, 826, 919][self.index()]
    }
    pub fn max_item(self) -> usize {
        [536, 638, 775, 959, 1607, 2600][self.index()]
    }
    pub fn max_ability(self) -> usize {
        [123, 164, 191, 233, 267, 310][self.index()]
    }
}

// ------------------------------------------------------------------ names and base stats

pub struct Names {
    pub species: Vec<String>,
    pub moves: Vec<String>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    s9n: Vec<i32>,
    s9i: Vec<i32>,
    gen4chars: Vec<char>,
}

pub fn names() -> &'static Names {
    static N: OnceLock<Names> = OnceLock::new();
    N.get_or_init(|| {
        let src = include_str!("data/pkm_names.txt");
        let line = |k: &str| src.lines().find_map(|l| l.strip_prefix(k)).unwrap_or("");
        let split = |s: &str| s.split('|').map(str::to_string).collect::<Vec<_>>();
        let nums = |s: &str| s.split(',').filter_map(|x| x.parse().ok()).collect::<Vec<i32>>();
        Names {
            species: split(line("SP=")),
            moves: split(line("MV=")),
            items: split(line("IT=")),
            abilities: split(line("AB=")),
            s9n: nums(line("S9N=")),
            s9i: nums(line("S9I=")),
            gen4chars: include_str!("data/pkm_gen4chars.txt").trim_end_matches('\n').chars().collect(),
        }
    })
}

/// A drop-down list: "(none)" first when `none`, then every named entry up to `max`.
fn pick_list(v: &[String], max: usize, none: bool) -> Arc<Vec<String>> {
    let mut l: Vec<String> = Vec::new();
    if none {
        l.push(NONE.into());
    }
    l.extend(v.iter().enumerate().filter(|(i, s)| *i >= 1 && *i <= max && !s.is_empty()).map(|(_, s)| s.clone()));
    Arc::new(l)
}

pub fn species_list(g: Gen) -> Arc<Vec<String>> {
    static L: OnceLock<[Arc<Vec<String>>; 6]> = OnceLock::new();
    L.get_or_init(|| std::array::from_fn(|i| pick_list(&names().species, [493, 649, 721, 807, 898, 1025][i], false)))[g.index()].clone()
}
pub fn move_list(g: Gen) -> Arc<Vec<String>> {
    static L: OnceLock<[Arc<Vec<String>>; 6]> = OnceLock::new();
    L.get_or_init(|| std::array::from_fn(|i| pick_list(&names().moves, [467, 559, 621, 728, 826, 919][i], true)))[g.index()].clone()
}
pub fn item_list(g: Gen) -> Arc<Vec<String>> {
    static L: OnceLock<[Arc<Vec<String>>; 6]> = OnceLock::new();
    L.get_or_init(|| std::array::from_fn(|i| pick_list(&names().items, [536, 638, 775, 959, 1607, 2600][i], true)))[g.index()].clone()
}

pub fn name_of(table: &[String], i: usize) -> String {
    match table.get(i) {
        _ if i == 0 => NONE.into(),
        Some(s) if !s.is_empty() => s.clone(),
        _ => format!("#{i}"),
    }
}

pub fn index_of(table: &[String], name: &str) -> Option<usize> {
    let n = name.trim();
    if n.eq_ignore_ascii_case(NONE) || n.is_empty() {
        return Some(0);
    }
    if let Some(num) = n.strip_prefix('#') {
        return num.parse().ok().filter(|&i: &usize| i < 0x10000);
    }
    table.iter().position(|s| !s.is_empty() && s.eq_ignore_ascii_case(n))
}

/// One species' row of the base-stat table.
#[derive(Clone, Copy, Debug, Default)]
pub struct Base {
    /// HP, Attack, Defense, Speed, Sp. Atk, Sp. Def
    pub stats: [u16; 6],
    pub growth: u8,
    pub gender: u8,
    pub abil: [u16; 3],
    pub fidx: u16,
    pub fcount: u8,
}

fn personal(g: Gen) -> &'static Vec<Base> {
    static T: OnceLock<[Vec<Base>; 6]> = OnceLock::new();
    &T.get_or_init(|| {
        let raw: [&[u8]; 6] = [
            include_bytes!("data/pkm_pers4.bin"),
            include_bytes!("data/pkm_pers5.bin"),
            include_bytes!("data/pkm_pers6.bin"),
            include_bytes!("data/pkm_pers7.bin"),
            include_bytes!("data/pkm_pers8.bin"),
            include_bytes!("data/pkm_pers9.bin"),
        ];
        std::array::from_fn(|i| {
            raw[i]
                .chunks_exact(18)
                .map(|e| Base {
                    stats: std::array::from_fn(|k| e[k] as u16),
                    growth: e[6],
                    gender: e[7],
                    abil: [u16::from_le_bytes([e[8], e[9]]), u16::from_le_bytes([e[10], e[11]]), u16::from_le_bytes([e[12], e[13]])],
                    fidx: u16::from_le_bytes([e[14], e[15]]),
                    fcount: e[16],
                })
                .collect()
        })
    })[g.index()]
}

/// Base stats of a species and form in a generation's games.
pub fn base_for(g: Gen, species: usize, form: usize) -> Option<Base> {
    let t = personal(g);
    let b = *t.get(species)?;
    if b.stats.iter().all(|&s| s == 0) {
        return None;
    }
    if form > 0 && b.fcount as usize > form && b.fidx > 0 {
        if let Some(f) = t.get(b.fidx as usize + form - 1) {
            if f.stats.iter().any(|&s| s != 0) {
                return Some(*f);
            }
        }
    }
    Some(b)
}

pub fn abilities_of(g: Gen, species: usize, form: usize) -> Vec<String> {
    let n = names();
    let Some(b) = base_for(g, species, form) else { return vec![] };
    let mut v: Vec<String> = Vec::new();
    for (i, &a) in b.abil.iter().enumerate() {
        if i == 2 && g == Gen::G4 {
            break;
        }
        if a != 0 {
            let name = name_of(&n.abilities, a as usize);
            if !v.contains(&name) {
                v.push(name);
            }
        }
    }
    v
}

/// Stats from base stats, IVs, EVs, level and nature (stored order: HP, Atk, Def, Spe, SpA, SpD).
pub fn calc_stats(g: Gen, species: usize, form: usize, ivs: [u32; 6], evs: [u32; 6], level: u32, nature: usize) -> Option<[u32; 6]> {
    let b = base_for(g, species, form)?.stats;
    let mut out = [0; 6];
    for i in 0..6 {
        let core = (2 * b[i] as u32 + ivs[i] + evs[i] / 4) * level / 100;
        out[i] = if i == 0 {
            if species == 292 { 1 } else { core + level + 10 }
        } else {
            // Natures raise stat (n / 5) and lower stat (n % 5), counting from Attack in the
            // order Attack, Defense, Speed, Sp. Atk, Sp. Def (the order stored here).
            let (up, down) = (nature / 5 + 1, nature % 5 + 1);
            let v = core + 5;
            if up == down {
                v
            } else if i == up {
                v * 110 / 100
            } else if i == down {
                v * 90 / 100
            } else {
                v
            }
        };
    }
    Some(out)
}

pub const BALLS: [&str; 28] = [
    "", "Master Ball", "Ultra Ball", "Great Ball", "Poké Ball", "Safari Ball", "Net Ball", "Dive Ball", "Nest Ball", "Repeat Ball", "Timer Ball",
    "Luxury Ball", "Premier Ball", "Dusk Ball", "Heal Ball", "Quick Ball", "Cherish Ball", "Fast Ball", "Level Ball", "Lure Ball", "Heavy Ball",
    "Love Ball", "Friend Ball", "Moon Ball", "Sport Ball", "Dream Ball", "Beast Ball", "Strange Ball",
];

// ------------------------------------------------------------------ text

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Txt {
    /// Generation 4's own character table, terminated by 0xFFFF.
    G4,
    /// UTF-16 terminated by 0xFFFF (Generation 5).
    U16F,
    /// UTF-16 terminated by 0 (Generation 6 and later).
    U16Z,
}

pub fn decode_text(k: Txt, b: &[u8]) -> Option<String> {
    let mut s = String::new();
    for w in b.chunks_exact(2) {
        let v = u16::from_le_bytes([w[0], w[1]]);
        if v == 0xFFFF || v == 0 {
            break;
        }
        match k {
            Txt::G4 => {
                let c = *names().gen4chars.get(v as usize)?;
                if c == '\u{1}' {
                    return None;
                }
                s.push(c);
            }
            _ => s.push(char::from_u32(v as u32).filter(|c| !c.is_control())?),
        }
    }
    Some(s)
}

/// `len` bytes holding `s` (at most `max` characters), terminated and zero-padded like the game does.
pub fn encode_text(k: Txt, s: &str, max: usize, len: usize) -> Result<Vec<u8>, String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return Err("a name can't be empty".into());
    }
    if chars.len() > max {
        return Err(format!("names can be at most {max} characters"));
    }
    let mut out = vec![0u8; len];
    for (i, &c) in chars.iter().enumerate() {
        let v: u16 = match k {
            Txt::G4 => {
                let t = &names().gen4chars;
                t.iter().position(|&x| x == c && x != '\u{1}').ok_or_else(|| format!("the game can't show \u{201C}{c}\u{201D}"))? as u16
            }
            _ => {
                let mut b = [0u16; 2];
                let e = c.encode_utf16(&mut b);
                if e.len() != 1 || c.is_control() {
                    return Err(format!("the game can't show \u{201C}{c}\u{201D}"));
                }
                b[0]
            }
        };
        out[i * 2..i * 2 + 2].copy_from_slice(&v.to_le_bytes());
    }
    if chars.len() * 2 < len {
        let term: u16 = if k == Txt::U16Z { 0 } else { 0xFFFF };
        out[chars.len() * 2..chars.len() * 2 + 2].copy_from_slice(&term.to_le_bytes());
    }
    Ok(out)
}

// ------------------------------------------------------------------ record encryption

const BLOCK_POS: [u8; 96] = [
    0, 1, 2, 3, 0, 1, 3, 2, 0, 2, 1, 3, 0, 3, 1, 2, 0, 2, 3, 1, 0, 3, 2, 1, 1, 0, 2, 3, 1, 0, 3, 2, 2, 0, 1, 3, 3, 0, 1, 2, 2, 0, 3, 1, 3, 0, 2, 1, 1, 2,
    0, 3, 1, 3, 0, 2, 2, 1, 0, 3, 3, 1, 0, 2, 2, 3, 0, 1, 3, 2, 0, 1, 1, 2, 3, 0, 1, 3, 2, 0, 2, 1, 3, 0, 3, 1, 2, 0, 2, 3, 1, 0, 3, 2, 1, 0,
];

fn crypt(data: &mut [u8], mut seed: u32) {
    for w in data.chunks_exact_mut(2) {
        seed = seed.wrapping_mul(0x41C6_4E6D).wrapping_add(0x6073);
        let x = (seed >> 16) as u16;
        let v = u16::from_le_bytes([w[0], w[1]]) ^ x;
        w.copy_from_slice(&v.to_le_bytes());
    }
}

fn shuffle(g: Gen, body: &[u8], sv: usize, inverse: bool) -> Vec<u8> {
    let bs = g.block();
    let sv = sv % 24;
    let pos = &BLOCK_POS[sv * 4..sv * 4 + 4];
    let mut out = vec![0u8; body.len()];
    for i in 0..4 {
        let p = pos[i] as usize;
        if inverse {
            out[p * bs..(p + 1) * bs].copy_from_slice(&body[i * bs..(i + 1) * bs]);
        } else {
            out[i * bs..(i + 1) * bs].copy_from_slice(&body[p * bs..(p + 1) * bs]);
        }
    }
    out
}

fn rd16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn rd32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

/// Decrypted record (stored or party length).
pub fn decrypt(g: Gen, enc: &[u8]) -> Vec<u8> {
    let st = g.stored();
    let mut out = enc.to_vec();
    let pv = rd32(enc, 0);
    let sv = ((pv >> 13) & 31) as usize;
    let seed = if matches!(g, Gen::G4 | Gen::G5) { rd16(enc, 6) as u32 } else { pv };
    let mut body = enc[8..st].to_vec();
    crypt(&mut body, seed);
    let body = shuffle(g, &body, sv, false);
    out[8..st].copy_from_slice(&body);
    if enc.len() > st {
        crypt(&mut out[st..], pv);
    }
    out
}

/// Encrypted record from a decrypted one; the checksum must already be right.
pub fn encrypt(g: Gen, dec: &[u8]) -> Vec<u8> {
    let st = g.stored();
    let mut out = dec.to_vec();
    let pv = rd32(dec, 0);
    let sv = ((pv >> 13) & 31) as usize;
    let seed = if matches!(g, Gen::G4 | Gen::G5) { rd16(dec, 6) as u32 } else { pv };
    let mut body = shuffle(g, &dec[8..st], sv, true);
    crypt(&mut body, seed);
    out[8..st].copy_from_slice(&body);
    if dec.len() > st {
        crypt(&mut out[st..], pv);
    }
    out
}

pub fn record_checksum(d: &[u8], g: Gen) -> u16 {
    d[8..g.stored()].chunks_exact(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])))
}

// ------------------------------------------------------------------ record layout

/// Where things sit inside a decrypted record.
pub struct Lay {
    pub species: usize,
    pub item: usize,
    pub tid: usize,
    pub sid: usize,
    pub exp: usize,
    pub friend: usize,
    pub ability: usize,
    pub ability_u16: bool,
    pub pid: usize,
    pub evs: usize,
    pub moves: usize,
    pub pp: usize,
    pub ppups: usize,
    pub iv: usize,
    pub nick: usize,
    pub nature: Option<usize>,
    pub form: usize,
    /// byte with gender in bits 1..2
    pub gender: usize,
    pub level: usize,
    pub hp_cur: usize,
    pub hp_max: usize,
}

pub fn lay(g: Gen) -> &'static Lay {
    const L4: Lay = Lay { species: 8, item: 0xA, tid: 0xC, sid: 0xE, exp: 0x10, friend: 0x14, ability: 0x15, ability_u16: false, pid: 0, evs: 0x18, moves: 0x28, pp: 0x30, ppups: 0x34, iv: 0x38, nick: 0x48, nature: None, form: 0x40, gender: 0x40, level: 0x8C, hp_cur: 0x8E, hp_max: 0x90 };
    const L5: Lay = Lay { nature: Some(0x41), ..L4 };
    const L6: Lay = Lay { species: 8, item: 0xA, tid: 0xC, sid: 0xE, exp: 0x10, friend: 0xCA, ability: 0x14, ability_u16: false, pid: 0x18, evs: 0x1E, moves: 0x5A, pp: 0x62, ppups: 0x66, iv: 0x74, nick: 0x40, nature: Some(0x1C), form: 0x1D, gender: 0x1D, level: 0xEC, hp_cur: 0xF0, hp_max: 0xF2 };
    const L8: Lay = Lay { species: 8, item: 0xA, tid: 0xC, sid: 0xE, exp: 0x10, friend: 0x112, ability: 0x14, ability_u16: true, pid: 0x1C, evs: 0x26, moves: 0x72, pp: 0x7A, ppups: 0x7E, iv: 0x8C, nick: 0x58, nature: Some(0x21), form: 0x24, gender: 0x22, level: 0x148, hp_cur: 0x8A, hp_max: 0x14A };
    match g {
        Gen::G4 => &L4,
        Gen::G5 => &L5,
        Gen::G6 | Gen::G7 => &L6,
        Gen::G8 | Gen::G9 => &L8,
    }
}

/// A decrypted Pokémon record.
#[derive(Clone)]
pub struct Pk {
    pub g: Gen,
    pub d: Vec<u8>,
}

impl Pk {
    pub fn read(g: Gen, raw: &[u8], off: usize, party: bool) -> Option<Self> {
        let len = if party { g.party() } else { g.stored() };
        let enc = raw.get(off..off + len)?;
        Some(Self { g, d: decrypt(g, enc) })
    }
    pub fn u8(&self, o: usize) -> u32 {
        self.d[o] as u32
    }
    pub fn u16(&self, o: usize) -> u32 {
        rd16(&self.d, o) as u32
    }
    pub fn u32(&self, o: usize) -> u32 {
        rd32(&self.d, o)
    }
    pub fn put(&mut self, o: usize, size: usize, v: u32) {
        self.d[o..o + size].copy_from_slice(&v.to_le_bytes()[..size]);
    }
    pub fn pid(&self) -> u32 {
        self.u32(lay(self.g).pid)
    }
    pub fn checksum_ok(&self) -> bool {
        rd16(&self.d, 6) == record_checksum(&self.d, self.g)
    }
    pub fn species(&self) -> usize {
        let raw = self.u16(lay(self.g).species) as usize;
        if self.g == Gen::G9 {
            let n = names();
            if raw >= 917 {
                if let Some(&d) = n.s9i.get(raw - 917) {
                    return (raw as i32 + d) as usize;
                }
            }
        }
        raw
    }
    pub fn set_species(&mut self, nat: usize) {
        let mut raw = nat as i32;
        if self.g == Gen::G9 && nat >= 917 {
            if let Some(&d) = names().s9n.get(nat - 917) {
                raw += d;
            }
        }
        let l = lay(self.g);
        self.put(l.species, 2, raw as u32);
    }
    pub fn form(&self) -> usize {
        let l = lay(self.g);
        match self.g {
            Gen::G4 | Gen::G5 | Gen::G6 | Gen::G7 => (self.u8(l.form) >> 3) as usize,
            _ => self.u16(l.form) as usize,
        }
    }
    pub fn exp(&self) -> u32 {
        self.u32(lay(self.g).exp)
    }
    pub fn ivs(&self) -> [u32; 6] {
        let w = self.u32(lay(self.g).iv);
        std::array::from_fn(|i| (w >> (5 * i)) & 31)
    }
    pub fn evs(&self) -> [u32; 6] {
        let o = lay(self.g).evs;
        std::array::from_fn(|i| self.u8(o + i))
    }
    pub fn is_egg(&self) -> bool {
        self.u32(lay(self.g).iv) >> 30 & 1 == 1
    }
    pub fn nature(&self) -> usize {
        match lay(self.g).nature {
            Some(o) => (self.u8(o) as usize).min(24),
            None => (self.pid() % 25) as usize,
        }
    }
    pub fn shiny(&self) -> bool {
        let l = lay(self.g);
        let (tid, sid, pid) = (self.u16(l.tid), self.u16(l.sid), self.pid());
        let x = tid ^ sid ^ (pid >> 16) ^ (pid & 0xFFFF);
        match self.g {
            Gen::G4 | Gen::G5 => x < 8,
            _ => x < 16,
        }
    }
    pub fn ball(&self) -> usize {
        match self.g {
            Gen::G4 => {
                let h = self.u8(0x86);
                (if h != 0 { h } else { self.u8(0x83) }) as usize
            }
            Gen::G5 => self.u8(0x83) as usize,
            Gen::G6 | Gen::G7 => self.u8(0xDC) as usize,
            _ => self.u8(0x124) as usize,
        }
    }
    pub fn set_ball(&mut self, v: usize) {
        let v = v as u32;
        match self.g {
            Gen::G4 => {
                if self.u8(0x86) != 0 || v > 16 {
                    self.put(0x86, 1, v);
                    self.put(0x83, 1, if v > 16 { 0 } else { 0 });
                } else {
                    self.put(0x83, 1, v);
                }
            }
            Gen::G5 => self.put(0x83, 1, v),
            Gen::G6 | Gen::G7 => self.put(0xDC, 1, v),
            _ => self.put(0x124, 1, v),
        }
    }
    /// Gender as the game stores it: 0 male, 1 female, 2 genderless.
    pub fn gender(&self) -> u32 {
        (self.u8(lay(self.g).gender) >> 1) & 3
    }
    /// Index of the ability among the species' abilities (0, 1, 2 hidden), if it matches one.
    pub fn ability_id(&self) -> usize {
        let l = lay(self.g);
        if l.ability_u16 { self.u16(l.ability) as usize } else { self.u8(l.ability) as usize }
    }
    pub fn seal(&mut self) {
        let sum = record_checksum(&self.d, self.g);
        self.d[6..8].copy_from_slice(&sum.to_le_bytes());
    }
    pub fn encrypted(&mut self) -> Vec<u8> {
        self.seal();
        encrypt(self.g, &self.d)
    }
    pub fn text(&self, o: usize, len: usize) -> Option<String> {
        decode_text(txt_kind(self.g), &self.d[o..o + len])
    }
}

pub fn txt_kind(g: Gen) -> Txt {
    match g {
        Gen::G4 => Txt::G4,
        Gen::G5 => Txt::U16F,
        _ => Txt::U16Z,
    }
}

/// Bytes of a Pokémon nickname / trainer name field.
pub fn nick_len(g: Gen) -> (usize, usize) {
    // (bytes, characters)
    match g {
        Gen::G4 | Gen::G5 => (22, 10),
        Gen::G6 | Gen::G7 => (26, 12),
        Gen::G8 | Gen::G9 => (26, 12),
    }
}

// ------------------------------------------------------------------ backend

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Species,
    Nickname,
    HeldItem,
    Experience,
    Friendship,
    Move(usize),
    Pp(usize),
    PpUp(usize),
    Ev(usize),
    Iv(usize),
    Ability,
    Nature,
    Shiny,
    Ball,
    Level,
    CurrentHp,
    Stat(usize),
}

/// How an item slot is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// u16 item id, u16 quantity
    Pair,
    /// u32: item id in the low 10 bits, quantity in the next 10 (Generation 6)
    Packed10,
    /// u32: item id in the low 10 bits, quantity in the next 10, then flags (Generation 7)
    Packed7,
    /// u32: item id in the low 11 bits, quantity in the 10 bits from bit 15, then flags (Sword / Shield)
    Swsh,
    /// 16-byte record whose item id is its position: pouch, quantity (u32 at +4), flags (Scarlet / Violet)
    Sv,
}

#[derive(Clone, Debug)]
pub enum Field {
    /// Unsigned integer of `size` bytes at a file offset.
    Int { off: usize, size: usize, max: u32 },
    Text { off: usize, bytes: usize, chars: usize, kind: Txt },
    /// Boy / Girl trainer byte (0 / 1).
    Gender { off: usize },
    Bit { off: usize, bit: u8 },
    ItemId { off: usize, slot: Slot },
    ItemQty { off: usize, slot: Slot, max: u32 },
    Mon { mon: usize, part: Part },
}

#[derive(Clone, Debug)]
pub struct Mon {
    pub off: usize,
    pub party: bool,
    pub ptr: String,
}

/// How the edited bytes become a file again (checksums, re-encryption).
#[derive(Clone, Debug)]
pub enum Finish {
    /// Nothing to fix.
    None,
    Custom(fn(&mut Vec<u8>, &[usize]) -> Result<Vec<u8>, String>, Vec<usize>),
}

#[derive(Clone, Debug)]
pub struct PkBackend {
    pub gen: Gen,
    pub raw: Vec<u8>,
    pub fields: Vec<Field>,
    pub mons: Vec<Mon>,
    pub finish: Finish,
}

fn as_str(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "expected text".into())
}

fn num_at(view: &Value, ptr: &str) -> u32 {
    view.pointer(ptr).and_then(Value::as_u64).unwrap_or(0) as u32
}

fn slot_get(raw: &[u8], off: usize, slot: Slot) -> (usize, u32) {
    match slot {
        Slot::Pair => (rd16(raw, off) as usize, rd16(raw, off + 2) as u32),
        Slot::Packed10 | Slot::Packed7 => {
            let v = rd32(raw, off);
            ((v & 0x3FF) as usize, (v >> 10) & 0x3FF)
        }
        Slot::Swsh => {
            let v = rd32(raw, off);
            ((v & 0x7FF) as usize, (v >> 15) & 0x3FF)
        }
        Slot::Sv => (0, rd32(raw, off + 4)),
    }
}

fn slot_put(raw: &mut [u8], off: usize, slot: Slot, id: Option<usize>, count: Option<u32>) {
    match slot {
        Slot::Pair => {
            if let Some(i) = id {
                raw[off..off + 2].copy_from_slice(&(i as u16).to_le_bytes());
            }
            if let Some(c) = count {
                raw[off + 2..off + 4].copy_from_slice(&(c as u16).to_le_bytes());
            }
        }
        Slot::Swsh => {
            let mut v = rd32(raw, off);
            if let Some(i) = id {
                v = v & !0x7FF | (i as u32 & 0x7FF);
            }
            if let Some(c) = count {
                v = v & !(0x3FF << 15) | ((c & 0x3FF) << 15);
            }
            raw[off..off + 4].copy_from_slice(&v.to_le_bytes());
        }
        Slot::Sv => {
            if let Some(c) = count {
                raw[off + 4..off + 8].copy_from_slice(&c.to_le_bytes());
                if c > 0 {
                    let f = rd32(raw, off + 8) | 4;
                    raw[off + 8..off + 12].copy_from_slice(&f.to_le_bytes());
                }
            }
        }
        Slot::Packed10 | Slot::Packed7 => {
            let mut v = rd32(raw, off);
            if let Some(i) = id {
                v = v & !0x3FF | (i as u32 & 0x3FF);
            }
            if let Some(c) = count {
                v = v & !(0x3FF << 10) | ((c & 0x3FF) << 10);
            }
            raw[off..off + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
}

impl PkBackend {
    fn p(&self, mon: usize, key: &str) -> String {
        format!("{}/{}", self.mons[mon].ptr, key)
    }
    fn species_in(&self, mon: usize, view: &Value) -> usize {
        view.pointer(&self.p(mon, "Species")).and_then(Value::as_str).and_then(|s| index_of(&names().species, s)).unwrap_or(0)
    }
    fn form_in(&self, mon: usize, view: &Value) -> usize {
        num_at(view, &self.p(mon, "Form")) as usize
    }
}

fn shiny_pid(g: Gen, pk: &mut Pk, on: bool) -> Result<(), String> {
    let l = lay(g);
    let (tid, sid) = (pk.u16(l.tid), pk.u16(l.sid));
    let pid = pk.pid();
    let xor_ok = |p: u32| {
        let x = tid ^ sid ^ (p >> 16) ^ (p & 0xFFFF);
        match g {
            Gen::G4 | Gen::G5 => x < 8,
            _ => x < 16,
        }
    };
    if xor_ok(pid) == on {
        return Ok(());
    }
    let is_new = !matches!(g, Gen::G4 | Gen::G5);
    if !on {
        // Flip a high bit that doesn't touch what the personality value encodes.
        let mut p = pid ^ 0x8000_0000;
        if xor_ok(p) {
            p ^= 0x4000_0000;
        }
        pk.put(l.pid, 4, p);
        if is_new {
            // The encryption constant is separate, nothing else to keep in step.
        }
        return Ok(());
    }
    let (nature, gender_byte, abil_bit) = (pk.nature() as u32, pid & 0xFF, pid & 1);
    let abil_hi = pid >> 16 & 1;
    for mid in 0..256u32 {
        let lo = (mid << 8) | gender_byte;
        for k in 0..if is_new { 16u32 } else { 8 } {
            let hi = lo ^ tid ^ sid ^ k;
            let cand = (hi << 16) | lo;
            if !xor_ok(cand) {
                continue;
            }
            let ok = match g {
                Gen::G4 => cand % 25 == nature && cand & 1 == abil_bit,
                Gen::G5 => (cand >> 16) & 1 == abil_hi,
                _ => cand & 0xFFFF == pid & 0xFFFF,
            };
            if ok {
                pk.put(l.pid, 4, cand);
                return Ok(());
            }
        }
    }
    Err("couldn't find a shiny personality value that keeps this Pokémon's nature and gender".into())
}

impl Backend for PkBackend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let g = self.gen;
        let n = names();
        let mut out = self.raw.clone();
        if changes.is_empty() {
            return match &self.finish {
                Finish::None => Ok(out),
                Finish::Custom(f, p) => f(&mut out, p),
            };
        }
        let mut mons: HashMap<usize, Pk> = HashMap::new();
        for (leaf, v) in changes {
            match &self.fields[*leaf] {
                Field::Int { off, size, max } => {
                    let x = int_in(v, 0, *max as i128, "This value")? as u32;
                    out[*off..*off + size].copy_from_slice(&x.to_le_bytes()[..*size]);
                }
                Field::Text { off, bytes, chars, kind } => {
                    let e = encode_text(*kind, as_str(v)?, *chars, *bytes)?;
                    out[*off..*off + bytes].copy_from_slice(&e);
                }
                Field::Gender { off } => out[*off] = u8::from(as_str(v)? == "Girl"),
                Field::Bit { off, bit } => {
                    let on = v.as_bool().ok_or("expected true or false")?;
                    out[*off] = if on { out[*off] | 1 << bit } else { out[*off] & !(1 << bit) };
                }
                Field::ItemId { off, slot } => {
                    let s = as_str(v)?;
                    let id = index_of(&n.items, s).ok_or("unknown item")?;
                    slot_put(&mut out, *off, *slot, Some(id), None);
                    // A slot that gets an item with no quantity yet starts at one.
                    if id != 0 && slot_get(&out, *off, *slot).1 == 0 {
                        slot_put(&mut out, *off, *slot, None, Some(1));
                    }
                    if id == 0 {
                        slot_put(&mut out, *off, *slot, None, Some(0));
                    }
                }
                Field::ItemQty { off, slot, max } => {
                    let c = int_in(v, 0, *max as i128, "This value")? as u32;
                    slot_put(&mut out, *off, *slot, None, Some(c));
                }
                Field::Mon { mon, part } => {
                    let m = &self.mons[*mon];
                    let pk = mons.entry(*mon).or_insert_with(|| Pk::read(g, &self.raw, m.off, m.party).expect("record in range"));
                    let l = lay(g);
                    let int = |max: i128| int_in(v, 0, max, "This value").map(|x| x as u32);
                    match *part {
                        Part::Species => {
                            let sp = index_of(&n.species, as_str(v)?).ok_or("unknown Pokémon")?;
                            pk.set_species(sp);
                        }
                        Part::Nickname => {
                            let (bytes, chars) = nick_len(g);
                            let e = encode_text(txt_kind(g), as_str(v)?, chars, bytes)?;
                            pk.d[l.nick..l.nick + bytes].copy_from_slice(&e);
                            // Naming it marks it as nicknamed.
                            let w = pk.u32(l.iv) | 0x8000_0000;
                            pk.put(l.iv, 4, w);
                        }
                        Part::HeldItem => pk.put(l.item, 2, index_of(&n.items, as_str(v)?).ok_or("unknown item")? as u32),
                        Part::Experience => pk.put(l.exp, 4, int(u32::MAX as i128)?),
                        Part::Friendship => pk.put(l.friend, 1, int(255)?),
                        Part::Move(i) => pk.put(l.moves + i * 2, 2, index_of(&n.moves, as_str(v)?).ok_or("unknown move")? as u32),
                        Part::Pp(i) => pk.put(l.pp + i, 1, int(255)?),
                        Part::PpUp(i) => pk.put(l.ppups + i, 1, int(3)?),
                        Part::Ev(i) => pk.put(l.evs + i, 1, int(255)?),
                        Part::Iv(i) => {
                            let w = pk.u32(l.iv) & !(31 << (5 * i));
                            pk.put(l.iv, 4, w | (int(31)? << (5 * i)));
                        }
                        Part::Ability => {
                            let want = as_str(v)?;
                            let id = index_of(&n.abilities, want).ok_or("unknown ability")?;
                            if l.ability_u16 {
                                pk.put(l.ability, 2, id as u32);
                            } else {
                                pk.put(l.ability, 1, id as u32);
                            }
                            // Which of the species' ability slots it is (Generation 6 and later store this).
                            let (sp, form) = (pk.species(), pk.form());
                            if let Some(b) = base_for(g, sp, form) {
                                let slot = b.abil.iter().position(|&a| a as usize == id).unwrap_or(0);
                                match g {
                                    Gen::G6 | Gen::G7 => pk.put(0x15, 1, 1 << slot),
                                    Gen::G8 | Gen::G9 => {
                                        let cur = pk.u8(0x16) & !7;
                                        pk.put(0x16, 1, cur | (1 << slot));
                                    }
                                    Gen::G5 => {
                                        let cur = pk.u8(0x42) & !1;
                                        pk.put(0x42, 1, cur | u32::from(slot == 2));
                                    }
                                    Gen::G4 => {}
                                }
                            }
                        }
                        Part::Nature => {
                            let want = as_str(v)?;
                            let nat = NATURES.iter().position(|x| x.eq_ignore_ascii_case(want)).ok_or("unknown nature")?;
                            match l.nature {
                                Some(o) => {
                                    pk.put(o, 1, nat as u32);
                                    // Generations 8 and 9 keep the nature it was born with separately.
                                    if matches!(g, Gen::G8 | Gen::G9) {
                                        pk.put(0x20, 1, nat as u32);
                                    }
                                }
                                None => return Err("a Generation 4 nature comes from the personality value; it can't be changed here".into()),
                            }
                        }
                        Part::Shiny => shiny_pid(g, pk, v.as_bool().ok_or("expected true or false")?)?,
                        Part::Ball => {
                            let want = as_str(v)?;
                            let ball = BALLS.iter().position(|b| !b.is_empty() && *b == want).ok_or("unknown ball")?;
                            pk.set_ball(ball);
                        }
                        Part::Level if m.party => pk.put(l.level, 1, int(100)?),
                        Part::Level => {}
                        Part::CurrentHp if m.party => pk.put(l.hp_cur, 2, int(0xFFFF)?),
                        Part::Stat(i) if m.party => pk.put(l.hp_max + i * 2, 2, int(0xFFFF)?),
                        Part::CurrentHp | Part::Stat(_) => {}
                    }
                }
            }
        }
        for (mon, mut pk) in mons {
            let enc = pk.encrypted();
            let off = self.mons[mon].off;
            out[off..off + enc.len()].copy_from_slice(&enc);
        }
        match &self.finish {
            Finish::None => Ok(out),
            Finish::Custom(f, p) => f(&mut out, p),
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let g = self.gen;
        let n = names();
        let known = |table: &[String], what: &str| -> Result<(), String> {
            let s = as_str(v)?;
            index_of(table, s).map(|_| ()).ok_or_else(|| format!("there's no {what} called \u{201C}{s}\u{201D}"))
        };
        match &self.fields[leaf] {
            Field::Int { max, .. } | Field::ItemQty { max, .. } => int_in(v, 0, *max as i128, "This value").map(|_| ()),
            Field::Text { chars, kind, bytes, .. } => encode_text(*kind, as_str(v)?, *chars, *bytes).map(|_| ()),
            Field::Gender { .. } => matches!(as_str(v)?, "Boy" | "Girl").then_some(()).ok_or_else(|| "choose Boy or Girl".into()),
            Field::Bit { .. } => v.as_bool().map(|_| ()).ok_or_else(|| "expected true or false".into()),
            Field::ItemId { .. } => known(&n.items, "item"),
            Field::Mon { part, .. } => match part {
                Part::Species => match index_of(&n.species, as_str(v)?) {
                    Some(0) | None => Err(format!("there's no Pokémon called \u{201C}{}\u{201D}", as_str(v)?)),
                    Some(i) if i > g.max_species() => Err("this game doesn't have that Pokémon".into()),
                    Some(_) => Ok(()),
                },
                Part::Nickname => {
                    let (bytes, chars) = nick_len(g);
                    encode_text(txt_kind(g), as_str(v)?, chars, bytes).map(|_| ())
                }
                Part::HeldItem => known(&n.items, "item"),
                Part::Move(_) => known(&n.moves, "move"),
                Part::Ability => known(&n.abilities, "ability"),
                Part::Nature => {
                    let s = as_str(v)?;
                    NATURES.iter().any(|x| x.eq_ignore_ascii_case(s)).then_some(()).ok_or_else(|| "unknown nature".into())
                }
                Part::Shiny => v.as_bool().map(|_| ()).ok_or_else(|| "expected true or false".into()),
                Part::Ball => Ok(()),
                Part::Iv(_) => int_in(v, 0, 31, "IVs").map(|_| ()),
                Part::Level => int_in(v, 1, 100, "Level").map(|_| ()),
                Part::Experience => int_in(v, 0, 2_000_000, "Experience").map(|_| ()),
                Part::PpUp(_) => int_in(v, 0, 3, "PP Ups").map(|_| ()),
                Part::Friendship | Part::Pp(_) | Part::Ev(_) => int_in(v, 0, 255, "This value").map(|_| ()),
                Part::CurrentHp | Part::Stat(_) => int_in(v, 0, 999, "Stats").map(|_| ()),
            },
        }
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Field::Mon { mon, part } = &self.fields[leaf] else { return vec![] };
        let (mon, part) = (*mon, *part);
        let g = self.gen;
        let p = |k: &str| self.p(mon, k);
        let species = self.species_in(mon, view);
        let form = self.form_in(mon, view);
        let Some(b) = base_for(g, species, form) else { return vec![] };
        let growth = b.growth as u16;
        let mut out = Vec::new();
        let mut level = num_at(view, &p("Level"));
        match part {
            Part::Level => out.push((p("Experience"), Value::from(exp_for(growth, level)))),
            Part::Experience | Part::Species => {
                level = level_for(growth, num_at(view, &p("Experience")));
                out.push((p("Level"), Value::from(level)));
            }
            Part::Iv(_) | Part::Ev(_) | Part::Nature => {}
            _ => return vec![],
        }
        if part == Part::Species {
            let cur = view.pointer(&p("Ability")).and_then(Value::as_str).unwrap_or("");
            let opts = abilities_of(g, species, form);
            if !opts.is_empty() {
                out.push((p("Ability"), Value::from(if opts.iter().any(|a| a == cur) { cur.to_string() } else { opts[0].clone() })));
            }
        }
        if self.mons[mon].party {
            let ivs = std::array::from_fn(|i| num_at(view, &p(&format!("IVs/{}", STATS[i]))));
            let evs = std::array::from_fn(|i| num_at(view, &p(&format!("EVs/{}", STATS[i]))));
            let nature = view.pointer(&p("Nature")).and_then(Value::as_str).and_then(|n| NATURES.iter().position(|x| *x == n)).unwrap_or(0);
            if let Some(stats) = calc_stats(g, species, form, ivs, evs, level, nature) {
                let old_max = num_at(view, &p("Stats/Max HP"));
                let cur = num_at(view, &p("Stats/Current HP"));
                let new_cur = (cur + stats[0].saturating_sub(old_max)).min(stats[0]);
                out.push((p("Stats/Max HP"), Value::from(stats[0])));
                out.push((p("Stats/Current HP"), Value::from(new_cur)));
                for i in 1..6 {
                    out.push((p(&format!("Stats/{}", STATS[i])), Value::from(stats[i])));
                }
            }
        }
        out
    }

    fn choices(&self, leaf: usize, view: &Value) -> Option<Arc<Vec<String>>> {
        let g = self.gen;
        match &self.fields[leaf] {
            Field::Gender { .. } => Some(Arc::new(vec!["Boy".into(), "Girl".into()])),
            Field::ItemId { .. } => Some(item_list(g)),
            Field::Mon { mon, part } => match part {
                Part::Species => Some(species_list(g)),
                Part::HeldItem => Some(item_list(g)),
                Part::Move(_) => Some(move_list(g)),
                Part::Nature => Some(Arc::new(NATURES.iter().map(|s| s.to_string()).collect())),
                Part::Ball => Some(Arc::new(BALLS[1..].iter().map(|s| s.to_string()).collect())),
                Part::Ability => {
                    let l = abilities_of(g, self.species_in(*mon, view), self.form_in(*mon, view));
                    (!l.is_empty()).then(|| Arc::new(l))
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

// ------------------------------------------------------------------ view building

#[derive(Default)]
pub struct Builder {
    pub fields: Vec<Field>,
    pub mons: Vec<Mon>,
    pub editable: HashMap<String, usize>,
    pub gen: Option<Gen>,
}

impl Builder {
    pub fn add(&mut self, map: &mut Map<String, Value>, base: &str, key: &str, v: Value, f: Option<Field>) {
        if let Some(f) = f {
            self.editable.insert(format!("{base}/{}", esc(key)), self.fields.len());
            self.fields.push(f);
        }
        map.insert(key.to_string(), v);
    }

    /// An integer at a file offset (little endian).
    pub fn int(&mut self, map: &mut Map<String, Value>, base: &str, key: &str, raw: &[u8], off: usize, size: usize, max: u32) {
        let mut b = [0u8; 4];
        b[..size].copy_from_slice(&raw[off..off + size]);
        self.add(map, base, key, Value::from(u32::from_le_bytes(b)), Some(Field::Int { off, size, max }));
    }

    pub fn text(&mut self, map: &mut Map<String, Value>, base: &str, key: &str, raw: &[u8], off: usize, bytes: usize, chars: usize, kind: Txt) {
        match decode_text(kind, &raw[off..off + bytes]) {
            Some(s) => self.add(map, base, key, Value::from(s), Some(Field::Text { off, bytes, chars, kind })),
            None => self.add(map, base, key, Value::from("?"), None),
        }
    }

    /// One bag pocket: `slots` entries (Item + Quantity) starting at `off`.
    #[allow(clippy::too_many_arguments)]
    pub fn pocket(&mut self, bag: &mut Map<String, Value>, title: &str, raw: &[u8], off: usize, slots: usize, slot: Slot, stride: usize, max_qty: u32, g: Gen) {
        let t = names();
        let mut pocket = Map::new();
        let pp = format!("/Bag/{}", esc(title));
        for i in 0..slots {
            let at = off + i * stride;
            let (id, qty) = slot_get(raw, at, slot);
            let mut s = Map::new();
            let key = format!("Slot {}", i + 1);
            let sp = format!("{pp}/{}", esc(&key));
            let _ = g;
            self.add(&mut s, &sp, "Item", Value::from(name_of(&t.items, id)), Some(Field::ItemId { off: at, slot }));
            self.add(&mut s, &sp, "Quantity", Value::from(qty), Some(Field::ItemQty { off: at, slot, max: max_qty }));
            pocket.insert(key, Value::Object(s));
        }
        bag.insert(title.into(), Value::Object(pocket));
    }

    /// One Pokémon at `off`, or `None` for an empty or damaged slot.
    pub fn mon(&mut self, g: Gen, raw: &[u8], off: usize, party: bool, ptr: &str) -> Option<Value> {
        let t = names();
        let l = lay(g);
        let pk = Pk::read(g, raw, off, party)?;
        let species = pk.species();
        if species == 0 || species > g.max_species() || (pk.u32(0) == 0 && pk.u32(4) == 0) {
            return None;
        }
        let ok = pk.checksum_ok();
        let mon = self.mons.len();
        let mf = |part| ok.then_some(Field::Mon { mon, part });
        self.mons.push(Mon { off, party, ptr: ptr.to_string() });
        let form = pk.form();
        let base = base_for(g, species, form);
        let egg = pk.is_egg();
        let mut m = Map::new();
        if !ok {
            m.insert("Note".into(), Value::from("This Pokémon's data is damaged (a \u{201C}Bad Egg\u{201D}); it's shown read-only."));
        }
        self.add(&mut m, ptr, "Species", Value::from(name_of(&t.species, species)), mf(Part::Species));
        if form != 0 {
            m.insert("Form".into(), Value::from(form));
        }
        let (nb, nc) = nick_len(g);
        let nick = pk.text(l.nick, nb);
        let nf = if nick.is_some() && !egg { mf(Part::Nickname) } else { None };
        self.add(&mut m, ptr, "Nickname", Value::from(nick.unwrap_or_else(|| "?".into())), nf);
        let _ = nc;
        let exp = pk.exp();
        let level = if party { pk.u8(l.level) } else { base.map_or(1, |b| level_for(b.growth as u16, exp)) };
        self.add(&mut m, ptr, "Level", Value::from(level), mf(Part::Level));
        self.add(&mut m, ptr, "Experience", Value::from(exp), mf(Part::Experience));
        self.add(&mut m, ptr, "Nature", Value::from(NATURES[pk.nature()]), mf(Part::Nature).filter(|_| l.nature.is_some()));
        self.add(&mut m, ptr, "Shiny", Value::Bool(pk.shiny()), mf(Part::Shiny));
        if egg {
            m.insert("Egg".into(), Value::Bool(true));
        }
        match pk.gender() {
            0 => m.insert("Gender".into(), Value::from("Male")),
            1 => m.insert("Gender".into(), Value::from("Female")),
            _ => None,
        };
        let abil = name_of(&t.abilities, pk.ability_id());
        let has_opts = !abilities_of(g, species, form).is_empty();
        self.add(&mut m, ptr, "Ability", Value::from(abil), mf(Part::Ability).filter(|_| has_opts));
        self.add(&mut m, ptr, "Held item", Value::from(name_of(&t.items, pk.u16(l.item) as usize)), mf(Part::HeldItem));
        self.add(&mut m, ptr, "Friendship", Value::from(pk.u8(l.friend)), mf(Part::Friendship));
        let ball = pk.ball();
        if (1..BALLS.len()).contains(&ball) {
            self.add(&mut m, ptr, "Poké Ball", Value::from(BALLS[ball]), mf(Part::Ball));
        }
        let mut moves = Map::new();
        let mptr = format!("{ptr}/Moves");
        for i in 0..4 {
            self.add(&mut moves, &mptr, &format!("Move {}", i + 1), Value::from(name_of(&t.moves, pk.u16(l.moves + i * 2) as usize)), mf(Part::Move(i)));
            self.add(&mut moves, &mptr, &format!("PP {}", i + 1), Value::from(pk.u8(l.pp + i)), mf(Part::Pp(i)));
            self.add(&mut moves, &mptr, &format!("PP Ups {}", i + 1), Value::from(pk.u8(l.ppups + i)), mf(Part::PpUp(i)));
        }
        m.insert("Moves".into(), Value::Object(moves));
        for (title, vals, part) in [("IVs", pk.ivs(), Part::Iv as fn(usize) -> Part), ("EVs", pk.evs(), Part::Ev as fn(usize) -> Part)] {
            let mut s = Map::new();
            let sp = format!("{ptr}/{title}");
            for i in 0..6 {
                self.add(&mut s, &sp, STATS[i], Value::from(vals[i]), mf(part(i)));
            }
            m.insert(title.into(), Value::Object(s));
        }
        if party {
            let mut s = Map::new();
            let sp = format!("{ptr}/Stats");
            self.add(&mut s, &sp, "Current HP", Value::from(pk.u16(l.hp_cur)), mf(Part::CurrentHp));
            self.add(&mut s, &sp, "Max HP", Value::from(pk.u16(l.hp_max)), mf(Part::Stat(0)));
            for i in 1..6 {
                self.add(&mut s, &sp, STATS[i], Value::from(pk.u16(l.hp_max + i * 2)), mf(Part::Stat(i)));
            }
            m.insert("Stats".into(), Value::Object(s));
        }
        Some(Value::Object(m))
    }

    pub fn finish(self, gen: Gen, raw: Vec<u8>, root: Map<String, Value>, kind: &str, finish: Finish, note: &str) -> TreeDoc {
        let backend = PkBackend { gen, raw, fields: self.fields, mons: self.mons, finish };
        TreeDoc::new(Value::Object(root), self.editable, kind, Box::new(backend)).with_note(note)
    }
}

pub fn crc16_ccitt(d: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in d {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

#[cfg(test)]
pub fn verify_party_stats(view: &Value, g: Gen) {
    let Some(Value::Object(party)) = view.get("Party") else { return };
    for (k, v) in party {
        let n = |p: &str| v.pointer(p).and_then(Value::as_u64).unwrap_or(0) as u32;
        let sp = v["Species"].as_str().and_then(|s| index_of(&names().species, s)).unwrap_or(0);
        let ivs = std::array::from_fn(|i| n(&format!("/IVs/{}", STATS[i])));
        let evs = std::array::from_fn(|i| n(&format!("/EVs/{}", STATS[i])));
        let nat = NATURES.iter().position(|x| Some(*x) == v["Nature"].as_str()).unwrap_or(0);
        let form = v["Form"].as_u64().unwrap_or(0) as usize;
        let Some(calc) = calc_stats(g, sp, form, ivs, evs, n("/Level"), nat) else { continue };
        let stored: Vec<u32> = (0..6).map(|i| n(&format!("/Stats/{}", if i == 0 { "Max HP" } else { STATS[i] }))).collect();
        println!("  {k}: stored {stored:?} calc {calc:?}");
        assert!(stored.iter().zip(calc).all(|(a, b)| a.abs_diff(b) <= 3), "{k}: stats don't match");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_line_up() {
        let n = names();
        assert_eq!(n.species[1], "Bulbasaur");
        assert_eq!(n.species[493], "Arceus");
        assert_eq!(n.species[1025], "Pecharunt");
        assert_eq!(n.moves[1], "Pound");
        assert_eq!(n.abilities[26], "Levitate");
        assert_eq!(n.items[4], "Poké Ball");
        assert_eq!(n.gen4chars[0x121], '0');
        let b = base_for(Gen::G4, 1, 0).unwrap();
        assert_eq!(b.stats, [45, 49, 49, 45, 65, 65]);
        assert_eq!(base_for(Gen::G9, 25, 0).unwrap().stats[..3], [35, 55, 40]);
    }

    #[test]
    fn crypto_round_trip() {
        for g in [Gen::G4, Gen::G5, Gen::G6, Gen::G7, Gen::G8, Gen::G9] {
            let mut d = vec![0u8; g.party()];
            for (i, b) in d.iter_mut().enumerate() {
                *b = (i * 31 + 7) as u8;
            }
            d[0..4].copy_from_slice(&0x1234_5678u32.to_le_bytes());
            let sum = record_checksum(&d, g);
            d[6..8].copy_from_slice(&sum.to_le_bytes());
            let e = encrypt(g, &d);
            assert_ne!(e, d);
            assert_eq!(decrypt(g, &e), d, "{g:?}");
        }
    }

    #[test]
    fn crc_known_value() {
        // CRC-16/CCITT-FALSE check value
        assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
    }

    #[test]
    fn text_round_trip() {
        for k in [Txt::G4, Txt::U16F, Txt::U16Z] {
            let e = encode_text(k, "Ash", 7, 16).unwrap();
            assert_eq!(decode_text(k, &e).unwrap(), "Ash");
        }
        assert!(encode_text(Txt::U16Z, "Ashketchum", 7, 16).is_err());
    }
}
