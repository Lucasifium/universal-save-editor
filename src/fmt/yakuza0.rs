//! Yakuza 0 (PC) `SaveData00NN.sav`: "YZFH", no checksum. Field positions from the
//! y0-save-editor layout (etra0, after Capitán Retraso's reversing) and the Nexus Mods
//! offsets list; checked against sample saves (save time, difficulty, money, CP).

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 0x11160 && d.starts_with(b"YZFH")
}

/// Yakuza 0's money goes up to ¥10 trillion.
const MAX_YEN: i64 = 9_999_999_999_999;

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Yakuza 0 save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Kiryu", "Money"], 0xF2C0, Kind::U64, 0, MAX_YEN);
    b.ranged(&["Kiryu", "CP"], 0xF3E0, Kind::U32, 0, 9_999);
    b.ranged(&["Kiryu", "Real estate takings (Kamurocho)"], 0x11120, Kind::U64, 0, MAX_YEN);
    b.ranged(&["Majima", "Money"], 0xF2C8, Kind::U64, 0, MAX_YEN);
    b.ranged(&["Majima", "CP"], 0xF3E8, Kind::U32, 0, 9_999);
    b.ranged(&["Majima", "Cabaret takings (Sotenbori)"], 0x11150, Kind::U64, 0, MAX_YEN);
    b.named(&["Game", "Difficulty"], 0x445, Kind::U8, &["Easy", "Normal", "Hard", "Legend"]);
    let t: Vec<u16> = (0..6).map(|i| u16::from_le_bytes([d[0x28 + 2 * i], d[0x29 + 2 * i]])).collect();
    b.info(&["Game", "Saved"], format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", t[0], t[1], t[2], t[3], t[4], t[5]));
    b.info(&["Game", "Playing as"], if d[0x4CC] == 2 { "Majima" } else { "Kiryu" });
    Ok(b.finish("Yakuza 0 save", Checksum::None).with_note(
        "Kiryu's and Majima's money and CP, the takings waiting in the real estate and cabaret businesses, and the difficulty.",
    ))
}
