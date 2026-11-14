//! Helpers shared by the PlayStation 1 game saves. A save as the editor opens it (from a memory card
//! image, a `.mcs`, `.psx`, `.psv`…) starts with the 0x2000-byte block header ("SC", the icon
//! flag, the title in Shift-JIS, the icon palette and frames), followed by the game's data.

use super::fields::{Builder, Kind};

/// The save's title (Shift-JIS), with the full-width Latin letters and digits turned into plain
/// ones: "ＣＡＳＴＬＥＶＡＮＩＡ" becomes "CASTLEVANIA".
pub fn title(d: &[u8]) -> String {
    let Some(t) = d.get(4..68) else { return String::new() };
    if &d[..2] != b"SC" {
        return String::new();
    }
    let mut out = String::new();
    let mut i = 0;
    while i < t.len() && t[i] != 0 {
        let c = t[i];
        if (c == 0x81 || c == 0x82) && i + 1 < t.len() {
            let w = ((c as u32) << 8) | t[i + 1] as u32;
            out.push(match w {
                0x824f..=0x8258 => (b'0' + (w - 0x824f) as u8) as char,
                0x8260..=0x8279 => (b'A' + (w - 0x8260) as u8) as char,
                0x8281..=0x829a => (b'a' + (w - 0x8281) as u8) as char,
                0x8140 => ' ',
                _ => '?',
            });
            i += 2;
        } else {
            out.push(if (32..127).contains(&c) { c as char } else { '?' });
            i += 1;
        }
    }
    out
}

/// True when the save has the block header and `blocks` 8 KiB blocks.
pub fn is_save(d: &[u8], blocks: usize) -> bool {
    d.len() == blocks * 0x2000 && &d[..2] == b"SC"
}

/// How many extra icon frames (0x80 bytes each) the header has beyond the first.
pub fn extra_frames(d: &[u8]) -> usize {
    ((d[2] & 3) as usize).saturating_sub(1)
}

/// A plain number (unsigned, `lo..=hi`) at `at`.
pub fn num(b: &mut Builder, path: &[&str], at: usize, kind: Kind, hi: i64) {
    b.ranged(path, at, kind, 0, hi);
}

/// A pick list of `(name, raw)`.
pub fn opts(list: &[(&str, u64)]) -> Vec<(String, u64)> {
    list.iter().map(|(n, v)| (n.to_string(), *v)).collect()
}

#[cfg(test)]
mod samples {
    /// `SAVE_SAMPLES=<dir> cargo test --release --lib ps1::samples -- --ignored --nocapture`:
    /// every save of every memory card in the folder that one of the PS1 readers recognises must
    /// open and write back byte for byte.
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut n = 0;
        let mut rd: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
        rd.sort_by_key(|e| e.path());
        for e in rd {
            let card = std::fs::read(e.path()).unwrap();
            let Some(off) = crate::fmt::memcard::ps1_offset(&card) else { continue };
            for s in crate::fmt::memcard::ps1_saves(&card[off..]) {
                let d = crate::fmt::memcard::ps1_read(&card[off..], &s);
                if let Some(mut doc) = crate::fmt::parse_binary(&d) {
                    if !doc.kind_name.contains("save") && !doc.kind_name.contains("Save") {
                        continue;
                    }
                    assert_eq!(doc.to_bytes().unwrap(), d, "{:?} {}", e.path(), s.name);
                    println!("{:?} {}: {} ok", e.file_name(), s.name, doc.kind_name);
                    n += 1;
                }
            }
        }
        println!("{n} saves");
    }
}

#[cfg(test)]
mod ps2samples {
    /// `PS2_SAMPLES=<dir> cargo test --release --lib ps1::ps2samples -- --ignored --nocapture`: lists
    /// the files of every PS2 single save and the game save each one opens as.
    #[test]
    #[ignore]
    fn list() {
        let Ok(dir) = std::env::var("PS2_SAMPLES") else { return };
        let mut rd: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
        rd.sort_by_key(|e| e.path());
        for e in rd {
            let p = e.path();
            let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            let Ok(d) = std::fs::read(&p) else { continue };
            if let Some((kind, files)) = crate::fmt::pssave::open(&d, &ext) {
                let big = files.iter().max_by_key(|f| f.data.len()).unwrap();
                if let Some(mut doc) = crate::fmt::parse_binary(&big.data) {
                    assert_eq!(doc.to_bytes().unwrap(), big.data, "{p:?}");
                    println!("{:?}: {} round-trips", p.file_name().unwrap(), doc.kind_name);
                }
                println!("{:?} {:?}: {:?} biggest {} {} bytes, head {:02x?}", p.file_name().unwrap(), kind, files.iter().map(|f| (f.name.clone(), f.data.len())).collect::<Vec<_>>(), big.name, big.data.len(), &big.data[..16]);
            }
        }
    }
}
