//! Super Mario Sunshine (GameCube) `.gci` saves: three Mario files, each a pair of 0x2000-byte
//! sections of which the game reads the newer one (see `gcsave`).
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::gcsave::{self, Gc};
use super::n64data::sms;
use crate::tree::TreeDoc;

pub const KIND: &str = "Super Mario Sunshine save";

pub static GAME: Gc = Gc {
    kind: KIND,
    ids: &[b"GMSE", b"GMSP", b"GMSJ"],
    size: 0x2040 + 3 * 0x4000,
    pairs: &[0x2040, 0x6040, 0xa040],
    rel: 0,
    slots: 1,
    slot_len: 0,
    live: |s, _| s[4..8] != [0; 4],
    label: "Mario {0}",
    fields: sms::SLOT,
    global: &[],
    note: "Close the emulator first. The count of shine sprites is kept in step with the ones you tick. Each edited section's sum is made again.",
    after,
};

/// The total of shine sprites (at 0x1c) is the number of bits set in the 15 bytes from 0x60.
fn after(s: &mut [u8]) {
    let n: u32 = s[0x60..0x6f].iter().map(|b| b.count_ones()).sum();
    s[0x1c..0x1e].copy_from_slice(&(n as u16).to_be_bytes());
}

pub fn looks(d: &[u8]) -> bool {
    gcsave::looks(&GAME, d)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    gcsave::parse(&GAME, d, fix)
}

fn fix(d: &mut Vec<u8>, touched: &[usize]) {
    gcsave::fix(&GAME, d, touched);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_the_newer_section_and_its_sum() {
        let mut d = vec![0u8; GAME.size];
        d[..6].copy_from_slice(b"GMSE01");
        for p in [0x2040usize, 0x6040, 0xa040] {
            d[p + 3] = 1;
            d[p + 4..p + 8].copy_from_slice(&1u32.to_be_bytes());
        }
        // The second section of Mario 1 is newer.
        d[0x4043] = 2;
        d[0x4044..0x4048].copy_from_slice(&1u32.to_be_bytes());
        for s in [0x2040usize, 0x4040, 0x6040, 0x8040, 0xa040, 0xc040] {
            let v = gcsave::sum(&d[s..s + 0x2000]).to_be_bytes();
            d[s + 0x1ffc..s + 0x2000].copy_from_slice(&v);
        }
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Mario 1/General/Lives", 55.into()).unwrap();
        doc.set("/Mario 1/Delfino Airstrip/Shine Sprites/Introduction", "Yes".into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(&out[0x4040 + 0x264..0x4040 + 0x268], &55u32.to_be_bytes());
        assert_eq!(&out[0x4040 + 0x1c..0x4040 + 0x1e], &1u16.to_be_bytes());
        assert_eq!(&out[0x2040..0x4040], &d[0x2040..0x4040]);
        assert!(looks(&out));
        assert_eq!(gcsave::sum(&out[0x4040..0x6040]).to_be_bytes(), out[0x6040 - 4..0x6040]);
    }
}
