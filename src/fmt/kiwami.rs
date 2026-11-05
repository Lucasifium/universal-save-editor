//! Yakuza Kiwami (PC) `SaveData00NN.sav` / `autosave00NN.sav`: "YKFH", then the save time
//! as seven u16s from 8 (year, month, weekday, day, hour, minute, second). No checksum: two
//! saves made seconds apart differ only in the time and a few counters. Money (u64) and the
//! experience Kiryu hasn't spent yet (u32) were found by following them across sample saves;
//! the block around them is laid out like Yakuza 0's.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const MONEY: usize = 0xC760;
const EXP: usize = 0xC7F0;

pub fn looks(d: &[u8]) -> bool {
    d.len() >= 0x10000 && d.starts_with(b"YKFH")
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Yakuza Kiwami save".into());
    }
    let mut b = Builder::new(d);
    b.ranged(&["Kiryu", "Money"], MONEY, Kind::U64, 0, 9_999_999_999);
    b.ranged(&["Kiryu", "Experience to spend"], EXP, Kind::U32, 0, 99_999_999);
    // Year, month, day of the week, day, hour, minute, second.
    let t: Vec<u16> = (0..7).map(|i| u16::from_le_bytes([d[8 + 2 * i], d[9 + 2 * i]])).collect();
    b.info(&["Game", "Saved"], format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", t[0], t[1], t[3], t[4], t[5], t[6]));
    Ok(b.finish("Yakuza Kiwami save", Checksum::None).with_note("Kiryu's money and the experience he hasn't spent yet."))
}

#[cfg(test)]
mod tests {
    #[test]
    fn money_and_experience() {
        let mut d = b"YKFH".to_vec();
        d.resize(0x10000, 0);
        d[super::MONEY..super::MONEY + 8].copy_from_slice(&999_904_931u64.to_le_bytes());
        d[super::EXP..super::EXP + 4].copy_from_slice(&99_999_809u32.to_le_bytes());
        let mut doc = super::parse(&d).unwrap();
        assert_eq!(doc.view.pointer("/Kiryu/Money").unwrap(), 999_904_931u64);
        doc.set("/Kiryu/Money", serde_json::Value::from(5_000_000_000u64)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u64::from_le_bytes(out[super::MONEY..super::MONEY + 8].try_into().unwrap()), 5_000_000_000);
    }
}
