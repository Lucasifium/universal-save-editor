//! Helpers shared by the console and handheld save formats built on `fields::Builder`.

use super::fields::{Builder, Kind};

/// An editable yes/no bit of the byte at `at` (shown as a drop-down).
pub fn flag(b: &mut Builder, path: &[&str], at: usize, bit: u32) {
    b.pick_bits(path, at, Kind::U8, bit, 1, &[("No".to_string(), 0), ("Yes".to_string(), 1)]);
}

/// Sum of the bytes in `d[from..to]`.
pub fn byte_sum(d: &[u8], from: usize, to: usize) -> u32 {
    d[from..to].iter().map(|&x| x as u32).sum()
}

#[cfg(test)]
mod samples {
    use crate::fmt::*;
    const KINDS: &[&str] = &[mk64::KIND, goldeneye::KIND, banjokazooie::KIND, zelda64::KIND, sunshine::KIND, luigi::KIND, windwaker::KIND];
    /// `SAVE_SAMPLES=<dir> cargo test --release --lib retro::samples -- --ignored --nocapture`:
    /// every console save in the folder that one of the readers here recognises must open and
    /// write back byte for byte.
    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("SAVE_SAMPLES") else { return };
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if let Some(mut doc) = crate::fmt::parse_binary(&d) {
                if !doc.kind_name.starts_with("Super ") && !doc.kind_name.contains("Link to") && !doc.kind_name.contains("Final Fantasy VI") && !doc.kind_name.contains("Chrono Trigger") && !doc.kind_name.contains("Secret of Mana") && !doc.kind_name.starts_with("Xenoblade") && !doc.kind_name.starts_with("Breath of") && !doc.kind_name.starts_with("Mario Kart 7") && !doc.kind_name.starts_with("Super Kirby") && !doc.kind_name.starts_with("Mario & Luigi") && !KINDS.contains(&doc.kind_name.as_str()) {
                    continue;
                }
                assert_eq!(doc.to_bytes().unwrap(), d, "{:?}", e.path());
                if let Ok(dir) = std::env::var("SAVE_DUMP") {
                    let name = format!("{dir}/{}.json", e.file_name().to_string_lossy());
                    std::fs::write(name, serde_json::to_string_pretty(&doc.view).unwrap()).unwrap();
                }
                println!("{:?}: {} ok", e.file_name(), doc.kind_name);
                n += 1;
            }
        }
        println!("{n} saves");
    }
}
