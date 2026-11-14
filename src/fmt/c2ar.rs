//! NeocoreGames "C2AR" containers (King Arthur: Knight's Tale): a 14-byte header, then entries
//! that are a 4-byte length and a text file (`Name.cfg`, a brace-nested `key=value` list).
//! Only value spans are rewritten and the entry lengths are worked out again.

use crate::tree::{child, insert_unique, text_to_value, unique_key, value_to_text, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

const HEAD: usize = 14;

#[derive(Clone, Debug)]
struct Leaf {
    entry: usize,
    start: usize,
    end: usize,
    orig: String,
}

#[derive(Clone, Debug)]
struct C2arBackend {
    head: Vec<u8>,
    entries: Vec<Vec<u8>>,
    leaves: Vec<Leaf>,
}

impl Backend for C2arBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = self.head.clone();
        for (i, e) in self.entries.iter().enumerate() {
            let mut p = Patches::default();
            for (id, v) in changes {
                let l = &self.leaves[*id];
                if l.entry == i {
                    p.add(l.start, l.end, value_to_text(&l.orig, v).into_bytes());
                }
            }
            let text = p.apply(e)?;
            out.extend_from_slice(&(text.len() as u32).to_le_bytes());
            out.extend_from_slice(&text);
        }
        Ok(out)
    }

    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.as_str().is_some_and(|s| s.contains(['\n', '\r'])) {
            return Err("values can't contain line breaks".into());
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > HEAD + 8 && d.starts_with(b"C2AR")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a C2AR container".into());
    }
    let mut pos = HEAD;
    let mut entries = Vec::new();
    while pos < d.len() {
        let n = u32::from_le_bytes(d.get(pos..pos + 4).ok_or("cut off")?.try_into().unwrap()) as usize;
        entries.push(d.get(pos + 4..pos + 4 + n).ok_or("entry runs past the file")?.to_vec());
        pos += 4 + n;
    }
    let mut root = Map::new();
    let mut leaves = Vec::new();
    let mut editable = HashMap::new();
    for (ei, e) in entries.iter().enumerate() {
        let text = std::str::from_utf8(e).map_err(|_| "entry is not text")?;
        let mut lines = text.split_inclusive('\n').scan(0usize, |off, l| {
            let s = *off;
            *off += l.len();
            Some((s, l))
        });
        let mut name = String::new();
        let mut pending: Option<String> = None;
        // Stack of (pointer, map being filled).
        let mut stack: Vec<(String, Map<String, Value>)> = Vec::new();
        let mut done: Option<(String, Map<String, Value>)> = None;
        // Leaves are registered with the pointer they will have once the maps are linked.
        let mut anon = 0;
        for (s, line) in &mut lines {
            let body = line.trim_end_matches(['\n', '\r']);
            let t = body.trim();
            if t.is_empty() {
                continue;
            }
            if t == "{" {
                let key = pending.take().unwrap_or_else(|| {
                    anon += 1;
                    format!("Block {anon}")
                });
                if stack.is_empty() {
                    let ptr = child("", &unique_key(&root, &name));
                    stack.push((ptr, Map::new()));
                } else {
                    let (pp, pm) = stack.last().unwrap();
                    let k = unique_key(pm, &key);
                    let ptr = child(pp, &k);
                    stack.push((ptr, Map::new()));
                    // Reserve the key so repeated names stay unique.
                    let _ = k;
                }
                continue;
            }
            if t == "}" {
                let (ptr, map) = stack.pop().ok_or("unbalanced braces")?;
                if let Some((_, pm)) = stack.last_mut() {
                    let k = ptr.rsplit('/').next().unwrap_or("").replace("~1", "/").replace("~0", "~");
                    pm.insert(k, Value::Object(map));
                } else {
                    done = Some((ptr, map));
                }
                continue;
            }
            if let Some(eq) = body.find('=') {
                let Some((ptr, map)) = stack.last_mut() else { continue };
                let key = body[..eq].trim();
                let raw = &body[eq + 1..];
                let val = raw.trim_end();
                let vs = s + eq + 1;
                let k = insert_unique(map, key, text_to_value(val));
                editable.insert(child(ptr, &k), leaves.len());
                leaves.push(Leaf { entry: ei, start: vs, end: vs + val.len(), orig: val.to_string() });
            } else if stack.is_empty() {
                name = t.to_string();
            } else {
                pending = Some(t.to_string());
            }
        }
        if let Some((ptr, map)) = done {
            let k = ptr.trim_start_matches('/').replace("~1", "/").replace("~0", "~");
            root.insert(k, Value::Object(map));
        }
    }
    if leaves.is_empty() {
        return Err("no values".into());
    }
    let backend = C2arBackend { head: d[..HEAD].to_vec(), entries, leaves };
    Ok(TreeDoc::new(Value::Object(root), editable, "King Arthur save (C2AR)", Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let a = "\nInfo.cfg\n{\n\tName=Save\n}\n";
        let b = "\nAdventure.cfg\n{\n\tGold=100\n\tHeroes\n\t{\n\t\tHeroSaveData\n\t\t{\n\t\t\tLevel=2\n\t\t}\n\t\tHeroSaveData\n\t\t{\n\t\t\tLevel=5\n\t\t}\n\t}\n}\n";
        let mut d = b"C2AR\x00\x1a\x05\x00\x00\x01\x00\x00\x00\x00".to_vec();
        for t in [a, b] {
            d.extend_from_slice(&(t.len() as u32).to_le_bytes());
            d.extend_from_slice(t.as_bytes());
        }
        d
    }

    #[test]
    fn reads_edits_and_keeps_lengths() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Adventure.cfg/Gold").unwrap(), 100);
        assert_eq!(doc.view.pointer("/Adventure.cfg/Heroes/HeroSaveData (2)/Level").unwrap(), 5);
        doc.set("/Adventure.cfg/Gold", serde_json::json!(123456)).unwrap();
        let out = doc.to_bytes().unwrap();
        let again = parse(&out).unwrap();
        assert_eq!(again.view.pointer("/Adventure.cfg/Gold").unwrap(), 123456);
        assert_eq!(again.view.pointer("/Info.cfg/Name").unwrap(), "Save");
    }
}
