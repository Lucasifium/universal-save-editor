//! Bethesda Creation Engine saves: Skyrim (LE, Special/Anniversary Edition, VR,
//! Enderal) `.ess` and Fallout 4 `.fos`.
//!
//! Layout: magic, header (player name, level, location, race…), screenshot, then the
//! game data (LZ4- or zlib-compressed in Skyrim SE). The game data starts with the
//! plugin list and a file location table, followed by "global data" tables. We
//! expose the header (read-only), the Misc Stats and the Global Variables (game
//! time, day counters…); those are fixed-size values, so they're patched in place
//! and the data is recompressed. Inventory, skills and the like live in "change
//! forms", whose format isn't documented well enough to edit safely.

use crate::tree::{Backend, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Game {
    Skyrim,
    Fallout4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Compression {
    None,
    Zlib,
    Lz4,
}

struct R<'a> {
    d: &'a [u8],
    i: usize,
}

impl R<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8], String> {
        let b = self.d.get(self.i..self.i + n).ok_or("unexpected end of save")?;
        self.i += n;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    /// u16 length + Windows-1252 text.
    fn wstring(&mut self) -> Result<String, String> {
        let n = self.u16()? as usize;
        Ok(self.bytes(n)?.iter().map(|&b| cp1252(b)).collect())
    }
    /// Variable-size value: low two bits give the width (1, 2 or 4 bytes).
    fn vsval(&mut self) -> Result<u32, String> {
        let b0 = self.u8()? as u32;
        Ok(match b0 & 3 {
            0 => b0 >> 2,
            1 => (b0 | (self.u8()? as u32) << 8) >> 2,
            _ => (b0 | (self.u8()? as u32) << 8 | (self.u16()? as u32) << 16) >> 2,
        })
    }
}

pub(super) fn cp1252(b: u8) -> char {
    const HIGH: &str = "€\u{81}‚ƒ„…†‡ˆ‰Š‹Œ\u{8D}Ž\u{8F}\u{90}‘’“”•–—˜™š›œ\u{9D}žŸ";
    match b {
        0x80..=0x9F => HIGH.chars().nth((b - 0x80) as usize).unwrap(),
        _ => b as char,
    }
}

/// Where the game data starts and how it's stored.
struct Container {
    game: Game,
    /// Bytes before the game data (magic, header, screenshot).
    prefix: usize,
    compression: Compression,
    header: Map<String, Value>,
    /// Where the header keeps the level (a copy of the player's real level).
    level_at: usize,
}

fn container(d: &[u8]) -> Result<Container, String> {
    let (game, magic) = if d.starts_with(b"TESV_SAVEGAME") {
        (Game::Skyrim, 13)
    } else if d.starts_with(b"FO4_SAVEGAME") {
        (Game::Fallout4, 12)
    } else {
        return Err("not a Bethesda save".into());
    };
    let mut r = R { d, i: magic };
    let header_size = r.u32()? as usize;
    let header_end = r.i + header_size;
    let version = r.u32()?;
    let save_number = r.u32()?;
    let mut h = Map::new();
    h.insert("Player name".into(), Value::from(r.wstring()?));
    let level_at = r.i;
    h.insert("Level".into(), Value::from(r.u32()?));
    h.insert("Location".into(), Value::from(r.wstring()?));
    h.insert(if game == Game::Skyrim { "In-game date" } else { "Play time" }.into(), Value::from(r.wstring()?));
    h.insert("Race".into(), Value::from(r.wstring()?));
    h.insert("Sex".into(), Value::from(if r.u16()? == 1 { "Female" } else { "Male" }));
    h.insert("Experience".into(), crate::tree::f32_value(r.f32()?));
    h.insert("Experience for next level".into(), crate::tree::f32_value(r.f32()?));
    h.insert("Save number".into(), Value::from(save_number));
    r.bytes(8)?; // file time
    let width = r.u32()? as usize;
    let height = r.u32()? as usize;
    let se = game == Game::Skyrim && version >= 12;
    let compression = if se {
        match r.u16()? {
            0 => Compression::None,
            1 => Compression::Zlib,
            2 => Compression::Lz4,
            _ => return Err("unknown compression".into()),
        }
    } else {
        Compression::None
    };
    if r.i != header_end {
        return Err("unexpected save header layout".into());
    }
    let bpp = if game == Game::Skyrim && version < 12 { 3 } else { 4 };
    let prefix = header_end + width * height * bpp;
    if prefix > d.len() {
        return Err("truncated save".into());
    }
    Ok(Container { game, prefix, compression, header: h, level_at })
}

fn decompress(d: &[u8], c: &Container) -> Result<Vec<u8>, String> {
    let rest = &d[c.prefix..];
    match c.compression {
        Compression::None => Ok(rest.to_vec()),
        _ => {
            let mut r = R { d: rest, i: 0 };
            let len = r.u32()? as usize;
            let clen = r.u32()? as usize;
            let body = rest.get(8..8 + clen).ok_or("truncated save")?;
            if 8 + clen != rest.len() {
                return Err("unexpected data after the compressed save".into());
            }
            let out = match c.compression {
                Compression::Lz4 => lz4_flex::block::decompress(body, len).map_err(|e| e.to_string())?,
                _ => {
                    let mut out = Vec::new();
                    flate2::read::ZlibDecoder::new(body).read_to_end(&mut out).map_err(|e| e.to_string())?;
                    out
                }
            };
            if out.len() != len {
                return Err("decompressed size doesn't match".into());
            }
            Ok(out)
        }
    }
}

fn recompress(d: &[u8], c: &Container, body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = d[..c.prefix].to_vec();
    match c.compression {
        Compression::None => out.extend_from_slice(body),
        _ => {
            let packed = match c.compression {
                Compression::Lz4 => lz4_flex::block::compress(body),
                _ => {
                    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                    e.write_all(body).map_err(|e| e.to_string())?;
                    e.finish().map_err(|e| e.to_string())?
                }
            };
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            out.extend_from_slice(&packed);
        }
    }
    Ok(out)
}

#[derive(Clone, Debug)]
enum Field {
    I32(usize),
    F32(usize),
    /// The player's level: in the player's base record (`body`, 16-bit) and the header (`raw`).
    Level { body_at: usize, header_at: usize },
}

#[derive(Clone, Debug)]
struct BethBackend {
    raw: Vec<u8>,
    body: Vec<u8>,
    fields: Vec<Field>,
}

impl Backend for BethBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        if changes.is_empty() {
            return Ok(self.raw.clone());
        }
        let mut body = self.body.clone();
        let mut raw = self.raw.clone();
        for (leaf, v) in changes {
            let (at, bytes) = match self.fields[*leaf] {
                Field::I32(at) => (at, (crate::tree::int_in(v, i32::MIN as i128, i32::MAX as i128, "This value")? as i32).to_le_bytes()),
                Field::F32(at) => (at, (v.as_f64().ok_or("expected a number")? as f32).to_le_bytes()),
                Field::Level { body_at, header_at } => {
                    let n = crate::tree::int_in(v, 1, 32767, "The level")? as u16;
                    body[body_at..body_at + 2].copy_from_slice(&n.to_le_bytes());
                    raw[header_at..header_at + 4].copy_from_slice(&u32::from(n).to_le_bytes());
                    continue;
                }
            };
            body[at..at + 4].copy_from_slice(&bytes);
        }
        let c = container(&raw)?;
        recompress(&raw, &c, &body)
    }

    fn normalize(&self, leaf: usize, v: Value) -> Value {
        if matches!(self.fields[leaf], Field::F32(_)) {
            crate::tree::round_f32(v)
        } else {
            v
        }
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match self.fields[leaf] {
            Field::Level { .. } => crate::tree::int_in(v, 1, 32767, "The level").map(|_| ()),
            Field::I32(_) => crate::tree::int_in(v, i32::MIN as i128, i32::MAX as i128, "This value").map(|_| ()),
            Field::F32(_) => v.as_f64().filter(|f| f.is_finite() && f.abs() < 3e38).map(|_| ()).ok_or_else(|| "expected a number".into()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// Position of the file location table in the game data, by trying the possible
/// plugin-list layouts and checking the table's own consistency.
fn location_table(body: &[u8], game: Game) -> Result<usize, String> {
    let mut r = R { d: body, i: 0 };
    let _form_version = r.u8()?;
    if game == Game::Fallout4 {
        r.wstring()?; // game version
    }
    let plugin_size = r.u32()? as usize;
    r.bytes(plugin_size)?;
    let after_plugins = r.i;
    // With or without the light-plugin (ESL) list.
    let mut candidates = vec![after_plugins];
    if let Ok(n) = (R { d: body, i: after_plugins }).u16() {
        let mut r2 = R { d: body, i: after_plugins + 2 };
        if (0..n).all(|_| r2.wstring().is_ok()) {
            candidates.push(r2.i);
        }
    }
    for p in candidates {
        let mut t = R { d: body, i: p };
        let offs: Vec<u32> = (0..6).filter_map(|_| t.u32().ok()).collect();
        if offs.len() < 6 {
            continue;
        }
        // globalDataTable1 < globalDataTable2 < changeForms < globalDataTable3, and the first
        // global data entry right after the table is type 0 (Misc Stats).
        let first = p + 25 * 4;
        let ordered = offs[2] < offs[3] && offs[3] <= offs[4] && offs[4] <= offs[5];
        if ordered && body.get(first..first + 4) == Some(&[0, 0, 0, 0][..]) {
            return Ok(p);
        }
    }
    Err("couldn't find the save's data tables".into())
}

fn global_name(form: u32) -> Option<&'static str> {
    Some(match form {
        0x35 => "Game year",
        0x36 => "Game month",
        0x37 => "Game day",
        0x38 => "Game hour",
        0x39 => "Days passed",
        0x3A => "Time scale",
        _ => return None,
    })
}

/// Where the player's level is in the game data: the base record NPC_ 0x000007's change
/// form, when it holds actor base data (ACBS, level at +8) and isn't compressed. Order of
/// a change form's data follows UESP's table (form flags first, then ACBS).
fn player_level(body: &[u8], table: usize) -> Option<usize> {
    let u = |o: usize| body.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let base = (u(table + 8)? as usize).checked_sub(table + 100)?;
    let at = (u(table + 16)? as usize).checked_sub(base)?;
    let n = u(table + 36)? as usize;
    let mut o = at;
    for _ in 0..n {
        let id = (u32::from(*body.get(o)?) << 16) | (u32::from(body[o + 1]) << 8) | u32::from(body[o + 2]);
        let flags = u(o + 3)?;
        let ty = *body.get(o + 7)?;
        let (l1, l2, h) = match ty >> 6 {
            0 => (usize::from(*body.get(o + 9)?), usize::from(*body.get(o + 10)?), 11),
            1 => (usize::from(u16::from_le_bytes([*body.get(o + 9)?, *body.get(o + 10)?])), usize::from(u16::from_le_bytes([*body.get(o + 11)?, *body.get(o + 12)?])), 13),
            _ => (u(o + 9)? as usize, u(o + 13)? as usize, 17),
        };
        if id == 0x40_0007 && ty & 0x3F == 9 {
            // ACBS present, data stored as is.
            if flags & 0x2 == 0 || l2 != 0 {
                return None;
            }
            let acbs = o + h + if flags & 0x1 != 0 { 6 } else { 0 };
            return (acbs + 24 <= o + h + l1).then_some(acbs + 8);
        }
        o += h + l1;
        if o > body.len() {
            return None;
        }
    }
    None
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"TESV_SAVEGAME") || d.starts_with(b"FO4_SAVEGAME")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let c = container(d)?;
    let body = decompress(d, &c)?;
    let table = location_table(&body, c.game)?;
    let mut t = R { d: &body, i: table + 24 };
    let count1 = t.u32()? as usize;
    let mut r = R { d: &body, i: table + 25 * 4 };
    let mut fields = Vec::new();
    let mut editable = HashMap::new();
    let mut stats = Map::new();
    let mut globals = Map::new();
    let mut time = Map::new();
    for _ in 0..count1.min(16) {
        let kind = r.u32()?;
        let len = r.u32()? as usize;
        let start = r.i;
        let data_end = start + len;
        match kind {
            0 => {
                let mut s = R { d: &body[..data_end], i: start };
                let n = s.u32()?;
                for _ in 0..n {
                    let name = s.wstring()?;
                    s.u8()?; // category
                    let at = s.i;
                    let v = s.u32()? as i32;
                    let key = crate::tree::unique_key(&stats, &name);
                    editable.insert(format!("/Misc stats/{}", crate::tree::esc(&key)), fields.len());
                    fields.push(Field::I32(at));
                    stats.insert(key, Value::from(v));
                }
            }
            3 => {
                let mut s = R { d: &body[..data_end], i: start };
                let n = s.vsval()?;
                for _ in 0..n {
                    let id = s.bytes(3)?;
                    let (kind, value) = (id[0] >> 6, ((id[0] as u32 & 0x3F) << 16) | (id[1] as u32) << 8 | id[2] as u32);
                    let at = s.i;
                    let v = s.f32()?;
                    let name = match (kind, global_name(value)) {
                        (1, Some(n)) => n.to_string(),
                        (1, None) => format!("Global {value:06X}"),
                        _ => format!("Global {value:06X} (plugin)"),
                    };
                    // Named ones (game time) get their own section; the rest are advanced.
                    let (section, map) = if kind == 1 && global_name(value).is_some() { ("Game time", &mut time) } else { ("Global variables", &mut globals) };
                    let key = crate::tree::unique_key(map, &name);
                    editable.insert(format!("/{section}/{}", crate::tree::esc(&key)), fields.len());
                    fields.push(Field::F32(at));
                    let shown = crate::tree::f32_value(v);
                    map.insert(key, shown);
                }
            }
            _ => {}
        }
        r.i = data_end;
    }
    let level_note = match (c.game, player_level(&body, table)) {
        (Game::Skyrim, Some(at)) => {
            editable.insert("/Save/Level".into(), fields.len());
            fields.push(Field::Level { body_at: at, header_at: c.level_at });
            true
        }
        _ => false,
    };
    let mut root = Map::new();
    root.insert("Save".into(), Value::Object(c.header.clone()));
    root.insert("Game time".into(), Value::Object(time));
    root.insert("Misc stats".into(), Value::Object(stats));
    root.insert("Global variables".into(), Value::Object(globals));
    let name = match c.game {
        Game::Skyrim => "Skyrim save",
        Game::Fallout4 => "Fallout 4 save",
    };
    let backend = BethBackend { raw: d.to_vec(), body, fields };
    let doc = TreeDoc::new(Value::Object(root), editable, name, Box::new(backend)).with_advanced(vec!["Global variables".into()]);
    let level = if level_note { " The level is changed in the character itself (the game recalculates the experience needed)." } else { "" };
    Ok(doc.with_note(format!(
        "Misc stats and global variables (game time, day counters\u{2026}) can be edited.{level} Items, gold and skills are stored in \
         a part of the save whose format isn't documented well enough to change safely \u{2014} use the game's console for those."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn real_bethesda_saves() {
        let docs = std::env::var("BETHESDA_SAVES").expect("BETHESDA_SAVES");
        for game in ["Enderal Special Edition", "Fallout4VR", "Skyrim Special Edition", "Skyrim", "Fallout4"] {
            let Ok(dir) = std::fs::read_dir(format!("{docs}\\{game}\\Saves")) else { continue };
            for e in dir.flatten().take(3) {
                let p = e.path();
                if !p.extension().is_some_and(|x| x == "ess" || x == "fos") {
                    continue;
                }
                let d = std::fs::read(&p).unwrap();
                let mut doc = parse(&d).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                println!("{}: {} | {} stats, time {}", p.display(), doc.view["Save"], doc.view["Misc stats"].as_object().unwrap().len(), doc.view["Game time"]);
                assert_eq!(doc.to_bytes().unwrap(), d);
                let key = doc.view["Misc stats"].as_object().unwrap().keys().next().unwrap().clone();
                doc.set(&format!("/Misc stats/{}", crate::tree::esc(&key)), Value::from(12345)).unwrap();
                doc.set("/Game time/Days passed", Value::from(100.5)).unwrap();
                let out = doc.to_bytes().unwrap();
                let again = parse(&out).unwrap();
                assert_eq!(again.view, doc.view);
            }
        }
    }

    /// Research aid: dumps the player's change forms (base NPC_ 0x000007, reference 0x000014).
    #[test]
    #[ignore]
    fn player_change_forms() {
        let dir = std::env::var("BETHESDA_COPIES").expect("BETHESDA_COPIES");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            if std::env::var("ONLY").is_ok_and(|x| !e.file_name().to_string_lossy().ends_with(&x)) {
                continue;
            }
            let d = std::fs::read(e.path()).unwrap();
            let c = container(&d).unwrap();
            let body = decompress(&d, &c).unwrap();
            let table = location_table(&body, c.game).unwrap();
            let u = |o: usize| u32::from_le_bytes(body[o..o + 4].try_into().unwrap());
            // The table's offsets count from somewhere before the body; the first global data
            // table starts right after the location table.
            let base = u(table + 8) as usize - (table + 100);
            let (cf_at, cf_n) = (u(table + 16) as usize - base, u(table + 36) as usize);
            println!("{} level {} xp {} | change forms at {cf_at:#x} x{cf_n}", e.file_name().to_string_lossy(), c.header["Level"], c.header["Experience"]);
            let mut o = cf_at;
            for _ in 0..cf_n {
                let id = ((body[o] as u32) << 16) | (body[o + 1] as u32) << 8 | body[o + 2] as u32;
                let flags = u(o + 3);
                let ty = body[o + 7];
                let ver = body[o + 8];
                let (l1, l2, h) = match ty >> 6 {
                    0 => (body[o + 9] as usize, body[o + 10] as usize, 11),
                    1 => (u16::from_le_bytes([body[o + 9], body[o + 10]]) as usize, u16::from_le_bytes([body[o + 11], body[o + 12]]) as usize, 13),
                    _ => (u(o + 9) as usize, u(o + 13) as usize, 17),
                };
                if id == 0x40_0007 || id == 0x40_0014 {
                    let raw = &body[o + h..o + h + l1];
                    let data = if l2 > 0 {
                        let mut out = Vec::new();
                        flate2::read::ZlibDecoder::new(raw).read_to_end(&mut out).unwrap();
                        out
                    } else {
                        raw.to_vec()
                    };
                    let hex: String = data.iter().take(96).map(|b| format!("{b:02x} ")).collect();
                    println!("  id {id:06x} type {} ver {ver} flags {flags:#010x} len {l1}/{l2}: {hex}", ty & 0x3F);
                }
                o += h + l1;
            }
        }
    }
}
