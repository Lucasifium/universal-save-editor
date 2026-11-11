//! Kenshi `.save`: lists of named values (u32 name length, name, 4-byte value). The
//! player's money is the one value named "player money".

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

const MONEY: &[u8] = b"\x0c\x00\x00\x00player money";

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn looks(d: &[u8]) -> bool {
    // A Kenshi save names itself ("…-quick.save-INGAME") near the start.
    d.len() > 4096 && find(&d[..256], b".save-").is_some()
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    if !looks(d) {
        return Err("not a Kenshi save".into());
    }
    let at = find(d, MONEY).ok_or("no player money in this save")? + MONEY.len();
    let mut b = Builder::new(d);
    b.ranged(
        &["Player", "Money (cats)"],
        at,
        Kind::I32,
        0,
        i32::MAX as i64,
    );
    Ok(b.finish("Kenshi save", Checksum::None).with_note(
        "The player faction's cats. Squads and characters are kept in the save's platoon folder.",
    ))
}
