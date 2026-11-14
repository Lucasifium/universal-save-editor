//! Kingdom Hearts HD 1.5+2.5 ReMIX (PC) `KHFM_WW.png`: a PNG whose private `sqEX` chunk is an
//! archive of 200 entries (a 0x158-byte directory record each, then the entries' data at a fixed
//! stride). Every save slot is the PlayStation 2 game's own 93,184-byte save, so the
//! Kingdom Hearts reader opens it. The PNG chunk CRC is written again on save.
//!
//! The archive layout follows Alba6rek/Kingdom-Hearts-1FM-Completion-Check (GPL-3.0; see
//! THIRD_PARTY_NOTICES.md).

pub const CHUNK: [u8; 4] = *b"sqEX";
const DIR: usize = 0x10;
const COUNT: usize = 200;
const RECORD: usize = 0x158;
const SAVE: usize = 93_184;

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

/// Entries sit 0x16C40 bytes apart (the save and 0x40 spare bytes); the chunk may end in padding.
fn stride(c: &[u8]) -> Option<usize> {
    let s = SAVE + 0x40;
    (c.len() >= DIR + COUNT * RECORD + COUNT * s).then_some(s)
}

/// (archive index, name shown, offset of its data in the chunk) of every save slot.
fn slots(c: &[u8]) -> Option<Vec<(usize, String, usize)>> {
    let s = stride(c)?;
    let mut dir = c.get(DIR..DIR + COUNT * RECORD)?.to_vec();
    // The first 0xF0 bytes of the directory are XORed with the 16 bytes at 0xE0.
    let key: Vec<u8> = dir[0xE0..0xF0].to_vec();
    for (k, b) in dir[..0xF0].iter_mut().enumerate() {
        *b ^= key[k % 16];
    }
    let mut v = vec![];
    for i in 0..COUNT {
        let r = &dir[i * RECORD..(i + 1) * RECORD];
        let len = i32::from_le_bytes(r[0x50..0x54].try_into().ok()?);
        if len as usize != SAVE {
            continue;
        }
        let end = r[..0x40].iter().position(|&b| b == 0).unwrap_or(0x40);
        let name = String::from_utf8_lossy(&r[..end]).to_string();
        v.push((i, format!("Slot {}: {name}", v.len() + 1), DIR + COUNT * RECORD + i * s));
    }
    Some(v)
}

pub fn looks(png: &[u8]) -> bool {
    chunk(png).and_then(slots).is_some_and(|s| !s.is_empty())
}

/// The chosen slot (by its listed name; the first when none is picked), every slot's name, and
/// the slot's save.
pub fn unpack(png: &[u8], pick: Option<&str>) -> Result<(usize, Vec<String>, Vec<u8>), String> {
    let c = chunk(png).ok_or("no Kingdom Hearts save data in this picture")?;
    let all = slots(c).ok_or("the save archive is damaged")?;
    if all.is_empty() {
        return Err("no saved game in this file yet".into());
    }
    let i = pick.and_then(|n| all.iter().position(|e| e.1 == n)).unwrap_or(0);
    let at = all[i].2;
    let data = c.get(at..at + SAVE).ok_or("the save archive is cut short")?.to_vec();
    Ok((i, all.into_iter().map(|e| e.1).collect(), data))
}

/// The chunk with slot `slot` (its position in the list) replaced by `data`.
pub fn repack(png: &[u8], slot: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() != SAVE {
        return Err("a Kingdom Hearts save must stay 93,184 bytes".into());
    }
    let c = chunk(png).ok_or("the save's picture has lost its save data")?;
    let all = slots(c).ok_or("the save archive is damaged")?;
    let at = all.get(slot).ok_or("no such save slot")?.2;
    let mut out = c.to_vec();
    out[at..at + SAVE].copy_from_slice(data);
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
        assert!(crate::fmt::kh1::looks(&save));
        assert_eq!(repack(&d, 0, &save).unwrap(), chunk(&d).unwrap());
        for n in &names {
            let (_, _, s) = unpack(&d, Some(n)).unwrap();
            assert!(crate::fmt::kh1::looks(&s), "{n}");
        }
    }
}
