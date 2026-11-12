//! Star Wars Jedi: Fallen Order (`SaveGame00.sav`, `Backup00.sav`, an Unreal Engine 4.21
//! save). The GVAS file holds one byte array, `CompressedActorRecords`, that is a zlib stream;
//! inflated it is a list of actor records, each with a `UniqueIdentifier` and a byte array
//! (`ActorData` or `ComponentData`) that is a property list of its own.
//!
//! The zlib part is opened behind the file's own GVAS header (as with Ace Combat 8's packed
//! data); the player's records are walked tag by tag so every number can be
//! patched in place, without changing any size.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::io::{Read, Write};
use std::sync::Arc;

pub const KIND: &str = "Star Wars Jedi: Fallen Order save";

/// Where the parts sit in the original file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    pub original: Arc<Vec<u8>>,
    /// The property value's size field, the array's element count, and the zlib bytes.
    size_at: usize,
    count_at: usize,
    start: usize,
    end: usize,
}

fn fstring(name: &str) -> Vec<u8> {
    let mut v = ((name.len() + 1) as i32).to_le_bytes().to_vec();
    v.extend_from_slice(name.as_bytes());
    v.push(0);
    v
}

fn find(d: &[u8], what: &[u8], from: usize) -> Option<usize> {
    d.get(from..)?.windows(what.len()).position(|w| w == what).map(|i| i + from)
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The inflated actor records behind the GVAS header.
pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !d.starts_with(b"GVAS") || d.len() < 64 {
        return None;
    }
    let name = find(d, &fstring("CompressedActorRecords"), 0)?;
    let mut p = name + fstring("CompressedActorRecords").len();
    if d.get(p..p + fstring("ArrayProperty").len())? != fstring("ArrayProperty").as_slice() {
        return None;
    }
    p += fstring("ArrayProperty").len();
    let size_at = p;
    p += 8;
    if d.get(p..p + fstring("ByteProperty").len())? != fstring("ByteProperty").as_slice() {
        return None;
    }
    p += fstring("ByteProperty").len() + 1;
    let count_at = p;
    let n = u32_at(d, count_at)? as usize;
    let (start, end) = (count_at + 4, count_at + 4 + n);
    if u32_at(d, size_at)? as usize != n + 4 || end > d.len() || d.get(start)? != &0x78 {
        return None;
    }
    let mut inner = Vec::new();
    flate2::read::ZlibDecoder::new(&d[start..end]).read_to_end(&mut inner).ok()?;
    if !looks(&inner) {
        return None;
    }
    Some((Packed { original: Arc::new(d.to_vec()), size_at, count_at, start, end }, inner))
}

pub fn close(p: &Packed, inner: &[u8]) -> Result<Vec<u8>, String> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
    e.write_all(inner).map_err(|e| e.to_string())?;
    let z = e.finish().map_err(|e| e.to_string())?;
    let o = &p.original;
    let mut out = o[..p.start].to_vec();
    out.extend_from_slice(&z);
    out.extend_from_slice(&o[p.end..]);
    out[p.size_at..p.size_at + 4].copy_from_slice(&(z.len() as u32 + 4).to_le_bytes());
    out[p.count_at..p.count_at + 4].copy_from_slice(&(z.len() as u32).to_le_bytes());
    Ok(out)
}

/// The inflated stream starts with the `ActorRecords` array property.
pub fn looks(d: &[u8]) -> bool {
    d.starts_with(&fstring("ActorRecords")) && d.len() > 4096 && d[..2048].windows(14).any(|w| w == b"RsActorRecord\0")
}

// ------------------------------------------------------------------ property walker

/// One property tag inside a byte array: where its value (or, for a boolean, its flag byte) is.
struct Prop {
    name: String,
    typ: String,
    at: usize,
    size: usize,
}

fn fstr(d: &[u8], p: usize) -> Option<(String, usize)> {
    let n = u32_at(d, p)? as i32;
    if n > 0 {
        let s = d.get(p + 4..p + 4 + n as usize)?;
        Some((String::from_utf8_lossy(&s[..s.len() - 1]).into_owned(), p + 4 + n as usize))
    } else if n == 0 {
        Some((String::new(), p + 4))
    } else {
        let len = (-n) as usize * 2;
        d.get(p + 4..p + 4 + len)?;
        Some((String::new(), p + 4 + len))
    }
}

/// Structs whose value is plain bytes, not a list of properties.
fn plain_struct(name: &str) -> bool {
    matches!(name, "Vector" | "Quat" | "Transform" | "Rotator" | "Vector2D" | "Guid" | "LinearColor" | "DateTime" | "Timespan" | "IntPoint")
}

fn walk(d: &[u8], mut p: usize, end: usize, out: &mut Vec<Prop>) {
    while p < end {
        let Some((name, q)) = fstr(d, p) else { return };
        if name == "None" || name.is_empty() {
            return;
        }
        let Some((typ, q)) = fstr(d, q) else { return };
        let (Some(size), Some(_idx)) = (u32_at(d, q), u32_at(d, q + 4)) else { return };
        let mut q = q + 8;
        let mut extra = String::new();
        match typ.as_str() {
            "StructProperty" => {
                let Some((s, n)) = fstr(d, q) else { return };
                extra = s;
                q = n + 16;
            }
            "ArrayProperty" | "SetProperty" | "ByteProperty" | "EnumProperty" => {
                let Some((s, n)) = fstr(d, q) else { return };
                extra = s;
                q = n;
            }
            "MapProperty" => {
                let Some((_, n)) = fstr(d, q) else { return };
                let Some((_, n)) = fstr(d, n) else { return };
                q = n;
            }
            _ => {}
        }
        if typ == "BoolProperty" {
            // The value is the byte in the tag, then the flags byte.
            out.push(Prop { name, typ, at: q, size: 1 });
            p = q + 2;
            continue;
        }
        q += 1;
        let size = size as usize;
        if q + size > end {
            return;
        }
        out.push(Prop { name, typ: typ.clone(), at: q, size });
        if typ == "StructProperty" && !plain_struct(&extra) {
            walk(d, q, q + size, out);
        }
        p = q + size;
    }
}

/// The properties inside the byte array of the record `id` (`ActorData` or `ComponentData`).
fn record(d: &[u8], id: &str, from: usize, to: usize) -> Option<Vec<Prop>> {
    let key = [fstring("UniqueIdentifier"), fstring("StrProperty")].concat();
    let value = fstring(id);
    let mut at = from;
    while let Some(i) = find(d.get(..to.min(d.len()))?, &key, at) {
        // size + index (8 bytes) and a flags byte, then the id.
        let v = i + key.len() + 9;
        if d.get(v..v + value.len())? == value.as_slice() {
            let mut tags = Vec::new();
            walk(d, v + value.len(), to.min(d.len()), &mut tags);
            let a = tags.iter().find(|t| t.name == "ActorData" || t.name == "ComponentData")?;
            let (s, e) = (a.at + 4, a.at + a.size);
            let mut inner = Vec::new();
            walk(d, s, e, &mut inner);
            return Some(inner);
        }
        at = i + key.len();
    }
    None
}

// ------------------------------------------------------------------ the screen's values

const PLAYER: &str = "BP_SwPlayerState_C";

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Jedi: Fallen Order save".into());
    }
    let window = 0x40000;
    let player = record(d, PLAYER, 0, window).ok_or("the player's record wasn't found")?;
    let mut b = Builder::new(d);
    let int = |b: &mut Builder, path: &[&str], props: &[Prop], name: &str, hi: i64| {
        if let Some(p) = props.iter().find(|p| p.name == name && p.typ == "IntProperty" && p.size == 4) {
            b.ranged(path, p.at, Kind::I32, 0, hi);
        }
    };
    if let Some(fp) = record(d, "BP_SwPlayerState_C:HC_ForcePoint", 0, window) {
        let g = "Skill points";
        int(&mut b, &[g, "Total skill points earned"], &fp, "TotalForcePointsEarned", 999);
        int(&mut b, &[g, "Unspent skill points"], &fp, "UnspentForcePoints", 999);
        if let Some(p) = fp.iter().find(|p| p.name == "CurrentXPCollected" && p.typ == "FloatProperty") {
            b.ranged(&[g, "XP collected toward the next point"], p.at, Kind::F32, 0, 99_999_999);
        }
    }
    let g = "Health and Force";
    int(&mut b, &[g, "Stims carried"], &player, "HealthPotsCur", 99);
    int(&mut b, &[g, "Stim capacity"], &player, "HealthPotsMax", 99);
    int(&mut b, &[g, "Health upgrades found"], &player, "HealthMeterPickUps", 99);
    int(&mut b, &[g, "Force upgrades found"], &player, "ForceMeterPickUps", 99);
    int(&mut b, &[g, "Health upgrade fragments"], &player, "HealthMeterFractions", 99);
    int(&mut b, &[g, "Force upgrade fragments"], &player, "ForceMeterFractions", 99);
    let g = "Progress";
    int(&mut b, &[g, "Secrets collected"], &player, "CurrentCollectables", 9_999);
    int(&mut b, &[g, "Secrets in the game"], &player, "TotalCollectables", 9_999);
    int(&mut b, &[g, "Map sections explored"], &player, "CurrentMapSections", 9_999);
    int(&mut b, &[g, "Map sections in the game"], &player, "TotalMapSections", 9_999);
    int(&mut b, &[g, "Deaths"], &player, "NumDeaths", 999_999);
    int(&mut b, &[g, "Revives used"], &player, "TotalRevives", 999_999);
    Ok(b.finish(KIND, Checksum::None).with_note("Close the game first. Skill points are what the skill tree spends. Unlocked abilities are kept as text that changes length, so they are left alone."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn prop(name: &str, typ: &str, size: u32, extra: &[u8], value: &[u8]) -> Vec<u8> {
        let mut v = fstring(name);
        v.extend(fstring(typ));
        v.extend(size.to_le_bytes());
        v.extend(0u32.to_le_bytes());
        v.extend(extra);
        v.push(0);
        v.extend(value);
        v
    }

    fn int(name: &str, v: i32) -> Vec<u8> {
        prop(name, "IntProperty", 4, &[], &v.to_le_bytes())
    }

    fn bytes_array(name: &str, body: &[u8]) -> Vec<u8> {
        let mut v = fstring(name);
        v.extend(fstring("ArrayProperty"));
        v.extend((body.len() as u32 + 4).to_le_bytes());
        v.extend(0u32.to_le_bytes());
        v.extend(fstring("ByteProperty"));
        v.push(0);
        v.extend((body.len() as u32).to_le_bytes());
        v.extend(body);
        v
    }

    fn unique(id: &str) -> Vec<u8> {
        let s = fstring(id);
        let mut v = fstring("UniqueIdentifier");
        v.extend(fstring("StrProperty"));
        v.extend((s.len() as u32).to_le_bytes());
        v.extend(0u32.to_le_bytes());
        v.push(0);
        v.extend(s);
        v
    }

    fn inner() -> Vec<u8> {
        let mut d = fstring("ActorRecords");
        d.extend(fstring("ArrayProperty"));
        d.extend(vec![0; 8]);
        d.extend(fstring("StructProperty"));
        d.push(0);
        d.extend(fstring("RsActorRecord"));
        d.extend(vec![0; 17]);
        d.extend(unique(PLAYER));
        let mut a = int("HealthPotsCur", 5);
        a.extend(int("HealthPotsMax", 7));
        a.extend(int("NumDeaths", 49));
        a.extend(fstring("None"));
        d.extend(bytes_array("ActorData", &a));
        d.extend(unique("BP_SwPlayerState_C:HC_ForcePoint"));
        let mut c = int("TotalForcePointsEarned", 48);
        c.extend(int("UnspentForcePoints", 3));
        c.extend(fstring("None"));
        d.extend(bytes_array("ComponentData", &c));
        d.extend(vec![0; 4096]);
        d
    }

    fn sample() -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
        e.write_all(&inner()).unwrap();
        let z = e.finish().unwrap();
        let mut d = b"GVAS".to_vec();
        d.extend(vec![0; 60]);
        d.extend(bytes_array("CompressedActorRecords", &z));
        d.extend(fstring("None"));
        d
    }

    #[test]
    fn skill_points_and_stims_are_written_and_packed_again() {
        let raw = sample();
        let (p, body) = open(&raw).expect("opens");
        let mut doc = parse(&body).expect("parses");
        assert_eq!(doc.view_and_editable().0.pointer("/Skill points/Unspent skill points").cloned(), Some(Value::from(3)));
        assert_eq!(doc.view_and_editable().0.pointer("/Health and Force/Stims carried").cloned(), Some(Value::from(5)));
        doc.set("/Skill points/Unspent skill points", Value::from(99)).unwrap();
        doc.set("/Health and Force/Stims carried", Value::from(12)).unwrap();
        let edited = doc.to_bytes().unwrap();
        let out = close(&p, &edited).unwrap();
        let (_, again) = open(&out).expect("opens again");
        let mut doc = parse(&again).unwrap();
        assert_eq!(doc.view_and_editable().0.pointer("/Skill points/Unspent skill points").cloned(), Some(Value::from(99)));
        assert_eq!(doc.view_and_editable().0.pointer("/Health and Force/Stims carried").cloned(), Some(Value::from(12)));
        assert_eq!(doc.view_and_editable().0.pointer("/Progress/Deaths").cloned(), Some(Value::from(49)));
    }
}

#[cfg(test)]
mod real {
    use super::*;

    /// Local check against a real save: `JFO_SAMPLE=<path> cargo test --release real_save -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_save() {
        let Ok(path) = std::env::var("JFO_SAMPLE") else { return };
        let raw = std::fs::read(path).unwrap();
        let (p, body) = open(&raw).expect("opens");
        let mut doc = parse(&body).expect("parses");
        println!("{}", serde_json::to_string_pretty(doc.view_and_editable().0).unwrap());
        let out = close(&p, &doc.to_bytes().unwrap()).unwrap();
        let (_, again) = open(&out).expect("reopens");
        assert_eq!(again, body);
    }
}
