//! Pokémon Ruby / Sapphire / Emerald / FireRed / LeafGreen (Game Boy Advance) saves,
//! as written by emulators (`.sav`, 128 KiB flash).
//!
//! The file holds two copies of the game (A/B), each split into 14 checksummed 4 KiB
//! sections in rotating order. Only the newest copy is edited. Pokémon data is
//! encrypted per Pokémon (personality XOR trainer ID) with its own checksum, and
//! money / bag quantities are XORed with a per-save security key. All of that is
//! redone on save, so the game accepts the file.

use crate::tree::{int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

const SECTION: usize = 0x1000;
const DATA: usize = 3968;
const BLOCK: usize = 14 * SECTION;
const SIGNATURE: u32 = 0x0801_2025;
const SIZES: [usize; 14] = [3884, 3968, 3968, 3968, 3848, 3968, 3968, 3968, 3968, 3968, 3968, 3968, 3968, 2000];
/// PC storage starts in section 5; addresses at or above this are PC-buffer offsets.
const PC: usize = 0x10000;

// ------------------------------------------------------------------ game data

pub struct Tables {
    pub species: Vec<String>,
    pub moves: Vec<String>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    /// hp, atk, def, spe, spa, spd, growth rate, ability 1, ability 2, gender ratio
    pub base: Vec<Option<[u16; 10]>>,
    species_list: Arc<Vec<String>>,
    moves_list: Arc<Vec<String>>,
    items_list: Arc<Vec<String>>,
}

pub fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let src = include_str!("data/gen3.txt");
        let line = |k: &str| src.lines().find_map(|l| l.strip_prefix(k)).unwrap_or("");
        let split = |s: &str| s.split('|').map(str::to_string).collect::<Vec<_>>();
        let mut moves = split(line("MV="));
        moves[0] = String::new();
        let base = line("BASE=")
            .split(';')
            .map(|e| {
                let v: Vec<u16> = e.split(',').filter_map(|x| x.parse().ok()).collect();
                (v.len() == 10).then(|| v.try_into().unwrap())
            })
            .collect();
        let species = split(line("SP="));
        let items = split(line("IT="));
        let list = |v: &[String], none: bool| {
            let mut l: Vec<String> = Vec::new();
            if none {
                l.push(NONE.into());
            }
            l.extend(v.iter().filter(|s| !s.is_empty()).cloned());
            Arc::new(l)
        };
        Tables {
            species_list: list(&species, false),
            moves_list: list(&moves, true),
            items_list: list(&items, true),
            species,
            moves,
            items,
            abilities: split(line("AB=")),
            base,
        }
    })
}

const NONE: &str = "(none)";
pub const NATURES: [&str; 25] = [
    "Hardy", "Lonely", "Brave", "Adamant", "Naughty", "Bold", "Docile", "Relaxed", "Impish", "Lax", "Timid", "Hasty", "Serious", "Jolly",
    "Naive", "Modest", "Mild", "Quiet", "Bashful", "Rash", "Calm", "Gentle", "Sassy", "Careful", "Quirky",
];
const BALLS: [&str; 13] = [
    "", "Master Ball", "Ultra Ball", "Great Ball", "Poké Ball", "Safari Ball", "Net Ball", "Dive Ball", "Nest Ball", "Repeat Ball", "Timer Ball",
    "Luxury Ball", "Premier Ball",
];
const STATS: [&str; 6] = ["HP", "Attack", "Defense", "Speed", "Sp. Atk", "Sp. Def"];

/// Name for an index in a table, or `#n` when the table has no name for it.
fn name_of(table: &[String], i: usize) -> String {
    match table.get(i) {
        _ if i == 0 => NONE.into(),
        Some(s) if !s.is_empty() => s.clone(),
        _ => format!("#{i}"),
    }
}

/// Index for a name (case-insensitive) or `#n`.
fn index_of(table: &[String], name: &str) -> Option<usize> {
    let n = name.trim();
    if n.eq_ignore_ascii_case(NONE) || n.is_empty() {
        return Some(0);
    }
    if let Some(num) = n.strip_prefix('#') {
        return num.parse().ok().filter(|&i: &usize| i < 0x10000);
    }
    table.iter().position(|s| !s.is_empty() && s.eq_ignore_ascii_case(n))
}

pub fn exp_for(growth: u16, n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    let n = n as i64;
    let c = n * n * n;
    let e = match growth {
        1 if n <= 50 => (100 - n) * c / 50,
        1 if n <= 68 => (150 - n) * c / 100,
        1 if n <= 98 => ((1911 - 10 * n) / 3) * c / 500,
        1 => (160 - n) * c / 100,
        2 if n <= 15 => ((n + 1) / 3 + 24) * c / 50,
        2 if n <= 36 => (n + 14) * c / 50,
        2 => ((n / 2) + 32) * c / 50,
        3 => 6 * c / 5 - 15 * n * n + 100 * n - 140,
        4 => 4 * c / 5,
        5 => 5 * c / 4,
        _ => c,
    };
    e.max(0) as u32
}

pub fn level_for(growth: u16, exp: u32) -> u32 {
    let mut level = 1;
    while level <= 100 && exp_for(growth, level) <= exp {
        level += 1;
    }
    (level - 1).max(1)
}

/// Party stats from base stats, IVs, EVs, level and nature (index 0 = HP, then
/// Attack, Defense, Speed, Sp. Atk, Sp. Def — the order they're stored in).
pub fn calc_stats(species: usize, ivs: [u32; 6], evs: [u32; 6], level: u32, nature: usize) -> Option<[u32; 6]> {
    let b = tables().base.get(species).copied().flatten()?;
    let mut out = [0; 6];
    for i in 0..6 {
        let core = (2 * b[i] as u32 + ivs[i] + evs[i] / 4) * level / 100;
        out[i] = if i == 0 {
            if species == 303 { 1 } else { core + level + 10 } // Shedinja always has 1 HP
        } else {
            // Nature: +10% to stat (n / 5), −10% to stat (n % 5), counting from Attack.
            // Stored order is Atk, Def, Spe, SpA, SpD — same as the nature order.
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

// ------------------------------------------------------------------ text

fn charset() -> &'static [Option<char>; 256] {
    static C: OnceLock<[Option<char>; 256]> = OnceLock::new();
    C.get_or_init(|| {
        let mut t = [None; 256];
        t[0] = Some(' ');
        for (i, c) in "ÀÁÂÇÈÉÊËÌ".chars().enumerate() {
            t[1 + i] = Some(c);
        }
        for (i, c) in [(0x0B, 'Î'), (0x0C, 'Ï'), (0x0D, 'Ò'), (0x0E, 'Ó'), (0x0F, 'Ô'), (0x10, 'Œ'), (0x11, 'Ù'), (0x12, 'Ú'), (0x13, 'Û'), (0x14, 'Ñ'), (0x15, 'ß'), (0x16, 'à'), (0x17, 'á'), (0x19, 'ç'), (0x1A, 'è'), (0x1B, 'é'), (0x1C, 'ê'), (0x1D, 'ë'), (0x1E, 'ì'), (0x20, 'î'), (0x21, 'ï'), (0x22, 'ò'), (0x23, 'ó'), (0x24, 'ô'), (0x25, 'œ'), (0x26, 'ù'), (0x27, 'ú'), (0x28, 'û'), (0x29, 'ñ'), (0x2A, 'º'), (0x2B, 'ª'), (0x2D, '&'), (0x2E, '+'), (0x35, '='), (0x36, ';'), (0x51, '¿'), (0x52, '¡'), (0x5A, 'Í'), (0x5B, '%'), (0x5C, '('), (0x5D, ')'), (0x68, 'â'), (0x6F, 'í'), (0x85, '<'), (0x86, '>'), (0xAB, '!'), (0xAC, '?'), (0xAD, '.'), (0xAE, '-'), (0xAF, '·'), (0xB0, '…'), (0xB1, '“'), (0xB2, '”'), (0xB3, '‘'), (0xB4, '’'), (0xB5, '♂'), (0xB6, '♀'), (0xB7, '¥'), (0xB8, ','), (0xB9, '×'), (0xBA, '/'), (0xF0, ':'), (0xF1, 'Ä'), (0xF2, 'Ö'), (0xF3, 'Ü'), (0xF4, 'ä'), (0xF5, 'ö'), (0xF6, 'ü')] {
            t[i] = Some(c);
        }
        for i in 0..10 {
            t[0xA1 + i] = char::from_digit(i as u32, 10);
        }
        for i in 0..26u8 {
            t[0xBB + i as usize] = Some((b'A' + i) as char);
            t[0xD5 + i as usize] = Some((b'a' + i) as char);
        }
        t
    })
}

/// Decode a fixed-length game string. `None` if it uses characters we can't edit safely.
pub fn decode_text(b: &[u8]) -> Option<String> {
    let mut s = String::new();
    for &c in b {
        if c == 0xFF {
            break;
        }
        s.push(charset()[c as usize]?);
    }
    Some(s)
}

pub fn encode_text(s: &str, len: usize, orig: &[u8]) -> Result<Vec<u8>, String> {
    let t = charset();
    let mut out = Vec::with_capacity(len);
    for ch in s.chars() {
        let b = t.iter().position(|&c| c == Some(ch)).ok_or_else(|| format!("the game can't show \u{201C}{ch}\u{201D}"))?;
        out.push(b as u8);
    }
    if out.len() > len {
        return Err(format!("names can be at most {len} characters"));
    }
    if out.is_empty() {
        return Err("a name can't be empty".into());
    }
    // Terminator, then the same filler the game used (0xFF or 0x00).
    let fill = if orig.iter().skip_while(|&&b| b != 0xFF).skip(1).all(|&b| b == 0xFF) { 0xFF } else { 0x00 };
    if out.len() < len {
        out.push(0xFF);
    }
    out.resize(len, fill);
    Ok(out)
}

// ------------------------------------------------------------------ save layout

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    RubySapphire,
    Emerald,
    FireRedLeafGreen,
}

impl Game {
    fn name(self) -> &'static str {
        match self {
            Game::RubySapphire => "Pokémon Ruby / Sapphire",
            Game::Emerald => "Pokémon Emerald",
            Game::FireRedLeafGreen => "Pokémon FireRed / LeafGreen",
        }
    }
}

#[derive(Clone, Debug)]
struct Layout {
    /// File offset of each section id in the current block.
    sec: [usize; 14],
    game: Game,
    key: u32,
}

impl Layout {
    /// File offset of a logical address (section * 0x1000 + offset, or PC + offset).
    fn at(&self, addr: usize) -> usize {
        if addr >= PC {
            let p = addr - PC;
            self.sec[5 + p / DATA] + p % DATA
        } else {
            self.sec[addr / SECTION] + addr % SECTION
        }
    }
    fn read(&self, d: &[u8], addr: usize, len: usize) -> Vec<u8> {
        (0..len).map(|i| d[self.at(addr + i)]).collect()
    }
    fn write(&self, d: &mut [u8], addr: usize, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            d[self.at(addr + i)] = *b;
        }
    }
    fn u(&self, d: &[u8], addr: usize, size: usize) -> u32 {
        self.read(d, addr, size).iter().rev().fold(0, |a, &b| (a << 8) | b as u32)
    }
}

fn checksum(b: &[u8], n: usize) -> u16 {
    let s = b[..n].chunks_exact(4).fold(0u32, |a, w| a.wrapping_add(u32::from_le_bytes(w.try_into().unwrap())));
    ((s >> 16) as u16).wrapping_add(s as u16)
}

fn rd32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

/// Section map of one block if every section is present with a valid signature and checksum.
fn block(d: &[u8], base: usize) -> Option<([usize; 14], u32)> {
    let mut sec = [usize::MAX; 14];
    let mut index = 0;
    for i in 0..14 {
        let o = base + i * SECTION;
        let id = u16::from_le_bytes([d[o + 0xFF4], d[o + 0xFF5]]) as usize;
        if id >= 14 || rd32(d, o + 0xFF8) != SIGNATURE || sec[id] != usize::MAX {
            return None;
        }
        if checksum(&d[o..], SIZES[id]) != u16::from_le_bytes([d[o + 0xFF6], d[o + 0xFF7]]) {
            return None;
        }
        sec[id] = o;
        index = rd32(d, o + 0xFFC);
    }
    Some((sec, index))
}

pub fn looks(d: &[u8]) -> bool {
    // 128 KiB, optionally followed by a few bytes an emulator appended (clock data).
    (0x20000..=0x20000 + 0x200).contains(&d.len()) && (block(d, 0).is_some() || block(d, BLOCK).is_some())
}

fn layout(d: &[u8]) -> Option<Layout> {
    let a = block(d, 0);
    let b = block(d, BLOCK);
    let (sec, _) = match (a, b) {
        (Some(a), Some(b)) => {
            if a.1 > b.1 { a } else { b }
        }
        (Some(x), None) | (None, Some(x)) => x,
        _ => return None,
    };
    let s0 = sec[0];
    let s1 = sec[1];
    let game_code = rd32(d, s0 + 0xAC);
    let plausible = |money: u32, team_off: usize| money <= 999_999 && rd32(d, s1 + team_off) <= 6;
    let game = if game_code == 1 {
        Game::FireRedLeafGreen
    } else if game_code != 0 && plausible(rd32(d, s1 + 0x490) ^ game_code, 0x234) {
        Game::Emerald
    } else if plausible(rd32(d, s1 + 0x490), 0x234) {
        Game::RubySapphire
    } else {
        return None;
    };
    let key = match game {
        Game::RubySapphire => 0,
        Game::Emerald => game_code,
        Game::FireRedLeafGreen => rd32(d, s0 + 0xF20),
    };
    Some(Layout { sec, game, key })
}

// ------------------------------------------------------------------ fields

#[derive(Clone, Debug)]
enum Field {
    /// Unsigned integer; `xor` = 2 or 4 bytes of the security key applied.
    Int { addr: usize, size: usize, max: u32, xor: bool },
    Text { addr: usize, len: usize },
    Gender { addr: usize },
    /// Item id in a bag/PC slot.
    Item { addr: usize },
    Mon { mon: usize, part: MonPart },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MonPart {
    Species,
    Nickname,
    HeldItem,
    Experience,
    Friendship,
    Move(usize),
    Pp(usize),
    Ev(usize),
    Iv(usize),
    Ability,
    Ball,
    /// Party only: level byte, current HP, stats.
    Level,
    CurrentHp,
    Stat(usize),
}

#[derive(Clone, Debug)]
struct Mon {
    addr: usize,
    party: bool,
    /// View pointer of this Pokémon's object.
    ptr: String,
}

// Substructure offsets within the decrypted 48 bytes, by personality % 24.
fn sub_offset(pid: u32, sub: usize) -> usize {
    const ORDER: [&str; 24] = [
        "GAEM", "GAME", "GEAM", "GEMA", "GMAE", "GMEA", "AGEM", "AGME", "AEGM", "AEMG", "AMGE", "AMEG", "EGAM", "EGMA", "EAGM", "EAMG", "EMGA",
        "EMAG", "MGAE", "MGEA", "MAGE", "MAEG", "MEGA", "MEAG",
    ];
    let c = b"GAEM"[sub];
    ORDER[(pid % 24) as usize].bytes().position(|b| b == c).unwrap() * 12
}

const G: usize = 0;
const A: usize = 1;
const E: usize = 2;
const M: usize = 3;

/// A Pokémon record with its 48 data bytes decrypted.
#[derive(Clone)]
struct MonData {
    head: Vec<u8>,
    data: [u8; 48],
}

impl MonData {
    fn read(l: &Layout, d: &[u8], addr: usize, party: bool) -> Self {
        let head = l.read(d, addr, if party { 100 } else { 80 });
        let pid = rd32(&head, 0);
        let k = pid ^ rd32(&head, 4);
        let mut data = [0u8; 48];
        for i in 0..12 {
            let w = rd32(&head, 32 + i * 4) ^ k;
            data[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        Self { head, data }
    }
    fn pid(&self) -> u32 {
        rd32(&self.head, 0)
    }
    fn get(&self, sub: usize, off: usize, size: usize) -> u32 {
        let o = sub_offset(self.pid(), sub) + off;
        self.data[o..o + size].iter().rev().fold(0, |a, &b| (a << 8) | b as u32)
    }
    fn set(&mut self, sub: usize, off: usize, size: usize, v: u32) {
        let o = sub_offset(self.pid(), sub) + off;
        self.data[o..o + size].copy_from_slice(&v.to_le_bytes()[..size]);
    }
    fn checksum_ok(&self) -> bool {
        u16::from_le_bytes([self.head[28], self.head[29]]) == self.calc_checksum()
    }
    fn calc_checksum(&self) -> u16 {
        self.data.chunks_exact(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])))
    }
    /// Re-encrypt into `head` (with a fresh checksum).
    fn seal(&mut self) {
        let sum = self.calc_checksum();
        self.head[28..30].copy_from_slice(&sum.to_le_bytes());
        let k = self.pid() ^ rd32(&self.head, 4);
        for i in 0..12 {
            let w = rd32(&self.data, i * 4) ^ k;
            self.head[32 + i * 4..36 + i * 4].copy_from_slice(&w.to_le_bytes());
        }
    }
    fn species(&self) -> usize {
        self.get(G, 0, 2) as usize
    }
    fn ivs(&self) -> [u32; 6] {
        let w = self.get(M, 4, 4);
        std::array::from_fn(|i| (w >> (5 * i)) & 31)
    }
    fn evs(&self) -> [u32; 6] {
        std::array::from_fn(|i| self.get(E, i, 1))
    }
}

// ------------------------------------------------------------------ backend

#[derive(Clone, Debug)]
struct Gen3Backend {
    raw: Vec<u8>,
    layout: Layout,
    fields: Vec<Field>,
    mons: Vec<Mon>,
}

/// The two ability names a species can have (the same name twice if it has one).
fn abilities_of(species: usize) -> [String; 2] {
    let t = tables();
    let b = t.base.get(species).copied().flatten().unwrap_or([0; 10]);
    let a1 = name_of(&t.abilities, b[7] as usize);
    let a2 = if b[8] == 0 { a1.clone() } else { name_of(&t.abilities, b[8] as usize) };
    [a1, a2]
}

fn as_str(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "expected text".into())
}

fn num_at(view: &Value, ptr: &str) -> u32 {
    view.pointer(ptr).and_then(Value::as_u64).unwrap_or(0) as u32
}

impl Gen3Backend {
    fn mon_ptr(&self, mon: usize, key: &str) -> String {
        format!("{}/{}", self.mons[mon].ptr, key)
    }

    fn species_in_view(&self, mon: usize, view: &Value) -> usize {
        let t = tables();
        view.pointer(&self.mon_ptr(mon, "Species")).and_then(Value::as_str).and_then(|s| index_of(&t.species, s)).unwrap_or(0)
    }
}

impl Backend for Gen3Backend {
    fn write(&self, view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let l = &self.layout;
        let t = tables();
        let mut out = self.raw.clone();
        if changes.is_empty() {
            return Ok(out);
        }
        let mut mons: HashMap<usize, MonData> = HashMap::new();
        for (leaf, v) in changes {
            match &self.fields[*leaf] {
                Field::Int { addr, size, max, xor } => {
                    let mut n = int_in(v, 0, *max as i128, "This value")? as u32;
                    if *xor {
                        n ^= if *size == 2 { l.key & 0xFFFF } else { l.key };
                    }
                    l.write(&mut out, *addr, &n.to_le_bytes()[..*size]);
                }
                Field::Text { addr, len } => {
                    let orig = l.read(&out, *addr, *len);
                    l.write(&mut out, *addr, &encode_text(as_str(v)?, *len, &orig)?);
                }
                Field::Gender { addr } => l.write(&mut out, *addr, &[u8::from(as_str(v)? == "Girl")]),
                Field::Item { addr } => {
                    let id = index_of(&t.items, as_str(v)?).ok_or("unknown item")? as u16;
                    l.write(&mut out, *addr, &id.to_le_bytes());
                }
                Field::Mon { mon, part } => {
                    let m = &self.mons[*mon];
                    let md = mons.entry(*mon).or_insert_with(|| MonData::read(l, &self.raw, m.addr, m.party));
                    let int = |max: i128| int_in(v, 0, max, "This value").map(|n| n as u32);
                    match *part {
                        MonPart::Species => {
                            let sp = index_of(&t.species, as_str(v)?).ok_or("unknown Pokémon")?;
                            md.set(G, 0, 2, sp as u32);
                            // The ability shown is re-read under the new species, so store its slot.
                            let shown = view.pointer(&self.mon_ptr(*mon, "Ability")).and_then(Value::as_str).unwrap_or("");
                            let second = abilities_of(sp)[1] == shown && abilities_of(sp)[0] != shown;
                            let w = md.get(M, 4, 4) & 0x7FFF_FFFF;
                            md.set(M, 4, 4, w | (u32::from(second) << 31));
                        }
                        MonPart::Nickname => {
                            let enc = encode_text(as_str(v)?, 10, &md.head[8..18])?;
                            md.head[8..18].copy_from_slice(&enc);
                        }
                        MonPart::HeldItem => md.set(G, 2, 2, index_of(&t.items, as_str(v)?).ok_or("unknown item")? as u32),
                        MonPart::Experience => md.set(G, 4, 4, int(u32::MAX as i128)?),
                        MonPart::Friendship => md.set(G, 9, 1, int(255)?),
                        MonPart::Move(i) => md.set(A, i * 2, 2, index_of(&t.moves, as_str(v)?).ok_or("unknown move")? as u32),
                        MonPart::Pp(i) => md.set(A, 8 + i, 1, int(255)?),
                        MonPart::Ev(i) => md.set(E, i, 1, int(255)?),
                        MonPart::Iv(i) => {
                            let w = md.get(M, 4, 4) & !(31 << (5 * i));
                            md.set(M, 4, 4, w | (int(31)? << (5 * i)));
                        }
                        MonPart::Ability => {
                            let opts = abilities_of(self.species_in_view(*mon, view));
                            let second = opts[1] != opts[0] && as_str(v)? == opts[1];
                            let w = md.get(M, 4, 4) & 0x7FFF_FFFF;
                            md.set(M, 4, 4, w | (u32::from(second) << 31));
                        }
                        MonPart::Ball => {
                            let want = as_str(v)?;
                            let ball = BALLS.iter().position(|b| !b.is_empty() && *b == want).ok_or("unknown ball")? as u32;
                            let w = md.get(M, 2, 2) & !(0xF << 11);
                            md.set(M, 2, 2, w | (ball << 11));
                        }
                        // Box Pokémon have no level byte: their level comes from experience.
                        MonPart::Level if m.party => md.head[84] = int(100)? as u8,
                        MonPart::Level => {}
                        MonPart::CurrentHp => md.head[86..88].copy_from_slice(&(int(0xFFFF)? as u16).to_le_bytes()),
                        MonPart::Stat(i) => md.head[88 + i * 2..90 + i * 2].copy_from_slice(&(int(0xFFFF)? as u16).to_le_bytes()),
                    }
                }
            }
        }
        for (mon, mut md) in mons {
            md.seal();
            l.write(&mut out, self.mons[mon].addr, &md.head);
        }
        // Fresh checksums for the edited copy of the game.
        for (id, &o) in l.sec.iter().enumerate() {
            let sum = checksum(&out[o..], SIZES[id]);
            out[o + 0xFF6..o + 0xFF8].copy_from_slice(&sum.to_le_bytes());
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let t = tables();
        let known = |table: &[String], what: &str| -> Result<(), String> {
            let s = as_str(v)?;
            index_of(table, s).map(|_| ()).ok_or_else(|| format!("there's no {what} called \u{201C}{s}\u{201D}"))
        };
        match &self.fields[leaf] {
            Field::Int { max, .. } => int_in(v, 0, *max as i128, "This value").map(|_| ()),
            Field::Text { len, .. } => encode_text(as_str(v)?, *len, &[]).map(|_| ()),
            Field::Gender { .. } => matches!(as_str(v)?, "Boy" | "Girl").then_some(()).ok_or_else(|| "choose Boy or Girl".into()),
            Field::Item { .. } => known(&t.items, "item"),
            Field::Mon { part, .. } => match part {
                MonPart::Species => match index_of(&t.species, as_str(v)?) {
                    Some(0) | None => Err(format!("there's no Pokémon called \u{201C}{}\u{201D}", as_str(v)?)),
                    Some(_) => Ok(()),
                },
                MonPart::Nickname => encode_text(as_str(v)?, 10, &[]).map(|_| ()),
                MonPart::HeldItem => known(&t.items, "item"),
                MonPart::Move(_) => known(&t.moves, "move"),
                MonPart::Ability | MonPart::Ball => Ok(()),
                MonPart::Iv(_) => int_in(v, 0, 31, "IVs").map(|_| ()),
                MonPart::Level => int_in(v, 1, 100, "Level").map(|_| ()),
                MonPart::Experience => int_in(v, 0, 2_000_000, "Experience").map(|_| ()),
                MonPart::Friendship | MonPart::Pp(_) | MonPart::Ev(_) => int_in(v, 0, 255, "This value").map(|_| ()),
                MonPart::CurrentHp | MonPart::Stat(_) => int_in(v, 0, 999, "Stats").map(|_| ()),
            },
        }
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let Field::Mon { mon, part } = &self.fields[leaf] else { return vec![] };
        let (mon, part) = (*mon, *part);
        let p = |k: &str| self.mon_ptr(mon, k);
        let species = self.species_in_view(mon, view);
        let Some(b) = tables().base.get(species).copied().flatten() else { return vec![] };
        let growth = b[6];
        let mut out = Vec::new();
        let mut level = num_at(view, &p("Level"));
        match part {
            MonPart::Level => out.push((p("Experience"), Value::from(exp_for(growth, level)))),
            MonPart::Experience | MonPart::Species => {
                level = level_for(growth, num_at(view, &p("Experience")));
                out.push((p("Level"), Value::from(level)));
            }
            MonPart::Iv(_) | MonPart::Ev(_) => {}
            _ => return vec![],
        }
        if part == MonPart::Species {
            // Keep the ability if the new species can have it, otherwise its first one.
            let cur = view.pointer(&p("Ability")).and_then(Value::as_str).unwrap_or("");
            let opts = abilities_of(species);
            out.push((p("Ability"), Value::from(if opts.iter().any(|a| a == cur) { cur.to_string() } else { opts[0].clone() })));
        }
        if self.mons[mon].party {
            let ivs = std::array::from_fn(|i| num_at(view, &p(&format!("IVs/{}", STATS[i]))));
            let evs = std::array::from_fn(|i| num_at(view, &p(&format!("EVs/{}", STATS[i]))));
            let nature = view.pointer(&p("Nature")).and_then(Value::as_str).and_then(|n| NATURES.iter().position(|x| *x == n)).unwrap_or(0);
            if let Some(stats) = calc_stats(species, ivs, evs, level, nature) {
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
        let t = tables();
        match &self.fields[leaf] {
            Field::Gender { .. } => Some(Arc::new(vec!["Boy".into(), "Girl".into()])),
            Field::Item { .. } => Some(t.items_list.clone()),
            Field::Mon { mon, part } => match part {
                MonPart::Species => Some(t.species_list.clone()),
                MonPart::HeldItem => Some(t.items_list.clone()),
                MonPart::Move(_) => Some(t.moves_list.clone()),
                MonPart::Ball => Some(Arc::new(BALLS[1..].iter().map(|s| s.to_string()).collect())),
                MonPart::Ability => {
                    let [a, b] = abilities_of(self.species_in_view(*mon, view));
                    Some(Arc::new(if a == b { vec![a] } else { vec![a, b] }))
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

// ------------------------------------------------------------------ view

#[derive(Default)]
struct Builder {
    fields: Vec<Field>,
    mons: Vec<Mon>,
    editable: HashMap<String, usize>,
}

impl Builder {
    fn add(&mut self, map: &mut Map<String, Value>, base: &str, key: &str, v: Value, f: Option<Field>) {
        if let Some(f) = f {
            self.editable.insert(format!("{base}/{}", crate::tree::esc(key)), self.fields.len());
            self.fields.push(f);
        }
        map.insert(key.to_string(), v);
    }
}

fn mon_view(b: &mut Builder, l: &Layout, d: &[u8], addr: usize, party: bool, ptr: &str) -> Option<Value> {
    let t = tables();
    let md = MonData::read(l, d, addr, party);
    let species = md.species();
    if species == 0 || md.pid() == 0 && rd32(&md.head, 4) == 0 {
        return None;
    }
    let ok = md.checksum_ok();
    let mon = b.mons.len();
    let mf = |part| ok.then_some(Field::Mon { mon, part });
    b.mons.push(Mon { addr, party, ptr: ptr.to_string() });
    let base = t.base.get(species).copied().flatten();
    let egg = md.get(M, 4, 4) >> 30 & 1 == 1;
    let name = name_of(&t.species, species);
    let mut m = Map::new();
    if !ok {
        m.insert("Note".into(), Value::from("This Pokémon's data is damaged (a \u{201C}Bad Egg\u{201D}); it's shown read-only."));
    }
    b.add(&mut m, ptr, "Species", Value::from(name.clone()), mf(MonPart::Species));
    let nick = decode_text(&md.head[8..18]);
    let nick_field = if nick.is_some() && !egg { mf(MonPart::Nickname) } else { None };
    b.add(&mut m, ptr, "Nickname", Value::from(nick.clone().unwrap_or_else(|| "?".into())), nick_field);
    let exp = md.get(G, 4, 4);
    let level = if party { md.head[84] as u32 } else { base.map_or(1, |b| level_for(b[6], exp)) };
    b.add(&mut m, ptr, "Level", Value::from(level), mf(MonPart::Level));
    b.add(&mut m, ptr, "Experience", Value::from(exp), mf(MonPart::Experience));
    let nature = (md.pid() % 25) as usize;
    m.insert("Nature".into(), Value::from(NATURES[nature]));
    let tid = rd32(&md.head, 4);
    let shiny = ((tid >> 16) ^ (tid & 0xFFFF) ^ (md.pid() >> 16) ^ (md.pid() & 0xFFFF)) < 8;
    m.insert("Shiny".into(), Value::Bool(shiny));
    if egg {
        m.insert("Egg".into(), Value::Bool(true));
    }
    let abil = abilities_of(species)[(md.get(M, 4, 4) >> 31) as usize].clone();
    b.add(&mut m, ptr, "Ability", Value::from(abil), mf(MonPart::Ability));
    b.add(&mut m, ptr, "Held item", Value::from(name_of(&t.items, md.get(G, 2, 2) as usize)), mf(MonPart::HeldItem));
    b.add(&mut m, ptr, "Friendship", Value::from(md.get(G, 9, 1)), mf(MonPart::Friendship));
    let ball = (md.get(M, 2, 2) >> 11 & 0xF) as usize;
    if (1..BALLS.len()).contains(&ball) {
        b.add(&mut m, ptr, "Poké Ball", Value::from(BALLS[ball]), mf(MonPart::Ball));
    }
    let mut moves = Map::new();
    let mptr = format!("{ptr}/Moves");
    for i in 0..4 {
        b.add(&mut moves, &mptr, &format!("Move {}", i + 1), Value::from(name_of(&t.moves, md.get(A, i * 2, 2) as usize)), mf(MonPart::Move(i)));
        b.add(&mut moves, &mptr, &format!("PP {}", i + 1), Value::from(md.get(A, 8 + i, 1)), mf(MonPart::Pp(i)));
    }
    m.insert("Moves".into(), Value::Object(moves));
    for (title, vals, part) in [("IVs", md.ivs(), MonPart::Iv as fn(usize) -> MonPart), ("EVs", md.evs(), MonPart::Ev as fn(usize) -> MonPart)] {
        let mut s = Map::new();
        let sp = format!("{ptr}/{title}");
        for i in 0..6 {
            b.add(&mut s, &sp, STATS[i], Value::from(vals[i]), mf(part(i)));
        }
        m.insert(title.into(), Value::Object(s));
    }
    if party {
        let mut s = Map::new();
        let sp = format!("{ptr}/Stats");
        let h = |o: usize| u16::from_le_bytes([md.head[o], md.head[o + 1]]) as u32;
        b.add(&mut s, &sp, "Current HP", Value::from(h(86)), mf(MonPart::CurrentHp));
        b.add(&mut s, &sp, "Max HP", Value::from(h(88)), mf(MonPart::Stat(0)));
        for i in 1..6 {
            b.add(&mut s, &sp, STATS[i], Value::from(h(88 + i * 2)), mf(MonPart::Stat(i)));
        }
        m.insert("Stats".into(), Value::Object(s));
    }
    Some(Value::Object(m))
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Generation III Pokémon save".into());
    }
    let l = layout(d).ok_or("unrecognised Pokémon save")?;
    let t = tables();
    let mut b = Builder::default();
    let mut root = Map::new();
    let s1 = SECTION;
    let frlg = l.game == Game::FireRedLeafGreen;

    // Trainer
    let mut tr = Map::new();
    let tp = "/Trainer";
    match decode_text(&l.read(d, 0, 7)) {
        Some(n) => b.add(&mut tr, tp, "Name", Value::from(n), Some(Field::Text { addr: 0, len: 7 })),
        None => b.add(&mut tr, tp, "Name", Value::from("?"), None),
    }
    b.add(&mut tr, tp, "Gender", Value::from(if l.u(d, 8, 1) == 1 { "Girl" } else { "Boy" }), Some(Field::Gender { addr: 8 }));
    tr.insert("Trainer ID".into(), Value::from(l.u(d, 10, 2)));
    tr.insert("Secret ID".into(), Value::from(l.u(d, 12, 2)));
    let money_at = s1 + if frlg { 0x290 } else { 0x490 };
    b.add(&mut tr, tp, "Money", Value::from(l.u(d, money_at, 4) ^ l.key), Some(Field::Int { addr: money_at, size: 4, max: 999_999, xor: true }));
    b.add(&mut tr, tp, "Coins", Value::from((l.u(d, money_at + 4, 2) ^ (l.key & 0xFFFF)) & 0xFFFF), Some(Field::Int { addr: money_at + 4, size: 2, max: 9999, xor: true }));
    b.add(&mut tr, tp, "Play time (hours)", Value::from(l.u(d, 0xE, 2)), Some(Field::Int { addr: 0xE, size: 2, max: 999, xor: false }));
    b.add(&mut tr, tp, "Play time (minutes)", Value::from(l.u(d, 0x10, 1)), Some(Field::Int { addr: 0x10, size: 1, max: 59, xor: false }));
    b.add(&mut tr, tp, "Play time (seconds)", Value::from(l.u(d, 0x11, 1)), Some(Field::Int { addr: 0x11, size: 1, max: 59, xor: false }));
    if frlg {
        let rival = 4 * SECTION + 0xBCC;
        if let Some(n) = decode_text(&l.read(d, rival, 7)) {
            b.add(&mut tr, tp, "Rival name", Value::from(n), Some(Field::Text { addr: rival, len: 7 }));
        }
    }
    root.insert("Trainer".into(), Value::Object(tr));

    // Party
    let (team_size, team) = if frlg { (s1 + 0x34, s1 + 0x38) } else { (s1 + 0x234, s1 + 0x238) };
    let n = l.u(d, team_size, 4).min(6) as usize;
    let mut party = Map::new();
    for i in 0..n {
        // Keys stay fixed when a Pokémon is renamed (the label shows its name).
        let key = format!("Pokémon {}", i + 1);
        let ptr = format!("/Party/{}", crate::tree::esc(&key));
        if let Some(v) = mon_view(&mut b, &l, d, team + i * 100, true, &ptr) {
            party.insert(key, v);
        }
    }
    root.insert("Party".into(), Value::Object(party));

    // Bag
    let pockets: &[(&str, usize, usize, bool)] = match l.game {
        Game::RubySapphire => &[("Items", 0x560, 20, true), ("Key items", 0x5B0, 20, true), ("Poké Balls", 0x600, 16, true), ("TMs & HMs", 0x640, 64, true), ("Berries", 0x740, 46, true), ("PC items", 0x498, 50, false)],
        Game::Emerald => &[("Items", 0x560, 30, true), ("Key items", 0x5D8, 30, true), ("Poké Balls", 0x650, 16, true), ("TMs & HMs", 0x690, 64, true), ("Berries", 0x790, 46, true), ("PC items", 0x498, 50, false)],
        Game::FireRedLeafGreen => &[("Items", 0x310, 42, true), ("Key items", 0x3B8, 30, true), ("Poké Balls", 0x430, 13, true), ("TMs & HMs", 0x464, 58, true), ("Berries", 0x54C, 43, true), ("PC items", 0x298, 30, false)],
    };
    let mut bag = Map::new();
    for &(title, off, count, xor) in pockets {
        let mut pocket = Map::new();
        let pp = format!("/Bag/{}", crate::tree::esc(title));
        // Every slot is listed, so filling an empty one doesn't change the layout.
        for i in 0..count {
            let addr = s1 + off + i * 4;
            let id = l.u(d, addr, 2) as usize;
            let qty = l.u(d, addr + 2, 2) ^ if xor { l.key & 0xFFFF } else { 0 };
            let mut slot = Map::new();
            let key = format!("Slot {}", i + 1);
            let sp = format!("{pp}/{}", crate::tree::esc(&key));
            b.add(&mut slot, &sp, "Item", Value::from(name_of(&t.items, id)), Some(Field::Item { addr }));
            b.add(&mut slot, &sp, "Quantity", Value::from(qty & 0xFFFF), Some(Field::Int { addr: addr + 2, size: 2, max: 999, xor }));
            pocket.insert(key, Value::Object(slot));
        }
        bag.insert(title.into(), Value::Object(pocket));
    }
    root.insert("Bag".into(), Value::Object(bag));

    // PC boxes
    let mut boxes = Map::new();
    for bx in 0..14 {
        let name_addr = PC + 0x8344 + bx * 9;
        let bname = decode_text(&l.read(d, name_addr, 9)).unwrap_or_default();
        let key = format!("Box {}", bx + 1);
        let bp = format!("/PC boxes/{}", crate::tree::esc(&key));
        let mut bm = Map::new();
        if !bname.is_empty() {
            b.add(&mut bm, &bp, "Box name", Value::from(bname), Some(Field::Text { addr: name_addr, len: 8 }));
        }
        for slot in 0..30 {
            let addr = PC + 4 + (bx * 30 + slot) * 80;
            if l.u(d, addr, 4) == 0 && l.u(d, addr + 4, 4) == 0 {
                continue;
            }
            let k = format!("Slot {}", slot + 1);
            let ptr = format!("{bp}/{}", crate::tree::esc(&k));
            if let Some(v) = mon_view(&mut b, &l, d, addr, false, &ptr) {
                bm.insert(k, v);
            }
        }
        boxes.insert(key, Value::Object(bm));
    }
    root.insert("PC boxes".into(), Value::Object(boxes));

    let backend = Gen3Backend { raw: d.to_vec(), layout: l.clone(), fields: b.fields, mons: b.mons };
    let doc = TreeDoc::new(Value::Object(root), b.editable, l.game.name(), Box::new(backend));
    Ok(doc.with_note(
        "Edits go into the newest of the two copies the game keeps, and checksums are fixed automatically. \
         Changing a Pokémon's level also sets its experience and stats. Close the game in your emulator before saving, \
         and don't load a save state afterwards (that would overwrite the edit).",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_line_up() {
        let t = tables();
        assert_eq!(t.species[277], "Treecko");
        assert_eq!(t.species[411], "Chimecho");
        assert_eq!(t.items[13], "Potion");
        assert_eq!(t.items[289], "TM01");
        assert_eq!(t.moves[354], "Psycho Boost");
        assert_eq!(t.abilities[26], "Levitate");
        assert_eq!(t.base[1].unwrap()[..6], [45, 49, 49, 45, 65, 65]);
        assert_eq!(exp_for(0, 100), 1_000_000);
        assert_eq!(exp_for(1, 100), 600_000);
        assert_eq!(exp_for(2, 100), 1_640_000);
        assert_eq!(exp_for(3, 100), 1_059_860);
        assert_eq!(level_for(3, 1_059_860), 100);
        assert_eq!(level_for(0, 999_999), 99);
    }

    #[test]
    fn text_round_trip() {
        let enc = encode_text("Ash", 7, &[0xFF; 7]).unwrap();
        assert_eq!(enc, [0xBB, 0xE7, 0xDC, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(decode_text(&enc).unwrap(), "Ash");
        assert!(encode_text("Ashketchum", 7, &[]).is_err());
    }

    /// Build a minimal valid Emerald save with one party Pokémon.
    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; 0x20000];
        let key = 0x1234_5678u32;
        for (blk, idx) in [(0usize, 4u32), (BLOCK, 5)] {
            for i in 0..14 {
                let o = blk + i * SECTION;
                let id = (i + 3) % 14;
                let s = &mut d[o..o + SECTION];
                if id == 0 {
                    s[..7].copy_from_slice(&[0xBB, 0xE7, 0xDC, 0xFF, 0, 0, 0]);
                    s[0xAC..0xB0].copy_from_slice(&key.to_le_bytes());
                }
                if id == 1 {
                    s[0x234] = 1;
                    s[0x490..0x494].copy_from_slice(&(3000 ^ key).to_le_bytes());
                    s[0x560..0x562].copy_from_slice(&13u16.to_le_bytes());
                    s[0x562..0x564].copy_from_slice(&((5 ^ key) as u16).to_le_bytes());
                    // Mudkip, level 5, personality 7 (nature Bashful? 7 % 25 = Relaxed).
                    let mut head = vec![0u8; 100];
                    head[0..4].copy_from_slice(&7u32.to_le_bytes());
                    head[4..8].copy_from_slice(&0x0001_0002u32.to_le_bytes());
                    head[8..18].copy_from_slice(&[0xC7, 0xE9, 0xD8, 0xDF, 0xDD, 0xE4, 0xFF, 0xFF, 0xFF, 0xFF]);
                    head[84] = 5;
                    let mut md = MonData { head, data: [0; 48] };
                    md.set(G, 0, 2, 283);
                    md.set(G, 4, 4, exp_for(3, 5));
                    md.set(A, 0, 2, 33);
                    md.set(M, 4, 4, 31 | 31 << 5);
                    md.seal();
                    s[0x238..0x238 + 100].copy_from_slice(&md.head);
                }
                s[0xFF4..0xFF6].copy_from_slice(&(id as u16).to_le_bytes());
                s[0xFF8..0xFFC].copy_from_slice(&SIGNATURE.to_le_bytes());
                s[0xFFC..0x1000].copy_from_slice(&idx.to_le_bytes());
                let sum = checksum(s, SIZES[id]);
                s[0xFF6..0xFF8].copy_from_slice(&sum.to_le_bytes());
            }
        }
        d
    }

    #[test]
    fn edit_emerald_save() {
        let d = synthetic();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.kind_name, "Pokémon Emerald");
        assert_eq!(doc.view.pointer("/Trainer/Money").unwrap(), 3000);
        assert_eq!(doc.view.pointer("/Bag/Items/Slot 1/Quantity").unwrap(), 5);
        let p = "/Party/Pokémon 1";
        assert_eq!(doc.view.pointer(&format!("{p}/Level")).unwrap(), 5);
        doc.set("/Trainer/Money", Value::from(999_999)).unwrap();
        doc.set("/Bag/Items/Slot 1/Item", Value::from("Rare Candy")).unwrap();
        doc.set("/Bag/Items/Slot 1/Quantity", Value::from(99)).unwrap();
        doc.set(&format!("{p}/Level"), Value::from(50)).unwrap();
        let derived = doc.take_derived();
        assert!(derived.iter().any(|(k, _, _)| k.ends_with("/Experience")));
        doc.set(&format!("{p}/Moves/Move 2"), Value::from("Surf")).unwrap();
        assert!(doc.set(&format!("{p}/Moves/Move 3"), Value::from("Not a move")).is_err());
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view, doc.view);
        assert_eq!(again.view.pointer("/Trainer/Money").unwrap(), 999_999);
        assert_eq!(again.view.pointer(&format!("{p}/Experience")).unwrap(), exp_for(3, 50));
        assert_eq!(again.view.pointer(&format!("{p}/Moves/Move 2")).unwrap(), "Surf");
        // Every section in the edited copy still has a valid checksum.
        assert!(block(&out, BLOCK).is_some());
        // The other (older) copy is untouched.
        assert_eq!(&out[..BLOCK], &d[..BLOCK]);
    }

    /// Party stats calculated from base stats match what the game stored (sample saves).
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("GEN3_SAVES").expect("GEN3_SAVES");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let doc = parse(&d).unwrap();
            println!("{} -> {} {}", e.path().display(), doc.kind_name, doc.view["Trainer"]);
            let l = layout(&d).unwrap();
            let s1 = SECTION;
            let team = s1 + if l.game == Game::FireRedLeafGreen { 0x38 } else { 0x238 };
            for i in 0..6 {
                let md = MonData::read(&l, &d, team + i * 100, true);
                if md.species() == 0 || !md.checksum_ok() {
                    continue;
                }
                let lv = md.head[84] as u32;
                let stored: Vec<u32> = (0..6).map(|k| u16::from_le_bytes([md.head[88 + k * 2], md.head[89 + k * 2]]) as u32).collect();
                let calc = calc_stats(md.species(), md.ivs(), md.evs(), lv, (md.pid() % 25) as usize).unwrap();
                println!("  {} lv{lv} stored {stored:?} calc {calc:?}", tables().species[md.species()]);
                // The game only recalculates stats on level-up, so EVs earned since then
                // can leave a point or two of difference.
                if !e.path().to_string_lossy().contains("Random") {
                    assert!(stored.iter().zip(calc).all(|(a, b)| a.abs_diff(b) <= 3), "stats don't match");
                }
            }
            let out = doc.to_bytes().unwrap();
            assert_eq!(out, d, "unchanged save must be written back identically");
        }
    }
}
