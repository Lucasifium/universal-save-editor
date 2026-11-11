//! Owlcat's Pathfinder saves (Kingmaker, Wrath of the Righteous; `.zks`): a zip of JSON
//! files. The party lives in `party.json`, money and the kingdom in `player.json`, the
//! save's title in `header.json`; a `checksums` entry holds an MD5 per file (written as
//! text the way .NET turns bytes into a string, so it's lossy: invalid UTF-8 becomes
//! U+FFFD).
//!
//! The editor sees one JSON document `{"header":…, "player":…, "party":…}`. On save,
//! only the parts that changed are written again (compact, as the game writes them),
//! their checksum lines are regenerated and every other entry is copied as it was.
//! Layout facts from Arcemi's save editor (MPL-2.0) and fmonniot's WotR editor.

use md5::{Digest, Md5};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::sync::OnceLock;

const PARTS: [(&str, &str); 3] = [("header", "header.json"), ("player", "player.json"), ("party", "party.json")];

fn strip_bom(b: &[u8]) -> &[u8] {
    b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(b)
}

/// The combined JSON text, if `data` is an Owlcat save.
pub fn open(data: &[u8]) -> Option<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(data)).ok()?;
    let mut out = b"{".to_vec();
    for (i, (key, file)) in PARTS.iter().enumerate() {
        let mut b = Vec::new();
        zip.by_name(file).ok()?.read_to_end(&mut b).ok()?;
        let b = strip_bom(&b);
        if b.first() != Some(&b'{') {
            return None;
        }
        if i > 0 {
            out.push(b',');
        }
        out.extend(format!("\"{key}\":").as_bytes());
        out.extend_from_slice(b);
    }
    out.push(b'}');
    Some(out)
}

/// The checksum text .NET would write for these bytes.
fn digest_text(b: &[u8]) -> Vec<u8> {
    let d = Md5::digest(b);
    String::from_utf8_lossy(&d).into_owned().into_bytes()
}

/// Writes the combined JSON back into the archive.
pub fn save(archive: &[u8], combined: &[u8]) -> Result<Vec<u8>, String> {
    let err = |e: zip::result::ZipError| e.to_string();
    let doc: Value = serde_json::from_slice(combined).map_err(|e| format!("the save's JSON doesn't read back: {e}"))?;
    let mut src = zip::ZipArchive::new(Cursor::new(archive)).map_err(err)?;
    let mut new: HashMap<&str, Vec<u8>> = HashMap::new();
    for (key, file) in PARTS {
        let part = doc.get(key).ok_or(format!("{key} is missing"))?;
        let mut old = Vec::new();
        src.by_name(file).map_err(err)?.read_to_end(&mut old).map_err(|e| e.to_string())?;
        let old_value: Value = serde_json::from_slice(strip_bom(&old)).map_err(|e| e.to_string())?;
        if &old_value != part {
            let mut b = if old.starts_with(b"\xEF\xBB\xBF") { b"\xEF\xBB\xBF".to_vec() } else { vec![] };
            b.extend(serde_json::to_vec(part).map_err(|e| e.to_string())?);
            new.insert(file, b);
        }
    }
    if new.is_empty() {
        return Ok(archive.to_vec());
    }
    let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let f = src.by_index_raw(i).map_err(err)?;
        let name = f.name().to_string();
        if let Some(b) = new.get(name.as_str()) {
            drop(f);
            out.start_file(name, deflated).map_err(err)?;
            out.write_all(b).map_err(|e| e.to_string())?;
        } else if name == "checksums" {
            drop(f);
            let mut old = Vec::new();
            src.by_index(i).map_err(err)?.read_to_end(&mut old).map_err(|e| e.to_string())?;
            out.start_file(name, deflated).map_err(err)?;
            out.write_all(&fix_checksums(&old, &new)).map_err(|e| e.to_string())?;
        } else {
            out.raw_copy_file(f).map_err(err)?;
        }
    }
    Ok(out.finish().map_err(err)?.into_inner())
}

/// Records are `<name>: <digest>\n===========\n`; the digest itself may contain newlines,
/// so records are split on the separator line.
fn fix_checksums(old: &[u8], new: &HashMap<&str, Vec<u8>>) -> Vec<u8> {
    const SEP: &[u8] = b"\n===========\n";
    let mut records: Vec<Vec<u8>> = vec![];
    let mut rest = old;
    while let Some(i) = rest.windows(SEP.len()).position(|w| w == SEP) {
        records.push(rest[..i].to_vec());
        rest = &rest[i + SEP.len()..];
    }
    records.push(rest.to_vec());
    for rec in &mut records {
        let Some(e) = rec.windows(2).position(|w| w == b": ") else { continue };
        let Ok(name) = std::str::from_utf8(&rec[..e]) else { continue };
        if let Some(b) = new.get(name) {
            *rec = [&rec[..e + 2], &digest_text(b)[..]].concat();
        }
    }
    records.join(SEP)
}

/// A class, race or companion name by blueprint GUID. Some GUIDs mean different
/// companions in the two games, so the game is given (`wotr`).
pub fn name(guid: &str, wotr: bool) -> Option<String> {
    static T: OnceLock<Vec<(String, String, String)>> = OnceLock::new();
    let t = T.get_or_init(|| {
        include_str!("data/owlcat_names.tsv")
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| {
                let f: Vec<&str> = l.splitn(4, '\t').collect();
                (f.len() == 4).then(|| (f[0].to_string(), f[2].to_string(), f[3].to_string()))
            })
            .collect()
    });
    let game = if wotr { "wotr" } else { "kingmaker" };
    let hits = || t.iter().filter(|x| x.0 == guid);
    hits().find(|x| x.1 == game).or_else(|| hits().find(|x| x.1.is_empty())).map(|x| x.2.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zks(player: &str, party: &str) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let o = zip::write::SimpleFileOptions::default();
        for (n, b) in [("header.json", r#"{"Name":"Test"}"#), ("player.json", player), ("party.json", party), ("area.json", "{}")] {
            z.start_file(n, o).unwrap();
            z.write_all(b.as_bytes()).unwrap();
        }
        let sums = format!("player.json: {}\n===========\nparty.json: {}\n===========\n", String::from_utf8_lossy(&digest_text(player.as_bytes())), String::from_utf8_lossy(&digest_text(party.as_bytes())));
        z.start_file("checksums", o).unwrap();
        z.write_all(sums.as_bytes()).unwrap();
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn edits_one_part_and_its_checksum() {
        let a = zks(r#"{"Money":100,"Chapter":1}"#, r#"{"m_EntityData":[]}"#);
        let combined = open(&a).unwrap();
        assert_eq!(save(&a, &combined).unwrap(), a, "no change keeps the archive");
        let mut v: Value = serde_json::from_slice(&combined).unwrap();
        v["player"]["Money"] = 5000.into();
        let out = save(&a, &serde_json::to_vec(&v).unwrap()).unwrap();
        let mut z = zip::ZipArchive::new(Cursor::new(&out)).unwrap();
        let mut p = String::new();
        z.by_name("player.json").unwrap().read_to_string(&mut p).unwrap();
        assert_eq!(p, r#"{"Money":5000,"Chapter":1}"#);
        let mut c = Vec::new();
        z.by_name("checksums").unwrap().read_to_end(&mut c).unwrap();
        let want = [b"player.json: ".to_vec(), digest_text(p.as_bytes())].concat();
        assert!(c.starts_with(&want));
        assert!(c.ends_with(b"\n===========\n"));
        let mut party = Vec::new();
        z.by_name("party.json").unwrap().read_to_end(&mut party).unwrap();
        assert_eq!(party, br#"{"m_EntityData":[]}"#);
    }
}

#[cfg(test)]
mod real {
    /// Research aid: SAVE_EDITOR_ZKS = a .zks; writes a copy with money +1 to ZKS_OUT.
    #[test]
    #[ignore]
    fn real_save() {
        use crate::formats::{decode, encode, Content};
        let raw = std::fs::read(std::env::var("SAVE_EDITOR_ZKS").unwrap()).unwrap();
        let mut d = decode(&raw);
        println!("{}", crate::formats::describe(&d.layers, &d.content));
        assert_eq!(encode(&d.layers, &d.content).unwrap(), raw, "no edit keeps the file");
        let Content::Json { value, .. } = &mut d.content else { panic!("not JSON") };
        assert!(crate::game_ui::detect("JSON", value).is_some(), "no screen");
        let m = value["player"]["Money"].as_i64().unwrap();
        value["player"]["Money"] = (m + 1).into();
        let out = encode(&d.layers, &d.content).unwrap();
        std::fs::write(std::env::var("ZKS_OUT").unwrap(), &out).unwrap();
        let again = decode(&out);
        let Content::Json { value, .. } = &again.content else { panic!("re-read failed") };
        assert_eq!(value["player"]["Money"].as_i64(), Some(m + 1));
    }
}
