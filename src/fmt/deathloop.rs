//! Deathloop `save.dloop` (Arkane's Void engine).
//!
//! ```text
//! u32   checksum: the four little-endian words of MD5(everything after these 4 bytes), XORed
//! "SAVE", u32 1, the Steam id as text padded with zeros up to offset 0x4C
//! "OOD\0", u32 8 (Kraken), u32 size of the JSON text, then one Oodle stream
//! ```
//!
//! The stream is unpacked with the Oodle library the game installs (see `oodle.rs`) and is the
//! game's state as JSON text. Packing it again rewrites the size and the checksum.

use md5::{Digest, Md5};

const HEAD: usize = 0x58;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Bytes 0x04..0x54 of the file (everything between the checksum and the size field).
    mid: Vec<u8>,
}

pub fn looks(d: &[u8]) -> bool {
    d.len() > HEAD && &d[4..8] == b"SAVE" && &d[0x4C..0x50] == b"OOD\0"
}

pub fn checksum(body: &[u8]) -> u32 {
    let m = Md5::digest(body);
    m.chunks(4).fold(0, |a, c| a ^ u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
}

/// The save's JSON text and the layout needed to pack it again.
pub fn unpack(d: &[u8]) -> Result<(Layout, Vec<u8>), String> {
    if !looks(d) {
        return Err("not a Deathloop save".into());
    }
    if u32::from_le_bytes(d[..4].try_into().unwrap()) != checksum(&d[4..]) {
        return Err("the Deathloop save's checksum doesn't match".into());
    }
    let raw = u32::from_le_bytes(d[0x54..0x58].try_into().unwrap()) as usize;
    if raw == 0 || raw > 256 << 20 {
        return Err("unexpected size in the Deathloop save".into());
    }
    let out = crate::oodle::decompress(&d[HEAD..], raw).ok_or("Oodle couldn't unpack the save's data")?;
    Ok((Layout { mid: d[4..0x54].to_vec() }, out))
}

pub fn pack(l: &Layout, json: &[u8]) -> Result<Vec<u8>, String> {
    let packed = crate::oodle::compress(json).ok_or("Oodle couldn't pack the save again (no Oodle library found)")?;
    let mut body = l.mid.clone();
    body.extend((json.len() as u32).to_le_bytes());
    body.extend(packed);
    let mut out = checksum(&body).to_le_bytes().to_vec();
    out.extend(body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(payload: &[u8]) -> Vec<u8> {
        let mut body = b"SAVE".to_vec();
        body.extend(1u32.to_le_bytes());
        body.extend(b"76561198000000000");
        body.resize(0x48, 0);
        body.extend(b"OOD\0");
        body.extend(8u32.to_le_bytes());
        body.extend((payload.len() as u32).to_le_bytes());
        body.extend(payload);
        let mut out = checksum(&body).to_le_bytes().to_vec();
        out.extend(body);
        out
    }

    #[test]
    fn recognised_and_checksum_verified() {
        let f = file(b"\x8c\x06not really oodle");
        assert!(looks(&f));
        assert_eq!(f.len() > HEAD, true);
        let mut bad = f.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(unpack(&bad).unwrap_err().contains("checksum"));
        assert!(!looks(b"SAVE"));
    }

    #[test]
    fn checksum_is_xor_of_md5_words() {
        // MD5("") = d41d8cd98f00b204e9800998ecf8427e
        assert_eq!(checksum(b""), 0xd98c1dd4 ^ 0x04b2008f ^ 0x980980e9 ^ 0x7e42f8ec);
    }

    #[test]
    fn the_screen_recognises_the_state() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"date":"x","save":{"commonState":{"TimeEssence":{"m_currency":3500},"Progression":{"m_day":32,"stressLoop":{"currentStressLevel":99}}}}}"#,
        )
        .unwrap();
        let g = crate::game_ui::detect("JSON", &v).expect("spec");
        assert_eq!(g.name(), Some("Deathloop"));
    }

    /// Runs against a real save when DEATHLOOP_SAVE names one (needs an Oodle library).
    #[test]
    fn real_save_roundtrip() {
        let Some(path) = std::env::var_os("DEATHLOOP_SAVE") else { return };
        let data = std::fs::read(path).unwrap();
        let (l, json) = unpack(&data).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&json).unwrap();
        eprintln!("json {} bytes, residuum {:?}, day {:?}", json.len(), v.pointer("/save/commonState/TimeEssence/m_currency"), v.pointer("/save/commonState/Progression/m_day"));
        let packed = pack(&l, &json).unwrap();
        assert_eq!(u32::from_le_bytes(packed[..4].try_into().unwrap()), checksum(&packed[4..]));
        let (_, again) = unpack(&packed).unwrap();
        assert_eq!(json, again, "JSON changed after packing");
        eprintln!("repack ok: {} -> {} bytes", data.len(), packed.len());
        let mut w = v.clone();
        *w.pointer_mut("/save/commonState/TimeEssence/m_currency").unwrap() = serde_json::json!(99999);
        let edited = pack(&l, serde_json::to_string_pretty(&w).unwrap().as_bytes()).unwrap();
        let (_, back) = unpack(&edited).unwrap();
        let b: serde_json::Value = serde_json::from_slice(&back).unwrap();
        assert_eq!(b.pointer("/save/commonState/TimeEssence/m_currency"), Some(&serde_json::json!(99999)));
        if let Some(out) = std::env::var_os("DEATHLOOP_OUT") {
            std::fs::write(out, &edited).unwrap();
            eprintln!("wrote edited save (residuum 99999), {} bytes", edited.len());
        }
    }
}
