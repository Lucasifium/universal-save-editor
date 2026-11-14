//! Luigi's Mansion (GameCube) `.gci` saves: a pair of 0x2000-byte sections holding three slots
//! (the game reads the newer section), see `gcsave`.
//!
//! Layout after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::gcsave::{self, Gc};
use super::n64data::lm;
use crate::tree::TreeDoc;

pub const KIND: &str = "Luigi's Mansion save";

pub static GAME: Gc = Gc {
    kind: KIND,
    ids: &[b"GLME", b"GLMP", b"GLMJ"],
    size: 0x6040,
    pairs: &[0x2040],
    rel: 0x2040,
    slots: 3,
    slot_len: 0x3b0,
    live: |s, k| s[0xd + k * 0x3b0] & 1 != 0,
    label: "Slot {0}",
    fields: lm::SLOT,
    global: lm::GLOBAL,
    note: "Close the emulator first. Rooms and boos are flags: a room you tick as visited is not drawn on the map by itself. Each edited section's sum is made again.",
    after: |_| {},
};

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
    fn edits_a_slot_of_the_newer_section() {
        let mut d = vec![0u8; GAME.size];
        d[..6].copy_from_slice(b"GLME01");
        for s in [0x2040usize, 0x4040] {
            d[s + 0xd] = 1;
            d[s + 3] = 1;
        }
        d[0x4043] = 2;
        for s in [0x2040usize, 0x4040] {
            let v = gcsave::sum(&d[s..s + 0x2000]).to_be_bytes();
            d[s + 0x1ffc..s + 0x2000].copy_from_slice(&v);
        }
        assert!(looks(&d));
        let mut doc = parse(&d).unwrap();
        assert_eq!(doc.to_bytes().unwrap(), d);
        doc.set("/Slot 1/General/Treasures/Gold Bars", 7.into()).unwrap();
        let out = doc.to_bytes().unwrap();
        assert_eq!(out[0x4040 + 0x300], 7);
        assert_eq!(out[0x2040..0x4040], d[0x2040..0x4040]);
        assert!(looks(&out));
        assert_eq!(gcsave::sum(&out[0x4040..0x6040]).to_be_bytes(), out[0x6040 - 4..0x6040]);
    }
}
