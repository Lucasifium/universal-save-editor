//! Saints Row: The Third and Remastered (`sr3save_NN.sr3s_pc`): a 108,776-byte memory image of
//! the player's profile, uncompressed. Offsets and the checksum (CRC-32 with a zero start and no
//! final inversion, over bytes 4 to the end of the 0x1AA00-byte save buffer, whose last 0x118
//! bytes are zeros that aren't written to the file) are from the MIT-licensed
//! IdolRecomp-Saints-Row-The-Third save format notes, and were checked against public saves.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Saints Row: The Third save";
const SIZE: usize = 0x1A8E8;
const BUFFER: usize = 0x1AA00;

fn u32_at(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

fn crc(d: &[u8]) -> u32 {
    let mut c = 0u32;
    let mut feed = |b: u8| {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg());
        }
    };
    d[4..SIZE].iter().for_each(|&b| feed(b));
    (0..BUFFER - SIZE).for_each(|_| feed(0));
    c
}

pub fn looks(d: &[u8]) -> bool {
    d.len() == SIZE && u32_at(d, 0) == crc(d) && u32_at(d, 4) == 0
}

fn fix(d: &mut Vec<u8>) {
    let c = crc(d);
    d[..4].copy_from_slice(&c.to_le_bytes());
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Saints Row: The Third save".into());
    }
    let mut b = Builder::new(d);
    // The game keeps cash as hundredths of a dollar and caps it at 2,000,000,000.
    b.ranged(&["Player", "Cash (in cents)"], 0x4A70, Kind::U32, 0, 2_000_000_000);
    b.ranged(&["Player", "Crib money stash (in cents)"], 0x4F58, Kind::U32, 0, 2_000_000_000);
    b.ranged(&["Player", "Respect level"], 0x4420, Kind::U32, 0, 50);
    b.ranged(&["Player", "Respect in this level"], 0x441C, Kind::U32, 0, 99_999_999);
    b.ranged(&["Player", "Total respect"], 0x4418, Kind::U32, 0, 99_999_999);
    b.ranged(&["Statistics", "Barnstorms found"], 0x3F0, Kind::U32, 0, 99);
    b.ranged(&["Statistics", "Stunt jumps found"], 0x3F8, Kind::U32, 0, 99);
    b.ranged(&["Game", "Difficulty index"], 0x2B64, Kind::U32, 0, 2);
    Ok(b.finish(KIND, Checksum::Custom(fix)).with_note("Close the game first. Cash is in cents (500000 is $5,000). The save's checksum is rewritten. Changing the respect level alone doesn't change the respect the game thinks you have earned, so set total respect with it."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn cash_is_written_and_the_checksum_fixed() {
        let mut d = vec![0u8; SIZE];
        d[8..12].copy_from_slice(&94u32.to_le_bytes());
        d[0x4A70..0x4A74].copy_from_slice(&5700u32.to_le_bytes());
        fix(&mut d);
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        doc.set("/Player/Cash (in cents)", Value::from(123_400_000u64)).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(u32_at(&out, 0x4A70), 123_400_000);
        assert!(looks(&out));
    }

    #[test]
    #[ignore]
    fn real_saves() {
        let Ok(dir) = std::env::var("SR3_SAMPLES") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(e.path()).unwrap();
            if d.len() != SIZE {
                continue;
            }
            assert!(looks(&d), "{:?}", e.path());
            let doc = parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d);
        }
    }
}
