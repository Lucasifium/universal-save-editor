//! Need for Speed: Most Wanted (2005) profiles: a fixed-size file (63,596 bytes on PC, 62,689 on
//! PS2) whose last 16 bytes are the MD5 of everything from byte 0x34 up to them.
//!
//! Where the profile name, the money and the pursuit bounty are kept, and the hash range, are as
//! published by nfsmw-save-editor (x07x08, Unlicense).

use crate::tree::{Backend, Patches, TreeDoc};
use md5::{Digest, Md5};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const KIND: &str = "Need for Speed: Most Wanted save";
const PC: usize = 63596;
const PS2: usize = 62689;
const MONEY: usize = 0x4039;
const NAME: usize = 0x5A31;

fn bounty_at(len: usize) -> usize {
    if len == PC {
        0xE865
    } else {
        0xE8A1
    }
}

fn hash(d: &[u8]) -> [u8; 16] {
    Md5::digest(&d[0x34..d.len() - 16]).into()
}

pub fn looks(d: &[u8]) -> bool {
    (d.len() == PC || d.len() == PS2) && hash(d)[..] == d[d.len() - 16..]
}

#[derive(Clone, Debug)]
struct B(Vec<u8>);

impl Backend for B {
    fn write(&self, _view: &Value, changes: &[(usize, Value)]) -> Result<Vec<u8>, String> {
        let mut out = self.0.clone();
        let mut p = Patches::default();
        for (id, v) in changes {
            let n = v.as_u64().ok_or("Must be a whole number")?;
            let at = if *id == 0 { MONEY } else { bounty_at(out.len()) };
            p.add(at, at + 4, (n as u32).to_le_bytes().to_vec());
        }
        out = p.apply(&out)?;
        let h = hash(&out);
        let n = out.len();
        out[n - 16..].copy_from_slice(&h);
        Ok(out)
    }
    fn check(&self, _leaf: usize, v: &Value) -> Result<(), String> {
        match v.as_u64() {
            Some(n) if n <= u32::MAX as u64 => Ok(()),
            _ => Err("Must be a whole number from 0 to 4,294,967,295".into()),
        }
    }
    fn clone_box(&self) -> Box<dyn Backend> {
        Box::new(self.clone())
    }
}

fn u32at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

pub fn parse(d: &[u8]) -> Option<TreeDoc> {
    if !looks(d) {
        return None;
    }
    let name: String = d[NAME..NAME + 8].iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
    let mut m = Map::new();
    m.insert("Profile name".into(), Value::String(name));
    m.insert("Money".into(), Value::from(u32at(d, MONEY)));
    m.insert("Pursuit bounty".into(), Value::from(u32at(d, bounty_at(d.len()))));
    let mut root = Map::new();
    root.insert("Profile".into(), Value::Object(m));
    let edit = HashMap::from([("/Profile/Money".to_string(), 0usize), ("/Profile/Pursuit bounty".to_string(), 1)]);
    Some(TreeDoc::new(Value::Object(root), edit, KIND, Box::new(B(d.to_vec()))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_updates_the_hash() {
        let mut d = vec![0u8; PC];
        d[NAME..NAME + 6].copy_from_slice(b"Razor\0");
        d[MONEY..MONEY + 4].copy_from_slice(&500u32.to_le_bytes());
        let h = hash(&d);
        let n = d.len();
        d[n - 16..].copy_from_slice(&h);
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        assert_eq!(doc.view.pointer("/Profile/Money").unwrap(), 500);
        doc.set("/Profile/Money", Value::from(9_999_999u32)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert!(looks(&out));
        assert_eq!(u32at(&out, MONEY), 9_999_999);
    }
}
