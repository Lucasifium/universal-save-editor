//! Persona 3 Reload (and Episode Aigis' shared container). A Steam `SaveDataNNN.sav` is an
//! Unreal GVAS file run through a fixed XOR key and a bit permutation (layer in
//! `formats.rs`, `steam_decrypt` below). Inside, the game state is one long run of
//! `SaveDataArea` UInt32 properties, each carrying its own array index; a value is edited in
//! place and records for zero values are simply absent. Index positions are from the
//! MIT-licensed P3R Save Lab (Luckyseer/p3r-save-editor) and, for the decryption,
//! P3R-Save-EnDecryptor (illusion0001, MIT); both are credited in THIRD_PARTY_NOTICES.md.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::collections::HashMap;

const KEY: &[u8] = b"ae5zeitaix1joowooNgie3fahP5Ohph";
pub const STEAM_MAGIC: [u8; 4] = [0x15, 0x00, 0x65, 0x0b];
const CLASS: &[u8] = b"/Script/xrd777.XRD777SaveGame\0";

fn permute(v: u8) -> u8 {
    ((v >> 4) & 0x03) | ((v & 0x03) << 4) | (v & 0xcc)
}

/// Steam file -> GVAS bytes (None when it isn't one).
pub fn steam_decrypt(data: &[u8]) -> Option<Vec<u8>> {
    if !data.starts_with(&STEAM_MAGIC) {
        return None;
    }
    let plain: Vec<u8> = data.iter().enumerate().map(|(i, b)| permute(b ^ KEY[i % KEY.len()])).collect();
    plain.starts_with(b"GVAS").then_some(plain)
}

pub fn steam_encrypt(plain: &[u8]) -> Vec<u8> {
    plain.iter().enumerate().map(|(i, b)| permute(*b) ^ KEY[i % KEY.len()]).collect()
}

pub fn looks(d: &[u8]) -> bool {
    d.starts_with(b"GVAS") && d.windows(CLASS.len()).any(|w| w == CLASS)
}

fn prefix() -> Vec<u8> {
    let s = |t: &str| {
        let mut v = (t.len() as u32 + 1).to_le_bytes().to_vec();
        v.extend_from_slice(t.as_bytes());
        v.push(0);
        v
    };
    let mut p = s("SaveDataArea");
    p.extend(s("UInt32Property"));
    p.extend(4u32.to_le_bytes());
    p
}

/// array index -> offset of the 4-byte value
fn records(d: &[u8]) -> HashMap<u32, usize> {
    let p = prefix();
    let mut out = HashMap::new();
    let mut from = 0;
    while let Some(i) = d.get(from..).and_then(|s| s.windows(p.len()).position(|w| w == &p[..])) {
        let at = from + i + p.len();
        if at + 9 <= d.len() && d[at + 4] == 0 {
            out.insert(u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]), at + 5);
        }
        from += i + 1;
    }
    out
}

const PARTY: [&str; 10] = ["Protagonist", "Yukari", "Junpei", "Akihiko", "Mitsuru", "Fuuka", "Aigis", "Koromaru", "Ken", "Shinjiro"];
const LINKS: [(&str, &str); 22] = [
    ("Fool", "SEES"), ("Magician", "Kenji Tomochika"), ("Priestess", "Fuuka Yamagishi"), ("Empress", "Mitsuru Kirijo"),
    ("Emperor", "Hidetoshi Odagiri"), ("Hierophant", "Bunkichi & Mitsuko"), ("Lovers", "Yukari Takeba"),
    ("Chariot", "Kazushi Miyamoto"), ("Justice", "Chihiro Fushimi"), ("Hermit", "Maya"), ("Fortune", "Keisuke Hiraga"),
    ("Strength", "Yuko Nishiwaki"), ("Hanged Man", "Maiko Oohashi"), ("Death", "Pharos"), ("Temperance", "Bebe"),
    ("Devil", "President Tanaka"), ("Tower", "Mutatsu"), ("Star", "Mamoru Hayase"), ("Moon", "Nozomi Suemitsu"),
    ("Sun", "Akinari Kamiki"), ("Judgement", "Nyx Annihilation Team"), ("Aeon", "Aigis"),
];

/// The protagonist's HP word is at 13070 (13074 for the later save layout). The block's
/// own tag words confirm which one this file uses.
fn shift(r: &HashMap<u32, usize>, d: &[u8]) -> Option<u32> {
    let w = |i: u32| r.get(&i).map(|&o| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]));
    [0u32, 4].into_iter().find(|s| {
        let hp = 13070 + s;
        w(hp - 6) == Some(0x10100) && w(hp - 5) == Some(3) && w(hp - 4) == Some(0x2b4)
    })
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Persona 3 Reload save".into());
    }
    let r = records(d);
    let s = shift(&r, d).ok_or("this Persona 3 Reload save layout is not recognised")?;
    let at = |i: u32| r.get(&(i + s)).copied();
    let mut b = Builder::new(d);
    if let Some(o) = at(7257) {
        b.ranged(&["Protagonist", "Yen"], o, Kind::U32, 0, 9_999_999);
    }
    for (n, name) in PARTY.iter().enumerate() {
        let hp = if n == 0 { 13070 } else { 13246 + (n as u32 - 1) * 176 };
        let (level, exp) = if n == 0 { (hp + 4, hp + 5) } else { (hp + 17, hp + 18) };
        if let Some(o) = at(hp) {
            b.ranged(&["Party", name, "HP"], o, Kind::U32, 0, 9999);
        }
        if let Some(o) = at(hp + 1) {
            b.ranged(&["Party", name, "SP"], o, Kind::U32, 0, 9999);
        }
        if let Some(o) = at(level) {
            b.ranged(&["Party", name, "Level"], o, Kind::U16, 1, 99);
        }
        if let Some(o) = at(exp) {
            b.ranged(&["Party", name, "Experience"], o, Kind::U32, 0, 9_999_999);
        }
    }
    for (i, name) in ["Academics", "Charm", "Courage"].iter().enumerate() {
        if let Some(o) = at(5352 + 2 * i as u32) {
            b.ranged(&["Social stats", name], o, Kind::U32, 0, 255);
        }
    }
    for (i, (arcana, who)) in LINKS.iter().enumerate() {
        if let Some(o) = at(5300 + 2 * i as u32) {
            b.ranged(&["Social links", &format!("{arcana} ({who}) rank")], o, Kind::U8, 0, 10);
        }
    }
    Ok(b.finish("Persona 3 Reload save", Checksum::None).with_note(
        "Yen, party HP, SP, level and experience, social stats and Social Link ranks. A value the game left at zero has no record in the file and is not listed.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(out: &mut Vec<u8>, idx: u32, val: u32) {
        out.extend(prefix());
        out.extend(idx.to_le_bytes());
        out.push(0);
        out.extend(val.to_le_bytes());
    }

    fn synthetic(shift: u32) -> Vec<u8> {
        let mut d = b"GVAS\x02\0\0\0".to_vec();
        d.extend_from_slice(CLASS);
        let hp = 13070 + shift;
        for (i, v) in [(hp - 6, 0x10100), (hp - 5, 3), (hp - 4, 0x2b4), (hp, 700), (hp + 1, 400), (hp + 4, 50), (hp + 5, 1234)] {
            record(&mut d, i, v);
        }
        record(&mut d, 7257 + shift, 5000);
        d
    }

    #[test]
    fn steam_layer_round_trips() {
        let plain = synthetic(0);
        let enc = steam_encrypt(&plain);
        assert!(enc.starts_with(&STEAM_MAGIC));
        assert_eq!(steam_decrypt(&enc).unwrap(), plain);
    }

    #[test]
    fn reads_both_layouts() {
        for shift in [0, 4] {
            let t = parse(&synthetic(shift)).expect("parses");
            assert_eq!(t.view["Protagonist"]["Yen"], 5000, "yen at shift {shift}");
            assert_eq!(t.view["Party"]["Protagonist"]["Level"], 50);
            assert_eq!(t.view["Party"]["Protagonist"]["HP"], 700);
        }
        assert!(parse(b"GVAS").is_err());
    }
}
