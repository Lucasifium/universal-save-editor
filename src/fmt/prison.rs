//! Prison Architect saves (`*.prison`): plain text, one `Key value` per line before the first
//! `BEGIN` block, and the money in either a top-level `Balance` line or in
//! `BEGIN Finance    Balance 38488.0  LastDay 15  END`.
//!
//! Only the top-level settings and the finance line are shown; a value is rewritten on its
//! own, so the rest of the (often 10 MB) file stays byte for byte.

use crate::tree::{child, text_to_value, value_to_text, Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Prison Architect save";

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    orig: String,
}

#[derive(Clone, Debug)]
struct PaBackend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for PaBackend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            p.add(l.start, l.end, value_to_text(&l.orig, v).into_bytes());
        }
        p.apply(&self.raw)
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        if v.as_str().is_some_and(|s| s.contains(['\n', '\r', ' '])) {
            return Err("A value can't contain a space or a line break".into());
        }
        Ok(())
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

/// A text file whose first lines are `NumCellsX`/`Version` style settings including `ObjectId.next`.
pub fn looks(text: &str) -> bool {
    let head: Vec<&str> = text.lines().take(60).collect();
    head.iter().any(|l| l.starts_with("NumCellsX ")) && head.iter().any(|l| l.starts_with("ObjectId.next"))
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not a Prison Architect save".into());
    }
    let mut leaves: Vec<Leaf> = vec![];
    let mut settings = Map::new();
    let mut finance = Map::new();
    let mut editable: HashMap<String, usize> = HashMap::new();
    let mut off = 0;
    let mut seen_finance = false;
    for line in text.split_inclusive('\n') {
        let start = off;
        off += line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let first = body.chars().next();
        if first.is_some_and(|c| c.is_ascii_alphabetic()) && !body.starts_with("BEGIN") && !body.starts_with("END") {
            let key_end = body.find(char::is_whitespace).unwrap_or(body.len());
            let rest = &body[key_end..];
            let val = rest.trim();
            if val.is_empty() || val.contains(char::is_whitespace) {
                continue;
            }
            let lead = rest.len() - rest.trim_start().len();
            let s = start + key_end + lead;
            let id = leaves.len();
            leaves.push(Leaf { start: s, end: s + val.len(), orig: val.to_string() });
            let key = &body[..key_end];
            if key == "Balance" {
                finance.insert("Balance".into(), text_to_value(val.trim_end_matches('.')));
                editable.insert(child("/Finance", "Balance"), id);
            } else {
                settings.insert(key.to_string(), text_to_value(val));
                editable.insert(child("/Settings", key), id);
            }
        } else if !seen_finance && body.starts_with("BEGIN Finance ") && body.contains(" END") {
            seen_finance = true;
            let toks: Vec<(usize, &str)> = {
                let mut v = vec![];
                let mut i = 0;
                for t in body.split_whitespace() {
                    let at = body[i..].find(t).unwrap() + i;
                    v.push((at, t));
                    i = at + t.len();
                }
                v
            };
            for w in toks.windows(2) {
                let (_, k) = w[0];
                let (at, v) = w[1];
                if k == "Balance" || k == "LastDay" {
                    let id = leaves.len();
                    leaves.push(Leaf { start: start + at, end: start + at + v.len(), orig: v.to_string() });
                    finance.insert(k.to_string(), text_to_value(v));
                    editable.insert(child("/Finance", k), id);
                }
            }
        }
    }
    if finance.is_empty() {
        return Err("no money (Balance) in this save".into());
    }
    let mut root = Map::new();
    root.insert("Finance".into(), Value::Object(finance));
    root.insert("Settings".into(), Value::Object(settings));
    let backend = PaBackend { raw: text.as_bytes().to_vec(), leaves };
    Ok(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(backend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "\nNumCellsX            100  \nTimeIndex            21688.0  \nObjectId.next        310861  \nBalance              1445112.  \nBEGIN Foo END\n";
    const B: &str = "\nNumCellsX            100  \nObjectId.next        310861  \nBEGIN Finance    Balance 38488.0  LastDay 15  END\n";

    #[test]
    fn money_edits_in_place() {
        for (s, from, to) in [(A, "1445112.", "9999999"), (B, "38488.0", "9999999")] {
            let mut doc = parse(s).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), s.as_bytes());
            doc.set("/Finance/Balance", Value::from(9999999)).unwrap();
            assert_eq!(String::from_utf8(doc.to_bytes().unwrap()).unwrap(), s.replace(from, to));
        }
    }
}
