//! Metro (4A Engine: 2033 Redux, Last Light Redux) saves, worked out from sample saves.
//!
//! Containers (every file ends in a CRC-32 of everything before it):
//! - `*.player` (the player at the start of each level, used by chapter select): "artm",
//!   the build string (NUL-terminated), the unpacked size (Exodus: a build id, the size, a
//!   hash), the packed size (+8), the unpacked size again, then one LZ4 block.
//! - `*_auto_save`: "DEZP" (Exodus: a build id and two more fields), three sections
//!   (unpacked size, section id, packed size), then
//!   each section as blocks of (block size including these 8 bytes, unpacked size, LZ4).
//!   Blocks may refer back into earlier ones (64 KB window); the editor writes them
//!   independently, which any LZ4 reader accepts.
//!
//! Game data: inventory items are stored by name (`ammo_<kind>_NNNN`); an ammo item's
//! counts follow its class id (see `CLASSES`): the magazine size twice, then the rounds
//! carried.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

fn u32_at(d: &[u8], o: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
}

fn crc_ok(d: &[u8]) -> bool {
    d.len() > 8 && {
        let mut h = flate2::Crc::new();
        h.update(&d[..d.len() - 4]);
        h.sum().to_le_bytes() == d[d.len() - 4..]
    }
}

fn with_crc(mut out: Vec<u8>) -> Vec<u8> {
    let mut h = flate2::Crc::new();
    h.update(&out);
    out.extend(h.sum().to_le_bytes());
    out
}

/// `.player`: (head = everything before the packed size, data).
pub fn unpack_player(d: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if !d.starts_with(b"artm") || !crc_ok(d) {
        return None;
    }
    let h = 4 + d.get(4..64)?.iter().position(|&b| b == 0)? + 1;
    // The packed size (counting the unpacked size after it and the CRC) is followed by the
    // unpacked size and the LZ4 block; Exodus has a few more fields before it.
    let at = (h..h + 32).find(|&p| u32_at(d, p) == Some(d.len().wrapping_sub(p + 4)))?;
    let raw = u32_at(d, at + 4)?;
    let data = lz4_flex::block::decompress(d.get(at + 8..d.len() - 4)?, raw).ok()?;
    (data.len() == raw).then(|| (d[..at].to_vec(), data))
}

pub fn pack_player(head: &[u8], data: &[u8]) -> Vec<u8> {
    let z = lz4_flex::block::compress(data);
    let mut out = head.to_vec();
    out.extend(((z.len() + 8) as u32).to_le_bytes());
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(z);
    with_crc(out)
}

/// The auto save's layout: what comes before the section table (Exodus has a build id
/// and two more fields there), and for each section its id and its blocks' unpacked sizes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dezp {
    pub prefix: Vec<u8>,
    pub sections: Vec<(u32, Vec<usize>)>,
}

pub fn unpack_dezp(d: &[u8]) -> Option<(Dezp, Vec<u8>)> {
    if !d.starts_with(b"DEZP") || !crc_ok(d) {
        return None;
    }
    [4, 0x14].into_iter().find_map(|t| unpack_dezp_at(d, t))
}

fn unpack_dezp_at(d: &[u8], table: usize) -> Option<(Dezp, Vec<u8>)> {
    let mut out: Vec<u8> = Vec::new();
    let mut sections = Vec::new();
    let mut p: usize = table + 36;
    for s in 0..3 {
        let e = table + 12 * s;
        let (raw, id, packed) = (u32_at(d, e)?, u32_at(d, e + 4)? as u32, u32_at(d, e + 8)?);
        let end = p.checked_add(packed)?;
        if end > d.len() - 4 {
            return None;
        }
        let start = out.len();
        let mut blocks = vec![];
        while p < end {
            let (n, r) = (u32_at(d, p)?, u32_at(d, p + 4)?);
            if n < 8 || p + n > end {
                return None;
            }
            let dict = &out[out.len().saturating_sub(65536)..];
            let b = lz4_flex::block::decompress_with_dict(&d[p + 8..p + n], r, dict).ok()?;
            if b.len() != r {
                return None;
            }
            out.extend(b);
            blocks.push(r);
            p += n;
        }
        if out.len() - start != raw {
            return None;
        }
        sections.push((id, blocks));
    }
    (p == d.len() - 4).then(|| (Dezp { prefix: d[4..table].to_vec(), sections }, out))
}

pub fn pack_dezp(z: &Dezp, data: &[u8]) -> Result<Vec<u8>, String> {
    let total: usize = z.sections.iter().flat_map(|(_, b)| b).sum();
    if total != data.len() {
        return Err("this save's data can't change size".into());
    }
    let mut table = b"DEZP".to_vec();
    table.extend(&z.prefix);
    let mut body = vec![];
    let mut q = 0;
    for (id, blocks) in &z.sections {
        let raw: usize = blocks.iter().sum();
        let start = body.len();
        for &r in blocks {
            let c = lz4_flex::block::compress(&data[q..q + r]);
            body.extend(((c.len() + 8) as u32).to_le_bytes());
            body.extend((r as u32).to_le_bytes());
            body.extend(c);
            q += r;
        }
        table.extend((raw as u32).to_le_bytes());
        table.extend(id.to_le_bytes());
        table.extend(((body.len() - start) as u32).to_le_bytes());
    }
    table.extend(body);
    Ok(with_crc(table))
}

// ------------------------------------------------------------------ the player's inventory

/// The ammo classes (money has its own): the class id, one byte, eight zero bytes, then
/// the magazine size twice and the rounds carried.
const CLASSES: [[u8; 4]; 3] = [[0x68, 0x8f, 0x25, 0x1d], [0x34, 0x5b, 0xc0, 0xf4], [0x2c, 0x26, 0x8f, 0xd6]];
#[cfg(test)]
const AMMO: [u8; 13] = [0x68, 0x8f, 0x25, 0x1d, 1, 0, 0, 0, 0, 0, 0, 0, 0];

fn ammo_at(w: &[u8]) -> bool {
    w.len() >= 13 && CLASSES.iter().any(|c| w[..4] == *c) && w[5..13].iter().all(|&b| b == 0)
}

fn label(kind: &str) -> String {
    match kind {
        "money" => "Military-grade rounds (money)".into(),
        "545x39_fmj" => "5.45 mm rounds".into(),
        "762x39_fmj" => "7.62 mm rounds".into(),
        "762x39_mg" => "7.62 mm machine-gun rounds".into(),
        "044_fmj" => ".44 rounds".into(),
        "044_ap" => ".44 armour-piercing rounds".into(),
        "12x70mm" => "Shotgun shells".into(),
        "12x70mm_fire" => "Incendiary shotgun shells".into(),
        "12x70mm_flamethrower" => "Flamethrower shells".into(),
        "15mm" => "15 mm (Tikhar) balls".into(),
        "dshk" => "DShK rounds".into(),
        "flame_fmj" | "flamethrower_liquid" => "Flamethrower fuel".into(),
        "15mm_sabot" => "15 mm sabot balls".into(),
        "makeshift" => "Makeshift rounds".into(),
        "craft_metal" => "Metal (crafting)".into(),
        "craft_chemical" => "Chemicals (crafting)".into(),
        "craft_consumable" => "Consumables (crafting)".into(),
        k => k.replace('_', " "),
    }
}

/// The length of an item name (`ammo_…_NNNN` or `craft_…_NNNN`, NUL-terminated) at `at`.
fn item_name(d: &[u8], at: usize) -> Option<usize> {
    let w = d.get(at..(at + 64).min(d.len()))?;
    if !(w.starts_with(b"ammo_") || w.starts_with(b"craft_")) {
        return None;
    }
    let len = w.iter().position(|&c| c == 0)?;
    let n = &w[..len];
    let ok = len > 10
        && n[len - 5] == b'_'
        && n[len - 4..].iter().all(u8::is_ascii_digit)
        && n.iter().all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_');
    ok.then_some(len)
}

pub fn looks(d: &[u8]) -> bool {
    // Level saves only (about 200 KB): an auto save (1 MB and up) also lists the items lying
    // around the level, which aren't the player's.
    d.len() > 1000 && d.len() < 600_000 && matches!(d.get(4..9), Some(b"2033\\") | Some(b"2034\\")) | matches!(d.get(4..7), Some(b"m3\\")) && d.windows(5).any(|w| w == b"ammo_")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not Metro player data".into());
    }
    let mut b = Builder::new(d);
    let mut any = false;
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut p = 0;
    let x = usize::from(d.get(4..7) == Some(b"m3\\"));
    while let Some(i) = d[p..].windows(5).position(|w| w == b"ammo_" || w == b"craft").map(|i| i + p) {
        p = i + 5;
        let Some(len) = item_name(d, i) else { continue };
        let name = String::from_utf8_lossy(&d[i..i + len]).into_owned();
        let rest = name.strip_prefix("ammo_").unwrap_or(&name);
        let (kind, _) = rest.rsplit_once('_').unwrap_or((rest, ""));
        let from = i + len;
        // Only up to the next item's name: an item without counts mustn't take the next one's.
        let mut stop = (from + 400).min(d.len());
        if let Some(n) = (from..stop).find(|&q| item_name(d, q).is_some()) {
            stop = n;
        }
        let Some(k) = d[from..stop].windows(13).position(ammo_at).map(|k| k + from) else { continue };
        // The magazine size twice (Exodus: one more byte before it, and 0 twice).
        let k = k + x;
        if d.len() < k + 19 || d[k + 13..k + 15] != d[k + 15..k + 17] {
            continue;
        }
        let n = seen.entry(kind.to_string()).or_default();
        *n += 1;
        let key = if *n == 1 { label(kind) } else { format!("{} ({})", label(kind), n) };
        b.ranged(&["Inventory", &key], k + 17, Kind::U16, 0, 65_535);
        any = true;
        p = k + 19;
    }
    if !any {
        return Err("no inventory found".into());
    }
    Ok(b.finish("Metro player save", Checksum::None).with_note(
        "Military-grade rounds (the metro's money) and the ammunition Artyom carries at the start of this level. \
         The level's own save (the auto save) keeps the current state.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_round_trip() {
        let mut data = vec![1, 2, 3, 4];
        data.extend(b"2034\\l00\0ammo_money_0001\0");
        data.extend(AMMO);
        data.extend([5, 0, 5, 0, 0x10, 0x27]);
        data.resize(2000, 0);
        let file = pack_player(b"artmPC_2013_05_14\0", &data);
        let (head, back) = unpack_player(&file).unwrap();
        assert_eq!(back, data);
        assert_eq!(pack_player(&head, &back), file);
        let mut doc = parse(&back).unwrap();
        assert_eq!(doc.view.pointer("/Inventory/Military-grade rounds (money)").unwrap(), 10000);
        doc.set("/Inventory/Military-grade rounds (money)", serde_json::Value::from(500)).unwrap();
        let at = data.windows(AMMO.len()).position(|w| w == AMMO).unwrap() + 17;
        assert_eq!(doc.to_bytes().unwrap()[at..at + 2], [0xF4, 1]);
    }

    #[test]
    fn dezp_round_trip() {
        let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        for prefix in [vec![], vec![1u8; 16]] {
            let z = Dezp { prefix, sections: vec![(7, vec![100_000]), (8, vec![100_000, 50_000]), (9, vec![50_000])] };
            let file = pack_dezp(&z, &data).unwrap();
            let (z2, back) = unpack_dezp(&file).unwrap();
            assert_eq!(back, data);
            assert_eq!(z2, z);
        }
    }

    /// Research aid: SAVE_EDITOR_METRO = a .player or auto save.
    #[test]
    #[ignore]
    fn real_file() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_METRO").unwrap()).unwrap();
        if let Some((h, data)) = unpack_player(&d) {
            println!("player: {} bytes", data.len());
            println!("{}", serde_json::to_string_pretty(&parse(&data).unwrap().view).unwrap());
            assert_eq!(unpack_player(&pack_player(&h, &data)).unwrap().1, data);
        } else {
            let (s, data) = unpack_dezp(&d).expect("unpack");
            println!("auto save: {} bytes, {:?}", data.len(), s);
            assert_eq!(unpack_dezp(&pack_dezp(&s, &data).unwrap()).unwrap().1, data);
        }
    }
}
