//! Warhammer 40,000: Mechanicus saves (`Meta.sav`): the game's own Json.NET dialect (type tags
//! and `$eref:` references that are not valid JSON), so it is read by line and each value is
//! patched in place; the rest of the (often 10 MB) file stays byte for byte.
//!
//! The game keeps the CRC-32 of `Meta.sav` in the `SaveState.sav` beside it; that file is
//! written again after a save (see `companions`).

use crate::tree::{child, text_to_value, value_to_text, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const KIND: &str = "Warhammer 40,000: Mechanicus save";

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    orig: String,
}

#[derive(Clone, Debug)]
struct MechBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for MechBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            p.add(l.start, l.end, value_to_text(&l.orig, v).into_bytes());
        }
        p.apply(&self.raw)
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.is_number() || v.is_boolean() {
            Ok(())
        } else {
            Err("This value is a number".into())
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(text: &str) -> bool {
    let head = &text[..text.len().min(600)];
    head.contains("Mechanicus.Saves.SaveDataArkMechanicus") && head.contains("\"$type\"")
}

/// `"key": 12,` on one line: (key, value start, value end) within the line.
fn number_line(line: &str) -> Option<(&str, usize, usize)> {
    let t = line.trim_start();
    let lead = line.len() - t.len();
    let rest = t.strip_prefix('"')?;
    let k_end = rest.find('"')?;
    let key = &rest[..k_end];
    let after = &rest[k_end + 1..];
    let colon = after.strip_prefix(':')?;
    let v = colon.trim_start();
    let v = v.trim_end_matches([',', '\r', '\n']);
    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit() || b == b'-' || b == b'.') {
        return None;
    }
    let start = lead + 1 + k_end + 1 + 1 + (colon.len() - colon.trim_start().len());
    Some((key, start, start + v.len()))
}

fn string_line(line: &str, key: &str) -> Option<String> {
    let t = line.trim_start().strip_prefix('"')?.strip_prefix(key)?.strip_prefix("\": \"")?;
    let t = t.trim_end_matches([',', '\r', '\n']).strip_suffix('"')?;
    serde_json::from_str::<String>(&format!("\"{t}\"")).ok()
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not a Mechanicus save".into());
    }
    let mut leaves: Vec<Leaf> = vec![];
    let mut editable: HashMap<String, usize> = HashMap::new();
    let mut ark = Map::new();
    let mut priests: Vec<Value> = vec![];
    let mut in_resources = false;
    // The Techpriest being read: (name, fields so far).
    let mut cur: Option<(String, Map<String, Value>, usize)> = None;
    let mut name_hint = (String::new(), String::new());
    let mut off = 0usize;
    let mut add = |leaves: &mut Vec<Leaf>, start: usize, end: usize, text: &str| -> (usize, Value) {
        let id = leaves.len();
        let orig = text[start..end].to_string();
        let v = text_to_value(&orig);
        leaves.push(Leaf { start, end, orig });
        (id, v)
    };
    let finish = |cur: &mut Option<(String, Map<String, Value>, usize)>, priests: &mut Vec<Value>| {
        if let Some((name, m, _)) = cur.take() {
            let mut o = Map::new();
            o.insert("Name".into(), Value::from(name));
            for (k, v) in m {
                o.insert(k, v);
            }
            priests.push(Value::Object(o));
        }
    };
    for line in text.split_inclusive('\n') {
        let start = off;
        off += line.len();
        if line.contains("SaveDataArkMechanicusResources") {
            in_resources = true;
            continue;
        }
        if let Some(n) = string_line(line, "firstname") {
            name_hint.0 = n;
        }
        if let Some(n) = string_line(line, "lastname") {
            name_hint.1 = n;
        }
        let Some((key, s, e)) = number_line(line) else { continue };
        let (s, e) = (start + s, start + e);
        match key {
            "data" if in_resources => {
                in_resources = false;
                let (id, v) = add(&mut leaves, s, e, text);
                ark.insert("Blackstone".into(), v);
                editable.insert(child("/Ark", "Blackstone"), id);
            }
            "additionalDeploymentPoints" | "reduceTroopDeploymentCost" | "cognitionBonusPoint" | "nbTechpriestSlots" | "nbTroopSlots" => {
                let label = match key {
                    "additionalDeploymentPoints" => "Extra deployment points",
                    "reduceTroopDeploymentCost" => "Troop deployment cost reduction",
                    "cognitionBonusPoint" => "Cognition bonus points",
                    "nbTechpriestSlots" => "Techpriest slots",
                    _ => "Troop slots",
                };
                let (id, v) = add(&mut leaves, s, e, text);
                ark.insert(label.into(), v);
                editable.insert(child("/Ark", label), id);
            }
            "level" => {
                finish(&mut cur, &mut priests);
                let (id, v) = add(&mut leaves, s, e, text);
                let mut m = Map::new();
                m.insert("Level".into(), v);
                let idx = priests.len();
                editable.insert(format!("/Techpriests/{idx}/Level"), id);
                let name = [name_hint.0.as_str(), name_hint.1.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
                name_hint = (String::new(), String::new());
                cur = Some((name, m, idx));
            }
            "xp" | "augmentCapacity" | "slotPoints" | "skillTreePoints" | "currentLife" => {
                if let Some((_, m, idx)) = cur.as_mut() {
                    let label = match key {
                        "xp" => "XP",
                        "augmentCapacity" => "Augment capacity",
                        "slotPoints" => "Slot points",
                        "skillTreePoints" => "Skill points",
                        _ => "Health",
                    };
                    if !m.contains_key(label) {
                        let (id, v) = add(&mut leaves, s, e, text);
                        m.insert(label.into(), v);
                        editable.insert(format!("/Techpriests/{idx}/{label}"), id);
                    }
                }
            }
            _ => {}
        }
    }
    finish(&mut cur, &mut priests);
    if ark.is_empty() {
        return Err("no Mechanicus values found".into());
    }
    let mut root = Map::new();
    root.insert("Ark".into(), Value::Object(ark));
    root.insert("Techpriests".into(), Value::Array(priests));
    let backend = MechBackend { raw: text.as_bytes().to_vec(), leaves };
    let mut doc = TreeDoc::new(Value::Object(root), editable, KIND, Box::new(backend));
    doc.note = Some("Only the Ark's resources and the Techpriests' levels are shown; the rest of Meta.sav is kept exactly as it was. SaveState.sav (the game's checksum of this file) is updated when you save.".into());
    Ok(doc)
}

/// After `Meta.sav` is written: its CRC-32 in the `SaveState.sav` beside it.
pub fn companions(path: &Path, bytes: &[u8]) -> Vec<(PathBuf, Vec<u8>)> {
    if path.file_name().and_then(|n| n.to_str()) != Some("Meta.sav") || !bytes.starts_with(b"{") {
        return vec![];
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(600)]).to_string();
    if !head.contains("Mechanicus.Saves.SaveDataArkMechanicus") {
        return vec![];
    }
    let state = path.with_file_name("SaveState.sav");
    let Ok(old) = std::fs::read_to_string(&state) else { return vec![] };
    let mut h = flate2::Crc::new();
    h.update(bytes);
    let sum = format!("{:08x}", h.sum());
    let key = "\"crc32Meta\": \"";
    let Some(at) = old.find(key) else { return vec![] };
    let from = at + key.len();
    let Some(len) = old[from..].find('"') else { return vec![] };
    if old[from..from + len] == sum {
        return vec![];
    }
    let new = format!("{}{}{}", &old[..from], sum, &old[from + len..]);
    vec![(state, new.into_bytes())]
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &str = "{\n    \"$id\": 0,\n    \"$type\": \"0|Mechanicus.Saves.SaveDataArkMechanicus, Assembly-CSharp\",\n    \"resources\": {\n        \"$id\": 5,\n        \"$type\": \"8|Mechanicus.Saves.SaveDataArkMechanicusResources, Assembly-CSharp\",\n        \"data\": 96\n    },\n    \"list\": [\n        $eref:3\n    ],\n    \"firstname\": \"\",\n    \"lastname\": \"Arturo\",\n    \"level\": 20,\n    \"xp\": 5,\n    \"skillTreePoints\": 7,\n    \"cognitionBonusPoint\": 6\n}\n";

    #[test]
    fn blackstone_and_priest_edit_in_place() {
        let mut doc = parse(S).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), S.as_bytes());
        assert_eq!(doc.view["Ark"]["Blackstone"], 96);
        assert_eq!(doc.view["Techpriests"][0]["Name"], "Arturo");
        doc.set("/Ark/Blackstone", Value::from(5000)).unwrap();
        doc.set("/Techpriests/0/Skill points", Value::from(99)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, S.replace("\"data\": 96", "\"data\": 5000").replace("\"skillTreePoints\": 7", "\"skillTreePoints\": 99"));
    }
}
