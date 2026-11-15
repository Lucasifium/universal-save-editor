//! Kingdom Hearts HD 1.5+2.5 ReMIX (PC) `KHFM_WW.png` and `KHIIFM_WW.png`: a PNG whose private
//! `sqEX` chunk is an archive (a 0x158-byte directory record per entry, then the entries' data at a
//! fixed stride: 200 entries 0x16C40 apart for Kingdom Hearts, 100 entries 0x10FC0 apart for
//! Kingdom Hearts II). Every save slot is the PlayStation 2 Final Mix save (93,184 or 69,568
//! bytes), so the PlayStation 2 readers open it. The PNG chunk CRC is written again on save.
//!
//! The archive layout follows Alba6rek/Kingdom-Hearts-1FM-Completion-Check (GPL-3.0; see
//! THIRD_PARTY_NOTICES.md).

pub const CHUNK: [u8; 4] = *b"sqEX";
const DIR: usize = 0x10;
const RECORD: usize = 0x158;

/// One game's archive: record count, entry stride, save size, and a test of a save's first bytes.
struct Layout {
    count: usize,
    stride: usize,
    save: usize,
    check: fn(&[u8]) -> bool,
}

const LAYOUTS: [Layout; 2] = [
    Layout { count: 200, stride: 0x16C40, save: 93_184, check: |d| d.len() > 8 && (4..=5).contains(&d[0]) && d[1..4] == [0, 0, 0] },
    Layout { count: 100, stride: 0x10FC0, save: 69_568, check: |d| d.starts_with(b"KH2J") },
];

/// The `sqEX` chunk's data.
pub fn chunk(png: &[u8]) -> Option<&[u8]> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let mut i = 8;
    while i + 12 <= png.len() {
        let len = u32::from_be_bytes(png[i..i + 4].try_into().ok()?) as usize;
        if &png[i + 4..i + 8] == &CHUNK {
            return png.get(i + 8..i + 8 + len);
        }
        i += 12 + len;
    }
    None
}

/// (layout, every save slot as (archive index, name shown, offset of its data in the chunk)).
fn slots(c: &[u8]) -> Option<(&'static Layout, Vec<(usize, String, usize)>)> {
    for l in &LAYOUTS {
        let Some(dir) = c.get(DIR..DIR + l.count * RECORD) else { continue };
        let mut dir = dir.to_vec();
        // The first 0xF0 bytes of the directory are XORed with the 16 bytes at 0xE0.
        let key: Vec<u8> = dir[0xE0..0xF0].to_vec();
        for (k, b) in dir[..0xF0].iter_mut().enumerate() {
            *b ^= key[k % 16];
        }
        let base = DIR + l.count * RECORD;
        let mut v = vec![];
        for i in 0..l.count {
            let r = &dir[i * RECORD..(i + 1) * RECORD];
            let len = i32::from_le_bytes(r[0x50..0x54].try_into().ok()?);
            let at = base + i * l.stride;
            if len as usize != l.save || at + l.save > c.len() {
                continue;
            }
            let end = r[..0x40].iter().position(|&b| b == 0).unwrap_or(0x40);
            let name = String::from_utf8_lossy(&r[..end]).to_string();
            v.push((i, format!("Slot {}: {name}", v.len() + 1), at));
        }
        if v.first().is_some_and(|e| (l.check)(&c[e.2..e.2 + l.save])) {
            return Some((l, v));
        }
    }
    None
}

pub fn looks(png: &[u8]) -> bool {
    chunk(png).and_then(slots).is_some()
}

/// The chosen slot (by its listed name; the first when none is picked), every slot's name, and
/// the slot's save.
pub fn unpack(png: &[u8], pick: Option<&str>) -> Result<(usize, Vec<String>, Vec<u8>), String> {
    let c = chunk(png).ok_or("no Kingdom Hearts save data in this picture")?;
    let (l, all) = slots(c).ok_or("no saved game in this file yet")?;
    let i = pick.and_then(|n| all.iter().position(|e| e.1 == n)).unwrap_or(0);
    let at = all[i].2;
    let data = c[at..at + l.save].to_vec();
    Ok((i, all.into_iter().map(|e| e.1).collect(), data))
}

/// The chunk with slot `slot` (its position in the list) replaced by `data`.
pub fn repack(png: &[u8], slot: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    let c = chunk(png).ok_or("the save's picture has lost its save data")?;
    let (l, all) = slots(c).ok_or("the save archive is damaged")?;
    if data.len() != l.save {
        return Err("a Kingdom Hearts save must keep its size".into());
    }
    let at = all.get(slot).ok_or("no such save slot")?.2;
    let mut out = c.to_vec();
    out[at..at + l.save].copy_from_slice(data);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_sample() {
        let Ok(p) = std::env::var("KHFM_SAMPLE") else { return };
        let d = std::fs::read(p).unwrap();
        assert!(looks(&d));
        let (i, names, save) = unpack(&d, None).unwrap();
        assert_eq!(i, 0);
        assert!(names.len() >= 1);
        assert!(crate::fmt::kh1::looks(&save) || crate::fmt::kh2::looks(&save));
        assert_eq!(repack(&d, 0, &save).unwrap(), chunk(&d).unwrap());
        for n in &names {
            let (_, _, s) = unpack(&d, Some(n)).unwrap();
            assert!(crate::fmt::kh1::looks(&s) || crate::fmt::kh2::looks(&s), "{n}");
        }
    }
}
