//! Field tables for the Nintendo 64 and GameCube saves, one module per game. Each entry says
//! where a value lives inside one slot (or file) and how it is shown; `add` puts them in the
//! view. The layouts follow RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Kind};

pub mod bk;
pub mod lm;
pub mod oot;
pub mod sms;
pub mod smash64;
pub mod ww;

/// One value: where it is, which bits of it, and its names (a drop-down) or its highest value.
pub struct F {
    pub path: &'static [&'static str],
    pub at: usize,
    pub kind: Kind,
    pub shift: u32,
    pub width: u32,
    pub be: bool,
    /// Highest value the game takes (-1: the type's own limit).
    pub max: i64,
    pub opts: &'static [(&'static str, u64)],
}

/// Adds `fields` below `prefix`, with every offset moved by `base`.
pub fn add(b: &mut Builder, prefix: &[&str], base: usize, fields: &[F]) {
    let was = b.big_endian;
    for f in fields {
        let mut path: Vec<&str> = prefix.to_vec();
        path.extend_from_slice(f.path);
        let at = base + f.at;
        b.big_endian = f.be;
        let full = f.width as usize == f.kind.size() * 8;
        if !f.opts.is_empty() {
            let opts: Vec<(String, u64)> = f.opts.iter().map(|(n, v)| (n.to_string(), *v)).collect();
            if full {
                b.pick(&path, at, f.kind, &opts);
            } else {
                b.pick_bits(&path, at, f.kind, f.shift, f.width, &opts);
            }
        } else if full {
            b.ranged(&path, at, f.kind, 0, if f.max >= 0 { f.max } else { i64::MAX });
        } else {
            b.bits(&path, at, f.kind, f.shift, f.width, 0, 0, if f.max >= 0 { f.max } else { (1 << f.width) - 1 });
        }
    }
    b.big_endian = was;
}

/// How a Nintendo 64 SRAM save (32 KiB) sits in its file.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sram {
    Plain(usize),
    /// Every 32-bit word reversed (Project64 and others).
    Swapped(usize),
}

pub fn swap32(b: &[u8]) -> Vec<u8> {
    b.chunks(4).flat_map(|c| c.iter().rev().copied().collect::<Vec<u8>>()).collect()
}

/// Finds the 32 KiB SRAM in a `.sra` or an emulator's combined `.srm` (it is at 0x20800 there)
/// and returns it in the console's own byte order, if `valid` accepts it.
pub fn sram(d: &[u8], valid: fn(&[u8]) -> bool) -> Option<(Sram, Vec<u8>)> {
    if d.len() != 0x8000 && d.len() != 296_960 {
        return None;
    }
    let base = if d.len() == 0x8000 { 0 } else { 0x20800 };
    let raw = &d[base..base + 0x8000];
    if valid(raw) {
        return Some((Sram::Plain(base), raw.to_vec()));
    }
    let sw = swap32(raw);
    valid(&sw).then_some((Sram::Swapped(base), sw))
}

/// Puts edited SRAM bytes back into the file `sram` read them from.
pub fn sram_put(outer: &[u8], inner: &[u8], valid: fn(&[u8]) -> bool) -> Vec<u8> {
    let mut o = outer.to_vec();
    match sram(outer, valid).map(|x| x.0) {
        Some(Sram::Plain(b)) => o[b..b + 0x8000].copy_from_slice(inner),
        Some(Sram::Swapped(b)) => o[b..b + 0x8000].copy_from_slice(&swap32(inner)),
        None => {}
    }
    o
}

/// True for the sizes an EEPROM save comes in (the EEPROM leads an emulator's `.srm`).
pub fn eeprom_size(d: &[u8]) -> bool {
    matches!(d.len(), 512 | 2048 | 296_960)
}
