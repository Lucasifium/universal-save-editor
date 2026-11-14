//! Chronicon characters (`*.char`): hex text. Older files are a `timestamp…` line followed by one
//! hex line per section; newer ones are a JSON object of hex strings. Section `c` holds the
//! character as 64-bit floats: level at byte 24, free skill points at 48, free mastery points at 204.
//!
//! The offsets follow ChroniconSaveEditor (PowerCleanX, MIT). The section is opened as bytes and
//! put back as hex in the same place and letter case.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Chronicon character";
const LEN_MIN: usize = 232;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wrap {
    pub pre: Vec<u8>,
    pub post: Vec<u8>,
    pub lower: bool,
}

fn from_hex(h: &[u8]) -> Option<Vec<u8>> {
    if h.len() % 2 != 0 {
        return None;
    }
    h.chunks(2)
        .map(|p| Some((char::from(p[0]).to_digit(16)? * 16 + char::from(p[1]).to_digit(16)?) as u8))
        .collect()
}

pub fn looks(d: &[u8]) -> bool {
    d.len() >= LEN_MIN && u32::from_le_bytes([d[0], d[1], d[2], d[3]]) == 302 && f64::from_le_bytes(d[24..32].try_into().unwrap()).fract() == 0.0
}

/// Finds section `c`: (start, end) of its hex text.
fn find(d: &[u8]) -> Option<(usize, usize)> {
    if d.starts_with(b"timestamp") {
        let a = d.iter().position(|b| *b == b'\n')? + 1;
        let mut e = a + d[a..].iter().position(|b| *b == b'\n').unwrap_or(d.len() - a);
        if e > a && d[e - 1] == b'\r' {
            e -= 1;
        }
        return Some((a, e));
    }
    let first = d.iter().find(|b| !b.is_ascii_whitespace())?;
    if *first != b'{' {
        return None;
    }
    let key = b"\"c\"";
    let mut from = 0;
    while let Some(p) = d[from..].windows(key.len()).position(|w| w == key) {
        let mut i = from + p + key.len();
        while d.get(i).is_some_and(|b| b.is_ascii_whitespace()) {
            i += 1;
        }
        if d.get(i) == Some(&b':') {
            i += 1;
            while d.get(i).is_some_and(|b| b.is_ascii_whitespace()) {
                i += 1;
            }
            if d.get(i) == Some(&b'"') {
                let s = i + 1;
                let e = s + d[s..].iter().position(|b| *b == b'"')?;
                return Some((s, e));
            }
        }
        from += p + key.len();
    }
    None
}

pub fn unpack(d: &[u8]) -> Option<(Wrap, Vec<u8>)> {
    let (s, e) = find(d)?;
    let plain = from_hex(&d[s..e])?;
    if !looks(&plain) {
        return None;
    }
    let lower = d[s..e].iter().any(|b| b.is_ascii_lowercase());
    Some((Wrap { pre: d[..s].to_vec(), post: d[e..].to_vec(), lower }, plain))
}

pub fn pack(w: &Wrap, plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = w.pre.clone();
    for b in plain {
        let h = if w.lower { format!("{b:02x}") } else { format!("{b:02X}") };
        out.extend_from_slice(h.as_bytes());
    }
    out.extend_from_slice(&w.post);
    Ok(out)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Chronicon character".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Character", "Level"], 24, Kind::F64Int, 1, 300);
    b.ranged(&["Character", "Free skill points"], 48, Kind::F64Int, 0, 1_000_000);
    b.ranged(&["Character", "Free mastery points"], 204, Kind::F64Int, 0, 1_000_000);
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("Level (up to 300), free skill points and free mastery points. The rest of the character is not mapped yet.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Vec<u8> {
        let mut d = vec![0u8; 240];
        d[..4].copy_from_slice(&302u32.to_le_bytes());
        d[24..32].copy_from_slice(&100f64.to_le_bytes());
        d[48..56].copy_from_slice(&38f64.to_le_bytes());
        d
    }

    #[test]
    fn legacy_and_json_round_trip() {
        let hex: String = plain().iter().map(|b| format!("{b:02X}")).collect();
        for text in [format!("timestamp2019\r\n{hex}\r\nrest\n"), format!("{{ \"b\": \"00\", \"c\": \"{hex}\", \"d\": \"11\" }}")] {
            let (w, p) = unpack(text.as_bytes()).unwrap();
            assert_eq!(pack(&w, &p).unwrap(), text.as_bytes());
            let mut t = parse(&p).unwrap();
            t.set("/Character/Level", serde_json::json!(120)).unwrap();
            let out = pack(&w, &t.to_bytes().unwrap()).unwrap();
            let (_, p2) = unpack(&out).unwrap();
            assert_eq!(f64::from_le_bytes(p2[24..32].try_into().unwrap()), 120.0);
        }
    }
}
