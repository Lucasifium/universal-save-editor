//! Terraria worlds (`.wld`), Terraria 1.4+: the world's settings, bosses and events, the
//! chests and signs, the town NPCs, and the map itself (blocks, walls and liquids), which
//! the World screen can paint.
//!
//! A world file is a short header (version, "relogic", a table of section offsets and a
//! bit per tile type saying whether it stores a frame), then sections: world info, tiles,
//! chests, signs, NPCs, tile entities, pressure plates, town rooms, bestiary, Journey
//! powers and a footer (name and id again). World info fields are edited in place; the
//! tiles, chest, sign and footer sections are written anew when they change, and the
//! offset table is fixed up. Untouched sections are copied byte for byte.
//!
//! Map edits are kept as one text value ("/Map edits": brush strokes), so each stroke is
//! one undo step; they're applied to the tile columns they touch when saving.

use super::terraria::{item_id, item_name, prefix_id, prefix_name, tables, EMPTY, NO_PREFIX};
use crate::tree::{esc, int_in, Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

const MIN_VERSION: i32 = 225; // Terraria 1.4.0
pub const EDITS: &str = "/Map edits";
const MODES: [&str; 4] = ["Classic", "Expert", "Master", "Journey"];
const EVIL: [&str; 2] = ["Corruption", "Crimson"];
const MAX_NAME: usize = 27;

// ------------------------------------------------------------------ tile and wall table

pub struct Kind {
    pub name: String,
    pub color: [u8; 3],
}

pub struct Palette {
    pub tiles: Vec<Option<Kind>>,
    pub walls: Vec<Option<Kind>>,
}

pub fn palette() -> &'static Palette {
    static P: OnceLock<Palette> = OnceLock::new();
    P.get_or_init(|| {
        let (mut tiles, mut walls): (Vec<Option<Kind>>, Vec<Option<Kind>>) = (vec![], vec![]);
        for l in include_str!("data/terraria_tiles.txt").lines().filter(|l| !l.starts_with('#')) {
            let mut f = l.splitn(3, '\t');
            let (Some(id), Some(col), Some(name)) = (f.next(), f.next(), f.next()) else { continue };
            let (list, n) = match id.split_at(1) {
                ("T", n) => (&mut tiles, n),
                ("W", n) => (&mut walls, n),
                _ => continue,
            };
            let (Ok(n), Ok(c)) = (n.parse::<usize>(), u32::from_str_radix(col, 16)) else { continue };
            if list.len() <= n {
                list.resize_with(n + 1, || None);
            }
            list[n] = Some(Kind { name: name.to_string(), color: [(c >> 16) as u8, (c >> 8) as u8, c as u8] });
        }
        Palette { tiles, walls }
    })
}

pub fn tile_name(id: u16) -> String {
    palette().tiles.get(id as usize).and_then(Option::as_ref).map_or_else(|| format!("Tile #{id}"), |k| k.name.clone())
}

pub fn wall_name(id: u16) -> String {
    palette().walls.get(id as usize).and_then(Option::as_ref).map_or_else(|| format!("Wall #{id}"), |k| k.name.clone())
}

/// Town NPC roles by NPC id (the name in the save is the NPC's own name).
fn npc_role(id: i32) -> String {
    let r = match id {
        17 => "Merchant",
        18 => "Nurse",
        19 => "Arms Dealer",
        20 => "Dryad",
        22 => "Guide",
        37 => "Old Man",
        38 => "Demolitionist",
        54 => "Clothier",
        107 => "Goblin Tinkerer",
        108 => "Wizard",
        124 => "Mechanic",
        142 => "Santa Claus",
        160 => "Truffle",
        178 => "Steampunker",
        207 => "Dye Trader",
        208 => "Party Girl",
        209 => "Cyborg",
        227 => "Painter",
        228 => "Witch Doctor",
        229 => "Pirate",
        353 => "Stylist",
        368 => "Traveling Merchant",
        369 => "Angler",
        441 => "Tax Collector",
        453 => "Skeleton Merchant",
        550 => "Tavernkeep",
        588 => "Golfer",
        633 => "Zoologist",
        637 => "Town Cat",
        638 => "Town Dog",
        656 => "Town Bunny",
        663 => "Princess",
        670 | 678..=684 => "Town Slime",
        _ => return format!("NPC #{id}"),
    };
    r.to_string()
}

// ------------------------------------------------------------------ reader

struct Reader<'a> {
    d: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<usize, String> {
        let at = self.pos;
        if at + n > self.d.len() {
            return Err("unexpected end of world file".into());
        }
        self.pos += n;
        Ok(at)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.d[self.take(1)?])
    }
    fn i16(&mut self) -> Result<i16, String> {
        let o = self.take(2)?;
        Ok(i16::from_le_bytes([self.d[o], self.d[o + 1]]))
    }
    fn i32(&mut self) -> Result<i32, String> {
        let o = self.take(4)?;
        Ok(i32::from_le_bytes(self.d[o..o + 4].try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(self.i32()? as u32))
    }
    fn f64(&mut self) -> Result<f64, String> {
        let o = self.take(8)?;
        Ok(f64::from_le_bytes(self.d[o..o + 8].try_into().unwrap()))
    }
    /// .NET string: 7-bit encoded length then UTF-8. Returns (start, end, text).
    fn string(&mut self) -> Result<(usize, usize, String), String> {
        let start = self.pos;
        let (mut len, mut shift) = (0usize, 0);
        loop {
            let b = self.u8()?;
            len |= ((b & 0x7F) as usize) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 28 {
                return Err("bad string".into());
            }
        }
        let o = self.take(len)?;
        let s = String::from_utf8_lossy(&self.d[o..o + len]).into_owned();
        Ok((start, self.pos, s))
    }
    fn text(&mut self) -> Result<String, String> {
        Ok(self.string()?.2)
    }
}

fn net_string(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut n = s.len();
    while n >= 0x80 {
        out.push((n as u8 & 0x7F) | 0x80);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(s.as_bytes());
    out
}

// ------------------------------------------------------------------ tiles

/// One tile as stored. `h2`/`h3`/`h4` keep the header bits not described by the other
/// fields (wires, slope, actuator, actuated, shimmer, invisible/full-bright paint…).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Cell {
    pub tile: Option<u16>,
    pub frame: (i16, i16),
    pub tcolor: u8,
    pub wall: u16,
    pub wcolor: u8,
    /// 1 water, 2 lava, 3 honey (shimmer is water with h3 bit 0x80).
    pub liquid: u8,
    pub amount: u8,
    pub h2: u8,
    pub h3: u8,
    pub h4: u8,
}

fn read_cell(r: &mut Reader, imp: &[bool]) -> Result<(Cell, usize), String> {
    let h1 = r.u8()?;
    let (mut h2, mut h3, mut h4) = (0, 0, 0);
    if h1 & 1 != 0 {
        h2 = r.u8()?;
        if h2 & 1 != 0 {
            h3 = r.u8()?;
            if h3 & 1 != 0 {
                h4 = r.u8()?;
            }
        }
    }
    let mut c = Cell::default();
    if h1 & 2 != 0 {
        let mut t = r.u8()? as u16;
        if h1 & 0x20 != 0 {
            t |= (r.u8()? as u16) << 8;
        }
        if t as usize >= imp.len() {
            return Err("unexpected tile data".into());
        }
        c.tile = Some(t);
        if imp[t as usize] {
            c.frame = (r.i16()?, r.i16()?);
        }
        if h3 & 8 != 0 {
            c.tcolor = r.u8()?;
        }
    }
    if h1 & 4 != 0 {
        c.wall = r.u8()? as u16;
        if h3 & 0x10 != 0 {
            c.wcolor = r.u8()?;
        }
    }
    c.liquid = (h1 >> 3) & 3;
    if c.liquid != 0 {
        c.amount = r.u8()?;
    }
    if h3 & 0x40 != 0 {
        c.wall |= (r.u8()? as u16) << 8;
    }
    let run = if h1 & 0x40 != 0 {
        r.u8()? as usize
    } else if h1 & 0x80 != 0 {
        r.i16()? as u16 as usize
    } else {
        0
    };
    c.h2 = h2 & !1;
    c.h3 = h3 & !(1 | 8 | 0x10 | 0x40);
    c.h4 = h4 & !1;
    Ok((c, run))
}

fn write_cell(c: &Cell, run: usize, imp: &[bool], out: &mut Vec<u8>) {
    let (mut h1, mut h2, mut h3, h4) = (0u8, c.h2, c.h3, c.h4);
    let mut body = Vec::with_capacity(8);
    if let Some(t) = c.tile {
        h1 |= 2;
        body.push(t as u8);
        if t > 255 {
            h1 |= 0x20;
            body.push((t >> 8) as u8);
        }
        if imp.get(t as usize).copied().unwrap_or(false) {
            body.extend_from_slice(&c.frame.0.to_le_bytes());
            body.extend_from_slice(&c.frame.1.to_le_bytes());
        }
        if c.tcolor != 0 {
            h3 |= 8;
            body.push(c.tcolor);
        }
    }
    if c.wall != 0 {
        h1 |= 4;
        body.push(c.wall as u8);
        if c.wcolor != 0 {
            h3 |= 0x10;
            body.push(c.wcolor);
        }
    }
    if c.liquid != 0 {
        h1 |= c.liquid << 3;
        body.push(c.amount);
    }
    if c.wall > 255 {
        h3 |= 0x40;
        body.push((c.wall >> 8) as u8);
    }
    if run > 255 {
        h1 |= 0x80;
        body.extend_from_slice(&(run as i16).to_le_bytes());
    } else if run > 0 {
        h1 |= 0x40;
        body.push(run as u8);
    }
    if h4 != 0 {
        h3 |= 1;
    }
    if h3 != 0 {
        h2 |= 1;
    }
    if h2 != 0 {
        h1 |= 1;
    }
    out.push(h1);
    if h1 & 1 != 0 {
        out.push(h2);
        if h2 & 1 != 0 {
            out.push(h3);
            if h3 & 1 != 0 {
                out.push(h4);
            }
        }
    }
    out.extend_from_slice(&body);
}

fn read_column(d: &[u8], h: usize, imp: &[bool]) -> Result<Vec<Cell>, String> {
    let mut r = Reader { d, pos: 0 };
    let mut col = Vec::with_capacity(h);
    while col.len() < h {
        let (c, run) = read_cell(&mut r, imp)?;
        for _ in 0..=run {
            col.push(c);
        }
    }
    if col.len() != h {
        return Err("tile column overruns the world".into());
    }
    Ok(col)
}

fn write_column(col: &[Cell], imp: &[bool], out: &mut Vec<u8>) {
    let mut y = 0;
    while y < col.len() {
        let mut run = 0;
        while y + run + 1 < col.len() && col[y + run + 1] == col[y] && run < i16::MAX as usize {
            run += 1;
        }
        write_cell(&col[y], run, imp, out);
        y += run + 1;
    }
}

/// A tile packed for drawing: bits 0-11 tile id + 1 (0 = none), 12-21 wall, 22-24 liquid
/// (1 water, 2 lava, 3 honey, 4 shimmer), 25 actuated.
pub fn pack(c: &Cell) -> u32 {
    let t = c.tile.map_or(0, |t| t as u32 + 1).min(0xFFF);
    let w = (c.wall as u32).min(0x3FF);
    let l = if c.liquid != 0 && c.amount > 0 {
        if c.h3 & 0x80 != 0 {
            4
        } else {
            c.liquid as u32
        }
    } else {
        0
    };
    t | w << 12 | l << 22 | u32::from(c.h3 & 4 != 0) << 25
}

pub fn packed_tile(p: u32) -> Option<u16> {
    (p & 0xFFF).checked_sub(1).map(|t| t as u16)
}
pub fn packed_wall(p: u32) -> u16 {
    ((p >> 12) & 0x3FF) as u16
}
pub fn packed_liquid(p: u32) -> u8 {
    ((p >> 22) & 7) as u8
}
pub fn packed_actuated(p: u32) -> bool {
    p >> 25 & 1 != 0
}

/// The decoded map, shared with the World screen.
pub struct WorldMap {
    pub width: usize,
    pub height: usize,
    /// Row-major: `cells[y * width + x]`, packed with `pack`.
    pub cells: Vec<u32>,
    /// Tile types that are multi-tile objects (furniture, chests, trees…); painting leaves them alone.
    pub important: Vec<bool>,
    pub surface: f64,
    pub rock: f64,
    pub version: i32,
    pub chests: Vec<(i32, i32)>,
    /// (role, name, home x, home y, homeless)
    pub npcs: Vec<(String, String, i32, i32, bool)>,
    pub pois: Vec<Poi>,
}

impl WorldMap {
    pub fn shimmer_ok(&self) -> bool {
        self.version >= 269
    }
}

// ------------------------------------------------------------------ points of interest

#[derive(Clone, Copy)]
enum Find {
    Tile(u16),
    Wall(u16),
    Shimmer,
    /// The sword in stone (Large Piles 2, style 17), inside a shrine (a room with the
    /// Flower wall around it) or not.
    Sword { shrine: bool },
    /// 0: a dungeon biome chest, 1: another dungeon chest, 2: a Jungle Temple chest.
    Chest(u8),
}

/// Dungeon background walls (blue, green, pink; brick, slab and tile).
const DUNGEON_WALLS: [u16; 9] = [7, 8, 9, 94, 95, 96, 97, 98, 99];

/// What a chest is, from its tile and style: (Find::Chest group, name).
fn chest_kind(tile: u16, style: u16, wall: u16) -> Option<(u8, &'static str)> {
    Some(match (tile, style) {
        (21, 23) => (0, "Jungle Chest (Piranha Gun)"),
        (21, 24) => (0, "Corruption Chest (Scourge of the Corruptor)"),
        (21, 25) => (0, "Crimson Chest (Vampire Knives)"),
        (21, 26) => (0, "Hallowed Chest (Rainbow Gun)"),
        (21, 27) => (0, "Frozen Chest (Staff of the Frost Hydra)"),
        (467, 13) => (0, "Desert Chest (Desert Tiger Staff)"),
        (21, 16) => (2, "Lihzahrd Chest"),
        (21, 2) => (1, "Locked Gold Chest"),
        (21 | 467, _) if DUNGEON_WALLS.contains(&wall) => (1, "Chest"),
        _ => return None,
    })
}

/// Large Piles 2 (tile 187) style 17, top-left frame: the sword in stone.
const SWORD_TILE: u16 = 187;
const SWORD_FRAME: (i16, i16) = (17 * 54, 0);
/// The background wall of Enchanted Sword shrines.
const SHRINE_WALL: u16 = 68;

/// What the map points out: name, marker colour, shown unless turned off, what to look
/// for, and for areas the fewest matching tiles that count.
pub struct PoiKind {
    pub name: &'static str,
    pub color: [u8; 3],
    pub shown: bool,
    find: Find,
    area: Option<u32>,
}

const fn kind(name: &'static str, color: [u8; 3], shown: bool, find: Find, area: Option<u32>) -> PoiKind {
    PoiKind { name, color, shown, find, area }
}

pub const POI_KINDS: [PoiKind; 20] = [
    kind("Shimmer", [214, 160, 255], true, Find::Shimmer, Some(20)),
    kind("Dungeon Biome Chest", [80, 230, 170], true, Find::Chest(0), None),
    kind("Dungeon Chest", [120, 140, 255], true, Find::Chest(1), None),
    kind("Jungle Temple Chest", [255, 190, 60], true, Find::Chest(2), None),
    kind("Enchanted Sword Shrine", [120, 230, 255], true, Find::Sword { shrine: true }, None),
    kind("Sword in stone (no shrine)", [120, 160, 180], false, Find::Sword { shrine: false }, None),
    kind("Plantera's Bulb", [225, 128, 206], true, Find::Tile(238), None),
    kind("Life Fruit", [149, 232, 87], true, Find::Tile(236), None),
    kind("Life Crystal", [235, 60, 90], false, Find::Tile(12), None),
    kind("Shadow Orb / Crimson Heart", [180, 90, 220], true, Find::Tile(31), None),
    kind("Demon / Crimson Altar", [150, 60, 60], true, Find::Tile(26), None),
    kind("Queen Bee Larva", [240, 200, 90], true, Find::Tile(231), None),
    kind("Lihzahrd Altar", [255, 241, 51], true, Find::Tile(237), None),
    kind("Jungle Temple", [200, 110, 40], true, Find::Wall(87), Some(400)),
    kind("Bee Hive", [255, 156, 12], true, Find::Wall(86), Some(150)),
    kind("Marble Cave", [235, 235, 245], true, Find::Wall(178), Some(200)),
    kind("Granite Cave", [90, 90, 200], true, Find::Wall(180), Some(200)),
    kind("Spider Nest", [200, 200, 200], true, Find::Wall(62), Some(200)),
    kind("Glowing Mushrooms", [93, 127, 255], true, Find::Tile(70), Some(60)),
    kind("Meteorite", [255, 110, 70], true, Find::Tile(37), Some(30)),
];

/// A point of interest: which kind, where (tiles), and for areas how many tiles match.
#[derive(Clone, Copy, Debug)]
pub struct Poi {
    pub kind: usize,
    pub x: u32,
    pub y: u32,
    pub size: u32,
    /// What exactly it is, when the kind covers several things (which biome chest…).
    pub note: &'static str,
}

fn poi_matches(f: Find, p: u32) -> bool {
    match f {
        Find::Tile(t) => packed_tile(p) == Some(t),
        Find::Wall(w) => packed_wall(p) == w,
        Find::Shimmer => packed_liquid(p) == 4,
        Find::Sword { .. } | Find::Chest(_) => false,
    }
}

/// Finds the points of interest: one per object (its top-left tile) and one per patch
/// of an area (16-tile squares holding enough of it, joined with their neighbours).
/// A chest's top-left tile as found in the map: (x, y, tile, style, wall).
pub type ChestTile = (u32, u32, u16, u16, u16);

pub fn points_of_interest(m: &WorldMap, swords: &[(u32, u32)], chests: &[ChestTile]) -> Vec<Poi> {
    const BIN: usize = 16;
    let (w, h) = (m.width, m.height);
    let (bw, bh) = (w.div_ceil(BIN), h.div_ceil(BIN));
    let areas: Vec<usize> = (0..POI_KINDS.len()).filter(|&k| POI_KINDS[k].area.is_some()).collect();
    let objects: Vec<usize> = (0..POI_KINDS.len()).filter(|&k| POI_KINDS[k].area.is_none()).collect();
    let mut counts = vec![vec![0u16; bw * bh]; areas.len()];
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let p = m.cells[y * w + x];
            for (ai, &k) in areas.iter().enumerate() {
                if poi_matches(POI_KINDS[k].find, p) {
                    counts[ai][(y / BIN) * bw + x / BIN] += 1;
                }
            }
            let Some(t) = packed_tile(p) else { continue };
            for &k in &objects {
                if let Find::Tile(want) = POI_KINDS[k].find {
                    let left = x > 0 && packed_tile(m.cells[y * w + x - 1]) == Some(want);
                    let up = y > 0 && packed_tile(m.cells[(y - 1) * w + x]) == Some(want);
                    if t == want && !left && !up {
                        out.push(Poi { kind: k, x: x as u32, y: y as u32, size: 1, note: "" });
                    }
                }
            }
        }
    }
    for &(x, y, tile, style, wall) in chests {
        if let Some((group, note)) = chest_kind(tile, style, wall) {
            if let Some(k) = POI_KINDS.iter().position(|k| matches!(k.find, Find::Chest(g) if g == group)) {
                out.push(Poi { kind: k, x, y, size: 1, note });
            }
        }
    }
    for &(x, y) in swords {
        let near = (y.saturating_sub(12)..(y + 8).min(h as u32))
            .flat_map(|yy| (x.saturating_sub(15)..(x + 18).min(w as u32)).map(move |xx| (xx, yy)))
            .filter(|&(xx, yy)| packed_wall(m.cells[yy as usize * w + xx as usize]) == SHRINE_WALL)
            .count();
        let shrine = near >= 100;
        if let Some(k) = POI_KINDS.iter().position(|k| matches!(k.find, Find::Sword { shrine: s } if s == shrine)) {
            out.push(Poi { kind: k, x, y, size: 1, note: "" });
        }
    }
    for (ai, &k) in areas.iter().enumerate() {
        let c = &counts[ai];
        let min_bin = if matches!(POI_KINDS[k].find, Find::Shimmer) { 4 } else { 24 };
        let mut seen = vec![false; bw * bh];
        for start in 0..bw * bh {
            if seen[start] || c[start] < min_bin {
                continue;
            }
            // Join neighbouring squares into one patch.
            let mut stack = vec![start];
            seen[start] = true;
            let mut bins = vec![];
            while let Some(b) = stack.pop() {
                bins.push(b);
                let (bx, by) = ((b % bw) as i64, (b / bw) as i64);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (bx + dx, by + dy);
                        if nx < 0 || ny < 0 || nx >= bw as i64 || ny >= bh as i64 {
                            continue;
                        }
                        let n = ny as usize * bw + nx as usize;
                        if !seen[n] && c[n] >= min_bin {
                            seen[n] = true;
                            stack.push(n);
                        }
                    }
                }
            }
            let total: u32 = bins.iter().map(|&b| c[b] as u32).sum();
            if total < POI_KINDS[k].area.unwrap_or(0) {
                continue;
            }
            // Mark the square nearest the middle of the patch (weighted by how much is there).
            let (sx, sy) = bins.iter().fold((0f64, 0f64), |(sx, sy), &b| (sx + (b % bw) as f64 * c[b] as f64, sy + (b / bw) as f64 * c[b] as f64));
            let (mx, my) = (sx / total as f64, sy / total as f64);
            let best = *bins.iter().min_by(|&&a, &&b| {
                let da = ((a % bw) as f64 - mx).powi(2) + ((a / bw) as f64 - my).powi(2);
                let db = ((b % bw) as f64 - mx).powi(2) + ((b / bw) as f64 - my).powi(2);
                da.total_cmp(&db)
            }).unwrap();
            let (x, y) = (((best % bw) * BIN + BIN / 2).min(w - 1), ((best / bw) * BIN + BIN / 2).min(h - 1));
            out.push(Poi { kind: k, x: x as u32, y: y as u32, size: total, note: "" });
        }
    }
    out
}

// ------------------------------------------------------------------ map edits

pub const PAINT_BLOCK: u8 = 1;
pub const ERASE_BLOCK: u8 = 2;
pub const PAINT_WALL: u8 = 3;
pub const ERASE_WALL: u8 = 4;
pub const PAINT_LIQUID: u8 = 5;
pub const DRAIN_LIQUID: u8 = 6;
pub const LIQUIDS: [&str; 4] = ["Water", "Lava", "Honey", "Shimmer"];

/// One brush stroke: a square brush of `size` tiles stamped at each point.
#[derive(Clone, Debug, PartialEq)]
pub struct Op {
    pub mode: u8,
    pub value: u16,
    pub size: u16,
    pub points: Vec<(u32, u32)>,
}

impl Op {
    pub fn to_text(&self) -> String {
        let mut s = format!("{} {} {}", self.mode, self.value, self.size);
        for (x, y) in &self.points {
            s.push_str(&format!(" {x} {y}"));
        }
        s
    }

    /// The tiles one stamp covers, clipped to the world.
    pub fn stamp(&self, (x, y): (u32, u32), w: usize, h: usize) -> impl Iterator<Item = (usize, usize)> {
        let half = (self.size as i64 - 1) / 2;
        let (x0, y0) = ((x as i64 - half).max(0), (y as i64 - half).max(0));
        let (x1, y1) = ((x as i64 - half + self.size as i64).min(w as i64), (y as i64 - half + self.size as i64).min(h as i64));
        (y0..y1.max(y0)).flat_map(move |yy| (x0..x1.max(x0)).map(move |xx| (xx as usize, yy as usize)))
    }
}

pub fn parse_ops(s: &str) -> Result<Vec<Op>, String> {
    let mut ops = Vec::new();
    for part in s.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let n: Vec<u32> = part.split_whitespace().map(|t| t.parse::<u32>().map_err(|_| "map edits aren't in the expected form".to_string())).collect::<Result<_, _>>()?;
        if n.len() < 5 || n.len() % 2 == 0 || n[0] > 255 || n[1] > u16::MAX as u32 || n[2] > 64 {
            return Err("map edits aren't in the expected form".into());
        }
        ops.push(Op { mode: n[0] as u8, value: n[1] as u16, size: n[2] as u16, points: n[3..].chunks(2).map(|p| (p[0], p[1])).collect() });
    }
    Ok(ops)
}

/// Is this a paint stroke the world can take?
pub fn op_ok(op: &Op, map: &WorldMap) -> Result<(), String> {
    let pal = palette();
    match op.mode {
        PAINT_BLOCK => {
            let t = op.value as usize;
            if t >= map.important.len() || map.important[t] || pal.tiles.get(t).is_none_or(Option::is_none) {
                return Err(format!("tile #{t} can't be painted"));
            }
        }
        PAINT_WALL => {
            let w = op.value as usize;
            if w == 0 || w > 0x3FF || pal.walls.get(w).is_none_or(Option::is_none) {
                return Err(format!("wall #{w} can't be painted"));
            }
        }
        PAINT_LIQUID => {
            if !(1..=4).contains(&op.value) || (op.value == 4 && !map.shimmer_ok()) {
                return Err("unknown liquid".into());
            }
        }
        ERASE_BLOCK | ERASE_WALL | DRAIN_LIQUID => {}
        _ => return Err("unknown map edit".into()),
    }
    if op.size == 0 || op.points.iter().any(|&(x, y)| x as usize >= map.width || y as usize >= map.height) {
        return Err("map edit outside the world".into());
    }
    Ok(())
}

fn is_object(t: Option<u16>, imp: &[bool]) -> bool {
    t.is_some_and(|t| imp.get(t as usize).copied().unwrap_or(true))
}

/// Applies one paint mode to a stored tile.
pub fn apply_cell(mode: u8, value: u16, c: &mut Cell, imp: &[bool]) {
    match mode {
        PAINT_BLOCK | ERASE_BLOCK => {
            if is_object(c.tile, imp) || (mode == ERASE_BLOCK && c.tile.is_none()) {
                return;
            }
            c.tile = (mode == PAINT_BLOCK).then_some(value);
            c.frame = (0, 0);
            c.tcolor = 0;
            c.h2 &= !0x70; // slope / half block
            c.h3 &= !4; // actuated
            c.h4 &= !(2 | 8); // invisible / full-bright block
            if mode == PAINT_BLOCK {
                c.liquid = 0;
                c.amount = 0;
                c.h3 &= !0x80;
            }
        }
        PAINT_WALL | ERASE_WALL => {
            c.wall = if mode == PAINT_WALL { value } else { 0 };
            c.wcolor = 0;
            c.h4 &= !(4 | 16);
        }
        PAINT_LIQUID => {
            if c.tile.is_some() {
                return;
            }
            c.liquid = if value == 4 { 1 } else { value as u8 };
            c.amount = 255;
            if value == 4 {
                c.h3 |= 0x80;
            } else {
                c.h3 &= !0x80;
            }
        }
        DRAIN_LIQUID => {
            c.liquid = 0;
            c.amount = 0;
            c.h3 &= !0x80;
        }
        _ => {}
    }
}

/// The same edit on a packed tile (for drawing), matching `apply_cell`.
pub fn apply_packed(mode: u8, value: u16, p: u32, imp: &[bool]) -> u32 {
    let t = packed_tile(p);
    match mode {
        PAINT_BLOCK | ERASE_BLOCK => {
            if is_object(t, imp) || (mode == ERASE_BLOCK && t.is_none()) {
                return p;
            }
            let mut p = p & !0xFFF & !(1 << 25);
            if mode == PAINT_BLOCK {
                p = (p & !(7 << 22)) | (value as u32 + 1);
            }
            p
        }
        PAINT_WALL => (p & !(0x3FF << 12)) | (value as u32) << 12,
        ERASE_WALL => p & !(0x3FF << 12),
        PAINT_LIQUID if t.is_none() => (p & !(7 << 22)) | (value as u32) << 22,
        DRAIN_LIQUID => p & !(7 << 22),
        _ => p,
    }
}

// ------------------------------------------------------------------ fields

#[derive(Clone, Debug)]
enum Field {
    Name,
    Int { at: usize, size: u8, min: i64, max: i64 },
    Bool { at: usize },
    Ticks { at: usize },
    Mode { at: usize },
    Evil { at: usize },
    ChestName(usize),
    Item(usize, usize),
    Stack(usize, usize),
    Prefix(usize, usize),
    Sign(usize),
    Edits,
}

#[derive(Default)]
struct Builder {
    fields: Vec<Field>,
    editable: HashMap<String, usize>,
}

impl Builder {
    fn add(&mut self, map: &mut Map<String, Value>, base: &str, key: &str, v: Value, f: Field) {
        self.editable.insert(format!("{base}/{}", esc(key)), self.fields.len());
        self.fields.push(f);
        map.insert(key.to_string(), v);
    }
}

#[derive(Clone, Debug)]
struct Chest {
    x: i32,
    y: i32,
    name: String,
    /// (stack, item id, prefix)
    slots: Vec<(i16, i32, u8)>,
}

#[derive(Clone, Debug)]
struct Sign {
    x: i32,
    y: i32,
    text: String,
}

// ------------------------------------------------------------------ world info

struct Header {
    root: Vec<(String, Map<String, Value>)>,
    spawn: Option<(i32, i32)>,
    surface: f64,
    rock: f64,
}

/// Reads the world info section. `newer`: the Terraria 1.4.5 layout (a ninth seed flag and
/// a second timestamp before the creation time, and a second banner table after the kill
/// counts). Checked by sanity limits, since the section ends with data we don't read.
fn read_header(d: &[u8], start: usize, v: i32, newer: bool, b: &mut Builder) -> Result<Header, String> {
    let mut r = Reader { d, pos: start };
    let mut world = Map::new();
    let wp = "/World";
    let (_, _, name) = r.string()?;
    b.add(&mut world, wp, "Name", Value::from(name), Field::Name);
    let seed = r.text()?;
    world.insert("Seed".into(), Value::from(seed));
    r.take(8 + 16)?; // generator version, GUID
    let id = r.i32()?;
    r.take(16)?; // bounds in pixels
    let max_y = r.i32()?;
    let max_x = r.i32()?;
    let size = match max_x {
        4200 => "Small",
        6400 => "Medium",
        8400 => "Large",
        _ => "Custom",
    };
    world.insert("Size".into(), Value::from(format!("{size} ({max_x} \u{00D7} {max_y} tiles)")));
    world.insert("World ID".into(), Value::from(id));
    world.insert("File version".into(), Value::from(v));
    let at = r.take(4)?;
    let mode = i32::from_le_bytes(d[at..at + 4].try_into().unwrap());
    let mode_name = MODES.get(mode as usize).ok_or("unexpected world info")?;
    b.add(&mut world, wp, "Difficulty", Value::from(*mode_name), Field::Mode { at });

    let mut bools: Vec<(usize, u8)> = vec![]; // for the sanity check
    fn flag_in(d: &[u8], bools: &mut Vec<(usize, u8)>, b: &mut Builder, r: &mut Reader, m: &mut Map<String, Value>, base: &str, key: Option<&str>) -> Result<(), String> {
        let at = r.take(1)?;
        bools.push((at, d[at]));
        if let Some(k) = key {
            b.add(m, base, k, Value::Bool(d[at] != 0), Field::Bool { at });
        }
        Ok(())
    }
    macro_rules! flag {
        ($b:expr, $r:expr, $m:expr, $base:expr, $key:expr) => {
            flag_in(d, &mut bools, $b, $r, $m, $base, $key)
        };
    }
    let mut seeds = Map::new();
    let sp = "/Special seeds";
    for (need, key) in [(222, "Drunk world (05162020)"), (227, "For the worthy"), (238, "Celebrationmk10"), (239, "The Constant"), (241, "Not the bees"), (249, "Don't dig up"), (266, "No traps"), (267, "Zenith (Get fixed boi)")] {
        if v >= need {
            flag!(b, &mut r, &mut seeds, sp, Some(key))?;
        }
    }
    if newer {
        flag!(b, &mut r, &mut seeds, sp, None)?;
        r.take(8)?;
    }
    r.take(8)?; // creation time
    r.take(1 + 17 * 4)?; // moon type, tree and cave background positions and styles
    let int = |b: &mut Builder, r: &mut Reader, m: &mut Map<String, Value>, base: &str, key: &str, min: i64, max: i64| -> Result<i64, String> {
        let at = r.take(4)?;
        let n = i32::from_le_bytes(d[at..at + 4].try_into().unwrap()) as i64;
        b.add(m, base, key, Value::from(n), Field::Int { at, size: 4, min, max });
        Ok(n)
    };
    let sx = int(b, &mut r, &mut world, wp, "Spawn X", 0, max_x as i64 - 1)?;
    let sy = int(b, &mut r, &mut world, wp, "Spawn Y", 0, max_y as i64 - 1)?;
    let surface = r.f64()?;
    let rock = r.f64()?;
    let at = r.take(8)?;
    let time = f64::from_le_bytes(d[at..at + 8].try_into().unwrap());
    b.add(&mut world, wp, "Time (ticks)", Value::from(time.round() as i64), Field::Ticks { at });
    flag!(b, &mut r, &mut world, wp, Some("Daytime"))?;
    int(b, &mut r, &mut world, wp, "Moon phase", 0, 7)?;
    flag!(b, &mut r, &mut world, wp, Some("Blood Moon"))?;
    flag!(b, &mut r, &mut world, wp, Some("Solar Eclipse"))?;
    let dx = int(b, &mut r, &mut world, wp, "Dungeon X", 0, max_x as i64 - 1)?;
    let dy = int(b, &mut r, &mut world, wp, "Dungeon Y", 0, max_y as i64 - 1)?;
    let at = r.take(1)?;
    bools.push((at, d[at]));
    b.add(&mut world, wp, "Evil", Value::from(EVIL[(d[at] != 0) as usize]), Field::Evil { at });

    let mut bosses = Map::new();
    let bp = "/Bosses defeated";
    let mut events = Map::new();
    let ep = "/Events";
    let mut town = Map::new();
    let tp = "/Town NPCs";
    // Fixed order in the file; shown in the order players meet them.
    let mut boss_at: Vec<(&str, usize)> = vec![];
    fn boss(d: &[u8], r: &mut Reader, bools: &mut Vec<(usize, u8)>, boss_at: &mut Vec<(&'static str, usize)>, key: &'static str) -> Result<(), String> {
        let at = r.take(1)?;
        bools.push((at, d[at]));
        boss_at.push((key, at));
        Ok(())
    }
    for k in ["Eye of Cthulhu", "Eater of Worlds / Brain of Cthulhu", "Skeletron", "Queen Bee", "The Destroyer", "The Twins", "Skeletron Prime", "Any mechanical boss", "Plantera", "Golem", "King Slime"] {
        boss(d, &mut r, &mut bools, &mut boss_at, k)?;
    }
    for k in ["Goblin Tinkerer rescued", "Wizard rescued", "Mechanic rescued"] {
        flag!(b, &mut r, &mut town, tp, Some(k))?;
    }
    for k in ["Goblin Army", "Clown (Blood Moon)", "Frost Legion", "Pirate Invasion"] {
        flag!(b, &mut r, &mut events, ep, Some(k))?;
    }
    flag!(b, &mut r, &mut world, wp, Some("Shadow Orb / Crimson Heart smashed"))?;
    flag!(b, &mut r, &mut world, wp, Some("Meteor due"))?;
    r.take(1)?; // orbs smashed (counter)
    int(b, &mut r, &mut world, wp, "Altars smashed", 0, 100_000)?;
    flag!(b, &mut r, &mut world, wp, Some("Hardmode"))?;
    if v >= 257 {
        flag!(b, &mut r, &mut events, ep, Some("Party of Doom over"))?;
    }
    r.take(4 * 3 + 8)?; // invasion delay, size, type, position
    r.take(8)?; // slime rain time
    r.take(1)?; // sundial cooldown
    flag!(b, &mut r, &mut world, wp, Some("Raining"))?;
    r.take(4 + 4)?; // rain time, max rain
    r.take(3 * 4 + 8)?; // hardmode ore tiers, background styles
    r.take(4)?; // cloud background
    r.i16()?; // clouds
    r.f32()?; // wind
    let anglers = r.i32()?;
    if !(0..=255).contains(&anglers) {
        return Err("unexpected world info".into());
    }
    for _ in 0..anglers {
        r.string()?;
    }
    flag!(b, &mut r, &mut town, tp, Some("Angler rescued"))?;
    int(b, &mut r, &mut world, wp, "Angler quest", 0, 100)?;
    flag!(b, &mut r, &mut town, tp, Some("Stylist rescued"))?;
    flag!(b, &mut r, &mut town, tp, Some("Tax Collector rescued"))?;
    flag!(b, &mut r, &mut town, tp, Some("Golfer rescued"))?;
    r.take(4 + 4)?; // invasion start size, cultist delay
    let kills = r.i16()?;
    if !(0..=5000).contains(&kills) {
        return Err("unexpected world info".into());
    }
    r.take(4 * kills as usize)?;
    if newer {
        let n = r.i16()?;
        if !(0..=5000).contains(&n) {
            return Err("unexpected world info".into());
        }
        r.take(2 * n as usize)?;
    }
    r.take(1)?; // fast-forward time
    for k in ["Duke Fishron", "Martian Madness", "Lunatic Cultist", "Moon Lord", "Pumpking", "Mourning Wood", "Ice Queen", "Santa-NK1", "Everscream"] {
        if matches!(k, "Martian Madness" | "Pumpking" | "Mourning Wood" | "Ice Queen" | "Santa-NK1" | "Everscream") {
            flag!(b, &mut r, &mut events, ep, Some(k))?;
        } else {
            boss(d, &mut r, &mut bools, &mut boss_at, k)?;
        }
    }
    for k in ["Solar Pillar", "Vortex Pillar", "Nebula Pillar", "Stardust Pillar"] {
        flag!(b, &mut r, &mut events, ep, Some(k))?;
    }
    for _ in 0..5 {
        flag!(b, &mut r, &mut events, ep, None)?; // pillars active, lunar events
    }
    r.take(2 + 4)?; // party
    let n = r.i32()?;
    if !(0..=1000).contains(&n) {
        return Err("unexpected world info".into());
    }
    r.take(4 * n as usize)?;
    r.take(1 + 4 + 4 + 4)?; // sandstorm
    flag!(b, &mut r, &mut town, tp, Some("Tavernkeep rescued"))?;
    for k in ["Old One's Army (tier 1)", "Old One's Army (tier 2)", "Old One's Army (tier 3)"] {
        flag!(b, &mut r, &mut events, ep, Some(k))?;
    }
    r.take(1 + 4)?; // background styles
    flag!(b, &mut r, &mut town, tp, Some("Advanced Combat Techniques used"))?;
    r.take(4 + 3)?; // lantern night
    let n = r.i32()?;
    if !(0..=1000).contains(&n) {
        return Err("unexpected world info".into());
    }
    r.take(4 * n as usize)?;
    r.take(2)?; // forced Halloween / Christmas
    r.take(4 * 4)?; // ore choices
    for k in ["Cat License used", "Dog License used", "Bunny License used"] {
        flag!(b, &mut r, &mut town, tp, Some(k))?;
    }
    boss(d, &mut r, &mut bools, &mut boss_at, "Empress of Light")?;
    boss(d, &mut r, &mut bools, &mut boss_at, "Queen Slime")?;
    if v >= 240 {
        boss(d, &mut r, &mut bools, &mut boss_at, "Deerclops")?;
    }
    for k in [
        "King Slime",
        "Eye of Cthulhu",
        "Eater of Worlds / Brain of Cthulhu",
        "Queen Bee",
        "Skeletron",
        "Deerclops",
        "Queen Slime",
        "The Destroyer",
        "The Twins",
        "Skeletron Prime",
        "Any mechanical boss",
        "Plantera",
        "Golem",
        "Duke Fishron",
        "Empress of Light",
        "Lunatic Cultist",
        "Moon Lord",
    ] {
        if let Some(&(_, at)) = boss_at.iter().find(|(n, _)| *n == k) {
            b.add(&mut bosses, bp, k, Value::Bool(d[at] != 0), Field::Bool { at });
        }
    }

    // Sanity: everything read as on/off is 0 or 1, and positions are inside the world.
    let inside = |x: i64, y: i64| (0..max_x as i64).contains(&x) && (0..max_y as i64).contains(&y);
    if bools.iter().any(|&(_, v)| v > 1) || !inside(sx, sy) || !inside(dx, dy) || !(1.0..max_y as f64).contains(&surface) || !(surface..max_y as f64).contains(&rock) || !(0.0..=86400.0).contains(&time) {
        return Err("unexpected world info".into());
    }
    let mut root = vec![("World".to_string(), world)];
    if !seeds.is_empty() {
        root.push(("Special seeds".into(), seeds));
    }
    root.push(("Bosses defeated".into(), bosses));
    root.push(("Events".into(), events));
    root.push(("Town NPCs".into(), town));
    Ok(Header { root, spawn: Some((sx as i32, sy as i32)), surface, rock })
}

/// When the world info can't be read fully: just the name, size and id.
fn read_header_basic(d: &[u8], start: usize, v: i32, b: &mut Builder) -> Result<(Header, usize, usize), String> {
    let mut r = Reader { d, pos: start };
    let mut world = Map::new();
    let (_, _, name) = r.string()?;
    b.add(&mut world, "/World", "Name", Value::from(name), Field::Name);
    world.insert("Seed".into(), Value::from(r.text()?));
    r.take(8 + 16 + 4 + 16)?;
    let max_y = r.i32()?;
    let max_x = r.i32()?;
    world.insert("Size".into(), Value::from(format!("{max_x} \u{00D7} {max_y} tiles")));
    world.insert("File version".into(), Value::from(v));
    world.insert("Note".into(), Value::from("This world's file version isn't fully known, so only its name, chests, signs and map can be edited."));
    Ok((Header { root: vec![("World".into(), world)], spawn: None, surface: max_y as f64 * 0.3, rock: max_y as f64 * 0.4 }, max_x as usize, max_y as usize))
}

fn world_size(d: &[u8], start: usize) -> Result<(usize, usize), String> {
    let mut r = Reader { d, pos: start };
    r.string()?;
    r.string()?;
    r.take(8 + 16 + 4 + 16)?;
    let h = r.i32()?;
    let w = r.i32()?;
    if !(100..=20_000).contains(&w) || !(100..=20_000).contains(&h) {
        return Err("unexpected world size".into());
    }
    Ok((w as usize, h as usize))
}

// ------------------------------------------------------------------ sections

fn read_chests(d: &[u8], start: usize, end: usize, per_chest: bool) -> Result<(Vec<Chest>, i16), String> {
    let mut r = Reader { d: &d[..end], pos: start };
    let n = r.i16()?;
    let mut per = if per_chest { 0 } else { r.i16()? };
    if n < 0 || per < 0 || per > 200 {
        return Err("unexpected chests".into());
    }
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let x = r.i32()?;
        let y = r.i32()?;
        let name = r.text()?;
        let slots = if per_chest {
            let k = r.i32()?;
            if !(0..=200).contains(&k) {
                return Err("unexpected chests".into());
            }
            k as usize
        } else {
            per as usize
        };
        let mut s = Vec::with_capacity(slots);
        for _ in 0..slots {
            let st = r.i16()?;
            if st > 0 {
                let id = r.i32()?;
                let pre = r.u8()?;
                s.push((st, id, pre));
            } else {
                s.push((0, 0, 0));
            }
        }
        out.push(Chest { x, y, name, slots: s });
    }
    if r.pos != end {
        return Err("unexpected chests".into());
    }
    if per_chest {
        per = 0;
    }
    Ok((out, per))
}

fn write_chests(chests: &[Chest], per: i16, per_chest: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(chests.len() as i16).to_le_bytes());
    if !per_chest {
        out.extend_from_slice(&per.to_le_bytes());
    }
    for c in chests {
        out.extend_from_slice(&c.x.to_le_bytes());
        out.extend_from_slice(&c.y.to_le_bytes());
        out.extend_from_slice(&net_string(&c.name));
        if per_chest {
            out.extend_from_slice(&(c.slots.len() as i32).to_le_bytes());
        }
        for &(st, id, pre) in &c.slots {
            if st > 0 && id != 0 {
                out.extend_from_slice(&st.to_le_bytes());
                out.extend_from_slice(&id.to_le_bytes());
                out.push(pre);
            } else {
                out.extend_from_slice(&0i16.to_le_bytes());
            }
        }
    }
    out
}

fn read_signs(d: &[u8], start: usize, end: usize) -> Result<Vec<Sign>, String> {
    let mut r = Reader { d: &d[..end], pos: start };
    let n = r.i16()?;
    let mut out = Vec::new();
    for _ in 0..n.max(0) {
        let text = r.text()?;
        let x = r.i32()?;
        let y = r.i32()?;
        out.push(Sign { x, y, text });
    }
    if r.pos != end {
        return Err("unexpected signs".into());
    }
    Ok(out)
}

fn write_signs(signs: &[Sign]) -> Vec<u8> {
    let mut out = (signs.len() as i16).to_le_bytes().to_vec();
    for s in signs {
        out.extend_from_slice(&net_string(&s.text));
        out.extend_from_slice(&s.x.to_le_bytes());
        out.extend_from_slice(&s.y.to_le_bytes());
    }
    out
}

/// Town NPCs: (id, name, home x, home y, homeless). `extra`: Terraria 1.4.5's extra byte per NPC.
fn read_npcs(d: &[u8], start: usize, end: usize, v: i32, extra: bool) -> Result<Vec<(i32, String, i32, i32, bool)>, String> {
    let mut r = Reader { d: &d[..end], pos: start };
    if v >= 268 {
        let n = r.i32()?;
        if !(0..=1000).contains(&n) {
            return Err("unexpected NPCs".into());
        }
        r.take(4 * n as usize)?;
    }
    let mut out = Vec::new();
    while r.u8()? != 0 {
        let id = r.i32()?;
        let name = r.text()?;
        r.take(8)?; // position
        let homeless = r.u8()? != 0;
        let hx = r.i32()?;
        let hy = r.i32()?;
        if r.u8()? & 1 != 0 {
            r.i32()?;
        }
        if extra {
            r.u8()?;
        }
        out.push((id, name, hx, hy, homeless));
    }
    while r.u8()? != 0 {
        r.take(12)?; // other saved NPCs: type and position
    }
    if r.pos != end {
        return Err("unexpected NPCs".into());
    }
    Ok(out)
}

// ------------------------------------------------------------------ backend

#[derive(Clone)]
struct WldBackend {
    raw: Arc<Vec<u8>>,
    /// Start of each section; the last one runs to the end of the file.
    sections: Vec<usize>,
    name_span: (usize, usize),
    fields: Vec<Field>,
    chests: Arc<Vec<Chest>>,
    chest_per: i16,
    per_chest: bool,
    signs: Arc<Vec<Sign>>,
    map: Arc<WorldMap>,
    /// Where each tile column starts in the file (plus the end of the last one).
    columns: Arc<Vec<usize>>,
    /// For each chest Item leaf: pointers of its Stack and Prefix.
    slot_ptrs: HashMap<usize, (String, String)>,
}

impl std::fmt::Debug for WldBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Terraria world ({} chests)", self.chests.len())
    }
}

impl WldBackend {
    fn section(&self, i: usize) -> std::ops::Range<usize> {
        self.sections[i]..self.sections.get(i + 1).copied().unwrap_or(self.raw.len())
    }

    fn tiles_with(&self, ops: &[Op]) -> Vec<u8> {
        let (w, h) = (self.map.width, self.map.height);
        let mut per_col: BTreeMap<usize, Vec<(usize, u8, u16)>> = BTreeMap::new();
        for op in ops {
            for &pt in &op.points {
                for (x, y) in op.stamp(pt, w, h) {
                    per_col.entry(x).or_default().push((y, op.mode, op.value));
                }
            }
        }
        let d = &self.raw;
        let imp = &self.map.important;
        let mut out = Vec::with_capacity(self.section(1).len() + 1024);
        let mut x = 0;
        for (&cx, edits) in &per_col {
            out.extend_from_slice(&d[self.columns[x]..self.columns[cx]]);
            match read_column(&d[self.columns[cx]..self.columns[cx + 1]], h, imp) {
                Ok(mut col) => {
                    for &(y, mode, value) in edits {
                        apply_cell(mode, value, &mut col[y], imp);
                    }
                    write_column(&col, imp, &mut out);
                }
                Err(_) => out.extend_from_slice(&d[self.columns[cx]..self.columns[cx + 1]]),
            }
            x = cx + 1;
        }
        out.extend_from_slice(&d[self.columns[x]..self.columns[w]]);
        out
    }
}

impl Backend for WldBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let d = &self.raw;
        let n = self.sections.len();
        let mut parts: Vec<Vec<u8>> = (0..n).map(|i| d[self.section(i)].to_vec()).collect();
        let h0 = self.sections[0];
        let mut name: Option<String> = None;
        let mut chests: Option<Vec<Chest>> = None;
        let mut signs: Option<Vec<Sign>> = None;
        let mut ops: Option<Vec<Op>> = None;
        for (leaf, v) in changes {
            let s = || v.as_str().ok_or_else(|| "expected text".to_string());
            let mut put = |at: usize, bytes: &[u8]| parts[0][at - h0..at - h0 + bytes.len()].copy_from_slice(bytes);
            match &self.fields[*leaf] {
                Field::Name => name = Some(s()?.to_string()),
                Field::Int { at, size, min, max } => {
                    let n = int_in(v, *min as i128, *max as i128, "This value")? as i64;
                    match size {
                        1 => put(*at, &[n as u8]),
                        _ => put(*at, &(n as i32).to_le_bytes()),
                    }
                }
                Field::Bool { at } => put(*at, &[u8::from(v.as_bool().ok_or("expected on/off")?)]),
                Field::Ticks { at } => put(*at, &(int_in(v, 0, 54_000, "Time")? as f64).to_le_bytes()),
                Field::Mode { at } => put(*at, &(MODES.iter().position(|m| Some(*m) == v.as_str()).ok_or("unknown difficulty")? as i32).to_le_bytes()),
                Field::Evil { at } => put(*at, &[u8::from(v.as_str() == Some(EVIL[1]))]),
                Field::ChestName(c) => chests.get_or_insert_with(|| self.chests.to_vec())[*c].name = s()?.to_string(),
                Field::Item(c, k) => {
                    let id = item_id(s()?).ok_or("unknown item")?;
                    chests.get_or_insert_with(|| self.chests.to_vec())[*c].slots[*k].1 = id;
                }
                Field::Stack(c, k) => {
                    let n = int_in(v, 0, 9999, "Stack")? as i16;
                    chests.get_or_insert_with(|| self.chests.to_vec())[*c].slots[*k].0 = n;
                }
                Field::Prefix(c, k) => {
                    let p = prefix_id(s()?).ok_or("unknown prefix")?;
                    chests.get_or_insert_with(|| self.chests.to_vec())[*c].slots[*k].2 = p;
                }
                Field::Sign(i) => signs.get_or_insert_with(|| self.signs.to_vec())[*i].text = s()?.to_string(),
                Field::Edits => ops = Some(parse_ops(s()?)?),
            }
        }
        if let Some(name) = &name {
            let (a, b) = (self.name_span.0 - h0, self.name_span.1 - h0);
            parts[0].splice(a..b, net_string(name));
            // The footer repeats the name: a flag byte, the name, the world id.
            let f = &d[self.section(n - 1)];
            let mut r = Reader { d: f, pos: 1 };
            r.string()?;
            let mut foot = vec![f[0]];
            foot.extend_from_slice(&net_string(name));
            foot.extend_from_slice(&f[r.pos..]);
            parts[n - 1] = foot;
        }
        if let Some(c) = &chests {
            parts[2] = write_chests(c, self.chest_per, self.per_chest);
        }
        if let Some(s) = &signs {
            parts[3] = write_signs(s);
        }
        if let Some(ops) = ops.filter(|o| !o.is_empty()) {
            parts[1] = self.tiles_with(&ops);
        }
        let mut out = d[..self.sections[0]].to_vec();
        for (i, p) in parts.iter().enumerate() {
            let at = out.len() as i32;
            out[26 + 4 * i..30 + 4 * i].copy_from_slice(&at.to_le_bytes());
            out.extend_from_slice(p);
        }
        Ok(out)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let s = || v.as_str().ok_or_else(|| "expected text".to_string());
        match &self.fields[leaf] {
            Field::Name => match s()?.chars().count() {
                0 => Err("the name can't be empty".into()),
                n if n > MAX_NAME => Err(format!("world names can be at most {MAX_NAME} characters")),
                _ => Ok(()),
            },
            Field::Int { min, max, .. } => int_in(v, *min as i128, *max as i128, "This value").map(|_| ()),
            Field::Ticks { .. } => int_in(v, 0, 54_000, "Time").map(|_| ()),
            Field::Bool { .. } => v.as_bool().map(|_| ()).ok_or_else(|| "expected on/off".into()),
            Field::Mode { .. } => MODES.contains(&s()?).then_some(()).ok_or_else(|| "unknown difficulty".into()),
            Field::Evil { .. } => EVIL.contains(&s()?).then_some(()).ok_or_else(|| "expected Corruption or Crimson".into()),
            Field::ChestName(_) => (s()?.chars().count() <= 20).then_some(()).ok_or_else(|| "chest names can be at most 20 characters".into()),
            Field::Item(..) => item_id(s()?).map(|_| ()).ok_or_else(|| format!("there's no item called \u{201C}{}\u{201D}", s().unwrap_or_default())),
            Field::Stack(..) => int_in(v, 0, 9999, "Stack").map(|_| ()),
            Field::Prefix(..) => prefix_id(s()?).map(|_| ()).ok_or_else(|| "unknown prefix".into()),
            Field::Sign(_) => s().map(|_| ()),
            Field::Edits => parse_ops(s()?)?.iter().try_for_each(|op| op_ok(op, &self.map)),
        }
    }

    fn derived(&self, leaf: usize, view: &Value) -> Vec<(String, Value)> {
        let (Field::Item(..), Some((stack, prefix))) = (&self.fields[leaf], self.slot_ptrs.get(&leaf)) else { return vec![] };
        let item_ptr = format!("{}/Item", stack.rsplit_once('/').map_or("", |p| p.0));
        let empty = view.pointer(&item_ptr).and_then(Value::as_str) == Some(EMPTY);
        let n = view.pointer(stack).and_then(Value::as_i64).unwrap_or(0);
        let mut out = Vec::new();
        if empty {
            out.push((stack.clone(), Value::from(0)));
            out.push((prefix.clone(), Value::from(NO_PREFIX)));
        } else if n <= 0 {
            out.push((stack.clone(), Value::from(1)));
        }
        out
    }

    fn choices(&self, leaf: usize, _view: &Value) -> Option<Arc<Vec<String>>> {
        match &self.fields[leaf] {
            Field::Item(..) => Some(tables().item_list_by_id.clone()),
            Field::Prefix(..) => Some(tables().prefix_list.clone()),
            Field::Mode { .. } => Some(Arc::new(MODES.iter().map(|s| s.to_string()).collect())),
            Field::Evil { .. } => Some(Arc::new(EVIL.iter().map(|s| s.to_string()).collect())),
            _ => None,
        }
    }

    fn reshapes(&self, leaf: usize) -> bool {
        matches!(self.fields[leaf], Field::Edits)
    }

    fn attachment(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        Some(self.map.clone())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

// ------------------------------------------------------------------ parse

pub fn looks(d: &[u8]) -> bool {
    d.len() > 64 && &d[4..11] == b"relogic" && d[11] == 2
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Terraria world".into());
    }
    let mut r = Reader { d, pos: 0 };
    let v = r.i32()?;
    if v < MIN_VERSION {
        return Err(format!("world file version {v} is older than Terraria 1.4"));
    }
    r.take(7 + 1 + 4 + 8)?; // signature, file type, revision, favourite flags
    let n = r.i16()?;
    if !(10..=32).contains(&n) {
        return Err("unexpected world layout".into());
    }
    let mut sections = Vec::new();
    for _ in 0..n {
        sections.push(r.i32()? as usize);
    }
    let nt = r.i16()?.max(0) as usize;
    let at = r.take(nt.div_ceil(8))?;
    let important: Vec<bool> = (0..nt).map(|i| d[at + i / 8] >> (i % 8) & 1 != 0).collect();
    if sections[0] != r.pos || sections.windows(2).any(|w| w[0] >= w[1]) || *sections.last().unwrap() >= d.len() {
        return Err("unexpected world layout".into());
    }

    let mut b = Builder::default();
    let (w, h) = world_size(d, sections[0])?;
    let header = read_header(d, sections[0], v, v >= 300, &mut b).or_else(|_| {
        b = Builder::default();
        read_header(d, sections[0], v, v < 300, &mut b)
    });
    let header = match header {
        Ok(hd) => hd,
        Err(_) => {
            b = Builder::default();
            read_header_basic(d, sections[0], v, &mut b)?.0
        }
    };
    let name_span = {
        let mut r = Reader { d, pos: sections[0] };
        let (s, e, _) = r.string()?;
        (s, e)
    };

    let (chests, chest_per, per_chest) = match read_chests(d, sections[2], sections[3], v >= 300) {
        Ok((c, p)) => (c, p, v >= 300),
        Err(_) => {
            let (c, p) = read_chests(d, sections[2], sections[3], v < 300)?;
            (c, p, v < 300)
        }
    };
    let chest_at: std::collections::HashSet<(i32, i32)> = chests.iter().map(|c| (c.x, c.y)).collect();
    let mut chest_tiles: Vec<ChestTile> = Vec::new();

    // Tiles, column by column.
    let mut cells = vec![0u32; w * h];
    let mut columns = Vec::with_capacity(w + 1);
    let mut swords = Vec::new();
    let mut r = Reader { d: &d[..sections[2]], pos: sections[1] };
    for x in 0..w {
        columns.push(r.pos);
        let mut y = 0;
        while y < h {
            let (c, run) = read_cell(&mut r, &important)?;
            if c.tile == Some(SWORD_TILE) && c.frame == SWORD_FRAME {
                swords.push((x as u32, y as u32));
            }
            if let Some(t @ (21 | 467)) = c.tile {
                if chest_at.contains(&(x as i32, y as i32)) {
                    chest_tiles.push((x as u32, y as u32, t, (c.frame.0 / 36) as u16, c.wall));
                }
            }
            let p = pack(&c);
            if y + run >= h {
                return Err("tile column overruns the world".into());
            }
            for k in 0..=run {
                cells[(y + k) * w + x] = p;
            }
            y += run + 1;
        }
    }
    columns.push(r.pos);
    if r.pos != sections[2] {
        return Err("unexpected tile data".into());
    }

    let signs = read_signs(d, sections[3], sections[4])?;
    let npcs = read_npcs(d, sections[4], sections[5], v, v >= 300).or_else(|_| read_npcs(d, sections[4], sections[5], v, v < 300)).unwrap_or_default();

    let mut root = Map::new();
    for (k, m) in header.root {
        root.insert(k, Value::Object(m));
    }
    let mut cm = Map::new();
    let mut slot_ptrs = HashMap::new();
    for (i, c) in chests.iter().enumerate() {
        let key = format!("Chest {}", i + 1);
        let base = format!("/Chests/{}", esc(&key));
        let mut m = Map::new();
        b.add(&mut m, &base, "Name", Value::from(c.name.clone()), Field::ChestName(i));
        m.insert("Position".into(), Value::from(format!("{}, {}", c.x, c.y)));
        for (k, &(st, id, pre)) in c.slots.iter().enumerate() {
            let sk = format!("Slot {}", k + 1);
            let sp = format!("{base}/{}", esc(&sk));
            let mut s = Map::new();
            let empty = st <= 0 || id == 0;
            let item_leaf = b.fields.len();
            b.add(&mut s, &sp, "Item", Value::from(if empty { EMPTY.to_string() } else { item_name(id) }), Field::Item(i, k));
            b.add(&mut s, &sp, "Stack", Value::from(if empty { 0 } else { st as i64 }), Field::Stack(i, k));
            b.add(&mut s, &sp, "Prefix", Value::from(if empty { NO_PREFIX.to_string() } else { prefix_name(pre) }), Field::Prefix(i, k));
            slot_ptrs.insert(item_leaf, (format!("{sp}/Stack"), format!("{sp}/Prefix")));
            m.insert(sk, Value::Object(s));
        }
        cm.insert(key, Value::Object(m));
    }
    root.insert("Chests".into(), Value::Object(cm));
    let mut sm = Map::new();
    for (i, s) in signs.iter().enumerate() {
        let key = format!("Sign {}", i + 1);
        let base = format!("/Signs/{}", esc(&key));
        let mut m = Map::new();
        b.add(&mut m, &base, "Text", Value::from(s.text.clone()), Field::Sign(i));
        m.insert("Position".into(), Value::from(format!("{}, {}", s.x, s.y)));
        sm.insert(key, Value::Object(m));
    }
    root.insert("Signs".into(), Value::Object(sm));
    let mut nm = Map::new();
    let mut npc_list = Vec::new();
    for (i, (id, name, hx, hy, homeless)) in npcs.iter().enumerate() {
        let role = npc_role(*id);
        let mut m = Map::new();
        m.insert("Role".into(), Value::from(role.clone()));
        m.insert("Name".into(), Value::from(name.clone()));
        m.insert("Home".into(), Value::from(if *homeless { "Homeless".to_string() } else { format!("{hx}, {hy}") }));
        nm.insert(format!("NPC {}", i + 1), Value::Object(m));
        npc_list.push((role, name.clone(), *hx, *hy, *homeless));
    }
    root.insert("NPCs".into(), Value::Object(nm));
    let mut em = Map::new();
    b.add(&mut em, "", "Map edits", Value::from(""), Field::Edits);
    root.insert("Map edits".into(), em.remove("Map edits").unwrap());

    let mut map = WorldMap {
        width: w,
        height: h,
        cells,
        important,
        surface: header.surface,
        rock: header.rock,
        version: v,
        chests: chests.iter().map(|c| (c.x, c.y)).collect(),
        npcs: npc_list,
        pois: vec![],
    };
    map.pois = points_of_interest(&map, &swords, &chest_tiles);
    let backend = WldBackend {
        raw: Arc::new(d.to_vec()),
        sections,
        name_span,
        fields: b.fields,
        chests: Arc::new(chests),
        chest_per,
        per_chest,
        signs: Arc::new(signs),
        map: Arc::new(map),
        columns: Arc::new(columns),
        slot_ptrs,
    };
    let _ = header.spawn;
    let doc = TreeDoc::new(Value::Object(root), b.editable, "Terraria world", Box::new(backend)).with_advanced(vec!["Map edits".into()]);
    Ok(doc.with_note("Close the world in Terraria (go back to the title screen) before saving, or the game will overwrite your edit. Keep a copy of the .wld until you've loaded it in the game."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_round_trip() {
        let imp = vec![false, true, false];
        let mut c = Cell { tile: Some(1), frame: (18, 36), tcolor: 3, wall: 300, wcolor: 2, liquid: 2, amount: 128, h2: 0x26, h3: 0x86, h4: 0x14 };
        let mut out = vec![];
        write_cell(&c, 300, &imp, &mut out);
        let mut r = Reader { d: &out, pos: 0 };
        let (back, run) = read_cell(&mut r, &imp).unwrap();
        assert_eq!((back, run, r.pos), (c, 300, out.len()));
        c = Cell::default();
        out.clear();
        write_cell(&c, 0, &imp, &mut out);
        assert_eq!(out, [0]);
    }

    #[test]
    fn ops_text() {
        let op = Op { mode: PAINT_WALL, value: 4, size: 3, points: vec![(10, 20), (11, 20)] };
        assert_eq!(parse_ops(&format!("{};{}", op.to_text(), op.to_text())).unwrap(), vec![op.clone(), op.clone()]);
        assert_eq!(op.stamp((0, 0), 100, 100).count(), 4);
        assert!(parse_ops("1 2").is_err());
    }

    /// TERRARIA_WORLDS = a folder of .wld files (copies). Reads each, checks an unchanged
    /// save is byte-identical, then edits world info, a chest, a sign, the name and the map
    /// and reads the result back.
    #[test]
    #[ignore]
    fn real_worlds() {
        let dir = std::env::var("TERRARIA_WORLDS").expect("TERRARIA_WORLDS");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            if e.path().extension().is_none_or(|x| x != "wld") {
                continue;
            }
            let raw = std::fs::read(e.path()).unwrap();
            let t = std::time::Instant::now();
            let mut doc = parse(&raw).unwrap();
            println!("{}: {:?}, {} chests, {:?}", e.path().display(), doc.view["World"]["Size"], doc.view["Chests"].as_object().unwrap().len(), t.elapsed());
            println!("  {}", serde_json::to_string(&doc.view["Bosses defeated"]).unwrap());
            {
                let m = doc.attachment().unwrap().downcast::<WorldMap>().unwrap();
                let mut n = vec![0; POI_KINDS.len()];
                for p in &m.pois {
                    n[p.kind] += 1;
                }
                println!("  points of interest: {:?}", POI_KINDS.iter().zip(n).filter(|(_, c)| *c > 0).map(|(k, c)| format!("{} {c}", k.name)).collect::<Vec<_>>());
            }
            assert_eq!(doc.to_bytes().unwrap(), raw);
            doc.set("/World/Name", Value::from("Edited World Name")).unwrap();
            if doc.view.pointer("/World/Hardmode").is_some() {
                doc.set("/World/Hardmode", Value::Bool(true)).unwrap();
                doc.set("/Bosses defeated/Moon Lord", Value::Bool(true)).unwrap();
            }
            doc.set("/Chests/Chest 1/Slot 1/Item", Value::from("Zenith")).unwrap();
            doc.set("/Chests/Chest 1/Name", Value::from("Loot")).unwrap();
            if doc.view.pointer("/Signs/Sign 1").is_some() {
                doc.set("/Signs/Sign 1/Text", Value::from("Hello")).unwrap();
            }
            let map = doc.attachment().unwrap().downcast::<WorldMap>().unwrap();
            let (cx, cy) = (map.width as u32 / 2, (map.surface as u32).saturating_sub(40));
            let ops = [
                Op { mode: PAINT_BLOCK, value: 1, size: 5, points: vec![(cx, cy), (cx + 3, cy)] },
                Op { mode: PAINT_WALL, value: 4, size: 3, points: vec![(cx, cy + 10)] },
                Op { mode: PAINT_LIQUID, value: 1, size: 2, points: vec![(cx + 20, cy)] },
            ];
            let text = ops.iter().map(Op::to_text).collect::<Vec<_>>().join(";");
            doc.set(EDITS, Value::from(text)).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out).unwrap();
            assert!(doc.agrees_with(&again));
            assert_eq!(again.view["World"]["Name"], "Edited World Name");
            assert_eq!(again.view["Chests"]["Chest 1"]["Slot 1"]["Item"], "Zenith");
            assert!(again.view["Chests"]["Chest 1"]["Slot 1"]["Stack"].as_i64().unwrap() >= 1);
            let m2 = again.attachment().unwrap().downcast::<WorldMap>().unwrap();
            let mut expect = map.cells.clone();
            for op in &ops {
                for &pt in &op.points {
                    for (x, y) in op.stamp(pt, map.width, map.height) {
                        expect[y * map.width + x] = apply_packed(op.mode, op.value, expect[y * map.width + x], &map.important);
                    }
                }
            }
            assert!(m2.cells == expect, "painted map reads back differently");
        }
    }
}
