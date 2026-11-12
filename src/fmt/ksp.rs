//! Kerbal Space Program saves (`persistent.sfs`, quicksaves): plain-text ConfigNodes.
//!
//! ```text
//! GAME
//! {
//!   version = 1.12.5
//!   SCENARIO
//!   {
//!     name = Funding
//!     funds = 100352.04
//!   }
//! }
//! ```
//!
//! A node whose first lines hold `name = X` is shown as `HEAD: X` (so the money is at
//! `/GAME/SCENARIO: Funding/funds`). Only the text of a changed value is rewritten; everything
//! else, including spacing and line endings, stays byte for byte.

use crate::tree::{child, insert_unique, text_to_value, value_to_text, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Kerbal Space Program save";

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    orig: String,
}

#[derive(Debug)]
enum Item {
    Leaf(String, usize),
    Node(Node),
}

#[derive(Debug)]
struct Node {
    head: String,
    items: Vec<Item>,
}

#[derive(Clone, Debug)]
struct KspBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for KspBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            p.add(l.start, l.end, value_to_text(&l.orig, v).into_bytes());
        }
        p.apply(&self.raw)
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.as_str().is_some_and(|s| s.contains(['\n', '\r'])) {
            return Err("A value can't contain a line break".into());
        }
        Ok(())
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// True when the text starts like a KSP save (or a craft / ConfigNode file with a node first).
pub fn looks(text: &str) -> bool {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    let mut lines = t.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("//"));
    let (Some(a), Some(b)) = (lines.next(), lines.next()) else { return false };
    b == "{" && !a.contains(['=', '{', '}', ' ']) && a.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if text.starts_with('\u{feff}') || !looks(text) {
        return Err("not a ConfigNode file".into());
    }
    // Split into trimmed lines with their offsets.
    let mut lines: Vec<(usize, &str)> = vec![];
    let mut off = 0;
    for l in text.split_inclusive('\n') {
        lines.push((off, l));
        off += l.len();
    }
    let mut leaves: Vec<Leaf> = vec![];
    let mut pos = 0;
    let mut root = Node { head: String::new(), items: vec![] };
    let mut stack: Vec<Node> = vec![];
    let mut pending: Option<String> = None;
    while pos < lines.len() {
        let (start, raw) = lines[pos];
        pos += 1;
        let body = raw.trim_end_matches(['\n', '\r']);
        let t = body.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t == "{" {
            let head = pending.take().ok_or("a { with no node name")?;
            stack.push(Node { head, items: vec![] });
            continue;
        }
        if t == "}" {
            let done = stack.pop().ok_or("a } with no node")?;
            match stack.last_mut() {
                Some(parent) => parent.items.push(Item::Node(done)),
                None => root.items.push(Item::Node(done)),
            }
            continue;
        }
        pending = None;
        if let Some(eq) = body.find('=') {
            let key = body[..eq].trim().to_string();
            let raw_val = &body[eq + 1..];
            let lead = raw_val.len() - raw_val.trim_start().len();
            let val = raw_val.trim();
            let s = start + eq + 1 + lead;
            let id = leaves.len();
            leaves.push(Leaf { start: if val.is_empty() { start + body.trim_end().len() } else { s }, end: if val.is_empty() { start + body.trim_end().len() } else { s + val.len() }, orig: val.to_string() });
            let item = Item::Leaf(key, id);
            match stack.last_mut() {
                Some(n) => n.items.push(item),
                None => return Err("a value outside any node".into()),
            }
        } else {
            pending = Some(t.to_string());
        }
    }
    if !stack.is_empty() {
        return Err("a node is not closed".into());
    }
    let mut editable = HashMap::new();
    let mut map = Map::new();
    fill(&root.items, "", &mut map, &mut editable, &leaves);
    let backend = KspBackend { raw: text.as_bytes().to_vec(), leaves };
    Ok(TreeDoc::new(Value::Object(map), editable, KIND, Box::new(backend)))
}

fn name_of(n: &Node, leaves: &[Leaf]) -> Option<String> {
    n.items.iter().find_map(|i| match i {
        Item::Leaf(k, id) if k == "name" => Some(leaves[*id].orig.clone()),
        _ => None,
    })
}

fn fill(items: &[Item], ptr: &str, map: &mut Map<String, Value>, editable: &mut HashMap<String, usize>, leaves: &[Leaf]) {
    for it in items {
        match it {
            Item::Leaf(k, id) => {
                let key = insert_unique(map, k, text_to_value(&leaves[*id].orig));
                editable.insert(child(ptr, &key), *id);
            }
            Item::Node(n) => {
                let label = match name_of(n, leaves) {
                    Some(nm) if !nm.is_empty() => format!("{}: {nm}", n.head),
                    _ => n.head.clone(),
                };
                let mut sub = Map::new();
                let key = crate::tree::unique_key(map, &label);
                let p = child(ptr, &key);
                fill(&n.items, &p, &mut sub, editable, leaves);
                map.insert(key, Value::Object(sub));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "GAME\r\n{\r\n\tversion = 1.12.5\r\n\tSCENARIO\r\n\t{\r\n\t\tname = Funding\r\n\t\tfunds = 100352.04\r\n\t}\r\n\tSCENARIO\r\n\t{\r\n\t\tname = Reputation\r\n\t\trep = 8.9\r\n\t\tlinkURL =\r\n\t}\r\n}\r\n";

    #[test]
    fn edit_keeps_the_rest() {
        let mut doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        assert_eq!(doc.view.pointer("/GAME/SCENARIO: Funding/funds").unwrap(), 100352.04);
        doc.set("/GAME/SCENARIO: Funding/funds", Value::from(9000000)).unwrap();
        doc.set("/GAME/SCENARIO: Reputation/linkURL", Value::from("x")).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, SAMPLE.replace("100352.04", "9000000").replace("linkURL =", "linkURL =x"));
    }
}

#[cfg(test)]
mod sample_tests {
    #[test]
    fn real_saves_round_trip() {
        let Ok(dir) = std::env::var("KSP_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            let raw = std::fs::read(&p).unwrap();
            let text = String::from_utf8(raw.clone()).unwrap();
            let doc = super::parse(&text).unwrap_or_else(|e| panic!("{p:?}: {e}"));
            assert_eq!(doc.to_bytes().unwrap(), raw, "{p:?}");
            assert!(doc.view.pointer("/GAME/SCENARIO: Funding/funds").is_some(), "{p:?}");
        }
    }
}
