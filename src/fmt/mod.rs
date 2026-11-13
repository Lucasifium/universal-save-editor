//! Structured save formats. Each module turns bytes into an editable `TreeDoc`.

pub mod bethesda;
pub mod borderlands;
pub mod borderlands3;
pub mod sii;
pub mod totalwar;
pub mod mgsv;
pub mod bsonf;
pub mod civ6;
pub mod containers;
pub mod d2s;
pub mod grimdawn;
pub mod pssave;
pub mod dreamcast;
pub mod saturn;
pub mod keyed;
pub mod ksp;
pub mod adf;
pub mod forest;
pub mod forest_items;
pub mod mechanicus;
pub mod planet_crafter;
pub mod prison;
pub mod tld;
pub mod nfsmw;
pub mod es2;
pub mod re5;
pub mod anno;
pub mod fable;
pub mod slime;
pub mod hzd;
pub mod sleeping_dogs;
pub mod dxmd;
pub mod gk;
pub mod civ7;
pub mod uepack;
pub mod stoneshard;
pub mod mcd;
pub mod ffpr;
pub mod fantasian;
pub mod kh3;
mod kh3_tables;
pub mod prey;
pub mod rf4s;
pub mod sky1st;
pub mod titanquest;
pub mod torchlight2;
pub mod dragonage;
pub mod gff4;
pub mod infinity;
pub mod darkest;
pub mod larian;
pub mod lsf;
pub mod lsmf;
pub mod lspk;
pub mod lzari;
pub mod crypto;
pub mod es3;
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
pub mod citrus;
pub mod lime;
pub mod mandarin;
pub mod regfile;
pub mod sol;
pub mod haxe;
pub mod deadcells;
pub mod memcard;
pub mod sumfind;
pub mod template;
pub mod katana;
pub mod ttgames;
pub mod pdx;
pub mod pdxbin;
mod rimworld;
mod oni;
pub mod oni_map;
pub mod owlcat;
mod ree;
mod ree_names;
pub(crate) mod er_items;
mod sekiro_items;
pub mod gvas;
pub mod gff;
pub mod kotor;
pub mod nwn;
pub mod half_life;
pub mod octopath;
pub mod godot;
pub mod goldensun;
pub mod ini;
pub mod antimatter;
pub mod nested;
pub mod lua;
pub mod luabins;
pub mod marshal;
pub mod msgpack;
pub mod ac8;
pub mod bsinfinite;
pub mod cbor;
pub mod nbt;
pub mod nrbf;
pub mod fields;
pub mod masseffect;
pub mod me1;
pub mod me1le;
pub mod fallout12;
pub mod fos;
pub mod deusex;
pub mod xcom2;
pub mod sn2;
pub mod xcom_eu;
pub mod ff7;
pub mod ff7r;
mod ff7r_items;
pub mod ff9;
pub mod sgd;
pub mod dl1;
pub mod witcher3;
pub mod cyberpunk;
pub mod kenshi;
pub mod p3r;
pub mod p4g;
pub mod ao;
pub mod p3p;
mod p3p_tables;
pub mod ffx2;
pub mod dead_rising;
pub mod ff13;
pub mod p5r;
pub mod cave_story;
pub mod ori;
pub mod lego_tss;
pub mod lego_classic;
pub mod alphaprotocol;
pub mod obsidian;
pub mod roadhog;
pub mod saltsanc;
pub mod wasteland2;
pub mod sonic_mania;
pub mod metro;
pub mod bioshock;
pub mod rgmh;
pub mod cairn;
pub mod rgg;
pub mod yakuza0;
pub mod kiwami;
pub mod yakuza_old;
pub mod ishin;
pub mod ffxv;
pub mod forspoken;
pub mod nier;
pub mod dbpf;
pub mod pvz;
pub mod gta3;
pub mod jedi;
pub mod sr3;
pub mod sr4;
pub mod wd2;
pub mod tos;
pub mod sims1;
pub mod sims2;
pub mod sims4;
pub mod ds2;
pub mod ds1;
pub mod mea;
pub mod gow;
pub mod plistf;
pub mod pokemon12;
pub mod pokemon3;
#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;
pub mod starbound;
pub mod terraria;
pub mod terraria_world;
pub mod tomlf;
pub mod undertale;
pub mod vampire;
pub mod valheim;
pub mod wasteland3;
pub mod xml;
pub mod yaml;

use crate::tree::TreeDoc;

/// Binary formats with a recognisable signature (checked before text detection).
pub fn parse_binary(d: &[u8]) -> Option<TreeDoc> {
    if let Some(t) = civ7::parse(d) {
        return Some(t);
    }
    if let Some(t) = nfsmw::parse(d) {
        return Some(t);
    }
    if slime::looks(d) {
        if let Some(t) = slime::parse(d) {
            return Some(t);
        }
    }
    if dxmd::looks(d) {
        if let Some(t) = dxmd::parse(d) {
            return Some(t);
        }
    }
    if sleeping_dogs::looks(d) {
        if let Some(t) = sleeping_dogs::parse(d) {
            return Some(t);
        }
    }
    if hzd::looks(d) {
        if let Some(t) = hzd::parse(d) {
            return Some(t);
        }
    }
    if fable::looks(d) {
        if let Some(t) = fable::parse(d) {
            return Some(t);
        }
    }
    if anno::looks(d) {
        if let Some(t) = anno::parse(d) {
            return Some(t);
        }
    }
    if re5::looks(d) {
        if let Some(t) = re5::parse(d) {
            return Some(t);
        }
    }
    if es2::looks(d) {
        if let Some(t) = es2::parse(d) {
            return Some(t);
        }
    }
    // Checksummed JSON: before the plain JSON reader gets it.
    if vampire::looks(d) {
        if let Ok(t) = vampire::parse(d) {
            return Some(t);
        }
    }
    if d.starts_with(b"SQLite format 3\0") {
        #[cfg(not(target_arch = "wasm32"))]
        return sqlite::parse(d).ok();
        // The browser version has no SQLite: the file opens as hex.
        #[cfg(target_arch = "wasm32")]
        return None;
    }
    if starbound::looks(d) {
        if let Ok(t) = starbound::parse(d) {
            return Some(t);
        }
    }
    if d.starts_with(b"bplist") {
        return plistf::parse(d).ok();
    }
    if let Some(t) = parse_aurora(d, None) {
        return Some(t);
    }
    if pdxbin::looks(d) {
        if let Ok(t) = pdxbin::parse(d) {
            return Some(t);
        }
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
    if goldensun::looks(d) {
        if let Ok(t) = goldensun::parse(d) {
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
    if mea::looks(d) {
        if let Ok(t) = mea::parse(d) {
            return Some(t);
        }
    }
    if ds2::looks_data(d) {
        if let Ok(t) = ds2::parse(d) {
            return Some(t);
        }
    }
    if ds1::looks_data(d) {
        if let Ok(t) = ds1::parse(d) {
            return Some(t);
        }
    }
    if ffxv::looks(d) {
        if let Ok(t) = ffxv::parse(d) {
            return Some(t);
        }
    }
    if kiwami::looks(d) {
        if let Ok(t) = kiwami::parse(d) {
            return Some(t);
        }
    }
    if ishin::looks(d) {
        if let Ok(t) = ishin::parse(d) {
            return Some(t);
        }
    }
    if yakuza_old::looks(d) {
        if let Ok(t) = yakuza_old::parse(d) {
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
    if alphaprotocol::looks(d) {
        if let Ok(t) = alphaprotocol::parse(d) {
            return Some(t);
        }
    }
    if saltsanc::looks(d) {
        if let Ok(t) = saltsanc::parse(d) {
            return Some(t);
        }
    }
    if lego_tss::looks(d) {
        if let Ok(t) = lego_tss::parse(d) {
            return Some(t);
        }
    }
    if lego_classic::looks(d) {
        if let Ok(t) = lego_classic::parse(d) {
            return Some(t);
        }
    }
    if ori::looks(d) {
        if let Ok(t) = ori::parse(d) {
            return Some(t);
        }
    }
    if forest::looks(d) {
        if let Ok(t) = forest::parse(d) {
            return Some(t);
        }
    }
    if adf::looks(d) {
        if let Ok(t) = adf::parse(d) {
            return Some(t);
        }
    }
    if cave_story::looks(d) {
        if let Ok(t) = cave_story::parse(d) {
            return Some(t);
        }
    }
    if sonic_mania::looks(d) {
        if let Ok(t) = sonic_mania::parse(d) {
            return Some(t);
        }
    }
    if ff13::looks(d) {
        if let Ok(t) = ff13::parse(d) {
            return Some(t);
        }
    }
    if p5r::looks(d) {
        if let Ok(t) = p5r::parse(d) {
            return Some(t);
        }
    }
    if dead_rising::looks_dr1(d) {
        if let Ok(t) = dead_rising::parse_dr1(d) {
            return Some(t);
        }
    }
    if dead_rising::looks_dr2(d) {
        if let Ok(t) = dead_rising::parse_dr2(d) {
            return Some(t);
        }
    }
    if dead_rising::looks_otr(d) {
        if let Ok(t) = dead_rising::parse_otr(d) {
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
    if let Ok(t) = bsinfinite::parse(d) {
        return Some(t);
    }
    if sky1st::looks(d) {
        if let Ok(t) = sky1st::parse(d) {
            return Some(t);
        }
    }
    if rf4s::looks(d) {
        if let Ok(t) = rf4s::parse(d) {
            return Some(t);
        }
    }
    if kh3::looks(d) {
        if let Ok(t) = kh3::parse(d) {
            return Some(t);
        }
    }
    if p4g::looks(d) {
        if let Ok(t) = p4g::parse(d) {
            return Some(t);
        }
    }
    if forspoken::looks(d) {
        if let Ok(t) = forspoken::parse(d) {
            return Some(t);
        }
    }
    if ao::looks(d) {
        if let Ok(t) = ao::parse(d) {
            return Some(t);
        }
    }
    if p3p::looks(d) {
        if let Ok(t) = p3p::parse(d) {
            return Some(t);
        }
    }
    if kenshi::looks(d) {
        if let Ok(t) = kenshi::parse(d) {
            return Some(t);
        }
    }
    if kenshi::looks_platoon(d) {
        if let Ok(t) = kenshi::parse_platoon(d) {
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
    if grimdawn::looks(d) {
        if let Ok(t) = grimdawn::parse(d) {
            return Some(t);
        }
    }
    if titanquest::looks(d) {
        if let Ok(t) = titanquest::parse(d) {
            return Some(t);
        }
    }
    if torchlight2::looks(d) {
        if let Ok(t) = torchlight2::parse(d) {
            return Some(t);
        }
    }
    if nier::looks(d) {
        if let Ok(t) = nier::parse(d) {
            return Some(t);
        }
    }
    if jedi::looks(d) {
        if let Ok(t) = jedi::parse(d) {
            return Some(t);
        }
    }
    if wd2::looks(d) {
        if let Ok(t) = wd2::parse(d) {
            return Some(t);
        }
    }
    if sr4::looks(d) {
        if let Ok(t) = sr4::parse(d) {
            return Some(t);
        }
    }
    if sr3::looks(d) {
        if let Ok(t) = sr3::parse(d) {
            return Some(t);
        }
    }
    if gta3::looks(d) {
        if let Ok(t) = gta3::parse(d) {
            return Some(t);
        }
    }
    if tos::looks(d) {
        if let Ok(t) = tos::parse(d) {
            return Some(t);
        }
    }
    if pvz::looks(d) {
        if let Ok(t) = pvz::parse(d) {
            return Some(t);
        }
    }
    if sims4::looks(d) {
        if let Ok(t) = sims4::parse(d) {
            return Some(t);
        }
    }
    if sims2::looks(d) {
        if let Ok(t) = sims2::parse(d) {
            return Some(t);
        }
    }
    if sims1::looks(d) {
        if let Ok(t) = sims1::parse(d) {
            return Some(t);
        }
    }
    if fos::looks(d) {
        if let Ok(t) = fos::parse(d) {
            return Some(t);
        }
    }
    if fallout12::looks(d) {
        if let Ok(t) = fallout12::parse(d) {
            return Some(t);
        }
    }
    if deusex::looks(d) {
        if let Ok(t) = deusex::parse(d) {
            return Some(t);
        }
    }
    if half_life::looks(d) {
        if let Ok(t) = half_life::parse(d) {
            return Some(t);
        }
    }
    if d2s::looks(d) {
        if let Ok(t) = d2s::parse(d) {
            return Some(t);
        }
    }
    if infinity::looks(d) {
        if let Ok(t) = infinity::parse(d) {
            return Some(t);
        }
    }
    if dragonage::looks(d) {
        if let Ok(t) = dragonage::parse(d) {
            return Some(t);
        }
    }
    if civ6::looks(d) {
        if let Ok(t) = civ6::parse(d) {
            return Some(t);
        }
    }
    if sn2::looks(d) {
        if let Ok(t) = sn2::parse(d) {
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
    if borderlands::looks(d) {
        if let Ok(t) = borderlands::parse(d) {
            return Some(t);
        }
    }
    if mgsv::looks(d) {
        if let Ok(t) = mgsv::parse(d) {
            return Some(t);
        }
    }
    if totalwar::looks(d) {
        if let Ok(t) = totalwar::parse(d) {
            return Some(t);
        }
    }
    if borderlands3::looks(d) {
        if let Ok(t) = borderlands3::parse(d) {
            return Some(t);
        }
    }
    if darkest::looks(d) {
        if let Ok(t) = darkest::parse(d) {
            return Some(t);
        }
    }
    if octopath::looks(d) {
        if let Ok(t) = octopath::parse(d) {
            return Some(t);
        }
    }
    if p3r::looks(d) {
        if let Ok(t) = p3r::parse(d) {
            return Some(t);
        }
    }
    if d.starts_with(b"GVAS") {
        let mut t = gvas::parse(d).ok()?;
        if ttgames::is_lego(&t.view) {
            t.kind_name = ttgames::KIND.into();
        }
        return Some(t);
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

/// BioWare Aurora saves (KOTOR 1 and 2, Neverwinter Nights). `path` finds the files kept
/// beside a save (KOTOR's savenfo.res names the module the player is in).
pub fn parse_aurora(d: &[u8], path: Option<&std::path::Path>) -> Option<TreeDoc> {
    if kotor::looks(d) {
        return kotor::parse(d, path).ok();
    }
    if nwn::looks(d) {
        return nwn::parse(d, path).ok();
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
    godot::parse(d).ok().or_else(|| msgpack::parse(d).ok()).or_else(|| cbor::parse(d).ok())
}

/// Text formats. Ambiguous ones (YAML, TOML) are only trusted by file extension.
pub fn parse_text(text: &str, ext: &str) -> Option<TreeDoc> {
    if roadhog::looks(text.as_bytes()) {
        if let Ok(doc) = roadhog::parse(text.as_bytes()) {
            return Some(doc);
        }
    }
    if obsidian::looks(text) {
        if let Ok(doc) = obsidian::parse(text) {
            return Some(doc);
        }
    }
    if let Some(doc) = undertale::parse(text) {
        return Some(doc);
    }
    if let Some(doc) = es3::parse(text) {
        return Some(doc);
    }
    if sii::looks(text) {
        if let Ok(doc) = sii::parse(text) {
            return Some(doc);
        }
    }
    let t = text.trim_start();
    if mechanicus::looks(text) {
        if let Ok(doc) = mechanicus::parse(text) {
            return Some(doc);
        }
    }
    if planet_crafter::looks(text) {
        if let Ok(doc) = planet_crafter::parse(text) {
            return Some(doc);
        }
    }
    if prison::looks(text) {
        if let Ok(doc) = prison::parse(text) {
            return Some(doc);
        }
    }
    if ksp::looks(text) {
        if let Ok(doc) = ksp::parse(text) {
            return Some(doc);
        }
    }
    if lua::looks(t) {
        if let Ok(doc) = lua::parse(text) {
            return Some(doc);
        }
    }
    if t.starts_with("<save><version>") {
        if let Ok(doc) = wasteland2::parse(text) {
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
