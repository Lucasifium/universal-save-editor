//! Underrail `global.dat` (and `info.dat`): a 24-byte header (a GUID and the game's save
//! version) kept as it is, then one gzip stream holding a .NET BinaryFormatter message. The
//! message is read by the `nrbf` reader; only the wrapper is handled here. Field names and the
//! layout of the player object follow the GPL-3.0 editor listed in THIRD_PARTY_NOTICES.md
//! (facts only, no code).

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::io::{Read, Write};
use std::sync::Arc;

const HEAD: usize = 24;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packed {
    head: Arc<Vec<u8>>,
    original: Arc<Vec<u8>>,
    file: Arc<Vec<u8>>,
}

pub fn maybe(d: &[u8]) -> bool {
    d.len() > HEAD + 20 && d[HEAD] == 0x1f && d[HEAD + 1] == 0x8b && d[HEAD + 2] == 8
}

pub fn open(d: &[u8]) -> Option<(Packed, Vec<u8>)> {
    if !maybe(d) {
        return None;
    }
    let mut inner = Vec::new();
    GzDecoder::new(&d[HEAD..]).take(1 << 30).read_to_end(&mut inner).ok()?;
    if !super::nrbf::looks(&inner) {
        return None;
    }
    Some((Packed { head: Arc::new(d[..HEAD].to_vec()), original: Arc::new(inner.clone()), file: Arc::new(d.to_vec()) }, inner))
}

pub fn close(p: &Packed, data: &[u8]) -> Result<Vec<u8>, String> {
    if data == p.original.as_slice() {
        return Ok((*p.file).clone());
    }
    let mut z = GzEncoder::new(p.head.to_vec(), Compression::default());
    z.write_all(data).map_err(|e| e.to_string())?;
    z.finish().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_is_kept_and_unchanged_files_come_back_exactly() {
        let mut msg = vec![0u8, 1, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 1, 0, 0, 0, 0, 0, 0, 0, 11];
        msg.extend_from_slice(&[0; 20]);
        let mut f = vec![7u8; HEAD];
        let mut z = GzEncoder::new(Vec::new(), Compression::fast());
        z.write_all(&msg).unwrap();
        f.extend(z.finish().unwrap());
        let (p, inner) = open(&f).unwrap();
        assert_eq!(inner, msg);
        assert_eq!(close(&p, &inner).unwrap(), f);
        let mut changed = inner.clone();
        changed[20] = 9;
        let out = close(&p, &changed).unwrap();
        assert_eq!(&out[..HEAD], &f[..HEAD]);
        assert_eq!(open(&out).unwrap().1, changed);
        assert!(open(&[0u8; 80]).is_none());
    }

    /// `UNDERRAIL_SAVE=<global.dat> cargo test --release --lib underrail -- --ignored --nocapture`:
    /// the save opens through the full chain, reads back unchanged and its view is printed.
    #[test]
    #[ignore]
    fn real_save_round_trips() {
        let Ok(p) = std::env::var("UNDERRAIL_SAVE") else { return };
        let raw = std::fs::read(&p).unwrap();
        let dec = crate::formats::decode(&raw);
        assert!(matches!(dec.layers.first(), Some(crate::formats::Layer::Underrail(_))));
        assert_eq!(crate::formats::encode(&dec.layers, &dec.content).unwrap(), raw);
        if let (Ok(out), crate::formats::Content::Tree(t)) = (std::env::var("UNDERRAIL_OUT"), &dec.content) {
            std::fs::write(out, serde_json::to_string_pretty(&t.view).unwrap()).unwrap();
            println!("{}", t.kind_name);
        }
    }
}
