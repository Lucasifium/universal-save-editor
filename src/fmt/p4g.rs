//! Persona 4 Golden (PC) `dataNNNN.bin`. Field positions from the MIT-licensed P4G Save
//! Tool (Fennec-kun): yen at 88, the protagonist's level at 3290, total experience at
//! 3348 and the five social stats from 3336. The game also keeps an MD5 of each save in
//! the matching `.binslot` (bytes 0x18–0x27), which is updated when the save is written
//! (see `formats::companion_files`).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub fn looks(d: &[u8]) -> bool {
    // "SG" at 8, and the protagonist's surname (the game's own text: 0x80, then the letter) at 16.
    d.len() > 10_000 && d.get(8..10) == Some(b"SG") && d.get(16) == Some(&0x80)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Persona 4 Golden save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Protagonist", "Yen"], 88, Kind::U32, 0, 9_999_999);
    b.ranged(&["Protagonist", "Level"], 3290, Kind::U8, 1, 99);
    b.ranged(
        &["Protagonist", "Total experience"],
        3348,
        Kind::U32,
        0,
        99_999_999,
    );
    for (i, s) in [
        "Knowledge",
        "Courage",
        "Diligence",
        "Understanding",
        "Expression",
    ]
    .iter()
    .enumerate()
    {
        b.ranged(&["Social stats", s], 3336 + 2 * i, Kind::U16, 0, 999);
    }
    Ok(b.finish("Persona 4 Golden save", Checksum::None).with_note(
        "Yen, level, experience and social stats. The matching .binslot file's checksum is updated when you save.",
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn binslot_checksum_follows_the_save() {
        use md5::Digest;
        let dir = std::env::temp_dir().join(format!("p4g_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let save = dir.join("data0001.bin");
        let mut slot = b"SAVE0001".to_vec();
        slot.resize(884, 0);
        std::fs::write(dir.join("data0001.binslot"), &slot).unwrap();
        let data = vec![7u8; 1000];
        let out = crate::formats::companion_files(&save, &data);
        assert_eq!(out.len(), 1);
        assert_eq!(&out[0].1[0x18..0x28], &md5::Md5::digest(&data)[..]);
        assert_eq!(out[0].1.len(), 884);
        let _ = std::fs::remove_dir_all(dir);
    }
}
