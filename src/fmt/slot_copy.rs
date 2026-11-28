//! Copying a save slot to another slot of the same file, for saves where a slot is a
//! self-contained block: Elden Ring characters, and the slot-based saves of Final Fantasy VII
//! (PC), Super Mario 64, Ocarina of Time, A Link to the Past, Super Metroid, Chrono Trigger and
//! Final Fantasy VI. The console games offer the same copy in their own file menus; this does
//! what they do (the slot's bytes, its spare copy where the game keeps one, and its checksum).
//!
//! **Elden Ring.** Each character is one archive entry (USER_DATA000…009, the same size). The
//! title screen reads USER_DATA010: one "in use" byte per slot at 0x1954, then a 0x24C-byte
//! summary per slot at 0x195E (name in UTF-16 first, level at +0x22). A copy is the slot's
//! entry as it is (its MD5 covers only itself), plus the summary and the "in use" byte;
//! USER_DATA010 then gets a new MD5. Offsets checked against a sample save with three characters.
//!
//! Every copy is read back before it is offered for writing: the file still reads as the
//! same kind of save, the target slot holds exactly the source slot's data, and every other
//! slot is byte for byte as it was.

use super::containers::{bnd4_entries, bnd4_open, bnd4_replace, Bnd4Entry};
use super::{chronotrigger as ct, ff6snes as ff6, ff7, sm64, supermetroid as smetroid, zelda64 as oot, zelda_lttp as lttp};
use crate::i18n::{tr, trf};

/// What a slot holds, as the game's own file menu would show it.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub used: bool,
    /// A short description ("Cloud, level 23, 12:05"); empty for an unused slot.
    pub label: String,
}

/// One kind of multi-slot save.
pub struct Family {
    /// The game, as the menus name it.
    pub game: &'static str,
    /// The name the format's reader gives an open file of this kind (a slot copy is offered only
    /// when the file was opened as this, so a file two readers accept is never copied wrongly).
    pub kind: &'static str,
    detect: fn(&[u8]) -> bool,
    slots: fn(&[u8]) -> Vec<Slot>,
    /// The bytes that make up slot `n` (what must match after a copy).
    block: fn(&[u8], usize) -> Option<Vec<u8>>,
    copy: fn(&[u8], usize, usize) -> Result<Vec<u8>, String>,
}

pub static FAMILIES: &[Family] = &[
    Family { game: "Elden Ring", kind: "Elden Ring character", detect: |d| profile(d).is_some(), slots: er_family_slots, block: er_block, copy: er_copy },
    Family { game: "Final Fantasy VII", kind: "Final Fantasy VII save", detect: ff7::looks, slots: ff7_slots, block: ff7_block, copy: ff7_copy },
    Family { game: "Super Mario 64", kind: sm64::KIND, detect: sm64::looks, slots: sm64_slots, block: sm64_block, copy: sm64_copy },
    Family { game: "The Legend of Zelda: Ocarina of Time", kind: oot::KIND, detect: oot::looks, slots: oot_slots, block: oot_block, copy: oot_copy },
    Family { game: "The Legend of Zelda: A Link to the Past", kind: lttp::KIND, detect: lttp::looks, slots: lttp_slots, block: lttp_block, copy: lttp_copy },
    Family { game: "Super Metroid", kind: smetroid::KIND, detect: smetroid::looks, slots: smetroid_slots, block: smetroid_block, copy: smetroid_copy },
    Family { game: "Chrono Trigger", kind: ct::KIND, detect: ct::looks, slots: ct_slots, block: ct_block, copy: ct_copy },
    Family { game: "Final Fantasy VI", kind: ff6::KIND, detect: ff6::looks, slots: ff6_slots, block: ff6_block, copy: ff6_copy },
];

/// The kind of multi-slot save this is, if any.
pub fn family(d: &[u8]) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| (f.detect)(d))
}

/// The slots of a multi-slot save, or `None` when slots can't be copied in this file.
pub fn slots(d: &[u8]) -> Option<(&'static Family, Vec<Slot>)> {
    let f = family(d)?;
    let s = (f.slots)(d);
    (s.len() > 1 && s.iter().any(|x| x.used)).then_some((f, s))
}

/// The save with slot `from` copied over slot `to`, read back and checked (see the module notes).
pub fn copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    let f = family(d).ok_or_else(|| tr("this kind of save has no slots that can be copied").to_string())?;
    let before = (f.slots)(d);
    if from == to || from >= before.len() || to >= before.len() {
        return Err(tr("pick two different slots").into());
    }
    if !before[from].used {
        return Err(tr("that slot is empty").into());
    }
    let out = (f.copy)(d, from, to)?;
    verify(f, d, &out, from, to)?;
    Ok(out)
}

fn verify(f: &Family, before: &[u8], out: &[u8], from: usize, to: usize) -> Result<(), String> {
    if out.len() != before.len() || !(f.detect)(out) {
        return Err(tr("the new file doesn't read as the same kind of save").into());
    }
    let (a, b) = ((f.slots)(before), (f.slots)(out));
    if a.len() != b.len() {
        return Err(tr("the new file doesn't read as the same kind of save").into());
    }
    let block = |d: &[u8], i: usize| (f.block)(d, i).ok_or_else(|| trf("slot {0} can't be read", &[&(i + 1)]));
    for i in 0..a.len() {
        let (want_slot, want) = if i == to { (Slot { used: true, ..a[from].clone() }, block(before, from)?) } else { (a[i].clone(), block(before, i)?) };
        if b[i] != want_slot || block(out, i)? != want {
            return Err(trf("slot {0} doesn't read back as expected", &[&(i + 1)]));
        }
    }
    Ok(())
}

fn u16le(d: &[u8], o: usize) -> u16 {
    d.get(o..o + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
}

fn u16be(d: &[u8], o: usize) -> u16 {
    d.get(o..o + 2).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]))
}

fn clock(h: u32, m: u32) -> String {
    format!("{h}:{m:02}")
}

fn unused() -> Slot {
    Slot { used: false, label: String::new() }
}

/// Copies `len` bytes at `from` to `to` in a copy of `d`.
fn moved(d: &[u8], from: usize, to: usize, len: usize) -> Result<Vec<u8>, String> {
    if from.max(to) + len > d.len() {
        return Err(tr("the file is shorter than its slots").into());
    }
    let mut out = d.to_vec();
    out.copy_within(from..from + len, to);
    Ok(out)
}

// ---------------------------------------------------------------- Elden Ring

const ER_SLOTS: usize = 10;
const ACTIVE: usize = 0x1954;
const SUMMARY: usize = 0x195E;
const SUMMARY_LEN: usize = 0x24C;
const PROFILE_LEN: usize = 0x6_0000;

/// What the Elden Ring title screen shows for a slot.
#[derive(Clone, Debug, PartialEq)]
pub struct ErSlot {
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
    if !d.starts_with(b"BND4") {
        return None;
    }
    let entries = bnd4_entries(d)?;
    let size = find(&entries, &slot_name(0))?.size;
    if (0..ER_SLOTS).any(|i| find(&entries, &slot_name(i)).is_none_or(|e| e.size != size)) {
        return None;
    }
    let (data, key, _) = bnd4_open(d, find(&entries, "USER_DATA010")?)?;
    (key.is_none() && data.len() == PROFILE_LEN).then_some((entries, data))
}

/// The ten slots, or `None` when this isn't an Elden Ring `.sl2`.
pub fn er_slots(d: &[u8]) -> Option<Vec<ErSlot>> {
    let (_, p) = profile(d)?;
    Some(
        (0..ER_SLOTS)
            .map(|i| {
                let s = SUMMARY + i * SUMMARY_LEN;
                let units: Vec<u16> = p[s..s + 32].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
                ErSlot { active: p[ACTIVE + i] == 1, name: String::from_utf16_lossy(&units), level: u32::from_le_bytes([p[s + 0x22], p[s + 0x23], p[s + 0x24], p[s + 0x25]]) }
            })
            .collect(),
    )
}

fn er_family_slots(d: &[u8]) -> Vec<Slot> {
    er_slots(d)
        .unwrap_or_default()
        .into_iter()
        .map(|s| if s.active { Slot { used: true, label: trf("{0}, level {1}", &[&s.name, &s.level]) } } else { unused() })
        .collect()
}

/// A character's data, checked against its MD5.
fn er_block(d: &[u8], i: usize) -> Option<Vec<u8>> {
    let entries = bnd4_entries(d)?;
    bnd4_open(d, find(&entries, &slot_name(i))?).map(|x| x.0)
}

/// The save with character `from` copied over slot `to` (not yet checked: see `copy`).
fn er_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    let (entries, mut p) = profile(d).ok_or_else(|| tr("this isn't an Elden Ring save with ten slots").to_string())?;
    if p[ACTIVE + from] != 1 {
        return Err(tr("that slot is empty").into());
    }
    let (Some(src), Some(dst)) = (find(&entries, &slot_name(from)), find(&entries, &slot_name(to))) else {
        return Err(tr("this isn't an Elden Ring save with ten slots").into());
    };
    // The source must be intact (its MD5 checks out) before it's duplicated.
    bnd4_open(d, src).ok_or_else(|| tr("the character's data is damaged").to_string())?;
    let mut out = d.to_vec();
    out[dst.offset..dst.offset + dst.size].copy_from_slice(&d[src.offset..src.offset + src.size]);
    p[ACTIVE + to] = 1;
    let (a, b) = (SUMMARY + from * SUMMARY_LEN, SUMMARY + to * SUMMARY_LEN);
    p.copy_within(a..a + SUMMARY_LEN, b);
    bnd4_replace(&out, "USER_DATA010", &p, None, &[])
}

// ---------------------------------------------------------------- Final Fantasy VII (PC)

/// Slot `n`'s offset; a slot holds its own CRC, so it can move as it is.
fn ff7_at(n: usize) -> usize {
    ff7::HEADER + n * ff7::SLOT
}

fn ff7_slots(d: &[u8]) -> Vec<Slot> {
    (0..ff7::SLOTS)
        .map(|n| {
            let s = ff7_at(n);
            let Some(sl) = d.get(s..s + ff7::SLOT) else { return unused() };
            if sl[4..].iter().all(|&b| b == 0) {
                return unused();
            }
            let secs = u32::from_le_bytes([sl[0x24], sl[0x25], sl[0x26], sl[0x27]]);
            let name = ff7::text(&sl[0x08..0x18]);
            Slot { used: true, label: trf("{0}, level {1}, {2}", &[&name, &sl[0x04], &clock(secs / 3600, secs / 60 % 60)]) }
        })
        .collect()
}

fn ff7_block(d: &[u8], n: usize) -> Option<Vec<u8>> {
    d.get(ff7_at(n)..ff7_at(n) + ff7::SLOT).map(<[u8]>::to_vec)
}

fn ff7_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    moved(d, ff7_at(from), ff7_at(to), ff7::SLOT)
}

// ---------------------------------------------------------------- Super Mario 64

/// Four files; each is two copies of 0x38 bytes that carry their own sums. The game's own
/// "copy file" moves the whole 0x70 bytes and leaves the menu record alone.
fn sm64_count(d: &[u8]) -> usize {
    if d.len() >= 4 * sm64::SLOT { 4 } else { 0 }
}

fn sm64_slots(d: &[u8]) -> Vec<Slot> {
    (0..sm64_count(d))
        .map(|n| if sm64::used(d, n) { Slot { used: true, label: trf("{0} stars", &[&sm64::stars(d, n * sm64::SLOT)]) } } else { unused() })
        .collect()
}

fn sm64_block(d: &[u8], n: usize) -> Option<Vec<u8>> {
    d.get(n * sm64::SLOT..(n + 1) * sm64::SLOT).map(<[u8]>::to_vec)
}

fn sm64_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    moved(d, from * sm64::SLOT, to * sm64::SLOT, sm64::SLOT)
}

// ---------------------------------------------------------------- Ocarina of Time

/// File `n` inside the SRAM (after its 0x20-byte header); its backup is three files later.
fn oot_at(n: usize) -> usize {
    0x20 + n * oot::FILE
}

fn oot_slots(d: &[u8]) -> Vec<Slot> {
    let Some((_, s)) = oot::find(d) else { return vec![] };
    (0..3)
        .map(|n| {
            let b = n * oot::FILE;
            if !oot::file_ok(&s, b) {
                return unused();
            }
            // Max health is in sixteenths of a heart.
            Slot { used: true, label: trf("{0} hearts", &[&(u16be(&s, b + 0x4e) / 16)]) }
        })
        .collect()
}

fn oot_block(d: &[u8], n: usize) -> Option<Vec<u8>> {
    let (_, s) = oot::find(d)?;
    s.get(oot_at(n)..oot_at(n) + oot::FILE).map(<[u8]>::to_vec)
}

/// Like the game's own copy: the file goes to the target and to the target's backup.
fn oot_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    let (_, s) = oot::find(d).ok_or_else(|| tr("this isn't an Ocarina of Time save").to_string())?;
    let mut s = moved(&s, oot_at(from), oot_at(to), oot::FILE)?;
    s.copy_within(oot_at(from)..oot_at(from) + oot::FILE, oot_at(to) + oot::BACKUP);
    Ok(oot::put(d, &s))
}

// ---------------------------------------------------------------- A Link to the Past

/// The spare copy the game keeps of each slot.
const LTTP_SPARE: usize = 0xf00;

fn lttp_used(d: &[u8], n: usize) -> bool {
    d.len() >= 0x2000 && d[n * lttp::SLOT + 0x36c] != 0 && lttp::valid(d, n)
}

fn lttp_slots(d: &[u8]) -> Vec<Slot> {
    (0..3)
        .map(|n| if lttp_used(d, n) { Slot { used: true, label: trf("{0} hearts", &[&(d[n * lttp::SLOT + 0x36c] / 8)]) } } else { unused() })
        .collect()
}

fn lttp_block(d: &[u8], n: usize) -> Option<Vec<u8>> {
    d.get(n * lttp::SLOT..(n + 1) * lttp::SLOT).map(<[u8]>::to_vec)
}

/// The slot (with its own sum) goes to the target and to the target's spare, as the game copies.
fn lttp_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    let mut out = moved(d, from * lttp::SLOT, to * lttp::SLOT, lttp::SLOT)?;
    out.copy_within(from * lttp::SLOT..(from + 1) * lttp::SLOT, to * lttp::SLOT + LTTP_SPARE);
    Ok(out)
}

// ---------------------------------------------------------------- Super Metroid

/// Slot `n`'s data; its sums live in two tables (at 0 and 0x1ff0).
fn smetroid_at(n: usize) -> usize {
    0x10 + n * smetroid::SLOT
}

fn smetroid_slots(d: &[u8]) -> Vec<Slot> {
    (0..3)
        .map(|n| {
            if !smetroid::used(d, n) {
                return unused();
            }
            let s = n * smetroid::SLOT;
            Slot { used: true, label: trf("{0}, {1} energy", &[&clock(u16le(d, s + 0x4e) as u32, u16le(d, s + 0x4c) as u32), &u16le(d, s + 0x32)]) }
        })
        .collect()
}

/// The slot's data and its four sum entries.
fn smetroid_block(d: &[u8], n: usize) -> Option<Vec<u8>> {
    let mut b = d.get(smetroid_at(n)..smetroid_at(n) + smetroid::SLOT)?.to_vec();
    for t in [0, smetroid::COPY] {
        b.extend(d.get(t + n * 2..t + n * 2 + 2)?);
        b.extend(d.get(t + 8 + n * 2..t + 10 + n * 2)?);
    }
    Some(b)
}

fn smetroid_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    let mut out = moved(d, smetroid_at(from), smetroid_at(to), smetroid::SLOT)?;
    let sum = out[smetroid_at(to)..smetroid_at(to) + smetroid::SLOT].chunks(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])));
    for t in [0, smetroid::COPY] {
        out[t + to * 2..t + to * 2 + 2].copy_from_slice(&sum.to_le_bytes());
        out[t + 8 + to * 2..t + 10 + to * 2].copy_from_slice(&(sum ^ 0xffff).to_le_bytes());
    }
    Ok(out)
}

// ---------------------------------------------------------------- Chrono Trigger

/// Each slot's sum sits in a table at the end of the file.
const CT_SUMS: usize = 0x1ff0;

fn ct_slots(d: &[u8]) -> Vec<Slot> {
    (0..3)
        .map(|n| {
            if !ct::valid(d, n) {
                return unused();
            }
            let s = n * ct::SLOT + 0x5e0;
            let gold = u32::from_le_bytes([d[s], d[s + 1], d[s + 2], 0]);
            Slot { used: true, label: trf("{0} gold", &[&gold]) }
        })
        .collect()
}

fn ct_block(d: &[u8], n: usize) -> Option<Vec<u8>> {
    let mut b = d.get(n * ct::SLOT..(n + 1) * ct::SLOT)?.to_vec();
    b.extend(d.get(CT_SUMS + 2 * n..CT_SUMS + 2 * n + 2)?);
    Some(b)
}

fn ct_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    let mut out = moved(d, from * ct::SLOT, to * ct::SLOT, ct::SLOT)?;
    let sum = ct::sum(&out, to * ct::SLOT);
    out[CT_SUMS + 2 * to..CT_SUMS + 2 * to + 2].copy_from_slice(&sum.to_le_bytes());
    Ok(out)
}

// ---------------------------------------------------------------- Final Fantasy VI (SNES)

fn ff6_slots(d: &[u8]) -> Vec<Slot> {
    (0..3)
        .map(|n| {
            if !ff6::valid(d, n) {
                return unused();
            }
            let s = n * ff6::SLOT;
            Slot { used: true, label: clock(d[s + 0x263] as u32, d[s + 0x264] as u32) }
        })
        .collect()
}

fn ff6_block(d: &[u8], n: usize) -> Option<Vec<u8>> {
    d.get(n * ff6::SLOT..(n + 1) * ff6::SLOT).map(<[u8]>::to_vec)
}

/// The slot ends in its own sum, so it moves as it is.
fn ff6_copy(d: &[u8], from: usize, to: usize) -> Result<Vec<u8>, String> {
    moved(d, from * ff6::SLOT, to * ff6::SLOT, ff6::SLOT)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn check_copy(d: &[u8], from: usize, to: usize) -> Vec<u8> {
        let (f, before) = slots(d).expect("a multi-slot save");
        let out = copy(d, from, to).unwrap_or_else(|e| panic!("{}: {e}", f.game));
        let (g, after) = slots(&out).expect("still a multi-slot save");
        assert_eq!(f.game, g.game);
        assert_eq!(after[to], before[from], "{}", f.game);
        for i in (0..before.len()).filter(|&i| i != to) {
            assert_eq!(after[i], before[i], "{}: slot {i}", f.game);
        }
        assert!(copy(d, from, from).is_err(), "same slot");
        out
    }

    #[test]
    fn final_fantasy_vii() {
        let mut d = vec![0u8; ff7::HEADER + ff7::SLOTS * ff7::SLOT];
        let s = ff7_at(0);
        d[s + 4] = 23;
        d[s + 8..s + 13].copy_from_slice(&[0x23, 0x4C, 0x4F, 0x55, 0x44]); // "Cloud"
        d[s + 13] = 0xFF;
        d[s + 0x24..s + 0x28].copy_from_slice(&(3 * 3600 + 5 * 60u32).to_le_bytes());
        d[s + 0x100] = 7;
        assert!(ff7::parse(&d).is_err(), "no CRC yet");
        let crc = {
            // CRC-16 (0x1021, init and final xor 0xFFFF) over the slot after its first 4 bytes.
            let mut r: u32 = 0xFFFF;
            for &t in &d[s + 4..s + ff7::SLOT] {
                r ^= u32::from(t) << 8;
                for _ in 0..8 {
                    r = if r & 0x8000 != 0 { (r << 1) ^ 0x1021 } else { r << 1 };
                    r &= 0xFFFF;
                }
            }
            (r ^ 0xFFFF) as u16
        };
        d[s..s + 2].copy_from_slice(&crc.to_le_bytes());
        assert!(ff7::parse(&d).is_ok());
        let (_, sl) = slots(&d).unwrap();
        assert_eq!(sl[0].label, "Cloud, level 23, 3:05");
        let out = check_copy(&d, 0, 4);
        assert!(ff7::parse(&out).is_ok());
        assert_eq!(&out[ff7_at(4)..ff7_at(5)], &d[s..s + ff7::SLOT]);
        assert!(copy(&d, 2, 3).is_err(), "an empty slot can't be copied");
    }

    #[test]
    fn super_mario_64() {
        let mut d = vec![0u8; 512];
        d[0xb] = 1;
        d[0xc] = 0x7f; // seven stars in the first course
        for c in 0..2 {
            let s = c * 0x38;
            d[s + 0x34..s + 0x36].copy_from_slice(b"DA");
            let sum: u32 = d[s..s + 0x36].iter().map(|&b| b as u32).sum();
            d[s + 0x36..s + 0x38].copy_from_slice(&(sum as u16).to_be_bytes());
        }
        d[0x1c0 + 0x1c..0x1c0 + 0x1e].copy_from_slice(b"HI");
        assert!(sm64::parse(&d).is_ok());
        let out = check_copy(&d, 0, 2);
        assert_eq!(slots(&out).unwrap().1[2].label, "7 stars");
        assert_eq!(&out[2 * 0x70..3 * 0x70], &d[..0x70]);
        assert_eq!(&out[0x1c0..], &d[0x1c0..], "the menu record is left alone");
        assert!(sm64::parse(&out).is_ok());
    }

    #[test]
    fn link_to_the_past() {
        let mut d = vec![0u8; 0x2000];
        d[0x36c] = 24;
        d[0x36d] = 24;
        let sum = 0x5a5au16.wrapping_sub(d[..0x4fe].chunks(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]]))));
        d[0x4fe..0x500].copy_from_slice(&sum.to_le_bytes());
        let out = check_copy(&d, 0, 1);
        assert_eq!(slots(&out).unwrap().1[1].label, "3 hearts");
        assert_eq!(&out[0x500 + LTTP_SPARE..0xa00 + LTTP_SPARE], &d[..0x500], "the spare copy matches");
        assert!(lttp::valid(&out, 1));
    }

    #[test]
    fn super_metroid() {
        let mut d = vec![0u8; 0x2000];
        d[0x1fe0..0x1fec].copy_from_slice(b"supermetroid");
        d[0x32..0x34].copy_from_slice(&299u16.to_le_bytes());
        d[0x4e] = 2;
        d[0x4c] = 7;
        let sum = d[smetroid_at(0)..smetroid_at(0) + smetroid::SLOT].chunks(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])));
        for t in [0, smetroid::COPY] {
            d[t..t + 2].copy_from_slice(&sum.to_le_bytes());
            d[t + 8..t + 10].copy_from_slice(&(sum ^ 0xffff).to_le_bytes());
        }
        let out = check_copy(&d, 0, 2);
        assert_eq!(slots(&out).unwrap().1[2].label, "2:07, 299 energy");
        let sum = out[smetroid_at(2)..smetroid_at(2) + smetroid::SLOT].chunks(2).fold(0u16, |a, w| a.wrapping_add(u16::from_le_bytes([w[0], w[1]])));
        assert_eq!(u16le(&out, 4), sum);
        assert_eq!(u16le(&out, 0x1ff0 + 12), sum ^ 0xffff);
    }

    #[test]
    fn chrono_trigger() {
        let mut d = vec![0u8; 0x2000];
        d[0x1ff8..0x1ffa].copy_from_slice(&[0x1b, 0xe4]);
        d[0x5e0..0x5e3].copy_from_slice(&[0x10, 0x27, 0]); // 10000 gold
        let sum = ct::sum(&d, 0);
        d[0x1ff0..0x1ff2].copy_from_slice(&sum.to_le_bytes());
        let out = check_copy(&d, 0, 1);
        assert_eq!(slots(&out).unwrap().1[1].label, "10000 gold");
        assert!(ct::valid(&out, 1));
    }

    #[test]
    fn final_fantasy_vi() {
        let mut d = vec![0u8; 0x2000];
        d[0x263] = 12;
        d[0x264] = 3;
        let sum = ff6::sum(&d, 0);
        d[0x9fe..0xa00].copy_from_slice(&sum.to_le_bytes());
        let out = check_copy(&d, 0, 2);
        assert_eq!(slots(&out).unwrap().1[2].label, "12:03");
        assert!(ff6::valid(&out, 2));
    }

    #[test]
    fn ocarina_of_time() {
        let mut s = vec![0u8; 0x8000];
        let b = 0;
        s[b + 0x3c] = 0x5a;
        s[b + 0x4e..b + 0x50].copy_from_slice(&(48u16).to_be_bytes());
        let sum = s[b + 0x20..b + 0x1372].chunks(2).fold(0u16, |a, c| a.wrapping_add(u16::from_be_bytes([c[0], c[1]])));
        s[b + 0x1372..b + 0x1374].copy_from_slice(&sum.to_be_bytes());
        assert!(oot::looks(&s));
        let out = check_copy(&s, 0, 2);
        assert_eq!(slots(&out).unwrap().1[2].label, "3 hearts");
        assert_eq!(&out[oot_at(2) + oot::BACKUP..oot_at(2) + oot::BACKUP + oot::FILE], &s[oot_at(0)..oot_at(0) + oot::FILE], "the backup matches");
        assert!(oot::file_ok(&out, 2 * oot::FILE));
    }

    /// A BND4 archive of plain entries (each MD5 + data), as the Elden Ring saves are.
    pub(crate) fn bnd4(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
        use md5::{Digest, Md5};
        let names = 0x40 + entries.len() * 0x20;
        let data_at = names + entries.len() * 0x40;
        let mut d = vec![0u8; data_at];
        d[..4].copy_from_slice(b"BND4");
        d[0x0C..0x10].copy_from_slice(&(entries.len() as u32).to_le_bytes());
        d[0x20] = 0x20;
        d[0x30] = 1;
        for (i, (name, data)) in entries.iter().enumerate() {
            let n: Vec<u8> = format!("{name}\0").encode_utf16().flat_map(u16::to_le_bytes).collect();
            let at = names + i * 0x40;
            d[at..at + n.len()].copy_from_slice(&n);
            let e = [Md5::digest(data).to_vec(), data.clone()].concat();
            let h = 0x40 + i * 0x20;
            let off = d.len();
            d[h + 8..h + 16].copy_from_slice(&(e.len() as u64).to_le_bytes());
            d[h + 0x10..h + 0x14].copy_from_slice(&(off as u32).to_le_bytes());
            d[h + 0x14..h + 0x18].copy_from_slice(&(at as u32).to_le_bytes());
            d.extend(e);
        }
        d
    }

    /// An Elden Ring save with characters in the given slots (name, level).
    pub(crate) fn elden_ring(chars: &[(usize, &str, u32)]) -> Vec<u8> {
        let mut profile = vec![0u8; PROFILE_LEN];
        let mut entries = vec![];
        for i in 0..ER_SLOTS {
            let mut slot = vec![0u8; 256];
            if let Some((_, name, level)) = chars.iter().find(|c| c.0 == i) {
                slot[10] = (*level % 251) as u8;
                slot[11] = i as u8 + 1;
                profile[ACTIVE + i] = 1;
                let s = SUMMARY + i * SUMMARY_LEN;
                for (k, u) in name.encode_utf16().enumerate() {
                    profile[s + k * 2..s + k * 2 + 2].copy_from_slice(&u.to_le_bytes());
                }
                profile[s + 0x22..s + 0x26].copy_from_slice(&level.to_le_bytes());
            }
            entries.push((slot_name(i), slot));
        }
        entries.push(("USER_DATA010".to_string(), profile));
        bnd4(&entries)
    }

    #[test]
    fn elden_ring_character() {
        let d = elden_ring(&[(0, "Tarnished", 125), (3, "Other", 40)]);
        let (f, sl) = slots(&d).unwrap();
        assert_eq!(f.game, "Elden Ring");
        assert_eq!(sl[0].label, "Tarnished, level 125");
        assert!(!sl[1].used);
        let out = check_copy(&d, 0, 1);
        assert_eq!(er_block(&out, 1), er_block(&d, 0));
        // Over a used slot too.
        let out = check_copy(&d, 3, 0);
        assert_eq!(er_slots(&out).unwrap()[0], ErSlot { active: true, name: "Other".into(), level: 40 });
        assert!(copy(&d, 5, 6).is_err(), "an empty slot can't be copied");
    }

    #[test]
    fn other_files_have_no_slots() {
        assert!(slots(b"not a save").is_none());
        assert!(slots(&vec![0u8; 0x2000]).is_none());
        assert!(copy(b"not a save", 0, 1).is_err());
    }

    /// Research aid: copies the first character of SAVE_EDITOR_ER into the first free slot
    /// and reads the result back.
    #[test]
    #[ignore]
    fn copy_real_character() {
        let d = std::fs::read(std::env::var("SAVE_EDITOR_ER").expect("SAVE_EDITOR_ER")).unwrap();
        let slots = er_slots(&d).expect("Elden Ring save");
        println!("{slots:?}");
        let to = slots.iter().position(|s| !s.active).unwrap();
        let out = copy(&d, 0, to).unwrap();
        let after = er_slots(&out).unwrap();
        assert_eq!(after[to], ErSlot { active: true, ..slots[0].clone() });
        let b = er_block(&out, to).unwrap();
        assert!(crate::fmt::fromsoft::parse(&b).is_ok());
        println!("copied to slot {}: {:?}", to + 1, after[to]);
    }
}
