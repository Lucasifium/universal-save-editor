//! GameCube `.gci` saves made of 0x2000-byte sections that end in a two-part sum, the way
//! Super Mario Sunshine and Luigi's Mansion keep them. Sections come in pairs: the game writes
//! them in turn and uses the one with the higher save count, so the editor changes that one.
//!
//! Layouts after RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

use super::fields::{Builder, Checksum};
use super::n64data::{add, F};
use crate::tree::TreeDoc;

const SECTION: usize = 0x2000;
const DATA: usize = 0x2040;

pub struct Gc {
    pub kind: &'static str,
    /// The first four bytes of the file (the game's code, one for each region).
    pub ids: &'static [&'static [u8; 4]],
    pub size: usize,
    /// Where each pair of sections starts (relative to the file).
    pub pairs: &'static [usize],
    /// Fields in a section are numbered from this offset into the file.
    pub rel: usize,
    /// Slots in a section and the distance between them.
    pub slots: usize,
    pub slot_len: usize,
    /// Whether slot `k` of a section (its 0x2000 bytes) is in use.
    pub live: fn(&[u8], usize) -> bool,
    /// The name of a slot's tab (`{0}` is its number).
    pub label: &'static str,
    pub fields: &'static [F],
    /// Values that belong to the file as a whole.
    pub global: &'static [F],
    pub note: &'static str,
    /// Called for every section that was edited, before its sum is made.
    pub after: fn(&mut [u8]),
}

fn word(d: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]])
}

pub fn sum(s: &[u8]) -> u32 {
    let (mut a, mut b) = (0u32, 0u32);
    for c in s[..SECTION - 4].chunks(2) {
        let v = u16::from_be_bytes([c[0], c[1]]);
        a = a.wrapping_add(u32::from(v));
        b = b.wrapping_add(u32::from(!v));
    }
    (a & 0xffff) << 16 | (b & 0xffff)
}

fn good(d: &[u8], at: usize) -> bool {
    word(d, at + SECTION - 4) == sum(&d[at..at + SECTION])
}

/// The section of the pair at `p` the game reads: the one with the higher save count.
fn newer(d: &[u8], p: usize) -> usize {
    let (a, b) = (word(d, p), word(d, p + SECTION));
    if b != 0xffff && b > a {
        p + SECTION
    } else {
        p
    }
}

pub fn looks(g: &Gc, d: &[u8]) -> bool {
    d.len() == g.size && d.len() > DATA && g.ids.iter().any(|i| d[..4] == **i) && g.pairs.iter().any(|&p| good(d, newer(d, p)))
}

pub fn parse(g: &'static Gc, d: &[u8], fix: fn(&mut Vec<u8>, &[usize])) -> Result<TreeDoc, String> {
    if !looks(g, d) {
        return Err(format!("not a {} save", g.kind));
    }
    let mut b = Builder::new(d);
    let mut shown = 0;
    for (pi, &p) in g.pairs.iter().enumerate() {
        let s = newer(d, p);
        if !good(d, s) {
            continue;
        }
        let mut any = false;
        for k in 0..g.slots {
            if !(g.live)(&d[s..s + SECTION], k) {
                continue;
            }
            shown += 1;
            any = true;
            let name = g.label.replace("{0}", &(pi * g.slots + k + 1).to_string());
            add(&mut b, &[&name], s - g.rel + k * g.slot_len, g.fields);
        }
        if any && !g.global.is_empty() {
            add(&mut b, &[], s - g.rel, g.global);
        }
    }
    if shown == 0 {
        return Err(format!("this {} save has no slot in use", g.kind));
    }
    Ok(b.finish(g.kind, Checksum::Touched(fix)).with_note(g.note))
}

/// Makes the sums right in every section an edit touched.
pub fn fix(g: &Gc, d: &mut Vec<u8>, touched: &[usize]) {
    let mut secs: Vec<usize> = touched
        .iter()
        .filter_map(|&t| g.pairs.iter().flat_map(|&p| [p, p + SECTION]).find(|&s| t >= s && t < s + SECTION))
        .collect();
    secs.sort_unstable();
    secs.dedup();
    for s in secs {
        (g.after)(&mut d[s..s + SECTION]);
        let v = sum(&d[s..s + SECTION]).to_be_bytes();
        d[s + SECTION - 4..s + SECTION].copy_from_slice(&v);
    }
}
