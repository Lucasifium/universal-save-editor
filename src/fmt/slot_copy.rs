//! Copying an Elden Ring character to another of the ten slots in `ER0000.sl2`.
//!
//! Each character is one archive entry (USER_DATA000…009, the same size). The title screen
//! reads USER_DATA010: one "in use" byte per slot at 0x1954, then a 0x24C-byte summary per
//! slot at 0x195E (name in UTF-16 first, level at +0x22). A copy is the slot's entry as it
//! is (its MD5 covers only itself), plus the summary and the "in use" byte; USER_DATA010
//! then gets a new MD5. Offsets checked against a sample save with three characters.

use super::containers::{bnd4_entries, bnd4_open, bnd4_replace, Bnd4Entry};

const SLOTS: usize = 10;
const ACTIVE: usize = 0x1954;
const SUMMARY: usize = 0x195E;
const SUMMARY_LEN: usize = 0x24C;
const PROFILE_LEN: usize = 0x6_0000;

/// What the title screen shows for a slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub active: bool,
    pub name: String,
    pub level: u32,
}

fn find<'a>(entries: &'a [Bnd4Entry], name: &str) -> Option<&'a Bnd4Entry> {
    entries.iter().find(|e| e.name == name)
}

fn slot_name(i: usize) -> String {
    format!("USER_DATA{i:03}")
}

/// The profile entry, if this is an Elden Ring save with all ten character slots.
fn profile(d: &[u8]) -> Option<(Vec<Bnd4Entry>, Vec<u8>)> {
    let entries = bnd4_entries(d)?;
    let size = find(&entries, &slot_name(0))?.size;
    if (0..SLOTS).any(|i| find(&entries, &slot_name(i)).is_none_or(|e| e.size != size)) {
        return None;
    }
    let (data, key, _) = bnd4_open(d, find(&entries, "USER_DATA010")?)?;
    (key.is_none() && data.len() == PROFILE_LEN && d.starts_with(b"BND4")).then_some((entries, data))
}

/// The ten slots, or `None` when this isn't an Elden Ring `.sl2`.
pub fn er_slots(d: &[u8]) -> Option<Vec<Slot>> {
    let (_, p) = profile(d)?;
    Some(
        (0..SLOTS)
            .map(|i| {
                let s = SUMMARY + i * SUMMARY_LEN;
                let units: Vec<u16> = p[s..s + 32].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
                Slot {
                    active: p[ACTIVE + i] == 1,
                    name: String::from_utf16_lossy(&units),
                    level: u32::from_le_bytes(p[s + 0x22..s + 0x26].try_into().unwrap()),
                }
            })
            .collect(),
    )
}

/// The save with character `from` copied over slot `to`.
pub fn er_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    if from == to || from >= SLOTS || to >= SLOTS {
        return Err("pick two different slots".into());
    }
    let (entries, mut p) = profile(d).ok_or("this isn't an Elden Ring save with ten slots")?;
    if p[ACTIVE + from] != 1 {
        return Err("that slot has no character".into());
    }
    let (src, dst) = (find(&entries, &slot_name(from)).unwrap(), find(&entries, &slot_name(to)).unwrap());
    // The source must be intact (its MD5 checks out) before it's duplicated.
    bnd4_open(d, src).ok_or("the character's data is damaged")?;
    let mut out = d.to_vec();
    out[dst.offset..dst.offset + dst.size].copy_from_slice(&d[src.offset..src.offset + src.size]);
    p[ACTIVE + to] = 1;
    let (a, b) = (SUMMARY + from * SUMMARY_LEN, SUMMARY + to * SUMMARY_LEN);
    p.copy_within(a..a + SUMMARY_LEN, b);
    bnd4_replace(&out, "USER_DATA010", &p, None, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Research aid: copies the first character of SAVE_EDITOR_ER into the first free slot
    /// and reads the result back.
    #[test]
    #[ignore]
    fn copy_real_character() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_ER").expect("SAVE_EDITOR_ER")).unwrap();
        let slots = er_slots(&d).expect("Elden Ring save");
        println!("{slots:?}");
        let to = slots.iter().position(|s| !s.active).unwrap();
        let out = er_copy(&d, 0, to).unwrap();
        let after = er_slots(&out).unwrap();
        assert_eq!(after[to], Slot { active: true, ..slots[0].clone() });
        for (i, s) in after.iter().enumerate().filter(|(i, _)| *i != to) {
            assert_eq!(s, &slots[i]);
        }
        let e = bnd4_entries(&out).unwrap();
        let (a, _, _) = bnd4_open(&out, find(&e, &slot_name(0)).unwrap()).unwrap();
        let (b, _, _) = bnd4_open(&out, find(&e, &slot_name(to)).unwrap()).unwrap();
        assert_eq!(a, b);
        assert!(crate::fmt::fromsoft::parse(&b).is_ok());
        println!("copied to slot {}: {:?}", to + 1, after[to]);
    }
}
