//! X-COM: UFO Defense and Terror from the Deep (original DOS saves): each save slot is a folder
//! `GAME_1` … `GAME_10` of fixed-record `.DAT` files with no packing or checksum. `LIGLOB.DAT`
//! starts with the funds, a signed 32-bit number.
//!
//! The funds position is the one documented by Jeff Lawson's X-COM II save editor (3-clause
//! BSD) and the UFOpaedia wiki, and was checked on a Terror from the Deep save; see
//! THIRD_PARTY_NOTICES.md.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;
use std::path::Path;

pub const KIND: &str = "X-COM save (LIGLOB.DAT)";

/// The file is only recognised by its name, since it has no signature.
pub fn looks(d: &[u8], path: Option<&Path>) -> bool {
    let named = path.and_then(|p| p.file_name()).is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("LIGLOB.DAT"));
    named && (4..=4096).contains(&d.len())
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let mut b = Builder::new(d);
    b.ranged(&["Funds", "Money"], 0, Kind::I32, 0, 2_146_000_000);
    Ok(b.finish(KIND, Checksum::None).with_note(
        "Close the game (and DOSBox) first. The game's money is a signed 32-bit number: keep it well under 2,000,000,000, \
         because monthly income is added to it and a total past the limit wraps around.",
    ))
}
