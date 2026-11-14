//! Nintendo 64 Controller Pak images (`.mpk`: one 32 KiB pak, or four of them in 128 KiB; or an
//! emulator's combined `.srm`, whose first pak starts at 0x800). A pak is 128 pages of 256
//! bytes: page 0 holds the ID block, pages 1-2 the page chain table, pages 3-4 the sixteen
//! notes (game code, publisher, first page). Saves of games that keep their data on the pak
//! are read from, and written back to, the pages of their notes.
//!
//! After the Controller Pak reader of RyudoSynbios/game-tools-collection (MIT; see THIRD_PARTY_NOTICES.md).

const PAGE: usize = 0x100;

/// Where the first pak starts in the file, if the file is one and its ID block's sum is right.
pub fn base(d: &[u8]) -> Option<usize> {
    let b = match d.len() {
        0x8000 | 0x2_0000 => 0,
        296_960 => 0x800,
        _ => return None,
    };
    let o = b + 0x20;
    let s = (0..0x1c).step_by(2).map(|i| u32::from(u16::from_be_bytes([d[o + i], d[o + i + 1]]))).sum::<u32>() & 0xffff;
    let want = (s << 16) | (0xfff2u32.wrapping_sub(s) & 0xffff);
    (u32::from_be_bytes(d[o + 0x1c..o + 0x20].try_into().ok()?) == want).then_some(b)
}

/// One note: its game code and the pages it is spread over, in order.
pub struct Note {
    pub code: [u8; 4],
    pub pages: Vec<usize>,
}

pub fn notes(d: &[u8], b: usize) -> Vec<Note> {
    let mut out = vec![];
    for n in 0..16 {
        let o = b + 3 * PAGE + n * 0x20;
        let first = usize::from(u16::from_be_bytes([d[o + 6], d[o + 7]]));
        if first == 0 {
            continue;
        }
        let (mut pages, mut p, mut ok) = (vec![first], first, first >= 5 && first < 0x80);
        while ok {
            let next = usize::from(u16::from_be_bytes([d[b + PAGE + p * 2], d[b + PAGE + p * 2 + 1]]));
            if next == 1 {
                break;
            }
            if next < 5 || next >= 0x80 || pages.len() >= 0x80 || pages.contains(&next) {
                ok = false;
                break;
            }
            pages.push(next);
            p = next;
        }
        if ok {
            out.push(Note { code: [d[o], d[o + 1], d[o + 2], d[o + 3]], pages });
        }
    }
    out
}

/// The notes `pick` accepts (only the first one unless `all`), joined.
pub fn take(d: &[u8], pick: fn(&[u8; 4]) -> bool, all: bool) -> Option<(Vec<[u8; 4]>, Vec<u8>)> {
    let b = base(d)?;
    let mut codes = vec![];
    let mut data = vec![];
    for n in notes(d, b).into_iter().filter(|n| pick(&n.code)) {
        for p in &n.pages {
            data.extend_from_slice(&d[b + p * PAGE..b + (p + 1) * PAGE]);
        }
        codes.push(n.code);
        if !all {
            break;
        }
    }
    (!codes.is_empty()).then_some((codes, data))
}

/// Writes `inner` (as `take` gave it) back over the pages of the same notes.
pub fn put(outer: &[u8], inner: &[u8], pick: fn(&[u8; 4]) -> bool, all: bool) -> Vec<u8> {
    let mut o = outer.to_vec();
    let Some(b) = base(outer) else { return o };
    let mut at = 0;
    for n in notes(outer, b).into_iter().filter(|n| pick(&n.code)) {
        for p in &n.pages {
            if at + PAGE <= inner.len() {
                o[b + p * PAGE..b + (p + 1) * PAGE].copy_from_slice(&inner[at..at + PAGE]);
            }
            at += PAGE;
        }
        if !all {
            break;
        }
    }
    o
}

/// A pak holding one note per entry of `notes` (code, data), for tests.
#[cfg(test)]
pub fn build(notes: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut d = vec![0u8; 0x8000];
    let s = (0..0x1c).step_by(2).map(|i| u32::from(u16::from_be_bytes([d[0x20 + i], d[0x21 + i]]))).sum::<u32>() & 0xffff;
    let id = (s << 16) | (0xfff2 - s);
    d[0x3c..0x40].copy_from_slice(&id.to_be_bytes());
    for p in 5..0x80 {
        d[PAGE + p * 2 + 1] = 3;
    }
    let mut page = 5;
    for (n, (code, data)) in notes.iter().enumerate() {
        let o = 3 * PAGE + n * 0x20;
        d[o..o + 4].copy_from_slice(code);
        d[o + 6..o + 8].copy_from_slice(&(page as u16).to_be_bytes());
        let count = data.len().div_ceil(PAGE);
        for i in 0..count {
            let p = page + i;
            let next: u16 = if i + 1 == count { 1 } else { (p + 1) as u16 };
            d[PAGE + p * 2..PAGE + p * 2 + 2].copy_from_slice(&next.to_be_bytes());
            let part = &data[i * PAGE..data.len().min((i + 1) * PAGE)];
            d[p * PAGE..p * PAGE + part.len()].copy_from_slice(part);
        }
        page += count;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_notes() {
        let d = build(&[(*b"NAAE", vec![1u8; 0x200]), (*b"NBBE", vec![2u8; 0x100])]);
        assert_eq!(base(&d), Some(0));
        let (codes, data) = take(&d, |c| c == b"NBBE", false).unwrap();
        assert_eq!(codes, vec![*b"NBBE"]);
        assert_eq!(data, vec![2u8; 0x100]);
        let out = put(&d, &[9u8; 0x100], |c| c == b"NBBE", false);
        assert_eq!(&out[7 * PAGE..8 * PAGE], &[9u8; 0x100][..]);
        assert_eq!(&out[5 * PAGE..7 * PAGE], &d[5 * PAGE..7 * PAGE]);
        let mut bad = d.clone();
        bad[0x3c] ^= 1;
        assert_eq!(base(&bad), None);
    }
}
