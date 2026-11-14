//! Pokémon Sword / Shield and Scarlet / Violet (Nintendo Switch) `main` saves. The file is a list of
//! keyed blocks, scrambled with a fixed pad and per-block xorshift streams and sealed with a SHA-256.
//! The scheme and the block keys follow PKHeX (GPL-3.0).

use super::pkm::{decode_text, name_of, names, Builder, Field, Finish, Gen, Slot, Txt};
use crate::tree::{esc, TreeDoc};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const INTRO: [u8; 64] = [
    0x9E, 0xC9, 0x9C, 0xD7, 0x0E, 0xD3, 0x3C, 0x44, 0xFB, 0x93, 0x03, 0xDC, 0xEB, 0x39, 0xB4, 0x2A, 0x19, 0x47, 0xE9, 0x63, 0x4B, 0xA2, 0x33, 0x44, 0x16, 0xBF, 0x82, 0xA2, 0xBA, 0x63, 0x55, 0xB6, 0x3D, 0x9D, 0xF2, 0x4B, 0x5F, 0x7B, 0x6A, 0xB2,
    0x62, 0x1D, 0xC2, 0x1B, 0x68, 0xE5, 0xC8, 0xB5, 0x3A, 0x05, 0x90, 0x00, 0xE8, 0xA8, 0x10, 0x3D, 0xE2, 0xEC, 0xF0, 0x0C, 0xB2, 0xED, 0x4F, 0x6D,
];
const OUTRO: [u8; 64] = [
    0xD6, 0xC0, 0x1C, 0x59, 0x8B, 0xC8, 0xB8, 0xCB, 0x46, 0xE1, 0x53, 0xFC, 0x82, 0x8C, 0x75, 0x75, 0x13, 0xE0, 0x45, 0xDF, 0x32, 0x69, 0x3C, 0x75, 0xF0, 0x59, 0xF8, 0xD9, 0xA2, 0x5F, 0xB2, 0x17, 0xE0, 0x80, 0x52, 0xDB, 0xEA, 0x89, 0x73, 0x99,
    0x75, 0x79, 0xAF, 0xCB, 0x2E, 0x80, 0x07, 0xE6, 0xF1, 0x26, 0xE0, 0x03, 0x0A, 0xE6, 0x6F, 0xF6, 0x41, 0xBF, 0x7E, 0x59, 0xC2, 0xAE, 0x55, 0xFD,
];
/// The fixed pad repeats every 0x7F bytes.
const PAD: [u8; 127] = [
    0xA0, 0x92, 0xD1, 0x06, 0x07, 0xDB, 0x32, 0xA1, 0xAE, 0x01, 0xF5, 0xC5, 0x1E, 0x84, 0x4F, 0xE3, 0x53, 0xCA, 0x37, 0xF4, 0xA7, 0xB0, 0x4D, 0xA0, 0x18, 0xB7, 0xC2, 0x97, 0xDA, 0x5F, 0x53, 0x2B, 0x75, 0xFA, 0x48, 0x16, 0xF8, 0xD4, 0x8A, 0x6F,
    0x61, 0x05, 0xF4, 0xE2, 0xFD, 0x04, 0xB5, 0xA3, 0x0F, 0xFC, 0x44, 0x92, 0xCB, 0x32, 0xE6, 0x1B, 0xB9, 0xB1, 0x2E, 0x01, 0xB0, 0x56, 0x53, 0x36, 0xD2, 0xD1, 0x50, 0x3D, 0xDE, 0x5B, 0x2E, 0x0E, 0x52, 0xFD, 0xDF, 0x2F, 0x7B, 0xCA, 0x63, 0x50,
    0xA4, 0x67, 0x5D, 0x23, 0x17, 0xC0, 0x52, 0xE1, 0xA6, 0x30, 0x7C, 0x2B, 0xB6, 0x70, 0x36, 0x5B, 0x2A, 0x27, 0x69, 0x33, 0xF5, 0x63, 0x7B, 0x36, 0x3F, 0x26, 0x9B, 0xA3, 0xED, 0x7A, 0x53, 0x00, 0xA4, 0x48, 0xB3, 0x50, 0x9E, 0x14, 0xA0, 0x52,
    0xDE, 0x7E, 0x10, 0x2B, 0x1B, 0x77, 0x6E,
];

// Block keys
const SWSH_PARTY: u32 = 0x2985_fe5d;
const SWSH_BOX: u32 = 0x0d66_012c;
const SWSH_ITEM: u32 = 0x1177_c2c4;
const SWSH_MISC: u32 = 0x1b88_2b09;
const SWSH_STATUS: u32 = 0xf25c_070e;
const SWSH_TIME: u32 = 0x8cbb_fd90;
const SWSH_LAYOUT: u32 = 0x1972_2c89;
const SV_PARTY: u32 = 0x3AA1_A9AD;
const SV_BOX: u32 = 0x0d66_012c;
const SV_ITEM: u32 = 0x21C9_BD44;
const SV_STATUS: u32 = 0xE3E8_9BD1;
const SV_TIME: u32 = 0xEDAF_F794;
const SV_LAYOUT: u32 = 0x1972_2c89;
const SV_MONEY: u32 = 0x4F35_D0DD;
const SV_LP: u32 = 0xADB4_FE17;
const SV_BP: u32 = 0x66A3_3824;

fn xor_pad(d: &mut [u8]) {
    for (i, b) in d.iter_mut().enumerate() {
        *b ^= PAD[i % 127];
    }
}

fn seal(payload: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(INTRO);
    h.update(payload);
    h.update(OUTRO);
    h.finalize().into()
}

struct XorShift {
    state: u32,
    counter: u8,
}

impl XorShift {
    fn new(seed: u32) -> Self {
        let mut s = seed;
        for _ in 0..seed.count_ones() {
            s = Self::adv(s);
        }
        XorShift { state: s, counter: 0 }
    }
    fn adv(mut s: u32) -> u32 {
        s ^= s << 2;
        s ^= s >> 15;
        s ^= s << 13;
        s
    }
    fn next(&mut self) -> u8 {
        let r = (self.state >> (self.counter * 8)) as u8;
        if self.counter == 3 {
            self.state = Self::adv(self.state);
            self.counter = 0;
        } else {
            self.counter += 1;
        }
        r
    }
    fn next32(&mut self) -> u32 {
        let b = [self.next(), self.next(), self.next(), self.next()];
        u32::from_le_bytes(b)
    }
}

fn type_size(t: u8) -> Option<usize> {
    Some(match t {
        3 | 8 | 12 => 1,
        9 | 13 => 2,
        10 | 14 | 16 => 4,
        11 | 15 | 17 => 8,
        _ => return None,
    })
}

struct Block {
    off: usize,
    len: usize,
}

/// Turn the sealed file into a plain stream (key, type, lengths, data; nothing scrambled) and a key index.
fn unpack(file: &[u8]) -> Result<(Vec<u8>, HashMap<u32, Block>), String> {
    let mut data = file[..file.len() - 32].to_vec();
    xor_pad(&mut data);
    let mut out = Vec::with_capacity(data.len());
    let mut idx = HashMap::new();
    let mut p = 0;
    let bad = || "damaged Pokémon save".to_string();
    while p < data.len() {
        if p + 5 > data.len() {
            return Err(bad());
        }
        let key = u32::from_le_bytes(data[p..p + 4].try_into().unwrap());
        p += 4;
        let mut xk = XorShift::new(key);
        let t = data[p] ^ xk.next();
        p += 1;
        out.extend_from_slice(&key.to_le_bytes());
        out.push(t);
        let (len, sub) = match t {
            1 | 2 => (0, 0),
            4 => {
                let n = u32::from_le_bytes(data.get(p..p + 4).ok_or_else(bad)?.try_into().unwrap()) ^ xk.next32();
                p += 4;
                out.extend_from_slice(&n.to_le_bytes());
                (n as usize, 0)
            }
            5 => {
                let n = u32::from_le_bytes(data.get(p..p + 4).ok_or_else(bad)?.try_into().unwrap()) ^ xk.next32();
                p += 4;
                let sub = data.get(p).ok_or_else(bad)? ^ xk.next();
                p += 1;
                out.extend_from_slice(&n.to_le_bytes());
                out.push(sub);
                (n as usize * type_size(sub).ok_or_else(bad)?, sub)
            }
            t => (type_size(t).ok_or_else(bad)?, 0),
        };
        let _ = sub;
        if p + len > data.len() {
            return Err(bad());
        }
        let off = out.len();
        out.extend(data[p..p + len].iter().map(|b| b ^ xk.next()));
        p += len;
        idx.insert(key, Block { off, len });
    }
    Ok((out, idx))
}

/// The reverse of [`unpack`]: scramble every block, apply the pad and add the seal.
fn finish(out: &mut Vec<u8>, _: &[usize]) -> Result<Vec<u8>, String> {
    let bad = || "damaged Pokémon save".to_string();
    let mut enc = Vec::with_capacity(out.len() + 32);
    let mut p = 0;
    while p < out.len() {
        let key = u32::from_le_bytes(out.get(p..p + 4).ok_or_else(bad)?.try_into().unwrap());
        p += 4;
        enc.extend_from_slice(&key.to_le_bytes());
        let mut xk = XorShift::new(key);
        let t = *out.get(p).ok_or_else(bad)?;
        p += 1;
        enc.push(t ^ xk.next());
        let len = match t {
            1 | 2 => 0,
            4 => {
                let n = u32::from_le_bytes(out.get(p..p + 4).ok_or_else(bad)?.try_into().unwrap());
                p += 4;
                enc.extend_from_slice(&(n ^ xk.next32()).to_le_bytes());
                n as usize
            }
            5 => {
                let n = u32::from_le_bytes(out.get(p..p + 4).ok_or_else(bad)?.try_into().unwrap());
                p += 4;
                let sub = *out.get(p).ok_or_else(bad)?;
                p += 1;
                enc.extend_from_slice(&(n ^ xk.next32()).to_le_bytes());
                enc.push(sub ^ xk.next());
                n as usize * type_size(sub).ok_or_else(bad)?
            }
            t => type_size(t).ok_or_else(bad)?,
        };
        let src = out.get(p..p + len).ok_or_else(bad)?;
        enc.extend(src.iter().map(|b| b ^ xk.next()));
        p += len;
    }
    xor_pad(&mut enc);
    let h = seal(&enc);
    enc.extend_from_slice(&h);
    Ok(enc)
}

pub fn looks(d: &[u8]) -> bool {
    if d.len() < 0x10_0000 || d.len() > 0x800_0000 {
        return false;
    }
    let mut p = d[..d.len() - 32].to_vec();
    // Cheap first test: the very first byte after the key must unscramble to a plausible type.
    xor_pad(&mut p[..64]);
    let key = u32::from_le_bytes(p[..4].try_into().unwrap());
    let t = p[4] ^ XorShift::new(key).next();
    if !matches!(t, 1 | 2 | 4 | 5 | 8..=17) {
        return false;
    }
    seal(&d[..d.len() - 32]) == d[d.len() - 32..]
}

fn rd32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

const LONG: usize = 0x22;

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Sword / Shield or Scarlet / Violet save".into());
    }
    let (raw, idx) = unpack(d)?;
    let sv = idx.contains_key(&SV_PARTY);
    let swsh = idx.contains_key(&SWSH_PARTY);
    if sv == swsh {
        return Err("unrecognised Pokémon save".into());
    }
    let g = if sv { Gen::G9 } else { Gen::G8 };
    let blk = |k: u32, min: usize| -> Result<usize, String> {
        match idx.get(&k) {
            Some(b) if b.len >= min => Ok(b.off),
            _ => Err("unrecognised Pokémon save".into()),
        }
    };
    let (status, party, boxes, items, layout, time) = if sv {
        (blk(SV_STATUS, 0x70)?, blk(SV_PARTY, 6 * 344 + 1)?, blk(SV_BOX, 32 * 30 * 344)?, blk(SV_ITEM, 3000 * 16)?, blk(SV_LAYOUT, 32 * LONG)?, blk(SV_TIME, 12)?)
    } else {
        (blk(SWSH_STATUS, 0xD4)?, blk(SWSH_PARTY, 6 * 344 + 1)?, blk(SWSH_BOX, 32 * 30 * 344)?, blk(SWSH_ITEM, 0x11F8 + 64 * 4)?, blk(SWSH_LAYOUT, 32 * LONG)?, blk(SWSH_TIME, 4)?)
    };
    let (ver_off, id_off, name_off) = if sv { (4, 0, 0x10) } else { (0xA4, 0xA0, 0xB0) };
    let ver = raw[status + ver_off];
    let kind = match (sv, ver) {
        (false, 44) => "Pokémon Sword",
        (false, 45) => "Pokémon Shield",
        (false, _) => "Pokémon Sword / Shield",
        (true, 50) => "Pokémon Scarlet",
        (true, 51) => "Pokémon Violet",
        (true, _) => "Pokémon Scarlet / Violet",
    };
    let mut b = Builder::default();
    let mut root = Map::new();
    let mut tr = Map::new();
    b.text(&mut tr, "/Trainer", "Name", &raw, status + name_off, 0x1A, 12, Txt::U16Z);
    b.add(&mut tr, "/Trainer", "Gender", Value::from(if raw[status + ver_off + 1] == 1 { "Girl" } else { "Boy" }), Some(Field::Gender { off: status + ver_off + 1 }));
    tr.insert("Trainer ID".into(), Value::from(u16::from_le_bytes([raw[status + id_off], raw[status + id_off + 1]])));
    tr.insert("Secret ID".into(), Value::from(u16::from_le_bytes([raw[status + id_off + 2], raw[status + id_off + 3]])));
    if sv {
        b.int(&mut tr, "/Trainer", "Money", &raw, blk(SV_MONEY, 4)?, 4, 9_999_999);
        if let Ok(o) = blk(SV_LP, 4) {
            b.int(&mut tr, "/Trainer", "League Points", &raw, o, 4, 9_999_999);
        }
        if let Ok(o) = blk(SV_BP, 4) {
            b.int(&mut tr, "/Trainer", "Blueberry Points", &raw, o, 4, 9_999_999);
        }
        for (i, k) in ["Play time (hours)", "Play time (minutes)", "Play time (seconds)"].iter().enumerate() {
            b.int(&mut tr, "/Trainer", k, &raw, time + i * 4, 4, if i == 0 { 99_999 } else { 59 });
        }
    } else {
        let misc = blk(SWSH_MISC, 0x120)?;
        b.int(&mut tr, "/Trainer", "Money", &raw, misc + 4, 4, 9_999_999);
        b.int(&mut tr, "/Trainer", "Watts", &raw, status + 0xD0, 4, 9_999_999);
        b.int(&mut tr, "/Trainer", "Battle Points", &raw, misc + 0x11C, 2, 9_999);
        b.int(&mut tr, "/Trainer", "Gym badges", &raw, misc, 1, 8);
        b.int(&mut tr, "/Trainer", "Play time (hours)", &raw, time, 2, 999);
        b.int(&mut tr, "/Trainer", "Play time (minutes)", &raw, time + 2, 1, 59);
        b.int(&mut tr, "/Trainer", "Play time (seconds)", &raw, time + 3, 1, 59);
    }
    root.insert("Trainer".into(), Value::Object(tr));

    let n = (raw[party + 6 * g.party()] as usize).min(6);
    let mut pm = Map::new();
    for i in 0..n {
        let key = format!("Pokémon {}", i + 1);
        let ptr = format!("/Party/{}", esc(&key));
        if let Some(v) = b.mon(g, &raw, party + i * g.party(), true, &ptr) {
            pm.insert(key, v);
        }
    }
    root.insert("Party".into(), Value::Object(pm));

    let mut bag = Map::new();
    if sv {
        let t = names();
        const POUCHES: [(&str, u32); 11] = [
            ("Medicine", 999),
            ("Poké Balls", 999),
            ("Battle items", 999),
            ("Berries", 999),
            ("Items", 999),
            ("TMs", 1),
            ("Treasures", 999),
            ("Picnic ingredients", 999),
            ("Key items", 1),
            ("Materials", 999),
            ("Recipes", 1),
        ];
        let mut pockets: Vec<Map<String, Value>> = vec![Map::new(); POUCHES.len()];
        for i in 0..3000 {
            let at = items + i * 16;
            let (pouch, count) = (rd32(&raw, at) as usize, rd32(&raw, at + 4));
            if count == 0 || pouch >= POUCHES.len() || count > 1_000_000 {
                continue;
            }
            let title = POUCHES[pouch].0;
            let nm = name_of(&t.items, i);
            if pockets[pouch].contains_key(&nm) {
                continue;
            }
            let sp = format!("/Bag/{}/{}", esc(title), esc(&nm));
            let mut s = Map::new();
            b.add(&mut s, &sp, "Quantity", Value::from(count), Some(Field::ItemQty { off: at, slot: Slot::Sv, max: POUCHES[pouch].1 }));
            pockets[pouch].insert(nm, Value::Object(s));
        }
        for (i, m) in pockets.into_iter().enumerate() {
            if !m.is_empty() {
                bag.insert(POUCHES[i].0.into(), Value::Object(m));
            }
        }
    } else {
        for &(title, off, slots, max) in &[
            ("Medicine", 0x0000usize, 60usize, 999u32),
            ("Poké Balls", 0x00F0, 30, 999),
            ("Battle items", 0x0168, 20, 999),
            ("Berries", 0x01B8, 80, 999),
            ("Items", 0x02F8, 550, 999),
            ("TMs & TRs", 0x0B90, 210, 1),
            ("Treasures", 0x0ED8, 100, 999),
            ("Candy", 0x1068, 100, 999),
            ("Key items", 0x11F8, 64, 1),
        ] {
            b.pocket(&mut bag, title, &raw, items + off, slots, Slot::Swsh, 4, max, g);
        }
    }
    root.insert("Bag".into(), Value::Object(bag));

    let mut bm = Map::new();
    for bx in 0..32 {
        let key = format!("Box {}", bx + 1);
        let bp = format!("/PC boxes/{}", esc(&key));
        let mut m = Map::new();
        let nm = layout + bx * LONG;
        if decode_text(Txt::U16Z, &raw[nm..nm + LONG]).is_some_and(|s| !s.is_empty()) {
            b.text(&mut m, &bp, "Box name", &raw, nm, LONG, 16, Txt::U16Z);
        }
        for s in 0..30 {
            let addr = boxes + (bx * 30 + s) * g.party();
            let k = format!("Slot {}", s + 1);
            let ptr = format!("{bp}/{}", esc(&k));
            if let Some(v) = b.mon(g, &raw, addr, true, &ptr) {
                m.insert(k, v);
            }
        }
        bm.insert(key, Value::Object(m));
    }
    root.insert("PC boxes".into(), Value::Object(bm));

    let note = "The save's seal is rebuilt automatically. Changing a Pokémon's level also sets its experience and stats. \
                Use the `main` file from your console (JKSV, Checkpoint) or from an emulator. \
                Only items you already own are listed in the Scarlet / Violet bag.";
    Ok(b.finish(g, raw, root, kind, Finish::Custom(finish, vec![]), note))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xorshift_stream_is_stable() {
        let mut x = XorShift::new(0x1234_5678);
        let a: Vec<u8> = (0..8).map(|_| x.next()).collect();
        let mut y = XorShift::new(0x1234_5678);
        assert_eq!(a, (0..8).map(|_| y.next()).collect::<Vec<u8>>());
        assert_eq!(XorShift::adv(1), 1 ^ 4 ^ ((1 ^ 4) >> 15) ^ ((((1 ^ 4) ^ ((1 ^ 4) >> 15)) << 13)));
    }

    /// Build a tiny sealed file by hand and take it apart again.
    #[test]
    fn pack_unpack_round_trip() {
        let mut plain = Vec::new();
        for (key, t, data) in [(0xAABB_CCDDu32, 10u8, vec![1, 2, 3, 4]), (0x1122_3344, 4, vec![9; 7]), (0x5566_7788, 5, vec![5; 6])] {
            plain.extend_from_slice(&key.to_le_bytes());
            plain.push(t);
            if t == 4 {
                plain.extend_from_slice(&(data.len() as u32).to_le_bytes());
            }
            if t == 5 {
                plain.extend_from_slice(&3u32.to_le_bytes());
                plain.push(9);
            }
            plain.extend_from_slice(&data);
        }
        plain.extend_from_slice(&0x0102_0304u32.to_le_bytes());
        plain.push(1);
        let enc = finish(&mut plain.clone(), &[]).unwrap();
        let (back, idx) = unpack(&enc).unwrap();
        assert_eq!(back, plain);
        assert_eq!(idx.len(), 4);
        assert_eq!(seal(&enc[..enc.len() - 32]), enc[enc.len() - 32..]);
    }

    /// Real saves: `PKM_SAMPLES=<dir> cargo test --release --lib pkm_switch::tests::real_saves -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_saves() {
        let dir = std::env::var("PKM_SAMPLES").expect("PKM_SAMPLES");
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if !looks(&d) {
                continue;
            }
            let mut doc = parse(&d).unwrap();
            println!("{} -> {} {}", e.path().display(), doc.kind_name, doc.view["Trainer"]);
            if let Some(Value::Object(p)) = doc.view.get("Party") {
                for (k, v) in p {
                    println!("  {k}: {} lv{} {} nature {} shiny {} stats {}", v["Species"], v["Level"], v["Nickname"], v["Nature"], v["Shiny"], v["Stats"]);
                }
            }
            super::super::pkm::verify_party_stats(&doc.view, if doc.kind_name.contains("Sword") || doc.kind_name.contains("Shield") { Gen::G8 } else { Gen::G9 });
            assert_eq!(doc.to_bytes().unwrap(), d, "unchanged save must be written back identically");
            doc.set("/Trainer/Money", Value::from(1234)).unwrap();
            let out = doc.to_bytes().unwrap();
            let again = parse(&out).unwrap();
            assert_eq!(again.view.pointer("/Trainer/Money").unwrap(), 1234);
            println!("  edited money ok");
        }
    }
}
