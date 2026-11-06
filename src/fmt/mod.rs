//! Structured save formats. Each module turns bytes into an editable `TreeDoc`.

pub mod bethesda;
pub mod bsonf;
pub mod civ6;
pub mod containers;
pub mod d2s;
pub mod crypto;
pub mod fromsoft;
pub mod sekiro_inv;
pub mod er_inv;
pub mod ds3_inv;

/// Names of every good a FromSoftware game's item lists know (for tests).
#[cfg(test)]
pub fn known_goods(game: &str) -> std::collections::HashSet<&'static str> {
    let t: &[(u32, &str)] = match game {
        "Elden Ring" => er_items::GOODS,
        "Dark Souls III" => ds3_items::GOODS,
        "Dark Souls II" => ds2_items::GOODS,
        "Dark Souls" => ds1_items::GOODS,
        _ => &[],
    };
    t.iter().map(|g| g.1).collect()
}
pub mod ds2_inv;
pub mod ds1_inv;
mod ds2_items;
mod ds1_items;
mod ds3_items;
mod er_equip;
mod er_flags;
pub mod slot_copy;
pub mod mandarin;
pub mod pdx;
mod rimworld;
mod oni;
pub mod oni_map;
pub mod owlcat;
mod ree;
mod ree_names;
mod er_items;
mod sekiro_items;
pub mod gvas;
pub mod godot;
pub mod ini;
pub mod lua;
pub mod luabins;
pub mod marshal;
pub mod msgpack;
pub mod nbt;
pub mod nrbf;
pub mod fields;
pub mod masseffect;
pub mod me1;
pub mod me1le;
pub mod xcom2;
pub mod xcom_eu;
pub mod ff7;
pub mod ff7r;
pub mod ff9;
pub mod sgd;
pub mod dl1;
pub mod witcher3;
pub mod cyberpunk;
pub mod kenshi;
pub mod p4g;
pub mod ffx2;
pub mod p5r;
pub mod metro;
pub mod bioshock;
pub mod rgmh;
pub mod cairn;
pub mod rgg;
pub mod yakuza0;
pub mod kiwami;
pub mod gow;
pub mod plistf;
pub mod pokemon12;
pub mod pokemon3;
pub mod sqlite;
pub mod terraria;
pub mod terraria_world;
pub mod tomlf;
pub mod valheim;
pub mod wasteland3;
pub mod xml;
pub mod yaml;

use crate::tree::TreeDoc;

/// Binary formats with a recognisable signature (checked before text detection).
pub fn parse_binary(d: &[u8]) -> Option<TreeDoc> {
    if d.starts_with(b"SQLite format 3\0") {
        return sqlite::parse(d).ok();
    }
    if d.starts_with(b"bplist") {
        return plistf::parse(d).ok();
    }
    if pokemon12::looks(d) {
        if let Ok(t) = pokemon12::parse(d) {
            return Some(t);
        }
    }
    if pokemon3::looks(d) {
        if let Ok(t) = pokemon3::parse(d) {
            return Some(t);
        }
    }
    if bethesda::looks(d) {
        if let Ok(t) = bethesda::parse(d) {
            return Some(t);
        }
    }
    if gow::looks_2018(d) {
        if let Ok(t) = gow::parse_2018(d) {
            return Some(t);
        }
    }
    if gow::looks_ragnarok(d) {
        if let Ok(t) = gow::parse_ragnarok(d) {
            return Some(t);
        }
    }
    if kiwami::looks(d) {
        if let Ok(t) = kiwami::parse(d) {
            return Some(t);
        }
    }
    if yakuza0::looks(d) {
        if let Ok(t) = yakuza0::parse(d) {
            return Some(t);
        }
    }
    if metro::looks(d) {
        if let Ok(t) = metro::parse(d) {
            return Some(t);
        }
    }
    if p5r::looks(d) {
        if let Ok(t) = p5r::parse(d) {
            return Some(t);
        }
    }
    if ffx2::looks_ffx(d) {
        if let Ok(t) = ffx2::parse_ffx(d) {
            return Some(t);
        }
    }
    if ffx2::looks(d) {
        if let Ok(t) = ffx2::parse(d) {
            return Some(t);
        }
    }
    if p4g::looks(d) {
        if let Ok(t) = p4g::parse(d) {
            return Some(t);
        }
    }
    if kenshi::looks(d) {
        if let Ok(t) = kenshi::parse(d) {
            return Some(t);
        }
    }
    if witcher3::looks(d) {
        if let Ok(t) = witcher3::parse(d) {
            return Some(t);
        }
    }
    if dl1::looks(d) {
        if let Ok(t) = dl1::parse(d) {
            return Some(t);
        }
    }
    if sgd::looks(d) {
        if let Ok(t) = sgd::parse(d) {
            return Some(t);
        }
    }
    if ff9::looks(d) {
        if let Ok(t) = ff9::parse(d) {
            return Some(t);
        }
    }
    if ff7r::looks(d) {
        if let Ok(t) = ff7r::parse(d) {
            return Some(t);
        }
    }
    if ff7::looks(d) {
        if let Ok(t) = ff7::parse(d) {
            return Some(t);
        }
    }
    if d2s::looks(d) {
        if let Ok(t) = d2s::parse(d) {
            return Some(t);
        }
    }
    if civ6::looks(d) {
        if let Ok(t) = civ6::parse(d) {
            return Some(t);
        }
    }
    if xcom2::looks(d) {
        if let Ok(t) = xcom2::parse(d) {
            return Some(t);
        }
    }
    if xcom_eu::looks(d) {
        if let Ok(t) = xcom_eu::parse(d) {
            return Some(t);
        }
    }
    if me1::looks(d) {
        if let Ok(t) = me1::parse(d) {
            return Some(t);
        }
    }
    if me1le::looks(d) {
        if let Ok(t) = me1le::parse(d) {
            return Some(t);
        }
    }
    if masseffect::looks(d) {
        if let Ok(t) = masseffect::parse(d) {
            return Some(t);
        }
    }
    if fromsoft::detect(d).is_some() {
        if let Ok(t) = fromsoft::parse(d) {
            return Some(t);
        }
    }
    if cairn::looks(d) {
        if let Ok(t) = cairn::parse(d) {
            return Some(t);
        }
    }
    if terraria::looks(d) {
        if let Ok(t) = terraria::parse(d) {
            return Some(t);
        }
    }
    if terraria_world::looks(d) {
        if let Ok(t) = terraria_world::parse(d) {
            return Some(t);
        }
    }
    if oni::looks(d) {
        if let Ok(t) = oni::parse(d) {
            return Some(t);
        }
    }
    if wasteland3::looks(d) {
        if let Ok(t) = wasteland3::parse(d) {
            return Some(t);
        }
    }
    if valheim::looks(d) {
        if let Ok(t) = valheim::parse(d) {
            return Some(t);
        }
    }
    if rimworld::looks(d) {
        if let Ok(t) = rimworld::parse(d) {
            return Some(t);
        }
    }
    if pdx::looks(d) {
        if let Ok(t) = pdx::parse(d) {
            return Some(t);
        }
    }
    if d.starts_with(b"GVAS") {
        return gvas::parse(d).ok();
    }
    if ree::looks(d) {
        if let Ok(t) = ree::parse(d) {
            return Some(t);
        }
    }
    if d.starts_with(&[4, 8]) {
        if let Ok(t) = marshal::parse(d) {
            return Some(t);
        }
    }
    if nrbf::looks(d) {
        if let Ok(t) = nrbf::parse(d) {
            return Some(t);
        }
    }
    if let Ok(t) = nbt::parse(d) {
        return Some(t);
    }
    if bsonf::looks(d) {
        if let Ok(t) = bsonf::parse(d) {
            return Some(t);
        }
    }
    None
}

/// Binary formats without a signature: only tried on data that isn't text, and each
/// one must decode the whole input exactly.
pub fn parse_binary_late(d: &[u8]) -> Option<TreeDoc> {
    if d.len() > 2 && (1..=16).contains(&d[0]) && b"T-01NS".contains(&d[1]) {
        if let Ok(t) = luabins::parse(d) {
            return Some(t);
        }
    }
    godot::parse(d).ok().or_else(|| msgpack::parse(d).ok())
}

/// Text formats. Ambiguous ones (YAML, TOML) are only trusted by file extension.
pub fn parse_text(text: &str, ext: &str) -> Option<TreeDoc> {
    let t = text.trim_start();
    if lua::looks(t) {
        if let Ok(doc) = lua::parse(text) {
            return Some(doc);
        }
    }
    if t.starts_with('<') {
        if t.contains("<plist") {
            if let Ok(doc) = plistf::parse(text.as_bytes()) {
                return Some(doc);
            }
        }
        return xml::parse(text).ok();
    }
    match ext {
        "yaml" | "yml" => return yaml::parse(text).ok(),
        "toml" => return tomlf::parse(text).ok(),
        _ => {}
    }
    ini::parse(text, ext).ok()
}

/// Windows-1252 byte to character (older Paradox and Bethesda text).
pub(crate) fn bethesda_cp1252(b: u8) -> char {
    bethesda::cp1252(b)
}

/// Character to Windows-1252 byte, if it has one.
pub(crate) fn bethesda_cp1252_encode(c: char) -> Option<u8> {
    if (c as u32) < 0x80 || (0xA0..=0xFF).contains(&(c as u32)) {
        return Some(c as u32 as u8);
    }
    (0x80..=0x9Fu8).find(|&b| bethesda::cp1252(b) == c)
}

/// CRC-32/BZIP2 (MSB-first, polynomial 0x04C11DB7), as XCOM uses.
pub(crate) fn crc32_bzip2(d: &[u8]) -> u32 {
    static T: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = T.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = (i as u32) << 24;
            for _ in 0..8 {
                c = if c & 0x8000_0000 != 0 { (c << 1) ^ 0x04C1_1DB7 } else { c << 1 };
            }
            *e = c;
        }
        t
    });
    !d.iter().fold(0xFFFF_FFFFu32, |c, &b| (c << 8) ^ t[((c >> 24) as u8 ^ b) as usize])
}
