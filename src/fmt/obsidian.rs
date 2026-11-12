//! Obsidian's Unity saves (Tyranny; Pillars of Eternity use the same layout): the `.savegame` is
//! a zip whose `MobileObjects.save` is an XML list of every object in the game. The party's
//! money and camping supplies and each character's experience, attributes and skills are plain
//! numbers inside it; they are edited in place and the rest of the text is written back exactly.

use crate::tree::{Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Obsidian Unity save";

const HEAD: &str = "<SingleArray name=\"Root\" type=\"ObjectPersistencePacket[]";

#[derive(Clone, Debug)]
struct Leaf {
    /// Every place the number is written (money is kept twice).
    spans: Vec<(usize, usize)>,
}

#[derive(Clone, Debug)]
struct Back {
    raw: Vec<u8>,
    leaves: Vec<Leaf>,
}

impl Backend for Back {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let Value::Number(n) = v else { return Err("this value can't be written".into()) };
            let text = match n.as_i64() {
                Some(i) => i.to_string(),
                None => return Err("this value must be a whole number".into()),
            };
            for &(s, e) in &self.leaves[*id].spans {
                p.add(s, e, text.clone().into_bytes());
            }
        }
        p.apply(&self.raw)
    }

    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn looks(text: &str) -> bool {
    text.trim_start_matches('\u{feff}').starts_with(HEAD)
}

struct Tag<'a> {
    name: Option<&'a str>,
    ty: Option<&'a str>,
    value: Option<(usize, usize)>,
}

fn attr<'a>(t: &'a str, key: &str, base: usize) -> Option<(&'a str, usize, usize)> {
    let pat = format!(" {key}=\"");
    let i = t.find(&pat)? + pat.len();
    let j = t[i..].find('"')? + i;
    Some((&t[i..j], base + i, base + j))
}

fn num_text(s: &str) -> Option<i64> {
    s.parse::<i64>().ok().or_else(|| s.parse::<f64>().ok().filter(|f| f.fract() == 0.0 && f.abs() < 1e15).map(|f| f as i64))
}

pub fn parse(text: &str) -> Result<TreeDoc, String> {
    if !looks(text) {
        return Err("not an Obsidian save".into());
    }
    let mut leaves: Vec<Leaf> = vec![];
    let mut editable: HashMap<String, usize> = HashMap::new();
    let mut party = Map::new();
    // (name, fields) in the order the characters appear.
    let mut chars: Vec<(String, Map<String, Value>)> = vec![];
    let mut cur_obj = String::new();
    let mut comp = String::new();
    let mut key: Option<String> = None;
    let mut money: Option<usize> = None;
    let mut pos = 0;
    while let Some(i) = text[pos..].find("<Simple ") {
        let s = pos + i;
        let Some(e) = text[s..].find("/>").map(|e| e + s) else { break };
        pos = e + 2;
        let t = &text[s..e];
        let tag = Tag {
            name: attr(t, "name", s).map(|a| a.0),
            ty: attr(t, "type", s).map(|a| a.0),
            value: attr(t, "value", s).map(|a| (a.1, a.2)),
        };
        let Some((vs, ve)) = tag.value else { continue };
        let val = &text[vs..ve];
        match tag.name {
            Some("ObjectName") => {
                cur_obj = val.trim_end_matches("(Clone)").to_string();
                key = None;
            }
            Some("TypeString") => {
                comp = val.split(',').next().unwrap_or("").to_string();
                key = None;
            }
            Some("amount") if key.as_deref() == Some("currencyTotalValue") && comp == "Game.PlayerInventory" => {
                if let Some(n) = num_text(val) {
                    money = Some(leaves.len());
                    leaves.push(Leaf { spans: vec![(vs, ve)] });
                    party.insert("Money".into(), Value::from(n));
                    editable.insert("/Party/Money".into(), leaves.len() - 1);
                }
            }
            Some("InitialValue") if key.as_deref() == Some("currencyTotalValue") => {
                if let Some(m) = money.take() {
                    leaves[m].spans.push((vs, ve));
                }
            }
            None if tag.ty.is_none() => key = Some(val.to_string()),
            None => {
                let k = key.take().unwrap_or_default();
                let Some(n) = num_text(val) else { continue };
                let int = tag.ty.is_some_and(|t| t.starts_with("System.Int32"));
                if comp == "Game.PlayerInventory" && k == "campingSupplies" && int {
                    leaves.push(Leaf { spans: vec![(vs, ve)] });
                    party.insert("Camping supplies".into(), Value::from(n));
                    editable.insert("/Party/Camping supplies".into(), leaves.len() - 1);
                } else if comp == "Game.CharacterStats" && int && !cur_obj.is_empty() {
                    let shown = if k == "Experience" || k == "AttributePoints" || k == "TalentPoints" || (k.len() > 4 && (k.starts_with("Base") || k.ends_with("Skill"))) {
                        Some(k.strip_prefix("Base").unwrap_or(&k).to_string())
                    } else {
                        None
                    };
                    if let Some(label) = shown {
                        let at = match chars.iter().rposition(|(n, _)| *n == cur_obj) {
                            Some(a) if !chars[a].1.contains_key(&label) => a,
                            _ => {
                                chars.push((cur_obj.clone(), Map::new()));
                                chars.len() - 1
                            }
                        };
                        leaves.push(Leaf { spans: vec![(vs, ve)] });
                        editable.insert(format!("/Characters/{at}/{label}"), leaves.len() - 1);
                        chars[at].1.insert(label, Value::from(n));
                    }
                }
            }
            _ => {}
        }
    }
    if party.is_empty() {
        return Err("no party data found".into());
    }
    let list: Vec<Value> = chars
        .into_iter()
        .map(|(n, mut m)| {
            let mut o = Map::new();
            o.insert("Name".into(), Value::String(n));
            o.append(&mut m);
            Value::Object(o)
        })
        .collect();
    let mut root = Map::new();
    root.insert("Party".into(), Value::Object(party));
    root.insert("Characters".into(), Value::Array(list));
    Ok(TreeDoc::new(Value::Object(root), editable, KIND, Box::new(Back { raw: text.as_bytes().to_vec(), leaves })))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\u{feff}<SingleArray name=\"Root\" type=\"ObjectPersistencePacket[], Assembly-CSharp-firstpass\">\r\n<Simple name=\"ObjectName\" value=\"Hero(Clone)\" />\r\n<Simple name=\"TypeString\" value=\"Game.CharacterStats, Assembly-CSharp\" />\r\n<Item>\r\n<Simple value=\"BaseMight\" />\r\n<Simple type=\"System.Int32, mscorlib\" value=\"13\" />\r\n</Item>\r\n<Item>\r\n<Simple value=\"Experience\" />\r\n<Simple type=\"System.Int32, mscorlib\" value=\"500\" />\r\n</Item>\r\n<Simple name=\"TypeString\" value=\"Game.PlayerInventory, Assembly-CSharp\" />\r\n<Item><Simple value=\"currencyTotalValue\" />\r\n<Complex type=\"CurrencyValue\"><Simple name=\"amount\" value=\"122414\" />\r\n<Simple name=\"InitialValue\" type=\"System.Single, mscorlib\" value=\"122414\" /></Complex></Item>\r\n<Item><Simple value=\"campingSupplies\" />\r\n<Simple type=\"System.Int32, mscorlib\" value=\"4\" /></Item>\r\n</SingleArray>";

    #[test]
    fn edits_money_and_stats() {
        let mut doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), SAMPLE.as_bytes());
        assert_eq!(doc.view.pointer("/Party/Money").unwrap(), 122414);
        assert_eq!(doc.view.pointer("/Characters/0/Might").unwrap(), 13);
        doc.set("/Party/Money", Value::from(9)).unwrap();
        doc.set("/Characters/0/Experience", Value::from(7)).unwrap();
        let out = String::from_utf8(doc.to_bytes().unwrap()).unwrap();
        assert_eq!(out, SAMPLE.replace("122414", "9").replace("\"500\"", "\"7\""));
    }
}
