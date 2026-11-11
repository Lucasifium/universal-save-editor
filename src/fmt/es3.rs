//! Easy Save 3 (Unity) key files that aren't strict JSON.
//!
//! An ES3 file is one object of `"key": {"__type": "...", "value": ...}` entries. ES3 writes
//! a dictionary whose keys aren't text with the key itself as an object
//! (`{{"low":2048,"high":0}:{...}}`), which no JSON reader accepts; Phasmophobia's
//! `SaveFile.txt` has such a dictionary. This reader walks the text itself: every number,
//! true/false and string becomes a value that is rewritten in place, an object used as a
//! key is shown as its text, and everything else in the file stays byte for byte.
//!
//! Strict JSON files (Lethal Company, R.E.P.O.) never get here: the JSON reader takes them.

use crate::tree::{child, Backend, Patches, TreeDoc};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;

pub const KIND: &str = "Easy Save 3 data";
pub const KIND_PHASMOPHOBIA: &str = "Phasmophobia save";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    Text,
    Int,
    Float,
    Bool,
}

#[derive(Clone, Debug)]
struct Leaf {
    start: usize,
    end: usize,
    tok: Tok,
}

#[derive(Clone, Debug)]
struct Es3Backend {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for Es3Backend {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let l = &self.leaves[*id];
            let text = match (l.tok, v) {
                (Tok::Text, Value::String(_)) => serde_json::to_string(v).map_err(|e| e.to_string())?,
                (Tok::Int | Tok::Float, Value::Number(n)) => n.to_string(),
                (Tok::Bool, Value::Bool(b)) => b.to_string(),
                _ => return Err("value of the wrong type".into()),
            };
            p.add(l.start, l.end, text.into_bytes());
        }
        p.apply(&self.raw)
    }

    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        match (self.leaves[leaf].tok, v) {
            (Tok::Text, Value::String(_)) | (Tok::Bool, Value::Bool(_)) => Ok(()),
            (Tok::Int, Value::Number(n)) if n.is_i64() || n.is_u64() => Ok(()),
            (Tok::Int, _) => Err("a whole number is needed here".into()),
            (Tok::Float, Value::Number(_)) => Ok(()),
            (Tok::Float, _) => Err("a number is needed here".into()),
            (Tok::Text, _) => Err("text is needed here".into()),
            (Tok::Bool, _) => Err("true or false is needed here".into()),
        }
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

struct Scan<'a> {
    d: &'a [u8],
    at: usize,
    leaves: Vec<Leaf>,
    editable: HashMap<String, usize>,
    depth: usize,
}

impl Scan<'_> {
    fn ws(&mut self) {
        while self.d.get(self.at).is_some_and(|b| b.is_ascii_whitespace()) {
            self.at += 1;
        }
    }

    fn eat(&mut self, b: u8) -> Option<()> {
        self.ws();
        (self.d.get(self.at) == Some(&b)).then(|| self.at += 1)
    }

    /// A quoted string starting at `at`; returns its decoded text.
    fn string(&mut self) -> Option<String> {
        let start = self.at;
        self.at += 1;
        loop {
            match *self.d.get(self.at)? {
                b'\\' => self.at += 2,
                b'"' => break,
                _ => self.at += 1,
            }
        }
        self.at += 1;
        serde_json::from_slice::<String>(self.d.get(start..self.at)?).ok()
    }

    fn leaf(&mut self, ptr: &str, start: usize, tok: Tok) {
        self.editable.insert(ptr.to_string(), self.leaves.len());
        self.leaves.push(Leaf { start, end: self.at, tok });
    }

    fn value(&mut self, ptr: &str) -> Option<Value> {
        self.ws();
        let start = self.at;
        match *self.d.get(self.at)? {
            b'{' => {
                self.depth += 1;
                if self.depth > 64 {
                    return None;
                }
                self.at += 1;
                let mut map = Map::new();
                loop {
                    self.ws();
                    match *self.d.get(self.at)? {
                        b'}' => {
                            self.at += 1;
                            break;
                        }
                        b',' => {
                            self.at += 1;
                            continue;
                        }
                        _ => {}
                    }
                    // A key that is itself an object or array: shown as its text.
                    let key = if self.d[self.at] == b'"' {
                        self.string()?
                    } else {
                        let from = self.at;
                        let (leaves, editable) = (self.leaves.len(), self.editable.clone());
                        self.value("")?;
                        self.leaves.truncate(leaves);
                        self.editable = editable;
                        String::from_utf8_lossy(&self.d[from..self.at]).into_owned()
                    };
                    self.eat(b':')?;
                    let key = crate::tree::unique_key(&map, &key);
                    let v = self.value(&child(ptr, &key))?;
                    map.insert(key, v);
                }
                self.depth -= 1;
                Some(Value::Object(map))
            }
            b'[' => {
                self.depth += 1;
                if self.depth > 64 {
                    return None;
                }
                self.at += 1;
                let mut list = vec![];
                loop {
                    self.ws();
                    match *self.d.get(self.at)? {
                        b']' => {
                            self.at += 1;
                            break;
                        }
                        b',' => {
                            self.at += 1;
                            continue;
                        }
                        _ => {}
                    }
                    let v = self.value(&format!("{ptr}/{}", list.len()))?;
                    list.push(v);
                }
                self.depth -= 1;
                Some(Value::Array(list))
            }
            b'"' => {
                let s = self.string()?;
                self.leaf(ptr, start, Tok::Text);
                Some(Value::String(s))
            }
            _ => {
                while self.d.get(self.at).is_some_and(|b| !matches!(b, b',' | b'}' | b']' | b':') && !b.is_ascii_whitespace()) {
                    self.at += 1;
                }
                let t = std::str::from_utf8(&self.d[start..self.at]).ok()?;
                match t {
                    "true" | "false" => {
                        self.leaf(ptr, start, Tok::Bool);
                        Some(Value::Bool(t == "true"))
                    }
                    "null" => Some(Value::Null),
                    _ => {
                        let n: Number = serde_json::from_str(t).ok()?;
                        let whole = !t.contains(['.', 'e', 'E']);
                        self.leaf(ptr, start, if whole { Tok::Int } else { Tok::Float });
                        Some(Value::Number(n))
                    }
                }
            }
        }
    }
}

/// Text that has ES3's `"__type"` entries but isn't strict JSON.
pub fn parse(text: &str) -> Option<TreeDoc> {
    let t = text.trim_start();
    if !t.starts_with('{') || !t.contains("\"__type\"") {
        return None;
    }
    let mut s = Scan { d: text.as_bytes(), at: text.len() - t.len(), leaves: vec![], editable: HashMap::new(), depth: 0 };
    let view = s.value("")?;
    s.ws();
    if s.at != text.len() {
        return None;
    }
    // Every entry must be ES3's {"__type", "value"} pair.
    let entries = view.as_object()?;
    if entries.is_empty() || !entries.values().all(|v| v.get("__type").is_some_and(Value::is_string)) {
        return None;
    }
    // The type names stay as they are.
    s.editable.retain(|p, _| !p.ends_with("/__type"));
    let kind = if view.pointer("/PlayersMoney/value").is_some() { KIND_PHASMOPHOBIA } else { KIND };
    Some(TreeDoc::new(view, s.editable, kind, Box::new(Es3Backend { raw: text.as_bytes().to_vec(), leaves: s.leaves })))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"PlayersMoney":{"__type":"int","value":250},"timeInDark":{"__type":"float","value":12.5},"Name":{"__type":"string","value":"a \"b\""},"Seen":{"__type":"bool","value":false},"Outfit":{"__type":"PlayerOutfit,Assembly-CSharp","value":{"Items":{{"low":2048,"high":0}:{"guid":{"value":"a8"},"materialOption":0},{"low":4,"high":0}:{"materialOption":3}}}},"Ghosts":{"__type":"Dictionary","value":{"Demon":4,"Oni":7}}}"#;

    #[test]
    fn reads_object_keys_and_edits_in_place() {
        assert!(serde_json::from_str::<Value>(SAMPLE).is_err(), "the sample must not be strict JSON");
        let mut doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.kind_name, KIND_PHASMOPHOBIA);
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        let v = |doc: &TreeDoc, p: &str| doc.view.pointer(p).cloned();
        assert_eq!(v(&doc, "/PlayersMoney/value"), Some(Value::from(250)));
        assert_eq!(v(&doc, "/Name/value"), Some(Value::from("a \"b\"")));
        assert_eq!(v(&doc, "/Ghosts/value/Oni"), Some(Value::from(7)));
        let items = doc.view.pointer("/Outfit/value/Items").unwrap().as_object().unwrap();
        assert_eq!(items.keys().collect::<Vec<_>>(), [r#"{"low":2048,"high":0}"#, r#"{"low":4,"high":0}"#]);
        assert!(!doc.editable.contains_key("/PlayersMoney/__type"));
        doc.set("/PlayersMoney/value", Value::from(999_999)).unwrap();
        doc.set("/Ghosts/value/Oni", Value::from(8)).unwrap();
        doc.set("/Seen/value", Value::Bool(true)).unwrap();
        doc.set("/Name/value", Value::from("x\"y")).unwrap();
        doc.set("/Outfit/value/Items/{\"low\":4,\"high\":0}/materialOption", Value::from(1)).unwrap();
        assert!(doc.set("/PlayersMoney/value", Value::from(1.5)).is_err());
        assert!(doc.set("/PlayersMoney/value", Value::from("many")).is_err());
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        let want = SAMPLE
            .replace("\"value\":250", "\"value\":999999")
            .replace("\"Oni\":7", "\"Oni\":8")
            .replace("\"value\":false", "\"value\":true")
            .replace(r#""a \"b\"""#, r#""x\"y""#)
            .replace("\"materialOption\":3", "\"materialOption\":1");
        assert_eq!(out, want);
        assert_eq!(parse(&out).unwrap().view.pointer("/PlayersMoney/value"), Some(&Value::from(999_999)));
    }

    #[test]
    fn other_text_is_left_alone() {
        assert!(parse("{\"a\":1}").is_none());
        assert!(parse("{\"a\":{\"__type\":\"int\",\"value\":1}} trailing").is_none());
        assert!(parse("{\"a\":{\"__type\":\"int\",\"value\":1},\"b\":2}").is_none());
        let other = parse("{\"a\":{\"__type\":\"int\",\"value\":1},{\"k\":1}:{\"__type\":\"int\",\"value\":2}}").unwrap();
        assert_eq!(other.kind_name, KIND);
    }

    /// Research aid: SAVE_EDITOR_ES3 = encrypted saves separated by `|` (Phasmophobia,
    /// Lethal Company, R.E.P.O.): each must open, write back unchanged and take a new number.
    #[test]
    #[ignore]
    fn real_files() {
        use crate::formats::{decode, encode, Content};
        for path in std::env::var("SAVE_EDITOR_ES3").unwrap().split('|').filter(|s| !s.is_empty()) {
            let raw = std::fs::read(path).unwrap();
            let mut d = decode(&raw);
            // JSON is written out again in the editor's own spacing, so compare what it says.
            let same = encode(&d.layers, &d.content).unwrap();
            assert!(same == raw || crate::formats::same_data(&decode(&same).content, &d.content), "{path}: round trip");
            let ptr = ["/PlayersMoney/value", "/GroupCredits/value", "/PlayerXPNum/value", "/dictionaryOfDictionaries/value/runStats/currency"];
            let (kind, hit) = match &mut d.content {
                Content::Tree(t) => {
                    let hit = ptr.iter().find(|p| t.view.pointer(p).is_some()).unwrap();
                    t.set(hit, Value::from(4321)).unwrap();
                    (t.kind_name.clone(), *hit)
                }
                Content::Json { value, .. } => {
                    let hit = ptr.iter().find(|p| value.pointer(p).is_some()).unwrap();
                    *value.pointer_mut(hit).unwrap() = Value::from(4321);
                    ("JSON".to_string(), *hit)
                }
                _ => panic!("{path}: not opened"),
            };
            let again = decode(&encode(&d.layers, &d.content).unwrap());
            let got = match &again.content {
                Content::Tree(t) => t.view.pointer(hit).cloned(),
                Content::Json { value, .. } => value.pointer(hit).cloned(),
                _ => None,
            };
            assert_eq!(got, Some(Value::from(4321)), "{path}");
            println!("{path}: {kind}, {} layer(s), {hit} edited", d.layers.len());
        }
    }
}
