//! Slime Rancher (the first game, 1.4) saves: Monomi Park's tagged records.
//!
//! ```text
//! record = tag (7-bit length + ASCII), i32 version, 00 30, body, 00 40
//! ```
//!
//! Bodies have no lengths, so only two records are read, found by their exact header bytes:
//! the player (`SRPL` version 14: i32 health, energy, radiation, newbucks, slime keys, ...)
//! and the load-screen summary (`SRGSUMM` version 4: game version text, i32, i32, newbucks),
//! whose newbucks are kept equal to the player's. Worked out from public speedrun saves; the
//! record framing matches SavePrism's notes on Slime Rancher 2 (MIT).

use crate::tree::{Backend, Patches, TreeDoc};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Slime Rancher save";
const ROOT: &[u8] = b"\x06SRGAME";
const PLAYER: &[u8] = b"\x04SRPL\x0e\x00\x00\x00\x00\x30";
const SUMMARY: &[u8] = b"\x07SRGSUMM\x04\x00\x00\x00\x00\x30";

const FIELDS: [(&str, usize, i64); 5] = [
    ("Newbucks", 12, 999_999_999),
    ("Health", 0, 1000),
    ("Energy", 4, 1000),
    ("Radiation", 8, 1000),
    ("Slime keys", 16, 999),
];

fn i32at(d: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn find_once(d: &[u8], pat: &[u8]) -> Option<usize> {
    let mut it = d.windows(pat.len()).enumerate().filter(|(_, w)| *w == pat).map(|(i, _)| i);
    let first = it.next()?;
    it.next().is_none().then_some(first)
}

#[derive(Clone, Debug)]
struct Save {
    raw: Vec<u8>,
    player: usize,
    /// Where the summary's newbucks are.
    summary_money: Option<usize>,
}

fn open(d: &[u8]) -> Option<Save> {
    if !d.starts_with(ROOT) {
        return None;
    }
    let player = find_once(d, PLAYER)? + PLAYER.len();
    i32at(d, player + 20)?;
    let summary_money = find_once(d, SUMMARY).and_then(|s| {
        let at = s + SUMMARY.len();
        let n = *d.get(at)? as usize;
        (n < 0x80).then_some(at + 1 + n + 8)
    });
    // The summary's copy only counts when it agrees with the player's.
    let summary_money = summary_money.filter(|&at| i32at(d, at) == i32at(d, player + 12));
    Some(Save { raw: d.to_vec(), player, summary_money })
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(ROOT)
}

#[derive(Clone, Debug)]
struct B(Save);

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut p = Patches::default();
        for (id, v) in changes {
            let n = v.as_i64().and_then(|n| i32::try_from(n).ok()).ok_or("Must be a whole number")?;
            let at = self.0.player + FIELDS[*id].1;
            p.add(at, at + 4, n.to_le_bytes().to_vec());
            if *id == 0 {
                if let Some(s) = self.0.summary_money {
                    p.add(s, s + 4, n.to_le_bytes().to_vec());
                }
            }
        }
        p.apply(&self.0.raw)
    }
    fn check(&self, leaf: usize, v: &Value) -> Result<(), String> {
        let max = FIELDS.get(leaf).map_or(0, |f| f.2);
        match v.as_i64() {
            Some(n) if (0..=max).contains(&n) => Ok(()),
            _ => Err(format!("Must be a whole number from 0 to {max}")),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    let s = open(d)?;
    let mut player = Map::new();
    let mut edit = HashMap::new();
    for (i, (label, off, _)) in FIELDS.iter().enumerate() {
        player.insert(label.to_string(), Value::from(i32at(d, s.player + off)?));
        edit.insert(format!("/Player/{label}"), i);
    }
    let mut root = Map::new();
    root.insert("Player".into(), Value::Object(player));
    Some(TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B(s))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = ROOT.to_vec();
        d.extend(b"\x0b\x00\x00\x00\x00\x30\x05slot1");
        d.extend(SUMMARY);
        d.extend(b"\x061.4.0d");
        d.extend([0, 0, 0, 0, 3, 0, 0, 0]);
        d.extend(2760i32.to_le_bytes());
        d.extend(b"\x00\x40 world ");
        d.extend(PLAYER);
        for n in [100i32, 150, 0, 2760, 1, 3760] {
            d.extend(n.to_le_bytes());
        }
        d.extend(b"\x00\x40\x00\x40");
        d
    }

    #[test]
    fn reads_and_edits_newbucks() {
        let d = sample();
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Player/Newbucks").unwrap(), 2760);
        assert_eq!(doc.view.pointer("/Player/Energy").unwrap(), 150);
        doc.set("/Player/Newbucks", Value::from(500_000)).unwrap();
        let out = doc.to_bytes().unwrap();
        let back = parse(&out).unwrap();
        assert_eq!(back.view.pointer("/Player/Newbucks").unwrap(), 500_000);
        let s = out.windows(SUMMARY.len()).position(|w| w == SUMMARY).unwrap() + SUMMARY.len() + 15;
        assert_eq!(i32at(&out, s), Some(500_000));
    }
}
