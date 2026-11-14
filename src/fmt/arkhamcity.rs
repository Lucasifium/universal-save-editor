//! Batman: Arkham City (PC) saves (`*.sgd`, 294,912 bytes): a 40-byte wrapper of big-endian words, then LZO1X
//! segments (each starting `9E 2A 83 C1`) that together hold one big-endian record. The record starts with the
//! options and the Riddler/map lists and reaches the experience and upgrade counters after a long run of arrays.
//!
//! The segment container and the order of the record follow ShikenNuggets/ArkhamSaveParser (MIT; see
//! THIRD_PARTY_NOTICES.md). Segment boundaries stay where they are; every segment is compressed again.

use super::fields::{Builder, Checksum, Kind};
use crate::tree::TreeDoc;

pub const KIND: &str = "Batman: Arkham City save";
const TAG: [u8; 4] = [0x9e, 0x2a, 0x83, 0xc1];
const BLOCK: usize = 0x20000;
const FILE: usize = 294_912;

struct Seg {
    /// Size of the whole segment in the file and once decompressed.
    len: usize,
    unc: usize,
}

fn be(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

/// The segments and the decompressed record.
fn unpack(d: &[u8]) -> Option<(Vec<Seg>, Vec<u8>)> {
    if d.len() < 80 || be(d, 0)? != 3 || be(d, 4)? != 1 || be(d, 20)? != 4 || d[40..44] != TAG {
        return None;
    }
    let (mut segs, mut rec, mut p) = (vec![], vec![], 40);
    while d.get(p..p + 4) == Some(&TAG[..]) {
        let (comp, unc) = (be(d, p + 8)? as usize, be(d, p + 12)? as usize);
        let n = unc.div_ceil(BLOCK);
        if be(d, p + 4)? as usize != BLOCK || n == 0 || segs.len() > 8 {
            return None;
        }
        let mut at = p + 16 + 8 * n;
        for i in 0..n {
            let (bc, bu) = (be(d, p + 16 + 8 * i)? as usize, be(d, p + 20 + 8 * i)? as usize);
            let data = d.get(at..at.checked_add(bc)?)?;
            let out = std::panic::catch_unwind(|| lzokay_native::decompress_all(data, Some(bu))).ok()?.ok()?;
            if out.len() != bu {
                return None;
            }
            rec.extend(out);
            at += bc;
        }
        if at - p - 16 - 8 * n != comp {
            return None;
        }
        segs.push(Seg { len: at - p, unc });
        p = at;
    }
    (!segs.is_empty() && rec.len() == segs.iter().map(|s| s.unc).sum::<usize>()).then_some((segs, rec))
}

struct Rd<'a> {
    d: &'a [u8],
    p: usize,
}

impl Rd<'_> {
    fn skip(&mut self, n: usize) -> Option<()> {
        self.p = self.p.checked_add(n).filter(|&p| p <= self.d.len())?;
        Some(())
    }
    fn u(&mut self) -> Option<u32> {
        let v = be(self.d, self.p)?;
        self.p += 4;
        Some(v)
    }
    /// A counted list of `size`-byte entries; the count is returned.
    fn list(&mut self, size: usize) -> Option<u32> {
        let n = self.u()?;
        self.skip((n as usize).checked_mul(size)?)?;
        Some(n)
    }
    /// An Unreal string: a signed length (negative for UTF-16) and the characters with their terminator.
    fn text(&mut self) -> Option<()> {
        let n = self.u()? as i32;
        self.skip(if n < 0 { n.unsigned_abs() as usize * 2 } else { n as usize })
    }
    fn texts(&mut self) -> Option<()> {
        for _ in 0..self.u()? {
            self.text()?;
        }
        Some(())
    }
}

/// Where the experience block (xp, xp at the previous upgrade, xp for the next one, xp level, then four counters) starts.
fn xp_at(d: &[u8]) -> Option<usize> {
    let mut r = Rd { d, p: 0 };
    if r.u()? != 16 || r.u()? != 12 {
        return None;
    }
    r.skip(4 * (7 + 3 + 22 + 3 + 1 + 2))?;
    for _ in 0..r.u()? {
        r.u()?;
        let sub = r.u()? as usize;
        r.skip(sub.checked_mul(19)?)?;
    }
    r.skip(32)?;
    r.list(1)?;
    r.skip(8)?;
    r.texts()?;
    r.skip(2 + 4 + 4 + 40)?;
    r.text()?;
    r.skip(24 + 8)?;
    r.texts()?;
    r.texts()?;
    r.skip(8)?;
    r.list(1)?;
    r.list(1)?;
    for n in [32, 32, 32, 91, 91, 81, 81, 82, 63] {
        if r.u()? != n {
            return None;
        }
        r.skip(n as usize)?;
    }
    r.skip(201)?;
    r.list(4)?;
    r.list(4)?;
    r.skip(8)?;
    r.list(1)?;
    for _ in 0..20 {
        r.list(4)?;
    }
    r.skip(36)?;
    for _ in 0..3 {
        r.list(4)?;
    }
    (r.p + 32 <= d.len()).then_some(r.p)
}

pub fn looks(d: &[u8]) -> bool {
    unpack(d).is_some_and(|(_, rec)| xp_at(&rec).is_some())
}

fn view(rec: &[u8]) -> Option<Vec<u8>> {
    let x = xp_at(rec)?;
    let mut inner = Vec::with_capacity(36);
    inner.extend(rec[8..12].iter().rev());
    // xp, previous, next, level (floats) then the four counters, stored in this order below.
    for off in [0, 4, 8, 28, 20, 24] {
        inner.extend(rec[x + off..x + off + 4].iter().rev());
    }
    Some(inner)
}

fn put(outer: &[u8], inner: &[u8]) -> Vec<u8> {
    repack(outer, inner).unwrap_or_else(|| outer.to_vec())
}

fn repack(outer: &[u8], inner: &[u8]) -> Option<Vec<u8>> {
    let (segs, mut rec) = unpack(outer)?;
    let x = xp_at(&rec)?;
    let flip = |at: usize| -> [u8; 4] { [inner[at + 3], inner[at + 2], inner[at + 1], inner[at]] };
    rec[8..12].copy_from_slice(&flip(0));
    for (i, off) in [0, 4, 8, 28, 20, 24].into_iter().enumerate() {
        rec[x + off..x + off + 4].copy_from_slice(&flip(4 + 4 * i));
    }
    let mut out = outer[..40].to_vec();
    let mut from = 0;
    for (k, s) in segs.iter().enumerate() {
        let old = s.len as u32;
        let mut seg = vec![];
        let mut blocks = vec![];
        for part in rec[from..from + s.unc].chunks(BLOCK) {
            blocks.push((lzokay_native::compress(part).ok()?, part.len()));
        }
        from += s.unc;
        let comp: usize = blocks.iter().map(|b| b.0.len()).sum();
        seg.extend(TAG);
        seg.extend((BLOCK as u32).to_be_bytes());
        seg.extend((comp as u32).to_be_bytes());
        seg.extend((s.unc as u32).to_be_bytes());
        for (c, u) in &blocks {
            seg.extend((c.len() as u32).to_be_bytes());
            seg.extend((*u as u32).to_be_bytes());
        }
        for (c, _) in &blocks {
            seg.extend(c);
        }
        // The wrapper lists each segment's size in word 3, 4, 6, 9 (segment 1, 2, 3, 4).
        let word = [3, 4, 6, 9].get(k).map(|w| w * 4)?;
        if be(&out, word)? == old {
            out[word..word + 4].copy_from_slice(&(seg.len() as u32).to_be_bytes());
        }
        out.extend(seg);
    }
    if out.len() > outer.len().max(FILE) {
        return None;
    }
    out.resize(outer.len().max(FILE), 0);
    Some(out)
}

pub fn parse(d: &[u8]) -> Result<TreeDoc, String> {
    let (_, rec) = unpack(d).ok_or("not a Batman: Arkham City save")?;
    let inner = view(&rec).ok_or("not a Batman: Arkham City save")?;
    let mut b = Builder::new(&inner);
    b.inside(d, put);
    let g = "Progress";
    b.named(&["Options", "Difficulty"], 0, Kind::U32, &["Easy", "Normal", "Hard"]);
    b.ranged(&[g, "Experience"], 4, Kind::F32Int, 0, 99_999_999);
    b.ranged(&[g, "Experience at the last upgrade point"], 8, Kind::F32Int, 0, 99_999_999);
    b.ranged(&[g, "Experience for the next upgrade point"], 12, Kind::F32Int, 0, 99_999_999);
    b.ranged(&[g, "Player level"], 16, Kind::U32, 0, 999);
    b.ranged(&[g, "Upgrades bought (Batman)"], 20, Kind::U32, 0, 999);
    b.ranged(&[g, "Upgrades bought (Catwoman)"], 24, Kind::U32, 0, 999);
    Ok(b.finish(KIND, Checksum::None).with_note(crate::i18n::tr("The save is compressed again when you write it. Experience is checked by the game at the next load.")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_saves_round_trip_and_edit() {
        let Ok(dir) = std::env::var("ARKHAM_SAMPLES") else { return };
        for f in std::fs::read_dir(dir).unwrap().flatten() {
            let d = std::fs::read(f.path()).unwrap();
            if f.file_name() == "DefaultSave.sgd" {
                // A brand-new game has no experience block yet.
                assert!(!looks(&d));
                continue;
            }
            assert!(looks(&d), "{:?}", f.path());
            let mut doc = parse(&d).unwrap();
            assert_eq!(doc.to_bytes().unwrap(), d);
            doc.set("/Progress/Experience", 123_456.into()).unwrap();
            doc.set("/Options/Difficulty", "Hard".into()).unwrap();
            let out = doc.to_bytes().unwrap();
            assert_eq!(out.len(), d.len());
            let again = parse(&out).unwrap();
            assert_eq!(again.view.pointer("/Progress/Experience"), Some(&serde_json::Value::from(123_456)));
            assert_eq!(again.view.pointer("/Options/Difficulty"), Some(&serde_json::Value::from("Hard")));
        }
    }
}
